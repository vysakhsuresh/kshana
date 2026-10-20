//! Kalachakra dasha: nine-sign rows per nakshatra pada, paramayus 100 / 85 / 83 / 86.
//!
//! Sources. The row data are taken verbatim from the JHora-derived tables in PyJHora
//! `src/jhora/const.py` (`savya_stars_1_rasis`, `savya_stars_2_rasis`,
//! `apasavya_stars_1_rasis`, `apasavya_stars_2_rasis`, `kalachakra_dhasa_duration`,
//! `*_paramayush`) and cross-checked against the 24-sign savya chain given in
//! `chethandvg/vedic-astrology-toolkit` `dashas.py` (BPHS ch. 46 tables 42/43), which the
//! test `rows_follow_the_savya_chain` reproduces: every row is nine consecutive members of
//! the chain Ar Ta Ge Cn Le Vi Li Sc Sg Cp Aq Pi Sc Li Vi Cn Le Ge Ta Ar Pi Aq Cp Sg,
//! the apasavya rows of the same chain run backwards. Group membership follows BPHS
//! 46.58/65/71–72/77 and Jataka Parijata 17.102–105 as catalogued in `TheDaniel166/moira`
//! (`VEDIC_KALACHAKRA_SOURCE_RESEARCH`) and `Horace-Maxwell/Horosa-Web`:
//!
//! * Savya (15 nakshatras, in triads from Ashwini, Punarvasu, Hasta, Mula, P.Bhadrapada):
//!   the first and third of each triad use the Ashwini pattern (`Savya1`), the second the
//!   Bharani pattern (`Savya2`). Revati is in `Savya1`; JHora's printed table (P.V.R.
//!   Narasimha Rao, tables 44/45) puts Revati in `Savya2`, which the toolkit above
//!   documents as a misprint, so that variant is not followed.
//! * Apasavya (12, in triads from Rohini, Magha, Vishakha, Shravana): the first of each
//!   triad uses the Rohini pattern (`Apasavya1`), the second and third the Mrigashira
//!   pattern (`Apasavya2`).
//! * Years per sign (BPHS 46): Aries 7, Taurus 16, Gemini 9, Cancer 21, Leo 5, Virgo 9,
//!   Libra 16, Scorpio 7, Sagittarius 10, Capricorn 4, Aquarius 4, Pisces 10.
//! * Paramayus of padas 1–4: savya 100, 85, 83, 86; apasavya 86, 83, 85, 100.
//! * Deha and jeeva (BPHS 46.94–95): the first and the ninth sign of the natal row for a
//!   savya nakshatra; reversed (deha = ninth, jeeva = first) for apasavya.
//! * Balance (BPHS 46.93): the fraction of the Moon's *pada* traversed, times the row's
//!   paramayus, locates the running sign and how much of it has elapsed.
//! * After the ninth sign the natal row repeats (periods are laid out to
//!   [`super::HORIZON_YEARS`]); the paramayus is itself the lifespan ceiling, so the
//!   second cycle is of theoretical interest only.
//! * Antardashas: the natal row rotated to start from the period's own sign, proportioned
//!   by the sign years over the paramayus (JHora's "rotate the natal row with its
//!   unchanged denominator").

use kshana_time::Instant;

use super::{lay_out_cycle, subdivide_weighted, Period, SIGN_NAMES};

/// Years of each sign from Aries (BPHS 46).
pub const KALACHAKRA_YEARS: [u32; 12] = [7, 16, 9, 21, 5, 9, 16, 7, 10, 4, 4, 10];

/// The four row patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// Savya, Ashwini pattern.
    Savya1,
    /// Savya, Bharani pattern.
    Savya2,
    /// Apasavya, Rohini pattern.
    Apasavya1,
    /// Apasavya, Mrigashira pattern.
    Apasavya2,
}

impl Group {
    /// True for the savya (clockwise) groups.
    pub const fn is_savya(self) -> bool {
        matches!(self, Group::Savya1 | Group::Savya2)
    }

