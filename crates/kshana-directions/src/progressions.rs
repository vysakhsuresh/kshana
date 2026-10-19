//! Secondary progressions: a day after birth stands for a year of life.
//!
//! The progressed sky is the real sky at the *progressed instant*, so every progressed
//! position is an ephemeris evaluation and every progressed contact is a solved root of
//! a function of real time. Nothing is scaled from a daily table.

use crate::{aspect_targets, require, scan_roots, DirectionsError};
use kshana_chart::{Chart, BODIES};
use kshana_ephem::{Body, Spk};
use kshana_events::MICROSECOND_NS;
use kshana_frames::{mean_obliquity_iau2006, nutation_iau2006a};
use kshana_houses::angles;
use kshana_sky::apparent;
use kshana_solver::{find_root, wrap180};
use kshana_time::{Instant, NS_PER_DAY};
use kshana_western::Aspect;

/// Days in the year of the day-for-a-year key: the mean tropical year, 365.2422 days.
///
/// Convention: one ephemeris day after birth corresponds to one tropical year of life,
/// so `progressed = birth + (at − birth) / 365.2422`. Other keys exist (the Julian year
/// of 365.25 days, or the "Q2" key that counts solar returns); Kshana uses the tropical
/// year because the solar return, not the calendar, closes a year of life. See
/// <https://www.augurine.com/tools/progressed-chart-calculator> for the day-for-a-year
/// rule and the choice of key.
pub const TROPICAL_YEAR_DAYS: f64 = 365.2422;

/// The same key as an exact integer ratio, so that the progressed instant is computed
/// without a float: real nanoseconds × 10 000 / 3 652 422.
const KEY_NUM: i128 = 10_000;
const KEY_DEN: i128 = 3_652_422;

/// Scan step in real time for progressed contacts: 30 days, in which the progressed
/// Moon (the fastest progressed body, about 13° a year) moves about 1°.
pub const PROGRESSED_SCAN_STEP_NS: i128 = 30 * NS_PER_DAY;

/// The ephemeris instant whose sky is the progressed sky for real instant `at`.
pub fn progressed_instant(birth: Instant, at: Instant) -> Instant {
    birth.add_ns(at.ns_since(birth) * KEY_NUM / KEY_DEN)
}

/// Inverse of [`progressed_instant`]: the real instant at which the progressed sky is
/// the real sky of `progressed`.
pub fn real_instant(birth: Instant, progressed: Instant) -> Instant {
    birth.add_ns(progressed.ns_since(birth) * KEY_DEN / KEY_NUM)
}

/// Tropical longitude of `body` in the progressed sky at real instant `at`.
pub fn progressed_longitude(
    spk: &Spk,
    birth: Instant,
    body: Body,
    at: Instant,
) -> Result<f64, DirectionsError> {
    Ok(apparent(spk, body, progressed_instant(birth, at))?.longitude)
}

/// One progressed body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressedBody {
    /// Chart name ("Sun", "Moon", ...).
    pub name: &'static str,
    /// Progressed tropical longitude of date in degrees.
    pub longitude: f64,
    /// Daily motion at the progressed instant, degrees per ephemeris day, which is
    /// degrees per year of life.
    pub speed: f64,
}

/// The progressed chart at one real instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Progressed {
    /// The real instant asked for.
    pub at: Instant,
    /// The ephemeris instant whose sky this is.
    pub instant: Instant,
    /// Progressed bodies in chart order.
    pub bodies: Vec<ProgressedBody>,
    /// Progressed Sun − natal Sun, `[0, 360)`.
    pub solar_arc: f64,
    /// Progressed Midheaven: natal MC + solar arc.
    pub mc: f64,
    /// Progressed Ascendant: the longitude rising when the progressed MC culminates at
    /// the natal latitude.
    pub asc: f64,
    /// Progressed Moon − progressed Sun, `[0, 360)`.
    pub moon_phase_deg: f64,
    /// Name of the progressed lunation phase, one of [`LUNATION_PHASES`].
    pub lunation_phase: &'static str,
}

/// Rudhyar's eight lunation phases, 45° each from the New Moon. Source: Dane Rudhyar,
/// *The Lunation Cycle*; the equal 45° division is summarised at
/// <https://www.augurine.com/learn/eight-moon-phases>.
pub const LUNATION_PHASES: [&str; 8] = [
    "New",
    "Crescent",
    "First Quarter",
    "Gibbous",
    "Full",
    "Disseminating",
    "Last Quarter",
    "Balsamic",
];

/// Phase name for a Moon − Sun elongation in degrees.
pub fn lunation_phase_name(phase_deg: f64) -> &'static str {
    LUNATION_PHASES[((phase_deg.rem_euclid(360.0) / 45.0).floor() as usize).min(7)]
}

