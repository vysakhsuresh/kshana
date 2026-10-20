//! Zodiacal releasing from the Lots of Fortune and Spirit (Vettius Valens, *Anthology*
//! IV, in the Schmidt translation as reconstructed by Project Hindsight).

use crate::{sign_index, SIGNS};
use kshana_time::{Instant, NS_PER_DAY};
use kshana_western::Sign;

/// The releasing year: 360 days exactly (Valens's ideal year of twelve 30-day months),
/// so every level's unit is an integer number of nanoseconds. Source: Ep. 192 of The
/// Astrology Podcast, "Zodiacal Releasing: An Ancient Timing Technique",
/// <https://theastrologypodcast.com/transcripts/ep-192-transcript-zodiacal-releasing-an-ancient-timing-technique/>.
pub const ZR_YEAR_NS: i128 = 360 * NS_PER_DAY;

/// Deepest level computed: level 4, whose unit is five hours.
pub const ZR_MAX_LEVEL: u8 = 4;

/// Years of one full circuit of the twelve signs (the sum of the sign periods), which
/// at level 2 is the 211 months (17 years 7 months) after which the bond is loosed.
pub const ZR_CIRCUIT_YEARS: u32 = 211;

/// Years of a sign's period: the planetary years of its ruler (Aries 15, Taurus 8,
/// Gemini 20, Cancer 25, Leo 19, Virgo 20, Libra 8, Scorpio 15, Sagittarius 12,
/// Capricorn 27, Aquarius 30, Pisces 12).
pub const fn zr_years(sign: Sign) -> u32 {
    match sign {
        Sign::Aries => 15,
        Sign::Taurus => 8,
        Sign::Gemini => 20,
        Sign::Cancer => 25,
        Sign::Leo => 19,
        Sign::Virgo => 20,
        Sign::Libra => 8,
        Sign::Scorpio => 15,
        Sign::Sagittarius => 12,
        Sign::Capricorn => 27,
        Sign::Aquarius => 30,
        Sign::Pisces => 12,
    }
}

/// The unit of one sign-year at `level` (1–4): a 360-day year at level 1, a 30-day
/// month at level 2, 2.5 days at level 3 and 5 hours at level 4, each a twelfth of the
/// one above. A sign's period at any level is its years times this unit.
pub fn zr_unit_ns(level: u8) -> i128 {
    ZR_YEAR_NS / 12i128.pow(level.clamp(1, ZR_MAX_LEVEL) as u32 - 1)
}

/// The Lots of Fortune and Spirit, tropical longitudes in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lots {
    /// Lot of Fortune.
    pub fortune: f64,
    /// Lot of Spirit.
    pub spirit: f64,
}

/// The lots by sect. Fortune: day `Asc + Moon − Sun`, night `Asc + Sun − Moon`;
/// Spirit is the reverse (day `Asc + Sun − Moon`, night `Asc + Moon − Sun`).
pub fn lots(asc: f64, sun: f64, moon: f64, is_day: bool) -> Lots {
    let day_fortune = (asc + moon - sun).rem_euclid(360.0);
    let day_spirit = (asc + sun - moon).rem_euclid(360.0);
    if is_day {
        Lots {
            fortune: day_fortune,
            spirit: day_spirit,
        }
    } else {
        Lots {
            fortune: day_spirit,
            spirit: day_fortune,
        }
    }
}

/// One releasing period at some level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZrPeriod {
    /// Level 1 (general) to 4.
    pub level: u8,
    /// The sign released to.
    pub sign: Sign,
    /// Start, inclusive.
    pub start: Instant,
    /// End, exclusive; a sub-period is cut off at its parent's end.
    pub end: Instant,
    /// True when this period begins with the loosing of the bond: the sequence had
    /// completed the twelve signs and jumped to the sign opposite the parent instead of
    /// returning to it.
    pub loosing: bool,
    /// True when the sign is angular to the reference lot: the 1st, 4th, 7th or 10th
    /// sign from it.
    pub peak: bool,
}

/// True when `sign` is the 1st, 4th, 7th or 10th sign counted from `from`.
pub fn is_peak(sign: Sign, from: Sign) -> bool {
    (sign_index(sign) + 12 - sign_index(from)) % 3 == 0
}

