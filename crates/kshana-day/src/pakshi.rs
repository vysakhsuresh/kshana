//! Pancha Pakshi: the five birds of the Tamil Siddhar tradition and their five
//! activities over the solar day.
//!
//! Birth bird (Wikipedia "Pancha Pakshi Shastra"; astrogle.com "Pancha Pakshi (5 bird)
//! Shastra"; PyJHora `pancha_pakshi_stars_birds_paksha`): by the Moon's nakshatra and
//! the paksha at birth. In Shukla paksha Ashwini–Mrigashira (1–5) are the Vulture,
//! Ardra–Purva Phalguni (6–11) the Owl, Uttara Phalguni–Vishakha (12–16) the Crow,
//! Anuradha–Uttara Ashadha (17–21) the Cock and Shravana–Revati (22–27) the Peacock;
//! in Krishna paksha the same groups are read in reverse (1–5 Peacock, 6–11 Cock,
//! 12–16 Crow, 17–21 Owl, 22–27 Vulture). Some web guides use an even 5/5/5/5/7 split;
//! the 5/6/5/5/6 grouping of the texts is used here.
//!
//! Activities: each day (sunrise→sunset) and each night (sunset→next sunrise) is
//! divided into five equal yamas, and in every yama each bird does one of Ruling
//! (Arasu), Eating (Oon), Walking (Nadai), Sleeping (Thujil) or Dying (Saavu); exactly
//! one bird rules in each yama and the five birds do five different things. Which
//! activity falls in which yama depends on the weekday, the paksha and whether it is
//! day or night. The table here ([`ACTIVITY_TABLE`]) was decoded from the 3 500-row
//! Pancha Pakshi database shipped with PyJHora (`data/pancha_pakshi_db.csv`, the
//! Python port of Jagannatha Hora), in which a bird's activities across the five
//! yamas are always a rotation of one cycle per (paksha, day/night): Shukla day
//! Rule–Sleep–Die–Eat–Walk, Shukla night Rule–Die–Walk–Sleep–Eat, Krishna day
//! Rule–Walk–Eat–Die–Sleep, Krishna night Rule–Eat–Sleep–Walk–Die. Web sources agree
//! on the framework (five birds, five activities, weekday and paksha dependence,
//! Ruling strongest and Dying weakest; dekhopanchang.com gives the Cock as the
//! Wednesday day ruler, which the table reproduces) but none publishes the full
//! weekday × paksha matrix, so the matrix itself rests on that one database and is
//! marked unverified below.
//!
//! Power (PyJHora `power_factor`): Ruling 1.0, Eating 0.8, Walking 0.6, Sleeping 0.4,
//! Dying 0.2; a sub-period's power is the product of the main and sub activities'
//! factors. Ruling and Eating count as favourable, Walking as neutral, Sleeping and
//! Dying as unfavourable (the web sources rank them in that order; one gives Eating
//! three quarters and Walking half of that instead of this linear scale).
//!
//! Sub-periods: each yama is itself divided among the five birds in cycle order from
//! the yama's main bird, the sub-activities following the same cycle from the main
//! activity, with unequal lengths by sub-activity. PyJHora's constants say the day
//! lengths are Ruling 48, Eating 30, Walking 36, Sleeping 18, Dying 12 minutes of a
//! 144-minute yama and the night lengths 24, 30, 30, 24, 36, while its database
//! applies them the other way round; the constants' reading is the default here and
//! both sets are exported so the caller can choose.

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{solar_day, SolarDay};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant};
use kshana_vedic::{nakshatra, tithi, Ayanamsa};

use crate::{sidereal_longitude, weekday, DayError};

/// The five birds, in cycle order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bird {
    /// Gridhra.
    Vulture,
    /// Uluka.
    Owl,
    /// Kaka.
    Crow,
    /// Kukkuta.
    Cock,
    /// Mayura.
    Peacock,
}

impl Bird {
    /// Every bird in cycle order.
    pub const ALL: [Bird; 5] = [
        Bird::Vulture,
        Bird::Owl,
        Bird::Crow,
        Bird::Cock,
        Bird::Peacock,
    ];

    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            Bird::Vulture => "Vulture",
            Bird::Owl => "Owl",
            Bird::Crow => "Crow",
            Bird::Cock => "Cock",
            Bird::Peacock => "Peacock",
        }
    }

    /// Index in cycle order, 0 = Vulture.
    pub fn index(self) -> usize {
        Bird::ALL.iter().position(|b| *b == self).unwrap_or(0)
    }
}

