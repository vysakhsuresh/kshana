//! Scoring one logged event against what the chart said about that instant.
//!
//! Three factors, each in `[0, 1]`, are oriented to the kind of event. For Began,
//! Ended, Gain and Milestone they measure favour: how many Vimshottari levels, from the
//! mahadasha down in unbroken order, had lords signifying the area's houses (the KP
//! timing funnel); how good the maha and antar lords were for the area (Parashari
//! period quality); and whether Jupiter and Saturn stood in the classically favourable
//! houses from the natal Moon. For Loss and Crisis they measure adversity: lords
//! signifying the area's negating houses, poor period quality, unfavourable transit
//! houses and a running Sade Sati. The fit is a fixed weighted sum of the three, so it
//! reads as the probability the engine would have given to "something of this kind
//! happens in this area now", and every factor is spelt out with its numbers.

use crate::event::{month_year, EventKind, LedgerError, LifeEvent};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_forecast::transits_at;
use kshana_rules::{
    agreeing_levels, area, period_quality, period_supports, significators, timing, Area, Facts,
    TimingLevel,
};
use kshana_rules::yogas::words;
use kshana_rules::Lang;
use kshana_time::Instant;
use kshana_vedic::{Ayanamsa, DashaLord};

/// Weight of the dasha factor in the fit.
pub const WEIGHT_DASHA: f64 = 0.5;
/// Weight of the Parashari factor in the fit.
pub const WEIGHT_PARASHARI: f64 = 0.3;
/// Weight of the transit factor in the fit.
pub const WEIGHT_TRANSIT: f64 = 0.2;
/// Houses from the natal Moon in which Jupiter's transit is favourable (Brihat Samhita
/// 104, Phaladeepika 26).
pub const JUPITER_GOOD_FROM_MOON: [u8; 5] = [2, 5, 7, 9, 11];
/// Houses from the natal Moon in which Saturn's transit is favourable (same sources).
pub const SATURN_GOOD_FROM_MOON: [u8; 3] = [3, 6, 11];
/// How much a running Sade Sati moves the transit factor: down for Health and Career
/// when the event is favourable, up for any Loss or Crisis.
pub const SADE_SATI_SHIFT: f64 = 0.25;
/// Levels of the Vimshottari funnel examined: maha, antar, pratyantar, sookshma, prana.
pub const FUNNEL_DEPTH: usize = 5;

const LEVEL_NAMES: [&str; 5] = ["mahadasha", "antardasha", "pratyantar", "sookshma", "prana"];

/// What the engine would have said about an event's area at its instant.
#[derive(Clone, Debug, PartialEq)]
pub struct EventScore {
    /// Consecutive funnel levels from the mahadasha down whose lords signify the area's
    /// houses (its negating houses for an adverse event), over [`FUNNEL_DEPTH`].
    pub dasha_support: f64,
    /// Mean Parashari period quality of the maha and antar lords for the area, or one
    /// minus that for an adverse event.
    pub parashari: f64,
    /// Jupiter's and Saturn's houses from the natal Moon against the classical table,
    /// shifted by a running Sade Sati; oriented like the other two.
    pub transit: f64,
    /// `WEIGHT_DASHA · dasha_support + WEIGHT_PARASHARI · parashari + WEIGHT_TRANSIT · transit`, in `[0, 1]`.
    pub fit: f64,
    /// True for Loss and Crisis: the factors above measure adversity, not favour.
    pub adverse: bool,
    /// Lords running at the instant, mahadasha first, down to prana. Empty when the
    /// instant lies outside the 120-year cycle from birth.
    pub running: Vec<DashaLord>,
    /// Raw mean period quality of the maha and antar lords, before orientation.
    pub period_quality: f64,
    /// Jupiter's sidereal house from the natal Moon, 1–12.
    pub jupiter_from_moon: u8,
    /// Saturn's sidereal house from the natal Moon, 1–12.
    pub saturn_from_moon: u8,
    /// Sade Sati phase: 0 none, 1 rising (Saturn in the 12th from the Moon), 2 peak
    /// (the 1st), 3 setting (the 2nd).
    pub sade_sati_phase: u8,
    /// Each factor as a sentence with its numbers, then the fit.
    pub explanation: Vec<String>,
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// A house ordinal in the reader's language: "7th", "ഏഴാം", "सप्तम".
pub(crate) fn ordinal_in(lang: Lang, h: u8) -> String {
    match lang {
        Lang::En => ordinal(h),
        _ => words::bhava(lang, h).to_string(),
    }
}

/// A dasha lord's name: "Saturn" (as the English always printed it), "ശനി", "शनि".
pub(crate) fn lord_in(lang: Lang, l: DashaLord) -> String {
    match lang {
        Lang::En => format!("{l:?}"),
        _ => words::graha(lang, l).to_string(),
    }
}

/// "Mar 2019" in English (as before); the month and year in the reader's language
/// otherwise, by the UTC date.
pub(crate) fn when_in(lang: Lang, at: Instant) -> String {
    match lang {
        Lang::En => month_year(at),
        _ => kshana_rules::lang::month_year(lang, at, 0.0),
    }
}

/// An area's name as shown: "Career", "ജോലി", "करियर".
pub(crate) fn area_word(lang: Lang, name: &str) -> String {
    let t = match name {
        "Career" => ["Career", "ജോലി", "करियर"],
        "Marriage" => ["Marriage", "വിവാഹം", "विवाह"],
        "Children" => ["Children", "മക്കൾ", "संतान"],
        "Wealth" => ["Wealth", "സമ്പത്ത്", "धन"],
        "Health" => ["Health", "ആരോഗ്യം", "सेहत"],
        "Property" => ["Property", "വീടും വസ്തുവും", "घर-संपत्ति"],
        "Education" => ["Education", "വിദ്യാഭ്യാസം", "शिक्षा"],
        "Family" => ["Family", "കുടുംബം", "परिवार"],
        "Foreign" => ["Foreign", "വിദേശം", "विदेश"],
        _ => return name.to_string(),
    };
    lang.pick(t).to_string()
}

/// An event kind's display name, capitalised in English: "Began", "തുടക്കം", "शुरुआत".
pub fn kind_word(lang: Lang, k: EventKind) -> &'static str {
    let t = match k {
        EventKind::Began => ["Began", "തുടക്കം", "शुरुआत"],
        EventKind::Ended => ["Ended", "അവസാനം", "अंत"],
        EventKind::Gain => ["Gain", "നേട്ടം", "लाभ"],
        EventKind::Loss => ["Loss", "നഷ്ടം", "हानि"],
        EventKind::Milestone => ["Milestone", "നാഴികക്കല്ല്", "उपलब्धि"],
        EventKind::Crisis => ["Crisis", "പ്രതിസന്ധി", "संकट"],
    };
    lang.pick(t)
}

