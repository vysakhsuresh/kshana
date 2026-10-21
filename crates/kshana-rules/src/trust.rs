//! The trust loop: what the person's own answers do to the forecast, and which of the
//! forecast's dated claims are firm enough to seal.
//!
//! **Learning feeds the forecast.** [`crate::learn`] weighs the techniques by how well
//! each would have predicted the person's answered past. [`Tilt::from_learned`] turns
//! those weights into a tilt of the forecast: a technique that has earned more than the
//! equal share, and beats chance on the answers, gets a say `ωₜ = s · max(0, k·wₜ − 1)`
//! (k = six techniques, so `k·wₜ − 1` is how far above the equal share it stands), where
//! `s = n / (n + 20)` shrinks the tilt toward the usual forecast with a prior worth
//! twenty answers, on top of the learner's own shrinkage (λ = n / (n + 10), so a
//! technique's excess is at most `5λ`). Below [`FORECAST_MIN_ANSWERS`] answers there is
//! no tilt at all, and the tilts together never outweigh the usual reading
//! ([`MAX_TILT`]): a few answers cannot swing a forecast.
//!
//! [`personal_slow_series`] then pools, per life and per month, each tilted technique's
//! distance from its own average (in standard deviations, over the series and the two
//! years around its start) with the usual slow score's, `ẑ = (z_slow + Σ ωₜ cₜ zₜ) / (1 +
//! Σ ωₜ cₜ)` (cₜ the technique's trust, the KP promise for KP and 1 for the others), the
//! same logarithmic pooling the birth-time witnesses use ([`crate::witness`]), and puts
//! it back on the slow score's own scale. With no tilt it returns [`slow_series`]
//! itself, so the usual forecast is reproduced exactly.
//!
//! **Firm predictions.** [`firm_predictions`] lists the dated claims worth sealing: the
//! stretch of the coming weeks ([`crate::reveal::seal_pick`]) and the forecast's own
//! clear openings and pressures that start after today (strength at least "clear",
//! within six months), each with the probability `logistic(1.6 · tanh z)` its distance
//! from the life's average gives, its statement, and the plain question that resolves it.

use crate::forecast::{sky, slow_series, ForecastError, Life, Natal};
use crate::future::{future, future_from_series, FutureEvent};
use crate::lang::{date, month_year};
use crate::learn::{Technique, TECHNIQUES};
use crate::pastcheck::rest;
use crate::plain::Lang;
use crate::reveal::seal_pick;
use crate::witness::{ascendant, Reading, Witness, NW, SLOPE};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

/// Fewest answers before the forecast follows the person's own weights.
pub const FORECAST_MIN_ANSWERS: usize = 8;
/// Answers' worth of prior toward the usual forecast.
pub const FORECAST_PRIOR: f64 = 20.0;
/// Most say the tilted techniques have together, against the usual reading's one.
pub const MAX_TILT: f64 = 1.0;

const MONTH_NS: i128 = (30.4375 * NS_PER_DAY as f64) as i128;

/// The witness a technique is read through in the forecast; the slow reading (on either
/// scale) is the forecast itself.
fn witness_of(t: Technique) -> Option<Witness> {
    match t {
        Technique::Slow | Technique::SlowOtherAyanamsa => None,
        Technique::Vimshottari => Some(Witness::Vimshottari),
        Technique::Yogini => Some(Witness::Yogini),
        Technique::Chara => Some(Witness::Chara),
        Technique::Kp => Some(Witness::Kp),
    }
}

/// How one technique stood on the person's answers (from [`crate::learn::learn`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LearnedWeight {
    /// Which technique.
    pub technique: Technique,
    /// Its learned weight, the six summing to 1.
    pub weight: f64,
    /// Brier skill against chance.
    pub skill: f64,
    /// Mean log gain over chance.
    pub log_gain: f64,
}

/// The tilt of one person's forecast toward the techniques that fit their past.
#[derive(Clone, Debug, PartialEq)]
pub struct Tilt {
    /// Answers the weights rest on.
    pub answers: usize,
    /// Say of each witness, by [`Witness::index`]; 0 for the slow reading.
    pub omega: [f64; NW],
}

impl Default for Tilt {
    fn default() -> Self {
        Tilt { answers: 0, omega: [0.0; NW] }
    }
}

