//! The life ledger and transit scoring over the FFI.

use kshana_chart::Chart;
use kshana_ledger::{EventKind, LifeEvent};
use kshana_rules::Lang;
use kshana_time::Instant;
use kshana_vedic::DashaLord;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// A logged life event.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LedgerEventRecord {
    pub at: ExactInstant,
    /// One of Career, Marriage, Children, Wealth, Health, Property, Education, Foreign.
    pub area: String,
    /// Began, Ended, Gain, Loss, Milestone, Crisis.
    pub kind: String,
    pub weight: f64,
    pub note: String,
}

fn to_event(r: &LedgerEventRecord) -> Result<LifeEvent, KshanaError> {
    let kind = EventKind::from_name(&r.kind)
        .ok_or_else(|| KshanaError::Engine(format!("unknown event kind {}", r.kind)))?;
    LifeEvent::new(r.at.into(), &r.area, kind, r.weight, &r.note)
        .map_err(|e| KshanaError::Engine(e.to_string()))
}

fn from_event(e: &LifeEvent) -> LedgerEventRecord {
    LedgerEventRecord {
        at: e.at.into(),
        area: e.area.clone(),
        kind: e.kind.name().to_string(),
        weight: e.weight,
        note: e.note.clone(),
    }
}

/// What the engine would have said about an event.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LedgerScoreRecord {
    pub dasha_support: f64,
    pub parashari: f64,
    pub transit: f64,
    pub fit: f64,
    pub adverse: bool,
    pub running: Vec<String>,
    pub jupiter_from_moon: u8,
    pub saturn_from_moon: u8,
    pub sade_sati_phase: u8,
    pub explanation: Vec<String>,
    pub words: String,
}

/// One reliability bin.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ReliabilityBinRecord {
    pub center: f64,
    pub predicted_mean: f64,
    pub observed_rate: f64,
    pub count: u32,
}

/// How the fits compare with chance.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LedgerCalibrationRecord {
    pub n: u32,
    pub baseline_n: u32,
    pub mean_fit: f64,
    pub support_rate: f64,
    pub base_rate: f64,
    pub lift: f64,
    pub brier: f64,
    pub reliability: Vec<ReliabilityBinRecord>,
    pub verdict: String,
    pub scores: Vec<LedgerScoreRecord>,
}

/// One antardasha of the past, read from the chart.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PeriodReadingRecord {
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub maha: String,
    pub antar: String,
    /// Nine strengths in AREAS order.
    pub area_strengths: Vec<f64>,
    pub top_areas: Vec<String>,
    pub jupiter_from_moon: u8,
    pub saturn_from_moon: u8,
    pub sade_sati_phase: u8,
    pub words: String,
}

/// One transit window of a graha through a sidereal sign.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitWindowRecord {
    pub body: String,
    pub sign: u8,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub house_from_moon: u8,
    pub house_from_lagna: u8,
    pub bav: u8,
    pub sav: u8,
    pub favourable: bool,
    pub vedha_by: Option<String>,
    pub verdict_words: String,
    pub moorthi: Option<String>,
    pub score: f64,
    pub factors: Vec<String>,
}

/// One kakshya (3°45′) window.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KakshyaWindowRecord {
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub sign: u8,
    pub kakshya: u8,
    pub lord: String,
    pub favourable: bool,
}

/// The sky's score for one instant.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitScoreRecord {
    pub score: f64,
    pub lines: Vec<String>,
}

fn lord(l: DashaLord) -> String {
    format!("{l:?}")
}

fn lord_of(name: &str) -> Option<DashaLord> {
    Some(match name {
        "Sun" => DashaLord::Sun,
        "Moon" => DashaLord::Moon,
        "Mars" => DashaLord::Mars,
        "Mercury" => DashaLord::Mercury,
        "Jupiter" => DashaLord::Jupiter,
        "Venus" => DashaLord::Venus,
        "Saturn" => DashaLord::Saturn,
        "Rahu" => DashaLord::Rahu,
        "Ketu" => DashaLord::Ketu,
        _ => return None,
    })
}

fn score_record(s: &kshana_ledger::EventScore, ev: &LifeEvent, lang: Lang) -> LedgerScoreRecord {
    LedgerScoreRecord {
        dasha_support: s.dasha_support,
        parashari: s.parashari,
        transit: s.transit,
        fit: s.fit,
        adverse: s.adverse,
        running: s.running.iter().map(|l| lord(*l)).collect(),
        jupiter_from_moon: s.jupiter_from_moon,
        saturn_from_moon: s.saturn_from_moon,
        sade_sati_phase: s.sade_sati_phase,
        explanation: s.explanation.clone(),
        words: kshana_ledger::fit_words_in(s, ev, lang),
    }
}

