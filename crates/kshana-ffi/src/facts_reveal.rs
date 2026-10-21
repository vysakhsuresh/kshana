//! Things nobody told the app, over the FFI: checkable facts read from the birth chart
//! (birth order, brothers and sisters, the mother's and father's nature, a mark on the
//! body and its side, the years of moves, marriage and job changes), each a yes-or-no
//! question; and the birth time sharpened continuously by every answer: the answered
//! stretches of the past ([`Engine::past_reveal`]), the answered facts, and every dated
//! event the person logged (the diary's life events, the ledger).
//!
//! Nothing here changes the birth data: [`Engine::birth_time_now`] suggests, the person
//! taps to accept. A wrong guess still helps, because it narrows down the birth time.

use kshana_rules::facts_reveal::{self as fr, Claim, FactAnswer, Logged};
use kshana_sky::Observer;

use crate::{learn, to_house_system, AyanamsaChoice, BirthFitRecord, BirthRefineRecord, Engine, ExactInstant, HouseSystem, KshanaError, PastAnswerInput};

/// One fact read from the chart, to be answered Yes or No.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChartFactRecord {
    /// Stable key of the claim, stored with the answer and passed back in
    /// [`FactAnswerInput::key`] (never shown).
    pub key: String,
    /// What it is about (never shown): "birth_order", "siblings", "mother", "father",
    /// "mark", "move", "marriage" or "job".
    pub topic: String,
    /// The question in plain words: "Your chart says … Is that right?"
    pub statement: String,
    /// How strongly the chart says it, 0–1; only clear ones are returned.
    pub strength: f64,
    /// True when the answer tells nearby birth times apart.
    pub sensitive: bool,
    /// The chart reason in an astrologer's words (expert view only).
    pub why_expert: String,
    /// The text it rests on (expert view only).
    pub source: String,
}

/// One answered fact.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FactAnswerInput {
    /// [`ChartFactRecord::key`].
    pub key: String,
    /// "yes" or "no"; anything else (not sure) is left out.
    pub verdict: String,
}

/// One dated event the person logged.
#[derive(Clone, Debug, uniffi::Record)]
pub struct LoggedEventInput {
    /// When it happened.
    pub at: ExactInstant,
    /// A life key ("Love", "Career", …) or a ledger area ("Marriage", "Wealth",
    /// "Property", "Education", "Foreign", …); unknown names are skipped.
    pub life: String,
    /// True for a good event (a start, a gain, a milestone), false for a hard one (a
    /// loss, a crisis).
    pub good: bool,
}

/// Where every answer so far places the birth time.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BirthNowRecord {
    /// The most likely birth instant.
    pub best: ExactInstant,
    /// Minutes from the recorded time (positive = later).
    pub best_shift_min: f64,
    /// Start of the stretch holding `hpd_mass` of the belief (a highest-density set's
    /// hull, as cell edges).
    pub hpd_from: Option<ExactInstant>,
    /// Its end.
    pub hpd_to: Option<ExactInstant>,
    /// Share of the belief inside it.
    pub hpd_mass: f64,
    /// Separate runs inside it (1 = one stretch).
    pub hpd_runs: u32,
    /// Step between the candidate times, minutes.
    pub step_min: f64,
    /// log10 of the evidence against answers given at random.
    pub log10_bayes: f64,
    /// The same as a ratio ("20 to 1" is 20).
    pub evidence_ratio: f64,
    /// True when the evidence is strong enough (20 to 1) to place the birth.
    pub strong: bool,
    /// Answered stretches used.
    pub stretches: u32,
    /// Answered facts used (those that change across the window).
    pub facts: u32,
    /// Answered facts every time in the window agrees on (counted, not scored).
    pub facts_flat: u32,
    /// Logged events used.
    pub logged: u32,
    /// Everything the person has answered or logged.
    pub answers: u32,
    /// One plain sentence: "Your 31 answers so far place your birth between 10:33 and
    /// 10:36."
    pub plain: String,
    /// The full refinement, as [`Engine::refine_birth_time`] gives it, over all the
    /// evidence; its `suggested` is the time to offer (the person taps to accept).
    pub refine: BirthRefineRecord,
}

fn fit_record(f: &kshana_rules::BirthFit) -> BirthFitRecord {
    BirthFitRecord {
        birth: f.birth.into(),
        shift_min: f.shift_min,
        prob: f.prob,
        explained: f.explained as u32,
        lagna_sign: f.lagna_sign,
        moon_star: f.moon_star as u32,
        by_witness: f.by_witness.to_vec(),
    }
}

