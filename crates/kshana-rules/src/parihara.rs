//! Parihara: remedies tied to a located cause, a karma class, a classical source and an
//! exact timing window.
//!
//! The cause is located in the chart and the period: the weakest of the running period
//! lords (maha, antar, pratyantar), or the lord of the area's house. Before any remedy
//! is shown the karma gate ([`crate::karma`]) decides what a remedy may claim, and the
//! dosha detectors ([`crate::doshas`]) say which curse, badhaka or yoga family the
//! chart carries. Each remedy then carries its rung on the ladder, its source, what it
//! may claim for this karma class, a schedule with the next exact windows from
//! [`crate::remedy_timing`], and the one thing the app records for it.
//!
//! The ladder (top engages the most and needs no one else): 1 karma (conduct: change
//! the act the house signifies, Prasna Marga's purushakara), 2 seva, 3 vrata and
//! discipline, 4 daana, 5 japa, 6 pooja (needs a priest; optional), 7 gems (purchase;
//! information only, gated by functional-benefic status, never recommended). Conduct,
//! service and discipline are always ranked above donation, mantra and pooja, and the
//! ritual items are marked optional: nothing in this module requires a purchase or a
//! priest. Efficacy is always the text's claim, never a number.

use std::collections::HashMap;

use crate::areas::Area;
use crate::doshas::{doshas, Dosha};
use crate::facts::Facts;
use crate::karma::{classify, may_claim, KarmaVerdict, TransitSigns};
use crate::parashari::{dignity, period_quality, Dignity};
use crate::remedy_timing::{fmt_utc, weekday_of, RemedyWindow, TimingTable, WEEKDAY_NAMES};
use kshana_chart::Chart;
use kshana_ephem::{Body, Spk};
use kshana_events::EventError;
use kshana_sky::{apparent, true_node_longitude, Observer};
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::{
    ayanamsa_degrees, dasha::YEAR_365_25_NS, nakshatra, sidereal, vimshottari_levels, Ayanamsa,
    DashaLord, Tithi, NAKSHATRA_SPAN,
};

/// What kind of remedy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum RemedyKind {
    Karma,
    Seva,
    Discipline,
    Daana,
    Vrata,
    Mantra,
    Pooja,
    /// Information only: never recommended, listed so nothing is hidden.
    Gem,
}

impl RemedyKind {
    /// Rung on the ladder: 1 karma, 2 seva, 3 vrata and discipline, 4 daana, 5 mantra,
    /// 6 pooja, 7 gem.
    pub fn rung(self) -> u8 {
        match self {
            RemedyKind::Karma => 1,
            RemedyKind::Seva => 2,
            RemedyKind::Discipline | RemedyKind::Vrata => 3,
            RemedyKind::Daana => 4,
            RemedyKind::Mantra => 5,
            RemedyKind::Pooja => 6,
            RemedyKind::Gem => 7,
        }
    }

    /// Base fit before the cause's severity is applied.
    fn base_fit(self) -> f64 {
        match self {
            RemedyKind::Karma => 0.95,
            RemedyKind::Seva | RemedyKind::Discipline => 0.85,
            RemedyKind::Vrata => 0.75,
            RemedyKind::Daana => 0.65,
            RemedyKind::Mantra => 0.45,
            RemedyKind::Pooja => 0.3,
            RemedyKind::Gem => 0.0,
        }
    }

    /// True for the rungs that are supportive, never required.
    pub fn is_optional(self) -> bool {
        matches!(
            self,
            RemedyKind::Mantra | RemedyKind::Pooja | RemedyKind::Gem
        )
    }
}

/// When and how often, with the next exact windows.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    /// `"daily"`, `"weekly on <day>"`, `"once"`, `"40 days"`, `"43 days"`, or a tithi cadence.
    pub cadence: String,
    /// A count to log against: mantra count (the Kali multiplier is noted in `why`),
    /// weeks of a vrata, days of a vow.
    pub count: Option<u32>,
    /// Length of the vow in days, when it has one.
    pub duration_days: Option<u32>,
    /// The next three exact windows the remedy is timed to.
    pub windows: Vec<RemedyWindow>,
}

/// One remedy.
#[derive(Clone, Debug, PartialEq)]
pub struct Remedy {
    /// Kind.
    pub kind: RemedyKind,
    /// Rung on the ladder, 1–7.
    pub rung: u8,
    /// Stable rule id.
    pub rule_id: String,
    /// Short title.
    pub title: String,
    /// Why this, in one or two sentences, with the classical column where there is one.
    pub why: String,
    /// The text behind it.
    pub source: &'static str,
    /// Fit score in `[0, 1]`.
    pub fit: f64,
    /// Effort, in words.
    pub effort: &'static str,
    /// True for mantra, pooja and gem: supportive, never required.
    pub optional: bool,
    /// What the texts let this rung claim for the cause's karma class.
    pub may_claim: &'static str,
    /// When, how often, and the next exact windows.
    pub schedule: Schedule,
    /// What the app records: "did it at <instant>" and the count.
    pub measurable: String,
}

/// The located cause.
#[derive(Clone, Debug, PartialEq)]
pub struct Cause {
    /// The graha.
    pub lord: DashaLord,
    /// The house whose duty the remedy concerns (the area's house, or the lord's own).
    pub house: u8,
    /// Rasi house the lord occupies.
    pub placed_in: u8,
    /// Its dignity.
    pub dignity: Dignity,
    /// `"mahadasha"`, `"antardasha"` or `"pratyantar"` when the lord runs a period now.
    pub running_level: Option<&'static str>,
    /// Exact start of that period.
    pub running_since: Option<Instant>,
    /// Exact end of that period.
    pub running_until: Option<Instant>,
    /// Severity in `[0, 1]`: 0 means nothing needs remedy.
    pub severity: f64,
    /// The karma gate's verdict.
    pub karma: KarmaVerdict,
    /// Dosha families present in the chart, most severe first.
    pub doshas: Vec<Dosha>,
    /// The cause in words, from the house-duty table.
    pub words: String,
}

/// The remedy plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Parihara {
    /// The graha the plan addresses.
    pub planet: DashaLord,
    /// Severity in `[0, 1]`: 0 means nothing needs remedy.
    pub severity: f64,
    /// The located cause.
    pub cause: Cause,
    /// Remedies, karma first, then seva and discipline, daana, vrata, mantra, pooja;
    /// gems last as information.
    pub remedies: Vec<Remedy>,
    /// How the ledger records the plan.
    pub ledger_hint: String,
}

// ---------------------------------------------------------------------------------
// Static tables, each with its citation.
// ---------------------------------------------------------------------------------

/// One row of the graha-shanti baseline: Yajnavalkya Smriti, Acharadhyaya 1.295–309
/// and Matsya Purana ch. 93 (Navagraha shanti). Image metal, colour, samidha, the
/// text's daana item and the naivedya, beside the practical item temple and Lal Kitab
/// usage gives. The texts add meat-and-rice for some grahas (1.304–305); not listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrahaShanti {
    /// The graha.
    pub planet: DashaLord,
    /// Image material.
    pub image: &'static str,
    /// Colour.
    pub colour: &'static str,
    /// Samidha (fuel wood).
    pub samidha: &'static str,
    /// The text's daana item.
    pub daana: &'static str,
    /// Naivedya.
    pub naivedya: &'static str,
    /// The practical item used today.
    pub practical: &'static str,
}

/// Source line for [`GRAHA_SHANTI`].
pub const GRAHA_SHANTI_SOURCE: &str = "Yajnavalkya Smriti 1.295–309 (Grahashanti, after the Vinayaka-kalpa); Matsya Purana ch. 93 (Navagraha shanti); V where the snippet confirmed, otherwise the standard reading of 1.297–305";

/// The graha-shanti table.
pub const GRAHA_SHANTI: [GrahaShanti; 9] = [
    GrahaShanti {
        planet: DashaLord::Sun,
        image: "copper",
        colour: "red",
        samidha: "arka (madaara) (V)",
        daana: "a kapila cow (V)",
        naivedya: "rice with jaggery",
        practical: "wheat or jaggery",
    },
    GrahaShanti {
        planet: DashaLord::Moon,
        image: "crystal (sphatika)",
        colour: "white",
        samidha: "palasha (V)",
        daana: "a conch (V)",
        naivedya: "rice with ghee and milk",
        practical: "rice, milk or white cloth",
    },
    GrahaShanti {
        planet: DashaLord::Mars,
        image: "red sandal",
        colour: "red",
        samidha: "khadira (V)",
        daana: "a bull (V)",
        naivedya: "rice with red gram",
        practical: "red lentils",
    },
    GrahaShanti {
        planet: DashaLord::Mercury,
        image: "gold",
        colour: "green",
        samidha: "apamarga (verify: the snippet's 'chichinda' is a slip)",
        daana: "gold (V)",
        naivedya: "rice with milk",
        practical: "green moong",
    },
    GrahaShanti {
        planet: DashaLord::Jupiter,
        image: "gold",
        colour: "yellow",
        samidha: "pippala / ashvattha (V)",
        daana: "yellow cloth (V; gold in some lists)",
        naivedya: "rice with curd",
        practical: "turmeric or chana dal",
    },
    GrahaShanti {
        planet: DashaLord::Venus,
        image: "silver",
        colour: "white",
        samidha: "udumbara (V)",
        daana: "a white horse (V)",
        naivedya: "rice with ghee",
        practical: "curd or white sweets",
    },
    GrahaShanti {
        planet: DashaLord::Saturn,
        image: "iron",
        colour: "black",
        samidha: "shami (standard)",
        daana: "a black cow (V)",
        naivedya: "rice with sesame",
        practical: "black til or a blanket",
    },
    GrahaShanti {
        planet: DashaLord::Rahu,
        image: "lead",
        colour: "blue-black",
        samidha: "durva (V)",
        daana: "a metallic object (V; iron in practice)",
        naivedya: "rice with urad",
        practical: "mustard oil or blue cloth",
    },
    GrahaShanti {
        planet: DashaLord::Ketu,
        image: "bell-metal (kamsya) (V)",
        colour: "variegated, smoke",
        samidha: "kusha (V)",
        daana: "a goat (V)",
        naivedya: "rice",
        practical: "a blanket or sesame",
    },
];

