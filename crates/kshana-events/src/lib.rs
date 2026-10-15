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
use kshana_time::{Instant, NS_PER_SEC};
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
