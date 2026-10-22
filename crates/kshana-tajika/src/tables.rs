//! The fixed tables of Tajika: the seven grahas in speed order, the Hadda (term) bounds,
//! the Tajika drekkana cycle, the Tri-rashi lords, the deeptamsa orbs and the deep
//! debilitation points.
//!
//! Sources are named per table. "Charak" is K. S. Charak, *A Textbook of Varshaphala*
//! (UMA Publications, 3rd ed.), a modern textbook built on the *Tajika Neelakanthi*;
//! "Nilakanthi" is the *Tajika Neelakanthi* itself (Nirnaya Sagar print, 1893).

use kshana_vedic::{sign_lord, varga, DashaLord};

/// The seven Tajika grahas, fastest first. Rahu and Ketu take no part in Tajika yogas,
/// strengths or offices. The order is the classical ranking (Charak, ch. X), used to
/// decide which planet applies to which; it is a ranking, not a measured speed, so a
/// retrograde Mars still counts as slower than the Sun.
pub const SEVEN: [DashaLord; 7] = [
    DashaLord::Moon,
    DashaLord::Mercury,
    DashaLord::Venus,
    DashaLord::Sun,
    DashaLord::Mars,
    DashaLord::Jupiter,
    DashaLord::Saturn,
];

/// Position of a graha in [`SEVEN`] (0 = fastest), `None` for the nodes.
pub fn speed_rank(g: DashaLord) -> Option<usize> {
    SEVEN.iter().position(|&x| x == g)
}

/// Sign names, 0 = Aries.
pub const SIGN_NAMES: [&str; 12] = [
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
    "Aquarius",
    "Pisces",
];

/// Sign index 0–11 of a longitude.
pub fn sign_index(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

/// Degrees inside the sign, `[0, 30)`.
pub fn within_sign(lon: f64) -> f64 {
    lon.rem_euclid(30.0)
}

/// Graha name in the reader's language (English: [`graha_name`]).
pub fn graha_in(g: DashaLord, lang: kshana_rules::Lang) -> &'static str {
    kshana_speak::lexicon::name(graha_name(g), lang)
}

/// Sign name in the reader's language, 0 = Aries (English: [`SIGN_NAMES`]).
pub fn sign_in(i: usize, lang: kshana_rules::Lang) -> &'static str {
    kshana_speak::lexicon::name(SIGN_NAMES[i % 12], lang)
}

/// Graha name for words.
pub fn graha_name(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Sun",
        DashaLord::Moon => "Moon",
        DashaLord::Mars => "Mars",
        DashaLord::Mercury => "Mercury",
        DashaLord::Jupiter => "Jupiter",
        DashaLord::Venus => "Venus",
        DashaLord::Saturn => "Saturn",
        DashaLord::Rahu => "Rahu",
        DashaLord::Ketu => "Ketu",
    }
}

