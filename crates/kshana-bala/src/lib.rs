//! Shadbala: the six-fold strength of the seven grahas, the strength of the twelve
//! bhavas, and the planetary avasthas, as Brihat Parashara Hora Shastra ch. 27 lays them
//! out and B. V. Raman's *Graha and Bhava Balas* works them.
//!
//! Everything is in virupas (sixtieths of a rupa). [`shadbala`] takes a computed
//! [`Chart`] and returns, for each graha, the five parts of Sthana bala, the Dig bala,
//! the nine parts of Kala bala, the Chesta, Naisargika and Drik balas, the total in
//! virupas and rupas, the required minimum and the ratio to it, the Ishta and Kashta
//! phalas, and a few sentences that say where each number came from; then the Bhava
//! balas, the lords of the year, month, day and hora, and any planetary wars.
//! [`avasthas`] gives the Baladi, Jagradadi and Deeptadi states.
//!
//! # Sources and conventions
//!
//! * BPHS ch. 27 (R. Santhanam's translation, Ranjan, 1984) for the structure, the
//!   tables of Sthana, Kala, Chesta, Naisargika and Drik bala, the war rule and the
//!   required strengths; ch. 26 for the sputa drishti; ch. 45 for the avasthas.
//! * B. V. Raman, *Graha and Bhava Balas* (UBS, 1992 ed.), for the arithmetic forms,
//!   the Saptavargaja values (22.5, 15, 7.5, 3.75, 1.875), the disc diameters, the
//!   Bhava Dig bala sign classes and the Bhava bala method.
//! * Jagannatha Hora, through its documented Python port PyJHora 4.8.7
//!   (`jhora/horoscope/chart/strength.py`, `const.py`), for the conventions software
//!   follows: the Sun's Ayana bala doubled, the Moon's Paksha bala doubled, the Moon a
//!   benefic while waxing, the aspects quartered, Mercury's and Jupiter's aspects whole
//!   on a bhava.
//!
//! Choices made where the texts allow more than one, each documented at the function:
//! houses are the chart's own cusps (Placidus or whatever the chart was cast with) for
//! Kendradi bala and Dig bala, with the cusps as the bhava madhyas; the Nathonnata
//! bala reads the Sun's true hour angle rather than clock midnight; the year and month
//! lords count 360-day years and 30-day months from the Kali epoch; the Chesta bala of
//! the five planets grades the eight motional states from the apparent speed against the
//! mean motion ([`chesta`]), the Sun and the Moon taking their Ayana and Paksha balas; a
//! war goes to the graha further north in latitude.

#![forbid(unsafe_code)]

pub mod avastha;
pub mod bhava;
pub mod chesta;
pub mod drik;
pub mod kala;
pub mod relation;
pub mod sthana;
pub mod tables;

pub use avastha::{avasthas, baladi, jagradadi, Avastha, Baladi, Deeptadi, Jagradadi};
pub use bhava::{bhava_balas, bhava_drishti_bala, BhavaBala};
pub use chesta::{chesta_kendra_bala, chesta_of, motion_state, Motion};
pub use drik::{drik_bala, drishti_pinda, Aspector};
pub use kala::{
    ayana_bala, declination_deg, kali_ahargana, nathonnata_bala, paksha_bala, sun_hour_angle_deg,
    time_lords, tribhaga_bala, wars, KalaParts, TimeLords, War,
};
pub use relation::{compound_relation, temporary_relation, Relation};
pub use sthana::{ojhayugma_bala, saptavargaja_bala, sthana, uchcha_bala, SthanaParts};
pub use tables::{
    bhava_dig_bala, graha_name, naisargika, required_virupas, sign_class, sputa_drishti, SignClass,
    RUPA, SEVEN,
};

use kshana_chart::Chart;
use kshana_ephem::{Spk, SpkError};
use kshana_events::{solar_day, EventError};
use kshana_rules::parashari::{dignity, is_benefic};
use kshana_rules::{Facts, Lang};

/// One of three per-language templates; only the chosen arm is formatted.
macro_rules! say3 {
    ($lang:expr, $en:literal, $ml:literal, $hi:literal $(,)?) => {
        match $lang {
            Lang::En => format!($en),
            Lang::Ml => format!($ml),
            Lang::Hi => format!($hi),
        }
    };
}

/// The chart words this crate's text needs, in the three scripts. They match
/// `kshana_rules::yogas::words`; kept here so the crate stands on its own.
pub mod words {
    use kshana_rules::parashari::Dignity;
    use kshana_rules::Lang;
    use kshana_vedic::DashaLord;

