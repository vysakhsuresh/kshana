//! Where this comes from: one registry of the books, chapters and customs behind every
//! rule the engine uses to say something, so each claim on a screen can be traced.
//!
//! # Numbering
//!
//! Brihat Parashara Hora Shastra chapters are numbered as in R. Santhanam's translation
//! (Ranjan Publications, two volumes), the numbering the rest of the engine's doc
//! comments use. Where prints differ (the curses of the previous birth: ch. 85 in
//! G. C. Sharma's numbering, 83–85 in Santhanam's by print) the entry says so. A chapter
//! the engine has not checked against a print is named by its title and given no
//! number, and no verse is ever given unless the engine's own tables cite it.
//!
//! # Kinds
//!
//! * `text`: a classical or standard text (BPHS, Phaladeepika, the KP Readers …).
//! * `folk`: a later folk text, said as such (Lal Kitab, 1939–1952).
//! * `custom`: temple or household custom, not a text (the Navagraha temples of Kerala
//!   and Tamil Nadu, the weekday vow counts, the 48-day mandala).
//! * `kshana`: the app's own method, with a one-line explanation (the pooling of the
//!   ways of reading, the score's weights, deeds chosen from what a planet stands for).
//!
//! # Keys
//!
//! A key is stable: `bphs.dashas`, `lal_kitab`, `kshana.consensus`. Some keys are
//! aliases that expand to several sources (`life.dasha`, `system.kp`, `fact.mother`,
//! `remedy.deeds`), and `yoga.<key>` reads the yoga catalogue's own citation
//! ([`crate::yogas::CATALOG`]). [`cite`] resolves a list of keys to the sources in the
//! reader's language, each once, in order; unknown keys are skipped.

use crate::plain::Lang;
use crate::yogas::CATALOG;

type T3 = [&'static str; 3];

/// What kind of authority a source is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A classical or standard text.
    Text,
    /// A later folk text.
    Folk,
    /// Temple or household custom.
    Custom,
    /// The app's own method.
    Kshana,
}

impl Kind {
    /// `"text"`, `"folk"`, `"custom"` or `"kshana"`.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Folk => "folk",
            Kind::Custom => "custom",
            Kind::Kshana => "kshana",
        }
    }
}

/// One source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    /// Stable key.
    pub key: &'static str,
    /// Kind.
    pub kind: Kind,
    /// The book (or the custom, or "Kshana's own method"), in en, ml, hi.
    pub book: T3,
    /// Chapter: a number ("35", "51–60") or the chapter's title.
    pub chapter: Option<T3>,
    /// Verse, only where the engine's own tables cite one.
    pub verse: Option<&'static str>,
    /// What the source says that the rule uses, in one line.
    pub note: T3,
}

const fn n(s: &'static str) -> Option<T3> {
    Some([s, s, s])
}

const BPHS: T3 = ["Brihat Parashara Hora Shastra", "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം", "बृहत् पराशर होरा शास्त्र"];
const PHALADEEPIKA: T3 = ["Phaladeepika (Mantreswara)", "ഫലദീപിക (മന്ത്രേശ്വരൻ)", "फलदीपिका (मंत्रेश्वर)"];
const SARAVALI: T3 = ["Saravali (Kalyana Varma)", "സാരാവലി (കല്യാണവർമ്മൻ)", "सारावली (कल्याण वर्मा)"];
const BRIHAT_JATAKA: T3 = ["Brihat Jataka (Varahamihira)", "ബൃഹജ്ജാതകം (വരാഹമിഹിരൻ)", "बृहज्जातक (वराहमिहिर)"];
const BRIHAT_SAMHITA: T3 = ["Brihat Samhita (Varahamihira)", "ബൃഹത്സംഹിത (വരാഹമിഹിരൻ)", "बृहत्संहिता (वराहमिहिर)"];
const JATAKA_PARIJATA: T3 = ["Jataka Parijata (Vaidyanatha)", "ജാതകപാരിജാതം (വൈദ്യനാഥൻ)", "जातक पारिजात (वैद्यनाथ)"];
const UTTARA_KALAMRITA: T3 = ["Uttara Kalamrita (Kalidasa)", "ഉത്തരകാലാമൃതം (കാളിദാസൻ)", "उत्तर कालामृत (कालिदास)"];
const PRASNA_MARGA: T3 = ["Prasna Marga", "പ്രശ്നമാർഗം", "प्रश्न मार्ग"];
const MUHURTA_CHINTAMANI: T3 = ["Muhurta Chintamani (Rama Daivajna)", "മുഹൂർത്തചിന്താമണി (രാമദൈവജ്ഞൻ)", "मुहूर्त चिंतामणि (राम दैवज्ञ)"];
const KP3: T3 = [
    "K. S. Krishnamurti, KP Reader III (Predictive Stellar Astrology)",
    "കെ. എസ്. കൃഷ്ണമൂർത്തി, കെപി റീഡർ III",
    "के. एस. कृष्णमूर्ति, केपी रीडर III",
];
const KP6: T3 = [
    "K. S. Krishnamurti, KP Reader VI (Horary Astrology)",
    "കെ. എസ്. കൃഷ്ണമൂർത്തി, കെപി റീഡർ VI (പ്രശ്നം)",
    "के. एस. कृष्णमूर्ति, केपी रीडर VI (प्रश्न)",
];
const JAIMINI: T3 = ["Jaimini Upadesa Sutras", "ജൈമിനി ഉപദേശസൂത്രങ്ങൾ", "जैमिनी उपदेश सूत्र"];
const TAJIKA: T3 = ["Tajika Neelakanthi (Nilakantha, 1587)", "താജിക നീലകണ്ഠി (നീലകണ്ഠൻ, 1587)", "ताजिक नीलकंठी (नीलकंठ, 1587)"];
const MATSYA: T3 = ["Matsya Purana", "മത്സ്യപുരാണം", "मत्स्य पुराण"];
const YAJNAVALKYA: T3 = ["Yajnavalkya Smriti", "യാജ്ഞവൽക്യസ്മൃതി", "याज्ञवल्क्य स्मृति"];
const DHARMA_SINDHU: T3 = ["Dharma Sindhu (Kashinatha, 1790)", "ധർമസിന്ധു (കാശിനാഥൻ, 1790)", "धर्मसिंधु (काशीनाथ, 1790)"];
const NADI: T3 = [
    "The Bhrigu Nadi method, as R. G. Rao and C. S. Patel teach it",
    "ഭൃഗു നാഡി രീതി, ആർ. ജി. റാവുവും സി. എസ്. പട്ടേലും പഠിപ്പിക്കുന്നതുപോലെ",
    "भृगु नाड़ी पद्धति, जैसे आर. जी. राव और सी. एस. पटेल सिखाते हैं",
];
const RAMAN: T3 = ["B. V. Raman, Three Hundred Important Combinations", "ബി. വി. രാമൻ, ത്രീ ഹണ്ട്രഡ് ഇംപോർട്ടന്റ് കോമ്പിനേഷൻസ്", "बी. वी. रमन, थ्री हंड्रेड इम्पॉर्टेंट कॉम्बिनेशन्स"];
const LAL_KITAB: T3 = [
    "Lal Kitab (1939–1952), a later folk text",
    "ലാൽ കിതാബ് (1939–1952), പിൽക്കാലത്തെ ഒരു നാടൻ ഗ്രന്ഥം",
    "लाल किताब (1939–1952), बाद का एक लोक ग्रंथ",
];
const KSHANA: T3 = ["Kshana's own method", "ക്ഷണയുടെ സ്വന്തം രീതി", "क्षण की अपनी पद्धति"];

