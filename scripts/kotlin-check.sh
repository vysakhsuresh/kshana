#!/usr/bin/env bash
# kotlin-check.sh — type-check (not package) the Android app's Kotlin sources
# without the Android SDK, using only jars from Maven Central / Google Maven.
#
#   scripts/kotlin-check.sh [--refresh] [--profile auto|android|desktop]
#                           [--no-plugin] [--no-stubs] [-v]
#
# First run resolves and caches every artifact under $KTCHECK_CACHE (default:
# the session scratchpad, never inside the repo); later runs are instant.
#
# What goes on the compile classpath (see $KTCHECK_CACHE/README.md):
#   * Android framework classes: org.robolectric:android-all (Maven Central).
#   * kotlin-stdlib 2.1.0, kotlinx-coroutines 1.9.0 (core-jvm + android), JNA 5.14.0.
#   * The AndroidX/Compose libraries the app declares in android/app/build.gradle.kts,
#     resolved transitively from Gradle module metadata / POMs.
#       profile "android": the real androidx.* artifacts from Google Maven
#         (maven.google.com, then dl.google.com/dl/android/maven2).
#       profile "desktop": fallback when Google Maven is unreachable — the
#         JetBrains Compose Multiplatform JVM artifacts from Maven Central.
#         They ship the same androidx.compose.* / androidx.navigation.* /
#         androidx.lifecycle.* packages built from the same sources, but the
#         Android-only entry points (androidx.activity.*, LocalContext) do not
#         exist there, so tiny stub sources are generated in the temp dir for
#         them (disable with --no-stubs).
#   * The Compose compiler plugin (kotlin-compose-compiler-plugin-embeddable 2.1.0)
#     via -Xplugin, so @Composable code gets full semantics. If it fails to
#     load, the run is repeated without it and the summary says so.
#
# Output: file count, error count, then "file:line:col: message" lines.
# Exit status is non-zero when there are errors.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KTCHECK_SRC:-$ROOT/android/app/src/main/java}"   # override only for testing the harness itself
CACHE="${KTCHECK_CACHE:-/tmp/claude-0/-home-user/8ea67731-6b4f-5c74-b408-4973ec91a725/scratchpad/ktcheck}"
JAVA="${JAVA:-/usr/bin/java}"
PROFILE=auto REFRESH=0 USE_PLUGIN=1 USE_STUBS=1 VERBOSE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --refresh) REFRESH=1 ;;
    --profile) PROFILE="$2"; shift ;;
    --profile=*) PROFILE="${1#*=}" ;;
    --no-plugin) USE_PLUGIN=0 ;;
    --no-stubs) USE_STUBS=0 ;;
    -v|--verbose) VERBOSE=1 ;;
    -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done

mkdir -p "$CACHE/bin"
RESOLVER="$CACHE/bin/resolve.py"

# ---------------------------------------------------------------------------
# Dependency resolver (python3 -I, standard library only). Written to the cache
# on every run so the cache is self-describing; downloads live elsewhere.
# ---------------------------------------------------------------------------
cat > "$RESOLVER" <<'PYEOF'
import argparse, json, os, re, sys, time, zipfile, io
import urllib.request, urllib.error
import xml.etree.ElementTree as ET

CENTRAL = ["https://repo1.maven.org/maven2",
           "https://maven-central.storage-download.googleapis.com/maven2"]
GOOGLE = ["https://maven.google.com",
          "https://dl.google.com/dl/android/maven2"]
GOOGLE_GROUPS = ("androidx.", "com.android.", "com.google.android.", "android.arch.")

KOTLIN = "2.1.0"
COROUTINES = "1.9.0"
JNA = "5.14.0"
COMPOSE_BOM = ("androidx.compose", "compose-bom", "2024.12.01")
ANDROID_ALL_FALLBACK = "14-robolectric-10818077"