/// An event kind inside a sentence: lower case in English.
fn kind_lower(lang: Lang, k: EventKind) -> String {
    match lang {
        Lang::En => k.name().to_lowercase(),
        _ => kind_word(lang, k).to_string(),
    }
}

/// The five Vimshottari levels by name.
fn level_name(lang: Lang, i: usize) -> &'static str {
    let t = [
        ["mahadasha", "മഹാദശ", "महादशा"],
        ["antardasha", "അന്തർദശ", "अंतर्दशा"],
        ["pratyantar", "പ്രത്യന്തരദശ", "प्रत्यंतर दशा"],
        ["sookshma", "സൂക്ഷ്മദശ", "सूक्ष्म दशा"],
        ["prana", "പ്രാണദശ", "प्राण दशा"],
    ];
    lang.pick(t[i.min(4)])
}

/// The Sade Sati phase in words, 1 rising, 2 peak, 3 setting.
pub(crate) fn sade_phase_words(lang: Lang, phase: u8) -> &'static str {
    let t = match phase {
        1 => [
            "rising, Saturn in the 12th from the Moon",
            "ആരംഭം, ചന്ദ്രന്റെ പന്ത്രണ്ടാം ഭാവത്തിൽ ശനി",
            "आरंभ, चंद्र से द्वादश भाव में शनि",
        ],
        2 => ["peak, Saturn on the Moon", "മധ്യം, ചന്ദ്രനു മേൽ ശനി", "शिखर, चंद्र पर शनि"],
        _ => [
            "setting, Saturn in the 2nd from the Moon",
            "ഒടുക്കം, ചന്ദ്രന്റെ രണ്ടാം ഭാവത്തിൽ ശനി",
            "अंत, चंद्र से द्वितीय भाव में शनि",
        ],
    };
    lang.pick(t)
}

fn list_u8(v: &[u8]) -> String {
    words::houses(v)
}

fn unit(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// Whether a lord signifies any of the area's negating houses (KP denial).
fn signifies_negating(facts: &Facts, a: Area, lord: DashaLord) -> bool {
    significators(facts, lord)
        .iter()
        .any(|s| a.negating.contains(&s.house))
}

fn agreeing_negating(facts: &Facts, a: Area, funnel: &[TimingLevel]) -> usize {
    funnel
        .iter()
        .take_while(|l| signifies_negating(facts, a, l.period.lord))
        .count()
}

/// Position of a lord in [`DashaLord::ORDER`].
pub(crate) fn lord_index(l: DashaLord) -> usize {
    DashaLord::ORDER.iter().position(|x| *x == l).unwrap_or(0)
}

/// Whether each of the nine lords supports an area in one orientation: for a
/// favourable kind, it signifies a group house; for an adverse kind, a negating house.
/// This is the per-lord predicate behind the dasha factor, tabulated once so a sweep
/// over thousands of instants or cells costs nine lookups, not nine significator scans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LordTable {
    flags: [bool; 9],
}

impl LordTable {
    /// Tabulate the predicate for an area and orientation.
    pub(crate) fn new(facts: &Facts, a: Area, adverse: bool) -> Self {
        let flags = DashaLord::ORDER.map(|l| {
            if adverse {
                signifies_negating(facts, a, l)
            } else {
                !period_supports(facts, a, l).is_empty()
            }
        });
        Self { flags }
    }

    /// Whether `l` supports.
    pub(crate) fn supports(&self, l: DashaLord) -> bool {
        self.flags[lord_index(l)]
    }

    /// Consecutive supporting lords from the top of a chain.
    pub(crate) fn agreeing(&self, chain: &[DashaLord]) -> usize {
        chain.iter().take_while(|l| self.supports(**l)).count()
    }

    /// The share of any Vimshottari period's sub-periods, by duration, whose lords
    /// support: `Σ years of supporting lords / 120`. Sub-periods at every level are
    /// proportioned by their lords' years, so this is also the fraction of time within
    /// any period during which the next level down supports.
    pub(crate) fn year_fraction(&self) -> f64 {
        DashaLord::ORDER
            .iter()
            .filter(|l| self.supports(**l))
            .map(|l| l.years() as f64)
            .sum::<f64>()
            / 120.0
    }
}

/// Jupiter's and Saturn's houses from the natal Moon at an instant and the Sade Sati
/// phase: the sky's part of a score before it is oriented to an area and a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitSample {
    /// Jupiter's sidereal house from the natal Moon, 1–12.
    pub jupiter_from_moon: u8,
    /// Saturn's sidereal house from the natal Moon, 1–12.
    pub saturn_from_moon: u8,
    /// Sade Sati phase: 0 none, 1 rising, 2 peak, 3 setting.
    pub sade_sati_phase: u8,
}

