//! The glossary: every term the app shows, in one or two plain sentences, in English,
//! Malayalam and Hindi. The Malayalam and Hindi entries name each term the way Kerala and
//! North Indian almanacs print it, and explain it in plain words.

use kshana_rules::Lang;

/// One term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// The term as shown.
    pub term: &'static str,
    /// Plain meaning.
    pub meaning: &'static str,
    /// Tradition: "Vedic", "Western", "Both", "Engine". Always English: it is the filter key;
    /// [`tradition_name`] gives the word to show.
    pub tradition: &'static str,
}

/// One row: the English entry, then `(term, meaning)` in Malayalam and in Hindi.
struct Row {
    en: (&'static str, &'static str),
    tradition: &'static str,
    ml: (&'static str, &'static str),
    hi: (&'static str, &'static str),
}

const fn e(
    term: &'static str,
    meaning: &'static str,
    tradition: &'static str,
    ml: (&'static str, &'static str),
    hi: (&'static str, &'static str),
) -> Row {
    Row {
        en: (term, meaning),
        tradition,
        ml,
        hi,
    }
}

/// The tradition's name in `lang`: "Vedic" → "ഭാരതീയം" / "वैदिक". Unknown keys come back as given.
pub fn tradition_name(tradition: &str, lang: Lang) -> &str {
    match tradition {
        "Vedic" => lang.pick(["Vedic", "ഭാരതീയം", "वैदिक"]),
        "Western" => lang.pick(["Western", "പാശ്ചാത്യം", "पाश्चात्य"]),
        "Both" => lang.pick(["Both", "രണ്ടിലും", "दोनों"]),
        "Engine" => lang.pick(["Engine", "കണക്കുകൂട്ടൽ", "गणना"]),
        other => other,
    }
}

