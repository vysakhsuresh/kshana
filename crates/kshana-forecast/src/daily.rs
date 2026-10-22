//! The day for one person: good hours, hours to avoid, the Moon's standing.

use kshana_chart::Chart;
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{hora_at, rahu_kala, solar_day, SolarDay};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, nakshatra, sidereal, Ayanamsa};
use kshana_rules::Lang;

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
    lang: Lang,
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
            reason: lang.pick(["Rahu kala", "രാഹുകാലം", "राहुकाल"]).into(),
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
                    reason: match lang {
                        Lang::En => format!("{lord} hora"),
                        Lang::Ml => format!("{} ഹോര", words::graha(lord, lang)),
                        Lang::Hi => format!("{} होरा", words::graha(lord, lang)),
                    },
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
            reason: lang.pick(["Abhijit", "അഭിജിത്", "अभिजित्"]).into(),
            avoid: false,
        });
    }
    windows.sort_by_key(|w| w.start);
    let (star, tara) = (words::star(n.index, lang), words::tara(count, lang));
    let headline = match (lang, tara_good, cb) {
        (Lang::En, true, true) => format!("A supportive day: the Moon is in {} ({}), a good star for you, and well placed from your natal Moon.", n.name(), TARA[count]),
        (Lang::En, true, false) => format!("A mixed day: the Moon's star {} is good for you, but its sign is not well placed from your natal Moon.", n.name()),
        (Lang::En, false, true) => format!("A cautious day: the Moon's star {} ({}) is unfavourable for you, though its sign is well placed.", n.name(), TARA[count]),
        (Lang::En, false, false) => format!("A day to keep simple: the Moon's star {} ({}) and sign are both unfavourable for you.", n.name(), TARA[count]),
        (Lang::Ml, true, true) => format!("തുണയുള്ള ദിവസം: ചന്ദ്രൻ {} നക്ഷത്രത്തിൽ ({}), നിങ്ങൾക്ക് നല്ല നാൾ, ജന്മചന്ദ്രനിൽ നിന്ന് നല്ല സ്ഥാനത്തും."  , star, tara),
        (Lang::Ml, true, false) => format!("ഇടകലർന്ന ദിവസം: ചന്ദ്രന്റെ നക്ഷത്രം {} നിങ്ങൾക്ക് നല്ലതാണ്, പക്ഷേ അതിന്റെ രാശി ജന്മചന്ദ്രനിൽ നിന്ന് നല്ല സ്ഥാനത്തല്ല.", star),
        (Lang::Ml, false, true) => format!("ശ്രദ്ധിക്കേണ്ട ദിവസം: ചന്ദ്രന്റെ നക്ഷത്രം {} ({}) നിങ്ങൾക്ക് അനുകൂലമല്ല, എങ്കിലും രാശി നല്ല സ്ഥാനത്താണ്.", star, tara),
        (Lang::Ml, false, false) => format!("ലളിതമായി കൊണ്ടുപോകേണ്ട ദിവസം: ചന്ദ്രന്റെ നക്ഷത്രം {} ({}) രാശിയും നിങ്ങൾക്ക് അനുകൂലമല്ല.", star, tara),
        (Lang::Hi, true, true) => format!("सहायक दिन: चंद्र {} नक्षत्र में है ({}), आपके लिए अच्छा नक्षत्र, और जन्म चंद्र से अच्छी स्थिति में भी।", star, tara),
        (Lang::Hi, true, false) => format!("मिला-जुला दिन: चंद्र का नक्षत्र {} आपके लिए अच्छा है, पर उसकी राशि जन्म चंद्र से अच्छी स्थिति में नहीं।", star),
        (Lang::Hi, false, true) => format!("सावधानी का दिन: चंद्र का नक्षत्र {} ({}) आपके अनुकूल नहीं, हालाँकि उसकी राशि अच्छी स्थिति में है।", star, tara),
        (Lang::Hi, false, false) => format!("सादा रखने का दिन: चंद्र का नक्षत्र {} ({}) और राशि दोनों आपके अनुकूल नहीं।", star, tara),
    };
    Ok(DailyForecast {
        day,
        tara: (tara.to_string(), tara_good),
        chandra_bala: (from_moon, cb),
        moon_nakshatra: star.to_string(),
        windows,
        headline,
    })
}

/// Names in the reader's language for this crate's own sentences: the planets, signs,
/// nakshatras and taras as Kerala panchangams and Hindi panchangs print them. (The full
/// lexicon lives in `kshana-speak`, above this crate.) English is the engine's spelling.
pub mod words {
    use kshana_rules::Lang;

    const GRAHA: [(&str, &str, &str); 9] = [
        ("Sun", "സൂര്യൻ", "सूर्य"),
        ("Moon", "ചന്ദ്രൻ", "चंद्र"),
        ("Mars", "ചൊവ്വ", "मंगल"),
        ("Mercury", "ബുധൻ", "बुध"),
        ("Jupiter", "വ്യാഴം", "गुरु"),
        ("Venus", "ശുക്രൻ", "शुक्र"),
        ("Saturn", "ശനി", "शनि"),
        ("Rahu", "രാഹു", "राहु"),
        ("Ketu", "കേതു", "केतु"),
    ];

