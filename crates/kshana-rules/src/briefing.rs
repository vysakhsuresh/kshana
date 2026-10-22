//! The briefing: what an astrologer would tell a person across the table, in plain
//! sentences, every one of them traceable. Where the Moon stands at birth (so the
//! reading can be checked against any other), which periods run and until when,
//! where Saturn is relative to the Moon and the day it moves on, how the moment
//! reads and why, and the days to circle in the month ahead. Used on the Loop for one
//! person and under Us for each of the two.

use crate::explain::explain_in;
use crate::forecast::{date_words, moment_in, series, Band, ForecastError, Life, Moment, Natal, SIGN_NAMES};
use crate::gochara::SaturnPhase;
use crate::parihara::parihara_from_facts_in;
use crate::plain::Lang;
use crate::transit::{next_sign_change, sign_changes};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, vimshottari_levels, Ayanamsa, DashaLord};

/// A day worth circling for one life.
#[derive(Clone, Debug, PartialEq)]
pub struct Circled {
    /// Which life.
    pub life: Life,
    /// The day (its sample instant).
    pub at: Instant,
    /// Score that day, `[0, 1]`.
    pub score: f64,
    /// Why, in words.
    pub words: String,
}

/// The remedy in brief: the graha, the cause, the first three things to do.
#[derive(Clone, Debug, PartialEq)]
pub struct RemedyBrief {
    /// The graha addressed.
    pub planet: DashaLord,
    /// Why this graha.
    pub cause: String,
    /// `(kind, title, why)` of the three best-fitting remedies.
    pub top: Vec<(String, String, String)>,
    /// How many the full dictionary holds for this.
    pub total: usize,
}

/// The briefing for one person at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Briefing {
    /// The instant.
    pub at: Instant,
    /// "Sagittarius · Mula": the birth Moon.
    pub moon: String,
    /// The birth Moon's sidereal sign, 0 = Aries.
    pub moon_sign: u8,
    /// "Moon mahadasha (to 10 Oct 2030) › Mercury antardasha (to 9 Jan 2028)".
    pub dasha: String,
    /// Saturn relative to the Moon.
    pub saturn: SaturnPhase,
    /// Saturn's phase in words, with its end date when it is active.
    pub saturn_words: String,
    /// When Saturn next changes sign.
    pub saturn_until: Option<Instant>,
    /// The paragraphs, in reading order.
    pub paragraphs: Vec<String>,
    /// Days to circle in the next thirty: the best day for each of the chosen lives.
    pub circled: Vec<Circled>,
    /// The remedy in brief for the weakest of the chosen lives.
    pub remedy: Option<RemedyBrief>,
    /// The full forecast of the instant.
    pub moment: Moment,
}

/// Saturn's phase named with its exit date, or its clearance.
pub fn saturn_sentence(spk: &Spk, ayanamsa: Ayanamsa, natal: &Natal, phase: SaturnPhase, at: Instant, lon: f64) -> (String, Option<Instant>) {
    saturn_sentence_in(Lang::En, spk, ayanamsa, natal, phase, at, lon)
}

