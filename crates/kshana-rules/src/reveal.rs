//! The past, told first, and the birth time sharpened from the answers.
//!
//! [`past_reveal`]: the strongest dated stretches of a person's life from their early
//! teens to now, said as statements the person marks Yes, Close or No. Each life's slow
//! score (periods and slow movers) is sampled monthly; a stretch is a three-month
//! window standing furthest from that life's own average, in standard deviations. In
//! the first reveal the stretches chosen are the ones that hold across the whole
//! birth-time window (they do not depend on the exact minute), so they are the safest
//! to say; asked for more, the stretches chosen are the ones where nearby birth times
//! disagree most, so every answer splits the candidates.
//!
//! [`refine_birth`]: every candidate birth time in the window is scored by how well it
//! explains the answers. A Yes rewards the times that mark the stretch strongly, a No
//! rewards the times that do not mark it, and Close counts half. A wrong guess still
//! helps, because it narrows down the birth time. A mild prior keeps the recorded time
//! unless the answers clearly prefer another.
//!
//! [`seal_pick`]: one stretch of the coming weeks, the strongest the reading marks,
//! said plainly so it can be sealed now and opened when it has passed.

use crate::forecast::{sky, slow_scores, ForecastError, Life, Natal};
use crate::lang::{date, month_year};
use crate::pastcheck::{rest, said, PastCheck};
use crate::plain::Lang;
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa};

const MONTH_NS: i128 = (30.4375 * NS_PER_DAY as f64) as i128;
const YEAR_NS: i128 = 365 * NS_PER_DAY + NS_PER_DAY / 4;
const WINDOW: usize = 3;

/// How the person answered a stretch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It happened.
    Yes,
    /// Something like it happened, or near that time.
    Close,
    /// It did not happen.
    No,
}

/// One answered stretch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Answer {
    /// First day of the stretch.
    pub from: Instant,
    /// Last day.
    pub to: Instant,
    /// Which life.
    pub life: Life,
    /// True if the stretch was said to be good.
    pub good: bool,
    /// The answer.
    pub verdict: Verdict,
}

/// One candidate birth time, scored against the answers.
#[derive(Clone, Debug, PartialEq)]
pub struct BirthFit {
    /// The candidate.
    pub birth: Instant,
    /// Minutes from the recorded time (positive = later).
    pub shift_min: f64,
    /// Share of the belief on this time, all candidates summing to 1.
    pub prob: f64,
    /// How many answers this time explains.
    pub explained: usize,
    /// Rising sign, 0 = Aries.
    pub lagna_sign: u8,
    /// Moon's star, 0 = Ashwini.
    pub moon_star: usize,
}

fn local_max(z: &[f64], i: usize) -> bool {
    let a = z[i].abs();
    (i == 0 || a >= z[i - 1].abs()) && (i + 1 >= z.len() || a >= z[i + 1].abs())
}

/// The nine lives' slow scores at each instant.
fn scores_at(spk: &Spk, eop: Option<&EopTable>, natal: &Natal, obs: &Observer, ayanamsa: Ayanamsa, at: &[Instant]) -> Result<Vec<Vec<f64>>, ForecastError> {
    let mut out = Vec::with_capacity(at.len());
    for &t in at {
        let s = sky(spk, eop, natal, obs, HouseSystem::WholeSign, ayanamsa, t)?;
        let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, t, 3);
        out.push(slow_scores(natal, &s, &levels));
    }
    Ok(out)
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    if v.is_empty() {
        return (0.0, 1.0);
    }
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64;
    (m, var.sqrt().max(1e-6))
}

/// Per life, the rolling three-month window's distance from the life's own average in
/// standard deviations.
fn zscores(vals: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = vals.len();
    (0..Life::ALL.len())
        .map(|li| {
            let col: Vec<f64> = vals.iter().map(|v| v[li]).collect();
            let (m, sd) = mean_sd(&col);
            if n < WINDOW {
                return Vec::new();
            }
            (0..=n - WINDOW).map(|i| (col[i..i + WINDOW].iter().sum::<f64>() / WINDOW as f64 - m) / sd).collect()
        })
        .collect()
}

