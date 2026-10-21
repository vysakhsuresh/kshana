//! Vimshottari dasha on exact instants.
//!
//! The 120-year cycle is laid out in integer nanoseconds, so a period boundary is an
//! exact [`Instant`] and no rounding accumulates across the cycle. The year length is
//! a parameter: 365.25 days is the common choice; 360-day (savana) and sidereal-year
//! variants are supported by passing a different `year_ns`.

use kshana_time::{Instant, NS_PER_DAY};

/// Lords of the nine dashas in cycle order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum DashaLord {
    Ketu,
    Venus,
    Sun,
    Moon,
    Mars,
    Rahu,
    Jupiter,
    Saturn,
    Mercury,
}

impl DashaLord {
    /// The nine lords in cycle order starting at Ketu.
    pub const ORDER: [DashaLord; 9] = [
        DashaLord::Ketu,
        DashaLord::Venus,
        DashaLord::Sun,
        DashaLord::Moon,
        DashaLord::Mars,
        DashaLord::Rahu,
        DashaLord::Jupiter,
        DashaLord::Saturn,
        DashaLord::Mercury,
    ];

    /// Length of this lord's mahadasha in years.
    pub const fn years(self) -> u32 {
        match self {
            DashaLord::Ketu => 7,
            DashaLord::Venus => 20,
            DashaLord::Sun => 6,
            DashaLord::Moon => 10,
            DashaLord::Mars => 7,
            DashaLord::Rahu => 18,
            DashaLord::Jupiter => 16,
            DashaLord::Saturn => 19,
            DashaLord::Mercury => 17,
        }
    }
}

/// One period at any level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashaPeriod {
    /// Lord of the period.
    pub lord: DashaLord,
    /// Exact start.
    pub start: Instant,
    /// Exact end (= the next period's start).
    pub end: Instant,
}

/// Nanoseconds in a 365.25-day year.
pub const YEAR_365_25_NS: i128 = NS_PER_DAY * 365 + NS_PER_DAY / 4;

/// Nanoseconds in the sidereal year Jagannatha Hora uses by default for dashas,
/// 365.256364 days (PyJHora `const.sidereal_year = 365.256364  # From JHora`,
/// `DHASA_YEAR_DURATION.JHORA_DEFAULT = TRUE_SIDEREAL_YEAR`). Pass it as `year_ns` to
/// reproduce JH's dasha dates; see docs/conventions.md.
pub const YEAR_SIDEREAL_NS: i128 = 31_558_149_849_600_000;

/// Mahadasha sequence for a whole 120-year cycle from the birth instant.
///
/// `moon_sidereal_lon` in degrees selects the starting lord and the balance.
pub fn vimshottari(moon_sidereal_lon: f64, birth: Instant, year_ns: i128) -> Vec<DashaPeriod> {
    let span = 360.0 / 27.0;
    let pos = moon_sidereal_lon.rem_euclid(360.0) / span;
    let nak = (pos.floor() as usize).min(26);
    let elapsed = pos - nak as f64;
    let first = nak % 9;
    let first_lord = DashaLord::ORDER[first];

    // Balance of the first dasha: the untraversed fraction of the nakshatra.
    // Computed as an exact integer from the fraction with nanosecond rounding once.
    let full_ns = first_lord.years() as i128 * year_ns;
    let elapsed_ns = (elapsed * full_ns as f64).round() as i128;
    let mut t = birth.add_ns(-elapsed_ns); // notional start of the running dasha
    let mut out = Vec::with_capacity(9);
    for i in 0..9 {
        let lord = DashaLord::ORDER[(first + i) % 9];
        let len = lord.years() as i128 * year_ns;
        let end = t.add_ns(len);
        out.push(DashaPeriod {
            lord,
            start: t,
            end,
        });
        t = end;
    }
    out
}

/// Sub-periods (antardasha) of a period: each lord in cycle order starting from the
/// period's own lord, proportioned by its years over 120.
pub fn subdivide(period: DashaPeriod) -> Vec<DashaPeriod> {
    let total = period.end.ns_since(period.start);
    let start_idx = DashaLord::ORDER
        .iter()
        .position(|l| *l == period.lord)
        .unwrap_or(0);
    let mut t = period.start;
    let mut out = Vec::with_capacity(9);
    let mut acc: i128 = 0;
    for i in 0..9 {
        let lord = DashaLord::ORDER[(start_idx + i) % 9];
        acc += lord.years() as i128;
        // cumulative split avoids drift: end_i = start + total * (sum years so far) / 120
        let end = if i == 8 {
            period.end
        } else {
            period.start.add_ns(total * acc / 120)
        };
        out.push(DashaPeriod {
            lord,
            start: t,
            end,
        });
        t = end;
    }
    out
}

/// The nested chain of periods containing `at`, from mahadasha down `depth` levels
/// (1 = maha, 2 = antar, 3 = pratyantar, 4 = sookshma, 5 = prana). Each level's bounds
/// are exact instants, so the prana period's start and end are microsecond-solvable
/// values the timing funnel can show directly.
pub fn vimshottari_levels(
    moon_sidereal_lon: f64,
    birth: Instant,
    year_ns: i128,
    at: Instant,
    depth: usize,
) -> Vec<DashaPeriod> {
    let mut chain = Vec::with_capacity(depth);
    let mut candidates = vimshottari(moon_sidereal_lon, birth, year_ns);
    for _ in 0..depth {
        let Some(p) = candidates
            .iter()
            .copied()
            .find(|p| p.start <= at && at < p.end)
        else {
            break;
        };
        chain.push(p);
        candidates = subdivide(p);
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_sums_to_120_years_exactly() {
        let birth = Instant::j2000();
        let v = vimshottari(243.2, birth, YEAR_365_25_NS);
        assert_eq!(v.len(), 9);
        let total = v[8].end.ns_since(v[0].start);
        assert_eq!(total, 120 * YEAR_365_25_NS);
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn mula_starts_with_ketu_and_birth_lies_inside_it() {
        let birth = Instant::j2000();
        let v = vimshottari(243.2, birth, YEAR_365_25_NS);
        assert_eq!(v[0].lord, DashaLord::Ketu);
        assert!(v[0].start <= birth && birth < v[0].end);
    }

    #[test]
    fn five_levels_nest_and_the_prana_period_lasts_hours_to_days() {
        let birth = Instant::j2000();
        let at = birth.add_ns(26 * YEAR_365_25_NS + 123_456_789_012);
        let chain = vimshottari_levels(243.2, birth, YEAR_365_25_NS, at, 5);
        assert_eq!(chain.len(), 5);
        for w in chain.windows(2) {
            assert!(w[0].start <= w[1].start && w[1].end <= w[0].end);
        }
        for p in &chain {
            assert!(p.start <= at && at < p.end);
        }
        let prana_h = chain[4].end.ns_since(chain[4].start) as f64 / 3.6e12;
        assert!(prana_h > 2.0 && prana_h < 120.0, "prana {prana_h} h");
    }

    #[test]
    fn antardashas_partition_the_mahadasha() {
        let birth = Instant::j2000();
        let v = vimshottari(100.0, birth, YEAR_365_25_NS);
        let subs = subdivide(v[1]);
        assert_eq!(subs[0].start, v[1].start);
        assert_eq!(subs[8].end, v[1].end);
        assert_eq!(subs[0].lord, v[1].lord);
        for w in subs.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
