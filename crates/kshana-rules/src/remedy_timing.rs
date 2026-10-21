//! Remedy timing: the planet's weekday (from true sunrise), its horas on that day, its
//! tithis, the Moon in its nakshatras, Brahma muhurta and the Rahu kala to avoid, all
//! as exact instants from the solvers in `kshana-events`. Nothing here is estimated:
//! every window is bracketed to a microsecond and carries the sunrise convention used
//! (the Sun's centre at −0°50′, refraction plus semi-diameter).
//!
//! Tables, with the convention each one rests on:
//! 1. Weekday lord, the day running sunrise to sunrise: Sun Sunday, Moon Monday, Mars
//!    Tuesday, Mercury Wednesday, Jupiter Thursday, Venus Friday, Saturn Saturday; by
//!    convention Rahu takes Saturday and Ketu Tuesday.
//! 2. Hora: twenty-four unequal hours, twelve from sunrise to sunset and twelve to the
//!    next sunrise; the first belongs to the day lord and the rest follow the Chaldean
//!    descent (`kshana_events::hora_at`). The nodes have no hora: Saturn's stands for
//!    Rahu and Mars's for Ketu (convention).
//! 3. Nakshatra of the planet: the Moon in one of its three Vimshottari stars (Ketu
//!    Ashwini, Magha, Mula; Venus Bharani, Purva Phalguni, Purva Ashadha; and so on in
//!    the dasha order). Pushya is the universal start-star for japa and vows.
//! 4. Tithi of the planet (Muhurta Chintamani tithi groups): Nanda 1, 6, 11 Venus;
//!    Bhadra 2, 7, 12 Mercury; Jaya 3, 8, 13 Mars; Rikta 4, 9, 14 Saturn; Purna 5, 10,
//!    15 and 30 Jupiter; the same in both pakshas.
//! 5. Observance tithis by planet (temple practice, not a single verse): Sun Saptami;
//!    Moon Purnima, Pradosha and Amavasya; Mars Krishna Chaturthi (Angaraki on a
//!    Tuesday); Mercury Ashtami; Jupiter Ekadashi and Purnima; Venus Shukla Panchami
//!    and Shukla Trayodashi; Saturn Trayodashi (Shani Trayodashi on a Saturday) and
//!    Amavasya; Rahu Amavasya and Panchami (Naga Panchami); Ketu Chaturthi.
//! 6. Brahma muhurta: sunrise minus 96 minutes to sunrise minus 48 minutes. Rahu kala:
//!    the eighth of the daytime fixed by the weekday (`kshana_events::rahu_kala`), to be
//!    avoided for every remedy except the Kerala custom of a Durga lamp for Rahu.

use kshana_ephem::{Body, Spk};
use kshana_events::{
    hora_at, nakshatra_boundaries, rahu_kala, sun_rise_set, tithi_boundaries, EventError, SolarDay,
    MICROSECOND_NS,
};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, nakshatra, sidereal, Ayanamsa, DashaLord, Tithi};

/// One exact window a remedy can be scheduled into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemedyWindow {
    /// Exact start.
    pub start: Instant,
    /// Exact end.
    pub end: Instant,
    /// `"weekday hora"`, `"tithi"`, `"nakshatra"`, `"Rahu kala to avoid"` or
    /// `"Brahma muhurta"`.
    pub kind: &'static str,
    /// Plain words: what the window is and the instants in UTC.
    pub words: String,
    /// For a hora window, its ordinal in the solar day (1 = the first hora after
    /// sunrise, 13 = the first after sunset); 0 for every other kind.
    pub hora_index: u8,
}

impl RemedyWindow {
    /// True for a hora between sunrise and sunset (a daana is given by day, never at night).
    pub fn is_daytime_hora(&self) -> bool {
        (1..=12).contains(&self.hora_index)
    }
}

/// English weekday names, 0 = Sunday.
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Vara names, 0 = Ravivara.
pub const VARA_NAMES: [&str; 7] = [
    "Ravivara",
    "Somavara",
    "Mangalavara",
    "Budhavara",
    "Guruvara",
    "Shukravara",
    "Shanivara",
];

/// Weekday (0 = Sunday … 6 = Saturday) on which the planet's remedies are timed.
/// Rahu takes Saturday and Ketu Tuesday by convention.
pub fn weekday_of(planet: DashaLord) -> u32 {
    match planet {
        DashaLord::Sun => 0,
        DashaLord::Moon => 1,
        DashaLord::Mars | DashaLord::Ketu => 2,
        DashaLord::Mercury => 3,
        DashaLord::Jupiter => 4,
        DashaLord::Venus => 5,
        DashaLord::Saturn | DashaLord::Rahu => 6,
    }
}

/// The hora lord that stands for the planet: the nodes borrow Saturn's and Mars's horas.
pub fn hora_lord_of(planet: DashaLord) -> DashaLord {
    match planet {
        DashaLord::Rahu => DashaLord::Saturn,
        DashaLord::Ketu => DashaLord::Mars,
        p => p,
    }
}

