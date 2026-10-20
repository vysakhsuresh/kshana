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
//! unless the answers clearly prefer another. Each candidate is judged by several
//! witnesses (the slow score, Yogini, Chara and Kalachakra periods, the KP cusp chain,
//! the matter's divisional chart; [`crate::witness`]) pooled on the log-odds scale, and
//! [`refine_birth_detail`] also reports how strong the evidence is against answers
//! given at random and where in the window the chart itself changes.
//!
//! [`seal_pick`]: one stretch of the coming weeks, the strongest the reading marks,
//! said plainly so it can be sealed now and opened when it has passed.

use crate::forecast::{sky, slow_scores, ForecastError, Life, Natal, Sky};
use crate::witness::{ascendant, boundaries, rebased, Boundary, Reading, Witness, NW, SLOPE, WITNESSES};
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
    /// Each witness's own log-likelihood of the answers against chance (nats), in the
    /// order of [`crate::witness::WITNESSES`]: what each technique alone says of this
    /// time.
    pub by_witness: [f64; NW],
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
    // Sharpening looks at more nearby times, since splitting them is its whole purpose.
    let cands = candidates(natal.birth, window_min, if sharpen { 6 } else { 2 });
    // The sky of each month once (it does not depend on the birth time), then every
    // candidate on the phone's cores: the slow score's distance for the statement, and
    // the pooled reading of every witness for how much the candidates disagree.
    let skies: Vec<Sky> = par_map(&months, |t| sky(spk, eop, natal, obs, HouseSystem::WholeSign, ayanamsa, *t)).into_iter().collect::<Result<_, _>>()?;
    let idx: Vec<usize> = (0..cands.len()).collect();
    let per: Vec<(Vec<Vec<f64>>, Vec<Vec<f64>>)> = par_map(&idx, |&k| {
        let nat = if k == 0 { None } else { Natal::compute(spk, eop, cands[k].1, obs, system, ayanamsa).ok() };
        let n_ref = nat.as_ref().unwrap_or(natal);
        let asc = ascendant(spk, eop, n_ref.birth, obs, ayanamsa).map(|a| a.0).unwrap_or(n_ref.facts.cusps[0]);
        let r = Reading::new(n_ref, asc);
        let vals: Vec<[[f64; 9]; NW]> = months
            .iter()
            .zip(&skies)
            .map(|(t, s)| {
                let own = rebased(s, n_ref);
                r.values(own.as_ref().unwrap_or(s), *t)
            })
            .collect();
        let slow: Vec<Vec<f64>> = vals.iter().map(|v| v[Witness::Slow.index()].to_vec()).collect();
        (zscores(&slow), r.pooled_rolling(&vals, WINDOW))
    });
    let (zs, pooled): (Vec<Vec<Vec<f64>>>, Vec<Vec<Vec<f64>>>) = per.into_iter().unzip();

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
            let value = if sharpen {
                // How much the candidates' pooled readings of this stretch disagree.
                let all: Vec<f64> = pooled.iter().map(|p| p[li].get(i).copied().unwrap_or(0.0) * sign).collect();
                1.5 * mean_sd(&all).1 + 0.3 * z0[i].abs()
            } else {
                z0[i].abs() + 0.5 * agree - 0.7 * spread
            };
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

/// The months a set of answers is judged over (from two years before the first answer
/// to the last, every second month, which sets each life's average; plus every month
/// inside each answer), with their skies.
pub(crate) struct Prepared {
    base: Vec<Instant>,
    inside: Vec<Vec<Instant>>,
    base_sky: Vec<Sky>,
    inside_sky: Vec<Vec<Sky>>,
}

fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(1, 8)
}

/// Run `f` over `items` on the phone's cores, keeping the order.
pub(crate) fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let chunk = items.len().div_ceil(threads()).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = items.chunks(chunk).map(|part| s.spawn(|| part.iter().map(&f).collect::<Vec<R>>())).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    })
}

