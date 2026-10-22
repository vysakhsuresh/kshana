//! The honest-measurement layer of the life ledger over the FFI: the exact coverage
//! tally, the time-shift test, calibration metrics, sealed forecasts, the asked-first
//! quiz and the birth-time posterior.

use kshana_chart::Chart;
use kshana_ledger::{EventKind, LifeEvent, Metrics, Question, SealedForecast};
use kshana_rules::Lang;
use kshana_sky::Observer;
use kshana_time::Instant;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError, LedgerEventRecord};

/// One event in a tally.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TallyEventRecord {
    pub area: String,
    pub kind: String,
    pub at: ExactInstant,
    /// Fit from the exact profile at the instant.
    pub fit: f64,
    /// Fit of the full score at the instant.
    pub exact_fit: f64,
    /// Coverage of the area and orientation over the span.
    pub coverage: f64,
    pub supported: bool,
    /// The area cannot fail: shown, not counted.
    pub cannot_fail: bool,
}

/// The headline tally with its exact chance line.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TallyRecord {
    pub supported: u32,
    pub total: u32,
    /// Expected hits by chance, `Σ coverage`.
    pub expected: f64,
    /// Standard deviation of the chance hits.
    pub sd: f64,
    /// Exact Poisson-binomial `P[hits ≥ supported]`.
    pub p_value: f64,
    pub excluded: u32,
    pub threshold: f64,
    pub per_event: Vec<TallyEventRecord>,
    pub coverage_words: Vec<String>,
    pub words: String,
}

/// The circular time-shift test.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SurrogateRecord {
    pub p: f64,
    pub real_mean: f64,
    pub shifted_means: Vec<f64>,
    pub shifts: u32,
    pub n: u32,
    pub words: String,
}

/// One bin of a calibration curve.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct MetricsBinRecord {
    pub lo: f64,
    pub hi: f64,
    pub count: u32,
    pub predicted_mean: f64,
    pub observed_rate: f64,
    pub observed_lo: f64,
    pub observed_hi: f64,
    pub shown: bool,
}

/// Brier, Murphy decomposition, skill, ECE and the curve.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct MetricsRecord {
    pub n: u32,
    pub positives: u32,
    pub base_rate: f64,
    pub base_rate_lo: f64,
    pub base_rate_hi: f64,
    pub brier: f64,
    pub brier_reference: f64,
    pub skill: f64,
    pub reliability: f64,
    pub resolution: f64,
    pub uncertainty: f64,
    pub within_bin_variance: f64,
    pub within_bin_covariance: f64,
    pub ece: f64,
    pub bins: Vec<MetricsBinRecord>,
    pub curve_shown: bool,
    pub words: String,
}

/// A sealed (pre-registered) forecast.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SealedRecord {
    /// SHA-256 of the sealed claim.
    pub id: String,
    pub created_at: ExactInstant,
    pub area: String,
    pub kind: String,
    pub window_start: ExactInstant,
    pub window_end: ExactInstant,
    pub probability: f64,
    pub basis: Vec<String>,
    pub question: String,
    pub resolved: bool,
    pub happened: bool,
    pub resolved_at: Option<ExactInstant>,
    /// Whether `id` still matches the claim.
    pub verified: bool,
}

/// One committed asked-first question.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct QuizQuestionRecord {
    pub area: String,
    /// Window starts, in the order offered.
    pub starts: Vec<ExactInstant>,
    /// Window ends, in the order offered.
    pub ends: Vec<ExactInstant>,
    /// The answer key: hide until answered.
    pub predicted_index: u32,
    pub prompt: String,
    /// SHA-256 commitment of the windows and the key, to log before asking.
    pub commitment: String,
}

/// The grade of a quiz.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct QuizResultRecord {
    pub correct: u32,
    pub total: u32,
    pub skipped: u32,
    pub chance_rate: f64,
    pub binomial_pvalue: f64,
    pub hit_lo: f64,
    pub hit_hi: f64,
    pub words: String,
}

/// The posterior over birth time.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PosteriorRecord {
    /// Candidate instants in time order.
    pub instants: Vec<ExactInstant>,
    /// Posterior mass of each candidate; sums to 1.
    pub masses: Vec<f64>,
    pub log_likelihood: Vec<f64>,
    pub step_seconds: f64,
    pub map: ExactInstant,
    pub mean: ExactInstant,
    pub hpd50_start: ExactInstant,
    pub hpd50_end: ExactInstant,
    pub hpd90_start: ExactInstant,
    pub hpd90_end: ExactInstant,
    pub hpd50_runs: u32,
    pub hpd90_runs: u32,
    pub events: u32,
    pub words: String,
}

