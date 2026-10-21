//! The plain scoreboard: one honest number for one person, "right 38 times out of 45",
//! with what guessing at random would have scored beside it.
//!
//! Every answered item counts once: a sealed prediction about the future once its
//! window has closed, a reading of the past the person marked, a quiz question. Yes is
//! right (1), Close is half right (½), No is wrong (0); "not sure" is left out. Each
//! item carries the chance a random guess would have been right: ½ for a yes/no
//! stretch (a coin saying "it will" or "it won't"), one over the number of choices for
//! a quiz question. The chance line is the Poisson-binomial distribution of those
//! chances ([`crate::expected_hits`]): its mean is what guessing scores on average, and
//! `P[K ≥ score]` is how often guessing alone would do as well. "Right" only means
//! something where it beats that line, and the words say so either way.

use crate::coverage::expected_hits;
use kshana_rules::Lang;

/// Where an answered item came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScoreSource {
    /// A sealed prediction about the future, answered after its window closed.
    Sealed,
    /// A reading of the person's past, marked Yes, Close or No.
    Past,
    /// A quiz question: which of several windows was the real one.
    Quiz,
}

impl ScoreSource {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            ScoreSource::Sealed => "sealed",
            ScoreSource::Past => "past",
            ScoreSource::Quiz => "quiz",
        }
    }

    /// From a key.
    pub fn parse(s: &str) -> Option<ScoreSource> {
        match s.trim().to_ascii_lowercase().as_str() {
            "sealed" | "future" => Some(ScoreSource::Sealed),
            "past" => Some(ScoreSource::Past),
            "quiz" => Some(ScoreSource::Quiz),
            _ => None,
        }
    }
}

/// One answered item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreItem {
    /// Where it came from.
    pub source: ScoreSource,
    /// 1 right, ½ close, 0 wrong.
    pub points: f64,
    /// The chance a random guess would have been right.
    pub chance: f64,
}

/// One source's own line.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreLine {
    /// Which source.
    pub source: ScoreSource,
    /// Points scored.
    pub right: f64,
    /// Items answered.
    pub total: usize,
    /// What guessing scores on average.
    pub chance: f64,
    /// The line in words.
    pub words: String,
}

/// Fewest answered items before the scoreboard says whether it beats guessing.
pub const MIN_TO_JUDGE: usize = 5;
/// `P[guessing does as well]` below which the score is said to beat guessing.
pub const BEATS_CHANCE_P: f64 = 0.05;

/// The scoreboard.
#[derive(Clone, Debug, PartialEq)]
pub struct Scoreboard {
    /// Points over every item.
    pub right: f64,
    /// Items answered.
    pub total: usize,
    /// What guessing scores on average over the same items.
    pub chance_expected: f64,
    /// Its standard deviation.
    pub chance_sd: f64,
    /// How often guessing alone scores at least as well, `P[K ≥ ⌊right⌋]`; 1 with
    /// nothing answered.
    pub p_luck: f64,
    /// Enough answers, and guessing would rarely do as well.
    pub beats_chance: bool,
    /// One line per source that has answers, sealed first.
    pub lines: Vec<ScoreLine>,
    /// "For you, Kshana has been right 38 times out of 45."
    pub headline: String,
    /// What guessing would have scored, and what that means.
    pub chance_line: String,
    /// A kind word on where the score stands.
    pub note: String,
    /// The whole thing as a short text to share.
    pub share_text: String,
}

/// "38", "37½" (or "37.5" style in the other scripts, which read ½ the same).
pub fn points_text(x: f64) -> String {
    let whole = x.floor();
    let half = x - whole >= 0.25;
    if half {
        format!("{}½", whole as i64)
    } else {
        format!("{}", x.round() as i64)
    }
}

fn times(lang: Lang, x: f64, n: usize) -> String {
    let r = points_text(x);
    match lang {
        Lang::En => format!("{r} {} out of {n}", if r == "1" { "time" } else { "times" }),
        Lang::Ml => format!("{n}-ൽ {r} തവണ"),
        Lang::Hi => format!("{n} में से {r} बार"),
    }
}

