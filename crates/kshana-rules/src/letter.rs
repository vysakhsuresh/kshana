//! The month's letter: one page for the thirty days ahead, written on the day the
//! Moon returns to the birth star (or whenever it is asked for). The season, each
//! life's story over the month with its best and lowest days, the observances, what
//! the person is waiting for, and the remedy in brief. Every sentence is a number
//! from the forecast with a date beside it.

use crate::briefing::{briefing_in, Briefing};
use crate::feed::{countdowns_in, festivals_in, Countdown, Festival};
use crate::forecast::{date_words, series, story_in, Band, ForecastError, Life, Natal, Story};
use crate::plain::Lang;
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

/// The letter.
#[derive(Clone, Debug, PartialEq)]
pub struct Letter {
    /// First day.
    pub from: Instant,
    /// Last day.
    pub to: Instant,
    /// Title.
    pub title: String,
    /// The paragraphs, in reading order.
    pub paragraphs: Vec<String>,
    /// Each life's story over the month.
    pub stories: Vec<Story>,
    /// The observances.
    pub festivals: Vec<Festival>,
    /// What is being waited for, within the month and just beyond.
    pub countdowns: Vec<Countdown>,
    /// The briefing the letter is built on.
    pub briefing: Briefing,
}

/// Write the letter for the thirty days from `at`.
#[allow(clippy::too_many_arguments)]
pub fn letter(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
    focus: &[Life],
) -> Result<Letter, ForecastError> {
    letter_in(Lang::En, spk, eop, natal, obs, system, ayanamsa, at, focus)
}

