//! Parashari rules: dignities, graha drishti, the classical yogas, and a per-area
//! assessment from house lords, occupants, aspects and karakas. Houses are counted by
//! sign from the lagna (rasi houses), as Brihat Parashara Hora Shastra does.
//!
//! Every rule names its source. Where schools differ (nodes' aspects, Mercury's
//! nature) the convention used is stated in the rule text.

use crate::areas::Area;
use crate::facts::Facts;
use crate::plain::Lang;
use crate::yogas::words;
use crate::Hit;
use kshana_vedic::{sign_lord, DashaLord};

/// Essential dignity of a graha in its sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Dignity {
    Debilitated,
    Enemy,
    Neutral,
    Friend,
    Own,
    Moolatrikona,
    Exalted,
}

impl Dignity {
    /// Score in `[0, 1]`.
    pub fn score(self) -> f64 {
        match self {
            Dignity::Exalted => 1.0,
            Dignity::Moolatrikona => 0.9,
            Dignity::Own => 0.8,
            Dignity::Friend => 0.6,
            Dignity::Neutral => 0.45,
            Dignity::Enemy => 0.25,
            Dignity::Debilitated => 0.0,
        }
    }
}

/// Exaltation sign (0 = Aries) and degree of deepest exaltation.
pub(crate) fn exaltation(g: DashaLord) -> Option<(u8, f64)> {
    match g {
        DashaLord::Sun => Some((0, 10.0)),
        DashaLord::Moon => Some((1, 3.0)),
        DashaLord::Mars => Some((9, 28.0)),
        DashaLord::Mercury => Some((5, 15.0)),
        DashaLord::Jupiter => Some((3, 5.0)),
        DashaLord::Venus => Some((11, 27.0)),
        DashaLord::Saturn => Some((6, 20.0)),
        _ => None,
    }
}

/// Moolatrikona sign and degree range.
fn moolatrikona(g: DashaLord) -> Option<(u8, f64, f64)> {
    match g {
        DashaLord::Sun => Some((4, 0.0, 20.0)),
        DashaLord::Moon => Some((1, 3.0, 30.0)),
        DashaLord::Mars => Some((0, 0.0, 12.0)),
        DashaLord::Mercury => Some((5, 15.0, 20.0)),
        DashaLord::Jupiter => Some((8, 0.0, 10.0)),
        DashaLord::Venus => Some((6, 0.0, 15.0)),
        DashaLord::Saturn => Some((10, 0.0, 20.0)),
        _ => None,
    }
}

/// Natural relationship (BPHS, naisargika maitri): 1 friend, 0 neutral, −1 enemy.
pub fn natural_relation(a: DashaLord, b: DashaLord) -> i8 {
    use DashaLord::*;
    let friends: &[DashaLord] = match a {
        Sun => &[Moon, Mars, Jupiter],
        Moon => &[Sun, Mercury],
        Mars => &[Sun, Moon, Jupiter],
        Mercury => &[Sun, Venus],
        Jupiter => &[Sun, Moon, Mars],
        Venus => &[Mercury, Saturn],
        Saturn => &[Mercury, Venus],
        _ => &[],
    };
    let enemies: &[DashaLord] = match a {
        Sun => &[Venus, Saturn],
        Moon => &[],
        Mars => &[Mercury],
        Mercury => &[Moon],
        Jupiter => &[Mercury, Venus],
        Venus => &[Sun, Moon],
        Saturn => &[Sun, Moon, Mars],
        _ => &[],
    };
    if friends.contains(&b) {
        1
    } else if enemies.contains(&b) {
        -1
    } else {
        0
    }
}

/// Dignity of a graha at a sidereal longitude. Nodes return `Neutral`.
pub fn dignity(g: DashaLord, sidereal: f64) -> Dignity {
    let sign = ((sidereal / 30.0).floor() as usize % 12) as u8;
    let deg = sidereal.rem_euclid(30.0);
    if matches!(g, DashaLord::Rahu | DashaLord::Ketu) {
        return Dignity::Neutral;
    }
    if let Some((s, _)) = exaltation(g) {
        if s == sign {
            return Dignity::Exalted;
        }
        if (s + 6) % 12 == sign {
            return Dignity::Debilitated;
        }
    }
    if let Some((s, a, b)) = moolatrikona(g) {
        if s == sign && deg >= a && deg < b {
            return Dignity::Moolatrikona;
        }
    }
    let lord = sign_lord(sign as usize);
    if lord == g {
        return Dignity::Own;
    }
    match natural_relation(g, lord) {
        1 => Dignity::Friend,
        -1 => Dignity::Enemy,
        _ => Dignity::Neutral,
    }
}

/// Rasi houses a graha aspects by graha drishti, counted from its own rasi house.
/// Convention: all grahas aspect the 7th; Mars adds 4 and 8, Jupiter 5 and 9, Saturn 3
/// and 10; Rahu and Ketu aspect 5, 7 and 9 (the common modern convention, stated here).
pub fn aspected_houses(g: DashaLord, from_rasi_house: u8) -> Vec<u8> {
    let offs: &[u8] = match g {
        DashaLord::Mars => &[4, 7, 8],
        DashaLord::Jupiter => &[5, 7, 9],
        DashaLord::Saturn => &[3, 7, 10],
        DashaLord::Rahu | DashaLord::Ketu => &[5, 7, 9],
        _ => &[7],
    };
    offs.iter()
        .map(|o| ((from_rasi_house as u16 + *o as u16 - 2) % 12 + 1) as u8)
        .collect()
}

/// Natural benefic or malefic. Mercury is benefic unless conjoined with a malefic; the
/// Moon is benefic when waxing (Shukla paksha).
pub fn is_benefic(facts: &Facts, g: DashaLord) -> bool {
    match g {
        DashaLord::Jupiter | DashaLord::Venus => true,
        DashaLord::Moon => facts.moon_phase_deg < 180.0,
        DashaLord::Mercury => {
            let m = facts.planet(DashaLord::Mercury);
            !facts.planets.iter().any(|p| {
                p.sign == m.sign
                    && matches!(
                        p.graha,
                        DashaLord::Sun
                            | DashaLord::Mars
                            | DashaLord::Saturn
                            | DashaLord::Rahu
                            | DashaLord::Ketu
                    )
            })
        }
        _ => false,
    }
}

