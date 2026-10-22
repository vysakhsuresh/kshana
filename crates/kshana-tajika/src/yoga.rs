//! Tajika aspects, the applying/separating links between planets, and the yogas built
//! on them.
//!
//! Sources: K. S. Charak, *A Textbook of Varshaphala*, ch. X (Table X-1 orbs, Table X-3
//! the Ithasala kinds and the Ishrafa threshold), following the *Tajika Neelakanthi*;
//! the implementation choices are those recorded in the teistro-sdk Tajika design notes
//! and cross-checked with PyJHora's `tajaka.py`/`tajaka_yoga.py`.
//!
//! * **Aspects are by sign.** Counting from a planet's sign, the 5th and 9th are
//!   friendly aspects, the 3rd and 11th secretly friendly, the 1st (same sign) and 7th
//!   inimical, the 4th and 10th secretly inimical; the 2nd, 6th, 8th and 12th carry no
//!   aspect. All eight aspecting positions count for the yogas.
//! * **Orbs.** The deeptamsa of each planet is in [`deeptamsa`]; a pair's orb is the mean
//!   of the two (Charak, Table X-1: "for the Ithasala the deeptamsa range means a mean").
//! * **Applying and separating** are judged by degrees within the sign, because the
//!   aspect is by sign: the faster planet (fixed ranking Moon, Mercury, Venus, Sun, Mars,
//!   Jupiter, Saturn) applies while it stands at fewer degrees than the slower.
//! * **Ithasala**: mutual aspect, within orb, faster behind: *Vartamana* when a degree or
//!   more behind, *Poorna* when within a degree either side; *Bhavishyat* when the faster
//!   is in the last degree of its sign (29°+) and the slower within the orb of the start
//!   of its own sign, provided the two signs will aspect once the faster moves on.
//! * **Ishrafa**: mutual aspect, within orb, the faster a degree or more past the slower.
//! * **Nakta**: two planets with no mutual aspect, and a third faster than both applying
//!   to each (it carries the light between them).
//! * **Yamaya**: two planets with no mutual aspect, both applying to a third slower than
//!   both.
//! * **Manau**: an Ithasala spoiled by Mars or Saturn standing in an inimical aspect
//!   (same sign, 4th, 7th or 10th) to the faster planet within that planet's own orb.
//! * **Kamboola**: an Ithasala between two planets with the Moon applying to either of
//!   them. The words report the Moon's and the planet's *adhikara* (the office that
//!   grades the Kamboola: Uttama in own sign or exaltation; Madhyama in own Hadda,
//!   drekkana or navamsa; Adhama debilitated or in an enemy's sign; Sama otherwise), from
//!   the *Neelakanthi*, Samjna Tantra vv. 36–46.
//! * **Ikkavala**: all seven planets in kendras or panapharas from the lagna sign;
//!   **Induvara**: all in apoklimas.
//!
//! Not implemented: Gairi-Kamboola, Khallasara, Radda, Duhphali-kuttha,
//! Dutthottha-davira, Tambira, Kuttha and Durapha, which grade an Ithasala by the state
//! (combustion, retrogression, strength) of its planets.

use crate::bala::{relation, Relation};
use crate::sky::AnnualSky;
use crate::tables::{
    deeptamsa, drekkana_lord, graha_in, hadda_lord, navamsa_lord, sign_in, sign_index,
    speed_rank, within_sign, SEVEN,
};
use kshana_rules::{dignity, Dignity, Lang};
use kshana_vedic::{sign_lord, DashaLord};

/// A Tajika aspect, by the sign distance between two planets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drishti {
    /// 5th or 9th sign: open friendship.
    Friendly,
    /// 3rd or 11th sign: secret friendship.
    SecretFriend,
    /// Same sign or 7th: open enmity.
    Inimical,
    /// 4th or 10th sign: secret enmity.
    SecretEnemy,
}

