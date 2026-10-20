//! Jaimini extras: rasi drishti, argala and virodhargala, the karakamsa, and the Pada
//! lagna and Upapada.
//!
//! # Sources
//!
//! * Rasi drishti (Jaimini Sutras 1.1.3–1.1.4; the rule as every modern reading gives
//!   it: astroapp.com "Jaimini Astrology: Rasi Drishti"; poojn.in "Jaimini Aspects and
//!   Drishti"): a movable sign aspects the fixed signs except the one next to it, a
//!   fixed sign the movable signs except the one next to it, a dual sign the other
//!   dual signs; the relation is mutual.
//! * Argala (Jaimini Sutras 1.1.5–1.1.8; Sanjay Rath, *Crux of Vedic Astrology* and
//!   his Jaimini notes as jyotishabharati.com transcribes them; panchangbodh.com
//!   "Argala calculator"): planets in the 2nd, 4th and 11th from a sign or planet
//!   intervene in its affairs (argala), and planets in the 12th, 10th and 3rd
//!   respectively obstruct them (virodhargala); the 5th is a secondary argala obstructed
//!   by the 9th (Rath; B. V. Raman counts it primary); malefics in the 3rd give an
//!   argala of their own that nothing obstructs (krura, nirbadha argala). Sutra 1.1.7:
//!   the stronger side prevails, counted here as the number of planets on each side.
//!   For Ketu the counting is reversed (Rath).
//! * Karakamsa (Jaimini Sutras 1.2.1–1.2.13 in B. Suryanarain Rao's translation, as
//!   vedicastro.bot "Karakamsa Lagna" quotes it): the navamsa sign of the Atmakaraka,
//!   with one sutra per sign ([`KARAKAMSA_RESULTS`]). Three of the twelve could not be
//!   re-verified against the translation and say so.
//! * Pada lagna and Upapada (Jaimini Sutras 1.1.29–30 and 1.4; `kshana_vedic::arudha`):
//!   the arudha of the 1st and of the 12th, with the readings K. N. Rao's and Sanjay
//!   Rath's schools commonly draw from them, marked as such.

use kshana_rules::parashari::is_benefic;
use kshana_rules::Facts;
use kshana_vedic::{arudha, chara_karakas, sign_lord, varga, DashaLord, KARAKA_NAMES};

use crate::{graha_name, sign_of, SIGN_NAMES};

/// Signs a sign aspects by rasi drishti, 0 = Aries.
pub fn rasi_drishti(sign: u8) -> Vec<u8> {
    let s = sign % 12;
    match s % 3 {
        // movable: the fixed signs except the next
        0 => [1u8, 4, 7, 10]
            .into_iter()
            .filter(|f| *f != (s + 1) % 12)
            .collect(),
        // fixed: the movable signs except the previous
        1 => [0u8, 3, 6, 9]
            .into_iter()
            .filter(|m| *m != (s + 11) % 12)
            .collect(),
        // dual: the other duals
        _ => [2u8, 5, 8, 11].into_iter().filter(|d| *d != s).collect(),
    }
}

/// True when sign `a` aspects sign `b` (and so `b` aspects `a`).
pub fn rasi_aspects(a: u8, b: u8) -> bool {
    rasi_drishti(a).contains(&(b % 12))
}

/// One argala house paired with its obstruction.
#[derive(Clone, Debug, PartialEq)]
pub struct ArgalaPair {
    /// Relative house giving argala (2, 4, 11 or 5).
    pub argala_house: u8,
    /// Relative house obstructing it (12, 10, 3 or 9).
    pub virodha_house: u8,
    /// True for the 5th/9th pair.
    pub secondary: bool,
    /// Planets giving the argala (malefics in the 3rd included on the 11th's pair).
    pub by: Vec<DashaLord>,
    /// Planets obstructing it.
    pub obstructed_by: Vec<DashaLord>,
    /// Givers less obstructors.
    pub net: i8,
}

