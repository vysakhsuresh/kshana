//! Dosha detectors: the curses of the previous birth (BPHS ch. 85), the badhaka of
//! Prasna Marga ch. 15, and the yoga and transit families (Kala Sarpa, Kemadruma,
//! Guru Chandala, Grahan, Mangal dosha, Sade Sati). Each detector returns a [`Dosha`]
//! whether or not it is present, with the indicators that fired, its source and the
//! remedy family it belongs to, so the remedy plan can tie every remedy to a located
//! cause.
//!
//! Structure of the curse yogas (BPHS ch. 85, Sharma numbering; Santhanam chs. 83–85
//! depending on the print): each curse is keyed to the natural karaka of the offended
//! party afflicting the 5th house or its lord: Rahu the serpents, the Sun the father,
//! the Moon the mother, Mars the brother, Mercury the maternal uncle, Jupiter the
//! Brahmin, Venus the wife, Saturn and Ketu the departed. The individual yogas are
//! transcribed from the research brief's reading of the Santhanam text and marked
//! "verify against Santhanam vol. 2" in the rule text; the Prasna Marga parent
//! indicators are snippet-verified. Houses are rasi houses (whole sign from the lagna).
//!
//! Every detector has an `_in` twin taking a [`Lang`]: the factors, the offended party,
//! the source and the remedy family come back in that language, while `name` and
//! `rule_id` stay English (they are identities other modules match on); use
//! [`Dosha::name_in`] for the shown name. The Malayalam and Hindi are gentle on
//! purpose: a dosha here is a debt to be mended, never a doom.

use crate::facts::Facts;
use crate::kuta::mangal_dosha;
use crate::parashari::{aspected_houses, dignity, is_benefic, yogas, Dignity};
use crate::plain::Lang;
use crate::say3;
use crate::yogas::words as w;
use kshana_time::Instant;
use kshana_vedic::{sign_lord, DashaLord};

/// One dosha, present or not.
#[derive(Clone, Debug, PartialEq)]
pub struct Dosha {
    /// Name, e.g. "Pitru dosha".
    pub name: &'static str,
    /// Stable rule id.
    pub rule_id: &'static str,
    /// The graha whose remedy family applies (the karaka, or the badhakesh).
    pub planet: DashaLord,
    /// Present in the chart.
    pub present: bool,
    /// Severity in `[0, 1]`: 0 when absent.
    pub severity: f64,
    /// The indicators that fired, in words.
    pub factors: Vec<String>,
    /// The text.
    pub source: &'static str,
    /// The remedy family, in words.
    pub remedy_family: &'static str,
    /// Whom the text says is offended or what the family concerns.
    pub offended: String,
}

/// Shown names by rule id: `[en, ml, hi]`.
const NAMES: &[(&str, [&str; 3])] = &[
    ("KS-R-SHAPA-01", ["Pitru dosha", "പിതൃദോഷം", "पितृ दोष"]),
    ("KS-R-SHAPA-02", ["Sarpa dosha", "സർപ്പദോഷം", "सर्प दोष"]),
    ("KS-R-SHAPA-03", ["Matru shapa", "മാതൃദോഷം", "मातृ दोष"]),
    ("KS-R-SHAPA-04", ["Bhratru shapa", "ഭ്രാതൃദോഷം", "भ्रातृ दोष"]),
    ("KS-R-SHAPA-05", ["Matula shapa", "മാതുലദോഷം", "मातुल दोष"]),
    ("KS-R-SHAPA-06", ["Brahma shapa", "ഗുരുദോഷം (ബ്രഹ്മശാപം)", "गुरु दोष (ब्रह्म शाप)"]),
    ("KS-R-SHAPA-07", ["Patni shapa", "പത്നീദോഷം", "पत्नी दोष"]),
    ("KS-R-SHAPA-08", ["Preta shapa", "പരേതരുടെ ദോഷം", "दिवंगतों का दोष"]),
    ("KS-R-BADH-01", ["Badhaka", "ബാധകം", "बाधक"]),
    ("KS-R-YOGA-02", ["Kala Sarpa", "കാളസർപ്പയോഗം", "कालसर्प योग"]),
    ("KS-R-YOGA-04", ["Kemadruma", "കേമദ്രുമയോഗം", "केमद्रुम योग"]),
    ("KS-R-YOGA-05", ["Guru Chandala", "ഗുരുചണ്ഡാലയോഗം", "गुरु चांडाल योग"]),
    ("KS-R-YOGA-06", ["Grahan dosha", "ഗ്രഹണദോഷം", "ग्रहण दोष"]),
    ("KS-R-YOGA-03", ["Mangal dosha", "ചൊവ്വാദോഷം", "मंगल दोष"]),
    ("KS-R-YOGA-01", ["Sade Sati", "ഏഴരശ്ശനി", "साढ़ेसाती"]),
];

impl Dosha {
    /// The shown name in the reader's language ("Mangal dosha", "ചൊവ്വാദോഷം", "मंगल दोष").
    pub fn name_in(&self, lang: Lang) -> &'static str {
        NAMES
            .iter()
            .find(|(id, _)| *id == self.rule_id)
            .map(|(_, t)| lang.pick(*t))
            .unwrap_or(self.name)
    }
}

fn build(
    name: &'static str,
    rule_id: &'static str,
    planet: DashaLord,
    source: &'static str,
    remedy_family: &'static str,
    offended: String,
    fired: Vec<(f64, String)>,
) -> Dosha {
    let severity: f64 = fired.iter().map(|(w, _)| w).sum::<f64>().clamp(0.0, 1.0);
    Dosha {
        name,
        rule_id,
        planet,
        present: !fired.is_empty(),
        severity,
        factors: fired.into_iter().map(|(_, f)| f).collect(),
        source,
        remedy_family,
        offended,
    }
}

fn lord_of(facts: &Facts, house: u8) -> DashaLord {
    facts.rasi_lords[(house - 1) as usize]
}

fn house_of(facts: &Facts, g: DashaLord) -> u8 {
    facts.planet(g).rasi_house
}

fn is_dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}

/// Natural malefics conjoined (same sign) with `g`, other than `g` itself.
fn malefics_with(facts: &Facts, g: DashaLord) -> Vec<DashaLord> {
    let s = facts.planet(g).sign;
    facts
        .planets
        .iter()
        .filter(|p| p.graha != g && p.sign == s && !is_benefic(facts, p.graha))
        .map(|p| p.graha)
        .collect()
}

fn conj(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    a != b && facts.planet(a).sign == facts.planet(b).sign
}

fn aspects(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    a != b && aspected_houses(a, house_of(facts, a)).contains(&house_of(facts, b))
}

/// Natural malefics aspecting `g` by graha drishti, other than `g`.
fn malefic_aspects_on(facts: &Facts, g: DashaLord) -> Vec<DashaLord> {
    facts
        .planets
        .iter()
        .filter(|p| p.graha != g && !is_benefic(facts, p.graha) && aspects(facts, p.graha, g))
        .map(|p| p.graha)
        .collect()
}

/// Afflicted: with a malefic, aspected by one, or in a dusthana.
fn afflicted(facts: &Facts, g: DashaLord, lang: Lang) -> Option<String> {
    let gw = w::graha(lang, g);
    let wl = malefics_with(facts, g);
    if !wl.is_empty() {
        let l = w::graha_list(lang, &wl);
        return Some(say3!(lang, "{g:?} is with {wl:?}", "{gw} {l} എന്നിവയോടൊപ്പം", "{gw} {l} के साथ"));
    }
    let a = malefic_aspects_on(facts, g);
    if !a.is_empty() {
        let l = w::graha_list(lang, &a);
        return Some(say3!(lang, "{g:?} is aspected by {a:?}", "{gw} {l} എന്നിവയുടെ ദൃഷ്ടിയിൽ", "{gw} पर {l} की दृष्टि"));
    }
    let h = house_of(facts, g);
    if is_dusthana(h) {
        let hw = w::bhava(lang, h);
        return Some(say3!(
            lang,
            "{g:?} is in rasi house {h}, a dusthana",
            "{gw} ദുഃസ്ഥാനമായ {hw} ഭാവത്തിൽ",
            "{gw} दुःस्थान, {hw} भाव में"
        ));
    }
    None
}

/// "Mars", "ചൊവ്വ", "मंगल".
fn gn(lang: Lang, g: DashaLord) -> &'static str {
    w::graha(lang, g)
}

/// Malayalam case forms of a graha's name, which change with the word's ending:
/// `[genitive "-ന്റെ", sociative "-ഓടൊപ്പം", accusative "-നെ", dative "-ന്"]`.
pub(crate) fn ml_forms(g: DashaLord) -> [&'static str; 4] {
    match g {
        DashaLord::Sun => ["സൂര്യന്റെ", "സൂര്യനോടൊപ്പം", "സൂര്യനെ", "സൂര്യന്"],
        DashaLord::Moon => ["ചന്ദ്രന്റെ", "ചന്ദ്രനോടൊപ്പം", "ചന്ദ്രനെ", "ചന്ദ്രന്"],
        DashaLord::Mars => ["ചൊവ്വയുടെ", "ചൊവ്വയോടൊപ്പം", "ചൊവ്വയെ", "ചൊവ്വയ്ക്ക്"],
        DashaLord::Mercury => ["ബുധന്റെ", "ബുധനോടൊപ്പം", "ബുധനെ", "ബുധന്"],
        DashaLord::Jupiter => ["വ്യാഴത്തിന്റെ", "വ്യാഴത്തോടൊപ്പം", "വ്യാഴത്തെ", "വ്യാഴത്തിന്"],
        DashaLord::Venus => ["ശുക്രന്റെ", "ശുക്രനോടൊപ്പം", "ശുക്രനെ", "ശുക്രന്"],
        DashaLord::Saturn => ["ശനിയുടെ", "ശനിയോടൊപ്പം", "ശനിയെ", "ശനിക്ക്"],
        DashaLord::Rahu => ["രാഹുവിന്റെ", "രാഹുവിനോടൊപ്പം", "രാഹുവിനെ", "രാഹുവിന്"],
        DashaLord::Ketu => ["കേതുവിന്റെ", "കേതുവിനോടൊപ്പം", "കേതുവിനെ", "കേതുവിന്"],
    }
}

/// A planet list: English keeps its old `[Saturn, Rahu]` form.
fn gl(lang: Lang, gs: &[DashaLord]) -> String {
    match lang {
        Lang::En => format!("{gs:?}"),
        _ => w::graha_list(lang, gs),
    }
}

/// "(Prasna Marga: …)" notes after a factor.
fn pm_note(lang: Lang, en: &str, ml: &str, hi: &str) -> String {
    say3!(lang, "(Prasna Marga: {en})", "(പ്രശ്നമാർഗ്ഗം: {ml})", "(प्रश्न मार्ग: {hi})")
}

const BPHS_85: &str = "BPHS ch. 85 (Sharma; Santhanam chs. 83–85), Curses of the Previous Birth: the karaka of the offended party afflicting the 5th house or its lord; stanzas as read in the research brief, verify against Santhanam vol. 2";

fn bphs_85(lang: Lang) -> &'static str {
    lang.pick([
        BPHS_85,
        "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 85 (ശർമ്മ; സന്താനം അധ്യായം 83–85), പൂർവ്വജന്മദോഷങ്ങൾ: ബന്ധപ്പെട്ടയാളുടെ കാരകഗ്രഹം അഞ്ചാം ഭാവത്തെയോ അതിന്റെ അധിപനെയോ പീഡിപ്പിക്കുന്നത്; ശ്ലോകങ്ങൾ ഗവേഷണക്കുറിപ്പിൽ വായിച്ചതുപോലെ, സന്താനം രണ്ടാം വാല്യവുമായി ഒത്തുനോക്കണം",
        "बृहत् पराशर होरा शास्त्र अध्याय 85 (शर्मा; संतानम अध्याय 83–85), पूर्वजन्म के दोष: संबंधित व्यक्ति का कारक ग्रह पंचम भाव या उसके स्वामी को पीड़ित करे; श्लोक शोध-टिप्पणी के पाठ के अनुसार, संतानम खंड 2 से मिलान करना है",
    ])
}

/// Pitru dosha: the father's and ancestors' curse (the Sun afflicting the 5th), with
/// the Prasna Marga father indicators and the later generalisation marked as such.
pub fn pitru_dosha(facts: &Facts) -> Dosha {
    pitru_dosha_in(facts, Lang::En)
}

