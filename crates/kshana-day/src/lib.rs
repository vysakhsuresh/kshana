//! The day's clocks: everything Jyotisha divides a day into, with every boundary an
//! exact instant derived from the solved sunrise or from a solved root.
//!
//! * [`clock`]: ghati and vighati readouts under the named conventions, the thirty
//!   muhurtas with their classical names, Abhijit, Brahma muhurta and the sixteen
//!   choghadiya of day and night.
//! * [`upagraha`]: Gulika, Mandi, Kala, Mrityu, Ardhaprahara and Yamaghantaka as kalas
//!   (eighths of the day or night, with the start, middle and end conventions) and as
//!   longitudes (the sidereal ascendant at the chosen instant of the kala), the popular
//!   Yamaganda and Gulika kala tables, and the five arithmetic upagrahas from the Sun.
//! * [`lagna`]: the BPHS sub-hour lagnas (Bhava, Hora, Ghati, Vighati, Pranapada and
//!   Sree) as longitude functions of time, with the exact instants at which they change
//!   sign.
//! * [`windows`]: Gandanta, Panchaka, Bhadra (Vishti karana), the named inauspicious
//!   yogas, tara bala and chandra bala as exact windows of the transiting Moon.
//! * [`pakshi`]: Pancha Pakshi, the five birds and their five activities over the
//!   solar day.
//!
//! Precision: a sunrise-derived edge is exact arithmetic on a sunrise that
//! `kshana_events::solar_day` solves to a millisecond, but the physical sunrise is
//! limited by refraction and the horizon to tens of seconds, so these edges should be
//! shown with that bracket. A transit edge (the Moon crossing a sidereal boundary) is
//! a root solved to a microsecond and carries its own bracket width.

#![forbid(unsafe_code)]

pub mod clock;
pub mod lagna;
pub mod pakshi;
pub mod upagraha;
pub mod windows;

pub use clock::{
    abhijit, brahma_muhurta, brahma_muhurta_classical, choghadiya, choghadiya_at, choghadiya_table,
    first_choghadiya_lord, ghati_length_ns, ghati_vighati, muhurta_at, muhurtas, Choghadiya,
    ChoghadiyaWindow, GhatiConvention, MuhurtaName, MuhurtaQuality, MuhurtaWindow, GHATI_NS,
    MUHURTAS, VIGHATI_NS, VIPALA_NS,
};
pub use lagna::{
    ghati_family_lagna, lagna_sign_changes, pranapada, pranapada_at, pranapada_auspicious,
    pranapada_offset, special_lagna_at, sree_lagna, sree_lagna_at, sun_sidereal_at, LagnaChange,
    LagnaContext, SpecialLagna, SunAnchor,
};
pub use pakshi::{
    activity_cycle, birth_bird, birth_bird_at, pancha_pakshi_day, pancha_pakshi_sub_windows,
    pancha_pakshi_windows, ruling_bird, yama_activity, Activity, Bird, PakshiSubWindow,
    PakshiWindow, SubPeriodDurations, ACTIVITY_TABLE, SUB_PERIOD_DAY, SUB_PERIOD_NIGHT,
};
pub use upagraha::{
    gulika_kala, mandi_instant_kerala, part_lord_name, part_lords, parts, solar_upagrahas,
    upagraha_instant, upagraha_kala, upagraha_longitude, upagraha_longitudes, yamaganda_kala,
    KalaPoint, Upagraha, UpagrahaPosition, GULIKA_SLOTS, YAMAGANDA_SLOTS,
};
pub use windows::{
    bhadra_windows, chandra_bala_windows, gandanta_windows, panchaka_windows, special_yoga_windows,
    tara_bala_windows, yoga_windows, LabelledWindow, CHANDRA_LABELS, INAUSPICIOUS_YOGAS,
    TARA_NAMES,
};

use kshana_chart::ut1_for;
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{solar_day, EventError, SolarDay};
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_iau2006a,
};
use kshana_houses::{angles, HouseError};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, Utc, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

/// Why a clock could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DayError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Sunrise or root search failure.
    Events(EventError),
    /// The ascendant is undefined at the place.
    Houses(HouseError),
    /// No sunrise brackets the instant (polar day or night, or outside the kernel).
    NoSolarDay,
}

impl From<SpkError> for DayError {
    fn from(e: SpkError) -> Self {
        DayError::Spk(e)
    }
}
impl From<EventError> for DayError {
    fn from(e: EventError) -> Self {
        DayError::Events(e)
    }
}
impl From<HouseError> for DayError {
    fn from(e: HouseError) -> Self {
        DayError::Houses(e)
    }
}

