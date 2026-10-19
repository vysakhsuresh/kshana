//! Vimshopaka bala: the twenty-point strength of a graha across the divisional charts.
//!
//! # Sources
//!
//! BPHS ch. 8 (the chapter on divisional considerations in G. C. Sharma's and R.
//! Santhanam's editions; vedicmystics.com "Vimsopaka Bala" and "Vimsopaka Bala
//! Calculation"; boloji.com "Vimshopaka: Vargabala was not enough"; the schemes
//! Jagannatha Hora prints): four schemes whose varga weights add to twenty,
//!
//! * Shadvarga: D1 6, D2 2, D3 4, D9 5, D12 2, D30 1;
//! * Saptavarga: D1 5, D2 2, D3 3, D7 1, D9 2.5, D12 4.5, D30 2 (the two sources that
//!   print the text's numbers agree on this order; a reading with D7 2.5, D9 4.5, D12 2,
//!   D30 1 circulates and is not used);
//! * Dashavarga: D1 3, D2 1.5, D3 1.5, D7 1.5, D9 1.5, D10 1.5, D12 1.5, D16 1.5, D30
//!   1.5, D60 5;
//! * Shodashavarga: D1 3.5, D2 1, D3 1, D4 0.5, D7 0.5, D9 3, D10 0.5, D12 0.5, D16 2,
//!   D20 0.5, D24 0.5, D27 0.5, D30 1, D40 0.5, D45 0.5, D60 4.
//!
//! In each varga the graha scores, out of twenty, 20 in its own sign and otherwise by
//! its compound (panchadha) relationship with the varga sign's lord: great friend 18,
//! friend 15, neutral 10, enemy 7, great enemy 5 (BPHS 8; the relationship from
//! `kshana_bala::compound_relation`, the temporary part read from the rasi chart). The
//! weighted sum of `points / 20` is the scheme's score. Convention: the text awards
//! nothing special for exaltation or debilitation in a varga, so none is given; some
//! software scores exaltation 20 and debilitation 0.
//!
//! D40 (Khavedamsa) counts from Aries for odd signs and Libra for even ones, D45
//! (Akshavedamsa) from Aries, Leo and Sagittarius for movable, fixed and dual signs
//! (BPHS ch. 6; indianastrologyarticles.blogspot.com "Divisional charts D40/D45";
//! jagannathhora.com "Akshavedamsa chart"); the other vargas come from
//! `kshana_vedic::varga`.

use kshana_bala::tables::SEVEN;
use kshana_bala::{compound_relation, Relation};
use kshana_chart::Chart;
use kshana_vedic::{sign_lord, varga, DashaLord};

use crate::{graha_name, sign_of, MargaError, SIGN_NAMES};

/// Shadvarga weights `(n, weight)`.
pub const SHADVARGA: [(u32, f64); 6] =
    [(1, 6.0), (2, 2.0), (3, 4.0), (9, 5.0), (12, 2.0), (30, 1.0)];
/// Saptavarga weights.
pub const SAPTAVARGA: [(u32, f64); 7] = [
    (1, 5.0),
    (2, 2.0),
    (3, 3.0),
    (7, 1.0),
    (9, 2.5),
    (12, 4.5),
    (30, 2.0),
];
/// Dashavarga weights.
pub const DASHAVARGA: [(u32, f64); 10] = [
    (1, 3.0),
    (2, 1.5),
    (3, 1.5),
    (7, 1.5),
    (9, 1.5),
    (10, 1.5),
    (12, 1.5),
    (16, 1.5),
    (30, 1.5),
    (60, 5.0),
];
/// Shodashavarga weights.
pub const SHODASHAVARGA: [(u32, f64); 16] = [
    (1, 3.5),
    (2, 1.0),
    (3, 1.0),
    (4, 0.5),
    (7, 0.5),
    (9, 3.0),
    (10, 0.5),
    (12, 0.5),
    (16, 2.0),
    (20, 0.5),
    (24, 0.5),
    (27, 0.5),
    (30, 1.0),
    (40, 0.5),
    (45, 0.5),
    (60, 4.0),
];

/// Sign index 0–11 of a longitude in the D-n chart, with D40 and D45 by their own
/// rules and every other n by `kshana_vedic::varga`.
pub fn varga_sign(sidereal: f64, n: u32) -> u8 {
    let l = sidereal.rem_euclid(360.0);
    let sign = (l / 30.0).floor() as u32 % 12;
    let in_sign = l.rem_euclid(30.0);
    match n {
        40 => {
            let part = ((in_sign / 0.75).floor() as u32).min(39);
            let start = if sign % 2 == 0 { 0 } else { 6 };
            ((start + part) % 12) as u8
        }
        45 => {
            let part = ((in_sign / (30.0 / 45.0)).floor() as u32).min(44);
            let start = match sign % 3 {
                0 => 0,
                1 => 4,
                _ => 8,
            };
            ((start + part) % 12) as u8
        }
        _ => sign_of(varga(l, n)),
    }
}