# The app's direct dependencies (android/app/build.gradle.kts). None = from BOM.
ROOTS_ANDROID = [
    ("androidx.compose.ui", "ui", None),
    ("androidx.compose.foundation", "foundation", None),
    ("androidx.compose.material3", "material3", None),
    ("androidx.compose.material", "material-icons-core", None),
    ("androidx.compose.ui", "ui-tooling-preview", None),
    ("androidx.activity", "activity-compose", "1.9.3"),
    ("androidx.lifecycle", "lifecycle-runtime-compose", "2.8.7"),
    ("androidx.navigation", "navigation-compose", "2.8.5"),
]
# Maven Central fallback: JetBrains Compose Multiplatform 1.7.3 == Jetpack Compose 1.7.6
# (its android variant depends on androidx.compose.ui:ui:1.7.6 exactly).
ROOTS_DESKTOP = [
    ("org.jetbrains.compose.ui", "ui", "1.7.3"),
    ("org.jetbrains.compose.foundation", "foundation", "1.7.3"),
    ("org.jetbrains.compose.material3", "material3", "1.7.3"),
    ("org.jetbrains.compose.material", "material-icons-core", "1.7.3"),
    ("org.jetbrains.compose.ui", "ui-tooling-preview", "1.7.3"),
    ("org.jetbrains.androidx.lifecycle", "lifecycle-runtime-compose", "2.8.4"),
    ("org.jetbrains.androidx.lifecycle", "lifecycle-viewmodel", "2.8.4"),
    ("org.jetbrains.androidx.lifecycle", "lifecycle-common", "2.8.4"),
    ("org.jetbrains.androidx.navigation", "navigation-compose", "2.8.0-alpha12"),
    # androidx.activity:activity-compose has no Central equivalent -> stubbed.
]
ROOTS_COMMON = [
    ("org.jetbrains.kotlin", "kotlin-stdlib", KOTLIN),
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-core-jvm", COROUTINES),
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-android", COROUTINES),
    ("net.java.dev.jna", "jna", JNA),
]
COMPILER_ROOTS = [
    ("org.jetbrains.kotlin", "kotlin-compiler-embeddable", KOTLIN),
    ("org.jetbrains.kotlin", "kotlin-script-runtime", KOTLIN),
    ("org.jetbrains.kotlin", "kotlin-reflect", KOTLIN),
    ("org.jetbrains.kotlin", "kotlin-daemon-embeddable", KOTLIN),
]
PLUGIN = ("org.jetbrains.kotlin", "kotlin-compose-compiler-plugin-embeddable", KOTLIN)

FORCED = {
    ("org.jetbrains.kotlin", "kotlin-stdlib"): KOTLIN,
    ("org.jetbrains.kotlin", "kotlin-reflect"): KOTLIN,
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-core"): COROUTINES,
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-core-jvm"): COROUTINES,
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-android"): COROUTINES,
    ("org.jetbrains.kotlinx", "kotlinx-coroutines-bom"): COROUTINES,
    ("net.java.dev.jna", "jna"): JNA,
}

def skip(g, a):
    if a.endswith("-sources") or a.endswith("-bom"):
        return True
    if a.startswith("skiko-awt-runtime"):          # native skia binaries, not needed to type-check
        return True
    if g == "org.jetbrains.kotlin" and a in ("kotlin-stdlib-common", "kotlin-stdlib-jdk7", "kotlin-stdlib-jdk8"):
        return True                                 # merged into kotlin-stdlib since 1.8/2.0
    if g.startswith("androidx.") and (a.endswith("-lint") or "-test" in a):
        return True
    return False

# ---------------------------------------------------------------------------- http
class Repo:
    def __init__(self, cache, log):
        self.cache, self.log = cache, log
        self.dead = set()

    def _get(self, url, timeout=60):
        req = urllib.request.Request(url, headers={"User-Agent": "kshana-ktcheck/1 (python-urllib)"})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.read()

    def fetch(self, rel, group, want_cache=True):
        """Return bytes for a repository-relative path, or None when no repository has it."""
        local = os.path.join(self.cache, "repo", rel)
        if want_cache and os.path.exists(local):
            with open(local, "rb") as f:
                return f.read()
        bases = (GOOGLE + CENTRAL) if group.startswith(GOOGLE_GROUPS) else CENTRAL
        for base in bases:
            if base in self.dead:
                continue
            url = base + "/" + rel
            for attempt in range(4):
                try:
                    data = self._get(url)
                    os.makedirs(os.path.dirname(local), exist_ok=True)
                    with open(local + ".part", "wb") as f:
                        f.write(data)
                    os.replace(local + ".part", local)
                    self.log(f"  fetched {rel} ({len(data)} bytes) from {base}")
                    return data
                except urllib.error.HTTPError as e:
                    if e.code == 404:
                        break
                    if e.code in (429, 500, 502, 503, 504) and attempt < 3:
                        time.sleep(2 * (attempt + 1))
                        continue
                    self.log(f"  {base}: HTTP {e.code} for {rel}; marking repository unusable")
                    self.dead.add(base)
                    break
                except (urllib.error.URLError, OSError) as e:
                    self.log(f"  {base}: {e}; marking repository unusable")
                    self.dead.add(base)
                    break
        return None

    def reachable(self, bases, rel):
        for base in bases:
            try:
                self._get(base + "/" + rel, timeout=25)
                return base
            except urllib.error.HTTPError as e:
                if e.code != 404:
                    self.dead.add(base)
            except (urllib.error.URLError, OSError):
                self.dead.add(base)
        return None

