//! Compare Kshana against Swiss Ephemeris (Moshier mode) reference values.

use kshana_ephem::{Body, Spk};
use kshana_houses::{cusps, HouseSystem};
use kshana_sky::{apparent, Observer};
use kshana_time::{Instant, TwoPartJd, NS_PER_DAY};
use kshana_vedic::{ayanamsa_degrees, Ayanamsa};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load() -> Value {
    let p = root().join("tests/golden/swisseph.json");
    serde_json::from_str(&std::fs::read_to_string(&p).expect("swisseph.json")).expect("json")
}

fn kernel() -> Option<Spk> {
    Spk::open(&root().join("data/de421.bsp")).ok()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn instant_from_jd_tt(jd: f64) -> Instant {
    let days = jd - 2_451_545.0;
    let whole = days.floor();
    let frac = days - whole;
    Instant::j2000().add_ns(whole as i128 * NS_PER_DAY + (frac * NS_PER_DAY as f64).round() as i128)
}

fn wrap(d: f64) -> f64 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

#[test]
fn house_cusps_match_swisseph_for_ten_systems() {
    let g = load();
    let systems = [
        ("P", HouseSystem::Placidus),
        ("K", HouseSystem::Koch),
        ("R", HouseSystem::Regiomontanus),
        ("C", HouseSystem::Campanus),
        ("B", HouseSystem::Alcabitius),
        ("M", HouseSystem::Morinus),
        ("T", HouseSystem::Topocentric),
        ("O", HouseSystem::Porphyry),
        ("E", HouseSystem::Equal),
        ("W", HouseSystem::WholeSign),
    ];
    let mut worst = (0.0f64, String::new());
    let mut compared = 0;
    for r in g["houses"].as_array().unwrap() {
        let lat = f(&r["lat_deg"]).to_radians();
        let eps = f(&r["obliquity"]["true_obliquity_deg"]).to_radians();
        let label = format!(
            "{} {}",
            r["place"].as_str().unwrap(),
            r["utc"].as_str().unwrap()
        );
        for (code, sys) in systems {
            let want = &r["systems"][code];
            let armc = (f(&r["sidtime_hours"]) * 15.0 + f(&r["lon_deg"]))
                .rem_euclid(360.0)
                .to_radians();
            if want.get("error").is_some()
                || want
                    .get("undefined_at_latitude")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                assert!(
                    cusps(sys, armc, eps, lat).is_err(),
                    "{label} {code}: swisseph says undefined, we returned cusps"
                );
                continue;
            }
            let got =
                cusps(sys, armc, eps, lat).unwrap_or_else(|e| panic!("{label} {code}: {e:?}"));
            let want_c = want["cusps_deg"].as_array().unwrap();
            for i in 0..12 {
                let d = wrap(got[i] - f(&want_c[i])).abs() * 3_600.0;
                if d > worst.0 {
                    worst = (d, format!("{label} {code} cusp {}", i + 1));
                }
                assert!(
                    d < 1.0,
                    "{label} {code} cusp {}: {} vs {} ({d:.3}″)",
                    i + 1,
                    got[i],
                    f(&want_c[i])
                );
            }
            compared += 1;
        }
    }
    eprintln!(
        "compared {compared} house sets; worst {:.4}″ at {}",
        worst.0, worst.1
    );
}

#[test]
fn positions_match_swisseph_moshier() {
    let Some(spk) = kernel() else { return };
    let g = load();
    let bodies = [
        ("sun", Body::Sun, 2.0),
        ("moon", Body::Moon, 15.0),
        ("mercury", Body::Mercury, 3.0),
        ("venus", Body::Venus, 3.0),
        ("mars", Body::MarsBarycenter, 3.0),
        ("jupiter", Body::JupiterBarycenter, 3.0),
        ("saturn", Body::SaturnBarycenter, 3.0),
        ("uranus", Body::UranusBarycenter, 3.0),
        ("neptune", Body::NeptuneBarycenter, 3.0),
        ("pluto", Body::PlutoBarycenter, 10.0),
    ];
    let mut worst = (0.0f64, String::new());
    for r in g["positions"].as_array().unwrap() {
        let t = instant_from_jd_tt(f(&r["jd_tt"]));
        let label = r["utc"].as_str().unwrap();
        for (key, body, tol) in bodies {
            let want = &r["bodies"][key];
            let a = apparent(&spk, body, t).unwrap();
            let dl = wrap(a.longitude - f(&want["lon_deg"])).abs() * 3_600.0;
            let db = (a.latitude - f(&want["lat_deg"])).abs() * 3_600.0;
            let d = dl.max(db);
            if d / tol > worst.0 {
                worst = (d / tol, format!("{label} {key} {d:.2}″"));
            }
            assert!(
                d < tol,
                "{label} {key}: {dl:.2}″ lon, {db:.2}″ lat (Moshier tolerance {tol}″)"
            );
            let ds = (a.speed - f(&want["lon_speed_deg_per_day"])).abs();
            assert!(
                ds < 0.01,
                "{label} {key}: speed {} vs {}",
                a.speed,
                f(&want["lon_speed_deg_per_day"])
            );
        }
        // true node
        let tn = kshana_sky::true_node_longitude(&spk, t).unwrap();
        let want_tn = f(&r["bodies"]["true_node"]["lon_deg"]);
        let d = wrap(tn - want_tn).abs() * 3_600.0;
        assert!(d < 120.0, "{label} true node: {tn} vs {want_tn} ({d:.1}″)");
        // mean node: SE_MEAN_NODE is on the true equinox of date (it carries Δψ). Before the
        // nutation term was added Kshana's mean node was off by up to 17″ here.
        let mn = kshana_sky::mean_node_longitude(t.tt_centuries_since_j2000());
        let want_mn = f(&r["bodies"]["mean_node"]["lon_deg"]);
        let d = wrap(mn - want_mn).abs() * 3_600.0;
        assert!(d < 5.0, "{label} mean node: {mn} vs {want_mn} ({d:.1}″)");
    }
    eprintln!("worst relative to tolerance: {:.2} at {}", worst.0, worst.1);
}