fn err<E: std::fmt::Display>(e: E) -> KshanaError {
    KshanaError::Engine(e.to_string())
}

fn to_event(r: &LedgerEventRecord) -> Result<LifeEvent, KshanaError> {
    let kind = EventKind::from_name(&r.kind)
        .ok_or_else(|| KshanaError::Engine(format!("unknown event kind {}", r.kind)))?;
    LifeEvent::new(r.at.into(), &r.area, kind, r.weight, &r.note).map_err(err)
}

fn to_events(rs: &[LedgerEventRecord]) -> Result<Vec<LifeEvent>, KshanaError> {
    rs.iter().map(to_event).collect()
}

fn kind_of(name: &str) -> Result<EventKind, KshanaError> {
    EventKind::from_name(name)
        .ok_or_else(|| KshanaError::Engine(format!("unknown event kind {name}")))
}

fn metrics_record(m: &Metrics) -> MetricsRecord {
    MetricsRecord {
        n: m.n as u32,
        positives: m.positives as u32,
        base_rate: m.base_rate,
        base_rate_lo: m.base_rate_interval.0,
        base_rate_hi: m.base_rate_interval.1,
        brier: m.brier,
        brier_reference: m.brier_reference,
        skill: m.skill,
        reliability: m.reliability,
        resolution: m.resolution,
        uncertainty: m.uncertainty,
        within_bin_variance: m.within_bin_variance,
        within_bin_covariance: m.within_bin_covariance,
        ece: m.ece,
        bins: m
            .bins
            .iter()
            .map(|b| MetricsBinRecord {
                lo: b.lo,
                hi: b.hi,
                count: b.count as u32,
                predicted_mean: b.predicted_mean,
                observed_rate: b.observed_rate,
                observed_lo: b.observed_interval.0,
                observed_hi: b.observed_interval.1,
                shown: b.shown,
            })
            .collect(),
        curve_shown: m.curve_shown,
        words: m.words.clone(),
    }
}

fn sealed_record(f: &SealedForecast) -> SealedRecord {
    SealedRecord {
        id: f.id.clone(),
        created_at: f.created_at.into(),
        area: f.area.clone(),
        kind: f.kind.name().to_string(),
        window_start: f.window_start.into(),
        window_end: f.window_end.into(),
        probability: f.probability,
        basis: f.basis.clone(),
        question: f.question.clone(),
        resolved: f.resolved.is_some(),
        happened: f.resolved.is_some_and(|r| r.0),
        resolved_at: f.resolved.map(|r| r.1.into()),
        verified: f.verify(),
    }
}

fn sealed_of(r: &SealedRecord) -> Result<SealedForecast, KshanaError> {
    Ok(SealedForecast {
        id: r.id.clone(),
        created_at: r.created_at.into(),
        area: r.area.clone(),
        kind: kind_of(&r.kind)?,
        window_start: r.window_start.into(),
        window_end: r.window_end.into(),
        probability: r.probability,
        basis: r.basis.clone(),
        question: r.question.clone(),
        resolved: if r.resolved {
            Some((
                r.happened,
                Instant::from(r.resolved_at.unwrap_or(r.window_end)),
            ))
        } else {
            None
        },
    })
}

fn question_record(q: &Question) -> QuizQuestionRecord {
    QuizQuestionRecord {
        area: q.area.clone(),
        starts: q.windows.iter().map(|w| w.0.into()).collect(),
        ends: q.windows.iter().map(|w| w.1.into()).collect(),
        predicted_index: q.predicted_index as u32,
        prompt: q.prompt.clone(),
        commitment: q.commitment.clone(),
    }
}

