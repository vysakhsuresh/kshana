//! Krishnamurti Padhdhati: significators and the cusp sub-lord promise rule.
//!
//! A planet signifies a house at four levels of strength, strongest first:
//! 1. it is in the star of an occupant of the house,
//! 2. it occupies the house,
//! 3. it is in the star of the house's lord,
//! 4. it owns the house.
//!
//! Rahu and Ketu own no sign; they act as agents. A node signifies, in addition to its
//! own star and occupancy levels, everything signified by the planets conjoined with it
//! in the same sign and, failing any, by the lord of the sign it occupies (KP agency
//! rule, Reader III). That is the convention implemented here; aspect-based agency is
//! not included.

use crate::areas::Area;
use crate::facts::Facts;
use crate::plain::Lang;
use crate::yogas::words;
use crate::Hit;
use kshana_vedic::DashaLord;

/// Strength level of a signification (1 strongest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// In the star of an occupant.
    StarOfOccupant = 1,
    /// Occupant.
    Occupant = 2,
    /// In the star of the owner.
    StarOfOwner = 3,
    /// Owner.
    Owner = 4,
}

impl Level {
    /// Weight used when scoring.
    pub fn weight(self) -> f64 {
        match self {
            Level::StarOfOccupant => 1.0,
            Level::Occupant => 0.8,
            Level::StarOfOwner => 0.6,
            Level::Owner => 0.4,
        }
    }
}

/// One signification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Significance {
    /// Which house.
    pub house: u8,
    /// How.
    pub level: Level,
}

fn own_significations(facts: &Facts, g: DashaLord, best: &mut [Option<Level>; 13]) {
    let p = facts.planet(g);
    let mut note = |house: u8, level: Level| {
        let slot = &mut best[house as usize];
        if slot.map_or(true, |l| level < l) {
            *slot = Some(level);
        }
    };
    // 1 and 3: through the star lord
    let star = facts.planet(p.star_lord);
    note(star.house, Level::StarOfOccupant);
    for &h in &star.owns {
        note(h, Level::StarOfOwner);
    }
    // 2 and 4: in its own right
    note(p.house, Level::Occupant);
    for &h in &p.owns {
        note(h, Level::Owner);
    }
}

/// Planets a node stands for: those in its sign, else the lord of its sign.
pub fn node_agents(facts: &Facts, node: DashaLord) -> Vec<DashaLord> {
    let n = facts.planet(node);
    let sign = (n.sidereal / 30.0).floor() as i64;
    let conjoined: Vec<DashaLord> = facts
        .planets
        .iter()
        .filter(|p| p.graha != DashaLord::Rahu && p.graha != DashaLord::Ketu)
        .filter(|p| (p.sidereal / 30.0).floor() as i64 == sign)
        .map(|p| p.graha)
        .collect();
    if conjoined.is_empty() {
        vec![kshana_vedic::sign_lord(sign.rem_euclid(12) as usize)]
    } else {
        conjoined
    }
}

/// All houses a graha signifies, strongest level per house, sorted by house. Nodes add
/// the significations of their agents.
pub fn significators(facts: &Facts, g: DashaLord) -> Vec<Significance> {
    let mut best: [Option<Level>; 13] = [None; 13];
    own_significations(facts, g, &mut best);
    if matches!(g, DashaLord::Rahu | DashaLord::Ketu) {
        for agent in node_agents(facts, g) {
            own_significations(facts, agent, &mut best);
        }
    }
    (1..=12u8)
        .filter_map(|h| best[h as usize].map(|level| Significance { house: h, level }))
        .collect()
}

/// The verdict on whether an area is promised by the natal chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Promise {
    /// The area.
    pub area: Area,
    /// Sub-lord of the primary cusp.
    pub sub_lord: DashaLord,
    /// Group houses the sub-lord signifies.
    pub fulfilling: Vec<Significance>,
    /// Negating houses the sub-lord signifies.
    pub denying: Vec<Significance>,
    /// True when at least one group house is signified and the fulfilling score beats
    /// the denying score.
    pub promised: bool,
    /// Score in `[0, 1]`: fulfilling weight over the group, less the denying share.
    pub strength: f64,
    /// Fulfilling weight averaged over the group houses, before any denial is subtracted.
    pub fulfilling_weight: f64,
    /// Denying weight averaged over the negating houses.
    pub denying_weight: f64,
    /// Signed net `fulfilling_weight − denying_weight / 2`; `strength` is this clamped
    /// to `[0, 1]`, so the UI can tell "nothing supports it" from "outweighed".
    pub net: f64,
    /// The fired rule, with factors in words.
    pub hit: Hit,
}

/// KP cusp sub-lord promise rule.
pub fn promise(facts: &Facts, area: Area) -> Promise {
    promise_in(facts, area, Lang::En)
}

