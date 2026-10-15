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
            c[3] = norm360(mc + 180.0);
            c[6] = norm360(asc + 180.0);
            // Trisect the arc from MC to Asc (houses 10→1) and from Asc to IC (1→4).
            let arc_a = norm360(asc - mc); // MC → Asc
            let arc_b = norm360(c[3] - asc); // Asc → IC
            c[10] = norm360(mc + arc_a / 3.0);
            c[11] = norm360(mc + 2.0 * arc_a / 3.0);
            c[1] = norm360(asc + arc_b / 3.0);
            c[2] = norm360(asc + 2.0 * arc_b / 3.0);
            for i in 0..6 {
                c[(i + 6) % 12] = norm360(c[i] + 180.0);
            }
            // keep the explicit angles exact
            c[6] = norm360(asc + 180.0);
            c[9] = mc;
            Ok(c)
        }
        HouseSystem::Placidus => placidus(lst, eps, lat, asc, mc),
        _ => Err(HouseError::NotImplemented),
    }
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
