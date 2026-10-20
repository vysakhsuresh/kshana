//! Compatibility: the Ashtakoota (eight kutas) of the Moon, Mangal dosha, and Western
//! synastry. Every score is a table lookup over the two charts; the tables are the
//! conventional ones and are stated in the source so nothing is hidden.

use crate::parashari::natural_relation;
use crate::Facts;
use kshana_chart::Chart;
use kshana_vedic::{nakshatra, sign_lord, DashaLord};
use kshana_western::{aspect_between, Aspect};

/// One kuta's score.
#[derive(Clone, Debug, PartialEq)]
pub struct KutaScore {
    /// Name: Varna, Vashya, Tara, Yoni, Graha Maitri, Gana, Bhakoot, Nadi.
    pub name: &'static str,
    /// Points obtained.
    pub points: f64,
    /// Maximum for this kuta.
    pub max: f64,
    /// What each side is (e.g. the two yonis), in words.
    pub detail: String,
}

/// The Ashtakoota result.
#[derive(Clone, Debug, PartialEq)]
pub struct Ashtakoota {
    /// The eight scores in canonical order.
    pub kutas: Vec<KutaScore>,
    /// Total out of 36.
    pub total: f64,
    /// Nadi dosha (0 Nadi points) present.
    pub nadi_dosha: bool,
    /// Bhakoot dosha (0 Bhakoot points) present.
    pub bhakoot_dosha: bool,
    /// Verdict band: Excellent (≥ 28), Good (≥ 24), Acceptable (≥ 18), Asks for care.
    pub verdict: &'static str,
}

const VARNA_NAMES: [&str; 4] = ["Shudra", "Vaishya", "Kshatriya", "Brahmin"];

/// Varna rank 0–3 by Moon sign: water signs Brahmin, fire Kshatriya, earth Vaishya, air Shudra.
fn varna(sign: usize) -> usize {
    match sign % 4 {
        0 => 2, // Aries, Leo, Sagittarius
        1 => 1, // Taurus, Virgo, Capricorn
        2 => 0, // Gemini, Libra, Aquarius
        _ => 3, // Cancer, Scorpio, Pisces
    }
}

const VASHYA_NAMES: [&str; 5] = ["Chatushpada", "Manava", "Jalachara", "Vanachara", "Keeta"];

/// Vashya group of a sidereal Moon longitude.
fn vashya(lon: f64) -> usize {
    let sign = (lon.rem_euclid(360.0) / 30.0).floor() as usize % 12;
    let deg = lon.rem_euclid(30.0);
    match sign {
        0 | 1 => 0,
        2 | 5 | 6 | 10 => 1,
        3 | 11 => 2,
        4 => 3,
        7 => 4,
        8 => {
            if deg < 15.0 {
                1
            } else {
                0
            }
        }
        _ => {
            if deg < 15.0 {
                0
            } else {
                2
            }
        }
    }
}

/// Vashya points, rows and columns in `VASHYA_NAMES` order. The common published table.
const VASHYA_TABLE: [[f64; 5]; 5] = [
    [2.0, 1.0, 1.0, 0.0, 1.0],
    [1.0, 2.0, 0.5, 0.0, 1.0],
    [1.0, 0.5, 2.0, 1.0, 1.0],
    [0.0, 0.0, 1.0, 2.0, 0.0],
    [1.0, 1.0, 1.0, 0.0, 2.0],
];

const YONI_NAMES: [&str; 14] = [
    "Horse", "Elephant", "Sheep", "Serpent", "Dog", "Cat", "Rat", "Cow", "Buffalo", "Tiger",
    "Deer", "Monkey", "Mongoose", "Lion",
];

/// Yoni of each nakshatra, index into `YONI_NAMES`.
const YONI_OF: [usize; 27] = [
    0, 1, 2, 3, 3, 4, 5, 2, 5, 6, 6, 7, 8, 9, 8, 9, 10, 10, 4, 11, 12, 11, 13, 0, 13, 7, 1,
];

