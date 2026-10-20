//! Civil (calendar) UTC and its exact conversion to and from [`Instant`].

use crate::instant::{Instant, NS_PER_SEC};
use crate::leap::{tai_minus_utc_at_utc_day, LEAP_SECONDS};

/// Days from 1970-01-01 to 1958-01-01 (the TAI epoch): 12 years, 3 of them leap.
pub(crate) const TAI_EPOCH_DAYS_FROM_1970: i64 = -4_383;

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
/// Exact for every date in the i64 range that matters here.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`].
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Why a civil UTC value could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UtcError {
    /// Month, day, hour, minute, second or nanosecond out of range.
    FieldOutOfRange,
    /// `second == 60` on a day that has no leap second at its end.
    NoLeapSecondHere,
    /// Dates before 1972-01-01 are not on the modern UTC scale.
    BeforeModernUtc,
}

/// A civil UTC date-time. `second` may be 60 during a positive leap second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Utc {
    /// Proleptic Gregorian year.
    pub year: i64,
    /// Month 1–12.
    pub month: u32,
    /// Day 1–31.
    pub day: u32,
    /// Hour 0–23.
    pub hour: u32,
    /// Minute 0–59.
    pub minute: u32,
    /// Second 0–60 (60 only inside a leap second).
    pub second: u32,
    /// Nanoseconds 0–999 999 999.
    pub nanos: u32,
}

