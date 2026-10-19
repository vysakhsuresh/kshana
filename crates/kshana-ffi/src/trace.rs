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
    /// How far the real sky's event may lie from `at`, from the ephemeris and model
    /// limits (0 for a dasha boundary, which is exact arithmetic).
    pub physical_ns: i64,
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

/// A transition as the app receives it.
fn transition_record(x: kshana_trace::Transition) -> TransitionRecord {
    let words = x.describe();
    let physical_ns = x.physical_uncertainty_ns().min(i64::MAX as i128) as i64;
    let mut r = TransitionRecord {
        at: x.at.into(),
        half_width_ns: x.half_width_ns.min(i64::MAX as i128) as i64,
        physical_ns,
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
        lagna_finest: Finest,
    ) -> Result<Vec<TransitionRecord>, KshanaError> {
        let who = person(birth, moon_sidereal, latitude_deg, longitude_deg, ayanamsa);
        let (from_i, to_i) = self.clip_to_kernel(from.into(), to.into())?;
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
            lagna_finest: lagna_finest.division(),
        };
        let t = kshana_trace::trace(&self.spk, self.eop.as_ref(), &who, from_i, to_i, opts)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(t.into_iter().map(transition_record).collect())
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

/// The kernel's coverage.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct CoverageRecord {
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub segments: u32,
}

impl Engine {
    /// Clip a window to the kernel's coverage (a day inside each end), erroring only
    /// when nothing of it is covered.
    fn clip_to_kernel(
        &self,
        from: Instant,
        to: Instant,
    ) -> Result<(Instant, Instant), KshanaError> {
        let c = self.kernel_coverage();
        let start: Instant = c.start.into();
        let end: Instant = c.end.into();
        let day = 86_400 * 1_000_000_000;
        let lo = start.add_ns(day);
        let hi = end.add_ns(-day);
        let f = if from < lo { lo } else { from };
        let t = if to > hi { hi } else { to };
        if t <= f {
            return Err(KshanaError::Engine(format!(
                "the loaded kernel covers {} to {} only; this window lies outside it",
                kshana_speak::date_words(start),
                kshana_speak::date_words(end)
            )));
        }
        Ok((f, t))
    }
}

#[uniffi::export]
impl Engine {
    /// The next `count` transitions after `from`, found by widening the window
    /// (6 h, 1 d, 7 d, 30 d, 400 d) until enough are solved. Same options as `trace`.
    #[allow(clippy::too_many_arguments)]
    pub fn next_transitions(
        &self,
        birth: ExactInstant,
        moon_sidereal: f64,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        count: u32,
        dasha_depth: u8,
        moon_finest: Finest,
        planet_finest: Finest,
        panchanga: bool,
        horas: bool,
        lagna_finest: Finest,
    ) -> Result<Vec<TransitionRecord>, KshanaError> {
        let who = person(birth, moon_sidereal, latitude_deg, longitude_deg, ayanamsa);
        let from_i: Instant = from.into();
        let opts = TraceOptions {
            dasha_depth: dasha_depth.min(9) as usize,
            moon_finest: moon_finest.division(),
            planet_finest: planet_finest.division(),
            panchanga,
            horas,
            lagna_finest: lagna_finest.division(),
        };
        const HOUR: i128 = 3_600 * 1_000_000_000;
        for span in [
            6 * HOUR,
            24 * HOUR,
            7 * 24 * HOUR,
            30 * 24 * HOUR,
            400 * 24 * HOUR,
        ] {
            let t = kshana_trace::trace(
                &self.spk,
                self.eop.as_ref(),
                &who,
                from_i,
                from_i.add_ns(span),
                opts,
            )
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
            if t.len() >= count as usize || span == 400 * 24 * HOUR {
                return Ok(t
                    .into_iter()
                    .take(count as usize)
                    .map(transition_record)
                    .collect());
            }
        }
        Ok(Vec::new())
    }

    /// How well a civil label of `at` can be printed today, in seconds (the leap-second
    /// floor): 0 within the known table, one per possible insertion until 2035.
    pub fn civil_label_uncertainty(&self, at: ExactInstant) -> f64 {
        kshana_time::civil_label_uncertainty_s(at.into())
    }

    /// The Sun's apparent altitude in degrees at a place, refraction excluded.
    pub fn sun_altitude(
        &self,
        at: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Result<f64, KshanaError> {
        let obs = Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        };
        kshana_events::sun_altitude(&self.spk, self.eop.as_ref(), &obs, at.into())
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))
    }

    /// The span of time the loaded kernel covers (the intersection of its segments).
    pub fn kernel_coverage(&self) -> CoverageRecord {
        let segs = self.spk.segments();
        let start = segs
            .iter()
            .map(|s| s.start_et)
            .fold(f64::NEG_INFINITY, f64::max);
        let end = segs.iter().map(|s| s.end_et).fold(f64::INFINITY, f64::min);
        let j2000 = Instant::j2000();
        CoverageRecord {
            start: j2000.add_seconds_f64(start).into(),
            end: j2000.add_seconds_f64(end).into(),
            segments: segs.len() as u32,
        }
    }
}