/// [`pitru_dosha`] in the reader's language.
pub fn pitru_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let sun_h = house_of(facts, Sun);
    let l5 = lord_of(facts, 5);
    let l9 = lord_of(facts, 9);
    let l6 = lord_of(facts, 6);
    let l1 = lord_of(facts, 1);
    let (l5w, l9w, l6w, l1w) = (gn(lang, l5), gn(lang, l9), gn(lang, l6), gn(lang, l1));
    let father = |lang| pm_note(lang, "the father's displeasure", "അച്ഛന്റെ അതൃപ്തി", "पिता की नाराज़गी");
    let occ5 = facts.rasi_occupants(5);
    if sun_h == 5 {
        let m = malefics_with(facts, Sun);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((0.4, say3!(lang, "the Sun is in the 5th with {ml}", "സൂര്യൻ അഞ്ചാം ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "सूर्य पंचम भाव में {ml} के साथ")));
        }
        if occ5.contains(&Saturn) && occ5.contains(&Mars) {
            f.push((0.5, lang.pick(["Sun, Saturn and Mars together in the 5th", "സൂര്യനും ശനിയും ചൊവ്വയും ഒരുമിച്ച് അഞ്ചാം ഭാവത്തിൽ", "सूर्य, शनि और मंगल एक साथ पंचम भाव में"]).into()));
        }
        if dignity(Sun, facts.planet(Sun).sidereal) == Dignity::Debilitated {
            let a = malefic_aspects_on(facts, Sun);
            if !a.is_empty() {
                let al = gl(lang, &a);
                f.push((
                    0.4,
                    say3!(lang, "the Sun is debilitated in the 5th and aspected by {al}", "സൂര്യൻ അഞ്ചാം ഭാവത്തിൽ നീചത്തിൽ, {al} ദൃഷ്ടിയോടെ", "सूर्य पंचम भाव में नीच है और उस पर {al} की दृष्टि"),
                ));
            }
        }
    }
    if l5 != Sun && conj(facts, l5, Sun) {
        let m: Vec<DashaLord> = malefics_with(facts, l5)
            .into_iter()
            .filter(|g| *g != Sun)
            .collect();
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((
                0.4,
                say3!(lang, "{l5:?}, lord of the 5th, is with the Sun and {ml}", "അഞ്ചാം ഭാവാധിപനായ {l5w} സൂര്യനോടും {ml} എന്നിവയോടുമൊപ്പം", "पंचमेश {l5w} सूर्य और {ml} के साथ"),
            ));
        }
    }
    if house_of(facts, l9) == 5 && (conj(facts, l9, Saturn) || conj(facts, l9, Rahu)) {
        f.push((
            0.35,
            say3!(lang, "{l9:?}, lord of the 9th, is in the 5th with Saturn or Rahu", "ഒമ്പതാം ഭാവാധിപനായ {l9w} അഞ്ചാം ഭാവത്തിൽ ശനിയോടോ രാഹുവിനോടോ ഒപ്പം", "नवमेश {l9w} पंचम भाव में शनि या राहु के साथ"),
        ));
    }
    if house_of(facts, l1) == 5 && is_dusthana(house_of(facts, l5)) && conj(facts, l5, Sun) {
        f.push((0.3, say3!(lang, "the lagna lord {l1:?} is in the 5th while {l5:?}, lord of the 5th, is in a dusthana with the Sun", "ലഗ്നാധിപനായ {l1w} അഞ്ചാം ഭാവത്തിൽ; അഞ്ചാം ഭാവാധിപനായ {l5w} സൂര്യനോടൊപ്പം ദുഃസ്ഥാനത്തിൽ", "लग्नेश {l1w} पंचम भाव में है, जबकि पंचमेश {l5w} सूर्य के साथ दुःस्थान में")));
    }
    if house_of(facts, l6) == 9 {
        let n = father(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => format!(
                    "{l6:?}, lord of the 6th, is in the 9th (Prasna Marga: the father's displeasure)"
                ),
                Lang::Ml => format!("ആറാം ഭാവാധിപനായ {l6w} ഒമ്പതാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("षष्ठेश {l6w} नवम भाव में {n}"),
            },
        ));
    }
    if house_of(facts, l9) == 12 {
        let n = father(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => format!(
                    "{l9:?}, lord of the 9th, is in the 12th (Prasna Marga: the father's displeasure)"
                ),
                Lang::Ml => format!("ഒമ്പതാം ഭാവാധിപനായ {l9w} പന്ത്രണ്ടാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("नवमेश {l9w} द्वादश भाव में {n}"),
            },
        ));
    }
    if sun_h == 6 {
        let n = father(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => "the Sun is in the 6th (Prasna Marga: the father's displeasure)".into(),
                Lang::Ml => format!("സൂര്യൻ ആറാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("सूर्य षष्ठ भाव में {n}"),
            },
        ));
    }
    let nodes_sat: Vec<DashaLord> = malefics_with(facts, Sun)
        .into_iter()
        .filter(|g| matches!(g, Rahu | Ketu | Saturn))
        .collect();
    if !nodes_sat.is_empty() && sun_h != 5 {
        let nl = gl(lang, &nodes_sat);
        f.push((0.15, say3!(lang, "the Sun is with {nl} (the broad modern 'pitra dosha', a later generalisation, not BPHS)", "സൂര്യൻ {nl} എന്നിവയോടൊപ്പം (ഇന്ന് പരക്കെ പറയുന്ന 'പിതൃദോഷം'; പിൽക്കാലത്തെ പൊതുവായ്പ്പ്, പരാശരഗ്രന്ഥത്തിലില്ല)", "सूर्य {nl} के साथ (आजकल व्यापक रूप से कहा जाने वाला 'पितृ दोष'; बाद का सामान्यीकरण, पराशर के ग्रंथ में नहीं)")));
    }
    let occ9: Vec<DashaLord> = facts
        .rasi_occupants(9)
        .into_iter()
        .filter(|g| matches!(g, Rahu | Ketu | Saturn))
        .collect();
    if !occ9.is_empty() {
        let ol = gl(lang, &occ9);
        f.push((0.15, say3!(lang, "{ol} in the 9th house of the father (the broad modern reading, a later generalisation)", "{ol} അച്ഛന്റെ ഭാവമായ ഒമ്പതാം ഭാവത്തിൽ (ഇന്നത്തെ പൊതുവായ വായന, പിൽക്കാലത്തെ പൊതുവായ്പ്പ്)", "{ol} पिता के नवम भाव में (आजकल का व्यापक पाठ, बाद का सामान्यीकरण)")));
    }
    build(
        "Pitru dosha",
        "KS-R-SHAPA-01",
        DashaLord::Sun,
        lang.pick([
            "BPHS ch. 85: the father's curse (Sun afflicting the 5th); Prasna Marga ch. 15: 6th lord in 9th, 9th lord in 12th, Sun in 6th; remedies named: shraddha at Gaya, feeding Brahmins, kanyadana, gift of a cow",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 85: അച്ഛനുമായി ബന്ധപ്പെട്ട ദോഷം (സൂര്യൻ അഞ്ചാം ഭാവത്തെ പീഡിപ്പിക്കുന്നത്); പ്രശ്നമാർഗ്ഗം അധ്യായം 15: ആറാം ഭാവാധിപൻ ഒമ്പതിൽ, ഒമ്പതാം ഭാവാധിപൻ പന്ത്രണ്ടിൽ, സൂര്യൻ ആറിൽ; പറയുന്ന പരിഹാരങ്ങൾ: ഗയയിൽ ശ്രാദ്ധം, ബ്രാഹ്മണഭോജനം, കന്യാദാനം, ഗോദാനം",
            "बृहत् पराशर होरा शास्त्र अध्याय 85: पिता से जुड़ा दोष (सूर्य पंचम भाव को पीड़ित करे); प्रश्न मार्ग अध्याय 15: षष्ठेश नवम में, नवमेश द्वादश में, सूर्य षष्ठ में; बताए गए उपाय: गया में श्राद्ध, ब्राह्मण भोजन, कन्यादान, गोदान",
        ]),
        lang.pick([
            "Sun: a duty to ancestors left undone; conduct toward elders, the yearly remembrance days of the departed kept, tarpana on Amavasya, Mahalaya in Pitru Paksha, Sunday in the Sun's hora",
            "സൂര്യൻ: പൂർവ്വികരോടുള്ള ഒരു കടമ ബാക്കിയുണ്ട്; മുതിർന്നവരോടുള്ള പെരുമാറ്റം, പരേതരുടെ ആണ്ടുബലി മുടങ്ങാതെ, കറുത്തവാവിന് തർപ്പണം, പിതൃപക്ഷത്തിൽ മഹാലയം, ഞായറാഴ്ച സൂര്യഹോരയിൽ",
            "सूर्य: पूर्वजों के प्रति एक कर्तव्य अधूरा है; बड़ों के प्रति आचरण, दिवंगतों की बरसी निभाना, अमावस्या पर तर्पण, पितृ पक्ष में महालय, रविवार को सूर्य की होरा में",
        ]),
        lang.pick(["the father and the pitris", "അച്ഛനും പിതൃക്കളും", "पिता और पितर"]).into(),
        f,
    )
}

/// Sarpa shapa / Naga dosha: the serpent curse (Rahu, with Mars, on the 5th), with the
/// Prasna Marga serpent indicators.
pub fn sarpa_dosha(facts: &Facts) -> Dosha {
    sarpa_dosha_in(facts, Lang::En)
}