impl Utc {
    /// Build and validate a civil UTC value.
    pub fn new(
        year: i64,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        nanos: u32,
    ) -> Result<Self, UtcError> {
        if !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 60
            || nanos > 999_999_999
        {
            return Err(UtcError::FieldOutOfRange);
        }
        let utc = Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanos,
        };
        if second == 60 {
            let today = utc.utc_day();
            let is_leap_end = hour == 23
                && minute == 59
                && tai_minus_utc_at_utc_day(today + 1) == tai_minus_utc_at_utc_day(today) + 1;
            if !is_leap_end {
                return Err(UtcError::NoLeapSecondHere);
            }
        }
        Ok(utc)
    }

    /// Days since 1970-01-01 of this civil date.
    pub fn utc_day(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Exact conversion to an [`Instant`] on the TAI scale.
    pub fn to_instant(&self) -> Result<Instant, UtcError> {
        let day = self.utc_day();
        if day < LEAP_SECONDS[0].0 {
            // Before 1972 there is no modern UTC. The civil time is read as Universal
            // Time, and TAI = UT + ΔT − 32.184 s with ΔT taken for the calendar year
            // (so the conversion inverts exactly). Good to about a second, which is
            // what a pre-1972 birth record can carry anyway.
            if self.second == 60 {
                return Err(UtcError::NoLeapSecondHere);
            }
            let ut_s = (day - TAI_EPOCH_DAYS_FROM_1970) as i128 * 86_400
                + self.hour as i128 * 3_600
                + self.minute as i128 * 60
                + self.second as i128;
            let ut_ns = ut_s * NS_PER_SEC + self.nanos as i128;
            return Ok(Instant::from_tai_ns(
                ut_ns + pre_1972_tai_minus_ut_ns(self.year),
            ));
        }
        // Seconds since the TAI epoch counting every civil day as 86 400 s …
        let civil_s = (day - TAI_EPOCH_DAYS_FROM_1970) as i128 * 86_400
            + self.hour as i128 * 3_600
            + self.minute as i128 * 60
            + self.second as i128;
        // … plus the accumulated leap seconds in force on this day.
        let tai_s = civil_s + tai_minus_utc_at_utc_day(day) as i128;
        Ok(Instant::from_tai_ns(
            tai_s * NS_PER_SEC + self.nanos as i128,
        ))
    }

    /// Exact conversion from an [`Instant`], including the 23:59:60 leap second.
    /// Instants before 1972 are rendered with TAI − UTC = 10 s, which is a label, not
    /// the historical rubber-second UTC.
    pub fn from_instant(i: Instant) -> Self {
        let tai_s = i.tai_ns().div_euclid(NS_PER_SEC);
        let nanos = i.tai_ns().rem_euclid(NS_PER_SEC) as u32;

        // Walk the table from the latest entry backwards and find the one in force.
        let n = LEAP_SECONDS.len();
        for idx in (0..n).rev() {
            let (eff_day, dtai) = LEAP_SECONDS[idx];
            let threshold = (eff_day - TAI_EPOCH_DAYS_FROM_1970) as i128 * 86_400;
            let civil_s = tai_s - dtai as i128;
            if civil_s >= threshold {
                // Inside the leap second that introduces the NEXT entry?
                if idx + 1 < n {
                    let (next_eff, _) = LEAP_SECONDS[idx + 1];
                    let next_threshold = (next_eff - TAI_EPOCH_DAYS_FROM_1970) as i128 * 86_400;
                    if civil_s >= next_threshold {
                        let (y, m, d) = civil_from_days(next_eff - 1);
                        return Self {
                            year: y,
                            month: m,
                            day: d,
                            hour: 23,
                            minute: 59,
                            second: 60,
                            nanos,
                        };
                    }
                }
                return Self::from_civil_seconds(civil_s, nanos);
            }
        }
        // Before 1972: invert the Universal-Time reading above, with ΔT for the
        // calendar year estimated from the instant itself.
        let rough = Self::from_civil_seconds(tai_s - 10, nanos);
        let ut_ns = i.tai_ns() - pre_1972_tai_minus_ut_ns(rough.year);
        Self::from_civil_seconds(
            ut_ns.div_euclid(NS_PER_SEC),
            ut_ns.rem_euclid(NS_PER_SEC) as u32,
        )
    }

    fn from_civil_seconds(civil_s: i128, nanos: u32) -> Self {
        let days = civil_s.div_euclid(86_400);
        let sod = civil_s.rem_euclid(86_400) as u32;
        let (y, m, d) = civil_from_days(days as i64 + TAI_EPOCH_DAYS_FROM_1970);
        Self {
            year: y,
            month: m,
            day: d,
            hour: sod / 3_600,
            minute: (sod % 3_600) / 60,
            second: sod % 60,
            nanos,
        }
    }
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

/// The last day on which a leap second has been ruled out: the IERS Bulletin C in
/// force (July 2026) announces none at the end of December 2026, so the earliest
/// possible insertion is 2027-06-30. Bump with each bulletin.
const LEAP_RULED_OUT_BEFORE: (i64, u32, u32) = (2027, 6, 30);

/// How well a civil (UTC) label of `i` can be printed today, in seconds: 0 within the
/// leap-second table and up to the day the Bulletin C in force has ruled a leap second
/// out to; one second from then on, since one insertion (or, with the Earth's recent
/// acceleration, a deletion) remains possible until the CGPM's 2035 deadline and the
/// rule after it is not yet written; one second before 1972, when UTC had no leap
/// seconds and the label comes from ΔT. The instant itself is exact in TAI either way.
pub fn civil_label_uncertainty_s(i: Instant) -> f64 {
    let u = Utc::from_instant(i);
    if u.utc_day() < LEAP_SECONDS[0].0 {
        return 1.0;
    }
    let (y, m, d) = LEAP_RULED_OUT_BEFORE;
    if u.utc_day() < days_from_civil(y, m, d) {
        return 0.0;
    }
    1.0
}

/// ΔT (TT − UT1) to use for an instant when no IERS UT1 − UTC is loaded.
///
/// Before 1972 this is exactly the ΔT that [`Utc::to_instant`] used to read the civil
/// time as Universal Time, so `UT1 = TT − ΔT` gives back the civil reading itself and the
/// sidereal time (hence the Ascendant) is that of the clock time entered. (Taking ΔT at
/// a different fraction of the year there moved UT1 off the entered time by up to about
/// a second in the 1910s, 15″ of sidereal time.) From 1972 on it is the ΔT table at the
/// exact decimal year.
pub fn delta_t_for_instant(i: Instant) -> crate::delta_t::DeltaT {
    let u = Utc::from_instant(i);
    if u.utc_day() < LEAP_SECONDS[0].0 {
        let tai_minus_ut = pre_1972_tai_minus_ut_ns(u.year) as f64 / 1e9;
        return crate::delta_t::DeltaT {
            seconds: tai_minus_ut + 32.184,
            sigma: crate::delta_t::delta_t_for_year(u.year as f64).sigma,
        };
    }
    let jan1 = days_from_civil(u.year, 1, 1);
    let len = (days_from_civil(u.year + 1, 1, 1) - jan1) as f64;
    let sod = u.hour as f64 * 3_600.0 + u.minute as f64 * 60.0 + u.second.min(59) as f64;
    let year = u.year as f64 + ((u.utc_day() - jan1) as f64 + sod / 86_400.0) / len;
    crate::delta_t::delta_t_for_year(year)
}

/// TAI − UT in nanoseconds for a calendar year before 1972: ΔT − 32.184 s, rounded to
/// the millisecond so the two directions of the conversion agree exactly.
fn pre_1972_tai_minus_ut_ns(year: i64) -> i128 {
    let dt = crate::delta_t::delta_t_for_year(year as f64).seconds;
    (((dt - 32.184) * 1_000.0).round() as i128) * 1_000_000
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_labels_far_ahead_carry_the_leap_second_floor() {
        use super::*;
        let now = Utc::new(2026, 10, 19, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let far = Utc::new(2033, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let beyond = Utc::new(2060, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let old = Utc::new(1950, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        assert_eq!(civil_label_uncertainty_s(now), 0.0);
        assert_eq!(civil_label_uncertainty_s(far), 1.0);
        assert_eq!(
            civil_label_uncertainty_s(beyond),
            civil_label_uncertainty_s(
                Utc::new(2036, 1, 1, 0, 0, 0, 0)
                    .unwrap()
                    .to_instant()
                    .unwrap()
            )
        );
        assert_eq!(civil_label_uncertainty_s(old), 1.0);
    }

    #[test]
    fn births_before_1972_round_trip_through_universal_time() {
        use super::*;
        let u = Utc::new(1950, 6, 15, 4, 30, 0, 123_456_789).unwrap();
        let i = u.to_instant().unwrap();
        let back = Utc::from_instant(i);
        assert_eq!(back, u);
        // 1950: ΔT ≈ 29 s, so TAI − UT ≈ −3 s: the instant lies a few seconds before
        // the naive civil reading.
        let naive = (u.utc_day() - TAI_EPOCH_DAYS_FROM_1970) as i128 * 86_400 * NS_PER_SEC
            + (4 * 3_600 + 30 * 60) as i128 * NS_PER_SEC
            + 123_456_789;
        let diff_s = (i.tai_ns() - naive) as f64 / 1e9;
        assert!(diff_s > -6.0 && diff_s < 0.0, "{diff_s}");
        assert!(Utc::new(1900, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .is_ok());
    }

    use super::*;

    #[test]
    fn before_1972_ut1_is_the_civil_reading_itself() {
        // A pre-1972 civil time is read as UT; UT1 = TT − ΔT must give it back exactly, or
        // the sidereal time (and the Ascendant) drifts off the clock time entered. Late in
        // the year is the worst case for a per-year ΔT (1915: ΔT grows ~1 s a year).
        for (y, m, d, h) in [(1915, 12, 28, 6), (1950, 11, 30, 22), (1969, 7, 20, 20)] {
            let u = Utc::new(y, m, d, h, 17, 0, 0).unwrap();
            let i = u.to_instant().unwrap();
            let ut1 = crate::delta_t::ut1_jd_from_delta_t(i, delta_t_for_instant(i));
            let want = 2_440_587.5
                + u.utc_day() as f64
                + (h as f64 * 3_600.0 + 17.0 * 60.0) / 86_400.0;
            let got = ut1.jd1 + ut1.jd2;
            assert!(
                ((got - want) * 86_400.0).abs() < 1e-3,
                "{y}-{m}-{d}: UT1 off the civil time by {} s",
                (got - want) * 86_400.0
            );
        }
    }

    #[test]
    fn hinnant_round_trip() {
        for z in [-800_000i64, -4_383, -1, 0, 1, 10_957, 19_723, 2_000_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1958, 1, 1), TAI_EPOCH_DAYS_FROM_1970);
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
    }

    #[test]
    fn j2000_from_utc_is_exact() {
        // J2000.0 = 2000-01-01T12:00:00 TT = 11:58:55.816 UTC (TAI − UTC was 32 s).
        let utc = Utc::new(2000, 1, 1, 11, 58, 55, 816_000_000).unwrap();
        assert_eq!(utc.to_instant().unwrap(), Instant::j2000());
        assert_eq!(Utc::from_instant(Instant::j2000()), utc);
    }

    #[test]
    fn leap_second_2016_is_a_real_second() {
        let before = Utc::new(2016, 12, 31, 23, 59, 59, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let leap = Utc::new(2016, 12, 31, 23, 59, 60, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let after = Utc::new(2017, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        assert_eq!(leap.ns_since(before), NS_PER_SEC);
        assert_eq!(after.ns_since(leap), NS_PER_SEC);
        // and it renders back as 23:59:60, not as 00:00:00
        let back = Utc::from_instant(leap.add_ns(250_000_000));
        assert_eq!(
            (back.hour, back.minute, back.second, back.nanos),
            (23, 59, 60, 250_000_000)
        );
        assert_eq!((back.year, back.month, back.day), (2016, 12, 31));
        assert_eq!(
            Utc::from_instant(after),
            Utc::new(2017, 1, 1, 0, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn second_60_rejected_where_no_leap() {
        assert_eq!(
            Utc::new(2016, 6, 30, 23, 59, 60, 0).unwrap_err(),
            UtcError::NoLeapSecondHere
        );
        assert_eq!(
            Utc::new(2016, 12, 31, 12, 0, 60, 0).unwrap_err(),
            UtcError::NoLeapSecondHere
        );
    }

    #[test]
    fn round_trip_many_instants() {
        let mut t = Utc::new(1972, 1, 1, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let step = 7 * 86_400 * NS_PER_SEC + 123_456_789;
        for _ in 0..3_000 {
            let u = Utc::from_instant(t);
            assert_eq!(u.to_instant().unwrap(), t, "round trip failed at {u:?}");
            t = t.add_ns(step);
        }
    }
}
