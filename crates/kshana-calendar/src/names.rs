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
