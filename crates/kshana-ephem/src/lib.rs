//! Planetary positions straight from JPL kernels.
//!
//! Kshana reads the SPK files NASA publishes (DE440, DE441, small-body kernels) rather
//! than a compressed derivative, so its positions are the ephemeris's positions. The
//! reader supports DAF files with Type 2 (Chebyshev position) segments, which is what
//! the DE planetary kernels use, and Type 3 (position + velocity) for completeness.
//!
//! Run `scripts/fetch-ephemeris.sh` to download `de440s.bsp`, then
//! `KSHANA_SPK=data/de440s.bsp cargo test -p kshana-ephem -- --ignored` to exercise
//! the reader on a real kernel.

#![forbid(unsafe_code)]

pub mod chebyshev;
pub mod daf;
pub mod spk;

pub use spk::{Body, Spk, SpkError, State};
