//! The future, as a list: what is coming in the months and years ahead, in order, each
//! with its dates, how strong it is, and what it will mean for the person's life. Built
//! from the slow monthly score of each life the person cares about (windows that open
//! and stretches that press), the turns of the long chapters of life, and the start and
//! end of the long testing periods. Plain words only; nothing names the astrology.

use crate::forecast::{slow_series, ForecastError, Life, Natal};
use crate::gochara::SaturnPhase;
use crate::lang::{chapter_prediction, chapter_theme, life_name, long_test, long_test_prediction, month_year};
use crate::plain::Lang;
use crate::transit::sign_changes;
use kshana_ephem::Spk;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa, DashaLord};

/// One thing that is coming.
#[derive(Clone, Debug, PartialEq)]
pub struct FutureEvent {
    /// When it begins.
    pub from: Instant,
    /// When it ends (for a turn of chapter, when the new chapter ends).
    pub to: Instant,
    /// The part of life it is about; none for a turn that touches the whole of life.
    pub life: Option<Life>,
    /// `opening`, `pressure`, `chapter`, `test_begins` or `test_ends`.
    pub kind: &'static str,
    /// The prediction, one sentence.
    pub headline: String,
    /// What it will be like and what to do, a sentence or two.
    pub detail: String,
    /// How strong, `[0, 1]`.
    pub strength: f64,
    /// The month it is at its strongest (or deepest).
    pub peak: Option<Instant>,
}

