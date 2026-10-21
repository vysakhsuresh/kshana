//! The trust loop over the FFI: every firm dated prediction sealed before its window
//! opens, answered once it closes, one plain scoreboard over everything answered, and
//! the person's own learned weights fed into the forecast.
//!
//! Sealing is generic: anything that names a part of life, a direction and a dated
//! window with a probability can be sealed with [`seal_new_claims`] (several, never the
//! same prediction twice) or [`seal_claim`] (one). [`Engine::firm_predictions`] lists
//! the forecast's own firm windows in that shape; any other feed (a window several
//! readings agree on) is passed the same way.

use kshana_ledger::{ClaimInput, Outcome, ScoreItem, ScoreSource, SealedClaim};
use kshana_rules::learn::Technique;
use kshana_rules::trust::{LearnedWeight, Tilt};
use kshana_sky::Observer;
use kshana_time::NS_PER_DAY;

use crate::{focus_of, AyanamsaChoice, Engine, ExactInstant, FutureEventRecord, HouseSystem, KshanaError, SampleRecord};

fn err<E: std::fmt::Display>(e: E) -> KshanaError {
    KshanaError::Engine(e.to_string())
}

/// A dated prediction to seal.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ClaimInputRecord {
    /// What produced it: "weeks", "forecast", "consensus", … (expert view only).
    pub source: String,
    /// The part of life, by its key ("Money").
    pub area: String,
    /// "opening", "pressure" or the feed's own kind (expert view only).
    pub kind: String,
    /// True if the stretch is said to be good.
    pub good: bool,
    /// Window start; must be after the moment of sealing.
    pub window_start: ExactInstant,
    /// Window end.
    pub window_end: ExactInstant,
    /// Probability that it comes as said, 0–1.
    pub probability: f64,
    /// The prediction in plain words.
    pub statement: String,
    /// The yes/no question that resolves it; empty to have it written from the area,
    /// direction and window ([`claim_question`]).
    pub question: String,
}

/// A sealed prediction.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ClaimRecord {
    /// SHA-256 of the sealed claim: its fingerprint.
    pub id: String,
    /// SHA-256 of what is claimed without when: the same prediction, the same key.
    pub key: String,
    /// When it was sealed.
    pub created_at: ExactInstant,
    /// What produced it (expert view only).
    pub source: String,
    /// The part of life's key.
    pub area: String,
    /// The claim's kind (expert view only).
    pub kind: String,
    /// True if said to be good.
    pub good: bool,
    /// Window start.
    pub window_start: ExactInstant,
    /// Window end.
    pub window_end: ExactInstant,
    /// Probability given when sealed.
    pub probability: f64,
    /// The prediction.
    pub statement: String,
    /// The question that resolves it.
    pub question: String,
    /// "yes", "close", "no", or empty while unanswered.
    pub outcome: String,
    /// When answered.
    pub resolved_at: Option<ExactInstant>,
    /// Whether the fingerprint still matches the words.
    pub verified: bool,
    /// The claim as one JSON line, to store exactly as given.
    pub line: String,
}

fn claim_record(c: &SealedClaim) -> ClaimRecord {
    ClaimRecord {
        id: c.id.clone(),
        key: c.key.clone(),
        created_at: c.created_at.into(),
        source: c.source.clone(),
        area: c.area.clone(),
        kind: c.kind.clone(),
        good: c.good,
        window_start: c.window_start.into(),
        window_end: c.window_end.into(),
        probability: c.probability,
        statement: c.statement.clone(),
        question: c.question.clone(),
        outcome: c.resolved.map(|r| r.0.key().to_string()).unwrap_or_default(),
        resolved_at: c.resolved.map(|r| r.1.into()),
        verified: c.verify(),
        line: c.to_json_line(),
    }
}

fn claim_input(r: &ClaimInputRecord, lang: kshana_rules::Lang, lon: f64) -> ClaimInput {
    let question = if r.question.trim().is_empty() {
        kshana_rules::Life::parse(&r.area)
            .map(|life| kshana_rules::trust::claim_question(lang, life, r.good, r.window_start.into(), r.window_end.into(), lon))
            .unwrap_or_default()
    } else {
        r.question.clone()
    };
    ClaimInput {
        source: r.source.clone(),
        area: r.area.clone(),
        kind: r.kind.clone(),
        good: r.good,
        window_start: r.window_start.into(),
        window_end: r.window_end.into(),
        probability: r.probability,
        statement: r.statement.clone(),
        question,
    }
}

