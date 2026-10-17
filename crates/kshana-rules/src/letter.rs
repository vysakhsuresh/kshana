//! The month's letter: one page for the thirty days ahead, written on the day the
//! Moon returns to the birth star (or whenever it is asked for). The season, each
//! life's story over the month with its best and lowest days, the observances, what
//! the person is waiting for, and the remedy in brief. Every sentence is a number
//! from the forecast with a date beside it.

use crate::briefing::{briefing, Briefing};
use crate::feed::{countdowns, festivals, Countdown, Festival};
use crate::forecast::{date_words, series, story, Band, ForecastError, Life, Natal, Story};
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
) -> Result<Letter, ForecastError> {
    let lon = obs.longitude_deg;
    let to = at.add_ns(30 * NS_PER_DAY);
    let b = briefing(spk, eop, natal, obs, system, ayanamsa, &Life::ALL, at)?;
    let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, 30)?;
    let stories: Vec<Story> = Life::ALL.iter().map(|&l| story(l, &samples)).collect();
    let fest = festivals(spk, natal, ayanamsa, at, 30);
    let mut walls = countdowns(spk, eop, natal, obs, ayanamsa, at);
    walls.retain(|c| c.at <= at.add_ns(120 * NS_PER_DAY));
    walls.truncate(6);

    let title = format!("Your month, {} to {}", date_words(at, lon), date_words(to, lon));
    let mut p = Vec::new();
    // 1. The season.
    p.push(format!(
        "You are in {}. {}.",
        b.dasha.replace(" (to ", ", to ").replace(")", ""),
        b.saturn_words
    ));
    // 2. The shape of the month.
    let mut rising: Vec<&Story> = stories.iter().filter(|s| s.trend == "rising").collect();
    let mut falling: Vec<&Story> = stories.iter().filter(|s| s.trend == "falling").collect();
    rising.sort_by(|a, b| b.mean.partial_cmp(&a.mean).unwrap_or(std::cmp::Ordering::Equal));
    falling.sort_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let best = stories.iter().max_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let weak = stories.iter().min_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
    let mut s = String::new();
    if let Some(bst) = best {
        s.push_str(&format!("{} is the month's gift, averaging {:.0}", bst.life.name(), bst.mean * 100.0));
        if let Some((a, _, v)) = bst.best {
            s.push_str(&format!(", at its best around {} ({:.0})", date_words(a, lon), v * 100.0));
        }
        s.push_str(". ");
    }
    if let Some(w) = weak {
        s.push_str(&format!("{} asks the most patience at {:.0}", w.life.name(), w.mean * 100.0));
        if let Some((a, _, v)) = w.low {
            s.push_str(&format!(", lowest around {} ({:.0})", date_words(a, lon), v * 100.0));
        }
        s.push_str(". ");
    }
    if !rising.is_empty() {
        s.push_str(&format!(
            "Rising through the month: {}. ",
            rising.iter().map(|x| x.life.name().to_ascii_lowercase()).collect::<Vec<_>>().join(", ")
        ));
    }
    if !falling.is_empty() {
        s.push_str(&format!(
            "Act early in: {}.",
            falling.iter().map(|x| x.life.name().to_ascii_lowercase()).collect::<Vec<_>>().join(", ")
        ));
    }
    p.push(s.trim().to_string());
    // 3. The days to circle.
    if !b.circled.is_empty() {
        let mut c: Vec<&crate::briefing::Circled> = b.circled.iter().collect();
        c.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        p.push(format!(
            "Days to circle: {}.",
            c.iter()
                .take(5)
                .map(|x| format!("{} for {} ({:.0})", date_words(x.at, lon), x.life.name().to_ascii_lowercase(), x.score * 100.0))
                .collect::<Vec<_>>()
                .join("; ")
        ));
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
        p.push(format!("Nearest on the wall: {} on {}. {}", c.title, date_words(c.at, lon), c.words));
    }
    // 6. The remedy.
    if let Some(r) = &b.remedy {
        p.push(format!(
            "For {}: {}. First, {}.",
            weak.map(|w| w.life.name().to_ascii_lowercase()).unwrap_or_default(),
            r.cause,
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
        let l = letter(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
        assert!(l.paragraphs.len() >= 4, "{:?}", l.paragraphs);
        assert_eq!(l.stories.len(), 9);
        assert!(!l.festivals.is_empty());
        assert!(l.title.starts_with("Your month"));
    }
}
