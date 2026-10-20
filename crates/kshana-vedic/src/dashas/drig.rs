//! Drig dasha (Jaimini's aspect dasha, for spiritual life and the dharma houses).
//!
//! # Sources
//!
//! Jaimini, *Upadesa Sutras* 2.4 (Sanjay Rath's translation, Sagar 1997): the dasha runs
//! through the 9th, 10th and 11th from the lagna, each followed by the signs it aspects;
//! P. V. R. Narasimha Rao, *Vedic Astrology: An Integrated Approach* (Sagar, 2000),
//! chapter "Drig Dasa"; PyJHora `src/jhora/horoscope/dhasa/raasi/drig.py` (`drig_dhasa`,
//! `_antardhasa_from_planet_positions`) with `house.aspected_kendras_of_raasi`.
//!
//! * Order: for each of the 9th, 10th and 11th signs from the lagna, the sign itself and
//!   then the three signs it aspects by rasi drishti — zodiacally from it for an
//!   odd-footed sign, anti-zodiacally for an even-footed one ([`drig_sequence`]).
//!   For a dual lagna the 9th (fixed) and the 11th (movable) aspect each other, so the
//!   literal rule lists each of them twice and never reaches the two signs next to
//!   them (lagna Gemini: Aquarius and Aries repeat, Taurus and Capricorn are absent).
//!   PyJHora keeps the repetition and so does this module; no source consulted gives a
//!   different rule for this case.
//! * Years: as in Narayana dasha ([`super::narayana::narayana_years`]); a second cycle
//!   gives each sign 12 minus its first years, 144 years in all; zero-year periods are
//!   left out.
//! * Antardashas: twelve equal parts from the stronger of the dasha sign and the 7th
//!   from it, zodiacally for an odd dasha sign and anti-zodiacally for an even one; a
//!   movable dasha sign takes them one after another, a fixed one every sixth sign, a
//!   dual one the kendras, then the panapharas, then the apoklimas (offsets 1, 4, 7, 10,
//!   5, 8, 11, 2, 9, 12, 3, 6).

use kshana_time::Instant;

use super::chara::is_odd_footed;
use super::narayana::{equal_sub, lay_out_signs, narayana_years};
use super::rasi_strength::{is_fixed, is_movable, rasi_aspects, stronger_rasi, RasiChart};
use super::{sign_index, Period};

/// The twelve Drig dasha signs from a lagna sign.
pub fn drig_sequence(lagna_sign: usize) -> [usize; 12] {
    let mut out = [0usize; 12];
    for (k, h) in [8usize, 9, 10].iter().enumerate() {
        let s = (lagna_sign + h) % 12;
        let mut asp = rasi_aspects(s); // zodiacal order after s
        if !is_odd_footed(s) {
            asp.reverse();
        }
        out[4 * k] = s;
        out[4 * k + 1..4 * k + 4].copy_from_slice(&asp);
    }
    out
}

/// Two cycles of Drig mahadashas from `birth`, 144 years in all.
pub fn drig(lagna_sidereal: f64, grahas: &[f64; 9], birth: Instant, year_ns: i128) -> Vec<Period> {
    let chart = RasiChart::new(lagna_sidereal, grahas);
    let seq = drig_sequence(chart.lagna);
    let first: Vec<(usize, u32)> = seq
        .iter()
        .map(|s| (*s, narayana_years(&chart, *s)))
        .collect();
    let second = first.iter().map(|(s, y)| (*s, 12u32.saturating_sub(*y)));
    let both: Vec<(usize, u32)> = first.iter().copied().chain(second).collect();
    lay_out_signs(&both, birth, year_ns)
}

/// The antardasha order inside a Drig dasha of `dasha_sign`.
pub fn drig_antar_order(chart: &RasiChart, dasha_sign: usize) -> [usize; 12] {
    const DUAL: [usize; 12] = [0, 3, 6, 9, 4, 7, 10, 1, 8, 11, 2, 5];
    let d = dasha_sign % 12;
    let seed = stronger_rasi(chart, d, d + 6);
    let forward = d % 2 == 0;
    std::array::from_fn(|h| {
        let raw = if is_fixed(d) {
            5 * h
        } else if is_movable(d) {
            h
        } else {
            DUAL[h]
        };
        let step = raw % 12;
        if forward {
            (seed + step) % 12
        } else {
            (seed + 12 - step) % 12
        }
    })
}

