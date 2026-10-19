//! Sahams: the sensitive points of the annual chart, each the sum of a longitude
//! difference and a third point.
//!
//! Every saham is `A − B + C`. When `C` does not lie on the zodiacal arc from `B`
//! forward to `A` (inclusive), one sign (30°) is added; this is the rule stated by the
//! *Tajika Neelakanthi* and by every modern source (Charak ch. XI; B. V. Raman,
//! *Varshaphal*; Jagannatha Hora), with the between-test taken as coded in PyJHora's
//! `saham.py`: `(C − B) mod 360 ≤ (A − B) mod 360`. By night the first two points of
//! most formulas swap; the ones the *Neelakanthi* gives for day and night alike are
//! marked `night: None`.
//!
//! The table follows the thirty-five sahams of Jagannatha Hora (B. V. Raman's list),
//! cross-checked against K. S. Charak, *A Textbook of Varshaphala*, ch. XI (which
//! prints forty-one, from the *Neelakanthi*) and PyJHora. Where the two readings differ
//! the entry's comment says so:
//!
//! * Mitra: Jupiter − Punya + Venus (JHora); Charak reads the Guru saham (= Vidya) in
//!   Jupiter's place.
//! * Asha: Saturn − Mars + Lagna (JHora); Charak reads Saturn − Venus + Lagna.
//! * Preeti: Sastra − Punya + Lagna (JHora); Charak reads Vidya − Punya + Lagna.
//! * Mrityu: 8th cusp − Moon + Saturn (Charak, Raman); PyJHora adds the lagna instead.
//! * Vyapara: Mars − Mercury + Lagna (Charak, Raman); PyJHora reads Mars − Saturn.
//! * Putra, Bandhu, Paradara, Vanik, Vivaha, Santapa, Sraddha, Preeti: the same by day
//!   and night (Charak, *Neelakanthi*); PyJHora swaps them at night.
//! * Samartha: Jupiter stands in for the lagna lord when that lord is Mars (JHora).
//! * Apamrityu is not in Charak's forty-one; it is JHora's 8th cusp − Mars + Lagna.
//!
//! House points (`Cusp`) are the chart's cusps, Placidus in [`crate::varshaphala`];
//! Jagannatha Hora uses whole-sign house starts, so those sahams can differ from it by
//! the cusp offset.

use crate::sky::AnnualSky;
use crate::tables::sign_index;
use kshana_vedic::{sign_lord, DashaLord};

/// One point in a saham formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Term {
    /// The ascendant degree.
    Lagna,
    /// A graha's longitude.
    Graha(DashaLord),
    /// A house cusp, 1–12.
    Cusp(u8),
    /// The longitude of the lord of the sign on a cusp, 1–12.
    CuspLord(u8),
    /// The longitude of the lagna lord; Jupiter when the lagna lord is Mars.
    LagnaLord,
    /// The longitude of the lord of the sign a graha occupies.
    SignLordOf(DashaLord),
    /// A fixed longitude (Jalapatana uses 15° Cancer).
    Degree(f64),
    /// An earlier saham, by index into [`SAHAMS`].
    Saham(usize),
}

/// A saham's name, meaning and formula. `night` is `None` when the day formula holds by
/// night too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SahamFormula {
    /// Name.
    pub name: &'static str,
    /// What it signifies.
    pub meaning: &'static str,
    /// `A − B + C` by day.
    pub day: [Term; 3],
    /// `A − B + C` by night, or the day formula when `None`.
    pub night: Option<[Term; 3]>,
}

/// Index of the Punya saham in [`SAHAMS`].
pub const PUNYA: usize = 0;
/// Index of the Sastra saham in [`SAHAMS`].
pub const SASTRA: usize = 16;

const L: Term = Term::Lagna;
const SU: Term = Term::Graha(DashaLord::Sun);
const MO: Term = Term::Graha(DashaLord::Moon);
const MA: Term = Term::Graha(DashaLord::Mars);
const ME: Term = Term::Graha(DashaLord::Mercury);
const JU: Term = Term::Graha(DashaLord::Jupiter);
const VE: Term = Term::Graha(DashaLord::Venus);
const SA: Term = Term::Graha(DashaLord::Saturn);
const PU: Term = Term::Saham(PUNYA);

const fn both(name: &'static str, meaning: &'static str, day: [Term; 3]) -> SahamFormula {
    SahamFormula {
        name,
        meaning,
        day,
        night: None,
    }
}

