//! What's coming, in dates, over the FFI: the windows of the coming months where the
//! independent ways of reading a chart agree ([`kshana_rules::consensus`]), said more
//! specifically the more of them agree, with the likely days and, when sharp, the hours.
//!
//! What the person reads: `area_name`, `also_names`, `headline`, `detail`, `sure_line`,
//! `date_line`, `who`, `kind`, `best_line`, `best_hours`, every day's `label` and
//! `hours`, `remedy_hint`, `systems` (the plain names of the agreeing ways of reading)
//! and `seal_text`. The expert layer (shown only behind the chart-pages setting) is
//! `system_keys` and `expert`; `area`, `also` and `tier` are stable keys for the app
//! (see `plain_audit` below).
//!
//! Only `tier == "strong"` is ever said firmly ("very likely"); nothing is said to be
//! certain. Every hard window (`is_hard`) carries humane words and a `remedy_hint`.

use kshana_rules::consensus::{self, BestDay, Narrowing, SureWindow};
use kshana_rules::{Lang, Life};
use kshana_sky::Observer;

use crate::{AyanamsaChoice, Engine, ExactInstant, HouseSystem, KshanaError};

/// One day inside a window that stands out.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SureDayRecord {
    /// Local midnight that starts the day (at the present place's clock).
    pub start: ExactInstant,
    /// The next local midnight.
    pub end: ExactInstant,
    /// "Monday 16 Nov".
    pub label: String,
    /// The likely hours, when the narrowing is sharp.
    pub hours_from: Option<ExactInstant>,
    /// End of the likely hours.
    pub hours_to: Option<ExactInstant>,
    /// "morning (about 9–11 am)".
    pub hours: Option<String>,
}

/// A window where the ways of reading agree.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SureWindowRecord {
    /// Stable key of the part of life: "Love", "Career", "Money", "Health", "Family",
    /// "Study", "Home", "Travel", "Children".
    pub area: String,
    /// The part of life in the reader's language.
    pub area_name: String,
    /// Keys of other parts of life that share these dates (said once, on this card).
    pub also: Vec<String>,
    /// Their names in the reader's language.
    pub also_names: Vec<String>,
    /// Start, to about two minutes (before `from` when already running).
    pub start: ExactInstant,
    /// End, to about two minutes.
    pub end: ExactInstant,
    /// The strongest moment.
    pub peak: ExactInstant,
    /// "strong", "likely" or "possible".
    pub tier: String,
    /// Ways of reading that agree.
    pub agree: u32,
    /// Ways of reading with something to say here.
    pub of: u32,
    /// Ways of reading that lean the other way.
    pub against: u32,
    /// 0.5–1: how far into the person's own range the agreement goes.
    pub strength: f64,
    /// "14–19 Nov 2026" (the month for a possible window): the big date.
    pub date_line: String,
    /// The sentence: more specific the stronger the agreement.
    pub headline: String,
    /// How sure, through what, why; for a hard window, when it eases and what helps.
    pub detail: String,
    /// "6 of 8 ways of reading agree."
    pub sure_line: String,
    /// Who is involved (strong windows only, else empty).
    pub who: String,
    /// The kind of event (likely and strong windows, else empty).
    pub kind: String,
    /// The likely days (likely and strong windows).
    pub best_days: Vec<SureDayRecord>,
    /// "Most likely Monday 16 Nov, morning (about 9–11 am)." or "Most likely on …";
    /// empty when nothing stands out.
    pub best_line: String,
    /// The hours alone, only when the narrowing is sharp.
    pub best_hours: Option<String>,
    /// A hard stretch.
    pub is_hard: bool,
    /// What helps (always present for a hard window).
    pub remedy_hint: Option<String>,
    /// The agreeing ways of reading, in plain words.
    pub systems: Vec<String>,
    /// Expert: their keys ("vimshottari", "yogini", "chara", "kalachakra", "kp",
    /// "varga", "gochara", "tajika", "nadi").
    pub system_keys: Vec<String>,
    /// Expert: the pooled lean, the triggering lords and Nadi passages, and the houses
    /// the words came from.
    pub expert: String,
    /// A self-contained statement with both dates, for sealing.
    pub seal_text: String,
    /// Where this comes from: the agreeing ways of reading, the pooling and (when days
    /// are named) the narrowing. Resolve with `source_lines`; never shown as is.
    pub source_keys: Vec<String>,
}

