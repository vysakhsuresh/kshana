//! Learning from the person's own past over the FFI: which technique has fit their
//! answered stretches best, with weights the forecast can use; and the birth-time
//! boundaries record of the rectification.
//!
//! The answers are the stretches the person marked Yes, Close or No: the past told
//! first ([`Engine::past_reveal`]) and every sealed stretch ([`Engine::seal_pick`]) once
//! it has been opened and answered, passed the same way as [`PastAnswerInput`].

use kshana_sky::Observer;

use crate::{to_house_system, AyanamsaChoice, Engine, ExactInstant, HouseSystem, PastAnswerInput};

/// The answers as the engine reads them; unknown lives or verdicts are skipped.
pub(crate) fn answers_of(answers: &[PastAnswerInput]) -> Vec<kshana_rules::PastAnswer> {
    answers
        .iter()
        .filter_map(|a| {
            let life = kshana_rules::Life::parse(&a.life)?;
            let verdict = match a.verdict.as_str() {
                "yes" => kshana_rules::PastVerdict::Yes,
                "close" => kshana_rules::PastVerdict::Close,
                "no" => kshana_rules::PastVerdict::No,
                _ => return None,
            };
            Some(kshana_rules::PastAnswer { from: a.from.into(), to: a.to.into(), life, good: a.good, verdict })
        })
        .collect()
}

/// A birth time inside the window where something in the chart changes, so the app can
/// say "your time sits between 10:31 and 10:39".
#[derive(Clone, Debug, uniffi::Record)]
pub struct BirthBoundaryRecord {
    /// The moment, to the second.
    pub at: ExactInstant,
    /// Minutes from the recorded time.
    pub shift_min: f64,
    /// What changes (expert view only): "rising_sign", "moon_pada", "d9_rising" or
    /// "kp_rising_sub".
    pub kind: String,
    /// Its value just before: a sign 0 = Aries for the rising signs, the quarter-star
    /// 0–107 (star × 4 + quarter) for the Moon, a graha 0 = Sun … 8 = Ketu for the sub.
    pub before: u32,
    /// Its value just after.
    pub after: u32,
}

pub(crate) fn boundary_record(b: &kshana_rules::witness::Boundary) -> BirthBoundaryRecord {
    BirthBoundaryRecord { at: b.at.into(), shift_min: b.shift_min, kind: b.kind.key().to_string(), before: b.before, after: b.after }
}

/// How one technique would have done on the person's answers.
#[derive(Clone, Debug, uniffi::Record)]
pub struct TechniqueScoreRecord {
    /// Stable key: "slow", "vimshottari", "yogini", "chara", "kp" or
    /// "slow_other_ayanamsa".
    pub key: String,
    /// The technique in an astrologer's words (expert view only).
    pub technique_expert: String,
    /// The technique in plain words, in the reader's language.
    pub name: String,
    /// Answers on which it had a reading.
    pub read: u32,
    /// Brier score, 0 best.
    pub brier: f64,
    /// Mean log score per answer over chance, nats; 0 is chance.
    pub log_gain: f64,
    /// Brier skill against chance; above 0 beats chance.
    pub skill: f64,
    /// Weight for the forecast, all techniques summing to 1; equal with no answers.
    pub weight: f64,
}

/// Which technique fits the person's past.
#[derive(Clone, Debug, uniffi::Record)]
pub struct TechniqueLearnRecord {
    /// Every technique.
    pub techniques: Vec<TechniqueScoreRecord>,
    /// Answers used.
    pub answers: u32,
    /// The chance forecaster's Brier score on these answers, the line to beat.
    pub chance_brier: f64,
    /// Key of the technique that fits best, once there are enough answers and it
    /// clearly leads (expert view only).
    pub best_key: Option<String>,
    /// One plain sentence: "Your past fits the long-period reading best so far; 9
    /// answers."
    pub plain: String,
}

#[uniffi::export]
impl Engine {
    /// Which technique (the whole-picture slow score, Vimshottari, Yogini or Chara
    /// periods, KP timing, the slow score on the other ayanamsa) would have predicted
    /// the person's answered past best: Brier and log scores against chance, and
    /// weights shrunk toward equal while the answers are few.
    #[allow(clippy::too_many_arguments)]
    pub fn learn_techniques(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        answers: Vec<PastAnswerInput>,
        lang: String,
    ) -> TechniqueLearnRecord {
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let ans = answers_of(&answers);
        let lang = kshana_rules::Lang::parse(&lang);
        let ay: kshana_vedic::Ayanamsa = ayanamsa.into();
        let l = kshana_rules::learn::learn(&self.spk, self.eop.as_ref(), &obs, to_house_system(system), ay, birth.into(), &ans, lang);
        TechniqueLearnRecord {
            techniques: l
                .techniques
                .iter()
                .map(|t| TechniqueScoreRecord {
                    key: t.technique.key().to_string(),
                    technique_expert: t.technique.expert(ay),
                    name: t.technique.name(lang).to_string(),
                    read: t.read as u32,
                    brier: t.brier,
                    log_gain: t.log_gain,
                    skill: t.skill,
                    weight: t.weight,
                })
                .collect(),
            answers: l.answers as u32,
            chance_brier: l.chance_brier,
            best_key: l.best.map(|t| t.key().to_string()),
            plain: l.plain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    fn at(y: i64, mo: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    #[test]
    fn the_person_reads_no_technique_names() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let birth = at(1990, 2, 20, 5);
        let (la, lo) = (9.9312, 76.2673);
        let qs = engine.past_reveal(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, at(2026, 10, 20, 5), "en".into(), 30.0, vec![], vec![], 6, false).unwrap();
        let answers: Vec<PastAnswerInput> = qs
            .iter()
            .enumerate()
            .map(|(i, q)| PastAnswerInput { from: q.from, to: q.to, life: q.life.clone(), good: q.good, verdict: ["yes", "no", "close"][i % 3].into() })
            .collect();
        for lang in ["en", "ml", "hi"] {
            let r = engine.learn_techniques(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, answers.clone(), lang.into());
            let l = kshana_rules::Lang::parse(lang);
            eprintln!("{}", r.plain);
            assert_eq!(kshana_rules::plain::jargon_in(l, &r.plain), None, "{}", r.plain);
            for t in &r.techniques {
                assert_eq!(kshana_rules::plain::jargon_in(l, &t.name), None, "{}", t.name);
            }
            assert_eq!(r.answers as usize, answers.len());
            assert_eq!(r.techniques.len(), 6);
            assert!((r.techniques.iter().map(|t| t.weight).sum::<f64>() - 1.0).abs() < 1e-9);
        }
        // The refined birth time reports its boundaries and evidence.
        let rr = engine.refine_birth_time(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, 30.0, answers.clone());
        assert!(!rr.boundaries.is_empty() && rr.bracket_from.is_some() && rr.bracket_to.is_some());
        assert!(rr.log10_bayes.is_finite());
        assert_eq!(rr.witness_keys.len(), rr.fits[0].by_witness.len());
    }
}
