//! Yogini dasha: eight yoginis in a 36-year cycle.
//!
//! Conventions, verified against the Yogini-dasha chapter of the Brihat Parashara Hora
//! Shastra (Santhanam tr.; verse numbering varies by edition) and the JHora-derived
//! tables in PyJHora (`src/jhora/horoscope/dhasa/graha/yogini.py`, which maps each lord
//! to its nakshatras: Moon {6,14,22}, Sun {7,15,23}, Jupiter {8,16,24},
//! Mars {1,9,17,25}, Mercury {2,10,18,26}, Saturn {3,11,19,27}, Venus {4,12,20},
//! Rahu {5,13,21}, 1-based):
//!
//! * Order and years: Mangala 1 (Moon), Pingala 2 (Sun), Dhanya 3 (Jupiter),
//!   Bhramari 4 (Mars), Bhadrika 5 (Mercury), Ulka 6 (Saturn), Siddha 7 (Venus),
//!   Sankata 8 (Rahu); total 36.
//! * Starting yogini: `(nakshatra number, 1 = Ashwini, + 3) mod 8`, remainder 1 =
//!   Mangala … 7 = Siddha, 0 = Sankata. Worked example from the sources: Anuradha (17)
//!   → 20 mod 8 = 4 → Bhramari; Ashwini (1) → 4 → Bhramari.
//! * Balance at birth: the untraversed fraction of the Moon's nakshatra times the
//!   yogini's years.
//! * Antardashas: the eight yoginis in cycle order starting from the running one, each
//!   `maha years × antar years / 36` (BPHS). PyJHora instead divides equally by eight;
//!   that variant is not implemented.

use kshana_time::Instant;

use super::{fraction_ns, lay_out_cycle, subdivide_weighted, Period};
use crate::DashaLord;

/// Total years of one Yogini cycle.
pub const YOGINI_CYCLE_YEARS: u32 = 36;

/// The eight yoginis in cycle order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Yogini {
    /// 1 year, Moon.
    Mangala,
    /// 2 years, Sun.
    Pingala,
    /// 3 years, Jupiter.
    Dhanya,
    /// 4 years, Mars.
    Bhramari,
    /// 5 years, Mercury.
    Bhadrika,
    /// 6 years, Saturn.
    Ulka,
    /// 7 years, Venus.
    Siddha,
    /// 8 years, Rahu.
    Sankata,
}

impl Yogini {
    /// The eight yoginis in cycle order starting at Mangala.
    pub const ORDER: [Yogini; 8] = [
        Yogini::Mangala,
        Yogini::Pingala,
        Yogini::Dhanya,
        Yogini::Bhramari,
        Yogini::Bhadrika,
        Yogini::Ulka,
        Yogini::Siddha,
        Yogini::Sankata,
    ];

    /// Position in [`Yogini::ORDER`], 0 = Mangala.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Length of this yogini's mahadasha in years (1 … 8).
    pub const fn years(self) -> u32 {
        self as u32 + 1
    }

    /// The graha the yogini is identified with.
    pub const fn graha(self) -> DashaLord {
        match self {
            Yogini::Mangala => DashaLord::Moon,
            Yogini::Pingala => DashaLord::Sun,
            Yogini::Dhanya => DashaLord::Jupiter,
            Yogini::Bhramari => DashaLord::Mars,
            Yogini::Bhadrika => DashaLord::Mercury,
            Yogini::Ulka => DashaLord::Saturn,
            Yogini::Siddha => DashaLord::Venus,
            Yogini::Sankata => DashaLord::Rahu,
        }
    }

    /// Name used as the period label.
    pub const fn name(self) -> &'static str {
        match self {
            Yogini::Mangala => "Mangala",
            Yogini::Pingala => "Pingala",
            Yogini::Dhanya => "Dhanya",
            Yogini::Bhramari => "Bhramari",
            Yogini::Bhadrika => "Bhadrika",
            Yogini::Ulka => "Ulka",
            Yogini::Siddha => "Siddha",
            Yogini::Sankata => "Sankata",
        }
    }

    /// Yogini of a label produced by [`Yogini::name`].
    pub fn from_name(name: &str) -> Option<Yogini> {
        Yogini::ORDER.into_iter().find(|y| y.name() == name)
    }

    /// Starting yogini for a 0-based nakshatra index: `((index + 1) + 3) mod 8`, 0 → Sankata.
    pub const fn for_nakshatra(index: usize) -> Yogini {
        let r = (index + 1 + 3) % 8;
        if r == 0 {
            Yogini::Sankata
        } else {
            Yogini::ORDER[r - 1]
        }
    }
}

