//! The full yoga catalogue and the Jaimini additions over the FFI: every classical
//! yoga a chart holds (`kshana_rules::yogas`), with its strength, the Vimshottari
//! periods that bring it to life and its plain meaning; and the eight-karaka scheme,
//! all twelve arudha padas and the karakamsa (`kshana_vedic::jaimini`).

use kshana_marga::SIGN_NAMES;
use kshana_rules::yogas::{chart_yogas, chart_yogas_in, YogaFound, CATALOG};
use kshana_rules::{Facts, Lang};
use kshana_time::{Instant, Utc};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{
    arudha_padas, chara_karakas, chara_karakas_8, varga, vimshottari, DashaLord, KARAKA_NAMES,
    KARAKA_NAMES_8,
};

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// A Vimshottari major period that activates a yoga.
#[derive(Clone, Debug, uniffi::Record)]
pub struct YogaPeriodRecord {
    /// The period's lord (expert).
    pub lord: String,
    /// Start.
    pub start: ExactInstant,
    /// End.
    pub end: ExactInstant,
}

/// One yoga found in a chart.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ChartYogaRecord {
    /// Stable key, e.g. `gaja_kesari`.
    pub key: String,
    /// Expert name, e.g. "Gaja Kesari" (never shown on the plain screens).
    pub yoga: String,
    /// Family key: nabhasa, lunar, solar, mahapurusha, raja, dhana, parivartana,
    /// viparita, named, hardship, pravrajya, jaimini (expert).
    pub group: String,
    /// Text and chapter (expert).
    pub source: String,
    /// The condition in the texts' terms (expert).
    pub rule: String,
    /// What fired it in this chart (expert).
    pub factors: Vec<String>,
    /// Strength 0..1.
    pub strength: f64,
    /// True for a gift, false for a difficulty.
    pub good: bool,
    /// Plain title.
    pub title: String,
    /// What it means for the person, in plain words.
    pub meaning: String,
    /// For a difficulty: humane words and what helps. Empty for a gift.
    pub care: String,
    /// When it is felt, in plain words.
    pub when: String,
    /// Dasha lords that activate it (expert).
    pub lords: Vec<String>,
    /// True when it works throughout life.
    pub lifelong: bool,
    /// The Vimshottari major periods of those lords in a 120-year cycle from birth.
    pub periods: Vec<YogaPeriodRecord>,
    /// Where it comes from (the yoga's own text and chapter, and the periods that bring
    /// it to life). Resolve with `source_lines`; never shown as is.
    pub source_keys: Vec<String>,
}

/// One catalogue entry, for an index page.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct YogaDefRecord {
    pub key: String,
    pub yoga: String,
    pub group: String,
    pub source: String,
    pub rule: String,
    pub title: String,
    pub meaning: String,
    pub care: String,
    pub good: bool,
}

/// Every yoga the engine can detect, with its source and plain meaning.
#[uniffi::export]
pub fn yoga_catalogue() -> Vec<YogaDefRecord> {
    CATALOG
        .iter()
        .map(|d| YogaDefRecord {
            key: d.key.into(),
            yoga: d.name.into(),
            group: d.group.key().into(),
            source: d.source.into(),
            rule: d.rule.into(),
            title: d.title.into(),
            meaning: d.meaning.into(),
            care: d.care.into(),
            good: d.good,
        })
        .collect()
}

fn year(i: Instant) -> i64 {
    Utc::from_instant(i).year
}

