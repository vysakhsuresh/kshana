//! Ask: a plain answer to a question about one life, deduced from the birth chart,
//! the running periods and the sky ahead. The horary (Prashna) chart of the moment is
//! a witness, never a gate: a question is always answered.
//!
//! Each question is its own computation, because each asks something different:
//!
//! | question | what is weighed |
//! |---|---|
//! | Will it happen? | the promise: natal 0.35, running periods 0.30, the best of the next 180 days 0.20, the horary witness 0.15 |
//! | When? | timing: the nearest daily window at or above good, the next period level whose lord carries the houses, the first Moon window of the ruling planets; the score is how soon |
//! | Is now a good time? | the life score this second, the caution windows, the best hour left today |
//! | Should I? | the next seven days against the best stretch of the next 180, on the ground of the natal promise |
//! | Will it come back? | the slow line twelve months back against twenty-four ahead: a real dip, and whether the line climbs back to where it was |
//! | Will it be set right? | the first lasting lift (Explain) and the first month the slow line climbs a band |

use crate::areas::Area;
use crate::explain::explain;
use crate::facts::Facts;
use crate::forecast::{
    date_words, moment, natal_component_for, series, slow_series, story, Band, ForecastError, Life, Moment,
    Natal, Sample, Shift,
};
use crate::kp::{agreeing_levels, period_supports, timing};
use crate::prashna::{prashna, prashna_windows, QuestionShape, Verdict};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::dasha::{subdivide, vimshottari, YEAR_365_25_NS};
use kshana_vedic::{Ayanamsa, DashaPeriod};

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
    /// The verdict in a few words.
    pub verdict: String,
    /// Combined score in `[0, 1]`.
    pub score: f64,
    /// How much the sources agree, `[0, 1]`.
    pub confidence: f64,
    /// One to three sentences, with dates.
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

/// The first stretch of at least `min_len` samples at or above `floor`, nearest first;
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

/// The next period (antardasha or pratyantar) that begins after `at` within two years
/// and whose lord carries the area's houses: `(period, level name)`.
fn next_carrying_period(natal: &Natal, area: Area, at: Instant) -> Option<(DashaPeriod, &'static str)> {
    let horizon = at.add_ns(730 * NS_PER_DAY);
    let mut best: Option<(DashaPeriod, &'static str)> = None;
    let mut consider = |p: DashaPeriod, level: &'static str| {
        if p.start > at && p.start <= horizon && !period_supports(&natal.facts, area, p.lord).is_empty() {
            if best.as_ref().map_or(true, |b| p.start < b.0.start) {
                best = Some((p, level));
            }
        }
    };
    for maha in vimshottari(natal.moon_sidereal, natal.birth, YEAR_365_25_NS) {
        if maha.end <= at || maha.start > horizon {
            continue;
        }
        for antar in subdivide(maha) {
            if antar.end <= at || antar.start > horizon {
                continue;
            }
            consider(antar, "antardasha");
            if antar.start <= at {
                for pr in subdivide(antar) {
                    consider(pr, "pratyantar");
                }
            }
        }
    }
    best
}

fn months_between(a: Instant, b: Instant) -> f64 {
    b.ns_since(a) as f64 / (30.4375 * NS_PER_DAY as f64)
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
    answer_with(spk, eop, natal, obs, system, ayanamsa, life, life.area(), &life.name().to_ascii_lowercase(), question, at)
}

/// Answer a question about a topic of the dictionary: the topic's houses carry the
/// promise and the timing, the life's daily sky carries the rest.
#[allow(clippy::too_many_arguments)]
pub fn answer_topic(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    topic: &crate::topics::Topic,
    question: Question,
    at: Instant,
) -> Result<Answer, ForecastError> {
    let label = topic.words.to_ascii_lowercase();
    answer_with(spk, eop, natal, obs, system, ayanamsa, topic.life, topic.area, &label, question, at)
}