macro_rules! s {
    ($key:literal, $kind:ident, $book:expr, $ch:expr, $v:expr, $note:expr) => {
        Source { key: $key, kind: Kind::$kind, book: $book, chapter: $ch, verse: $v, note: $note }
    };
}

/// Every base source.
pub static SOURCES: &[Source] = &[
    // ---- Brihat Parashara Hora Shastra ----
    s!("bphs.grahas", Text, BPHS, n("3"), None, ["The natures of the planets, benefic and malefic.", "ഗ്രഹങ്ങളുടെ സ്വഭാവം, ശുഭവും പാപവും.", "ग्रहों के स्वभाव, शुभ और पाप।"]),
    s!("bphs.maitri", Text, BPHS, n("3"), Some("55–60"), ["Which planets are natural friends and enemies.", "ഏതു ഗ്രഹങ്ങൾ സ്വാഭാവിക മിത്രങ്ങളും ശത്രുക്കളുമാണ്.", "कौन से ग्रह स्वाभाविक मित्र और शत्रु हैं।"]),
    s!("bphs.vargas", Text, BPHS, n("6–7"), None, ["The divisional charts and what each one is read for.", "വിഭാഗചാർട്ടുകളും ഓരോന്നും എന്തിനു നോക്കുന്നു എന്നതും.", "विभाग कुंडलियाँ और हर एक किस बात के लिए देखी जाती है।"]),
    s!("bphs.arishta", Text, BPHS, n("9–10"), None, ["Dangers of early life and how they are cancelled.", "ബാല്യത്തിലെ അപകടസൂചനകളും അവ എങ്ങനെ നീങ്ങുന്നു എന്നതും.", "बचपन के संकट और वे कैसे टलते हैं।"]),
    s!("bphs.bhava", Text, BPHS, Some(["The chapters on the twelve houses", "പന്ത്രണ്ടു ഭാവങ്ങളുടെ അധ്യായങ്ങൾ", "बारह भावों के अध्याय"]), None, ["Each part of life follows the strength of its house and the house's lord.", "ജീവിതത്തിന്റെ ഓരോ ഭാഗവും അതിന്റെ ഭാവത്തിന്റെയും ഭാവനാഥന്റെയും ബലം പിന്തുടരുന്നു.", "जीवन का हर हिस्सा अपने भाव और उसके स्वामी के बल पर चलता है।"]),
    s!("bphs.sahaja", Text, BPHS, Some(["Sahaja bhava (brothers and sisters)", "സഹജഭാവം (സഹോദരങ്ങൾ)", "सहज भाव (भाई-बहन)"]), None, ["Brothers and sisters are read from the 3rd and 11th, with Mars as their sign.", "സഹോദരങ്ങളെ 3, 11 ഭാവങ്ങളിൽ നിന്നും ചൊവ്വയിൽ നിന്നും വായിക്കുന്നു.", "भाई-बहन तीसरे और ग्यारहवें भाव और मंगल से देखे जाते हैं।"]),
    s!("bphs.sukha", Text, BPHS, Some(["Sukha bhava (the mother, home)", "സുഖഭാവം (അമ്മ, വീട്)", "सुख भाव (माँ, घर)"]), None, ["The mother is read from the 4th, with the Moon as her sign.", "അമ്മയെ നാലാം ഭാവത്തിൽ നിന്നും ചന്ദ്രനിൽ നിന്നും വായിക്കുന്നു.", "माँ चौथे भाव और चंद्रमा से देखी जाती है।"]),
    s!("bphs.dharma", Text, BPHS, Some(["Dharma bhava (the father, fortune)", "ധർമഭാവം (അച്ഛൻ, ഭാഗ്യം)", "धर्म भाव (पिता, भाग्य)"]), None, ["The father is read from the 9th, with the Sun as his sign.", "അച്ഛനെ ഒമ്പതാം ഭാവത്തിൽ നിന്നും സൂര്യനിൽ നിന്നും വായിക്കുന്നു.", "पिता नौवें भाव और सूर्य से देखे जाते हैं।"]),
    s!("bphs.drishti", Text, BPHS, n("26"), None, ["How each planet looks on the houses (its aspects).", "ഓരോ ഗ്രഹവും ഭാവങ്ങളെ നോക്കുന്ന വിധം (ദൃഷ്ടി).", "हर ग्रह भावों को कैसे देखता है (दृष्टि)।"]),
    s!("bphs.shadbala", Text, BPHS, n("27"), None, ["The six sources of a planet's strength.", "ഒരു ഗ്രഹത്തിന്റെ ബലത്തിന്റെ ആറു ഉറവിടങ്ങൾ.", "ग्रह के बल के छह स्रोत।"]),
    s!("bphs.karaka", Text, BPHS, n("32"), None, ["What each planet stands for: the people and matters it governs.", "ഓരോ ഗ്രഹവും പ്രതിനിധീകരിക്കുന്ന ആളുകളും കാര്യങ്ങളും.", "हर ग्रह किन लोगों और बातों का कारक है।"]),
    s!("bphs.yoga_karaka", Text, BPHS, n("34"), None, ["Which planets help and which harm for each rising sign.", "ഓരോ ലഗ്നത്തിനും ഏതു ഗ്രഹങ്ങൾ സഹായിക്കുന്നു, ഏതു ദോഷം ചെയ്യുന്നു.", "हर लग्न के लिए कौन से ग्रह मदद करते हैं और कौन नुकसान।"]),
    s!("bphs.yogas", Text, BPHS, n("34–42"), None, ["The combinations of planets and what they give.", "ഗ്രഹയോഗങ്ങളും അവ നൽകുന്ന ഫലങ്ങളും.", "ग्रहों के योग और उनके फल।"]),
    s!("bphs.dashas", Text, BPHS, n("46"), None, ["The periods of life and the order they run in (Vimshottari).", "ജീവിതത്തിലെ കാലഘട്ടങ്ങളും അവയുടെ ക്രമവും (വിംശോത്തരി).", "जीवन के दौर और उनका क्रम (विंशोत्तरी)।"]),
    s!("bphs.dasha_phala", Text, BPHS, n("47"), None, ["What each period brings, by the planet that runs it.", "ഓരോ കാലഘട്ടവും നടത്തുന്ന ഗ്രഹമനുസരിച്ച് അത് കൊണ്ടുവരുന്നത്.", "हर दौर क्या लाता है, उसे चलाने वाले ग्रह के अनुसार।"]),
    s!("bphs.yogini", Text, BPHS, Some(["The Yogini period chapter", "യോഗിനി ദശയുടെ അധ്യായം", "योगिनी दशा का अध्याय"]), None, ["A second, shorter cycle of periods.", "രണ്ടാമത്തെ, ചെറിയ ഒരു ദശാചക്രം.", "दौर का दूसरा, छोटा चक्र।"]),
    s!("bphs.kalachakra", Text, BPHS, Some(["The Kalachakra period chapter", "കാലചക്ര ദശയുടെ അധ്യായം", "कालचक्र दशा का अध्याय"]), None, ["Periods counted through the signs from the birth star.", "ജന്മനക്ഷത്രത്തിൽ നിന്ന് രാശികളിലൂടെ എണ്ണുന്ന ദശകൾ.", "जन्म नक्षत्र से राशियों में गिने जाने वाले दौर।"]),
    s!("bphs.antardasha_remedies", Text, BPHS, n("51–60"), None, ["Each sub-period chapter closes with what eases its hard effects.", "ഓരോ അന്തർദശാ അധ്യായവും അതിന്റെ ദോഷം കുറയ്ക്കുന്നതെന്തെന്ന് പറഞ്ഞ് അവസാനിക്കുന്നു.", "हर अंतर्दशा का अध्याय उसके कठिन फल घटाने के उपाय से ख़त्म होता है।"]),
    s!("bphs.ashtakavarga", Text, BPHS, Some(["The Ashtakavarga chapters", "അഷ്ടകവർഗ അധ്യായങ്ങൾ", "अष्टकवर्ग के अध्याय"]), None, ["A sign's points decide how much a passing planet gives there.", "ഒരു രാശിയിലെ ബിന്ദുക്കൾ അവിടെ കടന്നുപോകുന്ന ഗ്രഹം എത്ര തരുമെന്ന് തീരുമാനിക്കുന്നു.", "राशि के बिंदु तय करते हैं कि वहाँ से गुज़रता ग्रह कितना देगा।"]),
    s!("bphs.curses", Text, BPHS, Some(["Curses of the previous birth (ch. 85 in Sharma's numbering; 83–85 in Santhanam's, by print)", "പൂർവജന്മശാപങ്ങൾ (ശർമയുടെ ക്രമത്തിൽ അധ്യായം 85; സന്താനത്തിന്റേതിൽ 83–85)", "पूर्वजन्म के शाप (शर्मा के क्रम में अध्याय 85; संतानम में 83–85)"]), None, ["The old reading of family burdens and the rites it names to ease them.", "കുടുംബഭാരങ്ങളുടെ പഴയ വായനയും അവ കുറയ്ക്കാൻ പറയുന്ന കർമങ്ങളും.", "पारिवारिक बोझ की पुरानी व्याख्या और उन्हें हल्का करने के बताए कर्म।"]),
    s!("bphs.graha_shanti", Text, BPHS, Some(["The Graha shanti chapter", "ഗ്രഹശാന്തി അധ്യായം", "ग्रह शांति का अध्याय"]), None, ["The day, gift, prayer and colour for each planet.", "ഓരോ ഗ്രഹത്തിനും ദിവസം, ദാനം, പ്രാർത്ഥന, നിറം.", "हर ग्रह का दिन, दान, प्रार्थना और रंग।"]),
    // ---- Other classical texts ----
    s!("phaladeepika.yogas", Text, PHALADEEPIKA, n("6"), None, ["Combinations of planets, good and hard.", "ഗ്രഹയോഗങ്ങൾ, നല്ലതും കഠിനവും.", "ग्रहों के योग, अच्छे और कठिन।"]),
    s!("phaladeepika.bhavat_bhavam", Text, PHALADEEPIKA, n("15"), None, ["A relative's matters are read by counting from that relative's house.", "ഒരു ബന്ധുവിന്റെ കാര്യങ്ങൾ അവരുടെ ഭാവത്തിൽ നിന്ന് എണ്ണി വായിക്കുന്നു.", "किसी रिश्तेदार की बातें उसके भाव से गिनकर देखी जाती हैं।"]),
    s!("phaladeepika.gochara", Text, PHALADEEPIKA, n("26"), None, ["What the moving planets bring, counted from the birth Moon.", "ജന്മചന്ദ്രനിൽ നിന്ന് എണ്ണി, സഞ്ചരിക്കുന്ന ഗ്രഹങ്ങൾ കൊണ്ടുവരുന്നത്.", "जन्म के चंद्रमा से गिनकर, चलते ग्रह क्या लाते हैं।"]),
    s!("phaladeepika.gems", Text, PHALADEEPIKA, None, None, ["The stone the old text gives for each planet.", "ഓരോ ഗ്രഹത്തിനും പഴയ ഗ്രന്ഥം പറയുന്ന രത്നം.", "हर ग्रह के लिए पुराने ग्रंथ का बताया रत्न।"]),
    s!("brihat_samhita.gochara", Text, BRIHAT_SAMHITA, n("104"), None, ["The good and hard places of the moving planets, and what blocks them.", "സഞ്ചരിക്കുന്ന ഗ്രഹങ്ങളുടെ നല്ലതും കഠിനവുമായ സ്ഥാനങ്ങളും അവയെ തടയുന്നതും.", "चलते ग्रहों के अच्छे और कठिन स्थान, और उन्हें रोकने वाले।"]),
    s!("brihat_jataka.natures", Text, BRIHAT_JATAKA, n("2"), None, ["The nature and look each planet gives a person.", "ഓരോ ഗ്രഹവും ഒരാൾക്ക് നൽകുന്ന സ്വഭാവവും രൂപവും.", "हर ग्रह व्यक्ति को कैसा स्वभाव और रूप देता है।"]),
    s!("brihat_jataka.marks", Text, BRIHAT_JATAKA, n("5"), None, ["Marks on the body read from the rising part of the sky at birth.", "ജനനസമയത്ത് ഉദിക്കുന്ന ഭാഗത്തു നിന്ന് ശരീരത്തിലെ അടയാളങ്ങൾ.", "जन्म के समय उदित भाग से शरीर के निशान।"]),
    s!("brihat_jataka.yogas", Text, BRIHAT_JATAKA, n("11–15"), None, ["Royal, lunar and other combinations.", "രാജയോഗങ്ങളും ചന്ദ്രയോഗങ്ങളും മറ്റു യോഗങ്ങളും.", "राज योग, चंद्र योग और दूसरे योग।"]),
    s!("saravali.lunar", Text, SARAVALI, n("13"), None, ["Combinations counted from the Moon.", "ചന്ദ്രനിൽ നിന്ന് എണ്ണുന്ന യോഗങ്ങൾ.", "चंद्रमा से गिने जाने वाले योग।"]),
    s!("jataka_parijata.bhanga", Text, JATAKA_PARIJATA, None, None, ["When a hard combination is cancelled.", "ഒരു കഠിനയോഗം റദ്ദാകുന്നത് എപ്പോൾ.", "कठिन योग कब रद्द होता है।"]),
    s!("uttara_kalamrita.yogas", Text, UTTARA_KALAMRITA, n("4"), None, ["When a combination gives its purest result.", "ഒരു യോഗം ഏറ്റവും ശുദ്ധമായ ഫലം നൽകുന്നത് എപ്പോൾ.", "योग अपना सबसे शुद्ध फल कब देता है।"]),
    s!("prasna_marga.badhaka", Text, PRASNA_MARGA, n("15"), None, ["The house of obstruction for each rising sign.", "ഓരോ ലഗ്നത്തിനും ബാധകസ്ഥാനം.", "हर लग्न के लिए बाधक भाव।"]),
    s!("prasna_marga.effort", Text, PRASNA_MARGA, None, None, ["Human effort (purushakara) shapes what the chart shows.", "മനുഷ്യപ്രയത്നം (പുരുഷകാരം) ചാർട്ട് കാണിക്കുന്നതിനെ രൂപപ്പെടുത്തുന്നു.", "मनुष्य का प्रयास (पुरुषकार) कुंडली के फल को गढ़ता है।"]),
    s!("muhurta_chintamani.tara", Text, MUHURTA_CHINTAMANI, None, None, ["The nine kinds of day counted from the birth star.", "ജന്മനക്ഷത്രത്തിൽ നിന്ന് എണ്ണുന്ന ഒമ്പതു തരം ദിവസങ്ങൾ.", "जन्म नक्षत्र से गिने जाने वाले नौ तरह के दिन।"]),
    s!("muhurta_chintamani.muhurta", Text, MUHURTA_CHINTAMANI, None, None, ["Good times for beginning things.", "കാര്യങ്ങൾ തുടങ്ങാൻ നല്ല സമയങ്ങൾ.", "काम शुरू करने के अच्छे समय।"]),
    s!("muhurta_chintamani.vivaha", Text, MUHURTA_CHINTAMANI, Some(["Vivaha prakarana (marriage)", "വിവാഹപ്രകരണം", "विवाह प्रकरण"]), None, ["How two births are matched for marriage.", "വിവാഹത്തിന് രണ്ടു ജനനങ്ങൾ പൊരുത്തപ്പെടുത്തുന്ന വിധം.", "विवाह के लिए दो जन्मों का मिलान कैसे होता है।"]),
    s!("muhurta_chintamani.tithi", Text, MUHURTA_CHINTAMANI, None, None, ["Which lunar days suit which acts.", "ഏതു തിഥി ഏതു കർമത്തിന് ചേരും.", "कौन सी तिथि किस काम के लिए ठीक है।"]),
    s!("kp.significators", Text, KP3, None, None, ["An event comes when the periods' planets signify its houses.", "ദശാനാഥന്മാർ ഒരു സംഭവത്തിന്റെ ഭാവങ്ങളെ സൂചിപ്പിക്കുമ്പോൾ അത് വരുന്നു.", "घटना तब आती है जब दौर के ग्रह उसके भावों के कारक हों।"]),
    s!("kp.horary", Text, KP6, None, None, ["Answering a question from the moment it is asked.", "ചോദ്യം ചോദിക്കുന്ന നിമിഷത്തിൽ നിന്ന് ഉത്തരം.", "प्रश्न पूछे जाने के पल से उत्तर।"]),
    s!("jaimini.chara", Text, JAIMINI, Some(["Chara dasha (periods of the signs)", "ചര ദശ (രാശികളുടെ ദശകൾ)", "चर दशा (राशियों के दौर)"]), None, ["Periods that run through the signs.", "രാശികളിലൂടെ കടന്നുപോകുന്ന ദശകൾ.", "राशियों में चलने वाले दौर।"]),
    s!("jaimini.raja", Text, JAIMINI, Some(["The raja yoga sutras", "രാജയോഗസൂത്രങ്ങൾ", "राजयोग सूत्र"]), None, ["Rise in life from the planets that stand for the self and the work.", "സ്വയത്തെയും ജോലിയെയും പ്രതിനിധീകരിക്കുന്ന ഗ്രഹങ്ങളിൽ നിന്നുള്ള ഉയർച്ച.", "स्वयं और काम के कारक ग्रहों से उन्नति।"]),
    s!("tajika.varsha", Text, TAJIKA, None, None, ["The year's chart, cast at each birthday.", "ഓരോ ജന്മദിനത്തിലും കുറിക്കുന്ന വർഷചാർട്ട്.", "हर जन्मदिन पर बनाई जाने वाली वर्ष कुंडली।"]),
    s!("nadi.transits", Text, NADI, None, None, ["Slow planets crossing the exact degrees of the birth chart bring its events.", "മന്ദഗ്രഹങ്ങൾ ജനനചാർട്ടിലെ കൃത്യ ഡിഗ്രികൾ കടക്കുമ്പോൾ സംഭവങ്ങൾ വരുന്നു.", "धीमे ग्रह जन्म कुंडली के ठीक अंशों से गुज़रते हैं तो घटनाएँ आती हैं।"]),
    s!("raman.combinations", Text, RAMAN, None, None, ["A modern collection of the classical combinations.", "പഴയ യോഗങ്ങളുടെ ഒരു ആധുനിക ശേഖരം.", "पुराने योगों का एक आधुनिक संग्रह।"]),
    s!("matsya.navagraha", Text, MATSYA, n("93"), None, ["The gifts, colours and grains of the nine planets.", "ഒമ്പതു ഗ്രഹങ്ങളുടെ ദാനങ്ങളും നിറങ്ങളും ധാന്യങ്ങളും.", "नौ ग्रहों के दान, रंग और अनाज।"]),
    s!("yajnavalkya.graha_shanti", Text, YAJNAVALKYA, Some(["Acharadhyaya", "ആചാരാധ്യായം", "आचाराध्याय"]), Some("1.295–309"), ["The gifts and offerings for each planet.", "ഓരോ ഗ്രഹത്തിനുമുള്ള ദാനങ്ങളും അർപ്പണങ്ങളും.", "हर ग्रह के दान और अर्पण।"]),
    s!("dharma_sindhu.vrata", Text, DHARMA_SINDHU, None, None, ["The rules for keeping a vow or a fast.", "ഒരു വ്രതം പാലിക്കുന്നതിന്റെ നിയമങ്ങൾ.", "व्रत रखने के नियम।"]),
    // ---- Folk text ----
    s!("lal_kitab", Folk, LAL_KITAB, None, None, ["Simple acts and gifts, kept 40 to 43 days.", "40 മുതൽ 43 ദിവസം വരെ പാലിക്കുന്ന ലളിതമായ പ്രവൃത്തികളും ദാനങ്ങളും.", "40 से 43 दिन तक किए जाने वाले सरल काम और दान।"]),
    // ---- Custom ----
    s!("custom.temple", Custom, ["Temple custom in Kerala and Tamil Nadu (the Navagraha temples)", "കേരളത്തിലെയും തമിഴ്നാട്ടിലെയും ക്ഷേത്രാചാരം (നവഗ്രഹക്ഷേത്രങ്ങൾ)", "केरल और तमिलनाडु की मंदिर परंपरा (नवग्रह मंदिर)"], None, None, ["Custom, not a text: the temple, day and offering kept for each planet.", "ആചാരമാണ്, ഗ്രന്ഥമല്ല: ഓരോ ഗ്രഹത്തിനും പാലിക്കുന്ന ക്ഷേത്രവും ദിവസവും അർപ്പണവും.", "परंपरा है, ग्रंथ नहीं: हर ग्रह के लिए माना जाने वाला मंदिर, दिन और अर्पण।"]),
    s!("custom.vrata", Custom, ["Household custom of weekday vows", "ആഴ്ചവ്രതങ്ങളുടെ ഗാർഹികാചാരം", "वार के व्रतों की घरेलू परंपरा"], None, None, ["Custom, not a single verse: 12 Sundays, 16 Mondays, 21 Tuesdays and so on.", "ആചാരമാണ്, ഒരൊറ്റ ശ്ലോകമല്ല: 12 ഞായർ, 16 തിങ്കൾ, 21 ചൊവ്വ എന്നിങ്ങനെ.", "परंपरा है, कोई एक श्लोक नहीं: 12 रविवार, 16 सोमवार, 21 मंगलवार आदि।"]),
    s!("custom.mandala", Custom, ["Temple custom of South India: the 48-day mandala", "ദക്ഷിണേന്ത്യൻ ക്ഷേത്രാചാരം: 48 ദിവസത്തെ മണ്ഡലം", "दक्षिण भारत की मंदिर परंपरा: 48 दिन का मंडल"], None, None, ["A daily vow kept for a mandala of 48 days (41 in the Sabarimala vow).", "ദിവസേനയുള്ള വ്രതം 48 ദിവസത്തെ മണ്ഡലം (ശബരിമല വ്രതത്തിൽ 41).", "रोज़ का व्रत 48 दिन के मंडल तक (सबरीमला व्रत में 41)।"]),
    s!("custom.vow21", Custom, ["Household custom: the 21-day vow", "ഗാർഹികാചാരം: 21 ദിവസത്തെ വ്രതം", "घरेलू परंपरा: 21 दिन का व्रत"], None, None, ["The common short vow of household practice; no single verse.", "വീടുകളിലെ സാധാരണ ചെറിയ വ്രതം; ഒരൊറ്റ ശ്ലോകമില്ല.", "घरों का आम छोटा व्रत; कोई एक श्लोक नहीं।"]),
    s!("custom.chants", Custom, ["The common chants and hymns for each planet", "ഓരോ ഗ്രഹത്തിനുമുള്ള സാധാരണ മന്ത്രങ്ങളും സ്തോത്രങ്ങളും", "हर ग्रह के आम मंत्र और स्तोत्र"], None, None, ["Custom: the short planet chants and the hymns families recite on its day.", "ആചാരം: ഗ്രഹത്തിന്റെ ദിവസം കുടുംബങ്ങൾ ചൊല്ലുന്ന ചെറുമന്ത്രങ്ങളും സ്തോത്രങ്ങളും.", "परंपरा: ग्रह के दिन परिवारों में पढ़े जाने वाले छोटे मंत्र और स्तोत्र।"]),
    s!("custom.panchanga", Custom, ["The almanac (panchanga) tradition", "പഞ്ചാംഗ പാരമ്പര്യം", "पंचांग परंपरा"], None, None, ["The parts of each day to begin things in and to avoid, as almanacs print them.", "പഞ്ചാംഗങ്ങൾ അച്ചടിക്കുന്നതുപോലെ, ഓരോ ദിവസത്തിലെ തുടങ്ങാനും ഒഴിവാക്കാനുമുള്ള സമയങ്ങൾ.", "पंचांग में छपे दिन के वे हिस्से जिनमें काम शुरू करें या टालें।"]),
    s!("custom.hora", Custom, ["The planetary-hour tradition of the almanac", "പഞ്ചാംഗത്തിലെ ഹോരാ പാരമ്പര്യം", "पंचांग की होरा परंपरा"], None, None, ["Each hour of the day has a ruling planet and suits its kind of work.", "ദിവസത്തിലെ ഓരോ മണിക്കൂറിനും ഒരു ഗ്രഹമുണ്ട്, അതിന്റെ തരം ജോലിക്ക് ചേരും.", "दिन के हर घंटे का एक ग्रह होता है जो अपने तरह के काम के लिए ठीक है।"]),
    s!("custom.sade_sati", Custom, ["A later popular tradition (not in the classical texts)", "പിൽക്കാലത്തെ ജനകീയ പാരമ്പര്യം (പഴയ ഗ്രന്ഥങ്ങളിലില്ല)", "बाद की लोकप्रिय परंपरा (पुराने ग्रंथों में नहीं)"], None, None, ["The seven and a half years of Saturn's passage over the birth Moon.", "ജന്മചന്ദ്രനു മേൽ ശനിയുടെ ഏഴരവർഷം.", "जन्म के चंद्रमा पर शनि के साढ़े सात साल।"]),
    s!("custom.matching", Custom, ["The Kerala and Tamil matching tradition", "കേരളത്തിലെയും തമിഴ്നാട്ടിലെയും പൊരുത്ത പാരമ്പര്യം", "केरल और तमिल मिलान परंपरा"], None, None, ["Custom, beside the text: how families weigh the parts of a match.", "ഗ്രന്ഥത്തിനൊപ്പം ആചാരം: കുടുംബങ്ങൾ പൊരുത്തത്തിന്റെ ഭാഗങ്ങൾ തൂക്കുന്ന വിധം.", "ग्रंथ के साथ परंपरा: परिवार मिलान के हिस्सों को कैसे तौलते हैं।"]),
    // ---- Kshana's own methods ----
    s!("kshana.consensus", Kshana, KSHANA, None, None, ["When independent ways of reading point to the same weeks, the window is said more firmly; the pooling and the thresholds are the app's.", "സ്വതന്ത്രമായ വായനാരീതികൾ ഒരേ ആഴ്ചകളെ ചൂണ്ടുമ്പോൾ അത് കൂടുതൽ ഉറപ്പോടെ പറയുന്നു; കൂട്ടിച്ചേർക്കലും പരിധികളും ആപ്പിന്റേതാണ്.", "जब अलग-अलग पढ़ने के तरीके एक ही हफ़्तों की ओर इशारा करें, तो बात ज़्यादा भरोसे से कही जाती है; जोड़ने का तरीका और सीमाएँ ऐप की अपनी हैं।"]),
    s!("kshana.hours", Kshana, KSHANA, None, None, ["The likely days and hours inside a window are found by reading the same rules day by day; the narrowing is the app's.", "ഒരു കാലയളവിലെ സാധ്യതയുള്ള ദിവസങ്ങളും മണിക്കൂറുകളും അതേ നിയമങ്ങൾ ദിവസംതോറും വായിച്ച് കണ്ടെത്തുന്നു; ഈ ചുരുക്കൽ ആപ്പിന്റേതാണ്.", "किसी समय के भीतर संभावित दिन और घंटे वही नियम रोज़ पढ़कर निकाले जाते हैं; यह छँटाई ऐप की है।"]),
    s!("kshana.score", Kshana, KSHANA, None, None, ["Each part of life gets one score from named classical rules; the weight given to each rule is the app's choice.", "ജീവിതത്തിന്റെ ഓരോ ഭാഗത്തിനും പേരുള്ള പഴയ നിയമങ്ങളിൽ നിന്ന് ഒരു സ്കോർ; ഓരോ നിയമത്തിനും നൽകുന്ന ഭാരം ആപ്പിന്റെ തീരുമാനമാണ്.", "जीवन के हर हिस्से को नामित पुराने नियमों से एक अंक मिलता है; हर नियम को दिया गया वज़न ऐप का चुनाव है।"]),
    s!("kshana.family", Kshana, KSHANA, None, None, ["Your chart and your family's charts are each read for the same event, and the agreement is pooled; no text does this cross-check.", "നിങ്ങളുടെയും കുടുംബത്തിന്റെയും ചാർട്ടുകൾ ഒരേ സംഭവത്തിനായി വായിച്ച് യോജിപ്പ് കൂട്ടിച്ചേർക്കുന്നു; ഒരു ഗ്രന്ഥവും ഈ ഒത്തുനോക്കൽ ചെയ്യുന്നില്ല.", "आपकी और परिवार की कुंडलियाँ एक ही घटना के लिए पढ़ी जाती हैं और सहमति जोड़ी जाती है; कोई ग्रंथ यह मिलान नहीं करता।"]),
    s!("kshana.facts", Kshana, KSHANA, None, None, ["Only the clearest claims are asked, and your answers are used to place your birth time; the thresholds are the app's.", "ഏറ്റവും വ്യക്തമായ കാര്യങ്ങൾ മാത്രം ചോദിക്കുന്നു, നിങ്ങളുടെ ഉത്തരങ്ങൾ ജനനസമയം കണ്ടെത്താൻ ഉപയോഗിക്കുന്നു; പരിധികൾ ആപ്പിന്റേതാണ്.", "सिर्फ़ सबसे साफ़ बातें पूछी जाती हैं, और आपके उत्तर जन्म समय तय करने में लगते हैं; सीमाएँ ऐप की हैं।"]),
    s!("kshana.deeds", Kshana, KSHANA, None, None, ["Each deed is chosen from the people and matters the texts give to what weighs on you; putting a deed in the place of a rite is the app's reading.", "നിങ്ങളെ ഭാരപ്പെടുത്തുന്നതിന് ഗ്രന്ഥങ്ങൾ നൽകുന്ന ആളുകളിൽ നിന്നും കാര്യങ്ങളിൽ നിന്നുമാണ് ഓരോ പ്രവൃത്തിയും തിരഞ്ഞെടുക്കുന്നത്; കർമത്തിനു പകരം പ്രവൃത്തി വയ്ക്കുന്നത് ആപ്പിന്റെ വായനയാണ്.", "हर काम उन लोगों और बातों से चुना गया है जो ग्रंथ आप पर भारी चीज़ को देते हैं; कर्मकांड की जगह काम रखना ऐप की व्याख्या है।"]),
    s!("kshana.deeds_first", Kshana, KSHANA, None, None, ["Deeds, service and simple prayer are always offered first; anything that costs money comes last and only if you choose it.", "പ്രവൃത്തികളും സേവനവും ലളിതമായ പ്രാർത്ഥനയും എപ്പോഴും ആദ്യം; പണച്ചെലവുള്ളതെല്ലാം അവസാനം, നിങ്ങൾ തിരഞ്ഞെടുത്താൽ മാത്രം.", "काम, सेवा और सरल प्रार्थना हमेशा पहले; पैसे वाला हर उपाय आख़िर में, और सिर्फ़ आपकी पसंद पर।"]),
    s!("kshana.habits", Kshana, KSHANA, None, None, ["Everyday discipline chosen to work on the habit the pressure feeds on; common good practice, not a verse.", "സമ്മർദ്ദം പിടിമുറുക്കുന്ന ശീലത്തിൽ പ്രവർത്തിക്കാൻ തിരഞ്ഞെടുത്ത ദൈനംദിന ചിട്ട; നല്ല സാധാരണ ശീലം, ശ്ലോകമല്ല.", "रोज़ का अनुशासन जो उस आदत पर काम करे जिससे दबाव पनपता है; आम अच्छी आदत, कोई श्लोक नहीं।"]),
    s!("kshana.mind", Kshana, KSHANA, None, None, ["A few quiet minutes for any faith or none; common practice of calm, not from a text.", "ഏതു വിശ്വാസത്തിനും വിശ്വാസമില്ലാത്തവർക്കും കുറച്ച് ശാന്തമായ മിനിറ്റുകൾ; ശാന്തതയുടെ സാധാരണ ശീലം, ഗ്രന്ഥത്തിൽ നിന്നല്ല.", "किसी भी धर्म के लिए या बिना धर्म के कुछ शांत मिनट; शांति की आम आदत, किसी ग्रंथ से नहीं।"]),
    s!("kshana.prayer", Kshana, KSHANA, None, None, ["Prayer in your own faith on the day the texts give, holding its people in mind; the faith-neutral form is the app's.", "ഗ്രന്ഥങ്ങൾ പറയുന്ന ദിവസം, അതിന്റെ ആളുകളെ മനസ്സിൽ കരുതി, നിങ്ങളുടെ വിശ്വാസത്തിലെ പ്രാർത്ഥന; എല്ലാ വിശ്വാസത്തിനും ചേരുന്ന രൂപം ആപ്പിന്റേതാണ്.", "ग्रंथों के बताए दिन, उसके लोगों को मन में रखकर, अपने धर्म में प्रार्थना; सब धर्मों वाला रूप ऐप का है।"]),
    s!("kshana.gem_gate", Kshana, KSHANA, None, None, ["A stone is shown only when the old texts' own logic allows it (the modern rule of B. V. Raman and KP); it is never necessary, and only with a qualified astrologer's advice.", "പഴയ ഗ്രന്ഥങ്ങളുടെ യുക്തി അനുവദിക്കുമ്പോൾ മാത്രം രത്നം കാണിക്കുന്നു (ബി. വി. രാമന്റെയും കെപിയുടെയും ആധുനിക നിയമം); അത് ഒരിക്കലും ആവശ്യമില്ല, യോഗ്യനായ ജ്യോതിഷിയുടെ ഉപദേശത്തോടെ മാത്രം.", "रत्न तभी दिखाया जाता है जब पुराने ग्रंथों का अपना तर्क अनुमति दे (बी. वी. रमन और केपी का आधुनिक नियम); यह कभी ज़रूरी नहीं, और सिर्फ़ योग्य ज्योतिषी की सलाह पर।"]),
    s!("kshana.keep_default", Kshana, KSHANA, None, None, ["Where the tradition fixes no length, eight weeks is the app's plain default.", "പാരമ്പര്യം കാലാവധി നിശ്ചയിക്കാത്തിടത്ത് എട്ട് ആഴ്ച ആപ്പിന്റെ ലളിതമായ സ്ഥിരം.", "जहाँ परंपरा अवधि तय नहीं करती, वहाँ आठ हफ़्ते ऐप की सीधी अवधि है।"]),
    s!("kshana.timing", Kshana, KSHANA, None, None, ["When to begin is solved from the real sunrise and the planet's own day and hour at your place.", "എപ്പോൾ തുടങ്ങണമെന്ന് നിങ്ങളുടെ സ്ഥലത്തെ യഥാർത്ഥ സൂര്യോദയത്തിൽ നിന്നും ഗ്രഹത്തിന്റെ ദിവസത്തിൽ നിന്നും മണിക്കൂറിൽ നിന്നും കണക്കാക്കുന്നു.", "कब शुरू करें, यह आपकी जगह के असली सूर्योदय और ग्रह के अपने दिन और घंटे से निकाला जाता है।"]),
];

