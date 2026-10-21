//! Understanding a question in the person's own words, offline, honestly.
//!
//! The app answers a known set of questions: 50 matters of life, each with six kinds of
//! question; the day, week, month, year and the years ahead; what helps; the fear that a
//! chart is bad; a good day for something; two people together; and a look back. A
//! question typed or spoken in English, Malayalam or Hindi (in their own scripts, or in
//! Latin letters as people type them: "kalyanam eppo", "shaadi kab hogi") is matched to
//! one of these by its cue words.
//!
//! The rule is: never answer a question that was not understood. When the match is
//! clear, the app answers and says how it understood the question. When it is unclear or
//! two readings tie, it says so and offers the closest questions it can answer. When the
//! question is outside what a reading of the sky can say (the day of a death, lottery
//! numbers, a baby's sex, a diagnosis, another person's private acts, a name, an exact
//! sum), it says so plainly and offers what it can answer instead. When the words carry
//! distress, it answers with care first.

use crate::forecast::Life;
use crate::topics::TOPICS;

/// What to do with the question.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Answer a question about a matter or a part of life.
    Ask {
        /// The part of life the question is about.
        life: Life,
        /// The matter within it, when one was recognised.
        topic: Option<&'static str>,
        /// The question as the app will put it.
        question: &'static str,
    },
    /// Show a part of life as it stands (its own page).
    Life(Life),
    /// The day, week, month or year.
    Outlook(&'static str),
    /// What is coming, in years.
    Future,
    /// What helps, for a life or for what weighs most.
    Remedy(Option<Life>),
    /// "Is something wrong in my chart?"
    ChartFear,
    /// Two people together.
    Us,
    /// A good day or hour for something.
    Plan(&'static str),
    /// A look back at the past.
    Past,
}

/// One question the app can answer, offered when it was not sure.
#[derive(Clone, Debug, PartialEq)]
pub struct Alternative {
    /// How it reads to the person.
    pub label: String,
    /// What it does.
    pub action: Action,
}

/// How the question was understood.
#[derive(Clone, Debug, PartialEq)]
pub struct Understanding {
    /// `answer` (understood), `choose` (not sure: pick one), `outside` (not something a
    /// reading can say), `care` (distress: care first).
    pub status: &'static str,
    /// What to do, when understood.
    pub action: Option<Action>,
    /// "Understood as: …" for an answer; the reply for the others.
    pub message: String,
    /// The closest questions the app can answer.
    pub alternatives: Vec<Alternative>,
    /// How sure the match is, `[0, 1]`.
    pub confidence: f64,
    /// A short key for why it was not answered (`unclear`, `tie`, `death`, `gamble`,
    /// `gender`, `medical`, `curse`, `other_mind`, `names`, `amount`, `lost`, `world`,
    /// `other_person`, `harm`), empty when answered.
    pub reason: &'static str,
}

// ---------------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------------

/// Lower case, letters and digits only, single spaces, padded with a space each side.
pub fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push(' ');
    let mut space = true;
    for ch in s.chars() {
        let keep = ch.is_alphanumeric() || is_mark(ch);
        if keep {
            for l in ch.to_lowercase() {
                out.push(l);
            }
            space = false;
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    if !space {
        out.push(' ');
    }
    out
}

/// Combining marks of Indic scripts (vowel signs, viramas) belong to their word.
fn is_mark(ch: char) -> bool {
    matches!(ch as u32, 0x0900..=0x097F | 0x0D00..=0x0D7F | 0x200C | 0x200D)
}

/// Whether `cue` occurs in normalised `t`. ASCII cues match whole words (a trailing `*`
/// matches any ending: `marr*` finds marry, married, marriage); other scripts match
/// anywhere, since Malayalam and Hindi join endings onto words.
fn has(t: &str, cue: &str) -> bool {
    if cue.is_ascii() {
        if let Some(stem) = cue.strip_suffix('*') {
            t.contains(&format!(" {stem}"))
        } else {
            t.contains(&format!(" {cue} "))
        }
    } else {
        t.contains(cue)
    }
}

fn any(t: &str, cues: &[&str]) -> bool {
    cues.iter().any(|c| has(t, c.trim_start_matches('~')))
}

fn count(t: &str, cues: &[&str]) -> f64 {
    // A cue starting with "~" is a weak one: it counts half, and never decides alone between two matters.
    cues.iter()
        .map(|c| match c.strip_prefix('~') { Some(w) => (w, 0.5), None => (*c, if c.contains(' ') || !c.is_ascii() { 1.5 } else { 1.0 }) })
        .filter(|(c, _)| has(t, c))
        .map(|(_, w)| w)
        .sum()
}

// ---------------------------------------------------------------------------------
// Cues
// ---------------------------------------------------------------------------------

/// Cue words for each matter. English, Malayalam (script and Latin), Hindi (script and Latin).
const TOPIC_CUES: &[(&str, &[&str])] = &[
    ("marriage", &["marr*", "wedding", "wed", "spouse", "bride", "groom", "life partner", "settle down", "get settled", "kalyan*", "vivaha*", "vivah", "shaadi", "shadi", "kalyanam", "കല്യാണ", "വിവാഹ", "शादी", "विवाह", "ब्याह", "proposal", "alliance", "arranged"]),
    ("new_love", &["meet someone", "meet the one", "love life", "find love", "right person", "girlfriend", "boyfriend", "relationship", "someone special", "lover", "crush", "date someone", "pranay*", "premam", "pyar", "प्यार", "प्रेम", "പ്രണയ", "പ്രേമ", "soulmate", "partner"]),
    ("reunion", &["ex", "away for", "has been away", "reconcile", "contact me again", "call me again", "text me again", "blocked me", "staying away", "stays away", "away from me", "living separately", "living apart", "left the house", "come back", "comeback", "return to me", "back together", "get back", "reunite", "reconcil*", "patch up", "patch-up", "left me", "leave me", "she left", "he left", "walked out", "thirichu varum", "thirichu", "തിരിച്ചു", "മടങ്ങി", "വാപസ", "वापस", "लौट", "wapas", "vapas", "laut"]),
    ("second_marriage", &["second marriage", "remarr*", "marry again", "again marriage", "dusri shaadi", "doosri shadi", "രണ്ടാം വിവാഹ", "दूसरी शादी", "punar vivah"]),
    ("separation", &["marriage going to break", "marriage break", "break up", "breakup", "divorce", "separat*", "fight with wife", "fight with husband", "fights with my wife", "fights with my husband", "quarrel with wife", "quarrel with husband", "marriage problem", "marriage trouble", "marriage issues", "wife fight*", "husband fight*", "talaq", "talak", "vivahamochana", "വിവാഹമോചന", "तलाक", "अलग", "vazhakku"]),
    ("in_laws", &["in law", "in laws", "inlaw*", "mother in law", "father in law", "sasural", "saas", "ammayiamma", "അമ്മായിയമ്മ", "ससुराल", "सास"]),
    ("new_job", &["jobless", "leave my job", "leave my company", "quit", "applied", "job", "jobs", "resign", "new company", "quit my job", "switch job", "change job", "change my job", "employment", "employed", "unemployed", "work offer", "offer letter", "placement", "hired", "get work", "joli", "naukri", "naukari", "jolikku", "ജോലി", "नौकरी", "रोजगार", "रोज़गार", "career start"]),
    ("promotion", &["promot*", "raise", "hike", "increment", "salary increase", "pay rise", "appraisal", "next level", "sthanakkayattam", "സ്ഥാനക്കയറ്റ", "प्रमोशन", "तरक्की", "taraqqi", "tarakki"]),
    ("transfer", &["transfer", "posting", "relocat*", "sthalamattam", "സ്ഥലംമാറ്റ", "तबादला", "ट्रांसफर", "tabadla"]),
    ("own_business", &["business", "startup", "start up", "own company", "shop", "self employ*", "entrepreneur*", "venture", "kachavadam", "ബിസിനസ", "കച്ചവട", "व्यापार", "व्यवसाय", "बिज़नेस", "dhandha", "dukan", "vyapar"]),
    ("government_post", &["government job", "govt job", "govt", "psc", "upsc", "ias", "ips", "civil service*", "sarkari", "sarkar naukri", "bank exam job", "railway job", "sarkkar joli", "സർക്കാർ ജോലി", "സർക്കാർ", "सरकारी"]),
    ("business_partner", &["business partner", "partnership", "co founder", "cofounder", "pankali", "പങ്കാളി", "साझेदार", "sajhedar"]),
    ("foreign_job", &["job abroad", "abroad job", "foreign job", "gulf job", "job in gulf", "job in dubai", "work abroad", "overseas job", "videsha joli", "വിദേശ ജോലി", "गल्फ", "विदेश में नौकरी", "videsh naukri"]),
    ("court_case", &["kesil", "kesu", "court", "case", "lawsuit", "legal", "litigat*", "dispute", "police case", "verdict", "judgement", "judgment", "kesu", "കേസ", "കോടതി", "कोर्ट", "मुकदमा", "केस", "mukadma", "adalat"]),
    ("loan", &["loan", "mortgage", "credit approv*", "vaypa", "വായ്പ", "ലോൺ", "लोन", "ऋण", "karza milega"]),
    ("debt_cleared", &["debt", "debts", "repay", "pay off", "clear my loan", "close my loan", "owe", "creditor*", "kadam", "kadam theerum", "കടം", "कर्ज", "क़र्ज़", "karz", "karza", "udhaar", "udhar"]),
    ("investment", &["invest*", "stock*", "share market", "shares", "mutual fund*", "crypto", "trading", "nikshepam", "നിക്ഷേപ", "निवेश", "nivesh"]),
    ("inheritance", &["inherit*", "ancestral property", "property share", "will of my father", "paitrikam", "പൈതൃക", "विरासत", "पैतृक", "virasat"]),
    ("windfall", &["windfall", "sudden money", "unexpected money", "big money", "rich", "wealthy", "lottery win"]),
    ("sale", &["sell my", "selling my", "sale of", "buyer for", "vilkkan", "വിൽക്ക", "बेच", "bech"]),
];

/// The second half of the matters (kept apart only to keep lines readable).
const TOPIC_CUES_2: &[(&str, &[&str])] = &[
    ("recovery", &["ill", "~recover*", "cure", "cured", "~heal", "~healing", "~healed", "get well", "~get better", "illness", "disease", "sick", "unwell", "asukham", "rogam", "sukhamakum", "അസുഖ", "രോഗ", "സുഖമാകു", "बीमार", "ठीक हो", "theek ho", "bimari", "rog"]),
    ("surgery", &["surgery", "operation", "operated", "transplant", "procedure", "shasthrakriya", "ശസ്ത്രക്രിയ", "ഓപ്പറേഷൻ", "ऑपरेशन", "सर्जरी"]),
    ("peace_of_mind", &["peace of mind", "peace", "anxiety", "anxious", "stress*", "tension", "worried", "worry", "depress*", "mental", "sleep", "can t sleep", "manasamadhanam", "samadhanam", "മനസ്സമാധാന", "സമാധാന", "ടെൻഷൻ", "शांति", "तनाव", "चिंता", "shanti", "chinta"]),
    ("mother_health", &["mother s health", "mothers health", "my mother health", "mom health", "amma health", "ammayude asukham", "അമ്മയുടെ ആരോഗ്യ", "അമ്മയ്ക്ക് അസുഖ", "माँ की तबीयत", "माँ की सेहत", "maa ki tabiyat", "mummy ki tabiyat"]),
    ("father_health", &["father s health", "fathers health", "my father health", "dad health", "achan health", "achante asukham", "അച്ഛന്റെ ആരോഗ്യ", "അച്ഛന് അസുഖ", "पिता की तबीयत", "पापा की तबीयत", "papa ki tabiyat"]),
    ("home_peace", &["peace at home", "home peace", "fights at home", "family fight*", "family problem*", "family issue*", "family tension", "veettil prashnam", "veetil prashnam", "വീട്ടിൽ പ്രശ്ന", "घर में झगड़ा", "घर में अशांति", "ghar me jhagda", "ghar mein jhagda", "gharelu"]),
    ("sibling", &["brother", "sister", "sibling*", "chettan", "chechi", "aniyan", "aniyathi", "ചേട്ടൻ", "ചേച്ചി", "അനിയ", "भाई", "बहन", "bhai", "behen", "bahan"]),
    ("mother", &["my mother", "my mom", "amma", "maa", "mummy", "അമ്മ", "माँ", "मम्मी"]),
    ("father", &["my father", "my dad", "achan", "papa", "pitaji", "അച്ഛ", "पिता", "पापा"]),
    ("moving_home", &["shift to my new house", "shift to new house", "move into", "shift to", "move house", "moving house", "shift house", "shifting house", "new place to live", "relocate home", "veedu maaran", "വീട് മാറ", "घर बदल", "ghar badal"]),
    ("exam", &["exam", "exams", "test result", "pass", "fail", "marks", "board exam", "pareeksha", "പരീക്ഷ", "परीक्षा", "pariksha", "imtihan"]),
    ("admission", &["admission", "college seat", "seat", "university", "get into", "praveshanam", "പ്രവേശന", "അഡ്മിഷൻ", "दाखिला", "प्रवेश", "admission milega"]),
    ("competitive_exam", &["neet", "jee", "cat exam", "gate", "competitive exam", "entrance", "entrance exam", "ssc", "bank exam", "എൻട്രൻസ", "प्रतियोगी परीक्षा"]),
    ("study_abroad", &["videsh me padhai", "padhai videsh", "study abroad", "studies abroad", "masters abroad", "ms abroad", "phd abroad", "student visa", "videshathu padikkan", "വിദേശത്ത് പഠ", "विदेश में पढ़ाई", "videsh padhai"]),
    ("scholarship", &["scholarship", "fellowship", "stipend", "സ്കോളർഷിപ്പ", "छात्रवृत्ति"]),
    ("buy_house", &["sontham veedu", "swantham veedu", "own a house", "~खरीद", "buy a house", "buy house", "buy land", "buying a house", "buy property", "buy a flat", "own house", "own home", "veedu vangan", "sthalam vangan", "വീട് വാങ്ങ", "സ്ഥലം വാങ്ങ", "घर खरीद", "ज़मीन खरीद", "ghar kharid", "zameen kharid", "makaan"]),
    ("build_house", &["build a house", "build house", "building a house", "construction", "construct", "griha pravesh", "veedu panayan", "veedu paniyan", "വീട് പണി", "घर बनाना", "निर्माण", "ghar banana"]),
    ("sell_house", &["~sell", "land sell", "house sell", "property sell", "sell house", "sell my house", "sell land", "sell my land", "sell property", "sell the flat", "veedu vilkkan", "വീട് വിൽക്ക", "घर बेच", "ghar bech"]),
    ("rent", &["rent", "rented", "tenant", "landlord", "vaadaka", "വാടക", "किराया", "kiraya"]),
    ("vehicle", &["car", "bike", "vehicle", "scooter", "vandi", "വണ്ടി", "കാർ", "गाड़ी", "gaadi", "gadi"]),
    ("visa", &["visa", "work permit", "green card", "pr ", "permanent residen*", "h1b", "വിസ", "वीज़ा", "वीजा"]),
    ("journey_abroad", &["trip abroad", "travel abroad", "foreign trip", "go abroad", "go to abroad", "flight", "videsha yathra", "വിദേശ യാത്ര", "विदेश यात्रा", "videsh yatra"]),
    ("settle_abroad", &["settle abroad", "settle in", "migrate", "immigrat*", "emigrat*", "move abroad", "canada", "australia", "uk", "usa", "america", "germany", "europe", "videsathu settle", "വിദേശത്ത് സ്ഥിര", "विदेश में बस", "videsh me bas"]),
    ("pilgrimage", &["pilgrimage", "sabarimala", "kashi", "haj", "hajj", "umrah", "tirupati", "theertha*", "തീർത്ഥ", "തീർഥ", "तीर्थ", "yatra"]),
    ("return_home", &["return to my country", "back to my country", "home country", "native place", "come back home", "return home", "return to india", "back to india", "back to kerala", "nattil", "naattil", "naattilekku", "നാട്ടിൽ", "നാട്ടിലേക്ക", "घर वापसी", "वतन", "desh wapas"]),
    ("child_birth", &["kuttikal", "kutti undakum", "pregnan*", "conceive", "conception", "baby", "child birth", "have a child", "have children", "have kids", "kunj*", "garbh*", "ivf", "santhanam", "കുഞ്ഞ", "ഗർഭ", "സന്താന", "गर्भ", "संतान", "बच्चा होगा", "bachcha", "santan", "aulad", "औलाद"]),
    ("child_marriage", &["son s marriage", "daughter s marriage", "son marriage", "daughter marriage", "my son marry", "my daughter marry", "makante kalyanam", "makalude kalyanam", "മകന്റെ കല്യാണ", "മകളുടെ കല്യാണ", "बेटे की शादी", "बेटी की शादी", "bete ki shadi", "beti ki shadi"]),
    ("child_studies", &["son s studies", "daughter s studies", "child s studies", "child studies", "kids studies", "my son exam", "my daughter exam", "makante padippu", "makalude padippu", "മകന്റെ പഠ", "മകളുടെ പഠ", "बच्चे की पढ़ाई", "बेटे की पढ़ाई", "बेटी की पढ़ाई", "bachche ki padhai"]),
    ("child_health", &["son s health", "daughter s health", "child s health", "child health", "baby health", "kids health", "makante asukham", "makalude asukham", "കുട്ടിയുടെ ആരോഗ്യ", "മകന് അസുഖ", "മകൾക്ക് അസുഖ", "बच्चे की तबीयत", "bachche ki tabiyat"]),
    ("adoption", &["adopt*", "dathu", "ദത്ത", "गोद"]),
];

/// The part of life, when no single matter is named.
const LIFE_CUES: &[(Life, &[&str])] = &[
    (Life::Love, &["love", "romance", "romantic", "wife", "husband", "partner", "marri*", "relationship", "bharya", "bharthavu", "bhartav*", "ഭാര്യ", "ഭർത്താവ", "പ്രണയ", "पत्नी", "पति", "प्रेम", "patni", "pati", "biwi"]),
    (Life::Career, &["career", "work", "office", "boss", "profession", "colleague*", "ജോലി", "उद्योग", "करियर", "kaam", "काम", "joli"]),
    (Life::Money, &["money", "finance*", "financial", "income", "salary", "wealth", "cash", "earning*", "profit", "loss", "panam", "paisa", "paise", "dhan", "പണ", "സാമ്പത്തിക", "पैसा", "पैसे", "धन"]),
    (Life::Health, &["health", "healthy", "body", "energy", "illness", "arogyam", "aarogyam", "ആരോഗ്യ", "स्वास्थ्य", "सेहत", "sehat", "tabiyat"]),
    (Life::Family, &["family", "home life", "parents", "relatives", "kudumbam", "veedu", "കുടുംബ", "परिवार", "parivar", "ghar wale"]),
    (Life::Study, &["study", "studies", "studying", "education", "college", "school", "course", "degree", "padippu", "padanam", "പഠന", "പഠിപ്പ", "पढ़ाई", "शिक्षा", "padhai"]),
    (Life::Home, &["house", "home", "land", "property", "flat", "apartment", "plot", "veed*", "sthalam", "വീട", "സ്ഥലം", "घर", "ज़मीन", "जमीन", "संपत्ति", "ghar", "zameen", "jameen"]),
    (Life::Travel, &["travel", "trip", "journey", "abroad", "foreign", "overseas", "yathra", "videsham", "യാത്ര", "വിദേശ", "यात्रा", "विदेश", "videsh", "safar"]),
    (Life::Children, &["child", "children", "kid", "kids", "son", "daughter", "makan", "makal", "kutti", "മകൻ", "മകൾ", "കുട്ടി", "बच्चे", "बच्चा", "बेटा", "बेटी", "beta", "beti", "bachche"]),
];

/// The kind of question.
const Q_CUES: &[(&str, &[&str])] = &[
    ("when", &["when", "how long", "how soon", "which month", "which year", "what year", "by when", "till when", "until when", "how many years", "how many months", "eppol", "eppo", "ennu", "eppozha", "എപ്പോൾ", "എപ്പോ", "എന്ന്", "എത്ര കാലം", "कब", "kab", "कितने समय", "kitne samay", "kitna time"]),
    ("will_it_happen", &["will i", "will it", "will we", "will my", "be approved", "approved", "come through", "go through", "succeed", "success", "work out", "aakumo", "avumo", "pattumo", "kazhiyumo", "sadhikkumo", "ho payega", "ho payegi", "payega", "payegi", "will there be", "is there a chance", "chance of", "chances", "possible", "possibility", "is it possible", "kittumo", "nadakkumo", "undakumo", "ആകുമോ", "കിട്ടുമോ", "നടക്കുമോ", "ഉണ്ടാകുമോ", "होगा", "होगी", "मिलेगा", "मिलेगी", "hoga", "hogi", "milega", "milegi"]),
    ("should_i", &["should i", "shall i", "should we", "shall we", "should my", "is it wise", "is it right to", "go ahead", "is it better to", "must i", "venamo", "cheyyano", "ചെയ്യണോ", "വേണോ", "करूँ", "करना चाहिए", "chahiye", "karun"]),
    ("good_time_now", &["good time", "right time", "time right", "is the time", "right moment", "now", "today", "this week", "this month", "is this the time", "nalla samayam", "ippol", "നല്ല സമയ", "ഇപ്പോൾ", "सही समय", "अच्छा समय", "अभी", "abhi", "sahi samay"]),
    ("will_it_return", &["come back", "comeback", "return", "get back", "back to me", "recover my money", "get my money back", "thirichu", "madangi", "തിരിച്ചു", "മടങ്ങ", "वापस", "लौट", "wapas", "vapas"]),
    ("will_it_recover", &["set right", "get better", "improve", "recover", "heal", "end", "be fixed", "solve*", "sorted", "okay again", "ok again", "sheriyakumo", "maarumo", "ശരിയാകു", "മാറുമോ", "മെച്ചപ്പെ", "ठीक", "सुधर", "theek", "sudhar", "khatam"]),
];

/// The purposes a good day can be found for.
const PURPOSE_CUES: &[(&str, &[&str])] = &[
    ("interview", &["interview", "pitch", "appraisal meeting"]),
    ("exam", &["exam", "test", "pareeksha", "परीक्षा"]),
    ("journey", &["journey", "travel", "trip", "flight", "yathra", "യാത്ര", "यात्रा"]),
    ("deal", &["deal", "sign", "signing", "contract", "agreement", "purchase", "buy", "kararu", "കരാർ", "सौदा"]),
    ("doctor", &["doctor", "surgery", "operation", "treatment", "hospital", "ഡോക്ടർ", "ऑपरेशन", "इलाज"]),
    ("date", &["date", "propose", "proposal", "first meeting", "meet her", "meet him", "pennu kanal", "pennukanal", "പെണ്ണുകാണ", "रिश्ता देखने"]),
    ("family", &["function", "ceremony", "naming", "engagement", "nischayam", "നിശ്ചയ", "सगाई", "family event", "griha pravesh", "house warming", "housewarming", "palukachu", "പാലുകാച്ച"]),
    ("property", &["register", "registration", "property", "land", "house", "flat", "आधार"]),
    ("launch", &["launch", "open shop", "start", "shop", "opening", "start business", "inaugurat*", "udhghadanam", "ഉദ്ഘാടന", "उद्घाटन", "shuru"]),
    ("meeting", &["meeting", "meet", "talk to", "conversation"]),
];

const SPAN_CUES: &[(&str, &[&str])] = &[
    ("day", &["today", "tonight", "this morning", "this evening", "today s", "innu", "innathe", "ഇന്ന്", "ഇന്നത്തെ", "आज", "aaj"]),
    ("week", &["this week", "next week", "coming week", "ee aazhcha", "ee azhcha", "ഈ ആഴ്ച", "इस हफ्ते", "इस हफ़्ते", "is hafte", "is week"]),
    ("month", &["this month", "next month", "coming month", "ee maasam", "ഈ മാസം", "इस महीने", "is mahine"]),
    ("year", &["this year", "next year", "coming year", "2026", "2027", "ee varsham", "ee kollam", "ഈ വർഷം", "ഈ കൊല്ലം", "इस साल", "अगले साल", "is saal", "agle saal"]),
    ("future", &["aage kya", "aage", "future", "years ahead", "coming years", "next few years", "rest of my life", "long term", "bhaavi", "bhavi", "bhavishya", "bhavishyam", "ഭാവി", "भविष्य"]),
];

const REMEDY_CUES: &[&str] = &["remed*", "parihar*", "pariharam", "upay", "upaay", "upaya", "solution", "what should i do", "what can i do", "how to fix", "how to solve", "how to reduce", "pooja", "puja", "mantra", "vazhipad*", "പരിഹാര", "വഴിപാട", "उपाय", "पूजा", "totka", "what to do"];
const CHART_FEAR_CUES: &[&str] = &["dosha", "dosham", "dosh", "sade sati", "sadesati", "sadhe sathi", "ezhara*", "ezharasani", "kandaka*", "kandakasani", "ashtama*", "manglik", "mangalik", "chovva", "kuja dosha", "bad time", "bad period", "bad phase", "bad luck", "kashta kalam", "kashtakalam", "dasha", "shani", "sani", "saturn", "rahu", "ketu", "is my chart bad", "chart is bad", "horoscope is bad", "jathakam", "jathaka dosham", "ദോഷ", "ഏഴരശ്ശനി", "ഏഴര", "കണ്ടകശ്ശനി", "അഷ്ടമശ്ശനി", "ചൊവ്വാദോഷ", "കഷ്ടകാല", "साढ़ेसाती", "साढ़े साती", "मांगलिक", "दोष", "बुरा समय", "bura samay", "kharab samay"];
const US_CUES: &[&str] = &["happy together", "compatib*", "matching", "horoscope match", "porutham", "kundli milan", "kundali milan", "gun milan", "are we compatible", "our relationship", "between us", "പൊരുത്ത", "कुंडली मिलान", "गुण मिलान"];
const PLAN_CUES: &[&str] = &["good day", "good date", "best day", "best date", "auspicious", "muhurat*", "muhurtham", "muhoortham", "shubh", "which day", "what day", "which date", "what date", "good hour", "nalla divasam", "nalla samayam", "മുഹൂർത്ത", "നല്ല ദിവസ", "शुभ मुहूर्त", "मुहूर्त", "अच्छा दिन", "शुभ दिन"];
const PAST_STRONG: &[&str] = &["why did", "why was", "what went wrong", "what happened", "happened in", "enthu patti", "എന്തുപറ്റി", "क्या हुआ था"];
const PAST_CUES: &[&str] = &["last year", "in the past", "back then", "my past", "previous year", "पिछले साल"];

// Questions a reading of the sky cannot answer, and what to say.
const HARM_CUES: &[&str] = &["ending my life", "end my life", "take my life", "kill my self", "no point living", "no point in living", "better off dead", "can t go on", "cannot go on", "don t want to be alive", "kill myself", "suicide", "suicidal", "end my life", "end it all", "want to die", "don t want to live", "dont want to live", "no reason to live", "self harm", "hurt myself", "marikkanam", "ജീവിക്കേണ്ട", "ആത്മഹത്യ", "मरना चाहता", "मरना चाहती", "आत्महत्या", "jeena nahi"];
const DEATH_CUES: &[&str] = &["pass away", "passed away", "passing away", "expire", "live till", "live up to", "live upto", "live until", "how old will i be when", "when will i die", "when will he die", "when will she die", "death date", "date of death", "my death", "die", "dying", "how long will i live", "lifespan", "life span", "age of death", "maranam", "marikkum", "മരണ", "മരിക്കും", "मृत्यु", "मौत", "मरूंगा", "maut", "mrityu"];
const GAMBLE_CUES: &[&str] = &["dream11", "dream 11", "fantasy team", "rummy", "poker", "teen patti", "betting app", "lucky number", "lottery number", "lottery", "jackpot", "lucky number", "betting", "bet on", "gamble", "gambling", "satta", "matka", "which stock", "stock tip", "which share", "crypto tip", "number will come", "ലോട്ടറി", "लॉटरी", "सट्टा"];
const GENDER_CUES: &[&str] = &["boy or a girl", "ladka hoga ya ladki", "ladka ya ladki", "ladki hogi ya ladka", "beta ya beti", "boy or girl", "girl or boy", "baby boy or", "male or female baby", "gender of baby", "baby gender", "son or daughter", "aano pennano", "aankutti", "ആൺകുട്ടിയോ", "लड़का या लड़की", "ladka ya ladki", "beta hoga ya beti"];
const MEDICAL_CUES: &[&str] = &["antibiotic*", "injection", "insulin", "should i take this", "take this medicine", "dose", "which medicine", "what medicine", "which tablet", "dosage", "diagnos*", "do i have cancer", "is it cancer", "test report", "blood report", "which doctor should", "should i stop medicine", "stop my medicine", "मरुन्न", "ഏത് മരുന്ന", "कौन सी दवा", "dawa"];
const CURSE_CUES: &[&str] = &["black magic", "blackmagic", "witchcraft", "evil eye", "curse", "cursed", "who did", "someone did something", "koodothram", "kooodothram", "aabhicharam", "ചാത്തൻ", "കൂടോത്ര", "ആഭിചാര", "काला जादू", "टोना", "jadu tona", "kala jadu", "nazar"];
const OTHER_MIND_CUES: &[&str] = &["seeing someone else", "someone else", "cheat me", "betray*", "lying to me", "lie to me", "faithful", "two timing", "loyal to me", "cheating", "cheat on me", "affair", "is she loyal", "is he loyal", "does she love me", "does he love me", "does she still love", "does he still love", "what is she thinking", "what is he thinking", "other man", "other woman", "another man", "another woman", "dhoka", "धोखा", "चक्कर", "chakkar", "vanchana", "വഞ്ചന", "അവിഹിത"];
const NAMES_CUES: &[&str] = &["look like", "looks like", "appearance", "complexion", "how tall", "his face", "her face", "name of my", "what is the name", "first letter of", "starting letter", "who will i marry", "who is my soulmate", "what will be the name", "pere entha", "പേര് എന്ത", "नाम क्या", "किससे होगी शादी", "kisse hogi"];
const AMOUNT_CUES: &[&str] = &["dowry", "how much", "how many lakh*", "how many rupees", "how much money", "how much salary", "how much will i earn", "how many lakhs", "how many crores", "exact amount", "what will be my salary", "how much profit", "ethra panam", "എത്ര പണ", "कितना पैसा", "कितनी सैलरी", "kitna paisa"];
const LOST_CUES: &[&str] = &["where is my", "where did i keep", "missing item", "stolen", "who stole", "theft", "kaanathayi", "കാണാതായ", "മോഷണ", "चोरी", "खो गया", "kho gaya"];
const WORLD_CUES: &[&str] = &["election", "who will win the match", "cricket", "ipl", "world cup", "ipl", "weather", "rain tomorrow", "share market crash", "stock market crash", "war", "pm of india", "prime minister", "chief minister", "politics", "തിരഞ്ഞെടുപ്പ", "चुनाव", "मैच कौन जीतेगा"];
// Someone else, when the question is about their own life (their birth details are needed).
const OTHER_PEOPLE: &[&str] = &["my friend", "my friend s", "friend s", "my neighbour", "my neighbor", "my cousin", "my uncle", "my aunt", "my boyfriend s", "my girlfriend s", "her job", "his job", "her exam", "his exam", "her marriage", "his marriage", "her visa", "his visa", "her health", "his health", "my sister s job", "my brother s job", "my sister s marriage", "my brother s marriage", "my sister s exam", "my brother s exam", "my wife s job", "my husband s job", "suhruth*", "കൂട്ടുകാര", "दोस्त", "dost"];

// ---------------------------------------------------------------------------------
// Words for the person
// ---------------------------------------------------------------------------------

fn q_words(q: &str) -> &'static str {
    match q {
        "when" => "when?",
        "will_it_happen" => "will it happen?",
        "should_i" => "should I go ahead?",
        "good_time_now" => "is now a good time?",
        "will_it_return" => "will it come back?",
        _ => "will it be set right?",
    }
}

fn topic_words(key: &str) -> &'static str {
    TOPICS.iter().find(|t| t.key == key).map(|t| t.words).unwrap_or("this")
}

