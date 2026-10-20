//! The sky the calendar is built on: rising and setting of the Sun and Moon at a
//! chosen altitude, the Sun's sidereal ingresses (sankrantis), and the tithi and
//! nakshatra spans.
//!
//! A year of a calendar needs about 750 sunrises and sunsets, 470 tithi and 450
//! nakshatra boundaries; scanning for each in short steps (as `kshana-events` does for
//! general searches) costs seconds. Every event here is instead sought in a narrow
//! bracket predicted from the previous one, and solved by the same Brent root-finder
//! to the same tolerance, so the instants agree with the scanning searches (pinned in
//! the tests below) at a fraction of the cost.

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{EventError, RiseConvention, MICROSECOND_NS};
use kshana_frames::{
    earth_rotation_angle, local_sidereal_time, mean_obliquity_iau2006, nutation_cached,
};
use kshana_sky::{apparent, apparent_longitude, Observer};
use kshana_solver::{find_root, wrap180};
use kshana_time::{delta_t_for_instant, ut1_jd_from_delta_t, EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa, NAKSHATRA_SPAN};

/// Equatorial Earth radius, km (IERS Conventions 2010, Table 1.1).
const EARTH_RADIUS_KM: f64 = 6_378.136_6;

/// Greenwich apparent sidereal time (radians), equinox-based IAU 2006 form (IERS
/// Conventions 2010, eqs. 5.30–5.32): GAST = ERA + the GMST polynomial + the equation
/// of the equinoxes Δψ·cos ε_A + its two largest complementary terms (0.002 640 96″
/// sin Ω + 0.000 063 52″ sin 2Ω; the rest are under 12 µas). It needs only the Δψ that
/// the interpolating nutation cache already holds, where `kshana_frames::gast` (the
/// CIO-based form) evaluates the full nutation and CIO-locator series on every call;
/// the two agree to a fraction of a milliarcsecond (pinned in the tests).
fn gast_fast(era: f64, tc: f64) -> f64 {
    const AS: f64 = std::f64::consts::PI / 648_000.0;
    let poly = 0.014_506
        + tc * (4_612.156_534 + tc * (1.391_581_7 + tc * (-0.000_000_44 + tc * (-0.000_029_956 + tc * -0.000_000_036_8))));
    let (dpsi, _) = nutation_cached(tc);
    // Mean longitude of the Moon's node (IERS Conventions 2010 eq. 5.43).
    let om = (450_160.398_036 - 6_962_890.543_1 * tc) * AS;
    let ee = dpsi * mean_obliquity_iau2006(tc).cos() + (0.002_640_96 * om.sin() + 0.000_063_52 * (2.0 * om).sin()) * AS;
    (era + poly * AS + ee).rem_euclid(std::f64::consts::TAU)
}

