//! The life ledger: dated life events scored against what the chart said about the
//! moment, with an honest calibration against instants nobody logged.
//!
//! A person logs what happened and when ([`LifeEvent`]). [`score_event`] asks the
//! engine what it would have said about that area at that instant: the Vimshottari
//! funnel of KP significators, the Parashari quality of the running lords, and
//! Jupiter's and Saturn's houses from the natal Moon, folded into one
//! [`EventScore::fit`] with the reasoning in sentences. [`baseline_fits`] scores the
//! same area at evenly spaced instants across a span, which is what a random moment
//! scores; [`calibrate`] sets the two against each other and says in words whether the
//! engine did better than chance. [`retrodict`] reads every antardasha of a span before
//! anything is logged, so the app can show what the chart said about each stretch of
//! the past first, and [`fit_words`] turns one score into one plain sentence.
//!
//! The measurement layer makes the chance line exact and the ledgers honest:
//!
//! * [`fit_profile`] and [`coverage`]: the fit as a piecewise function of time cut at
//!   every exact pratyantar boundary, with the deeper levels handled analytically, so
//!   the fraction of a life the engine would have called "supported" is computed, not
//!   sampled; [`tally`] and [`expected_hits`] turn it into "31 of 40 supported; chance
//!   would give 27 ± 3" with the exact Poisson-binomial tail.
//! * [`specificity_factor`] and [`score_event_discounted`]: the fit per equivalent
//!   label when the taxonomy's house groups overlap.
//! * [`time_shift_pvalue`]: the circular time-shift surrogate test, a p-value that
//!   keeps the shape of a life.
//! * [`metrics`]: Brier with Murphy's decomposition, the base-rate skill score, ECE,
//!   the calibration curve and Wilson intervals.
//! * [`seal`], [`resolve`] and [`sealed_brier`]: pre-registered forecasts with a
//!   SHA-256 id that makes any later edit visible, scored only once resolved.
//! * [`retrodiction_quiz`] and [`grade`]: asked-first questions with decoy windows,
//!   committed before the person answers.
//! * [`birth_posterior`]: a real posterior over birth time from likelihood ratios of
//!   fit to coverage, with HPD intervals as exact instants.
//!
//! Nothing here uses randomness or serde: the JSON lines are written and read by hand
//! so the Android side can store them as plain text, and every shuffle is seeded by
//! a hash of the chart.

#![forbid(unsafe_code)]

mod calibrate;
mod claim;
mod coverage;
mod decoy;
mod event;
mod hash;
mod json;
mod metrics;
mod opentest;
mod posterior;
mod retrodict;
mod score;
mod scoreboard;
mod sealed;
mod specificity;
mod surrogate;

pub use calibrate::{
    baseline_fits, baseline_fits_for, baseline_for_ledger, baseline_instants, calibrate,
    Calibration, ReliabilityBin, RELIABILITY_BINS,
};
pub use coverage::{
    chance_words, coverage, coverage_of, expected_hits, fit_profile, tally, Coverage,
    EventCoverage, FitProfile, FitSegment, PoissonBinomial, Tally, TransitTable,
    CANNOT_FAIL_COVERAGE, PROFILE_DEPTH, SUPPORT_THRESHOLD,
};
pub use decoy::{
    binomial_p_at_least, grade, retrodiction_quiz, verify_question, Question, QuizResult,
};
pub use event::{iso_utc, month_year, parse_iso_utc, EventKind, LedgerError, LifeEvent};
pub use hash::{seed_of, sha256, sha256_hex, SplitMix};
pub use metrics::{
    count_words, cube_root_bins, metrics, wilson_interval, CalibrationBin, Metrics, MIN_FOR_CURVE,
    MIN_FOR_PERCENT, MIN_PER_BIN, Z95,
};
pub use opentest::{
    alpha_at_look, contribution, evaluate, evaluate_under, holm, install_code, no_better_than_chance, own_records,
    own_words, protocol_hash, protocol_text, record_words, short_hash, silenced_areas, silenced_line, tail_p,
    Contribution, Exclusions, GroupResult, LeftOut, OpenRecord, OpenReport, PooledArea, Silenced, SilencedBy,
    TestClaim, Tier, ALPHA, LOOK_EVERY, MIN_SALT, POOLED_AREAS, PROTOCOL_HASH_V1, PROTOCOL_VERSION, SILENCE_MIN,
};
pub use posterior::{birth_posterior, hpd_interval, Posterior, LIKELIHOOD_FLOOR, MAX_CANDIDATES};
pub use retrodict::{retrodict, PeriodReading};
pub use score::{
    combine_fit, fit_words, parashari_factor, score_at, score_event, transit_factor,
    transit_sample, EventScore, TransitSample, FUNNEL_DEPTH, JUPITER_GOOD_FROM_MOON,
    SADE_SATI_SHIFT, SATURN_GOOD_FROM_MOON, WEIGHT_DASHA, WEIGHT_PARASHARI, WEIGHT_TRANSIT,
};
pub use sealed::{resolve, seal, sealed_brier, SealedForecast};
pub use claim::{claim_key, resolve_claim, seal_claim, select_new, ClaimInput, Outcome, SealedClaim, MIN_LEAD_DAYS};
pub use scoreboard::{points_text, scoreboard, ScoreItem, ScoreLine, ScoreSource, Scoreboard, BEATS_CHANCE_P, MIN_TO_JUDGE};
pub use specificity::{
    equivalent_areas, score_event_discounted, specificity_factor, tested_houses, DiscountedScore,
};
pub use surrogate::{circular_shift, time_shift_pvalue, SurrogateResult};

#[cfg(test)]
pub(crate) mod fixture {
    //! The Kochi birth used across the crate's kernel tests. Each test returns early
    //! when the DE421 kernel is not on disk.

    use kshana_chart::{compute, Chart, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::{Instant, Utc};
    use kshana_vedic::Ayanamsa;

    /// Kernel and natal chart for 1990-10-29 17:36 IST (12:06 UTC) at Kochi.
    pub(crate) fn kochi() -> Option<(Spk, Chart)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let birth = Utc::new(1990, 10, 29, 12, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: birth,
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
        .expect("natal chart");
        Some((spk, ch))
    }

    /// Noon UTC on a civil date.
    pub(crate) fn noon(y: i64, m: u32, d: u32) -> Instant {
        Utc::new(y, m, d, 12, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }
}
