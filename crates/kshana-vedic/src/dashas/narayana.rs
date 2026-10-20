//! Narayana dasha (Jaimini rasi dasha), the method of P. V. R. Narasimha Rao and Sanjay
//! Rath as Jagannatha Hora computes it.
//!
//! # Sources
//!
//! BPHS has no Narayana chapter; the dasha is Jaimini's (*Upadesa Sutras* 2.4, Sanjay
//! Rath's translation and commentary, Sagar 1997), taught as below in P. V. R. Narasimha
//! Rao, *Vedic Astrology: An Integrated Approach* (Sagar, 2000), chapter "Narayana
//! Dasa", and implemented to match Jagannatha Hora in PyJHora
//! `src/jhora/horoscope/dhasa/raasi/narayana.py` (`_dhasa_duration`,
//! `_narayana_dhasa_calculation`, `_narayana_antardhasa`) with the progression tables
//! `const.narayana_dhasa_normal_progression`, `..._saturn_exception_progression` and
//! `..._ketu_exception_progression`.
//!
//! * Starting sign: the stronger of the lagna and the 7th
//!   ([`super::rasi_strength::stronger_rasi`]).
//! * Order of the twelve dashas from that sign: an odd sign runs zodiacally, an even
//!   sign anti-zodiacally; a movable sign takes the signs one after another, a fixed sign
//!   every sixth sign, a dual sign the kendras from it each followed by its trines
//!   ([`NORMAL_ORDER`]). Exceptions: Saturn in the starting sign makes the order plain
//!   zodiacal ([`SATURN_ORDER`]); Ketu in it reverses the direction ([`KETU_ORDER`]); Ketu
//!   is checked first, so Ketu with Saturn gives the Ketu order.
//! * Years of a sign: count from the sign to its lord's sign — forward for an odd-footed
//!   sign (Aries, Taurus, Gemini, Libra, Scorpio, Sagittarius), backward for an
//!   even-footed one — and subtract one; a lord in the sign itself gives 12. Then one
//!   year more if the lord is exalted, one less if debilitated ([`super::rasi_strength::
//!   DIGNITY`]). Scorpio and Aquarius take the stronger co-lord. The result lies in
//!   0..=12; a 0-year dasha is skipped in the output (PyJHora emits it with zero length).
//! * Second cycle: the same twelve signs again, each for 12 minus its first-cycle years
//!   (floored at zero), so the two cycles total 144 years.
//! * Antardashas: twelve equal parts of the dasha. They start from the stronger of the
//!   sign holding the dasha sign's lord and the sign holding the lord of the 7th from the
//!   dasha sign (co-lords judged with the dasha rule 5a), run zodiacally if that sign is
//!   odd and anti-zodiacally if even, always zodiacally if Saturn is in it, and reverse
//!   again if Ketu is in the dasha sign.
//!
//! Inputs are sidereal longitudes in degrees: the lagna and the nine grahas Sun, Moon,
//! Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu.

use kshana_time::Instant;

use super::chara::is_odd_footed;
use super::rasi_strength::{
    sign_lord, stronger_rasi, RasiChart, DEBILITATED, DIGNITY, EXALTED, KETU, SATURN,
};
use super::{sign_index, Period, SIGN_NAMES};

/// PyJHora `const.narayana_dhasa_normal_progression`: the dasha order from each sign.
pub const NORMAL_ORDER: [[usize; 12]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    [1, 8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6],
    [2, 10, 6, 5, 1, 9, 8, 4, 0, 11, 7, 3],
    [3, 2, 1, 0, 11, 10, 9, 8, 7, 6, 5, 4],
    [4, 9, 2, 7, 0, 5, 10, 3, 8, 1, 6, 11],
    [5, 9, 1, 2, 6, 10, 11, 3, 7, 8, 0, 4],
    [6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5],
    [7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5, 0],
    [8, 4, 0, 11, 7, 3, 2, 10, 6, 5, 1, 9],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 11, 10],
    [10, 3, 8, 1, 6, 11, 4, 9, 2, 7, 0, 5],
    [11, 3, 7, 8, 0, 4, 5, 9, 1, 2, 6, 10],
];