/// Zodiacal releasing from the lot in `lot_sign`, to `levels` deep (1–4), for every
/// period starting before `until`, with peaks counted from the same lot.
///
/// Valens counts peak periods from the Lot of Fortune even when releasing from Spirit;
/// use [`zodiacal_releasing_from`] with the Fortune sign for that.
pub fn zodiacal_releasing(
    lot_sign: Sign,
    birth: Instant,
    levels: u8,
    until: Instant,
) -> Vec<ZrPeriod> {
    zodiacal_releasing_from(lot_sign, lot_sign, birth, levels, until)
}

/// Zodiacal releasing from the lot in `lot_sign` with peaks counted from `peak_from`
/// (the Lot of Fortune's sign in Valens), to `levels` deep (1–4), for every period
/// starting before `until`. Periods come in pre-order: each parent before its
/// sub-periods, so the list is sorted by start and, within a start, by level.
///
/// Rules (Valens IV.4–11 as read by Schmidt and Brennan):
/// * Level 1 runs through the signs in order from the lot's sign, each for its years of
///   360 days; past the twelfth sign it continues round (nobody lives the 211 years).
/// * Each sub-level divides its parent in the same proportions from the parent's own
///   sign: a sign's period is its years in months at level 2, in 2.5-day units at
///   level 3 and in 5-hour units at level 4. The last sub-period is cut off at the
///   parent's end.
/// * Loosing of the bond (levels 2 and below): when the sub-sequence has run through
///   all twelve signs and would return to the parent's sign, it jumps instead to the
///   sign opposite the parent and continues in order from there; this happens once per
///   parent period. Source:
///   <https://theastrologypodcast.com/transcripts/ep-192-transcript-zodiacal-releasing-an-ancient-timing-technique/>.
/// * Peak periods: the sign is the 1st, 10th, 7th or 4th from `peak_from`; see
///   <https://kerykeion.net/content/learn-astrology/predictive-zodiacal-releasing-peak-periods>.
pub fn zodiacal_releasing_from(
    lot_sign: Sign,
    peak_from: Sign,
    birth: Instant,
    levels: u8,
    until: Instant,
) -> Vec<ZrPeriod> {
    let walk = Walk {
        levels: levels.clamp(1, ZR_MAX_LEVEL),
        peak_from,
        until,
    };
    let mut out = Vec::new();
    fill(&mut out, &walk, 1, lot_sign, birth, None);
    out
}

/// The fixed parameters of one releasing.
struct Walk {
    levels: u8,
    peak_from: Sign,
    until: Instant,
}