/// [`sarpa_dosha`] in the reader's language.
pub fn sarpa_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let rahu_h = house_of(facts, Rahu);
    let mars_h = house_of(facts, Mars);
    let l5 = lord_of(facts, 5);
    let l5w = gn(lang, l5);
    if rahu_h == 5 {
        if conj(facts, Rahu, Mars) || aspects(facts, Mars, Rahu) {
            f.push((0.5, lang.pick(["Rahu in the 5th with or aspected by Mars", "രാഹു അഞ്ചാം ഭാവത്തിൽ, ചൊവ്വയോടൊപ്പമോ ചൊവ്വയുടെ ദൃഷ്ടിയിലോ", "राहु पंचम भाव में, मंगल के साथ या उसकी दृष्टि में"]).into()));
        }
        if matches!(facts.planet(Rahu).sign, 0 | 7) {
            f.push((0.4, lang.pick(["Rahu in a sign of Mars in the 5th", "രാഹു അഞ്ചാം ഭാവത്തിൽ ചൊവ്വയുടെ രാശിയിൽ", "राहु पंचम भाव में मंगल की राशि में"]).into()));
        }
    }
    if l5 != Rahu && conj(facts, l5, Rahu) && conj(facts, l5, Mars) {
        f.push((
            0.4,
            say3!(lang, "{l5:?}, lord of the 5th, is with Rahu and Mars", "അഞ്ചാം ഭാവാധിപനായ {l5w} രാഹുവിനോടും ചൊവ്വയോടുമൊപ്പം", "पंचमेश {l5w} राहु और मंगल के साथ"),
        ));
    }
    if mars_h == 5 && aspects(facts, Rahu, Mars) && rahu_h != 5 {
        f.push((0.35, lang.pick(["Mars in the 5th aspected by Rahu", "ചൊവ്വ അഞ്ചാം ഭാവത്തിൽ, രാഹുവിന്റെ ദൃഷ്ടിയിൽ", "मंगल पंचम भाव में, राहु की दृष्टि में"]).into()));
    }
    if matches!(rahu_h, 1 | 5) && is_dusthana(house_of(facts, l5)) {
        let rw = w::bhava(lang, rahu_h);
        f.push((
            0.3,
            say3!(lang, "Rahu in house {rahu_h} while {l5:?}, lord of the 5th, is in a dusthana", "രാഹു {rw} ഭാവത്തിൽ; അഞ്ചാം ഭാവാധിപനായ {l5w} ദുഃസ്ഥാനത്തിൽ", "राहु {rw} भाव में, जबकि पंचमेश {l5w} दुःस्थान में"),
        ));
    }
    let b = badhaka(facts);
    if rahu_h == b.house || conj(facts, Rahu, b.lord) {
        let bh = w::bhava(lang, b.house);
        let bl = gn(lang, b.lord);
        let bl_soc = ml_forms(b.lord)[1];
        f.push((0.3, match lang {
            Lang::En => format!("Rahu is in the badhaka house ({}) or with the badhakesh {:?} (Prasna Marga: a serpent deity)", b.house, b.lord),
            Lang::Ml => format!("രാഹു ബാധകസ്ഥാനമായ {bh} ഭാവത്തിൽ, അല്ലെങ്കിൽ ബാധകാധിപനായ {bl_soc} (പ്രശ്നമാർഗ്ഗം: ഒരു സർപ്പദേവത)"),
            Lang::Hi => format!("राहु बाधक स्थान ({bh} भाव) में या बाधकेश {bl} के साथ (प्रश्न मार्ग: एक नाग देवता)"),
        }));
    }
    if conj(facts, Moon, Rahu) {
        f.push((
            0.2,
            lang.pick(["the Moon is with Rahu (Prasna Marga: malicious serpents)", "ചന്ദ്രൻ രാഹുവിനോടൊപ്പം (പ്രശ്നമാർഗ്ഗം: പ്രീതിപ്പെടുത്തേണ്ട സർപ്പങ്ങൾ)", "चंद्र राहु के साथ (प्रश्न मार्ग: नाग जिन्हें प्रसन्न करना है)"]).into(),
        ));
    }
    if conj(facts, Sun, Rahu) {
        f.push((
            0.1,
            lang.pick(["the Sun is with Rahu (Prasna Marga: benevolent serpents, long-term good)", "സൂര്യൻ രാഹുവിനോടൊപ്പം (പ്രശ്നമാർഗ്ഗം: അനുഗ്രഹിക്കുന്ന സർപ്പങ്ങൾ, ദീർഘകാല നന്മ)", "सूर्य राहु के साथ (प्रश्न मार्ग: कृपालु नाग, लंबे समय का भला)"]).into(),
        ));
    }
    build(
        "Sarpa dosha",
        "KS-R-SHAPA-02",
        Rahu,
        lang.pick([
            "BPHS ch. 85: the serpent curse (Rahu, with Mars, afflicting the 5th); Prasna Marga ch. 15: Rahu or Gulika with the badhakesh, Moon with Rahu; remedies named: Naga pratishtha, Sarpa shanti, gift of a cow and gold",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 85: സർപ്പദോഷം (ചൊവ്വയോടൊപ്പം രാഹു അഞ്ചാം ഭാവത്തെ പീഡിപ്പിക്കുന്നത്); പ്രശ്നമാർഗ്ഗം അധ്യായം 15: രാഹുവോ ഗുളികനോ ബാധകാധിപനോടൊപ്പം, ചന്ദ്രൻ രാഹുവിനോടൊപ്പം; പറയുന്ന പരിഹാരങ്ങൾ: നാഗപ്രതിഷ്ഠ, സർപ്പശാന്തി, ഗോദാനവും സ്വർണ്ണദാനവും",
            "बृहत् पराशर होरा शास्त्र अध्याय 85: सर्प दोष (मंगल के साथ राहु पंचम भाव को पीड़ित करे); प्रश्न मार्ग अध्याय 15: राहु या गुलिक बाधकेश के साथ, चंद्र राहु के साथ; बताए गए उपाय: नाग प्रतिष्ठा, सर्प शांति, गाय और सोने का दान",
        ]),
        lang.pick([
            "Rahu: a debt to serpents and the land; guard a living thing or a plot, end a deception; Panchami, the Moon in Ashlesha, Rahu kala, Saturday",
            "രാഹു: സർപ്പങ്ങളോടും ഭൂമിയോടുമുള്ള ഒരു കടം; ഒരു ജീവനെയോ ഒരു തുണ്ടു ഭൂമിയെയോ കാക്കുക, ഒരു വഞ്ചന അവസാനിപ്പിക്കുക; പഞ്ചമി, ചന്ദ്രൻ ആയില്യത്തിൽ, രാഹുകാലം, ശനിയാഴ്ച",
            "राहु: नागों और धरती का एक ऋण; किसी जीव या ज़मीन के टुकड़े की रक्षा करें, कोई छल समाप्त करें; पंचमी, चंद्र आश्लेषा में, राहु काल, शनिवार",
        ]),
        lang.pick(["the serpent deities", "സർപ്പദേവതകൾ", "नाग देवता"]).into(),
        f,
    )
}

/// Matru shapa: the mother's curse (the Moon afflicted in or ruling the 5th), with the
/// Prasna Marga mother indicators.
pub fn matru_shapa(facts: &Facts) -> Dosha {
    matru_shapa_in(facts, Lang::En)
}

/// [`matru_shapa`] in the reader's language.
pub fn matru_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let moon_h = house_of(facts, Moon);
    let l4 = lord_of(facts, 4);
    let l5 = lord_of(facts, 5);
    let l6 = lord_of(facts, 6);
    let (l4w, l6w) = (gn(lang, l4), gn(lang, l6));
    let mother = |lang| pm_note(lang, "the mother's displeasure", "അമ്മയുടെ അതൃപ്തി", "माता की नाराज़गी");
    if matches!(moon_h, 4 | 5) && (conj(facts, Moon, Saturn) || conj(facts, Moon, Rahu)) {
        let mw = w::bhava(lang, moon_h);
        f.push((
            0.4,
            say3!(lang, "the Moon is in house {moon_h} with Saturn or Rahu", "ചന്ദ്രൻ {mw} ഭാവത്തിൽ ശനിയോടോ രാഹുവിനോടോ ഒപ്പം", "चंद्र {mw} भाव में शनि या राहु के साथ"),
        ));
    }
    if is_dusthana(moon_h) && is_dusthana(house_of(facts, l4)) {
        f.push((
            0.35,
            say3!(lang, "the Moon and {l4:?}, lord of the 4th, are both in dusthanas", "ചന്ദ്രൻ, നാലാം ഭാവാധിപനായ {l4w} എന്നിവ രണ്ടും ദുഃസ്ഥാനങ്ങളിൽ", "चंद्र और चतुर्थेश {l4w} दोनों दुःस्थानों में"),
        ));
    }
    if moon_h == 5 {
        let m = malefics_with(facts, Moon);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((0.3, say3!(lang, "the Moon is in the 5th with {ml}", "ചന്ദ്രൻ അഞ്ചാം ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "चंद्र पंचम भाव में {ml} के साथ")));
        }
    }
    if l5 == Moon {
        if let Some(a) = afflicted(facts, Moon, lang) {
            f.push((0.3, say3!(lang, "the Moon rules the 5th and {a}", "ചന്ദ്രൻ അഞ്ചാം ഭാവാധിപനാണ്; {a}", "चंद्र पंचम भाव का स्वामी है और {a}")));
        }
    }
    if house_of(facts, l6) == 4 {
        let n = mother(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => format!(
                    "{l6:?}, lord of the 6th, is in the 4th (Prasna Marga: the mother's displeasure)"
                ),
                Lang::Ml => format!("ആറാം ഭാവാധിപനായ {l6w} നാലാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("षष्ठेश {l6w} चतुर्थ भाव में {n}"),
            },
        ));
    }
    if house_of(facts, l4) == 12 {
        let n = mother(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => format!(
                    "{l4:?}, lord of the 4th, is in the 12th (Prasna Marga: the mother's displeasure)"
                ),
                Lang::Ml => format!("നാലാം ഭാവാധിപനായ {l4w} പന്ത്രണ്ടാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("चतुर्थेश {l4w} द्वादश भाव में {n}"),
            },
        ));
    }
    if moon_h == 6 {
        let n = mother(lang);
        f.push((
            0.2,
            match lang {
                Lang::En => "the Moon is in the 6th (Prasna Marga: the mother's displeasure)".into(),
                Lang::Ml => format!("ചന്ദ്രൻ ആറാം ഭാവത്തിൽ {n}"),
                Lang::Hi => format!("चंद्र षष्ठ भाव में {n}"),
            },
        ));
    }
    build(
        "Matru shapa",
        "KS-R-SHAPA-03",
        Moon,
        bphs_85(lang),
        lang.pick([
            "Moon: tend the mother in person, fixed sleep; Gayatri japa and Peepal worship; Monday in the Moon's hora, Purnima",
            "ചന്ദ്രൻ: അമ്മയെ നേരിട്ടു ചെന്ന് പരിചരിക്കുക, ഉറക്കത്തിന് കൃത്യസമയം; ഗായത്രീജപവും അരയാൽ പ്രദക്ഷിണവും; തിങ്കളാഴ്ച ചന്ദ്രഹോരയിൽ, പൗർണ്ണമി",
            "चंद्र: माँ की सेवा स्वयं जाकर करें, सोने का तय समय; गायत्री जप और पीपल पूजा; सोमवार को चंद्र की होरा में, पूर्णिमा",
        ]),
        lang.pick(["the mother", "അമ്മ", "माता"]).into(),
        f,
    )
}

/// Bhratru shapa: the brother's curse (Mars afflicting the 5th; the 3rd lord afflicted).
pub fn bhratru_shapa(facts: &Facts) -> Dosha {
    bhratru_shapa_in(facts, Lang::En)
}

