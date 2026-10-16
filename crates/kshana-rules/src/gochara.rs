//! Gochara (transits) counted from the natal Moon, Tara bala, and the planetary hour,
//! as the classical texts give them. Everything here is a table from a named source;
//! the forecast layer combines them.
//!
//! * Favourable transit houses from the Moon and their vedha (obstruction) points:
//!   Brihat Samhita 104, Phaladeepika 26, as practised in Kerala and Tamil panchangas.
//! * Ashtakavarga filter on a transit: BPHS, Ashtakavargadhyaya (a planet transiting a
//!   sign where its own Bhinnashtakavarga holds four or more bindus gives its promise;
//!   three or fewer withholds it).
//! * Tara bala: Muhurta Chintamani, the nine taras counted from the birth star.
//! * Sade sati and dhaiya: Saturn in the 12th, 1st, 2nd (sade sati) or 4th, 8th
//!   (dhaiya) from the natal Moon.
//! * Hora: the planetary hour and what each lord suits, Muhurta tradition.

use kshana_vedic::DashaLord;

/// Favourable houses from the natal Moon for a transiting graha (Brihat Samhita 104).
pub fn favourable_from_moon(g: DashaLord) -> &'static [u8] {
    match g {
        DashaLord::Sun => &[3, 6, 10, 11],
        DashaLord::Moon => &[1, 3, 6, 7, 10, 11],
        DashaLord::Mars => &[3, 6, 11],
        DashaLord::Mercury => &[2, 4, 6, 8, 10, 11],
        DashaLord::Jupiter => &[2, 5, 7, 9, 11],
        DashaLord::Venus => &[1, 2, 3, 4, 5, 8, 9, 11, 12],
        DashaLord::Saturn => &[3, 6, 11],
        DashaLord::Rahu | DashaLord::Ketu => &[3, 6, 11],
    }
}

/// Vedha points: `(favourable house, house whose occupant obstructs it)`, Phaladeepika 26.
pub fn vedha_pairs(g: DashaLord) -> &'static [(u8, u8)] {
    match g {
        DashaLord::Sun => &[(3, 9), (6, 12), (10, 4), (11, 5)],
        DashaLord::Moon => &[(1, 5), (3, 9), (6, 12), (7, 2), (10, 4), (11, 8)],
        DashaLord::Mars => &[(3, 12), (6, 9), (11, 5)],
        DashaLord::Mercury => &[(2, 5), (4, 3), (6, 9), (8, 1), (10, 8), (11, 12)],
        DashaLord::Jupiter => &[(2, 12), (5, 4), (7, 3), (9, 10), (11, 8)],
        DashaLord::Venus => &[
            (1, 8),
            (2, 7),
            (3, 1),
            (4, 10),
            (5, 9),
            (8, 5),
            (9, 11),
            (11, 6),
            (12, 3),
        ],
        DashaLord::Saturn => &[(3, 12), (6, 9), (11, 5)],
        DashaLord::Rahu | DashaLord::Ketu => &[(3, 12), (6, 9), (11, 5)],
    }
}

/// Houses from the Moon that the texts call plainly adverse for a transiting graha:
/// the 8th for every graha, the 12th for the luminaries and malefics, and the 4th
/// for the malefics (kantaka) — Phaladeepika 26.
pub fn adverse_from_moon(g: DashaLord) -> &'static [u8] {
    match g {
        DashaLord::Sun => &[4, 8, 12],
        DashaLord::Moon => &[4, 8, 12],
        DashaLord::Mars => &[1, 2, 4, 5, 7, 8, 9, 10, 12],
        DashaLord::Mercury => &[1, 3, 5, 7, 9, 12],
        DashaLord::Jupiter => &[1, 3, 4, 6, 8, 10, 12],
        DashaLord::Venus => &[6, 7, 10],
        DashaLord::Saturn => &[1, 2, 4, 5, 7, 8, 9, 10, 12],
        DashaLord::Rahu | DashaLord::Ketu => &[1, 2, 4, 5, 7, 8, 9, 10, 12],
    }
}

/// Whether the pair is exempt from vedha: Sun–Saturn and Moon–Mercury do not obstruct
/// each other (Phaladeepika 26.4).
pub fn vedha_exempt(a: DashaLord, b: DashaLord) -> bool {
    matches!(
        (a, b),
        (DashaLord::Sun, DashaLord::Saturn)
            | (DashaLord::Saturn, DashaLord::Sun)
            | (DashaLord::Moon, DashaLord::Mercury)
            | (DashaLord::Mercury, DashaLord::Moon)
    )
}

