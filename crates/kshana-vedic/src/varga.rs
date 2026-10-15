//! Divisional charts (vargas) by the Brihat Parashara Hora Shastra rules.
//!
//! Each function maps a sidereal longitude to the longitude in the divisional chart:
//! the varga sign index times 30 plus the position inside that sign, so the same
//! drawing code renders every varga. Sign indices are 0 = Aries.

fn sign_and_part(sidereal: f64, n: u32) -> (u32, u32, f64) {
    let l = sidereal.rem_euclid(360.0);
    let sign = (l / 30.0).floor() as u32 % 12;
    let in_sign = l.rem_euclid(30.0);
    let part_size = 30.0 / n as f64;
    let part = ((in_sign / part_size).floor() as u32).min(n - 1);
    let frac = (in_sign - part as f64 * part_size) / part_size;
    (sign, part, frac)
}

fn out(varga_sign: u32, frac: f64) -> f64 {
    (varga_sign % 12) as f64 * 30.0 + frac * 30.0
}

/// Longitude in the D-n chart. Supported n: 1, 2, 3, 4, 7, 9, 10, 12, 16, 20, 24, 27,
/// 30, 60. Other n use the uniform "count from the same sign" rule.
pub fn varga(sidereal: f64, n: u32) -> f64 {
    if n <= 1 {
        return sidereal.rem_euclid(360.0);
    }
    let (sign, part, frac) = sign_and_part(sidereal, n);
    let odd = sign % 2 == 0; // Aries (0) is odd in the 1-based count
    let vs = match n {
        // Hora: odd signs first half Sun (Leo), second half Moon (Cancer); even reversed
        2 => {
            let sun = 4;
            let moon = 3;
            if odd {
                if part == 0 {
                    sun
                } else {
                    moon
                }
            } else if part == 0 {
                moon
            } else {
                sun
            }
        }
        // Drekkana: 1st, 5th, 9th from the sign
        3 => sign + 4 * part,
        // Chaturthamsha: 1st, 4th, 7th, 10th
        4 => sign + 3 * part,
        // Saptamsha: odd from the sign, even from the 7th
        7 => (if odd { sign } else { sign + 6 }) + part,
        // Navamsa: movable from the sign, fixed from the 9th, dual from the 5th
        9 => {
            let start = match sign % 3 {
                0 => sign,
                1 => sign + 8,
                _ => sign + 4,
            };
            start + part
        }
        // Dashamsha: odd from the sign, even from the 9th
        10 => (if odd { sign } else { sign + 8 }) + part,
        // Dwadashamsha: from the sign
        12 => sign + part,
        // Shodashamsha: movable from Aries, fixed from Leo, dual from Sagittarius
        16 => {
            let start = match sign % 3 {
                0 => 0,
                1 => 4,
                _ => 8,
            };
            start + part
        }
        // Vimshamsha: movable from Aries, fixed from Sagittarius, dual from Leo
        20 => {
            let start = match sign % 3 {
                0 => 0,
                1 => 8,
                _ => 4,
            };
            start + part
        }
        // Chaturvimshamsha: odd from Leo, even from Cancer
        24 => (if odd { 4 } else { 3 }) + part,
        // Saptavimshamsha (Bhamsha): fiery from Aries, earthy from Cancer, airy from Libra, watery from Capricorn
        27 => {
            let start = match sign % 4 {
                0 => 0,
                1 => 3,
                2 => 6,
                _ => 9,
            };
            start + part
        }
        // Trimshamsha: unequal parts. Odd: Mars 5 (Aries), Saturn 5 (Aquarius), Jupiter 8
        // (Sagittarius), Mercury 7 (Gemini), Venus 5 (Libra). Even: Venus 5 (Taurus),
        // Mercury 7 (Virgo), Jupiter 8 (Pisces), Saturn 5 (Capricorn), Mars 5 (Scorpio).
        30 => {
            let in_sign = sidereal.rem_euclid(30.0);
            let table: [(f64, u32); 5] = if odd {
                [(5.0, 0), (10.0, 10), (18.0, 8), (25.0, 2), (30.0, 6)]
            } else {
                [(5.0, 1), (12.0, 5), (20.0, 11), (25.0, 9), (30.0, 7)]
            };
            let mut lo = 0.0;
            let mut chosen = (table[4].1, 0.0);
            for (hi, s) in table {
                if in_sign < hi {
                    chosen = (s, (in_sign - lo) / (hi - lo));
                    break;
                }
                lo = hi;
            }
            return out(chosen.0, chosen.1);
        }
        // Shashtiamsha: from the sign
        60 => sign + part,
        _ => sign + part,
    };
    out(vs, frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(l: f64) -> u32 {
        (l.rem_euclid(360.0) / 30.0).floor() as u32
    }

    #[test]
    fn navamsa_rules() {
        assert_eq!(sign(varga(1.0, 9)), 0); // Aries first navamsa → Aries
        assert_eq!(sign(varga(29.9, 9)), 8); // Aries ninth navamsa → Sagittarius
        assert_eq!(sign(varga(45.0, 9)), 1); // 15° Taurus (fixed): 5th navamsa from Capricorn → Taurus
        assert_eq!(sign(varga(61.0, 9)), 6); // 1° Gemini (dual): from Libra
        for i in 0..108 {
            let l = i as f64 * 360.0 / 108.0 + 0.5;
            assert!((0.0..360.0).contains(&varga(l, 9)));
        }
    }

    #[test]
    fn hora_and_drekkana() {
        assert_eq!(sign(varga(10.0, 2)), 4); // odd sign, first half → Leo
        assert_eq!(sign(varga(20.0, 2)), 3); // odd sign, second half → Cancer
        assert_eq!(sign(varga(40.0, 2)), 3); // even sign, first half → Cancer
        assert_eq!(sign(varga(5.0, 3)), 0);
        assert_eq!(sign(varga(15.0, 3)), 4);
        assert_eq!(sign(varga(25.0, 3)), 8);
    }

    #[test]
    fn trimshamsha_parts() {
        assert_eq!(sign(varga(2.0, 30)), 0); // Aries 0–5 → Mars → Aries
        assert_eq!(sign(varga(7.0, 30)), 10); // 5–10 → Saturn → Aquarius
        assert_eq!(sign(varga(15.0, 30)), 8); // 10–18 → Jupiter → Sagittarius
        assert_eq!(sign(varga(32.0, 30)), 1); // Taurus 0–5 → Venus → Taurus
        assert_eq!(sign(varga(59.0, 30)), 7); // Taurus 25–30 → Mars → Scorpio
    }

    #[test]
    fn dashamsha_and_dwadashamsha() {
        assert_eq!(sign(varga(2.0, 10)), 0);
        assert_eq!(sign(varga(33.0, 10)), 10); // Taurus (even): 2nd dashamsha counted from Capricorn → Aquarius
        assert_eq!(sign(varga(27.5, 12)), 11); // Aries 12th dwadashamsha → Pisces
        assert_eq!(sign(varga(0.4, 60)), 0);
        assert_eq!(sign(varga(29.9, 60)), 11);
    }
}
