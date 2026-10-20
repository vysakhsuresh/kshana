//! Dosha detectors: the curses of the previous birth (BPHS ch. 85), the badhaka of
//! Prasna Marga ch. 15, and the yoga and transit families (Kala Sarpa, Kemadruma,
//! Guru Chandala, Grahan, Mangal dosha, Sade Sati). Each detector returns a [`Dosha`]
//! whether or not it is present, with the indicators that fired, its source and the
//! remedy family it belongs to, so the remedy plan can tie every remedy to a located
//! cause.
//!
//! Structure of the curse yogas (BPHS ch. 85, Sharma numbering; Santhanam chs. 83–85
//! depending on the print): each curse is keyed to the natural karaka of the offended
//! party afflicting the 5th house or its lord: Rahu the serpents, the Sun the father,
//! the Moon the mother, Mars the brother, Mercury the maternal uncle, Jupiter the
//! Brahmin, Venus the wife, Saturn and Ketu the departed. The individual yogas are
//! transcribed from the research brief's reading of the Santhanam text and marked
//! "verify against Santhanam vol. 2" in the rule text; the Prasna Marga parent
//! indicators are snippet-verified. Houses are rasi houses (whole sign from the lagna).

use crate::facts::Facts;
use crate::kuta::mangal_dosha;
use crate::parashari::{aspected_houses, dignity, is_benefic, yogas, Dignity};
use kshana_time::Instant;
use kshana_vedic::{sign_lord, DashaLord};

/// One dosha, present or not.
#[derive(Clone, Debug, PartialEq)]
pub struct Dosha {
    /// Name, e.g. "Pitru dosha".
    pub name: &'static str,
    /// Stable rule id.
    pub rule_id: &'static str,
    /// The graha whose remedy family applies (the karaka, or the badhakesh).
    pub planet: DashaLord,
    /// Present in the chart.
    pub present: bool,
    /// Severity in `[0, 1]`: 0 when absent.
    pub severity: f64,
    /// The indicators that fired, in words.
    pub factors: Vec<String>,
    /// The text.
    pub source: &'static str,
    /// The remedy family, in words.
    pub remedy_family: &'static str,
    /// Whom the text says is offended or what the family concerns.
    pub offended: String,
}

fn build(
    name: &'static str,
    rule_id: &'static str,
    planet: DashaLord,
    source: &'static str,
    remedy_family: &'static str,
    offended: String,
    fired: Vec<(f64, String)>,
) -> Dosha {
    let severity: f64 = fired.iter().map(|(w, _)| w).sum::<f64>().clamp(0.0, 1.0);
    Dosha {
        name,
        rule_id,
        planet,
        present: !fired.is_empty(),
        severity,
        factors: fired.into_iter().map(|(_, f)| f).collect(),
        source,
        remedy_family,
        offended,
    }
}

fn lord_of(facts: &Facts, house: u8) -> DashaLord {
    facts.rasi_lords[(house - 1) as usize]
}

fn house_of(facts: &Facts, g: DashaLord) -> u8 {
    facts.planet(g).rasi_house
}

fn is_dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}

/// Natural malefics conjoined (same sign) with `g`, other than `g` itself.
fn malefics_with(facts: &Facts, g: DashaLord) -> Vec<DashaLord> {
    let s = facts.planet(g).sign;
    facts
        .planets
        .iter()
        .filter(|p| p.graha != g && p.sign == s && !is_benefic(facts, p.graha))
        .map(|p| p.graha)
        .collect()
}

fn conj(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    a != b && facts.planet(a).sign == facts.planet(b).sign
}

fn aspects(facts: &Facts, a: DashaLord, b: DashaLord) -> bool {
    a != b && aspected_houses(a, house_of(facts, a)).contains(&house_of(facts, b))
}

/// Natural malefics aspecting `g` by graha drishti, other than `g`.
fn malefic_aspects_on(facts: &Facts, g: DashaLord) -> Vec<DashaLord> {
    facts
        .planets
        .iter()
        .filter(|p| p.graha != g && !is_benefic(facts, p.graha) && aspects(facts, p.graha, g))
        .map(|p| p.graha)
        .collect()
}

/// Afflicted: with a malefic, aspected by one, or in a dusthana.
fn afflicted(facts: &Facts, g: DashaLord) -> Option<String> {
    let w = malefics_with(facts, g);
    if !w.is_empty() {
        return Some(format!("{g:?} is with {w:?}"));
    }
    let a = malefic_aspects_on(facts, g);
    if !a.is_empty() {
        return Some(format!("{g:?} is aspected by {a:?}"));
    }
    let h = house_of(facts, g);
    if is_dusthana(h) {
        return Some(format!("{g:?} is in rasi house {h}, a dusthana"));
    }
    None
}

const BPHS_85: &str = "BPHS ch. 85 (Sharma; Santhanam chs. 83–85), Curses of the Previous Birth: the karaka of the offended party afflicting the 5th house or its lord; stanzas as read in the research brief, verify against Santhanam vol. 2";