static ROWS: [Row; 56] = [
    e("Antardasha", "A sub-period inside a mahadasha. Each mahadasha is split among the nine planets in the same order and proportion as the whole cycle.", "Vedic",
        ("അപഹാരം (അന്തർദശ)", "ഒരു മഹാദശയ്ക്കുള്ളിലെ ചെറിയ കാലം. ഓരോ മഹാദശയും ഒമ്പത് ഗ്രഹങ്ങൾക്കിടയിൽ, മുഴുവൻ ചക്രത്തിലെ അതേ ക്രമത്തിലും അനുപാതത്തിലും വിഭജിക്കപ്പെടുന്നു."),
        ("अंतर्दशा", "महादशा के भीतर की छोटी अवधि। हर महादशा नौ ग्रहों में उसी क्रम और अनुपात में बँटती है जिसमें पूरा चक्र बँटता है।")),
    e("Ascendant", "The sign rising on the eastern horizon at the moment of birth. Called Lagna in Vedic astrology. It fixes the houses.", "Both",
        ("ലഗ്നം (ഉദയരാശി)", "ജനനസമയത്ത് കിഴക്കേ ചക്രവാളത്തിൽ ഉദിച്ചുകൊണ്ടിരിക്കുന്ന രാശി. ഭാരതീയ ജ്യോതിഷത്തിൽ ലഗ്നം. ഭാവങ്ങൾ ഇതിൽ നിന്ന് നിശ്ചയിക്കുന്നു."),
        ("लग्न (उदय राशि)", "जन्म के क्षण पूर्वी क्षितिज पर उदित हो रही राशि। वैदिक ज्योतिष में इसे लग्न कहते हैं। भाव इसी से तय होते हैं।")),
    e("Ashtakavarga", "A point system: each planet gives 0 or 1 point to each sign from eight reference points. A sign with many points is strong for transits and periods.", "Vedic",
        ("അഷ്ടകവർഗം", "ഒരു ബിന്ദു സമ്പ്രദായം: എട്ട് ആധാരങ്ങളിൽ നിന്ന് ഓരോ ഗ്രഹവും ഓരോ രാശിക്കും 0 അല്ലെങ്കിൽ 1 ബിന്ദു നൽകുന്നു. കൂടുതൽ ബിന്ദുക്കളുള്ള രാശി ഗോചരത്തിനും ദശയ്ക്കും ബലമുള്ളതാണ്."),
        ("अष्टकवर्ग", "अंकों की पद्धति: आठ आधार-बिंदुओं से हर ग्रह हर राशि को 0 या 1 अंक देता है। अधिक अंकों वाली राशि गोचर और दशा के लिए बलवान होती है।")),
    e("Ashtakoota", "The eight-fold Moon matching used for marriage: Varna, Vashya, Tara, Yoni, Graha Maitri, Gana, Bhakoot, Nadi, scored out of 36.", "Vedic",
        ("അഷ്ടകൂടം", "വിവാഹത്തിനുള്ള എട്ടു തരം ചന്ദ്രപ്പൊരുത്തം: വർണം, വശ്യം, താര, യോനി, ഗ്രഹമൈത്രി, ഗണം, ഭകൂടം, നാഡി; 36-ൽ എത്ര എന്ന് കണക്കാക്കുന്നു."),
        ("अष्टकूट", "विवाह के लिए चंद्र का आठ प्रकार का मिलान: वर्ण, वश्य, तारा, योनि, ग्रह मैत्री, गण, भकूट, नाड़ी; 36 में से अंक।")),
    e("Aspect", "An angle between two planets. Western: conjunction 0°, sextile 60°, square 90°, trine 120°, opposition 180°. Vedic: planets aspect whole houses by count.", "Both",
        ("ദൃഷ്ടി", "രണ്ട് ഗ്രഹങ്ങൾക്കിടയിലെ കോൺ. പാശ്ചാത്യം: യോഗം 0°, ഷഷ്ഠാംശം 60°, കേന്ദ്രം 90°, ത്രികോണം 120°, സമസപ്തമം 180°. ഭാരതീയം: ഗ്രഹങ്ങൾ എണ്ണം കൊണ്ട് മുഴുവൻ ഭാവങ്ങളെയും നോക്കുന്നു."),
        ("दृष्टि", "दो ग्रहों के बीच का कोण। पाश्चात्य: युति 0°, षष्ठांश 60°, केंद्र 90°, त्रिकोण 120°, प्रतियुति 180°। वैदिक: ग्रह गिनती से पूरे भावों को देखते हैं।")),
    e("Atmakaraka", "In Jaimini astrology, the planet with the highest degree in its sign. It stands for the self and the soul's purpose.", "Vedic",
        ("ആത്മകാരകൻ", "ജൈമിനി ജ്യോതിഷത്തിൽ, തന്റെ രാശിയിൽ ഏറ്റവും കൂടുതൽ ഡിഗ്രിയുള്ള ഗ്രഹം. ആത്മാവിനെയും ജീവിതലക്ഷ്യത്തെയും കുറിക്കുന്നു."),
        ("आत्मकारक", "जैमिनी ज्योतिष में अपनी राशि में सबसे अधिक अंश वाला ग्रह। यह आत्मा और जीवन के उद्देश्य का सूचक है।")),
    e("Ayanamsa", "The gap between the tropical zodiac (anchored to the equinox) and the sidereal zodiac (anchored to the stars). Lahiri is the Indian government standard.", "Vedic",
        ("അയനാംശം", "സായന രാശിചക്രവും (വിഷുവത്തിൽ ഉറപ്പിച്ചത്) നിരയന രാശിചക്രവും (നക്ഷത്രങ്ങളിൽ ഉറപ്പിച്ചത്) തമ്മിലുള്ള വ്യത്യാസം. ലാഹിരിയാണ് ഇന്ത്യൻ സർക്കാരിന്റെ മാനദണ്ഡം."),
        ("अयनांश", "सायन राशिचक्र (विषुव से बँधा) और निरयन राशिचक्र (तारों से बँधा) के बीच का अंतर। लाहिड़ी भारत सरकार का मानक है।")),
    e("Benefic", "A planet that tends to help: Jupiter, Venus, a waxing Moon, and Mercury when alone. Malefics are Saturn, Mars, the Sun, Rahu and Ketu.", "Vedic",
        ("ശുഭഗ്രഹം", "സഹായിക്കുന്ന സ്വഭാവമുള്ള ഗ്രഹം: വ്യാഴം, ശുക്രൻ, വളരുന്ന ചന്ദ്രൻ, തനിച്ചുള്ള ബുധൻ. പാപഗ്രഹങ്ങൾ ശനി, ചൊവ്വ, സൂര്യൻ, രാഹു, കേതു."),
        ("शुभ ग्रह", "सहायता करने वाला ग्रह: गुरु, शुक्र, बढ़ता चंद्र, और अकेला बुध। पाप ग्रह हैं शनि, मंगल, सूर्य, राहु और केतु।")),
    e("Bhava", "A house. The twelve divisions of the sky from the ascendant, each ruling an area of life.", "Vedic",
        ("ഭാവം", "ലഗ്നം മുതൽ ആകാശത്തിന്റെ പന്ത്രണ്ട് ഭാഗങ്ങളിൽ ഒന്ന്; ഓരോന്നും ജീവിതത്തിലെ ഒരു മേഖലയെ ഭരിക്കുന്നു."),
        ("भाव", "लग्न से आकाश के बारह भागों में से एक; हर भाव जीवन के एक क्षेत्र का स्वामी है।")),
    e("Cusp", "The exact starting degree of a house. KP astrology reads the sub-lord of the cusp to decide whether a matter is promised.", "Both",
        ("ഭാവസന്ധി (കസ്പ്)", "ഒരു ഭാവം തുടങ്ങുന്ന കൃത്യമായ ഡിഗ്രി. ഒരു കാര്യം വാഗ്ദത്തമാണോ എന്ന് KP ജ്യോതിഷം ഈ സന്ധിയുടെ ഉപനാഥനെ നോക്കി തീരുമാനിക്കുന്നു."),
        ("भाव संधि (कस्प)", "भाव का ठीक आरंभिक अंश। KP ज्योतिष इसी संधि के उप-स्वामी से तय करता है कि कोई बात मिलनी लिखी है या नहीं।")),
    e("Dasha", "A planetary period. The Vimshottari system gives each planet a fixed number of years in a 120-year cycle, starting from the Moon's star at birth.", "Vedic",
        ("ദശ", "ഒരു ഗ്രഹത്തിന്റെ കാലം. വിംശോത്തരി സമ്പ്രദായം 120 വർഷത്തെ ചക്രത്തിൽ ഓരോ ഗ്രഹത്തിനും നിശ്ചിത വർഷങ്ങൾ നൽകുന്നു; ജനനസമയത്തെ ചന്ദ്രന്റെ നക്ഷത്രത്തിൽ നിന്ന് തുടങ്ങുന്നു."),
        ("दशा", "ग्रह की अवधि। विंशोत्तरी पद्धति 120 वर्ष के चक्र में हर ग्रह को निश्चित वर्ष देती है, जो जन्म के समय चंद्र के नक्षत्र से शुरू होता है।")),
    e("Debilitation", "The sign where a planet is weakest (for example the Sun in Libra). The opposite of exaltation.", "Both",
        ("നീചം", "ഒരു ഗ്രഹം ഏറ്റവും ദുർബലമാകുന്ന രാശി (ഉദാഹരണം: തുലാത്തിലെ സൂര്യൻ). ഉച്ചത്തിന്റെ വിപരീതം."),
        ("नीच", "वह राशि जहाँ ग्रह सबसे दुर्बल होता है (जैसे तुला में सूर्य)। उच्च का उलटा।")),
    e("Delta T", "The difference between Earth-rotation time (UT1) and uniform atomic time (TT). The engine uses measured values, not a formula, where data exist.", "Engine",
        ("ഡെൽറ്റ T", "ഭൂമിയുടെ ഭ്രമണസമയവും (UT1) ഏകീകൃത അണുസമയവും (TT) തമ്മിലുള്ള വ്യത്യാസം. അളന്ന കണക്കുകൾ ലഭ്യമായിടത്ത് ക്ഷണ അവ തന്നെ ഉപയോഗിക്കുന്നു, സൂത്രവാക്യമല്ല."),
        ("डेल्टा T", "पृथ्वी के घूर्णन के समय (UT1) और एकसमान परमाणु समय (TT) का अंतर। जहाँ मापे गए आँकड़े हैं, वहाँ क्षण उन्हीं का उपयोग करता है, सूत्र का नहीं।")),
    e("Dosha", "A flaw in a chart or match, such as Mangal dosha (Mars in certain houses) or Nadi dosha. Most have stated cancellations.", "Vedic",
        ("ദോഷം", "ജാതകത്തിലോ പൊരുത്തത്തിലോ ഉള്ള ഒരു കുറവ്, ഉദാഹരണം ചൊവ്വാദോഷം (ചില ഭാവങ്ങളിലെ ചൊവ്വ) അല്ലെങ്കിൽ നാഡീദോഷം. മിക്കതിനും ശാസ്ത്രം പറയുന്ന പരിഹാരങ്ങളുണ്ട്."),
        ("दोष", "कुंडली या मिलान की कोई कमी, जैसे मांगलिक दोष (कुछ भावों में मंगल) या नाड़ी दोष। अधिकांश के लिए शास्त्र में बताए गए परिहार हैं।")),
    e("Ekadashi", "The eleventh tithi of each fortnight, a traditional fasting day. Observed on the day whose sunrise falls within it.", "Vedic",
        ("ഏകാദശി", "ഓരോ പക്ഷത്തിലെയും പതിനൊന്നാം തിഥി, പരമ്പരാഗത വ്രതദിനം. സൂര്യോദയം ഈ തിഥിയിൽ വരുന്ന ദിവസം ആചരിക്കുന്നു."),
        ("एकादशी", "हर पक्ष की ग्यारहवीं तिथि, पारंपरिक व्रत का दिन। जिस दिन सूर्योदय इस तिथि में हो, उस दिन मनाई जाती है।")),
    e("Ephemeris", "The table of planetary positions. Kshana uses NASA JPL's DE440s kernel (1849 to 2150) directly, the same data used for spacecraft navigation.", "Engine",
        ("പഞ്ചാംഗപ്പട്ടിക (എഫിമെറിസ്)", "ഗ്രഹസ്ഥാനങ്ങളുടെ പട്ടിക. ബഹിരാകാശ പേടകങ്ങളുടെ ദിശ നിർണയിക്കാൻ ഉപയോഗിക്കുന്ന അതേ NASA JPL DE440s ഡാറ്റ (1849 മുതൽ 2150 വരെ) ക്ഷണ നേരിട്ട് ഉപയോഗിക്കുന്നു."),
        ("ग्रह सारणी (एफ़ेमेरिस)", "ग्रहों की स्थिति की सारणी। क्षण सीधे NASA JPL का DE440s डेटा (1849 से 2150) उपयोग करता है, वही जो अंतरिक्ष यानों के मार्गदर्शन में काम आता है।")),
    e("Exaltation", "The sign where a planet is strongest (for example the Sun in Aries, the Moon in Taurus, Jupiter in Cancer).", "Both",
        ("ഉച്ചം", "ഒരു ഗ്രഹം ഏറ്റവും ബലവാനാകുന്ന രാശി (ഉദാഹരണം: മേടത്തിൽ സൂര്യൻ, ഇടവത്തിൽ ചന്ദ്രൻ, കർക്കടകത്തിൽ വ്യാഴം)."),
        ("उच्च", "वह राशि जहाँ ग्रह सबसे बलवान होता है (जैसे मेष में सूर्य, वृषभ में चंद्र, कर्क में गुरु)।")),
    e("Gochara", "Transits: where the planets are now relative to your birth Moon and chart. Sade Sati is a gochara of Saturn.", "Vedic",
        ("ഗോചരം", "ഇപ്പോൾ ഗ്രഹങ്ങൾ നിങ്ങളുടെ ജന്മചന്ദ്രനിൽ നിന്നും ജാതകത്തിൽ നിന്നും എവിടെ നിൽക്കുന്നു എന്നത്. ഏഴരശനി ശനിയുടെ ഒരു ഗോചരമാണ്."),
        ("गोचर", "ग्रह अभी आपके जन्म चंद्र और कुंडली के सापेक्ष कहाँ हैं। साढ़े साती शनि का एक गोचर है।")),
    e("Hora", "A planetary hour. Day and night are each divided into twelve; each hour belongs to a planet in a fixed sequence from the day's ruler.", "Both",
        ("ഹോര", "ഗ്രഹമണിക്കൂർ. പകലും രാത്രിയും പന്ത്രണ്ടായി വിഭജിക്കുന്നു; ദിവസാധിപനിൽ നിന്ന് നിശ്ചിത ക്രമത്തിൽ ഓരോ മണിക്കൂറും ഓരോ ഗ്രഹത്തിനുള്ളതാണ്."),
        ("होरा", "ग्रह का घंटा। दिन और रात दोनों बारह भागों में बँटते हैं; वार के स्वामी से एक निश्चित क्रम में हर घंटा किसी ग्रह का होता है।")),
    e("House system", "The method of dividing the sky into twelve houses. Placidus is the Western default; whole sign and equal are the simplest; KP uses Placidus.", "Both",
        ("ഭാവ സമ്പ്രദായം", "ആകാശത്തെ പന്ത്രണ്ട് ഭാവങ്ങളായി വിഭജിക്കുന്ന രീതി. പാശ്ചാത്യത്തിൽ സാധാരണ പ്ലാസിഡസ്; മുഴുരാശിയും സമഭാവവുമാണ് ഏറ്റവും ലളിതം; KP പ്ലാസിഡസ് ഉപയോഗിക്കുന്നു."),
        ("भाव पद्धति", "आकाश को बारह भावों में बाँटने की विधि। पाश्चात्य में सामान्यतः प्लेसिडस; पूर्ण राशि और सम भाव सबसे सरल हैं; KP प्लेसिडस का उपयोग करता है।")),
    e("Karaka", "A significator: the planet that naturally stands for a matter (Venus for marriage, Jupiter for children, Saturn for work).", "Vedic",
        ("കാരകൻ", "ഒരു കാര്യത്തെ സ്വാഭാവികമായി കുറിക്കുന്ന ഗ്രഹം (വിവാഹത്തിന് ശുക്രൻ, മക്കൾക്ക് വ്യാഴം, തൊഴിലിന് ശനി)."),
        ("कारक", "वह ग्रह जो स्वभाव से किसी बात का सूचक है (विवाह के लिए शुक्र, संतान के लिए गुरु, काम के लिए शनि)।")),
    e("Karana", "Half a tithi. Eleven karanas cycle through the month; four of them are fixed to particular half-tithis.", "Vedic",
        ("കരണം", "തിഥിയുടെ പകുതി. പതിനൊന്ന് കരണങ്ങൾ മാസം മുഴുവൻ ചുറ്റിവരുന്നു; അവയിൽ നാലെണ്ണം ചില പ്രത്യേക അർധതിഥികളിൽ സ്ഥിരമാണ്."),
        ("करण", "तिथि का आधा भाग। ग्यारह करण पूरे मास में घूमते हैं; उनमें से चार कुछ विशेष अर्ध-तिथियों पर स्थिर हैं।")),
    e("Kendra", "The angular houses 1, 4, 7 and 10, the pillars of a chart. Trikonas are 1, 5 and 9, the houses of fortune.", "Vedic",
        ("കേന്ദ്രം", "1, 4, 7, 10 ഭാവങ്ങൾ, ജാതകത്തിന്റെ തൂണുകൾ. 1, 5, 9 ത്രികോണങ്ങൾ, ഭാഗ്യത്തിന്റെ ഭാവങ്ങൾ."),
        ("केंद्र", "1, 4, 7 और 10 भाव, कुंडली के स्तंभ। 1, 5 और 9 त्रिकोण हैं, भाग्य के भाव।")),
    e("Ketu", "The Moon's south node, where the Moon's path crosses the Sun's going south. Always opposite Rahu. Stands for detachment and the past.", "Vedic",
        ("കേതു", "ചന്ദ്രന്റെ തെക്കേ പാതം, ചന്ദ്രന്റെ പാത തെക്കോട്ട് സൂര്യന്റെ പാത മുറിക്കുന്നിടം. എപ്പോഴും രാഹുവിന് നേരെ എതിർവശം. വിരക്തിയെയും കഴിഞ്ഞകാലത്തെയും കുറിക്കുന്നു."),
        ("केतु", "चंद्र का दक्षिणी पात, जहाँ चंद्र का मार्ग दक्षिण की ओर सूर्य के मार्ग को काटता है। सदा राहु के ठीक सामने। वैराग्य और अतीत का सूचक।")),
    e("KP", "Krishnamurti Paddhati: a Vedic method that divides each star into nine unequal sub-parts and reads the sub-lord of each cusp. Precise and rule-based.", "Vedic",
        ("KP", "കൃഷ്ണമൂർത്തി പദ്ധതി: ഓരോ നക്ഷത്രത്തെയും ഒമ്പത് അസമ ഉപഭാഗങ്ങളായി വിഭജിച്ച് ഓരോ ഭാവസന്ധിയുടെയും ഉപനാഥനെ വായിക്കുന്ന ഭാരതീയ രീതി. കൃത്യവും നിയമബദ്ധവും."),
        ("KP", "कृष्णमूर्ति पद्धति: एक वैदिक विधि जो हर नक्षत्र को नौ असमान उप-भागों में बाँटती है और हर भाव संधि का उप-स्वामी पढ़ती है। सटीक और नियमबद्ध।")),
    e("Kundli", "The Vedic birth chart, drawn as a square (North or South Indian style) rather than a wheel.", "Vedic",
        ("ജാതകം", "ചക്രത്തിനു പകരം ചതുരമായി (വടക്കേ ഇന്ത്യൻ അല്ലെങ്കിൽ തെക്കേ ഇന്ത്യൻ ശൈലിയിൽ) വരയ്ക്കുന്ന ജന്മചക്രം."),
        ("कुंडली", "वैदिक जन्म चक्र, जो पहिये के बजाय वर्ग के रूप में (उत्तर या दक्षिण भारतीय शैली में) बनाया जाता है।")),
    e("Lagna", "The ascendant. See Ascendant.", "Vedic",
        ("ലഗ്നം", "ഉദയരാശി. ലഗ്നം (ഉദയരാശി) കാണുക."),
        ("लग्न", "उदय राशि। लग्न (उदय राशि) देखें।")),
    e("Mahadasha", "A major planetary period, from 6 years (Sun) to 20 years (Venus).", "Vedic",
        ("മഹാദശ", "ഒരു ഗ്രഹത്തിന്റെ വലിയ കാലം, 6 വർഷം (സൂര്യൻ) മുതൽ 20 വർഷം (ശുക്രൻ) വരെ."),
        ("महादशा", "ग्रह की बड़ी अवधि, 6 वर्ष (सूर्य) से 20 वर्ष (शुक्र) तक।")),
    e("Mangal dosha", "Mars in the 1st, 2nd, 4th, 7th, 8th or 12th house from the ascendant, Moon or Venus. Read as strain in marriage unless cancelled.", "Vedic",
        ("ചൊവ്വാദോഷം", "ലഗ്നം, ചന്ദ്രൻ അല്ലെങ്കിൽ ശുക്രനിൽ നിന്ന് 1, 2, 4, 7, 8, 12 ഭാവങ്ങളിലെ ചൊവ്വ. പരിഹാരം ഇല്ലെങ്കിൽ ദാമ്പത്യത്തിലെ സമ്മർദമായി വായിക്കുന്നു."),
        ("मांगलिक दोष", "लग्न, चंद्र या शुक्र से 1, 2, 4, 7, 8 या 12वें भाव में मंगल। परिहार न हो तो विवाह में तनाव माना जाता है।")),
    e("Muhurta", "A chosen auspicious moment for starting something. Judged from the tithi, star, day, hora and the Moon's standing for the person.", "Vedic",
        ("മുഹൂർത്തം", "എന്തെങ്കിലും തുടങ്ങാൻ തിരഞ്ഞെടുക്കുന്ന ശുഭസമയം. തിഥി, നക്ഷത്രം, ദിവസം, ഹോര, ആ വ്യക്തിക്ക് ചന്ദ്രന്റെ നില എന്നിവ നോക്കി നിശ്ചയിക്കുന്നു."),
        ("मुहूर्त", "किसी काम को शुरू करने के लिए चुना गया शुभ क्षण। तिथि, नक्षत्र, वार, होरा और व्यक्ति के लिए चंद्र की स्थिति से तय होता है।")),
    e("Nadi", "One of three pulse groups of the 27 stars. Two Moons in the same Nadi is the heaviest matching flaw.", "Vedic",
        ("നാഡി", "27 നക്ഷത്രങ്ങളുടെ മൂന്ന് നാഡീഗണങ്ങളിൽ ഒന്ന്. രണ്ടുപേരുടെയും ചന്ദ്രൻ ഒരേ നാഡിയിലാകുന്നത് പൊരുത്തത്തിലെ ഏറ്റവും വലിയ കുറവാണ്."),
        ("नाड़ी", "27 नक्षत्रों के तीन नाड़ी समूहों में से एक। दोनों के चंद्र एक ही नाड़ी में हों तो यह मिलान का सबसे भारी दोष है।")),
    e("Nakshatra", "One of 27 lunar stars, each 13°20′ wide. The Moon's star at birth sets your dasha, your temperament and your naming syllable.", "Vedic",
        ("നക്ഷത്രം (നാൾ)", "27 ചാന്ദ്ര നക്ഷത്രങ്ങളിൽ ഒന്ന്, ഓരോന്നും 13°20′ വീതി. ജനനസമയത്തെ ചന്ദ്രന്റെ നക്ഷത്രം നിങ്ങളുടെ ദശയും സ്വഭാവവും പേരിന്റെ ആദ്യാക്ഷരവും നിശ്ചയിക്കുന്നു."),
        ("नक्षत्र", "27 चंद्र नक्षत्रों में से एक, हर एक 13°20′ चौड़ा। जन्म के समय चंद्र का नक्षत्र आपकी दशा, स्वभाव और नाम का अक्षर तय करता है।")),
    e("Navamsa", "The ninth divisional chart (D9), each sign split into nine. Read for marriage and for the inner strength of planets.", "Vedic",
        ("നവാംശം", "ഒമ്പതാം വർഗചക്രം (D9), ഓരോ രാശിയും ഒമ്പതായി വിഭജിച്ചത്. വിവാഹത്തിനും ഗ്രഹങ്ങളുടെ ഉള്ളിലെ ബലത്തിനും നോക്കുന്നു."),
        ("नवांश", "नौवाँ वर्ग चक्र (D9), हर राशि नौ भागों में बँटी। विवाह और ग्रहों के भीतरी बल के लिए देखा जाता है।")),
    e("Node", "Rahu and Ketu, where the Moon's orbit crosses the Sun's path. The true node wobbles; the mean node is its average. Both are shown.", "Both",
        ("പാതം", "രാഹുവും കേതുവും, ചന്ദ്രന്റെ ഭ്രമണപഥം സൂര്യന്റെ പാത മുറിക്കുന്നിടങ്ങൾ. സ്പഷ്ട പാതം അല്പം ആടിക്കൊണ്ടിരിക്കും; മധ്യമ പാതം അതിന്റെ ശരാശരി. രണ്ടും കാണിക്കുന്നു."),
        ("पात", "राहु और केतु, जहाँ चंद्र की कक्षा सूर्य के मार्ग को काटती है। स्पष्ट पात डोलता रहता है; मध्यम पात उसका औसत है। दोनों दिखाए जाते हैं।")),
    e("Pada", "A quarter of a nakshatra, 3°20′ wide. The four padas map onto the navamsa.", "Vedic",
        ("പാദം", "നക്ഷത്രത്തിന്റെ നാലിലൊന്ന്, 3°20′ വീതി. നാല് പാദങ്ങൾ നവാംശത്തിലേക്ക് ചേരുന്നു."),
        ("पाद (चरण)", "नक्षत्र का चौथाई भाग, 3°20′ चौड़ा। चार पाद नवांश से मेल खाते हैं।")),
    e("Panchanga", "The five limbs of the Vedic day: tithi, vara (weekday), nakshatra, yoga and karana.", "Vedic",
        ("പഞ്ചാംഗം", "ദിവസത്തിന്റെ അഞ്ച് അംഗങ്ങൾ: തിഥി, വാരം (ആഴ്ചദിവസം), നക്ഷത്രം, യോഗം, കരണം."),
        ("पंचांग", "दिन के पाँच अंग: तिथि, वार, नक्षत्र, योग और करण।")),
    e("Parashari", "The classical Vedic system from Parashara's text (BPHS): house lords, dignities, aspects and yogas.", "Vedic",
        ("പാരാശരി", "പരാശരന്റെ ഗ്രന്ഥത്തിൽ (BPHS) നിന്നുള്ള ശാസ്ത്രീയ രീതി: ഭാവാധിപന്മാർ, ഗ്രഹബലങ്ങൾ, ദൃഷ്ടികൾ, യോഗങ്ങൾ."),
        ("पाराशरी", "पराशर के ग्रंथ (BPHS) से शास्त्रीय वैदिक पद्धति: भावेश, ग्रह की स्थिति-बल, दृष्टियाँ और योग।")),
    e("Parihara", "Remedy. In Kshana the first remedies are actions that match the cause; rituals are listed but never required.", "Vedic",
        ("പരിഹാരം", "പ്രതിവിധി. ക്ഷണയിൽ ആദ്യത്തെ പരിഹാരങ്ങൾ കാരണത്തിന് ചേരുന്ന പ്രവൃത്തികളാണ്; ചടങ്ങുകൾ പട്ടികയിലുണ്ട്, പക്ഷേ ഒരിക്കലും നിർബന്ധമല്ല."),
        ("परिहार", "उपाय। क्षण में पहले उपाय वे कर्म हैं जो कारण से मेल खाते हैं; अनुष्ठान सूची में हैं, पर कभी अनिवार्य नहीं।")),
    e("Prashna", "A question chart cast for the moment of asking. KP prashna uses a number from 1 to 249 chosen by the asker.", "Vedic",
        ("പ്രശ്നം", "ചോദിക്കുന്ന നിമിഷത്തിന് കുറിക്കുന്ന ചക്രം. KP പ്രശ്നത്തിൽ ചോദിക്കുന്നയാൾ 1 മുതൽ 249 വരെ ഒരു സംഖ്യ തിരഞ്ഞെടുക്കുന്നു."),
        ("प्रश्न", "पूछने के क्षण की कुंडली। KP प्रश्न में पूछने वाला 1 से 249 तक कोई संख्या चुनता है।")),
    e("Rahu", "The Moon's north node. Stands for hunger, the foreign, the new. Always opposite Ketu.", "Vedic",
        ("രാഹു", "ചന്ദ്രന്റെ വടക്കേ പാതം. ആർത്തിയെയും വിദേശത്തെയും പുതുമയെയും കുറിക്കുന്നു. എപ്പോഴും കേതുവിന് നേരെ എതിർവശം."),
        ("राहु", "चंद्र का उत्तरी पात। भूख, विदेश और नएपन का सूचक। सदा केतु के ठीक सामने।")),
    e("Rahu kala", "A daily window of about ninety minutes ruled by Rahu, avoided for starting things. Its position depends on the weekday and the actual sunrise.", "Vedic",
        ("രാഹുകാലം", "ദിവസവും ഏകദേശം തൊണ്ണൂറ് മിനിറ്റ് നീളുന്ന, രാഹു ഭരിക്കുന്ന സമയം; പുതിയ കാര്യങ്ങൾ തുടങ്ങാൻ ഒഴിവാക്കുന്നു. ആഴ്ചദിവസവും അന്നത്തെ യഥാർഥ സൂര്യോദയവും അനുസരിച്ച് അതിന്റെ സ്ഥാനം മാറും."),
        ("राहुकाल", "रोज़ लगभग नब्बे मिनट का राहु का समय, जिसमें नए काम शुरू करने से बचा जाता है। इसकी जगह वार और उस दिन के वास्तविक सूर्योदय पर निर्भर करती है।")),
    e("Rectification", "Fixing an uncertain birth time from dated life events: the engine tests each candidate time against the events.", "Both",
        ("ജനനസമയ ശോധന", "തീയതിയുള്ള ജീവിതസംഭവങ്ങൾ വെച്ച് ഉറപ്പില്ലാത്ത ജനനസമയം ശരിയാക്കൽ: സാധ്യമായ ഓരോ സമയവും ആ സംഭവങ്ങളുമായി ഒത്തുനോക്കുന്നു."),
        ("जन्म समय शोधन", "तारीख वाली जीवन घटनाओं से अनिश्चित जन्म समय ठीक करना: हर संभावित समय को उन घटनाओं से मिलाकर देखा जाता है।")),
    e("Retrograde", "A planet appearing to move backwards against the stars. An optical effect of the orbits; read as turning inward or revisiting.", "Both",
        ("വക്രം", "നക്ഷത്രങ്ങൾക്കെതിരെ ഒരു ഗ്രഹം പിന്നോട്ട് നീങ്ങുന്നതായി തോന്നുന്നത്. ഭ്രമണപഥങ്ങൾ കൊണ്ടുള്ള കാഴ്ചയുടെ പ്രതിഭാസം; ഉള്ളിലേക്ക് തിരിയുന്നതായോ പഴയതിലേക്ക് മടങ്ങുന്നതായോ വായിക്കുന്നു."),
        ("वक्री", "ग्रह का तारों के सामने पीछे चलता दिखना। कक्षाओं के कारण दिखने वाला प्रभाव; भीतर मुड़ने या पुरानी बात पर लौटने के रूप में पढ़ा जाता है।")),
    e("Sade Sati", "The seven and a half years when Saturn passes through the sign before, over and after your birth Moon. Three phases of about 2½ years each.", "Vedic",
        ("ഏഴരശനി", "ശനി നിങ്ങളുടെ ജന്മചന്ദ്രന്റെ മുൻപിലെ രാശിയിലൂടെയും അതിനു മുകളിലൂടെയും ശേഷമുള്ള രാശിയിലൂടെയും കടന്നുപോകുന്ന ഏഴര വർഷം. ഏകദേശം രണ്ടര വർഷം വീതമുള്ള മൂന്ന് ഘട്ടങ്ങൾ."),
        ("साढ़े साती", "वे साढ़े सात वर्ष जब शनि आपके जन्म चंद्र से पहले की राशि, उसी राशि और बाद की राशि से गुज़रता है। लगभग ढाई-ढाई वर्ष के तीन चरण।")),
    e("Sankranti", "The Sun entering a sidereal sign. Makara Sankranti (Capricorn) is the best known.", "Vedic",
        ("സംക്രമം", "സൂര്യൻ ഒരു നിരയന രാശിയിൽ പ്രവേശിക്കുന്നത്. മകരസംക്രമമാണ് ഏറ്റവും പ്രസിദ്ധം."),
        ("संक्रांति", "सूर्य का किसी निरयन राशि में प्रवेश। मकर संक्रांति सबसे प्रसिद्ध है।")),
    e("Sidereal", "Measured against the fixed stars. The Vedic zodiac is sidereal; the Western zodiac is tropical.", "Both",
        ("നിരയനം", "സ്ഥിരനക്ഷത്രങ്ങളെ ആധാരമാക്കി അളക്കുന്നത്. ഭാരതീയ രാശിചക്രം നിരയനമാണ്; പാശ്ചാത്യ രാശിചക്രം സായനവും."),
        ("निरयन", "स्थिर तारों के आधार पर नापा गया। वैदिक राशिचक्र निरयन है; पाश्चात्य राशिचक्र सायन।")),
    e("Sub-lord", "In KP, the lord of the sub-division of a star where a point falls. The sub-lord of a cusp decides whether the house delivers.", "Vedic",
        ("ഉപനാഥൻ", "KP-യിൽ, ഒരു ബിന്ദു വീഴുന്ന നക്ഷത്ര ഉപഭാഗത്തിന്റെ അധിപൻ. ഭാവസന്ധിയുടെ ഉപനാഥനാണ് ആ ഭാവം ഫലം തരുമോ എന്ന് തീരുമാനിക്കുന്നത്."),
        ("उप-स्वामी", "KP में, नक्षत्र के उस उप-भाग का स्वामी जहाँ कोई बिंदु पड़ता है। भाव संधि का उप-स्वामी तय करता है कि भाव फल देगा या नहीं।")),
    e("Synastry", "Western compatibility: the aspects between one person's planets and another's.", "Western",
        ("സിനാസ്ട്രി", "പാശ്ചാത്യ പൊരുത്തം: ഒരാളുടെ ഗ്രഹങ്ങളും മറ്റൊരാളുടെ ഗ്രഹങ്ങളും തമ്മിലുള്ള ദൃഷ്ടികൾ."),
        ("सिनैस्ट्री", "पाश्चात्य मिलान: एक व्यक्ति के ग्रहों और दूसरे के ग्रहों के बीच की दृष्टियाँ।")),
    e("Tithi", "A lunar day: every 12° the Moon gains on the Sun. Thirty tithis make a month; the full Moon is the 15th, the new Moon the 30th.", "Vedic",
        ("തിഥി", "ചാന്ദ്രദിനം: ചന്ദ്രൻ സൂര്യനെക്കാൾ 12° മുന്നേറുന്ന ഓരോ ഘട്ടവും. മുപ്പത് തിഥികൾ ഒരു മാസം; വെളുത്തവാവ് 15-ാമത്തേത്, കറുത്തവാവ് 30-ാമത്തേത്."),
        ("तिथि", "चंद्र दिवस: चंद्र सूर्य से हर 12° आगे बढ़ता है। तीस तिथियों का एक मास; पूर्णिमा 15वीं है, अमावस्या 30वीं।")),
    e("Transit", "Where a planet is now, placed in your birth chart. See Gochara.", "Both",
        ("ഗോചാരം (ട്രാൻസിറ്റ്)", "ഒരു ഗ്രഹം ഇപ്പോൾ എവിടെ, അത് നിങ്ങളുടെ ജാതകത്തിൽ വെച്ചു നോക്കുമ്പോൾ. ഗോചരം കാണുക."),
        ("गोचर (ट्रांज़िट)", "कोई ग्रह अभी कहाँ है, आपकी जन्म कुंडली में रखकर देखा गया। गोचर देखें।")),
    e("Tropical", "Measured from the spring equinox. The Western zodiac. Slowly drifts against the stars (precession).", "Western",
        ("സായനം", "വസന്തവിഷുവത്തിൽ നിന്ന് അളക്കുന്നത്. പാശ്ചാത്യ രാശിചക്രം. നക്ഷത്രങ്ങൾക്കെതിരെ പതുക്കെ നീങ്ങുന്നു (അയനചലനം)."),
        ("सायन", "वसंत विषुव से नापा गया। पाश्चात्य राशिचक्र। तारों के सामने धीरे-धीरे खिसकता है (अयन चलन)।")),
    e("Varga", "A divisional chart. D1 is the birth chart; D9 the navamsa; D10 for career; D7 children; D4 property; D24 learning; D30 misfortune.", "Vedic",
        ("വർഗം", "വിഭാഗചക്രം. D1 ജന്മചക്രം; D9 നവാംശം; D10 തൊഴിലിന്; D7 മക്കൾ; D4 സ്വത്ത്; D24 പഠനം; D30 ദുരിതങ്ങൾ."),
        ("वर्ग", "विभागीय चक्र। D1 जन्म कुंडली; D9 नवांश; D10 करियर के लिए; D7 संतान; D4 संपत्ति; D24 विद्या; D30 कष्ट।")),
    e("Vimshottari", "The 120-year dasha system. See Dasha.", "Vedic",
        ("വിംശോത്തരി", "120 വർഷത്തെ ദശാസമ്പ്രദായം. ദശ കാണുക."),
        ("विंशोत्तरी", "120 वर्ष की दशा पद्धति। दशा देखें।")),
    e("Yoga (panchanga)", "One of 27 daily yogas from the sum of the Sun's and Moon's longitudes. Some are avoided for new beginnings.", "Vedic",
        ("യോഗം (പഞ്ചാംഗം)", "സൂര്യന്റെയും ചന്ദ്രന്റെയും സ്ഫുടങ്ങളുടെ തുകയിൽ നിന്നുള്ള 27 നിത്യയോഗങ്ങളിൽ ഒന്ന്. ചിലത് പുതിയ തുടക്കങ്ങൾക്ക് ഒഴിവാക്കുന്നു."),
        ("योग (पंचांग)", "सूर्य और चंद्र के भोगांशों के योग से बने 27 नित्य योगों में से एक। कुछ में नई शुरुआत से बचा जाता है।")),
    e("Yoga (chart)", "A planetary combination with a classical result, such as Gaja Kesari (Jupiter in a kendra from the Moon) or a Raja yoga.", "Vedic",
        ("യോഗം (ജാതകം)", "ശാസ്ത്രം ഫലം പറയുന്ന ഗ്രഹസംയോഗം, ഉദാഹരണം ഗജകേസരി (ചന്ദ്രനിൽ നിന്ന് കേന്ദ്രത്തിൽ വ്യാഴം) അല്ലെങ്കിൽ രാജയോഗം."),
        ("योग (कुंडली)", "शास्त्रीय फल वाला ग्रह-संयोग, जैसे गजकेसरी (चंद्र से केंद्र में गुरु) या राजयोग।")),
    e("Yoni", "The animal nature of each nakshatra, used in matching for physical compatibility. Fourteen animals in seven pairs.", "Vedic",
        ("യോനി", "ഓരോ നക്ഷത്രത്തിന്റെയും മൃഗസ്വഭാവം, ശാരീരിക പൊരുത്തം നോക്കാൻ ഉപയോഗിക്കുന്നു. ഏഴ് ജോടികളായി പതിനാല് മൃഗങ്ങൾ."),
        ("योनि", "हर नक्षत्र का पशु स्वभाव, जो मिलान में शारीरिक अनुकूलता के लिए देखा जाता है। सात जोड़ों में चौदह पशु।")),
];

