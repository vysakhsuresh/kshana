//! Sealed forecasts: a pre-registered claim about a dated window, committed before
//! the window opens, so the Brier ledger cannot be gamed.
//!
//! A [`SealedForecast`] holds the engine's instant of creation, the window as exact
//! instants, the area and kind, the probability the engine gave (the mean fit over
//! the window, from the exact [`FitProfile`](crate::FitProfile)), the basis in rule
//! ids and numbers, the yes/no question that resolves it, and an `id` that is the
//! SHA-256 of the canonical JSON of all of that. Anything created after its window
//! opened is refused: it belongs in the Explained ledger, never here. Resolution is
//! appended later and is not part of the id, so [`SealedForecast::verify`] exposes any
//! edit to the claim after sealing. [`sealed_brier`] scores the resolved forecasts
//! with [`metrics`] and nothing else: the ledger starts at "sealed 0, resolved 0" and
//! that honesty is the feature.

use crate::coverage::{fit_profile, SUPPORT_THRESHOLD};
use crate::event::{iso_utc, month_year, parse_iso_utc, EventKind, LedgerError};
use crate::hash::sha256_hex;
use crate::json::{self, Value};
use crate::metrics::{metrics, Metrics};
use crate::score::{WEIGHT_DASHA, WEIGHT_PARASHARI, WEIGHT_TRANSIT};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::{area, promise, Facts, Lang};
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa};

/// A pre-registered forecast.
#[derive(Clone, Debug, PartialEq)]
pub struct SealedForecast {
    /// SHA-256 (hex) of the canonical JSON of every field below except `resolved`.
    pub id: String,
    /// When the forecast was sealed, by the engine's clock.
    pub created_at: Instant,
    /// One of the eight areas.
    pub area: String,
    /// The kind of event claimed.
    pub kind: EventKind,
    /// Exact start of the window (open after `created_at`).
    pub window_start: Instant,
    /// Exact end of the window.
    pub window_end: Instant,
    /// The engine's probability: the mean fit over the window.
    pub probability: f64,
    /// Rule ids and numbers the probability rests on.
    pub basis: Vec<String>,
    /// The yes/no question that resolves the forecast, with its close date.
    pub question: String,
    /// `(happened, when resolved)` once resolved.
    pub resolved: Option<(bool, Instant)>,
}

fn kind_phrase(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Began => "something begin",
        EventKind::Ended => "something end",
        EventKind::Gain => "a gain come",
        EventKind::Loss => "a loss come",
        EventKind::Milestone => "a milestone come",
        EventKind::Crisis => "a crisis come",
    }
}