/// The answered facts as the engine reads them; unknown keys and "not sure" are skipped.
pub(crate) fn facts_of(answers: &[FactAnswerInput]) -> Vec<FactAnswer> {
    answers
        .iter()
        .filter_map(|a| {
            let claim = Claim::parse(&a.key)?;
            let yes = match a.verdict.as_str() {
                "yes" => true,
                "no" => false,
                _ => return None,
            };
            Some(FactAnswer { claim, yes })
        })
        .collect()
}

pub(crate) fn logged_of(events: &[LoggedEventInput]) -> Vec<Logged> {
    events.iter().filter_map(|e| Some(Logged { at: e.at.into(), life: fr::life_of(&e.life)?, good: e.good })).collect()
}

#[uniffi::export]
impl Engine {
    /// The facts the chart says clearly, as yes-or-no questions, skipping `asked_keys`;
    /// at most one per topic. `window_min` is how far the birth time may be off: each
    /// fact is marked `sensitive` when it changes across it.
    #[allow(clippy::too_many_arguments)]
    pub fn chart_facts(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        lang: String,
        window_min: f64,
        asked_keys: Vec<String>,
    ) -> Result<Vec<ChartFactRecord>, KshanaError> {
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let fs = fr::chart_facts(&self.spk, self.eop.as_ref(), &natal, &obs, to_house_system(system), ayanamsa.into(), kshana_rules::Lang::parse(&lang), at.into(), window_min, &asked_keys);
        Ok(fs
            .into_iter()
            .map(|f| ChartFactRecord {
                key: f.claim.key(),
                topic: f.claim.topic().key().to_string(),
                statement: f.statement,
                strength: f.strength,
                sensitive: f.sensitive,
                why_expert: f.why_expert,
                source: f.source.to_string(),
            })
            .collect())
    }

