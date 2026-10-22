//! The five dasha systems over the FFI: Vimshottari, Yogini, Ashtottari, Kalachakra and
//! Chara, each laid out from the natal chart with its level-1 periods, the period
//! running at an instant with that period's sub-periods, and its conventions in words.

use kshana_chart::Chart;
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::dashas::{
    ashtottari, ashtottari_applies, ashtottari_sub, chara, chara_direction, chara_sub,
    graha_from_name, graha_name, kalachakra_detail, kalachakra_sub, sign_index, yogini, yogini_sub,
    Period,
};
use kshana_rules::Lang;
use kshana_vedic::{sign_lord, subdivide, vimshottari, DashaLord, DashaPeriod};

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// The systems, in the order [`dasha_system_names`] and `dasha_systems_at` use.
const SYSTEMS: [&str; 5] = ["vimshottari", "yogini", "ashtottari", "kalachakra", "chara"];

/// One period of any dasha system at any level.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PeriodRecord {
    /// Graha, yogini or sign name.
    pub label: String,
    pub start: ExactInstant,
    pub end: ExactInstant,
    /// 1 = mahadasha, 2 = antardasha.
    pub level: u8,
}

/// One dasha system laid out from a birth.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DashaSystemRecord {
    /// "vimshottari", "yogini", "ashtottari", "kalachakra" or "chara".
    pub system: String,
    /// Whether the system applies to this chart: the BPHS rule for Ashtottari (Rahu in a
    /// kendra or trikona from the lagna lord's sign and not in the lagna); always true
    /// for the others.
    pub applies: bool,
    /// The level-1 periods (mahadashas) from the running one at birth.
    pub periods: Vec<PeriodRecord>,
    /// The level-1 period containing the asked instant followed by all its level-2
    /// sub-periods; empty when the instant falls outside the laid-out periods.
    pub running: Vec<PeriodRecord>,
    /// The system's conventions, one per line.
    pub notes: Vec<String>,
}

/// The yoginis in Malayalam and Hindi, `(English, ml, hi)`.
const YOGINI_WORDS: [(&str, &str, &str); 8] = [
    ("Mangala", "മംഗള", "मंगला"),
    ("Pingala", "പിംഗള", "पिंगला"),
    ("Dhanya", "ധാന്യ", "धान्या"),
    ("Bhramari", "ഭ്രാമരി", "भ्रामरी"),
    ("Bhadrika", "ഭദ്രിക", "भद्रिका"),
    ("Ulka", "ഉൽക്ക", "उल्का"),
    ("Siddha", "സിദ്ധ", "सिद्धा"),
    ("Sankata", "സങ്കട", "संकटा"),
];

/// A period label (graha, yogini or sign) in the reader's language. The periods keep their
/// English labels inside, since the sub-periods are found from them; only the record is
/// translated.
pub(crate) fn label_in(label: &str, lang: Lang) -> String {
    if let Some(w) = YOGINI_WORDS.iter().find(|w| w.0 == label) {
        return lang.pick([w.0, w.1, w.2]).to_string();
    }
    kshana_speak::lexicon::name(label, lang).to_string()
}

fn rec(p: &Period, lang: Lang) -> PeriodRecord {
    PeriodRecord {
        label: label_in(&p.label, lang),
        start: p.start.into(),
        end: p.end.into(),
        level: p.level,
    }
}

fn vim_period(p: DashaPeriod, level: u8) -> Period {
    Period {
        label: graha_name(p.lord).to_string(),
        start: p.start,
        end: p.end,
        level,
    }
}