/// [`promise`] with the factors and source in the reader's language.
pub fn promise_in(facts: &Facts, area: Area, lang: Lang) -> Promise {
    let sub_lord = facts.cusp_sub_lords[(area.primary - 1) as usize];
    let sig = significators(facts, sub_lord);
    let fulfilling: Vec<Significance> = sig
        .iter()
        .copied()
        .filter(|s| area.group.contains(&s.house))
        .collect();
    let denying: Vec<Significance> = sig
        .iter()
        .copied()
        .filter(|s| area.negating.contains(&s.house))
        .collect();
    let f: f64 = fulfilling.iter().map(|s| s.level.weight()).sum::<f64>() / area.group.len() as f64;
    let d: f64 =
        denying.iter().map(|s| s.level.weight()).sum::<f64>() / area.negating.len().max(1) as f64;
    let net = f - 0.5 * d;
    let strength = net.clamp(0.0, 1.0);
    let promised = !fulfilling.is_empty() && f > d;
    let mut factors = Vec::new();
    if lang == Lang::En {
        factors.push(format!(
            "{} cusp sub-lord is {:?}",
            ordinal(area.primary),
            sub_lord
        ));
        for s in &fulfilling {
            factors.push(format!(
                "{sub_lord:?} signifies house {} ({})",
                s.house,
                level_words(s.level)
            ));
        }
        for s in &denying {
            factors.push(format!(
                "{sub_lord:?} also signifies house {}, a negating house ({})",
                s.house,
                level_words(s.level)
            ));
        }
        if fulfilling.is_empty() {
            factors.push(format!(
                "{sub_lord:?} signifies none of houses {:?}",
                area.group
            ));
        }
    } else {
        let g = words::graha(lang, sub_lord);
        let ord = words::bhava(lang, area.primary);
        factors.push(crate::say3!(lang, "", "{ord} ഭാവാരംഭത്തിന്റെ ഉപനാഥൻ {g}", "{ord} भावारंभ का उपस्वामी {g}"));
        for s in &fulfilling {
            let (h, lw) = (words::bhava(lang, s.house), level_words_in(lang, s.level));
            factors.push(crate::say3!(lang, "", "{g} {h} ഭാവത്തെ സൂചിപ്പിക്കുന്നു ({lw})", "{g} {h} भाव का कार्येश है ({lw})"));
        }
        for s in &denying {
            let (h, lw) = (words::bhava(lang, s.house), level_words_in(lang, s.level));
            factors.push(crate::say3!(
                lang,
                "",
                "{g} നിഷേധ ഭാവമായ {h} ഭാവത്തെയും സൂചിപ്പിക്കുന്നു ({lw})",
                "{g} निषेधक {h} भाव का भी कार्येश है ({lw})"
            ));
        }
        if fulfilling.is_empty() {
            let hs = words::houses(area.group);
            factors.push(crate::say3!(
                lang,
                "",
                "{g} {hs} എന്നീ ഭാവങ്ങളിൽ ഒന്നിനെയും സൂചിപ്പിക്കുന്നില്ല",
                "{g} भाव {hs} में से किसी का कार्येश नहीं है"
            ));
        }
    }
    Promise {
        area,
        sub_lord,
        fulfilling,
        denying,
        promised,
        strength,
        fulfilling_weight: f,
        denying_weight: d,
        net,
        hit: Hit {
            rule_id: "KS-K-0001",
            source: lang.pick([
                "Krishnamurti Padhdhati, Reader III: the cusp sub-lord must signify the houses of the matter",
                "കൃഷ്ണമൂർത്തി പദ്ധതി, കെപി റീഡർ III: ഭാവാരംഭത്തിന്റെ ഉപനാഥൻ ആ കാര്യത്തിന്റെ ഭാവങ്ങളെ സൂചിപ്പിക്കണം",
                "कृष्णमूर्ति पद्धति, केपी रीडर III: भावारंभ का उपस्वामी उस विषय के भावों का कार्येश होना चाहिए",
            ]),
            weight: strength,
            factors,
        },
    }
}

/// Whether a dasha lord supports an area: it signifies at least one group house.
pub fn period_supports(facts: &Facts, area: Area, lord: DashaLord) -> Vec<Significance> {
    significators(facts, lord)
        .into_iter()
        .filter(|s| area.group.contains(&s.house))
        .collect()
}

/// Net support of a dasha lord for an area, in `[0, 1]`, with the reasons.
///
/// A bare "signifies at least one group house" saturates: for most charts nearly
/// every lord signifies something in a four-house group. This score weighs each
/// signification by its level, counts the negating houses against it, and rewards the
/// primary cusp's house, the KP practice that a dasha lord must be a significator of
/// the matter's houses and not a stronger significator of the houses that deny it.
pub fn support_score(facts: &Facts, area: Area, lord: DashaLord) -> (f64, Vec<String>) {
    support_score_in(facts, area, lord, Lang::En)
}

/// [`support_score`] with the reasons in the reader's language.
pub fn support_score_in(facts: &Facts, area: Area, lord: DashaLord, lang: Lang) -> (f64, Vec<String>) {
    let g = words::graha(lang, lord);
    let sigs = significators(facts, lord);
    let mut fulfil = 0.0;
    let mut deny = 0.0;
    let mut primary = 0.0;
    let mut why = Vec::new();
    for s in &sigs {
        if area.group.contains(&s.house) {
            fulfil += s.level.weight();
            if s.house == area.primary {
                primary = 0.2;
            }
            let (h, lw) = (words::bhava(lang, s.house), level_words_in(lang, s.level));
            let (hn, lwe) = (s.house, level_words(s.level));
            why.push(crate::say3!(
                lang,
                "{lord:?} signifies house {hn} ({lwe})",
                "{g} {h} ഭാവത്തെ സൂചിപ്പിക്കുന്നു ({lw})",
                "{g} {h} भाव का कार्येश है ({lw})"
            ));
        } else if area.negating.contains(&s.house) {
            deny += s.level.weight();
            let (h, lw) = (words::bhava(lang, s.house), level_words_in(lang, s.level));
            let (hn, lwe) = (s.house, level_words(s.level));
            why.push(crate::say3!(
                lang,
                "{lord:?} also signifies negating house {hn} ({lwe})",
                "{g} നിഷേധ ഭാവമായ {h} ഭാവത്തെയും സൂചിപ്പിക്കുന്നു ({lw})",
                "{g} निषेधक {h} भाव का भी कार्येश है ({lw})"
            ));
        }
    }
    let f = (fulfil / area.group.len() as f64).min(1.0);
    let d = (deny / area.negating.len().max(1) as f64).min(1.0);
    let score = (f - 0.8 * d + primary).clamp(0.0, 1.0);
    why.push(crate::say3!(
        lang,
        "net support {score:.2} = fulfilling {f:.2} − 0.8 × denying {d:.2} + primary {primary:.1}",
        "ആകെ പിന്തുണ {score:.2} = അനുകൂലം {f:.2} − 0.8 × നിഷേധം {d:.2} + പ്രധാന ഭാവം {primary:.1}",
        "कुल समर्थन {score:.2} = अनुकूल {f:.2} − 0.8 × निषेधक {d:.2} + मुख्य भाव {primary:.1}"
    ));
    (score, why)
}

