//! Golden tests against the classical rules: Vimshottari dasha, Ashtakavarga and the
//! Ashtakoota (kuta) match.
//!
//! Every expected value is either (a) recomputed inside the test by an independent,
//! deliberately naive implementation transcribed from the text named in the comment, or
//! (b) a fixed number the text itself gives (the Ashtakavarga row totals, the kuta
//! maxima). Nothing is copied from the engine's own tables.

use kshana_chart::{compute, Chart, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{bhinna, sarva, subdivide, vimshottari, AshtakavargaPositions, DashaLord};
use std::path::PathBuf;

fn kernel() -> Option<Spk> {
    Spk::open(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp")).ok()
}

fn at(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> Instant {
    Utc::new(y, mo, d, h, mi, 0, 0).unwrap().to_instant().unwrap()
}

fn chart(spk: &Spk, t: Instant, lat: f64, lon: f64) -> Chart {
    let input = ChartInput {
        instant: t,
        observer: Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 },
        system: HouseSystem::Placidus,
        ayanamsa: kshana_vedic::Ayanamsa::Lahiri,
        topocentric_moon: false,
    };
    compute(spk, None, &input).unwrap()
}

/// A deterministic spread of births: 1901–2049, every month, all hours, eight places
/// from 41° S to 60° N.
fn births(n: usize) -> Vec<(Instant, f64, f64)> {
    const PLACES: [(f64, f64); 8] = [
        (9.9312, 76.2673),   // Kochi
        (28.6139, 77.2090),  // New Delhi
        (13.0827, 80.2707),  // Chennai
        (19.0760, 72.8777),  // Mumbai
        (51.5074, -0.1278),  // London
        (40.7128, -74.0060), // New York
        (-33.8688, 151.2093), // Sydney
        (59.9139, 10.7522),  // Oslo
    ];
    (0..n)
        .map(|i| {
            let y = 1901 + (i as i64 * 37) % 149;
            let mo = 1 + (i as u32 * 5) % 12;
            let d = 1 + (i as u32 * 11) % 28;
            let h = (i as u32 * 7) % 24;
            let mi = (i as u32 * 13) % 60;
            let (lat, lon) = PLACES[i % PLACES.len()];
            (at(y, mo, d, h, mi), lat, lon)
        })
        .collect()
}

// ---------------------------------------------------------------------------------
// Vimshottari
// ---------------------------------------------------------------------------------

/// BPHS, Vimshottari-dasha chapter (Santhanam tr. ch. 46): the cycle from Ketu, with
/// Ketu 7, Venus 20, Sun 6, Moon 10, Mars 7, Rahu 18, Jupiter 16, Saturn 19,
/// Mercury 17 years; the lord of the Moon's nakshatra runs first, for the part of its
/// years proportional to the unexpired part of the nakshatra (the "rule of three" of
/// every panchanga's ready reckoner); antardashas run in the same order from the
/// mahadasha lord, each lasting maha × own / 120.
const BPHS_ORDER: [(&str, f64); 9] = [
    ("Ketu", 7.0),
    ("Venus", 20.0),
    ("Sun", 6.0),
    ("Moon", 10.0),
    ("Mars", 7.0),
    ("Rahu", 18.0),
    ("Jupiter", 16.0),
    ("Saturn", 19.0),
    ("Mercury", 17.0),
];

/// Independent schedule in seconds from birth: (lord index, start s, end s) for the nine
/// mahadashas and, for each, its nine antardashas.
#[allow(clippy::type_complexity)]
fn naive_vimshottari(moon_sid: f64) -> Vec<((usize, f64, f64), Vec<(usize, f64, f64)>)> {
    let year_s = 365.25 * 86_400.0; // the engine's documented year (dasha.rs)
    let nak_span = 360.0 / 27.0;
    let n = (moon_sid / nak_span).floor();
    let traversed = moon_sid / nak_span - n;
    let first = n as usize % 9;
    let balance_years = (1.0 - traversed) * BPHS_ORDER[first].1;
    // notional start of the running mahadasha, years relative to birth
    let mut start = balance_years - BPHS_ORDER[first].1;
    let mut out = Vec::new();
    for i in 0..9 {
        let lord = (first + i) % 9;
        let len = BPHS_ORDER[lord].1;
        let mut subs = Vec::new();
        let mut s = start;
        for j in 0..9 {
            let sl = (lord + j) % 9;
            let sublen = len * BPHS_ORDER[sl].1 / 120.0;
            subs.push((sl, s * year_s, (s + sublen) * year_s));
            s += sublen;
        }
        out.push(((lord, start * year_s, (start + len) * year_s), subs));
        start += len;
    }
    out
}

fn lord_index(l: DashaLord) -> usize {
    DashaLord::ORDER.iter().position(|x| *x == l).unwrap()
}

fn check_every_boundary(moon_sid: f64, birth: Instant, label: &str) -> f64 {
    let engine = vimshottari(moon_sid, birth, YEAR_365_25_NS);
    let naive = naive_vimshottari(moon_sid);
    let secs = |t: Instant| t.ns_since(birth) as f64 / 1e9;
    let mut worst = 0.0f64;
    for (md, ((lord, s, e), subs)) in engine.iter().zip(naive) {
        assert_eq!(BPHS_ORDER[lord_index(md.lord)].0, BPHS_ORDER[lord].0, "{label}");
        for (got, want) in [(secs(md.start), s), (secs(md.end), e)] {
            worst = worst.max((got - want).abs());
            assert!((got - want).abs() < 1e-3, "{label} {:?}: {got} vs {want} s", md.lord);
        }
        for (ad, (sl, s, e)) in subdivide(*md).iter().zip(subs) {
            assert_eq!(lord_index(ad.lord), sl, "{label} {:?}/{:?}", md.lord, ad.lord);
            for (got, want) in [(secs(ad.start), s), (secs(ad.end), e)] {
                worst = worst.max((got - want).abs());
                assert!(
                    (got - want).abs() < 1e-3,
                    "{label} {:?}/{:?}: {got} vs {want} s",
                    md.lord,
                    ad.lord
                );
            }
        }
    }
    worst
}

/// Three charts whose Moon is computed by the engine (the Moon's longitude itself is
/// pinned against Swiss Ephemeris and Skyfield elsewhere in this crate), then every
/// mahadasha and antardasha boundary (9 + 81 per chart, 180 instants) checked against the
/// naive BPHS schedule above to a millisecond.
#[test]
fn vimshottari_every_maha_and_antar_boundary_matches_bphs_rule_of_three() {
    let Some(spk) = kernel() else { return };
    // (1) Kochi 1990-02-20 05:04 UTC, the hand-checked chart of kshana-chart's
    //     `kochi_1990_02_20_birth_chart_and_dasha_balance` (Meeus-based hand reduction:
    //     Moon 4°30′20″ Sagittarius, Mula, 33.79 % traversed, Ketu balance 1 692.758 d).
    // (2) Indian Independence, 1947-08-15 00:00 IST (1947-08-14 18:30 UTC), New Delhi:
    //     the widely published mundane chart with a Taurus lagna and the Moon in Pushya
    //     (Cancer), so Saturn's mahadasha runs at birth.
    // (3) J2000.0 at Greenwich, a chart with no astrological pedigree: a plain number.
    let charts = [
        ("Kochi 1990", at(1990, 2, 20, 5, 4), 9.9312, 76.2673),
        ("India 1947", at(1947, 8, 14, 18, 30), 28.6139, 77.2090),
        ("J2000 Greenwich", at(2000, 1, 1, 11, 58), 51.4779, 0.0),
        ("Chennai 1975", at(1975, 6, 25, 23, 40), 13.0827, 80.2707),
    ];
    for (label, t, lat, lon) in charts {
        let ch = chart(&spk, t, lat, lon);
        let moon = ch.bodies[1].sidereal;
        let worst = check_every_boundary(moon, t, label);
        eprintln!("{label}: Moon {moon:.6}°, worst boundary difference {worst:.2e} s");
    }

    // Published facts of the two named charts.
    let k = chart(&spk, at(1990, 2, 20, 5, 4), 9.9312, 76.2673);
    let v = vimshottari(k.bodies[1].sidereal, k.instant, YEAR_365_25_NS);
    assert_eq!(v[0].lord, DashaLord::Ketu);
    let bal = v[0].end.ns_since(k.instant) as f64 / NS_PER_DAY as f64;
    assert!((bal - 1_692.758).abs() < 0.01, "{bal}");

    let india = chart(&spk, at(1947, 8, 14, 18, 30), 28.6139, 77.2090);
    assert_eq!(india.bodies[1].nakshatra.0, 7, "Moon in Pushya");
    assert_eq!((india.cusps_sidereal[0] / 30.0).floor() as usize, 1, "Taurus lagna");
    let v = vimshottari(india.bodies[1].sidereal, india.instant, YEAR_365_25_NS);
    assert_eq!(v[0].lord, DashaLord::Saturn);
}

/// The two edge cases every ready reckoner prints: the Moon at the very start of a
/// nakshatra leaves the whole period, and halfway through leaves half.
#[test]
fn vimshottari_balance_edges_follow_the_rule_of_three() {
    let birth = Instant::j2000();
    let span = 360.0 / 27.0;
    for n in 0..27 {
        let (name, years) = BPHS_ORDER[n % 9];
        for (frac, want_years) in [(0.0, years), (0.5, years / 2.0), (0.75, years / 4.0)] {
            let v = vimshottari(span * (n as f64 + frac), birth, YEAR_365_25_NS);
            assert_eq!(format!("{:?}", v[0].lord), name);
            let got = v[0].end.ns_since(birth) as f64 / YEAR_365_25_NS as f64;
            assert!((got - want_years).abs() < 1e-9, "nak {n} frac {frac}: {got} vs {want_years}");
        }
    }
}

// ---------------------------------------------------------------------------------
// Ashtakavarga
// ---------------------------------------------------------------------------------

/// BPHS, Ashtakavargadhyaya (Santhanam tr. ch. 66), the benefic places, transcribed
/// afresh for this test as "graha: from Sun | Moon | Mars | Mercury | Jupiter | Venus |
/// Saturn | Lagna".
const BPHS_BINDUS: [&str; 7] = [
    "Sun: 1 2 4 7 8 9 10 11 | 3 6 10 11 | 1 2 4 7 8 9 10 11 | 3 5 6 9 10 11 12 | 5 6 9 11 | 6 7 12 | 1 2 4 7 8 9 10 11 | 3 4 6 10 11 12",
    "Moon: 3 6 7 8 10 11 | 1 3 6 7 10 11 | 2 3 5 6 9 10 11 | 1 3 4 5 7 8 10 11 | 1 4 7 8 10 11 12 | 3 4 5 7 9 10 11 | 3 5 6 11 | 3 6 10 11",
    "Mars: 3 5 6 10 11 | 3 6 11 | 1 2 4 7 8 10 11 | 3 5 6 11 | 6 10 11 12 | 6 8 11 12 | 1 4 7 8 9 10 11 | 1 3 6 10 11",
    "Mercury: 5 6 9 11 12 | 2 4 6 8 10 11 | 1 2 4 7 8 9 10 11 | 1 3 5 6 9 10 11 12 | 6 8 11 12 | 1 2 3 4 5 8 9 11 | 1 2 4 7 8 9 10 11 | 1 2 4 6 8 10 11",
    "Jupiter: 1 2 3 4 7 8 9 10 11 | 2 5 7 9 11 | 1 2 4 7 8 10 11 | 1 2 4 5 6 9 10 11 | 1 2 3 4 7 8 10 11 | 2 5 6 9 10 11 | 3 5 6 12 | 1 2 4 5 6 7 9 10 11",
    "Venus: 8 11 12 | 1 2 3 4 5 8 9 11 12 | 3 5 6 9 11 12 | 3 5 6 9 11 | 5 8 9 10 11 | 1 2 3 4 5 8 9 10 11 | 3 4 5 8 9 10 11 | 1 2 3 4 5 8 9 11",
    "Saturn: 1 2 4 7 8 10 11 | 3 6 11 | 3 5 6 10 11 12 | 6 8 9 10 11 12 | 5 6 11 12 | 6 11 12 | 3 5 6 11 | 1 3 4 6 10 11",
];

/// The fixed totals the texts give for each graha's table (BPHS ibid.; B. V. Raman,
/// *Ashtakavarga System of Prediction*, ch. 2): Sun 48, Moon 49, Mars 39, Mercury 54,
/// Jupiter 56, Venus 52, Saturn 39; Sarvashtakavarga 337.
const CLASSICAL_TOTALS: [u32; 7] = [48, 49, 39, 54, 56, 52, 39];

fn naive_bhinna(signs: &[u8; 8], g: usize) -> [u8; 12] {
    let row = BPHS_BINDUS[g].split(':').nth(1).unwrap();
    let mut out = [0u8; 12];
    for (r, houses) in row.split('|').enumerate() {
        for h in houses.split_whitespace() {
            let h: usize = h.parse().unwrap();
            out[(signs[r] as usize + h - 1) % 12] += 1;
        }
    }
    out
}

/// Sign indices in Ashtakavarga order (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn,
/// Lagna) from a chart, whose bodies run Sun, Moon, Mercury, Venus, Mars, Jupiter, Saturn.
fn av_signs(ch: &Chart) -> [u8; 8] {
    let s = |lon: f64| (lon.rem_euclid(360.0) / 30.0).floor() as u8 % 12;
    let b = &ch.bodies;
    [
        s(b[0].sidereal),
        s(b[1].sidereal),
        s(b[4].sidereal),
        s(b[2].sidereal),
        s(b[5].sidereal),
        s(b[3].sidereal),
        s(b[6].sidereal),
        s(ch.cusps_sidereal[0]),
    ]
}

#[test]
fn ashtakavarga_matches_bphs_and_classical_totals_for_any_chart() {
    let transcribed: Vec<u32> = (0..7).map(|g| naive_bhinna(&[0; 8], g).iter().map(|&b| b as u32).sum()).collect();
    assert_eq!(transcribed, CLASSICAL_TOTALS, "the test's own transcription");
    assert_eq!(CLASSICAL_TOTALS.iter().sum::<u32>(), 337);

    let Some(spk) = kernel() else { return };
    let mut checked = 0;
    for (t, lat, lon) in births(150) {
        let ch = chart(&spk, t, lat, lon);
        let signs = av_signs(&ch);
        let p = AshtakavargaPositions { signs };
        let mut sav = [0u32; 12];
        for g in 0..7 {
            let b = bhinna(&p, g);
            assert_eq!(b, naive_bhinna(&signs, g), "graha {g} at {signs:?}");
            assert_eq!(b.iter().map(|&x| x as u32).sum::<u32>(), CLASSICAL_TOTALS[g]);
            assert!(b.iter().all(|&x| x <= 8), "at most 8 contributors per sign");
            for i in 0..12 {
                sav[i] += b[i] as u32;
            }
        }
        let s = sarva(&p);
        assert_eq!(s.map(|x| x as u32), sav);
        assert_eq!(s.iter().map(|&x| x as u32).sum::<u32>(), 337);
        checked += 1;
    }
    assert_eq!(checked, 150);
}

// ---------------------------------------------------------------------------------
// Ashtakoota
// ---------------------------------------------------------------------------------

/// Muhurta Chintamani, Vivaha-prakarana (and B. V. Raman, *Muhurtha*, ch. on marriage),
/// transcribed afresh as nakshatra lists, Ashwini = 0.
mod mc {
    pub const DEVA: [usize; 9] = [0, 4, 6, 7, 12, 14, 16, 21, 26];
    pub const MANUSHYA: [usize; 9] = [1, 3, 5, 10, 11, 19, 20, 24, 25];
    // Rakshasa: the other nine (Krittika, Ashlesha, Magha, Chitra, Vishakha, Jyeshtha,
    // Mula, Dhanishta, Shatabhisha).
    pub const ADI: [usize; 9] = [0, 5, 6, 11, 12, 17, 18, 23, 24];
    pub const MADHYA: [usize; 9] = [1, 4, 7, 10, 13, 16, 19, 22, 25];
    // Antya: Krittika, Rohini, Ashlesha, Magha, Swati, Vishakha, U.Ashadha, Shravana, Revati.
    /// Yoni animal of each nakshatra, by name.
    pub const YONI: [&str; 27] = [
        "horse", "elephant", "sheep", "serpent", "serpent", "dog", "cat", "sheep", "cat",
        "rat", "rat", "cow", "buffalo", "tiger", "buffalo", "tiger", "deer", "deer", "dog",
        "monkey", "mongoose", "monkey", "lion", "horse", "lion", "cow", "elephant",
    ];
    /// The seven sworn-enemy pairs (0 points).
    pub const ENEMIES: [(&str, &str); 7] = [
        ("horse", "buffalo"),
        ("elephant", "lion"),
        ("sheep", "monkey"),
        ("serpent", "mongoose"),
        ("dog", "deer"),
        ("cat", "rat"),
        ("cow", "tiger"),
    ];
    /// Naisargika maitri, BPHS ch. 3: (graha, friends, enemies); the rest are neutral.
    pub const MAITRI: [(&str, &[&str], &[&str]); 7] = [
        ("Sun", &["Moon", "Mars", "Jupiter"], &["Venus", "Saturn"]),
        ("Moon", &["Sun", "Mercury"], &[]),
        ("Mars", &["Sun", "Moon", "Jupiter"], &["Mercury"]),
        ("Mercury", &["Sun", "Venus"], &["Moon"]),
        ("Jupiter", &["Sun", "Moon", "Mars"], &["Mercury", "Venus"]),
        ("Venus", &["Mercury", "Saturn"], &["Sun", "Moon"]),
        ("Saturn", &["Mercury", "Venus"], &["Sun", "Moon", "Mars"]),
    ];
    /// Sign lords, Aries … Pisces.
    pub const LORD: [&str; 12] = [
        "Mars", "Venus", "Mercury", "Moon", "Sun", "Mercury", "Venus", "Mars", "Jupiter",
        "Saturn", "Saturn", "Jupiter",
    ];
}

fn rel(a: &str, b: &str) -> i8 {
    let (_, f, e) = mc::MAITRI.iter().find(|(g, _, _)| *g == a).unwrap();
    if f.contains(&b) {
        1
    } else if e.contains(&b) {
        -1
    } else {
        0
    }
}

/// The independent score of every kuta except Vashya, whose published tables differ
/// between texts (see docs/conventions.md); Vashya is checked by invariants only.
fn naive_kuta(bride: f64, groom: f64) -> [f64; 8] {
    let sign = |l: f64| (l / 30.0).floor() as usize % 12;
    let nak = |l: f64| (l / (360.0 / 27.0)).floor() as usize % 27;
    let (sa, sb, na, nb) = (sign(bride), sign(groom), nak(bride), nak(groom));
    // Varna: Cancer/Scorpio/Pisces Brahmin 3, fire Kshatriya 2, earth Vaishya 1, air
    // Shudra 0; one point when the groom's varna is not lower than the bride's.
    let varna = |s: usize| [2, 1, 0, 3][s % 4];
    let v = if varna(sb) >= varna(sa) { 1.0 } else { 0.0 };
    // Tara: count from one star to the other inclusive, remainder by 9; 3, 5, 7 (Vipat,
    // Pratyak, Naidhana) fail. 1½ points for each direction.
    let tara = |a: usize, b: usize| {
        let r = ((b + 27 - a) % 27 + 1) % 9;
        if matches!(r, 3 | 5 | 7) {
            0.0
        } else {
            1.5
        }
    };
    let t = tara(na, nb) + tara(nb, na);
    // Yoni: same 4, sworn enemies 0, otherwise 1–3 (checked separately).
    let (ya, yb) = (mc::YONI[na], mc::YONI[nb]);
    let y = if ya == yb {
        4.0
    } else if mc::ENEMIES.iter().any(|&(p, q)| (p, q) == (ya, yb) || (q, p) == (ya, yb)) {
        0.0
    } else {
        -1.0 // marker: "between 1 and 3"
    };
    // Graha maitri of the Moon-sign lords: friend/friend 5, friend/neutral 4,
    // neutral/neutral 3, friend/enemy 1, neutral/enemy ½, enemy/enemy 0.
    let (la, lb) = (mc::LORD[sa], mc::LORD[sb]);
    let m = if la == lb {
        5.0
    } else {
        match (rel(la, lb), rel(lb, la)) {
            (1, 1) => 5.0,
            (1, 0) | (0, 1) => 4.0,
            (0, 0) => 3.0,
            (1, -1) | (-1, 1) => 1.0,
            (0, -1) | (-1, 0) => 0.5,
            _ => 0.0,
        }
    };
    // Gana: same 6, Deva–Manushya 5, Deva–Rakshasa 1, Manushya–Rakshasa 0 (the
    // symmetric table; see docs/conventions.md for the directional variant).
    let gana = |n: usize| {
        if mc::DEVA.contains(&n) {
            0
        } else if mc::MANUSHYA.contains(&n) {
            1
        } else {
            2
        }
    };
    let g = match (gana(na), gana(nb)) {
        (x, y) if x == y => 6.0,
        (0, 1) | (1, 0) => 5.0,
        (0, 2) | (2, 0) => 1.0,
        _ => 0.0,
    };
    // Bhakoot: the Moon signs 2/12, 5/9 or 6/8 from each other score 0, else 7.
    let d = (sb + 12 - sa) % 12 + 1;
    let b = if matches!(d, 2 | 12 | 5 | 9 | 6 | 8) { 0.0 } else { 7.0 };
    // Nadi: the same nadi scores 0, different 8.
    let nadi = |n: usize| {
        if mc::ADI.contains(&n) {
            0
        } else if mc::MADHYA.contains(&n) {
            1
        } else {
            2
        }
    };
    let n = if nadi(na) == nadi(nb) { 0.0 } else { 8.0 };
    [v, -1.0, t, y, m, g, b, n]
}

/// Every pair of Moons at 216 points of the zodiac (two in each pada, away from the
/// 15° Vashya splits), 46 656 pairs, against the independent tables.
#[test]
fn ashtakoota_matches_muhurta_chintamani_for_every_pada_pair() {
    const MAX: [(&str, f64); 8] = [
        ("Varna", 1.0),
        ("Vashya", 2.0),
        ("Tara", 3.0),
        ("Yoni", 4.0),
        ("Graha Maitri", 5.0),
        ("Gana", 6.0),
        ("Bhakoot", 7.0),
        ("Nadi", 8.0),
    ];
    let pada = 360.0 / 108.0;
    let moons: Vec<f64> = (0..108).flat_map(|p| [(p as f64 + 0.2) * pada, (p as f64 + 0.7) * pada]).collect();
    let mut totals_seen = std::collections::BTreeSet::new();
    for &a in &moons {
        for &b in &moons {
            let r = kshana_rules::ashtakoota(a, b);
            let want = naive_kuta(a, b);
            assert_eq!(r.kutas.len(), 8);
            let mut sum = 0.0;
            for (i, k) in r.kutas.iter().enumerate() {
                assert_eq!((k.name, k.max), MAX[i]);
                assert!(k.points >= 0.0 && k.points <= k.max, "{a} {b} {}", k.name);
                assert_eq!((k.points * 2.0).fract(), 0.0, "half points at most");
                match want[i] {
                    w if w >= 0.0 => assert_eq!(k.points, w, "{} for {a:.3}° / {b:.3}°", k.name),
                    _ if i == 3 => assert!((1.0..=3.0).contains(&k.points), "Yoni {a} {b}"),
                    _ => {}
                }
                sum += k.points;
            }
            assert_eq!(r.total, sum);
            assert!(r.total <= 36.0);
            assert_eq!(r.nadi_dosha, want[7] == 0.0);
            assert_eq!(r.bhakoot_dosha, want[6] == 0.0);
            // Vashya is symmetric and full for the same group
            let rv = kshana_rules::ashtakoota(b, a).kutas[1].points;
            assert_eq!(r.kutas[1].points, rv, "Vashya symmetric {a} {b}");
            totals_seen.insert((r.total * 2.0) as i32);
        }
    }
    // The same Moon: everything but Nadi is full (Varna 1, Vashya 2, Tara 3, Yoni 4,
    // Maitri 5, Gana 6, Bhakoot 7) = 28 — the textbook "same nakshatra" case.
    for &a in &moons {
        assert_eq!(kshana_rules::ashtakoota(a, a).total, 28.0, "{a}");
    }
    eprintln!("distinct totals: {}", totals_seen.len());
}