/// Geocentric altitude (degrees, no refraction) of a direction given by apparent
/// ecliptic longitude and latitude of date: the reduction of
/// `kshana_events::sun_altitude` (ecliptic of date → equator of date → hour angle from
/// GAST with UT1).
fn altitude_of(eop: Option<&EopTable>, obs: &Observer, t: Instant, lon: f64, lat: f64) -> f64 {
    let tc = t.tt_centuries_since_j2000();
    // The interpolated series (under 1e-7″ from the full IAU 2006A) that the apparent
    // position already filled for this instant.
    let (_, deps) = nutation_cached(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let (sl, cl) = lon.to_radians().sin_cos();
    let (sb, cb) = lat.to_radians().sin_cos();
    let (se, ce) = eps.sin_cos();
    let x = cb * cl;
    let y = cb * sl * ce - sb * se;
    let z = cb * sl * se + sb * ce;
    let ra = y.atan2(x);
    let dec = z.asin();
    let utc = Utc::from_instant(t);
    let mjd = utc.utc_day() as f64
        + 40_587.0
        + (utc.hour as f64 * 3_600.0 + utc.minute as f64 * 60.0 + utc.second as f64 + utc.nanos as f64 * 1e-9)
            / 86_400.0;
    let ut1 = match eop.and_then(|e| e.ut1_minus_utc(mjd)) {
        Some(u) => kshana_time::TwoPartJd {
            jd1: 2_400_000.5 + mjd.floor(),
            jd2: mjd.fract() + u.ut1_minus_utc / 86_400.0,
        },
        None => ut1_jd_from_delta_t(t, delta_t_for_instant(t)),
    };
    let lst = local_sidereal_time(gast_fast(earth_rotation_angle(ut1), tc), obs.longitude_deg.to_radians());
    let h = lst - ra;
    let phi = obs.latitude_deg.to_radians();
    let sin_alt = phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos();
    sin_alt.asin().to_degrees()
}

/// Geocentric apparent altitude (degrees, no refraction) of `body`'s centre and its
/// horizontal parallax (degrees).
///
/// For the Sun the ecliptic latitude is taken as zero: it never exceeds 1.2″ (Meeus,
/// *Astronomical Algorithms*, 2nd ed., ch. 25), which moves a sunrise by under 0.1 s,
/// and it lets the cheaper longitude-only position be used.
pub fn altitude(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    body: Body,
    t: Instant,
) -> Result<(f64, f64), SpkError> {
    if body == Body::Sun {
        let lon = apparent_longitude(spk, Body::Sun, t)?;
        return Ok((altitude_of(eop, obs, t, lon, 0.0), 0.0));
    }
    let a = apparent(spk, body, t)?;
    let parallax = (EARTH_RADIUS_KM / a.distance_km).clamp(-1.0, 1.0).asin().to_degrees();
    Ok((altitude_of(eop, obs, t, a.longitude, a.latitude), parallax))
}

/// The geocentric altitude of the Moon's centre at which it rises or sets.
///
/// Upper limb with refraction: h₀ = 0.7275·π − 0°34′ (Meeus, *Astronomical Algorithms*,
/// 2nd ed., ch. 15, the Moon's standard altitude). Centre on the geometric horizon (the
/// Hindu rising): the topocentric centre at 0°, i.e. a geocentric altitude of π.
pub fn moon_rise_altitude(rise: RiseConvention, parallax_deg: f64) -> f64 {
    match rise {
        RiseConvention::UpperLimbRefracted => 0.7275 * parallax_deg - 34.0 / 60.0,
        RiseConvention::CentreGeometric => parallax_deg,
    }
}

/// Rises and sets of `body` in `[from, to]` under a rising convention, as
/// `(rises, sets)`, each solved to a millisecond.
pub fn rise_set(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    body: Body,
    rise: RiseConvention,
    from: Instant,
    to: Instant,
) -> Result<(Vec<Instant>, Vec<Instant>), EventError> {
    let f = |t: Instant| -> Result<f64, SpkError> {
        let (alt, p) = altitude(spk, eop, obs, body, t)?;
        Ok(match body {
            Body::Moon => alt - moon_rise_altitude(rise, p),
            _ => alt - rise.altitude_deg(),
        })
    };
    if body == Body::Sun {
        if let Some(found) = predicted(&f, from, to)? {
            return Ok(found);
        }
    }
    scan(&f, from, to)
}

/// Rises and sets by a 20-minute scan: safe anywhere the body crosses the horizon
/// at most once in 20 minutes.
fn scan<F: Fn(Instant) -> Result<f64, SpkError>>(
    f: &F,
    from: Instant,
    to: Instant,
) -> Result<(Vec<Instant>, Vec<Instant>), EventError> {
    let step = 20 * 60 * NS_PER_SEC;
    let (mut rises, mut sets) = (Vec::new(), Vec::new());
    let mut a = from;
    let mut fa = f(a)?;
    while a < to {
        let b = if a.add_ns(step) > to { to } else { a.add_ns(step) };
        let fb = f(b)?;
        if (fa > 0.0) != (fb > 0.0) {
            let root = find_root(|t| f(t).unwrap_or(f64::NAN), a, b, 1_000_000)?;
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

/// Sunrises and sunsets found day by day from the first ones (found by the scan):
/// each is sought in a bracket of ±6 minutes about the previous one plus the previous
/// day's length, widened once to ±40 minutes. Returns `None`, so that the caller scans
/// the whole range, whenever a bracket does not hold a crossing of the right
/// direction (polar day or night, or a range too short to seed).
fn predicted<F: Fn(Instant) -> Result<f64, SpkError>>(
    f: &F,
    from: Instant,
    to: Instant,
) -> Result<Option<(Vec<Instant>, Vec<Instant>)>, EventError> {
    let seed_end = from.add_ns(26 * 3_600 * NS_PER_SEC);
    if seed_end >= to {
        return Ok(None);
    }
    let (r0, s0) = scan(f, from, seed_end)?;
    let (Some(&r), Some(&s)) = (r0.first(), s0.first()) else { return Ok(None) };
    let mut out = (vec![r], vec![s]);
    for (list, rising) in [(&mut out.0, true), (&mut out.1, false)] {
        loop {
            let last = *list.last().expect("seeded");
            let day = if list.len() >= 2 { last.ns_since(list[list.len() - 2]) } else { NS_PER_DAY };
            let guess = last.add_ns(day);
            let mut found = None;
            for half in [6 * 60 * NS_PER_SEC, 40 * 60 * NS_PER_SEC] {
                let (a, b) = (guess.add_ns(-half), guess.add_ns(half));
                if b > to {
                    // The next event may lie beyond the range: settle it by a scan of
                    // the tail, which is shorter than a day and a bracket.
                    let (rr, ss) = scan(f, last.add_ns(NS_PER_SEC), to)?;
                    let tail = if rising { rr } else { ss };
                    if tail.len() > 1 {
                        return Ok(None);
                    }
                    list.extend(tail);
                    found = Some(None);
                    break;
                }
                let (fa, fb) = (f(a)?, f(b)?);
                if (fa > 0.0) != (fb > 0.0) && (fb > 0.0) == rising {
                    found = Some(Some(find_root(|t| f(t).unwrap_or(f64::NAN), a, b, 1_000_000)?.t));
                    break;
                }
            }
            match found {
                Some(Some(t)) => list.push(t),
                Some(None) => break,
                None => return Ok(None),
            }
        }
    }
    Ok(Some(out))
}

/// Sidereal longitude of the Sun (degrees).
pub fn sun_sidereal(spk: &Spk, ayanamsa: Ayanamsa, t: Instant) -> Result<f64, SpkError> {
    let s = apparent_longitude(spk, Body::Sun, t)?;
    Ok(sidereal(s, ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000())))
}

/// Tropical apparent longitude of the Sun (degrees).
pub fn sun_tropical(spk: &Spk, t: Instant) -> Result<f64, SpkError> {
    apparent_longitude(spk, Body::Sun, t)
}

/// Sidereal longitude of the Moon (degrees).
pub fn moon_sidereal(spk: &Spk, ayanamsa: Ayanamsa, t: Instant) -> Result<f64, SpkError> {
    let m = apparent_longitude(spk, Body::Moon, t)?;
    Ok(sidereal(m, ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000())))
}

/// Moon − Sun elongation (degrees, `[0, 360)`).
pub fn elongation(spk: &Spk, t: Instant) -> Result<f64, SpkError> {
    let m = apparent_longitude(spk, Body::Moon, t)?;
    let s = apparent_longitude(spk, Body::Sun, t)?;
    Ok((m - s).rem_euclid(360.0))
}

/// Tithi (1–30) at `t` from the apparent Moon − Sun elongation.
pub fn tithi_at(spk: &Spk, t: Instant) -> Result<u8, SpkError> {
    Ok(((elongation(spk, t)? / 12.0).floor() as u8).min(29) + 1)
}

/// Nakshatra (0–26) of the Moon at `t`.
pub fn star_at(spk: &Spk, ayanamsa: Ayanamsa, t: Instant) -> Result<u8, SpkError> {
    Ok(kshana_vedic::nakshatra(moon_sidereal(spk, ayanamsa, t)?).index as u8)
}

/// One sankranti: the Sun enters sidereal sign `sign` (0 = Mesha) at `at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sankranti {
    /// Sign entered, 0 = Mesha.
    pub sign: u8,
    /// The instant.
    pub at: Instant,
}

/// Instants in `[from, to]` at which an increasing phase (degrees mod 360) crosses a
/// multiple of `span`, each with the multiple's index (0-based, mod 360/span).
/// `mean_rate` (degrees a day) seeds the first prediction; later ones use the last
/// interval. Each crossing is bracketed at ±3 h about the prediction, widened to ±1 and
/// ±4 days, then solved to a microsecond.
fn crossings<P: Fn(Instant) -> Result<f64, SpkError>>(
    phase: P,
    span: f64,
    mean_rate: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<(Instant, u32)>, EventError> {
    let n = (360.0 / span).round() as u32;
    let p0 = phase(from)?;
    let mut k = ((p0 / span).floor() as u32 + 1) % n;
    let days = |deg: f64, rate: f64| (deg / rate * NS_PER_DAY as f64) as i128;
    let mut guess = from.add_ns(days((k as f64 * span - p0).rem_euclid(360.0), mean_rate));
    let mut out: Vec<(Instant, u32)> = Vec::new();
    loop {
        let target = k as f64 * span;
        let g = |t: Instant| phase(t).map(|p| wrap180(p - target)).unwrap_or(f64::NAN);
        let mut root = None;
        for half in [3 * 3_600 * NS_PER_SEC, NS_PER_DAY, 4 * NS_PER_DAY] {
            let (a, b) = (guess.add_ns(-half), guess.add_ns(half));
            let (ga, gb) = (g(a), g(b));
            if ga.is_nan() || gb.is_nan() {
                // surface the ephemeris error
                phase(a)?;
                phase(b)?;
            }
            if ga < 0.0 && gb >= 0.0 {
                root = Some(find_root(g, a, b, MICROSECOND_NS)?.t);
                break;
            }
        }
        let Some(t) = root else {
            return Err(EventError::Solver(kshana_solver::SolverError::NotBracketed));
        };
        if t > to {
            break;
        }
        if t >= from && out.last().map_or(true, |l| t > l.0) {
            out.push((t, k));
        }
        let step = match out.len() {
            n if n >= 2 => out[n - 1].0.ns_since(out[n - 2].0),
            _ => days(span, mean_rate),
        };
        guess = t.add_ns(step);
        k = (k + 1) % n;
    }
    Ok(out)
}

/// Sankrantis in `[from, to]`, each to a microsecond.
pub fn sankrantis(spk: &Spk, ayanamsa: Ayanamsa, from: Instant, to: Instant) -> Result<Vec<Sankranti>, EventError> {
    // The Sun's mean sidereal motion, 360° in 365.256 days.
    let c = crossings(|t| sun_sidereal(spk, ayanamsa, t), 30.0, 0.9856, from, to)?;
    Ok(c.into_iter().map(|(at, k)| Sankranti { sign: k as u8, at }).collect())
}

/// A tithi or nakshatra and the instants it runs between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Tithi 1–30, or nakshatra 0–26.
    pub index: u8,
    /// Its start.
    pub start: Instant,
    /// Its end.
    pub end: Instant,
}

impl Span {
    /// Length of the overlap with `[a, b]`, ns (0 when disjoint).
    pub fn overlap(&self, a: Instant, b: Instant) -> i128 {
        let s = if self.start > a { self.start } else { a };
        let e = if self.end < b { self.end } else { b };
        e.ns_since(s).max(0)
    }

    /// True when `t` falls inside.
    pub fn contains(&self, t: Instant) -> bool {
        self.start <= t && t < self.end
    }
}

fn spans(c: &[(Instant, u32)], index: impl Fn(u32) -> u8) -> Vec<Span> {
    c.windows(2).map(|w| Span { index: index(w[0].1), start: w[0].0, end: w[1].0 }).collect()
}

/// Tithi spans fully inside `[from, to]`. The span that begins at a boundary of 12·k
/// degrees is tithi k + 1; the starts of tithi 1 are the new Moons.
pub fn tithi_spans(spk: &Spk, from: Instant, to: Instant) -> Result<Vec<Span>, EventError> {
    // Mean elongation rate: 360° in a synodic month of 29.5306 days.
    let c = crossings(|t| elongation(spk, t), 12.0, 12.190_7, from, to)?;
    Ok(spans(&c, |k| k as u8 + 1))
}

/// New Moons (Moon − Sun elongation 0°, the starts of tithi 1) in `[from, to]`.
pub fn new_moons(spk: &Spk, from: Instant, to: Instant) -> Result<Vec<Instant>, EventError> {
    Ok(crossings(|t| elongation(spk, t), 360.0, 12.190_7, from, to)?.into_iter().map(|c| c.0).collect())
}

/// Nakshatra spans of the Moon fully inside `[from, to]`.
pub fn star_spans(spk: &Spk, ayanamsa: Ayanamsa, from: Instant, to: Instant) -> Result<Vec<Span>, EventError> {
    // Mean sidereal motion of the Moon: 360° in 27.3217 days.
    let c = crossings(|t| moon_sidereal(spk, ayanamsa, t), NAKSHATRA_SPAN, 13.176_4, from, to)?;
    Ok(spans(&c, |k| k as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;
    use kshana_frames::gast;

    fn utc(y: i64, m: u32, d: u32) -> Instant {
        Utc::new(y, m, d, 0, 0, 0, 0).unwrap().to_instant().unwrap()
    }

    #[test]
    fn equinox_based_gast_matches_the_cio_based_one() {
        for k in 0..200 {
            let tc = -0.3 + k as f64 * 0.004; // 1970–2050
            let era = k as f64 * 0.37;
            let d = wrap180((gast_fast(era, tc) - gast(era, tc)).to_degrees()) * 3.6e6; // mas
            assert!(d.abs() < 0.5, "{tc}: {d} mas");
        }
    }

    #[test]
    fn predicted_boundaries_agree_with_the_scanning_search() {
        let Some(spk) = kernel() else { return };
        let (a, b) = (utc(2025, 9, 1), utc(2025, 11, 1));
        let ours = tithi_spans(&spk, a, b).unwrap();
        let theirs = kshana_events::tithi_boundaries(&spk, a, b, MICROSECOND_NS).unwrap();
        // Our spans run from the first boundary to the last.
        assert_eq!(ours.len() + 1, theirs.len());
        for (s, e) in ours.iter().zip(&theirs) {
            assert!(s.start.ns_since(e.root.t).abs() <= 2 * MICROSECOND_NS);
            assert_eq!(s.index, tithi_at(&spk, s.start.add_ns(NS_PER_SEC)).unwrap());
        }
        let ours = star_spans(&spk, Ayanamsa::Lahiri, a, b).unwrap();
        let theirs = kshana_events::nakshatra_boundaries(&spk, Body::Moon, Ayanamsa::Lahiri, a, b, MICROSECOND_NS).unwrap();
        assert_eq!(ours.len() + 1, theirs.len());
        for (s, e) in ours.iter().zip(&theirs) {
            assert!(s.start.ns_since(e.root.t).abs() <= 2 * MICROSECOND_NS);
            assert_eq!(s.index, star_at(&spk, Ayanamsa::Lahiri, s.start.add_ns(NS_PER_SEC)).unwrap());
        }
        let nm = kshana_events::new_moons(&spk, a, b, MICROSECOND_NS).unwrap();
        let ones: Vec<Instant> = tithi_spans(&spk, a, b).unwrap().iter().filter(|s| s.index == 1).map(|s| s.start).collect();
        assert_eq!(ones.len(), nm.len());
        for (x, e) in ones.iter().zip(&nm) {
            assert!(x.ns_since(e.root.t).abs() <= 2 * MICROSECOND_NS);
        }
    }

    #[test]
    fn predicted_sunrises_agree_with_the_event_search() {
        let Some(spk) = kernel() else { return };
        let (a, b) = (utc(2025, 3, 1), utc(2025, 4, 15));
        kshana_events::set_rise_convention(RiseConvention::UpperLimbRefracted);
        for obs in [DELHI, KOCHI, Observer { latitude_deg: 64.0, longitude_deg: -21.0, height_m: 0.0 }] {
            let (r, s) = rise_set(&spk, None, &obs, Body::Sun, RiseConvention::UpperLimbRefracted, a, b).unwrap();
            let (er, es) = kshana_events::sun_rise_set(&spk, None, &obs, a, b, 1_000_000).unwrap();
            assert_eq!((r.len(), s.len()), (er.len(), es.len()));
            // The Sun's ecliptic latitude (≤ 1.2″) is dropped here: well under a second.
            for (x, y) in r.iter().zip(&er).chain(s.iter().zip(&es)) {
                assert!(x.ns_since(*y).abs() < 300_000_000, "{}", x.ns_since(*y));
            }
        }
    }

    #[test]
    fn sankrantis_of_a_year() {
        let Some(spk) = kernel() else { return };
        let s = sankrantis(&spk, Ayanamsa::Lahiri, utc(2025, 1, 1), utc(2026, 1, 1)).unwrap();
        assert_eq!(s.len(), 12);
        // Makara 14 Jan 2025 and Mesha 14 Apr 2025, mornings IST.
        assert_eq!(s[0].sign, 9);
        let ist = |t: Instant| Utc::from_instant(t.add_ns(330 * 60 * NS_PER_SEC));
        assert_eq!((ist(s[0].at).month, ist(s[0].at).day), (1, 14));
        let mesha = s.iter().find(|x| x.sign == 0).unwrap();
        assert_eq!((ist(mesha.at).month, ist(mesha.at).day), (4, 14));
        for x in &s {
            let l = sun_sidereal(&spk, Ayanamsa::Lahiri, x.at).unwrap();
            assert!(wrap180(l - x.sign as f64 * 30.0).abs() < 1e-6);
        }
    }
}
