//! Birth-time rectification from life events.
//!
//! The birth window is scanned in steps. For every candidate instant the chart is cast
//! and each known event is scored by how well that chart explains it: the KP promise of
//! the event's area, the number of Vimshottari levels whose lords signify the area at
//! the event's date, and the Parashari quality of the running antardasha lord. The
//! candidate's score is the mean over events. Candidates are returned sorted, each with
//! the Ascendant, its KP sub-lord (the quantity rectification actually fixes) and the
//! per-event reasoning, so the user sees why one time is preferred over another.
//!
//! This is a ranking of hypotheses, not a measurement: with few events many times score
//! alike, and the screen says so.

use crate::areas::Area;
use crate::facts::Facts;
use crate::kp::{agreeing_levels, promise, timing};
use crate::parashari::period_quality;
use kshana_chart::{compute, ChartError, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::{Ayanamsa, DashaLord};

/// A dated life event in one area.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeEvent {
    /// Area of life.
    pub area: Area,
    /// When it happened.
    pub at: Instant,
    /// What it was, in the user's words.
    pub label: String,
}

/// How one candidate explains one event.
#[derive(Clone, Debug, PartialEq)]
pub struct EventScore {
    /// The event's label.
    pub label: String,
    /// Score in `[0, 1]`.
    pub score: f64,
    /// Reasons.
    pub factors: Vec<String>,
}

/// One candidate birth time.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The candidate instant.
    pub birth: Instant,
    /// Mean score over events, `[0, 1]`.
    pub score: f64,
    /// Sidereal Ascendant, degrees.
    pub asc_sidereal: f64,
    /// KP sub-lord of the Ascendant.
    pub asc_sub_lord: DashaLord,
    /// Per-event detail.
    pub per_event: Vec<EventScore>,
}

/// Score a chart against the events.
fn score_chart(facts: &Facts, moon: f64, birth: Instant, events: &[LifeEvent]) -> Vec<EventScore> {
    events
        .iter()
        .map(|e| {
            let mut factors = Vec::new();
            let p = promise(facts, e.area);
            let funnel = timing(facts, e.area, moon, birth, e.at, 3);
            let agree = agreeing_levels(&funnel);
            let mut s = 0.0;
            if p.promised {
                s += 0.3;
                factors.push(format!(
                    "{} promised: {:?} as {}th sub-lord signifies {:?}",
                    e.area.name,
                    p.sub_lord,
                    e.area.primary,
                    p.fulfilling.iter().map(|x| x.house).collect::<Vec<_>>()
                ));
            } else {
                factors.push(format!(
                    "{} not promised by the {}th sub-lord {:?}",
                    e.area.name, e.area.primary, p.sub_lord
                ));
            }
            s += 0.1 * p.strength;
            s += 0.4 * agree as f64 / 3.0;
            if let Some(l) = funnel.first() {
                factors.push(format!(
                    "at the event: {:?} maha, {} of 3 levels signify the area",
                    l.period.lord, agree
                ));
            }
            if let Some(antar) = funnel.get(1) {
                let (q, why) = period_quality(facts, e.area, antar.period.lord);
                s += 0.2 * q;
                factors.push(format!(
                    "antardasha {:?}: {}",
                    antar.period.lord,
                    why.join("; ")
                ));
            }
            EventScore {
                label: e.label.clone(),
                score: s.clamp(0.0, 1.0),
                factors,
            }
        })
        .collect()
}

/// Scan `[center − half_window, center + half_window]` in `step` nanoseconds and rank
/// the candidate birth instants by how well they explain the events.
#[allow(clippy::too_many_arguments)]
pub fn rectify(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    center: Instant,
    half_window_ns: i128,
    step_ns: i128,
    events: &[LifeEvent],
) -> Result<Vec<Candidate>, ChartError> {
    let step = step_ns.max(1_000_000_000);
    let mut out = Vec::new();
    let mut t = center.add_ns(-half_window_ns);
    let end = center.add_ns(half_window_ns);
    while t <= end {
        let input = ChartInput {
            instant: t,
            observer: *obs,
            system: HouseSystem::Placidus,
            ayanamsa,
            topocentric_moon: false,
        };
        let ch = compute(spk, eop, &input)?;
        let facts = Facts::from_chart(&ch);
        let moon = ch.bodies[1].sidereal;
        let per_event = score_chart(&facts, moon, t, events);
        let score = if per_event.is_empty() {
            0.0
        } else {
            per_event.iter().map(|e| e.score).sum::<f64>() / per_event.len() as f64
        };
        out.push(Candidate {
            birth: t,
            score,
            asc_sidereal: ch.cusps_sidereal[0],
            asc_sub_lord: facts.cusp_sub_lords[0],
            per_event,
        });
        t = t.add_ns(step);
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.birth.cmp(&b.birth))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::area;
    use kshana_time::{Utc, NS_PER_SEC};

    #[test]
    fn candidates_cover_the_window_and_are_ranked() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let center = Utc::new(1990, 10, 29, 17, 36, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let events = vec![
            LifeEvent {
                area: area("Career").unwrap(),
                at: Utc::new(2013, 7, 1, 4, 0, 0, 0)
                    .unwrap()
                    .to_instant()
                    .unwrap(),
                label: "first job".into(),
            },
            LifeEvent {
                area: area("Marriage").unwrap(),
                at: Utc::new(2018, 11, 22, 5, 0, 0, 0)
                    .unwrap()
                    .to_instant()
                    .unwrap(),
                label: "wedding".into(),
            },
        ];
        let c = rectify(
            &spk,
            None,
            &obs,
            Ayanamsa::Lahiri,
            center,
            15 * 60 * NS_PER_SEC,
            60 * NS_PER_SEC,
            &events,
        )
        .unwrap();
        assert_eq!(c.len(), 31);
        for w in c.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
        for cand in &c {
            assert_eq!(cand.per_event.len(), 2);
            assert!((0.0..=1.0).contains(&cand.score));
            assert!(cand.per_event.iter().all(|e| !e.factors.is_empty()));
        }
        // the ascendant moves about 0.25° per minute at this latitude
        let first = c.iter().min_by_key(|x| x.birth).unwrap();
        let last = c.iter().max_by_key(|x| x.birth).unwrap();
        let moved = (last.asc_sidereal - first.asc_sidereal).rem_euclid(360.0);
        assert!(moved > 5.0 && moved < 10.0, "{moved}");
    }
}