/// Yoni points, symmetric, the standard table (4 same, 0 enemy).
const YONI_TABLE: [[u8; 14]; 14] = [
    [4, 2, 2, 3, 2, 2, 2, 1, 0, 1, 3, 3, 2, 1],
    [2, 4, 3, 3, 2, 2, 2, 2, 3, 1, 2, 3, 2, 0],
    [2, 3, 4, 2, 1, 2, 1, 3, 3, 1, 2, 0, 3, 1],
    [3, 3, 2, 4, 2, 1, 1, 1, 1, 2, 2, 2, 0, 2],
    [2, 2, 1, 2, 4, 2, 1, 2, 2, 1, 0, 2, 1, 1],
    [2, 2, 2, 1, 2, 4, 0, 2, 2, 1, 3, 3, 2, 1],
    [2, 2, 1, 1, 1, 0, 4, 2, 2, 2, 2, 2, 1, 2],
    [1, 2, 3, 1, 2, 2, 2, 4, 3, 0, 3, 2, 2, 1],
    [0, 3, 3, 1, 2, 2, 2, 3, 4, 1, 2, 2, 2, 1],
    [1, 1, 1, 2, 1, 1, 2, 0, 1, 4, 1, 1, 2, 1],
    [3, 2, 2, 2, 0, 3, 2, 3, 2, 1, 4, 2, 2, 1],
    [3, 3, 0, 2, 2, 3, 2, 2, 2, 1, 2, 4, 3, 2],
    [2, 2, 3, 0, 1, 2, 1, 2, 2, 2, 2, 3, 4, 2],
    [1, 0, 1, 2, 1, 1, 2, 1, 1, 1, 1, 2, 2, 4],
];

const GANA_NAMES: [&str; 3] = ["Deva", "Manushya", "Rakshasa"];

/// Gana of each nakshatra: 0 Deva, 1 Manushya, 2 Rakshasa.
const GANA_OF: [usize; 27] = [
    0, 1, 2, 1, 0, 1, 0, 0, 2, 2, 1, 1, 0, 2, 0, 2, 0, 2, 2, 1, 1, 0, 2, 2, 1, 1, 0,
];

const NADI_NAMES: [&str; 3] = ["Adi", "Madhya", "Antya"];

/// Nadi of each nakshatra: the cycle 0,1,2,2,1,0 repeating.
fn nadi(n: usize) -> usize {
    [0, 1, 2, 2, 1, 0][n % 6]
}

/// Tara count from `from` nakshatra to `to`, 1–9.
fn tara(from: usize, to: usize) -> usize {
    ((to + 27 - from) % 27) % 9 + 1
}

fn graha_maitri(a: DashaLord, b: DashaLord) -> f64 {
    if a == b {
        return 5.0;
    }
    let ab = natural_relation(a, b);
    let ba = natural_relation(b, a);
    match (ab, ba) {
        (1, 1) => 5.0,
        (1, 0) | (0, 1) => 4.0,
        (0, 0) => 3.0,
        (1, -1) | (-1, 1) => 1.0,
        (0, -1) | (-1, 0) => 0.5,
        _ => 0.0,
    }
}