/// Sample the sky at an instant for a natal chart.
pub fn transit_sample(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<TransitSample, LedgerError> {
    let transits = transits_at(spk, natal, at, ayanamsa)?;
    let house_from_moon = |body: &str| {
        transits
            .iter()
            .find(|t| t.body == body && t.natal == "Moon")
            .map_or(0, |t| t.house)
    };
    let jupiter_from_moon = house_from_moon("Jupiter");
    let saturn_from_moon = house_from_moon("Saturn");
    let sade_sati_phase = match saturn_from_moon {
        12 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    };
    Ok(TransitSample {
        jupiter_from_moon,
        saturn_from_moon,
        sade_sati_phase,
    })
}

/// The transit factor of a sample for an area and kind: half a point each for
/// Jupiter in [`JUPITER_GOOD_FROM_MOON`] and Saturn in [`SATURN_GOOD_FROM_MOON`]
/// (their complements for an adverse kind), then the [`SADE_SATI_SHIFT`] up for any
/// adverse kind and down for a favourable one in Health or Career. This is exactly
/// the transit part of [`score_at`].
pub fn transit_factor(sample: &TransitSample, a: Area, kind: EventKind) -> f64 {
    let adverse = kind.is_adverse();
    let j_good = JUPITER_GOOD_FROM_MOON.contains(&sample.jupiter_from_moon);
    let s_good = SATURN_GOOD_FROM_MOON.contains(&sample.saturn_from_moon);
    let mut transit = if adverse {
        0.5 * (unit(!j_good) + unit(!s_good))
    } else {
        0.5 * (unit(j_good) + unit(s_good))
    };
    if sample.sade_sati_phase != 0 {
        if adverse {
            transit = (transit + SADE_SATI_SHIFT).min(1.0);
        } else if matches!(a.name, "Health" | "Career") {
            transit = (transit - SADE_SATI_SHIFT).max(0.0);
        }
    }
    transit
}

/// The Parashari factor for the lords running at an instant (mahadasha first): the
/// mean period quality of the first two, or one minus that for an adverse kind, with
/// the raw mean second. Both are 0 when no lord is running.
pub fn parashari_factor(
    facts: &Facts,
    a: Area,
    kind: EventKind,
    running: &[DashaLord],
) -> (f64, f64) {
    let qualities: Vec<f64> = running
        .iter()
        .take(2)
        .map(|l| period_quality(facts, a, *l).0)
        .collect();
    if qualities.is_empty() {
        return (0.0, 0.0);
    }
    let mean = qualities.iter().sum::<f64>() / qualities.len() as f64;
    let oriented = if kind.is_adverse() { 1.0 - mean } else { mean };
    (oriented, mean)
}

/// `WEIGHT_DASHA · dasha + WEIGHT_PARASHARI · parashari + WEIGHT_TRANSIT · transit`,
/// clamped to `[0, 1]`: the fit of [`score_at`] from its three factors.
pub fn combine_fit(dasha_support: f64, parashari: f64, transit: f64) -> f64 {
    (WEIGHT_DASHA * dasha_support + WEIGHT_PARASHARI * parashari + WEIGHT_TRANSIT * transit)
        .clamp(0.0, 1.0)
}

/// Score a logged event. Fails for an area name outside [`kshana_rules::AREAS`] or
/// an instant the ephemeris does not cover.
pub fn score_event(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    ev: &LifeEvent,
) -> Result<EventScore, LedgerError> {
    score_event_in(spk, natal, ayanamsa, ev, Lang::En)
}

/// [`score_event`] with the explanation in the reader's language.
pub fn score_event_in(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    ev: &LifeEvent,
    lang: Lang,
) -> Result<EventScore, LedgerError> {
    let a = area(&ev.area).ok_or_else(|| LedgerError::UnknownArea(ev.area.clone()))?;
    let facts = Facts::from_chart(natal);
    score_at_in(spk, natal, &facts, ayanamsa, a, ev.kind, ev.at, lang)
}

/// Score an area and kind at an instant, with the natal [`Facts`] already derived (so
/// a sweep over many instants derives them once).
pub fn score_at(
    spk: &Spk,
    natal: &Chart,
    facts: &Facts,
    ayanamsa: Ayanamsa,
    a: Area,
    kind: EventKind,
    at: Instant,
) -> Result<EventScore, LedgerError> {
    score_at_in(spk, natal, facts, ayanamsa, a, kind, at, Lang::En)
}

/// [`score_at`] with the explanation in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn score_at_in(
    spk: &Spk,
    natal: &Chart,
    facts: &Facts,
    ayanamsa: Ayanamsa,
    a: Area,
    kind: EventKind,
    at: Instant,
    lang: Lang,
) -> Result<EventScore, LedgerError> {
    let adverse = kind.is_adverse();
    let when = when_in(lang, at);
    let area_w = area_word(lang, a.name);
    let kind_w = kind_lower(lang, kind);
    let moon = natal.bodies[1].sidereal;
    let mut explanation = Vec::with_capacity(5);

    // 1. The KP timing funnel.
    let funnel = timing(facts, a, moon, natal.instant, at, FUNNEL_DEPTH);
    let running: Vec<DashaLord> = funnel.iter().map(|l| l.period.lord).collect();
    let agree = if adverse {
        agreeing_negating(facts, a, &funnel)
    } else {
        agreeing_levels(&funnel)
    };
    let dasha_support = agree as f64 / FUNNEL_DEPTH as f64;
    if funnel.is_empty() {
        explanation.push(kshana_rules::say3!(
            lang,
            "In {when}, no Vimshottari period was running (the instant lies outside the 120-year cycle from birth): dasha support 0.00.",
            "{when}-ൽ ഒരു വിംശോത്തരി ദശയും നടന്നിരുന്നില്ല (ആ സമയം ജനനം മുതലുള്ള 120 വർഷ ചക്രത്തിന് പുറത്താണ്): ദശാ പിന്തുണ 0.00.",
            "{when} में कोई विंशोत्तरी दशा नहीं चल रही थी (वह क्षण जन्म से 120 वर्ष के चक्र के बाहर है): दशा समर्थन 0.00।"
        ));
    } else {
        let per_level: Vec<String> = funnel
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let s = if adverse {
                    signifies_negating(facts, a, l.period.lord)
                } else {
                    !l.supports.is_empty()
                };
                match lang {
                    Lang::En => format!(
                        "{:?} {}: {}",
                        l.period.lord,
                        LEVEL_NAMES[i.min(4)],
                        yes_no(s)
                    ),
                    _ => format!(
                        "{} {}: {}",
                        lord_in(lang, l.period.lord),
                        level_name(lang, i),
                        lang.pick(["", if s { "ഉണ്ട്" } else { "ഇല്ല" }, if s { "हाँ" } else { "नहीं" }])
                    ),
                }
            })
            .collect();
        let (houses, what, denial) = if adverse {
            (a.negating, "negating houses", ", the houses that deny it")
        } else {
            (a.group, "houses", "")
        };
        let levels = per_level.join(", ");
        let hl = list_u8(houses);
        explanation.push(match lang {
            Lang::En => format!(
                "In {when}, {agree} of {FUNNEL_DEPTH} Vimshottari levels in unbroken order from the mahadasha had lords signifying {}'s {what} {houses:?}{denial} ({}): {}dasha support {dasha_support:.2}.",
                a.name,
                levels,
                if adverse { "adverse " } else { "" }
            ),
            Lang::Ml => format!(
                "{when}-ൽ, മഹാദശ മുതൽ മുറിയാതെ താഴോട്ടുള്ള {FUNNEL_DEPTH} വിംശോത്തരി തലങ്ങളിൽ {agree} എണ്ണത്തിന്റെ നാഥന്മാർ {area_w} എന്ന മേഖലയുടെ {} ({hl}){} സൂചിപ്പിച്ചു ({levels}): {}ദശാ പിന്തുണ {dasha_support:.2}.",
                if adverse { "നിഷേധ ഭാവങ്ങൾ" } else { "ഭാവങ്ങൾ" },
                if adverse { ", അതിനെ തടയുന്ന ഭാവങ്ങൾ," } else { "" },
                if adverse { "പ്രതികൂല " } else { "" }
            ),
            Lang::Hi => format!(
                "{when} में, महादशा से बिना टूटे क्रम में {FUNNEL_DEPTH} में से {agree} विंशोत्तरी स्तरों के स्वामी {area_w} के {} ({hl}){} के कारक थे ({levels}): {}दशा समर्थन {dasha_support:.2}।",
                if adverse { "निषेधक भावों" } else { "भावों" },
                if adverse { ", यानी इसे रोकने वाले भावों," } else { "" },
                if adverse { "प्रतिकूल " } else { "" }
            ),
        });
    }

    // 2. Parashari quality of the maha and antar lords.
    let qualities: Vec<(DashaLord, f64)> = running
        .iter()
        .take(2)
        .map(|l| (*l, period_quality(facts, a, *l).0))
        .collect();
    let period_quality_mean = if qualities.is_empty() {
        0.0
    } else {
        qualities.iter().map(|q| q.1).sum::<f64>() / qualities.len() as f64
    };
    let parashari = if qualities.is_empty() {
        0.0
    } else if adverse {
        1.0 - period_quality_mean
    } else {
        period_quality_mean
    };
    if qualities.is_empty() {
        explanation.push(
            lang.pick([
                "No period lord to judge by Parashari period quality: parashari 0.00.",
                "പരാശരി ദശാഗുണം നോക്കാൻ ഒരു ദശാനാഥനും ഇല്ല: പരാശരി 0.00.",
                "पाराशरी दशा-गुण से परखने को कोई दशा-स्वामी नहीं: पाराशरी 0.00।",
            ])
            .to_string(),
        );
    } else {
        let parts: Vec<String> = qualities
            .iter()
            .enumerate()
            .map(|(i, (l, q))| match lang {
                Lang::En => {
                    let level = if i == 0 { "Mahadasha" } else { "antardasha" };
                    format!("{level} lord {l:?} scored {q:.2}")
                }
                Lang::Ml => format!(
                    "{}നാഥൻ {} {q:.2}",
                    if i == 0 { "മഹാദശാ" } else { "അന്തർദശാ" },
                    lord_in(lang, *l)
                ),
                Lang::Hi => format!(
                    "{} स्वामी {} को {q:.2}",
                    if i == 0 { "महादशा" } else { "अंतर्दशा" },
                    lord_in(lang, *l)
                ),
            })
            .collect();
        let tail = if adverse {
            kshana_rules::say3!(
                lang,
                "mean {period_quality_mean:.2}; a {kind_w} is supported by a poor period, so this counts as 1 − {period_quality_mean:.2} = {parashari:.2}.",
                "ശരാശരി {period_quality_mean:.2}; {kind_w} എന്നതിന് താങ്ങാകുന്നത് മോശം ദശയാണ്, അതിനാൽ ഇത് 1 − {period_quality_mean:.2} = {parashari:.2} ആയി എണ്ണുന്നു.",
                "औसत {period_quality_mean:.2}; {kind_w} को कमज़ोर दशा का सहारा मिलता है, इसलिए यह 1 − {period_quality_mean:.2} = {parashari:.2} गिना गया।"
            )
        } else {
            kshana_rules::say3!(lang, "mean {parashari:.2}.", "ശരാശരി {parashari:.2}.", "औसत {parashari:.2}।")
        };
        explanation.push(match lang {
            Lang::En => format!(
                "{} for {} by Parashari period quality: {tail}",
                parts.join(" and "),
                a.name
            ),
            Lang::Ml => format!(
                "പരാശരി ദശാഗുണം പ്രകാരം {area_w} എന്ന മേഖലയ്ക്ക് {}: {tail}",
                parts.join(", ")
            ),
            Lang::Hi => format!(
                "पाराशरी दशा-गुण से {area_w} के लिए {}: {tail}",
                parts.join(" और ")
            ),
        });
    }

    // 3. Jupiter and Saturn from the natal Moon, and Sade Sati.
    let sample = transit_sample(spk, natal, ayanamsa, at)?;
    let TransitSample {
        jupiter_from_moon,
        saturn_from_moon,
        sade_sati_phase,
    } = sample;
    let j_good = JUPITER_GOOD_FROM_MOON.contains(&jupiter_from_moon);
    let s_good = SATURN_GOOD_FROM_MOON.contains(&saturn_from_moon);
    let mut transit = if adverse {
        0.5 * (unit(!j_good) + unit(!s_good))
    } else {
        0.5 * (unit(j_good) + unit(s_good))
    };
    let favour_in = |good: bool| {
        if good {
            lang.pick(["favourable", "അനുകൂലം", "अनुकूल"])
        } else {
            lang.pick(["unfavourable", "പ്രതികൂലം", "प्रतिकूल"])
        }
    };
    let favour = |good: bool| if good { "favourable" } else { "unfavourable" };
    let (jo, so) = (ordinal_in(lang, jupiter_from_moon), ordinal_in(lang, saturn_from_moon));
    let (jf, sf) = (favour_in(j_good), favour_in(s_good));
    let (jl, sl) = (list_u8(&JUPITER_GOOD_FROM_MOON), list_u8(&SATURN_GOOD_FROM_MOON));
    if lang != Lang::En {
        let adv = match (lang, adverse) {
            (_, false) => String::new(),
            (Lang::Ml, true) => format!("{kind_w} ആയതിനാൽ പ്രതികൂല സ്ഥാനങ്ങളാണ് എണ്ണുന്നത്; "),
            (_, true) => format!("{kind_w} के लिए प्रतिकूल स्थान गिने जाते हैं, इसलिए "),
        };
        explanation.push(kshana_rules::say3!(
            lang,
            "",
            "പ്രമാണ പട്ടിക പ്രകാരം ജന്മചന്ദ്രനിൽ നിന്ന് {jo} ഭാവത്തിൽ വ്യാഴവും ({jf}) {so} ഭാവത്തിൽ ശനിയും ({sf}) സഞ്ചരിച്ചു (വ്യാഴം {jl} ഭാവങ്ങളിലും ശനി {sl} ഭാവങ്ങളിലും ഗുണം ചെയ്യും): {adv}ഗോചരം {transit:.2}.",
            "शास्त्रीय तालिका के अनुसार गुरु जन्म-चंद्र से {jo} भाव में ({jf}) और शनि {so} भाव में ({sf}) गोचर कर रहे थे (गुरु {jl} भावों में, शनि {sl} भावों में शुभ फल देता है): {adv}गोचर {transit:.2}।"
        ));
    } else {
    explanation.push(format!(
        "Jupiter transited the {} from the natal Moon ({}) and Saturn the {} ({}) by the classical table (Jupiter gives in {:?}, Saturn in {:?}): {}transit {transit:.2}.",
        ordinal(jupiter_from_moon),
        favour(j_good),
        ordinal(saturn_from_moon),
        favour(s_good),
        JUPITER_GOOD_FROM_MOON,
        SATURN_GOOD_FROM_MOON,
        if adverse {
            format!(
                "the unfavourable placements count for a {}, so ",
                kind.name().to_lowercase()
            )
        } else {
            String::new()
        }
    ));
    }
    if sade_sati_phase != 0 {
        let phase_words = sade_phase_words(lang, sade_sati_phase);
        let before = transit;
        if adverse {
            transit = (transit + SADE_SATI_SHIFT).min(1.0);
            explanation.push(kshana_rules::say3!(
                lang,
                "Sade Sati was running (phase {sade_sati_phase} of 3: {phase_words}), which counts for a {kind_w} (+{SADE_SATI_SHIFT:.2}, capped at 1): transit {before:.2} → {transit:.2}.",
                "ഏഴരശ്ശനി നടക്കുകയായിരുന്നു (3-ൽ {sade_sati_phase}-ാം ഘട്ടം: {phase_words}), ഇത് {kind_w} എന്നതിന് അനുകൂലമായി എണ്ണുന്നു (+{SADE_SATI_SHIFT:.2}, പരമാവധി 1): ഗോചരം {before:.2} → {transit:.2}.",
                "साढ़ेसाती चल रही थी (3 में से चरण {sade_sati_phase}: {phase_words}), जो {kind_w} के पक्ष में गिनी जाती है (+{SADE_SATI_SHIFT:.2}, अधिकतम 1): गोचर {before:.2} → {transit:.2}।"
            ));
        } else if matches!(a.name, "Health" | "Career") {
            transit = (transit - SADE_SATI_SHIFT).max(0.0);
            explanation.push(kshana_rules::say3!(
                lang,
                "Sade Sati was running (phase {sade_sati_phase} of 3: {phase_words}), which counts against {area_w} (−{SADE_SATI_SHIFT:.2}, floored at 0): transit {before:.2} → {transit:.2}.",
                "ഏഴരശ്ശനി നടക്കുകയായിരുന്നു (3-ൽ {sade_sati_phase}-ാം ഘട്ടം: {phase_words}), ഇത് {area_w} എന്ന മേഖലയ്ക്ക് എതിരായി എണ്ണുന്നു (−{SADE_SATI_SHIFT:.2}, കുറഞ്ഞത് 0): ഗോചരം {before:.2} → {transit:.2}.",
                "साढ़ेसाती चल रही थी (3 में से चरण {sade_sati_phase}: {phase_words}), जो {area_w} के विरुद्ध गिनी जाती है (−{SADE_SATI_SHIFT:.2}, न्यूनतम 0): गोचर {before:.2} → {transit:.2}।"
            ));
        } else {
            explanation.push(kshana_rules::say3!(
                lang,
                "Sade Sati was running (phase {sade_sati_phase} of 3: {phase_words}); it bears on Health and Career, not on {area_w}, so the transit stays {transit:.2}.",
                "ഏഴരശ്ശനി നടക്കുകയായിരുന്നു (3-ൽ {sade_sati_phase}-ാം ഘട്ടം: {phase_words}); ഇത് ആരോഗ്യത്തെയും ജോലിയെയുമാണ് ബാധിക്കുക, {area_w} എന്ന മേഖലയെ അല്ല, അതിനാൽ ഗോചരം {transit:.2} ആയി തുടരുന്നു.",
                "साढ़ेसाती चल रही थी (3 में से चरण {sade_sati_phase}: {phase_words}); इसका असर सेहत और करियर पर होता है, {area_w} पर नहीं, इसलिए गोचर {transit:.2} ही रहा।"
            ));
        }
    }

    // 4. The fit.
    debug_assert_eq!(transit, transit_factor(&sample, a, kind));
    debug_assert_eq!(
        (parashari, period_quality_mean),
        parashari_factor(facts, a, kind, &running)
    );
    let fit = combine_fit(dasha_support, parashari, transit);
    let sum = kshana_rules::say3!(
        lang,
        "fit {fit:.2} = {WEIGHT_DASHA} × {dasha_support:.2} (dasha) + {WEIGHT_PARASHARI} × {parashari:.2} (Parashari) + {WEIGHT_TRANSIT} × {transit:.2} (transit).",
        "ചേർച്ച {fit:.2} = {WEIGHT_DASHA} × {dasha_support:.2} (ദശ) + {WEIGHT_PARASHARI} × {parashari:.2} (പരാശരി) + {WEIGHT_TRANSIT} × {transit:.2} (ഗോചരം).",
        "मेल {fit:.2} = {WEIGHT_DASHA} × {dasha_support:.2} (दशा) + {WEIGHT_PARASHARI} × {parashari:.2} (पाराशरी) + {WEIGHT_TRANSIT} × {transit:.2} (गोचर)।"
    );
    explanation.push(if adverse {
        kshana_rules::say3!(
            lang,
            "For a {kind_w}, support means the running periods were adverse for {area_w}, so each factor above measures adversity: {sum}",
            "{kind_w} ആയതിനാൽ, ഇവിടെ പിന്തുണ എന്നാൽ നടന്ന ദശകൾ {area_w} എന്ന മേഖലയ്ക്ക് പ്രതികൂലമായിരുന്നു എന്നാണ്; അതിനാൽ മുകളിലെ ഓരോ ഘടകവും പ്രതികൂലതയാണ് അളക്കുന്നത്: {sum}",
            "{kind_w} के लिए समर्थन का अर्थ है कि चल रही दशाएँ {area_w} के प्रतिकूल थीं, इसलिए ऊपर का हर घटक प्रतिकूलता मापता है: {sum}"
        )
    } else {
        kshana_rules::say3!(
            lang,
            "Read as the chance the engine gave to something happening in {area_w} in {when}: {sum}",
            "{when}-ൽ {area_w} എന്ന മേഖലയിൽ എന്തെങ്കിലും സംഭവിക്കാൻ ക്ഷണ നൽകിയ സാധ്യതയായി ഇതു വായിക്കാം: {sum}",
            "इसे {when} में {area_w} में कुछ होने की उस संभावना की तरह पढ़ें जो क्षण ने दी थी: {sum}"
        )
    });

    Ok(EventScore {
        dasha_support,
        parashari,
        transit,
        fit,
        adverse,
        running,
        period_quality: period_quality_mean,
        jupiter_from_moon,
        saturn_from_moon,
        sade_sati_phase,
        explanation,
    })
}

