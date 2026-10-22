//! The karma gate: what a remedy may claim, decided before any remedy is shown.
//!
//! Two taxonomies, both computed from the chart and the running periods:
//!
//! **(A) Sanchita, prarabdha, kriyamana** (the Vedantic store, the portion ripening
//! in this life, and what is being made now). The commentators agree a remedy cannot
//! touch sanchita and only negotiates with prarabdha; kriyamana is entirely in the
//! person's hands. Sivananda says prarabdha can be changed by right action, other
//! acharyas say it cannot; this module states the disagreement and does not pick a
//! side. The chart is a map of prarabdha, so the kind in play is read from the
//! period: a cause whose planet rules a running period is prarabdha ripening now; a
//! natal affliction no period brings is sanchita, stored; a sound planet with no
//! period pressing is kriyamana, where only present conduct counts.
//!
//! **(B) Dridha and adridha** (Prasna Marga ch. 1, stanza 101 ff., B. V. Raman's
//! translation): results flow from dridha (fixed, from deliberate action) and adridha
//! (not fixed, from unintentional action); later commentators add dridha-adridha
//! (mixed). The chart test (ch. 15): the evil karma is dridha or adridha according as
//! the indicator planet stands in the house of the Moon or of the Sun, the Kerala
//! guide reading "house" as the sign or the hora. The hora of a sign follows BPHS
//! ch. 7: in an odd sign the first fifteen degrees are the Sun's hora and the second
//! the Moon's; in an even sign the reverse.
//!
//! The concurrence count is a practitioner extension, not in the text: an affliction
//! shown by the natal yoga, the running dasha and the current transit together is
//! dridha; by one alone, adridha; by two, mixed. The rule used here, so the label is a
//! function of exact facts: with `m` the Moon test, `s` the Sun test and `c` the
//! concurrence out of three, the class is dridha when `m` holds and `c = 3`, adridha
//! when `s` holds and `c ≤ 1`, and dridha-adridha otherwise.
//!
//! What each class lets a remedy claim (the remedy ladder, finding "what each rung may
//! claim"): dridha is endured with discipline, the texts promising no relief and asking
//! for grace and patience; dridha-adridha is softened by remedy after intense and
//! prolonged effort (the full purascharana, repeated vows); adridha is changed by action
//! and "can be undone entirely" by parihara.

use crate::areas::Area;
use crate::facts::Facts;
use crate::parashari::{dignity, is_benefic, Dignity};
use kshana_vedic::DashaLord;
use crate::plain::Lang;
use crate::say3;
use crate::doshas::ml_forms;
use crate::yogas::words as w;

/// Dridha (fixed), dridha-adridha (mixed) or adridha (not fixed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaClass {
    /// Fixed: deliberate karma; endured with discipline, no relief promised.
    Dridha,
    /// Mixed: softened by remedy after intense and prolonged effort.
    DridhaAdridha,
    /// Not fixed: changed by action; the texts say it can be undone entirely.
    Adridha,
}

impl KarmaClass {
    /// Sanskrit label.
    pub fn name(self) -> &'static str {
        match self {
            KarmaClass::Dridha => "dridha",
            KarmaClass::DridhaAdridha => "dridha-adridha",
            KarmaClass::Adridha => "adridha",
        }
    }

    /// What the texts allow a remedy to claim for this class, in one line.
    pub fn may_claim(self) -> &'static str {
        match self {
            KarmaClass::Dridha => "endured with discipline: the texts do not promise relief for this; they ask for grace and patience, and conduct is offered as dignity, not as a bargain",
            KarmaClass::DridhaAdridha => "softened by remedy: the texts allow relief after intense and prolonged effort, the full dose repeated (whole purascharana, vows kept to their count)",
            KarmaClass::Adridha => "changed by action: the texts say this can be undone entirely by parihara, and new karma is entirely in your hands",
        }
    }

    /// The Sanskrit label in the reader's script: "dridha", "ദൃഢം", "दृढ़".
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            KarmaClass::Dridha => ["dridha", "ദൃഢം", "दृढ़"],
            KarmaClass::DridhaAdridha => ["dridha-adridha", "ദൃഢാദൃഢം", "दृढ़-अदृढ़"],
            KarmaClass::Adridha => ["adridha", "അദൃഢം", "अदृढ़"],
        })
    }

    /// [`KarmaClass::may_claim`] in the reader's language.
    pub fn may_claim_in(self, lang: Lang) -> &'static str {
        match self {
            KarmaClass::Dridha => lang.pick([
                self.may_claim(),
                "ക്ഷമയോടെ, ചിട്ടയോടെ കടന്നുപോകേണ്ടത്: ഇതിന് ആശ്വാസം ഗ്രന്ഥങ്ങൾ വാഗ്ദാനം ചെയ്യുന്നില്ല; അനുഗ്രഹവും ക്ഷമയുമാണ് അവ ആവശ്യപ്പെടുന്നത്. നല്ല നടപ്പ് ഒരു വിലപേശലല്ല, അന്തസ്സായി അർപ്പിക്കുന്നതാണ്",
                "धैर्य और अनुशासन से सहना: ग्रंथ इसमें राहत का वचन नहीं देते; वे कृपा और धीरज माँगते हैं, और अच्छा आचरण सौदे की तरह नहीं, गरिमा की तरह अर्पित होता है",
            ]),
            KarmaClass::DridhaAdridha => lang.pick([
                self.may_claim(),
                "പരിഹാരം കൊണ്ട് മയപ്പെടുന്നത്: തീവ്രവും ദീർഘവുമായ പ്രയത്നത്തിനു ശേഷം ആശ്വാസം ഗ്രന്ഥങ്ങൾ അനുവദിക്കുന്നു; മുഴുവൻ അളവും ആവർത്തിച്ച് (സമ്പൂർണ്ണ പുരശ്ചരണം, വ്രതങ്ങൾ എണ്ണം തികച്ച്)",
                "उपाय से नरम होता है: ग्रंथ तीव्र और लंबे प्रयास के बाद राहत मानते हैं, पूरी मात्रा दोहराकर (पूरा पुरश्चरण, व्रत अपनी गिनती तक)",
            ]),
            KarmaClass::Adridha => lang.pick([
                self.may_claim(),
                "പ്രവൃത്തി കൊണ്ട് മാറുന്നത്: പരിഹാരം കൊണ്ട് ഇത് പൂർണ്ണമായി നീക്കാമെന്ന് ഗ്രന്ഥങ്ങൾ പറയുന്നു; പുതിയ കർമ്മം മുഴുവനും നിങ്ങളുടെ കൈയിലാണ്",
                "कर्म से बदलता है: ग्रंथ कहते हैं कि परिहार से यह पूरी तरह मिट सकता है, और नया कर्म पूरी तरह आपके हाथ में है",
            ]),
        }
    }
}