/// [`letter`] with every sentence in `lang`.
#[allow(clippy::too_many_arguments)]
pub fn letter_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
    focus: &[Life],
) -> Result<Letter, ForecastError> {
    let lon = obs.longitude_deg;
    let to = at.add_ns(30 * NS_PER_DAY);
    let chosen: Vec<Life> = if focus.is_empty() { Life::ALL.to_vec() } else { focus.to_vec() };
    let b = briefing_in(lang, spk, eop, natal, obs, system, ayanamsa, &chosen, at)?;
    let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, 30)?;
    let stories: Vec<Story> = Life::ALL.iter().map(|&l| story_in(lang, l, &samples)).collect();
    let fest = festivals_in(lang, spk, natal, ayanamsa, at, 30);
    let mut walls = countdowns_in(lang, spk, eop, natal, obs, ayanamsa, at);
    walls.retain(|c| c.at <= at.add_ns(120 * NS_PER_DAY));
    walls.truncate(6);
    if lang != Lang::En {
        let p = paragraphs_indic(lang, lon, focus, &b, &stories, &fest, &walls);
        let d = |t: Instant| crate::lang::date(lang, t, lon);
        let title = match lang {
            Lang::Ml => format!("നിങ്ങളുടെ മാസം, {} മുതൽ {} വരെ", d(at), d(to)),
            _ => format!("आपका महीना, {} से {} तक", d(at), d(to)),
        };
        return Ok(Letter { from: at, to, title, paragraphs: p, stories, festivals: fest, countdowns: walls, briefing: b });
    }
    let in_focus = |l: Life| focus.is_empty() || focus.contains(&l);

    let title = format!("Your month, {} to {}", date_words(at, lon), date_words(to, lon));
    let mut p = Vec::new();
    // 1. The season.
    match b.paragraphs.first() {
        Some(season) => p.push(season.clone()),
        None => p.push(format!("{}.", b.saturn_words)),
    }
    // 2. The shape of the month.
    let mut rising: Vec<&Story> = stories.iter().filter(|s| in_focus(s.life) && s.trend == "rising").collect();
    let mut falling: Vec<&Story> = stories.iter().filter(|s| in_focus(s.life) && s.trend == "falling").collect();
    rising.sort_by(|a, b| b.mean.partial_cmp(&a.mean).unwrap_or(std::cmp::Ordering::Equal));
    falling.sort_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let best = stories.iter().filter(|s| in_focus(s.life)).max_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let weak = stories.iter().filter(|s| in_focus(s.life)).min_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let mut s = String::new();
    if let Some(bst) = best {
        s.push_str(&format!("{} is the month's gift", crate::plain::subject(bst.life.name())));
        if let Some((a, _, v)) = bst.best {
            let _ = v;
            s.push_str(&format!(", at its best around {}", date_words(a, lon)));
        }
        s.push_str(". ");
    }
    if let Some(w) = weak {
        s.push_str(&format!("The most patience is needed for {}", w.life.name().to_ascii_lowercase()));
        if let Some((a, _, v)) = w.low {
            let _ = v;
            s.push_str(&format!(", hardest around {}", date_words(a, lon)));
        }
        s.push_str(". ");
    }
    if !rising.is_empty() {
        s.push_str(&format!(
            "Getting better through the month: {}. ",
            rising.iter().map(|x| x.life.name().to_ascii_lowercase()).collect::<Vec<_>>().join(", ")
        ));
    }
    if !falling.is_empty() {
        s.push_str(&format!(
            "Act early in the month for: {}.",
            falling.iter().map(|x| x.life.name().to_ascii_lowercase()).collect::<Vec<_>>().join(", ")
        ));
    }
    p.push(s.trim().to_string());
    // 3. The days to circle.
    if !b.circled.is_empty() {
        let mut c: Vec<&crate::briefing::Circled> = b.circled.iter().collect();
        c.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        // Group the lives by their best day, so one date is named once.
        let mut days: Vec<(String, Vec<String>)> = Vec::new();
        for x in c.iter().take(5) {
            let d = date_words(x.at, lon);
            let l = x.life.name().to_ascii_lowercase();
            match days.iter_mut().find(|e| e.0 == d) {
                Some(e) => e.1.push(l),
                None => days.push((d, vec![l])),
            }
        }
        let and_list = |v: &[String]| match v.len() {
            0 => String::new(),
            1 => v[0].clone(),
            n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
        };
        let parts: Vec<String> = days.iter().map(|(d, ls)| format!("{d} for {}", and_list(ls))).collect();
        p.push(format!("Days to circle: {}.", parts.join("; ")));
    }
    // 4. The observances.
    if !fest.is_empty() {
        p.push(format!(
            "The month's observances: {}.",
            fest.iter()
                .take(8)
                .map(|f| format!("{} on {}", f.name, date_words(f.at, lon)))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    // 5. What is coming.
    if let Some(c) = walls.first() {
        p.push(format!("Coming up: {}", c.words));
    }
    // 6. The remedy.
    if let Some(r) = &b.remedy {
        p.push(format!(
            "For {}, what weighs is {}. Start with this: {}.",
            weak.map(|w| w.life.name().to_ascii_lowercase()).unwrap_or_default(),
            crate::plain::pressure(r.planet),
            r.top.first().map(|t| t.1.to_ascii_lowercase()).unwrap_or_default()
        ));
    } else if weak.map(|w| Band::of(w.mean) >= Band::Good).unwrap_or(false) {
        p.push("Nothing this month asks for a remedy. Keep the commitments you hold.".into());
    }

    Ok(Letter {
        from: at,
        to,
        title,
        paragraphs: p,
        stories,
        festivals: fest,
        countdowns: walls,
        briefing: b,
    })
}

/// The letter's paragraphs in Malayalam or Hindi, in the same order as the English.
#[allow(clippy::too_many_arguments)]
fn paragraphs_indic(lang: Lang, lon: f64, focus: &[Life], b: &Briefing, stories: &[Story], fest: &[Festival], walls: &[Countdown]) -> Vec<String> {
    let ml = lang == Lang::Ml;
    let stop = if ml { "." } else { "।" };
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    let name = |l: Life| crate::lang::life_name(lang, l);
    let in_focus = |l: Life| focus.is_empty() || focus.contains(&l);
    let by = |a: f64, b: f64| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
    let mut p = Vec::new();
    // 1. The season.
    match b.paragraphs.first() {
        Some(season) => p.push(season.clone()),
        None => p.push(format!("{}{stop}", b.saturn_words)),
    }
    // 2. The shape of the month.
    let mut rising: Vec<&Story> = stories.iter().filter(|s| in_focus(s.life) && s.trend == "rising").collect();
    let mut falling: Vec<&Story> = stories.iter().filter(|s| in_focus(s.life) && s.trend == "falling").collect();
    rising.sort_by(|a, b| by(b.mean, a.mean));
    falling.sort_by(|a, b| by(a.mean, b.mean));
    let best = stories.iter().filter(|s| in_focus(s.life)).max_by(|a, b| by(a.mean, b.mean));
    let weak = stories.iter().filter(|s| in_focus(s.life)).min_by(|a, b| by(a.mean, b.mean));
    let mut s = String::new();
    if let Some(bst) = best {
        s.push_str(&match (ml, bst.best) {
            (true, Some((a, _, _))) => format!("ഈ മാസത്തെ സമ്മാനം {} ആണ്, ഏറ്റവും നന്നാകുന്നത് {}-നോടടുത്ത്. ", name(bst.life), d(a)),
            (true, None) => format!("ഈ മാസത്തെ സമ്മാനം {} ആണ്. ", name(bst.life)),
            (false, Some((a, _, _))) => format!("इस महीने का तोहफ़ा {} है, सबसे अच्छा {} के आसपास। ", name(bst.life), d(a)),
            (false, None) => format!("इस महीने का तोहफ़ा {} है। ", name(bst.life)),
        });
    }
    if let Some(w) = weak {
        let k = crate::forecast::life_in(lang, w.life);
        s.push_str(&match (ml, w.low) {
            (true, Some((a, _, _))) => format!("ഏറ്റവും ക്ഷമ വേണ്ടത് {k} ആണ്, ഏറ്റവും കഠിനം {}-നോടടുത്ത്. ", d(a)),
            (true, None) => format!("ഏറ്റവും ക്ഷമ വേണ്ടത് {k} ആണ്. "),
            (false, Some((a, _, _))) => format!("सबसे ज़्यादा धैर्य {k} चाहिए, सबसे कठिन {} के आसपास। ", d(a)),
            (false, None) => format!("सबसे ज़्यादा धैर्य {k} चाहिए। "),
        });
    }
    let list = |v: &[&Story]| v.iter().map(|x| name(x.life)).collect::<Vec<_>>().join(", ");
    if !rising.is_empty() {
        s.push_str(&match ml {
            true => format!("മാസം മുഴുവൻ മെച്ചപ്പെടുന്നത്: {}. ", list(&rising)),
            false => format!("महीने भर बेहतर होते जाएँगे: {}। ", list(&rising)),
        });
    }
    if !falling.is_empty() {
        s.push_str(&match ml {
            true => format!("മാസത്തിന്റെ തുടക്കത്തിൽ തന്നെ ശ്രദ്ധിക്കേണ്ടത്: {}.", list(&falling)),
            false => format!("महीने की शुरुआत में ही ध्यान दें: {}।", list(&falling)),
        });
    }
    p.push(s.trim().to_string());
    // 3. The days to circle.
    if !b.circled.is_empty() {
        let mut c: Vec<&crate::briefing::Circled> = b.circled.iter().collect();
        c.sort_by(|a, b| by(b.score, a.score));
        let mut days: Vec<(String, Vec<String>)> = Vec::new();
        for x in c.iter().take(5) {
            let dd = d(x.at);
            let l = name(x.life).to_string();
            match days.iter_mut().find(|e| e.0 == dd) {
                Some(e) => e.1.push(l),
                None => days.push((dd, vec![l])),
            }
        }
        let and_list = |v: &[String]| match v.len() {
            0 => String::new(),
            1 => v[0].clone(),
            n if !ml => format!("{} और {}", v[..n - 1].join(", "), v[n - 1]),
            _ => v.join(", "),
        };
        let parts: Vec<String> = days.iter().map(|(dd, ls)| format!("{dd}: {}", and_list(ls))).collect();
        p.push(match ml {
            true => format!("അടയാളപ്പെടുത്തേണ്ട ദിവസങ്ങൾ: {}.", parts.join("; ")),
            false => format!("याद रखने लायक दिन: {}।", parts.join("; ")),
        });
    }
    // 4. The observances.
    if !fest.is_empty() {
        let list = fest.iter().take(8).map(|f| format!("{} ({})", f.name, d(f.at))).collect::<Vec<_>>().join("; ");
        p.push(match ml {
            true => format!("ഈ മാസത്തെ വിശേഷദിവസങ്ങൾ: {list}."),
            false => format!("इस महीने के व्रत-त्योहार: {list}।"),
        });
    }
    // 5. What is coming.
    if let Some(c) = walls.first() {
        p.push(match ml {
            true => format!("വരാനിരിക്കുന്നത്: {}", c.words),
            false => format!("आगे आ रहा है: {}", c.words),
        });
    }
    // 6. The remedy.
    if let Some(r) = &b.remedy {
        let pr = crate::lang::pressure(lang, r.planet);
        p.push(match (ml, weak) {
            (true, Some(w)) => format!("{} ഇപ്പോൾ ഭാരമാകുന്നത് {pr} ആണ്. താഴെയുള്ള പരിഹാരങ്ങളിൽ ആദ്യത്തേതിൽ നിന്ന് തുടങ്ങുക.", crate::forecast::life_in(lang, w.life)),
            (true, None) => format!("ഇപ്പോൾ ഭാരമാകുന്നത് {pr} ആണ്. താഴെയുള്ള പരിഹാരങ്ങളിൽ ആദ്യത്തേതിൽ നിന്ന് തുടങ്ങുക."),
            (false, Some(w)) => format!("{} अभी जो भारी है, वह है {pr}। नीचे दिए उपायों में पहले से शुरुआत करें।", crate::forecast::life_in(lang, w.life)),
            (false, None) => format!("अभी जो भारी है, वह है {pr}। नीचे दिए उपायों में पहले से शुरुआत करें।"),
        });
    } else if weak.map(|w| Band::of(w.mean) >= Band::Good).unwrap_or(false) {
        p.push(lang.pick(["", "ഈ മാസം ഒന്നിനും പരിഹാരം ആവശ്യമില്ല. ഇപ്പോഴുള്ള നല്ല ശീലങ്ങൾ തുടരുക.", "इस महीने किसी उपाय की ज़रूरत नहीं। जो नियम निभा रहे हैं, उन्हें निभाते रहें।"]).to_string());
    }
    p
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
    fn the_letter_has_its_paragraphs() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let l = letter(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at, &[]).unwrap();
        assert!(l.paragraphs.len() >= 4, "{:?}", l.paragraphs);
        assert_eq!(l.stories.len(), 9);
        assert!(!l.festivals.is_empty());
        assert!(l.title.starts_with("Your month"));
        for p in &l.paragraphs {
            assert_eq!(crate::plain::jargon(p), None, "{p}");
        }
    }

    /// A run of three or more Latin letters: English left in a Malayalam or Hindi text.
    fn latin(s: &str) -> Option<String> {
        let mut run = String::new();
        for c in s.chars().chain(std::iter::once(' ')) {
            if c.is_ascii_alphabetic() {
                run.push(c);
            } else {
                if run.len() >= 3 {
                    return Some(run);
                }
                run.clear();
            }
        }
        None
    }

    #[test]
    fn the_daily_words_are_written_in_malayalam_and_hindi() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 22, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        // English stays as it was.
        let en = letter(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at, &[]).unwrap();
        let en2 = letter_in(Lang::En, &spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at, &[]).unwrap();
        assert_eq!(en, en2);
        for lang in [Lang::Ml, Lang::Hi] {
            let l = letter_in(lang, &spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at, &[]).unwrap();
            let b = &l.briefing;
            let m = &b.moment;
            let mut texts: Vec<String> = vec![l.title.clone(), b.saturn_words.clone(), m.headline.clone(), m.title.clone(), m.day_title.clone(), m.tara_words.to_string()];
            texts.extend(l.paragraphs.iter().cloned());
            texts.extend(b.paragraphs.iter().cloned());
            texts.extend(b.circled.iter().map(|c| c.words.clone()));
            texts.extend(l.countdowns.iter().flat_map(|c| [c.title.clone(), c.words.clone()]));
            texts.extend(l.stories.iter().map(|s| s.trend_words.clone()));
            texts.extend(l.stories.iter().flat_map(|s| s.shifts.iter().map(|x| x.what.clone())));
            texts.extend(m.shifts.iter().map(|s| s.what.clone()));
            texts.extend(m.windows.iter().map(|w| w.label.clone()));
            for x in &m.lives {
                texts.push(x.headline.clone());
                texts.push(x.because.clone());
                texts.extend(x.components.iter().map(|c| c.note.clone()));
            }
            if let Some(h) = &m.hora {
                texts.push(h.words.to_string());
                let lore = crate::hora::hora_lore_in(lang, h.lord);
                texts.extend([lore.deity, lore.colour, lore.direction, lore.metal, lore.for_, lore.avoid].iter().map(|s| s.to_string()));
            }
            let tl = crate::timeline::timeline_in(lang, &spk, &natal, Ayanamsa::Lahiri, 76.2673, at);
            texts.push(tl.headline.clone());
            texts.extend(tl.chapters.iter().chain(tl.pages.iter()).map(|c| c.words.clone()));
            texts.extend(tl.saturn.iter().map(|s| s.words.clone()));
            texts.extend(tl.saturn.iter().flat_map(|s| s.phases.iter().map(|p| p.1.clone())));
            texts.extend(tl.jupiter.iter().map(|p| p.words.clone()));
            texts.extend(tl.returns.iter().map(|r| r.words.clone()));
            let y = crate::varsha::year_chapter_in(lang, &spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
            texts.push(y.headline.clone());
            texts.push(y.theme.clone());
            for s in &texts {
                assert_eq!(crate::plain::jargon_in(lang, s), None, "{lang:?}: {s}");
                assert_eq!(latin(s), None, "{lang:?}: {s}");
            }
        }
        for lang in [Lang::Ml, Lang::Hi] {
            for l in kshana_vedic::DashaLord::ORDER {
                let h = crate::hora::hora_lore_in(lang, l);
                for s in [h.deity, h.colour, h.direction, h.metal, h.for_, h.avoid, crate::gochara::hora_words_in(lang, l)] {
                    assert_eq!(crate::plain::jargon_in(lang, s), None, "{lang:?}: {s}");
                    assert_eq!(latin(s), None, "{lang:?}: {s}");
                }
            }
        }
    }
}
