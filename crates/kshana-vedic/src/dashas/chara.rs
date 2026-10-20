//! Chara dasha (Jaimini), K. N. Rao's variant.
//!
//! The rules implemented are those of K. N. Rao, *Predicting through Jaimini's Chara
//! Dasha* (Vani, 1995), as stated in the chapter excerpt on astrojyoti.com ("note the ninth
//! house from lagna … six dashas are direct, six indirect"), the Saptarishis article by
//! U. K. Jha on Rao's procedure, and as implemented for JHora's "K.N. Rao" option in
//! PyJHora `src/jhora/horoscope/dhasa/raasi/chara.py` (`_dhasa_progression_knrao_method`,
//! `_dhasa_duration_knrao_method`, `_antardhasa_order_knrao`), PyJHora issue #59 and
//! `ramanflorfresca/antar-fastapi` PR #512 (tested against Rao's published Rajiv Gandhi
//! periods):
//!
//! * Odd-footed (vishama-pada) signs: Aries, Taurus, Gemini, Libra, Scorpio, Sagittarius;
//!   even-footed: Cancer, Leo, Virgo, Capricorn, Aquarius, Pisces.
//! * Direction of the dasha sequence (Rao's ninth-house rule): from the lagna, direct when
//!   the ninth sign from the lagna is odd-footed, reverse when it is even-footed — i.e.
//!   direct for lagnas Aries, Leo, Virgo, Libra, Aquarius, Pisces, reverse for the rest.
//!   One cycle of twelve signs is produced.
//! * Years of a sign: count from the sign to the sign its lord occupies — direct for an
//!   odd-footed sign, reverse for an even-footed one — minus one. Lord in the sign itself:
//!   12 years flat (no exaltation bonus, which only matters for Mercury in Virgo).
//!   Otherwise an exalted lord adds one year and a debilitated lord subtracts one
//!   ([`EXALTATION_SIGN`]; debilitation is the seventh from it; Rahu Taurus/Scorpio,
//!   Ketu Scorpio/Taurus). A result of zero (only Sagittarius with Jupiter debilitated in
//!   Capricorn) is floored to one year.
//! * Scorpio (Mars, Ketu) and Aquarius (Saturn, Rahu): if both co-lords occupy the sign,
//!   12 years; if exactly one does, count from the other; if neither, the stronger is
//!   used — the one conjoined with more of the other grahas, then the one in exaltation,
//!   then the traditional lord (Mars, Saturn). Rao states the first two cases; the
//!   strength tie-break is the Jaimini conjunction rule, stated here as the convention.
//! * Antardashas: twelve, each one twelfth of the period, starting from the sign *after*
//!   the dasha sign in the dasha sign's own direction (ninth-house rule applied to that
//!   sign) and ending with the dasha sign itself.
//!
//! Inputs are sidereal longitudes in degrees: the lagna and the nine grahas in the order
//! Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu.

use kshana_time::Instant;

use super::{sign_index, sign_of, subdivide_weighted, Period, SIGN_NAMES};

/// Direction of a sign sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Zodiacal order.
    Direct,
    /// Anti-zodiacal order.
    Reverse,
}

/// Exaltation sign (0 = Aries) of each graha in the input order Sun, Moon, Mars, Mercury,
/// Jupiter, Venus, Saturn, Rahu, Ketu. Debilitation is the seventh sign from it.
pub const EXALTATION_SIGN: [usize; 9] = [0, 1, 9, 5, 3, 11, 6, 1, 7];

/// Index in the graha array of the lord(s) of each sign: the traditional lord first.
const LORDS: [(usize, Option<usize>); 12] = [
    (2, None),    // Aries: Mars
    (5, None),    // Taurus: Venus
    (3, None),    // Gemini: Mercury
    (1, None),    // Cancer: Moon
    (0, None),    // Leo: Sun
    (3, None),    // Virgo: Mercury
    (5, None),    // Libra: Venus
    (2, Some(8)), // Scorpio: Mars, Ketu
    (4, None),    // Sagittarius: Jupiter
    (6, None),    // Capricorn: Saturn
    (6, Some(7)), // Aquarius: Saturn, Rahu
    (4, None),    // Pisces: Jupiter
];

/// True for the odd-footed signs Aries, Taurus, Gemini, Libra, Scorpio, Sagittarius.
pub const fn is_odd_footed(sign: usize) -> bool {
    matches!(sign % 12, 0 | 1 | 2 | 6 | 7 | 8)
}