/// Argala on one house or sign.
#[derive(Clone, Debug, PartialEq)]
pub struct Argala {
    /// Rasi house from the lagna, 1–12.
    pub house: u8,
    /// The sign, 0 = Aries.
    pub sign: u8,
    /// Every planet giving an argala.
    pub by: Vec<DashaLord>,
    /// Every planet obstructing one.
    pub obstructed_by: Vec<DashaLord>,
    /// Sum of the pairs' nets.
    pub net: i8,
    /// The four pairs.
    pub pairs: Vec<ArgalaPair>,
    /// The reading, in words.
    pub words: Vec<String>,
}

const PAIRS: [(u8, u8, bool); 4] = [(2, 12, false), (4, 10, false), (11, 3, false), (5, 9, true)];

fn planets_in(facts: &Facts, sign: u8) -> Vec<DashaLord> {
    facts
        .planets
        .iter()
        .filter(|p| p.sign == sign)
        .map(|p| p.graha)
        .collect()
}

/// Argala on a sign. `reversed` applies Ketu's reversed counting (the 12th, 10th and
/// 3rd give argala, the 2nd, 4th and 11th obstruct).
pub fn argala_on_sign(facts: &Facts, sign: u8, reversed: bool) -> Argala {
    let sign = sign % 12;
    let house = ((sign as i32 - facts.lagna_sign as i32).rem_euclid(12) + 1) as u8;
    let rel = |h: u8| (sign + h - 1) % 12;
    let mut pairs = Vec::with_capacity(4);
    let mut by_all = Vec::new();
    let mut obs_all = Vec::new();
    let mut words = Vec::new();
    for (a, v, secondary) in PAIRS {
        let (a, v) = if reversed { (v, a) } else { (a, v) };
        let mut by = planets_in(facts, rel(a));
        let mut obstructed_by = planets_in(facts, rel(v));
        if !reversed && v == 3 {
            // malefics in the 3rd give an unobstructed argala instead of obstructing
            let (malefic, benefic): (Vec<DashaLord>, Vec<DashaLord>) =
                obstructed_by.iter().partition(|g| !is_benefic(facts, **g));
            for m in &malefic {
                words.push(format!(
                    "{} in the 3rd from {} gives a krura argala that nothing obstructs",
                    graha_name(*m),
                    SIGN_NAMES[sign as usize]
                ));
            }
            by.extend(malefic);
            obstructed_by = benefic;
        }
        let net = (by.len() as i32 - obstructed_by.len() as i32).clamp(-127, 127) as i8;
        if !by.is_empty() || !obstructed_by.is_empty() {
            words.push(format!(
                "{} argala from the {a}th ({}): {}; obstruction from the {v}th ({}): {}; {}",
                if secondary { "secondary" } else { "primary" },
                SIGN_NAMES[rel(a) as usize],
                names(&by),
                SIGN_NAMES[rel(v) as usize],
                names(&obstructed_by),
                match net.cmp(&0) {
                    std::cmp::Ordering::Greater => "the argala prevails",
                    std::cmp::Ordering::Less => "the obstruction prevails",
                    std::cmp::Ordering::Equal => "evenly matched",
                }
            ));
        }
        by_all.extend(by.iter().copied());
        obs_all.extend(obstructed_by.iter().copied());
        pairs.push(ArgalaPair {
            argala_house: a,
            virodha_house: v,
            secondary,
            by,
            obstructed_by,
            net,
        });
    }
    let net = pairs
        .iter()
        .map(|p| p.net as i32)
        .sum::<i32>()
        .clamp(-127, 127) as i8;
    if words.is_empty() {
        words.push(format!(
            "no planet in the 2nd, 4th, 11th or 5th from {} and none in the 12th, 10th, 3rd or 9th: no argala",
            SIGN_NAMES[sign as usize]
        ));
    }
    Argala {
        house,
        sign,
        by: by_all,
        obstructed_by: obs_all,
        net,
        pairs,
        words,
    }
}

