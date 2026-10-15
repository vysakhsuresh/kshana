//! Angles and house cusps.
//!
//! All angles are ecliptic longitudes in degrees, `[0, 360)`. Inputs are the local
//! apparent sidereal time, the true obliquity and the geographic latitude, in radians.
//! Nothing here is approximate: Placidus is iterated to convergence, and a latitude
//! where a system is undefined returns an error instead of a made-up cusp.

#![forbid(unsafe_code)]

use std::f64::consts::TAU;

/// The two primary angles of a chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angles {
    /// Ascendant longitude in degrees.
    pub asc: f64,
    /// Midheaven longitude in degrees.
    pub mc: f64,
}

/// Supported house systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum HouseSystem {
    WholeSign,
    Equal,
    Porphyry,
    Placidus,
    Koch,
    Regiomontanus,
    Campanus,
    Alcabitius,
    Morinus,
    Topocentric,
}

/// Why cusps could not be computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseError {
    /// The system is undefined at this latitude (beyond the polar circles for the
    /// quadrant systems). Kshana shows this to the user rather than guessing.
    PolarLatitude,
    /// Iteration did not converge (should not happen inside the valid latitude range).
    NoConvergence,
    /// Not yet implemented in this scaffold.
    NotImplemented,
}

fn norm360(d: f64) -> f64 {
    d.rem_euclid(360.0)
}

/// Ecliptic longitude (degrees) of the point on the ecliptic with right ascension `ra`.
fn ecliptic_longitude_of_ra(ra: f64, eps: f64) -> f64 {
    norm360(ra.sin().atan2(ra.cos() * eps.cos()).to_degrees())
}

/// Ascendant and Midheaven from local apparent sidereal time `lst`, true obliquity
/// `eps` and geographic latitude `lat`, all radians.
pub fn angles(lst: f64, eps: f64, lat: f64) -> Angles {
    let mc = ecliptic_longitude_of_ra(lst, eps);
    let asc = lst
        .cos()
        .atan2(-(lst.sin() * eps.cos() + lat.tan() * eps.sin()));
    Angles {
        asc: norm360(asc.to_degrees()),
        mc,
    }
}

/// Twelve cusps, index 0 = first house. Cusps 1 and 10 are the Ascendant and Midheaven
/// for every quadrant system.
pub fn cusps(system: HouseSystem, lst: f64, eps: f64, lat: f64) -> Result<[f64; 12], HouseError> {
    let Angles { asc, mc } = angles(lst, eps, lat);
    match system {
        HouseSystem::WholeSign => {
            let first = (asc / 30.0).floor() * 30.0;
            Ok(std::array::from_fn(|i| norm360(first + 30.0 * i as f64)))
        }
        HouseSystem::Equal => Ok(std::array::from_fn(|i| norm360(asc + 30.0 * i as f64))),
        HouseSystem::Porphyry => {
            let mut c = [0.0; 12];
            c[0] = asc;
            c[9] = mc;
            // Trisect the arc from MC to Asc (houses 10→1) and from Asc to IC (1→4).
            let arc_a = norm360(asc - mc);
            let arc_b = norm360(mc + 180.0 - asc);
            c[10] = norm360(mc + arc_a / 3.0);
            c[11] = norm360(mc + 2.0 * arc_a / 3.0);
            c[1] = norm360(asc + arc_b / 3.0);
            c[2] = norm360(asc + 2.0 * arc_b / 3.0);
            fill_opposites(&mut c);
            Ok(c)
        }
        HouseSystem::Placidus => placidus(lst, eps, lat, asc, mc),
        HouseSystem::Koch => koch(lst, eps, lat, asc, mc),
        HouseSystem::Regiomontanus => poles(lst, eps, lat, asc, mc, Pole::Regiomontanus),
        HouseSystem::Campanus => poles(lst, eps, lat, asc, mc, Pole::Campanus),
        HouseSystem::Topocentric => poles(lst, eps, lat, asc, mc, Pole::Topocentric),
        HouseSystem::Alcabitius => alcabitius(lst, eps, lat, asc, mc),
        HouseSystem::Morinus => morinus(lst, eps, asc, mc),
    }
}

fn fill_opposites(c: &mut [f64; 12]) {
    for i in [0usize, 1, 2, 9, 10, 11] {
        c[(i + 6) % 12] = norm360(c[i] + 180.0);
    }
}

