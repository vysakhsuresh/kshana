//! Sthana bala: positional strength in five parts (BPHS ch. 27, slokas 2–7; Raman,
//! *Graha and Bhava Balas*, ch. on Sthana bala).

use crate::relation::compound_relation;
use crate::tables::{deep_debilitation, drekkana_bala, kendradi};
use kshana_rules::parashari::{dignity, Dignity};
use kshana_vedic::{sign_lord, varga, DashaLord};

/// The five parts of Sthana bala, virupas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SthanaParts {
    /// Uchcha bala: arc from deepest debilitation over three, at most 60.
    pub uchcha: f64,
    /// Saptavargaja bala: dignity in D1, D2, D3, D7, D9, D12 and D30 added up.
    pub saptavargaja: f64,
    /// Ojhayugma bala: odd or even sign and navamsa, 15 each.
    pub ojhayugma: f64,
    /// Kendradi bala: kendra 60, panapara 30, apoklima 15.
    pub kendradi: f64,
    /// Drekkana bala: 15 in the decanate of the graha's gender.
    pub drekkana: f64,
    /// The five added.
    pub total: f64,
}

/// The seven vargas of Saptavargaja bala: rasi, hora, drekkana, saptamsa, navamsa,
/// dwadasamsa, trimsamsa (BPHS 27.2; PyJHora `const.sapthavargaja_factors`).
pub const SAPTAVARGA: [u32; 7] = [1, 2, 3, 7, 9, 12, 30];

/// Uchcha bala (BPHS 27.? "deduct the debilitation point from the graha's longitude; if
/// over six signs deduct from twelve; divide by three"): 60 at deepest exaltation, 0 at
/// deepest debilitation. Nodes get 0.
pub fn uchcha_bala(g: DashaLord, sidereal: f64) -> f64 {
    let Some(d) = deep_debilitation(g) else {
        return 0.0;
    };
    let arc = (sidereal - d).rem_euclid(360.0);
    let arc = if arc > 180.0 { 360.0 - arc } else { arc };
    arc / 3.0
}

/// Saptavargaja bala. In each of the seven vargas the graha scores 30 in its own sign,
/// or the compound-relationship value for the sign's lord (22.5, 15, 7.5, 3.75, 1.875);
/// in the rasi alone a moolatrikona placement scores 45 (BPHS 27.2–4; Raman; JHora).
/// `rasi_signs` are the seven grahas' rasi signs, which fix the temporary friendships.
pub fn saptavargaja_bala(g: DashaLord, sidereal: f64, rasi_signs: &[(DashaLord, u8)]) -> f64 {
    let sign_of = |x: DashaLord| rasi_signs.iter().find(|(p, _)| *p == x).map(|(_, s)| *s);
    let Some(my_sign) = sign_of(g) else {
        return 0.0;
    };
    let mut total = 0.0;
    for n in SAPTAVARGA {
        let lon = varga(sidereal, n);
        let sign = ((lon / 30.0).floor() as usize % 12) as u8;
        let lord = sign_lord(sign as usize);
        total += if n == 1 && dignity(g, sidereal) == Dignity::Moolatrikona {
            45.0
        } else if lord == g {
            30.0
        } else {
            let lord_sign = sign_of(lord).unwrap_or(sign);
            compound_relation(g, lord, my_sign, lord_sign).saptavargaja_virupas()
        };
    }
    total
}

/// Ojhayugma bala (BPHS 27.5): the Moon and Venus gain 15 in an even sign and 15 in an
/// even navamsa; the other grahas in odd ones.
pub fn ojhayugma_bala(g: DashaLord, sidereal: f64) -> f64 {
    let wants_even = matches!(g, DashaLord::Moon | DashaLord::Venus);
    let rasi_odd = ((sidereal.rem_euclid(360.0) / 30.0).floor() as u32) % 2 == 0;
    let nav_odd = ((varga(sidereal, 9) / 30.0).floor() as u32) % 2 == 0;
    let mut v = 0.0;
    if rasi_odd != wants_even {
        v += 15.0;
    }
    if nav_odd != wants_even {
        v += 15.0;
    }
    v
}

