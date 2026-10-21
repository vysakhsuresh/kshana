//! The whole chart path (`kshana_chart::compute`, the one the app and the CLI use)
//! against published references:
//!
//! * Swiss Ephemeris 2.10.03 (`tests/golden/swisseph.json`, see tests/golden/README.md):
//!   sidereal (Lahiri) and tropical longitudes of every body, the nodes, the ayanamsa
//!   the chart applies, the ascendant and the twelve Placidus cusps.
//! * Drik Panchang (drikpanchang.com day pages, read 2026-10-21): sunrise, and the
//!   tithi, nakshatra, yoga and karana at sunrise together with the minute each ends.
//!
//! `swisseph.json` was made in Moshier mode, so the tolerances on the bodies are the
//! Moshier ones used in `swisseph.rs` (Sun 2″, Moon 15″, planets 3″, Pluto 10″).

use kshana_calendar::{Calendar, Config, Date};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::{Body, Spk};
use kshana_houses::HouseSystem;
use kshana_sky::{apparent, Observer};
use kshana_time::{Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, karana, nakshatra, sidereal, tithi, yoga, Ayanamsa};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn kernel() -> Option<Spk> {
    Spk::open(&root().join("data/de421.bsp")).ok()
}

fn swisseph() -> Value {
    let p = root().join("tests/golden/swisseph.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

fn wrap(d: f64) -> f64 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

fn instant_from_jd_tt(jd: f64) -> Instant {
    let days = jd - 2_451_545.0;
    let whole = days.floor();
    Instant::j2000()
        .add_ns(whole as i128 * NS_PER_DAY + ((days - whole) * NS_PER_DAY as f64).round() as i128)
}

fn input(t: Instant, lat: f64, lon: f64, system: HouseSystem) -> ChartInput {
    ChartInput {
        instant: t,
        observer: Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 },
        system,
        ayanamsa: Ayanamsa::Lahiri,
        topocentric_moon: false,
    }
}

/// Sidereal (Lahiri) positions of the ten bodies and both nodes through the chart path,
/// against Swiss Ephemeris `FLG_SIDEREAL` (Sun, Moon) and SE tropical − SE ayanamsa (the
/// rest), at the twelve epochs 1950–2050 of `swisseph.json`.
#[test]
fn chart_sidereal_positions_match_swisseph_lahiri() {
    let Some(spk) = kernel() else { return };
    let g = swisseph();
    let keys = [
        ("sun", 2.0),
        ("moon", 15.0),
        ("mercury", 3.0),
        ("venus", 3.0),
        ("mars", 3.0),
        ("jupiter", 3.0),
        ("saturn", 3.0),
        ("uranus", 3.0),
        ("neptune", 3.0),
        ("pluto", 10.0),
    ];
    let mut worst = (0.0f64, String::new());
    for r in g["positions"].as_array().unwrap() {
        let label = r["utc"].as_str().unwrap();
        let t = instant_from_jd_tt(f(&r["jd_tt"]));
        let ch = compute(&spk, None, &input(t, 9.9312, 76.2673, HouseSystem::WholeSign)).unwrap();
        let sid = &r["sidereal_lahiri"];
        let se_aya = f(&sid["get_ayanamsa_ex_ut_deg"]);
        // the ayanamsa the chart subtracted (SE evaluates it at UT, Kshana at TT: the
        // ~1 minute between them is 0.001″ of precession)
        let da = wrap(ch.ayanamsa_deg - se_aya).abs() * 3_600.0;
        assert!(da < 0.5, "{label} ayanamsa {} vs {se_aya} ({da:.3}″)", ch.ayanamsa_deg);
        for (i, (key, tol)) in keys.iter().enumerate() {
            let b = &ch.bodies[i];
            assert_eq!(b.name.to_lowercase(), *key);
            let want = match sid["bodies"].get(*key) {
                Some(s) => f(&s["lon_deg"]),
                None => (f(&r["bodies"][key]["lon_deg"]) - se_aya).rem_euclid(360.0),
            };
            let d = wrap(b.sidereal - want).abs() * 3_600.0;
            let dt = wrap(b.tropical - f(&r["bodies"][key]["lon_deg"])).abs() * 3_600.0;
            if d / tol > worst.0 {
                worst = (d / tol, format!("{label} {key} {d:.2}″"));
            }
            assert!(d < *tol, "{label} {key} sidereal {} vs {want} ({d:.2}″)", b.sidereal);
            assert!(dt < *tol, "{label} {key} tropical ({dt:.2}″)");
            // sign, nakshatra and pada follow, unless the reference sits on a boundary
            let pada = 360.0 / 108.0;
            if (want / pada - (want / pada).round()).abs() * pada * 3_600.0 > *tol {
                let n = nakshatra(want);
                assert_eq!(b.nakshatra, (n.index, n.pada), "{label} {key}");
            }
        }
        // nodes: SE MEAN_NODE / TRUE_NODE (tropical) less the same ayanamsa
        let mn = (f(&r["bodies"]["mean_node"]["lon_deg"]) - se_aya).rem_euclid(360.0);
        let tn = (f(&r["bodies"]["true_node"]["lon_deg"]) - se_aya).rem_euclid(360.0);
        let dm = wrap(ch.mean_node.1 - mn).abs() * 3_600.0;
        let dn = wrap(ch.true_node.1 - tn).abs() * 3_600.0;
        assert!(dm < 5.0, "{label} sidereal mean node {dm:.1}″");
        // Moshier's osculating node vs DE421's: the 120″ of swisseph.rs
        assert!(dn < 120.0, "{label} sidereal true node {dn:.1}″");
    }
    eprintln!("worst relative to the Moshier tolerance: {:.2} at {}", worst.0, worst.1);
}

/// The ascendant, MC and twelve Placidus cusps (Porphyry inside the polar circle, where
/// Placidus is undefined) through the chart path at the twelve `houses` records:
/// Kochi, London, Reykjavik, Sydney, Quito, Christchurch, New York and Tromsø.
///
/// Swiss Ephemeris takes UT1 = UTC; the chart takes UT1 from the ΔT table. The chart is
/// therefore computed at the instant whose UT1 equals the record's `jd_ut`, so both turn
/// the same Earth; what remains is the house arithmetic and the sidereal time.
#[test]
fn chart_ascendant_and_cusps_match_swisseph() {
    let Some(spk) = kernel() else { return };
    let g = swisseph();
    let mut worst = (0.0f64, String::new());
    for r in g["houses"].as_array().unwrap() {
        let utc = r["utc"].as_str().unwrap();
        let place = r["place"].as_str().unwrap();
        let (lat, lon) = (f(&r["lat_deg"]), f(&r["lon_deg"]));
        let p = |a: usize, b: usize| utc[a..b].parse::<u32>().unwrap();
        let t0 = Utc::new(utc[0..4].parse().unwrap(), p(5, 7), p(8, 10), p(11, 13), p(14, 16), p(17, 19), 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let (code, sys) = match r["systems"]["P"].get("cusps_deg") {
            Some(_) => ("P", HouseSystem::Placidus),
            None => ("O", HouseSystem::Porphyry),
        };
        let first = compute(&spk, None, &input(t0, lat, lon, sys)).unwrap();
        let dut = (first.ut1.jd1 - f(&r["jd_ut"])) + first.ut1.jd2;
        let t = t0.add_ns(-(dut * NS_PER_DAY as f64).round() as i128);
        let ch = compute(&spk, None, &input(t, lat, lon, sys)).unwrap();
        let want = &r["systems"][code];
        let label = format!("{place} {utc} {code}");
        let darmc = wrap(ch.lst_deg - f(&want["armc_deg"])).abs() * 3_600.0;
        assert!(darmc < 0.5, "{label} ARMC {darmc:.3}″");
        let dasc = wrap(ch.cusps[0] - f(&want["ascendant_deg"])).abs() * 3_600.0;
        assert!(dasc < 1.0, "{label} ascendant {dasc:.3}″");
        for (i, c) in want["cusps_deg"].as_array().unwrap().iter().enumerate() {
            let d = wrap(ch.cusps[i] - f(c)).abs() * 3_600.0;
            if d > worst.0 {
                worst = (d, format!("{label} cusp {}", i + 1));
            }
            assert!(d < 1.0, "{label} cusp {}: {} vs {} ({d:.3}″)", i + 1, ch.cusps[i], f(c));
            let ds = wrap(ch.cusps_sidereal[i] - (f(c) - ch.ayanamsa_deg)).abs() * 3_600.0;
            assert!(ds < 1.0, "{label} sidereal cusp {} ({ds:.3}″)", i + 1);
        }
    }
    eprintln!("worst cusp difference {:.4}″ at {}", worst.0, worst.1);
}

// ---------------------------------------------------------------------------------
// Panchanga against Drik Panchang
// ---------------------------------------------------------------------------------

/// An IST wall-clock time.
fn ist(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> Instant {
    Utc::new(y, mo, d, h, mi, 0, 0).unwrap().to_instant().unwrap().add_ns(-330 * 60 * NS_PER_SEC)
}

/// (tithi 1–30, nakshatra 0–26, yoga 0–26, karana name) at `t`, Lahiri.
fn panchanga_at(spk: &Spk, t: Instant) -> (u8, usize, usize, &'static str) {
    let ay = ayanamsa_degrees(Ayanamsa::Lahiri, t.tt_centuries_since_j2000());
    let sun = sidereal(apparent(spk, Body::Sun, t).unwrap().longitude, ay);
    let moon = sidereal(apparent(spk, Body::Moon, t).unwrap().longitude, ay);
    let k = karana(moon, sun).name;
    let k = if k == "Gara" { "Garaja" } else { k }; // Drik spells it Garaja
    (tithi(moon, sun).number, nakshatra(moon).index, yoga(moon, sun).index, k)
}

struct DrikDay {
    place: &'static str,
    obs: Observer,
    date: (i64, u32, u32),
    sunrise: (u32, u32),
    /// (tithi, nakshatra, yoga, karana) at sunrise
    at_sunrise: (u8, usize, usize, &'static str),
    /// published ends, IST: what runs just before and just after each
    ends: Vec<(&'static str, (i64, u32, u32, u32, u32))>,
}

/// Published Drik Panchang day pages. Their place is the page's geoname; times are IST,
/// to the minute.
fn drik_days() -> Vec<DrikDay> {
    vec![
        // drikpanchang.com/panchang/day-panchang.html?geoname-id=1273874&date=15/01/2025
        // "Cochin, Kerala, 09°56′23″N 76°15′36″E": sunrise 06:46; Krishna Dwitiya until
        // 03:23 (Jan 16); Pushya until 10:28; Priti until 01:47 (Jan 16); Taitila until
        // 15:17, Garaja until 03:23 (Jan 16).
        DrikDay {
            place: "Kochi",
            obs: Observer { latitude_deg: 9.0 + 56.0 / 60.0 + 23.0 / 3600.0, longitude_deg: 76.26, height_m: 0.0 },
            date: (2025, 1, 15),
            sunrise: (6, 46),
            at_sunrise: (17, 7, 1, "Taitila"),
            ends: vec![
                ("tithi", (2025, 1, 16, 3, 23)),
                ("nakshatra", (2025, 1, 15, 10, 28)),
                ("yoga", (2025, 1, 16, 1, 47)),
                ("karana", (2025, 1, 15, 15, 17)),
                ("karana", (2025, 1, 16, 3, 23)),
            ],
        },
        // drikpanchang.com/panchang/day-panchang.html?geoname-id=1261481&date=31/10/2024
        // New Delhi: sunrise 06:32; Krishna Chaturdashi until 15:52; Chitra until 00:45
        // (Nov 1); Vishkambha until 09:51; Shakuni until 15:52, Chatushpada until 05:06
        // (Nov 1).
        DrikDay {
            place: "New Delhi",
            obs: Observer { latitude_deg: 28.6139, longitude_deg: 77.2090, height_m: 0.0 },
            date: (2024, 10, 31),
            sunrise: (6, 32),
            at_sunrise: (29, 13, 0, "Shakuni"),
            ends: vec![
                ("tithi", (2024, 10, 31, 15, 52)),
                ("nakshatra", (2024, 11, 1, 0, 45)),
                ("yoga", (2024, 10, 31, 9, 51)),
                ("karana", (2024, 10, 31, 15, 52)),
                ("karana", (2024, 11, 1, 5, 6)),
            ],
        },
    ]
}

#[test]
fn panchanga_at_sunrise_and_end_times_match_drik_panchang() {
    let Some(spk) = kernel() else { return };
    for d in drik_days() {
        let date = Date::new(d.date.0, d.date.1, d.date.2);
        let cal = Calendar::new(&spk, None, d.obs, Config::india(), date, 1).unwrap();
        let rise = cal.days[cal.index_of(date).unwrap()].sunrise;
        let want = ist(d.date.0, d.date.1, d.date.2, d.sunrise.0, d.sunrise.1);
        // Drik prints the minute; allow the rounding plus the place's few hundred metres
        let off = rise.ns_since(want) as f64 / 1e9;
        assert!((-60.0..=90.0).contains(&off), "{} sunrise off by {off:.0} s", d.place);
        assert_eq!(panchanga_at(&spk, rise), d.at_sunrise, "{} at sunrise", d.place);
        for (what, (y, mo, dd, h, mi)) in &d.ends {
            let end = ist(*y, *mo, *dd, *h, *mi);
            // Drik's minute is the minute the element ends in; 90 s either side holds
            // the old one before and the new one after.
            let before = panchanga_at(&spk, end.add_ns(-90 * NS_PER_SEC));
            let after = panchanga_at(&spk, end.add_ns(90 * NS_PER_SEC));
            let changed = match *what {
                "tithi" => before.0 != after.0,
                "nakshatra" => before.1 != after.1,
                "yoga" => before.2 != after.2,
                _ => before.3 != after.3,
            };
            assert!(changed, "{} {what} should end at {h:02}:{mi:02} IST: {before:?} → {after:?}", d.place);
        }
    }
    // Mumbai, 19 March 2026 (kshana-calendar's `eras_and_cycles_through_a_year`, published
    // times): Amavasya at sunrise ends 06:53 IST and Shukla Pratipada ends 04:52 on the
    // 20th, before that day's sunrise.
    for (y, mo, dd, h, mi, from, to) in [(2026, 3, 19, 6, 53, 30, 1), (2026, 3, 20, 4, 52, 1, 2)] {
        let end = ist(y, mo, dd, h, mi);
        assert_eq!(panchanga_at(&spk, end.add_ns(-90 * NS_PER_SEC)).0, from);
        assert_eq!(panchanga_at(&spk, end.add_ns(90 * NS_PER_SEC)).0, to);
    }
}