# ------------------------------------------------------------------------- versions
QUAL = {"dev": 0, "snapshot": 0, "alpha": 1, "beta": 2, "rc": 3, "": 5}
def vkey(v):
    out = []
    for part in re.split(r"[.\-]", v):
        m = re.match(r"^([a-zA-Z]*)(\d*)$", part)
        if m and m.group(1):
            out.append((QUAL.get(m.group(1).lower(), 4), int(m.group(2) or 0)))
        elif part.isdigit():
            out.append((5, int(part)))
        else:
            out.append((4, 0))
    return out

def clean_version(v):
    if not v:
        return v
    v = v.strip()
    m = re.match(r"^[\[(]\s*([^,\]\)]+)\s*(,\s*([^\]\)]*))?[\])]$", v)
    if m:                       # "[1.7.3]" or "[1.0,2.0)" -> lower bound
        return m.group(1).strip()
    return v

# ------------------------------------------------------------------------- parsing
def strip_ns(elem):
    for e in elem.iter():
        if "}" in e.tag:
            e.tag = e.tag.split("}", 1)[1]
    return elem

def parse_pom(data):
    root = strip_ns(ET.fromstring(data))
    props = {}
    parent = root.find("parent")
    g = root.findtext("groupId") or (parent.findtext("groupId") if parent is not None else None)
    v = root.findtext("version") or (parent.findtext("version") if parent is not None else None)
    props.update({"project.groupId": g, "project.version": v, "pom.version": v, "version": v})
    p = root.find("properties")
    if p is not None:
        for e in p:
            props[e.tag] = (e.text or "").strip()
    def sub(s):
        if not s:
            return s
        return re.sub(r"\$\{([^}]+)\}", lambda m: props.get(m.group(1), m.group(0)), s)
    deps, mgmt = [], {}
    for d in root.findall("dependencies/dependency"):
        scope = (d.findtext("scope") or "compile").strip()
        if scope not in ("compile", "runtime"):
            continue
        if (d.findtext("optional") or "false").strip() == "true":
            continue
        deps.append((sub(d.findtext("groupId")), sub(d.findtext("artifactId")), clean_version(sub(d.findtext("version")))))
    for d in root.findall("dependencyManagement/dependencies/dependency"):
        mgmt[(sub(d.findtext("groupId")), sub(d.findtext("artifactId")))] = clean_version(sub(d.findtext("version")))
    return {"packaging": (root.findtext("packaging") or "jar").strip(), "deps": deps, "mgmt": mgmt,
            "gradle_metadata": b"published-with-gradle-metadata" in data}

def pick_variants(module, profile):
    """Choose the JVM/Android variants of a Gradle module: returns (files, deps, available_at)."""
    prefs = ["androidJvm", "jvm", None] if profile == "android" else ["jvm", None]
    variants = module.get("variants", [])
    def ptype(v):
        return v.get("attributes", {}).get("org.jetbrains.kotlin.platform.type")
    def usage(v):
        return v.get("attributes", {}).get("org.gradle.usage")
    for pt in prefs:
        cands = [v for v in variants if ptype(v) == pt and usage(v) in ("java-runtime", "java-api")]
        if pt is None:
            cands = [v for v in cands if v.get("attributes", {}).get("org.gradle.category", "library") == "library"]
        if not cands:
            continue
        cands = [v for v in cands if "sources" not in v["name"].lower() and "javadoc" not in v["name"].lower()]
        if profile != "android":    # e.g. skiko publishes "android*" and "awt*" variants, both typed jvm
            cands = [v for v in cands if not v["name"].lower().startswith("android")] or cands
        rel = [v for v in cands if "release" in v["name"].lower()] or cands
        runtime = [v for v in rel if usage(v) == "java-runtime"] or rel
        chosen = runtime[0]
        if "available-at" in chosen:
            return [], [], chosen["available-at"]["url"]
        files = [(f["name"], f["url"]) for f in chosen.get("files", [])]
        deps = []
        for v in rel:               # union of api + runtime dependencies
            for d in v.get("dependencies", []):
                ver = d.get("version") or {}
                deps.append((d["group"], d["module"], clean_version(ver.get("strictly") or ver.get("requires") or ver.get("prefers"))))
        return files, deps, None
    return None

