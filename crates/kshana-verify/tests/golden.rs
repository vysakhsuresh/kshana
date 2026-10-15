//! Compare Kshana against the recorded reference values.

use kshana_ephem::{Body, Spk};
use kshana_frames::{
    earth_rotation_angle, gmst_iau2006, mean_obliquity_iau2006, nutation_iau2000b,
    nutation_iau2006a, ARCSEC,
};
use kshana_sky::precession::{apply, bias_precession_nutation_matrix, fw_angles};
use kshana_sky::{apparent, apparent_topocentric, Observer};
use kshana_time::{Instant, TwoPartJd, NS_PER_DAY};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load(name: &str) -> Value {
    let p = root().join("tests/golden").join(name);
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&text).expect("json")
}

fn kernel() -> Option<Spk> {
    Spk::open(&root().join("data/de421.bsp")).ok()
}

fn jd2(v: &Value) -> TwoPartJd {
    TwoPartJd {
        jd1: v[0].as_f64().unwrap(),
        jd2: v[1].as_f64().unwrap(),
    }
}

/// Instant from a TT two-part JD, exact to the nanosecond.
fn instant_from_tt(jd: TwoPartJd) -> Instant {
    let days = (jd.jd1 - 2_451_545.0) + jd.jd2;
    let whole = days.floor();
    let frac = days - whole;
    let ns = whole as i128 * NS_PER_DAY + (frac * NS_PER_DAY as f64).round() as i128;
    Instant::j2000().add_ns(ns)
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn wrap_deg(d: f64) -> f64 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

#[test]
fn frames_match_erfa() {
    let g = load("frames.json");
    let recs = g["records"].as_array().unwrap();
    assert_eq!(recs.len(), 24);
    let mut worst_nut = 0.0f64;
    for r in recs {
        let tt = jd2(&r["tt_jd"]);
        let t = tt.centuries_since_j2000();
        let ut1 = jd2(&r["ut1_jd"]);
        let label = r["utc"].as_str().unwrap();

        let era = earth_rotation_angle(ut1);
        assert!(
            (era - f(&r["era00_rad"])).abs() < 1e-12,
            "{label}: ERA {era} vs {}",
            f(&r["era00_rad"])
        );

        let gmst = gmst_iau2006(era, t);
        assert!((gmst - f(&r["gmst06_rad"])).abs() < 1e-12, "{label}: GMST");

        let obl = mean_obliquity_iau2006(t);
        assert!(
            (obl - f(&r["obl06_rad"])).abs() < 1e-15,
            "{label}: obliquity"
        );

        let (gamb, phib, psib) = fw_angles(t);
        let p = &r["pfw06_rad"];
        assert!((gamb - f(&p["gamb"])).abs() < 1e-15, "{label}: gamb");
        assert!((phib - f(&p["phib"])).abs() < 1e-15, "{label}: phib");
        assert!((psib - f(&p["psib"])).abs() < 1e-15, "{label}: psib");

        // the matrix, fed ERFA's own nutation so only the rotation algebra is tested
        let n = &r["nut06a_rad"];
        let m = bias_precession_nutation_matrix(t, f(&n["dpsi"]), f(&n["deps"]));
        for i in 0..3 {
            for j in 0..3 {
                let want = f(&r["pnm06a"][i][j]);
                assert!(
                    (m[i][j] - want).abs() < 1e-14,
                    "{label}: pnm06a[{i}][{j}] {} vs {want}",
                    m[i][j]
                );
            }
        }

        // nutation: full series must reproduce ERFA to rounding
        let (dpsi, deps) = nutation_iau2006a(t);
        let e = ((dpsi - f(&n["dpsi"])).abs()).max((deps - f(&n["deps"])).abs());
        worst_nut = worst_nut.max(e);
        assert!(e < 1e-13, "{label}: nut06a off by {} rad", e);
        let (dpsi_b, deps_b) = nutation_iau2000b(t);
        let nb = &r["nut00b_rad"];
        let eb = ((dpsi_b - f(&nb["dpsi"])).abs()).max((deps_b - f(&nb["deps"])).abs());
        assert!(eb < 1e-13, "{label}: nut00b off by {} rad", eb);
        let gst = kshana_frames::gast(era, t);
        assert!(
            (gst - f(&r["gst06a_rad"])).abs() < 1e-12,
            "{label}: GAST {gst} vs {}",
            f(&r["gst06a_rad"])
        );
    }
    eprintln!("worst nutation error: {:.4}″", worst_nut / ARCSEC);
}

#[test]
fn spk_states_match_jplephem() {
    let Some(spk) = kernel() else {
        eprintln!("skipped: data/de421.bsp missing");
        return;
    };
    let g = load("states.json");
    let recs = g["records"].as_array().unwrap();
    assert_eq!(recs.len(), 12);
    let pairs = [
        ("399_rel_3", Body::Earth, Body::EarthMoonBarycenter),
        ("301_rel_3", Body::Moon, Body::EarthMoonBarycenter),
        (
            "3_rel_0",
            Body::EarthMoonBarycenter,
            Body::SolarSystemBarycenter,
        ),
        ("10_rel_0", Body::Sun, Body::SolarSystemBarycenter),
        ("1_rel_0", Body::Mercury, Body::SolarSystemBarycenter),
        ("2_rel_0", Body::Venus, Body::SolarSystemBarycenter),
        ("4_rel_0", Body::MarsBarycenter, Body::SolarSystemBarycenter),
        (
            "5_rel_0",
            Body::JupiterBarycenter,
            Body::SolarSystemBarycenter,
        ),
        (
            "6_rel_0",
            Body::SaturnBarycenter,
            Body::SolarSystemBarycenter,
        ),
        (
            "7_rel_0",
            Body::UranusBarycenter,
            Body::SolarSystemBarycenter,
        ),
        (
            "8_rel_0",
            Body::NeptuneBarycenter,
            Body::SolarSystemBarycenter,
        ),
        (
            "9_rel_0",
            Body::PlutoBarycenter,
            Body::SolarSystemBarycenter,
        ),
    ];
    for r in recs {
        let et = f(&r["et_s"]);
        for (key, target, center) in pairs {
            let want = &r["segments"][key];
            let st = spk.state(target, center, et).unwrap();
            for k in 0..3 {
                let dp = (st.position[k] - f(&want["position_km"][k])).abs();
                let dv = (st.velocity[k] - f(&want["velocity_km_s"][k])).abs();
                assert!(dp < 1e-4, "et {et} {key} pos[{k}] off by {dp} km");
                assert!(dv < 1e-9, "et {et} {key} vel[{k}] off by {dv} km/s");
            }
        }
    }
}

#[test]
fn apparent_positions_match_skyfield() {
    let Some(spk) = kernel() else {
        return;
    };
    let g = load("apparent.json");
    let recs = g["records"].as_array().unwrap();
    assert_eq!(recs.len(), 20);
    let bodies = [
        ("sun", Body::Sun),
        ("moon", Body::Moon),
        ("mercury", Body::Mercury),
        ("venus", Body::Venus),
        ("mars_barycenter", Body::MarsBarycenter),
        ("jupiter_barycenter", Body::JupiterBarycenter),
        ("saturn_barycenter", Body::SaturnBarycenter),
        ("uranus_barycenter", Body::UranusBarycenter),
        ("neptune_barycenter", Body::NeptuneBarycenter),
        ("pluto_barycenter", Body::PlutoBarycenter),
    ];
    // Budget: no gravitational deflection yet (≤1.75″ at the solar limb, usually
    // < 0.02″), truncated TDB−TT (≈30 µs, negligible). 0.5″.
    let tol_deg = 0.5 / 3_600.0;
    let mut worst = (0.0f64, String::new());
    for r in recs {
        let t = instant_from_tt(jd2(&r["tt_jd"]));
        let label = r["utc"].as_str().unwrap();
        for (key, body) in bodies {
            let want = &r["geocentric"][key];
            let a = apparent(&spk, body, t).unwrap();
            let dl = wrap_deg(a.longitude - f(&want["lon_deg"])).abs()
                * f(&want["lat_deg"]).to_radians().cos();
            let db = (a.latitude - f(&want["lat_deg"])).abs();
            let sep = (dl * dl + db * db).sqrt();
            if sep > worst.0 {
                worst = (sep, format!("{label} {key}"));
            }
            assert!(
                sep < tol_deg,
                "{label} {key}: {sep:.6}° off ({:.2}″)",
                sep * 3_600.0
            );
            let dd = (a.distance_km - f(&want["distance_km"])).abs();
            assert!(dd < 2.0, "{label} {key}: distance off by {dd} km");
        }
    }
    eprintln!(
        "worst apparent separation: {:.3}″ at {}",
        worst.0 * 3_600.0,
        worst.1
    );
}

#[test]
fn topocentric_moon_matches_skyfield() {
    let Some(spk) = kernel() else {
        return;
    };
    let g = load("apparent.json");
    let obs = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };
    let tol_deg = 0.5 / 3_600.0;
    for r in g["records"].as_array().unwrap() {
        let t = instant_from_tt(jd2(&r["tt_jd"]));
        let label = r["utc"].as_str().unwrap();
        let ut1 = jd2(&r["skyfield_ut1_jd"]);
        let tc = jd2(&r["tt_jd"]).centuries_since_j2000();
        let gast = kshana_frames::gast(earth_rotation_angle(ut1), tc);
        let want = &r["moon_topocentric_kochi"];
        let a = apparent_topocentric(&spk, Body::Moon, t, &obs, gast).unwrap();
        let dl = wrap_deg(a.longitude - f(&want["lon_deg"])).abs()
            * f(&want["lat_deg"]).to_radians().cos();
        let db = (a.latitude - f(&want["lat_deg"])).abs();
        let sep = (dl * dl + db * db).sqrt();
        assert!(
            sep < tol_deg,
            "{label}: topocentric Moon {sep:.6}° off ({:.2}″)",
            sep * 3_600.0
        );
    }
    let _ = apply;
}