type T3 = [&'static str; 3];

fn opening_words(life: Life) -> T3 {
    match life {
        Life::Love => [
            "Love opens up: a good time to meet someone, to mend a bond, or to settle a question of marriage.",
            "പ്രണയം തുറക്കുന്നു: ഒരാളെ കണ്ടുമുട്ടാനോ, ഒരു ബന്ധം നന്നാക്കാനോ, വിവാഹക്കാര്യം തീരുമാനിക്കാനോ നല്ല സമയം.",
            "प्रेम खुलता है: किसी से मिलने, रिश्ता सुधारने या शादी की बात तय करने का अच्छा समय।",
        ],
        Life::Career => [
            "Work moves up: a new role, a raise or recognition is likely. Apply, ask and be seen.",
            "ജോലിയിൽ ഉയർച്ച: പുതിയ സ്ഥാനമോ ശമ്പളവർധനയോ അംഗീകാരമോ വരാൻ സാധ്യത. അപേക്ഷിക്കുക, ചോദിക്കുക.",
            "काम में तरक्की: नई भूमिका, वेतन वृद्धि या पहचान मिलने की संभावना। आवेदन करें, माँगें।",
        ],
        Life::Money => [
            "Money improves: income rises or a stuck payment comes in. A good time to save and to clear debts.",
            "പണം മെച്ചപ്പെടുന്നു: വരുമാനം കൂടുകയോ കുടുങ്ങിയ പണം കിട്ടുകയോ ചെയ്യും. സമ്പാദിക്കാനും കടം തീർക്കാനും നല്ല സമയം.",
            "पैसा सुधरता है: आमदनी बढ़ेगी या अटका पैसा आएगा। बचत करने और कर्ज़ चुकाने का अच्छा समय।",
        ],
        Life::Health => [
            "Health strengthens: energy comes back. A good time to start a treatment or a healthy habit.",
            "ആരോഗ്യം മെച്ചപ്പെടുന്നു: ഊർജ്ജം തിരിച്ചുവരും. ചികിത്സയോ നല്ല ശീലമോ തുടങ്ങാൻ നല്ല സമയം.",
            "सेहत मज़बूत होती है: ऊर्जा लौटेगी। इलाज या अच्छी आदत शुरू करने का अच्छा समय।",
        ],
        Life::Family => [
            "Family life warms: gatherings, reconciliations and good news at home.",
            "കുടുംബജീവിതം ഊഷ്മളമാകുന്നു: ഒത്തുചേരലുകൾ, പിണക്കം തീരൽ, വീട്ടിൽ നല്ല വാർത്തകൾ.",
            "पारिवारिक जीवन में गर्माहट: मिलना-जुलना, सुलह और घर में अच्छी खबरें।",
        ],
        Life::Study => [
            "Study goes well: exams, courses and admissions favour you.",
            "പഠനം നന്നായി പോകും: പരീക്ഷകളും കോഴ്സുകളും പ്രവേശനവും അനുകൂലം.",
            "पढ़ाई अच्छी चलेगी: परीक्षा, कोर्स और दाखिले आपके पक्ष में।",
        ],
        Life::Home => [
            "Property moves: buying, moving, building or repairs go through.",
            "വസ്തുകാര്യങ്ങൾ നീങ്ങും: വാങ്ങൽ, താമസം മാറൽ, പണി, അറ്റകുറ്റപ്പണി എല്ലാം നടക്കും.",
            "संपत्ति के काम बनेंगे: खरीद, घर बदलना, निर्माण या मरम्मत पूरी होगी।",
        ],
        Life::Travel => [
            "Travel opens: journeys, visas and moves abroad go smoothly.",
            "യാത്ര തുറക്കുന്നു: യാത്രകളും വിസയും വിദേശത്തേക്കുള്ള മാറ്റവും സുഗമമാകും.",
            "यात्रा खुलती है: सफ़र, वीज़ा और विदेश जाना आसानी से होगा।",
        ],
        Life::Children => [
            "Good news about children: their progress, or, for those hoping, a favourable time.",
            "കുട്ടികളെക്കുറിച്ച് നല്ല വാർത്ത: അവരുടെ പുരോഗതി, അല്ലെങ്കിൽ കാത്തിരിക്കുന്നവർക്ക് അനുകൂല സമയം.",
            "बच्चों के बारे में अच्छी खबर: उनकी तरक्की, या उम्मीद रखने वालों के लिए अनुकूल समय।",
        ],
    }
}

fn pressure_words(life: Life) -> T3 {
    match life {
        Life::Love => [
            "Love is tested: misunderstandings and distance are likely. Avoid ultimatums and big decisions.",
            "പ്രണയം പരീക്ഷിക്കപ്പെടും: തെറ്റിദ്ധാരണയും അകലവും വരാം. അന്ത്യശാസനങ്ങളും വലിയ തീരുമാനങ്ങളും ഒഴിവാക്കുക.",
            "प्रेम की परीक्षा: गलतफ़हमी और दूरी हो सकती है। अल्टीमेटम और बड़े फ़ैसलों से बचें।",
        ],
        Life::Career => [
            "Work gets heavy: delays, pressure from seniors or a stalled role. Keep your head down; do not resign in anger.",
            "ജോലി ഭാരമാകും: താമസം, മേലുദ്യോഗസ്ഥരുടെ സമ്മർദ്ദം, മുരടിപ്പ്. ശാന്തമായി ചെയ്യുക; ദേഷ്യത്തിൽ രാജിവെക്കരുത്.",
            "काम भारी होगा: देरी, बड़ों का दबाव या रुकी तरक्की। शांत रहें; गुस्से में इस्तीफ़ा न दें।",
        ],
        Life::Money => [
            "Money tightens: expenses rise and payments get stuck. Avoid loans, lending and big purchases.",
            "പണം ഞെരുങ്ങും: ചെലവ് കൂടും, പണം കുടുങ്ങും. കടം വാങ്ങലും കൊടുക്കലും വലിയ വാങ്ങലുകളും ഒഴിവാക്കുക.",
            "पैसा तंग होगा: खर्च बढ़ेंगे, भुगतान अटकेंगे। कर्ज़ लेने-देने और बड़ी खरीद से बचें।",
        ],
        Life::Health => [
            "Health needs care: low energy and small ailments. Get check-ups, rest more, do not ignore pain.",
            "ആരോഗ്യത്തിന് ശ്രദ്ധ വേണം: ക്ഷീണവും ചെറിയ അസുഖങ്ങളും. പരിശോധന നടത്തുക, കൂടുതൽ വിശ്രമിക്കുക.",
            "सेहत का ध्यान रखें: थकान और छोटी बीमारियाँ। जाँच कराएँ, आराम करें, दर्द को अनदेखा न करें।",
        ],
        Life::Family => [
            "Home is tense: arguments with elders or relatives. Choose patience over winning.",
            "വീട്ടിൽ പിരിമുറുക്കം: മുതിർന്നവരുമായോ ബന്ധുക്കളുമായോ തർക്കം. ജയിക്കുന്നതിനേക്കാൾ ക്ഷമ തിരഞ്ഞെടുക്കുക.",
            "घर में तनाव: बड़ों या रिश्तेदारों से बहस। जीतने से ज़्यादा धैर्य चुनें।",
        ],
        Life::Study => [
            "Study slows: concentration suffers. Prepare earlier than usual.",
            "പഠനം മന്ദഗതിയിൽ: ശ്രദ്ധ കുറയും. പതിവിലും നേരത്തെ തയ്യാറെടുക്കുക.",
            "पढ़ाई धीमी: ध्यान भटकेगा। हमेशा से पहले तैयारी करें।",
        ],
        Life::Home => [
            "Property matters stall: disputes and delays with land, rent or repairs. Check every paper.",
            "വസ്തുകാര്യങ്ങൾ കുടുങ്ങും: ഭൂമി, വാടക, അറ്റകുറ്റപ്പണി എന്നിവയിൽ തർക്കവും താമസവും. എല്ലാ രേഖകളും പരിശോധിക്കുക.",
            "संपत्ति के काम अटकेंगे: ज़मीन, किराए या मरम्मत में विवाद और देरी। हर कागज़ जाँचें।",
        ],
        Life::Travel => [
            "Travel is troublesome: delays and cancellations. Keep plans flexible.",
            "യാത്ര പ്രശ്നമാകും: താമസവും റദ്ദാക്കലും. പദ്ധതികൾ അയവുള്ളതാക്കുക.",
            "यात्रा में परेशानी: देरी और रद्द होना। योजनाएँ लचीली रखें।",
        ],
        Life::Children => [
            "Children need attention: health or schooling worries. Stay close.",
            "കുട്ടികൾക്ക് ശ്രദ്ധ വേണം: ആരോഗ്യത്തെയോ പഠനത്തെയോ കുറിച്ച് ആശങ്ക. അടുത്തുണ്ടാകുക.",
            "बच्चों पर ध्यान दें: सेहत या पढ़ाई की चिंता। पास रहें।",
        ],
    }
}

fn between(lang: Lang, a: Instant, b: Instant, lon: f64) -> String {
    let (x, y) = (month_year(lang, a, lon), month_year(lang, b, lon));
    if x == y {
        return match lang {
            Lang::En => format!("In {x}"),
            Lang::Ml => format!("{x}-ൽ"),
            Lang::Hi => format!("{x} में"),
        };
    }
    match lang {
        Lang::En => format!("From {x} to {y}"),
        Lang::Ml => format!("{x} മുതൽ {y} വരെ"),
        Lang::Hi => format!("{x} से {y} तक"),
    }
}

fn strength_words(lang: Lang, s: f64, opening: bool) -> &'static str {
    match (s >= 0.66, s >= 0.33, opening) {
        (true, _, true) => lang.pick(["A strong opening.", "ശക്തമായ അവസരം.", "एक मज़बूत मौका।"]),
        (false, true, true) => lang.pick(["A clear opening.", "വ്യക്തമായ അവസരം.", "एक साफ़ मौका।"]),
        (false, false, true) => lang.pick(["A mild opening.", "ചെറിയ അവസരം.", "एक हल्का मौका।"]),
        (true, _, false) => lang.pick(["It presses hard.", "ശക്തമായി ബാധിക്കും.", "यह भारी पड़ेगा।"]),
        (false, true, false) => lang.pick(["It presses moderately.", "മിതമായി ബാധിക്കും.", "यह कुछ भारी पड़ेगा।"]),
        (false, false, false) => lang.pick(["It presses lightly.", "ചെറുതായി ബാധിക്കും.", "यह हल्का असर करेगा।"]),
    }
}