impl Drishti {
    /// Word for the aspect.
    pub fn name(self) -> &'static str {
        match self {
            Drishti::Friendly => "friendly (5/9)",
            Drishti::SecretFriend => "secretly friendly (3/11)",
            Drishti::Inimical => "inimical (1/7)",
            Drishti::SecretEnemy => "secretly inimical (4/10)",
        }
    }

    /// Word for the aspect in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        match self {
            Drishti::Friendly => lang.pick(["friendly (5/9)", "മിത്രദൃഷ്ടി (5/9)", "मित्र दृष्टि (5/9)"]),
            Drishti::SecretFriend => lang.pick([
                "secretly friendly (3/11)",
                "ഗൂഢമിത്രദൃഷ്ടി (3/11)",
                "गुप्त मित्र दृष्टि (3/11)",
            ]),
            Drishti::Inimical => lang.pick(["inimical (1/7)", "ശത്രുദൃഷ്ടി (1/7)", "शत्रु दृष्टि (1/7)"]),
            Drishti::SecretEnemy => lang.pick([
                "secretly inimical (4/10)",
                "ഗൂഢശത്രുദൃഷ്ടി (4/10)",
                "गुप्त शत्रु दृष्टि (4/10)",
            ]),
        }
    }

    /// True for the two friendly kinds.
    pub fn is_friendly(self) -> bool {
        matches!(self, Drishti::Friendly | Drishti::SecretFriend)
    }
}

/// The aspect from one sign to another, `None` for the 2nd, 6th, 8th and 12th.
pub fn drishti(from_sign: usize, to_sign: usize) -> Option<Drishti> {
    match (to_sign % 12 + 12 - from_sign % 12) % 12 + 1 {
        5 | 9 => Some(Drishti::Friendly),
        3 | 11 => Some(Drishti::SecretFriend),
        1 | 7 => Some(Drishti::Inimical),
        4 | 10 => Some(Drishti::SecretEnemy),
        _ => None,
    }
}

/// Whether two signs aspect each other in the Tajika sense.
pub fn aspects_by_sign(a: usize, b: usize) -> bool {
    drishti(a, b).is_some()
}

/// The orb for a pair: the mean of the two deeptamsas; `None` if either is a node.
pub fn pair_orb(a: DashaLord, b: DashaLord) -> Option<f64> {
    Some((deeptamsa(a)? + deeptamsa(b)?) / 2.0)
}

/// How two planets are joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// Applying, a degree or more to go.
    Vartamana,
    /// Applying or just past, within one degree.
    Poorna,
    /// Will apply after the faster planet changes sign.
    Bhavishyat,
    /// Separating by a degree or more.
    Ishrafa,
}

impl LinkKind {
    /// True for the three Ithasala kinds.
    pub fn is_applying(self) -> bool {
        !matches!(self, LinkKind::Ishrafa)
    }

    /// Word for the kind.
    pub fn name(self) -> &'static str {
        match self {
            LinkKind::Vartamana => "Vartamana Ithasala",
            LinkKind::Poorna => "Poorna Ithasala",
            LinkKind::Bhavishyat => "Bhavishyat Ithasala",
            LinkKind::Ishrafa => "Ishrafa",
        }
    }

    /// Word for the kind in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        match self {
            LinkKind::Vartamana => {
                lang.pick(["Vartamana Ithasala", "വർത്തമാന ഇത്ഥശാല", "वर्तमान इत्थशाल"])
            }
            LinkKind::Poorna => lang.pick(["Poorna Ithasala", "പൂർണ ഇത്ഥശാല", "पूर्ण इत्थशाल"]),
            LinkKind::Bhavishyat => {
                lang.pick(["Bhavishyat Ithasala", "ഭവിഷ്യത് ഇത്ഥശാല", "भविष्यत् इत्थशाल"])
            }
            LinkKind::Ishrafa => lang.pick(["Ishrafa", "ഈസറാഫ", "ईसराफ"]),
        }
    }
}

/// A link between two planets within orb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// The faster planet (by the fixed ranking).
    pub fast: DashaLord,
    /// The slower planet.
    pub slow: DashaLord,
    /// What kind of link.
    pub kind: LinkKind,
    /// Degrees the slower stands ahead of the faster, within the sign (negative when the
    /// faster has passed; for Bhavishyat, after the faster's sign change).
    pub apart: f64,
    /// The pair's orb.
    pub orb: f64,
    /// The aspect between the two signs (for Bhavishyat, after the sign change).
    pub drishti: Drishti,
}