fn topic_life(key: &str) -> Life {
    TOPICS.iter().find(|t| t.key == key).map(|t| t.life).unwrap_or(Life::Love)
}

fn ask_alt(topic: &'static str, q: &'static str) -> Alternative {
    Alternative { label: format!("{}: {}", topic_words(topic), q_words(q)), action: Action::Ask { life: topic_life(topic), topic: Some(topic), question: q } }
}

fn general_alternatives() -> Vec<Alternative> {
    vec![
        Alternative { label: "How is my day today?".into(), action: Action::Outlook("day") },
        Alternative { label: "What does this year hold for me?".into(), action: Action::Outlook("year") },
        ask_alt("marriage", "when"),
        ask_alt("new_job", "when"),
        Alternative { label: "How is money for me?".into(), action: Action::Life(Life::Money) },
        Alternative { label: "How is my health?".into(), action: Action::Life(Life::Health) },
        Alternative { label: "What is coming in the years ahead?".into(), action: Action::Future },
        Alternative { label: "What will help me now?".into(), action: Action::Remedy(None) },
        Alternative { label: "Is something wrong in my chart?".into(), action: Action::ChartFear },
    ]
}

fn life_alternatives(life: Life) -> Vec<Alternative> {
    let mut v: Vec<Alternative> = TOPICS.iter().filter(|t| t.life == life).take(4).map(|t| ask_alt(t.key, if t.key == "recovery" { "will_it_recover" } else { "when" })).collect();
    v.push(Alternative { label: format!("How is {} for me now?", life.name().to_lowercase()), action: Action::Life(life) });
    v.push(Alternative { label: format!("What will help my {}?", life.name().to_lowercase()), action: Action::Remedy(Some(life)) });
    v
}

