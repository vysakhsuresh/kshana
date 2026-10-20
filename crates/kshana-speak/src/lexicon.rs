//! Regional terminology: every name the app shows, in the scripts the first users read.
//!
//! The tables cover the 12 rasis, 27 nakshatras, 30 tithis, 7 varas, 27 yogas, 11 karanas,
//! 9 grahas, 12 bhavas, the Vimshottari levels, the day clocks (muhurta, hora, ghati,
//! nazhika …), the observances (Ekadashi, Pradosha, Sankranti) and three month systems
//! (Kollavarsham, Tamil solar, Saka/Vikrama lunar) in Malayalam, Tamil, Hindi, Telugu
//! and Kannada, each cell carrying the native script and a Latin transliteration.
//!
//! **Transliteration convention.** The Latin column is the common romanisation used by
//! Kerala and Tamil Nadu almanacs and by Indian-language apps (Aswathi, Thiruvathira,
//! Makayiram; Valarpirai, Mesham), not ISO 15919. It has no diacritics so it can be typed
//! into a search box. The English key is the engine's own spelling (`NAKSHATRA_NAMES`,
//! `YOGA_NAMES`, the sign and planet names), so a string the engine emits can be handed
//! straight to [`term`].
//!
//! **Verification.** Every table was checked against published lists (the sources are in
//! the doc comment above each table). A cell that could not be matched to a source is left
//! empty, and [`term`] then returns the English key, so nothing invented reaches the screen.
//!
//! Besides the tables this module holds the regional readouts the Malayalam panchangam
//! users expect: udayadi nazhika and vinazhika since sunrise, the Kollavarsham date, the
//! Tamil solar month, and a note on why a Drik panchanga differs from a Vakya almanac.

use kshana_time::{civil_from_days, Instant, NS_PER_DAY, NS_PER_SEC};

/// A language the lexicon speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    /// English, the engine's own names (always complete).
    English,
    /// Malayalam, Kerala: Kollavarsham, nazhika, the Malayalam panchangam.
    Malayalam,
    /// Tamil: Tamil solar months, Thirukkanitha and Vakya almanacs.
    Tamil,
    /// Hindi (Devanagari): the Sanskrit forms as printed in North Indian panchangs.
    Hindi,
    /// Telugu: Telugu panchangam usage (Suddha/Bahula paksha, Padyami …).
    Telugu,
    /// Kannada: Kannada panchanga usage (Padya, Bidige, Hunnime …).
    Kannada,
}

impl Language {
    /// Every language, English first.
    pub const ALL: [Language; 6] = [
        Language::English,
        Language::Malayalam,
        Language::Tamil,
        Language::Hindi,
        Language::Telugu,
        Language::Kannada,
    ];

    /// ISO 639-1 code: `en`, `ml`, `ta`, `hi`, `te`, `kn`.
    pub const fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Malayalam => "ml",
            Language::Tamil => "ta",
            Language::Hindi => "hi",
            Language::Telugu => "te",
            Language::Kannada => "kn",
        }
    }

    /// The language's own name for itself.
    pub const fn native_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Malayalam => "മലയാളം",
            Language::Tamil => "தமிழ்",
            Language::Hindi => "हिन्दी",
            Language::Telugu => "తెలుగు",
            Language::Kannada => "ಕನ್ನಡ",
        }
    }

    /// The language's English name.
    pub const fn english_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Malayalam => "Malayalam",
            Language::Tamil => "Tamil",
            Language::Hindi => "Hindi",
            Language::Telugu => "Telugu",
            Language::Kannada => "Kannada",
        }
    }

    /// The language for a code (`en`, `ml`, `ta`, `hi`, `te`, `kn`; case does not matter),
    /// or `None` for anything else.
    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL
            .into_iter()
            .find(|l| l.code().eq_ignore_ascii_case(code.trim()))
    }

    const fn column(self) -> usize {
        match self {
            Language::English => 0,
            Language::Malayalam => 1,
            Language::Tamil => 2,
            Language::Hindi => 3,
            Language::Telugu => 4,
            Language::Kannada => 5,
        }
    }
}

/// `(code, native name)` for every language, English first: `("ml", "മലയാളം")`.
pub fn language_names() -> Vec<(&'static str, &'static str)> {
    Language::ALL
        .iter()
        .map(|l| (l.code(), l.native_name()))
        .collect()
}

/// One cell: `(native script, Latin transliteration)`. Both empty when unverified.
type Cell = (&'static str, &'static str);

/// One term: the English key and its six cells (English, Malayalam, Tamil, Hindi,
/// Telugu, Kannada). The English cell is `(display name, Sanskrit or alternate name)`.
#[derive(Clone, Copy, Debug)]
struct Row {
    key: &'static str,
    cells: [Cell; 6],
}

const fn r(key: &'static str, en: Cell, ml: Cell, ta: Cell, hi: Cell, te: Cell, kn: Cell) -> Row {
    Row {
        key,
        cells: [en, ml, ta, hi, te, kn],
    }
}

/// One term of the lexicon in one language, for a screen that lists everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconEntry {
    /// Family: one of [`FAMILIES`].
    pub family: &'static str,
    /// The key the engine uses (`"Ashwini"`, `"Shukla Pratipada"`, `"Rahu kala"`).
    pub key: &'static str,
    /// English display name (`"Sunday"` for `"Ravivara"`, `"1st house"` for `"Tanu"`).
    pub english: &'static str,
    /// Sanskrit or alternate English name, or a one-line gloss (`"Mesha"` for Aries).
    pub sanskrit: &'static str,
    /// Native script in the requested language; empty when unverified.
    pub native: &'static str,
    /// Latin transliteration in the requested language; empty when unverified.
    pub latin: &'static str,
}

/// Family names in display order.
pub const FAMILIES: [&str; 14] = [
    "rasi",
    "nakshatra",
    "tithi",
    "vara",
    "yoga",
    "karana",
    "graha",
    "bhava",
    "period",
    "clock",
    "observance",
    "month_kolla",
    "month_tamil",
    "month_lunar",
];

/// The 12 rasis, keyed by the engine's Western sign names (`SIGN_NAMES`).
///
/// Sources: Malayalam — manoramaonline.com/astrology/astro-news/2019/09/20/what-is-rashi-in-horoscope.html,
/// ml.wikipedia.org/wiki/രാശിചക്രം, malayalamworld.in/zodiac-sign-in-malayalam;
/// Tamil — neerodai.com/12-rasikal-tamil, dheivegam.com/12-zodiac-signs-in-tamil,
/// languagereef.com (Tamil lessons: zodiac); Hindi — bhagyaasegyan.com/all-rashi-name-in-english-and-hindi,
/// hindi.astrosage.com/rashi; Telugu — te.wikipedia.org/wiki/రాశి, te.wikibooks.org/wiki/రాశులు;
/// Kannada — kn.wikipedia.org/wiki/ರಾಶಿ, astromitra.app/zodiac-signs?lang=kn.
static RASI: [Row; 12] = [
    r(
        "Aries",
        ("Aries", "Mesha"),
        ("മേടം", "Medam"),
        ("மேஷம்", "Mesham"),
        ("मेष", "Mesh"),
        ("మేషం", "Mesham"),
        ("ಮೇಷ", "Mesha"),
    ),
    r(
        "Taurus",
        ("Taurus", "Vrishabha"),
        ("ഇടവം", "Edavam"),
        ("ரிஷபம்", "Rishabam"),
        ("वृषभ", "Vrishabh"),
        ("వృషభం", "Vrushabham"),
        ("ವೃಷಭ", "Vrushabha"),
    ),
    r(
        "Gemini",
        ("Gemini", "Mithuna"),
        ("മിഥുനം", "Mithunam"),
        ("மிதுனம்", "Mithunam"),
        ("मिथुन", "Mithun"),
        ("మిథునం", "Mithunam"),
        ("ಮಿಥುನ", "Mithuna"),
    ),
    r(
        "Cancer",
        ("Cancer", "Karka"),
        ("കർക്കടകം", "Karkidakam"),
        ("கடகம்", "Kadagam"),
        ("कर्क", "Kark"),
        ("కర్కాటకం", "Karkatakam"),
        ("ಕಟಕ", "Kataka"),
    ),
    r(
        "Leo",
        ("Leo", "Simha"),
        ("ചിങ്ങം", "Chingam"),
        ("சிம்மம்", "Simmam"),
        ("सिंह", "Simha"),
        ("సింహం", "Simham"),
        ("ಸಿಂಹ", "Simha"),
    ),
    r(
        "Virgo",
        ("Virgo", "Kanya"),
        ("കന്നി", "Kanni"),
        ("கன்னி", "Kanni"),
        ("कन्या", "Kanya"),
        ("కన్య", "Kanya"),
        ("ಕನ್ಯಾ", "Kanya"),
    ),
    r(
        "Libra",
        ("Libra", "Tula"),
        ("തുലാം", "Thulam"),
        ("துலாம்", "Thulam"),
        ("तुला", "Tula"),
        ("తుల", "Tula"),
        ("ತುಲಾ", "Tula"),
    ),
    r(
        "Scorpio",
        ("Scorpio", "Vrischika"),
        ("വൃശ്ചികം", "Vrischikam"),
        ("விருச்சிகம்", "Viruchigam"),
        ("वृश्चिक", "Vrishchik"),
        ("వృశ్చికం", "Vruschikam"),
        ("ವೃಶ್ಚಿಕ", "Vrushchika"),
    ),
    r(
        "Sagittarius",
        ("Sagittarius", "Dhanu"),
        ("ധനു", "Dhanu"),
        ("தனுசு", "Dhanusu"),
        ("धनु", "Dhanu"),
        ("ధనుస్సు", "Dhanussu"),
        ("ಧನು", "Dhanu"),
    ),
    r(
        "Capricorn",
        ("Capricorn", "Makara"),
        ("മകരം", "Makaram"),
        ("மகரம்", "Magaram"),
        ("मकर", "Makar"),
        ("మకరం", "Makaram"),
        ("ಮಕರ", "Makara"),
    ),
    r(
        "Aquarius",
        ("Aquarius", "Kumbha"),
        ("കുംഭം", "Kumbham"),
        ("கும்பம்", "Kumbam"),
        ("कुंभ", "Kumbh"),
        ("కుంభం", "Kumbham"),
        ("ಕುಂಭ", "Kumbha"),
    ),
    r(
        "Pisces",
        ("Pisces", "Meena"),
        ("മീനം", "Meenam"),
        ("மீனம்", "Meenam"),
        ("मीन", "Meen"),
        ("మీనం", "Meenam"),
        ("ಮೀನ", "Meena"),
    ),
];