/// The graha-shanti row of a graha.
pub fn graha_shanti(g: DashaLord) -> &'static GrahaShanti {
    GRAHA_SHANTI
        .iter()
        .find(|r| r.planet == g)
        .expect("nine rows")
}

/// The BPHS antardasha remedy formula of a period lord (Santhanam vol. 2, chs. 51–60,
/// each chapter ending with "remedial measures for relief from their evil effects").
/// Snippet-verified: Saturn-dasha bhuktis → Mrityunjaya japa and a goat, Durga
/// Saptashati and a cow or female buffalo; Ketu → Mrityunjaya and Shiva Sahasranama;
/// Jupiter unfavourable → Shiva Sahasranama and gold. The rest as the brief recalls
/// the text; verify each against the scan. The text prescribes them when the
/// antardasha lord is in 6, 8 or 12 from the lagna or the dasha lord, debilitated,
/// combust or an enemy of the dasha lord: the condition `period_quality` scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashaFormula {
    /// The period lord.
    pub planet: DashaLord,
    /// The text's japa or worship.
    pub japa: &'static str,
    /// The text's gift.
    pub daana: &'static str,
    /// Conduct equivalent of the japa, so nothing needs a priest.
    pub japa_conduct: &'static str,
    /// Conduct equivalent of the gift.
    pub daana_conduct: &'static str,
}

/// Source line for [`DASHA_FORMULAE`].
pub const DASHA_FORMULAE_SOURCE: &str = "BPHS chs. 51–60 (Santhanam vol. 2), the remedial measures closing each antardasha chapter; verify each pairing against the scan";

/// The antardasha remedy formulae.
pub const DASHA_FORMULAE: [DashaFormula; 9] = [
    DashaFormula {
        planet: DashaLord::Sun,
        japa: "Surya puja, Aditya worship",
        daana: "a cow (a red cow in some bhuktis)",
        japa_conduct: "arghya to the Sun at the computed sunrise, logged daily",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Moon,
        japa: "Durga Saptashati (Devi worship)",
        daana: "a white cow or a female buffalo",
        japa_conduct: "Friday reading, 13 chapters over 13 days",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Mars,
        japa: "Mrityunjaya japa",
        daana: "a bull",
        japa_conduct: "108 recitations daily with the count logged",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Mercury,
        japa: "Vishnu Sahasranama",
        daana: "silver, or a horse",
        japa_conduct: "one reading daily, logged",
        daana_conduct: "fund one learner's books or fees",
    },
    DashaFormula {
        planet: DashaLord::Jupiter,
        japa: "Shiva Sahasranama, Rudra japa (V)",
        daana: "gold and yellow cloth (V)",
        japa_conduct: "one reading daily, logged",
        daana_conduct: "feed students, or fund one student",
    },
    DashaFormula {
        planet: DashaLord::Venus,
        japa: "Durga or Lakshmi worship",
        daana: "a cow or a buffalo",
        japa_conduct: "Friday worship, logged",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Saturn,
        japa: "Mrityunjaya japa (V); Durga Saptashati in another bhukti (V)",
        daana: "a black cow or buffalo; a goat in one bhukti (V)",
        japa_conduct: "108 recitations daily with the count logged",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Rahu,
        japa: "Durga japa, Mrityunjaya",
        daana: "a goat",
        japa_conduct: "108 recitations daily with the count logged",
        daana_conduct: "feed or fund animals for 43 days",
    },
    DashaFormula {
        planet: DashaLord::Ketu,
        japa: "Mrityunjaya japa, Shiva Sahasranama (V)",
        daana: "a goat",
        japa_conduct: "108 recitations daily with the count logged",
        daana_conduct: "feed or fund animals for 43 days",
    },
];

/// The formula of a period lord.
pub fn dasha_formula(g: DashaLord) -> &'static DashaFormula {
    DASHA_FORMULAE
        .iter()
        .find(|r| r.planet == g)
        .expect("nine rows")
}

/// Mantra and japa per graha: the count used in temple practice (commercial services
/// cite Mantra Mahodadhi; no Parashara or Yajnavalkya verse found; snippet-verified
/// through Saturn), the Tirupati alternate where it differs, the beeja form, the Vedic
/// rk and the hymn BPHS names. Kali Yuga rule: multiply by four (practice, unsourced
/// scripturally). Purascharana ladder: homa N/10, tarpana N/100, marjana N/1000,
/// brahmana bhojana N/10000.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MantraDose {
    /// The graha.
    pub planet: DashaLord,
    /// Count used in temple practice.
    pub count: u32,
    /// The Tirupati list's count where it differs.
    pub alternate: Option<u32>,
    /// Tantric beeja form.
    pub beeja: &'static str,
    /// Vedic graha rk (first words).
    pub vedic: &'static str,
    /// Hymn.
    pub hymn: &'static str,
}

/// The Kali Yuga multiplier (practice, unsourced scripturally).
pub const KALI_MULTIPLIER: u32 = 4;

/// Source line for [`MANTRA_DOSES`].
pub const MANTRA_DOSES_SOURCE: &str = "Temple practice counts (Mantra Mahodadhi cited by services; Tirupati list as the alternate); BPHS dasha chapters for the hymns; the ×4 Kali rule is practice";

/// The mantra table.
pub const MANTRA_DOSES: [MantraDose; 9] = [
    MantraDose {
        planet: DashaLord::Sun,
        count: 7_000,
        alternate: Some(6_000),
        beeja: "Om hram hrim hraum sah Suryaya namah",
        vedic: "A krishnena rajasa",
        hymn: "Aditya Hridaya",
    },
    MantraDose {
        planet: DashaLord::Moon,
        count: 11_000,
        alternate: Some(10_000),
        beeja: "Om shram shrim shraum sah Chandraya namah",
        vedic: "Imam deva asapatnam",
        hymn: "Shiva stotra; Durga Saptashati (BPHS, Moon bhuktis)",
    },
    MantraDose {
        planet: DashaLord::Mars,
        count: 10_000,
        alternate: Some(7_000),
        beeja: "Om kram krim kraum sah Bhaumaya namah",
        vedic: "Agnir murdha",
        hymn: "Hanuman Chalisa; Mrityunjaya (BPHS)",
    },
    MantraDose {
        planet: DashaLord::Mercury,
        count: 9_000,
        alternate: Some(17_000),
        beeja: "Om bram brim braum sah Budhaya namah",
        vedic: "Udbudhyasva",
        hymn: "Vishnu Sahasranama (BPHS)",
    },
    MantraDose {
        planet: DashaLord::Jupiter,
        count: 19_000,
        alternate: None,
        beeja: "Om gram grim graum sah Gurave namah",
        vedic: "Brihaspate ati yad aryo",
        hymn: "Shiva Sahasranama, Rudra (BPHS)",
    },
    MantraDose {
        planet: DashaLord::Venus,
        count: 16_000,
        alternate: None,
        beeja: "Om dram drim draum sah Shukraya namah",
        vedic: "Annat parisruto",
        hymn: "Lakshmi stotra; Durga Saptashati (BPHS)",
    },
    MantraDose {
        planet: DashaLord::Saturn,
        count: 23_000,
        alternate: None,
        beeja: "Om pram prim praum sah Shanaishcharaya namah",
        vedic: "Sham no devir",
        hymn: "Hanuman Chalisa; Mrityunjaya, Dasharatha Shani Stotra",
    },
    MantraDose {
        planet: DashaLord::Rahu,
        count: 18_000,
        alternate: None,
        beeja: "Om bhram bhrim bhraum sah Rahave namah",
        vedic: "Kaya nas chitra",
        hymn: "Durga; Sarpa Sukta",
    },
    MantraDose {
        planet: DashaLord::Ketu,
        count: 17_000,
        alternate: None,
        beeja: "Om sram srim sraum sah Ketave namah",
        vedic: "Ketum krinvan",
        hymn: "Ganesha; Mrityunjaya, Shiva Sahasranama (BPHS)",
    },
];

/// The mantra row of a graha.
pub fn mantra_dose(g: DashaLord) -> &'static MantraDose {
    MANTRA_DOSES
        .iter()
        .find(|r| r.planet == g)
        .expect("nine rows")
}

/// Weekday vratas as practised across India (practice-level, not a single verse), with
/// the traditional counts. Shown as a vow of simplicity for the day, never as treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VrataRow {
    /// The graha.
    pub planet: DashaLord,
    /// Name.
    pub name: &'static str,
    /// Count of weekdays, when the tradition gives one.
    pub count: Option<u32>,
    /// An alternate count some traditions give.
    pub alternate: Option<u32>,
    /// The rule of the day.
    pub rule: &'static str,
}

/// Source line for [`VRATAS`].
pub const VRATAS_SOURCE: &str = "Weekday vratas as practised across India (practice; the counts are the common ones, no single classical verse); Chandrayana per BPHS ch. 85 for the Brahmin's curse";

