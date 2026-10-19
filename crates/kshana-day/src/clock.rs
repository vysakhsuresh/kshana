//! Ghati and vighati, the thirty muhurtas, Abhijit, Brahma muhurta and choghadiya.
//!
//! Units (Surya Siddhanta I.11–12; Siddhanta Shiromani): 1 day = 60 ghati (nadi, 24
//! min); 1 ghati = 60 vighati (pala, vinadi, 24 s); 1 vighati = 60 vipala (0.4 s);
//! 1 muhurta = 2 ghati = 48 min, thirty to the day. Two conventions are in use for the
//! ghati and both are offered by name ([`GhatiConvention`]): the fixed 24-minute ghati
//! counted from sunrise (the savana reckoning of the siddhantas, used by the BPHS
//! lagnas and by Jagannatha Hora), and the proportional ghati that scales thirty ghatis
//! to the real day and thirty to the real night (the dinamana/ratrimana reckoning of
//! the panchangas, the convention `kshana_events` already uses for horas and Rahu
//! kala). A third, sixty equal ghatis from sunrise to the next sunrise, is included
//! for completeness.
//!
//! Muhurta names: Muhurta Chintamani as tabulated on Wikipedia ("Muhurta") and in the
//! ISKCON astrological tables (veda.harekrsna.cz/encyclopedia/astrotables.htm); the
//! same thirty names and qualities, with Girisha inauspicious, are in the research
//! brief. PyJHora's `const.muhurthas_of_the_day` agrees except that it marks Girisha
//! auspicious; Wikipedia's reading is followed. Choghadiya: Drik Panchang
//! (drikpanchang.com/muhurat/choghadiya.html) and PyJHora's
//! `gauri_choghadiya_day_table` / `gauri_choghadiya_night_table`, which agree on every
//! cell; note that the night sequence is not the day cycle restarted but its own
//! cycle (Sunday night Shubh, Amrit, Chal, Rog, Kaal, Labh, Udveg, Shubh).

use kshana_events::SolarDay;
use kshana_sky::Observer;
use kshana_time::{Instant, NS_PER_SEC};

use crate::{weekday, WEEKDAY_LORDS};

/// One fixed ghati: 24 minutes.
pub const GHATI_NS: i128 = 24 * 60 * NS_PER_SEC;
/// One fixed vighati (pala): 24 seconds.
pub const VIGHATI_NS: i128 = 24 * NS_PER_SEC;
/// One fixed vipala: 0.4 seconds.
pub const VIPALA_NS: i128 = 400_000_000;

/// How long a ghati is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GhatiConvention {
    /// Fixed 24-minute ghatis counted from sunrise (savana; Surya Siddhanta, BPHS,
    /// Jagannatha Hora). Sixty of them end a few minutes before or after the next
    /// sunrise.
    Fixed,
    /// Thirty ghatis from sunrise to sunset, each a thirtieth of the day, and thirty from
    /// sunset to the next sunrise, each a thirtieth of the night (dinamana/ratrimana;
    /// the panchanga convention).
    DayNight,
    /// Sixty equal ghatis from sunrise to the next sunrise.
    SolarDay,
}

/// Length in nanoseconds of the ghati running at `at` under `convention`.
pub fn ghati_length_ns(day: &SolarDay, convention: GhatiConvention, at: Instant) -> i128 {
    match convention {
        GhatiConvention::Fixed => GHATI_NS,
        GhatiConvention::SolarDay => day.next_sunrise.ns_since(day.sunrise) / 60,
        GhatiConvention::DayNight => {
            if at < day.sunset {
                day.sunset.ns_since(day.sunrise) / 30
            } else {
                day.next_sunrise.ns_since(day.sunset) / 30
            }
        }
    }
}