/// Keys that stand for several sources.
pub static ALIASES: &[(&str, &[&str])] = &[
    // The parts of a life score (`Component::key`).
    ("life.natal", &["bphs.bhava", "bphs.karaka", "bphs.shadbala"]),
    ("life.dasha", &["bphs.dashas", "bphs.dasha_phala", "kp.significators"]),
    ("life.gochara", &["brihat_samhita.gochara", "phaladeepika.gochara", "bphs.ashtakavarga", "custom.sade_sati"]),
    ("life.bhava", &["bphs.bhava", "bphs.drishti"]),
    ("life.tara", &["muhurta_chintamani.tara"]),
    ("life.hora", &["custom.hora"]),
    ("life.kala", &["custom.panchanga", "muhurta_chintamani.tithi"]),
    ("life.score", &["kshana.score"]),
    // A life read now and over the coming months.
    ("life", &["bphs.bhava", "bphs.dashas", "bphs.dasha_phala", "kp.significators", "phaladeepika.gochara", "kshana.score"]),
    // The ways of reading of `consensus::System::key`.
    ("system.vimshottari", &["bphs.dashas"]),
    ("system.yogini", &["bphs.yogini"]),
    ("system.chara", &["jaimini.chara"]),
    ("system.kalachakra", &["bphs.kalachakra"]),
    ("system.kp", &["kp.significators"]),
    ("system.varga", &["bphs.vargas"]),
    ("system.gochara", &["phaladeepika.gochara", "brihat_samhita.gochara", "bphs.ashtakavarga"]),
    ("system.tajika", &["tajika.varsha"]),
    ("system.nadi", &["nadi.transits"]),
    // The facts of the past check (`Topic::key`).
    ("fact.birth_order", &["bphs.sahaja", "bphs.karaka", "kshana.facts"]),
    ("fact.siblings", &["bphs.sahaja", "bphs.karaka", "kshana.facts"]),
    ("fact.mother", &["bphs.sukha", "bphs.karaka", "brihat_jataka.natures", "kshana.facts"]),
    ("fact.father", &["bphs.dharma", "bphs.karaka", "brihat_jataka.natures", "kshana.facts"]),
    ("fact.mark", &["brihat_jataka.marks", "kshana.facts"]),
    ("fact.move", &["kp.significators", "bphs.dashas", "kshana.facts"]),
    ("fact.marriage", &["kp.significators", "bphs.dashas", "kshana.facts"]),
    ("fact.job", &["kp.significators", "bphs.dashas", "kshana.facts"]),
    // The family proof.
    ("family", &["bphs.bhava", "bphs.karaka", "phaladeepika.bhavat_bhavam", "bphs.dashas", "kshana.family"]),
    // The remedy tree's branches and the tradition's three kinds.
    ("remedy.deeds", &["kshana.deeds", "bphs.karaka", "prasna_marga.effort", "kshana.deeds_first"]),
    ("remedy.giving", &["matsya.navagraha", "yajnavalkya.graha_shanti", "bphs.graha_shanti"]),
    ("remedy.habits", &["kshana.habits"]),
    ("remedy.nature", &["matsya.navagraha", "custom.temple"]),
    ("remedy.mind", &["kshana.mind"]),
    ("remedy.prayer", &["kshana.prayer", "bphs.graha_shanti"]),
    ("remedy.fast", &["custom.vrata", "dharma_sindhu.vrata"]),
    ("remedy.chant", &["custom.chants", "bphs.graha_shanti"]),
    ("remedy.temple", &["custom.temple"]),
    ("remedy.gem", &["phaladeepika.gems", "kshana.gem_gate"]),
    ("remedy.sub_period", &["bphs.antardasha_remedies"]),
    ("remedy.curse", &["bphs.curses"]),
    ("remedy.badhaka", &["prasna_marga.badhaka", "custom.temple"]),
    ("remedy.match", &["muhurta_chintamani.vivaha", "custom.matching", "dharma_sindhu.vrata"]),
];