/// The vrata table.
pub const VRATAS: [VrataRow; 9] = [
    VrataRow {
        planet: DashaLord::Sun,
        name: "Ravivara vrata",
        count: Some(12),
        alternate: Some(30),
        rule: "one meal, no salt after the computed sunset",
    },
    VrataRow {
        planet: DashaLord::Moon,
        name: "Somavara vrata (Solah Somvar)",
        count: Some(16),
        alternate: None,
        rule: "Shiva's Monday; white food",
    },
    VrataRow {
        planet: DashaLord::Mars,
        name: "Mangalavara vrata (Hanuman)",
        count: Some(21),
        alternate: None,
        rule: "red; no salt",
    },
    VrataRow {
        planet: DashaLord::Mercury,
        name: "Budhavara vrata",
        count: Some(21),
        alternate: Some(45),
        rule: "green moong",
    },
    VrataRow {
        planet: DashaLord::Jupiter,
        name: "Guruvara vrata",
        count: Some(16),
        alternate: None,
        rule: "yellow food; Brihaspati katha",
    },
    VrataRow {
        planet: DashaLord::Venus,
        name: "Shukravara vrata (Lakshmi, Santoshi)",
        count: Some(16),
        alternate: None,
        rule: "white food; nothing sour",
    },
    VrataRow {
        planet: DashaLord::Saturn,
        name: "Shanivara vrata",
        count: Some(7),
        alternate: Some(19),
        rule: "black til; an oil lamp under a Peepal at the computed sunset",
    },
    VrataRow {
        planet: DashaLord::Rahu,
        name: "Saturday fast with a Durga lamp in Rahu kala (Kerala), or Amavasya",
        count: None,
        alternate: None,
        rule: "no classical count; 43 days by the Lal Kitab convention",
    },
    VrataRow {
        planet: DashaLord::Ketu,
        name: "Chaturthi (Sankashti) fast",
        count: None,
        alternate: None,
        rule: "broken at the computed moonrise; no classical count",
    },
];

/// The vrata row of a graha.
pub fn vrata_row(g: DashaLord) -> &'static VrataRow {
    VRATAS.iter().find(|r| r.planet == g).expect("nine rows")
}

/// Phaladeepika's gem of each planet (V), with the common metal, finger and day (sources
/// vary). The chart rules are modern: a gem strengthens its planet, so it is shown only
/// for a functional benefic that is weak, never for the lords of 6, 8 and 12.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GemRow {
    /// The graha.
    pub planet: DashaLord,
    /// The stone.
    pub gem: &'static str,
    /// Metal.
    pub metal: &'static str,
    /// Finger.
    pub finger: &'static str,
}

/// Source line for [`GEMS`].
pub const GEMS_SOURCE: &str = "Phaladeepika (gem per planet, V); metal, finger and day are common practice and sources vary; the functional-benefic gate is modern (Raman, KP, Jataka Tattva), rule KS-R-GEM-01";

/// The gem table.
pub const GEMS: [GemRow; 9] = [
    GemRow {
        planet: DashaLord::Sun,
        gem: "ruby",
        metal: "gold or copper",
        finger: "ring finger",
    },
    GemRow {
        planet: DashaLord::Moon,
        gem: "spotless pearl",
        metal: "silver",
        finger: "little finger",
    },
    GemRow {
        planet: DashaLord::Mars,
        gem: "coral",
        metal: "gold or copper",
        finger: "ring finger",
    },
    GemRow {
        planet: DashaLord::Mercury,
        gem: "emerald",
        metal: "gold",
        finger: "little finger",
    },
    GemRow {
        planet: DashaLord::Jupiter,
        gem: "yellow sapphire",
        metal: "gold",
        finger: "index finger",
    },
    GemRow {
        planet: DashaLord::Venus,
        gem: "diamond",
        metal: "silver or platinum",
        finger: "middle or ring finger",
    },
    GemRow {
        planet: DashaLord::Saturn,
        gem: "blue sapphire (the most reactive; test first)",
        metal: "silver, iron or panchadhatu",
        finger: "middle finger",
    },
    GemRow {
        planet: DashaLord::Rahu,
        gem: "hessonite (gomedha)",
        metal: "silver",
        finger: "middle finger",
    },
    GemRow {
        planet: DashaLord::Ketu,
        gem: "cat's eye",
        metal: "silver",
        finger: "ring finger",
    },
];

/// The gem row of a graha.
pub fn gem_row(g: DashaLord) -> &'static GemRow {
    GEMS.iter().find(|r| r.planet == g).expect("nine rows")
}

/// The duty of a house (BPHS Bhavadhyaya and Karakadhyaya read through the
/// dharma-shastra): domain, karaka, the conduct that is the remedy, whom to serve, and
/// the daana family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseDuty {
    /// House 1–12.
    pub house: u8,
    /// Domain of duty.
    pub domain: &'static str,
    /// Karaka(s).
    pub karaka: &'static str,
    /// The conduct remedy.
    pub conduct: &'static str,
    /// Seva target.
    pub seva: &'static str,
    /// Daana family.
    pub daana: &'static str,
}

/// Source line for [`HOUSE_DUTIES`].
pub const HOUSE_DUTIES_SOURCE: &str = "BPHS Bhavadhyaya and Karakadhyaya; the dharma-shastra reading of houses and karakas the conduct items already use";

/// The house-duty table.
pub const HOUSE_DUTIES: [HouseDuty; 12] = [
    HouseDuty {
        house: 1,
        domain: "the self and the body",
        karaka: "Sun",
        conduct: "keep your word; rise with the Sun",
        seva: "no one else: the self kept in order",
        daana: "wheat or jaggery",
    },
    HouseDuty {
        house: 2,
        domain: "speech, food and the family's wealth",
        karaka: "Jupiter, Venus",
        conduct: "truthful speech; feed the family",
        seva: "feed students",
        daana: "chana or turmeric; curd or sweets",
    },
    HouseDuty {
        house: 3,
        domain: "siblings, courage and effort",
        karaka: "Mars",
        conduct: "end a quarrel with a sibling; finish the hard task",
        seva: "those who protect others; blood donation",
        daana: "red lentils",
    },
    HouseDuty {
        house: 4,
        domain: "the mother, home and land",
        karaka: "Moon",
        conduct: "tend the mother; steady sleep",
        seva: "women and children without support",
        daana: "rice, milk or white cloth",
    },
    HouseDuty {
        house: 5,
        domain: "children, learning and past merit",
        karaka: "Jupiter",
        conduct: "teach; honour the teacher",
        seva: "students",
        daana: "chana or turmeric",
    },
    HouseDuty {
        house: 6,
        domain: "debts, enemies, service and illness",
        karaka: "Mars, Saturn",
        conduct: "repay one debt; serve the sick",
        seva: "the sick without family",
        daana: "black til",
    },
    HouseDuty {
        house: 7,
        domain: "the spouse and partners",
        karaka: "Venus",
        conduct: "keep a promise in the relationship",
        seva: "women, artists",
        daana: "curd or white sweets",
    },
    HouseDuty {
        house: 8,
        domain: "longevity, the hidden and inheritance",
        karaka: "Saturn",
        conduct: "complete the rites of the dead; visit a grave",
        seva: "the old",
        daana: "a blanket or sesame",
    },
    HouseDuty {
        house: 9,
        domain: "the father, dharma and the guru",
        karaka: "Sun, Jupiter",
        conduct: "serve the father; keep the family rites",
        seva: "elders without family",
        daana: "a cow (classical)",
    },
    HouseDuty {
        house: 10,
        domain: "work and duty to society",
        karaka: "Saturn, Sun, Mercury, Jupiter",
        conduct: "finish the postponed duty",
        seva: "labourers",
        daana: "iron or a blanket",
    },
    HouseDuty {
        house: 11,
        domain: "gains and elders",
        karaka: "Jupiter",
        conduct: "share a gain",
        seva: "the needy elder",
        daana: "chana",
    },
    HouseDuty {
        house: 12,
        domain: "loss, foreign lands, sleep and moksha",
        karaka: "Saturn, Ketu",
        conduct: "simplify one indulgence; keep silence",
        seva: "renunciates, stray animals",
        daana: "a blanket or sesame",
    },
];

/// The duty of a house.
pub fn house_duty(house: u8) -> &'static HouseDuty {
    &HOUSE_DUTIES[((house as usize).clamp(1, 12)) - 1]
}

