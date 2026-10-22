//! Two charts together: the Ashtakoota match (Guna Milan), Kuja dosha, the Moon-to-Moon
//! synastry, and how the two stand at an instant and over the months ahead.
//!
//! The eight kootas are the classical tables of Muhurta Chintamani and the Kerala and
//! Tamil matching tradition, keyed to the two Moons (sign and nakshatra). Conventions
//! are stated where schools differ. Nothing here refuses a pair: a low total is read,
//! explained and dated like everything else.

use crate::ask::{answer_in, Answer, Question};
use crate::lang;
use crate::briefing::{briefing_in, Briefing};
use crate::explain::{explain, Relief};
use crate::forecast::{date_words, moment, slow_series, Band, ForecastError, Life, Natal, SIGN_NAMES};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::plain::{chapter_theme, long_test, Lang};
use crate::parashari::{dignity, natural_relation, Dignity};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, sign_lord, Ayanamsa, DashaLord};

/// One koota.
#[derive(Clone, Debug, PartialEq)]
pub struct Koota {
    /// The classical name (Varna … Nadi): a key, never shown to the person.
    pub key: &'static str,
    /// The plain name the person reads.
    pub name: &'static str,
    /// Points given.
    pub points: f64,
    /// Points possible.
    pub max: f64,
    /// Why, in words.
    pub words: String,
    /// Source.
    pub source: &'static str,
}

/// The match of two birth charts.
#[derive(Clone, Debug, PartialEq)]
pub struct Kundali {
    /// Total out of 36.
    pub total: f64,
    /// The eight kootas, in the classical order.
    pub kootas: Vec<Koota>,
    /// `Excellent match`, `Very good`, `Good enough`, `Asks for care`, `Asks for much care`.
    pub verdict: String,
    /// One or two sentences.
    pub headline: String,
    /// Kuja dosha in the first chart.
    pub kuja_a: bool,
    /// Kuja dosha in the second chart.
    pub kuja_b: bool,
    /// Kuja dosha, in words.
    pub kuja_words: String,
    /// "Pisces · Revati" for the first Moon.
    pub moon_a: String,
    /// For the second Moon.
    pub moon_b: String,
    /// The second Moon's house from the first, 1–12.
    pub moon_distance: u8,
    /// What binds, in words.
    pub strengths: Vec<String>,
    /// What rubs, in words.
    pub frictions: Vec<String>,
    /// The classical remedies for the doshas the match carries.
    pub remedies: Vec<DoshaRemedy>,
    /// The verdict for two people already together: never a yes or no, only how much
    /// care the bond asks for.
    pub together_verdict: String,
    /// The headline for two people already together.
    pub together_headline: String,
    /// Every area that asks for care: what it brings, what each can do, the remedies
    /// and the chart reason on request.
    pub care: Vec<BondCare>,
}

/// A remedy for a dosha of the match, from the Vivaha tradition.
#[derive(Clone, Debug, PartialEq)]
pub struct DoshaRemedy {
    /// `Nadi`, `Bhakoot`, `Gana`, `Kuja`, `Graha Maitri`, `Yoni`.
    pub dosha: &'static str,
    /// What to do.
    pub title: &'static str,
    /// Why, in a sentence.
    pub why: &'static str,
    /// Where it comes from.
    pub source: &'static str,
    /// True for ritual items: supportive, never required.
    pub optional: bool,
}

const VIVAHA: &str = "Vivaha prakarana of Muhurta Chintamani and the Kerala and Tamil matching tradition";
const DHARMA_SINDHU: &str = "Dharma Sindhu, Nirnaya Sindhu (dosha parihara)";
const LAL_KITAB: &str = "Lal Kitab";

const DHARMA_READING: &str = "Dharma reading of the dosha";
const FAITH_NEUTRAL: &str = "Faith-neutral prayer";
const VENUS_DAANA: &str = "Graha daana for Venus";
const BPHS_MAITRI: &str = "BPHS, Naisargika maitri; Muhurta Chintamani";

/// A source citation in the reader's language (English is the key and stays as is).
fn source_in(lang: Lang, en: &'static str) -> &'static str {
    let t = match en {
        VIVAHA => [VIVAHA, "മുഹൂർത്ത ചിന്താമണിയിലെ വിവാഹ പ്രകരണവും കേരള-തമിഴ് പൊരുത്ത പാരമ്പര്യവും", "मुहूर्त चिंतामणि का विवाह प्रकरण और केरल-तमिल मिलान परंपरा"],
        DHARMA_SINDHU => [DHARMA_SINDHU, "ധർമ്മസിന്ധു, നിർണ്ണയസിന്ധു (ദോഷപരിഹാരം)", "धर्मसिंधु, निर्णयसिंधु (दोष परिहार)"],
        LAL_KITAB => [LAL_KITAB, "ലാൽ കിതാബ്", "लाल किताब"],
        DHARMA_READING => [DHARMA_READING, "ദോഷത്തിന്റെ ധാർമ്മിക വായന", "दोष की धर्म-सम्मत व्याख्या"],
        FAITH_NEUTRAL => [FAITH_NEUTRAL, "ഏതു വിശ്വാസത്തിനും ചേരുന്ന പ്രാർത്ഥന", "हर आस्था के लिए प्रार्थना"],
        VENUS_DAANA => [VENUS_DAANA, "ശുക്രനുള്ള ഗ്രഹദാനം", "शुक्र के लिए ग्रह-दान"],
        MC => [MC, "മുഹൂർത്ത ചിന്താമണി, വിവാഹ പ്രകരണം; കേരള-തമിഴ് പൊരുത്ത പട്ടികകൾ", "मुहूर्त चिंतामणि, विवाह प्रकरण; केरल और तमिल मिलान तालिकाएँ"],
        BPHS_MAITRI => [BPHS_MAITRI, "BPHS, നൈസർഗിക മൈത്രി; മുഹൂർത്ത ചിന്താമണി", "BPHS, नैसर्गिक मैत्री; मुहूर्त चिंतामणि"],
        _ => return en,
    };
    lang.pick(t)
}

/// One remedy, its words picked for the reader.
fn rem(lang: Lang, dosha: &'static str, title: [&'static str; 3], why: [&'static str; 3], source: &'static str, optional: bool) -> DoshaRemedy {
    DoshaRemedy { dosha, title: lang.pick(title), why: lang.pick(why), source: source_in(lang, source), optional }
}

#[allow(dead_code)] // the English list, read by the remedy audit
pub(crate) fn dosha_remedies(nadi: bool, bhakoot: bool, gana: bool, kuja_unbalanced: bool, maitri_low: bool, yoni_low: bool) -> Vec<DoshaRemedy> {
    dosha_remedies_in(nadi, bhakoot, gana, kuja_unbalanced, maitri_low, yoni_low, Lang::En)
}

#[allow(clippy::too_many_arguments)]
fn dosha_remedies_in(nadi: bool, bhakoot: bool, gana: bool, kuja_unbalanced: bool, maitri_low: bool, yoni_low: bool, lang: Lang) -> Vec<DoshaRemedy> {
    let mut v = Vec::new();
    let l = lang;
    if nadi {
        v.push(rem(l, "Nadi", ["Chant the Mahamrityunjaya mantra together, 108 times a day for 40 days", "മഹാമൃത്യുഞ്ജയ മന്ത്രം ഒരുമിച്ച് ദിവസവും 108 തവണ, 40 ദിവസം ജപിക്കുക", "महामृत्युंजय मंत्र साथ मिलकर रोज़ 108 बार, 40 दिन तक जपें"], ["Protects health and the coming of children, the two things this warning touches.", "ഈ മുന്നറിയിപ്പ് തൊടുന്ന രണ്ടു കാര്യങ്ങളായ ആരോഗ്യത്തെയും മക്കളുടെ വരവിനെയും ഇത് കാക്കുന്നു.", "यह सेहत और संतान के आगमन की रक्षा करता है, जिन दो बातों को यह चेतावनी छूती है।"], DHARMA_SINDHU, true));
        v.push(rem(l, "Nadi", ["Feed and clothe the poor on a Monday, and give rice or grain", "തിങ്കളാഴ്ച പാവപ്പെട്ടവർക്ക് ഭക്ഷണവും വസ്ത്രവും കൊടുക്കുക, അരിയോ ധാന്യമോ ദാനം ചെയ്യുക", "सोमवार को ग़रीबों को भोजन और कपड़े दें, और चावल या अनाज दान करें"], ["The tradition pays this warning down in food and care given to others.", "മറ്റുള്ളവർക്ക് കൊടുക്കുന്ന ഭക്ഷണത്തിലൂടെയും കരുതലിലൂടെയുമാണ് പാരമ്പര്യം ഈ മുന്നറിയിപ്പിന് പരിഹാരം കാണുന്നത്.", "परंपरा इस चेतावनी का भार दूसरों को दिए भोजन और देखभाल से उतारती है।"], VIVAHA, false));
        v.push(rem(l, "Nadi", ["Keep the body's rhythm: shared meals, shared sleep hours, no all-nighters", "ശരീരത്തിന്റെ താളം കാക്കുക: ഒരുമിച്ചുള്ള ഭക്ഷണം, ഒരേ ഉറക്കസമയം, ഉറക്കമൊഴിക്കൽ വേണ്ട", "शरीर की लय बनाए रखें: साथ खाना, सोने का एक ही समय, रात भर जागना नहीं"], ["Two people with this warning tire the same way at the same time; a kept routine protects both.", "ഈ മുന്നറിയിപ്പുള്ള രണ്ടുപേർ ഒരേ സമയത്ത് ഒരേ രീതിയിൽ തളരും; പാലിക്കുന്ന ഒരു ദിനചര്യ രണ്ടുപേരെയും കാക്കും.", "इस चेतावनी वाले दो लोग एक ही समय, एक ही तरह थकते हैं; बनी रहने वाली दिनचर्या दोनों की रक्षा करती है।"], DHARMA_READING, false));
    }
    if bhakoot {
        v.push(rem(l, "Bhakoot", ["Lakshmi-Narayana or Gauri-Shankara pooja on a Friday or Monday", "വെള്ളിയാഴ്ചയോ തിങ്കളാഴ്ചയോ ലക്ഷ്മീനാരായണ പൂജയോ ഗൗരീശങ്കര പൂജയോ", "शुक्रवार या सोमवार को लक्ष्मी-नारायण या गौरी-शंकर की पूजा"], ["Worship of the divine couple is the tradition's way to reconcile two people whose fortunes pull apart.", "ഭാഗ്യങ്ങൾ രണ്ടു വഴിക്ക് വലിക്കുന്ന രണ്ടുപേരെ ഒന്നിപ്പിക്കാൻ പാരമ്പര്യം കാണിക്കുന്ന വഴിയാണ് ദിവ്യദമ്പതികളുടെ ആരാധന.", "दिव्य दंपती की पूजा परंपरा का वह तरीका है जिससे अलग-अलग दिशा में खिंचते भाग्य वाले दो लोग एक होते हैं।"], DHARMA_SINDHU, true));
        v.push(rem(l, "Bhakoot", ["Give water, rice and white cloth on Mondays, and keep a light Monday fast", "തിങ്കളാഴ്ചകളിൽ വെള്ളം, അരി, വെള്ളത്തുണി എന്നിവ ദാനം ചെയ്യുക, ലഘുവായ തിങ്കളാഴ്ച വ്രതം എടുക്കുക", "सोमवार को जल, चावल और सफ़ेद कपड़ा दान करें, और सोमवार का हल्का व्रत रखें"], ["Calms the mind of the more anxious partner; a calm mind rides out the hard combination.", "കൂടുതൽ ആശങ്കപ്പെടുന്ന പങ്കാളിയുടെ മനസ്സിനെ ഇത് ശാന്തമാക്കും; ശാന്തമായ മനസ്സ് പ്രയാസമുള്ള ഈ ചേർച്ചയെ കടന്നുപോകും.", "ज़्यादा चिंता करने वाले साथी का मन शांत होता है; शांत मन इस कठिन मेल को पार कर लेता है।"], VIVAHA, false));
        v.push(rem(l, "Bhakoot", ["Decide money matters only after sleeping on them, and never during a quarrel", "പണകാര്യങ്ങൾ ഒരു രാത്രി ആലോചിച്ചിട്ടു മാത്രം തീരുമാനിക്കുക, വഴക്കിനിടയിൽ ഒരിക്കലും വേണ്ട", "पैसे के फ़ैसले एक रात सोचकर ही करें, और झगड़े के बीच कभी नहीं"], ["This combination works through money, children and sudden turns; a one-night rule takes the heat out.", "ഈ ചേർച്ച പണം, മക്കൾ, പെട്ടെന്നുള്ള മാറ്റങ്ങൾ എന്നിവയിലൂടെയാണ് പ്രവർത്തിക്കുന്നത്; ഒരു രാത്രിയുടെ നിയമം ചൂട് കുറയ്ക്കും.", "यह मेल पैसे, संतान और अचानक बदलावों से होकर असर करता है; एक रात का नियम गर्मी उतार देता है।"], DHARMA_READING, false));
    }
    if gana {
        v.push(rem(l, "Gana", ["Gauri pooja, and Durga Saptashati read once a month (or heard)", "ഗൗരീപൂജ, ഒപ്പം മാസത്തിലൊരിക്കൽ ദുർഗാസപ്തശതി വായിക്കുകയോ കേൾക്കുകയോ ചെയ്യുക", "गौरी पूजा, और महीने में एक बार दुर्गा सप्तशती का पाठ (या सुनना)"], ["A gentle and a fiery temper under one roof: Gauri and Durga are the two tempers made one.", "ഒരു കൂരയ്ക്കു കീഴിൽ ഒരു ശാന്തപ്രകൃതവും ഒരു ചൂടൻ പ്രകൃതവും: ഗൗരിയും ദുർഗയും ഒന്നായിച്ചേർന്ന ആ രണ്ടു പ്രകൃതങ്ങളാണ്.", "एक छत के नीचे एक नरम और एक तेज़ स्वभाव: गौरी और दुर्गा वही दो स्वभाव हैं जो एक हो गए।"], DHARMA_SINDHU, true));
        v.push(rem(l, "Gana", ["A rule of words: no verdicts after nine at night, and the sharp word apologised for before sleep", "വാക്കിന് ഒരു നിയമം: രാത്രി ഒൻപതിനു ശേഷം തീർപ്പുകൾ വേണ്ട, പറഞ്ഞുപോയ കടുത്ത വാക്കിന് ഉറങ്ങും മുമ്പ് ക്ഷമ ചോദിക്കുക", "शब्दों का एक नियम: रात नौ बजे के बाद कोई फ़ैसला नहीं, और कड़वी बात के लिए सोने से पहले माफ़ी"], ["The clash of tempers shows as sharp words at the wrong hour; the remedy is a rule about hours and words.", "പ്രകൃതങ്ങളുടെ ഉരസൽ തെറ്റായ നേരത്തെ കടുത്ത വാക്കുകളായാണ് പുറത്തുവരുന്നത്; നേരത്തെയും വാക്കുകളെയും കുറിച്ചുള്ള ഒരു നിയമമാണ് പരിഹാരം.", "स्वभावों की टक्कर ग़लत समय पर कड़वी बातों के रूप में दिखती है; उपाय समय और शब्दों का एक नियम है।"], DHARMA_READING, false));
    }
    if kuja_unbalanced {
        v.push(rem(l, "Kuja", ["Hanuman Chalisa on Tuesdays, and red lentils given", "ചൊവ്വാഴ്ചകളിൽ ഹനുമാൻ ചാലീസ, ഒപ്പം ചുവന്ന പരിപ്പ് ദാനം", "मंगलवार को हनुमान चालीसा, और लाल मसूर का दान"], ["Cools the heat, haste and quarrels one partner brings into the home.", "ഒരു പങ്കാളി വീട്ടിലേക്ക് കൊണ്ടുവരുന്ന ചൂടും ധൃതിയും വഴക്കും ഇത് തണുപ്പിക്കും.", "एक साथी जो गर्मी, जल्दबाज़ी और झगड़े घर में लाता है, उन्हें यह ठंडा करता है।"], DHARMA_SINDHU, true));
        v.push(rem(l, "Kuja", ["Mangala Gauri vrata on the Tuesdays of Shravana, or any four Tuesdays", "ശ്രാവണമാസത്തിലെ ചൊവ്വാഴ്ചകളിലോ ഏതെങ്കിലും നാല് ചൊവ്വാഴ്ചകളിലോ മംഗളഗൗരീ വ്രതം", "श्रावण के मंगलवारों को, या किन्हीं चार मंगलवारों को, मंगला गौरी व्रत"], ["The fast the tradition prescribes for the hot-tempered partner, kept for the marriage.", "ചൂടൻ പ്രകൃതമുള്ള പങ്കാളിക്കായി പാരമ്പര്യം നിർദ്ദേശിക്കുന്ന വ്രതം, വിവാഹത്തിനുവേണ്ടി എടുക്കുന്നത്.", "गर्म स्वभाव वाले साथी के लिए परंपरा का बताया व्रत, जो विवाह के लिए रखा जाता है।"], VIVAHA, false));
        v.push(rem(l, "Kuja", ["Burn the heat with the hands: hard exercise, repairs, a kitchen garden", "ചൂട് കൈകൊണ്ട് ചെലവാക്കുക: കഠിനവ്യായാമം, അറ്റകുറ്റപ്പണി, ഒരു അടുക്കളത്തോട്ടം", "गर्मी हाथों से निकालें: कड़ी कसरत, मरम्मत, घर की बग़िया"], ["Energy spent on effort is not spent on the partner.", "അധ്വാനത്തിൽ ചെലവായ ഊർജം പങ്കാളിയുടെ മേൽ ചെലവാകില്ല.", "मेहनत में लगी ऊर्जा साथी पर नहीं निकलती।"], LAL_KITAB, false));
        v.push(rem(l, "Kuja", ["The tradition also names a Kumbha vivaha (a symbolic first wedding) before the marriage; many families keep it, many do not", "വിവാഹത്തിനു മുമ്പ് കുംഭവിവാഹം (പ്രതീകാത്മകമായ ഒരു ആദ്യവിവാഹം) എന്നൊന്നും പാരമ്പര്യം പറയുന്നുണ്ട്; പല കുടുംബങ്ങളും അത് ചെയ്യും, പലരും ചെയ്യില്ല", "परंपरा विवाह से पहले कुंभ विवाह (एक प्रतीकात्मक पहला विवाह) भी बताती है; कई परिवार इसे करते हैं, कई नहीं करते"], ["Stated for completeness; it is a rite of the family's choosing, not a requirement.", "പൂർണ്ണതയ്ക്കായി മാത്രം പറയുന്നു; കുടുംബം ഇഷ്ടപ്പെട്ടാൽ ചെയ്യാവുന്ന ചടങ്ങാണ്, നിർബന്ധമില്ല.", "पूरी बात के लिए बताया गया; यह परिवार की पसंद की रस्म है, ज़रूरी नहीं।"], VIVAHA, true));
    }
    if maitri_low {
        v.push(rem(l, "Graha Maitri", ["Each of you does one small kind act a week in the way the other likes, without being asked", "ആഴ്ചയിലൊരിക്കൽ, ചോദിക്കാതെ തന്നെ, മറ്റേയാൾക്ക് ഇഷ്ടമുള്ള രീതിയിൽ ഓരോരുത്തരും ഒരു ചെറിയ നല്ല കാര്യം ചെയ്യുക", "आप दोनों हफ़्ते में एक छोटा-सा अच्छा काम, बिना कहे, दूसरे की पसंद के ढंग से करें"], ["When minds pull apart, serving the other's way of seeing things brings them back together.", "മനസ്സുകൾ രണ്ടു വഴിക്ക് നീങ്ങുമ്പോൾ, മറ്റേയാളുടെ കാഴ്ചപ്പാടിനെ മാനിക്കുന്നത് അവയെ വീണ്ടും അടുപ്പിക്കും.", "जब मन अलग दिशाओं में खिंचें, तो दूसरे के देखने के ढंग का मान रखना उन्हें फिर पास लाता है।"], LAL_KITAB, false));
        v.push(rem(l, "Graha Maitri", ["Say the hard thing in writing first, then talk", "കടുപ്പമുള്ള കാര്യം ആദ്യം എഴുതി അറിയിക്കുക, പിന്നെ സംസാരിക്കുക", "कठिन बात पहले लिखकर कहें, फिर बात करें"], ["When the minds pull apart, the written word gives each mind its own hour to understand.", "മനസ്സുകൾ രണ്ടു വഴിക്ക് നീങ്ങുമ്പോൾ, എഴുതിയ വാക്ക് ഓരോ മനസ്സിനും മനസ്സിലാക്കാൻ അതിന്റേതായ സമയം നൽകും.", "जब मन अलग दिशाओं में खिंचें, तो लिखा हुआ शब्द हर मन को समझने का अपना समय देता है।"], DHARMA_READING, false));
    }
    if yoni_low {
        v.push(rem(l, "Yoni", ["Rati-Kama or Uma-Maheshwara worship on a Friday", "വെള്ളിയാഴ്ച രതീകാമ ആരാധനയോ ഉമാമഹേശ്വര ആരാധനയോ", "शुक्रवार को रति-काम या उमा-महेश्वर की पूजा"], ["For physical ease between the two of you; the tradition turns to the divine pair for it.", "നിങ്ങൾ തമ്മിലുള്ള ശാരീരിക അടുപ്പത്തിന്; അതിനായി പാരമ്പര്യം ദിവ്യദമ്പതികളിലേക്ക് തിരിയുന്നു.", "आप दोनों के बीच शारीरिक सहजता के लिए; परंपरा इसके लिए दिव्य जोड़े की ओर मुड़ती है।"], VIVAHA, true));
        v.push(rem(l, "Yoni", ["Unhurried time without a screen, at the same hour, twice a week", "ആഴ്ചയിൽ രണ്ടുതവണ, ഒരേ നേരത്ത്, സ്ക്രീനില്ലാതെ ധൃതിയില്ലാത്ത സമയം", "हफ़्ते में दो बार, एक ही समय पर, बिना स्क्रीन के इत्मीनान का समय"], ["Bodies and habits that must learn each other need kept time, not a rite.", "പരസ്പരം പഠിക്കേണ്ട ശരീരങ്ങൾക്കും ശീലങ്ങൾക്കും വേണ്ടത് ചടങ്ങല്ല, പാലിക്കുന്ന സമയമാണ്.", "जिन शरीरों और आदतों को एक-दूसरे को सीखना है, उन्हें रस्म नहीं, तय किया हुआ समय चाहिए।"], DHARMA_READING, false));
    }
    v
}

/// One part of the match that asks for care, read for two people who share (or will
/// share) a life: what it brings day to day, what each can do for the other, and the
/// remedies, deeds first and prayer as a choice. Nothing here says "do not marry": it
/// says where to take care and how.
#[derive(Clone, Debug, PartialEq)]
pub struct BondCare {
    /// The engine's key (`Nadi`, `Bhakoot`, `Gana`, `Kuja`, `Graha Maitri`, `Yoni`,
    /// `Tara`, `Vashya`, `Varna`, `Venus`); never shown.
    pub key: &'static str,
    /// The plain name of the area ("Health and children").
    pub area: String,
    /// What it tends to bring when two such people live together, in plain words.
    pub living: String,
    /// What each of you can do for the other. "The first person" and "The second
    /// person" are replaced by names on the screen.
    pub care: Vec<String>,
    /// The remedies for this area: deeds and giving first, prayer and rites marked
    /// optional.
    pub remedies: Vec<DoshaRemedy>,
    /// The chart reason, for the person who taps "how this helps": which part of the
    /// two charts causes it and why the remedies ease it. Names planets; shown only on
    /// request.
    pub how_it_helps: String,
}

/// A prayer of the person's own faith, offered for every area so no one is left with
/// only one tradition's rites.
fn own_faith(lang: Lang, dosha: &'static str, title: [&'static str; 3], why: [&'static str; 3]) -> DoshaRemedy {
    rem(lang, dosha, title, why, FAITH_NEUTRAL, true)
}

/// "The first person" / "the second person" in Malayalam and Hindi. The screen puts
/// names in their place (TogetherScreen's `personalise` and `byNames`), so these exact
/// forms matter: Malayalam nominative ("ഒന്നാമത്തെ ആൾ") and genitive ("ഒന്നാമത്തെ
/// ആളുടെ"), Hindi oblique ("पहले व्यक्ति").
fn person(lang: Lang, first: bool, genitive: bool) -> &'static str {
    match (lang, first, genitive) {
        (Lang::Ml, true, false) => "ഒന്നാമത്തെ ആൾ",
        (Lang::Ml, false, false) => "രണ്ടാമത്തെ ആൾ",
        (Lang::Ml, true, true) => "ഒന്നാമത്തെ ആളുടെ",
        (Lang::Ml, false, true) => "രണ്ടാമത്തെ ആളുടെ",
        (Lang::Hi, true, _) => "पहले व्यक्ति",
        (Lang::Hi, false, _) => "दूसरे व्यक्ति",
        (Lang::En, true, _) => "The first person",
        (Lang::En, false, _) => "The second person",
    }
}

