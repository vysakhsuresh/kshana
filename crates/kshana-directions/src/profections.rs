//! Annual and monthly profections between exact solar returns.

use crate::{require, sign_add, DirectionsError};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_events::{solar_return, MICROSECOND_NS};
use kshana_time::{Instant, Utc};
use kshana_western::{Planet, Sign};

/// The profected year and month in force at an instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profection {
    /// Completed years of life: solar returns since birth.
    pub age: u32,
    /// The annual profected sign: the natal Ascendant's sign counted forward by `age`.
    pub year_sign: Sign,
    /// Traditional domicile ruler of `year_sign`, the lord of the year.
    pub year_lord: Planet,
    /// The exact solar return that opened this year (the birth itself for age 0).
    pub year_start: Instant,
    /// The exact solar return that closes it.
    pub year_end: Instant,
    /// Month of the profected year, 0–11.
    pub month: u8,
    /// The monthly profected sign: `year_sign` counted forward by `month`.
    pub month_sign: Sign,
    /// Traditional domicile ruler of `month_sign`.
    pub month_lord: Planet,
    /// Start of the month: `year_start + month × span / 12`.
    pub month_start: Instant,
    /// End of the month: `year_start + (month + 1) × span / 12`.
    pub month_end: Instant,
}

/// Annual and monthly profections at `at`.
///
/// Conventions (Hellenistic, whole-sign):
/// * The year turns at the exact solar return (the Sun's return to its natal tropical
///   longitude), found with `kshana_events::solar_return`; the first year starts at
///   the birth itself. The annual sign is the natal Ascendant's sign advanced one sign
///   per completed year; its domicile ruler is the lord of the year. See
///   <https://theastrologypodcast.com/2018/04/26/annual-profections-a-basic-time-lord-technique/>.
/// * The twelve months are equal twelfths of the span between the two solar returns
///   that bound the year (about 30.44 days each, not calendar months). The first month
///   is the annual sign itself and each month advances one sign; see
///   <https://www.augurine.com/learn/monthly-profections>.
pub fn profections(spk: &Spk, natal: &Chart, at: Instant) -> Result<Profection, DirectionsError> {
    let birth = natal.instant;
    if at < birth {
        return Err(DirectionsError::BeforeBirth);
    }
    let natal_sun = require(natal, "Sun")?;
    let birth_year = Utc::from_instant(birth).year;
    let sr = |year: i64| -> Result<Instant, DirectionsError> {
        if year == birth_year {
            return Ok(birth);
        }
        solar_return(spk, natal_sun, birth, year, MICROSECOND_NS)?
            .ok_or(DirectionsError::NoSolarReturn(year))
    };

    let mut year = Utc::from_instant(at).year;
    let mut start = sr(year)?;
    if at < start {
        year -= 1;
        start = sr(year)?;
    }
    let mut end = sr(year + 1)?;
    // a birthday in the first days of January can have its next return late in December
    if at >= end {
        year += 1;
        start = end;
        end = sr(year + 1)?;
    }

    let age = (year - birth_year) as u32;
    let year_sign = sign_add(Sign::of(natal.cusps[0]), age as usize);
    let span = end.ns_since(start);
    let month = ((at.ns_since(start) * 12) / span).clamp(0, 11) as u8;
    let month_sign = sign_add(year_sign, month as usize);
    Ok(Profection {
        age,
        year_sign,
        year_lord: year_sign.traditional_ruler(),
        year_start: start,
        year_end: end,
        month,
        month_sign,
        month_lord: month_sign.traditional_ruler(),
        month_start: start.add_ns(span * month as i128 / 12),
        month_end: start.add_ns(span * (month as i128 + 1) / 12),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use kshana_time::NS_PER_DAY;

    #[test]
    fn year_and_month_bracket_the_instant() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let at = fixture::utc(2026, 10, 19, 12, 0);
        let p = profections(&spk, &natal, at).unwrap();
        assert_eq!(p.age, 35);
        assert_eq!(p.year_sign, sign_add(Sign::of(natal.cusps[0]), 35));
        assert_eq!(p.year_lord, p.year_sign.traditional_ruler());
        assert!(p.year_start <= at && at < p.year_end);
        assert!(p.month_start <= at && at < p.month_end);
        assert!(p.year_start <= p.month_start && p.month_end <= p.year_end);
        let span_days = p.year_end.ns_since(p.year_start) as f64 / NS_PER_DAY as f64;
        assert!((364.0..=367.0).contains(&span_days), "{span_days}");
        // ten days before the 2026 birthday is the last month of the year
        assert_eq!(p.month, 11);
        assert_eq!(p.month_sign, sign_add(p.year_sign, 11));
        assert_eq!(p.month_end, p.year_end);
        // the solar return really is a return of the Sun
        let sun = kshana_sky::apparent(&spk, kshana_ephem::Body::Sun, p.year_start).unwrap();
        assert!(kshana_solver::wrap180(sun.longitude - natal.bodies[0].tropical).abs() < 1e-6);
    }

    #[test]
    fn the_first_year_starts_at_birth() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let p = profections(&spk, &natal, natal.instant.add_ns(NS_PER_DAY)).unwrap();
        assert_eq!(p.age, 0);
        assert_eq!(p.year_start, natal.instant);
        assert_eq!(p.month, 0);
        assert_eq!(p.year_sign, Sign::of(natal.cusps[0]));
        assert_eq!(
            profections(&spk, &natal, natal.instant.add_ns(-1)),
            Err(DirectionsError::BeforeBirth)
        );
    }
}
