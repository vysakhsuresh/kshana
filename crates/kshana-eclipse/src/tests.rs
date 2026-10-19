//! Kernel-backed checks against NASA's published circumstances.
use super::*;
use kshana_sky::apparent;
use kshana_solver::wrap180;
use kshana_time::Utc;

fn kernel() -> Option<Spk> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
    Spk::open(&p).ok()
}

fn eop() -> Option<EopTable> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/finals2000A.all");
    std::fs::read_to_string(p).ok().map(|s| EopTable::parse(&s))
}

fn utc(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Instant {
    Utc::new(y, mo, d, h, mi, s, 0)
        .unwrap()
        .to_instant()
        .unwrap()
}

fn seconds_from(t: Instant, want: Instant) -> f64 {
    t.ns_since(want) as f64 / 1e9
}

fn show(label: &str, t: Instant) {
    let u = Utc::from_instant(t);
    eprintln!(
        "{label}: {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} UTC",
        u.year,
        u.month,
        u.day,
        u.hour,
        u.minute,
        u.second,
        u.nanos / 1_000_000
    );
}

#[test]
fn lunation_and_saros_closed_forms_match_nasa_anchors() {
    // Lunation 0 is the new Moon of 2000 Jan 6 18:14 UTC.
    assert_eq!(lunation_number(utc(2000, 1, 6, 18, 14, 0)), 0);
    // Solar: 2017 Aug 21 → 145, 2019 Dec 26 → 132, 2024 Apr 8 → 139, 2026 Aug 12 → 126.
    assert_eq!(
        solar_saros(lunation_number(utc(2017, 8, 21, 18, 30, 0))),
        145
    );
    assert_eq!(
        solar_saros(lunation_number(utc(2019, 12, 26, 5, 17, 0))),
        132
    );
    assert_eq!(
        solar_saros(lunation_number(utc(2024, 4, 8, 18, 17, 0))),
        139
    );
    assert_eq!(
        solar_saros(lunation_number(utc(2026, 8, 12, 17, 46, 0))),
        126
    );
    // Lunar (lunation of the preceding new Moon): 2000 Jan 21 → 124, 2019 Jan 21 → 134,
    // 2022 Nov 8 → 136.
    assert_eq!(
        lunar_saros(lunation_number(utc(2000, 1, 6, 18, 14, 0))),
        124
    );
    assert_eq!(lunar_saros(lunation_number(utc(2019, 1, 6, 1, 28, 0))), 134);
    assert_eq!(
        lunar_saros(lunation_number(utc(2022, 10, 25, 10, 49, 0))),
        136
    );
    // Meeus's example 49.a: the mean new Moon of lunation −283 (1977 Feb 18) is
    // JDE 2443192.94102 (the true one, after his periodic terms, 2443192.65118).
    let jde = mean_phase_jde(-283.0);
    assert!((jde - 2_443_192.941_02).abs() < 1e-4, "{jde}");
}

#[test]
fn obscuration_limits() {
    assert_eq!(obscuration(1.0, 1.0, 2.5), 0.0);
    assert_eq!(obscuration(1.0, 1.1, 0.0), 1.0);
    assert!((obscuration(1.0, 0.5, 0.0) - 0.25).abs() < 1e-12);
    // equal discs, centres one radius apart: lens = 2π/3 − √3/2 of a disc's area
    let want = (2.0 * std::f64::consts::PI / 3.0 - 3f64.sqrt() / 2.0) / std::f64::consts::PI;
    assert!((obscuration(1.0, 1.0, 1.0) - want).abs() < 1e-12);
}

#[test]
fn total_solar_eclipse_2024_04_08_matches_nasa() {
    let Some(spk) = kernel() else { return };
    let v = solar_eclipses(&spk, utc(2024, 4, 1, 0, 0, 0), utc(2024, 4, 15, 0, 0, 0)).unwrap();
    assert_eq!(v.len(), 1, "{v:?}");
    let e = &v[0];
    show("2024-04-08 greatest", e.greatest);
    show("2024-04-08 P1", e.begins);
    show("2024-04-08 P4", e.ends);
    eprintln!(
        "2024-04-08 kind {:?} gamma {:.5} magnitude {:.5} saros {:?} central {}",
        e.kind, e.gamma, e.magnitude_max, e.saros, e.central
    );
    // NASA: greatest 18:18:29 TD = 18:17:20 UT (ΔT 69 s), gamma 0.3431, magnitude 1.0566,
    // Saros 139, total.
    assert_eq!(e.kind, SolarKind::Total);
    assert!(e.central);
    let off = seconds_from(e.greatest, utc(2024, 4, 8, 18, 17, 20));
    assert!(off.abs() < 120.0, "greatest off by {off} s");
    assert!((e.gamma - 0.3431).abs() < 0.002, "gamma {}", e.gamma);
    assert!(
        (e.magnitude_max - 1.0566).abs() < 0.002,
        "magnitude {}",
        e.magnitude_max
    );
    assert_eq!(e.saros, Some(139));
    // NASA: penumbra first contact 15:42:15 UT, last 20:52:19 UT.
    let p1 = seconds_from(e.begins, utc(2024, 4, 8, 15, 42, 15));
    let p4 = seconds_from(e.ends, utc(2024, 4, 8, 20, 52, 19));
    assert!(p1.abs() < 120.0 && p4.abs() < 120.0, "P1 {p1} s, P4 {p4} s");
}

#[test]
fn total_lunar_eclipse_2022_11_08_matches_nasa() {
    let Some(spk) = kernel() else { return };
    let v = lunar_eclipses(
        &spk,
        utc(2022, 11, 1, 0, 0, 0),
        utc(2022, 11, 15, 0, 0, 0),
        ShadowRule::Danjon,
    )
    .unwrap();
    assert_eq!(v.len(), 1, "{v:?}");
    let e = &v[0];
    show("2022-11-08 greatest", e.greatest);
    for (i, c) in e.contacts.iter().enumerate() {
        if let Some(t) = c {
            show(["P1", "U1", "U2", "U3", "U4", "P4"][i], *t);
        }
    }
    eprintln!(
        "2022-11-08 kind {:?} gamma {:.5} pen {:.5} umb {:.5} saros {:?}",
        e.kind, e.gamma, e.pen_magnitude, e.umb_magnitude, e.saros
    );
    // NASA (eclipse.gsfc.nasa.gov, ΔT 72 s assumed): P1 08:02:17, U1 09:09:12, U2 10:16:39,
    // greatest 10:59:11, U3 11:41:36, U4 12:49:03, P4 13:56:09 UT; umbral magnitude 1.3589;
    // gamma +0.2570 (the Moon north of the axis); Saros 136. EclipseWise's later
    // predictions (umbral magnitude 1.3607) lengthen each phase by about 40 s.
    assert_eq!(e.kind, LunarKind::Total);
    assert_eq!(e.saros, Some(136));
    let off = seconds_from(e.greatest, utc(2022, 11, 8, 10, 59, 11));
    assert!(off.abs() < 120.0, "greatest off by {off} s");
    assert!(
        (e.umb_magnitude - 1.3589).abs() < 0.02,
        "umbral {}",
        e.umb_magnitude
    );
    assert!((e.gamma - 0.2570).abs() < 0.003, "gamma {}", e.gamma);
    let want = [
        utc(2022, 11, 8, 8, 2, 17),
        utc(2022, 11, 8, 9, 9, 12),
        utc(2022, 11, 8, 10, 16, 39),
        utc(2022, 11, 8, 11, 41, 36),
        utc(2022, 11, 8, 12, 49, 3),
        utc(2022, 11, 8, 13, 56, 9),
    ];
    for (i, w) in want.iter().enumerate() {
        let t = e.contacts[i].expect("all six contacts of a total eclipse");
        let off = seconds_from(t, *w);
        assert!(off.abs() < 120.0, "contact {i} off by {off} s");
    }
    for w in e.contacts.windows(2) {
        assert!(
            w[0].unwrap() < w[1].unwrap(),
            "contacts out of order: {:?}",
            e.contacts
        );
    }
    assert!(e.contacts[2].unwrap() < e.greatest && e.greatest < e.contacts[3].unwrap());
    // Chauvenet's enlargement raises the umbral magnitude by about 0.006 (NASA).
    let c = lunar_eclipses(
        &spk,
        utc(2022, 11, 1, 0, 0, 0),
        utc(2022, 11, 15, 0, 0, 0),
        ShadowRule::Chauvenet,
    )
    .unwrap();
    let d = c[0].umb_magnitude - e.umb_magnitude;
    assert!(
        d > 0.003 && d < 0.012,
        "Chauvenet − Danjon umbral magnitude {d}"
    );
}

#[test]
fn total_solar_eclipse_2026_08_12_exists() {
    let Some(spk) = kernel() else { return };
    let v = solar_eclipses(&spk, utc(2026, 8, 1, 0, 0, 0), utc(2026, 8, 31, 0, 0, 0)).unwrap();
    assert_eq!(v.len(), 1, "{v:?}");
    let e = &v[0];
    show("2026-08-12 greatest", e.greatest);
    eprintln!(
        "2026-08-12 kind {:?} gamma {:.5} magnitude {:.5} saros {:?}",
        e.kind, e.gamma, e.magnitude_max, e.saros
    );
    // NASA Besselian elements: greatest 17:45:53.8 UT, gamma 0.8978, magnitude 1.0386,
    // Saros 126.
    assert_eq!(e.kind, SolarKind::Total);
    let off = seconds_from(e.greatest, utc(2026, 8, 12, 17, 45, 54));
    assert!(off.abs() < 120.0, "greatest off by {off} s");
    assert!((e.gamma - 0.8978).abs() < 0.002, "gamma {}", e.gamma);
    assert!(
        (e.magnitude_max - 1.0386).abs() < 0.002,
        "magnitude {}",
        e.magnitude_max
    );
    assert_eq!(e.saros, Some(126));
}

#[test]
fn annular_eclipse_2019_12_26_local_circumstances_at_kochi() {
    let Some(spk) = kernel() else { return };
    let eop = eop();
    let v = solar_eclipses(&spk, utc(2019, 12, 20, 0, 0, 0), utc(2019, 12, 31, 0, 0, 0)).unwrap();
    assert_eq!(v.len(), 1, "{v:?}");
    let e = &v[0];
    eprintln!(
        "2019-12-26 kind {:?} gamma {:.5} magnitude {:.5} saros {:?}",
        e.kind, e.gamma, e.magnitude_max, e.saros
    );
    assert_eq!(e.kind, SolarKind::Annular);
    assert_eq!(e.saros, Some(132));
    // NASA: gamma 0.4135, magnitude 0.9701.
    assert!((e.gamma - 0.4135).abs() < 0.002, "gamma {}", e.gamma);
    assert!(
        (e.magnitude_max - 0.9701).abs() < 0.002,
        "magnitude {}",
        e.magnitude_max
    );
    let kochi = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };
    let l = local_solar(&spk, eop.as_ref(), &kochi, e)
        .unwrap()
        .expect("Kochi saw the eclipse");
    show("Kochi C1", l.c1);
    show("Kochi max", l.max);
    show("Kochi C4", l.c4);
    eprintln!(
        "Kochi magnitude {:.4} obscuration {:.4} altitude {:.2} C2 {:?} C3 {:?}",
        l.magnitude, l.obscuration, l.altitude_at_max, l.c2, l.c3
    );
    // Positional Astronomy Centre (PIB appendix) for Cochin: partial begins 08:06:08 IST,
    // greatest 09:28:29 IST, ends 11:08:58 IST, obscuration 91.0 %, no annular phase
    // (IST = UTC + 5:30).
    let c1 = seconds_from(l.c1, utc(2019, 12, 26, 2, 36, 8));
    let mx = seconds_from(l.max, utc(2019, 12, 26, 3, 58, 29));
    let c4 = seconds_from(l.c4, utc(2019, 12, 26, 5, 38, 58));
    assert!(c1.abs() < 180.0, "C1 off by {c1} s");
    assert!(mx.abs() < 180.0, "max off by {mx} s");
    assert!(c4.abs() < 180.0, "C4 off by {c4} s");
    assert!(
        (l.obscuration - 0.91).abs() < 0.02,
        "obscuration {}",
        l.obscuration
    );
    assert!(l.c2.is_none() && l.c3.is_none());
    assert!(l.altitude_at_max > 0.0 && l.visible);
    assert!(l.c1 < l.max && l.max < l.c4);
    // A site that saw annularity: Coimbatore (PIB: annular 09:27:47–09:30:56 IST).
    let coimbatore = Observer {
        latitude_deg: 11.0168,
        longitude_deg: 76.9558,
        height_m: 410.0,
    };
    let c = local_solar(&spk, eop.as_ref(), &coimbatore, e)
        .unwrap()
        .expect("Coimbatore saw the eclipse");
    let c2 = c.c2.expect("annular phase at Coimbatore");
    let c3 = c.c3.expect("annular phase at Coimbatore");
    show("Coimbatore C2", c2);
    show("Coimbatore C3", c3);
    let d2 = seconds_from(c2, utc(2019, 12, 26, 3, 57, 47));
    let d3 = seconds_from(c3, utc(2019, 12, 26, 4, 0, 56));
    assert!(d2.abs() < 180.0 && d3.abs() < 180.0, "C2 {d2} s, C3 {d3} s");
    // Somewhere on the night side nothing is seen.
    let auckland = Observer {
        latitude_deg: -36.85,
        longitude_deg: 174.76,
        height_m: 0.0,
    };
    let night = local_solar(&spk, eop.as_ref(), &auckland, e).unwrap();
    assert!(night.as_ref().map_or(true, |n| !n.visible), "{night:?}");
}

