//! The ascendant as a clock. The rising degree moves a full sign in about two hours,
//! a KP sub in a few minutes and a sub-sub in seconds: the finest exact clock there is,
//! and the one most sensitive to the recorded birth time.

use kshana_chart::ut1_for;
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_cached,
};
use kshana_houses::angles;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

/// Sidereal longitude of the ascendant at `t` for an observer.
pub fn ascendant_sidereal(
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    t: Instant,
) -> f64 {
    let tc = t.tt_centuries_since_j2000();
    let (ut1, _) = ut1_for(t, eop);
    let era = earth_rotation_angle(ut1);
    let g = gast(era, tc);
    let lst = local_sidereal_time(g, obs.longitude_deg.to_radians());
    let (_, deps) = nutation_cached(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let a = angles(lst, eps, obs.latitude_deg.to_radians());
    sidereal(a.asc, ayanamsa_degrees(ayanamsa, tc))
}

/// Shift of every Vimshottari boundary per second of birth-time error, in nanoseconds.
///
/// The dasha balance is the fraction of the Moon's nakshatra left at birth; one
/// arcsecond of the Moon is 120 years × 1″ / 48 000″ of the whole cycle, about 22
/// hours. With the Moon's speed at birth in degrees per day the shift per second
/// follows exactly.
pub fn dasha_shift_per_second_ns(moon_speed_deg_per_day: f64, year_ns: i128) -> i128 {
    let arcsec_per_second = moon_speed_deg_per_day.abs() * 3_600.0 / 86_400.0;
    let cycle_ns = 120.0 * year_ns as f64;
    (cycle_ns * arcsec_per_second / 48_000.0) as i128
}

/// The deepest Vimshottari level whose periods are longer than the boundary shift
/// implied by a birth-time half-window, so the app can grey the levels below it.
/// Level spans used: 1 year or more for levels 1–3, 32 days for level 4, 5 days for
/// level 5, 22 hours for level 6, 3.7 hours for level 7, 37 minutes for level 8 and
/// 6 minutes for level 9 (the longest sub-period of a Venus period at each level).
pub fn meaningful_depth(
    moon_speed_deg_per_day: f64,
    half_window_seconds: f64,
    year_ns: i128,
) -> u8 {
    let shift =
        dasha_shift_per_second_ns(moon_speed_deg_per_day, year_ns) as f64 * half_window_seconds;
    const SPANS_NS: [f64; 9] = [
        20.0 * 365.25 * 86_400e9,
        3.33 * 365.25 * 86_400e9,
        203.0 * 86_400e9,
        32.0 * 86_400e9,
        5.3 * 86_400e9,
        22.0 * 3_600e9,
        3.7 * 3_600e9,
        37.0 * 60e9,
        6.0 * 60e9,
    ];
    let mut depth = 0u8;
    for span in SPANS_NS {
        if span > 4.0 * shift {
            depth += 1;
        } else {
            break;
        }
    }
    depth.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_vedic::dasha::YEAR_365_25_NS;

    #[test]
    fn a_second_of_birth_time_moves_the_dashas_by_hours() {
        let per_s = dasha_shift_per_second_ns(13.2, YEAR_365_25_NS);
        let hours = per_s as f64 / 3_600e9;
        assert!((hours - 12.0).abs() < 1.0, "{hours}");
        assert_eq!(meaningful_depth(13.2, 0.0, YEAR_365_25_NS), 9);
        // ±1 s: boundaries move ~12 h; prana (5 days) still stands, deha (22 h) does not
        assert_eq!(meaningful_depth(13.2, 1.0, YEAR_365_25_NS), 5);
        // ±5 min: every boundary moves ~150 days; only maha and antar remain
        assert_eq!(meaningful_depth(13.2, 300.0, YEAR_365_25_NS), 2);
        // ±3 h: about 15 years; only the mahadasha is worth showing
        assert_eq!(meaningful_depth(13.2, 10_800.0, YEAR_365_25_NS), 1);
    }
}