/// All five parts for one graha. `house` is the graha's house by the chart's cusps.
pub fn sthana(
    g: DashaLord,
    sidereal: f64,
    house: u8,
    rasi_signs: &[(DashaLord, u8)],
) -> SthanaParts {
    let uchcha = uchcha_bala(g, sidereal);
    let saptavargaja = saptavargaja_bala(g, sidereal, rasi_signs);
    let ojhayugma = ojhayugma_bala(g, sidereal);
    let kendradi = kendradi(house);
    let drekkana = drekkana_bala(g, sidereal.rem_euclid(30.0));
    SthanaParts {
        uchcha,
        saptavargaja,
        ojhayugma,
        kendradi,
        drekkana,
        total: uchcha + saptavargaja + ojhayugma + kendradi + drekkana,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_vedic::DashaLord::*;

    #[test]
    fn uchcha_is_sixty_at_exaltation_and_zero_at_debilitation() {
        assert!((uchcha_bala(Sun, 10.0) - 60.0).abs() < 1e-12);
        assert!(uchcha_bala(Sun, 190.0).abs() < 1e-12);
        assert!((uchcha_bala(Sun, 100.0) - 30.0).abs() < 1e-12);
        assert!((uchcha_bala(Mars, 298.0) - 60.0).abs() < 1e-12);
        assert_eq!(uchcha_bala(Rahu, 0.0), 0.0);
    }

    #[test]
    fn ojhayugma_counts_rasi_and_navamsa() {
        // Sun at 1° Aries: odd rasi, first navamsa Aries (odd) → 30
        assert_eq!(ojhayugma_bala(Sun, 1.0), 30.0);
        // Moon at 1° Aries → 0
        assert_eq!(ojhayugma_bala(Moon, 1.0), 0.0);
        // Moon at 1° Taurus: even rasi, navamsa from Capricorn (even) → 30
        assert_eq!(ojhayugma_bala(Moon, 31.0), 30.0);
    }

    #[test]
    fn saptavargaja_in_own_signs_everywhere() {
        // Sun at 4° Leo: rasi moolatrikona 45; hora (odd sign, first half → Leo) 30;
        // drekkana (first) Leo 30; saptamsa (odd sign, first part) Leo 30; navamsa (fixed
        // sign counts from Aries; 4° is the second part) Taurus → Venus; dwadasamsa (second
        // part) Virgo → Mercury; trimsamsa (odd: 0–5 → Mars) Aries → Mars.
        // Mars, Mercury and Venus all in Virgo, the 2nd from Leo: temporary friends, so
        // Mars (natural friend) is a great friend 22.5, Mercury (neutral) a friend 15 and
        // Venus (natural enemy) a neutral 7.5.
        let signs = [
            (Sun, 4u8),
            (Moon, 0),
            (Mars, 5),
            (Mercury, 5),
            (Jupiter, 0),
            (Venus, 5),
            (Saturn, 0),
        ];
        let v = saptavargaja_bala(Sun, 124.0, &signs);
        assert!(
            (v - (45.0 + 30.0 * 3.0 + 7.5 + 15.0 + 22.5)).abs() < 1e-9,
            "{v}"
        );
    }

    #[test]
    fn sthana_total_adds_up() {
        let signs = [
            (Sun, 0u8),
            (Moon, 1),
            (Mars, 9),
            (Mercury, 5),
            (Jupiter, 3),
            (Venus, 11),
            (Saturn, 6),
        ];
        let s = sthana(Sun, 10.0, 1, &signs);
        assert!((s.uchcha - 60.0).abs() < 1e-9);
        assert_eq!(s.kendradi, 60.0);
        assert_eq!(s.drekkana, 0.0); // 10° is the second decanate
        assert!(
            (s.total - (s.uchcha + s.saptavargaja + s.ojhayugma + s.kendradi + s.drekkana)).abs()
                < 1e-9
        );
    }
}
