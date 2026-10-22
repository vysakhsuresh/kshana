//! The Persian firdaria: a 75-year cycle of planetary periods by sect.

use kshana_time::{Instant, NS_PER_DAY};

/// One firdaria year: 365.25 days exactly (the Julian year of the medieval tables), so
/// every boundary is integer nanoseconds from the birth.
pub const FIRDARIA_YEAR_NS: i128 = 365 * NS_PER_DAY + NS_PER_DAY / 4;

/// Length of the whole cycle in years.
pub const FIRDARIA_CYCLE_YEARS: u32 = 75;

/// A firdaria lord: the seven traditional planets and the two lunar nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum FirdariaLord {
    Sun,
    Venus,
    Mercury,
    Moon,
    Saturn,
    Jupiter,
    Mars,
    NorthNode,
    SouthNode,
}

impl FirdariaLord {
    /// Years of the lord's major period.
    pub const fn years(self) -> u32 {
        match self {
            FirdariaLord::Sun => 10,
            FirdariaLord::Venus => 8,
            FirdariaLord::Mercury => 13,
            FirdariaLord::Moon => 9,
            FirdariaLord::Saturn => 11,
            FirdariaLord::Jupiter => 12,
            FirdariaLord::Mars => 7,
            FirdariaLord::NorthNode => 3,
            FirdariaLord::SouthNode => 2,
        }
    }

    /// Name in words.
    pub const fn name(self) -> &'static str {
        match self {
            FirdariaLord::Sun => "Sun",
            FirdariaLord::Venus => "Venus",
            FirdariaLord::Mercury => "Mercury",
            FirdariaLord::Moon => "Moon",
            FirdariaLord::Saturn => "Saturn",
            FirdariaLord::Jupiter => "Jupiter",
            FirdariaLord::Mars => "Mars",
            FirdariaLord::NorthNode => "North Node",
            FirdariaLord::SouthNode => "South Node",
        }
    }

    /// Name in the reader's language: the planets as the almanacs print them, the nodes as
    /// Rahu and Ketu.
    pub fn name_in(self, lang: kshana_rules::Lang) -> &'static str {
        match self {
            FirdariaLord::Sun => lang.pick(["Sun", "സൂര്യൻ", "सूर्य"]),
            FirdariaLord::Venus => lang.pick(["Venus", "ശുക്രൻ", "शुक्र"]),
            FirdariaLord::Mercury => lang.pick(["Mercury", "ബുധൻ", "बुध"]),
            FirdariaLord::Moon => lang.pick(["Moon", "ചന്ദ്രൻ", "चंद्र"]),
            FirdariaLord::Saturn => lang.pick(["Saturn", "ശനി", "शनि"]),
            FirdariaLord::Jupiter => lang.pick(["Jupiter", "വ്യാഴം", "गुरु"]),
            FirdariaLord::Mars => lang.pick(["Mars", "ചൊവ്വ", "मंगल"]),
            FirdariaLord::NorthNode => lang.pick(["North Node", "രാഹു (വടക്കേ പാതം)", "राहु (उत्तरी पात)"]),
            FirdariaLord::SouthNode => lang.pick(["South Node", "കേതു (തെക്കേ പാതം)", "केतु (दक्षिणी पात)"]),
        }
    }

    /// True for the two nodes, which have no sub-periods.
    pub const fn is_node(self) -> bool {
        matches!(self, FirdariaLord::NorthNode | FirdariaLord::SouthNode)
    }
}

/// Major periods of a day birth: Sun 10, Venus 8, Mercury 13, Moon 9, Saturn 11,
/// Jupiter 12, Mars 7, North Node 3, South Node 2 (75 years).
pub const DAY_SEQUENCE: [FirdariaLord; 9] = [
    FirdariaLord::Sun,
    FirdariaLord::Venus,
    FirdariaLord::Mercury,
    FirdariaLord::Moon,
    FirdariaLord::Saturn,
    FirdariaLord::Jupiter,
    FirdariaLord::Mars,
    FirdariaLord::NorthNode,
    FirdariaLord::SouthNode,
];

/// Major periods of a night birth: the same wheel entered at the Moon, with the nodes
/// last: Moon 9, Saturn 11, Jupiter 12, Mars 7, Sun 10, Venus 8, Mercury 13, North
/// Node 3, South Node 2. This is the Abu Ma'shar / al-Biruni / Robert Hand order; the
/// Bonatti (Zoller) variant that puts the nodes after Mars is not used.
pub const NIGHT_SEQUENCE: [FirdariaLord; 9] = [
    FirdariaLord::Moon,
    FirdariaLord::Saturn,
    FirdariaLord::Jupiter,
    FirdariaLord::Mars,
    FirdariaLord::Sun,
    FirdariaLord::Venus,
    FirdariaLord::Mercury,
    FirdariaLord::NorthNode,
    FirdariaLord::SouthNode,
];

/// The Chaldean order of the seven planets, slowest first.
pub const CHALDEAN_ORDER: [FirdariaLord; 7] = [
    FirdariaLord::Saturn,
    FirdariaLord::Jupiter,
    FirdariaLord::Mars,
    FirdariaLord::Sun,
    FirdariaLord::Venus,
    FirdariaLord::Mercury,
    FirdariaLord::Moon,
];