/// Split an elapsed span into (ghati, vighati, vipala) when `span` ns hold `n` ghatis;
/// exact integer arithmetic, so the last ghati ends exactly at the span's end.
fn split(elapsed: i128, span: i128, n: i128) -> (u32, u32, f64) {
    let sixtieths = elapsed * 60 * n / span;
    let ghati = (sixtieths / 60) as u32;
    let vighati = (sixtieths % 60) as u32;
    let rem = elapsed * 60 * n - sixtieths * span; // in units of ns × 60 × n
    let vipala = rem as f64 / span as f64 * 60.0;
    (ghati, vighati, vipala)
}

/// Ghati, vighati and fractional vipala elapsed since sunrise at `at` under
/// `convention`: `(ghati, vighati, vipala)` with `0 ≤ vipala < 60`. Under
/// [`GhatiConvention::DayNight`] the night's ghatis continue from 30 at sunset. An
/// instant before the day's sunrise reads as zero.
pub fn ghati_vighati(day: &SolarDay, at: Instant, convention: GhatiConvention) -> (u32, u32, f64) {
    match convention {
        GhatiConvention::Fixed => split(at.ns_since(day.sunrise).max(0), GHATI_NS, 1),
        GhatiConvention::SolarDay => split(
            at.ns_since(day.sunrise).max(0),
            day.next_sunrise.ns_since(day.sunrise),
            60,
        ),
        GhatiConvention::DayNight if at >= day.sunset => {
            let (gh, vi, pa) = split(
                at.ns_since(day.sunset).max(0),
                day.next_sunrise.ns_since(day.sunset),
                30,
            );
            (gh + 30, vi, pa)
        }
        GhatiConvention::DayNight => split(
            at.ns_since(day.sunrise).max(0),
            day.sunset.ns_since(day.sunrise),
            30,
        ),
    }
}

/// Quality of a muhurta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MuhurtaQuality {
    /// Good.
    Auspicious,
    /// Very good (Amrita and Brahma).
    VeryAuspicious,
    /// Bad.
    Inauspicious,
    /// Good except on the listed weekdays (0 = Sunday).
    AuspiciousExcept(&'static [u32]),
}

impl MuhurtaQuality {
    /// Whether the muhurta is auspicious on `weekday` (0 = Sunday).
    pub fn is_auspicious(self, weekday: u32) -> bool {
        match self {
            MuhurtaQuality::Auspicious | MuhurtaQuality::VeryAuspicious => true,
            MuhurtaQuality::Inauspicious => false,
            MuhurtaQuality::AuspiciousExcept(days) => !days.contains(&(weekday % 7)),
        }
    }
}

/// One of the thirty named muhurtas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MuhurtaName {
    /// 1–30; 1–15 by day from sunrise, 16–30 by night from sunset.
    pub number: u8,
    /// Classical name, which is also its presiding deity.
    pub name: &'static str,
    /// Other name in common use.
    pub alias: Option<&'static str>,
    /// Quality.
    pub quality: MuhurtaQuality,
}

const fn mh(
    number: u8,
    name: &'static str,
    alias: Option<&'static str>,
    quality: MuhurtaQuality,
) -> MuhurtaName {
    MuhurtaName {
        number,
        name,
        alias,
        quality,
    }
}

use MuhurtaQuality::{
    Auspicious as A, AuspiciousExcept as AE, Inauspicious as I, VeryAuspicious as VA,
};