/// Which store the cause is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaKind {
    /// Stored, not ripening in any running period.
    Sanchita,
    /// Ripening now: the planet rules a running period.
    Prarabdha,
    /// Being made now: nothing from the past presses; only present conduct counts.
    Kriyamana,
}

impl KarmaKind {
    /// Sanskrit label.
    pub fn name(self) -> &'static str {
        match self {
            KarmaKind::Sanchita => "sanchita",
            KarmaKind::Prarabdha => "prarabdha",
            KarmaKind::Kriyamana => "kriyamana",
        }
    }

    /// The Sanskrit label in the reader's script.
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            KarmaKind::Sanchita => ["sanchita", "സഞ്ചിതം", "संचित"],
            KarmaKind::Prarabdha => ["prarabdha", "പ്രാരബ്ധം", "प्रारब्ध"],
            KarmaKind::Kriyamana => ["kriyamana", "ക്രിയമാണം", "क्रियमाण"],
        })
    }
}

/// Sidereal signs of the slow transits at the instant of judgement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitSigns {
    /// Saturn's sidereal sign, 0 = Aries.
    pub saturn: u8,
    /// Jupiter's sidereal sign.
    pub jupiter: u8,
    /// Rahu's sidereal sign (true node).
    pub rahu: u8,
    /// Ketu's sidereal sign.
    pub ketu: u8,
}

/// The verdict: class, kind in play, the evidence and what a remedy may claim.
#[derive(Clone, Debug, PartialEq)]
pub struct KarmaVerdict {
    /// Dridha, dridha-adridha or adridha.
    pub class: KarmaClass,
    /// Sanchita, prarabdha or kriyamana.
    pub kind_in_play: KarmaKind,
    /// Which tests fired, in words the evidence drawer can show.
    pub basis: Vec<String>,
    /// What the texts let a remedy claim for this class.
    pub may_claim: &'static str,
    /// Concurrence out of three (natal yoga, running dasha, current transit).
    pub concurrence: u8,
    /// The text behind the rule.
    pub source: &'static str,
    /// Stable rule id.
    pub rule_id: &'static str,
}

/// Source line for every verdict.
pub const SOURCE: &str = "Prasna Marga ch. 1 st. 101 ff. and ch. 15 (Raman): dridha or adridha as the indicator stands in the Moon's or the Sun's sign or hora; BPHS ch. 7 (hora of a sign); the three-fold concurrence of yoga, dasha and transit is practitioner convention, not the text; sanchita, prarabdha and kriyamana per the Vedantic commentators, who disagree on whether prarabdha can be changed";

/// [`SOURCE`] in the reader's language.
pub fn source_in(lang: Lang) -> &'static str {
    lang.pick([
        SOURCE,
        "പ്രശ്നമാർഗ്ഗം അധ്യായം 1 ശ്ലോകം 101 മുതൽ, അധ്യായം 15 (രാമൻ): സൂചകഗ്രഹം ചന്ദ്രന്റെയോ സൂര്യന്റെയോ രാശിയിലോ ഹോരയിലോ നിൽക്കുന്നതനുസരിച്ച് ദൃഢം അല്ലെങ്കിൽ അദൃഢം; ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 7 (രാശിയുടെ ഹോര); യോഗം, ദശ, ഗോചരം എന്നിവ മൂന്നും ഒത്തുവരുന്ന കണക്ക് ആചാര്യന്മാരുടെ രീതിയാണ്, ഗ്രന്ഥത്തിലുള്ളതല്ല; സഞ്ചിതം, പ്രാരബ്ധം, ക്രിയമാണം എന്നിവ വേദാന്ത വ്യാഖ്യാതാക്കൾ പറയുന്നതുപോലെ; പ്രാരബ്ധം മാറ്റാനാകുമോ എന്നതിൽ അവർക്കിടയിൽ അഭിപ്രായഭേദമുണ്ട്",
        "प्रश्न मार्ग अध्याय 1 श्लोक 101 से आगे और अध्याय 15 (रमन): सूचक ग्रह चंद्र या सूर्य की राशि या होरा में हो, उसी अनुसार दृढ़ या अदृढ़; बृहत् पराशर होरा शास्त्र अध्याय 7 (राशि की होरा); योग, दशा और गोचर का तिहरा मेल ज्योतिषियों की परंपरा है, ग्रंथ का नहीं; संचित, प्रारब्ध और क्रियमाण वेदांत के टीकाकारों के अनुसार, जिनमें इस पर मतभेद है कि प्रारब्ध बदल सकता है या नहीं",
    ])
}

