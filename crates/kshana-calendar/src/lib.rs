//! The Hindu calendar (panchanga) for a place, by the drik (observed-sky) rules of
//! modern Indian almanacs.
//!
//! References, cited at each rule:
//! * Indian Calendar Reform Committee, *Report* (CSIR, 1955), ch. V: true (drik)
//!   positions, the Lahiri (Chitrapaksha) ayanamsa, the National Calendar; the rules the
//!   *Rashtriya Panchang* (Positional Astronomy Centre, Kolkata) is computed by.
//! * E. M. Reingold & N. Dershowitz, *Calendrical Calculations* (3rd ed., 2008), ch. 20
//!   "Indian Calendars": the algorithmic statement of the lunisolar and solar calendars.
//! * R. Sewell & S. B. Dikshit, *The Indian Calendar* (1896), §28: the regional rules
//!   for the first day of a solar month.
//! * Kashinatha Upadhyaya, *Dharmasindhu* (1790) and Kamalakara Bhatta, *Nirnaya Sindhu*
//!   (1612): which civil day keeps a festival when a tithi spans two days (the kala,
//!   or time-of-day, conditions).
//!
//! The civil day runs from sunrise to sunrise and is labelled by the local date of its
//! sunrise; everything is computed for the observer's place (observances can differ
//! by a day between places and between almanac conventions, which is why the place
//! and the sunrise convention are inputs).

#![forbid(unsafe_code)]

pub mod astro;
pub mod eras;
pub mod festivals;
pub mod lunar;
pub mod names;
pub mod solar;

use std::collections::HashMap;

pub use astro::{Sankranti, Span};
pub use eras::NationalDate;
pub use festivals::{festivals, Festival};
pub use lunar::{LunarMonth, MonthName};
pub use solar::{SolarDate, SolarRule};

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{EventError, RiseConvention};
use kshana_sky::Observer;
use kshana_time::{civil_from_days, days_from_civil, EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::Ayanamsa;

/// How the calendar is reckoned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Sidereal zodiac for month names, sankrantis and nakshatras (Lahiri per the CRC).
    pub ayanamsa: Ayanamsa,
    /// Which sunrise begins the civil day.
    pub rise: RiseConvention,
    /// The local civil time zone, minutes east of UTC (IST = 330), used only to label
    /// days with their local date.
    pub utc_offset_min: i32,
}

impl Config {
    /// Lahiri, the upper-limb refracted sunrise of Drik Panchang, and IST.
    pub fn india() -> Self {
        Config { ayanamsa: Ayanamsa::Lahiri, rise: RiseConvention::UpperLimbRefracted, utc_offset_min: 330 }
    }
}

/// A civil (Gregorian) date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year CE.
    pub year: i64,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
}

impl Date {
    /// A date.
    pub fn new(year: i64, month: u32, day: u32) -> Self {
        Date { year, month, day }
    }
    /// Days since 1970-01-01.
    pub fn days(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }
    /// From days since 1970-01-01.
    pub fn from_days(z: i64) -> Self {
        let (year, month, day) = civil_from_days(z);
        Date { year, month, day }
    }
    /// `n` days later.
    pub fn plus(&self, n: i64) -> Self {
        Date::from_days(self.days() + n)
    }
    /// 0 = Sunday.
    pub fn weekday(&self) -> u8 {
        (self.days() + 4).rem_euclid(7) as u8
    }
}

/// Errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Event search failure.
    Event(EventError),
    /// The Sun does not rise and set every day here (polar day or night).
    NoSunrise,
}

impl From<SpkError> for CalendarError {
    fn from(e: SpkError) -> Self {
        CalendarError::Spk(e)
    }
}
impl From<EventError> for CalendarError {
    fn from(e: EventError) -> Self {
        CalendarError::Event(e)
    }
}

