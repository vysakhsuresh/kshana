//! Solar and lunar eclipses computed offline from the JPL kernel.
//!
//! Nothing here is tabulated: every number comes from the kernel's state vectors and
//! the same light-time and aberration pipeline `kshana-sky` uses, and every instant is
//! the root of a smooth function of time found by `kshana-solver` to one microsecond.
//!
//! * **Lunar eclipses** ([`lunar_eclipses`]): the angular distance of the apparent Moon
//!   from the apparent anti-Sun point, against the Earth's shadow radii at the Moon's
//!   distance enlarged by Danjon's rule (NASA's convention) or Chauvenet's
//!   ([`ShadowRule`]). Greatest eclipse, penumbral and umbral magnitudes, gamma, the
//!   contacts P1 U1 U2 U3 U4 P4 and the Saros series number.
//! * **Solar eclipses** ([`solar_eclipses`]): the Besselian fundamental plane built
//!   directly from the vectors (the plane through the Earth's centre perpendicular to
//!   the shadow axis), the umbral and penumbral cone radii on it and at the surface,
//!   the exact ellipsoidal outline, gamma, kind (partial, annular, total, hybrid),
//!   greatest magnitude and Saros number.
//! * **Local circumstances** ([`local_solar`]): topocentric contacts C1–C4, maximum,
//!   magnitude, obscuration and the Sun's altitude for an observer.
//! * **Cycles** ([`lunation_number`], [`solar_saros`], [`lunar_saros`]) and the
//!   **prenatal syzygy and eclipse** ([`prenatal_syzygy`], [`prenatal_eclipse`]) that
//!   Ptolemy's Animodar and Brady's eclipse work start from.
//!
//! Time scales: inputs and outputs are [`Instant`]s (TAI). Published NASA times are
//! in TD or in a UT predicted with an assumed ΔT; the difference from UTC is
//! sub-second for the years the IERS table covers.
//!
//! Sources, cited at the item that uses them: Meeus, *Astronomical Algorithms* (2nd
//! ed., 1998) ch. 49 (phases of the Moon) and ch. 54 (eclipses); Espenak & Meeus,
//! *Five Millennium Canon of Lunar Eclipses* and NASA's "Enlargement of Earth's
//! Shadows" (<https://eclipse.gsfc.nasa.gov/LEcat5/shadow.html>) for Danjon's rule and
//! the lunar constants; the *Explanatory Supplement to the Astronomical Almanac*
//! (Besselian elements); R. H. van Gent's *Catalogue of Eclipse Cycles*
//! (<https://webspace.science.uu.nl/~gent0113/eclipse/eclipsecycles_cycles.htm>) for
//! Kluepfel's Saros numbering.

#![forbid(unsafe_code)]
// Small fixed-size vector code reads more clearly with indices.
#![allow(clippy::needless_range_loop)]

mod local;
mod lunar;
mod prenatal;
mod solar;

pub use local::{local_solar, obscuration, LocalSolar};
pub use lunar::{lunar_eclipses, LunarContact, LunarEclipse, LunarKind, ShadowRule};
pub use prenatal::{prenatal_eclipse, prenatal_syzygy, PrenatalEclipse, Syzygy, SyzygyKind};
pub use solar::{solar_eclipses, SolarEclipse, SolarKind};

use kshana_ephem::{Body, Spk, SpkError, State};
use kshana_events::{EventError, MICROSECOND_NS};
use kshana_frames::precession::{apply_transpose, bias_precession_nutation_matrix, Mat3};
use kshana_frames::{earth_rotation_angle, gast, nutation_cached};
use kshana_sky::Observer;
use kshana_solver::{find_root, SolverError};
use kshana_time::{EopTable, Instant, NS_PER_SEC};

/// Speed of light, km/s (IAU).
pub const C_KM_S: f64 = 299_792.458;
/// Earth's equatorial radius, km (the unit of gamma and of the Besselian elements).
pub const EARTH_EQUATORIAL_RADIUS_KM: f64 = 6_378.137;
/// Earth's flattening used for the shadow outline (NASA eclipse predictions).
pub const EARTH_FLATTENING: f64 = 1.0 / 298.257;
/// Solar radius, km.
pub const SUN_RADIUS_KM: f64 = 696_000.0;
/// Moon's radius in Earth equatorial radii for penumbral (external) contacts and
/// lunar eclipses: NASA's `k = 0.2725076` (IAU 1982 value; 1 738.09 km).
pub const K_PENUMBRAL: f64 = 0.272_507_6;
/// Moon's radius in Earth equatorial radii for umbral (internal) contacts of solar
/// eclipses: NASA's `k = 0.272281` (1 736.65 km), the mean radius of the lunar limb
/// profile that sets total and annular durations.
pub const K_UMBRAL: f64 = 0.272_281;