/// The hora lord's name as `kshana_events::hora_at` spells it.
fn hora_name(planet: DashaLord) -> &'static str {
    match hora_lord_of(planet) {
        DashaLord::Sun => "Sun",
        DashaLord::Moon => "Moon",
        DashaLord::Mars => "Mars",
        DashaLord::Mercury => "Mercury",
        DashaLord::Jupiter => "Jupiter",
        DashaLord::Venus => "Venus",
        DashaLord::Saturn => "Saturn",
        DashaLord::Rahu | DashaLord::Ketu => unreachable!("nodes map to Saturn and Mars"),
    }
}

/// Vimshottari lord of a nakshatra by index (0 = Ashwini).
pub fn nakshatra_lord(index: usize) -> DashaLord {
    DashaLord::ORDER[index % 9]
}

/// The planet's tithi group (Muhurta Chintamani), as numbers 1–15 within a paksha.
pub fn tithi_group(planet: DashaLord) -> (&'static str, &'static [u8]) {
    match planet {
        DashaLord::Venus => ("Nanda", &[1, 6, 11]),
        DashaLord::Mercury => ("Bhadra", &[2, 7, 12]),
        DashaLord::Mars => ("Jaya", &[3, 8, 13]),
        DashaLord::Saturn => ("Rikta", &[4, 9, 14]),
        DashaLord::Jupiter => ("Purna", &[5, 10, 15]),
        _ => ("", &[]),
    }
}

/// Observance tithis of the planet (practice), as full numbers 1–30 with their names.
pub fn observance_tithis(planet: DashaLord) -> &'static [(u8, &'static str)] {
    match planet {
        DashaLord::Sun => &[
            (7, "Saptami (Shukla; Bhanu Saptami on a Sunday)"),
            (22, "Saptami (Krishna)"),
        ],
        DashaLord::Moon => &[
            (15, "Purnima"),
            (13, "Pradosha (Shukla Trayodashi)"),
            (28, "Pradosha (Krishna Trayodashi)"),
            (30, "Amavasya (Somavati on a Monday)"),
        ],
        DashaLord::Mars => &[(19, "Krishna Chaturthi (Sankashti; Angaraki on a Tuesday)")],
        DashaLord::Mercury => &[
            (8, "Ashtami (Shukla; Budha Ashtami on a Wednesday)"),
            (23, "Ashtami (Krishna)"),
        ],
        DashaLord::Jupiter => &[
            (11, "Ekadashi (Shukla)"),
            (26, "Ekadashi (Krishna)"),
            (15, "Purnima (Guru Purnima in Ashadha)"),
        ],
        DashaLord::Venus => &[
            (5, "Shukla Panchami"),
            (13, "Shukla Trayodashi (Shukra Pradosha on a Friday)"),
        ],
        DashaLord::Saturn => &[
            (13, "Trayodashi (Shukla; Shani Trayodashi on a Saturday)"),
            (28, "Trayodashi (Krishna; Shani Trayodashi on a Saturday)"),
            (30, "Amavasya (Shani Amavasya on a Saturday)"),
        ],
        DashaLord::Rahu => &[
            (30, "Amavasya"),
            (5, "Shukla Panchami (Naga Panchami in Shravana)"),
        ],
        DashaLord::Ketu => &[
            (4, "Chaturthi (Shukla)"),
            (19, "Sankashti Chaturthi (broken at moonrise)"),
        ],
    }
}

/// Every tithi number (1–30) the planet's remedies may be timed to, with its label:
/// the Muhurta Chintamani group in both pakshas plus the observance tithis.
pub fn tithi_numbers(planet: DashaLord) -> Vec<(u8, String)> {
    let mut out: Vec<(u8, String)> = Vec::new();
    let (group, base) = tithi_group(planet);
    for &n in base {
        for k in [n, n + 15] {
            let t = Tithi {
                number: k,
                fraction: 0.0,
            };
            out.push((
                k,
                format!(
                    "{} ({}), a {group} tithi of {planet:?} (Muhurta Chintamani)",
                    t.name(),
                    if t.is_shukla() { "Shukla" } else { "Krishna" }
                ),
            ));
        }
    }
    for &(n, label) in observance_tithis(planet) {
        if !out.iter().any(|(k, _)| *k == n) {
            out.push((
                n,
                format!("{label}, an observance tithi of {planet:?} (practice)"),
            ));
        }
    }
    out.sort_by_key(|(k, _)| *k);
    out
}

/// A civil date-time in UTC for words.
pub fn fmt_utc(t: Instant) -> String {
    let u = Utc::from_instant(t);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        u.year, u.month, u.day, u.hour, u.minute, u.second
    )
}

fn ordinal(n: u8) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        21 => "21st".into(),
        22 => "22nd".into(),
        23 => "23rd".into(),
        n => format!("{n}th"),
    }
}