impl Tilt {
    /// The tilt from the learned weights over `answers` answers.
    pub fn from_learned(answers: usize, learned: &[LearnedWeight]) -> Tilt {
        let mut omega = [0.0; NW];
        if answers < FORECAST_MIN_ANSWERS {
            return Tilt { answers, omega };
        }
        let k = TECHNIQUES.len() as f64;
        let s = answers as f64 / (answers as f64 + FORECAST_PRIOR);
        for l in learned {
            let Some(w) = witness_of(l.technique) else { continue };
            if !(l.weight.is_finite() && l.skill > 0.0 && l.log_gain > 0.0) {
                continue;
            }
            omega[w.index()] += s * (k * l.weight - 1.0).max(0.0);
        }
        let total: f64 = omega.iter().sum();
        if total > MAX_TILT {
            for o in omega.iter_mut() {
                *o *= MAX_TILT / total;
            }
        }
        Tilt { answers, omega }
    }

    /// Whether the forecast moves at all.
    pub fn is_active(&self) -> bool {
        self.omega.iter().any(|o| *o > 0.0)
    }

    /// The technique with the most say, if any.
    pub fn leading(&self) -> Option<Technique> {
        TECHNIQUES
            .iter()
            .copied()
            .filter_map(|t| witness_of(t).map(|w| (t, self.omega[w.index()])))
            .filter(|x| x.1 > 0.0)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|x| x.0)
    }

    /// Share of the say of each technique, the usual reading included, summing to 1.
    pub fn shares(&self) -> Vec<(Technique, f64)> {
        let total = 1.0 + self.omega.iter().sum::<f64>();
        TECHNIQUES
            .iter()
            .filter(|t| **t != Technique::SlowOtherAyanamsa)
            .map(|&t| (t, witness_of(t).map(|w| self.omega[w.index()]).unwrap_or(1.0) / total))
            .collect()
    }

    /// One plain sentence on what the answers do to the forecast.
    pub fn plain(&self, lang: Lang) -> String {
        match (self.answers, self.leading()) {
            (n, _) if n < FORECAST_MIN_ANSWERS => match lang {
                Lang::En => format!("Your forecast uses the usual reading. After {FORECAST_MIN_ANSWERS} answers about your past it starts to follow what fits you; {n} so far."),
                Lang::Ml => format!("നിങ്ങളുടെ പ്രവചനം സാധാരണ വായന ഉപയോഗിക്കുന്നു. ഭൂതകാലത്തെക്കുറിച്ച് {FORECAST_MIN_ANSWERS} ഉത്തരങ്ങൾക്ക് ശേഷം അത് നിങ്ങൾക്ക് ചേരുന്നതിനെ പിന്തുടരും; ഇതുവരെ {n}."),
                Lang::Hi => format!("आपका पूर्वानुमान सामान्य पढ़त से बनता है। अतीत के {FORECAST_MIN_ANSWERS} जवाबों के बाद यह उसी पर चलेगा जो आप पर बैठता है; अब तक {n}।"),
            },
            (_, None) => lang
                .pick([
                    "Your forecast uses the usual reading: no other reading has fit your past clearly better.",
                    "നിങ്ങളുടെ പ്രവചനം സാധാരണ വായന ഉപയോഗിക്കുന്നു: മറ്റൊരു വായനയും നിങ്ങളുടെ ഭൂതകാലത്തോട് വ്യക്തമായി കൂടുതൽ ചേർന്നിട്ടില്ല.",
                    "आपका पूर्वानुमान सामान्य पढ़त से बनता है: कोई और पढ़त आपके अतीत से साफ़ बेहतर नहीं बैठी।",
                ])
                .to_string(),
            (_, Some(t)) => {
                let name = t.name(lang);
                match lang {
                    Lang::En => format!("Your forecast now leans a little on {name}, which has fit your past best; the usual reading still has the larger say."),
                    Lang::Ml => format!("നിങ്ങളുടെ പ്രവചനം ഇപ്പോൾ അല്പം {name} എന്നതിനെ ആശ്രയിക്കുന്നു, അതാണ് നിങ്ങളുടെ ഭൂതകാലത്തോട് ഏറ്റവും ചേർന്നത്; സാധാരണ വായനയ്ക്കാണ് ഇപ്പോഴും കൂടുതൽ പങ്ക്."),
                    Lang::Hi => format!("आपका पूर्वानुमान अब थोड़ा {name} पर टिकता है, जो आपके अतीत से सबसे अच्छी बैठी; सामान्य पढ़त का हिस्सा अब भी बड़ा है।"),
                }
            }
        }
    }
}

