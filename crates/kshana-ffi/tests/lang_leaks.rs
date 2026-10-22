//! Language leaks: every piece of English (any Latin-script word of three letters or
//! more) that reaches a person who chose Malayalam or Hindi.
//!
//! Every engine function the Android app calls is run for three birth charts at
//! 2026-10-22, in "ml" and "hi" where the function takes a language, and once per chart
//! where it takes none (those return English to everyone; they are listed apart). Every
//! string in every returned record is read (the Debug rendering, decoded), and each run
//! of Latin-script text in it is traced to the source file whose string literals cover
//! it, so translators can be given work file by file and can rerun this to confirm it.
//!
//! Not counted: names passed in (the family's names, "Kshana"), hashes and JSON, short
//! all-capital acronyms and units (KP, BPHS, UTC), fields that hold machine keys (`id`,
//! `key`, `*_key(s)`, `rule_id(s)`, `source_keys`, `system_keys`, provenance), fields
//! for the chart pages' expert lines (`expert`, `*_expert`), and a one-word value in a
//! key-like field (`area: "Career"`, `kind: "opening"`). Text that the app's Kotlin
//! dictionaries (`Lang.kt`, `LangExplore.kt`, `LangTrust*.kt`) translate whole is listed
//! apart too, as "translated in Kotlin".
//!
//! Run: `cargo test --release -p kshana-ffi --test lang_leaks -- --ignored --nocapture`.
//! It writes, to the temp directory (or `KSHANA_LEAKS_OUT`):
//! - `kshana_lang_leaks.tsv`: every leak, `file \t function \t field \t lang \t run \t string`;
//! - `kshana_lang_leaks_by_file.txt`: the leaks grouped by source file, with counts,
//!   the functions and fields they come through, whether those take a language, whether
//!   their screens are plain or the chart pages, and examples.
//!
//! It never fails on leaks (it is an inventory); it fails only if it reads too little.
//! Set `KSHANA_LEAKS_STRICT=1` to make it fail when any leak remains, once a batch of
//! files is translated.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kshana_ffi::family::FamilyMemberInput;
use kshana_ffi::integrity::{
    blind_answers, blind_judge, blind_question, blind_resolve, confidence_line, match_logged_event, record_card, remedy_kept_compare,
    system_hit_rates, tier_of_probability, DaySignalInput, SystemVoteInput,
};
use kshana_ffi::opentest::{open_test_contribution, open_test_own, open_test_protocol, silenced_areas, OpenTestClaimInput};
use kshana_ffi::sources::source_lines;
use kshana_ffi::trust::{forecast_tilt, resolve_claim, scoreboard, seal_new_claims, PersonalWeightsInput, ScoreItemInput};
use kshana_ffi::*;

// ---------------------------------------------------------------------------------
// What each function is: whether it takes a language, and where the app shows it.
// ---------------------------------------------------------------------------------