    /// A graha's name: "Mars", "ചൊവ്വ", "मंगल".
    pub fn graha(lang: Lang, g: DashaLord) -> &'static str {
        lang.pick(match g {
            DashaLord::Sun => ["Sun", "സൂര്യൻ", "सूर्य"],
            DashaLord::Moon => ["Moon", "ചന്ദ്രൻ", "चंद्र"],
            DashaLord::Mars => ["Mars", "ചൊവ്വ", "मंगल"],
            DashaLord::Mercury => ["Mercury", "ബുധൻ", "बुध"],
            DashaLord::Jupiter => ["Jupiter", "വ്യാഴം", "गुरु"],
            DashaLord::Venus => ["Venus", "ശുക്രൻ", "शुक्र"],
            DashaLord::Saturn => ["Saturn", "ശനി", "शनि"],
            DashaLord::Rahu => ["Rahu", "രാഹു", "राहु"],
            DashaLord::Ketu => ["Ketu", "കേതു", "केतु"],
        })
    }

    const ORD: [[&str; 3]; 12] = [
        ["1st", "ഒന്നാം", "प्रथम"],
        ["2nd", "രണ്ടാം", "द्वितीय"],
        ["3rd", "മൂന്നാം", "तृतीय"],
        ["4th", "നാലാം", "चतुर्थ"],
        ["5th", "അഞ്ചാം", "पंचम"],
        ["6th", "ആറാം", "षष्ठ"],
        ["7th", "ഏഴാം", "सप्तम"],
        ["8th", "എട്ടാം", "अष्टम"],
        ["9th", "ഒമ്പതാം", "नवम"],
        ["10th", "പത്താം", "दशम"],
        ["11th", "പതിനൊന്നാം", "एकादश"],
        ["12th", "പന്ത്രണ്ടാം", "द्वादश"],
    ];

    /// An ordinal, 1-based: "7th", "ഏഴാം", "सप्तम".
    pub fn bhava(lang: Lang, h: u8) -> &'static str {
        lang.pick(ORD[((h.max(1) - 1) % 12) as usize])
    }

    /// A dignity: "Exalted", "ഉച്ചം", "उच्च".
    pub fn dignity(lang: Lang, d: Dignity) -> &'static str {
        lang.pick(match d {
            Dignity::Debilitated => ["Debilitated", "നീചം", "नीच"],
            Dignity::Enemy => ["Enemy", "ശത്രുക്ഷേത്രം", "शत्रु राशि"],
            Dignity::Neutral => ["Neutral", "സമക്ഷേത്രം", "सम राशि"],
            Dignity::Friend => ["Friend", "മിത്രക്ഷേത്രം", "मित्र राशि"],
            Dignity::Own => ["Own", "സ്വക്ഷേത്രം", "स्वराशि"],
            Dignity::Moolatrikona => ["Moolatrikona", "മൂലത്രികോണം", "मूलत्रिकोण"],
            Dignity::Exalted => ["Exalted", "ഉച്ചം", "उच्च"],
        })
    }
}

impl Baladi {
    /// The state's name in the reader's language: "Kumara", "കുമാര", "कुमार".
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Baladi::Bala => ["Bala", "ബാല", "बाल"],
            Baladi::Kumara => ["Kumara", "കുമാര", "कुमार"],
            Baladi::Yuva => ["Yuva", "യുവ", "युवा"],
            Baladi::Vriddha => ["Vriddha", "വൃദ്ധ", "वृद्ध"],
            Baladi::Mrita => ["Mrita", "മൃത", "मृत"],
        })
    }
    /// What the state gives, in one line (BPHS ch. 45).
    pub fn meaning_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Baladi::Bala => ["infant, a quarter of its fruit", "ശിശു, ഫലത്തിന്റെ നാലിലൊന്ന്", "शिशु, फल का चौथाई"],
            Baladi::Kumara => ["youth, half its fruit", "കൗമാരം, പകുതി ഫലം", "किशोर, आधा फल"],
            Baladi::Yuva => ["adult, its full fruit", "യൗവനം, പൂർണ ഫലം", "युवा, पूरा फल"],
            Baladi::Vriddha => ["old, a trace of its fruit", "വാർധക്യം, ഫലം നാമമാത്രം", "वृद्ध, फल नाममात्र"],
            Baladi::Mrita => ["spent, no fruit", "ക്ഷീണിച്ച നില, ഫലമില്ല", "क्षीण, कोई फल नहीं"],
        })
    }
}

impl Jagradadi {
    /// The state's name in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Jagradadi::Jagrat => ["Jagrat", "ജാഗ്രത്", "जाग्रत"],
            Jagradadi::Swapna => ["Swapna", "സ്വപ്ന", "स्वप्न"],
            Jagradadi::Sushupti => ["Sushupti", "സുഷുപ്തി", "सुषुप्ति"],
        })
    }
    /// What the state gives, in one line (BPHS ch. 45).
    pub fn meaning_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Jagradadi::Jagrat => ["awake, full results", "ഉണർന്ന നില, പൂർണ ഫലം", "जागा हुआ, पूरा फल"],
            Jagradadi::Swapna => ["dreaming, middling results", "സ്വപ്നനില, മധ്യമ ഫലം", "स्वप्न में, मध्यम फल"],
            Jagradadi::Sushupti => ["asleep, no results", "ഗാഢനിദ്ര, ഫലമില്ല", "गहरी नींद, कोई फल नहीं"],
        })
    }
}

