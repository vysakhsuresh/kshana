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
//! Nothing here uses randomness or serde: the JSON lines are written and read by hand
//! so the Android side can store them as plain text.

#![forbid(unsafe_code)]

mod calibrate;
mod event;
mod json;
mod retrodict;
mod score;

pub use calibrate::{
    baseline_fits, baseline_fits_for, baseline_for_ledger, baseline_instants, calibrate,
    Calibration, ReliabilityBin, RELIABILITY_BINS,
};
pub use event::{iso_utc, month_year, parse_iso_utc, EventKind, LedgerError, LifeEvent};
pub use retrodict::{retrodict, PeriodReading};
pub use score::{
    fit_words, score_at, score_event, EventScore, FUNNEL_DEPTH, JUPITER_GOOD_FROM_MOON,
    SADE_SATI_SHIFT, SATURN_GOOD_FROM_MOON, WEIGHT_DASHA, WEIGHT_PARASHARI, WEIGHT_TRANSIT,
};

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