# ------------------------------------------------------------------------- resolver
class Resolver:
    def __init__(self, repo, profile, bom, log):
        self.repo, self.profile, self.bom, self.log = repo, profile, bom, log
        self.warnings, self.missing = [], []

    def artifact(self, g, a, v, _hops=0):
        """Resolve one artifact: {'files': [(rel, name)], 'deps': [(g,a,v)]} or None if absent."""
        d = f"{g.replace('.', '/')}/{a}/{v}"
        base = f"{d}/{a}-{v}"
        pom = self.repo.fetch(base + ".pom", g)
        if pom is None:
            return None
        info = parse_pom(pom)
        if info["gradle_metadata"]:
            mod = self.repo.fetch(base + ".module", g)
            if mod is not None:
                try:
                    picked = pick_variants(json.loads(mod.decode("utf-8")), self.profile)
                except ValueError as e:
                    picked = None
                    self.warnings.append(f"{g}:{a}:{v}: bad module metadata ({e})")
                if picked is not None:
                    files, deps, avail = picked
                    if avail:
                        # "../../ui-android/1.7.6/ui-android-1.7.6.module" -> that artifact
                        target = os.path.normpath(os.path.join(d, avail))
                        parts = target.split("/")
                        ta, tv = parts[-3], parts[-2]
                        return {"files": [], "deps": [(g, ta, tv)]}
                    return {"files": [(f"{d}/{url}", name) for name, url in files], "deps": deps}
        # Plain POM path.
        deps = list(info["deps"])
        if info["packaging"] == "pom":
            # Multiplatform umbrella without metadata: find its platform variant by suffix.
            suffixes = ["-android", "-jvm"] if self.profile == "android" else ["-desktop", "-jvm"]
            for s in suffixes:
                if _hops < 2 and self.repo.fetch(f"{g.replace('.', '/')}/{a}{s}/{v}/{a}{s}-{v}.pom", g) is not None:
                    deps.append((g, a + s, v))
                    break
            return {"files": [], "deps": deps}
        ext = "aar" if info["packaging"] == "aar" else "jar"
        return {"files": [(f"{base}.{ext}", f"{a}-{v}.{ext}")], "deps": deps}

    def graph(self, roots):
        chosen, info, queue = {}, {}, list(roots)
        while queue:
            g, a, v = queue.pop(0)
            if not g or not a or skip(g, a):
                continue
            if self.profile == "desktop" and g.startswith(GOOGLE_GROUPS):
                self.missing.append(f"{g}:{a}:{v or '?'} (Google Maven only; skipped in desktop profile)")
                continue
            v = FORCED.get((g, a), v) or self.bom.get((g, a))
            if not v:
                self.warnings.append(f"{g}:{a}: no version (not in BOM); skipped")
                continue
            if (g, a) in chosen and vkey(v) <= vkey(chosen[(g, a)]):
                continue
            chosen[(g, a)] = v
            if (g, a, v) not in info:
                info[(g, a, v)] = self.artifact(g, a, v)
                if info[(g, a, v)] is None:
                    self.missing.append(f"{g}:{a}:{v} (not found in any reachable repository)")
            r = info[(g, a, v)]
            if r:
                queue.extend(r["deps"])
        files = []
        for (g, a), v in sorted(chosen.items()):
            r = info[(g, a, v)]
            if r:
                for rel, name in r["files"]:
                    files.append((g, a, v, rel, name))
        return chosen, files