impl SealedForecast {
    fn canonical_fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("created_at", Value::Str(iso_utc(self.created_at))),
            ("area", Value::Str(self.area.clone())),
            ("kind", Value::Str(self.kind.name().to_string())),
            ("window_start", Value::Str(iso_utc(self.window_start))),
            ("window_end", Value::Str(iso_utc(self.window_end))),
            ("probability", Value::Num(self.probability)),
            (
                "basis",
                Value::List(self.basis.iter().cloned().map(Value::Str).collect()),
            ),
            ("question", Value::Str(self.question.clone())),
        ]
    }

    /// The id the content deserves: SHA-256 of its canonical JSON.
    pub fn content_id(&self) -> String {
        sha256_hex(&json::write_object(&self.canonical_fields()))
    }

    /// Whether `id` matches the content: false after any edit to the sealed claim.
    pub fn verify(&self) -> bool {
        self.id == self.content_id()
    }

    /// Whether a resolution has been recorded.
    pub fn is_resolved(&self) -> bool {
        self.resolved.is_some()
    }

    /// One line of JSON, no newline.
    pub fn to_json_line(&self) -> String {
        let mut fields = vec![("id", Value::Str(self.id.clone()))];
        fields.extend(self.canonical_fields());
        match self.resolved {
            Some((happened, when)) => {
                fields.push(("happened", Value::Bool(happened)));
                fields.push(("resolved_at", Value::Str(iso_utc(when))));
            }
            None => {
                fields.push(("happened", Value::Null));
                fields.push(("resolved_at", Value::Null));
            }
        }
        json::write_object(&fields)
    }

    /// Read one line written by [`SealedForecast::to_json_line`]. The id is read as
    /// stored, not recomputed: call [`SealedForecast::verify`] to check it.
    pub fn from_json_line(line: &str) -> Result<Self, LedgerError> {
        let fields = json::parse_object(line).map_err(LedgerError::BadLine)?;
        let get = |k: &str| fields.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        let text = |k: &str| match get(k) {
            Some(Value::Str(s)) => Ok(s.clone()),
            _ => Err(LedgerError::BadLine(format!("missing \"{k}\""))),
        };
        let when = |k: &str| parse_iso_utc(&text(k)?);
        let area_name = text("area")?;
        let a = area(&area_name).ok_or(LedgerError::UnknownArea(area_name))?;
        let kind_name = text("kind")?;
        let kind = EventKind::from_name(&kind_name).ok_or(LedgerError::UnknownKind(kind_name))?;
        let probability = match get("probability") {
            Some(Value::Num(n)) => *n,
            _ => return Err(LedgerError::BadLine("missing \"probability\"".into())),
        };
        let basis = get("basis").map(Value::strings).unwrap_or_default();
        let question = text("question").unwrap_or_default();
        let resolved = match (get("happened"), get("resolved_at")) {
            (Some(Value::Bool(h)), Some(Value::Str(s))) => Some((*h, parse_iso_utc(s)?)),
            _ => None,
        };
        Ok(Self {
            id: text("id")?,
            created_at: when("created_at")?,
            area: a.name.to_string(),
            kind,
            window_start: when("window_start")?,
            window_end: when("window_end")?,
            probability,
            basis,
            question,
            resolved,
        })
    }
}

/// Seal a forecast for an area and kind over `window`, with the probability computed
/// from the engine as the mean fit over the window. Refused when the window does not
/// open after `at_creation`, is empty, or lies before birth.
#[allow(clippy::too_many_arguments)]
pub fn seal(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    at_creation: Instant,
    area_name: &str,
    kind: EventKind,
    window: (Instant, Instant),
) -> Result<SealedForecast, LedgerError> {
    seal_in(spk, natal, ayanamsa, at_creation, area_name, kind, window, Lang::En)
}

fn kind_phrase_in(lang: Lang, kind: EventKind) -> &'static str {
    let t = match kind {
        EventKind::Began => ["something begin", "ഒരു പുതിയ തുടക്കം ഉണ്ടായോ", "कुछ शुरू हुआ"],
        EventKind::Ended => ["something end", "എന്തെങ്കിലും അവസാനിച്ചോ", "कुछ ख़त्म हुआ"],
        EventKind::Gain => ["a gain come", "ഒരു നേട്ടം വന്നോ", "कोई लाभ आया"],
        EventKind::Loss => ["a loss come", "ഒരു നഷ്ടം വന്നോ", "कोई नुकसान आया"],
        EventKind::Milestone => ["a milestone come", "ഒരു നാഴികക്കല്ല് പിന്നിട്ടോ", "कोई अहम पड़ाव आया"],
        EventKind::Crisis => ["a crisis come", "ഒരു പ്രതിസന്ധി വന്നോ", "कोई संकट आया"],
    };
    lang.pick(t)
}