    /// Where every answer so far places the birth time, to the minute: the answered
    /// stretches, the answered facts and the logged events pooled per candidate minute
    /// within ±`window_min`. The best time, the stretch of times holding 80 % of the
    /// belief as exact instants, the evidence, and one plain sentence with the times read
    /// at `utc_offset_minutes` (the birth clock's offset). It never changes the birth
    /// data: `refine.suggested` is offered, the person taps to accept.
    #[allow(clippy::too_many_arguments)]
    pub fn birth_time_now(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        window_min: f64,
        answers: Vec<PastAnswerInput>,
        facts: Vec<FactAnswerInput>,
        logged: Vec<LoggedEventInput>,
        utc_offset_minutes: i32,
        lang: String,
    ) -> BirthNowRecord {
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let ans = learn::answers_of(&answers);
        let fa = facts_of(&facts);
        let lg = logged_of(&logged);
        let r = fr::refine_birth_pooled(&self.spk, self.eop.as_ref(), &obs, to_house_system(system), ayanamsa.into(), birth.into(), window_min, 1.0, &ans, &fa, &lg, at.into(), true);
        let plain = fr::birth_now_words(kshana_rules::Lang::parse(&lang), &r, birth.into(), utc_offset_minutes.clamp(-14 * 60, 14 * 60));
        let fits = &r.refine.fits;
        let settled = fits.first().map(|b| fits.iter().filter(|f| (f.shift_min - b.shift_min).abs() <= 5.0).map(|f| f.prob).sum()).unwrap_or(0.0);
        let recorded_explains = fits.iter().find(|f| f.shift_min.abs() < 1e-9).map(|f| f.explained as u32).unwrap_or(0);
        let refine = BirthRefineRecord {
            suggested: r.suggested.as_ref().map(fit_record),
            fits: fits.iter().take(7).map(fit_record).collect(),
            settled,
            recorded_explains,
            log10_bayes: r.refine.log10_bayes,
            boundaries: r.refine.boundaries.iter().map(learn::boundary_record).collect(),
            bracket_from: r.refine.bracket.map(|b| b.0.into()),
            bracket_to: r.refine.bracket.map(|b| b.1.into()),
            d60_in_bracket: r.refine.d60_in_bracket as u32,
            witness_keys: kshana_rules::witness::WITNESSES.iter().map(|w| w.key().to_string()).collect(),
        };
        let best = fits.first();
        BirthNowRecord {
            best: best.map(|f| f.birth.into()).unwrap_or(birth),
            best_shift_min: best.map(|f| f.shift_min).unwrap_or(0.0),
            hpd_from: r.hpd.map(|h| h.0.into()),
            hpd_to: r.hpd.map(|h| h.1.into()),
            hpd_mass: fr::HPD_MASS,
            hpd_runs: r.hpd_runs as u32,
            step_min: r.step_min,
            log10_bayes: r.refine.log10_bayes,
            evidence_ratio: 10f64.powf(r.refine.log10_bayes.clamp(-300.0, 300.0)),
            strong: r.refine.log10_bayes >= kshana_rules::reveal::SUGGEST_LOG10_BAYES,
            stretches: r.stretches as u32,
            facts: r.facts as u32,
            facts_flat: r.facts_flat as u32,
            logged: r.logged as u32,
            answers: (r.stretches + r.facts + r.facts_flat + r.logged) as u32,
            plain,
            refine,
        }
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    /// Fields never shown on the common screens: keys, the topic, the chart reason and
    /// its source.
    const NOT_SHOWN: &[&str] = &["key", "topic", "why_expert", "source"];

    fn at(y: i64, mo: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn check(lang: &str, field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        let l = kshana_rules::Lang::parse(lang);
        if let Some(w) = kshana_rules::plain::jargon_in(l, v) {
            hits.push(format!("{lang} {field} [{w}]: {v}"));
        }
        for bad in ["guaranteed", "certain", "certainly", "fate", "destiny"] {
            if v.split(|c: char| !c.is_alphanumeric()).any(|x| x.eq_ignore_ascii_case(bad)) {
                hits.push(format!("{lang} {field} [{bad}]: {v}"));
            }
        }
    }

    #[test]
    fn facts_and_the_birth_time_name_no_astrology() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let (la, lo) = (9.9312, 76.2673);
        let now = at(2026, 10, 20, 5);
        let mut hits = Vec::new();
        for (y, mo, d, h) in [(1990, 2, 20, 5), (1985, 7, 3, 12), (1978, 11, 30, 22)] {
            let birth = at(y, mo, d, h);
            for lang in ["en", "ml", "hi"] {
                let fs = engine.chart_facts(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, lang.into(), 30.0, vec![]).unwrap();
                assert!(!fs.is_empty());
                for f in &fs {
                    check(lang, "statement", &f.statement, &mut hits);
                    check(lang, "why_expert", &f.why_expert, &mut hits);
                    assert!(Claim::parse(&f.key).is_some(), "{}", f.key);
                }
                // Answer every fact No (a wrong guess still helps) and a few stretches.
                let qs = engine.past_reveal(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, lang.into(), 30.0, vec![], vec![], 4, false).unwrap();
                let answers: Vec<PastAnswerInput> = qs.iter().map(|q| PastAnswerInput { from: q.from, to: q.to, life: q.life.clone(), good: q.good, verdict: "yes".into() }).collect();
                let facts: Vec<FactAnswerInput> = fs.iter().map(|f| FactAnswerInput { key: f.key.clone(), verdict: "no".into() }).collect();
                let logged = vec![LoggedEventInput { at: at(2015, 6, 1, 0), life: "Marriage".into(), good: true }, LoggedEventInput { at: at(2019, 3, 1, 0), life: "nonsense".into(), good: false }];
                let r = engine.birth_time_now(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 30.0, answers.clone(), facts.clone(), logged, 330, lang.into());
                eprintln!("{lang}: {}", r.plain);
                check(lang, "plain", &r.plain, &mut hits);
                assert_eq!(r.logged, 1, "the unknown area is skipped");
                assert_eq!(r.stretches as usize, answers.len());
                assert_eq!((r.facts + r.facts_flat) as usize, facts.len());
                assert_eq!(r.answers as usize, answers.len() + facts.len() + 1);
                assert!(r.hpd_from.is_some() && r.hpd_to.is_some());
                let (a, b) = (r.hpd_from.unwrap(), r.hpd_to.unwrap());
                assert!(a.tai_seconds <= r.best.tai_seconds && r.best.tai_seconds <= b.tai_seconds);
                assert!(r.evidence_ratio > 0.0 && r.log10_bayes.is_finite());
                assert!(!r.refine.fits.is_empty());
            }
        }
        // Nothing answered: still a sentence, no bracket.
        let r = engine.birth_time_now(at(1990, 2, 20, 5), la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, now, 30.0, vec![], vec![], vec![], 330, "en".into());
        assert_eq!(r.answers, 0);
        assert!(r.hpd_from.is_none() && r.refine.fits.is_empty());
        check("en", "plain", &r.plain, &mut hits);
        assert!(hits.is_empty(), "astrology in plain fields:\n{}", hits.join("\n"));
    }
}