/// Why no eclipse result could be produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EclipseError {
    /// Ephemeris failure (instant outside the kernel, file problem).
    Spk(SpkError),
    /// A contact or extremum could not be bracketed or did not converge.
    Solver(SolverError),
    /// A syzygy search (via `kshana-events`) failed.
    Event(EventError),
    /// The search window held no event of the kind asked for.
    NotFound,
}

impl From<SpkError> for EclipseError {
    fn from(e: SpkError) -> Self {
        EclipseError::Spk(e)
    }
}
impl From<SolverError> for EclipseError {
    fn from(e: SolverError) -> Self {
        EclipseError::Solver(e)
    }
}
impl From<EventError> for EclipseError {
    fn from(e: EventError) -> Self {
        EclipseError::Event(e)
    }
}

impl std::fmt::Display for EclipseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EclipseError::Spk(e) => write!(f, "ephemeris error: {e:?}"),
            EclipseError::Solver(e) => write!(f, "solver error: {e:?}"),
            EclipseError::Event(e) => write!(f, "syzygy search error: {e:?}"),
            EclipseError::NotFound => write!(f, "no event in the search window"),
        }
    }
}

impl std::error::Error for EclipseError {}

// ---------------------------------------------------------------------------------
// Lunations and series numbers
// ---------------------------------------------------------------------------------

/// JDE (TT) of the reference new Moon of lunation 0, 2000 Jan 6 18:14 TD (Meeus 49.1).
const LUNATION_EPOCH_JDE: f64 = 2_451_550.097_66;
/// Mean synodic month in days (Meeus 49.1).
const SYNODIC_MONTH_DAYS: f64 = 29.530_588_861;

/// JDE (TT) of the mean phase `k` of the Moon: integer `k` is a new Moon, `k + 0.5` a
/// full Moon, counted from the new Moon of 2000 Jan 6. Meeus, *Astronomical
/// Algorithms* eq. 49.1. The true phase lies within about ±15 h of this.
pub fn mean_phase_jde(k: f64) -> f64 {
    let t = k / 1_236.85;
    LUNATION_EPOCH_JDE
        + SYNODIC_MONTH_DAYS * k
        + t * t * (0.000_154_37 + t * (-0.000_000_150 + t * 0.000_000_000_73))
}

/// Mean argument of latitude of the Moon (degrees) at mean phase `k`, Meeus eq. 49.6.
/// Meeus (ch. 54) shows that no eclipse is possible when `|sin F| > 0.36`.
pub fn mean_argument_of_latitude(k: f64) -> f64 {
    let t = k / 1_236.85;
    (160.710_8
        + 390.670_502_84 * k
        + t * t * (-0.001_611_8 + t * (-0.000_002_27 + t * 0.000_000_011)))
        .rem_euclid(360.0)
}

/// TT Julian Date of an instant as one float (µs resolution near the present).
pub(crate) fn tt_jd(t: Instant) -> f64 {
    let j = t.tt_jd();
    j.jd1 + j.jd2
}

/// Instant of a TT Julian Date (rounded to the nanosecond).
pub(crate) fn instant_of_tt_jd(jd: f64) -> Instant {
    Instant::j2000().add_ns(((jd - 2_451_545.0) * 86_400.0 * 1e9).round() as i128)
}

/// Meeus lunation number of the syzygy nearest `t`: 0 for the new Moon of
/// 2000 Jan 6, +1 per synodic month (the Brown lunation number is this + 953).
/// For a full Moon this is the number of the new Moon that precedes it.
pub fn lunation_number(t: Instant) -> i64 {
    ((tt_jd(t) - LUNATION_EPOCH_JDE) / SYNODIC_MONTH_DAYS).round() as i64
}

/// Saros series number of a solar eclipse from its Meeus lunation number.
///
/// Van den Bergh's numbering advances by one per Inex (358 lunations), and
/// `38 × 358 ≡ 1 (mod 223)`, so with the lunation `LN` of the new Moon the series is
/// `((38 LN + 111) mod 223) + 1`: the mod-223 reduction of Kluepfel's (1985) algorithm
/// as given in van Gent's *Catalogue of Eclipse Cycles*
/// (`ND = LN + 105, NS = 136 + 38 ND, SNS = (NS − 1) mod 223 + 1`). Checked here
/// against NASA's catalogue: 2017 Aug 21 → 145, 2019 Dec 26 → 132, 2024 Apr 8 → 139,
/// 2026 Aug 12 → 126.
pub fn solar_saros(lunation: i64) -> i32 {
    ((38 * lunation + 111).rem_euclid(223) + 1) as i32
}