/// [`seal`], with the basis and the question written in the reader's language: the
/// words sealed are the words the person reads.
#[allow(clippy::too_many_arguments)]
pub fn seal_in(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    at_creation: Instant,
    area_name: &str,
    kind: EventKind,
    window: (Instant, Instant),
    lang: Lang,
) -> Result<SealedForecast, LedgerError> {
    let a = area(area_name).ok_or_else(|| LedgerError::UnknownArea(area_name.to_string()))?;
    let (start, end) = window;
    if end <= start {
        return Err(LedgerError::Refused("the window is empty".into()));
    }
    if start <= at_creation {
        return Err(LedgerError::Refused(format!(
            "the window opens at {} but the forecast is being sealed at {}: a claim about a window already open belongs in the Explained ledger",
            iso_utc(start),
            iso_utc(at_creation)
        )));
    }
    let profile = fit_profile(spk, natal, ayanamsa, a.name, kind, start, end)?;
    let probability = profile
        .mean_fit_over(start, end)
        .ok_or_else(|| LedgerError::Refused("the window lies before birth".into()))?;
    let facts = Facts::from_chart(natal);
    let p = promise(&facts, a);
    let lords: Vec<String> = vimshottari_levels(
        natal.bodies[1].sidereal,
        natal.instant,
        YEAR_365_25_NS,
        start,
        3,
    )
    .iter()
    .map(|x| format!("{:?}", x.lord))
    .collect();
    let coverage = profile
        .coverage_within(start, end, SUPPORT_THRESHOLD)
        .unwrap_or(0.0);
    let basis = if lang != Lang::En {
        let ml = lang == Lang::Ml;
        let an = kshana_rules::lang::area_name(lang, a.name);
        let sub = kshana_rules::lang::graha_name(lang, p.sub_lord);
        let lord_names: Vec<&str> = vimshottari_levels(natal.bodies[1].sidereal, natal.instant, YEAR_365_25_NS, start, 3)
            .iter()
            .map(|x| kshana_rules::lang::graha_name(lang, x.lord))
            .collect();
        let ls = if lord_names.is_empty() { lang.pick(["none", "ഇല്ല", "कोई नहीं"]).to_string() } else { lord_names.join(" / ") };
        let (rule, s, prim, n, c) = (p.hit.rule_id, p.strength, a.primary, profile.segments.len(), coverage * 100.0);
        if ml {
            vec![
                format!("{rule} {an} വാഗ്ദാനം: ശക്തി {s:.2}, {prim}-ാം ഭാവാരംഭത്തിന്റെ ഉപനാഥൻ {sub}, {}", if p.promised { "വാഗ്ദാനമുണ്ട്" } else { "വാഗ്ദാനമില്ല" }),
                format!("കാലം തുടങ്ങുമ്പോഴുള്ള വിംശോത്തരി നാഥന്മാർ: {ls} (മഹാദശ / അന്തർദശ / പ്രത്യന്തരം)"),
                format!("{n} കൃത്യ പ്രത്യന്തര ഖണ്ഡങ്ങളിലെ ശരാശരി ചേർച്ച {probability:.3}; കാലത്തിന്റെ {c:.0}% സമയം ചേർച്ച {SUPPORT_THRESHOLD:.2} അല്ലെങ്കിൽ അതിലേറെ"),
                format!("സാധ്യത = ശരാശരി ചേർച്ച = {WEIGHT_DASHA} × ദശ + {WEIGHT_PARASHARI} × പാരാശരി + {WEIGHT_TRANSIT} × ഗോചരം; ഗോചരം ഓരോ ഖണ്ഡത്തിലും ഒരിക്കൽ അളന്നു"),
            ]
        } else {
            vec![
                format!("{rule} {an} का वादा: बल {s:.2}, {prim}वें भाव-संधि का उप-स्वामी {sub}, {}", if p.promised { "वादा है" } else { "वादा नहीं" }),
                format!("अवधि की शुरुआत में विंशोत्तरी स्वामी: {ls} (महादशा / अंतर्दशा / प्रत्यंतर)"),
                format!("{n} सटीक प्रत्यंतर खंडों पर औसत मेल {probability:.3}; अवधि के {c:.0}% समय मेल {SUPPORT_THRESHOLD:.2} या उससे ऊपर"),
                format!("संभावना = औसत मेल = {WEIGHT_DASHA} × दशा + {WEIGHT_PARASHARI} × पाराशरी + {WEIGHT_TRANSIT} × गोचर; गोचर हर खंड में एक बार मापा गया"),
            ]
        }
    } else {
        vec![
        format!(
            "{} promise for {}: strength {:.2}, {}th cusp sub-lord {:?}, {}",
            p.hit.rule_id,
            a.name,
            p.strength,
            a.primary,
            p.sub_lord,
            if p.promised { "promised" } else { "not promised" }
        ),
        format!(
            "Vimshottari lords at the window's start: {} (mahadasha / antardasha / pratyantar)",
            if lords.is_empty() {
                "none".to_string()
            } else {
                lords.join(" / ")
            }
        ),
        format!(
            "mean fit {probability:.3} over {} exact pratyantar cells; fit at or above {SUPPORT_THRESHOLD:.2} during {:.0}% of the window",
            profile.segments.len(),
            coverage * 100.0
        ),
        format!(
            "probability = mean fit = {WEIGHT_DASHA} × dasha + {WEIGHT_PARASHARI} × Parashari + {WEIGHT_TRANSIT} × transit; transit sampled once per cell"
        ),
    ]
    };
    let question = if lang != Lang::En {
        let an = kshana_rules::lang::area_name(lang, a.name);
        let (s, e) = (kshana_rules::lang::month_year(lang, start, 0.0), kshana_rules::lang::month_year(lang, end, 0.0));
        let k = kind_phrase_in(lang, kind);
        if lang == Lang::Ml {
            format!("{s} മുതൽ {e} വരെ നിങ്ങളുടെ {an} കാര്യത്തിൽ {k}? {e} ആകുമ്പോഴേക്ക് ഉണ്ട് എന്നോ ഇല്ല എന്നോ ഉത്തരം നൽകുക.")
        } else {
            format!("{s} से {e} के बीच क्या आपके {an} में {k}? {e} तक हाँ या नहीं में जवाब दें।")
        }
    } else {
        format!(
        "Did {} in your {} between {} and {}? Answer yes or no by {}.",
        kind_phrase(kind),
        a.name,
        month_year(start),
        month_year(end),
        month_year(end)
    )
    };
    let mut f = SealedForecast {
        id: String::new(),
        created_at: at_creation,
        area: a.name.to_string(),
        kind,
        window_start: start,
        window_end: end,
        probability,
        basis,
        question,
        resolved: None,
    };
    f.id = f.content_id();
    Ok(f)
}