/// The plain question for a claim about `area` (a life key) over `[from, to]`, read at
/// longitude `lon`; empty for an unknown area.
#[uniffi::export]
pub fn claim_question(lang: String, area: String, good: bool, from: ExactInstant, to: ExactInstant, lon: f64) -> String {
    kshana_rules::Life::parse(&area)
        .map(|life| kshana_rules::trust::claim_question(kshana_rules::Lang::parse(&lang), life, good, from.into(), to.into(), lon))
        .unwrap_or_default()
}

/// Seal one prediction at `at`; refused once its window has opened.
#[uniffi::export]
pub fn seal_claim(candidate: ClaimInputRecord, at: ExactInstant, lang: String, lon: f64) -> Result<ClaimRecord, KshanaError> {
    let c = kshana_ledger::seal_claim(&claim_input(&candidate, kshana_rules::Lang::parse(&lang), lon), at.into()).map_err(err)?;
    Ok(claim_record(&c))
}

/// Seal at `at` up to `max_new` of `candidates` that none of the `existing` stored lines
/// already claims (same prediction, or the same part of life and direction over an
/// overlapping window) and that open at least a day later; soonest first. Lines that
/// cannot be read are ignored. The new claims come back with the line to store.
#[uniffi::export]
pub fn seal_new_claims(existing: Vec<String>, candidates: Vec<ClaimInputRecord>, at: ExactInstant, max_new: u32, lang: String, lon: f64) -> Vec<ClaimRecord> {
    let have: Vec<SealedClaim> = existing.iter().filter_map(|l| SealedClaim::from_json_line(l).ok()).collect();
    let lang = kshana_rules::Lang::parse(&lang);
    let inputs: Vec<ClaimInput> = candidates.iter().map(|c| claim_input(c, lang, lon)).collect();
    kshana_ledger::select_new(&have, &inputs, at.into(), max_new as usize).iter().map(claim_record).collect()
}

/// A stored claim read back; `verified` says whether its fingerprint still matches.
#[uniffi::export]
pub fn claim_from_json(line: String) -> Result<ClaimRecord, KshanaError> {
    SealedClaim::from_json_line(&line).map(|c| claim_record(&c)).map_err(err)
}

/// Record the answer to a stored claim ("yes", "close" or "no"); the new record's line
/// replaces the old one.
#[uniffi::export]
pub fn resolve_claim(line: String, outcome: String, when: ExactInstant) -> Result<ClaimRecord, KshanaError> {
    let c = SealedClaim::from_json_line(&line).map_err(err)?;
    let o = Outcome::parse(&outcome).ok_or_else(|| KshanaError::Engine(format!("unknown answer {outcome}")))?;
    Ok(claim_record(&kshana_ledger::resolve_claim(&c, o, when.into())))
}

/// One answered item for the scoreboard.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ScoreItemInput {
    /// "sealed", "past" or "quiz".
    pub source: String,
    /// "yes", "close" or "no"; anything else ("not sure") is left out.
    pub outcome: String,
    /// The chance a random guess would have been right: ½ for a yes/no stretch, one
    /// over the number of choices for a quiz question.
    pub chance: f64,
}

/// One source's own line on the scoreboard.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ScoreLineRecord {
    /// "sealed", "past" or "quiz" (expert view only).
    pub source: String,
    /// Points: Yes 1, Close ½.
    pub right: f64,
    /// Answered.
    pub total: u32,
    /// What guessing scores on average.
    pub chance: f64,
    /// The line in words.
    pub words: String,
}

/// The plain scoreboard.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ScoreboardRecord {
    /// Points over everything answered.
    pub right: f64,
    /// "38" or "37½".
    pub right_text: String,
    /// Items answered.
    pub total: u32,
    /// What guessing scores on average.
    pub chance_expected: f64,
    /// Its standard deviation.
    pub chance_sd: f64,
    /// How often guessing alone would do as well.
    pub p_luck: f64,
    /// Enough answers, and guessing would rarely do as well.
    pub beats_chance: bool,
    /// Sealed future first, then the past, then the quiz.
    pub lines: Vec<ScoreLineRecord>,
    /// "For you, Kshana has been right 38 times out of 45."
    pub headline: String,
    /// What guessing would score, and what it means.
    pub chance_line: String,
    /// A kind word on where the score stands.
    pub note: String,
    /// Everything as one text to share.
    pub share_text: String,
}

