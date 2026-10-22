//! The names of the calendar: lunar months, the sixty-year cycle, seasons, the
//! regional solar months, tithis and weekdays.

/// The twelve lunar months, 1 = Chaitra. A month is named after the sign the Sun
/// enters during it (Reingold & Dershowitz, *Calendrical Calculations*, 3rd ed.,
/// ch. 20 "Indian Calendars", §20.3 "Hindu lunisolar calendar"; Indian Calendar Reform
/// Committee Report, 1955, ch. V).
pub const LUNAR_MONTHS: [&str; 12] = [
    "Chaitra",
    "Vaishakha",
    "Jyeshtha",
    "Ashadha",
    "Shravana",
    "Bhadrapada",
    "Ashvina",
    "Kartika",
    "Margashirsha",
    "Pausha",
    "Magha",
    "Phalguna",
];

/// The sixty years of the Jupiter cycle, 0 = Prabhava (Surya Siddhanta 1.55 counts the
/// cycle from Vijaya at the Kali epoch; the names and order are those of the Rashtriya
/// Panchang and of every South Indian almanac).
pub const SAMVATSARAS: [&str; 60] = [
    "Prabhava",
    "Vibhava",
    "Shukla",
    "Pramoda",
    "Prajapati",
    "Angirasa",
    "Shrimukha",
    "Bhava",
    "Yuva",
    "Dhatri",
    "Ishvara",
    "Bahudhanya",
    "Pramathi",
    "Vikrama",
    "Vrisha",
    "Chitrabhanu",
    "Svabhanu",
    "Tarana",
    "Parthiva",
    "Vyaya",
    "Sarvajit",
    "Sarvadhari",
    "Virodhi",
    "Vikriti",
    "Khara",
    "Nandana",
    "Vijaya",
    "Jaya",
    "Manmatha",
    "Durmukhi",
    "Hevilambi",
    "Vilambi",
    "Vikari",
    "Sharvari",
    "Plava",
    "Shubhakrit",
    "Shobhakrit",
    "Krodhi",
    "Vishvavasu",
    "Parabhava",
    "Plavanga",
    "Kilaka",
    "Saumya",
    "Sadharana",
    "Virodhikrit",
    "Paridhavi",
    "Pramadi",
    "Ananda",
    "Rakshasa",
    "Nala",
    "Pingala",
    "Kalayukti",
    "Siddharthi",
    "Raudra",
    "Durmati",
    "Dundubhi",
    "Rudhirodgari",
    "Raktakshi",
    "Krodhana",
    "Akshaya",
];

/// The six seasons, 0 = Vasanta.
pub const RITUS: [&str; 6] = ["Vasanta", "Grishma", "Varsha", "Sharad", "Hemanta", "Shishira"];

/// The seasons in plain English, same order.
pub const RITUS_PLAIN: [&str; 6] = ["spring", "summer", "monsoon", "autumn", "pre-winter", "winter"];

/// Malayalam solar months, indexed by the sign the Sun is in (0 = Mesha → Medam).
pub const MALAYALAM_MONTHS: [&str; 12] = [
    "Medam",
    "Edavam",
    "Mithunam",
    "Karkidakam",
    "Chingam",
    "Kanni",
    "Thulam",
    "Vrischikam",
    "Dhanu",
    "Makaram",
    "Kumbham",
    "Meenam",
];

/// Tamil solar months, indexed by the sign the Sun is in (0 = Mesha → Chithirai).
pub const TAMIL_MONTHS: [&str; 12] = [
    "Chithirai",
    "Vaikasi",
    "Aani",
    "Aadi",
    "Avani",
    "Purattasi",
    "Aippasi",
    "Karthigai",
    "Margazhi",
    "Thai",
    "Maasi",
    "Panguni",
];

/// Bengali solar months, indexed by the sign the Sun is in (0 = Mesha → Boishakh).
pub const BENGALI_MONTHS: [&str; 12] = [
    "Boishakh",
    "Joishtho",
    "Asharh",
    "Srabon",
    "Bhadro",
    "Ashwin",
    "Kartik",
    "Ogrohayon",
    "Poush",
    "Magh",
    "Falgun",
    "Choitro",
];