/// The Sun's or the Moon's hora of a sidereal longitude (BPHS ch. 7): odd signs give
/// the Sun the first half and the Moon the second; even signs the reverse.
pub fn hora_lord_of(sidereal: f64) -> DashaLord {
    let sign = (sidereal.rem_euclid(360.0) / 30.0).floor() as u32;
    let first_half = sidereal.rem_euclid(30.0) < 15.0;
    let odd = sign % 2 == 0; // Aries is the first (odd) sign
    if odd == first_half {
        DashaLord::Sun
    } else {
        DashaLord::Moon
    }
}

fn is_dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}

fn sign_name(s: u8) -> &'static str {
    [
        "Aries",
        "Taurus",
        "Gemini",
        "Cancer",
        "Leo",
        "Virgo",
        "Libra",
        "Scorpio",
        "Sagittarius",
        "Capricorn",
        "Aquarius",
        "Pisces",
    ][s as usize % 12]
}

/// Classify the karma behind a located cause: the planet `lord` afflicting the area's
/// house (or, with no area, the house it occupies), given the running period lords
/// (maha first) and, when known, the slow transits at the instant.
pub fn classify(
    facts: &Facts,
    lord: DashaLord,
    area: Option<Area>,
    running: &[DashaLord],
    transits: Option<&TransitSigns>,
) -> KarmaVerdict {
    classify_in(facts, lord, area, running, transits, Lang::En)
}

