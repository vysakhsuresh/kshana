//! Further dasha systems beyond Vimshottari: Yogini, Ashtottari, Kalachakra, Chara (Jaimini).
//!
//! Every system here lays its periods out the way [`crate::dasha`] does: the balance at
//! birth is rounded to a nanosecond exactly once, and every later boundary is the notional
//! start of the running period plus an integer number of years times `year_ns`, so no
//! drift accumulates across a cycle. Sub-periods are cut by cumulative integer splits.
//!
//! The common [`Period`] carries a text label (a graha, a yogini or a sign) so one type
//! serves the nakshatra-based and the sign-based systems alike.

use kshana_time::Instant;

use crate::DashaLord;

pub mod ashtottari;
pub mod chara;
pub mod conditional;
pub mod drig;
pub mod kalachakra;
pub mod narayana;
pub mod rasi_strength;
pub mod shoola;
pub mod sthira;
pub mod yogini;

pub use ashtottari::{
    applies as ashtottari_applies, ashtottari, ashtottari_sub, lord_of_nakshatra,
    years_of as ashtottari_years_of, ASHTOTTARI_CYCLE_YEARS, ASHTOTTARI_GROUPS, ASHTOTTARI_ORDER,
    ASHTOTTARI_YEARS,
};
pub use chara::{
    chara, chara_direction, chara_lord, chara_sequence, chara_sub, chara_years, is_odd_footed,
    Direction, EXALTATION_SIGN,
};
pub use conditional::{
    applies as conditional_applies, conditional, conditional_sub, AntarRule, ChartFacts,
    Conditional,
};
pub use kalachakra::{
    deha_jeeva, kalachakra, kalachakra_detail, kalachakra_row, kalachakra_sub, paramayus, row,
    Group, KalachakraDasha, KalachakraRow, GROUP_OF_NAKSHATRA, KALACHAKRA_TABLE, KALACHAKRA_YEARS,
    PATTERN_PARAMAYUS, PATTERN_ROWS,
};
pub use drig::{drig, drig_antar_order, drig_sequence, drig_sub};
pub use narayana::{
    narayana, narayana_antar_order, narayana_seed, narayana_sequence, narayana_sub,
    narayana_years,
};
pub use rasi_strength::{brahma, sign_lord as jaimini_sign_lord, stronger_rasi, RasiChart};
pub use shoola::{shoola, shoola_seed, shoola_sub, SHOOLA_YEARS};
pub use sthira::{sthira, sthira_seed, sthira_sub, sthira_years};
pub use yogini::{yogini, yogini_sub, Yogini, YOGINI_CYCLE_YEARS};

/// One period of any system at any level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Period {
    /// Lord, yogini or sign name (see [`SIGN_NAMES`] and [`graha_name`]).
    pub label: String,
    /// Exact start.
    pub start: Instant,
    /// Exact end (= the next period's start).
    pub end: Instant,
    /// 1 = mahadasha, 2 = antardasha, 3 = pratyantardasha, …
    pub level: u8,
}

/// The period containing `at` (`start <= at < end`), if any.
pub fn period_at(periods: &[Period], at: Instant) -> Option<&Period> {
    periods.iter().find(|p| p.start <= at && at < p.end)
}

/// Years after birth up to which the cyclic systems lay out periods.
pub const HORIZON_YEARS: i128 = 120;

/// Names of the twelve signs from Aries.
pub const SIGN_NAMES: [&str; 12] = [
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
    "Aquarius",
    "Pisces",
];

/// Sign index (0 = Aries) of a name from [`SIGN_NAMES`].
pub fn sign_index(name: &str) -> Option<usize> {
    SIGN_NAMES.iter().position(|s| *s == name)
}

/// English name of a graha, as used in period labels.
pub const fn graha_name(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Ketu => "Ketu",
        DashaLord::Venus => "Venus",
        DashaLord::Sun => "Sun",
        DashaLord::Moon => "Moon",
        DashaLord::Mars => "Mars",
        DashaLord::Rahu => "Rahu",
        DashaLord::Jupiter => "Jupiter",
        DashaLord::Saturn => "Saturn",
        DashaLord::Mercury => "Mercury",
    }
}

/// Graha of a name produced by [`graha_name`].
pub fn graha_from_name(name: &str) -> Option<DashaLord> {
    DashaLord::ORDER
        .into_iter()
        .find(|g| graha_name(*g) == name)
}

