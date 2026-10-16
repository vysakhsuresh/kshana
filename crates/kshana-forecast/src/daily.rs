//! The day for one person: good hours, hours to avoid, the Moon's standing.

use kshana_chart::Chart;
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{hora_at, rahu_kala, solar_day, SolarDay};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, nakshatra, sidereal, Ayanamsa};

/// A window of hours with a reason.
#[derive(Clone, Debug, PartialEq)]
pub struct HourWindow {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Why.
    pub reason: String,
    /// True for a window to avoid.
    pub avoid: bool,
}

/// The day's forecast.
#[derive(Clone, Debug, PartialEq)]
pub struct DailyForecast {
    /// Sunrise, sunset, next sunrise.
    pub day: Option<SolarDay>,
    /// Tara bala name (Janma … Parama Mitra) and whether it is favourable.
    pub tara: (String, bool),
    /// Transit Moon's house from the natal Moon (1–12) and whether Chandra bala holds.
    pub chandra_bala: (u8, bool),
    /// Transit Moon's nakshatra name.
    pub moon_nakshatra: String,
    /// Windows, sorted by start.
    pub windows: Vec<HourWindow>,
    /// A one-line summary built from the facts above.
    pub headline: String,
}

const TARA: [&str; 9] = [
    "Janma",
    "Sampat",
    "Vipat",
    "Kshema",
    "Pratyak",
    "Sadhana",
    "Naidhana",
    "Mitra",
    "Parama Mitra",
];

/// Compute the day's forecast for the solar day containing `at`.
pub fn daily(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    natal: &Chart,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<DailyForecast, SpkError> {
    let ay = ayanamsa_degrees(ayanamsa, at.tt_centuries_since_j2000());
    let moon_t = sidereal(apparent(spk, Body::Moon, at)?.longitude, ay);
    let n = nakshatra(moon_t);
    let janma = nakshatra(natal.bodies[1].sidereal).index;
    let count = (n.index + 27 - janma) % 27 % 9;
    let tara_good = !matches!(count, 2 | 4 | 6);
    let from_moon = (((moon_t / 30.0).floor() as i64
        - (natal.bodies[1].sidereal / 30.0).floor() as i64)
        .rem_euclid(12)
        + 1) as u8;
    let cb = matches!(from_moon, 1 | 3 | 6 | 7 | 10 | 11);
    let day = solar_day(spk, eop, obs, at).ok().flatten();
    let mut windows = Vec::new();
    if let Some(d) = &day {
        let (rs, re) = rahu_kala(d, obs);
        windows.push(HourWindow {
            start: rs,
            end: re,
            reason: "Rahu kala".into(),
            avoid: true,
        });
        // walk the horas of the day and night
        let mut t = d.sunrise;
        while t < d.next_sunrise {
            let (lord, s, e) = hora_at(d, obs, t);
            let good = matches!(lord, "Jupiter" | "Venus" | "Mercury" | "Moon");
            if good && e.ns_since(s) > 10 * 60 * NS_PER_SEC {
                windows.push(HourWindow {
                    start: s,
                    end: e,
                    reason: format!("{lord} hora"),
                    avoid: false,
                });
            }
            t = e.add_ns(1_000);
        }
        // Abhijit
        let len = d.sunset.ns_since(d.sunrise);
        let noon = d.sunrise.add_ns(len / 2);
        windows.push(HourWindow {
            start: noon.add_ns(-len / 30),
            end: noon.add_ns(len / 30),
            reason: "Abhijit".into(),
            avoid: false,
        });
    }
    windows.sort_by_key(|w| w.start);
    let headline = match (tara_good, cb) {
        (true, true) => format!("A supportive day: the Moon is in {} ({}), a good star for you, and well placed from your natal Moon.", n.name(), TARA[count]),
        (true, false) => format!("A mixed day: the Moon's star {} is good for you, but its sign is not well placed from your natal Moon.", n.name()),
        (false, true) => format!("A cautious day: the Moon's star {} ({}) is unfavourable for you, though its sign is well placed.", n.name(), TARA[count]),
        (false, false) => format!("A day to keep simple: the Moon's star {} ({}) and sign are both unfavourable for you.", n.name(), TARA[count]),
    };
    Ok(DailyForecast {
        day,
        tara: (TARA[count].to_string(), tara_good),
        chandra_bala: (from_moon, cb),
        moon_nakshatra: n.name().to_string(),
        windows,
        headline,
    })
}
