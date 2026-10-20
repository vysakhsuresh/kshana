//! Learning which technique fits one person's past.
//!
//! The person's own answered stretches (the past told first, and every sealed stretch
//! once it has been opened and answered) are the evidence. Each technique reads every
//! answered stretch at the recorded birth time, alone, exactly as the birth-time
//! rectification's witnesses do ([`crate::witness`]): its distance from the life's own
//! average becomes a probability `p = logistic(1.6 z)` that the stretch was as said,
//! and the answer (Yes = 1, Close = ½, No = 0) scores it.
//!
//! * Brier score `mean (p − y)²` (Brier 1950, *Mon. Weather Rev.* 78, 1), against the
//!   chance forecaster (½ every time: ¼ per Yes or No, 0 per Close), and the skill
//!   `1 − Brier / chance` (the Brier skill score).
//! * Log score: mean `ln p(y) − ln ½` per answer (Good 1952, *J. R. Statist. Soc. B*
//!   14, 107), zero for chance.
//! * Weights: a tempered Bayesian model average, `wᵢ ∝ exp(η Σ log-gainᵢ)` with η = ½
//!   (the "safe Bayes" learning rate for misspecified, overlapping models; Grünwald
//!   2012, *ALT* LNCS 7568, 169), shrunk toward equal weights by `λ = n / (n + 10)`, so
//!   with few answers the weights stay near equal and ten answers carry half the say.
//!
//! A selection effect to keep in mind: the past is asked where the whole-picture reading
//! stands out most, so that reading makes the boldest claims on the asked stretches and
//! is tested hardest; the others are near chance where they have little to say.

use crate::plain::Lang;
use crate::reveal::{gain, judge, prepare, prob, Answer, Verdict};
use crate::witness::{Witness, SLOPE};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::Ayanamsa;

/// The safe-Bayes learning rate of the weights.
pub const ETA: f64 = 0.5;
/// Answers at which the data carry half the say in the weights.
pub const PRIOR_ANSWERS: f64 = 10.0;
/// Fewest answers before one technique is named as fitting best.
pub const MIN_ANSWERS: usize = 5;

/// One technique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Technique {
    /// The forecast's slow score (Parashari: birth chart, Vimshottari, slow movers).
    Slow,
    /// Vimshottari periods alone.
    Vimshottari,
    /// Yogini periods.
    Yogini,
    /// Chara (Jaimini) periods.
    Chara,
    /// KP timing.
    Kp,
    /// The slow score with the other ayanamsa (Krishnamurti for Lahiri and the rest,
    /// Lahiri for Krishnamurti).
    SlowOtherAyanamsa,
}

/// Every technique, in report order.
pub const TECHNIQUES: [Technique; 6] = [Technique::Slow, Technique::Vimshottari, Technique::Yogini, Technique::Chara, Technique::Kp, Technique::SlowOtherAyanamsa];

impl Technique {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Technique::Slow => "slow",
            Technique::Vimshottari => "vimshottari",
            Technique::Yogini => "yogini",
            Technique::Chara => "chara",
            Technique::Kp => "kp",
            Technique::SlowOtherAyanamsa => "slow_other_ayanamsa",
        }
    }

    /// The technique in an astrologer's words (expert view only).
    pub fn expert(self, ayanamsa: Ayanamsa) -> String {
        match self {
            Technique::Slow => format!("Parashari slow score (birth chart, Vimshottari three levels, slow transits), {ayanamsa:?} ayanamsa"),
            Technique::Vimshottari => "Vimshottari periods alone, lords read for KP signification and Parashari quality".into(),
            Technique::Yogini => "Yogini dasha (maha and antar), lords read as for Vimshottari".into(),
            Technique::Chara => "Chara dasha (K. N. Rao), sign and sign lord read for the matter's houses".into(),
            Technique::Kp => "KP timing: dasha, bhukti and antara significators, agreement with the cusp sub-lord and sub-sub-lord, weighted by the cusp promise".into(),
            Technique::SlowOtherAyanamsa => format!("Parashari slow score, {:?} ayanamsa", other_ayanamsa(ayanamsa)),
        }
    }

    /// The technique in plain words.
    pub fn name(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Technique::Slow => ["the whole-picture reading", "മുഴുവൻ ചിത്രം നോക്കുന്ന വായന", "पूरी तस्वीर वाली पढ़त"],
            Technique::Vimshottari => ["the long-period reading", "നീണ്ട കാലഘട്ടങ്ങളുടെ വായന", "लंबे दौरों वाली पढ़त"],
            Technique::Yogini => ["the short-cycle reading", "ചെറിയ ചക്രങ്ങളുടെ വായന", "छोटे चक्रों वाली पढ़त"],
            Technique::Chara => ["the life-chapter reading", "ജീവിത അധ്യായങ്ങളുടെ വായന", "जीवन के अध्यायों वाली पढ़त"],
            Technique::Kp => ["the fine-timing reading", "സൂക്ഷ്മ സമയ വായന", "बारीक समय वाली पढ़त"],
            Technique::SlowOtherAyanamsa => ["the whole-picture reading on a second scale", "രണ്ടാം അളവിലെ മുഴുവൻ ചിത്രം", "दूसरे पैमाने पर पूरी तस्वीर वाली पढ़त"],
        })
    }
}

