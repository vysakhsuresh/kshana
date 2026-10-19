//! Every constant Shadbala needs, each with its source.
//!
//! Units: a *virupa* (shashtiamsa) is one sixtieth of a *rupa*; every strength here is
//! in virupas. Sign indices are 0 = Aries; house indices are 0 = lagna where a function
//! says so, else houses are 1–12.

use kshana_vedic::DashaLord;

/// Virupas in one rupa.
pub const RUPA: f64 = 60.0;

/// The seven grahas in the order BPHS lists them.
pub const SEVEN: [DashaLord; 7] = [
    DashaLord::Sun,
    DashaLord::Moon,
    DashaLord::Mars,
    DashaLord::Mercury,
    DashaLord::Jupiter,
    DashaLord::Venus,
    DashaLord::Saturn,
];

/// The five tara grahas, the only ones that fight a planetary war (BPHS 27.20; Raman,
/// *Graha and Bhava Balas*, Kala bala: "the Sun and the Moon never take part").
pub const TARA: [DashaLord; 5] = [
    DashaLord::Mars,
    DashaLord::Mercury,
    DashaLord::Jupiter,
    DashaLord::Venus,
    DashaLord::Saturn,
];

/// English name, which is also the body's name in a [`kshana_chart::Chart`].
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

/// Naisargika (natural) bala. BPHS ch. 27 gives the order Sun, Moon, Venus, Jupiter,
/// Mercury, Mars, Saturn with the strength falling by one seventh of a rupa each step:
/// 60, 51.43, 42.86, 34.29, 25.71, 17.14, 8.57 (Raman, *Graha and Bhava Balas*, ch. on
/// Naisargika bala; JHora prints the same seven). Nodes get nothing.
pub fn naisargika(g: DashaLord) -> f64 {
    let k = match g {
        DashaLord::Sun => 7.0,
        DashaLord::Moon => 6.0,
        DashaLord::Venus => 5.0,
        DashaLord::Jupiter => 4.0,
        DashaLord::Mercury => 3.0,
        DashaLord::Mars => 2.0,
        DashaLord::Saturn => 1.0,
        _ => 0.0,
    };
    RUPA * k / 7.0
}

/// Minimum Shadbala a graha needs to give its full results, in virupas (BPHS ch. 27,
/// the closing slokas; Raman, *Graha and Bhava Balas*, "Required strengths"): Sun 6.5
/// rupas, Moon 6, Mars 5, Mercury 7, Jupiter 6.5, Venus 5.5, Saturn 5. Some editions
/// and JHora's port give the Sun 5 rupas; the 6.5 of Santhanam's BPHS and Raman is used.
pub fn required_virupas(g: DashaLord) -> f64 {
    match g {
        DashaLord::Sun => 390.0,
        DashaLord::Moon => 360.0,
        DashaLord::Mars => 300.0,
        DashaLord::Mercury => 420.0,
        DashaLord::Jupiter => 390.0,
        DashaLord::Venus => 330.0,
        DashaLord::Saturn => 300.0,
        _ => 0.0,
    }
}

/// Bhava bala threshold used for the ratio: seven rupas (JHora's port,
/// `const.minimum_bhava_bala_rupa`; Raman calls a bhava above this "strong").
pub const BHAVA_REQUIRED_VIRUPAS: f64 = 7.0 * RUPA;

/// Point of deepest exaltation, sidereal degrees (BPHS ch. 3; Raman): Sun 10° Aries,
/// Moon 3° Taurus, Mars 28° Capricorn, Mercury 15° Virgo, Jupiter 5° Cancer, Venus 27°
/// Pisces, Saturn 20° Libra.
pub fn deep_exaltation(g: DashaLord) -> Option<f64> {
    Some(match g {
        DashaLord::Sun => 10.0,
        DashaLord::Moon => 33.0,
        DashaLord::Mars => 298.0,
        DashaLord::Mercury => 165.0,
        DashaLord::Jupiter => 95.0,
        DashaLord::Venus => 357.0,
        DashaLord::Saturn => 200.0,
        _ => return None,
    })
}

/// Point of deepest debilitation: 180° from deepest exaltation.
pub fn deep_debilitation(g: DashaLord) -> Option<f64> {
    deep_exaltation(g).map(|e| (e + 180.0).rem_euclid(360.0))
}

/// Kendradi bala (BPHS ch. 27; Raman): a graha in a kendra (1, 4, 7, 10) gets 60, in a
/// panapara (2, 5, 8, 11) 30, in an apoklima (3, 6, 9, 12) 15.
pub fn kendradi(house: u8) -> f64 {
    match house {
        1 | 4 | 7 | 10 => 60.0,
        2 | 5 | 8 | 11 => 30.0,
        _ => 15.0,
    }
}