/// One honest number over every answered item, with its chance line.
#[uniffi::export]
pub fn scoreboard(items: Vec<ScoreItemInput>, lang: String) -> ScoreboardRecord {
    let v: Vec<ScoreItem> = items
        .iter()
        .filter_map(|i| Some(ScoreItem { source: ScoreSource::parse(&i.source)?, points: Outcome::parse(&i.outcome)?.points(), chance: i.chance }))
        .collect();
    let b = kshana_ledger::scoreboard(&v, kshana_rules::Lang::parse(&lang));
    ScoreboardRecord {
        right: b.right,
        right_text: kshana_ledger::points_text(b.right),
        total: b.total as u32,
        chance_expected: b.chance_expected,
        chance_sd: b.chance_sd,
        p_luck: b.p_luck,
        beats_chance: b.beats_chance,
        lines: b.lines.iter().map(|l| ScoreLineRecord { source: l.source.key().into(), right: l.right, total: l.total as u32, chance: l.chance, words: l.words.clone() }).collect(),
        headline: b.headline,
        chance_line: b.chance_line,
        note: b.note,
        share_text: b.share_text,
    }
}

/// One technique's learned standing, as `learn_techniques` returned it.
#[derive(Clone, Debug, uniffi::Record)]
pub struct TechniqueWeightInput {
    /// The technique's key ("yogini").
    pub key: String,
    /// Its learned weight.
    pub weight: f64,
    /// Brier skill against chance.
    pub skill: f64,
    /// Mean log gain over chance.
    pub log_gain: f64,
}

/// The person's learned weights: the `answers` and `techniques` of a
/// `TechniqueLearnRecord`, stored by the app and passed to the forecast.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PersonalWeightsInput {
    /// Answers the weights rest on.
    pub answers: u32,
    /// Every technique's standing.
    pub techniques: Vec<TechniqueWeightInput>,
}

fn technique_of(key: &str) -> Option<Technique> {
    kshana_rules::learn::TECHNIQUES.iter().copied().find(|t| t.key() == key)
}

fn tilt_of(w: &PersonalWeightsInput) -> Tilt {
    let learned: Vec<LearnedWeight> = w
        .techniques
        .iter()
        .filter_map(|t| Some(LearnedWeight { technique: technique_of(&t.key)?, weight: t.weight, skill: t.skill, log_gain: t.log_gain }))
        .collect();
    Tilt::from_learned(w.answers as usize, &learned)
}

/// One technique's share of the person's forecast.
#[derive(Clone, Debug, uniffi::Record)]
pub struct TiltShareRecord {
    /// The technique's key (expert view only).
    pub key: String,
    /// The technique in plain words.
    pub name: String,
    /// Its share of the say, all summing to 1.
    pub share: f64,
}

/// What the person's answers do to their forecast.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ForecastTiltRecord {
    /// Whether the forecast follows the person's weights at all.
    pub active: bool,
    /// Answers the weights rest on.
    pub answers: u32,
    /// Fewest answers before it does.
    pub min_answers: u32,
    /// Each technique's share.
    pub shares: Vec<TiltShareRecord>,
    /// One plain sentence.
    pub plain: String,
}

/// What the learned weights do to the forecast, said plainly.
#[uniffi::export]
pub fn forecast_tilt(weights: PersonalWeightsInput, lang: String) -> ForecastTiltRecord {
    let lang = kshana_rules::Lang::parse(&lang);
    let t = tilt_of(&weights);
    ForecastTiltRecord {
        active: t.is_active(),
        answers: weights.answers,
        min_answers: kshana_rules::trust::FORECAST_MIN_ANSWERS as u32,
        shares: t.shares().iter().map(|(k, s)| TiltShareRecord { key: k.key().into(), name: k.name(lang).into(), share: *s }).collect(),
        plain: t.plain(lang),
    }
}

#[uniffi::export]
impl Engine {
    /// The firm dated predictions worth sealing at `at`, in the shape
    /// [`seal_new_claims`] takes: the stretch of the coming weeks and the forecast's clear
    /// openings and pressures of the next six months, read with the person's weights
    /// when given.
    #[allow(clippy::too_many_arguments)]
    pub fn firm_predictions(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        lat: f64,
        lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        lang: String,
        weights: Option<PersonalWeightsInput>,
    ) -> Result<Vec<ClaimInputRecord>, KshanaError> {
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let born = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let tilt = weights.as_ref().map(tilt_of).unwrap_or_default();
        let f = kshana_rules::trust::firm_predictions(&self.spk, self.eop.as_ref(), &natal, &born, &obs, ayanamsa.into(), kshana_rules::Lang::parse(&lang), at.into(), &tilt)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(f.into_iter()
            .map(|x| ClaimInputRecord {
                source: x.source.into(),
                area: x.life.name().into(),
                kind: x.kind.into(),
                good: x.good,
                window_start: x.from.into(),
                window_end: x.to.into(),
                probability: x.probability,
                statement: x.statement,
                question: x.question,
            })
            .collect())
    }

