//! "I have something at this time. How is that time for me?"
//!
//! A purpose names the lives it touches and how much (an interview is career first,
//! money second); the forecast of the chosen instant gives each life its score; the
//! weighted sum is the reading. The limbs of the day are a gate: inside Rahu kala,
//! Yamaganda or Gulika the reading is capped at "shift it", inside Abhijit it is
//! lifted a little (Muhurta Chintamani). Then the engine looks for a better hour the
//! same day (by the planetary hours, outside the cautions) and a better day in the
//! next fortnight (by the daily series), so the person is never told only "no".

use crate::forecast::{date_words, moment, series, Band, ForecastError, Hora, Life, Moment};
use crate::gochara::{tara_words, SaturnPhase};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::{Ayanamsa, DashaLord};

/// What the appointment is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Purpose {
    Meeting,
    Interview,
    Exam,
    Journey,
    Deal,
    Doctor,
    Date,
    Family,
    Property,
    Launch,
    Anything,
}

impl Purpose {
    /// All, in display order.
    pub const ALL: [Purpose; 11] = [
        Purpose::Meeting,
        Purpose::Interview,
        Purpose::Exam,
        Purpose::Journey,
        Purpose::Deal,
        Purpose::Doctor,
        Purpose::Date,
        Purpose::Family,
        Purpose::Property,
        Purpose::Launch,
        Purpose::Anything,
    ];

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Purpose::Meeting => "meeting",
            Purpose::Interview => "interview",
            Purpose::Exam => "exam",
            Purpose::Journey => "journey",
            Purpose::Deal => "deal",
            Purpose::Doctor => "doctor",
            Purpose::Date => "date",
            Purpose::Family => "family",
            Purpose::Property => "property",
            Purpose::Launch => "launch",
            Purpose::Anything => "anything",
        }
    }

    /// Parse a key.
    pub fn parse(s: &str) -> Option<Purpose> {
        Purpose::ALL.iter().copied().find(|p| p.key() == s)
    }

    /// The words on the chip.
    pub fn words(self) -> &'static str {
        match self {
            Purpose::Meeting => "Meeting someone",
            Purpose::Interview => "Interview, pitch",
            Purpose::Exam => "Exam, test",
            Purpose::Journey => "Journey",
            Purpose::Deal => "Deal, signing, purchase",
            Purpose::Doctor => "Doctor, treatment",
            Purpose::Date => "Date, proposal",
            Purpose::Family => "Family matter",
            Purpose::Property => "House, land, rent",
            Purpose::Launch => "Starting something new",
            Purpose::Anything => "Anything at all",
        }
    }

    /// The lives it touches and their weights (sum 1).
    pub fn lives(self) -> Vec<(Life, f64)> {
        use Life::*;
        match self {
            Purpose::Meeting => vec![(Career, 0.30), (Money, 0.20), (Love, 0.20), (Family, 0.15), (Travel, 0.15)],
            Purpose::Interview => vec![(Career, 0.70), (Money, 0.20), (Study, 0.10)],
            Purpose::Exam => vec![(Study, 0.70), (Career, 0.20), (Health, 0.10)],
            Purpose::Journey => vec![(Travel, 0.60), (Health, 0.20), (Money, 0.10), (Family, 0.10)],
            Purpose::Deal => vec![(Money, 0.50), (Career, 0.30), (Home, 0.20)],
            Purpose::Doctor => vec![(Health, 0.70), (Family, 0.15), (Money, 0.15)],
            Purpose::Date => vec![(Love, 0.70), (Family, 0.15), (Travel, 0.15)],
            Purpose::Family => vec![(Family, 0.60), (Love, 0.20), (Children, 0.20)],
            Purpose::Property => vec![(Home, 0.60), (Money, 0.30), (Family, 0.10)],
            Purpose::Launch => vec![(Career, 0.30), (Money, 0.30), (Study, 0.10), (Travel, 0.10), (Home, 0.10), (Health, 0.10)],
            Purpose::Anything => Life::ALL.iter().map(|&l| (l, 1.0 / 9.0)).collect(),
        }
    }
}

/// A better hour.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Hora lord.
    pub lord: DashaLord,
    /// Fit for the purpose, `[0, 1]`.
    pub score: f64,
}

