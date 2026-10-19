//! Solar eclipses: the Moon's shadow cone against the Earth, in the Besselian
//! fundamental plane built directly from the kernel's vectors.
//!
//! The plane passes through the Earth's centre perpendicular to the shadow axis (the
//! line from the Sun's centre through the Moon's). Its `y` axis is the projection of
//! the celestial pole of date, `z` points toward the Moon and `x = y × z`. Positions
//! are astrometric (light-time only): the shadow at the Earth at `t` is the cone of
//! sunlight that passed the Moon at `t − τ`, a straight line in the inertial frame,
//! so neither body is aberrated (*Explanatory Supplement to the Astronomical
//! Almanac*, Besselian elements). In this plane (Meeus, *Astronomical Algorithms*
//! ch. 54; Explanatory Supplement ch. 11):
//!
//! * the penumbral and umbral cone half-angles are `sin f1 = (R☉ + k1 R⊕) / D`,
//!   `sin f2 = (R☉ − k2 R⊕) / D` with `D` the Sun–Moon distance, `k1 = 0.2725076`
//!   and `k2 = 0.272281` (NASA);
//! * the cone radii on the plane, with `z` the Moon's height above it, are
//!   `l1 = z tan f1 + k1 R⊕ / cos f1` and `l2 = z tan f2 − k2 R⊕ / cos f2`, `l2` being
//!   negative when the umbral vertex lies beyond the plane (a total eclipse);
//! * at a surface point of height `ζ` above the plane they are `L = l − ζ tan f`;
//! * the Earth's outline on the plane is an ellipse with semi-axes `R⊕` and
//!   `R⊕ √(1 − e² cos² d)`, `d` being the declination of the shadow axis; the
//!   height `ζ` of the surface under the axis is the larger root of the ellipsoid
//!   equation in the plane's frame, solved exactly here rather than to first order in
//!   `e²`.
//!
//! Gamma is the axis distance from the Earth's centre in Earth radii at its minimum
//! (greatest eclipse), signed by `y`. The kind follows Meeus's conditions, evaluated
//! with the exact radii instead of his mean constants: no eclipse unless the penumbra
//! reaches the outline (`Δ < l1`, Meeus's `|γ| < 1.5433 + u`); central when the axis
//! meets the surface (`|γ| < 0.9972` on the mean outline); a central eclipse is total
//! when `L2 < 0` at the surface under the axis, annular when `L2 > 0`, and hybrid
//! when it is total there but annular where the central line meets the limb
//! (`l2 > 0` at the ends; Meeus's `u < ω = 0.00464 √(1 − γ²)`); a non-central eclipse
//! whose umbra still touches the limb (`Δ < |l2|`) is a non-central total or annular
//! eclipse. The greatest magnitude is `(L1 − L2) / (L1 + L2)` under the axis for a
//! central eclipse (NASA's "ratio of apparent diameters") and `(l1 − Δ) / (l1 + l2)`
//! at the limb point nearest the axis for a partial one (Meeus eq. 54.2 with the exact
//! radii).

use crate::{
    add, argmin, bary, bpn, cross, dot, eclipse_possible, instant_of_tt_jd, lunations_in,
    mean_phase_jde, norm, perp, pole_icrf, retarded, root, saros_in_range, scale, solar_saros, sub,
    unit, EclipseError, EARTH_EQUATORIAL_RADIUS_KM, EARTH_FLATTENING, K_PENUMBRAL, K_UMBRAL,
    SUN_RADIUS_KM,
};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_time::{Instant, NS_PER_SEC};

/// What the shadow does at the Earth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SolarKind {
    /// Only the penumbra reaches the Earth.
    Partial,
    /// The antumbra reaches the surface: a ring is seen.
    Annular,
    /// The umbra reaches the surface.
    Total,
    /// Total along part of the central line and annular along the rest.
    Hybrid,
}

/// One solar eclipse, global circumstances.
#[derive(Clone, Debug, PartialEq)]
pub struct SolarEclipse {
    /// Instant of greatest eclipse: the shadow axis nearest the Earth's centre.
    pub greatest: Instant,
    /// Partial, annular, total or hybrid.
    pub kind: SolarKind,
    /// Distance of the shadow axis from the Earth's centre at greatest eclipse in
    /// Earth equatorial radii, positive when the axis passes north of the centre.
    pub gamma: f64,
    /// Greatest magnitude: for a central eclipse the ratio of the Moon's and Sun's
    /// apparent diameters under the axis; for a partial one the fraction of the Sun's
    /// diameter covered at the limb point nearest the axis.
    pub magnitude_max: f64,
    /// Saros series number ([`crate::solar_saros`]), `None` outside the span where the
    /// closed form is known to match van den Bergh's numbering.
    pub saros: Option<i32>,
    /// True when the shadow axis meets the Earth's surface (the eclipse has a central
    /// line). A total or annular eclipse with `central == false` is one whose umbra
    /// grazes the polar limb while the axis misses the Earth.
    pub central: bool,
    /// Meeus lunation number of the new Moon.
    pub lunation: i64,
    /// First external contact of the penumbra with the Earth (P1): the eclipse begins
    /// somewhere on Earth.
    pub begins: Instant,
    /// Last external contact of the penumbra (P4): the eclipse ends everywhere.
    pub ends: Instant,
}