/// The plain "when" sentence from the activating periods.
fn when_words(y: &YogaFound, periods: &[(DashaLord, Instant, Instant)], now: Instant, lang: Lang) -> String {
    if y.lifelong {
        return lang.pick(["This runs through your whole life.", "ഇത് ജീവിതം മുഴുവൻ കൂടെയുണ്ട്.", "यह जीवन भर साथ रहता है।"]).into();
    }
    if let Some((_, _, e)) = periods.iter().find(|(_, s, e)| *s <= now && now < *e) {
        let e = year(*e);
        return kshana_rules::say3!(lang, "It is most active now, until {e}.", "ഇപ്പോഴാണ് ഇത് ഏറ്റവും സജീവം, {e} വരെ.", "यह अभी सबसे ज़्यादा सक्रिय है, {e} तक।");
    }
    if let Some((_, s, e)) = periods.iter().find(|(_, s, _)| *s > now) {
        let (s, e) = (year(*s), year(*e));
        return kshana_rules::say3!(lang, "It comes alive from {s} to {e}.", "{s} മുതൽ {e} വരെയാണ് ഇത് ഉണർന്നുവരുന്ന കാലം.", "{s} से {e} तक इसके जागने का समय है।");
    }
    match periods.last() {
        Some((_, s, e)) => {
            let (s, e) = (year(*s), year(*e));
            kshana_rules::say3!(lang, "Its strongest years were {s} to {e}; what it gave stays with you.", "{s} മുതൽ {e} വരെയായിരുന്നു ഇതിന്റെ ഏറ്റവും നല്ല വർഷങ്ങൾ; അത് തന്നത് ഇന്നും നിങ്ങളോടൊപ്പമുണ്ട്.", "इसके सबसे मज़बूत वर्ष {s} से {e} तक थे; जो इसने दिया, वह आपके साथ है।")
        }
        None => lang.pick(["It colours your life quietly rather than in one stretch of years.", "ഏതെങ്കിലും ഒരു കാലത്ത് മാത്രമല്ല, ജീവിതത്തിലുടനീളം ഇത് നിശ്ശബ്ദമായി നിറം പകരുന്നു.", "यह किसी एक दौर में नहीं, पूरे जीवन में चुपचाप रंग भरता है।"]).into(),
    }
}

fn record(y: &YogaFound, mahas: &[kshana_vedic::DashaPeriod], now: Instant) -> ChartYogaRecord {
    record_in(y, mahas, now, Lang::En)
}

fn record_in(y: &YogaFound, mahas: &[kshana_vedic::DashaPeriod], now: Instant, lang: Lang) -> ChartYogaRecord {
    let periods: Vec<(DashaLord, Instant, Instant)> = if y.lifelong {
        Vec::new()
    } else {
        mahas.iter().filter(|p| y.activators.contains(&p.lord)).map(|p| (p.lord, p.start, p.end)).collect()
    };
    ChartYogaRecord {
        key: y.def.key.into(),
        yoga: y.def.name_in(lang).into(),
        group: y.def.group.key().into(),
        source: y.def.source_in(lang).into(),
        rule: y.def.rule_in(lang).into(),
        factors: y.factors.clone(),
        strength: y.strength,
        good: y.def.good,
        title: y.def.title_in(lang).into(),
        meaning: y.def.meaning_in(lang).into(),
        care: y.def.care_in(lang).into(),
        when: when_words(y, &periods, now, lang),
        lords: y.activators.iter().map(|g| crate::lord_in(*g, lang)).collect(),
        lifelong: y.lifelong,
        periods: periods
            .iter()
            .map(|(l, s, e)| YogaPeriodRecord { lord: format!("{l:?}"), start: (*s).into(), end: (*e).into() })
            .collect(),
        source_keys: if y.lifelong {
            vec![format!("yoga.{}", y.def.key)]
        } else {
            vec![format!("yoga.{}", y.def.key), "bphs.dashas".into()]
        },
    }
}

/// One chara karaka.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KarakaFullRecord {
    pub rank: u8,
    pub name: String,
    pub graha: String,
    /// Degree in sign used for the ranking (Rahu's counted back from 30°).
    pub degree_in_sign: f64,
}

/// One arudha pada.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PadaFullRecord {
    /// "A1" (Pada lagna) .. "A12" (Upapada).
    pub label: String,
    pub house: u8,
    pub sign: u8,
    pub sign_name: String,
    /// Rasi house of the pada from the lagna.
    pub rasi_house: u8,
}

