//! The briefing: what an astrologer would tell a person across the table, in plain
//! sentences, every one of them traceable. Where the Moon stands at birth (so the
//! reading can be checked against any other), which periods run and until when,
//! where Saturn is relative to the Moon and the day it moves on, how the moment
//! reads and why, and the days to circle in the month ahead. Used on the Loop for one
//! person and under Us for each of the two.

use crate::explain::explain;
use crate::forecast::{date_words, moment, series, Band, ForecastError, Life, Moment, Natal, SIGN_NAMES};
use crate::gochara::SaturnPhase;
use crate::parihara::parihara;
use crate::transit::next_sign_change;
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

fn lord_words(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "standing, the father, the self",
        DashaLord::Moon => "home, feeling, the mind",
        DashaLord::Mars => "effort, property, courage",
        DashaLord::Mercury => "learning, trade, talk",
        DashaLord::Jupiter => "growth, children, faith, money",
        DashaLord::Venus => "love, comfort, partnership",
        DashaLord::Saturn => "work, duty, slow lasting gains",
        DashaLord::Rahu => "ambition, foreign things, sudden turns",
        DashaLord::Ketu => "letting go, insight, the inner road",
    }
}

/// Saturn's phase named with its exit date, or its clearance.
pub fn saturn_sentence(spk: &Spk, ayanamsa: Ayanamsa, natal: &Natal, phase: SaturnPhase, at: Instant, lon: f64) -> (String, Option<Instant>) {
    let next = next_sign_change(spk, ayanamsa, DashaLord::Saturn, at);
    match phase {
        SaturnPhase::Clear => {
            let words = match next {
                Some(c) => {
                    let h = ((c.to as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                    let coming = SaturnPhase::from_house(h);
                    if coming != SaturnPhase::Clear {
                        format!(
                            "Saturn is clear of your Moon until {}, when it enters {} and {} begins",
                            date_words(c.at, lon),
                            SIGN_NAMES[c.to as usize],
                            coming.words().split(':').next().unwrap_or("a Saturn phase")
                        )
                    } else {
                        format!("Saturn is clear of your Moon, and stays clear when it enters {} on {}", SIGN_NAMES[c.to as usize], date_words(c.at, lon))
                    }
                }
                None => "Saturn is clear of your Moon".to_string(),
            };
            (words, next.map(|c| c.at))
        }
        p => {
            let words = match next {
                Some(c) => {
                    let h = ((c.to as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                    let after = SaturnPhase::from_house(h);
                    if after == SaturnPhase::Clear {
                        format!("{}. It ends on {}, when Saturn enters {}", capitalise(p.words()), date_words(c.at, lon), SIGN_NAMES[c.to as usize])
                    } else {
                        format!(
                            "{}. On {} Saturn enters {} and the {} phase follows",
                            capitalise(p.words()),
                            date_words(c.at, lon),
                            SIGN_NAMES[c.to as usize],
                            after.words().split(':').next().unwrap_or("next")
                        )
                    }
                }
                None => capitalise(p.words()),
            };
            (words, next.map(|c| c.at))
        }
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
    let lon = obs.longitude_deg;
    let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let n = nakshatra(natal.moon_sidereal);
    let moon_sign = (natal.moon_sign().rem_euclid(12)) as u8;
    let moon = format!("{} · {}", SIGN_NAMES[moon_sign as usize], NAKSHATRA_NAMES[n.index]);
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    let dasha = levels
        .iter()
        .enumerate()
        .take(2)
        .map(|(i, p)| format!("{:?} {} (to {})", p.lord, ["mahadasha", "antardasha"][i], date_words(p.end, lon)))
        .collect::<Vec<_>>()
        .join(" › ");
    let (saturn_words, saturn_until) = saturn_sentence(spk, ayanamsa, natal, m.saturn, at, lon);

    // Paragraph 1: the ground. Stated so it can be checked against any other reading.
    let mut paragraphs = Vec::new();
    paragraphs.push(format!(
        "Your birth Moon is in {} in the star {}. Every count below starts there: the periods from the star, the transits from the sign. If another reading names a different Moon sign, one of the two birth times is off by hours.",
        SIGN_NAMES[moon_sign as usize],
        NAKSHATRA_NAMES[n.index]
    ));
    // Paragraph 2: the season.
    if let (Some(maha), Some(antar)) = (levels.first(), levels.get(1)) {
        paragraphs.push(format!(
            "You are in the {:?} mahadasha, the years of {}, until {}. Inside it the {:?} antardasha runs until {}: {}. {}.",
            maha.lord,
            lord_words(maha.lord),
            date_words(maha.end, lon),
            antar.lord,
            date_words(antar.end, lon),
            lord_words(antar.lord),
            saturn_words
        ));
    } else {
        paragraphs.push(format!("{saturn_words}."));
    }
    // Paragraph 3: the moment.
    let top = m.lives.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    let low = m.lives.iter().min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    if let (Some(t), Some(l)) = (top, low) {
        let hora = m
            .hora
            .as_ref()
            .map(|h| format!(" The {:?} hour runs until the clock turns; {}.", h.lord, h.words))
            .unwrap_or_default();
        paragraphs.push(format!(
            "This second reads {:.0}, {}. {} is your strongest life now at {:.0} ({}), {} the weakest at {:.0} ({}).{} The Moon is in {} today, your {} tara: {}.",
            m.overall * 100.0,
            m.band.word(),
            t.life.name(),
            t.score * 100.0,
            t.because,
            l.life.name(),
            l.score * 100.0,
            l.because,
            hora,
            m.moon_star,
            crate::gochara::TARA_NAMES[m.tara],
            m.tara_words
        ));
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
                    "{}: {} ({:.0}, {} tara). {}",
                    life.name(),
                    date_words(best.at, lon),
                    s * 100.0,
                    crate::gochara::TARA_NAMES[best.tara],
                    crate::forecast::headline(life, Band::of(s), (best.at.tai_ns() / NS_PER_DAY) as u64)
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
        let p = parihara(&natal.facts, Some(life.area()), &lords);
        Some(RemedyBrief {
            planet: p.planet,
            cause: p.cause.clone(),
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
            if let Ok(e) = explain(spk, eop, natal, obs, system, ayanamsa, life, at) {
                let mut s = String::new();
                if let Some(c) = e.causes.iter().find(|c| matches!(c.kind, "dasha" | "gochara")) {
                    s.push_str(&format!("{} is held back by {}", life.name(), c.what));
                    if !c.until_words.is_empty() {
                        s.push_str(&format!(", {}", c.until_words));
                    }
                    s.push_str(". ");
                }
                if let Some(r) = e.relief.iter().find(|r| r.lasting && r.gain >= 0.015) {
                    s.push_str(&format!("The first lasting lift comes on {}: {} ({:+.0} points). ", date_words(r.at, lon), r.what, r.gain * 100.0));
                }
                if let Some(t) = &e.eases {
                    s.push_str(&t.words);
                    s.push('.');
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
        assert!(b.paragraphs.len() >= 3, "{:?}", b.paragraphs);
        assert!(b.paragraphs[0].contains("Sagittarius"), "{}", b.paragraphs[0]);
        assert!(b.dasha.contains("mahadasha") && b.dasha.contains("antardasha"));
        assert!(b.saturn_words.contains("Saturn"));
        assert!(b.saturn_until.is_some());
        assert_eq!(b.circled.len(), 4);
        for c in &b.circled {
            assert!(c.at > at && (0.0..=1.0).contains(&c.score) && !c.words.is_empty());
        }
    }
}
