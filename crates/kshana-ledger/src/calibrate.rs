//! Calibration: did the engine score the logged events above what a random instant
//! scores? The baseline is not random at all: it is the same area scored at evenly
//! spaced instants across a span, which is deterministic and estimates the same thing.
//! Every logged event is a positive (label 1) and every baseline instant a negative
//! (label 0); the Brier score and the reliability bins treat them as one forecast set.

use crate::event::{EventKind, LedgerError, LifeEvent};
use crate::score::{score_at, EventScore};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::{area, Facts, Lang};
use kshana_time::Instant;
use kshana_vedic::Ayanamsa;

/// Number of reliability bins across `[0, 1]`.
pub const RELIABILITY_BINS: usize = 5;

/// One reliability bin.
#[derive(Clone, Debug, PartialEq)]
pub struct ReliabilityBin {
    /// Centre of the bin's fit range (0.1, 0.3, 0.5, 0.7, 0.9).
    pub center: f64,
    /// Mean fit of the items in the bin; the centre when the bin is empty.
    pub predicted_mean: f64,
    /// Share of the items in the bin that are logged events rather than baseline instants.
    pub observed_rate: f64,
    /// Items in the bin, events and baseline instants together.
    pub count: usize,
}

/// How the engine's fits compare with chance.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    /// Logged events scored.
    pub n: usize,
    /// Baseline instants scored.
    pub baseline_n: usize,
    /// Mean fit over the events.
    pub mean_fit: f64,
    /// Share of events with fit ≥ 0.5.
    pub support_rate: f64,
    /// Mean fit over the baseline instants: what a random instant scores.
    pub base_rate: f64,
    /// `mean_fit − base_rate`: positive when the engine scored the real events above
    /// instants nobody logged. Zero when there is no baseline.
    pub lift: f64,
    /// Brier score: mean of `(fit − 1)²` over events and `fit²` over baseline instants,
    /// combined over `n + baseline_n` items. Lower is better; 0.25 is a constant 0.5.
    pub brier: f64,
    /// [`RELIABILITY_BINS`] bins over `[0, 1]`; their counts sum to `n + baseline_n`.
    pub reliability: Vec<ReliabilityBin>,
    /// The result in words.
    pub verdict: String,
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// Compare the scored events with the baseline fits.
pub fn calibrate(scores: &[EventScore], baseline: &[f64]) -> Calibration {
    let fits: Vec<f64> = scores.iter().map(|s| s.fit.clamp(0.0, 1.0)).collect();
    let base: Vec<f64> = baseline.iter().map(|f| f.clamp(0.0, 1.0)).collect();
    let n = fits.len();
    let baseline_n = base.len();
    let mean_fit = mean(&fits);
    let support_rate = if n == 0 {
        0.0
    } else {
        fits.iter().filter(|f| **f >= 0.5).count() as f64 / n as f64
    };
    let base_rate = mean(&base);
    let lift = if baseline_n == 0 {
        0.0
    } else {
        mean_fit - base_rate
    };
    let total = n + baseline_n;
    let brier = if total == 0 {
        0.0
    } else {
        (fits.iter().map(|f| (f - 1.0).powi(2)).sum::<f64>()
            + base.iter().map(|f| f.powi(2)).sum::<f64>())
            / total as f64
    };

    let width = 1.0 / RELIABILITY_BINS as f64;
    let bin_of = |f: f64| ((f / width).floor() as usize).min(RELIABILITY_BINS - 1);
    let mut sum_fit = [0.0; RELIABILITY_BINS];
    let mut events = [0usize; RELIABILITY_BINS];
    let mut count = [0usize; RELIABILITY_BINS];
    for f in &fits {
        let b = bin_of(*f);
        sum_fit[b] += f;
        events[b] += 1;
        count[b] += 1;
    }
    for f in &base {
        let b = bin_of(*f);
        sum_fit[b] += f;
        count[b] += 1;
    }
    let reliability: Vec<ReliabilityBin> = (0..RELIABILITY_BINS)
        .map(|b| {
            let center = (b as f64 + 0.5) * width;
            let (predicted_mean, observed_rate) = if count[b] == 0 {
                (center, 0.0)
            } else {
                (
                    sum_fit[b] / count[b] as f64,
                    events[b] as f64 / count[b] as f64,
                )
            };
            ReliabilityBin {
                center,
                predicted_mean,
                observed_rate,
                count: count[b],
            }
        })
        .collect();

    let verdict = verdict_words(
        n,
        baseline_n,
        mean_fit,
        support_rate,
        base_rate,
        lift,
        brier,
    );
    Calibration {
        n,
        baseline_n,
        mean_fit,
        support_rate,
        base_rate,
        lift,
        brier,
        reliability,
        verdict,
    }
}

