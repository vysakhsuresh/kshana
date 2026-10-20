//! Time scales for Kshana.
//!
//! # Why this crate exists
//!
//! A chart is a function of one instant. If that instant is wrong by a microsecond,
//! every number downstream inherits the error. A single `f64` Julian Date cannot hold
//! a microsecond: near the present day its resolution is roughly 40 µs. So Kshana
//! never stores time as one float.
//!
//! * [`Instant`] is an exact integer count of nanoseconds on the TAI scale. TAI is
//!   uniform, has no leap seconds, and is what every other scale is defined against.
//! * [`TwoPartJd`] is the `jd1 + jd2` pair used by the IAU SOFA library, produced only
//!   at the edge where an ephemeris or a frame routine needs a float.
//! * [`Utc`] is a civil date-time, including the 61st second of a leap-second day.
//! * [`DeltaT`] and [`Ut1`] carry the Earth-rotation scale, which is measured, not
//!   computed, and therefore has an explicit uncertainty.
//!
//! The scales are distinct Rust types so that mixing them is a compile error.
//!
//! # Precision notes
//!
//! * UTC ↔ TAI is exact (integer leap-second table).
//! * TT = TAI + 32.184 s, exact by definition.
//! * TDB − TT uses a truncated Fairhead & Bretagnon series (≈ 30 µs). The full series
//!   or the ephemeris's own TT-TDB segment replaces it in a later milestone.
//! * UT1 − UTC comes from IERS Bulletin A and must be supplied as data; without it ΔT
//!   comes from the published series in `delta_t` (monthly IERS values from 1973,
//!   Morrison, Stephenson, Hohenkerk & Zawilski 2021 before).

#![forbid(unsafe_code)]

mod civil;
mod delta_t;
mod eop;
mod instant;
mod leap;
mod tdb;

pub use civil::{
    civil_from_days, civil_label_uncertainty_s, days_from_civil, delta_t_for_instant, Utc,
    UtcError,
};
pub use delta_t::{delta_t_for_year, ut1_jd_from_delta_t, DeltaT, Ut1};
pub use eop::{EopRow, EopTable};
pub use instant::{Instant, TwoPartJd, NS_PER_DAY, NS_PER_SEC, TT_MINUS_TAI_NS};
pub use leap::{tai_minus_utc_at_utc_day, LEAP_SECONDS};
pub use tdb::tdb_minus_tt_seconds;
