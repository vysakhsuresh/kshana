//! Western and Hellenistic timing on exact instants.
//!
//! Two kinds of "when" live here. Calendar techniques (profections, firdaria, zodiacal
//! releasing) are integer arithmetic on nanoseconds from a stated year length, so their
//! boundaries are exact by construction. Sky techniques (secondary progressions, solar
//! arc, lunar returns, transits) are roots of a smooth function of the real sky found
//! with `kshana-solver`: every instant they return is a solved crossing, never an
//! interpolation between tabulated positions.
//!
//! Each technique states its convention and its source on the function that uses it:
//!
//! * [`progressions`]: secondary progressions, a day for a year on the mean tropical
//!   year, angles by the solar-arc MC.
//! * [`solar_arc`]: solar arc directions, arc = progressed Sun − natal Sun.
//! * [`profections`]: annual and monthly profections between exact solar returns.
//! * [`firdaria`]: the Persian firdaria, a 75-year cycle of 365.25-day years.
//! * [`zodiacal_releasing`]: releasing from the Lots of Fortune and Spirit in 360-day
//!   years with the loosing of the bond.
//! * [`lunar_returns`] and [`return_chart`]: returns on the real sky.
//! * [`transit_hits`]: exact transit-to-natal contacts.

#![forbid(unsafe_code)]

mod firdaria;
mod profections;
mod progressions;
mod releasing;
mod returns;
mod solar_arc;
mod transits;

pub use firdaria::{
    firdaria, firdaria_at, firdaria_major, FirdariaLord, FirdariaPeriod, CHALDEAN_ORDER,
    DAY_SEQUENCE, FIRDARIA_CYCLE_YEARS, FIRDARIA_YEAR_NS, NIGHT_SEQUENCE,
};
pub use profections::{profections, Profection};
pub use progressions::{
    ascendant_for_mc, lunation_phase_name, progressed_aspect_instants, progressed_ingresses,
    progressed_instant, progressed_longitude, progressed_moon_ingresses, progressions,
    real_instant, Progressed, ProgressedBody, LUNATION_PHASES, PROGRESSED_SCAN_STEP_NS,
    TROPICAL_YEAR_DAYS,
};
pub use releasing::{
    is_peak, lots, zodiacal_releasing, zodiacal_releasing_from, zr_unit_ns, zr_years, Lots,
    ZrPeriod, ZR_CIRCUIT_YEARS, ZR_MAX_LEVEL, ZR_YEAR_NS,
};
pub use returns::{demi_returns, lunar_returns, return_chart, solar_returns};
pub use solar_arc::{
    directed_positions, solar_arc, solar_arc_hits, solar_arc_hits_with, DirectionHit,
    ARC_SCAN_STEP_NS,
};
pub use transits::{transit_hits, TransitHit, TRANSIT_SCAN_STEP_NS};

use std::collections::HashMap;

use kshana_chart::{Chart, ChartError, BODIES};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{sun_altitude, EventError};
use kshana_houses::HouseError;
use kshana_sky::{apparent, Observer};
use kshana_solver::{find_root, Root, SolverError};
use kshana_time::{EopTable, Instant};
use kshana_western::{is_day_chart, Aspect, Sign};

/// Why a timing routine could not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectionsError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Root finder failure.
    Solver(SolverError),
    /// Event search failure.
    Events(EventError),
    /// Chart computation failure.
    Chart(ChartError),
    /// House computation failure.
    Houses(HouseError),
    /// The natal chart has no placement with this name.
    MissingBody(&'static str),
    /// No solar return was found in the search window of this calendar year.
    NoSolarReturn(i64),
    /// The instant precedes the birth it is measured from.
    BeforeBirth,
}

impl From<SpkError> for DirectionsError {
    fn from(e: SpkError) -> Self {
        DirectionsError::Spk(e)
    }
}
impl From<SolverError> for DirectionsError {
    fn from(e: SolverError) -> Self {
        DirectionsError::Solver(e)
    }
}
impl From<EventError> for DirectionsError {
    fn from(e: EventError) -> Self {
        DirectionsError::Events(e)
    }
}
impl From<ChartError> for DirectionsError {
    fn from(e: ChartError) -> Self {
        DirectionsError::Chart(e)
    }
}
impl From<HouseError> for DirectionsError {
    fn from(e: HouseError) -> Self {
        DirectionsError::Houses(e)
    }
}

/// Name of the Ascendant among the natal points.
pub const ASCENDANT: &str = "Ascendant";
/// Name of the Midheaven among the natal points.
pub const MC: &str = "MC";