/// Saturn's station relative to the natal Moon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaturnPhase {
    /// Saturn in the 12th from the Moon: sade sati, first phase.
    SadeSatiRising,
    /// Saturn on the Moon: sade sati, peak.
    SadeSatiPeak,
    /// Saturn in the 2nd from the Moon: sade sati, setting.
    SadeSatiSetting,
    /// Saturn in the 4th from the Moon: ardhashtama dhaiya.
    DhaiyaFourth,
    /// Saturn in the 8th from the Moon: ashtama dhaiya.
    DhaiyaEighth,
    /// None of the above.
    Clear,
}

impl SaturnPhase {
    /// From Saturn's house counted from the natal Moon (1–12).
    pub fn from_house(h: u8) -> Self {
        match h {
            12 => SaturnPhase::SadeSatiRising,
            1 => SaturnPhase::SadeSatiPeak,
            2 => SaturnPhase::SadeSatiSetting,
            4 => SaturnPhase::DhaiyaFourth,
            8 => SaturnPhase::DhaiyaEighth,
            _ => SaturnPhase::Clear,
        }
    }

    /// Plain words.
    pub fn words(self) -> &'static str {
        match self {
            SaturnPhase::SadeSatiRising => "sade sati, first phase: Saturn in the 12th from your Moon",
            SaturnPhase::SadeSatiPeak => "sade sati, peak: Saturn on your Moon",
            SaturnPhase::SadeSatiSetting => "sade sati, last phase: Saturn in the 2nd from your Moon",
            SaturnPhase::DhaiyaFourth => "ardhashtama: Saturn in the 4th from your Moon",
            SaturnPhase::DhaiyaEighth => "ashtama Shani: Saturn in the 8th from your Moon",
            SaturnPhase::Clear => "Saturn clear of your Moon",
        }
    }
}

/// The nine taras counted from the birth star (Muhurta Chintamani).
pub const TARA_NAMES: [&str; 9] = [
    "Janma",
    "Sampat",
    "Vipat",
    "Kshema",
    "Pratyak",
    "Sadhana",
    "Naidhana",
    "Mitra",
    "Parama Mitra",
];

/// Tara score in `[0, 1]` for the Moon's star counted from the birth star.
pub const TARA_SCORE: [f64; 9] = [0.45, 1.0, 0.1, 0.8, 0.2, 0.9, 0.0, 0.8, 1.0];

/// Tara index 0–8 for a transit nakshatra given the birth nakshatra (both 0-based).
pub fn tara_index(janma: usize, transit: usize) -> usize {
    (transit + 27 - janma) % 27 % 9
}

/// Plain words for a tara.
pub fn tara_words(i: usize) -> &'static str {
    match i {
        0 => "your birth star returns: a day to keep things light",
        1 => "Sampat tara: a day that gives",
        2 => "Vipat tara: a day to avoid risk",
        3 => "Kshema tara: a safe, settled day",
        4 => "Pratyak tara: a day of small obstacles",
        5 => "Sadhana tara: a day that rewards effort",
        6 => "Naidhana tara: a day to hold back",
        7 => "Mitra tara: a friendly day",
        _ => "Parama Mitra tara: one of your best days",
    }
}

/// Hora lord by name, as `kshana_events::hora_at` returns it.
pub fn hora_lord(name: &str) -> Option<DashaLord> {
    Some(match name {
        "Sun" => DashaLord::Sun,
        "Moon" => DashaLord::Moon,
        "Mars" => DashaLord::Mars,
        "Mercury" => DashaLord::Mercury,
        "Jupiter" => DashaLord::Jupiter,
        "Venus" => DashaLord::Venus,
        "Saturn" => DashaLord::Saturn,
        _ => return None,
    })
}

/// What each planetary hour suits, in plain words (Muhurta tradition).
pub fn hora_words(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "an hour for authority, applications and anything official",
        DashaLord::Moon => "an hour for family, travel and matters of the heart",
        DashaLord::Mars => "an hour for effort and courage, not for quarrels",
        DashaLord::Mercury => "an hour for study, trade, messages and deals",
        DashaLord::Jupiter => "an hour for money, learning, children and good beginnings",
        DashaLord::Venus => "an hour for love, art, pleasure and partnership",
        DashaLord::Saturn => "an hour for routine work and patience; start nothing new",
        DashaLord::Rahu | DashaLord::Ketu => "",
    }
}