fn lowercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

fn kind_phrase(kind: EventKind, area_name: &str) -> String {
    match kind {
        EventKind::Began => format!("something began in your {area_name}"),
        EventKind::Ended => format!("something ended in your {area_name}"),
        EventKind::Gain => format!("you gained in {area_name}"),
        EventKind::Loss => format!("you suffered a loss in {area_name}"),
        EventKind::Milestone => format!("you reached a {area_name} milestone"),
        EventKind::Crisis => format!("a {area_name} crisis struck"),
    }
}

/// One plain sentence for a scored event, e.g. "When you changed jobs in Mar 2019,
/// Saturn's sub-period in Jupiter's mahadasha carried Career at 0.72 and Jupiter
/// transited your 11th from the Moon while Saturn crossed your 4th: a fit of 0.81."
pub fn fit_words(score: &EventScore, ev: &LifeEvent) -> String {
    fit_words_in(score, ev, Lang::En)
}

fn kind_phrase_in(lang: Lang, kind: EventKind, area_w: &str) -> String {
    match (lang, kind) {
        (Lang::Ml, EventKind::Began) => format!("{area_w} എന്ന മേഖലയിൽ ഒന്ന് തുടങ്ങി"),
        (Lang::Ml, EventKind::Ended) => format!("{area_w} എന്ന മേഖലയിൽ ഒന്ന് അവസാനിച്ചു"),
        (Lang::Ml, EventKind::Gain) => format!("{area_w} എന്ന മേഖലയിൽ നേട്ടമുണ്ടായി"),
        (Lang::Ml, EventKind::Loss) => format!("{area_w} എന്ന മേഖലയിൽ നഷ്ടമുണ്ടായി"),
        (Lang::Ml, EventKind::Milestone) => format!("{area_w} എന്ന മേഖലയിൽ ഒരു നാഴികക്കല്ല് കടന്നു"),
        (Lang::Ml, EventKind::Crisis) => format!("{area_w} എന്ന മേഖലയിൽ ഒരു പ്രതിസന്ധി വന്നു"),
        (_, EventKind::Began) => format!("{area_w} में कुछ शुरू हुआ"),
        (_, EventKind::Ended) => format!("{area_w} में कुछ ख़त्म हुआ"),
        (_, EventKind::Gain) => format!("{area_w} में लाभ हुआ"),
        (_, EventKind::Loss) => format!("{area_w} में हानि हुई"),
        (_, EventKind::Milestone) => format!("{area_w} में एक उपलब्धि मिली"),
        (_, EventKind::Crisis) => format!("{area_w} में संकट आया"),
    }
}