fn question_of(r: &QuizQuestionRecord) -> Question {
    Question {
        area: r.area.clone(),
        windows: r
            .starts
            .iter()
            .zip(&r.ends)
            .map(|(s, e)| ((*s).into(), (*e).into()))
            .collect(),
        predicted_index: r.predicted_index as usize,
        prompt: r.prompt.clone(),
        commitment: r.commitment.clone(),
    }
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
    /// "31 of 40 supported; chance would give 27 ± 3": the events judged at the
    /// support threshold against the exact coverage of each area over `[from, to]`.
    #[allow(clippy::too_many_arguments)]
    pub fn coverage_tally(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        events: Vec<LedgerEventRecord>,
        from: ExactInstant,
        to: ExactInstant,
    ) -> Result<TallyRecord, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let evs = to_events(&events)?;
        let t = kshana_ledger::tally(
            &self.spk,
            &ch,
            ayanamsa.into(),
            &evs,
            from.into(),
            to.into(),
            kshana_ledger::SUPPORT_THRESHOLD,
        )
        .map_err(err)?;
        Ok(TallyRecord {
            supported: t.supported as u32,
            total: t.total as u32,
            expected: t.expected,
            sd: t.sd,
            p_value: t.p_value,
            excluded: t.excluded as u32,
            threshold: t.threshold,
            per_event: t
                .per_event
                .iter()
                .map(|e| TallyEventRecord {
                    area: e.area.clone(),
                    kind: e.kind.name().to_string(),
                    at: e.at.into(),
                    fit: e.fit,
                    exact_fit: e.exact_fit,
                    coverage: e.coverage,
                    supported: e.supported,
                    cannot_fail: e.cannot_fail,
                })
                .collect(),
            coverage_words: t.coverage_words,
            words: t.words,
        })
    }

    /// The circular time-shift test over `[from, to]` with `shifts` offsets (at most
    /// 20 000).
    #[allow(clippy::too_many_arguments)]
    pub fn time_shift_test(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        events: Vec<LedgerEventRecord>,
        from: ExactInstant,
        to: ExactInstant,
        shifts: u32,
        lang: String,
    ) -> Result<SurrogateRecord, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let evs = to_events(&events)?;
        let r = kshana_ledger::time_shift_pvalue(
            &self.spk,
            &ch,
            ayanamsa.into(),
            &evs,
            from.into(),
            to.into(),
            shifts.min(20_000) as usize,
        )
        .map_err(err)?;
        let words = kshana_ledger::surrogate_words(&r, Lang::parse(&lang));
        Ok(SurrogateRecord {
            p: r.p,
            real_mean: r.real_mean,
            shifted_means: r.shifted_means,
            shifts: r.shifts as u32,
            n: r.n as u32,
            words,
        })
    }

    /// Calibration metrics of the Explained ledger: the events as positives and
    /// `per_event` matched baseline instants each over `[from, to]` as negatives,
    /// scored by their fits over `bins` bins (0 for the cube-root rule).
    #[allow(clippy::too_many_arguments)]
    pub fn ledger_metrics(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        events: Vec<LedgerEventRecord>,
        from: ExactInstant,
        to: ExactInstant,
        per_event: u32,
        bins: u32,
        lang: String,
    ) -> Result<MetricsRecord, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let ay = ayanamsa.into();
        let evs = to_events(&events)?;
        let mut ps = Vec::with_capacity(evs.len());
        let mut labels = Vec::with_capacity(evs.len());
        for ev in &evs {
            ps.push(
                kshana_ledger::score_event(&self.spk, &ch, ay, ev)
                    .map_err(err)?
                    .fit,
            );
            labels.push(true);
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
        labels.extend(std::iter::repeat(false).take(baseline.len()));
        ps.extend(baseline);
        let mut m = kshana_ledger::metrics(&ps, &labels, bins as usize);
        m.words = kshana_ledger::metrics_words(&m, Lang::parse(&lang));
        Ok(metrics_record(&m))
    }

    /// Seal a forecast for `area` and `kind` over `[window_start, window_end]` at
    /// `created_at`; refused unless the window opens after creation.
    #[allow(clippy::too_many_arguments)]
    pub fn seal_forecast(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        created_at: ExactInstant,
        area: String,
        kind: String,
        window_start: ExactInstant,
        window_end: ExactInstant,
    ) -> Result<SealedRecord, KshanaError> {
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = kshana_ledger::seal(
            &self.spk,
            &ch,
            ayanamsa.into(),
            created_at.into(),
            &area,
            kind_of(&kind)?,
            (window_start.into(), window_end.into()),
        )
        .map_err(err)?;
        Ok(sealed_record(&f))
    }

    /// Commit `n_questions` asked-first questions for `area` over `[from, to]`, each
    /// with `decoys` decoy windows beside the chart's own.
    #[allow(clippy::too_many_arguments)]
    pub fn retrodiction_quiz(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        area: String,
        from: ExactInstant,
        to: ExactInstant,
        n_questions: u32,
        decoys: u32,
        lang: String,
    ) -> Result<Vec<QuizQuestionRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = chart_of(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let qs = kshana_ledger::retrodiction_quiz(
            &self.spk,
            &ch,
            ayanamsa.into(),
            &area,
            from.into(),
            to.into(),
            n_questions.min(50) as usize,
            decoys.min(9) as usize,
        )
        .map_err(err)?;
        Ok(qs
            .iter()
            .map(|q| QuizQuestionRecord {
                prompt: kshana_ledger::question_prompt(q, lang),
                ..question_record(q)
            })
            .collect())
    }

    /// The posterior over birth time across `±half_window_seconds` around `birth` in
    /// `step_seconds` steps (widened to at most 400 candidates), from the events.
    #[allow(clippy::too_many_arguments)]
    pub fn birth_posterior(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        half_window_seconds: i64,
        step_seconds: i64,
        events: Vec<LedgerEventRecord>,
    ) -> Result<PosteriorRecord, KshanaError> {
        let evs = to_events(&events)?;
        let obs = Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        };
        let p = kshana_ledger::birth_posterior(
            &self.spk,
            self.eop.as_ref(),
            &obs,
            kshana_houses::HouseSystem::Placidus,
            ayanamsa.into(),
            birth.into(),
            half_window_seconds,
            step_seconds,
            &evs,
        )
        .map_err(err)?;
        Ok(PosteriorRecord {
            instants: p.grid.iter().map(|g| g.0.into()).collect(),
            masses: p.grid.iter().map(|g| g.1).collect(),
            log_likelihood: p.log_likelihood,
            step_seconds: p.step_ns as f64 / 1e9,
            map: p.map.into(),
            mean: p.mean.into(),
            hpd50_start: p.hpd50.0.into(),
            hpd50_end: p.hpd50.1.into(),
            hpd90_start: p.hpd90.0.into(),
            hpd90_end: p.hpd90.1.into(),
            hpd50_runs: p.hpd50_runs as u32,
            hpd90_runs: p.hpd90_runs as u32,
            events: p.events as u32,
            words: p.words,
        })
    }
}

