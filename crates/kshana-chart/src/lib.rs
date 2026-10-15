//! Assemble a chart. Everything the screens show comes from [`compute`]; the CLI prints
//! it and the FFI serialises it, neither adds astronomy of its own.

#![forbid(unsafe_code)]

use kshana_ephem::{Body, Spk, SpkError};
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_iau2006a,
};
use kshana_houses::{cusps, horary_cusps, HouseError, HouseSystem};
use kshana_sky::{
    apparent, apparent_topocentric, mean_node_longitude, true_node_longitude, Observer,
};
use kshana_time::{delta_t_for_year, ut1_jd_from_delta_t, EopTable, Instant, TwoPartJd, Utc};
use kshana_vedic::{
    ayanamsa_degrees, horary_longitude, karana, kp_lords, nakshatra, sidereal, tithi, yoga,
    Ayanamsa, DashaLord,
};

/// What to compute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartInput {
    /// The instant.
    pub instant: Instant,
    /// Where.
    pub observer: Observer,
    /// House system.
    pub system: HouseSystem,
    /// Sidereal zodiac definition.
    pub ayanamsa: Ayanamsa,
    /// Use the topocentric Moon (parallax-corrected). Jyotisha tradition is geocentric;
    /// Western practice varies. Off by default.
    pub topocentric_moon: bool,
}

/// One body's placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// Name.
    pub name: &'static str,
    /// Tropical longitude of date, degrees.
    pub tropical: f64,
    /// Sidereal longitude, degrees.
    pub sidereal: f64,
    /// Ecliptic latitude, degrees.
    pub latitude: f64,
    /// Daily motion, degrees per day.
    pub speed: f64,
    /// Distance, km (0 for computed points).
    pub distance_km: f64,
    /// Nakshatra index 0–26 and pada 1–4 (sidereal).
    pub nakshatra: (usize, u8),
    /// KP star, sub and sub-sub lords (sidereal).
    pub kp: (DashaLord, DashaLord, DashaLord),
    /// House 1–12 by the chart's cusps (tropical).
    pub house: u8,
}

/// Panchanga at the instant.
#[derive(Clone, Debug, PartialEq)]
pub struct PanchangaAt {
    /// Nakshatra index 0–26, pada, fraction elapsed.
    pub nakshatra: (usize, u8, f64),
    /// Tithi number 1–30 and fraction elapsed.
    pub tithi: (u8, f64),
    /// Yoga index 0–26 and fraction.
    pub yoga: (usize, f64),
    /// Karana ordinal 0–59 and name.
    pub karana: (u8, &'static str),
}

/// Data and algorithm versions that produced the chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// Engine version.
    pub engine: String,
    /// Kernel description from the SPK file.
    pub kernel: String,
    /// Where UT1 came from.
    pub ut1_source: String,
    /// Leap-second table version.
    pub leap_seconds: String,
    /// Frame and model names.
    pub models: String,
}

/// The chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    /// Input instant.
    pub instant: Instant,
    /// TT two-part JD.
    pub tt: TwoPartJd,
    /// UT1 two-part JD.
    pub ut1: TwoPartJd,
    /// Local apparent sidereal time, degrees.
    pub lst_deg: f64,
    /// True obliquity, degrees.
    pub obliquity_deg: f64,
    /// Ayanamsa, degrees.
    pub ayanamsa_deg: f64,
    /// Sun through Pluto.
    pub bodies: Vec<Placement>,
    /// Mean node (tropical, sidereal).
    pub mean_node: (f64, f64),
    /// True node (tropical, sidereal).
    pub true_node: (f64, f64),
    /// Twelve cusps, tropical degrees, index 0 = house 1.
    pub cusps: [f64; 12],
    /// Twelve cusps, sidereal.
    pub cusps_sidereal: [f64; 12],
    /// Panchanga.
    pub panchanga: PanchangaAt,
    /// Provenance stamp.
    pub provenance: Provenance,
}

/// Why a chart could not be computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChartError {
    /// Ephemeris problem (range, file).
    Spk(SpkError),
    /// House system undefined here.
    Houses(HouseError),
}

impl From<SpkError> for ChartError {
    fn from(e: SpkError) -> Self {
        ChartError::Spk(e)
    }
}
impl From<HouseError> for ChartError {
    fn from(e: HouseError) -> Self {
        ChartError::Houses(e)
    }
}