/// Koch (Birthplace): the semi-diurnal arc of the MC degree is trisected in time, and
/// the cusps are the Ascendants at those sidereal times. Undefined beyond the polar
/// circles like Placidus.
fn koch(ramc: f64, eps: f64, lat: f64, asc: f64, mc: f64) -> Result<[f64; 12], HouseError> {
    // declination of the MC degree (sin δ = sin ε · sin λ), then its semi-diurnal arc
    let dec_mc = (eps.sin() * mc.to_radians().sin()).asin();
    let arg = -dec_mc.tan() * lat.tan();
    if arg.abs() > 1.0 {
        return Err(HouseError::PolarLatitude);
    }
    let sa = arg.acos();
    let asc_at = |t: f64| angles(t, eps, lat).asc;
    let mut c = [0.0; 12];
    c[0] = asc;
    c[9] = mc;
    c[10] = asc_at(ramc - 2.0 * sa / 3.0);
    c[11] = asc_at(ramc - sa / 3.0);
    c[1] = asc_at(ramc + sa / 3.0);
    c[2] = asc_at(ramc + 2.0 * sa / 3.0);
    fill_opposites(&mut c);
    Ok(c)
}

#[derive(Clone, Copy)]
enum Pole {
    Regiomontanus,
    Campanus,
    Topocentric,
}

/// Systems defined by a "pole" latitude and a sidereal-time offset per cusp: the cusp
/// is the Ascendant computed for that pole at that time.
/// With H measured from the meridian (30°, 60°, 120°, 150° for cusps 11, 12, 2, 3), the
/// Ascendant formula is evaluated at sidereal time RAMC + H − 90° (so that at the equator
/// the cusp has right ascension RAMC + H) for the system's pole:
/// Regiomontanus: tan φ_h = tan φ · sin H.
/// Campanus: sin φ_h = sin φ · sin H, and the time offset Z with tan Z = tan H / cos φ.
/// Topocentric (Polich–Page): tan φ_h = tan φ · (H / 90°) for H ≤ 90°, else (180° − H) / 90°.
fn poles(
    ramc: f64,
    eps: f64,
    lat: f64,
    asc: f64,
    mc: f64,
    which: Pole,
) -> Result<[f64; 12], HouseError> {
    let mut c = [0.0; 12];
    c[0] = asc;
    c[9] = mc;
    for (i, h_deg) in [(10usize, 30.0f64), (11, 60.0), (1, 120.0), (2, 150.0)] {
        let h = h_deg.to_radians();
        let quarter = std::f64::consts::FRAC_PI_2;
        let (pole, t) = match which {
            Pole::Regiomontanus => ((lat.tan() * h.sin()).atan(), ramc + h - quarter),
            Pole::Campanus => {
                // handled geometrically below
                c[i] = campanus_cusp(ramc, eps, lat, h_deg);
                continue;
            }
            Pole::Topocentric => {
                let f = if h_deg <= 90.0 {
                    h_deg / 90.0
                } else {
                    (180.0 - h_deg) / 90.0
                };
                ((lat.tan() * f).atan(), ramc + h - quarter)
            }
        };
        c[i] = angles(t, eps, pole).asc;
    }
    fill_opposites(&mut c);
    Ok(c)
}

/// Campanus cusp for house angle `h_deg` (30, 60, 120, 150 for cusps 11, 12, 2, 3):
/// the prime vertical is divided into 30° arcs from the east point; each house circle
/// passes through the north and south points of the horizon and one division point;
/// the cusp is where that circle meets the ecliptic, on the division point's side.
fn campanus_cusp(ramc: f64, eps: f64, lat: f64, h_deg: f64) -> f64 {
    // equatorial frame of date: x toward RA 0, z toward the north celestial pole
    let (sr, cr) = ramc.sin_cos();
    let (sp, cp) = lat.sin_cos();
    let up = [cp * cr, cp * sr, sp];
    let north = [-sp * cr, -sp * sr, cp];
    let east = [-sr, cr, 0.0];
    // altitude of the division point on the prime vertical: 11 → 60°, 12 → 30° above
    // the east point; 2 → 30°, 3 → 60° below it
    let alt = match h_deg as i32 {
        30 => 60.0f64,
        60 => 30.0,
        120 => -30.0,
        _ => -60.0,
    }
    .to_radians();
    let p = [
        east[0] * alt.cos() + up[0] * alt.sin(),
        east[1] * alt.cos() + up[1] * alt.sin(),
        east[2] * alt.cos() + up[2] * alt.sin(),
    ];
    let n = [
        north[1] * p[2] - north[2] * p[1],
        north[2] * p[0] - north[0] * p[2],
        north[0] * p[1] - north[1] * p[0],
    ];
    // ecliptic point e(λ) = (cos λ, cos ε sin λ, sin ε sin λ) with e · n = 0
    let lam = (-n[0]).atan2(n[1] * eps.cos() + n[2] * eps.sin());
    let e = [lam.cos(), eps.cos() * lam.sin(), eps.sin() * lam.sin()];
    let dot = e[0] * p[0] + e[1] * p[1] + e[2] * p[2];
    let lam = if dot >= 0.0 {
        lam
    } else {
        lam + std::f64::consts::PI
    };
    norm360(lam.to_degrees())
}

