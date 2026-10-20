//! Gulika, Mandi and the other upagrahas as kalas and as longitudes.
//!
//! Rule (BPHS ch. 3, verses 66–67, as the Sanskrit is read on
//! enjoylearningsanskrit.com and in PVR Narasimha Rao's *Vedic Astrology: An Integrated
//! Approach*): the day from sunrise to sunset is divided into eight parts; the first
//! belongs to the weekday lord and the following parts to the following weekday lords
//! in weekday order (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn), and the eighth
//! part is lordless. The night from sunset to the next sunrise is divided the same way,
//! its first part belonging to the lord of the fifth weekday counted from the day's.
//! Kala is the Sun's part, Mrityu Mars's, Ardhaprahara Mercury's, Yamaghantaka
//! (Yamakantaka) Jupiter's and Gulika Saturn's; Mandi is Saturn's part read at another
//! point. PyJHora's `day_rulers` and `night_rulers` keep the lordless slot right after
//! Saturn's part instead of at the end; that shifts every part after Saturn's by one on
//! the weekdays where Saturn's is not the seventh, which never moves Gulika or Mandi
//! but does move the other four. The text's reading is used here.
//!
//! Longitude: the sidereal ascendant at the start, middle or end of the planet's part
//! ([`KalaPoint`]). Jagannatha Hora's defaults (jagannathhora.com/yamakantaka-calculator,
//! PyJHora `upagraha_longitude`): Gulika at the **start** of Saturn's part, Mandi at
//! the **middle** of Saturn's part, Kala, Mrityu, Ardhaprahara and Yamaghantaka at the
//! middle of their parts ([`Upagraha::default_point`]). The Kerala Mandi table (day:
//! Sun 26, Mon 22, Tue 18, Wed 14, Thu 10, Fri 6, Sat 2 ghatis; night: Sun 10, Mon 6,
//! Tue 2, Wed 26, Thu 22, Fri 18, Sat 14, of the thirty ghatis of the day or night;
//! blog.indianastrologysoftware.com/gulika-maandi, hinducalculator.com/gulika-calculator)
//! is offered as [`mandi_instant_kerala`]; it is not exactly the middle rule.
//!
//! Kala windows: [`gulika_kala`] and [`yamaganda_kala`] reproduce the popular eighth-of-
//! the-daytime tables (Gulika: Sun 7th, Mon 6th … Sat 1st; Yamaganda: Sun 5th, Mon 4th,
//! Tue 3rd, Wed 2nd, Thu 1st, Fri 7th, Sat 6th), which PyJHora's `trikalam` and
//! `kshana_rules::muhurta` also use. Under the rule above they are exactly Saturn's
//! and Jupiter's day parts on every weekday; under PyJHora's variant Jupiter's part
//! would fall one slot late on Friday and Saturday, a further reason to keep the
//! text's reading.
//!
//! Arithmetic upagrahas (BPHS ch. 3): Dhuma = Sun + 133°20′, Vyatipata = 360° − Dhuma,
//! Parivesha = Vyatipata + 180°, Indrachapa = 360° − Parivesha, Upaketu = Indrachapa +
//! 16°40′ (Wikipedia "Upagraha"; PyJHora `solar_upagraha_longitudes`).

use kshana_events::SolarDay;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::Ayanamsa;

use crate::{ascendant_sidereal, weekday, WEEKDAY_LORDS};

/// The six upagrahas taken from the parts of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Upagraha {
    /// The Sun's part.
    Kala,
    /// Mars's part.
    Mrityu,
    /// Mercury's part.
    Ardhaprahara,
    /// Jupiter's part (Yamakantaka).
    Yamaghantaka,
    /// Saturn's part, read at its start.
    Gulika,
    /// Saturn's part, read at its middle.
    Mandi,
}

/// Where in the part the upagraha is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KalaPoint {
    /// The start of the part (BPHS as read by Jagannatha Hora for Gulika).
    Start,
    /// The middle of the part (Jagannatha Hora for Mandi and the other four).
    Middle,
    /// The end of the part (some commentators).
    End,
}