/// Rao's ninth-house rule: the sequence from `sign` runs direct when the ninth sign from
/// it is odd-footed, reverse otherwise (direct for Aries, Leo, Virgo, Libra, Aquarius,
/// Pisces).
pub const fn chara_direction(sign: usize) -> Direction {
    if is_odd_footed((sign + 8) % 12) {
        Direction::Direct
    } else {
        Direction::Reverse
    }
}

/// Twelve signs from `from` in `direction`.
fn sequence_from(from: usize, direction: Direction) -> [usize; 12] {
    std::array::from_fn(|k| match direction {
        Direction::Direct => (from + k) % 12,
        Direction::Reverse => (from + 12 - k) % 12,
    })
}

/// The twelve mahadasha signs from the lagna in Rao's direction.
pub fn chara_sequence(lagna_sign: usize) -> [usize; 12] {
    sequence_from(lagna_sign % 12, chara_direction(lagna_sign))
}

/// Inclusive count from `from` to `to` in `direction` (1 when equal).
fn count(from: usize, to: usize, direction: Direction) -> usize {
    match direction {
        Direction::Direct => (to + 12 - from) % 12 + 1,
        Direction::Reverse => (from + 12 - to) % 12 + 1,
    }
}

/// The graha (index into the input array) whose placement gives the years of `sign`, or
/// `None` when both co-lords of Scorpio or Aquarius occupy the sign (12 years).
pub fn chara_lord(sign: usize, grahas: &[f64; 9]) -> Option<usize> {
    let sign = sign % 12;
    let pos: [usize; 9] = std::array::from_fn(|i| sign_of(grahas[i]));
    let (a, b) = LORDS[sign];
    let Some(b) = b else {
        return Some(a);
    };
    match (pos[a] == sign, pos[b] == sign) {
        (true, true) => None,
        (true, false) => Some(b),
        (false, true) => Some(a),
        (false, false) => {
            let conj = |g: usize| {
                (0..9)
                    .filter(|i| *i != a && *i != b && pos[*i] == pos[g])
                    .count()
            };
            let (ca, cb) = (conj(a), conj(b));
            if ca != cb {
                return Some(if ca > cb { a } else { b });
            }
            let (ea, eb) = (pos[a] == EXALTATION_SIGN[a], pos[b] == EXALTATION_SIGN[b]);
            if ea != eb {
                return Some(if ea { a } else { b });
            }
            Some(a)
        }
    }
}

/// Years of the mahadasha of `sign` by Rao's rules (1 … 12).
pub fn chara_years(sign: usize, grahas: &[f64; 9]) -> u32 {
    let sign = sign % 12;
    let Some(lord) = chara_lord(sign, grahas) else {
        return 12;
    };
    let lord_sign = sign_of(grahas[lord]);
    if lord_sign == sign {
        return 12;
    }
    let direction = if is_odd_footed(sign) {
        Direction::Direct
    } else {
        Direction::Reverse
    };
    let mut years = count(sign, lord_sign, direction) as i32 - 1;
    if lord_sign == EXALTATION_SIGN[lord] {
        years += 1;
    } else if lord_sign == (EXALTATION_SIGN[lord] + 6) % 12 {
        years -= 1;
    }
    years.clamp(1, 12) as u32
}

/// One cycle of twelve mahadashas from the lagna. Boundaries are exact: birth plus
/// cumulative years × `year_ns`.
pub fn chara(lagna_sidereal: f64, grahas: &[f64; 9], birth: Instant, year_ns: i128) -> Vec<Period> {
    let lagna = sign_of(lagna_sidereal);
    let mut out = Vec::with_capacity(12);
    let mut t = birth;
    let mut cum: i128 = 0;
    for sign in chara_sequence(lagna) {
        cum += chara_years(sign, grahas) as i128;
        let end = birth.add_ns(cum * year_ns);
        out.push(Period {
            label: SIGN_NAMES[sign].to_string(),
            start: t,
            end,
            level: 1,
        });
        t = end;
    }
    out
}

