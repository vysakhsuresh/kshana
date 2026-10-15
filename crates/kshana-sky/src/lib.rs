//! From kernel state vectors to the numbers astrology uses: apparent geocentric
//! ecliptic longitude and latitude of date, distance, and daily motion.
//!
//! Pipeline for one body at one instant:
//! 1. Light-time: iterate the target's barycentric position at `t − τ` until
//!    `τ = |target − observer| / c` converges (three passes, < 1 ns).
//! 2. Annual aberration: add the observer's velocity over `c` to the unit direction.
//! 3. Frame: rotate from the kernel's ICRF to the true equator and equinox of date
//!    with the IAU 2006 bias-precession angles plus nutation.
//! 4. Ecliptic of date: rotate about the x-axis by the true obliquity.
//!
//! Gravitational light deflection is not yet applied (at most 1.75″ at the solar limb,
//! under 0.01″ for bodies more than 10° from the Sun); it is listed in PRECISION.md.

#![forbid(unsafe_code)]
// Small fixed-size vector and matrix code reads more clearly with indices.
#![allow(clippy::needless_range_loop)]

pub mod precession;

use kshana_ephem::{Body, Spk, SpkError};
use kshana_frames::{mean_obliquity_iau2006, nutation_truncated};
use kshana_time::Instant;
use precession::{apply, bias_precession_nutation_matrix, rotate_x};

/// Speed of light, km/s.
pub const C_KM_S: f64 = 299_792.458;

/// Apparent geocentric ecliptic position of date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apparent {
    /// Ecliptic longitude in degrees, `[0, 360)`.
    pub longitude: f64,
    /// Ecliptic latitude in degrees.
    pub latitude: f64,
    /// Distance in km.
    pub distance_km: f64,
    /// Daily motion in longitude, degrees per day (positive direct, negative retrograde).
    pub speed: f64,
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Geometric position of `target` as seen from `observer`, light-time corrected and
/// aberrated, still in the ICRF frame. Returns the direction (unit vector) and distance.
fn astrometric_direction(
    spk: &Spk,
    target: Body,
    observer: Body,
    et: f64,
) -> Result<([f64; 3], f64), SpkError> {
    let obs = spk.state(observer, Body::SolarSystemBarycenter, et)?;
    let mut tau = 0.0;
    let mut p = [0.0; 3];
    let mut dist = 0.0;
    for _ in 0..3 {
        let tgt = spk.state(target, Body::SolarSystemBarycenter, et - tau)?;
        p = [
            tgt.position[0] - obs.position[0],
            tgt.position[1] - obs.position[1],
            tgt.position[2] - obs.position[2],
        ];
        dist = norm(p);
        tau = dist / C_KM_S;
    }
    // annual aberration (classical form; relativistic terms are below 1 mas)
    let mut u = [p[0] / dist, p[1] / dist, p[2] / dist];
    for k in 0..3 {
        u[k] += obs.velocity[k] / C_KM_S;
    }
    let n = norm(u);
    Ok(([u[0] / n, u[1] / n, u[2] / n], dist))
}

/// Apparent ecliptic longitude and latitude (degrees) of an ICRF direction at `t`.
fn to_ecliptic_of_date(dir: [f64; 3], t_centuries: f64) -> (f64, f64) {
    let (dpsi, deps) = nutation_truncated(t_centuries);
    let m = bias_precession_nutation_matrix(t_centuries, dpsi, deps);
    let eq = apply(&m, dir);
    let eps_true = mean_obliquity_iau2006(t_centuries) + deps;
    let ecl = rotate_x(eps_true, eq);
    let lon = ecl[1].atan2(ecl[0]).to_degrees().rem_euclid(360.0);
    let lat = ecl[2]
        .atan2((ecl[0] * ecl[0] + ecl[1] * ecl[1]).sqrt())
        .to_degrees();
    (lon, lat)
}