#[test]
fn counts_2000_to_2030_match_the_published_catalogues() {
    let Some(spk) = kernel() else { return };
    let from = utc(2000, 1, 1, 0, 0, 0);
    let to = utc(2030, 12, 31, 23, 59, 59);
    let solar = solar_eclipses(&spk, from, to).unwrap();
    let lunar = lunar_eclipses(&spk, from, to, ShadowRule::Danjon).unwrap();
    let year_of = |t: Instant| Utc::from_instant(t).year;
    // NASA's decade tables: solar eclipses per year 2000–2030.
    let solar_per_year = [
        4, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // 2000–2010
        4, 2, 2, 2, 2, 2, 2, 3, 3, 2, // 2011–2020
        2, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 2021–2030
    ];
    // Lunar eclipses per year (penumbral included). 2013, 2016 and 2027 each have a
    // penumbral eclipse of magnitude below 0.02 whose presence depends on the shadow
    // convention, so those years are allowed ±1.
    let lunar_per_year = [
        2, 3, 3, 2, 2, 2, 2, 2, 2, 4, 2, // 2000–2010
        2, 2, 3, 2, 2, 2, 2, 2, 2, 4, // 2011–2020
        2, 2, 2, 2, 2, 2, 3, 3, 2, 2, // 2021–2030
    ];
    for (i, want) in solar_per_year.iter().enumerate() {
        let y = 2000 + i as i64;
        let n = solar.iter().filter(|e| year_of(e.greatest) == y).count();
        assert_eq!(n, *want, "solar eclipses in {y}");
    }
    for (i, want) in lunar_per_year.iter().enumerate() {
        let y = 2000 + i as i64;
        let n = lunar.iter().filter(|e| year_of(e.greatest) == y).count() as i64;
        let slack = if [2013, 2016, 2027].contains(&y) {
            1
        } else {
            0
        };
        assert!(
            (n - *want).abs() <= slack,
            "lunar eclipses in {y}: {n}, want {want}"
        );
        assert!((2..=5).contains(&n), "{y}: {n}");
    }
    eprintln!("2000–2030: {} solar, {} lunar", solar.len(), lunar.len());
    // Known kinds: 2005 Apr 8 and 2013 Nov 3 and 2023 Apr 20 are hybrid.
    let kind_on = |y: i64, m: u32, d: u32| {
        solar
            .iter()
            .find(|e| {
                let u = Utc::from_instant(e.greatest);
                (u.year, u.month, u.day) == (y, m, d)
            })
            .map(|e| e.kind)
    };
    assert_eq!(kind_on(2005, 4, 8), Some(SolarKind::Hybrid));
    assert_eq!(kind_on(2013, 11, 3), Some(SolarKind::Hybrid));
    assert_eq!(kind_on(2023, 4, 20), Some(SolarKind::Hybrid));
    assert_eq!(kind_on(2017, 8, 21), Some(SolarKind::Total));
    assert_eq!(kind_on(2023, 10, 14), Some(SolarKind::Annular));
    assert_eq!(kind_on(2025, 3, 29), Some(SolarKind::Partial));
    // Every eclipse is internally consistent.
    for e in &solar {
        assert!(e.gamma.abs() < 1.57, "{e:?}");
        assert!(e.begins < e.greatest && e.greatest < e.ends, "{e:?}");
        assert!(e.magnitude_max > 0.0 && e.magnitude_max < 1.1, "{e:?}");
        if e.kind == SolarKind::Partial {
            assert!(!e.central && e.magnitude_max < 1.0, "{e:?}");
        }
    }
    for e in &lunar {
        let present: Vec<Instant> = e.contacts.iter().flatten().copied().collect();
        for w in present.windows(2) {
            assert!(w[0] < w[1], "{e:?}");
        }
        assert!(e.pen_magnitude > 0.0 && e.pen_magnitude < 3.0, "{e:?}");
        match e.kind {
            LunarKind::Total => {
                assert!(
                    e.umb_magnitude >= 1.0 && e.contacts.iter().all(Option::is_some),
                    "{e:?}"
                )
            }
            LunarKind::Partial => assert!(
                e.umb_magnitude > 0.0
                    && e.umb_magnitude < 1.0
                    && e.contacts[1].is_some()
                    && e.contacts[2].is_none(),
                "{e:?}"
            ),
            LunarKind::Penumbral => {
                assert!(e.umb_magnitude <= 0.0 && e.contacts[1].is_none(), "{e:?}")
            }
        }
        assert!(e.gamma.abs() < 1.6, "{e:?}");
    }
    // The published magnitude of the 2025 Mar 29 partial eclipse is 0.9376 at gamma 1.0405.
    let p = solar
        .iter()
        .find(|e| {
            let u = Utc::from_instant(e.greatest);
            (u.year, u.month, u.day) == (2025, 3, 29)
        })
        .unwrap();
    eprintln!(
        "2025-03-29 gamma {:.5} magnitude {:.5}",
        p.gamma, p.magnitude_max
    );
    assert!((p.gamma - 1.0405).abs() < 0.002, "{}", p.gamma);
    assert!(
        (p.magnitude_max - 0.9376).abs() < 0.01,
        "{}",
        p.magnitude_max
    );
}

