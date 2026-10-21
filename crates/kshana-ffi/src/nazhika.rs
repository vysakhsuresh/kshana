//! Birth time given in nazhika over the FFI, and the height of a place for sunrise.
//!
//! Many Kerala and Tamil horoscopes write the birth as nazhika and vinazhika after
//! sunrise: a nazhika is 24 minutes and a vinazhika 24 seconds, sixty nazhika to the
//! day from one sunrise to the next. Most count from the actual sunrise of that day at
//! the birth place, some from six in the morning by the clock.
//!
//! The sunrise is the engine's true sunrise at the birth place under the process-wide
//! rising convention (`set_rise_convention`: the Hindu rising, the Sun's centre on the
//! geometric horizon, as the app sets by default; or the visible rising, the Sun's centre
//! at −0°50′, which is the upper limb on the horizon with standard refraction), lowered
//! by the dip of the horizon for the place's height above sea level
//! (`kshana_events::horizon`). The count uses the fixed units (24 minutes, 24 seconds),
//! as the almanacs do.
//!
//! A horoscope's day runs from sunrise to the next sunrise, so a birth after midnight and
//! before sunrise belongs to the day that began at the previous date's sunrise:
//! `previous_day` counts from that sunrise while the date stays the birth date by the
//! clock.
//!
//! Height: `set_place_elevation(lat, lon, m)` records a place's height once; every
//! sunrise-based clock at those coordinates (day, hours, unlucky stretches, remedy
//! timing, the birth day's clocks) then follows it with no change to their functions.

use kshana_events::sun_rise_set;
use kshana_sky::Observer;
use kshana_time::{civil_from_days, days_from_civil, Instant, Utc, NS_PER_SEC};

use crate::{Engine, ExactInstant, KshanaError};

const NAZHIKA_S: i64 = 24 * 60;
const VINAZHIKA_S: i64 = 24;

/// A clock time worked out from a nazhika count.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct NazhikaTimeRecord {
    /// The birth instant.
    pub at: ExactInstant,
    /// What the count starts from: the sunrise, or six in the morning by the clock.
    pub start: ExactInstant,
    /// Sunrise, sunset and the next sunrise of the day the count starts on (absent where
    /// the Sun does not rise that day).
    pub sunrise: Option<ExactInstant>,
    pub sunset: Option<ExactInstant>,
    pub next_sunrise: Option<ExactInstant>,
    /// Sunrise to sunset, in nazhika (about 30 at the equinox).
    pub daytime_nazhika: f64,
    /// Sunrise to the next sunrise, in nazhika (60, give or take a few vinazhika).
    pub whole_day_nazhika: f64,
    /// The count runs past sunset (a night birth).
    pub after_sunset: bool,
    /// The count runs past the next sunrise: more than the whole day holds.
    pub past_next_sunrise: bool,
    /// One plain sentence: what the count was taken from.
    pub note: String,
}

/// A clock time read back as a nazhika count.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct NazhikaCountRecord {
    pub nazhika: u32,
    pub vinazhika: u32,
    /// Fractional sixtieths of a vinazhika.
    pub vilipta: f64,
    /// What the count starts from.
    pub start: ExactInstant,
    /// The count starts on the date before the clock date (a birth after midnight, before sunrise).
    pub previous_day: bool,
    /// One plain sentence: what the count was taken from.
    pub note: String,
}

fn err<E: std::fmt::Debug>(e: E) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

fn observer(latitude_deg: f64, longitude_deg: f64, elevation_m: f64) -> Observer {
    Observer {
        latitude_deg,
        longitude_deg,
        height_m: if elevation_m.is_finite() { elevation_m.max(0.0) } else { 0.0 },
    }
}

/// Local midnight of a civil date at a fixed UTC offset.
fn local_midnight(days: i64, utc_offset_seconds: i32) -> Result<Instant, KshanaError> {
    let (y, m, d) = civil_from_days(days);
    let u = Utc::new(y, m, d, 0, 0, 0, 0).map_err(|e| KshanaError::InvalidUtc(format!("{e:?}")))?;
    let i = u.to_instant().map_err(|e| KshanaError::InvalidUtc(format!("{e:?}")))?;
    Ok(i.add_ns(-(utc_offset_seconds as i128) * NS_PER_SEC))
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December",
];

