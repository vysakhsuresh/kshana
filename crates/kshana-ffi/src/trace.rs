//! The life trace over the FFI: every transition in a window, and the state at an instant.

use kshana_sky::Observer;
use kshana_time::Instant;
use kshana_trace::{Change, Division, Person, TraceOptions};
use kshana_vedic::dasha::YEAR_365_25_NS;

use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// How fine the zodiac divisions go.
#[derive(Clone, Copy, Debug, uniffi::Enum)]
#[allow(missing_docs)]
pub enum Finest {
    None,
    Sign,
    Nakshatra,
    Pada,
    Sub,
    SubSub,
}

impl Finest {
    fn division(self) -> Option<Division> {
        match self {
            Finest::None => None,
            Finest::Sign => Some(Division::Sign),
            Finest::Nakshatra => Some(Division::Nakshatra),
            Finest::Pada => Some(Division::Pada),
            Finest::Sub => Some(Division::Sub),
            Finest::SubSub => Some(Division::SubSub),
        }
    }
}

/// One transition.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitionRecord {
    pub at: ExactInstant,
    pub half_width_ns: i64,
    /// "dasha", "enter", "tithi", "karana", "yoga", "hora", "sunrise", "sunset".
    pub kind: String,
    /// Dasha level (1–9) for "dasha"; 0 otherwise.
    pub level: u8,
    /// Dasha chain after the transition, for "dasha".
    pub chain: Vec<String>,
    /// End of the new dasha period, for "dasha".
    pub until: Option<ExactInstant>,
    /// Body for "enter".
    pub body: String,
    /// Division name for "enter": sign, nakshatra, pada, sub, sub-sub.
    pub division: String,
    /// Cell index for "enter", tithi number, karana ordinal or yoga index.
    pub index: u32,
    /// Direct motion for "enter".
    pub direct: bool,
    /// Hora lord for "hora".
    pub lord: String,
    /// The transition in words.
    pub words: String,
}

/// One graha's state in a snapshot.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct GrahaStateRecord {
    pub name: String,
    pub sidereal: f64,
    pub sign: u8,
    pub nakshatra: u8,
    pub pada: u8,
    pub sub_index: u16,
    pub star_lord: String,
    pub sub_lord: String,
    pub subsub_index: u16,
    pub subsub_lord: String,
    pub direct: bool,
}

/// Everything running at one instant.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SnapshotRecord {
    pub dasha_lords: Vec<String>,
    pub dasha_starts: Vec<ExactInstant>,
    pub dasha_ends: Vec<ExactInstant>,
    pub grahas: Vec<GrahaStateRecord>,
    pub tithi: u8,
    pub tithi_fraction: f64,
    pub karana: u8,
    pub yoga: u8,
    pub hora_lord: Option<String>,
    pub hora_start: Option<ExactInstant>,
    pub hora_end: Option<ExactInstant>,
}

fn person(
    birth: ExactInstant,
    moon_sidereal: f64,
    latitude_deg: f64,
    longitude_deg: f64,
    ayanamsa: AyanamsaChoice,
) -> Person {
    Person {
        birth: birth.into(),
        moon_sidereal,
        observer: Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        },
        ayanamsa: ayanamsa.into(),
        year_ns: YEAR_365_25_NS,
    }
}