    /// Index into [`PATTERN_ROWS`] and [`PATTERN_PARAMAYUS`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Group of a 0-based nakshatra index.
    pub const fn of_nakshatra(index: usize) -> Group {
        GROUP_OF_NAKSHATRA[index % 27]
    }
}

/// Group of each nakshatra from Ashwini.
pub const GROUP_OF_NAKSHATRA: [Group; 27] = [
    Group::Savya1,    // Ashwini
    Group::Savya2,    // Bharani
    Group::Savya1,    // Krittika
    Group::Apasavya1, // Rohini
    Group::Apasavya2, // Mrigashira
    Group::Apasavya2, // Ardra
    Group::Savya1,    // Punarvasu
    Group::Savya2,    // Pushya
    Group::Savya1,    // Ashlesha
    Group::Apasavya1, // Magha
    Group::Apasavya2, // Purva Phalguni
    Group::Apasavya2, // Uttara Phalguni
    Group::Savya1,    // Hasta
    Group::Savya2,    // Chitra
    Group::Savya1,    // Swati
    Group::Apasavya1, // Vishakha
    Group::Apasavya2, // Anuradha
    Group::Apasavya2, // Jyeshtha
    Group::Savya1,    // Mula
    Group::Savya2,    // Purva Ashadha
    Group::Savya1,    // Uttara Ashadha
    Group::Apasavya1, // Shravana
    Group::Apasavya2, // Dhanishta
    Group::Apasavya2, // Shatabhisha
    Group::Savya1,    // Purva Bhadrapada
    Group::Savya2,    // Uttara Bhadrapada
    Group::Savya1,    // Revati (JHora: Savya2; see module docs)
];

/// Dasha order of the nine signs (0 = Aries) for each group and pada 1–4.
pub const PATTERN_ROWS: [[[u8; 9]; 4]; 4] = [
    // Savya1 (Ashwini pattern): 100, 85, 83, 86
    [
        [0, 1, 2, 3, 4, 5, 6, 7, 8],
        [9, 10, 11, 7, 6, 5, 3, 4, 2],
        [1, 0, 11, 10, 9, 8, 0, 1, 2],
        [3, 4, 5, 6, 7, 8, 9, 10, 11],
    ],
    // Savya2 (Bharani pattern): 100, 85, 83, 86
    [
        [7, 6, 5, 3, 4, 2, 1, 0, 11],
        [10, 9, 8, 0, 1, 2, 3, 4, 5],
        [6, 7, 8, 9, 10, 11, 7, 6, 5],
        [3, 4, 2, 1, 0, 11, 10, 9, 8],
    ],
    // Apasavya1 (Rohini pattern): 86, 83, 85, 100
    [
        [8, 9, 10, 11, 0, 1, 2, 4, 3],
        [5, 6, 7, 11, 10, 9, 8, 7, 6],
        [5, 4, 3, 2, 1, 0, 8, 9, 10],
        [11, 0, 1, 2, 4, 3, 5, 6, 7],
    ],
    // Apasavya2 (Mrigashira pattern): 86, 83, 85, 100
    [
        [11, 10, 9, 8, 7, 6, 5, 4, 3],
        [2, 1, 0, 8, 9, 10, 11, 0, 1],
        [2, 4, 3, 5, 6, 7, 11, 10, 9],
        [8, 7, 6, 5, 4, 3, 2, 1, 0],
    ],
];

/// Paramayus (years of the whole row) for each group and pada 1–4.
pub const PATTERN_PARAMAYUS: [[u32; 4]; 4] = [
    [100, 85, 83, 86],
    [100, 85, 83, 86],
    [86, 83, 85, 100],
    [86, 83, 85, 100],
];

const fn build_table() -> [[[u8; 9]; 4]; 27] {
    let mut t = [[[0u8; 9]; 4]; 27];
    let mut n = 0;
    while n < 27 {
        t[n] = PATTERN_ROWS[GROUP_OF_NAKSHATRA[n].index()];
        n += 1;
    }
    t
}

/// The full 108-row table: `KALACHAKRA_TABLE[nakshatra][pada - 1]` is the dasha order of
/// the nine signs (0 = Aries).
pub const KALACHAKRA_TABLE: [[[u8; 9]; 4]; 27] = build_table();

