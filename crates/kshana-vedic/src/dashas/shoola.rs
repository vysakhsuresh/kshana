//! Shoola dasha (Jaimini's trident dasha, used for longevity and the timing of death and
//! danger), in its Niryana form.
//!
//! # Sources
//!
//! Jaimini, *Upadesa Sutras* 2.4 (Sanjay Rath's translation and commentary, Sagar 1997)
//! and K. N. Rao's longevity teaching: the Niryana Shoola dasha of the native starts
//! from the stronger of the 2nd and 8th houses, every sign runs 9 years, in zodiacal
//! order. P. V. R. Narasimha Rao, *Vedic Astrology: An Integrated Approach* (Sagar,
//! 2000), chapter "Shoola Dasa", and PyJHora
//! `src/jhora/horoscope/dhasa/raasi/shoola.py` (`shoola_dhasa`, parameter
//! `house_index`: the dasha starts from the stronger of the sign `house_index` from the
//! lagna and the 7th from it; PyJHora's default is 1, the lagna/7th pair; the 2nd/8th
//! pair is `house_index = 2`). The relative being studied changes the pair (e.g. 3rd/9th
//! for the father in some schools), so the house is a parameter here.
//!
//! * Order: twelve signs zodiacally from the starting sign, 9 years each, one cycle of
//!   108 years.
//! * Antardashas: twelve equal parts zodiacally from the stronger of the dasha sign and
//!   the 7th from it (PyJHora `get_dhasa_bhukthi` default `antardhasa_seed_option = 2`).

use kshana_time::Instant;

use super::narayana::{equal_sub, lay_out_signs};
use super::rasi_strength::{stronger_rasi, RasiChart};
use super::{sign_index, Period};

/// Years of every Shoola dasha.
pub const SHOOLA_YEARS: u32 = 9;

/// The starting sign: the stronger of the sign `house` (1..=12) from the lagna and the
/// 7th from it. `house = 2` is the native's Niryana Shoola (2nd vs 8th).
pub fn shoola_seed(chart: &RasiChart, house: usize) -> usize {
    let base = (chart.lagna + house.clamp(1, 12) - 1) % 12;
    stronger_rasi(chart, base, base + 6)
}

/// One cycle of twelve Shoola mahadashas (108 years) from `birth`, starting from the
/// stronger of house `house` and the 7th from it.
pub fn shoola(
    lagna_sidereal: f64,
    grahas: &[f64; 9],
    house: usize,
    birth: Instant,
    year_ns: i128,
) -> Vec<Period> {
    let chart = RasiChart::new(lagna_sidereal, grahas);
    let seed = shoola_seed(&chart, house);
    let cycle: Vec<(usize, u32)> = (0..12).map(|k| ((seed + k) % 12, SHOOLA_YEARS)).collect();
    lay_out_signs(&cycle, birth, year_ns)
}

/// Twelve equal antardashas zodiacally from the stronger of the period's sign and the
/// 7th from it. Empty when the label is not a sign.
pub fn shoola_sub(period: &Period, lagna_sidereal: f64, grahas: &[f64; 9]) -> Vec<Period> {
    let Some(sign) = sign_index(&period.label) else {
        return Vec::new();
    };
    let chart = RasiChart::new(lagna_sidereal, grahas);
    let seed = stronger_rasi(&chart, sign, sign + 6);
    equal_sub(period, &std::array::from_fn(|i| (seed + i) % 12))
}

#[cfg(test)]
mod tests {
    use super::super::rasi_strength::{KOCHI_GRAHAS, KOCHI_LAGNA};
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn kochi_by_hand() {
        // 2nd/8th: Taurus beats Scorpio by rule 6 (see rasi_strength) → Ta, Ge, … 9 each.
        let b = Instant::j2000();
        let v = shoola(KOCHI_LAGNA, &KOCHI_GRAHAS, 2, b, YEAR_365_25_NS);
        assert_eq!(v.len(), 12);
        assert_eq!(v[0].label, "Taurus");
        assert_eq!(v[1].label, "Gemini");
        assert_eq!(v[0].end.ns_since(b), 9 * YEAR_365_25_NS);
        assert_eq!(v[11].end.ns_since(b), 108 * YEAR_365_25_NS);
        // Lagna/7th (PyJHora's default pair): Libra.
        let c = RasiChart::new(KOCHI_LAGNA, &KOCHI_GRAHAS);
        assert_eq!(shoola_seed(&c, 1), 6);
        // Taurus antardashas: Taurus vs Scorpio again → Taurus first.
        let s = shoola_sub(&v[0], KOCHI_LAGNA, &KOCHI_GRAHAS);
        assert_eq!(s[0].label, "Taurus");
        assert_eq!(s[11].label, "Aries");
        // Gemini vs Sagittarius: Sagittarius holds four grahas → antardashas from it.
        let s = shoola_sub(&v[1], KOCHI_LAGNA, &KOCHI_GRAHAS);
        assert_eq!(s[0].label, "Sagittarius");
        for w in s.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