/// [`bhratru_shapa`] in the reader's language.
pub fn bhratru_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l3 = lord_of(facts, 3);
    let l3w = gn(lang, l3);
    if house_of(facts, Mars) == 5 {
        let m = malefics_with(facts, Mars);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((0.4, say3!(lang, "Mars is in the 5th with {ml}", "ചൊവ്വ അഞ്ചാം ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "मंगल पंचम भाव में {ml} के साथ")));
        }
    }
    if is_dusthana(house_of(facts, l3)) && (conj(facts, l3, Mars) || conj(facts, l3, Rahu)) {
        f.push((
            0.35,
            say3!(lang, "{l3:?}, lord of the 3rd, is in a dusthana with Mars or Rahu", "മൂന്നാം ഭാവാധിപനായ {l3w} ദുഃസ്ഥാനത്തിൽ ചൊവ്വയോടോ രാഹുവിനോടോ ഒപ്പം", "तृतीयेश {l3w} दुःस्थान में मंगल या राहु के साथ"),
        ));
    }
    build(
        "Bhratru shapa",
        "KS-R-SHAPA-04",
        Mars,
        bphs_85(lang),
        lang.pick([
            "Mars: end a quarrel with a sibling; Gayatri japa with homa, Harivamsha shravana; Tuesday in Mars's hora",
            "ചൊവ്വ: ഒരു സഹോദരനുമായുള്ള പിണക്കം തീർക്കുക; ഹോമത്തോടെ ഗായത്രീജപം, ഹരിവംശ ശ്രവണം; ചൊവ്വാഴ്ച ചൊവ്വയുടെ ഹോരയിൽ",
            "मंगल: किसी भाई-बहन से झगड़ा खत्म करें; होम के साथ गायत्री जप, हरिवंश श्रवण; मंगलवार को मंगल की होरा में",
        ]),
        lang.pick(["the brother", "സഹോദരൻ", "भाई"]).into(),
        f,
    )
}

/// Matula shapa: the maternal uncle's curse (Mercury afflicting the 5th; the 6th lord
/// and Mercury afflicted).
pub fn matula_shapa(facts: &Facts) -> Dosha {
    matula_shapa_in(facts, Lang::En)
}

/// [`matula_shapa`] in the reader's language.
pub fn matula_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l6 = lord_of(facts, 6);
    if house_of(facts, Mercury) == 5 {
        let m = malefics_with(facts, Mercury);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((0.4, say3!(lang, "Mercury is in the 5th with {ml}", "ബുധൻ അഞ്ചാം ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "बुध पंचम भाव में {ml} के साथ")));
        }
    }
    if let (Some(a), Some(b)) = (afflicted(facts, l6, lang), afflicted(facts, Mercury, lang)) {
        if l6 != Mercury {
            f.push((0.3, say3!(lang, "{a} (lord of the 6th) and {b}", "{a} (ആറാം ഭാവാധിപൻ); {b}", "{a} (षष्ठेश) और {b}")));
        }
    }
    build(
        "Matula shapa",
        "KS-R-SHAPA-05",
        Mercury,
        bphs_85(lang),
        lang.pick([
            "Mercury: restore a cut family tie, truthful speech; Vishnu pratishtha (ritual, optional); Wednesday in Mercury's hora",
            "ബുധൻ: മുറിഞ്ഞ ഒരു കുടുംബബന്ധം വീണ്ടും കൂട്ടിച്ചേർക്കുക, സത്യമുള്ള വാക്ക്; വിഷ്ണുപ്രതിഷ്ഠ (ചടങ്ങ്, നിർബന്ധമില്ല); ബുധനാഴ്ച ബുധഹോരയിൽ",
            "बुध: टूटा हुआ पारिवारिक रिश्ता फिर जोड़ें, सच्ची वाणी; विष्णु प्रतिष्ठा (अनुष्ठान, ऐच्छिक); बुधवार को बुध की होरा में",
        ]),
        lang.pick(["the maternal uncle", "അമ്മാവൻ", "मामा"]).into(),
        f,
    )
}

/// Brahma shapa: the Brahmin's curse (Jupiter afflicting the 5th; Rahu in Jupiter's
/// sign in the 5th; the 9th lord and Jupiter afflicted).
pub fn brahma_shapa(facts: &Facts) -> Dosha {
    brahma_shapa_in(facts, Lang::En)
}

/// [`brahma_shapa`] in the reader's language.
pub fn brahma_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l9 = lord_of(facts, 9);
    if house_of(facts, Jupiter) == 5 {
        let m = malefics_with(facts, Jupiter);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            f.push((0.4, say3!(lang, "Jupiter is in the 5th with {ml}", "വ്യാഴം അഞ്ചാം ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "गुरु पंचम भाव में {ml} के साथ")));
        }
    }
    if house_of(facts, Rahu) == 5 && matches!(facts.planet(Rahu).sign, 8 | 11) {
        f.push((0.4, lang.pick(["Rahu in a sign of Jupiter in the 5th", "രാഹു അഞ്ചാം ഭാവത്തിൽ വ്യാഴത്തിന്റെ രാശിയിൽ", "राहु पंचम भाव में गुरु की राशि में"]).into()));
    }
    if let (Some(a), Some(b)) = (afflicted(facts, l9, lang), afflicted(facts, Jupiter, lang)) {
        if l9 != Jupiter {
            f.push((0.3, say3!(lang, "{a} (lord of the 9th) and {b}", "{a} (ഒമ്പതാം ഭാവാധിപൻ); {b}", "{a} (नवमेश) और {b}")));
        }
    }
    build(
        "Brahma shapa",
        "KS-R-SHAPA-06",
        Jupiter,
        lang.pick([
            "BPHS ch. 85: the Brahmin's curse; remedy named: Chandrayana vrata and service to a cow (citing Parashara)",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 85: ഗുരുജനങ്ങളുമായി ബന്ധപ്പെട്ട ദോഷം; പറയുന്ന പരിഹാരം: ചാന്ദ്രായണ വ്രതവും ഗോസേവയും (പരാശരനെ ഉദ്ധരിച്ച്)",
            "बृहत् पराशर होरा शास्त्र अध्याय 85: गुरुजनों से जुड़ा दोष; बताया गया उपाय: चांद्रायण व्रत और गो सेवा (पराशर के हवाले से)",
        ]),
        lang.pick([
            "Jupiter: honour a teacher, return what was taken from a learned person; Chandrayana (a lunar-month vow whose daily dose follows the tithi); Thursday in Jupiter's hora",
            "വ്യാഴം: ഒരു ഗുരുവിനെ ആദരിക്കുക, ഒരു വിദ്വാനിൽ നിന്ന് എടുത്തത് തിരികെ നൽകുക; ചാന്ദ്രായണം (തിഥിക്കനുസരിച്ച് ദിവസേനയുള്ള അളവ് മാറുന്ന ഒരു മാസവ്രതം); വ്യാഴാഴ്ച വ്യാഴഹോരയിൽ",
            "गुरु: किसी शिक्षक का सम्मान करें, किसी विद्वान से लिया हुआ लौटाएँ; चांद्रायण (एक चांद्र मास का व्रत जिसकी रोज़ की मात्रा तिथि के साथ चलती है); गुरुवार को गुरु की होरा में",
        ]),
        lang.pick(["a Brahmin, the preceptor", "ഒരു ബ്രാഹ്മണൻ, ഗുരു", "एक ब्राह्मण, गुरु"]).into(),
        f,
    )
}

/// Patni shapa: the wife's curse (Venus afflicted in the 5th or 7th; the 7th lord in a
/// dusthana with Venus).
pub fn patni_shapa(facts: &Facts) -> Dosha {
    patni_shapa_in(facts, Lang::En)
}

/// [`patni_shapa`] in the reader's language.
pub fn patni_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l7 = lord_of(facts, 7);
    let l7w = gn(lang, l7);
    let vh = house_of(facts, Venus);
    if matches!(vh, 5 | 7) {
        let m = malefics_with(facts, Venus);
        if !m.is_empty() {
            let ml = gl(lang, &m);
            let vw = w::bhava(lang, vh);
            f.push((0.4, say3!(lang, "Venus is in house {vh} with {ml}", "ശുക്രൻ {vw} ഭാവത്തിൽ {ml} എന്നിവയോടൊപ്പം", "शुक्र {vw} भाव में {ml} के साथ")));
        }
    }
    if l7 != Venus && is_dusthana(house_of(facts, l7)) && conj(facts, l7, Venus) {
        f.push((
            0.35,
            say3!(lang, "{l7:?}, lord of the 7th, is in a dusthana with Venus", "ഏഴാം ഭാവാധിപനായ {l7w} ശുക്രനോടൊപ്പം ദുഃസ്ഥാനത്തിൽ", "सप्तमेश {l7w} शुक्र के साथ दुःस्थान में"),
        ));
    }
    build(
        "Patni shapa",
        "KS-R-SHAPA-07",
        Venus,
        bphs_85(lang),
        lang.pick([
            "Venus: keep a promise to the spouse, no deception in love; Lakshmi-Narayana pratishtha, kanyadana, a cow (ritual, optional); Friday in Venus's hora",
            "ശുക്രൻ: ജീവിതപങ്കാളിക്കു നൽകിയ വാക്ക് പാലിക്കുക, സ്നേഹത്തിൽ വഞ്ചനയരുത്; ലക്ഷ്മീനാരായണ പ്രതിഷ്ഠ, കന്യാദാനം, ഗോദാനം (ചടങ്ങ്, നിർബന്ധമില്ല); വെള്ളിയാഴ്ച ശുക്രഹോരയിൽ",
            "शुक्र: जीवनसाथी से किया वादा निभाएँ, प्रेम में छल नहीं; लक्ष्मी-नारायण प्रतिष्ठा, कन्यादान, गोदान (अनुष्ठान, ऐच्छिक); शुक्रवार को शुक्र की होरा में",
        ]),
        lang.pick(["the wife", "ഭാര്യ", "पत्नी"]).into(),
        f,
    )
}

/// Preta shapa: the curse of the departed (Saturn or Ketu with the 5th lord; Sun and
/// Saturn in the 5th with Ketu).
pub fn preta_shapa(facts: &Facts) -> Dosha {
    preta_shapa_in(facts, Lang::En)
}

/// [`preta_shapa`] in the reader's language.
pub fn preta_shapa_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l5 = lord_of(facts, 5);
    let l5w = gn(lang, l5);
    if l5 != Saturn && conj(facts, l5, Saturn) {
        f.push((0.35, say3!(lang, "{l5:?}, lord of the 5th, is with Saturn", "അഞ്ചാം ഭാവാധിപനായ {l5w} ശനിയോടൊപ്പം", "पंचमेश {l5w} शनि के साथ")));
    }
    if conj(facts, l5, Ketu) {
        f.push((0.35, say3!(lang, "{l5:?}, lord of the 5th, is with Ketu", "അഞ്ചാം ഭാവാധിപനായ {l5w} കേതുവിനോടൊപ്പം", "पंचमेश {l5w} केतु के साथ")));
    }
    let occ5 = facts.rasi_occupants(5);
    if occ5.contains(&Sun) && occ5.contains(&Saturn) && occ5.contains(&Ketu) {
        f.push((0.5, lang.pick(["Sun and Saturn in the 5th with Ketu", "സൂര്യനും ശനിയും കേതുവിനോടൊപ്പം അഞ്ചാം ഭാവത്തിൽ", "सूर्य और शनि केतु के साथ पंचम भाव में"]).into()));
    }
    build(
        "Preta shapa",
        "KS-R-SHAPA-08",
        Saturn,
        lang.pick([
            "BPHS ch. 85: the curse of departed spirits; remedies named: Gaya shraddha, Rudra japa, gift of a cow or silver",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 85: പരേതരുമായി ബന്ധപ്പെട്ട ദോഷം; പറയുന്ന പരിഹാരങ്ങൾ: ഗയാശ്രാദ്ധം, രുദ്രജപം, ഗോദാനമോ വെള്ളിദാനമോ",
            "बृहत् पराशर होरा शास्त्र अध्याय 85: दिवंगतों से जुड़ा दोष; बताए गए उपाय: गया श्राद्ध, रुद्र जप, गाय या चाँदी का दान",
        ]),
        lang.pick([
            "Saturn and Ketu: complete the rites or the memory of a departed relative, visit a grave; tarpana on Amavasya; Saturday in Saturn's hora",
            "ശനിയും കേതുവും: പരേതനായ ഒരു ബന്ധുവിന്റെ കർമ്മങ്ങളോ ഓർമ്മയോ പൂർത്തിയാക്കുക, അവരുടെ അന്ത്യവിശ്രമസ്ഥലം സന്ദർശിക്കുക; കറുത്തവാവിന് തർപ്പണം; ശനിയാഴ്ച ശനിഹോരയിൽ",
            "शनि और केतु: किसी दिवंगत संबंधी के संस्कार या स्मृति पूरी करें, उनके विश्राम-स्थल पर जाएँ; अमावस्या पर तर्पण; शनिवार को शनि की होरा में",
        ]),
        lang.pick(["the departed (pretas)", "പരേതർ (പിതൃക്കൾ)", "दिवंगत (पितर)"]).into(),
        f,
    )
}

/// The badhaka reading of Prasna Marga ch. 15.
#[derive(Clone, Debug, PartialEq)]
pub struct Badhaka {
    /// "movable", "fixed" or "dual".
    pub lagna_type: &'static str,
    /// Badhaka house: movable 11, fixed 9, dual 7.
    pub house: u8,
    /// Its lord, the badhakesh.
    pub lord: DashaLord,
    /// Each companion (in the badhaka house, with the badhakesh, or aspecting it), the
    /// being it names, and how it is joined.
    pub companions: Vec<(DashaLord, &'static str, String)>,
    /// The offence read from the house, when the text gives one.
    pub offence: Option<String>,
    /// The kind of remedy from the tattva of the badhakesh's dispositor.
    pub remedy_kind: &'static str,
    /// Parent-type and serpent indicators that fired.
    pub parent_rules: Vec<String>,
}

/// The being a companion of the badhakesh names (V = snippet-verified from Prasna
/// Marga ch. 15; K = Kerala customary reading).
pub fn offended_being(g: DashaLord) -> &'static str {
    offended_being_in(g, Lang::En)
}

/// [`offended_being`] in the reader's language. The (V) and (K) marks become
/// "(പ്രശ്നമാർഗ്ഗം)" / "(प्रश्न मार्ग)" and "(കേരള രീതി)" / "(केरल परंपरा)".
pub fn offended_being_in(g: DashaLord, lang: Lang) -> &'static str {
    lang.pick(match g {
        DashaLord::Sun => ["Shiva, or a temple deity neglected (V)", "ശിവൻ, അല്ലെങ്കിൽ ശ്രദ്ധ കിട്ടാതെപോയ ഒരു ക്ഷേത്രദേവത (പ്രശ്നമാർഗ്ഗം)", "शिव, या उपेक्षित कोई मंदिर देवता (प्रश्न मार्ग)"],
        DashaLord::Moon => ["Durga / Devi (V)", "ദുർഗ്ഗ / ദേവി (പ്രശ്നമാർഗ്ഗം)", "दुर्गा / देवी (प्रश्न मार्ग)"],
        DashaLord::Mars => ["Subrahmanya (Karthikeya) or Bhadrakali; bad management of a shrine (V)", "സുബ്രഹ്മണ്യൻ (കാർത്തികേയൻ) അല്ലെങ്കിൽ ഭദ്രകാളി; ഒരു ആരാധനാലയത്തിന്റെ നടത്തിപ്പിലെ പിഴവ് (പ്രശ്നമാർഗ്ഗം)", "सुब्रह्मण्य (कार्तिकेय) या भद्रकाली; किसी देवस्थान की देखरेख में चूक (प्रश्न मार्ग)"],
        DashaLord::Rahu => ["a serpent deity (V)", "ഒരു സർപ്പദേവത (പ്രശ്നമാർഗ്ഗം)", "एक नाग देवता (प्रश्न मार्ग)"],
        DashaLord::Saturn => ["impurity (ashaucha), pretas and pitris (V)", "അശുദ്ധി (അശൗചം), പരേതരും പിതൃക്കളും (പ്രശ്നമാർഗ്ഗം)", "अशुद्धि (अशौच), दिवंगत और पितर (प्रश्न मार्ग)"],
        DashaLord::Mercury => ["Vishnu (K)", "വിഷ്ണു (കേരള രീതി)", "विष्णु (केरल परंपरा)"],
        DashaLord::Jupiter => ["a Brahmin's curse, the preceptor, Ganapati as Vighnesha (K)", "ഒരു ബ്രാഹ്മണന്റെ അതൃപ്തി, ഗുരു, വിഘ്നേശ്വരനായ ഗണപതി (കേരള രീതി)", "किसी ब्राह्मण की नाराज़गी, गुरु, विघ्नेश्वर गणेश (केरल परंपरा)"],
        DashaLord::Venus => ["Yakshi / Lakshmi, a women's or household deity (K)", "യക്ഷി / ലക്ഷ്മി, സ്ത്രീകളുടെയോ വീടിന്റെയോ ദേവത (കേരള രീതി)", "यक्षी / लक्ष्मी, स्त्रियों या घर की देवी (केरल परंपरा)"],
        DashaLord::Ketu => ["Chandi, a spirit; Ganapati (K)", "ചണ്ഡി, ഒരു ആത്മാവ്; ഗണപതി (കേരള രീതി)", "चंडी, कोई आत्मा; गणेश (केरल परंपरा)"],
    })
}