/// The thirty muhurtas in order from sunrise (Muhurta Chintamani; Wikipedia "Muhurta";
/// ISKCON astrological tables). Vidhi is Abhijit, the eighth of the day; Jiva is
/// Amrita; Brahma, the twenty-ninth, is the Brahma muhurta.
pub const MUHURTAS: [MuhurtaName; 30] = [
    mh(1, "Rudra", None, I),
    mh(2, "Ahi", None, I),
    mh(3, "Mitra", None, A),
    mh(4, "Pitri", None, I),
    mh(5, "Vasu", None, A),
    mh(6, "Varaha", None, A),
    mh(7, "Vishvedeva", None, A),
    mh(8, "Vidhi", Some("Abhijit"), AE(&[1, 5])),
    mh(9, "Satamukhi", None, A),
    mh(10, "Puruhuta", None, I),
    mh(11, "Vahini", None, I),
    mh(12, "Naktanakara", None, I),
    mh(13, "Varuna", None, A),
    mh(14, "Aryaman", None, AE(&[0])),
    mh(15, "Bhaga", None, I),
    mh(16, "Girisha", None, I),
    mh(17, "Ajapada", None, I),
    mh(18, "Ahirbudhnya", None, A),
    mh(19, "Pushya", None, A),
    mh(20, "Ashwini", None, A),
    mh(21, "Yama", None, I),
    mh(22, "Agni", None, A),
    mh(23, "Vidhatri", None, A),
    mh(24, "Kanda", None, A),
    mh(25, "Aditi", None, A),
    mh(26, "Jiva", Some("Amrita"), VA),
    mh(27, "Vishnu", None, A),
    mh(28, "Dyumadgadyuti", Some("Yumigadyuti"), A),
    mh(29, "Brahma", None, VA),
    mh(30, "Samudram", None, A),
];

/// One muhurta of a particular day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MuhurtaWindow {
    /// Which of the thirty.
    pub muhurta: MuhurtaName,
    /// True for the fifteen night muhurtas.
    pub night: bool,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Auspicious on this weekday.
    pub auspicious: bool,
}

/// The thirty muhurtas of the solar day: fifteen equal parts of sunrise→sunset, then
/// fifteen of sunset→next sunrise. They tile the day exactly.
pub fn muhurtas(day: &SolarDay, obs: &Observer) -> Vec<MuhurtaWindow> {
    let wd = weekday(day, obs);
    let day_len = day.sunset.ns_since(day.sunrise);
    let night_len = day.next_sunrise.ns_since(day.sunset);
    MUHURTAS
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let (base, len, k) = if i < 15 {
                (day.sunrise, day_len, i as i128)
            } else {
                (day.sunset, night_len, (i - 15) as i128)
            };
            MuhurtaWindow {
                muhurta: *m,
                night: i >= 15,
                start: base.add_ns(len * k / 15),
                end: base.add_ns(len * (k + 1) / 15),
                auspicious: m.quality.is_auspicious(wd),
            }
        })
        .collect()
}

/// The muhurta running at `at`, or `None` outside the day.
pub fn muhurta_at(day: &SolarDay, obs: &Observer, at: Instant) -> Option<MuhurtaWindow> {
    muhurtas(day, obs)
        .into_iter()
        .find(|m| at >= m.start && at < m.end)
}

/// Abhijit: the eighth muhurta of the day, one fifteenth of the daytime centred on
/// local apparent noon.
pub fn abhijit(day: &SolarDay) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise);
    (
        day.sunrise.add_ns(len * 7 / 15),
        day.sunrise.add_ns(len * 8 / 15),
    )
}

/// Brahma muhurta in the popular reckoning: the last two muhurtas of the night, from
/// two night-fifteenths before the next sunrise up to that sunrise (96 minutes before
/// sunrise for a twelve-hour night).
pub fn brahma_muhurta(day: &SolarDay) -> (Instant, Instant) {
    let night = day.next_sunrise.ns_since(day.sunset);
    (day.sunset.add_ns(night * 13 / 15), day.next_sunrise)
}

/// Brahma muhurta in the strict reckoning: the twenty-ninth muhurta alone, ending one
/// night-fifteenth before the next sunrise (the window PyJHora's `brahma_muhurtha`
/// returns).
pub fn brahma_muhurta_classical(day: &SolarDay) -> (Instant, Instant) {
    let night = day.next_sunrise.ns_since(day.sunset);
    (
        day.sunset.add_ns(night * 13 / 15),
        day.sunset.add_ns(night * 14 / 15),
    )
}

/// The seven choghadiya.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choghadiya {
    /// Anxiety (Sun); avoid.
    Udveg,
    /// Movement (Venus); good, especially for travel.
    Chal,
    /// Gain (Mercury); good.
    Labh,
    /// Nectar (Moon); good.
    Amrit,
    /// Death (Saturn); avoid.
    Kaal,
    /// Auspicious (Jupiter); good.
    Shubh,
    /// Disease (Mars); avoid.
    Rog,
}