/// The link between two of the seven, if they aspect within orb.
pub fn link(sky: &AnnualSky, a: DashaLord, b: DashaLord) -> Option<Link> {
    if a == b {
        return None;
    }
    let (ra, rb) = (speed_rank(a)?, speed_rank(b)?);
    let (fast, slow) = if ra < rb { (a, b) } else { (b, a) };
    let lf = sky.longitude(fast)?;
    let ls = sky.longitude(slow)?;
    let (sf, ss) = (sign_index(lf), sign_index(ls));
    let orb = pair_orb(fast, slow)?;
    let apart = within_sign(ls) - within_sign(lf);
    if let Some(d) = drishti(sf, ss) {
        if apart.abs() <= orb {
            let kind = if apart >= 1.0 {
                LinkKind::Vartamana
            } else if apart > -1.0 {
                LinkKind::Poorna
            } else {
                LinkKind::Ishrafa
            };
            return Some(Link {
                fast,
                slow,
                kind,
                apart,
                orb,
                drishti: d,
            });
        }
    }
    if within_sign(lf) >= 29.0 && within_sign(ls) <= orb {
        if let Some(d) = drishti((sf + 1) % 12, ss) {
            return Some(Link {
                fast,
                slow,
                kind: LinkKind::Bhavishyat,
                apart: apart + 30.0,
                orb,
                drishti: d,
            });
        }
    }
    None
}

/// The yogas this crate detects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YogaKind {
    /// All planets in kendras or panapharas.
    Ikkavala,
    /// All planets in apoklimas.
    Induvara,
    /// An applying aspect within orb.
    Ithasala,
    /// A separating aspect within orb.
    Ishrafa,
    /// Light carried between two unaspecting planets by a faster third.
    Nakta,
    /// Two unaspecting planets both applying to a slower third.
    Yamaya,
    /// An Ithasala spoiled by a malefic on the faster planet.
    Manau,
    /// An Ithasala the Moon joins.
    Kamboola,
}

impl YogaKind {
    /// Name of the yoga.
    pub fn name(self) -> &'static str {
        match self {
            YogaKind::Ikkavala => "Ikkavala",
            YogaKind::Induvara => "Induvara",
            YogaKind::Ithasala => "Ithasala",
            YogaKind::Ishrafa => "Ishrafa",
            YogaKind::Nakta => "Nakta",
            YogaKind::Yamaya => "Yamaya",
            YogaKind::Manau => "Manau",
            YogaKind::Kamboola => "Kamboola",
        }
    }

    /// Name of the yoga in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        match self {
            YogaKind::Ikkavala => lang.pick(["Ikkavala", "ഇക്കവാല", "इक्कबाल"]),
            YogaKind::Induvara => lang.pick(["Induvara", "ഇന്ദുവാര", "इंदुवार"]),
            YogaKind::Ithasala => lang.pick(["Ithasala", "ഇത്ഥശാല", "इत्थशाल"]),
            YogaKind::Ishrafa => lang.pick(["Ishrafa", "ഈസറാഫ", "ईसराफ"]),
            YogaKind::Nakta => lang.pick(["Nakta", "നക്ത", "नक्त"]),
            YogaKind::Yamaya => lang.pick(["Yamaya", "യമയ", "यमया"]),
            YogaKind::Manau => lang.pick(["Manau", "മണഊ", "मणऊ"]),
            YogaKind::Kamboola => lang.pick(["Kamboola", "കംബൂല", "कंबूल"]),
        }
    }
}

/// One detected yoga.
#[derive(Clone, Debug, PartialEq)]
pub struct TajikaYoga {
    /// Which yoga.
    pub kind: YogaKind,
    /// Its name.
    pub name: &'static str,
    /// The planets concerned: the pair first, then any third planet.
    pub planets: Vec<DashaLord>,
    /// True when the decisive link is within one degree (Poorna).
    pub exact: bool,
    /// The configuration in words.
    pub words: String,
}

/// The office of a planet that grades a Kamboola.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adhikara {
    /// In its own sign or exaltation.
    Uttama,
    /// In its own Hadda, drekkana or navamsa.
    Madhyama,
    /// None of the others.
    Sama,
    /// Debilitated, or in an enemy's sign.
    Adhama,
}

impl Adhikara {
    /// Word for the office.
    pub fn name(self) -> &'static str {
        match self {
            Adhikara::Uttama => "Uttama",
            Adhikara::Madhyama => "Madhyama",
            Adhikara::Sama => "Sama",
            Adhikara::Adhama => "Adhama",
        }
    }

    /// Word for the office in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        match self {
            Adhikara::Uttama => lang.pick(["Uttama", "ഉത്തമം", "उत्तम"]),
            Adhikara::Madhyama => lang.pick(["Madhyama", "മധ്യമം", "मध्यम"]),
            Adhikara::Sama => lang.pick(["Sama", "സമം", "सम"]),
            Adhikara::Adhama => lang.pick(["Adhama", "അധമം", "अधम"]),
        }
    }
}