/// Emit the periods of `level` inside `[start, parent_end)` starting from `parent_sign`,
/// and recurse into each.
fn fill(
    out: &mut Vec<ZrPeriod>,
    walk: &Walk,
    level: u8,
    parent_sign: Sign,
    start: Instant,
    parent_end: Option<Instant>,
) {
    let unit = zr_unit_ns(level);
    let mut t = start;
    let mut idx = sign_index(parent_sign);
    let mut count = 0usize;
    let mut loosed = false;
    while t < walk.until && parent_end.map_or(true, |pe| t < pe) {
        let mut loosing = false;
        if level >= 2 && count == 12 && !loosed {
            idx = (sign_index(parent_sign) + 6) % 12;
            loosed = true;
            loosing = true;
        }
        let sign = SIGNS[idx];
        let mut end = t.add_ns(zr_years(sign) as i128 * unit);
        if let Some(pe) = parent_end {
            if end > pe {
                end = pe;
            }
        }
        out.push(ZrPeriod {
            level,
            sign,
            start: t,
            end,
            loosing,
            peak: is_peak(sign, walk.peak_from),
        });
        if level < walk.levels {
            fill(out, walk, level + 1, sign, t, Some(end));
        }
        t = end;
        idx = (idx + 1) % 12;
        count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn units_are_twelfths_and_the_circuit_is_211_years() {
        assert_eq!(zr_unit_ns(1), 360 * NS_PER_DAY);
        assert_eq!(zr_unit_ns(2), 30 * NS_PER_DAY);
        assert_eq!(zr_unit_ns(3), 5 * NS_PER_DAY / 2);
        assert_eq!(zr_unit_ns(4), 5 * 3_600 * kshana_time::NS_PER_SEC);
        let circuit: u32 = SIGNS.iter().map(|s| zr_years(*s)).sum();
        assert_eq!(circuit, ZR_CIRCUIT_YEARS);
    }

    #[test]
    fn lots_swap_by_sect() {
        let d = lots(100.0, 10.0, 40.0, true);
        assert_eq!((d.fortune, d.spirit), (130.0, 70.0));
        let n = lots(100.0, 10.0, 40.0, false);
        assert_eq!((n.fortune, n.spirit), (70.0, 130.0));
    }

    #[test]
    fn level_one_is_contiguous_from_the_lot() {
        let birth = Instant::j2000();
        let until = birth.add_ns(100 * ZR_YEAR_NS);
        let all = zodiacal_releasing(Sign::Leo, birth, 2, until);
        let l1: Vec<&ZrPeriod> = all.iter().filter(|p| p.level == 1).collect();
        assert_eq!(l1[0].sign, Sign::Leo);
        assert_eq!(l1[0].start, birth);
        assert_eq!(l1[0].end, birth.add_ns(19 * ZR_YEAR_NS));
        assert_eq!(l1[1].sign, Sign::Virgo);
        for w in l1.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert!(l1.iter().all(|p| p.start < until && !p.loosing));
        assert!(l1[0].peak);
        assert!(!l1[1].peak);
        assert!(l1.iter().find(|p| p.sign == Sign::Scorpio).unwrap().peak);
        // pre-order: sorted by start, parent before child
        assert!(all.windows(2).all(|w| w[0].start <= w[1].start));
        assert_eq!(all[1].level, 2);
        assert_eq!(all[1].start, all[0].start);
    }

    #[test]
    fn sub_periods_fill_their_parent_and_loose_the_bond() {
        let birth = Instant::j2000();
        let until = birth.add_ns(27 * ZR_YEAR_NS);
        let all = zodiacal_releasing(Sign::Capricorn, birth, 3, until);
        let l1 = all[0];
        assert_eq!((l1.level, l1.sign), (1, Sign::Capricorn));
        let l2: Vec<&ZrPeriod> = all
            .iter()
            .filter(|p| p.level == 2 && p.start < l1.end)
            .collect();
        assert_eq!(l2[0].sign, Sign::Capricorn);
        assert_eq!(l2[0].end.ns_since(l2[0].start), 27 * zr_unit_ns(2));
        for w in l2.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert_eq!(l2[l2.len() - 1].end, l1.end);
        // the 13th sub-period, 211 months in, is the loosing to Cancer, not Capricorn
        let month = zr_unit_ns(2);
        assert_eq!(l2[12].start, birth.add_ns(211 * month));
        assert_eq!(l2[12].sign, Sign::Cancer);
        assert!(l2[12].loosing);
        assert!(l2[12].peak);
        assert_eq!(l2[13].sign, Sign::Leo);
        assert_eq!(l2.iter().filter(|p| p.loosing).count(), 1);
        // the sequence can reach Capricorn again without a second jump
        assert_eq!(l2[l2.len() - 1].sign, Sign::Capricorn);
        assert!(!l2[l2.len() - 1].loosing);
        // level 3 fills each level-2 period exactly
        for p2 in &l2 {
            let l3: Vec<&ZrPeriod> = all
                .iter()
                .filter(|p| p.level == 3 && p.start >= p2.start && p.start < p2.end)
                .collect();
            assert_eq!(l3[0].sign, p2.sign);
            assert_eq!(l3[0].start, p2.start);
            assert_eq!(l3[l3.len() - 1].end, p2.end);
            for w in l3.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
        }
        // peaks from Fortune when releasing from Spirit
        let from_spirit = zodiacal_releasing_from(Sign::Capricorn, Sign::Aries, birth, 1, until);
        assert!(from_spirit[0].peak);
    }

    #[test]
    fn fixture_lots_release_from_their_signs() {
        let Some(spk) = fixture::kernel() else { return };
        let natal = fixture::natal(&spk);
        let is_day = crate::is_day_at(&spk, None, &fixture::observer(), natal.instant).unwrap();
        let l = lots(
            natal.cusps[0],
            natal.bodies[0].tropical,
            natal.bodies[1].tropical,
            is_day,
        );
        let until = fixture::utc(2030, 1, 1, 0, 0);
        let zr = zodiacal_releasing(Sign::of(l.fortune), natal.instant, 4, until);
        assert_eq!(zr[0].sign, Sign::of(l.fortune));
        assert!(zr.iter().any(|p| p.level == 4));
        let l1: Vec<&ZrPeriod> = zr.iter().filter(|p| p.level == 1).collect();
        for w in l1.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}
