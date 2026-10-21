//! Parihara: remedies tied to a located cause, a karma class, a classical source and an
//! exact timing window, ranked by fit and effort, actions first.
//!
//! The cause is located in the chart and the period: the weakest of the running period
//! lords (maha, antar, pratyantar), or the lord of the area's house. Before any remedy
//! is shown the karma gate ([`crate::karma`]) decides what a remedy may claim, and the
//! dosha detectors ([`crate::doshas`]) say which curse, badhaka or yoga family the
//! chart carries. Each remedy then carries its rung on the ladder, its source, what it
//! may claim for this karma class, a schedule with the next exact windows from
//! [`crate::remedy_timing`], and the one thing the app records for it.
//!
//! Each graha carries a dictionary of remedies drawn from the Jyotisha remedial
//! tradition: conduct and service from the dharma reading of each graha's
//! signification; fasts (vrata), donations (daana), japa counts, deities and temples
//! from Brihat Parashara Hora Shastra (Graha shanti), the Puranic graha-daana lists,
//! Lal Kitab and Kerala and Tamil temple practice; food, creatures, water and colour
//! from [`crate::daana`]. When a life area is known, its own remedies are added.
//!
//! The ladder (top engages the most and needs no one else): 1 karma (conduct: change
//! the act the house signifies, Prasna Marga's purushakara), 2 seva (and food,
//! creatures, water and trees), 3 vrata and discipline (and colours kept), 4 daana,
//! 5 japa, 6 pooja (needs a priest; optional), 7 gems (purchase; information only,
//! gated by functional-benefic status, never recommended). Conduct, service and
//! discipline are always ranked above donation, mantra and pooja, and the ritual items
//! are marked optional: nothing in this module requires a purchase or a priest.
//! Efficacy is always the text's claim, never a number.
//!
//! Plain words: every remedy's `title` and `why` is read by the person and never names
//! a planet, house, sign, star or period ([`crate::plain::jargon`] holds the line). The
//! texts' own column (graha, hora, karaka, dosha factors) is kept in `why_expert` and in
//! [`Cause::words`], for the expert pages behind "Show the chart pages".

use std::collections::HashMap;

use crate::areas::Area;
use crate::doshas::{doshas, Dosha};
use crate::facts::Facts;
use crate::karma::{classify, may_claim, KarmaVerdict, TransitSigns};
use crate::parashari::{dignity, period_quality, Dignity};
use crate::plain::jargon;
use crate::remedy_timing::{fmt_utc, weekday_of, RemedyWindow, TimingTable, WEEKDAY_NAMES};
use kshana_chart::Chart;
use kshana_ephem::{Body, Spk};
use kshana_events::EventError;
use kshana_sky::{apparent, rahu_longitude, Observer};
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
    /// Food given to the graha's people.
    Anna,
    /// Creatures fed: crows, cows, dogs, ants, fish, birds.
    Jeeva,
    /// Water, trees and places tended.
    Prakriti,
    /// Colours worn and kept from, small things carried.
    Vastra,
    /// Information only: never recommended, listed so nothing is hidden.
    Gem,
}

