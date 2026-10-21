//! Phase 4 safety audit: the app never foretells a death, a serious or terminal illness,
//! an accident that harms, or a divorce or separation, for the person or anyone in the
//! family, in English, Malayalam or Hindi, on any screen.
//!
//! How it works: every engine function that produces words is called for twenty birth
//! charts (1940–2010, eleven cities in India and four abroad, chosen so that the hard
//! cases come up: the Mars marriage warning, the lonely-Moon combination, the long
//! Saturn test, a same-Nadi match, eclipse births), at several dates, in all three
//! languages where the function takes one. Every string in every returned record is
//! read (the Debug rendering, decoded back to the script the person reads) and checked
//! against [`BANNED`]. Fields that hold only stable keys ([`KEYS`]) are skipped; the
//! expert pages behind "Show the chart pages" are checked too.
//!
//! Health may appear only as a time to take extra care, with a remedy; marriage
//! difficulty only as strain, with care. Longevity (ayurdaya) is never shown: the
//! Kotlin screens must not call `ayurdayaClassical` (checked by
//! [`kotlin_never_shows_longevity_or_the_banned_words`], which also reads every string
//! literal in the app's Kotlin and `strings.xml`).
//!
//! Run: `cargo test --release -p kshana-ffi --test safety_audit`. Set
//! `KSHANA_SAFETY_FULL=1` for every question on every chart (slow). Hits are written to
//! `kshana_safety_audit_*.txt` in the temp directory.
//!
//! # The banned list
//!
//! Matching is case-insensitive. `Word` must be a whole word; `Start` must begin a word
//! (Malayalam and Hindi add endings, so the stem is enough); `Sub` matches anywhere.
//! Word boundaries know the Malayalam and Devanagari blocks, so "स्मरण" (remembrance)
//! does not hit "मरण" and "സ്മരണ" does not hit "മരണ".
//!
//! English: death, dead, die/dies/died/dying, deadly, fatal, terminal, cancer (the
//! illness; the zodiac sign written "Cancer" with a capital C is allowed, since the
//! jargon audit already keeps sign names off the plain screens), tumour/tumor,
//! accident(s)/accidental (banned outright: say "take care on the road"), injury,
//! injuries, injured, injure, divorce(d), separation, separated, widow(ed), widower,
//! widowhood, maraka, longevity, lifespan, "life span", "short life", demise, suicide,
//! suicidal, mortality, "loss of spouse/husband/wife/partner", kill/killed.
//!
//! Hindi (Devanagari): मृत्यु (death; the mantra name महामृत्युंजय / मृत्युंजय,
//! "victory over death", a prayer for health, is allowed), मौत (death), मरण (dying),
//! मर जा- / मरने / मरेगा / मरेगी (to die), तलाक (divorce), विधवा (widow), विधुर
//! (widower), अलगाव (separation; banned outright, since in these feeds it can only mean
//! a couple apart), दुर्घटना (accident), हादसा (mishap), कैंसर (cancer), ट्यूमर
//! (tumour), चोट (injury; whole word, so चोटी "peak" passes), घायल (injured), घातक
//! (fatal), जानलेवा (deadly), आत्महत्या (suicide), आयुर्दाय (longevity reckoning),
//! अल्पायु (short-lived). Plain आयु is allowed: it also means "age".
//!
//! Malayalam: മരണ (death), മരിക്ക / മരിച്ച (to die), മൃത്യു (death; മൃത്യുഞ്ജയ, the
//! mantra, is allowed), വിവാഹമോചന (divorce), വിധവ (widow), വിഭാര്യ (widower),
//! വേർപിരി (separation), വേർപാട (bereavement), അപകട (accident; also "danger", banned
//! outright as asked), അർബുദ (cancer), കാൻസർ (cancer), പരിക്ക (injury), മാരക (fatal),
//! ആത്മഹത്യ (suicide), ആയുസ്സ / ആയുസ് (lifespan), ആയുർദായ (longevity reckoning).

use std::sync::Arc;

use kshana_ffi::dashas_more::more_dasha_system_names;
use kshana_ffi::family::FamilyMemberInput;
use kshana_ffi::trust::{claim_question, forecast_tilt, scoreboard, PersonalWeightsInput, ScoreItemInput};
use kshana_ffi::yogas::yoga_catalogue;
use kshana_ffi::*;

#[derive(Clone, Copy, PartialEq)]
enum M {
    /// The whole word.
    Word,
    /// The start of a word.
    Start,
    /// Anywhere.
    Sub,
}