/// Apparent geocentric ecliptic position of date for `body` at `t`.
pub fn apparent(spk: &Spk, body: Body, t: Instant) -> Result<Apparent, SpkError> {
    let et = Spk::et_of(t);
    let tc = t.tt_centuries_since_j2000();
    let (dir, dist) = astrometric_direction(spk, body, Body::Earth, et)?;
    let (lon, lat) = to_ecliptic_of_date(dir, tc);
    // daily motion from a symmetric 1-hour difference
    let h = 1_800.0;
    let (d1, _) = astrometric_direction(spk, body, Body::Earth, et - h)?;
    let (d2, _) = astrometric_direction(spk, body, Body::Earth, et + h)?;
    let (l1, _) = to_ecliptic_of_date(d1, tc - h / 3_155_760_000.0);
    let (l2, _) = to_ecliptic_of_date(d2, tc + h / 3_155_760_000.0);
    let mut dl = l2 - l1;
    if dl > 180.0 {
        dl -= 360.0;
    } else if dl < -180.0 {
        dl += 360.0;
    }
    Ok(Apparent {
        longitude: lon,
        latitude: lat,
        distance_km: dist,
        speed: dl * 86_400.0 / (2.0 * h),
    })
}

/// Mean lunar node, tropical longitude of date in degrees (Meeus), for `t` TT centuries.
/// The true node needs the lunar orbit's osculating elements and comes in milestone 3.
pub fn mean_node_longitude(t: f64) -> f64 {
    (125.044_52 - 1_934.136_261 * t + 0.002_070_8 * t * t + t * t * t / 450_000.0).rem_euclid(360.0)
}

/// Low-precision apparent solar longitude (Meeus, ≈0.01°), used only as a cross-check.
pub fn meeus_sun_longitude(t: f64) -> f64 {
    let l0 = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let m = (357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t).to_radians();
    let c = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * m.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * m).sin()
        + 0.000_289 * (3.0 * m).sin();
    let omega = (125.04 - 1_934.136 * t).to_radians();
    (l0 + c - 0.005_69 - 0.004_78 * omega.sin()).rem_euclid(360.0)
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
    fn sun_matches_meeus_to_a_hundredth_of_a_degree() {
        let Some(spk) = kernel() else {
            eprintln!("skipped: data/de421.bsp not present");
            return;
        };
        for (y, m, d) in [(2000, 1, 1), (2010, 6, 21), (2026, 10, 15), (2040, 3, 20)] {
            let t = Utc::new(y, m, d, 12, 0, 0, 0)
                .unwrap()
                .to_instant()
                .unwrap();
            let a = apparent(&spk, Body::Sun, t).unwrap();
            let want = meeus_sun_longitude(t.tt_centuries_since_j2000());
            let diff = (a.longitude - want + 180.0).rem_euclid(360.0) - 180.0;
            assert!(
                diff.abs() < 0.01,
                "{y}-{m}-{d}: {} vs {want} ({diff})",
                a.longitude
            );
            assert!(a.latitude.abs() < 0.001);
            assert!(a.speed > 0.95 && a.speed < 1.03, "speed {}", a.speed);
        }
    }

    #[test]
    fn new_moon_has_equal_longitudes() {
        let Some(spk) = kernel() else {
            return;
        };
        // New Moon 2000-01-06 18:14 UTC (Astronomical Almanac)
        let t = Utc::new(2000, 1, 6, 18, 14, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let s = apparent(&spk, Body::Sun, t).unwrap();
        let m = apparent(&spk, Body::Moon, t).unwrap();
        let diff = (m.longitude - s.longitude + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            diff.abs() < 0.02,
            "moon {} sun {} diff {diff}",
            m.longitude,
            s.longitude
        );
        assert!(m.speed > 11.0 && m.speed < 15.5, "moon speed {}", m.speed);
        assert!(m.distance_km > 356_000.0 && m.distance_km < 407_000.0);
    }

    #[test]
    fn mars_retrograde_is_negative_speed() {
        let Some(spk) = kernel() else {
            return;
        };
        // Mars was retrograde around its 2003-08-28 opposition.
        let t = Utc::new(2003, 8, 28, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let a = apparent(&spk, Body::MarsBarycenter, t).unwrap();
        assert!(a.speed < 0.0, "speed {}", a.speed);
    }
}
