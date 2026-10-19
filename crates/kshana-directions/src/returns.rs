//! Returns on the real sky: the Moon back to its natal longitude, and return charts.

use crate::DirectionsError;
use kshana_chart::{compute, Chart, ChartInput};
use kshana_ephem::{Body, Spk};
use kshana_events::{longitude_crossings, solar_return, MICROSECOND_NS};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::Ayanamsa;

/// Exact instants in `[from, to]` when the Moon returns to `natal_moon_tropical`
/// (about every 27.32 days), solved to a microsecond.
pub fn lunar_returns(
    spk: &Spk,
    natal_moon_tropical: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    moon_crossings(spk, natal_moon_tropical, from, to)
}

/// Exact instants in `[from, to]` when the Moon opposes `natal_moon_tropical`
/// (the demi-return), solved to a microsecond.
pub fn demi_returns(
    spk: &Spk,
    natal_moon_tropical: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    moon_crossings(spk, natal_moon_tropical + 180.0, from, to)
}

fn moon_crossings(
    spk: &Spk,
    target: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    let events = longitude_crossings(
        spk,
        Body::Moon,
        target.rem_euclid(360.0),
        from,
        to,
        MICROSECOND_NS,
    )?;
    Ok(events.into_iter().map(|e| e.root.t).collect())
}

/// Exact solar returns for each calendar year in `years`, in order; a year whose
/// return is not found (outside the kernel) is skipped.
pub fn solar_returns(
    spk: &Spk,
    natal_sun_tropical: f64,
    birth: Instant,
    years: std::ops::RangeInclusive<i64>,
) -> Result<Vec<Instant>, DirectionsError> {
    let mut out = Vec::new();
    for year in years {
        if let Some(t) = solar_return(spk, natal_sun_tropical, birth, year, MICROSECOND_NS)? {
            out.push(t);
        }
    }
    Ok(out)
}

/// The chart of a return instant for an observer, in `system`, through
/// `kshana_chart::compute` (geocentric Moon, Lahiri ayanamsa for the sidereal columns).
pub fn return_chart(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    system: HouseSystem,
) -> Result<Chart, DirectionsError> {
    Ok(compute(
        spk,
        eop,
        &ChartInput {
            instant: at,
            observer: *obs,
            system,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use kshana_sky::apparent;
    use kshana_solver::wrap180;
    use kshana_time::NS_PER_DAY;

    #[test]
    fn lunar_returns_every_twenty_seven_days() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let natal_moon = natal.bodies[1].tropical;
        let from = fixture::utc(2026, 1, 1, 0, 0);
        let to = from.add_ns(100 * NS_PER_DAY);
        let r = lunar_returns(&spk, natal_moon, from, to).unwrap();
        assert!((3..=4).contains(&r.len()), "{}", r.len());
        for w in r.windows(2) {
            let days = w[1].ns_since(w[0]) as f64 / NS_PER_DAY as f64;
            assert!((27.0..=28.0).contains(&days), "{days}");
        }
        for t in &r {
            let m = apparent(&spk, Body::Moon, *t).unwrap();
            assert!(wrap180(m.longitude - natal_moon).abs() < 1e-6);
        }
        let d = demi_returns(&spk, natal_moon, from, to).unwrap();
        assert!((3..=4).contains(&d.len()));
        for t in &d {
            let m = apparent(&spk, Body::Moon, *t).unwrap();
            assert!((wrap180(m.longitude - natal_moon).abs() - 180.0).abs() < 1e-6);
        }
        // demi-returns interleave the returns
        let mut all: Vec<(Instant, bool)> = r.iter().map(|t| (*t, true)).collect();
        all.extend(d.iter().map(|t| (*t, false)));
        all.sort();
        assert!(all.windows(2).all(|w| w[0].1 != w[1].1));
    }

    #[test]
    fn return_chart_has_the_moon_on_the_natal_degree() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let natal_moon = natal.bodies[1].tropical;
        let from = fixture::utc(2026, 3, 1, 0, 0);
        let r = lunar_returns(&spk, natal_moon, from, from.add_ns(30 * NS_PER_DAY)).unwrap();
        let chart = return_chart(
            &spk,
            None,
            &fixture::observer(),
            r[0],
            HouseSystem::Placidus,
        )
        .unwrap();
        assert_eq!(chart.instant, r[0]);
        assert!(wrap180(chart.bodies[1].tropical - natal_moon).abs() < 1e-6);
        let srs =
            solar_returns(&spk, natal.bodies[0].tropical, natal.instant, 2025..=2026).unwrap();
        assert_eq!(srs.len(), 2);
        assert!(srs[1].ns_since(srs[0]) > 364 * NS_PER_DAY);
    }
}