/// The base source of a key, if it is one.
pub fn source(key: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|s| s.key == key)
}

/// The base keys an alias stands for.
pub fn alias(key: &str) -> Option<&'static [&'static str]> {
    ALIASES.iter().find(|a| a.0 == key).map(|a| a.1)
}

/// A source said in the reader's language.
#[derive(Clone, Debug, PartialEq)]
pub struct Cited {
    /// Stable key (`yoga.<key>` for a yoga's own citation).
    pub key: String,
    /// `"text"`, `"folk"`, `"custom"` or `"kshana"`.
    pub kind: &'static str,
    /// The book, the custom, or "Kshana's own method".
    pub book: String,
    /// "ch. 35", "chs. 51–60", or the chapter's title.
    pub chapter: Option<String>,
    /// "v. 55–60", only where the engine's tables cite a verse.
    pub verse: Option<String>,
    /// What the source says that the rule uses.
    pub note: String,
    /// The whole citation in one line: "Brihat Parashara Hora Shastra, ch. 46" /
    /// "Temple custom in Kerala and Tamil Nadu (custom, not a text)" / "Kshana's own
    /// method: …".
    pub line: String,
}

fn chapter_words(lang: Lang, ch: &str) -> String {
    if ch.starts_with(|c: char| c.is_ascii_digit()) {
        let many = ch.contains('–') || ch.contains(',');
        match lang {
            Lang::En => format!("{} {ch}", if many { "chs." } else { "ch." }),
            Lang::Ml => format!("അധ്യായം {ch}"),
            Lang::Hi => format!("अध्याय {ch}"),
        }
    } else {
        ch.to_string()
    }
}

