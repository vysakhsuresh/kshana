//! Where Kshana's defaults differ from Jagannatha Hora's, switching Kshana to the JH-like
//! setting moves the result the way, and by the amount, docs/conventions.md states.
//! JH itself is not run here; its defaults are taken from its documented Python port,
//! PyJHora (`jhora/const.py`), as cited in the doc.

use kshana_calendar::{Calendar, Config, Date};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_events::RiseConvention;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_DAY};
use kshana_vedic::dasha::{YEAR_365_25_NS, YEAR_SIDEREAL_NS};
use kshana_vedic::{ayanamsa_degrees, subdivide, vimshottari, Ayanamsa};
use std::path::PathBuf;

fn kernel() -> Option<Spk> {
    Spk::open(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp")).ok()
}

fn at(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> Instant {
    Utc::new(y, mo, d, h, mi, 0, 0).unwrap().to_instant().unwrap()
}

const KOCHI: Observer = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };

/// Dasha year: Kshana 365.25 days; JH's default is the true sidereal year, 365.256 364 d
/// (PyJHora `DHASA_YEAR_DURATION.JHORA_DEFAULT = TRUE_SIDEREAL_YEAR`). With the JH year
/// every boundary after birth comes later, by 0.006 364 days for every year elapsed since
/// birth (0.38 d at 60 years), and every boundary before birth earlier.
#[test]
fn sidereal_dasha_year_moves_every_later_boundary_later_by_the_stated_amount() {
    let birth = at(1990, 2, 20, 5, 4);
    let moon = 244.505_683; // the Kochi 1990 chart's Moon (kshana-chart's pinned test)
    let ours = vimshottari(moon, birth, YEAR_365_25_NS);
    let jh = vimshottari(moon, birth, YEAR_SIDEREAL_NS);
    let per_year_days = (YEAR_SIDEREAL_NS - YEAR_365_25_NS) as f64 / NS_PER_DAY as f64;
    assert!((per_year_days - 0.006_364).abs() < 1e-6);
    let mut checked = 0;
    for (a, b) in ours.iter().zip(&jh) {
        for (pa, pb) in subdivide(*a).iter().zip(subdivide(*b).iter()) {
            assert_eq!(pa.lord, pb.lord);
            let years_from_birth = pa.end.ns_since(birth) as f64 / YEAR_365_25_NS as f64;
            let shift_days = pb.end.ns_since(pa.end) as f64 / NS_PER_DAY as f64;
            assert!(
                (shift_days - years_from_birth * per_year_days).abs() < 1e-6,
                "{:?}/{:?}: {shift_days} d at {years_from_birth} y",
                a.lord,
                pa.lord
            );
            assert_eq!(shift_days > 0.0, years_from_birth > 0.0);
            checked += 1;
        }
    }
    assert_eq!(checked, 81);
    let sixty = 60.0 * per_year_days;
    assert!((0.38..0.39).contains(&sixty), "{sixty}");
}

/// Rahu: Kshana's default is the true (osculating) node, as PyJHora's
/// (`_use_true_nodes_for_rahu_ketu = True`); a JH set to the mean node differs by the
/// node's wobble: up to about 1°57′ either way (measured 1950–1999 below).
#[test]
fn mean_node_differs_from_true_node_by_less_than_two_degrees_either_way() {
    let Some(spk) = kernel() else { return };
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for i in 0..400 {
        let t = at(1950, 1, 1, 0, 0).add_ns(i as i128 * 45 * NS_PER_DAY + i as i128 * 3_600 * 1_000_000_000);
        let input = ChartInput {
            instant: t,
            observer: KOCHI,
            system: HouseSystem::WholeSign,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        };
        let ch = compute(&spk, None, &input).unwrap();
        let d = ((ch.true_node.1 - ch.mean_node.1 + 180.0).rem_euclid(360.0)) - 180.0;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    eprintln!("true − mean node over 1950–1999: {lo:.3}° … {hi:.3}°");
    assert!(lo > -2.0 && hi < 2.0, "{lo} {hi}");
    assert!(lo < -1.5 && hi > 1.5, "both directions reached: {lo} {hi}");
}

/// Ayanamsa: both default to Lahiri (Chitrapaksha). JH's "True Chitrapaksha" option is
/// Kshana's `TrueChitra` (Spica held at 180° from its apparent place). True Chitra is
/// smaller than Lahiri all through 1900–2050 (the star's annual aberration makes the
/// gap swing by about ±20″ within each year; the range is printed and pinned below);
/// Swiss Ephemeris gives the same difference (tests/golden/swisseph_ayanamsa.json,
/// SE_SIDM_TRUE_CITRA 27 minus SE_SIDM_LAHIRI 1). Switching moves every sidereal
/// longitude and cusp by minus the ayanamsa change, so under True Chitra every sidereal
/// position is that much larger.
#[test]
fn true_chitra_moves_every_sidereal_longitude_by_minus_the_ayanamsa_change() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/swisseph_ayanamsa.json");
    let g: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    for r in g["ayanamsa"].as_array().unwrap() {
        let tc = (r["jd_tt"].as_f64().unwrap() - 2_451_545.0) / 36_525.0;
        let se = (r["27"].as_f64().unwrap() - r["1"].as_f64().unwrap()) * 3_600.0;
        let ours = (ayanamsa_degrees(Ayanamsa::TrueChitra, tc) - ayanamsa_degrees(Ayanamsa::Lahiri, tc)) * 3_600.0;
        assert!((ours - se).abs() < 0.1, "{}: {ours:.2}″ vs SE {se:.2}″", r["utc"]);
    }
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for m in 0..(150 * 12) {
        let tc = (-100.0 + m as f64 / 12.0) / 100.0;
        let d = (ayanamsa_degrees(Ayanamsa::TrueChitra, tc) - ayanamsa_degrees(Ayanamsa::Lahiri, tc)) * 3_600.0;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    eprintln!("True Chitra − Lahiri, 1900–2050 monthly: {lo:.1}″ … {hi:.1}″");
    assert!(lo > -90.0 && hi < -25.0, "{lo} {hi}");
    let Some(spk) = kernel() else { return };
    for (y, mo, d) in [(1950, 6, 1), (1975, 1, 1), (2000, 1, 1), (2026, 10, 21)] {
        let t = at(y, mo, d, 6, 0);
        let mk = |ayanamsa| ChartInput {
            instant: t,
            observer: KOCHI,
            system: HouseSystem::Placidus,
            ayanamsa,
            topocentric_moon: false,
        };
        let a = compute(&spk, None, &mk(Ayanamsa::Lahiri)).unwrap();
        let b = compute(&spk, None, &mk(Ayanamsa::TrueChitra)).unwrap();
        let dayn = (b.ayanamsa_deg - a.ayanamsa_deg) * 3_600.0;
        assert!(dayn < 0.0, "{y}: {dayn}″");
        for (pa, pb) in a.bodies.iter().zip(&b.bodies) {
            let shift = ((pb.sidereal - pa.sidereal + 180.0).rem_euclid(360.0) - 180.0) * 3_600.0;
            assert!((shift + dayn).abs() < 1e-6, "{} {shift}″", pa.name);
        }
        let cs = ((b.cusps_sidereal[0] - a.cusps_sidereal[0] + 180.0).rem_euclid(360.0) - 180.0) * 3_600.0;
        assert!((cs + dayn).abs() < 1e-6);
    }
}

/// Sunrise: Kshana's panchanga (like Drik Panchang) uses the Sun's upper limb with
/// refraction; the "Hindu" rising of the Kerala and Tamil almanacs (and a JH option)
/// takes the centre on the geometric horizon. That rising is later, by 3.4–3.7 minutes at
/// Kochi through the year.
#[test]
fn centre_geometric_sunrise_is_later_by_a_few_minutes_at_kochi() {
    let Some(spk) = kernel() else { return };
    let from = Date::new(2026, 1, 1);
    let drik = Calendar::new(&spk, None, KOCHI, Config::india(), from, 365).unwrap();
    let hindu = Calendar::new(
        &spk,
        None,
        KOCHI,
        Config { rise: RiseConvention::CentreGeometric, ..Config::india() },
        from,
        365,
    )
    .unwrap();
    let (mut lo, mut hi) = (f64::MAX, 0.0f64);
    for d in drik.requested() {
        let date = drik.days[d].date;
        let h = hindu.days[hindu.index_of(date).unwrap()].sunrise;
        let late = h.ns_since(drik.days[d].sunrise) as f64 / 60e9;
        lo = lo.min(late);
        hi = hi.max(late);
    }
    eprintln!("centre-geometric minus upper-limb-refracted sunrise at Kochi: {lo:.2}–{hi:.2} min");
    assert!(lo > 3.2 && hi < 3.9, "{lo} {hi}");
}