/// Office of one of the seven in the annual sky, `None` for a node. Debilitation is
/// read first, then own sign or exaltation, then an owned division, then an enemy's sign
/// (Tajika relation to the sign lord).
pub fn adhikara(sky: &AnnualSky, g: DashaLord) -> Option<Adhikara> {
    let lon = sky.longitude(g)?;
    let d = dignity(g, lon);
    if d == Dignity::Debilitated {
        return Some(Adhikara::Adhama);
    }
    if matches!(d, Dignity::Exalted | Dignity::Own | Dignity::Moolatrikona) {
        return Some(Adhikara::Uttama);
    }
    if hadda_lord(lon) == g || drekkana_lord(lon) == g || navamsa_lord(lon) == g {
        return Some(Adhikara::Madhyama);
    }
    if relation(sky, g, sign_lord(sign_index(lon))) == Relation::Enemy {
        return Some(Adhikara::Adhama);
    }
    Some(Adhikara::Sama)
}

fn place(sky: &AnnualSky, g: DashaLord, lang: Lang) -> String {
    let l = sky.longitude(g).unwrap_or(0.0);
    format!(
        "{} ({} {:.2}°)",
        graha_in(g, lang),
        sign_in(sign_index(l), lang),
        within_sign(l)
    )
}

fn link_words(sky: &AnnualSky, l: &Link, lang: Lang) -> String {
    let (a, b) = (place(sky, l.fast, lang), place(sky, l.slow, lang));
    let (d, k) = (l.drishti.name_in(lang), l.kind.name_in(lang));
    match (l.kind, lang) {
        (LinkKind::Ishrafa, Lang::En) => format!(
            "{} has passed {} by {:.2}° within the {:.1}° orb, {} aspect: {}",
            a, b, -l.apart, l.orb, d, k
        ),
        (LinkKind::Ishrafa, Lang::Ml) => format!(
            "{} {}-യെ {:.2}° കടന്നുപോയി, {:.1}° ദീപ്താംശത്തിനുള്ളിൽ, {}: {}",
            a, b, -l.apart, l.orb, d, k
        ),
        (LinkKind::Ishrafa, Lang::Hi) => format!(
            "{} {} से {:.2}° आगे निकल चुका है, {:.1}° दीप्तांश के भीतर, {}: {}",
            a, b, -l.apart, l.orb, d, k
        ),
        (LinkKind::Bhavishyat, Lang::En) => format!(
            "{} at the end of its sign will apply to {} {:.2}° on entering the next, {} aspect: {}",
            a, b, l.apart, l.orb, k
        ),
        (LinkKind::Bhavishyat, Lang::Ml) => format!(
            "{} രാശിയുടെ അവസാനത്തിൽ; അടുത്ത രാശിയിൽ കടക്കുമ്പോൾ {}-യോട് {:.2}° അടുക്കും, {:.1}° ദീപ്താംശം: {}",
            a, b, l.apart, l.orb, k
        ),
        (LinkKind::Bhavishyat, Lang::Hi) => format!(
            "{} अपनी राशि के अंत में है; अगली राशि में जाते ही {} से {:.2}° पर जुड़ेगा, {:.1}° दीप्तांश: {}",
            a, b, l.apart, l.orb, k
        ),
        (LinkKind::Poorna, Lang::En) => format!(
            "{} and {} are within a degree ({:+.2}° from the faster to the slower) within the {:.1}° orb, {} aspect: {}",
            a, b, l.apart, l.orb, d, k
        ),
        (LinkKind::Poorna, Lang::Ml) => format!(
            "{}, {} ഒരു ഡിഗ്രിക്കുള്ളിൽ (വേഗമുള്ളതിൽ നിന്ന് മന്ദമായതിലേക്ക് {:+.2}°), {:.1}° ദീപ്താംശത്തിനുള്ളിൽ, {}: {}",
            a, b, l.apart, l.orb, d, k
        ),
        (LinkKind::Poorna, Lang::Hi) => format!(
            "{} और {} एक अंश के भीतर हैं (तेज़ से धीमे तक {:+.2}°), {:.1}° दीप्तांश के भीतर, {}: {}",
            a, b, l.apart, l.orb, d, k
        ),
        (LinkKind::Vartamana, Lang::En) => format!(
            "{} applies to {} {:.2}° behind within the {:.1}° orb, {} aspect: {}",
            a, b, l.apart, l.orb, d, k
        ),
        (LinkKind::Vartamana, Lang::Ml) => format!(
            "{} {:.2}° പിന്നിൽ നിന്ന് {}-യോട് അടുക്കുന്നു, {:.1}° ദീപ്താംശത്തിനുള്ളിൽ, {}: {}",
            a, l.apart, b, l.orb, d, k
        ),
        (LinkKind::Vartamana, Lang::Hi) => format!(
            "{} {:.2}° पीछे से {} की ओर बढ़ रहा है, {:.1}° दीप्तांश के भीतर, {}: {}",
            a, l.apart, b, l.orb, d, k
        ),
    }
}