/// The 27 nakshatras, keyed by `kshana_vedic::panchanga::NAKSHATRA_NAMES`.
///
/// Sources: Malayalam — hindupad.com/malayalam-nakshatra-list-27-janma-naal-in-malayali-calendar,
/// srirajpulari.blogspot.com/2013/03/nakshatras.html; Tamil — hindupad.com/tamil-nakshatra-names-nakshatras-list-in-tamil-astrology,
/// divineinfoguru.com (27 நட்சத்திரங்கள் பெயர்கள்); Hindi — drikpanchang.com/tutorials/nakshatra/nakshatra.html?lang=hi,
/// grahaguru.in/hi/blog/nakshatra-complete-guide; Telugu — everythingintelugu.blogspot.com/2016/03/27-telugu-nakshtralu.html,
/// nityapanchangam.com/nakshatras (Pubba, Uttara, Chitta, Sravanam, Satabhisham are the Telugu almanac forms);
/// Kannada — crispwave.in/27-nakshatra, techiedj.in/27-nakshatra (Hubba and Uttara are the Kannada names
/// of the two Phalgunis).
static NAKSHATRA: [Row; 27] = [
    r(
        "Ashwini",
        ("Ashwini", "Ashwini"),
        ("അശ്വതി", "Aswathi"),
        ("அசுவினி", "Aswini"),
        ("अश्विनी", "Ashwini"),
        ("అశ్విని", "Ashwini"),
        ("ಅಶ್ವಿನಿ", "Ashwini"),
    ),
    r(
        "Bharani",
        ("Bharani", "Bharani"),
        ("ഭരണി", "Bharani"),
        ("பரணி", "Bharani"),
        ("भरणी", "Bharani"),
        ("భరణి", "Bharani"),
        ("ಭರಣಿ", "Bharani"),
    ),
    r(
        "Krittika",
        ("Krittika", "Krittika"),
        ("കാർത്തിക", "Karthika"),
        ("கார்த்திகை", "Karthigai"),
        ("कृत्तिका", "Krittika"),
        ("కృత్తిక", "Krittika"),
        ("ಕೃತ್ತಿಕಾ", "Krittika"),
    ),
    r(
        "Rohini",
        ("Rohini", "Rohini"),
        ("രോഹിണി", "Rohini"),
        ("ரோகிணி", "Rohini"),
        ("रोहिणी", "Rohini"),
        ("రోహిణి", "Rohini"),
        ("ರೋಹಿಣಿ", "Rohini"),
    ),
    r(
        "Mrigashira",
        ("Mrigashira", "Mrigashirsha"),
        ("മകയിരം", "Makayiram"),
        ("மிருகசீரிடம்", "Mirugaseeridam"),
        ("मृगशिरा", "Mrigashira"),
        ("మృగశిర", "Mrigasira"),
        ("ಮೃಗಶಿರ", "Mrigashira"),
    ),
    r(
        "Ardra",
        ("Ardra", "Ardra"),
        ("തിരുവാതിര", "Thiruvathira"),
        ("திருவாதிரை", "Thiruvathirai"),
        ("आर्द्रा", "Ardra"),
        ("ఆర్ద్ర", "Arudra"),
        ("ಆರ್ದ್ರಾ", "Ardra"),
    ),
    r(
        "Punarvasu",
        ("Punarvasu", "Punarvasu"),
        ("പുണർതം", "Punartham"),
        ("புனர்பூசம்", "Punarpoosam"),
        ("पुनर्वसु", "Punarvasu"),
        ("పునర్వసు", "Punarvasu"),
        ("ಪುನರ್ವಸು", "Punarvasu"),
    ),
    r(
        "Pushya",
        ("Pushya", "Pushya"),
        ("പൂയം", "Pooyam"),
        ("பூசம்", "Poosam"),
        ("पुष्य", "Pushya"),
        ("పుష్యమి", "Pushyami"),
        ("ಪುಷ್ಯ", "Pushya"),
    ),
    r(
        "Ashlesha",
        ("Ashlesha", "Ashlesha"),
        ("ആയില്യം", "Ayilyam"),
        ("ஆயில்யம்", "Ayilyam"),
        ("आश्लेषा", "Ashlesha"),
        ("ఆశ్లేష", "Aslesha"),
        ("ಆಶ್ಲೇಷ", "Ashlesha"),
    ),
    r(
        "Magha",
        ("Magha", "Magha"),
        ("മകം", "Makam"),
        ("மகம்", "Magam"),
        ("मघा", "Magha"),
        ("మఖ", "Makha"),
        ("ಮಖ", "Makha"),
    ),
    r(
        "Purva Phalguni",
        ("Purva Phalguni", "Purva Phalguni"),
        ("പൂരം", "Pooram"),
        ("பூரம்", "Pooram"),
        ("पूर्वाफाल्गुनी", "Purva Phalguni"),
        ("పుబ్బ", "Pubba"),
        ("ಹುಬ್ಬ", "Hubba"),
    ),
    r(
        "Uttara Phalguni",
        ("Uttara Phalguni", "Uttara Phalguni"),
        ("ഉത്രം", "Uthram"),
        ("உத்திரம்", "Uthiram"),
        ("उत्तराफाल्गुनी", "Uttara Phalguni"),
        ("ఉత్తర", "Uttara"),
        ("ಉತ್ತರ", "Uttara"),
    ),
    r(
        "Hasta",
        ("Hasta", "Hasta"),
        ("അത്തം", "Atham"),
        ("அஸ்தம்", "Hastham"),
        ("हस्त", "Hasta"),
        ("హస్త", "Hasta"),
        ("ಹಸ್ತ", "Hasta"),
    ),
    r(
        "Chitra",
        ("Chitra", "Chitra"),
        ("ചിത്തിര", "Chithira"),
        ("சித்திரை", "Chithirai"),
        ("चित्रा", "Chitra"),
        ("చిత్త", "Chitta"),
        ("ಚಿತ್ತಾ", "Chitta"),
    ),
    r(
        "Swati",
        ("Swati", "Swati"),
        ("ചോതി", "Chothi"),
        ("சுவாதி", "Swathi"),
        ("स्वाति", "Swati"),
        ("స్వాతి", "Swati"),
        ("ಸ್ವಾತಿ", "Swati"),
    ),
    r(
        "Vishakha",
        ("Vishakha", "Vishakha"),
        ("വിശാഖം", "Vishakham"),
        ("விசாகம்", "Visakam"),
        ("विशाखा", "Vishakha"),
        ("విశాఖ", "Visakha"),
        ("ವಿಶಾಖ", "Vishakha"),
    ),
    r(
        "Anuradha",
        ("Anuradha", "Anuradha"),
        ("അനിഴം", "Anizham"),
        ("அனுஷம்", "Anusham"),
        ("अनुराधा", "Anuradha"),
        ("అనూరాధ", "Anuradha"),
        ("ಅನುರಾಧ", "Anuradha"),
    ),
    r(
        "Jyeshtha",
        ("Jyeshtha", "Jyeshtha"),
        ("തൃക്കേട്ട", "Thrikketta"),
        ("கேட்டை", "Kettai"),
        ("ज्येष्ठा", "Jyeshtha"),
        ("జ్యేష్ఠ", "Jyeshta"),
        ("ಜ್ಯೇಷ್ಠ", "Jyeshtha"),
    ),
    r(
        "Mula",
        ("Mula", "Mula"),
        ("മൂലം", "Moolam"),
        ("மூலம்", "Moolam"),
        ("मूल", "Mula"),
        ("మూల", "Moola"),
        ("ಮೂಲ", "Moola"),
    ),
    r(
        "Purva Ashadha",
        ("Purva Ashadha", "Purva Ashadha"),
        ("പൂരാടം", "Pooradam"),
        ("பூராடம்", "Pooradam"),
        ("पूर्वाषाढ़ा", "Purvashadha"),
        ("పూర్వాషాఢ", "Purvashadha"),
        ("ಪೂರ್ವಾಷಾಢ", "Purvashadha"),
    ),
    r(
        "Uttara Ashadha",
        ("Uttara Ashadha", "Uttara Ashadha"),
        ("ഉത്രാടം", "Uthradam"),
        ("உத்திராடம்", "Uthiradam"),
        ("उत्तराषाढ़ा", "Uttarashadha"),
        ("ఉత్తరాషాఢ", "Uttarashadha"),
        ("ಉತ್ತರಾಷಾಢ", "Uttarashadha"),
    ),
    r(
        "Shravana",
        ("Shravana", "Shravana"),
        ("തിരുവോണം", "Thiruvonam"),
        ("திருவோணம்", "Thiruvonam"),
        ("श्रवण", "Shravana"),
        ("శ్రవణం", "Sravanam"),
        ("ಶ್ರವಣ", "Shravana"),
    ),
    r(
        "Dhanishta",
        ("Dhanishta", "Dhanishtha"),
        ("അവിട്ടം", "Avittam"),
        ("அவிட்டம்", "Avittam"),
        ("धनिष्ठा", "Dhanishtha"),
        ("ధనిష్ఠ", "Dhanishta"),
        ("ಧನಿಷ್ಠ", "Dhanishtha"),
    ),
    r(
        "Shatabhisha",
        ("Shatabhisha", "Shatabhisha"),
        ("ചതയം", "Chathayam"),
        ("சதயம்", "Sadayam"),
        ("शतभिषा", "Shatabhisha"),
        ("శతభిషం", "Satabhisham"),
        ("ಶತಭಿಷ", "Shatabhisha"),
    ),
    r(
        "Purva Bhadrapada",
        ("Purva Bhadrapada", "Purva Bhadrapada"),
        ("പൂരുരുട്ടാതി", "Pooruruttathi"),
        ("பூரட்டாதி", "Poorattathi"),
        ("पूर्वाभाद्रपद", "Purva Bhadrapada"),
        ("పూర్వాభాద్ర", "Purvabhadra"),
        ("ಪೂರ್ವಾಭಾದ್ರ", "Purvabhadra"),
    ),
    r(
        "Uttara Bhadrapada",
        ("Uttara Bhadrapada", "Uttara Bhadrapada"),
        ("ഉത്രട്ടാതി", "Uthrattathi"),
        ("உத்திரட்டாதி", "Uthirattathi"),
        ("उत्तराभाद्रपद", "Uttara Bhadrapada"),
        ("ఉత్తరాభాద్ర", "Uttarabhadra"),
        ("ಉತ್ತರಾಭಾದ್ರ", "Uttarabhadra"),
    ),
    r(
        "Revati",
        ("Revati", "Revati"),
        ("രേവതി", "Revathi"),
        ("ரேவதி", "Revathi"),
        ("रेवती", "Revati"),
        ("రేవతి", "Revati"),
        ("ರೇವತಿ", "Revati"),
    ),
];

/// The 15 tithis of the bright fortnight, keyed `"Shukla <name>"` as the engine's
/// `PanchangaDetail.tithi` prints them; the fifteenth is `"Purnima"`.
///
/// Paksha words: Malayalam ശുക്ലപക്ഷം/കൃഷ്ണപക്ഷം and the day names വെളുത്തവാവ്/കറുത്തവാവ്
/// (ml.wikipedia.org/wiki/തിഥി, janmabhumi.in/2021/05/29/3000088/samskriti/article-on-thithi:
/// "ശുക്ല പ്രഥമ, ശുക്ല ദ്വിതീയ എന്നിങ്ങനെ"); Tamil வளர்பிறை/தேய்பிறை
/// (ta.wikipedia.org/wiki/திதி_(பஞ்சாங்கம்), astrotamil.in/blog/30-thithigal-pattiyal); Hindi शुक्ल/कृष्ण पक्ष
/// (amarujala.com daily panchang: "शुक्ल पक्ष सप्तमी तिथि"); Telugu శుద్ధ/బహుళ with Padyami, Vidiya, Thadiya,
/// Chavithi (te.wikisource.org/wiki/తిధులు, te.wiktionary.org/wiki/తిథులు, telugukidsinfo.blogspot.com
/// "Suddha Padyami"); Kannada ಶುಕ್ಲ/ಕೃಷ್ಣ with Padya, Bidige, Tadige, Chauthi, Hunnime, Amavasye
/// (kn.wikipedia.org/wiki/ಪಂಚಾಂಗ, kn.wiktionary.org/wiki/ಪಂಚದಶ_ತಿಥಿಗಳು).
static TITHI_SHUKLA: [Row; 15] = [
    r(
        "Shukla Pratipada",
        ("Shukla Pratipada", "1st, bright"),
        ("ശുക്ല പ്രഥമ", "Shukla Prathama"),
        ("வளர்பிறை பிரதமை", "Valarpirai Prathamai"),
        ("शुक्ल प्रतिपदा", "Shukla Pratipada"),
        ("శుద్ధ పాడ్యమి", "Suddha Padyami"),
        ("ಶುಕ್ಲ ಪಾಡ್ಯ", "Shukla Padya"),
    ),
    r(
        "Shukla Dwitiya",
        ("Shukla Dwitiya", "2nd, bright"),
        ("ശുക്ല ദ്വിതീയ", "Shukla Dwitheeya"),
        ("வளர்பிறை துவிதியை", "Valarpirai Thuvithiyai"),
        ("शुक्ल द्वितीया", "Shukla Dwitiya"),
        ("శుద్ధ విదియ", "Suddha Vidiya"),
        ("ಶುಕ್ಲ ಬಿದಿಗೆ", "Shukla Bidige"),
    ),
    r(
        "Shukla Tritiya",
        ("Shukla Tritiya", "3rd, bright"),
        ("ശുക്ല തൃതീയ", "Shukla Thritheeya"),
        ("வளர்பிறை திருதியை", "Valarpirai Thirithiyai"),
        ("शुक्ल तृतीया", "Shukla Tritiya"),
        ("శుద్ధ తదియ", "Suddha Thadiya"),
        ("ಶುಕ್ಲ ತದಿಗೆ", "Shukla Tadige"),
    ),
    r(
        "Shukla Chaturthi",
        ("Shukla Chaturthi", "4th, bright"),
        ("ശുക്ല ചതുർത്ഥി", "Shukla Chathurthi"),
        ("வளர்பிறை சதுர்த்தி", "Valarpirai Chathurthi"),
        ("शुक्ल चतुर्थी", "Shukla Chaturthi"),
        ("శుద్ధ చవితి", "Suddha Chavithi"),
        ("ಶುಕ್ಲ ಚೌತಿ", "Shukla Chauthi"),
    ),
    r(
        "Shukla Panchami",
        ("Shukla Panchami", "5th, bright"),
        ("ശുക്ല പഞ്ചമി", "Shukla Panchami"),
        ("வளர்பிறை பஞ்சமி", "Valarpirai Panchami"),
        ("शुक्ल पंचमी", "Shukla Panchami"),
        ("శుద్ధ పంచమి", "Suddha Panchami"),
        ("ಶುಕ್ಲ ಪಂಚಮಿ", "Shukla Panchami"),
    ),
    r(
        "Shukla Shashthi",
        ("Shukla Shashthi", "6th, bright"),
        ("ശുക്ല ഷഷ്ഠി", "Shukla Shashti"),
        ("வளர்பிறை சஷ்டி", "Valarpirai Sashti"),
        ("शुक्ल षष्ठी", "Shukla Shashthi"),
        ("శుద్ధ షష్ఠి", "Suddha Shashti"),
        ("ಶುಕ್ಲ ಷಷ್ಠಿ", "Shukla Shashti"),
    ),
    r(
        "Shukla Saptami",
        ("Shukla Saptami", "7th, bright"),
        ("ശുക്ല സപ്തമി", "Shukla Sapthami"),
        ("வளர்பிறை சப்தமி", "Valarpirai Sapthami"),
        ("शुक्ल सप्तमी", "Shukla Saptami"),
        ("శుద్ధ సప్తమి", "Suddha Sapthami"),
        ("ಶುಕ್ಲ ಸಪ್ತಮಿ", "Shukla Saptami"),
    ),
    r(
        "Shukla Ashtami",
        ("Shukla Ashtami", "8th, bright"),
        ("ശുക്ല അഷ്ടമി", "Shukla Ashtami"),
        ("வளர்பிறை அஷ்டமி", "Valarpirai Ashtami"),
        ("शुक्ल अष्टमी", "Shukla Ashtami"),
        ("శుద్ధ అష్టమి", "Suddha Ashtami"),
        ("ಶುಕ್ಲ ಅಷ್ಟಮಿ", "Shukla Ashtami"),
    ),
    r(
        "Shukla Navami",
        ("Shukla Navami", "9th, bright"),
        ("ശുക്ല നവമി", "Shukla Navami"),
        ("வளர்பிறை நவமி", "Valarpirai Navami"),
        ("शुक्ल नवमी", "Shukla Navami"),
        ("శుద్ధ నవమి", "Suddha Navami"),
        ("ಶುಕ್ಲ ನವಮಿ", "Shukla Navami"),
    ),
    r(
        "Shukla Dashami",
        ("Shukla Dashami", "10th, bright"),
        ("ശുക്ല ദശമി", "Shukla Dashami"),
        ("வளர்பிறை தசமி", "Valarpirai Dasami"),
        ("शुक्ल दशमी", "Shukla Dashami"),
        ("శుద్ధ దశమి", "Suddha Dasami"),
        ("ಶುಕ್ಲ ದಶಮಿ", "Shukla Dashami"),
    ),
    r(
        "Shukla Ekadashi",
        ("Shukla Ekadashi", "11th, bright"),
        ("ശുക്ല ഏകാദശി", "Shukla Ekadashi"),
        ("வளர்பிறை ஏகாதசி", "Valarpirai Ekadasi"),
        ("शुक्ल एकादशी", "Shukla Ekadashi"),
        ("శుద్ధ ఏకాదశి", "Suddha Ekadasi"),
        ("ಶುಕ್ಲ ಏಕಾದಶಿ", "Shukla Ekadashi"),
    ),
    r(
        "Shukla Dwadashi",
        ("Shukla Dwadashi", "12th, bright"),
        ("ശുക്ല ദ്വാദശി", "Shukla Dwadashi"),
        ("வளர்பிறை துவாதசி", "Valarpirai Thuvadasi"),
        ("शुक्ल द्वादशी", "Shukla Dwadashi"),
        ("శుద్ధ ద్వాదశి", "Suddha Dwadasi"),
        ("ಶುಕ್ಲ ದ್ವಾದಶಿ", "Shukla Dwadashi"),
    ),
    r(
        "Shukla Trayodashi",
        ("Shukla Trayodashi", "13th, bright"),
        ("ശുക്ല ത്രയോദശി", "Shukla Thrayodashi"),
        ("வளர்பிறை திரயோதசி", "Valarpirai Thirayodasi"),
        ("शुक्ल त्रयोदशी", "Shukla Trayodashi"),
        ("శుద్ధ త్రయోదశి", "Suddha Trayodasi"),
        ("ಶುಕ್ಲ ತ್ರಯೋದಶಿ", "Shukla Trayodashi"),
    ),
    r(
        "Shukla Chaturdashi",
        ("Shukla Chaturdashi", "14th, bright"),
        ("ശുക്ല ചതുർദശി", "Shukla Chathurdashi"),
        ("வளர்பிறை சதுர்த்தசி", "Valarpirai Chathurthasi"),
        ("शुक्ल चतुर्दशी", "Shukla Chaturdashi"),
        ("శుద్ధ చతుర్దశి", "Suddha Chaturdasi"),
        ("ಶುಕ್ಲ ಚತುರ್ದಶಿ", "Shukla Chaturdashi"),
    ),
    r(
        "Purnima",
        ("Purnima", "Full Moon, 15th bright"),
        ("പൗർണമി (വെളുത്തവാവ്)", "Pournami (Veluthavavu)"),
        ("பௌர்ணமி", "Pournami"),
        ("पूर्णिमा", "Purnima"),
        ("పౌర్ణమి", "Pournami"),
        ("ಹುಣ್ಣಿಮೆ", "Hunnime"),
    ),
];