/// Record the outcome of a sealed forecast stored as a JSON line.
#[uniffi::export]
pub fn resolve_forecast(
    json: String,
    happened: bool,
    when: ExactInstant,
) -> Result<SealedRecord, KshanaError> {
    let f = SealedForecast::from_json_line(&json).map_err(err)?;
    Ok(sealed_record(&kshana_ledger::resolve(
        &f,
        happened,
        when.into(),
    )))
}

/// A sealed forecast as one JSON line, for storage.
#[uniffi::export]
pub fn sealed_forecast_to_json(forecast: SealedRecord) -> Result<String, KshanaError> {
    Ok(sealed_of(&forecast)?.to_json_line())
}

/// A sealed forecast from one JSON line; `verified` says whether its id still
/// matches the claim.
#[uniffi::export]
pub fn sealed_forecast_from_json(line: String) -> Result<SealedRecord, KshanaError> {
    SealedForecast::from_json_line(&line)
        .map(|f| sealed_record(&f))
        .map_err(err)
}

/// Brier and calibration over the resolved forecasts among `forecasts`.
#[uniffi::export]
pub fn sealed_forecast_metrics(forecasts: Vec<SealedRecord>) -> Result<MetricsRecord, KshanaError> {
    let fs: Vec<SealedForecast> = forecasts.iter().map(sealed_of).collect::<Result<_, _>>()?;
    Ok(metrics_record(&kshana_ledger::sealed_brier(&fs)))
}

/// Grade quiz answers (the chosen window's index, or a negative number for "not
/// sure") against the questions, paired by position.
#[uniffi::export]
pub fn grade_quiz(questions: Vec<QuizQuestionRecord>, answers: Vec<i64>, lang: String) -> QuizResultRecord {
    let qs: Vec<Question> = questions.iter().map(question_of).collect();
    let ans: Vec<Option<usize>> = answers.iter().map(|a| usize::try_from(*a).ok()).collect();
    let mut r = kshana_ledger::grade(&qs, &ans);
    r.words = kshana_ledger::grade_words(&r, Lang::parse(&lang));
    QuizResultRecord {
        correct: r.correct as u32,
        total: r.total as u32,
        skipped: r.skipped as u32,
        chance_rate: r.chance_rate,
        binomial_pvalue: r.binomial_pvalue,
        hit_lo: r.hit_interval.0,
        hit_hi: r.hit_interval.1,
        words: r.words,
    }
}