/// Sign index (0 = Aries) of a sidereal longitude.
fn sign_of(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

/// Sidereal longitude of a graha in the natal chart: Rahu is the true node, Ketu is
/// opposite it.
fn graha_sidereal(natal: &Chart, g: DashaLord) -> Result<f64, KshanaError> {
    match g {
        DashaLord::Rahu => Ok(natal.rahu().1),
        DashaLord::Ketu => Ok((natal.rahu().1 + 180.0).rem_euclid(360.0)),
        other => {
            let name = graha_name(other);
            natal
                .bodies
                .iter()
                .find(|b| b.name == name)
                .map(|b| b.sidereal)
                .ok_or_else(|| KshanaError::Engine(format!("chart has no {name}")))
        }
    }
}

/// The nine grahas in Chara's input order: Sun, Moon, Mars, Mercury, Jupiter, Venus,
/// Saturn, Rahu, Ketu.
fn chara_grahas(natal: &Chart) -> Result<[f64; 9], KshanaError> {
    let order = [
        DashaLord::Sun,
        DashaLord::Moon,
        DashaLord::Mars,
        DashaLord::Mercury,
        DashaLord::Jupiter,
        DashaLord::Venus,
        DashaLord::Saturn,
        DashaLord::Rahu,
        DashaLord::Ketu,
    ];
    let mut out = [0.0; 9];
    for (slot, g) in out.iter_mut().zip(order) {
        *slot = graha_sidereal(natal, g)?;
    }
    Ok(out)
}

const YEAR_NOTE: [&str; 3] = [
    "Years of 365.25 days; the balance at birth is rounded to a nanosecond once and every later boundary is exact.",
    "വർഷം 365.25 ദിവസം; ജനനസമയത്തെ ബാക്കി ഒരിക്കൽ മാത്രം നാനോസെക്കൻഡിലേക്ക് ഉരുട്ടുന്നു, പിന്നീടുള്ള എല്ലാ അതിരുകളും കൃത്യം.",
    "वर्ष 365.25 दिन का; जन्म के समय का शेष एक ही बार नैनोसेकंड तक पूर्णांकित होता है, बाद की हर सीमा सटीक है।",
];

/// Sub-periods of level-1 period number `k` (its index in the laid-out list).
type SubFn<'a> = Box<dyn Fn(usize, &Period) -> Vec<Period> + 'a>;

fn system_record(
    natal: &Chart,
    name: &str,
    at: Instant,
    lang: Lang,
) -> Result<DashaSystemRecord, KshanaError> {
    let birth = natal.instant;
    let moon = graha_sidereal(natal, DashaLord::Moon)?;
    let lagna_sidereal = natal.cusps_sidereal[0];
    let year_ns = YEAR_365_25_NS;
    let mut applies = true;
    let periods: Vec<Period>;
    let sub: SubFn<'_>;
    let notes: Vec<[&str; 3]>;
    match name {
        "vimshottari" => {
            periods = vimshottari(moon, birth, year_ns)
                .into_iter()
                .map(|p| vim_period(p, 1))
                .collect();
            sub = Box::new(|_, p: &Period| {
                let Some(lord) = graha_from_name(&p.label) else {
                    return Vec::new();
                };
                subdivide(DashaPeriod {
                    lord,
                    start: p.start,
                    end: p.end,
                })
                .into_iter()
                .map(|q| vim_period(q, 2))
                .collect()
            });
            notes = vec![
                [
                    "Vimshottari: Ketu 7, Venus 20, Sun 6, Moon 10, Mars 7, Rahu 18, Jupiter 16, Saturn 19, Mercury 17 (120 years), starting from the lord of the Moon's nakshatra.",
                    "വിംശോത്തരി: കേതു 7, ശുക്രൻ 20, സൂര്യൻ 6, ചന്ദ്രൻ 10, ചൊവ്വ 7, രാഹു 18, വ്യാഴം 16, ശനി 19, ബുധൻ 17 (120 വർഷം), ചന്ദ്രന്റെ നക്ഷത്രാധിപനിൽ നിന്ന് തുടങ്ങുന്നു.",
                    "विंशोत्तरी: केतु 7, शुक्र 20, सूर्य 6, चंद्र 10, मंगल 7, राहु 18, गुरु 16, शनि 19, बुध 17 (120 वर्ष), चंद्र के नक्षत्र के स्वामी से आरंभ।",
                ],
                [
                    "Balance at birth: the untraversed fraction of the Moon's nakshatra times the lord's years.",
                    "ജനനസമയത്തെ ദശാശിഷ്ടം: ചന്ദ്രൻ നക്ഷത്രത്തിൽ ഇനിയും കടക്കാനുള്ള ഭാഗം × അധിപന്റെ വർഷങ്ങൾ.",
                    "जन्म के समय दशा शेष: चंद्र के नक्षत्र का बचा हुआ भाग × स्वामी के वर्ष।",
                ],
                [
                    "Antardashas: the nine lords in cycle order from the period's own lord, proportioned by their years over 120.",
                    "അപഹാരങ്ങൾ: ദശാനാഥനിൽ നിന്ന് ചക്രക്രമത്തിൽ ഒമ്പത് ഗ്രഹങ്ങൾ, അവരുടെ വർഷങ്ങൾ / 120 എന്ന അനുപാതത്തിൽ.",
                    "अंतर्दशाएँ: दशा के अपने स्वामी से चक्र क्रम में नौ ग्रह, उनके वर्ष / 120 के अनुपात में।",
                ],
                YEAR_NOTE,
            ];
        }
        "yogini" => {
            periods = yogini(moon, birth, year_ns);
            sub = Box::new(|_, p: &Period| yogini_sub(p));
            notes = vec![
                [
                    "Yogini: Mangala 1 (Moon), Pingala 2 (Sun), Dhanya 3 (Jupiter), Bhramari 4 (Mars), Bhadrika 5 (Mercury), Ulka 6 (Saturn), Siddha 7 (Venus), Sankata 8 (Rahu); 36 years.",
                    "യോഗിനി: മംഗള 1 (ചന്ദ്രൻ), പിംഗള 2 (സൂര്യൻ), ധാന്യ 3 (വ്യാഴം), ഭ്രാമരി 4 (ചൊവ്വ), ഭദ്രിക 5 (ബുധൻ), ഉൽക്ക 6 (ശനി), സിദ്ധ 7 (ശുക്രൻ), സങ്കട 8 (രാഹു); 36 വർഷം.",
                    "योगिनी: मंगला 1 (चंद्र), पिंगला 2 (सूर्य), धान्या 3 (गुरु), भ्रामरी 4 (मंगल), भद्रिका 5 (बुध), उल्का 6 (शनि), सिद्धा 7 (शुक्र), संकटा 8 (राहु); 36 वर्ष।",
                ],
                [
                    "Starting yogini (BPHS): (nakshatra number + 3) mod 8, 0 = Sankata; balance from the untraversed fraction of the Moon's nakshatra.",
                    "ആദ്യ യോഗിനി (BPHS): (നക്ഷത്ര സംഖ്യ + 3) mod 8, 0 = സങ്കട; ബാക്കി ചന്ദ്രൻ നക്ഷത്രത്തിൽ ഇനിയും കടക്കാനുള്ള ഭാഗത്തിൽ നിന്ന്.",
                    "पहली योगिनी (BPHS): (नक्षत्र संख्या + 3) mod 8, 0 = संकटा; शेष चंद्र के नक्षत्र के बचे भाग से।",
                ],
                [
                    "Antardashas: the eight yoginis in cycle order from the period's own, proportioned by their years over 36 (BPHS; not PyJHora's equal eighths).",
                    "അന്തർദശകൾ: ദശയുടെ സ്വന്തം യോഗിനിയിൽ നിന്ന് ചക്രക്രമത്തിൽ എട്ട് യോഗിനികൾ, വർഷങ്ങൾ / 36 എന്ന അനുപാതത്തിൽ (BPHS; പൈജെഹോറയുടെ തുല്യ എട്ടിലൊന്നുകളല്ല).",
                    "अंतर्दशाएँ: दशा की अपनी योगिनी से चक्र क्रम में आठ योगिनियाँ, वर्ष / 36 के अनुपात में (BPHS; पाइजेहोरा के बराबर आठवें भाग नहीं)।",
                ],
                YEAR_NOTE,
            ];
        }
        "ashtottari" => {
            periods = ashtottari(moon, birth, year_ns);
            sub = Box::new(|_, p: &Period| ashtottari_sub(p));
            let lagna = sign_of(lagna_sidereal);
            let lord_sign = sign_of(graha_sidereal(natal, sign_lord(lagna))?);
            applies = ashtottari_applies(lagna, lord_sign, sign_of(natal.rahu().1));
            notes = vec![
                [
                    "Ashtottari: Sun 6, Moon 15, Mars 8, Mercury 17, Saturn 10, Jupiter 19, Rahu 12, Venus 21 (108 years); Ketu has no period.",
                    "അഷ്ടോത്തരി: സൂര്യൻ 6, ചന്ദ്രൻ 15, ചൊവ്വ 8, ബുധൻ 17, ശനി 10, വ്യാഴം 19, രാഹു 12, ശുക്രൻ 21 (108 വർഷം); കേതുവിന് ദശയില്ല.",
                    "अष्टोत्तरी: सूर्य 6, चंद्र 15, मंगल 8, बुध 17, शनि 10, गुरु 19, राहु 12, शुक्र 21 (108 वर्ष); केतु की कोई दशा नहीं।",
                ],
                [
                    "Nakshatra groups of 4 and 3 from Ardra (27-nakshatra scheme); balance from the Moon's place within its lord's whole group.",
                    "തിരുവാതിര മുതൽ 4, 3 നക്ഷത്രങ്ങൾ വീതമുള്ള ഗണങ്ങൾ (27 നക്ഷത്ര രീതി); ബാക്കി അധിപന്റെ മുഴുവൻ ഗണത്തിനുള്ളിലെ ചന്ദ്രസ്ഥാനത്തിൽ നിന്ന്.",
                    "आर्द्रा से 4 और 3 नक्षत्रों के समूह (27 नक्षत्र पद्धति); शेष स्वामी के पूरे समूह में चंद्र की स्थिति से।",
                ],
                [
                    "Antardashas: the eight lords in cycle order from the period's own lord, proportioned by their years over 108 (JHora).",
                    "അന്തർദശകൾ: ദശാനാഥനിൽ നിന്ന് ചക്രക്രമത്തിൽ എട്ട് അധിപന്മാർ, വർഷങ്ങൾ / 108 എന്ന അനുപാതത്തിൽ (ജെഹോറ).",
                    "अंतर्दशाएँ: दशा के अपने स्वामी से चक्र क्रम में आठ स्वामी, वर्ष / 108 के अनुपात में (जेहोरा)।",
                ],
                [
                    "Applies (BPHS 47.1-2) when Rahu is in a kendra or trikona from the sign of the lagna lord and not in the lagna; Rahu is the true node.",
                    "ബാധകം (BPHS 47.1-2): ലഗ്നാധിപന്റെ രാശിയിൽ നിന്ന് കേന്ദ്രത്തിലോ ത്രികോണത്തിലോ രാഹു, ലഗ്നത്തിലല്ലാതെ; രാഹു സ്പഷ്ട പാതം.",
                    "लागू (BPHS 47.1-2) जब राहु लग्नेश की राशि से केंद्र या त्रिकोण में हो और लग्न में न हो; राहु स्पष्ट पात है।",
                ],
                YEAR_NOTE,
            ];
        }
        "kalachakra" => {
            let d = kalachakra_detail(moon, birth, year_ns);
            let (row, first) = (d.row, d.first_position);
            periods = d.periods;
            sub = Box::new(move |k, p: &Period| kalachakra_sub(p, &row, (first + k) % 9));
            notes = vec![
                [
                    "Kalachakra: nine-sign rows per nakshatra pada (JHora tables, Revati savya as in BPHS); paramayus 100/85/83/86 savya, 86/83/85/100 apasavya.",
                    "കാലചക്രം: ഓരോ നക്ഷത്രപാദത്തിനും ഒമ്പത് രാശികളുടെ നിര (ജെഹോറ പട്ടികകൾ, BPHS പോലെ രേവതി സവ്യം); പരമായുസ്സ് സവ്യത്തിൽ 100/85/83/86, അപസവ്യത്തിൽ 86/83/85/100.",
                    "कालचक्र: हर नक्षत्र पाद के लिए नौ राशियों की पंक्ति (जेहोरा तालिकाएँ, BPHS की तरह रेवती सव्य); परमायु सव्य में 100/85/83/86, अपसव्य में 86/83/85/100।",
                ],
                [
                    "Sign years (BPHS 46): Aries 7, Taurus 16, Gemini 9, Cancer 21, Leo 5, Virgo 9, Libra 16, Scorpio 7, Sagittarius 10, Capricorn 4, Aquarius 4, Pisces 10.",
                    "രാശി വർഷങ്ങൾ (BPHS 46): മേടം 7, ഇടവം 16, മിഥുനം 9, കർക്കടകം 21, ചിങ്ങം 5, കന്നി 9, തുലാം 16, വൃശ്ചികം 7, ധനു 10, മകരം 4, കുംഭം 4, മീനം 10.",
                    "राशि वर्ष (BPHS 46): मेष 7, वृषभ 16, मिथुन 9, कर्क 21, सिंह 5, कन्या 9, तुला 16, वृश्चिक 7, धनु 10, मकर 4, कुंभ 4, मीन 10।",
                ],
                [
                    "Balance (BPHS 46.93): the fraction of the Moon's pada traversed times the paramayus; the natal row repeats after its ninth sign.",
                    "ബാക്കി (BPHS 46.93): ചന്ദ്രൻ പാദത്തിൽ കടന്ന ഭാഗം × പരമായുസ്സ്; ജന്മനിര ഒമ്പതാം രാശിക്കു ശേഷം ആവർത്തിക്കുന്നു.",
                    "शेष (BPHS 46.93): चंद्र द्वारा पाद का पार किया भाग × परमायु; जन्म पंक्ति नौवीं राशि के बाद दोहराती है।",
                ],
                [
                    "Antardashas: the natal row rotated to start from the period's sign, proportioned by sign years over the paramayus.",
                    "അന്തർദശകൾ: ദശയുടെ രാശിയിൽ നിന്ന് തുടങ്ങുംവിധം തിരിച്ച ജന്മനിര, രാശിവർഷങ്ങൾ / പരമായുസ്സ് എന്ന അനുപാതത്തിൽ.",
                    "अंतर्दशाएँ: दशा की राशि से शुरू होने के लिए घुमाई गई जन्म पंक्ति, राशि वर्ष / परमायु के अनुपात में।",
                ],
                YEAR_NOTE,
            ];
        }
        "chara" => {
            let grahas = chara_grahas(natal)?;
            periods = chara(lagna_sidereal, &grahas, birth, year_ns);
            sub = Box::new(|_, p: &Period| {
                sign_index(&p.label)
                    .map(|s| chara_sub(p, chara_direction(s)))
                    .unwrap_or_default()
            });
            notes = vec![
                [
                    "Chara (Jaimini, K. N. Rao): twelve signs from the lagna, direct when the ninth sign from the lagna is odd-footed, reverse otherwise.",
                    "ചരദശ (ജൈമിനി, കെ. എൻ. റാവു): ലഗ്നം മുതൽ പന്ത്രണ്ട് രാശികൾ; ലഗ്നത്തിൽ നിന്ന് ഒമ്പതാം രാശി ഓജപാദമെങ്കിൽ നേർക്രമം, അല്ലെങ്കിൽ വിപരീതം.",
                    "चर दशा (जैमिनी, के. एन. राव): लग्न से बारह राशियाँ; लग्न से नौवीं राशि विषम-पाद हो तो सीधा क्रम, नहीं तो उलटा।",
                ],
                [
                    "Years: count from the sign to its lord's sign (direct for odd-footed signs, reverse for even-footed) minus one; lord in its own sign 12; exalted +1, debilitated -1; Scorpio and Aquarius by their co-lords.",
                    "വർഷങ്ങൾ: രാശിയിൽ നിന്ന് അധിപന്റെ രാശി വരെ എണ്ണി (ഓജപാദ രാശിക്ക് നേരെ, യുഗ്മപാദത്തിന് വിപരീതം) ഒന്ന് കുറയ്ക്കുക; അധിപൻ സ്വരാശിയിലെങ്കിൽ 12; ഉച്ചത്തിൽ +1, നീചത്തിൽ -1; വൃശ്ചികവും കുംഭവും സഹാധിപന്മാരെ കൊണ്ട്.",
                    "वर्ष: राशि से उसके स्वामी की राशि तक गिनें (विषम-पाद राशि के लिए सीधे, सम-पाद के लिए उलटे) और एक घटाएँ; स्वामी अपनी राशि में हो तो 12; उच्च में +1, नीच में -1; वृश्चिक और कुंभ सह-स्वामियों से।",
                ],
                [
                    "Rahu is the true node and Ketu its opposite; all positions sidereal by the chosen ayanamsa.",
                    "രാഹു സ്പഷ്ട പാതം, കേതു അതിന് നേരെ എതിർവശം; എല്ലാ സ്ഥാനങ്ങളും തിരഞ്ഞെടുത്ത അയനാംശം കൊണ്ടുള്ള നിരയനം.",
                    "राहु स्पष्ट पात है और केतु उसके ठीक सामने; सभी स्थितियाँ चुने गए अयनांश से निरयन।",
                ],
                [
                    "Antardashas: twelve equal, from the sign after the period's sign in that sign's own direction, ending on the sign itself.",
                    "അന്തർദശകൾ: പന്ത്രണ്ട് തുല്യം, ദശാരാശിക്ക് അടുത്ത രാശിയിൽ നിന്ന് ആ രാശിയുടെ സ്വന്തം ദിശയിൽ, ദശാരാശിയിൽ തന്നെ അവസാനിക്കുന്നു.",
                    "अंतर्दशाएँ: बारह बराबर, दशा राशि के बाद की राशि से उसी राशि की अपनी दिशा में, दशा राशि पर ही समाप्त।",
                ],
                YEAR_NOTE,
            ];
        }
        other => {
            return Err(KshanaError::Engine(format!(
                "unknown dasha system {other:?}; one of {}",
                SYSTEMS.join(", ")
            )))
        }
    }
    let mut running = Vec::new();
    if let Some(k) = periods.iter().position(|p| p.start <= at && at < p.end) {
        running.push(rec(&periods[k], lang));
        running.extend(sub(k, &periods[k]).iter().map(|p| rec(p, lang)));
    }
    Ok(DashaSystemRecord {
        system: name.to_string(),
        applies,
        periods: periods.iter().map(|p| rec(p, lang)).collect(),
        running,
        notes: notes.iter().map(|s| lang.pick(*s).to_string()).collect(),
    })
}

#[uniffi::export]
impl Engine {
    /// One dasha system ("vimshottari", "yogini", "ashtottari", "kalachakra" or "chara")
    /// laid out from the natal chart, with the period running at `at` and its
    /// sub-periods.
    pub fn dasha_system(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        system: String,
        at: ExactInstant,
        lang: String,
    ) -> Result<DashaSystemRecord, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        system_record(
            &natal,
            &system.to_ascii_lowercase(),
            at.into(),
            Lang::parse(&lang),
        )
    }

    /// All five dasha systems at `at`, in the order of [`dasha_system_names`].
    pub fn dasha_systems_at(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        lang: String,
    ) -> Result<Vec<DashaSystemRecord>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let lang = Lang::parse(&lang);
        SYSTEMS
            .iter()
            .map(|s| system_record(&natal, s, at.into(), lang))
            .collect()
    }
}

