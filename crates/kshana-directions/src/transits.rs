//! Exact transit-to-natal contacts.

use crate::{aspect_targets, body_name, natal_points, scan_roots, DirectionsError, LongitudeCache};
use kshana_chart::Chart;
use kshana_ephem::{Body, Spk};
use kshana_events::MICROSECOND_NS;
use kshana_sky::apparent;
use kshana_solver::wrap180;
use kshana_time::{Instant, NS_PER_SEC};
use kshana_western::Aspect;

/// Scan step: six hours, under half the time the Moon takes to move 180° and the same
/// grid `kshana_events::longitude_crossings` uses.
pub const TRANSIT_SCAN_STEP_NS: i128 = 6 * 3_600 * NS_PER_SEC;

/// A transiting body at an exact aspect to a natal point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitHit {
    /// The instant, exact to a microsecond.
    pub at: Instant,
    /// The transiting body.
    pub body: Body,
    /// Its chart name ("Saturn", ...).
    pub body_name: &'static str,
    /// The natal point hit: a body name, "Ascendant" or "MC".
    pub natal_point: &'static str,
    /// The aspect formed.
    pub aspect: Aspect,
    /// True when the transiting body is retrograde at that instant.
    pub retrograde: bool,
    /// Half-width of the solver's final bracket, nanoseconds.
    pub half_width_ns: i128,
}

/// Exact instants in `[from, to]` at which each of `bodies` forms each of `aspects` to a
/// natal point (every chart body, the Ascendant and the MC), sorted by time.
///
/// For each natal point and aspect the targets are `natal + angle` and, for aspects
/// other than the conjunction and opposition, `natal − angle`; each target is a
/// longitude crossing solved to a microsecond, as `kshana_events::longitude_crossings`
/// does, with the six-hour samples of the body shared across all targets.
pub fn transit_hits(
    spk: &Spk,
    natal: &Chart,
    bodies: &[Body],
    aspects: &[Aspect],
    from: Instant,
    to: Instant,
) -> Result<Vec<TransitHit>, DirectionsError> {
    let points = natal_points(natal);
    let mut out = Vec::new();
    if to <= from {
        return Ok(out);
    }
    for &body in bodies {
        let cache = LongitudeCache::new(spk, body, from, to, TRANSIT_SCAN_STEP_NS)?;
        for p in &points {
            for &aspect in aspects {
                for target in aspect_targets(p.longitude, aspect) {
                    let f = |t: Instant| cache.longitude(t).map(|l| wrap180(l - target));
                    for root in scan_roots(f, from, to, TRANSIT_SCAN_STEP_NS, MICROSECOND_NS)? {
                        let retrograde = apparent(spk, body, root.t)?.speed < 0.0;
                        out.push(TransitHit {
                            at: root.t,
                            body,
                            body_name: body_name(body),
                            natal_point: p.name,
                            aspect,
                            retrograde,
                            half_width_ns: root.half_width_ns,
                        });
                    }
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
    fn saturn_hits_in_two_years_are_exact() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let from = fixture::utc(2024, 1, 1, 0, 0);
        let to = fixture::utc(2026, 1, 1, 0, 0);
        let major: Vec<Aspect> = Aspect::ALL
            .iter()
            .copied()
            .filter(|a| a.is_major())
            .collect();
        let hits = transit_hits(&spk, &natal, &[Body::SaturnBarycenter], &major, from, to).unwrap();
        assert!(!hits.is_empty());
        assert!(hits.windows(2).all(|w| w[0].at <= w[1].at));
        let mut retro = 0;
        for h in &hits {
            assert_eq!(h.body_name, "Saturn");
            assert!(from <= h.at && h.at <= to);
            assert!(h.half_width_ns <= MICROSECOND_NS);
            let a = apparent(&spk, h.body, h.at).unwrap();
            let natal_lon = crate::natal_longitude(&natal, h.natal_point).unwrap();
            assert!(
                (separation(a.longitude, natal_lon) - h.aspect.angle()).abs() < 1e-6,
                "{h:?}"
            );
            assert_eq!(h.retrograde, a.speed < 0.0);
            retro += h.retrograde as u32;
        }
        // Saturn retrogrades every year, so some contacts are retrograde ones
        assert!(retro > 0);
        // an empty window is empty
        assert!(
            transit_hits(&spk, &natal, &[Body::Moon], &major, from, from)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn the_moon_conjoins_every_natal_point_each_month() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let from = fixture::utc(2026, 1, 1, 0, 0);
        let to = from.add_ns(28 * kshana_time::NS_PER_DAY);
        let hits = transit_hits(
            &spk,
            &natal,
            &[Body::Moon],
            &[Aspect::Conjunction],
            from,
            to,
        )
        .unwrap();
        // twelve natal points, one lunar month: each conjoined once or twice
        assert!((12..=24).contains(&hits.len()), "{}", hits.len());
        assert!(hits.iter().all(|h| !h.retrograde));
    }
}