# ------------------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", required=True)
    ap.add_argument("--profile", default="auto", choices=["auto", "android", "desktop"])
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()
    cache = args.cache
    logf = open(os.path.join(cache, "resolve.log"), "a")
    def log(msg):
        logf.write(msg + "\n"); logf.flush()
        if args.verbose:
            print(msg, file=sys.stderr)
    log(f"=== resolve {time.strftime('%Y-%m-%d %H:%M:%S')} profile={args.profile}")
    repo = Repo(cache, log)

    # 1. Which profile? Try Google Maven (the BOM) unless told otherwise.
    profile = args.profile
    bom = {}
    bom_rel = f"{COMPOSE_BOM[0].replace('.', '/')}/{COMPOSE_BOM[1]}/{COMPOSE_BOM[2]}/{COMPOSE_BOM[1]}-{COMPOSE_BOM[2]}.pom"
    google_base = None
    if profile in ("auto", "android"):
        google_base = repo.reachable(GOOGLE, bom_rel)
        if google_base is None:
            msg = "Google Maven is unreachable (maven.google.com redirects to dl.google.com, which this network refuses)"
            if profile == "android":
                print("error: " + msg, file=sys.stderr); sys.exit(3)
            log(msg)
            profile = "desktop"
        else:
            profile = "android"
    if profile == "android":
        data = repo.fetch(bom_rel, COMPOSE_BOM[0])
        bom = parse_pom(data)["mgmt"]
        roots = ROOTS_ANDROID
    else:
        roots = ROOTS_DESKTOP

    # 2. android-all: newest 15-robolectric build on Central, else the known 14 build.
    meta = repo.fetch("org/robolectric/android-all/maven-metadata.xml", "org.robolectric", want_cache=False)
    aa_version = ANDROID_ALL_FALLBACK
    if meta:
        vs = [e.text for e in strip_ns(ET.fromstring(meta)).iter("version") if e.text and e.text.startswith("15-robolectric-")]
        if vs:
            aa_version = max(vs, key=lambda s: int(s.rsplit("-", 1)[1]))
    aa_rel = f"org/robolectric/android-all/{aa_version}/android-all-{aa_version}.jar"
    if repo.fetch(aa_rel, "org.robolectric") is None:
        print("error: could not download org.robolectric:android-all:" + aa_version, file=sys.stderr); sys.exit(3)

    # 3. Library graph, compiler graph, plugin.
    res = Resolver(repo, profile, bom, log)
    chosen, files = res.graph(ROOTS_COMMON + roots)
    cres = Resolver(repo, "desktop", {}, log)
    cchosen, cfiles = cres.graph(COMPILER_ROOTS)
    plugin_rel = f"{PLUGIN[0].replace('.', '/')}/{PLUGIN[1]}/{PLUGIN[2]}/{PLUGIN[1]}-{PLUGIN[2]}.jar"
    plugin_ok = repo.fetch(plugin_rel, PLUGIN[0]) is not None

    def materialize(entries):
        out = []
        for g, a, v, rel, name in entries:
            data = repo.fetch(rel, g)
            if data is None:
                res.missing.append(f"{g}:{a}:{v} file {rel} not downloadable"); continue
            local = os.path.join(cache, "repo", rel)
            if rel.endswith(".aar"):
                dest = os.path.join(cache, "classes", f"{g}__{a}__{v}")
                if not os.path.isdir(dest):
                    os.makedirs(dest + ".tmp", exist_ok=True)
                    with zipfile.ZipFile(local) as z:
                        for n in z.namelist():
                            if n == "classes.jar" or (n.startswith("libs/") and n.endswith(".jar")):
                                tgt = os.path.join(dest + ".tmp", n.replace("/", "__"))
                                with open(tgt, "wb") as f:
                                    f.write(z.read(n))
                    os.rename(dest + ".tmp", dest)
                for n in sorted(os.listdir(dest)):
                    out.append(os.path.join(dest, n))
            else:
                out.append(local)
        return out

    classpath = [os.path.join(cache, "repo", aa_rel)] + materialize(files)
    compiler_cp = materialize(cfiles)

    with open(os.path.join(cache, "classpath.txt"), "w") as f:
        f.write("\n".join(classpath) + "\n")
    with open(os.path.join(cache, "compiler-classpath.txt"), "w") as f:
        f.write("\n".join(compiler_cp) + "\n")
    with open(os.path.join(cache, "plugin.txt"), "w") as f:
        f.write((os.path.join(cache, "repo", plugin_rel) if plugin_ok else "") + "\n")
    def dedupe(xs):
        seen, out = set(), []
        for x in xs:
            if x not in seen:
                seen.add(x); out.append(x)
        return out
    all_missing = dedupe(res.missing + cres.missing)
    all_warnings = dedupe(res.warnings + cres.warnings)
    summary = {
        "profile": profile, "google_maven": google_base,
        "android_all": aa_version, "compose_bom": dict((f"{k[0]}:{k[1]}", v) for k, v in bom.items()) if bom else None,
        "libraries": dict((f"{k[0]}:{k[1]}", v) for k, v in sorted(chosen.items())),
        "compiler": dict((f"{k[0]}:{k[1]}", v) for k, v in sorted(cchosen.items())),
        "compose_plugin": plugin_ok, "classpath_jars": len(classpath),
        "warnings": all_warnings, "missing": all_missing,
    }
    with open(os.path.join(cache, "resolve.json"), "w") as f:
        json.dump(summary, f, indent=2)
    print(f"profile={profile} android_all={aa_version} jars={len(classpath)} plugin={'yes' if plugin_ok else 'NO'}")
    google_only = [m for m in all_missing if "Google Maven only" in m]
    if google_only:
        print(f"  skipped {len(google_only)} Google-Maven-only transitive artifacts "
              f"(androidx.lifecycle/annotation/collection/arch.core; details in resolve.json) - stubbed by the script")
    for m in all_missing:
        if m not in google_only:
            print("  missing: " + m)
    for w in all_warnings:
        print("  warning: " + w)