/// The 15 tithis of the dark fortnight, keyed `"Krishna <name>"`; the thirtieth is `"Amavasya"`.
/// Sources as for [`TITHI_SHUKLA`].
static TITHI_KRISHNA: [Row; 15] = [
    r(
        "Krishna Pratipada",
        ("Krishna Pratipada", "1st, dark"),
        ("കൃഷ്ണ പ്രഥമ", "Krishna Prathama"),
        ("தேய்பிறை பிரதமை", "Theipirai Prathamai"),
        ("कृष्ण प्रतिपदा", "Krishna Pratipada"),
        ("బహుళ పాడ్యమి", "Bahula Padyami"),
        ("ಕೃಷ್ಣ ಪಾಡ್ಯ", "Krishna Padya"),
    ),
    r(
        "Krishna Dwitiya",
        ("Krishna Dwitiya", "2nd, dark"),
        ("കൃഷ്ണ ദ്വിതീയ", "Krishna Dwitheeya"),
        ("தேய்பிறை துவிதியை", "Theipirai Thuvithiyai"),
        ("कृष्ण द्वितीया", "Krishna Dwitiya"),
        ("బహుళ విదియ", "Bahula Vidiya"),
        ("ಕೃಷ್ಣ ಬಿದಿಗೆ", "Krishna Bidige"),
    ),
    r(
        "Krishna Tritiya",
        ("Krishna Tritiya", "3rd, dark"),
        ("കൃഷ്ണ തൃതീയ", "Krishna Thritheeya"),
        ("தேய்பிறை திருதியை", "Theipirai Thirithiyai"),
        ("कृष्ण तृतीया", "Krishna Tritiya"),
        ("బహుళ తదియ", "Bahula Thadiya"),
        ("ಕೃಷ್ಣ ತದಿಗೆ", "Krishna Tadige"),
    ),
    r(
        "Krishna Chaturthi",
        ("Krishna Chaturthi", "4th, dark"),
        ("കൃഷ്ണ ചതുർത്ഥി", "Krishna Chathurthi"),
        ("தேய்பிறை சதுர்த்தி", "Theipirai Chathurthi"),
        ("कृष्ण चतुर्थी", "Krishna Chaturthi"),
        ("బహుళ చవితి", "Bahula Chavithi"),
        ("ಕೃಷ್ಣ ಚೌತಿ", "Krishna Chauthi"),
    ),
    r(
        "Krishna Panchami",
        ("Krishna Panchami", "5th, dark"),
        ("കൃഷ്ണ പഞ്ചമി", "Krishna Panchami"),
        ("தேய்பிறை பஞ்சமி", "Theipirai Panchami"),
        ("कृष्ण पंचमी", "Krishna Panchami"),
        ("బహుళ పంచమి", "Bahula Panchami"),
        ("ಕೃಷ್ಣ ಪಂಚಮಿ", "Krishna Panchami"),
    ),
    r(
        "Krishna Shashthi",
        ("Krishna Shashthi", "6th, dark"),
        ("കൃഷ്ണ ഷഷ്ഠി", "Krishna Shashti"),
        ("தேய்பிறை சஷ்டி", "Theipirai Sashti"),
        ("कृष्ण षष्ठी", "Krishna Shashthi"),
        ("బహుళ షష్ఠి", "Bahula Shashti"),
        ("ಕೃಷ್ಣ ಷಷ್ಠಿ", "Krishna Shashti"),
    ),
    r(
        "Krishna Saptami",
        ("Krishna Saptami", "7th, dark"),
        ("കൃഷ്ണ സപ്തമി", "Krishna Sapthami"),
        ("தேய்பிறை சப்தமி", "Theipirai Sapthami"),
        ("कृष्ण सप्तमी", "Krishna Saptami"),
        ("బహుళ సప్తమి", "Bahula Sapthami"),
        ("ಕೃಷ್ಣ ಸಪ್ತಮಿ", "Krishna Saptami"),
    ),
    r(
        "Krishna Ashtami",
        ("Krishna Ashtami", "8th, dark"),
        ("കൃഷ്ണ അഷ്ടമി", "Krishna Ashtami"),
        ("தேய்பிறை அஷ்டமி", "Theipirai Ashtami"),
        ("कृष्ण अष्टमी", "Krishna Ashtami"),
        ("బహుళ అష్టమి", "Bahula Ashtami"),
        ("ಕೃಷ್ಣ ಅಷ್ಟಮಿ", "Krishna Ashtami"),
    ),
    r(
        "Krishna Navami",
        ("Krishna Navami", "9th, dark"),
        ("കൃഷ്ണ നവമി", "Krishna Navami"),
        ("தேய்பிறை நவமி", "Theipirai Navami"),
        ("कृष्ण नवमी", "Krishna Navami"),
        ("బహుళ నవమి", "Bahula Navami"),
        ("ಕೃಷ್ಣ ನವಮಿ", "Krishna Navami"),
    ),
    r(
        "Krishna Dashami",
        ("Krishna Dashami", "10th, dark"),
        ("കൃഷ്ണ ദശമി", "Krishna Dashami"),
        ("தேய்பிறை தசமி", "Theipirai Dasami"),
        ("कृष्ण दशमी", "Krishna Dashami"),
        ("బహుళ దశమి", "Bahula Dasami"),
        ("ಕೃಷ್ಣ ದಶಮಿ", "Krishna Dashami"),
    ),
    r(
        "Krishna Ekadashi",
        ("Krishna Ekadashi", "11th, dark"),
        ("കൃഷ്ണ ഏകാദശി", "Krishna Ekadashi"),
        ("தேய்பிறை ஏகாதசி", "Theipirai Ekadasi"),
        ("कृष्ण एकादशी", "Krishna Ekadashi"),
        ("బహుళ ఏకాదశి", "Bahula Ekadasi"),
        ("ಕೃಷ್ಣ ಏಕಾದಶಿ", "Krishna Ekadashi"),
    ),
    r(
        "Krishna Dwadashi",
        ("Krishna Dwadashi", "12th, dark"),
        ("കൃഷ്ണ ദ്വാദശി", "Krishna Dwadashi"),
        ("தேய்பிறை துவாதசி", "Theipirai Thuvadasi"),
        ("कृष्ण द्वादशी", "Krishna Dwadashi"),
        ("బహుళ ద్వాదశి", "Bahula Dwadasi"),
        ("ಕೃಷ್ಣ ದ್ವಾದಶಿ", "Krishna Dwadashi"),
    ),
    r(
        "Krishna Trayodashi",
        ("Krishna Trayodashi", "13th, dark"),
        ("കൃഷ്ണ ത്രയോദശി", "Krishna Thrayodashi"),
        ("தேய்பிறை திரயோதசி", "Theipirai Thirayodasi"),
        ("कृष्ण त्रयोदशी", "Krishna Trayodashi"),
        ("బహుళ త్రయోదశి", "Bahula Trayodasi"),
        ("ಕೃಷ್ಣ ತ್ರಯೋದಶಿ", "Krishna Trayodashi"),
    ),
    r(
        "Krishna Chaturdashi",
        ("Krishna Chaturdashi", "14th, dark"),
        ("കൃഷ്ണ ചതുർദശി", "Krishna Chathurdashi"),
        ("தேய்பிறை சதுர்த்தசி", "Theipirai Chathurthasi"),
        ("कृष्ण चतुर्दशी", "Krishna Chaturdashi"),
        ("బహుళ చతుర్దశి", "Bahula Chaturdasi"),
        ("ಕೃಷ್ಣ ಚತುರ್ದಶಿ", "Krishna Chaturdashi"),
    ),
    r(
        "Amavasya",
        ("Amavasya", "New Moon, 30th"),
        ("അമാവാസി (കറുത്തവാവ്)", "Amavasi (Karuthavavu)"),
        ("அமாவாசை", "Amavasai"),
        ("अमावस्या", "Amavasya"),
        ("అమావాస్య", "Amavasya"),
        ("ಅಮಾವಾಸ್ಯೆ", "Amavasye"),
    ),
];

/// The 7 varas, keyed as `kshana_events::vara_name` prints them (`"Ravivara"` …).
///
/// Sources: Malayalam — malayalam-panchangam.blogspot.com/2012/12/malayalam-weekdays-name.html,
/// decodemalayalam.com/learn-days-and-months-in-malayalam-guide; Tamil — rhymesvideo.com/tamil/days-of-the-week.html,
/// turnupindia.com/7-week-days-name-in-tamil-and-english; Hindi — en.wiktionary.org/wiki/रविवार (and the
/// other days); Telugu — te.wikipedia.org/wiki/వారం; Kannada — en.wiktionary.org/wiki/Appendix:Days_of_the_week.
static VARA: [Row; 7] = [
    r(
        "Ravivara",
        ("Sunday", "Ravivara"),
        ("ഞായർ", "Njayar"),
        ("ஞாயிறு", "Nyayiru"),
        ("रविवार", "Ravivar"),
        ("ఆదివారం", "Adivaram"),
        ("ಭಾನುವಾರ", "Bhanuvara"),
    ),
    r(
        "Somavara",
        ("Monday", "Somavara"),
        ("തിങ്കൾ", "Thinkal"),
        ("திங்கள்", "Thingal"),
        ("सोमवार", "Somvar"),
        ("సోమవారం", "Somavaram"),
        ("ಸೋಮವಾರ", "Somavara"),
    ),
    r(
        "Mangalavara",
        ("Tuesday", "Mangalavara"),
        ("ചൊവ്വ", "Chovva"),
        ("செவ்வாய்", "Sevvai"),
        ("मंगलवार", "Mangalvar"),
        ("మంగళవారం", "Mangalavaram"),
        ("ಮಂಗಳವಾರ", "Mangalavara"),
    ),
    r(
        "Budhavara",
        ("Wednesday", "Budhavara"),
        ("ബുധൻ", "Budhan"),
        ("புதன்", "Budhan"),
        ("बुधवार", "Budhvar"),
        ("బుధవారం", "Budhavaram"),
        ("ಬುಧವಾರ", "Budhavara"),
    ),
    r(
        "Guruvara",
        ("Thursday", "Guruvara"),
        ("വ്യാഴം", "Vyazham"),
        ("வியாழன்", "Viyazhan"),
        ("गुरुवार", "Guruvar"),
        ("గురువారం", "Guruvaram"),
        ("ಗುರುವಾರ", "Guruvara"),
    ),
    r(
        "Shukravara",
        ("Friday", "Shukravara"),
        ("വെള്ളി", "Velli"),
        ("வெள்ளி", "Velli"),
        ("शुक्रवार", "Shukravar"),
        ("శుక్రవారం", "Shukravaram"),
        ("ಶುಕ್ರವಾರ", "Shukravara"),
    ),
    r(
        "Shanivara",
        ("Saturday", "Shanivara"),
        ("ശനി", "Shani"),
        ("சனி", "Sani"),
        ("शनिवार", "Shanivar"),
        ("శనివారం", "Shanivaram"),
        ("ಶನಿವಾರ", "Shanivara"),
    ),
];