/// Alcabitius: the Ascendant's semi-diurnal and semi-nocturnal arcs trisected in right
/// ascension; cusps are the ecliptic points at those right ascensions.
fn alcabitius(ramc: f64, eps: f64, lat: f64, asc: f64, mc: f64) -> Result<[f64; 12], HouseError> {
    let asc_rad = asc.to_radians();
    // declination and right ascension of the Ascendant degree
    let dec = (eps.sin() * asc_rad.sin()).asin();
    let arg = -dec.tan() * lat.tan();
    if arg.abs() > 1.0 {
        return Err(HouseError::PolarLatitude);
    }
    let sa = arg.acos(); // semi-diurnal arc of the Ascendant
    let sna = std::f64::consts::PI - sa;
    let mut c = [0.0; 12];
    c[0] = asc;
    c[9] = mc;
    c[10] = ecliptic_longitude_of_ra(ramc + sa / 3.0, eps);
    c[11] = ecliptic_longitude_of_ra(ramc + 2.0 * sa / 3.0, eps);
    c[1] = ecliptic_longitude_of_ra(ramc + sa + sna / 3.0, eps);
    c[2] = ecliptic_longitude_of_ra(ramc + sa + 2.0 * sna / 3.0, eps);
    fill_opposites(&mut c);
    Ok(c)
}

/// Morinus: the equator divided from the RAMC in 30° steps, each point projected onto
/// the ecliptic along a great circle through the ecliptic poles (λ = atan2(cos ε sin α,
/// cos α)); independent of latitude. Cusp 10 is therefore not the MC in general.
fn morinus(ramc: f64, eps: f64, asc: f64, mc: f64) -> Result<[f64; 12], HouseError> {
    let c: [f64; 12] = std::array::from_fn(|i| {
        let alpha = ramc + ((i as f64 + 3.0) * 30.0).to_radians();
        norm360((eps.cos() * alpha.sin()).atan2(alpha.cos()).to_degrees())
    });
    let _ = (asc, mc);
    Ok(c)
}

/// Placidus by iteration on the semi-arcs.
///
/// For cusp 11 the point lies one third of its own semi-diurnal arc east of the MC,
/// cusp 12 two thirds; cusps 2 and 3 use the semi-nocturnal arc from the IC. Each
/// cusp's RA is a fixed point of `ra = ramc + k · acos(∓ sin(ra) · tan ε · tan φ)`.
fn placidus(ramc: f64, eps: f64, lat: f64, asc: f64, mc: f64) -> Result<[f64; 12], HouseError> {
    let k = eps.tan() * lat.tan();
    if k.abs() >= 1.0 {
        return Err(HouseError::PolarLatitude);
    }
    let solve = |offset_deg: f64, frac: f64, above: bool| -> Result<f64, HouseError> {
        let mut ra = ramc + offset_deg.to_radians();
        for _ in 0..100 {
            let arg = if above { -ra.sin() * k } else { ra.sin() * k };
            if arg.abs() > 1.0 {
                return Err(HouseError::PolarLatitude);
            }
            let sa = arg.acos();
            let next = if above {
                ramc + frac * sa
            } else {
                ramc + std::f64::consts::PI - frac * sa
            };
            let delta = (next - ra).rem_euclid(TAU);
            let delta = if delta > std::f64::consts::PI {
                delta - TAU
            } else {
                delta
            };
            ra = next;
            if delta.abs() < 1e-13 {
                return Ok(ecliptic_longitude_of_ra(ra, eps));
            }
        }
        Err(HouseError::NoConvergence)
    };
    let c11 = solve(30.0, 1.0 / 3.0, true)?;
    let c12 = solve(60.0, 2.0 / 3.0, true)?;
    let c2 = solve(120.0, 2.0 / 3.0, false)?;
    let c3 = solve(150.0, 1.0 / 3.0, false)?;
    let mut c = [0.0; 12];
    c[0] = asc;
    c[1] = c2;
    c[2] = c3;
    c[9] = mc;
    c[10] = c11;
    c[11] = c12;
    for i in [0usize, 1, 2, 9, 10, 11] {
        c[(i + 6) % 12] = norm360(c[i] + 180.0);
    }
    Ok(c)
}