/// The five activities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    /// Arasu: strongest.
    Ruling,
    /// Oon.
    Eating,
    /// Nadai.
    Walking,
    /// Thujil.
    Sleeping,
    /// Saavu: weakest.
    Dying,
}

impl Activity {
    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            Activity::Ruling => "Ruling",
            Activity::Eating => "Eating",
            Activity::Walking => "Walking",
            Activity::Sleeping => "Sleeping",
            Activity::Dying => "Fading",
        }
    }

    /// Power factor: 1.0, 0.8, 0.6, 0.4, 0.2.
    pub fn strength(self) -> f64 {
        match self {
            Activity::Ruling => 1.0,
            Activity::Eating => 0.8,
            Activity::Walking => 0.6,
            Activity::Sleeping => 0.4,
            Activity::Dying => 0.2,
        }
    }

    /// Ruling and Eating.
    pub fn favourable(self) -> bool {
        matches!(self, Activity::Ruling | Activity::Eating)
    }

    fn from_letter(c: u8) -> Activity {
        match c {
            b'R' => Activity::Ruling,
            b'E' => Activity::Eating,
            b'W' => Activity::Walking,
            b'S' => Activity::Sleeping,
            _ => Activity::Dying,
        }
    }
}

/// Birth bird from the Moon's sidereal longitude and the paksha.
pub fn birth_bird(moon_sidereal: f64, shukla: bool) -> Bird {
    let n = nakshatra(moon_sidereal).index;
    let group = match n {
        0..=4 => 0,
        5..=10 => 1,
        11..=15 => 2,
        16..=20 => 3,
        _ => 4,
    };
    Bird::ALL[if shukla { group } else { 4 - group }]
}

/// Birth bird at `at`: the Moon's nakshatra and the paksha of the tithi at that instant.
pub fn birth_bird_at(spk: &Spk, at: Instant, ayanamsa: Ayanamsa) -> Result<Bird, SpkError> {
    let moon = sidereal_longitude(spk, Body::Moon, at, ayanamsa)?;
    let sun = sidereal_longitude(spk, Body::Sun, at, ayanamsa)?;
    Ok(birth_bird(moon, tithi(moon, sun).is_shukla()))
}

