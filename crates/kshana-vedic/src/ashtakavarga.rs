//! Ashtakavarga (BPHS, Ashtakavargadhyaya): each of the seven grahas and the lagna
//! contributes a bindu to the signs at fixed distances from each of the eight
//! reference points (the seven grahas and the lagna). Bhinnashtakavarga is one graha's
//! 12-sign table; Sarvashtakavarga is the sum of the seven.
//!
//! The benefic-position table below is the standard one. Its transcription is checked
//! by the fixed row totals the texts give: Sun 48, Moon 49, Mars 39, Mercury 54,
//! Jupiter 56, Venus 52, Saturn 39, total 337.

/// Reference points in table order: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Lagna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Reference {
    Sun,
    Moon,
    Mars,
    Mercury,
    Jupiter,
    Venus,
    Saturn,
    Lagna,
}

/// The grahas that own a Bhinnashtakavarga, in table order.
pub const GRAHAS: [Reference; 7] = [
    Reference::Sun,
    Reference::Moon,
    Reference::Mars,
    Reference::Mercury,
    Reference::Jupiter,
    Reference::Venus,
    Reference::Saturn,
];

/// Houses (1-based, counted from the reference) that receive a bindu from each graha,
/// indexed `[graha][reference]` in the order of [`Reference`].
const TABLE: [[&[u8]; 8]; 7] = [
    // Sun
    [
        &[1, 2, 4, 7, 8, 9, 10, 11], // from Sun
        &[3, 6, 10, 11],             // from Moon
        &[1, 2, 4, 7, 8, 9, 10, 11], // from Mars
        &[3, 5, 6, 9, 10, 11, 12],   // from Mercury
        &[5, 6, 9, 11],              // from Jupiter
        &[6, 7, 12],                 // from Venus
        &[1, 2, 4, 7, 8, 9, 10, 11], // from Saturn
        &[3, 4, 6, 10, 11, 12],      // from Lagna
    ],
    // Moon
    [
        &[3, 6, 7, 8, 10, 11],
        &[1, 3, 6, 7, 10, 11],
        &[2, 3, 5, 6, 9, 10, 11],
        &[1, 3, 4, 5, 7, 8, 10, 11],
        &[1, 4, 7, 8, 10, 11, 12],
        &[3, 4, 5, 7, 9, 10, 11],
        &[3, 5, 6, 11],
        &[3, 6, 10, 11],
    ],
    // Mars
    [
        &[3, 5, 6, 10, 11],
        &[3, 6, 11],
        &[1, 2, 4, 7, 8, 10, 11],
        &[3, 5, 6, 11],
        &[6, 10, 11, 12],
        &[6, 8, 11, 12],
        &[1, 4, 7, 8, 9, 10, 11],
        &[1, 3, 6, 10, 11],
    ],
    // Mercury
    [
        &[5, 6, 9, 11, 12],
        &[2, 4, 6, 8, 10, 11],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[1, 3, 5, 6, 9, 10, 11, 12],
        &[6, 8, 11, 12],
        &[1, 2, 3, 4, 5, 8, 9, 11],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[1, 2, 4, 6, 8, 10, 11],
    ],
    // Jupiter
    [
        &[1, 2, 3, 4, 7, 8, 9, 10, 11],
        &[2, 5, 7, 9, 11],
        &[1, 2, 4, 7, 8, 10, 11],
        &[1, 2, 4, 5, 6, 9, 10, 11],
        &[1, 2, 3, 4, 7, 8, 10, 11],
        &[2, 5, 6, 9, 10, 11],
        &[3, 5, 6, 12],
        &[1, 2, 4, 5, 6, 7, 9, 10, 11],
    ],
    // Venus
    [
        &[8, 11, 12],
        &[1, 2, 3, 4, 5, 8, 9, 11, 12],
        &[3, 5, 6, 9, 11, 12],
        &[3, 5, 6, 9, 11],
        &[5, 8, 9, 10, 11],
        &[1, 2, 3, 4, 5, 8, 9, 10, 11],
        &[3, 4, 5, 8, 9, 10, 11],
        &[1, 2, 3, 4, 5, 8, 9, 11],
    ],
    // Saturn
    [
        &[1, 2, 4, 7, 8, 10, 11],
        &[3, 6, 11],
        &[3, 5, 6, 10, 11, 12],
        &[6, 8, 9, 10, 11, 12],
        &[5, 6, 11, 12],
        &[6, 11, 12],
        &[3, 5, 6, 11],
        &[1, 3, 4, 6, 10, 11],
    ],
];