/// One level of the timing funnel: a dasha period and the group houses its lord signifies.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingLevel {
    /// 1 = mahadasha … 5 = prana.
    pub level: u8,
    /// The period.
    pub period: kshana_vedic::DashaPeriod,
    /// Group houses signified by the period lord (empty = this level does not support).
    pub supports: Vec<Significance>,
    /// Net support of the lord for the area, `[0, 1]`, from [`support_score`].
    pub strength: f64,
}

/// Weighted strength of a funnel: the levels' net supports blended with the weights
/// 0.35, 0.30, 0.20, 0.10, 0.05 (maha … prana), renormalised to the levels present.
pub fn funnel_strength(funnel: &[TimingLevel]) -> f64 {
    const W: [f64; 5] = [0.35, 0.30, 0.20, 0.10, 0.05];
    let mut num = 0.0;
    let mut den = 0.0;
    for l in funnel {
        let w = W[(l.level as usize).clamp(1, 5) - 1];
        num += w * l.strength;
        den += w;
    }
    if den > 0.0 {
        num / den
    } else {
        0.0
    }
}

/// The Vimshottari funnel for an area at instant `at`, for a birth with the Moon at
/// `moon_sidereal`: each level's lord is checked against the area's houses. The
/// tightest supported window is the deepest level whose lord supports with every
/// level above it supporting too.
pub fn timing(
    facts: &Facts,
    area: Area,
    moon_sidereal: f64,
    birth: kshana_time::Instant,
    at: kshana_time::Instant,
    depth: usize,
) -> Vec<TimingLevel> {
    kshana_vedic::vimshottari_levels(
        moon_sidereal,
        birth,
        kshana_vedic::dasha::YEAR_365_25_NS,
        at,
        depth,
    )
    .into_iter()
    .enumerate()
    .map(|(i, period)| TimingLevel {
        level: (i + 1) as u8,
        period,
        supports: period_supports(facts, area, period.lord),
        strength: support_score(facts, area, period.lord).0,
    })
    .collect()
}

/// Number of consecutive supporting levels from the top of a funnel.
pub fn agreeing_levels(funnel: &[TimingLevel]) -> usize {
    funnel.iter().take_while(|l| !l.supports.is_empty()).count()
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn level_words(l: Level) -> &'static str {
    match l {
        Level::StarOfOccupant => "in the star of an occupant",
        Level::Occupant => "as occupant",
        Level::StarOfOwner => "in the star of its lord",
        Level::Owner => "as its lord",
    }
}

/// [`level_words`] in the reader's language.
pub fn level_words_in(lang: Lang, l: Level) -> &'static str {
    match l {
        Level::StarOfOccupant => lang.pick(["in the star of an occupant", "ഭാവസ്ഥിതന്റെ നക്ഷത്രത്തിൽ", "भावस्थ ग्रह के नक्षत्र में"]),
        Level::Occupant => lang.pick(["as occupant", "ഭാവത്തിൽ നിൽക്കുന്നതിനാൽ", "भाव में स्थित होकर"]),
        Level::StarOfOwner => lang.pick(["in the star of its lord", "ഭാവാധിപന്റെ നക്ഷത്രത്തിൽ", "भावेश के नक्षत्र में"]),
        Level::Owner => lang.pick(["as its lord", "ഭാവാധിപനായി", "भावेश होकर"]),
    }
}

// ---------------------------------------------------------------------------------
// The complete KP table: house significators at four levels, node agency, the
// four-step chain and cuspal interlinks, ruling planets. Expert layer only.
// ---------------------------------------------------------------------------------

/// Significators of one house, level by level (K. S. Krishnamurti, *KP Reader III:
/// Predictive Stellar Astrology*, and *Reader IV: Marriage, Married Life & Children*,
/// "significators"): A planets in the star of an occupant, B occupants, C planets in
/// the star of the owner, D the owner. A is strongest, D weakest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseSignificators {
    /// House 1–12.
    pub house: u8,
    /// A: planets in the star of an occupant.
    pub star_of_occupants: Vec<DashaLord>,
    /// B: occupants.
    pub occupants: Vec<DashaLord>,
    /// C: planets in the star of the owner (the sign lord of the cusp).
    pub star_of_owner: Vec<DashaLord>,
    /// D: the owner.
    pub owner: DashaLord,
    /// Rahu or Ketu acting as agent for a planet in A–D (not already listed there).
    pub node_agents: Vec<DashaLord>,
    /// All of them strongest first: A, B, C, D, each planet once at its strongest
    /// level, with a node that represents a planet placed just before it (KP: the node
    /// gives the results of the planet it represents, before that planet does).
    pub ordered: Vec<DashaLord>,
}