/// `(function, takes a lang, screens)`; screens are "plain" (the Loop, the tabs and
/// the Explore pages everyone sees), "expert" (only with "Show the chart pages" on) or
/// "both". From grepping the Kotlin (`android/app/src/main/java/com/kshana/app`).
const FNS: &[(&str, bool, &str)] = &[
    // Engine methods
    ("appointment", true, "plain"),       // PlanScreen
    ("area_readings", true, "expert"),    // AreasScreen, ReadingScreen (reading_expert)
    ("ashtakavarga", false, "expert"),     // VedicScreen
    ("ask", true, "plain"),               // AskScreen, ReadingScreen2
    ("ask_topic", true, "plain"),         // AskScreen, ReadingScreen2
    ("aspects", true, "expert"),          // ChartScreen
    ("best_slots", true, "plain"),        // CalendarScreen (appointments), PlanScreen
    ("best_slots_together", true, "plain"), // FamilyDatesScreen
    ("birth_posterior", true, "plain"),   // RectifyScreen
    ("birth_time_now", true, "plain"),     // PastCheckScreen
    ("briefing", true, "plain"),           // LoopScreen, ReadingScreen2, Hindsight, Sunrise notification
    ("calendar", true, "expert"),         // CalendarScreen (skycalendar)
    ("calendar_days", true, "plain"),     // AlmanacScreen (DepthScreens)
    ("calibrate_ledger", true, "expert"), // LedgerScreen
    ("chart", false, "both"),              // Loop, Day, Trace, Naming, Now, Vedic, Chart, …
    ("chart_facts", true, "plain"),        // PastCheckScreen
    ("chart_yogas", true, "plain"),       // GiftsScreen (DepthScreens)
    ("compatibility", true, "expert"),    // CompatibilityScreen
    ("countdowns", true, "plain"),        // LoopScreen, WallScreen
    ("couple", true, "plain"),            // TogetherScreen
    ("couple_ask", true, "plain"),        // TogetherScreen
    ("coverage_tally", true, "expert"),   // LedgerScreen
    ("daily", true, "expert"),            // TodayScreen, RightTimeScreen
    ("dasha_chidra_lords", true, "expert"), // MargaScreen
    ("dasha_sandhi_windows", true, "expert"), // MargaScreen
    ("dasha_system", true, "expert"),     // MargaScreen
    ("dasha_systems_at", true, "expert"), // TimeLordsScreen
    ("dasha_systems_more_at", true, "expert"), // DashasMoreScreen
    ("day_clock", true, "expert"),        // DayScreen
    ("eclipses", false, "expert"),         // CalendarScreen (skycalendar)
    ("explain", true, "plain"),           // ExplainPanel (LifeScreen)
    ("family_proof", true, "plain"),       // FamilyProofScreen, Trust.kt
    ("festivals_here", true, "plain"),    // LoopScreen, WallScreen
    ("firdaria", true, "expert"),         // TimeLordsScreen
    ("firm_predictions", true, "plain"),   // Trust.kt (sealed claims)
    ("forecast_horizon", true, "plain"),  // Forecast.kt
    ("forecast_moment", true, "plain"),    // LoopScreen, TogetherScreen, notifications
    ("future_personal", true, "plain"),    // FutureScreen, ReadingScreen2
    ("jaimini_extras", true, "expert"),   // MargaScreen
    ("kakshya_windows", false, "expert"),  // TransitsScreen
    ("karakas", true, "expert"),          // MeScreen
    ("kp_full", true, "expert"),          // KpFullScreen
    ("kundali", true, "plain"),           // TogetherScreen
    ("learn_techniques", true, "plain"),   // PastCheckScreen
    ("ledger_metrics", true, "expert"),   // LedgerScreen
    ("letter", true, "plain"),            // LetterScreen
    ("life_map", false, "expert"),         // LifeMapScreen
    ("moon_windows", true, "expert"),     // DayScreen
    ("muhurta", false, "expert"),          // MuhurtaScreen
    ("nazhika_to_instant", true, "plain"), // BirthScreen
    ("next_events", false, "expert"),      // MomentsScreen, TodayScreen
    ("outlook", true, "plain"),            // AheadScreen
    ("pancha_pakshi", true, "expert"),    // DayScreen
    ("panchanga_detail", false, "both"),   // Lamp (plain), LexiconScreen, VedicScreen
    ("parihara", true, "expert"),         // PariharaScreen (parihara_classic)
    ("past_checks", true, "plain"),        // ReadingScreen2
    ("past_reveal", true, "plain"),        // PastCheckScreen
    ("plain_reading", true, "plain"),     // LifeScreen (life/{life} and mylife), NarrateScreen
    ("prashna", true, "expert"),          // PrashnaScreen (prashna_expert)
    ("prasna_arudha_sphutas", true, "expert"), // MargaScreen
    ("prasna_marga_sphutas", true, "expert"), // MargaScreen
    ("profections", true, "expert"),      // TimeLordsScreen
    ("progressed_hits", true, "expert"),  // TimeLordsScreen
    ("progressions", true, "expert"),     // TimeLordsScreen
    ("reading_report_json", false, "expert"), // NarrateScreen, TrustScreen
    ("rectify", true, "plain"),           // RectifyScreen
    ("rectify_kp", true, "plain"),        // RectifyScreen
    ("remedy_schedule", true, "plain"),    // RemedyTreeScreen
    ("remedy_tree", true, "plain"),        // RemedyTreeScreen, ReadingScreen2
    ("retrodict", true, "expert"),        // LedgerScreen
    ("retrodiction_quiz", true, "expert"), // QuizScreen
    ("score_events", true, "expert"),     // LedgerScreen
    ("seal_forecast", true, "expert"),    // SealedScreen
    ("seal_pick", true, "plain"),          // PastCheckScreen
    ("self_test", false, "expert"),        // TrustScreen
    ("shadbala", true, "expert"),         // StrengthScreen
    ("slow_horizon", false, "plain"),      // HindsightScreen
    ("snapshot", true, "expert"),         // DiaryScreen, TraceScreen
    ("stability", false, "expert"),        // ChartScreen, TrustScreen
    ("sure_next", true, "plain"),          // SureScreen
    ("sure_window_hours", true, "plain"),  // SureScreen
    ("sure_windows_trusted", true, "plain"), // SureScreen, Trust.kt
    ("talk_window", false, "plain"),       // LoopScreen
    ("time_shift_test", true, "expert"),  // LedgerScreen
    ("timeline", true, "plain"),          // BookScreen
    ("trace", true, "expert"),            // TraceScreen
    ("transit_score", true, "expert"),    // TransitsScreen
    ("transit_windows", true, "expert"),  // TransitsScreen
    ("transits", false, "expert"),         // TodayScreen
    ("varshaphala", true, "expert"),      // YearScreen
    ("vimshopaka_bala", true, "expert"),  // MargaScreen
    ("year_chapter", true, "plain"),      // BookScreen
    ("zodiacal_releasing", true, "expert"), // TimeLordsScreen
    // Free functions
    ("ashtamangala_number", true, "expert"), // MargaScreen
    ("birth_bird", true, "expert"),       // DayScreen
    ("blind_answers", true, "plain"),      // ScoreboardScreen
    ("blind_judge", true, "plain"),        // ScoreboardScreen
    ("blind_question", true, "plain"),     // ScoreboardScreen
    ("blind_resolve", true, "plain"),      // Integrity.kt
    ("calibrate", true, "plain"),         // LifeScreen, RecordScreen, TrustScreen, PredictionStore
    ("confidence_line", true, "plain"),    // Integrity.kt
    ("dasha_chain", false, "expert"),      // VedicScreen
    ("dasha_level_names", true, "expert"), // TraceScreen
    ("dasha_sensitivity", true, "expert"), // TraceScreen
    ("dasha_table", false, "expert"),      // VedicScreen
    ("error_budget", true, "expert"),     // TrustScreen
    ("forecast_tilt", true, "plain"),      // ScoreboardScreen
    ("glossary", true, "expert"),         // GlossaryScreen
    ("grade_quiz", true, "expert"),       // QuizScreen
    ("language_names", false, "plain"),    // SettingsScreen, LexiconScreen
    ("ledger_event_kind_name", true, "expert"), // LedgerScreen, SealedScreen
    ("sure_words", true, "plain"),         // AskScreen, ReadingScreen2, TogetherScreen
    ("ledger_event_kinds", false, "expert"), // LedgerScreen, SealedScreen
    ("lexicon", true, "expert"),           // LexiconScreen
    ("match_logged_event", true, "plain"), // Integrity.kt
    ("naming", true, "expert"),           // NamingScreen
    ("nazhika", false, "expert"),          // LexiconScreen
    ("open_test_contribution", true, "plain"), // OpenTest.kt
    ("open_test_own", true, "plain"),      // OpenTestScreen
    ("open_test_protocol", false, "plain"), // OpenTestScreen
    ("purposes", true, "plain"),          // PlanScreen, FamilyDatesScreen
    ("questions", true, "plain"),         // AskScreen, TogetherScreen
    ("record_card", true, "plain"),        // Integrity.kt
    ("remedy_how_it_helps", true, "both"), // PariharaScreen (expert), TogetherScreen (plain)
    ("remedy_kept_compare", true, "plain"), // Integrity.kt
    ("resolve_forecast", false, "expert"), // SealedScreen
    ("scoreboard", true, "plain"),         // ScoreboardScreen, OpenTestScreen, TrustScreen
    ("seal_new_claims", true, "plain"),    // Trust.kt
    ("silenced_areas", true, "plain"),     // OpenTest.kt
    ("source_lines", true, "plain"),       // WhereFrom.kt
    ("system_hit_rates", true, "plain"),   // Integrity.kt
    ("tier_of_probability", false, "plain"), // Integrity.kt
    ("topics", true, "plain"),            // AskScreen, ReadingScreen2
    ("understand", true, "plain"),        // AskScreen
    ("hora_lore_all", false, "plain"),     // the hour's lore on the Loop comes through forecast_moment; this is the whole table
    ("vakya_note", false, "expert"),       // LexiconScreen
];

fn fn_info(f: &str) -> (bool, &'static str) {
    FNS.iter().find(|(n, _, _)| *n == f).map(|(_, l, s)| (*l, *s)).unwrap_or((false, "?"))
}

// ---------------------------------------------------------------------------------
// Reading the strings.
// ---------------------------------------------------------------------------------

/// Fields that hold keys, hashes or the expert lines; never counted.
fn skip_field(f: &str) -> bool {
    const SKIP: &[&str] = &[
        "id", "key", "keys", "rule_id", "rule_ids", "source_keys", "system_keys", "witness_keys", "best_key", "signature", "provenance_engine", "provenance_kernel",
        "provenance_ut1", "provenance_models", "fingerprint", "salt", "hash", "sha", "json", "expert", "why_expert", "verse",
    ];
    SKIP.contains(&f) || f.ends_with("_expert") || f.ends_with("_key") || f.ends_with("_keys") || f.ends_with("_id") || f.ends_with("_ids") || f.ends_with("_hash")
}

/// Fields where a one-word value is a key (`area: "Career"`, `kind: "opening"`).
fn keyish_field(f: &str) -> bool {
    const K: &[&str] = &[
        "area", "areas", "life", "lives", "kind", "tier", "band", "verdict", "source", "topic", "relation", "status", "trend", "purpose", "intent", "unit", "also", "reason",
        "outcome", "planet", "lord", "body", "system", "shape", "lot", "activity", "level", "grade", "direction_key", "mood",
    ];
    K.contains(&f)
}

/// `(field, value)` for every string in a Debug rendering, decoded (Debug escapes the
/// Malayalam and Devanagari vowel signs as `\u{..}`). From `safety_audit.rs`.
fn strings(debug: &str) -> Vec<(String, String)> {
    let b = debug.as_bytes();
    let mut out = Vec::new();
    let mut field = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' {
            let mut j = i + 1;
            let mut s: Vec<u8> = Vec::new();
            while j < b.len() && b[j] != b'"' {
                if b[j] == b'\\' && j + 1 < b.len() {
                    j += 1;
                    match b[j] {
                        b'u' if j + 1 < b.len() && b[j + 1] == b'{' => {
                            let end = debug[j..].find('}').map(|k| j + k).unwrap_or(j);
                            if let Some(ch) = u32::from_str_radix(&debug[j + 2..end], 16).ok().and_then(char::from_u32) {
                                let mut buf = [0u8; 4];
                                s.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            }
                            j = end + 1;
                            continue;
                        }
                        b'n' => s.push(b'\n'),
                        b't' => s.push(b'\t'),
                        other => s.push(other),
                    }
                    j += 1;
                    continue;
                }
                s.push(b[j]);
                j += 1;
            }
            out.push((field.clone(), String::from_utf8_lossy(&s).into_owned()));
            i = j + 1;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            if i + 1 < b.len() && b[i] == b':' && b[i + 1] == b' ' {
                field = debug[start..i].to_string();
            }
            continue;
        }
        i += 1;
    }
    out
}

