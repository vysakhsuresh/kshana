//! The book of a life: its chapters and pages (the Vimshottari mahadashas and the
//! antardashas of the running one), Saturn's trials (every sade sati and dhaiya from
//! birth, solved to the second), Jupiter's passages over the Moon's houses, and the
//! returns of Saturn, Jupiter and Rahu to their birth places. Past, present and future
//! on one line, each with a tone per life from the birth chart.

use crate::forecast::{date_words, lord_score, Band, Life, Natal};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::plain::Lang;
use crate::transit::{kernel_end, kernel_start, returns, sidereal_longitude, sign_changes};
use kshana_ephem::Spk;
use kshana_time::{Instant, NS_PER_DAY};
use kshana_vedic::dasha::{subdivide, vimshottari, YEAR_365_25_NS};
use kshana_vedic::{Ayanamsa, DashaLord};

/// One chapter (mahadasha) or page (antardasha).
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    /// Lord.
    pub lord: DashaLord,
    /// 1 = mahadasha, 2 = antardasha.
    pub level: u8,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Age at the start, years.
    pub age: f64,
    /// Overall tone, `[0, 1]`.
    pub tone: f64,
    /// Band of the tone.
    pub band: Band,
    /// Tone per life, `Life::ALL` order.
    pub lives: Vec<f64>,
    /// The life it serves best.
    pub best: Life,
    /// The life it tests most.
    pub weak: Life,
    /// Plain words.
    pub words: String,
    /// True when `at` falls inside it.
    pub current: bool,
}

/// A span of Saturn over the Moon's testing houses.
#[derive(Clone, Debug, PartialEq)]
pub struct SaturnSpan {
    /// `sade sati`, `ardhashtama` or `ashtama Shani`.
    pub kind: &'static str,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Age at the start.
    pub age: f64,
    /// Every phase change inside it: `(instant, words)`.
    pub phases: Vec<(Instant, String)>,
    /// True when `at` falls inside it.
    pub current: bool,
    /// Plain words with dates.
    pub words: String,
}

/// Jupiter in one house from the Moon.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Sign.
    pub sign: u8,
    /// House from the natal Moon.
    pub house: u8,
    /// Favourable by the transit tables.
    pub favourable: bool,
    /// True when `at` falls inside it.
    pub current: bool,
    /// Plain words.
    pub words: String,
}

/// A graha back at its birth place.
#[derive(Clone, Debug, PartialEq)]
pub struct Return {
    /// Which graha.
    pub graha: DashaLord,
    /// First, second, third return.
    pub nth: u8,
    /// When.
    pub at: Instant,
    /// Age then.
    pub age: f64,
    /// Plain words.
    pub words: String,
}

/// The book.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    /// Birth.
    pub birth: Instant,
    /// The instant the book is opened at.
    pub at: Instant,
    /// Age now, years.
    pub age: f64,
    /// The mahadashas of the 120-year cycle.
    pub chapters: Vec<Chapter>,
    /// The antardashas of the running mahadasha.
    pub pages: Vec<Chapter>,
    /// Saturn's trials from birth to age 100.
    pub saturn: Vec<SaturnSpan>,
    /// Jupiter's passages, three years back to twelve ahead.
    pub jupiter: Vec<Passage>,
    /// Returns of Saturn, Jupiter and Rahu, chronological.
    pub returns: Vec<Return>,
    /// One or two sentences about now.
    pub headline: String,
    /// The last instant the ephemeris on this device can see; Saturn's spans and the
    /// returns stop here.
    pub horizon: Instant,
}

fn years(from: Instant, to: Instant) -> f64 {
    to.ns_since(from) as f64 / (365.25 * NS_PER_DAY as f64)
}

/// What a phase of a long test brings, as the line in the book.
fn phase_words(ph: SaturnPhase) -> &'static str {
    match ph {
        SaturnPhase::SadeSatiRising => "The first part begins: expenses rise and sleep gets lighter",
        SaturnPhase::SadeSatiPeak => "The hardest part begins: the mind feels heavy and work doubles",
        SaturnPhase::SadeSatiSetting => "The last part begins: the pressure starts to ease",
        SaturnPhase::DhaiyaFourth => "It begins: home and peace of mind are tested",
        SaturnPhase::DhaiyaEighth => "It begins: health and sudden changes need care",
        SaturnPhase::Clear => "The pressure ends",
    }
}