impl Choghadiya {
    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            Choghadiya::Udveg => "Udveg",
            Choghadiya::Chal => "Chal",
            Choghadiya::Labh => "Labh",
            Choghadiya::Amrit => "Amrit",
            Choghadiya::Kaal => "Kaal",
            Choghadiya::Shubh => "Shubh",
            Choghadiya::Rog => "Rog",
        }
    }

    /// Ruling planet.
    pub fn lord(self) -> &'static str {
        match self {
            Choghadiya::Udveg => "Sun",
            Choghadiya::Chal => "Venus",
            Choghadiya::Labh => "Mercury",
            Choghadiya::Amrit => "Moon",
            Choghadiya::Kaal => "Saturn",
            Choghadiya::Shubh => "Jupiter",
            Choghadiya::Rog => "Mars",
        }
    }

    /// Amrit, Shubh, Labh and Chal are good; Rog, Kaal and Udveg are to be avoided.
    pub fn favourable(self) -> bool {
        matches!(
            self,
            Choghadiya::Amrit | Choghadiya::Shubh | Choghadiya::Labh | Choghadiya::Chal
        )
    }
}

/// Day cycle (the Chaldean order from the Sun) and the index of each weekday's first
/// day choghadiya: Sunday Udveg, Monday Amrit, Tuesday Rog, Wednesday Labh, Thursday
/// Shubh, Friday Chal, Saturday Kaal.
const DAY_CYCLE: [Choghadiya; 7] = [
    Choghadiya::Udveg,
    Choghadiya::Chal,
    Choghadiya::Labh,
    Choghadiya::Amrit,
    Choghadiya::Kaal,
    Choghadiya::Shubh,
    Choghadiya::Rog,
];
const DAY_START: [usize; 7] = [0, 3, 6, 2, 5, 1, 4];
/// Night cycle and the index of each weekday's first night choghadiya: Sunday Shubh,
/// Monday Chal, Tuesday Kaal, Wednesday Udveg, Thursday Amrit, Friday Rog, Saturday
/// Labh.
const NIGHT_CYCLE: [Choghadiya; 7] = [
    Choghadiya::Shubh,
    Choghadiya::Amrit,
    Choghadiya::Chal,
    Choghadiya::Rog,
    Choghadiya::Kaal,
    Choghadiya::Labh,
    Choghadiya::Udveg,
];
const NIGHT_START: [usize; 7] = [0, 2, 4, 6, 1, 3, 5];

/// The eight day and eight night choghadiya of a weekday (0 = Sunday); the eighth of
/// each repeats the first.
pub fn choghadiya_table(weekday: u32) -> ([Choghadiya; 8], [Choghadiya; 8]) {
    let wd = (weekday % 7) as usize;
    (
        std::array::from_fn(|i| DAY_CYCLE[(DAY_START[wd] + i) % 7]),
        std::array::from_fn(|i| NIGHT_CYCLE[(NIGHT_START[wd] + i) % 7]),
    )
}

/// One choghadiya of a particular day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoghadiyaWindow {
    /// Which.
    pub kind: Choghadiya,
    /// True for the eight night choghadiya.
    pub night: bool,
    /// Slot 1–8 within the day or the night.
    pub slot: u8,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Good to act in.
    pub favourable: bool,
}