impl Upagraha {
    /// Every variant.
    pub const ALL: [Upagraha; 6] = [
        Upagraha::Kala,
        Upagraha::Mrityu,
        Upagraha::Ardhaprahara,
        Upagraha::Yamaghantaka,
        Upagraha::Gulika,
        Upagraha::Mandi,
    ];

    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            Upagraha::Kala => "Kala",
            Upagraha::Mrityu => "Mrityu",
            Upagraha::Ardhaprahara => "Ardhaprahara",
            Upagraha::Yamaghantaka => "Yamaghantaka",
            Upagraha::Gulika => "Gulika",
            Upagraha::Mandi => "Mandi",
        }
    }

    /// Weekday index (0 = Sun … 6 = Saturn) of the planet whose part it is.
    pub fn planet(self) -> usize {
        match self {
            Upagraha::Kala => 0,
            Upagraha::Mrityu => 2,
            Upagraha::Ardhaprahara => 3,
            Upagraha::Yamaghantaka => 4,
            Upagraha::Gulika | Upagraha::Mandi => 6,
        }
    }

    /// Jagannatha Hora's default point: Gulika at the start, everything else at the middle.
    pub fn default_point(self) -> KalaPoint {
        match self {
            Upagraha::Gulika => KalaPoint::Start,
            _ => KalaPoint::Middle,
        }
    }
}

/// Lords of the eight parts of the day (`night == false`) or night of `weekday`
/// (0 = Sunday), as weekday indices; `None` is the lordless eighth part.
pub fn part_lords(weekday: u32, night: bool) -> [Option<usize>; 8] {
    let first = if night {
        (weekday as usize + 4) % 7
    } else {
        weekday as usize % 7
    };
    std::array::from_fn(|i| if i < 7 { Some((first + i) % 7) } else { None })
}

/// The eight parts of the day or night as `(start, end)`, tiling sunrise→sunset or
/// sunset→next sunrise exactly.
pub fn parts(day: &SolarDay, night: bool) -> [(Instant, Instant); 8] {
    let (base, len) = if night {
        (day.sunset, day.next_sunrise.ns_since(day.sunset))
    } else {
        (day.sunrise, day.sunset.ns_since(day.sunrise))
    };
    std::array::from_fn(|i| {
        (
            base.add_ns(len * i as i128 / 8),
            base.add_ns(len * (i as i128 + 1) / 8),
        )
    })
}

/// The part (kala) of `which` in the day or night: `(start, end)`.
pub fn upagraha_kala(
    day: &SolarDay,
    obs: &Observer,
    which: Upagraha,
    night: bool,
) -> (Instant, Instant) {
    let lords = part_lords(weekday(day, obs), night);
    let idx = lords
        .iter()
        .position(|l| *l == Some(which.planet()))
        .unwrap_or(0);
    parts(day, night)[idx]
}

/// The instant of `which` at `point` of its part in the day or night.
pub fn upagraha_instant(
    day: &SolarDay,
    obs: &Observer,
    which: Upagraha,
    point: KalaPoint,
    night: bool,
) -> Instant {
    let (s, e) = upagraha_kala(day, obs, which, night);
    match point {
        KalaPoint::Start => s,
        KalaPoint::Middle => s.add_ns(e.ns_since(s) / 2),
        KalaPoint::End => e,
    }
}

/// Popular Gulika kala slots (1-based eighth of the daytime) by weekday Sun..Sat.
pub const GULIKA_SLOTS: [usize; 7] = [7, 6, 5, 4, 3, 2, 1];
/// Popular Yamaganda kala slots (1-based eighth of the daytime) by weekday Sun..Sat.
pub const YAMAGANDA_SLOTS: [usize; 7] = [5, 4, 3, 2, 1, 7, 6];

fn day_slot(day: &SolarDay, slot: usize) -> (Instant, Instant) {
    parts(day, false)[slot - 1]
}

/// Gulika kala of the day by the popular table (Saturn's day part).
pub fn gulika_kala(day: &SolarDay, obs: &Observer) -> (Instant, Instant) {
    day_slot(day, GULIKA_SLOTS[weekday(day, obs) as usize])
}

/// Yamaganda kala of the day by the popular table.
pub fn yamaganda_kala(day: &SolarDay, obs: &Observer) -> (Instant, Instant) {
    day_slot(day, YAMAGANDA_SLOTS[weekday(day, obs) as usize])
}

/// Mandi's instant by the Kerala thirty-ghati table: the stated ghati of the day or
/// night, each ghati a thirtieth of the real day or night.
pub fn mandi_instant_kerala(day: &SolarDay, obs: &Observer, night: bool) -> Instant {
    const DAY: [i128; 7] = [26, 22, 18, 14, 10, 6, 2];
    const NIGHT: [i128; 7] = [10, 6, 2, 26, 22, 18, 14];
    let wd = weekday(day, obs) as usize;
    if night {
        day.sunset
            .add_ns(day.next_sunrise.ns_since(day.sunset) * NIGHT[wd] / 30)
    } else {
        day.sunrise
            .add_ns(day.sunset.ns_since(day.sunrise) * DAY[wd] / 30)
    }
}