const OFFLINE_NOTE: &str = "Kshana works offline: every answer comes from your birth time and the sky, worked out on this phone, for a set list of questions.";

fn outside(reason: &'static str, message: &str, alternatives: Vec<Alternative>) -> Understanding {
    Understanding { status: "outside", action: None, message: format!("{message} {OFFLINE_NOTE}"), alternatives, confidence: 1.0, reason }
}

fn choose(reason: &'static str, message: &str, alternatives: Vec<Alternative>, confidence: f64) -> Understanding {
    Understanding { status: "choose", action: None, message: message.to_string(), alternatives, confidence, reason }
}

fn answer(action: Action, message: String, confidence: f64) -> Understanding {
    Understanding { status: "answer", action: Some(action), message, alternatives: Vec::new(), confidence, reason: "" }
}

// ---------------------------------------------------------------------------------
// Understanding
// ---------------------------------------------------------------------------------

// Word families used by the combination rules below.
const W_MOTHER: &[&str] = &["mother", "mom", "mum", "amma", "ammayude", "maa", "mummy", "mata", "അമ്മ", "माँ", "मम्मी", "माता"];
const W_FATHER: &[&str] = &["father", "dad", "daddy", "achan", "achante", "papa", "pitaji", "അച്ഛ", "पिता", "पापा"];
const W_CHILD: &[&str] = &["son", "daughter", "child", "kid", "kids", "makan", "makal", "makante", "makalude", "beta", "beti", "bete", "bachcha", "bachche", "മകൻ", "മകള", "മകന്റെ", "കുട്ടി", "बेटा", "बेटी", "बच्चे", "बच्चा"];
const W_SPOUSE: &[&str] = &["wife", "husband", "spouse", "partner", "bharya", "bharthavu", "patni", "pati", "biwi", "ഭാര്യ", "ഭർത്താവ", "पत्नी", "पति"];
const W_SIBLING: &[&str] = &["brother", "sister", "sibling", "siblings", "chettan", "chechi", "aniyan", "aniyathi", "bhai", "behen", "bahan", "ചേട്ട", "ചേച്ചി", "അനിയ", "भाई", "बहन"];
const W_HEALTH: &[&str] = &["health", "sick", "ill", "illness", "recover", "hospital", "surgery", "operation", "disease", "fever", "asukham", "asukhum", "tabiyat", "bimar", "bimari", "അസുഖ", "ആരോഗ്യ", "तबीयत", "बीमार", "सेहत"];
const W_STUDY: &[&str] = &["study", "studies", "studying", "exam", "exams", "marks", "school", "college", "padippu", "padhai", "പഠ", "पढ़ाई", "परीक्षा"];
const W_WED: &[&str] = &["marr*", "wedding", "kalyanam", "shadi", "shaadi", "vivaham", "കല്യാണ", "വിവാഹ", "शादी", "विवाह"];
const W_FIGHT: &[&str] = &["fight*", "quarrel*", "argu*", "jhagda", "jhagde", "vazhakku", "kalaham", "problems with", "trouble with", "issues with", "വഴക്ക", "കലഹ", "झगड़ा", "झगड़े"];
const W_ABROAD: &[&str] = &["abroad", "foreign", "overseas", "gulf", "dubai", "uae", "saudi", "qatar", "oman", "kuwait", "bahrain", "canada", "australia", "uk", "england", "london", "usa", "us", "america", "germany", "europe", "singapore", "new zealand", "ireland", "videsh*", "videsha*", "വിദേശ", "विदेश", "गल्फ"];
const W_JOB: &[&str] = &["job", "jobs", "work", "employment", "joli", "naukri", "naukari", "ജോലി", "नौकरी"];
const W_HOUSE: &[&str] = &["house", "flat", "home", "land", "plot", "property", "apartment", "villa", "veedu", "sthalam", "ghar", "makaan", "zameen", "jameen", "വീട", "സ്ഥലം", "घर", "ज़मीन", "जमीन", "मकान"];
const W_BUY: &[&str] = &["buy", "buying", "afford", "purchase", "own", "sontham", "swantham", "vangan", "vaanganam", "kharid*", "വാങ്ങ", "സ്വന്ത", "खरीद", "अपना"];
const W_LIVE: &[&str] = &["live", "living", "survive", "jeevikkum", "jeeyega", "जिएगा", "जिएगी", "ജീവിക്കും"];
const W_LONG: &[&str] = &["how long", "how many years", "till what age", "until what age", "what age", "ethra kaalam", "kitne saal", "kab tak"];
const W_SEX: &[&str] = &["boy", "girl", "male", "female", "aan", "penn", "ladka", "ladki", "beta hoga", "beti hogi", "ആൺ", "പെൺ", "लड़का", "लड़की"];
const W_BABY: &[&str] = &["baby", "pregnan*", "scan", "garbh*", "kunj*", "child", "ഗർഭ", "കുഞ്ഞ", "गर्भ", "बच्चा"];