fn source_words(lang: Lang, s: ScoreSource) -> &'static str {
    lang.pick(match s {
        ScoreSource::Sealed => ["Sealed predictions about your future", "നിങ്ങളുടെ ഭാവിയെക്കുറിച്ച് മുദ്രവെച്ച പ്രവചനങ്ങൾ", "आपके भविष्य के बारे में सील की गई भविष्यवाणियाँ"],
        ScoreSource::Past => ["Readings of your past", "നിങ്ങളുടെ ഭൂതകാലത്തിന്റെ വായനകൾ", "आपके अतीत की पढ़त"],
        ScoreSource::Quiz => ["The which-window quiz", "ഏത് കാലം എന്ന ക്വിസ്", "कौन सा समय वाली प्रश्नोत्तरी"],
    })
}

fn line_words(lang: Lang, s: ScoreSource, right: f64, n: usize, chance: f64) -> String {
    let what = source_words(lang, s);
    let r = points_text(right);
    let c = points_text((chance * 2.0).round() / 2.0);
    match lang {
        Lang::En => format!("{what}: right {r} of {n}; guessing would get about {c}."),
        Lang::Ml => format!("{what}: {n}-ൽ {r} ശരി; ഊഹിച്ചാൽ ഏകദേശം {c}."),
        Lang::Hi => format!("{what}: {n} में से {r} सही; अंदाज़े से लगभग {c}।"),
    }
}

/// "about 1 time in 40"; "less than 1 time in 1,000" when smaller.
fn luck_words(lang: Lang, p: f64) -> String {
    if p < 0.001 {
        return lang.pick(["less than 1 time in 1,000", "1,000-ൽ ഒന്നിൽ താഴെ", "1,000 में 1 बार से भी कम"]).to_string();
    }
    let n = (1.0 / p.max(1e-9)).round() as i64;
    match lang {
        Lang::En => format!("about 1 time in {n}"),
        Lang::Ml => format!("ഏകദേശം {n}-ൽ ഒരിക്കൽ"),
        Lang::Hi => format!("लगभग {n} में 1 बार"),
    }
}