if __name__ == "__main__":
    main()
PYEOF

# ---------------------------------------------------------------------------
# Resolve (cached).
# ---------------------------------------------------------------------------
need_resolve=$REFRESH
if [ ! -s "$CACHE/classpath.txt" ] || [ ! -s "$CACHE/compiler-classpath.txt" ] || [ ! -f "$CACHE/resolve.json" ]; then
  need_resolve=1
else
  while IFS= read -r p; do [ -f "$p" ] || { need_resolve=1; break; }; done < "$CACHE/classpath.txt"
  if [ "$PROFILE" != auto ] && ! grep -q "\"profile\": \"$PROFILE\"" "$CACHE/resolve.json"; then need_resolve=1; fi
fi
if [ "$need_resolve" = 1 ]; then
  echo "resolving dependencies into $CACHE ..."
  python3 -I "$RESOLVER" --cache "$CACHE" --profile "$PROFILE" $([ "$VERBOSE" = 1 ] && echo --verbose) || { echo "dependency resolution failed (see $CACHE/resolve.log)"; exit 3; }
fi
RES_PROFILE=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["profile"])' "$CACHE/resolve.json")
PLUGIN_JAR=$(head -n1 "$CACHE/plugin.txt" 2>/dev/null || true)
CP=$(paste -sd: "$CACHE/classpath.txt")
COMPILER_CP=$(paste -sd: "$CACHE/compiler-classpath.txt")
NJARS=$(grep -c . "$CACHE/classpath.txt")

cat > "$CACHE/README.md" <<EOF
# ktcheck cache