/// Fundamental-plane geometry at one instant, in km.
struct Geometry {
    /// Distance of the axis from the Earth's centre in the plane.
    axis: f64,
    /// Northward (`y`) coordinate of the axis.
    y: f64,
    /// Penumbral radius on the plane.
    l1: f64,
    /// Umbral radius on the plane, negative beyond the vertex.
    l2: f64,
    tan_f1: f64,
    tan_f2: f64,
    /// Signed distance of the axis from the Earth's outline, negative inside.
    delta: f64,
    /// Height above the plane of the surface point under the axis, when the axis meets
    /// the Earth.
    zeta: Option<f64>,
}

/// Signed distance from `(x, y)` to the ellipse `x²/a² + y²/b² = 1`, negative inside.
/// Newton's method on the parametric angle; the outline is within 0.34 % of a circle,
/// so a handful of steps reaches machine precision.
fn ellipse_distance(x: f64, y: f64, a: f64, b: f64) -> f64 {
    let mut th = (y / b).atan2(x / a);
    for _ in 0..12 {
        let (s, c) = th.sin_cos();
        let dx = x - a * c;
        let dy = y - b * s;
        let g = dx * a * s - dy * b * c;
        let gp = a * a * s * s + dx * a * c + b * b * c * c + dy * b * s;
        if gp == 0.0 {
            break;
        }
        let step = g / gp;
        th -= step;
        if step.abs() < 1e-15 {
            break;
        }
    }
    let (s, c) = th.sin_cos();
    let d = ((x - a * c).powi(2) + (y - b * s).powi(2)).sqrt();
    if (x / a).powi(2) + (y / b).powi(2) < 1.0 {
        -d
    } else {
        d
    }
}

/// Height `ζ` above the fundamental plane of the ellipsoid's surface under the point
/// `(x, y)` of the plane, on the side facing the Moon; `None` when the line misses the
/// Earth. With the pole at `(0, cos d, sin d)` in the plane's frame, a point
/// `(x, y, ζ)` has polar component `y cos d + ζ sin d` and equatorial radius²
/// `x² + (y sin d − ζ cos d)²`, and the ellipsoid `X² + Y² + Z²/(1−f)² = R⊕²` is a
/// quadratic in `ζ`.
fn surface_zeta(x: f64, y: f64, sin_d: f64, cos_d: f64) -> Option<f64> {
    let b2 = (1.0 - EARTH_FLATTENING).powi(2);
    let qa = cos_d * cos_d + sin_d * sin_d / b2;
    let qb = 2.0 * y * sin_d * cos_d * (1.0 / b2 - 1.0);
    let qc = x * x + y * y * (sin_d * sin_d + cos_d * cos_d / b2)
        - EARTH_EQUATORIAL_RADIUS_KM * EARTH_EQUATORIAL_RADIUS_KM;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        None
    } else {
        Some((-qb + disc.sqrt()) / (2.0 * qa))
    }
}