/// Pitru dosha: the father's and ancestors' curse (the Sun afflicting the 5th), with
/// the Prasna Marga father indicators and the later generalisation marked as such.
pub fn pitru_dosha(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let sun_h = house_of(facts, Sun);
    let l5 = lord_of(facts, 5);
    let l9 = lord_of(facts, 9);
    let l6 = lord_of(facts, 6);
    let l1 = lord_of(facts, 1);
    let occ5 = facts.rasi_occupants(5);
    if sun_h == 5 {
        let m = malefics_with(facts, Sun);
        if !m.is_empty() {
            f.push((0.4, format!("the Sun is in the 5th with {m:?}")));
        }
        if occ5.contains(&Saturn) && occ5.contains(&Mars) {
            f.push((0.5, "Sun, Saturn and Mars together in the 5th".into()));
        }
        if dignity(Sun, facts.planet(Sun).sidereal) == Dignity::Debilitated {
            let a = malefic_aspects_on(facts, Sun);
            if !a.is_empty() {
                f.push((
                    0.4,
                    format!("the Sun is debilitated in the 5th and aspected by {a:?}"),
                ));
            }
        }
    }
    if l5 != Sun && conj(facts, l5, Sun) {
        let m: Vec<DashaLord> = malefics_with(facts, l5)
            .into_iter()
            .filter(|g| *g != Sun)
            .collect();
        if !m.is_empty() {
            f.push((
                0.4,
                format!("{l5:?}, lord of the 5th, is with the Sun and {m:?}"),
            ));
        }
    }
    if house_of(facts, l9) == 5 && (conj(facts, l9, Saturn) || conj(facts, l9, Rahu)) {
        f.push((
            0.35,
            format!("{l9:?}, lord of the 9th, is in the 5th with Saturn or Rahu"),
        ));
    }
    if house_of(facts, l1) == 5 && is_dusthana(house_of(facts, l5)) && conj(facts, l5, Sun) {
        f.push((0.3, format!("the lagna lord {l1:?} is in the 5th while {l5:?}, lord of the 5th, is in a dusthana with the Sun")));
    }
    if house_of(facts, l6) == 9 {
        f.push((
            0.2,
            format!(
                "{l6:?}, lord of the 6th, is in the 9th (Prasna Marga: the father's displeasure)"
            ),
        ));
    }
    if house_of(facts, l9) == 12 {
        f.push((
            0.2,
            format!(
                "{l9:?}, lord of the 9th, is in the 12th (Prasna Marga: the father's displeasure)"
            ),
        ));
    }
    if sun_h == 6 {
        f.push((
            0.2,
            "the Sun is in the 6th (Prasna Marga: the father's displeasure)".into(),
        ));
    }
    let nodes_sat: Vec<DashaLord> = malefics_with(facts, Sun)
        .into_iter()
        .filter(|g| matches!(g, Rahu | Ketu | Saturn))
        .collect();
    if !nodes_sat.is_empty() && sun_h != 5 {
        f.push((0.15, format!("the Sun is with {nodes_sat:?} (the broad modern 'pitra dosha', a later generalisation, not BPHS)")));
    }
    let occ9: Vec<DashaLord> = facts
        .rasi_occupants(9)
        .into_iter()
        .filter(|g| matches!(g, Rahu | Ketu | Saturn))
        .collect();
    if !occ9.is_empty() {
        f.push((0.15, format!("{occ9:?} in the 9th house of the father (the broad modern reading, a later generalisation)")));
    }
    build(
        "Pitru dosha",
        "KS-R-SHAPA-01",
        DashaLord::Sun,
        "BPHS ch. 85: the father's curse (Sun afflicting the 5th); Prasna Marga ch. 15: 6th lord in 9th, 9th lord in 12th, Sun in 6th; remedies named: shraddha at Gaya, feeding Brahmins, kanyadana, gift of a cow",
        "Sun: a duty to ancestors left undone; conduct toward elders, the death tithis kept, tarpana on Amavasya, Mahalaya in Pitru Paksha, Sunday in the Sun's hora",
        "the father and the pitris".into(),
        f,
    )
}

/// Sarpa shapa / Naga dosha: the serpent curse (Rahu, with Mars, on the 5th), with the
/// Prasna Marga serpent indicators.
pub fn sarpa_dosha(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let rahu_h = house_of(facts, Rahu);
    let mars_h = house_of(facts, Mars);
    let l5 = lord_of(facts, 5);
    if rahu_h == 5 {
        if conj(facts, Rahu, Mars) || aspects(facts, Mars, Rahu) {
            f.push((0.5, "Rahu in the 5th with or aspected by Mars".into()));
        }
        if matches!(facts.planet(Rahu).sign, 0 | 7) {
            f.push((0.4, "Rahu in a sign of Mars in the 5th".into()));
        }
    }
    if l5 != Rahu && conj(facts, l5, Rahu) && conj(facts, l5, Mars) {
        f.push((
            0.4,
            format!("{l5:?}, lord of the 5th, is with Rahu and Mars"),
        ));
    }
    if mars_h == 5 && aspects(facts, Rahu, Mars) && rahu_h != 5 {
        f.push((0.35, "Mars in the 5th aspected by Rahu".into()));
    }
    if matches!(rahu_h, 1 | 5) && is_dusthana(house_of(facts, l5)) {
        f.push((
            0.3,
            format!("Rahu in house {rahu_h} while {l5:?}, lord of the 5th, is in a dusthana"),
        ));
    }
    let b = badhaka(facts);
    if rahu_h == b.house || conj(facts, Rahu, b.lord) {
        f.push((0.3, format!("Rahu is in the badhaka house ({}) or with the badhakesh {:?} (Prasna Marga: a serpent deity)", b.house, b.lord)));
    }
    if conj(facts, Moon, Rahu) {
        f.push((
            0.2,
            "the Moon is with Rahu (Prasna Marga: malicious serpents)".into(),
        ));
    }
    if conj(facts, Sun, Rahu) {
        f.push((
            0.1,
            "the Sun is with Rahu (Prasna Marga: benevolent serpents, long-term good)".into(),
        ));
    }
    build(
        "Sarpa dosha",
        "KS-R-SHAPA-02",
        Rahu,
        "BPHS ch. 85: the serpent curse (Rahu, with Mars, afflicting the 5th); Prasna Marga ch. 15: Rahu or Gulika with the badhakesh, Moon with Rahu; remedies named: Naga pratishtha, Sarpa shanti, gift of a cow and gold",
        "Rahu: a debt to serpents and the land; guard a living thing or a plot, end a deception; Panchami, the Moon in Ashlesha, Rahu kala, Saturday",
        "the serpent deities".into(),
        f,
    )
}