Populated by \`scripts/kotlin-check.sh\` (repo: $ROOT). Safe to delete; it is rebuilt on the next run
(\`--refresh\` forces re-resolution). Nothing here is tracked by git.

- \`bin/resolve.py\`          the resolver (rewritten by the script on every run; python3 -I, stdlib only)
- \`repo/<group>/<artifact>/<version>/\`  downloaded .pom/.module/.jar/.aar files, Maven layout
- \`classes/<group>__<artifact>__<version>/\`  classes.jar (+ libs/*.jar) extracted from each .aar
- \`classpath.txt\`           the resolved compile classpath, one path per line (android-all first)
- \`compiler-classpath.txt\`  jars that run the Kotlin $(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["compiler"]["org.jetbrains.kotlin:kotlin-compiler-embeddable"])' "$CACHE/resolve.json") compiler itself
- \`plugin.txt\`              path of the Compose compiler plugin jar (empty if it could not be fetched)
- \`resolve.json\`            what was resolved: profile, versions, warnings, missing artifacts
- \`resolve.log\`             every download and repository failure
- \`build.*/\`                temp output of a run in progress (removed when the run ends)

Current profile: **$RES_PROFILE** — "android" = real androidx artifacts from Google Maven;
"desktop" = JetBrains Compose Multiplatform JVM artifacts from Maven Central (same packages),
used because Google Maven (maven.google.com -> dl.google.com) is not reachable from this network.
EOF

# ---------------------------------------------------------------------------
# Collect sources and generate stubs.
# ---------------------------------------------------------------------------
if [ ! -d "$SRC" ]; then echo "no sources at $SRC"; exit 2; fi
mapfile -t FILES < <(find "$SRC" -name '*.kt' -type f | sort)
if [ ${#FILES[@]} -eq 0 ]; then echo "no .kt files under $SRC"; exit 2; fi
FFI="$SRC/uniffi/kshana_ffi/kshana_ffi.kt"
[ -f "$FFI" ] || echo "note: generated $FFI is absent (uniffi-bindgen has not run); uniffi.* references will be unresolved"

TMP=$(mktemp -d "$CACHE/build.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/out" "$TMP/stubs"
STUBS=()
NOTES=()

if grep -rqE '\bR\.(string|mipmap|drawable|color|dimen|style|layout|id|raw|xml|array|integer|bool|font)\b' "${FILES[@]}"; then
  cat > "$TMP/stubs/R.kt" <<'EOF'
// Generated by scripts/kotlin-check.sh: stand-in for the aapt-generated R class.
package com.kshana.app
object R { object string { const val app_name = 1 }; object mipmap { const val ic_launcher = 1 }; object drawable { const val ic_kshana_mark = 1 } }
EOF
  STUBS+=("$TMP/stubs/R.kt"); NOTES+=("R stub generated (sources reference R.*)")
fi

if [ "$RES_PROFILE" = desktop ] && [ "$USE_STUBS" = 1 ]; then
  # Android-only entry points that the Maven Central (JVM) Compose artifacts do not carry.
  cat > "$TMP/stubs/ActivityStub.kt" <<'EOF'
// Generated by scripts/kotlin-check.sh (desktop profile): androidx.activity is Google-Maven-only.
package androidx.activity

open class ComponentActivity : android.app.Activity()

class SystemBarStyle private constructor() {
    companion object {
        fun auto(lightScrim: Int, darkScrim: Int, detectDarkMode: (android.content.res.Resources) -> Boolean = { false }): SystemBarStyle = SystemBarStyle()
        fun dark(scrim: Int): SystemBarStyle = SystemBarStyle()
        fun light(scrim: Int, darkScrim: Int): SystemBarStyle = SystemBarStyle()
    }
}

fun ComponentActivity.enableEdgeToEdge(
    statusBarStyle: SystemBarStyle = SystemBarStyle.auto(0, 0),
    navigationBarStyle: SystemBarStyle = SystemBarStyle.auto(0, 0),
) {}
EOF
  cat > "$TMP/stubs/ActivityComposeStub.kt" <<'EOF'
// Generated by scripts/kotlin-check.sh (desktop profile): androidx.activity.compose is Google-Maven-only.
package androidx.activity.compose

import androidx.activity.ComponentActivity
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionContext

fun ComponentActivity.setContent(parent: CompositionContext? = null, content: @Composable () -> Unit) {}
EOF
  cat > "$TMP/stubs/LocalContextStub.kt" <<'EOF'
// Generated by scripts/kotlin-check.sh (desktop profile): LocalContext exists only in the Android ui artifact.
package androidx.compose.ui.platform

import android.content.Context
import androidx.compose.runtime.ProvidableCompositionLocal
import androidx.compose.runtime.staticCompositionLocalOf

val LocalContext: ProvidableCompositionLocal<Context> = staticCompositionLocalOf { error("stub") }
EOF
  # The JetBrains JVM lifecycle/navigation jars reference these base types from Google's
  # androidx.lifecycle:lifecycle-common (not on Central); they are supertypes of NavBackStackEntry.
  cat > "$TMP/stubs/LifecycleStub.kt" <<'EOF'
// Generated by scripts/kotlin-check.sh (desktop profile): androidx.lifecycle:lifecycle-common is Google-Maven-only.
package androidx.lifecycle

abstract class Lifecycle {
    abstract val currentState: State
    enum class State { DESTROYED, INITIALIZED, CREATED, STARTED, RESUMED; fun isAtLeast(state: State): Boolean = compareTo(state) >= 0 }
    enum class Event { ON_CREATE, ON_START, ON_RESUME, ON_PAUSE, ON_STOP, ON_DESTROY, ON_ANY }
}
interface LifecycleOwner { val lifecycle: Lifecycle }
open class ViewModelStore
interface ViewModelStoreOwner { val viewModelStore: ViewModelStore }
interface HasDefaultViewModelProviderFactory
EOF
  STUBS+=("$TMP/stubs/ActivityStub.kt" "$TMP/stubs/ActivityComposeStub.kt" "$TMP/stubs/LocalContextStub.kt" "$TMP/stubs/LifecycleStub.kt")
  NOTES+=("desktop profile: stubbed androidx.activity.ComponentActivity/enableEdgeToEdge, androidx.activity.compose.setContent, androidx.compose.ui.platform.LocalContext, androidx.lifecycle.{Lifecycle,LifecycleOwner,ViewModelStore(Owner),HasDefaultViewModelProviderFactory} (--no-stubs disables)")
fi

# ---------------------------------------------------------------------------
# Compile.
# ---------------------------------------------------------------------------
KOTLINC=("$JAVA" -Xmx3g -cp "$COMPILER_CP" org.jetbrains.kotlin.cli.jvm.K2JVMCompiler
         -no-stdlib -no-reflect -jvm-target 17 -Xallow-unstable-dependencies -nowarn
         -classpath "$CP" -d "$TMP/out")

run_compiler() {   # $1 = "plugin" | "noplugin"
  local extra=()
  [ "$1" = plugin ] && extra=("-Xplugin=$PLUGIN_JAR")
  "${KOTLINC[@]}" "${extra[@]}" "${FILES[@]}" "${STUBS[@]}" > "$TMP/log.$1" 2>&1
  echo $?
}

PLUGIN_STATUS="not used (--no-plugin)"
if [ "$USE_PLUGIN" = 1 ] && [ -n "$PLUGIN_JAR" ] && [ -f "$PLUGIN_JAR" ]; then
  rc=$(run_compiler plugin); LOG="$TMP/log.plugin"
  if [ "$rc" != 0 ] && [ "$rc" != 1 ] || grep -qE "Could not load plugin|Plugin .* (failed|not found)|PluginCliParser|exception: java\.lang\.(NoSuchMethod|ClassNotFound|NoClassDefFound|AbstractMethod)" "$LOG"; then
    reason=$(grep -vE "Picked up JAVA_TOOL_OPTIONS|^\s+at |^\s*$" "$LOG" | head -n1)
    PLUGIN_STATUS="FAILED to load, compiled without it (${reason:-exit code $rc})"
    rc=$(run_compiler noplugin); LOG="$TMP/log.noplugin"
  else
    PLUGIN_STATUS="loaded ($(basename "$PLUGIN_JAR"))"
  fi
else
  [ "$USE_PLUGIN" = 1 ] && PLUGIN_STATUS="not downloaded, compiled without it"
  rc=$(run_compiler noplugin); LOG="$TMP/log.noplugin"
fi

# ---------------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------------
# Error lines plus their indented continuation lines (candidate lists etc.); stack traces and
# JAVA_TOOL_OPTIONS noise are dropped.
ERRS=$(grep -v "Picked up JAVA_TOOL_OPTIONS" "$LOG" | awk '
  /^[^ ]+\.kt:[0-9]+:[0-9]+: error: / || /^error: / { p=1; print; next }
  /^[^ ]+\.kt:[0-9]+:[0-9]+: (warning|info): / { p=0; next }
  p && /^[ \t]+/ && !/^[ \t]+at / { print; next }
  { p=0 }')
NERR=$(printf '%s\n' "$ERRS" | grep -cE '^[^ ]+\.kt:[0-9]+:[0-9]+: error: |^error: ' || true)
echo "kotlin-check: profile=$RES_PROFILE, classpath=$NJARS jars, compose plugin: $PLUGIN_STATUS"
for n in "${NOTES[@]}"; do echo "note: $n"; done
echo "files: ${#FILES[@]}  errors: $NERR  (compiler exit $rc)"
if [ -n "$ERRS" ]; then
  printf '%s\n' "$ERRS" | sed -E "s#^$SRC/##; s/: error: /: /"
fi
if [ "$VERBOSE" = 1 ]; then echo "--- full compiler log ---"; grep -v "Picked up JAVA_TOOL_OPTIONS" "$LOG"; fi
if [ "$rc" != 0 ] && [ "$rc" != 1 ]; then
  echo "compiler crashed (exit $rc); first lines of the log:"
  grep -vE "Picked up JAVA_TOOL_OPTIONS|^\s+at " "$LOG" | head -n 15
fi
[ "$rc" = 0 ] && [ "$NERR" = 0 ]
