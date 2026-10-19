//! Planetary avasthas (BPHS ch. 45): the Baladi (age) states by degree in the sign, the
//! Jagradadi (waking) states by dignity, and the Deeptadi states by condition.

use crate::relation::{compound_relation, Relation};
use crate::tables::{combustion_orb, graha_name, SEVEN};
use kshana_chart::Chart;
use kshana_rules::parashari::{dignity, Dignity};
use kshana_vedic::{sign_lord, DashaLord};

/// Baladi avastha (BPHS 45.3–4): in an odd sign the first six degrees are Bala (infant),
/// then Kumara (youth), Yuva (adult), Vriddha (old) and Mrita (dead) six degrees each;
/// in an even sign the order runs the other way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Baladi {
    Bala,
    Kumara,
    Yuva,
    Vriddha,
    Mrita,
}

/// Jagradadi avastha (BPHS 45.5): awake in its own or exaltation sign, dreaming in a
/// friend's or a neutral's, asleep in an enemy's or in debilitation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Jagradadi {
    Jagrat,
    Swapna,
    Sushupti,
}

/// Deeptadi avastha. BPHS 45.6–7 names the states; the conditions used here follow the
/// reading common to Phaladeepika and Jataka Parijata that software reports: Deepta in
/// exaltation, Swastha in own sign (moolatrikona included), Mudita in a friend's sign,
/// Shanta in a neutral's, Deena in an enemy's, Dukhita in debilitation, Vikala when
/// joined in a sign by a natural malefic, Khala in a great enemy's sign, Kopa when
/// combust. Friend, neutral, enemy and great enemy are the compound relationship with the
/// sign's lord. One state is reported with the precedence Kopa, then Vikala, then the
/// placement states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Deeptadi {
    Deepta,
    Swastha,
    Mudita,
    Shanta,
    Deena,
    Dukhita,
    Vikala,
    Khala,
    Kopa,
}

/// The three avasthas of one graha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Avastha {
    /// The graha.
    pub graha: DashaLord,
    /// Age state.
    pub baladi: Baladi,
    /// Waking state.
    pub jagradadi: Jagradadi,
    /// Condition state.
    pub deeptadi: Deeptadi,
}

/// Baladi avastha of a sidereal longitude.
pub fn baladi(sidereal: f64) -> Baladi {
    let l = sidereal.rem_euclid(360.0);
    let odd = ((l / 30.0).floor() as u32) % 2 == 0;
    let sixth = ((l.rem_euclid(30.0) / 6.0).floor() as usize).min(4);
    let order = [
        Baladi::Bala,
        Baladi::Kumara,
        Baladi::Yuva,
        Baladi::Vriddha,
        Baladi::Mrita,
    ];
    if odd {
        order[sixth]
    } else {
        order[4 - sixth]
    }
}

/// Jagradadi avastha from a dignity.
pub fn jagradadi(d: Dignity) -> Jagradadi {
    match d {
        Dignity::Exalted | Dignity::Moolatrikona | Dignity::Own => Jagradadi::Jagrat,
        Dignity::Friend | Dignity::Neutral => Jagradadi::Swapna,
        Dignity::Enemy | Dignity::Debilitated => Jagradadi::Sushupti,
    }
}

fn sign_of(l: f64) -> u8 {
    (l.rem_euclid(360.0) / 30.0).floor() as u8
}

/// The avasthas of the seven grahas in a chart.
pub fn avasthas(chart: &Chart) -> Vec<Avastha> {
    let find = |g: DashaLord| chart.bodies.iter().find(|b| b.name == graha_name(g));
    let signs: Vec<(DashaLord, u8)> = SEVEN
        .iter()
        .filter_map(|g| find(*g).map(|b| (*g, sign_of(b.sidereal))))
        .collect();
    let sign_lookup = |g: DashaLord| signs.iter().find(|(p, _)| *p == g).map(|(_, s)| *s);
    let rahu = sign_of(chart.true_node.1);
    let ketu = (rahu + 6) % 12;
    let sun = find(DashaLord::Sun).map(|b| b.sidereal);
    let malefic_signs: Vec<(DashaLord, u8)> = [DashaLord::Sun, DashaLord::Mars, DashaLord::Saturn]
        .iter()
        .filter_map(|g| sign_lookup(*g).map(|s| (*g, s)))
        .chain([(DashaLord::Rahu, rahu), (DashaLord::Ketu, ketu)])
        .collect();
    let mut out = Vec::with_capacity(7);
    for g in SEVEN {
        let Some(b) = find(g) else { continue };
        let sign = sign_of(b.sidereal);
        let combust = match (sun, combustion_orb(g, b.speed < 0.0)) {
            (Some(s), Some(orb)) if g != DashaLord::Sun => {
                let d = (b.sidereal - s).rem_euclid(360.0);
                let d = if d > 180.0 { 360.0 - d } else { d };
                d < orb
            }
            _ => false,
        };
        let with_malefic = malefic_signs.iter().any(|(m, s)| *m != g && *s == sign);
        let d = dignity(g, b.sidereal);
        let deeptadi = if combust {
            Deeptadi::Kopa
        } else if with_malefic {
            Deeptadi::Vikala
        } else {
            match d {
                Dignity::Exalted => Deeptadi::Deepta,
                Dignity::Own | Dignity::Moolatrikona => Deeptadi::Swastha,
                Dignity::Debilitated => Deeptadi::Dukhita,
                _ => {
                    let lord = sign_lord(sign as usize);
                    let lord_sign = sign_lookup(lord).unwrap_or(sign);
                    match compound_relation(g, lord, sign, lord_sign) {
                        Relation::AdhiMitra | Relation::Mitra => Deeptadi::Mudita,
                        Relation::Sama => Deeptadi::Shanta,
                        Relation::Satru => Deeptadi::Deena,
                        Relation::AdhiSatru => Deeptadi::Khala,
                    }
                }
            }
        };
        out.push(Avastha {
            graha: g,
            baladi: baladi(b.sidereal),
            jagradadi: jagradadi(d),
            deeptadi,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baladi_runs_forward_in_odd_signs_and_back_in_even() {
        assert_eq!(baladi(3.0), Baladi::Bala);
        assert_eq!(baladi(9.0), Baladi::Kumara);
        assert_eq!(baladi(15.0), Baladi::Yuva);
        assert_eq!(baladi(21.0), Baladi::Vriddha);
        assert_eq!(baladi(27.0), Baladi::Mrita);
        assert_eq!(baladi(33.0), Baladi::Mrita);
        assert_eq!(baladi(45.0), Baladi::Yuva);
        assert_eq!(baladi(57.0), Baladi::Bala);
        assert_eq!(baladi(359.9), Baladi::Bala);
    }

    #[test]
    fn jagradadi_by_dignity() {
        assert_eq!(jagradadi(Dignity::Exalted), Jagradadi::Jagrat);
        assert_eq!(jagradadi(Dignity::Own), Jagradadi::Jagrat);
        assert_eq!(jagradadi(Dignity::Friend), Jagradadi::Swapna);
        assert_eq!(jagradadi(Dignity::Neutral), Jagradadi::Swapna);
        assert_eq!(jagradadi(Dignity::Enemy), Jagradadi::Sushupti);
        assert_eq!(jagradadi(Dignity::Debilitated), Jagradadi::Sushupti);
    }
}