/// Activities of each bird (columns Vulture, Owl, Crow, Cock, Peacock) in the five
/// yamas (letters R, E, W, S, D), by weekday row (Sunday first), for Shukla day,
/// Shukla night, Krishna day and Krishna night. Decoded from PyJHora's
/// `pancha_pakshi_db.csv`.
pub const ACTIVITY_TABLE: [[[&str; 5]; 7]; 4] = [
    // Shukla paksha, day
    [
        ["EWRSD", "WRSDE", "RSDEW", "SDEWR", "DEWRS"], // Sunday
        ["DEWRS", "EWRSD", "WRSDE", "RSDEW", "SDEWR"], // Monday
        ["EWRSD", "WRSDE", "RSDEW", "SDEWR", "DEWRS"], // Tuesday
        ["DEWRS", "EWRSD", "WRSDE", "RSDEW", "SDEWR"], // Wednesday
        ["SDEWR", "DEWRS", "EWRSD", "WRSDE", "RSDEW"], // Thursday
        ["RSDEW", "SDEWR", "DEWRS", "EWRSD", "WRSDE"], // Friday
        ["WRSDE", "RSDEW", "SDEWR", "DEWRS", "EWRSD"], // Saturday
    ],
    // Shukla paksha, night
    [
        ["DWSER", "RDWSE", "ERDWS", "SERDW", "WSERD"], // Sunday
        ["WSERD", "DWSER", "RDWSE", "ERDWS", "SERDW"], // Monday
        ["DWSER", "RDWSE", "ERDWS", "SERDW", "WSERD"], // Tuesday
        ["WSERD", "DWSER", "RDWSE", "ERDWS", "SERDW"], // Wednesday
        ["SERDW", "WSERD", "DWSER", "RDWSE", "ERDWS"], // Thursday
        ["ERDWS", "SERDW", "WSERD", "DWSER", "RDWSE"], // Friday
        ["RDWSE", "ERDWS", "SERDW", "WSERD", "DWSER"], // Saturday
    ],
    // Krishna paksha, day
    [
        ["WEDSR", "DSRWE", "RWEDS", "EDSRW", "SRWED"], // Sunday
        ["SRWED", "WEDSR", "DSRWE", "RWEDS", "EDSRW"], // Monday
        ["WEDSR", "DSRWE", "RWEDS", "EDSRW", "SRWED"], // Tuesday
        ["DSRWE", "RWEDS", "EDSRW", "SRWED", "WEDSR"], // Wednesday
        ["RWEDS", "EDSRW", "SRWED", "WEDSR", "DSRWE"], // Thursday
        ["EDSRW", "SRWED", "WEDSR", "DSRWE", "RWEDS"], // Friday
        ["SRWED", "WEDSR", "DSRWE", "RWEDS", "EDSRW"], // Saturday
    ],
    // Krishna paksha, night
    [
        ["ESWDR", "RESWD", "DRESW", "WDRES", "SWDRE"], // Sunday
        ["DRESW", "WDRES", "SWDRE", "ESWDR", "RESWD"], // Monday
        ["ESWDR", "RESWD", "DRESW", "WDRES", "SWDRE"], // Tuesday
        ["SWDRE", "ESWDR", "RESWD", "DRESW", "WDRES"], // Wednesday
        ["WDRES", "SWDRE", "ESWDR", "RESWD", "DRESW"], // Thursday
        ["RESWD", "DRESW", "WDRES", "SWDRE", "ESWDR"], // Friday
        ["DRESW", "WDRES", "SWDRE", "ESWDR", "RESWD"], // Saturday
    ],
];

fn table_row(shukla: bool, night: bool) -> usize {
    (if shukla { 0 } else { 2 }) + usize::from(night)
}

/// The activity of `bird` in yama `yama` (0–4) of the day or night of `weekday`
/// (0 = Sunday) in the given paksha.
pub fn yama_activity(shukla: bool, night: bool, weekday: u32, bird: Bird, yama: usize) -> Activity {
    let s = ACTIVITY_TABLE[table_row(shukla, night)][(weekday % 7) as usize][bird.index()];
    Activity::from_letter(s.as_bytes()[yama % 5])
}

/// The bird that rules yama `yama` (0–4).
pub fn ruling_bird(shukla: bool, night: bool, weekday: u32, yama: usize) -> Bird {
    Bird::ALL
        .into_iter()
        .find(|b| yama_activity(shukla, night, weekday, *b, yama) == Activity::Ruling)
        .unwrap_or(Bird::Vulture)
}

/// The activity cycle for a paksha and day/night: the order in which any bird's
/// activities follow one another across the yamas, starting from Ruling.
pub fn activity_cycle(shukla: bool, night: bool) -> [Activity; 5] {
    let s = ACTIVITY_TABLE[table_row(shukla, night)][0][0].as_bytes();
    let start = s.iter().position(|c| *c == b'R').unwrap_or(0);
    std::array::from_fn(|i| Activity::from_letter(s[(start + i) % 5]))
}

/// One yama of a bird's day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PakshiWindow {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// True for the five night yamas.
    pub night: bool,
    /// Yama 1–5 within the day or the night.
    pub yama: u8,
    /// The person's bird.
    pub bird: Bird,
    /// What the bird is doing.
    pub activity: Activity,
    /// Power factor of the activity.
    pub strength: f64,
    /// Ruling or Eating.
    pub favourable: bool,
    /// The bird that rules this yama.
    pub ruling_bird: Bird,
}