/// Drekkana bala, 15 virupas (a quarter rupa): male grahas (Sun, Mars, Jupiter) in the
/// first decanate of a sign, neuter grahas (Mercury, Saturn) in the second, female
/// grahas (Moon, Venus) in the third. This is the Saravali / Raman / JHora reading
/// (PyJHora `const.dreshkon_bala_list`); Santhanam's BPHS 27.6 lists the genders as
/// "male, female, hermaphrodite" for the first, second and third decanates.
pub fn drekkana_bala(g: DashaLord, degree_in_sign: f64) -> f64 {
    let decanate = (degree_in_sign.rem_euclid(30.0) / 10.0).floor() as u8;
    let wanted = match g {
        DashaLord::Sun | DashaLord::Mars | DashaLord::Jupiter => 0,
        DashaLord::Mercury | DashaLord::Saturn => 1,
        DashaLord::Moon | DashaLord::Venus => 2,
        _ => return 0.0,
    };
    if decanate == wanted {
        15.0
    } else {
        0.0
    }
}

/// Cusp index (0 = lagna) at which a graha has full Dig bala (BPHS ch. 27, read from
/// the points of weakness it names: the 4th for the Sun and Mars, the 7th for Jupiter
/// and Mercury, the 10th for Venus and the Moon, the lagna for Saturn; Raman: "Jupiter
/// and Mercury in the ascendant, Sun and Mars in the 10th, Saturn in the 7th, Moon and
/// Venus in the 4th").
pub fn dig_strong_cusp(g: DashaLord) -> Option<usize> {
    Some(match g {
        DashaLord::Sun | DashaLord::Mars => 9,
        DashaLord::Moon | DashaLord::Venus => 3,
        DashaLord::Mercury | DashaLord::Jupiter => 0,
        DashaLord::Saturn => 6,
        _ => return None,
    })
}

/// Disc diameters used for Yuddha bala (Raman, *Graha and Bhava Balas*, Kala bala;
/// PyJHora `const.planets_disc_diameters`): Mars 9.4, Mercury 6.6, Jupiter 190.4, Venus
/// 16.6, Saturn 158.0. Only their differences enter the formula, so the unit does not
/// matter.
pub fn disc_diameter(g: DashaLord) -> Option<f64> {
    Some(match g {
        DashaLord::Mars => 9.4,
        DashaLord::Mercury => 6.6,
        DashaLord::Jupiter => 190.4,
        DashaLord::Venus => 16.6,
        DashaLord::Saturn => 158.0,
        _ => return None,
    })
}

/// Mean geocentric daily motion in longitude, degrees per day, from the sidereal orbital
/// periods in the NASA/JPL planetary fact sheet (Mars 686.980 d, Jupiter 4332.589 d,
/// Saturn 10759.22 d). Mercury and Venus circle the Sun, so their mean geocentric motion
/// is the Sun's (365.25636 d). Used to grade the motional states for Chesta bala.
pub fn mean_motion(g: DashaLord) -> Option<f64> {
    Some(match g {
        DashaLord::Mars => 360.0 / 686.980,
        DashaLord::Jupiter => 360.0 / 4_332.589,
        DashaLord::Saturn => 360.0 / 10_759.22,
        DashaLord::Mercury | DashaLord::Venus => 360.0 / 365.256_36,
        _ => return None,
    })
}

/// Combustion orb from the Sun, degrees (Surya Siddhanta ix.6–9 as the commentaries on
/// BPHS and JHora use it): Moon 12, Mars 17, Mercury 14 (12 retrograde), Jupiter 11,
/// Venus 10 (8 retrograde), Saturn 15.
pub fn combustion_orb(g: DashaLord, retrograde: bool) -> Option<f64> {
    Some(match (g, retrograde) {
        (DashaLord::Moon, _) => 12.0,
        (DashaLord::Mars, _) => 17.0,
        (DashaLord::Mercury, false) => 14.0,
        (DashaLord::Mercury, true) => 12.0,
        (DashaLord::Jupiter, _) => 11.0,
        (DashaLord::Venus, false) => 10.0,
        (DashaLord::Venus, true) => 8.0,
        (DashaLord::Saturn, _) => 15.0,
        _ => return None,
    })
}

/// Lord of a weekday, 0 = Sunday.
pub fn weekday_lord(weekday: u8) -> DashaLord {
    match weekday % 7 {
        0 => DashaLord::Sun,
        1 => DashaLord::Moon,
        2 => DashaLord::Mars,
        3 => DashaLord::Mercury,
        4 => DashaLord::Jupiter,
        5 => DashaLord::Venus,
        _ => DashaLord::Saturn,
    }
}