    /// `future`, read with the person's learned weights: identical to `future` until
    /// they rest on enough answers, then leaning a little on the readings that fit their
    /// past.
    #[allow(clippy::too_many_arguments)]
    pub fn future_personal(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        lat: f64,
        lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        months: u32,
        lang: String,
        focus: Vec<String>,
        weights: PersonalWeightsInput,
    ) -> Result<Vec<FutureEventRecord>, KshanaError> {
        let tilt = tilt_of(&weights);
        if !tilt.is_active() {
            return self.future(birth, birth_lat, birth_lon, lat, lon, system, ayanamsa, at, months, lang, focus);
        }
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let born = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let mut f = focus_of(&focus);
        if f.is_empty() {
            f = kshana_rules::Life::ALL.to_vec();
        }
        let ev = kshana_rules::trust::personal_future(&self.spk, self.eop.as_ref(), &natal, &born, &obs, ayanamsa.into(), at.into(), months as usize, &f, kshana_rules::Lang::parse(&lang), &tilt)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(ev
            .into_iter()
            .map(|e| FutureEventRecord {
                from: e.from.into(),
                to: e.to.into(),
                life: e.life.map(|l| l.name().to_string()),
                kind: e.kind.to_string(),
                headline: e.headline,
                detail: e.detail,
                strength: e.strength * 100.0,
                peak: e.peak.map(Into::into),
            })
            .collect())
    }

