//! Transits over the natal chart, with exact instants.

use kshana_chart::{Chart, BODIES};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::MICROSECOND_NS;
use kshana_sky::apparent;
use kshana_solver::wrap180;
use kshana_time::{Instant, NS_PER_DAY};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

/// What kind of transit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitKind {
    /// Transiting body in the same sidereal sign as a natal body.
    OverNatal,
    /// Transiting body in a Parashari aspect sign of a natal body (7th; Mars 4/8; Jupiter 5/9; Saturn 3/10).
    AspectingNatal,
    /// Transiting body in a sidereal house counted from the natal Moon.
    FromMoon,
}

/// One transit relationship in force.
#[derive(Clone, Debug, PartialEq)]
pub struct Transit {
    /// Transiting body.
    pub body: &'static str,
    /// Natal point ("Moon", "Sun", … or "Lagna").
    pub natal: &'static str,
    /// Kind.
    pub kind: TransitKind,
    /// House from the natal point (1–12) in the sidereal zodiac.
    pub house: u8,
    /// Transiting sidereal longitude.
    pub longitude: f64,
    /// Exact separation from the natal point in degrees (0 = conjunction), for `OverNatal`.
    pub separation: f64,
}

fn sign(l: f64) -> i64 {
    (l.rem_euclid(360.0) / 30.0).floor() as i64
}

fn aspect_offsets(body: &str) -> &'static [i64] {
    match body {
        "Mars" => &[4, 7, 8],
        "Jupiter" => &[5, 7, 9],
        "Saturn" => &[3, 7, 10],
        _ => &[7],
    }
}

