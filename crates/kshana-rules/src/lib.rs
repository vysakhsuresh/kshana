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
pub mod explain;
pub mod facts;
pub mod forecast;
pub mod gochara;
pub mod kp;
pub mod kundali;
pub mod muhurta;
pub mod parashari;
pub mod parihara;
pub mod prashna;
pub mod timeline;
pub mod transit;

pub use areas::{area, Area, AREAS};
pub use appointment::{appointment, Appointment, Purpose, Slot};
pub use ask::{answer, Answer, Question};
pub use briefing::{briefing, saturn_sentence, Briefing, Circled, RemedyBrief};
pub use explain::{explain, Cause, Explanation, Relief, Turning};
pub use kundali::{couple, couple_ask, kundali, Couple, CoupleAnswer, Koota, Kundali};
pub use timeline::{timeline, Chapter, Passage, Return, SaturnSpan, Timeline};
pub use transit::{next_sign_change, returns, sidereal_longitude, sign_changes, SignChange};
pub use facts::{Facts, PlanetFact};
pub use forecast::{
    headline, moment, series, slow_series, story, Band, Component, Hora, Life, LifeScore, Moment, Natal,
    Sample, Shift, Story, Window,
};
pub use gochara::{tara_index, tara_words, SaturnPhase, TARA_NAMES, TARA_SCORE};
pub use kp::{
    agreeing_levels, node_agents, period_supports, promise, significators, timing, Level, Promise,
    Significance, TimingLevel,
};
pub use muhurta::{muhurta, Activity, MuhurtaWindow};
pub use parashari::{
    assess, bhinna_for, dignity, period_quality, sarva_for, yogas, Assessment, Dignity, Yoga,
};
pub use parihara::{parihara, Parihara, Remedy, RemedyKind};
pub use prashna::{
    prashna, prashna_windows, MoonWindow, PrashnaVerdict, QuestionShape, RulingPlanets, Verdict,
};

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
