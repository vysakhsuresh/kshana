//! The Ashtamangala number of the Kerala prasna, and the Arudha, Veethi and Chatra
//! rasis with the lagna, Chandra and Arudha sphutas.
//!
//! # The number
//!
//! One hundred and eight cowries are divided by the astrologer into three heaps; each
//! heap is counted off in eights and the remainder kept, a remainder of zero counting
//! as eight; the three remainders, read left to right, make a three-digit number with
//! no digit above eight, the Ashtamangala number (Prasna Marga ch. IV; Wikipedia
//! "Ashtamangala prasnam"; shyamasundaradasa.com "Ashtamangala Deva Prasna" part 3).
//! The digits stand for the past, the present and the future, each digit for a planet:
//! 1 Sun, 2 Mars, 3 Jupiter, 4 Mercury, 5 Venus, 6 Saturn, 7 Moon, 8 Rahu; odd digits
//! are read as good, even as bad (the same sources; a tambula scheme with 1 Sun, 2 Moon
//! … 7 Saturn and no Rahu circulates for betel-leaf counts and is not this one).
//!
//! Convention ([`ashtamangala`]): the sign the number signals is the remainder of the
//! number by twelve counted from Aries, a zero remainder being Pisces, and its nakshatra
//! the remainder by twenty-seven from Ashwini, zero being Revati, as Kerala practice
//! reads a number into a rasi; the available translations of the text do not print
//! this step and it is marked as practice, not stanza. The deities and doshas each
//! planet signals ([`DIGIT_SIGNALS`]) follow the planet–deity correspondences of
//! Prasna Marga ch. XV (the deity chapter) as the Kerala Deva prasna applies them.
//!
//! # Arudha, Veethi and Chatra
//!
//! The Arudha rasi is the sign on which the querent's gold piece is placed (Prasna
//! Marga ch. IV and the notes of Raman's edition; the Arudha sphuta used here is the
//! lagna's degree carried into the Arudha sign, a convention). The Veethi rasi follows
//! the Sun: Sun in Taurus, Gemini, Cancer or Leo gives Aries; in Scorpio, Sagittarius,
//! Capricorn or Aquarius Gemini; in Pisces, Aries, Virgo or Libra Taurus (the three
//! paths, uttara, dakshina and madhya; Prasna Marga's table as the Chatra rasi section,
//! Raman p. 275, and sksastro.blogspot.com "Dagdha rasis" transcribe it). The Chatra
//! rasi is the sign reached by counting from the Veethi rasi as many signs as the lagna
//! is from the Arudha, both ends counted (Raman's example: Arudha Taurus, lagna Leo,
//! count four; from Veethi Taurus the fourth is Leo).

use kshana_chart::Chart;
use kshana_vedic::DashaLord;

use crate::sphuta::{gn, make_in, nn, sn, Sphuta};
use kshana_rules::{say3, Lang};
use crate::{body_sidereal, norm360, sign_of, MargaError};

/// For each digit 1–8: the planet, the deity it signals and the dosha or matter it
/// points to in the Kerala Deva prasna (convention stated in the module notes).
pub const DIGIT_SIGNALS: [(DashaLord, &str, &str); 8] = [
    (
        DashaLord::Sun,
        "Siva",
        "the displeasure of the presiding deity, the father's line, authority",
    ),
    (
        DashaLord::Mars,
        "Subrahmanya and Bhadrakali",
        "fierce deities, fire, quarrels, enemies, blood",
    ),
    (
        DashaLord::Jupiter,
        "Brahma and the family deity (Vishnu in some readings)",
        "lapses towards the preceptor and Brahmins, the ancestral shrine",
    ),
    (
        DashaLord::Mercury,
        "Vishnu",
        "learning, trade, speech, vows left unfulfilled",
    ),
    (
        DashaLord::Venus,
        "Lakshmi and the Yakshi",
        "women, wealth, pleasures, the Yakshi's dosha",
    ),
    (
        DashaLord::Saturn,
        "Sasta (Ayyappa) and the Kirata",
        "the departed (pitru and preta dosha), servants, disease",
    ),
    (
        DashaLord::Moon,
        "Durga (Bhagavati)",
        "the mother, the mind, water, the Devi's displeasure",
    ),
    (
        DashaLord::Rahu,
        "the Nagas",
        "sarpa dosha, poison, deceit, the serpent grove",
    ),
];

