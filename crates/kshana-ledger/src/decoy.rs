//! Asked-first retrodiction with decoy windows: "did something happen to you in
//! 1998" done without cold reading.
//!
//! Before the person types any event, the engine commits to dated questions. For an
//! area it reads every antardasha of a span through the exact
//! [`FitProfile`](crate::FitProfile), takes the strongest as signal windows and, for
//! each, picks decoys of similar length from the weakest, shuffles each question's
//! windows by a hash of the chart (deterministic, so the app can regenerate and
//! check), and records a SHA-256 commitment of the windows and the answer key. The
//! person is asked which window it was; the chart's reasoning stays hidden until
//! they answer. [`grade`] counts hits against the forced-choice chance rate
//! `1 / (decoys + 1)` with the exact binomial tail and a Wilson interval, so the
//! headline is always the hit rate next to chance, never the hit rate alone.

use crate::coverage::fit_profile;
use crate::event::{iso_utc, month_year, EventKind, LedgerError};
use crate::hash::{seed_of, sha256_hex, SplitMix};
use crate::metrics::{count_words, wilson_interval, Z95};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::area;
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{subdivide, vimshottari, Ayanamsa};

/// One committed question.
#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    /// The area asked about.
    pub area: String,
    /// The windows offered, signal and decoys shuffled, as exact instants.
    pub windows: Vec<(Instant, Instant)>,
    /// Index of the signal window in `windows`: the answer key, to be hidden until
    /// the person answers.
    pub predicted_index: usize,
    /// The question in words, with the windows numbered from 1.
    pub prompt: String,
    /// SHA-256 of the area, the windows and the answer key, to log before asking.
    pub commitment: String,
}

/// The grade of a quiz.
#[derive(Clone, Debug, PartialEq)]
pub struct QuizResult {
    /// Answers that named the signal window.
    pub correct: usize,
    /// Questions answered.
    pub total: usize,
    /// Questions skipped (answer `None`).
    pub skipped: usize,
    /// `1 / windows per question`, averaged over the answered questions.
    pub chance_rate: f64,
    /// Exact binomial `P[X ≥ correct]` with `total` trials at `chance_rate`.
    pub binomial_pvalue: f64,
    /// Wilson 95 % interval of the hit rate.
    pub hit_interval: (f64, f64),
    /// The grade in words.
    pub words: String,
}