/// Odia solar months, indexed by the sign the Sun is in (0 = Mesha → Baisakha).
pub const ODIA_MONTHS: [&str; 12] = [
    "Baisakha",
    "Jyeshtha",
    "Ashadha",
    "Shrabana",
    "Bhadraba",
    "Ashwina",
    "Kartika",
    "Margashira",
    "Pousha",
    "Magha",
    "Phalguna",
    "Chaitra",
];

/// Months of the Indian National Calendar (Calendar Reform Committee, 1955; in civil
/// use from 22 March 1957), 0 = Chaitra.
pub const NATIONAL_MONTHS: [&str; 12] = [
    "Chaitra",
    "Vaishakha",
    "Jyeshtha",
    "Ashadha",
    "Shravana",
    "Bhadra",
    "Ashvina",
    "Kartika",
    "Agrahayana",
    "Pausha",
    "Magha",
    "Phalguna",
];

/// Weekdays in English, 0 = Sunday.
pub const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// The name of tithi `n` (1–30) with its fortnight, e.g. "Shukla Ekadashi".
pub fn tithi_full_name(n: u8) -> String {
    let n = n.clamp(1, 30);
    let within = kshana_vedic::panchanga::TITHI_NAMES[((n - 1) % 15) as usize];
    match n {
        15 => "Purnima".to_string(),
        30 => "Amavasya".to_string(),
        1..=14 => format!("Shukla {within}"),
        _ => format!("Krishna {within}"),
    }
}

/// The name of nakshatra `i` (0 = Ashwini).
pub fn star_name(i: u8) -> &'static str {
    kshana_vedic::panchanga::NAKSHATRA_NAMES[i as usize % 27]
}

/// The traditional name of the Ekadashi of an amanta month (1 = Chaitra), by fortnight;
/// `adhika` months have their own pair (Padmini, Parama). Names as listed in the
/// Padma Purana's Ekadashi-mahatmya and printed in every North Indian panchang.
pub fn ekadashi_name(month: u8, shukla: bool, adhika: bool) -> &'static str {
    if adhika {
        return if shukla { "Padmini" } else { "Parama" };
    }
    // Krishna-paksha names follow the amanta month (the purnimanta month is one later).
    const S: [&str; 12] = [
        "Kamada",
        "Mohini",
        "Nirjala",
        "Devshayani",
        "Shravana Putrada",
        "Parsva",
        "Papankusha",
        "Devutthana",
        "Mokshada",
        "Pausha Putrada",
        "Jaya",
        "Amalaki",
    ];
    const K: [&str; 12] = [
        "Varuthini",
        "Apara",
        "Yogini",
        "Kamika",
        "Aja",
        "Indira",
        "Rama",
        "Utpanna",
        "Saphala",
        "Shattila",
        "Vijaya",
        "Papmochani",
    ];
    let i = (month.clamp(1, 12) - 1) as usize;
    if shukla {
        S[i]
    } else {
        K[i]
    }
}

/// The reader's language as an index into the `[en, ml, hi]` tables below: 1 for "ml",
/// 2 for "hi", 0 (English) for anything else.
pub fn lang_index(code: &str) -> usize {
    match code.trim().to_ascii_lowercase().as_str() {
        "ml" => 1,
        "hi" => 2,
        _ => 0,
    }
}

/// The sixty years in Malayalam script, same order as [`SAMVATSARAS`].
pub const SAMVATSARAS_ML: [&str; 60] = [
    "പ്രഭവ", "വിഭവ", "ശുക്ല", "പ്രമോദ", "പ്രജാപതി", "ആംഗിരസ", "ശ്രീമുഖ", "ഭാവ", "യുവ", "ധാതു",
    "ഈശ്വര", "ബഹുധാന്യ", "പ്രമാഥി", "വിക്രമ", "വൃഷ", "ചിത്രഭാനു", "സ്വഭാനു", "താരണ", "പാർത്ഥിവ", "വ്യയ",
    "സർവജിത്", "സർവധാരി", "വിരോധി", "വികൃതി", "ഖര", "നന്ദന", "വിജയ", "ജയ", "മന്മഥ", "ദുർമുഖി",
    "ഹേവിളംബി", "വിളംബി", "വികാരി", "ശാർവരി", "പ്ലവ", "ശുഭകൃത്", "ശോഭകൃത്", "ക്രോധി", "വിശ്വാവസു", "പരാഭവ",
    "പ്ലവംഗ", "കീലക", "സൗമ്യ", "സാധാരണ", "വിരോധികൃത്", "പരിധാവി", "പ്രമാദി", "ആനന്ദ", "രാക്ഷസ", "നള",
    "പിംഗള", "കാളയുക്തി", "സിദ്ധാർത്ഥി", "രൗദ്ര", "ദുർമതി", "ദുന്ദുഭി", "രുധിരോദ്ഗാരി", "രക്താക്ഷി", "ക്രോധന", "അക്ഷയ",
];

