//! Height above sea level and the horizon.
//!
//! From a hill the sea-level horizon lies below the eye: the Sun clears it earlier in
//! the morning and drops behind it later in the evening. The angle (the dip of the
//! horizon) is about 1.76′·√h for a height of h metres, terrestrial refraction
//! included (Meeus, *Astronomical Algorithms*, 2nd ed., ch. 15; the U.S. Naval
//! Observatory's almanac rule), and acos(R/(R+h)) ≈ 1.93′·√h on the purely geometric
//! horizon, without refraction.
//!
//! Every sunrise solver here lowers the rising altitude by the dip for the observer's
//! height: by the refracted dip under the refracted upper-limb convention, and by the
//! geometric dip under the Hindu rising (the Sun's centre on the geometric horizon,
//! also without refraction), so each convention stays consistent with itself. At
//! 2,000 m, the height of Ooty or Shimla, the dip is about 79′ (refracted) and the Sun
//! rises some five to seven minutes earlier at Indian latitudes.
//!
//! The height comes from `Observer::height_m` when it is non-zero, else from a small
//! process-wide table of known places ([`set_place_elevation`]): the app registers the
//! height of the birth place and of the current place once, and every sunrise-based
//! clock (day, hora, Rahu kala, ghati, remedy timing) at those coordinates follows it
//! without any change to the many functions that build an observer at sea level.

use std::sync::RwLock;

use kshana_sky::Observer;

/// Mean Earth radius, metres (IUGG).
const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// Places whose height is known: (latitude°, longitude°, metres).
static PLACES: RwLock<Vec<(f64, f64, f64)>> = RwLock::new(Vec::new());

/// How close (degrees in latitude and longitude) a place must be to a registered one to
/// take its height: about 100 m, wide enough for coordinates stored as 32-bit floats.
const MATCH_DEG: f64 = 1e-3;

/// At most this many registered places; the oldest is dropped beyond it.
const MAX_PLACES: usize = 64;

/// Dip of the visible horizon with terrestrial refraction, degrees: 1.76′·√h.
pub fn horizon_dip_refracted_deg(height_m: f64) -> f64 {
    if height_m <= 0.0 || !height_m.is_finite() {
        return 0.0;
    }
    1.76 * height_m.sqrt() / 60.0
}

/// Dip of the geometric horizon (no refraction), degrees: acos(R/(R+h)).
pub fn horizon_dip_geometric_deg(height_m: f64) -> f64 {
    if height_m <= 0.0 || !height_m.is_finite() {
        return 0.0;
    }
    (EARTH_RADIUS_M / (EARTH_RADIUS_M + height_m)).acos().to_degrees()
}

/// Record the height above sea level of a place (0 or less forgets it).
pub fn set_place_elevation(latitude_deg: f64, longitude_deg: f64, height_m: f64) {
    let Ok(mut v) = PLACES.write() else { return };
    v.retain(|&(la, lo, _)| !same(la, lo, latitude_deg, longitude_deg));
    if height_m > 0.0 && height_m.is_finite() && latitude_deg.is_finite() && longitude_deg.is_finite() {
        v.push((latitude_deg, longitude_deg, height_m.min(9_000.0)));
        if v.len() > MAX_PLACES {
            v.remove(0);
        }
    }
}

/// Forget every registered height.
pub fn clear_place_elevations() {
    if let Ok(mut v) = PLACES.write() {
        v.clear();
    }
}

/// The registered height of a place, metres (0 when unknown).
pub fn place_elevation(latitude_deg: f64, longitude_deg: f64) -> f64 {
    PLACES
        .read()
        .ok()
        .and_then(|v| v.iter().rev().find(|&&(la, lo, _)| same(la, lo, latitude_deg, longitude_deg)).map(|p| p.2))
        .unwrap_or(0.0)
}

fn same(a_lat: f64, a_lon: f64, b_lat: f64, b_lon: f64) -> bool {
    (a_lat - b_lat).abs() <= MATCH_DEG && (a_lon - b_lon).abs() <= MATCH_DEG
}