/// One civil day: sunrise to the next sunrise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDay {
    /// Local date of the sunrise.
    pub date: Date,
    /// Sunrise.
    pub sunrise: Instant,
    /// Sunset.
    pub sunset: Instant,
    /// The next sunrise.
    pub next_sunrise: Instant,
}

impl CivilDay {
    /// Length of the daytime, ns.
    pub fn day_ns(&self) -> i128 {
        self.sunset.ns_since(self.sunrise)
    }
    /// Length of the night, ns.
    pub fn night_ns(&self) -> i128 {
        self.next_sunrise.ns_since(self.sunset)
    }
}

/// Everything the almanac prints for one civil day.
#[derive(Clone, Debug, PartialEq)]
pub struct DayInfo {
    /// The day.
    pub day: CivilDay,
    /// The tithi at sunrise and its span.
    pub tithi: Span,
    /// A tithi that begins and ends between this sunrise and the next (kshaya).
    pub tithi_skipped: Option<Span>,
    /// The sunrise tithi also held at the previous sunrise (vriddhi): this is its second day.
    pub tithi_repeated: bool,
    /// The Moon's nakshatra at sunrise and its span.
    pub star: Span,
    /// A nakshatra that begins and ends inside this civil day.
    pub star_skipped: Option<Span>,
    /// The sunrise nakshatra also held at the previous sunrise.
    pub star_repeated: bool,
    /// The amanta month containing the sunrise.
    pub month: LunarMonth,
    /// The purnimanta month name.
    pub purnimanta: MonthName,
    /// Bright fortnight.
    pub shukla: bool,
    /// Lunisolar Shaka year.
    pub shaka: i64,
    /// Vikram Samvat (Chaitradi).
    pub vikram: i64,
    /// Vikram Samvat (Kartikadi, Gujarat).
    pub vikram_kartikadi: i64,
    /// Kali Yuga year.
    pub kali: i64,
    /// Sixty-year name, South Indian reckoning (index into `names::SAMVATSARAS`).
    pub samvatsara_south: usize,
    /// Sixty-year name, North Indian (Jupiter) reckoning, as at the year's Chaitra.
    pub samvatsara_north: usize,
    /// Indian National Calendar date.
    pub national: NationalDate,
    /// Season by lunar month (index into `names::RITUS`).
    pub ritu: usize,
    /// Season by the Sun's tropical longitude.
    pub ritu_tropical: usize,
    /// Uttarayana as the almanacs count it (Makara to Mithuna sidereal).
    pub uttarayana: bool,
    /// Uttarayana by the solstices.
    pub uttarayana_tropical: bool,
    /// The Sun's sidereal sign at sunrise, 0 = Mesha.
    pub sun_sign: u8,
    /// Malayalam (Kollam era) date.
    pub malayalam: Option<SolarDate>,
    /// Tamil date.
    pub tamil: Option<SolarDate>,
    /// Bengali date.
    pub bengali: Option<SolarDate>,
    /// Odia date.
    pub odia: Option<SolarDate>,
}

/// A computed stretch of the calendar for one place.
pub struct Calendar<'a> {
    spk: &'a Spk,
    eop: Option<&'a EopTable>,
    /// The place.
    pub obs: Observer,
    /// The reckoning.
    pub cfg: Config,
    /// First requested date.
    pub from: Date,
    /// Number of requested days.
    pub count: u32,
    /// Civil days, padded a few days either side of the request.
    pub days: Vec<CivilDay>,
    /// Tithi spans covering the days.
    pub tithis: Vec<Span>,
    /// Nakshatra spans covering the days.
    pub stars: Vec<Span>,
    /// Amanta months covering the days.
    pub months: Vec<LunarMonth>,
    /// Sankrantis from a month before the request, with the civil date each solar
    /// month begins on under [Tamil, Kerala, Bengal, Odisha].
    pub solar_starts: Vec<(Sankranti, [Option<Date>; 4])>,
    north: HashMap<i64, usize>,
}