/// Krishnamurti for every ayanamsa but Krishnamurti, Lahiri for it.
pub fn other_ayanamsa(a: Ayanamsa) -> Ayanamsa {
    if a == Ayanamsa::Krishnamurti {
        Ayanamsa::Lahiri
    } else {
        Ayanamsa::Krishnamurti
    }
}

/// How one technique would have done on the person's answers.
#[derive(Clone, Debug, PartialEq)]
pub struct TechniqueScore {
    /// Which technique.
    pub technique: Technique,
    /// Answers on which it had a reading (the rest count as chance).
    pub read: usize,
    /// Brier score, 0 best; chance is ¼ on Yes and No answers ([`chance_brier`]).
    pub brier: f64,
    /// Mean log score per answer over chance, nats; 0 is chance.
    pub log_gain: f64,
    /// Brier skill against chance, `1 − Brier / chance Brier`; above 0 beats chance.
    pub skill: f64,
    /// Weight for the forecast, all techniques summing to 1.
    pub weight: f64,
}

/// What the answers say about which technique fits.
#[derive(Clone, Debug, PartialEq)]
pub struct Learned {
    /// Every technique, in [`TECHNIQUES`] order.
    pub techniques: Vec<TechniqueScore>,
    /// Answers used.
    pub answers: usize,
    /// The chance forecaster's Brier score on these answers, the line to beat.
    pub chance_brier: f64,
    /// The technique that fits best, once there are enough answers and it clearly leads.
    pub best: Option<Technique>,
    /// One plain sentence for the person.
    pub plain: String,
}

fn label(v: Verdict) -> f64 {
    match v {
        Verdict::Yes => 1.0,
        Verdict::Close => 0.5,
        Verdict::No => 0.0,
    }
}

/// The Brier score of the chance forecaster (½ every time) on these answers: ¼ for a
/// Yes or No, 0 for a Close.
pub fn chance_brier(answers: &[Answer]) -> f64 {
    if answers.is_empty() {
        return 0.25;
    }
    answers.iter().map(|a| (0.5 - label(a.verdict)).powi(2)).sum::<f64>() / answers.len() as f64
}

/// Score `probs[t][a]` (None = no reading) against the answers and weigh the
/// techniques: per technique `(answers read, Brier, mean log gain, total log gain)`, and
/// the weights.
pub fn score(probs: &[Vec<Option<f64>>], answers: &[Answer]) -> (Vec<(usize, f64, f64, f64)>, Vec<f64>) {
    let n = answers.len();
    let k = probs.len().max(1);
    let stats: Vec<(usize, f64, f64, f64)> = probs
        .iter()
        .map(|ps| {
            let mut read = 0;
            let mut brier = 0.0;
            let mut total = 0.0;
            for (p, a) in ps.iter().zip(answers) {
                let q = p.unwrap_or(0.5);
                if p.is_some() {
                    read += 1;
                }
                brier += (q - label(a.verdict)).powi(2);
                total += gain(a.verdict, q);
            }
            let nn = n.max(1) as f64;
            (read, brier / nn, total / nn, total)
        })
        .collect();
    let top = stats.iter().map(|s| ETA * s.3).fold(f64::NEG_INFINITY, f64::max);
    let bma: Vec<f64> = stats.iter().map(|s| (ETA * s.3 - top).exp()).collect();
    let sum: f64 = bma.iter().sum::<f64>().max(1e-300);
    let lambda = n as f64 / (n as f64 + PRIOR_ANSWERS);
    let weights = bma.iter().map(|b| lambda * b / sum + (1.0 - lambda) / k as f64).collect();
    (stats, weights)
}