/// The six base rows per graha (karma, seva, discipline, daana, mantra, pooja), kept
/// from the first version: Lal Kitab and temple practice for the ritual items, dharma
/// texts for the conduct items.
fn table(g: DashaLord) -> Vec<(RemedyKind, &'static str, &'static str, &'static str)> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Karma, "Keep your word to those who depend on you", "The Sun is authority earned by reliability. Do what you said, when you said.", "daily"),
            (Seva, "Serve your father, or an elder man without family", "The Sun's karaka is the father; service to him is the direct remedy.", "weekly"),
            (Discipline, "Be up before sunrise for 40 days", "The Sun is strengthened by meeting it; the sunrise time is computed for your place.", "40 days"),
            (Daana, "Give wheat or jaggery on a Sunday, by hand", "Sun's grains, given in Surya hora, to someone who needs them.", "once"),
            (Mantra, "Aditya Hridayam on Sundays", "Supports the discipline above; not a substitute for it.", "if you wish"),
            (Pooja, "Surya arghya at sunrise", "Optional. Listed so you know it is not hidden.", "not required"),
        ],
        DashaLord::Moon => vec![
            (Karma, "Care for your mother, in person, this week", "The Moon is the mother and the mind; the remedy for a weak Moon is to tend to both.", "weekly"),
            (Seva, "Serve women and children who have no one", "The Moon's people. An hour of help, not money alone.", "weekly"),
            (Discipline, "Fixed sleep and waking times for 40 days", "A steady routine is the Moon's medicine.", "40 days"),
            (Daana, "Give rice, milk or white cloth on a Monday", "The Moon's offerings, in Chandra hora.", "once"),
            (Mantra, "Shiva or Chandra beeja on Mondays", "Optional support.", "if you wish"),
            (Pooja, "Abhisheka on a Monday", "Optional.", "not required"),
        ],
        DashaLord::Mars => vec![
            (Karma, "End one quarrel on your own initiative", "Mars rules conflict; a weak Mars is remedied by courage used for peace.", "this week"),
            (Seva, "Donate blood, or serve those who protect others", "Mars is blood and guardianship.", "once"),
            (Discipline, "No anger spoken aloud for 40 days", "Restraint is the discipline Mars respects.", "40 days"),
            (Daana, "Give red lentils on a Tuesday", "Mars's grain, in Mangala hora.", "once"),
            (Mantra, "Hanuman Chalisa on Tuesdays", "Optional support.", "if you wish"),
            (Pooja, "Mangala shanti", "Optional.", "not required"),
        ],
        DashaLord::Mercury => vec![
            (Karma, "Speak less and only what is true for a week", "Mercury is speech and trade; truthful, measured speech is its remedy.", "7 days"),
            (Seva, "Teach a child or an adult learner for an hour", "Mercury is learning passed on.", "weekly"),
            (Discipline, "No gossip, no idle scrolling, for 40 days", "Mercury weakens with scattered attention.", "40 days"),
            (Daana, "Give green moong on a Wednesday", "Mercury's grain, in Budha hora.", "once"),
            (Mantra, "Vishnu Sahasranama on Wednesdays", "Optional support.", "if you wish"),
            (Pooja, "Budha shanti", "Optional.", "not required"),
        ],
        DashaLord::Jupiter => vec![
            (Karma, "Honour a teacher you have neglected", "Jupiter is the guru; gratitude restores it.", "this week"),
            (Seva, "Feed students, or fund one student's books", "Jupiter is knowledge given freely.", "once"),
            (Discipline, "Read scripture or a serious book daily for 40 days", "Jupiter grows with study.", "40 days"),
            (Daana, "Give turmeric or chana dal on a Thursday", "Jupiter's offerings, in Guru hora.", "once"),
            (Mantra, "Guru beeja on Thursdays", "Optional support.", "if you wish"),
            (Pooja, "Guru shanti", "Optional.", "not required"),
        ],
        DashaLord::Venus => vec![
            (Karma, "Keep a promise made in a relationship", "Venus is bonds kept; keeping them is the remedy.", "this week"),
            (Seva, "Serve women, artists or the sick with your skill", "Venus is grace given.", "weekly"),
            (Discipline, "Simplify one indulgence for 40 days", "Venus weakens with excess.", "40 days"),
            (Daana, "Give curd or white sweets on a Friday", "Venus's offerings, in Shukra hora.", "once"),
            (Mantra, "Lakshmi or Shukra beeja on Fridays", "Optional support.", "if you wish"),
            (Pooja, "Shukra shanti", "Optional.", "not required"),
        ],
        DashaLord::Saturn => vec![
            (Karma, "Finish the work you have postponed longest", "Saturn pays completed duty and withholds for avoided duty.", "20 min · daily"),
            (Seva, "Serve the old or the labouring on Saturdays", "Saturn signifies the elderly and workers. An hour of real help.", "1 h · Saturdays"),
            (Discipline, "Rise before the Sun for 40 days", "Discipline is the Saturn remedy that needs no one else.", "40 days"),
            (Daana, "Give black til or a blanket on a Saturday", "By hand, in Shani hora, to a person who needs it.", "once"),
            (Mantra, "Hanuman Chalisa on Saturdays", "Optional. Supports the discipline above.", "if you wish"),
            (Pooja, "Shani shanti at a temple", "Optional. Nothing here requires it.", "not required"),
        ],
        DashaLord::Rahu => vec![
            (Karma, "Stop one shortcut or deception you have been running", "Rahu is the shortcut; refusing it is the remedy.", "this week"),
            (Seva, "Serve the outcast, the sick without family, the foreigner", "Rahu's people.", "weekly"),
            (Discipline, "No intoxicants for 40 days", "Rahu feeds on confusion.", "40 days"),
            (Daana, "Give mustard oil or blue cloth on a Saturday", "Rahu's offerings.", "once"),
            (Mantra, "Durga or Rahu beeja", "Optional support.", "if you wish"),
            (Pooja, "Rahu shanti", "Optional.", "not required"),
        ],
        DashaLord::Ketu => vec![
            (Karma, "Finish something you abandoned", "Ketu is detachment misapplied; completing restores it.", "this week"),
            (Seva, "Feed and care for stray dogs, or serve a renunciate", "Ketu's charges.", "weekly"),
            (Discipline, "Live simply for 40 days: one meal plain, one hour of silence", "Ketu rewards simplicity.", "40 days"),
            (Daana, "Give a blanket or sesame", "Ketu's offerings.", "once"),
            (Mantra, "Ganesha mantra", "Optional support.", "if you wish"),
            (Pooja, "Ketu shanti", "Optional.", "not required"),
        ],
    }
}

// ---------------------------------------------------------------------------------
// Rows and their cues.
// ---------------------------------------------------------------------------------

/// Which of the planet's windows a row is timed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cue {
    /// Any hora of the planet on its weekday.
    Hora,
    /// A daytime hora (a daana is never given at night).
    DayHora,
    /// The first hora after sunrise: the day's cue for a vow.
    FirstHora,
    /// Brahma muhurta before the planet's weekday sunrise.
    Brahma,
    /// The planet's own tithis.
    PlanetTithi,
    /// Specific tithis (1–30).
    Tithis(&'static [u8]),
    /// The Moon in one named nakshatra (index 0 = Ashwini).
    NakshatraIndex(usize),
    /// Rahu kala itself (the Kerala Durga lamp).
    RahuKala,
}

/// A remedy before its windows are attached.
struct Row {
    kind: RemedyKind,
    planet: DashaLord,
    rule_id: String,
    title: String,
    why: String,
    source: &'static str,
    effort: &'static str,
    cadence: String,
    count: Option<u32>,
    duration_days: Option<u32>,
    cue: Cue,
    /// Severity the fit is scaled by (the cause's, or the dosha's).
    weight: f64,
    measurable: String,
}

fn weekday_name(g: DashaLord) -> &'static str {
    WEEKDAY_NAMES[weekday_of(g) as usize]
}

fn label_tithis(nums: &[u8]) -> Vec<(u8, String)> {
    nums.iter()
        .map(|&n| {
            let t = Tithi {
                number: n,
                fraction: 0.0,
            };
            (
                n,
                format!(
                    "{} ({})",
                    t.name(),
                    if t.is_shukla() { "Shukla" } else { "Krishna" }
                ),
            )
        })
        .collect()
}

/// Attaches windows to rows from one computed table, caching per planet.
struct Scheduler<'a> {
    table: &'a TimingTable,
    obs: &'a Observer,
    cache: HashMap<DashaLord, Vec<RemedyWindow>>,
}

impl<'a> Scheduler<'a> {
    fn planet_windows(&mut self, g: DashaLord) -> Vec<RemedyWindow> {
        if let Some(w) = self.cache.get(&g) {
            return w.clone();
        }
        let w = self.table.windows(self.obs, g, 3);
        self.cache.insert(g, w.clone());
        w
    }

    fn pick(&mut self, planet: DashaLord, cue: Cue) -> Vec<RemedyWindow> {
        let all = self.planet_windows(planet);
        let mut chosen: Vec<RemedyWindow> = match cue {
            Cue::Hora => all
                .iter()
                .filter(|w| w.kind == "weekday hora")
                .cloned()
                .collect(),
            Cue::DayHora => all
                .iter()
                .filter(|w| w.is_daytime_hora())
                .cloned()
                .collect(),
            Cue::FirstHora => all.iter().filter(|w| w.hora_index == 1).cloned().collect(),
            Cue::Brahma => all
                .iter()
                .filter(|w| w.kind == "Brahma muhurta")
                .cloned()
                .collect(),
            Cue::PlanetTithi => all.iter().filter(|w| w.kind == "tithi").cloned().collect(),
            Cue::Tithis(nums) => self.table.tithi_windows(&label_tithis(nums), 3),
            Cue::NakshatraIndex(i) => self
                .table
                .nakshatras
                .iter()
                .filter(|s| s.index == i && s.end > self.table.from)
                .map(|s| RemedyWindow {
                    start: s.start,
                    end: s.end,
                    kind: "nakshatra",
                    words: format!(
                        "the Moon in {}: {} to {}",
                        nakshatra((i as f64 + 0.5) * NAKSHATRA_SPAN).name(),
                        fmt_utc(s.start),
                        fmt_utc(s.end)
                    ),
                    hora_index: 0,
                })
                .collect(),
            Cue::RahuKala => all
                .iter()
                .filter(|w| w.kind == "Rahu kala to avoid")
                .cloned()
                .collect(),
        };
        chosen.truncate(3);
        if chosen.len() < 3 {
            // pad with the planet's sunrise horas first, then its day horas, then the rest
            let padding = all
                .iter()
                .filter(|w| w.hora_index == 1)
                .chain(
                    all.iter()
                        .filter(|w| w.is_daytime_hora() && w.hora_index != 1),
                )
                .chain(all.iter().filter(|w| !w.is_daytime_hora()));
            for w in padding {
                if chosen.len() == 3 {
                    break;
                }
                if (w.kind == "Rahu kala to avoid" && cue != Cue::RahuKala) || chosen.contains(w) {
                    continue;
                }
                chosen.push(w.clone());
            }
        }
        chosen.sort_by_key(|w| w.start);
        chosen
    }
}