impl Deeptadi {
    /// The state's name in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Deeptadi::Deepta => ["Deepta", "ദീപ്തം", "दीप्त"],
            Deeptadi::Swastha => ["Swastha", "സ്വസ്ഥം", "स्वस्थ"],
            Deeptadi::Mudita => ["Mudita", "മുദിതം", "मुदित"],
            Deeptadi::Shanta => ["Shanta", "ശാന്തം", "शांत"],
            Deeptadi::Deena => ["Deena", "ദീനം", "दीन"],
            Deeptadi::Dukhita => ["Dukhita", "ദുഃഖിതം", "दुःखित"],
            Deeptadi::Vikala => ["Vikala", "വികലം", "विकल"],
            Deeptadi::Khala => ["Khala", "ഖലം", "खल"],
            Deeptadi::Kopa => ["Kopa", "കോപം", "कोप"],
        })
    }
    /// What the state gives, in one line (BPHS ch. 45).
    pub fn meaning_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Deeptadi::Deepta => ["bright, exalted", "പ്രകാശമാനം, ഉച്ചത്തിൽ", "दीप्तिमान, उच्च में"],
            Deeptadi::Swastha => ["at ease, in its own sign", "സുഖസ്ഥിതി, സ്വക്ഷേത്രത്തിൽ", "सहज, स्वराशि में"],
            Deeptadi::Mudita => ["glad, in a friend's sign", "സന്തുഷ്ടം, മിത്രക്ഷേത്രത്തിൽ", "प्रसन्न, मित्र राशि में"],
            Deeptadi::Shanta => ["calm, in a neutral's sign", "ശാന്തം, സമക്ഷേത്രത്തിൽ", "शांत, सम राशि में"],
            Deeptadi::Deena => ["poor, in an enemy's sign", "ദീനം, ശത്രുക്ഷേത്രത്തിൽ", "दीन, शत्रु राशि में"],
            Deeptadi::Dukhita => ["grieving, debilitated", "ദുഃഖിതം, നീചത്തിൽ", "दुःखी, नीच में"],
            Deeptadi::Vikala => ["maimed, joined by a malefic", "വികലം, പാപഗ്രഹയോഗത്തിൽ", "विकल, पाप ग्रह से युत"],
            Deeptadi::Khala => ["wicked, in a great enemy's sign", "ഖലം, അധിശത്രുക്ഷേത്രത്തിൽ", "खल, अधिशत्रु राशि में"],
            Deeptadi::Kopa => ["angry, combust", "കോപം, മൗഢ്യത്തിൽ", "कुपित, अस्त"],
        })
    }
}
use kshana_sky::Observer;
use kshana_time::EopTable;
use kshana_vedic::{Ayanamsa, DashaLord};

/// Why a Shadbala could not be computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalaError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Sunrise search failure.
    Events(EventError),
    /// No sunrise and sunset frame the instant (polar day or night).
    NoSolarDay,
    /// The chart lacks a body.
    MissingBody(&'static str),
}

impl From<SpkError> for BalaError {
    fn from(e: SpkError) -> Self {
        BalaError::Spk(e)
    }
}
impl From<EventError> for BalaError {
    fn from(e: EventError) -> Self {
        BalaError::Events(e)
    }
}

/// The six-fold strength of one graha.
#[derive(Clone, Debug, PartialEq)]
pub struct GrahaBala {
    /// The graha.
    pub graha: DashaLord,
    /// Sthana bala and its parts.
    pub sthana: SthanaParts,
    /// Dig bala.
    pub dig: f64,
    /// Kala bala and its parts.
    pub kala: KalaParts,
    /// Chesta bala.
    pub chesta: f64,
    /// Motional state behind the Chesta bala; `None` for the Sun and the Moon.
    pub motion: Option<Motion>,
    /// Naisargika bala.
    pub naisargika: f64,
    /// Drik bala (may be negative).
    pub drik: f64,
    /// The six added, virupas.
    pub total_virupas: f64,
    /// The total in rupas.
    pub rupas: f64,
    /// Required minimum, virupas.
    pub required: f64,
    /// Total over required; 1 or more is enough.
    pub ratio: f64,
    /// Ishta phala: √(Uchcha × Chesta), 0–60.
    pub ishta: f64,
    /// Kashta phala: √((60 − Uchcha) × (60 − Chesta)), 0–60.
    pub kashta: f64,
    /// Where the numbers came from, in words.
    pub words: Vec<String>,
}

/// The whole result.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadbala {
    /// The seven grahas, Sun to Saturn.
    pub grahas: Vec<GrahaBala>,
    /// The twelve houses.
    pub bhavas: Vec<BhavaBala>,
    /// Lords of the year, month, day and hora.
    pub lords: TimeLords,
    /// Planetary wars found.
    pub wars: Vec<War>,
    /// Graha with the highest ratio to its requirement.
    pub strongest: DashaLord,
    /// Graha with the lowest ratio to its requirement.
    pub weakest: DashaLord,
}