/// Ashtakoota of two sidereal Moon longitudes. `a` is conventionally the bride, `b` the
/// groom; only Varna and Tara are direction-dependent, and the direction is stated in
/// the detail text.
pub fn ashtakoota(moon_a: f64, moon_b: f64) -> Ashtakoota {
    let sa = (moon_a.rem_euclid(360.0) / 30.0).floor() as usize % 12;
    let sb = (moon_b.rem_euclid(360.0) / 30.0).floor() as usize % 12;
    let na = nakshatra(moon_a).index;
    let nb = nakshatra(moon_b).index;
    let mut kutas = Vec::with_capacity(8);

    let (va, vb) = (varna(sa), varna(sb));
    kutas.push(KutaScore {
        name: "Varna",
        points: if vb >= va { 1.0 } else { 0.0 },
        max: 1.0,
        detail: format!("{} and {}", VARNA_NAMES[va], VARNA_NAMES[vb]),
    });

    let (wa, wb) = (vashya(moon_a), vashya(moon_b));
    kutas.push(KutaScore {
        name: "Vashya",
        points: VASHYA_TABLE[wa][wb],
        max: 2.0,
        detail: format!("{} and {}", VASHYA_NAMES[wa], VASHYA_NAMES[wb]),
    });

    let t1 = tara(na, nb);
    let t2 = tara(nb, na);
    let bad = |t: usize| matches!(t, 3 | 5 | 7);
    let tp = (if bad(t1) { 0.0 } else { 1.5 }) + (if bad(t2) { 0.0 } else { 1.5 });
    kutas.push(KutaScore {
        name: "Tara",
        points: tp,
        max: 3.0,
        detail: format!("counts {t1} and {t2} of 9"),
    });

    let (ya, yb) = (YONI_OF[na], YONI_OF[nb]);
    kutas.push(KutaScore {
        name: "Yoni",
        points: YONI_TABLE[ya][yb] as f64,
        max: 4.0,
        detail: format!("{} and {}", YONI_NAMES[ya], YONI_NAMES[yb]),
    });

    let (la, lb) = (sign_lord(sa), sign_lord(sb));
    kutas.push(KutaScore {
        name: "Graha Maitri",
        points: graha_maitri(la, lb),
        max: 5.0,
        detail: format!("Moon lords {la:?} and {lb:?}"),
    });

    let (ga, gb) = (GANA_OF[na], GANA_OF[nb]);
    let gp = match (ga, gb) {
        (x, y) if x == y => 6.0,
        (0, 1) | (1, 0) => 5.0,
        (0, 2) | (2, 0) => 1.0,
        _ => 0.0,
    };
    kutas.push(KutaScore {
        name: "Gana",
        points: gp,
        max: 6.0,
        detail: format!("{} and {}", GANA_NAMES[ga], GANA_NAMES[gb]),
    });

    let d = (sb + 12 - sa) % 12 + 1;
    let bp = if matches!(d, 2 | 12 | 5 | 9 | 6 | 8) {
        0.0
    } else {
        7.0
    };
    kutas.push(KutaScore {
        name: "Bhakoot",
        points: bp,
        max: 7.0,
        detail: format!("Moon signs {d} and {} apart", (sa + 12 - sb) % 12 + 1),
    });

    let (da, db) = (nadi(na), nadi(nb));
    kutas.push(KutaScore {
        name: "Nadi",
        points: if da == db { 0.0 } else { 8.0 },
        max: 8.0,
        detail: format!("{} and {}", NADI_NAMES[da], NADI_NAMES[db]),
    });

    let total: f64 = kutas.iter().map(|k| k.points).sum();
    let verdict = if total >= 28.0 {
        "Excellent"
    } else if total >= 24.0 {
        "Good"
    } else if total >= 18.0 {
        "Acceptable"
    } else {
        "Asks for care"
    };
    Ashtakoota {
        nadi_dosha: da == db,
        bhakoot_dosha: bp == 0.0,
        kutas,
        total,
        verdict,
    }
}

/// Mangal (Kuja) dosha from one chart.
#[derive(Clone, Debug, PartialEq)]
pub struct MangalDosha {
    /// Mars' rasi house from the lagna.
    pub from_lagna: u8,
    /// Mars' rasi house from the Moon.
    pub from_moon: u8,
    /// Mars' rasi house from Venus.
    pub from_venus: u8,
    /// Present from any of the three references (houses 1, 2, 4, 7, 8, 12).
    pub present: bool,
    /// Present from the lagna alone.
    pub from_lagna_present: bool,
    /// Cancellations that apply, in words.
    pub cancellations: Vec<String>,
    /// Net: present and not cancelled.
    pub effective: bool,
}

/// Mangal dosha: Mars in 1, 2, 4, 7, 8 or 12 from the lagna, Moon or Venus (South Indian
/// convention including the 2nd). Cancellations stated are the common ones: Mars in own
/// or exalted sign, Mars in Cancer or Leo (as some texts hold), Jupiter aspecting Mars.
pub fn mangal_dosha(facts: &Facts) -> MangalDosha {
    let mars = &facts.planets[2];
    let moon = &facts.planets[1];
    let venus = &facts.planets[5];
    let from = |ref_sign: u8| ((mars.sign as i32 - ref_sign as i32).rem_euclid(12) + 1) as u8;
    let from_lagna = mars.rasi_house;
    let from_moon = from(moon.sign);
    let from_venus = from(venus.sign);
    let bad = |h: u8| matches!(h, 1 | 2 | 4 | 7 | 8 | 12);
    let present = bad(from_lagna) || bad(from_moon) || bad(from_venus);
    let mut cancellations = Vec::new();
    if matches!(mars.sign, 0 | 7) {
        cancellations.push("Mars in its own sign".to_string());
    }
    if mars.sign == 9 {
        cancellations.push("Mars exalted in Capricorn".to_string());
    }
    if matches!(mars.sign, 3 | 4) {
        cancellations.push("Mars in Cancer or Leo (held harmless by some texts)".to_string());
    }
    let jup = &facts.planets[4];
    let jup_aspects = crate::parashari::aspected_houses(DashaLord::Jupiter, jup.rasi_house);
    if jup_aspects.contains(&mars.rasi_house) || jup.rasi_house == mars.rasi_house {
        cancellations.push("Jupiter aspects or joins Mars".to_string());
    }
    MangalDosha {
        from_lagna,
        from_moon,
        from_venus,
        present,
        from_lagna_present: bad(from_lagna),
        effective: present && cancellations.is_empty(),
        cancellations,
    }
}