/// Fixed row totals from the texts, used to check the table.
pub const ROW_TOTALS: [u32; 7] = [48, 49, 39, 54, 56, 52, 39];

/// Sign indices (0 = Aries) of the eight reference points: the seven grahas and the lagna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Positions {
    /// Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Lagna sign indices.
    pub signs: [u8; 8],
}

/// Bhinnashtakavarga of graha `g` (index into [`GRAHAS`]): bindus per sign, 0 = Aries.
pub fn bhinna(p: &Positions, g: usize) -> [u8; 12] {
    let mut out = [0u8; 12];
    for (r, houses) in TABLE[g].iter().enumerate() {
        let from = p.signs[r] as usize;
        for &h in houses.iter() {
            out[(from + h as usize - 1) % 12] += 1;
        }
    }
    out
}

/// Sarvashtakavarga: the seven tables summed, per sign.
pub fn sarva(p: &Positions) -> [u8; 12] {
    let mut out = [0u8; 12];
    for g in 0..7 {
        for (i, b) in bhinna(p, g).iter().enumerate() {
            out[i] += b;
        }
    }
    out
}

// ---------------------------------------------------------------------------------
// Reductions and pindas (BPHS, the Trikona-shodhana, Ekadhipatya-shodhana and
// Pinda-sadhana chapters, R. Santhanam's translation ch. 67–69; B. V. Raman,
// *Ashtakavarga System of Prediction*, ch. on "Reductions" and "Sodhya Pinda").
// ---------------------------------------------------------------------------------

/// The four triplicities (trikonas) by sign index: fiery, earthy, airy, watery.
pub const TRIKONAS: [[usize; 3]; 4] = [[0, 4, 8], [1, 5, 9], [2, 6, 10], [3, 7, 11]];

/// The five pairs of signs with one lord (Mars, Venus, Mercury, Jupiter, Saturn).
/// Cancer and Leo, the single signs of the Moon and the Sun, are never reduced.
pub const EKADHIPATYA_PAIRS: [[usize; 2]; 5] = [[0, 7], [1, 6], [2, 5], [8, 11], [9, 10]];

/// Rasi gunakara (sign multipliers), Aries … Pisces: 7, 10, 8, 4, 10, 5, 7, 8, 9, 5,
/// 11, 12 (BPHS Pinda-sadhana ch., Santhanam 69; Raman, same table).
pub const RASI_GUNAKARA: [u32; 12] = [7, 10, 8, 4, 10, 5, 7, 8, 9, 5, 11, 12];

/// Graha gunakara (planet multipliers), Sun … Saturn: 5, 5, 8, 5, 10, 7, 5 (ibid.).
pub const GRAHA_GUNAKARA: [u32; 7] = [5, 5, 8, 5, 10, 7, 5];

/// Trikona shodhana of one Bhinnashtakavarga. In each triplicity: if any sign has no
/// bindu, nothing is removed; if all three are equal, all three are cleared; otherwise
/// the smallest figure is subtracted from all three (BPHS, Santhanam 67.2–5).
pub fn trikona_shodhana(b: &[u8; 12]) -> [u8; 12] {
    let mut out = *b;
    for t in TRIKONAS {
        let v = t.map(|i| b[i]);
        if v.contains(&0) {
            continue;
        }
        let m = if v[0] == v[1] && v[1] == v[2] {
            v[0]
        } else {
            *v.iter().min().unwrap()
        };
        for i in t {
            out[i] -= m;
        }
    }
    out
}

/// Ekadhipatya shodhana, applied after trikona shodhana, to each pair of signs owned by
/// one planet (BPHS, Santhanam 68; Raman's list of rules):
/// - both signs occupied by a graha: no reduction;
/// - either sign already at 0: no reduction;
/// - one occupied, one empty: if the empty sign has fewer or equal bindus it is
///   cleared; if more, it is brought down to the occupied sign's figure (the
///   occupied sign is never touched);
/// - both empty: equal figures are both cleared; unequal, the greater is brought down
///   to the smaller.
///
/// `occupied` marks signs holding one of the seven grahas (the nodes and the lagna do
/// not count).
pub fn ekadhipatya_shodhana(b: &[u8; 12], occupied: &[bool; 12]) -> [u8; 12] {
    let mut out = *b;
    for [x, y] in EKADHIPATYA_PAIRS {
        let (bx, by) = (b[x], b[y]);
        if (occupied[x] && occupied[y]) || bx == 0 || by == 0 {
            continue;
        }
        match (occupied[x], occupied[y]) {
            (true, false) | (false, true) => {
                let (full, empty) = if occupied[x] { (x, y) } else { (y, x) };
                out[empty] = if b[empty] <= b[full] { 0 } else { b[full] };
            }
            _ => {
                if bx == by {
                    out[x] = 0;
                    out[y] = 0;
                } else {
                    let m = bx.min(by);
                    out[x] = m;
                    out[y] = m;
                }
            }
        }
    }
    out
}