/// The twelve signs in zodiacal order, so that a sign can be counted from.
pub const SIGNS: [Sign; 12] = [
    Sign::Aries,
    Sign::Taurus,
    Sign::Gemini,
    Sign::Cancer,
    Sign::Leo,
    Sign::Virgo,
    Sign::Libra,
    Sign::Scorpio,
    Sign::Sagittarius,
    Sign::Capricorn,
    Sign::Aquarius,
    Sign::Pisces,
];

/// Zero-based index of a sign in [`SIGNS`] (Aries = 0).
pub fn sign_index(sign: Sign) -> usize {
    SIGNS.iter().position(|s| *s == sign).unwrap_or(0)
}

/// The sign `count` signs after `sign` (counting inclusively, `count = 0` is the sign itself).
pub fn sign_add(sign: Sign, count: usize) -> Sign {
    SIGNS[(sign_index(sign) + count) % 12]
}

/// A natal reference point: a chart body or an angle, with its tropical longitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NatalPoint {
    /// Body name as in the chart ("Sun", "Moon", ...) or [`ASCENDANT`] / [`MC`].
    pub name: &'static str,
    /// Tropical longitude of date in degrees, `[0, 360)`.
    pub longitude: f64,
}

/// The natal points a direction or transit can hit: the chart bodies in chart order,
/// then the Ascendant and the MC (tropical cusps 1 and 10).
pub fn natal_points(natal: &Chart) -> Vec<NatalPoint> {
    let mut v: Vec<NatalPoint> = natal
        .bodies
        .iter()
        .map(|p| NatalPoint {
            name: p.name,
            longitude: p.tropical,
        })
        .collect();
    v.push(NatalPoint {
        name: ASCENDANT,
        longitude: natal.cusps[0],
    });
    v.push(NatalPoint {
        name: MC,
        longitude: natal.cusps[9],
    });
    v
}

/// Tropical longitude of a named natal point (body, [`ASCENDANT`] or [`MC`]).
pub fn natal_longitude(natal: &Chart, name: &str) -> Option<f64> {
    natal_points(natal)
        .into_iter()
        .find(|p| p.name == name)
        .map(|p| p.longitude)
}

/// Like [`natal_longitude`] but an error when the point is missing.
pub(crate) fn require(natal: &Chart, name: &'static str) -> Result<f64, DirectionsError> {
    natal_longitude(natal, name).ok_or(DirectionsError::MissingBody(name))
}

/// Sect at an instant: day when the Sun's centre is above the geometric horizon.
pub fn is_day_at(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
) -> Result<bool, DirectionsError> {
    Ok(is_day_chart(sun_altitude(spk, eop, obs, t)?))
}

/// The longitudes at which `aspect` to `point` is exact: `point + angle`, and also
/// `point − angle` unless the aspect is the conjunction or the opposition.
pub fn aspect_targets(point: f64, aspect: Aspect) -> Vec<f64> {
    let a = aspect.angle();
    if a == 0.0 || a == 180.0 {
        vec![(point + a).rem_euclid(360.0)]
    } else {
        vec![(point + a).rem_euclid(360.0), (point - a).rem_euclid(360.0)]
    }
}

/// Chart name of a body, or "body" for one outside the chart's list.
pub(crate) fn body_name(body: Body) -> &'static str {
    BODIES
        .iter()
        .find(|(_, b)| *b == body)
        .map(|(n, _)| *n)
        .unwrap_or("body")
}

/// The instant one scan step after `a`, clipped to `to`.
pub(crate) fn next_step(a: Instant, to: Instant, step_ns: i128) -> Instant {
    let b = a.add_ns(step_ns);
    if b > to {
        to
    } else {
        b
    }
}

/// Scan `[from, to]` in `step_ns` steps for sign changes of `f` and solve each one to
/// `tol_ns`. A sign change whose jump exceeds 180° is the 360° wrap of a longitude
/// difference, not a root. An exact zero at a sample is reported as a root of width 0.
pub(crate) fn scan_roots<F>(
    mut f: F,
    from: Instant,
    to: Instant,
    step_ns: i128,
    tol_ns: i128,
) -> Result<Vec<Root>, DirectionsError>
where
    F: FnMut(Instant) -> Result<f64, SpkError>,
{
    let mut out = Vec::new();
    if to <= from {
        return Ok(out);
    }
    let mut a = from;
    let mut fa = f(a)?;
    while a < to {
        let b = next_step(a, to, step_ns);
        let fb = f(b)?;
        if fa == 0.0 {
            out.push(Root {
                t: a,
                half_width_ns: 0,
                evaluations: 0,
            });
        } else if (fa > 0.0) != (fb > 0.0) && (fa - fb).abs() < 180.0 {
            let root = find_root(|t| f(t).unwrap_or(f64::NAN), a, b, tol_ns)?;
            out.push(root);
        }
        a = b;
        fa = fb;
    }
    if fa == 0.0 {
        out.push(Root {
            t: a,
            half_width_ns: 0,
            evaluations: 0,
        });
    }
    Ok(out)
}

