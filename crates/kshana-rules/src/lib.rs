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
pub mod doshas;
pub mod facts;
pub mod karma;
pub mod kp;
pub mod kuta;
pub mod muhurta;
pub mod parashari;
pub mod parihara;
pub mod prashna;
pub mod rectify;
pub mod remedy_timing;

pub use areas::{area, Area, AREAS};
pub use doshas::{badhaka, doshas, Badhaka, Dosha};
pub use facts::{Facts, PlanetFact};
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
pub use parihara::{parihara, Cause, Parihara, Remedy, RemedyKind, Schedule};
pub use prashna::{
    prashna, prashna_windows, MoonWindow, PrashnaVerdict, QuestionShape, RulingPlanets, Verdict,
};
pub use rectify::{rectify, Candidate, EventScore, LifeEvent};
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
