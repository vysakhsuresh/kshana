//! Regional terminology and the Kerala and Tamil readouts: every name in Malayalam, Tamil,
//! Hindi, Telugu and Kannada script, udayadi nazhika since sunrise, the Kollavarsham date,
//! the Tamil solar month, and the note on Vakya versus Drik panchanga. Thin wrappers over
//! `kshana_speak::lexicon`.

use kshana_speak::lexicon as lex;
use kshana_speak::Language;

use crate::ExactInstant;

/// One language the lexicon speaks: its code (`ml`), its own name (`മലയാളം`) and its English name.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LanguageRecord {
    pub code: String,
    pub native: String,
    pub english: String,
}

/// Time since sunrise in the Kerala almanac's units: 60 nazhikas of 24 minutes to a day,
/// 60 vinazhikas of 24 seconds to a nazhika, 60 viliptas to a vinazhika (fractional).
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct NazhikaRecord {
    pub nazhika: u32,
    pub vinazhika: u32,
    pub vilipta: f64,
}

/// A Kollavarsham (Malayalam era) date: year, month index 0–11 (Chingam = 0), the English
/// month name, and the day of the month (1-based).
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KollavarshamRecord {
    pub year: i64,
    pub month: u32,
    pub month_name: String,
    pub day: u32,
}

/// One term in one language: its family, the engine key, the English display name, the
/// Sanskrit or alternate name, and the native script and Latin transliteration (both empty
/// when the cell has not been verified for that language).
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LexiconEntryRecord {
    pub family: String,
    pub key: String,
    pub english: String,
    pub sanskrit: String,
    pub native: String,
    pub latin: String,
}

fn lang_of(code: &str) -> Language {
    Language::from_code(code).unwrap_or(Language::English)
}

/// The term `key` (an engine spelling such as `"Ashwini"`, `"Shukla Pratipada"`, `"Rahu kala"`,
/// or an alias such as `"Mesha"`, `"Sunday"`, `"House 7"`) in `language` (`en`, `ml`, `ta`, `hi`,
/// `te`, `kn`; anything else is English), in native script. Falls back to the English name
/// when the cell is unverified and to `key` itself when the key is unknown.
#[uniffi::export]
pub fn term(key: String, language: String) -> String {
    let t = lex::term(&key, lang_of(&language));
    if t.is_empty() {
        key
    } else {
        t.to_string()
    }
}

/// The Latin transliteration of `key` in `language` (for English, the Sanskrit or alternate
/// name), with the same fallbacks as [`term`].
#[uniffi::export]
pub fn term_latin(key: String, language: String) -> String {
    let t = lex::term_latin(&key, lang_of(&language));
    if t.is_empty() {
        key
    } else {
        t.to_string()
    }
}

/// The six languages, English first.
#[uniffi::export]
pub fn language_names() -> Vec<LanguageRecord> {
    Language::ALL
        .iter()
        .map(|l| LanguageRecord {
            code: l.code().to_string(),
            native: l.native_name().to_string(),
            english: l.english_name().to_string(),
        })
        .collect()
}

/// Every term in `language`, family by family: rasi, nakshatra, tithi, vara, yoga, karana,
/// graha, bhava, period, clock, observance, month_kolla, month_tamil, month_lunar.
#[uniffi::export]
pub fn lexicon(language: String) -> Vec<LexiconEntryRecord> {
    lex::lexicon(lang_of(&language))
        .into_iter()
        .map(|e| LexiconEntryRecord {
            family: e.family.to_string(),
            key: e.key.to_string(),
            english: e.english.to_string(),
            sanskrit: e.sanskrit.to_string(),
            native: e.native.to_string(),
            latin: e.latin.to_string(),
        })
        .collect()
}

/// Udayadi nazhika: the time from `sunrise` to `at` as nazhika, vinazhika and fractional
/// vilipta. At sunrise `(0, 0, 0.0)`; 25 minutes later `(1, 2, 30.0)`.
#[uniffi::export]
pub fn nazhika(sunrise: ExactInstant, at: ExactInstant) -> NazhikaRecord {
    let (nazhika, vinazhika, vilipta) = lex::nazhika_vinazhika(sunrise.into(), at.into());
    NazhikaRecord {
        nazhika,
        vinazhika,
        vilipta,
    }
}