/// The reading of one scheduled time for one purpose.
#[derive(Clone, Debug, PartialEq)]
pub struct Appointment {
    /// The purpose.
    pub purpose: Purpose,
    /// The time asked about.
    pub at: Instant,
    /// Combined score, `[0, 1]`.
    pub score: f64,
    /// Band.
    pub band: Band,
    /// `Go`, `Go, with care`, `Shift it if you can`, `Avoid this time`.
    pub verdict: String,
    /// One or two sentences.
    pub headline: String,
    /// Plain lines, most useful first.
    pub advice: Vec<String>,
    /// `(life, weight, score)` for the lives the purpose touches, heaviest first.
    pub lives: Vec<(Life, f64, f64)>,
    /// The planetary hour then.
    pub hora: Option<Hora>,
    /// The caution window the time falls in, if any.
    pub caution: Option<String>,
    /// True inside Abhijit.
    pub abhijit: bool,
    /// Tara then.
    pub tara: usize,
    /// Better hours the same day, outside the cautions, best first (up to three).
    pub better_today: Vec<Slot>,
    /// A better day within a fortnight, when one clearly exists: `(midday, score)`.
    pub better_day: Option<(Instant, f64)>,
    /// The better day in words.
    pub better_day_words: String,
    /// The full forecast of the instant.
    pub moment: Moment,
}

fn verdict_words(score: f64, caution: Option<&str>) -> (&'static str, Band) {
    let b = Band::of(score);
    if caution.is_some() {
        return ("Shift it if you can", b);
    }
    (
        if score >= 0.62 {
            "Go"
        } else if score >= 0.50 {
            "Go, with care"
        } else if score >= 0.40 {
            "Shift it if you can"
        } else {
            "Avoid this time"
        },
        b,
    )
}