/// Weekday lords in weekday order, index 0 = Sunday.
pub const WEEKDAY_LORDS: [&str; 7] = [
    "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn",
];

/// The day clocks' own names in Malayalam and Hindi, `(English, ml, hi)`: the thirty
/// muhurtas and their other names, the choghadiyas, the clock lagnas, the upagrahas, the
/// Pancha Pakshi birds and what they do, the Moon's window labels and the nine taras.
/// Mrityu, Mars's upagraha, is named by its father in Malayalam and Hindi, as the app never
/// prints the word for death.
const DAY_WORDS: &[(&str, &str, &str)] = &[
    // Muhurtas (Muhurta Chintamani's thirty), and their other names.
    ("Rudra", "രുദ്ര", "रुद्र"),
    ("Ahi", "അഹി", "अहि"),
    ("Mitra", "മിത്രം", "मित्र"),
    ("Pitri", "പിതൃ", "पितृ"),
    ("Vasu", "വസു", "वसु"),
    ("Varaha", "വരാഹ", "वराह"),
    ("Vishvedeva", "വിശ്വേദേവ", "विश्वेदेव"),
    ("Vidhi", "വിധി", "विधि"),
    ("Satamukhi", "ശതമുഖി", "शतमुखी"),
    ("Puruhuta", "പുരുഹൂത", "पुरुहूत"),
    ("Vahini", "വാഹിനി", "वाहिनी"),
    ("Naktanakara", "നക്തനകര", "नक्तनकर"),
    ("Varuna", "വരുണ", "वरुण"),
    ("Aryaman", "അര്യമാവ്", "अर्यमा"),
    ("Bhaga", "ഭഗ", "भग"),
    ("Girisha", "ഗിരീശ", "गिरीश"),
    ("Ajapada", "അജപാദ", "अजपाद"),
    ("Ahirbudhnya", "അഹിർബുധ്ന്യ", "अहिर्बुध्न्य"),
    ("Pushya", "പുഷ്യ", "पुष्य"),
    ("Ashwini", "അശ്വിനി", "अश्विनी"),
    ("Yama", "യമ", "यम"),
    ("Agni", "അഗ്നി", "अग्नि"),
    ("Vidhatri", "വിധാതൃ", "विधातृ"),
    ("Kanda", "കണ്ഡ", "कण्ड"),
    ("Aditi", "അദിതി", "अदिति"),
    ("Jiva", "ജീവ", "जीव"),
    ("Vishnu", "വിഷ്ണു", "विष्णु"),
    ("Dyumadgadyuti", "ദ്യുമദ്ഗദ്യുതി", "द्युमद्गद्युति"),
    ("Brahma", "ബ്രഹ്മ", "ब्रह्मा"),
    ("Samudram", "സമുദ്രം", "समुद्र"),
    ("Abhijit", "അഭിജിത്", "अभिजित्"),
    ("Amrita", "അമൃത", "अमृत"),
    ("Yumigadyuti", "യുമിഗദ്യുതി", "युमिगद्युति"),
    // Choghadiyas.
    ("Udveg", "ഉദ്വേഗ", "उद्वेग"),
    ("Chal", "ചല", "चल"),
    ("Labh", "ലാഭ", "लाभ"),
    ("Amrit", "അമൃത്", "अमृत"),
    ("Kaal", "കാല", "काल"),
    ("Shubh", "ശുഭ", "शुभ"),
    ("Rog", "രോഗ", "रोग"),
    // Clock lagnas.
    ("Bhava lagna", "ഭാവലഗ്നം", "भाव लग्न"),
    ("Hora lagna", "ഹോരാലഗ്നം", "होरा लग्न"),
    ("Ghati lagna", "ഘടികാലഗ്നം", "घटी लग्न"),
    ("Vighati lagna", "വിഘടികാലഗ്നം", "विघटी लग्न"),
    ("Pranapada", "പ്രാണപദം", "प्राणपद"),
    ("Sree lagna", "ശ്രീലഗ്നം", "श्री लग्न"),
    // Upagrahas, and where in its part each is taken.
    ("Kala", "കാലൻ", "काल"),
    ("Mrityu", "കുജോപഗ്രഹം", "मंगल उपग्रह"),
    ("Ardhaprahara", "അർധപ്രഹരൻ", "अर्धप्रहर"),
    ("Yamaghantaka", "യമകണ്ടകൻ", "यमघंटक"),
    ("Gulika", "ഗുളികൻ", "गुलिक"),
    ("Mandi", "മാന്ദി", "मांदि"),
    ("Dhuma", "ധൂമം", "धूम"),
    ("Vyatipata", "വ്യതീപാതം", "व्यतीपात"),
    ("Parivesha", "പരിവേഷം", "परिवेश"),
    ("Indrachapa", "ഇന്ദ്രചാപം", "इंद्रचाप"),
    ("Upaketu", "ഉപകേതു", "उपकेतु"),
    ("Start", "ആരംഭം", "आरंभ"),
    ("Middle", "മധ്യം", "मध्य"),
    ("End", "അവസാനം", "अंत"),
    // Pancha Pakshi.
    ("Vulture", "കഴുകൻ", "गिद्ध"),
    ("Owl", "മൂങ്ങ", "उल्लू"),
    ("Crow", "കാക്ക", "कौआ"),
    ("Cock", "കോഴി", "मुर्गा"),
    ("Peacock", "മയിൽ", "मोर"),
    ("Ruling", "ഭരണം", "शासन"),
    ("Eating", "ഭക്ഷണം", "भोजन"),
    ("Walking", "നടത്തം", "चलना"),
    ("Sleeping", "ഉറക്കം", "निद्रा"),
    ("Fading", "ക്ഷയം", "क्षय"),
    // The Moon's windows.
    ("Cancer–Leo gandanta", "കർക്കടകം–ചിങ്ങം ഗണ്ഡാന്തം", "कर्क–सिंह गंडांत"),
    ("Scorpio–Sagittarius gandanta", "വൃശ്ചികം–ധനു ഗണ്ഡാന്തം", "वृश्चिक–धनु गंडांत"),
    ("Pisces–Aries gandanta", "മീനം–മേടം ഗണ്ഡാന്തം", "मीन–मेष गंडांत"),
    ("Panchaka", "പഞ്ചകം", "पंचक"),
    ("Vishti (Bhadra)", "വിഷ്ടി (ഭദ്ര)", "विष्टि (भद्रा)"),
    // The nine taras.
    ("Janma", "ജന്മ", "जन्म"),
    ("Sampat", "സമ്പത്ത്", "संपत्"),
    ("Vipat", "വിപത്ത്", "विपत्"),
    ("Kshema", "ക്ഷേമം", "क्षेम"),
    ("Pratyak", "പ്രത്യക്", "प्रत्यक्"),
    ("Sadhana", "സാധന", "साधना"),
    ("Naidhana", "നൈധനം", "नैधन"),
    ("Parama Mitra", "പരമമിത്രം", "परम मित्र"),
];

