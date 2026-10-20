//! Years: Shaka, Vikram and Kali eras of the lunisolar year, the sixty-year cycle by
//! the South and North Indian reckonings, the Indian National Calendar, seasons and
//! the Sun's half-years.

use crate::names::{NATIONAL_MONTHS, SAMVATSARAS};
use kshana_time::days_from_civil;

/// R.D. (Reingold & Dershowitz fixed day, R.D. 1 = 1 January 1 CE, proleptic Gregorian)
/// of a civil date. 1970-01-01 is R.D. 719 163.
pub fn fixed(y: i64, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d) + 719_163
}

/// The Kali Yuga epoch, R.D. −1 132 959 = 18 February 3102 BCE (Julian), the
/// `hindu-epoch` of *Calendrical Calculations* (3rd ed., §20.1).
pub const KALI_EPOCH_RD: f64 = -1_132_959.0;

/// Civil days in a mahayuga, Surya Siddhanta 1.37.
pub const SS_CIVIL_DAYS: f64 = 1_577_917_828.0;

/// Jupiter's sidereal revolutions in a mahayuga, Surya Siddhanta 1.29–30.
pub const SS_JUPITER_REVOLUTIONS: f64 = 364_220.0;

/// The lunisolar Shaka year of a day in amanta month `month` (1 = Chaitra) on Gregorian
/// date `year-greg_month`. The year turns at Chaitra Shukla Pratipada; months
/// Margashirsha to Phalguna that fall early in a Gregorian year still belong to the
/// year that began the previous spring. Shaka = CE − 78 from the turn.
pub fn shaka_year(month: u8, year: i64, greg_month: u32) -> i64 {
    if greg_month <= 6 && month >= 9 {
        year - 79
    } else {
        year - 78
    }
}

/// Vikram Samvat, Chaitradi (North India, year from Chaitra): Shaka + 135.
pub fn vikram_chaitradi(shaka: i64) -> i64 {
    shaka + 135
}

/// Vikram Samvat, Kartikadi (Gujarat, year from Kartika Shukla Pratipada, the day
/// after Diwali): one behind the Chaitradi count from Chaitra to Ashvina.
pub fn vikram_kartikadi(shaka: i64, month: u8) -> i64 {
    if month <= 7 {
        shaka + 134
    } else {
        shaka + 135
    }
}

/// Kali Yuga year: Shaka + 3179 (Kali 3179 elapsed at Shaka 0; *Calendrical
/// Calculations* §20.1).
pub fn kali_year(shaka: i64) -> i64 {
    shaka + 3179
}

/// The South Indian samvatsara (0-based index into [`SAMVATSARAS`]) of a lunisolar Shaka
/// year: the cycle is tied to the year itself, Prabhava = Shaka 1909 (CE 1987–88), as in
/// every Telugu, Kannada and Tamil almanac (Krodhi = 2024–25, Vishvavasu = 2025–26,
/// Parabhava = 2026–27).
pub fn samvatsara_south(shaka: i64) -> usize {
    (shaka + 11).rem_euclid(60) as usize
}

/// The North Indian (Barhaspatya) samvatsara current at R.D. `rd` (fractional): one
/// name per mean sidereal sign of Jupiter (Surya Siddhanta 1.55), counted from the Kali
/// epoch where the cycle stood at its 27th year. The formula is *Calendrical
/// Calculations* `jovian-year`: amod(27 + ⌊(d − epoch) / (P/12)⌋, 60), with P the Surya
/// Siddhanta mean sidereal period of Jupiter (≈ 4332.32 days). Because a Jupiter year
/// (≈ 361.03 days) is shorter than a solar one, about once in 85.8 years a name is
/// skipped, which is why the North and South cycles drift apart.
pub fn samvatsara_north(rd: f64) -> usize {
    let signs = (rd - KALI_EPOCH_RD) * SS_JUPITER_REVOLUTIONS * 12.0 / SS_CIVIL_DAYS;
    (27 + signs.floor() as i64 - 1).rem_euclid(60) as usize
}

/// Name of a samvatsara by index.
pub fn samvatsara_name(i: usize) -> &'static str {
    SAMVATSARAS[i % 60]
}

/// A date of the Indian National Calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NationalDate {
    /// Saka year.
    pub year: i64,
    /// 0 = Chaitra.
    pub month: u8,
    /// 1-based.
    pub day: u8,
}

impl NationalDate {
    /// Month name.
    pub fn month_name(&self) -> &'static str {
        NATIONAL_MONTHS[self.month as usize]
    }
}