/// Dasha order of the nine signs for a 0-based nakshatra index and pada 1–4.
pub fn row(nakshatra: usize, pada: u8) -> &'static [u8; 9] {
    &KALACHAKRA_TABLE[nakshatra % 27][(pada.clamp(1, 4) - 1) as usize]
}

/// Paramayus for a 0-based nakshatra index and pada 1–4.
pub fn paramayus(nakshatra: usize, pada: u8) -> u32 {
    PATTERN_PARAMAYUS[Group::of_nakshatra(nakshatra).index()][(pada.clamp(1, 4) - 1) as usize]
}

/// `(deha, jeeva)` sign indices for a 0-based nakshatra index and pada 1–4.
pub fn deha_jeeva(nakshatra: usize, pada: u8) -> (usize, usize) {
    let r = row(nakshatra, pada);
    let (first, last) = (r[0] as usize, r[8] as usize);
    if Group::of_nakshatra(nakshatra).is_savya() {
        (first, last)
    } else {
        (last, first)
    }
}

/// The natal row of a Moon longitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KalachakraRow {
    /// 0-based nakshatra index.
    pub nakshatra: usize,
    /// Pada 1–4.
    pub pada: u8,
    /// Row pattern.
    pub group: Group,
    /// Dasha order of the nine signs, 0 = Aries.
    pub signs: [u8; 9],
    /// Years of the whole row.
    pub paramayus: u32,
    /// Deha sign.
    pub deha: usize,
    /// Jeeva sign.
    pub jeeva: usize,
}

impl KalachakraRow {
    /// Years of the sign at `position` (0–8) of the row.
    pub fn years_at(&self, position: usize) -> u32 {
        KALACHAKRA_YEARS[self.signs[position % 9] as usize]
    }
}

/// Natal row for a sidereal Moon longitude in degrees.
pub fn kalachakra_row(moon_sidereal: f64) -> KalachakraRow {
    let nak = crate::nakshatra(moon_sidereal);
    let group = Group::of_nakshatra(nak.index);
    let (deha, jeeva) = deha_jeeva(nak.index, nak.pada);
    KalachakraRow {
        nakshatra: nak.index,
        pada: nak.pada,
        group,
        signs: *row(nak.index, nak.pada),
        paramayus: paramayus(nak.index, nak.pada),
        deha,
        jeeva,
    }
}

/// A Kalachakra dasha with its natal row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KalachakraDasha {
    /// The natal row.
    pub row: KalachakraRow,
    /// Position (0–8) in the row of `periods[0]`; `periods[k]` is at `(first + k) % 9`.
    pub first_position: usize,
    /// Mahadashas from the running sign at birth.
    pub periods: Vec<Period>,
}

/// Mahadashas with the natal row. The running sign and its balance come from the
/// fraction of the Moon's pada traversed times the paramayus, rounded to a nanosecond
/// once; later boundaries are exact.
pub fn kalachakra_detail(moon_sidereal: f64, birth: Instant, year_ns: i128) -> KalachakraDasha {
    let nak = crate::nakshatra(moon_sidereal);
    let row = kalachakra_row(moon_sidereal);
    // Fraction of the pada traversed, from the nakshatra fraction.
    let pada_fraction = (nak.fraction * 4.0 - (nak.pada as f64 - 1.0)).clamp(0.0, 1.0);
    let total_ns = row.paramayus as i128 * year_ns;
    // Rounded once; kept strictly inside the row so the running sign always exists.
    let completed_ns = if total_ns > 0 {
        super::fraction_ns(pada_fraction, total_ns).min(total_ns - 1)
    } else {
        0
    };
    let mut cum_ns: i128 = 0;
    let mut first = 0usize;
    let mut elapsed_ns = 0i128;
    for (i, s) in row.signs.iter().enumerate() {
        let len = KALACHAKRA_YEARS[*s as usize] as i128 * year_ns;
        if completed_ns < cum_ns + len {
            first = i;
            elapsed_ns = completed_ns - cum_ns;
            break;
        }
        cum_ns += len;
    }
    let cycle: Vec<(&str, u32)> = row
        .signs
        .iter()
        .map(|s| (SIGN_NAMES[*s as usize], KALACHAKRA_YEARS[*s as usize]))
        .collect();
    let periods = lay_out_cycle(&cycle, first, elapsed_ns, birth, year_ns, 1);
    KalachakraDasha {
        row,
        first_position: first,
        periods,
    }
}