/// Seven grahas that cast aspects, with their special aspects (houses counted from
/// their own sign, the 7th being common to all): BPHS ch. 26 (Graha drishti): Mars 4th
/// and 8th, Jupiter 5th and 9th, Saturn 3rd and 10th.
fn aspects_sign(g: DashaLord, from_sign: u8, to_sign: u8) -> bool {
    let d = (to_sign as i32 - from_sign as i32).rem_euclid(12) + 1;
    d == 7
        || match g {
            DashaLord::Mars => d == 4 || d == 8,
            DashaLord::Jupiter => d == 5 || d == 9,
            DashaLord::Saturn => d == 3 || d == 10,
            _ => false,
        }
}

/// Everything a node stands for, in the KP order of precedence (Krishnamurti, *KP
/// Reader III*, "Rahu and Ketu"; the order as taught in KP practice): planets conjoined
/// with it, planets aspecting it, the lord of its sign, the lord of its star.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeAgency {
    /// Rahu or Ketu.
    pub node: DashaLord,
    /// Grahas in the node's sign.
    pub conjoined: Vec<DashaLord>,
    /// Grahas whose (Parashari, sign-to-sign) aspect falls on the node's sign.
    pub aspecting: Vec<DashaLord>,
    /// Lord of the sign the node occupies.
    pub sign_lord: DashaLord,
    /// Lord of the star the node occupies.
    pub star_lord: DashaLord,
}

/// Full agency of a node. ([`node_agents`], used by the promise rule, keeps the
/// narrower conjunction-else-sign-lord convention.)
pub fn node_agency(facts: &Facts, node: DashaLord) -> NodeAgency {
    let n = facts.planet(node);
    let seven = facts
        .planets
        .iter()
        .filter(|p| !matches!(p.graha, DashaLord::Rahu | DashaLord::Ketu));
    NodeAgency {
        node,
        conjoined: seven.clone().filter(|p| p.sign == n.sign).map(|p| p.graha).collect(),
        aspecting: seven
            .filter(|p| aspects_sign(p.graha, p.sign, n.sign))
            .map(|p| p.graha)
            .collect(),
        sign_lord: kshana_vedic::sign_lord(n.sign as usize),
        star_lord: n.star_lord,
    }
}

/// The agents of a node as one list, strongest first, each once.
fn agency_list(a: &NodeAgency) -> Vec<DashaLord> {
    let mut out: Vec<DashaLord> = Vec::new();
    for g in a
        .conjoined
        .iter()
        .chain(a.aspecting.iter())
        .chain([a.sign_lord, a.star_lord].iter())
    {
        if !out.contains(g) {
            out.push(*g);
        }
    }
    out
}

/// Significators of a house at the four levels with node agency and the strength order.
pub fn house_significators(facts: &Facts, house: u8) -> HouseSignificators {
    let occupants = facts.occupants(house);
    let owner = facts.house_lords[(house as usize - 1) % 12];
    let in_star_of = |lords: &[DashaLord]| -> Vec<DashaLord> {
        facts
            .planets
            .iter()
            .filter(|p| lords.contains(&p.star_lord))
            .map(|p| p.graha)
            .collect()
    };
    let star_of_occupants = in_star_of(&occupants);
    let star_of_owner = in_star_of(&[owner]);
    let mut listed: Vec<DashaLord> = Vec::new();
    for g in star_of_occupants
        .iter()
        .chain(occupants.iter())
        .chain(star_of_owner.iter())
        .chain(std::iter::once(&owner))
    {
        if !listed.contains(g) {
            listed.push(*g);
        }
    }
    let mut node_agents = Vec::new();
    let mut ordered = Vec::new();
    let agencies = [
        node_agency(facts, DashaLord::Rahu),
        node_agency(facts, DashaLord::Ketu),
    ];
    for g in &listed {
        for a in &agencies {
            if !listed.contains(&a.node)
                && !ordered.contains(&a.node)
                && agency_list(a).contains(g)
            {
                ordered.push(a.node);
                node_agents.push(a.node);
            }
        }
        if !ordered.contains(g) {
            ordered.push(*g);
        }
    }
    HouseSignificators {
        house,
        star_of_occupants,
        occupants,
        star_of_owner,
        owner,
        node_agents,
        ordered,
    }
}

/// Houses a planet stands for in its own right: the house it occupies and the houses
/// it owns. A node owns nothing; it takes the occupied and owned houses of its sign
/// lord and of the grahas conjoined with it (the agency rule above, as the four-step
/// method applies it).
pub fn own_houses(facts: &Facts, g: DashaLord) -> Vec<u8> {
    let p = facts.planet(g);
    let mut hs = vec![p.house];
    hs.extend(p.owns.iter().copied());
    if matches!(g, DashaLord::Rahu | DashaLord::Ketu) {
        let a = node_agency(facts, g);
        for agent in a.conjoined.iter().chain(std::iter::once(&a.sign_lord)) {
            let q = facts.planet(*agent);
            hs.push(q.house);
            hs.extend(q.owns.iter().copied());
        }
    }
    hs.sort_unstable();
    hs.dedup();
    hs
}

/// One step of the four-step chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The planet at this step.
    pub planet: DashaLord,
    /// Its own houses ([`own_houses`]).
    pub houses: Vec<u8>,
}