/// Weekday (0 = Sunday) of the local civil date at a sunrise, by longitude offset: the
/// same rule `kshana_events` and the muhurta scorer use.
pub fn weekday_at(sunrise: Instant, obs: &Observer) -> u32 {
    let local = sunrise.add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

/// One solar day with its weekday.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayFrame {
    /// Sunrise, sunset, next sunrise.
    pub day: SolarDay,
    /// Weekday, 0 = Sunday.
    pub weekday: u32,
}

/// One tithi's exact span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TithiSpan {
    /// Exact start.
    pub start: Instant,
    /// Exact end.
    pub end: Instant,
    /// 1–30.
    pub number: u8,
}

/// The Moon's exact stay in one nakshatra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NakshatraSpan {
    /// Exact entry.
    pub start: Instant,
    /// Exact exit.
    pub end: Instant,
    /// 0 = Ashwini.
    pub index: usize,
}

/// The solved panchanga frame for a span of days, computed once and queried per planet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimingTable {
    /// Where the table was computed.
    pub from: Instant,
    /// Solar days from the one containing `from`.
    pub days: Vec<DayFrame>,
    /// Tithi spans overlapping the range.
    pub tithis: Vec<TithiSpan>,
    /// The Moon's nakshatra spans overlapping the range.
    pub nakshatras: Vec<NakshatraSpan>,
}

fn tithi_number_at(spk: &Spk, t: Instant) -> Result<u8, EventError> {
    let m = apparent(spk, Body::Moon, t)
        .map_err(EventError::Spk)?
        .longitude;
    let s = apparent(spk, Body::Sun, t)
        .map_err(EventError::Spk)?
        .longitude;
    Ok(((m - s).rem_euclid(360.0) / 12.0).floor() as u8 + 1)
}

fn nakshatra_index_at(spk: &Spk, ayanamsa: Ayanamsa, t: Instant) -> Result<usize, EventError> {
    let m = apparent(spk, Body::Moon, t)
        .map_err(EventError::Spk)?
        .longitude;
    let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
    Ok(nakshatra(sidereal(m, ay)).index)
}

impl TimingTable {
    /// Solve sunrises, sunsets, tithi and nakshatra boundaries for `days` days from
    /// `from` (the day containing `from` is included), each to a microsecond.
    pub fn compute(
        spk: &Spk,
        eop: Option<&EopTable>,
        obs: &Observer,
        ayanamsa: Ayanamsa,
        from: Instant,
        days: u32,
    ) -> Result<Self, EventError> {
        let to = from.add_ns(days as i128 * NS_PER_DAY);
        let (rises, sets) = sun_rise_set(
            spk,
            eop,
            obs,
            from.add_ns(-2 * NS_PER_DAY),
            to.add_ns(NS_PER_DAY),
            MICROSECOND_NS,
        )?;
        let mut frames = Vec::new();
        for w in rises.windows(2) {
            let (r0, r1) = (w[0], w[1]);
            if r1 <= from {
                continue;
            }
            if let Some(s) = sets.iter().copied().find(|s| *s > r0 && *s < r1) {
                frames.push(DayFrame {
                    day: SolarDay {
                        sunrise: r0,
                        sunset: s,
                        next_sunrise: r1,
                    },
                    weekday: weekday_at(r0, obs),
                });
            }
        }
        let tb = tithi_boundaries(spk, from.add_ns(-2 * NS_PER_DAY), to, MICROSECOND_NS)?;
        let mut tithis = Vec::new();
        for w in tb.windows(2) {
            let (a, b) = (w[0].root.t, w[1].root.t);
            if b <= from {
                continue;
            }
            tithis.push(TithiSpan {
                start: a,
                end: b,
                number: tithi_number_at(spk, a.add_ns(60 * NS_PER_SEC))?,
            });
        }
        let nb = nakshatra_boundaries(
            spk,
            Body::Moon,
            ayanamsa,
            from.add_ns(-2 * NS_PER_DAY),
            to,
            MICROSECOND_NS,
        )?;
        let mut nakshatras = Vec::new();
        for w in nb.windows(2) {
            let (a, b) = (w[0].root.t, w[1].root.t);
            if b <= from {
                continue;
            }
            nakshatras.push(NakshatraSpan {
                start: a,
                end: b,
                index: nakshatra_index_at(spk, ayanamsa, a.add_ns(60 * NS_PER_SEC))?,
            });
        }
        Ok(Self {
            from,
            days: frames,
            tithis,
            nakshatras,
        })
    }

