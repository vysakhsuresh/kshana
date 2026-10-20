//! Lunar eclipses: the Moon against the Earth's shadow at the Moon's distance.
//!
//! Geometry (Espenak & Meeus, *Five Millennium Canon of Lunar Eclipses*, and NASA's
//! "Enlargement of Earth's Shadows", <https://eclipse.gsfc.nasa.gov/LEcat5/shadow.html>):
//! let `πm`, `πs` be the equatorial horizontal parallaxes of the Moon and Sun, `ss` the
//! Sun's semidiameter and `sm = asin(k R⊕ / Δm)` the Moon's semidiameter with
//! `k = 0.2725076`. The geocentric angular radii of the shadows at the Moon's distance
//! are
//!
//! * Danjon's rule (NASA, Connaissance des Temps since 1951, Meeus & Mucke):
//!   umbra `ρu = 1.01 πm + πs − ss`, penumbra `ρp = 1.01 πm + πs + ss`, where
//!   `1.01 ≈ 1 + 1/85 − 1/594` enlarges the Earth by its 75 km occulting atmosphere
//!   and shrinks it to the radius at 45° latitude;
//! * Chauvenet's rule (Astronomical Almanac before 2015): the same radii enlarged by
//!   1/50, `1.02 (πm + πs ∓ ss)`.
//!
//! With `d` the angular distance of the Moon's centre from the shadow axis, the
//! magnitudes are `(ρ + sm − d) / (2 sm)`, greatest eclipse is the minimum of `d`,
//! and the contacts are the roots of `d = ρp + sm` (P1, P4), `d = ρu + sm` (U1, U4) and
//! `d = ρu − sm` (U2, U3).
//!
//! Both the Moon and the anti-Sun point are apparent (light-time and annual aberration
//! applied to both, as in NASA's predictions). This is the physically correct choice:
//! the shadow that the Moon shows at the observed instant `T` was cast by sunlight that
//! passed the Earth at `T − 2τ`, where `τ` is the Moon's light-time, so the relevant
//! geocentric vector is `M(T − τ) − E(T − 2τ)`, whose direction is the astrometric Moon
//! plus twice the aberration, i.e. the apparent Moon measured from the apparent
//! anti-Sun. Using astrometric places for both shifts every instant by about 75 s.

use crate::{
    altitude, angle, argmin, bary, bpn, direction, dot, eclipse_possible, instant_of_tt_jd,
    lunar_saros, lunations_in, mean_phase_jde, perp, pole_icrf, root, saros_in_range, scale, unit,
    EclipseError, EARTH_EQUATORIAL_RADIUS_KM, K_PENUMBRAL, SUN_RADIUS_KM,
};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};

/// How the Earth's shadow is enlarged for its atmosphere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadowRule {
    /// Danjon (1951): the Moon's parallax term scaled by 1.01. NASA's convention and
    /// the default.
    #[default]
    Danjon,
    /// Chauvenet: both shadow radii enlarged by 2 % (1/50).
    Chauvenet,
    /// No enlargement: the geometric shadow of the equatorial Earth.
    None,
}

/// What the Moon passes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LunarKind {
    /// Only the penumbra touches the Moon.
    Penumbral,
    /// Part of the Moon enters the umbra.
    Partial,
    /// The whole Moon is inside the umbra at greatest eclipse.
    Total,
}

/// Index into [`LunarEclipse::contacts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LunarContact {
    /// Penumbral eclipse begins.
    P1 = 0,
    /// Partial (umbral) eclipse begins.
    U1 = 1,
    /// Total eclipse begins.
    U2 = 2,
    /// Total eclipse ends.
    U3 = 3,
    /// Partial eclipse ends.
    U4 = 4,
    /// Penumbral eclipse ends.
    P4 = 5,
}

/// One lunar eclipse.
#[derive(Clone, Debug, PartialEq)]
pub struct LunarEclipse {
    /// Instant of greatest eclipse: the Moon's centre nearest the shadow axis.
    pub greatest: Instant,
    /// Penumbral, partial or total.
    pub kind: LunarKind,
    /// Penumbral magnitude at greatest eclipse (fraction of the Moon's diameter inside
    /// the penumbra).
    pub pen_magnitude: f64,
    /// Umbral magnitude at greatest eclipse; negative for a penumbral eclipse, ≥ 1 for
    /// a total one.
    pub umb_magnitude: f64,
    /// Contacts P1, U1, U2, U3, U4, P4 (see [`LunarContact`]); `None` where the Moon
    /// does not reach that shadow. Each is an exact root to one microsecond.
    pub contacts: [Option<Instant>; 6],
    /// Saros series number ([`crate::lunar_saros`]), `None` outside the span where the
    /// closed form is known to match van den Bergh's numbering.
    pub saros: Option<i32>,
    /// Distance of the Moon's centre from the shadow axis at greatest eclipse in Earth
    /// equatorial radii, positive when the Moon passes north of the axis.
    pub gamma: f64,
    /// Meeus lunation number of the new Moon preceding this full Moon.
    pub lunation: i64,
    /// The shadow rule these numbers were computed with.
    pub rule: ShadowRule,
}

impl LunarEclipse {
    /// One contact by name.
    pub fn contact(&self, c: LunarContact) -> Option<Instant> {
        self.contacts[c as usize]
    }

