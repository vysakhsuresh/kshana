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
    let mut factors = vec![format!(
        "{} cusp sub-lord is {:?}",
        ordinal(area.primary),
        sub_lord
    )];
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
            source: "Krishnamurti Padhdhati, Reader III: the cusp sub-lord must signify the houses of the matter",
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

/// One level of the timing funnel: a dasha period and the group houses its lord signifies.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingLevel {
    /// 1 = mahadasha … 5 = prana.
    pub level: u8,
    /// The period.
    pub period: kshana_vedic::DashaPeriod,
    /// Group houses signified by the period lord (empty = this level does not support).
    pub supports: Vec<Significance>,
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
