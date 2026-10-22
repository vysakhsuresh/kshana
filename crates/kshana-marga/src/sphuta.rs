//! The Prasna Marga sensitive points (sphutas) and the readings the text attaches to
//! them.
//!
//! # Sources
//!
//! * Prasna Marga ch. VIII (B. V. Raman's translation, Part I, chs. I–XVI, Motilal
//!   Banarsidass), stanzas 1–5 for Trisphuta, Chatussphuta and Panchasphuta, 6–10 for
//!   the Prana, Deha and Mrityu sphutas, 15 for the navamsa ("multiply by nine")
//!   sphutas, 26–30 for the Trisphuta's sign and nakshatra readings. The stanza numbers
//!   are those of Raman's edition as the blog transcriptions of it number them
//!   (vedanga-jyotish.blogspot.com "Prasna II"; varahamihira.blogspot.com "Praśna II:
//!   Use of Trisphuta in praśna").
//! * Prasna Marga Part II, the chapter on progeny, for the Beeja and Kshetra sphutas
//!   (the longitude sums as every modern reading of the text gives them:
//!   eastrovedica.com lesson 52; planetarypositions.com "Beeja Sphuta, Kshetra
//!   Sphuta"); the text's own procedure (stanza 5, the expired portions of the planets'
//!   asterisms in ghatis) is older and not implemented.
//! * The Yogi point: Sun + Moon + 93°20′ (the start of Pushya), its nakshatra lord the
//!   Yogi, the sign lord the Sahayogi, the lord of the nakshatra 186°40′ on the Avayogi
//!   (hinducalculator.com "Yogi-Avayogi"; astrosaxena.in "Concept of Yogi, Avayogi";
//!   the convention Jagannatha Hora, Kala and Parashara's Light print).
//! * The Bhrigu Bindu: the midpoint of the Moon and Rahu (Bhrigu Nandi Nadi practice;
//!   blog.cosmicinsights.net "Secrets of Destiny point"; astroindus.com "Bhrigu Bindu
//!   calculator").
//!
//! # Gulika
//!
//! Gulika is taken from `kshana_day`: the sidereal ascendant at the **start** of
//! Saturn's part of the day (sunrise to sunset) or of the night, the eight parts being
//! lorded in weekday order from the weekday lord (BPHS ch. 3, 66–67). The start of the
//! part is Jagannatha Hora's default for Gulika ([`GulikaConvention::JhoraStart`]); the
//! Kerala thirty-ghati Mandi table of the Malayalam almanacs is the alternative
//! ([`GulikaConvention::KeralaMandi`]).

use kshana_chart::Chart;
use kshana_day::{
    ascendant_sidereal, mandi_instant_kerala, upagraha_longitude, KalaPoint, Upagraha,
};
use kshana_ephem::Spk;
use kshana_events::solar_day;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{kp_lords, nakshatra, sign_lord, varga, Ayanamsa, DashaLord};

use crate::{body_sidereal, graha_name, norm360, sign_of, MargaError, SIGN_NAMES};
use kshana_rules::yogas::words;
use kshana_rules::{say3, Lang};

/// A sensitive point.
#[derive(Clone, Debug, PartialEq)]
pub struct Sphuta {
    /// Name.
    pub name: &'static str,
    /// Sidereal longitude, degrees in `[0, 360)`.
    pub longitude: f64,
    /// Sign index 0–11.
    pub sign: u8,
    /// Nakshatra index 0–26.
    pub nakshatra: usize,
    /// Pada 1–4.
    pub pada: u8,
    /// Lord of the sign.
    pub lord: DashaLord,
    /// Lord of the nakshatra (the Vimshottari lord).
    pub star_lord: DashaLord,
    /// The formula, the place, and the text's reading, in words.
    pub words: Vec<String>,
}

/// A graha's name in the reader's language (English: the name the text always printed).
pub(crate) fn gn(lang: Lang, g: DashaLord) -> &'static str {
    match lang {
        Lang::En => graha_name(g),
        _ => words::graha(lang, g),
    }
}

/// A sign's name in the reader's language, 0 = Aries.
pub(crate) fn sn(lang: Lang, s: u8) -> &'static str {
    match lang {
        Lang::En => SIGN_NAMES[(s % 12) as usize],
        _ => words::rasi(lang, s as usize),
    }
}

/// A nakshatra's name in the reader's language, 0 = Ashwini.
pub(crate) fn nn(lang: Lang, n: usize) -> &'static str {
    match lang {
        Lang::En => NAKSHATRA_NAMES[n % 27],
        _ => words::star(lang, n),
    }
}

/// Graha names joined with commas, or "none".
pub(crate) fn gnames(lang: Lang, v: &[DashaLord]) -> String {
    if v.is_empty() {
        lang.pick(["none", "ഇല്ല", "कोई नहीं"]).into()
    } else {
        v.iter().map(|g| gn(lang, *g)).collect::<Vec<_>>().join(", ")
    }
}

