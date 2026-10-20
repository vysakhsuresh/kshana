//! Exact instants on the real sky.
//!
//! Every function here scans a time range in coarse steps, looks for a sign change in
//! some smooth function of the sky, and hands the bracket to `kshana-solver`, which
//! returns the instant to the requested tolerance together with its bracket width.
//! The step must be shorter than half the shortest period of the function: the Moon's
//! longitude moves about 13° a day, so six-hour steps are safe for every function here.

#![forbid(unsafe_code)]

use kshana_ephem::{Body, Spk, SpkError};
use kshana_sky::apparent;
use kshana_solver::{find_root, wrap180, Root, SolverError};
use kshana_time::{Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa, NAKSHATRA_SPAN};

/// Default solver tolerance: one microsecond.
pub const MICROSECOND_NS: i128 = 1_000;

/// What was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// A body reaches an exact longitude (ingress, or any chosen degree).
    Longitude,
    /// Two bodies reach an exact angular separation (aspect, lunation).
    Aspect,
    /// Moon − Sun elongation crosses a tithi boundary.
    Tithi,
    /// A body crosses a nakshatra boundary (sidereal).
    Nakshatra,
    /// A body's longitude speed crosses zero (station).
    Station,
}

/// One solved instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// What kind of event.
    pub kind: EventKind,
    /// The solved instant and bracket.
    pub root: Root,
}

/// Errors from event search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Solver failure.
    Solver(SolverError),
}

impl From<SpkError> for EventError {
    fn from(e: SpkError) -> Self {
        EventError::Spk(e)
    }
}
impl From<SolverError> for EventError {
    fn from(e: SolverError) -> Self {
        EventError::Solver(e)
    }
}

/// Scan `[from, to]` in `step_ns` steps for sign changes of `f`, solving each to `tol_ns`.
/// `f` returns `Err` only on ephemeris failure.
fn scan<F>(
    mut f: F,
    from: Instant,
    to: Instant,
    step_ns: i128,
    tol_ns: i128,
    kind: EventKind,
) -> Result<Vec<Event>, EventError>
where
    F: FnMut(Instant) -> Result<f64, SpkError>,
{
    let mut out = Vec::new();
    let mut a = from;
    let mut fa = f(a)?;
    while a < to {
        let b = if a.add_ns(step_ns) > to {
            to
        } else {
            a.add_ns(step_ns)
        };
        let fb = f(b)?;
        if fa == 0.0 {
            out.push(Event {
                kind,
                root: Root {
                    t: a,
                    half_width_ns: 0,
                    evaluations: 0,
                },
            });
        } else if (fa > 0.0) != (fb > 0.0) && (fa - fb).abs() < 180.0 {
            // the second test rejects the 360° wrap of a longitude difference
            let root = find_root(|t| f(t).unwrap_or(f64::NAN), a, b, tol_ns)?;
            out.push(Event { kind, root });
        }
        a = b;
        fa = fb;
    }
    Ok(out)
}

/// Instants in `[from, to]` when `body` reaches tropical longitude `target_deg`.
pub fn longitude_crossings(
    spk: &Spk,
    body: Body,
    target_deg: f64,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    let f = |t: Instant| apparent(spk, body, t).map(|a| wrap180(a.longitude - target_deg));
    scan(
        f,
        from,
        to,
        6 * 3_600 * NS_PER_SEC,
        tol_ns,
        EventKind::Longitude,
    )
}

/// Instants when `body` enters a tropical sign (any of the twelve 30° boundaries).
pub fn sign_ingresses(
    spk: &Spk,
    body: Body,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    // distance to the nearest boundary, signed by direction of travel: f = wrap30 − 15
    let f = |t: Instant| apparent(spk, body, t).map(|a| a.longitude.rem_euclid(30.0) - 15.0);
    // a crossing of a boundary is a jump from +15 to −15 (direct) or the reverse; detect
    // via the wrapped difference instead, so use the raw 30° phase and look for jumps
    let mut out = Vec::new();
    let step = 6 * 3_600 * NS_PER_SEC;
    let mut a = from;
    let mut pa = apparent(spk, body, a)?.longitude;
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let pb = apparent(spk, body, b)?.longitude;
        let sa = (pa / 30.0).floor();
        let sb = (pb / 30.0).floor();
        if sa != sb {
            // the boundary crossed is the one between the two signs in the direction moved
            let boundary = if wrap180(pb - pa) > 0.0 {
                sb * 30.0
            } else {
                sa * 30.0
            };
            let g = |t: Instant| {
                apparent(spk, body, t)
                    .map(|x| wrap180(x.longitude - boundary))
                    .unwrap_or(f64::NAN)
            };
            let root = find_root(g, a, b, tol_ns)?;
            out.push(Event {
                kind: EventKind::Longitude,
                root,
            });
        }
        a = b;
        pa = pb;
    }
    let _ = f;
    Ok(out)
}

