//! The plain scoreboard: one honest number for one person, "right 38 times out of 45",
//! with what guessing at random would have scored beside it.
//!
//! **The hold-out rule.** The headline number counts *only* predictions sealed before
//! their dates began and answered after their window closed ([`ScoreSource::Sealed`]
//! with `used_for_fitting == false`, see [`ScoreItem::counts`]). Anything that helped
//! fit the birth time or the person's own weights before it was scored (the readings of
//! the past the person marked, the facts and logged events behind the birth-time
//! narrowing) can never enter it: a reading tuned to an answer would otherwise be graded
//! on that very answer. Those items get their own separate line, "how well the reading
//! fits your past", said plainly not to count. The which-window quiz is about the past
//! too, so it sits on that separate line as well.
//!
//! A sealed prediction is scored the moment it is answered, and that score is frozen in
//! its stored line. Only *then* may its answer join the birth-time narrowing (it was
//! scored blind, before any fitting saw it), and the app records that use; the headline
//! keeps counting it exactly once, from the frozen answer, and never re-counts it.
//!
//! Yes is right (1), Close is half right (½), No is wrong (0); "not sure" is left out.
//! Each item carries the chance a random guess would have been right: ½ for a yes/no
//! stretch (a coin saying "it will" or "it won't"), one over the number of choices for a
//! quiz question. The chance line is the Poisson-binomial distribution of those chances
//! ([`crate::expected_hits`]): its mean is what guessing scores on average, and
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
    /// True if this answer helped fit the birth time or the person's weights *before*
    /// it was scored. Such an item never counts toward the headline. A sealed prediction
    /// answered first and only afterwards used for fitting is `false`: it was scored
    /// blind, and its score is frozen.
    pub used_for_fitting: bool,
}

impl ScoreItem {
    /// Whether the item may count toward the headline: a sealed forward prediction that
    /// nothing was fitted to before it was scored.
    pub fn counts(&self) -> bool {
        self.source == ScoreSource::Sealed && !self.used_for_fitting
    }
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
    /// Points over the sealed forward predictions only (the hold-out rule).
    pub right: f64,
    /// Sealed forward predictions answered.
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
    /// The headline's own line (sealed predictions), when any are answered.
    pub lines: Vec<ScoreLine>,
    /// Points over the items kept out of the headline (the past, the quiz, anything
    /// used for fitting before it was scored).
    pub fit_right: f64,
    /// How many such items.
    pub fit_total: usize,
    /// What guessing scores on them on average.
    pub fit_chance: f64,
    /// "How well the reading fits your past (used to tune your birth time, so not
    /// counted in the score): right 30 of 34; guessing would get about 17." Empty when
    /// there are none.
    pub fit_line: String,
    /// One line per kept-out source.
    pub fit_lines: Vec<ScoreLine>,
    /// "For you, Kshana has been right 38 times out of 45."
    pub headline: String,
    /// What guessing would have scored, and what that means.
    pub chance_line: String,
    /// A kind word on where the score stands.
    pub note: String,
    /// The headline as a short text to share (never the kept-out lines).
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

fn fit_source_words(lang: Lang, s: ScoreSource) -> &'static str {
    lang.pick(match s {
        ScoreSource::Sealed => [
            "Sealed predictions used to tune your birth time before they were answered (not counted)",
            "ഉത്തരം കിട്ടുംമുമ്പ് ജനനസമയം ശരിയാക്കാൻ ഉപയോഗിച്ച മുദ്രവെച്ച പ്രവചനങ്ങൾ (എണ്ണുന്നില്ല)",
            "जवाब से पहले जन्म-समय सुधारने में इस्तेमाल हुई सील की गई भविष्यवाणियाँ (गिनी नहीं जातीं)",
        ],
        ScoreSource::Past => [
            "Readings of your past (used to tune your birth time, so not counted)",
            "നിങ്ങളുടെ ഭൂതകാലത്തിന്റെ വായനകൾ (ജനനസമയം ശരിയാക്കാൻ ഉപയോഗിച്ചു, അതിനാൽ എണ്ണുന്നില്ല)",
            "आपके अतीत की पढ़त (जन्म-समय सुधारने में इस्तेमाल हुई, इसलिए गिनी नहीं जाती)",
        ],
        ScoreSource::Quiz => [
            "The which-window quiz about your past (not counted: only predictions sealed ahead count)",
            "ഭൂതകാലത്തെക്കുറിച്ചുള്ള ഏത് കാലം എന്ന ക്വിസ് (എണ്ണുന്നില്ല: മുൻകൂട്ടി മുദ്രവെച്ചവ മാത്രം എണ്ണും)",
            "अतीत के बारे में कौन सा समय वाली प्रश्नोत्तरी (गिनी नहीं जाती: सिर्फ़ पहले से सील की गई बातें गिनी जाती हैं)",
        ],
    })
}