/// The four-step reading of a planet (KP practice after Krishnamurti's rule that a
/// planet gives the results of its star lord, modified by its sub lord; Reader III):
/// 1 the planet, 2 its star lord, 3 its sub lord, 4 the sub lord's star lord.
pub fn four_step(facts: &Facts, g: DashaLord) -> [Step; 4] {
    let p = facts.planet(g);
    let sub = facts.planet(p.sub_lord);
    let step = |x: DashaLord| Step {
        planet: x,
        houses: own_houses(facts, x),
    };
    [step(g), step(p.star_lord), step(p.sub_lord), step(sub.star_lord)]
}

/// A cusp with its full chain of lords and its interlink: the houses its sub lord
/// signifies through the four steps (Krishnamurti, Reader III: "the sub lord of the
/// cusp decides; it signifies the houses of its star lord").
#[derive(Clone, Debug, PartialEq)]
pub struct CuspLink {
    /// House 1–12.
    pub house: u8,
    /// Sidereal cusp longitude, degrees.
    pub sidereal: f64,
    /// Sign lord, star lord, sub lord, sub-sub lord, sub-sub-sub lord.
    pub lords: [DashaLord; 5],
    /// Four-step chain of the cusp's sub lord.
    pub steps: [Step; 4],
    /// Houses signified by the sub lord's star lord (step 2 of the sub lord), the
    /// decisive set in KP; with step 1's houses as the secondary set.
    pub through_star: Vec<u8>,
}

/// Cuspal interlinks of all twelve cusps.
pub fn cusp_links(facts: &Facts) -> Vec<CuspLink> {
    (0..12)
        .map(|i| {
            let l = facts.cusps[i];
            let k = kshana_vedic::kp_lords(l);
            let steps = four_step(facts, k.sub);
            CuspLink {
                house: (i + 1) as u8,
                sidereal: l,
                lords: [k.sign, k.star, k.sub, k.sub_sub, k.sub_sub_sub],
                through_star: steps[1].houses.clone(),
                steps,
            }
        })
        .collect()
}

/// Lord of a weekday, 0 = Sunday (the Vedic day runs sunrise to sunrise).
pub fn weekday_lord(weekday: u32) -> DashaLord {
    use DashaLord::*;
    [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn][weekday as usize % 7]
}

/// Ruling planets of a moment (Krishnamurti, *KP Reader VI: Horary Astrology*,
/// "Ruling planets"; and Reader II on rectification by ruling planets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulingPlanets {
    /// Lord of the weekday (sunrise to sunrise).
    pub day_lord: DashaLord,
    /// Moon's sign, star and sub lords.
    pub moon: [DashaLord; 3],
    /// Lagna's sign, star and sub lords.
    pub lagna: [DashaLord; 3],
    /// Rahu or Ketu admitted as ruling planets: a node in the sign of a ruling planet
    /// or conjoined with one represents it.
    pub nodes: Vec<DashaLord>,
    /// Strongest first, each once: Lagna star lord, Lagna sign lord, Moon star lord,
    /// Moon sign lord, day lord (Krishnamurti's order of strength, Reader VI), then the
    /// Lagna and Moon sub lords (later KP practice, as in KP StarOne), each node placed
    /// just before the planet it represents.
    pub ordered: Vec<DashaLord>,
    /// Ruling planets that sit in the star of a retrograde planet: KP rejects them
    /// for a matter to fructify while that planet is retrograde.
    pub in_retro_star: Vec<DashaLord>,
}

/// Ruling planets from the facts of the moment and its day lord.
pub fn ruling_planets(moment: &Facts, day_lord: DashaLord) -> RulingPlanets {
    let moon_p = moment.planet(DashaLord::Moon);
    let mk = kshana_vedic::kp_lords(moon_p.sidereal);
    let lk = kshana_vedic::kp_lords(moment.cusps[0]);
    let core = [lk.star, lk.sign, mk.star, mk.sign, day_lord, lk.sub, mk.sub];
    let mut nodes = Vec::new();
    let mut ordered: Vec<DashaLord> = Vec::new();
    for g in core {
        for node in [DashaLord::Rahu, DashaLord::Ketu] {
            let n = moment.planet(node);
            let sign_lord = kshana_vedic::sign_lord(n.sign as usize);
            let conj = moment.planet(g).sign == n.sign && g != node;
            if (sign_lord == g || conj) && !ordered.contains(&node) {
                ordered.push(node);
                nodes.push(node);
            }
        }
        if !ordered.contains(&g) {
            ordered.push(g);
        }
    }
    let in_retro_star = ordered
        .iter()
        .copied()
        .filter(|g| {
            let s = moment.planet(*g).star_lord;
            !matches!(s, DashaLord::Rahu | DashaLord::Ketu) && moment.planet(s).retrograde
        })
        .collect();
    RulingPlanets {
        day_lord,
        moon: [mk.sign, mk.star, mk.sub],
        lagna: [lk.sign, lk.star, lk.sub],
        nodes,
        ordered,
        in_retro_star,
    }
}

/// Ruling planets that are also natal significators of any of `houses`, in ruling
/// planet order (KP: the significators that are also ruling planets are the ones that
/// will deliver the matter, Reader VI).
pub fn rp_overlap(natal: &Facts, rp: &RulingPlanets, houses: &[u8]) -> Vec<DashaLord> {
    let sigs: Vec<DashaLord> = houses
        .iter()
        .flat_map(|&h| house_significators(natal, h).ordered)
        .collect();
    rp.ordered
        .iter()
        .copied()
        .filter(|g| sigs.contains(g))
        .collect()
}