/// A name the day's clocks print, in the reader's language: a muhurta, choghadiya,
/// clock lagna, upagraha, bird, activity, Moon window, tara or Chandra bala label; planet,
/// sign, star and yoga names go through the lexicon. English, and anything unknown, comes
/// back unchanged.
pub fn word_in(key: &str, lang: kshana_rules::Lang) -> String {
    use kshana_rules::Lang;
    if lang == Lang::En {
        return key.to_string();
    }
    if let Some((_, ml, hi)) = DAY_WORDS.iter().find(|(k, _, _)| *k == key) {
        return lang.pick([key, ml, hi]).to_string();
    }
    if let Some(i) = windows::CHANDRA_LABELS.iter().position(|l| *l == key) {
        let n = i + 1;
        return match (lang, n) {
            (Lang::Ml, 8) => "ചന്ദ്രാഷ്ടമം: ജന്മചന്ദ്രനിൽ നിന്ന് 8-ാം രാശിയിൽ ചന്ദ്രൻ".to_string(),
            (Lang::Hi, 8) => "चंद्राष्टम: जन्म चंद्र से 8वीं राशि में चंद्र".to_string(),
            (Lang::Ml, _) => format!("ജന്മചന്ദ്രനിൽ നിന്ന് {n}-ാം രാശിയിൽ ചന്ദ്രൻ"),
            (_, 2) => "जन्म चंद्र से दूसरी राशि में चंद्र".to_string(),
            (_, 3) => "जन्म चंद्र से तीसरी राशि में चंद्र".to_string(),
            (_, 4) => "जन्म चंद्र से चौथी राशि में चंद्र".to_string(),
            (_, 1) => "जन्म चंद्र की ही राशि में चंद्र".to_string(),
            _ => format!("जन्म चंद्र से {n}वीं राशि में चंद्र"),
        };
    }
    kshana_speak::lexicon::names_in(key, lang)
}