/// PyJHora `const.narayana_dhasa_saturn_exception_progression`: Saturn in the starting
/// sign, plain zodiacal order.
pub const SATURN_ORDER: [[usize; 12]; 12] = {
    let mut t = [[0usize; 12]; 12];
    let mut s = 0;
    while s < 12 {
        let mut k = 0;
        while k < 12 {
            t[s][k] = (s + k) % 12;
            k += 1;
        }
        s += 1;
    }
    t
};

/// PyJHora `const.narayana_dhasa_ketu_exception_progression`: Ketu in the starting sign,
/// the normal pattern with its direction reversed.
pub const KETU_ORDER: [[usize; 12]; 12] = [
    [0, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
    [1, 6, 11, 4, 9, 2, 7, 0, 5, 10, 3, 8],
    [2, 6, 10, 11, 3, 7, 8, 0, 4, 5, 9, 1],
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 1, 2],
    [4, 11, 6, 1, 8, 3, 10, 5, 0, 7, 2, 9],
    [5, 1, 9, 8, 4, 0, 11, 7, 3, 2, 10, 6],
    [6, 5, 4, 3, 2, 1, 0, 11, 10, 9, 8, 7],
    [7, 0, 5, 10, 3, 8, 1, 6, 11, 4, 9, 2],
    [8, 0, 4, 5, 9, 1, 2, 6, 10, 11, 3, 7],
    [9, 10, 11, 0, 1, 2, 3, 4, 5, 6, 7, 8],
    [10, 5, 0, 7, 2, 9, 4, 11, 6, 1, 8, 3],
    [11, 7, 3, 2, 10, 6, 5, 1, 9, 8, 4, 0],
];

/// Inclusive count from `from` to `to`, forward or backward (1 when equal).
pub(crate) const fn count(from: usize, to: usize, forward: bool) -> usize {
    if forward {
        (to + 12 - from) % 12 + 1
    } else {
        (from + 12 - to) % 12 + 1
    }
}

/// First-cycle Narayana years of `sign` were `lord` its lord (0..=12).
pub fn years_with_lord(chart: &RasiChart, sign: usize, lord: usize) -> u32 {
    let sign = sign % 12;
    let at = chart.sign[lord];
    let mut years = count(sign, at, is_odd_footed(sign)) as i32 - 1;
    if years <= 0 {
        years = 12;
    }
    match DIGNITY[lord][at] {
        EXALTED => years += 1,
        DEBILITATED => years -= 1,
        _ => {}
    }
    years.clamp(0, 12) as u32
}

/// First-cycle Narayana years of `sign` (0..=12), Scorpio and Aquarius by the stronger
/// co-lord.
pub fn narayana_years(chart: &RasiChart, sign: usize) -> u32 {
    years_with_lord(chart, sign, sign_lord(chart, sign, false))
}

/// The starting sign: the stronger of the lagna and the 7th.
pub fn narayana_seed(chart: &RasiChart) -> usize {
    stronger_rasi(chart, chart.lagna, chart.lagna + 6)
}

/// The twelve dasha signs in order, with the Saturn and Ketu exceptions.
pub fn narayana_sequence(chart: &RasiChart) -> [usize; 12] {
    let seed = narayana_seed(chart);
    if chart.sign[KETU] == seed {
        KETU_ORDER[seed]
    } else if chart.sign[SATURN] == seed {
        SATURN_ORDER[seed]
    } else {
        NORMAL_ORDER[seed]
    }
}

/// Lay out `(sign, years)` back to back from `birth`, skipping zero-year entries.
/// Boundaries are exact: birth plus cumulative years × `year_ns`.
pub(crate) fn lay_out_signs(cycle: &[(usize, u32)], birth: Instant, year_ns: i128) -> Vec<Period> {
    let mut out = Vec::with_capacity(cycle.len());
    let mut t = birth;
    let mut cum: i128 = 0;
    for (sign, years) in cycle {
        if *years == 0 {
            continue;
        }
        cum += *years as i128;
        let end = birth.add_ns(cum * year_ns);
        out.push(Period {
            label: SIGN_NAMES[*sign % 12].to_string(),
            start: t,
            end,
            level: 1,
        });
        t = end;
    }
    out
}