/// All entries in English, alphabetical by term.
pub fn glossary() -> Vec<GlossaryEntry> {
    glossary_in(Lang::En)
}

/// All entries in `lang`, in the English alphabetical order (so the list reads the same
/// way in every language). The tradition stays the English key; see [`tradition_name`].
pub fn glossary_in(lang: Lang) -> Vec<GlossaryEntry> {
    let mut rows: Vec<&Row> = ROWS.iter().collect();
    rows.sort_by(|a, b| a.en.0.cmp(b.en.0));
    rows.into_iter()
        .map(|r| {
            let (term, meaning) = match lang {
                Lang::En => r.en,
                Lang::Ml => r.ml,
                Lang::Hi => r.hi,
            };
            GlossaryEntry {
                term,
                meaning,
                tradition: r.tradition,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glossary_is_sorted_and_nonempty() {
        let g = glossary();
        assert!(g.len() >= 50);
        assert!(g.windows(2).all(|w| w[0].term <= w[1].term));
    }

    #[test]
    fn every_language_has_every_entry() {
        let en = glossary();
        for lang in [Lang::Ml, Lang::Hi] {
            let g = glossary_in(lang);
            assert_eq!(g.len(), en.len());
            for (a, b) in en.iter().zip(&g) {
                assert_eq!(a.tradition, b.tradition);
                assert!(!b.term.is_empty() && !b.meaning.is_empty());
                assert!(!b.meaning.is_ascii(), "{:?} {}", lang, a.term);
            }
            assert_ne!(tradition_name("Vedic", lang), "Vedic");
        }
        assert_eq!(tradition_name("Vedic", Lang::En), "Vedic");
    }
}