const PAD_DAYS: i64 = 4;

fn local_midnight(d: Date, offset_min: i32) -> Result<Instant, CalendarError> {
    let u = Utc::new(d.year, d.month, d.day, 0, 0, 0, 0).map_err(|_| CalendarError::NoSunrise)?;
    let i = u.to_instant().map_err(|_| CalendarError::NoSunrise)?;
    Ok(i.add_ns(-(offset_min as i128) * 60 * NS_PER_SEC))
}

fn local_date(t: Instant, offset_min: i32) -> Date {
    let u = Utc::from_instant(t.add_ns(offset_min as i128 * 60 * NS_PER_SEC));
    Date::new(u.year, u.month, u.day)
}

/// Fractional R.D. of an instant (UTC day count).
fn rd_of(t: Instant) -> f64 {
    let u = Utc::from_instant(t);
    u.utc_day() as f64
        + 719_163.0
        + (u.hour as f64 * 3_600.0 + u.minute as f64 * 60.0 + u.second as f64) / 86_400.0
}

impl<'a> Calendar<'a> {
    /// Compute `count` civil days from `from` at `obs`.
    pub fn new(
        spk: &'a Spk,
        eop: Option<&'a EopTable>,
        obs: Observer,
        cfg: Config,
        from: Date,
        count: u32,
    ) -> Result<Self, CalendarError> {
        let t0 = local_midnight(from.plus(-PAD_DAYS), cfg.utc_offset_min)?;
        let t1 = local_midnight(from.plus(count as i64 + PAD_DAYS), cfg.utc_offset_min)?;
        let days = civil_days(spk, eop, &obs, cfg, t0, t1)?;
        if days.len() < 2 {
            return Err(CalendarError::NoSunrise);
        }
        let tithis = astro::tithi_spans(spk, t0.add_ns(-2 * NS_PER_DAY), t1.add_ns(2 * NS_PER_DAY))?;
        let stars = astro::star_spans(spk, cfg.ayanamsa, t0.add_ns(-2 * NS_PER_DAY), t1.add_ns(2 * NS_PER_DAY))?;
        let nm = astro::new_moons(spk, t0.add_ns(-64 * NS_PER_DAY), t1.add_ns(34 * NS_PER_DAY))?;
        let mut marks = Vec::with_capacity(nm.len());
        for t in nm {
            let s = astro::sun_sidereal(spk, cfg.ayanamsa, t)?;
            marks.push((t, (s / 30.0).floor() as u8 % 12));
        }
        let months = lunar::months(&marks);
        let mut solar_starts = Vec::new();
        for s in astro::sankrantis(spk, cfg.ayanamsa, t0.add_ns(-40 * NS_PER_DAY), t1.add_ns(2 * NS_PER_DAY))? {
            let around = civil_days(spk, eop, &obs, cfg, s.at.add_ns(-36 * 3_600 * NS_PER_SEC), s.at.add_ns(84 * 3_600 * NS_PER_SEC))?;
            let mut starts = [None; 4];
            for (k, rule) in [SolarRule::Tamil, SolarRule::Kerala, SolarRule::Bengal, SolarRule::Odisha].into_iter().enumerate() {
                starts[k] = solar::month_start_day(&around, &s, rule).map(|i| around[i].date);
            }
            solar_starts.push((s, starts));
        }
        let mut cal = Calendar {
            spk,
            eop,
            obs,
            cfg,
            from,
            count,
            days,
            tithis,
            stars,
            months,
            solar_starts,
            north: HashMap::new(),
        };
        // North Indian samvatsara for each lunisolar year touched: the Jupiter year
        // current at the new Moon that begins Chaitra (the North Indian new year).
        let mut years: Vec<i64> = Vec::new();
        for i in 0..cal.days.len() {
            if let Some(m) = cal.month_at(cal.days[i].sunrise) {
                let d = cal.days[i].date;
                let y = eras::shaka_year(m.number, d.year, d.month) + 78;
                if !years.contains(&y) {
                    years.push(y);
                }
            }
        }
        for y in years {
            let idx = cal.chaitra_start(y)?.map(|t| eras::samvatsara_north(rd_of(t)));
            if let Some(i) = idx {
                cal.north.insert(y, i);
            }
        }
        Ok(cal)
    }

