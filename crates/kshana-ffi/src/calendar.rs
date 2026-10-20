//! The Hindu calendar over the FFI: the almanac day (lunar month by both reckonings,
//! fortnight, tithi and nakshatra at sunrise with skipped and repeated ones, the
//! extra month, eras and the sixty-year names, the regional solar dates) and the
//! festival calendar for a place.
//!
//! What the person reads: festival `name` and `what`, the month names, `fortnight`,
//! `season`, `weekday`, the solar and national dates. Everything else is the expert
//! layer (shown only behind the chart-pages setting): `tithi`, `nakshatra`,
//! `samvatsara*`, `ritu*`, `ayana*`, `rule`, `kind`, `key` and the skipped/repeated
//! flags (see `plain_audit` below).

use kshana_calendar::names::{RITUS, RITUS_PLAIN, SAMVATSARAS, WEEKDAYS};
use kshana_calendar::{names, Calendar, Config, Date, DayInfo, Festival, SolarRule};
use kshana_events::rise_convention;
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_SEC};

use crate::{AyanamsaChoice, Engine, ExactInstant, FestivalRecord, HouseSystem, KshanaError};

/// A date of a regional solar calendar.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SolarDateRecord {
    /// "malayalam", "tamil", "bengali" or "odia".
    pub calendar: String,
    pub month: String,
    pub day: u16,
    /// Kollam era (Malayalam), Bangabda (Bengali), CE year of the solar new year (Odia);
    /// for Tamil, see `year_name`.
    pub year: i64,
    /// The sixty-year name (Tamil), else empty.
    pub year_name: String,
}

/// One almanac day.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct CalendarDayRecord {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub weekday: String,
    pub sunrise: ExactInstant,
    pub sunset: ExactInstant,
    pub next_sunrise: ExactInstant,
    /// Amanta month (new Moon to new Moon), e.g. "Jyeshtha" or "Adhika Jyeshtha".
    pub lunar_month: String,
    /// 1 = Chaitra.
    pub lunar_month_number: u8,
    /// Purnimanta month (North India).
    pub lunar_month_north: String,
    /// The intercalary month (adhika masa).
    pub extra_month: bool,
    /// "waxing" or "waning".
    pub fortnight: String,
    /// Expert: the tithi at sunrise, e.g. "Shukla Ekadashi".
    pub tithi: String,
    /// 1–30.
    pub tithi_number: u8,
    pub tithi_ends: ExactInstant,
    /// Expert: a tithi that begins and ends inside this day (kshaya).
    pub tithi_skipped: Option<String>,
    /// Expert: the sunrise tithi also held yesterday's sunrise (vriddhi).
    pub tithi_repeated: bool,
    /// Expert: the Moon's nakshatra at sunrise.
    pub nakshatra: String,
    pub nakshatra_ends: ExactInstant,
    pub nakshatra_skipped: Option<String>,
    pub nakshatra_repeated: bool,
    /// Expert: the sixty-year name, South Indian reckoning.
    pub samvatsara: String,
    /// Expert: the sixty-year name, North Indian (Jupiter) reckoning.
    pub samvatsara_north: String,
    pub shaka: i64,
    pub vikram: i64,
    /// Vikram Samvat as Gujarat counts it (from the day after Diwali).
    pub vikram_gujarati: i64,
    pub kali: i64,
    /// Indian National Calendar, e.g. "Chaitra 1, 1948".
    pub national_date: String,
    /// Plain season, e.g. "spring".
    pub season: String,
    /// Expert: the season by lunar month and by the solar tropical year.
    pub ritu: String,
    pub ritu_tropical: String,
    /// Expert: "Uttarayana" or "Dakshinayana", almanac count and by the solstices.
    pub ayana: String,
    pub ayana_tropical: String,
    pub solar: Vec<SolarDateRecord>,
    /// Names of the festivals kept this day.
    pub festivals: Vec<String>,
}

/// A festival on a civil day.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct CalendarFestivalRecord {
    pub key: String,
    pub name: String,
    pub year: i64,
    pub month: u32,
    pub day: u32,
    /// Sunrise of the day it is kept.
    pub day_start: ExactInstant,
    pub day_end: ExactInstant,
    /// When the governing tithi, star or solar month begins.
    pub begins: ExactInstant,
    pub ends: Option<ExactInstant>,
    /// "tithi", "sun" or "star".
    pub kind: String,
    pub what: String,
    /// Expert: the rule that fixed the day.
    pub rule: String,
}