fn planet_rows(
    g: DashaLord,
    severity: f64,
    debilitated: bool,
    functional_benefic: bool,
    weak: bool,
) -> Vec<Row> {
    let day = weekday_name(g);
    let mut rows = Vec::new();
    for (kind, title, why, effort) in table(g) {
        let (cadence, count, duration_days, cue, measurable) = match kind {
            RemedyKind::Karma => {
                let cadence = if effort.contains("daily") {
                    "daily".to_string()
                } else if effort.contains("weekly") {
                    format!("weekly on {day}")
                } else {
                    "once".to_string()
                };
                let dur = if cadence == "once" { Some(7) } else { None };
                (cadence, None, dur, Cue::FirstHora, format!("Log 'did it at <instant>' once a day; the ledger counts days kept over days planned, from the next {g:?} sunrise hora."))
            }
            RemedyKind::Seva => (
                format!("weekly on {day}"),
                None,
                None,
                Cue::DayHora,
                format!("Log the hour served with its start instant; one planned per {day}."),
            ),
            RemedyKind::Discipline => {
                let sunrise = title.contains("sunrise") || title.contains("before the Sun");
                (
                    "40 days".to_string(),
                    Some(40),
                    Some(40),
                    if sunrise { Cue::Brahma } else { Cue::FirstHora },
                    if sunrise {
                        "Log each morning's wake instant against the Brahma muhurta window; 40 planned; one missed day does not void it (repair in the next hora, no shame).".to_string()
                    } else {
                        "Log each day kept at its sunrise cue; 40 planned; one missed day does not void it.".to_string()
                    },
                )
            }
            RemedyKind::Daana => (
                "once".to_string(),
                None,
                None,
                Cue::DayHora,
                "Log the instant you handed it over, by your own hand, and to whom; no amount is needed.".to_string(),
            ),
            RemedyKind::Mantra => (
                format!("weekly on {day}"),
                None,
                None,
                Cue::Hora,
                "Tap once per mala (108) inside the hora; the count is yours to log, never assumed.".to_string(),
            ),
            RemedyKind::Pooja => (
                "once".to_string(),
                None,
                None,
                Cue::PlanetTithi,
                "Log the instant performed; optional, so nothing is planned against it.".to_string(),
            ),
            _ => unreachable!("base table has six kinds"),
        };
        let mut why = why.to_string();
        if kind == RemedyKind::Daana {
            let s = graha_shanti(g);
            why.push_str(&format!(
                " The text's item: {}, with an image of {}, samidha {}, colour {}, naivedya {}; a modern equivalent is {}. Given on {day} in {g:?}'s hora, after sunrise and before sunset, never at night, by your own hand, to a person who needs it.",
                s.daana, s.image, s.samidha, s.colour, s.naivedya, s.practical
            ));
        }
        let source = match kind {
            RemedyKind::Daana => GRAHA_SHANTI_SOURCE,
            RemedyKind::Karma | RemedyKind::Seva => HOUSE_DUTIES_SOURCE,
            RemedyKind::Discipline => "Dharma-shastra conduct items; the 40-day horizon sits inside Lally 2010's 18–254-day habit range",
            RemedyKind::Mantra => MANTRA_DOSES_SOURCE,
            _ => "Temple practice (Kerala); optional, never required",
        };
        rows.push(Row {
            kind,
            planet: g,
            rule_id: format!("KS-R-GRAHA-{g:?}-{}", kind.rung()),
            title: title.to_string(),
            why,
            source,
            effort,
            cadence,
            count,
            duration_days,
            cue,
            weight: severity
                + if debilitated && kind == RemedyKind::Daana {
                    0.1
                } else {
                    0.0
                },
            measurable,
        });
    }
    // Vrata
    let v = vrata_row(g);
    let (cadence, duration) = match v.count {
        Some(n) => (format!("weekly on {day}"), Some(n * 7)),
        None => ("43 days".to_string(), Some(43)),
    };
    let title = match v.count {
        Some(n) => format!("{}: {n} {day}s", v.name),
        None => v.name.to_string(),
    };
    rows.push(Row {
        kind: RemedyKind::Vrata,
        planet: g,
        rule_id: format!("KS-R-VRATA-{g:?}"),
        title,
        why: format!(
            "{}. {}The day runs from the true sunrise at your place; the break (sunset or moonrise) is a computed instant. A vow of simplicity for the day, not a treatment: eat if you are unwell.",
            v.rule,
            match v.alternate {
                Some(a) => format!("Some keep {a} instead. "),
                None => String::new(),
            }
        ),
        source: VRATAS_SOURCE,
        effort: "one day a week",
        cadence,
        count: v.count,
        duration_days: duration,
        cue: Cue::FirstHora,
        weight: severity,
        measurable: format!("Log each {day} kept at its sunrise cue; {} planned.", v.count.map(|n| n.to_string()).unwrap_or_else(|| "43 days".into())),
    });
    // Full japa dose
    let m = mantra_dose(g);
    let days = m.count.div_ceil(216);
    rows.push(Row {
        kind: RemedyKind::Mantra,
        planet: g,
        rule_id: format!("KS-R-JAPA-{g:?}"),
        title: format!("{g:?} beeja japa: {} in all", m.count),
        why: format!(
            "The count used in temple practice is {} (×{KALI_MULTIPLIER} in Kali Yuga by practice = {}){}. Beeja: {}. Vedic rk: {}. Hymn: {}. Purascharana ladder: homa a tenth, tarpana a hundredth, marjana a thousandth, brahmana bhojana a ten-thousandth of the count. At two malas a day the count ends in {days} days; begin on {day} in {g:?}'s hora in Shukla paksha, the Moon in {g:?}'s star or Pushya. Optional, never required; the count is shown so you know the full classical dose rather than a token eleven.",
            m.count,
            m.count * KALI_MULTIPLIER,
            match m.alternate {
                Some(a) => format!("; the Tirupati list gives {a}"),
                None => String::new(),
            },
            m.beeja,
            m.vedic,
            m.hymn
        ),
        source: MANTRA_DOSES_SOURCE,
        effort: "daily · 2 malas",
        cadence: "daily".to_string(),
        count: Some(m.count),
        duration_days: Some(days),
        cue: Cue::Hora,
        weight: severity,
        measurable: format!("Tap once per mala (108); {} planned; the end date follows from your malas per day.", m.count),
    });
    // Gem: information only, gated
    let gr = gem_row(g);
    let allowed = functional_benefic && weak;
    rows.push(Row {
        kind: RemedyKind::Gem,
        planet: g,
        rule_id: "KS-R-GEM-01".to_string(),
        title: format!("{} (information only)", gr.gem),
        why: if allowed {
            format!(
                "Phaladeepika's stone for {g:?}: {}, in {}, on the {}, put on in {g:?}'s hora on {day} in Shukla paksha. {g:?} is a functional benefic for this lagna and weak by dignity, so the texts allow it. Purchase; optional; not required; no trial claims.",
                gr.gem, gr.metal, gr.finger
            )
        } else {
            format!(
                "No gem: {g:?} is {} for this lagna, and a gem strengthens its planet, so by the texts' own logic it would strengthen the harm. The texts advise mantra, daana and vrata for this planet. Listed only so nothing is hidden.",
                if functional_benefic { "a functional benefic but not weak" } else { "not a functional benefic (it does not own 1, 5 or 9 free of 6, 8 and 12)" }
            )
        },
        source: GEMS_SOURCE,
        effort: "purchase · optional · not required",
        cadence: "once".to_string(),
        count: None,
        duration_days: None,
        cue: Cue::DayHora,
        weight: 0.0,
        measurable: "Nothing to log; information only.".to_string(),
    });
    rows
}

fn house_rows(g: DashaLord, house: u8, severity: f64) -> Vec<Row> {
    let d = house_duty(house);
    let day = weekday_name(g);
    vec![
        Row {
            kind: RemedyKind::Karma,
            planet: g,
            rule_id: format!("KS-R-DUTY-{house:02}"),
            title: {
                let mut c = d.conduct.to_string();
                if let Some(f) = c.get_mut(0..1) {
                    f.make_ascii_uppercase();
                }
                c
            },
            why: format!(
                "The {house}th house is {}; its karaka is {}. The texts read the affliction as this duty left undone, and the remedy is the duty itself.",
                d.domain, d.karaka
            ),
            source: HOUSE_DUTIES_SOURCE,
            effort: "daily",
            cadence: "daily".to_string(),
            count: None,
            duration_days: None,
            cue: Cue::FirstHora,
            weight: severity,
            measurable: format!("Log 'did it at <instant>' once a day from the next {g:?} sunrise hora."),
        },
        Row {
            kind: RemedyKind::Seva,
            planet: g,
            rule_id: format!("KS-R-DUTY-{house:02}-SEVA"),
            title: format!("Serve {}", d.seva),
            why: format!("The people the {house}th house signifies; an hour of real help on {day}, not money alone."),
            source: HOUSE_DUTIES_SOURCE,
            effort: "1 h · weekly",
            cadence: format!("weekly on {day}"),
            count: None,
            duration_days: None,
            cue: Cue::DayHora,
            weight: severity,
            measurable: format!("Log the hour served with its start instant; one planned per {day}."),
        },
    ]
}

fn dasha_rows(
    g: DashaLord,
    level: &'static str,
    since: Instant,
    until: Instant,
    at: Instant,
    severity: f64,
) -> Vec<Row> {
    let f = dasha_formula(g);
    let day = weekday_name(g);
    let days_left = (until.ns_since(at) / NS_PER_DAY).clamp(1, 365) as u32;
    vec![
        Row {
            kind: RemedyKind::Mantra,
            planet: g,
            rule_id: format!("KS-R-DASHA-{g:?}"),
            title: format!("{} (BPHS, for the {level} of {g:?})", f.japa),
            why: format!(
                "The text's remedial measure for an unfavourable period of {g:?}: {}. Conduct equivalent: {}. The {level} began at {} and ends at {}; begin on {day} in {g:?}'s hora in Shukla paksha, and a 'finish within the period' plan has {days_left} days.",
                f.japa,
                f.japa_conduct,
                fmt_utc(since),
                fmt_utc(until)
            ),
            source: DASHA_FORMULAE_SOURCE,
            effort: "daily · 108",
            cadence: "daily".to_string(),
            count: Some(108),
            duration_days: Some(days_left.min(43)),
            cue: Cue::Hora,
            weight: severity,
            measurable: "Tap once per mala (108) in the hora; 108 a day planned until the period ends.".to_string(),
        },
        Row {
            kind: RemedyKind::Daana,
            planet: g,
            rule_id: format!("KS-R-DASHA-{g:?}-DAANA"),
            title: format!("{} (BPHS gift for the {level})", f.daana),
            why: format!(
                "The text's gift for this period: {}. The conduct equivalent, so nothing requires a purchase: {}. Given on {day} in {g:?}'s hora by day.",
                f.daana, f.daana_conduct
            ),
            source: DASHA_FORMULAE_SOURCE,
            effort: "43 days",
            cadence: "43 days".to_string(),
            count: Some(43),
            duration_days: Some(43),
            cue: Cue::DayHora,
            weight: severity,
            measurable: "Log each day's feeding or the one gift with its instant; 43 planned.".to_string(),
        },
    ]
}