    /// The ephemeris.
    pub fn spk(&self) -> &'a Spk {
        self.spk
    }

    /// The new Moon that begins Chaitra of the lunisolar year starting in CE `y`: the
    /// first new Moon from February to April with the Sun in sidereal Meena.
    fn chaitra_start(&self, y: i64) -> Result<Option<Instant>, CalendarError> {
        let a = local_midnight(Date::new(y, 2, 10), 0)?;
        let b = local_midnight(Date::new(y, 4, 30), 0)?;
        for t in astro::new_moons(self.spk, a, b)? {
            let s = astro::sun_sidereal(self.spk, self.cfg.ayanamsa, t)?;
            if (s / 30.0).floor() as u8 % 12 == 11 {
                return Ok(Some(t));
            }
        }
        Ok(None)
    }

    /// The amanta month containing `t`.
    pub fn month_at(&self, t: Instant) -> Option<LunarMonth> {
        self.months.iter().copied().find(|m| m.start <= t && t < m.end)
    }

    /// The tithi span containing `t`.
    pub fn tithi_span_at(&self, t: Instant) -> Option<Span> {
        self.tithis.iter().copied().find(|s| s.contains(t))
    }

    /// The nakshatra span containing `t`.
    pub fn star_span_at(&self, t: Instant) -> Option<Span> {
        self.stars.iter().copied().find(|s| s.contains(t))
    }

    /// Index into `days` of a date.
    pub fn index_of(&self, d: Date) -> Option<usize> {
        self.days.iter().position(|x| x.date == d)
    }

    /// The requested days, in order.
    pub fn requested(&self) -> impl Iterator<Item = usize> + '_ {
        let (a, b) = (self.from, self.from.plus(self.count as i64));
        (0..self.days.len()).filter(move |&i| self.days[i].date >= a && self.days[i].date < b)
    }

    /// The solar date of `d` under `rule`.
    pub fn solar_date(&self, d: Date, rule: SolarRule) -> Option<SolarDate> {
        let k = rule_index(rule);
        let (s, start) = self
            .solar_starts
            .iter()
            .filter_map(|(s, st)| st[k].map(|x| (s, x)))
            .filter(|(_, x)| *x <= d)
            .last()?;
        Some(SolarDate {
            rule,
            sign: s.sign,
            day: (d.days() - start.days() + 1) as u16,
            year: solar::solar_year(rule, s.sign, d.year, d.month),
        })
    }

    /// The first day of the solar month of `sign` under `rule` whose sankranti falls
    /// in the computed stretch.
    pub fn solar_month_starts(&self, sign: u8, rule: SolarRule) -> Vec<(Sankranti, Date)> {
        let k = rule_index(rule);
        self.solar_starts.iter().filter(|(s, _)| s.sign == sign).filter_map(|(s, st)| st[k].map(|d| (*s, d))).collect()
    }

    /// Moonrise within a civil day (the first one after its sunrise), if any.
    pub fn moonrise(&self, day: &CivilDay) -> Result<Option<Instant>, CalendarError> {
        let (r, _) = astro::rise_set(self.spk, self.eop, &self.obs, Body::Moon, self.cfg.rise, day.sunrise, day.next_sunrise)?;
        Ok(r.first().copied())
    }

    /// The almanac entry for `days[i]`; `None` when it lies at the padded edge.
    pub fn info(&self, i: usize) -> Result<Option<DayInfo>, CalendarError> {
        let day = self.days[i];
        let (Some(tithi), Some(star), Some(month)) =
            (self.tithi_span_at(day.sunrise), self.star_span_at(day.sunrise), self.month_at(day.sunrise))
        else {
            return Ok(None);
        };
        let inside = |s: &&Span| s.start > day.sunrise && s.end < day.next_sunrise;
        let prev = i.checked_sub(1).map(|j| self.days[j].sunrise);
        let shukla = tithi.index <= 15;
        let d = day.date;
        let shaka = eras::shaka_year(month.number, d.year, d.month);
        let sun_sid = astro::sun_sidereal(self.spk, self.cfg.ayanamsa, day.sunrise)?;
        let sun_trop = astro::sun_tropical(self.spk, day.sunrise)?;
        let sun_sign = (sun_sid / 30.0).floor() as u8 % 12;
        Ok(Some(DayInfo {
            day,
            tithi,
            tithi_skipped: self.tithis.iter().find(inside).copied(),
            tithi_repeated: prev.is_some_and(|p| tithi.contains(p)),
            star,
            star_skipped: self.stars.iter().find(inside).copied(),
            star_repeated: prev.is_some_and(|p| star.contains(p)),
            month,
            purnimanta: lunar::purnimanta(&month, tithi.index),
            shukla,
            shaka,
            vikram: eras::vikram_chaitradi(shaka),
            vikram_kartikadi: eras::vikram_kartikadi(shaka, month.number),
            kali: eras::kali_year(shaka),
            samvatsara_south: eras::samvatsara_south(shaka),
            samvatsara_north: self
                .north
                .get(&(shaka + 78))
                .copied()
                .unwrap_or_else(|| eras::samvatsara_north(rd_of(day.sunrise))),
            national: eras::national(d.year, d.month, d.day),
            ritu: eras::ritu_lunar(month.number),
            ritu_tropical: eras::ritu_tropical(sun_trop),
            uttarayana: eras::uttarayana_sidereal(sun_sign),
            uttarayana_tropical: eras::uttarayana_tropical(sun_trop),
            sun_sign,
            malayalam: self.solar_date(d, SolarRule::Kerala),
            tamil: self.solar_date(d, SolarRule::Tamil),
            bengali: self.solar_date(d, SolarRule::Bengal),
            odia: self.solar_date(d, SolarRule::Odisha),
        }))
    }
}

