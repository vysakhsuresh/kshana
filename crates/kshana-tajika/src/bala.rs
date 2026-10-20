//! Panchavargeeya bala: the five-fold Tajika strength that picks the lord of the year.
//!
//! Source: K. S. Charak, *A Textbook of Varshaphala*, ch. VI (Tables VI-4 to VI-10),
//! which follows the *Tajika Neelakanthi*. The five parts and their maxima are
//! Kshetra (Griha) 30, Uchcha 20, Hadda 15, Drekkana 10, Navamsa 5; the total (at most
//! 80) divided by four is the Vishwa bala out of 20.
//!
//! Four of the parts depend on the relation between the planet and the lord of the
//! division it occupies (sign, Hadda, drekkana, navamsa): the lord itself gets the full
//! value, a friend three quarters, a neutral one half, an enemy one quarter. The
//! relation is the Tajika positional one, read from the annual chart: counting from the
//! planet's sign, the lord in the 3rd, 5th, 9th or 11th is a friend, in the 1st, 4th,
//! 7th or 10th an enemy, elsewhere neutral. Natural (Parashari) friendship plays no
//! part. The Uchcha part is the arc from the point of deepest debilitation, folded to at
//! most 180°, divided by nine.

use crate::sky::AnnualSky;
use crate::tables::{deep_debilitation, drekkana_lord, hadda_lord, navamsa_lord, sign_index};
use kshana_vedic::{sign_lord, DashaLord};

/// Tajika relation between a planet and the lord of a division it occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// The planet is the lord itself.
    Own,
    /// The lord stands in the 3rd, 5th, 9th or 11th sign from the planet.
    Friend,
    /// The lord stands in the 2nd, 6th, 8th or 12th sign from the planet.
    Neutral,
    /// The lord stands in the same sign or the 4th, 7th or 10th from the planet.
    Enemy,
}

impl Relation {
    /// Share of a division's full value: 1, 3/4, 1/2, 1/4.
    pub fn fraction(self) -> f64 {
        match self {
            Relation::Own => 1.0,
            Relation::Friend => 0.75,
            Relation::Neutral => 0.5,
            Relation::Enemy => 0.25,
        }
    }

    /// Word for the relation.
    pub fn name(self) -> &'static str {
        match self {
            Relation::Own => "own",
            Relation::Friend => "friend's",
            Relation::Neutral => "neutral's",
            Relation::Enemy => "enemy's",
        }
    }
}

/// Tajika positional relation of `planet` to `lord` in the annual chart.
pub fn relation(sky: &AnnualSky, planet: DashaLord, lord: DashaLord) -> Relation {
    if planet == lord {
        return Relation::Own;
    }
    let (Some(p), Some(l)) = (sky.sign(planet), sky.sign(lord)) else {
        return Relation::Neutral;
    };
    match (l + 12 - p) % 12 + 1 {
        3 | 5 | 9 | 11 => Relation::Friend,
        1 | 4 | 7 | 10 => Relation::Enemy,
        _ => Relation::Neutral,
    }
}

/// The five parts and their sum for one planet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panchavargeeya {
    /// The planet.
    pub planet: DashaLord,
    /// Kshetra (Griha) bala, at most 30.
    pub kshetra: f64,
    /// Uchcha bala, at most 20.
    pub uchcha: f64,
    /// Hadda bala, at most 15.
    pub hadda: f64,
    /// Drekkana bala, at most 10.
    pub drekkana: f64,
    /// Navamsa bala, at most 5.
    pub navamsa: f64,
    /// Sum of the five, at most 80.
    pub total: f64,
    /// Vishwa bala: the total over four, at most 20.
    pub vishwa: f64,
}