/// True obliquity of the ecliptic (radians) at `t`.
fn obliquity_at(t: Instant) -> f64 {
    let tc = t.tt_centuries_since_j2000();
    let (_, deps) = nutation_iau2006a(tc);
    mean_obliquity_iau2006(tc) + deps
}

/// The Ascendant (degrees) rising when ecliptic longitude `mc_deg` culminates, for true
/// obliquity `eps` (radians) and geographic latitude `latitude_deg`.
///
/// The MC's right ascension is `atan2(sin MC · cos ε, cos MC)`, which is the local
/// sidereal time; the Ascendant follows from `kshana_houses::angles`.
pub fn ascendant_for_mc(mc_deg: f64, eps: f64, latitude_deg: f64) -> f64 {
    let mc = mc_deg.to_radians();
    let ramc = (mc.sin() * eps.cos()).atan2(mc.cos());
    angles(ramc, eps, latitude_deg.to_radians()).asc
}

/// The secondary progressed chart for real instant `at`.
///
/// Conventions:
/// * Bodies: the real sky at [`progressed_instant`] (day for a tropical year).
/// * Progressed MC: natal MC + solar arc (progressed Sun − natal Sun), the "solar arc
///   MC" of most software, as against the Naibod or quotidian keys; see
///   <https://kerykeion.net/content/learn-astrology/rectification-progressed-angles>.
/// * Progressed Ascendant: from the progressed MC with the obliquity of the progressed
///   instant and the natal latitude, via [`ascendant_for_mc`]. The chart does not carry
///   the birthplace, so `latitude_deg` is passed explicitly.
/// * Lunation phase: progressed Moon − progressed Sun in Rudhyar's eight 45° phases.
pub fn progressions(
    spk: &Spk,
    natal: &Chart,
    latitude_deg: f64,
    at: Instant,
) -> Result<Progressed, DirectionsError> {
    let natal_sun = require(natal, "Sun")?;
    let natal_mc = natal.cusps[9];
    let instant = progressed_instant(natal.instant, at);
    let mut bodies = Vec::with_capacity(BODIES.len());
    for (name, body) in BODIES {
        let a = apparent(spk, body, instant)?;
        bodies.push(ProgressedBody {
            name,
            longitude: a.longitude,
            speed: a.speed,
        });
    }
    let sun = bodies[0].longitude;
    let moon = bodies[1].longitude;
    let solar_arc = (sun - natal_sun).rem_euclid(360.0);
    let mc = (natal_mc + solar_arc).rem_euclid(360.0);
    let asc = ascendant_for_mc(mc, obliquity_at(instant), latitude_deg);
    let moon_phase_deg = (moon - sun).rem_euclid(360.0);
    Ok(Progressed {
        at,
        instant,
        bodies,
        solar_arc,
        mc,
        asc,
        moon_phase_deg,
        lunation_phase: lunation_phase_name(moon_phase_deg),
    })
}