fn verse_words(lang: Lang, v: &str) -> String {
    match lang {
        Lang::En => format!("v. {v}"),
        Lang::Ml => format!("ശ്ലോകം {v}"),
        Lang::Hi => format!("श्लोक {v}"),
    }
}

/// One base source in the reader's language.
pub fn said(s: &Source, lang: Lang) -> Cited {
    let book = lang.pick(s.book).to_string();
    let chapter = s.chapter.map(|c| chapter_words(lang, lang.pick(c)));
    let verse = s.verse.map(|v| verse_words(lang, v));
    let note = lang.pick(s.note).to_string();
    let line = match s.kind {
        Kind::Kshana => format!("{book}: {note}"),
        Kind::Custom => {
            let tag = lang.pick(["custom, not a text", "ആചാരം, ഗ്രന്ഥമല്ല", "परंपरा, ग्रंथ नहीं"]);
            format!("{book} ({tag})")
        }
        Kind::Text | Kind::Folk => {
            let mut l = book.clone();
            if let Some(c) = &chapter {
                l.push_str(", ");
                l.push_str(c);
            }
            if let Some(v) = &verse {
                l.push_str(", ");
                l.push_str(v);
            }
            l
        }
    };
    Cited { key: s.key.to_string(), kind: s.kind.key(), book, chapter, verse, note, line }
}