/// Signs (0 = Aries) holding at least one of the seven grahas.
pub fn occupied_signs(p: &Positions) -> [bool; 12] {
    let mut o = [false; 12];
    for &s in &p.signs[..7] {
        o[s as usize % 12] = true;
    }
    o
}

/// The pindas of one reduced table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pinda {
    /// Σ reduced bindus × rasi gunakara over the twelve signs.
    pub rasi: u32,
    /// Σ over the seven grahas of the reduced bindus in the sign each occupies × its
    /// graha gunakara.
    pub graha: u32,
    /// Shodhya pinda = rasi + graha.
    pub shodhya: u32,
}

/// Rasi, graha and shodhya pinda of a reduced Bhinnashtakavarga (BPHS Pinda-sadhana).
pub fn pinda(reduced: &[u8; 12], p: &Positions) -> Pinda {
    let rasi: u32 = (0..12).map(|i| reduced[i] as u32 * RASI_GUNAKARA[i]).sum();
    let graha: u32 = (0..7)
        .map(|g| reduced[p.signs[g] as usize % 12] as u32 * GRAHA_GUNAKARA[g])
        .sum();
    Pinda {
        rasi,
        graha,
        shodhya: rasi + graha,
    }
}

/// One graha's table through every stage of reduction, with its pindas and the
/// transit-sensitive points derived from them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reduction {
    /// Raw Bhinnashtakavarga.
    pub bhinna: [u8; 12],
    /// After trikona shodhana.
    pub trikona: [u8; 12],
    /// After ekadhipatya shodhana (the final, "shodhita" table).
    pub ekadhipatya: [u8; 12],
    /// Pindas of the final table.
    pub pinda: Pinda,
    /// Raw bindus in the 8th sign from the graha in its own table.
    pub eighth_bindus: u8,
    /// Shodhya pinda × `eighth_bindus`.
    pub product: u32,
    /// Sensitive nakshatra, 0 = Ashwini: the remainder of `product` / 27 counted from
    /// Ashwini (remainder 0 = Revati). Its trines (+9, +18) are sensitive too.
    pub nakshatra: u8,
    /// Sensitive sign, 0 = Aries: remainder of `product` / 12 counted from Aries
    /// (remainder 0 = Pisces).
    pub sign: u8,
}

/// Reduce graha `g`'s table (index into [`GRAHAS`]). The sensitive points follow BPHS
/// (Santhanam ch. 70–71) and Raman: the shodhya pinda multiplied by the bindus in the
/// 8th from the graha, divided by 27 (nakshatra) and by 12 (sign); Saturn's transit over
/// Saturn's points, the Sun's points for the father and the Moon's for the mother, is
/// read as a time to take care. Convention used here: the 8th-house figure is taken
/// from the raw (unreduced) table, as the texts count "the bindus in the 8th".
pub fn reduce(p: &Positions, g: usize) -> Reduction {
    let bh = bhinna(p, g);
    let tri = trikona_shodhana(&bh);
    let eka = ekadhipatya_shodhana(&tri, &occupied_signs(p));
    let pin = pinda(&eka, p);
    let eighth = bh[(p.signs[g] as usize + 7) % 12];
    let product = pin.shodhya * eighth as u32;
    let r27 = product % 27;
    let r12 = product % 12;
    Reduction {
        bhinna: bh,
        trikona: tri,
        ekadhipatya: eka,
        pinda: pin,
        eighth_bindus: eighth,
        product,
        nakshatra: if r27 == 0 { 26 } else { (r27 - 1) as u8 },
        sign: if r12 == 0 { 11 } else { (r12 - 1) as u8 },
    }
}

#[cfg(test)]
mod reduction_tests {
    use super::*;