/// Read `at` for `purpose`.
#[allow(clippy::too_many_arguments)]
pub fn appointment(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    at: Instant,
) -> Result<Appointment, ForecastError> {
    let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let weights = purpose.lives();
    let idx = |l: Life| Life::ALL.iter().position(|x| *x == l).unwrap_or(0);
    let mut lives: Vec<(Life, f64, f64)> = weights
        .iter()
        .map(|&(l, w)| (l, w, m.lives[idx(l)].score))
        .collect();
    lives.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let raw: f64 = lives.iter().map(|(_, w, s)| w * s).sum::<f64>() / lives.iter().map(|(_, w, _)| w).sum::<f64>();

    let caution = m
        .windows
        .iter()
        .find(|w| w.caution && at >= w.start && at < w.end)
        .map(|w| w.label.clone());
    let abhijit = m
        .windows
        .iter()
        .any(|w| !w.caution && at >= w.start && at < w.end);
    let mut score = raw;
    if caution.is_some() {
        score = score.min(0.40);
    }
    if abhijit {
        score = (score + 0.05).min(1.0);
    }
    let (verdict, band) = verdict_words(score, caution.as_deref());

    // Better hours the same day, by the planetary hour's fit for the purpose.
    let fit_of = |h: &crate::forecast::HourSlot| -> f64 {
        lives.iter().map(|(l, w, _)| w * h.fit[idx(*l)]).sum::<f64>()
            / lives.iter().map(|(_, w, _)| w).sum::<f64>()
    };
    let current_fit = m
        .hours
        .iter()
        .find(|h| at >= h.start && at < h.end)
        .map(fit_of)
        .unwrap_or(0.5);
    let mut better_today: Vec<Slot> = m
        .hours
        .iter()
        .filter(|h| !h.caution && !(at >= h.start && at < h.end))
        .map(|h| Slot {
            start: h.start,
            end: h.end,
            lord: h.lord,
            score: fit_of(h),
        })
        .filter(|s| s.score > current_fit + 0.08 || caution.is_some())
        .collect();
    better_today.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    better_today.truncate(3);
    better_today.sort_by(|a, b| a.start.cmp(&b.start));

    // A better day within a fortnight.
    let samples = series(spk, eop, natal, obs, ayanamsa, at.add_ns(NS_PER_DAY), NS_PER_DAY, 14)?;
    let day_score = |s: &crate::forecast::Sample| -> f64 {
        lives.iter().map(|(l, w, _)| w * s.scores[idx(*l)]).sum::<f64>()
            / lives.iter().map(|(_, w, _)| w).sum::<f64>()
    };
    let best_day = samples
        .iter()
        .map(|s| (s.at, day_score(s)))
        .fold(None, |acc: Option<(Instant, f64)>, (t, v)| match acc {
            Some((_, bv)) if bv >= v => acc,
            _ => Some((t, v)),
        });
    let better_day = best_day.filter(|(_, v)| *v >= raw + 0.08 || (caution.is_some() && *v >= raw));
    let better_day_words = match better_day {
        Some((t, v)) => format!(
            "If it can move: {} reads {:.0} for this, against {:.0} at the time you have.",
            date_words(t, obs.longitude_deg),
            v * 100.0,
            raw * 100.0
        ),
        None => String::new(),
    };

    // Words.
    let top = lives.first().map(|(l, _, _)| *l).unwrap_or(Life::Career);
    let top_ls = &m.lives[idx(top)];
    let weak = lives
        .iter()
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(l, _, s)| (*l, *s));
    let pw = purpose.words();
    let headline = match (caution.as_deref(), verdict) {
        (Some(c), _) => format!(
            "{pw} falls inside {c}. Finish, do not begin: start it once {c} ends if you can, or take one of the hours below."
        ),
        (None, "Go") => format!(
            "Go. {} is {} at that hour and the sky carries {}. {}",
            top.name(),
            top_ls.band.word(),
            pw.to_ascii_lowercase(),
            top_ls.headline
        ),
        (None, "Go, with care") => format!(
            "Go, with care. {} is {} then; {} will work, but prepare more than usual.{}",
            top.name(),
            top_ls.band.word(),
            pw.to_ascii_lowercase(),
            weak.filter(|(_, s)| *s < 0.44)
                .map(|(l, _)| format!(" Watch {}.", l.name().to_ascii_lowercase()))
                .unwrap_or_default()
        ),
        (None, "Shift it if you can") => format!(
            "Shift it if you can. {} is {} at that hour, and that is what {} leans on. The hours and days below serve it better.",
            top.name(),
            top_ls.band.word(),
            pw.to_ascii_lowercase()
        ),
        _ => format!(
            "Avoid this time. {} is on hold then, and {} would carry that weight. Move it to one of the times below.",
            top.name(),
            pw.to_ascii_lowercase()
        ),
    };

    let mut advice: Vec<String> = Vec::new();
    if let Some(h) = &m.hora {
        let fit = top.hora_affinity(h.lord);
        advice.push(format!(
            "{:?} hour: {}{}",
            h.lord,
            h.words,
            if fit >= 0.7 {
                format!(" · suits {}", top.name().to_ascii_lowercase())
            } else if fit <= 0.3 {
                format!(" · not an hour for {}", top.name().to_ascii_lowercase())
            } else {
                String::new()
            }
        ));
    }
    if abhijit {
        advice.push("Abhijit muhurta: the day's winning minutes. Begin exactly then.".into());
    }
    for w in &m.windows {
        if w.caution && w.start > at && w.start.ns_since(at) < 3 * 3_600 * kshana_time::NS_PER_SEC {
            advice.push(format!("{} begins soon after; finish before it, do not let it spill in.", w.label));
        }
    }
    advice.push(format!("{} · Moon in {}", tara_words(m.tara).to_string().replace("your birth star returns", "Your birth star returns"), m.moon_star));
    if m.moon_from_moon == 8 {
        advice.push("Chandrashtama: the Moon is in the 8th from your Moon that day. Keep it short and simple.".into());
    }
    if m.saturn != SaturnPhase::Clear {
        advice.push(format!("{}. Expect it to take longer than it should.", m.saturn.words()).replace("sade sati", "Sade sati").replace("ardhashtama", "Ardhashtama").replace("ashtama Shani", "Ashtama Shani"));
    }
    for (l, _, s) in &lives {
        let ls = &m.lives[idx(*l)];
        if *s < 0.44 && *l != top {
            advice.push(format!("{}: {}", l.name(), ls.headline));
        }
    }
    if let Some((_, s)) = weak {
        if s >= 0.56 && lives.len() > 1 {
            advice.push("Every life this touches is good or better at that hour.".into());
        }
    }

    Ok(Appointment {
        purpose,
        at,
        score,
        band,
        verdict: verdict.to_string(),
        headline,
        advice,
        lives,
        hora: m.hora.clone(),
        caution,
        abhijit,
        tara: m.tara,
        better_today,
        better_day,
        better_day_words,
        moment: m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::Natal;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn every_purpose_is_read_and_never_left_without_an_alternative_when_poor() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 20, 2, 0, 0, 0).unwrap().to_instant().unwrap();
        for p in Purpose::ALL {
            let a = appointment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, p, at).unwrap();
            assert!((0.0..=1.0).contains(&a.score));
            assert!(!a.verdict.is_empty() && !a.headline.is_empty());
            assert!(!a.advice.is_empty());
            let w: f64 = a.lives.iter().map(|x| x.1).sum();
            assert!((w - 1.0).abs() < 1e-9);
            if a.score < 0.50 {
                assert!(!a.better_today.is_empty() || a.better_day.is_some(), "{:?}", a.verdict);
            }
        }
        // Rahu kala on a Tuesday in Kochi runs about 15:00–16:30 local; a time inside it is capped.
        let rk = Utc::new(2026, 10, 20, 10, 0, 0, 0).unwrap().to_instant().unwrap();
        let a = appointment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Purpose::Deal, rk).unwrap();
        assert!(a.caution.is_some(), "{:?}", a.moment.windows);
        assert!(a.score <= 0.45);
        assert_eq!(a.verdict, "Shift it if you can");
    }
}