fn ayana(uttara: bool) -> String {
    if uttara { "Uttarayana" } else { "Dakshinayana" }.into()
}

fn solar_record(d: &kshana_calendar::SolarDate) -> SolarDateRecord {
    let (calendar, year_name) = match d.rule {
        SolarRule::Kerala => ("malayalam", String::new()),
        SolarRule::Tamil => ("tamil", SAMVATSARAS[d.year as usize % 60].to_string()),
        SolarRule::Bengal => ("bengali", String::new()),
        SolarRule::Odisha => ("odia", String::new()),
    };
    SolarDateRecord { calendar: calendar.into(), month: d.month_name().into(), day: d.day, year: d.year, year_name }
}

fn day_record(i: &DayInfo, festivals: &[Festival]) -> CalendarDayRecord {
    let d = i.day.date;
    CalendarDayRecord {
        year: d.year,
        month: d.month,
        day: d.day,
        weekday: WEEKDAYS[d.weekday() as usize].into(),
        sunrise: i.day.sunrise.into(),
        sunset: i.day.sunset.into(),
        next_sunrise: i.day.next_sunrise.into(),
        lunar_month: kshana_calendar::MonthName { number: i.month.number, adhika: i.month.adhika }.label(),
        lunar_month_number: i.month.number,
        lunar_month_north: i.purnimanta.label(),
        extra_month: i.month.adhika,
        fortnight: if i.shukla { "waxing" } else { "waning" }.into(),
        tithi: names::tithi_full_name(i.tithi.index),
        tithi_number: i.tithi.index,
        tithi_ends: i.tithi.end.into(),
        tithi_skipped: i.tithi_skipped.map(|s| names::tithi_full_name(s.index)),
        tithi_repeated: i.tithi_repeated,
        nakshatra: names::star_name(i.star.index).into(),
        nakshatra_ends: i.star.end.into(),
        nakshatra_skipped: i.star_skipped.map(|s| names::star_name(s.index).to_string()),
        nakshatra_repeated: i.star_repeated,
        samvatsara: SAMVATSARAS[i.samvatsara_south].into(),
        samvatsara_north: SAMVATSARAS[i.samvatsara_north].into(),
        shaka: i.shaka,
        vikram: i.vikram,
        vikram_gujarati: i.vikram_kartikadi,
        kali: i.kali,
        national_date: format!("{} {}, {}", i.national.month_name(), i.national.day, i.national.year),
        season: RITUS_PLAIN[i.ritu].into(),
        ritu: RITUS[i.ritu].into(),
        ritu_tropical: RITUS[i.ritu_tropical].into(),
        ayana: ayana(i.uttarayana),
        ayana_tropical: ayana(i.uttarayana_tropical),
        solar: [i.malayalam, i.tamil, i.bengali, i.odia].iter().flatten().map(solar_record).collect(),
        festivals: festivals.iter().filter(|f| f.date == d).map(|f| f.name.clone()).collect(),
    }
}

fn festival_out(f: &Festival) -> CalendarFestivalRecord {
    CalendarFestivalRecord {
        key: f.key.into(),
        name: f.name.clone(),
        year: f.date.year,
        month: f.date.month,
        day: f.date.day,
        day_start: f.day_start.into(),
        day_end: f.day_end.into(),
        begins: f.begins.into(),
        ends: f.ends.map(Into::into),
        kind: f.kind.into(),
        what: f.what.into(),
        rule: f.rule.clone(),
    }
}

fn config(ayanamsa: AyanamsaChoice, utc_offset_minutes: i32) -> Config {
    Config { ayanamsa: ayanamsa.into(), rise: rise_convention(), utc_offset_min: utc_offset_minutes.clamp(-14 * 60, 14 * 60) }
}

fn engine_err(e: kshana_calendar::CalendarError) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

fn local_date(at: Instant, utc_offset_minutes: i32) -> Date {
    let u = Utc::from_instant(at.add_ns(utc_offset_minutes as i128 * 60 * NS_PER_SEC));
    Date::new(u.year, u.month, u.day)
}