const NADI_NAMES_IN: [[&str; 3]; 3] = [["Aadi", "ആദി", "आदि"], ["Madhya", "മധ്യ", "मध्य"], ["Antya", "അന്ത്യ", "अंत्य"]];
const GANA_NAMES_IN: [[&str; 3]; 3] = [["Deva", "ദേവ", "देव"], ["Manushya", "മനുഷ്യ", "मनुष्य"], ["Rakshasa", "രാക്ഷസ", "राक्षस"]];
const YONI_NAMES_IN: [[&str; 3]; 14] = [
    ["Horse", "കുതിര", "घोड़ा"], ["Elephant", "ആന", "हाथी"], ["Sheep", "ആട്", "भेड़"], ["Serpent", "പാമ്പ്", "सर्प"], ["Dog", "നായ", "कुत्ता"],
    ["Cat", "പൂച്ച", "बिल्ली"], ["Rat", "എലി", "चूहा"], ["Cow", "പശു", "गाय"], ["Buffalo", "എരുമ", "भैंस"], ["Tiger", "കടുവ", "बाघ"],
    ["Deer", "മാൻ", "हिरण"], ["Monkey", "കുരങ്ങ്", "बंदर"], ["Mongoose", "കീരി", "नेवला"], ["Lion", "സിംഹം", "सिंह"],
];