/// A window narrowed to its days and hours.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SureHoursRecord {
    /// The days, in date order (at most three).
    pub days: Vec<SureDayRecord>,
    /// "Most likely Monday 16 Nov, morning (about 9–11 am)."; empty when nothing stands out.
    pub line: String,
    /// The hours alone, only when sharp.
    pub hours: Option<String>,
}

fn day_record(d: &BestDay) -> SureDayRecord {
    SureDayRecord {
        start: d.start.into(),
        end: d.end.into(),
        label: d.label.clone(),
        hours_from: d.hours.as_ref().map(|h| h.0.into()),
        hours_to: d.hours.as_ref().map(|h| h.1.into()),
        hours: d.hours.as_ref().map(|h| h.2.clone()),
    }
}

fn window_record(w: &SureWindow, lang: Lang) -> SureWindowRecord {
    SureWindowRecord {
        area: w.life.name().to_string(),
        area_name: kshana_rules::lang::life_name(lang, w.life).to_string(),
        also: w.also.iter().map(|l| l.name().to_string()).collect(),
        also_names: w.also.iter().map(|l| kshana_rules::lang::life_name(lang, *l).to_string()).collect(),
        start: w.start.into(),
        end: w.end.into(),
        peak: w.peak.into(),
        tier: w.tier.key().to_string(),
        agree: w.agree as u32,
        of: w.of as u32,
        against: w.against as u32,
        strength: w.strength,
        date_line: w.date_line.clone(),
        headline: w.headline.clone(),
        detail: w.detail.clone(),
        sure_line: w.sure_line.clone(),
        who: w.who.clone(),
        kind: w.kind.clone(),
        best_days: w.narrowing.days.iter().map(day_record).collect(),
        best_line: w.narrowing.line.clone(),
        best_hours: w.narrowing.hours_line.clone(),
        is_hard: !w.good,
        remedy_hint: w.remedy_hint.clone(),
        systems: w.systems.iter().map(|s| s.name(lang).to_string()).collect(),
        system_keys: w.systems.iter().map(|s| s.key().to_string()).collect(),
        expert: w.expert.clone(),
        seal_text: w.seal_text.clone(),
        source_keys: window_source_keys(w),
    }
}

/// The ways of reading that agree, then the app's pooling, then its narrowing to days.
fn window_source_keys(w: &SureWindow) -> Vec<String> {
    let mut k: Vec<String> = w.systems.iter().map(|s| format!("system.{}", s.key())).collect();
    k.push("kshana.consensus".into());
    if !w.narrowing.days.is_empty() {
        k.push("kshana.hours".into());
    }
    k
}

fn hours_record(n: &Narrowing) -> SureHoursRecord {
    SureHoursRecord { days: n.days.iter().map(day_record).collect(), line: n.line.clone(), hours: n.hours_line.clone() }
}

fn lives_of(areas: &[String]) -> Vec<Life> {
    areas.iter().filter_map(|a| Life::parse(a)).collect()
}