/// The transits of every month the answers need, computed once: the sky does not depend
/// on the birth time, only the houses from the natal Moon do (see `witness::rebased`).
pub(crate) fn prepare(spk: &Spk, eop: Option<&EopTable>, obs: &Observer, system: HouseSystem, ayanamsa: Ayanamsa, birth: Instant, answers: &[Answer]) -> Option<Prepared> {
    let first = answers.iter().map(|a| a.from).min()?;
    let last = answers.iter().map(|a| a.to).max()?;
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
    let natal = Natal::compute(spk, eop, birth, obs, system, ayanamsa).ok()?;
    let all: Vec<Instant> = base.iter().copied().chain(inside.iter().flatten().copied()).collect();
    let skies: Vec<Sky> = par_map(&all, |t| sky(spk, eop, &natal, obs, HouseSystem::WholeSign, ayanamsa, *t).ok()).into_iter().collect::<Option<Vec<Sky>>>()?;
    let mut skies = skies.into_iter();
    let base_sky: Vec<Sky> = skies.by_ref().take(base.len()).collect();
    let inside_sky: Vec<Vec<Sky>> = inside.iter().map(|v| skies.by_ref().take(v.len()).collect()).collect();
    Some(Prepared { base, inside, base_sky, inside_sky })
}

/// One candidate chart's witnesses on every answer.
pub(crate) struct Judged {
    /// Per answer, each witness's signed distance (see `Reading::zs`), already scaled by
    /// the witness's trust (the KP promise).
    pub(crate) zs: Vec<[Option<f64>; NW]>,
    /// Rising sign.
    pub(crate) lagna_sign: u8,
    /// Moon's star.
    pub(crate) moon_star: usize,
}

/// Judge the chart born at `b` against the answers.
#[allow(clippy::too_many_arguments)]
pub(crate) fn judge(spk: &Spk, eop: Option<&EopTable>, obs: &Observer, system: HouseSystem, ayanamsa: Ayanamsa, b: Instant, prep: &Prepared, answers: &[Answer]) -> Option<Judged> {
    let natal = Natal::compute(spk, eop, b, obs, system, ayanamsa).ok()?;
    let (asc, _) = ascendant(spk, eop, b, obs, ayanamsa)?;
    let r = Reading::new(&natal, asc);
    let read = |s: &Sky, t: Instant| match rebased(s, &natal) {
        Some(own) => r.values(&own, t),
        None => r.values(s, t),
    };
    let base: Vec<[[f64; 9]; NW]> = prep.base.iter().zip(&prep.base_sky).map(|(t, s)| read(s, *t)).collect();
    let zs = answers
        .iter()
        .zip(prep.inside.iter().zip(&prep.inside_sky))
        .map(|(a, (ts, ss))| {
            let ins: Vec<[[f64; 9]; NW]> = ts.iter().zip(ss).map(|(t, s)| read(s, *t)).collect();
            let z = Reading::zs(&base, &ins, a.life, a.good);
            std::array::from_fn(|w| z[w].map(|v| v * r.conf(w, a.life)))
        })
        .collect();
    Some(Judged { zs, lagna_sign: natal.facts.lagna_sign, moon_star: natal.janma() })
}

/// The probability a reading gives to the stretch being as it was said, from its
/// log-odds, kept within 2 % of certainty.
pub(crate) fn prob(logit: f64) -> f64 {
    (1.0 / (1.0 + (-logit).exp())).clamp(0.02, 0.98)
}

/// Log-likelihood of a verdict against chance (ln ½): Yes `ln p`, No `ln(1 − p)`, Close
/// `ln(½p + ¼)` (half of a Yes), each less `ln ½`.
pub(crate) fn gain(v: Verdict, p: f64) -> f64 {
    let l = match v {
        Verdict::Yes => p.ln(),
        Verdict::Close => (0.5 * p + 0.25).ln(),
        Verdict::No => (1.0 - p).ln(),
    };
    l - 0.5f64.ln()
}