impl RemedyKind {
    /// Rung on the ladder: 1 karma, 2 seva (food, creatures, water and trees with it),
    /// 3 vrata and discipline (colours kept with it), 4 daana, 5 mantra, 6 pooja, 7 gem.
    pub fn rung(self) -> u8 {
        match self {
            RemedyKind::Karma => 1,
            RemedyKind::Seva | RemedyKind::Anna | RemedyKind::Jeeva | RemedyKind::Prakriti => 2,
            RemedyKind::Discipline | RemedyKind::Vrata | RemedyKind::Vastra => 3,
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
            RemedyKind::Anna => 0.82,
            RemedyKind::Jeeva => 0.78,
            RemedyKind::Vrata => 0.75,
            RemedyKind::Prakriti => 0.72,
            RemedyKind::Daana => 0.65,
            RemedyKind::Vastra => 0.5,
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
    /// A count to log against: mantra count (the Kali multiplier is noted in
    /// `why_expert`), weeks of a vrata, days of a vow.
    pub count: Option<u32>,
    /// Length of the vow in days, when it has one.
    pub duration_days: Option<u32>,
    /// The next three exact windows the remedy is timed to (empty when no sky was
    /// computed, as in [`parihara_from_facts`]).
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
    /// Short title, in plain words.
    pub title: String,
    /// Why this, in one or two plain sentences: what it eases in the person's life.
    pub why: String,
    /// The texts' own column (graha, hora, karaka, dosha factors), expert pages only.
    pub why_expert: String,
    /// The text behind it.
    pub source: &'static str,
    /// Fit score in `[0, 1]`.
    pub fit: f64,
    /// Effort, in words.
    pub effort: &'static str,
    /// The day to begin or keep it, in words ("Saturday", "any day").
    pub day: &'static str,
    /// True for mantra, pooja and gem: supportive, never required.
    pub optional: bool,
    /// What the texts let this rung claim for the cause's karma class.
    pub may_claim: &'static str,
    /// When, how often, and the next exact windows.
    pub schedule: Schedule,
    /// What the app records: "did it at <instant>" and the count.
    pub measurable: String,
    /// True when it costs real money (a gemstone, a paid rite, a large gift): such a
    /// remedy is never offered first and sits behind "Ways that cost money".
    pub costs_money: bool,
    /// Where it comes from: keys of [`crate::sources`].
    pub source_keys: Vec<&'static str>,
}

/// Words in a remedy's English title that mean it costs real money: a paid rite, a
/// precious metal, a cow, school fees, a wedding. Food, a lamp, a blanket, a small
/// vessel and footwear are small gifts and do not count.
const PAID_WORDS: &[&str] = &[
    "gold", "silver", "a cow (", "gift of a cow", "school fees", "schooling", "full rite", "homa", "pratishtha", "shraddha", "bali",
    "kanyadana", "kumbha", "abhisheka", "pooja", "puja", "shanti", "brahmin feeding", "feeding brahmins", "a poor bride",
    "setu snana", "gaya", "a rite the old texts name",
];

/// True when a remedy of `kind` titled `title_en` (its English title) costs real money.
pub fn costs_money(kind: RemedyKind, title_en: &str) -> bool {
    kind == RemedyKind::Gem || costs_money_title(title_en)
}

/// True when a remedy's English title names something that costs real money.
pub fn costs_money_title(title_en: &str) -> bool {
    let t = title_en.to_lowercase();
    PAID_WORDS.iter().any(|w| t.contains(w))
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
    /// The cause in the texts' words, from the house-duty table (expert pages only).
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
    /// The cause in plain words, for the person: what the period brings to which part
    /// of life and how heavily, never naming the astrology.
    pub plain: String,
    /// Remedies, karma first, then seva and discipline, daana, vrata, mantra, pooja;
    /// gems last as information.
    pub remedies: Vec<Remedy>,
    /// The graha's deity, day, metal and grain in one line, for the header.
    pub signature: String,
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
        domain: "health to guard, the hidden and inheritance",
        karaka: "Saturn",
        conduct: "complete the rites for the departed; visit a grave",
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

// ---------------------------------------------------------------------------------
// The plain dictionary: what the person reads.
// ---------------------------------------------------------------------------------

/// `(kind, title, why, effort, day, source)`.
pub type Row = (RemedyKind, &'static str, &'static str, &'static str, &'static str, &'static str);

const DHARMA: &str = "Dharma reading of the graha's signification";
const BPHS: &str = "BPHS, Graha shanti adhyaya";
const PURANA: &str = "Puranic graha-daana lists (Matsya, Yajnavalkya)";
const LAL: &str = "Lal Kitab";
const TEMPLE: &str = "Temple practice (Navagraha kshetras, Kerala and Tamil Nadu)";
const MUHURTA: &str = "Vrata tradition (Muhurta and Dharma Sindhu)";
/// The graha's own dictionary. Titles say what to do; the why says what it eases in
/// the person's life. No planet is named: the graha is the engine's, the life is theirs.
pub fn table(g: DashaLord) -> Vec<Row> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Karma, "Keep your word to those who depend on you", "Respect and standing are earned by reliability. Do what you said, when you said it.", "daily", "any day", DHARMA),
            (Karma, "Mend things with your father, or honour his memory", "A strained bond with your father blocks recognition; mending it opens doors with seniors too.", "this week", "Sunday", DHARMA),
            (Karma, "Take responsibility for one thing you have been blaming others for", "Confidence and standing grow with ownership and shrink with excuses.", "this week", "any day", DHARMA),
            (Seva, "Serve your father, or an elder man without family", "Eases friction with bosses, officials and anyone in authority over you.", "weekly", "Sunday", DHARMA),
            (Seva, "Help someone get a government paper or a right they are owed", "Clearing another's path with officials clears your own: approvals and paperwork move.", "once", "any day", DHARMA),
            (Discipline, "Be up before sunrise for 40 days", "Steadies energy, confidence and willpower; the sunrise time is computed for your place.", "40 days", "start Sunday", MUHURTA),
            (Discipline, "Twelve rounds of Suryanamaskar at dawn", "Done daily, it steadies heart, spine and will, and lifts low confidence.", "20 min · daily", "any day", TEMPLE),
            (Daana, "Give wheat or jaggery on a Sunday, by hand", "Eases trouble with authority, government and self-respect.", "once", "Sunday", PURANA),
            (Daana, "Give a copper vessel or red cloth to a man who works under the open sky", "Eases trouble with authority, father and reputation.", "once", "Sunday", PURANA),
            (Vrata, "Sunday fast: one meal, no salt, before sunset", "Kept for 12 Sundays, the classical fast for standing, health of the heart and the eyes.", "12 Sundays", "Sunday", BPHS),
            (Mantra, "Aditya Hridayam on Sundays", "Supports the discipline above; not a substitute for it.", "if you wish", "Sunday", TEMPLE),
            (Mantra, "Om hraam hreem hraum sah Suryaya namah · 7,000 in all", "The classical chant for confidence and recognition. A mala a day finishes it in two months.", "if you wish", "start Sunday", BPHS),
            (Pooja, "Arghya at sunrise: water offered from a copper vessel, facing east", "Optional. Listed so you know it is not hidden.", "not required", "any day", TEMPLE),
            (Pooja, "Shiva temple on a Sunday; Suryanar Kovil for the full rite", "Optional.", "not required", "Sunday", TEMPLE),
        ],
        DashaLord::Moon => vec![
            (Karma, "Care for your mother, in person, this week", "A restless mind and an unsettled home calm down when the bond with your mother is tended.", "weekly", "Monday", DHARMA),
            (Karma, "Say one true thing you have been holding in, gently", "Feelings held back build up until they flood; speaking early prevents the outburst.", "this week", "any day", DHARMA),
            (Karma, "Forgive one person in your own mind, without telling them", "Grudges weigh on sleep and mood; letting one go lightens all of them.", "once", "full moon", DHARMA),
            (Seva, "Serve women and children who have no one", "Calms worry and brings goodwill from people around you. An hour of help, not money alone.", "weekly", "Monday", DHARMA),
            (Seva, "Feed and comfort someone who is grieving", "Comforting another steadies your own mind.", "once", "any day", DHARMA),
            (Discipline, "Fixed sleep and waking times for 40 days", "A steady routine is the best medicine for mood swings and poor sleep.", "40 days", "start Monday", MUHURTA),
            (Discipline, "Ten minutes of still breathing each night, no screen after it", "A quiet close to the day settles an anxious mind.", "10 min · nightly", "any day", DHARMA),
            (Daana, "Give rice, milk or white cloth on a Monday", "Calms worry, mood swings and trouble at home.", "once", "Monday", PURANA),
            (Daana, "Give silver (even a small coin) or a conch to a temple or a woman elder", "Brings peace of mind and the blessing of the mother.", "once", "Monday", PURANA),
            (Vrata, "Somavara vrata: Mondays, one meal after sunset, for 16 Mondays", "Kept for Shiva, the classical fast for a shaken mind.", "16 Mondays", "Monday", MUHURTA),
            (Mantra, "Om shraam shreem shraum sah Chandraya namah · 11,000 in all", "The classical chant for a calm mind.", "if you wish", "start Monday", BPHS),
            (Mantra, "Shiva Panchakshari on Mondays", "Optional support for peace of mind.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Abhisheka with milk on a Monday", "Optional.", "not required", "Monday", TEMPLE),
            (Pooja, "Gauri or Parvati temple; Thingalur for the full rite", "Optional.", "not required", "Monday", TEMPLE),
        ],
        DashaLord::Mars => vec![
            (Karma, "End one quarrel on your own initiative", "Anger and conflict ease when courage is used for peace.", "this week", "Tuesday", DHARMA),
            (Karma, "Stand up, once, for someone weaker than you", "Turns restless energy into protection; quarrels around you drop.", "this week", "any day", DHARMA),
            (Karma, "Repay a brother or a sibling what you owe, in money or in time", "Debts between siblings feed disputes over money and property.", "this month", "Tuesday", DHARMA),
            (Seva, "Donate blood, or serve those who protect others", "Brings safety on the road and at work, and cools fights.", "once", "any day", DHARMA),
            (Seva, "Cook and serve a hot meal to labourers or soldiers", "Cools heat and haste, and eases property and land troubles.", "once", "Tuesday", TEMPLE),
            (Discipline, "No anger spoken aloud for 40 days", "Restraint is the surest remedy for quarrels and their costs.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Hard physical work or sport, 30 minutes, until you sweat", "Energy not spent in effort comes out as temper.", "30 min · daily", "any day", DHARMA),
            (Daana, "Give red lentils on a Tuesday", "Cools anger and quarrels, brings safety on the road; helps property matters.", "once", "Tuesday", PURANA),
            (Daana, "Give red cloth, copper or red sweets to a young man who serves", "Eases disputes, debts and land troubles.", "once", "Tuesday", PURANA),
            (Vrata, "Tuesday fast with one meal of plain food; Mangala Gauri for women", "Kept 21 Tuesdays, for courage and for a calmer marriage.", "21 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Tuesdays", "Optional support for courage and protection.", "if you wish", "Tuesday", TEMPLE),
            (Mantra, "Om kraam kreem kraum sah Bhaumaya namah · 10,000 in all", "The classical chant for courage and calm.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Subrahmanya or Hanuman temple on a Tuesday", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Vaitheeswaran Koil for the full rite", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
        DashaLord::Mercury => vec![
            (Karma, "Speak less and only what is true for a week", "Misunderstandings, broken deals and gossip ease when speech is measured.", "7 days", "Wednesday", DHARMA),
            (Karma, "Settle one account you have let slide, to the rupee", "A loose ledger brings money mix-ups; a clean one ends them.", "this week", "Wednesday", DHARMA),
            (Karma, "Return a borrowed book, tool or favour", "Keeps exchanges even, so trade and talks flow.", "this week", "any day", LAL),
            (Seva, "Teach a child or an adult learner for an hour", "Sharpens your own studies and thinking.", "weekly", "Wednesday", DHARMA),
            (Seva, "Help a small trader or a student with paperwork or accounts", "Clears confusion in your own business and papers.", "once", "any day", DHARMA),
            (Discipline, "No gossip, no idle scrolling, for 40 days", "Scattered attention is the root of forgetfulness and nervous worry.", "40 days", "start Wednesday", DHARMA),
            (Discipline, "Write one page by hand every morning", "Writing daily sharpens memory, speech and focus.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give green moong on a Wednesday", "Clears confusion in talk, studies, trade and paperwork.", "once", "Wednesday", PURANA),
            (Daana, "Give green cloth, a bronze vessel or books to a student", "Helps exams, contracts and business.", "once", "Wednesday", PURANA),
            (Vrata, "Wednesday fast, one meal, with Vishnu in mind", "For clear thinking, studies and trade.", "7 Wednesdays", "Wednesday", MUHURTA),
            (Mantra, "Vishnu Sahasranama on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Mantra, "Om braam breem braum sah Budhaya namah · 9,000 in all", "The classical chant for intellect and speech.", "if you wish", "start Wednesday", BPHS),
            (Pooja, "Vishnu or Krishna temple on a Wednesday; tulasi water", "Optional.", "not required", "Wednesday", TEMPLE),
            (Pooja, "Thiruvenkadu for the full rite", "Optional.", "not required", "Wednesday", TEMPLE),
        ],
        DashaLord::Jupiter => vec![
            (Karma, "Honour a teacher you have neglected", "Gratitude to teachers brings back good advice and lucky breaks.", "this week", "Thursday", DHARMA),
            (Karma, "Keep one promise made to a child", "Faith kept with children brings their blessings and good news about them.", "this week", "any day", DHARMA),
            (Karma, "Give advice only when asked, and then fully", "Luck and respect shrink when wisdom is forced or withheld.", "40 days", "any day", DHARMA),
            (Seva, "Feed students, or fund one student's books", "Knowledge given freely returns as growth in money and status.", "once", "Thursday", DHARMA),
            (Seva, "Serve a priest, a teacher or an elder scholar without being asked", "Brings guidance at the moment you need it.", "weekly", "Thursday", TEMPLE),
            (Discipline, "Read scripture or a serious book daily for 40 days", "Steadies judgement, so money and family decisions go right.", "40 days", "start Thursday", DHARMA),
            (Discipline, "No lie, not even a small one, for 40 days", "Truthfulness is the fastest way to win back trust and luck.", "40 days", "any day", DHARMA),
            (Daana, "Give turmeric or chana dal on a Thursday", "Brings back luck, savings and the blessing of children.", "once", "Thursday", PURANA),
            (Daana, "Give yellow cloth, a book or (if you can) a little gold to a teacher", "For growth in money, marriage and wisdom.", "once", "Thursday", PURANA),
            (Vrata, "Thursday fast: yellow food once, no salt", "Kept 16 Thursdays, for marriage, children and wisdom.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Om graam greem graum sah Gurave namah · 19,000 in all", "The classical chant for wisdom and good fortune.", "if you wish", "start Thursday", BPHS),
            (Mantra, "Dakshinamurthy stotra on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Dakshinamurthy or Vishnu temple on a Thursday", "Optional.", "not required", "Thursday", TEMPLE),
            (Pooja, "Alangudi for the full rite", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        DashaLord::Venus => vec![
            (Karma, "Keep a promise made in a relationship", "Love and trust grow back when promises are kept.", "this week", "Friday", DHARMA),
            (Karma, "Say thank you, specifically, to the person who makes your home", "Naming the comfort others give brings warmth back into the home.", "this week", "Friday", DHARMA),
            (Karma, "Make one thing beautiful: a corner, a meal, a letter", "Beauty made, not bought, restores joy and attraction.", "this week", "any day", DHARMA),
            (Seva, "Serve women, artists or the sick with your skill", "Brings grace and ease into your own relationships.", "weekly", "Friday", DHARMA),
            (Seva, "Help a couple who are struggling, or a girl's education", "Harmony given to others returns to your own marriage.", "once", "Friday", DHARMA),
            (Discipline, "Simplify one indulgence for 40 days", "Overspending and indulgence drain love and money alike.", "40 days", "start Friday", DHARMA),
            (Discipline, "Clean clothes, clean room, clean words, for 40 days", "Small refinements bring back comfort and attraction.", "40 days", "any day", LAL),
            (Daana, "Give curd or white sweets on a Friday", "Restores love, comfort and harmony in marriage.", "once", "Friday", PURANA),
            (Daana, "Give white cloth, rice, ghee or silver to a woman in need", "For a happier marriage and comfort at home.", "once", "Friday", PURANA),
            (Vrata, "Friday fast with white food once; Vaibhava Lakshmi vrata", "For marriage, comfort and the arts.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Om draam dreem draum sah Shukraya namah · 16,000 in all", "The classical chant for love and comfort.", "if you wish", "start Friday", BPHS),
            (Mantra, "Sri Suktam or Lakshmi Ashtottara on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi or Devi temple on a Friday", "Optional.", "not required", "Friday", TEMPLE),
            (Pooja, "Kanjanur for the full rite", "Optional.", "not required", "Friday", TEMPLE),
        ],
        DashaLord::Saturn => vec![
            (Karma, "Finish the work you have postponed longest", "Delays lift when duty is completed rather than avoided.", "20 min · daily", "any day", DHARMA),
            (Karma, "Pay every worker and helper on time, in full, this month", "An unpaid wage is the surest way to invite delays and losses.", "this month", "Saturday", DHARMA),
            (Karma, "Visit someone old who is alone", "Eases loneliness and heaviness in your own life.", "weekly", "Saturday", DHARMA),
            (Seva, "Serve the old or the labouring on Saturdays", "Eases delays, debts and tiredness at work. An hour of real help.", "1 h · Saturdays", "Saturday", DHARMA),
            (Seva, "Clean a public place, a temple floor or a shared stair, yourself", "Humble work done without witness lightens a heavy period.", "once", "Saturday", TEMPLE),
            (Discipline, "Rise before sunrise for 40 days", "Discipline is the remedy for delay that needs no one else.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Walk instead of ride for one errand a day", "Builds the patience a slow period asks for.", "daily", "any day", LAL),
            (Daana, "Give black til or a blanket on a Saturday", "Eases delays, debts and heavy duty; give by hand to a person who needs it.", "once", "Saturday", PURANA),
            (Daana, "Give iron (a tool, a pan) or mustard oil to a labourer", "Eases job troubles, chronic aches and slow progress.", "once", "Saturday", PURANA),
            (Daana, "Feed crows and a black dog on Saturdays", "Eases delays, debts and tiredness at work.", "Saturdays", "Saturday", TEMPLE),
            (Vrata, "Saturday fast, one meal after sunset, for 19 Saturdays", "The classical fast through a long testing period.", "19 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays", "Optional. Supports the discipline above.", "if you wish", "Saturday", TEMPLE),
            (Mantra, "Om praam preem praum sah Shanaischaraya namah · 23,000 in all", "The classical chant for patience and protection in hard times.", "if you wish", "start Saturday", BPHS),
            (Pooja, "Til-oil lamp to Hanuman on a Saturday", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Thirunallar for the full rite", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Rahu => vec![
            (Karma, "Stop one shortcut or deception you have been running", "Shortcuts are where sudden trouble comes from; dropping one closes the door on it.", "this week", "any day", DHARMA),
            (Karma, "Tell the one person you have been avoiding the plain truth", "Clears the fog and suspicion between people.", "this week", "Saturday", DHARMA),
            (Karma, "Leave one obsession alone for a week: the feed, the rival, the fear", "Anxiety feeds on appetite without end; starving it calms the mind.", "7 days", "any day", DHARMA),
            (Seva, "Serve the outcast, the sick without family, the foreigner", "Eases confusion, fear and trouble from strangers.", "weekly", "Saturday", DHARMA),
            (Seva, "Help a migrant worker or a stranger far from home", "Opens foreign and unusual opportunities, safely.", "once", "any day", DHARMA),
            (Discipline, "No intoxicants for 40 days", "Confusion and bad decisions feed on intoxication.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Eat only what you have cooked or seen cooked, for 21 days", "Guards against hidden harm and sudden illness.", "21 days", "any day", LAL),
            (Daana, "Give mustard oil or blue cloth on a Saturday", "Clears confusion, anxiety and sudden setbacks.", "once", "Saturday", PURANA),
            (Daana, "Give a coconut or a dark blanket; let a coconut float in running water", "Eases fear, cheating and sudden trouble.", "once", "Saturday", LAL),
            (Vrata, "Saturday fast with Durga in mind, 18 Saturdays", "For protection from fear and sudden trouble.", "18 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Om bhraam bhreem bhraum sah Rahave namah · 18,000 in all", "The classical chant against fear and confusion.", "if you wish", "start Saturday", BPHS),
            (Mantra, "Durga Saptashati or Durga mantra on Saturdays", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga temple on a Saturday evening", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Thirunageswaram for the full rite", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Ketu => vec![
            (Karma, "Finish something you abandoned", "Drift and loss of interest end when one thing is completed.", "this week", "any day", DHARMA),
            (Karma, "Apologise for one thing you walked away from", "Mends the silent breaks that leave you feeling cut off.", "this week", "Tuesday", DHARMA),
            (Karma, "Give away ten things you do not use", "Letting go on purpose prevents losses that come uninvited.", "this week", "any day", DHARMA),
            (Seva, "Feed and care for stray dogs, or serve a renunciate", "Eases sudden losses and a sense of emptiness.", "weekly", "Tuesday", DHARMA),
            (Seva, "Sit with the very ill or those in hospice care, in silence", "Brings clarity and calm in a confusing time.", "once", "any day", DHARMA),
            (Discipline, "Live simply for 40 days: one meal plain, one hour of silence", "Simplicity turns restlessness into insight.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Ten minutes of meditation on the breath, daily", "Gives direction when you feel lost.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give a blanket or sesame", "Eases losses, drift and lack of direction.", "once", "Tuesday", PURANA),
            (Daana, "Give a multicoloured cloth or a lamp to a temple of Ganesha", "Clears obstacles and sudden breaks.", "once", "Tuesday", TEMPLE),
            (Vrata, "Tuesday fast with Ganesha in mind; Sankashti Chaturthi", "For the removal of obstacles.", "7 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Ganesha mantra · Om gam Ganapataye namah", "Optional support.", "if you wish", "any day", TEMPLE),
            (Mantra, "Om sraam sreem sraum sah Ketave namah · 17,000 in all", "The classical chant for clarity and release.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Ganesha temple on a Tuesday or Chaturthi", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Keezhperumpallam for the full rite", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
    }
}

/// Remedies for a life area, added when the matter is known.
pub fn life_table(area_name: &str) -> Vec<Row> {
    use RemedyKind::*;
    match area_name {
        "Marriage" => vec![
            (Karma, "Treat your partner's family as your own this month", "Marriages are strained most often through the in-laws; this is where it mends.", "this month", "any day", DHARMA),
            (Karma, "Settle an old hurt with a former partner in your own mind", "A bond left bleeding weakens the next one.", "once", "Friday", DHARMA),
            (Seva, "Help with a wedding you are not part of; give to a girl's marriage", "Marriage given to another returns.", "once", "Friday", TEMPLE),
            (Vrata, "Swayamvara Parvathi or Mangala Gauri vrata on Tuesdays or Fridays", "The classical fast for marriage and its harmony.", "16 weeks", "Tuesday", MUHURTA),
            (Mantra, "Swayamvara Parvathi mantra, or Katyayani for women, Shiva for men", "Optional support, from the marriage tradition.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Umamaheshwara pooja; a visit to a Parvati or Subrahmanya temple as a couple", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Career" => vec![
            (Karma, "Do the part of your work you have been avoiding, first thing", "The avoided task is the blocked promotion.", "daily", "any day", DHARMA),
            (Karma, "Give your superior the respect you expect from those under you", "Authority flows both ways; respect given upward is returned.", "this week", "Sunday", DHARMA),
            (Seva, "Mentor someone junior without credit", "Lifting others at work raises your own standing.", "weekly", "any day", DHARMA),
            (Discipline, "Twelve rounds of Suryanamaskar before work", "Meet the day with energy before you meet your work.", "15 min · daily", "Sunday", TEMPLE),
            (Vrata, "Sunday fast for recognition, or Saturday fast for stability at work", "Choose by what you need: to be seen, or to be secure.", "12 weeks", "Sunday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays; Aditya Hridayam on Sundays", "Optional support.", "if you wish", "Saturday", TEMPLE),
        ],
        "Wealth" => vec![
            (Karma, "Write down every rupee in and out for 40 days", "Money leaks where it is not watched.", "5 min · daily", "any day", DHARMA),
            (Karma, "Repay the oldest small debt, and lend nothing for 40 days", "Gains come to one who does not scatter.", "40 days", "any day", LAL),
            (Seva, "Give one tenth of a windfall away before you spend it", "Wealth shared at once is wealth that stays.", "when it comes", "any day", DHARMA),
            (Seva, "Annadanam: feed people on a day of the month, however few", "Food given is the oldest remedy for money.", "monthly", "Friday", TEMPLE),
            (Vrata, "Vaibhava Lakshmi vrata on Fridays; Ekadashi fast", "The fast for steady wealth.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Sri Suktam or Kanakadhara stotra on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi-Kubera pooja; a lamp at the front door at dusk", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Health" => vec![
            (Karma, "Keep the appointment, take the medicine, finish the course", "The first remedy is the one the doctor gave.", "as prescribed", "any day", DHARMA),
            (Discipline, "Sleep by ten, up before sunrise, for 40 days", "The body heals on a steady rhythm.", "40 days", "any day", DHARMA),
            (Discipline, "A plain diet without stimulants for 21 days", "Illness feeds on excess.", "21 days", "any day", LAL),
            (Seva, "Give medicines or a day of care to someone ill and poor", "Health given returns as health.", "once", "any day", DHARMA),
            (Mantra, "Mahamrityunjaya mantra, 108 a day", "The classical mantra for health and long life.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Dhanvantari or Shiva abhisheka; Subrahmanya on Tuesdays for recovery", "Optional.", "not required", "Monday", TEMPLE),
        ],
        "Family" => vec![
            (Karma, "Eat one meal a day with your family, phones away", "The shared meal is what holds a family together.", "daily", "any day", DHARMA),
            (Karma, "Call or visit your mother; keep her comfort before your plans", "Peace at home begins with the mother's ease.", "weekly", "Monday", DHARMA),
            (Seva, "Serve your elders and your in-laws without being asked", "Family given to returns as family.", "weekly", "any day", DHARMA),
            (Vrata, "Pitru tarpana on Amavasya; Somavara vrata", "The ancestors steady the home.", "monthly", "Monday", MUHURTA),
            (Mantra, "Shiva-Parvati or Gauri mantra on Mondays", "Optional support.", "if you wish", "Monday", TEMPLE),
            (Pooja, "A lamp at home at dusk, lit by the family together", "Optional, and the simplest.", "not required", "any day", TEMPLE),
        ],
        "Education" => vec![
            (Karma, "Study the hardest subject first, at the same hour daily", "A fixed hour is the discipline that wins exams.", "daily", "Wednesday", DHARMA),
            (Seva, "Teach a junior what you have learned, free", "Knowledge given is knowledge fixed.", "weekly", "Wednesday", DHARMA),
            (Discipline, "Gayatri at sunrise, 11 times, for 40 days", "The classical discipline for the mind and the voice.", "40 days", "any day", TEMPLE),
            (Daana, "Give books or school fees for one child", "Learning given opens your own.", "once", "Thursday", PURANA),
            (Mantra, "Saraswati or Hayagriva mantra on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Pooja, "Saraswati pooja before an exam; Dakshinamurthy on Thursdays", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        "Property" => vec![
            (Karma, "Repair what is broken at home before buying anything new", "A neglected home blocks a new one.", "this month", "Tuesday", DHARMA),
            (Karma, "Clear the north-east corner of the home and keep it light", "Vastu's first rule for a settled home.", "once", "any day", TEMPLE),
            (Seva, "Help someone keep or find a roof: rent, a repair, a reference", "Home given returns as home.", "once", "any day", DHARMA),
            (Daana, "Give red lentils and a copper vessel on a Tuesday", "The classical gift for land and property matters.", "once", "Tuesday", PURANA),
            (Mantra, "Hanuman Chalisa on Tuesdays; Bhumi sukta before a deal", "Optional support.", "if you wish", "Tuesday", TEMPLE),
            (Pooja, "Vastu or Bhumi pooja before building; Ganesha before signing", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
        "Foreign" => vec![
            (Karma, "Set your papers in order: passport, documents, debts, before you plan the journey", "Disorder at home follows you abroad.", "this month", "any day", DHARMA),
            (Seva, "Help a traveller or a stranger far from home", "Serving travellers opens the road for you.", "once", "Saturday", DHARMA),
            (Daana, "Give footwear or an umbrella to someone who walks far", "The traveller's gift.", "once", "Wednesday", TEMPLE),
            (Discipline, "No intoxicants and plain food for 21 days before a long journey", "Keeps the journey clear of confusion and mishap.", "21 days", "any day", LAL),
            (Mantra, "Hanuman or Ganesha before setting out; Durga for protection", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga pooja on a Saturday; a lamp before the journey", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        "Children" => vec![
            (Karma, "Keep every promise made to a child this month", "Faith kept with children brings their blessings.", "this month", "Thursday", DHARMA),
            (Seva, "Feed children, or pay one child's schooling", "Children served bring children's joy.", "once", "Thursday", DHARMA),
            (Seva, "Serve a teacher or an old scholar", "The teacher's blessing is the classical blessing for children.", "weekly", "Thursday", DHARMA),
            (Vrata, "Putrada Ekadashi; Thursday fast", "The classical fast for children.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Santana Gopala mantra on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Santana Gopala or Krishna pooja; Alangudi for the full rite", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        _ => Vec::new(),
    }
}

/// The remedy's deity, day, metal, grain and colour, in one line. No planet named.
pub fn signature(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Shiva and Aditya · Sunday · copper · wheat · red and saffron",
        DashaLord::Moon => "Parvati · Monday · silver · rice · white",
        DashaLord::Mars => "Subrahmanya and Hanuman · Tuesday · copper · red lentil · red",
        DashaLord::Mercury => "Vishnu · Wednesday · bronze · green moong · green",
        DashaLord::Jupiter => "Dakshinamurthy · Thursday · gold · chana dal · yellow",
        DashaLord::Venus => "Lakshmi · Friday · silver · rice and curd · white",
        DashaLord::Saturn => "Hanuman · Saturday · iron · black til · black and blue",
        DashaLord::Rahu => "Durga · Saturday · lead · mustard and urad · smoke blue",
        DashaLord::Ketu => "Ganesha · Tuesday · sesame · many colours",
    }
}

/// The six base rows per graha (karma, seva, discipline, daana, mantra, pooja) in the
/// texts' own terms, kept from the first version: Lal Kitab and temple practice for the
/// ritual items, dharma texts for the conduct items. They name the graha, so they feed
/// `why_expert` only; the person reads [`table`].
fn expert_table(g: DashaLord) -> Vec<(RemedyKind, &'static str, &'static str, &'static str)> {
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
struct PlanRow {
    kind: RemedyKind,
    planet: DashaLord,
    rule_id: String,
    /// Plain title, read by the person.
    title: String,
    /// Plain why, read by the person.
    why: String,
    /// The texts' column, expert pages only.
    why_expert: String,
    source: &'static str,
    effort: &'static str,
    day: &'static str,
    cadence: String,
    count: Option<u32>,
    duration_days: Option<u32>,
    cue: Cue,
    /// Severity the fit is scaled by (the cause's, or the dosha's).
    weight: f64,
    /// Added to the fit: a shade less for the alternatives and the matter's own rows.
    bonus: f64,
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

/// `s` when it reads plainly, otherwise the plain fallback (the texts' wording then
/// lives on in `why_expert`).
/// Words that frighten: a remedy the person reads never carries them (the texts' own
/// words stay in `why_expert`).
const FRIGHTENING: &[&str] = &["curse", "spirit", "ghost", "demon", "black magic", "evil eye", "(v)", "(k)"];

fn plain_or(s: String, fallback: &str) -> String {
    let low = s.to_lowercase();
    if jargon(&s).is_none() && !FRIGHTENING.iter().any(|w| low.contains(w)) {
        s
    } else {
        fallback.to_string()
    }
}

/// Cadence, count and vow length of a plain dictionary row, read from its effort.
fn cadence_of(effort: &str, weekday: &str) -> (String, Option<u32>, Option<u32>) {
    let e = effort.to_ascii_lowercase();
    // "40 days", "21 days", "7 days"
    for n in [40u32, 43, 21, 7] {
        if e.contains(&format!("{n} days")) {
            return (format!("{n} days"), Some(n), Some(n));
        }
    }
    // "12 Sundays", "16 weeks", "19 Saturdays"
    let mut words = e.split_whitespace();
    if let (Some(first), Some(second)) = (words.next(), words.next()) {
        if let Ok(n) = first.parse::<u32>() {
            if second.ends_with("days") || second.starts_with("week") {
                return (format!("weekly on {weekday}"), Some(n), Some(n * 7));
            }
        }
    }
    if e.contains("daily") || e.contains("nightly") {
        ("daily".to_string(), None, None)
    } else if e.contains("weekly") || e.contains("saturdays") || e.contains("week") && !e.contains("this week") {
        (format!("weekly on {weekday}"), None, None)
    } else if e.contains("monthly") {
        ("monthly".to_string(), None, None)
    } else if e.contains("this month") {
        ("once".to_string(), None, Some(30))
    } else if e.contains("this week") {
        ("once".to_string(), None, Some(7))
    } else {
        ("once".to_string(), None, None)
    }
}

fn cue_of(kind: RemedyKind, title: &str) -> Cue {
    match kind {
        RemedyKind::Karma | RemedyKind::Vrata | RemedyKind::Vastra | RemedyKind::Prakriti => {
            Cue::FirstHora
        }
        RemedyKind::Discipline => {
            let t = title.to_ascii_lowercase();
            if t.contains("sunrise") || t.contains("dawn") || t.contains("before the sun") {
                Cue::Brahma
            } else {
                Cue::FirstHora
            }
        }
        RemedyKind::Seva | RemedyKind::Daana | RemedyKind::Anna | RemedyKind::Jeeva | RemedyKind::Gem => {
            Cue::DayHora
        }
        RemedyKind::Mantra => Cue::Hora,
        RemedyKind::Pooja => Cue::PlanetTithi,
    }
}

fn measurable_of(kind: RemedyKind, g: DashaLord, cadence: &str, count: Option<u32>) -> String {
    let day = weekday_name(g);
    match kind {
        RemedyKind::Karma => format!("Log 'did it at <instant>' once a day; the ledger counts days kept over days planned, from the next {g:?} sunrise hora."),
        RemedyKind::Seva => format!("Log the hour served with its start instant; one planned per {day}."),
        RemedyKind::Anna | RemedyKind::Jeeva => "Log the instant the food was given, by your own hand, and to whom.".to_string(),
        RemedyKind::Prakriti => "Log each time with its instant; days kept over days planned.".to_string(),
        RemedyKind::Discipline | RemedyKind::Vastra => format!(
            "Log each day kept at its sunrise cue; {} planned; one missed day does not void it (repair in the next hora, no shame).",
            count.map(|n| n.to_string()).unwrap_or_else(|| cadence.to_string())
        ),
        RemedyKind::Vrata => format!(
            "Log each day kept at its sunrise cue; {} planned.",
            count.map(|n| n.to_string()).unwrap_or_else(|| cadence.to_string())
        ),
        RemedyKind::Daana => "Log the instant you handed it over, by your own hand, and to whom; no amount is needed.".to_string(),
        RemedyKind::Mantra => "Tap once per mala (108) inside the hora; the count is yours to log, never assumed.".to_string(),
        RemedyKind::Pooja => "Log the instant performed; optional, so nothing is planned against it.".to_string(),
        RemedyKind::Gem => "Nothing to log; information only.".to_string(),
    }
}

/// Plan rows from a plain dictionary ([`table`], [`crate::daana::alternatives`],
/// [`life_table`]): the person's words, the texts' column beside them, a cadence read
/// from the effort and the cue its kind is timed to.
fn dictionary_rows(
    g: DashaLord,
    rows: Vec<Row>,
    prefix: &str,
    severity: f64,
    bonus: f64,
    debilitated: bool,
) -> Vec<PlanRow> {
    let weekday = weekday_name(g);
    let expert = expert_table(g);
    rows.into_iter()
        .map(|(kind, title, why, effort, day, source)| {
            let (cadence, count, duration_days) = cadence_of(effort, weekday);
            let mut why_expert = expert
                .iter()
                .find(|r| r.0 == kind)
                .map(|r| r.2.to_string())
                .unwrap_or_else(|| format!("{g:?}: {source}."));
            if kind == RemedyKind::Daana && prefix == "GRAHA" {
                let s = graha_shanti(g);
                why_expert.push_str(&format!(
                    " The text's item: {}, with an image of {}, samidha {}, colour {}, naivedya {}; a modern equivalent is {}. Given on {weekday} in {g:?}'s hora, after sunrise and before sunset, never at night, by your own hand, to a person who needs it. [{GRAHA_SHANTI_SOURCE}]",
                    s.daana, s.image, s.samidha, s.colour, s.naivedya, s.practical
                ));
            }
            PlanRow {
                kind,
                planet: g,
                rule_id: format!("KS-R-{prefix}-{g:?}-{}", kind.rung()),
                title: title.to_string(),
                why: why.to_string(),
                why_expert,
                source,
                effort,
                day,
                measurable: measurable_of(kind, g, &cadence, count),
                cadence,
                count,
                duration_days,
                cue: cue_of(kind, title),
                weight: severity
                    + if debilitated && kind == RemedyKind::Daana {
                        0.1
                    } else {
                        0.0
                    },
                bonus,
            }
        })
        .collect()
}

fn planet_rows(
    g: DashaLord,
    severity: f64,
    debilitated: bool,
    functional_benefic: bool,
    weak: bool,
) -> Vec<PlanRow> {
    let day = weekday_name(g);
    // The graha's own dictionary, then food, creatures, water and colour.
    let mut rows = dictionary_rows(g, table(g), "GRAHA", severity, 0.0, debilitated);
    rows.extend(dictionary_rows(
        g,
        crate::daana::alternatives(g),
        "ALT",
        severity,
        -0.01,
        debilitated,
    ));
    // Vrata, with the tradition's count
    let v = vrata_row(g);
    let (cadence, duration) = match v.count {
        Some(n) => (format!("weekly on {day}"), Some(n * 7)),
        None => ("43 days".to_string(), Some(43)),
    };
    rows.push(PlanRow {
        kind: RemedyKind::Vrata,
        planet: g,
        rule_id: format!("KS-R-VRATA-{g:?}"),
        title: match v.count {
            Some(n) => format!("A simple fast on {n} {day}s in a row"),
            None => "A simple fast for 43 days, the way your family keeps it".to_string(),
        },
        why: format!(
            "A vow of simplicity for the day, not a treatment: eat if you are unwell. The day runs from sunrise at your place.{}",
            match v.alternate {
                Some(a) => format!(" Some families keep {a} instead."),
                None => String::new(),
            }
        ),
        why_expert: format!(
            "{}{}: {}. The day runs from the true sunrise at your place; the break (sunset or moonrise) is a computed instant.",
            v.name,
            match v.count {
                Some(n) => format!(", {n} {day}s"),
                None => String::new(),
            },
            v.rule
        ),
        source: VRATAS_SOURCE,
        effort: "one day a week",
        day,
        cadence,
        count: v.count,
        duration_days: duration,
        cue: Cue::FirstHora,
        weight: severity,
        bonus: 0.0,
        measurable: format!(
            "Log each {day} kept at its sunrise cue; {} planned.",
            v.count
                .map(|n| n.to_string())
                .unwrap_or_else(|| "43 days".into())
        ),
    });
    // Full japa dose
    let m = mantra_dose(g);
    let days = m.count.div_ceil(216);
    rows.push(PlanRow {
        kind: RemedyKind::Mantra,
        planet: g,
        rule_id: format!("KS-R-JAPA-{g:?}"),
        title: format!("The full chant: {} repetitions in all", m.count),
        why: format!(
            "At two rounds of 108 a day it ends in {days} days. Optional, never required; the full count is shown so you know the whole of it rather than a token eleven."
        ),
        why_expert: format!(
            "{g:?} beeja japa. The count used in temple practice is {} (×{KALI_MULTIPLIER} in Kali Yuga by practice = {}){}. Beeja: {}. Vedic rk: {}. Hymn: {}. Purascharana ladder: homa a tenth, tarpana a hundredth, marjana a thousandth, brahmana bhojana a ten-thousandth of the count. At two malas a day the count ends in {days} days; begin on {day} in {g:?}'s hora in Shukla paksha, the Moon in {g:?}'s star or Pushya.",
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
        day,
        cadence: "daily".to_string(),
        count: Some(m.count),
        duration_days: Some(days),
        cue: Cue::Hora,
        weight: severity,
        bonus: 0.0,
        measurable: format!(
            "Tap once per mala (108); {} planned; the end date follows from your malas per day.",
            m.count
        ),
    });
    // Gem: information only, gated
    let gr = gem_row(g);
    let allowed = functional_benefic && weak;
    rows.push(PlanRow {
        kind: RemedyKind::Gem,
        planet: g,
        rule_id: "KS-R-GEM-01".to_string(),
        title: format!("{} (information only)", gr.gem),
        why: if allowed {
            format!(
                "The old texts allow this stone for you: {}, set in {}, worn on the {}. Only with a qualified astrologer's advice; never necessary. It is a purchase, and no claim is made for it; listed for information.",
                gr.gem, gr.metal, gr.finger
            )
        } else {
            "No gem for you: by the old texts' own logic a stone here would strengthen what already presses on you. A stone is never necessary; the steps above are the remedy. Listed only so nothing is hidden.".to_string()
        },
        why_expert: if allowed {
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
        day,
        cadence: "once".to_string(),
        count: None,
        duration_days: None,
        cue: Cue::DayHora,
        weight: 0.0,
        bonus: 0.0,
        measurable: "Nothing to log; information only.".to_string(),
    });
    rows
}

fn house_rows(g: DashaLord, house: u8, severity: f64) -> Vec<PlanRow> {
    let d = house_duty(house);
    let day = weekday_name(g);
    vec![
        PlanRow {
            kind: RemedyKind::Karma,
            planet: g,
            rule_id: format!("KS-R-DUTY-{house:02}"),
            title: plain_or(crate::plain::cap(d.conduct), "Do the duty you have been putting off, first thing each day"),
            why: plain_or(
                format!("This is the duty behind {}: the trouble there is this duty left undone, and doing it is the remedy.", d.domain),
                "The trouble here is a duty left undone; doing it is the remedy.",
            ),
            why_expert: format!(
                "The {house}th house is {}; its karaka is {}. The texts read the affliction as this duty left undone, and the remedy is the duty itself: {}.",
                d.domain, d.karaka, d.conduct
            ),
            source: HOUSE_DUTIES_SOURCE,
            effort: "daily",
            day: "any day",
            cadence: "daily".to_string(),
            count: None,
            duration_days: None,
            cue: Cue::FirstHora,
            weight: severity,
            bonus: 0.0,
            measurable: format!("Log 'did it at <instant>' once a day from the next {g:?} sunrise hora."),
        },
        PlanRow {
            kind: RemedyKind::Seva,
            planet: g,
            rule_id: format!("KS-R-DUTY-{house:02}-SEVA"),
            title: plain_or(format!("Serve {}", d.seva), "Give an hour of real help to someone who needs it"),
            why: plain_or(
                format!("The people {} depends on; an hour of real help on {day}, not money alone.", d.domain),
                "An hour of real help each week, not money alone.",
            ),
            why_expert: format!(
                "The people the {house}th house signifies ({}); an hour of real help on {day}, not money alone. Daana family: {}.",
                d.seva, d.daana
            ),
            source: HOUSE_DUTIES_SOURCE,
            effort: "1 h · weekly",
            day,
            cadence: format!("weekly on {day}"),
            count: None,
            duration_days: None,
            cue: Cue::DayHora,
            weight: severity,
            bonus: 0.0,
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
) -> Vec<PlanRow> {
    let f = dasha_formula(g);
    let day = weekday_name(g);
    let days_left = (until.ns_since(at) / NS_PER_DAY).clamp(1, 365) as u32;
    vec![
        PlanRow {
            kind: RemedyKind::Mantra,
            planet: g,
            rule_id: format!("KS-R-DASHA-{g:?}"),
            title: "A daily prayer of 108 while this hard stretch lasts".to_string(),
            why: format!(
                "The old texts' remedy for a stretch of life like this one; {days_left} days of it remain. Optional, never required."
            ),
            why_expert: format!(
                "{} (BPHS, for the {level} of {g:?}). The text's remedial measure for an unfavourable period of {g:?}: {}. Conduct equivalent: {}. The {level} began at {} and ends at {}; begin on {day} in {g:?}'s hora in Shukla paksha, and a 'finish within the period' plan has {days_left} days.",
                f.japa,
                f.japa,
                f.japa_conduct,
                fmt_utc(since),
                fmt_utc(until)
            ),
            source: DASHA_FORMULAE_SOURCE,
            effort: "daily · 108",
            day,
            cadence: "daily".to_string(),
            count: Some(108),
            duration_days: Some(days_left.min(43)),
            cue: Cue::Hora,
            weight: severity,
            bonus: 0.0,
            measurable: "Tap once per mala (108) in the hora; 108 a day planned until the period ends.".to_string(),
        },
        PlanRow {
            kind: RemedyKind::Daana,
            planet: g,
            rule_id: format!("KS-R-DASHA-{g:?}-DAANA"),
            title: plain_or(
                crate::plain::cap(f.daana_conduct),
                "Feed or help animals for 43 days",
            ),
            why: "The old texts name a gift for a hard stretch like this; this is the form of it that needs no purchase. Given by day, by your own hand.".to_string(),
            why_expert: format!(
                "{} (BPHS gift for the {level}). The text's gift for this period: {}. The conduct equivalent, so nothing requires a purchase: {}. Given on {day} in {g:?}'s hora by day.",
                f.daana, f.daana, f.daana_conduct
            ),
            source: DASHA_FORMULAE_SOURCE,
            effort: "43 days",
            day,
            cadence: "43 days".to_string(),
            count: Some(43),
            duration_days: Some(43),
            cue: Cue::DayHora,
            weight: severity,
            bonus: 0.0,
            measurable: "Log each day's feeding or the one gift with its instant; 43 planned.".to_string(),
        },
    ]
}

/// A dosha row before it becomes a [`PlanRow`]: kind, title, why, effort, cadence, count,
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
fn dosha_rows(d: &Dosha) -> Vec<PlanRow> {
    use RemedyKind::*;
    let g = d.planet;
    let day = weekday_name(g);
    let specs: Vec<Spec> = match d.rule_id {
        "KS-R-SHAPA-01" => vec![
            (Karma, "Visit or call your father or elders on a fixed day; keep the yearly remembrance day of the departed".into(), "A duty to ancestors left undone is the karmic domain of this curse; the conduct is the remedy.".into(), "weekly", "weekly on Sunday".into(), None, None, Cue::FirstHora),
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
            (Karma, "Complete the rites or the memory of a departed relative; visit a grave".into(), "Saturn and Ketu are the departed; the rite completed is the restitution.".into(), "this week", "once".into(), None, Some(7), Cue::FirstHora),
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
            |(kind, title, why, effort, cadence, count, duration_days, cue)| {
                let (plain_title, plain_why) = match kind {
                    Karma => (
                        "Do the duty your family has let slide: the visit, the rite, the promise",
                        "The old texts read this trouble as a duty left undone; doing it is the remedy.",
                    ),
                    Seva => (
                        "Give an hour of real help to the old or the labouring",
                        "An hour of real help each week, not money alone.",
                    ),
                    Vrata => (
                        "Keep a simple fast on the days the tradition gives",
                        "A vow of simplicity for the day, not a treatment: eat if you are unwell.",
                    ),
                    Daana => (
                        "Give food or a useful thing by day, by your own hand",
                        "Given by day, by your own hand, to someone who needs it.",
                    ),
                    Mantra => ("A prayer on the day given, if you wish", "Optional support."),
                    _ => (
                        "A rite the old texts name, if your family keeps it",
                        "Optional, never required. Listed so you know it is not hidden.",
                    ),
                };
                PlanRow {
                    kind,
                    planet: g,
                    rule_id: format!("{}-{}", d.rule_id, kind.rung()),
                    title: plain_or(title.clone(), plain_title),
                    why: plain_why.to_string(),
                    why_expert: format!(
                        "{title}. {why} [{}: {}]",
                        d.name,
                        d.factors.join("; ")
                    ),
                    source: d.source,
                    effort,
                    day,
                    cadence,
                    count,
                    duration_days,
                    cue,
                    weight: d.severity,
                    bonus: 0.0,
                    measurable: match kind {
                        Karma => {
                            "Log 'did it at <instant>' against the cue; days kept over days planned."
                                .to_string()
                        }
                        Seva => format!(
                            "Log the hour served with its start instant; one planned per {day}."
                        ),
                        Vrata => "Log each day kept at its cue; the count planned is the vow's."
                            .to_string(),
                        Daana => "Log the instant you handed it over and to whom.".to_string(),
                        Mantra => "Tap once per mala; the count is yours to log.".to_string(),
                        _ => "Log the instant performed; optional, nothing planned against it."
                            .to_string(),
                    },
                }
            },
        )
        .collect()
}

/// Rule ids keyed by the app's commitments must be unique: a second row with the same
/// id gets `-2`, a third `-3`.
fn unique_ids(rows: &mut [PlanRow]) {
    let mut seen: HashMap<String, u32> = HashMap::new();
    for r in rows.iter_mut() {
        let n = seen.entry(r.rule_id.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            r.rule_id = format!("{}-{}", r.rule_id, n);
        }
    }
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
    let rahu = rahu_longitude(spk, at).map_err(EventError::Spk)?;
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

const LEVELS: [&str; 3] = ["mahadasha", "antardasha", "pratyantar"];

/// The cause in plain words: which part of the running period presses (`level`, 0 the
/// long chapter), what it brings and to which part of life, and how heavily.
pub fn plain_cause(lord: DashaLord, level: Option<usize>, matter: Option<&str>, severity: f64) -> String {
    let to = matter.map(|m| format!(" to your {m}")).unwrap_or_default();
    let head = match (level, matter) {
        (Some(i), _) => format!(
            "The {} you are in brings {}{to}",
            crate::plain::level_noun(i),
            crate::plain::pressure(lord)
        ),
        (None, Some(m)) => format!(
            "From birth your {m} is open to {}",
            crate::plain::pressure(lord)
        ),
        (None, None) => "Nothing particular weighs on you".to_string(),
    };
    if severity < 0.25 {
        format!("{head}, but only lightly. Nothing serious is against you: your own effort is the remedy, and the list below is extra help.")
    } else {
        format!("{head}. The remedies below are chosen to ease exactly that.")
    }
}

fn into_remedy(
    r: PlanRow,
    karma: &KarmaVerdict,
    windows: Vec<RemedyWindow>,
) -> Remedy {
    let paid = costs_money(r.kind, &r.title);
    let fit = if r.kind == RemedyKind::Gem {
        0.0
    } else {
        (r.kind.base_fit() * (0.5 + 0.5 * r.weight.min(1.0)) + r.bonus).clamp(0.0, 1.0)
    };
    Remedy {
        kind: r.kind,
        rung: r.kind.rung(),
        rule_id: r.rule_id,
        title: r.title,
        why: r.why,
        why_expert: r.why_expert,
        source: r.source,
        fit,
        effort: r.effort,
        day: r.day,
        optional: r.kind.is_optional(),
        may_claim: may_claim(karma.class, karma.kind_in_play, r.kind.rung()),
        schedule: Schedule {
            cadence: r.cadence,
            count: r.count,
            duration_days: r.duration_days,
            windows,
        },
        measurable: r.measurable,
        costs_money: paid,
        source_keys: crate::sources::of_remedy(r.kind, r.source),
    }
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
    let matter = area.map(|a| a.name.to_ascii_lowercase());

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
    let plain = plain_cause(planet, level_idx, matter.as_deref(), severity);
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
    if let Some(a) = area {
        rows.extend(house_rows(planet, house, severity));
        // The matter's own remedies sit just under the graha's: same kind, a shade less fit.
        rows.extend(dictionary_rows(
            planet,
            life_table(a.name),
            "LIFE",
            severity,
            -0.03,
            debilitated,
        ));
    }
    if let (Some(level), Some(since), Some(until)) = (running_level, running_since, running_until) {
        if severity >= 0.5 {
            rows.extend(dasha_rows(planet, level, since, until, at, severity));
        }
    }
    for d in present.iter().take(3) {
        rows.extend(dosha_rows(d));
    }
    unique_ids(&mut rows);

    let mut remedies: Vec<Remedy> = rows
        .into_iter()
        .map(|r| {
            let windows = sched.pick(r.planet, r.cue);
            into_remedy(r, &karma, windows)
        })
        .collect();
    // Free first: whatever costs money comes after every free remedy, gems last.
    remedies.sort_by(|a, b| {
        a.costs_money
            .cmp(&b.costs_money)
            .then(a.rung.cmp(&b.rung))
            .then(
                b.fit
                    .partial_cmp(&a.fit)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.title.cmp(&b.title))
    });

    // Words (the texts' terms, for the expert pages)
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
        plain,
        remedies,
        signature: signature(planet).to_string(),
        ledger_hint,
    })
}

/// The remedy plan from natal facts alone, without the sky: `period_lords` are the
/// running dasha lords, maha first; `area` narrows the cause to the matter at hand and
/// adds the matter's own remedies. The plain dictionary and the other ways (food,
/// creatures, water, colour), strongest fit first; no exact windows, no dosha families
/// (those need the sky at an instant: use [`parihara`]).
pub fn parihara_from_facts(facts: &Facts, area: Option<Area>, period_lords: &[DashaLord]) -> Parihara {
    // Candidate causes with severity
    let mut candidates: Vec<(DashaLord, f64, Option<usize>)> = Vec::new();
    let matter = area.map(|a| a.name.to_ascii_lowercase());
    for (i, &l) in period_lords.iter().enumerate() {
        let (q, _why) = match area {
            Some(a) => period_quality(facts, a, l),
            None => {
                let p = facts.planet(l);
                let d = dignity(l, p.sidereal);
                (d.score(), Vec::new())
            }
        };
        candidates.push((l, 1.0 - q, Some(i)));
    }
    if let Some(a) = area {
        let lord = facts.rasi_lords[(a.primary - 1) as usize];
        let p = facts.planet(lord);
        let d = dignity(lord, p.sidereal);
        candidates.push((lord, 1.0 - d.score(), None));
    }
    let (planet, severity, level_idx) = candidates
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((DashaLord::Saturn, 0.0, None));
    let plain = plain_cause(planet, level_idx, matter.as_deref(), severity);
    let p = facts.planet(planet);
    let dig = dignity(planet, p.sidereal);
    let debilitated = dig == Dignity::Debilitated;
    let karma = classify(facts, planet, area, period_lords, None);
    let house = duty_house(facts, planet, area);

    let mut rows = dictionary_rows(planet, table(planet), "GRAHA", severity, 0.0, debilitated);
    // The other ways: food to the graha's people, its creatures fed, water and trees,
    // colour. They sit with the daana and seva of the main table.
    rows.extend(dictionary_rows(
        planet,
        crate::daana::alternatives(planet),
        "ALT",
        severity,
        -0.01,
        debilitated,
    ));
    if let Some(a) = area {
        // The matter's own remedies sit just under the graha's: same kind, a shade less fit.
        rows.extend(dictionary_rows(
            planet,
            life_table(a.name),
            "LIFE",
            severity,
            -0.03,
            debilitated,
        ));
    }
    unique_ids(&mut rows);
    let mut remedies: Vec<Remedy> = rows
        .into_iter()
        .map(|r| into_remedy(r, &karma, Vec::new()))
        .collect();
    remedies.sort_by(|a, b| {
        a.costs_money.cmp(&b.costs_money).then(
            b.fit
                .partial_cmp(&a.fit)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    let running_level = level_idx.and_then(|i| LEVELS.get(i).copied());
    let words = format!(
        "{planet:?} stands {dig:?} in rasi house {}{}; the duty it bears on is {}. Karma: {} ({}): {}.",
        p.rasi_house,
        running_level
            .map(|l| format!(" and runs as {l} lord"))
            .unwrap_or_default(),
        house_duty(house).domain,
        karma.class.name(),
        karma.kind_in_play.name(),
        karma.may_claim
    );
    Parihara {
        planet,
        severity,
        cause: Cause {
            lord: planet,
            house,
            placed_in: p.rasi_house,
            dignity: dig,
            running_level,
            running_since: None,
            running_until: None,
            severity,
            karma,
            doshas: Vec::new(),
            words,
        },
        plain,
        remedies,
        signature: signature(planet).to_string(),
        ledger_hint: "Record each occurrence as 'did it at <instant>'; one missed day does not void a 40-day vow.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::AREAS;
    use kshana_chart::{compute, ChartInput};
    use kshana_houses::HouseSystem;
    use kshana_time::Utc;

    #[test]
    fn every_graha_has_a_karma_first_dictionary() {
        for g in DashaLord::ORDER {
            let t = table(g);
            assert!(t.len() >= 14, "{g:?} has {}", t.len());
            assert_eq!(t[0].0, RemedyKind::Karma);
            for kind in [
                RemedyKind::Karma,
                RemedyKind::Seva,
                RemedyKind::Discipline,
                RemedyKind::Daana,
                RemedyKind::Vrata,
                RemedyKind::Mantra,
                RemedyKind::Pooja,
            ] {
                assert!(t.iter().any(|r| r.0 == kind), "{g:?} lacks {kind:?}");
            }
            assert!(!signature(g).is_empty());
        }
    }

    #[test]
    fn every_remedy_speaks_plainly() {
        let mut rows: Vec<Row> = Vec::new();
        for g in DashaLord::ORDER {
            rows.extend(table(g));
            rows.extend(crate::daana::alternatives(g));
            assert_eq!(jargon(signature(g)), None, "{}", signature(g));
        }
        for a in crate::areas::AREAS {
            rows.extend(life_table(a.name));
        }
        for r in &rows {
            assert_eq!(jargon(r.1), None, "title: {}", r.1);
            assert_eq!(jargon(r.2), None, "why: {}", r.2);
        }
    }

    #[test]
    fn every_area_has_its_own_remedies() {
        for a in crate::areas::AREAS {
            let t = life_table(a.name);
            assert!(t.len() >= 6, "{} has {}", a.name, t.len());
            assert!(t.iter().any(|r| !matches!(r.0, RemedyKind::Mantra | RemedyKind::Pooja)));
        }
    }

    #[test]
    fn every_graha_has_a_karma_first_plan_and_full_tables() {
        for g in DashaLord::ORDER {
            let t = expert_table(g);
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
            // Free remedies first, by rung; then whatever costs money, by rung.
            assert!(plan.remedies.windows(2).all(|w| (w[0].costs_money, w[0].rung) <= (w[1].costs_money, w[1].rung)));
            assert!(!plan.remedies[0].costs_money);
            for r in plan.remedies.iter().filter(|r| r.kind == RemedyKind::Gem) {
                assert!(r.costs_money);
                assert!(r.why.contains("never necessary"), "{}", r.why);
            }
            assert!(plan.remedies.iter().all(|r| !r.source_keys.is_empty()));
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
            assert!(japa.why_expert.contains("Kali Yuga"));
            // every sentence the person reads is plain; the texts' column is kept apart
            assert_eq!(jargon(&plan.plain), None, "{}", plan.plain);
            for r in &plan.remedies {
                assert_eq!(jargon(&r.title), None, "title: {}", r.title);
                assert_eq!(jargon(&r.why), None, "why: {} ({})", r.why, r.rule_id);
                assert!(!r.why_expert.is_empty(), "{}", r.rule_id);
            }
            let mut ids: Vec<&str> = plan.remedies.iter().map(|r| r.rule_id.as_str()).collect();
            ids.sort();
            let n = ids.len();
            ids.dedup();
            assert_eq!(ids.len(), n, "rule ids are unique");
            assert!(!plan.signature.is_empty());
            if area.is_some() {
                assert!(plan.remedies.iter().any(|r| r.rule_id.starts_with("KS-R-LIFE-")));
            }
            assert!(plan.remedies.iter().any(|r| r.kind == RemedyKind::Anna));
            // the facts-only plan agrees on the plain dictionary
            let quick = parihara_from_facts(&Facts::from_chart(&ch), area, &[]);
            assert!(quick.remedies.len() >= 20);
            assert!(quick.remedies.windows(2).all(|w| w[0].costs_money < w[1].costs_money || (w[0].costs_money == w[1].costs_money && w[0].fit >= w[1].fit)));
            assert!(matches!(quick.remedies[0].kind, RemedyKind::Karma | RemedyKind::Seva | RemedyKind::Discipline));
            assert_eq!(jargon(&quick.plain), None, "{}", quick.plain);
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