/// One synastry aspect.
#[derive(Clone, Debug, PartialEq)]
pub struct SynastryAspect {
    /// Body in chart A.
    pub a: &'static str,
    /// Body in chart B.
    pub b: &'static str,
    /// Aspect name.
    pub aspect: &'static str,
    /// Orb in degrees (absolute).
    pub orb: f64,
    /// Harmonious (+1), tense (−1) or neutral (0).
    pub tone: i8,
    /// Weight in `[0, 1]`: closer orbs and personal planets weigh more.
    pub weight: f64,
}

/// Western synastry: the cross-aspects between the two charts' tropical bodies within
/// `max_orb`, with a weight that favours tight orbs and the Sun, Moon, Venus and Mars.
pub fn synastry(a: &Chart, b: &Chart, max_orb: f64) -> Vec<SynastryAspect> {
    let personal = |n: &str| matches!(n, "Sun" | "Moon" | "Venus" | "Mars" | "Mercury");
    let mut out = Vec::new();
    for pa in &a.bodies {
        for pb in &b.bodies {
            let Some(h) = aspect_between(pa.tropical, pb.tropical, None, None, max_orb) else {
                continue;
            };
            let tone = match h.aspect {
                Aspect::Conjunction => 0,
                Aspect::Trine | Aspect::Sextile => 1,
                Aspect::Square | Aspect::Opposition => -1,
                _ => 0,
            };
            let mut w = 1.0 - h.orb.abs() / max_orb;
            if personal(pa.name) {
                w *= 1.0;
            } else {
                w *= 0.6;
            }
            if !personal(pb.name) {
                w *= 0.6;
            }
            out.push(SynastryAspect {
                a: pa.name,
                b: pb.name,
                aspect: h.aspect.name(),
                orb: h.orb.abs(),
                tone,
                weight: w.clamp(0.0, 1.0),
            });
        }
    }
    out.sort_by(|x, y| {
        y.weight
            .partial_cmp(&x.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_moon_is_not_a_match_for_nadi() {
        let r = ashtakoota(100.0, 100.0);
        assert!(r.nadi_dosha);
        assert_eq!(
            r.kutas.iter().find(|k| k.name == "Nadi").unwrap().points,
            0.0
        );
        assert_eq!(r.kutas.len(), 8);
        let max: f64 = r.kutas.iter().map(|k| k.max).sum();
        assert_eq!(max, 36.0);
    }

    #[test]
    fn classic_pair_scores_in_band() {
        // Rohini Moon (Taurus) with Hasta Moon (Virgo): trine signs, 5/9 Bhakoot is a dosha.
        let r = ashtakoota(45.0, 165.0);
        assert!(r.bhakoot_dosha);
        assert!(r.total <= 36.0 && r.total >= 0.0);
        for k in &r.kutas {
            assert!(k.points <= k.max);
        }
    }

    #[test]
    fn tables_are_symmetric() {
        for (i, row) in YONI_TABLE.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                assert_eq!(*v, YONI_TABLE[j][i]);
            }
            assert_eq!(row[i], 4);
        }
        for (i, row) in VASHYA_TABLE.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                assert_eq!(*v, VASHYA_TABLE[j][i]);
            }
        }
    }

    #[test]
    fn tara_counts_from_one() {
        assert_eq!(tara(0, 0), 1);
        assert_eq!(tara(0, 2), 3);
        assert_eq!(tara(26, 0), 2);
    }
}