fn rule_index(rule: SolarRule) -> usize {
    match rule {
        SolarRule::Tamil => 0,
        SolarRule::Kerala => 1,
        SolarRule::Bengal => 2,
        SolarRule::Odisha => 3,
    }
}

/// Civil days whose sunrises fall in `[from, to]`, each closed by its next sunrise.
fn civil_days(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    cfg: Config,
    from: Instant,
    to: Instant,
) -> Result<Vec<CivilDay>, CalendarError> {
    let (rises, sets) = astro::rise_set(spk, eop, obs, Body::Sun, cfg.rise, from, to)?;
    let mut out = Vec::new();
    for w in rises.windows(2) {
        let Some(sunset) = sets.iter().copied().find(|s| *s > w[0] && *s < w[1]) else {
            continue;
        };
        out.push(CivilDay { date: local_date(w[0], cfg.utc_offset_min), sunrise: w[0], sunset, next_sunrise: w[1] });
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod testkit {
    use super::*;

    pub fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    pub const DELHI: Observer = Observer { latitude_deg: 28.6139, longitude_deg: 77.2090, height_m: 0.0 };
    pub const KOCHI: Observer = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
    pub const CHENNAI: Observer = Observer { latitude_deg: 13.0827, longitude_deg: 80.2707, height_m: 0.0 };
    pub const MUMBAI: Observer = Observer { latitude_deg: 19.0760, longitude_deg: 72.8777, height_m: 0.0 };
    pub const KOLKATA: Observer = Observer { latitude_deg: 22.5726, longitude_deg: 88.3639, height_m: 0.0 };
}

#[cfg(test)]
mod tests {
    use super::testkit::*;
    use super::*;

    fn info_on(cal: &Calendar, d: Date) -> DayInfo {
        cal.info(cal.index_of(d).unwrap()).unwrap().unwrap()
    }

    #[test]
    fn adhika_jyeshtha_2026_and_both_reckonings() {
        let Some(spk) = kernel() else { return };
        let cal = Calendar::new(&spk, None, DELHI, Config::india(), Date::new(2026, 5, 1), 60).unwrap();
        // 1 June 2026 lies in Adhika Jyeshtha (17 May – 15 June 2026).
        let i = info_on(&cal, Date::new(2026, 6, 1));
        assert_eq!(i.month.number, 3, "{:?}", i.month);
        assert!(i.month.adhika);
        assert_eq!(i.purnimanta.label(), "Adhika Jyeshtha");
        // 10 May 2026 (dark half of amanta Vaishakha) is nija Jyeshtha by purnimanta.
        let j = info_on(&cal, Date::new(2026, 5, 10));
        assert_eq!((j.month.number, j.month.adhika, j.shukla), (2, false, false));
        assert_eq!(j.purnimanta.label(), "Jyeshtha");
        // The month after the adhika is nija Jyeshtha.
        let k = info_on(&cal, Date::new(2026, 6, 25));
        assert_eq!((k.month.number, k.month.adhika), (3, false));
        // No other month in the stretch is adhika or kshaya.
        assert_eq!(cal.months.iter().filter(|m| m.adhika).count(), 1);
        assert!(cal.months.iter().all(|m| m.kshaya_skipped.is_none()));
    }

    #[test]
    fn adhika_shravana_2023() {
        let Some(spk) = kernel() else { return };
        let cal = Calendar::new(&spk, None, DELHI, Config::india(), Date::new(2023, 8, 1), 3).unwrap();
        let i = info_on(&cal, Date::new(2023, 8, 1));
        assert_eq!((i.month.number, i.month.adhika), (5, true), "{:?}", i.month);
    }

    #[test]
    fn eras_and_cycles_through_a_year() {
        let Some(spk) = kernel() else { return };
        let cal = Calendar::new(&spk, None, MUMBAI, Config::india(), Date::new(2026, 3, 15), 10).unwrap();
        let before = info_on(&cal, Date::new(2026, 3, 18));
        let after = info_on(&cal, Date::new(2026, 3, 21));
        assert_eq!((before.month.number, before.shaka, before.vikram), (12, 1947, 2082));
        assert_eq!((after.month.number, after.shaka, after.vikram), (1, 1948, 2083));
        assert_eq!(names::SAMVATSARAS[before.samvatsara_south], "Vishvavasu");
        assert_eq!(names::SAMVATSARAS[after.samvatsara_south], "Parabhava");
        assert_eq!(names::SAMVATSARAS[before.samvatsara_north], "Siddharthi");
        assert_eq!(names::SAMVATSARAS[after.samvatsara_north], "Raudra");
        assert_eq!(after.vikram_kartikadi, 2082);
        assert_eq!(names::RITUS[after.ritu], "Vasanta");
        assert!(after.uttarayana && after.uttarayana_tropical);
        assert_eq!(after.national, NationalDate { year: 1947, month: 11, day: 30 });
        // 19 March 2026 at Mumbai: Amavasya at sunrise (06:43 IST) ends at 06:53 and
        // Shukla Pratipada ends at 04:52 next morning, before sunrise: a skipped tithi,
        // which is why Ugadi / Gudi Padwa is kept on the 19th (published times).
        let u = info_on(&cal, Date::new(2026, 3, 19));
        assert_eq!(u.tithi.index, 30);
        assert_eq!(u.tithi_skipped.map(|s| s.index), Some(1));
        assert_eq!(info_on(&cal, Date::new(2026, 3, 20)).tithi.index, 2);
    }

    #[test]
    fn solar_calendars_on_new_year_days() {
        let Some(spk) = kernel() else { return };
        let cal = Calendar::new(&spk, None, KOCHI, Config::india(), Date::new(2025, 8, 10), 20).unwrap();
        // Chingam 1, Kollam 1201 = 17 August 2025.
        let c = info_on(&cal, Date::new(2025, 8, 17)).malayalam.unwrap();
        assert_eq!((c.month_name(), c.day, c.year), ("Chingam", 1, 1201));
        let k = info_on(&cal, Date::new(2025, 8, 16)).malayalam.unwrap();
        assert_eq!((k.month_name(), k.year), ("Karkidakam", 1200));
        // Mesha 2026 (sankranti on 14 April, mid-morning IST): Vishu / Medam 1 and
        // Puthandu / Chithirai 1 on the 14th, Pohela Boishakh on the 15th (Kolkata).
        let cal = Calendar::new(&spk, None, KOLKATA, Config::india(), Date::new(2026, 4, 12), 6).unwrap();
        let b = info_on(&cal, Date::new(2026, 4, 15)).bengali.unwrap();
        assert_eq!((b.month_name(), b.day, b.year), ("Boishakh", 1, 1433));
        let b0 = info_on(&cal, Date::new(2026, 4, 14)).bengali.unwrap();
        assert_eq!(b0.month_name(), "Choitro");
        let o = info_on(&cal, Date::new(2026, 4, 14)).odia.unwrap();
        assert_eq!((o.month_name(), o.day), ("Baisakha", 1));
        let cal = Calendar::new(&spk, None, CHENNAI, Config::india(), Date::new(2026, 4, 12), 6).unwrap();
        let t = info_on(&cal, Date::new(2026, 4, 14)).tamil.unwrap();
        assert_eq!((t.month_name(), t.day), ("Chithirai", 1));
        assert_eq!(names::SAMVATSARAS[t.year as usize], "Parabhava");
    }

    #[test]
    fn skipped_and_repeated_tithis_balance() {
        let Some(spk) = kernel() else { return };
        let cal = Calendar::new(&spk, None, DELHI, Config::india(), Date::new(2025, 1, 1), 365).unwrap();
        let infos: Vec<DayInfo> = cal.requested().filter_map(|i| cal.info(i).unwrap()).collect();
        assert_eq!(infos.len(), 365);
        let (mut skipped, mut repeated) = (0, 0);
        for w in infos.windows(2) {
            let step = (w[1].tithi.index as i32 - w[0].tithi.index as i32).rem_euclid(30);
            assert!(step <= 2, "{:?} → {:?}", w[0].day.date, w[1].day.date);
            match step {
                0 => assert!(w[1].tithi_repeated),
                2 => {
                    let s = w[0].tithi_skipped.expect("a skipped tithi");
                    assert_eq!(s.index, w[0].tithi.index % 30 + 1);
                }
                _ => assert!(w[0].tithi_skipped.is_none() && !w[1].tithi_repeated),
            }
            skipped += usize::from(w[0].tithi_skipped.is_some());
            repeated += usize::from(w[1].tithi_repeated);
        }
        // About 371 tithis pass in 365 days (365 × 30 / 29.53), so skipped days
        // outnumber repeated ones by about six.
        let net = skipped as i32 - repeated as i32;
        assert!((4..=8).contains(&net), "skipped {skipped}, repeated {repeated}");
        assert!(skipped >= 4 && repeated >= 1);
        // Nakshatras: 27 per sidereal month, about 361 a year, so repeats outnumber skips.
        let ss = infos.iter().filter(|i| i.star_skipped.is_some()).count() as i32;
        let sr = infos.iter().filter(|i| i.star_repeated).count() as i32;
        assert!((sr - ss - 4).abs() <= 2, "stars skipped {ss}, repeated {sr}");
    }
}