#[allow(clippy::too_many_arguments)]
fn answer_with(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    area: Area,
    label: &str,
    question: Question,
    at: Instant,
) -> Result<Answer, ForecastError> {
    let idx = life_index(life);
    let name = label.to_string();
    let title = {
        let mut c = name.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    };
    let lon = obs.longitude_deg;

    // The common ground: the birth chart, the moment, the periods, the sky ahead and
    // the witness. Every question reads these differently.
    let natal_c = natal_component_for(natal, area, &name);
    let m: Moment = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let now = m.lives[idx].score;
    let funnel = timing(&natal.facts, area, natal.moon_sidereal, natal.birth, at, 5);
    let agreeing = agreeing_levels(&funnel);
    let period = (0.6 * (agreeing as f64 / 5.0) + 0.4 * now).clamp(0.0, 1.0);
    let period_words = match agreeing {
        0 => format!("the phase of life you are in does not favour {name} yet"),
        1 | 2 => format!("the phase of life you are in partly favours {name}"),
        _ => format!("the phase of life you are in strongly favours {name}"),
    };
    let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, 180)?;
    let st = story(life, &samples);
    let ahead_best = st.best.map(|b| b.2).unwrap_or(st.mean);
    let ahead = (0.5 * st.mean + 0.5 * ahead_best).clamp(0.0, 1.0);

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
        Verdict::Yes => format!("the very moment you asked also points to yes for {name}"),
        Verdict::Delayed => format!("the very moment you asked says yes for {name}, but late"),
        Verdict::Unclear => "the very moment you asked gives no clear sign".to_string(),
        Verdict::No => format!("the very moment you asked holds back on {name}"),
    };
    let openings = prashna_windows(spk, ayanamsa, &pv.agents, at, 30);
    let first_opening = openings.first().map(|w| (w.start, w.end));

    let mut reasons: Vec<String> = Vec::new();
    let mut rule_ids: Vec<&'static str> = Vec::new();
    let push_common = |reasons: &mut Vec<String>, rule_ids: &mut Vec<&'static str>| {
        reasons.push(crate::plain::birth_base(&name, natal_c.score));
        rule_ids.push("KS-N-0001");
        reasons.push(period_words.clone());
        rule_ids.push("KS-F-0001");
        reasons.push(format!(
            "the next six months average {:.0} for {name}, with a best stretch at {:.0}",
            st.mean * 100.0,
            ahead_best * 100.0
        ));
        rule_ids.push("KS-G-0001");
        reasons.push(witness_words.clone());
        rule_ids.push("KS-P-0001");
    };

    let (verdict, score, confidence, headline, when, when_words): (String, f64, f64, String, Option<(Instant, Instant)>, String) = match question {
        // ---------------------------------------------------------------- will it happen
        Question::WillItHappen => {
            push_common(&mut reasons, &mut rule_ids);
            let parts = [natal_c.score, period, ahead, witness];
            let score = (0.35 * natal_c.score + 0.30 * period + 0.20 * ahead + 0.15 * witness).clamp(0.0, 1.0);
            let floor = if score >= 0.62 { 0.56 } else { 0.50 };
            let w = best_window(&samples, idx, floor, 2).or(st.best);
            let (v, h) = if score >= 0.62 {
                ("Yes", format!("Yes. {} is written in your birth chart and the phase you are in brings it forward. The dates below are when it shows first.", title))
            } else if score >= 0.50 {
                ("Yes, with patience", format!("Yes, but not quickly. {} comes through the dates below and your effort inside them; the phase you are in helps only in part.", title))
            } else if score >= 0.40 {
                ("Not yet", format!("Not yet. The phase you are in does not favour {name}. The nearest opening is below; until then the remedies do the work."))
            } else {
                ("Not in this phase", format!("Not in this phase of life. {} needs remedies and time before it is worth asking again.", title))
            };
            (v.to_string(), score, agreement(&parts), h, w.map(|x| (x.0, x.1)), if score >= 0.50 { "the window to act in" } else { "the nearest opening" }.to_string())
        }

        // ------------------------------------------------------------------------- when
        Question::When => {
            push_common(&mut reasons, &mut rule_ids);
            let daily = best_window(&samples, idx, 0.56, 3);
            let period_next = next_carrying_period(natal, area, at);
            let slow = slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(30 * NS_PER_DAY), (30.4375 * NS_PER_DAY as f64) as i128, 24)?;
            let slow_now_band = Band::of(m.lives[idx].score);
            let climb = slow.iter().find(|(_, v)| Band::of(v[idx]) > slow_now_band).map(|(t, v)| (*t, v[idx]));
            if let Some((p, level)) = &period_next {
                reasons.push(format!(
                    "a {} that favours {name} begins on {}",
                    if *level == "antardasha" { "stretch of a year or more" } else { "season of a few months" },
                    date_words(p.start, lon)
                ));
                rule_ids.push("KS-F-0001");
            }
            // The answer is the soonest of a sound daily window and a carrying period.
            let soonest: Option<(Instant, Instant, &str, f64)> = match (daily, period_next) {
                (Some((s, e, pk)), Some((p, _))) => {
                    if s <= p.start {
                        Some((s, e, "daily", pk))
                    } else {
                        Some((p.start, p.end.min(p.start.add_ns(90 * NS_PER_DAY)), "period", 0.6))
                    }
                }
                (Some((s, e, pk)), None) => Some((s, e, "daily", pk)),
                (None, Some((p, _))) => Some((p.start, p.end.min(p.start.add_ns(90 * NS_PER_DAY)), "period", 0.6)),
                (None, None) => None,
            };
            match soonest {
                Some((s, e, kind, pk)) => {
                    let months = months_between(at, s).max(0.0);
                    let score = if months < 1.0 { 0.85 } else if months < 3.0 { 0.7 } else if months < 6.0 { 0.55 } else if months < 12.0 { 0.42 } else { 0.32 };
                    let v = if months < 1.0 { "Within the month" } else if months < 3.0 { "Within three months" } else if months < 6.0 { "Within six months" } else if months < 12.0 { "Within the year" } else { "Beyond a year" };
                    let mut h = if kind == "daily" {
                        format!("{} opens {} to {}, your best stretch for it at {:.0} out of 100. That is the first time to act.", title, date_words(s, lon), date_words(e, lon), pk * 100.0)
                    } else {
                        format!("The first stretch of life that truly favours {name} begins {}. Before it no strong days come; use the time to prepare.", date_words(s, lon))
                    };
                    if let Some((p, _level)) = next_carrying_period(natal, area, at) {
                        if kind == "daily" {
                            h.push_str(&format!(" A longer supportive stretch begins {} and holds it in place.", date_words(p.start, lon)));
                        }
                    }
                    if let Some((t, v2)) = climb {
                        h.push_str(&format!(" The season itself climbs to {} from {}.", Band::of(v2).word(), date_words(t, lon)));
                    }
                    (v.to_string(), score, agreement(&[natal_c.score, period, ahead]).max(0.5), h, Some((s, e)), "the first opening".to_string())
                }
                None => {
                    let h = match climb {
                        Some((t, v2)) => format!("No strong stretch within six months, and no new phase that favours {name} within two years. Things turn {} from {}; until then the remedies do the work.", Band::of(v2).word().to_ascii_lowercase(), date_words(t, lon)),
                        None => format!("Not within two years: no phase that favours {name} begins, and things hold where they are. Lean on the remedies, and ask again after them."),
                    };
                    ("Not within two years".to_string(), 0.25, 0.6, h, climb.map(|(t, _)| (t, t.add_ns(30 * NS_PER_DAY))), "the season's turn".to_string())
                }
            }
        }

        // ------------------------------------------------------------- is now a good time
        Question::GoodTimeNow => {
            if let Some(h) = &m.hora {
                reasons.push(format!("this hour is {}", crate::plain::hour_nature(h.lord)));
            } else {
                reasons.push(m.lives[idx].because.clone());
            }
            rule_ids.push("KS-H-0001");
            reasons.push(format!("{name} reads {:.0} this second ({})", now * 100.0, m.lives[idx].band.word()));
            rule_ids.push("KS-M-0001");
            push_common(&mut reasons, &mut rule_ids);
            let band = m.lives[idx].band;
            let hour_fit = m
                .hours
                .iter()
                .filter(|h| h.end > at && !h.caution)
                .map(|h| (h.start, h.end, h.fit[idx]))
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let in_caution = m.windows.iter().find(|w| w.caution && w.start <= at && at < w.end).map(|w| w.label.clone());
            if let Some(c) = in_caution {
                let end = m.windows.iter().find(|w| w.label == c).map(|w| w.end).unwrap_or(at);
                ("Not this minute".to_string(), now.min(0.4), 0.8, "Not this minute: an unlucky stretch of the day is running. Finish things, do not begin them; it closes shortly and the better hours after it are below.".to_string(), Some((end, m.hora.as_ref().map(|h| h.end.max(end)).unwrap_or(end))), "after the caution ends".to_string())
            } else if now >= 0.56 {
                ("Go ahead".to_string(), now, 0.85, format!("{} This hour serves it.", m.lives[idx].headline), Some((at, m.hora.as_ref().map(|h| h.end).unwrap_or(at))), "now, until the hour turns".to_string())
            } else if let Some((s, e, f)) = hour_fit.filter(|h| h.2 >= 0.5 && h.0 > at) {
                ("Wait for the hour".to_string(), (now + f) / 2.0, 0.7, format!("Not this minute. A better hour comes later today{}.", if band >= Band::Steady { "" } else { ", and the day itself is slow" }), Some((s, e)), format!("the best hour left today, fit {:.0}", f * 100.0))
            } else {
                ("Not today".to_string(), now.min(0.42), 0.7, format!("Today does not serve {name}. Let it wait for the stretch below."), st.best.map(|b| (b.0, b.1)), "the next good stretch".to_string())
            }
        }

        // -------------------------------------------------------------------- should I
        Question::ShouldI => {
            push_common(&mut reasons, &mut rule_ids);
            let week: f64 = samples.iter().take(7).map(|s| s.scores[idx]).sum::<f64>() / 7.0_f64.min(samples.len() as f64);
            let best = st.best;
            reasons.insert(0, format!("the next seven days average {:.0} for {name}; the best stretch of the half-year reaches {:.0}", week * 100.0, ahead_best * 100.0));
            rule_ids.insert(0, "KS-G-0001");
            let ground = natal_c.score;
            if ground >= 0.5 && week >= ahead_best - 0.06 && week >= 0.5 {
                ("Take it now".to_string(), (0.5 * week + 0.3 * ground + 0.2 * witness).clamp(0.0, 1.0), agreement(&[ground, week, witness]), format!("Take it now. The next seven days ({:.0}) are as good as anything the half-year offers ({:.0}), and your birth chart favours {name}.", week * 100.0, ahead_best * 100.0), Some((at, at.add_ns(7 * NS_PER_DAY))), "this week".to_string())
            } else if ground >= 0.5 && best.map_or(false, |b| b.2 >= week + 0.06) {
                let b = best.unwrap();
                ("Take it, from the window".to_string(), (0.4 * b.2 + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0), agreement(&[ground, b.2, witness]), format!("Take it, but not this week. This week reads {:.0}; from {} the same step reads {:.0}. Prepare now, move then.", week * 100.0, date_words(b.0, lon), b.2 * 100.0), Some((b.0, b.1)), "the window to move in".to_string())
            } else if ground < 0.45 && week < 0.5 {
                ("Hold".to_string(), (0.4 * week + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0), agreement(&[ground, week, witness]), format!("Hold. Your birth chart does not make {name} easy ({:.0}) and the coming week is slow ({:.0}). If you must move, keep the stakes small and the exit open.", ground * 100.0, week * 100.0), best.map(|b| (b.0, b.1)), "the least bad window".to_string())
            } else {
                ("Take it, with small stakes".to_string(), (0.4 * week + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0), agreement(&[ground, week, witness]), format!("Take it with small stakes. The week is fair ({:.0}) and your birth chart is {} on {name}; a step you can undo is the right size.", week * 100.0, if ground >= 0.5 { "willing" } else { "cautious" }), Some((at, at.add_ns(7 * NS_PER_DAY))), "this week, lightly".to_string())
            }
        }

        // ------------------------------------------------------------- will it come back
        Question::WillItReturn => {
            push_common(&mut reasons, &mut rule_ids);
            let month = (30.4375 * NS_PER_DAY as f64) as i128;
            let slow = slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(-12 * month), month, 37)?;
            let past: Vec<f64> = slow.iter().take(12).map(|(_, v)| v[idx]).collect();
            let future: Vec<(Instant, f64)> = slow.iter().skip(13).map(|(t, v)| (*t, v[idx])).collect();
            let now_slow = slow.get(12).map(|(_, v)| v[idx]).unwrap_or(now);
            let past_peak = past.iter().cloned().fold(0.0_f64, f64::max);
            let (fut_t, fut_peak) = future.iter().cloned().fold((at, 0.0_f64), |a, (t, v)| if v > a.1 { (t, v) } else { a });
            let back_at = future.iter().find(|(_, v)| *v >= past_peak - 0.03).map(|(t, _)| *t);
            reasons.insert(0, format!("the long-term trend of {name} peaked at {:.0} in the past year, stands at {:.0} now, and reaches {:.0} around {}", past_peak * 100.0, now_slow * 100.0, fut_peak * 100.0, date_words(fut_t, lon)));
            rule_ids.insert(0, "KS-G-0001");
            let dip = now_slow < past_peak - 0.05;
            if dip && back_at.is_some() {
                let t = back_at.unwrap();
                let months = months_between(at, t);
                // A yes is at least a half; nearer and fuller returns score higher.
                let score = (0.58 + 0.25 * (1.0 - months / 24.0).max(0.0) + 0.5 * (fut_peak - past_peak).clamp(-0.1, 0.1) + 0.1 * (witness - 0.5)).clamp(0.5, 0.9);
                let v = if months <= 12.0 { "Yes, it comes back" } else { "Yes, in its season" };
                (v.to_string(), score, agreement(&[natal_c.score, fut_peak, witness]), format!("Yes. {} fell from {:.0} to {:.0} in the long-term trend, and climbs back to where it was around {}. Keep the door open and do not chase; the return is the season's, not an argument's.", title, past_peak * 100.0, now_slow * 100.0, date_words(t, lon)), Some((t, t.add_ns(60 * NS_PER_DAY))), "when it is back where it was".to_string())
            } else if dip {
                let score = (0.42 + 0.1 * (witness - 0.5)).clamp(0.3, 0.55);
                ("Not as it was".to_string(), score, agreement(&[natal_c.score, fut_peak, witness]), format!("Not as it was. {} eases to {:.0} around {} but does not reach the {:.0} of the past year within two years. What comes back comes changed, or what comes is new.", title, fut_peak * 100.0, date_words(fut_t, lon), past_peak * 100.0), Some((fut_t, fut_t.add_ns(60 * NS_PER_DAY))), "the best it reaches".to_string())
            } else {
                let score = (0.6 + 0.2 * (witness - 0.5)).clamp(0.45, 0.8);
                ("Nothing has truly left".to_string(), score, agreement(&[natal_c.score, now_slow, witness]), format!("Nothing has truly left. The long-term trend of {name} stands at {:.0}, as high as any month of the past year ({:.0}). What you miss is within reach; the move is yours, and the window below is the time for it.", now_slow * 100.0, past_peak * 100.0), best_window(&samples, idx, 0.56, 2).map(|w| (w.0, w.1)), "the time to reach out".to_string())
            }
        }

        // ------------------------------------------------------------ will it be set right
        Question::WillItRecover => {
            push_common(&mut reasons, &mut rule_ids);
            let e = explain(spk, eop, natal, obs, system, ayanamsa, life, at)?;
            let lift = e.relief.iter().find(|r| r.lasting && r.gain >= 0.015).cloned();
            if let Some(r) = &lift {
                reasons.insert(0, format!("the first lasting lift for {name} comes on {}: {} ({:+.0} points)", date_words(r.at, lon), r.what, r.gain * 100.0));
                rule_ids.insert(0, "KS-G-0001");
            }
            if let Some(c) = e.causes.iter().find(|c| matches!(c.kind, "dasha" | "gochara")) {
                reasons.insert(0, format!("what weighs on {name} now: {}{}", c.what, if c.until_words.is_empty() { String::new() } else { format!(", {}", c.until_words) }));
                rule_ids.insert(0, c.rule_id);
            }
            match (&e.eases, &lift) {
                (Some(t), _) => {
                    let months = months_between(at, t.at);
                    let score = (0.8 - months / 60.0).clamp(0.5, 0.85);
                    let soon = months <= 12.0;
                    let mut h = format!("{} {} climbs to {} from {}", if soon { "Yes, it mends." } else { "It mends, slowly." }, title, t.band.word(), date_words(t.at, lon));
                    if let Some(r) = &lift {
                        h.push_str(&format!("; the first lift comes earlier, on {}, when {}", date_words(r.at, lon), r.what));
                    }
                    h.push_str(". Steady care until then; the season does the rest.");
                    (if soon { "Yes, it mends" } else { "It mends, slowly" }.to_string(), score, agreement(&[natal_c.score, ahead, witness]).max(0.55), h, Some((t.at, t.at.add_ns(60 * NS_PER_DAY))), "when it climbs a band".to_string())
                }
                (None, Some(r)) => {
                    let score = (0.5 + r.gain).clamp(0.4, 0.6);
                    ("It steadies, slowly".to_string(), score, agreement(&[natal_c.score, ahead, witness]), format!("It steadies rather than mends. The first lift comes on {} ({}), but the season holds its band for three years. Hold what you have; the remedy below is the lever.", date_words(r.at, lon), r.what), Some((r.at, r.at.add_ns(30 * NS_PER_DAY))), "the first lift".to_string())
                }
                (None, None) => {
                    ("Not in this period".to_string(), 0.3, 0.6, format!("Not in this period. Nothing in the next two years lifts {name} lastingly and the season holds. This is where the remedy, not the sky, must do the work."), st.best.map(|b| (b.0, b.1)), "the best the sky gives meanwhile".to_string())
                }
            }
        }
    };

    if pv.hits.iter().any(|h| h.rule_id == "KS-P-0012") {
        reasons.push("the very moment you asked shows a hold-up: expect a delay before it moves".to_string());
        rule_ids.push("KS-P-0012");
    }

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
    fn every_question_about_every_life_is_answered_and_the_answers_differ() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for life in [Life::Love, Life::Career, Life::Money] {
            let mut headlines = Vec::new();
            let mut scores = Vec::new();
            for q in Question::ALL {
                let a = answer(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, life, q, at).unwrap();
                assert!(!a.verdict.is_empty() && !a.headline.is_empty());
                assert!((0.0..=1.0).contains(&a.score), "{q:?} {}", a.score);
                assert!((0.3..=0.95).contains(&a.confidence));
                assert!(a.reasons.len() >= 4);
                for s in std::iter::once(&a.verdict).chain(std::iter::once(&a.headline)).chain(std::iter::once(&a.when_words)).chain(a.reasons.iter()) {
                    assert_eq!(crate::plain::jargon(s), None, "{q:?}: {s}");
                }
                if let (Some(s), Some(e)) = (a.when_start, a.when_end) {
                    assert!(s <= e);
                }
                headlines.push(a.headline);
                scores.push(a.score);
            }
            // six questions, six different headlines; and the numbers are not all one number
            let mut uniq = headlines.clone();
            uniq.sort();
            uniq.dedup();
            assert_eq!(uniq.len(), 6, "{headlines:?}");
            let distinct = scores.iter().map(|s| (s * 100.0).round() as i64).collect::<std::collections::BTreeSet<_>>();
            assert!(distinct.len() >= 3, "{scores:?}");
        }
    }
}