/// Lord of a hora as [`kshana_events::hora_at`] names it.
pub fn lord_by_name(name: &str) -> Option<DashaLord> {
    Some(match name {
        "Sun" | "Ravivara" => DashaLord::Sun,
        "Moon" | "Somavara" => DashaLord::Moon,
        "Mars" | "Mangalavara" => DashaLord::Mars,
        "Mercury" | "Budhavara" => DashaLord::Mercury,
        "Jupiter" | "Guruvara" => DashaLord::Jupiter,
        "Venus" | "Shukravara" => DashaLord::Venus,
        "Saturn" | "Shanivara" => DashaLord::Saturn,
        _ => return None,
    })
}

/// Sputa drishti: the aspect value in virupas that `aspecting` throws on a point `d`
/// degrees ahead of it (BPHS ch. 26, slokas 7–12, in the piecewise form Raman tabulates
/// in *Graha and Bhava Balas* and JHora implements):
///
/// | arc `d`   | value               |
/// |-----------|---------------------|
/// | 0–30      | 0                   |
/// | 30–60     | (d − 30) / 2        |
/// | 60–90     | d − 45              |
/// | 90–120    | (120 − d) / 2 + 30  |
/// | 120–150   | 150 − d             |
/// | 150–180   | 2 (d − 150)         |
/// | 180–300   | (300 − d) / 2       |
/// | 300–360   | 0                   |
///
/// The special aspects are added on top: Mars 15 over 90–120 and 210–240 (4th and 8th),
/// Jupiter 30 over 120–150 and 240–270 (5th and 9th), Saturn 45 over 60–90 and 270–300
/// (3rd and 10th), so each is a full 60 at the start of its house. As in Raman's table
/// the sum can exceed 60 inside those bands; nothing is clamped.
pub fn sputa_drishti(aspecting: DashaLord, d: f64) -> f64 {
    let d = d.rem_euclid(360.0);
    let base = if d < 30.0 {
        0.0
    } else if d < 60.0 {
        (d - 30.0) / 2.0
    } else if d < 90.0 {
        d - 45.0
    } else if d < 120.0 {
        (120.0 - d) / 2.0 + 30.0
    } else if d < 150.0 {
        150.0 - d
    } else if d < 180.0 {
        2.0 * (d - 150.0)
    } else if d < 300.0 {
        (300.0 - d) / 2.0
    } else {
        0.0
    };
    let special = match aspecting {
        DashaLord::Mars if (90.0..120.0).contains(&d) || (210.0..240.0).contains(&d) => 15.0,
        DashaLord::Jupiter if (120.0..150.0).contains(&d) || (240.0..270.0).contains(&d) => 30.0,
        DashaLord::Saturn if (60.0..90.0).contains(&d) || (270.0..300.0).contains(&d) => 45.0,
        _ => 0.0,
    };
    base + special
}

/// Class of a sign for Bhava Dig bala (Raman, *Graha and Bhava Balas*, Bhava bala;
/// PyJHora `const.nara_rasi_longitudes` and companions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignClass {
    /// Human: Gemini, Virgo, Libra, Aquarius, the first half of Sagittarius. Strong in
    /// the lagna.
    Nara,
    /// Watery: Cancer, Pisces, the second half of Capricorn. Strong in the 4th.
    Jalachara,
    /// Insect: Scorpio. Strong in the 7th.
    Keeta,
    /// Quadruped: Aries, Taurus, Leo, the second half of Sagittarius, the first half of
    /// Capricorn. Strong in the 10th.
    Chatushpada,
}

impl SignClass {
    /// House index (0 = lagna) where a cusp of this class has full Dig bala.
    pub fn strong_house(self) -> usize {
        match self {
            SignClass::Nara => 0,
            SignClass::Jalachara => 3,
            SignClass::Keeta => 6,
            SignClass::Chatushpada => 9,
        }
    }

    /// Word for the class.
    pub fn name(self) -> &'static str {
        match self {
            SignClass::Nara => "human",
            SignClass::Jalachara => "watery",
            SignClass::Keeta => "insect",
            SignClass::Chatushpada => "quadruped",
        }
    }
}

/// Class of the sign at a sidereal longitude.
pub fn sign_class(sidereal: f64) -> SignClass {
    let l = sidereal.rem_euclid(360.0);
    let sign = (l / 30.0).floor() as u8;
    let first_half = l.rem_euclid(30.0) < 15.0;
    match sign {
        0 | 1 | 4 => SignClass::Chatushpada,
        2 | 5 | 6 | 10 => SignClass::Nara,
        3 | 11 => SignClass::Jalachara,
        7 => SignClass::Keeta,
        8 if first_half => SignClass::Nara,
        8 => SignClass::Chatushpada,
        9 if first_half => SignClass::Chatushpada,
        _ => SignClass::Jalachara,
    }
}