/// A yoga's own citation from the catalogue (`yoga.<key>`).
fn yoga_cited(key: &str, lang: Lang) -> Option<Cited> {
    let d = CATALOG.iter().find(|d| d.key == key)?;
    // The catalogue marks the combinations the classical texts do not hold.
    let later = d.source.starts_with("Later texts");
    let note = lang
        .pick(if d.good {
            ["Where the combination is named and what it gives.", "ഈ യോഗം പറയുന്നിടവും അത് നൽകുന്നതും.", "यह योग कहाँ बताया गया है और क्या देता है।"]
        } else {
            ["Where the combination is named; the care beside it is the app's.", "ഈ യോഗം പറയുന്നിടം; ഒപ്പമുള്ള കരുതൽ ആപ്പിന്റേതാണ്.", "यह योग कहाँ बताया गया है; साथ की देखभाल ऐप की है।"]
        })
        .to_string();
    Some(Cited {
        key: format!("yoga.{key}"),
        kind: if later { "custom" } else { "text" },
        book: d.source_in(lang).to_string(),
        chapter: None,
        verse: None,
        note,
        line: d.source_in(lang).to_string(),
    })
}

/// Every source behind `keys` (base keys, aliases or `yoga.<key>`), each once, in the
/// order first met. Unknown keys are skipped.
pub fn cite<S: AsRef<str>>(keys: &[S], lang: Lang) -> Vec<Cited> {
    let mut out: Vec<Cited> = Vec::new();
    let push = |c: Cited, out: &mut Vec<Cited>| {
        if !out.iter().any(|x| x.key == c.key) {
            out.push(c);
        }
    };
    for k in keys {
        let k = k.as_ref();
        if let Some(y) = k.strip_prefix("yoga.") {
            if let Some(c) = yoga_cited(y, lang) {
                push(c, &mut out);
            }
        } else if let Some(s) = source(k) {
            push(said(s, lang), &mut out);
        } else if let Some(list) = alias(k) {
            for b in list {
                if let Some(s) = source(b) {
                    push(said(s, lang), &mut out);
                }
            }
        }
    }
    out
}