/// Every area of the match that asks for care, heaviest first.
#[allow(clippy::too_many_arguments)]
fn bond_care(
    kootas: &[Koota],
    all: &[DoshaRemedy],
    a: &Natal,
    b: &Natal,
    na: usize,
    nb: usize,
    d: u8,
    la: DashaLord,
    lb: DashaLord,
    ka: bool,
    kb: bool,
    vd: u8,
    lang: Lang,
) -> Vec<BondCare> {
    let l = lang;
    let s = |t: [&str; 3]| lang.pick(t).to_string();
    let pts = |key: &str| kootas.iter().find(|k| k.key == key).map(|k| k.points).unwrap_or(0.0);
    // Deeds and giving first; prayers and rites (optional) after them.
    let take = |key: &'static str| -> Vec<DoshaRemedy> {
        let mut v: Vec<DoshaRemedy> = all.iter().filter(|r| r.dosha == key).cloned().collect();
        v.sort_by_key(|r| r.optional);
        v
    };
    let mut out = Vec::new();

    // Nadi: the heaviest.
    if pts("Nadi") == 0.0 {
        let mut r = take("Nadi");
        r.push(own_faith(l, "Nadi", ["Pray together, in your own faith, for each other's health, once a week", "ആഴ്ചയിലൊരിക്കൽ, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, പരസ്പരം ആരോഗ്യത്തിനായി ഒരുമിച്ച് പ്രാർത്ഥിക്കുക", "हफ़्ते में एक बार, अपनी-अपनी आस्था से, एक-दूसरे की सेहत के लिए साथ प्रार्थना करें"], ["Holding the other's health in mind each week keeps the care in the habit.", "ഓരോ ആഴ്ചയും മറ്റേയാളുടെ ആരോഗ്യം മനസ്സിൽ വയ്ക്കുന്നത് കരുതലിനെ ഒരു ശീലമാക്കി നിർത്തും.", "हर हफ़्ते दूसरे की सेहत को मन में रखना देखभाल को आदत बनाए रखता है।"]));
        let (sa, sb) = (lang::star_name(l, na), lang::star_name(l, nb));
        let nadi_name = l.pick(NADI_NAMES_IN[NADI[na] as usize]);
        out.push(BondCare {
            key: "Nadi",
            area: s(["Health and children", "ആരോഗ്യവും മക്കളും", "सेहत और संतान"]),
            living: s(["You two have the same kind of body and rhythm. You tend to tire, fall ill and recover at the same times, so it is easy for both of you to be low together. The care this asks for is about health, and about pregnancy and children if you plan them. It is a call for care, not a sentence: many couples with this live long, healthy lives together.", "നിങ്ങൾ രണ്ടുപേർക്കും ഒരേ തരം ശരീരവും താളവുമാണ്. ഒരേ സമയത്ത് തളരാനും സുഖമില്ലാതാകാനും ഭേദമാകാനും ഇടയുണ്ട്, അതുകൊണ്ട് രണ്ടുപേരും ഒരുമിച്ച് തളർന്നുപോകാൻ എളുപ്പമാണ്. ആരോഗ്യത്തിലും, കുഞ്ഞുങ്ങളെ ആഗ്രഹിക്കുന്നുണ്ടെങ്കിൽ ഗർഭകാലത്തിലും മക്കളുടെ കാര്യത്തിലുമാണ് ഇത് കരുതൽ ചോദിക്കുന്നത്. ഇത് കരുതലിനുള്ള ഒരു വിളിയാണ്, ഒരു വിധിയല്ല: ഇതുള്ള പല ദമ്പതികളും ഒരുമിച്ച് നീണ്ട, ആരോഗ്യമുള്ള ജീവിതം ജീവിക്കുന്നു.", "आप दोनों का शरीर और लय एक जैसी है। आप एक ही समय थकते, बीमार पड़ते और ठीक होते हैं, इसलिए दोनों का एक साथ ढीला पड़ जाना आसान है। यह देखभाल सेहत के बारे में है, और अगर आप संतान चाहते हैं तो गर्भ और बच्चों के बारे में। यह देखभाल की पुकार है, कोई फ़ैसला नहीं: ऐसे कई जोड़े साथ में लंबा, स्वस्थ जीवन जीते हैं।"]),
            care: vec![
                s(["Look after each other's health without waiting to be asked: when one of you is unwell, the other takes over, even when tired.", "ചോദിക്കാൻ കാത്തുനിൽക്കാതെ പരസ്പരം ആരോഗ്യം നോക്കുക: ഒരാൾക്ക് സുഖമില്ലാത്തപ്പോൾ, ക്ഷീണമുണ്ടെങ്കിലും മറ്റേയാൾ കാര്യങ്ങൾ ഏറ്റെടുക്കുക.", "बिना कहे एक-दूसरे की सेहत का ध्यान रखें: जब एक की तबीयत ठीक न हो, तो दूसरा थका हो तब भी ज़िम्मेदारी संभाले।"]),
                s(["Keep shared meal and sleep times, and do not both work late in the same week.", "ഭക്ഷണവും ഉറക്കവും ഒരേ സമയത്താക്കുക, ഒരേ ആഴ്ചയിൽ രണ്ടുപേരും വൈകി ജോലി ചെയ്യരുത്.", "खाने और सोने का समय एक रखें, और एक ही हफ़्ते में दोनों देर तक काम न करें।"]),
                s(["A yearly health check for both of you, on the same day, as a small ritual of care.", "വർഷത്തിലൊരിക്കൽ രണ്ടുപേർക്കും ഒരേ ദിവസം ആരോഗ്യപരിശോധന, കരുതലിന്റെ ഒരു ചെറിയ ചടങ്ങായി.", "साल में एक बार दोनों की सेहत की जाँच, एक ही दिन, देखभाल की एक छोटी रस्म की तरह।"]),
                s(["If you plan a child, see a doctor early and keep every check-up; the old warning is about taking care, not about fate.", "കുഞ്ഞിനെ ആഗ്രഹിക്കുന്നുണ്ടെങ്കിൽ നേരത്തെ ഡോക്ടറെ കാണുക, എല്ലാ പരിശോധനകളും മുടക്കാതിരിക്കുക; ഈ പഴയ മുന്നറിയിപ്പ് കരുതലിനെക്കുറിച്ചാണ്, വിധിയെക്കുറിച്ചല്ല.", "अगर संतान चाहते हैं, तो जल्दी डॉक्टर से मिलें और हर जाँच करवाएँ; यह पुरानी चेतावनी सावधानी के बारे में है, किस्मत के बारे में नहीं।"]),
            ],
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "Nadi compares your two birth-stars ({} and {}). Both fall in the {} Nadi, which the old texts tie to the same body type (one of vata, pitta and kapha), so the same strains reach both of you. The Moon rules the body's fluids, health and the mind. Food and clothes given to the poor on a Monday, the Moon's day, is the tradition's way to strengthen the Moon: care given to others comes back as health. The Mahamrityunjaya prayer is addressed to Shiva as the giver of health and long life, the deity the tradition sets over this warning. Keeping a shared rhythm is the practical side of the same remedy.",
                    NAKSHATRA_NAMES[na], NAKSHATRA_NAMES[nb], NADI_NAMES[NADI[na] as usize]
                ),
                Lang::Ml => format!("നാഡി നിങ്ങളുടെ രണ്ടു ജന്മനക്ഷത്രങ്ങളെ ({sa}, {sb}) താരതമ്യം ചെയ്യുന്നു. രണ്ടും {nadi_name} നാഡിയിലാണ്; പഴയ ഗ്രന്ഥങ്ങൾ ഇതിനെ ഒരേ ശരീരപ്രകൃതിയുമായി (വാതം, പിത്തം, കഫം എന്നിവയിൽ ഒന്ന്) ബന്ധിപ്പിക്കുന്നു, അതിനാൽ ഒരേ ആയാസങ്ങൾ രണ്ടുപേരെയും ബാധിക്കും. ചന്ദ്രൻ ശരീരത്തിലെ ദ്രവങ്ങളെയും ആരോഗ്യത്തെയും മനസ്സിനെയും ഭരിക്കുന്നു. ചന്ദ്രന്റെ ദിവസമായ തിങ്കളാഴ്ച പാവപ്പെട്ടവർക്ക് ഭക്ഷണവും വസ്ത്രവും കൊടുക്കുന്നത് ചന്ദ്രനെ ബലപ്പെടുത്താനുള്ള പാരമ്പര്യവഴിയാണ്: മറ്റുള്ളവർക്ക് കൊടുക്കുന്ന കരുതൽ ആരോഗ്യമായി തിരിച്ചുവരും. മഹാമൃത്യുഞ്ജയ പ്രാർത്ഥന ആരോഗ്യവും നല്ല ജീവിതവും തരുന്ന ശിവനോടാണ്; ഈ മുന്നറിയിപ്പിന് മേൽ പാരമ്പര്യം വയ്ക്കുന്ന ദേവനും ശിവനാണ്. ഒരേ താളം കാക്കുന്നത് ഇതേ പരിഹാരത്തിന്റെ പ്രായോഗിക വശമാണ്."),
                Lang::Hi => format!("नाड़ी आपके दोनों जन्म-नक्षत्रों ({sa} और {sb}) की तुलना करती है। दोनों {nadi_name} नाड़ी में हैं, जिसे पुराने ग्रंथ एक ही शरीर-प्रकृति (वात, पित्त और कफ में से एक) से जोड़ते हैं, इसलिए एक-से तनाव दोनों तक पहुँचते हैं। चंद्र शरीर के द्रव, सेहत और मन का स्वामी है। चंद्र के दिन सोमवार को ग़रीबों को भोजन और कपड़े देना चंद्र को बल देने का परंपरा का तरीका है: दूसरों को दी देखभाल सेहत बनकर लौटती है। महामृत्युंजय प्रार्थना सेहत और लंबे जीवन के दाता शिव से की जाती है, जिन्हें परंपरा इस चेतावनी का देवता मानती है। एक साझा लय बनाए रखना इसी उपाय का व्यावहारिक पक्ष है।"),
            },
        });
    }

    // Bhakoot: shared fortunes.
    if pts("Bhakoot") == 0.0 {
        let other = match (14 - d as i32).rem_euclid(12) { 0 => 12, x => x };
        let (living, care): ([&str; 3], Vec<[&str; 3]>) = match d {
            2 | 12 => (
                ["Money and words are where you two rub. One of you tends to save while the other spends, and words said about money cut deeper than either of you means.", "പണവും വാക്കുകളുമാണ് നിങ്ങൾ തമ്മിൽ ഉരസുന്ന ഇടം. ഒരാൾ സൂക്ഷിച്ചുവയ്ക്കുമ്പോൾ മറ്റേയാൾ ചെലവാക്കും; പണത്തെക്കുറിച്ച് പറയുന്ന വാക്കുകൾ ഉദ്ദേശിച്ചതിനേക്കാൾ ആഴത്തിൽ മുറിപ്പെടുത്തും.", "पैसा और बातें, यहीं आप दोनों में खटपट होती है। एक बचाता है तो दूसरा ख़र्च करता है, और पैसे पर कही बातें जितना सोचा उससे ज़्यादा चुभती हैं।"],
                vec![
                    ["Keep one shared money plan, and a small private allowance each, so neither feels watched.", "ഒരു പൊതു പണപ്ലാൻ വയ്ക്കുക, ഓരോരുത്തർക്കും ചെറിയൊരു സ്വന്തം തുകയും; അപ്പോൾ ആർക്കും നിരീക്ഷിക്കപ്പെടുന്നതായി തോന്നില്ല.", "पैसे की एक साझा योजना रखें, और हर किसी के लिए थोड़ा निजी ख़र्च, ताकि किसी को निगरानी जैसा न लगे।"],
                    ["Never decide about money during a quarrel; sleep on it and talk in the morning.", "വഴക്കിനിടയിൽ പണത്തെക്കുറിച്ച് ഒരിക്കലും തീരുമാനിക്കരുത്; ഒരു രാത്രി കഴിഞ്ഞ് രാവിലെ സംസാരിക്കുക.", "झगड़े के बीच पैसे का फ़ैसला कभी न करें; रात भर सोचें और सुबह बात करें।"],
                    ["Praise in front of others, correct only in private.", "മറ്റുള്ളവരുടെ മുന്നിൽ പ്രശംസിക്കുക, തിരുത്തുന്നത് സ്വകാര്യമായി മാത്രം.", "दूसरों के सामने तारीफ़ करें, टोकें सिर्फ़ अकेले में।"],
                ],
            ),
            5 | 9 => (
                ["Children, beliefs and plans for the future are where you two rub. You may see faith, teachers or the raising of children differently.", "മക്കളും വിശ്വാസങ്ങളും ഭാവിയിലേക്കുള്ള പദ്ധതികളുമാണ് നിങ്ങൾ തമ്മിൽ ഉരസുന്ന ഇടം. വിശ്വാസം, ഗുരുക്കന്മാർ, കുട്ടികളെ വളർത്തൽ എന്നിവ നിങ്ങൾ വ്യത്യസ്തമായി കണ്ടേക്കാം.", "संतान, आस्था और भविष्य की योजनाएँ, यहीं आप दोनों में खटपट होती है। आस्था, गुरु या बच्चों की परवरिश को आप अलग-अलग नज़र से देख सकते हैं।"],
                vec![
                    ["Respect each other's faith and teachers, even where you do not share them.", "പങ്കിടാത്തിടത്തും പരസ്പരം വിശ്വാസത്തെയും ഗുരുക്കന്മാരെയും മാനിക്കുക.", "जहाँ साझा न हो, वहाँ भी एक-दूसरे की आस्था और गुरुओं का सम्मान करें।"],
                    ["Decide matters about children together, never one over the other's head.", "മക്കളുടെ കാര്യങ്ങൾ ഒരുമിച്ച് തീരുമാനിക്കുക; ഒരാൾ മറ്റേയാളെ മറികടന്ന് ഒരിക്കലും അരുത്.", "बच्चों की बातें साथ मिलकर तय करें, कभी एक दूसरे को दरकिनार करके नहीं।"],
                    ["Once a month, talk about the future you each want, and listen to the end.", "മാസത്തിലൊരിക്കൽ, ഓരോരുത്തരും ആഗ്രഹിക്കുന്ന ഭാവിയെക്കുറിച്ച് സംസാരിക്കുക, അവസാനം വരെ കേൾക്കുക.", "महीने में एक बार, हर कोई जो भविष्य चाहता है उस पर बात करें, और पूरी बात सुनें।"],
                ],
            ),
            _ => (
                ["Health, debts and sudden turns are where you two rub. When one of you has a hard time, the other is pulled into it.", "ആരോഗ്യവും കടവും പെട്ടെന്നുള്ള മാറ്റങ്ങളുമാണ് നിങ്ങൾ തമ്മിൽ ഉരസുന്ന ഇടം. ഒരാൾക്ക് പ്രയാസം വരുമ്പോൾ മറ്റേയാളും അതിലേക്ക് വലിക്കപ്പെടും.", "सेहत, कर्ज़ और अचानक बदलाव, यहीं आप दोनों में खटपट होती है। जब एक पर मुश्किल आती है, तो दूसरा भी उसमें खिंच जाता है।"],
                vec![
                    ["Stand by each other in illness and in money trouble; never keep a debt secret.", "അസുഖത്തിലും പണപ്രശ്നത്തിലും പരസ്പരം കൂടെ നിൽക്കുക; ഒരു കടവും രഹസ്യമാക്കരുത്.", "बीमारी और पैसे की तंगी में एक-दूसरे के साथ खड़े रहें; कोई कर्ज़ छिपाकर न रखें।"],
                    ["Keep some savings for sudden turns, so a shock does not become a quarrel.", "പെട്ടെന്നുള്ള മാറ്റങ്ങൾക്കായി കുറച്ച് സമ്പാദ്യം കരുതുക, ഒരു ആഘാതം വഴക്കായി മാറാതിരിക്കാൻ.", "अचानक बदलावों के लिए कुछ बचत रखें, ताकि कोई झटका झगड़ा न बन जाए।"],
                    ["When one of you is down, the other stays steady instead of sinking too.", "ഒരാൾ തളരുമ്പോൾ, മറ്റേയാൾ കൂടെ താഴാതെ ഉറച്ചുനിൽക്കുക.", "जब एक उदास हो, तो दूसरा साथ डूबने के बजाय स्थिर रहे।"],
                ],
            ),
        };
        let mut r = take("Bhakoot");
        r.push(own_faith(l, "Bhakoot", ["Pray together, in your own faith, for the household, on a Friday", "വെള്ളിയാഴ്ച, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, കുടുംബത്തിനായി ഒരുമിച്ച് പ്രാർത്ഥിക്കുക", "शुक्रवार को, अपनी आस्था से, घर-परिवार के लिए साथ प्रार्थना करें"], ["Asking together for the home's good joins two sets of hopes into one.", "വീടിന്റെ നന്മയ്ക്കായി ഒരുമിച്ച് ചോദിക്കുന്നത് രണ്ടുപേരുടെയും പ്രതീക്ഷകളെ ഒന്നാക്കും.", "घर की भलाई के लिए साथ माँगना दो लोगों की उम्मीदों को एक कर देता है।"]));
        let living = l.pick(living);
        out.push(BondCare {
            key: "Bhakoot",
            area: s(["Shared fortunes", "ഒരുമിച്ചുള്ള ഐശ്വര്യം", "साझा भाग्य"]),
            living: match l {
                Lang::En => format!("{living} This marks where to take care, and it eases with the steps below."),
                Lang::Ml => format!("{living} എവിടെയാണ് കരുതൽ വേണ്ടതെന്ന് ഇത് കാണിക്കുന്നു; താഴെയുള്ള ചുവടുകൾ കൊണ്ട് ഇത് അയയും."),
                Lang::Hi => format!("{living} यह बताता है कि कहाँ ध्यान रखना है, और नीचे के क़दमों से यह आसान होता है।"),
            },
            care: care.into_iter().map(|c| l.pick(c).to_string()).collect(),
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "Bhakoot measures how far apart the two Moon signs are: yours sit {d} and {other} signs from each other. The texts call this distance hard because each Moon falls in a house of loss, conflict or change from the other. The Moon sign carries a person's mind and fortune, so two fortunes pull in different ways. Water, rice and white cloth given on a Monday, and a light Monday fast, strengthen and calm the Moon. Worship of the divine couple (Lakshmi-Narayana or Gauri-Shankara) is the tradition's way to join two fortunes into one. The money and talk rules work on the place where the strain shows."
                ),
                Lang::Ml => format!("ഭകൂടം രണ്ടു ചന്ദ്രരാശികൾ തമ്മിലുള്ള അകലം അളക്കുന്നു: നിങ്ങളുടേത് പരസ്പരം {d}, {other} രാശി അകലെയാണ്. ഓരോ ചന്ദ്രനും മറ്റേതിൽ നിന്ന് നഷ്ടത്തിന്റെയോ കലഹത്തിന്റെയോ മാറ്റത്തിന്റെയോ ഭാവത്തിൽ വരുന്നതിനാൽ ഗ്രന്ഥങ്ങൾ ഈ അകലത്തെ പ്രയാസമുള്ളതായി കാണുന്നു. ചന്ദ്രരാശി ഒരാളുടെ മനസ്സും ഭാഗ്യവുമാണ്, അതിനാൽ രണ്ടു ഭാഗ്യങ്ങൾ രണ്ടു വഴിക്ക് വലിക്കുന്നു. തിങ്കളാഴ്ച കൊടുക്കുന്ന വെള്ളവും അരിയും വെള്ളത്തുണിയും, ലഘുവായ തിങ്കളാഴ്ച വ്രതവും ചന്ദ്രനെ ബലപ്പെടുത്തുകയും ശാന്തമാക്കുകയും ചെയ്യുന്നു. ദിവ്യദമ്പതികളുടെ (ലക്ഷ്മീനാരായണൻ അല്ലെങ്കിൽ ഗൗരീശങ്കരൻ) ആരാധന രണ്ടു ഭാഗ്യങ്ങളെ ഒന്നാക്കാനുള്ള പാരമ്പര്യവഴിയാണ്. പണത്തെയും സംസാരത്തെയും കുറിച്ചുള്ള നിയമങ്ങൾ ആയാസം കാണുന്ന ഇടത്തുതന്നെ പ്രവർത്തിക്കുന്നു."),
                Lang::Hi => format!("भकूट दोनों चंद्र राशियों की दूरी मापता है: आपकी राशियाँ एक-दूसरे से {d} और {other} राशि दूर हैं। ग्रंथ इस दूरी को कठिन मानते हैं क्योंकि हर चंद्र दूसरे से हानि, टकराव या बदलाव के भाव में पड़ता है। चंद्र राशि व्यक्ति का मन और भाग्य है, इसलिए दो भाग्य अलग-अलग दिशा में खिंचते हैं। सोमवार को जल, चावल और सफ़ेद कपड़े का दान, और सोमवार का हल्का व्रत, चंद्र को बल देते और शांत करते हैं। दिव्य दंपती (लक्ष्मी-नारायण या गौरी-शंकर) की पूजा दो भाग्यों को एक करने का परंपरा का तरीका है। पैसे और बातचीत के नियम वहीं काम करते हैं जहाँ खिंचाव दिखता है।"),
            },
        });
    }

    // Kuja: heat in one chart only.
    if ka != kb {
        let (who, other_who, n) = if ka { ("The first person", "The second person", a) } else { ("The second person", "The first person", b) };
        let mars = n.facts.planet(DashaLord::Mars);
        let from_moon = ((mars.sign as i64 - n.moon_sign()).rem_euclid(12) + 1) as u8;
        let mut r = take("Kuja");
        r.push(own_faith(l, "Kuja", ["Pray, in your own faith, for patience, on a Tuesday", "ചൊവ്വാഴ്ച, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, ക്ഷമയ്ക്കായി പ്രാർത്ഥിക്കുക", "मंगलवार को, अपनी आस्था से, धैर्य के लिए प्रार्थना करें"], ["A weekly prayer for patience names the thing to watch and asks for help with it.", "ക്ഷമയ്ക്കായുള്ള ആഴ്ചതോറുമുള്ള പ്രാർത്ഥന ശ്രദ്ധിക്കേണ്ട കാര്യത്തിന് പേരിടുകയും അതിന് സഹായം ചോദിക്കുകയും ചെയ്യുന്നു.", "धैर्य के लिए हर हफ़्ते की प्रार्थना उस बात को नाम देती है जिस पर ध्यान रखना है, और उसमें मदद माँगती है।"]));
        let (w, o) = (person(l, ka, false), person(l, !ka, false));
        let wg = person(l, ka, true);
        let h = mars.rasi_house;
        out.push(BondCare {
            key: "Kuja",
            area: s(["Heat and quarrels", "ചൂടും വഴക്കും", "गर्मी और झगड़े"]),
            living: match l {
                Lang::En => format!("{who} brings more drive, haste and temper into the home. Quarrels can flare fast and turn hurtful, and {} may feel run over. Used well, the same fire is energy that protects and builds the home.", other_who.to_lowercase()),
                Lang::Ml => format!("{w} വീട്ടിലേക്ക് കൂടുതൽ ആവേശവും ധൃതിയും ദേഷ്യവും കൊണ്ടുവരുന്നു. വഴക്കുകൾ പെട്ടെന്ന് ആളിക്കത്താം, മനസ്സിനെ മുറിപ്പെടുത്താം; {o} ഞെരുങ്ങിപ്പോകുന്നതായി തോന്നിയേക്കാം. നന്നായി ഉപയോഗിച്ചാൽ, അതേ തീ വീടിനെ കാക്കുകയും പണിതുയർത്തുകയും ചെയ്യുന്ന ഊർജമാണ്."),
                Lang::Hi => format!("{w} घर में ज़्यादा जोश, जल्दबाज़ी और गुस्सा लाते हैं। झगड़े जल्दी भड़क सकते हैं और चुभ सकते हैं, और {o} को लग सकता है कि उन्हें दबाया जा रहा है। सही इस्तेमाल हो तो यही आग वह ऊर्जा है जो घर को बचाती और बनाती है।"),
            },
            care: match l {
                Lang::En => vec![
                    format!("{who}: spend the heat on work, exercise and repairs; never make a big decision while angry."),
                    format!("{other_who}: do not answer heat with heat. Let it cool, then talk."),
                    "Both: one rule, that a quarrel is ended, with an apology from whoever started it, before you sleep.".into(),
                ],
                Lang::Ml => vec![
                    format!("{w}: ആ ചൂട് ജോലിയിലും വ്യായാമത്തിലും അറ്റകുറ്റപ്പണിയിലും ചെലവാക്കുക; ദേഷ്യത്തിലിരിക്കുമ്പോൾ ഒരിക്കലും വലിയ തീരുമാനം എടുക്കരുത്."),
                    format!("{o}: ചൂടിന് ചൂടുകൊണ്ട് മറുപടി പറയരുത്. തണുക്കാൻ അനുവദിക്കുക, പിന്നെ സംസാരിക്കുക."),
                    "രണ്ടുപേരും: ഒരു നിയമം, ഉറങ്ങും മുമ്പ് വഴക്ക് തീർക്കണം, തുടങ്ങിയയാൾ ക്ഷമ ചോദിച്ച്.".into(),
                ],
                Lang::Hi => vec![
                    format!("{w}: यह गर्मी काम, कसरत और मरम्मत में लगाएँ; गुस्से में कभी बड़ा फ़ैसला न लें।"),
                    format!("{o}: गर्मी का जवाब गर्मी से न दें। ठंडा होने दें, फिर बात करें।"),
                    "दोनों: एक नियम, कि सोने से पहले झगड़ा ख़त्म हो, और जिसने शुरू किया वह माफ़ी माँगे।".into(),
                ],
            },
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "In {}'s chart, Mars sits in house {} from the rising sign and {} from the Moon: the classic Mars (Kuja) dosha, held by only one of you. Mars is the planet of fire, courage and conflict. Hanuman is the picture of strength under discipline, and red lentils are Mars' grain: given on a Tuesday, Mars' day, they turn Mars' fire into service. Hard physical work uses up the same energy on purpose, so less of it reaches the partner. When both partners carry it, it balances; here it does not, so the remedies matter.",
                    if ka { "the first person" } else { "the second person" },
                    mars.rasi_house,
                    from_moon
                ),
                Lang::Ml => format!("{wg} ജാതകത്തിൽ ചൊവ്വ ലഗ്നത്തിൽ നിന്ന് {h}-ാം ഭാവത്തിലും ചന്ദ്രനിൽ നിന്ന് {from_moon}-ാം ഭാവത്തിലുമാണ്: നിങ്ങളിൽ ഒരാൾക്കു മാത്രമുള്ള പ്രസിദ്ധമായ ചൊവ്വാദോഷം (കുജദോഷം). ചൊവ്വ തീയുടെയും ധൈര്യത്തിന്റെയും കലഹത്തിന്റെയും ഗ്രഹമാണ്. ഹനുമാൻ അച്ചടക്കത്തിനു കീഴിലുള്ള ബലത്തിന്റെ രൂപമാണ്; ചുവന്ന പരിപ്പ് ചൊവ്വയുടെ ധാന്യമാണ്: ചൊവ്വയുടെ ദിവസമായ ചൊവ്വാഴ്ച അത് ദാനം ചെയ്യുമ്പോൾ ചൊവ്വയുടെ തീ സേവനമായി മാറുന്നു. കഠിനമായ ശാരീരികാധ്വാനം അതേ ഊർജം മനഃപൂർവം ചെലവാക്കുന്നു, അതിനാൽ പങ്കാളിയിലേക്ക് കുറച്ചേ എത്തൂ. രണ്ടു പങ്കാളികൾക്കും ഇതുണ്ടെങ്കിൽ അത് സമനിലയിലാകും; ഇവിടെ അങ്ങനെയല്ല, അതിനാൽ പരിഹാരങ്ങൾ പ്രധാനമാണ്."),
                Lang::Hi => format!("{w} की कुंडली में मंगल लग्न से {h}वें भाव में और चंद्र से {from_moon}वें भाव में है: प्रसिद्ध मंगल (कुज) दोष, जो आप में से केवल एक में है। मंगल आग, साहस और टकराव का ग्रह है। हनुमान अनुशासन में बंधी शक्ति का रूप हैं, और लाल मसूर मंगल का अनाज है: मंगल के दिन मंगलवार को इसका दान मंगल की आग को सेवा में बदल देता है। कड़ी शारीरिक मेहनत वही ऊर्जा जान-बूझकर ख़र्च कराती है, इसलिए साथी तक उसका कम हिस्सा पहुँचता है। जब दोनों साथियों में यह हो, तो संतुलन बन जाता है; यहाँ ऐसा नहीं है, इसलिए उपाय मायने रखते हैं।"),
            },
        });
    }

    // Gana: temperament.
    if pts("Gana") <= 1.0 {
        let mut r = take("Gana");
        r.push(own_faith(l, "Gana", ["Pray, each in your own faith, for a gentle tongue, every evening", "എല്ലാ വൈകുന്നേരവും, ഓരോരുത്തരും സ്വന്തം വിശ്വാസത്തിൽ, മൃദുവായ നാവിനായി പ്രാർത്ഥിക്കുക", "हर शाम, अपनी-अपनी आस्था से, मीठी ज़ुबान के लिए प्रार्थना करें"], ["A short evening prayer ends the day softly, at the very hour when sharp words come.", "കടുത്ത വാക്കുകൾ വരുന്ന അതേ നേരത്ത്, ഒരു ചെറിയ സന്ധ്യാപ്രാർത്ഥന ദിവസം മൃദുവായി അവസാനിപ്പിക്കും.", "शाम की छोटी-सी प्रार्थना दिन को नरमी से ख़त्म करती है, ठीक उसी समय जब कड़वी बातें आती हैं।"]));
        let (sa, sb) = (lang::star_name(l, na), lang::star_name(l, nb));
        let (ga, gb) = (l.pick(GANA_NAMES_IN[GANA[na] as usize]), l.pick(GANA_NAMES_IN[GANA[nb] as usize]));
        out.push(BondCare {
            key: "Gana",
            area: s(["Temperament", "പ്രകൃതം", "स्वभाव"]),
            living: s(["One of you is easy-going and the other quick to anger. Sharp words come, often late in the evening or when tired, and the gentler one may go quiet and store the hurt. Neither temper is wrong; they need a few rules to live together well.", "നിങ്ങളിൽ ഒരാൾ ശാന്തപ്രകൃതമുള്ളയാളും മറ്റേയാൾ പെട്ടെന്ന് ദേഷ്യം വരുന്നയാളുമാണ്. കടുത്ത വാക്കുകൾ വരും, പലപ്പോഴും വൈകുന്നേരം വൈകിയോ ക്ഷീണിച്ചിരിക്കുമ്പോഴോ; ശാന്തമായയാൾ മിണ്ടാതെ വേദന ഉള്ളിൽ വച്ചേക്കാം. രണ്ടു പ്രകൃതവും തെറ്റല്ല; ഒരുമിച്ച് നന്നായി ജീവിക്കാൻ കുറച്ച് നിയമങ്ങൾ മതി.", "आप में से एक सहज है और दूसरे को जल्दी गुस्सा आता है। कड़वी बातें निकलती हैं, अक्सर देर शाम या थकान में, और नरम साथी चुप होकर दर्द मन में रख सकता है। कोई भी स्वभाव ग़लत नहीं; साथ अच्छे से रहने के लिए कुछ नियम चाहिए।"]),
            care: vec![
                s(["The quicker one: pause before replying, or walk away for ten minutes rather than say the sharp thing.", "പെട്ടെന്ന് ദേഷ്യം വരുന്നയാൾ: മറുപടി പറയും മുമ്പ് ഒന്ന് നിർത്തുക, അല്ലെങ്കിൽ കടുത്ത വാക്ക് പറയുന്നതിനു പകരം പത്തു മിനിറ്റ് മാറിനിൽക്കുക.", "जल्दी भड़कने वाला: जवाब देने से पहले रुकें, या कड़वी बात कहने के बजाय दस मिनट के लिए हट जाएँ।"]),
                s(["The gentler one: say what hurt the same day, calmly, instead of storing it.", "ശാന്തമായയാൾ: വേദനിപ്പിച്ചത് ഉള്ളിൽ വയ്ക്കാതെ അന്നുതന്നെ ശാന്തമായി പറയുക.", "नरम साथी: जो बात चुभी, उसे मन में रखने के बजाय उसी दिन शांति से कहें।"]),
                s(["Both: no serious talks after nine at night, and an apology before sleep.", "രണ്ടുപേരും: രാത്രി ഒൻപതിനു ശേഷം ഗൗരവമുള്ള സംസാരം വേണ്ട, ഉറങ്ങും മുമ്പ് ക്ഷമ പറയുക.", "दोनों: रात नौ बजे के बाद कोई गंभीर बात नहीं, और सोने से पहले माफ़ी।"]),
            ],
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "Gana sorts the 27 birth-stars into three temperaments: gentle (Deva), human (Manushya) and fiery (Rakshasa). Your birth-stars, {} and {}, are {} and {}. The Moon is the mind, and its star sets the temper. Gauri (the gentle mother) and Durga (the fierce one) are two faces of one Goddess, so their worship is the tradition's picture of two tempers made one. The rule about hours and words works directly where the friction shows.",
                    NAKSHATRA_NAMES[na], NAKSHATRA_NAMES[nb], GANA_NAMES[GANA[na] as usize], GANA_NAMES[GANA[nb] as usize]
                ),
                Lang::Ml => format!("ഗണം 27 ജന്മനക്ഷത്രങ്ങളെ മൂന്നു പ്രകൃതങ്ങളായി തിരിക്കുന്നു: ശാന്തം (ദേവ), മാനുഷം (മനുഷ്യ), ഉഗ്രം (രാക്ഷസ). നിങ്ങളുടെ ജന്മനക്ഷത്രങ്ങളായ {sa}, {sb} എന്നിവ യഥാക്രമം {ga}, {gb} ഗണങ്ങളാണ്. ചന്ദ്രൻ മനസ്സാണ്, അതിന്റെ നക്ഷത്രം പ്രകൃതം നിശ്ചയിക്കുന്നു. ഗൗരിയും (ശാന്തയായ അമ്മ) ദുർഗയും (ഉഗ്രരൂപിണി) ഒരേ ദേവിയുടെ രണ്ടു മുഖങ്ങളാണ്; അതിനാൽ അവരുടെ ആരാധന രണ്ടു പ്രകൃതങ്ങൾ ഒന്നാകുന്നതിന്റെ പാരമ്പര്യചിത്രമാണ്. നേരത്തെയും വാക്കുകളെയും കുറിച്ചുള്ള നിയമം ഉരസൽ കാണുന്നിടത്ത് നേരിട്ട് പ്രവർത്തിക്കുന്നു."),
                Lang::Hi => format!("गण 27 जन्म-नक्षत्रों को तीन स्वभावों में बाँटता है: सौम्य (देव), मानवीय (मनुष्य) और उग्र (राक्षस)। आपके जन्म-नक्षत्र, {sa} और {sb}, क्रमशः {ga} और {gb} गण के हैं। चंद्र मन है, और उसका नक्षत्र स्वभाव तय करता है। गौरी (सौम्य माँ) और दुर्गा (उग्र रूप) एक ही देवी के दो रूप हैं, इसलिए उनकी पूजा दो स्वभावों के एक होने की परंपरा की तस्वीर है। समय और शब्दों का नियम सीधे वहीं काम करता है जहाँ टकराव दिखता है।"),
            },
        });
    }

    // Graha Maitri: minds.
    if pts("Graha Maitri") <= 1.0 {
        let mut r = take("Graha Maitri");
        r.push(own_faith(l, "Graha Maitri", ["Pray, in your own faith, for understanding before a hard talk", "കടുപ്പമുള്ള ഒരു സംസാരത്തിനു മുമ്പ്, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, പരസ്പരം മനസ്സിലാക്കാനായി പ്രാർത്ഥിക്കുക", "किसी कठिन बातचीत से पहले, अपनी आस्था से, समझ के लिए प्रार्थना करें"], ["A moment of quiet before talking turns a fight into a conversation.", "സംസാരിക്കുന്നതിനു മുമ്പുള്ള ഒരു നിമിഷത്തെ നിശ്ശബ്ദത വഴക്കിനെ ഒരു സംഭാഷണമാക്കി മാറ്റും.", "बात करने से पहले का एक पल का मौन झगड़े को बातचीत में बदल देता है।"]));
        let (ga, gb) = (lang::graha_name(l, la), lang::graha_name(l, lb));
        let (wa, wb) = (lang::weekday(l, lang::day_of(lb)), lang::weekday(l, lang::day_of(la)));
        out.push(BondCare {
            key: "Graha Maitri",
            area: s(["Meeting of minds", "മനസ്സുകളുടെ ചേർച്ച", "मन का मेल"]),
            living: s(["You think in different ways. What is obvious to one of you can seem strange to the other, so it is easy to feel misunderstood. Learned, the difference becomes a strength: two ways of seeing are wiser than one.", "നിങ്ങൾ വ്യത്യസ്ത രീതിയിലാണ് ചിന്തിക്കുന്നത്. ഒരാൾക്ക് വ്യക്തമായത് മറ്റേയാൾക്ക് വിചിത്രമായി തോന്നാം, അതിനാൽ തെറ്റിദ്ധരിക്കപ്പെട്ടതായി തോന്നാൻ എളുപ്പമാണ്. പഠിച്ചെടുത്താൽ ഈ വ്യത്യാസം ഒരു ബലമാകും: രണ്ടു കാഴ്ചപ്പാടുകൾ ഒന്നിനേക്കാൾ വിവേകമുള്ളതാണ്.", "आप अलग-अलग ढंग से सोचते हैं। जो एक को साफ़ लगता है, वह दूसरे को अजीब लग सकता है, इसलिए ग़लत समझे जाने का एहसास आसानी से होता है। समझ लिया जाए तो यही फ़र्क ताक़त बनता है: देखने के दो ढंग एक से ज़्यादा समझदार होते हैं।"]),
            care: vec![
                s(["Ask \"what did you mean?\" before reacting.", "പ്രതികരിക്കും മുമ്പ് \"എന്താണ് ഉദ്ദേശിച്ചത്?\" എന്ന് ചോദിക്കുക.", "प्रतिक्रिया से पहले पूछें, \"आपका मतलब क्या था?\""]),
                s(["Write the hard thing down first, then talk about it.", "കടുപ്പമുള്ള കാര്യം ആദ്യം എഴുതിവയ്ക്കുക, പിന്നെ അതിനെക്കുറിച്ച് സംസാരിക്കുക.", "कठिन बात पहले लिख लें, फिर उस पर बात करें।"]),
                s(["Once a week, do one thing the other's way, without comment.", "ആഴ്ചയിലൊരിക്കൽ, ഒന്നും പറയാതെ ഒരു കാര്യം മറ്റേയാളുടെ രീതിയിൽ ചെയ്യുക.", "हफ़्ते में एक बार, बिना कुछ कहे, एक काम दूसरे के ढंग से करें।"]),
            ],
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "Graha Maitri compares the lords of your two Moon signs: {la:?} and {lb:?}, which are not friends by nature. The lord of the Moon sign colours how each mind works, so the two minds pull apart. Doing small things the other's way, and kind acts on the weekday of the other's lord ({} and {}), please both lords and bring the minds closer.",
                    weekday_of(lb),
                    weekday_of(la)
                ),
                Lang::Ml => format!("ഗ്രഹമൈത്രി നിങ്ങളുടെ രണ്ടു ചന്ദ്രരാശികളുടെ അധിപന്മാരെ താരതമ്യം ചെയ്യുന്നു: {ga}, {gb}; ഇവർ സ്വാഭാവികമായി മിത്രങ്ങളല്ല. ചന്ദ്രരാശിയുടെ അധിപൻ ഓരോ മനസ്സും പ്രവർത്തിക്കുന്ന രീതിക്ക് നിറം നൽകുന്നു, അതിനാൽ രണ്ടു മനസ്സുകൾ രണ്ടു വഴിക്ക് നീങ്ങുന്നു. മറ്റേയാളുടെ രീതിയിൽ ചെറിയ കാര്യങ്ങൾ ചെയ്യുന്നതും, മറ്റേയാളുടെ അധിപന്റെ ആഴ്ചദിവസത്തിൽ ({wa}, {wb}) ചെയ്യുന്ന നല്ല പ്രവൃത്തികളും രണ്ട് അധിപന്മാരെയും പ്രസാദിപ്പിച്ച് മനസ്സുകളെ അടുപ്പിക്കും."),
                Lang::Hi => format!("ग्रह मैत्री आपकी दोनों चंद्र राशियों के स्वामियों की तुलना करती है: {ga} और {gb}, जो स्वभाव से मित्र नहीं हैं। चंद्र राशि का स्वामी हर मन के काम करने के ढंग को रंग देता है, इसलिए दोनों मन अलग दिशाओं में खिंचते हैं। दूसरे के ढंग से छोटे काम करना, और दूसरे के स्वामी के वार ({wa} और {wb}) पर अच्छे काम करना, दोनों स्वामियों को प्रसन्न करता है और मनों को पास लाता है।"),
            },
        });
    }

    // Yoni: physical ease.
    if pts("Yoni") <= 1.0 {
        let mut r = take("Yoni");
        r.push(own_faith(l, "Yoni", ["Pray, in your own faith, for tenderness between you, on a Friday", "വെള്ളിയാഴ്ച, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, നിങ്ങൾക്കിടയിലെ ആർദ്രതയ്ക്കായി പ്രാർത്ഥിക്കുക", "शुक्रवार को, अपनी आस्था से, आपस की कोमलता के लिए प्रार्थना करें"], ["Asking for tenderness keeps it in mind for the week.", "ആർദ്രതയ്ക്കായി ചോദിക്കുന്നത് ആഴ്ച മുഴുവൻ അത് മനസ്സിൽ നിർത്തും.", "कोमलता माँगना उसे पूरे हफ़्ते मन में रखता है।"]));
        let (ya, yb) = (l.pick(YONI_NAMES_IN[YONI[na]]), l.pick(YONI_NAMES_IN[YONI[nb]]));
        out.push(BondCare {
            key: "Yoni",
            area: s(["Physical ease", "ശാരീരിക അടുപ്പം", "शारीरिक सहजता"]),
            living: s(["Your bodies, rhythms and habits are different: one of you may want closeness when the other wants rest. This can feel like rejection when it is only difference. With patience it becomes ease.", "നിങ്ങളുടെ ശരീരവും താളവും ശീലങ്ങളും വ്യത്യസ്തമാണ്: ഒരാൾ അടുപ്പം ആഗ്രഹിക്കുമ്പോൾ മറ്റേയാൾ വിശ്രമം ആഗ്രഹിച്ചേക്കാം. ഇത് വെറും വ്യത്യാസമാണെങ്കിലും തിരസ്കാരമായി തോന്നാം. ക്ഷമയോടെ ഇത് എളുപ്പമാകും.", "आपके शरीर, लय और आदतें अलग हैं: एक को नज़दीकी चाहिए हो सकती है जब दूसरे को आराम। यह बस फ़र्क है, पर ठुकराए जाने जैसा लग सकता है। धैर्य से यह सहज हो जाता है।"]),
            care: vec![
                s(["Talk about what each of you likes, kindly and without blame.", "ഓരോരുത്തർക്കും ഇഷ്ടമുള്ളതിനെക്കുറിച്ച് സ്നേഹത്തോടെ, കുറ്റപ്പെടുത്താതെ സംസാരിക്കുക.", "एक-दूसरे को क्या अच्छा लगता है, इस पर प्यार से और बिना दोष दिए बात करें।"]),
                s(["Keep unhurried time together, without phones, twice a week.", "ആഴ്ചയിൽ രണ്ടുതവണ, ഫോണില്ലാതെ, ധൃതിയില്ലാത്ത സമയം ഒരുമിച്ച് ചെലവിടുക.", "हफ़्ते में दो बार, बिना फ़ोन के, इत्मीनान से साथ समय बिताएँ।"]),
                s(["Never use closeness as a reward or a punishment.", "അടുപ്പം ഒരിക്കലും സമ്മാനമായോ ശിക്ഷയായോ ഉപയോഗിക്കരുത്.", "नज़दीकी को कभी इनाम या सज़ा की तरह इस्तेमाल न करें।"]),
            ],
            remedies: r,
            how_it_helps: match l {
                Lang::En => format!(
                    "Yoni pairs each birth-star with an animal nature; yours are the {} and the {}, not natural companions in the classical table. Venus rules pleasure and closeness, and Friday is Venus' day: worship of the divine couple on a Friday is the tradition's way to bring ease. Kept, gentle time does on purpose what the stars do not do by themselves.",
                    YONI_NAMES[YONI[na]].to_lowercase(),
                    YONI_NAMES[YONI[nb]].to_lowercase()
                ),
                Lang::Ml => format!("യോനി ഓരോ ജന്മനക്ഷത്രത്തെയും ഒരു മൃഗപ്രകൃതവുമായി ചേർക്കുന്നു; നിങ്ങളുടേത് {ya}, {yb} എന്നിവയാണ്, പ്രാചീന പട്ടികയിൽ സ്വാഭാവിക കൂട്ടുകാരല്ല. ശുക്രൻ സുഖത്തെയും അടുപ്പത്തെയും ഭരിക്കുന്നു, വെള്ളിയാഴ്ച ശുക്രന്റെ ദിവസമാണ്: വെള്ളിയാഴ്ച ദിവ്യദമ്പതികളെ ആരാധിക്കുന്നത് അടുപ്പം എളുപ്പമാക്കാനുള്ള പാരമ്പര്യവഴിയാണ്. പാലിക്കുന്ന, മൃദുവായ സമയം നക്ഷത്രങ്ങൾ തനിയെ ചെയ്യാത്തത് മനഃപൂർവം ചെയ്യുന്നു."),
                Lang::Hi => format!("योनि हर जन्म-नक्षत्र को एक पशु-स्वभाव से जोड़ती है; आपके {ya} और {yb} हैं, जो शास्त्रीय तालिका में स्वाभाविक साथी नहीं। शुक्र सुख और नज़दीकी का स्वामी है, और शुक्रवार शुक्र का दिन है: शुक्रवार को दिव्य जोड़े की पूजा सहजता लाने का परंपरा का तरीका है। तय किया हुआ, कोमल समय वह जान-बूझकर करता है जो नक्षत्र अपने-आप नहीं करते।"),
            },
        });
    }

    // Tara: luck together.
    if pts("Tara") == 0.0 {
        out.push(BondCare {
            key: "Tara",
            area: s(["Luck together", "ഒരുമിച്ചുള്ള ഭാഗ്യം", "साथ की किस्मत"]),
            living: s(["Big decisions made together in a hurry tend to go wrong for you two. Made calmly, they go well.", "ധൃതിയിൽ ഒരുമിച്ചെടുക്കുന്ന വലിയ തീരുമാനങ്ങൾ നിങ്ങൾക്ക് പിഴയ്ക്കാറുണ്ട്. ശാന്തമായി എടുത്താൽ അവ നന്നായി പോകും.", "जल्दबाज़ी में साथ लिए बड़े फ़ैसले आप दोनों के लिए अक्सर ग़लत पड़ते हैं। शांति से लिए जाएँ तो अच्छे रहते हैं।"]),
            care: vec![
                s(["Before a big step, each of you thinks it over alone first, then you decide together.", "ഒരു വലിയ ചുവടിനു മുമ്പ്, ഓരോരുത്തരും ആദ്യം തനിയെ ആലോചിക്കുക, പിന്നെ ഒരുമിച്ച് തീരുമാനിക്കുക.", "किसी बड़े क़दम से पहले, हर कोई पहले अकेले सोचे, फिर साथ मिलकर तय करें।"]),
                s(["Begin big things on a good day; the app marks them under Future.", "വലിയ കാര്യങ്ങൾ നല്ല ദിവസം തുടങ്ങുക; ആപ്പ് അവ 'ഭാവി' എന്ന ഭാഗത്ത് അടയാളപ്പെടുത്തുന്നു.", "बड़े काम अच्छे दिन शुरू करें; ऐप उन्हें 'भविष्य' में दिखाता है।"]),
            ],
            remedies: vec![
                rem(l, "Tara", ["Give food to someone in need on the day you begin anything big together", "ഒരുമിച്ച് വലിയ എന്തെങ്കിലും തുടങ്ങുന്ന ദിവസം ആവശ്യമുള്ള ഒരാൾക്ക് ഭക്ഷണം കൊടുക്കുക", "जिस दिन आप साथ मिलकर कुछ बड़ा शुरू करें, उस दिन किसी ज़रूरतमंद को भोजन दें"], ["Starting with a gift turns a shared beginning into a blessed one.", "ഒരു ദാനത്തോടെ തുടങ്ങുന്നത് ഒരുമിച്ചുള്ള തുടക്കത്തെ അനുഗ്രഹമുള്ളതാക്കും.", "दान से शुरुआत करना साझा शुरुआत को शुभ बना देता है।"], VIVAHA, false),
                own_faith(l, "Tara", ["Pray together, in your own faith, before a big step", "ഒരു വലിയ ചുവടിനു മുമ്പ്, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, ഒരുമിച്ച് പ്രാർത്ഥിക്കുക", "कोई बड़ा क़दम उठाने से पहले, अपनी आस्था से, साथ प्रार्थना करें"], ["A shared moment of quiet slows the hurry that causes the trouble.", "ഒരുമിച്ചുള്ള ഒരു നിമിഷത്തെ നിശ്ശബ്ദത പ്രശ്നമുണ്ടാക്കുന്ന ധൃതിയെ കുറയ്ക്കും.", "साथ बिताया एक पल का मौन उस जल्दबाज़ी को धीमा करता है जिससे परेशानी आती है।"]),
            ],
            how_it_helps: s(["Tara counts from one birth-star to the other in groups of nine. Yours fall on the 3rd, 5th or 7th of the group (the stars of danger, obstacle and loss) both ways. Giving food on a day of beginnings is the classical remedy for a bad tara, and a calm, unhurried start avoids the clash the count warns of.", "താര ഒരു ജന്മനക്ഷത്രത്തിൽ നിന്ന് മറ്റേതിലേക്ക് ഒൻപതിന്റെ കൂട്ടങ്ങളായി എണ്ണുന്നു. നിങ്ങളുടേത് രണ്ടു വഴിക്കും കൂട്ടത്തിലെ 3, 5, 7 എന്നീ സ്ഥാനങ്ങളിൽ (വിപത്ത്, തടസ്സം, നഷ്ടം എന്നിവയുടെ താരകൾ) വരുന്നു. തുടക്കങ്ങളുടെ ദിവസം ഭക്ഷണം ദാനം ചെയ്യുന്നതാണ് മോശം താരയ്ക്കുള്ള പ്രാചീന പരിഹാരം; ശാന്തവും ധൃതിയില്ലാത്തതുമായ തുടക്കം ഈ എണ്ണം മുന്നറിയിപ്പ് തരുന്ന ഉരസൽ ഒഴിവാക്കും.", "तारा एक जन्म-नक्षत्र से दूसरे तक नौ-नौ के समूहों में गिनती है। आपकी गिनती दोनों ओर से समूह के तीसरे, पाँचवें या सातवें स्थान पर पड़ती है (विपत्ति, बाधा और हानि के तारे)। शुरुआत के दिन भोजन का दान बुरी तारा का शास्त्रीय उपाय है, और शांत, बिना जल्दबाज़ी की शुरुआत उस टकराव से बचाती है जिसकी यह गिनती चेतावनी देती है।"]),
        });
    }

    // Vashya: mutual pull.
    if pts("Vashya") == 0.0 {
        out.push(BondCare {
            key: "Vashya",
            area: s(["Mutual pull", "പരസ്പരമുള്ള ആകർഷണം", "आपसी खिंचाव"]),
            living: s(["Neither of you naturally gives way to the other, so closeness has to be built on purpose rather than left to happen.", "നിങ്ങളിൽ ആരും സ്വാഭാവികമായി മറ്റേയാൾക്ക് വഴങ്ങുന്നില്ല, അതിനാൽ അടുപ്പം തനിയെ ഉണ്ടാകാൻ വിടാതെ മനഃപൂർവം പണിതെടുക്കണം.", "आप में से कोई भी स्वाभाविक रूप से दूसरे के आगे नहीं झुकता, इसलिए नज़दीकी अपने-आप होने के भरोसे नहीं, जान-बूझकर बनानी होगी।"]),
            care: vec![
                s(["Take turns choosing: this week one decides, next week the other.", "മാറി മാറി തിരഞ്ഞെടുക്കുക: ഈ ആഴ്ച ഒരാൾ, അടുത്ത ആഴ്ച മറ്റേയാൾ.", "बारी-बारी से चुनें: इस हफ़्ते एक तय करे, अगले हफ़्ते दूसरा।"]),
                s(["Say thank you for small things, every day.", "ചെറിയ കാര്യങ്ങൾക്കും എല്ലാ ദിവസവും നന്ദി പറയുക.", "छोटी-छोटी बातों के लिए हर दिन धन्यवाद कहें।"]),
            ],
            remedies: vec![
                rem(l, "Vashya", ["Do one task a week together that neither of you leads, like cooking or a walk", "ആഴ്ചയിൽ ഒരു ജോലി, ആരും നയിക്കാതെ, ഒരുമിച്ച് ചെയ്യുക: പാചകമോ ഒരു നടത്തമോ പോലെ", "हफ़्ते में एक काम साथ करें जिसकी अगुवाई कोई न करे, जैसे खाना बनाना या टहलना"], ["Shared work without a leader builds the pull the match does not give by itself.", "നേതാവില്ലാത്ത കൂട്ടായ ജോലി, പൊരുത്തം തനിയെ തരാത്ത അടുപ്പം പണിതുയർത്തും.", "बिना अगुवा का साझा काम वह खिंचाव बनाता है जो मिलान अपने-आप नहीं देता।"], DHARMA_READING, false),
                own_faith(l, "Vashya", ["Pray, in your own faith, for each other by name, once a week", "ആഴ്ചയിലൊരിക്കൽ, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, പരസ്പരം പേരു ചൊല്ലി പ്രാർത്ഥിക്കുക", "हफ़्ते में एक बार, अपनी आस्था से, एक-दूसरे के लिए नाम लेकर प्रार्थना करें"], ["Praying for someone by name keeps them close.", "ഒരാളുടെ പേരു ചൊല്ലി പ്രാർത്ഥിക്കുന്നത് അവരെ അടുത്തു നിർത്തും.", "किसी के लिए नाम लेकर प्रार्थना करना उसे पास रखता है।"]),
            ],
            how_it_helps: s(["Vashya sorts the Moon signs into five kinds (four-footed, human, water, wild and insect) and asks which kind is drawn to the other. Yours are not drawn either way. The remedy is to build the pull on purpose, which is what shared work does.", "വശ്യം ചന്ദ്രരാശികളെ അഞ്ചു തരങ്ങളായി (നാൽക്കാലി, മനുഷ്യൻ, ജലജീവി, വന്യം, കീടം) തിരിച്ച് ഏതു തരം ഏതിനോട് ആകർഷിക്കപ്പെടുന്നു എന്ന് നോക്കുന്നു. നിങ്ങളുടേത് രണ്ടു വഴിക്കും ആകർഷിക്കപ്പെടുന്നില്ല. ആകർഷണം മനഃപൂർവം പണിയുകയാണ് പരിഹാരം; ഒരുമിച്ചുള്ള ജോലി അതാണ് ചെയ്യുന്നത്.", "वश्य चंद्र राशियों को पाँच प्रकारों (चौपाया, मानव, जलचर, वनचर और कीट) में बाँटता है और देखता है कि कौन किसकी ओर खिंचता है। आपकी राशियाँ किसी ओर नहीं खिंचतीं। उपाय है खिंचाव को जान-बूझकर बनाना, और साझा काम यही करता है।"]),
        });
    }

    // Venus: taste in love and comfort.
    if matches!(vd, 6 | 8) {
        out.push(BondCare {
            key: "Venus",
            area: s(["Love and comfort", "സ്നേഹവും സുഖവും", "प्रेम और सुख"]),
            living: s(["What pleases one of you in love and comfort can puzzle the other. Gifts and gestures may miss, not for lack of love but for lack of knowing.", "സ്നേഹത്തിലും സുഖത്തിലും ഒരാൾക്ക് ഇഷ്ടമുള്ളത് മറ്റേയാളെ കുഴക്കിയേക്കാം. സമ്മാനങ്ങളും സ്നേഹപ്രകടനങ്ങളും ലക്ഷ്യം തെറ്റിയേക്കാം, സ്നേഹം കുറഞ്ഞിട്ടല്ല, അറിയാത്തതുകൊണ്ട്.", "प्रेम और सुख में जो एक को अच्छा लगता है, वह दूसरे को उलझा सकता है। तोहफ़े और इशारे चूक सकते हैं, प्यार की कमी से नहीं, जानकारी की कमी से।"]),
            care: vec![
                s(["Ask what pleases the other; do not assume.", "മറ്റേയാൾക്ക് എന്താണ് ഇഷ്ടമെന്ന് ചോദിക്കുക; ഊഹിക്കരുത്.", "पूछें कि दूसरे को क्या अच्छा लगता है; अंदाज़ा न लगाएँ।"]),
                s(["Each of you names one small thing a week that made you feel loved.", "ആഴ്ചയിലൊരിക്കൽ, സ്നേഹിക്കപ്പെട്ടതായി തോന്നിച്ച ഒരു ചെറിയ കാര്യം ഓരോരുത്തരും പറയുക.", "हर हफ़्ते आप दोनों एक छोटी बात बताएँ जिससे आपको प्यार महसूस हुआ।"]),
            ],
            remedies: vec![
                rem(l, "Venus", ["On a Friday, give sweets or clothes to a woman in need", "വെള്ളിയാഴ്ച, ആവശ്യമുള്ള ഒരു സ്ത്രീക്ക് മധുരമോ വസ്ത്രമോ കൊടുക്കുക", "शुक्रवार को किसी ज़रूरतमंद स्त्री को मिठाई या कपड़े दें"], ["Giving comfort to others is the tradition's way to bring comfort home.", "മറ്റുള്ളവർക്ക് സുഖം കൊടുക്കുന്നതാണ് വീട്ടിൽ സുഖം കൊണ്ടുവരാൻ പാരമ്പര്യം പറയുന്ന വഴി.", "दूसरों को सुख देना परंपरा का घर में सुख लाने का तरीका है।"], VENUS_DAANA, false),
                own_faith(l, "Venus", ["Pray, in your own faith, in thanks for each other, on a Friday", "വെള്ളിയാഴ്ച, നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിൽ, പരസ്പരം കിട്ടിയതിന് നന്ദി പറഞ്ഞ് പ്രാർത്ഥിക്കുക", "शुक्रवार को, अपनी आस्था से, एक-दूसरे के लिए धन्यवाद की प्रार्थना करें"], ["Thankfulness is what keeps affection noticed.", "നന്ദിയാണ് സ്നേഹം ശ്രദ്ധിക്കപ്പെടാൻ സഹായിക്കുന്നത്.", "कृतज्ञता ही स्नेह को नज़र में बनाए रखती है।"]),
            ],
            how_it_helps: match l {
                Lang::En => format!("Your two Venus signs sit {vd} signs apart (a 6-8 distance). Venus rules love, comfort and taste, so your tastes differ. Friday is Venus' day; giving comfort to women on a Friday is the classical gift for Venus, and asking instead of assuming does the same work in daily life."),
                Lang::Ml => format!("നിങ്ങളുടെ രണ്ടു ശുക്രരാശികൾ {vd} രാശി അകലെയാണ് (6-8 അകലം). ശുക്രൻ സ്നേഹത്തെയും സുഖത്തെയും അഭിരുചിയെയും ഭരിക്കുന്നു, അതിനാൽ നിങ്ങളുടെ അഭിരുചികൾ വ്യത്യസ്തമാണ്. വെള്ളിയാഴ്ച ശുക്രന്റെ ദിവസമാണ്; വെള്ളിയാഴ്ച സ്ത്രീകൾക്ക് സുഖം നൽകുന്ന ദാനമാണ് ശുക്രനുള്ള പ്രാചീന ദാനം, ഊഹിക്കുന്നതിനു പകരം ചോദിക്കുന്നത് ദൈനംദിന ജീവിതത്തിൽ അതേ ജോലി ചെയ്യുന്നു."),
                Lang::Hi => format!("आपकी दोनों शुक्र राशियाँ {vd} राशि दूर हैं (6-8 की दूरी)। शुक्र प्रेम, सुख और रुचि का स्वामी है, इसलिए आपकी रुचियाँ अलग हैं। शुक्रवार शुक्र का दिन है; शुक्रवार को स्त्रियों को सुख का दान शुक्र का शास्त्रीय दान है, और अंदाज़े के बजाय पूछना रोज़ के जीवन में वही काम करता है।"),
            },
        });
    }
    out
}