/// `(key, value)` for every string in a JSON document.
fn json_strings(v: &serde_json::Value, key: &str, out: &mut Vec<(String, String)>) {
    match v {
        serde_json::Value::String(s) => out.push((key.to_string(), s.clone())),
        serde_json::Value::Array(a) => a.iter().for_each(|x| json_strings(x, key, out)),
        serde_json::Value::Object(m) => m.iter().for_each(|(k, x)| json_strings(x, k, out)),
        _ => {}
    }
}

/// Names passed in, and other words that are fine in any script.
const ALLOWED_WORDS: &[&str] = &["Kshana", "Anu", "Ravi", "Meera", "Kiran", "Devi", "UTC", "IST", "GMT", "km", "deg", "min", "sec", "hrs"];

/// Whether `w` (a run of Latin letters) counts as English.
fn counts(w: &str) -> bool {
    if w.chars().count() < 3 || ALLOWED_WORDS.contains(&w) {
        return false;
    }
    // Short all-capital acronyms: KP, BPHS, SHA, JSON.
    if w.len() <= 5 && w.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    true
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c) || ('\u{1E00}'..='\u{1EFF}').contains(&c)
}

/// The runs of Latin-script text in `s` that hold at least one counted word: maximal
/// stretches of Latin words with the spaces, digits and light punctuation between them,
/// split at "·", "|", line breaks and any non-Latin script.
fn latin_runs(s: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, runs: &mut Vec<String>| {
        let t = cur.trim_matches(|c: char| !is_latin(c) && !c.is_ascii_digit() && c != ')' && c != '.' && c != '?' && c != '!').trim().to_string();
        let words: Vec<&str> = t.split(|c: char| !is_latin(c) && c != '\'').filter(|w| !w.is_empty()).collect();
        if words.iter().any(|w| counts(w.trim_matches('\''))) {
            runs.push(t);
        }
        cur.clear();
    };
    for c in s.chars() {
        let soft = c.is_ascii() && !matches!(c, '|' | '\n' | '\t');
        let latin_ext = is_latin(c) || matches!(c, '–' | '—' | '’' | '‘' | '“' | '”' | '…' | '°' | '′' | '″' | '½' | '¼' | '¾' | '×');
        if soft || latin_ext {
            cur.push(c);
        } else {
            flush(&mut cur, &mut runs);
        }
    }
    flush(&mut cur, &mut runs);
    runs
}

fn is_hexish(s: &str) -> bool {
    s.len() >= 16 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-' || c == ':')
}

/// A machine key: one token of lower-case letters, digits, `_`, `-`, `.`, `:` or `/`.
fn is_machine_key(s: &str) -> bool {
    !s.is_empty() && !s.contains(' ') && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.' | ':' | '/'))
}

/// Digits folded, so the same template with other numbers is one entry.
fn norm(s: &str) -> String {
    let mut o = String::new();
    let mut in_d = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !in_d {
                o.push('#');
            }
            in_d = true;
        } else {
            in_d = false;
            o.push(c);
        }
    }
    o
}

// ---------------------------------------------------------------------------------
// The sources: every string literal in crates/*/src, split at format placeholders.
// ---------------------------------------------------------------------------------