fn yoga(
    kind: YogaKind,
    planets: Vec<DashaLord>,
    exact: bool,
    words: String,
    lang: Lang,
) -> TajikaYoga {
    TajikaYoga {
        kind,
        name: kind.name_in(lang),
        planets,
        exact,
        words,
    }
}

/// Every yoga in the annual sky, chart-wide ones first, then pair by pair.
pub fn tajika_yogas(sky: &AnnualSky) -> Vec<TajikaYoga> {
    tajika_yogas_in(sky, Lang::En)
}

/// [`tajika_yogas`] with the names and words in the reader's language.
pub fn tajika_yogas_in(sky: &AnnualSky, lang: Lang) -> Vec<TajikaYoga> {
    let graha_name = |g: DashaLord| graha_in(g, lang);
    let place = |sky: &AnnualSky, g: DashaLord| place(sky, g, lang);
    let yoga = |kind, planets, exact, words| yoga(kind, planets, exact, words, lang);
    let mut out = Vec::new();
    let houses: Vec<u8> = SEVEN
        .iter()
        .filter_map(|g| sky.sign(*g))
        .map(|s| sky.sign_house(s))
        .collect();
    if houses.len() == 7 {
        if houses.iter().all(|h| !matches!(h, 3 | 6 | 9 | 12)) {
            out.push(yoga(
                YogaKind::Ikkavala,
                SEVEN.to_vec(),
                true,
                lang.pick([
                    "all seven planets stand in kendras or panapharas from the lagna",
                    "ഏഴു ഗ്രഹങ്ങളും ലഗ്നത്തിൽ നിന്ന് കേന്ദ്രങ്ങളിലോ പണപരങ്ങളിലോ നിൽക്കുന്നു",
                    "सातों ग्रह लग्न से केंद्र या पणफर भावों में हैं",
                ])
                .into(),
            ));
        } else if houses.iter().all(|h| matches!(h, 3 | 6 | 9 | 12)) {
            out.push(yoga(
                YogaKind::Induvara,
                SEVEN.to_vec(),
                true,
                lang.pick([
                    "all seven planets stand in apoklimas from the lagna",
                    "ഏഴു ഗ്രഹങ്ങളും ലഗ്നത്തിൽ നിന്ന് ആപോക്ലിമങ്ങളിൽ നിൽക്കുന്നു",
                    "सातों ग्रह लग्न से आपोक्लिम भावों में हैं",
                ])
                .into(),
            ));
        }
    }

    let mut links = Vec::new();
    for (i, &a) in SEVEN.iter().enumerate() {
        for &b in &SEVEN[i + 1..] {
            if let Some(l) = link(sky, a, b) {
                links.push(l);
            }
        }
    }
    for l in &links {
        let kind = if l.kind.is_applying() {
            YogaKind::Ithasala
        } else {
            YogaKind::Ishrafa
        };
        out.push(yoga(
            kind,
            vec![l.fast, l.slow],
            l.kind == LinkKind::Poorna,
            link_words(sky, l, lang),
        ));
    }

    // Nakta and Yamaya: pairs without a mutual aspect, joined through a third planet.
    for (i, &a) in SEVEN.iter().enumerate() {
        for &b in &SEVEN[i + 1..] {
            let (Some(sa), Some(sb)) = (sky.sign(a), sky.sign(b)) else {
                continue;
            };
            if aspects_by_sign(sa, sb) {
                continue;
            }
            for &c in &SEVEN {
                if c == a || c == b {
                    continue;
                }
                let (Some(x), Some(y)) = (link(sky, c, a), link(sky, c, b)) else {
                    continue;
                };
                if !(x.kind.is_applying() && y.kind.is_applying()) {
                    continue;
                }
                let exact = x.kind == LinkKind::Poorna && y.kind == LinkKind::Poorna;
                if x.fast == c && y.fast == c {
                    out.push(yoga(
                        YogaKind::Nakta,
                        vec![a, b, c],
                        exact,
                        match lang {
                            Lang::En => format!(
                                "{} and {} do not aspect each other; the faster {} applies to both ({:.2}° and {:.2}° behind) and carries the light",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                            Lang::Ml => format!(
                                "{}, {} പരസ്പരം നോക്കുന്നില്ല; വേഗമേറിയ {} രണ്ടിനോടും അടുക്കുന്നു ({:.2}°, {:.2}° പിന്നിൽ), പ്രകാശം കൈമാറുന്നു",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                            Lang::Hi => format!(
                                "{} और {} एक-दूसरे को नहीं देखते; तेज़ {} दोनों की ओर बढ़ता है ({:.2}° और {:.2}° पीछे) और प्रकाश पहुँचाता है",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                        },
                    ));
                } else if x.slow == c && y.slow == c {
                    out.push(yoga(
                        YogaKind::Yamaya,
                        vec![a, b, c],
                        exact,
                        match lang {
                            Lang::En => format!(
                                "{} and {} do not aspect each other; both apply to the slower {} ({:.2}° and {:.2}° behind)",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                            Lang::Ml => format!(
                                "{}, {} പരസ്പരം നോക്കുന്നില്ല; രണ്ടും മന്ദമായ {}-യോട് അടുക്കുന്നു ({:.2}°, {:.2}° പിന്നിൽ)",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                            Lang::Hi => format!(
                                "{} और {} एक-दूसरे को नहीं देखते; दोनों धीमे {} की ओर बढ़ते हैं ({:.2}° और {:.2}° पीछे)",
                                place(sky, a),
                                place(sky, b),
                                graha_name(c),
                                x.apart,
                                y.apart
                            ),
                        },
                    ));
                }
            }
        }
    }

    // Manau: a malefic on the faster planet of an Ithasala.
    for l in links.iter().filter(|l| l.kind.is_applying()) {
        for m in [DashaLord::Mars, DashaLord::Saturn] {
            if m == l.fast || m == l.slow {
                continue;
            }
            let (Some(lm), Some(lf)) = (sky.longitude(m), sky.longitude(l.fast)) else {
                continue;
            };
            let Some(d) = drishti(sign_index(lf), sign_index(lm)) else {
                continue;
            };
            if d.is_friendly() {
                continue;
            }
            let gap = (within_sign(lm) - within_sign(lf)).abs();
            let orb = deeptamsa(l.fast).unwrap_or(0.0);
            if gap <= orb {
                out.push(yoga(
                    YogaKind::Manau,
                    vec![l.fast, l.slow, m],
                    gap <= 1.0,
                    match lang {
                        Lang::En => format!(
                            "the Ithasala of {} and {} is spoiled: {} stands in {} aspect to {} {:.2}° off, within its {:.0}° orb",
                            graha_name(l.fast),
                            graha_name(l.slow),
                            place(sky, m),
                            d.name(),
                            graha_name(l.fast),
                            gap,
                            orb
                        ),
                        Lang::Ml => format!(
                            "{}, {} ഇവരുടെ ഇത്ഥശാല കേടാകുന്നു: {} {}-നെ {} ആയി നോക്കുന്നു, {:.2}° അകലെ, അതിന്റെ {:.0}° ദീപ്താംശത്തിനുള്ളിൽ",
                            graha_name(l.fast),
                            graha_name(l.slow),
                            place(sky, m),
                            graha_name(l.fast),
                            d.name_in(lang),
                            gap,
                            orb
                        ),
                        Lang::Hi => format!(
                            "{} और {} का इत्थशाल बिगड़ता है: {} {} को {} से देखता है, {:.2}° दूर, अपने {:.0}° दीप्तांश के भीतर",
                            graha_name(l.fast),
                            graha_name(l.slow),
                            place(sky, m),
                            graha_name(l.fast),
                            d.name_in(lang),
                            gap,
                            orb
                        ),
                    },
                ));
            }
        }
    }

    // Kamboola: the Moon applying to either planet of an Ithasala.
    for l in links
        .iter()
        .filter(|l| l.kind.is_applying() && l.fast != DashaLord::Moon && l.slow != DashaLord::Moon)
    {
        for p in [l.fast, l.slow] {
            let Some(ml) = link(sky, DashaLord::Moon, p) else {
                continue;
            };
            if !ml.kind.is_applying() {
                continue;
            }
            let moon_office = adhikara(sky, DashaLord::Moon).map_or("-", |a| a.name_in(lang));
            let p_office = adhikara(sky, p).map_or("-", |a| a.name_in(lang));
            let moon_sign = sign_in(sky.sign(DashaLord::Moon).unwrap_or(0), lang);
            let p_sign = sign_in(sky.sign(p).unwrap_or(0), lang);
            let moon = graha_name(DashaLord::Moon);
            out.push(yoga(
                YogaKind::Kamboola,
                vec![l.fast, l.slow, DashaLord::Moon],
                ml.kind == LinkKind::Poorna,
                match lang {
                    Lang::En => format!(
                        "the Moon ({}) applies to {} ({}) {:.2}° behind and joins the Ithasala of {} and {}; Moon {}, {} {}",
                        moon_sign,
                        graha_name(p),
                        p_sign,
                        ml.apart,
                        graha_name(l.fast),
                        graha_name(l.slow),
                        moon_office,
                        graha_name(p),
                        p_office
                    ),
                    Lang::Ml => format!(
                        "{} ({}) {:.2}° പിന്നിൽ നിന്ന് {} ({})-യോട് അടുക്കുന്നു, {}, {} ഇവരുടെ ഇത്ഥശാലയിൽ ചേരുന്നു; {} {}, {} {}",
                        moon,
                        moon_sign,
                        ml.apart,
                        graha_name(p),
                        p_sign,
                        graha_name(l.fast),
                        graha_name(l.slow),
                        moon,
                        moon_office,
                        graha_name(p),
                        p_office
                    ),
                    Lang::Hi => format!(
                        "{} ({}) {:.2}° पीछे से {} ({}) की ओर बढ़ता है और {} व {} के इत्थशाल से जुड़ता है; {} {}, {} {}",
                        moon,
                        moon_sign,
                        ml.apart,
                        graha_name(p),
                        p_sign,
                        graha_name(l.fast),
                        graha_name(l.slow),
                        moon,
                        moon_office,
                        graha_name(p),
                        p_office
                    ),
                },
            ));
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // SEVEN order: Moon, Mercury, Venus, Sun, Mars, Jupiter, Saturn
    fn sky(longitudes: [f64; 7]) -> AnnualSky {
        AnnualSky {
            lagna: 0.0,
            cusps: std::array::from_fn(|i| 30.0 * i as f64),
            longitudes,
            speeds: [0.0; 7],
        }
    }

    fn find<'a>(
        ys: &'a [TajikaYoga],
        kind: YogaKind,
        planets: &[DashaLord],
    ) -> Option<&'a TajikaYoga> {
        ys.iter().find(|y| y.kind == kind && y.planets == planets)
    }

    #[test]
    fn aspects_by_sign_distance() {
        assert_eq!(drishti(0, 4), Some(Drishti::Friendly));
        assert_eq!(drishti(0, 8), Some(Drishti::Friendly));
        assert_eq!(drishti(0, 2), Some(Drishti::SecretFriend));
        assert_eq!(drishti(0, 10), Some(Drishti::SecretFriend));
        assert_eq!(drishti(0, 0), Some(Drishti::Inimical));
        assert_eq!(drishti(0, 6), Some(Drishti::Inimical));
        assert_eq!(drishti(0, 3), Some(Drishti::SecretEnemy));
        assert_eq!(drishti(0, 9), Some(Drishti::SecretEnemy));
        for d in [1, 5, 7, 11] {
            assert_eq!(drishti(0, d), None);
        }
        assert_eq!(pair_orb(DashaLord::Sun, DashaLord::Moon), Some(13.5));
        assert_eq!(pair_orb(DashaLord::Sun, DashaLord::Rahu), None);
    }

    #[test]
    fn ithasala_kinds_and_ishrafa() {
        // Moon Aries 5°, Jupiter Leo 8°: trine, Moon 3° behind → Vartamana
        let s = sky([5.0, 200.0, 230.0, 260.0, 290.0, 128.0, 320.0]);
        let l = link(&s, DashaLord::Jupiter, DashaLord::Moon).unwrap();
        assert_eq!(
            (l.fast, l.slow, l.kind),
            (DashaLord::Moon, DashaLord::Jupiter, LinkKind::Vartamana)
        );
        assert_eq!(l.apart, 3.0);
        assert_eq!(l.orb, 10.5);
        let ys = tajika_yogas(&s);
        let y = find(
            &ys,
            YogaKind::Ithasala,
            &[DashaLord::Moon, DashaLord::Jupiter],
        )
        .unwrap();
        assert!(!y.exact);
        // Moon Aries 5.5°, Jupiter Leo 5°: within a degree → Poorna, exact
        let s = sky([5.5, 200.0, 230.0, 260.0, 290.0, 125.0, 320.0]);
        assert_eq!(
            link(&s, DashaLord::Moon, DashaLord::Jupiter).unwrap().kind,
            LinkKind::Poorna
        );
        assert!(
            find(
                &tajika_yogas(&s),
                YogaKind::Ithasala,
                &[DashaLord::Moon, DashaLord::Jupiter]
            )
            .unwrap()
            .exact
        );
        // Moon Aries 10°, Jupiter Leo 5°: Moon 5° past → Ishrafa
        let s = sky([10.0, 200.0, 230.0, 260.0, 290.0, 125.0, 320.0]);
        assert_eq!(
            link(&s, DashaLord::Moon, DashaLord::Jupiter).unwrap().kind,
            LinkKind::Ishrafa
        );
        assert!(find(
            &tajika_yogas(&s),
            YogaKind::Ishrafa,
            &[DashaLord::Moon, DashaLord::Jupiter]
        )
        .is_some());
        // Moon Aries 29.5°, Jupiter Virgo 3°: no aspect now (6th), trine after the Moon enters
        // Taurus → Bhavishyat
        let s = sky([29.5, 200.0, 230.0, 260.0, 290.0, 153.0, 320.0]);
        let l = link(&s, DashaLord::Moon, DashaLord::Jupiter).unwrap();
        assert_eq!(l.kind, LinkKind::Bhavishyat);
        assert!((l.apart - 3.5).abs() < 1e-9);
        // Moon Aries 10°, Jupiter Leo 25°: beyond the 10.5° orb → nothing
        let s = sky([10.0, 200.0, 230.0, 260.0, 290.0, 145.0, 320.0]);
        assert!(link(&s, DashaLord::Moon, DashaLord::Jupiter).is_none());
        // Moon Aries 5°, Jupiter Virgo 8°: 6th, no aspect
        let s = sky([5.0, 200.0, 230.0, 260.0, 290.0, 158.0, 320.0]);
        assert!(link(&s, DashaLord::Moon, DashaLord::Jupiter).is_none());
    }

    #[test]
    fn nakta_yamaya_manau_kamboola() {
        // Nakta: Sun Aries 10°, Saturn Virgo 12° (no aspect); Moon Gemini 4° applies to both
        let s = sky([64.0, 200.0, 230.0, 10.0, 290.0, 320.0, 162.0]);
        let ys = tajika_yogas(&s);
        assert!(find(
            &ys,
            YogaKind::Nakta,
            &[DashaLord::Sun, DashaLord::Saturn, DashaLord::Moon]
        )
        .is_some());
        // Yamaya: Moon Aries 3°, Mercury Virgo 5° (no aspect); both apply to Saturn Sagittarius 9°
        let s = sky([3.0, 155.0, 230.0, 200.0, 290.0, 320.0, 249.0]);
        let ys = tajika_yogas(&s);
        assert!(find(
            &ys,
            YogaKind::Yamaya,
            &[DashaLord::Moon, DashaLord::Mercury, DashaLord::Saturn]
        )
        .is_some());
        // Manau: Moon Aries 5° → Jupiter Leo 8° Ithasala; Mars Libra 7° opposes the Moon
        let s = sky([5.0, 200.0, 230.0, 260.0, 187.0, 128.0, 320.0]);
        let ys = tajika_yogas(&s);
        assert!(find(
            &ys,
            YogaKind::Manau,
            &[DashaLord::Moon, DashaLord::Jupiter, DashaLord::Mars]
        )
        .is_some());
        // Kamboola: Mercury Aries 5° → Jupiter Leo 8°; Moon Sagittarius 2° applies to Mercury
        let s = sky([242.0, 5.0, 230.0, 260.0, 290.0, 128.0, 320.0]);
        let ys = tajika_yogas(&s);
        let k = find(
            &ys,
            YogaKind::Kamboola,
            &[DashaLord::Mercury, DashaLord::Jupiter, DashaLord::Moon],
        )
        .unwrap();
        assert!(k.words.contains("Moon"));
    }

    #[test]
    fn ikkavala_and_induvara() {
        // all in kendras/panapharas from Aries lagna: signs 0,1,3,4,6,7,9,10
        let s = sky([5.0, 35.0, 95.0, 125.0, 185.0, 215.0, 275.0]);
        assert!(find(&tajika_yogas(&s), YogaKind::Ikkavala, &SEVEN).is_some());
        // all in apoklimas: signs 2,5,8,11
        let s = sky([65.0, 155.0, 245.0, 335.0, 65.0, 155.0, 245.0]);
        assert!(find(&tajika_yogas(&s), YogaKind::Induvara, &SEVEN).is_some());
    }
}
