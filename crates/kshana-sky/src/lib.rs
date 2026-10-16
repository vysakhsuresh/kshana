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
//! Between 1 and 2, gravitational light deflection by the Sun (ERFA `ld` with the Sun as
//! the single deflector: 1.75″ at the solar limb, 4 mas at 90°, zero at opposition).

#![forbid(unsafe_code)]
// Small fixed-size vector and matrix code reads more clearly with indices.
#![allow(clippy::needless_range_loop)]

pub use kshana_frames::precession;

use kshana_ephem::{Body, Spk, SpkError};
use kshana_frames::{mean_obliquity_iau2006, nutation_iau2006a};
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
    if target != Body::Sun {
        u = deflect_by_sun(spk, u, et - tau, obs.position, p)?;
    }
    for k in 0..3 {
        u[k] += obs.velocity[k] / C_KM_S;
    }
    let n = norm(u);
    Ok(([u[0] / n, u[1] / n, u[2] / n], dist))
}

/// Apparent ecliptic longitude and latitude (degrees) of an ICRF direction at `t`.
fn to_ecliptic_of_date(dir: [f64; 3], t_centuries: f64) -> (f64, f64) {
    let (dpsi, deps) = nutation_iau2006a(t_centuries);
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

/// Apparent geocentric ecliptic longitude of date (degrees) for `body` at `t`, without
/// the daily motion: one third of the cost of [`apparent`], for scans that only need the
/// longitude (Moon windows, muhurta steps).
pub fn apparent_longitude(spk: &Spk, body: Body, t: Instant) -> Result<f64, SpkError> {
    let et = Spk::et_of(t);
    let tc = t.tt_centuries_since_j2000();
    let (dir, _) = astrometric_direction(spk, body, Body::Earth, et)?;
    Ok(to_ecliptic_of_date(dir, tc).0)
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

/// 2GM☉/c², km: ERFA `SRS` (1.97412574336e-8 AU) in kilometres.
const SRS_KM: f64 = 1.97412574336e-8 * 149_597_870.7;

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Light deflection by one body of one solar mass (ERFA `ld`, `bm = 1`): `u` is the unit
/// direction observer → source, `q` the unit direction deflector → source, `e` the unit
/// direction deflector → observer and `em_km` that distance. Returns the deflected unit
/// direction. The limiter `dlim` follows ERFA `ldsun`.
pub fn ld(u: [f64; 3], q: [f64; 3], e: [f64; 3], em_km: f64) -> [f64; 3] {
    let qpe = [q[0] + e[0], q[1] + e[1], q[2] + e[2]];
    let qdqpe = dot(q, qpe);
    let em_au = em_km / 149_597_870.7;
    let dlim = 1e-6 / (em_au * em_au).max(1.0);
    let w = SRS_KM / em_km / qdqpe.max(dlim);
    let eq = cross(e, q);
    let peq = cross(u, eq);
    let p1 = [u[0] + w * peq[0], u[1] + w * peq[1], u[2] + w * peq[2]];
    let n = norm(p1);
    [p1[0] / n, p1[1] / n, p1[2] / n]
}

/// Deflect the light of a source by the Sun. `u` is the unit direction observer → source,
/// `obs_pos` the observer's barycentric ICRF position (km) and `src_rel` the source's
/// position relative to the observer (km) at emission; `et` is the ephemeris time at
/// which the Sun is taken.
fn deflect_by_sun(
    spk: &Spk,
    u: [f64; 3],
    et: f64,
    obs_pos: [f64; 3],
    src_rel: [f64; 3],
) -> Result<[f64; 3], SpkError> {
    let sun = spk.state(Body::Sun, Body::SolarSystemBarycenter, et)?;
    let ev = [
        obs_pos[0] - sun.position[0],
        obs_pos[1] - sun.position[1],
        obs_pos[2] - sun.position[2],
    ];
    let em = norm(ev);
    let e = [ev[0] / em, ev[1] / em, ev[2] / em];
    let sv = [ev[0] + src_rel[0], ev[1] + src_rel[1], ev[2] + src_rel[2]];
    let sn = norm(sv);
    let q = [sv[0] / sn, sv[1] / sn, sv[2] / sn];
    Ok(ld(u, q, e, em))
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

/// Geodetic observer on the WGS84 ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observer {
    /// Geodetic latitude, degrees north.
    pub latitude_deg: f64,
    /// Longitude, degrees east.
    pub longitude_deg: f64,
    /// Height above the ellipsoid, metres.
    pub height_m: f64,
}

const WGS84_A_KM: f64 = 6_378.137;
const WGS84_F: f64 = 1.0 / 298.257_223_563;
/// Earth rotation rate, rad/s.
const OMEGA_EARTH: f64 = 7.292_115e-5;

impl Observer {
    /// Geocentric position (km) and velocity (km/s) of the observer in the ICRF at `t`,
    /// given Greenwich apparent sidereal time `gast` (radians). Polar motion is ignored
    /// (< 0.3″ on the ground, far below the Moon's parallax this is used for).
    pub fn icrf_state(&self, t: Instant, gast: f64) -> ([f64; 3], [f64; 3]) {
        let phi = self.latitude_deg.to_radians();
        let lam = self.longitude_deg.to_radians();
        let h = self.height_m / 1_000.0;
        let (sp, cp) = phi.sin_cos();
        let one_f2 = (1.0 - WGS84_F) * (1.0 - WGS84_F);
        let c = 1.0 / (cp * cp + one_f2 * sp * sp).sqrt();
        let s = one_f2 * c;
        // Earth-fixed
        let r_ef = [
            (WGS84_A_KM * c + h) * cp * lam.cos(),
            (WGS84_A_KM * c + h) * cp * lam.sin(),
            (WGS84_A_KM * s + h) * sp,
        ];
        // rotate by GAST about z into the true equator and equinox of date
        let (sg, cg) = gast.sin_cos();
        let r_tod = [
            r_ef[0] * cg - r_ef[1] * sg,
            r_ef[0] * sg + r_ef[1] * cg,
            r_ef[2],
        ];
        let v_tod = [-OMEGA_EARTH * r_tod[1], OMEGA_EARTH * r_tod[0], 0.0];
        // true-of-date → ICRF is the transpose of the bias-precession-nutation matrix
        let tc = t.tt_centuries_since_j2000();
        let (dpsi, deps) = nutation_iau2006a(tc);
        let m = bias_precession_nutation_matrix(tc, dpsi, deps);
        (
            precession::apply_transpose(&m, r_tod),
            precession::apply_transpose(&m, v_tod),
        )
    }
}

/// Apparent topocentric ecliptic position of date: as [`apparent`], but seen from an
/// observer on the Earth's surface. For the Moon this differs from the geocentric
/// value by up to about one degree (parallax); for the Sun by up to 9″.
pub fn apparent_topocentric(
    spk: &Spk,
    body: Body,
    t: Instant,
    obs: &Observer,
    gast: f64,
) -> Result<Apparent, SpkError> {
    let et = Spk::et_of(t);
    let tc = t.tt_centuries_since_j2000();
    let (r_obs, v_obs) = obs.icrf_state(t, gast);
    let dir_at = |et: f64| -> Result<([f64; 3], f64), SpkError> {
        let earth = spk.state(Body::Earth, Body::SolarSystemBarycenter, et)?;
        let o_pos = [
            earth.position[0] + r_obs[0],
            earth.position[1] + r_obs[1],
            earth.position[2] + r_obs[2],
        ];
        let o_vel = [
            earth.velocity[0] + v_obs[0],
            earth.velocity[1] + v_obs[1],
            earth.velocity[2] + v_obs[2],
        ];
        let mut tau = 0.0;
        let mut p = [0.0; 3];
        let mut dist = 0.0;
        for _ in 0..3 {
            let tgt = spk.state(body, Body::SolarSystemBarycenter, et - tau)?;
            p = [
                tgt.position[0] - o_pos[0],
                tgt.position[1] - o_pos[1],
                tgt.position[2] - o_pos[2],
            ];
            dist = norm(p);
            tau = dist / C_KM_S;
        }
        let mut u = [p[0] / dist, p[1] / dist, p[2] / dist];
    if body != Body::Sun {
        u = deflect_by_sun(spk, u, et - tau, o_pos, p)?;
    }
        for k in 0..3 {
            u[k] += o_vel[k] / C_KM_S;
        }
        let n = norm(u);
        Ok(([u[0] / n, u[1] / n, u[2] / n], dist))
    };
    let (dir, dist) = dir_at(et)?;
    let (lon, lat) = to_ecliptic_of_date(dir, tc);
    let h = 1_800.0;
    let (d1, _) = dir_at(et - h)?;
    let (d2, _) = dir_at(et + h)?;
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

/// True (osculating) ascending node of the Moon's geocentric orbit, tropical longitude
/// of date in degrees. Computed from the instantaneous position and velocity in the
/// ecliptic of date: the node line is `ẑ × (r × v)`.
pub fn true_node_longitude(spk: &Spk, t: Instant) -> Result<f64, SpkError> {
    let et = Spk::et_of(t);
    let tc = t.tt_centuries_since_j2000();
    let st = spk.state(Body::Moon, Body::Earth, et)?;
    let (dpsi, deps) = nutation_iau2006a(tc);
    let m = bias_precession_nutation_matrix(tc, dpsi, deps);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let r = rotate_x(eps, apply(&m, st.position));
    let v = rotate_x(eps, apply(&m, st.velocity));
    let h = [
        r[1] * v[2] - r[2] * v[1],
        r[2] * v[0] - r[0] * v[2],
        r[0] * v[1] - r[1] * v[0],
    ];
    // n = ẑ × h = (−h_y, h_x, 0)
    Ok((h[0]).atan2(-h[1]).to_degrees().rem_euclid(360.0))
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

    #[test]
    fn true_node_stays_within_two_degrees_of_the_mean_node() {
        let Some(spk) = kernel() else { return };
        let mut max = 0.0f64;
        for i in 0..40 {
            let t = Utc::new(2000 + i, 3, 1, 0, 0, 0, 0)
                .unwrap()
                .to_instant()
                .unwrap();
            let tn = true_node_longitude(&spk, t).unwrap();
            let mn = mean_node_longitude(t.tt_centuries_since_j2000());
            let d = ((tn - mn + 180.0).rem_euclid(360.0) - 180.0).abs();
            max = max.max(d);
            assert!(d < 2.0, "year {}: true {tn} mean {mn} diff {d}", 2000 + i);
        }
        assert!(
            max > 0.2,
            "true node never departed from the mean node: suspicious ({max})"
        );
    }

    #[test]
    fn topocentric_moon_parallax_is_under_a_degree_and_sun_under_ten_arcsec() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        for d in [1u32, 8, 15, 22] {
            let t = Utc::new(2026, 10, d, 9, 0, 0, 0)
                .unwrap()
                .to_instant()
                .unwrap();
            // GAST from UT1 ≈ UTC (error < 1 s → < 0.01″ here), via ΔT-free shortcut
            let ut1 = kshana_time::TwoPartJd {
                jd1: t.tt_jd().jd1,
                jd2: t.tt_jd().jd2 - 69.2 / 86_400.0,
            };
            let g = kshana_frames::gast(
                kshana_frames::earth_rotation_angle(ut1),
                t.tt_centuries_since_j2000(),
            );
            let geo = apparent(&spk, Body::Moon, t).unwrap();
            let topo = apparent_topocentric(&spk, Body::Moon, t, &obs, g).unwrap();
            let dl = ((topo.longitude - geo.longitude + 180.0).rem_euclid(360.0) - 180.0).abs();
            let db = (topo.latitude - geo.latitude).abs();
            let sep = (dl * dl + db * db).sqrt();
            assert!(sep > 0.05 && sep < 1.05, "day {d}: parallax {sep}°");
            assert!(
                topo.distance_km < geo.distance_km + 6_400.0
                    && topo.distance_km > geo.distance_km - 6_400.0
            );
            let sg = apparent(&spk, Body::Sun, t).unwrap();
            let st = apparent_topocentric(&spk, Body::Sun, t, &obs, g).unwrap();
            let ds =
                ((st.longitude - sg.longitude + 180.0).rem_euclid(360.0) - 180.0).abs() * 3_600.0;
            assert!(ds < 10.0, "sun parallax {ds}″");
        }
    }
}

#[cfg(test)]
mod deflection_tests {
    use super::*;

    const AU_KM: f64 = 149_597_870.7;
    const ARCSEC: f64 = std::f64::consts::PI / 180.0 / 3600.0;

    /// A distant source (q = u) seen from 1 AU: ERFA's closed form is
    /// δ = (2GM/c²)/em · tan(φ/2), φ the angle between the anti-Sun direction and the source.
    fn delta(phi_deg: f64) -> f64 {
        let e = [1.0, 0.0, 0.0]; // Sun → observer
        let phi = phi_deg.to_radians();
        let u = [phi.cos(), phi.sin(), 0.0]; // source at angle φ from e (anti-Sun)
        let p1 = ld(u, u, e, AU_KM);
        norm(cross(u, p1)).clamp(-1.0, 1.0).asin()
    }

    #[test]
    fn deflection_is_zero_at_opposition_and_4_mas_at_quadrature() {
        assert!(delta(0.0) < 1e-12);
        let d90 = delta(90.0) / ARCSEC;
        assert!((d90 - 0.00407).abs() < 0.0001, "{d90}");
    }

    #[test]
    fn deflection_at_the_solar_limb_is_about_1_75_arcsec() {
        // the limb is 0.2666° from the Sun's centre, i.e. φ = 180° − 0.2666°
        let d = delta(180.0 - 0.2666) / ARCSEC;
        assert!((d - 1.75).abs() < 0.02, "{d}");
    }

    #[test]
    fn deflection_bends_light_away_from_the_sun() {
        // the apparent position moves away from the Sun: the angle from the Sun grows
        let e = [1.0, 0.0, 0.0];
        let phi = (180.0_f64 - 1.0).to_radians();
        let u = [phi.cos(), phi.sin(), 0.0];
        let p1 = ld(u, u, e, AU_KM);
        let sun_dir = [-1.0, 0.0, 0.0];
        assert!(dot(p1, sun_dir) < dot(u, sun_dir));
    }
}