/// Exact binomial upper tail `P[X ≥ k]` for `n` trials at `p`.
pub fn binomial_p_at_least(k: usize, n: usize, p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    if k == 0 {
        return 1.0;
    }
    if k > n {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    if p <= 0.0 {
        return 0.0;
    }
    let mut pmf = (1.0 - p).powi(n as i32);
    let mut tail = 0.0;
    for i in 0..=n {
        if i >= k {
            tail += pmf;
        }
        pmf *= (n - i) as f64 / (i + 1) as f64 * p / (1.0 - p);
    }
    tail.clamp(0.0, 1.0)
}

fn commitment_text(area_name: &str, windows: &[(Instant, Instant)], predicted: usize) -> String {
    let w: Vec<String> = windows
        .iter()
        .map(|(s, e)| format!("{}/{}", iso_utc(*s), iso_utc(*e)))
        .collect();
    format!("{area_name}|{}|{predicted}", w.join(","))
}

/// Commit `n_questions` questions for an area over `[from, to]`, each with one
/// signal window (a strongest antardasha, favourable reading) and
/// `decoys_per_question` decoys of similar length from the weakest half. Fewer
/// questions are returned when the span holds too few antardasha cells.
#[allow(clippy::too_many_arguments)]
pub fn retrodiction_quiz(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    area_name: &str,
    from: Instant,
    to: Instant,
    n_questions: usize,
    decoys_per_question: usize,
) -> Result<Vec<Question>, LedgerError> {
    let a = area(area_name).ok_or_else(|| LedgerError::UnknownArea(area_name.to_string()))?;
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let from = from.max(natal.instant);
    if to <= from || n_questions == 0 {
        return Ok(Vec::new());
    }
    let profile = fit_profile(spk, natal, ayanamsa, a.name, EventKind::Milestone, from, to)?;
    // Every antardasha overlapping the span, clipped, with its mean fit.
    let mut cells: Vec<((Instant, Instant), f64)> = Vec::new();
    for m in vimshottari(natal.bodies[1].sidereal, natal.instant, YEAR_365_25_NS) {
        if m.end <= from || m.start >= to {
            continue;
        }
        for p in subdivide(m) {
            let s = p.start.max(from);
            let e = p.end.min(to);
            if e > s {
                if let Some(fit) = profile.mean_fit_over(s, e) {
                    cells.push(((s, e), fit));
                }
            }
        }
    }
    let per_question = decoys_per_question + 1;
    let n_questions = n_questions.min(cells.len() / per_question);
    if n_questions == 0 {
        return Ok(Vec::new());
    }
    let by_strength = |x: &((Instant, Instant), f64), y: &((Instant, Instant), f64)| {
        y.1.partial_cmp(&x.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.0 .0.cmp(&y.0 .0))
    };
    cells.sort_by(by_strength);
    let signals: Vec<((Instant, Instant), f64)> = cells.iter().take(n_questions).cloned().collect();
    // The decoy pool: the rest, weakest first.
    let mut pool: Vec<((Instant, Instant), f64)> =
        cells.iter().skip(n_questions).cloned().collect();
    pool.reverse();
    let keep = (pool.len() / 2).max(n_questions * decoys_per_question);
    pool.truncate(keep.min(pool.len()));
    let log_len = |w: (Instant, Instant)| (w.1.ns_since(w.0).max(1) as f64).ln();
    let chart_key = format!(
        "{}|{:.9}|{:.9}|{}",
        natal.instant.tai_ns(),
        natal.bodies[1].sidereal,
        natal.cusps_sidereal[0],
        a.name
    );
    let mut out = Vec::with_capacity(n_questions);
    for (qi, (signal, _)) in signals.iter().enumerate() {
        let mut windows = vec![*signal];
        for _ in 0..decoys_per_question {
            if pool.is_empty() {
                break;
            }
            let target = log_len(*signal);
            let (best, _) = pool
                .iter()
                .enumerate()
                .map(|(i, (w, fit))| (i, ((log_len(*w) - target).abs(), *fit)))
                .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap();
            windows.push(pool.remove(best).0);
        }
        let mut order: Vec<usize> = (0..windows.len()).collect();
        SplitMix::new(seed_of(&format!("{chart_key}|{qi}"))).shuffle(&mut order);
        let shuffled: Vec<(Instant, Instant)> = order.iter().map(|i| windows[*i]).collect();
        let predicted_index = order.iter().position(|i| *i == 0).unwrap_or(0);
        let listed: Vec<String> = shuffled
            .iter()
            .enumerate()
            .map(|(i, (s, e))| format!("{}. {} to {}", i + 1, month_year(*s), month_year(*e)))
            .collect();
        let prompt = format!(
            "In which one of these stretches did something of note happen in your {}? {}. Answer with the number, or say you are not sure.",
            a.name,
            listed.join("; ")
        );
        let commitment = sha256_hex(&commitment_text(a.name, &shuffled, predicted_index));
        out.push(Question {
            area: a.name.to_string(),
            windows: shuffled,
            predicted_index,
            prompt,
            commitment,
        });
    }
    Ok(out)
}

/// Whether a question's commitment still matches its windows and answer key.
pub fn verify_question(q: &Question) -> bool {
    q.commitment == sha256_hex(&commitment_text(&q.area, &q.windows, q.predicted_index))
}

/// Grade answers (the chosen window's index, or `None` for "not sure") against the
/// questions, paired by position.
pub fn grade(questions: &[Question], answers: &[Option<usize>]) -> QuizResult {
    let mut correct = 0;
    let mut total = 0;
    let mut skipped = 0;
    let mut chance_sum = 0.0;
    for (q, a) in questions.iter().zip(answers) {
        match a {
            None => skipped += 1,
            Some(i) => {
                total += 1;
                chance_sum += 1.0 / q.windows.len().max(1) as f64;
                if *i == q.predicted_index {
                    correct += 1;
                }
            }
        }
    }
    skipped += questions.len().saturating_sub(answers.len());
    let chance_rate = if total == 0 {
        questions
            .first()
            .map_or(0.0, |q| 1.0 / q.windows.len().max(1) as f64)
    } else {
        chance_sum / total as f64
    };
    let binomial_pvalue = if total == 0 {
        1.0
    } else {
        binomial_p_at_least(correct, total, chance_rate)
    };
    let hit_interval = wilson_interval(correct, total, Z95);
    let words = if total == 0 {
        format!(
            "No question answered yet ({} skipped); chance would be {:.0}% per question.",
            skipped,
            chance_rate * 100.0
        )
    } else {
        let expected = chance_rate * total as f64;
        let verdict = if binomial_pvalue <= 0.05 {
            "more than chance would give"
        } else {
            "no more than chance would give"
        };
        format!(
            "You named the chart's window in {}; guessing would get about {expected:.1} of {total} ({:.0}% each), so this is {verdict} (p = {binomial_pvalue:.3}){}.",
            count_words(correct, total),
            chance_rate * 100.0,
            if skipped > 0 {
                format!("; {skipped} skipped")
            } else {
                String::new()
            }
        )
    };
    QuizResult {
        correct,
        total,
        skipped,
        chance_rate,
        binomial_pvalue,
        hit_interval,
        words,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};

    #[test]
    fn binomial_tail_is_exact() {
        assert!((binomial_p_at_least(2, 3, 0.5) - 0.5).abs() < 1e-12);
        assert!((binomial_p_at_least(3, 3, 0.5) - 0.125).abs() < 1e-12);
        assert_eq!(binomial_p_at_least(0, 3, 0.5), 1.0);
        assert_eq!(binomial_p_at_least(4, 3, 0.5), 0.0);
        assert_eq!(binomial_p_at_least(1, 3, 1.0), 1.0);
        assert_eq!(binomial_p_at_least(1, 3, 0.0), 0.0);
        let p: f64 = 1.0 / 3.0;
        let want = 1.0 - (1.0 - p).powi(5);
        assert!((binomial_p_at_least(1, 5, p) - want).abs() < 1e-12);
    }

    #[test]
    fn kernel_quiz_commits_and_grades() {
        let Some((spk, ch)) = kochi() else { return };
        let qs = retrodiction_quiz(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            noon(2000, 1, 1),
            noon(2026, 1, 1),
            3,
            2,
        )
        .unwrap();
        assert_eq!(qs.len(), 3, "{qs:?}");
        let profile = fit_profile(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            EventKind::Milestone,
            noon(2000, 1, 1),
            noon(2026, 1, 1),
        )
        .unwrap();
        let mut seen = Vec::new();
        for q in &qs {
            assert_eq!(q.windows.len(), 3);
            assert!(q.predicted_index < 3);
            assert!(verify_question(q));
            assert_eq!(q.commitment.len(), 64);
            assert!(
                q.prompt.contains("1. ") && q.prompt.contains("3. "),
                "{}",
                q.prompt
            );
            assert!(
                !q.prompt.contains("fit"),
                "the prompt must not leak the reasoning"
            );
            for (s, e) in &q.windows {
                assert!(s < e);
                assert!(*s >= noon(2000, 1, 1) && *e <= noon(2026, 1, 1));
                assert!(!seen.contains(&(*s, *e)), "windows are not reused");
                seen.push((*s, *e));
            }
            // The signal is stronger than its decoys.
            let strength = |w: (Instant, Instant)| profile.mean_fit_over(w.0, w.1).unwrap();
            let signal = strength(q.windows[q.predicted_index]);
            for (i, w) in q.windows.iter().enumerate() {
                if i != q.predicted_index {
                    assert!(strength(*w) <= signal + 1e-12);
                }
            }
        }
        // Deterministic.
        let again = retrodiction_quiz(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            noon(2000, 1, 1),
            noon(2026, 1, 1),
            3,
            2,
        )
        .unwrap();
        assert_eq!(again, qs);
        // Grading.
        let key: Vec<Option<usize>> = qs.iter().map(|q| Some(q.predicted_index)).collect();
        let all = grade(&qs, &key);
        assert_eq!((all.correct, all.total, all.skipped), (3, 3, 0));
        assert!((all.chance_rate - 1.0 / 3.0).abs() < 1e-12);
        assert!((all.binomial_pvalue - 1.0 / 27.0).abs() < 1e-12);
        assert!(all.words.contains("more than chance"), "{}", all.words);
        let wrong: Vec<Option<usize>> = qs
            .iter()
            .map(|q| Some((q.predicted_index + 1) % 3))
            .collect();
        let none = grade(&qs, &wrong);
        assert_eq!(none.correct, 0);
        assert_eq!(none.binomial_pvalue, 1.0);
        assert!(none.words.contains("no more than chance"), "{}", none.words);
        let partial = grade(&qs, &[None, Some(qs[1].predicted_index)]);
        assert_eq!((partial.correct, partial.total, partial.skipped), (1, 1, 2));
        let empty = grade(&qs, &[None, None, None]);
        assert_eq!(empty.total, 0);
        assert_eq!(empty.binomial_pvalue, 1.0);
        assert!(empty.words.contains("No question answered"));
        let mut forged = qs[0].clone();
        forged.predicted_index = (forged.predicted_index + 1) % 3;
        assert!(!verify_question(&forged));
        // Too short a span for the request gives fewer questions, never an error.
        let few = retrodiction_quiz(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            noon(2010, 1, 1),
            noon(2011, 1, 1),
            3,
            2,
        )
        .unwrap();
        assert!(few.len() < 3);
        assert!(retrodiction_quiz(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Pets",
            noon(2000, 1, 1),
            noon(2026, 1, 1),
            1,
            1
        )
        .is_err());
    }
}
