//! Proper scores for probabilities against outcomes: the Brier score with Murphy's
//! decomposition, the base-rate skill score, the expected calibration error, the
//! calibration curve and Wilson intervals.
//!
//! For `N` forecasts `p_i` with outcomes `o_i ∈ {0, 1}`:
//! `BS = (1/N) Σ (p_i − o_i)²`; `BS_ref = ō (1 − ō)` with `ō` the base rate;
//! `BSS = 1 − BS / BS_ref` (0 is no better than always saying the base rate). Over
//! `K` bins of predicted probability with `n_k` members, mean prediction `p̄_k` and
//! observed frequency `ō_k`: reliability `REL = Σ (n_k/N)(p̄_k − ō_k)²`, resolution
//! `RES = Σ (n_k/N)(ō_k − ō)²`, uncertainty `UNC = ō (1 − ō)` (Murphy 1973). The
//! three sum to the Brier score only when every forecast in a bin equals the bin
//! mean; for raw probabilities the identity is `BS = REL − RES + UNC + WBV − WBC`
//! with the within-bin variance `WBV = (1/N) Σ_k Σ_{i∈k} (p_i − p̄_k)²` and
//! covariance `WBC = (2/N) Σ_k Σ_{i∈k} (p_i − p̄_k)(o_i − ō_k)` (Stephenson, Coelho
//! and Jolliffe 2008), which are reported so the identity holds to the last digit.
//! `ECE = Σ (n_k/N) |ō_k − p̄_k|`. Wilson's interval for `x` of `n` at `z`: centre
//! `(p̂ + z²/2n) / (1 + z²/n)`, half-width `z √(p̂(1−p̂)/n + z²/4n²) / (1 + z²/n)`.
//!
//! Display rules from the research: no percentage below ten items, a bin needs five,
//! no calibration curve below twenty, and bins default to `⌈N^(1/3)⌉` (sklearn's
//! cube-root rule).

use kshana_rules::Lang;

/// `z` for a 95 % interval.
pub const Z95: f64 = 1.959_963_984_540_054;
/// Items below which no percentage is shown.
pub const MIN_FOR_PERCENT: usize = 10;
/// Items below which no calibration curve is shown.
pub const MIN_FOR_CURVE: usize = 20;
/// Members a bin needs to be shown on the curve.
pub const MIN_PER_BIN: usize = 5;

/// Wilson's score interval for `successes` of `n` at `z` standard errors, `(lo, hi)`;
/// `(0, 1)` when `n == 0`.
pub fn wilson_interval(successes: usize, n: usize, z: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let n_f = n as f64;
    let p = successes as f64 / n_f;
    let z2 = z * z;
    let denom = 1.0 + z2 / n_f;
    let centre = (p + z2 / (2.0 * n_f)) / denom;
    let half = z * (p * (1.0 - p) / n_f + z2 / (4.0 * n_f * n_f)).sqrt() / denom;
    // At 0 of n and n of n the interval touches the edge exactly; snap the rounding.
    let lo = if successes == 0 {
        0.0
    } else {
        (centre - half).max(0.0)
    };
    let hi = if successes == n {
        1.0
    } else {
        (centre + half).min(1.0)
    };
    (lo, hi)
}

/// `⌈n^(1/3)⌉`, at least 1.
pub fn cube_root_bins(n: usize) -> usize {
    ((n as f64).cbrt().ceil() as usize).max(1)
}

/// One bin of the calibration curve.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationBin {
    /// Lower edge of predicted probability (inclusive).
    pub lo: f64,
    /// Upper edge (exclusive, except 1 in the last bin).
    pub hi: f64,
    /// Members.
    pub count: usize,
    /// Mean predicted probability; the bin centre when empty.
    pub predicted_mean: f64,
    /// Observed frequency; 0 when empty.
    pub observed_rate: f64,
    /// Wilson 95 % interval of the observed frequency.
    pub observed_interval: (f64, f64),
    /// `count ≥ MIN_PER_BIN`.
    pub shown: bool,
}