/// True when every key resolves to at least one source.
pub fn known(key: &str) -> bool {
    source(key).is_some() || alias(key).is_some() || key.strip_prefix("yoga.").is_some_and(|y| CATALOG.iter().any(|d| d.key == y))
}

/// The source keys of a parihara remedy, from its `source` line and kind.
pub fn of_remedy(kind: crate::parihara::RemedyKind, source: &str) -> Vec<&'static str> {
    use crate::parihara::RemedyKind as K;
    let s = source;
    let mut v: Vec<&'static str> = Vec::new();
    if s.contains("Lal Kitab") {
        v.push("lal_kitab");
    }
    if s.contains("Phaladeepika (gem") || kind == K::Gem {
        v.extend(["phaladeepika.gems", "kshana.gem_gate"]);
    }
    if s.contains("ch. 85") || s.contains("Curses") {
        v.push("bphs.curses");
    }
    if s.contains("chs. 51") {
        v.push("bphs.antardasha_remedies");
    }
    if s.contains("Prasna Marga") {
        v.push("prasna_marga.badhaka");
    }
    if s.contains("Puranic") || s.contains("Matsya") || s.contains("Yajnavalkya") {
        v.extend(["matsya.navagraha", "yajnavalkya.graha_shanti"]);
    }
    if s.contains("Graha shanti") {
        v.push("bphs.graha_shanti");
    }
    if s.contains("Vrata tradition") || s.contains("Dharma Sindhu") {
        v.extend(["custom.vrata", "dharma_sindhu.vrata"]);
    }
    if s.contains("Muhurta Chintamani") || s.contains("Vivaha") {
        v.push("muhurta_chintamani.vivaha");
    }
    if s.contains("Temple") || s.contains("temple") || s.contains("Kerala") {
        v.push("custom.temple");
    }
    if s.contains("Dharma reading") || s.contains("Bhavadhyaya") || s.contains("Karakadhyaya") {
        v.extend(["kshana.deeds", "bphs.karaka"]);
    }
    if s.contains("Faith-neutral") {
        v.push("kshana.prayer");
    }
    if v.is_empty() {
        v.push(match kind {
            K::Karma | K::Seva | K::Discipline => "kshana.deeds",
            K::Daana | K::Anna | K::Jeeva | K::Prakriti => "matsya.navagraha",
            K::Vrata => "custom.vrata",
            K::Mantra => "custom.chants",
            K::Pooja | K::Vastra => "custom.temple",
            K::Gem => "phaladeepika.gems",
        });
    }
    v.dedup();
    v
}