/// Instants when the separation `lon(a) − lon(b)` equals `angle_deg` (0 = conjunction,
/// 180 = opposition, 90 = square...). Both directions of the angle are found.
pub fn aspects(
    spk: &Spk,
    a: Body,
    b: Body,
    angle_deg: f64,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    let mut out = Vec::new();
    let targets: Vec<f64> = if angle_deg == 0.0 || angle_deg == 180.0 {
        vec![angle_deg]
    } else {
        vec![angle_deg, -angle_deg]
    };
    for target in targets {
        let f = |t: Instant| {
            let la = apparent(spk, a, t)?.longitude;
            let lb = apparent(spk, b, t)?.longitude;
            Ok(wrap180(la - lb - target))
        };
        out.extend(scan(
            f,
            from,
            to,
            6 * 3_600 * NS_PER_SEC,
            tol_ns,
            EventKind::Aspect,
        )?);
    }
    out.sort_by_key(|e| e.root.t);
    Ok(out)
}

/// New moons in the range.
pub fn new_moons(
    spk: &Spk,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    aspects(spk, Body::Moon, Body::Sun, 0.0, from, to, tol_ns)
}

/// Full moons in the range.
pub fn full_moons(
    spk: &Spk,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    aspects(spk, Body::Moon, Body::Sun, 180.0, from, to, tol_ns)
}

/// Tithi boundaries: every 12° of Moon − Sun elongation.
pub fn tithi_boundaries(
    spk: &Spk,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    phase_boundaries(spk, 12.0, from, to, tol_ns, EventKind::Tithi, |spk, t| {
        let m = apparent(spk, Body::Moon, t)?.longitude;
        let s = apparent(spk, Body::Sun, t)?.longitude;
        Ok((m - s).rem_euclid(360.0))
    })
}

/// Nakshatra boundaries of `body` in the sidereal zodiac (every 13°20′).
pub fn nakshatra_boundaries(
    spk: &Spk,
    body: Body,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    phase_boundaries(
        spk,
        NAKSHATRA_SPAN,
        from,
        to,
        tol_ns,
        EventKind::Nakshatra,
        move |spk, t| {
            let a = apparent(spk, body, t)?;
            let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
            Ok(sidereal(a.longitude, ay))
        },
    )
}

/// Generic "phase crosses a multiple of `span`" finder.
fn phase_boundaries<F>(
    spk: &Spk,
    span: f64,
    from: Instant,
    to: Instant,
    tol_ns: i128,
    kind: EventKind,
    phase: F,
) -> Result<Vec<Event>, EventError>
where
    F: Fn(&Spk, Instant) -> Result<f64, SpkError>,
{
    let mut out = Vec::new();
    let step = 3 * 3_600 * NS_PER_SEC;
    let mut a = from;
    let mut pa = phase(spk, a)?;
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let pb = phase(spk, b)?;
        let ka = (pa / span).floor();
        let kb = (pb / span).floor();
        if ka != kb {
            let boundary = if wrap180(pb - pa) > 0.0 {
                kb * span
            } else {
                ka * span
            };
            let g = |t: Instant| {
                phase(spk, t)
                    .map(|p| wrap180(p - boundary))
                    .unwrap_or(f64::NAN)
            };
            let root = find_root(g, a, b, tol_ns)?;
            out.push(Event { kind, root });
        }
        a = b;
        pa = pb;
    }
    Ok(out)
}