/// How the recorded birth time limits the dasha levels.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DashaSensitivityRecord {
    /// Shift of every Vimshottari boundary per second of birth-time error, in seconds.
    pub shift_seconds_per_second: f64,
    /// Shift for the given half-window, in seconds.
    pub shift_seconds: f64,
    /// Deepest level (1–9) whose periods outlast four times that shift.
    pub meaningful_depth: u8,
    pub words: String,
}

/// The dasha boundaries' sensitivity to the birth time, for a Moon speed in degrees
/// per day (from `chart(...).bodies[1].speed`) and a birth-time half-window in seconds.
#[uniffi::export]
pub fn dasha_sensitivity(
    moon_speed_deg_per_day: f64,
    half_window_seconds: f64,
) -> DashaSensitivityRecord {
    let per_s = kshana_trace::dasha_shift_per_second_ns(moon_speed_deg_per_day, YEAR_365_25_NS)
        as f64
        / 1e9;
    let shift = per_s * half_window_seconds;
    let depth =
        kshana_trace::meaningful_depth(moon_speed_deg_per_day, half_window_seconds, YEAR_365_25_NS);
    let human = if shift >= 86_400.0 * 2.0 {
        format!("{:.0} days", shift / 86_400.0)
    } else if shift >= 7_200.0 {
        format!("{:.1} hours", shift / 3_600.0)
    } else {
        format!("{:.0} minutes", shift / 60.0)
    };
    DashaSensitivityRecord {
        shift_seconds_per_second: per_s,
        shift_seconds: shift,
        meaningful_depth: depth,
        words: if half_window_seconds <= 0.0 {
            "Your birth time is exact, so every dasha level down to minutes stands.".to_string()
        } else {
            format!(
                "Each second of birth-time error moves every dasha boundary by about {:.0} hours; your window of ±{} moves them by ±{}, so levels below {} are shown greyed.",
                per_s / 3_600.0,
                if half_window_seconds >= 3_600.0 { format!("{:.0} h", half_window_seconds / 3_600.0) } else { format!("{:.0} min", half_window_seconds / 60.0) },
                human,
                kshana_trace::LEVEL_NAMES[(depth as usize).clamp(1, 9) - 1]
            )
        },
    }
}

/// Set the process-wide sunrise convention: "hindu" (the Sun's centre on the
/// geometric horizon, as Kerala and Tamil almanacs reckon) or "western" (upper limb
/// with refraction, the default). Every hora, Rahu kala, ghati, Gulika and muhurta
/// follows it.
#[uniffi::export]
pub fn set_rise_convention(name: String) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "hindu" | "centre" | "center" | "geometric" => {
            kshana_events::set_rise_convention(kshana_events::RiseConvention::CentreGeometric);
            true
        }
        "western" | "limb" | "refracted" => {
            kshana_events::set_rise_convention(kshana_events::RiseConvention::UpperLimbRefracted);
            true
        }
        _ => false,
    }
}

/// The current sunrise convention: "hindu" or "western".
#[uniffi::export]
pub fn rise_convention() -> String {
    match kshana_events::rise_convention() {
        kshana_events::RiseConvention::CentreGeometric => "hindu".into(),
        kshana_events::RiseConvention::UpperLimbRefracted => "western".into(),
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
                Finest::Nakshatra,
            )
            .unwrap();
        assert!(t.len() > 100);
        assert!(t.iter().any(|x| x.body == "Lagna"));
        let n = engine
            .next_transitions(
                birth,
                316.344,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                from,
                12,
                5,
                Finest::Sub,
                Finest::Sign,
                true,
                true,
                Finest::Nakshatra,
            )
            .unwrap();
        assert_eq!(n.len(), 12);
        assert!(n
            .windows(2)
            .all(|w| (w[0].at.tai_seconds, w[0].at.nanos) <= (w[1].at.tai_seconds, w[1].at.nanos)));
        let alt = engine.sun_altitude(from, 9.9312, 76.2673).unwrap();
        assert!((-90.0..=90.0).contains(&alt));
        let cov = engine.kernel_coverage();
        assert!(
            cov.start.tai_seconds < birth.tai_seconds && cov.end.tai_seconds > from.tai_seconds
        );
        assert_eq!(dasha_sensitivity(13.2, 300.0).meaningful_depth, 2);
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
        assert_eq!(s.grahas.len(), 10);
        assert_eq!(dasha_level_names().len(), 9);
    }
}