/// Twelve equal antardashas of a Chara period, from the sign after the period's sign in
/// `direction`, ending with the period's own sign. Use [`chara_direction`] of that sign
/// for Rao's default. Empty when the label is not a sign.
pub fn chara_sub(period: &Period, direction: Direction) -> Vec<Period> {
    let Some(sign) = sign_index(&period.label) else {
        return Vec::new();
    };
    let first = match direction {
        Direction::Direct => (sign + 1) % 12,
        Direction::Reverse => (sign + 11) % 12,
    };
    let seq = sequence_from(first, direction);
    let parts: Vec<(&str, u32)> = seq.iter().map(|s| (SIGN_NAMES[*s], 1)).collect();
    subdivide_weighted(period, &parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    // Sun Leo, Moon Cancer, Mars Aries, Mercury Gemini, Jupiter Sagittarius, Venus Taurus,
    // Saturn Capricorn, Rahu Aquarius, Ketu Leo.
    const OWN: [f64; 9] = [124.0, 100.0, 10.0, 70.0, 250.0, 40.0, 280.0, 310.0, 130.0];

    #[test]
    fn direction_follows_the_ninth_house_rule() {
        let direct: Vec<usize> = (0..12)
            .filter(|s| chara_direction(*s) == Direction::Direct)
            .collect();
        assert_eq!(direct, vec![0, 4, 5, 6, 10, 11]);
        assert_eq!(chara_sequence(0)[1], 1);
        assert_eq!(chara_sequence(1)[1], 0);
        assert_eq!(chara_sequence(1)[11], 2);
    }

    #[test]
    fn years_by_rao_rules() {
        // Own-sign lords → 12.
        for s in [0, 1, 2, 3, 4, 8, 9] {
            assert_eq!(chara_years(s, &OWN), 12, "sign {s}");
        }
        // Virgo (even-footed), Mercury in Gemini: Vi→Le→Cn→Ge = 4 − 1 = 3.
        assert_eq!(chara_years(5, &OWN), 3);
        // Libra (odd-footed), Venus in Taurus: 8 − 1 = 7.
        assert_eq!(chara_years(6, &OWN), 7);
        // Scorpio: Mars alone in Aries, Ketu with the Sun in Leo → Ketu; Sc→Le = 10 − 1 = 9.
        assert_eq!(chara_lord(7, &OWN), Some(8));
        assert_eq!(chara_years(7, &OWN), 9);
        // Aquarius: Rahu in Aquarius → count from Saturn, reverse Aq→Cp = 2 − 1 = 1.
        assert_eq!(chara_lord(10, &OWN), Some(6));
        assert_eq!(chara_years(10, &OWN), 1);
        // Pisces (even-footed), Jupiter in Sagittarius: Pi→Aq→Cp→Sg = 4 − 1 = 3.
        assert_eq!(chara_years(11, &OWN), 3);
        // Exaltation: Sun in Aries for Leo: Le→Cn→Ge→Ta→Ar = 5 − 1 + 1 = 5.
        let mut g = OWN;
        g[0] = 15.0;
        assert_eq!(chara_years(4, &g), 5);
        // Debilitation: Jupiter in Capricorn for Sagittarius: 2 − 1 − 1 = 0 → floored to 1.
        g[4] = 285.0;
        assert_eq!(chara_years(8, &g), 1);
        // Both co-lords in the sign: Mars and Ketu in Scorpio → 12.
        g[2] = 220.0;
        g[8] = 225.0;
        assert_eq!(chara_lord(7, &g), None);
        assert_eq!(chara_years(7, &g), 12);
        // Mercury in Virgo: own sign, 12 flat.
        g[3] = 160.0;
        assert_eq!(chara_years(5, &g), 12);
    }

    #[test]
    fn cycle_bounds_and_contiguity() {
        let b = Instant::j2000();
        for lagna in [5.0, 35.0, 100.0, 200.0, 340.0] {
            let v = chara(lagna, &OWN, b, YEAR_365_25_NS);
            assert_eq!(v.len(), 12);
            assert_eq!(v[0].start, b);
            let total = v[11].end.ns_since(b) / YEAR_365_25_NS;
            assert!((12..=144).contains(&total), "{total}");
            for p in &v {
                let y = p.end.ns_since(p.start) / YEAR_365_25_NS;
                assert!((1..=12).contains(&y), "{} {y}", p.label);
            }
            for w in v.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
        }
        let v = chara(5.0, &OWN, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Aries");
        assert_eq!(v[11].end.ns_since(b), 107 * YEAR_365_25_NS);
    }

    #[test]
    fn antardashas_start_after_the_sign_and_end_on_it() {
        let b = Instant::j2000();
        let v = chara(5.0, &OWN, b, YEAR_365_25_NS);
        let p = &v[1]; // Taurus, reverse by the ninth-house rule
        let s = chara_sub(p, chara_direction(1));
        assert_eq!(s.len(), 12);
        assert_eq!(s[0].label, "Aries");
        assert_eq!(s[1].label, "Pisces");
        assert_eq!(s[11].label, "Taurus");
        assert_eq!(s[0].start, p.start);
        assert_eq!(s[11].end, p.end);
        assert_eq!(s[0].end.ns_since(s[0].start), YEAR_365_25_NS);
        let s = chara_sub(&v[0], chara_direction(0));
        assert_eq!(s[0].label, "Taurus");
        assert_eq!(s[11].label, "Aries");
        for w in s.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