/// Solar return: the instant in calendar year `year` (UTC) when the Sun returns to the
/// natal tropical longitude. Searched in a ±3 day window around the birthday.
pub fn solar_return(
    spk: &Spk,
    natal_sun_tropical: f64,
    birth: Instant,
    year: i64,
    tol_ns: i128,
) -> Result<Option<Instant>, EventError> {
    let b = Utc::from_instant(birth);
    let day = b.day.min(28); // keep the anchor valid in February; the window covers the rest
    let anchor = Utc::new(year, b.month, day, b.hour, b.minute, b.second.min(59), 0)
        .ok()
        .and_then(|u| u.to_instant().ok());
    let Some(anchor) = anchor else {
        return Ok(None);
    };
    let from = anchor.add_ns(-4 * NS_PER_DAY);
    let to = anchor.add_ns(4 * NS_PER_DAY);
    let ev = longitude_crossings(spk, Body::Sun, natal_sun_tropical, from, to, tol_ns)?;
    Ok(ev.first().map(|e| e.root.t))
}

/// Stations: instants where the body's longitude speed is zero.
pub fn stations(
    spk: &Spk,
    body: Body,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<Vec<Event>, EventError> {
    let f = |t: Instant| apparent(spk, body, t).map(|a| a.speed);
    scan(
        f,
        from,
        to,
        24 * 3_600 * NS_PER_SEC,
        tol_ns,
        EventKind::Station,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn utc(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> Instant {
        Utc::new(y, mo, d, h, mi, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }

    #[test]
    fn new_moon_january_2000_within_a_minute_of_the_almanac() {
        let Some(spk) = kernel() else { return };
        // Astronomical Almanac: New Moon 2000 Jan 6 18:14 UTC
        let ev = new_moons(
            &spk,
            utc(2000, 1, 1, 0, 0),
            utc(2000, 1, 31, 0, 0),
            MICROSECOND_NS,
        )
        .unwrap();
        assert_eq!(ev.len(), 1, "{ev:?}");
        let want = utc(2000, 1, 6, 18, 14);
        let off_s = ev[0].root.t.ns_since(want) as f64 / 1e9;
        assert!(off_s.abs() < 60.0, "off by {off_s} s");
        assert!(ev[0].root.half_width_ns <= MICROSECOND_NS);
    }

    #[test]
    fn sun_enters_aries_at_the_2026_equinox() {
        let Some(spk) = kernel() else { return };
        // March equinox 2026: 2026-03-20 14:46 UTC
        let ev = sign_ingresses(
            &spk,
            Body::Sun,
            utc(2026, 3, 1, 0, 0),
            utc(2026, 4, 1, 0, 0),
            MICROSECOND_NS,
        )
        .unwrap();
        assert_eq!(ev.len(), 1, "{ev:?}");
        let off_s = ev[0].root.t.ns_since(utc(2026, 3, 20, 14, 46)) as f64 / 1e9;
        assert!(off_s.abs() < 120.0, "off by {off_s} s");
    }

    #[test]
    fn thirty_tithis_in_a_lunar_month() {
        let Some(spk) = kernel() else { return };
        let ev = tithi_boundaries(
            &spk,
            utc(2026, 1, 1, 0, 0),
            utc(2026, 2, 1, 0, 0),
            MICROSECOND_NS,
        )
        .unwrap();
        // 31 days ≈ 1.05 lunar months → 31 or 32 boundaries
        assert!(ev.len() >= 30 && ev.len() <= 33, "{}", ev.len());
        for w in ev.windows(2) {
            let gap_h = w[1].root.t.ns_since(w[0].root.t) as f64 / 3.6e12;
            assert!(gap_h > 18.0 && gap_h < 28.0, "gap {gap_h} h");
        }
    }

    #[test]
    fn solar_return_lands_near_the_birthday() {
        let Some(spk) = kernel() else { return };
        let birth = utc(1990, 10, 29, 17, 36);
        let natal = apparent(&spk, Body::Sun, birth).unwrap().longitude;
        let sr = solar_return(&spk, natal, birth, 2026, MICROSECOND_NS)
            .unwrap()
            .unwrap();
        let u = Utc::from_instant(sr);
        assert_eq!((u.year, u.month), (2026, 10));
        assert!((u.day as i32 - 29).abs() <= 1, "{u:?}");
        let again = apparent(&spk, Body::Sun, sr).unwrap().longitude;
        assert!(wrap180(again - natal).abs() < 1e-6, "{again} vs {natal}");
    }

    #[test]
    fn mars_stations_twice_around_the_2003_opposition() {
        let Some(spk) = kernel() else { return };
        // Mars stationed retrograde 2003-07-29 and direct 2003-09-27 (approx.)
        let ev = stations(
            &spk,
            Body::MarsBarycenter,
            utc(2003, 6, 1, 0, 0),
            utc(2003, 11, 1, 0, 0),
            MICROSECOND_NS,
        )
        .unwrap();
        assert_eq!(ev.len(), 2, "{ev:?}");
        let d1 = Utc::from_instant(ev[0].root.t);
        let d2 = Utc::from_instant(ev[1].root.t);
        assert_eq!((d1.month, d1.day), (7, 29), "{d1:?}");
        assert_eq!((d2.month, d2.day), (9, 27), "{d2:?}");
    }
}

// ---------------------------------------------------------------------------------
// Rising and setting, planetary hours, Rahu kala
// ---------------------------------------------------------------------------------

use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_iau2006a,
};
use kshana_sky::Observer;
use kshana_time::{delta_t_for_instant, ut1_jd_from_delta_t, EopTable, Utc};

/// Apparent altitude (degrees) of the Sun's centre above the geometric horizon, without
/// refraction, for an observer, using UT1 from `eop` when available.
pub fn sun_altitude(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
) -> Result<f64, SpkError> {
    let a = apparent(spk, Body::Sun, t)?;
    let tc = t.tt_centuries_since_j2000();
    let (_, deps) = nutation_iau2006a(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    // ecliptic → equatorial of date
    let (sl, cl) = a.longitude.to_radians().sin_cos();
    let (sb, cb) = a.latitude.to_radians().sin_cos();
    let (se, ce) = eps.sin_cos();
    let x = cb * cl;
    let y = cb * sl * ce - sb * se;
    let z = cb * sl * se + sb * ce;
    let ra = y.atan2(x);
    let dec = z.asin();
    // UT1
    let utc = Utc::from_instant(t);
    let mjd = utc.utc_day() as f64
        + 40_587.0
        + (utc.hour as f64 * 3_600.0
            + utc.minute as f64 * 60.0
            + utc.second as f64
            + utc.nanos as f64 * 1e-9)
            / 86_400.0;
    let ut1 = match eop.and_then(|e| e.ut1_minus_utc(mjd)) {
        Some(u) => kshana_time::TwoPartJd {
            jd1: 2_400_000.5 + mjd.floor(),
            jd2: mjd.fract() + u.ut1_minus_utc / 86_400.0,
        },
        None => ut1_jd_from_delta_t(t, delta_t_for_instant(t)),
    };
    let lst = local_sidereal_time(
        gast(earth_rotation_angle(ut1), tc),
        obs.longitude_deg.to_radians(),
    );
    let h = lst - ra;
    let phi = obs.latitude_deg.to_radians();
    let sin_alt = phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos();
    Ok(sin_alt.asin().to_degrees())
}

/// Standard rising/setting altitude of the Sun's centre: refraction 34′ plus semi-diameter 16′.
pub const SUN_RISE_SET_ALT: f64 = -0.8333;

/// Which sunrise the day's clocks are built on. The two conventions differ by about
/// two minutes at the tropics and more at high latitudes, which moves every hora,
/// Rahu kala, ghati and Gulika by the same amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiseConvention {
    /// The Sun's upper limb with refraction (centre at −0°50′): Western almanacs, Drik Panchang.
    UpperLimbRefracted,
    /// The Sun's centre on the geometric horizon, no refraction: the Hindu rising
    /// (Swiss Ephemeris `SE_BIT_HINDU_RISING`), Kerala and Tamil almanacs.
    CentreGeometric,
}

impl RiseConvention {
    /// The altitude of the Sun's centre at rising and setting under this convention.
    pub fn altitude_deg(self) -> f64 {
        match self {
            RiseConvention::UpperLimbRefracted => SUN_RISE_SET_ALT,
            RiseConvention::CentreGeometric => 0.0,
        }
    }
}

static RISE_CONVENTION: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Set the process-wide rising convention used by every sunrise-based clock.
pub fn set_rise_convention(c: RiseConvention) {
    RISE_CONVENTION.store(
        match c {
            RiseConvention::UpperLimbRefracted => 0,
            RiseConvention::CentreGeometric => 1,
        },
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// The process-wide rising convention (upper limb with refraction unless changed).
pub fn rise_convention() -> RiseConvention {
    match RISE_CONVENTION.load(std::sync::atomic::Ordering::Relaxed) {
        1 => RiseConvention::CentreGeometric,
        _ => RiseConvention::UpperLimbRefracted,
    }
}

/// Sunrise and sunset instants under the process-wide [`rise_convention`] in
/// `[from, to]`, as `(rises, sets)` lists. Polar day or night yields empty lists.
pub fn sun_rise_set(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    from: Instant,
    to: Instant,
    tol_ns: i128,
) -> Result<(Vec<Instant>, Vec<Instant>), EventError> {
    let alt = rise_convention().altitude_deg();
    let f = |t: Instant| sun_altitude(spk, eop, obs, t).map(|a| a - alt);
    let mut rises = Vec::new();
    let mut sets = Vec::new();
    let step = 20 * 60 * NS_PER_SEC;
    let mut a = from;
    let mut fa = f(a)?;
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let fb = f(b)?;
        if (fa > 0.0) != (fb > 0.0) {
            let root = find_root(|t| f(t).unwrap_or(f64::NAN), a, b, tol_ns)?;
            if fb > 0.0 {
                rises.push(root.t);
            } else {
                sets.push(root.t);
            }
        }
        a = b;
        fa = fb;
    }
    Ok((rises, sets))
}

/// The day's frame: sunrise, sunset and next sunrise around `at` (the civil day is the
/// one whose sunrise precedes `at`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolarDay {
    /// Sunrise that began the day.
    pub sunrise: Instant,
    /// Sunset of the day.
    pub sunset: Instant,
    /// Next sunrise.
    pub next_sunrise: Instant,
}

/// Find the solar day containing `at`.
pub fn solar_day(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
) -> Result<Option<SolarDay>, EventError> {
    let from = at.add_ns(-36 * 3_600 * NS_PER_SEC);
    let to = at.add_ns(36 * 3_600 * NS_PER_SEC);
    let (rises, sets) = sun_rise_set(spk, eop, obs, from, to, 1_000_000)?;
    let sunrise = rises.iter().copied().filter(|r| *r <= at).max();
    let Some(sunrise) = sunrise else {
        return Ok(None);
    };
    let sunset = sets.iter().copied().find(|s| *s > sunrise);
    let next_sunrise = rises.iter().copied().find(|r| *r > sunrise);
    match (sunset, next_sunrise) {
        (Some(sunset), Some(next_sunrise)) => Ok(Some(SolarDay {
            sunrise,
            sunset,
            next_sunrise,
        })),
        _ => Ok(None),
    }
}

const HORA_ORDER: [&str; 7] = [
    "Saturn", "Jupiter", "Mars", "Sun", "Venus", "Mercury", "Moon",
];

/// Weekday lord index into the Chaldean order for a weekday (0 = Sunday).
fn day_lord_index(weekday: u32) -> usize {
    // Sunday→Sun(3), Monday→Moon(6), Tuesday→Mars(2), Wednesday→Mercury(5), Thursday→Jupiter(1), Friday→Venus(4), Saturday→Saturn(0)
    [3, 6, 2, 5, 1, 4, 0][weekday as usize % 7]
}

/// Weekday (0 = Sunday) of the local civil date at sunrise, by longitude offset.
fn weekday_at(sunrise: Instant, obs: &Observer) -> u32 {
    let local = sunrise.add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    let u = Utc::from_instant(local);
    ((u.utc_day() + 4).rem_euclid(7)) as u32
}

/// The planetary hour at `at`: lord name, start and end. Day horas divide sunrise to
/// sunset into twelve, night horas sunset to next sunrise; the first hora of the day
/// belongs to the day lord and they follow the Chaldean order.
pub fn hora_at(day: &SolarDay, obs: &Observer, at: Instant) -> (&'static str, Instant, Instant) {
    let wd = weekday_at(day.sunrise, obs);
    let first = day_lord_index(wd);
    let (start, span, offset) = if at < day.sunset {
        (day.sunrise, day.sunset.ns_since(day.sunrise), 0)
    } else {
        (day.sunset, day.next_sunrise.ns_since(day.sunset), 12)
    };
    let len = span / 12;
    let idx = (at.ns_since(start) / len).clamp(0, 11) as usize;
    let lord = HORA_ORDER[(first + offset + idx) % 7];
    (
        lord,
        start.add_ns(idx as i128 * len),
        start.add_ns((idx as i128 + 1) * len),
    )
}

/// Rahu kala: one eighth of the daytime, at the slot fixed by the weekday
/// (Mon 2nd, Tue 7th, Wed 5th, Thu 6th, Fri 4th, Sat 3rd, Sun 8th).
pub fn rahu_kala(day: &SolarDay, obs: &Observer) -> (Instant, Instant) {
    let wd = weekday_at(day.sunrise, obs);
    let slot = [8usize, 2, 7, 5, 6, 4, 3][wd as usize % 7] - 1;
    let len = day.sunset.ns_since(day.sunrise) / 8;
    (
        day.sunrise.add_ns(slot as i128 * len),
        day.sunrise.add_ns((slot as i128 + 1) * len),
    )
}

/// Vara (weekday) name whose day runs sunrise to sunrise.
pub fn vara_name(day: &SolarDay, obs: &Observer) -> &'static str {
    [
        "Ravivara",
        "Somavara",
        "Mangalavara",
        "Budhavara",
        "Guruvara",
        "Shukravara",
        "Shanivara",
    ][weekday_at(day.sunrise, obs) as usize % 7]
}

#[cfg(test)]
mod solar_tests {
    use super::*;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn kochi_sunrise_and_sunset_on_the_equinox() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        // 2026-03-20 equinox at Kochi. Solar noon ≈ 12:32 IST (equation of time −7.5 min),
        // half-day with refraction ≈ 6h03m: sunrise ≈ 06:29 IST (00:59 UTC), sunset ≈ 18:36 IST (13:06 UTC).
        let at = Utc::new(2026, 3, 20, 6, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let day = solar_day(&spk, None, &obs, at).unwrap().unwrap();
        let rise = Utc::from_instant(day.sunrise);
        let set = Utc::from_instant(day.sunset);
        let rise_min = rise.hour as i32 * 60 + rise.minute as i32;
        let set_min = set.hour as i32 * 60 + set.minute as i32;
        assert_eq!((rise.month, rise.day), (3, 20), "{rise:?}");
        assert!((rise_min - 59).abs() <= 3, "{rise:?}");
        assert!((set_min - (13 * 60 + 6)).abs() <= 3, "{set:?}");
        let day_len_h = day.sunset.ns_since(day.sunrise) as f64 / 3.6e12;
        assert!((day_len_h - 12.1).abs() < 0.2, "{day_len_h}");
        // Friday 2026-03-20: first hora Venus; Rahu kala 4th slot
        assert_eq!(vara_name(&day, &obs), "Shukravara");
        let (lord, s, e) = hora_at(&day, &obs, day.sunrise.add_ns(1_000));
        assert_eq!(lord, "Venus");
        assert!(s <= day.sunrise.add_ns(1_000) && e > s);
        let (rs, re) = rahu_kala(&day, &obs);
        let frac = rs.ns_since(day.sunrise) as f64 / day.sunset.ns_since(day.sunrise) as f64;
        assert!((frac - 3.0 / 8.0).abs() < 1e-9);
        assert!(re > rs);
    }

    #[test]
    fn sun_altitude_is_negative_at_midnight_positive_at_noon() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let noon = Utc::new(2026, 10, 15, 6, 50, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap(); // ~12:20 IST local noon
        let midnight = Utc::new(2026, 10, 15, 18, 50, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        assert!(sun_altitude(&spk, None, &obs, noon).unwrap() > 70.0);
        assert!(sun_altitude(&spk, None, &obs, midnight).unwrap() < -60.0);
    }
}