fn verdict_words(
    n: usize,
    baseline_n: usize,
    mean_fit: f64,
    support_rate: f64,
    base_rate: f64,
    lift: f64,
    brier: f64,
) -> String {
    if n == 0 {
        return "No events logged yet, so there is nothing to calibrate.".to_string();
    }
    let mut v = format!(
        "{n} event{} scored a mean fit of {mean_fit:.2}; {:.0}% of them scored at least 0.50.",
        if n == 1 { "" } else { "s" },
        support_rate * 100.0
    );
    if baseline_n == 0 {
        v.push_str(" No baseline instants were scored, so the base rate is unknown and the lift cannot be judged.");
    } else {
        v.push_str(&format!(
            " {baseline_n} evenly spaced instant{} nobody logged scored {base_rate:.2} on average, a lift of {lift:+.2}; Brier {brier:.2}.",
            if baseline_n == 1 { "" } else { "s" }
        ));
        v.push(' ');
        v.push_str(if lift >= 0.15 {
            "The engine scored the logged events clearly above chance."
        } else if lift >= 0.05 {
            "The engine scored the logged events a little above chance: suggestive, not proof."
        } else if lift > -0.05 {
            "The engine scored the logged events no better than instants nobody logged: no evidence of fit."
        } else {
            "The engine scored the logged events below instants nobody logged: it pointed away from them."
        });
    }
    if n < 5 {
        v.push_str(" With fewer than five events this says little either way.");
    }
    v
}