fn gregorian_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// The Indian National Calendar date of a Gregorian date (Calendar Reform Committee,
/// 1955, adopted 22 March 1957): Chaitra 1 falls on 22 March (21 March in a Gregorian
/// leap year); Chaitra has 30 days (31 in a leap year), Vaishakha to Bhadra 31, Ashvina
/// to Phalguna 30; Saka year = CE − 78.
pub fn national(y: i64, m: u32, d: u32) -> NationalDate {
    let start = |yy: i64| fixed(yy, 3, if gregorian_leap(yy) { 21 } else { 22 });
    let f = fixed(y, m, d);
    let base = if f >= start(y) { y } else { y - 1 };
    let mut rest = f - start(base);
    let lens = |i: usize| -> i64 {
        match i {
            0 => {
                if gregorian_leap(base) {
                    31
                } else {
                    30
                }
            }
            1..=5 => 31,
            _ => 30,
        }
    };
    let mut month = 0usize;
    while month < 11 && rest >= lens(month) {
        rest -= lens(month);
        month += 1;
    }
    NationalDate { year: base - 78, month: month as u8, day: rest as u8 + 1 }
}

/// The season of a lunar month (1 = Chaitra): two months each, Vasanta = Chaitra and
/// Vaishakha (Taittiriya Samhita 4.4.11, Madhu–Madhava = Vasanta). 0-based into RITUS.
pub fn ritu_lunar(month: u8) -> usize {
    ((month.clamp(1, 12) - 1) / 2) as usize
}

/// The season by the Sun's tropical longitude (Drik Panchang's "Drik Ritu"): Vasanta
/// from 330° (Sun entering tropical Pisces) and each season 60°, so Shishira begins at
/// the winter solstice. 0-based into RITUS.
pub fn ritu_tropical(sun_tropical: f64) -> usize {
    (((sun_tropical + 30.0).rem_euclid(360.0)) / 60.0).floor() as usize % 6
}

/// Uttarayana by the solstices (the Sun's tropical longitude from 270° to 90°).
pub fn uttarayana_tropical(sun_tropical: f64) -> bool {
    let l = sun_tropical.rem_euclid(360.0);
    !(90.0..270.0).contains(&l)
}

/// Uttarayana as the almanacs count it: from Makara Sankranti to Karka Sankranti
/// (the Sun's sidereal sign Makara to Mithuna).
pub fn uttarayana_sidereal(sun_sign: u8) -> bool {
    matches!(sun_sign, 9 | 10 | 11 | 0 | 1 | 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rd_epoch_matches_calendrical_calculations() {
        assert_eq!(fixed(1970, 1, 1), 719_163);
        assert_eq!(fixed(1, 1, 1), 1);
    }

    #[test]
    fn south_samvatsara_names() {
        assert_eq!(samvatsara_name(samvatsara_south(1909)), "Prabhava");
        assert_eq!(samvatsara_name(samvatsara_south(1945)), "Shobhakrit"); // 2023–24
        assert_eq!(samvatsara_name(samvatsara_south(1946)), "Krodhi"); // 2024–25
        assert_eq!(samvatsara_name(samvatsara_south(1947)), "Vishvavasu"); // 2025–26
        assert_eq!(samvatsara_name(samvatsara_south(1948)), "Parabhava"); // 2026–27
    }

    #[test]
    fn north_samvatsara_names() {
        // At Chaitra Shukla Pratipada of each year (VS 2081, 2082, 2083), as printed
        // in North Indian panchangs: Kalayukti, Siddharthi, Raudra.
        assert_eq!(samvatsara_name(samvatsara_north(fixed(2024, 4, 9) as f64)), "Kalayukti");
        assert_eq!(samvatsara_name(samvatsara_north(fixed(2025, 3, 30) as f64)), "Siddharthi");
        assert_eq!(samvatsara_name(samvatsara_north(fixed(2026, 3, 19) as f64)), "Raudra");
    }

    #[test]
    fn eras() {
        assert_eq!(shaka_year(1, 2026, 3), 1948);
        assert_eq!(shaka_year(12, 2026, 3), 1947);
        assert_eq!(shaka_year(10, 2026, 1), 1947);
        assert_eq!(shaka_year(9, 2025, 12), 1947);
        assert_eq!(vikram_chaitradi(1948), 2083);
        assert_eq!(vikram_kartikadi(1947, 8), 2082); // from 22 Oct 2025
        assert_eq!(vikram_kartikadi(1948, 1), 2082);
        assert_eq!(kali_year(1948), 5127);
    }

    #[test]
    fn national_calendar() {
        assert_eq!(national(2026, 3, 22), NationalDate { year: 1948, month: 0, day: 1 });
        assert_eq!(national(2026, 1, 1), NationalDate { year: 1947, month: 9, day: 11 });
        assert_eq!(national(2024, 3, 21), NationalDate { year: 1946, month: 0, day: 1 });
        assert_eq!(national(2026, 3, 21), NationalDate { year: 1947, month: 11, day: 30 });
        // Independence Day: 24 Shravana (CRC table).
        assert_eq!(national(2025, 8, 15), NationalDate { year: 1947, month: 4, day: 24 });
    }

    #[test]
    fn seasons_and_half_years() {
        assert_eq!(ritu_lunar(1), 0);
        assert_eq!(ritu_lunar(12), 5);
        assert_eq!(ritu_tropical(0.0), 0);
        assert_eq!(ritu_tropical(275.0), 5);
        assert!(uttarayana_tropical(300.0) && !uttarayana_tropical(100.0));
        assert!(uttarayana_sidereal(9) && !uttarayana_sidereal(3));
    }
}