fn mean_sd(v: &[f64]) -> Option<(f64, f64)> {
    if v.len() < 3 {
        return None;
    }
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt();
    (sd >= 1e-3).then_some((m, sd))
}

/// The monthly slow series of [`slow_series`], pooled with the person's tilt.
/// `birth_obs` is where they were born (for the chart's rising degree), `obs` where the
/// sky is read from, as in [`slow_series`]. With no tilt, [`slow_series`] itself.
#[allow(clippy::too_many_arguments)]
pub fn personal_slow_series(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    birth_obs: &Observer,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    from: Instant,
    step_ns: i128,
    count: usize,
    tilt: &Tilt,
) -> Result<Vec<(Instant, Vec<f64>)>, ForecastError> {
    if !tilt.is_active() || count == 0 {
        return slow_series(spk, eop, natal, obs, ayanamsa, from, step_ns, count);
    }
    let Some((asc, _)) = ascendant(spk, eop, natal.birth, birth_obs, ayanamsa) else {
        return slow_series(spk, eop, natal, obs, ayanamsa, from, step_ns, count);
    };
    let reading = Reading::new(natal, asc);
    let series: Vec<Instant> = (0..count).map(|k| from.add_ns(step_ns * k as i128)).collect();
    // Each technique's own average is taken over the series and the two years around
    // its start, so a short series still has a steady baseline.
    let base: Vec<Instant> = (-6i128..=6).map(|k| from.add_ns(k * 2 * MONTH_NS)).filter(|t| *t > natal.birth).collect();
    let all: Vec<Instant> = series.iter().chain(base.iter()).copied().collect();
    let skies = crate::reveal::par_map(&all, |t| sky(spk, eop, natal, obs, HouseSystem::WholeSign, ayanamsa, *t));
    let mut vals = Vec::with_capacity(all.len());
    for (t, s) in all.iter().zip(skies) {
        vals.push(reading.values(&s?, *t));
    }
    let n = series.len();
    let mut out: Vec<(Instant, Vec<f64>)> = series.iter().map(|t| (*t, vec![0.0; Life::ALL.len()])).collect();
    for (li, &life) in Life::ALL.iter().enumerate() {
        let slow: Vec<f64> = vals.iter().map(|v| v[Witness::Slow.index()][li]).collect();
        let stats = mean_sd(&slow);
        let mut shift = vec![0.0; n];
        let mut say = 0.0;
        if let Some((ms, ss)) = stats {
            for w in crate::witness::WITNESSES {
                let omega = tilt.omega[w.index()] * reading.conf(w.index(), life);
                if omega <= 0.0 {
                    continue;
                }
                let col: Vec<f64> = vals.iter().map(|v| v[w.index()][li]).collect();
                let fin: Vec<f64> = col.iter().copied().filter(|x| x.is_finite()).collect();
                let Some((m, sd)) = mean_sd(&fin) else { continue };
                say += omega;
                for i in 0..n {
                    // Where the technique has nothing to say it pulls nowhere.
                    let zs = (slow[i] - ms) / ss;
                    let z = if col[i].is_finite() { (col[i] - m) / sd } else { zs };
                    shift[i] += omega * (z - zs);
                }
            }
            for (i, o) in out.iter_mut().enumerate() {
                o.1[li] = if say > 0.0 { (slow[i] + ss * shift[i] / (1.0 + say)).clamp(0.0, 1.0) } else { slow[i] };
            }
        } else {
            for (i, o) in out.iter_mut().enumerate() {
                o.1[li] = slow[i];
            }
        }
    }
    Ok(out)
}

/// [`future`] with the person's tilt; with no tilt, [`future`] itself.
#[allow(clippy::too_many_arguments)]
pub fn personal_future(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    birth_obs: &Observer,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    at: Instant,
    months: usize,
    focus: &[Life],
    lang: Lang,
    tilt: &Tilt,
) -> Result<Vec<FutureEvent>, ForecastError> {
    if !tilt.is_active() {
        return future(spk, eop, natal, obs, ayanamsa, at, months, focus, lang);
    }
    let months = months.clamp(3, 60);
    let slow = personal_slow_series(spk, eop, natal, birth_obs, obs, ayanamsa, at, MONTH_NS, months + 1, tilt)?;
    future_from_series(spk, natal, obs, ayanamsa, at, &slow, focus, lang)
}