    /// True when the Moon's centre is above the geometric horizon of `obs` at greatest
    /// eclipse or at any of the contacts, i.e. some part of the eclipse can be seen
    /// there. Refraction and the Moon's semidiameter are not applied.
    pub fn visible_at(
        &self,
        spk: &Spk,
        eop: Option<&EopTable>,
        obs: &Observer,
    ) -> Result<bool, EclipseError> {
        for t in std::iter::once(self.greatest).chain(self.contacts.iter().flatten().copied()) {
            if altitude(spk, eop, obs, Body::Moon, t)? > 0.0 {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Apparent geocentric geometry at one instant.
struct Geometry {
    /// Angular distance of the Moon's centre from the shadow axis, radians.
    sep: f64,
    /// Offset of the Moon from the axis along the projected north pole, radians.
    north: f64,
    /// Moon's distance, km.
    moon_dist: f64,
    /// Sun's distance, km.
    sun_dist: f64,
}

fn geometry(spk: &Spk, t: Instant) -> Result<Geometry, SpkError> {
    let et = Spk::et_of(t);
    let earth = bary(spk, Body::Earth, et)?;
    let (m, moon_dist) = direction(spk, Body::Moon, et, earth.position, earth.velocity, true)?;
    let (s, sun_dist) = direction(spk, Body::Sun, et, earth.position, earth.velocity, true)?;
    let axis = scale(s, -1.0);
    let sep = angle(m, axis);
    let pole = pole_icrf(&bpn(t));
    let north = dot(perp(m, axis), unit(perp(pole, axis)));
    Ok(Geometry {
        sep,
        north,
        moon_dist,
        sun_dist,
    })
}

/// Angular radii (radians) at the Moon's distance.
struct Radii {
    penumbra: f64,
    umbra: f64,
    /// The Moon's semidiameter.
    moon: f64,
}

fn radii(g: &Geometry, rule: ShadowRule) -> Radii {
    let r = EARTH_EQUATORIAL_RADIUS_KM;
    let pi_m = (r / g.moon_dist).asin();
    let pi_s = (r / g.sun_dist).asin();
    let s_s = (SUN_RADIUS_KM / g.sun_dist).asin();
    let moon = (K_PENUMBRAL * r / g.moon_dist).asin();
    let (umbra, penumbra) = match rule {
        ShadowRule::Danjon => (1.01 * pi_m + pi_s - s_s, 1.01 * pi_m + pi_s + s_s),
        ShadowRule::Chauvenet => (1.02 * (pi_m + pi_s - s_s), 1.02 * (pi_m + pi_s + s_s)),
        ShadowRule::None => (pi_m + pi_s - s_s, pi_m + pi_s + s_s),
    };
    Radii {
        penumbra,
        umbra,
        moon,
    }
}

/// The lunar eclipse at the full Moon of lunation `k`, if the Moon touches the penumbra.
pub(crate) fn at_lunation(
    spk: &Spk,
    k: i64,
    rule: ShadowRule,
) -> Result<Option<LunarEclipse>, EclipseError> {
    let t0 = instant_of_tt_jd(mean_phase_jde(k as f64 + 0.5));
    let span = 30 * 3_600 * NS_PER_SEC;
    let greatest = argmin(
        |t| geometry(spk, t).map(|g| g.sep),
        t0.add_ns(-span),
        t0.add_ns(span),
    )?;
    let g = geometry(spk, greatest)?;
    let r = radii(&g, rule);
    let pen_magnitude = (r.penumbra + r.moon - g.sep) / (2.0 * r.moon);
    if pen_magnitude <= 0.0 {
        return Ok(None);
    }
    let umb_magnitude = (r.umbra + r.moon - g.sep) / (2.0 * r.moon);
    let kind = if umb_magnitude >= 1.0 {
        LunarKind::Total
    } else if umb_magnitude > 0.0 {
        LunarKind::Partial
    } else {
        LunarKind::Penumbral
    };
    // The Moon leaves the largest shadow (ρp + sm < 1.6°) in under 3.6 h at 0.45°/h.
    let half = 9 * 1_800 * NS_PER_SEC;
    let contact =
        |target: fn(&Radii) -> f64, before: bool| -> Result<Option<Instant>, EclipseError> {
            if g.sep >= target(&r) {
                return Ok(None);
            }
            let (a, b) = if before {
                (greatest.add_ns(-half), greatest)
            } else {
                (greatest, greatest.add_ns(half))
            };
            root(
                |t| geometry(spk, t).map(|gg| gg.sep - target(&radii(&gg, rule))),
                a,
                b,
            )
            .map(Some)
        };
    let contacts = [
        contact(|r| r.penumbra + r.moon, true)?,
        contact(|r| r.umbra + r.moon, true)?,
        contact(|r| r.umbra - r.moon, true)?,
        contact(|r| r.umbra - r.moon, false)?,
        contact(|r| r.umbra + r.moon, false)?,
        contact(|r| r.penumbra + r.moon, false)?,
    ];
    let gamma = g.moon_dist * g.sep.sin() / EARTH_EQUATORIAL_RADIUS_KM * g.north.signum();
    Ok(Some(LunarEclipse {
        greatest,
        kind,
        pen_magnitude,
        umb_magnitude,
        contacts,
        saros: saros_in_range(k).then(|| lunar_saros(k)),
        gamma,
        lunation: k,
        rule,
    }))
}

/// Every lunar eclipse (penumbral ones included) whose greatest eclipse lies in
/// `[from, to]`, in time order.
pub fn lunar_eclipses(
    spk: &Spk,
    from: Instant,
    to: Instant,
    rule: ShadowRule,
) -> Result<Vec<LunarEclipse>, EclipseError> {
    let mut out = Vec::new();
    for k in lunations_in(from, to, 0.5) {
        if !eclipse_possible(k as f64 + 0.5) {
            continue;
        }
        if let Some(e) = at_lunation(spk, k, rule)? {
            if e.greatest >= from && e.greatest <= to {
                out.push(e);
            }
        }
    }
    Ok(out)
}
