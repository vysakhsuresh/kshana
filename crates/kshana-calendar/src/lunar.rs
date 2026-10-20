//! Lunar months: amanta (new Moon to new Moon) and purnimanta (full Moon to full
//! Moon), with the intercalary (adhika) and the rare suppressed (kshaya) month.
//!
//! Rules (Reingold & Dershowitz, *Calendrical Calculations*, 3rd ed., §20.3; Indian
//! Calendar Reform Committee Report, 1955, ch. V, which fixed the modern drik rules):
//!
//! * An amanta month runs from one new Moon (the true conjunction) to the next.
//! * It is named by the sidereal sign the Sun is in at the new Moon that begins it,
//!   plus one: a month beginning with the Sun in Meena is Chaitra, the month in which
//!   the Sun enters Mesha.
//! * A month with no sankranti in it (the Sun in the same sign at both new Moons) is
//!   adhika: it takes the name of the following month and is marked as the extra one.
//! * A month with two sankrantis is kshaya: it carries the name of the first sign
//!   entered, and the second month's name is not used that year.
//! * Purnimanta (North India): the bright fortnight keeps the amanta name; the dark
//!   fortnight takes the name of the next month, so the purnimanta month begins a
//!   fortnight earlier. An adhika month stays whole, with its own dark fortnight.
//!
//! Usage: amanta in Karnataka, Maharashtra, Gujarat, Andhra Pradesh and Telangana;
//! purnimanta in the North (Uttar Pradesh, Bihar, Madhya Pradesh, Rajasthan, Punjab,
//! Himachal, Jharkhand). Kerala, Tamil Nadu, Bengal and Odisha keep solar months for
//! civil use (see `solar`).

use crate::names::LUNAR_MONTHS;
use kshana_time::Instant;

/// One amanta lunar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LunarMonth {
    /// 1 = Chaitra … 12 = Phalguna.
    pub number: u8,
    /// The intercalary month (no sankranti inside).
    pub adhika: bool,
    /// For a kshaya month (two sankrantis inside), the number of the suppressed month.
    pub kshaya_skipped: Option<u8>,
    /// The new Moon that begins it.
    pub start: Instant,
    /// The new Moon that ends it.
    pub end: Instant,
}

impl LunarMonth {
    /// The month's name, e.g. "Jyeshtha".
    pub fn name(&self) -> &'static str {
        LUNAR_MONTHS[(self.number - 1) as usize]
    }
}

/// Name and flags of a month from the Sun's sidereal sign (0 = Mesha) at the new Moon
/// that begins it and at the one that ends it: `(number, adhika, kshaya_skipped)`.
pub fn month_from_signs(sign_at_start: u8, sign_at_end: u8) -> (u8, bool, Option<u8>) {
    let number = (sign_at_start + 1) % 12 + 1;
    let advance = (sign_at_end as i32 - sign_at_start as i32).rem_euclid(12);
    let adhika = advance == 0;
    let kshaya_skipped = if advance >= 2 { Some(number % 12 + 1) } else { None };
    (number, adhika, kshaya_skipped)
}

/// The amanta months bounded by consecutive new Moons, given each new Moon and the
/// Sun's sidereal sign at it.
pub fn months(new_moons: &[(Instant, u8)]) -> Vec<LunarMonth> {
    new_moons
        .windows(2)
        .map(|w| {
            let (number, adhika, kshaya_skipped) = month_from_signs(w[0].1, w[1].1);
            LunarMonth { number, adhika, kshaya_skipped, start: w[0].0, end: w[1].0 }
        })
        .collect()
}

/// A month's name under one reckoning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthName {
    /// 1 = Chaitra.
    pub number: u8,
    /// The intercalary month.
    pub adhika: bool,
}

impl MonthName {
    /// "Jyeshtha", or "Adhika Jyeshtha".
    pub fn label(&self) -> String {
        let n = LUNAR_MONTHS[(self.number - 1) as usize];
        if self.adhika {
            format!("Adhika {n}")
        } else {
            n.to_string()
        }
    }
}

/// The purnimanta month of a day that lies in amanta month `m` with tithi `tithi`
/// (1–30) at sunrise.
pub fn purnimanta(m: &LunarMonth, tithi: u8) -> MonthName {
    if m.adhika || tithi <= 15 {
        MonthName { number: m.number, adhika: m.adhika }
    } else {
        MonthName { number: m.number % 12 + 1, adhika: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaitra_begins_with_the_sun_in_meena() {
        assert_eq!(month_from_signs(11, 0), (1, false, None));
        assert_eq!(month_from_signs(0, 1), (2, false, None));
        assert_eq!(month_from_signs(10, 11), (12, false, None));
    }

    #[test]
    fn a_month_without_a_sankranti_is_adhika() {
        // Sun in Vrishabha at both ends: Adhika Jyeshtha (as in 2026).
        assert_eq!(month_from_signs(1, 1), (3, true, None));
    }

    #[test]
    fn a_month_with_two_sankrantis_is_kshaya() {
        // Sun in Vrischika at the start and Makara at the end: Margashirsha, with
        // Pausha suppressed (possible only near perihelion, when the Sun is fastest).
        assert_eq!(month_from_signs(7, 9), (9, false, Some(10)));
    }
}