/// The banned words, `(word, how it matches, gloss)`.
const BANNED: &[(&str, M, &str)] = &[
    // English
    ("death", M::Word, "death"),
    ("deaths", M::Word, "death"),
    ("dead", M::Word, "dead"),
    ("deadly", M::Word, "deadly"),
    ("die", M::Word, "to die"),
    ("dies", M::Word, "to die"),
    ("died", M::Word, "to die"),
    ("dying", M::Word, "to die"),
    ("fatal", M::Word, "fatal"),
    ("terminal", M::Word, "terminal illness"),
    ("cancer", M::Word, "the illness (capital-C Cancer, the sign, allowed)"),
    ("tumour", M::Word, "tumour"),
    ("tumor", M::Word, "tumour"),
    ("accident", M::Word, "accident"),
    ("accidents", M::Word, "accident"),
    ("accidental", M::Word, "accident"),
    ("injury", M::Word, "injury"),
    ("injuries", M::Word, "injury"),
    ("injured", M::Word, "injury"),
    ("injure", M::Word, "injury"),
    ("divorce", M::Word, "divorce"),
    ("divorced", M::Word, "divorce"),
    ("separation", M::Word, "separation"),
    ("separated", M::Word, "separation"),
    ("widow", M::Word, "widow"),
    ("widowed", M::Word, "widow"),
    ("widower", M::Word, "widower"),
    ("widowhood", M::Word, "widowhood"),
    ("maraka", M::Word, "the death-inflicting planet"),
    ("longevity", M::Word, "longevity"),
    ("lifespan", M::Word, "lifespan"),
    ("life span", M::Sub, "lifespan"),
    ("short life", M::Sub, "short life"),
    ("demise", M::Word, "demise"),
    ("suicide", M::Word, "suicide"),
    ("suicidal", M::Word, "suicide"),
    ("mortality", M::Word, "mortality"),
    ("kill", M::Word, "to kill"),
    ("killed", M::Word, "to kill"),
    ("loss of spouse", M::Sub, "loss of spouse"),
    ("loss of the spouse", M::Sub, "loss of spouse"),
    ("loss of husband", M::Sub, "loss of husband"),
    ("loss of wife", M::Sub, "loss of wife"),
    ("loss of a partner", M::Sub, "loss of a partner"),
    // Hindi
    ("मृत्यु", M::Start, "death"),
    ("मौत", M::Sub, "death"),
    ("मरण", M::Start, "dying"),
    ("मर जा", M::Start, "to die"),
    ("मरने", M::Word, "to die"),
    ("मरेगा", M::Word, "will die"),
    ("मरेगी", M::Word, "will die"),
    ("तलाक", M::Sub, "divorce"),
    ("विधवा", M::Sub, "widow"),
    ("विधुर", M::Sub, "widower"),
    ("अलगाव", M::Sub, "separation"),
    ("दुर्घटना", M::Sub, "accident"),
    ("हादसा", M::Sub, "mishap"),
    ("हादसे", M::Sub, "mishap"),
    ("कैंसर", M::Sub, "cancer"),
    ("ट्यूमर", M::Sub, "tumour"),
    ("चोट", M::Word, "injury"),
    ("चोटें", M::Word, "injuries"),
    ("चोटों", M::Word, "injuries"),
    ("घायल", M::Sub, "injured"),
    ("घातक", M::Sub, "fatal"),
    ("जानलेवा", M::Sub, "deadly"),
    ("आत्महत्या", M::Sub, "suicide"),
    ("आयुर्दाय", M::Sub, "longevity reckoning"),
    ("अल्पायु", M::Sub, "short-lived"),
    // Malayalam
    ("മരണ", M::Start, "death"),
    ("മരിക്ക", M::Start, "to die"),
    ("മരിച്ച", M::Start, "died"),
    ("മൃത്യു", M::Start, "death"),
    ("വിവാഹമോചന", M::Sub, "divorce"),
    ("വിധവ", M::Sub, "widow"),
    ("വിഭാര്യ", M::Sub, "widower"),
    ("വേർപിരി", M::Sub, "separation"),
    ("വേർപാട", M::Sub, "bereavement"),
    ("അപകട", M::Sub, "accident, danger"),
    ("അർബുദ", M::Sub, "cancer"),
    ("കാൻസർ", M::Sub, "cancer"),
    ("പരിക്ക", M::Sub, "injury"),
    ("മാരക", M::Start, "fatal"),
    ("ആത്മഹത്യ", M::Sub, "suicide"),
    ("ആയുസ്സ", M::Sub, "lifespan"),
    ("ആയുസ്", M::Sub, "lifespan"),
    ("ആയുർദായ", M::Sub, "longevity reckoning"),
];

/// Names that contain a banned stem and mean something else: the Mahamrityunjaya
/// mantra ("victory over death"), a prayer for health and long life.
const ALLOWED_NAMES: &[&str] = &["महामृत्युंजय", "मृत्युंजय", "മഹാമൃത്യുഞ്ജയ", "മൃത്യുഞ്ജയ"];

/// Fields that hold stable keys the person never reads (topic keys such as
/// "separation", life keys, technique keys, ids).
const KEYS: &[&str] = &[
    "key", "keys", "area", "also", "kind", "tier", "system_keys", "life", "topic", "id", "source", "rule_id", "rule_ids", "purpose", "verdict", "band",
    "trend", "best_key", "witness_keys", "relation", "signature", "provenance_engine", "provenance_kernel", "provenance_models", "unit", "status", "intent", "reason",
];

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || ('\u{0900}'..='\u{097F}').contains(&c) || ('\u{0D00}'..='\u{0D7F}').contains(&c) || c == '\u{200C}' || c == '\u{200D}'
}