/// The plain question that resolves a claim about `life` over `[from, to]`: days for a
/// window of six weeks or less, months for a longer one.
pub fn claim_question(lang: Lang, life: Life, good: bool, from: Instant, to: Instant, lon: f64) -> String {
    let short = to.ns_since(from) <= 45 * NS_PER_DAY;
    let (a, b) = if short { (date(lang, from, lon), date(lang, to, lon)) } else { (month_year(lang, from, lon), month_year(lang, to, lon)) };
    let r = lang.pick(rest(life, good));
    if a == b {
        return match lang {
            Lang::En => format!("In {a}, {r}"),
            Lang::Ml => format!("{a}-ൽ, {r}"),
            Lang::Hi => format!("{a} में, {r}"),
        };
    }
    match lang {
        Lang::En => format!("Between {a} and {b}, {r}"),
        Lang::Ml => format!("{a} മുതൽ {b} വരെ, {r}"),
        Lang::Hi => format!("{a} से {b} के बीच, {r}"),
    }
}

/// A dated claim worth sealing.
#[derive(Clone, Debug, PartialEq)]
pub struct Firm {
    /// "weeks" (the stretch of the coming weeks) or "forecast" (an opening or pressure).
    pub source: &'static str,
    /// The life.
    pub life: Life,
    /// "opening" or "pressure".
    pub kind: &'static str,
    /// True for an opening.
    pub good: bool,
    /// Window start.
    pub from: Instant,
    /// Window end.
    pub to: Instant,
    /// Probability that it comes as said.
    pub probability: f64,
    /// The prediction.
    pub statement: String,
    /// The question that resolves it.
    pub question: String,
}

/// Probability of a stretch said in the direction its distance `z` (standard
/// deviations, signed toward what is said) points.
pub fn firm_probability(z: f64) -> f64 {
    (1.0 / (1.0 + (-SLOPE * z.tanh()).exp())).clamp(0.02, 0.98)
}

/// Fewest days ahead a forecast window must open to be sealed.
pub const FIRM_LEAD_DAYS: i128 = 2;
/// Months of the forecast searched for firm windows.
pub const FIRM_MONTHS: usize = 6;
/// Weakest forecast strength sealed: "a clear opening", "it presses moderately".
pub const FIRM_STRENGTH: f64 = 0.33;