/// Remedy kind from the tattva of a planet (the element map is the brief's convention;
/// that the kind follows the dispositor's tattva is the text's rule).
pub fn remedy_kind_of_tattva(g: DashaLord) -> &'static str {
    remedy_kind_of_tattva_in(g, Lang::En)
}

/// [`remedy_kind_of_tattva`] in the reader's language.
pub fn remedy_kind_of_tattva_in(g: DashaLord, lang: Lang) -> &'static str {
    lang.pick(match g {
        DashaLord::Sun | DashaLord::Mars | DashaLord::Ketu => ["fire: homa", "അഗ്നി: ഹോമം", "अग्नि: होम"],
        DashaLord::Mercury => ["earth: daana, pratishtha, feeding", "ഭൂമി: ദാനം, പ്രതിഷ്ഠ, അന്നദാനം", "पृथ्वी: दान, प्रतिष्ठा, भोजन कराना"],
        DashaLord::Moon | DashaLord::Venus => ["water: abhisheka, tarpana, bathing at a tirtha", "ജലം: അഭിഷേകം, തർപ്പണം, തീർത്ഥസ്നാനം", "जल: अभिषेक, तर्पण, तीर्थ स्नान"],
        DashaLord::Saturn | DashaLord::Rahu => ["air: japa, vrata, seva", "വായു: ജപം, വ്രതം, സേവ", "वायु: जप, व्रत, सेवा"],
        DashaLord::Jupiter => ["ether: upadesha, Brahmin feeding, vows of study", "ആകാശം: ഉപദേശം, ബ്രാഹ്മണഭോജനം, പഠനവ്രതങ്ങൾ", "आकाश: उपदेश, ब्राह्मण भोजन, अध्ययन के व्रत"],
    })
}

/// The lagna type key ("movable", "fixed", "dual") in the reader's language.
fn lagna_type_in(key: &'static str, lang: Lang) -> &'static str {
    match key {
        "movable" => lang.pick(["movable", "ചരം", "चर"]),
        "fixed" => lang.pick(["fixed", "സ്ഥിരം", "स्थिर"]),
        _ => lang.pick(["dual", "ഉഭയം", "द्विस्वभाव"]),
    }
}

/// Badhaka house, badhakesh, companions and what they say is offended.
pub fn badhaka(facts: &Facts) -> Badhaka {
    badhaka_in(facts, Lang::En)
}

/// [`badhaka`] with the beings, the offence, the remedy kind and the rules in the
/// reader's language; `lagna_type` stays the English key.
pub fn badhaka_in(facts: &Facts, lang: Lang) -> Badhaka {
    use DashaLord::*;
    let (lagna_type, house) = match facts.lagna_sign % 3 {
        0 => ("movable", 11u8),
        1 => ("fixed", 9),
        _ => ("dual", 7),
    };
    let lord = lord_of(facts, house);
    let lw = gn(lang, lord);
    let [_, l_soc, l_acc, _] = ml_forms(lord);
    let hw = w::bhava(lang, house);
    let lp = facts.planet(lord);
    let mut companions: Vec<(DashaLord, &'static str, String)> = Vec::new();
    for g in facts.rasi_occupants(house) {
        if g != lord {
            let gw = gn(lang, g);
            companions.push((
                g,
                offended_being_in(g, lang),
                say3!(lang, "{g:?} occupies the badhaka house ({house})", "{gw} ബാധകസ്ഥാനമായ {hw} ഭാവത്തിൽ", "{gw} बाधक स्थान ({hw} भाव) में"),
            ));
        }
    }
    for p in &facts.planets {
        if p.graha == lord || companions.iter().any(|c| c.0 == p.graha) {
            continue;
        }
        let pg = p.graha;
        let gw = gn(lang, pg);
        if p.sign == lp.sign {
            companions.push((
                pg,
                offended_being_in(pg, lang),
                say3!(lang, "{pg:?} is with the badhakesh {lord:?}", "{gw} ബാധകാധിപനായ {l_soc}", "{gw} बाधकेश {lw} के साथ"),
            ));
        } else if aspects(facts, pg, lord) {
            companions.push((
                pg,
                offended_being_in(pg, lang),
                say3!(lang, "{pg:?} aspects the badhakesh {lord:?}", "{gw} ബാധകാധിപനായ {l_acc} നോക്കുന്നു", "{gw} की बाधकेश {lw} पर दृष्टि"),
            ));
        }
    }
    let offence = (house_of(facts, Mars) == 1).then(|| {
        lang.pick([
            "Mars in the lagna: a broken idol or sacred object; the remedy is installing a fresh idol (V)",
            "ചൊവ്വ ലഗ്നത്തിൽ: കേടുവന്ന ഒരു വിഗ്രഹമോ പൂജാവസ്തുവോ; പുതിയ വിഗ്രഹം പ്രതിഷ്ഠിക്കുന്നതാണ് പരിഹാരം (പ്രശ്നമാർഗ്ഗം)",
            "मंगल लग्न में: कोई खंडित मूर्ति या पूजा की वस्तु; नई मूर्ति की प्रतिष्ठा उपाय है (प्रश्न मार्ग)",
        ])
        .to_string()
    });
    let dispositor = sign_lord(lp.sign as usize);
    let remedy_kind = remedy_kind_of_tattva_in(dispositor, lang);
    let mut parent_rules = Vec::new();
    let l4 = lord_of(facts, 4);
    let l4w = gn(lang, l4);
    if house_of(facts, l4) == house || conj(facts, l4, lord) {
        parent_rules.push(say3!(lang, "{l4:?}, lord of the 4th, joins the badhaka: the family deity (kula or grama devata) (V)", "നാലാം ഭാവാധിപനായ {l4w} ബാധകത്തോടു ചേരുന്നു: കുടുംബദേവത (കുലദേവത അല്ലെങ്കിൽ ഗ്രാമദേവത) (പ്രശ്നമാർഗ്ഗം)", "चतुर्थेश {l4w} बाधक से जुड़ता है: कुल देवता (कुलदेवता या ग्रामदेवता) (प्रश्न मार्ग)"));
    }
    let l6 = lord_of(facts, 6);
    let l9 = lord_of(facts, 9);
    if house_of(facts, l6) == 9 || house_of(facts, l9) == 12 || house_of(facts, Sun) == 6 {
        parent_rules.push(
            lang.pick([
                "6th lord in 9th, 9th lord in 12th or Sun in 6th: the father's displeasure (V)",
                "ആറാം ഭാവാധിപൻ ഒമ്പതിൽ, ഒമ്പതാം ഭാവാധിപൻ പന്ത്രണ്ടിൽ, അല്ലെങ്കിൽ സൂര്യൻ ആറിൽ: അച്ഛന്റെ അതൃപ്തി (പ്രശ്നമാർഗ്ഗം)",
                "षष्ठेश नवम में, नवमेश द्वादश में या सूर्य षष्ठ में: पिता की नाराज़गी (प्रश्न मार्ग)",
            ])
            .into(),
        );
    }
    if house_of(facts, l6) == 4 || house_of(facts, l4) == 12 || house_of(facts, Moon) == 6 {
        parent_rules.push(
            lang.pick([
                "6th lord in 4th, 4th lord in 12th or Moon in 6th: the mother's displeasure (V)",
                "ആറാം ഭാവാധിപൻ നാലിൽ, നാലാം ഭാവാധിപൻ പന്ത്രണ്ടിൽ, അല്ലെങ്കിൽ ചന്ദ്രൻ ആറിൽ: അമ്മയുടെ അതൃപ്തി (പ്രശ്നമാർഗ്ഗം)",
                "षष्ठेश चतुर्थ में, चतुर्थेश द्वादश में या चंद्र षष्ठ में: माता की नाराज़गी (प्रश्न मार्ग)",
            ])
            .into(),
        );
    }
    if conj(facts, Sun, Rahu) {
        parent_rules.push(lang.pick(["Sun with Rahu: benevolent serpents, long-term good (V)", "സൂര്യൻ രാഹുവിനോടൊപ്പം: അനുഗ്രഹിക്കുന്ന സർപ്പങ്ങൾ, ദീർഘകാല നന്മ (പ്രശ്നമാർഗ്ഗം)", "सूर्य राहु के साथ: कृपालु नाग, लंबे समय का भला (प्रश्न मार्ग)"]).into());
    }
    if conj(facts, Moon, Rahu) {
        parent_rules.push(lang.pick(["Moon with Rahu: malicious serpents intent on disaster (V)", "ചന്ദ്രൻ രാഹുവിനോടൊപ്പം: സർപ്പങ്ങളെ പ്രീതിപ്പെടുത്തണം; നാഗാരാധന കൊണ്ട് ശാന്തമാകുന്നത് (പ്രശ്നമാർഗ്ഗം)", "चंद्र राहु के साथ: नागों को प्रसन्न करना है; नाग पूजा से शांत होता है (प्रश्न मार्ग)"]).into());
    }
    Badhaka {
        lagna_type,
        house,
        lord,
        companions,
        offence,
        remedy_kind,
        parent_rules,
    }
}

/// Badhaka as a dosha: present when the badhakesh has companions.
pub fn badhaka_dosha(facts: &Facts) -> Dosha {
    badhaka_dosha_in(facts, Lang::En)
}

/// [`badhaka_dosha`] in the reader's language.
pub fn badhaka_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    let b = badhaka_in(facts, lang);
    let mut f = Vec::new();
    for (g, being, how) in &b.companions {
        let w = if is_benefic(facts, *g) { 0.15 } else { 0.3 };
        f.push((w, format!("{how}: {being}")));
    }
    if let Some(o) = &b.offence {
        f.push((0.1, o.clone()));
    }
    for r in &b.parent_rules {
        f.push((0.1, r.clone()));
    }
    let lt = lagna_type_in(b.lagna_type, lang);
    let lw = gn(lang, b.lord);
    let l_dat = ml_forms(b.lord)[3];
    let hw = w::bhava(lang, b.house);
    let offended = if b.companions.is_empty() {
        match lang {
            Lang::En => format!(
                "no companion of the badhakesh {:?} ({} lagna, badhaka house {})",
                b.lord, b.lagna_type, b.house
            ),
            Lang::Ml => format!("ബാധകാധിപനായ {l_dat} കൂട്ടില്ല ({lt} ലഗ്നം, ബാധകസ്ഥാനം {hw} ഭാവം)"),
            Lang::Hi => format!("बाधकेश {lw} का कोई साथी नहीं ({lt} लग्न, बाधक स्थान {hw} भाव)"),
        }
    } else {
        b.companions
            .iter()
            .map(|c| c.1)
            .collect::<Vec<_>>()
            .join("; ")
    };
    let mut d = build(
        "Badhaka",
        "KS-R-BADH-01",
        b.lord,
        lang.pick([
            "Prasna Marga ch. 15 (Harihara): badhaka house by lagna type (movable 11th, fixed 9th, dual 7th); the planet in it, with or aspecting its lord names the offended being; the remedy kind follows the tattva of the badhakesh's dispositor; read for a specific trouble, not a whole life",
            "പ്രശ്നമാർഗ്ഗം അധ്യായം 15 (ഹരിഹരൻ): ലഗ്നത്തിന്റെ തരമനുസരിച്ച് ബാധകസ്ഥാനം (ചരത്തിന് പതിനൊന്ന്, സ്ഥിരത്തിന് ഒമ്പത്, ഉഭയത്തിന് ഏഴ്); അവിടെയുള്ളതോ അതിന്റെ അധിപനോടു ചേർന്നതോ അധിപനെ നോക്കുന്നതോ ആയ ഗ്രഹം പ്രീതിപ്പെടുത്തേണ്ട ശക്തിയെ കാണിക്കുന്നു; ബാധകാധിപന്റെ രാശ്യധിപന്റെ തത്ത്വമനുസരിച്ചാണ് പരിഹാരത്തിന്റെ തരം; ഒരു പ്രത്യേക പ്രശ്നത്തിനു വേണ്ടി നോക്കുന്നത്, ജീവിതം മുഴുവനുമല്ല",
            "प्रश्न मार्ग अध्याय 15 (हरिहर): लग्न के प्रकार से बाधक स्थान (चर में एकादश, स्थिर में नवम, द्विस्वभाव में सप्तम); उसमें बैठा, उसके स्वामी के साथ या उस पर दृष्टि डालने वाला ग्रह बताता है कि किसे प्रसन्न करना है; उपाय का प्रकार बाधकेश के राशि-स्वामी के तत्त्व से; किसी खास परेशानी के लिए देखा जाता है, पूरे जीवन के लिए नहीं",
        ]),
        lang.pick([
            "the badhakesh's family: propitiate the named being; kind by tattva",
            "ബാധകാധിപന്റെ വിഭാഗം: പറഞ്ഞ ശക്തിയെ പ്രീതിപ്പെടുത്തുക; തരം തത്ത്വമനുസരിച്ച്",
            "बाधकेश का वर्ग: बताई गई शक्ति को प्रसन्न करें; प्रकार तत्त्व के अनुसार",
        ]),
        offended,
        f,
    );
    let ch = w::bhava(lang, house_of(facts, b.lord));
    d.factors.insert(
        0,
        match lang {
            Lang::En => format!(
                "{} lagna: badhaka house {}, badhakesh {:?} in rasi house {}; remedy kind {}",
                b.lagna_type,
                b.house,
                b.lord,
                house_of(facts, b.lord),
                b.remedy_kind
            ),
            Lang::Ml => format!("{lt} ലഗ്നം: ബാധകസ്ഥാനം {hw} ഭാവം, ബാധകാധിപൻ {lw} {ch} ഭാവത്തിൽ; പരിഹാരത്തിന്റെ തരം {}", b.remedy_kind),
            Lang::Hi => format!("{lt} लग्न: बाधक स्थान {hw} भाव, बाधकेश {lw} {ch} भाव में; उपाय का प्रकार {}", b.remedy_kind),
        },
    );
    d
}

/// The fixed factor lines the Parashari yoga detector gives for Kala Sarpa and
/// Kemadruma, in the reader's language (the text there is English only).
fn yoga_factor_in(lang: Lang, en: &str) -> String {
    if lang == Lang::En {
        return en.to_string();
    }
    const T: &[[&str; 3]] = &[
        ["all seven grahas lie on one side of the Rahu–Ketu axis", "ഏഴു ഗ്രഹങ്ങളും രാഹു-കേതു അച്ചുതണ്ടിന്റെ ഒരു വശത്ത്", "सातों ग्रह राहु-केतु धुरी के एक ही ओर"],
        ["no planet in the 2nd or 12th sign from the Moon; none of the cancellations applies", "ചന്ദ്രന്റെ രണ്ടാം രാശിയിലോ പന്ത്രണ്ടാം രാശിയിലോ ഗ്രഹമില്ല; ഭംഗങ്ങളൊന്നും ബാധകമല്ല", "चंद्र से द्वितीय या द्वादश राशि में कोई ग्रह नहीं; कोई भंग लागू नहीं"],
        ["a planet stands in a kendra from the Moon", "ചന്ദ്രന്റെ ഒരു കേന്ദ്രത്തിൽ ഒരു ഗ്രഹമുണ്ട്", "चंद्र से एक केंद्र में कोई ग्रह है"],
        ["the Moon is in a kendra from the lagna", "ചന്ദ്രൻ ലഗ്നത്തിന്റെ ഒരു കേന്ദ്രത്തിൽ", "चंद्र लग्न से एक केंद्र में है"],
        ["Jupiter is with or aspects the Moon", "വ്യാഴം ചന്ദ്രനോടൊപ്പമോ ചന്ദ്രനെ നോക്കുന്നോ", "गुरु चंद्र के साथ है या उस पर दृष्टि डालता है"],
    ];
    let pick = |s: &str| T.iter().find(|t| t[0] == s).map(|t| lang.pick(*t).to_string());
    if let Some(t) = pick(en) {
        return t;
    }
    if let Some(rest) = en.strip_prefix("would be Kemadruma, but ") {
        let r = pick(rest).unwrap_or_else(|| rest.to_string());
        return say3!(lang, "", "കേമദ്രുമയോഗം ആകുമായിരുന്നു, പക്ഷേ {r}", "केमद्रुम योग बनता, पर {r}");
    }
    en.to_string()
}

/// Kala Sarpa (KS-Y-0070), as a dosha family of its own.
pub fn kala_sarpa_dosha(facts: &Facts) -> Dosha {
    kala_sarpa_dosha_in(facts, Lang::En)
}

/// [`kala_sarpa_dosha`] in the reader's language.
pub fn kala_sarpa_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    let y = yogas(facts);
    let f = y
        .iter()
        .filter(|y| y.name == "Kala Sarpa")
        .map(|y| (0.5, y.hit.factors.iter().map(|x| yoga_factor_in(lang, x)).collect::<Vec<_>>().join("; ")))
        .collect();
    build(
        "Kala Sarpa",
        "KS-R-YOGA-02",
        DashaLord::Rahu,
        lang.pick([
            "Later texts (not BPHS): all planets between Rahu and Ketu; many say it is cancelled by a strong benefic in a kendra",
            "പിൽക്കാല ഗ്രന്ഥങ്ങൾ (പരാശരഗ്രന്ഥത്തിലില്ല): എല്ലാ ഗ്രഹങ്ങളും രാഹുവിനും കേതുവിനും ഇടയിൽ; ഒരു കേന്ദ്രത്തിലെ ബലവാനായ ശുഭഗ്രഹം ഇതിനെ ഇല്ലാതാക്കുമെന്ന് പലരും പറയുന്നു",
            "बाद के ग्रंथ (पराशर के ग्रंथ में नहीं): सभी ग्रह राहु और केतु के बीच; बहुत से कहते हैं कि केंद्र में बलवान शुभ ग्रह इसे भंग करता है",
        ]),
        lang.pick([
            "Rahu: Naga puja on Panchami, Durga lamp in Rahu kala, no shortcut for 43 days",
            "രാഹു: പഞ്ചമിയിൽ നാഗപൂജ, രാഹുകാലത്ത് ദുർഗ്ഗയ്ക്ക് വിളക്ക്, 43 ദിവസം കുറുക്കുവഴികളില്ലാതെ",
            "राहु: पंचमी पर नाग पूजा, राहु काल में दुर्गा के लिए दीपक, 43 दिन कोई शॉर्टकट नहीं",
        ]),
        lang.pick(["the serpents (nodal axis)", "സർപ്പങ്ങൾ (രാഹു-കേതു അച്ചുതണ്ട്)", "नाग (राहु-केतु धुरी)"]).into(),
        f,
    )
}