/// Lay out a repeating cycle of `(label, years)` from the running member `first`, of which
/// `elapsed_ns` had passed at `birth`, until the horizon of [`HORIZON_YEARS`] after birth
/// is covered. Boundaries are exact: the notional start plus cumulative years × `year_ns`.
/// Returns an empty list when the cycle is empty or `year_ns` is not positive.
pub(crate) fn lay_out_cycle(
    cycle: &[(&str, u32)],
    first: usize,
    elapsed_ns: i128,
    birth: Instant,
    year_ns: i128,
    level: u8,
) -> Vec<Period> {
    let n = cycle.len();
    if n == 0 || year_ns <= 0 || cycle.iter().all(|(_, y)| *y == 0) {
        return Vec::new();
    }
    let t0 = birth.add_ns(-elapsed_ns);
    let horizon = birth.add_ns(HORIZON_YEARS * year_ns);
    let mut out = Vec::with_capacity(n * 2);
    let mut t = t0;
    let mut cum: i128 = 0;
    let mut k = 0usize;
    loop {
        let (label, years) = cycle[(first + k) % n];
        cum += years as i128;
        let end = t0.add_ns(cum * year_ns);
        out.push(Period {
            label: label.to_string(),
            start: t,
            end,
            level,
        });
        t = end;
        k += 1;
        if end >= horizon {
            break;
        }
    }
    out
}

/// Cut `period` into sub-periods proportioned by integer weights, by cumulative splits
/// (`end_i = start + total × Σweights_i / Σweights`), the last ending exactly at
/// `period.end`. The sub-periods are one level deeper than `period`.
pub fn subdivide_weighted(period: &Period, parts: &[(&str, u32)]) -> Vec<Period> {
    let total = period.end.ns_since(period.start);
    let sum: i128 = parts.iter().map(|(_, w)| *w as i128).sum();
    if parts.is_empty() || sum == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(parts.len());
    let mut t = period.start;
    let mut acc: i128 = 0;
    for (i, (label, w)) in parts.iter().enumerate() {
        acc += *w as i128;
        let end = if i + 1 == parts.len() {
            period.end
        } else {
            period.start.add_ns(total * acc / sum)
        };
        out.push(Period {
            label: label.to_string(),
            start: t,
            end,
            level: period.level.saturating_add(1),
        });
        t = end;
    }
    out
}

/// Round a fraction of a span of nanoseconds to an exact nanosecond count, once.
pub(crate) fn fraction_ns(fraction: f64, span_ns: i128) -> i128 {
    (fraction.clamp(0.0, 1.0) * span_ns as f64).round() as i128
}

/// Sign index (0 = Aries) of a sidereal longitude in degrees.
pub(crate) fn sign_of(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;

    #[test]
    fn period_at_finds_the_half_open_container() {
        let b = Instant::j2000();
        let cycle = [("A", 1), ("B", 2)];
        let v = lay_out_cycle(&cycle, 0, 0, b, YEAR_365_25_NS, 1);
        assert_eq!(period_at(&v, b).map(|p| p.label.as_str()), Some("A"));
        assert_eq!(period_at(&v, v[0].end).map(|p| p.label.as_str()), Some("B"));
        assert!(period_at(&v, b.add_ns(-1)).is_none());
        let last = v.last().unwrap();
        assert!(last.end >= b.add_ns(HORIZON_YEARS * YEAR_365_25_NS));
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn weighted_subdivision_partitions_exactly() {
        let b = Instant::j2000();
        let p = Period {
            label: "X".into(),
            start: b,
            end: b.add_ns(36 * YEAR_365_25_NS),
            level: 1,
        };
        let parts = [("a", 1), ("b", 2), ("c", 3)];
        let s = subdivide_weighted(&p, &parts);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].start, p.start);
        assert_eq!(s[2].end, p.end);
        assert_eq!(s[0].end.ns_since(s[0].start), 6 * YEAR_365_25_NS);
        assert_eq!(s[1].end.ns_since(s[1].start), 12 * YEAR_365_25_NS);
        assert_eq!(s[2].end.ns_since(s[2].start), 18 * YEAR_365_25_NS);
        assert!(s.iter().all(|q| q.level == 2));
    }

    #[test]
    fn names_round_trip() {
        for (i, n) in SIGN_NAMES.iter().enumerate() {
            assert_eq!(sign_index(n), Some(i));
        }
        for g in DashaLord::ORDER {
            assert_eq!(graha_from_name(graha_name(g)), Some(g));
        }
        assert_eq!(sign_of(359.999), 11);
        assert_eq!(sign_of(-1.0), 11);
        assert_eq!(sign_of(30.0), 1);
    }
}