fn weekday_of(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "Sunday",
        DashaLord::Moon => "Monday",
        DashaLord::Mars => "Tuesday",
        DashaLord::Mercury => "Wednesday",
        DashaLord::Jupiter => "Thursday",
        DashaLord::Venus => "Friday",
        DashaLord::Saturn | DashaLord::Rahu => "Saturday",
        DashaLord::Ketu => "Tuesday",
    }
}

const MC: &str = "Muhurta Chintamani, Vivaha prakarana; Kerala and Tamil matching tables";

/// Varna by Moon sign: 3 Brahmin (water), 2 Kshatriya (fire), 1 Vaishya (earth), 0 Shudra (air).
fn varna(sign: u8) -> u8 {
    match sign % 12 {
        3 | 7 | 11 => 3,
        0 | 4 | 8 => 2,
        1 | 5 | 9 => 1,
        _ => 0,
    }
}

#[allow(dead_code)] // the classical names, kept for the expert layer
const VARNA_NAMES: [&str; 4] = ["Shudra", "Vaishya", "Kshatriya", "Brahmin"];

/// Vashya class by Moon sign and half: 0 Chatushpada, 1 Manava, 2 Jalachara, 3 Vanachara, 4 Keeta.
fn vashya(sidereal: f64) -> usize {
    let sign = ((sidereal / 30.0).floor() as usize) % 12;
    let second_half = sidereal.rem_euclid(30.0) >= 15.0;
    match sign {
        0 | 1 => 0,
        2 | 5 | 6 | 10 => 1,
        3 | 11 => 2,
        4 => 3,
        7 => 4,
        8 => {
            if second_half {
                0
            } else {
                1
            }
        }
        _ => {
            // Capricorn: first half quadruped, second half water
            if second_half {
                2
            } else {
                0
            }
        }
    }
}

