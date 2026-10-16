//! Ask: a plain answer to a question about one life, deduced from the birth chart,
//! the running periods and the sky ahead. The horary (Prashna) chart of the moment is
//! a witness, never a gate: a question is always answered.
//!
//! Four sources, each a number in `[0, 1]` with words:
//!
//! | source  | weight | what |
//! |---------|--------|------|
//! | natal   | 0.35 | the birth chart's promise for the life (KP cusp sub-lord net and Parashari assessment, [`crate::forecast::natal_component`]) |
//! | period  | 0.25 | the running Vimshottari funnel: how many of the five levels carry the life's houses, blended with the life score now |
//! | ahead   | 0.25 | the best stretch in the next 180 days of the daily series |
//! | witness | 0.15 | the KP horary verdict for the instant of the question (Yes 1.0, Delayed 0.6, Unclear 0.5, No 0.2) |
//!
//! Verdicts: `Yes` ≥ 0.62, `Yes, with patience` ≥ 0.50, `Not yet` ≥ 0.40, else
//! `Not in this period`. "Is now a good time" questions are answered from the life
//! score at the instant and the best remaining hour of the day instead.

use crate::areas::Area;
use crate::facts::Facts;
use crate::forecast::{
    moment, natal_component, series, story, Life, Moment, Natal, Sample, Shift, ForecastError,
};
use crate::kp::{agreeing_levels, timing};
use crate::prashna::{prashna, prashna_windows, QuestionShape, Verdict};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

/// What is being asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// Will it happen?
    WillItHappen,
    /// When?
    When,
    /// Is now a good time to act on it?
    GoodTimeNow,
    /// Should I take the step in front of me?
    ShouldI,
    /// Will it come back (a person, a thing, a position)?
    WillItReturn,
    /// Will they recover, will it be set right?
    WillItRecover,
}

impl Question {
    /// All, in display order.
    pub const ALL: [Question; 6] = [
        Question::WillItHappen,
        Question::When,
        Question::GoodTimeNow,
        Question::ShouldI,
        Question::WillItReturn,
        Question::WillItRecover,
    ];

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Question::WillItHappen => "will_it_happen",
            Question::When => "when",
            Question::GoodTimeNow => "good_time_now",
            Question::ShouldI => "should_i",
            Question::WillItReturn => "will_it_return",
            Question::WillItRecover => "will_it_recover",
        }
    }

    /// Parse a key.
    pub fn parse(s: &str) -> Option<Question> {
        Question::ALL.iter().copied().find(|q| q.key() == s)
    }

    /// The words on the button.
    pub fn words(self) -> &'static str {
        match self {
            Question::WillItHappen => "Will it happen?",
            Question::When => "When?",
            Question::GoodTimeNow => "Is now a good time?",
            Question::ShouldI => "Should I?",
            Question::WillItReturn => "Will it come back?",
            Question::WillItRecover => "Will it be set right?",
        }
    }

    fn shape(self) -> QuestionShape {
        match self {
            Question::WillItHappen | Question::GoodTimeNow => QuestionShape::WillItHappen,
            Question::When => QuestionShape::When,
            Question::ShouldI => QuestionShape::Choice,
            Question::WillItReturn => QuestionShape::WillItReturn,
            Question::WillItRecover => QuestionShape::WillTheyRecover,
        }
    }
}

/// The answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    /// Which life.
    pub life: Life,
    /// What was asked.
    pub question: Question,
    /// `Yes`, `Yes, with patience`, `Not yet`, `Not in this period`; for "now" questions
    /// `Go ahead`, `Wait for the hour`, `Not today`.
    pub verdict: String,
    /// Combined score in `[0, 1]`.
    pub score: f64,
    /// How much the four sources agree, `[0, 1]`.
    pub confidence: f64,
    /// One sentence.
    pub headline: String,
    /// The window to act in, when there is one.
    pub when_start: Option<Instant>,
    /// End of that window.
    pub when_end: Option<Instant>,
    /// The window in words (without dates; the app adds them).
    pub when_words: String,
    /// The first Moon window from the horary chart's ruling planets, within 30 days.
    pub first_opening_start: Option<Instant>,
    /// Its end.
    pub first_opening_end: Option<Instant>,
    /// Plain reasons, strongest first.
    pub reasons: Vec<String>,
    /// Rule ids behind the reasons.
    pub rule_ids: Vec<&'static str>,
    /// The life score now, 0–1.
    pub now: f64,
    /// Mean of the 180-day series, 0–1.
    pub ahead_mean: f64,
    /// The best stretch's score, 0–1.
    pub ahead_best: f64,
    /// Shifts that matter in the period ahead.
    pub shifts: Vec<Shift>,
}