/// Record the outcome of a forecast.
pub fn resolve(forecast: &SealedForecast, happened: bool, when: Instant) -> SealedForecast {
    SealedForecast {
        resolved: Some((happened, when)),
        ..forecast.clone()
    }
}

/// [`metrics`] over the resolved forecasts only, with cube-root bins.
pub fn sealed_brier(forecasts: &[SealedForecast]) -> Metrics {
    let resolved: Vec<&SealedForecast> = forecasts.iter().filter(|f| f.is_resolved()).collect();
    let ps: Vec<f64> = resolved.iter().map(|f| f.probability).collect();
    let os: Vec<bool> = resolved
        .iter()
        .map(|f| f.resolved.is_some_and(|r| r.0))
        .collect();
    metrics(&ps, &os, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};

    #[test]
    fn kernel_seal_round_trips_and_detects_tampering() {
        let Some((spk, ch)) = kochi() else { return };
        let created = noon(2026, 10, 1);
        let f = seal(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            created,
            "career",
            EventKind::Gain,
            (noon(2027, 1, 1), noon(2027, 7, 1)),
        )
        .unwrap();
        assert_eq!(f.area, "Career");
        assert_eq!(f.id.len(), 64);
        assert!(f.verify());
        assert!((0.0..=1.0).contains(&f.probability));
        assert_eq!(f.basis.len(), 4);
        assert!(f.basis[0].starts_with("KS-K-0001"), "{}", f.basis[0]);
        assert!(f.question.contains("Jan 2027") && f.question.contains("Jul 2027"));
        assert!(!f.is_resolved());
        let line = f.to_json_line();
        assert!(line.contains("\"basis\":["), "{line}");
        assert!(
            line.ends_with("\"happened\":null,\"resolved_at\":null}"),
            "{line}"
        );
        let back = SealedForecast::from_json_line(&line).unwrap();
        assert_eq!(back, f);
        assert!(back.verify());
        // Sealed in the reader's language: the same numbers, the reader's words.
        for lang in [Lang::Ml, Lang::Hi] {
            let g = seal_in(&spk, &ch, Ayanamsa::Lahiri, created, "career", EventKind::Gain, (noon(2027, 1, 1), noon(2027, 7, 1)), lang).unwrap();
            assert!(g.verify());
            assert_eq!(g.probability, f.probability);
            assert_eq!(g.basis.len(), 4);
            assert!(g.basis[0].starts_with("KS-K-0001"));
            assert!(!g.question.contains("Did") && !g.question.contains("Career"), "{}", g.question);
            assert_eq!(SealedForecast::from_json_line(&g.to_json_line()).unwrap(), g);
        }
        // Tampering with the claim after sealing shows.
        let mut tampered = back.clone();
        tampered.probability = 0.99;
        assert!(!tampered.verify());
        let edited = line.replace("\"area\":\"Career\"", "\"area\":\"Wealth\"");
        assert!(!SealedForecast::from_json_line(&edited).unwrap().verify());
        // Resolution does not change the id.
        let r = resolve(&f, true, noon(2027, 8, 1));
        assert!(r.verify());
        assert_eq!(r.resolved, Some((true, noon(2027, 8, 1))));
        let rline = r.to_json_line();
        assert!(rline.contains("\"happened\":true"));
        let rback = SealedForecast::from_json_line(&rline).unwrap();
        assert_eq!(rback, r);
        // Refusals.
        assert!(matches!(
            seal(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                created,
                "Career",
                EventKind::Gain,
                (noon(2026, 9, 1), noon(2027, 1, 1))
            ),
            Err(LedgerError::Refused(_))
        ));
        assert!(matches!(
            seal(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                created,
                "Career",
                EventKind::Gain,
                (noon(2027, 1, 1), noon(2027, 1, 1))
            ),
            Err(LedgerError::Refused(_))
        ));
        assert!(matches!(
            seal(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                created,
                "Pets",
                EventKind::Gain,
                (noon(2027, 1, 1), noon(2027, 7, 1))
            ),
            Err(LedgerError::UnknownArea(_))
        ));
        // Brier over the resolved only.
        let m = sealed_brier(&[f.clone(), r.clone(), resolve(&f, false, noon(2027, 8, 2))]);
        assert_eq!(m.n, 2);
        assert_eq!(m.positives, 1);
        let want = ((f.probability - 1.0).powi(2) + f.probability.powi(2)) / 2.0;
        assert!((m.brier - want).abs() < 1e-12);
        assert_eq!(sealed_brier(&[]).n, 0);
    }

    #[test]
    fn json_line_tolerates_hand_written_shapes() {
        let f = SealedForecast::from_json_line(
            "{\"id\":\"x\",\"created_at\":\"2026-01-01\",\"area\":\"wealth\",\"kind\":\"gain\",\"window_start\":\"2026-02-01\",\"window_end\":\"2026-03-01\",\"probability\":0.4}",
        )
        .unwrap();
        assert_eq!(f.area, "Wealth");
        assert!(f.basis.is_empty());
        assert!(f.question.is_empty());
        assert!(f.resolved.is_none());
        assert!(!f.verify());
        assert!(SealedForecast::from_json_line("{\"id\":\"x\"}").is_err());
        assert!(SealedForecast::from_json_line("nope").is_err());
    }
}