/// "6:21:05 am on 5 January 1990" in local clock time.
fn local_words(i: Instant, utc_offset_seconds: i32) -> String {
    let u = Utc::from_instant(i.add_ns(utc_offset_seconds as i128 * NS_PER_SEC));
    let (h12, ap) = match u.hour {
        0 => (12, "am"),
        1..=11 => (u.hour, "am"),
        12 => (12, "pm"),
        h => (h - 12, "pm"),
    };
    format!("{h12}:{:02}:{:02} {ap} on {} {} {}", u.minute, u.second, u.day, MONTHS[(u.month as usize).saturating_sub(1) % 12], u.year)
}

/// Sunrise of a local date, then the sunset and the next sunrise after it.
fn day_of(
    engine: &Engine,
    obs: &Observer,
    midnight: Instant,
) -> Result<Option<(Instant, Instant, Instant)>, KshanaError> {
    let from = midnight.add_ns(-60 * NS_PER_SEC);
    let to = midnight.add_ns(52 * 3_600 * NS_PER_SEC);
    let (rises, sets) = sun_rise_set(&engine.spk, engine.eop.as_ref(), obs, from, to, 1_000_000).map_err(err)?;
    let Some(&rise) = rises.iter().find(|r| r.ns_since(midnight) < 24 * 3_600 * NS_PER_SEC) else {
        return Ok(None);
    };
    let set = sets.iter().copied().find(|s| *s > rise);
    let next = rises.iter().copied().find(|r| *r > rise);
    Ok(match (set, next) {
        (Some(s), Some(n)) => Some((rise, s, n)),
        _ => None,
    })
}

fn nazhika_of(ns: i128) -> f64 {
    ns as f64 / (NAZHIKA_S as f64 * 1e9)
}

#[uniffi::export]
impl Engine {
    /// The birth instant for a horoscope that gives the time as `nazhika` and `vinazhika`
    /// (0–59) counted from the sunrise (`from_sunrise`) or from six in the morning by the
    /// clock, on the civil date `year-month-day` at a place. `utc_offset_seconds` is the
    /// clock's offset from UTC on that date (IST = 19 800; Bombay Time = 17 460).
    /// `previous_day` counts from the date before (a birth after midnight and before
    /// sunrise, whose horoscope day began at the previous sunrise). `elevation_m` is the
    /// height above sea level (0 when unknown: a height registered for the place by
    /// `set_place_elevation` is then used).
    #[allow(clippy::too_many_arguments)]
    pub fn nazhika_to_instant(
        &self,
        year: i64,
        month: u32,
        day: u32,
        utc_offset_seconds: i32,
        latitude_deg: f64,
        longitude_deg: f64,
        elevation_m: f64,
        nazhika: u32,
        vinazhika: u32,
        from_sunrise: bool,
        previous_day: bool,
    ) -> Result<NazhikaTimeRecord, KshanaError> {
        if nazhika > 60 || vinazhika > 59 || (nazhika == 60 && vinazhika > 0) {
            return Err(KshanaError::InvalidUtc("nazhika runs 0–60 and vinazhika 0–59".into()));
        }
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(KshanaError::InvalidUtc("no such date".into()));
        }
        let obs = observer(latitude_deg, longitude_deg, elevation_m);
        let days = days_from_civil(year, month, day) - i64::from(previous_day);
        let midnight = local_midnight(days, utc_offset_seconds)?;
        let sun = day_of(self, &obs, midnight)?;
        let start = if from_sunrise {
            match sun {
                Some((rise, _, _)) => rise,
                None => return Err(KshanaError::Engine("the Sun does not rise at this place on that date".into())),
            }
        } else {
            midnight.add_ns(6 * 3_600 * NS_PER_SEC)
        };
        let count_s = i64::from(nazhika) * NAZHIKA_S + i64::from(vinazhika) * VINAZHIKA_S;
        let at = start.add_ns(count_s as i128 * NS_PER_SEC);
        let note = if from_sunrise {
            format!("Counted from the sunrise at {}, at this place.", local_words(start, utc_offset_seconds))
        } else {
            format!("Counted from {} by the clock.", local_words(start, utc_offset_seconds))
        };
        Ok(NazhikaTimeRecord {
            at: at.into(),
            start: start.into(),
            sunrise: sun.map(|s| s.0.into()),
            sunset: sun.map(|s| s.1.into()),
            next_sunrise: sun.map(|s| s.2.into()),
            daytime_nazhika: sun.map_or(0.0, |(r, s, _)| nazhika_of(s.ns_since(r))),
            whole_day_nazhika: sun.map_or(60.0, |(r, _, n)| nazhika_of(n.ns_since(r))),
            after_sunset: sun.is_some_and(|(_, s, _)| at >= s),
            past_next_sunrise: sun.is_some_and(|(_, _, n)| at >= n),
            note,
        })
    }

    /// The nazhika count of an instant at a place: from the sunrise that began its day
    /// (`from_sunrise`) or from the last six in the morning by the clock. `previous_day`
    /// in the answer says the count starts on the date before the clock date.
    pub fn instant_to_nazhika(
        &self,
        at: ExactInstant,
        utc_offset_seconds: i32,
        latitude_deg: f64,
        longitude_deg: f64,
        elevation_m: f64,
        from_sunrise: bool,
    ) -> Result<NazhikaCountRecord, KshanaError> {
        let obs = observer(latitude_deg, longitude_deg, elevation_m);
        let at_i: Instant = at.into();
        let local_day = Utc::from_instant(at_i.add_ns(utc_offset_seconds as i128 * NS_PER_SEC)).utc_day();
        let mut start = None;
        for back in 0..2 {
            let midnight = local_midnight(local_day - back, utc_offset_seconds)?;
            let s = if from_sunrise {
                day_of(self, &obs, midnight)?.map(|d| d.0)
            } else {
                Some(midnight.add_ns(6 * 3_600 * NS_PER_SEC))
            };
            if let Some(s) = s.filter(|s| *s <= at_i) {
                start = Some((s, back == 1));
                break;
            }
        }
        let Some((start, previous_day)) = start else {
            return Err(KshanaError::Engine("the Sun does not rise at this place on that date".into()));
        };
        let elapsed = at_i.ns_since(start).max(0);
        let n = elapsed / (NAZHIKA_S as i128 * NS_PER_SEC);
        let rest = elapsed % (NAZHIKA_S as i128 * NS_PER_SEC);
        let v = rest / (VINAZHIKA_S as i128 * NS_PER_SEC);
        let vilipta = (rest % (VINAZHIKA_S as i128 * NS_PER_SEC)) as f64 / (VINAZHIKA_S as f64 * 1e9) * 60.0;
        let note = if from_sunrise {
            format!("Counted from the sunrise at {}, at this place.", local_words(start, utc_offset_seconds))
        } else {
            format!("Counted from {} by the clock.", local_words(start, utc_offset_seconds))
        };
        Ok(NazhikaCountRecord {
            nazhika: n as u32,
            vinazhika: v as u32,
            vilipta,
            start: start.into(),
            previous_day,
            note,
        })
    }
}