/// Weekday (0 = Sunday) of the solar day: the local civil date at its sunrise, the
/// local offset taken from the observer's longitude (the same rule as
/// `kshana_events::vara_name`).
pub fn weekday(day: &SolarDay, obs: &Observer) -> u32 {
    let local = day
        .sunrise
        .add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

/// Sidereal longitude (degrees) of the rising point at `t` for the observer: the
/// Ascendant from local apparent sidereal time and the true obliquity, less the
/// ayanamsa of date. The same chain as `kshana_chart::compute` uses for cusp 1.
pub fn ascendant_sidereal(
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
    ayanamsa: Ayanamsa,
) -> f64 {
    let tc = t.tt_centuries_since_j2000();
    let (ut1, _) = ut1_for(t, eop);
    let era = earth_rotation_angle(ut1);
    let lst = local_sidereal_time(gast(era, tc), obs.longitude_deg.to_radians());
    let (_, deps) = nutation_iau2006a(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let asc = angles(lst, eps, obs.latitude_deg.to_radians()).asc;
    sidereal(asc, ayanamsa_degrees(ayanamsa, tc))
}

/// Apparent sidereal longitude (degrees) of `body` at `t`.
pub fn sidereal_longitude(
    spk: &Spk,
    body: Body,
    t: Instant,
    ayanamsa: Ayanamsa,
) -> Result<f64, SpkError> {
    let a = apparent(spk, body, t)?;
    Ok(sidereal(
        a.longitude,
        ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000()),
    ))
}

/// The consecutive solar days that cover `[from, to)`: the first is the day containing
/// `from`, and each later day begins at exactly the previous day's `next_sunrise`, so
/// the days tile the range without a gap or an overlap at the seams.
pub fn solar_days_covering(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    from: Instant,
    to: Instant,
) -> Result<Vec<SolarDay>, DayError> {
    let mut days = Vec::new();
    let mut day = solar_day(spk, eop, obs, from)?.ok_or(DayError::NoSolarDay)?;
    loop {
        days.push(day);
        if day.next_sunrise >= to {
            break;
        }
        let next = solar_day(spk, eop, obs, day.next_sunrise.add_ns(NS_PER_SEC))?
            .ok_or(DayError::NoSolarDay)?;
        day = SolarDay {
            sunrise: day.next_sunrise,
            sunset: next.sunset,
            next_sunrise: next.next_sunrise,
        };
    }
    Ok(days)
}

#[cfg(test)]
pub(crate) mod fixture {
    use kshana_ephem::Spk;
    use kshana_sky::Observer;
    use kshana_time::{Instant, Utc};

    /// Kochi, the fixture place used across the workspace.
    pub const KOCHI: Observer = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };

    pub fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    pub fn utc(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Instant {
        Utc::new(y, mo, d, h, mi, s, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixture::{kernel, utc, KOCHI};
    use kshana_time::NS_PER_DAY;

    #[test]
    fn weekday_matches_the_events_crate_vara() {
        let Some(spk) = kernel() else { return };
        // 2026-03-20 is a Friday
        let day = solar_day(&spk, None, &KOCHI, utc(2026, 3, 20, 6, 0, 0))
            .unwrap()
            .unwrap();
        assert_eq!(weekday(&day, &KOCHI), 5);
        assert_eq!(kshana_events::vara_name(&day, &KOCHI), "Shukravara");
    }

    #[test]
    fn covering_days_tile_without_seams() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 15, 9, 0, 0);
        let days =
            solar_days_covering(&spk, None, &KOCHI, from, from.add_ns(3 * NS_PER_DAY)).unwrap();
        assert_eq!(days.len(), 4, "{days:?}");
        for w in days.windows(2) {
            assert_eq!(w[0].next_sunrise, w[1].sunrise);
            assert!(w[1].sunrise < w[1].sunset && w[1].sunset < w[1].next_sunrise);
        }
    }

    #[test]
    fn ascendant_agrees_with_the_chart() {
        let Some(spk) = kernel() else { return };
        let t = utc(2026, 10, 15, 9, 2, 7);
        let ch = kshana_chart::compute(
            &spk,
            None,
            &kshana_chart::ChartInput {
                instant: t,
                observer: KOCHI,
                system: kshana_houses::HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let asc = ascendant_sidereal(None, &KOCHI, t, Ayanamsa::Lahiri);
        assert!(
            (asc - ch.cusps_sidereal[0]).abs() < 1e-9,
            "{asc} vs {}",
            ch.cusps_sidereal[0]
        );
    }
}