/// The sixty years in Devanagari, same order as [`SAMVATSARAS`].
pub const SAMVATSARAS_HI: [&str; 60] = [
    "प्रभव", "विभव", "शुक्ल", "प्रमोद", "प्रजापति", "आंगिरस", "श्रीमुख", "भाव", "युवा", "धाता",
    "ईश्वर", "बहुधान्य", "प्रमाथी", "विक्रम", "वृष", "चित्रभानु", "स्वभानु", "तारण", "पार्थिव", "व्यय",
    "सर्वजित", "सर्वधारी", "विरोधी", "विकृति", "खर", "नंदन", "विजय", "जय", "मन्मथ", "दुर्मुख",
    "हेविलंबी", "विलंबी", "विकारी", "शार्वरी", "प्लव", "शुभकृत", "शोभकृत", "क्रोधी", "विश्वावसु", "पराभव",
    "प्लवंग", "कीलक", "सौम्य", "साधारण", "विरोधकृत", "परिधावी", "प्रमादी", "आनंद", "राक्षस", "नल",
    "पिंगल", "कालयुक्त", "सिद्धार्थी", "रौद्र", "दुर्मति", "दुंदुभी", "रुधिरोद्गारी", "रक्ताक्षी", "क्रोधन", "अक्षय",
];

/// A sixty-year name in the reader's language.
pub fn samvatsara_in(i: usize, lang: &str) -> &'static str {
    [SAMVATSARAS[i % 60], SAMVATSARAS_ML[i % 60], SAMVATSARAS_HI[i % 60]][lang_index(lang)]
}

const RITUS_IN: [[&str; 3]; 6] = [
    ["Vasanta", "വസന്തം", "वसंत"],
    ["Grishma", "ഗ്രീഷ്മം", "ग्रीष्म"],
    ["Varsha", "വർഷം", "वर्षा"],
    ["Sharad", "ശരത്", "शरद"],
    ["Hemanta", "ഹേമന്തം", "हेमंत"],
    ["Shishira", "ശിശിരം", "शिशिर"],
];

/// A season's traditional name in the reader's language (expert text).
pub fn ritu_in(i: usize, lang: &str) -> &'static str {
    RITUS_IN[i % 6][lang_index(lang)]
}

const RITUS_PLAIN_IN: [[&str; 3]; 6] = [
    ["spring", "വസന്തകാലം", "वसंत"],
    ["summer", "വേനൽക്കാലം", "गर्मी"],
    ["monsoon", "മഴക്കാലം", "बरसात"],
    ["autumn", "ശരത്കാലം", "शरद ऋतु"],
    ["pre-winter", "മഞ്ഞുകാലത്തിന്റെ തുടക്കം", "जाड़े की शुरुआत"],
    ["winter", "മഞ്ഞുകാലം", "जाड़ा"],
];

/// A season in plain words in the reader's language.
pub fn ritu_plain_in(i: usize, lang: &str) -> &'static str {
    RITUS_PLAIN_IN[i % 6][lang_index(lang)]
}

const NATIONAL_MONTHS_IN: [[&str; 3]; 12] = [
    ["Chaitra", "ചൈത്രം", "चैत्र"],
    ["Vaishakha", "വൈശാഖം", "वैशाख"],
    ["Jyeshtha", "ജ്യേഷ്ഠം", "ज्येष्ठ"],
    ["Ashadha", "ആഷാഢം", "आषाढ़"],
    ["Shravana", "ശ്രാവണം", "श्रावण"],
    ["Bhadra", "ഭാദ്രം", "भाद्र"],
    ["Ashvina", "ആശ്വിനം", "आश्विन"],
    ["Kartika", "കാർത്തികം", "कार्तिक"],
    ["Agrahayana", "അഗ്രഹായണം", "अग्रहायण"],
    ["Pausha", "പൗഷം", "पौष"],
    ["Magha", "മാഘം", "माघ"],
    ["Phalguna", "ഫാൽഗുനം", "फाल्गुन"],
];