/// The Hadda (term) bounds: for each sign, five `(lord, upper bound in degrees)` entries
/// in zodiacal order; a planet at `d` degrees inside the sign is in the first term whose
/// upper bound exceeds `d`.
///
/// Source: the *Tajika Neelakanthi* (Nirnaya Sagar print, 1893, closing gloss to the
/// Prashna Tantra), as transcribed in the teistro-sdk Tajika crate
/// (`crates/tajika/src/bala.rs`). The widths are exactly those of Ptolemy's "Egyptian"
/// terms (Tetrabiblos I.21) and the lords agree with the Egyptian assignment in every
/// sign except two, where two adjacent lords are transposed: Gemini (Venus before
/// Jupiter) and Sagittarius (Mars before Saturn). Charak's Table VI-4 further transposes
/// Aquarius (Venus 7, Mercury 13); that reading is not used here. Invariants checked by
/// the tests: each sign's bounds end at 30 and carry Mars, Mercury, Jupiter, Venus and
/// Saturn once each.
pub const HADDA: [[(DashaLord, f64); 5]; 12] = [
    // Aries
    [
        (DashaLord::Jupiter, 6.0),
        (DashaLord::Venus, 12.0),
        (DashaLord::Mercury, 20.0),
        (DashaLord::Mars, 25.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Taurus
    [
        (DashaLord::Venus, 8.0),
        (DashaLord::Mercury, 14.0),
        (DashaLord::Jupiter, 22.0),
        (DashaLord::Saturn, 27.0),
        (DashaLord::Mars, 30.0),
    ],
    // Gemini (Nilakanthi: Venus 6–12, Jupiter 12–17; Egyptian terms have them swapped)
    [
        (DashaLord::Mercury, 6.0),
        (DashaLord::Venus, 12.0),
        (DashaLord::Jupiter, 17.0),
        (DashaLord::Mars, 24.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Cancer
    [
        (DashaLord::Mars, 7.0),
        (DashaLord::Venus, 13.0),
        (DashaLord::Mercury, 19.0),
        (DashaLord::Jupiter, 26.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Leo
    [
        (DashaLord::Jupiter, 6.0),
        (DashaLord::Venus, 11.0),
        (DashaLord::Saturn, 18.0),
        (DashaLord::Mercury, 24.0),
        (DashaLord::Mars, 30.0),
    ],
    // Virgo
    [
        (DashaLord::Mercury, 7.0),
        (DashaLord::Venus, 17.0),
        (DashaLord::Jupiter, 21.0),
        (DashaLord::Mars, 28.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Libra
    [
        (DashaLord::Saturn, 6.0),
        (DashaLord::Mercury, 14.0),
        (DashaLord::Jupiter, 21.0),
        (DashaLord::Venus, 28.0),
        (DashaLord::Mars, 30.0),
    ],
    // Scorpio
    [
        (DashaLord::Mars, 7.0),
        (DashaLord::Venus, 11.0),
        (DashaLord::Mercury, 19.0),
        (DashaLord::Jupiter, 24.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Sagittarius (Nilakanthi: Mars 21–26, Saturn 26–30; Egyptian terms have them swapped)
    [
        (DashaLord::Jupiter, 12.0),
        (DashaLord::Venus, 17.0),
        (DashaLord::Mercury, 21.0),
        (DashaLord::Mars, 26.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Capricorn
    [
        (DashaLord::Mercury, 7.0),
        (DashaLord::Jupiter, 14.0),
        (DashaLord::Venus, 22.0),
        (DashaLord::Saturn, 26.0),
        (DashaLord::Mars, 30.0),
    ],
    // Aquarius
    [
        (DashaLord::Mercury, 7.0),
        (DashaLord::Venus, 13.0),
        (DashaLord::Jupiter, 20.0),
        (DashaLord::Mars, 25.0),
        (DashaLord::Saturn, 30.0),
    ],
    // Pisces
    [
        (DashaLord::Venus, 12.0),
        (DashaLord::Jupiter, 16.0),
        (DashaLord::Mercury, 19.0),
        (DashaLord::Mars, 28.0),
        (DashaLord::Saturn, 30.0),
    ],
];

/// Lord of the Hadda a sidereal longitude falls in.
pub fn hadda_lord(sidereal: f64) -> DashaLord {
    let row = &HADDA[sign_index(sidereal)];
    let d = within_sign(sidereal);
    row.iter()
        .find(|(_, up)| d < *up)
        .map(|(l, _)| *l)
        .unwrap_or(row[4].0)
}

/// The Tajika drekkana cycle: the first decanate of Aries is Mars's and each following
/// decanate takes the next planet in the Chaldean descending order. The 36 cells this
/// yields agree with Charak's printed table (Table VI-6) in every cell; it is the
/// Hellenistic "faces" scheme, not the Parashari drekkana (same, fifth, ninth sign).
pub const CHALDEAN_DECANS: [DashaLord; 7] = [
    DashaLord::Mars,
    DashaLord::Sun,
    DashaLord::Venus,
    DashaLord::Mercury,
    DashaLord::Moon,
    DashaLord::Saturn,
    DashaLord::Jupiter,
];

/// Lord of the Tajika drekkana a sidereal longitude falls in.
pub fn drekkana_lord(sidereal: f64) -> DashaLord {
    let s = sign_index(sidereal);
    let d = ((within_sign(sidereal) / 10.0).floor() as usize).min(2);
    CHALDEAN_DECANS[(s * 3 + d) % 7]
}

/// Lord of the Parashari navamsa (D9) sign a sidereal longitude falls in. Charak uses the
/// ordinary navamsa for the fifth part of the five-fold strength.
pub fn navamsa_lord(sidereal: f64) -> DashaLord {
    sign_lord(sign_index(varga(sidereal, 9)))
}

/// Tri-rashi (triplicity) lords `(by day, by night)` for each sign, 0 = Aries.
///
/// Source: Charak, Table VII-1 (24 cells). The rule behind the table is the Dorothean
/// triplicity scheme (*Carmen Astrologicum* I.1): the first sign of each element takes
/// its day and night lords in that order, the second sign swaps them, the third takes the
/// participating lord for both. Fire Sun/Jupiter (participating Saturn), Earth
/// Venus/Moon (Mars), Air Saturn/Mercury (Jupiter), Water Venus/Mars (Moon).
pub const TRIRASHI: [(DashaLord, DashaLord); 12] = [
    (DashaLord::Sun, DashaLord::Jupiter),     // Aries
    (DashaLord::Venus, DashaLord::Moon),      // Taurus
    (DashaLord::Saturn, DashaLord::Mercury),  // Gemini
    (DashaLord::Venus, DashaLord::Mars),      // Cancer
    (DashaLord::Jupiter, DashaLord::Sun),     // Leo
    (DashaLord::Moon, DashaLord::Venus),      // Virgo
    (DashaLord::Mercury, DashaLord::Saturn),  // Libra
    (DashaLord::Mars, DashaLord::Venus),      // Scorpio
    (DashaLord::Saturn, DashaLord::Saturn),   // Sagittarius
    (DashaLord::Mars, DashaLord::Mars),       // Capricorn
    (DashaLord::Jupiter, DashaLord::Jupiter), // Aquarius
    (DashaLord::Moon, DashaLord::Moon),       // Pisces
];

/// Tri-rashi lord of a sign for a day (`day = true`) or night return.
pub fn trirashi_lord(sign: usize, day: bool) -> DashaLord {
    let (d, n) = TRIRASHI[sign % 12];
    if day {
        d
    } else {
        n
    }
}

/// Deeptamsa (orb of radiance) in degrees. Source: Charak, Table X-1, which matches the
/// *Tajika Neelakanthi*: Sun 15, Moon 12, Mars 8, Mercury 7, Jupiter 9, Venus 7,
/// Saturn 9. A pair's orb is the mean of the two (see [`crate::pair_orb`]). The nodes
/// have none.
pub fn deeptamsa(g: DashaLord) -> Option<f64> {
    Some(match g {
        DashaLord::Sun => 15.0,
        DashaLord::Moon => 12.0,
        DashaLord::Mars => 8.0,
        DashaLord::Mercury => 7.0,
        DashaLord::Jupiter => 9.0,
        DashaLord::Venus => 7.0,
        DashaLord::Saturn => 9.0,
        DashaLord::Rahu | DashaLord::Ketu => return None,
    })
}

/// Point of deepest debilitation, sidereal degrees: the Uchcha bala counts the arc from
/// here. Sun 10° Libra, Moon 3° Scorpio, Mars 28° Cancer, Mercury 15° Pisces, Jupiter
/// 5° Capricorn, Venus 27° Virgo, Saturn 20° Aries (Parashari deep exaltation + 180°;
/// Charak ch. VI).
pub fn deep_debilitation(g: DashaLord) -> Option<f64> {
    Some(match g {
        DashaLord::Sun => 190.0,
        DashaLord::Moon => 213.0,
        DashaLord::Mars => 118.0,
        DashaLord::Mercury => 345.0,
        DashaLord::Jupiter => 275.0,
        DashaLord::Venus => 177.0,
        DashaLord::Saturn => 20.0,
        DashaLord::Rahu | DashaLord::Ketu => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hadda_rows_are_well_formed() {
        for row in &HADDA {
            assert_eq!(row[4].1, 30.0);
            let mut prev = 0.0;
            for (_, up) in row {
                assert!(*up > prev);
                prev = *up;
            }
            for g in [
                DashaLord::Mars,
                DashaLord::Mercury,
                DashaLord::Jupiter,
                DashaLord::Venus,
                DashaLord::Saturn,
            ] {
                assert_eq!(row.iter().filter(|(l, _)| *l == g).count(), 1);
            }
        }
        assert_eq!(hadda_lord(0.0), DashaLord::Jupiter);
        assert_eq!(hadda_lord(6.0), DashaLord::Venus);
        assert_eq!(hadda_lord(29.999), DashaLord::Saturn);
        // Samjna Tantra v. 40: Venus holds Gemini 8°
        assert_eq!(hadda_lord(68.0), DashaLord::Venus);
    }

    #[test]
    fn drekkana_cycle_matches_the_printed_table() {
        let printed = [
            [DashaLord::Mars, DashaLord::Sun, DashaLord::Venus],
            [DashaLord::Mercury, DashaLord::Moon, DashaLord::Saturn],
            [DashaLord::Jupiter, DashaLord::Mars, DashaLord::Sun],
            [DashaLord::Venus, DashaLord::Mercury, DashaLord::Moon],
            [DashaLord::Saturn, DashaLord::Jupiter, DashaLord::Mars],
            [DashaLord::Sun, DashaLord::Venus, DashaLord::Mercury],
            [DashaLord::Moon, DashaLord::Saturn, DashaLord::Jupiter],
            [DashaLord::Mars, DashaLord::Sun, DashaLord::Venus],
            [DashaLord::Mercury, DashaLord::Moon, DashaLord::Saturn],
            [DashaLord::Jupiter, DashaLord::Mars, DashaLord::Sun],
            [DashaLord::Venus, DashaLord::Mercury, DashaLord::Moon],
            [DashaLord::Saturn, DashaLord::Jupiter, DashaLord::Mars],
        ];
        for (s, row) in printed.iter().enumerate() {
            for (d, lord) in row.iter().enumerate() {
                let lon = s as f64 * 30.0 + d as f64 * 10.0 + 5.0;
                assert_eq!(drekkana_lord(lon), *lord, "sign {s} decan {d}");
            }
        }
    }

    #[test]
    fn trirashi_day_column_is_the_standard_table() {
        let day = [
            DashaLord::Sun,
            DashaLord::Venus,
            DashaLord::Saturn,
            DashaLord::Venus,
            DashaLord::Jupiter,
            DashaLord::Moon,
            DashaLord::Mercury,
            DashaLord::Mars,
            DashaLord::Saturn,
            DashaLord::Mars,
            DashaLord::Jupiter,
            DashaLord::Moon,
        ];
        for (s, d) in day.iter().enumerate() {
            assert_eq!(trirashi_lord(s, true), *d);
        }
        // third sign of each element: same lord by day and night
        for s in [8, 9, 10, 11] {
            assert_eq!(trirashi_lord(s, true), trirashi_lord(s, false));
        }
    }
}
