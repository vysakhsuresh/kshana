//! The remedy tree: why a part of life is pressed, and what eases it, laid out so anyone
//! can use it whatever they believe.
//!
//! For one life (or the weakest one), the one to three strongest weighing influences are
//! found with `explain`. Each becomes a cause node: what it is called in plain words, what
//! it does in the person's life, how hard it presses, when it eases, and seven branches of
//! remedies in this order:
//!
//! 1. **Deeds anyone can do**: conduct and service toward the people the influence
//!    governs in the remedial tradition (the old, workers and the disabled for the slow,
//!    heavy influence; the mother and women for the mind; and so on). These carry the
//!    work and need no faith.
//! 2. **Giving**: food, clothes, money or time, by hand, from the graha-daana lists.
//! 3. **Habits**: self-discipline, including fasting as discipline rather than rite.
//! 4. **Nature and creatures**: water, trees and animals.
//! 5. **Quiet the mind**: for any faith or none.
//! 6. **Prayer in your own faith**: faith-neutral; it names the day and the people to
//!    hold in mind and nothing of any faith's doctrine.
//! 7. **Hindu tradition (optional)**: fasts, chants and temples, never required.
//!
//! The graha is the engine's key; the person reads only the pressure and its remedies.

use crate::explain::explain;
use crate::forecast::{moment, ForecastError, Life, Natal};
use crate::lang::{self, any_day, date, day_of, life_lower, life_name, weekday};
use crate::plain::{cap, Lang};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa, DashaLord};

/// One remedy in a branch.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRemedy {
    /// What to do.
    pub title: String,
    /// What it eases, in life terms.
    pub why: String,
    /// Concrete steps.
    pub how: String,
    /// The day to do it ("Saturday", "any day").
    pub day: String,
    /// How much it asks ("once a week").
    pub effort: String,
    /// The branch key it belongs to.
    pub kind: &'static str,
    /// A chant's text, in the reader's script, when the remedy is a chant.
    pub mantra: Option<String>,
    /// True when it costs real money (school fees, a paid rite, a wedding): shown only
    /// behind "Ways that cost money", never first.
    pub costs_money: bool,
    /// Where it comes from: keys of [`crate::sources`] (aliases such as `remedy.deeds`).
    pub source_keys: Vec<&'static str>,
}

/// The source keys of a tree remedy: its branch, and for the tradition branch whether
/// it is a fast, a chant or a temple visit (read from the English title).
pub fn tree_source_keys(branch: &str, title_en: &str, chant: bool) -> Vec<&'static str> {
    let t = title_en.to_lowercase();
    let k = match branch {
        "deeds" => "remedy.deeds",
        "giving" => "remedy.giving",
        "habits" => "remedy.habits",
        "nature" => "remedy.nature",
        "mind" => "remedy.mind",
        "prayer" => "remedy.prayer",
        _ if chant || ["chant", "chalisa", "stotram", "sahasranama", "hridayam", "ashtottaram", "panchakshari", "saptashati", "kavacham"].iter().any(|w| t.contains(w)) => "remedy.chant",
        _ if t.contains("fast") => "remedy.fast",
        _ => "remedy.temple",
    };
    vec![k]
}

/// One branch of remedies.
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    /// `deeds`, `giving`, `habits`, `nature`, `mind`, `prayer`, `tradition`.
    pub key: &'static str,
    /// Title.
    pub title: String,
    /// One or two sentences on what the branch is.
    pub intro: String,
    /// The remedies.
    pub items: Vec<TreeRemedy>,
}

/// One cause: a pressure on the life, and its remedies.
#[derive(Clone, Debug, PartialEq)]
pub struct CauseNode {
    /// The graha behind it (the engine's key; never shown to the person).
    pub key: DashaLord,
    /// `dasha`, `gochara` or `natal` (the engine's key).
    pub source: &'static str,
    /// The pressure in plain words.
    pub title: String,
    /// What it does in the person's life.
    pub what_it_does: String,
    /// When it began, when known.
    pub since: Option<Instant>,
    /// When it eases, when known.
    pub until: Option<Instant>,
    /// The end in words.
    pub until_words: String,
    /// Its weight on the life, in points (0–50).
    pub strength: f64,
    /// "Pressing hard", "Pressing moderately", "Pressing lightly".
    pub strength_words: String,
    /// The remedies, branch by branch.
    pub branches: Vec<Branch>,
    /// The chart reason, shown only when the person taps "How this helps".
    pub how_it_helps: String,
}

/// The whole tree.
#[derive(Clone, Debug, PartialEq)]
pub struct RemedyTree {
    /// The life it is for.
    pub life: Life,
    /// What is wrong and roughly until when.
    pub headline: String,
    /// The causes, strongest first.
    pub causes: Vec<CauseNode>,
}