/// [`classify`] with the evidence, the claim and the source in the reader's language.
pub fn classify_in(
    facts: &Facts,
    lord: DashaLord,
    area: Option<Area>,
    running: &[DashaLord],
    transits: Option<&TransitSigns>,
    lang: Lang,
) -> KarmaVerdict {
    let p = facts.planet(lord);
    let house = area.map(|a| a.primary).unwrap_or(p.rasi_house);
    let house_sign = ((facts.lagna_sign as u16 + house as u16 - 1) % 12) as u8;
    let house_lord = facts.rasi_lords[(house - 1) as usize];
    let mut basis = Vec::new();
    let g = |x: DashaLord| w::graha(lang, x);
    let lord_w = g(lord);
    let lord_gen = ml_forms(lord)[0];

    // (B) the Moon / Sun test: sign first, then the hora of the sign
    let (moon_test, sun_test) = match p.sign {
        3 => {
            basis.push(say3!(
                lang,
                "{lord:?} stands in Cancer, the Moon's sign: the Prasna Marga test reads the karma as fixed (dridha-leaning)",
                "{lord_w} ചന്ദ്രന്റെ രാശിയായ കർക്കടകത്തിൽ നിൽക്കുന്നു: പ്രശ്നമാർഗ്ഗത്തിലെ പരിശോധനയിൽ കർമ്മം ഉറച്ചതാണ് (ദൃഢത്തിലേക്ക് ചായുന്നു)",
                "{lord_w} चंद्र की राशि कर्क में है: प्रश्न मार्ग की कसौटी पर कर्म स्थिर है (दृढ़ की ओर झुका)"
            ));
            (true, false)
        }
        4 => {
            basis.push(say3!(
                lang,
                "{lord:?} stands in Leo, the Sun's sign: the Prasna Marga test reads the karma as not fixed (adridha-leaning)",
                "{lord_w} സൂര്യന്റെ രാശിയായ ചിങ്ങത്തിൽ നിൽക്കുന്നു: പ്രശ്നമാർഗ്ഗത്തിലെ പരിശോധനയിൽ കർമ്മം ഉറച്ചതല്ല (അദൃഢത്തിലേക്ക് ചായുന്നു)",
                "{lord_w} सूर्य की राशि सिंह में है: प्रश्न मार्ग की कसौटी पर कर्म स्थिर नहीं है (अदृढ़ की ओर झुका)"
            ));
            (false, true)
        }
        s => {
            let h = hora_lord_of(p.sidereal);
            let first = p.sidereal.rem_euclid(30.0) < 15.0;
            let half = if first { "first" } else { "second" };
            let deg = p.sidereal.rem_euclid(30.0);
            let moon_hora = h == DashaLord::Moon;
            basis.push(match lang {
                Lang::En => format!(
                    "{lord:?} at {:.2}° of {} stands in the {half} half, the {h:?}'s hora of the sign (BPHS ch. 7): the Prasna Marga test reads the karma as {}",
                    deg,
                    sign_name(s),
                    if moon_hora { "fixed (dridha-leaning)" } else { "not fixed (adridha-leaning)" }
                ),
                Lang::Ml => format!(
                    "{lord_w} {} രാശിയിൽ {deg:.2}°-ൽ, രാശിയുടെ {} പകുതിയിൽ, {} ഹോരയിൽ നിൽക്കുന്നു (ബൃഹത് പരാശര ഹോരാശാസ്ത്രം അധ്യായം 7): പ്രശ്നമാർഗ്ഗത്തിലെ പരിശോധനയിൽ കർമ്മം {}",
                    w::rasi(lang, s as usize),
                    if first { "ആദ്യ" } else { "രണ്ടാം" },
                    ml_forms(h)[0],
                    if moon_hora { "ഉറച്ചതാണ് (ദൃഢത്തിലേക്ക് ചായുന്നു)" } else { "ഉറച്ചതല്ല (അദൃഢത്തിലേക്ക് ചായുന്നു)" }
                ),
                Lang::Hi => format!(
                    "{lord_w} {} राशि के {deg:.2}° पर, राशि के {} आधे में, {} की होरा में है (बृहत् पराशर होरा शास्त्र अध्याय 7): प्रश्न मार्ग की कसौटी पर कर्म {}",
                    w::rasi(lang, s as usize),
                    if first { "पहले" } else { "दूसरे" },
                    g(h),
                    if moon_hora { "स्थिर है (दृढ़ की ओर झुका)" } else { "स्थिर नहीं है (अदृढ़ की ओर झुका)" }
                ),
            });
            (h == DashaLord::Moon, h == DashaLord::Sun)
        }
    };

    // concurrence 1: the natal affliction holds
    let d = dignity(lord, p.sidereal);
    let d_w = w::dignity(lang, d);
    let owns_dusthana = p.owns.iter().any(|h| is_dusthana(*h));
    let owns_trikona = p.owns.iter().any(|h| matches!(h, 1 | 5 | 9));
    let with_malefic: Vec<DashaLord> = facts
        .planets
        .iter()
        .filter(|q| q.graha != lord && q.sign == p.sign && !is_benefic(facts, q.graha))
        .map(|q| q.graha)
        .collect();
    let rh = p.rasi_house;
    let rh_w = w::bhava(lang, rh);
    let mut natal_reasons = Vec::new();
    if d <= Dignity::Enemy {
        natal_reasons.push(say3!(
            lang,
            "{lord:?} is {d:?} by dignity",
            "{lord_w} {d_w} അവസ്ഥയിൽ",
            "{lord_w} {d_w} में"
        ));
    }
    if is_dusthana(rh) {
        natal_reasons.push(say3!(
            lang,
            "{lord:?} sits in rasi house {rh}, a dusthana",
            "{lord_w} ദുഃസ്ഥാനമായ {rh_w} ഭാവത്തിൽ",
            "{lord_w} दुःस्थान, {rh_w} भाव में"
        ));
    }
    if owns_dusthana && !owns_trikona {
        let owns = w::houses(&p.owns);
        natal_reasons.push(match lang {
            Lang::En => format!(
                "{lord:?} owns a dusthana ({:?}) and no trikona: a functional malefic for this lagna",
                p.owns
            ),
            Lang::Ml => format!("{lord_w} ഒരു ദുഃസ്ഥാനത്തിന്റെ ({owns}) അധിപനാണ്, ത്രികോണാധിപനല്ല: ഈ ലഗ്നത്തിന് പാപഫലം നൽകുന്ന ഗ്രഹം"),
            Lang::Hi => format!("{lord_w} एक दुःस्थान ({owns}) का स्वामी है और किसी त्रिकोण का नहीं: इस लग्न के लिए कार्येश पापी"),
        });
    }
    if !with_malefic.is_empty() {
        let wm = w::graha_list(lang, &with_malefic);
        natal_reasons.push(say3!(
            lang,
            "{lord:?} is conjoined with {with_malefic:?}",
            "{lord_w} {wm} എന്നിവയോടൊപ്പം യോഗത്തിൽ",
            "{lord_w} की {wm} से युति"
        ));
    }
    let natal = !natal_reasons.is_empty();
    if natal {
        basis.push(say3!(
            lang,
            "natal yoga holds: {}",
            "ജനനയോഗം നിലനിൽക്കുന്നു: {}",
            "जन्म का योग बना है: {}",
            natal_reasons.join("; ")
        ));
    } else {
        basis.push(say3!(
            lang,
            "no natal affliction: {lord:?} is {d:?} in rasi house {rh} with no malefic beside it",
            "ജനനത്തിൽ പീഡയില്ല: {lord_w} {rh_w} ഭാവത്തിൽ {d_w} അവസ്ഥയിൽ, അടുത്ത് പാപഗ്രഹമില്ല",
            "जन्म में पीड़ा नहीं: {lord_w} {rh_w} भाव में {d_w} में है, पास में कोई पापी ग्रह नहीं"
        ));
    }

    // concurrence 2: the running dasha
    let runs = running.contains(&lord);
    let house_lord_runs = running.contains(&house_lord);
    let hl_w = g(house_lord);
    let hl_gen = ml_forms(house_lord)[0];
    let house_w = w::bhava(lang, house);
    if runs {
        basis.push(say3!(
            lang,
            "running dasha: {lord:?} rules a running period",
            "നടപ്പുദശ: {lord_w} ഇപ്പോൾ നടക്കുന്ന ഒരു ദശാകാലത്തിന്റെ നാഥനാണ്",
            "चल रही दशा: {lord_w} एक चालू दशा-काल का स्वामी है"
        ));
    } else if house_lord_runs {
        basis.push(say3!(
            lang,
            "running dasha: {house_lord:?}, lord of the {house}th, rules a running period",
            "നടപ്പുദശ: {house_w} ഭാവാധിപനായ {hl_w} ഇപ്പോൾ നടക്കുന്ന ഒരു ദശാകാലത്തിന്റെ നാഥനാണ്",
            "चल रही दशा: {house_w} भाव का स्वामी {hl_w} एक चालू दशा-काल का स्वामी है"
        ));
    } else {
        basis.push(say3!(
            lang,
            "no running period of {lord:?} or of the {house}th lord {house_lord:?}",
            "{lord_gen}യോ {house_w} ഭാവാധിപനായ {hl_gen}യോ ദശാകാലം ഇപ്പോൾ നടക്കുന്നില്ല",
            "{lord_w} या {house_w} भाव के स्वामी {hl_w} का कोई दशा-काल अभी नहीं चल रहा"
        ));
    }
    let running_hit = runs || house_lord_runs;

    // concurrence 3: the slow transits over the planet or the house
    let transit_hit = match transits {
        Some(t) => {
            let mut hits = Vec::new();
            for (name, who, s) in [
                ("Saturn", DashaLord::Saturn, t.saturn),
                ("Jupiter", DashaLord::Jupiter, t.jupiter),
                ("Rahu", DashaLord::Rahu, t.rahu),
                ("Ketu", DashaLord::Ketu, t.ketu),
            ] {
                let who_w = g(who);
                let s_w = w::rasi(lang, s as usize);
                let sn = sign_name(s);
                if s == p.sign {
                    hits.push(say3!(
                        lang,
                        "{name} transits {sn}, over {lord:?}",
                        "{who_w} {s_w} രാശിയിലൂടെ, {lord_gen} മേൽ സഞ്ചരിക്കുന്നു",
                        "{who_w} {s_w} राशि में, {lord_w} के ऊपर से गोचर कर रहा है",
                    ));
                } else if s == house_sign {
                    hits.push(say3!(
                        lang,
                        "{name} transits {sn}, the {house}th house",
                        "{who_w} {house_w} ഭാവമായ {s_w} രാശിയിലൂടെ സഞ്ചരിക്കുന്നു",
                        "{who_w} {house_w} भाव, {s_w} राशि में गोचर कर रहा है",
                    ));
                }
            }
            if hits.is_empty() {
                basis.push(
                    lang.pick([
                        "no slow transit (Saturn, Jupiter, the nodes) over the planet or the house now",
                        "ഇപ്പോൾ ഗ്രഹത്തിന്റെയോ ഭാവത്തിന്റെയോ മേൽ മന്ദഗ്രഹങ്ങളുടെ (ശനി, വ്യാഴം, രാഹു-കേതുക്കൾ) ഗോചരമില്ല",
                        "अभी ग्रह या भाव के ऊपर धीमे ग्रहों (शनि, गुरु, राहु-केतु) का गोचर नहीं",
                    ])
                    .into(),
                );
                false
            } else {
                basis.push(say3!(lang, "transit: {}", "ഗോചരം: {}", "गोचर: {}", hits.join("; ")));
                true
            }
        }
        None => {
            basis.push(
                lang.pick([
                    "transits not supplied: the transit leg of the concurrence is not counted",
                    "ഗോചരവിവരം നൽകിയിട്ടില്ല: ഒത്തുവരവിൽ ഗോചരത്തിന്റെ പങ്ക് എണ്ണിയിട്ടില്ല",
                    "गोचर की जानकारी नहीं दी गई: मेल में गोचर का हिस्सा नहीं गिना गया",
                ])
                .into(),
            );
            false
        }
    };

    let concurrence = natal as u8 + running_hit as u8 + transit_hit as u8;
    basis.push(say3!(
        lang,
        "concurrence {concurrence} of 3 (natal yoga, running dasha, transit)",
        "ഒത്തുവരവ് 3-ൽ {concurrence} (ജനനയോഗം, നടപ്പുദശ, ഗോചരം)",
        "मेल 3 में से {concurrence} (जन्म का योग, चल रही दशा, गोचर)"
    ));

    let class = if moon_test && concurrence == 3 {
        KarmaClass::Dridha
    } else if sun_test && concurrence <= 1 {
        KarmaClass::Adridha
    } else {
        KarmaClass::DridhaAdridha
    };
    basis.push(match lang {
        Lang::En => format!(
            "rule: dridha when the Moon test holds and all three concur; adridha when the Sun test holds and at most one concurs; otherwise dridha-adridha → {}",
            class.name()
        ),
        Lang::Ml => format!(
            "നിയമം: ചന്ദ്രപരിശോധന ശരിയാകുകയും മൂന്നും ഒത്തുവരികയും ചെയ്താൽ ദൃഢം; സൂര്യപരിശോധന ശരിയാകുകയും ഒന്നിൽ കൂടുതൽ ഒത്തുവരാതിരിക്കുകയും ചെയ്താൽ അദൃഢം; അല്ലെങ്കിൽ ദൃഢാദൃഢം → {}",
            class.name_in(lang)
        ),
        Lang::Hi => format!(
            "नियम: चंद्र-कसौटी सही हो और तीनों मिलें तो दृढ़; सूर्य-कसौटी सही हो और अधिक से अधिक एक मिले तो अदृढ़; नहीं तो दृढ़-अदृढ़ → {}",
            class.name_in(lang)
        ),
    });

    let kind_in_play = if runs {
        basis.push(say3!(
            lang,
            "prarabdha: the period of {lord:?} is ripening it now",
            "പ്രാരബ്ധം: {lord_gen} ദശാകാലം ഇതിനെ ഇപ്പോൾ ഫലത്തിലെത്തിക്കുന്നു",
            "प्रारब्ध: {lord_w} का दशा-काल इसे अभी फल तक ला रहा है"
        ));
        KarmaKind::Prarabdha
    } else if natal {
        basis.push(say3!(
            lang,
            "sanchita: the chart carries it but no running period of {lord:?} brings it now",
            "സഞ്ചിതം: ജാതകത്തിൽ ഇതുണ്ട്, പക്ഷേ {lord_gen} ദശാകാലം ഇപ്പോൾ നടക്കാത്തതിനാൽ ഇത് ഇപ്പോൾ ഫലിക്കുന്നില്ല",
            "संचित: कुंडली में यह है, पर {lord_w} का कोई दशा-काल अभी इसे नहीं ला रहा"
        ));
        KarmaKind::Sanchita
    } else {
        basis.push(
            lang.pick([
                "kriyamana: nothing from the past presses here; what you do now is what counts",
                "ക്രിയമാണം: ഇവിടെ പഴയതൊന്നും അമർത്തുന്നില്ല; ഇപ്പോൾ നിങ്ങൾ ചെയ്യുന്നതാണ് പ്രധാനം",
                "क्रियमाण: यहाँ अतीत का कोई दबाव नहीं; अभी आप जो करते हैं, वही मायने रखता है",
            ])
            .into(),
        );
        KarmaKind::Kriyamana
    };

    KarmaVerdict {
        class,
        kind_in_play,
        basis,
        may_claim: class.may_claim_in(lang),
        concurrence,
        source: source_in(lang),
        rule_id: "KS-R-KARMA-01",
    }
}

