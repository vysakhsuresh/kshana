//! Solar arc directions: every natal point advanced by the progressed Sun's arc.

use crate::progressions::progressed_instant;
use crate::{aspect_targets, natal_points, require, DirectionsError, NatalPoint};
use kshana_chart::Chart;
use kshana_ephem::{Body, Spk};
use kshana_events::MICROSECOND_NS;
use kshana_sky::apparent;
use kshana_solver::{find_root, wrap180};
use kshana_time::{Instant, NS_PER_DAY};
use kshana_western::Aspect;

/// Scan step for the arc in real time: 60 days, over which the arc grows about 0.16°.
/// It is well under the solver's 2⁵³ ns bracket limit.
pub const ARC_SCAN_STEP_NS: i128 = 60 * NS_PER_DAY;

/// The solar arc at real instant `at`: progressed Sun − natal Sun, `[0, 360)`.
///
/// Convention: the arc of the secondary progressed Sun (the real Sun at
/// [`progressed_instant`]), about but not exactly 1° a year, as against the mean
/// Naibod arc of 0°59′08″; see
/// <https://kerykeion.net/content/learn-astrology/solar-arc-introduction>.
pub fn solar_arc(spk: &Spk, natal: &Chart, at: Instant) -> Result<f64, DirectionsError> {
    let natal_sun = require(natal, "Sun")?;
    Ok(arc_from_natal_sun(spk, natal.instant, natal_sun, at)?)
}

fn arc_from_natal_sun(
    spk: &Spk,
    birth: Instant,
    natal_sun: f64,
    at: Instant,
) -> Result<f64, kshana_ephem::SpkError> {
    let sun = apparent(spk, Body::Sun, progressed_instant(birth, at))?.longitude;
    Ok((sun - natal_sun).rem_euclid(360.0))
}

/// Every natal point (bodies, Ascendant, MC) advanced by the solar arc at `at`.
pub fn directed_positions(
    spk: &Spk,
    natal: &Chart,
    at: Instant,
) -> Result<Vec<NatalPoint>, DirectionsError> {
    let arc = solar_arc(spk, natal, at)?;
    Ok(natal_points(natal)
        .into_iter()
        .map(|p| NatalPoint {
            name: p.name,
            longitude: (p.longitude + arc).rem_euclid(360.0),
        })
        .collect())
}

/// A directed point reaching an exact aspect to a natal point.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectionHit {
    /// The real instant, exact to a microsecond.
    pub at: Instant,
    /// Name of the directed point.
    pub directed: &'static str,
    /// Name of the natal point it reaches.
    pub natal: &'static str,
    /// The aspect formed.
    pub aspect: Aspect,
    /// The solar arc at that instant, degrees.
    pub arc: f64,
    /// Always true: the instant is solved to exactness, not found within an orb.
    pub orb_exact: bool,
}

/// Exact instants in `[from, to]` at which a directed point (any body, the Ascendant
/// or the MC) forms a Ptolemaic aspect (conjunction, opposition, trine, square,
/// sextile) to a natal point. See [`solar_arc_hits_with`].
pub fn solar_arc_hits(
    spk: &Spk,
    natal: &Chart,
    from: Instant,
    to: Instant,
) -> Result<Vec<DirectionHit>, DirectionsError> {
    let major: Vec<Aspect> = Aspect::ALL
        .iter()
        .copied()
        .filter(|a| a.is_major())
        .collect();
    solar_arc_hits_with(spk, natal, &major, from, to)
}

/// Exact instants in `[from, to]` at which a directed point forms one of `aspects` to a
/// natal point, sorted by time.
///
/// The arc grows monotonically from 0 at birth, so for each pair and aspect the needed
/// arc `(natal ± angle − directed) mod 360` is reached at most once in the first 360
/// years; that instant is the root of `arc(t) − needed` in real time, bracketed on a
/// 60-day grid and solved to a microsecond. `from` is clamped to the birth.
pub fn solar_arc_hits_with(
    spk: &Spk,
    natal: &Chart,
    aspects: &[Aspect],
    from: Instant,
    to: Instant,
) -> Result<Vec<DirectionHit>, DirectionsError> {
    let birth = natal.instant;
    let natal_sun = require(natal, "Sun")?;
    let from = from.max(birth);
    let mut out = Vec::new();
    if to <= from {
        return Ok(out);
    }
    let arc_at = |t: Instant| arc_from_natal_sun(spk, birth, natal_sun, t);

    let mut samples: Vec<(Instant, f64)> = Vec::new();
    let mut t = from;
    loop {
        samples.push((t, arc_at(t)?));
        if t >= to {
            break;
        }
        t = crate::next_step(t, to, ARC_SCAN_STEP_NS);
    }
    let arc_from = samples[0].1;
    let arc_to = samples[samples.len() - 1].1;

    let points = natal_points(natal);
    for d in &points {
        for n in &points {
            for &asp in aspects {
                for target in aspect_targets(n.longitude, asp) {
                    let needed = (target - d.longitude).rem_euclid(360.0);
                    if needed <= arc_from || needed > arc_to {
                        continue;
                    }
                    let Some(i) = samples
                        .windows(2)
                        .position(|w| w[0].1 < needed && needed <= w[1].1)
                    else {
                        continue;
                    };
                    let g = |t: Instant| arc_at(t).map(|a| wrap180(a - needed)).unwrap_or(f64::NAN);
                    let root = find_root(g, samples[i].0, samples[i + 1].0, MICROSECOND_NS)?;
                    out.push(DirectionHit {
                        at: root.t,
                        directed: d.name,
                        natal: n.name,
                        aspect: asp,
                        arc: needed,
                        orb_exact: true,
                    });
                }
            }
        }
    }
    out.sort_by_key(|h| h.at);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use kshana_western::separation;

    #[test]
    fn arc_at_thirty_six_is_about_thirty_six_degrees() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let at = fixture::utc(2026, 10, 29, 12, 6);
        let arc = solar_arc(&spk, &natal, at).unwrap();
        assert!((arc - 36.0).abs() < 2.0, "{arc}");
        let directed = directed_positions(&spk, &natal, at).unwrap();
        assert_eq!(directed.len(), 12);
        for (d, n) in directed.iter().zip(natal_points(&natal)) {
            assert_eq!(d.name, n.name);
            assert!((wrap180(d.longitude - n.longitude) - wrap180(arc)).abs() < 1e-9);
        }
    }

    #[test]
    fn directed_hits_are_exact_and_sorted() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let from = fixture::utc(2024, 1, 1, 0, 0);
        let to = fixture::utc(2027, 1, 1, 0, 0);
        let hits = solar_arc_hits(&spk, &natal, from, to).unwrap();
        assert!(!hits.is_empty());
        assert!(hits.windows(2).all(|w| w[0].at <= w[1].at));
        for h in &hits {
            assert!(h.orb_exact);
            assert!(from <= h.at && h.at <= to);
            let arc = solar_arc(&spk, &natal, h.at).unwrap();
            assert!((arc - h.arc).abs() < 1e-6, "{arc} vs {}", h.arc);
            let d = crate::natal_longitude(&natal, h.directed).unwrap() + arc;
            let n = crate::natal_longitude(&natal, h.natal).unwrap();
            assert!((separation(d, n) - h.aspect.angle()).abs() < 1e-6);
        }
        // nothing before birth, and a window ending at birth is empty
        assert!(solar_arc_hits(&spk, &natal, natal.instant, natal.instant)
            .unwrap()
            .is_empty());
    }
}