fn err<E: std::fmt::Display>(e: E) -> KshanaError {
    KshanaError::Engine(e.to_string())
}

fn chart_of(
    engine: &Engine,
    birth: ExactInstant,
    latitude_deg: f64,
    longitude_deg: f64,
    ayanamsa: AyanamsaChoice,
) -> Result<Chart, KshanaError> {
    natal_chart(engine, birth, latitude_deg, longitude_deg, ayanamsa)
}

#[uniffi::export]
impl Engine {
    /// Score every logged event against the chart.
    pub fn score_events(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        events: Vec<LedgerEventRecord>,
        lang: String,
    ) -> Result<Vec<LedgerScoreRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let mut out = Vec::with_capacity(events.len());
        for r in &events {
            let ev = to_event(r)?;
            let s = kshana_ledger::score_event_in(&self.spk, &ch, ayanamsa.into(), &ev, lang)
                .map_err(err)?;
            out.push(score_record(&s, &ev, lang));
        }
        Ok(out)
    }

    /// Score the events and compare them with `per_event` matched baseline instants
    /// each, spread over `[from, to]`: the honest answer to "did the chart know".
    #[allow(clippy::too_many_arguments)]
    pub fn calibrate_ledger(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        events: Vec<LedgerEventRecord>,
        from: ExactInstant,
        to: ExactInstant,
        per_event: u32,
        lang: String,
    ) -> Result<LedgerCalibrationRecord, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let ay = ayanamsa.into();
        let evs: Vec<LifeEvent> = events.iter().map(to_event).collect::<Result<_, _>>()?;
        let mut scores = Vec::with_capacity(evs.len());
        let mut records = Vec::with_capacity(evs.len());
        for ev in &evs {
            let s = kshana_ledger::score_event_in(&self.spk, &ch, ay, ev, lang).map_err(err)?;
            records.push(score_record(&s, ev, lang));
            scores.push(s);
        }
        let baseline = kshana_ledger::baseline_for_ledger(
            &self.spk,
            &ch,
            ay,
            &evs,
            from.into(),
            to.into(),
            per_event.clamp(1, 60) as usize,
        )
        .map_err(err)?;
        let c = kshana_ledger::calibrate(&scores, &baseline);
        Ok(LedgerCalibrationRecord {
            n: c.n as u32,
            baseline_n: c.baseline_n as u32,
            mean_fit: c.mean_fit,
            support_rate: c.support_rate,
            base_rate: c.base_rate,
            lift: c.lift,
            brier: c.brier,
            reliability: c
                .reliability
                .iter()
                .map(|b| ReliabilityBinRecord {
                    center: b.center,
                    predicted_mean: b.predicted_mean,
                    observed_rate: b.observed_rate,
                    count: b.count as u32,
                })
                .collect(),
            verdict: kshana_ledger::calibration_verdict(&c, lang),
            scores: records,
        })
    }

    /// Every antardasha overlapping `[from, to]`, read from the chart.
    pub fn retrodict(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
        lang: String,
    ) -> Result<Vec<PeriodReadingRecord>, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let r = kshana_ledger::retrodict_in(
            &self.spk,
            &ch,
            ayanamsa.into(),
            from.into(),
            to.into(),
            Lang::parse(&lang),
        )
        .map_err(err)?;
        Ok(r.into_iter()
            .map(|p| PeriodReadingRecord {
                start: p.start.into(),
                end: p.end.into(),
                maha: lord(p.maha),
                antar: lord(p.antar),
                area_strengths: p.area_strengths.to_vec(),
                top_areas: p.top_areas,
                jupiter_from_moon: p.jupiter_from_moon,
                saturn_from_moon: p.saturn_from_moon,
                sade_sati_phase: p.sade_sati_phase,
                words: p.words,
            })
            .collect())
    }

    /// Transit windows of the nine grahas through the sidereal signs over `[from, to]`.
    pub fn transit_windows(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
        lang: String,
    ) -> Result<Vec<TransitWindowRecord>, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let from_i: Instant = from.into();
        let to_i: Instant = to.into();
        if to_i.ns_since(from_i) > 800 * 86_400 * 1_000_000_000 {
            return Err(KshanaError::Engine(
                "transit windows cover at most 800 days per call".to_string(),
            ));
        }
        let w = kshana_forecast::transit_windows(&self.spk, &ch, ayanamsa.into(), from_i, to_i, lang)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(w.into_iter()
            .map(|t| TransitWindowRecord {
                body: lord(t.body),
                sign: t.sign,
                start: t.start.into(),
                end: t.end.into(),
                house_from_moon: t.house_from_moon,
                house_from_lagna: t.house_from_lagna,
                bav: t.bav,
                sav: t.sav,
                favourable: t.verdict.favourable,
                vedha_by: t
                    .verdict
                    .vedha_by
                    .map(|g| kshana_speak::lexicon::graha(g, lang).to_string()),
                verdict_words: t.verdict.words,
                moorthi: t.moorthi.map(|m| m.name_in(lang).to_string()),
                score: t.score,
                factors: t.factors,
            })
            .collect())
    }

    /// Kakshya windows of one graha (name) over `[from, to]`.
    #[allow(clippy::too_many_arguments)]
    pub fn kakshya_windows(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        planet: String,
        from: ExactInstant,
        to: ExactInstant,
    ) -> Result<Vec<KakshyaWindowRecord>, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let g = lord_of(&planet)
            .ok_or_else(|| KshanaError::Engine(format!("unknown graha {planet}")))?;
        let facts = kshana_rules::Facts::from_chart(&ch);
        let w = kshana_forecast::kakshya_windows(
            &self.spk,
            &facts,
            g,
            ayanamsa.into(),
            from.into(),
            to.into(),
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(w.into_iter()
            .map(|k| KakshyaWindowRecord {
                start: k.start.into(),
                end: k.end.into(),
                sign: k.sign,
                kakshya: k.kakshya,
                lord: k.kakshya_lord.name().to_string(),
                favourable: k.favourable,
            })
            .collect())
    }

    /// The nine grahas' transit score for one instant, with a line each.
    pub fn transit_score(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        lang: String,
    ) -> Result<TransitScoreRecord, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let (score, lines) =
            kshana_forecast::today_transit_score(&self.spk, &ch, ayanamsa.into(), at.into(), lang)
                .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(TransitScoreRecord { score, lines })
    }
}