/// Longitudes of one body sampled on exactly the instants [`scan_roots`] visits for the
/// same `from`, `to` and `step_ns`, so that many targets can be scanned against one set
/// of ephemeris evaluations. Off-grid instants (the solver's) go to the ephemeris.
pub(crate) struct LongitudeCache<'a> {
    spk: &'a Spk,
    body: Body,
    samples: HashMap<Instant, f64>,
}

impl<'a> LongitudeCache<'a> {
    pub(crate) fn new(
        spk: &'a Spk,
        body: Body,
        from: Instant,
        to: Instant,
        step_ns: i128,
    ) -> Result<Self, SpkError> {
        let mut samples = HashMap::new();
        let mut t = from;
        loop {
            samples.insert(t, apparent(spk, body, t)?.longitude);
            if t >= to {
                break;
            }
            t = next_step(t, to, step_ns);
        }
        Ok(Self { spk, body, samples })
    }

    pub(crate) fn longitude(&self, t: Instant) -> Result<f64, SpkError> {
        match self.samples.get(&t) {
            Some(l) => Ok(*l),
            None => apparent(self.spk, self.body, t).map(|a| a.longitude),
        }
    }
}

/// The shared test fixture: the DE421 kernel (tests return early without it) and a
/// Placidus natal chart for 1990-10-29 12:06 UTC at Kochi.
#[cfg(test)]
pub(crate) mod fixture {
    use kshana_chart::{compute, Chart, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::{Instant, Utc};
    use kshana_vedic::Ayanamsa;

    pub fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    pub fn utc(y: i64, m: u32, d: u32, h: u32, min: u32) -> Instant {
        Utc::new(y, m, d, h, min, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }

    pub fn birth() -> Instant {
        utc(1990, 10, 29, 12, 6)
    }

    pub fn observer() -> Observer {
        Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        }
    }

    pub fn natal(spk: &Spk) -> Chart {
        compute(
            spk,
            None,
            &ChartInput {
                instant: birth(),
                observer: observer(),
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;

    #[test]
    fn signs_count_from_aries() {
        assert_eq!(sign_index(Sign::Aries), 0);
        assert_eq!(sign_index(Sign::Pisces), 11);
        assert_eq!(sign_add(Sign::Capricorn, 6), Sign::Cancer);
        assert_eq!(sign_add(Sign::Pisces, 1), Sign::Aries);
    }

    #[test]
    fn aspect_targets_are_one_or_two() {
        assert_eq!(aspect_targets(10.0, Aspect::Conjunction), vec![10.0]);
        assert_eq!(aspect_targets(10.0, Aspect::Opposition), vec![190.0]);
        assert_eq!(aspect_targets(10.0, Aspect::Square), vec![100.0, 280.0]);
    }

    #[test]
    fn scan_finds_a_linear_root_without_the_ephemeris() {
        let t0 = Instant::j2000();
        let target = t0.add_ns(5 * NS_PER_DAY + 123_456_789);
        let f = |t: Instant| Ok(t.ns_since(target) as f64 / NS_PER_DAY as f64);
        let roots = scan_roots(f, t0, t0.add_ns(20 * NS_PER_DAY), 3 * NS_PER_DAY, 1).unwrap();
        assert_eq!(roots.len(), 1);
        assert!(roots[0].t.ns_since(target).abs() <= 2);
    }

    #[test]
    fn natal_points_end_with_the_angles() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let pts = natal_points(&natal);
        assert_eq!(pts.len(), 12);
        assert_eq!(pts[0].name, "Sun");
        assert_eq!(pts[10].name, ASCENDANT);
        assert_eq!(pts[11].name, MC);
        assert_eq!(natal_longitude(&natal, "MC"), Some(natal.cusps[9]));
        assert!(require(&natal, "Vulcan").is_err());
        // the fixture is a late-afternoon birth: still a day chart
        assert!(is_day_at(&spk, None, &fixture::observer(), natal.instant).unwrap());
    }
}