/// [`fit_words`] in the reader's language. The person's own note is quoted as written.
pub fn fit_words_in(score: &EventScore, ev: &LifeEvent, lang: Lang) -> String {
    if lang != Lang::En {
        let when = when_in(lang, ev.at);
        let area_w = area_word(lang, &ev.area);
        let note = ev.note.trim();
        let what = if note.is_empty() {
            kind_phrase_in(lang, ev.kind, &area_w)
        } else {
            format!("“{note}”")
        };
        let q = score.period_quality;
        let kind_w = kind_lower(lang, ev.kind);
        let periods = match (score.running.first(), score.running.get(1)) {
            (Some(m), Some(a)) => {
                let (m, a) = (lord_in(lang, *m), lord_in(lang, *a));
                kshana_rules::say3!(lang, "", "{m} മഹാദശയിൽ {a} അന്തർദശ", "{m} महादशा में {a} की अंतर्दशा")
            }
            (Some(m), None) => {
                let m = lord_in(lang, *m);
                kshana_rules::say3!(lang, "", "{m} മഹാദശ", "{m} महादशा")
            }
            _ => String::new(),
        };
        let carried = if score.running.is_empty() {
            lang.pick(["", "ഒരു വിംശോത്തരി ദശയും നടന്നിരുന്നില്ല", "कोई विंशोत्तरी दशा नहीं चल रही थी"]).to_string()
        } else if score.adverse {
            kshana_rules::say3!(
                lang,
                "",
                "{periods} {area_w} എന്ന മേഖലയെ {q:.2}-ൽ നിർത്തി, {kind_w} വരുന്ന തരം ദശ",
                "{periods} ने {area_w} को {q:.2} पर छोड़ा, ठीक वैसी दशा जैसी {kind_w} माँगता है"
            )
        } else {
            kshana_rules::say3!(
                lang,
                "",
                "{periods} {area_w} എന്ന മേഖലയെ {q:.2}-ൽ താങ്ങി",
                "{periods} ने {area_w} को {q:.2} पर थामा"
            )
        };
        let (jo, so) = (ordinal_in(lang, score.jupiter_from_moon), ordinal_in(lang, score.saturn_from_moon));
        let mut sky = kshana_rules::say3!(
            lang,
            "",
            "ചന്ദ്രനിൽ നിന്ന് {jo} ഭാവത്തിൽ വ്യാഴവും {so} ഭാവത്തിൽ ശനിയും സഞ്ചരിച്ചു",
            "गुरु चंद्र से {jo} भाव में और शनि {so} भाव में गोचर कर रहे थे"
        );
        if score.sade_sati_phase != 0 {
            sky.push_str(lang.pick(["", " (ഏഴരശ്ശനിക്കാലം)", " (साढ़ेसाती में)"]));
        }
        let fit = score.fit;
        return kshana_rules::say3!(
            lang,
            "",
            "{when}-ൽ {what}: {carried}, {sky}; ചേർച്ച {fit:.2}.",
            "{when} में {what}: {carried}, और {sky}; मेल {fit:.2}।"
        );
    }
    let when = month_year(ev.at);
    let note = ev.note.trim();
    let what = if note.is_empty() {
        kind_phrase(ev.kind, &ev.area)
    } else {
        format!("you {}", lowercase_first(note))
    };
    let periods = match (score.running.first(), score.running.get(1)) {
        (Some(m), Some(a)) => format!("{a:?}'s sub-period in {m:?}'s mahadasha"),
        (Some(m), None) => format!("{m:?}'s mahadasha"),
        _ => "no Vimshottari period".to_string(),
    };
    let carried = if score.running.is_empty() {
        format!("was running for {}", ev.area)
    } else if score.adverse {
        format!(
            "left {} at {:.2}, the kind of period a {} asks for,",
            ev.area,
            score.period_quality,
            ev.kind.name().to_lowercase()
        )
    } else {
        format!("carried {} at {:.2}", ev.area, score.period_quality)
    };
    let mut sky = format!(
        "Jupiter transited your {} from the Moon while Saturn crossed your {}",
        ordinal(score.jupiter_from_moon),
        ordinal(score.saturn_from_moon)
    );
    if score.sade_sati_phase != 0 {
        sky.push_str(" under Sade Sati");
    }
    format!(
        "When {what} in {when}, {periods} {carried} and {sky}: a fit of {:.2}.",
        score.fit
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};

    fn three_events() -> Vec<LifeEvent> {
        vec![
            LifeEvent::new(
                noon(2013, 7, 1),
                "Career",
                EventKind::Began,
                0.9,
                "started my first job",
            )
            .unwrap(),
            LifeEvent::new(
                noon(2018, 11, 22),
                "Marriage",
                EventKind::Milestone,
                1.0,
                "got married",
            )
            .unwrap(),
            LifeEvent::new(noon(2020, 6, 15), "Health", EventKind::Crisis, 0.7, "").unwrap(),
        ]
    }

    #[test]
    fn three_events_are_scored_bounded_and_explained() {
        let Some((spk, ch)) = kochi() else { return };
        for ev in three_events() {
            let s = score_event(&spk, &ch, Ayanamsa::Lahiri, &ev).unwrap();
            for v in [
                s.dasha_support,
                s.parashari,
                s.transit,
                s.fit,
                s.period_quality,
            ] {
                assert!((0.0..=1.0).contains(&v), "{ev:?}: {s:?}");
            }
            assert_eq!(s.running.len(), FUNNEL_DEPTH);
            assert!((1..=12).contains(&s.jupiter_from_moon));
            assert!((1..=12).contains(&s.saturn_from_moon));
            assert!(s.sade_sati_phase <= 3);
            assert_eq!(s.adverse, ev.kind.is_adverse());
            assert!(s.explanation.len() >= 4, "{:?}", s.explanation);
            assert!(s.explanation.iter().all(|e| !e.is_empty()));
            let last = s.explanation.last().unwrap();
            assert!(last.contains(&format!("fit {:.2}", s.fit)), "{last}");
            if s.adverse {
                assert!(last.contains("adverse"), "{last}");
                assert!(s.explanation[0].contains("negating"));
                assert!(((1.0 - s.period_quality) - s.parashari).abs() < 1e-12);
            } else {
                assert!((s.period_quality - s.parashari).abs() < 1e-12);
            }
            let expect = (WEIGHT_DASHA * s.dasha_support
                + WEIGHT_PARASHARI * s.parashari
                + WEIGHT_TRANSIT * s.transit)
                .clamp(0.0, 1.0);
            assert!((s.fit - expect).abs() < 1e-12);
            let w = fit_words(&s, &ev);
            assert!(w.starts_with("When "), "{w}");
            assert!(w.contains("a fit of"), "{w}");
            assert!(w.contains(&ev.area), "{w}");
        }
        // The dated words carry the user's own note and the month.
        let ev = &three_events()[0];
        let s = score_event(&spk, &ch, Ayanamsa::Lahiri, ev).unwrap();
        let w = fit_words(&s, ev);
        assert!(
            w.starts_with("When you started my first job in Jul 2013, "),
            "{w}"
        );
        assert!(w.contains("mahadasha carried Career at"), "{w}");
    }

    /// Latin words of three or more letters, apart from the metric names kept on purpose.
    pub(crate) fn latin_words(s: &str) -> Vec<String> {
        s.split(|c: char| !c.is_ascii_alphabetic())
            .filter(|w| w.len() >= 3 && !matches!(*w, "Brier" | "ECE"))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn malayalam_and_hindi_carry_no_english() {
        let Some((spk, ch)) = kochi() else { return };
        for lang in [Lang::Ml, Lang::Hi] {
            for ev in three_events().into_iter().map(|mut e| {
                e.note.clear();
                e
            }) {
                let s = score_event_in(&spk, &ch, Ayanamsa::Lahiri, &ev, lang).unwrap();
                let en = score_event(&spk, &ch, Ayanamsa::Lahiri, &ev).unwrap();
                assert_eq!(s.fit, en.fit);
                assert_eq!(s.explanation.len(), en.explanation.len());
                for line in s.explanation.iter().chain([&fit_words_in(&s, &ev, lang)]) {
                    assert!(latin_words(line).is_empty(), "{lang:?}: {line}");
                }
            }
            let r = crate::retrodict_in(&spk, &ch, Ayanamsa::Lahiri, noon(2010, 1, 1), noon(2014, 1, 1), lang).unwrap();
            for p in &r {
                assert!(latin_words(&p.words).is_empty(), "{lang:?}: {}", p.words);
            }
            for k in EventKind::ALL {
                assert!(latin_words(kind_word(lang, k)).is_empty());
            }
        }
        // English is the old text.
        let r = crate::retrodict_in(&spk, &ch, Ayanamsa::Lahiri, noon(2010, 1, 1), noon(2014, 1, 1), Lang::En).unwrap();
        assert_eq!(r, crate::retrodict(&spk, &ch, Ayanamsa::Lahiri, noon(2010, 1, 1), noon(2014, 1, 1)).unwrap());
    }

    #[test]
    fn summaries_in_malayalam_and_hindi_carry_no_english() {
        use crate::{calibrate, calibration_verdict, metrics, metrics_words, surrogate_words, SurrogateResult};
        let m = metrics(&[0.9, 0.2, 0.7, 0.4, 0.6], &[true, false, true, false, false], 0);
        let m0 = metrics(&[], &[], 0);
        let sr = SurrogateResult { p: 0.1, real_mean: 0.6, shifted_means: vec![0.5, 0.7], shifts: 2, n: 3, words: String::new() };
        let c = calibrate(&[], &[0.3]);
        for lang in [Lang::Ml, Lang::Hi] {
            for w in [metrics_words(&m, lang), metrics_words(&m0, lang), surrogate_words(&sr, lang), calibration_verdict(&c, lang)] {
                assert!(!w.is_empty() && latin_words(&w).is_empty(), "{lang:?}: {w}");
            }
        }
        assert_eq!(metrics_words(&m, Lang::En), m.words);
        assert_eq!(calibration_verdict(&c, Lang::En), c.verdict);
    }

    #[test]
    fn unknown_area_and_out_of_kernel_instants_are_errors() {
        let Some((spk, ch)) = kochi() else { return };
        let mut ev = three_events()[0].clone();
        ev.area = "Pets".into();
        assert!(matches!(
            score_event(&spk, &ch, Ayanamsa::Lahiri, &ev),
            Err(LedgerError::UnknownArea(_))
        ));
        let far = LifeEvent::new(
            Instant::j2000().add_ns(2_000 * 365 * kshana_time::NS_PER_DAY),
            "Career",
            EventKind::Gain,
            1.0,
            "",
        )
        .unwrap();
        assert!(matches!(
            score_event(&spk, &ch, Ayanamsa::Lahiri, &far),
            Err(LedgerError::Ephemeris(_))
        ));
    }
}