#[test]
fn sunrise_matches_swisseph_default_convention() {
    let Some(spk) = kernel() else { return };
    let g = load();
    for r in g["sunrise"].as_array().unwrap() {
        let obs = Observer {
            latitude_deg: f(&r["lat_deg"]),
            longitude_deg: f(&r["lon_deg"]),
            height_m: 0.0,
        };
        let start = instant_from_jd_tt(f(&r["search_start_jd_ut"]) + 69.0 / 86_400.0);
        let end = start.add_ns(NS_PER_DAY);
        let (rises, sets) =
            kshana_events::sun_rise_set(&spk, None, &obs, start, end, 1_000_000).unwrap();
        for (key, list) in [("rise_default", &rises), ("set_default", &sets)] {
            let want_jd = f(&r["events"][key]["jd_ut"]);
            // swisseph's jd_ut is UTC; our instants are TAI, so compare via TT with ΔT ≈ 69 s
            let want = instant_from_jd_tt(want_jd + 69.2 / 86_400.0);
            let best = list
                .iter()
                .map(|t| t.ns_since(want).abs())
                .min()
                .unwrap_or(i128::MAX);
            assert!(
                best < 20 * 1_000_000_000,
                "{} {} {key}: off by {:.1} s",
                r["place"],
                r["date_utc"],
                best as f64 / 1e9
            );
        }
    }
}

#[test]
fn ayanamsa_matches_swisseph_true_equinox() {
    let g = load();
    let modes = [
        ("SIDM_LAHIRI", Ayanamsa::Lahiri),
        ("SIDM_RAMAN", Ayanamsa::Raman),
        ("SIDM_KRISHNAMURTI", Ayanamsa::Krishnamurti),
        ("SIDM_FAGAN_BRADLEY", Ayanamsa::FaganBradley),
    ];
    let mut worst = (0.0f64, String::new());
    for r in g["ayanamsa"].as_array().unwrap() {
        let jd = TwoPartJd {
            jd1: f(&r["jd_tt"]).floor() + 0.5,
            jd2: f(&r["jd_tt"]) - (f(&r["jd_tt"]).floor() + 0.5),
        };
        let t = jd.centuries_since_j2000();
        let label = r["utc"].as_str().unwrap();
        for (key, mode) in modes {
            let want = f(&r["ayanamsa"][key]["get_ayanamsa_ex_ut_deg"]);
            let got = ayanamsa_degrees(mode, t);
            let d = wrap(got - want).abs() * 3_600.0;
            if d > worst.0 {
                worst = (d, format!("{label} {key}"));
            }
            assert!(d < 0.5, "{label} {key}: {got} vs {want} ({d:.3}″)");
        }
    }
    eprintln!("worst ayanamsa difference {:.4}″ at {}", worst.0, worst.1);
}

/// Every ayanamsa Kshana offers against Swiss Ephemeris 2.10.03 with its own ephemeris
/// files (`tests/golden/swisseph_ayanamsa.json`, made by `scripts/golden/swisseph_oracle.c`),
/// 17 epochs 1900–2050, true equinox of date. Fixed zero points are carried from their
/// J2000 anchors by IAU 2006/2000A precession-nutation; the star-based ones (True
/// Chitra, True Revati, True Pushya) are computed from the star's apparent place.
#[test]
fn every_ayanamsa_matches_swisseph_1900_to_2050() {
    let p = root().join("tests/golden/swisseph_ayanamsa.json");
    let g: Value = serde_json::from_str(&std::fs::read_to_string(&p).expect("json")).unwrap();
    let mut worst = vec![(0.0f64, String::new()); Ayanamsa::ALL.len()];
    for r in g["ayanamsa"].as_array().unwrap() {
        let jd_tt = f(&r["jd_tt"]);
        let jd = TwoPartJd {
            jd1: jd_tt.floor() + 0.5,
            jd2: jd_tt - (jd_tt.floor() + 0.5),
        };
        let t = jd.centuries_since_j2000();
        let label = r["utc"].as_str().unwrap();
        for (i, mode) in Ayanamsa::ALL.into_iter().enumerate() {
            let key = mode.swiss_ephemeris_mode().to_string();
            let want = f(&r[key.as_str()]);
            let got = ayanamsa_degrees(mode, t);
            let d = wrap(got - want).abs() * 3_600.0;
            if d > worst[i].0 {
                worst[i] = (d, label.to_string());
            }
            // measured worst: 0.0014″ (Lahiri), 0.016″ (the three stars, Meeus Earth
            // velocity), 0.021″ (VP285 / VP291: SE measures precession on the ecliptic of
            // a t0 seventeen centuries back)
            assert!(d < 0.05, "{label} {mode:?}: {got} vs {want} ({d:.4}″)");
        }
    }
    for (mode, (d, at)) in Ayanamsa::ALL.iter().zip(worst) {
        eprintln!("{mode:?}: worst {d:.4}″ at {at}");
    }
}