/// The ten yama windows of `bird` over `day`, given the paksha: five equal parts of
/// sunrise→sunset and five of sunset→next sunrise, tiling the day exactly.
pub fn pancha_pakshi_day(
    day: &SolarDay,
    obs: &Observer,
    shukla: bool,
    bird: Bird,
) -> Vec<PakshiWindow> {
    let wd = weekday(day, obs);
    let mut out = Vec::with_capacity(10);
    for night in [false, true] {
        let (base, len) = if night {
            (day.sunset, day.next_sunrise.ns_since(day.sunset))
        } else {
            (day.sunrise, day.sunset.ns_since(day.sunrise))
        };
        for y in 0..5usize {
            let activity = yama_activity(shukla, night, wd, bird, y);
            out.push(PakshiWindow {
                start: base.add_ns(len * y as i128 / 5),
                end: base.add_ns(len * (y as i128 + 1) / 5),
                night,
                yama: y as u8 + 1,
                bird,
                activity,
                strength: activity.strength(),
                favourable: activity.favourable(),
                ruling_bird: ruling_bird(shukla, night, wd, y),
            });
        }
    }
    out
}

/// Pancha Pakshi windows of `birth_bird` for the solar day containing `at`: the
/// paksha is that of the tithi at the day's sunrise, the weekday that of the sunrise.
///
/// The activity matrix comes from a single reference implementation (PyJHora's
/// database) and could not be checked against an independent published table.
#[doc = "unverified"]
pub fn pancha_pakshi_windows(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    birth_bird: Bird,
    at: Instant,
) -> Result<Vec<PakshiWindow>, DayError> {
    let day = solar_day(spk, eop, obs, at)?.ok_or(DayError::NoSolarDay)?;
    let m = apparent(spk, Body::Moon, day.sunrise)?.longitude;
    let s = apparent(spk, Body::Sun, day.sunrise)?.longitude;
    Ok(pancha_pakshi_day(
        &day,
        obs,
        tithi(m, s).is_shukla(),
        birth_bird,
    ))
}

/// Lengths of the five sub-periods of a yama, in 144ths of the yama (minutes of a
/// 144-minute yama), by sub-activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubPeriodDurations {
    /// Ruling.
    pub ruling: u32,
    /// Eating.
    pub eating: u32,
    /// Walking.
    pub walking: u32,
    /// Sleeping.
    pub sleeping: u32,
    /// Dying.
    pub dying: u32,
}

impl SubPeriodDurations {
    fn of(self, a: Activity) -> i128 {
        i128::from(match a {
            Activity::Ruling => self.ruling,
            Activity::Eating => self.eating,
            Activity::Walking => self.walking,
            Activity::Sleeping => self.sleeping,
            Activity::Dying => self.dying,
        })
    }

    /// Sum of the five lengths (144 for the published sets).
    pub fn total(self) -> i128 {
        i128::from(self.ruling + self.eating + self.walking + self.sleeping + self.dying)
    }
}

/// Day sub-period lengths per PyJHora's constants: 48, 30, 36, 18, 12 minutes.
pub const SUB_PERIOD_DAY: SubPeriodDurations = SubPeriodDurations {
    ruling: 48,
    eating: 30,
    walking: 36,
    sleeping: 18,
    dying: 12,
};

/// Night sub-period lengths per PyJHora's constants: 24, 30, 30, 24, 36 minutes.
pub const SUB_PERIOD_NIGHT: SubPeriodDurations = SubPeriodDurations {
    ruling: 24,
    eating: 30,
    walking: 30,
    sleeping: 24,
    dying: 36,
};

/// One sub-period of a yama.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PakshiSubWindow {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// True at night.
    pub night: bool,
    /// Yama 1–5.
    pub yama: u8,
    /// Sub-period 1–5 within the yama.
    pub sub: u8,
    /// The person's bird.
    pub bird: Bird,
    /// The person's bird's activity in the yama.
    pub activity: Activity,
    /// The sub-period's bird.
    pub sub_bird: Bird,
    /// The sub-period's activity.
    pub sub_activity: Activity,
    /// Power: the product of the two activities' factors.
    pub power: f64,
}