/// The 27 nitya yogas, keyed by `kshana_vedic::panchanga::YOGA_NAMES`.
///
/// Sources: Hindi — nakshatraprabha.com/en/list-all-27-yogas.html, panchang.click/yoga-today (Devanagari);
/// Malayalam — nityapanchangam.com/ml, hindudevotionalblog.com/2021/07/27-nitya-yogas-in-astrology.html;
/// Tamil — dinakaran.com (27 நாம யோகங்கள்), ta.wikipedia.org/wiki/யோகம்_(பஞ்சாங்கம்);
/// Telugu — teluguone.com/devotional (యోగములు), te.wikipedia.org/wiki/యోగం_(పంచాంగం);
/// Kannada — kn.wikipedia.org/wiki/ಪಂಚಾಂಗ ("ವಿಷ್ಕಂಭಾದಿ ಇಪ್ಪತ್ತೇಳು ಯೋಗಗಳು"; the page names the first five
/// and daily Kannada panchangas print the rest, e.g. "ಶಿವ ಯೋಗ" on kannada.webdunia.com).
static YOGA: [Row; 27] = [
    r(
        "Vishkambha",
        ("Vishkambha", "Vishkambha"),
        ("വിഷ്കംഭം", "Vishkambham"),
        ("விஷ்கம்பம்", "Vishkambam"),
        ("विष्कम्भ", "Vishkambha"),
        ("విష్కంభం", "Vishkambham"),
        ("ವಿಷ್ಕಂಭ", "Vishkambha"),
    ),
    r(
        "Priti",
        ("Priti", "Priti"),
        ("പ്രീതി", "Preethi"),
        ("பிரீதி", "Preethi"),
        ("प्रीति", "Priti"),
        ("ప్రీతి", "Preethi"),
        ("ಪ್ರೀತಿ", "Preeti"),
    ),
    r(
        "Ayushman",
        ("Ayushman", "Ayushman"),
        ("ആയുഷ്മാൻ", "Ayushman"),
        ("ஆயுஷ்மான்", "Ayushman"),
        ("आयुष्मान", "Ayushman"),
        ("ఆయుష్మాన్", "Ayushman"),
        ("ಆಯುಷ್ಮಾನ್", "Ayushman"),
    ),
    r(
        "Saubhagya",
        ("Saubhagya", "Saubhagya"),
        ("സൗഭാഗ്യം", "Saubhagyam"),
        ("சௌபாக்கியம்", "Saubhagyam"),
        ("सौभाग्य", "Saubhagya"),
        ("సౌభాగ్యం", "Saubhagyam"),
        ("ಸೌಭಾಗ್ಯ", "Saubhagya"),
    ),
    r(
        "Shobhana",
        ("Shobhana", "Shobhana"),
        ("ശോഭനം", "Shobhanam"),
        ("சோபனம்", "Sobanam"),
        ("शोभन", "Shobhana"),
        ("శోభనం", "Shobhanam"),
        ("ಶೋಭನ", "Shobhana"),
    ),
    r(
        "Atiganda",
        ("Atiganda", "Atiganda"),
        ("അതിഗണ്ഡം", "Athigandam"),
        ("அதிகண்டம்", "Athigandam"),
        ("अतिगण्ड", "Atiganda"),
        ("అతిగండం", "Athigandam"),
        ("ಅತಿಗಂಡ", "Atiganda"),
    ),
    r(
        "Sukarma",
        ("Sukarma", "Sukarman"),
        ("സുകർമ്മം", "Sukarmam"),
        ("சுகர்மம்", "Sukarmam"),
        ("सुकर्मा", "Sukarma"),
        ("సుకర్మ", "Sukarma"),
        ("ಸುಕರ್ಮ", "Sukarma"),
    ),
    r(
        "Dhriti",
        ("Dhriti", "Dhriti"),
        ("ധൃതി", "Dhrithi"),
        ("திருதி", "Dhiruthi"),
        ("धृति", "Dhriti"),
        ("ధృతి", "Dhruthi"),
        ("ಧೃತಿ", "Dhruti"),
    ),
    r(
        "Shula",
        ("Shula", "Shula"),
        ("ശൂലം", "Shoolam"),
        ("சூலம்", "Soolam"),
        ("शूल", "Shula"),
        ("శూలం", "Shoolam"),
        ("ಶೂಲ", "Shoola"),
    ),
    r(
        "Ganda",
        ("Ganda", "Ganda"),
        ("ഗണ്ഡം", "Gandam"),
        ("கண்டம்", "Gandam"),
        ("गण्ड", "Ganda"),
        ("గండం", "Gandam"),
        ("ಗಂಡ", "Ganda"),
    ),
    r(
        "Vriddhi",
        ("Vriddhi", "Vriddhi"),
        ("വൃദ്ധി", "Vriddhi"),
        ("விருத்தி", "Viruthi"),
        ("वृद्धि", "Vriddhi"),
        ("వృద్ధి", "Vruddhi"),
        ("ವೃದ್ಧಿ", "Vruddhi"),
    ),
    r(
        "Dhruva",
        ("Dhruva", "Dhruva"),
        ("ധ്രുവം", "Dhruvam"),
        ("துருவம்", "Dhuruvam"),
        ("ध्रुव", "Dhruva"),
        ("ధ్రువం", "Dhruvam"),
        ("ಧ್ರುವ", "Dhruva"),
    ),
    r(
        "Vyaghata",
        ("Vyaghata", "Vyaghata"),
        ("വ്യാഘാതം", "Vyaghatham"),
        ("வியாகாதம்", "Viyagatham"),
        ("व्याघात", "Vyaghata"),
        ("వ్యాఘాతం", "Vyaghatham"),
        ("ವ್ಯಾಘಾತ", "Vyaghata"),
    ),
    r(
        "Harshana",
        ("Harshana", "Harshana"),
        ("ഹർഷണം", "Harshanam"),
        ("ஹர்ஷணம்", "Harshanam"),
        ("हर्षण", "Harshana"),
        ("హర్షణం", "Harshanam"),
        ("ಹರ್ಷಣ", "Harshana"),
    ),
    r(
        "Vajra",
        ("Vajra", "Vajra"),
        ("വജ്രം", "Vajram"),
        ("வஜ்ரம்", "Vajram"),
        ("वज्र", "Vajra"),
        ("వజ్రం", "Vajram"),
        ("ವಜ್ರ", "Vajra"),
    ),
    r(
        "Siddhi",
        ("Siddhi", "Siddhi"),
        ("സിദ്ധി", "Siddhi"),
        ("சித்தி", "Siddhi"),
        ("सिद्धि", "Siddhi"),
        ("సిద్ధి", "Siddhi"),
        ("ಸಿದ್ಧಿ", "Siddhi"),
    ),
    r(
        "Vyatipata",
        ("Vyatipata", "Vyatipata"),
        ("വ്യതീപാതം", "Vyatheepatham"),
        ("வியதீபாதம்", "Viyatheepatham"),
        ("व्यतीपात", "Vyatipata"),
        ("వ్యతీపాతం", "Vyatheepatham"),
        ("ವ್ಯತೀಪಾತ", "Vyatipata"),
    ),
    r(
        "Variyan",
        ("Variyan", "Variyan"),
        ("വരീയാൻ", "Vareeyan"),
        ("வரியான்", "Variyan"),
        ("वरीयान", "Variyan"),
        ("వరీయాన్", "Vareeyan"),
        ("ವರೀಯಾನ್", "Variyan"),
    ),
    r(
        "Parigha",
        ("Parigha", "Parigha"),
        ("പരിഘം", "Parigham"),
        ("பரிகம்", "Parigam"),
        ("परिघ", "Parigha"),
        ("పరిఘం", "Parigham"),
        ("ಪರಿಘ", "Parigha"),
    ),
    r(
        "Shiva",
        ("Shiva", "Shiva"),
        ("ശിവം", "Shivam"),
        ("சிவம்", "Sivam"),
        ("शिव", "Shiva"),
        ("శివం", "Shivam"),
        ("ಶಿವ", "Shiva"),
    ),
    r(
        "Siddha",
        ("Siddha", "Siddha"),
        ("സിദ്ധം", "Siddham"),
        ("சித்தம்", "Siddham"),
        ("सिद्ध", "Siddha"),
        ("సిద్ధం", "Siddham"),
        ("ಸಿದ್ಧ", "Siddha"),
    ),
    r(
        "Sadhya",
        ("Sadhya", "Sadhya"),
        ("സാധ്യം", "Sadhyam"),
        ("சாத்தியம்", "Sadhyam"),
        ("साध्य", "Sadhya"),
        ("సాధ్యం", "Sadhyam"),
        ("ಸಾಧ್ಯ", "Sadhya"),
    ),
    r(
        "Shubha",
        ("Shubha", "Shubha"),
        ("ശുഭം", "Shubham"),
        ("சுபம்", "Subam"),
        ("शुभ", "Shubha"),
        ("శుభం", "Shubham"),
        ("ಶುಭ", "Shubha"),
    ),
    r(
        "Shukla",
        ("Shukla", "Shukla"),
        ("ശുക്ലം", "Shuklam"),
        ("சுக்லம்", "Suklam"),
        ("शुक्ल", "Shukla"),
        ("శుక్లం", "Shuklam"),
        ("ಶುಕ್ಲ", "Shukla"),
    ),
    r(
        "Brahma",
        ("Brahma", "Brahma"),
        ("ബ്രഹ്മം", "Brahmam"),
        ("பிரம்மம்", "Brahmam"),
        ("ब्रह्म", "Brahma"),
        ("బ్రహ్మం", "Brahmam"),
        ("ಬ್ರಹ್ಮ", "Brahma"),
    ),
    r(
        "Indra",
        ("Indra", "Aindra"),
        ("ഐന്ദ്രം", "Aindram"),
        ("இந்திரம்", "Indiram"),
        ("इन्द्र", "Indra"),
        ("ఐంద్రం", "Aindram"),
        ("ಐಂದ್ರ", "Aindra"),
    ),
    r(
        "Vaidhriti",
        ("Vaidhriti", "Vaidhriti"),
        ("വൈധൃതി", "Vaidhrithi"),
        ("வைதிருதி", "Vaidhiruthi"),
        ("वैधृति", "Vaidhriti"),
        ("వైధృతి", "Vaidhruthi"),
        ("ವೈಧೃತಿ", "Vaidhruti"),
    ),
];

/// The 11 karanas, keyed as `kshana_vedic::panchanga::karana` names them.
///
/// Sources: Hindi — hi.wikipedia.org/wiki/पञ्चाङ्गम्, hindi.webdunia.com (type-of-karan-astrology);
/// Malayalam — ml.wikipedia.org/wiki/കരണം_(ജ്യോതിഷം); Tamil — ta.wikipedia.org/wiki/கரணம்_(சோதிடம்),
/// svat21.in/2023/12/karanam-in-tamil-astrology.html; Telugu — te.wikipedia.org/wiki/కరణం,
/// teluguone.com/devotional (కరణములు); Kannada — kn.wikipedia.org/wiki/ಪಂಚಾಂಗ (ಬವ, ಬಾಲವ, ಕೌಲವ, ತೈತಲೆ, ಗರಜೆ,
/// ವಣಿಕ್, ಭದ್ರೆ, ಶಕುನಿ, ಚತುಷ್ಪಾತ್, ನಾಗವಾನ್, ಕಿಂಸ್ತುಘ್ನ).
static KARANA: [Row; 11] = [
    r(
        "Bava",
        ("Bava", "Bava"),
        ("ബവം", "Bavam"),
        ("பவம்", "Bavam"),
        ("बव", "Bava"),
        ("బవ", "Bava"),
        ("ಬವ", "Bava"),
    ),
    r(
        "Balava",
        ("Balava", "Balava"),
        ("ബാലവം", "Balavam"),
        ("பாலவம்", "Balavam"),
        ("बालव", "Balava"),
        ("బాలవ", "Balava"),
        ("ಬಾಲವ", "Balava"),
    ),
    r(
        "Kaulava",
        ("Kaulava", "Kaulava"),
        ("കൗലവം", "Kaulavam"),
        ("கௌலவம்", "Kaulavam"),
        ("कौलव", "Kaulava"),
        ("కౌలవ", "Kaulava"),
        ("ಕೌಲವ", "Kaulava"),
    ),
    r(
        "Taitila",
        ("Taitila", "Taitila"),
        ("തൈതിലം", "Thaithilam"),
        ("தைதுலம்", "Thaithulam"),
        ("तैतिल", "Taitila"),
        ("తైతుల", "Thaithula"),
        ("ತೈತಲೆ", "Taitale"),
    ),
    r(
        "Gara",
        ("Gara", "Garaja"),
        ("ഗരജം", "Garajam"),
        ("கரசை", "Garasai"),
        ("गर", "Gara"),
        ("గరజి", "Garaji"),
        ("ಗರಜೆ", "Garaje"),
    ),
    r(
        "Vanija",
        ("Vanija", "Vanija"),
        ("വണിജം", "Vanijam"),
        ("வணிசை", "Vanisai"),
        ("वणिज", "Vanija"),
        ("వణిజ", "Vanija"),
        ("ವಣಿಕ್", "Vanik"),
    ),
    r(
        "Vishti",
        ("Vishti", "Bhadra"),
        ("വിഷ്ടി", "Vishti"),
        ("பத்திரை", "Bhadrai"),
        ("विष्टि", "Vishti"),
        ("భద్ర", "Bhadra"),
        ("ಭದ್ರೆ", "Bhadre"),
    ),
    r(
        "Shakuni",
        ("Shakuni", "Shakuni"),
        ("ശകുനി", "Shakuni"),
        ("சகுனி", "Sakuni"),
        ("शकुनि", "Shakuni"),
        ("శకుని", "Sakuni"),
        ("ಶಕುನಿ", "Shakuni"),
    ),
    r(
        "Chatushpada",
        ("Chatushpada", "Chatushpada"),
        ("ചതുഷ്പാദം", "Chathushpadam"),
        ("சதுஷ்பாதம்", "Chathushpadam"),
        ("चतुष्पाद", "Chatushpada"),
        ("చతుష్పాత్", "Chatushpath"),
        ("ಚತುಷ್ಪಾತ್", "Chatushpat"),
    ),
    r(
        "Naga",
        ("Naga", "Naga"),
        ("നാഗം", "Nagam"),
        ("நாகவம்", "Nagavam"),
        ("नाग", "Naga"),
        ("నాగవం", "Nagavam"),
        ("ನಾಗವಾನ್", "Nagavan"),
    ),
    r(
        "Kimstughna",
        ("Kimstughna", "Kimstughna"),
        ("കിംസ്തുഘ്നം", "Kimsthughnam"),
        ("கிம்ஸ்துக்னம்", "Kimsthugnam"),
        ("किंस्तुघ्न", "Kimstughna"),
        ("కింస్తుఘ్నం", "Kimsthughnam"),
        ("ಕಿಂಸ್ತುಘ್ನ", "Kimstughna"),
    ),
];