/// Saros series number of a lunar eclipse from the lunation number of the new Moon
/// that precedes the full Moon: `((38 LN + 123) mod 223) + 1`, the same reduction with
/// the lunar offset (Liu & Fiala's numbering as tabulated by NASA). Checked against
/// NASA's catalogue: 2000 Jan 21 → 124, 2019 Jan 21 → 134, 2022 Nov 8 → 136.
pub fn lunar_saros(lunation: i64) -> i32 {
    ((38 * lunation + 123).rem_euclid(223) + 1) as i32
}

/// The closed forms coincide with van den Bergh's numbers only while those lie in
/// `1..=223`: from about −1700 (when the last negative solar series ended) to about
/// +3800. Outside that span the series number is withheld rather than guessed.
pub(crate) fn saros_in_range(lunation: i64) -> bool {
    (-45_700..=22_200).contains(&lunation)
}

// ---------------------------------------------------------------------------------
// Vector and ephemeris helpers
// ---------------------------------------------------------------------------------

pub(crate) type V3 = [f64; 3];

pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(crate) fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub(crate) fn unit(a: V3) -> V3 {
    scale(a, 1.0 / norm(a))
}

/// Angle (radians) between two unit vectors, accurate for small and large angles.
pub(crate) fn angle(a: V3, b: V3) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}

/// Component of `v` perpendicular to the unit vector `axis`.
pub(crate) fn perp(v: V3, axis: V3) -> V3 {
    sub(v, scale(axis, dot(v, axis)))
}

/// Barycentric ICRF state of `body` at TDB seconds past J2000.
pub(crate) fn bary(spk: &Spk, body: Body, et: f64) -> Result<State, SpkError> {
    spk.state(body, Body::SolarSystemBarycenter, et)
}

/// State of `target` at its retarded time: light now (at `et`) arriving at the
/// barycentric point `from` left the target at `et − τ`. Three passes converge far
/// below a nanosecond. Returns the state and `τ` (s).
pub(crate) fn retarded(
    spk: &Spk,
    target: Body,
    et: f64,
    from: V3,
) -> Result<(State, f64), SpkError> {
    let mut tau = 0.0;
    let mut st = bary(spk, target, et)?;
    for _ in 0..3 {
        st = bary(spk, target, et - tau)?;
        tau = norm(sub(st.position, from)) / C_KM_S;
    }
    Ok((st, tau))
}

/// Direction (unit vector, ICRF) and distance (km) of `target` seen from an observer
/// at barycentric position `pos` moving at `vel` (km/s): light-time corrected, and
/// with annual aberration (`v/c`, classical form as in `kshana-sky`) when `aberrate`.
pub(crate) fn direction(
    spk: &Spk,
    target: Body,
    et: f64,
    pos: V3,
    vel: V3,
    aberrate: bool,
) -> Result<(V3, f64), SpkError> {
    let (st, _) = retarded(spk, target, et, pos)?;
    let p = sub(st.position, pos);
    let dist = norm(p);
    let mut u = scale(p, 1.0 / dist);
    if aberrate {
        u = unit(add(u, scale(vel, 1.0 / C_KM_S)));
    }
    Ok((u, dist))
}

/// Bias-precession-nutation matrix (ICRF → true equator and equinox of date) at `t`,
/// from the frames crate's interpolating nutation cache.
pub(crate) fn bpn(t: Instant) -> Mat3 {
    let tc = t.tt_centuries_since_j2000();
    let (dpsi, deps) = nutation_cached(tc);
    bias_precession_nutation_matrix(tc, dpsi, deps)
}

/// The true celestial pole of date as a unit vector in the ICRF: the third row of
/// the BPN matrix (`mᵀ ẑ`).
pub(crate) fn pole_icrf(m: &Mat3) -> V3 {
    [m[2][0], m[2][1], m[2][2]]
}

