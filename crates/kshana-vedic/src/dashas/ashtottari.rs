//! Ashtottari dasha: eight lords in a 108-year cycle.
//!
//! Conventions, verified against BPHS ch. 47 (Santhanam tr., "Ashtottari Dasa") as
//! encoded in four independent JHora-lineage implementations — PyJHora
//! `src/jhora/horoscope/dhasa/graha/ashtottari.py`
//! (`{0: [(6, 9), 6], 1: [(10, 12), 15], 2: [(13, 16), 8], 3: [(17, 19), 17],
//! 6: [(20, 22), 10], 4: [(23, 25), 19], 7: [(26, 2), 12], 5: [(3, 5), 21]}`, 1-based
//! inclusive nakshatra ranges), `vedika-io/xalen-ephemeris` `ashtottari.rs`,
//! `Horace-Maxwell/Horosa-Web` `jyotish_engine.py` and `TheDaniel166/moira`
//! `ALTERNATE_DASHAS_BACKEND_STANDARD.md`:
//!
//! * Order and years: Sun 6, Moon 15, Mars 8, Mercury 17, Saturn 10, Jupiter 19,
//!   Rahu 12, Venus 21 (Ketu has no dasha); total 108.
//! * Nakshatra groups, alternating 4 and 3 from Ardra: Sun = Ardra, Punarvasu, Pushya,
//!   Ashlesha; Moon = Magha, P.Phalguni, U.Phalguni; Mars = Hasta, Chitra, Swati,
//!   Vishakha; Mercury = Anuradha, Jyeshtha, Mula; Saturn = P.Ashadha, U.Ashadha,
//!   Shravana; Jupiter = Dhanishta, Shatabhisha, P.Bhadrapada; Rahu = U.Bhadrapada,
//!   Revati, Ashwini, Bharani; Venus = Krittika, Rohini, Mrigashira. (A 28-nakshatra
//!   variant inserting Abhijit into Saturn's group exists and is not implemented.)
//! * Balance at birth: the fraction of the lord's whole group span (3 or 4 nakshatras)
//!   that the Moon has traversed, times the lord's years; Rahu's group wraps through 0°.
//! * Antardashas: the eight lords in cycle order from the period's own lord, each
//!   `maha years × antar years / 108`. This is JHora's choice (PyJHora notes
//!   `ashtottari_bhukthi_starts_from_dhasa_lord = True`; P.V.R. Narasimha Rao's book
//!   starts from the next lord instead).
//! * Applicability (BPHS 47.1–2, PyJHora `applicability_check`): Rahu is in a kendra or a
//!   trikona from the sign of the lagna lord and not in the lagna itself.

use kshana_time::Instant;

use super::{fraction_ns, graha_from_name, graha_name, lay_out_cycle, subdivide_weighted, Period};
use crate::{DashaLord, NAKSHATRA_SPAN};

/// The eight lords in cycle order.
pub const ASHTOTTARI_ORDER: [DashaLord; 8] = [
    DashaLord::Sun,
    DashaLord::Moon,
    DashaLord::Mars,
    DashaLord::Mercury,
    DashaLord::Saturn,
    DashaLord::Jupiter,
    DashaLord::Rahu,
    DashaLord::Venus,
];

/// Years of each lord, in [`ASHTOTTARI_ORDER`]; they sum to 108.
pub const ASHTOTTARI_YEARS: [u32; 8] = [6, 15, 8, 17, 10, 19, 12, 21];

/// Nakshatra group of each lord in [`ASHTOTTARI_ORDER`]: `(first nakshatra index, count)`,
/// 0 = Ashwini. Rahu's group (25, 4) runs U.Bhadrapada, Revati, Ashwini, Bharani.
pub const ASHTOTTARI_GROUPS: [(usize, usize); 8] = [
    (5, 4),  // Sun: Ardra … Ashlesha
    (9, 3),  // Moon: Magha … U.Phalguni
    (12, 4), // Mars: Hasta … Vishakha
    (16, 3), // Mercury: Anuradha … Mula
    (19, 3), // Saturn: P.Ashadha … Shravana
    (22, 3), // Jupiter: Dhanishta … P.Bhadrapada
    (25, 4), // Rahu: U.Bhadrapada, Revati, Ashwini, Bharani
    (2, 3),  // Venus: Krittika … Mrigashira
];