/// The first banned word in `text`, with its gloss.
fn banned_in(text: &str) -> Option<(&'static str, &'static str)> {
    let mut t = text.to_string();
    for n in ALLOWED_NAMES {
        t = t.replace(n, " ");
    }
    let low = t.to_lowercase();
    for &(w, m, gloss) in BANNED {
        let mut from = 0;
        while let Some(k) = low[from..].find(w) {
            let s = from + k;
            let e = s + w.len();
            from = s + w.chars().next().map_or(1, char::len_utf8);
            let before_ok = low[..s].chars().next_back().map_or(true, |c| !word_char(c));
            let after_ok = low[e..].chars().next().map_or(true, |c| !word_char(c));
            let hit = match m {
                M::Sub => true,
                M::Start => before_ok,
                M::Word => before_ok && after_ok,
            };
            if !hit {
                continue;
            }
            // "Cancer" with a capital C is the zodiac sign.
            if w == "cancer" && &t[s..e] == "Cancer" {
                continue;
            }
            return Some((w, gloss));
        }
    }
    None
}

/// `(field, value)` for every string in a Debug rendering, decoded (Debug escapes
/// Malayalam and Devanagari vowel signs as `\u{..}`).
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

/// What was read: how many strings, and the hits.
#[derive(Default)]
struct Audit {
    strings: usize,
    calls: usize,
    hits: Vec<String>,
    /// Seconds spent per function, to keep the audit quick.
    time: std::collections::BTreeMap<String, f64>,
}

impl Audit {
    fn read(&mut self, what: &str, debug: String) {
        self.calls += 1;
        for (f, v) in strings(&debug) {
            if KEYS.contains(&f.as_str()) {
                continue;
            }
            self.strings += 1;
            if let Some((w, gloss)) = banned_in(&v) {
                self.hits.push(format!("{what}.{f} [{w}: {gloss}]: {v}"));
            }
        }
    }

    fn read_ok<T: std::fmt::Debug, E>(&mut self, what: &str, r: Result<T, E>) {
        if let Ok(v) = r {
            self.read(what, format!("{v:?}"));
        }
    }

    /// Call `f`, time it (by the function's part of `what`), and read what it returns.
    fn run<T: std::fmt::Debug, E>(&mut self, what: &str, f: impl FnOnce() -> Result<T, E>) {
        let t0 = std::time::Instant::now();
        let r = f();
        let key = what.rsplit('.').next().unwrap_or(what).split('_').next().unwrap_or(what).to_string();
        *self.time.entry(key).or_default() += t0.elapsed().as_secs_f64();
        self.read_ok(what, r);
    }

    fn finish(mut self, name: &str, min_strings: usize) {
        self.hits.sort();
        self.hits.dedup();
        eprintln!("safety audit {name}: {} calls, {} strings read, {} hits", self.calls, self.strings, self.hits.len());
        let mut slow: Vec<_> = self.time.iter().collect();
        slow.sort_by(|a, b| b.1.total_cmp(a.1));
        eprintln!("slowest: {:?}", slow.iter().take(12).map(|(k, s)| format!("{k} {s:.1}s")).collect::<Vec<_>>());
        assert!(self.strings >= min_strings, "{name}: only {} strings were read; the audit is not reaching the text", self.strings);
        if !self.hits.is_empty() {
            let path = std::env::temp_dir().join(format!("kshana_safety_audit_{name}.txt"));
            let _ = std::fs::write(&path, self.hits.join("\n"));
            panic!("{} banned words in {name} (all in {}); first 60:\n{}", self.hits.len(), path.display(), self.hits.iter().take(60).cloned().collect::<Vec<_>>().join("\n"));
        }
    }
}

fn engine() -> Arc<Engine> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
    Engine::open(p.to_string_lossy().into_owned(), None).expect("the kernel is needed for this test")
}

fn at(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> ExactInstant {
    instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: mi, second: 0, nanos: 0 }).unwrap()
}

/// Twenty births, `(UTC instant, latitude, longitude, born male)`.
fn charts() -> Vec<(ExactInstant, f64, f64, bool)> {
    vec![
        (at(1940, 8, 15, 3, 30), 19.0760, 72.8777, true),  // Mumbai
        (at(1944, 1, 26, 22, 0), 22.5726, 88.3639, false), // Kolkata
        (at(1948, 11, 3, 7, 15), 13.0827, 80.2707, true),  // Chennai
        (at(1952, 5, 19, 12, 40), 28.6139, 77.2090, false), // Delhi
        (at(1955, 12, 31, 18, 5), 8.5241, 76.9366, true),  // Thiruvananthapuram
        (at(1959, 7, 7, 1, 20), 25.3176, 82.9739, false),  // Varanasi
        (at(1962, 2, 4, 5, 0), 11.2588, 75.7804, true),    // Kozhikode, the 1962 eight-planet gathering
        (at(1966, 9, 21, 14, 30), 12.9716, 77.5946, false), // Bengaluru
        (at(1970, 3, 7, 17, 45), 23.0225, 72.5714, true),  // Ahmedabad, solar eclipse day
        (at(1973, 10, 28, 9, 10), 17.3850, 78.4867, false), // Hyderabad
        (at(1977, 6, 13, 20, 55), 25.5941, 85.1376, true), // Patna
        (at(1981, 1, 9, 4, 25), 26.9124, 75.7873, false),  // Jaipur
        (at(1984, 8, 30, 11, 0), 9.9312, 76.2673, true),   // Kochi
        (at(1988, 4, 16, 23, 35), 51.5074, -0.1278, false), // London
        (at(1990, 2, 20, 5, 4), 9.9312, 76.2673, true),    // Kochi (the owner's chart)
        (at(1993, 11, 11, 15, 15), 40.7128, -74.0060, false), // New York
        (at(1997, 9, 22, 13, 12), 9.9312, 76.2673, false), // Kochi
        (at(2001, 5, 5, 6, 50), 25.2048, 55.2708, true),   // Dubai
        (at(2005, 12, 25, 10, 30), 1.3521, 103.8198, false), // Singapore
        (at(2010, 7, 11, 19, 40), 10.5276, 76.2144, true), // Thrissur, solar eclipse day
    ]
}