/// Points out of twenty for a compound relationship with the varga sign's lord.
pub fn relation_points(r: Relation) -> f64 {
    match r {
        Relation::AdhiMitra => 18.0,
        Relation::Mitra => 15.0,
        Relation::Sama => 10.0,
        Relation::Satru => 7.0,
        Relation::AdhiSatru => 5.0,
    }
}

/// Points out of twenty for `g` in a varga sign, and the reason: 20 in its own sign,
/// else by the compound relationship with the sign's lord. `rasi_signs` are the seven
/// grahas' rasi signs, which fix the temporary friendships.
pub fn varga_points(g: DashaLord, varga_sign: u8, rasi_signs: &[(DashaLord, u8)]) -> (f64, String) {
    let lord = sign_lord(varga_sign as usize);
    if lord == g {
        return (20.0, "own sign".into());
    }
    let sign_of_g = |x: DashaLord| rasi_signs.iter().find(|(p, _)| *p == x).map(|(_, s)| *s);
    let (Some(a), Some(b)) = (sign_of_g(g), sign_of_g(lord)) else {
        return (10.0, "lord's rasi position unknown: neutral".into());
    };
    let r = compound_relation(g, lord, a, b);
    (
        relation_points(r),
        format!("{}'s sign, a {}", graha_name(lord), r.name()),
    )
}

/// The word for a score out of twenty. Convention: the bands software prints, 18–20
/// full, 15–18 strong, 10–15 middling, 5–10 weak, below 5 very weak.
pub fn grade(score: f64) -> &'static str {
    if score >= 18.0 {
        "full"
    } else if score >= 15.0 {
        "strong"
    } else if score >= 10.0 {
        "middling"
    } else if score >= 5.0 {
        "weak"
    } else {
        "very weak"
    }
}

/// One graha's Vimshopaka in the four schemes.
#[derive(Clone, Debug, PartialEq)]
pub struct Vimshopaka {
    /// The graha.
    pub graha: DashaLord,
    /// Shadvarga score, 0–20.
    pub shadvarga: f64,
    /// Saptavarga score, 0–20.
    pub saptavarga: f64,
    /// Dashavarga score, 0–20.
    pub dashavarga: f64,
    /// Shodashavarga score, 0–20.
    pub shodashavarga: f64,
    /// The word for the Shodashavarga score.
    pub grade: &'static str,
    /// Each varga's sign, points and reason, and the four totals, in words.
    pub words: Vec<String>,
}

fn scheme(
    g: DashaLord,
    sidereal: f64,
    rasi_signs: &[(DashaLord, u8)],
    weights: &[(u32, f64)],
    words: &mut Vec<String>,
    name: &str,
) -> f64 {
    let mut total = 0.0;
    let mut parts = Vec::with_capacity(weights.len());
    for (n, w) in weights {
        let vs = varga_sign(sidereal, *n);
        let (pts, why) = varga_points(g, vs, rasi_signs);
        total += w * pts / 20.0;
        parts.push(format!(
            "D{n} {} {pts:.0} ({why}) × {w}",
            SIGN_NAMES[vs as usize]
        ));
    }
    words.push(format!("{name} {total:.3}: {}", parts.join("; ")));
    total
}

/// Vimshopaka of one graha at a sidereal longitude, given the seven rasi signs.
pub fn vimshopaka_of(g: DashaLord, sidereal: f64, rasi_signs: &[(DashaLord, u8)]) -> Vimshopaka {
    let mut words = Vec::with_capacity(5);
    let shadvarga = scheme(g, sidereal, rasi_signs, &SHADVARGA, &mut words, "Shadvarga");
    let saptavarga = scheme(
        g,
        sidereal,
        rasi_signs,
        &SAPTAVARGA,
        &mut words,
        "Saptavarga",
    );
    let dashavarga = scheme(
        g,
        sidereal,
        rasi_signs,
        &DASHAVARGA,
        &mut words,
        "Dashavarga",
    );
    let shodashavarga = scheme(
        g,
        sidereal,
        rasi_signs,
        &SHODASHAVARGA,
        &mut words,
        "Shodashavarga",
    );
    let grade = grade(shodashavarga);
    words.push(format!(
        "{} at {sidereal:.4}°: Shadvarga {shadvarga:.2}, Saptavarga {saptavarga:.2}, Dashavarga {dashavarga:.2}, Shodashavarga {shodashavarga:.2} of 20, {grade}",
        graha_name(g)
    ));
    Vimshopaka {
        graha: g,
        shadvarga,
        saptavarga,
        dashavarga,
        shodashavarga,
        grade,
        words,
    }
}