/// Which technique has fit the person's answered past best, with weights the forecast
/// can use. `ayanamsa` is the person's own; the other ayanamsa is compared too.
#[allow(clippy::too_many_arguments)]
pub fn learn(spk: &Spk, eop: Option<&EopTable>, obs: &Observer, system: HouseSystem, ayanamsa: Ayanamsa, birth: Instant, answers: &[Answer], lang: Lang) -> Learned {
    let n = answers.len();
    let witness_of = |t: Technique| match t {
        Technique::Slow | Technique::SlowOtherAyanamsa => Witness::Slow,
        Technique::Vimshottari => Witness::Vimshottari,
        Technique::Yogini => Witness::Yogini,
        Technique::Chara => Witness::Chara,
        Technique::Kp => Witness::Kp,
    };
    let read = |ay: Ayanamsa| -> Option<Vec<[Option<f64>; crate::witness::NW]>> {
        let prep = prepare(spk, eop, obs, system, ay, birth, answers)?;
        Some(judge(spk, eop, obs, system, ay, birth, &prep, answers)?.zs)
    };
    let (own, other) = if n == 0 { (None, None) } else { std::thread::scope(|s| {
        let h = s.spawn(|| read(other_ayanamsa(ayanamsa)));
        (read(ayanamsa), h.join().ok().flatten())
    }) };
    let probs: Vec<Vec<Option<f64>>> = TECHNIQUES
        .iter()
        .map(|&t| {
            let zs = if t == Technique::SlowOtherAyanamsa { &other } else { &own };
            (0..n).map(|a| zs.as_ref().and_then(|z| z[a][witness_of(t).index()]).map(|z| prob(SLOPE * z))).collect()
        })
        .collect();
    let (stats, weights) = score(&probs, answers);
    let chance = chance_brier(answers);
    let techniques: Vec<TechniqueScore> = TECHNIQUES
        .iter()
        .zip(stats.iter().zip(&weights))
        .map(|(&technique, (s, &weight))| TechniqueScore { technique, read: s.0, brier: s.1, log_gain: s.2, skill: if chance > 0.0 { 1.0 - s.1 / chance } else { 0.0 }, weight })
        .collect();
    // Named only with enough answers, a lead over chance, and a weight clearly above
    // the equal share (a fifth more).
    let k = TECHNIQUES.len() as f64;
    let best = techniques
        .iter()
        .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|b| n >= MIN_ANSWERS && b.log_gain > 0.0 && b.skill > 0.0 && b.weight >= 1.2 / k)
        .map(|b| b.technique);
    Learned { plain: plain_line(lang, best, n), techniques, answers: n, chance_brier: chance, best }
}

fn answers_words(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::En, 1) => "1 answer".into(),
        (Lang::En, n) => format!("{n} answers"),
        (Lang::Ml, 1) => "1 ഉത്തരം".into(),
        (Lang::Ml, n) => format!("{n} ഉത്തരങ്ങൾ"),
        (Lang::Hi, 1) => "1 उत्तर".into(),
        (Lang::Hi, n) => format!("{n} उत्तर"),
    }
}