/// Matru shapa: the mother's curse (the Moon afflicted in or ruling the 5th), with the
/// Prasna Marga mother indicators.
pub fn matru_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let moon_h = house_of(facts, Moon);
    let l4 = lord_of(facts, 4);
    let l5 = lord_of(facts, 5);
    let l6 = lord_of(facts, 6);
    if matches!(moon_h, 4 | 5) && (conj(facts, Moon, Saturn) || conj(facts, Moon, Rahu)) {
        f.push((
            0.4,
            format!("the Moon is in house {moon_h} with Saturn or Rahu"),
        ));
    }
    if is_dusthana(moon_h) && is_dusthana(house_of(facts, l4)) {
        f.push((
            0.35,
            format!("the Moon and {l4:?}, lord of the 4th, are both in dusthanas"),
        ));
    }
    if moon_h == 5 {
        let m = malefics_with(facts, Moon);
        if !m.is_empty() {
            f.push((0.3, format!("the Moon is in the 5th with {m:?}")));
        }
    }
    if l5 == Moon {
        if let Some(a) = afflicted(facts, Moon) {
            f.push((0.3, format!("the Moon rules the 5th and {a}")));
        }
    }
    if house_of(facts, l6) == 4 {
        f.push((
            0.2,
            format!(
                "{l6:?}, lord of the 6th, is in the 4th (Prasna Marga: the mother's displeasure)"
            ),
        ));
    }
    if house_of(facts, l4) == 12 {
        f.push((
            0.2,
            format!(
                "{l4:?}, lord of the 4th, is in the 12th (Prasna Marga: the mother's displeasure)"
            ),
        ));
    }
    if moon_h == 6 {
        f.push((
            0.2,
            "the Moon is in the 6th (Prasna Marga: the mother's displeasure)".into(),
        ));
    }
    build(
        "Matru shapa",
        "KS-R-SHAPA-03",
        Moon,
        BPHS_85,
        "Moon: tend the mother in person, fixed sleep; Gayatri japa and Peepal worship; Monday in the Moon's hora, Purnima",
        "the mother".into(),
        f,
    )
}

/// Bhratru shapa: the brother's curse (Mars afflicting the 5th; the 3rd lord afflicted).
pub fn bhratru_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l3 = lord_of(facts, 3);
    if house_of(facts, Mars) == 5 {
        let m = malefics_with(facts, Mars);
        if !m.is_empty() {
            f.push((0.4, format!("Mars is in the 5th with {m:?}")));
        }
    }
    if is_dusthana(house_of(facts, l3)) && (conj(facts, l3, Mars) || conj(facts, l3, Rahu)) {
        f.push((
            0.35,
            format!("{l3:?}, lord of the 3rd, is in a dusthana with Mars or Rahu"),
        ));
    }
    build(
        "Bhratru shapa",
        "KS-R-SHAPA-04",
        Mars,
        BPHS_85,
        "Mars: end a quarrel with a sibling; Gayatri japa with homa, Harivamsha shravana; Tuesday in Mars's hora",
        "the brother".into(),
        f,
    )
}

/// Matula shapa: the maternal uncle's curse (Mercury afflicting the 5th; the 6th lord
/// and Mercury afflicted).
pub fn matula_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l6 = lord_of(facts, 6);
    if house_of(facts, Mercury) == 5 {
        let m = malefics_with(facts, Mercury);
        if !m.is_empty() {
            f.push((0.4, format!("Mercury is in the 5th with {m:?}")));
        }
    }
    if let (Some(a), Some(b)) = (afflicted(facts, l6), afflicted(facts, Mercury)) {
        if l6 != Mercury {
            f.push((0.3, format!("{a} (lord of the 6th) and {b}")));
        }
    }
    build(
        "Matula shapa",
        "KS-R-SHAPA-05",
        Mercury,
        BPHS_85,
        "Mercury: restore a cut family tie, truthful speech; Vishnu pratishtha (ritual, optional); Wednesday in Mercury's hora",
        "the maternal uncle".into(),
        f,
    )
}