/// The Jaimini layer of a chart.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct JaiminiFullRecord {
    /// "seven" or "eight".
    pub scheme: String,
    pub karakas: Vec<KarakaFullRecord>,
    pub padas: Vec<PadaFullRecord>,
    /// The Atmakaraka's navamsa sign under the chosen scheme.
    pub karakamsa: u8,
    pub karakamsa_name: String,
    /// Jaimini raja yogas found (seven-karaka scheme).
    pub yogas: Vec<ChartYogaRecord>,
}

#[uniffi::export]
impl Engine {
    /// Every catalogued yoga in a natal chart, gifts first and strongest first, with
    /// the Vimshottari major periods that bring each to life and a plain "when" read
    /// against `now`. `lang` is "en", "ml" or "hi".
    pub fn chart_yogas(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        now: ExactInstant,
        lang: String,
    ) -> Result<Vec<ChartYogaRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        let mahas = vimshottari(f.planet(DashaLord::Moon).sidereal, ch.instant, YEAR_365_25_NS);
        let now: Instant = now.into();
        let mut v: Vec<ChartYogaRecord> = chart_yogas_in(&f, lang).iter().map(|y| record_in(y, &mahas, now, lang)).collect();
        v.sort_by(|a, b| {
            (!a.good, -a.strength)
                .partial_cmp(&(!b.good, -b.strength))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(v)
    }

    /// Chara karakas (seven- or eight-karaka scheme), all twelve arudha padas, the
    /// karakamsa and the Jaimini raja yogas of a natal chart.
    pub fn jaimini_full(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        eight_karakas: bool,
        now: ExactInstant,
    ) -> Result<JaiminiFullRecord, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        Ok(jaimini_of(&f, eight_karakas, &vimshottari(f.planet(DashaLord::Moon).sidereal, ch.instant, YEAR_365_25_NS), now.into()))
    }
}