/// The sixteen choghadiya of the solar day: eight equal parts of sunrise→sunset and
/// eight of sunset→next sunrise. They tile the day exactly.
pub fn choghadiya(day: &SolarDay, obs: &Observer) -> Vec<ChoghadiyaWindow> {
    let (d, n) = choghadiya_table(weekday(day, obs));
    let day_len = day.sunset.ns_since(day.sunrise);
    let night_len = day.next_sunrise.ns_since(day.sunset);
    let mut out = Vec::with_capacity(16);
    for (i, kind) in d.iter().enumerate() {
        out.push(ChoghadiyaWindow {
            kind: *kind,
            night: false,
            slot: i as u8 + 1,
            start: day.sunrise.add_ns(day_len * i as i128 / 8),
            end: day.sunrise.add_ns(day_len * (i as i128 + 1) / 8),
            favourable: kind.favourable(),
        });
    }
    for (i, kind) in n.iter().enumerate() {
        out.push(ChoghadiyaWindow {
            kind: *kind,
            night: true,
            slot: i as u8 + 1,
            start: day.sunset.add_ns(night_len * i as i128 / 8),
            end: day.sunset.add_ns(night_len * (i as i128 + 1) / 8),
            favourable: kind.favourable(),
        });
    }
    out
}

/// The choghadiya running at `at`, or `None` outside the day.
pub fn choghadiya_at(day: &SolarDay, obs: &Observer, at: Instant) -> Option<ChoghadiyaWindow> {
    choghadiya(day, obs)
        .into_iter()
        .find(|c| at >= c.start && at < c.end)
}