const fn swap(name: &'static str, meaning: &'static str, day: [Term; 3]) -> SahamFormula {
    SahamFormula {
        name,
        meaning,
        day,
        night: Some([day[1], day[0], day[2]]),
    }
}

/// The thirty-five sahams in Jagannatha Hora's order. See the module documentation for
/// sources and variant readings.
pub const SAHAMS: [SahamFormula; 35] = [
    swap("Punya", "fortune, merit", [MO, SU, L]),
    swap("Vidya", "learning", [SU, MO, L]),
    swap("Yasas", "fame", [JU, PU, L]),
    swap("Mitra", "friends", [JU, PU, VE]),
    swap("Mahatmya", "greatness", [PU, MA, L]),
    swap("Asha", "hopes and desires", [SA, MA, L]),
    SahamFormula {
        name: "Samartha",
        meaning: "ability, enterprise",
        day: [MA, Term::LagnaLord, L],
        night: Some([Term::LagnaLord, MA, L]),
    },
    both("Bhratri", "brothers", [JU, SA, L]),
    swap("Gaurava", "honour, respect", [SU, MO, JU]),
    swap("Pitri", "father", [SA, SU, L]),
    swap("Matri", "mother", [MO, VE, L]),
    both("Putra", "children", [JU, MO, L]),
    swap("Jeeva", "life, livelihood", [SA, JU, L]),
    swap("Karma", "work, profession", [MA, ME, L]),
    both("Roga", "disease", [L, MO, L]),
    swap("Kali", "strife, quarrel", [JU, MA, L]),
    swap("Sastra", "learning of the scriptures", [JU, SA, ME]),
    both("Bandhu", "relatives", [ME, MO, L]),
    both("Mrityu", "death", [Term::Cusp(8), MO, SA]),
    both(
        "Paradesa",
        "foreign residence, travel",
        [Term::Cusp(9), Term::CuspLord(9), L],
    ),
    both("Artha", "wealth", [Term::Cusp(2), Term::CuspLord(2), L]),
    both("Paradara", "adultery", [VE, SU, L]),
    both("Vanik", "trade", [MO, ME, L]),
    SahamFormula {
        name: "Karyasiddhi",
        meaning: "success in undertakings",
        day: [SA, SU, Term::SignLordOf(DashaLord::Sun)],
        night: Some([SA, MO, Term::SignLordOf(DashaLord::Moon)]),
    },
    both("Vivaha", "marriage", [VE, SA, L]),
    both("Santapa", "grief, suffering", [SA, MO, Term::Cusp(6)]),
    both("Sraddha", "faith, devotion", [VE, MA, L]),
    both("Preeti", "love, affection", [Term::Saham(SASTRA), PU, L]),
    swap("Jadya", "chronic illness, lethargy", [MA, SA, ME]),
    both("Vyapara", "business", [MA, ME, L]),
    swap("Satru", "enemies", [MA, SA, L]),
    swap(
        "Jalapatana",
        "crossing water, voyages",
        [Term::Degree(105.0), SA, L],
    ),
    swap("Bandhana", "imprisonment, bondage", [PU, SA, L]),
    both(
        "Apamrityu",
        "accidents, unnatural death",
        [Term::Cusp(8), MA, L],
    ),
    both("Labha", "gains", [Term::Cusp(11), Term::CuspLord(11), L]),
];

/// A computed saham.
#[derive(Clone, Debug, PartialEq)]
pub struct Saham {
    /// Name.
    pub name: &'static str,
    /// Sidereal longitude, `[0, 360)`.
    pub longitude: f64,
    /// House 1–12 by the annual chart's cusps.
    pub house: u8,
    /// What it signifies.
    pub meaning: &'static str,
}

/// Whether `c` lies on the zodiacal arc from `b` forward to `a`, inclusive.
pub fn between(a: f64, b: f64, c: f64) -> bool {
    (c - b).rem_euclid(360.0) <= (a - b).rem_euclid(360.0)
}

/// `a − b + c`, plus one sign when `c` is not between `b` and `a`, normalised.
pub fn combine(a: f64, b: f64, c: f64) -> f64 {
    let s = (a - b + c).rem_euclid(360.0);
    if between(a, b, c) {
        s
    } else {
        (s + 30.0).rem_euclid(360.0)
    }
}