fn dates() -> Vec<ExactInstant> {
    vec![at(2026, 10, 21, 5, 0), at(2027, 6, 2, 12, 30), at(2028, 4, 10, 2, 0), at(2030, 1, 15, 18, 0), at(2033, 8, 8, 9, 0)]
}

const LANGS: [&str; 3] = ["en", "ml", "hi"];
const HS: HouseSystem = HouseSystem::Placidus;
const AY: AyanamsaChoice = AyanamsaChoice::Lahiri;

fn full() -> bool {
    std::env::var("KSHANA_SAFETY_FULL").is_ok()
}

fn no_weights() -> PersonalWeightsInput {
    PersonalWeightsInput { answers: 0, techniques: vec![] }
}

/// The daily and long feeds, for charts `range`, at two dates each, in three languages.
fn feeds(range: std::ops::Range<usize>, name: &str) {
    let e = engine();
    let cs = charts();
    let ds = dates();
    let mut a = Audit::default();
    let relations = ["Spouse", "Father", "Mother", "Child", "Sibling", "husband", "wife", "son", "daughter"];
    for ci in range {
        let (b, la, lo, male) = cs[ci];
        let (ob, ola, olo, _) = cs[(ci + 7) % cs.len()];
        // Where the person is now: home for most, a big city for some.
        let (hla, hlo) = if ci % 3 == 0 { (28.6139, 77.2090) } else { (la, lo) };
        let off = if lo > 60.0 && lo < 100.0 { 330 } else { (lo / 15.0 * 60.0).round() as i32 };
        for (di, &t) in [ds[0], ds[1 + ci % 4]].iter().enumerate() {
            let tag = |s: &str| format!("c{ci}.{s}");
            for (li, lang) in LANGS.into_iter().enumerate() {
                let l = |s: &str| format!("c{ci}.{lang}.{s}");
                // The quick feeds in every language at both dates; the long reckonings
                // (months of windows) in English at both, Malayalam at the first and
                // Hindi at the second: the same templates, read across all charts.
                let heavy = li == 0 || li == di + 1 || full();
                a.run(&l("moment"), || e.forecast_moment(b, la, lo, hla, hlo, HS, AY, t, lang.into()));
                a.run(&l("briefing"), || e.briefing(b, la, lo, hla, hlo, HS, AY, life_names(), t, lang.into()));
                for s in ["day", "week", "month", "year"] {
                    a.run(&l(&format!("outlook_{s}")), || e.outlook(b, la, lo, hla, hlo, HS, AY, s.into(), t, lang.into(), vec![]));
                }
                a.run(&l("tree"), || e.remedy_tree(b, la, lo, hla, hlo, HS, AY, None, t, lang.into(), vec![]));
                for life in ["Health", "Love", "Family"] {
                    a.run(&l(&format!("tree_{life}")), || e.remedy_tree(b, la, lo, hla, hlo, HS, AY, Some(life.into()), t, lang.into(), vec![]));
                }
                a.run(&l("past"), || e.past_checks(b, la, lo, HS, AY, t, lang.into(), vec![]));
                a.run(&l("seal_pick"), || e.seal_pick(b, la, lo, HS, AY, t, lang.into()));
                if !heavy {
                    continue;
                }
                a.run(&l("past_reveal"), || e.past_reveal(b, la, lo, HS, AY, t, lang.into(), 30.0, vec![], vec![], 8, ci % 2 == 0));
                a.run(&l("future"),|| e.future(b, la, lo, hla, hlo, HS, AY, t, 24, lang.into(), vec![]));
                a.run(&l("future_personal"), || e.future_personal(b, la, lo, hla, hlo, HS, AY, t, 12, lang.into(), vec![], no_weights()));
                a.run(&l("firm"), || e.firm_predictions(b, la, lo, hla, hlo, HS, AY, t, lang.into(), None));
                let t0 = std::time::Instant::now();
                let sure = e.sure_windows(b, la, lo, HS, AY, hla, hlo, off, t, 18, vec![], 16, lang.into());
                *a.time.entry("sure".into()).or_default() += t0.elapsed().as_secs_f64();
                if let Ok(ws) = sure {
                    for w in ws.iter().filter(|w| w.is_hard).take(2) {
                        a.run(&l("sure_hours"), || e.sure_window_hours(b, la, lo, HS, AY, hla, hlo, off, w.area.clone(), w.is_hard, w.start, w.end, t, Some(w.peak), w.system_keys.clone(), lang.into()));
                    }
                    a.read(&l("sure"), format!("{ws:?}"));
                }
                a.run(&l("sure_next"), || e.sure_next(b, la, lo, HS, AY, hla, hlo, off, t, 12, lang.into()));
                a.run(&l("schedule"), || e.remedy_schedule(b, la, lo, hla, hlo, HS, AY, None, t, lang.into(), vec![], off));
                a.run(&l("facts"), || e.chart_facts(b, la, lo, HS, AY, t, lang.into(), 30.0, vec![]));
                let group = vec![PersonInput { birth: b, lat: la, lon: lo }, PersonInput { birth: ob, lat: ola, lon: olo }];
                a.run(&l("together"), || e.best_slots_together(group, hla, hlo, HS, AY, "family".into(), t, 7, 4, lang.into()));
            }
            a.run(&tag("letter"), || e.letter(b, la, lo, hla, hlo, HS, AY, t, vec![]));
            a.run(&tag("year"), || e.year_chapter(b, la, lo, hla, hlo, HS, AY, t));
            a.run(&tag("countdowns"), || e.countdowns(b, la, lo, hla, hlo, HS, AY, t));
            a.run(&tag("festivals"), || e.festivals(b, la, lo, HS, AY, t, 35));
            a.run(&tag("festivals_here"), || e.festivals_here(b, la, lo, HS, AY, hla, hlo, off, t, 35));
            a.run(&tag("slow"), || e.slow_horizon(b, la, lo, hla, hlo, HS, AY, t, 12));
            a.run(&tag("parihara"), || e.parihara(b, la, lo, AY, t, None));
            a.run(&tag("timeline"), || e.timeline(b, la, lo, HS, AY, t));
            a.run(&tag("plain_reading"), || e.plain_reading(b, la, lo, AY, t));
            a.run(&tag("daily"), || e.daily(b, la, lo, AY, t, hla, hlo));
            a.run(&tag("transits"), || e.transits(b, la, lo, AY, t, 60));
            a.run(&tag("chart_yogas"), || e.chart_yogas(b, la, lo, AY, t));
            a.run(&tag("couple"), || e.couple(b, la, lo, ob, ola, olo, hla, hlo, HS, AY, male, t));
            a.run(&tag("talk"), || e.talk_window(b, la, lo, ob, ola, olo, hla, hlo, HS, AY, t, 14));
            if di == 0 || full() {
                for life in life_names() {
                    a.run(&tag(&format!("explain_{life}")), || e.explain(b, la, lo, hla, hlo, HS, AY, life.clone(), t));
                }
            }
            // The family's shared windows (no language parameter: English).
            let fam: Vec<FamilyMemberInput> = (1..=4)
                .map(|k| {
                    let (rb, rla, rlo, _) = cs[(ci + k * 3) % cs.len()];
                    FamilyMemberInput { name: format!("R{k}"), relation: relations[(ci + k) % relations.len()].into(), birth: rb, lat: rla, lon: rlo }
                })
                .collect();
            let main_male = [Some(male), None, Some(!male)][ci % 3];
            a.run(&tag("family"), || e.family_proof(b, la, lo, main_male, fam, hla, hlo, HS, AY, t, 24));
        }
        // Once per chart: the natal pages and the match with another chart.
        let t = ds[0];
        let tag = |s: &str| format!("c{ci}.{s}");
        a.run(&tag("yogas"), || e.yogas(b, la, lo, AY));
        a.run(&tag("kundali"), || e.kundali(b, la, lo, ob, ola, olo, HS, AY, male));
        a.run(&tag("kundali_rev"), || e.kundali(ob, ola, olo, b, la, lo, HS, AY, !male));
        a.run(&tag("compat"), || e.compatibility(b, la, lo, ob, ola, olo, AY));
        a.run(&tag("life_map"), || e.life_map(b, la, lo, AY, 90));
        a.run(&tag("karakas"), || e.karakas(b, la, lo, AY));
        a.run(&tag("jaimini_full"), || e.jaimini_full(b, la, lo, AY, ci % 2 == 0, t));
        for life in std::iter::once(None).chain(life_names().into_iter().map(Some)) {
            a.run(&tag(&format!("parihara_{life:?}")), || e.parihara(b, la, lo, AY, t, life));
        }
        // The birth-time pages, in one language per chart (rotating).
        let lang = LANGS[ci % 3];
        let answers = vec![PastAnswerInput { from: at(2015, 1, 1, 0, 0), to: at(2015, 6, 1, 0, 0), life: "Love".into(), good: false, verdict: "yes".into() }];
        let ans = answers.clone();
        a.run(&format!("c{ci}.{lang}.learn"), || Ok::<_, ()>(e.learn_techniques(b, la, lo, HS, AY, ans, lang.into())));
        a.run(&format!("c{ci}.{lang}.birthnow"), || Ok::<_, ()>(e.birth_time_now(b, la, lo, HS, AY, t, 30.0, answers, vec![], vec![], 330, lang.into())));
    }
    a.finish(name, 2000);
}