/// Matters found by a combination of words rather than one phrase: a relative and a
/// health word is that relative's health; a job and a country is a job abroad.
fn combined(t: &str) -> Vec<(&'static str, f64)> {
    let mut v = Vec::new();
    let health = any(t, W_HEALTH);
    if any(t, W_MOTHER) && health {
        v.push(("mother_health", 3.0));
    }
    if any(t, W_FATHER) && health {
        v.push(("father_health", 3.0));
    }
    if any(t, W_CHILD) {
        if health {
            v.push(("child_health", 3.0));
        }
        if any(t, W_STUDY) {
            v.push(("child_studies", 3.0));
        }
        if any(t, W_WED) {
            v.push(("child_marriage", 3.0));
        }
    }
    if any(t, W_SPOUSE) && any(t, W_FIGHT) {
        v.push(("separation", 3.0));
    }
    if any(t, W_SIBLING) && (any(t, W_FIGHT) || any(t, &["peace", "talk", "speak", "make up", "patch"])) {
        v.push(("sibling", 3.0));
    }
    if any(t, W_JOB) && any(t, W_ABROAD) {
        v.push(("foreign_job", 3.0));
    }
    if any(t, &["masters", "ms", "phd", "mba", "university", "degree", "course", "study", "studies", "padhai", "padikkan"]) && any(t, W_ABROAD) {
        v.push(("study_abroad", 3.0));
    }
    if any(t, W_HOUSE) {
        if any(t, W_BUY) {
            v.push(("buy_house", 3.0));
        }
        if any(t, &["sell", "selling", "vilkkan", "bech*", "വിൽക്ക", "बेच"]) {
            v.push(("sell_house", 3.0));
        }
        if any(t, &["build", "building", "construct*", "paniyan", "panayan", "banana", "പണി", "बना"]) {
            v.push(("build_house", 3.0));
        }
        if (any(t, W_FATHER) || any(t, W_MOTHER) || any(t, &["ancestral", "parents", "family share", "share"])) && any(t, &["property", "land", "share", "sthalam", "zameen", "स्वत्व"]) {
            v.push(("inheritance", 3.0));
        }
    }
    if any(t, &["return", "come back", "go back", "back to"]) && any(t, &["my country", "home country", "native", "home town", "hometown", "india", "kerala", "nattil", "naattil", "naattilekku", "desh", "watan"]) {
        v.push(("return_home", 3.0));
    }
    v
}

/// Questions the reading cannot answer, found by a combination of words.
fn combined_outside(t: &str) -> Option<&'static str> {
    if (any(t, W_LIVE) && any(t, W_LONG)) || any(t, &["will die", "going to die", "die soon"]) {
        return Some("death");
    }
    if any(t, W_BABY) && any(t, W_SEX) {
        return Some("gender");
    }
    if any(t, &["crypto", "bitcoin", "coin", "share", "shares", "stock", "stocks", "nifty", "sensex"]) && any(t, &["which", "go up", "goes up", "price", "tip", "tips", "rise", "will rise"]) {
        return Some("gamble");
    }
    if any(t, &["report", "reports", "scan result", "test result", "blood test", "sugar level", "bp", "cholesterol", "x ray", "mri"]) && any(t, &["normal", "is my", "what does", "mean", "result", "ok", "fine", "bad"]) && !any(t, &["exam"]) {
        return Some("medical");
    }
    if any(t, &["tablet", "tablets", "medicine", "medicines", "dawa", "davai", "marunnu", "മരുന്ന", "दवा"]) && any(t, &["which", "what", "should i take", "for my", "for fever", "for pain"]) {
        return Some("medical");
    }
    if any(t, &["name", "names", "naam", "peru", "per", "letter", "initial", "पेर", "नाम", "പേര"]) && (any(t, W_SPOUSE) || any(t, &["future wife", "future husband", "life partner", "soulmate", "who will i marry", "girl", "boy", "person i marry"])) {
        return Some("names");
    }
    let things = ["ring", "chain", "phone", "mobile", "wallet", "purse", "bag", "keys", "key", "gold", "jewel*", "watch", "document*", "passport", "certificate*", "laptop", "bike", "car", "item", "thing", "things", "mala", "maala"];
    if any(t, &["misplaced", "where is my", "where did i keep", "where did i put"]) || (any(t, &["lose", "lost", "losing", "missing", "stolen", "kaanathayi", "kho"]) && any(t, &things)) {
        return Some("lost");
    }
    if any(t, &["does", "do", "is", "kya", "does he", "does she"]) && any(t, &["like me", "love me", "hate me", "care about me", "miss me", "think of me", "think about me", "trust me"]) {
        return Some("other_mind");
    }
    None
}

/// Words that carry no matter of their own: question words, pronouns, little joining
/// words, in English and as Malayalam and Hindi are typed in Latin letters.
const STOP: &[&str] = &[
    "a", "an", "the", "i", "me", "my", "mine", "we", "us", "our", "you", "your", "he", "she", "him", "her", "his", "they", "them", "their", "it", "its",
    "is", "am", "are", "was", "were", "be", "been", "being", "will", "would", "shall", "should", "can", "could", "may", "might", "must", "do", "does", "did",
    "have", "has", "had", "get", "got", "getting", "go", "going", "come", "coming", "make", "take", "give", "tell", "know", "want", "need", "please",
    "what", "when", "where", "which", "who", "why", "how", "whether", "if", "then", "than", "so", "and", "or", "but", "not", "no", "yes", "ok",
    "to", "for", "of", "in", "on", "at", "by", "with", "from", "about", "into", "over", "after", "before", "again", "ever", "still", "soon", "now", "any", "some", "this", "that", "these", "those", "there", "here",
    "very", "much", "many", "more", "most", "really", "just", "also", "all", "one", "time", "times", "year", "years", "month", "months", "week", "day", "days", "life", "good", "bad", "well",
    "ente", "enikku", "njan", "ningal", "undo", "aano", "aakumo", "eppol", "eppo", "ennu", "enthu", "ethu", "aaya", "aayi", "il", "nte", "kku", "ku",
    "mera", "meri", "mere", "mujhe", "main", "mai", "hum", "kya", "kab", "kyu", "kyon", "hai", "hain", "hoga", "hogi", "ho", "me", "mein", "ki", "ka", "ke", "se", "ko", "ne", "par", "bhi", "aur", "ya",
    "chance", "chances", "likely", "destiny", "fate", "luck", "lucky", "happen", "happens", "happening", "possible", "news", "idea", "right", "able", "try", "trying", "think", "feel", "hold", "store", "ahead", "place", "decide", "favour", "favor", "said", "say", "says", "improve", "affecting", "affect", "sanction", "sanctioned", "crack", "score", "high", "big", "heavy", "new", "old", "actually", "kind", "type", "able", "manage", "start", "finally", "anything", "something", "everything", "someone", "anyone", "work", "out", "up", "down", "back", "away", "long", "far", "next", "coming", "future", "check", "see", "find", "look", "looking", "interviewed", "applied", "going", "well", "better", "worse", "together", "alone", "family", "home", "selection", "result", "results", "status", "tak", "kaisa", "kaise", "kaisi", "rahega", "rahegi", "rahenge", "sakta", "sakti", "sakte", "payega", "payegi", "chalega", "milega", "milegi", "engane", "enthanu", "ano", "varum", "varumo", "kittum", "kittumo", "nadakkum", "nadakkumo", "aakum", "edukkum", "naal", "pattumo", "pattum", "undakumo", "undakum", "cheyyano", "venam", "venamo", "ethra", "jana", "pokan", "pokumo", "koi", "batao", "bataiye", "bata", "kerala", "india", "astrologer", "astrology", "jyotishi", "pandit", "panditji", "chart", "horoscope", "kundli", "kundali", "jathakam", "reading", "बताओ", "बताइए", "क्या", "कब", "है", "होगा", "होगी", "मेरा", "मेरी", "मेरे", "मुझे", "में", "की", "का", "के", "से", "को", "कैसा", "कैसी", "എന്റെ", "എനിക്ക്", "എന്ത്",
];

/// The share of the person's own meaningful words that the matched cues explain.
fn coverage(t: &str) -> f64 {
    let tables: Vec<&[&str]> = TOPIC_CUES.iter().chain(TOPIC_CUES_2.iter()).map(|x| x.1)
        .chain(LIFE_CUES.iter().map(|x| x.1))
        .chain(Q_CUES.iter().map(|x| x.1))
        .chain(SPAN_CUES.iter().map(|x| x.1))
        .chain(PURPOSE_CUES.iter().map(|x| x.1))
        .chain([REMEDY_CUES, CHART_FEAR_CUES, US_CUES, PLAN_CUES, PAST_STRONG, PAST_CUES, W_MOTHER, W_FATHER, W_CHILD, W_SPOUSE, W_SIBLING, W_HEALTH, W_STUDY, W_WED, W_FIGHT, W_ABROAD, W_JOB, W_HOUSE, W_BUY])
        .collect();
    let mut content = 0usize;
    let mut explained = 0usize;
    for w in t.split_whitespace() {
        if STOP.contains(&w) || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        content += 1;
        let hit = tables.iter().any(|tab| {
            tab.iter().any(|c| {
                let c = c.trim_start_matches('~');
                if !c.is_ascii() {
                    return w.contains(c) || c.contains(w) && w.chars().count() > 1;
                }
                if c.contains(' ') {
                    has(t, c) && c.split(' ').any(|p| p == w || p.strip_suffix('*').map_or(false, |s| w.starts_with(s)))
                } else if let Some(stem) = c.strip_suffix('*') {
                    w.starts_with(stem)
                } else {
                    c == w
                }
            })
        });
        if hit {
            explained += 1;
        }
    }
    if content == 0 {
        1.0
    } else {
        explained as f64 / content as f64
    }
}

/// Understand a question in the person's own words.
pub fn understand(text: &str) -> Understanding {
    let u = understand_with(text, true);
    // The gate: an answer is given only when most of the person's own words were
    // understood. Otherwise the app says it is not sure, and offers what it can answer
    // about the part it did understand.
    if u.status == "answer" {
        let t = normalise(text);
        let c = coverage(&t);
        let strong = matches!(&u.action, Some(Action::Ask { .. })) && u.confidence >= 0.9;
        if c < 0.5 && !(strong && c >= 0.34) {
            let mut alternatives = match &u.action {
                Some(Action::Ask { topic: Some(k), .. }) => vec![ask_alt(k, "when"), ask_alt(k, "will_it_happen"), ask_alt(k, "good_time_now")],
                Some(Action::Ask { life, .. }) | Some(Action::Life(life)) => life_alternatives(*life),
                _ => Vec::new(),
            };
            if let Some(a) = u.action.clone() {
                if !alternatives.iter().any(|x| x.action == a) {
                    alternatives.insert(0, Alternative { label: u.message.trim_start_matches("Understood as: ").to_string(), action: a });
                }
            }
            alternatives.extend(general_alternatives().into_iter().take(3));
            return choose(
                "unclear",
                "Kshana is not sure it understood all of your question, so it will not guess. It works offline, from your birth time and the sky, for a set list of questions. Is it one of these?",
                alternatives,
                c,
            );
        }
    }
    u
}