#[allow(dead_code)] // the classical names, kept for the expert layer
const VASHYA_NAMES: [&str; 5] = ["Chatushpada", "Manava", "Jalachara", "Vanachara", "Keeta"];

/// Vashya points (convention: the common symmetric table).
fn vashya_points(a: usize, b: usize) -> f64 {
    const T: [[f64; 5]; 5] = [
        [2.0, 1.0, 1.0, 0.0, 1.0],
        [1.0, 2.0, 0.5, 0.5, 0.0],
        [1.0, 0.5, 2.0, 0.0, 1.0],
        [0.0, 0.5, 0.0, 2.0, 0.0],
        [1.0, 0.0, 1.0, 0.0, 2.0],
    ];
    T[a][b]
}

/// Yoni by nakshatra: 0 Horse, 1 Elephant, 2 Sheep, 3 Serpent, 4 Dog, 5 Cat, 6 Rat, 7 Cow,
/// 8 Buffalo, 9 Tiger, 10 Deer, 11 Monkey, 12 Mongoose, 13 Lion.
const YONI: [usize; 27] = [0, 1, 2, 3, 3, 4, 5, 2, 5, 6, 6, 7, 8, 9, 8, 9, 10, 10, 4, 11, 12, 11, 13, 0, 13, 7, 1];
#[allow(dead_code)] // the classical names, kept for the expert layer
const YONI_NAMES: [&str; 14] = [
    "Horse", "Elephant", "Sheep", "Serpent", "Dog", "Cat", "Rat", "Cow", "Buffalo", "Tiger", "Deer", "Monkey",
    "Mongoose", "Lion",
];
/// The classical 14 × 14 yoni table, 4 = same, 0 = sworn enemies.
const YONI_POINTS: [[u8; 14]; 14] = [
    [4, 2, 2, 3, 2, 2, 2, 1, 0, 1, 3, 3, 2, 1],
    [2, 4, 3, 3, 2, 2, 2, 2, 3, 1, 2, 3, 2, 0],
    [2, 3, 4, 2, 1, 2, 1, 3, 3, 1, 2, 0, 3, 1],
    [3, 3, 2, 4, 2, 1, 1, 1, 1, 2, 2, 2, 0, 2],
    [2, 2, 1, 2, 4, 2, 1, 2, 2, 1, 0, 2, 1, 1],
    [2, 2, 2, 1, 2, 4, 0, 2, 2, 1, 3, 3, 2, 1],
    [2, 2, 1, 1, 1, 0, 4, 2, 2, 2, 2, 2, 1, 2],
    [1, 2, 3, 1, 2, 2, 2, 4, 3, 0, 3, 2, 2, 1],
    [0, 3, 3, 1, 2, 2, 2, 3, 4, 1, 2, 2, 2, 1],
    [1, 1, 1, 2, 1, 1, 2, 0, 1, 4, 1, 1, 2, 1],
    [3, 2, 2, 2, 0, 3, 2, 3, 2, 1, 4, 2, 2, 1],
    [3, 3, 0, 2, 2, 3, 2, 2, 2, 1, 2, 4, 3, 2],
    [2, 2, 3, 0, 1, 2, 1, 2, 2, 2, 2, 3, 4, 2],
    [1, 0, 1, 2, 1, 1, 2, 1, 1, 1, 1, 2, 2, 4],
];

/// Gana by nakshatra: 0 Deva, 1 Manushya, 2 Rakshasa.
const GANA: [u8; 27] = [0, 1, 2, 1, 0, 1, 0, 0, 2, 2, 1, 1, 0, 2, 0, 2, 0, 2, 2, 1, 1, 0, 2, 2, 1, 1, 0];
#[allow(dead_code)] // the classical names, kept for the expert layer
const GANA_NAMES: [&str; 3] = ["Deva", "Manushya", "Rakshasa"];

/// Nadi by nakshatra: 0 Aadi, 1 Madhya, 2 Antya.
const NADI: [u8; 27] = [0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2];
#[allow(dead_code)] // the classical names, kept for the expert layer
const NADI_NAMES: [&str; 3] = ["Aadi", "Madhya", "Antya"];

fn moon_words(sidereal: f64, lang: Lang) -> String {
    let n = nakshatra(sidereal);
    let sign = ((sidereal / 30.0).floor() as usize) % 12;
    match lang {
        Lang::En => format!("{} · {}", SIGN_NAMES[sign], NAKSHATRA_NAMES[n.index]),
        _ => format!("{} · {}", lang::sign_name(lang, sign), lang::star_name(lang, n.index)),
    }
}

/// Kuja dosha: Mars in 1, 2, 4, 7, 8 or 12 from the lagna or from the Moon (convention:
/// both counted, the 2nd included as in the South). Cancelled when Mars is in its own
/// or exaltation sign.
fn kuja(n: &Natal, lang: Lang) -> (bool, String) {
    let mars = n.facts.planet(DashaLord::Mars);
    let from_lagna = mars.rasi_house;
    let from_moon = ((mars.sign as i64 - n.moon_sign()).rem_euclid(12) + 1) as u8;
    let bad = [1u8, 2, 4, 7, 8, 12];
    let hit_l = bad.contains(&from_lagna);
    let hit_m = bad.contains(&from_moon);
    let d = dignity(DashaLord::Mars, mars.sidereal);
    let cancelled = matches!(d, Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted);
    if !(hit_l || hit_m) {
        return (false, lang.pick(["no sign of heat in married life", "ദാമ്പത്യത്തിൽ ചൂടിന്റെ സൂചനയില്ല", "वैवाहिक जीवन में गर्मी का कोई संकेत नहीं"]).to_string());
    }
    if cancelled {
        return (false, lang.pick(["a sign of heat in married life, cancelled by its own strength", "ദാമ്പത്യത്തിൽ ചൂടിന്റെ ഒരു സൂചന, അതിന്റെ തന്നെ ബലത്താൽ റദ്ദായത്", "वैवाहिक जीवन में गर्मी का एक संकेत, जो अपनी ही शक्ति से रद्द हो जाता है"]).to_string());
    }
    (true, lang.pick(["a sign of heat, haste and quarrels in married life", "ദാമ്പത്യത്തിൽ ചൂടിന്റെയും ധൃതിയുടെയും വഴക്കിന്റെയും ഒരു സൂചന", "वैवाहिक जीवन में गर्मी, जल्दबाज़ी और झगड़ों का एक संकेत"]).to_string())
}

/// Match two charts. `a_is_groom` orients the two one-directional kootas (Varna, and the
/// Deva–Manushya exception of Gana).
pub fn kundali(a: &Natal, b: &Natal, a_is_groom: bool) -> Kundali {
    kundali_in(a, b, a_is_groom, Lang::En)
}