/// [`phase_words`] in `lang`.
fn phase_words_in(lang: Lang, ph: SaturnPhase) -> &'static str {
    let t = match ph {
        SaturnPhase::SadeSatiRising => ["ആദ്യ ഭാഗം തുടങ്ങുന്നു: ചെലവ് കൂടും, ഉറക്കം കുറയും", "पहला हिस्सा शुरू होता है: खर्च बढ़ता है और नींद हल्की होती है"],
        SaturnPhase::SadeSatiPeak => ["ഏറ്റവും കഠിനമായ ഭാഗം തുടങ്ങുന്നു: മനസ്സിന് ഭാരം, ജോലി ഇരട്ടി", "सबसे कठिन हिस्सा शुरू होता है: मन भारी, काम दोगुना"],
        SaturnPhase::SadeSatiSetting => ["അവസാന ഭാഗം തുടങ്ങുന്നു: സമ്മർദ്ദം അയഞ്ഞുവരും", "आख़िरी हिस्सा शुरू होता है: दबाव कम होने लगता है"],
        SaturnPhase::DhaiyaFourth => ["തുടങ്ങുന്നു: വീടും മനസ്സമാധാനവും പരീക്ഷിക്കപ്പെടും", "शुरू होता है: घर और मन की शांति की परीक्षा"],
        SaturnPhase::DhaiyaEighth => ["തുടങ്ങുന്നു: ആരോഗ്യവും പെട്ടെന്നുള്ള മാറ്റങ്ങളും ശ്രദ്ധിക്കണം", "शुरू होता है: सेहत और अचानक बदलावों का ध्यान रखें"],
        SaturnPhase::Clear => ["സമ്മർദ്ദം തീരുന്നു", "दबाव ख़त्म होता है"],
    };
    match lang {
        Lang::En => phase_words(ph),
        Lang::Ml => t[0],
        Lang::Hi => t[1],
    }
}

/// The line for one long test in the book, in `lang`.
#[allow(clippy::too_many_arguments)]
fn span_words(lang: Lang, kind: &str, st: Instant, en: Instant, a: f64, b: f64, current: bool, passed: bool, lon: f64) -> String {
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    match lang {
        Lang::En => format!(
            "{} from {} to {} (age {:.0} to {:.0}){}",
            match kind {
                "sade sati" => "A long testing period of about seven and a half years,",
                "ardhashtama" => "A testing time for home and peace of mind,",
                _ => "A testing time for health and sudden changes,",
            },
            date_words(st, lon),
            date_words(en, lon),
            a,
            b,
            if current {
                format!(". You are in it now; it ends {}.", date_words(en, lon))
            } else if passed {
                ". Passed.".to_string()
            } else {
                ". Ahead.".to_string()
            }
        ),
        Lang::Ml => format!(
            "{}: {} മുതൽ {} വരെ ({:.0} മുതൽ {:.0} വയസ്സു വരെ){}",
            match kind {
                "sade sati" => "ഏകദേശം ഏഴര വർഷം നീളുന്ന ഒരു നീണ്ട പരീക്ഷണകാലം",
                "ardhashtama" => "വീടിനും മനസ്സമാധാനത്തിനും ഒരു പരീക്ഷണകാലം",
                _ => "ആരോഗ്യത്തിനും പെട്ടെന്നുള്ള മാറ്റങ്ങൾക്കും ഒരു പരീക്ഷണകാലം",
            },
            d(st),
            d(en),
            a,
            b,
            if current {
                format!(". നിങ്ങൾ ഇപ്പോൾ ഇതിലാണ്; ഇത് {}-ന് തീരും.", d(en))
            } else if passed {
                ". കടന്നുപോയി.".to_string()
            } else {
                ". വരാനിരിക്കുന്നു.".to_string()
            }
        ),
        Lang::Hi => format!(
            "{}: {} से {} तक ({:.0} से {:.0} साल की उम्र){}",
            match kind {
                "sade sati" => "सात साल से कुछ ज़्यादा का एक लंबा कठिन दौर",
                "ardhashtama" => "घर और मन की शांति के लिए कठिन समय",
                _ => "सेहत और अचानक बदलावों के लिए कठिन समय",
            },
            d(st),
            d(en),
            a,
            b,
            if current {
                format!("। आप अभी इसी में हैं; यह {} को ख़त्म होगा।", d(en))
            } else if passed {
                "। बीत गया।".to_string()
            } else {
                "। आगे है।".to_string()
            }
        ),
    }
}