/// [`DIGIT_SIGNALS`]' deity and signal in Malayalam and Hindi: `[[ml deity, ml signal],
/// [hi deity, hi signal]]` for digits 1–8.
const DIGIT_SIGNALS_IN: [[[&str; 2]; 2]; 8] = [
    [
        ["ശിവൻ", "പ്രതിഷ്ഠാദേവതയുടെ അപ്രീതി, പിതൃപരമ്പര, അധികാരം"],
        ["शिव", "प्रतिष्ठित देवता की अप्रसन्नता, पितृ वंश, अधिकार"],
    ],
    [
        ["സുബ്രഹ്മണ്യനും ഭദ്രകാളിയും", "ഉഗ്രമൂർത്തികൾ, അഗ്നി, കലഹം, ശത്രുക്കൾ, രക്തം"],
        ["सुब्रह्मण्य और भद्रकाली", "उग्र देवता, अग्नि, कलह, शत्रु, रक्त"],
    ],
    [
        ["ബ്രഹ്മാവും കുലദേവതയും (ചില വായനകളിൽ വിഷ്ണു)", "ഗുരുവിനോടും ബ്രാഹ്മണരോടുമുള്ള വീഴ്ചകൾ, തറവാട്ടു ദേവസ്ഥാനം"],
        ["ब्रह्मा और कुलदेवता (कुछ पाठों में विष्णु)", "गुरु और ब्राह्मणों के प्रति चूक, पैतृक देवस्थान"],
    ],
    [
        ["വിഷ്ണു", "വിദ്യ, വ്യാപാരം, വാക്ക്, നിറവേറ്റാത്ത നേർച്ചകൾ"],
        ["विष्णु", "विद्या, व्यापार, वाणी, अधूरी मन्नतें"],
    ],
    [
        ["ലക്ഷ്മിയും യക്ഷിയും", "സ്ത്രീകൾ, ധനം, സുഖഭോഗങ്ങൾ, യക്ഷിദോഷം"],
        ["लक्ष्मी और यक्षी", "स्त्रियाँ, धन, भोग-विलास, यक्षी दोष"],
    ],
    [
        ["ശാസ്താവും (അയ്യപ്പൻ) കിരാതമൂർത്തിയും", "പരേതർ (പിതൃദോഷം, പ്രേതദോഷം), ഭൃത്യർ, രോഗം"],
        ["शास्ता (अय्यप्पा) और किरात", "दिवंगत (पितृ और प्रेत दोष), सेवक, रोग"],
    ],
    [
        ["ദുർഗ്ഗ (ഭഗവതി)", "അമ്മ, മനസ്സ്, ജലം, ദേവിയുടെ അപ്രീതി"],
        ["दुर्गा (भगवती)", "माता, मन, जल, देवी की अप्रसन्नता"],
    ],
    [
        ["നാഗങ്ങൾ", "സർപ്പദോഷം, വിഷം, വഞ്ചന, സർപ്പക്കാവ്"],
        ["नाग देवता", "सर्प दोष, विष, छल, नाग स्थान"],
    ],
];

/// The deity and signal of a digit 1–8 in the reader's language.
fn digit_words(lang: Lang, d: u8) -> (&'static str, &'static str) {
    let i = (d - 1) as usize;
    match lang {
        Lang::En => (DIGIT_SIGNALS[i].1, DIGIT_SIGNALS[i].2),
        Lang::Ml => (DIGIT_SIGNALS_IN[i][0][0], DIGIT_SIGNALS_IN[i][0][1]),
        Lang::Hi => (DIGIT_SIGNALS_IN[i][1][0], DIGIT_SIGNALS_IN[i][1][1]),
    }
}