#[test]
fn prenatal_syzygy_precedes_birth_and_is_exact() {
    let Some(spk) = kernel() else { return };
    for (y, m, d, h) in [
        (2000, 3, 15, 12),
        (2019, 7, 20, 4),
        (2024, 4, 9, 6),
        (1995, 11, 30, 23),
    ] {
        let birth = utc(y, m, d, h, 0, 0);
        let s = prenatal_syzygy(&spk, birth).unwrap();
        assert!(s.at < birth, "{s:?}");
        assert!(birth.ns_since(s.at) < 30 * NS_PER_SEC * 86_400, "{s:?}");
        let moon = apparent(&spk, Body::Moon, s.at).unwrap();
        let sun = apparent(&spk, Body::Sun, s.at).unwrap();
        let target = match s.kind {
            SyzygyKind::NewMoon => 0.0,
            SyzygyKind::FullMoon => 180.0,
        };
        let elong = wrap180(moon.longitude - sun.longitude - target).abs();
        assert!(elong < 1e-6, "elongation residual {elong}° for {s:?}");
        assert!((s.longitude - moon.longitude).abs() < 1e-9);
        // the lunation number is the one Meeus's mean phase predicts
        let predicted = instant_of_tt_jd(mean_phase_jde(
            s.lunation as f64
                + if s.kind == SyzygyKind::FullMoon {
                    0.5
                } else {
                    0.0
                },
        ));
        assert!(
            s.at.ns_since(predicted).abs() < 20 * 3_600 * NS_PER_SEC,
            "{s:?}"
        );
    }
    // Born the day after the 2024 Apr 8 eclipse: that eclipse is the prenatal one.
    let birth = utc(2024, 4, 9, 6, 0, 0);
    let e = prenatal_eclipse(&spk, birth, ShadowRule::Danjon).unwrap();
    assert!(e.at() < birth);
    match &e {
        PrenatalEclipse::Solar(s) => assert_eq!(s.saros, Some(139)),
        PrenatalEclipse::Lunar(l) => panic!("expected the solar eclipse, got {l:?}"),
    }
    // Born 2024 Apr 8 at noon UTC, before greatest eclipse: the 2024 Mar 25 penumbral
    // lunar eclipse comes first.
    let e = prenatal_eclipse(&spk, utc(2024, 4, 8, 12, 0, 0), ShadowRule::Danjon).unwrap();
    match &e {
        PrenatalEclipse::Lunar(l) => {
            assert_eq!(l.kind, LunarKind::Penumbral);
            assert_eq!(Utc::from_instant(l.greatest).day, 25);
        }
        PrenatalEclipse::Solar(s) => panic!("expected the lunar eclipse, got {s:?}"),
    }
}