#[allow(dead_code)]
pub(crate) fn lord_theme(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "years of standing and authority: the father, the state, the self",
        DashaLord::Moon => "years of home and mind: mother, feeling, the public",
        DashaLord::Mars => "years of effort, property and courage; keep the temper",
        DashaLord::Mercury => "years of learning, trade and talk",
        DashaLord::Jupiter => "years of growth: teachers, children, faith and money",
        DashaLord::Venus => "years of love, comfort, art and partnership",
        DashaLord::Saturn => "years of work and duty; slow, lasting gains",
        DashaLord::Rahu => "years of ambition, foreign things and sudden turns",
        DashaLord::Ketu => "years of letting go, insight and the inner road",
    }
}

#[allow(clippy::too_many_arguments)]
fn chapter(lang: Lang, natal: &Natal, lord: DashaLord, parent: Option<DashaLord>, level: u8, start: Instant, end: Instant, at: Instant) -> Chapter {
    let lives: Vec<f64> = Life::ALL
        .iter()
        .map(|&life| {
            let (s, _) = lord_score(natal, life, lord);
            match parent {
                Some(p) => 0.5 * s + 0.5 * lord_score(natal, life, p).0,
                None => s,
            }
        })
        .collect();
    let tone = lives.iter().sum::<f64>() / lives.len() as f64;
    let (bi, _) = lives.iter().enumerate().fold((0, f64::MIN), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    let (wi, _) = lives.iter().enumerate().fold((0, f64::MAX), |a, (i, &v)| if v < a.1 { (i, v) } else { a });
    let best = Life::ALL[bi];
    let weak = Life::ALL[wi];
    let band = Band::of(tone);
    let bi = match band {
        Band::Bright => 0,
        Band::Good => 1,
        Band::Steady => 2,
        Band::Slow => 3,
        Band::Hold => 4,
    };
    let words = match lang {
        Lang::Ml => format!(
            "{} {} എന്നിവയെക്കുറിച്ചാണ്. മൊത്തത്തിൽ {}; ഏറ്റവും നല്ലത് {}, ക്ഷമ വേണ്ടത് {}.",
            if level == 1 { "ഈ അധ്യായം" } else { "ഈ ഘട്ടം" },
            crate::lang::chapter_theme(lang, lord),
            ["ഏറെ നൽകുന്ന കാലം", "നല്ല കാലം", "സമനിലയുള്ള കാലം", "മന്ദമായ കാലം", "ക്ഷമ വേണ്ട കാലം"][bi],
            crate::lang::life_name(lang, best),
            crate::lang::life_name(lang, weak)
        ),
        Lang::Hi => format!(
            "{} {} के बारे में है। कुल मिलाकर {}; सबसे अच्छा {}, धैर्य चाहिए {} में।",
            if level == 1 { "यह अध्याय" } else { "यह दौर" },
            crate::lang::chapter_theme(lang, lord),
            ["देने वाला समय", "अच्छा समय", "बराबर समय", "धीमा समय", "धैर्य का समय"][bi],
            crate::lang::life_name(lang, best),
            crate::lang::life_name(lang, weak)
        ),
        Lang::En => format!(
        "{} {}. {} for {}; {} asks patience.",
        if level == 1 { "A chapter about" } else { "A stretch about" },
        crate::plain::chapter_theme(lord),
        match band {
            Band::Bright => "Giving",
            Band::Good => "Good",
            Band::Steady => "Even",
            Band::Slow => "Slow",
            Band::Hold => "Patient",
        },
        best.name().to_ascii_lowercase(),
        crate::plain::subject(&weak.name().to_ascii_lowercase())
    ),
    };
    Chapter {
        lord,
        level,
        start,
        end,
        age: years(natal.birth, start),
        tone,
        band,
        lives,
        best,
        weak,
        words,
        current: start <= at && at < end,
    }
}

/// Open the book at `at`. `longitude_deg` only places the dates in words.
pub fn timeline(spk: &Spk, natal: &Natal, ayanamsa: Ayanamsa, longitude_deg: f64, at: Instant) -> Timeline {
    timeline_in(Lang::En, spk, natal, ayanamsa, longitude_deg, at)
}

/// [`timeline`] with every line in `lang`.
pub fn timeline_in(lang: Lang, spk: &Spk, natal: &Natal, ayanamsa: Ayanamsa, longitude_deg: f64, at: Instant) -> Timeline {
    let birth = natal.birth;
    let dl = |t: Instant| crate::lang::date(lang, t, longitude_deg);
    let mahas = vimshottari(natal.moon_sidereal, birth, YEAR_365_25_NS);
    let chapters: Vec<Chapter> = mahas
        .iter()
        .map(|p| chapter(lang, natal, p.lord, None, 1, p.start.max(birth), p.end, at))
        .collect();
    let running = mahas.iter().copied().find(|p| p.start <= at && at < p.end);
    let pages: Vec<Chapter> = running
        .map(|p| {
            subdivide(p)
                .into_iter()
                .map(|q| chapter(lang, natal, q.lord, Some(p.lord), 2, q.start.max(birth), q.end, at))
                .collect()
        })
        .unwrap_or_default();

    // Saturn from birth to age 100, or as far as the kernel on this device sees.
    let horizon = kernel_end(spk);
    let end100 = birth
        .add_ns((100.0 * 365.25 * NS_PER_DAY as f64) as i128)
        .min(horizon);
    let sat_changes = sign_changes(spk, ayanamsa, DashaLord::Saturn, birth, end100);
    let sat_sign0 = sidereal_longitude(spk, ayanamsa, DashaLord::Saturn, birth)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let ms = natal.moon_sign();
    let house_of = |sign: u8| ((sign as i64 - ms).rem_euclid(12) + 1) as u8;
    // Intervals of Saturn's sign: (start, end, sign).
    let mut intervals: Vec<(Instant, Instant, u8)> = Vec::new();
    let mut t = birth;
    let mut s = sat_sign0;
    for c in &sat_changes {
        intervals.push((t, c.at, s));
        t = c.at;
        s = c.to;
    }
    intervals.push((t, end100, s));
    let mut saturn: Vec<SaturnSpan> = Vec::new();
    let kind_of = |h: u8| -> Option<&'static str> {
        match h {
            12 | 1 | 2 => Some("sade sati"),
            4 => Some("ardhashtama"),
            8 => Some("ashtama Shani"),
            _ => None,
        }
    };
    let mut i = 0;
    while i < intervals.len() {
        let (st, _, sg) = intervals[i];
        let h = house_of(sg);
        if let Some(kind) = kind_of(h) {
            // extend while the kind holds (retrograde re-entries included)
            let mut j = i;
            let mut phases: Vec<(Instant, String)> = Vec::new();
            while j < intervals.len() && kind_of(house_of(intervals[j].2)) == Some(kind) {
                let hh = house_of(intervals[j].2);
                let ph = SaturnPhase::from_house(hh);
                phases.push((
                    intervals[j].0,
                    phase_words_in(lang, ph).to_string(),
                ));
                j += 1;
            }
            let en = intervals[j - 1].1;
            let current = st <= at && at < en;
            let words = span_words(lang, kind, st, en, years(birth, st), years(birth, en), current, en <= at, longitude_deg);
            saturn.push(SaturnSpan {
                kind,
                start: st,
                end: en,
                age: years(birth, st),
                phases,
                current,
                words,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    // A retrograde loop can carry Saturn back out of the span for a few months and in
    // again; that is one trial, not two. Merge same-kind spans separated by under 1.5 years.
    let mut merged: Vec<SaturnSpan> = Vec::new();
    for s in saturn {
        if let Some(last) = merged.last_mut() {
            if last.kind == s.kind && years(last.end, s.start) < 1.5 {
                let months = (years(last.end, s.start) * 12.0).round() as i64;
                last.phases.push((
                    last.end,
                    match lang {
                        Lang::En => format!("The pressure lifts for about {months} month{}, then returns", if months == 1 { "" } else { "s" }),
                        Lang::Ml => format!("സമ്മർദ്ദം ഏകദേശം {months} മാസം അയയും, പിന്നെ തിരിച്ചുവരും"),
                        Lang::Hi => format!("दबाव लगभग {months} महीने के लिए हटता है, फिर लौटता है"),
                    },
                ));
                let (from, to) = match lang {
                    Lang::En => (" begins", " resumes"),
                    Lang::Ml => ("തുടങ്ങുന്നു", "വീണ്ടും തുടങ്ങുന്നു"),
                    Lang::Hi => ("शुरू होता है", "फिर शुरू होता है"),
                };
                last.phases.extend(
                    s.phases
                        .into_iter()
                        .map(|(t, w)| (t, w.replacen(from, to, 1))),
                );
                last.end = s.end;
                last.current = last.start <= at && at < last.end;
                last.words = span_words(lang, last.kind, last.start, last.end, years(birth, last.start), years(birth, last.end), last.current, last.end <= at, longitude_deg);
                continue;
            }
        }
        merged.push(s);
    }
    let saturn = merged;

    // Jupiter: three years back to twelve ahead.
    let jfrom = at.add_ns(-3 * 365 * NS_PER_DAY).max(kernel_start(spk));
    let jto = at.add_ns(12 * 365 * NS_PER_DAY).min(horizon);
    let jch = sign_changes(spk, ayanamsa, DashaLord::Jupiter, jfrom, jto);
    let jsign0 = sidereal_longitude(spk, ayanamsa, DashaLord::Jupiter, jfrom)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let mut jupiter = Vec::new();
    let mut t = jfrom;
    let mut s = jsign0;
    let mut push = |start: Instant, end: Instant, sign: u8| {
        let h = house_of(sign);
        let fav = favourable_from_moon(DashaLord::Jupiter).contains(&h);
        jupiter.push(Passage {
            start,
            end,
            sign,
            house: h,
            favourable: fav,
            current: start <= at && at < end,
            words: if fav {
                lang.pick([
                    "A giving year: growth, help and good news come easier",
                    "നൽകുന്ന വർഷം: വളർച്ചയും സഹായവും നല്ല വാർത്തകളും എളുപ്പം വരും",
                    "देने वाला साल: तरक्की, मदद और अच्छी ख़बरें आसानी से आती हैं",
                ])
                .to_string()
            } else {
                lang.pick(match h {
                    1 => [
                        "A year of expense and focus on yourself: spend with care",
                        "ചെലവിന്റെയും സ്വന്തം കാര്യങ്ങളിലുള്ള ശ്രദ്ധയുടെയും വർഷം: ശ്രദ്ധിച്ച് ചെലവാക്കുക",
                        "खर्च और अपने आप पर ध्यान का साल: सोच-समझकर खर्च करें",
                    ],
                    4 | 8 | 12 => ["A quiet year: build inside, not outside", "ശാന്തമായ വർഷം: പുറത്തല്ല, ഉള്ളിൽ പണിയുക", "शांत साल: बाहर नहीं, भीतर बनाएँ"],
                    _ => ["An even year for growth", "വളർച്ചയ്ക്ക് സമനിലയുള്ള വർഷം", "तरक्की के लिए बराबर साल"],
                })
                .to_string()
            },
        });
    };
    for c in &jch {
        push(t, c.at, s);
        t = c.at;
        s = c.to;
    }
    push(t, jto, s);

    // Returns.
    let mut rets: Vec<Return> = Vec::new();
    for g in [DashaLord::Saturn, DashaLord::Jupiter, DashaLord::Rahu] {
        let Some(natal_lon) = sidereal_longitude(spk, ayanamsa, g, birth) else { continue };
        // A graha re-crossing its natal degree in the months after birth is a loop, not a return.
        let v = returns(spk, ayanamsa, g, natal_lon, birth.add_ns(2 * 365 * NS_PER_DAY), end100);
        // A retrograde loop gives three crossings within months; count them as one return.
        let mut n = 0u8;
        let mut last: Option<Instant> = None;
        for t in v {
            if let Some(l) = last {
                if years(l, t) < 1.5 {
                    continue;
                }
            }
            n += 1;
            last = Some(t);
            let age = years(birth, t);
            rets.push(Return {
                graha: g,
                nth: n,
                at: t,
                age,
                words: {
                    let gi = match g {
                        DashaLord::Saturn => 0,
                        DashaLord::Jupiter => 1,
                        _ => 2,
                    };
                    let ni = (n as usize).clamp(1, 4) - 1;
                    match lang {
                        Lang::En => format!(
                            "{} {}, {} (age {:.1}): {}",
                            ["First", "Second", "Third", "Fourth"][ni],
                            ["thirty-year reckoning", "twelve-year renewal", "eighteen-year turn"][gi],
                            date_words(t, longitude_deg),
                            age,
                            [
                                "the structure of your life is tested and rebuilt; what is true stays",
                                "a year of expansion, teachers and second chances",
                                "an old hunger comes round, and a chance to answer it",
                            ][gi]
                        ),
                        Lang::Ml => format!(
                            "{} {}, {} ({:.1} വയസ്സ്): {}",
                            ["ആദ്യത്തെ", "രണ്ടാമത്തെ", "മൂന്നാമത്തെ", "നാലാമത്തെ"][ni],
                            ["മുപ്പതു വർഷത്തെ കണക്കെടുപ്പ്", "പന്ത്രണ്ടു വർഷത്തെ പുതുക്കൽ", "പതിനെട്ടു വർഷത്തെ വഴിത്തിരിവ്"][gi],
                            dl(t),
                            age,
                            [
                                "ജീവിതത്തിന്റെ ഘടന പരീക്ഷിക്കപ്പെട്ട് പുതുക്കിപ്പണിയപ്പെടും; സത്യമായത് നിലനിൽക്കും",
                                "വികാസത്തിന്റെയും അധ്യാപകരുടെയും രണ്ടാമൂഴങ്ങളുടെയും വർഷം",
                                "പഴയൊരു ആഗ്രഹം തിരികെ വരും, അതിന് മറുപടി നൽകാൻ ഒരവസരവും",
                            ][gi]
                        ),
                        Lang::Hi => format!(
                            "{} {}, {} ({:.1} साल की उम्र): {}",
                            ["पहला", "दूसरा", "तीसरा", "चौथा"][ni],
                            ["तीस साल का हिसाब", "बारह साल का नवीनीकरण", "अठारह साल का मोड़"][gi],
                            dl(t),
                            age,
                            [
                                "ज़िंदगी के ढाँचे की परीक्षा होती है और वह फिर से बनता है; जो सच्चा है वह टिकता है",
                                "फैलाव, शिक्षकों और दूसरे मौक़ों का साल",
                                "एक पुरानी भूख लौटती है, और उसका जवाब देने का मौक़ा भी",
                            ][gi]
                        ),
                    }
                },
            });
        }
    }
    rets.sort_by(|a, b| a.at.cmp(&b.at));

    let age = years(birth, at);
    let cur = chapters.iter().find(|c| c.current);
    let page = pages.iter().find(|c| c.current);
    let sat_now = saturn.iter().find(|s| s.current);
    let next_ret = rets.iter().find(|r| r.at > at);
    let mut headline = match (lang, cur, page) {
        (Lang::Ml, Some(c), p) => {
            let mut s = format!("നിങ്ങൾക്ക് {age:.1} വയസ്സ്. {} വരെ നിങ്ങളുടെ ജീവിതം {} എന്നിവയെ ചുറ്റിയാണ്.", dl(c.end), crate::lang::chapter_theme(lang, c.lord));
            if let Some(p) = p {
                s.push_str(&format!(" {} വരെ ഏറ്റവും കൂടുതൽ ചലനം {} എന്നിവയിലാണ്.", dl(p.end), crate::lang::chapter_theme(lang, p.lord)));
            }
            s
        }
        (Lang::Hi, Some(c), p) => {
            let mut s = format!("आपकी उम्र {age:.1} साल है। {} तक आपकी ज़िंदगी {} के इर्द-गिर्द है।", dl(c.end), crate::lang::chapter_theme(lang, c.lord));
            if let Some(p) = p {
                s.push_str(&format!(" {} तक सबसे ज़्यादा हलचल {} में है।", dl(p.end), crate::lang::chapter_theme(lang, p.lord)));
            }
            s
        }
        (Lang::Ml, None, _) => format!("നിങ്ങൾക്ക് {age:.1} വയസ്സ്."),
        (Lang::Hi, None, _) => format!("आपकी उम्र {age:.1} साल है।"),
        (_, Some(c), Some(p)) => format!(
            "You are {:.1}. Until {} your life centres on {}. Until {} the most movement is in {}.",
            age,
            date_words(c.end, longitude_deg),
            crate::plain::chapter_theme(c.lord),
            date_words(p.end, longitude_deg),
            crate::plain::chapter_theme(p.lord)
        ),
        (_, Some(c), None) => format!("You are {:.1}. Until {} your life centres on {}.", age, date_words(c.end, longitude_deg), crate::plain::chapter_theme(c.lord)),
        _ => format!("You are {age:.1}."),
    };
    if let Some(s) = sat_now {
        headline.push(' ');
        headline.push_str(&s.words);
    }
    if let Some(r) = next_ret {
        let first = r.words.split(':').next().unwrap_or("");
        headline.push_str(&match lang {
            Lang::En => format!(" Next: {first}."),
            Lang::Ml => format!(" അടുത്തത്: {first}."),
            Lang::Hi => format!(" आगे: {first}।"),
        });
    }

    Timeline {
        birth,
        at,
        age,
        chapters,
        pages,
        saturn,
        jupiter,
        returns: rets,
        headline,
        horizon,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_book_has_chapters_trials_passages_and_returns() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let t0 = std::time::Instant::now();
        let tl = timeline(&spk, &natal, Ayanamsa::Lahiri, 76.2673, at);
        let dt = t0.elapsed();
        assert_eq!(tl.chapters.len(), 9);
        assert_eq!(tl.chapters.iter().filter(|c| c.current).count(), 1);
        assert_eq!(tl.pages.len(), 9);
        assert_eq!(tl.pages.iter().filter(|c| c.current).count(), 1);
        // Saturn's 29.5-year cycle gives three or four sade satis in a hundred years.
        let ss: Vec<&SaturnSpan> = tl.saturn.iter().filter(|s| s.kind == "sade sati").collect();
        assert!(ss.len() >= 3 && ss.len() <= 4, "{}", ss.len());
        let end100 = birth.add_ns((100.0 * 365.25 * NS_PER_DAY as f64) as i128);
        let whole = |s: &SaturnSpan| s.start > birth.add_ns(NS_PER_DAY) && s.end < end100.add_ns(-NS_PER_DAY);
        for s in ss.iter().filter(|s| whole(s)) {
            let len = years(s.start, s.end);
            assert!(len > 6.0 && len < 9.5, "sade sati {len} years: {}", s.words);
            assert!(s.phases.len() >= 3);
        }
        // and the ardhashtama and ashtama spans, each about 2.5 years
        for s in tl.saturn.iter().filter(|s| s.kind != "sade sati" && whole(s)) {
            let len = years(s.start, s.end);
            assert!(len > 1.5 && len < 4.0, "{} {len}", s.kind);
        }
        for s in &tl.saturn {
            eprintln!("{}", s.words);
        }
        assert!(tl.saturn.windows(2).all(|w| w[0].end <= w[1].start));
        assert!(tl.jupiter.len() >= 12 && tl.jupiter.len() <= 48, "{}", tl.jupiter.len());
        assert_eq!(tl.jupiter.iter().filter(|p| p.current).count(), 1);
        // The horizon is the kernel end (DE421 in tests: 2053; the app ships DE440s: 2150):
        // to 2053, two Saturn returns, five Jupiter, three Rahu for a 1990 birth.
        let seen = years(birth, tl.horizon);
        assert!(seen > 60.0, "{seen}");
        assert!(tl.returns.iter().all(|r| r.at <= tl.horizon));
        let sat_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Saturn).collect();
        assert_eq!(sat_ret.len(), (seen / 29.46) as usize, "{:?}", sat_ret);
        assert!((sat_ret[0].age - 29.5).abs() < 1.0, "{}", sat_ret[0].age);
        let jup_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Jupiter).collect();
        assert_eq!(jup_ret.len(), (seen / 11.86) as usize, "{:?}", jup_ret.iter().map(|r| r.age).collect::<Vec<_>>());
        assert!((jup_ret[0].age - 11.86).abs() < 1.0, "{}", jup_ret[0].age);
        let rahu_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Rahu).collect();
        assert_eq!(rahu_ret.len(), (seen / 18.61) as usize, "{:?}", rahu_ret.iter().map(|r| r.age).collect::<Vec<_>>());
        assert!((rahu_ret[0].age - 18.6).abs() < 0.8, "{}", rahu_ret[0].age);
        assert!(!tl.headline.is_empty());
        eprintln!("timeline in {dt:?}");
    }
}