/// Candidate births spread over ±`window_min` minutes, the recorded one first.
fn candidates(birth: Instant, window_min: f64, n_side: usize) -> Vec<(f64, Instant)> {
    let mut v = vec![(0.0, birth)];
    if window_min >= 2.0 && n_side > 0 {
        for k in 1..=n_side {
            let m = window_min * k as f64 / n_side as f64;
            for s in [-m, m] {
                v.push((s, birth.add_ns((s * 60.0 * NS_PER_SEC as f64) as i128)));
            }
        }
    }
    v
}

/// The strongest dated stretches of the person's life, from age 14 to `at` (at most 25
/// years), skipping the stretches already asked. `sharpen` picks the stretches that best
/// split nearby birth times; otherwise the safest, the ones every nearby time agrees on.
#[allow(clippy::too_many_arguments)]
pub fn past_reveal(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    lang: Lang,
    at: Instant,
    window_min: f64,
    asked: &[(Instant, Instant)],
    count: usize,
    sharpen: bool,
) -> Result<Vec<PastCheck>, ForecastError> {
    let earliest = natal.birth.add_ns(14 * YEAR_NS);
    let start = at.add_ns(-25 * YEAR_NS).max(earliest);
    if start.add_ns(12 * MONTH_NS) >= at {
        return Ok(Vec::new());
    }
    let n = ((at.ns_since(start) / MONTH_NS) as usize).max(WINDOW + 1);
    let months: Vec<Instant> = (0..n).map(|k| start.add_ns(MONTH_NS * k as i128)).collect();

    // The recorded birth, and nearby births across the window.
    let cands = candidates(natal.birth, window_min, if sharpen { 3 } else { 2 });
    let mut zs: Vec<Vec<Vec<f64>>> = Vec::with_capacity(cands.len());
    for (k, (_, b)) in cands.iter().enumerate() {
        let nat = if k == 0 { None } else { Natal::compute(spk, eop, *b, obs, system, ayanamsa).ok() };
        let n_ref = nat.as_ref().unwrap_or(natal);
        let vals = scores_at(spk, eop, n_ref, obs, ayanamsa, &months)?;
        zs.push(zscores(&vals));
    }

    let asked_idx: Vec<(i128, i128)> = asked.iter().map(|(a, b)| (a.ns_since(start) / MONTH_NS, b.ns_since(start) / MONTH_NS)).collect();
    // (value, life, index, good, strength)
    let mut pool: Vec<(f64, usize, usize, bool, f64)> = Vec::new();
    for li in 0..Life::ALL.len() {
        let z0 = &zs[0][li];
        for i in 0..z0.len() {
            if z0[i].abs() < 0.8 || !local_max(z0, i) {
                continue;
            }
            let lo = i as i128;
            let hi = lo + WINDOW as i128 - 1;
            if asked_idx.iter().any(|(a, b)| lo <= b + 2 && hi >= a - 2) {
                continue;
            }
            let sign = z0[i].signum();
            let others: Vec<f64> = zs[1..].iter().map(|z| z[li].get(i).copied().unwrap_or(0.0) * sign).collect();
            let (agree, spread) = if others.is_empty() { (z0[i].abs(), 0.0) } else { mean_sd(&others) };
            let spread = if others.is_empty() { 0.0 } else { spread };
            let value = if sharpen { 1.5 * spread + 0.3 * z0[i].abs() } else { z0[i].abs() + 0.5 * agree - 0.7 * spread };
            pool.push((value, li, i, sign > 0.0, z0[i].abs()));
        }
    }
    pool.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut chosen: Vec<(f64, usize, usize, bool, f64)> = Vec::new();
    // First pass one per life, then a second per life if more are needed.
    for per_life in [1usize, 2] {
        for c in &pool {
            if chosen.len() >= count {
                break;
            }
            if chosen.iter().filter(|x| x.1 == c.1).count() >= per_life {
                continue;
            }
            if chosen.iter().any(|x| (x.2 as i64 - c.2 as i64).abs() < 6) {
                continue;
            }
            chosen.push(*c);
        }
    }
    chosen.sort_by_key(|c| c.2);
    let lon = obs.longitude_deg;
    Ok(chosen
        .into_iter()
        .map(|(_, li, i, good, strength)| {
            let from = months[i];
            let to = months[(i + WINDOW - 1).min(months.len() - 1)].add_ns(MONTH_NS - NS_PER_DAY);
            let life = Life::ALL[li];
            PastCheck {
                from,
                to,
                life,
                question: crate::pastcheck::question(lang, life, good, from, to, lon),
                statement: crate::pastcheck::statement(lang, life, good, from, to, lon),
                good,
                strength,
            }
        })
        .collect())
}