#[cfg(test)]
mod support_tests {
    use super::*;

    #[test]
    fn funnel_strength_weights_and_normalises() {
        let p = kshana_vedic::DashaPeriod {
            lord: DashaLord::Sun,
            start: kshana_time::Instant::j2000(),
            end: kshana_time::Instant::j2000(),
        };
        let mk = |level: u8, strength: f64| TimingLevel {
            level,
            period: p,
            supports: Vec::new(),
            strength,
        };
        assert!((funnel_strength(&[mk(1, 1.0), mk(2, 0.0)]) - 0.35 / 0.65).abs() < 1e-12);
        assert_eq!(funnel_strength(&[]), 0.0);
        assert!((funnel_strength(&[mk(1, 0.5)]) - 0.5).abs() < 1e-12);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::AREAS;
    use kshana_chart::{compute, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;
    use kshana_vedic::Ayanamsa;

    fn kochi_facts() -> Option<Facts> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let t = Utc::new(2026, 10, 15, 9, 2, 7, 418_263_000)
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
        Some(Facts::from_chart(&ch))
    }

    /// A Latin word of three or more letters, leaving out Roman numerals (III, VI).
    fn latin_word(s: &str) -> Option<String> {
        s.split(|c: char| !c.is_ascii_alphabetic())
            .find(|w| w.len() >= 3 && !w.chars().all(|c| "IVX".contains(c)))
            .map(str::to_string)
    }

    #[test]
    fn kp_parashari_and_prashna_speak_malayalam_and_hindi() {
        let Some(f) = kochi_facts() else { return };
        let t = Utc::new(2026, 10, 15, 9, 2, 7, 0).unwrap().to_instant().unwrap();
        for lang in [Lang::Ml, Lang::Hi] {
            let mut texts: Vec<String> = Vec::new();
            for a in AREAS {
                let p = promise_in(&f, a, lang);
                texts.push(p.hit.source.into());
                texts.extend(p.hit.factors);
                let b = crate::parashari::assess_in(&f, a, lang);
                for h in b.hits {
                    texts.push(h.source.into());
                    texts.extend(h.factors);
                }
                for g in [DashaLord::Sun, DashaLord::Saturn, DashaLord::Rahu] {
                    texts.extend(support_score_in(&f, a, g, lang).1);
                    texts.extend(crate::parashari::period_quality_in(&f, a, g, lang).1);
                }
                for shape in [crate::prashna::QuestionShape::WillItHappen] {
                    let v = crate::prashna::prashna_in(&f, a, shape, t, 76.27, lang);
                    for h in v.hits {
                        texts.push(h.source.into());
                        texts.extend(h.factors);
                    }
                }
            }
            for y in crate::parashari::yogas_in(&f, lang) {
                texts.push(crate::parashari::yoga_name_in(lang, y.name));
                texts.push(y.hit.source.into());
                texts.extend(y.hit.factors);
            }
            for s in &texts {
                assert_eq!(latin_word(s), None, "{lang:?}: {s}");
            }
        }
        // English is the old text.
        for a in AREAS {
            assert_eq!(promise_in(&f, a, Lang::En), promise(&f, a));
            assert_eq!(crate::parashari::assess_in(&f, a, Lang::En), crate::parashari::assess(&f, a));
        }
        assert_eq!(crate::parashari::yogas_in(&f, Lang::En), crate::parashari::yogas(&f));
    }

    #[test]
    fn facts_are_complete_and_consistent() {
        let Some(f) = kochi_facts() else { return };
        assert_eq!(f.planets.len(), 9);
        let total_owned: usize = f.planets.iter().map(|p| p.owns.len()).sum();
        assert_eq!(
            total_owned, 12,
            "every house has exactly one lord among the seven"
        );
        let rahu = f.planet(DashaLord::Rahu);
        let ketu = f.planet(DashaLord::Ketu);
        assert!(((rahu.sidereal - ketu.sidereal).rem_euclid(360.0) - 180.0).abs() < 1e-9);
        for p in &f.planets {
            assert!((1..=12).contains(&p.house));
        }
    }

    #[test]
    fn significators_follow_the_four_levels() {
        let Some(f) = kochi_facts() else { return };
        for p in &f.planets {
            let sig = significators(&f, p.graha);
            // a planet always signifies the house it occupies, at level ≤ 2
            let occ = sig
                .iter()
                .find(|s| s.house == p.house)
                .expect("occupied house signified");
            assert!(occ.level <= Level::Occupant);
            // and every house it owns
            for h in &p.owns {
                assert!(
                    sig.iter().any(|s| s.house == *h),
                    "{:?} should signify owned house {h}",
                    p.graha
                );
            }
        }
    }

    #[test]
    fn nodes_act_through_agents() {
        let Some(f) = kochi_facts() else { return };
        for node in [DashaLord::Rahu, DashaLord::Ketu] {
            let agents = node_agents(&f, node);
            assert!(!agents.is_empty());
            let sig = significators(&f, node);
            for a in agents {
                for s in significators(&f, a) {
                    assert!(
                        sig.iter().any(|x| x.house == s.house),
                        "{node:?} should carry {a:?}'s house {}",
                        s.house
                    );
                }
            }
        }
    }

    #[test]
    fn timing_funnel_nests_and_counts_agreement() {
        let Some(f) = kochi_facts() else { return };
        let birth = Utc::new(1990, 10, 29, 23, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let at = Utc::new(2026, 10, 15, 9, 2, 7, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let moon = f.planet(DashaLord::Moon).sidereal;
        let funnel = timing(&f, AREAS[0], moon, birth, at, 5);
        assert_eq!(funnel.len(), 5);
        for w in funnel.windows(2) {
            assert!(w[0].period.start <= w[1].period.start && w[1].period.end <= w[0].period.end);
        }
        let n = agreeing_levels(&funnel);
        assert!(n <= 5);
    }

    #[test]
    fn promise_is_explained_for_every_area() {
        let Some(f) = kochi_facts() else { return };
        for a in AREAS {
            let p = promise(&f, a);
            assert!(!p.hit.factors.is_empty());
            assert!(p.hit.factors[0].contains("cusp sub-lord"));
            assert!((0.0..=1.0).contains(&p.strength));
            if p.promised {
                assert!(!p.fulfilling.is_empty());
            }
        }
    }
}

#[cfg(test)]
mod full_table_tests {
    use super::*;
    use kshana_chart::{compute, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;
    use kshana_vedic::Ayanamsa;

    /// Kochi, 1990-02-20 05:04 UTC, 9.9312 N 76.2673 E, Placidus, KP ayanamsa.
    fn kochi_1990() -> Option<(Facts, kshana_chart::Chart)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let t = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: t,
                observer: Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 },
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Krishnamurti,
                topocentric_moon: false,
            },
        )
        .unwrap();
        Some((Facts::from_chart(&ch), ch))
    }