/// A detected yoga.
#[derive(Clone, Debug, PartialEq)]
pub struct Yoga {
    /// Name.
    pub name: &'static str,
    /// The rule.
    pub hit: Hit,
}

fn same_sign(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    facts.planet(a).sign == facts.planet(b).sign
}

fn mutual_aspect(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    let pa = facts.planet(a);
    let pb = facts.planet(b);
    aspected_houses(a, pa.rasi_house).contains(&pb.rasi_house)
        && aspected_houses(b, pb.rasi_house).contains(&pa.rasi_house)
}

fn exchange(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    let pa = facts.planet(a);
    let pb = facts.planet(b);
    a != b && sign_lord(pa.sign as usize) == b && sign_lord(pb.sign as usize) == a
}

fn connected(facts: &Facts, a: DashaLord, b: DashaLord, lang: Lang) -> Option<&'static str> {
    if a == b {
        return Some(lang.pick(["the same planet rules both", "ഒരേ ഗ്രഹം തന്നെയാണ്", "एक ही ग्रह हैं"]));
    }
    if same_sign(facts, a, b) {
        Some(lang.pick(["conjoined in one sign", "ഒരേ രാശിയിൽ യോഗം ചെയ്യുന്നു", "एक ही राशि में युत हैं"]))
    } else if exchange(facts, a, b) {
        Some(lang.pick([
            "in mutual exchange of signs (parivartana)",
            "പരസ്പരം രാശി മാറി നിൽക്കുന്നു (പരിവർത്തനം)",
            "परस्पर राशि परिवर्तन में हैं (परिवर्तन)",
        ]))
    } else if mutual_aspect(facts, a, b) {
        Some(lang.pick(["in mutual aspect", "പരസ്പരം ദൃഷ്ടി ചെയ്യുന്നു", "परस्पर दृष्टि में हैं"]))
    } else {
        None
    }
}

/// The lord of a house, as a title: "ലഗ്നാധിപൻ", "सप्तमेश". Not used in English.
fn lord_of(lang: Lang, h: u8) -> String {
    const HI: [&str; 12] = [
        "लग्नेश", "द्वितीयेश", "तृतीयेश", "चतुर्थेश", "पंचमेश", "षष्ठेश", "सप्तमेश", "अष्टमेश", "नवमेश",
        "दशमेश", "एकादशेश", "द्वादशेश",
    ];
    match lang {
        Lang::Hi => HI[((h.max(1) - 1) % 12) as usize].to_string(),
        _ if h == 1 => "ലഗ്നാധിപൻ".to_string(),
        _ => format!("{} ഭാവാധിപൻ", words::bhava(lang, h)),
    }
}

fn nature(lang: Lang, benefic: bool) -> &'static str {
    if benefic {
        lang.pick(["benefic", "ശുഭൻ", "शुभ"])
    } else {
        lang.pick(["malefic", "പാപൻ", "पाप"])
    }
}

/// The name of a yoga from [`yogas`] in the reader's language; unknown names come back
/// unchanged.
pub fn yoga_name_in(lang: Lang, name: &str) -> String {
    const NAMES: [[&str; 3]; 15] = [
        ["Gajakesari", "ഗജകേസരി യോഗം", "गजकेसरी योग"],
        ["Budhaditya", "ബുധാദിത്യ യോഗം", "बुधादित्य योग"],
        ["Chandra-Mangala", "ചന്ദ്രമംഗള യോഗം", "चंद्र-मंगल योग"],
        ["Ruchaka", "രുചക യോഗം", "रुचक योग"],
        ["Bhadra", "ഭദ്ര യോഗം", "भद्र योग"],
        ["Hamsa", "ഹംസ യോഗം", "हंस योग"],
        ["Malavya", "മാളവ്യ യോഗം", "मालव्य योग"],
        ["Sasa", "ശശ യോഗം", "शश योग"],
        ["Raja yoga", "രാജയോഗം", "राजयोग"],
        ["Dhana yoga", "ധനയോഗം", "धनयोग"],
        ["Viparita Raja yoga", "വിപരീത രാജയോഗം", "विपरीत राजयोग"],
        ["Kemadruma", "കേമദ്രുമ യോഗം", "केमद्रुम योग"],
        ["Kemadruma (cancelled)", "കേമദ്രുമ യോഗം (ഭംഗം)", "केमद्रुम योग (भंग)"],
        ["Neecha Bhanga", "നീചഭംഗം", "नीचभंग"],
        ["Kala Sarpa", "കാലസർപ്പ യോഗം", "कालसर्प योग"],
    ];
    NAMES.iter().find(|t| t[0] == name).map(|t| lang.pick(*t).to_string()).unwrap_or_else(|| name.to_string())
}

fn is_kendra(h: u8) -> bool {
    matches!(h, 1 | 4 | 7 | 10)
}
fn is_trikona(h: u8) -> bool {
    matches!(h, 1 | 5 | 9)
}
fn is_dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}

/// Detect the classical yogas present.
pub fn yogas(facts: &Facts) -> Vec<Yoga> {
    yogas_in(facts, Lang::En)
}