#[test]
fn feeds_a() {
    feeds(0..5, "feeds_a");
}

#[test]
fn feeds_b() {
    feeds(5..10, "feeds_b");
}

#[test]
fn feeds_c() {
    feeds(10..15, "feeds_c");
}

#[test]
fn feeds_d() {
    feeds(15..20, "feeds_d");
}

/// Every question on every life and matter, every purpose, every couple question:
/// four charts by default, all twenty with `KSHANA_SAFETY_FULL`.
#[test]
fn every_question() {
    let e = engine();
    let cs = charts();
    let mut a = Audit::default();
    let which: Vec<usize> = if full() { (0..cs.len()).collect() } else { vec![1, 6, 14, 18] };
    let t = at(2026, 10, 21, 5, 0);
    let qs = questions();
    for (wi, &ci) in which.iter().enumerate() {
        let (b, la, lo, male) = cs[ci];
        let (ob, ola, olo, _) = cs[(ci + 5) % cs.len()];
        let tag = |s: &str| format!("c{ci}.{s}");
        // Every question on the first chart; on the others two questions per life and
        // matter, rotating, so that across the charts every pair is read at least once.
        let pick = |i: usize, qi: usize| wi == 0 || full() || qi == (i + wi) % qs.len() || qi == (i + wi + 3) % qs.len();
        for (i, life) in life_names().into_iter().enumerate() {
            for (qi, q) in qs.iter().enumerate() {
                if pick(i, qi) {
                    a.run(&tag(&format!("ask_{life}_{}", q.key)), || e.ask(b, la, lo, la, lo, HS, AY, life.clone(), q.key.clone(), t));
                }
            }
        }
        for (i, tp) in topics().into_iter().enumerate() {
            for (qi, q) in qs.iter().enumerate() {
                if pick(i, qi) {
                    a.run(&tag(&format!("topic_{}_{}", tp.key, q.key)), || e.ask_topic(b, la, lo, la, lo, HS, AY, tp.key.clone(), q.key.clone(), t));
                }
            }
        }
        for q in &qs {
            a.run(&tag(&format!("couple_ask_{}", q.key)), || e.couple_ask(b, la, lo, ob, ola, olo, la, lo, HS, AY, q.key.clone(), t));
        }
        for pu in purposes() {
            a.run(&tag(&format!("appointment_{}", pu.key)), || e.appointment(b, la, lo, la, lo, HS, AY, pu.key.clone(), t));
            a.run(&tag(&format!("slots_{}", pu.key)), || e.best_slots(b, la, lo, la, lo, HS, AY, pu.key.clone(), t, 7, 4));
        }
        for (li, lang) in LANGS.into_iter().enumerate() {
            for (i, life) in life_names().into_iter().enumerate() {
                a.run(&format!("c{ci}.{lang}.tree_{life}"), || e.remedy_tree(b, la, lo, la, lo, HS, AY, Some(life.clone()), t, lang.into(), vec![]));
                // The dated plan is slow: each life in one language, rotating.
                if (i + wi) % 3 == li || full() {
                    a.run(&format!("c{ci}.{lang}.schedule_{life}"), || e.remedy_schedule(b, la, lo, la, lo, HS, AY, Some(life.clone()), t, lang.into(), vec![], 330));
                }
            }
        }
        let _ = male;
    }
    a.finish("every_question", 5000);
}