#[uniffi::export]
impl Engine {
    /// Every transition in `[from, to)` for a person. `moon_sidereal` is the natal Moon
    /// (from `chart(...).panchanga.moon_sidereal`); the observer is where they are now.
    #[allow(clippy::too_many_arguments)]
    pub fn trace(
        &self,
        birth: ExactInstant,
        moon_sidereal: f64,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
        dasha_depth: u8,
        moon_finest: Finest,
        planet_finest: Finest,
        panchanga: bool,
        horas: bool,
    ) -> Result<Vec<TransitionRecord>, KshanaError> {
        let who = person(birth, moon_sidereal, latitude_deg, longitude_deg, ayanamsa);
        let from_i: Instant = from.into();
        let to_i: Instant = to.into();
        if to_i.ns_since(from_i) > 400 * 86_400 * 1_000_000_000 {
            return Err(KshanaError::Engine(
                "trace windows are at most 400 days; ask for the next window".to_string(),
            ));
        }
        let opts = TraceOptions {
            dasha_depth: dasha_depth.min(9) as usize,
            moon_finest: moon_finest.division(),
            planet_finest: planet_finest.division(),
            panchanga,
            horas,
        };
        let t = kshana_trace::trace(&self.spk, self.eop.as_ref(), &who, from_i, to_i, opts)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(t.into_iter()
            .map(|x| {
                let words = x.describe();
                let mut r = TransitionRecord {
                    at: x.at.into(),
                    half_width_ns: x.half_width_ns.min(i64::MAX as i128) as i64,
                    kind: String::new(),
                    level: 0,
                    chain: Vec::new(),
                    until: None,
                    body: String::new(),
                    division: String::new(),
                    index: 0,
                    direct: true,
                    lord: String::new(),
                    words,
                };
                match x.change {
                    Change::Dasha {
                        level,
                        chain,
                        until,
                    } => {
                        r.kind = "dasha".into();
                        r.level = level;
                        r.chain = chain.iter().map(|l| format!("{l:?}")).collect();
                        r.until = Some(until.into());
                    }
                    Change::Enter {
                        body,
                        division,
                        index,
                        direct,
                    } => {
                        r.kind = "enter".into();
                        r.body = body.into();
                        r.division = division.name().into();
                        r.index = index as u32;
                        r.direct = direct;
                    }
                    Change::Tithi(n) => {
                        r.kind = "tithi".into();
                        r.index = n as u32;
                    }
                    Change::Karana(k) => {
                        r.kind = "karana".into();
                        r.index = k as u32;
                    }
                    Change::Yoga(y) => {
                        r.kind = "yoga".into();
                        r.index = y as u32;
                    }
                    Change::Hora(l) => {
                        r.kind = "hora".into();
                        r.lord = l.into();
                    }
                    Change::Sunrise => r.kind = "sunrise".into(),
                    Change::Sunset => r.kind = "sunset".into(),
                }
                r
            })
            .collect())
    }

    /// Everything running at `at` for a person, dasha chain to `depth` (1–9).
    #[allow(clippy::too_many_arguments)]
    pub fn snapshot(
        &self,
        birth: ExactInstant,
        moon_sidereal: f64,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        depth: u8,
    ) -> Result<SnapshotRecord, KshanaError> {
        let who = person(birth, moon_sidereal, latitude_deg, longitude_deg, ayanamsa);
        let s = kshana_trace::snapshot(
            &self.spk,
            self.eop.as_ref(),
            &who,
            at.into(),
            depth.clamp(1, 9) as usize,
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(SnapshotRecord {
            dasha_lords: s.dashas.iter().map(|p| format!("{:?}", p.lord)).collect(),
            dasha_starts: s.dashas.iter().map(|p| p.start.into()).collect(),
            dasha_ends: s.dashas.iter().map(|p| p.end.into()).collect(),
            grahas: s
                .grahas
                .iter()
                .map(|g| GrahaStateRecord {
                    name: g.name.into(),
                    sidereal: g.sidereal,
                    sign: g.sign as u8,
                    nakshatra: g.nakshatra as u8,
                    pada: g.pada,
                    sub_index: g.sub.0 as u16,
                    star_lord: format!("{:?}", g.sub.1),
                    sub_lord: format!("{:?}", g.sub.2),
                    subsub_index: g.subsub.0 as u16,
                    subsub_lord: format!("{:?}", g.subsub.1),
                    direct: g.direct,
                })
                .collect(),
            tithi: s.tithi.0,
            tithi_fraction: s.tithi.1,
            karana: s.karana,
            yoga: s.yoga,
            hora_lord: s.hora.map(|h| h.0.to_string()),
            hora_start: s.hora.map(|h| h.1.into()),
            hora_end: s.hora.map(|h| h.2.into()),
        })
    }
}

/// Names of the dasha levels, 1-based.
#[uniffi::export]
pub fn dasha_level_names() -> Vec<String> {
    kshana_trace::LEVEL_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn trace_and_snapshot_cross_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let u = |y, mo, d, h, mi| {
            instant_from_utc(UtcFields {
                year: y,
                month: mo,
                day: d,
                hour: h,
                minute: mi,
                second: 0,
                nanos: 0,
            })
            .unwrap()
        };
        let birth = u(1990, 10, 29, 12, 6);
        let from = u(2026, 10, 16, 0, 0);
        let to = u(2026, 10, 17, 0, 0);
        let t = engine
            .trace(
                birth,
                316.344,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                from,
                to,
                7,
                Finest::SubSub,
                Finest::Sub,
                true,
                true,
            )
            .unwrap();
        assert!(t.len() > 100);
        assert!(t.iter().all(|x| !x.words.is_empty() && !x.kind.is_empty()));
        let s = engine
            .snapshot(
                birth,
                316.344,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                from,
                9,
            )
            .unwrap();
        assert_eq!(s.dasha_lords.len(), 9);
        assert_eq!(s.grahas.len(), 9);
        assert_eq!(dasha_level_names().len(), 9);
    }
}