/// What the answers say about the birth time.
#[derive(Clone, Debug, PartialEq)]
pub struct BirthRefine {
    /// Every candidate, best first.
    pub fits: Vec<BirthFit>,
    /// Evidence that the answers carry information about the birth time at all: log10 of
    /// the Bayes factor of the pooled reading (averaged over the prior on the window)
    /// against answers given at random. Each answer's pooled prediction is a proper
    /// distribution, so under random answering the factor's expectation is 1 and, by
    /// Markov's inequality, a factor of 20 or more turns up by chance at most one time in
    /// twenty. Kass & Raftery 1995 (*JASA* 90, 773): above 20 (log10 1.3) is "strong".
    pub log10_bayes: f64,
    /// The moments in the window where the chart changes, in time order.
    pub boundaries: Vec<Boundary>,
    /// The boundaries on either side of the best time (the window's edge where none):
    /// the stretch of birth times over which everything the boundaries mark holds.
    pub bracket: Option<(Instant, Instant)>,
    /// D60 rising changes inside the bracket: a flag only.
    pub d60_in_bracket: usize,
}

/// A Bayes factor of at least 20 (log10 1.3), Kass & Raftery's "strong", before a shift
/// is suggested.
pub const SUGGEST_LOG10_BAYES: f64 = 1.3;

impl BirthRefine {
    /// The time to suggest: [`suggested_shift`]'s rule, and the answers must carry strong
    /// evidence at all ([`SUGGEST_LOG10_BAYES`]), so answers given at random do not move
    /// the birth time.
    pub fn suggested(&self, answered: usize) -> Option<BirthFit> {
        if self.log10_bayes < SUGGEST_LOG10_BAYES {
            return None;
        }
        suggested_shift(&self.fits, answered)
    }
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
    refine_birth_detail(spk, eop, obs, system, ayanamsa, birth, window_min, step_min, answers, false).fits
}