/// Twelve equal sub-periods of `period` in the given sign order.
pub(crate) fn equal_sub(period: &Period, order: &[usize; 12]) -> Vec<Period> {
    let parts: Vec<(&str, u32)> = order.iter().map(|s| (SIGN_NAMES[*s % 12], 1)).collect();
    super::subdivide_weighted(period, &parts)
}

/// Two cycles of Narayana mahadashas from `birth`: 144 years in all, since each sign's
/// two periods add to 12. Zero-year periods are left out.
pub fn narayana(
    lagna_sidereal: f64,
    grahas: &[f64; 9],
    birth: Instant,
    year_ns: i128,
) -> Vec<Period> {
    let chart = RasiChart::new(lagna_sidereal, grahas);
    let seq = narayana_sequence(&chart);
    let first: Vec<(usize, u32)> = seq
        .iter()
        .map(|s| (*s, narayana_years(&chart, *s)))
        .collect();
    let second = first.iter().map(|(s, y)| (*s, 12u32.saturating_sub(*y)));
    let both: Vec<(usize, u32)> = first.iter().copied().chain(second).collect();
    lay_out_signs(&both, birth, year_ns)
}

/// The antardasha order inside a Narayana dasha of `dasha_sign`.
pub fn narayana_antar_order(chart: &RasiChart, dasha_sign: usize) -> [usize; 12] {
    let dasha_sign = dasha_sign % 12;
    let lord = sign_lord(chart, dasha_sign, true);
    let lord7 = sign_lord(chart, dasha_sign + 6, true);
    let seed = stronger_rasi(chart, chart.sign[lord], chart.sign[lord7]);
    let mut forward = seed % 2 == 0;
    if chart.sign[SATURN] == seed {
        forward = true;
    }
    if chart.sign[KETU] == dasha_sign {
        forward = !forward;
    }
    std::array::from_fn(|i| {
        if forward {
            (seed + i) % 12
        } else {
            (seed + 12 - i) % 12
        }
    })
}

/// Twelve equal antardashas of a Narayana period. Empty when the label is not a sign.
pub fn narayana_sub(period: &Period, lagna_sidereal: f64, grahas: &[f64; 9]) -> Vec<Period> {
    let Some(sign) = sign_index(&period.label) else {
        return Vec::new();
    };
    let chart = RasiChart::new(lagna_sidereal, grahas);
    equal_sub(period, &narayana_antar_order(&chart, sign))
}

#[cfg(test)]
mod tests {
    use super::super::rasi_strength::{KOCHI_GRAHAS, KOCHI_LAGNA};
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn tables_are_permutations_and_ketu_reverses_direction() {
        for s in 0..12 {
            for t in [&NORMAL_ORDER, &SATURN_ORDER, &KETU_ORDER] {
                let mut v = t[s].to_vec();
                assert_eq!(v[0], s);
                v.sort_unstable();
                assert_eq!(v, (0..12).collect::<Vec<_>>());
            }
            // Ketu's order is the normal order's steps negated.
            for k in 0..12 {
                let step_n = (NORMAL_ORDER[s][k] + 12 - s) % 12;
                let step_k = (KETU_ORDER[s][k] + 12 - s) % 12;
                assert_eq!((step_n + step_k) % 12, 0, "sign {s} step {k}");
            }
        }
    }

