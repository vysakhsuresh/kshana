//! The prenatal syzygy and the prenatal eclipse.
//!
//! Ptolemy (*Tetrabiblos* III.2) rectifies a birth time from the new or full Moon that
//! preceded the birth: for a new Moon the degree of the conjunction, for a full Moon
//! the degree of the luminary that was above the horizon at birth. Hellenistic hyleg
//! lists and Brady's Saros work use the last eclipse before birth in the same way.
//! Both instants are exact roots from `kshana-events` and this crate.

use crate::{
    lunar_eclipses, solar_eclipses, tt_jd, EclipseError, LunarEclipse, ShadowRule, SolarEclipse,
};
use kshana_ephem::{Body, Spk};
use kshana_events::{full_moons, new_moons, MICROSECOND_NS};
use kshana_sky::apparent;
use kshana_time::{Instant, NS_PER_DAY};

/// New or full Moon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyzygyKind {
    /// Conjunction of the Moon with the Sun.
    NewMoon,
    /// Opposition.
    FullMoon,
}

/// A syzygy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Syzygy {
    /// The exact instant (Moon − Sun apparent longitude = 0° or 180°, to 1 µs).
    pub at: Instant,
    /// New or full.
    pub kind: SyzygyKind,
    /// Apparent tropical longitude of the Moon at the syzygy, degrees. For a new Moon
    /// this is the conjunction degree; for a full Moon the Sun stands at
    /// `longitude + 180°`.
    pub longitude: f64,
    /// Meeus lunation number: of this new Moon, or of the new Moon preceding this full
    /// Moon.
    pub lunation: i64,
}

/// The last new or full Moon strictly before `birth`.
pub fn prenatal_syzygy(spk: &Spk, birth: Instant) -> Result<Syzygy, EclipseError> {
    let from = birth.add_ns(-32 * NS_PER_DAY);
    let mut best: Option<(Instant, SyzygyKind)> = None;
    let candidates = new_moons(spk, from, birth, MICROSECOND_NS)?
        .into_iter()
        .map(|e| (e.root.t, SyzygyKind::NewMoon))
        .chain(
            full_moons(spk, from, birth, MICROSECOND_NS)?
                .into_iter()
                .map(|e| (e.root.t, SyzygyKind::FullMoon)),
        );
    for (t, kind) in candidates {
        if t < birth && best.map_or(true, |(b, _)| t > b) {
            best = Some((t, kind));
        }
    }
    let (at, kind) = best.ok_or(EclipseError::NotFound)?;
    let longitude = apparent(spk, Body::Moon, at)?.longitude;
    let phase = (tt_jd(at) - crate::LUNATION_EPOCH_JDE) / crate::SYNODIC_MONTH_DAYS;
    let lunation = match kind {
        SyzygyKind::NewMoon => phase.round() as i64,
        SyzygyKind::FullMoon => phase.floor() as i64,
    };
    Ok(Syzygy {
        at,
        kind,
        longitude,
        lunation,
    })
}

/// The last eclipse of either kind before a birth.
#[derive(Clone, Debug, PartialEq)]
pub enum PrenatalEclipse {
    /// A solar eclipse.
    Solar(SolarEclipse),
    /// A lunar eclipse (penumbral ones count).
    Lunar(LunarEclipse),
}

impl PrenatalEclipse {
    /// Instant of greatest eclipse.
    pub fn at(&self) -> Instant {
        match self {
            PrenatalEclipse::Solar(e) => e.greatest,
            PrenatalEclipse::Lunar(e) => e.greatest,
        }
    }
}

/// The last solar or lunar eclipse whose greatest eclipse falls strictly before
/// `birth`. Successive eclipses are never more than about seven months apart, so the
/// search steps back in 220-day windows (three at most).
pub fn prenatal_eclipse(
    spk: &Spk,
    birth: Instant,
    rule: ShadowRule,
) -> Result<PrenatalEclipse, EclipseError> {
    let mut to = birth;
    for _ in 0..3 {
        let from = to.add_ns(-220 * NS_PER_DAY);
        let mut best: Option<PrenatalEclipse> = None;
        let mut consider = |e: PrenatalEclipse| {
            if e.at() < birth && best.as_ref().map_or(true, |b| e.at() > b.at()) {
                best = Some(e);
            }
        };
        for e in solar_eclipses(spk, from, to)? {
            consider(PrenatalEclipse::Solar(e));
        }
        for e in lunar_eclipses(spk, from, to, rule)? {
            consider(PrenatalEclipse::Lunar(e));
        }
        if let Some(b) = best {
            return Ok(b);
        }
        to = from;
    }
    Err(EclipseError::NotFound)
}