/// A calibration's verdict in the reader's language, rebuilt from its numbers
/// (English gives back [`Calibration::verdict`]).
pub fn calibration_verdict(c: &Calibration, lang: Lang) -> String {
    if lang == Lang::En {
        return c.verdict.clone();
    }
    let (n, bn, mean_fit, base_rate, lift, brier) = (c.n, c.baseline_n, c.mean_fit, c.base_rate, c.lift, c.brier);
    if n == 0 {
        return lang
            .pick([
                "",
                "ഇതുവരെ സംഭവങ്ങളൊന്നും കുറിച്ചിട്ടില്ല, അതിനാൽ ഒത്തുനോക്കാൻ ഒന്നുമില്ല.",
                "अभी तक कोई घटना दर्ज नहीं हुई, इसलिए मिलाने को कुछ नहीं।",
            ])
            .to_string();
    }
    let pct = c.support_rate * 100.0;
    let mut v = kshana_rules::say3!(
        lang,
        "",
        "{n} സംഭവങ്ങൾക്ക് ശരാശരി ചേർച്ച {mean_fit:.2}; അവയിൽ {pct:.0}% എണ്ണം 0.50 എങ്കിലും നേടി.",
        "{n} घटनाओं का औसत मेल {mean_fit:.2}; इनमें से {pct:.0}% ने कम से कम 0.50 पाया।"
    );
    if bn == 0 {
        v.push_str(lang.pick([
            "",
            " ആരും കുറിക്കാത്ത താരതമ്യ സമയങ്ങൾ ഒന്നും നോക്കിയില്ല, അതിനാൽ അടിസ്ഥാന നിരക്ക് അറിയില്ല, മുന്നേറ്റം വിലയിരുത്താനാവില്ല.",
            " तुलना के लिए कोई बिना-दर्ज समय नहीं परखा गया, इसलिए आधार दर पता नहीं और बढ़त नहीं आँकी जा सकती।",
        ]));
    } else {
        v.push_str(&kshana_rules::say3!(
            lang,
            "",
            " ആരും കുറിക്കാത്ത, തുല്യ ഇടവേളയിലുള്ള {bn} സമയങ്ങൾക്ക് ശരാശരി {base_rate:.2}; മുന്നേറ്റം {lift:+.2}; Brier {brier:.2}.",
            " बराबर अंतराल पर लिए गए {bn} बिना-दर्ज समयों का औसत {base_rate:.2}; बढ़त {lift:+.2}; Brier {brier:.2}।"
        ));
        v.push(' ');
        v.push_str(if lift >= 0.15 {
            lang.pick([
                "",
                "കുറിച്ച സംഭവങ്ങൾക്ക് ക്ഷണ നൽകിയ ചേർച്ച ഊഹത്തേക്കാൾ വ്യക്തമായി മുകളിലാണ്.",
                "दर्ज घटनाओं पर क्षण का मेल अंदाज़े से साफ़ ऊपर रहा।",
            ])
        } else if lift >= 0.05 {
            lang.pick([
                "",
                "കുറിച്ച സംഭവങ്ങൾക്ക് ക്ഷണ നൽകിയ ചേർച്ച ഊഹത്തേക്കാൾ അൽപം മുകളിലാണ്: സൂചനയുണ്ട്, തെളിവല്ല.",
                "दर्ज घटनाओं पर क्षण का मेल अंदाज़े से थोड़ा ऊपर रहा: संकेत है, प्रमाण नहीं।",
            ])
        } else if lift > -0.05 {
            lang.pick([
                "",
                "കുറിച്ച സംഭവങ്ങൾക്ക് ആരും കുറിക്കാത്ത സമയങ്ങളേക്കാൾ മെച്ചമായ ചേർച്ചയില്ല: ചേർച്ചയ്ക്ക് തെളിവില്ല.",
                "दर्ज घटनाओं का मेल बिना-दर्ज समयों से बेहतर नहीं रहा: मेल का कोई प्रमाण नहीं।",
            ])
        } else {
            lang.pick([
                "",
                "കുറിച്ച സംഭവങ്ങൾക്ക് ആരും കുറിക്കാത്ത സമയങ്ങളേക്കാൾ കുറഞ്ഞ ചേർച്ച: അത് അവയിൽ നിന്ന് അകന്നാണ് ചൂണ്ടിയത്.",
                "दर्ज घटनाओं का मेल बिना-दर्ज समयों से भी कम रहा: इसने उनसे दूर इशारा किया।",
            ])
        });
    }
    if n < 5 {
        v.push_str(lang.pick([
            "",
            " അഞ്ചിൽ കുറവ് സംഭവങ്ങൾ കൊണ്ട് ഇത് ഏതുവശത്തേക്കും കാര്യമായി ഒന്നും പറയുന്നില്ല.",
            " पाँच से कम घटनाओं के साथ यह किसी भी ओर ज़्यादा नहीं कहता।",
        ]));
    }
    v
}

/// `n` evenly spaced instants across `[from, to]`, both ends included when `n ≥ 2`;
/// the midpoint when `n == 1`. Integer arithmetic, so the same inputs always give the
/// same instants.
pub fn baseline_instants(from: Instant, to: Instant, n: usize) -> Vec<Instant> {
    let (lo, hi) = if to < from { (to, from) } else { (from, to) };
    let span = hi.ns_since(lo);
    match n {
        0 => Vec::new(),
        1 => vec![lo.add_ns(span / 2)],
        _ => (0..n)
            .map(|i| lo.add_ns(span * i as i128 / (n as i128 - 1)))
            .collect(),
    }
}

/// Fits for a favourable event (a Milestone) in `area_name` at [`baseline_instants`]:
/// what a random instant scores, which is the honest base rate for the logged events.
pub fn baseline_fits(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    area_name: &str,
    from: Instant,
    to: Instant,
    n: usize,
) -> Result<Vec<f64>, LedgerError> {
    baseline_fits_for(
        spk,
        natal,
        ayanamsa,
        area_name,
        EventKind::Milestone,
        from,
        to,
        n,
    )
}

