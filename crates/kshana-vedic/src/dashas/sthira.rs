//! Sthira dasha (Jaimini fixed-period rasi dasha).
//!
//! # Sources
//!
//! Jaimini, *Upadesa Sutras* 2.4 (Sanjay Rath's translation, Sagar 1997): the sthira
//! ("fixed") dasha gives each sign a fixed term, 7 years to a movable, 8 to a fixed and 9
//! to a dual sign, and begins from the sign of Brahma; P. V. R. Narasimha Rao, *Vedic
//! Astrology: An Integrated Approach* (Sagar, 2000), chapter "Sthira Dasa" (dasas from
//! Brahma's sign in zodiacal order, antardasas from the dasa sign in zodiacal order);
//! PyJHora `src/jhora/horoscope/dhasa/raasi/sthira.py` (`_dhasa_duration`,
//! `get_dhasa_antardhasa`: seed `house.brahma`, forward sequence, children forward from
//! the parent sign, equal split).
//!
//! * Brahma: [`super::rasi_strength::brahma`].
//! * Order: twelve signs zodiacally from Brahma's sign; one cycle of 96 years.
//! * Antardashas: twelve equal parts, zodiacally from the dasha sign itself.

use kshana_time::Instant;

use super::narayana::{equal_sub, lay_out_signs};
use super::rasi_strength::{brahma, is_dual, is_movable, RasiChart};
use super::{sign_index, Period};

/// Fixed years of a sign: movable 7, fixed 8, dual 9.
pub const fn sthira_years(sign: usize) -> u32 {
    if is_movable(sign) {
        7
    } else if is_dual(sign) {
        9
    } else {
        8
    }
}

/// The starting sign: the sign Brahma occupies.
pub fn sthira_seed(chart: &RasiChart) -> usize {
    chart.sign[brahma(chart)]
}

/// One cycle of twelve Sthira mahadashas (96 years) from `birth`.
pub fn sthira(
    lagna_sidereal: f64,
    grahas: &[f64; 9],
    birth: Instant,
    year_ns: i128,
) -> Vec<Period> {
    let chart = RasiChart::new(lagna_sidereal, grahas);
    let seed = sthira_seed(&chart);
    let cycle: Vec<(usize, u32)> = (0..12)
        .map(|k| {
            let s = (seed + k) % 12;
            (s, sthira_years(s))
        })
        .collect();
    lay_out_signs(&cycle, birth, year_ns)
}

/// Twelve equal antardashas zodiacally from the period's own sign. Empty when the label
/// is not a sign.
pub fn sthira_sub(period: &Period) -> Vec<Period> {
    let Some(sign) = sign_index(&period.label) else {
        return Vec::new();
    };
    equal_sub(period, &std::array::from_fn(|i| (sign + i) % 12))
}

#[cfg(test)]
mod tests {
    use super::super::rasi_strength::{KOCHI_GRAHAS, KOCHI_LAGNA};
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn years_by_nature() {
        let y: Vec<u32> = (0..12).map(sthira_years).collect();
        assert_eq!(y, vec![7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9]);
        assert_eq!(y.iter().sum::<u32>(), 96);
    }

    #[test]
    fn kochi_from_brahma_by_hand() {
        // Brahma is Venus (see rasi_strength), in Sagittarius: Sg 9, Cp 7, Aq 8, Pi 9, …
        let b = Instant::j2000();
        let v = sthira(KOCHI_LAGNA, &KOCHI_GRAHAS, b, YEAR_365_25_NS);
        assert_eq!(v.len(), 12);
        assert_eq!(v[0].label, "Sagittarius");
        assert_eq!(v[0].end.ns_since(b), 9 * YEAR_365_25_NS);
        assert_eq!(v[1].label, "Capricorn");
        assert_eq!(v[1].end.ns_since(v[1].start), 7 * YEAR_365_25_NS);
        assert_eq!(v[2].label, "Aquarius");
        assert_eq!(v[11].label, "Scorpio");
        assert_eq!(v[11].end.ns_since(b), 96 * YEAR_365_25_NS);
        let s = sthira_sub(&v[0]);
        assert_eq!(s[0].label, "Sagittarius");
        assert_eq!(s[11].label, "Scorpio");
        assert_eq!(s[11].end, v[0].end);
        assert_eq!(s[0].end.ns_since(s[0].start), 9 * YEAR_365_25_NS / 12);
    }
}