    /// Every lord below was checked by hand from the printed sub table (each sub is
    /// years × 6′40″, from the star lord): e.g. the Sun at Aquarius 7°40′10″ is in
    /// Shatabhisha (Rahu), whose Rahu sub runs 6°40′–8°40′; within it the sub-subs run
    /// Rahu 18′, Jupiter 16′, Saturn 19′, Mercury 17′ (×1′ = 2°/120), so 1°00′10″ in
    /// lies in Mercury, and the sub-sub-sub there is the Moon. The ayanamsa is KP's
    /// (23°37′34″ for this date).
    #[test]
    fn kochi_1990_lords_pinned() {
        use DashaLord::*;
        let Some((f, ch)) = kochi_1990() else { return };
        assert!((ch.ayanamsa_deg - 23.626_081).abs() < 1e-5);
        // (cusp, sign 0-based, d, m, s, [sign, star, sub, sub-sub, sub-sub-sub])
        let cusps: [(u8, u8, (u8, u8, u8), [DashaLord; 5]); 12] = [
            (1, 0, (13, 19, 56), [Mars, Ketu, Mercury, Saturn, Jupiter]),
            (2, 1, (12, 59, 44), [Venus, Moon, Rahu, Mercury, Jupiter]),
            (3, 2, (9, 40, 20), [Mercury, Rahu, Jupiter, Venus, Rahu]),
            (4, 3, (6, 20, 21), [Moon, Saturn, Mercury, Moon, Mercury]),
            (5, 4, (5, 42, 26), [Sun, Ketu, Rahu, Rahu, Mercury]),
            (6, 5, (8, 50, 13), [Mercury, Sun, Venus, Rahu, Venus]),
            (7, 6, (13, 19, 56), [Venus, Rahu, Mercury, Sun, Venus]),
            (8, 7, (12, 59, 44), [Mars, Saturn, Rahu, Rahu, Saturn]),
            (9, 8, (9, 40, 20), [Jupiter, Ketu, Saturn, Mercury, Mercury]),
            (10, 9, (6, 20, 21), [Saturn, Sun, Mercury, Rahu, Ketu]),
            (11, 10, (5, 42, 26), [Saturn, Mars, Moon, Mars, Sun]),
            (12, 11, (8, 50, 13), [Jupiter, Saturn, Venus, Mars, Saturn]),
        ];
        let links = cusp_links(&f);
        for (c, (house, sign, dms, lords)) in links.iter().zip(cusps) {
            assert_eq!(c.house, house);
            assert_eq!((c.sidereal / 30.0).floor() as u8, sign, "cusp {house}");
            assert_eq!(kshana_vedic::dms_in_sign(c.sidereal), dms, "cusp {house}");
            assert_eq!(c.lords, lords, "cusp {house}");
        }
        // (graha, house, dms, [sign, star, sub, sub-sub, sub-sub-sub])
        let planets: [(DashaLord, u8, (u8, u8, u8), [DashaLord; 5]); 9] = [
            (Sun, 11, (7, 40, 10), [Saturn, Rahu, Rahu, Mercury, Moon]),
            (Moon, 8, (4, 36, 9), [Jupiter, Ketu, Moon, Venus, Mars]),
            (Mars, 9, (22, 3, 52), [Jupiter, Venus, Saturn, Saturn, Sun]),
            (Mercury, 10, (18, 1, 3), [Saturn, Moon, Mercury, Mercury, Saturn]),
            (Jupiter, 2, (7, 13, 2), [Mercury, Rahu, Rahu, Jupiter, Rahu]),
            (Venus, 9, (29, 54, 0), [Jupiter, Sun, Rahu, Saturn, Ketu]),
            (Saturn, 9, (27, 38, 50), [Jupiter, Sun, Moon, Rahu, Moon]),
            (Rahu, 10, (22, 47, 40), [Saturn, Moon, Sun, Rahu, Rahu]),
            (Ketu, 4, (22, 47, 40), [Moon, Mercury, Moon, Saturn, Sun]),
        ];
        for (g, house, dms, lords) in planets {
            let p = f.planet(g);
            let k = kshana_vedic::kp_lords(p.sidereal);
            assert_eq!(p.house, house, "{g:?}");
            assert_eq!(kshana_vedic::dms_in_sign(p.sidereal), dms, "{g:?}");
            assert_eq!([k.sign, k.star, k.sub, k.sub_sub, k.sub_sub_sub], lords, "{g:?}");
        }
        assert!(f.planet(Jupiter).retrograde, "Jupiter was retrograde, Nov 1989 – Mar 1990");
    }