/// [`yogas`] with the factors and sources in the reader's language. The `name` stays
/// the English name (a key); show it through [`yoga_name_in`].
pub fn yogas_in(facts: &Facts, lang: Lang) -> Vec<Yoga> {
    use DashaLord::*;
    let mut out = Vec::new();
    let moon = facts.planet(Moon);
    let jup = facts.planet(Jupiter);
    let gn = |g: DashaLord| words::graha(lang, g);

    // Gajakesari: Jupiter in a kendra from the Moon.
    let from_moon = ((jup.sign as i32 - moon.sign as i32).rem_euclid(12) + 1) as u8;
    if is_kendra(from_moon) {
        let fm = words::bhava(lang, from_moon);
        out.push(Yoga {
            name: "Gajakesari",
            hit: Hit {
                rule_id: "KS-Y-0001",
                source: lang.pick([
                    "BPHS, Yogadhyaya; Phaladeepika 6.4: Jupiter in a kendra from the Moon",
                    "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, യോഗാധ്യായം; ഫലദീപിക 6.4: ചന്ദ്രന്റെ കേന്ദ്രത്തിൽ വ്യാഴം",
                    "बृहत् पराशर होरा शास्त्र, योगाध्याय; फलदीपिका 6.4: चंद्र से केंद्र में गुरु",
                ]),
                weight: 0.7,
                factors: vec![crate::say3!(
                    lang,
                    "Jupiter is in the {from_moon}th sign from the Moon",
                    "വ്യാഴം ചന്ദ്രനിൽ നിന്ന് {fm} രാശിയിൽ",
                    "गुरु चंद्र से {fm} राशि में है"
                )],
            },
        });
    }
    // Budhaditya
    if same_sign(facts, Sun, Mercury) {
        out.push(Yoga {
            name: "Budhaditya",
            hit: Hit {
                rule_id: "KS-Y-0002",
                source: lang.pick([
                    "Phaladeepika 6: Sun and Mercury in one sign",
                    "ഫലദീപിക 6: സൂര്യനും ബുധനും ഒരേ രാശിയിൽ",
                    "फलदीपिका 6: सूर्य और बुध एक ही राशि में",
                ]),
                weight: 0.5,
                factors: vec![lang.pick(["Sun and Mercury conjoined", "സൂര്യനും ബുധനും യോഗത്തിൽ", "सूर्य और बुध की युति"]).into()],
            },
        });
    }
    // Chandra-Mangala
    if same_sign(facts, Moon, Mars) {
        out.push(Yoga {
            name: "Chandra-Mangala",
            hit: Hit {
                rule_id: "KS-Y-0003",
                source: lang.pick([
                    "BPHS, Chandra yogas: Moon and Mars in one sign",
                    "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ചന്ദ്രയോഗങ്ങൾ: ചന്ദ്രനും ചൊവ്വയും ഒരേ രാശിയിൽ",
                    "बृहत् पराशर होरा शास्त्र, चंद्र योग: चंद्र और मंगल एक ही राशि में",
                ]),
                weight: 0.5,
                factors: vec![lang.pick(["Moon and Mars conjoined", "ചന്ദ്രനും ചൊവ്വയും യോഗത്തിൽ", "चंद्र और मंगल की युति"]).into()],
            },
        });
    }
    // Panch Mahapurusha
    for (g, name) in [
        (Mars, "Ruchaka"),
        (Mercury, "Bhadra"),
        (Jupiter, "Hamsa"),
        (Venus, "Malavya"),
        (Saturn, "Sasa"),
    ] {
        let p = facts.planet(g);
        let d = dignity(g, p.sidereal);
        if is_kendra(p.rasi_house)
            && matches!(d, Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted)
        {
            out.push(Yoga {
                name,
                hit: Hit {
                    rule_id: "KS-Y-0010",
                    source: lang.pick([
                        "BPHS, Pancha Mahapurusha yogas: the planet in its own or exaltation sign in a kendra from the lagna",
                        "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, പഞ്ചമഹാപുരുഷ യോഗങ്ങൾ: ഗ്രഹം സ്വക്ഷേത്രത്തിലോ ഉച്ചത്തിലോ ലഗ്നകേന്ദ്രത്തിൽ",
                        "बृहत् पराशर होरा शास्त्र, पंच महापुरुष योग: ग्रह स्वराशि या उच्च राशि में लग्न से केंद्र में",
                    ]),
                    weight: 0.8,
                    factors: vec![{
                        let (rh, n, dw, h) = (p.rasi_house, gn(g), words::dignity(lang, d), words::bhava(lang, p.rasi_house));
                        crate::say3!(
                            lang,
                            "{g:?} is {d:?} in rasi house {rh}",
                            "{n} {h} ഭാവത്തിൽ {dw}",
                            "{n} {h} भाव में {dw} है"
                        )
                    }],
                },
            });
        }
    }
    // Raja yoga: a kendra lord and a trikona lord connected.
    let lords = &facts.rasi_lords;
    let mut seen: Vec<(DashaLord, DashaLord)> = Vec::new();
    for kh in [1u8, 4, 7, 10] {
        for th in [1u8, 5, 9] {
            if kh == th {
                continue;
            }
            let kl = lords[(kh - 1) as usize];
            let tl = lords[(th - 1) as usize];
            if kl == tl || seen.contains(&(kl, tl)) || seen.contains(&(tl, kl)) {
                continue;
            }
            if let Some(how) = connected(facts, kl, tl, lang) {
                seen.push((kl, tl));
                let (kn, tn, ko, to) = (gn(kl), gn(tl), lord_of(lang, kh), lord_of(lang, th));
                out.push(Yoga {
                    name: "Raja yoga",
                    hit: Hit {
                        rule_id: "KS-Y-0020",
                        source: lang.pick([
                            "BPHS, Raja yogadhyaya: lords of a kendra and a trikona in association",
                            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, രാജയോഗാധ്യായം: കേന്ദ്രാധിപനും ത്രികോണാധിപനും തമ്മിൽ ബന്ധം",
                            "बृहत् पराशर होरा शास्त्र, राजयोगाध्याय: केंद्रेश और त्रिकोणेश का संबंध",
                        ]),
                        weight: 0.9,
                        factors: vec![crate::say3!(
                            lang,
                            "{kl:?} (lord of {kh}) and {tl:?} (lord of {th}) are {how}",
                            "{kn} ({ko}), {tn} ({to}) എന്നിവർ {how}",
                            "{kn} ({ko}) और {tn} ({to}) {how}"
                        )],
                    },
                });
            }
        }
    }
    // Dhana yoga: 2nd and 11th lords connected, or either with the 5th/9th lord.
    let l2 = lords[1];
    let l11 = lords[10];
    for (a, b, label) in [
        (l2, l11, lang.pick(["2nd and 11th lords", "രണ്ടാം ഭാവാധിപനും പതിനൊന്നാം ഭാവാധിപനും", "द्वितीयेश और एकादशेश"])),
        (l2, lords[4], lang.pick(["2nd and 5th lords", "രണ്ടാം ഭാവാധിപനും അഞ്ചാം ഭാവാധിപനും", "द्वितीयेश और पंचमेश"])),
        (l11, lords[8], lang.pick(["11th and 9th lords", "പതിനൊന്നാം ഭാവാധിപനും ഒമ്പതാം ഭാവാധിപനും", "एकादशेश और नवमेश"])),
    ] {
        if a != b {
            if let Some(how) = connected(facts, a, b, lang) {
                let (an, bn) = (gn(a), gn(b));
                out.push(Yoga {
                    name: "Dhana yoga",
                    hit: Hit {
                        rule_id: "KS-Y-0030",
                        source: lang.pick([
                            "BPHS, Dhana yogadhyaya: lords of wealth houses in association",
                            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ധനയോഗാധ്യായം: ധനഭാവാധിപന്മാർ തമ്മിൽ ബന്ധം",
                            "बृहत् पराशर होरा शास्त्र, धनयोगाध्याय: धन भावों के स्वामियों का संबंध",
                        ]),
                        weight: 0.6,
                        factors: vec![crate::say3!(
                            lang,
                            "{label} ({a:?}, {b:?}) are {how}",
                            "{label} ({an}, {bn}) {how}",
                            "{label} ({an}, {bn}) {how}"
                        )],
                    },
                });
                break;
            }
        }
    }
    // Viparita Raja yoga: a dusthana lord in another dusthana.
    for h in [6u8, 8, 12] {
        let l = lords[(h - 1) as usize];
        let p = facts.planet(l);
        if is_dusthana(p.rasi_house) && p.rasi_house != h {
            out.push(Yoga {
                name: "Viparita Raja yoga",
                hit: Hit {
                    rule_id: "KS-Y-0040",
                    source: lang.pick([
                        "Uttara Kalamrita / Phaladeepika: lord of 6, 8 or 12 placed in another of them",
                        "ഉത്തരകാലാമൃതം / ഫലദീപിക: 6, 8, 12 ഭാവങ്ങളിൽ ഒന്നിന്റെ അധിപൻ മറ്റൊന്നിൽ",
                        "उत्तर कालामृत / फलदीपिका: 6, 8 या 12 भाव का स्वामी इनमें से किसी दूसरे में",
                    ]),
                    weight: 0.5,
                    factors: vec![{
                        let (rh, n, lo, at) = (p.rasi_house, gn(l), lord_of(lang, h), words::bhava(lang, p.rasi_house));
                        crate::say3!(
                            lang,
                            "{l:?}, lord of {h}, sits in rasi house {rh}",
                            "{lo} {n} {at} ഭാവത്തിൽ നിൽക്കുന്നു",
                            "{lo} {n} {at} भाव में बैठा है"
                        )
                    }],
                },
            });
        }
    }
    // Kemadruma: no graha (Sun and nodes excluded) in the 2nd or 12th from the Moon,
    // cancelled (bhanga) when a planet is in a kendra from the Moon, the Moon is in a
    // kendra from the lagna, or the Moon is conjoined with or aspected by Jupiter.
    let flanked = facts.planets.iter().any(|p| {
        !matches!(p.graha, Sun | Moon | Rahu | Ketu) && {
            let d = (p.sign as i32 - moon.sign as i32).rem_euclid(12);
            d == 1 || d == 11
        }
    });
    if !flanked {
        let kendra_from_moon = facts.planets.iter().any(|p| {
            !matches!(p.graha, Moon | Rahu | Ketu)
                && is_kendra(((p.sign as i32 - moon.sign as i32).rem_euclid(12) + 1) as u8)
        });
        let moon_in_kendra = is_kendra(moon.rasi_house);
        let jupiter_on_moon = same_sign(facts, Moon, Jupiter)
            || aspected_houses(Jupiter, jup.rasi_house).contains(&moon.rasi_house);
        let mut cancels = Vec::new();
        if kendra_from_moon {
            cancels.push(lang.pick([
                "a planet stands in a kendra from the Moon",
                "ചന്ദ്രന്റെ കേന്ദ്രത്തിൽ ഒരു ഗ്രഹം നിൽക്കുന്നു",
                "चंद्र से केंद्र में एक ग्रह स्थित है",
            ]));
        }
        if moon_in_kendra {
            cancels.push(lang.pick([
                "the Moon is in a kendra from the lagna",
                "ചന്ദ്രൻ ലഗ്നകേന്ദ്രത്തിലാണ്",
                "चंद्र लग्न से केंद्र में है",
            ]));
        }
        if jupiter_on_moon {
            cancels.push(lang.pick([
                "Jupiter is with or aspects the Moon",
                "വ്യാഴം ചന്ദ്രനോടു ചേർന്നോ ചന്ദ്രനെ ദൃഷ്ടി ചെയ്തോ നിൽക്കുന്നു",
                "गुरु चंद्र के साथ है या उस पर दृष्टि डालता है",
            ]));
        }
        if cancels.is_empty() {
            out.push(Yoga {
                name: "Kemadruma",
                hit: Hit {
                    rule_id: "KS-Y-0050",
                    source: lang.pick([
                        "BPHS, Chandra yogas: no planet on either side of the Moon and no cancellation",
                        "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ചന്ദ്രയോഗങ്ങൾ: ചന്ദ്രന്റെ ഇരുവശത്തും ഗ്രഹമില്ല, ഭംഗവുമില്ല",
                        "बृहत् पराशर होरा शास्त्र, चंद्र योग: चंद्र के दोनों ओर कोई ग्रह नहीं और कोई भंग नहीं",
                    ]),
                    weight: 0.4,
                    factors: vec![lang
                        .pick([
                            "no planet in the 2nd or 12th sign from the Moon; none of the cancellations applies",
                            "ചന്ദ്രനിൽ നിന്ന് രണ്ടിലും പന്ത്രണ്ടിലും ഗ്രഹമില്ല; ഭംഗകാരണങ്ങൾ ഒന്നും ബാധകമല്ല",
                            "चंद्र से द्वितीय या द्वादश राशि में कोई ग्रह नहीं; कोई भंग लागू नहीं होता",
                        ])
                        .into()],
                },
            });
        } else {
            out.push(Yoga {
                name: "Kemadruma (cancelled)",
                hit: Hit {
                    rule_id: "KS-Y-0051",
                    source: lang.pick([
                        "Phaladeepika 6; Jataka Parijata: Kemadruma bhanga by a planet in a kendra from the Moon, the Moon in a kendra, or Jupiter's association",
                        "ഫലദീപിക 6; ജാതകപാരിജാതം: ചന്ദ്രന്റെ കേന്ദ്രത്തിൽ ഗ്രഹം, കേന്ദ്രത്തിൽ ചന്ദ്രൻ, അല്ലെങ്കിൽ വ്യാഴത്തിന്റെ ബന്ധം കേമദ്രുമത്തെ ഭംഗപ്പെടുത്തുന്നു",
                        "फलदीपिका 6; जातक पारिजात: चंद्र से केंद्र में ग्रह, केंद्र में चंद्र, या गुरु का संबंध केमद्रुम को भंग करता है",
                    ]),
                    weight: 0.1,
                    factors: cancels
                        .iter()
                        .map(|c| {
                            crate::say3!(
                                lang,
                                "would be Kemadruma, but {c}",
                                "കേമദ്രുമം ആകുമായിരുന്നു, പക്ഷേ {c}",
                                "केमद्रुम होता, पर {c}"
                            )
                        })
                        .collect(),
                },
            });
        }
    }
    // Neecha Bhanga: a debilitated planet whose dispositor is in a kendra from the lagna or the Moon.
    for p in &facts.planets {
        if dignity(p.graha, p.sidereal) == Dignity::Debilitated {
            let disp = sign_lord(p.sign as usize);
            let dp = facts.planet(disp);
            let from_moon = ((dp.sign as i32 - moon.sign as i32).rem_euclid(12) + 1) as u8;
            if is_kendra(dp.rasi_house) || is_kendra(from_moon) {
                out.push(Yoga {
                    name: "Neecha Bhanga",
                    hit: Hit {
                        rule_id: "KS-Y-0060",
                        source: lang.pick([
                            "Phaladeepika 7: debilitation cancelled when the dispositor is in a kendra from the lagna or the Moon",
                            "ഫലദീപിക 7: രാശ്യധിപൻ ലഗ്നത്തിന്റെയോ ചന്ദ്രന്റെയോ കേന്ദ്രത്തിലെങ്കിൽ നീചത്വം ഭംഗപ്പെടുന്നു",
                            "फलदीपिका 7: राशि स्वामी लग्न या चंद्र से केंद्र में हो तो नीचता भंग होती है",
                        ]),
                        weight: 0.5,
                        factors: vec![{
                            let (pg, n, dn) = (p.graha, gn(p.graha), gn(disp));
                            crate::say3!(
                                lang,
                                "{pg:?} is debilitated; its dispositor {disp:?} is in a kendra",
                                "{n} നീചത്തിലാണ്; അതിന്റെ രാശ്യധിപൻ {dn} കേന്ദ്രത്തിലാണ്",
                                "{n} नीच का है; उसका राशि स्वामी {dn} केंद्र में है"
                            )
                        }],
                    },
                });
            }
        }
    }
    // Kala Sarpa: all seven grahas on one side of the nodal axis.
    let rahu = facts.planet(Rahu).sidereal;
    let side: Vec<bool> = facts
        .planets
        .iter()
        .filter(|p| !matches!(p.graha, Rahu | Ketu))
        .map(|p| (p.sidereal - rahu).rem_euclid(360.0) < 180.0)
        .collect();
    if side.iter().all(|&b| b) || side.iter().all(|&b| !b) {
        out.push(Yoga {
            name: "Kala Sarpa",
            hit: Hit {
                rule_id: "KS-Y-0070",
                source: lang.pick([
                    "Later texts (not in BPHS): all planets between Rahu and Ketu",
                    "പിൽക്കാല ഗ്രന്ഥങ്ങൾ (ബൃഹത് പരാശര ഹോരാശാസ്ത്രത്തിൽ ഇല്ല): എല്ലാ ഗ്രഹങ്ങളും രാഹുവിനും കേതുവിനും ഇടയിൽ",
                    "बाद के ग्रंथ (बृहत् पराशर होरा शास्त्र में नहीं): सभी ग्रह राहु और केतु के बीच",
                ]),
                weight: 0.4,
                factors: vec![lang
                    .pick([
                        "all seven grahas lie on one side of the Rahu–Ketu axis",
                        "ഏഴു ഗ്രഹങ്ങളും രാഹു-കേതു അക്ഷത്തിന്റെ ഒരു വശത്ത്",
                        "सातों ग्रह राहु-केतु अक्ष के एक ही ओर हैं",
                    ])
                    .into()],
            },
        });
    }
    out
}