fn jaimini_of(f: &Facts, eight: bool, mahas: &[kshana_vedic::DashaPeriod], now: Instant) -> JaiminiFullRecord {
    use DashaLord::*;
    let lon = |g: DashaLord| f.planet(g).sidereal;
    let seven = [lon(Sun), lon(Moon), lon(Mars), lon(Mercury), lon(Jupiter), lon(Venus), lon(Saturn)];
    let (karakas, ak): (Vec<KarakaFullRecord>, DashaLord) = if eight {
        let k = chara_karakas_8([seven[0], seven[1], seven[2], seven[3], seven[4], seven[5], seven[6], lon(Rahu)]);
        (
            k.iter()
                .map(|x| KarakaFullRecord { rank: x.rank as u8, name: KARAKA_NAMES_8[x.rank].into(), graha: format!("{:?}", x.graha), degree_in_sign: x.degree_in_sign })
                .collect(),
            k[0].graha,
        )
    } else {
        let k = chara_karakas(seven);
        (
            k.iter()
                .map(|x| KarakaFullRecord { rank: x.rank as u8, name: KARAKA_NAMES[x.rank].into(), graha: format!("{:?}", x.graha), degree_in_sign: x.degree_in_sign })
                .collect(),
            k[0].graha,
        )
    };
    let signs: [usize; 7] = std::array::from_fn(|i| (seven[i] / 30.0).floor() as usize % 12);
    let padas = arudha_padas(f.lagna_sign as usize, &signs)
        .iter()
        .enumerate()
        .map(|(i, s)| PadaFullRecord {
            label: format!("A{}", i + 1),
            house: (i + 1) as u8,
            sign: *s as u8,
            sign_name: SIGN_NAMES[*s].into(),
            rasi_house: ((*s as i32 - f.lagna_sign as i32).rem_euclid(12) + 1) as u8,
        })
        .collect();
    let karakamsa = ((varga(lon(ak), 9) / 30.0).floor() as usize % 12) as u8;
    JaiminiFullRecord {
        scheme: if eight { "eight" } else { "seven" }.into(),
        karakas,
        padas,
        karakamsa,
        karakamsa_name: SIGN_NAMES[karakamsa as usize].into(),
        yogas: chart_yogas(f).iter().filter(|y| y.def.group.key() == "jaimini").map(|y| record(y, mahas, now)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    fn engine() -> Option<std::sync::Arc<Engine>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return None;
        }
        Engine::open(p.to_string_lossy().into_owned(), None).ok()
    }

    fn at(y: i64, mo: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    #[test]
    fn chart_yogas_over_the_boundary_are_plain_and_bounded() {
        let Some(e) = engine() else { return };
        let now = at(2026, 10, 20, 6);
        for (b, la, lo) in [(at(1990, 2, 20, 5), 9.9312, 76.2673), (at(1997, 9, 22, 13), 9.9312, 76.2673), (at(1975, 6, 3, 23), 28.6139, 77.2090)] {
            for code in ["ml", "hi"] {
                let lang = Lang::parse(code);
                for y in e.chart_yogas(b, la, lo, AyanamsaChoice::Lahiri, now, code.into()).unwrap() {
                    for t in [&y.title, &y.meaning, &y.care, &y.when] {
                        assert_eq!(kshana_rules::plain::jargon_in(lang, t), None, "{code} {}: {t}", y.key);
                    }
                    for t in y.factors.iter().chain([&y.when, &y.rule, &y.yoga]) {
                        let latin = t.split(|ch: char| !ch.is_ascii_alphabetic()).any(|w| w.len() >= 3 && w != "BPHS");
                        assert!(!latin, "{code} {}: {t}", y.key);
                    }
                }
            }
            let ys = e.chart_yogas(b, la, lo, AyanamsaChoice::Lahiri, now, "en".into()).unwrap();
            assert!(!ys.is_empty());
            // Exactly one Nabhasa yoga family member at least: Sankhya always forms
            // when nothing else does.
            assert!(ys.iter().any(|y| y.group == "nabhasa"));
            for y in &ys {
                assert!((0.0..=1.0).contains(&y.strength), "{} {}", y.key, y.strength);
                for t in [&y.title, &y.meaning, &y.care, &y.when] {
                    assert_eq!(kshana_rules::plain::jargon(t), None, "{}: {t}", y.key);
                }
                assert!(!y.when.is_empty());
                assert!(y.source_keys.iter().all(|k| kshana_rules::sources::known(k)), "{:?}", y.source_keys);
                assert_eq!(crate::sources::source_lines(y.source_keys.clone(), "en".into())[0].line, y.source);
                // Lifelong yogas carry no periods; the others always have some.
                assert_eq!(y.lifelong, y.periods.is_empty(), "{}", y.key);
            }
            // Gifts come first.
            let first_hard = ys.iter().position(|y| !y.good).unwrap_or(ys.len());
            assert!(ys[first_hard..].iter().all(|y| !y.good));
        }
        assert!(yoga_catalogue().len() >= 100);
    }

    #[test]
    fn jaimini_full_schemes_and_padas() {
        let Some(e) = engine() else { return };
        let b = at(1990, 10, 29, 12);
        let now = at(2026, 10, 20, 6);
        let j7 = e.jaimini_full(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, false, now).unwrap();
        let j8 = e.jaimini_full(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, true, now).unwrap();
        assert_eq!(j7.karakas.len(), 7);
        assert_eq!(j8.karakas.len(), 8);
        assert!(j8.karakas.iter().any(|k| k.graha == "Rahu"));
        assert_eq!(j7.padas.len(), 12);
        assert_eq!(j7.padas[0].label, "A1");
        // The Pada lagna and Upapada agree with the marga module's.
        let m = e.jaimini_extras(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into()).unwrap();
        assert_eq!(j7.padas[0].sign, m.pada_lagna.sign);
        assert_eq!(j7.padas[11].sign, m.upapada.sign);
        assert_eq!(j7.karakamsa, m.karakamsa.sign);
        for p in &j7.padas {
            // BPHS ch. 29: never in its own house or the 7th from it.
            assert!(p.rasi_house != p.house && p.rasi_house != (p.house + 5) % 12 + 1, "{p:?}");
        }
    }
}