/// A ledger event as one JSON line, for storage.
#[uniffi::export]
pub fn ledger_event_to_json(event: LedgerEventRecord) -> Result<String, KshanaError> {
    Ok(to_event(&event)?.to_json_line())
}

/// A ledger event from one JSON line.
#[uniffi::export]
pub fn ledger_event_from_json(line: String) -> Result<LedgerEventRecord, KshanaError> {
    LifeEvent::from_json_line(&line)
        .map(|e| from_event(&e))
        .map_err(err)
}

/// The event kinds the ledger accepts.
#[uniffi::export]
pub fn ledger_event_kinds() -> Vec<String> {
    EventKind::ALL
        .iter()
        .map(|k| k.name().to_string())
        .collect()
}

/// An event kind's display name in the reader's language ("Loss" → "നഷ്ടം"); the
/// kind itself stays the English key the ledger stores. Unknown kinds come back as given.
#[uniffi::export]
pub fn ledger_event_kind_name(kind: String, lang: String) -> String {
    match EventKind::from_name(&kind) {
        Some(k) => kshana_ledger::kind_word(Lang::parse(&lang), k).to_string(),
        None => kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn ledger_and_transits_cross_the_boundary() {
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
        let ev = LedgerEventRecord {
            at: u(2013, 7, 1, 4, 0),
            area: "Career".into(),
            kind: "Began".into(),
            weight: 0.8,
            note: "first job".into(),
        };
        let line = ledger_event_to_json(ev.clone()).unwrap();
        let back = ledger_event_from_json(line).unwrap();
        assert_eq!(back.area, "Career");
        let s = engine
            .score_events(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                vec![ev.clone()],
                "en".into(),
            )
            .unwrap();
        assert_eq!(s.len(), 1);
        assert!((0.0..=1.0).contains(&s[0].fit) && !s[0].words.is_empty());
        let c = engine
            .calibrate_ledger(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                vec![ev],
                u(2005, 1, 1, 0, 0),
                u(2026, 1, 1, 0, 0),
                8,
                "ml".into(),
            )
            .unwrap();
        assert_eq!(c.n, 1);
        assert_eq!(c.baseline_n, 8);
        assert_eq!(c.reliability.len(), 5);
        let r = engine
            .retrodict(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                u(2010, 1, 1, 0, 0),
                u(2012, 1, 1, 0, 0),
                "hi".into(),
            )
            .unwrap();
        assert!(!r.is_empty());
        let w = engine
            .transit_windows(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                u(2026, 10, 1, 0, 0),
                u(2026, 11, 1, 0, 0),
                "en".into(),
            )
            .unwrap();
        assert!(w.iter().any(|x| x.body == "Moon"));
        let k = engine
            .kakshya_windows(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                "Saturn".into(),
                u(2026, 10, 1, 0, 0),
                u(2026, 11, 1, 0, 0),
            )
            .unwrap();
        assert!(!k.is_empty());
        let t = engine
            .transit_score(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                u(2026, 10, 16, 3, 30),
                "ml".into(),
            )
            .unwrap();
        assert_eq!(t.lines.len(), 9);
        assert_eq!(ledger_event_kinds().len(), 6);
    }
}
