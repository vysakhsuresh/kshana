//! Regional solar calendars. A solar month is the Sun's stay in one sidereal sign; the
//! regions differ only in which civil day (sunrise to sunrise) counts as the first of
//! the month when the sankranti falls part-way through a day. The four rules are those
//! of Sewell & Dikshit, *The Indian Calendar* (1896), §28, restated in Reingold &
//! Dershowitz, *Calendrical Calculations* (3rd ed.), §20.2:
//!
//! * Odisha: the month begins on the civil day in which the sankranti falls.
//! * Tamil Nadu: if the sankranti falls before sunset, that day; otherwise the next.
//! * Kerala (Malabar): if it falls before the end of the first three-fifths of the
//!   daytime (sunrise to sunset), that day; otherwise the next.
//! * Bengal: if it falls between sunrise and midnight, the next day; after midnight,
//!   the day after that. Midnight is taken as the middle of the night (half-way
//!   from sunset to the next sunrise); Bangladesh's revised calendar (fixed 14 April)
//!   is not modelled.

use crate::astro::Sankranti;
use crate::names::{BENGALI_MONTHS, MALAYALAM_MONTHS, ODIA_MONTHS, TAMIL_MONTHS};
use crate::CivilDay;

/// Which regional rule fixes the first day of a solar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarRule {
    /// Tamil Nadu: sankranti before sunset → same day.
    Tamil,
    /// Kerala: sankranti before 3/5 of the daytime → same day.
    Kerala,
    /// Bengal: before midnight → next day; after → the day after.
    Bengal,
    /// Odisha: the civil day of the sankranti.
    Odisha,
}

impl SolarRule {
    /// The month names of this calendar, indexed by the sign the Sun is in.
    pub fn months(self) -> &'static [&'static str; 12] {
        match self {
            SolarRule::Tamil => &TAMIL_MONTHS,
            SolarRule::Kerala => &MALAYALAM_MONTHS,
            SolarRule::Bengal => &BENGALI_MONTHS,
            SolarRule::Odisha => &ODIA_MONTHS,
        }
    }
}

/// Index into `days` of the civil day on which a solar month begins, given the
/// sankranti that opens it. `None` when the days do not cover it.
pub fn month_start_day(days: &[CivilDay], s: &Sankranti, rule: SolarRule) -> Option<usize> {
    let d = days.iter().position(|d| d.sunrise <= s.at && s.at < d.next_sunrise)?;
    let day = &days[d];
    let offset = match rule {
        SolarRule::Odisha => 0,
        SolarRule::Tamil => usize::from(s.at >= day.sunset),
        SolarRule::Kerala => {
            let three_fifths = day.sunrise.add_ns(day.sunset.ns_since(day.sunrise) * 3 / 5);
            usize::from(s.at >= three_fifths)
        }
        SolarRule::Bengal => {
            let midnight = day.sunset.add_ns(day.next_sunrise.ns_since(day.sunset) / 2);
            if s.at < midnight {
                1
            } else {
                2
            }
        }
    };
    let i = d + offset;
    (i < days.len()).then_some(i)
}

/// A solar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolarDate {
    /// The rule.
    pub rule: SolarRule,
    /// Sign of the month, 0 = Mesha.
    pub sign: u8,
    /// Day of the month, 1-based.
    pub day: u16,
    /// Year in the region's era: Kollam (Malayalam), Bangabda (Bengali); for Tamil the
    /// 0-based index of the sixty-year name; for Odia the CE year in which Mesha began.
    pub year: i64,
}

impl SolarDate {
    /// The month name.
    pub fn month_name(&self) -> &'static str {
        self.rule.months()[self.sign as usize % 12]
    }
}

/// The year in the region's reckoning of a day in solar month `sign`, Gregorian
/// `year-greg_month`.
///
/// * Kollam (Malayalam) era: the year turns at Chingam 1 (Sun entering Simha); ME =
///   CE − 824 from Chingam to December, CE − 825 from January to Karkidakam (ME 1201
///   began on 17 August 2025).
/// * Bangabda: the year turns at Boishakh 1; BS = CE − 593 from Boishakh to December
///   (BS 1433 began in April 2026).
/// * Tamil: the sixty-year name turns at Chithirai 1, Prabhava = 1987.
/// * Odia: the CE year in which the solar year (from Mesha) began.
pub fn solar_year(rule: SolarRule, sign: u8, year: i64, greg_month: u32) -> i64 {
    match rule {
        SolarRule::Kerala => {
            if (4..=8).contains(&sign) && greg_month >= 8 {
                year - 824
            } else {
                year - 825
            }
        }
        SolarRule::Bengal | SolarRule::Tamil | SolarRule::Odisha => {
            let start = if sign <= 8 && greg_month >= 4 { year } else { year - 1 };
            match rule {
                SolarRule::Bengal => start - 593,
                SolarRule::Tamil => (start - 1987).rem_euclid(60),
                _ => start,
            }
        }
    }
}