/// Compute the five-fold strength of one of the seven grahas; `None` for a node.
pub fn panchavargeeya(sky: &AnnualSky, planet: DashaLord) -> Option<Panchavargeeya> {
    let lon = sky.longitude(planet)?;
    let debilitation = deep_debilitation(planet)?;
    let kshetra = 30.0 * relation(sky, planet, sign_lord(sign_index(lon))).fraction();
    let arc = (lon - debilitation).rem_euclid(360.0);
    let arc = if arc > 180.0 { 360.0 - arc } else { arc };
    let uchcha = arc / 9.0;
    let hadda = 15.0 * relation(sky, planet, hadda_lord(lon)).fraction();
    let drekkana = 10.0 * relation(sky, planet, drekkana_lord(lon)).fraction();
    let navamsa = 5.0 * relation(sky, planet, navamsa_lord(lon)).fraction();
    let total = kshetra + uchcha + hadda + drekkana + navamsa;
    Some(Panchavargeeya {
        planet,
        kshetra,
        uchcha,
        hadda,
        drekkana,
        navamsa,
        total,
        vishwa: total / 4.0,
    })
}

/// Grade of a Vishwa bala (Shanker Adawal, *Encyclopedia of Vedic Astrology*, ch. III):
/// 15 and above Parakrami, 10–15 Poorna bali, 5–10 Madhya bali, below 5 Nirbali.
pub fn grade(vishwa: f64) -> &'static str {
    if vishwa >= 15.0 {
        "Parakrami (extraordinarily strong)"
    } else if vishwa >= 10.0 {
        "Poorna bali (full strength)"
    } else if vishwa >= 5.0 {
        "Madhya bali (middling)"
    } else {
        "Nirbali (weak)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::SEVEN;

    fn sky(longitudes: [f64; 7]) -> AnnualSky {
        AnnualSky {
            lagna: 0.0,
            cusps: std::array::from_fn(|i| 30.0 * i as f64),
            longitudes,
            speeds: [0.0; 7],
        }
    }

    #[test]
    fn relation_follows_the_tajika_houses() {
        // Moon 0 Aries, Mercury 60 Gemini (3rd from Aries), Venus 90 Cancer (4th),
        // Sun 30 Taurus (2nd), Mars 120 Leo (5th), Jupiter 180 Libra (7th), Saturn 300 Aquarius (11th)
        let s = sky([0.0, 60.0, 90.0, 30.0, 120.0, 180.0, 300.0]);
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Mercury),
            Relation::Friend
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Venus),
            Relation::Enemy
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Sun),
            Relation::Neutral
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Mars),
            Relation::Friend
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Jupiter),
            Relation::Enemy
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Saturn),
            Relation::Friend
        );
        assert_eq!(
            relation(&s, DashaLord::Moon, DashaLord::Moon),
            Relation::Own
        );
    }

    #[test]
    fn maxima_and_uchcha_fold() {
        // Mars at 28° Capricorn (298°): deep exaltation, 180° from 118° → uchcha 20.
        // Capricorn's lord Saturn at Aquarius 300°: 2nd from Capricorn → neutral → kshetra 15.
        let s = sky([0.0, 0.0, 0.0, 0.0, 298.0, 0.0, 300.0]);
        let b = panchavargeeya(&s, DashaLord::Mars).unwrap();
        assert!((b.uchcha - 20.0).abs() < 1e-9);
        assert_eq!(b.kshetra, 15.0);
        assert!((b.total - (b.kshetra + b.uchcha + b.hadda + b.drekkana + b.navamsa)).abs() < 1e-9);
        assert!((b.vishwa - b.total / 4.0).abs() < 1e-12);
        // Mars at its deep debilitation: uchcha 0
        let s = sky([0.0, 0.0, 0.0, 0.0, 118.0, 0.0, 0.0]);
        assert_eq!(panchavargeeya(&s, DashaLord::Mars).unwrap().uchcha, 0.0);
        for g in SEVEN {
            let b = panchavargeeya(&s, g).unwrap();
            assert!(b.vishwa <= 20.0 && b.vishwa >= 0.0);
        }
        assert!(panchavargeeya(&s, DashaLord::Rahu).is_none());
    }
}