/// Bodies in chart order with their kernel ids.
pub const BODIES: [(&str, Body); 10] = [
    ("Sun", Body::Sun),
    ("Moon", Body::Moon),
    ("Mercury", Body::Mercury),
    ("Venus", Body::Venus),
    ("Mars", Body::MarsBarycenter),
    ("Jupiter", Body::JupiterBarycenter),
    ("Saturn", Body::SaturnBarycenter),
    ("Uranus", Body::UranusBarycenter),
    ("Neptune", Body::NeptuneBarycenter),
    ("Pluto", Body::PlutoBarycenter),
];

/// UT1 as a two-part JD and a description of its source.
pub fn ut1_for(instant: Instant, eop: Option<&EopTable>) -> (TwoPartJd, String) {
    let utc = Utc::from_instant(instant);
    let mjd_utc = utc.utc_day() as f64
        + 40_587.0
        + (utc.hour as f64 * 3_600.0
            + utc.minute as f64 * 60.0
            + utc.second as f64
            + utc.nanos as f64 * 1e-9)
            / 86_400.0;
    if let Some(u) = eop.and_then(|e| e.ut1_minus_utc(mjd_utc)) {
        let jd = TwoPartJd {
            jd1: 2_400_000.5 + mjd_utc.floor(),
            jd2: mjd_utc.fract() + u.ut1_minus_utc / 86_400.0,
        };
        return (
            jd,
            format!(
                "IERS finals2000A UT1−UTC {:+.4} s ±{:.4} s",
                u.ut1_minus_utc, u.sigma
            ),
        );
    }
    let year = utc.year as f64 + (utc.month as f64 - 1.0) / 12.0;
    let dt = delta_t_for_year(year);
    (
        ut1_jd_from_delta_t(instant, dt),
        format!("ΔT table {:.3} s ±{:.2} s", dt.seconds, dt.sigma),
    )
}

fn house_of(lon: f64, cusps: &[f64; 12]) -> u8 {
    for i in 0..12 {
        let a = cusps[i];
        let b = cusps[(i + 1) % 12];
        let span = (b - a).rem_euclid(360.0);
        let off = (lon - a).rem_euclid(360.0);
        if off < span {
            return (i + 1) as u8;
        }
    }
    12
}

/// Compute the chart.
pub fn compute(spk: &Spk, eop: Option<&EopTable>, input: &ChartInput) -> Result<Chart, ChartError> {
    compute_with_lagna(spk, eop, input, None)
}

/// A KP horary chart: the lagna is the start of sub `number` (1–249) in the sidereal
/// zodiac, and the other cusps are those of the moment that lagna rises at the place.
/// Planets are for the instant of the question as usual.
pub fn compute_horary(
    spk: &Spk,
    eop: Option<&EopTable>,
    input: &ChartInput,
    number: u16,
) -> Result<Chart, ChartError> {
    let lagna_sid =
        horary_longitude(number).ok_or(ChartError::Houses(HouseError::NotImplemented))?;
    compute_with_lagna(spk, eop, input, Some(lagna_sid))
}