/// The long test in the reader's language: what it is, until when, what it brings, and
/// any pause on the way.
pub fn saturn_sentence_in(lang: Lang, spk: &Spk, ayanamsa: Ayanamsa, natal: &Natal, phase: SaturnPhase, at: Instant, lon: f64) -> (String, Option<Instant>) {
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    let next = next_sign_change(spk, ayanamsa, DashaLord::Saturn, at);
    match phase {
        SaturnPhase::Clear => {
            let none_now = lang.pick([
                "No long testing time is on you now",
                "ഇപ്പോൾ നീണ്ട പരീക്ഷണകാലമൊന്നുമില്ല",
                "अभी कोई लंबा कठिन दौर नहीं है",
            ]);
            let words = match next {
                Some(c) => {
                    let h = ((c.to as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                    let coming = SaturnPhase::from_house(h);
                    if coming != SaturnPhase::Clear {
                        let t = crate::lang::long_test(lang, coming);
                        let pr = crate::lang::long_test_prediction(lang, coming);
                        match lang {
                            Lang::En => format!("{none_now}. From {} {t} begins. {pr}", d(c.at)),
                            Lang::Ml => format!("{none_now}. {} മുതൽ {t} തുടങ്ങും. {pr}", d(c.at)),
                            Lang::Hi => format!("{none_now}। {} से: {t}। {pr}", d(c.at)),
                        }
                    } else {
                        match lang {
                            Lang::En => format!("{none_now}, and none begins before {}", d(c.at)),
                            Lang::Ml => format!("{none_now}, {} വരെ ഒന്നും തുടങ്ങുകയുമില്ല", d(c.at)),
                            Lang::Hi => format!("{none_now}, और {} से पहले कोई शुरू नहीं होगा", d(c.at)),
                        }
                    }
                }
                None => none_now.to_string(),
            };
            (words.trim_end_matches(['.', '।']).to_string(), next.map(|c| c.at))
        }
        p => {
            // The trial is the same kind of phase (sade sati: 12, 1, 2; a dhaiya: its one
            // house). Walk Saturn's sign changes ahead: a retrograde step out and back
            // within 1.5 years is a pause, not the end. The end is the last exit.
            let kind = |ph: SaturnPhase| match ph {
                SaturnPhase::SadeSatiRising | SaturnPhase::SadeSatiPeak | SaturnPhase::SadeSatiSetting => 1,
                SaturnPhase::DhaiyaFourth => 2,
                SaturnPhase::DhaiyaEighth => 3,
                SaturnPhase::Clear => 0,
            };
            let my_kind = kind(p);
            let changes = sign_changes(spk, ayanamsa, DashaLord::Saturn, at, at.add_ns(4_000 * NS_PER_DAY));
            let house_of = |sign: u8| ((sign as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
            // (instant, inside the trial after this change)
            let steps: Vec<(Instant, bool, u8)> = changes
                .iter()
                .map(|c| (c.at, kind(SaturnPhase::from_house(house_of(c.to))) == my_kind, c.to))
                .collect();
            let mut end: Option<(Instant, u8)> = None;
            let mut pauses: Vec<(Instant, Instant)> = Vec::new();
            let mut i = 0;
            while i < steps.len() {
                let (t, inside, sign) = steps[i];
                if !inside {
                    // find the next re-entry, if any, within 1.5 years
                    let re = steps[i + 1..].iter().find(|s| s.1).map(|s| s.0);
                    match re {
                        Some(r) if r.ns_since(t) < (548 * NS_PER_DAY) => {
                            pauses.push((t, r));
                        }
                        _ => {
                            end = Some((t, sign));
                            break;
                        }
                    }
                }
                i += 1;
            }
            let words = match end {
                Some((t, _sign)) => {
                    let mut s = format!(
                        "{} runs until {}. {}",
                        capitalise(crate::plain::long_test(p)),
                        date_words(t, lon),
                        crate::plain::long_test_prediction(p)
                    );
                    if lang != Lang::En {
                        let lt = crate::lang::long_test(lang, p);
                        let pr = crate::lang::long_test_prediction(lang, p);
                        s = match lang {
                            Lang::Ml => format!("{} വരെ: {lt}. {pr}", d(t)),
                            _ => format!("{} तक: {lt}। {pr}", d(t)),
                        };
                    }
                    if let Some((a, b)) = pauses.first() {
                        s.push_str(&match lang {
                            Lang::En => format!(" Between {} and {} it lifts for a while, then returns for its last stretch.", d(*a), d(*b)),
                            Lang::Ml => format!(" {} മുതൽ {} വരെ ഇത് കുറച്ചുകാലം അയയും, പിന്നെ അവസാന ഭാഗത്തിനായി തിരിച്ചുവരും.", d(*a), d(*b)),
                            Lang::Hi => format!(" {} से {} तक यह कुछ समय के लिए हल्का होगा, फिर आख़िरी हिस्से के लिए लौटेगा।", d(*a), d(*b)),
                        });
                    }
                    s.trim_end_matches(['.', '।']).to_string()
                }
                None => match lang {
                    Lang::En => format!("{}. {}", capitalise(crate::plain::long_test(p)), crate::plain::long_test_prediction(p)),
                    Lang::Ml => format!("{}. {}", crate::lang::long_test(lang, p), crate::lang::long_test_prediction(lang, p)),
                    Lang::Hi => format!("{}। {}", crate::lang::long_test(lang, p), crate::lang::long_test_prediction(lang, p)),
                }
                .trim_end_matches(['.', '।'])
                .to_string(),
            };
            (words, end.map(|e| e.0))
        }
    }
}

/// The season paragraph in the reader's language: the long chapter and what it brings,
/// the stretch inside it, and the long test (`saturn_words`, already in that language).
pub fn season_paragraph_in(lang: Lang, natal: &Natal, at: Instant, lon: f64, saturn_words: &str) -> String {
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    let stop = if lang == Lang::Hi { "।" } else { "." };
    match (levels.first(), levels.get(1)) {
        (Some(maha), Some(antar)) => {
            let theme = crate::lang::chapter_theme(lang, maha.lord);
            let pred = crate::lang::chapter_prediction(lang, maha.lord);
            let mut s = match lang {
                Lang::En => format!("Until {} your life centres on {theme}. {pred}", d(maha.end)),
                Lang::Ml => format!("{} വരെ നിങ്ങളുടെ ജീവിതം {theme} എന്നിവയെ ചുറ്റിയാണ്. {pred}", d(maha.end)),
                Lang::Hi => format!("{} तक आपकी ज़िंदगी {theme} के इर्द-गिर्द है। {pred}", d(maha.end)),
            };
            if antar.lord != maha.lord {
                let t2 = crate::lang::chapter_theme(lang, antar.lord);
                s.push_str(&match lang {
                    Lang::En => format!(" Until {} the most movement is in {t2}.", d(antar.end)),
                    Lang::Ml => format!(" {} വരെ ഏറ്റവും കൂടുതൽ ചലനം {t2} എന്നിവയിലാണ്.", d(antar.end)),
                    Lang::Hi => format!(" {} तक सबसे ज़्यादा हलचल {t2} में है।", d(antar.end)),
                });
            }
            s.push_str(&format!(" {saturn_words}{stop}"));
            s
        }
        _ => format!("{saturn_words}{stop}"),
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Brief `natal` at `at`, standing at `obs`. `lives` are the lives to circle days for
/// and to pick the remedy from (the Loop passes four; Us passes Love alone).
#[allow(clippy::too_many_arguments)]
pub fn briefing(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    lives: &[Life],
    at: Instant,
) -> Result<Briefing, ForecastError> {
    briefing_in(Lang::En, spk, eop, natal, obs, system, ayanamsa, lives, at)
}

/// A graha's name in the reader's script, for the expert line of the periods.
fn graha_name(lang: Lang, l: DashaLord) -> String {
    let i = DashaLord::ORDER.iter().position(|x| *x == l).unwrap_or(0);
    // ORDER is Ketu, Venus, Sun, Moon, Mars, Rahu, Jupiter, Saturn, Mercury.
    const ML: [&str; 9] = ["കേതു", "ശുക്രൻ", "സൂര്യൻ", "ചന്ദ്രൻ", "ചൊവ്വ", "രാഹു", "വ്യാഴം", "ശനി", "ബുധൻ"];
    const HI: [&str; 9] = ["केतु", "शुक्र", "सूर्य", "चंद्र", "मंगल", "राहु", "गुरु", "शनि", "बुध"];
    match lang {
        Lang::En => format!("{l:?}"),
        Lang::Ml => ML[i].to_string(),
        Lang::Hi => HI[i].to_string(),
    }
}

/// [`briefing`] with every sentence in `lang`.
#[allow(clippy::too_many_arguments)]
pub fn briefing_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    lives: &[Life],
    at: Instant,
) -> Result<Briefing, ForecastError> {
    let lon = obs.longitude_deg;
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    let stop = if lang == Lang::Hi { "।" } else { "." };
    let m = moment_in(lang, spk, eop, natal, obs, system, ayanamsa, at)?;
    let n = nakshatra(natal.moon_sidereal);
    let moon_sign = (natal.moon_sign().rem_euclid(12)) as u8;
    let moon = format!("{} · {}", SIGN_NAMES[moon_sign as usize], NAKSHATRA_NAMES[n.index]);
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    let dasha = levels
        .iter()
        .enumerate()
        .take(2)
        .map(|(i, p)| match lang {
            Lang::En => format!("{:?} {} (to {})", p.lord, ["mahadasha", "antardasha"][i], date_words(p.end, lon)),
            Lang::Ml => format!("{} {} ({} വരെ)", graha_name(lang, p.lord), ["ദശ", "അപഹാരം"][i], d(p.end)),
            Lang::Hi => format!("{} {} ({} तक)", graha_name(lang, p.lord), ["महादशा", "अंतर्दशा"][i], d(p.end)),
        })
        .collect::<Vec<_>>()
        .join(" › ");
    let (saturn_words, saturn_until) = saturn_sentence_in(lang, spk, ayanamsa, natal, m.saturn, at, lon);

    let mut paragraphs = Vec::new();
    // Paragraph 1: the season of life, as what it brings and until when.
    paragraphs.push(season_paragraph_in(lang, natal, at, lon, &saturn_words));
    // Paragraph 3: the moment.
    let top = m.lives.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    let low = m.lives.iter().min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    if let (Some(t), Some(l)) = (top, low) {
        let hora = m
            .hora
            .as_ref()
            .map(|h| match lang {
                Lang::En => format!(" Right now it is {}.", crate::plain::hour_nature(h.lord)),
                Lang::Ml => format!(" ഇപ്പോൾ {}.", crate::lang::hour_nature(lang, h.lord)),
                Lang::Hi => format!(" अभी {}।", crate::lang::hour_nature(lang, h.lord)),
            })
            .unwrap_or_default();
        let (tn, ln) = (crate::lang::life_name(lang, t.life), crate::lang::life_name(lang, l.life));
        let dq = crate::lang::day_quality(lang, m.tara);
        paragraphs.push(match lang {
            Lang::En => format!(
                "Today is {}. {} is your strongest side now: {}. {} needs the most care: {}.{}",
                dq,
                t.life.name(),
                t.because,
                l.life.name(),
                l.because,
                hora
            ),
            Lang::Ml => format!(
                "ഇന്ന് {dq}. ഇപ്പോൾ ഏറ്റവും ശക്തമായ ഭാഗം {tn}: {}. ഏറ്റവും ശ്രദ്ധ വേണ്ടത് {ln}: {}.{hora}",
                t.because, l.because
            ),
            Lang::Hi => format!(
                "आज {dq}। अभी आपका सबसे मज़बूत पहलू {tn} है: {}। सबसे ज़्यादा ध्यान {ln} को चाहिए: {}।{hora}",
                t.because, l.because
            ),
        });
    }

    // Days to circle: the best day in the next thirty for each chosen life.
    let samples = series(spk, eop, natal, obs, ayanamsa, at.add_ns(NS_PER_DAY), NS_PER_DAY, 30)?;
    let mut circled = Vec::new();
    for &life in lives {
        let idx = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        if let Some(best) = samples
            .iter()
            .max_by(|a, b| a.scores[idx].partial_cmp(&b.scores[idx]).unwrap_or(std::cmp::Ordering::Equal))
        {
            let s = best.scores[idx];
            circled.push(Circled {
                life,
                at: best.at,
                score: s,
                words: format!(
                    "{}: {}, {}{stop} {}",
                    crate::lang::life_name(lang, life),
                    d(best.at),
                    crate::lang::day_short(lang, best.tara),
                    crate::forecast::headline_in(lang, life, Band::of(s), (best.at.tai_ns() / NS_PER_DAY) as u64)
                ),
            });
        }
    }

    // The remedy in brief for the weakest chosen life, when it is below good.
    let weakest = lives
        .iter()
        .copied()
        .min_by(|a, b| {
            let ia = Life::ALL.iter().position(|l| l == a).unwrap_or(0);
            let ib = Life::ALL.iter().position(|l| l == b).unwrap_or(0);
            m.lives[ia].score.partial_cmp(&m.lives[ib].score).unwrap_or(std::cmp::Ordering::Equal)
        });
    let remedy = weakest.and_then(|life| {
        let idx = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        if m.lives[idx].score >= 0.56 {
            return None;
        }
        let lords: Vec<DashaLord> = levels.iter().take(2).map(|p| p.lord).collect();
        let p = parihara_from_facts_in(&natal.facts, Some(life.area()), &lords, lang);
        Some(RemedyBrief {
            planet: p.planet,
            cause: p.plain.clone(),
            top: p
                .remedies
                .iter()
                .filter(|r| !r.optional)
                .take(3)
                .map(|r| (format!("{:?}", r.kind), r.title.clone(), r.why.clone()))
                .collect(),
            total: p.remedies.len(),
        })
    });

    // Paragraph 4: the first lasting lift for the weakest chosen life.
    if let Some(life) = weakest {
        let idx = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        if m.lives[idx].band <= Band::Steady {
            if let Ok(e) = explain_in(lang, spk, eop, natal, obs, system, ayanamsa, life, at) {
                let mut s = String::new();
                if let Some(c) = e.causes.iter().find(|c| matches!(c.kind, "dasha" | "gochara")) {
                    s.push_str(&match lang {
                        Lang::En => format!("What holds {} back: {}", life.name().to_ascii_lowercase(), c.what),
                        Lang::Ml => format!("{} തടസ്സമാകുന്നത്: {}", crate::forecast::life_in(lang, life), c.what),
                        Lang::Hi => format!("{} रुकावट: {}", crate::forecast::life_in(lang, life), c.what),
                    });
                    if !c.until_words.is_empty() {
                        s.push_str(&format!(", {}", c.until_words));
                    }
                    s.push_str(stop);
                    s.push(' ');
                }
                if let Some(r) = e.relief.iter().find(|r| r.lasting && r.gain >= 0.015) {
                    s.push_str(&match lang {
                        Lang::En => format!("It first lifts for good on {}: {}. ", date_words(r.at, lon), r.what),
                        Lang::Ml => format!("{} മുതൽ ഇത് ആദ്യമായി നല്ലവണ്ണം അയയും: {}. ", d(r.at), r.what),
                        Lang::Hi => format!("{} से यह पहली बार टिककर हल्का होगा: {}। ", d(r.at), r.what),
                    });
                }
                if let Some(t) = &e.eases {
                    s.push_str(&t.words);
                    s.push_str(stop);
                }
                if !s.is_empty() {
                    paragraphs.push(s);
                }
            }
        }
    }

    Ok(Briefing {
        at,
        moon,
        moon_sign,
        dasha,
        saturn: m.saturn,
        saturn_words,
        saturn_until,
        paragraphs,
        circled,
        remedy,
        moment: m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_briefing_names_the_moon_the_periods_saturn_and_the_days_to_circle() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let b = briefing(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, &[Life::Love, Life::Career, Life::Money, Life::Health], at).unwrap();
        assert!(b.paragraphs.len() >= 2, "{:?}", b.paragraphs);
        for p in &b.paragraphs {
            assert_eq!(crate::plain::jargon(p), None, "{p}");
        }
        assert!(b.paragraphs[0].starts_with("Until "), "{}", b.paragraphs[0]);
        assert!(b.dasha.contains("mahadasha") && b.dasha.contains("antardasha"));
        assert_eq!(crate::plain::jargon(&b.saturn_words), None, "{}", b.saturn_words);
        for c in &b.circled {
            assert_eq!(crate::plain::jargon(&c.words), None, "{}", c.words);
        }
        assert!(b.saturn_until.is_some());
        assert_eq!(b.circled.len(), 4);
        for c in &b.circled {
            assert!(c.at > at && (0.0..=1.0).contains(&c.score) && !c.words.is_empty());
        }
    }
}