/// A month of the Indian National Calendar (0 = Chaitra) in the reader's language.
pub fn national_month_in(i: usize, lang: &str) -> &'static str {
    NATIONAL_MONTHS_IN[i % 12][lang_index(lang)]
}

/// A lunar month (1 = Chaitra) in the reader's language, "Adhika" before the extra month.
/// English is exactly `MonthName::label`.
pub fn lunar_month_in(number: u8, adhika: bool, lang: &str) -> String {
    let i = (number.clamp(1, 12) - 1) as usize;
    // The lunar sixth month is Bhadrapada; the national calendar calls it Bhadra.
    let n = if i == 5 { ["Bhadrapada", "ഭാദ്രപദം", "भाद्रपद"] } else { [LUNAR_MONTHS[i], NATIONAL_MONTHS_IN[i][1], NATIONAL_MONTHS_IN[i][2]] };
    let l = lang_index(lang);
    if adhika {
        format!("{} {}", ["Adhika", "അധിക", "अधिक"][l], n[l])
    } else {
        n[l].to_string()
    }
}

const EKADASHI_IN: [[&str; 3]; 26] = [
    ["Kamada", "കാമദ", "कामदा"],
    ["Mohini", "മോഹിനി", "मोहिनी"],
    ["Nirjala", "നിർജല", "निर्जला"],
    ["Devshayani", "ദേവശയനി", "देवशयनी"],
    ["Shravana Putrada", "ശ്രാവണ പുത്രദ", "श्रावण पुत्रदा"],
    ["Parsva", "പാർശ്വ", "पार्श्व"],
    ["Papankusha", "പാപാങ്കുശ", "पापांकुशा"],
    ["Devutthana", "ദേവോത്ഥാന", "देवउठनी"],
    ["Mokshada", "മോക്ഷദ", "मोक्षदा"],
    ["Pausha Putrada", "പൗഷ പുത്രദ", "पौष पुत्रदा"],
    ["Jaya", "ജയ", "जया"],
    ["Amalaki", "ആമലകി", "आमलकी"],
    ["Varuthini", "വരൂഥിനി", "वरूथिनी"],
    ["Apara", "അപര", "अपरा"],
    ["Yogini", "യോഗിനി", "योगिनी"],
    ["Kamika", "കാമിക", "कामिका"],
    ["Aja", "അജ", "अजा"],
    ["Indira", "ഇന്ദിര", "इंदिरा"],
    ["Rama", "രമ", "रमा"],
    ["Utpanna", "ഉത്പന്ന", "उत्पन्ना"],
    ["Saphala", "സഫല", "सफला"],
    ["Shattila", "ഷട്തില", "षटतिला"],
    ["Vijaya", "വിജയ", "विजया"],
    ["Papmochani", "പാപമോചനി", "पापमोचनी"],
    ["Padmini", "പദ്മിനി", "पद्मिनी"],
    ["Parama", "പരമ", "परमा"],
];

/// An Ekadashi's own name (as [`ekadashi_name`] gives it) in the reader's language;
/// `None` for a name not in the table.
pub fn ekadashi_name_in(en: &str, lang: &str) -> Option<&'static str> {
    EKADASHI_IN.iter().find(|r| r[0] == en).map(|r| r[lang_index(lang)])
}

const MALAYALAM_MONTHS_IN: [[&str; 2]; 12] = [
    ["മേടം", "मेडम"],
    ["ഇടവം", "इडवम"],
    ["മിഥുനം", "मिथुनम"],
    ["കർക്കടകം", "कर्किडकम"],
    ["ചിങ്ങം", "चिंगम"],
    ["കന്നി", "कन्नि"],
    ["തുലാം", "तुलाम"],
    ["വൃശ്ചികം", "वृश्चिकम"],
    ["ധനു", "धनु"],
    ["മകരം", "मकरम"],
    ["കുംഭം", "कुंभम"],
    ["മീനം", "मीनम"],
];