/// A sphuta's or prasna point's name in the reader's language.
pub fn sphuta_name(lang: Lang, en: &'static str) -> &'static str {
    let t = match en {
        "Trisphuta" => ["Trisphuta", "ത്രിസ്ഫുടം", "त्रिस्फुट"],
        "Chatussphuta" => ["Chatussphuta", "ചതുസ്ഫുടം", "चतुःस्फुट"],
        "Panchasphuta" => ["Panchasphuta", "പഞ്ചസ്ഫുടം", "पंचस्फुट"],
        "Pranasphuta" => ["Pranasphuta", "പ്രാണസ്ഫുടം", "प्राणस्फुट"],
        "Dehasphuta" => ["Dehasphuta", "ദേഹസ്ഫുടം", "देहस्फुट"],
        "Mrityusphuta" => ["Mrityusphuta", "ഗുളിക-സൂര്യ സ്ഫുടം", "गुलिक-सूर्य स्फुट"],
        "Sookshma Trisphuta" => ["Sookshma Trisphuta", "സൂക്ഷ്മ ത്രിസ്ഫുടം", "सूक्ष्म त्रिस्फुट"],
        "Beeja sphuta" => ["Beeja sphuta", "ബീജസ്ഫുടം", "बीज स्फुट"],
        "Kshetra sphuta" => ["Kshetra sphuta", "ക്ഷേത്രസ്ഫുടം", "क्षेत्र स्फुट"],
        "Yogi point" => ["Yogi point", "യോഗി ബിന്ദു", "योगी बिंदु"],
        "Bhrigu Bindu" => ["Bhrigu Bindu", "ഭൃഗു ബിന്ദു", "भृगु बिंदु"],
        "Lagna" => ["Lagna", "ലഗ്നം", "लग्न"],
        "Chandra" => ["Chandra", "ചന്ദ്രൻ", "चंद्र"],
        "Arudha" => ["Arudha", "ആരൂഢം", "आरूढ़"],
        _ => return en,
    };
    lang.pick(t)
}

/// A term in a formula ("Lagna", "Gulika", a graha, a sphuta) in the reader's language.
fn fw(lang: Lang, en: &'static str) -> &'static str {
    match en {
        "Gulika" => lang.pick(["Gulika", "ഗുളികൻ", "गुलिक"]),
        "Sun" => gn(lang, DashaLord::Sun),
        "Moon" => gn(lang, DashaLord::Moon),
        "Mars" => gn(lang, DashaLord::Mars),
        "Jupiter" => gn(lang, DashaLord::Jupiter),
        "Venus" => gn(lang, DashaLord::Venus),
        "Rahu" => gn(lang, DashaLord::Rahu),
        _ => sphuta_name(lang, en),
    }
}

/// "Prasna Marga" with a stanza: the Roman chapter in English, Arabic in the scripts.
fn pm(lang: Lang, roman: &str, arabic: &str) -> String {
    say3!(lang, "Prasna Marga {}", "പ്രശ്നമാർഗ്ഗം {}", "प्रश्न मार्ग {}", if lang == Lang::En { roman } else { arabic })
}

/// A sphuta from its longitude, with the header line (formula, place, lords) in the
/// reader's language; `name` is the English name and is stored translated.
pub(crate) fn make_in(
    lang: Lang,
    name: &'static str,
    longitude: f64,
    formula: String,
    mut words: Vec<String>,
) -> Sphuta {
    let name = sphuta_name(lang, name);
    let l = norm360(longitude);
    let n = nakshatra(l);
    let sign = sign_of(l);
    let star_lord = kp_lords(l).star;
    let lord = sign_lord(sign as usize);
    let (deg, sign_w, star_w, lord_w, star_lord_w) =
        (l.rem_euclid(30.0), sn(lang, sign), nn(lang, n.index), gn(lang, lord), gn(lang, star_lord));
    let pada = n.pada;
    words.insert(
        0,
        say3!(
            lang,
            "{name} = {formula} = {l:.4}°: {deg:.2}° {sign_w}, {star_w} pada {pada}, sign lord {lord_w}, star lord {star_lord_w}",
            "{name} = {formula} = {l:.4}°: {sign_w} {deg:.2}°, {star_w} {pada}-ാം പാദം, രാശ്യധിപൻ {lord_w}, നക്ഷത്രാധിപൻ {star_lord_w}",
            "{name} = {formula} = {l:.4}°: {sign_w} {deg:.2}°, {star_w} पाद {pada}, राशि स्वामी {lord_w}, नक्षत्र स्वामी {star_lord_w}"
        ),
    );
    Sphuta {
        name,
        longitude: l,
        sign,
        nakshatra: n.index,
        pada: n.pada,
        lord,
        star_lord,
        words,
    }
}

/// Events the text attaches to the Trisphuta's nakshatra, by trine group: group `i`
/// holds nakshatras `i`, `i + 9` and `i + 18` (0 = Ashwini, Magha, Mula). Prasna Marga
/// VIII lists nine events; the five transcribed here are those the available
/// translations give in full, the other four are left `None` rather than guessed.
pub const TRISPHUTA_TRINE_EVENTS: [Option<&str>; 9] = [
    Some("a causeless quarrel within three days"),
    Some("a poor old Brahmin turning up within three days"),
    Some("fear from fire, or a conflagration"),
    None,
    None,
    None,
    None,
    Some("the loss of a quadruped"),
    Some("a servant falling from a tree, a lamp put out by an insect, or sad news"),
];

/// [`TRISPHUTA_TRINE_EVENTS`] in Malayalam and Hindi.
const TRISPHUTA_TRINE_EVENTS_IN: [Option<[&str; 2]>; 9] = [
    Some(["മൂന്നു ദിവസത്തിനകം കാരണമില്ലാത്ത ഒരു കലഹം", "तीन दिनों के भीतर अकारण कलह"]),
    Some([
        "മൂന്നു ദിവസത്തിനകം ദരിദ്രനായ ഒരു വൃദ്ധബ്രാഹ്മണന്റെ വരവ്",
        "तीन दिनों के भीतर किसी निर्धन वृद्ध ब्राह्मण का आगमन",
    ]),
    Some(["അഗ്നിഭയം, അല്ലെങ്കിൽ തീപിടുത്തം", "अग्नि से भय, या आग लगना"]),
    None,
    None,
    None,
    None,
    Some(["ഒരു നാൽക്കാലിയുടെ നഷ്ടം", "किसी चौपाए की हानि"]),
    Some([
        "ഒരു ഭൃത്യൻ മരത്തിൽ നിന്നു വീഴുക, ഒരു പ്രാണി വിളക്ക് കെടുത്തുക, അല്ലെങ്കിൽ ദുഃഖവാർത്ത",
        "किसी सेवक का पेड़ से गिरना, किसी कीट से दीपक बुझना, या दुखद समाचार",
    ]),
];

fn trine_event(lang: Lang, group: usize) -> Option<&'static str> {
    match lang {
        Lang::En => TRISPHUTA_TRINE_EVENTS[group],
        Lang::Ml => TRISPHUTA_TRINE_EVENTS_IN[group].map(|t| t[0]),
        Lang::Hi => TRISPHUTA_TRINE_EVENTS_IN[group].map(|t| t[1]),
    }
}

/// The text's readings of a Trisphuta longitude (Prasna Marga VIII.26–30): the sign
/// rule (Cancer, Scorpio and Pisces bring intense trouble, most in Pisces, least in
/// Cancer), the last-navamsa timing of those three signs (a year, a month, a day), and
/// the nakshatra trine's confirming event. These are the text's words about a prasna
/// on illness; the app shows them as the text's, never as a verdict.
pub fn trisphuta_words(longitude: f64) -> Vec<String> {
    trisphuta_words_in(Lang::En, longitude)
}

/// [`trisphuta_words`] in the reader's language.
pub fn trisphuta_words_in(lang: Lang, longitude: f64) -> Vec<String> {
    let l = norm360(longitude);
    let sign = sign_of(l);
    let in_sign = l.rem_euclid(30.0);
    let last_navamsa = in_sign >= 30.0 - 30.0 / 9.0;
    let s = sn(lang, sign);
    let mut w = Vec::new();
    match sign {
        3 | 7 | 11 => {
            let (how, when) = match sign {
                3 => (
                    lang.pick(["least", "ഏറ്റവും കുറവ്", "सबसे कम"]),
                    lang.pick(["a year", "ഒരു വർഷം", "एक वर्ष"]),
                ),
                7 => (
                    lang.pick(["more", "കൂടുതൽ", "अधिक"]),
                    lang.pick(["a month", "ഒരു മാസം", "एक महीना"]),
                ),
                _ => (
                    lang.pick(["most", "ഏറ്റവും കൂടുതൽ", "सबसे अधिक"]),
                    lang.pick(["a day", "ഒരു ദിവസം", "एक दिन"]),
                ),
            };
            let p = pm(lang, "VIII.28", "8.28");
            w.push(say3!(
                lang,
                "{p}: Trisphuta in {s} is read as intense trouble ({how} of the three signs Cancer, Scorpio, Pisces)",
                "{p}: ത്രിസ്ഫുടം {s} രാശിയിൽ കഠിനമായ ക്ലേശമായി വായിക്കുന്നു (കർക്കടകം, വൃശ്ചികം, മീനം എന്നീ മൂന്നു രാശികളിൽ {how})",
                "{p}: त्रिस्फुट {s} में तीव्र कष्ट माना जाता है (कर्क, वृश्चिक, मीन इन तीन राशियों में {how})"
            ));
            if last_navamsa {
                let p = pm(lang, "VIII.29", "8.29");
                w.push(say3!(
                    lang,
                    "{p}: in the last navamsa of {s} the text's timing is {when}",
                    "{p}: {s} രാശിയുടെ അവസാന നവാംശത്തിൽ ഗ്രന്ഥം പറയുന്ന കാലം {when}",
                    "{p}: {s} के अंतिम नवांश में ग्रंथ का बताया समय {when} है"
                ));
            }
        }
        _ => {
            let p = pm(lang, "VIII.28", "8.28");
            w.push(say3!(
                lang,
                "{p} names Cancer, Scorpio and Pisces as the Trisphuta signs of intense trouble; {s} is not one of them",
                "{p} കർക്കടകം, വൃശ്ചികം, മീനം എന്നിവയെ ത്രിസ്ഫുടത്തിന്റെ കഠിനക്ലേശ രാശികളായി പറയുന്നു; {s} അവയിൽ ഒന്നല്ല",
                "{p} कर्क, वृश्चिक और मीन को त्रिस्फुट की तीव्र कष्ट वाली राशियाँ बताता है; {s} इनमें से नहीं है"
            ))
        }
    }
    let n = nakshatra(l).index;
    let group = n % 9;
    let (nk, a, b, c) = (nn(lang, n), nn(lang, group), nn(lang, group + 9), nn(lang, group + 18));
    let p = pm(lang, "VIII", "8");
    match trine_event(lang, group) {
        Some(ev) => w.push(say3!(
            lang,
            "{p}: Trisphuta in {nk} (trine of {a}, {b} and {c}): the confirming event the text names is {ev}",
            "{p}: ത്രിസ്ഫുടം {nk} നക്ഷത്രത്തിൽ ({a}, {b}, {c} എന്ന ത്രികോണം): ഗ്രന്ഥം പറയുന്ന സ്ഥിരീകരണ സംഭവം: {ev}",
            "{p}: त्रिस्फुट {nk} नक्षत्र में ({a}, {b} और {c} का त्रिकोण): ग्रंथ जो पुष्टि करने वाली घटना बताता है, वह है {ev}"
        )),
        None => w.push(say3!(
            lang,
            "{p}: Trisphuta in {nk} (trine of {a}, {b} and {c}); the text's confirming event for this trine is not transcribed in the available translations",
            "{p}: ത്രിസ്ഫുടം {nk} നക്ഷത്രത്തിൽ ({a}, {b}, {c} എന്ന ത്രികോണം); ഈ ത്രികോണത്തിന്റെ സ്ഥിരീകരണ സംഭവം ലഭ്യമായ പരിഭാഷകളിൽ രേഖപ്പെടുത്തിയിട്ടില്ല",
            "{p}: त्रिस्फुट {nk} नक्षत्र में ({a}, {b} और {c} का त्रिकोण); इस त्रिकोण की पुष्टि करने वाली घटना उपलब्ध अनुवादों में नहीं मिलती"
        )),
    }
    w
}

/// Trisphuta = Lagna + Moon + Gulika (Prasna Marga VIII.1–3).
pub fn trisphuta(lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    trisphuta_in(Lang::En, lagna, moon, gulika)
}

/// [`trisphuta`] in the reader's language.
pub fn trisphuta_in(lang: Lang, lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    let l = norm360(lagna + moon + gulika);
    let (a, b, c) = (fw(lang, "Lagna"), fw(lang, "Moon"), fw(lang, "Gulika"));
    make_in(
        lang,
        "Trisphuta",
        l,
        format!("{a} {lagna:.4} + {b} {moon:.4} + {c} {gulika:.4}"),
        trisphuta_words_in(lang, l),
    )
}

/// Chatussphuta = Trisphuta + Sun (Prasna Marga VIII.4).
pub fn chatussphuta(lagna: f64, moon: f64, gulika: f64, sun: f64) -> Sphuta {
    chatussphuta_in(Lang::En, lagna, moon, gulika, sun)
}

/// [`chatussphuta`] in the reader's language.
pub fn chatussphuta_in(lang: Lang, lagna: f64, moon: f64, gulika: f64, sun: f64) -> Sphuta {
    let p = pm(lang, "VIII.4", "8.4");
    make_in(
        lang,
        "Chatussphuta",
        lagna + moon + gulika + sun,
        format!("{} + {} {sun:.4}", fw(lang, "Trisphuta"), fw(lang, "Sun")),
        vec![say3!(
            lang,
            "{p}: the Trisphuta with the Sun added; read with the Trisphuta and the Panchasphuta for the course of an illness",
            "{p}: ത്രിസ്ഫുടത്തോട് സൂര്യനെ കൂട്ടിയത്; രോഗത്തിന്റെ ഗതിക്കായി ത്രിസ്ഫുടത്തോടും പഞ്ചസ്ഫുടത്തോടും ചേർത്തു വായിക്കുന്നു",
            "{p}: त्रिस्फुट में सूर्य जोड़कर; रोग की गति के लिए त्रिस्फुट और पंचस्फुट के साथ पढ़ा जाता है"
        )],
    )
}

/// Panchasphuta = Chatussphuta + Rahu (Prasna Marga VIII.5).
pub fn panchasphuta(lagna: f64, moon: f64, gulika: f64, sun: f64, rahu: f64) -> Sphuta {
    panchasphuta_in(Lang::En, lagna, moon, gulika, sun, rahu)
}

/// [`panchasphuta`] in the reader's language.
pub fn panchasphuta_in(lang: Lang, lagna: f64, moon: f64, gulika: f64, sun: f64, rahu: f64) -> Sphuta {
    let p = pm(lang, "VIII.5", "8.5");
    make_in(
        lang,
        "Panchasphuta",
        lagna + moon + gulika + sun + rahu,
        format!("{} + {} {rahu:.4}", fw(lang, "Chatussphuta"), fw(lang, "Rahu")),
        vec![say3!(
            lang,
            "{p}: the Chatussphuta with Rahu added",
            "{p}: ചതുസ്ഫുടത്തോട് രാഹുവിനെ കൂട്ടിയത്",
            "{p}: चतुःस्फुट में राहु जोड़कर"
        )],
    )
}

/// Pranasphuta = Lagna × 5 + Gulika (Prasna Marga VIII.6).
pub fn pranasphuta(lagna: f64, gulika: f64) -> Sphuta {
    pranasphuta_in(Lang::En, lagna, gulika)
}

/// [`pranasphuta`] in the reader's language.
pub fn pranasphuta_in(lang: Lang, lagna: f64, gulika: f64) -> Sphuta {
    make_in(
        lang,
        "Pranasphuta",
        5.0 * lagna + gulika,
        format!("5 × {} {lagna:.4} + {} {gulika:.4}", fw(lang, "Lagna"), fw(lang, "Gulika")),
        vec![prana_deha_mrityu_reading(lang)],
    )
}

/// Dehasphuta = Moon × 8 + Gulika (Prasna Marga VIII.7).
pub fn dehasphuta(moon: f64, gulika: f64) -> Sphuta {
    dehasphuta_in(Lang::En, moon, gulika)
}

/// [`dehasphuta`] in the reader's language.
pub fn dehasphuta_in(lang: Lang, moon: f64, gulika: f64) -> Sphuta {
    make_in(
        lang,
        "Dehasphuta",
        8.0 * moon + gulika,
        format!("8 × {} {moon:.4} + {} {gulika:.4}", fw(lang, "Moon"), fw(lang, "Gulika")),
        vec![prana_deha_mrityu_reading(lang)],
    )
}

/// Mrityusphuta = Gulika × 7 + Sun (Prasna Marga VIII.8).
pub fn mrityusphuta(gulika: f64, sun: f64) -> Sphuta {
    mrityusphuta_in(Lang::En, gulika, sun)
}

/// [`mrityusphuta`] in the reader's language.
pub fn mrityusphuta_in(lang: Lang, gulika: f64, sun: f64) -> Sphuta {
    make_in(
        lang,
        "Mrityusphuta",
        7.0 * gulika + sun,
        format!("7 × {} {gulika:.4} + {} {sun:.4}", fw(lang, "Gulika"), fw(lang, "Sun")),
        vec![prana_deha_mrityu_reading(lang)],
    )
}

fn prana_deha_mrityu_reading(lang: Lang) -> String {
    let p = pm(lang, "VIII.6–10", "8.6–10");
    say3!(
        lang,
        "{p} reads the Prana, Deha and Mrityu sphutas together: the transit of malefics, or of the Mrityusphuta's lord, over the Prana and Deha sphutas marks the crisis of an illness, and benefics there mark relief; the app shows the points and the text's words, not a verdict",
        "{p} പ്രാണ, ദേഹ, ഗുളിക-സൂര്യ സ്ഫുടങ്ങളെ ഒന്നിച്ചു വായിക്കുന്നു: പാപഗ്രഹങ്ങളോ ഗുളിക-സൂര്യ സ്ഫുടത്തിന്റെ അധിപനോ പ്രാണ-ദേഹ സ്ഫുടങ്ങൾക്കു മേൽ ഗോചരിക്കുന്നത് രോഗത്തിന്റെ മൂർച്ഛാഘട്ടം കാണിക്കുന്നു, ശുഭഗ്രഹങ്ങൾ അവിടെ വരുന്നത് ആശ്വാസവും; ആപ്പ് ബിന്ദുക്കളും ഗ്രന്ഥവചനങ്ങളും മാത്രം കാണിക്കുന്നു, വിധി പറയുന്നില്ല",
        "{p} प्राण, देह और गुलिक-सूर्य स्फुट को साथ पढ़ता है: पाप ग्रहों का, या गुलिक-सूर्य स्फुट के स्वामी का, प्राण और देह स्फुट पर गोचर रोग के संकट का समय बताता है, और वहाँ शुभ ग्रह राहत; ऐप केवल बिंदु और ग्रंथ के वचन दिखाता है, कोई निर्णय नहीं"
    )
}

/// Sookshma (subtle) Trisphuta = Trisphuta × 9, the Trisphuta's navamsa longitude
/// (Prasna Marga VIII.15: "multiply the Lagna, Moon, Gulika and Trisphuta by nine").
///
/// Convention: the text's "multiply by nine and drop the signs" is the navamsa
/// longitude; summing the navamsa longitudes of the Lagna, Moon and Gulika separately
/// gives the same point modulo 360°, so the two readings agree.
pub fn sookshma_trisphuta(lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    sookshma_trisphuta_in(Lang::En, lagna, moon, gulika)
}

/// [`sookshma_trisphuta`] in the reader's language.
pub fn sookshma_trisphuta_in(lang: Lang, lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    let t = norm360(lagna + moon + gulika);
    let p = pm(lang, "VIII.15–16", "8.15–16");
    make_in(
        lang,
        "Sookshma Trisphuta",
        9.0 * t,
        format!("9 × {} {t:.4}", fw(lang, "Trisphuta")),
        vec![say3!(
            lang,
            "{p}: sickness is read to begin on the day of the nakshatra of the Trisphuta navamsa; the text continues with the next nakshatra days for its stages",
            "{p}: ത്രിസ്ഫുട നവാംശത്തിന്റെ നക്ഷത്രദിവസം രോഗം തുടങ്ങുന്നതായി വായിക്കുന്നു; തുടർന്നുള്ള ഘട്ടങ്ങൾക്ക് ഗ്രന്ഥം അടുത്ത നക്ഷത്രദിവസങ്ങൾ പറയുന്നു",
            "{p}: रोग का आरंभ त्रिस्फुट नवांश के नक्षत्र वाले दिन माना जाता है; आगे की अवस्थाओं के लिए ग्रंथ अगले नक्षत्र-दिन बताता है"
        )],
    )
}

fn parity_words(lang: Lang, name: &str, longitude: f64, wants_odd: bool) -> Vec<String> {
    let rasi_odd = sign_of(longitude) % 2 == 0;
    let nav_odd = sign_of(varga(longitude, 9)) % 2 == 0;
    let want = if wants_odd {
        lang.pick(["odd", "ഓജ", "विषम"])
    } else {
        lang.pick(["even", "യുഗ്മ", "सम"])
    };
    let fits = |b: bool| {
        if b == wants_odd {
            lang.pick(["fits", "ചേരുന്നു", "अनुकूल"])
        } else {
            lang.pick(["does not fit", "ചേരുന്നില്ല", "अनुकूल नहीं"])
        }
    };
    let name = match name {
        "Beeja" => lang.pick(["Beeja", "ബീജം", "बीज"]),
        "Kshetra" => lang.pick(["Kshetra", "ക്ഷേത്രം", "क्षेत्र"]),
        other => other,
    };
    let (s1, f1, s2, f2) = (
        sn(lang, sign_of(longitude)),
        fits(rasi_odd),
        sn(lang, sign_of(varga(longitude, 9))),
        fits(nav_odd),
    );
    vec![
        say3!(
            lang,
            "{name}: rasi {s1} ({f1}), navamsa {s2} ({f2}); the text asks for an {want} sign and an {want} navamsa",
            "{name}: രാശി {s1} ({f1}), നവാംശം {s2} ({f2}); ഗ്രന്ഥം {want}രാശിയും {want} നവാംശവും ആവശ്യപ്പെടുന്നു",
            "{name}: राशि {s1} ({f1}), नवांश {s2} ({f2}); ग्रंथ {want} राशि और {want} नवांश माँगता है"
        ),
        if rasi_odd == wants_odd && nav_odd == wants_odd {
            lang.pick([
                "both the sign and the navamsa fit: the text's favourable configuration",
                "രാശിയും നവാംശവും ചേരുന്നു: ഗ്രന്ഥം പറയുന്ന ശുഭസ്ഥിതി",
                "राशि और नवांश दोनों अनुकूल: ग्रंथ की शुभ स्थिति",
            ])
            .into()
        } else if rasi_odd == wants_odd || nav_odd == wants_odd {
            lang.pick([
                "one of the two fits: a middling configuration in the text",
                "രണ്ടിൽ ഒന്നു മാത്രം ചേരുന്നു: ഗ്രന്ഥപ്രകാരം മധ്യമസ്ഥിതി",
                "दोनों में से एक अनुकूल: ग्रंथ के अनुसार मध्यम स्थिति",
            ])
            .into()
        } else {
            lang.pick([
                "neither fits: the text's unfavourable configuration; the text adds that benefics' aspects or conjunction repair it",
                "രണ്ടും ചേരുന്നില്ല: ഗ്രന്ഥം പറയുന്ന അശുഭസ്ഥിതി; ശുഭഗ്രഹങ്ങളുടെ ദൃഷ്ടിയോ യോഗമോ അതു പരിഹരിക്കുമെന്നും ഗ്രന്ഥം ചേർക്കുന്നു",
                "दोनों अनुकूल नहीं: ग्रंथ की अशुभ स्थिति; ग्रंथ यह भी कहता है कि शुभ ग्रहों की दृष्टि या युति इसे सुधारती है",
            ])
            .into()
        },
    ]
}

/// Beeja sphuta = Sun + Venus + Jupiter, the male factor (Prasna Marga Part II, the
/// chapter on progeny): favourable in an odd sign and an odd navamsa.
pub fn beeja_sphuta(sun: f64, venus: f64, jupiter: f64) -> Sphuta {
    beeja_sphuta_in(Lang::En, sun, venus, jupiter)
}

/// [`beeja_sphuta`] in the reader's language.
pub fn beeja_sphuta_in(lang: Lang, sun: f64, venus: f64, jupiter: f64) -> Sphuta {
    let l = norm360(sun + venus + jupiter);
    make_in(
        lang,
        "Beeja sphuta",
        l,
        format!(
            "{} {sun:.4} + {} {venus:.4} + {} {jupiter:.4}",
            fw(lang, "Sun"),
            fw(lang, "Venus"),
            fw(lang, "Jupiter")
        ),
        parity_words(lang, "Beeja", l, true),
    )
}

/// Kshetra sphuta = Moon + Mars + Jupiter, the female factor (Prasna Marga Part II,
/// the chapter on progeny): favourable in an even sign and an even navamsa.
///
/// Convention: every reading of the text consulted sums the Moon, Mars and Jupiter
/// (Jupiter being the karaka of children in both sphutas); a Moon + Mars + Venus form
/// circulates in some software and is not used here.
pub fn kshetra_sphuta(moon: f64, mars: f64, jupiter: f64) -> Sphuta {
    kshetra_sphuta_in(Lang::En, moon, mars, jupiter)
}

/// [`kshetra_sphuta`] in the reader's language.
pub fn kshetra_sphuta_in(lang: Lang, moon: f64, mars: f64, jupiter: f64) -> Sphuta {
    let l = norm360(moon + mars + jupiter);
    make_in(
        lang,
        "Kshetra sphuta",
        l,
        format!(
            "{} {moon:.4} + {} {mars:.4} + {} {jupiter:.4}",
            fw(lang, "Moon"),
            fw(lang, "Mars"),
            fw(lang, "Jupiter")
        ),
        parity_words(lang, "Kshetra", l, false),
    )
}

/// The Yogi point and its three planets.
#[derive(Clone, Debug, PartialEq)]
pub struct Yogi {
    /// Sun + Moon + 93°20′.
    pub point: Sphuta,
    /// Lord of the Yogi point's nakshatra.
    pub yogi: DashaLord,
    /// Lord of the Yogi point's sign (the duplicate Yogi).
    pub sahayogi: DashaLord,
    /// The Avayogi point: Yogi point + 186°40′.
    pub avayogi_point: f64,
    /// Lord of the Avayogi point's nakshatra.
    pub avayogi: DashaLord,
    /// The reading, in words.
    pub words: Vec<String>,
}

/// The Yogi point (Sun + Moon + 93°20′) with the Yogi (its nakshatra lord), the
/// Sahayogi (its sign lord) and the Avayogi (the nakshatra lord 186°40′ on).
///
/// Convention: 186°40′ is fourteen nakshatras, and since the Vimshottari lords repeat
/// every nine, the Avayogi is also the lord of the sixth nakshatra counted from the
/// Yogi's; the two descriptions in circulation name the same planet. A few authors take
/// the sign lord of the Avayogi point instead; that is not done here.
pub fn yogi(sun: f64, moon: f64) -> Yogi {
    yogi_in(Lang::En, sun, moon)
}

/// [`yogi`] in the reader's language.
pub fn yogi_in(lang: Lang, sun: f64, moon: f64) -> Yogi {
    let l = norm360(sun + moon + 93.0 + 20.0 / 60.0);
    let point = make_in(
        lang,
        "Yogi point",
        l,
        format!("{} {sun:.4} + {} {moon:.4} + 93°20′", fw(lang, "Sun"), fw(lang, "Moon")),
        Vec::new(),
    );
    let yogi = point.star_lord;
    let sahayogi = point.lord;
    let avayogi_point = norm360(l + 186.0 + 40.0 / 60.0);
    let avayogi = kp_lords(avayogi_point).star;
    let (yg, star) = (gn(lang, yogi), nn(lang, point.nakshatra));
    let (sg, sign) = (gn(lang, sahayogi), sn(lang, point.sign));
    let same = if sahayogi == yogi {
        lang.pick([" (the same planet)", " (അതേ ഗ്രഹം)", " (वही ग्रह)"])
    } else {
        ""
    };
    let (ag, astar) = (gn(lang, avayogi), nn(lang, nakshatra(avayogi_point).index));
    let mixed = if avayogi == sahayogi {
        lang.pick([
            "; it is also the Sahayogi, so the reading is mixed",
            "; ഇതുതന്നെ സഹയോഗിയും ആയതിനാൽ ഫലം സമ്മിശ്രം",
            "; यही सहयोगी भी है, इसलिए फल मिश्रित है",
        ])
    } else {
        ""
    };
    let words = vec![
        say3!(
            lang,
            "Yogi {yg}: the lord of {star}; its periods and transits are read as the chart's prosperity",
            "യോഗി {yg}: {star} നക്ഷത്രത്തിന്റെ അധിപൻ; ഇതിന്റെ ദശകളും ഗോചരങ്ങളും ജാതകത്തിന്റെ ഐശ്വര്യമായി വായിക്കുന്നു",
            "योगी {yg}: {star} नक्षत्र का स्वामी; इसकी दशाएँ और गोचर कुंडली की समृद्धि माने जाते हैं"
        ),
        say3!(
            lang,
            "Sahayogi {sg}: the lord of {sign}, the Yogi's helper{same}",
            "സഹയോഗി {sg}: {sign} രാശിയുടെ അധിപൻ, യോഗിയുടെ സഹായി{same}",
            "सहयोगी {sg}: {sign} का स्वामी, योगी का सहायक{same}"
        ),
        say3!(
            lang,
            "Avayogi {ag}: the lord of {astar} at {avayogi_point:.4}° (Yogi point + 186°40′, the sixth nakshatra's lord); its periods and transits are read as obstruction{mixed}",
            "അവയോഗി {ag}: {avayogi_point:.4}°-ലെ {astar} നക്ഷത്രത്തിന്റെ അധിപൻ (യോഗി ബിന്ദു + 186°40′, ആറാം നക്ഷത്രത്തിന്റെ അധിപൻ); ഇതിന്റെ ദശകളും ഗോചരങ്ങളും തടസ്സമായി വായിക്കുന്നു{mixed}",
            "अवयोगी {ag}: {avayogi_point:.4}° पर {astar} नक्षत्र का स्वामी (योगी बिंदु + 186°40′, छठे नक्षत्र का स्वामी); इसकी दशाएँ और गोचर बाधा माने जाते हैं{mixed}"
        ),
    ];
    Yogi {
        point,
        yogi,
        sahayogi,
        avayogi_point,
        avayogi,
        words,
    }
}

/// The Bhrigu Bindu: the midpoint of the Moon and Rahu on the shorter arc.
///
/// Convention: the plain average of two longitudes picks the midpoint of the longer arc
/// whenever the two straddle 0° Aries; the shorter-arc midpoint is taken here, as
/// blog.cosmicinsights.net and astroindus.com describe. With the Moon at 10° and Rahu
/// at 300° this gives 335°, not 155°.
pub fn bhrigu_bindu(moon: f64, rahu: f64) -> Sphuta {
    bhrigu_bindu_in(Lang::En, moon, rahu)
}

/// [`bhrigu_bindu`] in the reader's language.
pub fn bhrigu_bindu_in(lang: Lang, moon: f64, rahu: f64) -> Sphuta {
    let d = (rahu - moon).rem_euclid(360.0);
    let half = if d > 180.0 {
        (d - 360.0) / 2.0
    } else {
        d / 2.0
    };
    let l = norm360(moon + half);
    let (m, r) = (fw(lang, "Moon"), fw(lang, "Rahu"));
    make_in(
        lang,
        "Bhrigu Bindu",
        l,
        say3!(
            lang,
            "midpoint of {m} {moon:.4} and {r} {rahu:.4} on the shorter arc",
            "{m} {moon:.4}, {r} {rahu:.4} എന്നിവയുടെ ചെറിയ ചാപത്തിലെ മധ്യബിന്ദു",
            "{m} {moon:.4} और {r} {rahu:.4} का छोटे चाप पर मध्य बिंदु"
        ),
        vec![lang
            .pick([
                "the midpoint of the Moon and Rahu on the shorter arc (convention stated); Bhrigu Nandi Nadi practice watches the transits of Jupiter, Saturn and the nodes over this point and its trines",
                "ചന്ദ്രന്റെയും രാഹുവിന്റെയും ചെറിയ ചാപത്തിലെ മധ്യബിന്ദു (സ്വീകരിച്ച രീതി); ഭൃഗു നന്ദി നാഡി സമ്പ്രദായം ഈ ബിന്ദുവിനും അതിന്റെ ത്രികോണങ്ങൾക്കും മേലുള്ള വ്യാഴം, ശനി, രാഹു-കേതുക്കൾ എന്നിവയുടെ ഗോചരം ശ്രദ്ധിക്കുന്നു",
                "चंद्र और राहु का छोटे चाप पर मध्य बिंदु (घोषित पद्धति); भृगु नंदी नाड़ी परंपरा इस बिंदु और इसके त्रिकोणों पर गुरु, शनि और राहु-केतु के गोचर देखती है",
            ])
            .to_string()],
    )
}

/// Which instant of Saturn's part gives Gulika.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GulikaConvention {
    /// The start of Saturn's part of the day or night (Jagannatha Hora's default).
    JhoraStart,
    /// The Kerala thirty-ghati Mandi table (Sun 26, Mon 22, … of the day; 10, 6, … of
    /// the night).
    KeralaMandi,
}