    /// The planet's windows: on each of the next `count` of its weekdays, every hora
    /// of the planet (the first after sunrise marked as the day's cue), the Rahu kala
    /// to avoid and the Brahma muhurta before that sunrise; plus up to `count` of its
    /// tithis and `count` of the Moon's stays in its nakshatras. Sorted by start. A
    /// hora that overlaps Rahu kala is left out.
    pub fn windows(&self, obs: &Observer, planet: DashaLord, count: usize) -> Vec<RemedyWindow> {
        let wd = weekday_of(planet);
        let hname = hora_name(planet);
        let mut out = Vec::new();
        let mut found = 0;
        for f in &self.days {
            if f.weekday != wd {
                continue;
            }
            if found == count {
                break;
            }
            found += 1;
            let day = &f.day;
            let rk = rahu_kala(day, obs);
            let day_len = day.sunset.ns_since(day.sunrise) / 12;
            let night_len = day.next_sunrise.ns_since(day.sunset) / 12;
            for i in 0..24u8 {
                let probe = if i < 12 {
                    day.sunrise.add_ns(i as i128 * day_len + day_len / 2)
                } else {
                    day.sunset
                        .add_ns((i - 12) as i128 * night_len + night_len / 2)
                };
                let (lord, s, e) = hora_at(day, obs, probe);
                if lord != hname || (s < rk.1 && e > rk.0) {
                    continue;
                }
                let idx = i + 1;
                let when = if idx == 1 {
                    "the first hora after sunrise, the day's cue"
                } else if idx <= 12 {
                    "by day"
                } else {
                    "by night"
                };
                out.push(RemedyWindow {
                    start: s,
                    end: e,
                    kind: "weekday hora",
                    words: format!(
                        "{hname}'s hora, the {} of {} ({}); {when}: {} to {}",
                        ordinal(idx),
                        WEEKDAY_NAMES[wd as usize],
                        VARA_NAMES[wd as usize],
                        fmt_utc(s),
                        fmt_utc(e)
                    ),
                    hora_index: idx,
                });
            }
            out.push(RemedyWindow {
                start: rk.0,
                end: rk.1,
                kind: "Rahu kala to avoid",
                words: format!(
                    "Rahu kala on {}: {} to {}; no remedy is begun here (the Kerala custom lights a Durga lamp for Rahu in it); a hora of {hname} overlapping it is left out",
                    WEEKDAY_NAMES[wd as usize],
                    fmt_utc(rk.0),
                    fmt_utc(rk.1)
                ),
                hora_index: 0,
            });
            let bm_start = day.sunrise.add_ns(-96 * 60 * NS_PER_SEC);
            let bm_end = day.sunrise.add_ns(-48 * 60 * NS_PER_SEC);
            out.push(RemedyWindow {
                start: bm_start,
                end: bm_end,
                kind: "Brahma muhurta",
                words: format!(
                    "Brahma muhurta before the {} sunrise (sunrise − 96 min to − 48 min; sunrise at {} with the Sun's centre at −0°50′): {} to {}",
                    WEEKDAY_NAMES[wd as usize],
                    fmt_utc(day.sunrise),
                    fmt_utc(bm_start),
                    fmt_utc(bm_end)
                ),
                hora_index: 0,
            });
        }
        let numbers = tithi_numbers(planet);
        out.extend(self.tithi_windows(&numbers, count));
        let mut n = 0;
        for sp in &self.nakshatras {
            if n == count {
                break;
            }
            if nakshatra_lord(sp.index) == planet {
                n += 1;
                let name = nakshatra((sp.index as f64 + 0.5) * kshana_vedic::NAKSHATRA_SPAN).name();
                out.push(RemedyWindow {
                    start: sp.start,
                    end: sp.end,
                    kind: "nakshatra",
                    words: format!(
                        "the Moon in {name}, a star of {planet:?} (Vimshottari): {} to {}",
                        fmt_utc(sp.start),
                        fmt_utc(sp.end)
                    ),
                    hora_index: 0,
                });
            }
        }
        out.retain(|w| w.end > self.from);
        out.sort_by_key(|w| w.start);
        out
    }

    /// Up to `count` exact spans of the given tithis (numbers 1–30 with labels), in order.
    pub fn tithi_windows(&self, numbers: &[(u8, String)], count: usize) -> Vec<RemedyWindow> {
        let mut out = Vec::new();
        for sp in &self.tithis {
            if out.len() == count {
                break;
            }
            if sp.end <= self.from {
                continue;
            }
            if let Some((_, label)) = numbers.iter().find(|(n, _)| *n == sp.number) {
                out.push(RemedyWindow {
                    start: sp.start,
                    end: sp.end,
                    kind: "tithi",
                    words: format!("{label}: {} to {}", fmt_utc(sp.start), fmt_utc(sp.end)),
                    hora_index: 0,
                });
            }
        }
        out
    }

    /// The first sunrise of the planet's weekday at or after `from`, if solved.
    pub fn next_weekday_sunrise(&self, planet: DashaLord) -> Option<Instant> {
        let wd = weekday_of(planet);
        self.days
            .iter()
            .find(|f| f.weekday == wd && f.day.sunrise >= self.from)
            .map(|f| f.day.sunrise)
    }
}