/// Bhava Dig bala (Raman; BPHS Bhava bala): 60 when the cusp's sign class sits in its
/// strong house, ten fewer for every house away, nothing opposite. `house_index` is
/// 0 = lagna.
pub fn bhava_dig_bala(cusp_sidereal: f64, house_index: usize) -> f64 {
    let strong = sign_class(cusp_sidereal).strong_house();
    let away = (house_index as i32 - strong as i32).rem_euclid(12);
    let away = away.min(12 - away);
    60.0 - 10.0 * away as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naisargika_matches_the_seventh_series() {
        assert_eq!(naisargika(DashaLord::Sun), 60.0);
        assert!((naisargika(DashaLord::Moon) - 51.43).abs() < 0.005);
        assert!((naisargika(DashaLord::Venus) - 42.857).abs() < 0.005);
        assert!((naisargika(DashaLord::Jupiter) - 34.286).abs() < 0.005);
        assert!((naisargika(DashaLord::Mercury) - 25.714).abs() < 0.005);
        assert!((naisargika(DashaLord::Mars) - 17.143).abs() < 0.005);
        assert!((naisargika(DashaLord::Saturn) - 8.571).abs() < 0.005);
        assert_eq!(naisargika(DashaLord::Rahu), 0.0);
    }

    #[test]
    fn sputa_drishti_bands_and_special_aspects() {
        assert_eq!(sputa_drishti(DashaLord::Sun, 20.0), 0.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 45.0), 7.5);
        assert_eq!(sputa_drishti(DashaLord::Sun, 60.0), 15.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 90.0), 45.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 120.0), 30.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 150.0), 0.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 180.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 240.0), 30.0);
        assert_eq!(sputa_drishti(DashaLord::Sun, 310.0), 0.0);
        // full special aspects at the start of their houses
        assert_eq!(sputa_drishti(DashaLord::Saturn, 60.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Saturn, 270.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Mars, 90.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Mars, 210.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Jupiter, 120.0), 60.0);
        assert_eq!(sputa_drishti(DashaLord::Jupiter, 240.0), 60.0);
        // and nothing special elsewhere
        assert_eq!(sputa_drishti(DashaLord::Jupiter, 90.0), 45.0);
    }

    #[test]
    fn sign_classes_follow_raman() {
        assert_eq!(sign_class(70.0), SignClass::Nara); // Gemini
        assert_eq!(sign_class(310.0), SignClass::Nara); // Aquarius
        assert_eq!(sign_class(245.0), SignClass::Nara); // first half of Sagittarius
        assert_eq!(sign_class(265.0), SignClass::Chatushpada); // second half
        assert_eq!(sign_class(275.0), SignClass::Chatushpada); // first half of Capricorn
        assert_eq!(sign_class(295.0), SignClass::Jalachara); // second half
        assert_eq!(sign_class(225.0), SignClass::Keeta);
        assert_eq!(sign_class(100.0), SignClass::Jalachara);
        // a quadruped cusp: 60 in the 10th, 30 in the lagna, 0 in the 4th
        assert_eq!(bhava_dig_bala(10.0, 9), 60.0);
        assert_eq!(bhava_dig_bala(10.0, 0), 30.0);
        assert_eq!(bhava_dig_bala(10.0, 3), 0.0);
        assert_eq!(bhava_dig_bala(10.0, 10), 50.0);
        assert_eq!(bhava_dig_bala(10.0, 8), 50.0);
    }

    #[test]
    fn drekkana_and_kendradi() {
        assert_eq!(drekkana_bala(DashaLord::Sun, 5.0), 15.0);
        assert_eq!(drekkana_bala(DashaLord::Sun, 15.0), 0.0);
        assert_eq!(drekkana_bala(DashaLord::Saturn, 15.0), 15.0);
        assert_eq!(drekkana_bala(DashaLord::Venus, 25.0), 15.0);
        assert_eq!(drekkana_bala(DashaLord::Venus, 5.0), 0.0);
        assert_eq!(kendradi(10), 60.0);
        assert_eq!(kendradi(11), 30.0);
        assert_eq!(kendradi(12), 15.0);
        assert_eq!(deep_debilitation(DashaLord::Sun), Some(190.0));
        assert_eq!(deep_debilitation(DashaLord::Mars), Some(118.0));
    }
}