/// Total years of one Ashtottari cycle.
pub const ASHTOTTARI_CYCLE_YEARS: u32 = 108;

/// Years of a lord's mahadasha; 0 for Ketu, which has none.
pub fn years_of(lord: DashaLord) -> u32 {
    ASHTOTTARI_ORDER
        .iter()
        .position(|l| *l == lord)
        .map_or(0, |i| ASHTOTTARI_YEARS[i])
}

/// Position in [`ASHTOTTARI_ORDER`] and lord of a 0-based nakshatra index.
pub fn lord_of_nakshatra(index: usize) -> (usize, DashaLord) {
    let index = index % 27;
    let pos = ASHTOTTARI_GROUPS
        .iter()
        .position(|(start, count)| (0..*count).any(|k| (start + k) % 27 == index))
        .unwrap_or(0);
    (pos, ASHTOTTARI_ORDER[pos])
}

/// Mahadasha sequence from the running lord at birth, repeating the 108-year cycle until
/// [`super::HORIZON_YEARS`] after birth. `moon_sidereal` in degrees.
pub fn ashtottari(moon_sidereal: f64, birth: Instant, year_ns: i128) -> Vec<Period> {
    let nak = crate::nakshatra(moon_sidereal);
    let (pos, _) = lord_of_nakshatra(nak.index);
    let (start, count) = ASHTOTTARI_GROUPS[pos];
    let group_start = start as f64 * NAKSHATRA_SPAN;
    let span = count as f64 * NAKSHATRA_SPAN;
    let elapsed = (moon_sidereal.rem_euclid(360.0) - group_start).rem_euclid(360.0) / span;
    let elapsed_ns = fraction_ns(elapsed, ASHTOTTARI_YEARS[pos] as i128 * year_ns);
    let cycle: Vec<(&str, u32)> = ASHTOTTARI_ORDER
        .iter()
        .zip(ASHTOTTARI_YEARS.iter())
        .map(|(l, y)| (graha_name(*l), *y))
        .collect();
    lay_out_cycle(&cycle, pos, elapsed_ns, birth, year_ns, 1)
}

/// Antardashas of an Ashtottari period: the eight lords in cycle order from the period's
/// own lord, proportioned by their years over 108. Empty when the label is not a lord.
pub fn ashtottari_sub(period: &Period) -> Vec<Period> {
    let Some(first) =
        graha_from_name(&period.label).and_then(|g| ASHTOTTARI_ORDER.iter().position(|l| *l == g))
    else {
        return Vec::new();
    };
    let parts: Vec<(&str, u32)> = (0..8)
        .map(|i| (first + i) % 8)
        .map(|i| (graha_name(ASHTOTTARI_ORDER[i]), ASHTOTTARI_YEARS[i]))
        .collect();
    subdivide_weighted(period, &parts)
}