#[uniffi::export]
impl Engine {
    /// The almanac for `days` civil days (1–400) from the local date `year-month-day`
    /// at a place. `utc_offset_minutes` labels days by local date (IST = 330). The
    /// sunrise follows `set_rise_convention`.
    #[allow(clippy::too_many_arguments)]
    pub fn calendar_days(
        &self,
        lat: f64,
        lon: f64,
        utc_offset_minutes: i32,
        year: i64,
        month: u32,
        day: u32,
        days: u32,
        ayanamsa: AyanamsaChoice,
    ) -> Result<Vec<CalendarDayRecord>, KshanaError> {
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let cal = Calendar::new(&self.spk, self.eop.as_ref(), obs, config(ayanamsa, utc_offset_minutes), Date::new(year, month, day), days.clamp(1, 400))
            .map_err(engine_err)?;
        let fest = kshana_calendar::festivals(&cal).map_err(engine_err)?;
        let mut out = Vec::new();
        for i in cal.requested() {
            if let Some(info) = cal.info(i).map_err(engine_err)? {
                out.push(day_record(&info, &fest));
            }
        }
        Ok(out)
    }

    /// The festivals and monthly observances kept at a place in `days` civil days
    /// (1–400) from the local date `year-month-day`, in date order.
    #[allow(clippy::too_many_arguments)]
    pub fn calendar_festivals(
        &self,
        lat: f64,
        lon: f64,
        utc_offset_minutes: i32,
        year: i64,
        month: u32,
        day: u32,
        days: u32,
        ayanamsa: AyanamsaChoice,
    ) -> Result<Vec<CalendarFestivalRecord>, KshanaError> {
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let cal = Calendar::new(&self.spk, self.eop.as_ref(), obs, config(ayanamsa, utc_offset_minutes), Date::new(year, month, day), days.clamp(1, 400))
            .map_err(engine_err)?;
        Ok(kshana_calendar::festivals(&cal).map_err(engine_err)?.iter().map(festival_out).collect())
    }

    /// `festivals(...)` for a place: the same observances (Ekadashi, Pradosham, the
    /// person's own star day …) plus the year's named festivals as kept at `lat, lon`
    /// (Diwali, Onam, Vishu, Holi, …; kind "festival", `at` = the sunrise of the day
    /// it is kept, `end` = the next sunrise). Existing callers of `festivals` are
    /// unchanged; the countdown wall can switch to this when it knows the place.
    #[allow(clippy::too_many_arguments)]
    pub fn festivals_here(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        lat: f64,
        lon: f64,
        utc_offset_minutes: i32,
        at: ExactInstant,
        days: u32,
    ) -> Result<Vec<FestivalRecord>, KshanaError> {
        let mut out = self.festivals(birth, birth_lat, birth_lon, system, ayanamsa, at, days)?;
        let at_i: Instant = at.into();
        let days = days.clamp(1, 400);
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let cal = Calendar::new(&self.spk, self.eop.as_ref(), obs, config(ayanamsa, utc_offset_minutes), local_date(at_i, utc_offset_minutes), days + 1)
            .map_err(engine_err)?;
        let end = at_i.add_ns(days as i128 * kshana_time::NS_PER_DAY);
        for f in kshana_calendar::festivals(&cal).map_err(engine_err)? {
            if kshana_calendar::festivals::MONTHLY_KEYS.contains(&f.key) || f.day_end <= at_i || f.day_start > end {
                continue;
            }
            out.push(FestivalRecord {
                name: f.name.clone(),
                at: f.day_start.into(),
                end: Some(f.day_end.into()),
                what: f.what.into(),
                kind: "festival".into(),
            });
        }
        let key = |r: &FestivalRecord| Instant::from(r.at.clone());
        out.sort_by_key(key);
        Ok(out)
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    /// Expert-only fields of these records (never on the common screens).
    const NOT_SHOWN: &[&str] = &[
        "tithi", "tithi_skipped", "nakshatra", "nakshatra_skipped", "samvatsara", "samvatsara_north", "ritu",
        "ritu_tropical", "ayana", "ayana_tropical", "rule", "kind", "key", "calendar",
    ];

    fn engine() -> std::sync::Arc<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Engine::open(p.to_string_lossy().into_owned(), None).expect("the kernel is needed for this test")
    }