/// Parashari assessment of an area.
#[derive(Clone, Debug, PartialEq)]
pub struct Assessment {
    /// The area.
    pub area: Area,
    /// Lord of the primary rasi house.
    pub lord: DashaLord,
    /// Score in `[0, 1]`.
    pub score: f64,
    /// Fired rules with factors.
    pub hits: Vec<Hit>,
}

fn karaka(area: &Area) -> DashaLord {
    match area.name {
        "Career" => DashaLord::Saturn,
        "Marriage" => DashaLord::Venus,
        "Children" => DashaLord::Jupiter,
        "Wealth" => DashaLord::Jupiter,
        "Health" => DashaLord::Sun,
        "Property" => DashaLord::Mars,
        "Education" => DashaLord::Mercury,
        "Foreign" => DashaLord::Rahu,
        _ => DashaLord::Jupiter,
    }
}

/// Assess an area by Parashari principles: the house lord's dignity and placement, the
/// house's occupants and aspects, and the karaka's dignity.
pub fn assess(facts: &Facts, area: Area) -> Assessment {
    assess_in(facts, area, Lang::En)
}

/// [`assess`] with the factors and sources in the reader's language.
pub fn assess_in(facts: &Facts, area: Area, lang: Lang) -> Assessment {
    let h = area.primary;
    let lord = facts.rasi_lords[(h - 1) as usize];
    let lp = facts.planet(lord);
    let mut hits = Vec::new();
    let mut score = 0.0;
    let mut weight = 0.0;
    let gn = |g: DashaLord| words::graha(lang, g);
    let hw = words::bhava(lang, h);
    let ln = gn(lord);

    // 1. Lord's dignity
    let d = dignity(lord, lp.sidereal);
    hits.push(Hit {
        rule_id: "KS-B-0001",
        source: lang.pick([
            "BPHS, Bhava phala: results of a house follow the strength of its lord",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ഭാവഫലം: ഒരു ഭാവത്തിന്റെ ഫലം അതിന്റെ അധിപന്റെ ബലം പിന്തുടരുന്നു",
            "बृहत् पराशर होरा शास्त्र, भाव फल: भाव का फल उसके स्वामी के बल के अनुसार होता है",
        ]),
        weight: d.score(),
        factors: vec![{
            let (sid, lo, dw) = (lp.sidereal, lord_of(lang, h), words::dignity(lang, d));
            crate::say3!(
                lang,
                "{lord:?}, lord of the {h}th, is {d:?} at {sid:.2}°",
                "{lo} {ln} {sid:.2}°-ൽ {dw}",
                "{lo} {ln} {sid:.2}° पर {dw} है"
            )
        }],
    });
    score += d.score() * 1.0;
    weight += 1.0;

    // 2. Lord's placement
    let (ps, pw) = if is_kendra(lp.rasi_house) || is_trikona(lp.rasi_house) {
        (0.85, lang.pick(["a kendra or trikona", "കേന്ദ്രമോ ത്രികോണമോ", "केंद्र या त्रिकोण"]))
    } else if is_dusthana(lp.rasi_house) {
        (0.2, lang.pick(["a dusthana (6, 8 or 12)", "ദുഃസ്ഥാനം (6, 8, 12)", "दुःस्थान (6, 8 या 12)"]))
    } else {
        (0.55, lang.pick(["an upachaya or neutral house", "ഉപചയമോ സമമോ ആയ ഭാവം", "उपचय या सम भाव"]))
    };
    hits.push(Hit {
        rule_id: "KS-B-0002",
        source: lang.pick([
            "BPHS, Bhava phala: the lord in a kendra or trikona protects the house; in 6, 8 or 12 it harms it",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ഭാവഫലം: അധിപൻ കേന്ദ്രത്രികോണങ്ങളിലെങ്കിൽ ഭാവത്തെ രക്ഷിക്കുന്നു; 6, 8, 12-ൽ എങ്കിൽ ദോഷം ചെയ്യുന്നു",
            "बृहत् पराशर होरा शास्त्र, भाव फल: स्वामी केंद्र या त्रिकोण में हो तो भाव की रक्षा करता है; 6, 8 या 12 में हो तो हानि करता है",
        ]),
        weight: ps,
        factors: vec![{
            let (rh, at) = (lp.rasi_house, words::bhava(lang, lp.rasi_house));
            crate::say3!(
                lang,
                "{lord:?} is placed in rasi house {rh} ({pw})",
                "{ln} {at} ഭാവത്തിൽ ({pw})",
                "{ln} {at} भाव में स्थित है ({pw})"
            )
        }],
    });
    score += ps;
    weight += 1.0;

    // 3. Occupants
    let occ = facts.rasi_occupants(h);
    if !occ.is_empty() {
        let ben = occ.iter().filter(|g| is_benefic(facts, **g)).count() as f64;
        let mal = occ.len() as f64 - ben;
        let s = ((ben - mal) / occ.len() as f64 * 0.5 + 0.5).clamp(0.0, 1.0);
        hits.push(Hit {
            rule_id: "KS-B-0003",
            source: lang.pick([
                "BPHS, Bhava phala: benefics in a house support it, malefics afflict it",
                "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ഭാവഫലം: ഭാവത്തിലെ ശുഭന്മാർ അതിനെ തുണയ്ക്കുന്നു, പാപന്മാർ പീഡിപ്പിക്കുന്നു",
                "बृहत् पराशर होरा शास्त्र, भाव फल: भाव में शुभ ग्रह उसे बल देते हैं, पाप ग्रह पीड़ा देते हैं",
            ]),
            weight: s,
            factors: occ
                .iter()
                .map(|g| {
                    let (nw, n) = (nature(lang, is_benefic(facts, *g)), gn(*g));
                    crate::say3!(
                        lang,
                        "{g:?} occupies the {h}th ({nw})",
                        "{n} {hw} ഭാവത്തിൽ ({nw})",
                        "{n} {hw} भाव में ({nw})"
                    )
                })
                .collect(),
        });
        score += s;
        weight += 1.0;
    }

    // 4. Aspects on the house
    let mut asp_factors = Vec::new();
    let mut asp = 0.0;
    for p in &facts.planets {
        if aspected_houses(p.graha, p.rasi_house).contains(&h) {
            let b = is_benefic(facts, p.graha);
            asp += if b { 1.0 } else { -1.0 };
            let (pg, nw, n) = (p.graha, nature(lang, b), gn(p.graha));
            asp_factors.push(crate::say3!(
                lang,
                "{pg:?} aspects the {h}th ({nw})",
                "{n} {hw} ഭാവത്തെ ദൃഷ്ടി ചെയ്യുന്നു ({nw})",
                "{n} की {hw} भाव पर दृष्टि ({nw})"
            ));
        }
    }
    if !asp_factors.is_empty() {
        let s = (asp / asp_factors.len() as f64 * 0.5 + 0.5).clamp(0.0, 1.0);
        hits.push(Hit {
            rule_id: "KS-B-0004",
            source: lang.pick([
                "BPHS, Bhava phala: aspects of benefics strengthen, of malefics weaken",
                "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, ഭാവഫലം: ശുഭദൃഷ്ടി ബലം കൂട്ടുന്നു, പാപദൃഷ്ടി ബലം കുറയ്ക്കുന്നു",
                "बृहत् पराशर होरा शास्त्र, भाव फल: शुभ दृष्टि बल बढ़ाती है, पाप दृष्टि घटाती है",
            ]),
            weight: s,
            factors: asp_factors,
        });
        score += s;
        weight += 1.0;
    }

    // 5. Ashtakavarga: bindus in the primary house (Sarvashtakavarga; 28 is the mean)
    let sarva = sarva_for(facts);
    let house_sign = (facts.lagna_sign as usize + h as usize - 1) % 12;
    let bindus = sarva[house_sign];
    let bs = ((bindus as f64 - 20.0) / 16.0).clamp(0.0, 1.0);
    hits.push(Hit {
        rule_id: "KS-B-0006",
        source: lang.pick([
            "BPHS, Ashtakavargadhyaya: a house with more than 28 bindus in Sarvashtakavarga gives; fewer than 25 withholds",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, അഷ്ടകവർഗാധ്യായം: സർവാഷ്ടകവർഗത്തിൽ 28-ൽ കൂടുതൽ ബിന്ദുക്കളുള്ള ഭാവം ഫലം നൽകുന്നു; 25-ൽ കുറവെങ്കിൽ തടയുന്നു",
            "बृहत् पराशर होरा शास्त्र, अष्टकवर्गाध्याय: सर्वाष्टकवर्ग में 28 से अधिक बिंदु वाला भाव फल देता है; 25 से कम हो तो रोकता है",
        ]),
        weight: bs,
        factors: vec![crate::say3!(
            lang,
            "the {h}th house carries {bindus} bindus in Sarvashtakavarga",
            "സർവാഷ്ടകവർഗത്തിൽ {hw} ഭാവത്തിന് {bindus} ബിന്ദുക്കൾ",
            "सर्वाष्टकवर्ग में {hw} भाव के {bindus} बिंदु हैं"
        )],
    });
    score += bs * 0.8;
    weight += 0.8;

    // 6. Karaka
    let k = karaka(&area);
    let kp = facts.planet(k);
    let kd = dignity(k, kp.sidereal);
    let ks = if is_dusthana(kp.rasi_house) {
        kd.score() * 0.6
    } else {
        kd.score()
    };
    hits.push(Hit {
        rule_id: "KS-B-0005",
        source: lang.pick([
            "BPHS, Karakadhyaya: the natural significator must be sound for the matter to flourish",
            "ബൃഹത് പരാശര ഹോരാശാസ്ത്രം, കാരകാധ്യായം: കാര്യം തഴയ്ക്കാൻ സ്വാഭാവിക കാരകൻ ബലവാനാകണം",
            "बृहत् पराशर होरा शास्त्र, कारकाध्याय: विषय फले, इसके लिए नैसर्गिक कारक सबल होना चाहिए",
        ]),
        weight: ks,
        factors: vec![{
            let (rh, kn, kdw, at) = (kp.rasi_house, gn(k), words::dignity(lang, kd), words::bhava(lang, kp.rasi_house));
            crate::say3!(
                lang,
                "karaka {k:?} is {kd:?} in rasi house {rh}",
                "കാരകൻ {kn} {at} ഭാവത്തിൽ {kdw}",
                "कारक {kn} {at} भाव में {kdw} है"
            )
        }],
    });
    score += ks * 0.8;
    weight += 0.8;

    Assessment {
        area,
        lord,
        score: (score / weight).clamp(0.0, 1.0),
        hits,
    }
}