/// The chosen first hour of a remedy: a daytime hora of the planet on its own weekday,
/// clear of that day's Rahu kala, preferring a day whose tithi or the Moon's nakshatra
/// belongs to the planet (Pushya counts for every planet) and the waxing fortnight, and
/// never more than a fortnight later than the soonest such hora.
///
/// The preference, in points: the planet's tithi (table 4 or 5 above) 2; the Moon in the
/// planet's star or in Pushya 2; Shukla paksha 1 (the texts begin japa and vows in the
/// bright fortnight); the first hora after sunrise 1 (the day's cue); each week of delay
/// costs 2, so a supportive day is waited for at most two weeks. Ties go to the earlier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartChoice {
    /// The hora window the remedy begins in.
    pub hora: RemedyWindow,
    /// The solar day it falls in.
    pub day: SolarDay,
    /// Weekday, 0 = Sunday (always the planet's own).
    pub weekday: u32,
    /// That day's Rahu kala, which the hora keeps clear of.
    pub rahu_kala: (Instant, Instant),
    /// Tithi (1–30) at the middle of the hora.
    pub tithi: Option<u8>,
    /// True when that tithi is one of the planet's.
    pub tithi_supports: bool,
    /// The Moon's nakshatra (0 = Ashwini) at the middle of the hora.
    pub star: Option<usize>,
    /// True when that star is the planet's own or Pushya.
    pub star_supports: bool,
    /// True in Shukla paksha.
    pub waxing: bool,
    /// Weeks after the soonest candidate day (0 = the soonest of the planet's weekdays).
    pub weeks_later: u32,
    /// The preference score above.
    pub score: i32,
}

/// Pushya, the universal start-star for vows and japa (index 7).
pub const PUSHYA: usize = 7;

impl TimingTable {
    /// The tithi number (1–30) at `t`, if the table spans it.
    pub fn tithi_at(&self, t: Instant) -> Option<u8> {
        self.tithis.iter().find(|s| s.start <= t && t < s.end).map(|s| s.number)
    }

    /// The Moon's nakshatra index at `t`, if the table spans it.
    pub fn nakshatra_at(&self, t: Instant) -> Option<usize> {
        self.nakshatras.iter().find(|s| s.start <= t && t < s.end).map(|s| s.index)
    }

    /// The best first hour for a remedy of `planet` beginning after `after`, looking at
    /// up to `weeks` of the planet's weekdays (see [`StartChoice`]). Only daytime horas
    /// (sunrise to sunset) are offered: a remedy is begun by day. None when the table
    /// holds none of the planet's weekdays after `after`.
    pub fn best_start(&self, obs: &Observer, planet: DashaLord, after: Instant, weeks: usize) -> Option<StartChoice> {
        let own = tithi_numbers(planet);
        let mut best: Option<StartChoice> = None;
        let mut first_day: Option<Instant> = None;
        for w in self.windows(obs, planet, weeks + 1) {
            if !w.is_daytime_hora() || w.start <= after {
                continue;
            }
            let Some(f) = self.days.iter().find(|f| f.day.sunrise <= w.start && w.start < f.day.next_sunrise) else {
                continue;
            };
            let first = *first_day.get_or_insert(f.day.sunrise);
            let weeks_later = ((f.day.sunrise.ns_since(first) + 7 * NS_PER_DAY / 2) / (7 * NS_PER_DAY)) as u32;
            if weeks_later as usize >= weeks {
                break;
            }
            let rk = rahu_kala(&f.day, obs);
            if w.start < rk.1 && w.end > rk.0 {
                continue;
            }
            let mid = w.start.add_ns(w.end.ns_since(w.start) / 2);
            let tithi = self.tithi_at(mid);
            let star = self.nakshatra_at(mid);
            let tithi_supports = tithi.is_some_and(|n| own.iter().any(|(k, _)| *k == n));
            let star_supports = star.is_some_and(|i| i == PUSHYA || nakshatra_lord(i) == planet);
            let waxing = tithi.is_some_and(|n| n <= 15);
            let score = 2 * i32::from(tithi_supports) + 2 * i32::from(star_supports) + i32::from(waxing) + i32::from(w.hora_index == 1)
                - 2 * weeks_later as i32;
            let c = StartChoice {
                hora: w,
                day: f.day,
                weekday: f.weekday,
                rahu_kala: rk,
                tithi,
                tithi_supports,
                star,
                star_supports,
                waxing,
                weeks_later,
                score,
            };
            if best.as_ref().map_or(true, |b| c.score > b.score) {
                best = Some(c);
            }
        }
        best
    }
}

/// How long a remedy is kept, with the count's standing: a text, a temple or family
/// custom, or the app's plain default where the tradition fixes no count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keep {
    /// How many times it is done (days for a daily vow, weekdays for a weekly one).
    pub times: u32,
    /// Days from the first time to the end of the last, inclusive.
    pub days: u32,
    /// Days between two times: 1 daily, 7 weekly, 30 monthly, 0 for a single act.
    pub every: u32,
    /// `"text"`, `"custom"` or `"default"`.
    pub basis: &'static str,
    /// Where the count comes from.
    pub source: &'static str,
}