/// Vimshopaka of the seven grahas of a chart, Sun to Saturn.
pub fn vimshopaka(chart: &Chart) -> Result<Vec<Vimshopaka>, MargaError> {
    let mut placed = Vec::with_capacity(7);
    for g in SEVEN {
        let name = graha_name(g);
        let b = chart
            .bodies
            .iter()
            .find(|b| b.name == name)
            .ok_or(MargaError::MissingBody(name))?;
        placed.push((g, b.sidereal));
    }
    let rasi_signs: Vec<(DashaLord, u8)> = placed.iter().map(|(g, l)| (*g, sign_of(*l))).collect();
    Ok(placed
        .iter()
        .map(|(g, l)| vimshopaka_of(*g, *l, &rasi_signs))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_chart};
    use kshana_vedic::DashaLord::*;

    #[test]
    fn weights_sum_to_twenty() {
        for (name, w) in [
            ("shad", &SHADVARGA[..]),
            ("sapta", &SAPTAVARGA[..]),
            ("dasha", &DASHAVARGA[..]),
            ("shodasha", &SHODASHAVARGA[..]),
        ] {
            let s: f64 = w.iter().map(|(_, x)| x).sum();
            assert!((s - 20.0).abs() < 1e-12, "{name} {s}");
        }
        assert_eq!(relation_points(Relation::AdhiMitra), 18.0);
        assert_eq!(relation_points(Relation::AdhiSatru), 5.0);
        assert_eq!(grade(20.0), "full");
        assert_eq!(grade(12.0), "middling");
        assert_eq!(grade(1.0), "very weak");
    }

    #[test]
    fn d40_and_d45_rules() {
        assert_eq!(varga_sign(0.1, 40), 0); // Aries first part → Aries
        assert_eq!(varga_sign(29.9, 40), 3); // 40th part: Aries + 39 → Cancer
        assert_eq!(varga_sign(30.1, 40), 6); // Taurus (even) first part → Libra
        assert_eq!(varga_sign(0.1, 45), 0);
        assert_eq!(varga_sign(30.1, 45), 4); // fixed → Leo
        assert_eq!(varga_sign(60.1, 45), 8); // dual → Sagittarius
        assert_eq!(varga_sign(29.9, 45), (44 % 12) as u8);
        assert_eq!(varga_sign(45.0, 9), sign_of(varga(45.0, 9)));
    }

    #[test]
    fn maxima_and_ordering() {
        // the Sun at 4° Leo scores 20 in D1, D2, D3, D7 (own) and less elsewhere; the
        // Shadvarga maximum is 20 only when every varga is its own sign
        let signs = [
            (Sun, 4u8),
            (Moon, 0),
            (Mars, 5),
            (Mercury, 5),
            (Jupiter, 0),
            (Venus, 5),
            (Saturn, 0),
        ];
        let v = vimshopaka_of(Sun, 124.0, &signs);
        assert!(v.shadvarga <= 20.0 && v.saptavarga <= 20.0);
        assert!(v.dashavarga <= 20.0 && v.shodashavarga <= 20.0);
        assert!(v.shadvarga > 10.0, "{}", v.shadvarga);
        assert_eq!(v.words.len(), 5);
        // a graha in its own sign in every varga reaches exactly 20: synthesise by scoring
        // D1 alone with the weights
        let (p, why) = varga_points(Sun, 4, &signs);
        assert_eq!((p, why.as_str()), (20.0, "own sign"));
        // Saturn in Aries (Mars's sign, enemy; Mars in Virgo is the 6th from Aries →
        // temporary enemy → great enemy 5)
        let (p, _) = varga_points(Saturn, 0, &signs);
        assert_eq!(p, 5.0);
        // a planet nowhere in its own sign scores under 20, and the great-friend ceiling is 18
        let weak = vimshopaka_of(Saturn, 0.5, &signs);
        assert!(weak.shodashavarga < 20.0);
        assert!(weak.shadvarga <= 18.0 || weak.words[0].contains("own sign"));
        assert!(weak.shadvarga < v.shadvarga);
    }

    #[test]
    fn kochi_vimshopaka() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let v = vimshopaka(&ch).unwrap();
        assert_eq!(v.len(), 7);
        for (i, x) in v.iter().enumerate() {
            assert_eq!(x.graha, SEVEN[i]);
            for s in [x.shadvarga, x.saptavarga, x.dashavarga, x.shodashavarga] {
                assert!((5.0..=20.0).contains(&s), "{:?} {s}", x.graha);
            }
            assert_eq!(x.grade, grade(x.shodashavarga));
            assert!(x.words.iter().all(|w| !w.is_empty()));
        }
    }
}