/// Sarvashtakavarga bindus per sign (0 = Aries) for the chart.
pub fn sarva_for(facts: &Facts) -> [u8; 12] {
    use DashaLord::*;
    let sign = |g: DashaLord| facts.planet(g).sign;
    let p = kshana_vedic::AshtakavargaPositions {
        signs: [
            sign(Sun),
            sign(Moon),
            sign(Mars),
            sign(Mercury),
            sign(Jupiter),
            sign(Venus),
            sign(Saturn),
            facts.lagna_sign,
        ],
    };
    kshana_vedic::sarva(&p)
}

/// Bhinnashtakavarga tables, Sun through Saturn, bindus per sign.
pub fn bhinna_for(facts: &Facts) -> [[u8; 12]; 7] {
    use DashaLord::*;
    let sign = |g: DashaLord| facts.planet(g).sign;
    let p = kshana_vedic::AshtakavargaPositions {
        signs: [
            sign(Sun),
            sign(Moon),
            sign(Mars),
            sign(Mercury),
            sign(Jupiter),
            sign(Venus),
            sign(Saturn),
            facts.lagna_sign,
        ],
    };
    std::array::from_fn(|g| kshana_vedic::bhinna(&p, g))
}

/// Quality of a dasha period lord for an area: a functional benefic (kendra or trikona
/// lord) well placed scores high; a 6/8/12 lord or a debilitated lord scores low.
pub fn period_quality(facts: &Facts, area: Area, lord: DashaLord) -> (f64, Vec<String>) {
    period_quality_in(facts, area, lord, Lang::En)
}