    #[test]
    fn kochi_1990_significators_and_links_pinned() {
        use DashaLord::*;
        let Some((f, _)) = kochi_1990() else { return };
        // 10th: Mercury and Rahu occupy it; the Sun and Jupiter (Rahu's stars) and
        // Ketu (Mercury's star) are in their stars; Saturn owns it, nobody in its star.
        let h10 = house_significators(&f, 10);
        assert_eq!(h10.star_of_occupants, vec![Sun, Jupiter, Ketu]);
        assert_eq!(h10.occupants, vec![Mercury, Rahu]);
        assert!(h10.star_of_owner.is_empty());
        assert_eq!(h10.owner, Saturn);
        assert_eq!(h10.ordered, vec![Sun, Jupiter, Ketu, Mercury, Rahu, Saturn]);
        // 9th: Mars (in Venus's star, Venus in the 9th), Mars, Venus, Saturn occupy;
        // Ketu is aspected by Mars, so it comes in front of Mars; Rahu sits in Saturn's sign.
        let h9 = house_significators(&f, 9);
        assert_eq!(h9.star_of_occupants, vec![Mars]);
        assert_eq!(h9.occupants, vec![Mars, Venus, Saturn]);
        assert_eq!(h9.ordered, vec![Ketu, Mars, Venus, Rahu, Saturn, Jupiter]);
        // 12th: untenanted, nobody in Jupiter's star; Jupiter alone.
        assert_eq!(house_significators(&f, 12).ordered, vec![Jupiter]);
        // Rahu in Capricorn with Mercury; Ketu in Cancer under Mars's 8th and Mercury's 7th aspect
        let r = node_agency(&f, Rahu);
        assert_eq!((r.conjoined.clone(), r.aspecting.clone(), r.sign_lord, r.star_lord), (vec![Mercury], vec![], Saturn, Moon));
        let k = node_agency(&f, Ketu);
        assert_eq!((k.conjoined.clone(), k.aspecting.clone(), k.sign_lord, k.star_lord), (vec![], vec![Mars, Mercury], Moon, Mercury));
        // Lagna sub lord Mercury: Mercury occupies 10, owns 3 and 6; in the Moon's star
        // (Moon in 8, owns 4); Mercury is in its own sub; so steps 3–4 repeat 1–2.
        let c1 = &cusp_links(&f)[0];
        let steps: Vec<(DashaLord, Vec<u8>)> = c1.steps.iter().map(|s| (s.planet, s.houses.clone())).collect();
        assert_eq!(steps, vec![(Mercury, vec![3, 6, 10]), (Moon, vec![4, 8]), (Mercury, vec![3, 6, 10]), (Moon, vec![4, 8])]);
        assert_eq!(c1.through_star, vec![4, 8]);
        // 3rd cusp sub lord Jupiter: 2, 9, 12; its star lord Rahu (10, plus Saturn's 9,
        // 10, 11 and Mercury's 3, 6, 10 as agent); its sub lord Rahu; Rahu's star lord the Moon.
        let c3 = &cusp_links(&f)[2];
        assert_eq!(c3.steps[0].houses, vec![2, 9, 12]);
        assert_eq!((c3.steps[1].planet, c3.steps[1].houses.clone()), (Rahu, vec![3, 6, 9, 10, 11]));
        assert_eq!((c3.steps[3].planet, c3.steps[3].houses.clone()), (Moon, vec![4, 8]));
    }

    /// Ruling planets of the birth moment itself, Tuesday (Mars) 10:34 IST: Lagna in
    /// Aries (Mars), Ashwini (Ketu), Mercury sub; Moon in Sagittarius (Jupiter), Mula
    /// (Ketu), Moon sub. Rahu joins as it is conjoined with Mercury in Capricorn.
    #[test]
    fn kochi_1990_ruling_planets_pinned() {
        use DashaLord::*;
        let Some((f, _)) = kochi_1990() else { return };
        let rp = ruling_planets(&f, weekday_lord(2));
        assert_eq!(rp.day_lord, Mars);
        assert_eq!(rp.lagna, [Mars, Ketu, Mercury]);
        assert_eq!(rp.moon, [Jupiter, Ketu, Moon]);
        assert_eq!(rp.nodes, vec![Rahu]);
        assert_eq!(rp.ordered, vec![Ketu, Mars, Jupiter, Rahu, Mercury, Moon]);
        assert!(rp.in_retro_star.is_empty());
        // houses 2, 7, 11 (marriage): Ketu (7 via Mars), Mars (2, 7), Jupiter (2),
        // Rahu (11) overlap; Mercury and the Moon do not signify them.
        assert_eq!(rp_overlap(&f, &rp, &[2, 7, 11]), vec![Ketu, Mars, Jupiter, Rahu]);
        assert_eq!(weekday_lord(0), Sun);
        assert_eq!(weekday_lord(6), Saturn);
    }
}