/// Brahma shapa: the Brahmin's curse (Jupiter afflicting the 5th; Rahu in Jupiter's
/// sign in the 5th; the 9th lord and Jupiter afflicted).
pub fn brahma_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l9 = lord_of(facts, 9);
    if house_of(facts, Jupiter) == 5 {
        let m = malefics_with(facts, Jupiter);
        if !m.is_empty() {
            f.push((0.4, format!("Jupiter is in the 5th with {m:?}")));
        }
    }
    if house_of(facts, Rahu) == 5 && matches!(facts.planet(Rahu).sign, 8 | 11) {
        f.push((0.4, "Rahu in a sign of Jupiter in the 5th".into()));
    }
    if let (Some(a), Some(b)) = (afflicted(facts, l9), afflicted(facts, Jupiter)) {
        if l9 != Jupiter {
            f.push((0.3, format!("{a} (lord of the 9th) and {b}")));
        }
    }
    build(
        "Brahma shapa",
        "KS-R-SHAPA-06",
        Jupiter,
        "BPHS ch. 85: the Brahmin's curse; remedy named: Chandrayana vrata and service to a cow (citing Parashara)",
        "Jupiter: honour a teacher, return what was taken from a learned person; Chandrayana (a lunar-month vow whose daily dose follows the tithi); Thursday in Jupiter's hora",
        "a Brahmin, the preceptor".into(),
        f,
    )
}

/// Patni shapa: the wife's curse (Venus afflicted in the 5th or 7th; the 7th lord in a
/// dusthana with Venus).
pub fn patni_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l7 = lord_of(facts, 7);
    let vh = house_of(facts, Venus);
    if matches!(vh, 5 | 7) {
        let m = malefics_with(facts, Venus);
        if !m.is_empty() {
            f.push((0.4, format!("Venus is in house {vh} with {m:?}")));
        }
    }
    if l7 != Venus && is_dusthana(house_of(facts, l7)) && conj(facts, l7, Venus) {
        f.push((
            0.35,
            format!("{l7:?}, lord of the 7th, is in a dusthana with Venus"),
        ));
    }
    build(
        "Patni shapa",
        "KS-R-SHAPA-07",
        Venus,
        BPHS_85,
        "Venus: keep a promise to the spouse, no deception in love; Lakshmi-Narayana pratishtha, kanyadana, a cow (ritual, optional); Friday in Venus's hora",
        "the wife".into(),
        f,
    )
}

/// Preta shapa: the curse of the departed (Saturn or Ketu with the 5th lord; Sun and
/// Saturn in the 5th with Ketu).
pub fn preta_shapa(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let l5 = lord_of(facts, 5);
    if l5 != Saturn && conj(facts, l5, Saturn) {
        f.push((0.35, format!("{l5:?}, lord of the 5th, is with Saturn")));
    }
    if conj(facts, l5, Ketu) {
        f.push((0.35, format!("{l5:?}, lord of the 5th, is with Ketu")));
    }
    let occ5 = facts.rasi_occupants(5);
    if occ5.contains(&Sun) && occ5.contains(&Saturn) && occ5.contains(&Ketu) {
        f.push((0.5, "Sun and Saturn in the 5th with Ketu".into()));
    }
    build(
        "Preta shapa",
        "KS-R-SHAPA-08",
        Saturn,
        "BPHS ch. 85: the curse of departed spirits; remedies named: Gaya shraddha, Rudra japa, gift of a cow or silver",
        "Saturn and Ketu: complete the rites or the memory of a dead relative, visit a grave; tarpana on Amavasya; Saturday in Saturn's hora",
        "the departed (pretas)".into(),
        f,
    )
}

/// The badhaka reading of Prasna Marga ch. 15.
#[derive(Clone, Debug, PartialEq)]
pub struct Badhaka {
    /// "movable", "fixed" or "dual".
    pub lagna_type: &'static str,
    /// Badhaka house: movable 11, fixed 9, dual 7.
    pub house: u8,
    /// Its lord, the badhakesh.
    pub lord: DashaLord,
    /// Each companion (in the badhaka house, with the badhakesh, or aspecting it), the
    /// being it names, and how it is joined.
    pub companions: Vec<(DashaLord, &'static str, String)>,
    /// The offence read from the house, when the text gives one.
    pub offence: Option<String>,
    /// The kind of remedy from the tattva of the badhakesh's dispositor.
    pub remedy_kind: &'static str,
    /// Parent-type and serpent indicators that fired.
    pub parent_rules: Vec<String>,
}

/// The being a companion of the badhakesh names (V = snippet-verified from Prasna
/// Marga ch. 15; K = Kerala customary reading).
pub fn offended_being(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Shiva, or a temple deity neglected (V)",
        DashaLord::Moon => "Durga / Devi (V)",
        DashaLord::Mars => "Subrahmanya (Karthikeya) or Bhadrakali; bad management of a shrine (V)",
        DashaLord::Rahu => "a serpent deity (V)",
        DashaLord::Saturn => "impurity (ashaucha), pretas and pitris (V)",
        DashaLord::Mercury => "Vishnu (K)",
        DashaLord::Jupiter => "a Brahmin's curse, the preceptor, Ganapati as Vighnesha (K)",
        DashaLord::Venus => "Yakshi / Lakshmi, a women's or household deity (K)",
        DashaLord::Ketu => "Chandi, a spirit; Ganapati (K)",
    }
}

