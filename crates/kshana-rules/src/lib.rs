//! The prediction layer.
//!
//! Nothing here invents a fact. [`Facts`] is derived from a computed [`Chart`]; every
//! rule is a function of those facts that returns a [`Hit`] with the factors that fired
//! it and a citation. The app renders hits; it never writes a sentence that cannot
//! point at one.
//!
//! This first version carries the Krishnamurti Padhdhati significator model and the
//! cusp sub-lord promise rule for eight life areas, because they are formal enough to
//! implement without interpretation. Parashari rules follow the same shape.

#![forbid(unsafe_code)]

pub mod areas;
pub mod appointment;
pub mod ask;
pub mod briefing;
pub mod calibrate;
pub mod consensus;
pub mod daana;
pub mod doshas;
pub mod explain;
pub mod facts;
pub mod family;
pub mod facts_reveal;
pub mod feed;
pub mod forecast;
pub mod future;
pub mod gochara;
pub mod hora;
pub mod karma;
pub mod kp;
pub mod kundali;
pub mod kuta;
pub mod lang;
pub mod learn;
pub mod letter;
pub mod muhurta;
pub mod nadi;
pub mod parashari;
pub mod parihara;
pub mod pastcheck;
pub mod plain;
pub mod prashna;
pub mod prose;
pub mod rectify;
pub mod reveal;
pub mod remedy_timing;
pub mod remedy_tree;
#[cfg(test)]
mod remedy_audit;
pub mod sources;
pub mod timeline;
pub mod topics;
pub mod transit;
pub mod trust;
pub mod understand;
pub mod varsha;
pub mod yogas;
pub mod witness;

pub use areas::{area, Area, AREAS};
pub use appointment::{appointment, best_slots, best_slots_together, Appointment, DaySlot, GroupSlot, Purpose, Slot};
pub use ask::{answer, answer_topic, Answer, Question};
pub use briefing::{briefing, saturn_sentence, saturn_sentence_in, season_paragraph_in, Briefing, Circled, RemedyBrief};
pub use calibrate::{calibrate, Calibration, LifeCalibration, Record};
pub use doshas::{badhaka, doshas, Badhaka, Dosha};
pub use feed::{countdowns, day_title, festivals, next_good, Countdown, Festival, GoodMinute};
pub use letter::{letter, Letter};
pub use prose::{outlook, outlook_in, saturn_plain, LifeOutlook, Outlook, Span};
pub use plain::Lang;
pub use pastcheck::{past_checks, PastCheck};
pub use reveal::{past_reveal, refine_birth, refine_birth_detail, seal_pick, suggested_shift, Answer as PastAnswer, BirthFit, BirthRefine, Verdict as PastVerdict};
pub use remedy_tree::{remedy_tree, Branch, CauseNode, RemedyTree, TreeRemedy};
pub use hora::{hora_lore, moment_title, HoraLore};
pub use topics::{topic, topics_for, Topic, TOPICS};
pub use varsha::{year_chapter, YearChapter};
pub use explain::{explain, Cause, Explanation, Relief, Turning};
pub use kundali::{couple, couple_ask, kundali, talk_window, BondCare, Couple, CoupleAnswer, DoshaRemedy, Koota, Kundali};
pub use timeline::{timeline, Chapter, Passage, Return, SaturnSpan, Timeline};
pub use transit::{next_sign_change, returns, sidereal_longitude, sign_changes, SignChange};
pub use facts::{Facts, PlanetFact};
pub use forecast::{
    headline, moment, series, slow_series, story, Band, Component, Hora, Life, LifeScore, Moment, Natal,
    Sample, Shift, Story, Window,
};
pub use gochara::{tara_index, tara_words, SaturnPhase, TARA_NAMES, TARA_SCORE};
pub use karma::{classify, KarmaClass, KarmaKind, KarmaVerdict, TransitSigns};
pub use kp::{
    agreeing_levels, funnel_strength, node_agents, period_supports, promise, significators,
    support_score, timing, Level, Promise, Significance, TimingLevel,
};
pub use kuta::{
    ashtakoota, mangal_dosha, synastry, Ashtakoota, KutaScore, MangalDosha, SynastryAspect,
};
pub use muhurta::{muhurta, Activity, MuhurtaWindow};
pub use parashari::{
    assess, bhinna_for, dignity, period_quality, sarva_for, yogas, Assessment, Dignity, Yoga,
};
pub use parihara::{parihara, parihara_from_facts, plain_cause, Cause as RemedyCause, Parihara, Remedy, RemedyKind, Schedule};
pub use prashna::{
    prashna, prashna_windows, MoonWindow, PrashnaVerdict, QuestionShape, RulingPlanets, Verdict,
};
pub use rectify::{by_rising_sign, rectify, rectify_by_areas, AreaCandidate, AreaEvent, Candidate, EventScore, LifeEvent};
pub use remedy_timing::{next_windows, RemedyWindow, TimingTable};

/// One fired rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    /// Stable rule id, e.g. `KS-K-0001`.
    pub rule_id: &'static str,
    /// Tradition and source text.
    pub source: &'static str,
    /// Weight in `[0, 1]`.
    pub weight: f64,
    /// The factors that fired it, in words the UI can show.
    pub factors: Vec<String>,
}