fn names(v: &[DashaLord]) -> String {
    if v.is_empty() {
        "none".into()
    } else {
        v.iter()
            .map(|g| graha_name(*g))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Argala on each of the twelve rasi houses from the lagna.
pub fn argala(facts: &Facts) -> Vec<Argala> {
    (0..12u8)
        .map(|i| argala_on_sign(facts, (facts.lagna_sign + i) % 12, false))
        .collect()
}

/// The Jaimini one-liners for the Atmakaraka's navamsa sign, Aries to Pisces (Jaimini
/// Sutras 1.2.2–1.2.13, Rao's translation). The three marked could not be re-verified.
pub const KARAKAMSA_RESULTS: [&str; 12] = [
    "1.2.2: fear and bites of rats, cats and such animals",
    "1.2.3: fear from quadrupeds",
    "1.2.4: corpulence, itches and skin eruptions",
    "1.2.5: dangers from watery places and leprosy",
    "1.2.6: trouble through dogs and such canine animals",
    "1.2.7: fire, itches and corpulence",
    "1.2.8: wealth through merchandise and trade",
    "1.2.9: fear from watery animals and snakes; want of the mother's milk",
    "1.2.10: falls from heights and vehicles (reading not re-verified against the translation)",
    "1.2.11: trouble from aquatic creatures, birds and spirits (reading not re-verified against the translation)",
    "1.2.12: charities: wells, tanks, gardens, temples and rest-houses",
    "1.2.13: fondness for virtuous deeds and charity, tending to final emancipation",
];

/// The karakamsa.
#[derive(Clone, Debug, PartialEq)]
pub struct Karakamsa {
    /// The Atmakaraka (seven-karaka scheme, as `kshana_vedic::chara_karakas`).
    pub atmakaraka: DashaLord,
    /// Its degree within its sign.
    pub degree_in_sign: f64,
    /// Its navamsa sign, 0 = Aries: the karakamsa.
    pub sign: u8,
    /// The lagna's navamsa sign.
    pub navamsa_lagna: u8,
    /// Planets whose navamsa sign is the karakamsa.
    pub occupants: Vec<DashaLord>,
    /// The reading, in words.
    pub words: Vec<String>,
}

/// The karakamsa of a chart: the Atmakaraka's navamsa sign, with the sutra for it.
pub fn karakamsa(facts: &Facts) -> Karakamsa {
    let lon = |g: DashaLord| facts.planet(g).sidereal;
    let k = chara_karakas([
        lon(DashaLord::Sun),
        lon(DashaLord::Moon),
        lon(DashaLord::Mars),
        lon(DashaLord::Mercury),
        lon(DashaLord::Jupiter),
        lon(DashaLord::Venus),
        lon(DashaLord::Saturn),
    ]);
    let ak = k[0];
    let sign = sign_of(varga(lon(ak.graha), 9));
    let navamsa_lagna = sign_of(varga(facts.cusps[0], 9));
    let occupants: Vec<DashaLord> = facts
        .planets
        .iter()
        .filter(|p| sign_of(varga(p.sidereal, 9)) == sign)
        .map(|p| p.graha)
        .collect();
    let words = vec![
        format!(
            "{} {} at {:.2}° in its sign is the Atmakaraka (seven-karaka scheme; with Rahu as an eighth the ranking can differ)",
            KARAKA_NAMES[0],
            graha_name(ak.graha),
            ak.degree_in_sign
        ),
        format!(
            "Karakamsa {}: the Atmakaraka's navamsa sign; Jaimini {}",
            SIGN_NAMES[sign as usize], KARAKAMSA_RESULTS[sign as usize]
        ),
        format!(
            "planets in the karakamsa: {}; navamsa lagna {}",
            names(&occupants),
            SIGN_NAMES[navamsa_lagna as usize]
        ),
    ];
    Karakamsa {
        atmakaraka: ak.graha,
        degree_in_sign: ak.degree_in_sign,
        sign,
        navamsa_lagna,
        occupants,
        words,
    }
}

/// An arudha pada.
#[derive(Clone, Debug, PartialEq)]
pub struct Pada {
    /// "Pada lagna", "Upapada", or "Pada of the Nth".
    pub name: String,
    /// The house whose pada this is, 1–12.
    pub house: u8,
    /// The pada's sign, 0 = Aries.
    pub sign: u8,
    /// Lord of the pada's sign.
    pub lord: DashaLord,
    /// Rasi house of the pada from the lagna, 1–12.
    pub rasi_house: u8,
    /// Planets in the 2nd from the pada.
    pub second: Vec<DashaLord>,
    /// Planets in the 11th from the pada.
    pub eleventh: Vec<DashaLord>,
    /// Planets in the 7th from the pada.
    pub seventh: Vec<DashaLord>,
    /// The reading, in words.
    pub words: Vec<String>,
}

/// The arudha pada of a rasi house (1 = the Pada lagna, 12 = the Upapada).
pub fn pada_of(facts: &Facts, house: u8) -> Pada {
    let house = ((house.max(1) - 1) % 12) + 1;
    let house_sign = (facts.lagna_sign + house - 1) % 12;
    let lord = sign_lord(house_sign as usize);
    let lord_sign = facts.planet(lord).sign;
    let sign = arudha(house_sign as usize, lord_sign as usize) as u8;
    let rel = |h: u8| planets_in(facts, (sign + h - 1) % 12);
    let second = rel(2);
    let eleventh = rel(11);
    let seventh = rel(7);
    let name = match house {
        1 => "Pada lagna".to_string(),
        12 => "Upapada".to_string(),
        h => format!("Pada of the {h}th"),
    };
    let rasi_house = ((sign as i32 - facts.lagna_sign as i32).rem_euclid(12) + 1) as u8;
    let benefics = |v: &[DashaLord]| v.iter().filter(|g| is_benefic(facts, **g)).count();
    let mut words = vec![format!(
        "{name}: house {house} is {}, its lord {} is in {}, {} signs away; the same count again gives {}{}",
        SIGN_NAMES[house_sign as usize],
        graha_name(lord),
        SIGN_NAMES[lord_sign as usize],
        (lord_sign as i32 - house_sign as i32).rem_euclid(12),
        SIGN_NAMES[sign as usize],
        if (lord_sign as i32 - house_sign as i32).rem_euclid(12) % 3 == 0 {
            " (the count landed on the house or its 7th, so the 10th from there is taken)"
        } else {
            ""
        }
    )];
    match house {
        1 => {
            words.push(format!(
                "Pada lagna in rasi house {rasi_house}: the image the world sees; 2nd from it {} ({} benefic), 11th {} ({} benefic): benefics there are read as visible wealth, malefics as struggle for it; 7th from it {}: what opposes the image (Jaimini 1.3 as commonly rendered)",
                names(&second), benefics(&second), names(&eleventh), benefics(&eleventh), names(&seventh)
            ));
        }
        12 => {
            let b2 = benefics(&second);
            let m2 = second.len() - b2;
            words.push(format!(
                "Upapada in rasi house {rasi_house}: the marriage and the spouse's family; 2nd from it {} ({b2} benefic, {m2} malefic): benefics or an exalted planet there are read as a lasting marriage, malefics or a debilitated planet as harm to it (Jaimini 1.4 as commonly rendered); the Upapada's lord {} describes the spouse's background",
                names(&second), graha_name(lord)
            ));
        }
        h => words.push(format!(
            "the pada of the {h}th in rasi house {rasi_house}: where the {h}th's matters show in the world"
        )),
    }
    Pada {
        name,
        house,
        sign,
        lord,
        rasi_house,
        second,
        eleventh,
        seventh,
        words,
    }
}

/// The Pada lagna (arudha of the 1st).
pub fn pada_lagna(facts: &Facts) -> Pada {
    pada_of(facts, 1)
}

/// The Upapada (arudha of the 12th).
pub fn upapada(facts: &Facts) -> Pada {
    pada_of(facts, 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_chart};
    use kshana_rules::PlanetFact;
    use kshana_vedic::DashaLord::*;

    fn planet(graha: DashaLord, sidereal: f64, lagna_sign: u8) -> PlanetFact {
        let sign = sign_of(sidereal);
        let rasi_house = ((sign as i32 - lagna_sign as i32).rem_euclid(12) + 1) as u8;
        PlanetFact {
            graha,
            sidereal,
            house: rasi_house,
            rasi_house,
            sign,
            star_lord: Moon,
            sub_lord: Moon,
            owns: Vec::new(),
            retrograde: false,
        }
    }

    /// Aries lagna. Jupiter in Taurus (2nd), Saturn in Pisces (12th), Venus in Cancer
    /// (4th), Mars in Capricorn (10th), Moon in Aquarius (11th), Sun in Gemini (3rd),
    /// Mercury in Leo (5th), Rahu in Sagittarius (9th), Ketu in Gemini (3rd).
    fn synthetic() -> Facts {
        let lagna_sign = 0u8;
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] = std::array::from_fn(|i| (i as f64 * 30.0 + 5.0) % 360.0);
        let p = |g, lon| planet(g, lon, lagna_sign);
        Facts {
            planets: vec![
                p(Sun, 75.0),
                p(Moon, 310.0),
                p(Mars, 280.0),
                p(Mercury, 130.0),
                p(Jupiter, 40.0),
                p(Venus, 100.0),
                p(Saturn, 340.0),
                p(Rahu, 250.0),
                p(Ketu, 70.0),
            ],
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
    fn rasi_drishti_rule_table() {
        assert_eq!(rasi_drishti(0), vec![4, 7, 10]); // Aries: Leo, Scorpio, Aquarius
        assert_eq!(rasi_drishti(3), vec![1, 7, 10]); // Cancer: Taurus, Scorpio, Aquarius
        assert_eq!(rasi_drishti(1), vec![3, 6, 9]); // Taurus: Cancer, Libra, Capricorn
        assert_eq!(rasi_drishti(10), vec![0, 3, 6]); // Aquarius: Aries, Cancer, Libra
        assert_eq!(rasi_drishti(2), vec![5, 8, 11]); // Gemini: the other duals
        assert_eq!(rasi_drishti(11), vec![2, 5, 8]);
        // mutual, and never a neighbour or itself
        for a in 0..12u8 {
            assert_eq!(rasi_drishti(a).len(), 3);
            for b in rasi_drishti(a) {
                assert!(rasi_aspects(b, a), "{a} {b}");
                assert_ne!(a, b);
                assert_ne!((a + 1) % 12, b);
                assert_ne!((b + 1) % 12, a);
            }
        }
    }

    #[test]
    fn argala_and_virodhargala_on_the_synthetic_chart() {
        let f = synthetic();
        let a = argala_on_sign(&f, 0, false);
        assert_eq!(a.house, 1);
        // 2nd Jupiter vs 12th Saturn: even; 4th Venus vs 10th Mars: even;
        // 11th Moon vs 3rd Sun & Ketu: both malefic → krura argala, nothing obstructs: +3;
        // 5th Mercury vs 9th Rahu: even
        let p = &a.pairs;
        assert_eq!(
            (p[0].by.clone(), p[0].obstructed_by.clone()),
            (vec![Jupiter], vec![Saturn])
        );
        assert_eq!(p[0].net, 0);
        assert_eq!(
            (p[1].by.clone(), p[1].obstructed_by.clone()),
            (vec![Venus], vec![Mars])
        );
        assert_eq!(p[2].by, vec![Moon, Sun, Ketu]);
        assert!(p[2].obstructed_by.is_empty());
        assert_eq!(p[2].net, 3);
        assert!(p[3].secondary);
        assert_eq!(p[3].net, 0);
        assert_eq!(a.net, 3);
        assert!(a.words.iter().any(|w| w.contains("krura argala")));
        // symmetry: reversed counting on the same sign swaps the sides
        let r = argala_on_sign(&f, 0, true);
        assert_eq!(r.pairs[0].by, vec![Saturn]);
        assert_eq!(r.pairs[0].obstructed_by, vec![Jupiter]);
        assert_eq!(r.pairs[2].by, vec![Sun, Ketu]);
        assert_eq!(r.pairs[2].obstructed_by, vec![Moon]);
        // only the 11/3 pair is unbalanced: two argala-givers against one obstructor
        assert_eq!(r.pairs[2].net, 1);
        assert_eq!(r.net, 1);
        // the full table has twelve entries and consistent house numbering
        let all = argala(&f);
        assert_eq!(all.len(), 12);
        for (i, x) in all.iter().enumerate() {
            assert_eq!(x.house as usize, i + 1);
            assert_eq!(x.pairs.len(), 4);
            assert!(!x.words.is_empty());
            assert_eq!(
                x.net as i32,
                x.pairs.iter().map(|p| p.net as i32).sum::<i32>()
            );
        }
        // a planet in the 2nd from sign X is in the 12th from X + 2: argala on one side is
        // obstruction on the other
        for s in 0..12u8 {
            let here = argala_on_sign(&f, s, false);
            let there = argala_on_sign(&f, (s + 2) % 12, false);
            assert_eq!(here.pairs[0].by, there.pairs[0].obstructed_by);
        }
    }

    #[test]
    fn karakamsa_and_padas() {
        let f = synthetic();
        let k = karakamsa(&f);
        // highest degree in sign: Saturn 340° = 10° Pisces? no: Sun 15°, Moon 10°, Mars 10°,
        // Mercury 10°, Jupiter 10°, Venus 10°, Saturn 10° → Sun
        assert_eq!(k.atmakaraka, Sun);
        assert_eq!(k.sign, sign_of(varga(75.0, 9)));
        assert!(k.words[1].contains("Karakamsa"));
        assert_eq!(KARAKAMSA_RESULTS.len(), 12);
        // Pada lagna: Aries, lord Mars in Capricorn (10th): 9 on → Libra; 9 again → Cancer
        let pl = pada_lagna(&f);
        assert_eq!(pl.sign, 3);
        assert_eq!(pl.rasi_house, 4);
        assert_eq!(pl.name, "Pada lagna");
        // Upapada: 12th is Pisces, lord Jupiter in Taurus (2 on) → Cancer
        let ul = upapada(&f);
        assert_eq!(ul.sign, 3);
        assert_eq!(ul.name, "Upapada");
        assert_eq!(ul.second, vec![Mercury]); // Leo
        assert!(ul.words[1].contains("Upapada"));
        let p5 = pada_of(&f, 5);
        assert_eq!(p5.name, "Pada of the 5th");
        assert!(p5.words.iter().all(|w| !w.is_empty()));
    }

    #[test]
    fn kochi_jaimini_from_the_kernel() {
        let Some(spk) = kernel() else { return };
        let f = Facts::from_chart(&kochi_chart(&spk));
        let a = argala(&f);
        assert_eq!(a.len(), 12);
        let k = karakamsa(&f);
        assert!(k.words.iter().all(|w| !w.is_empty()));
        let pl = pada_lagna(&f);
        let ul = upapada(&f);
        assert!(pl.sign < 12 && ul.sign < 12);
        assert!(pl.words.iter().all(|w| !w.is_empty()));
        assert!(ul.words.iter().all(|w| !w.is_empty()));
    }
}