/// Planet of a digit 1–8, `None` for any other digit.
pub fn digit_planet(d: u8) -> Option<DashaLord> {
    DIGIT_SIGNALS.get(d.checked_sub(1)? as usize).map(|s| s.0)
}

/// An Ashtamangala number read.
#[derive(Clone, Debug, PartialEq)]
pub struct Ashtamangala {
    /// The number.
    pub number: u32,
    /// Its decimal digits, most significant first.
    pub digits: Vec<u8>,
    /// The planet of each digit (`None` for a 0 or a 9).
    pub planets: Vec<Option<DashaLord>>,
    /// The sign the number signals, 0 = Aries.
    pub arudha_sign: u8,
    /// The nakshatra the number signals, 0 = Ashwini; `None` for zero.
    pub nakshatra: Option<usize>,
    /// The readings, in words.
    pub indications: Vec<String>,
}

/// Read an Ashtamangala number.
pub fn ashtamangala(number: u32) -> Ashtamangala {
    ashtamangala_in(number, Lang::En)
}

/// [`ashtamangala`] with the readings in the reader's language.
pub fn ashtamangala_in(number: u32, lang: Lang) -> Ashtamangala {
    let digits: Vec<u8> = number.to_string().bytes().map(|b| b - b'0').collect();
    let planets: Vec<Option<DashaLord>> = digits.iter().map(|d| digit_planet(*d)).collect();
    let r12 = (number % 12) as u8;
    let arudha_sign = (r12 + 11) % 12;
    let nakshatra = if number == 0 {
        None
    } else {
        Some(((number % 27) as usize + 26) % 27)
    };
    let mut indications = Vec::new();
    let labels: [&str; 3] = match lang {
        Lang::En => ["past", "present", "future"],
        Lang::Ml => ["ഭൂതം", "വർത്തമാനം", "ഭാവി"],
        Lang::Hi => ["भूत", "वर्तमान", "भविष्य"],
    };
    let labels: &[&str] = if digits.len() == 3 { &labels } else { &[] };
    for (i, (d, p)) in digits.iter().zip(planets.iter()).enumerate() {
        let label = labels
            .get(i)
            .map(|l| l.to_string())
            .unwrap_or_else(|| say3!(lang, "digit {}", "{}-ാം അക്കം", "अंक {}", i + 1));
        match p {
            Some(g) => {
                let (deity, signal) = digit_words(lang, *d);
                let graha = gn(lang, *g);
                let parity = if d % 2 == 1 {
                    lang.pick(["odd, read as good", "ഓജസംഖ്യ, ശുഭമായി വായിക്കുന്നു", "विषम, शुभ माना जाता है"])
                } else {
                    lang.pick(["even, read as bad", "യുഗ്മസംഖ്യ, അശുഭമായി വായിക്കുന്നു", "सम, अशुभ माना जाता है"])
                };
                indications.push(say3!(
                    lang,
                    "{label}: {d} is {graha} ({parity}); deity {deity}; signals {signal}",
                    "{label}: {d} = {graha} ({parity}); ദേവത {deity}; സൂചന: {signal}",
                    "{label}: {d} = {graha} ({parity}); देवता {deity}; संकेत: {signal}"
                ));
            }
            None => indications.push(say3!(
                lang,
                "{label}: {d} is not an Ashtamangala digit (the cowrie remainders run 1–8)",
                "{label}: {d} അഷ്ടമംഗല അക്കമല്ല (കവടി ശിഷ്ടങ്ങൾ 1–8 വരെ)",
                "{label}: {d} अष्टमंगल अंक नहीं है (कौड़ी शेष 1–8 तक होते हैं)"
            )),
        }
    }
    if digits.len() == 3 {
        let good = digits.iter().filter(|d| *d % 2 == 1).count();
        let verdict = match good {
            3 => lang.pick(["the number is wholly favourable", "സംഖ്യ പൂർണ്ണമായും ശുഭം", "संख्या पूर्णतः शुभ"]),
            2 => lang.pick(["favourable on the whole", "മൊത്തത്തിൽ ശുഭം", "कुल मिलाकर शुभ"]),
            1 => lang.pick(["unfavourable on the whole", "മൊത്തത്തിൽ അശുഭം", "कुल मिलाकर अशुभ"]),
            _ => lang.pick(["the number is wholly unfavourable", "സംഖ്യ പൂർണ്ണമായും അശുഭം", "संख्या पूर्णतः अशुभ"]),
        };
        indications.push(say3!(
            lang,
            "{good} of the three digits are odd: {verdict}",
            "മൂന്ന് അക്കങ്ങളിൽ {good} എണ്ണം ഓജം: {verdict}",
            "तीन अंकों में से {good} विषम हैं: {verdict}"
        ));
    }
    let s = sn(lang, arudha_sign);
    indications.push(say3!(
        lang,
        "remainder by twelve {r12}: the number signals {s} (practice: remainder counted from Aries, zero is Pisces)",
        "പന്ത്രണ്ടു കൊണ്ടുള്ള ശിഷ്ടം {r12}: സംഖ്യ {s} രാശി സൂചിപ്പിക്കുന്നു (ആചാരം: ശിഷ്ടം മേടം മുതൽ എണ്ണുന്നു, പൂജ്യം മീനം)",
        "बारह से भाग का शेष {r12}: संख्या {s} राशि बताती है (परंपरा: शेष मेष से गिना जाता है, शून्य मीन है)"
    ));
    if let Some(n) = nakshatra {
        let (r27, star) = (number % 27, nn(lang, n));
        indications.push(say3!(
            lang,
            "remainder by twenty-seven {r27}: {star} (practice: counted from Ashwini, zero is Revati)",
            "ഇരുപത്തിയേഴു കൊണ്ടുള്ള ശിഷ്ടം {r27}: {star} (ആചാരം: അശ്വതി മുതൽ എണ്ണുന്നു, പൂജ്യം രേവതി)",
            "सत्ताईस से भाग का शेष {r27}: {star} (परंपरा: अश्विनी से गिना जाता है, शून्य रेवती है)"
        ));
    }
    Ashtamangala {
        number,
        digits,
        planets,
        arudha_sign,
        nakshatra,
        indications,
    }
}