    fn check(what: &str, field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(kshana_rules::Lang::En, v) {
            hits.push(format!("{what}.{field} [{w}]: {v}"));
        }
    }

    #[test]
    fn calendar_text_names_no_astrology() {
        let e = engine();
        let mut hits = Vec::new();
        // A year at Delhi and at Kochi covers every festival rule and every month name.
        for (lat, lon) in [(28.6139, 77.2090), (9.9312, 76.2673)] {
            for f in e.calendar_festivals(lat, lon, 330, 2025, 1, 1, 366, AyanamsaChoice::Lahiri).unwrap() {
                check("festival", "name", &f.name, &mut hits);
                check("festival", "what", &f.what, &mut hits);
                check("festival", "rule", &f.rule, &mut hits);
                assert!(!f.what.trim().is_empty());
            }
        }
        for d in e.calendar_days(9.9312, 76.2673, 330, 2026, 5, 10, 40, AyanamsaChoice::Lahiri).unwrap() {
            for (field, v) in [
                ("weekday", d.weekday.as_str()),
                ("lunar_month", &d.lunar_month),
                ("lunar_month_north", &d.lunar_month_north),
                ("fortnight", &d.fortnight),
                ("national_date", &d.national_date),
                ("season", &d.season),
                ("tithi", &d.tithi),
                ("nakshatra", &d.nakshatra),
                ("samvatsara", &d.samvatsara),
            ] {
                check("day", field, v, &mut hits);
            }
            for s in &d.solar {
                check("day", "month", &s.month, &mut hits);
            }
            for f in &d.festivals {
                check("day", "festivals", f, &mut hits);
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }

    #[test]
    fn festivals_here_extends_festivals() {
        let e = engine();
        let at = instant_from_utc(UtcFields { year: 2025, month: 10, day: 1, hour: 0, minute: 0, second: 0, nanos: 0 }).unwrap();
        let b = instant_from_utc(UtcFields { year: 1990, month: 2, day: 20, hour: 5, minute: 4, second: 0, nanos: 0 }).unwrap();
        let old = e.festivals(b.clone(), 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, at.clone(), 35).unwrap();
        let new = e
            .festivals_here(b, 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 28.6139, 77.2090, 330, at, 35)
            .unwrap();
        // Everything the old call returned is still there.
        for o in &old {
            assert!(new.iter().any(|n| n.name == o.name && n.at.tai_seconds == o.at.tai_seconds), "{}", o.name);
        }
        let added: Vec<&str> = new.iter().filter(|n| n.kind == "festival").map(|n| n.name.as_str()).collect();
        assert!(added.contains(&"Dussehra / Vijayadashami"), "{added:?}");
        assert!(added.contains(&"Karva Chauth") && added.contains(&"Diwali (Lakshmi Puja)"), "{added:?}");
        let mut hits = Vec::new();
        for n in &new {
            check("festivals_here", "name", &n.name, &mut hits);
            check("festivals_here", "what", &n.what, &mut hits);
        }
        assert!(hits.is_empty(), "{hits:#?}");
        assert!(new.windows(2).all(|w| Instant::from(w[0].at.clone()) <= Instant::from(w[1].at.clone())));
    }

    #[test]
    fn the_almanac_day_over_the_ffi() {
        let e = engine();
        // 1 June 2026 at Delhi: Adhika Jyeshtha; Raudra (North) / Parabhava (South).
        let d = &e.calendar_days(28.6139, 77.2090, 330, 2026, 6, 1, 1, AyanamsaChoice::Lahiri).unwrap()[0];
        assert_eq!((d.year, d.month, d.day), (2026, 6, 1));
        assert_eq!(d.lunar_month, "Adhika Jyeshtha");
        assert!(d.extra_month);
        assert_eq!((d.samvatsara.as_str(), d.samvatsara_north.as_str()), ("Parabhava", "Raudra"));
        assert_eq!((d.shaka, d.vikram), (1948, 2083));
        assert!(d.solar.iter().any(|s| s.calendar == "malayalam" && s.month == "Edavam" && s.year == 1201));
    }
}