/// The mandala: 48 days.
pub const KEEP_SOURCE_MANDALA: &str = "A mandala of 48 days (41 in the Sabarimala vow) is the temple custom of South India for a daily vow or japa; custom, not a single verse";
/// Lal Kitab's vow length.
pub const KEEP_SOURCE_LAL_KITAB: &str = "Lal Kitab (1939–1952): a remedy is kept 40 to 43 days without a break";
/// The 21-day vow.
pub const KEEP_SOURCE_21: &str = "Twenty-one days is the common short vow of household practice; custom, no single verse";
/// The one-month vow.
pub const KEEP_SOURCE_MONTH: &str = "One lunar month, the span of a month-long vrata (as Chandrayana is kept, BPHS ch. 85); the length is the text's, the act is the app's";
/// The weekday counts.
pub const KEEP_SOURCE_WEEKDAYS: &str = "Weekday vratas as practised across India: 12 Sundays, 16 Mondays, 21 Tuesdays, 21 Wednesdays, 16 Thursdays, 16 Fridays, 7 Saturdays (19 in some families); practice, no single classical verse";
/// No count in the tradition.
pub const KEEP_SOURCE_DEFAULT: &str = "The tradition fixes no count for this; eight weeks (three months for a monthly act, one week or month for a single act) is the app's plain default";

/// The weekday count the tradition gives for the planet's weekday vow, if any (the
/// nodes have none).
pub fn weekday_count(planet: DashaLord) -> Option<u32> {
    match planet {
        DashaLord::Sun => Some(12),
        DashaLord::Moon => Some(16),
        DashaLord::Mars | DashaLord::Mercury => Some(21),
        DashaLord::Jupiter | DashaLord::Venus => Some(16),
        DashaLord::Saturn => Some(7),
        DashaLord::Rahu | DashaLord::Ketu => None,
    }
}

/// How long to keep a remedy, from its effort key (`remedy_tree::effort_key`), its
/// branch and whether it is tied to the planet's weekday: a daily act of faith a mandala
/// of 48 days; a daily deed or habit 21 days; a "40 days" item the Lal Kitab's 40; a
/// "30 days" item one lunar month; a weekly act the weekday count of the planet's vow
/// (eight weeks for the nodes or when no day is set); a single act once, in its week or
/// month.
pub fn keep_for(effort_key: &str, branch: &str, planet: DashaLord, on_weekday: bool) -> Keep {
    let faith = matches!(branch, "tradition" | "prayer");
    let weekly = |n: Option<u32>| match n {
        Some(n) if on_weekday => Keep { times: n, days: (n - 1) * 7 + 1, every: 7, basis: "custom", source: KEEP_SOURCE_WEEKDAYS },
        _ => Keep { times: 8, days: 7 * 7 + 1, every: 7, basis: "default", source: KEEP_SOURCE_DEFAULT },
    };
    let daily = |n: u32, basis: &'static str, source: &'static str| Keep { times: n, days: n, every: 1, basis, source };
    match effort_key {
        "40" => daily(40, "text", KEEP_SOURCE_LAL_KITAB),
        "30" => daily(30, "text", KEEP_SOURCE_MONTH),
        "daily" | "10min" if faith => daily(48, "custom", KEEP_SOURCE_MANDALA),
        "daily" | "10min" => daily(21, "custom", KEEP_SOURCE_21),
        "weekly" => weekly(weekday_count(planet)),
        "wish" if on_weekday => weekly(weekday_count(planet)),
        "wish" => daily(48, "custom", KEEP_SOURCE_MANDALA),
        "3w" => Keep { times: 24, days: 7 * 8, every: 2, basis: "default", source: KEEP_SOURCE_DEFAULT },
        "monthly" => Keep { times: 3, days: 61, every: 30, basis: "default", source: KEEP_SOURCE_DEFAULT },
        "week" => Keep { times: 1, days: 7, every: 0, basis: "default", source: KEEP_SOURCE_DEFAULT },
        "month" => Keep { times: 1, days: 30, every: 0, basis: "default", source: KEEP_SOURCE_DEFAULT },
        _ => Keep { times: 1, days: 1, every: 0, basis: "default", source: KEEP_SOURCE_DEFAULT },
    }
}