/// Mahadasha sequence from the running yogini at birth, repeating the 36-year cycle until
/// [`super::HORIZON_YEARS`] after birth. `moon_sidereal` in degrees selects the yogini and
/// the balance.
pub fn yogini(moon_sidereal: f64, birth: Instant, year_ns: i128) -> Vec<Period> {
    let nak = crate::nakshatra(moon_sidereal);
    let first = Yogini::for_nakshatra(nak.index);
    let elapsed_ns = fraction_ns(nak.fraction, first.years() as i128 * year_ns);
    let cycle: Vec<(&str, u32)> = Yogini::ORDER
        .iter()
        .map(|y| (y.name(), y.years()))
        .collect();
    lay_out_cycle(&cycle, first.index(), elapsed_ns, birth, year_ns, 1)
}

/// Antardashas of a Yogini period: the eight yoginis in cycle order from the period's own,
/// proportioned by their years over 36. Empty when the label is not a yogini.
pub fn yogini_sub(period: &Period) -> Vec<Period> {
    let Some(first) = Yogini::from_name(&period.label) else {
        return Vec::new();
    };
    let parts: Vec<(&str, u32)> = (0..8)
        .map(|i| Yogini::ORDER[(first.index() + i) % 8])
        .map(|y| (y.name(), y.years()))
        .collect();
    subdivide_weighted(period, &parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;
    use crate::NAKSHATRA_SPAN;

    #[test]
    fn cycle_totals_36_years_and_is_contiguous() {
        let total: u32 = Yogini::ORDER.iter().map(|y| y.years()).sum();
        assert_eq!(total, YOGINI_CYCLE_YEARS);
        let b = Instant::j2000();
        let v = yogini(0.0, b, YEAR_365_25_NS);
        assert!(v.len() >= 8);
        assert_eq!(v[7].end.ns_since(v[0].start), 36 * YEAR_365_25_NS);
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(v.last().unwrap().end >= b.add_ns(120 * YEAR_365_25_NS));
    }

    #[test]
    fn ashwini_starts_with_bhramari_and_anuradha_too() {
        // (1 + 3) mod 8 = 4 → Bhramari; (17 + 3) mod 8 = 4 → Bhramari.
        assert_eq!(Yogini::for_nakshatra(0), Yogini::Bhramari);
        assert_eq!(Yogini::for_nakshatra(16), Yogini::Bhramari);
        // (5 + 3) mod 8 = 0 → Sankata (Mrigashira); (6 + 3) mod 8 = 1 → Mangala (Ardra).
        assert_eq!(Yogini::for_nakshatra(4), Yogini::Sankata);
        assert_eq!(Yogini::for_nakshatra(5), Yogini::Mangala);
        // Revati (27): 30 mod 8 = 6 → Ulka.
        assert_eq!(Yogini::for_nakshatra(26), Yogini::Ulka);
        let v = yogini(0.0, Instant::j2000(), YEAR_365_25_NS);
        assert_eq!(v[0].label, "Bhramari");
        assert_eq!(v[1].label, "Bhadrika");
    }

    #[test]
    fn pyjhora_nakshatra_map_matches_the_formula() {
        // PyJHora yogini.py, 1-based nakshatra numbers per lord.
        let map: [(DashaLord, &[usize]); 8] = [
            (DashaLord::Moon, &[6, 14, 22]),
            (DashaLord::Sun, &[7, 15, 23]),
            (DashaLord::Jupiter, &[8, 16, 24]),
            (DashaLord::Mars, &[1, 9, 17, 25]),
            (DashaLord::Mercury, &[2, 10, 18, 26]),
            (DashaLord::Saturn, &[3, 11, 19, 27]),
            (DashaLord::Venus, &[4, 12, 20]),
            (DashaLord::Rahu, &[5, 13, 21]),
        ];
        for (lord, naks) in map {
            for n in naks {
                assert_eq!(Yogini::for_nakshatra(n - 1).graha(), lord, "nakshatra {n}");
            }
        }
    }

    #[test]
    fn balance_full_at_nakshatra_start_and_zero_at_its_end() {
        let b = Instant::j2000();
        let v = yogini(0.0, b, YEAR_365_25_NS);
        assert_eq!(v[0].start, b);
        assert_eq!(v[0].end.ns_since(b), 4 * YEAR_365_25_NS);
        let v = yogini(NAKSHATRA_SPAN - 1e-9, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Bhramari");
        assert!(v[0].end.ns_since(b) < 1_000_000_000, "balance should be ~0");
        assert!(v[0].start < b);
    }

    #[test]
    fn antardashas_are_proportional_from_the_running_yogini() {
        let b = Instant::j2000();
        let v = yogini(100.0, b, YEAR_365_25_NS);
        let s = yogini_sub(&v[1]);
        assert_eq!(s.len(), 8);
        assert_eq!(s[0].label, v[1].label);
        assert_eq!(s[0].start, v[1].start);
        assert_eq!(s[7].end, v[1].end);
        let y = Yogini::from_name(&v[1].label).unwrap();
        let total = v[1].end.ns_since(v[1].start);
        assert_eq!(
            s[0].end.ns_since(s[0].start),
            total * y.years() as i128 / 36
        );
        for w in s.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(s.iter().all(|p| p.level == 2));
    }
}