/// Mahadashas from the running sign at birth; labels are sign names.
pub fn kalachakra(moon_sidereal: f64, birth: Instant, year_ns: i128) -> Vec<Period> {
    kalachakra_detail(moon_sidereal, birth, year_ns).periods
}

/// Antardashas of a Kalachakra period at `position` (0–8) of its natal `row`: the row
/// rotated to start from that position, proportioned by sign years over the paramayus.
pub fn kalachakra_sub(period: &Period, row: &KalachakraRow, position: usize) -> Vec<Period> {
    let parts: Vec<(&str, u32)> = (0..9)
        .map(|i| row.signs[(position + i) % 9] as usize)
        .map(|s| (SIGN_NAMES[s], KALACHAKRA_YEARS[s]))
        .collect();
    subdivide_weighted(period, &parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;
    use crate::NAKSHATRA_SPAN;

    const PADA: f64 = NAKSHATRA_SPAN / 4.0;

    fn row_years(r: &[u8; 9]) -> u32 {
        r.iter().map(|s| KALACHAKRA_YEARS[*s as usize]).sum()
    }

    #[test]
    fn years_sum_to_118_and_rows_sum_to_their_paramayus() {
        assert_eq!(KALACHAKRA_YEARS.iter().sum::<u32>(), 118);
        for (g, rows) in PATTERN_ROWS.iter().enumerate() {
            for (p, r) in rows.iter().enumerate() {
                assert_eq!(
                    row_years(r),
                    PATTERN_PARAMAYUS[g][p],
                    "group {g} pada {}",
                    p + 1
                );
            }
        }
        for n in 0..27 {
            for p in 1..=4 {
                assert_eq!(row_years(row(n, p)), paramayus(n, p));
            }
        }
        // Savya 15, apasavya 12.
        let savya = GROUP_OF_NAKSHATRA.iter().filter(|g| g.is_savya()).count();
        assert_eq!(savya, 15);
    }

    #[test]
    fn rows_follow_the_savya_chain() {
        // BPHS ch. 46 chain (chethandvg/vedic-astrology-toolkit): the savya rows are
        // consecutive 9-sign windows stepping by 9; the Bharani pattern is offset by 36;
        // the apasavya patterns are the same windows of the reversed chain.
        let chain: [u8; 24] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 7, 6, 5, 3, 4, 2, 1, 0, 11, 10, 9, 8,
        ];
        let mut rev = chain;
        rev.reverse();
        let window = |src: &[u8; 24], offset: usize| -> [u8; 9] {
            std::array::from_fn(|k| src[(offset + k) % 24])
        };
        let patterns: [(&str, &[u8; 24], usize); 4] = [
            ("Savya1", &chain, 0),
            ("Savya2", &chain, 36),
            ("Apasavya1", &rev, 0),
            ("Apasavya2", &rev, 36),
        ];
        for ((name, src, base), rows) in patterns.iter().zip(PATTERN_ROWS.iter()) {
            for (p, row) in rows.iter().enumerate() {
                assert_eq!(*row, window(src, base + 9 * p), "{name} pada {}", p + 1);
            }
        }
    }

    #[test]
    fn table_spot_checks_against_the_sources() {
        // Ashwini 1: Ar Ta Ge Cn Le Vi Li Sc Sg, 100, deha Aries, jeeva Sagittarius.
        assert_eq!(row(0, 1), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(paramayus(0, 1), 100);
        assert_eq!(deha_jeeva(0, 1), (0, 8));
        // Ashwini 2: Cp Aq Pi Sc Li Vi Cn Le Ge, 85.
        assert_eq!(row(0, 2), &[9, 10, 11, 7, 6, 5, 3, 4, 2]);
        assert_eq!(paramayus(0, 2), 85);
        // Rohini padas 1, 3, 4: 86, 85, 100 (modernastrologer / PyJHora).
        assert_eq!(paramayus(3, 1), 86);
        assert_eq!(paramayus(3, 3), 85);
        assert_eq!(paramayus(3, 4), 100);
        // Rohini 1 runs Sg … Cn: apasavya, so deha Cancer, jeeva Sagittarius.
        assert_eq!(row(3, 1), &[8, 9, 10, 11, 0, 1, 2, 4, 3]);
        assert_eq!(deha_jeeva(3, 1), (3, 8));
        // Mrigashira 1: Pi Aq Cp Sg Sc Li Vi Le Cn, 86.
        assert_eq!(row(4, 1), &[11, 10, 9, 8, 7, 6, 5, 4, 3]);
        // Mula 4: 86 (modernastrologer); Bharani 1 = Sc Li Vi Cn Le Ge Ta Ar Pi.
        assert_eq!(paramayus(18, 4), 86);
        assert_eq!(row(1, 1), &[7, 6, 5, 3, 4, 2, 1, 0, 11]);
        // Krittika and Revati share Ashwini's rows; U.Bhadrapada shares Bharani's.
        assert_eq!(KALACHAKRA_TABLE[2], KALACHAKRA_TABLE[0]);
        assert_eq!(KALACHAKRA_TABLE[26], KALACHAKRA_TABLE[0]);
        assert_eq!(KALACHAKRA_TABLE[25], KALACHAKRA_TABLE[1]);
    }

    #[test]
    fn cycle_totals_paramayus_and_is_contiguous() {
        let b = Instant::j2000();
        for lon in [0.0, 3.5, 50.0, 123.4, 200.0, 333.3, 359.9] {
            let d = kalachakra_detail(lon, b, YEAR_365_25_NS);
            let v = &d.periods;
            assert!(v.len() >= 9);
            assert_eq!(
                v[8].end.ns_since(v[0].start),
                d.row.paramayus as i128 * YEAR_365_25_NS
            );
            for w in v.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
            assert!(v[0].start <= b && b < v[0].end);
            assert_eq!(
                v[0].label,
                SIGN_NAMES[d.row.signs[d.first_position] as usize]
            );
            assert!(v.last().unwrap().end >= b.add_ns(120 * YEAR_365_25_NS));
        }
    }

    #[test]
    fn balance_full_at_pada_start_and_zero_at_pada_end() {
        let b = Instant::j2000();
        let v = kalachakra(0.0, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Aries");
        assert_eq!(v[0].start, b);
        assert_eq!(v[0].end.ns_since(b), 7 * YEAR_365_25_NS);
        // Start of Ashwini pada 2: Capricorn, full 4 years.
        let v = kalachakra(PADA, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Capricorn");
        assert_eq!(v[0].end.ns_since(b), 4 * YEAR_365_25_NS);
        // End of Ashwini pada 1: the ninth sign (Sagittarius) with ~zero balance.
        let v = kalachakra(PADA - 1e-9, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Sagittarius");
        assert!(v[0].end.ns_since(b) < 1_000_000_000);
        // 8/100 through Ashwini pada 1: one year of Taurus elapsed, 15 remain.
        let v = kalachakra(PADA * 0.08, b, YEAR_365_25_NS);
        assert_eq!(v[0].label, "Taurus");
        assert!(v[0].end.ns_since(b).abs_diff(15 * YEAR_365_25_NS) < 1_000_000);
    }

    #[test]
    fn antardashas_rotate_the_natal_row() {
        let b = Instant::j2000();
        let d = kalachakra_detail(0.0, b, YEAR_365_25_NS);
        let s = kalachakra_sub(&d.periods[1], &d.row, (d.first_position + 1) % 9);
        assert_eq!(s.len(), 9);
        assert_eq!(s[0].label, "Taurus");
        assert_eq!(s[8].label, "Aries");
        assert_eq!(s[0].start, d.periods[1].start);
        assert_eq!(s[8].end, d.periods[1].end);
        let total = d.periods[1].end.ns_since(d.periods[1].start);
        assert_eq!(s[0].end.ns_since(s[0].start), total * 16 / 100);
        for w in s.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