/// The 9 grahas, keyed by the engine's planet names (`"Sun"`, `"Rahu"` …).
///
/// Sources: Malayalam — ml.wikipedia.org/wiki/നവഗ്രഹങ്ങൾ, janmabhumi.in/2021/07/22/3006958/samskriti/astrology-32;
/// Tamil — astrology.tamilcube.com/tamil-panchangam (hora order புதன், சந்திரன், சனி, குரு, செவ்வாய்),
/// blog.bhairavapeedam.org (ராகு காலம்), the Tamil weekday sources (செவ்வாய், புதன், வெள்ளி, சனி);
/// Hindi — drikpanchang.com/vedic-mantra/gods/navagraha/navagraha-mantras.html;
/// Telugu — te.wiktionary.org/wiki/నవగ్రహాలు(జ్యోతిషం); Kannada — vvkrishnamurthy01.wordpress.com (ನವಗ್ರಹಗಳು).
static GRAHA: [Row; 9] = [
    r(
        "Sun",
        ("Sun", "Surya"),
        ("സൂര്യൻ", "Sooryan"),
        ("சூரியன்", "Sooriyan"),
        ("सूर्य", "Surya"),
        ("సూర్యుడు", "Sooryudu"),
        ("ಸೂರ್ಯ", "Surya"),
    ),
    r(
        "Moon",
        ("Moon", "Chandra"),
        ("ചന്ദ്രൻ", "Chandran"),
        ("சந்திரன்", "Chandiran"),
        ("चंद्र", "Chandra"),
        ("చంద్రుడు", "Chandrudu"),
        ("ಚಂದ್ರ", "Chandra"),
    ),
    r(
        "Mars",
        ("Mars", "Mangala / Kuja"),
        ("ചൊവ്വ", "Chovva"),
        ("செவ்வாய்", "Sevvai"),
        ("मंगल", "Mangal"),
        ("కుజుడు", "Kujudu"),
        ("ಮಂಗಳ", "Mangala"),
    ),
    r(
        "Mercury",
        ("Mercury", "Budha"),
        ("ബുധൻ", "Budhan"),
        ("புதன்", "Budhan"),
        ("बुध", "Budh"),
        ("బుధుడు", "Budhudu"),
        ("ಬುಧ", "Budha"),
    ),
    r(
        "Jupiter",
        ("Jupiter", "Guru / Brihaspati"),
        ("വ്യാഴം", "Vyazham"),
        ("குரு", "Guru"),
        ("गुरु", "Guru"),
        ("గురుడు", "Gurudu"),
        ("ಗುರು", "Guru"),
    ),
    r(
        "Venus",
        ("Venus", "Shukra"),
        ("ശുക്രൻ", "Shukran"),
        ("சுக்கிரன்", "Sukkiran"),
        ("शुक्र", "Shukra"),
        ("శుక్రుడు", "Shukrudu"),
        ("ಶುಕ್ರ", "Shukra"),
    ),
    r(
        "Saturn",
        ("Saturn", "Shani"),
        ("ശനി", "Shani"),
        ("சனி", "Sani"),
        ("शनि", "Shani"),
        ("శని", "Shani"),
        ("ಶನಿ", "Shani"),
    ),
    r(
        "Rahu",
        ("Rahu", "North node"),
        ("രാഹു", "Rahu"),
        ("ராகு", "Ragu"),
        ("राहु", "Rahu"),
        ("రాహువు", "Rahuvu"),
        ("ರಾಹು", "Rahu"),
    ),
    r(
        "Ketu",
        ("Ketu", "South node"),
        ("കേതു", "Kethu"),
        ("கேது", "Kethu"),
        ("केतु", "Ketu"),
        ("కేతువు", "Kethuvu"),
        ("ಕೇತು", "Ketu"),
    ),
];