/// How the score is kept, in plain words: what counts, what does not, and why.
pub fn how_it_works(lang: Lang) -> &'static str {
    lang.pick([
        "Only predictions written down and sealed before their dates began count in the score, and only once their dates have passed. Your answers about your past help tune your birth time, so they are kept on a separate line and never counted: a reading tuned to an answer cannot be graded on that same answer. Once a sealed prediction is answered, its score is fixed; only then may it also help tune your birth time, and it is still counted just once. Yes counts as right, Close as half, No as wrong; Not sure is left out. Guessing means a coin for each yes-or-no stretch.",
        "തീയതികൾ തുടങ്ങുംമുമ്പ് എഴുതി മുദ്രവെച്ച പ്രവചനങ്ങൾ മാത്രമേ സ്കോറിൽ എണ്ണൂ, അതും അവയുടെ തീയതികൾ കഴിഞ്ഞശേഷം മാത്രം. നിങ്ങളുടെ ഭൂതകാലത്തെക്കുറിച്ചുള്ള ഉത്തരങ്ങൾ ജനനസമയം ശരിയാക്കാൻ സഹായിക്കുന്നു, അതിനാൽ അവ വേറെ വരിയിൽ സൂക്ഷിക്കുന്നു, ഒരിക്കലും എണ്ണുന്നില്ല: ഒരു ഉത്തരത്തിനനുസരിച്ച് ശരിയാക്കിയ വായനയെ അതേ ഉത്തരം കൊണ്ട് വിലയിരുത്താനാവില്ല. മുദ്രവെച്ച ഒരു പ്രവചനത്തിന് ഉത്തരം കിട്ടിയാൽ അതിന്റെ സ്കോർ ഉറച്ചു; അതിനുശേഷം മാത്രം അത് ജനനസമയം ശരിയാക്കാനും സഹായിച്ചേക്കാം, എന്നിട്ടും ഒരിക്കൽ മാത്രം എണ്ണും. അതെ ശരി, ഏകദേശം പകുതി, ഇല്ല തെറ്റ്; ഉറപ്പില്ല ഒഴിവാക്കുന്നു. ഊഹം എന്നാൽ ഓരോ അതെ-ഇല്ല കാലത്തിനും ഒരു നാണയം.",
        "सिर्फ़ वही भविष्यवाणियाँ स्कोर में गिनी जाती हैं जो अपनी तारीखें शुरू होने से पहले लिखकर सील की गईं, और वह भी तारीखें बीतने के बाद। आपके अतीत के जवाब जन्म-समय सुधारने में मदद करते हैं, इसलिए वे अलग पंक्ति में रखे जाते हैं और कभी गिने नहीं जाते: किसी जवाब से सुधारी गई पढ़त को उसी जवाब से नहीं परखा जा सकता। सील की गई भविष्यवाणी का जवाब मिलते ही उसका स्कोर तय हो जाता है; उसके बाद ही वह जन्म-समय सुधारने में भी मदद कर सकती है, और फिर भी एक ही बार गिनी जाती है। हाँ सही, करीब आधा, ना गलत; पक्का नहीं छोड़ दिया जाता है। अंदाज़ा यानी हर हाँ-या-ना वाले समय के लिए एक सिक्का।",
    ])
}

