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

/// Years of the Vimshottari lord of the nakshatra holding `moon_sidereal_deg`: the
/// first mahadasha's lord, whose years set how far every boundary moves per second of
/// birth-time error.
pub fn first_lord_years(moon_sidereal_deg: f64) -> f64 {
    const YEARS: [f64; 9] = [7.0, 20.0, 6.0, 10.0, 7.0, 18.0, 16.0, 19.0, 17.0];
    let idx = (moon_sidereal_deg.rem_euclid(360.0) / (360.0 / 27.0)).floor() as usize % 9;
    YEARS[idx]
}

/// Shift of every Vimshottari boundary per second of birth-time error, in nanoseconds.
/// The balance of the first dasha is the Moon's share of its 13°20′ nakshatra, so one
/// second of birth time moves the first dasha's end, and with it every later boundary,
/// by (the Moon's motion in a second ÷ 13°20′) × the first lord's years: about 36
/// minutes for a Sun-first chart and two hours for a Venus-first one at 13.2°/day.
pub fn dasha_shift_per_second_ns(
    moon_speed_deg_per_day: f64,
    first_lord_years: f64,
    year_ns: i128,
) -> i128 {
    let deg_per_second = moon_speed_deg_per_day.abs() / 86_400.0;
    let fraction = deg_per_second / (360.0 / 27.0);
    (first_lord_years * year_ns as f64 * fraction) as i128
}

/// The deepest Vimshottari level (1–9) worth showing for a birth-time half-window:
/// a level stands while its shortest period outlasts four times the shift the window
/// implies. With an exact birth time every level stands.
pub fn meaningful_depth(
    moon_speed_deg_per_day: f64,
    first_lord_years: f64,
    half_window_seconds: f64,
    year_ns: i128,
) -> u8 {
    let shift = dasha_shift_per_second_ns(moon_speed_deg_per_day, first_lord_years, year_ns) as f64
        * half_window_seconds;
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
    fn a_second_of_birth_time_moves_the_dashas_by_the_first_lords_years() {
        // Venus-first (Bharani): 20 years × the Moon's share of the nakshatra per second.
        let per_s = dasha_shift_per_second_ns(13.2, 20.0, YEAR_365_25_NS);
        let hours = per_s as f64 / 3_600e9;
        assert!((hours - 2.0).abs() < 0.1, "{hours}");
        // Sun-first (Krittika): 36 minutes per second.
        let sun = dasha_shift_per_second_ns(13.2, 6.0, YEAR_365_25_NS) as f64 / 60e9;
        assert!((sun - 36.0).abs() < 2.0, "{sun}");
        assert_eq!(first_lord_years(0.0), 7.0);
        assert_eq!(first_lord_years(20.0), 20.0);
        assert_eq!(first_lord_years(359.9), 17.0);
        assert_eq!(meaningful_depth(13.2, 20.0, 0.0, YEAR_365_25_NS), 9);
        // ±1 s: about two hours for Venus-first; deha (22 h) stands, the seventh (3.7 h) not
        assert_eq!(meaningful_depth(13.2, 20.0, 1.0, YEAR_365_25_NS), 6);
        assert_eq!(meaningful_depth(13.2, 6.0, 1.0, YEAR_365_25_NS), 7);
        // ±5 min: about 25 days; pratyantar (203 d) stands, sookshma (32 d) not
        assert_eq!(meaningful_depth(13.2, 20.0, 300.0, YEAR_365_25_NS), 3);
        assert_eq!(meaningful_depth(13.2, 6.0, 300.0, YEAR_365_25_NS), 4);
        // ±3 h: about 900 days; only the mahadasha
        assert_eq!(meaningful_depth(13.2, 20.0, 10_800.0, YEAR_365_25_NS), 1);
        assert_eq!(meaningful_depth(13.2, 6.0, 10_800.0, YEAR_365_25_NS), 2);
    }
}