/// [`refine_birth`] with the evidence and, when `with_boundaries`, the moments in the
/// window where the chart changes.
///
/// Each candidate is judged by every witness of [`crate::witness`]; per answer the
/// witnesses' signed distances are pooled as `logit p = 1.6 Σ wᵢ zᵢ` and the answer
/// scores `ln p` for a Yes, `ln(1 − p)` for a No and `ln(½p + ¼)` for a Close. A mild
/// Gaussian prior (σ = the window, at least four minutes) keeps the recorded time unless
/// the answers clearly prefer another.
#[allow(clippy::too_many_arguments)]
pub fn refine_birth_detail(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    birth: Instant,
    window_min: f64,
    step_min: f64,
    answers: &[Answer],
    with_boundaries: bool,
) -> BirthRefine {
    let empty = BirthRefine { fits: Vec::new(), log10_bayes: 0.0, boundaries: Vec::new(), bracket: None, d60_in_bracket: 0 };
    if answers.is_empty() {
        return empty;
    }
    let window_min = window_min.clamp(0.0, 180.0);
    // At most 61 candidates: the step widens with the window.
    let step = step_min.max(window_min / 30.0).max(0.5);
    let n_side = (window_min / step).floor() as i64;
    let sigma = window_min.max(4.0);
    let ks: Vec<i64> = (-n_side..=n_side).collect();
    let (scored, bounds) = std::thread::scope(|s| {
        let bounds = with_boundaries.then(|| s.spawn(|| boundaries(spk, eop, obs, ayanamsa, birth, window_min)));
        let scored: Vec<(f64, f64, BirthFit)> = match prepare(spk, eop, obs, system, ayanamsa, birth, answers) {
            None => Vec::new(),
            Some(prep) => par_map(&ks, |&k| {
                let shift = k as f64 * step;
                let b = birth.add_ns((shift * 60.0 * NS_PER_SEC as f64) as i128);
                let j = judge(spk, eop, obs, system, ayanamsa, b, &prep, answers)?;
                let mut like = 0.0;
                let mut explained = 0;
                let mut by_witness = [0.0; NW];
                for (a, z) in answers.iter().zip(&j.zs) {
                    let logit: f64 = WITNESSES.iter().map(|w| w.weight() * z[w.index()].unwrap_or(0.0)).sum::<f64>() * SLOPE;
                    like += gain(a.verdict, prob(logit));
                    for (w, zw) in z.iter().enumerate() {
                        if let Some(zw) = zw {
                            by_witness[w] += gain(a.verdict, prob(SLOPE * zw));
                        }
                    }
                    // The slow score's own thresholds (z > 0.3) on the pooled scale.
                    let hit = match a.verdict {
                        Verdict::Yes => logit > 0.48,
                        Verdict::Close => logit > 0.0,
                        Verdict::No => logit < 0.48,
                    };
                    if hit {
                        explained += 1;
                    }
                }
                let prior = -0.5 * (shift / sigma) * (shift / sigma);
                Some((prior, like, BirthFit { birth: b, shift_min: shift, prob: 0.0, explained, lagna_sign: j.lagna_sign, moon_star: j.moon_star, by_witness }))
            })
            .into_iter()
            .flatten()
            .collect(),
        };
        (scored, bounds.and_then(|h| h.join().ok()))
    });
    if scored.is_empty() {
        return empty;
    }
    let lse = |v: Vec<f64>| -> f64 {
        let top = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        top + v.iter().map(|x| (x - top).exp()).sum::<f64>().ln()
    };
    let log_prior_norm = lse(scored.iter().map(|x| x.0).collect());
    let log_post_norm = lse(scored.iter().map(|x| x.0 + x.1).collect());
    let log_bf = log_post_norm - log_prior_norm;
    let mut fits: Vec<BirthFit> = scored
        .into_iter()
        .map(|(p, l, mut f)| {
            f.prob = (p + l - log_post_norm).exp();
            f
        })
        .collect();
    fits.sort_by(|a, b| b.prob.partial_cmp(&a.prob).unwrap_or(std::cmp::Ordering::Equal));
    let (boundaries, d60) = bounds.unwrap_or_default();
    let (bracket, d60_in_bracket) = match (with_boundaries, fits.first()) {
        (true, Some(best)) => {
            let edge = |m: f64| birth.add_ns((m * 60.0 * NS_PER_SEC as f64) as i128);
            let lo = boundaries.iter().filter(|b| b.shift_min <= best.shift_min).map(|b| b.at).next_back().unwrap_or(edge(-window_min));
            let hi = boundaries.iter().find(|b| b.shift_min > best.shift_min).map(|b| b.at).unwrap_or(edge(window_min));
            (Some((lo, hi)), d60.iter().filter(|t| **t >= lo && **t < hi).count())
        }
        _ => (None, 0),
    };
    BirthRefine { fits, log10_bayes: log_bf / std::f64::consts::LN_10, boundaries, bracket, d60_in_bracket }
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
        // Answers as a person born 12 minutes later would give them.
        let (answers, r) = simulate(&spk, &obs, &natal, at, 12.0, 30.0, 3);
        let fits = &r.fits;
        let total: f64 = fits.iter().map(|f| f.prob).sum();
        assert!((total - 1.0).abs() < 1e-6);
        // The answers must move the belief toward the truth, not away from it.
        let mean: f64 = fits.iter().map(|f| f.prob * f.shift_min).sum();
        assert!(mean > 0.0, "belief moved the wrong way: mean shift {mean:+.1}");
        assert!(r.log10_bayes > 0.0, "answers from a real chart carry evidence: {}", r.log10_bayes);
        let _ = answers;
    }

    /// Answers as a person born `shift` minutes from the recorded time would give them,
    /// to three rounds of questions asked over ±`window` minutes: does the true chart
    /// mark each stretch the same way? Then the refined belief over the window.
    fn simulate(spk: &Spk, obs: &Observer, natal: &Natal, at: Instant, shift: f64, window: f64, rounds: usize) -> (Vec<Answer>, BirthRefine) {
        let truth = natal.birth.add_ns((shift * 60.0) as i128 * NS_PER_SEC);
        let mut asked = Vec::new();
        let mut answers = Vec::new();
        for round in 0..rounds {
            let qs = past_reveal(spk, None, natal, obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, window, &asked, 4, round > 0).unwrap();
            for q in qs {
                asked.push((q.from, q.to));
                let fits = refine_birth(spk, None, obs, HouseSystem::Placidus, Ayanamsa::Lahiri, truth, 0.0, 1.0, &[Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: Verdict::Yes }]);
                let yes = fits.first().map(|f| f.explained == 1).unwrap_or(false);
                answers.push(Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: if yes { Verdict::Yes } else { Verdict::No } });
            }
        }
        let t = std::time::Instant::now();
        let r = refine_birth_detail(spk, None, obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, window, 2.0, &answers, true);
        let el = t.elapsed();
        let yes = answers.iter().filter(|a| a.verdict == Verdict::Yes).count();
        let mean: f64 = r.fits.iter().map(|f| f.prob * f.shift_min).sum();
        eprintln!(
            "truth {shift:+} min, ±{window}: {} answers ({yes} yes) over {} times in {el:?}; best {:+.0} min ({:.0}%), mean {mean:+.1}, log10 BF {:.2}, {} boundaries, suggested {:?}",
            answers.len(),
            r.fits.len(),
            r.fits[0].shift_min,
            r.fits[0].prob * 100.0,
            r.log10_bayes,
            r.boundaries.len(),
            r.suggested(answers.len()).map(|f| f.shift_min)
        );
        (answers, r)
    }

    #[test]
    fn a_birth_twenty_minutes_early_is_found_in_an_hour_wide_window() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let (answers, r) = simulate(&spk, &obs, &natal, at, -20.0, 60.0, 6);
        assert_eq!(r.fits.len(), 61);
        // Pinned 2026-10-20: 24 answers, mean shift −6.2 min, belief earlier 0.76 against
        // later 0.21, best −10 min, a suggestion of −10 min, log10 BF 3.7.
        let mean: f64 = r.fits.iter().map(|f| f.prob * f.shift_min).sum();
        assert!(mean < -3.0, "belief should move toward −20: mean shift {mean:+.1}");
        let earlier: f64 = r.fits.iter().filter(|f| f.shift_min < -1.0).map(|f| f.prob).sum();
        let later: f64 = r.fits.iter().filter(|f| f.shift_min > 1.0).map(|f| f.prob).sum();
        assert!(earlier > 2.0 * later, "earlier {earlier:.2} vs later {later:.2}");
        assert!(r.log10_bayes > SUGGEST_LOG10_BAYES, "{}", r.log10_bayes);
        if let Some(s) = r.suggested(answers.len()) {
            assert!(s.shift_min < 0.0, "suggested the wrong side: {s:?}");
        }
        // The boundaries are in time order, inside the window, and bracket the best time.
        assert!(!r.boundaries.is_empty());
        for w in r.boundaries.windows(2) {
            assert!(w[0].shift_min <= w[1].shift_min);
        }
        for b in &r.boundaries {
            assert!(b.shift_min.abs() <= 60.0 && b.before != b.after, "{b:?}");
        }
        let (lo, hi) = r.bracket.expect("a bracket");
        assert!(lo <= r.fits[0].birth && r.fits[0].birth < hi);
    }

    #[test]
    fn random_answers_never_move_the_birth_time() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let mut asked = Vec::new();
        let mut qs = Vec::new();
        for round in 0..3 {
            for q in past_reveal(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 60.0, &asked, 4, round > 0).unwrap() {
                asked.push((q.from, q.to));
                qs.push(q);
            }
        }
        // Several fixed seeds of a 64-bit LCG (Knuth, MMIX constants).
        for seed in [1u64, 7, 42, 2026, 99991] {
            let mut x = seed;
            let answers: Vec<Answer> = qs
                .iter()
                .map(|q| {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let verdict = match (x >> 33) % 5 {
                        0 | 1 => Verdict::Yes,
                        2 | 3 => Verdict::No,
                        _ => Verdict::Close,
                    };
                    Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict }
                })
                .collect();
            let r = refine_birth_detail(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, 60.0, 2.0, &answers, false);
            let s = r.suggested(answers.len());
            eprintln!("seed {seed}: best {:+.0} min ({:.0}%), log10 BF {:.2}, suggested {:?}", r.fits[0].shift_min, r.fits[0].prob * 100.0, r.log10_bayes, s.as_ref().map(|f| f.shift_min));
            assert!(s.is_none(), "random answers suggested {s:?}");
        }
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
