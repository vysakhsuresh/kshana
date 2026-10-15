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