/// [`kundali`] in the reader's language.
pub fn kundali_in(a: &Natal, b: &Natal, a_is_groom: bool, lang: Lang) -> Kundali {
    let l = lang;
    let ma = a.moon_sidereal;
    let mb = b.moon_sidereal;
    let sa = ((ma / 30.0).floor() as u8) % 12;
    let sb = ((mb / 30.0).floor() as u8) % 12;
    let na = nakshatra(ma).index;
    let nb = nakshatra(mb).index;
    let (g, br) = if a_is_groom { (sa, sb) } else { (sb, sa) };
    let mut kootas = Vec::with_capacity(8);

    // 1. Varna
    let (va, vb) = (varna(g), varna(br));
    let p = if va >= vb { 1.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Varna",
        name: l.pick(["Outlook on life", "ജീവിതവീക്ഷണം", "जीवन का नज़रिया"]),
        points: p,
        max: 1.0,
        words: if p > 0.0 {
            l.pick(["your ways of seeing life fit together; who leads and who supports comes naturally", "ജീവിതത്തെ കാണുന്ന നിങ്ങളുടെ രീതികൾ ചേരുന്നു; ആര് നയിക്കണം, ആര് താങ്ങാകണം എന്നത് സ്വാഭാവികമായി വരും", "ज़िंदगी देखने के आपके ढंग मेल खाते हैं; कौन आगे चले और कौन साथ दे, यह सहज ही तय हो जाता है"]).to_string()
        } else {
            l.pick(["your ways of seeing life sit in the opposite order to the tradition's; agree early who decides what", "ജീവിതത്തെ കാണുന്ന നിങ്ങളുടെ രീതികൾ പാരമ്പര്യം പറയുന്നതിന്റെ എതിർക്രമത്തിലാണ്; ആര് എന്ത് തീരുമാനിക്കുമെന്ന് നേരത്തെ ധാരണയാകുക", "ज़िंदगी देखने के आपके ढंग परंपरा के उलटे क्रम में हैं; जल्दी तय कर लें कि कौन क्या तय करेगा"]).to_string()
        },
        source: source_in(l, MC),
    });

    // 2. Vashya
    let (xa, xb) = (vashya(ma), vashya(mb));
    let p = vashya_points(xa, xb);
    kootas.push(Koota {
        key: "Vashya",
        name: l.pick(["Mutual pull", "പരസ്പരമുള്ള ആകർഷണം", "आपसी खिंचाव"]),
        points: p,
        max: 2.0,
        words: {
            l.pick(match p {
                x if x >= 2.0 => ["you are drawn to each other equally; each holds the other easily", "നിങ്ങൾ പരസ്പരം ഒരുപോലെ ആകർഷിക്കപ്പെടുന്നു; ഓരോരുത്തരും മറ്റേയാളെ എളുപ്പം ചേർത്തുനിർത്തും", "आप एक-दूसरे की ओर बराबर खिंचते हैं; हर कोई दूसरे को आसानी से थामे रखता है"],
                x if x >= 1.0 => ["you are different but get on; the pull is there on both sides", "നിങ്ങൾ വ്യത്യസ്തരാണെങ്കിലും ഒത്തുപോകും; ആകർഷണം രണ്ടു ഭാഗത്തുമുണ്ട്", "आप अलग हैं पर निभ जाती है; खिंचाव दोनों ओर है"],
                x if x > 0.0 => ["one of you is drawn more than the other; balance it with attention", "ഒരാൾ മറ്റേയാളേക്കാൾ കൂടുതൽ ആകർഷിക്കപ്പെടുന്നു; ശ്രദ്ധ കൊണ്ട് അത് സമനിലയിലാക്കുക", "एक दूसरे से ज़्यादा खिंचता है; ध्यान देकर इसे संतुलित करें"],
                _ => ["neither naturally holds the other; closeness has to be built on purpose", "ആരും മറ്റേയാളെ സ്വാഭാവികമായി ചേർത്തുനിർത്തുന്നില്ല; അടുപ്പം മനഃപൂർവം പണിതെടുക്കണം", "कोई भी दूसरे को स्वाभाविक रूप से नहीं थामता; नज़दीकी जान-बूझकर बनानी होगी"],
            })
            .to_string()
        },
        source: source_in(l, MC),
    });

    // 3. Tara
    let ta = (nb + 27 - na) % 27 % 9;
    let tb = (na + 27 - nb) % 27 % 9;
    let good = |t: usize| !matches!(t, 2 | 4 | 6);
    let p = (if good(ta) { 1.5 } else { 0.0 }) + (if good(tb) { 1.5 } else { 0.0 });
    kootas.push(Koota {
        key: "Tara",
        name: l.pick(["Luck together", "ഒരുമിച്ചുള്ള ഭാഗ്യം", "साथ की किस्मत"]),
        points: p,
        max: 3.0,
        words: l.pick(if p >= 3.0 {
            ["you bring each other luck: good things tend to happen when you are together", "നിങ്ങൾ പരസ്പരം ഭാഗ്യം കൊണ്ടുവരുന്നു: ഒരുമിച്ചായിരിക്കുമ്പോൾ നല്ല കാര്യങ്ങൾ നടക്കാറുണ്ട്", "आप एक-दूसरे के लिए किस्मत लाते हैं: साथ होने पर अच्छी बातें होती हैं"]
        } else if p > 0.0 {
            ["one of you brings the other luck; the other brings tests", "ഒരാൾ മറ്റേയാൾക്ക് ഭാഗ്യം കൊണ്ടുവരുന്നു; മറ്റേയാൾ പരീക്ഷണങ്ങളും", "एक दूसरे के लिए किस्मत लाता है; दूसरा परीक्षाएँ"]
        } else {
            ["you test each other's luck; big decisions go better when made calmly, apart first", "നിങ്ങൾ പരസ്പരം ഭാഗ്യം പരീക്ഷിക്കുന്നു; വലിയ തീരുമാനങ്ങൾ ആദ്യം വെവ്വേറെ, ശാന്തമായി ആലോചിച്ചാൽ നന്നായി പോകും", "आप एक-दूसरे की किस्मत परखते हैं; बड़े फ़ैसले पहले अलग-अलग, शांति से सोचकर लें तो अच्छे रहते हैं"]
        })
        .to_string(),
        source: source_in(l, MC),
    });

    // 4. Yoni
    let (ya, yb) = (YONI[na], YONI[nb]);
    let p = YONI_POINTS[ya][yb] as f64;
    kootas.push(Koota {
        key: "Yoni",
        name: l.pick(["Physical ease", "ശാരീരിക അടുപ്പം", "शारीरिक सहजता"]),
        points: p,
        max: 4.0,
        words: {
            l.pick(match p as u8 {
                4 => ["deep physical ease and closeness", "ആഴമുള്ള ശാരീരിക സുഖവും അടുപ്പവും", "गहरी शारीरिक सहजता और नज़दीकी"],
                3 => ["easy, friendly closeness", "എളുപ്പമുള്ള, സൗഹൃദമുള്ള അടുപ്പം", "आसान, दोस्ताना नज़दीकी"],
                2 => ["physical ease comes with time", "ശാരീരിക അടുപ്പം സമയംകൊണ്ട് വരും", "शारीरिक सहजता समय के साथ आती है"],
                1 => ["your bodies and habits must learn each other", "നിങ്ങളുടെ ശരീരങ്ങളും ശീലങ്ങളും പരസ്പരം പഠിക്കണം", "आपके शरीर और आदतों को एक-दूसरे को सीखना होगा"],
                _ => ["physical closeness needs patience and gentleness", "ശാരീരിക അടുപ്പത്തിന് ക്ഷമയും മൃദുത്വവും വേണം", "शारीरिक नज़दीकी को धैर्य और कोमलता चाहिए"],
            })
            .to_string()
        },
        source: source_in(l, MC),
    });

    // 5. Graha Maitri
    let (la, lb) = (sign_lord(sa as usize), sign_lord(sb as usize));
    let p = if la == lb {
        5.0
    } else {
        let r1 = natural_relation(la, lb);
        let r2 = natural_relation(lb, la);
        match (r1, r2) {
            (1, 1) => 5.0,
            (1, 0) | (0, 1) => 4.0,
            (0, 0) => 3.0,
            (1, -1) | (-1, 1) => 1.0,
            (0, -1) | (-1, 0) => 0.5,
            _ => 0.0,
        }
    };
    kootas.push(Koota {
        key: "Graha Maitri",
        name: l.pick(["Meeting of minds", "മനസ്സുകളുടെ ചേർച്ച", "मन का मेल"]),
        points: p,
        max: 5.0,
        words: l.pick(match p {
            x if x >= 5.0 => ["your minds meet: you understand each other without many words", "നിങ്ങളുടെ മനസ്സുകൾ ചേരുന്നു: അധികം വാക്കുകളില്ലാതെ പരസ്പരം മനസ്സിലാകും", "आपके मन मिलते हैं: बिना ज़्यादा कहे एक-दूसरे को समझ लेते हैं"],
            x if x >= 4.0 => ["your minds meet most days", "മിക്ക ദിവസങ്ങളിലും നിങ്ങളുടെ മനസ്സുകൾ ചേരും", "ज़्यादातर दिन आपके मन मिलते हैं"],
            x if x >= 3.0 => ["you respect each other's thinking more than you share it", "പരസ്പരം ചിന്തയെ പങ്കിടുന്നതിനേക്കാൾ നിങ്ങൾ മാനിക്കുന്നു", "आप एक-दूसरे की सोच साझा करने से ज़्यादा उसका सम्मान करते हैं"],
            x if x >= 1.0 => ["one of you gives more understanding than the other", "ഒരാൾ മറ്റേയാളേക്കാൾ കൂടുതൽ മനസ്സിലാക്കൽ നൽകുന്നു", "एक दूसरे से ज़्यादा समझ देता है"],
            _ => ["your minds pull in different directions; talk things through slowly", "നിങ്ങളുടെ മനസ്സുകൾ രണ്ടു ദിശയിലേക്ക് വലിക്കുന്നു; കാര്യങ്ങൾ പതുക്കെ സംസാരിച്ചു തീർക്കുക", "आपके मन अलग दिशाओं में खिंचते हैं; बातें धीरे-धीरे सुलझाएँ"],
        })
        .to_string(),
        source: source_in(l, BPHS_MAITRI),
    });

    // 6. Gana
    let (ga, gb) = (GANA[na], GANA[nb]);
    let (gg, gbr) = if a_is_groom { (ga, gb) } else { (gb, ga) };
    let p = match (gg, gbr) {
        (x, y) if x == y => 6.0,
        (0, 1) => 6.0,
        (1, 0) => 5.0,
        (0, 2) | (2, 0) => 1.0,
        _ => 0.0,
    };
    kootas.push(Koota {
        key: "Gana",
        name: l.pick(["Temperament", "പ്രകൃതം", "स्वभाव"]),
        points: p,
        max: 6.0,
        words: {
            l.pick(match p as u8 {
                6 | 5 => ["your tempers suit each other", "നിങ്ങളുടെ പ്രകൃതങ്ങൾ പരസ്പരം ചേരുന്നു", "आपके स्वभाव एक-दूसरे से मेल खाते हैं"],
                1 => ["one of you is gentle and the other fiery; sharp words come at times", "ഒരാൾക്ക് ശാന്തപ്രകൃതം, മറ്റേയാൾക്ക് ചൂടൻ പ്രകൃതം; ഇടയ്ക്ക് കടുത്ത വാക്കുകൾ വരും", "एक नरम है और दूसरा तेज़; कभी-कभी कड़वी बातें निकलती हैं"],
                _ => ["one of you is easy-going and the other fiery; anger needs a rule between you", "ഒരാൾക്ക് അയഞ്ഞ പ്രകൃതം, മറ്റേയാൾക്ക് ചൂടൻ പ്രകൃതം; ദേഷ്യത്തിന് നിങ്ങൾക്കിടയിൽ ഒരു നിയമം വേണം", "एक सहज है और दूसरा तेज़; गुस्से के लिए आपके बीच एक नियम चाहिए"],
            })
            .to_string()
        },
        source: source_in(l, MC),
    });

    // 7. Bhakoot
    let d = ((sb as i64 - sa as i64).rem_euclid(12) + 1) as u8;
    let dosha = matches!(d, 2 | 12 | 5 | 9 | 6 | 8);
    let friends = la == lb || (natural_relation(la, lb) == 1 && natural_relation(lb, la) == 1);
    let p = if !dosha || friends { 7.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Bhakoot",
        name: l.pick(["Shared fortunes", "ഒരുമിച്ചുള്ള ഐശ്വര്യം", "साझा भाग्य"]),
        points: p,
        max: 7.0,
        words: l.pick(if !dosha {
            ["money, health and children flow well between you", "പണവും ആരോഗ്യവും മക്കളും നിങ്ങൾക്കിടയിൽ നന്നായി ഒഴുകുന്നു", "पैसा, सेहत और संतान आपके बीच अच्छे से चलते हैं"]
        } else if friends {
            ["a hard combination for shared fortunes, softened because your natures are friendly", "ഒരുമിച്ചുള്ള ഐശ്വര്യത്തിന് പ്രയാസമുള്ള ഒരു ചേർച്ച, നിങ്ങളുടെ പ്രകൃതങ്ങൾ സൗഹൃദമുള്ളതായതിനാൽ മയപ്പെട്ടത്", "साझा भाग्य के लिए कठिन मेल, जो आपके दोस्ताना स्वभाव से नरम पड़ गया है"]
        } else {
            match d {
                2 | 12 => ["money and harsh words need care between you", "പണത്തിനും കടുത്ത വാക്കുകൾക്കും നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം", "पैसे और कड़वी बातों में आपके बीच ध्यान चाहिए"],
                5 | 9 => ["children and beliefs need care between you", "മക്കളുടെയും വിശ്വാസങ്ങളുടെയും കാര്യത്തിൽ നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം", "संतान और आस्था में आपके बीच ध्यान चाहिए"],
                _ => ["health and sudden turns need care between you", "ആരോഗ്യത്തിലും പെട്ടെന്നുള്ള മാറ്റങ്ങളിലും നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം", "सेहत और अचानक बदलावों में आपके बीच ध्यान चाहिए"],
            }
        })
        .to_string(),
        source: source_in(l, MC),
    });

    // 8. Nadi
    let (da, db) = (NADI[na], NADI[nb]);
    let same_nadi = da == db;
    let cancelled = same_nadi && ((sa == sb && na != nb) || (na == nb && sa != sb));
    let p = if !same_nadi || cancelled { 8.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Nadi",
        name: l.pick(["Health and children", "ആരോഗ്യവും മക്കളും", "सेहत और संतान"]),
        points: p,
        max: 8.0,
        words: {
            l.pick(if !same_nadi {
                ["your bodies balance each other; health and children are protected", "നിങ്ങളുടെ ശരീരങ്ങൾ പരസ്പരം സമനിലയിലാക്കുന്നു; ആരോഗ്യവും മക്കളും കാക്കപ്പെടുന്നു", "आपके शरीर एक-दूसरे का संतुलन बनाते हैं; सेहत और संतान सुरक्षित रहते हैं"]
            } else if cancelled {
                ["a classic warning about health and children, cancelled in your case", "ആരോഗ്യത്തെയും മക്കളെയും കുറിച്ചുള്ള ഒരു പ്രസിദ്ധ മുന്നറിയിപ്പ്, നിങ്ങളുടെ കാര്യത്തിൽ റദ്ദായത്", "सेहत और संतान को लेकर एक जानी-मानी चेतावनी, जो आपके मामले में रद्द हो जाती है"]
            } else {
                ["the most serious warning in matching: health and children need care and the remedies below", "പൊരുത്തത്തിലെ ഏറ്റവും ഗൗരവമുള്ള മുന്നറിയിപ്പ്: ആരോഗ്യത്തിനും മക്കൾക്കും കരുതലും താഴെയുള്ള പരിഹാരങ്ങളും വേണം", "मिलान की सबसे गंभीर चेतावनी: सेहत और संतान को ध्यान और नीचे दिए उपाय चाहिए"]
            })
            .to_string()
        },
        source: source_in(l, MC),
    });

    let total: f64 = kootas.iter().map(|k| k.points).sum();
    let verdict = l.pick(if total >= 28.0 {
        ["Excellent match", "മികച്ച പൊരുത്തം", "बहुत बढ़िया मेल"]
    } else if total >= 24.0 {
        ["Very good", "വളരെ നല്ലത്", "बहुत अच्छा"]
    } else if total >= 18.0 {
        ["Good enough", "തൃപ്തികരം", "ठीक-ठाक अच्छा"]
    } else if total >= 13.0 {
        ["Asks for care", "കരുതൽ ചോദിക്കുന്നു", "ध्यान माँगता है"]
    } else {
        ["Asks for much care", "ഏറെ കരുതൽ ചോദിക്കുന്നു", "बहुत ध्यान माँगता है"]
    });

    let (ka, kwa) = kuja(a, l);
    let (kb, kwb) = kuja(b, l);
    let kuja_words = if ka && kb {
        l.pick(["Both of you carry the same sign of heat in married life, so it balances out: neither burns the other.", "നിങ്ങൾ രണ്ടുപേർക്കും ദാമ്പത്യത്തിൽ ചൂടിന്റെ ഒരേ സൂചനയുണ്ട്, അതിനാൽ അത് സമനിലയിലാകും: ആരും മറ്റേയാളെ പൊള്ളിക്കില്ല.", "आप दोनों में वैवाहिक जीवन में गर्मी का एक ही संकेत है, इसलिए संतुलन बन जाता है: कोई दूसरे को नहीं जलाता।"]).to_string()
    } else if ka || kb {
        let kw = if ka { &kwa } else { &kwb };
        match l {
            Lang::En => format!(
                "The {} person carries {}; the other does not. Expect sharper quarrels than usual; the remedies below cool it.",
                if ka { "first" } else { "second" },
                kw
            ),
            Lang::Ml => format!("{} ജനനസമയത്ത് {kw} ഉണ്ട്; മറ്റേയാൾക്ക് ഇല്ല. പതിവിലും കടുത്ത വഴക്കുകൾ വരാം; താഴെയുള്ള പരിഹാരങ്ങൾ അത് തണുപ്പിക്കും.", person(l, ka, true)),
            Lang::Hi => format!("{} में {kw} है; दूसरे में नहीं। आम से तीखे झगड़े हो सकते हैं; नीचे दिए उपाय इसे ठंडा करते हैं।", person(l, ka, false)),
        }
    } else {
        match l {
            Lang::En => format!("First person: {kwa}. Second person: {kwb}."),
            Lang::Ml => format!("ഒന്നാമത്തെ ആൾ: {kwa}. രണ്ടാമത്തെ ആൾ: {kwb}."),
            Lang::Hi => format!("पहले व्यक्ति: {kwa}। दूसरे व्यक्ति: {kwb}।"),
        }
    };

    // The heaviest kootas speak first.
    let mut strengths = Vec::new();
    let mut frictions = Vec::new();
    let mut by_weight: Vec<&Koota> = kootas.iter().collect();
    by_weight.sort_by(|a, b| b.max.partial_cmp(&a.max).unwrap_or(std::cmp::Ordering::Equal));
    for k in by_weight {
        if k.points >= k.max * 0.75 && k.max >= 2.0 {
            strengths.push(format!("{}: {}", k.name, k.words));
        } else if k.points <= k.max * 0.25 {
            frictions.push(format!("{}: {}", k.name, k.words));
        }
    }
    // Moon-to-Moon synastry beyond the kootas.
    let say = |t: [&str; 3]| l.pick(t).to_string();
    match d {
        7 => strengths.push(say(["Emotionally you complete each other: the classic bond of life partners.", "വികാരപരമായി നിങ്ങൾ പരസ്പരം പൂർത്തിയാക്കുന്നു: ജീവിതപങ്കാളികളുടെ പ്രസിദ്ധമായ ബന്ധം.", "भावनात्मक रूप से आप एक-दूसरे को पूरा करते हैं: जीवनसाथियों का जाना-माना रिश्ता।"])),
        1 => strengths.push(say(["You feel the same moods on the same days: easy to understand, but low days come together.", "ഒരേ ദിവസങ്ങളിൽ നിങ്ങൾക്ക് ഒരേ മനോനിലയാണ്: മനസ്സിലാക്കാൻ എളുപ്പം, പക്ഷേ മോശം ദിവസങ്ങളും ഒരുമിച്ച് വരും.", "एक ही दिनों में आपका मन एक-सा रहता है: समझना आसान, पर उदास दिन भी साथ आते हैं।"])),
        3 | 11 => strengths.push(say(["Friendship and gain flow easily between you.", "സൗഹൃദവും നേട്ടവും നിങ്ങൾക്കിടയിൽ എളുപ്പം ഒഴുകുന്നു.", "दोस्ती और लाभ आपके बीच आसानी से बहते हैं।"])),
        _ => {}
    }
    if favourable_from_moon(DashaLord::Moon).contains(&d) && d != 1 && d != 7 {
        strengths.push(say(["Your moods lift each other more often than they drag.", "നിങ്ങളുടെ മനോനിലകൾ പരസ്പരം താഴ്ത്തുന്നതിനേക്കാൾ കൂടുതൽ ഉയർത്തുന്നു.", "आपके मन एक-दूसरे को नीचे खींचने से ज़्यादा ऊपर उठाते हैं।"]));
    }
    let venus_a = a.facts.planet(DashaLord::Venus).sign as i64;
    let venus_b = b.facts.planet(DashaLord::Venus).sign as i64;
    let vd = ((venus_b - venus_a).rem_euclid(12) + 1) as u8;
    if matches!(vd, 1 | 5 | 7 | 9) {
        strengths.push(say(["You share the same taste in love, comfort and pleasure.", "സ്നേഹത്തിലും സുഖത്തിലും ആനന്ദത്തിലും നിങ്ങൾക്ക് ഒരേ അഭിരുചിയാണ്.", "प्रेम, सुख और आनंद में आपकी रुचि एक जैसी है।"]));
    } else if matches!(vd, 6 | 8) {
        frictions.push(say(["What pleases one of you in love and comfort puzzles the other: ask, do not assume.", "സ്നേഹത്തിലും സുഖത്തിലും ഒരാൾക്ക് ഇഷ്ടമുള്ളത് മറ്റേയാളെ കുഴക്കുന്നു: ചോദിക്കുക, ഊഹിക്കരുത്.", "प्रेम और सुख में जो एक को अच्छा लगता है, वह दूसरे को उलझाता है: पूछें, अंदाज़ा न लगाएँ।"]));
    }

    let trim_end = |s: &str| s.trim_end_matches(|c| c == '.' || c == '।').to_string();
    let headline = match l {
        Lang::En => format!(
            "{verdict}: {total:.1} of 36. {}{}",
            if strengths.is_empty() {
                String::new()
            } else {
                format!("Strongest, {}. ", strengths[0].replacen(": ", ": ", 1))
            },
            if frictions.is_empty() {
                "Nothing in the matching stands against you.".to_string()
            } else {
                format!("Hardest, {}.", frictions[0].trim_end_matches('.'))
            }
        ),
        Lang::Ml => format!(
            "{verdict}: 36-ൽ {total:.1}. {}{}",
            strengths.first().map(|s| format!("ഏറ്റവും ബലമുള്ളത്, {}. ", trim_end(s))).unwrap_or_default(),
            frictions.first().map(|s| format!("ഏറ്റവും പ്രയാസമുള്ളത്, {}.", trim_end(s))).unwrap_or_else(|| "പൊരുത്തത്തിൽ ഒന്നും നിങ്ങൾക്കെതിരല്ല.".to_string())
        ),
        Lang::Hi => format!(
            "{verdict}: 36 में से {total:.1}। {}{}",
            strengths.first().map(|s| format!("सबसे मज़बूत, {}। ", trim_end(s))).unwrap_or_default(),
            frictions.first().map(|s| format!("सबसे कठिन, {}।", trim_end(s))).unwrap_or_else(|| "मिलान में कुछ भी आपके ख़िलाफ़ नहीं।".to_string())
        ),
    };

    let points_of = |name: &str| kootas.iter().find(|k| k.key == name).map(|k| k.points).unwrap_or(0.0);
    let mut remedies = dosha_remedies_in(
        points_of("Nadi") == 0.0,
        points_of("Bhakoot") == 0.0,
        points_of("Gana") <= 1.0,
        ka != kb,
        points_of("Graha Maitri") <= 1.0,
        points_of("Yoni") <= 1.0,
        l,
    );
    // Deeds and giving first; prayers and rites (optional) after them.
    remedies.sort_by_key(|r| r.optional);

    let care = bond_care(&kootas, &remedies, a, b, na, nb, d, la, lb, ka, kb, vd, l);
    let together_verdict = l
        .pick(if total >= 28.0 {
            ["A naturally strong bond", "സ്വാഭാവികമായി ബലമുള്ള ബന്ധം", "स्वाभाविक रूप से मज़बूत रिश्ता"]
        } else if total >= 24.0 {
            ["A strong bond", "ബലമുള്ള ബന്ധം", "मज़बूत रिश्ता"]
        } else if total >= 18.0 {
            ["A good bond, with places to tend", "നല്ല ബന്ധം, ചില ഇടങ്ങളിൽ കരുതലോടെ", "अच्छा रिश्ता, कुछ जगहों पर देखभाल के साथ"]
        } else {
            ["A bond that grows with care", "കരുതലിലൂടെ വളരുന്ന ബന്ധം", "देखभाल से बढ़ने वाला रिश्ता"]
        })
        .to_string();
    // The strongest part, as a noun for "strong in …" (a koota's name), or the
    // sentence itself when the strongest is one of the Moon-to-Moon lines.
    let strong_in = strengths.first().map(|s| match s.split_once(':') {
        Some((name, _)) => (name.to_string(), true),
        None => (trim_end(s), false),
    });
    let together_headline = match l {
        Lang::En => format!(
            "You already share a life, so this is not a yes or a no. Matching was made for choosing a partner; once two people are together, what matters is how they care for each other. {}{}",
            if strengths.is_empty() {
                String::new()
            } else {
                format!("Your bond is naturally strong in {}. ", strengths[0].split(':').next().unwrap_or("").to_lowercase())
            },
            match care.len() {
                0 => "Nothing in the match stands against you.".to_string(),
                1 => "One part of it asks for care; below is what it brings, what each of you can do for the other, and what helps.".to_string(),
                n => format!("{n} parts of it ask for care; below is what each brings, what each of you can do for the other, and what helps."),
            }
        ),
        Lang::Ml => format!(
            "നിങ്ങൾ ഇതിനകം ഒരു ജീവിതം പങ്കിടുന്നു, അതിനാൽ ഇതൊരു അതെ എന്നോ ഇല്ല എന്നോ ഉള്ള ഉത്തരമല്ല. പൊരുത്തം നോക്കുന്നത് പങ്കാളിയെ തിരഞ്ഞെടുക്കാനാണ്; രണ്ടുപേർ ഒരുമിച്ചായിക്കഴിഞ്ഞാൽ പ്രധാനം അവർ പരസ്പരം എങ്ങനെ കരുതുന്നു എന്നതാണ്. {}{}",
            match &strong_in {
                Some((n, true)) => format!("{n} എന്നതിൽ നിങ്ങളുടെ ബന്ധം സ്വാഭാവികമായി ബലമുള്ളതാണ്. "),
                Some((s, false)) => format!("{s}. "),
                None => String::new(),
            },
            match care.len() {
                0 => "പൊരുത്തത്തിൽ ഒന്നും നിങ്ങൾക്കെതിരല്ല.".to_string(),
                1 => "അതിൽ ഒരു ഭാഗം കരുതൽ ചോദിക്കുന്നു; അത് എന്തു കൊണ്ടുവരുന്നു, ഓരോരുത്തർക്കും മറ്റേയാൾക്കായി എന്തു ചെയ്യാം, എന്തു സഹായിക്കും എന്നിവ താഴെ.".to_string(),
                n => format!("അതിൽ {n} ഭാഗങ്ങൾ കരുതൽ ചോദിക്കുന്നു; ഓരോന്നും എന്തു കൊണ്ടുവരുന്നു, ഓരോരുത്തർക്കും മറ്റേയാൾക്കായി എന്തു ചെയ്യാം, എന്തു സഹായിക്കും എന്നിവ താഴെ."),
            }
        ),
        Lang::Hi => format!(
            "आप पहले से एक जीवन साझा करते हैं, इसलिए यह हाँ या ना का सवाल नहीं। मिलान साथी चुनने के लिए बना था; दो लोग साथ हो जाएँ तो मायने यह रखता है कि वे एक-दूसरे का ख़याल कैसे रखते हैं। {}{}",
            match &strong_in {
                Some((n, true)) => format!("{n} में आपका रिश्ता स्वाभाविक रूप से मज़बूत है। "),
                Some((s, false)) => format!("{s}। "),
                None => String::new(),
            },
            match care.len() {
                0 => "मिलान में कुछ भी आपके ख़िलाफ़ नहीं।".to_string(),
                1 => "इसका एक हिस्सा ध्यान माँगता है; नीचे लिखा है कि वह क्या लाता है, आप दोनों एक-दूसरे के लिए क्या कर सकते हैं, और क्या मदद करता है।".to_string(),
                n => format!("इसके {n} हिस्से ध्यान माँगते हैं; नीचे लिखा है कि हर एक क्या लाता है, आप दोनों एक-दूसरे के लिए क्या कर सकते हैं, और क्या मदद करता है।"),
            }
        ),
    };

    Kundali {
        together_verdict,
        together_headline,
        care,
        total,
        kootas,
        verdict: verdict.to_string(),
        headline,
        kuja_a: ka,
        kuja_b: kb,
        kuja_words,
        moon_a: moon_words(ma, l),
        moon_b: moon_words(mb, l),
        moon_distance: d,
        strengths,
        frictions,
        remedies,
    }
}

/// Two people at one instant and over the months around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Couple {
    /// The instant.
    pub at: Instant,
    /// The first person's Love score now.
    pub a_love: f64,
    /// The second person's.
    pub b_love: f64,
    /// The pair's score now, `[0, 1]`: the two Loves and the match.
    pub together: f64,
    /// Band of `together`.
    pub band: Band,
    /// Two or three sentences with dates.
    pub headline: String,
    /// Saturn relative to each Moon.
    pub a_saturn: SaturnPhase,
    /// For the second person.
    pub b_saturn: SaturnPhase,
    /// Plain notes about the two skies.
    pub notes: Vec<String>,
    /// The first month the pair climbs a band: `(instant, score)`.
    pub eases: Option<(Instant, f64)>,
    /// The best month of the window.
    pub peak: Option<(Instant, f64)>,
    /// The lowest month of the window.
    pub low: Option<(Instant, f64)>,
    /// Monthly slow samples, twelve months back to twenty-four ahead: `(at, a, b, together)`.
    pub series: Vec<(Instant, f64, f64, f64)>,
    /// What lifts the first person's love next.
    pub a_relief: Vec<Relief>,
    /// What lifts the second person's.
    pub b_relief: Vec<Relief>,
    /// What drags the first person's love now, in words.
    pub a_cause: String,
    /// The second's.
    pub b_cause: String,
    /// The first person's briefing: Moon, periods, Saturn, the moment, the first lift.
    pub a_briefing: Briefing,
    /// The second's.
    pub b_briefing: Briefing,
    /// The pair's own paragraphs: what the two seasons mean together.
    pub together_words: Vec<String>,
}