    /// Hand-worked: a 39-bindu table through both reductions and the pindas.
    #[test]
    fn hand_worked_reduction() {
        let b = [5, 3, 4, 6, 2, 4, 3, 5, 1, 0, 4, 2];
        // fire 5,2,1 −1; earth 3,4,0 untouched (a zero); air 4,3,4 −3; water 6,5,2 −2
        let t = trikona_shodhana(&b);
        assert_eq!(t, [4, 3, 1, 4, 1, 4, 0, 3, 0, 0, 1, 0]);
        // Sun, Mercury, Venus in Aries; Moon, Jupiter in Taurus; Mars, Saturn in Libra
        let p = Positions { signs: [0, 1, 6, 0, 1, 0, 6, 3] };
        let occ = occupied_signs(&p);
        let e = ekadhipatya_shodhana(&t, &occ);
        // Mars pair: Aries 4 occupied, Scorpio 3 empty and smaller → cleared.
        // Venus pair both occupied; Mercury pair both empty 1/4 → 1/1;
        // Jupiter pair zeros; Saturn pair Capricorn 0 → untouched.
        assert_eq!(e, [4, 3, 1, 4, 1, 1, 0, 0, 0, 0, 1, 0]);
        let pin = pinda(&e, &p);
        // 4·7 + 3·10 + 1·8 + 4·4 + 1·10 + 1·5 + 1·11 = 108
        // Sun 4·5 + Moon 3·5 + Mars 0 + Mercury 4·5 + Jupiter 3·10 + Venus 4·7 + Saturn 0 = 113
        assert_eq!(pin, Pinda { rasi: 108, graha: 113, shodhya: 221 });
    }

    #[test]
    fn trikona_equal_triplicity_is_cleared() {
        let b = [3, 1, 1, 1, 3, 1, 1, 1, 3, 1, 1, 1];
        let t = trikona_shodhana(&b);
        assert_eq!(t, [0; 12]);
    }

    #[test]
    fn ekadhipatya_rules_one_by_one() {
        let mut occ = [false; 12];
        // Mars pair both empty, unequal: Aries 2, Scorpio 4 → both 2
        // Venus pair both empty, equal: Taurus 3, Libra 3 → both 0
        // Mercury pair: Gemini occupied 2, Virgo empty 5 → Virgo 2
        // Jupiter pair: Sagittarius occupied 4, Pisces empty 4 → Pisces 0
        // Saturn pair both occupied: untouched
        occ[2] = true;
        occ[8] = true;
        occ[9] = true;
        occ[10] = true;
        let b = [2, 3, 2, 7, 6, 5, 3, 4, 4, 1, 6, 4];
        let e = ekadhipatya_shodhana(&b, &occ);
        assert_eq!(e, [2, 0, 2, 7, 6, 2, 0, 2, 4, 1, 6, 0]);
    }

    #[test]
    fn reduce_is_consistent_for_every_graha() {
        let p = Positions { signs: [10, 0, 9, 10, 2, 9, 8, 9] };
        for g in 0..7 {
            let r = reduce(&p, g);
            for i in 0..12 {
                assert!(r.trikona[i] <= r.bhinna[i] && r.ekadhipatya[i] <= r.trikona[i]);
            }
            assert_eq!(r.pinda.shodhya, r.pinda.rasi + r.pinda.graha);
            assert_eq!(r.product, r.pinda.shodhya * r.eighth_bindus as u32);
            assert!(r.nakshatra < 27 && r.sign < 12);
            assert_eq!((r.nakshatra as u32 + 1) % 27, r.product % 27);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_row_totals_match_the_texts() {
        for (g, rows) in TABLE.iter().enumerate() {
            let total: u32 = rows.iter().map(|r| r.len() as u32).sum();
            assert_eq!(total, ROW_TOTALS[g], "graha {g}");
            for r in rows.iter() {
                assert!(
                    r.windows(2).all(|w| w[0] < w[1]),
                    "rows are ascending and distinct"
                );
                assert!(r.iter().all(|h| (1..=12).contains(h)));
            }
        }
        assert_eq!(ROW_TOTALS.iter().sum::<u32>(), 337);
    }

    #[test]
    fn sarva_always_totals_337() {
        let p = Positions {
            signs: [6, 7, 3, 5, 3, 5, 11, 9],
        };
        let s = sarva(&p);
        assert_eq!(s.iter().map(|&b| b as u32).sum::<u32>(), 337);
        for (g, total) in ROW_TOTALS.iter().enumerate() {
            assert_eq!(bhinna(&p, g).iter().map(|&b| b as u32).sum::<u32>(), *total);
            assert!(bhinna(&p, g).iter().all(|&b| b <= 8));
        }
    }

    #[test]
    fn a_graha_gives_itself_a_bindu_in_its_own_sign_where_the_table_says() {
        // The Sun gives a bindu to the sign it occupies (house 1 from itself).
        let p = Positions {
            signs: [4, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(bhinna(&p, 0)[4] >= 1);
    }
}