/// What is coming in the `months` from `at`, for the lives in `focus`, in order.
#[allow(clippy::too_many_arguments)]
pub fn future(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    at: Instant,
    months: usize,
    focus: &[Life],
    lang: Lang,
) -> Result<Vec<FutureEvent>, ForecastError> {
    let lon = obs.longitude_deg;
    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let months = months.clamp(3, 60);
    let slow = slow_series(spk, eop, natal, obs, ayanamsa, at, month, months + 1)?;
    let end = slow.last().map(|s| s.0).unwrap_or(at);
    let mut out: Vec<FutureEvent> = Vec::new();

    // 1. Each life's windows: two or more months above good, or below slow.
    for &life in focus {
        let i = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        let vals: Vec<(Instant, f64)> = slow.iter().map(|(t, v)| (*t, v[i])).collect();
        let mean = vals.iter().map(|x| x.1).sum::<f64>() / vals.len().max(1) as f64;
        // Relative to the person's own level as well as the absolute bands, so a life that
        // sits low for years still shows its better and its worse stretches.
        let hi = (mean + 0.05).max(0.53);
        let lo = (mean - 0.05).min(0.44);
        for opening in [true, false] {
            let mut k = 0;
            while k < vals.len() {
                let inside = |v: f64| if opening { v >= hi } else { v <= lo };
                if !inside(vals[k].1) {
                    k += 1;
                    continue;
                }
                let start = k;
                while k < vals.len() && inside(vals[k].1) {
                    k += 1;
                }
                let run = &vals[start..k];
                if run.len() < 2 {
                    continue;
                }
                let peak = if opening {
                    run.iter().cloned().fold(run[0], |a, x| if x.1 > a.1 { x } else { a })
                } else {
                    run.iter().cloned().fold(run[0], |a, x| if x.1 < a.1 { x } else { a })
                };
                let depth = if opening { peak.1 - 0.5 } else { 0.5 - peak.1 };
                let strength = (depth / 0.25).clamp(0.0, 1.0);
                let from = run[0].0;
                let to = run[run.len() - 1].0.add_ns(month);
                let words = if opening { opening_words(life) } else { pressure_words(life) };
                let headline = format!("{}: {}", between(lang, from, to, lon), lang.pick(words));
                let peak_words = match (lang, opening) {
                    (Lang::En, true) => format!("Strongest around {}.", month_year(lang, peak.0, lon)),
                    (Lang::En, false) => format!("Hardest around {}.", month_year(lang, peak.0, lon)),
                    (Lang::Ml, true) => format!("{}-ഓടെ ഏറ്റവും ശക്തം.", month_year(lang, peak.0, lon)),
                    (Lang::Ml, false) => format!("{}-ഓടെ ഏറ്റവും കഠിനം.", month_year(lang, peak.0, lon)),
                    (Lang::Hi, true) => format!("{} के आसपास सबसे मज़बूत।", month_year(lang, peak.0, lon)),
                    (Lang::Hi, false) => format!("{} के आसपास सबसे कठिन।", month_year(lang, peak.0, lon)),
                };
                let detail = format!("{} {} {}", life_name(lang, life), strength_words(lang, strength, opening), peak_words);
                out.push(FutureEvent {
                    from,
                    to,
                    life: Some(life),
                    kind: if opening { "opening" } else { "pressure" },
                    headline,
                    detail: detail.replacen(&format!("{} ", life_name(lang, life)), "", 1),
                    strength,
                    peak: Some(peak.0),
                });
            }
        }
    }

    // 2. The turns of the long chapters of life (the first two period levels).
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 2);
    for (lvl, p) in levels.iter().enumerate() {
        if p.end > end {
            continue;
        }
        let after = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, p.end.add_ns(NS_PER_DAY), 2);
        let Some(next) = after.get(lvl) else { continue };
        let theme = chapter_theme(lang, next.lord);
        let headline = match (lang, lvl) {
            (Lang::En, 0) => format!("From {}: a new long chapter of your life begins, about {theme}.", month_year(lang, p.end, lon)),
            (Lang::En, _) => format!("From {}: a new stretch of your life begins, about {theme}.", month_year(lang, p.end, lon)),
            (Lang::Ml, 0) => format!("{} മുതൽ: ജീവിതത്തിലെ ഒരു പുതിയ വലിയ അധ്യായം തുടങ്ങുന്നു, {theme} എന്നിവയെക്കുറിച്ച്.", month_year(lang, p.end, lon)),
            (Lang::Ml, _) => format!("{} മുതൽ: ജീവിതത്തിലെ ഒരു പുതിയ ഘട്ടം തുടങ്ങുന്നു, {theme} എന്നിവയെക്കുറിച്ച്.", month_year(lang, p.end, lon)),
            (Lang::Hi, 0) => format!("{} से: ज़िंदगी का एक नया लंबा अध्याय शुरू होता है, {theme} के बारे में।", month_year(lang, p.end, lon)),
            (Lang::Hi, _) => format!("{} से: ज़िंदगी का एक नया दौर शुरू होता है, {theme} के बारे में।", month_year(lang, p.end, lon)),
        };
        out.push(FutureEvent {
            from: p.end,
            to: next.end,
            life: None,
            kind: "chapter",
            headline,
            detail: chapter_prediction(lang, next.lord).to_string(),
            strength: if lvl == 0 { 1.0 } else { 0.6 },
            peak: None,
        });
    }

    // 3. The long testing periods: when one begins and when it ends for good.
    let house_of = |sign: u8| ((sign as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
    let kind_of = |p: SaturnPhase| match p {
        SaturnPhase::SadeSatiRising | SaturnPhase::SadeSatiPeak | SaturnPhase::SadeSatiSetting => 1,
        SaturnPhase::DhaiyaFourth => 2,
        SaturnPhase::DhaiyaEighth => 3,
        SaturnPhase::Clear => 0,
    };
    let changes = sign_changes(spk, ayanamsa, DashaLord::Saturn, at.add_ns(-400 * NS_PER_DAY), end.add_ns(600 * NS_PER_DAY));
    for (j, c) in changes.iter().enumerate() {
        if c.at < at || c.at > end {
            continue;
        }
        let before = SaturnPhase::from_house(house_of(c.from));
        let after = SaturnPhase::from_house(house_of(c.to));
        if kind_of(before) == kind_of(after) {
            continue;
        }
        // A step out that comes back within a year and a half is a pause, not an end.
        let returns = changes[j + 1..].iter().any(|d| d.at.ns_since(c.at) < 548 * NS_PER_DAY && kind_of(SaturnPhase::from_house(house_of(d.to))) == kind_of(before));
        // Coming back after such a pause is a return, not a beginning.
        let resumed = kind_of(after) != 0 && changes[..j].iter().any(|d| c.at.ns_since(d.at) < 548 * NS_PER_DAY && kind_of(SaturnPhase::from_house(house_of(d.from))) == kind_of(after));
        if kind_of(before) != 0 && kind_of(after) == 0 && returns {
            let back = changes[j + 1..].iter().find(|d| kind_of(SaturnPhase::from_house(house_of(d.to))) == kind_of(before)).map(|d| d.at).unwrap_or(c.at);
            let h = match lang {
                Lang::En => format!("{}: {} lifts for a while, until {}.", month_year(lang, c.at, lon), crate::plain::cap(long_test(lang, before)), month_year(lang, back, lon)),
                Lang::Ml => format!("{}: {} {} വരെ കുറച്ചുകാലം അയയുന്നു.", month_year(lang, c.at, lon), long_test(lang, before), month_year(lang, back, lon)),
                Lang::Hi => format!("{}: {} {} तक कुछ समय के लिए हल्का होता है।", month_year(lang, c.at, lon), long_test(lang, before), month_year(lang, back, lon)),
            };
            out.push(FutureEvent {
                from: c.at,
                to: back,
                life: None,
                kind: "test_pause",
                headline: h,
                detail: lang.pick(["A breathing space: use it to finish what the pressure held up. It comes back once more before it ends.", "ഒരു ഇടവേള: സമ്മർദ്ദം തടഞ്ഞുവെച്ചത് ഈ സമയത്ത് തീർക്കുക. അവസാനിക്കും മുമ്പ് ഇത് ഒരിക്കൽ കൂടി വരും.", "साँस लेने का समय: दबाव में जो रुका था उसे अभी पूरा करें। खत्म होने से पहले यह एक बार फिर लौटेगा।"]).to_string(),
                strength: 0.6,
                peak: None,
            });
            continue;
        }
        if resumed {
            let h = match lang {
                Lang::En => format!("{}: {} returns for its last stretch.", month_year(lang, c.at, lon), crate::plain::cap(long_test(lang, after))),
                Lang::Ml => format!("{}: {} അവസാന ഘട്ടത്തിനായി തിരിച്ചുവരുന്നു.", month_year(lang, c.at, lon), long_test(lang, after)),
                Lang::Hi => format!("{}: {} अपने आखिरी दौर के लिए लौटता है।", month_year(lang, c.at, lon), long_test(lang, after)),
            };
            out.push(FutureEvent {
                from: c.at,
                to: c.at,
                life: None,
                kind: "test_returns",
                headline: h,
                detail: long_test_prediction(lang, after).to_string(),
                strength: 0.7,
                peak: None,
            });
            continue;
        }
        if kind_of(before) != 0 && kind_of(after) == 0 && !returns {
            let h = match lang {
                Lang::En => format!("{}: {} ends for good.", month_year(lang, c.at, lon), crate::plain::cap(long_test(lang, before))),
                Lang::Ml => format!("{}: {} പൂർണ്ണമായി അവസാനിക്കുന്നു.", month_year(lang, c.at, lon), long_test(lang, before)),
                Lang::Hi => format!("{}: {} पूरी तरह खत्म होता है।", month_year(lang, c.at, lon), long_test(lang, before)),
            };
            out.push(FutureEvent {
                from: c.at,
                to: c.at,
                life: None,
                kind: "test_ends",
                headline: h,
                detail: lang.pick(["What it held back starts to move again; effort pays at its normal rate.", "അത് തടഞ്ഞുവെച്ചത് വീണ്ടും നീങ്ങിത്തുടങ്ങും; അധ്വാനത്തിന് സാധാരണ ഫലം കിട്ടും.", "जो रुका था वह फिर चलने लगेगा; मेहनत का पूरा फल मिलेगा।"]).to_string(),
                strength: 0.9,
                peak: None,
            });
        } else if kind_of(after) != 0 && kind_of(before) != kind_of(after) {
            let h = match lang {
                Lang::En => format!("{}: {} begins.", month_year(lang, c.at, lon), crate::plain::cap(long_test(lang, after))),
                Lang::Ml => format!("{}: {} തുടങ്ങുന്നു.", month_year(lang, c.at, lon), long_test(lang, after)),
                Lang::Hi => format!("{}: {} शुरू होता है।", month_year(lang, c.at, lon), long_test(lang, after)),
            };
            out.push(FutureEvent {
                from: c.at,
                to: c.at,
                life: None,
                kind: "test_begins",
                headline: h,
                detail: long_test_prediction(lang, after).to_string(),
                strength: 0.9,
                peak: None,
            });
        }
    }

    out.sort_by_key(|e| e.from);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_houses::HouseSystem;
    use kshana_time::Utc;

    #[test]
    fn the_future_is_dated_plain_and_only_for_the_chosen_lives() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 18, 10, 0, 0, 0).unwrap().to_instant().unwrap();
        let focus = [Life::Love, Life::Career, Life::Money, Life::Health, Life::Family];
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            let ev = future(&spk, None, &natal, &obs, Ayanamsa::Lahiri, at, 36, &focus, lang).unwrap();
            assert!(ev.len() >= 4, "{lang:?}: {}", ev.len());
            assert!(ev.windows(2).all(|w| w[0].from <= w[1].from));
            for e in &ev {
                if lang == Lang::En {
                    println!("{} | {}", e.headline, e.detail);
                }
                assert!(e.life.map_or(true, |l| focus.contains(&l)), "{:?}", e.life);
                assert_eq!(crate::plain::jargon_in(lang, &e.headline), None, "{}", e.headline);
                assert_eq!(crate::plain::jargon_in(lang, &e.detail), None, "{}", e.detail);
            }
            assert!(ev.iter().any(|e| e.kind == "opening") && ev.iter().any(|e| e.kind == "pressure"));
        }
    }
}
