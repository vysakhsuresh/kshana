//! Parashari rules: dignities, graha drishti, the classical yogas, and a per-area
//! assessment from house lords, occupants, aspects and karakas. Houses are counted by
//! sign from the lagna (rasi houses), as Brihat Parashara Hora Shastra does.
//!
//! Every rule names its source. Where schools differ (nodes' aspects, Mercury's
//! nature) the convention used is stated in the rule text.

use crate::areas::Area;
use crate::facts::Facts;
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
fn exaltation(g: DashaLord) -> Option<(u8, f64)> {
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
pub(crate) fn natural_relation(a: DashaLord, b: DashaLord) -> i8 {
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

fn connected(facts: &Facts, a: DashaLord, b: DashaLord) -> Option<&'static str> {
    if a == b {
        return Some("the same planet rules both");
    }
    if same_sign(facts, a, b) {
        Some("conjoined in one sign")
    } else if exchange(facts, a, b) {
        Some("in mutual exchange of signs (parivartana)")
    } else if mutual_aspect(facts, a, b) {
        Some("in mutual aspect")
    } else {
        None
    }
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
    use DashaLord::*;
    let mut out = Vec::new();
    let moon = facts.planet(Moon);
    let jup = facts.planet(Jupiter);

    // Gajakesari: Jupiter in a kendra from the Moon.
    let from_moon = ((jup.sign as i32 - moon.sign as i32).rem_euclid(12) + 1) as u8;
    if is_kendra(from_moon) {
        out.push(Yoga {
            name: "Gajakesari",
            hit: Hit {
                rule_id: "KS-Y-0001",
                source: "BPHS, Yogadhyaya; Phaladeepika 6.4: Jupiter in a kendra from the Moon",
                weight: 0.7,
                factors: vec![format!(
                    "Jupiter is in the {from_moon}th sign from the Moon"
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
                source: "Phaladeepika 6: Sun and Mercury in one sign",
                weight: 0.5,
                factors: vec!["Sun and Mercury conjoined".into()],
            },
        });
    }
    // Chandra-Mangala
    if same_sign(facts, Moon, Mars) {
        out.push(Yoga {
            name: "Chandra-Mangala",
            hit: Hit {
                rule_id: "KS-Y-0003",
                source: "BPHS, Chandra yogas: Moon and Mars in one sign",
                weight: 0.5,
                factors: vec!["Moon and Mars conjoined".into()],
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
                    source: "BPHS, Pancha Mahapurusha yogas: the planet in its own or exaltation sign in a kendra from the lagna",
                    weight: 0.8,
                    factors: vec![format!("{g:?} is {d:?} in rasi house {}", p.rasi_house)],
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
            if let Some(how) = connected(facts, kl, tl) {
                seen.push((kl, tl));
                out.push(Yoga {
                    name: "Raja yoga",
                    hit: Hit {
                        rule_id: "KS-Y-0020",
                        source:
                            "BPHS, Raja yogadhyaya: lords of a kendra and a trikona in association",
                        weight: 0.9,
                        factors: vec![format!(
                            "{kl:?} (lord of {kh}) and {tl:?} (lord of {th}) are {how}"
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
        (l2, l11, "2nd and 11th lords"),
        (l2, lords[4], "2nd and 5th lords"),
        (l11, lords[8], "11th and 9th lords"),
    ] {
        if a != b {
            if let Some(how) = connected(facts, a, b) {
                out.push(Yoga {
                    name: "Dhana yoga",
                    hit: Hit {
                        rule_id: "KS-Y-0030",
                        source: "BPHS, Dhana yogadhyaya: lords of wealth houses in association",
                        weight: 0.6,
                        factors: vec![format!("{label} ({a:?}, {b:?}) are {how}")],
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
                    source: "Uttara Kalamrita / Phaladeepika: lord of 6, 8 or 12 placed in another of them",
                    weight: 0.5,
                    factors: vec![format!("{l:?}, lord of {h}, sits in rasi house {}", p.rasi_house)],
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
            cancels.push("a planet stands in a kendra from the Moon");
        }
        if moon_in_kendra {
            cancels.push("the Moon is in a kendra from the lagna");
        }
        if jupiter_on_moon {
            cancels.push("Jupiter is with or aspects the Moon");
        }
        if cancels.is_empty() {
            out.push(Yoga {
                name: "Kemadruma",
                hit: Hit {
                    rule_id: "KS-Y-0050",
                    source: "BPHS, Chandra yogas: no planet on either side of the Moon and no cancellation",
                    weight: 0.4,
                    factors: vec!["no planet in the 2nd or 12th sign from the Moon; none of the cancellations applies".into()],
                },
            });
        } else {
            out.push(Yoga {
                name: "Kemadruma (cancelled)",
                hit: Hit {
                    rule_id: "KS-Y-0051",
                    source: "Phaladeepika 6; Jataka Parijata: Kemadruma bhanga by a planet in a kendra from the Moon, the Moon in a kendra, or Jupiter's association",
                    weight: 0.1,
                    factors: cancels.iter().map(|c| format!("would be Kemadruma, but {c}")).collect(),
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
                        source: "Phaladeepika 7: debilitation cancelled when the dispositor is in a kendra from the lagna or the Moon",
                        weight: 0.5,
                        factors: vec![format!("{:?} is debilitated; its dispositor {disp:?} is in a kendra", p.graha)],
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
                source: "Later texts (not in BPHS): all planets between Rahu and Ketu",
                weight: 0.4,
                factors: vec!["all seven grahas lie on one side of the Rahu–Ketu axis".into()],
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
    let h = area.primary;
    let lord = facts.rasi_lords[(h - 1) as usize];
    let lp = facts.planet(lord);
    let mut hits = Vec::new();
    let mut score = 0.0;
    let mut weight = 0.0;

    // 1. Lord's dignity
    let d = dignity(lord, lp.sidereal);
    hits.push(Hit {
        rule_id: "KS-B-0001",
        source: "BPHS, Bhava phala: results of a house follow the strength of its lord",
        weight: d.score(),
        factors: vec![format!(
            "{lord:?}, lord of the {h}th, is {d:?} at {:.2}°",
            lp.sidereal
        )],
    });
    score += d.score() * 1.0;
    weight += 1.0;

    // 2. Lord's placement
    let (ps, pw) = if is_kendra(lp.rasi_house) || is_trikona(lp.rasi_house) {
        (0.85, "a kendra or trikona")
    } else if is_dusthana(lp.rasi_house) {
        (0.2, "a dusthana (6, 8 or 12)")
    } else {
        (0.55, "an upachaya or neutral house")
    };
    hits.push(Hit {
        rule_id: "KS-B-0002",
        source: "BPHS, Bhava phala: the lord in a kendra or trikona protects the house; in 6, 8 or 12 it harms it",
        weight: ps,
        factors: vec![format!("{lord:?} is placed in rasi house {} ({pw})", lp.rasi_house)],
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
            source: "BPHS, Bhava phala: benefics in a house support it, malefics afflict it",
            weight: s,
            factors: occ
                .iter()
                .map(|g| {
                    format!(
                        "{g:?} occupies the {h}th ({})",
                        if is_benefic(facts, *g) {
                            "benefic"
                        } else {
                            "malefic"
                        }
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
            asp_factors.push(format!(
                "{:?} aspects the {h}th ({})",
                p.graha,
                if b { "benefic" } else { "malefic" }
            ));
        }
    }
    if !asp_factors.is_empty() {
        let s = (asp / asp_factors.len() as f64 * 0.5 + 0.5).clamp(0.0, 1.0);
        hits.push(Hit {
            rule_id: "KS-B-0004",
            source: "BPHS, Bhava phala: aspects of benefics strengthen, of malefics weaken",
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
        source: "BPHS, Ashtakavargadhyaya: a house with more than 28 bindus in Sarvashtakavarga gives; fewer than 25 withholds",
        weight: bs,
        factors: vec![format!("the {h}th house carries {bindus} bindus in Sarvashtakavarga")],
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
        source:
            "BPHS, Karakadhyaya: the natural significator must be sound for the matter to flourish",
        weight: ks,
        factors: vec![format!(
            "karaka {k:?} is {kd:?} in rasi house {}",
            kp.rasi_house
        )],
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
    let p = facts.planet(lord);
    let owns: Vec<u8> = (0..12u8)
        .filter(|i| facts.rasi_lords[*i as usize] == lord)
        .map(|i| i + 1)
        .collect();
    let mut s = dignity(lord, p.sidereal).score();
    let mut f = vec![format!(
        "{lord:?} is {:?} in rasi house {}",
        dignity(lord, p.sidereal),
        p.rasi_house
    )];
    if owns.iter().any(|h| is_kendra(*h) || is_trikona(*h)) {
        s += 0.3;
        f.push(format!("{lord:?} owns a kendra or trikona (house{} {})", if owns.len() > 1 { "s" } else { "" }, owns.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(", ")));
    }
    if owns.iter().any(|h| is_dusthana(*h)) {
        s -= 0.3;
        f.push(format!("{lord:?} owns a dusthana (house{} {})", if owns.len() > 1 { "s" } else { "" }, owns.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(", ")));
    }
    if owns.contains(&area.primary) || owns.iter().any(|h| area.group.contains(h)) {
        s += 0.3;
        f.push(format!("{lord:?} owns a house of the matter"));
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