/// Score the answered items.
pub fn scoreboard(items: &[ScoreItem], lang: Lang) -> Scoreboard {
    let items: Vec<ScoreItem> = items
        .iter()
        .copied()
        .filter(|i| i.points.is_finite() && i.chance.is_finite())
        .map(|i| ScoreItem { points: i.points.clamp(0.0, 1.0), chance: i.chance.clamp(0.0, 1.0), ..i })
        .collect();
    let total = items.len();
    let right: f64 = items.iter().map(|i| i.points).sum();
    let pb = expected_hits(&items.iter().map(|i| i.chance).collect::<Vec<_>>());
    let p_luck = if total == 0 { 1.0 } else { pb.p_at_least(right.floor() as usize) };
    let beats_chance = total >= MIN_TO_JUDGE && p_luck < BEATS_CHANCE_P && right > pb.mean;
    let lines: Vec<ScoreLine> = [ScoreSource::Sealed, ScoreSource::Past, ScoreSource::Quiz]
        .iter()
        .filter_map(|&s| {
            let mine: Vec<&ScoreItem> = items.iter().filter(|i| i.source == s).collect();
            if mine.is_empty() {
                return None;
            }
            let r: f64 = mine.iter().map(|i| i.points).sum();
            let c: f64 = mine.iter().map(|i| i.chance).sum();
            Some(ScoreLine { source: s, right: r, total: mine.len(), chance: c, words: line_words(lang, s, r, mine.len(), c) })
        })
        .collect();
    let headline = if total == 0 {
        lang.pick([
            "Nothing to score yet. Answer a question about your past, or wait for a sealed prediction to open: every answer counts here, right or wrong.",
            "ഇതുവരെ എണ്ണാൻ ഒന്നുമില്ല. നിങ്ങളുടെ ഭൂതകാലത്തെക്കുറിച്ചുള്ള ഒരു ചോദ്യത്തിന് ഉത്തരം നൽകൂ, അല്ലെങ്കിൽ മുദ്രവെച്ച ഒരു പ്രവചനം തുറക്കുന്നതുവരെ കാത്തിരിക്കൂ: ശരിയായാലും തെറ്റായാലും എല്ലാ ഉത്തരവും ഇവിടെ എണ്ണും.",
            "अभी गिनने को कुछ नहीं। अपने अतीत के किसी सवाल का जवाब दीजिए, या सील की गई भविष्यवाणी के खुलने का इंतज़ार कीजिए: सही हो या गलत, हर जवाब यहाँ गिना जाता है।",
        ])
        .to_string()
    } else {
        match lang {
            Lang::En => format!("For you, Kshana has been right {}.", times(lang, right, total)),
            Lang::Ml => format!("നിങ്ങളുടെ കാര്യത്തിൽ ക്ഷണ {} ശരിയായി.", times(lang, right, total)),
            Lang::Hi => format!("आपके लिए क्षण {} सही रहा।", times(lang, right, total)),
        }
    };
    let chance_line = if total == 0 {
        String::new()
    } else {
        let c = times(lang, (pb.mean * 2.0).round() / 2.0, total);
        let luck = luck_words(lang, p_luck);
        let second = if right <= pb.mean {
            lang.pick(["So far that is no better than guessing.", "ഇതുവരെ അത് ഊഹത്തേക്കാൾ മെച്ചമല്ല.", "अब तक यह अंदाज़े से बेहतर नहीं है।"]).to_string()
        } else if total < MIN_TO_JUDGE {
            lang.pick(["Too few answers yet to tell skill from luck.", "കഴിവും ഭാഗ്യവും തിരിച്ചറിയാൻ ഇനിയും ഉത്തരങ്ങൾ കുറവാണ്.", "हुनर और किस्मत में फ़र्क करने के लिए अभी जवाब कम हैं।"]).to_string()
        } else if beats_chance {
            match lang {
                Lang::En => format!("Guessing alone would do this well {luck}."),
                Lang::Ml => format!("ഊഹം മാത്രം കൊണ്ട് ഇത്ര നന്നാകുന്നത് {luck} മാത്രം."),
                Lang::Hi => format!("सिर्फ़ अंदाज़े से इतना अच्छा {luck} ही होता।"),
            }
        } else {
            match lang {
                Lang::En => format!("Better than guessing, but guessing would do this well {luck}, so it is not yet clear."),
                Lang::Ml => format!("ഊഹത്തേക്കാൾ മെച്ചം, പക്ഷേ ഊഹം കൊണ്ടും {luck} ഇത്ര നന്നാകാം; അതിനാൽ ഇനിയും വ്യക്തമല്ല."),
                Lang::Hi => format!("अंदाज़े से बेहतर, पर अंदाज़ा भी {luck} इतना अच्छा कर लेता, इसलिए अभी साफ़ नहीं।"),
            }
        };
        match lang {
            Lang::En => format!("Guessing at random would be right about {c}. {second}"),
            Lang::Ml => format!("വെറുതെ ഊഹിച്ചാൽ ഏകദേശം {c} ശരിയാകും. {second}"),
            Lang::Hi => format!("यूँ ही अंदाज़ा लगाने से लगभग {c} सही होता। {second}"),
        }
    };
    let note = if total == 0 {
        String::new()
    } else if beats_chance {
        lang.pick([
            "Misses count here as plainly as hits. Keep answering: each answer keeps the score honest.",
            "ശരികൾ പോലെ തന്നെ തെറ്റുകളും ഇവിടെ എണ്ണുന്നു. ഉത്തരം നൽകിക്കൊണ്ടിരിക്കൂ: ഓരോ ഉത്തരവും സ്കോർ സത്യസന്ധമാക്കുന്നു.",
            "यहाँ चूकें भी उतनी ही साफ़ गिनी जाती हैं जितनी सही बातें। जवाब देते रहिए: हर जवाब स्कोर को ईमानदार रखता है।",
        ])
        .to_string()
    } else {
        lang.pick([
            "Still learning your chart: each answer sharpens it, a No as much as a Yes.",
            "ഇപ്പോഴും നിങ്ങളെ പഠിച്ചുകൊണ്ടിരിക്കുകയാണ്: ഓരോ ഉത്തരവും വായനയെ മൂർച്ചയാക്കുന്നു, ഒരു അതെ പോലെ തന്നെ ഒരു ഇല്ലയും.",
            "अभी आपको समझ रहे हैं: हर जवाब पढ़त को और पैना करता है, एक हाँ जितना ही एक ना भी।",
        ])
        .to_string()
    };
    let share_text = if total == 0 {
        String::new()
    } else {
        let mut s = format!("{headline} {chance_line}");
        for l in &lines {
            s.push(' ');
            s.push_str(&l.words);
        }
        s
    };
    Scoreboard { right, total, chance_expected: pb.mean, chance_sd: pb.sd(), p_luck, beats_chance, lines, headline, chance_line, note, share_text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_rules::plain::jargon_in;

    fn item(s: ScoreSource, p: f64, c: f64) -> ScoreItem {
        ScoreItem { source: s, points: p, chance: c }
    }

    #[test]
    fn the_score_counts_halves_and_keeps_sources_apart() {
        let mut v = Vec::new();
        for _ in 0..30 {
            v.push(item(ScoreSource::Past, 1.0, 0.5));
        }
        for _ in 0..4 {
            v.push(item(ScoreSource::Past, 0.0, 0.5));
        }
        v.push(item(ScoreSource::Sealed, 0.5, 0.5));
        for _ in 0..5 {
            v.push(item(ScoreSource::Sealed, 1.0, 0.5));
        }
        for _ in 0..2 {
            v.push(item(ScoreSource::Sealed, 0.0, 0.5));
        }
        for _ in 0..3 {
            v.push(item(ScoreSource::Quiz, 1.0, 0.25));
        }
        let b = scoreboard(&v, Lang::En);
        assert_eq!(b.total, 45);
        assert!((b.right - 38.5).abs() < 1e-12);
        assert_eq!(b.headline, "For you, Kshana has been right 38½ times out of 45.");
        assert!((b.chance_expected - (42.0 * 0.5 + 0.75)).abs() < 1e-12);
        assert!(b.beats_chance && b.p_luck < 1e-4, "{b:?}");
        assert_eq!(b.lines.len(), 3);
        assert_eq!(b.lines[0].source, ScoreSource::Sealed);
        assert_eq!(b.lines[0].words, "Sealed predictions about your future: right 5½ of 8; guessing would get about 4.");
        assert_eq!(b.lines[1].total, 34);
        assert!(b.share_text.starts_with(&b.headline));
    }

    #[test]
    fn a_score_no_better_than_guessing_is_said_so_kindly() {
        let v: Vec<ScoreItem> = (0..10).map(|i| item(ScoreSource::Past, if i % 3 == 0 { 1.0 } else { 0.0 }, 0.5)).collect();
        let b = scoreboard(&v, Lang::En);
        assert!(!b.beats_chance);
        assert!(b.chance_line.contains("no better than guessing"), "{}", b.chance_line);
        assert!(b.note.starts_with("Still learning your chart"));
        // Nothing answered.
        let e = scoreboard(&[], Lang::En);
        assert_eq!((e.total, e.p_luck), (0, 1.0));
        assert!(e.lines.is_empty() && e.share_text.is_empty());
    }

    #[test]
    fn the_scoreboard_names_no_astrology_and_promises_nothing() {
        let sets: Vec<Vec<ScoreItem>> = vec![
            vec![],
            vec![item(ScoreSource::Past, 1.0, 0.5)],
            (0..20).map(|_| item(ScoreSource::Sealed, 1.0, 0.5)).collect(),
            (0..20).map(|i| item(ScoreSource::Quiz, (i % 2) as f64, 0.2)).collect(),
            (0..8).map(|_| item(ScoreSource::Past, 0.0, 0.5)).collect(),
        ];
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for s in &sets {
                let b = scoreboard(s, lang);
                for t in [&b.headline, &b.chance_line, &b.note, &b.share_text].into_iter().chain(b.lines.iter().map(|l| &l.words)) {
                    assert_eq!(jargon_in(lang, t), None, "{t}");
                    let low = t.to_lowercase();
                    for banned in ["guarantee", "always right", "fate", "destiny"] {
                        assert!(!low.contains(banned), "{t}");
                    }
                }
            }
        }
    }
}