const TAMIL_MONTHS_IN: [[&str; 2]; 12] = [
    ["ചിത്തിര", "चित्तिरै"],
    ["വൈകാശി", "वैकासी"],
    ["ആനി", "आनी"],
    ["ആടി", "आडी"],
    ["ആവണി", "आवणी"],
    ["പുരട്ടാശി", "पुरट्टासी"],
    ["ഐപ്പശി", "ऐप्पसी"],
    ["കാർത്തിക", "कार्तिगै"],
    ["മാർകഴി", "मार्गळी"],
    ["തൈ", "तै"],
    ["മാസി", "मासी"],
    ["പങ്കുനി", "पंगुनी"],
];

/// A regional solar month (by the sign the Sun is in, 0 = Mesha) in the reader's
/// language. Bengali and Odia months carry the lunar names, one month on from the
/// sign (the Sun in Mesha is Boishakh / Baisakha).
pub fn solar_month_in(rule: crate::SolarRule, sign: u8, lang: &str) -> &'static str {
    let s = sign as usize % 12;
    let l = lang_index(lang);
    if l == 0 {
        return rule.months()[s];
    }
    match rule {
        crate::SolarRule::Kerala => MALAYALAM_MONTHS_IN[s][l - 1],
        crate::SolarRule::Tamil => TAMIL_MONTHS_IN[s][l - 1],
        _ => national_month_in((s + 1) % 12, lang),
    }
}

/// "waxing" / "waning" in the reader's language.
pub fn fortnight_in(shukla: bool, lang: &str) -> &'static str {
    let t = if shukla { ["waxing", "വെളുത്ത പക്ഷം", "शुक्ल पक्ष"] } else { ["waning", "കറുത്ത പക്ഷം", "कृष्ण पक्ष"] };
    t[lang_index(lang)]
}

/// "Uttarayana" / "Dakshinayana" in the reader's language (expert text).
pub fn ayana_in(uttara: bool, lang: &str) -> &'static str {
    let t = if uttara { ["Uttarayana", "ഉത്തരായനം", "उत्तरायण"] } else { ["Dakshinayana", "ദക്ഷിണായനം", "दक्षिणायन"] };
    t[lang_index(lang)]
}

/// A weekday (0 = Sunday) in the reader's language.
pub fn weekday_in(i: usize, lang: &str) -> &'static str {
    const W: [[&str; 3]; 7] = [
        ["Sunday", "ഞായറാഴ്ച", "रविवार"],
        ["Monday", "തിങ്കളാഴ്ച", "सोमवार"],
        ["Tuesday", "ചൊവ്വാഴ്ച", "मंगलवार"],
        ["Wednesday", "ബുധനാഴ്ച", "बुधवार"],
        ["Thursday", "വ്യാഴാഴ്ച", "गुरुवार"],
        ["Friday", "വെള്ളിയാഴ്ച", "शुक्रवार"],
        ["Saturday", "ശനിയാഴ്ച", "शनिवार"],
    ];
    W[i % 7][lang_index(lang)]
}

#[cfg(test)]
mod lang_tests {
    use super::*;

    #[test]
    fn english_is_unchanged_and_every_table_is_full() {
        for i in 0..60 {
            assert_eq!(samvatsara_in(i, "en"), SAMVATSARAS[i]);
            assert!(!samvatsara_in(i, "ml").is_empty() && !samvatsara_in(i, "hi").is_empty());
        }
        for i in 0..6 {
            assert_eq!(ritu_in(i, "en"), RITUS[i]);
            assert_eq!(ritu_plain_in(i, "en"), RITUS_PLAIN[i]);
        }
        for i in 0..12 {
            assert_eq!(national_month_in(i, "en"), NATIONAL_MONTHS[i]);
            for adhika in [false, true] {
                let m = crate::MonthName { number: i as u8 + 1, adhika };
                assert_eq!(lunar_month_in(i as u8 + 1, adhika, "en"), m.label());
            }
        }
        for m in 1..=12 {
            for s in [true, false] {
                for a in [true, false] {
                    let n = ekadashi_name(m, s, a);
                    assert_eq!(ekadashi_name_in(n, "en"), Some(n));
                    assert!(ekadashi_name_in(n, "ml").is_some());
                }
            }
        }
        for i in 0..7 {
            assert_eq!(weekday_in(i, "en"), WEEKDAYS[i]);
        }
    }
}