#[test]
fn lunar_visibility_follows_the_moon_altitude() {
    let Some(spk) = kernel() else { return };
    let v = lunar_eclipses(
        &spk,
        utc(2022, 11, 1, 0, 0, 0),
        utc(2022, 11, 15, 0, 0, 0),
        ShadowRule::Danjon,
    )
    .unwrap();
    let e = &v[0];
    // 2022 Nov 8 10:59 UTC: the Moon was up over the Pacific and the Americas
    // (Honolulu), and below the horizon in Europe and Africa (Lagos).
    let honolulu = Observer {
        latitude_deg: 21.31,
        longitude_deg: -157.86,
        height_m: 0.0,
    };
    let lagos = Observer {
        latitude_deg: 6.52,
        longitude_deg: 3.38,
        height_m: 0.0,
    };
    assert!(e.visible_at(&spk, None, &honolulu).unwrap());
    assert!(!e.visible_at(&spk, None, &lagos).unwrap());
    let alt = altitude(&spk, None, &honolulu, Body::Moon, e.greatest).unwrap();
    assert!(alt > 30.0, "Moon altitude at Honolulu {alt}");
}

#[test]
fn the_node_distance_screen_drops_no_eclipse() {
    let Some(spk) = kernel() else { return };
    let from = utc(2000, 1, 1, 0, 0, 0);
    let to = utc(2030, 12, 31, 23, 59, 59);
    let screened_solar = solar_eclipses(&spk, from, to).unwrap().len();
    let screened_lunar = lunar_eclipses(&spk, from, to, ShadowRule::Danjon)
        .unwrap()
        .len();
    let mut solar = 0;
    let mut lunar = 0;
    for k in lunations_in(from, to, 0.0) {
        if let Some(e) = solar::at_lunation(&spk, k).unwrap() {
            if e.greatest >= from && e.greatest <= to {
                solar += 1;
            }
        }
        if let Some(e) = lunar::at_lunation(&spk, k, ShadowRule::Danjon).unwrap() {
            if e.greatest >= from && e.greatest <= to {
                lunar += 1;
            }
        }
    }
    assert_eq!((solar, lunar), (screened_solar, screened_lunar));
    assert_eq!((solar, lunar), (70, 71));
}