/// Kemadruma (KS-Y-0050): present only when not cancelled; a cancelled Kemadruma is
/// reported absent with the cancellation in the factors.
pub fn kemadruma_dosha(facts: &Facts) -> Dosha {
    kemadruma_dosha_in(facts, Lang::En)
}

/// [`kemadruma_dosha`] in the reader's language.
pub fn kemadruma_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    let y = yogas(facts);
    let mut f = Vec::new();
    let mut cancelled = Vec::new();
    for y in &y {
        if y.name == "Kemadruma" {
            f.push((0.4, y.hit.factors.iter().map(|x| yoga_factor_in(lang, x)).collect::<Vec<_>>().join("; ")));
        } else if y.name == "Kemadruma (cancelled)" {
            cancelled = y.hit.factors.iter().map(|x| yoga_factor_in(lang, x)).collect();
        }
    }
    let mut d = build(
        "Kemadruma",
        "KS-R-YOGA-04",
        DashaLord::Moon,
        lang.pick([
            "BPHS, Chandra yogas; Phaladeepika 6; Jataka Parijata (bhanga)",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ചന്ദ്രയോഗങ്ങൾ; ഫലദീപിക 6; ജാതകപാരിജാതം (ഭംഗം)",
            "बृहत् पराशर होरा शास्त्र, चंद्र योग; फलदीपिका 6; जातक पारिजात (भंग)",
        ]),
        lang.pick([
            "Moon: Somavara vrata, serve the mother, fixed sleep, abhisheka on Monday",
            "ചന്ദ്രൻ: തിങ്കളാഴ്ച വ്രതം, അമ്മയെ പരിചരിക്കുക, ഉറക്കത്തിന് കൃത്യസമയം, തിങ്കളാഴ്ച അഭിഷേകം",
            "चंद्र: सोमवार व्रत, माँ की सेवा, सोने का तय समय, सोमवार को अभिषेक",
        ]),
        lang.pick(["the Moon alone (the mind unsupported)", "ഒറ്റയ്ക്കായ ചന്ദ്രൻ (മനസ്സിന് താങ്ങു കുറവ്)", "अकेला चंद्र (मन को सहारा कम)"]).into(),
        f,
    );
    if !cancelled.is_empty() {
        d.factors = cancelled;
    }
    d
}

/// Guru Chandala: Jupiter with Rahu (with Ketu, a weaker reading some texts give).
pub fn guru_chandala_dosha(facts: &Facts) -> Dosha {
    guru_chandala_dosha_in(facts, Lang::En)
}

/// [`guru_chandala_dosha`] in the reader's language.
pub fn guru_chandala_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    if conj(facts, Jupiter, Rahu) {
        f.push((0.6, lang.pick(["Jupiter is conjoined with Rahu", "വ്യാഴം രാഹുവിനോടൊപ്പം യോഗത്തിൽ", "गुरु की राहु से युति"]).into()));
    } else if conj(facts, Jupiter, Ketu) {
        f.push((
            0.35,
            lang.pick(["Jupiter is conjoined with Ketu (the weaker reading some texts give)", "വ്യാഴം കേതുവിനോടൊപ്പം യോഗത്തിൽ (ചില ഗ്രന്ഥങ്ങൾ നൽകുന്ന ദുർബലമായ വായന)", "गुरु की केतु से युति (कुछ ग्रंथों का हल्का पाठ)"]).into(),
        ));
    }
    build(
        "Guru Chandala",
        "KS-R-YOGA-05",
        Jupiter,
        lang.pick([
            "Later compendia (not yet a yoga id in BPHS terms): Jupiter joined with Rahu",
            "പിൽക്കാല സമാഹാരങ്ങൾ (പരാശരഗ്രന്ഥത്തിന്റെ കണക്കിൽ ഇതുവരെ ഒരു യോഗമായി ചേർത്തിട്ടില്ല): വ്യാഴം രാഹുവിനോടു ചേർന്ന്",
            "बाद के संग्रह (पराशर के ग्रंथ के हिसाब से अभी कोई योग नहीं): गुरु राहु के साथ",
        ]),
        lang.pick([
            "Jupiter: Thursday Guru worship, honour teachers, turmeric and chana daana, Vishnu Sahasranama; stop one half-truth taught to others",
            "വ്യാഴം: വ്യാഴാഴ്ച ഗുരുപൂജ, ഗുരുക്കന്മാരെ ആദരിക്കുക, മഞ്ഞളും കടലയും ദാനം, വിഷ്ണുസഹസ്രനാമം; മറ്റുള്ളവരെ പഠിപ്പിക്കുന്ന ഒരു അർദ്ധസത്യം നിർത്തുക",
            "गुरु: गुरुवार को गुरु पूजा, शिक्षकों का सम्मान, हल्दी और चने का दान, विष्णु सहस्रनाम; दूसरों को सिखाया जाने वाला एक आधा-सच छोड़ें",
        ]),
        lang.pick(["the guru, truth taught", "ഗുരു, പഠിപ്പിക്കുന്ന സത്യം", "गुरु, सिखाया गया सत्य"]).into(),
        f,
    )
}

/// Grahan dosha: the Sun or the Moon with a node in the natal chart.
pub fn grahan_dosha(facts: &Facts) -> Dosha {
    grahan_dosha_in(facts, Lang::En)
}