/// The fifty sub-periods of `bird`'s day: each yama split among the five birds in
/// cycle order from the person's bird, the sub-activities following the paksha's
/// cycle from the yama's activity, with lengths from `day_lengths` and
/// `night_lengths`. The sub-periods of a yama tile it exactly.
///
/// The sub-period lengths are taken from PyJHora's constants, whose own database
/// applies them with day and night exchanged; neither reading could be verified
/// against a published text.
#[doc = "unverified"]
pub fn pancha_pakshi_sub_windows(
    day: &SolarDay,
    obs: &Observer,
    shukla: bool,
    bird: Bird,
    day_lengths: SubPeriodDurations,
    night_lengths: SubPeriodDurations,
) -> Vec<PakshiSubWindow> {
    let mut out = Vec::with_capacity(50);
    for w in pancha_pakshi_day(day, obs, shukla, bird) {
        let cycle = activity_cycle(shukla, w.night);
        let lengths = if w.night { night_lengths } else { day_lengths };
        let total = lengths.total().max(1);
        let a0 = cycle.iter().position(|a| *a == w.activity).unwrap_or(0);
        let len = w.end.ns_since(w.start);
        let mut acc = 0i128;
        for j in 0..5usize {
            let sub_activity = cycle[(a0 + j) % 5];
            let sub_bird = Bird::ALL[(bird.index() + j) % 5];
            let next = acc + lengths.of(sub_activity);
            out.push(PakshiSubWindow {
                start: w.start.add_ns(len * acc / total),
                end: w.start.add_ns(len * next / total),
                night: w.night,
                yama: w.yama,
                sub: j as u8 + 1,
                bird,
                activity: w.activity,
                sub_bird,
                sub_activity,
                power: w.activity.strength() * sub_activity.strength(),
            });
            acc = next;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, utc, KOCHI};

    #[test]
    fn birth_birds_by_nakshatra_and_paksha() {
        let span = 360.0 / 27.0;
        let at = |n: usize| n as f64 * span + 1.0;
        assert_eq!(birth_bird(at(0), true), Bird::Vulture);
        assert_eq!(birth_bird(at(4), true), Bird::Vulture);
        assert_eq!(birth_bird(at(5), true), Bird::Owl);
        assert_eq!(birth_bird(at(10), true), Bird::Owl);
        assert_eq!(birth_bird(at(11), true), Bird::Crow);
        assert_eq!(birth_bird(at(16), true), Bird::Cock);
        assert_eq!(birth_bird(at(21), true), Bird::Peacock);
        assert_eq!(birth_bird(at(26), true), Bird::Peacock);
        assert_eq!(birth_bird(at(0), false), Bird::Peacock);
        assert_eq!(birth_bird(at(5), false), Bird::Cock);
        assert_eq!(birth_bird(at(11), false), Bird::Crow);
        assert_eq!(birth_bird(at(16), false), Bird::Owl);
        assert_eq!(birth_bird(at(26), false), Bird::Vulture);
    }

    #[test]
    fn activity_table_is_a_latin_square_in_every_yama() {
        for shukla in [true, false] {
            for night in [false, true] {
                let cycle = activity_cycle(shukla, night);
                assert_eq!(cycle[0], Activity::Ruling);
                for wd in 0..7u32 {
                    for y in 0..5 {
                        let acts: Vec<_> = Bird::ALL
                            .iter()
                            .map(|b| yama_activity(shukla, night, wd, *b, y))
                            .collect();
                        for a in [
                            Activity::Ruling,
                            Activity::Eating,
                            Activity::Walking,
                            Activity::Sleeping,
                            Activity::Dying,
                        ] {
                            assert_eq!(
                                acts.iter().filter(|x| **x == a).count(),
                                1,
                                "{shukla} {night} {wd} {y}"
                            );
                        }
                        assert_eq!(
                            yama_activity(shukla, night, wd, ruling_bird(shukla, night, wd, y), y),
                            Activity::Ruling
                        );
                    }
                    for b in Bird::ALL {
                        // each bird's five activities are a rotation of the cycle
                        let seq: Vec<_> = (0..5)
                            .map(|y| yama_activity(shukla, night, wd, b, y))
                            .collect();
                        let r = seq.iter().position(|a| *a == Activity::Ruling).unwrap();
                        for (i, a) in cycle.iter().enumerate() {
                            assert_eq!(seq[(r + i) % 5], *a);
                        }
                    }
                }
            }
        }
        // the Wednesday day ruler of the first yama is the Cock (dekhopanchang)
        assert_eq!(ruling_bird(true, false, 3, 0), Bird::Cock);
        assert_eq!(
            activity_cycle(true, false),
            [
                Activity::Ruling,
                Activity::Sleeping,
                Activity::Dying,
                Activity::Eating,
                Activity::Walking
            ]
        );
        assert_eq!(
            activity_cycle(false, true),
            [
                Activity::Ruling,
                Activity::Eating,
                Activity::Sleeping,
                Activity::Walking,
                Activity::Dying
            ]
        );
    }

    #[test]
    fn windows_tile_the_day() {
        let Some(spk) = kernel() else { return };
        let at = utc(2026, 10, 15, 9, 0, 0);
        let day = solar_day(&spk, None, &KOCHI, at).unwrap().unwrap();
        let w = pancha_pakshi_windows(&spk, None, &KOCHI, Bird::Peacock, at).unwrap();
        assert_eq!(w.len(), 10);
        assert_eq!(w[0].start, day.sunrise);
        assert_eq!(w[4].end, day.sunset);
        assert_eq!(w[5].start, day.sunset);
        assert_eq!(w[9].end, day.next_sunrise);
        for p in w.windows(2) {
            assert_eq!(p[0].end, p[1].start);
        }
        assert!(w
            .iter()
            .all(|x| x.bird == Bird::Peacock && x.strength > 0.0 && x.strength <= 1.0));
        assert_eq!(
            w.iter().filter(|x| x.activity == Activity::Ruling).count(),
            2
        );
        assert_eq!(w.iter().filter(|x| x.favourable).count(), 4);
        assert!(w
            .iter()
            .filter(|x| !x.night)
            .all(|x| x.yama >= 1 && x.yama <= 5));
        // the ruling bird of a yama is the one whose activity is Ruling there
        for x in &w {
            if x.activity == Activity::Ruling {
                assert_eq!(x.ruling_bird, Bird::Peacock);
            }
        }
        // sub-periods tile each yama, start with the person's bird, and multiply powers
        let shukla = {
            let m = apparent(&spk, Body::Moon, day.sunrise).unwrap().longitude;
            let s = apparent(&spk, Body::Sun, day.sunrise).unwrap().longitude;
            tithi(m, s).is_shukla()
        };
        let sw = pancha_pakshi_sub_windows(
            &day,
            &KOCHI,
            shukla,
            Bird::Peacock,
            SUB_PERIOD_DAY,
            SUB_PERIOD_NIGHT,
        );
        assert_eq!(sw.len(), 50);
        for (i, x) in w.iter().enumerate() {
            let subs = &sw[5 * i..5 * i + 5];
            assert_eq!(subs[0].start, x.start);
            assert_eq!(subs[4].end, x.end);
            assert_eq!(subs[0].sub_bird, Bird::Peacock);
            assert_eq!(subs[0].sub_activity, x.activity);
            assert_eq!(subs[1].sub_bird, Bird::Vulture);
            for p in subs.windows(2) {
                assert_eq!(p[0].end, p[1].start);
            }
            for s in subs {
                assert!((s.power - x.strength * s.sub_activity.strength()).abs() < 1e-12);
            }
            // the ruling sub-period is a third of a day yama, a sixth of a night yama
            let ruling = subs
                .iter()
                .find(|s| s.sub_activity == Activity::Ruling)
                .unwrap();
            let frac = ruling.end.ns_since(ruling.start) as f64 / x.end.ns_since(x.start) as f64;
            let want = if x.night { 24.0 / 144.0 } else { 48.0 / 144.0 };
            assert!((frac - want).abs() < 1e-9, "{frac}");
        }
        assert_eq!(SUB_PERIOD_DAY.total(), 144);
        assert_eq!(SUB_PERIOD_NIGHT.total(), 144);
    }

    #[test]
    fn fixture_birth_is_a_peacock() {
        let Some(spk) = kernel() else { return };
        // 1990-10-29 17:36 UTC: Moon in Shatabhisha, waxing fortnight
        let b = birth_bird_at(&spk, utc(1990, 10, 29, 17, 36, 0), Ayanamsa::Lahiri).unwrap();
        assert_eq!(b, Bird::Peacock);
        assert_eq!(b.name(), "Peacock");
        assert_eq!(Activity::Dying.name(), "Fading");
    }
}