/// Whether a committed question's commitment still matches its windows and key.
#[uniffi::export]
pub fn verify_quiz_question(question: QuizQuestionRecord) -> bool {
    kshana_ledger::verify_question(&question_of(&question))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn science_crosses_the_boundary() {
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
        let ev = |y, mo, d, area: &str, kind: &str| LedgerEventRecord {
            at: u(y, mo, d, 6, 0),
            area: area.into(),
            kind: kind.into(),
            weight: 1.0,
            note: String::new(),
        };
        let events = vec![
            ev(2013, 7, 1, "Career", "Began"),
            ev(2018, 11, 22, "Marriage", "Milestone"),
            ev(2020, 6, 15, "Health", "Crisis"),
        ];
        let from = u(2005, 1, 1, 0, 0);
        let to = u(2026, 1, 1, 0, 0);

        let t = engine
            .coverage_tally(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                events.clone(),
                from,
                to,
            )
            .unwrap();
        assert_eq!(t.per_event.len(), 3);
        assert_eq!(t.total + t.excluded, 3);
        assert!((0.0..=1.0).contains(&t.p_value));
        assert!(!t.words.is_empty() && t.coverage_words.len() == 3);

        let s = engine
            .time_shift_test(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                events.clone(),
                from,
                to,
                50,
                "en".into(),
            )
            .unwrap();
        assert_eq!(s.shifted_means.len(), 50);
        assert!((0.0..=1.0).contains(&s.p));

        let m = engine
            .ledger_metrics(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                events.clone(),
                from,
                to,
                4,
                0,
                "ml".into(),
            )
            .unwrap();
        assert_eq!(m.n, 15);
        assert_eq!(m.positives, 3);
        assert!(!m.bins.is_empty() && !m.words.is_empty());

        let sealed = engine
            .seal_forecast(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                u(2026, 10, 1, 0, 0),
                "Career".into(),
                "Gain".into(),
                u(2027, 1, 1, 0, 0),
                u(2027, 7, 1, 0, 0),
            )
            .unwrap();
        assert!(sealed.verified && !sealed.resolved);
        assert_eq!(sealed.id.len(), 64);
        let line = sealed_forecast_to_json(sealed.clone()).unwrap();
        let back = sealed_forecast_from_json(line.clone()).unwrap();
        assert_eq!(back.id, sealed.id);
        assert!(back.verified);
        let resolved = resolve_forecast(line.clone(), true, u(2027, 8, 1, 0, 0)).unwrap();
        assert!(resolved.resolved && resolved.happened && resolved.verified);
        assert!(resolved.resolved_at.is_some());
        let tampered = line.replace("\"area\":\"Career\"", "\"area\":\"Wealth\"");
        assert!(!sealed_forecast_from_json(tampered).unwrap().verified);
        let sm = sealed_forecast_metrics(vec![sealed.clone(), resolved]).unwrap();
        assert_eq!(sm.n, 1);
        assert!(engine
            .seal_forecast(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                u(2027, 2, 1, 0, 0),
                "Career".into(),
                "Gain".into(),
                u(2027, 1, 1, 0, 0),
                u(2027, 7, 1, 0, 0),
            )
            .is_err());

        let qs = engine
            .retrodiction_quiz(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                "Career".into(),
                u(2000, 1, 1, 0, 0),
                to,
                3,
                2,
                "en".into(),
            )
            .unwrap();
        assert_eq!(qs.len(), 3);
        for q in &qs {
            assert_eq!(q.starts.len(), 3);
            assert_eq!(q.ends.len(), 3);
            assert!(q.predicted_index < 3);
            assert!(verify_quiz_question(q.clone()));
        }
        let answers: Vec<i64> = qs.iter().map(|q| q.predicted_index as i64).collect();
        let g = grade_quiz(qs.clone(), answers, "en".into());
        assert_eq!((g.correct, g.total, g.skipped), (3, 3, 0));
        assert!((g.chance_rate - 1.0 / 3.0).abs() < 1e-12);
        let partial = grade_quiz(qs.clone(), vec![-1, qs[1].predicted_index as i64], "hi".into());
        assert_eq!((partial.correct, partial.total, partial.skipped), (1, 1, 2));

        let post = engine
            .birth_posterior(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                600,
                120,
                events,
            )
            .unwrap();
        assert_eq!(post.instants.len(), 11);
        assert_eq!(post.masses.len(), 11);
        let sum: f64 = post.masses.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert!(Instant::from(post.hpd90_start) <= Instant::from(post.hpd50_start));
        assert!(Instant::from(post.hpd90_end) >= Instant::from(post.hpd50_end));
        assert!(post.words.contains("rated C"));
        assert!(engine
            .birth_posterior(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                600,
                120,
                vec![]
            )
            .is_err());
    }
}