/// [`grahan_dosha`] in the reader's language.
pub fn grahan_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let mut planet = Sun;
    for lum in [Sun, Moon] {
        for node in [Rahu, Ketu] {
            if conj(facts, lum, node) {
                let sep = (facts.planet(lum).sidereal - facts.planet(node).sidereal)
                    .rem_euclid(360.0)
                    .min(
                        (facts.planet(node).sidereal - facts.planet(lum).sidereal)
                            .rem_euclid(360.0),
                    );
                let wgt = if lum == Moon { 0.5 } else { 0.4 } + if sep < 12.0 { 0.2 } else { 0.0 };
                let lw = gn(lang, lum);
                let nw = gn(lang, node);
                let n_soc = ml_forms(node)[1];
                let near = sep < 12.0;
                f.push((
                    wgt,
                    match lang {
                        Lang::En => format!(
                            "{lum:?} is with {node:?}, {sep:.1}° apart{}",
                            if near { " (within an eclipse limit)" } else { "" }
                        ),
                        Lang::Ml => format!(
                            "{lw} {n_soc}, {sep:.1}° അകലത്തിൽ{}",
                            if near { " (ഗ്രഹണപരിധിക്കുള്ളിൽ)" } else { "" }
                        ),
                        Lang::Hi => format!(
                            "{lw} {nw} के साथ, {sep:.1}° की दूरी पर{}",
                            if near { " (ग्रहण की सीमा के भीतर)" } else { "" }
                        ),
                    },
                ));
                if lum == Moon {
                    planet = Moon;
                }
            }
        }
    }
    build(
        "Grahan dosha",
        "KS-R-YOGA-06",
        planet,
        lang.pick([
            "Practice (eclipse birth or a luminary with a node); Grahana shanti is performed during the next eclipse, japa from first to last contact (eclipse contacts on the roadmap)",
            "ആചാരം (ഗ്രഹണസമയത്തെ ജനനം, അല്ലെങ്കിൽ സൂര്യനോ ചന്ദ്രനോ രാഹുകേതുക്കളോടൊപ്പം); അടുത്ത ഗ്രഹണസമയത്ത് ഗ്രഹണശാന്തി, ഗ്രഹണം തുടങ്ങുന്നതു മുതൽ തീരുന്നതു വരെ ജപം (ഗ്രഹണസ്പർശങ്ങൾ പിന്നീട് ചേർക്കും)",
            "परंपरा (ग्रहण में जन्म, या सूर्य या चंद्र राहु-केतु के साथ); अगले ग्रहण में ग्रहण शांति, स्पर्श से मोक्ष तक जप (ग्रहण के स्पर्श आगे जोड़े जाएँगे)",
        ]),
        lang.pick([
            "the luminary: Grahana shanti at the next eclipse; Sun or Moon remedies by weekday",
            "സൂര്യനോ ചന്ദ്രനോ: അടുത്ത ഗ്രഹണത്തിൽ ഗ്രഹണശാന്തി; ആഴ്ചയിലെ ദിവസമനുസരിച്ച് സൂര്യന്റെയോ ചന്ദ്രന്റെയോ പരിഹാരങ്ങൾ",
            "सूर्य या चंद्र: अगले ग्रहण पर ग्रहण शांति; वार के अनुसार सूर्य या चंद्र के उपाय",
        ]),
        lang.pick(["the Sun or the Moon under the nodes", "രാഹുകേതുക്കളുടെ സ്വാധീനത്തിൽ സൂര്യനോ ചന്ദ്രനോ", "राहु-केतु के प्रभाव में सूर्य या चंद्र"]).into(),
        f,
    )
}

/// The cancellation lines of `kuta::mangal_dosha`, in the reader's language.
fn mangal_cancel_in(lang: Lang, en: &str) -> String {
    const T: &[[&str; 3]] = &[
        ["Mars in its own sign", "ചൊവ്വ സ്വക്ഷേത്രത്തിൽ", "मंगल अपनी राशि में"],
        ["Mars exalted in Capricorn", "ചൊവ്വ മകരത്തിൽ ഉച്ചത്തിൽ", "मंगल मकर में उच्च"],
        ["Mars in Cancer or Leo (held harmless by some texts)", "ചൊവ്വ കർക്കടകത്തിലോ ചിങ്ങത്തിലോ (ചില ഗ്രന്ഥങ്ങൾ ഇത് ദോഷമല്ലെന്നു കരുതുന്നു)", "मंगल कर्क या सिंह में (कुछ ग्रंथ इसे निर्दोष मानते हैं)"],
        ["Jupiter aspects or joins Mars", "വ്യാഴം ചൊവ്വയെ നോക്കുന്നു അല്ലെങ്കിൽ ചൊവ്വയോടു ചേരുന്നു", "गुरु मंगल पर दृष्टि डालता है या उसके साथ है"],
    ];
    T.iter().find(|t| t[0] == en).map(|t| lang.pick(*t).to_string()).unwrap_or_else(|| en.to_string())
}

/// Mangal dosha, through the existing detector with its cancellations.
pub fn mangal_dosha_as_dosha(facts: &Facts) -> Dosha {
    mangal_dosha_as_dosha_in(facts, Lang::En)
}

/// [`mangal_dosha_as_dosha`] in the reader's language.
pub fn mangal_dosha_as_dosha_in(facts: &Facts, lang: Lang) -> Dosha {
    let m = mangal_dosha(facts);
    let mut f = Vec::new();
    if m.effective {
        let (a, b, c) = (w::bhava(lang, m.from_lagna), w::bhava(lang, m.from_moon), w::bhava(lang, m.from_venus));
        f.push((
            if m.from_lagna_present { 0.6 } else { 0.4 },
            match lang {
                Lang::En => format!(
                    "Mars in house {} from the lagna, {} from the Moon, {} from Venus",
                    m.from_lagna, m.from_moon, m.from_venus
                ),
                Lang::Ml => format!("ചൊവ്വ ലഗ്നത്തിൽ നിന്ന് {a} ഭാവത്തിൽ, ചന്ദ്രനിൽ നിന്ന് {b}, ശുക്രനിൽ നിന്ന് {c}"),
                Lang::Hi => format!("मंगल लग्न से {a} भाव में, चंद्र से {b}, शुक्र से {c}"),
            },
        ));
    }
    let mut d = build(
        "Mangal dosha",
        "KS-R-YOGA-03",
        DashaLord::Mars,
        lang.pick([
            "Mars in 1, 2, 4, 7, 8 or 12 from the lagna, the Moon or Venus, with the standard cancellations (kuta.rs)",
            "ലഗ്നം, ചന്ദ്രൻ, ശുക്രൻ ഇവയിൽ നിന്ന് 1, 2, 4, 7, 8, 12 ഭാവങ്ങളിൽ ചൊവ്വ, സാധാരണ പറയുന്ന ഭംഗങ്ങളോടെ",
            "लग्न, चंद्र या शुक्र से 1, 2, 4, 7, 8 या 12 भाव में मंगल, सामान्य भंग नियमों के साथ",
        ]),
        lang.pick([
            "Mars: Tuesday Hanuman vow, red-lentil daana, Mangala Chandika stotra; end one quarrel; Kumbha-vivaha (ritual, optional)",
            "ചൊവ്വ: ചൊവ്വാഴ്ച ഹനുമാൻ വ്രതം, ചുവന്ന പരിപ്പ് ദാനം, മംഗളചണ്ഡികാ സ്തോത്രം; ഒരു പിണക്കം തീർക്കുക; കുംഭവിവാഹം (ചടങ്ങ്, നിർബന്ധമില്ല)",
            "मंगल: मंगलवार को हनुमान व्रत, मसूर दाल का दान, मंगल चंडिका स्तोत्र; एक झगड़ा खत्म करें; कुंभ विवाह (अनुष्ठान, ऐच्छिक)",
        ]),
        lang.pick(["the spouse's house (Mars's heat)", "ജീവിതപങ്കാളിയുടെ ഭാവം (ചൊവ്വയുടെ ചൂട്)", "जीवनसाथी का भाव (मंगल की गर्मी)"]).into(),
        f,
    );
    if m.present && !m.effective {
        d.factors = m
            .cancellations
            .iter()
            .map(|c| {
                let cw = mangal_cancel_in(lang, c);
                say3!(lang, "would be Mangal dosha, but {c}", "ചൊവ്വാദോഷം ആകുമായിരുന്നു, പക്ഷേ {cw}", "मंगल दोष बनता, पर {cw}")
            })
            .collect();
    }
    d
}

/// Sade Sati from its phase (0 none, 1 rising, 2 peak, 3 setting).
pub fn sade_sati_dosha(phase: u8, at: Instant) -> Dosha {
    sade_sati_dosha_in(phase, at, Lang::En)
}

/// [`sade_sati_dosha`] in the reader's language.
pub fn sade_sati_dosha_in(phase: u8, at: Instant, lang: Lang) -> Dosha {
    let t = crate::remedy_timing::fmt_utc(at);
    let f = match phase {
        1 => vec![(
            0.5,
            say3!(lang, "phase 1: Saturn in the 12th from the natal Moon at {t}", "ഒന്നാം ഘട്ടം: {t}-ന് ശനി ജന്മചന്ദ്രന്റെ പന്ത്രണ്ടാം ഭാവത്തിൽ", "पहला चरण: {t} पर शनि जन्म-चंद्र से द्वादश भाव में"),
        )],
        2 => vec![(
            0.8,
            say3!(lang, "phase 2: Saturn on the natal Moon sign at {t}", "രണ്ടാം ഘട്ടം: {t}-ന് ശനി ജന്മചന്ദ്രന്റെ രാശിയിൽ", "दूसरा चरण: {t} पर शनि जन्म-चंद्र की राशि में"),
        )],
        3 => vec![(
            0.5,
            say3!(lang, "phase 3: Saturn in the 2nd from the natal Moon at {t}", "മൂന്നാം ഘട്ടം: {t}-ന് ശനി ജന്മചന്ദ്രന്റെ രണ്ടാം ഭാവത്തിൽ", "तीसरा चरण: {t} पर शनि जन्म-चंद्र से द्वितीय भाव में"),
        )],
        _ => Vec::new(),
    };
    build(
        "Sade Sati",
        "KS-R-YOGA-01",
        DashaLord::Saturn,
        lang.pick([
            "Phaladeepika 26: Saturn's transit through the 12th, 1st and 2nd from the natal Moon; phase ingresses solved exactly",
            "ഫലദീപിക 26: ജന്മചന്ദ്രന്റെ പന്ത്രണ്ടിലും ഒന്നിലും രണ്ടിലും കൂടിയുള്ള ശനിയുടെ ഗോചരം; ഓരോ ഘട്ടത്തിന്റെയും തുടക്കം കൃത്യമായി കണക്കാക്കിയത്",
            "फलदीपिका 26: जन्म-चंद्र से द्वादश, प्रथम और द्वितीय में शनि का गोचर; हर चरण का प्रवेश ठीक-ठीक निकाला गया",
        ]),
        lang.pick([
            "Saturn: finish the longest-postponed duty; serve elders and labourers on Saturdays; black til, mustard oil, iron or a blanket in Shani hora; Shani Trayodashi, 7 Saturdays; Hanuman Chalisa; tila-taila lamp (optional)",
            "ശനി: ഏറ്റവും നീട്ടിവെച്ച ഒരു കടമ പൂർത്തിയാക്കുക; ശനിയാഴ്ചകളിൽ മുതിർന്നവരെയും തൊഴിലാളികളെയും സഹായിക്കുക; ശനിഹോരയിൽ കറുത്ത എള്ള്, കടുകെണ്ണ, ഇരുമ്പ് അല്ലെങ്കിൽ ഒരു പുതപ്പ് ദാനം; ശനിത്രയോദശി, 7 ശനിയാഴ്ച; ഹനുമാൻ ചാലീസ; എള്ളെണ്ണ വിളക്ക് (നിർബന്ധമില്ല)",
            "शनि: सबसे लंबे समय से टला हुआ कर्तव्य पूरा करें; शनिवार को बड़ों और मज़दूरों की सेवा; शनि की होरा में काले तिल, सरसों का तेल, लोहा या कंबल; शनि त्रयोदशी, 7 शनिवार; हनुमान चालीसा; तिल के तेल का दीपक (ऐच्छिक)",
        ]),
        lang.pick(["Saturn over the Moon", "ചന്ദ്രന്റെ മേൽ ശനി", "चंद्र के ऊपर शनि"]).into(),
        f,
    )
}

/// Every dosha family, present or not, in a fixed order.
pub fn doshas(facts: &Facts, at: Instant, sade_sati_phase: u8) -> Vec<Dosha> {
    doshas_in(facts, at, sade_sati_phase, Lang::En)
}