/// The observer's height for the horizon: its own when set, else the registered one.
pub fn observer_height_m(obs: &Observer) -> f64 {
    if obs.height_m > 0.0 && obs.height_m.is_finite() {
        obs.height_m
    } else {
        place_elevation(obs.latitude_deg, obs.longitude_deg)
    }
}

/// The dip that applies to a rising convention at the observer's height, degrees.
pub fn horizon_dip_deg(rise: crate::RiseConvention, obs: &Observer) -> f64 {
    let h = observer_height_m(obs);
    match rise {
        crate::RiseConvention::UpperLimbRefracted => horizon_dip_refracted_deg(h),
        crate::RiseConvention::CentreGeometric => horizon_dip_geometric_deg(h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sun_rise_set_with, RiseConvention};
    use kshana_ephem::Spk;
    use kshana_time::Utc;

    #[test]
    fn dip_follows_the_almanac_rule() {
        assert_eq!(horizon_dip_refracted_deg(0.0), 0.0);
        assert_eq!(horizon_dip_refracted_deg(-30.0), 0.0);
        // 1.76′·√2000 = 78.7′.
        assert!((horizon_dip_refracted_deg(2_000.0) * 60.0 - 78.71).abs() < 0.05);
        // Geometric: acos(R/(R+h)) ≈ √(2h/R) rad = 86.1′ at 2000 m.
        assert!((horizon_dip_geometric_deg(2_000.0) * 60.0 - 86.1).abs() < 0.3);
    }

    #[test]
    fn the_table_matches_nearby_coordinates_only() {
        set_place_elevation(10.0889, 77.0595, 1_532.0);
        assert_eq!(place_elevation(10.0889, 77.0595), 1_532.0);
        // A 32-bit float of the same coordinates still matches.
        assert_eq!(place_elevation(10.0889_f32 as f64, 77.0595_f32 as f64), 1_532.0);
        assert_eq!(place_elevation(10.2, 77.0595), 0.0);
        let o = Observer { latitude_deg: 10.0889, longitude_deg: 77.0595, height_m: 0.0 };
        assert_eq!(observer_height_m(&o), 1_532.0);
        let own = Observer { height_m: 100.0, ..o };
        assert_eq!(observer_height_m(&own), 100.0);
        set_place_elevation(10.0889, 77.0595, 0.0);
        assert_eq!(place_elevation(10.0889, 77.0595), 0.0);
    }

    /// The Sun's altitude changes at about 15°·cos φ·cos δ per hour near the horizon, so a
    /// dip d moves sunrise by d / (15′ · cos φ · cos δ · sin-of-path) minutes; the solver must
    /// agree with that rate at 2,000 m: a few minutes earlier rising, as much later setting.
    #[test]
    fn sunrise_at_two_thousand_metres_comes_minutes_earlier() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        // Ooty, near the equinox (δ ≈ 0): the Sun climbs at 15°·cos φ per hour.
        let (lat, lon) = (11.4102, 76.6950);
        let sea = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let hill = Observer { height_m: 2_000.0, ..sea };
        let a = Utc::new(2026, 3, 20, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let b = Utc::new(2026, 3, 21, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        for rise in [RiseConvention::UpperLimbRefracted, RiseConvention::CentreGeometric] {
            let (r0, s0) = sun_rise_set_with(&spk, None, &sea, rise, a, b, 1_000_000).unwrap();
            let (r1, s1) = sun_rise_set_with(&spk, None, &hill, rise, a, b, 1_000_000).unwrap();
            let earlier = r0[0].ns_since(r1[0]) as f64 / 60e9;
            let later = s1[0].ns_since(s0[0]) as f64 / 60e9;
            let dip = horizon_dip_deg(rise, &hill);
            let expected = dip / (15.0 * lat.to_radians().cos()) * 60.0;
            assert!((5.0..=7.0).contains(&earlier), "{rise:?}: {earlier} min");
            assert!((earlier - expected).abs() < 0.3, "{rise:?}: {earlier} vs {expected}");
            assert!((later - expected).abs() < 0.3, "{rise:?}: {later} vs {expected}");
        }
    }
}