/// Score every birth time within ±`window_min` minutes of `birth` in steps of
/// `step_min` against the answers. Best first. A No counts as much as a Yes: a wrong
/// guess still helps, because it narrows down the birth time.
#[allow(clippy::too_many_arguments)]
pub fn refine_birth(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    birth: Instant,
    window_min: f64,
    step_min: f64,
    answers: &[Answer],
) -> Vec<BirthFit> {
    if answers.is_empty() {
        return Vec::new();
    }
    let window_min = window_min.clamp(0.0, 180.0);
    // At most 61 candidates: the step widens with the window.
    let step = step_min.max(window_min / 30.0).max(0.5);
    let n_side = (window_min / step).floor() as i64;
    // The months that set each life's average: from two years before the first answer
    // to the last, every second month, plus every month inside each answer.
    let first = answers.iter().map(|a| a.from).min().unwrap_or(birth);
    let last = answers.iter().map(|a| a.to).max().unwrap_or(birth);
    let base_from = first.add_ns(-24 * MONTH_NS).max(birth.add_ns(12 * YEAR_NS));
    let mut base: Vec<Instant> = Vec::new();
    let mut t = base_from;
    while t <= last {
        base.push(t);
        t = t.add_ns(2 * MONTH_NS);
    }
    let inside: Vec<Vec<Instant>> = answers
        .iter()
        .map(|a| {
            let mut v = Vec::new();
            let mut t = a.from.add_ns(15 * NS_PER_DAY);
            while t <= a.to {
                v.push(t);
                t = t.add_ns(MONTH_NS);
            }
            if v.is_empty() {
                v.push(a.from);
            }
            v
        })
        .collect();

    let sigma = window_min.max(4.0);
    let score = |k: i64| -> Option<(f64, BirthFit)> {
        let shift = k as f64 * step;
        let b = birth.add_ns((shift * 60.0 * NS_PER_SEC as f64) as i128);
        let natal = Natal::compute(spk, eop, b, obs, system, ayanamsa).ok()?;
        let bv = scores_at(spk, eop, &natal, obs, ayanamsa, &base).ok()?;
        let mut logp = -0.5 * (shift / sigma) * (shift / sigma);
        let mut explained = 0;
        for (a, months) in answers.iter().zip(&inside) {
            let li = Life::ALL.iter().position(|l| *l == a.life).unwrap_or(0);
            let col: Vec<f64> = bv.iter().map(|v| v[li]).collect();
            let (m, sd) = mean_sd(&col);
            let Ok(iv) = scores_at(spk, eop, &natal, obs, ayanamsa, months) else { continue };
            let w = iv.iter().map(|v| v[li]).sum::<f64>() / iv.len() as f64;
            let z = (w - m) / sd * if a.good { 1.0 } else { -1.0 };
            let p = (1.0 / (1.0 + (-1.6 * z).exp())).clamp(0.02, 0.98);
            let (l, hit) = match a.verdict {
                Verdict::Yes => (p.ln(), z > 0.3),
                Verdict::Close => ((0.5 * p + 0.25).ln(), z > 0.0),
                Verdict::No => ((1.0 - p).ln(), z < 0.3),
            };
            logp += l;
            if hit {
                explained += 1;
            }
        }
        Some((
            logp,
            BirthFit { birth: b, shift_min: shift, prob: 0.0, explained, lagna_sign: natal.facts.lagna_sign, moon_star: natal.janma() },
        ))
    };
    // Every candidate is independent: spread them over the phone's cores.
    let ks: Vec<i64> = (-n_side..=n_side).collect();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(1, 8);
    let chunk = ks.len().div_ceil(threads).max(1);
    let out: Vec<(f64, BirthFit)> = std::thread::scope(|s| {
        let handles: Vec<_> = ks.chunks(chunk).map(|part| s.spawn(|| part.iter().filter_map(|&k| score(k)).collect::<Vec<_>>())).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    let top = out.iter().map(|x| x.0).fold(f64::NEG_INFINITY, f64::max);
    let total: f64 = out.iter().map(|x| (x.0 - top).exp()).sum();
    let mut fits: Vec<BirthFit> = out
        .into_iter()
        .map(|(l, mut f)| {
            f.prob = (l - top).exp() / total.max(1e-12);
            f
        })
        .collect();
    fits.sort_by(|a, b| b.prob.partial_cmp(&a.prob).unwrap_or(std::cmp::Ordering::Equal));
    fits
}

/// The time the answers point to, if they clearly beat the recorded one: at least four
/// answers, the best time at least three times as likely as the recorded time, and at
/// least two minutes away.
pub fn suggested_shift(fits: &[BirthFit], answered: usize) -> Option<BirthFit> {
    let best = fits.first()?;
    let here = fits.iter().find(|f| f.shift_min.abs() < 1e-9)?;
    (answered >= 4 && best.shift_min.abs() >= 2.0 && best.prob >= 3.0 * here.prob).then(|| best.clone())
}

/// The stretch of the coming weeks the reading marks most strongly, to be sealed now
/// and opened when it has passed: two to three weeks, starting one to eight weeks out.
#[allow(clippy::too_many_arguments)]
pub fn seal_pick(spk: &Spk, eop: Option<&EopTable>, natal: &Natal, obs: &Observer, ayanamsa: Ayanamsa, lang: Lang, at: Instant) -> Result<Option<PastCheck>, ForecastError> {
    let week = 7 * NS_PER_DAY;
    let base: Vec<Instant> = (-12i128..=12).map(|k| at.add_ns(k * MONTH_NS)).collect();
    let weeks: Vec<Instant> = (1i128..=11).map(|k| at.add_ns(k * week)).collect();
    let bv = scores_at(spk, eop, natal, obs, ayanamsa, &base)?;
    let wv = scores_at(spk, eop, natal, obs, ayanamsa, &weeks)?;
    let mut best: Option<(f64, usize, usize, bool)> = None;
    for li in 0..Life::ALL.len() {
        let (m, sd) = mean_sd(&bv.iter().map(|v| v[li]).collect::<Vec<_>>());
        for i in 0..=weeks.len() - 3 {
            let z = ((wv[i][li] + wv[i + 1][li] + wv[i + 2][li]) / 3.0 - m) / sd;
            // A good stretch is preferred: what is sealed is waited for, and waiting for
            // a hard time is no kindness. A hard one is sealed only when it clearly
            // outweighs every good one (and the app pairs it with what helps).
            let value = if z > 0.0 { z } else { 0.6 * z.abs() };
            if best.map(|b| value > b.0).unwrap_or(true) {
                best = Some((value, li, i, z > 0.0));
            }
        }
    }
    let Some((strength, li, i, good)) = best else { return Ok(None) };
    if strength < 0.5 {
        return Ok(None);
    }
    let life = Life::ALL[li];
    let from = weeks[i];
    let to = weeks[i + 2].add_ns(week - NS_PER_DAY);
    let lon = obs.longitude_deg;
    let a = date(lang, from, lon);
    let b = date(lang, to, lon);
    let said = lang.pick(said(life, good));
    let statement = match lang {
        Lang::En => format!("Between {a} and {b}: {said}."),
        Lang::Ml => format!("{a} മുതൽ {b} വരെ: {said}."),
        Lang::Hi => format!("{a} से {b} तक: {said}।"),
    };
    let r = lang.pick(rest(life, good));
    let question = match lang {
        Lang::En => format!("Between {a} and {b}, {r}"),
        Lang::Ml => format!("{a} മുതൽ {b} വരെ, {r}"),
        Lang::Hi => format!("{a} से {b} के बीच, {r}"),
    };
    let _ = month_year;
    Ok(Some(PastCheck { from, to, life, question, statement, good, strength }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn setup() -> Option<(Spk, Observer, Natal, Instant)> {
        let spk = kernel()?;
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        Some((spk, obs, natal, at))
    }

    #[test]
    fn the_reveal_spans_the_adult_life_plainly_and_more_never_repeats() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let t = std::time::Instant::now();
        let first = past_reveal(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &[], 5, false).unwrap();
        eprintln!("reveal in {:?}", t.elapsed());
        assert_eq!(first.len(), 5, "{first:?}");
        for c in &first {
            eprintln!("{}", c.statement);
            assert_eq!(jargon_in(Lang::En, &c.statement), None);
            assert!(c.from >= natal.birth.add_ns(14 * YEAR_NS) && c.to <= at.add_ns(31 * NS_PER_DAY));
        }
        let asked: Vec<_> = first.iter().map(|c| (c.from, c.to)).collect();
        let t = std::time::Instant::now();
        let more = past_reveal(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &asked, 3, true).unwrap();
        eprintln!("more in {:?}", t.elapsed());
        assert!(!more.is_empty());
        for m in &more {
            eprintln!("more: {}", m.statement);
            assert!(asked.iter().all(|(a, b)| m.to < *a || m.from > *b), "repeated {m:?}");
        }
    }

    #[test]
    fn answers_pull_the_birth_time_toward_the_time_that_explains_them() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        // Answers as a person born 12 minutes later would give them: take that chart's
        // reveal, say Yes to all of it.
        let shift = 12.0;
        let truth = Natal::compute(&spk, None, natal.birth.add_ns((shift * 60.0) as i128 * NS_PER_SEC), &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let mut asked = Vec::new();
        let mut answers = Vec::new();
        for round in 0..3 {
            let qs = past_reveal(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &asked, 4, round > 0).unwrap();
            for q in qs {
                asked.push((q.from, q.to));
                // Does the true chart mark this stretch the same way?
                let fits = refine_birth(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, truth.birth, 0.0, 1.0, &[Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: Verdict::Yes }]);
                let yes = fits.first().map(|f| f.explained == 1).unwrap_or(false);
                answers.push(Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: if yes { Verdict::Yes } else { Verdict::No } });
            }
        }
        let t = std::time::Instant::now();
        let fits = refine_birth(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, 30.0, 2.0, &answers);
        eprintln!("refine {} answers over {} times in {:?}", answers.len(), fits.len(), t.elapsed());
        let yes = answers.iter().filter(|a| a.verdict == Verdict::Yes).count();
        eprintln!("{} yes of {}; best shift {:+.0} min ({:.0}%)", yes, answers.len(), fits[0].shift_min, fits[0].prob * 100.0);
        let total: f64 = fits.iter().map(|f| f.prob).sum();
        assert!((total - 1.0).abs() < 1e-6);
        // The answers must move the belief toward the truth, not away from it.
        let mean: f64 = fits.iter().map(|f| f.prob * f.shift_min).sum();
        assert!(mean > 0.0, "belief moved the wrong way: mean shift {mean:+.1}");
    }

    #[test]
    fn a_stretch_of_the_coming_weeks_is_picked_to_seal() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let s = seal_pick(&spk, None, &natal, &obs, Ayanamsa::Lahiri, Lang::En, at).unwrap();
        let s = s.expect("a strong enough stretch");
        eprintln!("{}\n{}", s.statement, s.question);
        assert!(s.from > at.add_ns(6 * NS_PER_DAY) && s.to < at.add_ns(90 * NS_PER_DAY));
        assert_eq!(jargon_in(Lang::En, &s.statement), None);
    }
}