/// The Tamil solar month for a sidereal Sun longitude in degrees (Chithirai while the Sun
/// is in Mesha, … Panguni in Meena).
#[uniffi::export]
pub fn tamil_month(sun_sidereal: f64) -> String {
    lex::tamil_month(sun_sidereal).to_string()
}

/// The Kollavarsham month for a sidereal Sun longitude in degrees (Chingam while the Sun
/// is in Simha, … Karkidakam in Karka).
#[uniffi::export]
pub fn malayalam_month(sun_sidereal: f64) -> String {
    lex::malayalam_month(sun_sidereal).to_string()
}

/// The Kollavarsham date. `utc_day` is days since 1970-01-01 of the civil day (it fixes the
/// Kollam year: `CE − 824` from Chingam 1 to 31 December, `CE − 825` before Chingam 1),
/// `sun_sidereal` the Sun's sidereal longitude (its sign is the month), and
/// `days_since_sankranti` the caller's count of whole days since the day reckoned as the
/// month's first (Kerala: the Sankranti day itself when the ingress is before aparahna, the
/// first three fifths of the daytime, else the next day), so day 1 is `0`.
#[uniffi::export]
pub fn kollavarsham(
    utc_day: i64,
    sun_sidereal: f64,
    days_since_sankranti: u32,
) -> KollavarshamRecord {
    let (year, month, day) = lex::kollavarsham(utc_day, sun_sidereal, days_since_sankranti);
    KollavarshamRecord {
        year,
        month: month as u32,
        month_name: lex::KOLLAVARSHAM_MONTHS[month].to_string(),
        day,
    }
}

/// One paragraph on Vakya versus Drik panchanga, stating that Kshana is a Drik panchanga.
#[uniffi::export]
pub fn vakya_note() -> String {
    lex::vakya_note().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn lexicon_through_the_boundary() {
        assert_eq!(term("Ashwini".into(), "ml".into()), "അശ്വതി");
        assert_eq!(term("Mesha".into(), "ta".into()), "மேஷம்");
        assert_eq!(term("Ravivara".into(), "en".into()), "Sunday");
        assert_eq!(term("Choghadiya".into(), "ta".into()), "Choghadiya");
        assert_eq!(term("unknown".into(), "xx".into()), "unknown");
        assert_eq!(term_latin("Ashwini".into(), "ml".into()), "Aswathi");
        assert_eq!(term_latin("Aries".into(), "en".into()), "Mesha");

        let langs = language_names();
        assert_eq!(langs.len(), 6);
        assert_eq!(langs[1].code, "ml");
        assert_eq!(langs[1].native, "മലയാളം");
        assert_eq!(langs[1].english, "Malayalam");

        let ml = lexicon("ml".into());
        assert!(ml.iter().filter(|e| e.family == "nakshatra").count() == 27);
        assert!(ml.iter().all(|e| !e.english.is_empty()));
        let kn = lexicon("kn".into());
        assert!(
            kn.iter().any(|e| e.native.is_empty()),
            "unverified cells stay empty"
        );

        let u = |h, mi| {
            instant_from_utc(UtcFields {
                year: 2026,
                month: 10,
                day: 19,
                hour: h,
                minute: mi,
                second: 0,
                nanos: 0,
            })
            .unwrap()
        };
        let sunrise = u(0, 50);
        let n = nazhika(sunrise, sunrise);
        assert_eq!((n.nazhika, n.vinazhika), (0, 0));
        let n = nazhika(sunrise, u(1, 15));
        assert_eq!((n.nazhika, n.vinazhika), (1, 2));
        assert!((n.vilipta - 30.0).abs() < 1e-9);

        assert_eq!(tamil_month(182.0), "Aippasi");
        assert_eq!(malayalam_month(182.0), "Thulam");
        let k = kollavarsham(20_745, 182.0, 2); // 19 October 2026
        assert_eq!((k.year, k.month, k.day), (1202, 2, 3));
        assert_eq!(k.month_name, "Thulam");
        assert!(vakya_note().contains("Drik"));
    }
}