fn geometry(spk: &Spk, t: Instant) -> Result<Geometry, SpkError> {
    let et = Spk::et_of(t);
    let earth = bary(spk, Body::Earth, et)?;
    let (moon, tau_m) = retarded(spk, Body::Moon, et, earth.position)?;
    let (sun, _) = retarded(spk, Body::Sun, et - tau_m, moon.position)?;
    let r = EARTH_EQUATORIAL_RADIUS_KM;
    // shadow axis, from the Sun through the Moon toward the Earth
    let sun_to_moon = sub(moon.position, sun.position);
    let k = unit(sun_to_moon);
    let r_m = sub(moon.position, earth.position);
    let z = -dot(r_m, k);
    let p = add(r_m, scale(k, z));
    let pole = pole_icrf(&bpn(t));
    let yhat = unit(perp(pole, k));
    let zhat = scale(k, -1.0);
    let xhat = cross(yhat, zhat);
    let x = dot(p, xhat);
    let y = dot(p, yhat);
    let sin_d = dot(zhat, pole);
    let cos_d = (1.0 - sin_d * sin_d).max(0.0).sqrt();
    // cones
    let d_sm = norm(sun_to_moon);
    let r_pen = K_PENUMBRAL * r;
    let r_umb = K_UMBRAL * r;
    let f1 = ((SUN_RADIUS_KM + r_pen) / d_sm).asin();
    let f2 = ((SUN_RADIUS_KM - r_umb) / d_sm).asin();
    let l1 = z * f1.tan() + r_pen / f1.cos();
    let l2 = z * f2.tan() - r_umb / f2.cos();
    // outline and surface
    let e2 = 1.0 - (1.0 - EARTH_FLATTENING).powi(2);
    let rho1 = (1.0 - e2 * cos_d * cos_d).sqrt();
    let delta = ellipse_distance(x, y, r, r * rho1);
    let zeta = surface_zeta(x, y, sin_d, cos_d);
    Ok(Geometry {
        axis: (x * x + y * y).sqrt(),
        y,
        l1,
        l2,
        tan_f1: f1.tan(),
        tan_f2: f2.tan(),
        delta,
        zeta,
    })
}

/// The solar eclipse at the new Moon of lunation `k`, if the penumbra reaches the Earth.
pub(crate) fn at_lunation(spk: &Spk, k: i64) -> Result<Option<SolarEclipse>, EclipseError> {
    let t0 = instant_of_tt_jd(mean_phase_jde(k as f64));
    let span = 30 * 3_600 * NS_PER_SEC;
    let greatest = argmin(
        |t| geometry(spk, t).map(|g| g.axis),
        t0.add_ns(-span),
        t0.add_ns(span),
    )?;
    let g = geometry(spk, greatest)?;
    if g.delta >= g.l1 {
        return Ok(None);
    }
    let gamma = g.axis / EARTH_EQUATORIAL_RADIUS_KM * g.y.signum();
    // The axis crosses the plane at about 0.95 km/s, so it is well clear of the
    // outline (≤ 6 378 km) three hours from the minimum and of the penumbra
    // (≤ 9 900 km) four hours from it.
    let three_h = 3 * 3_600 * NS_PER_SEC;
    let four_h = 4 * 3_600 * NS_PER_SEC;
    let (kind, magnitude_max, central) = match g.zeta {
        Some(zeta) => {
            let l1s = g.l1 - zeta * g.tan_f1;
            let l2s = g.l2 - zeta * g.tan_f2;
            let magnitude = (l1s - l2s) / (l1s + l2s);
            let kind = if l2s < 0.0 {
                // total under the axis; annular where the central line meets the limb?
                let inside = |t: Instant| geometry(spk, t).map(|gg| gg.delta);
                let t1 = root(inside, greatest.add_ns(-three_h), greatest)?;
                let t2 = root(inside, greatest, greatest.add_ns(three_h))?;
                let l2_end = geometry(spk, t1)?.l2.max(geometry(spk, t2)?.l2);
                if l2_end > 0.0 {
                    SolarKind::Hybrid
                } else {
                    SolarKind::Total
                }
            } else {
                SolarKind::Annular
            };
            (kind, magnitude, true)
        }
        None => {
            let magnitude = (g.l1 - g.delta) / (g.l1 + g.l2);
            let kind = if g.delta < g.l2.abs() {
                if g.l2 < 0.0 {
                    SolarKind::Total
                } else {
                    SolarKind::Annular
                }
            } else {
                SolarKind::Partial
            };
            (kind, magnitude, false)
        }
    };
    let penumbra = |t: Instant| geometry(spk, t).map(|gg| gg.delta - gg.l1);
    let begins = root(penumbra, greatest.add_ns(-four_h), greatest)?;
    let ends = root(penumbra, greatest, greatest.add_ns(four_h))?;
    Ok(Some(SolarEclipse {
        greatest,
        kind,
        gamma,
        magnitude_max,
        saros: saros_in_range(k).then(|| solar_saros(k)),
        central,
        lunation: k,
        begins,
        ends,
    }))
}

/// Every solar eclipse whose greatest eclipse lies in `[from, to]`, in time order.
pub fn solar_eclipses(
    spk: &Spk,
    from: Instant,
    to: Instant,
) -> Result<Vec<SolarEclipse>, EclipseError> {
    let mut out = Vec::new();
    for k in lunations_in(from, to, 0.0) {
        if !eclipse_possible(k as f64) {
            continue;
        }
        if let Some(e) = at_lunation(spk, k)? {
            if e.greatest >= from && e.greatest <= to {
                out.push(e);
            }
        }
    }
    Ok(out)
}