fn right_of(lang: Lang, what: &str, right: f64, n: usize, chance: f64) -> String {
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

/// Score the answered items. Only sealed forward predictions not used for fitting before
/// they were scored count toward the headline (the module's hold-out rule); everything
/// else is scored on its own separate line.
pub fn scoreboard(items: &[ScoreItem], lang: Lang) -> Scoreboard {
    let clean: Vec<ScoreItem> = items
        .iter()
        .copied()
        .filter(|i| i.points.is_finite() && i.chance.is_finite())
        .map(|i| ScoreItem { points: i.points.clamp(0.0, 1.0), chance: i.chance.clamp(0.0, 1.0), ..i })
        .collect();
    let (items, kept_out): (Vec<ScoreItem>, Vec<ScoreItem>) = clean.into_iter().partition(|i| i.counts());
    let total = items.len();
    let right: f64 = items.iter().map(|i| i.points).sum();
    let pb = expected_hits(&items.iter().map(|i| i.chance).collect::<Vec<_>>());
    let p_luck = if total == 0 { 1.0 } else { pb.p_at_least(right.floor() as usize) };
    let beats_chance = total >= MIN_TO_JUDGE && p_luck < BEATS_CHANCE_P && right > pb.mean;
    let lines: Vec<ScoreLine> = if total == 0 {
        Vec::new()
    } else {
        let c: f64 = items.iter().map(|i| i.chance).sum();
        vec![ScoreLine { source: ScoreSource::Sealed, right, total, chance: c, words: right_of(lang, source_words(lang, ScoreSource::Sealed), right, total, c) }]
    };
    let fit_right: f64 = kept_out.iter().map(|i| i.points).sum();
    let fit_total = kept_out.len();
    let fit_chance: f64 = kept_out.iter().map(|i| i.chance).sum();
    let fit_lines: Vec<ScoreLine> = [ScoreSource::Past, ScoreSource::Quiz, ScoreSource::Sealed]
        .iter()
        .filter_map(|&s| {
            let mine: Vec<&ScoreItem> = kept_out.iter().filter(|i| i.source == s).collect();
            if mine.is_empty() {
                return None;
            }
            let r: f64 = mine.iter().map(|i| i.points).sum();
            let c: f64 = mine.iter().map(|i| i.chance).sum();
            Some(ScoreLine { source: s, right: r, total: mine.len(), chance: c, words: right_of(lang, fit_source_words(lang, s), r, mine.len(), c) })
        })
        .collect();
    let fit_line = if fit_total == 0 {
        String::new()
    } else {
        let what = lang.pick([
            "How well the reading fits your past (used to tune your birth time, so not counted in the score)",
            "വായന നിങ്ങളുടെ ഭൂതകാലവുമായി എത്ര ചേരുന്നു (ജനനസമയം ശരിയാക്കാൻ ഉപയോഗിച്ചു, അതിനാൽ സ്കോറിൽ എണ്ണുന്നില്ല)",
            "पढ़त आपके अतीत से कितनी मेल खाती है (जन्म-समय सुधारने में इस्तेमाल हुई, इसलिए स्कोर में नहीं गिनी जाती)",
        ]);
        right_of(lang, what, fit_right, fit_total, fit_chance)
    };
    let headline = if total == 0 {
        lang.pick([
            "Nothing to score yet. Only predictions sealed before their dates count here: once a sealed prediction's dates have passed, you will be asked how it went, and every answer counts, right or wrong.",
            "ഇതുവരെ എണ്ണാൻ ഒന്നുമില്ല. തീയതികൾക്ക് മുമ്പ് മുദ്രവെച്ച പ്രവചനങ്ങൾ മാത്രമേ ഇവിടെ എണ്ണൂ: മുദ്രവെച്ച ഒരു പ്രവചനത്തിന്റെ തീയതികൾ കഴിഞ്ഞാൽ, അത് എങ്ങനെയായി എന്ന് ചോദിക്കും; ശരിയായാലും തെറ്റായാലും എല്ലാ ഉത്തരവും എണ്ണും.",
            "अभी गिनने को कुछ नहीं। यहाँ सिर्फ़ तारीखों से पहले सील की गई भविष्यवाणियाँ गिनी जाती हैं: किसी सील की गई भविष्यवाणी की तारीखें बीतने पर आपसे पूछा जाएगा कि कैसा रहा, और सही हो या गलत, हर जवाब गिना जाता है।",
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
    Scoreboard {
        right,
        total,
        chance_expected: pb.mean,
        chance_sd: pb.sd(),
        p_luck,
        beats_chance,
        lines,
        fit_right,
        fit_total,
        fit_chance,
        fit_line,
        fit_lines,
        headline,
        chance_line,
        note,
        share_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_rules::plain::jargon_in;

    fn item(s: ScoreSource, p: f64, c: f64) -> ScoreItem {
        ScoreItem { source: s, points: p, chance: c, used_for_fitting: s == ScoreSource::Past }
    }

    #[test]
    fn only_sealed_forward_predictions_reach_the_headline() {
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
        assert_eq!(b.total, 8);
        assert!((b.right - 5.5).abs() < 1e-12);
        assert_eq!(b.headline, "For you, Kshana has been right 5½ times out of 8.");
        assert!((b.chance_expected - 4.0).abs() < 1e-12);
        assert_eq!(b.lines.len(), 1);
        assert_eq!(b.lines[0].source, ScoreSource::Sealed);
        assert_eq!(b.lines[0].words, "Sealed predictions about your future: right 5½ of 8; guessing would get about 4.");
        // The past and the quiz sit on their own line, said not to count.
        assert_eq!(b.fit_total, 37);
        assert!((b.fit_right - 33.0).abs() < 1e-12);
        assert!(
            b.fit_line.starts_with("How well the reading fits your past (used to tune your birth time, so not counted in the score): right 33 of 37"),
            "{}",
            b.fit_line
        );
        assert_eq!(b.fit_lines.len(), 2);
        assert!(b.share_text.starts_with(&b.headline));
        assert!(!b.share_text.contains("past"), "the share text carries the headline only: {}", b.share_text);
    }

    #[test]
    fn a_fitted_item_can_never_enter_the_headline() {
        // Whatever its source or answer, an item used for fitting before it was scored
        // leaves the headline untouched.
        let base: Vec<ScoreItem> = (0..6).map(|i| item(ScoreSource::Sealed, (i % 2) as f64, 0.5)).collect();
        let b0 = scoreboard(&base, Lang::En);
        for src in [ScoreSource::Sealed, ScoreSource::Past, ScoreSource::Quiz] {
            for pts in [0.0, 0.5, 1.0] {
                let mut v = base.clone();
                for _ in 0..50 {
                    v.push(ScoreItem { source: src, points: pts, chance: 0.5, used_for_fitting: true });
                }
                let b = scoreboard(&v, Lang::En);
                assert_eq!((b.total, b.right, b.p_luck, b.beats_chance), (b0.total, b0.right, b0.p_luck, b0.beats_chance));
                assert_eq!(b.headline, b0.headline);
                assert_eq!(b.fit_total, 50);
            }
        }
        // Past and quiz never count, even unflagged: they are not forward predictions.
        let v = vec![
            ScoreItem { source: ScoreSource::Past, points: 1.0, chance: 0.5, used_for_fitting: false },
            ScoreItem { source: ScoreSource::Quiz, points: 1.0, chance: 0.2, used_for_fitting: false },
        ];
        let b = scoreboard(&v, Lang::En);
        assert_eq!(b.total, 0);
        assert_eq!(b.fit_total, 2);
        // A sealed prediction scored first and only then used for fitting still counts once.
        let after = ScoreItem { source: ScoreSource::Sealed, points: 1.0, chance: 0.5, used_for_fitting: false };
        assert!(after.counts());
        assert_eq!(scoreboard(&[after], Lang::En).total, 1);
    }

    #[test]
    fn a_score_no_better_than_guessing_is_said_so_kindly() {
        let v: Vec<ScoreItem> = (0..10).map(|i| item(ScoreSource::Sealed, if i % 3 == 0 { 1.0 } else { 0.0 }, 0.5)).collect();
        let b = scoreboard(&v, Lang::En);
        assert!(!b.beats_chance);
        assert!(b.chance_line.contains("no better than guessing"), "{}", b.chance_line);
        assert!(b.note.starts_with("Still learning your chart"));
        // Nothing answered.
        let e = scoreboard(&[], Lang::En);
        assert_eq!((e.total, e.p_luck), (0, 1.0));
        assert!(e.lines.is_empty() && e.share_text.is_empty() && e.fit_line.is_empty());
    }

    #[test]
    fn the_scoreboard_names_no_astrology_and_promises_nothing() {
        let sets: Vec<Vec<ScoreItem>> = vec![
            vec![],
            vec![item(ScoreSource::Past, 1.0, 0.5)],
            (0..20).map(|_| item(ScoreSource::Sealed, 1.0, 0.5)).collect(),
            (0..20).map(|i| item(ScoreSource::Quiz, (i % 2) as f64, 0.2)).collect(),
            (0..8).map(|_| item(ScoreSource::Past, 0.0, 0.5)).collect(),
            (0..4).map(|_| ScoreItem { source: ScoreSource::Sealed, points: 1.0, chance: 0.5, used_for_fitting: true }).collect(),
        ];
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for s in &sets {
                let b = scoreboard(s, lang);
                let how = how_it_works(lang).to_string();
                for t in [&b.headline, &b.chance_line, &b.note, &b.share_text, &b.fit_line, &how]
                    .into_iter()
                    .chain(b.lines.iter().map(|l| &l.words))
                    .chain(b.fit_lines.iter().map(|l| &l.words))
                {
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