/// [`period_quality`] with the reasons in the reader's language.
pub fn period_quality_in(facts: &Facts, area: Area, lord: DashaLord, lang: Lang) -> (f64, Vec<String>) {
    let p = facts.planet(lord);
    let owns: Vec<u8> = (0..12u8)
        .filter(|i| facts.rasi_lords[*i as usize] == lord)
        .map(|i| i + 1)
        .collect();
    let ln = words::graha(lang, lord);
    let hs = words::houses(&owns);
    let plural = if owns.len() > 1 { "s" } else { "" };
    let mut s = dignity(lord, p.sidereal).score();
    let mut f = vec![{
        let (d, rh) = (dignity(lord, p.sidereal), p.rasi_house);
        let (dw, at) = (words::dignity(lang, d), words::bhava(lang, rh));
        crate::say3!(
            lang,
            "{lord:?} is {d:?} in rasi house {rh}",
            "{ln} {at} ഭാവത്തിൽ {dw}",
            "{ln} {at} भाव में {dw} है"
        )
    }];
    if owns.iter().any(|h| is_kendra(*h) || is_trikona(*h)) {
        s += 0.3;
        f.push(crate::say3!(
            lang,
            "{lord:?} owns a kendra or trikona (house{plural} {hs})",
            "{ln} കേന്ദ്രത്തിന്റെയോ ത്രികോണത്തിന്റെയോ അധിപൻ (ഭാവം {hs})",
            "{ln} केंद्र या त्रिकोण का स्वामी है (भाव {hs})"
        ));
    }
    if owns.iter().any(|h| is_dusthana(*h)) {
        s -= 0.3;
        f.push(crate::say3!(
            lang,
            "{lord:?} owns a dusthana (house{plural} {hs})",
            "{ln} ദുഃസ്ഥാനത്തിന്റെ അധിപൻ (ഭാവം {hs})",
            "{ln} दुःस्थान का स्वामी है (भाव {hs})"
        ));
    }
    if owns.contains(&area.primary) || owns.iter().any(|h| area.group.contains(h)) {
        s += 0.3;
        f.push(crate::say3!(
            lang,
            "{lord:?} owns a house of the matter",
            "{ln} ഈ കാര്യത്തിന്റെ ഒരു ഭാവത്തിന്റെ അധിപൻ",
            "{ln} इस विषय के एक भाव का स्वामी है"
        ));
    }
    if is_dusthana(p.rasi_house) {
        s -= 0.2;
    }
    (s.clamp(0.0, 1.0), f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::AREAS;
    use crate::facts::PlanetFact;
    use kshana_vedic::DashaLord::*;

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

    /// A synthetic chart: Aries lagna, Sun exalted in Aries (1st), Moon in Cancer (4th)
    /// with Jupiter exalted in Cancer, Mars in Capricorn (10th, exalted), Saturn in Libra
    /// (7th, exalted), Venus in Taurus (2nd), Mercury in Gemini (3rd), Rahu in Leo.
    fn synthetic() -> Facts {
        let lagna_sign = 0u8;
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] = std::array::from_fn(|i| (i as f64 * 30.0 + 5.0) % 360.0);
        let p = |g, lon| planet(g, lon, lagna_sign, &rasi_lords);
        let planets = vec![
            p(Sun, 10.0),
            p(Moon, 100.0),
            p(Mars, 298.0),
            p(Mercury, 75.0),
            p(Jupiter, 95.0),
            p(Venus, 40.0),
            p(Saturn, 200.0),
            p(Rahu, 130.0),
            p(Ketu, 310.0),
        ];
        Facts {
            planets,
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
    fn yogas_and_assessments_in_malayalam_and_hindi() {
        // The synthetic chart, then the same planets with Saturn moved round the
        // zodiac, so that many rules fire.
        let mut charts = vec![synthetic()];
        for k in 0..12 {
            let mut f = synthetic();
            let lords = f.rasi_lords;
            f.planets[6] = planet(Saturn, k as f64 * 30.0 + 12.0, 0, &lords);
            f.planets[2] = planet(Mars, (k as f64 * 67.0 + 3.0) % 360.0, 0, &lords);
            charts.push(f);
        }
        for f in &charts {
            assert_eq!(yogas_in(f, Lang::En), yogas(f));
            for lang in [Lang::Ml, Lang::Hi] {
                let mut texts = Vec::new();
                for y in yogas_in(f, lang) {
                    texts.push(yoga_name_in(lang, y.name));
                    texts.push(y.hit.source.to_string());
                    texts.extend(y.hit.factors);
                }
                for a in AREAS {
                    for h in assess_in(f, a, lang).hits {
                        texts.push(h.source.to_string());
                        texts.extend(h.factors);
                    }
                    texts.extend(period_quality_in(f, a, Jupiter, lang).1);
                }
                for s in texts {
                    let latin = s.split(|c: char| !c.is_ascii_alphabetic()).find(|w| w.len() >= 3);
                    assert_eq!(latin, None, "{lang:?}: {s}");
                }
            }
        }
    }

    #[test]
    fn dignities() {
        assert_eq!(dignity(Sun, 10.0), Dignity::Exalted);
        assert_eq!(dignity(Sun, 190.0), Dignity::Debilitated);
        assert_eq!(dignity(Sun, 125.0), Dignity::Moolatrikona);
        assert_eq!(dignity(Sun, 145.0), Dignity::Own);
        assert_eq!(dignity(Saturn, 200.0), Dignity::Exalted);
        assert_eq!(dignity(Jupiter, 95.0), Dignity::Exalted);
        assert_eq!(dignity(Venus, 10.0), Dignity::Neutral); // Venus in Mars's sign: neutral
        assert_eq!(dignity(Venus, 130.0), Dignity::Enemy); // Venus in the Sun's sign
        assert_eq!(dignity(Mercury, 40.0), Dignity::Friend); // Mercury in Venus's sign
    }

    #[test]
    fn aspects_follow_the_special_drishtis() {
        assert_eq!(aspected_houses(Mars, 1), vec![4, 7, 8]);
        assert_eq!(aspected_houses(Jupiter, 10), vec![2, 4, 6]);
        assert_eq!(aspected_houses(Saturn, 12), vec![2, 6, 9]);
        assert_eq!(aspected_houses(Sun, 7), vec![1]);
    }

    #[test]
    fn synthetic_chart_yogas() {
        let f = synthetic();
        let names: Vec<&str> = yogas(&f).iter().map(|y| y.name).collect();
        assert!(names.contains(&"Gajakesari"), "{names:?}"); // Jupiter with the Moon
        assert!(names.contains(&"Hamsa"), "{names:?}"); // Jupiter exalted in the 4th
        assert!(names.contains(&"Ruchaka"), "{names:?}"); // Mars exalted in the 10th
        assert!(names.contains(&"Sasa"), "{names:?}"); // Saturn exalted in the 7th
        assert!(
            !names.iter().any(|n| n.starts_with("Kemadruma")),
            "{names:?}"
        ); // Mercury in Gemini, the 12th from Cancer
           // Raja yoga: Sun (lord of 5, trikona) with ... Mars lord of 1 & 8 ; Jupiter lord of 9 & 12 conjoined with Moon lord of 4 (kendra) → Raja
        assert!(names.contains(&"Raja yoga"), "{names:?}");
    }

    #[test]
    fn assessment_is_bounded_and_explained() {
        let f = synthetic();
        for a in AREAS {
            let s = assess(&f, a);
            assert!((0.0..=1.0).contains(&s.score), "{}", s.score);
            assert!(s.hits.len() >= 4);
            assert!(s.hits.iter().any(|h| h.rule_id == "KS-B-0006"));
            assert!(s.hits.iter().all(|h| !h.factors.is_empty()));
        }
        // Career: 10th lord Saturn exalted in the 7th (kendra) with Mars exalted in the 10th → strong
        let career = assess(&f, AREAS[0]);
        assert!(career.score > 0.7, "{}", career.score);
        let (q, _) = period_quality(&f, AREAS[0], Saturn);
        assert!(q > 0.8);
    }
}