fn understand_with(text: &str, combos: bool) -> Understanding {
    let t = normalise(text);
    let words = t.split_whitespace().count();
    if words == 0 {
        return choose("unclear", "Type or say a question about your life, and Kshana will answer it from your birth time and the sky. Or pick one of these.", general_alternatives(), 0.0);
    }

    // 1. Care first.
    if any(&t, HARM_CUES) {
        return Understanding {
            status: "care",
            action: None,
            message: "It sounds like you are carrying something very heavy. Please talk to someone now: Tele-MANAS, the Government of India's free helpline, answers day and night in your language at 14416. If you are in danger, call 112. A hard season does pass, and Kshana can show you when this one eases.".into(),
            alternatives: vec![
                Alternative { label: "When does this hard time ease?".into(), action: Action::Ask { life: Life::Health, topic: Some("peace_of_mind"), question: "when" } },
                Alternative { label: "What will help me now?".into(), action: Action::Remedy(Some(Life::Health)) },
            ],
            confidence: 1.0,
            reason: "harm",
        };
    }

    // 2. What a reading of the sky cannot say.
    if combos {
        if let Some(r) = combined_outside(&t) {
            return refusal(r);
        }
    }
    if any(&t, GENDER_CUES) {
        return outside("gender", "Kshana does not foretell whether a baby will be a boy or a girl. No reading can say it truly, and in India asking it is against the law (the PCPNDT Act). It can tell you about the timing of a child.", vec![ask_alt("child_birth", "when"), ask_alt("child_health", "will_it_recover")]);
    }
    if any(&t, DEATH_CUES) && !has(&t, "die hard") {
        return outside("death", "Kshana never says how long a life will last, for you or for anyone. No one can know it, and it should not be said. It can tell you how health stands and when a hard stretch eases.", vec![Alternative { label: "How is my health?".into(), action: Action::Life(Life::Health) }, ask_alt("recovery", "will_it_recover"), ask_alt("mother_health", "will_it_recover"), ask_alt("father_health", "will_it_recover")]);
    }
    if any(&t, GAMBLE_CUES) {
        return outside("gamble", "Kshana does not give lottery numbers, betting tips or which share to buy. The sky does not hold numbers, and no honest reading will claim to. It can tell you how money stands and when it improves.", vec![Alternative { label: "How is money for me?".into(), action: Action::Life(Life::Money) }, ask_alt("investment", "good_time_now"), ask_alt("debt_cleared", "when")]);
    }
    if any(&t, MEDICAL_CUES) {
        return outside("medical", "Kshana cannot diagnose, read a report or choose a medicine; please ask your doctor, and never stop a treatment because of a reading. It can tell you how your health stands and good days for treatment.", vec![ask_alt("recovery", "will_it_recover"), ask_alt("surgery", "good_time_now"), Alternative { label: "A good day to see the doctor".into(), action: Action::Plan("doctor") }]);
    }
    if any(&t, CURSE_CUES) {
        return outside("curse", "Kshana does not look for black magic, curses or who might have done something to you; a reading of the sky cannot see another person's acts. It can show what is actually pressing on you now, when it ends, and what helps.", vec![Alternative { label: "Is something wrong in my chart?".into(), action: Action::ChartFear }, Alternative { label: "What will help me now?".into(), action: Action::Remedy(None) }, ask_alt("peace_of_mind", "when")]);
    }
    if any(&t, OTHER_MIND_CUES) {
        return outside("other_mind", "Kshana cannot see what another person is doing or feeling; that is theirs, not written in your chart. It can tell you how your marriage or relationship stands, and when it eases.", vec![ask_alt("separation", "will_it_recover"), ask_alt("reunion", "will_it_return"), Alternative { label: "How is love for me now?".into(), action: Action::Life(Life::Love) }, Alternative { label: "How do the two of us stand together?".into(), action: Action::Us }]);
    }
    if any(&t, NAMES_CUES) {
        return outside("names", "Kshana cannot tell a name, a first letter or who exactly a person will be. It can tell you when marriage or a relationship is likely.", vec![ask_alt("marriage", "when"), ask_alt("new_love", "will_it_happen")]);
    }
    if any(&t, AMOUNT_CUES) {
        return outside("amount", "Kshana cannot give an exact sum of money or a salary figure. It can tell you whether money rises or falls, and when.", vec![Alternative { label: "How is money for me?".into(), action: Action::Life(Life::Money) }, ask_alt("promotion", "when"), ask_alt("debt_cleared", "when")]);
    }
    if any(&t, LOST_CUES) {
        return outside("lost", "Kshana cannot find a lost or stolen thing or say who took it. It reads the course of your own life.", vec![Alternative { label: "How is my day today?".into(), action: Action::Outlook("day") }, Alternative { label: "How is money for me?".into(), action: Action::Life(Life::Money) }]);
    }
    if any(&t, WORLD_CUES) {
        return outside("world", "Kshana reads your own life from your own birth time; it does not predict elections, matches, markets or the weather.", general_alternatives().into_iter().take(4).collect());
    }

    // 3. Someone else's life: their birth details are needed.
    let family_topic_hint = any(&t, &["mother", "father", "amma", "achan", "maa", "papa", "son", "daughter", "makan", "makal", "child", "brother", "sister", "sibling", "in law", "in laws", "wife", "husband"]);
    if any(&t, OTHER_PEOPLE) && !(family_topic_hint && (any(&t, &["health", "tabiyat", "asukham", "fight", "trouble", "problem", "come back", "marriage"]) && !any(&t, &["my sister s marriage", "my brother s marriage", "my sister s job", "my brother s job", "my sister s exam", "my brother s exam", "my wife s job", "my husband s job"]))) {
        return outside("other_person", "This question is about someone else's own life, and a reading needs their birth date, time and place. Add them under Us to read them, or ask about how it touches your life.", vec![Alternative { label: "Add someone under Us".into(), action: Action::Us }, Alternative { label: "How is my family life?".into(), action: Action::Life(Life::Family) }]);
    }

    // 4. Kinds of question that are not about one matter.
    let asks_ahead = Q_CUES.iter().any(|(_, c)| any(&t, c));
    if any(&t, PAST_STRONG) || (any(&t, PAST_CUES) && !asks_ahead) {
        return answer(Action::Past, "Understood as: a look back at what happened, and how that time stood for you.".into(), 0.75);
    }
    let life_scores: Vec<(Life, f64)> = LIFE_CUES.iter().map(|(l, c)| (*l, count(&t, c))).collect();
    let best_life = life_scores.iter().cloned().fold(None, |a: Option<(Life, f64)>, x| if x.1 > a.map_or(0.0, |a| a.1) { Some(x) } else { a });
    if any(&t, CHART_FEAR_CUES) && !any(&t, REMEDY_CUES) {
        return answer(Action::ChartFear, "Understood as: is something wrong in my chart, and when does it end?".into(), 0.9);
    }
    if any(&t, REMEDY_CUES) {
        let life = best_life.filter(|x| x.1 >= 1.0).map(|x| x.0);
        return answer(Action::Remedy(life), format!("Understood as: what will help{}.", life.map(|l| format!(" with {}", l.name().to_lowercase())).unwrap_or_default()), 0.85);
    }
    if any(&t, US_CUES) {
        return answer(Action::Us, "Understood as: how the two of you stand together.".into(), 0.85);
    }
    if any(&t, PLAN_CUES) {
        let purpose = PURPOSE_CUES.iter().map(|(p, c)| (*p, count(&t, c))).filter(|x| x.1 > 0.0).fold(None, |a: Option<(&str, f64)>, x| if x.1 > a.map_or(0.0, |a| a.1) { Some(x) } else { a });
        return match purpose {
            Some((p, _)) => answer(Action::Plan(p), format!("Understood as: a good day and hour for {}.", plan_words(p)), 0.85),
            None => choose("unclear", "Understood: you want a good day. For what? Pick one, or ask again naming it.", ["meeting", "interview", "exam", "journey", "deal", "doctor", "family", "property", "launch"].iter().map(|p| Alternative { label: format!("A good day for {}", plan_words(p)), action: Action::Plan(p) }).collect(), 0.5),
        };
    }

    // 5. A matter.
    let mut topic_scores: Vec<(&'static str, f64)> = TOPIC_CUES.iter().chain(TOPIC_CUES_2.iter()).map(|(k, c)| (*k, count(&t, c))).filter(|x| x.1 > 0.0).collect();
    for (k, s) in combined(&t) {
        match topic_scores.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1 += s,
            None => topic_scores.push((k, s)),
        }
    }
    topic_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    // Matters that are part of another: a job abroad over a job, a government job over a
    // job, a second marriage over marriage, the children's matters over the general ones.
    let specific: &[(&str, &[&str])] = &[
        ("foreign_job", &["new_job", "settle_abroad", "journey_abroad"]),
        ("government_post", &["new_job", "exam", "competitive_exam"]),
        ("second_marriage", &["marriage"]),
        ("child_marriage", &["marriage", "sibling"]),
        ("child_studies", &["exam", "admission"]),
        ("child_health", &["recovery"]),
        ("mother_health", &["recovery", "mother"]),
        ("father_health", &["recovery", "father"]),
        ("study_abroad", &["settle_abroad", "journey_abroad", "admission"]),
        ("buy_house", &["rent", "moving_home"]),
        ("build_house", &["moving_home"]),
        ("sell_house", &["sale"]),
        ("business_partner", &["own_business", "new_love"]),
        ("reunion", &["marriage", "separation", "new_love"]),
        ("in_laws", &["mother", "father"]),
        ("home_peace", &["mother", "father", "sibling"]),
        ("competitive_exam", &["exam", "admission"]),
        ("return_home", &["reunion"]),
        ("sibling", &["peace_of_mind"]),
        ("home_peace", &["peace_of_mind"]),
        ("inheritance", &["father", "mother", "buy_house"]),
        ("child_health", &["recovery", "child_birth"]),
        ("child_studies", &["child_birth"]),
        ("child_marriage", &["child_birth"]),
        ("foreign_job", &["new_job"]),
    ];
    let present: Vec<&str> = topic_scores.iter().map(|x| x.0).collect();
    topic_scores.retain(|(k, _)| !specific.iter().any(|(s, general)| present.contains(s) && general.contains(k) && s != k));

    let q_scores: Vec<(&'static str, f64)> = {
        let mut v: Vec<(&'static str, f64)> = Q_CUES.iter().map(|(q, c)| (*q, count(&t, c))).filter(|x| x.1 > 0.0).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    };
    // A "when" or "will it come back" is clearer than a bare "will"; "should" is clearer than "now".
    let q = pick_question(&q_scores);

    if let Some(&(top, s1)) = topic_scores.first() {
        let second = topic_scores.get(1).cloned();
        let tie = second.map_or(false, |(k2, s2)| s2 >= s1 * 0.9 && topic_life(k2) != topic_life(top));
        if tie {
            let (k2, _) = second.unwrap();
            let qs = q.unwrap_or("when");
            return choose("tie", "Your question touches more than one thing. Which do you mean?", vec![ask_alt(top, qs), ask_alt(k2, qs), ask_alt(top, "will_it_happen"), ask_alt(k2, "will_it_happen")], 0.5);
        }
        let question = match q {
            Some(q) => fit_question(top, q),
            None => match default_question(top) {
                Some(d) => d,
                None => {
                    return choose("unclear", &format!("Understood: {}. What would you like to know?", topic_words(top).to_lowercase()), vec![ask_alt(top, "when"), ask_alt(top, "will_it_happen"), ask_alt(top, "good_time_now"), ask_alt(top, "should_i")], 0.6);
                }
            },
        };
        let conf = if q.is_some() { 0.9 } else { 0.75 };
        return answer(
            Action::Ask { life: topic_life(top), topic: Some(top), question },
            format!("Understood as: {} — {}", topic_words(top).to_lowercase(), q_words(question)),
            conf,
        );
    }

    // 6. A span of time.
    let span = SPAN_CUES.iter().find(|(_, c)| any(&t, c)).map(|(s, _)| *s);
    if let Some(s) = span {
        if s == "future" {
            return answer(Action::Future, "Understood as: what is coming in the years ahead.".into(), 0.85);
        }
        if best_life.map_or(true, |x| x.1 < 1.0) {
            return answer(Action::Outlook(s), format!("Understood as: how {} looks for you.", span_words(s)), 0.85);
        }
    }

    // 7. A part of life, no single matter.
    if let Some((life, s)) = best_life.filter(|x| x.1 >= 1.0) {
        let tie = life_scores.iter().filter(|x| x.0 != life && x.1 >= s * 0.9 && x.1 >= 1.0).count() > 0;
        if tie {
            let others: Vec<Alternative> = life_scores.iter().filter(|x| x.1 >= 1.0).map(|x| Alternative { label: format!("How is {} for me now?", x.0.name().to_lowercase()), action: Action::Life(x.0) }).collect();
            return choose("tie", "Your question touches more than one part of life. Which do you mean?", others, 0.5);
        }
        return match q {
            Some(question) if question != "good_time_now" => answer(
                Action::Ask { life, topic: None, question },
                format!("Understood as: your {} — {}", life.name().to_lowercase(), q_words(question)),
                0.7,
            ),
            _ => answer(Action::Life(life), format!("Understood as: how your {} stands now and ahead.", life.name().to_lowercase()), 0.7),
        };
    }

    // 8. Not understood: say so, and offer what can be answered.
    choose(
        "unclear",
        "Sorry, Kshana could not understand this question well enough to answer it truly. It works offline, from your birth time and the sky, for a set list of questions. Here are the closest ones it can answer; or ask again in other words.",
        general_alternatives(),
        0.0,
    )
}

fn pick_question(q: &[(&'static str, f64)]) -> Option<&'static str> {
    let has_q = |k: &str| q.iter().any(|x| x.0 == k);
    for k in ["should_i", "when", "will_it_return", "will_it_recover", "good_time_now", "will_it_happen"] {
        if has_q(k) {
            return Some(k);
        }
    }
    None
}

/// Some kinds of question only make sense for some matters: "will it come back" for a
/// person or money, "will it be set right" for a trouble.
fn fit_question(topic: &str, q: &'static str) -> &'static str {
    match (topic, q) {
        ("reunion", "will_it_recover") | ("reunion", "will_it_happen") => "will_it_return",
        (t, "will_it_return") if !matches!(t, "reunion" | "return_home" | "debt_cleared" | "sale" | "investment" | "inheritance" | "windfall") => "will_it_happen",
        ("recovery" | "peace_of_mind" | "home_peace" | "separation" | "in_laws" | "sibling" | "mother_health" | "father_health" | "child_health" | "child_studies" | "court_case", "will_it_happen") => "will_it_recover",
        _ => q,
    }
}

/// The question a matter asks when the person names only the matter ("my visa?").
fn default_question(topic: &str) -> Option<&'static str> {
    match topic {
        "marriage" | "new_job" | "promotion" | "visa" | "child_birth" | "buy_house" | "debt_cleared" | "settle_abroad" | "return_home" | "child_marriage" | "transfer" | "foreign_job" | "government_post" | "loan" | "admission" | "scholarship" => Some("when"),
        "reunion" => Some("will_it_return"),
        "recovery" | "peace_of_mind" | "home_peace" | "separation" | "in_laws" | "sibling" | "mother_health" | "father_health" | "child_health" | "child_studies" | "court_case" => Some("will_it_recover"),
        "exam" | "competitive_exam" | "new_love" | "inheritance" | "windfall" | "sale" | "sell_house" | "study_abroad" | "journey_abroad" => Some("will_it_happen"),
        "surgery" | "build_house" | "investment" => Some("good_time_now"),
        _ => None,
    }
}

fn plan_words(p: &str) -> &'static str {
    match p {
        "interview" => "an interview",
        "exam" => "an exam",
        "journey" => "a journey",
        "deal" => "a deal or a signing",
        "doctor" => "the doctor or a treatment",
        "date" => "meeting someone for marriage",
        "family" => "a family function",
        "property" => "a property matter",
        "launch" => "starting something new",
        "meeting" => "a meeting",
        _ => "anything",
    }
}

fn span_words(s: &str) -> &'static str {
    match s {
        "day" => "today",
        "week" => "this week",
        "month" => "this month",
        _ => "this year",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a phrasing must come out as: `ask:topic:question`, `ask:life:question`,
    /// `life:Life`, `outlook:span`, `future`, `remedy`, `fear`, `us`, `plan:purpose`,
    /// `past`, or a status for the ones not answered (`choose`, `outside:reason`, `care`).
    fn kind(u: &Understanding) -> String {
        match (&u.status[..], &u.action) {
            ("answer", Some(Action::Ask { topic: Some(t), question, .. })) => format!("ask:{t}:{question}"),
            ("answer", Some(Action::Ask { life, topic: None, question })) => format!("ask:{}:{question}", life.name()),
            ("answer", Some(Action::Life(l))) => format!("life:{}", l.name()),
            ("answer", Some(Action::Outlook(s))) => format!("outlook:{s}"),
            ("answer", Some(Action::Future)) => "future".into(),
            ("answer", Some(Action::Remedy(_))) => "remedy".into(),
            ("answer", Some(Action::ChartFear)) => "fear".into(),
            ("answer", Some(Action::Us)) => "us".into(),
            ("answer", Some(Action::Plan(p))) => format!("plan:{p}"),
            ("answer", Some(Action::Past)) => "past".into(),
            ("outside", _) => format!("outside:{}", u.reason),
            (s, _) => s.to_string(),
        }
    }

    const CASES: &[(&str, &str)] = &[
        // Marriage: English, Malayalam, Manglish, Hindi, Hinglish.
        ("When will I get married?", "ask:marriage:when"),
        ("when will my marriage happen", "ask:marriage:when"),
        ("will i get married this year", "ask:marriage:will_it_happen"),
        ("Is there a chance of marriage in 2027?", "ask:marriage:will_it_happen"),
        ("marriage?", "ask:marriage:when"),
        ("my marriage is getting delayed", "ask:marriage:when"),
        ("kalyanam eppol nadakkum", "ask:marriage:when"),
        ("kalyanam eppo", "ask:marriage:when"),
        ("എന്റെ കല്യാണം എപ്പോൾ നടക്കും", "ask:marriage:when"),
        ("വിവാഹം നടക്കുമോ", "ask:marriage:will_it_happen"),
        ("meri shaadi kab hogi", "ask:marriage:when"),
        ("मेरी शादी कब होगी", "ask:marriage:when"),
        ("शादी होगी क्या", "ask:marriage:will_it_happen"),
        ("should i marry him", "ask:marriage:should_i"),
        ("should I say yes to this proposal", "ask:marriage:should_i"),
        ("is this a good time to get married", "ask:marriage:good_time_now"),
        ("second marriage chances", "ask:second_marriage:will_it_happen"),
        ("will i marry again", "ask:second_marriage:will_it_happen"),
        ("dusri shaadi kab hogi", "ask:second_marriage:when"),
        // A partner who left.
        ("My wife left me. Will she come back?", "ask:reunion:will_it_return"),
        ("will my husband come back", "ask:reunion:will_it_return"),
        ("will we get back together", "ask:reunion:will_it_return"),
        ("my wife is staying away from me since july, will she return", "ask:reunion:will_it_return"),
        ("ente bharya thirichu varumo", "ask:reunion:will_it_return"),
        ("ഭാര്യ തിരിച്ചു വരുമോ", "ask:reunion:will_it_return"),
        ("kya meri wife wapas aayegi", "ask:reunion:will_it_return"),
        ("क्या मेरी पत्नी वापस आएगी", "ask:reunion:will_it_return"),
        ("will my ex come back to me", "ask:reunion:will_it_return"),
        ("patch up chances with my girlfriend", "ask:reunion:will_it_return"),
        // Marriage trouble.
        ("fights with my wife, will it settle", "ask:separation:will_it_recover"),
        ("will my divorce happen", "ask:separation:will_it_recover"),
        ("marriage problem when will it end", "ask:separation:when"),
        ("तलाक होगा क्या", "ask:separation:will_it_recover"),
        ("trouble with mother in law", "ask:in_laws:will_it_recover"),
        ("saas se jhagda kab khatam hoga", "ask:in_laws:when"),
        // Love.
        ("will i find the right person", "ask:new_love:will_it_happen"),
        ("when will i find love", "ask:new_love:when"),
        ("will i get a girlfriend", "ask:new_love:will_it_happen"),
        ("how is my love life", "ask:new_love:will_it_happen"),
        // Jobs.
        ("When will I get a job?", "ask:new_job:when"),
        ("will i get a job this year", "ask:new_job:will_it_happen"),
        ("job eppol kittum", "ask:new_job:when"),
        ("ജോലി എപ്പോൾ കിട്ടും", "ask:new_job:when"),
        ("ജോലി കിട്ടുമോ", "ask:new_job:will_it_happen"),
        ("naukri kab milegi", "ask:new_job:when"),
        ("नौकरी कब मिलेगी", "ask:new_job:when"),
        ("I am unemployed for a year, when will it change", "ask:new_job:when"),
        ("should i change my job", "ask:new_job:should_i"),
        ("should i resign and join the new company", "ask:new_job:should_i"),
        ("will i get the offer letter", "ask:new_job:will_it_happen"),
        ("will i get promoted", "ask:promotion:will_it_happen"),
        ("when will i get a hike", "ask:promotion:when"),
        ("promotion kab hoga", "ask:promotion:when"),
        ("transfer kittumo", "ask:transfer:will_it_happen"),
        ("when will my transfer happen", "ask:transfer:when"),
        ("will i get a government job", "ask:government_post:will_it_happen"),
        ("PSC job kittumo", "ask:government_post:will_it_happen"),
        ("sarkari naukri milegi kya", "ask:government_post:will_it_happen"),
        ("when will i get a job in gulf", "ask:foreign_job:when"),
        ("job abroad chances", "ask:foreign_job:will_it_happen"),
        // Business.
        ("should i start my own business", "ask:own_business:should_i"),
        ("my business is in loss, will it recover", "ask:own_business:will_it_recover"),
        ("business eppol nannakum", "ask:own_business:when"),
        ("vyapar me fayda kab hoga", "ask:own_business:when"),
        ("should i take a business partner", "ask:business_partner:should_i"),
        // Court.
        ("will i win my court case", "ask:court_case:will_it_recover"),
        ("when will the case end", "ask:court_case:when"),
        ("कोर्ट केस कब खत्म होगा", "ask:court_case:when"),
        // Money.
        ("when will my debts clear", "ask:debt_cleared:when"),
        ("kadam eppol theerum", "ask:debt_cleared:when"),
        ("कर्ज कब उतरेगा", "ask:debt_cleared:when"),
        ("will my loan be approved", "ask:loan:will_it_happen"),
        ("home loan kittumo", "ask:loan:will_it_happen"),
        ("is this a good time to invest", "ask:investment:good_time_now"),
        ("should i invest in mutual funds", "ask:investment:should_i"),
        ("will i get my money back", "ask:Money:will_it_return"),
        ("how is money for me", "life:Money"),
        ("how is my financial situation", "life:Money"),
        ("panam eppol varum", "ask:Money:when"),
        ("paisa kab aayega", "ask:Money:when"),
        ("will i become rich", "ask:windfall:will_it_happen"),
        ("will i get my share of ancestral property", "ask:inheritance:will_it_happen"),
        // Abroad.
        ("will my visa come through", "ask:visa:will_it_happen"),
        ("visa kittumo", "ask:visa:will_it_happen"),
        ("വിസ കിട്ടുമോ", "ask:visa:will_it_happen"),
        ("when can i settle abroad", "ask:settle_abroad:when"),
        ("will i settle in canada", "ask:settle_abroad:will_it_happen"),
        ("when can i come back home to kerala", "ask:return_home:when"),
        ("naattilekku eppol thirichu varum", "ask:return_home:when"),
        ("will i go abroad this year", "ask:journey_abroad:will_it_happen"),
        ("can i study abroad", "ask:study_abroad:will_it_happen"),
        ("videsh me padhai ho payegi", "ask:study_abroad:will_it_happen"),
        // Health and mind.
        ("when will i recover", "ask:recovery:when"),
        ("will my illness be cured", "ask:recovery:will_it_recover"),
        ("asukham eppol maarum", "ask:recovery:when"),
        ("बीमारी कब ठीक होगी", "ask:recovery:when"),
        ("I have a surgery next month, is the time right", "ask:surgery:good_time_now"),
        ("operation success aakumo", "ask:surgery:will_it_happen"),
        ("I have no peace of mind", "ask:peace_of_mind:will_it_recover"),
        ("when will this stress end", "ask:peace_of_mind:when"),
        ("tension kab khatam hogi", "ask:peace_of_mind:when"),
        ("how is my health", "life:Health"),
        ("my mother's health worries me", "ask:mother_health:will_it_recover"),
        ("ammayude asukham maarumo", "ask:mother_health:will_it_recover"),
        ("papa ki tabiyat kab theek hogi", "ask:father_health:when"),
        // Home and family.
        ("there is no peace at home", "ask:home_peace:will_it_recover"),
        ("family problems when will they end", "ask:home_peace:when"),
        ("ghar me jhagda kab khatam hoga", "ask:home_peace:when"),
        ("trouble with my brother", "ask:sibling:will_it_recover"),
        ("when can i buy a house", "ask:buy_house:when"),
        ("veedu vangan pattumo", "ask:buy_house:will_it_happen"),
        ("घर कब खरीद पाऊंगा", "ask:buy_house:when"),
        ("is this the time to build a house", "ask:build_house:good_time_now"),
        ("will my land sell", "ask:sell_house:will_it_happen"),
        ("when will i buy a car", "ask:vehicle:when"),
        ("should we move house", "ask:moving_home:should_i"),
        // Studies.
        ("will i pass my exam", "ask:exam:will_it_happen"),
        ("pareeksha jayikkumo", "ask:exam:will_it_happen"),
        ("will i clear neet", "ask:competitive_exam:will_it_happen"),
        ("will i get admission in this college", "ask:admission:will_it_happen"),
        ("scholarship milegi kya", "ask:scholarship:will_it_happen"),
        // Children.
        ("when will we have a baby", "ask:child_birth:when"),
        ("kunj eppol undakum", "ask:child_birth:when"),
        ("संतान कब होगी", "ask:child_birth:when"),
        ("when will my daughter's marriage happen", "ask:child_marriage:when"),
        ("beti ki shadi kab hogi", "ask:child_marriage:when"),
        ("my son's studies worry me", "ask:child_studies:will_it_recover"),
        ("my child's health", "ask:child_health:will_it_recover"),
        // Spans and the future.
        ("how is my day today", "outlook:day"),
        ("innathe divasam engane", "outlook:day"),
        ("आज का दिन कैसा है", "outlook:day"),
        ("how is this week for me", "outlook:week"),
        ("this month?", "outlook:month"),
        ("what does next year hold", "outlook:year"),
        ("tell me my future", "future"),
        ("ente bhaavi", "future"),
        ("मेरा भविष्य बताओ", "future"),
        // Remedies, fears, two people, good days, the past.
        ("what remedy should i do", "remedy"),
        ("pariharam enthanu", "remedy"),
        ("koi upay batao", "remedy"),
        ("remedy for my marriage problems", "remedy"),
        ("do i have sade sati", "fear"),
        ("ezharasani eppol theerum", "fear"),
        ("am i manglik", "fear"),
        ("my astrologer said my chart has dosha", "fear"),
        ("when will my bad time end", "fear"),
        ("are we compatible", "us"),
        ("check our porutham", "us"),
        ("good day for my interview", "plan:interview"),
        ("auspicious date for griha pravesh", "plan:family"),
        ("muhurtham for engagement", "plan:family"),
        ("best day to sign the deal", "plan:deal"),
        ("good day to start my shop", "plan:launch"),
        ("a good day", "choose"),
        ("why did my marriage fail", "past"),
        ("what happened in 2020", "past"),
        // Outside: never answered as if understood.
        ("when will i die", "outside:death"),
        ("how long will i live", "outside:death"),
        ("will my father die", "outside:death"),
        ("ente maranam eppol", "outside:death"),
        ("give me the lottery number", "outside:gamble"),
        ("which stock should i buy", "outside:gamble"),
        ("satta matka number", "outside:gamble"),
        ("will my baby be a boy or girl", "outside:gender"),
        ("ladka hoga ya ladki", "outside:gender"),
        ("which medicine should i take", "outside:medical"),
        ("do i have cancer", "outside:medical"),
        ("someone did black magic on me", "outside:curse"),
        ("koodothram undo", "outside:curse"),
        ("is my wife cheating on me", "outside:other_mind"),
        ("does she still love me", "outside:other_mind"),
        ("what is the name of my future wife", "outside:names"),
        ("who will i marry", "outside:names"),
        ("how much money will i earn", "outside:amount"),
        ("what will be my salary", "outside:amount"),
        ("where is my lost ring", "outside:lost"),
        ("who stole my phone", "outside:lost"),
        ("who will win the election", "outside:world"),
        ("will india win the world cup", "outside:world"),
        ("will my friend get a job", "outside:other_person"),
        ("my brother's marriage when", "outside:other_person"),
        ("when will my sister's marriage happen", "outside:other_person"),
        // Care.
        ("I want to die", "care"),
        ("i don't want to live anymore", "care"),
        ("thinking of suicide", "care"),
        // Not understood: never guessed.
        ("", "choose"),
        ("hello", "choose"),
        ("asdf qwer", "choose"),
        ("tell me something", "choose"),
        ("what is the meaning of life", "choose"),
        ("banana", "choose"),
        ("will it", "choose"),
    ];

    #[test]
    fn every_phrasing_is_understood_as_it_was_meant() {
        let mut wrong = Vec::new();
        for (text, want) in CASES {
            let got = kind(&understand(text));
            if &got != want {
                wrong.push(format!("{text:?}: wanted {want}, got {got}"));
            }
        }
        assert!(wrong.is_empty(), "{} of {} misunderstood:\n{}", wrong.len(), CASES.len(), wrong.join("\n"));
    }

    #[test]
    fn what_is_not_answered_always_offers_something_to_pick() {
        for (text, _) in CASES {
            let u = understand(text);
            if u.status != "answer" {
                assert!(!u.alternatives.is_empty(), "{text:?} offers nothing");
                assert!(!u.message.is_empty());
            } else {
                assert!(u.message.starts_with("Understood as"), "{text:?}: {}", u.message);
            }
        }
    }

    #[test]
    fn nothing_said_names_the_astrology() {
        for (text, _) in CASES {
            let u = understand(text);
            assert_eq!(crate::plain::jargon(&u.message), None, "{text:?}: {}", u.message);
            for a in &u.alternatives {
                assert_eq!(crate::plain::jargon(&a.label), None, "{text:?}: {}", a.label);
            }
        }
    }
}

/// Phrasings never used while the cues were written, to measure honestly. Each allows
/// the right answer or an honest "not sure, pick one"; a confident wrong answer fails.
#[cfg(test)]
mod held_out {
    use super::*;

    fn kind(u: &Understanding) -> String {
        match (&u.status[..], &u.action) {
            ("answer", Some(Action::Ask { topic: Some(t), .. })) => format!("ask:{t}"),
            ("answer", Some(Action::Ask { life, topic: None, .. })) => format!("ask:{}", life.name()),
            ("answer", Some(Action::Life(l))) => format!("life:{}", l.name()),
            ("answer", Some(Action::Outlook(s))) => format!("outlook:{s}"),
            ("answer", Some(Action::Future)) => "future".into(),
            ("answer", Some(Action::Remedy(_))) => "remedy".into(),
            ("answer", Some(Action::ChartFear)) => "fear".into(),
            ("answer", Some(Action::Us)) => "us".into(),
            ("answer", Some(Action::Plan(p))) => format!("plan:{p}"),
            ("answer", Some(Action::Past)) => "past".into(),
            ("outside", _) => format!("outside:{}", u.reason),
            (s, _) => s.to_string(),
        }
    }

    /// (phrasing, the answers that are right, separated by |). "choose" is always allowed
    /// except where the phrasing must be refused or cared for.
    const UNSEEN: &[(&str, &str)] = &[
        ("by when will my wedding take place", "ask:marriage"),
        ("is marriage in my destiny", "ask:marriage"),
        ("any chance of getting married soon", "ask:marriage"),
        ("my parents are searching for a bride for me, when will it be fixed", "ask:marriage"),
        ("kalyanam nadakkan chance undo", "ask:marriage"),
        ("ennu kalyanam", "ask:marriage"),
        ("शादी में देरी क्यों हो रही है", "ask:marriage|past"),
        ("shadi kab tak hogi", "ask:marriage"),
        ("will my girlfriend come back to me", "ask:reunion"),
        ("my husband walked out, will he return home", "ask:reunion|ask:return_home"),
        ("we separated last year, will we reunite", "ask:reunion"),
        ("avan thirichu varumo", "ask:reunion"),
        ("वो कभी लौटेगी क्या", "ask:reunion"),
        ("constant quarrels with my husband", "ask:separation"),
        ("will my married life improve", "ask:separation|ask:Love|life:Love|ask:marriage"),
        ("is divorce likely for me", "ask:separation"),
        ("I keep fighting with my wife", "ask:separation"),
        ("when will i meet someone", "ask:new_love"),
        ("is there love in my future", "ask:new_love|future"),
        ("I am jobless, any good news coming", "ask:new_job"),
        ("will i land a job soon", "ask:new_job"),
        ("I applied to many places, will something work out", "ask:new_job"),
        ("joli kittan ethra naal edukkum", "ask:new_job"),
        ("ജോലി എന്ന് ശരിയാകും", "ask:new_job"),
        ("नौकरी लगेगी क्या", "ask:new_job"),
        ("will i get the job i interviewed for", "ask:new_job"),
        ("should i leave my company", "ask:new_job"),
        ("is switching jobs a good idea now", "ask:new_job"),
        ("will my salary go up", "ask:promotion|life:Money|ask:Money"),
        ("increment kittumo", "ask:promotion"),
        ("will my boss promote me", "ask:promotion"),
        ("will i get a transfer to my home town", "ask:transfer|ask:return_home"),
        ("can i clear the PSC exam and get a govt job", "ask:government_post"),
        ("upsc me selection hoga", "ask:government_post"),
        ("dubai job kittumo", "ask:foreign_job"),
        ("gulf jana hoga", "ask:foreign_job|ask:journey_abroad|ask:Travel"),
        ("is it the right time to start a business", "ask:own_business"),
        ("my shop is not running well", "ask:own_business"),
        ("will my startup succeed", "ask:own_business"),
        ("dhandha kab chalega", "ask:own_business"),
        ("will the court decide in my favour", "ask:court_case"),
        ("property dispute with relatives, will it end", "ask:court_case|ask:inheritance"),
        ("kesil jayikkumo", "ask:court_case"),
        ("when will my financial problems end", "ask:Money|life:Money"),
        ("i am in heavy debt", "ask:debt_cleared"),
        ("loan theerkkan pattumo", "ask:debt_cleared|ask:loan"),
        ("कर्जा कब खत्म होगा", "ask:debt_cleared"),
        ("will the bank sanction my loan", "ask:loan"),
        ("money is stuck with a friend, will i get it back", "ask:Money|ask:debt_cleared|outside:other_person"),
        ("is investing in land good now", "ask:investment|ask:buy_house"),
        ("should i put money in shares", "ask:investment"),
        ("will i get my father's property", "ask:inheritance"),
        ("is it a good time to sell my plot", "ask:sell_house"),
        ("will my canada pr come", "ask:visa|ask:settle_abroad"),
        ("us visa interview result", "ask:visa"),
        ("will i be able to go to australia", "ask:settle_abroad|ask:journey_abroad"),
        ("videshathu pokan pattumo", "ask:journey_abroad|ask:Travel|ask:settle_abroad"),
        ("when will i return to my country", "ask:return_home"),
        ("can i do my masters in germany", "ask:study_abroad"),
        ("will my health improve", "ask:recovery|ask:Health|life:Health"),
        ("I have been ill for months", "ask:recovery"),
        ("is my operation going to be successful", "ask:surgery"),
        ("good day for surgery", "plan:doctor"),
        ("I can not sleep and feel anxious", "ask:peace_of_mind"),
        ("when will my mind be at peace", "ask:peace_of_mind"),
        ("manassinu samadhanam kittumo", "ask:peace_of_mind"),
        ("will my mother recover", "ask:mother_health"),
        ("achante asukham maarumo", "ask:father_health"),
        ("family tension kab tak", "ask:home_peace"),
        ("will my brother and I make peace", "ask:sibling"),
        ("can I afford a flat this year", "ask:buy_house"),
        ("sontham veedu eppol", "ask:buy_house|ask:Home|life:Home"),
        ("apna ghar kab hoga", "ask:buy_house|ask:Home|life:Home"),
        ("is this year good to construct my house", "ask:build_house"),
        ("should i rent a new place", "ask:rent"),
        ("will i buy a new bike", "ask:vehicle"),
        ("will i score well in my exams", "ask:exam"),
        ("will i crack jee", "ask:competitive_exam"),
        ("college admission kittumo", "ask:admission"),
        ("will i get the scholarship", "ask:scholarship"),
        ("we have been trying for a baby", "ask:child_birth"),
        ("pregnancy chances this year", "ask:child_birth"),
        ("kuttikal undakumo", "ask:child_birth|ask:Children"),
        ("when will my son get married", "ask:child_marriage"),
        ("my daughter is not studying well", "ask:child_studies"),
        ("my son is often sick", "ask:child_health"),
        ("what does today hold", "outlook:day"),
        ("innu engane", "outlook:day"),
        ("aaj kaisa rahega", "outlook:day"),
        ("next week how is it", "outlook:week"),
        ("how will this year be", "outlook:year"),
        ("what is in store for me in the coming years", "future"),
        ("future engane", "future"),
        ("aage kya hoga", "future"),
        ("what can i do to improve my luck", "remedy"),
        ("is there any remedy for delay in marriage", "remedy"),
        ("pooja cheyyano", "remedy"),
        ("shani ki sade sati kab khatam", "fear"),
        ("is rahu affecting me", "fear"),
        ("chovva dosham undo", "fear"),
        ("my horoscope is bad they said", "fear"),
        ("will we be happy together", "us"),
        ("check compatibility with my fiance", "us"),
        ("best date for marriage", "plan:family|plan:date"),
        ("auspicious time to buy a car", "plan:deal|plan:property"),
        ("which day is good for travel", "plan:journey"),
        ("why did i lose my job last year", "past"),
        ("what went wrong in 2023", "past"),
        // Refused, never answered.
        ("when am i going to die", "outside:death"),
        ("how many years will my mother live", "outside:death"),
        ("tell me winning lottery numbers", "outside:gamble"),
        ("which crypto will go up", "outside:gamble"),
        ("will it be a baby boy", "outside:gender"),
        ("my scan, is the baby a girl", "outside:gender"),
        ("which tablet for my fever", "outside:medical"),
        ("is my report normal", "outside:medical"),
        ("my neighbour did black magic", "outside:curse"),
        ("is someone doing jadu tona on my family", "outside:curse"),
        ("is my husband having an affair", "outside:other_mind"),
        ("does my boss like me", "outside:other_mind|outside:other_person"),
        ("tell me my wife's name", "outside:names"),
        ("which letter will my husband's name start with", "outside:names"),
        ("how much will i earn next year", "outside:amount"),
        ("how many crores will i make", "outside:amount"),
        ("where did i lose my gold chain", "outside:lost"),
        ("who will win ipl", "outside:world"),
        ("will my cousin pass his exam", "outside:other_person"),
        ("will my friend's marriage happen", "outside:other_person"),
        // Care.
        ("i feel like ending my life", "care"),
        ("no reason to live anymore", "care"),
        // Unclear: must not be guessed.
        ("hmm", "choose"),
        ("ok", "choose"),
        ("what", "choose"),
        ("please help", "choose"),
        ("tell", "choose"),
        ("is it", "choose"),
        ("are you real", "choose"),
        ("who made you", "choose"),
        ("good morning", "choose"),
    ];

    #[test]
    fn unseen_phrasings_are_answered_right_or_honestly_not_answered() {
        let mut confident_wrong = Vec::new();
        let mut unsure = Vec::new();
        for (text, ok) in UNSEEN {
            let got = kind(&understand(text));
            let allowed: Vec<&str> = ok.split('|').collect();
            let must_refuse = allowed.iter().all(|a| a.starts_with("outside") || *a == "care");
            if allowed.contains(&got.as_str()) {
                continue;
            }
            if got == "choose" && !must_refuse {
                unsure.push(format!("{text:?} (wanted {ok})"));
                continue;
            }
            confident_wrong.push(format!("{text:?}: allowed {ok}, got {got}"));
        }
        println!("unseen: {} phrasings; {} honestly unsure:\n{}", UNSEEN.len(), unsure.len(), unsure.join("\n"));
        assert!(confident_wrong.is_empty(), "{} confident wrong of {}:\n{}", confident_wrong.len(), UNSEEN.len(), confident_wrong.join("\n"));
    }
}

/// The reply for a question that cannot be answered, by its reason: the same words as
/// when it is asked in the plainest way.
fn refusal(reason: &str) -> Understanding {
    let plain = match reason {
        "death" => "when will i die",
        "gender" => "boy or girl",
        "gamble" => "lottery number",
        "medical" => "which medicine",
        "names" => "name of my",
        "lost" => "where is my",
        "curse" => "black magic",
        "other_mind" => "cheating",
        "amount" => "how much money",
        _ => "election",
    };
    understand_with(plain, false)
}

/// A third set, written after the second was fixed, measured before any change for it.
#[cfg(test)]
mod third_set {
    use super::*;

    fn kind(u: &Understanding) -> String {
        match (&u.status[..], &u.action) {
            ("answer", Some(Action::Ask { topic: Some(t), .. })) => format!("ask:{t}"),
            ("answer", Some(Action::Ask { life, topic: None, .. })) => format!("ask:{}", life.name()),
            ("answer", Some(Action::Life(l))) => format!("life:{}", l.name()),
            ("answer", Some(Action::Outlook(s))) => format!("outlook:{s}"),
            ("answer", Some(Action::Future)) => "future".into(),
            ("answer", Some(Action::Remedy(_))) => "remedy".into(),
            ("answer", Some(Action::ChartFear)) => "fear".into(),
            ("answer", Some(Action::Us)) => "us".into(),
            ("answer", Some(Action::Plan(p))) => format!("plan:{p}"),
            ("answer", Some(Action::Past)) => "past".into(),
            ("outside", _) => format!("outside:{}", u.reason),
            (s, _) => s.to_string(),
        }
    }

    const THIRD: &[(&str, &str)] = &[
        ("when is my marriage yog", "ask:marriage|fear"),
        ("my marriage keeps getting cancelled", "ask:marriage"),
        ("will i have a love marriage or arranged", "ask:marriage"),
        ("is this alliance good for me", "ask:marriage|us"),
        ("kalyana yogam undo", "ask:marriage"),
        ("vivahathinu thadassam undo", "ask:marriage|fear"),
        ("shaadi me rukawat kyu", "ask:marriage|past|fear"),
        ("my ex blocked me, will she contact me again", "ask:reunion"),
        ("will my wife and I reconcile", "ask:reunion"),
        ("husband has been away for two years", "ask:reunion"),
        ("we fight every day", "ask:separation|ask:home_peace"),
        ("is my marriage going to break", "ask:separation"),
        ("will i be single forever", "ask:new_love|ask:marriage"),
        ("will i find a good partner", "ask:new_love|ask:marriage"),
        ("job loss, what next", "ask:new_job"),
        ("will i be fired", "ask:new_job|ask:Career"),
        ("my manager is harassing me", "ask:Career|life:Career|outside:other_mind"),
        ("will my contract be renewed", "ask:new_job|ask:Career"),
        ("career growth in coming years", "future|life:Career|ask:Career"),
        ("is IT field right for me", "ask:Career|life:Career"),
        ("joli maaran nalla samayam aano", "ask:new_job"),
        ("naukri chhod du kya", "ask:new_job"),
        ("will i get bank job", "ask:government_post|ask:new_job"),
        ("kerala psc rank list il varumo", "ask:government_post"),
        ("can i go to saudi for work", "ask:foreign_job"),
        ("will my business partner cheat me", "outside:other_mind"),
        ("should i close my business", "ask:own_business"),
        ("new shop opening date", "plan:launch"),
        ("land case verdict", "ask:court_case"),
        ("police case against me, will it be cleared", "ask:court_case"),
        ("how to get out of debt", "remedy|ask:debt_cleared"),
        ("credit card debt is too much", "ask:debt_cleared"),
        ("will i get the money someone owes me", "ask:Money|ask:debt_cleared"),
        ("will i become a millionaire", "ask:windfall"),
        ("is gold investment good now", "ask:investment"),
        ("when will my money problems go away", "ask:Money|life:Money"),
        ("will my visa get rejected", "ask:visa"),
        ("canada pr kab milega", "ask:visa|ask:settle_abroad"),
        ("will i settle in uk with family", "ask:settle_abroad"),
        ("can i bring my family abroad", "ask:settle_abroad|ask:journey_abroad"),
        ("when will i go back to india for good", "ask:return_home"),
        ("my back pain, when will it go", "ask:recovery"),
        ("will my sugar come under control", "ask:recovery|outside:medical"),
        ("depression, when will it end", "ask:peace_of_mind"),
        ("my mind is restless always", "ask:peace_of_mind"),
        ("will my mother's surgery go well", "ask:mother_health|ask:surgery"),
        ("father is in hospital", "ask:father_health"),
        ("will my wife recover from illness", "outside:other_person|ask:recovery"),
        ("relatives are troubling us", "ask:home_peace|life:Family"),
        ("dispute with my brother over property", "ask:sibling|ask:inheritance|ask:court_case"),
        ("when can i shift to my new house", "ask:moving_home|ask:buy_house"),
        ("griha pravesam date", "plan:family"),
        ("will i get my own house before 35", "ask:buy_house"),
        ("plot vaangan pattiya samayam", "ask:buy_house"),
        ("will i pass plus two", "ask:exam"),
        ("will i get through medical entrance", "ask:competitive_exam"),
        ("should i take science or commerce", "ask:Study|life:Study"),
        ("will i get a seat in IIT", "ask:admission|ask:competitive_exam"),
        ("ivf success aakumo", "ask:child_birth"),
        ("santhana bhagyam undo", "ask:child_birth"),
        ("when will my kid start talking", "ask:Children|life:Children|ask:child_health"),
        ("my son is not listening to us", "ask:Children|life:Children|ask:child_studies"),
        ("how is my week going to be", "outlook:week"),
        ("is today good for me", "outlook:day"),
        ("this month how is money", "life:Money|outlook:month|ask:Money"),
        ("predict my next 5 years", "future"),
        ("my life ahead", "future"),
        ("what pooja should i do for job", "remedy"),
        ("which gemstone should i wear", "remedy|outside:medical"),
        ("is my kundali strong", "fear|life:Love"),
        ("am i in sade sati now", "fear"),
        ("rahu dasha how long", "fear"),
        ("best muhurat for buying car", "plan:deal|plan:property"),
        ("good time to propose her", "plan:date|ask:new_love|ask:marriage"),
        ("which day to start new job", "plan:launch|plan:interview"),
        ("why is everything going wrong in my life", "past|fear|remedy"),
        ("what was 2019 like for me", "past"),
        // Refuse.
        ("will i live till 80", "outside:death"),
        ("when will my grandfather pass away", "outside:death"),
        ("what is today's lucky number", "outside:gamble"),
        ("dream11 team", "outside:gamble"),
        ("is the baby in my womb a boy", "outside:gender"),
        ("should i take this antibiotic", "outside:medical"),
        ("is someone jealous of me and doing nazar", "outside:curse"),
        ("is my girlfriend seeing someone else", "outside:other_mind"),
        ("what will my husband look like", "outside:names"),
        ("how much dowry will i get", "outside:amount"),
        ("where is my missing dog", "outside:lost"),
        ("who will be the next chief minister", "outside:world"),
        ("will my friend recover", "outside:other_person"),
        // Care.
        ("everything is over i want to end it all", "care"),
        // Unclear.
        ("xyz", "choose"),
        ("tell me", "choose"),
        ("are you an astrologer", "choose"),
    ];

    #[test]
    fn a_third_set_measured_honestly() {
        let mut right = 0;
        let mut unsure = Vec::new();
        let mut wrong = Vec::new();
        for (text, ok) in THIRD {
            let got = kind(&understand(text));
            let allowed: Vec<&str> = ok.split('|').collect();
            let must_refuse = allowed.iter().all(|a| a.starts_with("outside") || *a == "care");
            if allowed.contains(&got.as_str()) {
                right += 1;
            } else if got == "choose" && !must_refuse {
                unsure.push(format!("{text:?}"));
            } else {
                wrong.push(format!("{text:?}: allowed {ok}, got {got}"));
            }
        }
        println!("third: {} phrasings: {} right, {} honestly unsure, {} wrong\nUNSURE: {}\nWRONG:\n{}", THIRD.len(), right, unsure.len(), wrong.len(), unsure.join(", "), wrong.join("\n"));
        assert!(wrong.is_empty());
    }
}