/// Real instants in `[from, to]` at which progressed `body` makes `aspect` to the fixed
/// natal longitude `natal_point_longitude`, exact to a microsecond of real time.
///
/// Each is a root of `wrap180(prog_lon(t) − (natal ± angle))` in real time `t`, where
/// `prog_lon` evaluates the ephemeris at [`progressed_instant`]. Both sides of the angle
/// are solved for aspects other than the conjunction and the opposition. Sorted.
pub fn progressed_aspect_instants(
    spk: &Spk,
    natal: &Chart,
    body: Body,
    natal_point_longitude: f64,
    aspect: Aspect,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    let birth = natal.instant;
    let mut out = Vec::new();
    for target in aspect_targets(natal_point_longitude, aspect) {
        let f = |t: Instant| {
            apparent(spk, body, progressed_instant(birth, t)).map(|a| wrap180(a.longitude - target))
        };
        let roots = scan_roots(f, from, to, PROGRESSED_SCAN_STEP_NS, MICROSECOND_NS)?;
        out.extend(roots.into_iter().map(|r| r.t));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Real instants in `[from, to]` at which progressed `body` crosses a sign boundary,
/// in either direction, exact to a microsecond of real time.
pub fn progressed_ingresses(
    spk: &Spk,
    natal: &Chart,
    body: Body,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    let birth = natal.instant;
    let lon = |t: Instant| apparent(spk, body, progressed_instant(birth, t)).map(|a| a.longitude);
    let mut out = Vec::new();
    if to <= from {
        return Ok(out);
    }
    let mut a = from;
    let mut la = lon(a)?;
    while a < to {
        let b = crate::next_step(a, to, PROGRESSED_SCAN_STEP_NS);
        let lb = lon(b)?;
        let ka = (la / 30.0).floor();
        let kb = (lb / 30.0).floor();
        if ka != kb {
            // moving forward the boundary entered is the new sign's start; moving
            // backward it is the old sign's start
            let boundary = if wrap180(lb - la) > 0.0 {
                kb * 30.0
            } else {
                ka * 30.0
            };
            let g = |t: Instant| lon(t).map(|l| wrap180(l - boundary)).unwrap_or(f64::NAN);
            let root = find_root(g, a, b, MICROSECOND_NS)?;
            out.push(root.t);
        }
        a = b;
        la = lb;
    }
    Ok(out)
}

/// Sign ingresses of the progressed Moon, in real time; see [`progressed_ingresses`].
pub fn progressed_moon_ingresses(
    spk: &Spk,
    natal: &Chart,
    from: Instant,
    to: Instant,
) -> Result<Vec<Instant>, DirectionsError> {
    progressed_ingresses(spk, natal, Body::Moon, from, to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use kshana_time::NS_PER_SEC;

    const YEAR_NS: i128 = 365_242_200 * (NS_PER_SEC / 10_000) * 864;

    #[test]
    fn the_key_is_exact_integer_arithmetic() {
        let birth = Instant::j2000();
        // 365.2422 days of life is exactly one ephemeris day
        let a_year = birth.add_ns(YEAR_NS);
        assert_eq!(YEAR_NS, 31_556_926_080_000_000);
        assert_eq!(progressed_instant(birth, a_year), birth.add_ns(NS_PER_DAY));
        assert_eq!(real_instant(birth, birth.add_ns(NS_PER_DAY)), a_year);
        assert_eq!(progressed_instant(birth, birth), birth);
    }

    #[test]
    fn lunation_phases_cover_the_circle() {
        assert_eq!(lunation_phase_name(0.0), "New");
        assert_eq!(lunation_phase_name(44.9), "New");
        assert_eq!(lunation_phase_name(180.0), "Full");
        assert_eq!(lunation_phase_name(359.9), "Balsamic");
        assert_eq!(lunation_phase_name(-10.0), "Balsamic");
    }

    #[test]
    fn progressed_sun_advances_a_degree_a_year() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let at = fixture::utc(2026, 10, 19, 12, 0);
        let p = progressions(&spk, &natal, fixture::observer().latitude_deg, at).unwrap();
        let age = at.ns_since(natal.instant) as f64 / YEAR_NS as f64;
        let natal_sun = natal.bodies[0].tropical;
        let arc = wrap180(p.bodies[0].longitude - natal_sun);
        assert!((arc - age).abs() < 1.5, "arc {arc} age {age}");
        assert!((p.solar_arc - arc).abs() < 1e-9);
        assert!((wrap180(p.mc - natal.cusps[9]) - arc).abs() < 1e-9);
        assert!((0.0..360.0).contains(&p.asc));
        assert!(LUNATION_PHASES.contains(&p.lunation_phase));
        // the solar-arc MC at birth is the natal MC, and its Ascendant the natal Ascendant
        let p0 = progressions(
            &spk,
            &natal,
            fixture::observer().latitude_deg,
            natal.instant,
        )
        .unwrap();
        assert!(p0.solar_arc.abs() < 1e-9);
        assert!(
            wrap180(p0.asc - natal.cusps[0]).abs() < 1e-6,
            "{} vs {}",
            p0.asc,
            natal.cusps[0]
        );
    }

    #[test]
    fn progressed_moon_changes_sign_every_two_and_a_half_years() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let from = fixture::utc(2020, 1, 1, 0, 0);
        let to = fixture::utc(2030, 1, 1, 0, 0);
        let ing = progressed_moon_ingresses(&spk, &natal, from, to).unwrap();
        assert!((3..=5).contains(&ing.len()), "{}", ing.len());
        for w in ing.windows(2) {
            let years = w[1].ns_since(w[0]) as f64 / YEAR_NS as f64;
            assert!((1.9..=3.0).contains(&years), "{years}");
        }
        for t in &ing {
            let lon = progressed_longitude(&spk, natal.instant, Body::Moon, *t).unwrap();
            let off = wrap180(lon - 30.0 * (lon / 30.0).round());
            assert!(off.abs() < 1e-6, "{lon}");
        }
    }

    #[test]
    fn progressed_moon_conjunct_natal_sun_is_exact() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let natal_sun = natal.bodies[0].tropical;
        let from = natal.instant;
        let to = fixture::utc(2030, 1, 1, 0, 0);
        let hits = progressed_aspect_instants(
            &spk,
            &natal,
            Body::Moon,
            natal_sun,
            Aspect::Conjunction,
            from,
            to,
        )
        .unwrap();
        assert!(!hits.is_empty());
        for t in &hits {
            let lon = progressed_longitude(&spk, natal.instant, Body::Moon, *t).unwrap();
            assert!(wrap180(lon - natal_sun).abs() < 1e-6);
        }
        let squares = progressed_aspect_instants(
            &spk,
            &natal,
            Body::Moon,
            natal_sun,
            Aspect::Square,
            from,
            to,
        )
        .unwrap();
        assert!(squares.len() >= hits.len());
        assert!(squares.windows(2).all(|w| w[0] < w[1]));
    }
}