/// Transits in force at `at` for the natal chart (sidereal, sign-based, the Jyotisha
/// gochara convention), for the seven classical grahas.
pub fn transits_at(
    spk: &Spk,
    natal: &Chart,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<Vec<Transit>, SpkError> {
    let ay = ayanamsa_degrees(ayanamsa, at.tt_centuries_since_j2000());
    let mut out = Vec::new();
    let natal_points: Vec<(&'static str, f64)> = natal
        .bodies
        .iter()
        .take(7)
        .map(|b| (b.name, b.sidereal))
        .chain(std::iter::once(("Lagna", natal.cusps_sidereal[0])))
        .collect();
    for (name, body) in BODIES.iter().take(7) {
        let t = sidereal(apparent(spk, *body, at)?.longitude, ay);
        for (pname, plon) in &natal_points {
            let house = ((sign(t) - sign(*plon)).rem_euclid(12) + 1) as u8;
            if house == 1 {
                out.push(Transit {
                    body: name,
                    natal: pname,
                    kind: TransitKind::OverNatal,
                    house,
                    longitude: t,
                    separation: wrap180(t - plon).abs(),
                });
            } else if aspect_offsets(name).contains(&(house as i64)) {
                out.push(Transit {
                    body: name,
                    natal: pname,
                    kind: TransitKind::AspectingNatal,
                    house,
                    longitude: t,
                    separation: wrap180(t - plon).abs(),
                });
            }
            if *pname == "Moon" && house != 1 {
                out.push(Transit {
                    body: name,
                    natal: "Moon",
                    kind: TransitKind::FromMoon,
                    house,
                    longitude: t,
                    separation: wrap180(t - plon).abs(),
                });
            }
        }
    }
    Ok(out)
}

/// Sade Sati: Saturn's transit through the 12th, 1st and 2nd signs from the natal Moon.
#[derive(Clone, Debug, PartialEq)]
pub struct SadeSati {
    /// Phase: 0 none, 1 rising (12th), 2 peak (1st), 3 setting (2nd).
    pub phase: u8,
    /// Saturn's sidereal sign now.
    pub saturn_sign: u8,
    /// Natal Moon sign.
    pub moon_sign: u8,
    /// Exact start of the current phase, if within the searched span.
    pub phase_start: Option<Instant>,
    /// Exact end of the current phase (Saturn's next ingress), if within the searched span.
    pub phase_end: Option<Instant>,
    /// Exact start of the next Sade Sati (Saturn entering the 12th from the Moon), if not in one now.
    pub next_start: Option<Instant>,
}

/// Sade Sati status at `at`, with exact ingress instants searched ±3 years (phase
/// boundaries) and +30 years (next start).
pub fn sade_sati(
    spk: &Spk,
    natal: &Chart,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<SadeSati, SpkError> {
    let ay = ayanamsa_degrees(ayanamsa, at.tt_centuries_since_j2000());
    let moon_sign = sign(natal.bodies[1].sidereal) as u8;
    let sat = sidereal(apparent(spk, Body::SaturnBarycenter, at)?.longitude, ay);
    let saturn_sign = sign(sat) as u8;
    let house = ((saturn_sign as i64 - moon_sign as i64).rem_euclid(12) + 1) as u8;
    let phase = match house {
        12 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    };
    // Saturn's sidereal ingresses: tropical ingresses of (sign boundary + ayanamsa); use the
    // longitude-crossing search on the sidereal boundary converted to tropical at each epoch.
    let span_back = at.add_ns(-3 * 365 * NS_PER_DAY);
    let span_fwd = at.add_ns(3 * 365 * NS_PER_DAY);
    let boundaries =
        sidereal_ingresses(spk, Body::SaturnBarycenter, ayanamsa, span_back, span_fwd)?;
    let phase_start = boundaries
        .iter()
        .rev()
        .find(|(t, _)| *t <= at)
        .map(|(t, _)| *t);
    let phase_end = boundaries.iter().find(|(t, _)| *t > at).map(|(t, _)| *t);
    let next_start = if phase == 0 {
        let far = at.add_ns(30 * 365 * NS_PER_DAY);
        let target = (moon_sign as i64 + 11).rem_euclid(12) as u8;
        sidereal_ingresses(spk, Body::SaturnBarycenter, ayanamsa, at, far)?
            .into_iter()
            .find(|(_, s)| *s == target)
            .map(|(t, _)| t)
    } else {
        None
    };
    Ok(SadeSati {
        phase,
        saturn_sign,
        moon_sign,
        phase_start,
        phase_end,
        next_start,
    })
}

/// Instants when `body` enters a sidereal sign in `[from, to]`, with the sign entered.
pub fn sidereal_ingresses(
    spk: &Spk,
    body: Body,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<(Instant, u8)>, SpkError> {
    // A sidereal boundary k·30° is the tropical longitude k·30° + ayanamsa(t); the
    // ayanamsa moves 50″/yr, so bracket on the tropical ingress of the shifted boundary.
    let mut out = Vec::new();
    let step = 10 * NS_PER_DAY;
    let mut a = from;
    let lon_sid = |t: Instant| -> Result<f64, SpkError> {
        let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
        Ok(sidereal(apparent(spk, body, t)?.longitude, ay))
    };
    let mut la = lon_sid(a)?;
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let lb = lon_sid(b)?;
        if sign(la) != sign(lb) {
            let entering = if wrap180(lb - la) > 0.0 {
                sign(lb)
            } else {
                sign(la)
            };
            let boundary = entering as f64 * 30.0;
            let f = |t: Instant| {
                lon_sid(t)
                    .map(|l| wrap180(l - boundary))
                    .unwrap_or(f64::NAN)
            };
            if let Ok(root) = kshana_solver::find_root(f, a, b, MICROSECOND_NS) {
                let s = sign(lon_sid(root.t.add_ns(NS_PER_DAY))?) as u8;
                out.push((root.t, s));
            }
        }
        a = b;
        la = lb;
    }
    Ok(out)
}

/// The next sign changes of the slow planets (Jupiter, Saturn, Rahu by mean motion is
/// omitted) and of the Sun and Moon in `days` days, sidereal, as (instant, body, sign).
pub fn upcoming_transits(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    from: Instant,
    days: i64,
) -> Result<Vec<(Instant, &'static str, u8)>, SpkError> {
    let to = from.add_ns(days as i128 * NS_PER_DAY);
    let mut out = Vec::new();
    for (name, body) in [
        ("Sun", Body::Sun),
        ("Moon", Body::Moon),
        ("Mars", Body::MarsBarycenter),
        ("Mercury", Body::Mercury),
        ("Venus", Body::Venus),
        ("Jupiter", Body::JupiterBarycenter),
        ("Saturn", Body::SaturnBarycenter),
    ] {
        for (t, s) in sidereal_ingresses(spk, body, ayanamsa, from, to)? {
            out.push((t, name, s));
        }
    }
    out.sort_by_key(|x| x.0);
    Ok(out)
}