/// An observer's barycentric position (km), velocity (km/s) and local geodetic
/// vertical (unit vector, ICRF) at `t`, with Greenwich apparent sidereal time from UT1
/// (IERS table when it covers `t`, else the ΔT model, as `kshana-chart` does).
pub(crate) fn observer_state(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
) -> Result<(V3, V3, V3), SpkError> {
    let (ut1, _) = kshana_chart::ut1_for(t, eop);
    let tc = t.tt_centuries_since_j2000();
    let g = gast(earth_rotation_angle(ut1), tc);
    let (r_obs, v_obs) = obs.icrf_state(t, g);
    let earth = bary(spk, Body::Earth, Spk::et_of(t))?;
    // geodetic vertical: Earth-fixed → true of date (rotate by GAST) → ICRF
    let phi = obs.latitude_deg.to_radians();
    let lam = obs.longitude_deg.to_radians();
    let n_ef = [phi.cos() * lam.cos(), phi.cos() * lam.sin(), phi.sin()];
    let (sg, cg) = g.sin_cos();
    let n_tod = [
        n_ef[0] * cg - n_ef[1] * sg,
        n_ef[0] * sg + n_ef[1] * cg,
        n_ef[2],
    ];
    let up = apply_transpose(&bpn(t), n_tod);
    Ok((add(earth.position, r_obs), add(earth.velocity, v_obs), up))
}

/// Topocentric geometric altitude (degrees, no refraction) of the centre of `body`
/// for `obs` at `t`, from its apparent direction and the geodetic vertical.
pub fn altitude(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    body: Body,
    t: Instant,
) -> Result<f64, EclipseError> {
    let (pos, vel, up) = observer_state(spk, eop, obs, t)?;
    let (dir, _) = direction(spk, body, Spk::et_of(t), pos, vel, true)?;
    Ok(dot(dir, up).clamp(-1.0, 1.0).asin().to_degrees())
}

// ---------------------------------------------------------------------------------
// Solving
// ---------------------------------------------------------------------------------

/// Root of `f` in `[a, b]` to one microsecond; ephemeris errors inside the solver are
/// surfaced instead of being folded into a NaN.
pub(crate) fn root<F>(mut f: F, a: Instant, b: Instant) -> Result<Instant, EclipseError>
where
    F: FnMut(Instant) -> Result<f64, SpkError>,
{
    let mut err: Option<SpkError> = None;
    let r = find_root(
        |t| match f(t) {
            Ok(v) => v,
            Err(e) => {
                err = Some(e);
                f64::NAN
            }
        },
        a,
        b,
        MICROSECOND_NS,
    );
    if let Some(e) = err {
        return Err(EclipseError::Spk(e));
    }
    Ok(r?.t)
}

/// Instant in `[a, b]` at which the unimodal `f` is least, found as the root of its
/// derivative. The derivative is the five-point central difference with a 10 s step:
/// its truncation error vanishes where `f` is symmetric about the minimum and the
/// rounding error of the kernel positions puts the root within a few microseconds,
/// whereas a bracketing minimiser on `f` itself could not do better than the square
/// root of the machine epsilon (tens of milliseconds for the angles involved).
pub(crate) fn argmin<F>(mut f: F, a: Instant, b: Instant) -> Result<Instant, EclipseError>
where
    F: FnMut(Instant) -> Result<f64, SpkError>,
{
    const H: i128 = 10 * NS_PER_SEC;
    let mut err: Option<SpkError> = None;
    let r = find_root(
        |t| {
            let mut sample = |k: i128| f(t.add_ns(k * H));
            match (sample(-2), sample(-1), sample(1), sample(2)) {
                (Ok(m2), Ok(m1), Ok(p1), Ok(p2)) => (m2 - 8.0 * m1 + 8.0 * p1 - p2) / 120.0,
                (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => {
                    err = Some(e);
                    f64::NAN
                }
            }
        },
        a,
        b,
        MICROSECOND_NS,
    );
    if let Some(e) = err {
        return Err(EclipseError::Spk(e));
    }
    Ok(r?.t)
}

/// Lunation numbers whose mean syzygy (new Moon for `half = 0.0`, full Moon for
/// `0.5`) can lie in `[from, to]`, with one lunation of slack either side.
pub(crate) fn lunations_in(from: Instant, to: Instant, half: f64) -> std::ops::RangeInclusive<i64> {
    let k0 = ((tt_jd(from) - LUNATION_EPOCH_JDE) / SYNODIC_MONTH_DAYS - half).floor() as i64 - 1;
    let k1 = ((tt_jd(to) - LUNATION_EPOCH_JDE) / SYNODIC_MONTH_DAYS - half).ceil() as i64 + 1;
    k0..=k1
}

/// Meeus's screen (ch. 54): an eclipse is impossible when the Sun is far from a node,
/// `|sin F| > 0.36` with `F` the mean argument of latitude at the mean syzygy. The
/// margin here is wider (0.40, i.e. 23.6° from the node) to be safe against the
/// difference between the mean and the true syzygy.
pub(crate) fn eclipse_possible(k: f64) -> bool {
    mean_argument_of_latitude(k).to_radians().sin().abs() <= 0.40
}

#[cfg(test)]
mod tests;