struct Sources {
    /// Distinct literal fragments (with Latin letters, 4+ bytes) and the files holding each.
    frags: Vec<(String, Vec<String>)>,
    /// Fragment indices by their first four bytes.
    by_prefix: HashMap<[u8; 4], Vec<usize>>,
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target" || n == "tests" || n == "benches" || n == "examples" || n == "kshana-cli" || n == "bin") {
                continue;
            }
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The string literals of a Rust source, decoded, comments skipped.
fn literals(src: &str) -> Vec<String> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let n = b.len();
    while i < n {
        let c = b[i];
        // Comments.
        if c == '/' && i + 1 < n && b[i + 1] == '/' {
            while i < n && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && b[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        // Char literals and lifetimes.
        if c == '\'' {
            if i + 2 < n && b[i + 1] == '\\' {
                // '\n', '\'', '\u{..}'
                let mut j = i + 2;
                while j < n && b[j] != '\'' && j - i < 12 {
                    j += 1;
                }
                i = j + 1;
                continue;
            }
            if i + 2 < n && b[i + 2] == '\'' {
                i += 3;
                continue;
            }
            i += 1;
            continue;
        }
        // Raw strings r"..." / r#"..."#.
        if c == 'r' && i + 1 < n && (b[i + 1] == '"' || b[i + 1] == '#') && (i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while j < n && b[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if j < n && b[j] == '"' {
                j += 1;
                let mut s = String::new();
                while j < n {
                    if b[j] == '"' && (0..hashes).all(|k| j + 1 + k < n && b[j + 1 + k] == '#') {
                        break;
                    }
                    s.push(b[j]);
                    j += 1;
                }
                out.push(s);
                i = j + 1 + hashes;
                continue;
            }
        }
        if c == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < n && b[j] != '"' {
                if b[j] == '\\' && j + 1 < n {
                    j += 1;
                    match b[j] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\n' | '\r' => {
                            while j + 1 < n && b[j + 1].is_whitespace() {
                                j += 1;
                            }
                        }
                        'u' if j + 1 < n && b[j + 1] == '{' => {
                            let mut k = j + 2;
                            let mut hex = String::new();
                            while k < n && b[k] != '}' {
                                hex.push(b[k]);
                                k += 1;
                            }
                            if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                s.push(ch);
                            }
                            j = k;
                        }
                        other => s.push(other),
                    }
                    j += 1;
                    continue;
                }
                s.push(b[j]);
                j += 1;
            }
            out.push(s);
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// A literal split at `{…}` placeholders (and `{{`/`}}` unescaped), trimmed.
fn pieces(lit: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let cs: Vec<char> = lit.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '{' && i + 1 < cs.len() && cs[i + 1] == '{' {
            cur.push('{');
            i += 2;
            continue;
        }
        if c == '}' && i + 1 < cs.len() && cs[i + 1] == '}' {
            cur.push('}');
            i += 2;
            continue;
        }
        if c == '{' {
            out.push(std::mem::take(&mut cur));
            while i < cs.len() && cs[i] != '}' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == '\n' {
            out.push(std::mem::take(&mut cur));
            i += 1;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    out.push(cur);
    out.into_iter().map(|p| p.trim().to_string()).filter(|p| p.len() >= 4 && p.chars().any(|c| c.is_ascii_alphabetic())).collect()
}

fn load_sources(root: &Path) -> Sources {
    let mut files = Vec::new();
    rs_files(&root.join("crates"), &mut files);
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let name = f.strip_prefix(root).unwrap_or(f).to_string_lossy().replace('\\', "/");
        for lit in literals(&text) {
            for p in pieces(&lit) {
                map.entry(p).or_default().insert(name.clone());
            }
        }
    }
    let frags: Vec<(String, Vec<String>)> = map.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect();
    let mut by_prefix: HashMap<[u8; 4], Vec<usize>> = HashMap::new();
    for (i, (f, _)) in frags.iter().enumerate() {
        let b = f.as_bytes();
        by_prefix.entry([b[0], b[1], b[2], b[3]]).or_default().push(i);
    }
    Sources { frags, by_prefix }
}

impl Sources {
    /// The files whose literals cover `run`: the longest literal inside it first, then
    /// the longest in what is left, and so on; or, when no literal covers much, the
    /// literals that contain the whole run. Each file with the length it covers.
    fn attribute(&self, run: &str) -> Vec<(String, Vec<String>)> {
        let b = run.as_bytes();
        let mut hits: Vec<(usize, usize, usize)> = Vec::new(); // (len, start, frag)
        if b.len() >= 4 {
            for s in 0..=b.len() - 4 {
                if let Some(ix) = self.by_prefix.get(&[b[s], b[s + 1], b[s + 2], b[s + 3]]) {
                    for &i in ix {
                        let f = self.frags[i].0.as_bytes();
                        if b[s..].starts_with(f) {
                            hits.push((f.len(), s, i));
                        }
                    }
                }
            }
        }
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut taken = vec![false; b.len()];
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        let mut covered = 0;
        for (len, s, i) in hits {
            if taken[s..s + len].iter().any(|t| *t) {
                continue;
            }
            // Only spans with a counted word are worth naming.
            let piece = &run[s..s + len];
            if !piece.split(|c: char| !is_latin(c)).any(counts) {
                continue;
            }
            taken[s..s + len].iter_mut().for_each(|t| *t = true);
            covered += len;
            // Short pieces beside a longer one are words dropped into its template.
            if !out.is_empty() && len < 8 {
                continue;
            }
            out.push((self.frags[i].0.clone(), self.frags[i].1.clone()));
        }
        if covered * 2 < b.len() {
            // A short run cut from a longer literal ("south" from "· red · south").
            let within: Vec<&(String, Vec<String>)> = self.frags.iter().filter(|(f, _)| f.len() > b.len() && f.contains(run)).collect();
            if !within.is_empty() {
                let mut files: BTreeSet<String> = BTreeSet::new();
                for (_, fs) in within {
                    files.extend(fs.iter().cloned());
                }
                out.insert(0, (run.to_string(), files.into_iter().collect()));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------------
// The audit.
// ---------------------------------------------------------------------------------

struct Leak {
    func: String,
    field: String,
    lang: String,
    run: String,
    string: String,
}

#[derive(Default)]
struct Audit {
    calls: usize,
    strings: usize,
    leaks: Vec<Leak>,
    kotlin_translated: BTreeMap<String, BTreeSet<String>>,
    errors: Vec<String>,
    source_keys: BTreeSet<String>,
    kotlin_keys: BTreeSet<String>,
    called: BTreeSet<String>,
}

impl Audit {
    fn take(&mut self, func: &str, lang: &str, pairs: Vec<(String, String)>) {
        self.calls += 1;
        self.called.insert(func.to_string());
        for (field, v) in pairs {
            if field == "source_keys" {
                self.source_keys.insert(v.clone());
            }
            if skip_field(&field) || v.trim().is_empty() {
                continue;
            }
            self.strings += 1;
            let t = v.trim();
            if t.starts_with('{') || t.starts_with('[') || is_hexish(t) || is_machine_key(t) {
                continue;
            }
            if keyish_field(&field) && !t.contains(' ') {
                continue;
            }
            if self.kotlin_keys.contains(t) {
                self.kotlin_translated.entry(t.to_string()).or_default().insert(format!("{func}.{field}"));
                continue;
            }
            for run in latin_runs(&v) {
                self.leaks.push(Leak { func: func.into(), field: field.clone(), lang: lang.into(), run, string: v.clone() });
            }
        }
    }

    fn read(&mut self, func: &str, lang: &str, debug: String) {
        let s = strings(&debug);
        self.take(func, lang, s);
    }

    fn run<T: std::fmt::Debug, E: std::fmt::Debug>(&mut self, func: &str, lang: &str, r: Result<T, E>) -> Option<T> {
        match r {
            Ok(v) => {
                self.read(func, lang, format!("{v:?}"));
                Some(v)
            }
            Err(e) => {
                self.called.insert(func.to_string());
                self.errors.push(format!("{func} [{lang}]: {e:?}"));
                None
            }
        }
    }
}

fn ok<T>(v: T) -> Result<T, ()> {
    Ok(v)
}

fn engine() -> Arc<Engine> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
    Engine::open(p.to_string_lossy().into_owned(), None).expect("the kernel is needed for this test")
}

fn at(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> ExactInstant {
    instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: mi, second: 0, nanos: 0 }).unwrap()
}

/// The keys of the app's Kotlin dictionaries (`"English" to "translation"`).
fn kotlin_keys(root: &Path) -> BTreeSet<String> {
    let dir = root.join("android/app/src/main/java/com/kshana/app/data");
    let mut keys = BTreeSet::new();
    for f in ["Lang.kt", "LangExplore.kt", "LangTrust.kt", "LangTrust2.kt"] {
        let Ok(text) = std::fs::read_to_string(dir.join(f)) else { continue };
        for line in text.lines() {
            let mut rest = line;
            while let Some(a) = rest.find('"') {
                let after = &rest[a + 1..];
                let Some(e) = after.find('"') else { break };
                let k = &after[..e];
                let tail = after[e + 1..].trim_start();
                if let Some(t) = tail.strip_prefix("to") {
                    if t.trim_start().starts_with('"') {
                        keys.insert(k.replace("\\'", "'"));
                    }
                }
                rest = &after[e + 1..];
            }
        }
    }
    keys
}

const HS: HouseSystem = HouseSystem::Placidus;
const AY: AyanamsaChoice = AyanamsaChoice::Lahiri;
const LANGS: [&str; 2] = ["ml", "hi"];

#[test]
#[ignore = "an inventory; run with --ignored --nocapture"]
fn lang_leaks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let e = engine();
    let mut a = Audit { kotlin_keys: kotlin_keys(&root), ..Default::default() };
    eprintln!("{} Kotlin dictionary keys", a.kotlin_keys.len());

    // (birth, lat, lon, male)
    let charts = [
        (at(1990, 2, 20, 5, 4), 9.9312, 76.2673, true),     // Kochi
        (at(1985, 7, 3, 12, 0), 28.6139, 77.2090, false),   // Delhi
        (at(1978, 11, 30, 22, 0), 19.0760, 72.8777, true),  // Mumbai
    ];
    let now = at(2026, 10, 22, 5, 0);
    let (hla, hlo) = (9.9312, 76.2673); // where the person is: Kochi
    let off = 330;
    let names = ["Anu", "Ravi", "Meera", "Kiran"];
    let relations = ["Spouse", "Father", "Mother", "Child"];
    let no_weights = || PersonalWeightsInput { answers: 0, techniques: vec![] };
    let lives = life_names();

    for (ci, &(b, la, lo, male)) in charts.iter().enumerate() {
        let (ob, ola, olo, _) = charts[(ci + 1) % charts.len()];
        let chart = a.run("chart", "-", e.chart(b, la, lo, 0.0, HS, AY, false));
        let moon = chart.as_ref().map(|c| c.panchanga.moon_sidereal).unwrap_or(100.0);
        let shukla = chart.as_ref().map(|c| c.panchanga.is_shukla).unwrap_or(true);
        eprintln!("chart {ci}");

        // ---- Functions that take a language: in Malayalam and Hindi.
        for lang in LANGS {
            let l = || lang.to_string();
            // Through the day, so every hour's lord and its lore come up.
            for h in [0u32, 3, 6, 9, 12, 15, 18, 21] {
                a.run("forecast_moment", lang, e.forecast_moment(b, la, lo, hla, hlo, HS, AY, at(2026, 10, 22, h, 17), l()));
            }
            a.run("briefing", lang, e.briefing(b, la, lo, hla, hlo, HS, AY, lives.clone(), now, l()));
            for s in ["day", "week", "month", "year"] {
                a.run("outlook", lang, e.outlook(b, la, lo, hla, hlo, HS, AY, s.into(), now, l(), vec![]));
            }
            a.run("remedy_tree", lang, e.remedy_tree(b, la, lo, hla, hlo, HS, AY, None, now, l(), vec![]));
            for life in &lives {
                a.run("remedy_tree", lang, e.remedy_tree(b, la, lo, hla, hlo, HS, AY, Some(life.clone()), now, l(), vec![]));
            }
            a.run("remedy_schedule", lang, e.remedy_schedule(b, la, lo, hla, hlo, HS, AY, None, now, l(), vec![], off));
            a.run("past_checks", lang, e.past_checks(b, la, lo, HS, AY, now, l(), vec![]));
            a.run("past_reveal", lang, e.past_reveal(b, la, lo, HS, AY, now, l(), 30.0, vec![], vec![], 8, ci % 2 == 0));
            a.run("seal_pick", lang, e.seal_pick(b, la, lo, HS, AY, now, l()));
            a.run("future_personal", lang, e.future_personal(b, la, lo, hla, hlo, HS, AY, now, 24, l(), vec![], no_weights()));
            let firm = a.run("firm_predictions", lang, e.firm_predictions(b, la, lo, hla, hlo, HS, AY, now, l(), None));
            let sure = a.run("sure_windows_trusted", lang, e.sure_windows_trusted(b, la, lo, HS, AY, hla, hlo, off, now, 18, vec![], 16, l(), vec![]));
            if let Some(ws) = &sure {
                for w in ws.iter().take(3) {
                    a.run("sure_window_hours", lang, e.sure_window_hours(b, la, lo, HS, AY, hla, hlo, off, w.area.clone(), w.is_hard, w.start, w.end, now, Some(w.peak), w.system_keys.clone(), l()));
                }
            }
            a.run("sure_next", lang, e.sure_next(b, la, lo, HS, AY, hla, hlo, off, now, 12, l()));
            a.run("chart_facts", lang, e.chart_facts(b, la, lo, HS, AY, now, l(), 30.0, vec![]));
            let group = vec![PersonInput { birth: b, lat: la, lon: lo }, PersonInput { birth: ob, lat: ola, lon: olo }];
            for pu in purposes(l()).into_iter().take(4).map(|p| p.key) {
                a.run("best_slots_together", lang, e.best_slots_together(group.clone(), hla, hlo, HS, AY, pu, now, 7, 4, l()));
            }
            let fam: Vec<FamilyMemberInput> = (0..3)
                .map(|k| {
                    let (rb, rla, rlo, _) = charts[(ci + k + 1) % charts.len()];
                    FamilyMemberInput { name: names[k].into(), relation: relations[(ci + k) % relations.len()].into(), birth: rb, lat: rla, lon: rlo }
                })
                .collect();
            a.run("family_proof", lang, e.family_proof(b, la, lo, Some(male), fam, hla, hlo, HS, AY, now, 24, l()));
            let answers = vec![
                PastAnswerInput { from: at(2015, 1, 1, 0, 0), to: at(2015, 6, 1, 0, 0), life: "Love".into(), good: false, verdict: "yes".into() },
                PastAnswerInput { from: at(2018, 3, 1, 0, 0), to: at(2018, 9, 1, 0, 0), life: "Career".into(), good: true, verdict: "no".into() },
            ];
            a.run("learn_techniques", lang, ok(e.learn_techniques(b, la, lo, HS, AY, answers.clone(), l())));
            a.run("birth_time_now", lang, ok(e.birth_time_now(b, la, lo, HS, AY, now, 30.0, answers, vec![], vec![], off, l())));

            // The trust feeds: seal the firm predictions a year and a half ago, answer
            // some, and read the record, the open test, the blind questions.
            let past = at(2025, 4, 1, 5, 0);
            let old = e.firm_predictions(b, la, lo, hla, hlo, HS, AY, past, l(), None).unwrap_or_default();
            let mut cands = old.clone();
            cands.extend(firm.clone().unwrap_or_default());
            let sealed = seal_new_claims(vec![], cands, past, 12, l(), lo);
            a.read("seal_new_claims", lang, format!("{sealed:?}"));
            let mut resolved = Vec::new();
            for (k, c) in sealed.iter().enumerate() {
                let outcome = ["yes", "no", "close"][k % 3];
                if let Ok(r) = resolve_claim(c.line.clone(), outcome.into(), now) {
                    resolved.push(r);
                }
            }
            let claims: Vec<OpenTestClaimInput> = resolved.iter().map(|c| OpenTestClaimInput { line: c.line.clone(), chance: Some(0.5), used_for_birth_time: false }).collect();
            a.run("open_test_own", lang, ok(open_test_own(claims.clone(), l())));
            a.run("silenced_areas", lang, ok(silenced_areas(claims.clone(), l())));
            a.run("open_test_contribution", lang, open_test_contribution(claims, "lang-leaks-install-salt".into(), l()));
            a.run("blind_answers", lang, ok(blind_answers(l())));
            for c in sealed.iter().take(4) {
                a.read("blind_question", lang, format!("{:?}", blind_question(l(), c.area.clone(), c.window_start, c.window_end, lo)));
                for ans in ["good", "hard", "mixed", "nothing", "unsure"] {
                    a.run("blind_resolve", lang, blind_resolve(c.line.clone(), ans.into(), now, l(), lo));
                    a.run("blind_judge", lang, blind_judge(c.good, ans.into(), c.created_at, c.statement.clone(), l(), lo));
                }
            }
            let lines: Vec<String> = sealed.iter().map(|c| c.line.clone()).collect();
            for (life, good) in [("Career", true), ("Love", false), ("Money", true), ("Health", false)] {
                a.run("match_logged_event", lang, ok(match_logged_event(lines.clone(), life.into(), good, at(2025, 9, 1, 6, 0), now, l(), lo)));
            }
            let votes: Vec<SystemVoteInput> = ["vimshottari", "yogini", "gochara", "kp", "chara", "ashtakavarga"]
                .iter()
                .enumerate()
                .flat_map(|(k, s)| ["yes", "no", "close"].iter().take(1 + k % 3).map(move |o| SystemVoteInput { key: (*s).into(), outcome: (*o).into() }))
                .collect();
            a.run("system_hit_rates", lang, ok(system_hit_rates(votes, l())));
            for p in [0.2, 0.5, 0.7, 0.9] {
                let tier = tier_of_probability(p);
                a.read("tier_of_probability", "-", format!("{tier:?}"));
                for (pts, n) in [(0.0, 0), (60.0, 5), (85.0, 20)] {
                    a.run("confidence_line", lang, ok(confidence_line(tier.clone(), pts, n, l())));
                }
            }
            let signals: Vec<DaySignalInput> = (0..60).map(|d| DaySignalInput { day: 20_600 + d, good: d % 3 != 0 }).collect();
            a.run("remedy_kept_compare", lang, ok(remedy_kept_compare((20_600..20_660).step_by(7).collect(), (20_600..20_660).step_by(14).collect(), signals, 20_660, l())));
            let mut items: Vec<ScoreItemInput> = ["yes", "no", "close", "yes", "no"].iter().map(|o| ScoreItemInput { source: "sealed".into(), outcome: (*o).into(), chance: 0.5, used_for_fitting: false }).collect();
            items.extend(["yes", "no"].iter().map(|o| ScoreItemInput { source: "past".into(), outcome: (*o).into(), chance: 0.5, used_for_fitting: true }));
            items.push(ScoreItemInput { source: "quiz".into(), outcome: "yes".into(), chance: 0.25, used_for_fitting: false });
            a.run("record_card", lang, ok(record_card(items.clone(), 7, 3, l())));
            a.run("record_card", lang, ok(record_card(vec![], 0, 0, l())));
            a.run("scoreboard", lang, ok(scoreboard(items, l())));
            a.run("scoreboard", lang, ok(scoreboard(vec![], l())));
            a.run("forecast_tilt", lang, ok(forecast_tilt(no_weights(), l())));
        }

        // ---- Functions that take no language: once per chart.
        let nl = "-";
        a.run("talk_window", nl, e.talk_window(b, la, lo, ob, ola, olo, hla, hlo, HS, AY, now, 14));
        a.run("slow_horizon", nl, e.slow_horizon(b, la, lo, hla, hlo, HS, AY, now, 12));
        a.run("transits", nl, e.transits(b, la, lo, AY, now, 30));
        a.run("life_map", nl, e.life_map(b, la, lo, AY, 90));
        a.run("ashtakavarga", nl, e.ashtakavarga(b, la, lo, AY));
        a.run("stability", nl, e.stability(b, la, lo, 600, HS, AY));
        for pl in ["Saturn", "Jupiter"] {
            a.run("kakshya_windows", nl, e.kakshya_windows(b, la, lo, AY, pl.into(), now, at(2027, 6, 1, 0, 0)));
        }
        a.run("reading_report_json", nl, Ok::<_, ()>(()));
        if let Ok(js) = e.reading_report_json(b, la, lo, HS, AY, now) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&js) {
                let mut out = Vec::new();
                json_strings(&v, "", &mut out);
                a.take("reading_report_json", nl, out);
            }
        }
        a.run("muhurta", nl, e.muhurta(["Marriage", "Travel", "Business"][ci].into(), now, 14, hla, hlo, AY, Some(moon), 5));
        a.run("dasha_table", nl, ok(dasha_table(moon, b)));
        a.run("dasha_chain", nl, ok(dasha_chain(moon, b, now, 5)));

        // ---- The rest of the chart functions, in Malayalam and Hindi.
        for lang in LANGS {
            let l = || lang.to_string();
            let (hb, hla2, hlo2) = (b, la, lo);
            for pu in purposes(l()) {
                a.run("appointment", lang, e.appointment(hb, hla2, hlo2, hla, hlo, HS, AY, pu.key.clone(), now, l()));
                a.run("best_slots", lang, e.best_slots(hb, hla2, hlo2, hla, hlo, HS, AY, pu.key.clone(), now, 7, 4, l()));
            }
            let qs = questions(l());
            for (i, life) in lives.iter().enumerate() {
                for (qi, q) in qs.iter().enumerate() {
                    if ci == 0 || (qi + i + ci) % 3 == 0 {
                        a.run("ask", lang, e.ask(b, la, lo, hla, hlo, HS, AY, life.clone(), q.key.clone(), now, l()));
                    }
                }
                a.run("explain", lang, e.explain(b, la, lo, hla, hlo, HS, AY, life.clone(), now, l()));
            }
            for (i, tp) in topics(l()).into_iter().enumerate() {
                for (qi, q) in qs.iter().enumerate() {
                    if ci == 0 || (qi + i + ci) % 3 == 0 {
                        a.run("ask_topic", lang, e.ask_topic(b, la, lo, hla, hlo, HS, AY, tp.key.clone(), q.key.clone(), now, l()));
                    }
                }
            }
            for q in &qs {
                a.run("couple_ask", lang, e.couple_ask(b, la, lo, ob, ola, olo, hla, hlo, HS, AY, q.key.clone(), now, l()));
            }
            a.run("couple", lang, e.couple(b, la, lo, ob, ola, olo, hla, hlo, HS, AY, male, now, l()));
            a.run("kundali", lang, e.kundali(b, la, lo, ob, ola, olo, HS, AY, male, l()));
            a.run("compatibility", lang, e.compatibility(b, la, lo, ob, ola, olo, AY, l()));
            a.run("countdowns", lang, e.countdowns(b, la, lo, hla, hlo, HS, AY, now, l()));
            a.run("festivals_here", lang, e.festivals_here(b, la, lo, HS, AY, hla, hlo, off, now, 60, l()));
            a.run("letter", lang, e.letter(b, la, lo, hla, hlo, HS, AY, now, vec![], l()));
            a.run("year_chapter", lang, e.year_chapter(b, la, lo, hla, hlo, HS, AY, now, l()));
            a.run("timeline", lang, e.timeline(b, la, lo, HS, AY, now, l()));
            a.run("forecast_horizon", lang, e.forecast_horizon(b, la, lo, hla, hlo, HS, AY, now, 1.0, 14, l()));
            a.run("plain_reading", lang, e.plain_reading(b, la, lo, AY, now, l()));
            a.run("chart_yogas", lang, e.chart_yogas(b, la, lo, AY, now, l()));
            a.run("parihara", lang, e.parihara(b, la, lo, AY, now, None, l()));
            for life in &lives {
                a.run("parihara", lang, e.parihara(b, la, lo, AY, now, Some(life.clone()), l()));
            }
            a.run("daily", lang, e.daily(b, la, lo, AY, now, hla, hlo, l()));
            a.run("karakas", lang, e.karakas(b, la, lo, AY, l()));
            a.run("area_readings", lang, e.area_readings(b, la, lo, 0.0, HS, AY, now, 3, l()));
            a.run("aspects", lang, e.aspects(b, la, lo, 8.0, l()));
            a.run("shadbala", lang, e.shadbala(b, la, lo, AY, l()));
            a.run("vimshopaka_bala", lang, e.vimshopaka_bala(b, la, lo, AY, l()));
            a.run("varshaphala", lang, e.varshaphala(b, la, lo, AY, 2026, hla, hlo, l()));
            a.run("jaimini_extras", lang, e.jaimini_extras(b, la, lo, AY, l()));
            a.run("dasha_sandhi_windows", lang, e.dasha_sandhi_windows(b, la, lo, AY, at(2026, 1, 1, 0, 0), at(2036, 1, 1, 0, 0), None, l()));
            for lord in ["Sun", "Moon", "Saturn", "Rahu"] {
                a.run("dasha_chidra_lords", lang, e.dasha_chidra_lords(b, la, lo, AY, lord.into(), l()));
            }
            a.run("dasha_system", lang, e.dasha_system(b, la, lo, AY, "vimshottari".into(), now, l()));
            a.run("dasha_systems_at", lang, e.dasha_systems_at(b, la, lo, AY, now, l()));
            a.run("dasha_systems_more_at", lang, e.dasha_systems_more_at(b, la, lo, AY, now, l()));
            a.run("kp_full", lang, e.kp_full(b, la, lo, AY, now, hla, hlo, l()));
            a.run("profections", lang, e.profections(b, la, lo, AY, now, l()));
            a.run("firdaria", lang, e.firdaria(b, la, lo, AY, l()));
            a.run("progressions", lang, e.progressions(b, la, lo, AY, now, l()));
            a.run("progressed_hits", lang, e.progressed_hits(b, la, lo, AY, now, at(2031, 1, 1, 0, 0), l()));
            for lot in ["fortune", "spirit"] {
                a.run("zodiacal_releasing", lang, e.zodiacal_releasing(b, la, lo, AY, lot.into(), 2, at(2027, 10, 22, 0, 0), l()));
            }
            a.run("retrodict", lang, e.retrodict(b, la, lo, AY, at(2010, 1, 1, 0, 0), at(2026, 1, 1, 0, 0), l()));
            a.run("transit_windows", lang, e.transit_windows(b, la, lo, AY, now, at(2028, 1, 1, 0, 0), l()));
            a.run("transit_score", lang, e.transit_score(b, la, lo, AY, now, l()));
            a.run("snapshot", lang, e.snapshot(b, moon, hla, hlo, AY, now, 9, l()));
            a.run("trace", lang, e.trace(b, moon, hla, hlo, AY, now, at(2026, 10, 25, 0, 0), 5, Finest::Sub, Finest::Sign, true, true, Finest::Nakshatra, l()));
            let ledger: Vec<LedgerEventRecord> = vec![
                LedgerEventRecord { at: at(2012, 6, 1, 0, 0), area: "Career".into(), kind: "Began".into(), weight: 1.0, note: "".into() },
                LedgerEventRecord { at: at(2016, 2, 14, 0, 0), area: "Marriage".into(), kind: "Milestone".into(), weight: 1.0, note: "".into() },
                LedgerEventRecord { at: at(2019, 9, 1, 0, 0), area: "Wealth".into(), kind: "Loss".into(), weight: 1.0, note: "".into() },
                LedgerEventRecord { at: at(2021, 3, 1, 0, 0), area: "Property".into(), kind: "Gain".into(), weight: 1.0, note: "".into() },
                LedgerEventRecord { at: at(2023, 11, 1, 0, 0), area: "Health".into(), kind: "Crisis".into(), weight: 1.0, note: "".into() },
            ];
            let (lf, lt) = (b, now);
            a.run("score_events", lang, e.score_events(b, la, lo, AY, ledger.clone(), l()));
            a.run("calibrate_ledger", lang, e.calibrate_ledger(b, la, lo, AY, ledger.clone(), lf, lt, 20, l()));
            a.run("coverage_tally", lang, e.coverage_tally(b, la, lo, AY, ledger.clone(), lf, lt, l()));
            a.run("ledger_metrics", lang, e.ledger_metrics(b, la, lo, AY, ledger.clone(), lf, lt, 20, 0, l()));
            a.run("time_shift_test", lang, e.time_shift_test(b, la, lo, AY, ledger.clone(), lf, lt, 8, l()));
            a.run("birth_posterior", lang, e.birth_posterior(b, la, lo, AY, 1800, 300, ledger.clone(), l()));
            let quiz = a.run("retrodiction_quiz", lang, e.retrodiction_quiz(b, la, lo, AY, "Career".into(), at(2005, 1, 1, 0, 0), now, 4, 3, l()));
            if let Some(qz) = quiz {
                let n = qz.len();
                a.run("grade_quiz", lang, ok(grade_quiz(qz, vec![0; n], l())));
            }
            if let Some(sf) = a.run("seal_forecast", lang, e.seal_forecast(b, la, lo, AY, now, "Career".into(), "Began".into(), at(2026, 11, 1, 0, 0), at(2027, 3, 1, 0, 0), l())) {
                if let Ok(js) = sealed_forecast_to_json(sf) {
                    a.run("resolve_forecast", nl, resolve_forecast(js, true, at(2027, 1, 1, 0, 0)));
                }
            }
            let lev = vec![
                LifeEventRecord { at: at(2012, 6, 1, 0, 0), life: "Career".into(), good: true },
                LifeEventRecord { at: at(2016, 2, 14, 0, 0), life: "Love".into(), good: true },
                LifeEventRecord { at: at(2019, 9, 1, 0, 0), life: "Money".into(), good: false },
            ];
            a.run("rectify", lang, ok(e.rectify(la, lo, HS, AY, at(1990, 2, 20, 4, 0), at(1990, 2, 20, 6, 0), lev, 5, l())));
            let kev = vec![
                KpLifeEventRecord { area: "Career".into(), at: at(2012, 6, 1, 0, 0), label: "first job".into() },
                KpLifeEventRecord { area: "Marriage".into(), at: at(2016, 2, 14, 0, 0), label: "wedding".into() },
            ];
            a.run("rectify_kp", lang, e.rectify_kp(b, 1800, 300, la, lo, AY, kev, l()));
            let shapes = [QuestionShape::WillItHappen, QuestionShape::When, QuestionShape::Choice, QuestionShape::WillItReturn, QuestionShape::WillTheyRecover];
            for (ai, area) in ["Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Family", "Foreign"].into_iter().enumerate() {
                let shape = shapes[(ai + ci) % shapes.len()];
                a.run("prashna", lang, e.prashna(now, hla, hlo, Some((ci * 40 + ai * 7 + 11) as u16), area.into(), shape, AY, 15, l()));
            }
            a.run("prasna_marga_sphutas", lang, e.prasna_marga_sphutas(now, hla, hlo, AY, l()));
            a.run("prasna_arudha_sphutas", lang, e.prasna_arudha_sphutas(now, hla, hlo, AY, (ci * 4) as u8, l()));
            a.run("dasha_sensitivity", lang, ok(dasha_sensitivity(moon, 13.2, 600.0, l())));
            a.run("naming", lang, ok(naming(moon, l())));
            a.read("birth_bird", lang, format!("{:?}", birth_bird(moon, shukla, l())));
            for &r in &[100.0, 250.0] {
                a.run("remedy_how_it_helps", lang, ok(remedy_how_it_helps(["Saturn", "Rahu"][(r as usize) % 2].into(), "dasha".into(), None, l())));
            }
            for planet in ["Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu"] {
                for (k, source) in ["dasha", "gochara", "natal"].into_iter().enumerate() {
                    if (k + ci) % 3 == 0 {
                        a.read("remedy_how_it_helps", lang, format!("{:?}", remedy_how_it_helps(planet.into(), source.into(), lives.get(k).cloned(), l())));
                    }
                }
            }
        }
    }

    // ---- Place and day functions (no birth).
    let nl = "-";
    let (la, lo) = (9.9312, 76.2673);
    a.run("panchanga_detail", nl, e.panchanga_detail(now, la, lo, AY));
    a.run("next_events", nl, ok(e.next_events(now, 14, AY)));
    a.run("eclipses", nl, e.eclipses(now, at(2030, 1, 1, 0, 0), la, lo));
    a.run("nazhika", nl, ok(nazhika(at(2026, 10, 22, 0, 45), now)));
    a.run("self_test", nl, ok(e.self_test()));
    a.run("ledger_event_kinds", nl, ok(ledger_event_kinds()));
    a.run("hora_lore_all", nl, ok(hora_lore_all()));
    a.run("open_test_protocol", nl, ok(open_test_protocol()));
    a.run("language_names", nl, ok(language_names()));
    a.read("vakya_note", nl, format!("{:?}", vakya_note()));
    for lang in LANGS {
        let l = || lang.to_string();
        a.run("day_clock", lang, e.day_clock(now, la, lo, AY, l()));
        a.run("pancha_pakshi", lang, e.pancha_pakshi(charts[0].0, AY, now, la, lo, l()));
        for k in ["gandanta", "panchaka", "bhadra", "yoga", "tara", "chandra"] {
            a.run("moon_windows", lang, e.moon_windows(k.into(), AY, 100.0, now, at(2026, 11, 22, 0, 0), l()));
        }
        a.run("calendar", lang, e.calendar(now, 30, la, lo, AY, l()));
        a.run("calendar_days", lang, e.calendar_days(la, lo, off, 2026, 10, 1, 31, AY, l()));
        a.run("nazhika_to_instant", lang, e.nazhika_to_instant(2026, 10, 22, off * 60, la, lo, 0.0, 10, 30, true, false, l()));
        a.run("error_budget", lang, ok(error_budget(l())));
        a.run("dasha_level_names", lang, ok(dasha_level_names(l())));
        a.run("glossary", lang, ok(glossary(l())));
        a.run("questions", lang, ok(questions(l())));
        a.run("purposes", lang, ok(purposes(l())));
        a.run("topics", lang, ok(topics(l())));
        for n in [111, 345, 678, 888] {
            a.run("ashtamangala_number", lang, ok(ashtamangala_number(n, l())));
        }
        let recs: Vec<RecordInput> = (0..30).map(|k| RecordInput { life: life_names()[k % 9].clone(), predicted: (k * 7 % 100) as f64, happened: k % 3 != 0 }).collect();
        a.run("calibrate", lang, ok(calibrate(recs, l())));
        a.run("calibrate", lang, ok(calibrate(vec![], l())));
        for s in ["when will i get a job", "kalyanam eppo", "मेरी शादी कब होगी", "എന്റെ വിവാഹം എപ്പോൾ", "is this a good week to travel", "my mother is ill", "money worries"] {
            a.run("understand", lang, ok(understand(s.into(), l())));
        }
        for k in ledger_event_kinds() {
            a.read("ledger_event_kind_name", lang, format!("{:?}", ledger_event_kind_name(k, l())));
        }
        for c in [0.2, 0.5, 0.75, 0.9, 0.97] {
            a.read("sure_words", lang, format!("{:?}", sure_words(l(), c)));
        }
    }
    for lang in LANGS {
        a.run("lexicon", lang, ok(lexicon(lang.into())));
        let keys: Vec<String> = a.source_keys.iter().cloned().collect();
        a.run("source_lines", lang, ok(source_lines(keys, lang.into())));
    }

    // ---- Every function the app calls was reached.
    let missing: Vec<&str> = FNS.iter().map(|f| f.0).filter(|f| !a.called.contains(*f)).collect();
    eprintln!("calls {}, strings read {}, leaks {}, errors {}", a.calls, a.strings, a.leaks.len(), a.errors.len());
    for er in &a.errors {
        eprintln!("  error: {er}");
    }
    assert!(missing.is_empty(), "not called: {missing:?}");
    assert!(a.strings > 5000, "only {} strings read", a.strings);

    report(&root, &a);
}

fn report(root: &Path, a: &Audit) {
    let src = load_sources(root);
    eprintln!("{} literal fragments in the crates", src.frags.len());
    let out_dir = std::env::var("KSHANA_LEAKS_OUT").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());

    // Attribute each distinct run once: the literals that cover it.
    let mut attr: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();
    for l in &a.leaks {
        if !attr.contains_key(&l.run) {
            attr.insert(l.run.clone(), src.attribute(&l.run));
        }
    }
    // A literal found in a few files goes to the one that owns most of the leaks found
    // in one file only; a literal in many files is a shared word (a planet, a weekday).
    let mut owned: HashMap<String, usize> = HashMap::new();
    for frs in attr.values() {
        for (_, fs) in frs {
            if fs.len() == 1 {
                *owned.entry(fs[0].clone()).or_default() += 1;
            }
        }
    }
    const SHARED: &str = "(shared words: the same literal in five or more files)";
    const UNFOUND: &str = "(not found as a literal: built at run time or read from data)";
    let owner = |fs: &Vec<String>| -> (String, Vec<String>) {
        if fs.len() > 4 {
            return (SHARED.to_string(), vec![]);
        }
        let best = fs.iter().max_by_key(|f| (owned.get(*f).copied().unwrap_or(0), std::cmp::Reverse((*f).clone()))).cloned().unwrap_or_default();
        let others = fs.iter().filter(|f| **f != best).cloned().collect();
        (best, others)
    };

    #[derive(Default)]
    struct FileGroup {
        /// Distinct leaked literals, each with one full string it was seen in.
        lits: BTreeMap<String, String>,
        via: BTreeMap<String, BTreeSet<String>>, // function -> fields
        also: BTreeSet<String>,
    }
    let mut groups: BTreeMap<String, FileGroup> = BTreeMap::new();
    let mut tsv = String::from("file\tliteral\tfunction\tfield\tlang\trun\tstring\n");
    let mut seen_rows: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    for l in &a.leaks {
        let frs = &attr[&l.run];
        let mut keyed: Vec<(String, String, Vec<String>)> = Vec::new();
        if frs.is_empty() {
            keyed.push((UNFOUND.to_string(), norm(&l.run), vec![]));
        }
        for (lit, fs) in frs {
            let (o, others) = owner(fs);
            keyed.push((o, lit.clone(), others));
        }
        for (file, lit, others) in keyed {
            let g = groups.entry(file.clone()).or_default();
            g.lits.entry(lit.clone()).or_insert_with(|| l.string.clone());
            g.via.entry(l.func.clone()).or_default().insert(l.field.clone());
            g.also.extend(others);
            if seen_rows.insert((file.clone(), lit.clone(), l.func.clone(), l.field.clone())) {
                tsv.push_str(&format!("{file}\t{lit}\t{}\t{}\t{}\t{}\t{}\n", l.func, l.field, l.lang, l.run.replace('\t', " "), l.string.replace(['\t', '\n'], " ")));
            }
        }
    }
    let _ = std::fs::write(out_dir.join("kshana_lang_leaks.tsv"), &tsv);

    let mut order: Vec<(&String, &FileGroup)> = groups.iter().collect();
    order.sort_by(|x, y| y.1.lits.len().cmp(&x.1.lits.len()).then(x.0.cmp(y.0)));
    let mut txt = String::new();
    let total: BTreeSet<String> = a.leaks.iter().map(|l| norm(&l.run)).collect();
    let lits: usize = groups.values().map(|g| g.lits.len()).sum();
    txt.push_str(&format!(
        "{lits} distinct leaked literals in {} files ({} distinct runs of English as shown); {} calls, {} strings read\n\n",
        groups.len(),
        total.len(),
        a.calls,
        a.strings
    ));
    for (file, g) in &order {
        let mut lang_fns = Vec::new();
        let mut nolang_fns = Vec::new();
        let mut screens = BTreeSet::new();
        for (f, fields) in &g.via {
            let (has, sc) = fn_info(f);
            screens.insert(sc);
            let s = format!("{f}[{}]", fields.iter().cloned().collect::<Vec<_>>().join(","));
            if has { lang_fns.push(s) } else { nolang_fns.push(s) }
        }
        txt.push_str(&format!("== {file}: {} distinct\n", g.lits.len()));
        txt.push_str(&format!("   screens: {}\n", screens.into_iter().collect::<Vec<_>>().join(", ")));
        if !lang_fns.is_empty() {
            txt.push_str(&format!("   via functions that take a lang: {}\n", lang_fns.join("; ")));
        }
        if !nolang_fns.is_empty() {
            txt.push_str(&format!("   via functions with no lang: {}\n", nolang_fns.join("; ")));
        }
        if !g.also.is_empty() {
            txt.push_str(&format!("   some literals also in: {}\n", g.also.iter().cloned().collect::<Vec<_>>().join(", ")));
        }
        let limit = if file.starts_with('(') { 40 } else { 12 };
        for (k, (lit, full)) in g.lits.iter().enumerate() {
            if k >= limit {
                txt.push_str(&format!("   … {} more\n", g.lits.len() - limit));
                break;
            }
            if full.trim() == lit.trim() {
                txt.push_str(&format!("   - {lit}\n"));
            } else {
                let short: String = full.chars().take(160).collect();
                txt.push_str(&format!("   - {lit}    ⟵ in: {short}\n"));
            }
        }
        txt.push('\n');
    }
    txt.push_str("== Translated whole by the Kotlin dictionaries (shown translated if the screen calls Lang.t on them):\n");
    for (s, via) in &a.kotlin_translated {
        txt.push_str(&format!("   - {s}   [{}]\n", via.iter().cloned().collect::<Vec<_>>().join(", ")));
    }
    let _ = std::fs::write(out_dir.join("kshana_lang_leaks_by_file.txt"), &txt);
    eprintln!("{}", txt.lines().filter(|l| l.starts_with("== ")).collect::<Vec<_>>().join("\n"));
    eprintln!("written: {} and {}", out_dir.join("kshana_lang_leaks.tsv").display(), out_dir.join("kshana_lang_leaks_by_file.txt").display());
    if std::env::var("KSHANA_LEAKS_STRICT").is_ok() {
        assert!(a.leaks.is_empty(), "{} leaks remain", total.len());
    }
}

#[test]
fn the_run_finder_reads_latin_words() {
    assert_eq!(latin_runs("Subrahmanya, and Hanuman · red · south"), vec!["Subrahmanya, and Hanuman", "red", "south"]);
    assert!(latin_runs("ഇന്ന് നല്ല ദിവസം 12:30").is_empty());
    assert_eq!(latin_runs("ഇന്ന് Jupiter ശക്തൻ"), vec!["Jupiter"]);
    assert!(latin_runs("KP · BPHS 46 · Kshana").is_empty());
    assert!(is_machine_key("money") && is_machine_key("rule.kp_promise") && !is_machine_key("An even stretch"));
    assert_eq!(pieces("A testing time for {} runs until {date}."), vec!["A testing time for", "runs until"]);
    assert_eq!(literals(r#"let c = '"'; let s = "a \"b\" c"; // "no""#), vec!["a \"b\" c"]);
}