/// Veethi rasi from the Sun's sign: Aries for the Sun in Taurus–Leo, Gemini for
/// Scorpio–Aquarius, Taurus for Pisces, Aries, Virgo and Libra.
pub fn veethi_rasi(sun_sign: u8) -> u8 {
    match sun_sign % 12 {
        1..=4 => 0,
        7..=10 => 2,
        _ => 1,
    }
}

/// Chatra rasi: count from the Arudha to the lagna, both ends included, and take that
/// many from the Veethi rasi of the Sun's sign.
pub fn chatra_rasi(arudha_sign: u8, lagna_sign: u8, sun_sign: u8) -> u8 {
    let n = (lagna_sign as i32 - arudha_sign as i32).rem_euclid(12) as u8 + 1;
    (veethi_rasi(sun_sign) + n - 1) % 12
}

/// The Kerala prasna's three sphutas and two derived signs.
#[derive(Clone, Debug, PartialEq)]
pub struct PrasnaSphutas {
    /// The lagna.
    pub lagna: Sphuta,
    /// The Moon.
    pub chandra: Sphuta,
    /// The Arudha: the lagna's degree in the Arudha sign.
    pub arudha: Sphuta,
    /// The Arudha sign given, 0 = Aries.
    pub arudha_sign: u8,
    /// The Veethi rasi of the Sun's sign.
    pub veethi_sign: u8,
    /// The Chatra rasi.
    pub chatra_sign: u8,
    /// The derivation, in words.
    pub words: Vec<String>,
}

/// The prasna sphutas of a chart with the Arudha sign the gold piece marked.
pub fn prasna_sphutas(chart: &Chart, arudha_sign: u8) -> Result<PrasnaSphutas, MargaError> {
    prasna_sphutas_in(chart, arudha_sign, Lang::En)
}