/// Local sidereal time (radians) at which `asc_deg` is rising, for obliquity `eps` and
/// latitude `lat`. Used for KP horary charts, where the lagna is fixed by the querent's
/// number and the other cusps follow from the moment that lagna rises at the place.
///
/// The ascendant is a monotonic function of sidereal time, so a bracketed bisection on
/// the wrapped difference converges to 1e-12 rad in about 45 steps.
pub fn lst_for_ascendant(asc_deg: f64, eps: f64, lat: f64) -> Result<f64, HouseError> {
    let target = norm360(asc_deg);
    let diff = |lst: f64| {
        let a = angles(lst, eps, lat).asc;
        let mut d = a - target;
        if d > 180.0 {
            d -= 360.0;
        } else if d <= -180.0 {
            d += 360.0;
        }
        d
    };
    // scan for a sign change in 5° steps of LST
    let n = 72;
    let step = TAU / n as f64;
    let mut lo = 0.0;
    let mut flo = diff(lo);
    for i in 1..=n {
        let hi = i as f64 * step;
        let fhi = diff(hi);
        if flo == 0.0 {
            return Ok(lo);
        }
        if (flo > 0.0) != (fhi > 0.0) && (flo - fhi).abs() < 180.0 {
            let (mut a, mut b, mut fa) = (lo, hi, flo);
            for _ in 0..80 {
                let m = 0.5 * (a + b);
                let fm = diff(m);
                if (fa > 0.0) == (fm > 0.0) {
                    a = m;
                    fa = fm;
                } else {
                    b = m;
                }
                if b - a < 1e-12 {
                    break;
                }
            }
            return Ok(0.5 * (a + b));
        }
        lo = hi;
        flo = fhi;
    }
    Err(HouseError::NoConvergence)
}