/// A dosha row before it becomes a [`Row`]: kind, title, why, effort, cadence, count,
/// duration in days, cue.
type Spec = (
    RemedyKind,
    String,
    String,
    &'static str,
    String,
    Option<u32>,
    Option<u32>,
    Cue,
);

/// Rows for a present dosha: conduct first, then the vow with its tithi, then the
/// rite the text names (optional).
fn dosha_rows(d: &Dosha) -> Vec<Row> {
    use RemedyKind::*;
    let g = d.planet;
    let day = weekday_name(g);
    let specs: Vec<Spec> = match d.rule_id {
        "KS-R-SHAPA-01" => vec![
            (Karma, "Visit or call your father or elders on a fixed day; keep the death tithi of the departed".into(), "A duty to ancestors left undone is the karmic domain of this curse; the conduct is the remedy.".into(), "weekly", "weekly on Sunday".into(), None, None, Cue::FirstHora),
            (Vrata, "Pitru tarpana on every Amavasya; Mahalaya shraddha in Pitru Paksha".into(), "Amavasya is the pitris' tithi; its exact span is below, and Pitru Paksha (Bhadrapada Krishna) is the fortnight for Mahalaya.".into(), "monthly", "monthly on Amavasya".into(), None, None, Cue::Tithis(&[30])),
            (Daana, "Wheat, jaggery or copper on Sunday in the Sun's hora; a cow (classical)".into(), "The Sun's daana (practice); the text's item is a cow, a modern equivalent is feeding animals.".into(), "once", "once".into(), None, None, Cue::DayHora),
            (Pooja, "Shraddha (at Gaya or anywhere), feeding Brahmins, kanyadana (BPHS)".into(), "The chapter's named propitiation: 'lineage continues afterwards'. Optional, ritual.".into(), "not required", "once".into(), None, None, Cue::Tithis(&[30])),
        ],
        "KS-R-SHAPA-02" => vec![
            (Karma, "Protect a tree or a patch of ground for 43 days; stop one deception".into(), "Harm to serpents, their groves or the land is the karmic domain; guarding a living thing is the restitution.".into(), "43 days", "43 days".into(), Some(43), Some(43), Cue::FirstHora),
            (Vrata, "Naga puja on Shukla Panchami; a Durga lamp in Rahu kala on Tuesday or Friday (Kerala)".into(), "Panchami is the serpents' tithi (Naga Panchami falls in Shravana); the exact spans are below.".into(), "monthly", "monthly on Panchami".into(), None, None, Cue::Tithis(&[5])),
            (Pooja, "Naga pratishtha, Sarpa shanti, Ashlesha bali when the Moon is in Ashlesha".into(), "The text's propitiation (installing a serpent image, a cow and gold); the Kerala and Karnataka rite is timed to the Moon in Ashlesha, the exact entry below. Optional, ritual.".into(), "not required", "once".into(), None, None, Cue::NakshatraIndex(8)),
        ],
        "KS-R-SHAPA-03" => vec![
            (Karma, "Tend your mother in person; fixed sleep".into(), "The Moon is the mother and the mind; the conduct is the remedy.".into(), "weekly", "weekly on Monday".into(), None, None, Cue::FirstHora),
            (Vrata, "Somavara vrata; water, milk, black sesame and bilva at a Peepal's roots".into(), "Monday from the true sunrise; the practice pairing for the mother's curse.".into(), "weekly", "weekly on Monday".into(), Some(16), Some(112), Cue::FirstHora),
            (Pooja, "Setu snana and Gayatri japa (the text's pairing; verify)".into(), "As the brief recalls the chapter; verify against Santhanam vol. 2. Optional.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-SHAPA-04" => vec![
            (Karma, "End a quarrel with a sibling".into(), "Mars is the brother; peace made on your own initiative is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Mangalavara vrata (Hanuman): 21 Tuesdays".into(), "Tuesday from the true sunrise; red, no salt.".into(), "weekly", "weekly on Tuesday".into(), Some(21), Some(147), Cue::FirstHora),
            (Pooja, "Gayatri japa with homa; Harivamsha shravana (the text's pairing; verify)".into(), "As the brief recalls the chapter. Optional.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-SHAPA-05" => vec![
            (Karma, "Restore a cut family tie; truthful speech".into(), "Mercury is the maternal uncle and speech; the tie restored is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Budhavara vrata: 21 Wednesdays".into(), "Wednesday from the true sunrise; green moong.".into(), "weekly", "weekly on Wednesday".into(), Some(21), Some(147), Cue::FirstHora),
            (Pooja, "Vishnu pratishtha (the text's pairing; verify)".into(), "Installing a Vishnu image is among the chapter's named remedies. Optional, ritual.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-SHAPA-06" => vec![
            (Karma, "Honour a teacher; return what was taken from a learned person".into(), "Jupiter is the Brahmin and the preceptor; restitution to the learned is the remedy.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Chandrayana: one mouthful more per tithi through Shukla, one less through Krishna, one lunar month".into(), "BPHS names Chandrayana for the Brahmin's curse (V). The day's dose is a pure function of the tithi, which the engine solves; start on Shukla Pratipada, the exact span below.".into(), "one lunar month", "one lunar month from Shukla Pratipada".into(), Some(30), Some(30), Cue::Tithis(&[1])),
            (Pooja, "Service to a cow; Brahmin feeding (BPHS)".into(), "The chapter's pairing, citing Parashara (V). Optional.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-SHAPA-07" => vec![
            (Karma, "Keep a promise to your spouse; no deception in love".into(), "Venus is the wife and bonds kept; the promise kept is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Shukravara vrata: 16 Fridays".into(), "Friday from the true sunrise; white food, nothing sour.".into(), "weekly", "weekly on Friday".into(), Some(16), Some(112), Cue::FirstHora),
            (Pooja, "Lakshmi-Narayana pratishtha, kanyadana, a cow (the text's pairing; verify)".into(), "As the brief recalls the chapter. Optional, ritual.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-SHAPA-08" => vec![
            (Karma, "Complete the rites or the memory of a dead relative; visit a grave".into(), "Saturn and Ketu are the departed; the rite completed is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Tarpana for the departed on Amavasya".into(), "Amavasya is the pretas' tithi; the exact spans are below.".into(), "monthly", "monthly on Amavasya".into(), None, None, Cue::Tithis(&[30])),
            (Pooja, "Gaya shraddha, Rudra japa, gift of a cow or silver (BPHS)".into(), "The chapter's named remedies for the curse of the departed. Optional, ritual.".into(), "not required", "once".into(), None, None, Cue::Tithis(&[30])),
        ],
        "KS-R-BADH-01" => vec![
            (Karma, format!("Visit the shrine or being the chart names ({}); mend what was neglected", d.offended), "Prasna Marga reads the offended party from the badhakesh's companions; the neglect mended is the remedy. Read for a specific trouble, not a whole life.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Pooja, format!("Propitiation by tattva for {}", d.offended), format!("The kind of remedy follows the tattva of the badhakesh's dispositor ({}). Optional, ritual.", d.factors.first().cloned().unwrap_or_default()), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-YOGA-02" => vec![
            (Karma, "No shortcut for 43 days".into(), "The nodal axis holds every planet; refusing the shortcut is the conduct item.".into(), "43 days", "43 days".into(), Some(43), Some(43), Cue::FirstHora),
            (Vrata, "Naga puja on Panchami".into(), "Both pakshas' Panchami; the exact spans are below.".into(), "monthly", "monthly on Panchami".into(), None, None, Cue::Tithis(&[5, 20])),
            (Pooja, "A Durga lamp in Rahu kala (Kerala custom)".into(), "The one use of Rahu kala as a positive window; the exact eighth of the day is below. Optional.".into(), "not required", "weekly on Saturday".into(), None, None, Cue::RahuKala),
        ],
        "KS-R-YOGA-04" => vec![
            (Karma, "Serve your mother; fixed sleep".into(), "The Moon stands alone; the mind is steadied by routine and by tending its karaka.".into(), "weekly", "weekly on Monday".into(), None, None, Cue::FirstHora),
            (Vrata, "Somavara vrata: 16 Mondays".into(), "Monday from the true sunrise.".into(), "weekly", "weekly on Monday".into(), Some(16), Some(112), Cue::FirstHora),
            (Pooja, "Abhisheka on a Monday".into(), "Optional.".into(), "not required", "once".into(), None, None, Cue::DayHora),
        ],
        "KS-R-YOGA-05" => vec![
            (Karma, "Stop one half-truth taught to others; honour teachers".into(), "Jupiter with Rahu is knowledge bent; truth taught is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Daana, "Turmeric and chana on Thursday in Jupiter's hora".into(), "Jupiter's daana by day.".into(), "once", "once".into(), None, None, Cue::DayHora),
            (Mantra, "Vishnu Sahasranama on Thursdays".into(), "Optional support.".into(), "if you wish", "weekly on Thursday".into(), None, None, Cue::Hora),
        ],
        "KS-R-YOGA-06" => vec![
            (Karma, if g == DashaLord::Moon { "Fixed sleep and waking for 40 days".into() } else { "Rise with the Sun for 40 days".into() }, "The luminary under the nodes is steadied by routine.".into(), "40 days", "40 days".into(), Some(40), Some(40), Cue::Brahma),
            (Pooja, "Grahana shanti during the next eclipse, japa from first to last contact".into(), "The eclipse-contact solver is on the roadmap; until then the windows below are the luminary's horas, not the contacts. Optional.".into(), "not required", "once".into(), None, None, Cue::Hora),
        ],
        "KS-R-YOGA-03" => vec![
            (Karma, "End one quarrel".into(), "Mars's heat on the house of the spouse; peace made is the conduct item.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
            (Vrata, "Tuesday Hanuman vow".into(), "Tuesday from the true sunrise.".into(), "weekly", "weekly on Tuesday".into(), Some(21), Some(147), Cue::FirstHora),
            (Daana, "Red lentils on Tuesday in Mars's hora".into(), "Mars's daana by day.".into(), "once", "once".into(), None, None, Cue::DayHora),
            (Pooja, "Mangala Chandika stotra; Kumbha-vivaha (ritual, optional)".into(), "Optional.".into(), "not required", "once".into(), None, None, Cue::PlanetTithi),
        ],
        "KS-R-YOGA-01" => vec![
            (Karma, "Finish the longest-postponed duty".into(), "Saturn over the Moon pays completed duty; this is anchored at the exact phase ingress and re-anchored at each phase.".into(), "daily", "daily".into(), None, None, Cue::FirstHora),
            (Seva, "Elders and labourers on Saturdays".into(), "Saturn's people; an hour of real help.".into(), "1 h · Saturdays", "weekly on Saturday".into(), None, None, Cue::DayHora),
            (Daana, "Black til, mustard oil, iron or a blanket in Shani hora on Saturday".into(), "Saturn's daana by day.".into(), "once", "once".into(), None, None, Cue::DayHora),
            (Vrata, "Shani Trayodashi; 7 Saturdays".into(), "Trayodashi on a Saturday is Shani Trayodashi (V); the exact spans of Trayodashi are below.".into(), "weekly", "weekly on Saturday".into(), Some(7), Some(49), Cue::Tithis(&[13, 28])),
            (Mantra, "Hanuman Chalisa on Tuesday and Saturday; Shani beeja 23,000; Dasharatha Shani Stotra".into(), "Optional support.".into(), "if you wish", "weekly on Saturday".into(), Some(23_000), None, Cue::Hora),
            (Pooja, "Tila-taila lamp".into(), "Optional.".into(), "not required", "weekly on Saturday".into(), None, None, Cue::DayHora),
        ],
        _ => Vec::new(),
    };
    specs
        .into_iter()
        .map(
            |(kind, title, why, effort, cadence, count, duration_days, cue)| Row {
                kind,
                planet: g,
                rule_id: format!("{}-{}", d.rule_id, kind.rung()),
                title,
                why: format!("{why} [{}: {}]", d.name, d.factors.join("; ")),
                source: d.source,
                effort,
                cadence,
                count,
                duration_days,
                cue,
                weight: d.severity,
                measurable: match kind {
                    Karma => {
                        "Log 'did it at <instant>' against the cue; days kept over days planned."
                            .to_string()
                    }
                    Seva => format!(
                        "Log the hour served with its start instant; one planned per {day}."
                    ),
                    Vrata => {
                        "Log each day kept at its cue; the count planned is the vow's.".to_string()
                    }
                    Daana => "Log the instant you handed it over and to whom.".to_string(),
                    Mantra => "Tap once per mala; the count is yours to log.".to_string(),
                    _ => "Log the instant performed; optional, nothing planned against it."
                        .to_string(),
                },
            },
        )
        .collect()
}

// ---------------------------------------------------------------------------------
// The plan.
// ---------------------------------------------------------------------------------

fn transits_at(spk: &Spk, at: Instant, ayanamsa: Ayanamsa) -> Result<TransitSigns, EventError> {
    let ay = ayanamsa_degrees(ayanamsa, at.tt_centuries_since_j2000());
    let sign = |lon: f64| (sidereal(lon, ay) / 30.0).floor() as u8 % 12;
    let sat = apparent(spk, Body::SaturnBarycenter, at)
        .map_err(EventError::Spk)?
        .longitude;
    let jup = apparent(spk, Body::JupiterBarycenter, at)
        .map_err(EventError::Spk)?
        .longitude;
    let rahu = true_node_longitude(spk, at).map_err(EventError::Spk)?;
    Ok(TransitSigns {
        saturn: sign(sat),
        jupiter: sign(jup),
        rahu: sign(rahu),
        ketu: sign(rahu + 180.0),
    })
}

/// Sade Sati phase from Saturn's sidereal sign and the natal Moon's: 0 none, 1 rising
/// (12th), 2 peak (1st), 3 setting (2nd).
pub fn sade_sati_phase(saturn_sign: u8, moon_sign: u8) -> u8 {
    match (saturn_sign as i32 - moon_sign as i32).rem_euclid(12) + 1 {
        12 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    }
}

fn duty_house(facts: &Facts, lord: DashaLord, area: Option<Area>) -> u8 {
    if let Some(a) = area {
        return a.primary;
    }
    let p = facts.planet(lord);
    let owned: Vec<u8> = (1..=12u8)
        .filter(|h| facts.rasi_lords[(*h - 1) as usize] == lord)
        .collect();
    owned
        .iter()
        .copied()
        .find(|h| matches!(h, 1 | 4 | 5 | 7 | 9 | 10))
        .or_else(|| owned.first().copied())
        .unwrap_or(p.rasi_house)
}

/// Build the remedy plan at `at` for the natal `chart`, optionally narrowed to an area.
/// The running lords (maha, antar, pratyantar) come from the chart's own Moon and
/// birth instant; the slow transits and Sade Sati phase from the sky at `at`; every
/// window from the solvers for the observer's place. Errors only on ephemeris failure.
#[allow(clippy::too_many_arguments)]
pub fn parihara(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
    at: Instant,
    area: Option<Area>,
) -> Result<Parihara, EventError> {
    let facts = Facts::from_chart(chart);
    let chain = vimshottari_levels(
        chart.bodies[1].sidereal,
        chart.instant,
        YEAR_365_25_NS,
        at,
        3,
    );
    let lords: Vec<DashaLord> = chain.iter().map(|p| p.lord).collect();
    const LEVELS: [&str; 3] = ["mahadasha", "antardasha", "pratyantar"];

    // Candidate causes with severity
    let mut candidates: Vec<(DashaLord, f64, String, Option<usize>)> = Vec::new();
    for (i, p) in chain.iter().enumerate() {
        let l = p.lord;
        let (q, why) = match area {
            Some(a) => period_quality(&facts, a, l),
            None => {
                let pf = facts.planet(l);
                let d = dignity(l, pf.sidereal);
                (
                    d.score(),
                    vec![format!("{l:?} is {d:?} in rasi house {}", pf.rasi_house)],
                )
            }
        };
        let level = LEVELS.get(i).copied().unwrap_or("period");
        candidates.push((
            l,
            1.0 - q,
            format!("{l:?}, the running {level} lord: {}", why.join("; ")),
            Some(i),
        ));
    }
    if let Some(a) = area {
        let lord = facts.rasi_lords[(a.primary - 1) as usize];
        let p = facts.planet(lord);
        let d = dignity(lord, p.sidereal);
        candidates.push((
            lord,
            1.0 - d.score(),
            format!(
                "{lord:?}, lord of the {}th house of {}, is {d:?} in rasi house {}",
                a.primary, a.name, p.rasi_house
            ),
            None,
        ));
    }
    let (planet, severity, located, level_idx) = candidates
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((
            DashaLord::Saturn,
            0.0,
            "no running period found".into(),
            None,
        ));
    let running_idx = level_idx.or_else(|| chain.iter().position(|p| p.lord == planet));
    let running_level = running_idx.and_then(|i| LEVELS.get(i).copied());
    let running_since = running_idx.map(|i| chain[i].start);
    let running_until = running_idx.map(|i| chain[i].end);

    let p = facts.planet(planet);
    let dig = dignity(planet, p.sidereal);
    let debilitated = dig == Dignity::Debilitated;
    let functional_benefic = p.owns.iter().any(|h| matches!(h, 1 | 5 | 9))
        && !p.owns.iter().any(|h| matches!(h, 6 | 8 | 12));
    let weak = dig < Dignity::Friend;

    // The karma gate and the dosha families
    let transits = transits_at(spk, at, ayanamsa)?;
    let moon_sign = facts.planet(DashaLord::Moon).sign;
    let phase = sade_sati_phase(transits.saturn, moon_sign);
    let karma = classify(&facts, planet, area, &lords, Some(&transits));
    let mut present: Vec<Dosha> = doshas(&facts, at, phase)
        .into_iter()
        .filter(|d| d.present)
        .collect();
    present.sort_by(|a, b| {
        b.severity
            .partial_cmp(&a.severity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Timing
    let table = TimingTable::compute(spk, eop, obs, ayanamsa, at, 35)?;
    let mut sched = Scheduler {
        table: &table,
        obs,
        cache: HashMap::new(),
    };

    // Rows
    let house = duty_house(&facts, planet, area);
    let mut rows = planet_rows(planet, severity, debilitated, functional_benefic, weak);
    if area.is_some() {
        rows.extend(house_rows(planet, house, severity));
    }
    if let (Some(level), Some(since), Some(until)) = (running_level, running_since, running_until) {
        if severity >= 0.5 {
            rows.extend(dasha_rows(planet, level, since, until, at, severity));
        }
    }
    for d in present.iter().take(3) {
        rows.extend(dosha_rows(d));
    }

    let mut remedies: Vec<Remedy> = rows
        .into_iter()
        .map(|r| {
            let windows = sched.pick(r.planet, r.cue);
            let fit = if r.kind == RemedyKind::Gem {
                0.0
            } else {
                (r.kind.base_fit() * (0.5 + 0.5 * r.weight.min(1.0))).clamp(0.0, 1.0)
            };
            Remedy {
                kind: r.kind,
                rung: r.kind.rung(),
                rule_id: r.rule_id,
                title: r.title,
                why: r.why,
                source: r.source,
                fit,
                effort: r.effort,
                optional: r.kind.is_optional(),
                may_claim: may_claim(karma.class, karma.kind_in_play, r.kind.rung()),
                schedule: Schedule {
                    cadence: r.cadence,
                    count: r.count,
                    duration_days: r.duration_days,
                    windows,
                },
                measurable: r.measurable,
            }
        })
        .collect();
    remedies.sort_by(|a, b| {
        a.rung
            .cmp(&b.rung)
            .then(
                b.fit
                    .partial_cmp(&a.fit)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.title.cmp(&b.title))
    });

    // Words
    let duty = house_duty(house);
    let first_cue = sched
        .pick(planet, Cue::FirstHora)
        .first()
        .map(|w| fmt_utc(w.start))
        .unwrap_or_else(|| "the next hora".into());
    let house_lord = facts.rasi_lords[(house - 1) as usize];
    let relation = if house_lord == planet {
        format!("rules your {house}th house of {}", duty.domain)
    } else {
        format!(
            "bears on your {house}th house of {} (ruled by {house_lord:?})",
            duty.domain
        )
    };
    let mut words = format!(
        "{planet:?} {relation}; it stands {dig:?} in rasi house {}{}. The texts read this as a duty to {} left undone. The remedy is the duty itself: {}, from {first_cue}.",
        p.rasi_house,
        match (running_level, running_since) {
            (Some(l), Some(s)) => format!(" and runs as {l} lord from {}", fmt_utc(s)),
            _ => String::new(),
        },
        duty.domain,
        duty.conduct
    );
    words.push_str(&format!(" Located as: {located}."));
    if !present.is_empty() {
        let names: Vec<String> = present
            .iter()
            .take(3)
            .map(|d| format!("{} ({})", d.name, d.offended))
            .collect();
        words.push_str(&format!(" The chart also carries {}.", names.join("; ")));
    }
    words.push_str(&format!(
        " Karma: {} ({}): {}.",
        karma.class.name(),
        karma.kind_in_play.name(),
        karma.may_claim
    ));
    if severity < 0.25 {
        words.push_str(" Nothing here is hostile. The period withholds only what duty leaves undone; no remedy beyond your own effort is required.");
    }

    let ledger_hint = format!(
        "Every row is an if-then plan: IF its window begins (an exact instant) THEN the one act, with its count. Record each occurrence as 'did it at <instant>' against the planned window; the ledger shows adherence (done over planned), punctuality (inside the window), streaks with repairs counted separately, and a weekly four-item automaticity check. One missed day does not void a 40- or 43-day vow (Lally 2010): repair in the next {planet:?} hora, with no shaming copy. Nothing here measures the sky; it measures what you did.{}",
        if karma.class == crate::karma::KarmaClass::Dridha {
            " This cause is dridha: no signal trend is shown for it."
        } else {
            ""
        }
    );

    Ok(Parihara {
        planet,
        severity,
        cause: Cause {
            lord: planet,
            house,
            placed_in: p.rasi_house,
            dignity: dig,
            running_level,
            running_since,
            running_until,
            severity,
            karma,
            doshas: present,
            words,
        },
        remedies,
        ledger_hint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::AREAS;
    use kshana_chart::{compute, ChartInput};
    use kshana_houses::HouseSystem;
    use kshana_time::Utc;

    #[test]
    fn every_graha_has_a_karma_first_plan_and_full_tables() {
        for g in DashaLord::ORDER {
            let t = table(g);
            assert_eq!(t.len(), 6);
            assert_eq!(t[0].0, RemedyKind::Karma);
            assert_eq!(
                t.iter()
                    .filter(|r| matches!(r.0, RemedyKind::Mantra | RemedyKind::Pooja))
                    .count(),
                2
            );
            assert_eq!(graha_shanti(g).planet, g);
            assert_eq!(dasha_formula(g).planet, g);
            assert_eq!(mantra_dose(g).planet, g);
            assert_eq!(vrata_row(g).planet, g);
            assert_eq!(gem_row(g).planet, g);
        }
        assert_eq!(mantra_dose(DashaLord::Saturn).count, 23_000);
        assert_eq!(mantra_dose(DashaLord::Sun).alternate, Some(6_000));
        for h in 1..=12u8 {
            assert_eq!(house_duty(h).house, h);
        }
        assert_eq!(sade_sati_phase(11, 0), 1);
        assert_eq!(sade_sati_phase(0, 0), 2);
        assert_eq!(sade_sati_phase(1, 0), 3);
        assert_eq!(sade_sati_phase(5, 0), 0);
        assert!(RemedyKind::Karma.rung() < RemedyKind::Gem.rung());
    }

    fn kochi_chart(spk: &Spk) -> Chart {
        let t = Utc::new(1990, 10, 29, 23, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        compute(
            spk,
            None,
            &ChartInput {
                instant: t,
                observer: Observer {
                    latitude_deg: 9.9312,
                    longitude_deg: 76.2673,
                    height_m: 0.0,
                },
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap()
    }

    #[test]
    fn kochi_plan_is_ranked_sourced_and_timed() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let ch = kochi_chart(&spk);
        let at = Utc::new(2026, 10, 15, 9, 2, 7, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        for area in [None, Some(AREAS[0])] {
            let plan = parihara(&spk, None, &obs, &ch, Ayanamsa::Lahiri, at, area).unwrap();
            assert!(!plan.remedies.is_empty());
            assert_eq!(plan.remedies[0].kind, RemedyKind::Karma);
            assert_eq!(plan.remedies.last().unwrap().kind, RemedyKind::Gem);
            assert!(plan.remedies.windows(2).all(|w| w[0].rung <= w[1].rung));
            for r in &plan.remedies {
                assert!(!r.source.is_empty() && !r.rule_id.is_empty(), "{}", r.title);
                assert!(!r.may_claim.is_empty() && !r.measurable.is_empty());
                assert!((0.0..=1.0).contains(&r.fit));
                assert_eq!(r.optional, r.kind.is_optional());
                assert!(
                    r.schedule.windows.len() >= 3,
                    "{} has {} windows",
                    r.title,
                    r.schedule.windows.len()
                );
                for w in &r.schedule.windows {
                    assert!(w.start < w.end && w.end > at, "{}", w.words);
                }
                assert!(r
                    .schedule
                    .windows
                    .windows(2)
                    .all(|p| p[0].start <= p[1].start));
                if r.kind == RemedyKind::Daana {
                    assert!(r
                        .schedule
                        .windows
                        .iter()
                        .all(|w| w.is_daytime_hora() || w.kind != "weekday hora"));
                }
                if r.kind == RemedyKind::Gem {
                    assert_eq!(r.fit, 0.0);
                    assert!(r.why.contains("information") || r.why.contains("No gem"));
                }
            }
            // the planet's own rows are all there
            let kinds: Vec<RemedyKind> = plan.remedies.iter().map(|r| r.kind).collect();
            for k in [
                RemedyKind::Karma,
                RemedyKind::Seva,
                RemedyKind::Discipline,
                RemedyKind::Daana,
                RemedyKind::Vrata,
                RemedyKind::Mantra,
                RemedyKind::Pooja,
                RemedyKind::Gem,
            ] {
                assert!(kinds.contains(&k), "{k:?} missing");
            }
            // cause
            let c = &plan.cause;
            assert_eq!(c.lord, plan.planet);
            assert!(c.words.contains(&format!("{:?}", c.lord)));
            assert!(c.words.contains("Karma:"));
            assert!(!c.karma.basis.is_empty());
            assert!(c.doshas.iter().all(|d| d.present));
            assert!(c.doshas.windows(2).all(|w| w[0].severity >= w[1].severity));
            assert!(c.doshas.iter().any(|d| d.name == "Sade Sati") || sade_sati_phase(0, 0) == 2);
            assert!(plan.ledger_hint.contains("did it at"));
            if let Some(a) = area {
                assert_eq!(c.house, a.primary);
                assert!(plan
                    .remedies
                    .iter()
                    .any(|r| r.rule_id.starts_with("KS-R-DUTY-")));
            }
            // mantra dose carries the count and the Kali note
            let japa = plan
                .remedies
                .iter()
                .find(|r| r.rule_id.starts_with("KS-R-JAPA-"))
                .unwrap();
            assert_eq!(japa.schedule.count, Some(mantra_dose(plan.planet).count));
            assert!(japa.why.contains("Kali Yuga"));
            // the windows belong to the cause planet's weekday
            let wd = weekday_of(plan.planet);
            for r in plan
                .remedies
                .iter()
                .filter(|r| r.rule_id.starts_with("KS-R-GRAHA-"))
            {
                for w in r
                    .schedule
                    .windows
                    .iter()
                    .filter(|w| w.kind == "weekday hora")
                {
                    let mid = w.start.add_ns(w.end.ns_since(w.start) / 2);
                    let d = kshana_events::solar_day(&spk, None, &obs, mid)
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        crate::remedy_timing::weekday_at(d.sunrise, &obs),
                        wd,
                        "{}",
                        w.words
                    );
                }
            }
        }
    }
}