/// The expert pages behind "Show the chart pages": the Kerala instruments, the year
/// chart, the dasha systems, the bird of the day, the KP page, the strengths.
#[test]
fn chart_pages() {
    let e = engine();
    let cs = charts();
    let mut a = Audit::default();
    let ds = dates();
    for (ci, &(b, la, lo, _)) in cs.iter().enumerate() {
        let t = ds[ci % ds.len()];
        let tag = |s: &str| format!("c{ci}.{s}");
        a.run(&tag("varsha"), || e.varshaphala(b, la, lo, AY, 2026 + (ci as i64 % 4), la, lo));
        a.run(&tag("jaimini_extras"), || e.jaimini_extras(b, la, lo, AY));
        a.run(&tag("sandhi"), || e.dasha_sandhi_windows(b, la, lo, AY, at(2026, 1, 1, 0, 0), at(2036, 1, 1, 0, 0), None));
        for lord in ["Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu"] {
            a.run(&tag(&format!("chidra_{lord}")), || e.dasha_chidra_lords(b, la, lo, AY, lord.into()));
        }
        a.run(&tag("vimshopaka"), || e.vimshopaka_bala(b, la, lo, AY));
        a.run(&tag("dashas"), || e.dasha_systems_at(b, la, lo, AY, t));
        a.run(&tag("dashas_more"), || e.dasha_systems_more_at(b, la, lo, AY, t));
        a.run(&tag("patyayini"), || e.patyayini(b, la, lo, AY, 2027, la, lo));
        a.run(&tag("pakshi"), || e.pancha_pakshi(b, AY, t, la, lo));
        a.run(&tag("day_clock"), || e.day_clock(t, la, lo, AY));
        a.run(&tag("kp_full"), || e.kp_full(b, la, lo, AY, t, la, lo));
        a.run(&tag("shadbala"), || e.shadbala(b, la, lo, AY));
        a.run(&tag("ashtakavarga"), || e.ashtakavarga(b, la, lo, AY));
        a.run(&tag("retrodict"), || e.retrodict(b, la, lo, AY, at(2010, 1, 1, 0, 0), at(2026, 1, 1, 0, 0)));
        a.run(&tag("transit_windows"), || e.transit_windows(b, la, lo, AY, t, at(2029, 1, 1, 0, 0)));
        a.run(&tag("areas"), || e.area_readings(b, la, lo, 0.0, HS, AY, t, 3));
        a.run(&tag("profections"), || e.profections(b, la, lo, AY, t));
        a.run(&tag("firdaria"), || e.firdaria(b, la, lo, AY));
        a.run(&tag("progressed_hits"), || e.progressed_hits(b, la, lo, AY, t, at(2031, 1, 1, 0, 0)));
        a.run(&tag("muhurta"), || e.muhurta(["Marriage", "Travel", "Business", "Housewarming", "Learning", "Medical", "General"][ci % 7].into(), t, 14, la, lo, AY, None, 5));
        a.run(&tag("panchanga"), || e.panchanga_detail(t, la, lo, AY));
        a.run(&tag("sphutas"), || e.prasna_marga_sphutas(t, la, lo, AY));
        a.run(&tag("arudha"), || e.prasna_arudha_sphutas(t, la, lo, AY, (ci % 12) as u8));
        a.run(&tag("calendar"), || e.calendar(t, 30, la, lo, AY));
        a.run(&tag("calendar_days"), || e.calendar_days(la, lo, 330, 2026, 1 + (ci as u32 % 12), 1, 31, AY));
        a.run(&tag("calendar_festivals"), || e.calendar_festivals(la, lo, 330, 2026, 1 + (ci as u32 % 12), 1, 31, AY));
        a.run(&tag("eclipses"), || e.eclipses(t, at(2036, 1, 1, 0, 0), la, lo));
        a.read(&tag("refine"), format!("{:?}", e.refine_birth_time(b, la, lo, HS, AY, 30.0, vec![])));
        // A question cast now: every area, and every shape on a quarter of the charts
        // (each cast costs seconds; the words depend on area and shape, not the chart).
        let shapes = [QuestionShape::WillItHappen, QuestionShape::When, QuestionShape::Choice, QuestionShape::WillItReturn, QuestionShape::WillTheyRecover];
        for (ai, area) in ["Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Family", "Foreign"].into_iter().enumerate() {
            for (si, &shape) in shapes.iter().enumerate() {
                if !(ci % 5 == 0 || si == (ci + ai) % shapes.len() || full()) {
                    continue;
                }
                let n = if ci % 2 == 0 { Some(((ci * 37 + 11) % 249 + 1) as u16) } else { None };
                a.run(&tag(&format!("prashna_{area}_{shape:?}")), || e.prashna(t, la, lo, n, area.into(), shape, AY, 15));
            }
        }
    }
    for n in [111, 123, 234, 345, 456, 567, 678, 781, 812, 888, 518, 647] {
        a.read(&format!("ashtamangala_{n}"), format!("{:?}", ashtamangala_number(n)));
    }
    a.finish("chart_pages", 3000);
}