/// Gulika's longitude and where it came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GulikaPosition {
    /// Convention used.
    pub convention: GulikaConvention,
    /// The instant of the kala point.
    pub at: Instant,
    /// True when taken from the night's parts.
    pub night: bool,
    /// Sidereal longitude of the ascendant at that instant, degrees.
    pub sidereal: f64,
}

/// Gulika for a birth or question at `at`: the sidereal ascendant at the start of
/// Saturn's part (Jagannatha Hora's default), in the day's parts when `at` is before
/// sunset and in the night's parts otherwise.
pub fn gulika_longitude(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<GulikaPosition, MargaError> {
    gulika_longitude_with(spk, eop, obs, at, ayanamsa, GulikaConvention::JhoraStart)
}

/// Gulika under a stated convention.
pub fn gulika_longitude_with(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    ayanamsa: Ayanamsa,
    convention: GulikaConvention,
) -> Result<GulikaPosition, MargaError> {
    let day = solar_day(spk, eop, obs, at)?.ok_or(MargaError::NoSolarDay)?;
    let night = at >= day.sunset;
    Ok(match convention {
        GulikaConvention::JhoraStart => {
            let u = upagraha_longitude(
                eop,
                obs,
                &day,
                Upagraha::Gulika,
                KalaPoint::Start,
                at,
                ayanamsa,
            );
            GulikaPosition {
                convention,
                at: u.at,
                night: u.night,
                sidereal: u.sidereal,
            }
        }
        GulikaConvention::KeralaMandi => {
            let t = mandi_instant_kerala(&day, obs, night);
            GulikaPosition {
                convention,
                at: t,
                night,
                sidereal: ascendant_sidereal(eop, obs, t, ayanamsa),
            }
        }
    })
}

/// All the Prasna Marga sphutas of a chart, given Gulika: Trisphuta, Chatussphuta,
/// Panchasphuta, Pranasphuta, Dehasphuta, Mrityusphuta, Sookshma Trisphuta, Beeja,
/// Kshetra, the Yogi point and the Bhrigu Bindu, in that order. Rahu is the true node.
pub fn sphutas(chart: &Chart, gulika: f64) -> Result<Vec<Sphuta>, MargaError> {
    sphutas_in(chart, gulika, Lang::En)
}

/// [`sphutas`] in the reader's language.
pub fn sphutas_in(chart: &Chart, gulika: f64, lang: Lang) -> Result<Vec<Sphuta>, MargaError> {
    let lagna = chart.cusps_sidereal[0];
    let sun = body_sidereal(chart, "Sun")?;
    let moon = body_sidereal(chart, "Moon")?;
    let mars = body_sidereal(chart, "Mars")?;
    let jupiter = body_sidereal(chart, "Jupiter")?;
    let venus = body_sidereal(chart, "Venus")?;
    let rahu = chart.rahu().1;
    Ok(vec![
        trisphuta_in(lang, lagna, moon, gulika),
        chatussphuta_in(lang, lagna, moon, gulika, sun),
        panchasphuta_in(lang, lagna, moon, gulika, sun, rahu),
        pranasphuta_in(lang, lagna, gulika),
        dehasphuta_in(lang, moon, gulika),
        mrityusphuta_in(lang, gulika, sun),
        sookshma_trisphuta_in(lang, lagna, moon, gulika),
        beeja_sphuta_in(lang, sun, venus, jupiter),
        kshetra_sphuta_in(lang, moon, mars, jupiter),
        yogi_in(lang, sun, moon).point,
        bhrigu_bindu_in(lang, moon, rahu),
    ])
}

/// The sphutas of a chart with the Gulika that produced them.
#[derive(Clone, Debug, PartialEq)]
pub struct SphutaReport {
    /// Gulika.
    pub gulika: GulikaPosition,
    /// The eleven sphutas of [`sphutas`].
    pub sphutas: Vec<Sphuta>,
    /// The Yogi point and its planets.
    pub yogi: Yogi,
    /// The Bhrigu Bindu.
    pub bhrigu_bindu: Sphuta,
}

/// Gulika from the sky, then every sphuta of the chart. `ayanamsa` must be the one the
/// chart was cast with.
pub fn sphuta_report(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
) -> Result<SphutaReport, MargaError> {
    sphuta_report_in(spk, eop, obs, chart, ayanamsa, Lang::En)
}

/// [`sphuta_report`] in the reader's language.
pub fn sphuta_report_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
    lang: Lang,
) -> Result<SphutaReport, MargaError> {
    let gulika = gulika_longitude(spk, eop, obs, chart.instant, ayanamsa)?;
    let all = sphutas_in(chart, gulika.sidereal, lang)?;
    let sun = body_sidereal(chart, "Sun")?;
    let moon = body_sidereal(chart, "Moon")?;
    Ok(SphutaReport {
        gulika,
        yogi: yogi_in(lang, sun, moon),
        bhrigu_bindu: bhrigu_bindu_in(lang, moon, chart.rahu().1),
        sphutas: all,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_birth, kochi_chart, KOCHI};

    #[test]
    fn formulas_on_hand_values() {
        // Raman's worked example: 43°9′ + 18°29′ + 64°26′ = 126°4′
        let t = trisphuta(43.0 + 9.0 / 60.0, 18.0 + 29.0 / 60.0, 64.0 + 26.0 / 60.0);
        assert!(
            (t.longitude - (126.0 + 4.0 / 60.0)).abs() < 1e-9,
            "{}",
            t.longitude
        );
        assert_eq!(t.sign, 4);
        assert_eq!(t.lord, DashaLord::Sun);
        let c = chatussphuta(100.0, 50.0, 20.0, 300.0);
        assert!((c.longitude - 110.0).abs() < 1e-9);
        let p = panchasphuta(100.0, 50.0, 20.0, 300.0, 200.0);
        assert!((p.longitude - 310.0).abs() < 1e-9);
        assert!((pranasphuta(100.0, 20.0).longitude - 160.0).abs() < 1e-9);
        assert!((dehasphuta(50.0, 20.0).longitude - 60.0).abs() < 1e-9);
        assert!((mrityusphuta(20.0, 300.0).longitude - 80.0).abs() < 1e-9);
        // 9 × Trisphuta: 9 × 170 = 1530 → 90
        assert!((sookshma_trisphuta(100.0, 50.0, 20.0).longitude - 90.0).abs() < 1e-9);
        assert!((beeja_sphuta(10.0, 20.0, 30.0).longitude - 60.0).abs() < 1e-9);
        assert!((kshetra_sphuta(350.0, 20.0, 30.0).longitude - 40.0).abs() < 1e-9);
        for s in [t, c, p] {
            assert!(!s.words.is_empty());
            assert!(s.words[0].contains('='));
        }
    }

    #[test]
    fn yogi_point_lords() {
        // Sun 1°, Moon 0°: Yogi point 94°20′ in Pushya (Saturn); sign Cancer (Moon);
        // Avayogi point 281° in Shravana (Moon), whose lord is also that of Hasta, the
        // sixth nakshatra from Pushya
        let y = yogi(1.0, 0.0);
        assert!((y.point.longitude - (94.0 + 20.0 / 60.0)).abs() < 1e-9);
        assert_eq!(y.point.nakshatra, 7);
        assert_eq!(y.yogi, DashaLord::Saturn);
        assert_eq!(y.sahayogi, DashaLord::Moon);
        assert!((y.avayogi_point - 281.0).abs() < 1e-9);
        assert_eq!(nakshatra(y.avayogi_point).index, 21);
        assert_eq!(y.avayogi, DashaLord::Moon);
        // the sixth nakshatra from the Yogi's has the same lord as the Avayogi point's
        for (s, m) in [(12.3, 200.1), (359.0, 2.0), (150.0, 150.0), (77.7, 311.2)] {
            let y = yogi(s, m);
            let sixth = (y.point.nakshatra + 5) % 27;
            assert_eq!(DashaLord::ORDER[sixth % 9], y.avayogi);
        }
        assert_eq!(y.words.len(), 3);
    }

    #[test]
    fn bhrigu_bindu_takes_the_shorter_arc() {
        assert!((bhrigu_bindu(10.0, 300.0).longitude - 335.0).abs() < 1e-9);
        assert!((bhrigu_bindu(300.0, 10.0).longitude - 335.0).abs() < 1e-9);
        assert!((bhrigu_bindu(100.0, 200.0).longitude - 150.0).abs() < 1e-9);
        assert!((bhrigu_bindu(0.0, 180.0).longitude - 90.0).abs() < 1e-9);
    }

    #[test]
    fn trisphuta_readings_are_data() {
        let w = trisphuta_words(359.0); // last navamsa of Pisces, Revati (group 8)
        assert!(w.iter().any(|s| s.contains("Pisces") && s.contains("most")));
        assert!(w.iter().any(|s| s.contains("a day")));
        assert!(w.iter().any(|s| s.contains("or sad news")));
        let w = trisphuta_words(5.0); // Aries, Ashwini
        assert!(w.iter().any(|s| s.contains("not one of them")));
        assert!(w.iter().any(|s| s.contains("quarrel")));
        let w = trisphuta_words(100.0); // Cancer, Pushya (group 7)
        assert!(w.iter().any(|s| s.contains("least")));
        assert!(w.iter().any(|s| s.contains("quadruped")));
        assert_eq!(TRISPHUTA_TRINE_EVENTS.iter().flatten().count(), 5);
    }

    #[test]
    fn kochi_sphutas_from_the_kernel() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let r = sphuta_report(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert_eq!(r.gulika.convention, GulikaConvention::JhoraStart);
        // 12:06 UTC is 17:36 local, before sunset at Kochi in late October
        assert!(!r.gulika.night);
        assert!(r.gulika.at < kochi_birth());
        assert_eq!(r.sphutas.len(), 11);
        let lagna = ch.cusps_sidereal[0];
        let moon = ch.bodies[1].sidereal;
        let want = norm360(lagna + moon + r.gulika.sidereal);
        assert!((r.sphutas[0].longitude - want).abs() < 1e-9);
        for s in &r.sphutas {
            assert!((0.0..360.0).contains(&s.longitude));
            assert_eq!(s.sign, sign_of(s.longitude));
            assert!(!s.words.is_empty(), "{}", s.name);
            assert!(s.words.iter().all(|w| !w.is_empty()));
        }
        assert_eq!(r.sphutas[9].longitude, r.yogi.point.longitude);
        assert_eq!(r.sphutas[10].longitude, r.bhrigu_bindu.longitude);
        // the Kerala table gives a different instant and so a different longitude
        let k = gulika_longitude_with(
            &spk,
            None,
            &KOCHI,
            ch.instant,
            Ayanamsa::Lahiri,
            GulikaConvention::KeralaMandi,
        )
        .unwrap();
        assert_eq!(k.convention, GulikaConvention::KeralaMandi);
        assert_ne!(k.at, r.gulika.at);
    }

    /// A Latin word of three or more letters, outside all-capital acronyms.
    fn latin_word(s: &str) -> Option<String> {
        s.split(|c: char| !c.is_ascii_alphabetic())
            .find(|w| w.len() >= 3 && !w.chars().all(|c| c.is_ascii_uppercase()))
            .map(str::to_string)
    }

    /// Words a Malayalam or Hindi reader must never meet on these pages.
    const HARSH: &[&str] = &[
        "മരണ", "മരിക്ക", "മൃത്യു", "മാരക", "അപകട", "പരിക്ക", "ആയുസ്", "വേർപിരി", "വിധവ",
        "मृत्यु", "मरण", "मौत", "तलाक", "विधवा", "अलगाव", "दुर्घटना", "घातक", "चोट", "अल्पायु",
    ];

    fn clean(lang: Lang, what: &str, texts: &[String]) {
        for t in texts {
            assert_eq!(latin_word(t), None, "{lang:?} {what}: {t}");
            for h in HARSH {
                assert!(!t.contains(h), "{lang:?} {what}: {h} in {t}");
            }
        }
    }

    #[test]
    fn every_page_reads_in_malayalam_and_hindi() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let f = kshana_rules::Facts::from_chart(&ch);
        // English is unchanged by the language-aware path.
        let en = sphuta_report(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert_eq!(en, sphuta_report_in(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri, Lang::En).unwrap());
        assert_eq!(crate::ashtamangala(518), crate::ashtamangala::ashtamangala_in(518, Lang::En));
        for lang in [Lang::Ml, Lang::Hi] {
            let r = sphuta_report_in(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri, lang).unwrap();
            for s in r.sphutas.iter().chain([&r.yogi.point, &r.bhrigu_bindu]) {
                clean(lang, s.name, &s.words);
                clean(lang, "name", &[s.name.to_string()]);
            }
            clean(lang, "yogi", &r.yogi.words);
            for a in 0..12 {
                let p = crate::ashtamangala::prasna_sphutas_in(&ch, a, lang).unwrap();
                clean(lang, "prasna", &p.words);
                for s in [&p.lagna, &p.chandra, &p.arudha] {
                    clean(lang, "prasna point", &s.words);
                    clean(lang, "prasna name", &[s.name.to_string()]);
                }
            }
            for n in [111, 123, 234, 345, 456, 567, 678, 781, 812, 888, 518, 647, 90, 5] {
                clean(lang, "ashtamangala", &crate::ashtamangala::ashtamangala_in(n, lang).indications);
            }
            for a in crate::jaimini::argala_in(&f, lang) {
                clean(lang, "argala", &a.words);
            }
            for s in 0..12 {
                clean(lang, "argala ketu", &crate::jaimini::argala_on_sign_in(&f, s, true, lang).words);
            }
            clean(lang, "karakamsa", &crate::jaimini::karakamsa_in(&f, lang).words);
            for h in 1..=12 {
                let p = crate::jaimini::pada_of_in(&f, h, lang);
                clean(lang, "pada", &p.words);
                clean(lang, "pada name", &[p.name]);
            }
            for g in [DashaLord::Sun, DashaLord::Moon, DashaLord::Mars, DashaLord::Mercury, DashaLord::Jupiter, DashaLord::Venus, DashaLord::Saturn, DashaLord::Rahu, DashaLord::Ketu] {
                let c: Vec<String> = crate::sandhi::chidra_lords_in(&f, g, lang).into_iter().map(|x| x.1).collect();
                clean(lang, "chidra", &c);
            }
            for rule in [crate::SandhiRule::SubPeriod, crate::SandhiRule::Fraction(0.125)] {
                clean(lang, "sandhi rule", &[rule.name_in(lang).to_string()]);
            }
            for v in crate::vimshopaka::vimshopaka_in(&ch, lang).unwrap() {
                clean(lang, "vimshopaka", &v.words);
                clean(lang, "grade", &[v.grade.to_string()]);
            }
        }
        // Every trine event and both parity outcomes, at any longitude.
        for lang in [Lang::Ml, Lang::Hi] {
            for i in 0..360 {
                let l = i as f64 + 0.5;
                clean(lang, "trisphuta", &trisphuta_words_in(lang, l));
                clean(lang, "beeja", &beeja_sphuta_in(lang, l, 0.0, 0.0).words);
                clean(lang, "kshetra", &kshetra_sphuta_in(lang, l, 0.0, 0.0).words);
                clean(lang, "yogi", &yogi_in(lang, l, 0.0).words);
            }
        }
    }
}