/// The planet's next windows from `from`: `count` of its weekdays with every hora of
/// the planet on each (the first after sunrise is the day's cue), the Rahu kala to
/// avoid and the Brahma muhurta before that sunrise, plus up to `count` of its tithis
/// and of the Moon's stays in its nakshatras. Every instant is solved to a microsecond.
pub fn next_windows(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    planet: DashaLord,
    from: Instant,
    count: usize,
) -> Result<Vec<RemedyWindow>, EventError> {
    let days = (7 * count as u32 + 2).max(35);
    let table = TimingTable::compute(spk, eop, obs, ayanamsa, from, days)?;
    Ok(table.windows(obs, planet, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_events::solar_day;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn kochi() -> Observer {
        Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        }
    }

    #[test]
    fn tables_are_consistent() {
        assert_eq!(weekday_of(DashaLord::Rahu), 6);
        assert_eq!(weekday_of(DashaLord::Ketu), 2);
        assert_eq!(hora_lord_of(DashaLord::Rahu), DashaLord::Saturn);
        assert_eq!(nakshatra_lord(0), DashaLord::Ketu);
        assert_eq!(nakshatra_lord(7), DashaLord::Saturn); // Pushya
        assert_eq!(nakshatra_lord(26), DashaLord::Mercury); // Revati
        let sat = tithi_numbers(DashaLord::Saturn);
        let nums: Vec<u8> = sat.iter().map(|(n, _)| *n).collect();
        assert_eq!(nums, vec![4, 9, 13, 14, 19, 24, 28, 29, 30]);
        let sun = tithi_numbers(DashaLord::Sun);
        assert_eq!(sun.iter().map(|(n, _)| *n).collect::<Vec<_>>(), vec![7, 22]);
        for g in DashaLord::ORDER {
            assert!(!tithi_numbers(g).is_empty());
        }
    }

    #[test]
    fn saturn_windows_fall_on_saturdays_in_saturn_horas_outside_rahu_kala() {
        let Some(spk) = kernel() else { return };
        let obs = kochi();
        let from = Utc::new(2026, 10, 15, 9, 2, 7, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let w = next_windows(
            &spk,
            None,
            &obs,
            Ayanamsa::Lahiri,
            DashaLord::Saturn,
            from,
            3,
        )
        .unwrap();
        assert!(w.windows(2).all(|p| p[0].start <= p[1].start));
        assert!(w.iter().all(|x| x.start < x.end && x.end > from));
        let horas: Vec<&RemedyWindow> = w.iter().filter(|x| x.kind == "weekday hora").collect();
        let firsts = horas.iter().filter(|x| x.hora_index == 1).count();
        assert_eq!(firsts, 3, "three Saturday sunrises");
        assert!(horas.len() >= 9, "{}", horas.len());
        let mut days_seen = Vec::new();
        // solar_day solves sunrise to 1 ms; the table to 1 µs, so probe at the midpoint
        // and allow 10 ms on the boundaries
        let tol = 10 * NS_PER_SEC / 1_000;
        for h in &horas {
            let mid = h.start.add_ns(h.end.ns_since(h.start) / 2);
            let d = solar_day(&spk, None, &obs, mid).unwrap().unwrap();
            assert_eq!(weekday_at(d.sunrise, &obs), 6, "{}", h.words);
            let (lord, s, e) = hora_at(&d, &obs, mid);
            assert_eq!(lord, "Saturn", "{}", h.words);
            assert!(
                (s.ns_since(h.start)).abs() < tol && (e.ns_since(h.end)).abs() < tol,
                "{}",
                h.words
            );
            let (rs, re) = rahu_kala(&d, &obs);
            assert!(
                h.end <= rs || h.start >= re,
                "{} overlaps Rahu kala",
                h.words
            );
            if h.hora_index == 1 {
                assert!((h.start.ns_since(d.sunrise)).abs() < tol);
                days_seen.push(d.sunrise);
            }
        }
        days_seen.dedup();
        assert_eq!(days_seen.len(), 3);
        let bm = w.iter().filter(|x| x.kind == "Brahma muhurta").count();
        let rk = w.iter().filter(|x| x.kind == "Rahu kala to avoid").count();
        assert!(bm >= 2 && rk == 3, "{bm} {rk}");
        // tithi windows carry Saturn's numbers, checked on the sky at the midpoint
        let tithis: Vec<&RemedyWindow> = w.iter().filter(|x| x.kind == "tithi").collect();
        assert_eq!(tithis.len(), 3);
        let allowed: Vec<u8> = tithi_numbers(DashaLord::Saturn)
            .iter()
            .map(|(n, _)| *n)
            .collect();
        for t in &tithis {
            let mid = t.start.add_ns(t.end.ns_since(t.start) / 2);
            let n = tithi_number_at(&spk, mid).unwrap();
            assert!(allowed.contains(&n), "{} gave tithi {n}", t.words);
        }
        // nakshatra windows have Saturn as Vimshottari lord at the midpoint
        let naks: Vec<&RemedyWindow> = w.iter().filter(|x| x.kind == "nakshatra").collect();
        assert_eq!(naks.len(), 3);
        for x in &naks {
            let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
            let i = nakshatra_index_at(&spk, Ayanamsa::Lahiri, mid).unwrap();
            assert_eq!(nakshatra_lord(i), DashaLord::Saturn, "{}", x.words);
        }
    }

    #[test]
    fn best_start_is_a_daytime_hora_on_the_weekday_outside_rahu_kala() {
        let Some(spk) = kernel() else { return };
        let obs = kochi();
        let from = Utc::new(2026, 10, 21, 3, 0, 0, 0).unwrap().to_instant().unwrap();
        let table = TimingTable::compute(&spk, None, &obs, Ayanamsa::Lahiri, from, 35).unwrap();
        let tol = 10 * NS_PER_SEC / 1_000;
        for g in DashaLord::ORDER {
            let c = table.best_start(&obs, g, from, 3).unwrap_or_else(|| panic!("{g:?}"));
            assert!(c.hora.start > from && c.hora.is_daytime_hora(), "{g:?} {}", c.hora.words);
            assert!(c.weeks_later < 3);
            let mid = c.hora.start.add_ns(c.hora.end.ns_since(c.hora.start) / 2);
            let d = solar_day(&spk, None, &obs, mid).unwrap().unwrap();
            assert_eq!(weekday_at(d.sunrise, &obs), weekday_of(g), "{g:?}");
            assert_eq!(c.weekday, weekday_of(g));
            let (lord, s, _) = hora_at(&d, &obs, mid);
            assert_eq!(lord, hora_name(g), "{g:?}");
            assert!(s.ns_since(c.hora.start).abs() < tol);
            let (rs, re) = rahu_kala(&d, &obs);
            assert!(c.hora.end <= rs || c.hora.start >= re, "{g:?} in Rahu kala");
            assert!(c.hora.end <= d.sunset.add_ns(tol), "{g:?} by day");
            // The support flags agree with the sky.
            if let Some(n) = c.tithi {
                assert_eq!(n, tithi_number_at(&spk, mid).unwrap());
                assert_eq!(c.tithi_supports, tithi_numbers(g).iter().any(|(k, _)| *k == n));
            }
            if let Some(i) = c.star {
                assert_eq!(i, nakshatra_index_at(&spk, Ayanamsa::Lahiri, mid).unwrap());
            }
            // Nothing else in reach scores higher.
            for w in table.windows(&obs, g, 4) {
                if w.is_daytime_hora() && w.start > from && w.start < c.hora.start {
                    let m = w.start.add_ns(w.end.ns_since(w.start) / 2);
                    let t = table.tithi_at(m).unwrap();
                    let st = table.nakshatra_at(m).unwrap();
                    let sc = 2 * i32::from(tithi_numbers(g).iter().any(|(k, _)| *k == t))
                        + 2 * i32::from(st == PUSHYA || nakshatra_lord(st) == g)
                        + i32::from(t <= 15)
                        + i32::from(w.hora_index == 1);
                    assert!(sc - 2 * ((w.start.ns_since(from) / (7 * NS_PER_DAY)) as i32) <= c.score + 2, "{g:?}");
                }
            }
        }
    }

    #[test]
    fn keep_lengths() {
        assert_eq!(keep_for("weekly", "deeds", DashaLord::Saturn, true).times, 7);
        assert_eq!(keep_for("weekly", "deeds", DashaLord::Sun, true).times, 12);
        assert_eq!(keep_for("weekly", "deeds", DashaLord::Sun, false).basis, "default");
        assert_eq!(keep_for("daily", "mind", DashaLord::Moon, false).days, 21);
        assert_eq!(keep_for("daily", "tradition", DashaLord::Moon, false).days, 48);
        assert_eq!(keep_for("40", "deeds", DashaLord::Moon, false).basis, "text");
        assert_eq!(keep_for("once", "giving", DashaLord::Moon, false).times, 1);
    }

    #[test]
    fn rahu_borrows_saturday_and_saturns_hora_and_the_sun_gets_sundays() {
        let Some(spk) = kernel() else { return };
        let obs = kochi();
        let from = Utc::new(2026, 10, 15, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let table = TimingTable::compute(&spk, None, &obs, Ayanamsa::Lahiri, from, 16).unwrap();
        assert!(table.days.len() >= 15);
        for d in table.days.windows(2) {
            assert_eq!((d[0].weekday + 1) % 7, d[1].weekday);
            assert_eq!(d[0].day.next_sunrise, d[1].day.sunrise);
        }
        let rahu = table.windows(&obs, DashaLord::Rahu, 2);
        for h in rahu.iter().filter(|x| x.kind == "weekday hora") {
            let mid = h.start.add_ns(h.end.ns_since(h.start) / 2);
            let d = solar_day(&spk, None, &obs, mid).unwrap().unwrap();
            assert_eq!(weekday_at(d.sunrise, &obs), 6);
            assert_eq!(hora_at(&d, &obs, mid).0, "Saturn");
        }
        let sun = table.windows(&obs, DashaLord::Sun, 2);
        for h in sun.iter().filter(|x| x.kind == "weekday hora") {
            let mid = h.start.add_ns(h.end.ns_since(h.start) / 2);
            let d = solar_day(&spk, None, &obs, mid).unwrap().unwrap();
            assert_eq!(weekday_at(d.sunrise, &obs), 0);
            assert_eq!(hora_at(&d, &obs, mid).0, "Sun");
        }
        assert!(table.next_weekday_sunrise(DashaLord::Sun).is_some());
    }
}
