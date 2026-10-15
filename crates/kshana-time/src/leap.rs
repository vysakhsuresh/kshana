//! The leap-second table.
//!
//! Each entry is `(utc_day, tai_minus_utc)`: from 00:00:00 UTC of `utc_day` (days since
//! 1970-01-01) onward, TAI − UTC equals `tai_minus_utc` whole seconds.
//!
//! Source: IERS Bulletin C. No leap second has been announced after 2016-12-31. When
//! IERS announces one, add a row here and bump the data version in the provenance stamp.

/// `(days since 1970-01-01, TAI − UTC in seconds)`, ascending.
pub const LEAP_SECONDS: &[(i64, i32)] = &[
    (730, 10),   // 1972-01-01
    (912, 11),   // 1972-07-01
    (1096, 12),  // 1973-01-01
    (1461, 13),  // 1974-01-01
    (1826, 14),  // 1975-01-01
    (2191, 15),  // 1976-01-01
    (2557, 16),  // 1977-01-01
    (2922, 17),  // 1978-01-01
    (3287, 18),  // 1979-01-01
    (3652, 19),  // 1980-01-01
    (4199, 20),  // 1981-07-01
    (4564, 21),  // 1982-07-01
    (4929, 22),  // 1983-07-01
    (5660, 23),  // 1985-07-01
    (6574, 24),  // 1988-01-01
    (7305, 25),  // 1990-01-01
    (7670, 26),  // 1991-01-01
    (8217, 27),  // 1992-07-01
    (8582, 28),  // 1993-07-01
    (8947, 29),  // 1994-07-01
    (9496, 30),  // 1996-01-01
    (10043, 31), // 1997-07-01
    (10592, 32), // 1999-01-01
    (13149, 33), // 2006-01-01
    (14245, 34), // 2009-01-01
    (15522, 35), // 2012-07-01
    (16617, 36), // 2015-07-01
    (17167, 37), // 2017-01-01
];

/// TAI − UTC in force on the given UTC day (days since 1970-01-01).
/// Days before 1972 return 10, a label for "not modern UTC".
pub fn tai_minus_utc_at_utc_day(utc_day: i64) -> i32 {
    let mut dtai = 10;
    for &(day, value) in LEAP_SECONDS {
        if utc_day >= day {
            dtai = value;
        } else {
            break;
        }
    }
    dtai
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civil::days_from_civil;

    #[test]
    fn table_dates_match_calendar() {
        let expect = [
            (1972, 1, 1),
            (1972, 7, 1),
            (1973, 1, 1),
            (1974, 1, 1),
            (1975, 1, 1),
            (1976, 1, 1),
            (1977, 1, 1),
            (1978, 1, 1),
            (1979, 1, 1),
            (1980, 1, 1),
            (1981, 7, 1),
            (1982, 7, 1),
            (1983, 7, 1),
            (1985, 7, 1),
            (1988, 1, 1),
            (1990, 1, 1),
            (1991, 1, 1),
            (1992, 7, 1),
            (1993, 7, 1),
            (1994, 7, 1),
            (1996, 1, 1),
            (1997, 7, 1),
            (1999, 1, 1),
            (2006, 1, 1),
            (2009, 1, 1),
            (2012, 7, 1),
            (2015, 7, 1),
            (2017, 1, 1),
        ];
        assert_eq!(expect.len(), LEAP_SECONDS.len());
        for (i, (y, m, d)) in expect.iter().enumerate() {
            assert_eq!(LEAP_SECONDS[i].0, days_from_civil(*y, *m, *d), "row {i}");
            assert_eq!(LEAP_SECONDS[i].1, 10 + i as i32);
        }
    }

    #[test]
    fn lookup() {
        assert_eq!(tai_minus_utc_at_utc_day(days_from_civil(2000, 1, 1)), 32);
        assert_eq!(tai_minus_utc_at_utc_day(days_from_civil(2016, 12, 31)), 36);
        assert_eq!(tai_minus_utc_at_utc_day(days_from_civil(2017, 1, 1)), 37);
        assert_eq!(tai_minus_utc_at_utc_day(days_from_civil(2026, 10, 15)), 37);
    }
}