/// The 12 bhavas, keyed by their Sanskrit names (alias `"House 1"` … `"House 12"`).
///
/// Sources: Sanskrit/Hindi — en.wikipedia.org/wiki/Bhāva (Tanu, Dhana, Sahaja, Sukha/Bandhu, Putra, Ari,
/// Yuvati, Randhra, Dharma, Karma, Labha, Vyaya), kundligpt.com/hi/houses; Malayalam — the Kerala usage
/// (hindustanjyothisham.blogspot.com "ലഗ്നം മുതൽ പന്ത്രണ്ട് വരെയുള്ള ഭാവങ്ങൾ", manoramaonline "വ്യയ സ്ഥാനം"):
/// the sources name the houses in prose rather than as one table, so this column is marked partially
/// verified; Tamil — jothidamkarka.blogspot.com, way2astro.in (12 பாவங்கள்), likewise prose; Telugu — only
/// the cells named by acchamgatelugu.com/2021/01/astrology-lessons-5.html and nsteluguastrology.com
/// (తను, ధన, కళత్ర, ఆయుష్షు, వ్యయ) are filled; Kannada — no list found, left empty.
static BHAVA: [Row; 12] = [
    r(
        "Tanu",
        ("1st house", "Tanu, Lagna"),
        ("തനു", "Thanu"),
        ("தனு", "Thanu"),
        ("तनु", "Tanu"),
        ("తను", "Tanu"),
        ("", ""),
    ),
    r(
        "Dhana",
        ("2nd house", "Dhana"),
        ("ധനം", "Dhanam"),
        ("தனம்", "Dhanam"),
        ("धन", "Dhana"),
        ("ధన", "Dhana"),
        ("", ""),
    ),
    r(
        "Sahaja",
        ("3rd house", "Sahaja"),
        ("സഹജം", "Sahajam"),
        ("சகஜம்", "Sagajam"),
        ("सहज", "Sahaja"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Bandhu",
        ("4th house", "Bandhu, Sukha"),
        ("ബന്ധു", "Bandhu"),
        ("சுகம்", "Sugam"),
        ("बन्धु", "Bandhu"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Putra",
        ("5th house", "Putra"),
        ("പുത്രം", "Puthram"),
        ("புத்திரம்", "Puthiram"),
        ("पुत्र", "Putra"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Ari",
        ("6th house", "Ari, Shatru"),
        ("ശത്രു", "Shathru"),
        ("சத்ரு", "Sathru"),
        ("अरि", "Ari"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Yuvati",
        ("7th house", "Yuvati, Kalatra"),
        ("കളത്രം", "Kalathram"),
        ("களத்திரம்", "Kalathiram"),
        ("युवति", "Yuvati"),
        ("కళత్ర", "Kalatra"),
        ("", ""),
    ),
    r(
        "Randhra",
        ("8th house", "Randhra, Ayu"),
        ("ആയുസ്സ്", "Ayussu"),
        ("ஆயுள்", "Ayul"),
        ("रन्ध्र", "Randhra"),
        ("ఆయుష్షు", "Ayushu"),
        ("", ""),
    ),
    r(
        "Dharma",
        ("9th house", "Dharma, Bhagya"),
        ("ഭാഗ്യം", "Bhagyam"),
        ("பாக்கியம்", "Bhagyam"),
        ("धर्म", "Dharma"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Karma",
        ("10th house", "Karma"),
        ("കർമ്മം", "Karmam"),
        ("கர்மம்", "Karmam"),
        ("कर्म", "Karma"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Labha",
        ("11th house", "Labha"),
        ("ലാഭം", "Labham"),
        ("லாபம்", "Labam"),
        ("लाभ", "Labha"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Vyaya",
        ("12th house", "Vyaya"),
        ("വ്യയം", "Vyayam"),
        ("விரயம்", "Virayam"),
        ("व्यय", "Vyaya"),
        ("వ్యయ", "Vyaya"),
        ("", ""),
    ),
];

/// The Vimshottari levels (the five the engine solves) and Sade Sati.
///
/// Sources: Hindi — healthwealthvaastu.com, parasharajyotish.com (महादशा, अंतर्दशा, प्रत्यंतर दशा, सूक्ष्म दशा,
/// प्राण दशा); Malayalam — rgkrishnanchry.blogspot.com/2015/11 (ദശാപഹാരങ്ങൾ), ml.wikisource.org
/// ജ്യോതിഷവും ജ്യോതിശ്ശാസ്ത്രവും/അധ്യായം 6 (ഛിദ്രം): Kerala calls the sub-period apaharam and the third level
/// chhidram; the fourth and fifth levels were not found in a Malayalam source and are left empty;
/// Tamil — astrotamil.in/blog/dasa-bukthi-vilakkam, madhanastronumerology.pro ("9 புக்தி, 81 அந்தரம்,
/// 729 சூட்சுமம்"); Telugu — te.wikipedia.org/wiki/జ్యోతిషం, vidhata.app/te (మహాదశ, అంతర్దశ, ప్రత్యంతర్దశ);
/// Kannada — zeenews.india.com/kannada ("ಗುರು ಮಹಾದಶಾ"); the finer Kannada levels were not found.
/// Sade Sati — onlinejyotish.com/sadesathi (Ezharashani), madhanastronumerology.pro (ஏழரை சனி),
/// onlinejyotish.com/astrology-lessons/lesson_103.php (ఏలినాటి శని), eesanje.com (ಸಾಡೇಸಾತಿ),
/// shanibhagwan.com/sadesati.html (साढ़े साती).
static PERIOD: [Row; 6] = [
    r(
        "Mahadasha",
        ("Mahadasha", "Major period"),
        ("മഹാദശ", "Mahadasha"),
        ("மகாதசை", "Mahadasai"),
        ("महादशा", "Mahadasha"),
        ("మహాదశ", "Mahadasha"),
        ("ಮಹಾದಶಾ", "Mahadasha"),
    ),
    r(
        "Antardasha",
        ("Antardasha", "Sub-period, bhukti"),
        ("അപഹാരം", "Apaharam"),
        ("புக்தி", "Bhukthi"),
        ("अंतर्दशा", "Antardasha"),
        ("అంతర్దశ", "Antardasha"),
        ("", ""),
    ),
    r(
        "Pratyantardasha",
        ("Pratyantardasha", "Sub-sub-period"),
        ("ഛിദ്രം", "Chhidram"),
        ("அந்தரம்", "Antharam"),
        ("प्रत्यंतर्दशा", "Pratyantardasha"),
        ("ప్రత్యంతర్దశ", "Pratyantardasha"),
        ("", ""),
    ),
    r(
        "Sookshma dasha",
        ("Sookshma dasha", "Fourth level"),
        ("", ""),
        ("சூட்சுமம்", "Sootchumam"),
        ("सूक्ष्म दशा", "Sookshma dasha"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Prana dasha",
        ("Prana dasha", "Fifth level"),
        ("", ""),
        ("", ""),
        ("प्राण दशा", "Prana dasha"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Sade Sati",
        ("Sade Sati", "Saturn's seven and a half years"),
        ("ഏഴരശനി", "Ezhara Shani"),
        ("ஏழரை சனி", "Ezharai Sani"),
        ("साढ़े साती", "Sadhe Sati"),
        ("ఏలినాటి శని", "Elinati Shani"),
        ("ಸಾಡೇಸಾತಿ", "Sadesati"),
    ),
];

/// The day clocks: muhurta, choghadiya, hora, ghati/vighati, nazhika/vinazhika, Rahu kala,
/// Gulika, Yamaganda, Abhijit and Brahma muhurta.
///
/// Sources: Malayalam — malayalam.boldsky.com (രാഹുകാലം, യമകണ്ടം, ഗുളികകാലം, അഭിജിത്ത് മുഹൂർത്തം),
/// ml.wikisource.org ജ്യോതിഷവും ജ്യോതിശ്ശാസ്ത്രവും/അധ്യായം 1 (ഹോര), janamtv.com (ബ്രഹ്മമുഹൂർത്തം), the
/// Kerala almanac count "12 nazhika 58 vinazhika from sunrise" (olamatch.com/en/calendar); Tamil —
/// ta.wikipedia.org/wiki/நாழிகை (1 நாழிகை = 24 நிமிடங்கள் = 60 விநாழிகை, 2.5 நாழிகை = 1 ஓரை),
/// blog.bhairavapeedam.org (ராகு காலம், எமகண்டம், குளிகை), tamilmuhurtham.in (அபிஜித் முகூர்த்தம்),
/// isha.sadhguru.org/ta (பிரம்ம முகூர்த்தம்); Hindi — amarujala.com daily panchang (राहुकाल, गुलिक काल,
/// यमगंड, अभिजीत मुहूर्त, चौघड़िया), kundligpt.com/hi/rahu-kaal (ब्रह्म मुहूर्त); Telugu — te.wiktionary.org/wiki/ఘడియ,
/// te.wikipedia.org/wiki/కాలం (విఘడియ, హోర), dishadaily.com (రాహుకాలం, యమగండం, గుళిక, అభిజిత్, బ్రహ్మ ముహూర్తం);
/// Kannada — shivamoggalive.com, vijayavani.net (ರಾಹುಕಾಲ, ಗುಳಿಕ ಕಾಲ, ಯಮಗಂಡ), drikpanchang.com/kannada
/// (ಅಭಿಜಿತ್, ಮುಹೂರ್ತ), tv9kannada.com (ಬ್ರಹ್ಮ ಮುಹೂರ್ತ), astrosage.com/kannada (ಶನಿ ಹೋರಾ). Choghadiya is a
/// North Indian clock and has no Southern form; ghati/vighati outside Telugu and Hindi were not found.
static CLOCK: [Row; 12] = [
    r(
        "Muhurta",
        ("Muhurta", "48 minutes, 1/30 of a day"),
        ("മുഹൂർത്തം", "Muhoortham"),
        ("முகூர்த்தம்", "Muhurtham"),
        ("मुहूर्त", "Muhurt"),
        ("ముహూర్తం", "Muhurtam"),
        ("ಮುಹೂರ್ತ", "Muhurta"),
    ),
    r(
        "Choghadiya",
        ("Choghadiya", "1/8 of the day or night"),
        ("", ""),
        ("", ""),
        ("चौघड़िया", "Chaughadiya"),
        ("", ""),
        ("", ""),
    ),
    r(
        "Hora",
        ("Hora", "Planetary hour, 1/12 of day or night"),
        ("ഹോര", "Hora"),
        ("ஓரை", "Orai"),
        ("", ""),
        ("హోర", "Hora"),
        ("ಹೋರಾ", "Hora"),
    ),
    r(
        "Ghati",
        ("Ghati", "24 minutes, 1/60 of a day"),
        ("നാഴിക", "Nazhika"),
        ("நாழிகை", "Nazhigai"),
        ("", ""),
        ("ఘడియ", "Ghadiya"),
        ("", ""),
    ),
    r(
        "Vighati",
        ("Vighati", "24 seconds, 1/60 ghati"),
        ("വിനാഴിക", "Vinazhika"),
        ("விநாழிகை", "Vinazhigai"),
        ("", ""),
        ("విఘడియ", "Vighadiya"),
        ("", ""),
    ),
    r(
        "Nazhika",
        ("Nazhika", "24 minutes counted from sunrise"),
        ("നാഴിക", "Nazhika"),
        ("நாழிகை", "Nazhigai"),
        ("", ""),
        ("ఘడియ", "Ghadiya"),
        ("", ""),
    ),
    r(
        "Vinazhika",
        ("Vinazhika", "24 seconds, 1/60 nazhika"),
        ("വിനാഴിക", "Vinazhika"),
        ("விநாழிகை", "Vinazhigai"),
        ("", ""),
        ("విఘడియ", "Vighadiya"),
        ("", ""),
    ),
    r(
        "Rahu kala",
        ("Rahu kala", "Rahu's eighth of the day"),
        ("രാഹുകാലം", "Rahukalam"),
        ("ராகு காலம்", "Ragu kalam"),
        ("राहुकाल", "Rahukal"),
        ("రాహుకాలం", "Rahukalam"),
        ("ರಾಹುಕಾಲ", "Rahukala"),
    ),
    r(
        "Gulika",
        ("Gulika", "Gulika kala, Saturn's son"),
        ("ഗുളികകാലം", "Gulikakalam"),
        ("குளிகை", "Guligai"),
        ("गुलिक काल", "Gulik kal"),
        ("గుళిక కాలం", "Gulika kalam"),
        ("ಗುಳಿಕ ಕಾಲ", "Gulika kala"),
    ),
    r(
        "Yamaganda",
        ("Yamaganda", "Yama's eighth of the day"),
        ("യമകണ്ടം", "Yamakandam"),
        ("எமகண்டம்", "Emagandam"),
        ("यमगंड", "Yamgand"),
        ("యమగండం", "Yamagandam"),
        ("ಯಮಗಂಡ", "Yamaganda"),
    ),
    r(
        "Abhijit",
        ("Abhijit", "Eighth muhurta, around midday"),
        ("അഭിജിത് മുഹൂർത്തം", "Abhijit muhoortham"),
        ("அபிஜித் முகூர்த்தம்", "Abhijit muhurtham"),
        ("अभिजीत मुहूर्त", "Abhijit muhurt"),
        ("అభిజిత్ ముహూర్తం", "Abhijit muhurtam"),
        ("ಅಭಿಜಿತ್ ಮುಹೂರ್ತ", "Abhijit muhurta"),
    ),
    r(
        "Brahma muhurta",
        ("Brahma muhurta", "Two muhurtas before sunrise"),
        ("ബ്രഹ്മമുഹൂർത്തം", "Brahmamuhoortham"),
        ("பிரம்ம முகூர்த்தம்", "Brahma muhurtham"),
        ("ब्रह्म मुहूर्त", "Brahma muhurt"),
        ("బ్రహ్మ ముహూర్తం", "Brahma muhurtam"),
        ("ಬ್ರಹ್ಮ ಮುಹೂರ್ತ", "Brahma muhurta"),
    ),
];

/// The observances tied to a tithi or an ingress (Purnima and Amavasya live in the tithi table).
///
/// Sources: Ekadashi — the tithi tables; Pradosha — janamtv.com/80803798 (പ്രദോഷം), poojalu.com
/// (ప్రదోషం), drikpanchang.com/kannada ("ಶುಕ್ರ ಪ್ರದೋಷ ವ್ರತ"); Sankranti — manoramaonline.com (മകരസംക്രമം /
/// സംക്രാന്തി). Tamil and Hindi forms of Pradosha and Sankranti were not matched to a source and are empty.
static OBSERVANCE: [Row; 3] = [
    r(
        "Ekadashi",
        ("Ekadashi", "Eleventh tithi, a fast day"),
        ("ഏകാദശി", "Ekadashi"),
        ("ஏகாதசி", "Ekadasi"),
        ("एकादशी", "Ekadashi"),
        ("ఏకాదశి", "Ekadasi"),
        ("ಏಕಾದಶಿ", "Ekadashi"),
    ),
    r(
        "Pradosha",
        ("Pradosha", "Dusk of the thirteenth tithi"),
        ("പ്രദോഷം", "Pradosham"),
        ("", ""),
        ("", ""),
        ("ప్రదోషం", "Pradosham"),
        ("ಪ್ರದೋಷ", "Pradosha"),
    ),
    r(
        "Sankranti",
        ("Sankranti", "Sun entering a sidereal sign"),
        ("സംക്രാന്തി", "Sankranthi"),
        ("", ""),
        ("", ""),
        ("", ""),
        ("", ""),
    ),
];

/// The 12 Kollavarsham months, Chingam first; each is the Sun's stay in one sidereal sign.
/// Other languages give their own name for the same solar month: Tamil the Tamil month, Kannada the
/// rasi name (Kannada almanacs use rasi names for solar months); Hindi and Telugu have none and are empty.
///
/// Sources: en.wikipedia.org/wiki/Malayalam_calendar ("Chingam aligns with Simha rashi"), ml.wikipedia.org
/// രാശിചക്രം and manoramaonline (the month names are the rasi names), whiteboardweb.blogspot.com
/// (മലയാള മാസങ്ങൾ); Tamil mapping — ta.wikipedia.org/wiki/தமிழ்_மாதங்கள், tamilcalendar.com/articles/tamil-months-explained;
/// Kannada — kn.wikipedia.org/wiki/ರಾಶಿ ("the names double as solar-calendar month names").
static MONTH_KOLLA: [Row; 12] = [
    r(
        "Chingam",
        ("Chingam", "Leo · from mid-August"),
        ("ചിങ്ങം", "Chingam"),
        ("ஆவணி", "Aavani"),
        ("", ""),
        ("", ""),
        ("ಸಿಂಹ", "Simha"),
    ),
    r(
        "Kanni",
        ("Kanni", "Virgo · from mid-September"),
        ("കന്നി", "Kanni"),
        ("புரட்டாசி", "Purattasi"),
        ("", ""),
        ("", ""),
        ("ಕನ್ಯಾ", "Kanya"),
    ),
    r(
        "Thulam",
        ("Thulam", "Libra · from mid-October"),
        ("തുലാം", "Thulam"),
        ("ஐப்பசி", "Aippasi"),
        ("", ""),
        ("", ""),
        ("ತುಲಾ", "Tula"),
    ),
    r(
        "Vrischikam",
        ("Vrischikam", "Scorpio · from mid-November"),
        ("വൃശ്ചികം", "Vrischikam"),
        ("கார்த்திகை", "Karthigai"),
        ("", ""),
        ("", ""),
        ("ವೃಶ್ಚಿಕ", "Vrushchika"),
    ),
    r(
        "Dhanu",
        ("Dhanu", "Sagittarius · from mid-December"),
        ("ധനു", "Dhanu"),
        ("மார்கழி", "Margazhi"),
        ("", ""),
        ("", ""),
        ("ಧನು", "Dhanu"),
    ),
    r(
        "Makaram",
        ("Makaram", "Capricorn · from mid-January"),
        ("മകരം", "Makaram"),
        ("தை", "Thai"),
        ("", ""),
        ("", ""),
        ("ಮಕರ", "Makara"),
    ),
    r(
        "Kumbham",
        ("Kumbham", "Aquarius · from mid-February"),
        ("കുംഭം", "Kumbham"),
        ("மாசி", "Maasi"),
        ("", ""),
        ("", ""),
        ("ಕುಂಭ", "Kumbha"),
    ),
    r(
        "Meenam",
        ("Meenam", "Pisces · from mid-March"),
        ("മീനം", "Meenam"),
        ("பங்குனி", "Panguni"),
        ("", ""),
        ("", ""),
        ("ಮೀನ", "Meena"),
    ),
    r(
        "Medam",
        ("Medam", "Aries · from mid-April"),
        ("മേടം", "Medam"),
        ("சித்திரை", "Chithirai"),
        ("", ""),
        ("", ""),
        ("ಮೇಷ", "Mesha"),
    ),
    r(
        "Edavam",
        ("Edavam", "Taurus · from mid-May"),
        ("ഇടവം", "Edavam"),
        ("வைகாசி", "Vaikasi"),
        ("", ""),
        ("", ""),
        ("ವೃಷಭ", "Vrushabha"),
    ),
    r(
        "Mithunam",
        ("Mithunam", "Gemini · from mid-June"),
        ("മിഥുനം", "Mithunam"),
        ("ஆனி", "Aani"),
        ("", ""),
        ("", ""),
        ("ಮಿಥುನ", "Mithuna"),
    ),
    r(
        "Karkidakam",
        ("Karkidakam", "Cancer · from mid-July"),
        ("കർക്കടകം", "Karkidakam"),
        ("ஆடி", "Aadi"),
        ("", ""),
        ("", ""),
        ("ಕಟಕ", "Kataka"),
    ),
];

/// The 12 Tamil solar months, Chithirai (Mesha) first. Malayalam gives the Kollavarsham month of the
/// same sign, Kannada the rasi name; Hindi and Telugu are empty.
///
/// Sources: ta.wikipedia.org/wiki/தமிழ்_மாதங்கள், tamilcalendar.com/articles/tamil-months-explained
/// (Chithirai–Mesham, Vaikasi–Rishabam, Aani–Mithunam, Aavani–Simmam, Thai–Makaram, Maasi–Kumbam,
/// Panguni–Meenam; the rest follow the sign order), en.wikipedia.org/wiki/Mesha_(month).
static MONTH_TAMIL: [Row; 12] = [
    r(
        "Chithirai",
        ("Chithirai", "Aries · from mid-April"),
        ("മേടം", "Medam"),
        ("சித்திரை", "Chithirai"),
        ("", ""),
        ("", ""),
        ("ಮೇಷ", "Mesha"),
    ),
    r(
        "Vaikasi",
        ("Vaikasi", "Taurus · from mid-May"),
        ("ഇടവം", "Edavam"),
        ("வைகாசி", "Vaikasi"),
        ("", ""),
        ("", ""),
        ("ವೃಷಭ", "Vrushabha"),
    ),
    r(
        "Aani",
        ("Aani", "Gemini · from mid-June"),
        ("മിഥുനം", "Mithunam"),
        ("ஆனி", "Aani"),
        ("", ""),
        ("", ""),
        ("ಮಿಥುನ", "Mithuna"),
    ),
    r(
        "Aadi",
        ("Aadi", "Cancer · from mid-July"),
        ("കർക്കടകം", "Karkidakam"),
        ("ஆடி", "Aadi"),
        ("", ""),
        ("", ""),
        ("ಕಟಕ", "Kataka"),
    ),
    r(
        "Aavani",
        ("Aavani", "Leo · from mid-August"),
        ("ചിങ്ങം", "Chingam"),
        ("ஆவணி", "Aavani"),
        ("", ""),
        ("", ""),
        ("ಸಿಂಹ", "Simha"),
    ),
    r(
        "Purattasi",
        ("Purattasi", "Virgo · from mid-September"),
        ("കന്നി", "Kanni"),
        ("புரட்டாசி", "Purattasi"),
        ("", ""),
        ("", ""),
        ("ಕನ್ಯಾ", "Kanya"),
    ),
    r(
        "Aippasi",
        ("Aippasi", "Libra · from mid-October"),
        ("തുലാം", "Thulam"),
        ("ஐப்பசி", "Aippasi"),
        ("", ""),
        ("", ""),
        ("ತುಲಾ", "Tula"),
    ),
    r(
        "Karthigai",
        ("Karthigai", "Scorpio · from mid-November"),
        ("വൃശ്ചികം", "Vrischikam"),
        ("கார்த்திகை", "Karthigai"),
        ("", ""),
        ("", ""),
        ("ವೃಶ್ಚಿಕ", "Vrushchika"),
    ),
    r(
        "Margazhi",
        ("Margazhi", "Sagittarius · from mid-December"),
        ("ധനു", "Dhanu"),
        ("மார்கழி", "Margazhi"),
        ("", ""),
        ("", ""),
        ("ಧನು", "Dhanu"),
    ),
    r(
        "Thai",
        ("Thai", "Capricorn · from mid-January"),
        ("മകരം", "Makaram"),
        ("தை", "Thai"),
        ("", ""),
        ("", ""),
        ("ಮಕರ", "Makara"),
    ),
    r(
        "Maasi",
        ("Maasi", "Aquarius · from mid-February"),
        ("കുംഭം", "Kumbham"),
        ("மாசி", "Maasi"),
        ("", ""),
        ("", ""),
        ("ಕುಂಭ", "Kumbha"),
    ),
    r(
        "Panguni",
        ("Panguni", "Pisces · from mid-March"),
        ("മീനം", "Meenam"),
        ("பங்குனி", "Panguni"),
        ("", ""),
        ("", ""),
        ("ಮೀನ", "Meena"),
    ),
];

/// The 12 Saka/Vikrama lunar months, Chaitra first. Tamil almanacs use the solar months and have no
/// separate list, so that column is empty. Jyeshtha, Shravana and Magha are also nakshatra names, so
/// those month keys are `"Jyeshtha (month)"`, `"Shravana (month)"` and `"Magha (month)"`; the bare
/// names resolve to the nakshatras.
///
/// Sources: Hindi — monthnameshindi.com, religionworld.in (चैत्र … फाल्गुन); Malayalam — ml.wikipedia.org/wiki/ശകവർഷം
/// and ml.wikipedia.org/wiki/മാസം (ചൈത്രം … ഫാൽഗുനം; the pages spell the sixth ഭാദ്രം/ഭാദ്രപാദം and the seventh
/// അശ്വിനം/അശ്വിനി, the forms used here are the ones on the മാസം page); Telugu — rashiphalalu.in/telugu-months-names,
/// hariome.com/names-of-telugu-months; Kannada — kn.wikipedia.org/wiki/ಚಾಂದ್ರಮಾನ, kn.wiktionary.org/wiki/ಮಾಸಗಳು.
static MONTH_LUNAR: [Row; 12] = [
    r(
        "Chaitra",
        ("Chaitra", "March–April"),
        ("ചൈത്രം", "Chaithram"),
        ("", ""),
        ("चैत्र", "Chaitra"),
        ("చైత్రం", "Chaitram"),
        ("ಚೈತ್ರ", "Chaitra"),
    ),
    r(
        "Vaishakha",
        ("Vaishakha", "April–May"),
        ("വൈശാഖം", "Vaishakham"),
        ("", ""),
        ("वैशाख", "Vaishakh"),
        ("వైశాఖం", "Vaisakham"),
        ("ವೈಶಾಖ", "Vaishakha"),
    ),
    r(
        "Jyeshtha (month)",
        ("Jyeshtha (month)", "May–June"),
        ("ജ്യേഷ്ഠം", "Jyeshtham"),
        ("", ""),
        ("ज्येष्ठ", "Jyeshth"),
        ("జ్యేష్ఠం", "Jyeshtham"),
        ("ಜ್ಯೇಷ್ಠ", "Jyeshtha"),
    ),
    r(
        "Ashadha",
        ("Ashadha", "June–July"),
        ("ആഷാഢം", "Ashadham"),
        ("", ""),
        ("आषाढ़", "Ashadh"),
        ("ఆషాఢం", "Ashadham"),
        ("ಆಷಾಢ", "Ashadha"),
    ),
    r(
        "Shravana (month)",
        ("Shravana (month)", "July–August"),
        ("ശ്രാവണം", "Shravanam"),
        ("", ""),
        ("श्रावण", "Shravan"),
        ("శ్రావణం", "Sravanam"),
        ("ಶ್ರಾವಣ", "Shravana"),
    ),
    r(
        "Bhadrapada",
        ("Bhadrapada", "August–September"),
        ("ഭാദ്രപാദം", "Bhadrapadam"),
        ("", ""),
        ("भाद्रपद", "Bhadrapad"),
        ("భాద్రపదం", "Bhadrapadam"),
        ("ಭಾದ್ರಪದ", "Bhadrapada"),
    ),
    r(
        "Ashwina",
        ("Ashwina", "September–October"),
        ("അശ്വിനം", "Aswinam"),
        ("", ""),
        ("आश्विन", "Ashwin"),
        ("ఆశ్వయుజం", "Aswayujam"),
        ("ಆಶ್ವಯುಜ", "Ashwayuja"),
    ),
    r(
        "Kartika",
        ("Kartika", "October–November"),
        ("കാർത്തികം", "Karthikam"),
        ("", ""),
        ("कार्तिक", "Kartik"),
        ("కార్తీకం", "Karthikam"),
        ("ಕಾರ್ತಿಕ", "Kartika"),
    ),
    r(
        "Margashirsha",
        ("Margashirsha", "November–December"),
        ("മാർഗശീർഷം", "Margasheersham"),
        ("", ""),
        ("मार्गशीर्ष", "Margashirsh"),
        ("మార్గశిరం", "Margasiram"),
        ("ಮಾರ್ಗಶಿರ", "Margashira"),
    ),
    r(
        "Pausha",
        ("Pausha", "December–January"),
        ("പൗഷം", "Pausham"),
        ("", ""),
        ("पौष", "Paush"),
        ("పుష్యం", "Pushyam"),
        ("ಪುಷ್ಯ", "Pushya"),
    ),
    r(
        "Magha (month)",
        ("Magha (month)", "January–February"),
        ("മാഘം", "Magham"),
        ("", ""),
        ("माघ", "Magh"),
        ("మాఘం", "Magham"),
        ("ಮಾಘ", "Magha"),
    ),
    r(
        "Phalguna",
        ("Phalguna", "February–March"),
        ("ഫാൽഗുനം", "Phalgunam"),
        ("", ""),
        ("फाल्गुन", "Phalgun"),
        ("ఫాల్గుణం", "Phalgunam"),
        ("ಫಾಲ್ಗುಣ", "Phalguna"),
    ),
];

/// Other spellings the engine or a caller may use, mapped to the table key.
const ALIASES: [(&str, &str); 52] = [
    ("Mesha", "Aries"),
    ("Vrishabha", "Taurus"),
    ("Mithuna", "Gemini"),
    ("Karka", "Cancer"),
    ("Kataka", "Cancer"),
    ("Karkata", "Cancer"),
    ("Simha", "Leo"),
    ("Kanya", "Virgo"),
    ("Tula", "Libra"),
    ("Vrischika", "Scorpio"),
    ("Dhanus", "Sagittarius"),
    ("Makara", "Capricorn"),
    ("Kumbha", "Aquarius"),
    ("Meena", "Pisces"),
    ("Sunday", "Ravivara"),
    ("Monday", "Somavara"),
    ("Tuesday", "Mangalavara"),
    ("Wednesday", "Budhavara"),
    ("Thursday", "Guruvara"),
    ("Friday", "Shukravara"),
    ("Saturday", "Shanivara"),
    ("Surya", "Sun"),
    ("Chandra", "Moon"),
    ("Mangala", "Mars"),
    ("Kuja", "Mars"),
    ("Budha", "Mercury"),
    ("Guru", "Jupiter"),
    ("Brihaspati", "Jupiter"),
    ("Shukra", "Venus"),
    ("Shani", "Saturn"),
    ("Shukla Purnima/Amavasya", "Purnima"),
    ("Krishna Purnima/Amavasya", "Amavasya"),
    ("Shukla Purnima", "Purnima"),
    ("Krishna Amavasya", "Amavasya"),
    ("Purnima/Amavasya", "Purnima"),
    ("House 1", "Tanu"),
    ("House 2", "Dhana"),
    ("House 3", "Sahaja"),
    ("House 4", "Bandhu"),
    ("House 5", "Putra"),
    ("House 6", "Ari"),
    ("House 7", "Yuvati"),
    ("House 8", "Randhra"),
    ("House 9", "Dharma"),
    ("House 10", "Karma"),
    ("House 11", "Labha"),
    ("House 12", "Vyaya"),
    ("Lagna", "Tanu"),
    ("Bhukti", "Antardasha"),
    ("Apahara", "Antardasha"),
    ("Rahukala", "Rahu kala"),
    ("Gulika kala", "Gulika"),
];

/// Every table with its family name, in display order (the tithi family spans two tables).
fn tables() -> [(&'static str, &'static [Row]); 15] {
    [
        ("rasi", &RASI),
        ("nakshatra", &NAKSHATRA),
        ("tithi", &TITHI_SHUKLA),
        ("tithi", &TITHI_KRISHNA),
        ("vara", &VARA),
        ("yoga", &YOGA),
        ("karana", &KARANA),
        ("graha", &GRAHA),
        ("bhava", &BHAVA),
        ("period", &PERIOD),
        ("clock", &CLOCK),
        ("observance", &OBSERVANCE),
        ("month_kolla", &MONTH_KOLLA),
        ("month_tamil", &MONTH_TAMIL),
        ("month_lunar", &MONTH_LUNAR),
    ]
}

fn canonical(key: &str) -> &str {
    let key = key.trim();
    ALIASES
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(key))
        .map_or(key, |(_, k)| k)
}

fn find(key: &str) -> Option<&'static Row> {
    let key = canonical(key);
    tables()
        .iter()
        .flat_map(|(_, rows)| rows.iter())
        .find(|row| row.key.eq_ignore_ascii_case(key))
}

/// The term in `lang`, native script. Falls back to the English name when the cell is
/// unverified (empty) and to `""` when `key` is not in the lexicon, so a caller can show the
/// key it already holds. Keys are the engine's spellings or an alias (`"Mesha"`, `"Sunday"`,
/// `"House 7"`); case does not matter.
pub fn term(key: &str, lang: Language) -> &'static str {
    match find(key) {
        Some(row) => {
            let native = row.cells[lang.column()].0;
            if native.is_empty() {
                row.cells[0].0
            } else {
                native
            }
        }
        None => "",
    }
}

/// The Latin transliteration in `lang` (for English, the Sanskrit or alternate name), with
/// the same fallbacks as [`term`].
pub fn term_latin(key: &str, lang: Language) -> &'static str {
    match find(key) {
        Some(row) => {
            let latin = row.cells[lang.column()].1;
            if latin.is_empty() {
                row.cells[0].1
            } else {
                latin
            }
        }
        None => "",
    }
}

/// Every term in `lang`, family by family in [`FAMILIES`] order. Unverified cells come back
/// empty here (not substituted), so a screen can show them as missing.
pub fn lexicon(lang: Language) -> Vec<LexiconEntry> {
    let col = lang.column();
    tables()
        .iter()
        .flat_map(|(family, rows)| {
            rows.iter().map(move |row| LexiconEntry {
                family,
                key: row.key,
                english: row.cells[0].0,
                sanskrit: row.cells[0].1,
                native: row.cells[col].0,
                latin: row.cells[col].1,
            })
        })
        .collect()
}

/// Nanoseconds in one nazhika (ghati): 24 minutes.
const NAZHIKA_NS: i128 = 24 * 60 * NS_PER_SEC;
/// Nanoseconds in one vinazhika (vighati): 24 seconds.
const VINAZHIKA_NS: i128 = NAZHIKA_NS / 60;

/// Udayadi nazhika: the time since sunrise in the Kerala almanac's units. A day holds 60
/// nazhikas of 24 minutes, a nazhika 60 vinazhikas of 24 seconds, a vinazhika 60 viliptas.
/// Returns `(nazhika, vinazhika, vilipta)` with the vilipta fractional. At sunrise this is
/// `(0, 0, 0.0)`; 25 minutes after sunrise `(1, 2, 30.0)`. If `at` precedes `sunrise` the count
/// wraps within one day, so pass the sunrise that begins the solar day in question.
///
/// Source for the units: ta.wikipedia.org/wiki/நாழிகை (1 நாழிகை = 24 நிமிடங்கள் = 60 விநாழிகை),
/// the Kerala almanac reading "tithi until 11:28, from sunrise 12 nazhika 58 vinazhika"
/// (olamatch.com/en/calendar).
pub fn nazhika_vinazhika(sunrise: Instant, at: Instant) -> (u32, u32, f64) {
    let elapsed = at.ns_since(sunrise).rem_euclid(NS_PER_DAY);
    let nazhika = (elapsed / NAZHIKA_NS) as u32;
    let rest = elapsed % NAZHIKA_NS;
    let vinazhika = (rest / VINAZHIKA_NS) as u32;
    let vilipta = (rest % VINAZHIKA_NS) as f64 / VINAZHIKA_NS as f64 * 60.0;
    (nazhika, vinazhika, vilipta)
}

/// Kollavarsham month names, Chingam (the Sun in Simha) first.
pub const KOLLAVARSHAM_MONTHS: [&str; 12] = [
    "Chingam",
    "Kanni",
    "Thulam",
    "Vrischikam",
    "Dhanu",
    "Makaram",
    "Kumbham",
    "Meenam",
    "Medam",
    "Edavam",
    "Mithunam",
    "Karkidakam",
];

/// Tamil solar month names, Chithirai (the Sun in Mesha) first.
pub const TAMIL_MONTHS: [&str; 12] = [
    "Chithirai",
    "Vaikasi",
    "Aani",
    "Aadi",
    "Aavani",
    "Purattasi",
    "Aippasi",
    "Karthigai",
    "Margazhi",
    "Thai",
    "Maasi",
    "Panguni",
];

/// Saka/Vikrama lunar month names, Chaitra first.
pub const LUNAR_MONTHS: [&str; 12] = [
    "Chaitra",
    "Vaishakha",
    "Jyeshtha",
    "Ashadha",
    "Shravana",
    "Bhadrapada",
    "Ashwina",
    "Kartika",
    "Margashirsha",
    "Pausha",
    "Magha",
    "Phalguna",
];

/// Sidereal sign index 0–11 (Mesha = 0) of a longitude in degrees.
fn sign_index(sidereal_deg: f64) -> usize {
    ((sidereal_deg.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

/// The Tamil solar month for a sidereal Sun longitude: Chithirai while the Sun is in Mesha
/// (0°–30°), Vaikasi in Vrishabha, … Panguni in Meena.
pub fn tamil_month(sun_sidereal: f64) -> &'static str {
    TAMIL_MONTHS[sign_index(sun_sidereal)]
}

/// The Kollavarsham month for a sidereal Sun longitude: Chingam while the Sun is in Simha
/// (120°–150°), … Karkidakam in Karka.
pub fn malayalam_month(sun_sidereal: f64) -> &'static str {
    KOLLAVARSHAM_MONTHS[kollavarsham_month_index(sun_sidereal)]
}

/// Month index into [`KOLLAVARSHAM_MONTHS`] for a sidereal Sun longitude.
fn kollavarsham_month_index(sun_sidereal: f64) -> usize {
    (sign_index(sun_sidereal) + 8) % 12
}

/// The Kollavarsham (Malayalam era) date as `(year, month_index, day)`.
///
/// * `utc_day` is the civil day as days since 1970-01-01 (`Utc::utc_day`); it fixes the Kollam
///   year. Kollam year 1 began at Chingam 1 of 825 CE, so from Chingam 1 (mid-August) to
///   31 December the year is `CE − 824` and from 1 January to the day before Chingam 1 it is
///   `CE − 825` (en.wikipedia.org/wiki/Malayalam_calendar, boloji.com Kolla Varsham).
/// * `sun_sidereal` is the Sun's sidereal longitude in degrees; its sign gives the month,
///   Chingam = Simha, index 0 into [`KOLLAVARSHAM_MONTHS`].
/// * `days_since_sankranti` is supplied by the caller: the number of whole civil days from
///   the day counted as the month's first to the day in question, so the first day of the
///   month is `0` and the returned day is `days_since_sankranti + 1`. Kerala counts the day
///   of the Sankranti itself as day 1 when the ingress falls before aparahna (the first three
///   fifths of the daytime, sunrise to sunset) and the following day otherwise
///   (en.wikipedia.org/wiki/Malayalam_calendar; jathaka.com/tools/calendar); Tamil Nadu uses
///   sunset as the cut-off. The caller applies whichever rule it follows when it computes
///   `days_since_sankranti`.
pub fn kollavarsham(
    utc_day: i64,
    sun_sidereal: f64,
    days_since_sankranti: u32,
) -> (i64, usize, u32) {
    let month = kollavarsham_month_index(sun_sidereal);
    let (year, civil_month, _) = civil_from_days(utc_day);
    // Chingam … Dhanu (months 0–4) fall from mid-August into mid-January; while the civil
    // month is still August–December the Kollam year has already turned.
    let kollam_year = if month <= 4 && civil_month >= 8 {
        year - 824
    } else {
        year - 825
    };
    (kollam_year, month, days_since_sankranti + 1)
}

/// One paragraph on Vakya versus Drik panchanga, and which one Kshana is.
///
/// Sources: en.wikipedia.org/wiki/Vākyapañcāṅga, en.wikipedia.org/wiki/Drigganita,
/// mypanchang.com/vakyam.php, thirukanitham.com/drik-vs-vakya (reported tithi-ending gaps of
/// minutes to a few hours), astroyogam.in/guides/vakya-vs-thirukanitha-panchangam.
pub fn vakya_note() -> &'static str {
    "A Vakya panchangam (the Pambu panchangam of Tamil Nadu, and the older Kerala almanacs) is \
     computed from memorised Sanskrit verses, the vakyas, that encode the mean motions of the \
     Sun and Moon as they were fitted centuries ago; it is a tradition of arithmetic, not of \
     observation, and it still sets the ritual calendar for many families and temples. A Drik \
     (Drigganita, Thirukkanitha) panchangam computes the true positions of the Sun and Moon as \
     they are actually seen, which is why its eclipses land on the sky while a Vakya almanac's \
     may not. Kshana is a Drik panchangam: every tithi, nakshatra, yoga and karana ending is \
     solved from the JPL kernel for your place, with the ayanamsa you chose. It will therefore \
     differ from a Vakya almanac by up to a few ghatis in the ending times of tithis and stars, \
     and on some days that moves a fast or a festival to the neighbouring day. Neither is a \
     mistake; they are two conventions, and Kshana names which one it follows."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SIGN_NAMES;
    use kshana_vedic::panchanga::{NAKSHATRA_NAMES, YOGA_NAMES};

    fn family(name: &str) -> Vec<LexiconEntry> {
        lexicon(Language::English)
            .into_iter()
            .filter(|e| e.family == name)
            .collect()
    }

    #[test]
    fn families_have_their_counts_in_english() {
        for (name, n) in [
            ("rasi", 12),
            ("nakshatra", 27),
            ("tithi", 30),
            ("vara", 7),
            ("yoga", 27),
            ("karana", 11),
            ("graha", 9),
            ("bhava", 12),
            ("period", 6),
            ("clock", 12),
            ("observance", 3),
            ("month_kolla", 12),
            ("month_tamil", 12),
            ("month_lunar", 12),
        ] {
            let f = family(name);
            assert_eq!(f.len(), n, "{name}");
            assert!(
                f.iter()
                    .all(|e| !e.english.is_empty() && !e.native.is_empty() && !e.key.is_empty()),
                "{name}"
            );
        }
        assert_eq!(FAMILIES.len(), 14);
        let all = lexicon(Language::English);
        for f in FAMILIES {
            assert!(all.iter().any(|e| e.family == f), "{f} listed");
        }
        // Keys are unique across the lexicon.
        let mut keys: Vec<&str> = all.iter().map(|e| e.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), all.len());
    }

    #[test]
    fn keys_match_the_engine_spellings() {
        for (i, n) in NAKSHATRA_NAMES.iter().enumerate() {
            assert_eq!(NAKSHATRA[i].key, *n);
        }
        for (i, n) in YOGA_NAMES.iter().enumerate() {
            assert_eq!(YOGA[i].key, *n);
        }
        for (i, n) in SIGN_NAMES.iter().enumerate() {
            assert_eq!(RASI[i].key, *n);
        }
        assert_eq!(KOLLAVARSHAM_MONTHS.len(), 12);
        for (i, n) in KOLLAVARSHAM_MONTHS.iter().enumerate() {
            assert_eq!(MONTH_KOLLA[i].key, *n);
        }
        for (i, n) in TAMIL_MONTHS.iter().enumerate() {
            assert_eq!(MONTH_TAMIL[i].key, *n);
        }
        for (i, n) in LUNAR_MONTHS.iter().enumerate() {
            // Jyeshtha, Shravana and Magha are also nakshatras; those month keys carry a suffix.
            assert_eq!(MONTH_LUNAR[i].key.trim_end_matches(" (month)"), *n);
        }
    }

    #[test]
    fn malayalam_and_tamil_core_tables_are_complete() {
        for lang in [Language::Malayalam, Language::Tamil] {
            let l = lexicon(lang);
            for fam in [
                "rasi",
                "nakshatra",
                "tithi",
                "vara",
                "yoga",
                "karana",
                "graha",
            ] {
                let rows: Vec<_> = l.iter().filter(|e| e.family == fam).collect();
                assert!(!rows.is_empty());
                for e in rows {
                    assert!(
                        !e.native.is_empty() && !e.latin.is_empty(),
                        "{lang:?} {fam} {}",
                        e.key
                    );
                    assert!(
                        !e.native.is_ascii(),
                        "{lang:?} {fam} {} must be native script",
                        e.key
                    );
                }
            }
        }
        assert_eq!(term("Ashwini", Language::Malayalam), "അശ്വതി");
        assert_eq!(term_latin("Ashwini", Language::Malayalam), "Aswathi");
        assert_eq!(term("Revati", Language::Tamil), "ரேவதி");
        assert_eq!(term("Shukla Pratipada", Language::Telugu), "శుద్ధ పాడ్యమి");
        assert_eq!(term("Amavasya", Language::Kannada), "ಅಮಾವಾಸ್ಯೆ");
    }

    #[test]
    fn term_aliases_and_fallbacks() {
        assert_eq!(term("Aries", Language::Hindi), "मेष");
        assert_eq!(term("Mesha", Language::Malayalam), "മേടം");
        assert_eq!(term("mesha", Language::Malayalam), "മേടം");
        assert_eq!(term("Sunday", Language::Tamil), "ஞாயிறு");
        assert_eq!(term("Ravivara", Language::English), "Sunday");
        assert_eq!(
            term("Shukla Purnima/Amavasya", Language::Malayalam),
            "പൗർണമി (വെളുത്തവാവ്)"
        );
        assert_eq!(term("House 7", Language::Hindi), "युवति");
        assert_eq!(term("Guru", Language::Kannada), "ಗುರು");
        // Unverified cell: English fallback.
        assert_eq!(term("Choghadiya", Language::Malayalam), "Choghadiya");
        assert_eq!(term("Sahaja", Language::Kannada), "3rd house");
        assert_eq!(term_latin("Sahaja", Language::Kannada), "Sahaja");
        // Unknown key.
        assert_eq!(term("not a term", Language::Malayalam), "");
        assert_eq!(term_latin("not a term", Language::English), "");
        // English latin column carries the Sanskrit name.
        assert_eq!(term_latin("Aries", Language::English), "Mesha");
        // The lexicon keeps empty cells empty so a screen can mark them.
        let kn = lexicon(Language::Kannada);
        let sahaja = kn.iter().find(|e| e.key == "Sahaja").unwrap();
        assert_eq!(sahaja.native, "");
        assert_eq!(sahaja.english, "3rd house");
    }

    #[test]
    fn languages() {
        let names = language_names();
        assert_eq!(names.len(), 6);
        assert_eq!(names[0], ("en", "English"));
        assert_eq!(names[1], ("ml", "മലയാളം"));
        assert_eq!(Language::from_code("ML"), Some(Language::Malayalam));
        assert_eq!(Language::from_code(" ta "), Some(Language::Tamil));
        assert_eq!(Language::from_code("fr"), None);
        for l in Language::ALL {
            assert_eq!(Language::from_code(l.code()), Some(l));
            assert!(!l.english_name().is_empty());
        }
    }

    #[test]
    fn nazhika_arithmetic() {
        let sunrise = Instant::j2000();
        assert_eq!(nazhika_vinazhika(sunrise, sunrise), (0, 0, 0.0));
        let (n, v, l) = nazhika_vinazhika(sunrise, sunrise.add_seconds_f64(24.0 * 60.0));
        assert_eq!((n, v), (1, 0));
        assert!(l.abs() < 1e-9);
        let (n, v, l) = nazhika_vinazhika(sunrise, sunrise.add_seconds_f64(25.0 * 60.0));
        assert_eq!((n, v), (1, 2));
        assert!(
            (l - 30.0).abs() < 1e-9,
            "2.5 vinazhika = 2 vinazhika 30 vilipta, got {l}"
        );
        let (n, v, l) = nazhika_vinazhika(
            sunrise,
            sunrise.add_seconds_f64(12.0 * 3600.0 + 17.0 * 60.0 + 36.0),
        );
        assert_eq!((n, v), (30, 44));
        assert!(l.abs() < 1e-9);
        // Just before the next sunrise: the last vinazhika of the day.
        let (n, v, _) = nazhika_vinazhika(sunrise, sunrise.add_seconds_f64(86_400.0 - 1.0));
        assert_eq!((n, v), (59, 59));
        // A full day later wraps to zero.
        assert_eq!(
            nazhika_vinazhika(sunrise, sunrise.add_seconds_f64(86_400.0)).0,
            0
        );
    }

    #[test]
    fn tamil_month_boundaries() {
        assert_eq!(tamil_month(0.0), "Chithirai");
        assert_eq!(tamil_month(29.999), "Chithirai");
        assert_eq!(tamil_month(30.0), "Vaikasi");
        assert_eq!(tamil_month(120.0), "Aavani");
        assert_eq!(tamil_month(270.0), "Thai");
        assert_eq!(tamil_month(359.999), "Panguni");
        assert_eq!(tamil_month(360.0), "Chithirai");
        assert_eq!(tamil_month(-0.001), "Panguni");
        assert_eq!(tamil_month(725.0), "Chithirai");
        assert_eq!(malayalam_month(120.0), "Chingam");
        assert_eq!(malayalam_month(119.999), "Karkidakam");
        assert_eq!(malayalam_month(0.0), "Medam");
        assert_eq!(malayalam_month(359.0), "Meenam");
    }

    #[test]
    fn kollavarsham_year_and_month() {
        use kshana_time::days_from_civil;
        // 19 October 2026, the Sun early in Thulam: Thulam 3, 1202.
        let (y, m, d) = kollavarsham(days_from_civil(2026, 10, 19), 182.0, 2);
        assert_eq!((y, m, d), (1202, 2, 3));
        assert_eq!(KOLLAVARSHAM_MONTHS[m], "Thulam");
        // Dhanu in late December and in January share the Kollam year.
        assert_eq!(
            kollavarsham(days_from_civil(2026, 12, 25), 248.0, 9).0,
            1202
        );
        assert_eq!(
            kollavarsham(days_from_civil(2027, 1, 10), 265.0, 25).0,
            1202
        );
        // Makaram 1, 1202 on 15 January 2027.
        assert_eq!(
            kollavarsham(days_from_civil(2027, 1, 15), 270.2, 0),
            (1202, 5, 1)
        );
        // Karkidakam 1201 ends mid-August 2026; Chingam 1, 1202 begins.
        assert_eq!(
            kollavarsham(days_from_civil(2026, 8, 16), 119.6, 31),
            (1201, 11, 32)
        );
        assert_eq!(
            kollavarsham(days_from_civil(2026, 8, 17), 120.1, 0),
            (1202, 0, 1)
        );
        // Medam (Vishu) 2026 is 1201.
        assert_eq!(
            kollavarsham(days_from_civil(2026, 4, 14), 0.3, 0),
            (1201, 8, 1)
        );
    }

    #[test]
    fn vakya_note_says_which_one_kshana_is() {
        let n = vakya_note();
        assert!(n.contains("Kshana is a Drik panchangam"));
        assert!(n.contains("Vakya"));
        assert!(n.contains("ghatis"));
        assert!(n.len() > 400);
    }
}