/// The dated claims worth sealing at `at`, soonest first.
#[allow(clippy::too_many_arguments)]
pub fn firm_predictions(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    birth_obs: &Observer,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    lang: Lang,
    at: Instant,
    tilt: &Tilt,
) -> Result<Vec<Firm>, ForecastError> {
    let lon = obs.longitude_deg;
    let mut out = Vec::new();
    if let Some(p) = seal_pick(spk, eop, natal, birth_obs, ayanamsa, lang, at)? {
        let z = if p.good { p.strength } else { p.strength / 0.6 };
        out.push(Firm {
            source: "weeks",
            life: p.life,
            kind: if p.good { "opening" } else { "pressure" },
            good: p.good,
            from: p.from,
            to: p.to,
            probability: firm_probability(z),
            statement: p.statement,
            question: p.question,
        });
    }
    let slow = personal_slow_series(spk, eop, natal, birth_obs, obs, ayanamsa, at, MONTH_NS, FIRM_MONTHS + 1, tilt)?;
    // Each life's own level over two years around today sets what "stands out" means.
    let base = slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(-12 * MONTH_NS), 2 * MONTH_NS, 13)?;
    let ev = future_from_series(spk, natal, obs, ayanamsa, at, &slow, Life::ALL.as_slice(), lang)?;
    let lead = at.add_ns(FIRM_LEAD_DAYS * NS_PER_DAY);
    for e in ev {
        let Some(life) = e.life else { continue };
        let good = match e.kind {
            "opening" => true,
            "pressure" => false,
            _ => continue,
        };
        if e.strength < FIRM_STRENGTH || e.from < lead {
            continue;
        }
        let li = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        let col: Vec<f64> = base.iter().chain(slow.iter()).map(|s| s.1[li]).collect();
        let inside: Vec<f64> = slow.iter().filter(|s| s.0 >= e.from && s.0 < e.to).map(|s| s.1[li]).collect();
        let z = match (mean_sd(&col), inside.is_empty()) {
            (Some((m, sd)), false) => (inside.iter().sum::<f64>() / inside.len() as f64 - m) / sd * if good { 1.0 } else { -1.0 },
            _ => e.strength,
        };
        out.push(Firm {
            source: "forecast",
            life,
            kind: e.kind,
            good,
            from: e.from,
            to: e.to,
            probability: firm_probability(z.max(0.0)),
            statement: e.headline,
            question: claim_question(lang, life, good, e.from, e.to, lon),
        });
    }
    out.sort_by_key(|f| f.from);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learn::learn;
    use crate::plain::jargon_in;
    use crate::reveal::{judge, past_reveal, prepare, Answer, Verdict};
    use kshana_time::Utc;

    fn lw(t: Technique, weight: f64, skill: f64, gain: f64) -> LearnedWeight {
        LearnedWeight { technique: t, weight, skill, log_gain: gain }
    }

    #[test]
    fn the_tilt_waits_for_answers_and_never_outweighs_the_usual_reading() {
        let strong = [lw(Technique::Slow, 0.1, 0.0, 0.0), lw(Technique::Yogini, 0.6, 0.3, 0.2), lw(Technique::Kp, 0.1, 0.1, 0.1), lw(Technique::Chara, 0.2, -0.1, -0.1)];
        // Too few answers: nothing moves.
        let few = Tilt::from_learned(FORECAST_MIN_ANSWERS - 1, &strong);
        assert!(!few.is_active());
        assert_eq!(few.leading(), None);
        // Enough: the technique that fit gets the say, one that lost to chance none.
        let t = Tilt::from_learned(12, &strong);
        let s = 12.0 / 32.0;
        let want_yogini = s * (6.0 * 0.6 - 1.0);
        let total = want_yogini;
        assert!(total <= MAX_TILT + 1e-12);
        assert!((t.omega[Witness::Yogini.index()] - want_yogini).abs() < 1e-12, "{t:?}");
        assert_eq!(t.omega[Witness::Kp.index()], 0.0, "below the equal share");
        assert_eq!(t.omega[Witness::Chara.index()], 0.0, "worse than chance");
        assert_eq!(t.omega[Witness::Slow.index()], 0.0);
        assert_eq!(t.leading(), Some(Technique::Yogini));
        assert!((t.shares().iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-12);
        // Many answers cannot push past the cap.
        let many = Tilt::from_learned(500, &[lw(Technique::Yogini, 0.9, 0.5, 0.4), lw(Technique::Kp, 0.05, 0.2, 0.1)]);
        assert!((many.omega.iter().sum::<f64>() - MAX_TILT).abs() < 1e-12);
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for x in [&few, &t, &many, &Tilt::from_learned(20, &[])] {
                let p = x.plain(lang);
                assert_eq!(jargon_in(lang, &p), None, "{p}");
            }
        }
    }

    fn setup() -> Option<(Spk, Observer, Natal, Instant)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        Some((spk, obs, natal, at))
    }

    #[test]
    fn default_weights_reproduce_the_usual_forecast_exactly() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let none = Tilt::default();
        let few = Tilt::from_learned(3, &[lw(Technique::Yogini, 0.9, 0.5, 0.4)]);
        for t in [&none, &few] {
            let a = personal_slow_series(&spk, None, &natal, &obs, &obs, Ayanamsa::Lahiri, at, MONTH_NS, 13, t).unwrap();
            let b = slow_series(&spk, None, &natal, &obs, Ayanamsa::Lahiri, at, MONTH_NS, 13).unwrap();
            assert_eq!(a, b);
            let fa = personal_future(&spk, None, &natal, &obs, &obs, Ayanamsa::Lahiri, at, 12, Life::ALL.as_slice(), Lang::En, t).unwrap();
            let fb = future(&spk, None, &natal, &obs, Ayanamsa::Lahiri, at, 12, Life::ALL.as_slice(), Lang::En).unwrap();
            assert_eq!(fa, fb);
        }
    }

    #[test]
    fn consistent_answers_raise_the_matching_reading_in_the_forecast() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let (hs, ay) = (HouseSystem::Placidus, Ayanamsa::Lahiri);
        let birth = natal.birth;
        let mut asked = Vec::new();
        let mut qs = Vec::new();
        for round in 0..3 {
            for q in past_reveal(&spk, None, &natal, &obs, hs, ay, Lang::En, at, 30.0, &asked, 4, round > 0).unwrap() {
                asked.push((q.from, q.to));
                qs.push(q);
            }
        }
        // A person whose life follows the short-cycle (Yogini) reading.
        let probe: Vec<Answer> = qs.iter().map(|q| Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: Verdict::Yes }).collect();
        let prep = prepare(&spk, None, &obs, hs, ay, birth, &probe).unwrap();
        let j = judge(&spk, None, &obs, hs, ay, birth, &prep, &probe).unwrap();
        let answers: Vec<Answer> =
            probe.iter().zip(&j.zs).map(|(a, z)| Answer { verdict: if z[Witness::Yogini.index()].unwrap_or(0.0) > 0.0 { Verdict::Yes } else { Verdict::No }, ..*a }).collect();
        let l = learn(&spk, None, &obs, hs, ay, birth, &answers, Lang::En);
        let learned: Vec<LearnedWeight> = l.techniques.iter().map(|s| lw(s.technique, s.weight, s.skill, s.log_gain)).collect();
        let tilt = Tilt::from_learned(l.answers, &learned);
        assert!(l.answers >= FORECAST_MIN_ANSWERS, "{}", l.answers);
        assert_eq!(tilt.leading(), Some(Technique::Yogini), "{tilt:?}");
        let yo = tilt.omega[Witness::Yogini.index()];
        assert!(yo > 0.0 && tilt.omega.iter().all(|o| *o <= yo));
        assert!(tilt.omega.iter().sum::<f64>() <= MAX_TILT + 1e-12);
        eprintln!("{} | {:?}", tilt.plain(Lang::En), tilt.omega);

        // The personal series moves toward the Yogini reading: its change from the usual
        // series correlates positively with the Yogini reading's distance from its mean.
        let n = 24;
        let usual = slow_series(&spk, None, &natal, &obs, ay, at, MONTH_NS, n).unwrap();
        let mine = personal_slow_series(&spk, None, &natal, &obs, &obs, ay, at, MONTH_NS, n, &tilt).unwrap();
        assert_ne!(usual, mine);
        let (asc, _) = ascendant(&spk, None, birth, &obs, ay).unwrap();
        let r = Reading::new(&natal, asc);
        let mut agree = 0.0;
        let mut moved = 0;
        for li in 0..Life::ALL.len() {
            let yog: Vec<f64> = usual.iter().map(|(t, _)| r.values(&sky(&spk, None, &natal, &obs, HouseSystem::WholeSign, ay, *t).unwrap(), *t)[Witness::Yogini.index()][li]).collect();
            if yog.iter().any(|x| !x.is_finite()) {
                continue;
            }
            let my = yog.iter().sum::<f64>() / n as f64;
            for i in 0..n {
                let d = mine[i].1[li] - usual[i].1[li];
                if d.abs() > 1e-9 {
                    moved += 1;
                }
                agree += d * (yog[i] - my);
            }
        }
        assert!(moved > 0);
        assert!(agree > 0.0, "the tilt should move the forecast toward the reading that fit: {agree}");
        // The tilt stays modest: no month moves by more than the life's spread.
        for i in 0..n {
            for li in 0..Life::ALL.len() {
                assert!((mine[i].1[li] - usual[i].1[li]).abs() < 0.25);
            }
        }
    }

    #[test]
    fn firm_predictions_open_later_and_ask_plainly() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            let f = firm_predictions(&spk, None, &natal, &obs, &obs, Ayanamsa::Lahiri, lang, at, &Tilt::default()).unwrap();
            assert!(!f.is_empty());
            for x in &f {
                assert!(x.from > at && x.to > x.from, "{x:?}");
                assert!((0.02..=0.98).contains(&x.probability));
                assert_eq!(jargon_in(lang, &x.statement), None, "{}", x.statement);
                assert_eq!(jargon_in(lang, &x.question), None, "{}", x.question);
                assert!(!x.question.is_empty() && !x.statement.is_empty());
            }
            if lang == Lang::En {
                for x in &f {
                    eprintln!("{} {} {:?} p={:.2} | {} | {}", x.source, x.kind, x.life, x.probability, x.statement, x.question);
                }
            }
        }
        // The question uses days for a short window.
        let q = claim_question(Lang::En, Life::Money, true, Utc::new(2026, 11, 14, 6, 0, 0, 0).unwrap().to_instant().unwrap(), Utc::new(2026, 11, 19, 6, 0, 0, 0).unwrap().to_instant().unwrap(), 76.27);
        assert_eq!(q, "Between 14 Nov 2026 and 19 Nov 2026, did money come in well: a raise, a gain or a good deal?");
    }
}