    /// `slow_horizon`, read with the person's learned weights; identical until they rest
    /// on enough answers.
    #[allow(clippy::too_many_arguments)]
    pub fn slow_horizon_personal(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        lat: f64,
        lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        count: u32,
        weights: PersonalWeightsInput,
    ) -> Result<Vec<SampleRecord>, KshanaError> {
        let tilt = tilt_of(&weights);
        if !tilt.is_active() {
            return self.slow_horizon(birth, birth_lat, birth_lon, lat, lon, system, ayanamsa, from, count);
        }
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let born = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let month = (30.4375 * NS_PER_DAY as f64) as i128;
        let v = kshana_rules::trust::personal_slow_series(&self.spk, self.eop.as_ref(), &natal, &born, &obs, ayanamsa.into(), from.into(), month, count.clamp(2, 60) as usize, &tilt)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(v.iter().map(|(t, s)| SampleRecord { at: (*t).into(), scores: s.iter().map(|x| x * 100.0).collect(), tara: 0 }).collect())
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    /// Keys, fingerprints and the expert layer: never shown as sentences.
    const NOT_SHOWN: &[&str] = &["id", "key", "source", "area", "kind", "outcome", "line"];

    fn at(y: i64, mo: u32, d: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: 5, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn check(lang: kshana_rules::Lang, what: &str, field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(lang, v) {
            hits.push(format!("{what}.{field} [{w}]: {v}"));
        }
        for banned in ["guarantee", "always right", "fate", "destiny"] {
            if v.to_lowercase().contains(banned) {
                hits.push(format!("{what}.{field} [promise]: {v}"));
            }
        }
    }

    #[test]
    fn the_trust_loop_speaks_plainly_and_seals_honestly() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let birth = at(1990, 2, 20);
        let (la, lo) = (9.9312, 76.2673);
        let now = at(2026, 10, 21);
        let mut hits = Vec::new();
        for lang in ["en", "ml", "hi"] {
            let l = kshana_rules::Lang::parse(lang);
            let firm = engine.firm_predictions(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, lang.into(), None).unwrap();
            assert!(!firm.is_empty());
            for f in &firm {
                check(l, "firm", "statement", &f.statement, &mut hits);
                check(l, "firm", "question", &f.question, &mut hits);
            }
            // Sealing them: once, and never twice.
            let sealed = seal_new_claims(vec![], firm.clone(), now, 3, lang.into(), lo);
            assert!(!sealed.is_empty() && sealed.len() <= 3);
            for c in &sealed {
                assert!(c.verified && c.id.len() == 64 && c.outcome.is_empty());
                check(l, "claim", "statement", &c.statement, &mut hits);
                check(l, "claim", "question", &c.question, &mut hits);
                let back = claim_from_json(c.line.clone()).unwrap();
                assert_eq!(back.id, c.id);
            }
            let lines: Vec<String> = sealed.iter().map(|c| c.line.clone()).collect();
            let again = seal_new_claims(lines.clone(), firm.clone(), now, 3, lang.into(), lo);
            for a in &again {
                assert!(sealed.iter().all(|s| s.key != a.key));
            }
            // Answered after the window closes.
            let r = resolve_claim(lines[0].clone(), "close".into(), sealed[0].window_end).unwrap();
            assert_eq!(r.outcome, "close");
            assert!(r.verified && r.id == sealed[0].id);
            // A claim from another feed, with the question written for it.
            let feed = ClaimInputRecord {
                source: "consensus".into(),
                area: "Money".into(),
                kind: "opening".into(),
                good: true,
                window_start: at(2026, 11, 14),
                window_end: at(2026, 11, 19),
                probability: 0.7,
                statement: "Money news is likely.".into(),
                question: String::new(),
            };
            let c = seal_claim(feed.clone(), now, lang.into(), lo).unwrap();
            check(l, "feed", "question", &c.question, &mut hits);
            assert!(!c.question.is_empty());
            assert!(seal_claim(feed, at(2026, 11, 15), lang.into(), lo).is_err(), "sealing after the window opens is refused");
            // The scoreboard.
            let items: Vec<ScoreItemInput> = ["yes", "yes", "no", "close", "unsure", "yes"]
                .iter()
                .enumerate()
                .map(|(i, o)| ScoreItemInput { source: ["sealed", "past", "quiz"][i % 3].into(), outcome: (*o).into(), chance: if i % 3 == 2 { 0.25 } else { 0.5 } })
                .collect();
            let b = scoreboard(items, lang.into());
            assert_eq!(b.total, 5);
            assert_eq!(b.right_text, "3½");
            for (f, v) in [("headline", &b.headline), ("chance_line", &b.chance_line), ("note", &b.note), ("share_text", &b.share_text)] {
                check(l, "scoreboard", f, v, &mut hits);
            }
            for x in &b.lines {
                check(l, "scoreboard", "words", &x.words, &mut hits);
            }
            // The tilt, before and after enough answers.
            for n in [0u32, 4, 12] {
                let w = PersonalWeightsInput {
                    answers: n,
                    techniques: vec![
                        TechniqueWeightInput { key: "slow".into(), weight: 0.2, skill: 0.1, log_gain: 0.05 },
                        TechniqueWeightInput { key: "yogini".into(), weight: 0.5, skill: 0.3, log_gain: 0.2 },
                    ],
                };
                let t = forecast_tilt(w, lang.into());
                assert_eq!(t.active, n >= 8);
                check(l, "tilt", "plain", &t.plain, &mut hits);
                for s in &t.shares {
                    check(l, "tilt", "name", &s.name, &mut hits);
                }
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }

    #[test]
    fn default_weights_reproduce_the_forecast_and_learned_ones_move_it() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let birth = at(1990, 2, 20);
        let (la, lo) = (9.9312, 76.2673);
        let now = at(2026, 10, 21);
        let none = PersonalWeightsInput { answers: 0, techniques: vec![] };
        let a = engine.future(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 12, "en".into(), vec![]).unwrap();
        let b = engine.future_personal(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 12, "en".into(), vec![], none.clone()).unwrap();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let s0 = engine.slow_horizon(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 13).unwrap();
        let s1 = engine.slow_horizon_personal(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 13, none).unwrap();
        assert_eq!(format!("{s0:?}"), format!("{s1:?}"));
        let learned = PersonalWeightsInput {
            answers: 20,
            techniques: vec![
                TechniqueWeightInput { key: "slow".into(), weight: 0.1, skill: 0.0, log_gain: 0.0 },
                TechniqueWeightInput { key: "yogini".into(), weight: 0.55, skill: 0.3, log_gain: 0.2 },
            ],
        };
        let s2 = engine.slow_horizon_personal(birth, la, lo, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 13, learned).unwrap();
        assert_ne!(format!("{s0:?}"), format!("{s2:?}"));
        assert_eq!(s2.len(), s0.len());
        for (x, y) in s0.iter().zip(&s2) {
            for (u, v) in x.scores.iter().zip(&y.scores) {
                assert!((0.0..=100.0).contains(v) && (u - v).abs() < 25.0);
            }
        }
    }
}