/// An upagraha's longitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpagrahaPosition {
    /// Which.
    pub which: Upagraha,
    /// Convention used.
    pub point: KalaPoint,
    /// True when taken from the night's parts.
    pub night: bool,
    /// The instant of the kala point.
    pub at: Instant,
    /// Sidereal longitude of the ascendant at that instant, degrees.
    pub sidereal: f64,
}

/// Longitude of `which` for a birth or question at `at` within `day`: the sidereal
/// ascendant at `point` of the planet's part, in the day's parts when `at` is before
/// sunset and in the night's parts otherwise.
pub fn upagraha_longitude(
    eop: Option<&EopTable>,
    obs: &Observer,
    day: &SolarDay,
    which: Upagraha,
    point: KalaPoint,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> UpagrahaPosition {
    let night = at >= day.sunset;
    let t = upagraha_instant(day, obs, which, point, night);
    UpagrahaPosition {
        which,
        point,
        night,
        at: t,
        sidereal: ascendant_sidereal(eop, obs, t, ayanamsa),
    }
}

/// All six upagrahas at their Jagannatha Hora default points.
pub fn upagraha_longitudes(
    eop: Option<&EopTable>,
    obs: &Observer,
    day: &SolarDay,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> [UpagrahaPosition; 6] {
    std::array::from_fn(|i| {
        let u = Upagraha::ALL[i];
        upagraha_longitude(eop, obs, day, u, u.default_point(), at, ayanamsa)
    })
}

/// The five arithmetic upagrahas from the Sun's sidereal longitude: `(name, degrees)`
/// for Dhuma, Vyatipata, Parivesha, Indrachapa and Upaketu.
pub fn solar_upagrahas(sun_sidereal: f64) -> [(&'static str, f64); 5] {
    let n = |d: f64| d.rem_euclid(360.0);
    let dhuma = n(sun_sidereal + 133.0 + 20.0 / 60.0);
    let vyatipata = n(360.0 - dhuma);
    let parivesha = n(vyatipata + 180.0);
    let indrachapa = n(360.0 - parivesha);
    let upaketu = n(indrachapa + 16.0 + 40.0 / 60.0);
    [
        ("Dhuma", dhuma),
        ("Vyatipata", vyatipata),
        ("Parivesha", parivesha),
        ("Indrachapa", indrachapa),
        ("Upaketu", upaketu),
    ]
}

/// Name of the lord of a part, for display.
pub fn part_lord_name(lord: Option<usize>) -> &'static str {
    lord.map(|l| WEEKDAY_LORDS[l % 7]).unwrap_or("none")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, utc, KOCHI};
    use kshana_events::{rahu_kala, solar_day};
    use kshana_time::NS_PER_SEC;

    #[test]
    fn part_lords_follow_the_text() {
        // Sunday: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, none
        assert_eq!(
            part_lords(0, false),
            [
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                None
            ]
        );
        // Sunday night starts with Jupiter, the fifth from the Sun
        assert_eq!(part_lords(0, true)[0], Some(4));
        assert_eq!(part_lords(3, true)[0], Some(0));
        assert_eq!(part_lord_name(part_lords(6, true)[0]), "Mercury");
        assert_eq!(part_lord_name(None), "none");
        // Saturn's day part is the popular Gulika slot on every weekday
        for wd in 0..7u32 {
            let idx = part_lords(wd, false)
                .iter()
                .position(|l| *l == Some(6))
                .unwrap();
            assert_eq!(idx + 1, GULIKA_SLOTS[wd as usize]);
            // Jupiter's day part is the popular Yamaganda slot on every weekday
            let j = part_lords(wd, false)
                .iter()
                .position(|l| *l == Some(4))
                .unwrap();
            assert_eq!(j + 1, YAMAGANDA_SLOTS[wd as usize]);
        }
    }

    #[test]
    fn kalas_and_longitudes_on_a_kochi_thursday() {
        let Some(spk) = kernel() else { return };
        let at = utc(2026, 10, 15, 9, 0, 0);
        let day = solar_day(&spk, None, &KOCHI, at).unwrap().unwrap();
        assert_eq!(weekday(&day, &KOCHI), 4);
        let p = parts(&day, false);
        assert_eq!(p[0].0, day.sunrise);
        assert_eq!(p[7].1, day.sunset);
        for w in p.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
        let n = parts(&day, true);
        assert_eq!(n[0].0, day.sunset);
        assert_eq!(n[7].1, day.next_sunrise);
        // Thursday: Gulika 3rd part, Yamaganda 1st, Rahu kala 6th (from the events crate)
        assert_eq!(gulika_kala(&day, &KOCHI), p[2]);
        assert_eq!(yamaganda_kala(&day, &KOCHI), p[0]);
        let (rs, re) = rahu_kala(&day, &KOCHI);
        assert!(rs.ns_since(p[5].0).abs() <= 8 && re.ns_since(p[5].1).abs() <= 8);
        assert_eq!(upagraha_kala(&day, &KOCHI, Upagraha::Gulika, false), p[2]);
        assert_eq!(upagraha_kala(&day, &KOCHI, Upagraha::Mandi, false), p[2]);
        assert_eq!(upagraha_kala(&day, &KOCHI, Upagraha::Kala, false), p[3]);
        // night: first part Moon (fifth from Jupiter), Saturn's is the 6th
        assert_eq!(upagraha_kala(&day, &KOCHI, Upagraha::Gulika, true), n[5]);
        // points
        let (s, e) = p[2];
        assert_eq!(
            upagraha_instant(&day, &KOCHI, Upagraha::Gulika, KalaPoint::Start, false),
            s
        );
        assert_eq!(
            upagraha_instant(&day, &KOCHI, Upagraha::Gulika, KalaPoint::End, false),
            e
        );
        let m = upagraha_instant(&day, &KOCHI, Upagraha::Mandi, KalaPoint::Middle, false);
        assert_eq!(m.ns_since(s), e.ns_since(m));
        // longitudes: Gulika at the start, Mandi at the middle, both ascendants
        let g = upagraha_longitude(
            None,
            &KOCHI,
            &day,
            Upagraha::Gulika,
            KalaPoint::Start,
            at,
            Ayanamsa::Lahiri,
        );
        let md = upagraha_longitude(
            None,
            &KOCHI,
            &day,
            Upagraha::Mandi,
            KalaPoint::Middle,
            at,
            Ayanamsa::Lahiri,
        );
        assert!(!g.night && g.at == s && md.at == m);
        assert_eq!(
            g.sidereal,
            ascendant_sidereal(None, &KOCHI, s, Ayanamsa::Lahiri)
        );
        // the ascendant moves about a degree every four minutes, so Mandi is ahead of Gulika
        let gap = (md.sidereal - g.sidereal).rem_euclid(360.0);
        let mins = m.ns_since(s) as f64 / 60e9;
        assert!(gap > mins / 5.0 && gap < mins / 3.0, "{gap}° in {mins} min");
        let all = upagraha_longitudes(None, &KOCHI, &day, at, Ayanamsa::Lahiri);
        assert_eq!(all[4].point, KalaPoint::Start);
        assert_eq!(all[5].point, KalaPoint::Middle);
        assert!(all.iter().all(|u| (0.0..360.0).contains(&u.sidereal)));
        // a night instant uses the night's parts
        let night_at = day.sunset.add_ns(3_600 * NS_PER_SEC);
        let gn = upagraha_longitude(
            None,
            &KOCHI,
            &day,
            Upagraha::Gulika,
            KalaPoint::Start,
            night_at,
            Ayanamsa::Lahiri,
        );
        assert!(gn.night && gn.at == n[5].0);
        // Kerala table: Thursday day Mandi at 10 of 30 ghatis = a third of the day
        let k = mandi_instant_kerala(&day, &KOCHI, false);
        assert_eq!(
            k.ns_since(day.sunrise),
            day.sunset.ns_since(day.sunrise) * 10 / 30
        );
        let kn = mandi_instant_kerala(&day, &KOCHI, true);
        assert_eq!(
            kn.ns_since(day.sunset),
            day.next_sunrise.ns_since(day.sunset) * 22 / 30
        );
    }

    #[test]
    fn solar_upagrahas_close_the_chain() {
        let u = solar_upagrahas(10.0);
        assert!((u[0].1 - 143.0 - 20.0 / 60.0).abs() < 1e-9);
        assert!((u[1].1 - (360.0 - u[0].1)).abs() < 1e-9);
        assert!((u[2].1 - (u[1].1 + 180.0).rem_euclid(360.0)).abs() < 1e-9);
        assert!((u[3].1 - (360.0 - u[2].1).rem_euclid(360.0)).abs() < 1e-9);
        assert!((u[4].1 - (u[3].1 + 16.0 + 40.0 / 60.0).rem_euclid(360.0)).abs() < 1e-9);
        assert_eq!(u[4].0, "Upaketu");
        assert_eq!(Upagraha::Yamaghantaka.name(), "Yamaghantaka");
    }
}