/// The plain sentence for the app.
pub fn plain_line(lang: Lang, best: Option<Technique>, n: usize) -> String {
    let a = answers_words(lang, n);
    match (n, best) {
        (0, _) => lang
            .pick([
                "Answer a few questions about your past and the app will learn which reading fits you.",
                "നിങ്ങളുടെ ഭൂതകാലത്തെക്കുറിച്ചുള്ള ചില ചോദ്യങ്ങൾക്ക് ഉത്തരം നൽകൂ; ഏത് വായനയാണ് നിങ്ങൾക്ക് ചേരുന്നതെന്ന് ആപ്പ് പഠിക്കും.",
                "अपने अतीत के कुछ सवालों के जवाब दीजिए; ऐप सीखेगा कि कौन सी पढ़त आप पर बैठती है।",
            ])
            .to_string(),
        (n, _) if n < MIN_ANSWERS => match lang {
            Lang::En => format!("Too early to tell which reading fits you; {a} so far."),
            Lang::Ml => format!("ഏത് വായനയാണ് ചേരുന്നതെന്ന് പറയാൻ ഇനിയും നേരത്തെയാണ്; ഇതുവരെ {a}."),
            Lang::Hi => format!("अभी यह कहना जल्दी है कि कौन सी पढ़त आप पर बैठती है; अब तक {a}।"),
        },
        (_, None) => match lang {
            Lang::En => format!("No reading fits your past clearly better than the others yet; {a}."),
            Lang::Ml => format!("ഇതുവരെ ഒരു വായനയും മറ്റുള്ളവയേക്കാൾ വ്യക്തമായി ചേരുന്നില്ല; {a}."),
            Lang::Hi => format!("अभी कोई पढ़त बाकियों से साफ़ बेहतर नहीं बैठती; {a}।"),
        },
        (_, Some(t)) => {
            let name = t.name(lang);
            match lang {
                Lang::En => format!("Your past fits {name} best so far; {a}."),
                Lang::Ml => format!("ഇതുവരെ നിങ്ങളുടെ ഭൂതകാലത്തോട് ഏറ്റവും ചേരുന്നത്: {name}; {a}."),
                Lang::Hi => format!("अब तक आपका अतीत {name} से सबसे अच्छा मेल खाता है; {a}।"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::{Life, Natal};
    use crate::plain::jargon_in;
    use crate::reveal::past_reveal;
    use kshana_time::{Utc, NS_PER_DAY};

    fn ans(v: Verdict) -> Answer {
        let t = Instant::j2000();
        Answer { from: t, to: t.add_ns(90 * NS_PER_DAY), life: Life::Love, good: true, verdict: v }
    }

    #[test]
    fn scores_and_weights_follow_the_definitions() {
        // Two techniques on four answers Yes, Yes, No, Close.
        let answers = [ans(Verdict::Yes), ans(Verdict::Yes), ans(Verdict::No), ans(Verdict::Close)];
        let probs = vec![vec![Some(0.8), Some(0.7), Some(0.2), Some(0.5)], vec![None, None, None, None]];
        let (s, w) = score(&probs, &answers);
        // Brier: (0.04 + 0.09 + 0.04 + 0) / 4 = 0.0425; chance on these answers
        // (0.25 + 0.25 + 0.25 + 0) / 4 = 0.1875, a Close being met exactly by ½.
        assert!((s[0].1 - 0.0425).abs() < 1e-12);
        assert!((s[1].1 - 0.1875).abs() < 1e-12);
        // Log gain: ln(0.8/0.5) + ln(0.7/0.5) + ln(0.8/0.5) + ln(0.5/0.5).
        let total = (1.6f64).ln() + (1.4f64).ln() + (1.6f64).ln();
        assert!((s[0].3 - total).abs() < 1e-12 && s[1].3.abs() < 1e-12);
        assert_eq!((s[0].0, s[1].0), (4, 0));
        // λ = 4/14; bma ∝ exp(½ total) vs 1.
        let lambda = 4.0 / 14.0;
        let b = (0.5 * total).exp();
        let w0 = lambda * b / (b + 1.0) + (1.0 - lambda) / 2.0;
        assert!((w[0] - w0).abs() < 1e-12 && (w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // No answers: equal weights.
        let (_, w) = score(&[vec![], vec![]], &[]);
        assert_eq!(w, vec![0.5, 0.5]);
    }

    #[test]
    fn the_plain_line_names_no_technique_and_waits_for_enough_answers() {
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for n in [0, 1, 3, 9] {
                for best in [None, Some(Technique::Vimshottari), Some(Technique::Kp), Some(Technique::SlowOtherAyanamsa)] {
                    let s = plain_line(lang, best, n);
                    assert_eq!(jargon_in(lang, &s), None, "{s}");
                }
            }
            for t in TECHNIQUES {
                assert_eq!(jargon_in(lang, t.name(lang)), None);
            }
        }
        assert_eq!(plain_line(Lang::En, Some(Technique::Vimshottari), 9), "Your past fits the long-period reading best so far; 9 answers.");
        assert_eq!(plain_line(Lang::En, Some(Technique::Vimshottari), 3), "Too early to tell which reading fits you; 3 answers so far.");
    }

    #[test]
    fn a_life_that_follows_one_reading_is_learned() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let mut asked = Vec::new();
        let mut qs = Vec::new();
        for round in 0..3 {
            for q in past_reveal(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &asked, 4, round > 0).unwrap() {
                asked.push((q.from, q.to));
                qs.push(q);
            }
        }
        // A person whose life follows the Yogini reading: Yes where it agrees with the
        // stretch, No where it disagrees.
        let probe: Vec<Answer> = qs.iter().map(|q| Answer { from: q.from, to: q.to, life: q.life, good: q.good, verdict: Verdict::Yes }).collect();
        let prep = prepare(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, birth, &probe).unwrap();
        let j = judge(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, birth, &prep, &probe).unwrap();
        let answers: Vec<Answer> = probe
            .iter()
            .zip(&j.zs)
            .map(|(a, z)| Answer { verdict: if z[Witness::Yogini.index()].unwrap_or(0.0) > 0.0 { Verdict::Yes } else { Verdict::No }, ..*a })
            .collect();
        let t = std::time::Instant::now();
        let l = learn(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, birth, &answers, Lang::En);
        eprintln!("learned in {:?}: {}", t.elapsed(), l.plain);
        for s in &l.techniques {
            eprintln!("{:>22} read {:>2} brier {:.3} gain {:+.3} skill {:+.2} weight {:.3}", s.technique.key(), s.read, s.brier, s.log_gain, s.skill, s.weight);
        }
        assert_eq!(l.answers, answers.len());
        assert!((l.techniques.iter().map(|s| s.weight).sum::<f64>() - 1.0).abs() < 1e-9);
        assert_eq!(l.best, Some(Technique::Yogini), "{}", l.plain);
        assert_eq!(jargon_in(Lang::En, &l.plain), None);
        // Few answers keep the weights near equal.
        let few = learn(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, birth, &answers[..2], Lang::En);
        assert_eq!(few.best, None);
        for s in &few.techniques {
            assert!((s.weight - 1.0 / 6.0).abs() < 0.15, "{s:?}");
        }
    }
}