/// The source key of a remedy's keeping length ([`crate::remedy_timing::Keep::source`]).
pub fn of_keep(source: &str) -> &'static str {
    use crate::remedy_timing as rt;
    match source {
        s if s == rt::KEEP_SOURCE_MANDALA => "custom.mandala",
        s if s == rt::KEEP_SOURCE_LAL_KITAB => "lal_kitab",
        s if s == rt::KEEP_SOURCE_21 => "custom.vow21",
        s if s == rt::KEEP_SOURCE_MONTH => "bphs.curses",
        s if s == rt::KEEP_SOURCE_WEEKDAYS => "custom.vrata",
        _ => "kshana.keep_default",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_aliases_resolve() {
        for (i, s) in SOURCES.iter().enumerate() {
            assert!(SOURCES[i + 1..].iter().all(|t| t.key != s.key), "duplicate {}", s.key);
            assert!(alias(s.key).is_none(), "{} is both a source and an alias", s.key);
            for t in [s.book, s.note] {
                assert!(t.iter().all(|x| !x.trim().is_empty()), "{} has an empty text", s.key);
            }
        }
        for (k, list) in ALIASES {
            assert!(!list.is_empty(), "{k}");
            for b in *list {
                assert!(source(b).is_some(), "alias {k} names unknown {b}");
            }
        }
        for d in CATALOG {
            assert!(known(&format!("yoga.{}", d.key)));
        }
    }

    #[test]
    fn the_app_s_own_methods_and_customs_say_so() {
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for s in SOURCES {
                let c = said(s, lang);
                assert!(!c.line.is_empty());
                match s.kind {
                    Kind::Kshana => {
                        assert!(c.line.starts_with(lang.pick(KSHANA)), "{}", c.line);
                        assert!(s.chapter.is_none() && s.verse.is_none());
                    }
                    Kind::Custom => assert!(s.verse.is_none() && s.chapter.is_none(), "{}", s.key),
                    Kind::Folk => assert!(c.line.contains("1939"), "{}", c.line),
                    Kind::Text => {}
                }
            }
        }
        let c = cite(&["custom.temple"], Lang::En);
        assert_eq!(c[0].line, "Temple custom in Kerala and Tamil Nadu (the Navagraha temples) (custom, not a text)");
    }

    #[test]
    fn citations_read_as_book_chapter_verse() {
        let c = cite(&["bphs.dashas", "bphs.maitri", "bphs.antardasha_remedies", "yajnavalkya.graha_shanti"], Lang::En);
        assert_eq!(c[0].line, "Brihat Parashara Hora Shastra, ch. 46");
        assert_eq!(c[1].line, "Brihat Parashara Hora Shastra, ch. 3, v. 55–60");
        assert_eq!(c[2].line, "Brihat Parashara Hora Shastra, chs. 51–60");
        assert_eq!(c[3].line, "Yajnavalkya Smriti, Acharadhyaya, v. 1.295–309");
        let ml = cite(&["bphs.dashas"], Lang::Ml);
        assert_eq!(ml[0].line, "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, അധ്യായം 46");
        // Verses are given only where the engine's own tables cite them.
        let with_verse: Vec<&str> = SOURCES.iter().filter(|s| s.verse.is_some()).map(|s| s.key).collect();
        assert_eq!(with_verse, vec!["bphs.maitri", "yajnavalkya.graha_shanti"]);
    }

    #[test]
    fn aliases_expand_once_in_order_and_unknown_keys_are_skipped() {
        let c = cite(&["life.dasha", "bphs.dashas", "nonsense", "yoga.gaja_kesari", "yoga.no_such"], Lang::En);
        let keys: Vec<&str> = c.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, vec!["bphs.dashas", "bphs.dasha_phala", "kp.significators", "yoga.gaja_kesari"]);
        let k = cite(&["yoga.kala_sarpa"], Lang::En);
        assert_eq!(k[0].kind, "custom", "the later combinations are not passed off as classical");
    }

    #[test]
    fn every_parihara_remedy_resolves_to_a_source() {
        use crate::parihara::{table, life_table, RemedyKind};
        for g in kshana_vedic::DashaLord::ORDER {
            for r in table(g).iter().chain(crate::daana::alternatives(g).iter()) {
                let keys = of_remedy(r.0, r.5);
                assert!(!keys.is_empty() && keys.iter().all(|k| source(k).is_some()), "{} {:?}", r.1, keys);
            }
        }
        for a in crate::areas::AREAS {
            for r in life_table(a.name) {
                assert!(!of_remedy(r.0, r.5).is_empty());
            }
        }
        assert!(of_remedy(RemedyKind::Gem, "").contains(&"kshana.gem_gate"));
    }
}