    const SIGN: [(&str, &str, &str); 12] = [
        ("Aries", "മേടം", "मेष"),
        ("Taurus", "ഇടവം", "वृषभ"),
        ("Gemini", "മിഥുനം", "मिथुन"),
        ("Cancer", "കർക്കടകം", "कर्क"),
        ("Leo", "ചിങ്ങം", "सिंह"),
        ("Virgo", "കന്നി", "कन्या"),
        ("Libra", "തുലാം", "तुला"),
        ("Scorpio", "വൃശ്ചികം", "वृश्चिक"),
        ("Sagittarius", "ധനു", "धनु"),
        ("Capricorn", "മകരം", "मकर"),
        ("Aquarius", "കുംഭം", "कुंभ"),
        ("Pisces", "മീനം", "मीन"),
    ];

    const STAR: [(&str, &str, &str); 27] = [
        ("Ashwini", "അശ്വതി", "अश्विनी"),
        ("Bharani", "ഭരണി", "भरणी"),
        ("Krittika", "കാർത്തിക", "कृत्तिका"),
        ("Rohini", "രോഹിണി", "रोहिणी"),
        ("Mrigashira", "മകയിരം", "मृगशिरा"),
        ("Ardra", "തിരുവാതിര", "आर्द्रा"),
        ("Punarvasu", "പുണർതം", "पुनर्वसु"),
        ("Pushya", "പൂയം", "पुष्य"),
        ("Ashlesha", "ആയില്യം", "आश्लेषा"),
        ("Magha", "മകം", "मघा"),
        ("Purva Phalguni", "പൂരം", "पूर्वाफाल्गुनी"),
        ("Uttara Phalguni", "ഉത്രം", "उत्तराफाल्गुनी"),
        ("Hasta", "അത്തം", "हस्त"),
        ("Chitra", "ചിത്തിര", "चित्रा"),
        ("Swati", "ചോതി", "स्वाति"),
        ("Vishakha", "വിശാഖം", "विशाखा"),
        ("Anuradha", "അനിഴം", "अनुराधा"),
        ("Jyeshtha", "തൃക്കേട്ട", "ज्येष्ठा"),
        ("Mula", "മൂലം", "मूल"),
        ("Purva Ashadha", "പൂരാടം", "पूर्वाषाढ़ा"),
        ("Uttara Ashadha", "ഉത്രാടം", "उत्तराषाढ़ा"),
        ("Shravana", "തിരുവോണം", "श्रवण"),
        ("Dhanishta", "അവിട്ടം", "धनिष्ठा"),
        ("Shatabhisha", "ചതയം", "शतभिषा"),
        ("Purva Bhadrapada", "പൂരുരുട്ടാതി", "पूर्वाभाद्रपद"),
        ("Uttara Bhadrapada", "ഉത്രട്ടാതി", "उत्तराभाद्रपद"),
        ("Revati", "രേവതി", "रेवती"),
    ];

    const TARA: [(&str, &str, &str); 9] = [
        ("Janma", "ജന്മ", "जन्म"),
        ("Sampat", "സമ്പത്ത്", "संपत्"),
        ("Vipat", "വിപത്ത്", "विपत्"),
        ("Kshema", "ക്ഷേമം", "क्षेम"),
        ("Pratyak", "പ്രത്യക്", "प्रत्यक्"),
        ("Sadhana", "സാധന", "साधना"),
        ("Naidhana", "നൈധനം", "नैधन"),
        ("Mitra", "മിത്രം", "मित्र"),
        ("Parama Mitra", "പരമമിത്രം", "परम मित्र"),
    ];

    fn pick(row: &(&'static str, &'static str, &'static str), lang: Lang) -> &'static str {
        lang.pick([row.0, row.1, row.2])
    }

    /// A planet by its English name; unknown names come back as given.
    pub fn graha<'a>(name: &'a str, lang: Lang) -> &'a str {
        GRAHA
            .iter()
            .find(|r| r.0 == name)
            .map_or(name, |r| pick(r, lang))
    }

    /// A sign by index, 0 = Aries.
    pub fn sign(i: usize, lang: Lang) -> &'static str {
        pick(&SIGN[i % 12], lang)
    }

    /// A sign by its English name; unknown names come back as given.
    pub fn sign_named<'a>(name: &'a str, lang: Lang) -> &'a str {
        SIGN.iter()
            .find(|r| r.0 == name)
            .map_or(name, |r| pick(r, lang))
    }

    /// A nakshatra by index, 0 = Ashwini.
    pub fn star(i: usize, lang: Lang) -> &'static str {
        pick(&STAR[i % 27], lang)
    }

    /// A tara by its count from the birth star, 0 = Janma.
    pub fn tara(i: usize, lang: Lang) -> &'static str {
        pick(&TARA[i % 9], lang)
    }
}