/// What a remedy on rung `rung` (1 karma … 7 gem) may claim for a verdict, per the
/// ladder mapping: kriyamana → rungs 1–3 are the remedy; adridha → rungs 1–6 can
/// destroy it; dridha-adridha → rungs 1–6 with the full dose; dridha → rungs 1–3 as
/// endurance and dignity, 4–6 as devotion with no promise; rung 7 is information only.
pub fn may_claim(class: KarmaClass, kind: KarmaKind, rung: u8) -> &'static str {
    if rung >= 7 {
        return "information only: a gem strengthens its planet; nothing here recommends a purchase and no claim is made";
    }
    if kind == KarmaKind::Kriyamana {
        return if rung <= 3 {
            "the remedy itself: conduct, service and discipline are the only things that change new karma"
        } else {
            "support only: nothing from the past presses here, so this adds devotion, not relief"
        };
    }
    match (class, rung <= 3) {
        (KarmaClass::Dridha, true) => "endurance and dignity: the texts do not promise relief for this; they ask for grace and patience",
        (KarmaClass::Dridha, false) => "devotion: the texts do not promise relief for dridha karma; this is offered, not bargained",
        (KarmaClass::DridhaAdridha, true) => "the work itself; relief, the texts say, after intense and prolonged effort",
        (KarmaClass::DridhaAdridha, false) => "softens with the full dose repeated: the whole purascharana, vows kept to their count",
        (KarmaClass::Adridha, true) => "changes it: the texts say an adridha affliction can be undone entirely",
        (KarmaClass::Adridha, false) => "the texts say it can be undone entirely by parihara; this supports the conduct above",
    }
}