fn compute_with_lagna(
    spk: &Spk,
    eop: Option<&EopTable>,
    input: &ChartInput,
    lagna_sidereal: Option<f64>,
) -> Result<Chart, ChartError> {
    let t = input.instant;
    let tt = t.tt_jd();
    let tc = t.tt_centuries_since_j2000();
    let (ut1, ut1_source) = ut1_for(t, eop);
    let era = earth_rotation_angle(ut1);
    let g = gast(era, tc);
    let lst = local_sidereal_time(g, input.observer.longitude_deg.to_radians());
    let (_, deps) = nutation_iau2006a(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let ay = ayanamsa_degrees(input.ayanamsa, tc);
    let lat = input.observer.latitude_deg.to_radians();

    let c = match lagna_sidereal {
        Some(ls) => horary_cusps(input.system, (ls + ay).rem_euclid(360.0), eps, lat)?,
        None => cusps(input.system, lst, eps, lat)?,
    };
    let cs: [f64; 12] = std::array::from_fn(|i| sidereal(c[i], ay));

    let mut bodies = Vec::with_capacity(10);
    for (name, body) in BODIES {
        let a = if input.topocentric_moon && body == Body::Moon {
            apparent_topocentric(spk, body, t, &input.observer, g)?
        } else {
            apparent(spk, body, t)?
        };
        let sid = sidereal(a.longitude, ay);
        let n = nakshatra(sid);
        let k = kp_lords(sid);
        bodies.push(Placement {
            name,
            tropical: a.longitude,
            sidereal: sid,
            latitude: a.latitude,
            speed: a.speed,
            distance_km: a.distance_km,
            nakshatra: (n.index, n.pada),
            kp: (k.star, k.sub, k.sub_sub),
            house: house_of(a.longitude, &c),
        });
    }
    let mn = mean_node_longitude(tc);
    let tn = true_node_longitude(spk, t)?;
    let moon = bodies[1].sidereal;
    let sun = bodies[0].sidereal;
    let n = nakshatra(moon);
    let ti = tithi(moon, sun);
    let y = yoga(moon, sun);
    let k = karana(moon, sun);
    Ok(Chart {
        instant: t,
        tt,
        ut1,
        lst_deg: lst.to_degrees(),
        obliquity_deg: eps.to_degrees(),
        ayanamsa_deg: ay,
        bodies,
        mean_node: (mn, sidereal(mn, ay)),
        true_node: (tn, sidereal(tn, ay)),
        cusps: c,
        cusps_sidereal: cs,
        panchanga: PanchangaAt {
            nakshatra: (n.index, n.pada, n.fraction),
            tithi: (ti.number, ti.fraction),
            yoga: (y.index, y.fraction),
            karana: (k.ordinal, k.name),
        },
        provenance: Provenance {
            engine: format!("kshana {}", env!("CARGO_PKG_VERSION")),
            kernel: spk.segments().first().map(|s| format!("SPK {} segments, type {}", spk.segments().len(), s.data_type)).unwrap_or_default(),
            ut1_source,
            leap_seconds: "IERS Bulletin C, last leap second 2017-01-01 (TAI−UTC 37 s)".into(),
            models: format!("IAU 2006 precession, IAU 2000A nutation, CIO-based GAST, {:?} houses, {:?} ayanamsa", input.system, input.ayanamsa),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn houses_wrap_correctly() {
        let c: [f64; 12] = std::array::from_fn(|i| (300.0 + 30.0 * i as f64).rem_euclid(360.0));
        assert_eq!(house_of(305.0, &c), 1);
        assert_eq!(house_of(5.0, &c), 3);
        assert_eq!(house_of(299.0, &c), 12);
    }

    #[test]
    fn horary_chart_fixes_the_lagna() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let t = Utc::new(2026, 10, 15, 9, 2, 7, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let input = ChartInput {
            instant: t,
            observer: Observer {
                latitude_deg: 9.9312,
                longitude_deg: 76.2673,
                height_m: 0.0,
            },
            system: HouseSystem::Placidus,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        };
        let ch = compute_horary(&spk, None, &input, 100).unwrap();
        let want = horary_longitude(100).unwrap();
        assert!(
            (ch.cusps_sidereal[0] - want).abs() < 1e-6,
            "{} vs {want}",
            ch.cusps_sidereal[0]
        );
        assert!(((ch.cusps[6] - ch.cusps[0]).rem_euclid(360.0) - 180.0).abs() < 1e-9);
        // planets unchanged from the natal computation at the same instant
        let natal = compute(&spk, None, &input).unwrap();
        assert_eq!(ch.bodies[0].tropical, natal.bodies[0].tropical);
    }

    #[test]
    fn kochi_chart_from_the_kernel() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let t = Utc::new(2026, 10, 15, 9, 2, 7, 418_263_000)
            .unwrap()
            .to_instant()
            .unwrap();
        let input = ChartInput {
            instant: t,
            observer: Observer {
                latitude_deg: 9.9312,
                longitude_deg: 76.2673,
                height_m: 0.0,
            },
            system: HouseSystem::Placidus,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        };
        let ch = compute(&spk, None, &input).unwrap();
        assert_eq!(ch.bodies.len(), 10);
        // Sun ~22° Libra, Asc ~20°42′ Aquarius (cross-checked with the CLI run)
        assert!(
            (ch.bodies[0].tropical - 202.03).abs() < 0.05,
            "{}",
            ch.bodies[0].tropical
        );
        assert!((ch.cusps[0] - 320.7).abs() < 0.2, "{}", ch.cusps[0]);
        assert!(
            (ch.cusps[6] - (ch.cusps[0] + 180.0))
                .rem_euclid(360.0)
                .abs()
                < 1e-9
        );
        assert_eq!(ch.panchanga.tithi.0, 5);
        assert!(ch.provenance.models.contains("Placidus"));
    }
}