#[uniffi::export]
impl Engine {
    /// The windows of the next `months` months (1–60) from `from` where the ways of
    /// reading agree, for `areas` (life keys; every part of life when empty), at most
    /// `max`, strongest first and then by date. `here_lat`/`here_lon` and
    /// `utc_offset_minutes` are where the person lives now and its clock (IST = 330): the
    /// birthday year is cast there and the dates and hours are said by that clock.
    #[allow(clippy::too_many_arguments)]
    pub fn sure_windows(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        here_lat: f64,
        here_lon: f64,
        utc_offset_minutes: i32,
        from: ExactInstant,
        months: u32,
        areas: Vec<String>,
        max: u32,
        lang: String,
    ) -> Result<Vec<SureWindowRecord>, KshanaError> {
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let here = Observer { latitude_deg: here_lat, longitude_deg: here_lon, height_m: 0.0 };
        let lang = Lang::parse(&lang);
        let w = consensus::sure_windows(
            &self.spk,
            self.eop.as_ref(),
            &natal,
            &obs,
            &here,
            ayanamsa.into(),
            lang,
            utc_offset_minutes.clamp(-14 * 60, 14 * 60),
            from.into(),
            months,
            &lives_of(&areas),
            max.clamp(1, 200) as usize,
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(w.iter().map(|x| window_record(x, lang)).collect())
    }

    /// The next strong window of the coming `months` months (1–24) from `from`, for the
    /// Now card: the earliest strong one, else the earliest likely one, else none.
    #[allow(clippy::too_many_arguments)]
    pub fn sure_next(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        here_lat: f64,
        here_lon: f64,
        utc_offset_minutes: i32,
        from: ExactInstant,
        months: u32,
        lang: String,
    ) -> Result<Option<SureWindowRecord>, KshanaError> {
        let all = self.sure_windows(birth, birth_lat, birth_lon, system, ayanamsa, here_lat, here_lon, utc_offset_minutes, from, months.clamp(1, 24), vec![], 40, lang)?;
        let first = |tier: &str| all.iter().filter(|w| w.tier == tier).min_by_key(|w| (w.start.tai_seconds, w.start.nanos)).cloned();
        Ok(first("strong").or_else(|| first("likely")))
    }

    /// Narrow a window (`start`–`end`, of `area`, hard or not) to its likely days and,
    /// when sharp, the hours of the best day, read where the person lives now. Pass the
    /// window's `peak` and `system_keys` (from `sure_windows`) to read it exactly as the
    /// window did; without them it is read at its middle.
    #[allow(clippy::too_many_arguments)]
    pub fn sure_window_hours(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        here_lat: f64,
        here_lon: f64,
        utc_offset_minutes: i32,
        area: String,
        is_hard: bool,
        start: ExactInstant,
        end: ExactInstant,
        now: ExactInstant,
        peak: Option<ExactInstant>,
        system_keys: Vec<String>,
        lang: String,
    ) -> Result<SureHoursRecord, KshanaError> {
        let life = Life::parse(&area).ok_or_else(|| KshanaError::Engine(format!("unknown area {area}")))?;
        let systems: Vec<consensus::System> = system_keys.iter().filter_map(|k| consensus::System::parse(k)).collect();
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let here = Observer { latitude_deg: here_lat, longitude_deg: here_lon, height_m: 0.0 };
        let n = consensus::sure_hours(
            &self.spk,
            self.eop.as_ref(),
            &natal,
            &obs,
            &here,
            ayanamsa.into(),
            Lang::parse(&lang),
            utc_offset_minutes.clamp(-14 * 60, 14 * 60),
            life,
            !is_hard,
            start.into(),
            end.into(),
            now.into(),
            peak.map(Into::into),
            (!system_keys.is_empty()).then_some(systems.as_slice()),
        );
        Ok(hours_record(&n))
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    /// Fields of these records the person never reads (expert view or stable keys).
    const NOT_SHOWN: &[&str] = &["area", "also", "tier", "system_keys", "expert", "source_keys"];

    /// Words that promise what no reading can.
    const NEVER: &[&str] = &["guarantee", "certain", "definitely", "fate", "only god", "will surely", "doom"];

    fn engine() -> std::sync::Arc<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Engine::open(p.to_string_lossy().into_owned(), None).expect("the kernel is needed for this test")
    }

    fn at(y: i64, mo: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn check(lang: Lang, what: &str, field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(lang, v) {
            hits.push(format!("{what}.{field} [{w}]: {v}"));
        }
        let low = v.to_lowercase();
        if let Some(w) = NEVER.iter().find(|w| low.contains(*w)) {
            hits.push(format!("{what}.{field} promises [{w}]: {v}"));
        }
    }

    #[test]
    fn sure_windows_name_no_astrology_and_promise_nothing() {
        let e = engine();
        let birth = at(1990, 2, 20, 5);
        let mut hits = Vec::new();
        for lang in ["en", "ml", "hi"] {
            let l = Lang::parse(lang);
            let ws = e
                .sure_windows(birth, 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 12.9716, 77.5946, 330, at(2026, 10, 20, 5), 24, vec![], 12, lang.into())
                .unwrap();
            assert!(!ws.is_empty());
            for w in &ws {
                for (field, v) in [
                    ("area", w.area.as_str()),
                    ("area_name", &w.area_name),
                    ("tier", &w.tier),
                    ("date_line", &w.date_line),
                    ("headline", &w.headline),
                    ("detail", &w.detail),
                    ("sure_line", &w.sure_line),
                    ("who", &w.who),
                    ("kind", &w.kind),
                    ("best_line", &w.best_line),
                    ("seal_text", &w.seal_text),
                    ("expert", &w.expert),
                ] {
                    check(l, "window", field, v, &mut hits);
                }
                for (field, list) in [("also_names", &w.also_names), ("systems", &w.systems), ("system_keys", &w.system_keys)] {
                    for v in list {
                        check(l, "window", field, v, &mut hits);
                    }
                }
                for v in w.remedy_hint.iter().chain(w.best_hours.iter()) {
                    check(l, "window", "remedy_hint/best_hours", v, &mut hits);
                }
                for d in &w.best_days {
                    check(l, "day", "label", &d.label, &mut hits);
                    if let Some(h) = &d.hours {
                        check(l, "day", "hours", h, &mut hits);
                    }
                }
                assert!(["strong", "likely", "possible"].contains(&w.tier.as_str()));
                assert!(w.agree <= w.of && w.systems.len() == w.agree as usize);
                // Every agreeing way of reading has its book, and the pooling says it is the app's.
                let src = crate::sources::source_lines(w.source_keys.clone(), lang.into());
                assert!(src.len() >= w.agree as usize, "{:?}", w.source_keys);
                assert!(src.iter().any(|s| s.kind == "kshana"));
                assert!(w.source_keys.iter().all(|k| kshana_rules::sources::known(k)), "{:?}", w.source_keys);
                if w.is_hard {
                    assert!(w.remedy_hint.is_some(), "{}", w.headline);
                }
                if w.tier != "strong" {
                    assert!(w.who.is_empty());
                }
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }

    #[test]
    fn the_next_strong_window_and_its_hours() {
        let e = engine();
        let birth = at(1990, 2, 20, 5);
        let now = at(2026, 10, 20, 5);
        let next = e.sure_next(birth, 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 9.9312, 76.2673, 330, now, 12, "en".into()).unwrap();
        let Some(w) = next else { return };
        eprintln!("{} | {} | {}", w.date_line, w.headline, w.best_line);
        assert!(w.tier == "strong" || w.tier == "likely");
        let h = e
            .sure_window_hours(birth, 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 9.9312, 76.2673, 330, w.area.clone(), w.is_hard, w.start, w.end, now, Some(w.peak), w.system_keys.clone(), "en".into())
            .unwrap();
        eprintln!("{} {:?}", h.line, h.hours);
        assert!(h.days.len() <= 3);
        if w.tier == "strong" {
            // Read from the window's own peak and systems, the days are the window's.
            assert_eq!(h.line, w.best_line);
        }
        for d in &h.days {
            assert!(d.end.tai_seconds > w.start.tai_seconds && d.start.tai_seconds < w.end.tai_seconds);
        }
        assert!(e
            .sure_window_hours(birth, 9.9312, 76.2673, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 9.9312, 76.2673, 330, "nonsense".into(), false, w.start, w.end, now, None, vec![], "en".into())
            .is_err());
    }
}