type T3 = [&'static str; 3];

const E_WEEK: T3 = ["this week", "ഈ ആഴ്ച", "इस हफ़्ते"];
const E_ONCE: T3 = ["once", "ഒരിക്കൽ", "एक बार"];
const E_WEEKLY: T3 = ["once a week", "ആഴ്ചയിൽ ഒരിക്കൽ", "हफ़्ते में एक बार"];
const E_DAILY: T3 = ["every day", "എല്ലാ ദിവസവും", "हर दिन"];
const E_10: T3 = ["10 minutes a day", "ദിവസം 10 മിനിറ്റ്", "रोज़ 10 मिनट"];
const E_40: T3 = ["for 40 days", "40 ദിവസം", "40 दिन तक"];
const E_30: T3 = ["for 30 days", "30 ദിവസം", "30 दिन तक"];
const E_MONTH: T3 = ["this month", "ഈ മാസം", "इस महीने"];
const E_WISH: T3 = ["if you wish", "താൽപര്യമുണ്ടെങ്കിൽ", "अगर आप चाहें"];
const E_3W: T3 = ["three times a week", "ആഴ്ചയിൽ മൂന്ന് തവണ", "हफ़्ते में तीन बार"];
const E_MONTHLY: T3 = ["once a month", "മാസത്തിലൊരിക്കൽ", "महीने में एक बार"];

const CHANT_WHY: T3 = [
    "Supports the deeds above; it does not replace them.",
    "മുകളിലെ പ്രവൃത്തികളെ പിന്തുണയ്ക്കും; അവയ്ക്ക് പകരമല്ല.",
    "ऊपर के कामों का साथ देता है; उनकी जगह नहीं लेता।",
];
const CHANT_HOW: T3 = [
    "108 times, once a week, unhurried.",
    "ആഴ്ചയിൽ ഒരിക്കൽ, 108 തവണ, തിടുക്കമില്ലാതെ.",
    "हफ़्ते में एक बार, 108 बार, बिना जल्दबाज़ी के।",
];
const TEMPLE_WHY: T3 = [
    "A quiet act of devotion for those who keep it. Never required.",
    "ഇത് പാലിക്കുന്നവർക്ക് ഒരു ശാന്തമായ ഭക്തികർമം. ഒരിക്കലും നിർബന്ധമില്ല.",
    "जो इसे मानते हैं उनके लिए एक शांत भक्ति। कभी ज़रूरी नहीं।",
];
const TEMPLE_HOW: T3 = [
    "Go once; sit five minutes; leave something for the temple's workers.",
    "ഒരിക്കൽ പോവുക; അഞ്ച് മിനിറ്റ് ഇരിക്കുക; ക്ഷേത്രത്തിലെ ജോലിക്കാർക്ക് എന്തെങ്കിലും കൊടുക്കുക.",
    "एक बार जाएँ; पाँच मिनट बैठें; मंदिर के कर्मचारियों के लिए कुछ छोड़ें।",
];
const FAST_HOW: T3 = [
    "Eat once, simple food, at the set time. Stop if health does not allow it; the sick, the pregnant and the old need not fast.",
    "നിശ്ചിത സമയത്ത്, ലളിതമായ ഭക്ഷണം ഒരു നേരം. ആരോഗ്യം അനുവദിക്കുന്നില്ലെങ്കിൽ നിർത്തുക; രോഗികളും ഗർഭിണികളും വയോധികരും വ്രതം എടുക്കേണ്ടതില്ല.",
    "तय समय पर एक बार सादा भोजन। सेहत साथ न दे तो छोड़ दें; बीमार, गर्भवती और बुज़ुर्ग व्रत न रखें।",
];

/// `(branch, title, why, how, on the influence's day, effort, mantra)`.
type Item = (&'static str, T3, T3, T3, bool, T3, Option<T3>);

struct Influence {
    effects: T3,
    people_en: &'static str,
    people_ml_for: &'static str,
    people_hi_for: &'static str,
    items: Vec<Item>,
}

fn influence(l: DashaLord) -> Influence {
    match l {
        DashaLord::Sun => Influence {
            effects: [
                "You may feel unseen or overruled; bosses, officials or your father push back. Confidence dips and pride gets hurt easily.",
                "നിങ്ങളെ ആരും കാണുന്നില്ലെന്നോ മറികടക്കുന്നുവെന്നോ തോന്നാം; മേലധികാരികളും ഉദ്യോഗസ്ഥരും അച്ഛനും എതിർത്തേക്കാം. ആത്മവിശ്വാസം കുറയും, അഭിമാനം എളുപ്പം മുറിവേൽക്കും.",
                "आपको लग सकता है कि कोई आपको नहीं देखता या आपकी नहीं चलती; बॉस, अधिकारी या पिता विरोध कर सकते हैं। आत्मविश्वास घटता है और आत्मसम्मान जल्दी आहत होता है।",
            ],
            people_en: "fathers, elders and those in charge",
            people_ml_for: "അച്ഛന്മാർക്കും മുതിർന്നവർക്കും ചുമതലയുള്ളവർക്കും",
            people_hi_for: "पिताओं, बड़ों और ज़िम्मेदारी संभालने वालों",
            items: vec![
                ("deeds",
                 ["Keep your word to those who depend on you", "നിങ്ങളെ ആശ്രയിക്കുന്നവരോട് വാക്ക് പാലിക്കുക", "जो आप पर निर्भर हैं, उनसे किया वादा निभाएँ"],
                 ["Standing is earned by reliability; keeping small promises restores how others see you.", "വിശ്വാസ്യതയിലൂടെയാണ് സ്ഥാനം നേടുന്നത്; ചെറിയ വാഗ്ദാനങ്ങൾ പാലിക്കുമ്പോൾ മറ്റുള്ളവർ നിങ്ങളെ കാണുന്ന രീതി തിരിച്ചുവരും.", "मान भरोसे से कमाया जाता है; छोटे वादे निभाने से लोगों की नज़र में आपकी जगह लौटती है।"],
                 ["Write down three promises you have made. Keep each one this week, on time.", "നിങ്ങൾ കൊടുത്ത മൂന്ന് വാഗ്ദാനങ്ങൾ എഴുതുക. ഈ ആഴ്ച ഓരോന്നും സമയത്ത് പാലിക്കുക.", "अपने किए तीन वादे लिखें। इस हफ़्ते हर एक समय पर निभाएँ।"],
                 false, E_WEEK, None),
                ("deeds",
                 ["Mend things with your father, or honour his memory", "അച്ഛനുമായുള്ള ബന്ധം നന്നാക്കുക, അല്ലെങ്കിൽ അദ്ദേഹത്തിന്റെ ഓർമയെ ആദരിക്കുക", "पिता से रिश्ता सुधारें, या उनकी याद का सम्मान करें"],
                 ["A strained bond with your father blocks recognition; mending it opens doors with seniors too.", "അച്ഛനുമായുള്ള ഉലഞ്ഞ ബന്ധം അംഗീകാരത്തെ തടയും; അത് നന്നാക്കുമ്പോൾ മേലധികാരികളുമായും വാതിലുകൾ തുറക്കും.", "पिता से खिंचा रिश्ता पहचान रोकता है; उसे सुधारने से बड़ों के साथ भी रास्ते खुलते हैं।"],
                 ["Call or visit him this Sunday. If he has passed, do one thing he valued, in his name.", "ഈ ഞായറാഴ്ച അദ്ദേഹത്തെ വിളിക്കുകയോ കാണുകയോ ചെയ്യുക. അദ്ദേഹം ഇല്ലെങ്കിൽ, അദ്ദേഹം വിലമതിച്ച ഒരു കാര്യം അദ്ദേഹത്തിന്റെ പേരിൽ ചെയ്യുക.", "इस रविवार उन्हें फ़ोन करें या मिलें। अगर वे नहीं रहे, तो उनकी पसंद का कोई काम उनके नाम से करें।"],
                 true, E_ONCE, None),
                ("deeds",
                 ["Own one thing you have been blaming on others", "മറ്റുള്ളവരെ കുറ്റപ്പെടുത്തിയിരുന്ന ഒരു കാര്യത്തിന്റെ ഉത്തരവാദിത്തം ഏറ്റെടുക്കുക", "जिस बात का दोष दूसरों पर डाल रहे थे, उसकी ज़िम्मेदारी लें"],
                 ["Confidence grows with ownership and shrinks with excuses.", "ഉത്തരവാദിത്തം ഏൽക്കുമ്പോൾ ആത്മവിശ്വാസം വളരും, ഒഴികഴിവുകളിൽ അത് ചുരുങ്ങും.", "ज़िम्मेदारी लेने से आत्मविश्वास बढ़ता है और बहानों से घटता है।"],
                 ["Say it plainly to the person involved: that part was yours, and here is how it will be put right.", "ബന്ധപ്പെട്ടയാളോട് നേരിട്ട് പറയുക: ആ ഭാഗം നിങ്ങളുടേതായിരുന്നു, ഇങ്ങനെയാണ് അത് ശരിയാക്കുക.", "संबंधित व्यक्ति से साफ़ कहें: वह हिस्सा आपका था, और उसे ऐसे ठीक किया जाएगा।"],
                 false, E_WEEK, None),
                ("giving",
                 ["Feed wheat roti and jaggery to people who labour in the midday heat", "ഉച്ചവെയിലിൽ പണിയെടുക്കുന്നവർക്ക് ഗോതമ്പ് റൊട്ടിയും ശർക്കരയും കൊടുക്കുക", "दोपहर की धूप में मेहनत करने वालों को गेहूँ की रोटी और गुड़ खिलाएँ"],
                 ["Eases trouble with authority and lifts your standing.", "അധികാരികളുമായുള്ള പ്രശ്നങ്ങൾ കുറയ്ക്കും, നിങ്ങളുടെ നില ഉയർത്തും.", "अधिकारियों से परेशानी घटाता है और आपका मान बढ़ाता है।"],
                 ["On Sunday, take a meal to masons, road workers or farm hands at noon.", "ഞായറാഴ്ച ഉച്ചയ്ക്ക് മേസ്തിരിമാർക്കോ റോഡുപണിക്കാർക്കോ കർഷകത്തൊഴിലാളികൾക്കോ ഭക്ഷണം കൊണ്ടുപോവുക.", "रविवार दोपहर मिस्त्रियों, सड़क मज़दूरों या खेतिहर मज़दूरों के लिए खाना ले जाएँ।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Help someone get a government paper or a right they are owed", "ആർക്കെങ്കിലും ഒരു സർക്കാർ രേഖയോ അവർക്ക് കിട്ടേണ്ട അവകാശമോ കിട്ടാൻ സഹായിക്കുക", "किसी को सरकारी कागज़ या उसका हक़ दिलाने में मदद करें"],
                 ["Clearing another's path with officials clears your own: approvals and paperwork move.", "ഉദ്യോഗസ്ഥരുടെ മുന്നിൽ മറ്റൊരാളുടെ വഴി തെളിക്കുമ്പോൾ നിങ്ങളുടേതും തെളിയും: അനുമതികളും കടലാസുകളും നീങ്ങും.", "अधिकारियों के सामने किसी और का रास्ता साफ़ करने से आपका भी होता है: मंज़ूरियाँ और कागज़ी काम आगे बढ़ते हैं।"],
                 ["Fill a form, stand in a queue, or pay a fee for someone who cannot.", "കഴിയാത്ത ഒരാൾക്കുവേണ്ടി ഫോം പൂരിപ്പിക്കുക, ക്യൂവിൽ നിൽക്കുക, അല്ലെങ്കിൽ ഫീസ് അടയ്ക്കുക.", "जो नहीं कर सकता, उसके लिए फ़ॉर्म भरें, लाइन में लगें, या फ़ीस भरें।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Up before sunrise, with twelve slow stretches", "സൂര്യോദയത്തിന് മുമ്പ് എഴുന്നേറ്റ് പന്ത്രണ്ട് തവണ പതുക്കെ ശരീരം നീട്ടുക", "सूर्योदय से पहले उठें और बारह बार धीरे-धीरे शरीर खींचें"],
                 ["Steadies energy, heart and will, and lifts low confidence.", "ഊർജവും ഹൃദയവും മനക്കരുത്തും ഉറപ്പിക്കും, താഴ്ന്ന ആത്മവിശ്വാസം ഉയർത്തും.", "ऊर्जा, दिल और इच्छाशक्ति को स्थिर करता है और गिरा आत्मविश्वास उठाता है।"],
                 ["Wake 20 minutes before sunrise; face the light and stretch slowly twelve times.", "സൂര്യോദയത്തിന് 20 മിനിറ്റ് മുമ്പ് ഉണരുക; വെളിച്ചത്തിന് നേരെ നിന്ന് പതുക്കെ പന്ത്രണ്ട് തവണ സ്ട്രെച്ച് ചെയ്യുക.", "सूर्योदय से 20 मिनट पहले उठें; रोशनी की ओर मुँह करके धीरे-धीरे बारह बार स्ट्रेच करें।"],
                 false, E_40, None),
                ("nature",
                 ["Water a plant at sunrise, facing east", "സൂര്യോദയസമയത്ത് കിഴക്കോട്ട് തിരിഞ്ഞ് ഒരു ചെടി നനയ്ക്കുക", "सूर्योदय के समय पूर्व की ओर मुँह करके पौधे को पानी दें"],
                 ["A calm start that steadies confidence and energy for the day.", "ദിവസത്തേക്കുള്ള ആത്മവിശ്വാസവും ഊർജവും ഉറപ്പിക്കുന്ന ശാന്തമായ തുടക്കം.", "शांत शुरुआत जो दिन भर के लिए आत्मविश्वास और ऊर्जा देती है।"],
                 ["Keep a plant by an east window or door; water it first thing every morning.", "കിഴക്കേ ജനലിനോ വാതിലിനോ അടുത്ത് ഒരു ചെടി വയ്ക്കുക; എല്ലാ രാവിലെയും ആദ്യം അത് നനയ്ക്കുക.", "पूर्व की खिड़की या दरवाज़े के पास एक पौधा रखें; हर सुबह सबसे पहले उसे पानी दें।"],
                 false, E_DAILY, None),
                ("mind",
                 ["Each morning, name one thing you are proud of", "എല്ലാ രാവിലെയും നിങ്ങൾക്ക് അഭിമാനമുള്ള ഒരു കാര്യം ഓർക്കുക", "हर सुबह एक ऐसी बात याद करें जिस पर आपको गर्व है"],
                 ["Quiet self-respect needs no one's approval and steadies a bruised ego.", "ശാന്തമായ ആത്മാഭിമാനത്തിന് ആരുടെയും അംഗീകാരം വേണ്ട; മുറിവേറ്റ അഭിമാനത്തെ അത് ഉറപ്പിക്കും.", "शांत आत्मसम्मान को किसी की मंज़ूरी नहीं चाहिए; यह आहत मन को सँभालता है।"],
                 ["Before your phone, say it to yourself or write it down: one thing done well yesterday.", "ഫോൺ എടുക്കുന്നതിന് മുമ്പ്, ഇന്നലെ നന്നായി ചെയ്ത ഒരു കാര്യം സ്വയം പറയുകയോ എഴുതുകയോ ചെയ്യുക.", "फ़ोन उठाने से पहले, कल अच्छे से किया एक काम ख़ुद से कहें या लिख लें।"],
                 false, E_10, None),
                ("tradition",
                 ["Sunday fast: one meal, no salt, before sunset, for 12 Sundays", "ഞായറാഴ്ച വ്രതം: സൂര്യാസ്തമയത്തിന് മുമ്പ് ഉപ്പില്ലാതെ ഒരു നേരം, 12 ഞായറാഴ്ച", "रविवार का व्रत: सूर्यास्त से पहले बिना नमक एक बार भोजन, 12 रविवार"],
                 ["The classical fast for standing and for the health of the heart and eyes.", "സ്ഥാനത്തിനും ഹൃദയത്തിന്റെയും കണ്ണിന്റെയും ആരോഗ്യത്തിനുമുള്ള പരമ്പരാഗത വ്രതം.", "मान-सम्मान और दिल व आँखों की सेहत के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Aditya Hridayam on Sundays, or the chant below", "ഞായറാഴ്ചകളിൽ ആദിത്യഹൃദയം, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "रविवार को आदित्य हृदय स्तोत्र, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om hraam hreem hraum sah Suryaya namah", "ഓം ഹ്രാം ഹ്രീം ഹ്രൗം സഃ സൂര്യായ നമഃ", "ॐ ह्रां ह्रीं ह्रौं सः सूर्याय नमः"])),
                ("tradition",
                 ["Water offered at sunrise; a Shiva temple on Sunday", "സൂര്യോദയത്തിൽ ജലം അർപ്പിക്കുക; ഞായറാഴ്ച ശിവക്ഷേത്രദർശനം", "सूर्योदय पर जल अर्पित करें; रविवार को शिव मंदिर जाएँ"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Moon => Influence {
            effects: [
                "The mind is restless: worry, mood swings and poor sleep. Home feels unsettled and small things hurt more than they should.",
                "മനസ്സ് അസ്വസ്ഥമാണ്: ആശങ്ക, മനോനിലയിലെ ചാഞ്ചാട്ടം, ഉറക്കക്കുറവ്. വീട് അസ്ഥിരമായി തോന്നും, ചെറിയ കാര്യങ്ങൾ വേണ്ടതിലും കൂടുതൽ വേദനിപ്പിക്കും.",
                "मन बेचैन है: चिंता, मिज़ाज में उतार-चढ़ाव और कच्ची नींद। घर अस्थिर लगता है और छोटी बातें ज़रूरत से ज़्यादा चुभती हैं।",
            ],
            people_en: "mothers, women alone and the grieving",
            people_ml_for: "അമ്മമാർക്കും ഒറ്റയ്ക്കായ സ്ത്രീകൾക്കും ദുഃഖിക്കുന്നവർക്കും",
            people_hi_for: "माँओं, अकेली महिलाओं और शोक में डूबे लोगों",
            items: vec![
                ("deeds",
                 ["Care for your mother in person this week", "ഈ ആഴ്ച നേരിട്ട് അമ്മയെ പരിചരിക്കുക", "इस हफ़्ते ख़ुद जाकर माँ की देखभाल करें"],
                 ["A restless mind and an unsettled home calm down when the bond with your mother is tended.", "അമ്മയുമായുള്ള ബന്ധം പരിപാലിക്കുമ്പോൾ അസ്വസ്ഥമായ മനസ്സും അസ്ഥിരമായ വീടും ശാന്തമാകും.", "माँ से रिश्ते को सँभालने पर बेचैन मन और अस्थिर घर शांत होते हैं।"],
                 ["Visit, cook for her, or sit an hour without your phone. If she has passed, care for an older woman.", "കാണാൻ പോവുക, അമ്മയ്ക്ക് പാചകം ചെയ്യുക, അല്ലെങ്കിൽ ഫോണില്ലാതെ ഒരു മണിക്കൂർ കൂടെയിരിക്കുക. അമ്മ ഇല്ലെങ്കിൽ, ഒരു മുതിർന്ന സ്ത്രീയെ പരിചരിക്കുക.", "मिलने जाएँ, उनके लिए खाना बनाएँ, या बिना फ़ोन एक घंटा साथ बैठें। अगर वे नहीं रहीं, तो किसी बुज़ुर्ग महिला की देखभाल करें।"],
                 true, E_WEEK, None),
                ("deeds",
                 ["Say one true thing you have been holding in, gently", "മനസ്സിൽ അടക്കിവച്ചിരുന്ന ഒരു സത്യം സൗമ്യമായി പറയുക", "जो सच दिल में दबा रखा है, उसे नरमी से कहें"],
                 ["Feelings held back build up until they flood; speaking early prevents the outburst.", "അടക്കിവച്ച വികാരങ്ങൾ കുമിഞ്ഞ് ഒടുവിൽ പൊട്ടിത്തെറിക്കും; നേരത്തെ പറഞ്ഞാൽ അത് തടയാം.", "दबी भावनाएँ जमा होकर फूटती हैं; जल्दी कहने से विस्फोट टलता है।"],
                 ["Choose a calm moment. Start with how you feel, not with what they always do.", "ശാന്തമായ ഒരു നിമിഷം തിരഞ്ഞെടുക്കുക. അവർ എപ്പോഴും ചെയ്യുന്നതിൽ നിന്നല്ല, നിങ്ങൾക്ക് എന്ത് തോന്നുന്നു എന്നതിൽ നിന്ന് തുടങ്ങുക.", "शांत पल चुनें। वे हमेशा क्या करते हैं, उससे नहीं, आपको कैसा लगता है, उससे शुरू करें।"],
                 false, E_WEEK, None),
                ("deeds",
                 ["Comfort someone who is grieving or alone", "ദുഃഖിക്കുന്നവരെയോ ഒറ്റയ്ക്കായവരെയോ ആശ്വസിപ്പിക്കുക", "किसी शोकाकुल या अकेले व्यक्ति को ढाढ़स दें"],
                 ["Comforting another steadies your own mind.", "മറ്റൊരാളെ ആശ്വസിപ്പിക്കുന്നത് നിങ്ങളുടെ മനസ്സിനെയും ഉറപ്പിക്കും.", "दूसरे को सांत्वना देने से अपना मन भी सँभलता है।"],
                 ["Bring food, sit with them, and listen more than you speak.", "ഭക്ഷണം കൊണ്ടുപോവുക, കൂടെയിരിക്കുക, പറയുന്നതിനേക്കാൾ കൂടുതൽ കേൾക്കുക.", "खाना ले जाएँ, साथ बैठें, और बोलने से ज़्यादा सुनें।"],
                 false, E_ONCE, None),
                ("giving",
                 ["Give rice, milk or white cloth on a Monday", "തിങ്കളാഴ്ച അരിയോ പാലോ വെള്ളവസ്ത്രമോ കൊടുക്കുക", "सोमवार को चावल, दूध या सफ़ेद कपड़ा दें"],
                 ["Calms worry, mood swings and trouble at home.", "ആശങ്കയും മനോനിലയിലെ ചാഞ്ചാട്ടവും വീട്ടിലെ പ്രശ്നങ്ങളും ശാന്തമാക്കും.", "चिंता, मिज़ाज के उतार-चढ़ाव और घर की परेशानी को शांत करता है।"],
                 ["Hand it to a woman or a family in need, by your own hand.", "ആവശ്യമുള്ള ഒരു സ്ത്രീക്കോ കുടുംബത്തിനോ സ്വന്തം കൈകൊണ്ട് കൊടുക്കുക.", "किसी ज़रूरतमंद महिला या परिवार को अपने हाथ से दें।"],
                 true, E_ONCE, None),
                ("giving",
                 ["Milk sweets or kheer for children in a shelter or a poor family", "ഒരു അനാഥാലയത്തിലെയോ പാവപ്പെട്ട കുടുംബത്തിലെയോ കുട്ടികൾക്ക് പാൽപ്പായസം", "किसी आश्रम या ग़रीब परिवार के बच्चों के लिए खीर या दूध की मिठाई"],
                 ["Small sweetness given to children returns as peace at home.", "കുട്ടികൾക്ക് കൊടുക്കുന്ന ചെറിയ മധുരം വീട്ടിലെ സമാധാനമായി തിരിച്ചുവരും.", "बच्चों को दी गई थोड़ी मिठास घर में शांति बनकर लौटती है।"],
                 ["Make or buy enough for a dozen children; serve it yourself.", "പന്ത്രണ്ട് കുട്ടികൾക്ക് മതിയാവുന്നത്ര ഉണ്ടാക്കുകയോ വാങ്ങുകയോ ചെയ്ത് സ്വയം വിളമ്പുക.", "दर्जन भर बच्चों के लिए बनाएँ या ख़रीदें; ख़ुद परोसें।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Fixed sleep and waking times for 40 days", "40 ദിവസം ഉറങ്ങാനും ഉണരാനും നിശ്ചിത സമയം", "40 दिन तक सोने-जागने का तय समय"],
                 ["A steady routine is the best medicine for mood swings and poor sleep.", "മനോനിലയിലെ ചാഞ്ചാട്ടത്തിനും ഉറക്കക്കുറവിനും ഏറ്റവും നല്ല മരുന്ന് സ്ഥിരമായ ദിനചര്യയാണ്.", "मिज़ाज के उतार-चढ़ाव और ख़राब नींद की सबसे अच्छी दवा नियमित दिनचर्या है।"],
                 ["Same bedtime, same wake time, screens off 30 minutes before bed.", "ഒരേ ഉറക്കസമയം, ഒരേ ഉണരൽസമയം, ഉറങ്ങുന്നതിന് 30 മിനിറ്റ് മുമ്പ് സ്ക്രീൻ ഓഫ്.", "रोज़ एक ही समय सोएँ और जागें, सोने से 30 मिनट पहले स्क्रीन बंद।"],
                 false, E_40, None),
                ("nature",
                 ["Sit by water for twenty minutes on a Monday", "തിങ്കളാഴ്ച ഇരുപത് മിനിറ്റ് വെള്ളത്തിനരികിൽ ഇരിക്കുക", "सोमवार को बीस मिनट पानी के पास बैठें"],
                 ["The cheapest cure there is for an anxious, heavy mind.", "ആശങ്കയും ഭാരവുമുള്ള മനസ്സിനുള്ള ഏറ്റവും ചെലവുകുറഞ്ഞ മരുന്ന്.", "बेचैन, भारी मन का सबसे सस्ता इलाज।"],
                 ["A river, a pond, the sea, or even a fountain. No phone; just watch the water.", "പുഴ, കുളം, കടൽ, അല്ലെങ്കിൽ ഒരു ജലധാര പോലും. ഫോൺ വേണ്ട; വെള്ളം മാത്രം നോക്കിയിരിക്കുക.", "नदी, तालाब, समुद्र या कोई फव्वारा भी। फ़ोन नहीं; बस पानी को देखें।"],
                 true, E_WEEKLY, None),
                ("mind",
                 ["Ten minutes of still breathing each night", "എല്ലാ രാത്രിയും പത്ത് മിനിറ്റ് ശാന്തമായ ശ്വസനം", "हर रात दस मिनट शांत साँस"],
                 ["A quiet close to the day settles an anxious mind and deepens sleep.", "ദിവസത്തിന്റെ ശാന്തമായ അവസാനം ആശങ്കയുള്ള മനസ്സിനെ ശമിപ്പിക്കും, ഉറക്കം ആഴമുള്ളതാക്കും.", "दिन का शांत अंत बेचैन मन को सँभालता है और नींद गहरी करता है।"],
                 ["Lights low, eyes closed, count ten slow breaths, three times over.", "വെളിച്ചം കുറച്ച്, കണ്ണടച്ച്, പത്ത് പതുക്കെയുള്ള ശ്വാസം എണ്ണുക, മൂന്ന് തവണ.", "रोशनी धीमी, आँखें बंद, दस धीमी साँसें गिनें, तीन बार।"],
                 false, E_10, None),
                ("tradition",
                 ["Monday fast for 16 Mondays: one meal after sunset", "16 തിങ്കളാഴ്ച വ്രതം: സൂര്യാസ്തമയശേഷം ഒരു നേരം", "16 सोमवार का व्रत: सूर्यास्त के बाद एक बार भोजन"],
                 ["The classical fast for a shaken mind.", "ഉലഞ്ഞ മനസ്സിനുള്ള പരമ്പരാഗത വ്രതം.", "डगमगाए मन के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Shiva Panchakshari on Mondays, or the chant below", "തിങ്കളാഴ്ചകളിൽ ശിവ പഞ്ചാക്ഷരി, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "सोमवार को शिव पंचाक्षरी, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om shraam shreem shraum sah Chandraya namah", "ഓം ശ്രാം ശ്രീം ശ്രൗം സഃ ചന്ദ്രായ നമഃ", "ॐ श्रां श्रीं श्रौं सः चन्द्राय नमः"])),
                ("tradition",
                 ["Milk abhisheka at a Shiva or Parvati temple on Monday", "തിങ്കളാഴ്ച ശിവക്ഷേത്രത്തിലോ പാർവതീക്ഷേത്രത്തിലോ പാലഭിഷേകം", "सोमवार को शिव या पार्वती मंदिर में दूध से अभिषेक"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Mars => Influence {
            effects: [
                "Tempers run hot: quarrels and haste; take extra care on the road and with tools. Disputes over land, money or with siblings flare quickly.",
                "ദേഷ്യം പെട്ടെന്ന് വരും: വഴക്ക്, ധൃതി; റോഡിലും ഉപകരണങ്ങളുമായും കൂടുതൽ ശ്രദ്ധ വേണം. ഭൂമിയെച്ചൊല്ലിയോ പണത്തെച്ചൊല്ലിയോ സഹോദരങ്ങളുമായോ ഉള്ള തർക്കങ്ങൾ വേഗം ആളിക്കത്തും.",
                "गुस्सा जल्दी आता है: झगड़े और जल्दबाज़ी; सड़क पर और औज़ारों के साथ ज़्यादा सावधानी रखें। ज़मीन, पैसे या भाई-बहनों से जुड़े विवाद जल्दी भड़कते हैं।",
            ],
            people_en: "brothers, soldiers and those in risky work",
            people_ml_for: "സഹോദരന്മാർക്കും സൈനികർക്കും സാഹസികമായ ജോലി ചെയ്യുന്നവർക്കും",
            people_hi_for: "भाइयों, सैनिकों और जोखिम भरा काम करने वालों",
            items: vec![
                ("deeds",
                 ["Walk away from one argument a day", "ദിവസവും ഒരു തർക്കത്തിൽ നിന്ന് മാറിനിൽക്കുക", "रोज़ एक बहस से हट जाएँ"],
                 ["Quarrels feed this heat; every fight you step out of cools the whole stretch.", "വഴക്കുകൾ ഈ ചൂടിനെ ആളിക്കത്തിക്കും; നിങ്ങൾ ഒഴിവാക്കുന്ന ഓരോ വഴക്കും ഈ കാലത്തെ മുഴുവൻ തണുപ്പിക്കും.", "झगड़े इस गर्मी को हवा देते हैं; हर लड़ाई से हटना पूरे दौर को ठंडा करता है।"],
                 ["When voices rise, say you will talk in an hour, and leave the room.", "ശബ്ദം ഉയരുമ്പോൾ, ഒരു മണിക്കൂർ കഴിഞ്ഞ് സംസാരിക്കാമെന്ന് പറഞ്ഞ് മുറിയിൽ നിന്ന് മാറുക.", "जब आवाज़ें ऊँची हों, कहें कि एक घंटे बाद बात करेंगे, और कमरे से बाहर चले जाएँ।"],
                 false, E_DAILY, None),
                ("deeds",
                 ["Make peace with a brother, sister or old rival", "ഒരു സഹോദരനോടോ സഹോദരിയോടോ പഴയ എതിരാളിയോടോ സമാധാനത്തിലാവുക", "किसी भाई, बहन या पुराने विरोधी से सुलह करें"],
                 ["Disputes with siblings and over shared land ease when one side lowers the sword first.", "ഒരു വശം ആദ്യം വാൾ താഴ്ത്തുമ്പോൾ സഹോദരങ്ങളുമായും പങ്കിട്ട ഭൂമിയെച്ചൊല്ലിയുമുള്ള തർക്കങ്ങൾ അയയും.", "भाई-बहनों और साझा ज़मीन के झगड़े तब नरम पड़ते हैं जब एक पक्ष पहले तलवार नीचे करे।"],
                 ["Call them. Talk about anything except the dispute, for once.", "അവരെ വിളിക്കുക. ഇത്തവണ തർക്കമല്ലാത്ത എന്തിനെക്കുറിച്ചും സംസാരിക്കുക.", "उन्हें फ़ोन करें। इस बार झगड़े के अलावा किसी भी बात पर बात करें।"],
                 false, E_ONCE, None),
                ("deeds",
                 ["Drive slower and use tools with care", "വേഗം കുറച്ച് വണ്ടിയോടിക്കുക, ഉപകരണങ്ങൾ ശ്രദ്ധയോടെ ഉപയോഗിക്കുക", "धीरे गाड़ी चलाएँ और औज़ार सावधानी से इस्तेमाल करें"],
                 ["Haste is what to watch in this time; slowing down keeps you safe.", "ഈ കാലത്ത് ശ്രദ്ധിക്കേണ്ടത് ധൃതിയാണ്; വേഗം കുറച്ചാൽ സുരക്ഷിതരായിരിക്കാം.", "इस समय जल्दबाज़ी से बचना है; धीमा चलना आपको सुरक्षित रखता है।"],
                 ["Leave ten minutes early. Wear the helmet, the gloves, the seat belt.", "പത്ത് മിനിറ്റ് നേരത്തെ ഇറങ്ങുക. ഹെൽമെറ്റ്, കയ്യുറ, സീറ്റ് ബെൽറ്റ് ധരിക്കുക.", "दस मिनट पहले निकलें। हेलमेट, दस्ताने, सीट बेल्ट पहनें।"],
                 false, E_DAILY, None),
                ("giving",
                 ["Red lentils and rice for labourers or guards, served hot", "തൊഴിലാളികൾക്കോ കാവൽക്കാർക്കോ ചൂടുള്ള പരിപ്പും ചോറും", "मज़दूरों या चौकीदारों को गरम मसूर दाल और चावल"],
                 ["Cools anger and quarrels; helps land and property matters.", "ദേഷ്യവും വഴക്കും തണുപ്പിക്കും; ഭൂമി-വസ്തു കാര്യങ്ങൾക്ക് സഹായിക്കും.", "गुस्सा और झगड़े ठंडे करता है; ज़मीन-जायदाद के कामों में मदद करता है।"],
                 ["On Tuesday, serve a hot meal by your own hand to workers or security guards.", "ചൊവ്വാഴ്ച തൊഴിലാളികൾക്കോ സെക്യൂരിറ്റി ജീവനക്കാർക്കോ സ്വന്തം കൈകൊണ്ട് ചൂടുള്ള ഭക്ഷണം വിളമ്പുക.", "मंगलवार को मज़दूरों या सुरक्षाकर्मियों को अपने हाथ से गरम खाना परोसें।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Donate blood, or pay for a poor patient's medicine", "രക്തദാനം ചെയ്യുക, അല്ലെങ്കിൽ ഒരു പാവപ്പെട്ട രോഗിയുടെ മരുന്നിന് പണം കൊടുക്കുക", "रक्तदान करें, या किसी ग़रीब मरीज़ की दवा का ख़र्च दें"],
                 ["Giving blood is the most direct answer to a time of heat and haste.", "ചൂടിന്റെയും ധൃതിയുടെയും കാലത്തിനുള്ള ഏറ്റവും നേരിട്ടുള്ള മറുപടിയാണ് രക്തദാനം.", "गर्मी और जल्दबाज़ी के समय का सबसे सीधा जवाब रक्तदान है।"],
                 ["Find the nearest blood donation camp, or ask a hospital pharmacy who cannot pay.", "അടുത്തുള്ള രക്തദാന ക്യാമ്പ് കണ്ടെത്തുക, അല്ലെങ്കിൽ ആശുപത്രി ഫാർമസിയിൽ പണമില്ലാത്തവരെക്കുറിച്ച് ചോദിക്കുക.", "पास का रक्तदान शिविर ढूँढें, या अस्पताल की फ़ार्मेसी से पूछें कि कौन पैसे नहीं दे पा रहा।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Hard exercise three times a week", "ആഴ്ചയിൽ മൂന്ന് തവണ കഠിന വ്യായാമം", "हफ़्ते में तीन बार कड़ी कसरत"],
                 ["Heat that is spent in effort does not come out as anger.", "അധ്വാനത്തിൽ ചെലവാകുന്ന ചൂട് ദേഷ്യമായി പുറത്തുവരില്ല.", "जो गर्मी मेहनत में खप जाती है, वह गुस्से में नहीं निकलती।"],
                 ["Run, swim, lift or dig for 30 minutes until you sweat.", "വിയർക്കുംവരെ 30 മിനിറ്റ് ഓടുക, നീന്തുക, ഭാരം ഉയർത്തുക, അല്ലെങ്കിൽ കിളയ്ക്കുക.", "पसीना आने तक 30 मिनट दौड़ें, तैरें, वज़न उठाएँ या खुदाई करें।"],
                 false, E_3W, None),
                ("nature",
                 ["Plant a tree, or give water to animals in the heat", "ഒരു മരം നടുക, അല്ലെങ്കിൽ ചൂടിൽ മൃഗങ്ങൾക്ക് വെള്ളം കൊടുക്കുക", "पेड़ लगाएँ, या गर्मी में जानवरों को पानी दें"],
                 ["Cooling care for living things cools the heat in your own days.", "ജീവജാലങ്ങൾക്ക് കൊടുക്കുന്ന തണുപ്പ് നിങ്ങളുടെ ദിവസങ്ങളിലെ ചൂടും തണുപ്പിക്കും.", "जीवों को दी गई ठंडक आपके दिनों की गर्मी भी ठंडी करती है।"],
                 ["Keep a bowl of water outside for birds and dogs through the hot months.", "ചൂടുകാലം മുഴുവൻ പക്ഷികൾക്കും നായ്ക്കൾക്കും പുറത്ത് ഒരു പാത്രം വെള്ളം വയ്ക്കുക.", "गर्मी भर पक्षियों और कुत्तों के लिए बाहर पानी का कटोरा रखें।"],
                 false, E_DAILY, None),
                ("mind",
                 ["Count to ten before every reply when angry", "ദേഷ്യം വരുമ്പോൾ ഓരോ മറുപടിക്കും മുമ്പ് പത്ത് വരെ എണ്ണുക", "गुस्से में हर जवाब से पहले दस तक गिनें"],
                 ["The pause is where the quarrel ends.", "ആ ഇടവേളയിലാണ് വഴക്ക് അവസാനിക്കുന്നത്.", "उसी ठहराव में झगड़ा ख़त्म हो जाता है।"],
                 ["Practise it twice a day, even when calm, so it is there when you need it.", "ശാന്തമായിരിക്കുമ്പോഴും ദിവസം രണ്ടുതവണ പരിശീലിക്കുക, ആവശ്യമുള്ളപ്പോൾ അത് കൂടെയുണ്ടാകും.", "शांत रहते हुए भी दिन में दो बार अभ्यास करें, ताकि ज़रूरत पड़ने पर काम आए।"],
                 false, E_DAILY, None),
                ("tradition",
                 ["Tuesday fast: one simple meal, for 21 Tuesdays", "ചൊവ്വാഴ്ച വ്രതം: ലളിതമായ ഒരു നേരം, 21 ചൊവ്വാഴ്ച", "मंगलवार का व्रत: एक बार सादा भोजन, 21 मंगलवार"],
                 ["The classical fast for anger, debts and land disputes.", "ദേഷ്യം, കടം, ഭൂമിതർക്കം എന്നിവയ്ക്കുള്ള പരമ്പരാഗത വ്രതം.", "गुस्से, कर्ज़ और ज़मीन के झगड़ों के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Hanuman Chalisa or Skanda Shashti Kavacham on Tuesdays, or the chant below", "ചൊവ്വാഴ്ചകളിൽ ഹനുമാൻ ചാലീസയോ സ്കന്ദ ഷഷ്ഠി കവചമോ, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "मंगलवार को हनुमान चालीसा या स्कंद षष्ठी कवच, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om kraam kreem kraum sah Bhaumaya namah", "ഓം ക്രാം ക്രീം ക്രൗം സഃ ഭൗമായ നമഃ", "ॐ क्रां क्रीं क्रौं सः भौमाय नमः"])),
                ("tradition",
                 ["A Subrahmanya or Hanuman temple on Tuesday", "ചൊവ്വാഴ്ച സുബ്രഹ്മണ്യ ക്ഷേത്രത്തിലോ ഹനുമാൻ ക്ഷേത്രത്തിലോ", "मंगलवार को सुब्रह्मण्य या हनुमान मंदिर"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Mercury => Influence {
            effects: [
                "Things get mixed up: messages misread, papers lost, accounts that do not add up. The mind runs in circles and decisions wobble.",
                "കാര്യങ്ങൾ കൂടിക്കുഴയും: സന്ദേശങ്ങൾ തെറ്റിദ്ധരിക്കപ്പെടും, കടലാസുകൾ നഷ്ടപ്പെടും, കണക്കുകൾ ഒക്കില്ല. മനസ്സ് വട്ടംചുറ്റും, തീരുമാനങ്ങൾ ആടിയുലയും.",
                "बातें उलझती हैं: संदेश ग़लत समझे जाते हैं, कागज़ खोते हैं, हिसाब नहीं मिलता। मन घूमता रहता है और फ़ैसले डगमगाते हैं।",
            ],
            people_en: "students, young people and small traders",
            people_ml_for: "വിദ്യാർത്ഥികൾക്കും യുവാക്കൾക്കും ചെറുകിട കച്ചവടക്കാർക്കും",
            people_hi_for: "विद्यार्थियों, युवाओं और छोटे व्यापारियों",
            items: vec![
                ("deeds",
                 ["Double-check every paper before you sign or send", "ഒപ്പിടുന്നതിനോ അയയ്ക്കുന്നതിനോ മുമ്പ് ഓരോ കടലാസും രണ്ടുതവണ പരിശോധിക്കുക", "दस्तख़त करने या भेजने से पहले हर कागज़ दो बार जाँचें"],
                 ["Mix-ups are the mark of this time; one careful look prevents most of them.", "പിഴവുകളാണ് ഈ കാലത്തിന്റെ അടയാളം; ഒരു ശ്രദ്ധയുള്ള നോട്ടം മിക്കതും തടയും.", "गड़बड़ियाँ इस समय की पहचान हैं; एक सावधान नज़र ज़्यादातर रोक देती है।"],
                 ["Read it twice, once aloud. Keep copies of everything.", "രണ്ടുതവണ വായിക്കുക, ഒരിക്കൽ ഉറക്കെ. എല്ലാറ്റിന്റെയും പകർപ്പ് സൂക്ഷിക്കുക.", "दो बार पढ़ें, एक बार ज़ोर से। हर चीज़ की कॉपी रखें।"],
                 false, E_DAILY, None),
                ("deeds",
                 ["Speak less, and only what is true, for a week", "ഒരാഴ്ച കുറച്ച് സംസാരിക്കുക, സത്യം മാത്രം പറയുക", "एक हफ़्ता कम बोलें, और सिर्फ़ सच"],
                 ["Careless words cause the trouble now; fewer, truer words undo it.", "അശ്രദ്ധമായ വാക്കുകളാണ് ഇപ്പോൾ പ്രശ്നം ഉണ്ടാക്കുന്നത്; കുറഞ്ഞ, സത്യമുള്ള വാക്കുകൾ അത് മാറ്റും.", "लापरवाह शब्द अभी परेशानी लाते हैं; कम और सच्चे शब्द उसे सुलझाते हैं।"],
                 ["No gossip, no promises you cannot keep, no white lies, for seven days.", "ഏഴ് ദിവസം: പരദൂഷണമില്ല, പാലിക്കാനാവാത്ത വാഗ്ദാനങ്ങളില്ല, ചെറിയ നുണകൾ പോലുമില്ല.", "सात दिन: न चुग़ली, न ऐसे वादे जो निभ न सकें, न छोटे झूठ।"],
                 false, E_WEEK, None),
                ("deeds",
                 ["Teach a child or a student something you know well", "നിങ്ങൾക്ക് നന്നായി അറിയാവുന്ന എന്തെങ്കിലും ഒരു കുട്ടിയെയോ വിദ്യാർത്ഥിയെയോ പഠിപ്പിക്കുക", "किसी बच्चे या विद्यार्थी को वह सिखाएँ जो आप अच्छे से जानते हैं"],
                 ["Sharing knowledge clears your own thinking and brings learning and trade back on track.", "അറിവ് പങ്കിടുന്നത് നിങ്ങളുടെ ചിന്ത തെളിയിക്കും, പഠനവും കച്ചവടവും വീണ്ടും വഴിക്കാകും.", "ज्ञान बाँटने से आपकी सोच साफ़ होती है और पढ़ाई व व्यापार पटरी पर लौटते हैं।"],
                 ["One hour a week: homework help, a skill, a language.", "ആഴ്ചയിൽ ഒരു മണിക്കൂർ: ഗൃഹപാഠസഹായം, ഒരു കഴിവ്, ഒരു ഭാഷ.", "हफ़्ते में एक घंटा: होमवर्क में मदद, कोई हुनर, कोई भाषा।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Give books, pens or school fees to a student who cannot afford them", "പണമില്ലാത്ത ഒരു വിദ്യാർത്ഥിക്ക് പുസ്തകമോ പേനയോ സ്കൂൾ ഫീസോ കൊടുക്കുക", "जो विद्यार्थी ख़र्च नहीं उठा सकता, उसे किताबें, क़लम या स्कूल फ़ीस दें"],
                 ["Eases mix-ups in study, paperwork and speech.", "പഠനത്തിലും കടലാസുകളിലും സംസാരത്തിലുമുള്ള പിഴവുകൾ കുറയ്ക്കും.", "पढ़ाई, कागज़ी काम और बातचीत की गड़बड़ियाँ कम करता है।"],
                 ["Ask a school teacher which child needs it most.", "ഏറ്റവും ആവശ്യമുള്ള കുട്ടി ആരാണെന്ന് ഒരു സ്കൂൾ അധ്യാപകനോട് ചോദിക്കുക.", "किसी स्कूल शिक्षक से पूछें कि किस बच्चे को सबसे ज़्यादा ज़रूरत है।"],
                 false, E_ONCE, None),
                ("giving",
                 ["Green gram or green vegetables to a poor family on Wednesday", "ബുധനാഴ്ച ഒരു പാവപ്പെട്ട കുടുംബത്തിന് ചെറുപയറോ പച്ചക്കറിയോ", "बुधवार को किसी ग़रीब परिवार को मूँग या हरी सब्ज़ियाँ"],
                 ["The traditional gift for clear thinking and smooth dealings.", "തെളിഞ്ഞ ചിന്തയ്ക്കും സുഗമമായ ഇടപാടുകൾക്കുമുള്ള പരമ്പരാഗത ദാനം.", "साफ़ सोच और सहज लेन-देन के लिए पारंपरिक दान।"],
                 ["Hand it over yourself, with a kind word.", "ഒരു നല്ല വാക്കോടെ സ്വയം കൈമാറുക.", "एक अच्छे शब्द के साथ ख़ुद दें।"],
                 true, E_WEEKLY, None),
                ("habits",
                 ["One task at a time, written down each morning", "ഒരു സമയം ഒരു ജോലി, എല്ലാ രാവിലെയും എഴുതിവച്ച്", "एक समय में एक काम, हर सुबह लिखकर"],
                 ["Scattered effort is the trap now; a short list keeps the mind on track.", "ചിതറിയ ശ്രമമാണ് ഇപ്പോഴത്തെ കെണി; ഒരു ചെറിയ പട്ടിക മനസ്സിനെ വഴിയിൽ നിർത്തും.", "बिखरी मेहनत अभी जाल है; छोटी सूची मन को पटरी पर रखती है।"],
                 ["Write three tasks before you start. Finish the first before touching the second.", "തുടങ്ങുന്നതിന് മുമ്പ് മൂന്ന് ജോലികൾ എഴുതുക. ആദ്യത്തേത് തീർത്തിട്ടേ രണ്ടാമത്തേത് തൊടാവൂ.", "शुरू करने से पहले तीन काम लिखें। पहला पूरा किए बिना दूसरा न छुएँ।"],
                 false, E_DAILY, None),
                ("nature",
                 ["Tend a tulasi or any green plant daily", "ദിവസവും ഒരു തുളസിയെയോ ഏതെങ്കിലും പച്ചച്ചെടിയെയോ പരിപാലിക്കുക", "रोज़ तुलसी या किसी हरे पौधे की देखभाल करें"],
                 ["Daily care of something green steadies a busy, scattered mind.", "പച്ചയായ ഒന്നിന്റെ ദിവസേനയുള്ള പരിചരണം തിരക്കുള്ള, ചിതറിയ മനസ്സിനെ ഉറപ്പിക്കും.", "किसी हरी चीज़ की रोज़ की देखभाल व्यस्त, बिखरे मन को सँभालती है।"],
                 ["Water it, pick off dry leaves, and spend one quiet minute with it.", "നനയ്ക്കുക, ഉണങ്ങിയ ഇലകൾ നുള്ളിക്കളയുക, ഒരു മിനിറ്റ് അതിനരികിൽ ശാന്തമായിരിക്കുക.", "पानी दें, सूखे पत्ते तोड़ें, और एक मिनट चुपचाप उसके पास रहें।"],
                 false, E_DAILY, None),
                ("mind",
                 ["Write your worries down before bed", "ഉറങ്ങുന്നതിന് മുമ്പ് ആശങ്കകൾ എഴുതിവയ്ക്കുക", "सोने से पहले चिंताएँ लिख दें"],
                 ["A mind that loops lets go once the thoughts are on paper.", "വട്ടംചുറ്റുന്ന മനസ്സ് ചിന്തകൾ കടലാസിലായാൽ അവയെ വിടും.", "घूमता हुआ मन विचार कागज़ पर आते ही उन्हें छोड़ देता है।"],
                 ["Three lines: what worries you, what you can do tomorrow, what you cannot control.", "മൂന്ന് വരി: എന്താണ് ആശങ്ക, നാളെ എന്ത് ചെയ്യാം, എന്താണ് നിങ്ങളുടെ നിയന്ത്രണത്തിലല്ലാത്തത്.", "तीन पंक्तियाँ: क्या चिंता है, कल क्या कर सकते हैं, क्या आपके बस में नहीं।"],
                 false, E_10, None),
                ("tradition",
                 ["Wednesday fast for 21 Wednesdays", "21 ബുധനാഴ്ച വ്രതം", "21 बुधवार का व्रत"],
                 ["The classical fast for learning, speech and trade.", "പഠനം, സംസാരം, കച്ചവടം എന്നിവയ്ക്കുള്ള പരമ്പരാഗത വ്രതം.", "पढ़ाई, वाणी और व्यापार के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Vishnu Sahasranama on Wednesdays, or the chant below", "ബുധനാഴ്ചകളിൽ വിഷ്ണു സഹസ്രനാമം, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "बुधवार को विष्णु सहस्रनाम, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om braam breem braum sah Budhaya namah", "ഓം ബ്രാം ബ്രീം ബ്രൗം സഃ ബുധായ നമഃ", "ॐ ब्रां ब्रीं ब्रौं सः बुधाय नमः"])),
                ("tradition",
                 ["A Vishnu temple on Wednesday", "ബുധനാഴ്ച വിഷ്ണുക്ഷേത്രദർശനം", "बुधवार को विष्णु मंदिर"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Jupiter => Influence {
            effects: [
                "Good faith is misplaced: promises run ahead of what can be kept, and money leaks through generosity or trust. Advice you get may be poor.",
                "വിശ്വാസം തെറ്റായ ഇടത്തേക്ക് പോകും: പാലിക്കാവുന്നതിനേക്കാൾ വാഗ്ദാനം ചെയ്യും, ഔദാര്യത്തിലൂടെയോ വിശ്വാസത്തിലൂടെയോ പണം ചോരും. കിട്ടുന്ന ഉപദേശം മോശമായേക്കാം.",
                "भरोसा ग़लत जगह जाता है: निभाने से ज़्यादा वादे होते हैं, और उदारता या भरोसे में पैसा रिसता है। मिलने वाली सलाह कमज़ोर हो सकती है।",
            ],
            people_en: "teachers, children and those who guide others",
            people_ml_for: "അധ്യാപകർക്കും കുട്ടികൾക്കും മറ്റുള്ളവരെ നയിക്കുന്നവർക്കും",
            people_hi_for: "शिक्षकों, बच्चों और राह दिखाने वालों",
            items: vec![
                ("deeds",
                 ["Promise less; keep all of it", "കുറച്ച് വാഗ്ദാനം ചെയ്യുക; മുഴുവൻ പാലിക്കുക", "कम वादा करें; पूरा निभाएँ"],
                 ["Over-promising is the leak now; saying only what you can do restores trust and money.", "അമിതവാഗ്ദാനമാണ് ഇപ്പോഴത്തെ ചോർച്ച; കഴിയുന്നത് മാത്രം പറഞ്ഞാൽ വിശ്വാസവും പണവും തിരിച്ചുവരും.", "ज़्यादा वादे अभी रिसाव हैं; उतना ही कहना जितना कर सकें, भरोसा और पैसा लौटाता है।"],
                 ["Before every yes this month, ask whether it can really be done, on time.", "ഈ മാസം ഓരോ സമ്മതത്തിനും മുമ്പ്, അത് ശരിക്കും സമയത്ത് ചെയ്യാനാകുമോ എന്ന് സ്വയം ചോദിക്കുക.", "इस महीने हर 'हाँ' से पहले पूछें कि क्या यह सच में समय पर हो पाएगा।"],
                 false, E_MONTH, None),
                ("deeds",
                 ["Honour a teacher who shaped you", "നിങ്ങളെ രൂപപ്പെടുത്തിയ ഒരു അധ്യാപകനെ ആദരിക്കുക", "जिस शिक्षक ने आपको गढ़ा, उनका सम्मान करें"],
                 ["Respect to those who guided you brings good advice and luck back into your life.", "നിങ്ങളെ നയിച്ചവരോടുള്ള ബഹുമാനം നല്ല ഉപദേശവും ഭാഗ്യവും ജീവിതത്തിലേക്ക് തിരികെ കൊണ്ടുവരും.", "राह दिखाने वालों का आदर अच्छी सलाह और किस्मत को ज़िंदगी में लौटाता है।"],
                 ["Visit, call or write to them this Thursday. Thank them for one specific thing.", "ഈ വ്യാഴാഴ്ച അവരെ കാണുക, വിളിക്കുക, അല്ലെങ്കിൽ എഴുതുക. ഒരു പ്രത്യേക കാര്യത്തിന് നന്ദി പറയുക.", "इस गुरुवार उनसे मिलें, फ़ोन करें या लिखें। किसी एक ख़ास बात के लिए धन्यवाद दें।"],
                 true, E_ONCE, None),
                ("deeds",
                 ["Check before you lend, invest or trust", "കടം കൊടുക്കുന്നതിനോ നിക്ഷേപിക്കുന്നതിനോ വിശ്വസിക്കുന്നതിനോ മുമ്പ് പരിശോധിക്കുക", "उधार देने, निवेश करने या भरोसा करने से पहले जाँचें"],
                 ["Money leaks through good faith now; one question saves a lot.", "ഇപ്പോൾ നല്ല വിശ്വാസത്തിലൂടെ പണം ചോരും; ഒരു ചോദ്യം ഏറെ രക്ഷിക്കും.", "अभी भले भरोसे में पैसा रिसता है; एक सवाल बहुत बचाता है।"],
                 ["Ask for it in writing. Sleep on every money decision for one night.", "എഴുതി വാങ്ങുക. ഓരോ പണതീരുമാനവും ഒരു രാത്രി കഴിഞ്ഞ് മാത്രം എടുക്കുക.", "लिखित में लें। पैसे का हर फ़ैसला एक रात सोचकर करें।"],
                 false, E_DAILY, None),
                ("giving",
                 ["Pay for a child's schooling or books", "ഒരു കുട്ടിയുടെ പഠനത്തിനോ പുസ്തകങ്ങൾക്കോ പണം കൊടുക്കുക", "किसी बच्चे की पढ़ाई या किताबों का ख़र्च दें"],
                 ["Children served bring luck, growth and the joy of your own children.", "കുട്ടികളെ സേവിക്കുന്നത് ഭാഗ്യവും വളർച്ചയും നിങ്ങളുടെ മക്കളുടെ സന്തോഷവും കൊണ്ടുവരും.", "बच्चों की सेवा किस्मत, तरक्की और अपने बच्चों की ख़ुशी लाती है।"],
                 ["One term's fees, or a full set of books, through the school.", "സ്കൂൾ വഴി ഒരു ടേമിന്റെ ഫീസ്, അല്ലെങ്കിൽ ഒരു സെറ്റ് പുസ്തകങ്ങൾ.", "स्कूल के ज़रिए एक सत्र की फ़ीस, या किताबों का पूरा सेट।"],
                 false, E_ONCE, None),
                ("giving",
                 ["Yellow food for the needy on Thursday: chana dal, bananas, turmeric rice", "വ്യാഴാഴ്ച ആവശ്യക്കാർക്ക് മഞ്ഞ ഭക്ഷണം: കടലപ്പരിപ്പ്, പഴം, മഞ്ഞൾച്ചോറ്", "गुरुवार को ज़रूरतमंदों को पीला भोजन: चना दाल, केले, हल्दी चावल"],
                 ["The traditional gift for luck and good counsel.", "ഭാഗ്യത്തിനും നല്ല ഉപദേശത്തിനുമുള്ള പരമ്പരാഗത ദാനം.", "किस्मत और अच्छी सलाह के लिए पारंपरिक दान।"],
                 ["Serve it at a shelter, an old-age home or the kitchen of a place of worship.", "ഒരു അഭയകേന്ദ്രത്തിലോ വൃദ്ധസദനത്തിലോ ആരാധനാലയത്തിലെ അടുക്കളയിലോ വിളമ്പുക.", "किसी आश्रय, वृद्धाश्रम या किसी पूजा-स्थल की रसोई में परोसें।"],
                 true, E_WEEKLY, None),
                ("habits",
                 ["No sweets, no excess, for 40 days", "40 ദിവസം മധുരവും അമിതത്വവും ഇല്ലാതെ", "40 दिन तक न मिठाई, न ज़्यादती"],
                 ["Too much is the trouble now, in food, in spending, in promises; restraint in one teaches the others.", "അമിതത്വമാണ് ഇപ്പോഴത്തെ പ്രശ്നം, ഭക്ഷണത്തിൽ, ചെലവിൽ, വാഗ്ദാനത്തിൽ; ഒന്നിലെ നിയന്ത്രണം മറ്റുള്ളവയെയും പഠിപ്പിക്കും.", "ज़्यादती ही अभी परेशानी है, खाने में, खर्च में, वादों में; एक में संयम बाकी सिखा देता है।"],
                 ["Pick one excess to cut. Note each day you kept it.", "കുറയ്ക്കാൻ ഒരു അമിതത്വം തിരഞ്ഞെടുക്കുക. പാലിച്ച ഓരോ ദിവസവും കുറിക്കുക.", "घटाने के लिए एक ज़्यादती चुनें। जिस दिन निभाएँ, लिख लें।"],
                 false, E_40, None),
                ("nature",
                 ["Water a banana plant or a peepal on Thursday", "വ്യാഴാഴ്ച ഒരു വാഴയ്ക്കോ അരയാലിനോ വെള്ളമൊഴിക്കുക", "गुरुवार को केले के पेड़ या पीपल को पानी दें"],
                 ["Patient care for a giving tree invites growth into your own life.", "ഫലം തരുന്ന മരത്തിന്റെ ക്ഷമയുള്ള പരിചരണം നിങ്ങളുടെ ജീവിതത്തിലേക്ക് വളർച്ച ക്ഷണിക്കും.", "देने वाले पेड़ की धैर्य से देखभाल आपकी ज़िंदगी में तरक्की बुलाती है।"],
                 ["Same tree, every Thursday, for as long as this time lasts.", "ഈ കാലം നീളുന്നിടത്തോളം, എല്ലാ വ്യാഴാഴ്ചയും ഒരേ മരത്തിന്.", "जब तक यह समय चले, हर गुरुवार उसी पेड़ को।"],
                 true, E_WEEKLY, None),
                ("mind",
                 ["End each day with three things you are grateful for", "ഓരോ ദിവസവും നന്ദിയുള്ള മൂന്ന് കാര്യങ്ങളോടെ അവസാനിപ്പിക്കുക", "हर दिन तीन ऐसी बातों के साथ ख़त्म करें जिनके लिए आप आभारी हैं"],
                 ["Gratitude settles the wanting that drives over-spending and over-promising.", "അമിതചെലവിനും അമിതവാഗ്ദാനത്തിനും പിന്നിലെ ആർത്തിയെ നന്ദി ശമിപ്പിക്കും.", "आभार उस चाह को शांत करता है जो ज़्यादा खर्च और ज़्यादा वादे करवाती है।"],
                 ["Write them in a notebook by your bed; read the week's list on Thursday.", "കിടക്കയ്ക്കരികിലെ നോട്ടുബുക്കിൽ എഴുതുക; വ്യാഴാഴ്ച ആഴ്ചയിലെ പട്ടിക വായിക്കുക.", "बिस्तर के पास की कॉपी में लिखें; गुरुवार को पूरे हफ़्ते की सूची पढ़ें।"],
                 false, E_10, None),
                ("tradition",
                 ["Thursday fast for 16 Thursdays: one simple, yellow meal", "16 വ്യാഴാഴ്ച വ്രതം: ലളിതമായ ഒരു നേരം, മഞ്ഞ ഭക്ഷണം", "16 गुरुवार का व्रत: एक बार सादा, पीला भोजन"],
                 ["The classical fast for luck, children and wise counsel.", "ഭാഗ്യത്തിനും മക്കൾക്കും നല്ല ഉപദേശത്തിനുമുള്ള പരമ്പരാഗത വ്രതം.", "किस्मत, संतान और अच्छी सलाह के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Dakshinamurthy stotram on Thursdays, or the chant below", "വ്യാഴാഴ്ചകളിൽ ദക്ഷിണാമൂർത്തി സ്തോത്രം, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "गुरुवार को दक्षिणामूर्ति स्तोत्र, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om graam greem graum sah Gurave namah", "ഓം ഗ്രാം ഗ്രീം ഗ്രൗം സഃ ഗുരവേ നമഃ", "ॐ ग्रां ग्रीं ग्रौं सः गुरवे नमः"])),
                ("tradition",
                 ["A Dakshinamurthy or Vishnu temple on Thursday", "വ്യാഴാഴ്ച ദക്ഷിണാമൂർത്തി അല്ലെങ്കിൽ വിഷ്ണു ക്ഷേത്രം", "गुरुवार को दक्षिणामूर्ति या विष्णु मंदिर"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Venus => Influence {
            effects: [
                "Comfort pulls hard: spending on pleasure, jealousy or friction with your partner. Relationships feel demanding and money goes on wants.",
                "സുഖത്തിലേക്കുള്ള ആകർഷണം ശക്തമാണ്: ആനന്ദത്തിനായുള്ള ചെലവ്, അസൂയ, പങ്കാളിയുമായുള്ള ഉരസൽ. ബന്ധങ്ങൾ ഏറെ ആവശ്യപ്പെടുന്നതായി തോന്നും, പണം ആഗ്രഹങ്ങൾക്കായി പോകും.",
                "सुख की खींच ज़्यादा है: मौज पर खर्च, जलन या जीवनसाथी से खटपट। रिश्ते भारी लगते हैं और पैसा शौक़ों पर जाता है।",
            ],
            people_en: "your partner, women and those who lack comfort",
            people_ml_for: "നിങ്ങളുടെ ജീവിതപങ്കാളിക്കും സ്ത്രീകൾക്കും സുഖമില്ലാതെ കഴിയുന്നവർക്കും",
            people_hi_for: "अपने जीवनसाथी, महिलाओं और अभाव में जीने वालों",
            items: vec![
                ("deeds",
                 ["Give your partner your full attention, an hour a week", "ആഴ്ചയിൽ ഒരു മണിക്കൂർ പങ്കാളിക്ക് മുഴുവൻ ശ്രദ്ധയും കൊടുക്കുക", "हफ़्ते में एक घंटा जीवनसाथी को पूरा ध्यान दें"],
                 ["Friction with a partner eases when they feel seen; attention costs nothing and mends a lot.", "തന്നെ കാണുന്നുണ്ടെന്ന് തോന്നുമ്പോൾ പങ്കാളിയുമായുള്ള ഉരസൽ കുറയും; ശ്രദ്ധയ്ക്ക് ചെലവില്ല, പക്ഷേ അത് ഏറെ നന്നാക്കും.", "जब साथी को लगता है कि उसे देखा जा रहा है, तो खटपट कम होती है; ध्यान मुफ़्त है और बहुत कुछ जोड़ता है।"],
                 ["A walk or a meal, phones away. Ask about their week and just listen.", "ഫോണുകൾ മാറ്റിവച്ച് ഒരു നടത്തമോ ഭക്ഷണമോ. അവരുടെ ആഴ്ചയെക്കുറിച്ച് ചോദിച്ച് കേൾക്കുക മാത്രം ചെയ്യുക.", "फ़ोन दूर रखकर टहलना या खाना। उनके हफ़्ते के बारे में पूछें और बस सुनें।"],
                 true, E_WEEKLY, None),
                ("deeds",
                 ["Respect every woman you deal with, in word and deed", "ഇടപെടുന്ന ഓരോ സ്ത്രീയെയും വാക്കിലും പ്രവൃത്തിയിലും ബഹുമാനിക്കുക", "जिन महिलाओं से वास्ता पड़ता है, उनका शब्द और काम से सम्मान करें"],
                 ["Disrespect to women is the surest way to sour this time; respect sweetens it.", "സ്ത്രീകളോടുള്ള അനാദരവ് ഈ കാലത്തെ കയ്പ്പാക്കും; ബഹുമാനം അതിനെ മധുരമാക്കും.", "महिलाओं का अनादर इस समय को कड़वा करने का सबसे पक्का तरीका है; सम्मान इसे मीठा करता है।"],
                 ["Notice how you speak to the women at home and at work this week, and correct one habit.", "ഈ ആഴ്ച വീട്ടിലും ജോലിസ്ഥലത്തും സ്ത്രീകളോട് നിങ്ങൾ സംസാരിക്കുന്ന രീതി ശ്രദ്ധിക്കുക, ഒരു ശീലം തിരുത്തുക.", "इस हफ़्ते देखें कि घर और काम पर आप महिलाओं से कैसे बात करते हैं, और एक आदत सुधारें।"],
                 false, E_WEEK, None),
                ("deeds",
                 ["Cut one indulgence for 30 days", "30 ദിവസം ഒരു ആഡംബരം ഒഴിവാക്കുക", "30 दिन के लिए एक शौक़ छोड़ें"],
                 ["Spending on wants is the leak now; one month of restraint resets the pull.", "ആഗ്രഹങ്ങൾക്കായുള്ള ചെലവാണ് ഇപ്പോഴത്തെ ചോർച്ച; ഒരു മാസത്തെ നിയന്ത്രണം ആ വലിവിനെ ശരിയാക്കും.", "शौक़ों पर ख़र्च अभी रिसाव है; एक महीने का संयम उस खिंचाव को ठीक करता है।"],
                 ["Pick one: eating out, shopping online, sweets. Put the money saved aside.", "ഒന്ന് തിരഞ്ഞെടുക്കുക: പുറത്ത് ഭക്ഷണം, ഓൺലൈൻ ഷോപ്പിംഗ്, മധുരം. ലാഭിക്കുന്ന പണം മാറ്റിവയ്ക്കുക.", "एक चुनें: बाहर खाना, ऑनलाइन ख़रीदारी, मिठाई। बचा पैसा अलग रखें।"],
                 false, E_30, None),
                ("giving",
                 ["Rice and curd, or white sweets, to women in need on Friday", "വെള്ളിയാഴ്ച ആവശ്യമുള്ള സ്ത്രീകൾക്ക് തൈർസാദമോ വെള്ള മധുരമോ", "शुक्रवार को ज़रूरतमंद महिलाओं को दही-चावल या सफ़ेद मिठाई"],
                 ["Brings harmony at home and eases trouble in love.", "വീട്ടിൽ ഐക്യം കൊണ്ടുവരും, പ്രണയത്തിലെ പ്രശ്നങ്ങൾ കുറയ്ക്കും.", "घर में मेलजोल लाता है और प्रेम की परेशानी घटाता है।"],
                 ["Give at a women's shelter or to a woman you know who lives alone, by your own hand.", "ഒരു വനിതാ അഭയകേന്ദ്രത്തിലോ നിങ്ങൾക്കറിയാവുന്ന, ഒറ്റയ്ക്ക് ജീവിക്കുന്ന ഒരു സ്ത്രീക്കോ സ്വന്തം കൈകൊണ്ട് കൊടുക്കുക.", "किसी महिला आश्रय में या किसी परिचित अकेली रहने वाली महिला को अपने हाथ से दें।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Help a poor bride, or give comforts to a family that lacks them", "ഒരു പാവപ്പെട്ട വധുവിനെ സഹായിക്കുക, അല്ലെങ്കിൽ ഇല്ലാത്ത ഒരു കുടുംബത്തിന് സൗകര്യങ്ങൾ കൊടുക്കുക", "किसी ग़रीब दुल्हन की मदद करें, या अभाव वाले परिवार को सुविधाएँ दें"],
                 ["Giving comfort to those without it steadies your own pull towards it.", "ഇല്ലാത്തവർക്ക് സുഖം കൊടുക്കുന്നത് അതിലേക്കുള്ള നിങ്ങളുടെ വലിവിനെ നിയന്ത്രിക്കും.", "जिनके पास सुख नहीं, उन्हें सुख देने से आपका अपना खिंचाव सँभलता है।"],
                 ["New clothes, a mattress, a fan: something that makes a home a little easier.", "പുതുവസ്ത്രം, ഒരു മെത്ത, ഒരു ഫാൻ: ഒരു വീടിനെ അൽപം എളുപ്പമാക്കുന്ന എന്തെങ്കിലും.", "नए कपड़े, एक गद्दा, एक पंखा: कुछ ऐसा जो घर को थोड़ा आसान बना दे।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Keep yourself and your home clean and simple", "നിങ്ങളെയും വീടിനെയും വൃത്തിയായും ലളിതമായും സൂക്ഷിക്കുക", "ख़ुद को और घर को साफ़ और सादा रखें"],
                 ["Order and simple beauty satisfy the pull towards comfort without spending.", "ക്രമവും ലളിതമായ ഭംഗിയും ചെലവില്ലാതെ സുഖത്തിനായുള്ള ആഗ്രഹം തൃപ്തിപ്പെടുത്തും.", "व्यवस्था और सादी सुंदरता बिना ख़र्च के सुख की चाह पूरी करती है।"],
                 ["Clear one cupboard or corner each Friday; keep fresh flowers or a clean cloth on the table.", "എല്ലാ വെള്ളിയാഴ്ചയും ഒരു അലമാരയോ ഒരു മൂലയോ വൃത്തിയാക്കുക; മേശപ്പുറത്ത് പുതിയ പൂക്കളോ വൃത്തിയുള്ള തുണിയോ വയ്ക്കുക.", "हर शुक्रवार एक अलमारी या कोना साफ़ करें; मेज़ पर ताज़े फूल या साफ़ कपड़ा रखें।"],
                 true, E_WEEKLY, None),
                ("nature",
                 ["Feed a cow or birds on Friday; grow a flowering plant", "വെള്ളിയാഴ്ച പശുവിനോ പക്ഷികൾക്കോ ഭക്ഷണം കൊടുക്കുക; ഒരു പൂച്ചെടി വളർത്തുക", "शुक्रवार को गाय या पक्षियों को खिलाएँ; एक फूलों वाला पौधा लगाएँ"],
                 ["Gentle care of living beauty brings harmony home.", "ജീവനുള്ള ഭംഗിയുടെ സൗമ്യമായ പരിചരണം വീട്ടിൽ ഐക്യം കൊണ്ടുവരും.", "जीवित सुंदरता की कोमल देखभाल घर में मेलजोल लाती है।"],
                 ["Rice for the birds at dawn; jasmine or rose in a pot by the door.", "രാവിലെ പക്ഷികൾക്ക് അരി; വാതിലിനരികിൽ ചട്ടിയിൽ മുല്ലയോ റോസോ.", "सुबह पक्षियों को चावल; दरवाज़े के पास गमले में चमेली या गुलाब।"],
                 true, E_WEEKLY, None),
                ("mind",
                 ["Ten minutes of music or art you love, with no screen", "സ്ക്രീനില്ലാതെ, ഇഷ്ടമുള്ള സംഗീതത്തിനോ കലയ്ക്കോ പത്ത് മിനിറ്റ്", "बिना स्क्रीन के, पसंदीदा संगीत या कला के दस मिनट"],
                 ["Beauty taken in quietly fills the need that otherwise drives spending.", "ശാന്തമായി ആസ്വദിക്കുന്ന ഭംഗി, അല്ലെങ്കിൽ ചെലവിലേക്ക് നയിക്കുന്ന ആവശ്യം നിറയ്ക്കും.", "चुपचाप ली गई सुंदरता वह कमी भरती है जो वरना ख़र्च करवाती है।"],
                 ["Sing, draw, play, or just listen with eyes closed.", "പാടുക, വരയ്ക്കുക, വായിക്കുക, അല്ലെങ്കിൽ കണ്ണടച്ച് കേൾക്കുക മാത്രം.", "गाएँ, चित्र बनाएँ, बजाएँ, या बस आँखें बंद करके सुनें।"],
                 false, E_10, None),
                ("tradition",
                 ["Friday fast for 21 Fridays: one simple, white meal", "21 വെള്ളിയാഴ്ച വ്രതം: ലളിതമായ ഒരു നേരം, വെള്ള ഭക്ഷണം", "21 शुक्रवार का व्रत: एक बार सादा, सफ़ेद भोजन"],
                 ["The classical fast for love, marriage and comfort.", "പ്രണയം, വിവാഹം, സുഖം എന്നിവയ്ക്കുള്ള പരമ്പരാഗത വ്രതം.", "प्रेम, विवाह और सुख के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Lakshmi Ashtottaram on Fridays, or the chant below", "വെള്ളിയാഴ്ചകളിൽ ലക്ഷ്മി അഷ്ടോത്തരം, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "शुक्रवार को लक्ष्मी अष्टोत्तरम्, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om draam dreem draum sah Shukraya namah", "ഓം ദ്രാം ദ്രീം ദ്രൗം സഃ ശുക്രായ നമഃ", "ॐ द्रां द्रीं द्रौं सः शुक्राय नमः"])),
                ("tradition",
                 ["A Lakshmi or Devi temple on Friday", "വെള്ളിയാഴ്ച ലക്ഷ്മി അല്ലെങ്കിൽ ദേവീക്ഷേത്രം", "शुक्रवार को लक्ष्मी या देवी मंदिर"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Saturn => Influence {
            effects: [
                "Everything takes longer: work piles up, results come late and you feel tired or alone. Debts and duties weigh more than usual.",
                "എല്ലാത്തിനും കൂടുതൽ സമയമെടുക്കും: ജോലി കുമിഞ്ഞുകൂടും, ഫലം വൈകും, ക്ഷീണമോ ഒറ്റപ്പെടലോ തോന്നും. കടവും കടമകളും പതിവിലും ഭാരമായി തോന്നും.",
                "हर काम में ज़्यादा समय लगता है: काम का ढेर, देर से फल, और थकान या अकेलापन। कर्ज़ और ज़िम्मेदारियाँ सामान्य से भारी लगती हैं।",
            ],
            people_en: "workers, the old and the disabled",
            people_ml_for: "തൊഴിലാളികൾക്കും വയോധികർക്കും ഭിന്നശേഷിക്കാർക്കും",
            people_hi_for: "मज़दूरों, बुज़ुर्गों और दिव्यांगों",
            items: vec![
                ("deeds",
                 ["Pay every worker on time, in full", "ഓരോ തൊഴിലാളിക്കും കൂലി കൃത്യസമയത്ത്, മുഴുവനായി കൊടുക്കുക", "हर कामगार को समय पर, पूरा पैसा दें"],
                 ["Unpaid or late wages invite delays and losses; paying fully clears the way for your own pay and progress.", "കൊടുക്കാതെയോ വൈകിയോ ഉള്ള കൂലി താമസവും നഷ്ടവും വരുത്തും; മുഴുവൻ കൊടുക്കുമ്പോൾ നിങ്ങളുടെ സ്വന്തം വരുമാനത്തിനും പുരോഗതിക്കും വഴി തെളിയും.", "बकाया या देर से दी गई मज़दूरी देरी और नुकसान लाती है; पूरा चुकाने से आपकी अपनी कमाई और तरक्की का रास्ता साफ़ होता है।"],
                 ["List everyone who works for you: maid, driver, mason, staff. Clear every due this week, with a word of thanks.", "നിങ്ങൾക്കുവേണ്ടി പണിയെടുക്കുന്ന എല്ലാവരുടെയും പേര് എഴുതുക: വീട്ടുജോലിക്കാരി, ഡ്രൈവർ, മേസ്തിരി, ജീവനക്കാർ. ഈ ആഴ്ച തന്നെ ബാക്കിയെല്ലാം കൊടുത്തുതീർക്കുക, ഒരു നന്ദിവാക്കും പറയുക.", "जो भी आपके लिए काम करते हैं, सबकी सूची बनाएँ: कामवाली, ड्राइवर, मिस्त्री, कर्मचारी। इसी हफ़्ते सारा बकाया चुका दें, और धन्यवाद भी कहें।"],
                 false, E_WEEK, None),
                ("deeds",
                 ["Finish the task you have postponed longest", "ഏറ്റവും കൂടുതൽ കാലം മാറ്റിവച്ച ജോലി തീർക്കുക", "सबसे लंबे समय से टाला हुआ काम पूरा करें"],
                 ["Delays in your life lift when your own pending duty is completed rather than avoided.", "നിങ്ങളുടെ സ്വന്തം ബാക്കി കടമ ഒഴിവാക്കാതെ പൂർത്തിയാക്കുമ്പോൾ ജീവിതത്തിലെ താമസങ്ങൾ മാറും.", "जब आप अपना बाकी फ़र्ज़ टालने के बजाय पूरा करते हैं, तो ज़िंदगी की देरी हटती है।"],
                 ["Pick the oldest pending task. Give it 20 minutes every day until it is done.", "ഏറ്റവും പഴയ ബാക്കി ജോലി എടുക്കുക. തീരുന്നതുവരെ ദിവസവും 20 മിനിറ്റ് അതിന് കൊടുക്കുക.", "सबसे पुराना अधूरा काम चुनें। पूरा होने तक रोज़ 20 मिनट उसे दें।"],
                 false, E_DAILY, None),
                ("deeds",
                 ["Help an old, sick or disabled person with your own hands", "വയോധികരെയോ രോഗികളെയോ ഭിന്നശേഷിക്കാരെയോ സ്വന്തം കൈകൊണ്ട് സഹായിക്കുക", "किसी बुज़ुर्ग, बीमार या दिव्यांग की अपने हाथों से मदद करें"],
                 ["Real service to those who carry life's heaviest load lightens your own heavy stretch.", "ജീവിതത്തിലെ ഏറ്റവും ഭാരം ചുമക്കുന്നവർക്കുള്ള യഥാർത്ഥ സേവനം നിങ്ങളുടെ ഭാരിച്ച കാലത്തെ ലഘുവാക്കും.", "ज़िंदगी का सबसे भारी बोझ उठाने वालों की सच्ची सेवा आपके भारी दौर को हल्का करती है।"],
                 ["Once a week, give an hour: shopping, a hospital visit, reading to them, or simply company.", "ആഴ്ചയിൽ ഒരു മണിക്കൂർ കൊടുക്കുക: സാധനം വാങ്ങിക്കൊടുക്കൽ, ആശുപത്രി സന്ദർശനം, വായിച്ചുകൊടുക്കൽ, അല്ലെങ്കിൽ കൂട്ടിരിക്കൽ.", "हफ़्ते में एक घंटा दें: सामान लाना, अस्पताल जाना, पढ़कर सुनाना, या बस साथ बैठना।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Give mustard oil, black sesame or a blanket to a labourer", "ഒരു തൊഴിലാളിക്ക് എണ്ണയോ എള്ളോ പുതപ്പോ കൊടുക്കുക", "किसी मज़दूर को सरसों का तेल, काले तिल या कंबल दें"],
                 ["The traditional gift for this kind of time; it eases delays, debts and tiredness at work.", "ഇത്തരം കാലത്തിനുള്ള പരമ്പരാഗത ദാനം; ജോലിയിലെ താമസവും കടവും ക്ഷീണവും കുറയ്ക്കും.", "ऐसे समय का पारंपरिक दान; काम में देरी, कर्ज़ और थकान कम करता है।"],
                 ["On Saturday, hand it yourself to a road worker, a cleaner or a watchman.", "ശനിയാഴ്ച, റോഡുപണിക്കാരനോ ശുചീകരണത്തൊഴിലാളിക്കോ കാവൽക്കാരനോ നേരിട്ട് കൊടുക്കുക.", "शनिवार को किसी सड़क मज़दूर, सफ़ाईकर्मी या चौकीदार को अपने हाथ से दें।"],
                 true, E_ONCE, None),
                ("giving",
                 ["Buy footwear or an umbrella for someone who walks to work", "നടന്ന് ജോലിക്ക് പോകുന്ന ഒരാൾക്ക് ചെരിപ്പോ കുടയോ വാങ്ങിക്കൊടുക്കുക", "पैदल काम पर जाने वाले को जूते या छाता दिलाएँ"],
                 ["Easing another's daily hardship eases the slowness in yours.", "മറ്റൊരാളുടെ ദൈനംദിന കഷ്ടപ്പാട് കുറയ്ക്കുമ്പോൾ നിങ്ങളുടെ ജീവിതത്തിലെ മന്ദത കുറയും.", "किसी की रोज़ की तकलीफ़ कम करने से आपकी ज़िंदगी की सुस्ती कम होती है।"],
                 ["Notice who walks to work in the heat or rain near you; buy them a pair and give it quietly.", "നിങ്ങളുടെ അടുത്ത് വെയിലിലും മഴയിലും നടന്ന് ജോലിക്ക് പോകുന്നവരെ ശ്രദ്ധിക്കുക; ഒരു ജോഡി വാങ്ങി നിശ്ശബ്ദമായി കൊടുക്കുക.", "आसपास कौन धूप-बारिश में पैदल काम पर जाता है, देखें; एक जोड़ी लेकर चुपचाप दे दें।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Rise before sunrise for 40 days", "40 ദിവസം സൂര്യോദയത്തിന് മുമ്പ് എഴുന്നേൽക്കുക", "40 दिन सूर्योदय से पहले उठें"],
                 ["Discipline is the cure for delay that needs no one else's help.", "മറ്റാരുടെയും സഹായം വേണ്ടാത്ത, താമസത്തിനുള്ള മരുന്നാണ് അച്ചടക്കം.", "देरी का ऐसा इलाज जिसके लिए किसी और की ज़रूरत नहीं: अनुशासन।"],
                 ["Set the alarm 30 minutes before sunrise. Use the first half hour for the day's hardest task.", "സൂര്യോദയത്തിന് 30 മിനിറ്റ് മുമ്പ് അലാറം വയ്ക്കുക. ആദ്യത്തെ അരമണിക്കൂർ ദിവസത്തെ ഏറ്റവും കഠിനമായ ജോലിക്ക്.", "सूर्योदय से 30 मिनट पहले का अलार्म लगाएँ। पहला आधा घंटा दिन के सबसे कठिन काम को दें।"],
                 false, E_40, None),
                ("nature",
                 ["Water a peepal or any large tree every Saturday", "എല്ലാ ശനിയാഴ്ചയും ഒരു അരയാലിനോ വലിയ മരത്തിനോ വെള്ളമൊഴിക്കുക", "हर शनिवार पीपल या किसी बड़े पेड़ को पानी दें"],
                 ["Patient care of something slow-growing builds the patience this time asks of you.", "പതുക്കെ വളരുന്ന ഒന്നിനെ ക്ഷമയോടെ പരിപാലിക്കുന്നത് ഈ കാലം ആവശ്യപ്പെടുന്ന ക്ഷമ വളർത്തും.", "धीरे बढ़ने वाली चीज़ की धैर्य से देखभाल वह धैर्य बनाती है जो यह समय माँगता है।"],
                 ["Carry a bottle of water on your Saturday walk and pour it at the root of the same tree each week.", "ശനിയാഴ്ച നടക്കുമ്പോൾ ഒരു കുപ്പി വെള്ളം കൊണ്ടുപോയി എല്ലാ ആഴ്ചയും ഒരേ മരത്തിന്റെ ചുവട്ടിൽ ഒഴിക്കുക.", "शनिवार की सैर पर पानी की बोतल ले जाएँ और हर हफ़्ते उसी पेड़ की जड़ में डालें।"],
                 true, E_WEEKLY, None),
                ("mind",
                 ["Five minutes of slow breathing when the weight comes", "ഭാരം തോന്നുമ്പോൾ അഞ്ച് മിനിറ്റ് പതുക്കെ ശ്വസിക്കുക", "भारीपन लगे तो पाँच मिनट धीमी साँस लें"],
                 ["Heavy times feel heavier in a tense body; slow breath lowers the pressure you carry.", "മുറുകിയ ശരീരത്തിൽ ഭാരിച്ച കാലം കൂടുതൽ ഭാരമായി തോന്നും; പതുക്കെയുള്ള ശ്വാസം നിങ്ങൾ ചുമക്കുന്ന സമ്മർദ്ദം കുറയ്ക്കും.", "तने हुए शरीर में भारी समय और भारी लगता है; धीमी साँस भीतर का दबाव घटाती है।"],
                 ["Breathe in for four counts, out for six, ten times. Do it at the same time each evening.", "നാല് എണ്ണുംവരെ ശ്വാസം അകത്തേക്ക്, ആറ് എണ്ണുംവരെ പുറത്തേക്ക്, പത്ത് തവണ. എല്ലാ വൈകുന്നേരവും ഒരേ സമയത്ത് ചെയ്യുക.", "चार गिनती तक साँस अंदर, छह तक बाहर, दस बार। हर शाम एक ही समय करें।"],
                 false, E_10, None),
                ("tradition",
                 ["Saturday fast: one meal after sunset, for 19 Saturdays", "ശനിയാഴ്ച വ്രതം: സൂര്യാസ്തമയശേഷം ഒരു നേരം ഭക്ഷണം, 19 ശനിയാഴ്ച", "शनिवार का व्रत: सूर्यास्त के बाद एक बार भोजन, 19 शनिवार"],
                 ["The classical fast through a long testing time.", "നീണ്ട പരീക്ഷണകാലത്തിനുള്ള പരമ്പരാഗത വ്രതം.", "लंबे कठिन दौर के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Hanuman Chalisa on Saturdays, or the chant below", "ശനിയാഴ്ചകളിൽ ഹനുമാൻ ചാലീസ, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "शनिवार को हनुमान चालीसा, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om praam preem praum sah Shanaischaraya namah", "ഓം പ്രാം പ്രീം പ്രൗം സഃ ശനൈശ്ചരായ നമഃ", "ॐ प्रां प्रीं प्रौं सः शनैश्चराय नमः"])),
                ("tradition",
                 ["A sesame-oil lamp at a Hanuman temple on Saturday; Thirunallar for the full rite", "ശനിയാഴ്ച ഹനുമാൻ ക്ഷേത്രത്തിൽ എള്ളെണ്ണ വിളക്ക്; പൂർണ്ണ ചടങ്ങിന് തിരുനള്ളാർ", "शनिवार को हनुमान मंदिर में तिल के तेल का दीया; पूरे अनुष्ठान के लिए तिरुनल्लार"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Rahu => Influence {
            effects: [
                "Restlessness and confusion: you want more, faster, and shortcuts look tempting. Anxiety rises, and people or deals may not be what they seem.",
                "അസ്വസ്ഥതയും ആശയക്കുഴപ്പവും: കൂടുതൽ, വേഗത്തിൽ വേണമെന്ന് തോന്നും, എളുപ്പവഴികൾ ആകർഷിക്കും. ഉത്കണ്ഠ കൂടും, ആളുകളും ഇടപാടുകളും കാണുന്നതുപോലെ ആകണമെന്നില്ല.",
                "बेचैनी और उलझन: ज़्यादा और जल्दी चाहिए, और शॉर्टकट लुभाते हैं। घबराहट बढ़ती है, और लोग या सौदे वैसे नहीं होते जैसे दिखते हैं।",
            ],
            people_en: "migrants, outsiders and those caught in addiction",
            people_ml_for: "കുടിയേറ്റക്കാർക്കും ഒറ്റപ്പെട്ടവർക്കും ലഹരിയിൽ കുടുങ്ങിയവർക്കും",
            people_hi_for: "प्रवासियों, बेगानों और नशे में फँसे लोगों",
            items: vec![
                ("deeds",
                 ["No shortcuts this month", "ഈ മാസം എളുപ്പവഴികൾ വേണ്ട", "इस महीने कोई शॉर्टकट नहीं"],
                 ["Tempting shortcuts are the trap now; doing it the straight way protects you from loss and shame.", "ആകർഷകമായ എളുപ്പവഴികളാണ് ഇപ്പോഴത്തെ കെണി; നേരായ വഴി നഷ്ടത്തിൽ നിന്നും നാണക്കേടിൽ നിന്നും രക്ഷിക്കും.", "लुभावने शॉर्टकट अभी जाल हैं; सीधा रास्ता नुकसान और शर्मिंदगी से बचाता है।"],
                 ["If a deal, a job or a person promises too much too fast, wait a week before saying yes.", "ഒരു ഇടപാടോ ജോലിയോ ആളോ വളരെ വേഗം വളരെയധികം വാഗ്ദാനം ചെയ്താൽ, സമ്മതിക്കുന്നതിന് മുമ്പ് ഒരാഴ്ച കാത്തിരിക്കുക.", "अगर कोई सौदा, नौकरी या व्यक्ति बहुत जल्दी बहुत ज़्यादा वादा करे, तो 'हाँ' कहने से पहले एक हफ़्ता रुकें।"],
                 false, E_MONTH, None),
                ("deeds",
                 ["Give up one addiction or bad habit, even for 40 days", "ഒരു ലഹരിയോ ദുശ്ശീലമോ, 40 ദിവസത്തേക്കെങ്കിലും, ഉപേക്ഷിക്കുക", "कोई एक नशा या बुरी आदत छोड़ें, कम से कम 40 दिन"],
                 ["Restlessness feeds on intoxicants, gambling and endless scrolling; starving it calms the mind.", "ലഹരി, ചൂതാട്ടം, അവസാനമില്ലാത്ത സ്ക്രോളിംഗ് എന്നിവയിലാണ് അസ്വസ്ഥത വളരുന്നത്; അവ നിർത്തിയാൽ മനസ്സ് ശാന്തമാകും.", "बेचैनी नशे, जुए और बेअंत स्क्रॉलिंग पर पलती है; उसे भूखा रखने से मन शांत होता है।"],
                 ["Choose one: alcohol, tobacco, betting, late-night phone. Tell one person you have stopped.", "ഒന്ന് തിരഞ്ഞെടുക്കുക: മദ്യം, പുകയില, വാതുവയ്പ്പ്, രാത്രി വൈകിയുള്ള ഫോൺ. നിർത്തിയെന്ന് ഒരാളോട് പറയുക.", "एक चुनें: शराब, तंबाकू, सट्टा, देर रात फ़ोन। किसी एक को बताएँ कि आपने छोड़ा है।"],
                 false, E_40, None),
                ("deeds",
                 ["Help a migrant worker or an outsider settle", "ഒരു കുടിയേറ്റത്തൊഴിലാളിയെയോ പുറമേനിന്ന് വന്ന ഒരാളെയോ സ്ഥിരപ്പെടാൻ സഹായിക്കുക", "किसी प्रवासी मज़दूर या बाहर से आए व्यक्ति को बसने में मदद करें"],
                 ["Kindness to those far from home eases confusion and brings honest help your way.", "വീട്ടിൽ നിന്ന് ദൂരെയുള്ളവരോടുള്ള ദയ ആശയക്കുഴപ്പം കുറയ്ക്കും, സത്യസന്ധമായ സഹായം നിങ്ങളിലേക്ക് കൊണ്ടുവരും.", "घर से दूर लोगों पर दया उलझन घटाती है और ईमानदार मदद आपकी ओर लाती है।"],
                 ["Help with a form, a room, a fair wage or a phone call home.", "ഒരു ഫോം, ഒരു മുറി, ന്യായമായ കൂലി, അല്ലെങ്കിൽ വീട്ടിലേക്കൊരു ഫോൺ വിളി: ഇവയിൽ സഹായിക്കുക.", "फ़ॉर्म, कमरा, उचित मज़दूरी या घर पर एक फ़ोन: इनमें मदद करें।"],
                 false, E_ONCE, None),
                ("giving",
                 ["Feed ants, fish or stray dogs on Saturday", "ശനിയാഴ്ച ഉറുമ്പുകൾക്കോ മീനുകൾക്കോ തെരുവുനായ്ക്കൾക്കോ ഭക്ഷണം കൊടുക്കുക", "शनिवार को चींटियों, मछलियों या आवारा कुत्तों को खिलाएँ"],
                 ["The traditional quiet gift for confusion and fear; it asks nothing back.", "ആശയക്കുഴപ്പത്തിനും ഭയത്തിനുമുള്ള പരമ്പരാഗതമായ നിശ്ശബ്ദ ദാനം; ഒന്നും തിരിച്ചു ചോദിക്കില്ല.", "उलझन और डर के लिए पारंपरिक, चुपचाप किया दान; बदले में कुछ नहीं माँगता।"],
                 ["Sugar and flour at an anthill, or food for street dogs, without being seen.", "ഉറുമ്പിൻകൂട്ടിനരികിൽ പഞ്ചസാരയും മാവും, അല്ലെങ്കിൽ തെരുവുനായ്ക്കൾക്ക് ഭക്ഷണം, ആരും കാണാതെ.", "चींटियों के बिल के पास चीनी और आटा, या गली के कुत्तों को खाना, बिना दिखावे के।"],
                 true, E_WEEKLY, None),
                ("giving",
                 ["Support someone fighting addiction", "ലഹരിക്കെതിരെ പോരാടുന്ന ഒരാളെ പിന്തുണയ്ക്കുക", "नशे से लड़ रहे किसी व्यक्ति का साथ दें"],
                 ["Helping one person out of the fog clears some of your own.", "ഒരാളെ മൂടൽമഞ്ഞിൽ നിന്ന് പുറത്തുകൊണ്ടുവരുന്നത് നിങ്ങളുടെ സ്വന്തം മൂടലും നീക്കും.", "किसी एक को धुंध से निकालने से आपकी अपनी धुंध भी छँटती है।"],
                 ["Pay for a counselling session, drive them to a meeting, or simply stay in touch.", "ഒരു കൗൺസലിംഗ് സെഷന് പണം കൊടുക്കുക, ഒരു മീറ്റിംഗിലേക്ക് കൊണ്ടുപോവുക, അല്ലെങ്കിൽ ബന്ധം നിലനിർത്തുക മാത്രം.", "काउंसलिंग का ख़र्च दें, उन्हें मीटिंग तक ले जाएँ, या बस संपर्क में रहें।"],
                 false, E_ONCE, None),
                ("habits",
                 ["Phone off an hour before bed, and no news after dark", "ഉറങ്ങുന്നതിന് ഒരു മണിക്കൂർ മുമ്പ് ഫോൺ ഓഫ്, ഇരുട്ടിയാൽ വാർത്ത വേണ്ട", "सोने से एक घंटा पहले फ़ोन बंद, और अँधेरे के बाद ख़बरें नहीं"],
                 ["Anxiety grows on endless input; a quiet hour lets the mind settle.", "അവസാനമില്ലാത്ത വിവരങ്ങളിലാണ് ഉത്കണ്ഠ വളരുന്നത്; ഒരു ശാന്തമായ മണിക്കൂർ മനസ്സിനെ അടക്കും.", "बेअंत जानकारी पर घबराहट बढ़ती है; एक शांत घंटा मन को बैठने देता है।"],
                 ["Charge the phone outside the bedroom. Read a paper book, talk, or rest instead.", "ഫോൺ കിടപ്പുമുറിക്ക് പുറത്ത് ചാർജ് ചെയ്യുക. പകരം പുസ്തകം വായിക്കുക, സംസാരിക്കുക, വിശ്രമിക്കുക.", "फ़ोन बेडरूम के बाहर चार्ज करें। इसके बजाय कागज़ की किताब पढ़ें, बात करें, या आराम करें।"],
                 false, E_DAILY, None),
                ("nature",
                 ["Clean a dirty public place: a drain, a dump, a neglected corner", "ഒരു വൃത്തികെട്ട പൊതുസ്ഥലം വൃത്തിയാക്കുക: ഓട, മാലിന്യക്കൂമ്പാരം, അവഗണിക്കപ്പെട്ട ഒരു മൂല", "कोई गंदी सार्वजनिक जगह साफ़ करें: नाली, कूड़े का ढेर, उपेक्षित कोना"],
                 ["Clearing what others avoid clears the confusion that hangs over this time.", "മറ്റുള്ളവർ ഒഴിവാക്കുന്നത് വൃത്തിയാക്കുന്നത് ഈ കാലത്തെ മൂടുന്ന ആശയക്കുഴപ്പം നീക്കും.", "जिससे दूसरे बचते हैं उसे साफ़ करना इस समय पर छाई उलझन को साफ़ करता है।"],
                 ["One hour, gloves on, with a friend or a group. Do it again next month.", "ഒരു മണിക്കൂർ, കയ്യുറ ധരിച്ച്, ഒരു സുഹൃത്തിനോടോ സംഘത്തോടോ ഒപ്പം. അടുത്ത മാസം വീണ്ടും ചെയ്യുക.", "एक घंटा, दस्ताने पहनकर, किसी दोस्त या समूह के साथ। अगले महीने फिर करें।"],
                 false, E_MONTHLY, None),
                ("mind",
                 ["Ten minutes of silent sitting, eyes on one point", "പത്ത് മിനിറ്റ് ഒരു ബിന്ദുവിൽ കണ്ണുനട്ട് നിശ്ശബ്ദമായി ഇരിക്കുക", "दस मिनट एक बिंदु पर नज़र टिकाकर चुपचाप बैठें"],
                 ["A racing mind slows when it has one thing to rest on.", "ഓടുന്ന മനസ്സിന് ഊന്നാൻ ഒന്നുണ്ടെങ്കിൽ അത് വേഗം കുറയ്ക്കും.", "भागता मन तब धीमा होता है जब उसे टिकने को एक चीज़ मिले।"],
                 ["Sit still, look at a lamp flame or a spot on the wall, and return to it each time the mind wanders.", "നിശ്ചലമായി ഇരുന്ന്, ഒരു വിളക്കിന്റെ നാളമോ ചുവരിലെ ഒരു പുള്ളിയോ നോക്കുക; മനസ്സ് അലയുമ്പോഴെല്ലാം അതിലേക്ക് തിരിച്ചുവരുക.", "शांत बैठें, दीये की लौ या दीवार के किसी बिंदु को देखें, और मन भटके तो हर बार वहीं लौटें।"],
                 false, E_10, None),
                ("tradition",
                 ["Saturday fast for 18 Saturdays", "18 ശനിയാഴ്ച വ്രതം", "18 शनिवार का व्रत"],
                 ["The classical fast for fear, confusion and sudden trouble.", "ഭയം, ആശയക്കുഴപ്പം, പെട്ടെന്നുള്ള പ്രശ്നം എന്നിവയ്ക്കുള്ള പരമ്പരാഗത വ്രതം.", "डर, उलझन और अचानक मुसीबत के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["Durga Saptashati or a Durga chant on Saturdays, or the chant below", "ശനിയാഴ്ചകളിൽ ദുർഗാ സപ്തശതിയോ ദുർഗാമന്ത്രമോ, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "शनिवार को दुर्गा सप्तशती या दुर्गा मंत्र, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om bhraam bhreem bhraum sah Rahave namah", "ഓം ഭ്രാം ഭ്രീം ഭ്രൗം സഃ രാഹവേ നമഃ", "ॐ भ्रां भ्रीं भ्रौं सः राहवे नमः"])),
                ("tradition",
                 ["A Durga temple; Thirunageswaram for the full rite", "ദുർഗാക്ഷേത്രം; പൂർണ്ണ ചടങ്ങിന് തിരുനാഗേശ്വരം", "दुर्गा मंदिर; पूरे अनुष्ठान के लिए तिरुनागेश्वरम"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
        DashaLord::Ketu => Influence {
            effects: [
                "Interest fades and things slip away: losses, detachment and a sense of drifting. Health complaints can be vague and hard to pin down.",
                "താൽപര്യം കുറയും, കാര്യങ്ങൾ കൈവിട്ടുപോകും: നഷ്ടങ്ങൾ, അകൽച്ച, ദിശയില്ലാത്ത തോന്നൽ. ആരോഗ്യപ്രശ്നങ്ങൾ അവ്യക്തവും കണ്ടുപിടിക്കാൻ പ്രയാസമുള്ളതുമാകാം.",
                "रुचि घटती है और चीज़ें हाथ से फिसलती हैं: नुकसान, विरक्ति और भटकाव का एहसास। सेहत की शिकायतें धुंधली और पकड़ में न आने वाली हो सकती हैं।",
            ],
            people_en: "the sick, the lonely and those who have lost their way",
            people_ml_for: "രോഗികൾക്കും ഏകാകികൾക്കും വഴിതെറ്റിയവർക്കും",
            people_hi_for: "बीमारों, अकेलों और राह भटके लोगों",
            items: vec![
                ("deeds",
                 ["Care for someone who is sick and alone", "രോഗിയും ഒറ്റയ്ക്കുമായ ഒരാളെ പരിചരിക്കുക", "किसी बीमार और अकेले व्यक्ति की देखभाल करें"],
                 ["Tending the forgotten turns loss and drift into meaning.", "മറന്നുപോയവരെ പരിചരിക്കുന്നത് നഷ്ടത്തെയും ദിശയില്ലായ്മയെയും അർത്ഥമാക്കി മാറ്റും.", "भुलाए गए लोगों की सेवा नुकसान और भटकाव को अर्थ में बदल देती है।"],
                 ["A weekly visit to a ward, a hospice or a neighbour who is bedridden.", "ആഴ്ചയിലൊരിക്കൽ ഒരു വാർഡിലോ സാന്ത്വനകേന്ദ്രത്തിലോ കിടപ്പിലായ ഒരു അയൽക്കാരനെയോ കാണാൻ പോവുക.", "हर हफ़्ते किसी वार्ड, हॉस्पिस या बिस्तर पर पड़े पड़ोसी से मिलने जाएँ।"],
                 false, E_WEEKLY, None),
                ("deeds",
                 ["Finish what you started before starting anything new", "പുതിയതൊന്ന് തുടങ്ങുന്നതിന് മുമ്പ് തുടങ്ങിയത് തീർക്കുക", "कुछ नया शुरू करने से पहले जो शुरू किया है, पूरा करें"],
                 ["Things slip away now when they are left half-done; completion holds them.", "പാതിയിൽ നിർത്തുന്ന കാര്യങ്ങൾ ഇപ്പോൾ കൈവിട്ടുപോകും; പൂർത്തിയാക്കുന്നത് അവയെ പിടിച്ചുനിർത്തും.", "अधूरी छोड़ी चीज़ें अभी हाथ से फिसलती हैं; पूरा करना उन्हें थामे रखता है।"],
                 ["List what is half-done. Close one each week, or let it go on purpose.", "പാതിയായവയുടെ പട്ടിക ഉണ്ടാക്കുക. ആഴ്ചയിൽ ഒന്ന് തീർക്കുക, അല്ലെങ്കിൽ ബോധപൂർവം ഉപേക്ഷിക്കുക.", "अधूरे कामों की सूची बनाएँ। हर हफ़्ते एक पूरा करें, या सोच-समझकर छोड़ दें।"],
                 false, E_WEEKLY, None),
                ("deeds",
                 ["Honour your elders and ancestors", "മുതിർന്നവരെയും പൂർവികരെയും ആദരിക്കുക", "बड़ों और पूर्वजों का सम्मान करें"],
                 ["Remembering where you come from gives direction when life feels aimless.", "ജീവിതത്തിന് ലക്ഷ്യമില്ലെന്ന് തോന്നുമ്പോൾ നിങ്ങൾ എവിടെ നിന്ന് വരുന്നുവെന്ന ഓർമ ദിശ തരും.", "जब ज़िंदगी बेमक़सद लगे, तो यह याद कि आप कहाँ से आए हैं, दिशा देती है।"],
                 ["Visit the eldest in your family; ask them for a story about your grandparents and write it down.", "കുടുംബത്തിലെ ഏറ്റവും മുതിർന്നയാളെ കാണുക; മുത്തച്ഛനെയും മുത്തശ്ശിയെയും കുറിച്ച് ഒരു കഥ ചോദിച്ച് എഴുതിവയ്ക്കുക.", "परिवार के सबसे बड़े से मिलें; दादा-दादी के बारे में कोई कहानी पूछें और लिख लें।"],
                 false, E_ONCE, None),
                ("giving",
                 ["Feed a stray dog every day", "എല്ലാ ദിവസവും ഒരു തെരുവുനായയ്ക്ക് ഭക്ഷണം കൊടുക്കുക", "हर दिन एक आवारा कुत्ते को खिलाएँ"],
                 ["The traditional gift for this time, simple and constant: it steadies a drifting mind.", "ഈ കാലത്തിനുള്ള പരമ്പരാഗതവും ലളിതവും സ്ഥിരവുമായ ദാനം: അലയുന്ന മനസ്സിനെ ഉറപ്പിക്കും.", "इस समय का पारंपरिक, सरल और नियमित दान: भटकते मन को सँभालता है।"],
                 ["Same street, same time, a little rice or bread. Keep it up through this time.", "ഒരേ തെരുവ്, ഒരേ സമയം, കുറച്ച് ചോറോ റൊട്ടിയോ. ഈ കാലം മുഴുവൻ തുടരുക.", "वही गली, वही समय, थोड़ा चावल या रोटी। इस पूरे समय जारी रखें।"],
                 false, E_DAILY, None),
                ("giving",
                 ["Give a blanket of many colours, or sesame, to a hermit or a homeless person", "പല നിറമുള്ള ഒരു പുതപ്പോ എള്ളോ ഒരു സന്യാസിക്കോ വീടില്ലാത്ത ഒരാൾക്കോ കൊടുക്കുക", "किसी संन्यासी या बेघर को रंग-बिरंगा कंबल या तिल दें"],
                 ["The old gift for losses and drift.", "നഷ്ടങ്ങൾക്കും ദിശയില്ലായ്മയ്ക്കുമുള്ള പഴയ ദാനം.", "नुकसान और भटकाव के लिए पुराना दान।"],
                 ["On Tuesday, hand it over yourself, without making a show of it.", "ചൊവ്വാഴ്ച, പ്രദർശനമില്ലാതെ, സ്വയം കൈമാറുക.", "मंगलवार को, बिना दिखावे के, ख़ुद दें।"],
                 true, E_ONCE, None),
                ("habits",
                 ["A light meal, early, and one quiet morning a week", "നേരത്തെ ലഘുഭക്ഷണം, ആഴ്ചയിലൊരു ശാന്തമായ പ്രഭാതം", "जल्दी, हल्का खाना और हफ़्ते में एक शांत सुबह"],
                 ["Vague complaints of body and mind ease with simple food and rest.", "ലളിതമായ ഭക്ഷണവും വിശ്രമവും ശരീരത്തിന്റെയും മനസ്സിന്റെയും അവ്യക്തമായ പ്രശ്നങ്ങൾ കുറയ്ക്കും.", "सादा भोजन और आराम तन-मन की धुंधली शिकायतें घटाते हैं।"],
                 ["Dinner before eight; one morning a week with no plans until noon.", "എട്ടിന് മുമ്പ് അത്താഴം; ആഴ്ചയിലൊരു പ്രഭാതം ഉച്ചവരെ പരിപാടികളൊന്നുമില്ലാതെ.", "आठ बजे से पहले रात का खाना; हफ़्ते में एक सुबह दोपहर तक कोई योजना नहीं।"],
                 false, E_DAILY, None),
                ("nature",
                 ["Plant and tend something that takes years to grow", "വർഷങ്ങളെടുത്ത് വളരുന്ന എന്തെങ്കിലും നട്ടുവളർത്തുക", "ऐसा कुछ लगाएँ और सँभालें जो बरसों में बढ़ता है"],
                 ["Committing to a slow future gives direction to a drifting present.", "പതുക്കെ വരുന്ന ഭാവിയോടുള്ള പ്രതിബദ്ധത ദിശയില്ലാത്ത വർത്തമാനത്തിന് ദിശ തരും.", "धीमे भविष्य से जुड़ना भटके वर्तमान को दिशा देता है।"],
                 ["A fruit tree in your yard or a community garden. Visit it monthly.", "മുറ്റത്തോ ഒരു പൊതുതോട്ടത്തിലോ ഒരു ഫലവൃക്ഷം. മാസത്തിലൊരിക്കൽ അതിനെ സന്ദർശിക്കുക.", "आँगन या किसी सामुदायिक बगीचे में एक फलदार पेड़। हर महीने उसे देखने जाएँ।"],
                 false, E_MONTHLY, None),
                ("mind",
                 ["Sit in silence for ten minutes and ask what you truly want", "പത്ത് മിനിറ്റ് നിശ്ശബ്ദമായിരുന്ന് നിങ്ങൾക്ക് ശരിക്കും എന്താണ് വേണ്ടതെന്ന് ചോദിക്കുക", "दस मिनट चुप बैठकर पूछें कि आपको सच में क्या चाहिए"],
                 ["Drift ends when the real question is faced, quietly, without hurry.", "യഥാർത്ഥ ചോദ്യം തിടുക്കമില്ലാതെ, ശാന്തമായി നേരിടുമ്പോൾ ദിശയില്ലായ്മ അവസാനിക്കും.", "भटकाव तब ख़त्म होता है जब असली सवाल का सामना शांति से, बिना जल्दी के किया जाए।"],
                 ["Same place each day. Write one honest line after.", "എല്ലാ ദിവസവും ഒരേ സ്ഥലത്ത്. അതിനുശേഷം സത്യസന്ധമായ ഒരു വരി എഴുതുക.", "हर दिन एक ही जगह। बाद में एक सच्ची पंक्ति लिखें।"],
                 false, E_10, None),
                ("tradition",
                 ["Tuesday fast for 18 Tuesdays", "18 ചൊവ്വാഴ്ച വ്രതം", "18 मंगलवार का व्रत"],
                 ["The classical fast for losses and confusion.", "നഷ്ടങ്ങൾക്കും ആശയക്കുഴപ്പത്തിനുമുള്ള പരമ്പരാഗത വ്രതം.", "नुकसान और उलझन के लिए पारंपरिक व्रत।"],
                 FAST_HOW, true, E_WEEKLY, None),
                ("tradition",
                 ["A Ganesha chant on Tuesdays, or the chant below", "ചൊവ്വാഴ്ചകളിൽ ഗണപതിമന്ത്രം, അല്ലെങ്കിൽ താഴെയുള്ള മന്ത്രം", "मंगलवार को गणेश मंत्र, या नीचे का मंत्र"],
                 CHANT_WHY, CHANT_HOW, true, E_WISH,
                 Some(["Om sraam sreem sraum sah Ketave namah", "ഓം സ്രാം സ്രീം സ്രൗം സഃ കേതവേ നമഃ", "ॐ स्रां स्रीं स्रौं सः केतवे नमः"])),
                ("tradition",
                 ["A Ganesha temple; Keezhaperumpallam for the full rite", "ഗണപതിക്ഷേത്രം; പൂർണ്ണ ചടങ്ങിന് കീഴപ്പെരുമ്പള്ളം", "गणेश मंदिर; पूरे अनुष्ठान के लिए कीझपेरुमपल्लम"],
                 TEMPLE_WHY, TEMPLE_HOW, true, E_WISH, None),
            ],
        },
    }
}

const BRANCHES: [(&str, T3, T3); 7] = [
    ("deeds",
     ["Deeds anyone can do", "ആർക്കും ചെയ്യാവുന്ന പ്രവൃത്തികൾ", "ऐसे काम जो कोई भी कर सकता है"],
     ["These work for anyone, of any faith or none. They do the real work.", "ഏത് വിശ്വാസമുള്ളവർക്കും വിശ്വാസമില്ലാത്തവർക്കും ഇവ ചെയ്യാം. യഥാർത്ഥ ഫലം തരുന്നത് ഇവയാണ്.", "ये किसी भी धर्म के या बिना धर्म के व्यक्ति के लिए हैं। असली असर इन्हीं का है।"]),
    ("giving",
     ["Giving", "ദാനം", "दान"],
     ["Give by your own hand, to someone who needs it. Small and regular beats big and once.", "സ്വന്തം കൈകൊണ്ട്, ആവശ്യമുള്ളവർക്ക് കൊടുക്കുക. ഒരിക്കൽ വലുതായി കൊടുക്കുന്നതിനേക്കാൾ നല്ലത് ചെറുതായി പതിവായി കൊടുക്കുന്നതാണ്.", "अपने हाथ से, ज़रूरतमंद को दें। एक बार बड़ा देने से बेहतर है थोड़ा-थोड़ा नियमित देना।"]),
    ("habits",
     ["Habits", "ശീലങ്ങൾ", "आदतें"],
     ["Self-discipline that steadies you from the inside.", "ഉള്ളിൽ നിന്ന് നിങ്ങളെ ഉറപ്പിക്കുന്ന ആത്മനിയന്ത്രണം.", "भीतर से संभालने वाला आत्म-अनुशासन।"]),
    ("nature",
     ["Nature and creatures", "പ്രകൃതിയും ജീവികളും", "प्रकृति और जीव"],
     ["Water, trees and animals: care that asks nothing back.", "വെള്ളം, മരങ്ങൾ, മൃഗങ്ങൾ: ഒന്നും തിരിച്ചു ചോദിക്കാത്ത പരിചരണം.", "पानी, पेड़ और जानवर: ऐसी सेवा जो बदले में कुछ नहीं माँगती।"]),
    ("mind",
     ["Quiet the mind, for any faith or none", "മനസ്സ് ശാന്തമാക്കുക, ഏത് വിശ്വാസത്തിനും വിശ്വാസമില്ലാത്തവർക്കും", "मन को शांत करें, किसी भी धर्म के लिए, या बिना धर्म के"],
     ["A few minutes a day that lower the pressure inside.", "ഉള്ളിലെ സമ്മർദ്ദം കുറയ്ക്കുന്ന ദിവസവും കുറച്ച് മിനിറ്റുകൾ.", "रोज़ के कुछ मिनट जो भीतर का दबाव घटाते हैं।"]),
    ("prayer",
     ["Prayer in your own faith", "നിങ്ങളുടെ സ്വന്തം വിശ്വാസത്തിലെ പ്രാർത്ഥന", "अपने धर्म में प्रार्थना"],
     ["In a church, mosque, gurdwara, temple or at home: pray the way you already pray.", "പള്ളിയിലോ മസ്ജിദിലോ ക്ഷേത്രത്തിലോ മറ്റേതെങ്കിലും ആരാധനാലയത്തിലോ വീട്ടിലോ: നിങ്ങൾ ഇപ്പോൾ പ്രാർത്ഥിക്കുന്ന രീതിയിൽ തന്നെ.", "चर्च, मस्जिद, गुरुद्वारा, मंदिर या घर पर: जैसे आप पहले से प्रार्थना करते हैं वैसे ही।"]),
    ("tradition",
     ["Hindu tradition (optional)", "ഹിന്ദു പാരമ്പര്യം (ഐച്ഛികം)", "हिंदू परंपरा (वैकल्पिक)"],
     ["For those who follow it. Never required, and never a substitute for the deeds above. A gemstone is never necessary; consider one only with a qualified astrologer's advice, never on anyone's word.", "ഇത് പിന്തുടരുന്നവർക്ക്. ഒരിക്കലും നിർബന്ധമില്ല, മുകളിലെ പ്രവൃത്തികൾക്ക് പകരവുമല്ല. രത്നം ഒരിക്കലും ആവശ്യമില്ല; യോഗ്യനായ ജ്യോതിഷിയുടെ ഉപദേശത്തോടെ മാത്രം ആലോചിക്കുക, ആരുടെയെങ്കിലും വാക്ക് കേട്ടല്ല.", "जो इसे मानते हैं उनके लिए। कभी ज़रूरी नहीं, और ऊपर के कामों का विकल्प भी नहीं। रत्न कभी ज़रूरी नहीं; सोचें तो सिर्फ़ योग्य ज्योतिषी की सलाह पर, किसी के कहने पर नहीं।"]),
];

/// The prayer remedy for an influence: faith-neutral, the influence's day and people.
fn prayer_item(lang: Lang, l: DashaLord, inf: &Influence) -> TreeRemedy {
    let d = day_of(l);
    let wd = weekday(lang, d);
    let pr = lang::pressure(lang, l);
    let (title, why, how) = match lang {
        Lang::En => (
            format!("On {wd}, pray for {}", inf.people_en),
            format!("Whatever you pray to, or in plain silence, holding these people in mind eases {pr}."),
            "In a church, mosque, gurdwara, temple or at home: five minutes, then do one small kind thing for one of them that week.".to_string(),
        ),
        Lang::Ml => (
            format!("{wd} {} വേണ്ടി പ്രാർത്ഥിക്കുക", inf.people_ml_for),
            format!("ആരോടാണോ നിങ്ങൾ പ്രാർത്ഥിക്കുന്നത്, അല്ലെങ്കിൽ വെറും നിശ്ശബ്ദതയിൽ, ഇവരെ മനസ്സിൽ കരുതുന്നത് {pr} എന്നിവ കുറയ്ക്കും."),
            "പള്ളിയിലോ മസ്ജിദിലോ ക്ഷേത്രത്തിലോ മറ്റേതെങ്കിലും ആരാധനാലയത്തിലോ വീട്ടിലോ: അഞ്ച് മിനിറ്റ്, പിന്നെ ആ ആഴ്ച അവരിൽ ഒരാൾക്ക് ഒരു ചെറിയ നന്മ ചെയ്യുക.".to_string(),
        ),
        Lang::Hi => (
            format!("{wd} को {} के लिए प्रार्थना करें", inf.people_hi_for),
            format!("आप जिससे भी प्रार्थना करते हैं, या बस मौन में, इन लोगों को मन में रखने से {pr} कम होते हैं।"),
            "चर्च, मस्जिद, गुरुद्वारा, मंदिर या घर पर: पाँच मिनट, फिर उस हफ़्ते उनमें से किसी एक के लिए छोटा सा भला काम करें।".to_string(),
        ),
    };
    TreeRemedy {
        title,
        why,
        how,
        day: wd.to_string(),
        effort: lang.pick(E_WEEKLY).to_string(),
        kind: "prayer",
        mantra: None,
        costs_money: false,
        source_keys: tree_source_keys("prayer", "", false),
    }
}

/// The seven branches of remedies for an influence, in the reader's language.
pub fn branches(lang: Lang, l: DashaLord) -> Vec<Branch> {
    let inf = influence(l);
    let d = day_of(l);
    BRANCHES
        .iter()
        .map(|(key, title, intro)| {
            let mut items: Vec<TreeRemedy> = inf
                .items
                .iter()
                .filter(|it| it.0 == *key)
                .map(|(k, t, w, h, on_day, e, m)| TreeRemedy {
                    title: lang.pick(*t).to_string(),
                    why: lang.pick(*w).to_string(),
                    how: lang.pick(*h).to_string(),
                    day: if *on_day { weekday(lang, d).to_string() } else { any_day(lang).to_string() },
                    effort: lang.pick(*e).to_string(),
                    kind: k,
                    mantra: m.map(|m| lang.pick(m).to_string()),
                    costs_money: crate::parihara::costs_money_title(t[0]),
                    source_keys: tree_source_keys(k, t[0], m.is_some()),
                })
                .collect();
            // Free first within a branch; whatever costs money after.
            items.sort_by_key(|it| it.costs_money);
            if *key == "prayer" {
                items.push(prayer_item(lang, l, &inf));
            }
            Branch {
                key,
                title: lang.pick(*title).to_string(),
                intro: lang.pick(*intro).to_string(),
                items,
            }
        })
        .collect()
}

fn strength_words(lang: Lang, pull: f64) -> &'static str {
    let a = pull.abs();
    let t = if a >= 0.05 {
        ["Pressing hard", "ശക്തമായി അമർത്തുന്നു", "भारी दबाव"]
    } else if a >= 0.02 {
        ["Pressing moderately", "മിതമായി അമർത്തുന്നു", "मध्यम दबाव"]
    } else {
        ["Pressing lightly", "ലഘുവായി അമർത്തുന്നു", "हल्का दबाव"]
    };
    lang.pick(t)
}

fn source_words(lang: Lang, kind: &str) -> &'static str {
    let t = match kind {
        "dasha" => [
            "It comes from the stretch of life you are in.",
            "നിങ്ങൾ കടന്നുപോകുന്ന ജീവിതഘട്ടത്തിൽ നിന്നാണ് ഇത് വരുന്നത്.",
            "यह ज़िंदगी के उस दौर से आता है जिसमें आप हैं।",
        ],
        "gochara" => [
            "It is a passing season, not a life sentence.",
            "ഇത് കടന്നുപോകുന്ന ഒരു കാലമാണ്, ജീവിതകാലശിക്ഷയല്ല.",
            "यह गुज़र जाने वाला मौसम है, उम्र भर की सज़ा नहीं।",
        ],
        _ => [
            "It is a tendency from birth that flares at times.",
            "ഇത് ജനനം മുതലുള്ള ഒരു പ്രവണതയാണ്, ഇടയ്ക്ക് ശക്തമാകും.",
            "यह जन्म से एक प्रवृत्ति है जो कभी-कभी उभरती है।",
        ],
    };
    lang.pick(t)
}

fn until_words(lang: Lang, kind: &str, until: Option<Instant>, lon: f64) -> String {
    match until {
        Some(u) => match lang {
            Lang::En => format!("It eases around {}.", date(lang, u, lon)),
            Lang::Ml => format!("{}-ഓടെ ഇത് അയയും.", date(lang, u, lon)),
            Lang::Hi => format!("{} के आसपास यह हल्का होगा।", date(lang, u, lon)),
        },
        None if kind == "natal" => lang
            .pick([
                "It is part of you from birth; the remedies keep it in check.",
                "ഇത് ജനനം മുതൽ നിങ്ങളുടെ ഭാഗമാണ്; പരിഹാരങ്ങൾ ഇതിനെ നിയന്ത്രണത്തിൽ നിർത്തും.",
                "यह जन्म से आपका हिस्सा है; उपाय इसे काबू में रखते हैं।",
            ])
            .to_string(),
        None => lang
            .pick([
                "No end is in sight within two years; the remedies are what you can do.",
                "രണ്ടു വർഷത്തിനുള്ളിൽ അവസാനം കാണുന്നില്ല; നിങ്ങൾക്ക് ചെയ്യാനാവുന്നത് ഈ പരിഹാരങ്ങളാണ്.",
                "दो साल में इसका अंत नहीं दिखता; आप जो कर सकते हैं वह ये उपाय हैं।",
            ])
            .to_string(),
    }
}

/// Build the remedy tree for `life` (or the weakest life now, when `None`).
#[allow(clippy::too_many_arguments)]
pub fn remedy_tree(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Option<Life>,
    lang: Lang,
    at: Instant,
    focus: &[Life],
) -> Result<RemedyTree, ForecastError> {
    let lon = obs.longitude_deg;
    let life = match life {
        Some(l) => l,
        None => {
            let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
            m.lives
                .iter()
                .filter(|l| focus.is_empty() || focus.contains(&l.life))
                .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
                .map(|l| l.life)
                .unwrap_or(Life::Love)
        }
    };
    let e = explain(spk, eop, natal, obs, system, ayanamsa, life, at)?;
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);

    // The strongest weighing influences with a graha behind them, one node per graha.
    let mut picked: Vec<(DashaLord, &'static str, f64, Option<Instant>)> = Vec::new();
    let mut causes: Vec<&crate::explain::Cause> = e
        .causes
        .iter()
        .filter(|c| c.pull < 0.0 && c.planet.is_some() && matches!(c.kind, "dasha" | "gochara" | "natal"))
        .collect();
    causes.sort_by(|a, b| a.pull.partial_cmp(&b.pull).unwrap_or(std::cmp::Ordering::Equal));
    for c in causes {
        let g = c.planet.unwrap_or(DashaLord::Saturn);
        if picked.iter().any(|p| p.0 == g) {
            continue;
        }
        picked.push((g, c.kind, c.pull, c.until));
        if picked.len() == 3 {
            break;
        }
    }
    let light = picked.is_empty();
    if light {
        // Nothing weighs: keep the running stretch's lord, lightly, so the person still
        // has the deeds that keep things well.
        let g = levels.get(1).or(levels.first()).map(|p| p.lord).unwrap_or(DashaLord::Saturn);
        picked.push((g, "dasha", 0.0, None));
    }

    let nodes: Vec<CauseNode> = picked
        .iter()
        .map(|&(g, kind, pull, until)| {
            let since = if kind == "dasha" {
                levels.iter().find(|p| p.lord == g).map(|p| p.start)
            } else {
                None
            };
            let inf = influence(g);
            let life_clause = match lang {
                Lang::En => format!("Right now it touches your {}.", life_lower(lang, life)),
                Lang::Ml => format!("ഇപ്പോൾ ഇത് ബാധിക്കുന്നത്: {}.", life_name(lang, life)),
                Lang::Hi => format!("अभी इसका असर: {}।", life_name(lang, life)),
            };
            CauseNode {
                key: g,
                source: kind,
                title: match lang {
                    Lang::En => cap(lang::pressure(lang, g)),
                    _ => lang::pressure(lang, g).to_string(),
                },
                what_it_does: format!("{} {} {}", lang.pick(inf.effects), source_words(lang, kind), life_clause),
                since,
                until,
                until_words: until_words(lang, kind, until, lon),
                strength: (pull.abs() * 100.0).min(50.0),
                strength_words: strength_words(lang, pull).to_string(),
                branches: branches(lang, g),
                how_it_helps: how_it_helps(g, kind, Some(life)),
            }
        })
        .collect();

    let first = &nodes[0];
    let headline = if light {
        match lang {
            Lang::En => format!("Nothing serious weighs on your {} now. These keep it that way.", life_lower(lang, life)),
            Lang::Ml => format!("ഇപ്പോൾ നിങ്ങളുടെ {} എന്നതിൽ വലിയ ഭാരമൊന്നുമില്ല. ഇവ അത് അങ്ങനെ നിലനിർത്തും.", life_name(lang, life)),
            Lang::Hi => format!("अभी आपके {} पर कोई बड़ा बोझ नहीं है। ये इसे ऐसा ही बनाए रखेंगे।", life_name(lang, life)),
        }
    } else {
        let p = lang::pressure(lang, first.key);
        let ease = first.until.map(|u| date(lang, u, lon));
        match lang {
            Lang::En => format!(
                "What weighs on your {} now: {}{}. Here is the cause, and what eases it: deeds anyone can do come first.",
                life_lower(lang, life),
                p,
                ease.map(|d| format!(", easing around {d}")).unwrap_or_default()
            ),
            Lang::Ml => format!(
                "നിങ്ങളുടെ {} {} എന്നിവയാൽ സമ്മർദ്ദത്തിലാണ്{}. കാരണവും അത് കുറയ്ക്കുന്നവയും താഴെ: ആർക്കും ചെയ്യാവുന്ന പ്രവൃത്തികൾ ആദ്യം.",
                life_name(lang, life),
                p,
                ease.map(|d| format!("; {d}-ഓടെ അയയും")).unwrap_or_default()
            ),
            Lang::Hi => format!(
                "आपके {} पर {} का दबाव है{}। नीचे कारण है, और जो इसे कम करे: पहले वे काम जो कोई भी कर सकता है।",
                life_name(lang, life),
                p,
                ease.map(|d| format!(", जो {d} के आसपास हल्का होगा")).unwrap_or_default()
            ),
        }
    };

    Ok(RemedyTree { life, headline, causes: nodes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;

    #[test]
    fn every_influence_has_every_branch_and_no_astrology_words() {
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for l in DashaLord::ORDER {
                let b = branches(lang, l);
                let count = |k: &str| b.iter().find(|x| x.key == k).map(|x| x.items.len()).unwrap_or(0);
                assert!(count("deeds") >= 3, "{l:?} deeds");
                assert!(count("giving") >= 2, "{l:?} giving");
                assert!(count("habits") >= 1 && count("nature") >= 1 && count("mind") >= 1 && count("prayer") >= 1);
                assert!(count("tradition") >= 3);
                for br in &b {
                    for s in [&br.title, &br.intro] {
                        assert_eq!(jargon_in(lang, s), None, "{lang:?} {l:?}: {s}");
                    }
                    for it in &br.items {
                        for s in [&it.title, &it.why, &it.how, &it.day, &it.effort] {
                            assert_eq!(jargon_in(lang, s), None, "{lang:?} {l:?}: {s}");
                        }
                    }
                }
                let inf = influence(l);
                assert_eq!(jargon_in(lang, lang.pick(inf.effects)), None, "{lang:?} {l:?} effects");
            }
        }
    }
}

/// The chart reason behind a cause and its remedies, for the person who taps "How this
/// helps": which planet presses the life, why, and why each kind of remedy eases it.
/// Names the planet on purpose; the common screens show it only on request.
pub fn how_it_helps(g: DashaLord, source: &str, life: Option<Life>) -> String {
    let (name, stands, people, day, gift, inner, deity) = match g {
        DashaLord::Sun => ("the Sun", "the self, the father, authority and government", "your father, seniors and those in authority", "Sunday", "wheat, jaggery or copper", "pride and the need to be seen", "Surya (or Shiva), with the Aditya Hridayam"),
        DashaLord::Moon => ("the Moon", "the mind, the mother, home and feelings", "your mother, older women and anyone who nurses others", "Monday", "rice, milk or white cloth", "the restless, worried mind", "Shiva and Parvati"),
        DashaLord::Mars => ("Mars", "energy, courage, conflict, brothers and property", "brothers, soldiers, police and people who work with fire and tools", "Tuesday", "red lentils or jaggery", "anger and haste", "Hanuman, or Murugan (Subrahmanya)"),
        DashaLord::Mercury => ("Mercury", "speech, learning, trade and paperwork", "students, young people, traders and your mother's brothers", "Wednesday", "green gram, green cloth or books", "scattered attention and careless words", "Vishnu"),
        DashaLord::Jupiter => ("Jupiter", "wisdom, teachers, children, faith and wealth", "teachers, elders, priests and children", "Thursday", "turmeric, chana dal, yellow cloth or books", "over-confidence and broken promises", "Dakshinamurthy (or Vishnu)"),
        DashaLord::Venus => ("Venus", "love, marriage, comfort and beauty", "your wife or husband, women and artists", "Friday", "white sweets, rice, curd or clothes", "excess and indulgence", "Lakshmi"),
        DashaLord::Saturn => ("Saturn", "work, duty, delay, discipline and the fruit of past deeds (karma)", "the old, the poor, labourers, servants and the disabled", "Saturday", "black sesame, mustard oil, iron or a dark blanket", "laziness, fear and cutting corners", "Shani, Hanuman or Ayyappa"),
        DashaLord::Rahu => ("Rahu (the Moon's north node)", "ambition, confusion, foreign things and sudden turns", "outcasts, foreigners, the forgotten and people fighting addiction", "Saturday", "a coconut, or black and blue cloth", "craving and confusion", "Durga"),
        DashaLord::Ketu => ("Ketu (the Moon's south node)", "letting go, loss, detachment and the inner life", "monks, the sick, and stray dogs", "Tuesday", "a blanket, or food for dogs", "withdrawal and sudden loss of heart", "Ganesha"),
    };
    let why_now = match source {
        "dasha" => format!("{name} is running a period of your life now (its chapter or sub-chapter), so what it stands for comes to the front."),
        "gochara" => format!("{name} is moving through a hard place counted from your birth Moon now, so it presses for a while and then moves on."),
        _ => format!("{name} sits weak or troubled in your birth chart, so it presses all your life and asks for steady care."),
    };
    let life_words = life.map(|l| l.name().to_lowercase()).unwrap_or_else(|| "life".to_string());
    format!(
        "Why this is happening: {why_now} {cap_name} stands for {stands}, and here it presses your {life_words}.\n\n\
         How the deeds help: in the remedial tradition every planet has its people. {cap_name}'s are {people}. Serving them is serving {name}: it is pleased and its pressure softens. That is why the deeds come first; they carry the work and need no faith.\n\n\
         How the giving helps: {gift}, given by hand on a {day} ({name}'s day), is the classical gift for {name}.\n\n\
         How the habits and quiet help: they work on the part of you that {name} rules, {inner}, so its pressure has less to grip.\n\n\
         How prayer helps: {deity} is the tradition's deity for {name}. In your own faith, any prayer on a {day} that holds these people in mind does the same work.",
        cap_name = cap(name),
    )
}

/// The language-free key of a tree remedy's effort, in any of the three languages:
/// `week`, `once`, `weekly`, `daily`, `10min`, `40`, `30`, `month`, `wish`, `3w`,
/// `monthly`; `once` for anything unknown. `remedy_timing::keep_for` reads it.
pub fn effort_key(effort: &str) -> &'static str {
    let table: [(&'static str, T3); 11] = [
        ("week", E_WEEK),
        ("once", E_ONCE),
        ("weekly", E_WEEKLY),
        ("daily", E_DAILY),
        ("10min", E_10),
        ("40", E_40),
        ("30", E_30),
        ("month", E_MONTH),
        ("wish", E_WISH),
        ("3w", E_3W),
        ("monthly", E_MONTHLY),
    ];
    table.iter().find(|(_, t)| t.contains(&effort)).map(|(k, _)| *k).unwrap_or("once")
}