/// The engine speaks to the person whose chart it reads; the second chart's sentences
/// are turned to the third person so a screen can show both side by side.
pub fn theirs(s: &str) -> String {
    s.replace("Your birth Moon", "Their birth Moon")
        .replace("You are in", "They are in")
        .replace("your Moon", "their Moon")
        .replace("your love", "their love")
        .replace("your birth", "their birth")
        .replace("your strongest", "their strongest")
        .replace("your chart", "their chart")
        .replace("You ", "They ")
        .replace(" you ", " them ")
        .replace("your ", "their ")
}

fn third_person_briefing(b: &mut Briefing) {
    b.saturn_words = theirs(&b.saturn_words);
    for p in b.paragraphs.iter_mut() {
        *p = theirs(p);
    }
    for c in b.circled.iter_mut() {
        c.words = theirs(&c.words);
    }
    if let Some(r) = b.remedy.as_mut() {
        r.cause = theirs(&r.cause);
    }
}

fn together_of(a: f64, b: f64, match_total: f64) -> f64 {
    (0.65 * (a + b) / 2.0 + 0.35 * (match_total / 36.0)).clamp(0.0, 1.0)
}

/// The pair at `at`, both standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn couple(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    match_total: f64,
    at: Instant,
) -> Result<Couple, ForecastError> {
    couple_in(spk, eop, a, b, obs, system, ayanamsa, match_total, at, Lang::En)
}

/// What a coming lift brings, in Malayalam or Hindi, rebuilt from the influence behind
/// it (the English sentence of `explain` is not spliced in).
fn relief_words(lang: Lang, r: &Relief) -> String {
    let good = r.gain >= 0.0;
    let core = match (r.planet, good) {
        (Some(p), true) => match lang {
            Lang::Hi => format!("{} का साथ मिलता है", lang::support(lang, p)),
            _ => format!("{} തുണയാകുന്നു", lang::support(lang, p)),
        },
        (Some(p), false) => match lang {
            Lang::Hi => format!("{} से सावधान रहें", lang::pressure(lang, p)),
            _ => format!("{} ശ്രദ്ധിക്കുക", lang::pressure(lang, p)),
        },
        (None, _) => lang.pick(["", "ഒരു നല്ല മാറ്റം", "एक अच्छा बदलाव"]).to_string(),
    };
    match (r.kind, r.planet) {
        ("dasha", Some(p)) => match lang {
            Lang::Hi => format!("जीवन में एक नया दौर शुरू होता है ({}); {core}", lang::chapter_theme(lang, p)),
            _ => format!("ജീവിതത്തിൽ പുതിയൊരു ഘട്ടം തുടങ്ങുന്നു ({}); {core}", lang::chapter_theme(lang, p)),
        },
        _ => core,
    }
}

/// What weighs on one person's love now, in Malayalam or Hindi, from the cause's
/// influence.
fn cause_words(lang: Lang, e: &crate::explain::Explanation) -> String {
    e.causes
        .iter()
        .find(|c| matches!(c.kind, "dasha" | "gochara"))
        .and_then(|c| c.planet)
        .map(|p| lang::pressure(lang, p).to_string())
        .unwrap_or_default()
}

/// [`couple`] in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn couple_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    match_total: f64,
    at: Instant,
    lang: Lang,
) -> Result<Couple, ForecastError> {
    let l = lang;
    let en = l == Lang::En;
    let lon = obs.longitude_deg;
    let ma = moment(spk, eop, a, obs, system, ayanamsa, at)?;
    let mb = moment(spk, eop, b, obs, system, ayanamsa, at)?;
    let a_love = ma.lives[0].score;
    let b_love = mb.lives[0].score;
    let together = together_of(a_love, b_love, match_total);
    let band = Band::of(together);

    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let start = at.add_ns(-12 * month);
    let sa = slow_series(spk, eop, a, obs, ayanamsa, start, month, 37)?;
    let sb = slow_series(spk, eop, b, obs, ayanamsa, start, month, 37)?;
    let series: Vec<(Instant, f64, f64, f64)> = sa
        .iter()
        .zip(sb.iter())
        .map(|((t, va), (_, vb))| (*t, va[0], vb[0], together_of(va[0], vb[0], match_total)))
        .collect();
    let now_slow = series
        .iter()
        .min_by_key(|s| (s.0.ns_since(at)).abs())
        .map(|s| s.3)
        .unwrap_or(together);
    let band_slow = Band::of(now_slow).max(band);
    let ahead: Vec<&(Instant, f64, f64, f64)> = series.iter().filter(|s| s.0 > at).collect();
    let eases = ahead
        .iter()
        .find(|s| Band::of(s.3) > band_slow)
        .map(|s| (s.0, s.3));
    let peak = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v >= s.3 => acc,
            _ => Some((s.0, s.3)),
        });
    let low = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v <= s.3 => acc,
            _ => Some((s.0, s.3)),
        });

    let ea = explain(spk, eop, a, obs, system, ayanamsa, Life::Love, at)?;
    let eb = explain(spk, eop, b, obs, system, ayanamsa, Life::Love, at)?;
    let lifts = |e: &crate::explain::Explanation| -> Vec<Relief> {
        let mut v: Vec<Relief> = e
            .relief
            .iter()
            .filter(|r| r.gain >= 0.01 && r.lasting)
            .cloned()
            .collect();
        v.truncate(3);
        v
    };
    // Each person's briefing for love (English: the second turned to the third person).
    let a_briefing = briefing_in(l, spk, eop, a, obs, system, ayanamsa, &[Life::Love], at)?;
    let mut b_briefing = briefing_in(l, spk, eop, b, obs, system, ayanamsa, &[Life::Love], at)?;
    if !en {
        return Ok(couple_local(CoupleLocal {
            lang: l,
            at,
            lon,
            a_love,
            b_love,
            together,
            band,
            now_slow,
            match_total,
            ma: &ma,
            mb: &mb,
            ea: &ea,
            eb: &eb,
            a_relief: lifts(&ea),
            b_relief: lifts(&eb),
            eases,
            peak,
            low,
            series,
            a_briefing,
            b_briefing,
            moon_d: ((b.moon_sign() - a.moon_sign()).rem_euclid(12) + 1) as u8,
        }));
    }
    let a_relief = lifts(&ea);
    let mut b_relief = lifts(&eb);
    for r in b_relief.iter_mut() {
        r.what = theirs(&r.what);
    }
    let lasting = |e: &crate::explain::Explanation| -> String {
        e.causes
            .iter()
            .find(|c| matches!(c.kind, "dasha" | "gochara"))
            .map(|c| c.what.clone())
            .unwrap_or_default()
    };
    let a_cause = lasting(&ea);
    let b_cause = theirs(&lasting(&eb));

    let mut notes = Vec::new();
    match (ma.saturn, mb.saturn) {
        (SaturnPhase::Clear, SaturnPhase::Clear) => notes.push("Neither of you is in a long testing period now.".into()),
        (x, SaturnPhase::Clear) => notes.push(format!(
            "The first person is in {}; the second is not. The weight is one-sided, so the other can carry the pair through it.",
            long_test(x)
        )),
        (SaturnPhase::Clear, y) => notes.push(format!(
            "The second person is in {}; the first is not. The weight is one-sided, so the other can carry the pair through it.",
            long_test(y)
        )),
        (x, y) => notes.push(format!(
            "Both of you are under pressure at once ({}; {}). A shared hard time passes, and it binds those who stay.",
            long_test(x),
            long_test(y)
        )),
    }
    if !a_cause.is_empty() {
        notes.push(format!("For the first person's love just now: {a_cause}."));
    }
    if !b_cause.is_empty() {
        notes.push(format!("For the second person's love just now: {b_cause}."));
    }
    if let (Some(pa), Some(pb)) = (ma.dasha.first(), mb.dasha.first()) {
        notes.push(format!(
            "The first person's life is centred on {}; the second's on {}.",
            chapter_theme(pa.lord),
            chapter_theme(pb.lord)
        ));
    }

    let mut headline = format!(
        "Together you score {:.0} now ({}): love is {:.0} for the first person and {:.0} for the second.",
        together * 100.0,
        band.word(),
        a_love * 100.0,
        b_love * 100.0
    );
    if band <= Band::Steady {
        let worst = if a_love <= b_love { &a_cause } else { &b_cause };
        if !worst.is_empty() {
            headline.push_str(&format!(" The heavier side: {worst}."));
        }
        let first = a_relief.iter().chain(b_relief.iter()).min_by_key(|r| r.at.tai_ns());
        if let Some(r) = first {
            headline.push_str(&format!(" The first lift comes on {}: {}.", date_words(r.at, lon), r.what));
        }
        if let Some((t, s)) = eases {
            headline.push_str(&format!(" The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)));
        } else if let Some((t, _)) = peak {
            headline.push_str(&format!(" The best month in the next two years is around {}.", date_words(t, lon)));
        }
    } else if let Some((t, s)) = low {
        if Band::of(s) < Band::of(now_slow).min(band) {
            headline.push_str(&format!(" Hold what you have around {}, the lowest month ahead.", date_words(t, lon)));
        }
    }

    // The pair's own paragraphs.
    third_person_briefing(&mut b_briefing);
    let mut together_words = Vec::new();
    together_words.push(format!(
        "You are living two different seasons at once. {} {} {}",
        match (ma.dasha.first(), mb.dasha.first()) {
            (Some(pa), Some(pb)) => format!(
                "The first person's life is about {} until {}; the second's is about {} until {}.",
                chapter_theme(pa.lord),
                date_words(pa.end, lon),
                chapter_theme(pb.lord),
                date_words(pb.end, lon)
            ),
            _ => String::new(),
        },
        match (ma.saturn, mb.saturn) {
            (SaturnPhase::Clear, SaturnPhase::Clear) => "Neither of you is in a long testing period now.".to_string(),
            (x, SaturnPhase::Clear) => format!("The first person is in {}; the second is free of it and can carry the pair.", long_test(x)),
            (SaturnPhase::Clear, y) => format!("The second person is in {}; the first is free of it and can carry the pair.", long_test(y)),
            (x, y) => format!("Both of you are in a hard time ({}; {}): a shared trial, which binds those who stay.", long_test(x), long_test(y)),
        },
        match (a_briefing.saturn_until, b_briefing.saturn_until) {
            (Some(ta), _) if ma.saturn != SaturnPhase::Clear => format!("The first person's testing time ends {}.", date_words(ta, lon)),
            (_, Some(tb)) if mb.saturn != SaturnPhase::Clear => format!("The second person's testing time ends {}.", date_words(tb, lon)),
            _ => String::new(),
        }
    ));
    let d = ((b.moon_sign() - a.moon_sign()).rem_euclid(12) + 1) as u8;
    together_words.push(format!(
        "{} Love scores {:.0} for the first person and {:.0} for the second right now; as a pair, with your match of {:.1} out of 36 counted in, you score {:.0}. {}",
        match d {
            1 => "You feel the same moods on the same days.",
            7 => "Emotionally you complete each other, the classic bond of partners.",
            3 | 11 => "Friendship and gain flow easily between you.",
            2 | 12 => "Money and harsh words need care between you.",
            5 | 9 => "Children and beliefs need care between you.",
            6 | 8 => "Health and sudden turns need care between you.",
            _ => "Your bond takes effort rather than ease, but it builds.",
        },
        a_love * 100.0,
        b_love * 100.0,
        match_total,
        together * 100.0,
        if a_love < b_love {
            format!("The wait is on the first person's side: {}.", a_cause)
        } else if b_love < a_love {
            format!("The wait is on the second person's side: {}.", b_cause)
        } else {
            "You both carry the same weight today.".to_string()
        }
    ));
    let first_lift = a_relief.iter().map(|r| (r, "first")).chain(b_relief.iter().map(|r| (r, "second"))).min_by_key(|(r, _)| r.at.tai_ns());
    if let Some((r, who)) = first_lift {
        together_words.push(format!(
            "The first lasting lift for the pair comes on {}, on the {who} person's side: {} ({:+.0} points). {}",
            date_words(r.at, lon),
            r.what,
            r.gain * 100.0,
            match (eases, peak) {
                (Some((t, s)), _) => format!("The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)),
                (None, Some((t, _))) => format!("The best month in the next two years is around {}.", date_words(t, lon)),
                _ => String::new(),
            }
        ));
    }

    Ok(Couple {
        at,
        a_love,
        b_love,
        together,
        band,
        headline,
        a_saturn: ma.saturn,
        b_saturn: mb.saturn,
        notes,
        eases,
        peak,
        low,
        series,
        a_relief,
        b_relief,
        a_cause,
        b_cause,
        a_briefing,
        b_briefing,
        together_words,
    })
}

/// What the Malayalam and Hindi pair reading is built from.
struct CoupleLocal<'a> {
    lang: Lang,
    at: Instant,
    lon: f64,
    a_love: f64,
    b_love: f64,
    together: f64,
    band: Band,
    now_slow: f64,
    match_total: f64,
    ma: &'a crate::forecast::Moment,
    mb: &'a crate::forecast::Moment,
    ea: &'a crate::explain::Explanation,
    eb: &'a crate::explain::Explanation,
    a_relief: Vec<Relief>,
    b_relief: Vec<Relief>,
    eases: Option<(Instant, f64)>,
    peak: Option<(Instant, f64)>,
    low: Option<(Instant, f64)>,
    series: Vec<(Instant, f64, f64, f64)>,
    a_briefing: Briefing,
    b_briefing: Briefing,
    moon_d: u8,
}

/// The pair in Malayalam or Hindi. The same reckoning as the English, said natively:
/// the English sentences of `explain` are not spliced in, and "the first person" and
/// "the second person" take the forms the screen replaces with names.
fn couple_local(c: CoupleLocal) -> Couple {
    let l = c.lang;
    let hi = l == Lang::Hi;
    let date = |t: Instant| lang::date(l, t, c.lon);
    let bw = |s: f64| lang::band_word(l, Band::of(s)).to_string();
    let (p1, p2) = (person(l, true, false), person(l, false, false));
    let (g1, g2) = (person(l, true, true), person(l, false, true));
    let mut a_relief = c.a_relief;
    let mut b_relief = c.b_relief;
    for r in a_relief.iter_mut().chain(b_relief.iter_mut()) {
        r.what = relief_words(l, r);
    }
    let a_cause = cause_words(l, c.ea);
    let b_cause = cause_words(l, c.eb);
    let (ma, mb) = (c.ma, c.mb);
    let lt = |p: SaturnPhase| lang::long_test(l, p);

    let mut notes = Vec::new();
    notes.push(match (ma.saturn, mb.saturn) {
        (SaturnPhase::Clear, SaturnPhase::Clear) => l.pick(["", "നിങ്ങളിൽ ആരും ഇപ്പോൾ ഒരു നീണ്ട പരീക്ഷണകാലത്തിലല്ല.", "आप में से कोई भी अभी किसी लंबे कठिन दौर में नहीं है।"]).to_string(),
        (x, SaturnPhase::Clear) | (SaturnPhase::Clear, x) => {
            let (who, other) = if mb.saturn == SaturnPhase::Clear { (p1, p2) } else { (p2, p1) };
            if hi {
                format!("{who} अभी इस दौर में हैं: {}; {other} नहीं। बोझ एक तरफ़ है, इसलिए दूसरा जोड़ी को इससे पार ले जा सकता है।", lt(x))
            } else {
                format!("{who} ഇപ്പോൾ ഈ ഘട്ടത്തിലാണ്: {}; {other} അല്ല. ഭാരം ഒരു ഭാഗത്തു മാത്രമാണ്, അതിനാൽ മറ്റേയാൾക്ക് ഇരുവരെയും അതിലൂടെ കൊണ്ടുപോകാം.", lt(x))
            }
        }
        (x, y) => {
            if hi {
                format!("आप दोनों एक साथ दबाव में हैं ({}; {})। साझा कठिन समय बीत जाता है, और जो साथ रहते हैं उन्हें और जोड़ देता है।", lt(x), lt(y))
            } else {
                format!("നിങ്ങൾ രണ്ടുപേരും ഒരേ സമയം സമ്മർദ്ദത്തിലാണ് ({}; {}). ഒരുമിച്ചുള്ള പ്രയാസകാലം കടന്നുപോകും, കൂടെ നിൽക്കുന്നവരെ അത് കൂടുതൽ അടുപ്പിക്കും.", lt(x), lt(y))
            }
        }
    });
    for (g, cause) in [(g1, &a_cause), (g2, &b_cause)] {
        if !cause.is_empty() {
            notes.push(if hi { format!("{g} के प्रेम में अभी: {cause}।") } else { format!("{g} പ്രണയത്തിൽ ഇപ്പോൾ: {cause}.") });
        }
    }
    if let (Some(pa), Some(pb)) = (ma.dasha.first(), mb.dasha.first()) {
        let (ta, tb) = (lang::chapter_theme(l, pa.lord), lang::chapter_theme(l, pb.lord));
        notes.push(if hi {
            format!("{p1} का जीवन अभी {ta} के इर्द-गिर्द है; {p2} का {tb} के इर्द-गिर्द।")
        } else {
            format!("{g1} ജീവിതം ഇപ്പോൾ {ta} എന്നിവയെ ചുറ്റിയാണ്; {g2} ജീവിതം {tb} എന്നിവയെ ചുറ്റിയും.")
        });
    }

    let (t100, a100, b100) = (c.together * 100.0, c.a_love * 100.0, c.b_love * 100.0);
    let band_w = lang::band_word(l, c.band);
    let mut headline = if hi {
        format!("साथ में अभी आपका स्कोर {t100:.0} ({band_w}) है: {p1} का प्रेम {a100:.0}, {p2} का {b100:.0}।")
    } else {
        format!("ഒരുമിച്ച് ഇപ്പോൾ നിങ്ങൾക്ക് {t100:.0} ({band_w}): {g1} പ്രണയം {a100:.0}, {g2} പ്രണയം {b100:.0}.")
    };
    let climbs = |t: Instant, s: f64| if hi { format!(" {} से जोड़ी {} स्तर तक पहुँचती है।", date(t), bw(s)) } else { format!(" {} മുതൽ ഇരുവരും {} എന്ന നിലയിലേക്ക് ഉയരും.", date(t), bw(s)) };
    let best = |t: Instant| if hi { format!(" अगले दो साल का सबसे अच्छा महीना लगभग {} के आसपास है।", date(t)) } else { format!(" അടുത്ത രണ്ടു വർഷത്തിലെ ഏറ്റവും നല്ല മാസം ഏകദേശം {} ആണ്.", date(t)) };
    if c.band <= Band::Steady {
        // The screen shows the heavier side on its own, so it is not repeated here.
        let first = a_relief.iter().chain(b_relief.iter()).min_by_key(|r| r.at.tai_ns());
        if let Some(r) = first {
            headline.push_str(&if hi { format!(" पहला उछाल {} को आता है: {}।", date(r.at), r.what) } else { format!(" ആദ്യത്തെ ഉയർച്ച {}-ന് വരും: {}.", date(r.at), r.what) });
        }
        if let Some((t, s)) = c.eases {
            headline.push_str(&climbs(t, s));
        } else if let Some((t, _)) = c.peak {
            headline.push_str(&best(t));
        }
    } else if let Some((t, s)) = c.low {
        if Band::of(s) < Band::of(c.now_slow).min(c.band) {
            headline.push_str(&if hi { format!(" {} के आसपास, आगे के सबसे धीमे महीने में, जो है उसे सँभालकर रखें।", date(t)) } else { format!(" മുന്നിലുള്ള ഏറ്റവും താഴ്ന്ന മാസമായ {} ഓടെ, ഉള്ളത് കാത്തുസൂക്ഷിക്കുക.", date(t)) });
        }
    }

    let mut together_words = Vec::new();
    let seasons = match (ma.dasha.first(), mb.dasha.first()) {
        (Some(pa), Some(pb)) => {
            let (ta, tb) = (lang::chapter_theme(l, pa.lord), lang::chapter_theme(l, pb.lord));
            if hi {
                format!("{p1} का जीवन {} तक {ta} के बारे में है; {p2} का {} तक {tb} के बारे में।", date(pa.end), date(pb.end))
            } else {
                format!("{} വരെ {g1} ജീവിതം {ta} എന്നിവയെക്കുറിച്ചാണ്; {} വരെ {g2} ജീവിതം {tb} എന്നിവയെക്കുറിച്ചും.", date(pa.end), date(pb.end))
            }
        }
        _ => String::new(),
    };
    let tests = match (ma.saturn, mb.saturn) {
        (SaturnPhase::Clear, SaturnPhase::Clear) => l.pick(["", "നിങ്ങളിൽ ആരും ഇപ്പോൾ ഒരു നീണ്ട പരീക്ഷണകാലത്തിലല്ല.", "आप में से कोई भी अभी किसी लंबे कठिन दौर में नहीं है।"]).to_string(),
        (x, SaturnPhase::Clear) | (SaturnPhase::Clear, x) => {
            let (who, other) = if mb.saturn == SaturnPhase::Clear { (p1, p2) } else { (p2, p1) };
            if hi {
                format!("{who} इस दौर में हैं: {}; {other} इससे मुक्त हैं और जोड़ी को सँभाल सकते हैं।", lt(x))
            } else {
                format!("{who} ഈ ഘട്ടത്തിലാണ്: {}; {other} അതിൽ നിന്ന് മുക്തമാണ്, ഇരുവരെയും താങ്ങാം.", lt(x))
            }
        }
        (x, y) => {
            if hi {
                format!("आप दोनों कठिन समय में हैं ({}; {}): एक साझा परीक्षा, जो साथ रहने वालों को जोड़ती है।", lt(x), lt(y))
            } else {
                format!("നിങ്ങൾ രണ്ടുപേരും പ്രയാസകാലത്തിലാണ് ({}; {}): ഒരുമിച്ചുള്ള ഒരു പരീക്ഷണം, കൂടെ നിൽക്കുന്നവരെ അത് ചേർത്തുനിർത്തും.", lt(x), lt(y))
            }
        }
    };
    let ends = match (c.a_briefing.saturn_until, c.b_briefing.saturn_until) {
        (Some(ta), _) if ma.saturn != SaturnPhase::Clear => if hi { format!("{p1} का कठिन समय {} को ख़त्म होता है।", date(ta)) } else { format!("{g1} പരീക്ഷണകാലം {}-ന് തീരും.", date(ta)) },
        (_, Some(tb)) if mb.saturn != SaturnPhase::Clear => if hi { format!("{p2} का कठिन समय {} को ख़त्म होता है।", date(tb)) } else { format!("{g2} പരീക്ഷണകാലം {}-ന് തീരും.", date(tb)) },
        _ => String::new(),
    };
    let open = l.pick(["", "നിങ്ങൾ ഒരേ സമയം രണ്ടു വ്യത്യസ്ത കാലങ്ങളിലൂടെയാണ് കടന്നുപോകുന്നത്.", "आप एक ही समय में दो अलग-अलग दौर जी रहे हैं।"]);
    together_words.push([open, &seasons, &tests, &ends].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" "));

    let bond = l.pick(match c.moon_d {
        1 => ["", "ഒരേ ദിവസങ്ങളിൽ നിങ്ങൾക്ക് ഒരേ മനോനിലയാണ്.", "एक ही दिनों में आपका मन एक-सा रहता है।"],
        7 => ["", "വികാരപരമായി നിങ്ങൾ പരസ്പരം പൂർത്തിയാക്കുന്നു, പങ്കാളികളുടെ പ്രസിദ്ധമായ ബന്ധം.", "भावनात्मक रूप से आप एक-दूसरे को पूरा करते हैं, साथियों का जाना-माना रिश्ता।"],
        3 | 11 => ["", "സൗഹൃദവും നേട്ടവും നിങ്ങൾക്കിടയിൽ എളുപ്പം ഒഴുകുന്നു.", "दोस्ती और लाभ आपके बीच आसानी से बहते हैं।"],
        2 | 12 => ["", "പണത്തിനും കടുത്ത വാക്കുകൾക്കും നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം.", "पैसे और कड़वी बातों में आपके बीच ध्यान चाहिए।"],
        5 | 9 => ["", "മക്കളുടെയും വിശ്വാസങ്ങളുടെയും കാര്യത്തിൽ നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം.", "संतान और आस्था में आपके बीच ध्यान चाहिए।"],
        6 | 8 => ["", "ആരോഗ്യത്തിലും പെട്ടെന്നുള്ള മാറ്റങ്ങളിലും നിങ്ങൾക്കിടയിൽ കരുതൽ വേണം.", "सेहत और अचानक बदलावों में आपके बीच ध्यान चाहिए।"],
        _ => ["", "നിങ്ങളുടെ ബന്ധം എളുപ്പത്തേക്കാൾ പരിശ്രമം ചോദിക്കുന്നു, പക്ഷേ അത് വളർന്നുകൊണ്ടിരിക്കുന്നു.", "आपका रिश्ता आसानी से ज़्यादा मेहनत माँगता है, पर बनता जाता है।"],
    });
    let side = |g: &str, p: &str, cause: &str| -> String {
        match (hi, cause.is_empty()) {
            (true, false) => format!("इंतज़ार {p} की तरफ़ है: {cause}।"),
            (true, true) => format!("इंतज़ार {p} की तरफ़ है।"),
            (false, false) => format!("കാത്തിരിപ്പ് {g} ഭാഗത്താണ്: {cause}."),
            (false, true) => format!("കാത്തിരിപ്പ് {g} ഭാഗത്താണ്."),
        }
    };
    let wait = if c.a_love < c.b_love {
        side(g1, p1, &a_cause)
    } else if c.b_love < c.a_love {
        side(g2, p2, &b_cause)
    } else {
        l.pick(["", "ഇന്ന് നിങ്ങൾ രണ്ടുപേരും ഒരേ ഭാരമാണ് ചുമക്കുന്നത്.", "आज आप दोनों एक-सा बोझ उठा रहे हैं।"]).to_string()
    };
    let m = c.match_total;
    together_words.push(if hi {
        format!("{bond} अभी {p1} का प्रेम {a100:.0} और {p2} का {b100:.0} है; 36 में से {m:.1} के आपके मिलान को जोड़कर, जोड़ी के रूप में आपका स्कोर {t100:.0} है। {wait}")
    } else {
        format!("{bond} ഇപ്പോൾ {g1} പ്രണയം {a100:.0}, {g2} പ്രണയം {b100:.0}; 36-ൽ {m:.1} എന്ന നിങ്ങളുടെ പൊരുത്തവും കൂട്ടി, ഇരുവരുമായി നിങ്ങൾക്ക് {t100:.0}. {wait}")
    });
    let first_lift = a_relief.iter().map(|r| (r, true)).chain(b_relief.iter().map(|r| (r, false))).min_by_key(|(r, _)| r.at.tai_ns());
    if let Some((r, first)) = first_lift {
        let after = match (c.eases, c.peak) {
            (Some((t, s)), _) => climbs(t, s).trim_start().to_string(),
            (None, Some((t, _))) => best(t).trim_start().to_string(),
            _ => String::new(),
        };
        let g = person(l, first, true);
        let gain = r.gain * 100.0;
        together_words.push(
            if hi {
                format!("जोड़ी के लिए पहला टिकाऊ उछाल {} को, {g} की तरफ़ से आता है: {} ({gain:+.0} अंक)। {after}", date(r.at), r.what)
            } else {
                format!("ഇരുവർക്കുമുള്ള ആദ്യത്തെ നിലനിൽക്കുന്ന ഉയർച്ച {}-ന്, {g} ഭാഗത്തുനിന്ന് വരും: {} ({gain:+.0} പോയിന്റ്). {after}", date(r.at), r.what)
            }
            .trim_end()
            .to_string(),
        );
    }

    Couple {
        at: c.at,
        a_love: c.a_love,
        b_love: c.b_love,
        together: c.together,
        band: c.band,
        headline,
        a_saturn: ma.saturn,
        b_saturn: mb.saturn,
        notes,
        eases: c.eases,
        peak: c.peak,
        low: c.low,
        series: c.series,
        a_relief,
        b_relief,
        a_cause,
        b_cause,
        a_briefing: c.a_briefing,
        b_briefing: c.b_briefing,
        together_words,
    }
}