fn witness_score(v: Verdict) -> f64 {
    match v {
        Verdict::Yes => 1.0,
        Verdict::Delayed => 0.6,
        Verdict::Unclear => 0.5,
        Verdict::No => 0.2,
    }
}

fn agreement(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (1.0 - var.sqrt() * 2.5).clamp(0.3, 0.95)
}

fn life_index(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

/// The best stretch of at least `min_len` samples at or above `floor`, nearest first;
/// falls back to the single best sample.
fn best_window(samples: &[Sample], idx: usize, floor: f64, min_len: usize) -> Option<(Instant, Instant, f64)> {
    let s: Vec<f64> = samples.iter().map(|x| x.scores[idx]).collect();
    let n = s.len();
    let mut i = 0;
    while i < n {
        if s[i] >= floor {
            let mut j = i;
            while j + 1 < n && s[j + 1] >= floor {
                j += 1;
            }
            if j + 1 - i >= min_len {
                let peak = s[i..=j].iter().cloned().fold(0.0_f64, f64::max);
                return Some((samples[i].at, samples[j].at, peak));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    story(Life::ALL[idx], samples).best
}

/// Answer a question about one life at `at`, for a person born as `natal` describes,
/// standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn answer(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    question: Question,
    at: Instant,
) -> Result<Answer, ForecastError> {
    let area: Area = life.area();
    let idx = life_index(life);
    let name = life.name().to_ascii_lowercase();

    // 1. The birth chart.
    let natal_c = natal_component(natal, life);

    // 2. The running periods and the moment.
    let m: Moment = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let now = m.lives[idx].score;
    let funnel = timing(&natal.facts, area, natal.moon_sidereal, natal.birth, at, 5);
    let agreeing = agreeing_levels(&funnel);
    let period = (0.6 * (agreeing as f64 / 5.0) + 0.4 * now).clamp(0.0, 1.0);
    let period_words = {
        let lords: Vec<String> = funnel
            .iter()
            .take(3)
            .map(|l| format!("{:?}", l.period.lord))
            .collect();
        if agreeing == 0 {
            format!(
                "the running periods ({}) do not carry your {name} houses yet",
                lords.join(" › ")
            )
        } else {
            format!(
                "{agreeing} of the 5 running period levels ({}) carry your {name} houses",
                lords.join(" › ")
            )
        }
    };

    // 3. The sky ahead: 180 days, daily.
    let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, 180)?;
    let st = story(life, &samples);
    let ahead_best = st.best.map(|b| b.2).unwrap_or(st.mean);
    let ahead = (0.5 * st.mean + 0.5 * ahead_best).clamp(0.0, 1.0);

    // 4. The witness: the horary chart of the instant, at the place of asking.
    let horary = compute(
        spk,
        eop,
        &ChartInput {
            instant: at,
            observer: *obs,
            system,
            ayanamsa,
            topocentric_moon: false,
        },
    )
    .map_err(|e| ForecastError::Chart(format!("{e:?}")))?;
    let hf = Facts::from_chart(&horary);
    let pv = prashna(&hf, area, question.shape(), at, obs.longitude_deg);
    let witness = witness_score(pv.verdict);
    let witness_words = match pv.verdict {
        Verdict::Yes => format!(
            "the sky at the moment you asked agrees: {:?}, sub-lord of the {} cusp now, signifies your {name} houses",
            pv.promise.sub_lord,
            crate::forecast::ordinal(area.primary)
        ),
        Verdict::Delayed => format!(
            "the sky at the moment you asked says yes but slowly: {:?} is retrograde or in a retrograde star",
            pv.promise.sub_lord
        ),
        Verdict::Unclear => "the sky at the moment you asked is silent on it".to_string(),
        Verdict::No => format!(
            "the sky at the moment you asked holds back: {:?} leans to the negating houses",
            pv.promise.sub_lord
        ),
    };
    let openings = prashna_windows(spk, ayanamsa, &pv.agents, at, 30);
    let first_opening = openings.first().map(|w| (w.start, w.end));

    // Combine.
    let parts = [natal_c.score, period, ahead, witness];
    let score = (0.35 * natal_c.score + 0.25 * period + 0.25 * ahead + 0.15 * witness).clamp(0.0, 1.0);
    let confidence = agreement(&parts);

    let mut reasons = vec![
        natal_c.note.clone(),
        period_words,
        format!(
            "the next six months average {:.0} for {name}, with a best stretch at {:.0}",
            st.mean * 100.0,
            ahead_best * 100.0
        ),
        witness_words,
    ];
    let mut rule_ids = vec!["KS-N-0001", "KS-F-0001", "KS-G-0001", "KS-P-0001"];
    if let Some(h) = pv.hits.iter().find(|h| h.rule_id == "KS-P-0012") {
        reasons.push(h.factors.join("; "));
        rule_ids.push("KS-P-0012");
    }

    let (verdict, headline, when, when_words) = match question {
        Question::GoodTimeNow => {
            let band = m.lives[idx].band;
            let hour_fit = m
                .hours
                .iter()
                .filter(|h| h.end > at && !h.caution)
                .map(|h| (h.start, h.end, h.fit[idx]))
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let in_caution = m.hours.iter().any(|h| h.start <= at && at < h.end && h.caution);
            if now >= 0.56 && !in_caution {
                (
                    "Go ahead".to_string(),
                    format!("{} {}", m.lives[idx].headline, "This hour serves it."),
                    Some((at, m.hora.as_ref().map(|h| h.end).unwrap_or(at))),
                    "now, until the hour turns".to_string(),
                )
            } else if let Some((s, e, f)) = hour_fit.filter(|h| h.2 >= 0.5 && h.0 > at) {
                (
                    "Wait for the hour".to_string(),
                    format!(
                        "Not this minute. A better hour comes later today{}.",
                        if band >= crate::forecast::Band::Steady { "" } else { ", and the day itself is slow" }
                    ),
                    Some((s, e)),
                    format!("the best hour left today, fit {:.0}", f * 100.0),
                )
            } else {
                (
                    "Not today".to_string(),
                    format!("Today does not serve {name}. Let it wait for a better day."),
                    st.best.map(|b| (b.0, b.1)),
                    "the next good stretch".to_string(),
                )
            }
        }
        _ => {
            let floor = if score >= 0.62 { 0.56 } else { 0.50 };
            let w = best_window(&samples, idx, floor, 2).or(st.best);
            let (verdict, headline) = if score >= 0.62 {
                (
                    match question {
                        Question::ShouldI => "Take it",
                        Question::WillItReturn => "Yes, it returns",
                        Question::WillItRecover => "Yes, it mends",
                        _ => "Yes",
                    }
                    .to_string(),
                    match question {
                        Question::When => format!("It comes. Watch the window below; that is when {name} opens."),
                        Question::ShouldI => "Take the step. The chart, the period and the sky ahead agree.".to_string(),
                        Question::WillItReturn => "It comes back. Keep the door open and do not chase.".to_string(),
                        Question::WillItRecover => "It mends. Steady care through the window below.".to_string(),
                        _ => format!("Yes. {} is promised and the period carries it.", life.name()),
                    },
                )
            } else if score >= 0.50 {
                (
                    match question {
                        Question::ShouldI => "Take it, slowly",
                        _ => "Yes, with patience",
                    }
                    .to_string(),
                    format!("Yes, but not quickly. {} needs the window below and your effort inside it.", life.name()),
                )
            } else if score >= 0.40 {
                (
                    "Not yet".to_string(),
                    format!("Not yet. The period does not carry {name} well; the nearest opening is below."),
                )
            } else {
                (
                    "Not in this period".to_string(),
                    format!("Not in this period. {} asks for remedy and time before it can be asked again.", life.name()),
                )
            };
            let words = if score >= 0.50 { "the window to act in" } else { "the nearest opening" };
            (verdict, headline, w.map(|x| (x.0, x.1)), words.to_string())
        }
    };

    Ok(Answer {
        life,
        question,
        verdict,
        score,
        confidence,
        headline,
        when_start: when.map(|w| w.0),
        when_end: when.map(|w| w.1),
        when_words,
        first_opening_start: first_opening.map(|w| w.0),
        first_opening_end: first_opening.map(|w| w.1),
        reasons,
        rule_ids,
        now,
        ahead_mean: st.mean,
        ahead_best,
        shifts: st.shifts.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn every_question_about_every_life_is_answered() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: birth,
                observer: obs,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let natal = Natal::new(Facts::from_chart(&ch), birth);
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for life in [Life::Love, Life::Career, Life::Money] {
            for q in Question::ALL {
                let a = answer(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, life, q, at).unwrap();
                assert!(!a.verdict.is_empty() && !a.headline.is_empty());
                assert!((0.0..=1.0).contains(&a.score));
                assert!((0.3..=0.95).contains(&a.confidence));
                assert!(a.reasons.len() >= 4);
                if let (Some(s), Some(e)) = (a.when_start, a.when_end) {
                    assert!(s <= e);
                }
            }
        }
    }
}