/// Lord of the weekday's first day choghadiya, for cross-checks: it is the weekday lord.
pub fn first_choghadiya_lord(weekday: u32) -> &'static str {
    WEEKDAY_LORDS[(weekday % 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, utc, KOCHI};
    use kshana_events::solar_day;

    fn day() -> Option<SolarDay> {
        let spk = kernel()?;
        // Thursday 2026-10-15 at Kochi
        solar_day(&spk, None, &KOCHI, utc(2026, 10, 15, 9, 0, 0)).ok()?
    }

    #[test]
    fn muhurtas_tile_sunrise_to_next_sunrise() {
        let Some(d) = day() else { return };
        let m = muhurtas(&d, &KOCHI);
        assert_eq!(m.len(), 30);
        assert_eq!(m[0].start, d.sunrise);
        assert_eq!(m[14].end, d.sunset);
        assert_eq!(m[15].start, d.sunset);
        assert_eq!(m[29].end, d.next_sunrise);
        for w in m.windows(2) {
            assert_eq!(w[0].end, w[1].start);
            assert!(w[0].end > w[0].start);
        }
        assert_eq!(m[7].muhurta.alias, Some("Abhijit"));
        assert_eq!((m[7].start, m[7].end), abhijit(&d));
        assert_eq!(m[28].muhurta.name, "Brahma");
        assert_eq!((m[28].start, m[28].end), brahma_muhurta_classical(&d));
        assert_eq!(brahma_muhurta(&d).0, m[28].start);
        assert_eq!(brahma_muhurta(&d).1, d.next_sunrise);
        // Thursday: Abhijit and Aryaman are auspicious, Rudra is not
        assert!(m[7].auspicious && m[13].auspicious && !m[0].auspicious);
        assert_eq!(muhurta_at(&d, &KOCHI, d.sunrise).unwrap().muhurta.number, 1);
        assert_eq!(
            muhurta_at(&d, &KOCHI, d.next_sunrise.add_ns(-1))
                .unwrap()
                .muhurta
                .number,
            30
        );
        assert!(muhurta_at(&d, &KOCHI, d.next_sunrise).is_none());
    }

    #[test]
    fn muhurta_weekday_exceptions() {
        assert!(!MUHURTAS[7].quality.is_auspicious(1));
        assert!(!MUHURTAS[7].quality.is_auspicious(5));
        assert!(MUHURTAS[7].quality.is_auspicious(0));
        assert!(!MUHURTAS[13].quality.is_auspicious(0));
        assert!(MUHURTAS[13].quality.is_auspicious(3));
        assert_eq!(MUHURTAS.iter().filter(|m| m.quality == I).count(), 10);
        for (i, m) in MUHURTAS.iter().enumerate() {
            assert_eq!(m.number as usize, i + 1);
        }
    }

    #[test]
    fn choghadiya_tables_match_drik_panchang() {
        let (d, n) = choghadiya_table(0);
        let names: Vec<_> = d.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            ["Udveg", "Chal", "Labh", "Amrit", "Kaal", "Shubh", "Rog", "Udveg"]
        );
        let names: Vec<_> = n.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            ["Shubh", "Amrit", "Chal", "Rog", "Kaal", "Labh", "Udveg", "Shubh"]
        );
        let (d, n) = choghadiya_table(1);
        assert_eq!(d[0], Choghadiya::Amrit);
        let names: Vec<_> = n.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            ["Chal", "Rog", "Kaal", "Labh", "Udveg", "Shubh", "Amrit", "Chal"]
        );
        for wd in 0..7 {
            let (d, n) = choghadiya_table(wd);
            assert_eq!(d[0], d[7]);
            assert_eq!(n[0], n[7]);
            assert_eq!(d[0].lord(), first_choghadiya_lord(wd));
            // the first night choghadiya belongs to the lord of the fifth weekday from the day's
            assert_eq!(n[0].lord(), WEEKDAY_LORDS[((wd + 4) % 7) as usize]);
        }
    }

    #[test]
    fn choghadiya_windows_tile_the_day() {
        let Some(d) = day() else { return };
        let c = choghadiya(&d, &KOCHI);
        assert_eq!(c.len(), 16);
        assert_eq!(c[0].start, d.sunrise);
        assert_eq!(c[7].end, d.sunset);
        assert_eq!(c[8].start, d.sunset);
        assert_eq!(c[15].end, d.next_sunrise);
        for w in c.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        // Thursday: first day choghadiya Shubh, first night Amrit
        assert_eq!(c[0].kind, Choghadiya::Shubh);
        assert_eq!(c[8].kind, Choghadiya::Amrit);
        assert!(c.iter().any(|w| !w.favourable) && c.iter().any(|w| w.favourable));
        let mid = d.sunrise.add_ns(d.sunset.ns_since(d.sunrise) / 2);
        assert_eq!(choghadiya_at(&d, &KOCHI, mid).unwrap().slot, 5);
    }

    #[test]
    fn ghati_readout_is_consistent_with_elapsed_time() {
        let Some(d) = day() else { return };
        // 1 h 00 m 10 s after sunrise = 2 ghati 30 vighati 25 vipala (fixed)
        let at = d.sunrise.add_ns(3_610 * NS_PER_SEC);
        let (g, v, p) = ghati_vighati(&d, at, GhatiConvention::Fixed);
        assert_eq!((g, v), (2, 30));
        assert!((p - 25.0).abs() < 1e-9, "{p}");
        // exactly 7 ghatis 7 vighatis
        let at = d.sunrise.add_ns(7 * GHATI_NS + 7 * VIGHATI_NS);
        assert_eq!(ghati_vighati(&d, at, GhatiConvention::Fixed), (7, 7, 0.0));
        // day/night convention: sunset is ghati 30, next sunrise is ghati 60
        let (g, v, _) = ghati_vighati(&d, d.sunset, GhatiConvention::DayNight);
        assert_eq!((g, v), (30, 0));
        let (g, _, _) = ghati_vighati(&d, d.next_sunrise.add_ns(-1), GhatiConvention::DayNight);
        assert_eq!(g, 59);
        // solar-day convention: the midpoint of the day is ghati 30
        let (g, v, p) = ghati_vighati(&d, d.next_sunrise.add_ns(-1), GhatiConvention::SolarDay);
        assert_eq!((g, v), (59, 59));
        assert!(p < 60.0 && p > 59.999);
        assert_eq!(
            ghati_vighati(&d, d.next_sunrise, GhatiConvention::SolarDay),
            (60, 0, 0.0)
        );
        // before sunrise reads zero
        assert_eq!(
            ghati_vighati(&d, d.sunrise.add_ns(-5), GhatiConvention::Fixed),
            (0, 0, 0.0)
        );
        // a sixtieth of the day-length ghati is the vighati
        assert_eq!(
            ghati_length_ns(&d, GhatiConvention::Fixed, at),
            60 * VIGHATI_NS
        );
        assert_eq!(VIPALA_NS * 60, VIGHATI_NS);
    }
}