/// Names of the dasha systems `Engine::dasha_system` accepts, in the order
/// `Engine::dasha_systems_at` returns them.
#[uniffi::export]
pub fn dasha_system_names() -> Vec<String> {
    SYSTEMS.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    fn key(i: ExactInstant) -> (i64, u32) {
        (i.tai_seconds, i.nanos)
    }

    fn u(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> ExactInstant {
        instant_from_utc(UtcFields {
            year: y,
            month: mo,
            day: d,
            hour: h,
            minute: mi,
            second: 0,
            nanos: 0,
        })
        .unwrap()
    }

    #[test]
    fn five_systems_cross_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let birth = u(1990, 10, 29, 12, 6);
        let (lat, lon) = (9.9312, 76.2673);
        let ay = AyanamsaChoice::Lahiri;
        let at = u(2026, 10, 16, 0, 0);

        let names = dasha_system_names();
        assert_eq!(names.len(), 5);
        let all = engine.dasha_systems_at(birth, lat, lon, ay, at, "en".into()).unwrap();
        assert_eq!(all.len(), 5);
        for (s, name) in all.iter().zip(&names) {
            assert_eq!(&s.system, name);
            assert!(!s.periods.is_empty(), "{name}");
            assert!(s
                .periods
                .iter()
                .all(|p| p.level == 1 && !p.label.is_empty()));
            assert!(s
                .periods
                .windows(2)
                .all(|w| key(w[0].end) == key(w[1].start)));
            assert!(key(s.periods[0].start) <= key(birth));
            assert!(s.running.len() > 1, "{name}");
            assert_eq!(s.running[0].level, 1);
            assert!(key(s.running[0].start) <= key(at) && key(at) < key(s.running[0].end));
            assert!(s.running[1..].iter().all(|p| p.level == 2));
            assert_eq!(key(s.running[1].start), key(s.running[0].start));
            assert_eq!(key(s.running.last().unwrap().end), key(s.running[0].end));
            assert!(s.notes.len() >= 4);
        }
        // one cycle of nine, eight, eight, nine and twelve
        assert_eq!(all[0].periods.len(), 9);
        assert_eq!(all[0].running.len(), 10);
        assert_eq!(all[1].running.len(), 9);
        assert_eq!(all[2].running.len(), 9);
        assert_eq!(all[3].running.len(), 10);
        assert_eq!(all[4].periods.len(), 12);
        assert_eq!(all[4].running.len(), 13);
        assert!(all.iter().enumerate().all(|(i, s)| i == 2 || s.applies));
        // the fixture Moon in Shatabhisha: Rahu, Jupiter, then Saturn running in 2026
        assert_eq!(all[0].periods[0].label, "Rahu");
        assert_eq!(all[0].running[0].label, "Saturn");

        let one = engine
            .dasha_system(birth, lat, lon, ay, "Vimshottari".into(), at, "en".into())
            .unwrap();
        assert_eq!(one.system, "vimshottari");
        assert_eq!(one.running[0].label, all[0].running[0].label);
        assert!(engine
            .dasha_system(birth, lat, lon, ay, "narayana".into(), at, "hi".into())
            .is_err());
    }
}