/// The fixed lists, the dictionary, the free-text understanding (including questions
/// about death, illness and divorce), the "How this helps" text, the trust words.
#[test]
fn catalogues_and_questions() {
    let mut a = Audit::default();
    a.read("questions", format!("{:?}", questions()));
    a.read("purposes", format!("{:?}", purposes()));
    a.read("topics", format!("{:?}", topics()));
    a.read("hora_lore", format!("{:?}", hora_lore_all()));
    a.read("glossary", format!("{:?}", glossary()));
    a.read("yoga_catalogue", format!("{:?}", yoga_catalogue()));
    a.read("dasha_names", format!("{:?} {:?} {:?}", dasha_system_names(), more_dasha_system_names(), dasha_level_names()));
    for lang in ["en", "ml", "hi", "ta", "te", "kn"] {
        a.read(&format!("lexicon_{lang}"), format!("{:?}", lexicon(lang.into())));
    }
    for lang in LANGS {
        for c in [0.0, 20.0, 45.0, 60.0, 75.0, 90.0, 100.0] {
            a.read(&format!("sure_words_{lang}"), sure_words(lang.into(), c));
        }
        for area in life_names() {
            for good in [true, false] {
                a.read(&format!("claim_q_{lang}_{area}"), format!("{:?}", claim_question(lang.into(), area.clone(), good, at(2026, 11, 1, 0, 0), at(2026, 11, 20, 0, 0), 76.27)));
            }
        }
        let mut items: Vec<ScoreItemInput> = ["yes", "no", "close", "yes", "no"].iter().map(|o| ScoreItemInput { source: "sealed".into(), outcome: (*o).into(), chance: 0.5, used_for_fitting: false }).collect();
        items.extend(["yes", "no"].iter().map(|o| ScoreItemInput { source: "past".into(), outcome: (*o).into(), chance: 0.5, used_for_fitting: true }));
        a.read(&format!("scoreboard_{lang}"), format!("{:?}", scoreboard(items, lang.into())));
        a.read(&format!("tilt_{lang}"), format!("{:?}", forecast_tilt(no_weights(), lang.into())));
    }
    for planet in ["Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu"] {
        for source in ["dasha", "gochara", "natal"] {
            for life in std::iter::once(None).chain(life_names().into_iter().map(Some)) {
                a.read(&format!("how_{planet}_{source}"), remedy_how_it_helps(planet.into(), source.into(), life));
            }
        }
    }
    // What a frightened person types: each is answered with care or a refusal that
    // offers what can be answered, never with the banned words.
    let typed = [
        "when will i die", "how long will i live", "will my father die", "will my mother die this year", "when will my husband die", "ente maranam eppol",
        "meri maut kab hogi", "मेरी मृत्यु कब होगी", "എന്റെ മരണം എപ്പോൾ", "do i have cancer", "is it cancer", "will i get cancer", "will i have an accident",
        "will my divorce happen", "is divorce likely for me", "तलाक होगा क्या", "will we separate", "will my wife leave me", "we separated last year, will we reunite",
        "will my husband die before me", "will i become a widow", "I want to die", "thinking of suicide", "no point living", "will my child be ok in surgery",
        "is my marriage going to break", "constant quarrels with my husband", "will my health recover", "my mother is very ill", "how many years will my mother live",
        "will i live till 80", "when will my grandfather pass away", "kalyanam eppo", "shaadi kab hogi", "will i get a job", "is there black magic on me",
        "what is my longevity", "my ayurdaya", "maraka dasha when", "is this a bad time for travel", "accident yoga in my chart", "widow yoga",
    ];
    for s in typed {
        a.read(&format!("understand[{s}]"), format!("{:?}", understand(s.into())));
    }
    a.finish("catalogues_and_questions", 1500);
}

