//! Birth-time rectification from dated events. The person names a few things that
//! happened and when (a marriage, a move, a loss, a job); every candidate birth time
//! inside the stated window is cast and scored by how well its periods and slow sky
//! explain those days: a good event should fall on a high slow score for its life with
//! the Vimshottari funnel carrying the life's houses, a hard one on a low score with
//! the funnel silent. The best-fitting times are returned with what changes between
//! them (rising sign, Moon's star, the dasha at birth), so the choice is informed, not
//! blind.

use crate::forecast::{sky, slow_scores, Life, Natal};
use crate::kp::{agreeing_levels, timing};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, vimshottari_levels, Ayanamsa, DashaLord};

/// Something that happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifeEvent {
    /// When.
    pub at: Instant,
    /// Which life it belongs to.
    pub life: Life,
    /// True for a good event, false for a hard one.
    pub good: bool,
}

/// One candidate birth time.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The birth instant.
    pub birth: Instant,
    /// Fit in `[0, 1]`.
    pub fit: f64,
    /// Rising sign, 0 = Aries.
    pub lagna_sign: u8,
    /// The Moon's star, 0 = Ashwini.
    pub moon_star: usize,
    /// The mahadasha running at birth.
    pub dasha_at_birth: DashaLord,
    /// How many events it explains (fit ≥ 0.5 each).
    pub explained: usize,
    /// In words.
    pub words: String,
}

/// Score `window_start..=window_end` at up to `max_candidates` evenly spaced times.
#[allow(clippy::too_many_arguments)]
pub fn rectify(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    window_start: Instant,
    window_end: Instant,
    events: &[LifeEvent],
    max_candidates: usize,
) -> Vec<Candidate> {
    let span = window_end.ns_since(window_start).max(0);
    let n = if span == 0 { 1 } else { max_candidates.clamp(3, 241) };
    let step = if n > 1 { span / (n as i128 - 1) } else { 0 };
    // The sky of each event is the same for every candidate: cast it once.
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let birth = window_start.add_ns(step * k as i128);
        let Ok(natal) = Natal::compute(spk, eop, birth, obs, system, ayanamsa) else {
            continue;
        };
        let mut total = 0.0;
        let mut explained = 0;
        for e in events {
            let Ok(s) = sky(spk, eop, &natal, obs, HouseSystem::WholeSign, ayanamsa, e.at) else {
                continue;
            };
            let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, e.at, 3);
            let idx = Life::ALL.iter().position(|l| *l == e.life).unwrap_or(0);
            let slow = slow_scores(&natal, &s, &levels)[idx];
            let funnel = timing(&natal.facts, e.life.area(), natal.moon_sidereal, natal.birth, e.at, 3);
            let agree = agreeing_levels(&funnel) as f64 / 3.0;
            let f = if e.good {
                0.6 * slow + 0.4 * agree
            } else {
                0.6 * (1.0 - slow) + 0.4 * (1.0 - agree)
            };
            if f >= 0.5 {
                explained += 1;
            }
            total += f;
        }
        let fit = if events.is_empty() { 0.5 } else { total / events.len() as f64 };
        let lagna_sign = natal.facts.lagna_sign;
        let moon_star = natal.janma();
        let dasha_at_birth = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, natal.birth.add_ns(NS_PER_SEC), 1)
            .first()
            .map(|p| p.lord)
            .unwrap_or(DashaLord::Moon);
        out.push(Candidate {
            birth,
            fit,
            lagna_sign,
            moon_star,
            dasha_at_birth,
            explained,
            words: format!(
                "This birth time explains {} of the {} things you marked, a {:.0}% fit",
                explained,
                events.len(),
                fit * 100.0
            ),
        });
    }
    out.sort_by(|a, b| b.fit.partial_cmp(&a.fit).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// The distinct rising signs among the candidates, best fit first, with the best
/// candidate of each: what the window actually lets vary.
pub fn by_rising_sign(candidates: &[Candidate]) -> Vec<Candidate> {
    let mut seen: Vec<u8> = Vec::new();
    let mut v = Vec::new();
    for c in candidates {
        if !seen.contains(&c.lagna_sign) {
            seen.push(c.lagna_sign);
            v.push(c.clone());
        }
    }
    v
}

#[allow(dead_code)]
fn star_name(i: usize) -> &'static str {
    NAKSHATRA_NAMES[nakshatra(i as f64 * 360.0 / 27.0 + 1.0).index]
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
    fn candidates_are_ranked_and_distinct_rising_signs_are_found() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let centre = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let events = [
            LifeEvent { at: Utc::new(2018, 11, 12, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Love, good: true },
            LifeEvent { at: Utc::new(2025, 7, 17, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Love, good: false },
            LifeEvent { at: Utc::new(2021, 3, 1, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Career, good: true },
        ];
        let v = rectify(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, centre.add_ns(-3 * 3600 * NS_PER_SEC), centre.add_ns(3 * 3600 * NS_PER_SEC), &events, 37);
        assert_eq!(v.len(), 37);
        assert!(v.windows(2).all(|w| w[0].fit >= w[1].fit));
        assert!(v.iter().all(|c| (0.0..=1.0).contains(&c.fit)));
        let signs = by_rising_sign(&v);
        assert!(signs.len() >= 3, "six hours of window cross at least three rising signs: {}", signs.len());
    }
}