/// [`may_claim`] in the reader's language.
pub fn may_claim_in(class: KarmaClass, kind: KarmaKind, rung: u8, lang: Lang) -> &'static str {
    let en = may_claim(class, kind, rung);
    if lang == Lang::En {
        return en;
    }
    let t: [&'static str; 2] = if rung >= 7 {
        [
            "അറിവിനു മാത്രം: രത്നം അതിന്റെ ഗ്രഹത്തെ ബലപ്പെടുത്തുന്നു; ഇവിടെ ഒന്നും വാങ്ങാൻ നിർദ്ദേശിക്കുന്നില്ല, ഒരു ഫലവും അവകാശപ്പെടുന്നില്ല",
            "केवल जानकारी के लिए: रत्न अपने ग्रह को बल देता है; यहाँ कुछ खरीदने की सलाह नहीं, और कोई दावा नहीं",
        ]
    } else if kind == KarmaKind::Kriyamana {
        if rung <= 3 {
            [
                "ഇതുതന്നെയാണ് പരിഹാരം: പുതിയ കർമ്മം മാറ്റുന്നത് നല്ല നടപ്പും സേവനവും ചിട്ടയും മാത്രമാണ്",
                "यही उपाय है: नए कर्म को बस आचरण, सेवा और अनुशासन ही बदलते हैं",
            ]
        } else {
            [
                "താങ്ങു മാത്രം: ഇവിടെ പഴയതൊന്നും അമർത്തുന്നില്ല, അതിനാൽ ഇത് ഭക്തി കൂട്ടുന്നു, ആശ്വാസമല്ല",
                "केवल सहारा: यहाँ अतीत का कोई दबाव नहीं, इसलिए यह भक्ति जोड़ता है, राहत नहीं",
            ]
        }
    } else {
        match (class, rung <= 3) {
            (KarmaClass::Dridha, true) => [
                "ക്ഷമയും അന്തസ്സും: ഇതിന് ആശ്വാസം ഗ്രന്ഥങ്ങൾ വാഗ്ദാനം ചെയ്യുന്നില്ല; അനുഗ്രഹവും ക്ഷമയുമാണ് അവ ആവശ്യപ്പെടുന്നത്",
                "धैर्य और गरिमा: ग्रंथ इसमें राहत का वचन नहीं देते; वे कृपा और धीरज माँगते हैं",
            ],
            (KarmaClass::Dridha, false) => [
                "ഭക്തി: ദൃഢകർമ്മത്തിന് ആശ്വാസം ഗ്രന്ഥങ്ങൾ വാഗ്ദാനം ചെയ്യുന്നില്ല; ഇത് അർപ്പിക്കുന്നതാണ്, വിലപേശലല്ല",
                "भक्ति: ग्रंथ दृढ़ कर्म में राहत का वचन नहीं देते; यह अर्पण है, सौदा नहीं",
            ],
            (KarmaClass::DridhaAdridha, true) => [
                "ചെയ്യേണ്ട പ്രവൃത്തി തന്നെ; തീവ്രവും ദീർഘവുമായ പ്രയത്നത്തിനു ശേഷം ആശ്വാസം എന്ന് ഗ്രന്ഥങ്ങൾ",
                "यही असली काम है; ग्रंथ कहते हैं कि तीव्र और लंबे प्रयास के बाद राहत मिलती है",
            ],
            (KarmaClass::DridhaAdridha, false) => [
                "മുഴുവൻ അളവും ആവർത്തിച്ചാൽ മയപ്പെടുന്നു: സമ്പൂർണ്ണ പുരശ്ചരണം, വ്രതങ്ങൾ എണ്ണം തികച്ച്",
                "पूरी मात्रा दोहराने से नरम होता है: पूरा पुरश्चरण, व्रत अपनी गिनती तक",
            ],
            (KarmaClass::Adridha, true) => [
                "ഇതിനെ മാറ്റുന്നു: അദൃഢമായ പീഡ പൂർണ്ണമായി നീക്കാമെന്ന് ഗ്രന്ഥങ്ങൾ പറയുന്നു",
                "यह इसे बदलता है: ग्रंथ कहते हैं कि अदृढ़ पीड़ा पूरी तरह मिट सकती है",
            ],
            (KarmaClass::Adridha, false) => [
                "പരിഹാരം കൊണ്ട് ഇത് പൂർണ്ണമായി നീക്കാമെന്ന് ഗ്രന്ഥങ്ങൾ പറയുന്നു; ഇത് മുകളിലെ നല്ല നടപ്പിന് താങ്ങാകുന്നു",
                "ग्रंथ कहते हैं कि परिहार से यह पूरी तरह मिट सकता है; यह ऊपर के आचरण को सहारा देता है",
            ],
        }
    };
    lang.pick([en, t[0], t[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PlanetFact;
    use kshana_vedic::sign_lord;
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

    /// Aries lagna. Saturn in Cancer (4th) at 20°: the Moon's sign; Mars in Leo 5° (5th,
    /// the Sun's sign); Venus in Scorpio 20° (8th, the Moon's hora of an odd sign);
    /// Jupiter in Taurus 10° (2nd, even sign first half = Moon's hora).
    fn synthetic() -> Facts {
        let lagna_sign = 0u8;
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] = std::array::from_fn(|i| (i as f64 * 30.0 + 5.0) % 360.0);
        let p = |g, lon| planet(g, lon, lagna_sign, &rasi_lords);
        Facts {
            planets: vec![
                p(Sun, 10.0),
                p(Moon, 100.0),
                p(Mars, 125.0),
                p(Mercury, 75.0),
                p(Jupiter, 40.0),
                p(Venus, 230.0),
                p(Saturn, 110.0),
                p(Rahu, 230.0),
                p(Ketu, 50.0),
            ],
            cusps,
            cusp_sub_lords: [Moon; 12],
            cusp_star_lords: [Moon; 12],
            house_lords: rasi_lords,
            lagna_sign,
            rasi_lords,
            moon_phase_deg: 90.0,
        }
    }

    #[test]
    fn hora_of_a_sign_follows_bphs_seven() {
        assert_eq!(hora_lord_of(5.0), Sun); // Aries first half
        assert_eq!(hora_lord_of(20.0), Moon); // Aries second half
        assert_eq!(hora_lord_of(35.0), Moon); // Taurus first half
        assert_eq!(hora_lord_of(50.0), Sun); // Taurus second half
        assert_eq!(hora_lord_of(230.0), Sun); // Scorpio is the 8th (even) sign: second half is the Sun's
        assert_eq!(hora_lord_of(215.0), Moon); // Scorpio first half
    }

    #[test]
    fn dridha_needs_the_moon_test_and_full_concurrence() {
        let f = synthetic();
        // Saturn in Cancer, Enemy (natal), running, Saturn transiting Cancer: all three
        let t = TransitSigns {
            saturn: 3,
            jupiter: 0,
            rahu: 0,
            ketu: 6,
        };
        let v = classify(&f, Saturn, None, &[Saturn, Moon], Some(&t));
        assert_eq!(v.concurrence, 3, "{:?}", v.basis);
        assert_eq!(v.class, KarmaClass::Dridha, "{:?}", v.basis);
        assert_eq!(v.kind_in_play, KarmaKind::Prarabdha);
        assert!(v.may_claim.contains("do not promise"));
        assert!(v.basis.iter().any(|b| b.contains("Cancer")));
        // same planet with nothing running and no transit: mixed (Moon test, c = 1)
        let v2 = classify(
            &f,
            Saturn,
            None,
            &[Jupiter],
            Some(&TransitSigns {
                saturn: 9,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v2.concurrence, 1);
        assert_eq!(v2.class, KarmaClass::DridhaAdridha);
        assert_eq!(v2.kind_in_play, KarmaKind::Sanchita);
    }

    #[test]
    fn adridha_needs_the_sun_test_and_little_concurrence() {
        let f = synthetic();
        // Mars in Leo: Sun's sign; Mars owns 1 and 8 (trikona too), Own? Mars in Leo is Friend;
        // not in a dusthana; Rahu not with it → no natal affliction; not running; no transit
        let v = classify(
            &f,
            Mars,
            None,
            &[Jupiter, Mercury],
            Some(&TransitSigns {
                saturn: 9,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v.concurrence, 0, "{:?}", v.basis);
        assert_eq!(v.class, KarmaClass::Adridha, "{:?}", v.basis);
        assert_eq!(v.kind_in_play, KarmaKind::Kriyamana);
        // with the house lord running and a transit, two concur: mixed
        let v2 = classify(
            &f,
            Mars,
            None,
            &[Sun],
            Some(&TransitSigns {
                saturn: 4,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v2.concurrence, 2, "{:?}", v2.basis);
        assert_eq!(v2.class, KarmaClass::DridhaAdridha);
    }

    #[test]
    fn missing_transits_are_stated_not_assumed() {
        let f = synthetic();
        let v = classify(&f, Venus, Some(crate::areas::AREAS[1]), &[Venus], None);
        assert!(v.basis.iter().any(|b| b.contains("not supplied")));
        assert!(v.concurrence <= 2);
        assert_eq!(v.source, SOURCE);
    }

    #[test]
    fn rung_claims_follow_the_ladder() {
        assert!(may_claim(KarmaClass::Dridha, KarmaKind::Prarabdha, 1).contains("endurance"));
        assert!(may_claim(KarmaClass::Dridha, KarmaKind::Prarabdha, 5).contains("devotion"));
        assert!(may_claim(KarmaClass::Adridha, KarmaKind::Prarabdha, 2).contains("undone entirely"));
        assert!(may_claim(KarmaClass::Adridha, KarmaKind::Kriyamana, 5).contains("support only"));
        assert!(may_claim(KarmaClass::DridhaAdridha, KarmaKind::Sanchita, 4).contains("full dose"));
        assert!(
            may_claim(KarmaClass::Adridha, KarmaKind::Prarabdha, 7).contains("information only")
        );
    }

    fn latin_word(s: &str) -> Option<String> {
        s.split(|c: char| !c.is_ascii_alphabetic())
            .find(|w| w.len() >= 3 && !(w.len() <= 4 && w.chars().all(|c| c.is_ascii_uppercase())))
            .map(str::to_string)
    }

    #[test]
    fn every_language_reads_the_same_verdict() {
        let f = synthetic();
        let sets = [
            TransitSigns { saturn: 3, jupiter: 0, rahu: 0, ketu: 6 },
            TransitSigns { saturn: 9, jupiter: 9, rahu: 9, ketu: 9 },
            TransitSigns { saturn: 4, jupiter: 7, rahu: 1, ketu: 7 },
        ];
        let lords = [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu];
        let runs: [&[DashaLord]; 3] = [&[Saturn, Moon], &[Jupiter], &[Sun, Venus, Mars]];
        for g in lords {
            for area in [None, Some(crate::areas::AREAS[1]), Some(crate::areas::AREAS[3])] {
                for r in runs {
                    for t in sets.iter().map(Some).chain([None]) {
                        let en = classify(&f, g, area, r, t);
                        assert_eq!(classify_in(&f, g, area, r, t, Lang::En), en);
                        for lang in [Lang::Ml, Lang::Hi] {
                            let v = classify_in(&f, g, area, r, t, lang);
                            assert_eq!((v.class, v.kind_in_play, v.concurrence, v.basis.len()), (en.class, en.kind_in_play, en.concurrence, en.basis.len()));
                            for s in v.basis.iter().map(String::as_str).chain([v.may_claim, v.source, v.class.name_in(lang), v.kind_in_play.name_in(lang)]) {
                                assert_eq!(latin_word(s), None, "{lang:?}: {s}");
                            }
                        }
                    }
                }
            }
        }
        for c in [KarmaClass::Dridha, KarmaClass::DridhaAdridha, KarmaClass::Adridha] {
            for k in [KarmaKind::Sanchita, KarmaKind::Prarabdha, KarmaKind::Kriyamana] {
                for rung in 1..=7 {
                    assert_eq!(may_claim_in(c, k, rung, Lang::En), may_claim(c, k, rung));
                    for lang in [Lang::Ml, Lang::Hi] {
                        assert_eq!(latin_word(may_claim_in(c, k, rung, lang)), None);
                    }
                }
            }
        }
    }
}