/// The talk window: the first hour in the next `days` when both charts read well for
/// love at once (both hora fits at or above 0.55, outside the cautions, both daily
/// love scores at or above 0.5). `(start, end, mean fit)`.
#[allow(clippy::too_many_arguments)]
pub fn talk_window(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    from: Instant,
    days: usize,
) -> Result<Option<(Instant, Instant, f64)>, ForecastError> {
    let sa = crate::forecast::series(spk, eop, a, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    let sb = crate::forecast::series(spk, eop, b, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    for d in 0..days.max(1) {
        if sa[d].scores[0] < 0.5 || sb[d].scores[0] < 0.5 {
            continue;
        }
        let t = from.add_ns(d as i128 * NS_PER_DAY + 6 * 3_600 * kshana_time::NS_PER_SEC);
        let ma = moment(spk, eop, a, obs, system, ayanamsa, t)?;
        let mb = moment(spk, eop, b, obs, system, ayanamsa, t)?;
        for (ha, hb) in ma.hours.iter().zip(mb.hours.iter()) {
            if ha.caution || ha.end <= from {
                continue;
            }
            if ha.fit[0] >= 0.55 && hb.fit[0] >= 0.55 {
                return Ok(Some((ha.start.max(from), ha.end, (ha.fit[0] + hb.fit[0]) / 2.0)));
            }
        }
    }
    Ok(None)
}

/// A question about the pair, answered from both charts.
#[derive(Clone, Debug, PartialEq)]
pub struct CoupleAnswer {
    /// The first chart's answer about love.
    pub a: Answer,
    /// The second chart's.
    pub b: Answer,
    /// Combined verdict.
    pub verdict: String,
    /// Combined score, `[0, 1]`.
    pub score: f64,
    /// Combined confidence.
    pub confidence: f64,
    /// Two sentences.
    pub headline: String,
    /// The window both charts open, or the earlier of the two.
    pub when_start: Option<Instant>,
    /// Its end.
    pub when_end: Option<Instant>,
}

/// Ask `question` about love for both charts and read them together.
#[allow(clippy::too_many_arguments)]
pub fn couple_ask(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    question: Question,
    at: Instant,
) -> Result<CoupleAnswer, ForecastError> {
    couple_ask_in(spk, eop, a, b, obs, system, ayanamsa, question, at, Lang::En)
}

/// [`couple_ask`] in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn couple_ask_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    question: Question,
    at: Instant,
    lang: Lang,
) -> Result<CoupleAnswer, ForecastError> {
    let l = lang;
    let aa = answer_in(spk, eop, a, obs, system, ayanamsa, Life::Love, question, at, l)?;
    let ab = answer_in(spk, eop, b, obs, system, ayanamsa, Life::Love, question, at, l)?;
    let score = (0.5 * aa.score + 0.5 * ab.score).clamp(0.0, 1.0);
    let confidence = ((aa.confidence + ab.confidence) / 2.0 - (aa.score - ab.score).abs() * 0.5).clamp(0.3, 0.95);
    // Each chart has answered the question in its own terms; the pair's verdict says
    // whether the two agree, and on whose side the wait is.
    let yes = |v: &str| {
        v.starts_with("Yes")
            || v.starts_with("Take it")
            || v.starts_with("Go ahead")
            || v.starts_with("Within")
            || v.starts_with("Nothing has truly left")
    };
    let (ya, yb) = (yes(&aa.verdict_en), yes(&ab.verdict_en));
    let p = |t: [&str; 3]| l.pick(t).to_string();
    let verdict: String = match (ya, yb, question) {
        (true, true, Question::WillItReturn) => p(["Yes, you come back together", "അതെ, നിങ്ങൾ വീണ്ടും അടുക്കും", "हाँ, आप फिर साथ आते हैं"]),
        (true, true, Question::WillItRecover) => p(["Yes, it mends between you", "അതെ, നിങ്ങൾക്കിടയിൽ ഇത് ശരിയാകും", "हाँ, आपके बीच यह ठीक होता है"]),
        (true, true, Question::ShouldI) => p(["Take the step, both of you", "ചുവടു വയ്ക്കുക, രണ്ടുപേരും", "क़दम उठाइए, आप दोनों"]),
        (true, true, Question::GoodTimeNow) => p(["Now serves you both", "ഇപ്പോഴത്തെ സമയം രണ്ടുപേർക്കും നല്ലതാണ്", "अभी का समय आप दोनों के लिए ठीक है"]),
        (true, true, Question::When) => {
            let later = if aa.when_start.map(|t| t.tai_ns()) <= ab.when_start.map(|t| t.tai_ns()) { &ab.verdict } else { &aa.verdict };
            match l {
                Lang::En => format!("Both of you: {}", later.to_ascii_lowercase()),
                Lang::Ml => format!("രണ്ടുപേർക്കും: {later}"),
                Lang::Hi => format!("आप दोनों के लिए: {later}"),
            }
        }
        (true, true, _) => p(["Yes, for both of you", "അതെ, രണ്ടുപേർക്കും", "हाँ, आप दोनों के लिए"]),
        (true, false, _) => p(["Yes for you; they need more time", "നിങ്ങൾക്ക് അതെ; അവർക്ക് കുറച്ചുകൂടി സമയം വേണം", "आपके लिए हाँ; उन्हें थोड़ा और समय चाहिए"]),
        (false, true, _) => p(["Yes for them; you need more time", "അവർക്ക് അതെ; നിങ്ങൾക്ക് കുറച്ചുകൂടി സമയം വേണം", "उनके लिए हाँ; आपको थोड़ा और समय चाहिए"]),
        (false, false, Question::WillItReturn) => p(["Not as it was, for either of you", "പഴയതുപോലെയല്ല, രണ്ടുപേർക്കും", "पहले जैसा नहीं, आप दोनों में से किसी के लिए"]),
        (false, false, Question::GoodTimeNow) => p(["Not this hour, for either of you", "ഈ മണിക്കൂറല്ല, രണ്ടുപേർക്കും", "यह घड़ी नहीं, आप दोनों में से किसी के लिए"]),
        (false, false, Question::When) => p(["Not soon, for either of you", "ഉടനെയല്ല, രണ്ടുപേർക്കും", "जल्दी नहीं, आप दोनों में से किसी के लिए"]),
        (false, false, _) => p(["Not yet, for either of you", "ഇപ്പോഴല്ല, രണ്ടുപേർക്കും", "अभी नहीं, आप दोनों में से किसी के लिए"]),
    };
    let (ws, we) = match (aa.when_start, aa.when_end, ab.when_start, ab.when_end) {
        (Some(s1), Some(e1), Some(s2), Some(e2)) => {
            let s = s1.max(s2);
            let e = e1.min(e2);
            if s <= e {
                (Some(s), Some(e))
            } else if s1 <= s2 {
                (Some(s1), Some(e1))
            } else {
                (Some(s2), Some(e2))
            }
        }
        (Some(s1), Some(e1), _, _) => (Some(s1), Some(e1)),
        (_, _, Some(s2), Some(e2)) => (Some(s2), Some(e2)),
        _ => (None, None),
    };
    let wait = l.pick(if (aa.score - ab.score).abs() > 0.15 {
        if aa.score > ab.score {
            ["The wait is on their side: their time has to open first.", "കാത്തിരിപ്പ് അവരുടെ ഭാഗത്താണ്: ആദ്യം അവരുടെ സമയം തുറക്കണം.", "इंतज़ार उनकी तरफ़ है: पहले उनका समय खुलना होगा।"]
        } else {
            ["The wait is on your side: your time has to open first.", "കാത്തിരിപ്പ് നിങ്ങളുടെ ഭാഗത്താണ്: ആദ്യം നിങ്ങളുടെ സമയം തുറക്കണം.", "इंतज़ार आपकी तरफ़ है: पहले आपका समय खुलना होगा।"]
        }
    } else {
        ["You both read the same.", "നിങ്ങൾ രണ്ടുപേരുടെയും വായന ഒരുപോലെയാണ്.", "आप दोनों की पढ़त एक-सी है।"]
    });
    let (sa, sb) = (aa.score * 100.0, ab.score * 100.0);
    let headline = match l {
        Lang::En => format!("{verdict}. For you: \"{}\" ({sa:.0}); for them: \"{}\" ({sb:.0}). {wait}", aa.verdict, ab.verdict),
        Lang::Ml => format!("{verdict}. നിങ്ങൾക്ക്: \"{}\" ({sa:.0}); അവർക്ക്: \"{}\" ({sb:.0}). {wait}", aa.verdict, ab.verdict),
        Lang::Hi => format!("{verdict}। आपके लिए: \"{}\" ({sa:.0}); उनके लिए: \"{}\" ({sb:.0})। {wait}", aa.verdict, ab.verdict),
    };
    Ok(CoupleAnswer {
        a: aa,
        b: ab,
        verdict,
        score,
        confidence,
        headline,
        when_start: ws,
        when_end: we,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn tables_are_well_formed() {
        for row in YONI_POINTS {
            assert_eq!(row.len(), 14);
        }
        for i in 0..14 {
            assert_eq!(YONI_POINTS[i][i], 4);
            for j in 0..14 {
                assert_eq!(YONI_POINTS[i][j], YONI_POINTS[j][i], "yoni {i} {j}");
            }
        }
        assert_eq!(GANA.iter().filter(|g| **g == 0).count(), 9);
        assert_eq!(GANA.iter().filter(|g| **g == 1).count(), 9);
        assert_eq!(NADI.iter().filter(|g| **g == 0).count(), 9);
        for a in 0..5 {
            for b in 0..5 {
                assert_eq!(vashya_points(a, b), vashya_points(b, a));
            }
        }
    }

    #[test]
    fn a_pair_is_matched_and_read_over_time() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let ba = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let bb = Utc::new(1992, 8, 11, 14, 30, 0, 0).unwrap().to_instant().unwrap();
        let a = Natal::compute(&spk, None, ba, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let b = Natal::compute(&spk, None, bb, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let k = kundali(&a, &b, true);
        assert_eq!(k.kootas.len(), 8);
        assert!((0.0..=36.0).contains(&k.total));
        let max: f64 = k.kootas.iter().map(|k| k.max).sum();
        assert_eq!(max, 36.0);
        assert!(!k.headline.is_empty() && !k.kuja_words.is_empty());
        // matching a chart with itself gives every symmetric koota its maximum except Nadi
        let s = kundali(&a, &a, true);
        assert!(s.kootas.iter().filter(|k| k.key != "Nadi").all(|k| k.points == k.max), "{:?}", s.kootas);
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let c = couple(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, k.total, at).unwrap();
        assert_eq!(c.series.len(), 37);
        assert!((0.0..=1.0).contains(&c.together));
        assert!(!c.headline.is_empty() && !c.notes.is_empty());
        let q = couple_ask(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Question::WillItReturn, at).unwrap();
        assert!(!q.verdict.is_empty());
        assert!((0.3..=0.95).contains(&q.confidence));
        use crate::plain::jargon;
        let mut said: Vec<String> = vec![k.headline.clone(), k.kuja_words.clone(), c.headline.clone(), q.headline.clone(), q.verdict.clone(), c.a_cause.clone(), c.b_cause.clone()];
        said.extend(k.kootas.iter().flat_map(|x| [x.name.to_string(), x.words.clone()]));
        said.extend(k.strengths.iter().chain(k.frictions.iter()).cloned());
        said.extend(k.remedies.iter().flat_map(|r| [r.title.to_string(), r.why.to_string()]));
        said.extend(c.notes.iter().chain(c.together_words.iter()).cloned());
        said.extend(c.a_relief.iter().chain(c.b_relief.iter()).map(|r| r.what.clone()));
        for s in &said {
            assert_eq!(jargon(s), None, "{s}");
        }
    }

    #[test]
    fn the_pair_reads_plainly_in_malayalam_and_hindi() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let births = [
            Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap(),
            Utc::new(1992, 8, 11, 14, 30, 0, 0).unwrap().to_instant().unwrap(),
            Utc::new(1985, 7, 3, 12, 0, 0, 0).unwrap().to_instant().unwrap(),
            Utc::new(1978, 11, 30, 22, 0, 0, 0).unwrap().to_instant().unwrap(),
        ];
        let natals: Vec<Natal> = births.iter().map(|b| Natal::compute(&spk, None, *b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap()).collect();
        use crate::plain::jargon_in;
        for lang in [Lang::Ml, Lang::Hi] {
            for (i, a) in natals.iter().enumerate() {
                for b in natals.iter().skip(i) {
                    let k = kundali_in(a, b, i % 2 == 0, lang);
                    let en = kundali(a, b, i % 2 == 0);
                    assert_eq!(k.total, en.total);
                    assert_eq!(k.care.len(), en.care.len());
                    let mut said: Vec<String> = vec![k.verdict.clone(), k.headline.clone(), k.kuja_words.clone(), k.together_verdict.clone(), k.together_headline.clone()];
                    said.extend(k.kootas.iter().flat_map(|x| [x.name.to_string(), x.words.clone()]));
                    said.extend(k.strengths.iter().chain(k.frictions.iter()).cloned());
                    said.extend(k.remedies.iter().flat_map(|r| [r.title.to_string(), r.why.to_string()]));
                    for c in &k.care {
                        said.push(c.area.clone());
                        said.push(c.living.clone());
                        said.extend(c.care.iter().cloned());
                        said.extend(c.remedies.iter().flat_map(|r| [r.title.to_string(), r.why.to_string()]));
                    }
                    for s in &said {
                        assert!(!s.trim().is_empty(), "{lang:?}: empty");
                        assert_eq!(jargon_in(lang, s), None, "{lang:?}: {s}");
                        assert!(!s.contains("person"), "{lang:?}: English left: {s}");
                    }
                }
            }
            let (a, b) = (&natals[0], &natals[1]);
            let k = kundali_in(a, b, true, lang);
            let c = couple_in(&spk, None, a, b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, k.total, at, lang).unwrap();
            let mut said: Vec<String> = vec![c.headline.clone(), c.a_cause.clone(), c.b_cause.clone()];
            said.extend(c.notes.iter().chain(c.together_words.iter()).cloned());
            said.extend(c.a_relief.iter().chain(c.b_relief.iter()).map(|r| r.what.clone()));
            for q in Question::ALL {
                let x = couple_ask_in(&spk, None, a, b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, q, at, lang).unwrap();
                said.push(x.verdict.clone());
                said.push(x.headline.clone());
            }
            for s in &said {
                assert_eq!(jargon_in(lang, s), None, "{lang:?}: {s}");
                for w in ["the first", "the second", "The first", "The second", "Together you", "Love scores"] {
                    assert!(!s.contains(w), "{lang:?}: English left: {s}");
                }
            }
        }
    }
}
