//! Local circumstances of a solar eclipse for one observer.
//!
//! The topocentric angular separation of the centres of the Sun and Moon (apparent
//! directions from the observer's position and velocity, the observer placed on the
//! WGS84 ellipsoid with Greenwich apparent sidereal time from UT1) is compared with
//! the topocentric semidiameters: the external contacts C1 and C4 are the roots of
//! `sep = s☉ + s☾(k1)`, the internal contacts C2 and C3 of `sep = |s☉ − s☾(k2)|`,
//! with `k1 = 0.2725076` for the penumbral and `k2 = 0.272281` for the umbral contacts
//! as in NASA's bulletins. Maximum is the minimum of `sep`; the magnitude there is
//! `(s☉ + s☾ − sep) / (2 s☉)` and the obscuration is the overlapping area of the two
//! discs over the Sun's.

use crate::{
    angle, argmin, direction, dot, observer_state, root, EclipseError, SolarEclipse,
    EARTH_EQUATORIAL_RADIUS_KM, K_PENUMBRAL, K_UMBRAL, SUN_RADIUS_KM,
};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};

/// Local circumstances of a solar eclipse.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalSolar {
    /// First contact: the Moon's limb first touches the Sun's.
    pub c1: Instant,
    /// Second contact: totality or annularity begins (central eclipses only).
    pub c2: Option<Instant>,
    /// Maximum eclipse: the centres nearest.
    pub max: Instant,
    /// Third contact: totality or annularity ends.
    pub c3: Option<Instant>,
    /// Fourth contact: the limbs part.
    pub c4: Instant,
    /// Magnitude at maximum: fraction of the Sun's diameter covered (> 1 for a total
    /// eclipse, the ratio of diameters).
    pub magnitude: f64,
    /// Obscuration at maximum: fraction of the Sun's area covered.
    pub obscuration: f64,
    /// Geometric altitude of the Sun's centre at maximum, degrees; negative means the
    /// maximum occurs below the horizon.
    pub altitude_at_max: f64,
    /// Sun's altitude at C1 and C4, degrees.
    pub altitude_at_contacts: (f64, f64),
    /// True when the Sun is above the horizon at C1, maximum or C4.
    pub visible: bool,
}

/// Fraction of the Sun's disc (angular radius `sun`) covered by the Moon's (`moon`)
/// when their centres are `sep` apart, all in radians: the lens of two overlapping
/// circles (Weisstein, "Circle-Circle Intersection", MathWorld), with the small-angle
/// discs treated as planar.
pub fn obscuration(sun: f64, moon: f64, sep: f64) -> f64 {
    if sep >= sun + moon {
        return 0.0;
    }
    if sep <= (moon - sun).abs() {
        return if moon >= sun {
            1.0
        } else {
            (moon / sun).powi(2)
        };
    }
    let (a, b, d) = (sun, moon, sep);
    let ca = ((d * d + a * a - b * b) / (2.0 * d * a))
        .clamp(-1.0, 1.0)
        .acos();
    let cb = ((d * d + b * b - a * a) / (2.0 * d * b))
        .clamp(-1.0, 1.0)
        .acos();
    let lens = a * a * ca + b * b * cb
        - 0.5
            * ((-d + a + b) * (d + a - b) * (d - a + b) * (d + a + b))
                .max(0.0)
                .sqrt();
    lens / (std::f64::consts::PI * a * a)
}

/// Topocentric geometry at one instant.
struct Topo {
    /// Separation of the centres, radians.
    sep: f64,
    /// Sun's semidiameter, radians.
    s_sun: f64,
    /// Moon's semidiameter with the penumbral `k`, radians.
    s_moon_pen: f64,
    /// Moon's semidiameter with the umbral `k`, radians.
    s_moon_umb: f64,
    /// Sun's geometric altitude, degrees.
    sun_alt: f64,
}

fn topocentric(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
) -> Result<Topo, SpkError> {
    let (pos, vel, up) = observer_state(spk, eop, obs, t)?;
    let et = Spk::et_of(t);
    let (m, moon_dist) = direction(spk, Body::Moon, et, pos, vel, true)?;
    let (s, sun_dist) = direction(spk, Body::Sun, et, pos, vel, true)?;
    Ok(Topo {
        sep: angle(m, s),
        s_sun: (SUN_RADIUS_KM / sun_dist).asin(),
        s_moon_pen: (K_PENUMBRAL * EARTH_EQUATORIAL_RADIUS_KM / moon_dist).asin(),
        s_moon_umb: (K_UMBRAL * EARTH_EQUATORIAL_RADIUS_KM / moon_dist).asin(),
        sun_alt: dot(s, up).clamp(-1.0, 1.0).asin().to_degrees(),
    })
}

/// Local circumstances of `eclipse` for `obs`, or `None` when the Moon never covers
/// any part of the Sun as seen from there (the horizon is not considered: see
/// [`LocalSolar::visible`]). UT1 comes from `eop` when the table covers the instant,
/// else from the ΔT model.
pub fn local_solar(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    eclipse: &SolarEclipse,
) -> Result<Option<LocalSolar>, EclipseError> {
    let external =
        |t: Instant| topocentric(spk, eop, obs, t).map(|g| g.sep - (g.s_sun + g.s_moon_pen));
    // The site's partial phase lies within the global P1–P4 span; scan it in two-minute
    // steps for the sign changes that bracket C1 and C4.
    let step = 120 * NS_PER_SEC;
    let start = eclipse.begins.add_ns(-step);
    let stop = eclipse.ends.add_ns(step);
    let mut t = start;
    let mut prev = external(t)?;
    let mut c1 = None;
    let mut c4 = None;
    while t < stop {
        let next = if t.add_ns(step) > stop {
            stop
        } else {
            t.add_ns(step)
        };
        let cur = external(next)?;
        if c1.is_none() && prev > 0.0 && cur <= 0.0 {
            c1 = Some(root(external, t, next)?);
        } else if c1.is_some() && prev <= 0.0 && cur > 0.0 {
            c4 = Some(root(external, t, next)?);
            break;
        }
        t = next;
        prev = cur;
    }
    let (Some(c1), Some(c4)) = (c1, c4) else {
        return Ok(None);
    };
    let max = argmin(|t| topocentric(spk, eop, obs, t).map(|g| g.sep), c1, c4)?;
    let g = topocentric(spk, eop, obs, max)?;
    let magnitude = (g.s_sun + g.s_moon_pen - g.sep) / (2.0 * g.s_sun);
    let obscured = obscuration(g.s_sun, g.s_moon_pen, g.sep);
    let internal =
        |t: Instant| topocentric(spk, eop, obs, t).map(|g| g.sep - (g.s_sun - g.s_moon_umb).abs());
    let (c2, c3) = if internal(max)? < 0.0 {
        (
            Some(root(internal, c1, max)?),
            Some(root(internal, max, c4)?),
        )
    } else {
        (None, None)
    };
    let alt_c1 = topocentric(spk, eop, obs, c1)?.sun_alt;
    let alt_c4 = topocentric(spk, eop, obs, c4)?.sun_alt;
    Ok(Some(LocalSolar {
        c1,
        c2,
        max,
        c3,
        c4,
        magnitude,
        obscuration: obscured,
        altitude_at_max: g.sun_alt,
        altitude_at_contacts: (alt_c1, alt_c4),
        visible: alt_c1 > 0.0 || g.sun_alt > 0.0 || alt_c4 > 0.0,
    }))
}