/// [`prasna_sphutas`] in the reader's language.
pub fn prasna_sphutas_in(chart: &Chart, arudha_sign: u8, lang: Lang) -> Result<PrasnaSphutas, MargaError> {
    let arudha_sign = arudha_sign % 12;
    let lagna_lon = chart.cusps_sidereal[0];
    let moon = body_sidereal(chart, "Moon")?;
    let sun = body_sidereal(chart, "Sun")?;
    let lagna_sign = sign_of(lagna_lon);
    let sun_sign = sign_of(sun);
    let arudha_lon = norm360(arudha_sign as f64 * 30.0 + lagna_lon.rem_euclid(30.0));
    let veethi_sign = veethi_rasi(sun_sign);
    let chatra_sign = chatra_rasi(arudha_sign, lagna_sign, sun_sign);
    let n = (lagna_sign as i32 - arudha_sign as i32).rem_euclid(12) + 1;
    let (a, v, s, c, l) = (
        sn(lang, arudha_sign),
        sn(lang, veethi_sign),
        sn(lang, sun_sign),
        sn(lang, chatra_sign),
        sn(lang, lagna_sign),
    );
    let words = vec![
        say3!(
            lang,
            "Arudha {a}: the sign the gold piece marked; its sphuta carries the lagna's degree (convention)",
            "ആരൂഢം {a}: സ്വർണ്ണം വെച്ച രാശി; അതിന്റെ സ്ഫുടത്തിന് ലഗ്നത്തിന്റെ അംശം തന്നെ (സ്വീകരിച്ച രീതി)",
            "आरूढ़ {a}: वह राशि जिस पर स्वर्ण रखा गया; इसके स्फुट में लग्न का अंश लिया जाता है (परिपाटी)"
        ),
        say3!(
            lang,
            "Veethi {v}: the Sun in {s} walks that path",
            "വീഥി {v}: {s} രാശിയിലുള്ള സൂര്യൻ ആ വീഥിയിൽ സഞ്ചരിക്കുന്നു",
            "वीथि {v}: {s} में स्थित सूर्य उसी पथ पर चलता है"
        ),
        say3!(
            lang,
            "Chatra {c}: from Arudha {a} to lagna {l} is {n} counting both ends; the {n}th from Veethi {v}",
            "ഛത്രം {c}: ആരൂഢം {a} മുതൽ ലഗ്നം {l} വരെ ഇരുവശവും കൂട്ടി {n}; വീഥി {v} മുതൽ {n}-ാമത്തേത്",
            "छत्र {c}: आरूढ़ {a} से लग्न {l} तक दोनों सिरे गिनकर {n}; वीथि {v} से {n}वाँ"
        ),
        lang.pick([
            "Prasna Marga reads the Arudha and the Chatra rasi beside the lagna and the Moon; wherever the lagna is named, Chandra lagna is read as well",
            "പ്രശ്നമാർഗ്ഗം ലഗ്നത്തിനും ചന്ദ്രനും ഒപ്പം ആരൂഢവും ഛത്രരാശിയും വായിക്കുന്നു; ലഗ്നം പറയുന്നിടത്തെല്ലാം ചന്ദ്രലഗ്നവും വായിക്കണം",
            "प्रश्न मार्ग लग्न और चंद्र के साथ आरूढ़ और छत्र राशि को भी पढ़ता है; जहाँ भी लग्न का नाम आता है, वहाँ चंद्र लग्न भी पढ़ा जाता है",
        ])
        .into(),
    ];
    Ok(PrasnaSphutas {
        lagna: make_in(
            lang,
            "Lagna",
            lagna_lon,
            lang.pick(["the rising degree", "ഉദയ ലഗ്നാംശം", "उदित अंश"]).into(),
            Vec::new(),
        ),
        chandra: make_in(lang, "Chandra", moon, lang.pick(["the Moon", "ചന്ദ്രൻ", "चंद्र"]).into(), Vec::new()),
        arudha: make_in(
            lang,
            "Arudha",
            arudha_lon,
            say3!(
                lang,
                "Arudha sign {a} + the lagna's degree",
                "ആരൂഢരാശി {a} + ലഗ്നാംശം",
                "आरूढ़ राशि {a} + लग्न का अंश"
            ),
            Vec::new(),
        ),
        arudha_sign,
        veethi_sign,
        chatra_sign,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_chart};

    #[test]
    fn digits_and_planets() {
        let a = ashtamangala(831);
        assert_eq!(a.digits, vec![8, 3, 1]);
        assert_eq!(
            a.planets,
            vec![
                Some(DashaLord::Rahu),
                Some(DashaLord::Jupiter),
                Some(DashaLord::Sun)
            ]
        );
        assert!(a.indications[0].contains("past") && a.indications[0].contains("Rahu"));
        assert!(a.indications[2].contains("odd, read as good"));
        assert!(a.indications.iter().any(|s| s.contains("2 of the three")));
        // 831 mod 12 = 3 → Gemini; 831 mod 27 = 21 → Uttara Ashadha (index 20)
        assert_eq!(a.arudha_sign, 2);
        assert_eq!(a.nakshatra, Some(20));
        assert_eq!(digit_planet(0), None);
        assert_eq!(digit_planet(9), None);
        assert_eq!(digit_planet(7), Some(DashaLord::Moon));
        // remainder zero is Pisces and Revati
        let z = ashtamangala(108);
        assert_eq!(z.arudha_sign, 11);
        assert_eq!(z.nakshatra, Some(26));
        let n = ashtamangala(0);
        assert_eq!(n.nakshatra, None);
        assert!(n.indications[0].contains("not an Ashtamangala digit"));
        assert!(ashtamangala(888).indications.iter().all(|s| !s.is_empty()));
    }

    #[test]
    fn veethi_and_chatra_follow_ramans_example() {
        assert_eq!(veethi_rasi(1), 0); // Sun in Taurus → Aries
        assert_eq!(veethi_rasi(4), 0);
        assert_eq!(veethi_rasi(7), 2); // Scorpio → Gemini
        assert_eq!(veethi_rasi(10), 2);
        assert_eq!(veethi_rasi(11), 1); // Pisces → Taurus
        assert_eq!(veethi_rasi(0), 1);
        assert_eq!(veethi_rasi(5), 1);
        assert_eq!(veethi_rasi(6), 1);
        // Arudha Taurus, lagna Leo, Sun in Aries (Veethi Taurus): count 4 → Leo
        assert_eq!(chatra_rasi(1, 4, 0), 4);
        // Arudha = lagna: count 1 → the Veethi rasi itself
        assert_eq!(chatra_rasi(3, 3, 2), 0);
        // wrap: Arudha Pisces, lagna Aries, Sun in Scorpio (Veethi Gemini): count 2 → Cancer
        assert_eq!(chatra_rasi(11, 0, 7), 3);
    }

    #[test]
    fn kochi_prasna_sphutas() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let p = prasna_sphutas(&ch, 5).unwrap();
        assert_eq!(p.arudha_sign, 5);
        assert_eq!(p.arudha.sign, 5);
        assert!(
            (p.arudha.longitude.rem_euclid(30.0) - p.lagna.longitude.rem_euclid(30.0)).abs() < 1e-9
        );
        assert_eq!(p.lagna.longitude, ch.cusps_sidereal[0]);
        assert_eq!(p.chandra.longitude, ch.bodies[1].sidereal);
        assert_eq!(p.veethi_sign, veethi_rasi(sign_of(ch.bodies[0].sidereal)));
        assert_eq!(
            p.chatra_sign,
            chatra_rasi(5, p.lagna.sign, sign_of(ch.bodies[0].sidereal))
        );
        assert_eq!(p.words.len(), 4);
        assert!(p.words.iter().all(|w| !w.is_empty()));
    }
}