    #[test]
    fn years_rules() {
        // Lord in its own sign → 12; exalted lord adds one; debilitated subtracts one.
        let g = [124.0, 100.0, 10.0, 70.0, 250.0, 40.0, 280.0, 310.0, 130.0];
        let c = RasiChart::new(0.0, &g);
        assert_eq!(narayana_years(&c, 0), 12); // Mars in Aries
                                               // Leo (even-footed) with the Sun exalted in Aries: backward Le→Ar = 5 − 1 + 1 = 5.
        let mut h = g;
        h[0] = 15.0;
        let c = RasiChart::new(0.0, &h);
        assert_eq!(narayana_years(&c, 4), 5);
        // Sagittarius (odd-footed), Jupiter debilitated in Capricorn: 2 − 1 − 1 = 0.
        h[4] = 285.0;
        let c = RasiChart::new(0.0, &h);
        assert_eq!(narayana_years(&c, 8), 0);
    }

    #[test]
    fn kochi_first_periods_by_hand() {
        // Seed: Libra (stronger than Aries by rule 6, see rasi_strength). No Saturn or
        // Ketu in Libra → movable odd sign, plain zodiacal order from Libra.
        // Years (count to lord, odd-footed forward / even-footed backward, minus one):
        // Li→Venus Sg 3−1 = 2; Sc→Mars Sg 2−1 = 1; Sg→Jupiter Ge 7−1 = 6; Cp→Saturn Sg
        // (back) 2−1 = 1; Aq→Saturn Sg (back) 3−1 = 2; Pi→Jupiter Ge (back) 10−1 = 9;
        // Ar→Mars Sg 9−1 = 8; Ta→Venus Sg 8−1 = 7; Ge→Mercury Cp 8−1 = 7; Cn→Moon Sg
        // (back) 8−1 = 7; Le→Sun Aq (back) 7−1 = 6; Vi→Mercury Cp (back) 9−1 = 8. No lord
        // exalted or debilitated. Sum 64; second cycle 80; total 144.
        let c = RasiChart::new(KOCHI_LAGNA, &KOCHI_GRAHAS);
        assert_eq!(narayana_seed(&c), 6);
        assert_eq!(
            narayana_sequence(&c),
            [6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5]
        );
        let years: Vec<u32> = narayana_sequence(&c)
            .iter()
            .map(|s| narayana_years(&c, *s))
            .collect();
        assert_eq!(years, vec![2, 1, 6, 1, 2, 9, 8, 7, 7, 7, 6, 8]);
        let b = Instant::j2000();
        let v = narayana(KOCHI_LAGNA, &KOCHI_GRAHAS, b, YEAR_365_25_NS);
        assert_eq!(v.len(), 24);
        assert_eq!(v[0].label, "Libra");
        assert_eq!(v[0].end.ns_since(b), 2 * YEAR_365_25_NS);
        assert_eq!(v[1].label, "Scorpio");
        assert_eq!(v[12].label, "Libra");
        assert_eq!(v[12].end.ns_since(v[12].start), 10 * YEAR_365_25_NS);
        assert_eq!(v[23].end.ns_since(b), 144 * YEAR_365_25_NS);
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        // Libra's antardashas: Venus (lord) and Mars (lord of Aries) both in Sagittarius
        // → seed Sagittarius, odd and holding Saturn → zodiacal: Sg, Cp, …, Sc.
        let s = narayana_sub(&v[0], KOCHI_LAGNA, &KOCHI_GRAHAS);
        assert_eq!(s.len(), 12);
        assert_eq!(s[0].label, "Sagittarius");
        assert_eq!(s[1].label, "Capricorn");
        assert_eq!(s[11].label, "Scorpio");
        assert_eq!(s[0].start, v[0].start);
        assert_eq!(s[11].end, v[0].end);
    }

    #[test]
    fn saturn_and_ketu_exceptions() {
        // Lagna Taurus with everything in Taurus except Ketu: Taurus has 8 grahas → seed;
        // Saturn there → zodiacal.
        let mut g = [45.0; 9];
        g[8] = 225.0;
        let c = RasiChart::new(40.0, &g);
        assert_eq!(narayana_seed(&c), 1);
        assert_eq!(narayana_sequence(&c), SATURN_ORDER[1]);
        // Move Ketu into Taurus too: the Ketu order wins.
        g[8] = 46.0;
        let c = RasiChart::new(40.0, &g);
        assert_eq!(narayana_sequence(&c), KETU_ORDER[1]);
    }
}