fn fold(d: f64) -> f64 {
    let d = d.rem_euclid(360.0);
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

/// Dig bala (BPHS 27.7; Raman): the arc from the graha to the cusp where it is
/// strongest ([`tables::dig_strong_cusp`]), folded to at most 180°, taken from 180 and
/// divided by three: 60 on that cusp, 0 on the opposite one.
pub fn dig_bala(g: DashaLord, sidereal: f64, cusps_sidereal: &[f64; 12]) -> f64 {
    let Some(i) = tables::dig_strong_cusp(g) else {
        return 0.0;
    };
    (180.0 - fold(sidereal - cusps_sidereal[i])) / 3.0
}

/// Ishta and Kashta phala (BPHS ch. 27 close; Raman, ch. on Ishta and Kashta): the
/// geometric means of the Uchcha and Chesta balas and of their complements. Chesta is
/// held to 60 first, since the luminaries' doubled stand-ins can exceed it.
pub fn ishta_kashta(uchcha: f64, chesta: f64) -> (f64, f64) {
    let u = uchcha.clamp(0.0, 60.0);
    let c = chesta.clamp(0.0, 60.0);
    ((u * c).sqrt(), ((60.0 - u) * (60.0 - c)).sqrt())
}

/// Compute the Shadbala of a chart cast at `obs`. `ayanamsa` must be the one the chart
/// was cast with; it is used when the ephemeris is stepped back to grade the motion.
pub fn shadbala(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
) -> Result<Shadbala, BalaError> {
    shadbala_in(spk, eop, obs, chart, ayanamsa, Lang::En)
}

/// [`shadbala`] with the `words` in the reader's language: the classical terms in
/// Malayalam or Devanagari script for `ml` and `hi`, the same English for `en`.
pub fn shadbala_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
    lang: Lang,
) -> Result<Shadbala, BalaError> {
    let facts = Facts::from_chart(chart);
    let day = solar_day(spk, eop, obs, chart.instant)?.ok_or(BalaError::NoSolarDay)?;
    let lords = time_lords(&day, obs, chart.instant);
    let hour_angle = sun_hour_angle_deg(chart);
    let elongation = facts.moon_phase_deg;
    let (is_day, third) = kala::tribhaga_of(&day, chart.instant);

    let find = |g: DashaLord| {
        chart
            .bodies
            .iter()
            .find(|b| b.name == graha_name(g))
            .ok_or(BalaError::MissingBody(graha_name(g)))
    };
    let mut placements = Vec::with_capacity(7);
    for g in SEVEN {
        placements.push((g, find(g)?));
    }
    let rasi_signs: Vec<(DashaLord, u8)> = placements
        .iter()
        .map(|(g, p)| (*g, (p.sidereal.rem_euclid(360.0) / 30.0).floor() as u8))
        .collect();
    let aspectors: Vec<Aspector> = placements
        .iter()
        .map(|(g, p)| Aspector {
            graha: *g,
            sidereal: p.sidereal,
            benefic: is_benefic(&facts, *g),
        })
        .collect();
    let positions: Vec<(DashaLord, f64, f64)> = placements
        .iter()
        .map(|(g, p)| (*g, p.sidereal, p.latitude))
        .collect();

    let mut grahas = Vec::with_capacity(7);
    for (g, p) in &placements {
        let g = *g;
        let benefic = is_benefic(&facts, g);
        let sthana = sthana(g, p.sidereal, p.house, &rasi_signs);
        let dig = dig_bala(g, p.sidereal, &chart.cusps_sidereal);
        let decl = declination_deg(p.tropical, p.latitude, chart.obliquity_deg);
        let mut kala = KalaParts {
            nathonnata: nathonnata_bala(g, hour_angle),
            paksha: paksha_bala(g, elongation, benefic),
            tribhaga: tribhaga_bala(g, &day, chart.instant),
            abda: if lords.year == g { 15.0 } else { 0.0 },
            masa: if lords.month == g { 30.0 } else { 0.0 },
            vara: if lords.day == g { 45.0 } else { 0.0 },
            hora: if lords.hora == g { 60.0 } else { 0.0 },
            ayana: ayana_bala(g, decl),
            yuddha: 0.0,
            total: 0.0,
        };
        kala.total = kala.before_war();
        let (motion, chesta) = match g {
            DashaLord::Sun => (None, kala.ayana),
            DashaLord::Moon => (None, kala.paksha),
            _ => chesta_of(spk, g, p.sidereal, p.speed, chart.instant, ayanamsa)?
                .map(|(m, v)| (Some(m), v))
                .unwrap_or((None, 0.0)),
        };
        let naisargika = naisargika(g);
        let drik = drik_bala(p.sidereal, &aspectors, Some(g));
        let d = dignity(g, p.sidereal);
        let words = vec![
            format!(
                "{} at {:.2}° sidereal is {:?}: Uchcha bala {:.1}, Saptavargaja {:.2}, Ojhayugma {:.0}, Kendradi {:.0} (house {}), Drekkana {:.0}",
                graha_name(g),
                p.sidereal,
                d,
                sthana.uchcha,
                sthana.saptavargaja,
                sthana.ojhayugma,
                sthana.kendradi,
                p.house,
                sthana.drekkana
            ),
            format!(
                "Dig bala {:.1}: {:.1}° from the cusp of house {}",
                dig,
                fold(p.sidereal - chart.cusps_sidereal[tables::dig_strong_cusp(g).unwrap_or(0)]),
                tables::dig_strong_cusp(g).unwrap_or(0) + 1
            ),
            format!(
                "Kala bala: Nathonnata {:.1} (Sun's hour angle {:.1}°), Paksha {:.1} ({}, elongation {:.1}°), Tribhaga {:.0} ({} third of the {}), Abda {:.0}, Masa {:.0}, Vara {:.0}, Hora {:.0}, Ayana {:.1} (declination {:+.2}°)",
                kala.nathonnata,
                hour_angle,
                kala.paksha,
                if benefic { "benefic" } else { "malefic" },
                elongation,
                kala.tribhaga,
                ["first", "second", "third"][third],
                if is_day { "day" } else { "night" },
                kala.abda,
                kala.masa,
                kala.vara,
                kala.hora,
                kala.ayana,
                decl
            ),
            match motion {
                Some(m) => format!(
                    "Chesta bala {:.1}: {} ({:+.4}°/day against a mean {:.4})",
                    chesta,
                    m.name(),
                    p.speed,
                    tables::mean_motion(g).unwrap_or(0.0)
                ),
                None => format!(
                    "Chesta bala {:.1}: the {} stands in for it",
                    chesta,
                    if g == DashaLord::Sun { "Ayana bala" } else { "Paksha bala" }
                ),
            },
            format!(
                "Naisargika {:.2}; Drik bala {:+.2} from the aspects received",
                naisargika, drik
            ),
        ];
        let words = if lang == Lang::En {
            words
        } else {
            let name = words::graha(lang, g);
            let lon = p.sidereal;
            let dg = words::dignity(lang, d);
            let (u, sv, oj, ke, dr) = (sthana.uchcha, sthana.saptavargaja, sthana.ojhayugma, sthana.kendradi, sthana.drekkana);
            let h = words::bhava(lang, p.house);
            let dig_cusp = tables::dig_strong_cusp(g).unwrap_or(0);
            let arc = fold(p.sidereal - chart.cusps_sidereal[dig_cusp]);
            let dh = words::bhava(lang, (dig_cusp + 1) as u8);
            let (na, pa, tr, ab, ma, va, ho, ay) =
                (kala.nathonnata, kala.paksha, kala.tribhaga, kala.abda, kala.masa, kala.vara, kala.hora, kala.ayana);
            let ben = if benefic { lang.pick(["", "ശുഭൻ", "शुभ"]) } else { lang.pick(["", "പാപൻ", "पाप"]) };
            let th = words::bhava(lang, third as u8 + 1);
            let dn = if is_day { lang.pick(["", "പകലിന്റെ", "दिन का"]) } else { lang.pick(["", "രാത്രിയുടെ", "रात्रि का"]) };
            let speed = p.speed;
            let mean = tables::mean_motion(g).unwrap_or(0.0);
            vec![
                say3!(
                    lang,
                    "",
                    "{name} നിരയന {lon:.2}° — {dg}: ഉച്ചബലം {u:.1}, സപ്തവർഗജബലം {sv:.2}, ഓജയുഗ്മബലം {oj:.0}, കേന്ദ്രാദിബലം {ke:.0} ({h} ഭാവം), ദ്രേക്കാണബലം {dr:.0}",
                    "{name} निरयन {lon:.2}° पर {dg}: उच्च बल {u:.1}, सप्तवर्गज बल {sv:.2}, ओजयुग्म बल {oj:.0}, केंद्रादि बल {ke:.0} ({h} भाव), द्रेष्काण बल {dr:.0}"
                ),
                say3!(
                    lang,
                    "",
                    "ദിഗ്ബലം {dig:.1}: {dh} ഭാവസ്ഫുടത്തിൽ നിന്ന് {arc:.1}°",
                    "दिग्बल {dig:.1}: {dh} भाव स्पष्ट से {arc:.1}°"
                ),
                say3!(
                    lang,
                    "",
                    "കാലബലം: നതോന്നതബലം {na:.1} (സൂര്യന്റെ ഹോരാകോണം {hour_angle:.1}°), പക്ഷബലം {pa:.1} ({ben}, സൂര്യ-ചന്ദ്ര അന്തരം {elongation:.1}°), ത്രിഭാഗബലം {tr:.0} ({dn} {th} ത്രിഭാഗം), അബ്ദബലം {ab:.0}, മാസബലം {ma:.0}, വാരബലം {va:.0}, ഹോരാബലം {ho:.0}, അയനബലം {ay:.1} (ക്രാന്തി {decl:+.2}°)",
                    "काल बल: नतोन्नत बल {na:.1} (सूर्य का होरा कोण {hour_angle:.1}°), पक्ष बल {pa:.1} ({ben}, सूर्य-चंद्र अंतर {elongation:.1}°), त्रिभाग बल {tr:.0} ({dn} {th} त्रिभाग), अब्द बल {ab:.0}, मास बल {ma:.0}, वार बल {va:.0}, होरा बल {ho:.0}, अयन बल {ay:.1} (क्रांति {decl:+.2}°)"
                ),
                match motion {
                    Some(m) => {
                        let mn = m.name_in(lang);
                        say3!(
                            lang,
                            "",
                            "ചേഷ്ടാബലം {chesta:.1}: {mn} (പ്രതിദിനം {speed:+.4}°, ശരാശരി ഗതി {mean:.4})",
                            "चेष्टा बल {chesta:.1}: {mn} (प्रतिदिन {speed:+.4}°, माध्य गति {mean:.4})"
                        )
                    }
                    None => {
                        let by = if g == DashaLord::Sun { lang.pick(["", "അയനബലം", "अयन बल"]) } else { lang.pick(["", "പക്ഷബലം", "पक्ष बल"]) };
                        say3!(
                            lang,
                            "",
                            "ചേഷ്ടാബലം {chesta:.1}: ഇതിനു പകരം {by} എടുക്കുന്നു",
                            "चेष्टा बल {chesta:.1}: इसके स्थान पर {by} लिया जाता है"
                        )
                    }
                },
                say3!(
                    lang,
                    "",
                    "നൈസർഗികബലം {naisargika:.2}; ലഭിക്കുന്ന ദൃഷ്ടികളിൽ നിന്ന് ദൃക്ബലം {drik:+.2}",
                    "नैसर्गिक बल {naisargika:.2}; प्राप्त दृष्टियों से दृग्बल {drik:+.2}"
                ),
            ]
        };
        grahas.push(GrahaBala {
            graha: g,
            sthana,
            dig,
            kala,
            chesta,
            motion,
            naisargika,
            drik,
            total_virupas: 0.0,
            rupas: 0.0,
            required: required_virupas(g),
            ratio: 0.0,
            ishta: 0.0,
            kashta: 0.0,
            words,
        });
    }

    // Planetary wars, on the strength gathered so far.
    let before = |g: DashaLord| {
        grahas
            .iter()
            .find(|b| b.graha == g)
            .map(|b| b.sthana.total + b.dig + b.kala.before_war())
            .unwrap_or(0.0)
    };
    let wars = wars(&positions, before);
    for w in &wars {
        let (sep, vir) = (w.separation_deg, w.virupas);
        for b in grahas.iter_mut() {
            if b.graha == w.winner {
                b.kala.yuddha += w.virupas;
                let other = words::graha(lang, w.loser);
                b.words.push(match lang {
                    Lang::En => format!(
                        "At war with {} ({:.2}° apart) and victorious: +{:.2}",
                        graha_name(w.loser),
                        w.separation_deg,
                        w.virupas
                    ),
                    _ => say3!(
                        lang,
                        "",
                        "ഗ്രഹയുദ്ധം, എതിരാളി {other} ({sep:.2}° അകലം): ജയം, +{vir:.2}",
                        "{other} से ग्रहयुद्ध ({sep:.2}° की दूरी): विजयी, +{vir:.2}"
                    ),
                });
            } else if b.graha == w.loser {
                b.kala.yuddha -= w.virupas;
                let other = words::graha(lang, w.winner);
                b.words.push(match lang {
                    Lang::En => format!(
                        "At war with {} ({:.2}° apart) and vanquished: −{:.2}",
                        graha_name(w.winner),
                        w.separation_deg,
                        w.virupas
                    ),
                    _ => say3!(
                        lang,
                        "",
                        "ഗ്രഹയുദ്ധം, എതിരാളി {other} ({sep:.2}° അകലം): പരാജയം, −{vir:.2}",
                        "{other} से ग्रहयुद्ध ({sep:.2}° की दूरी): पराजित, −{vir:.2}"
                    ),
                });
            }
        }
    }

    for b in grahas.iter_mut() {
        b.kala.total = b.kala.before_war() + b.kala.yuddha;
        b.total_virupas = b.sthana.total + b.dig + b.kala.total + b.chesta + b.naisargika + b.drik;
        b.rupas = b.total_virupas / RUPA;
        b.ratio = b.total_virupas / b.required;
        let (i, k) = ishta_kashta(b.sthana.uchcha, b.chesta);
        b.ishta = i;
        b.kashta = k;
        let (tv, ru, pc, rq, is, ks) = (b.total_virupas, b.rupas, b.ratio * 100.0, b.required, b.ishta, b.kashta);
        b.words.push(say3!(
            lang,
            "Shadbala {tv:.1} virupas = {ru:.2} rupas, {pc:.0}% of the required {rq:.0}; Ishta {is:.1}, Kashta {ks:.1}",
            "ഷഡ്ബലം {tv:.1} വിരൂപ = {ru:.2} രൂപ, വേണ്ട {rq:.0} വിരൂപയുടെ {pc:.0}%; ഇഷ്ടഫലം {is:.1}, കഷ്ടഫലം {ks:.1}",
            "षड्बल {tv:.1} विरूप = {ru:.2} रूप, आवश्यक {rq:.0} का {pc:.0}%; इष्ट फल {is:.1}, कष्ट फल {ks:.1}"
        ));
    }

    let total_of = |g: DashaLord| {
        grahas
            .iter()
            .find(|b| b.graha == g)
            .map(|b| b.total_virupas)
            .unwrap_or(0.0)
    };
    let bhavas = bhava_balas(&chart.cusps_sidereal, total_of, &aspectors);

    let by_ratio = |pick: fn(f64, f64) -> bool| {
        let mut best = grahas[0].graha;
        let mut r = grahas[0].ratio;
        for b in &grahas[1..] {
            if pick(b.ratio, r) {
                best = b.graha;
                r = b.ratio;
            }
        }
        best
    };
    let strongest = by_ratio(|a, b| a > b);
    let weakest = by_ratio(|a, b| a < b);

    Ok(Shadbala {
        grahas,
        bhavas,
        lords,
        wars,
        strongest,
        weakest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_chart::{compute, ChartInput};
    use kshana_houses::HouseSystem;
    use kshana_time::Utc;

    const KOCHI: Observer = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };

    /// Kochi birth, 1990-10-29 12:06 UTC, Placidus, Lahiri.
    fn fixture() -> Option<(Spk, Chart)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let t = Utc::new(1990, 10, 29, 12, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: t,
                observer: KOCHI,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        Some((spk, ch))
    }

    #[test]
    fn words_in_malayalam_and_hindi_keep_the_numbers() {
        let Some((spk, ch)) = fixture() else { return };
        let en = shadbala(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert_eq!(en, shadbala_in(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri, Lang::En).unwrap());
        let latin = |t: &str| t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.len() >= 3);
        for lang in [Lang::Ml, Lang::Hi] {
            let s = shadbala_in(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri, lang).unwrap();
            for (a, b) in s.grahas.iter().zip(&en.grahas) {
                assert_eq!(a.total_virupas, b.total_virupas);
                assert_eq!(a.words.len(), b.words.len());
                for w in &a.words {
                    assert!(!latin(w), "{lang:?}: {w}");
                }
            }
        }
        for m in [Motion::Vakra, Motion::Anuvakra, Motion::Vikala, Motion::Manda, Motion::Mandatara, Motion::Sama, Motion::Chara, Motion::Atichara] {
            assert_eq!(m.name_in(Lang::En), m.name());
            assert!(!latin(m.name_in(Lang::Ml)) && !latin(m.meaning_in(Lang::Hi)));
        }
        for a in avasthas(&ch) {
            assert_eq!(a.deeptadi.name_in(Lang::En), format!("{:?}", a.deeptadi));
            assert_eq!(a.baladi.name_in(Lang::En), format!("{:?}", a.baladi));
            assert_eq!(a.jagradadi.name_in(Lang::En), format!("{:?}", a.jagradadi));
        }
    }

    #[test]
    fn kochi_shadbala_is_complete_and_consistent() {
        let Some((spk, ch)) = fixture() else { return };
        let s = shadbala(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert_eq!(s.grahas.len(), 7);
        assert_eq!(s.bhavas.len(), 12);
        let facts = Facts::from_chart(&ch);
        for b in &s.grahas {
            let p = ch
                .bodies
                .iter()
                .find(|x| x.name == graha_name(b.graha))
                .unwrap();
            assert!(
                b.total_virupas.is_finite() && b.total_virupas > 0.0,
                "{b:?}"
            );
            assert!((b.rupas - b.total_virupas / 60.0).abs() < 1e-9);
            assert!((b.ratio - b.total_virupas / b.required).abs() < 1e-9);
            assert_eq!(b.required, required_virupas(b.graha));
            // the six add up
            let six = b.sthana.total + b.dig + b.kala.total + b.chesta + b.naisargika + b.drik;
            assert!((six - b.total_virupas).abs() < 1e-9);
            // Naisargika exact
            assert_eq!(b.naisargika, naisargika(b.graha));
            // Kendradi follows the chart's house
            let want = match p.house {
                1 | 4 | 7 | 10 => 60.0,
                2 | 5 | 8 | 11 => 30.0,
                _ => 15.0,
            };
            assert_eq!(
                b.sthana.kendradi, want,
                "{:?} in house {}",
                b.graha, p.house
            );
            // Ojhayugma follows the parity of the sign
            let odd_sign = ((p.sidereal / 30.0).floor() as u32) % 2 == 0;
            let wants_even = matches!(b.graha, DashaLord::Moon | DashaLord::Venus);
            let rasi_part = if odd_sign != wants_even { 15.0 } else { 0.0 };
            assert!(b.sthana.ojhayugma >= rasi_part && b.sthana.ojhayugma <= rasi_part + 15.0);
            // Dig bala is bounded and Ishta/Kashta are within 0–60
            assert!((0.0..=60.0).contains(&b.dig));
            assert!((0.0..=60.0).contains(&b.ishta) && (0.0..=60.0).contains(&b.kashta));
            assert!(b.words.len() >= 6, "{:?}", b.words);
        }
        // Dig bala of the Sun by hand: 180 less the folded arc to the 10th cusp, over 3
        let sun = &s.grahas[0];
        let sun_lon = ch.bodies[0].sidereal;
        let mc = ch.cusps_sidereal[9];
        let mut arc = (sun_lon - mc).rem_euclid(360.0);
        if arc > 180.0 {
            arc = 360.0 - arc;
        }
        assert!(
            (sun.dig - (180.0 - arc) / 3.0).abs() < 1e-9,
            "{} vs {}",
            sun.dig,
            (180.0 - arc) / 3.0
        );
        // the Sun's Chesta is its Ayana bala, the Moon's its Paksha bala, doubled
        assert_eq!(sun.chesta, sun.kala.ayana);
        let moon = &s.grahas[1];
        assert_eq!(moon.chesta, moon.kala.paksha);
        let e = facts.moon_phase_deg;
        let e = if e > 180.0 { 360.0 - e } else { e };
        let single = if is_benefic(&facts, DashaLord::Moon) {
            e / 3.0
        } else {
            60.0 - e / 3.0
        };
        assert!((moon.kala.paksha - 2.0 * single).abs() < 1e-9);
        // Jupiter's Tribhaga is always 60; Mercury's Nathonnata always 60
        assert_eq!(s.grahas[4].kala.tribhaga, 60.0);
        assert_eq!(s.grahas[3].kala.nathonnata, 60.0);
        // exactly one graha carries each of the four time lords
        type Pick = fn(&KalaParts) -> f64;
        let picks: [(f64, Pick); 4] = [
            (15.0, |k| k.abda),
            (30.0, |k| k.masa),
            (45.0, |k| k.vara),
            (60.0, |k| k.hora),
        ];
        for (want, pick) in picks {
            let sum: f64 = s.grahas.iter().map(|b| pick(&b.kala)).sum();
            assert_eq!(sum, want);
        }
        // Bhava balas: lord bala is that lord's total; dig within 0–60
        for h in &s.bhavas {
            let lord_total = s
                .grahas
                .iter()
                .find(|b| b.graha == h.lord)
                .unwrap()
                .total_virupas;
            assert_eq!(h.lord_bala, lord_total);
            assert!((0.0..=60.0).contains(&h.dig));
            assert!((h.total - (h.lord_bala + h.dig + h.drishti)).abs() < 1e-9);
        }
        assert_eq!(
            s.bhavas.iter().map(|h| h.house).collect::<Vec<_>>(),
            (1..=12).collect::<Vec<_>>()
        );
        // strongest and weakest by ratio
        let max = s.grahas.iter().map(|b| b.ratio).fold(f64::MIN, f64::max);
        let min = s.grahas.iter().map(|b| b.ratio).fold(f64::MAX, f64::min);
        assert_eq!(
            s.grahas
                .iter()
                .find(|b| b.graha == s.strongest)
                .unwrap()
                .ratio,
            max
        );
        assert_eq!(
            s.grahas
                .iter()
                .find(|b| b.graha == s.weakest)
                .unwrap()
                .ratio,
            min
        );
    }

    #[test]
    fn time_lords_agree_with_the_sunrise_weekday() {
        let Some((spk, ch)) = fixture() else { return };
        let day = solar_day(&spk, None, &KOCHI, ch.instant).unwrap().unwrap();
        let lords = time_lords(&day, &KOCHI, ch.instant);
        // 29 October 1990 was a Monday in Kochi (sunrise about 00:50 UTC)
        assert_eq!(lords.day, DashaLord::Moon);
        // the Kali ahargana's weekday says the same
        let wd = (5 + lords.kali_ahargana).rem_euclid(7) as u8;
        assert_eq!(tables::weekday_lord(wd), DashaLord::Moon);
        // JDN of 1990-10-29 is 2448194; Kali ahargana = JDN − 588466
        assert_eq!(lords.kali_ahargana, 2_448_194 - 588_466);
        // year lord: Friday + 3 × ⌊A/360⌋; month lord: Friday + 2 × ⌊A/30⌋
        let a = lords.kali_ahargana;
        assert_eq!(
            lords.year,
            tables::weekday_lord(((5 + 3 * (a / 360)) % 7) as u8)
        );
        assert_eq!(
            lords.month,
            tables::weekday_lord(((5 + 2 * (a / 30)) % 7) as u8)
        );
    }

    #[test]
    fn kochi_avasthas_are_valid() {
        let Some((_, ch)) = fixture() else { return };
        let a = avasthas(&ch);
        assert_eq!(a.len(), 7);
        for (i, g) in SEVEN.iter().enumerate() {
            assert_eq!(a[i].graha, *g);
            let p = ch.bodies.iter().find(|b| b.name == graha_name(*g)).unwrap();
            assert_eq!(a[i].baladi, baladi(p.sidereal));
            assert_eq!(a[i].jagradadi, jagradadi(dignity(*g, p.sidereal)));
        }
    }

    #[test]
    fn ishta_and_kashta_are_geometric_means() {
        let (i, k) = ishta_kashta(60.0, 60.0);
        assert_eq!((i, k), (60.0, 0.0));
        let (i, k) = ishta_kashta(0.0, 0.0);
        assert_eq!((i, k), (0.0, 60.0));
        let (i, k) = ishta_kashta(30.0, 120.0);
        assert!((i - (30.0f64 * 60.0).sqrt()).abs() < 1e-12);
        assert_eq!(k, 0.0);
    }

    #[test]
    fn dig_bala_by_strong_cusp() {
        let cusps: [f64; 12] = std::array::from_fn(|i| 30.0 * i as f64);
        assert_eq!(dig_bala(DashaLord::Sun, 270.0, &cusps), 60.0); // on the 10th cusp
        assert_eq!(dig_bala(DashaLord::Sun, 90.0, &cusps), 0.0);
        assert_eq!(dig_bala(DashaLord::Saturn, 180.0, &cusps), 60.0);
        assert_eq!(dig_bala(DashaLord::Jupiter, 0.0, &cusps), 60.0);
        assert_eq!(dig_bala(DashaLord::Moon, 90.0, &cusps), 60.0);
        assert_eq!(dig_bala(DashaLord::Venus, 0.0, &cusps), 30.0);
        assert_eq!(dig_bala(DashaLord::Rahu, 0.0, &cusps), 0.0);
    }
}