/// Record a place's height above sea level in metres (0 forgets it). Every sunrise and
/// sunset at those coordinates (within about 100 m) is then seen from that height: a
/// hill station's sun rises a few minutes earlier and sets a few minutes later.
#[uniffi::export]
pub fn set_place_elevation(latitude_deg: f64, longitude_deg: f64, elevation_m: f64) {
    kshana_events::horizon::set_place_elevation(latitude_deg, longitude_deg, elevation_m);
}

/// Forget every recorded place height.
#[uniffi::export]
pub fn clear_place_elevations() {
    kshana_events::horizon::clear_place_elevations();
}

/// The recorded height of a place in metres, 0 when none.
#[uniffi::export]
pub fn place_elevation(latitude_deg: f64, longitude_deg: f64) -> f64 {
    kshana_events::horizon::place_elevation(latitude_deg, longitude_deg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Option<std::sync::Arc<Engine>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Engine::open(p.to_string_lossy().into_owned(), None).ok()
    }

    const IST: i32 = 19_800;
    const KOCHI: (f64, f64) = (9.9312, 76.2673);

    fn local(i: ExactInstant) -> Utc {
        Utc::from_instant(Instant::from(i).add_ns(IST as i128 * NS_PER_SEC))
    }

    #[test]
    fn a_count_from_sunrise_lands_on_the_clock_and_reads_back() {
        let Some(e) = engine() else { return };
        // 12 nazhika 30 vinazhika after sunrise = 4 h 60 min… = 4 h 48 min + 12 min = 5 h 0 min.
        let r = e.nazhika_to_instant(2026, 3, 20, IST, KOCHI.0, KOCHI.1, 0.0, 12, 30, true, false).unwrap();
        let rise = Instant::from(r.sunrise.unwrap());
        assert_eq!(Instant::from(r.start), rise);
        assert_eq!(Instant::from(r.at).ns_since(rise), (12 * 1_440 + 30 * 24) as i128 * NS_PER_SEC);
        let u = local(r.at);
        assert_eq!((u.day, u.hour), (20, 11), "{u:?}");
        assert!((r.daytime_nazhika - 30.3).abs() < 0.6, "{}", r.daytime_nazhika);
        assert!((r.whole_day_nazhika - 60.0).abs() < 0.05, "{}", r.whole_day_nazhika);
        assert!(!r.after_sunset && !r.past_next_sunrise);
        assert!(r.note.contains("20 March 2026"), "{}", r.note);
        // Read back.
        let b = e.instant_to_nazhika(r.at, IST, KOCHI.0, KOCHI.1, 0.0, true).unwrap();
        assert_eq!((b.nazhika, b.vinazhika, b.previous_day), (12, 30, false));
        assert!(b.vilipta < 1e-6 || b.vilipta > 59.999);
    }

    #[test]
    fn a_birth_after_midnight_counts_from_the_previous_sunrise() {
        let Some(e) = engine() else { return };
        // 52 nazhika after the 19 March sunrise is about 03:20 on 20 March.
        let r = e.nazhika_to_instant(2026, 3, 20, IST, KOCHI.0, KOCHI.1, 0.0, 52, 0, true, true).unwrap();
        let u = local(r.at);
        assert_eq!((u.day, u.hour), (20, 3), "{u:?}");
        assert!(r.after_sunset && !r.past_next_sunrise);
        assert!(local(r.start).day == 19);
        let b = e.instant_to_nazhika(r.at, IST, KOCHI.0, KOCHI.1, 0.0, true).unwrap();
        assert_eq!((b.nazhika, b.vinazhika, b.previous_day), (52, 0, true));
        // The same count from the date's own sunrise runs into the next date.
        let same = e.nazhika_to_instant(2026, 3, 20, IST, KOCHI.0, KOCHI.1, 0.0, 52, 0, true, false).unwrap();
        assert_eq!(local(same.at).day, 21);
    }

    #[test]
    fn six_in_the_morning_is_by_the_clock() {
        let Some(e) = engine() else { return };
        let r = e.nazhika_to_instant(1980, 7, 1, IST, KOCHI.0, KOCHI.1, 0.0, 10, 0, false, false).unwrap();
        let u = local(r.at);
        assert_eq!((u.hour, u.minute, u.second), (10, 0, 0));
        assert_eq!(local(r.start).hour, 6);
        let b = e.instant_to_nazhika(r.at, IST, KOCHI.0, KOCHI.1, 0.0, false).unwrap();
        assert_eq!((b.nazhika, b.vinazhika), (10, 0));
        // Bombay Time, +04:51.
        let r = e.nazhika_to_instant(1950, 1, 1, 17_460, 19.076, 72.8777, 0.0, 0, 0, false, false).unwrap();
        let u = Utc::from_instant(Instant::from(r.at));
        assert_eq!((u.hour, u.minute), (1, 9));
    }

    #[test]
    fn height_moves_the_sunrise_the_count_starts_from() {
        let Some(e) = engine() else { return };
        // Munnar, about 1,530 m: the count starts some five minutes earlier than at sea level.
        let (la, lo) = (10.0889, 77.0595);
        let sea = e.nazhika_to_instant(2026, 3, 20, IST, la, lo, 0.0, 5, 0, true, false).unwrap();
        let hill = e.nazhika_to_instant(2026, 3, 20, IST, la, lo, 1_530.0, 5, 0, true, false).unwrap();
        let earlier = Instant::from(sea.at).ns_since(Instant::from(hill.at)) as f64 / 60e9;
        assert!((4.0..6.5).contains(&earlier), "{earlier}");
        assert!(e.nazhika_to_instant(2026, 3, 20, IST, la, lo, 0.0, 61, 0, true, false).is_err());
        assert!(e.nazhika_to_instant(2026, 3, 20, IST, la, lo, 0.0, 5, 60, true, false).is_err());
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;

    /// Numbers and instants only; nothing here is a sentence but the note.
    const NOT_SHOWN: &[&str] = &["at", "start", "sunrise", "sunset", "next_sunrise", "nazhika", "vinazhika", "vilipta"];

    fn check(field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(kshana_rules::Lang::En, v) {
            hits.push(format!("{field} [{w}]: {v}"));
        }
    }

    #[test]
    fn nazhika_notes_name_no_astrology() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(e) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let mut hits = Vec::new();
        for (from_sunrise, prev) in [(true, false), (true, true), (false, false), (false, true)] {
            let r = e.nazhika_to_instant(1990, 2, 20, 19_800, 9.9312, 76.2673, 0.0, 25, 12, from_sunrise, prev).unwrap();
            check("note", &r.note, &mut hits);
            let b = e.instant_to_nazhika(r.at, 19_800, 9.9312, 76.2673, 0.0, from_sunrise).unwrap();
            check("note", &b.note, &mut hits);
            assert!(!r.note.trim().is_empty() && !b.note.trim().is_empty());
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }
}