/// House cusps for a KP horary chart: the lagna is `asc_deg` (tropical) and the cusps
/// are those of the moment that longitude rises at the place.
pub fn horary_cusps(
    system: HouseSystem,
    asc_deg: f64,
    eps: f64,
    lat: f64,
) -> Result<[f64; 12], HouseError> {
    let lst = lst_for_ascendant(asc_deg, eps, lat)?;
    cusps(system, lst, eps, lat)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 23.439_279_444_f64 * std::f64::consts::PI / 180.0;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        let d = (a - b).rem_euclid(360.0);
        d < tol || d > 360.0 - tol
    }

    #[test]
    fn mc_is_aries_when_aries_culminates() {
        let a = angles(0.0, EPS, 0.5);
        assert!(close(a.mc, 0.0, 1e-9));
    }

    #[test]
    fn equator_ascendant_is_ninety_from_mc() {
        // At the equator the Ascendant RA is RAMC + 90°, so for RAMC = 0 the ascending
        // ecliptic point is the one with RA = 90°, which is longitude 90° (Cancer 0°).
        let a = angles(0.0, EPS, 0.0);
        assert!(close(a.asc, 90.0, 1e-9), "{}", a.asc);
    }

    #[test]
    fn equal_and_whole_sign() {
        let lst = 1.234;
        let eq = cusps(HouseSystem::Equal, lst, EPS, 0.3).unwrap();
        let ws = cusps(HouseSystem::WholeSign, lst, EPS, 0.3).unwrap();
        for i in 0..12 {
            assert!(close(eq[i], eq[0] + 30.0 * i as f64, 1e-9));
            assert_eq!(ws[i] % 30.0, 0.0);
        }
    }

    #[test]
    fn placidus_reduces_to_equal_ra_at_the_equator() {
        // At latitude 0 every semi-arc is 90°, so cusp 11 has RA = RAMC + 30°.
        let ramc = 0.7;
        let c = cusps(HouseSystem::Placidus, ramc, EPS, 0.0).unwrap();
        let want11 = ecliptic_longitude_of_ra(ramc + 30f64.to_radians(), EPS);
        let want2 = ecliptic_longitude_of_ra(ramc + 120f64.to_radians(), EPS);
        assert!(close(c[10], want11, 1e-9), "{} vs {}", c[10], want11);
        assert!(close(c[1], want2, 1e-9), "{} vs {}", c[1], want2);
    }

    #[test]
    fn placidus_opposites_and_order_at_mid_latitude() {
        let lat = 51.5f64.to_radians();
        let c = cusps(HouseSystem::Placidus, 2.1, EPS, lat).unwrap();
        for i in 0..6 {
            assert!(close(c[i + 6], c[i] + 180.0, 1e-9));
        }
        // cusps increase in longitude going 10 → 11 → 12 → 1 → 2 → 3 → 4
        let seq = [c[9], c[10], c[11], c[0], c[1], c[2], c[3]];
        let mut total = 0.0;
        for w in seq.windows(2) {
            let step = (w[1] - w[0]).rem_euclid(360.0);
            assert!(step > 0.0 && step < 180.0, "step {step}");
            total += step;
        }
        assert!((total - 180.0).abs() < 1e-9, "total {total}");
    }

    #[test]
    fn every_quadrant_system_keeps_angles_and_opposites() {
        let lat = 51.5f64.to_radians();
        for sys in [
            HouseSystem::Koch,
            HouseSystem::Regiomontanus,
            HouseSystem::Campanus,
            HouseSystem::Topocentric,
            HouseSystem::Alcabitius,
        ] {
            let c = cusps(sys, 2.1, EPS, lat).unwrap();
            let a = angles(2.1, EPS, lat);
            assert!(close(c[0], a.asc, 1e-9), "{sys:?} asc");
            assert!(close(c[9], a.mc, 1e-9), "{sys:?} mc");
            for i in 0..6 {
                assert!(close(c[i + 6], c[i] + 180.0, 1e-9), "{sys:?} opposite {i}");
            }
            // cusps run forward from 10 → 11 → 12 → 1 → 2 → 3 → 4
            let seq = [c[9], c[10], c[11], c[0], c[1], c[2], c[3]];
            let mut total = 0.0;
            for w in seq.windows(2) {
                let step = (w[1] - w[0]).rem_euclid(360.0);
                assert!(step > 0.0 && step < 180.0, "{sys:?} step {step}");
                total += step;
            }
            assert!((total - 180.0).abs() < 1e-9, "{sys:?} total {total}");
        }
    }

    #[test]
    fn at_the_equator_the_quadrant_systems_agree_with_equal_ra() {
        // With φ = 0 every pole is 0 and every semi-arc is 90°: all become the equal-RA
        // division of the equator from the RAMC.
        let ramc = 0.7;
        let want: Vec<f64> = (0..12)
            .map(|i| ecliptic_longitude_of_ra(ramc + ((i as f64 + 3.0) * 30.0).to_radians(), EPS))
            .collect();
        for sys in [
            HouseSystem::Placidus,
            HouseSystem::Koch,
            HouseSystem::Regiomontanus,
            HouseSystem::Campanus,
            HouseSystem::Topocentric,
            HouseSystem::Alcabitius,
        ] {
            let c = cusps(sys, ramc, EPS, 0.0).unwrap();
            for i in 0..12 {
                assert!(
                    close(c[i], want[i], 1e-7),
                    "{sys:?} cusp {} {} vs {}",
                    i + 1,
                    c[i],
                    want[i]
                );
            }
        }
    }

    #[test]
    fn lst_for_ascendant_inverts_angles() {
        let lat = 9.9312f64.to_radians();
        for i in 0..36 {
            let want = i as f64 * 10.0 + 3.7;
            let lst = lst_for_ascendant(want, EPS, lat).unwrap();
            let got = angles(lst, EPS, lat).asc;
            assert!(close(got, want, 1e-8), "{want} → lst {lst} → asc {got}");
        }
        let c = horary_cusps(HouseSystem::Placidus, 123.456, EPS, lat).unwrap();
        assert!(close(c[0], 123.456, 1e-8));
    }

    #[test]
    fn placidus_fails_honestly_at_the_pole() {
        let lat = 70f64.to_radians();
        assert_eq!(
            cusps(HouseSystem::Placidus, 1.0, EPS, lat).unwrap_err(),
            HouseError::PolarLatitude
        );
    }
}