fn term_value(t: Term, sky: &AnnualSky, done: &[f64]) -> f64 {
    let lord_lon = |l: DashaLord| sky.longitude(l).unwrap_or(0.0);
    match t {
        Term::Lagna => sky.lagna,
        Term::Graha(g) => lord_lon(g),
        Term::Cusp(n) => sky.cusps[(n as usize + 11) % 12],
        Term::CuspLord(n) => lord_lon(sign_lord(sign_index(sky.cusps[(n as usize + 11) % 12]))),
        Term::LagnaLord => {
            let l = sign_lord(sky.lagna_sign());
            lord_lon(if l == DashaLord::Mars {
                DashaLord::Jupiter
            } else {
                l
            })
        }
        Term::SignLordOf(g) => lord_lon(sign_lord(sky.sign(g).unwrap_or(0))),
        Term::Degree(d) => d,
        Term::Saham(i) => done.get(i).copied().unwrap_or(0.0),
    }
}

/// All sahams for the annual sky, by day or by night, in [`SAHAMS`] order.
pub fn sahams(sky: &AnnualSky, day: bool) -> Vec<Saham> {
    let mut vals: Vec<f64> = Vec::with_capacity(SAHAMS.len());
    for f in SAHAMS.iter() {
        let terms = if day { f.day } else { f.night.unwrap_or(f.day) };
        let a = term_value(terms[0], sky, &vals);
        let b = term_value(terms[1], sky, &vals);
        let c = term_value(terms[2], sky, &vals);
        vals.push(combine(a, b, c));
    }
    SAHAMS
        .iter()
        .zip(vals)
        .map(|(f, longitude)| Saham {
            name: f.name,
            longitude,
            house: sky.house(longitude),
            meaning: f.meaning,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky(lagna: f64, longitudes: [f64; 7]) -> AnnualSky {
        AnnualSky {
            lagna,
            cusps: std::array::from_fn(|i| (lagna + 30.0 * i as f64).rem_euclid(360.0)),
            longitudes,
            speeds: [0.0; 7],
        }
    }

    #[test]
    fn the_sign_rule() {
        // Moon 100, Sun 40, Lagna 70: lagna between Sun and Moon → no addition
        assert_eq!(combine(100.0, 40.0, 70.0), 130.0);
        // Lagna 20: not between → add a sign
        assert_eq!(combine(100.0, 40.0, 20.0), 110.0);
        assert!(between(100.0, 40.0, 100.0));
        assert!(between(100.0, 40.0, 40.0));
        assert!(!between(100.0, 40.0, 101.0));
    }

    #[test]
    fn punya_day_and_night_are_mirror_images() {
        // SEVEN order: Moon, Mercury, Venus, Sun, Mars, Jupiter, Saturn
        let day = sky(70.0, [100.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0]);
        let night = sky(70.0, [40.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0]); // Sun and Moon swapped
        let d = sahams(&day, true);
        let n = sahams(&night, false);
        assert_eq!(d[PUNYA].name, "Punya");
        assert_eq!(d[PUNYA].longitude, 130.0);
        assert_eq!(n[PUNYA].longitude, 130.0);
        // by night with the day sky, Punya = Sun − Moon + Lagna = 40 − 100 + 70 = 10, +30 (lagna
        // not between Moon and Sun going forward from the Moon? 70 − 100 = 330 > 300) → 40
        let nd = sahams(&day, false);
        assert_eq!(nd[PUNYA].longitude, 40.0);
        // Vidya by day equals Punya by night
        assert_eq!(nd[PUNYA].longitude, d[1].longitude);
    }

    #[test]
    fn every_saham_is_in_range_and_housed() {
        let s = sky(123.4, [12.0, 345.0, 200.0, 99.0, 250.0, 180.0, 300.0]);
        for day in [true, false] {
            let out = sahams(&s, day);
            assert_eq!(out.len(), SAHAMS.len());
            for sh in &out {
                assert!((0.0..360.0).contains(&sh.longitude), "{}", sh.name);
                assert!((1..=12).contains(&sh.house));
                assert_eq!(sh.house, s.house(sh.longitude));
            }
        }
        // Samartha: Aries lagna → Mars is the lagna lord → Jupiter stands in
        let aries = sky(5.0, [12.0, 345.0, 200.0, 99.0, 250.0, 180.0, 300.0]);
        let out = sahams(&aries, true);
        let samartha = out.iter().find(|x| x.name == "Samartha").unwrap();
        assert_eq!(samartha.longitude, combine(250.0, 180.0, 5.0));
    }
}