/// [`doshas`] in the reader's language: factors, offended, source and remedy family
/// translated; `name` and `rule_id` stay English (use [`Dosha::name_in`] to show it).
pub fn doshas_in(facts: &Facts, at: Instant, sade_sati_phase: u8, lang: Lang) -> Vec<Dosha> {
    vec![
        pitru_dosha_in(facts, lang),
        sarpa_dosha_in(facts, lang),
        matru_shapa_in(facts, lang),
        bhratru_shapa_in(facts, lang),
        matula_shapa_in(facts, lang),
        brahma_shapa_in(facts, lang),
        patni_shapa_in(facts, lang),
        preta_shapa_in(facts, lang),
        badhaka_dosha_in(facts, lang),
        kala_sarpa_dosha_in(facts, lang),
        kemadruma_dosha_in(facts, lang),
        guru_chandala_dosha_in(facts, lang),
        grahan_dosha_in(facts, lang),
        mangal_dosha_as_dosha_in(facts, lang),
        sade_sati_dosha_in(sade_sati_phase, at, lang),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PlanetFact;
    use DashaLord::*;

    fn planet(
        graha: DashaLord,
        sidereal: f64,
        lagna_sign: u8,
        lords: &[DashaLord; 12],
    ) -> PlanetFact {
        let sign = ((sidereal / 30.0).floor() as usize % 12) as u8;
        let rasi_house = ((sign as i32 - lagna_sign as i32).rem_euclid(12) + 1) as u8;
        let owns = (0..12u8)
            .filter(|i| lords[*i as usize] == graha)
            .map(|i| i + 1)
            .collect();
        PlanetFact {
            graha,
            sidereal,
            house: rasi_house,
            rasi_house,
            sign,
            star_lord: Moon,
            sub_lord: Moon,
            owns,
            retrograde: false,
        }
    }

    /// A clean base chart (Aries lagna, every planet alone, no 5th-house occupant), with
    /// overrides applied; Ketu follows Rahu.
    fn synth(lagna_sign: u8, overrides: &[(DashaLord, f64)]) -> Facts {
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] =
            std::array::from_fn(|i| ((lagna_sign as f64 + i as f64) * 30.0 + 5.0) % 360.0);
        let base = [
            (Sun, 10.0),
            (Moon, 70.0),
            (Mars, 275.0),
            (Mercury, 40.0),
            (Jupiter, 250.0),
            (Venus, 340.0),
            (Saturn, 190.0),
            (Rahu, 100.0),
            (Ketu, 280.0),
        ];
        let mut planets = Vec::new();
        for (g, lon) in base {
            let lon = overrides
                .iter()
                .find(|(o, _)| *o == g)
                .map(|(_, l)| *l)
                .unwrap_or(lon);
            planets.push(planet(g, lon, lagna_sign, &rasi_lords));
        }
        let rahu = planets[7].sidereal;
        planets[8] = planet(
            Ketu,
            (rahu + 180.0).rem_euclid(360.0),
            lagna_sign,
            &rasi_lords,
        );
        Facts {
            planets,
            cusps,
            cusp_sub_lords: [Moon; 12],
            cusp_star_lords: [Moon; 12],
            house_lords: rasi_lords,
            lagna_sign,
            rasi_lords,
            moon_phase_deg: 60.0,
        }
    }

    #[test]
    fn a_clean_chart_has_no_curse() {
        let f = synth(0, &[]);
        for d in [
            pitru_dosha(&f),
            sarpa_dosha(&f),
            matru_shapa(&f),
            bhratru_shapa(&f),
            matula_shapa(&f),
            brahma_shapa(&f),
            patni_shapa(&f),
            preta_shapa(&f),
            guru_chandala_dosha(&f),
            grahan_dosha(&f),
        ] {
            assert!(!d.present, "{} fired: {:?}", d.name, d.factors);
            assert_eq!(d.severity, 0.0);
        }
    }

    #[test]
    fn each_curse_fires_on_its_karaka_in_the_fifth() {
        // Aries lagna: the 5th is Leo (120–150)
        type Case = (&'static str, Facts, fn(&Facts) -> Dosha);
        let cases: Vec<Case> = vec![
            (
                "pitru",
                synth(0, &[(Sun, 125.0), (Saturn, 130.0)]),
                pitru_dosha,
            ),
            (
                "sarpa",
                synth(0, &[(Rahu, 125.0), (Mars, 128.0)]),
                sarpa_dosha,
            ),
            (
                "matru",
                synth(0, &[(Moon, 100.0), (Saturn, 105.0)]),
                matru_shapa,
            ),
            (
                "bhratru",
                synth(0, &[(Mars, 125.0), (Saturn, 130.0)]),
                bhratru_shapa,
            ),
            (
                "matula",
                synth(0, &[(Mercury, 125.0), (Saturn, 128.0)]),
                matula_shapa,
            ),
            (
                "brahma",
                synth(0, &[(Jupiter, 125.0), (Saturn, 128.0)]),
                brahma_shapa,
            ),
            (
                "patni",
                synth(0, &[(Venus, 185.0), (Mars, 190.0)]),
                patni_shapa,
            ),
            (
                "preta",
                synth(0, &[(Sun, 10.0), (Rahu, 195.0)]),
                preta_shapa,
            ), // Ketu at 15°, with the 5th lord Sun
            (
                "guru chandala",
                synth(0, &[(Jupiter, 95.0), (Rahu, 100.0)]),
                guru_chandala_dosha,
            ),
            (
                "grahan",
                synth(0, &[(Moon, 100.0), (Rahu, 102.0)]),
                grahan_dosha,
            ),
        ];
        for (name, f, det) in cases {
            let d = det(&f);
            assert!(d.present, "{name} should be present");
            assert!(
                !d.factors.is_empty() && d.severity > 0.0 && d.severity <= 1.0,
                "{name}"
            );
            assert!(!d.source.is_empty() && !d.remedy_family.is_empty());
        }
        let g = grahan_dosha(&synth(0, &[(Moon, 100.0), (Rahu, 102.0)]));
        assert!(g.factors[0].contains("eclipse limit"));
        assert_eq!(g.planet, Moon);
    }

    #[test]
    fn badhaka_house_follows_the_lagna_type() {
        let movable = badhaka(&synth(0, &[]));
        assert_eq!(
            (movable.lagna_type, movable.house, movable.lord),
            ("movable", 11, Saturn)
        );
        let fixed = badhaka(&synth(1, &[]));
        assert_eq!(
            (fixed.lagna_type, fixed.house, fixed.lord),
            ("fixed", 9, Saturn)
        ); // Taurus → Capricorn
        let dual = badhaka(&synth(2, &[]));
        assert_eq!(
            (dual.lagna_type, dual.house, dual.lord),
            ("dual", 7, Jupiter)
        ); // Gemini → Sagittarius
           // Rahu in Aquarius, the badhaka house of Aries: a serpent deity
        let f = synth(0, &[(Rahu, 310.0)]);
        let b = badhaka(&f);
        assert!(
            b.companions
                .iter()
                .any(|c| c.0 == Rahu && c.1.contains("serpent")),
            "{:?}",
            b.companions
        );
        let d = badhaka_dosha(&f);
        assert!(d.present && d.offended.contains("serpent"));
        assert_eq!(d.planet, Saturn);
        // Mars in the lagna names the broken idol
        let f2 = synth(0, &[(Mars, 20.0), (Rahu, 310.0)]);
        assert!(badhaka(&f2).offence.is_some());
    }

    #[test]
    fn sade_sati_follows_the_phase() {
        let at = Instant::j2000();
        assert!(!sade_sati_dosha(0, at).present);
        let p2 = sade_sati_dosha(2, at);
        assert!(p2.present && p2.severity > sade_sati_dosha(1, at).severity);
    }

    #[test]
    fn kochi_doshas_are_consistent_with_the_yoga_and_kuta_detectors() {
        use kshana_chart::{compute, ChartInput};
        use kshana_ephem::Spk;
        use kshana_houses::HouseSystem;
        use kshana_sky::Observer;
        use kshana_time::Utc;
        use kshana_vedic::Ayanamsa;
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let t = Utc::new(1990, 10, 29, 23, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: t,
                observer: Observer {
                    latitude_deg: 9.9312,
                    longitude_deg: 76.2673,
                    height_m: 0.0,
                },
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let f = Facts::from_chart(&ch);
        let all = doshas(&f, t, 1);
        assert_eq!(all.len(), 15);
        for d in &all {
            assert!((0.0..=1.0).contains(&d.severity), "{}", d.name);
            if d.present {
                assert!(!d.factors.is_empty(), "{}", d.name);
                assert!(d.severity > 0.0);
            } else {
                assert_eq!(d.severity, 0.0);
            }
            assert!(!d.source.is_empty() && !d.rule_id.is_empty());
        }
        let names: Vec<&str> = yogas(&f).iter().map(|y| y.name).collect();
        let ks = all.iter().find(|d| d.name == "Kala Sarpa").unwrap();
        assert_eq!(ks.present, names.contains(&"Kala Sarpa"));
        let km = all.iter().find(|d| d.name == "Kemadruma").unwrap();
        assert_eq!(km.present, names.contains(&"Kemadruma"));
        let md = all.iter().find(|d| d.name == "Mangal dosha").unwrap();
        assert_eq!(md.present, mangal_dosha(&f).effective);
        assert!(all.iter().find(|d| d.name == "Sade Sati").unwrap().present);
        let b = all.iter().find(|d| d.name == "Badhaka").unwrap();
        assert!(b.factors[0].contains("badhaka house"));
    }

    /// A Latin word of three letters or more that is not a short all-capital acronym
    /// (UTC, KP) is English left in Malayalam or Hindi text.
    pub(crate) fn latin_word(s: &str) -> Option<String> {
        s.split(|c: char| !c.is_ascii_alphabetic())
            .find(|w| w.len() >= 3 && !(w.len() <= 4 && w.chars().all(|c| c.is_ascii_uppercase())))
            .map(str::to_string)
    }

    fn charts() -> Vec<Facts> {
        let sets: &[&[(DashaLord, f64)]] = &[
            &[],
            &[(Sun, 125.0), (Saturn, 130.0), (Mars, 127.0)],
            &[(Rahu, 125.0), (Mars, 128.0)],
            &[(Moon, 100.0), (Saturn, 105.0)],
            &[(Mars, 125.0), (Saturn, 130.0)],
            &[(Mercury, 125.0), (Saturn, 128.0)],
            &[(Jupiter, 125.0), (Saturn, 128.0)],
            &[(Venus, 185.0), (Mars, 190.0)],
            &[(Sun, 10.0), (Rahu, 195.0)],
            &[(Jupiter, 95.0), (Rahu, 100.0)],
            &[(Moon, 100.0), (Rahu, 102.0)],
            &[(Sun, 12.0), (Rahu, 14.0), (Mars, 20.0)],
            &[(Rahu, 310.0)],
            &[(Mars, 20.0), (Rahu, 310.0)],
            &[(Mars, 300.0), (Jupiter, 10.0), (Moon, 160.0)],
        ];
        (0..12u8).flat_map(|l| sets.iter().map(move |s| synth(l, s))).collect()
    }

    #[test]
    fn english_is_unchanged_by_the_lang_twins() {
        let at = Instant::j2000();
        for f in charts() {
            for phase in 0..4 {
                assert_eq!(doshas_in(&f, at, phase, Lang::En), doshas(&f, at, phase));
            }
            assert_eq!(badhaka_in(&f, Lang::En), badhaka(&f));
        }
    }

    #[test]
    fn malayalam_and_hindi_leave_no_english_and_stay_gentle() {
        let at = Instant::j2000();
        let mut fired = 0;
        for lang in [Lang::Ml, Lang::Hi] {
            for f in charts() {
                for phase in 0..4 {
                    let en = doshas(&f, at, phase);
                    let v = doshas_in(&f, at, phase, lang);
                    for (d, e) in v.iter().zip(&en) {
                        assert_eq!((d.name, d.rule_id, d.present, d.severity), (e.name, e.rule_id, e.present, e.severity));
                        assert_eq!(d.factors.len(), e.factors.len());
                        fired += d.factors.len();
                        let name = d.name_in(lang);
                        assert_ne!(name, d.name);
                        for s in d.factors.iter().map(String::as_str).chain([d.offended.as_str(), d.source, d.remedy_family, name]) {
                            assert_eq!(latin_word(s), None, "{lang:?} {}: {s}", d.rule_id);
                            for bad in ["മരണ", "മൃത്യു", "അപകട", "मृत्यु", "मरण", "मौत", "दुर्घटना", "घातक"] {
                                assert!(!s.contains(bad), "{s}");
                            }
                        }
                    }
                }
            }
        }
        assert!(fired > 200, "{fired}");
    }
}