/// The scores of a forecast set.
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    /// Forecasts scored.
    pub n: usize,
    /// Outcomes that happened.
    pub positives: usize,
    /// `positives / n`.
    pub base_rate: f64,
    /// Wilson 95 % interval of the base rate.
    pub base_rate_interval: (f64, f64),
    /// Brier score.
    pub brier: f64,
    /// `ō (1 − ō)`, the Brier score of always saying the base rate.
    pub brier_reference: f64,
    /// `1 − brier / brier_reference`; 0 when the reference is 0.
    pub skill: f64,
    /// Murphy reliability (lower is better).
    pub reliability: f64,
    /// Murphy resolution (higher is better).
    pub resolution: f64,
    /// Murphy uncertainty, equal to `brier_reference`.
    pub uncertainty: f64,
    /// Within-bin variance of the predictions.
    pub within_bin_variance: f64,
    /// Within-bin covariance term.
    pub within_bin_covariance: f64,
    /// Expected calibration error.
    pub ece: f64,
    /// Equal-width bins over `[0, 1]`.
    pub bins: Vec<CalibrationBin>,
    /// `n ≥ MIN_FOR_CURVE`.
    pub curve_shown: bool,
    /// The scores in words, obeying the display rules.
    pub words: String,
}

fn clean(p: f64) -> f64 {
    if p.is_finite() {
        p.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

/// Score `probabilities` against `labels` (paired by index; the shorter length is
/// used) over `bins` equal-width bins, or `⌈N^(1/3)⌉` bins when `bins == 0`.
pub fn metrics(probabilities: &[f64], labels: &[bool], bins: usize) -> Metrics {
    let n = probabilities.len().min(labels.len());
    let ps: Vec<f64> = probabilities.iter().take(n).map(|p| clean(*p)).collect();
    let os: Vec<f64> = labels
        .iter()
        .take(n)
        .map(|b| if *b { 1.0 } else { 0.0 })
        .collect();
    let k = if bins == 0 { cube_root_bins(n) } else { bins };
    let n_f = n.max(1) as f64;
    let positives = labels.iter().take(n).filter(|b| **b).count();
    let base_rate = if n == 0 { 0.0 } else { positives as f64 / n_f };
    let brier = ps
        .iter()
        .zip(&os)
        .map(|(p, o)| (p - o).powi(2))
        .sum::<f64>()
        / n_f;
    let uncertainty = base_rate * (1.0 - base_rate);
    let skill = if uncertainty > 0.0 {
        1.0 - brier / uncertainty
    } else {
        0.0
    };

    let width = 1.0 / k as f64;
    let bin_of = |p: f64| ((p / width).floor() as usize).min(k - 1);
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (i, p) in ps.iter().enumerate() {
        members[bin_of(*p)].push(i);
    }
    let mut reliability = 0.0;
    let mut resolution = 0.0;
    let mut wbv = 0.0;
    let mut wbc = 0.0;
    let mut ece = 0.0;
    let mut out = Vec::with_capacity(k);
    for (b, idx) in members.iter().enumerate() {
        let lo = b as f64 * width;
        let hi = if b + 1 == k {
            1.0
        } else {
            (b + 1) as f64 * width
        };
        let count = idx.len();
        let (predicted_mean, observed_rate) = if count == 0 {
            (0.5 * (lo + hi), 0.0)
        } else {
            (
                idx.iter().map(|i| ps[*i]).sum::<f64>() / count as f64,
                idx.iter().map(|i| os[*i]).sum::<f64>() / count as f64,
            )
        };
        if count > 0 {
            let share = count as f64 / n_f;
            reliability += share * (predicted_mean - observed_rate).powi(2);
            resolution += share * (observed_rate - base_rate).powi(2);
            ece += share * (observed_rate - predicted_mean).abs();
            for i in idx {
                let dp = ps[*i] - predicted_mean;
                wbv += dp * dp / n_f;
                wbc += 2.0 * dp * (os[*i] - observed_rate) / n_f;
            }
        }
        let hits = idx.iter().filter(|i| os[**i] > 0.5).count();
        out.push(CalibrationBin {
            lo,
            hi,
            count,
            predicted_mean,
            observed_rate,
            observed_interval: wilson_interval(hits, count, Z95),
            shown: count >= MIN_PER_BIN,
        });
    }
    let base_rate_interval = wilson_interval(positives, n, Z95);
    let curve_shown = n >= MIN_FOR_CURVE;
    let words = words(
        n,
        positives,
        base_rate,
        base_rate_interval,
        brier,
        uncertainty,
        skill,
        reliability,
        resolution,
        ece,
        k,
        curve_shown,
    );
    Metrics {
        n,
        positives,
        base_rate,
        base_rate_interval,
        brier,
        brier_reference: uncertainty,
        skill,
        reliability,
        resolution,
        uncertainty,
        within_bin_variance: wbv,
        within_bin_covariance: wbc,
        ece,
        bins: out,
        curve_shown,
        words,
    }
}

/// "31 of 40 (78%, 95% CI 63–88%)" at ten or more, "3 of 7" below.
pub fn count_words(successes: usize, n: usize) -> String {
    count_words_in(successes, n, Lang::En)
}

/// [`count_words`] in the reader's language.
pub fn count_words_in(successes: usize, n: usize, lang: Lang) -> String {
    let head = kshana_rules::say3!(lang, "{successes} of {n}", "{n}-ൽ {successes}", "{n} में से {successes}");
    if n < MIN_FOR_PERCENT {
        head
    } else {
        let (lo, hi) = wilson_interval(successes, n, Z95);
        format!(
            "{head} ({:.0}%, 95% CI {:.0}–{:.0}%)",
            100.0 * successes as f64 / n as f64,
            100.0 * lo,
            100.0 * hi
        )
    }
}

/// A metrics summary's words in the reader's language, rebuilt from its numbers
/// (the English is the [`Metrics::words`] it already carries).
pub fn metrics_words(m: &Metrics, lang: Lang) -> String {
    words_in(
        lang,
        m.n,
        m.positives,
        m.base_rate,
        m.brier,
        m.brier_reference,
        m.skill,
        m.reliability,
        m.resolution,
        m.ece,
        m.bins.len(),
        m.curve_shown,
    )
}

#[allow(clippy::too_many_arguments)]
fn words_in(
    lang: Lang,
    n: usize,
    positives: usize,
    base_rate: f64,
    brier: f64,
    reference: f64,
    skill: f64,
    reliability: f64,
    resolution: f64,
    ece: f64,
    bins: usize,
    curve_shown: bool,
) -> String {
    if lang == Lang::En {
        return words(
            n, positives, base_rate, (0.0, 0.0), brier, reference, skill, reliability, resolution, ece, bins,
            curve_shown,
        );
    }
    if n == 0 {
        return lang
            .pick([
                "",
                "ഇതുവരെ ഫലം തീർന്ന പ്രവചനങ്ങളില്ല: മുദ്രവെച്ചത് 0, ഫലം തീർന്നത് 0.",
                "अभी तक कोई हल हुआ पूर्वानुमान नहीं: सील 0, हल 0।",
            ])
            .to_string();
    }
    let count = count_words_in(positives, n, lang);
    let mut w = kshana_rules::say3!(lang, "", "{count} സംഭവിച്ചു.", "{count} हुए।");
    if reference > 0.0 {
        let tail = if skill > 0.0 {
            lang.pick(["", ", അടിസ്ഥാന നിരക്കിനേക്കാൾ മെച്ചം", ", आधार दर से बेहतर"])
        } else if skill < 0.0 {
            lang.pick(["", ", അടിസ്ഥാന നിരക്കിനേക്കാൾ മോശം", ", आधार दर से बदतर"])
        } else {
            ""
        };
        w.push_str(&kshana_rules::say3!(
            lang,
            "",
            " Brier {brier:.3}; എപ്പോഴും അടിസ്ഥാന നിരക്ക് {base_rate:.2} എന്ന് പറഞ്ഞാൽ കിട്ടുന്നത് {reference:.3}: മികവ് {skill:+.2}{tail}.",
            " Brier {brier:.3}; हमेशा आधार दर {base_rate:.2} कहने पर {reference:.3} मिलता: कौशल {skill:+.2}{tail}।"
        ));
    } else {
        w.push_str(&kshana_rules::say3!(
            lang,
            "",
            " Brier {brier:.3}; എല്ലാ ഫലവും ഒരുപോലെയായിരുന്നു, അതിനാൽ മറികടക്കാൻ ഒരു അടിസ്ഥാന നിരക്കില്ല.",
            " Brier {brier:.3}; हर नतीजा एक-सा था, इसलिए पार करने को कोई आधार दर नहीं।"
        ));
    }
    w.push_str(&kshana_rules::say3!(
        lang,
        "",
        " വിശ്വാസ്യത {reliability:.3} (കുറവാണ് നല്ലത്), വേർതിരിവ് {resolution:.3} (കൂടുതലാണ് നല്ലത്), ECE {ece:.3}, {bins} തട്ടുകളിൽ.",
        " विश्वसनीयता {reliability:.3} (कम बेहतर), विभेदन {resolution:.3} (ज़्यादा बेहतर), ECE {ece:.3}, {bins} खानों में।"
    ));
    if !curve_shown {
        w.push_str(&kshana_rules::say3!(
            lang,
            "",
            " ഒത്തുനോക്കൽ രേഖ വരയ്ക്കാൻ ഫലം തീർന്ന {MIN_FOR_CURVE} പ്രവചനങ്ങൾ വേണം; ഇതുവരെ {n}.",
            " कैलिब्रेशन वक्र के लिए {MIN_FOR_CURVE} हल हुए पूर्वानुमान चाहिए; अभी तक {n}।"
        ));
    }
    if n < MIN_FOR_PERCENT {
        w.push_str(lang.pick([
            "",
            " എണ്ണത്തിനപ്പുറം എന്തെങ്കിലും പറയാൻ ഇത് പോരാ.",
            " गिनती से आगे कुछ कहने को ये बहुत कम हैं।",
        ]));
    }
    w
}

#[allow(clippy::too_many_arguments)]
fn words(
    n: usize,
    positives: usize,
    base_rate: f64,
    _interval: (f64, f64),
    brier: f64,
    reference: f64,
    skill: f64,
    reliability: f64,
    resolution: f64,
    ece: f64,
    bins: usize,
    curve_shown: bool,
) -> String {
    if n == 0 {
        return "No resolved forecasts yet: sealed 0, resolved 0.".to_string();
    }
    let mut w = format!("{} happened.", count_words(positives, n));
    if reference > 0.0 {
        w.push_str(&format!(
            " Brier {brier:.3} against {reference:.3} for always saying the base rate {base_rate:.2}: skill {skill:+.2}{}.",
            if skill > 0.0 { ", better than the base rate" } else if skill < 0.0 { ", worse than the base rate" } else { "" }
        ));
    } else {
        w.push_str(&format!(
            " Brier {brier:.3}; every outcome was the same, so there is no base rate to beat."
        ));
    }
    w.push_str(&format!(
        " Reliability {reliability:.3} (lower is better), resolution {resolution:.3} (higher is better), ECE {ece:.3} over {bins} bin{}.",
        if bins == 1 { "" } else { "s" }
    ));
    if !curve_shown {
        w.push_str(&format!(
            " The calibration curve needs {MIN_FOR_CURVE} resolved forecasts; {n} so far."
        ));
    }
    if n < MIN_FOR_PERCENT {
        w.push_str(" Too few to say more than the count.");
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_and_bins() {
        let (lo, hi) = wilson_interval(31, 40, Z95);
        // Textbook: 31/40 → about 62.5 % to 87.5 %.
        assert!(
            (lo - 0.625).abs() < 0.01 && (hi - 0.875).abs() < 0.01,
            "{lo} {hi}"
        );
        assert_eq!(wilson_interval(0, 0, Z95), (0.0, 1.0));
        let (z, _) = wilson_interval(0, 10, Z95);
        assert_eq!(z, 0.0);
        let (_, one) = wilson_interval(10, 10, Z95);
        assert_eq!(one, 1.0);
        assert_eq!(cube_root_bins(40), 4);
        assert_eq!(cube_root_bins(8), 2);
        assert_eq!(cube_root_bins(0), 1);
        assert_eq!(count_words(3, 7), "3 of 7");
        assert!(count_words(31, 40).starts_with("31 of 40 (78%, 95% CI "));
    }

    #[test]
    fn perfectly_calibrated_set_has_zero_ece_and_the_identity_holds() {
        // Four bins of ten: in the bin with p the outcome is true for exactly 10 p.
        let mut ps = Vec::new();
        let mut os = Vec::new();
        for p in [0.1, 0.3, 0.5, 0.9] {
            for i in 0..10 {
                ps.push(p);
                os.push((i as f64) < 10.0 * p - 1e-9);
            }
        }
        let m = metrics(&ps, &os, 5);
        assert_eq!(m.n, 40);
        assert_eq!(m.positives, 18);
        assert!(m.ece.abs() < 1e-12, "{}", m.ece);
        assert!(m.reliability.abs() < 1e-12);
        assert!(m.within_bin_variance.abs() < 1e-12);
        assert!(m.within_bin_covariance.abs() < 1e-12);
        let identity = m.reliability - m.resolution + m.uncertainty + m.within_bin_variance
            - m.within_bin_covariance;
        assert!(
            (m.brier - identity).abs() < 1e-12,
            "{} vs {identity}",
            m.brier
        );
        assert!((m.uncertainty - 0.45 * 0.55).abs() < 1e-12);
        assert!(m.skill > 0.0);
        assert_eq!(m.bins.len(), 5);
        assert_eq!(m.bins.iter().map(|b| b.count).sum::<usize>(), 40);
        assert!(m.curve_shown);
        assert!(m.words.contains("skill +"), "{}", m.words);
        assert!(!m.words.contains("needs 20"), "{}", m.words);

        // Raw probabilities: the identity still holds through the within-bin terms.
        let raw: Vec<f64> = (0..30).map(|i| (i as f64 * 0.37 + 0.05) % 1.0).collect();
        let lab: Vec<bool> = (0..30).map(|i| i % 3 != 0).collect();
        let r = metrics(&raw, &lab, 0);
        assert_eq!(r.bins.len(), cube_root_bins(30));
        let identity = r.reliability - r.resolution + r.uncertainty + r.within_bin_variance
            - r.within_bin_covariance;
        assert!(
            (r.brier - identity).abs() < 1e-12,
            "{} vs {identity}",
            r.brier
        );
        assert!((r.resolution - r.uncertainty) <= 1e-12);
        for b in &r.bins {
            assert!(b.lo < b.hi);
            assert!((0.0..=1.0).contains(&b.predicted_mean));
            assert!(b.observed_interval.0 <= b.observed_rate + 1e-12);
            assert!(b.observed_interval.1 >= b.observed_rate - 1e-12);
        }
        // Miscalibrated: say 0.9 and be right half the time.
        let bad = metrics(
            &[0.9; 10],
            &[
                true, false, true, false, true, false, true, false, true, false,
            ],
            2,
        );
        assert!((bad.ece - 0.4).abs() < 1e-12);
        assert!(bad.skill < 0.0);
        assert!(bad.words.contains("worse than"), "{}", bad.words);
        assert!(bad.words.contains("needs 20"), "{}", bad.words);
        // Degenerate inputs.
        let empty = metrics(&[], &[], 3);
        assert_eq!(empty.n, 0);
        assert!(empty.words.contains("sealed 0"));
        let same = metrics(
            &[0.2, 0.7, f64::NAN, 3.0],
            &[true, true, true, true, true],
            2,
        );
        assert_eq!(same.n, 4);
        assert_eq!(same.skill, 0.0);
        assert!(same.words.contains("no base rate"));
        assert!(same.words.contains("Too few"));
    }
}