/// Twelve equal antardashas of a Drig period. Empty when the label is not a sign.
pub fn drig_sub(period: &Period, lagna_sidereal: f64, grahas: &[f64; 9]) -> Vec<Period> {
    let Some(sign) = sign_index(&period.label) else {
        return Vec::new();
    };
    let chart = RasiChart::new(lagna_sidereal, grahas);
    equal_sub(period, &drig_antar_order(&chart, sign))
}

#[cfg(test)]
mod tests {
    use super::super::rasi_strength::{KOCHI_GRAHAS, KOCHI_LAGNA};
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn sequence_covers_all_signs_except_for_dual_lagnas() {
        for l in 0..12 {
            let mut v = drig_sequence(l).to_vec();
            v.sort_unstable();
            v.dedup();
            // Dual lagna: the 9th (fixed) and 11th (movable) aspect each other.
            let expect = if l % 3 == 2 { 10 } else { 12 };
            assert_eq!(v.len(), expect, "lagna {l}");
        }
        // Lagna Gemini: Aq (even-footed) Li, Cn, Ar; Pi (even-footed) Sg, Vi, Ge; Ar Le, Sc, Aq.
        assert_eq!(drig_sequence(2), [10, 6, 3, 0, 11, 8, 5, 2, 0, 4, 7, 10]);
    }

    #[test]
    fn kochi_by_hand() {
        // Lagna Aries: 9th Sagittarius (odd-footed) → Sg, Pi, Ge, Vi; 10th Capricorn
        // (even-footed, aspects Ta, Le, Sc) → Cp, Sc, Le, Ta; 11th Aquarius (even-footed,
        // aspects Ar, Cn, Li) → Aq, Li, Cn, Ar. Narayana years (see narayana tests): Sg 6,
        // Pi 9, Ge 7, Vi 8, Cp 1, Sc 1, Le 6, Ta 7, Aq 2, Li 2, Cn 7, Ar 8 = 64.
        assert_eq!(drig_sequence(0), [8, 11, 2, 5, 9, 7, 4, 1, 10, 6, 3, 0]);
        let b = Instant::j2000();
        let v = drig(KOCHI_LAGNA, &KOCHI_GRAHAS, b, YEAR_365_25_NS);
        assert_eq!(v.len(), 24);
        assert_eq!(v[0].label, "Sagittarius");
        assert_eq!(v[0].end.ns_since(b), 6 * YEAR_365_25_NS);
        assert_eq!(v[1].label, "Pisces");
        assert_eq!(v[11].end.ns_since(b), 64 * YEAR_365_25_NS);
        assert_eq!(v[23].end.ns_since(b), 144 * YEAR_365_25_NS);
        // Sagittarius antardashas: Sagittarius (four grahas) over Gemini; odd, dual →
        // Sg, Pi, Ge, Vi, Ar, Cn, Li, Cp, Le, Sc, Aq, Ta.
        let s = drig_sub(&v[0], KOCHI_LAGNA, &KOCHI_GRAHAS);
        let labels: Vec<&str> = s.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Sagittarius",
                "Pisces",
                "Gemini",
                "Virgo",
                "Aries",
                "Cancer",
                "Libra",
                "Capricorn",
                "Leo",
                "Scorpio",
                "Aquarius",
                "Taurus"
            ]
        );
        assert_eq!(s[11].end, v[0].end);
    }

    #[test]
    fn fixed_antardashas_jump_six() {
        // Everything in Taurus: a Taurus dasha (even, fixed) seeds from Taurus and runs
        // back every sixth sign: Ta, Sg, Cn, Aq, Vi, Ar, …
        let g = [45.0; 9];
        let c = RasiChart::new(45.0, &g);
        let o = drig_antar_order(&c, 1);
        assert_eq!(&o[..6], &[1, 8, 3, 10, 5, 0]);
    }
}