/// The matcher itself: hits what it should, passes what it should.
#[test]
fn the_matcher_reads_words_not_letters() {
    for bad in [
        "a death in the family", "He may die.", "an Accident on the road", "divorce is likely", "a separation", "longevity is short", "loss of spouse",
        "मृत्यु का योग", "तलाक", "दुर्घटना से बचें", "मरणासन्न", "മരണം", "മരിക്കും", "അപകടം", "ആയുസ്സ് കുറവ്", "cancer of the blood", "छोटी चोट",
    ] {
        assert!(banned_in(bad).is_some(), "should hit: {bad}");
    }
    for ok in [
        "studied hard", "a deadline", "Moon in Cancer", "Diet and rest", "dietary care", "the Mahamrityunjaya mantra", "महामृत्युंजय मंत्र", "स्मरण करें",
        "സ്മരണ", "आपकी आयु 36 वर्ष", "पहाड़ की चोटी", "a time to take extra care of your health", "skill", "audience", "indie",
    ] {
        assert!(banned_in(ok).is_none(), "should pass: {ok} ({:?})", banned_in(ok));
    }
}

/// Every string literal in the app's Kotlin and `strings.xml`, and the longevity rule:
/// no screen calls `ayurdayaClassical`.
#[test]
fn kotlin_never_shows_longevity_or_the_banned_words() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../android/app/src/main");
    if !root.exists() {
        eprintln!("no android sources here; skipped");
        return;
    }
    let mut files = Vec::new();
    let mut stack = vec![root.join("java"), root.join("res")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                // The generated bindings carry the engine's own doc comments.
                if p.file_name().is_some_and(|n| n == "uniffi") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "kt") || p.file_name().is_some_and(|n| n == "strings.xml") {
                files.push(p);
            }
        }
    }
    assert!(files.len() > 20, "the Kotlin sources were not found");
    let mut hits = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let name = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        for (n, line) in text.lines().enumerate() {
            let tl = line.trim_start();
            if tl.starts_with("//") || tl.starts_with('*') || tl.starts_with("/*") || tl.starts_with("import ") {
                continue;
            }
            if line.contains("ayurdayaClassical(") {
                hits.push(format!("{name}:{} calls the longevity reckoning: {}", n + 1, tl));
            }
            // String literals on the line (and the text of an XML string).
            let parts: Vec<&str> = if name.ends_with(".xml") { vec![line] } else { line.split('"').skip(1).step_by(2).collect() };
            for p in parts {
                // A bare lower-case identifier ("separation") is a stable key passed to
                // the engine, never shown.
                if !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    continue;
                }
                if let Some((w, gloss)) = banned_in(p) {
                    hits.push(format!("{name}:{} [{w}: {gloss}]: {}", n + 1, p));
                }
            }
        }
    }
    if !hits.is_empty() {
        panic!("{} Kotlin strings say what the app never says:\n{}", hits.len(), hits.join("\n"));
    }
}