/// One firdaria period, major or sub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirdariaPeriod {
    /// Lord of the major period.
    pub lord: FirdariaLord,
    /// Lord of the sub-period, `None` for a major period or a node period.
    pub sub: Option<FirdariaLord>,
    /// Start, inclusive.
    pub start: Instant,
    /// End, exclusive; equals the next period's start.
    pub end: Instant,
}

/// The nine major periods of one 75-year cycle from `birth`.
///
/// Sect: `is_day` is the Sun above the horizon at birth (see `crate::is_day_at`).
/// Sources: Robert Hand, "Firdar, Alfridaria, or Alfridaries",
/// <https://www.arhatmedia.com/firdar2.htm>; <https://www.augurine.com/learn/firdaria>.
pub fn firdaria_major(birth: Instant, is_day: bool) -> Vec<FirdariaPeriod> {
    let seq = if is_day { DAY_SEQUENCE } else { NIGHT_SEQUENCE };
    let mut out = Vec::with_capacity(9);
    let mut years = 0i128;
    for lord in seq {
        let start = birth.add_ns(years * FIRDARIA_YEAR_NS);
        years += lord.years() as i128;
        out.push(FirdariaPeriod {
            lord,
            sub: None,
            start,
            end: birth.add_ns(years * FIRDARIA_YEAR_NS),
        });
    }
    out
}

/// One 75-year cycle from `birth` with the sub-periods expanded: each planetary major
/// period is divided into seven equal parts whose lords run in Chaldean order starting
/// from the major lord itself; the node periods are not divided. 51 contiguous entries.
pub fn firdaria(birth: Instant, is_day: bool) -> Vec<FirdariaPeriod> {
    let mut out = Vec::with_capacity(51);
    for major in firdaria_major(birth, is_day) {
        if major.lord.is_node() {
            out.push(major);
            continue;
        }
        let total = major.end.ns_since(major.start);
        let first = CHALDEAN_ORDER
            .iter()
            .position(|l| *l == major.lord)
            .unwrap_or(0);
        for k in 0..7i128 {
            out.push(FirdariaPeriod {
                lord: major.lord,
                sub: Some(CHALDEAN_ORDER[(first + k as usize) % 7]),
                start: major.start.add_ns(total * k / 7),
                end: major.start.add_ns(total * (k + 1) / 7),
            });
        }
    }
    out
}

/// The sub-period (or node period) in force at `at`, `None` before birth. The cycle is
/// taken to repeat after 75 years, as the Persian tables have it.
pub fn firdaria_at(birth: Instant, is_day: bool, at: Instant) -> Option<FirdariaPeriod> {
    if at < birth {
        return None;
    }
    let cycle_ns = FIRDARIA_CYCLE_YEARS as i128 * FIRDARIA_YEAR_NS;
    let cycles = at.ns_since(birth) / cycle_ns;
    firdaria(birth.add_ns(cycles * cycle_ns), is_day)
        .into_iter()
        .find(|p| p.start <= at && at < p.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cycle_is_exactly_seventy_five_contiguous_years() {
        let birth = Instant::j2000();
        for is_day in [true, false] {
            let majors = firdaria_major(birth, is_day);
            assert_eq!(majors.len(), 9);
            assert_eq!(
                majors[0].lord,
                if is_day {
                    FirdariaLord::Sun
                } else {
                    FirdariaLord::Moon
                }
            );
            let subs = firdaria(birth, is_day);
            assert_eq!(subs.len(), 51);
            assert_eq!(subs[0].start, birth);
            let total: i128 = subs.iter().map(|p| p.end.ns_since(p.start)).sum();
            assert_eq!(total, 75 * FIRDARIA_YEAR_NS);
            assert_eq!(subs[50].end, birth.add_ns(75 * FIRDARIA_YEAR_NS));
            for w in subs.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
            for p in &subs {
                assert_eq!(p.sub.is_none(), p.lord.is_node());
            }
        }
    }

    #[test]
    fn sub_periods_run_in_chaldean_order_from_the_lord() {
        let birth = Instant::j2000();
        let subs = firdaria(birth, true);
        let sun: Vec<FirdariaLord> = subs
            .iter()
            .filter(|p| p.lord == FirdariaLord::Sun)
            .map(|p| p.sub.unwrap())
            .collect();
        assert_eq!(
            sun,
            [
                FirdariaLord::Sun,
                FirdariaLord::Venus,
                FirdariaLord::Mercury,
                FirdariaLord::Moon,
                FirdariaLord::Saturn,
                FirdariaLord::Jupiter,
                FirdariaLord::Mars,
            ]
        );
        // age 36 in a day chart is the Moon's period (31–40)
        let at = birth.add_ns(36 * FIRDARIA_YEAR_NS + 1);
        let p = firdaria_at(birth, true, at).unwrap();
        assert_eq!(p.lord, FirdariaLord::Moon);
        assert!(p.start <= at && at < p.end);
        // the cycle repeats: age 111 is age 36 again
        let later = firdaria_at(birth, true, at.add_ns(75 * FIRDARIA_YEAR_NS)).unwrap();
        assert_eq!((later.lord, later.sub), (p.lord, p.sub));
        assert_eq!(later.start, p.start.add_ns(75 * FIRDARIA_YEAR_NS));
        assert!(firdaria_at(birth, true, birth.add_ns(-1)).is_none());
    }
}