/// Remedy kind from the tattva of a planet (the element map is the brief's convention;
/// that the kind follows the dispositor's tattva is the text's rule).
pub fn remedy_kind_of_tattva(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun | DashaLord::Mars | DashaLord::Ketu => "fire: homa",
        DashaLord::Mercury => "earth: daana, pratishtha, feeding",
        DashaLord::Moon | DashaLord::Venus => "water: abhisheka, tarpana, bathing at a tirtha",
        DashaLord::Saturn | DashaLord::Rahu => "air: japa, vrata, seva",
        DashaLord::Jupiter => "ether: upadesha, Brahmin feeding, vows of study",
    }
}

/// Badhaka house, badhakesh, companions and what they say is offended.
pub fn badhaka(facts: &Facts) -> Badhaka {
    use DashaLord::*;
    let (lagna_type, house) = match facts.lagna_sign % 3 {
        0 => ("movable", 11u8),
        1 => ("fixed", 9),
        _ => ("dual", 7),
    };
    let lord = lord_of(facts, house);
    let lp = facts.planet(lord);
    let mut companions: Vec<(DashaLord, &'static str, String)> = Vec::new();
    for g in facts.rasi_occupants(house) {
        if g != lord {
            companions.push((
                g,
                offended_being(g),
                format!("{g:?} occupies the badhaka house ({house})"),
            ));
        }
    }
    for p in &facts.planets {
        if p.graha == lord || companions.iter().any(|c| c.0 == p.graha) {
            continue;
        }
        if p.sign == lp.sign {
            companions.push((
                p.graha,
                offended_being(p.graha),
                format!("{:?} is with the badhakesh {lord:?}", p.graha),
            ));
        } else if aspects(facts, p.graha, lord) {
            companions.push((
                p.graha,
                offended_being(p.graha),
                format!("{:?} aspects the badhakesh {lord:?}", p.graha),
            ));
        }
    }
    let offence = (house_of(facts, Mars) == 1)
        .then(|| "Mars in the lagna: a broken idol or sacred object; the remedy is installing a fresh idol (V)".to_string());
    let dispositor = sign_lord(lp.sign as usize);
    let remedy_kind = remedy_kind_of_tattva(dispositor);
    let mut parent_rules = Vec::new();
    let l4 = lord_of(facts, 4);
    if house_of(facts, l4) == house || conj(facts, l4, lord) {
        parent_rules.push(format!("{l4:?}, lord of the 4th, joins the badhaka: the family deity (kula or grama devata) (V)"));
    }
    let l6 = lord_of(facts, 6);
    let l9 = lord_of(facts, 9);
    if house_of(facts, l6) == 9 || house_of(facts, l9) == 12 || house_of(facts, Sun) == 6 {
        parent_rules.push(
            "6th lord in 9th, 9th lord in 12th or Sun in 6th: the father's displeasure (V)".into(),
        );
    }
    if house_of(facts, l6) == 4 || house_of(facts, l4) == 12 || house_of(facts, Moon) == 6 {
        parent_rules.push(
            "6th lord in 4th, 4th lord in 12th or Moon in 6th: the mother's displeasure (V)".into(),
        );
    }
    if conj(facts, Sun, Rahu) {
        parent_rules.push("Sun with Rahu: benevolent serpents, long-term good (V)".into());
    }
    if conj(facts, Moon, Rahu) {
        parent_rules.push("Moon with Rahu: malicious serpents intent on disaster (V)".into());
    }
    Badhaka {
        lagna_type,
        house,
        lord,
        companions,
        offence,
        remedy_kind,
        parent_rules,
    }
}

/// Badhaka as a dosha: present when the badhakesh has companions.
pub fn badhaka_dosha(facts: &Facts) -> Dosha {
    let b = badhaka(facts);
    let mut f = Vec::new();
    for (g, being, how) in &b.companions {
        let w = if is_benefic(facts, *g) { 0.15 } else { 0.3 };
        f.push((w, format!("{how}: {being}")));
    }
    if let Some(o) = &b.offence {
        f.push((0.1, o.clone()));
    }
    for r in &b.parent_rules {
        f.push((0.1, r.clone()));
    }
    let offended = if b.companions.is_empty() {
        format!(
            "no companion of the badhakesh {:?} ({} lagna, badhaka house {})",
            b.lord, b.lagna_type, b.house
        )
    } else {
        b.companions
            .iter()
            .map(|c| c.1)
            .collect::<Vec<_>>()
            .join("; ")
    };
    let mut d = build(
        "Badhaka",
        "KS-R-BADH-01",
        b.lord,
        "Prasna Marga ch. 15 (Harihara): badhaka house by lagna type (movable 11th, fixed 9th, dual 7th); the planet in it, with or aspecting its lord names the offended being; the remedy kind follows the tattva of the badhakesh's dispositor; read for a specific trouble, not a whole life",
        "the badhakesh's family: propitiate the named being; kind by tattva",
        offended,
        f,
    );
    d.factors.insert(
        0,
        format!(
            "{} lagna: badhaka house {}, badhakesh {:?} in rasi house {}; remedy kind {}",
            b.lagna_type,
            b.house,
            b.lord,
            house_of(facts, b.lord),
            b.remedy_kind
        ),
    );
    d
}

/// Kala Sarpa (KS-Y-0070), as a dosha family of its own.
pub fn kala_sarpa_dosha(facts: &Facts) -> Dosha {
    let y = yogas(facts);
    let f = y
        .iter()
        .filter(|y| y.name == "Kala Sarpa")
        .map(|y| (0.5, y.hit.factors.join("; ")))
        .collect();
    build(
        "Kala Sarpa",
        "KS-R-YOGA-02",
        DashaLord::Rahu,
        "Later texts (not BPHS): all planets between Rahu and Ketu; many say it is cancelled by a strong benefic in a kendra",
        "Rahu: Naga puja on Panchami, Durga lamp in Rahu kala, no shortcut for 43 days",
        "the serpents (nodal axis)".into(),
        f,
    )
}

/// Kemadruma (KS-Y-0050): present only when not cancelled; a cancelled Kemadruma is
/// reported absent with the cancellation in the factors.
pub fn kemadruma_dosha(facts: &Facts) -> Dosha {
    let y = yogas(facts);
    let mut f = Vec::new();
    let mut cancelled = Vec::new();
    for y in &y {
        if y.name == "Kemadruma" {
            f.push((0.4, y.hit.factors.join("; ")));
        } else if y.name == "Kemadruma (cancelled)" {
            cancelled = y.hit.factors.clone();
        }
    }
    let mut d = build(
        "Kemadruma",
        "KS-R-YOGA-04",
        DashaLord::Moon,
        "BPHS, Chandra yogas; Phaladeepika 6; Jataka Parijata (bhanga)",
        "Moon: Somavara vrata, serve the mother, fixed sleep, abhisheka on Monday",
        "the Moon alone (the mind unsupported)".into(),
        f,
    );
    if !cancelled.is_empty() {
        d.factors = cancelled;
    }
    d
}

/// Guru Chandala: Jupiter with Rahu (with Ketu, a weaker reading some texts give).
pub fn guru_chandala_dosha(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    if conj(facts, Jupiter, Rahu) {
        f.push((0.6, "Jupiter is conjoined with Rahu".into()));
    } else if conj(facts, Jupiter, Ketu) {
        f.push((
            0.35,
            "Jupiter is conjoined with Ketu (the weaker reading some texts give)".into(),
        ));
    }
    build(
        "Guru Chandala",
        "KS-R-YOGA-05",
        Jupiter,
        "Later compendia (not yet a yoga id in BPHS terms): Jupiter joined with Rahu",
        "Jupiter: Thursday Guru worship, honour teachers, turmeric and chana daana, Vishnu Sahasranama; stop one half-truth taught to others",
        "the guru, truth taught".into(),
        f,
    )
}

/// Grahan dosha: the Sun or the Moon with a node in the natal chart.
pub fn grahan_dosha(facts: &Facts) -> Dosha {
    use DashaLord::*;
    let mut f = Vec::new();
    let mut planet = Sun;
    for lum in [Sun, Moon] {
        for node in [Rahu, Ketu] {
            if conj(facts, lum, node) {
                let sep = (facts.planet(lum).sidereal - facts.planet(node).sidereal)
                    .rem_euclid(360.0)
                    .min(
                        (facts.planet(node).sidereal - facts.planet(lum).sidereal)
                            .rem_euclid(360.0),
                    );
                let w = if lum == Moon { 0.5 } else { 0.4 } + if sep < 12.0 { 0.2 } else { 0.0 };
                f.push((
                    w,
                    format!(
                        "{lum:?} is with {node:?}, {sep:.1}° apart{}",
                        if sep < 12.0 {
                            " (within an eclipse limit)"
                        } else {
                            ""
                        }
                    ),
                ));
                if lum == Moon {
                    planet = Moon;
                }
            }
        }
    }
    build(
        "Grahan dosha",
        "KS-R-YOGA-06",
        planet,
        "Practice (eclipse birth or a luminary with a node); Grahana shanti is performed during the next eclipse, japa from first to last contact (eclipse contacts on the roadmap)",
        "the luminary: Grahana shanti at the next eclipse; Sun or Moon remedies by weekday",
        "the Sun or the Moon under the nodes".into(),
        f,
    )
}

/// Mangal dosha, through the existing detector with its cancellations.
pub fn mangal_dosha_as_dosha(facts: &Facts) -> Dosha {
    let m = mangal_dosha(facts);
    let mut f = Vec::new();
    if m.effective {
        f.push((
            if m.from_lagna_present { 0.6 } else { 0.4 },
            format!(
                "Mars in house {} from the lagna, {} from the Moon, {} from Venus",
                m.from_lagna, m.from_moon, m.from_venus
            ),
        ));
    }
    let mut d = build(
        "Mangal dosha",
        "KS-R-YOGA-03",
        DashaLord::Mars,
        "Mars in 1, 2, 4, 7, 8 or 12 from the lagna, the Moon or Venus, with the standard cancellations (kuta.rs)",
        "Mars: Tuesday Hanuman vow, red-lentil daana, Mangala Chandika stotra; end one quarrel; Kumbha-vivaha (ritual, optional)",
        "the spouse's house (Mars's heat)".into(),
        f,
    );
    if m.present && !m.effective {
        d.factors = m
            .cancellations
            .iter()
            .map(|c| format!("would be Mangal dosha, but {c}"))
            .collect();
    }
    d
}

/// Sade Sati from its phase (0 none, 1 rising, 2 peak, 3 setting).
pub fn sade_sati_dosha(phase: u8, at: Instant) -> Dosha {
    let f = match phase {
        1 => vec![(
            0.5,
            format!(
                "phase 1: Saturn in the 12th from the natal Moon at {}",
                crate::remedy_timing::fmt_utc(at)
            ),
        )],
        2 => vec![(
            0.8,
            format!(
                "phase 2: Saturn on the natal Moon sign at {}",
                crate::remedy_timing::fmt_utc(at)
            ),
        )],
        3 => vec![(
            0.5,
            format!(
                "phase 3: Saturn in the 2nd from the natal Moon at {}",
                crate::remedy_timing::fmt_utc(at)
            ),
        )],
        _ => Vec::new(),
    };
    build(
        "Sade Sati",
        "KS-R-YOGA-01",
        DashaLord::Saturn,
        "Phaladeepika 26: Saturn's transit through the 12th, 1st and 2nd from the natal Moon; phase ingresses solved exactly",
        "Saturn: finish the longest-postponed duty; serve elders and labourers on Saturdays; black til, mustard oil, iron or a blanket in Shani hora; Shani Trayodashi, 7 Saturdays; Hanuman Chalisa; tila-taila lamp (optional)",
        "Saturn over the Moon".into(),
        f,
    )
}

/// Every dosha family, present or not, in a fixed order.
pub fn doshas(facts: &Facts, at: Instant, sade_sati_phase: u8) -> Vec<Dosha> {
    vec![
        pitru_dosha(facts),
        sarpa_dosha(facts),
        matru_shapa(facts),
        bhratru_shapa(facts),
        matula_shapa(facts),
        brahma_shapa(facts),
        patni_shapa(facts),
        preta_shapa(facts),
        badhaka_dosha(facts),
        kala_sarpa_dosha(facts),
        kemadruma_dosha(facts),
        guru_chandala_dosha(facts),
        grahan_dosha(facts),
        mangal_dosha_as_dosha(facts),
        sade_sati_dosha(sade_sati_phase, at),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PlanetFact;
    use DashaLord::*;

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

    /// A clean base chart (Aries lagna, every planet alone, no 5th-house occupant), with
    /// overrides applied; Ketu follows Rahu.
    fn synth(lagna_sign: u8, overrides: &[(DashaLord, f64)]) -> Facts {
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] =
            std::array::from_fn(|i| ((lagna_sign as f64 + i as f64) * 30.0 + 5.0) % 360.0);
        let base = [
            (Sun, 10.0),
            (Moon, 70.0),
            (Mars, 275.0),
            (Mercury, 40.0),
            (Jupiter, 250.0),
            (Venus, 340.0),
            (Saturn, 190.0),
            (Rahu, 100.0),
            (Ketu, 280.0),
        ];
        let mut planets = Vec::new();
        for (g, lon) in base {
            let lon = overrides
                .iter()
                .find(|(o, _)| *o == g)
                .map(|(_, l)| *l)
                .unwrap_or(lon);
            planets.push(planet(g, lon, lagna_sign, &rasi_lords));
        }
        let rahu = planets[7].sidereal;
        planets[8] = planet(
            Ketu,
            (rahu + 180.0).rem_euclid(360.0),
            lagna_sign,
            &rasi_lords,
        );
        Facts {
            planets,
            cusps,
            cusp_sub_lords: [Moon; 12],
            cusp_star_lords: [Moon; 12],
            house_lords: rasi_lords,
            lagna_sign,
            rasi_lords,
            moon_phase_deg: 60.0,
        }
    }

    #[test]
    fn a_clean_chart_has_no_curse() {
        let f = synth(0, &[]);
        for d in [
            pitru_dosha(&f),
            sarpa_dosha(&f),
            matru_shapa(&f),
            bhratru_shapa(&f),
            matula_shapa(&f),
            brahma_shapa(&f),
            patni_shapa(&f),
            preta_shapa(&f),
            guru_chandala_dosha(&f),
            grahan_dosha(&f),
        ] {
            assert!(!d.present, "{} fired: {:?}", d.name, d.factors);
            assert_eq!(d.severity, 0.0);
        }
    }

    #[test]
    fn each_curse_fires_on_its_karaka_in_the_fifth() {
        // Aries lagna: the 5th is Leo (120–150)
        type Case = (&'static str, Facts, fn(&Facts) -> Dosha);
        let cases: Vec<Case> = vec![
            (
                "pitru",
                synth(0, &[(Sun, 125.0), (Saturn, 130.0)]),
                pitru_dosha,
            ),
            (
                "sarpa",
                synth(0, &[(Rahu, 125.0), (Mars, 128.0)]),
                sarpa_dosha,
            ),
            (
                "matru",
                synth(0, &[(Moon, 100.0), (Saturn, 105.0)]),
                matru_shapa,
            ),
            (
                "bhratru",
                synth(0, &[(Mars, 125.0), (Saturn, 130.0)]),
                bhratru_shapa,
            ),
            (
                "matula",
                synth(0, &[(Mercury, 125.0), (Saturn, 128.0)]),
                matula_shapa,
            ),
            (
                "brahma",
                synth(0, &[(Jupiter, 125.0), (Saturn, 128.0)]),
                brahma_shapa,
            ),
            (
                "patni",
                synth(0, &[(Venus, 185.0), (Mars, 190.0)]),
                patni_shapa,
            ),
            (
                "preta",
                synth(0, &[(Sun, 10.0), (Rahu, 195.0)]),
                preta_shapa,
            ), // Ketu at 15°, with the 5th lord Sun
            (
                "guru chandala",
                synth(0, &[(Jupiter, 95.0), (Rahu, 100.0)]),
                guru_chandala_dosha,
            ),
            (
                "grahan",
                synth(0, &[(Moon, 100.0), (Rahu, 102.0)]),
                grahan_dosha,
            ),
        ];
        for (name, f, det) in cases {
            let d = det(&f);
            assert!(d.present, "{name} should be present");
            assert!(
                !d.factors.is_empty() && d.severity > 0.0 && d.severity <= 1.0,
                "{name}"
            );
            assert!(!d.source.is_empty() && !d.remedy_family.is_empty());
        }
        let g = grahan_dosha(&synth(0, &[(Moon, 100.0), (Rahu, 102.0)]));
        assert!(g.factors[0].contains("eclipse limit"));
        assert_eq!(g.planet, Moon);
    }

    #[test]
    fn badhaka_house_follows_the_lagna_type() {
        let movable = badhaka(&synth(0, &[]));
        assert_eq!(
            (movable.lagna_type, movable.house, movable.lord),
            ("movable", 11, Saturn)
        );
        let fixed = badhaka(&synth(1, &[]));
        assert_eq!(
            (fixed.lagna_type, fixed.house, fixed.lord),
            ("fixed", 9, Saturn)
        ); // Taurus → Capricorn
        let dual = badhaka(&synth(2, &[]));
        assert_eq!(
            (dual.lagna_type, dual.house, dual.lord),
            ("dual", 7, Jupiter)
        ); // Gemini → Sagittarius
           // Rahu in Aquarius, the badhaka house of Aries: a serpent deity
        let f = synth(0, &[(Rahu, 310.0)]);
        let b = badhaka(&f);
        assert!(
            b.companions
                .iter()
                .any(|c| c.0 == Rahu && c.1.contains("serpent")),
            "{:?}",
            b.companions
        );
        let d = badhaka_dosha(&f);
        assert!(d.present && d.offended.contains("serpent"));
        assert_eq!(d.planet, Saturn);
        // Mars in the lagna names the broken idol
        let f2 = synth(0, &[(Mars, 20.0), (Rahu, 310.0)]);
        assert!(badhaka(&f2).offence.is_some());
    }

    #[test]
    fn sade_sati_follows_the_phase() {
        let at = Instant::j2000();
        assert!(!sade_sati_dosha(0, at).present);
        let p2 = sade_sati_dosha(2, at);
        assert!(p2.present && p2.severity > sade_sati_dosha(1, at).severity);
    }

    #[test]
    fn kochi_doshas_are_consistent_with_the_yoga_and_kuta_detectors() {
        use kshana_chart::{compute, ChartInput};
        use kshana_ephem::Spk;
        use kshana_houses::HouseSystem;
        use kshana_sky::Observer;
        use kshana_time::Utc;
        use kshana_vedic::Ayanamsa;
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let t = Utc::new(1990, 10, 29, 23, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: t,
                observer: Observer {
                    latitude_deg: 9.9312,
                    longitude_deg: 76.2673,
                    height_m: 0.0,
                },
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let f = Facts::from_chart(&ch);
        let all = doshas(&f, t, 1);
        assert_eq!(all.len(), 15);
        for d in &all {
            assert!((0.0..=1.0).contains(&d.severity), "{}", d.name);
            if d.present {
                assert!(!d.factors.is_empty(), "{}", d.name);
                assert!(d.severity > 0.0);
            } else {
                assert_eq!(d.severity, 0.0);
            }
            assert!(!d.source.is_empty() && !d.rule_id.is_empty());
        }
        let names: Vec<&str> = yogas(&f).iter().map(|y| y.name).collect();
        let ks = all.iter().find(|d| d.name == "Kala Sarpa").unwrap();
        assert_eq!(ks.present, names.contains(&"Kala Sarpa"));
        let km = all.iter().find(|d| d.name == "Kemadruma").unwrap();
        assert_eq!(km.present, names.contains(&"Kemadruma"));
        let md = all.iter().find(|d| d.name == "Mangal dosha").unwrap();
        assert_eq!(md.present, mangal_dosha(&f).effective);
        assert!(all.iter().find(|d| d.name == "Sade Sati").unwrap().present);
        let b = all.iter().find(|d| d.name == "Badhaka").unwrap();
        assert!(b.factors[0].contains("badhaka house"));
    }
}