/// BPHS applicability: Rahu occupies a kendra (1, 4, 7, 10) or a trikona (1, 5, 9) counted
/// from the sign of the lagna lord, and is not in the lagna. All arguments are sign
/// indices, 0 = Aries.
pub fn applies(lagna_sign: usize, lagna_lord_sign: usize, rahu_sign: usize) -> bool {
    let (lagna, lord, rahu) = (lagna_sign % 12, lagna_lord_sign % 12, rahu_sign % 12);
    let from_lord = (rahu + 12 - lord) % 12;
    let kendra_or_trikona = matches!(from_lord, 0 | 3 | 6 | 9 | 4 | 8);
    kendra_or_trikona && rahu != lagna
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn cycle_totals_108_years_and_groups_cover_all_27_nakshatras() {
        assert_eq!(ASHTOTTARI_YEARS.iter().sum::<u32>(), ASHTOTTARI_CYCLE_YEARS);
        let mut seen = [0u8; 27];
        for (start, count) in ASHTOTTARI_GROUPS {
            for k in 0..count {
                seen[(start + k) % 27] += 1;
            }
        }
        assert!(seen.iter().all(|c| *c == 1), "{seen:?}");
        let b = Instant::j2000();
        let v = ashtottari(0.0, b, YEAR_365_25_NS);
        assert!(v.len() >= 8);
        assert_eq!(v[7].end.ns_since(v[0].start), 108 * YEAR_365_25_NS);
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(v.last().unwrap().end >= b.add_ns(120 * YEAR_365_25_NS));
    }

    #[test]
    fn table_spot_checks_against_the_sources() {
        // Ardra, Ashlesha → Sun; Magha → Moon; Vishakha → Mars; Mula → Mercury;
        // Shravana → Saturn; P.Bhadrapada → Jupiter; U.Bhadrapada, Bharani → Rahu;
        // Krittika, Mrigashira → Venus.
        let expect = [
            (5, DashaLord::Sun),
            (8, DashaLord::Sun),
            (9, DashaLord::Moon),
            (15, DashaLord::Mars),
            (18, DashaLord::Mercury),
            (21, DashaLord::Saturn),
            (24, DashaLord::Jupiter),
            (25, DashaLord::Rahu),
            (1, DashaLord::Rahu),
            (2, DashaLord::Venus),
            (4, DashaLord::Venus),
        ];
        for (n, lord) in expect {
            assert_eq!(lord_of_nakshatra(n).1, lord, "nakshatra {n}");
        }
        assert_eq!(years_of(DashaLord::Ketu), 0);
        assert_eq!(years_of(DashaLord::Venus), 21);
    }

    #[test]
    fn balance_full_at_group_start_and_zero_at_group_end() {
        let b = Instant::j2000();
        // Sun's group starts at Ardra (5 × 13°20' = 66°40').
        let v = ashtottari(5.0 * NAKSHATRA_SPAN, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Sun");
        assert_eq!(v[0].start, b);
        assert_eq!(v[0].end.ns_since(b), 6 * YEAR_365_25_NS);
        // End of Sun's group (start of Magha): zero balance of Sun.
        let v = ashtottari(9.0 * NAKSHATRA_SPAN - 1e-9, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Sun");
        assert!(v[0].end.ns_since(b) < 1_000_000_000);
        // Start of Pushya (nakshatra 7): half of the Sun group's 4 nakshatras elapsed.
        let v = ashtottari(7.0 * NAKSHATRA_SPAN, b, YEAR_365_25_NS);
        assert!(v[0].end.ns_since(b).abs_diff(3 * YEAR_365_25_NS) < 1_000);
        // Rahu's group wraps: Ashwini 0° is 2/4 of the way through it.
        let v = ashtottari(0.0, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Rahu");
        assert!(v[0].end.ns_since(b).abs_diff(6 * YEAR_365_25_NS) < 1_000);
        assert_eq!(v[1].label, "Venus");
    }

    #[test]
    fn antardashas_partition_from_the_own_lord() {
        let b = Instant::j2000();
        let v = ashtottari(200.0, b, YEAR_365_25_NS);
        let s = ashtottari_sub(&v[2]);
        assert_eq!(s.len(), 8);
        assert_eq!(s[0].label, v[2].label);
        assert_eq!(s[0].start, v[2].start);
        assert_eq!(s[7].end, v[2].end);
        for w in s.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn applicability_rule() {
        // Lagna Aries, lagna lord Mars in Leo (4): Rahu in Scorpio (7) is the 4th from Leo.
        assert!(applies(0, 4, 7));
        // Rahu in Sagittarius (8) is the 5th from Leo.
        assert!(applies(0, 4, 8));
        // Rahu in Aries would be the 9th from Leo but sits in the lagna.
        assert!(!applies(0, 4, 0));
        // Rahu in Virgo (5), the 2nd from Leo.
        assert!(!applies(0, 4, 5));
    }
}