/// [`baseline_fits`] with the orientation of `kind`, so a ledger of losses is set
/// against the adverse reading of random instants.
#[allow(clippy::too_many_arguments)]
pub fn baseline_fits_for(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    area_name: &str,
    kind: EventKind,
    from: Instant,
    to: Instant,
    n: usize,
) -> Result<Vec<f64>, LedgerError> {
    let a = area(area_name).ok_or_else(|| LedgerError::UnknownArea(area_name.to_string()))?;
    let facts = Facts::from_chart(natal);
    baseline_instants(from, to, n)
        .into_iter()
        .map(|t| score_at(spk, natal, &facts, ayanamsa, a, kind, t).map(|s| s.fit))
        .collect()
}

/// A baseline matched to a ledger: for every logged event, `per_event` evenly spaced
/// instants across `[from, to]` scored with that event's own area and orientation, so a
/// ledger of three Career gains and one Health crisis is set against the same mix of
/// readings. Pass the result to [`calibrate`] with the events' scores.
pub fn baseline_for_ledger(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    events: &[LifeEvent],
    from: Instant,
    to: Instant,
    per_event: usize,
) -> Result<Vec<f64>, LedgerError> {
    let mut done: Vec<(String, bool, Vec<f64>)> = Vec::new();
    let mut out = Vec::with_capacity(events.len() * per_event);
    for ev in events {
        let adverse = ev.kind.is_adverse();
        let found = done
            .iter()
            .find(|(area_name, adv, _)| *area_name == ev.area && *adv == adverse);
        let fits = match found {
            Some((_, _, fits)) => fits.clone(),
            None => {
                let fits = baseline_fits_for(
                    spk, natal, ayanamsa, &ev.area, ev.kind, from, to, per_event,
                )?;
                done.push((ev.area.clone(), adverse, fits.clone()));
                fits
            }
        };
        out.extend(fits);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};
    use crate::score::score_event;
    use kshana_time::NS_PER_DAY;

    fn synthetic(fit: f64, adverse: bool) -> EventScore {
        EventScore {
            dasha_support: fit,
            parashari: fit,
            transit: fit,
            fit,
            adverse,
            running: Vec::new(),
            period_quality: fit,
            jupiter_from_moon: 1,
            saturn_from_moon: 1,
            sade_sati_phase: 0,
            explanation: vec!["synthetic".into()],
        }
    }

    #[test]
    fn calibration_arithmetic_is_consistent() {
        let scores: Vec<EventScore> = [0.9, 0.7, 0.55, 0.3]
            .iter()
            .map(|f| synthetic(*f, false))
            .collect();
        let baseline = [0.1, 0.3, 0.5, 0.2, 0.4, 0.9];
        let c = calibrate(&scores, &baseline);
        assert_eq!(c.n, 4);
        assert_eq!(c.baseline_n, 6);
        assert!((c.mean_fit - 0.6125).abs() < 1e-12);
        assert!((c.support_rate - 0.75).abs() < 1e-12);
        assert!((c.base_rate - 0.4).abs() < 1e-12);
        assert!((c.lift - 0.2125).abs() < 1e-12);
        let brier = (0.01 + 0.09 + 0.2025 + 0.49 + 0.01 + 0.09 + 0.25 + 0.04 + 0.16 + 0.81) / 10.0;
        assert!((c.brier - brier).abs() < 1e-12, "{}", c.brier);
        assert_eq!(c.reliability.len(), RELIABILITY_BINS);
        assert_eq!(c.reliability.iter().map(|b| b.count).sum::<usize>(), 10);
        assert_eq!(c.reliability[4].count, 2); // 0.9 event, 0.9 baseline
        assert!((c.reliability[4].observed_rate - 0.5).abs() < 1e-12);
        assert!((c.reliability[4].predicted_mean - 0.9).abs() < 1e-12);
        assert_eq!(c.reliability[1].count, 3); // 0.3 event, 0.3 and 0.2 baseline
        assert!(c.reliability.windows(2).all(|w| w[0].center < w[1].center));
        assert!(c.verdict.contains("clearly above chance"), "{}", c.verdict);
        assert!(c.verdict.contains("fewer than five"), "{}", c.verdict);

        let empty = calibrate(&[], &baseline);
        assert_eq!(empty.n, 0);
        assert!(empty.verdict.contains("nothing to calibrate"));
        assert_eq!(empty.reliability.iter().map(|b| b.count).sum::<usize>(), 6);

        let no_base = calibrate(&scores, &[]);
        assert_eq!(no_base.lift, 0.0);
        assert!(no_base.verdict.contains("base rate is unknown"));
        assert!((no_base.brier - (0.01 + 0.09 + 0.2025 + 0.49) / 4.0).abs() < 1e-12);

        let flat: Vec<EventScore> = (0..6).map(|_| synthetic(0.4, true)).collect();
        let chance = calibrate(&flat, &[0.4; 6]);
        assert!(
            chance.verdict.contains("no better than"),
            "{}",
            chance.verdict
        );
        let below = calibrate(&flat, &[0.8; 6]);
        assert!(below.verdict.contains("below"), "{}", below.verdict);
    }

    #[test]
    fn baseline_instants_are_even_and_deterministic() {
        let from = Instant::j2000();
        let to = from.add_ns(10 * NS_PER_DAY);
        assert!(baseline_instants(from, to, 0).is_empty());
        assert_eq!(
            baseline_instants(from, to, 1),
            vec![from.add_ns(5 * NS_PER_DAY)]
        );
        let ts = baseline_instants(from, to, 6);
        assert_eq!(ts.len(), 6);
        assert_eq!(ts[0], from);
        assert_eq!(ts[5], to);
        assert_eq!(ts[1], from.add_ns(2 * NS_PER_DAY));
        assert_eq!(baseline_instants(to, from, 6), ts);
        assert_eq!(baseline_instants(from, to, 6), ts);
    }

    #[test]
    fn kernel_calibration_of_the_kochi_ledger_is_consistent() {
        let Some((spk, ch)) = kochi() else { return };
        let events = [
            LifeEvent::new(
                noon(2013, 7, 1),
                "Career",
                EventKind::Began,
                0.9,
                "first job",
            )
            .unwrap(),
            LifeEvent::new(
                noon(2018, 11, 22),
                "Marriage",
                EventKind::Milestone,
                1.0,
                "wedding",
            )
            .unwrap(),
            LifeEvent::new(
                noon(2020, 6, 15),
                "Health",
                EventKind::Crisis,
                0.7,
                "fell ill",
            )
            .unwrap(),
        ];
        let scores: Vec<EventScore> = events
            .iter()
            .map(|e| score_event(&spk, &ch, Ayanamsa::Lahiri, e).unwrap())
            .collect();
        let base = baseline_fits(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            noon(2010, 1, 1),
            noon(2024, 1, 1),
            12,
        )
        .unwrap();
        assert_eq!(base.len(), 12);
        assert!(base.iter().all(|f| (0.0..=1.0).contains(f)));
        let adverse = baseline_fits_for(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Health",
            EventKind::Crisis,
            noon(2010, 1, 1),
            noon(2024, 1, 1),
            3,
        )
        .unwrap();
        assert_eq!(adverse.len(), 3);
        let matched = baseline_for_ledger(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &events,
            noon(2010, 1, 1),
            noon(2024, 1, 1),
            4,
        )
        .unwrap();
        assert_eq!(matched.len(), 12);
        assert!(matched.iter().all(|f| (0.0..=1.0).contains(f)));
        let m = calibrate(&scores, &matched);
        assert_eq!(m.baseline_n, 12);
        assert!((0.0..=1.0).contains(&m.brier));
        let c = calibrate(&scores, &base);
        assert_eq!(c.n, 3);
        assert_eq!(c.baseline_n, 12);
        assert!((0.0..=1.0).contains(&c.mean_fit));
        assert!((0.0..=1.0).contains(&c.support_rate));
        assert!((0.0..=1.0).contains(&c.base_rate));
        assert!((-1.0..=1.0).contains(&c.lift));
        assert!((0.0..=1.0).contains(&c.brier), "{}", c.brier);
        assert_eq!(c.reliability.iter().map(|b| b.count).sum::<usize>(), 15);
        assert!(c
            .reliability
            .iter()
            .all(|b| (0.0..=1.0).contains(&b.observed_rate)
                && (0.0..=1.0).contains(&b.predicted_mean)));
        assert!(!c.verdict.is_empty());
        assert!(baseline_fits(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Pets",
            noon(2010, 1, 1),
            noon(2011, 1, 1),
            2
        )
        .is_err());
    }
}
