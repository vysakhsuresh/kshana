//! Transit scoring: Ashtakavarga bindus, Kakshya, Vedha, Moorthi Nirnaya, classical gochara results.
//!
//! Four classical instruments, each a pure function of the natal chart and an instant,
//! blended into one score per transit window:
//!
//! * **Gochara with Vedha** (Mantreswara, Phaladeepika ch. 26): for each graha, the houses
//!   counted from the natal Moon in which its transit is favourable, and for each such
//!   house the *vedha* (obstruction) house; another graha transiting the vedha house annuls
//!   the good result. The Sun and Saturn never obstruct each other, nor do the Moon and
//!   Mercury. See [`vedha_table`] for the verses.
//! * **Moorthi Nirnaya**: the house of the transit Moon from the natal Moon at the moment a
//!   graha enters a sign classifies that whole transit as gold, silver, copper or iron.
//!   See [`Moorthi`].
//! * **Ashtakavarga transit** (BPHS, Ashtakavargadhyaya): the bindus the transited sign
//!   holds in the graha's own Bhinnashtakavarga (4 or more is good) and in the
//!   Sarvashtakavarga (28, the mean, or more is good). See [`transit_bindus`].
//! * **Kakshya**: each sign is cut into eight cells of 3°45′ lorded, in order, by Saturn,
//!   Jupiter, Mars, the Sun, Venus, Mercury, the Moon and the Lagna; a graha moving
//!   through a cell gives its good while that cell's lord contributed a bindu to the
//!   graha's Bhinnashtakavarga in that sign. See [`kakshya_windows`].
//!
//! Every window boundary here is a solved root, never an interpolation.

use kshana_chart::Chart;
use kshana_ephem::{Body, Spk, SpkError};
use kshana_rules::{bhinna_for, sarva_for, Facts};
use kshana_sky::{apparent, lunar_node, node_longitude, LunarNode};
use kshana_solver::{find_root, wrap180};
use kshana_time::{Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa, DashaLord};

use crate::transits::sidereal_ingresses;

/// Root tolerance for every boundary solved here: one microsecond.
const TOL_NS: i128 = 1_000;

/// The nine grahas of gochara, in the order results are listed.
pub const GOCHARA_PLANETS: [DashaLord; 9] = [
    DashaLord::Sun,
    DashaLord::Moon,
    DashaLord::Mars,
    DashaLord::Mercury,
    DashaLord::Jupiter,
    DashaLord::Venus,
    DashaLord::Saturn,
    DashaLord::Rahu,
    DashaLord::Ketu,
];

const SIGN_NAMES: [&str; 12] = [
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
    "Aquarius",
    "Pisces",
];

fn name(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Sun",
        DashaLord::Moon => "Moon",
        DashaLord::Mars => "Mars",
        DashaLord::Mercury => "Mercury",
        DashaLord::Jupiter => "Jupiter",
        DashaLord::Venus => "Venus",
        DashaLord::Saturn => "Saturn",
        DashaLord::Rahu => "Rahu",
        DashaLord::Ketu => "Ketu",
    }
}

/// Kernel body of a graha; `None` for the nodes, which move by mean motion here.
fn body_of(g: DashaLord) -> Option<Body> {
    match g {
        DashaLord::Sun => Some(Body::Sun),
        DashaLord::Moon => Some(Body::Moon),
        DashaLord::Mars => Some(Body::MarsBarycenter),
        DashaLord::Mercury => Some(Body::Mercury),
        DashaLord::Jupiter => Some(Body::JupiterBarycenter),
        DashaLord::Venus => Some(Body::Venus),
        DashaLord::Saturn => Some(Body::SaturnBarycenter),
        DashaLord::Rahu | DashaLord::Ketu => None,
    }
}

/// Row of a graha in the Bhinnashtakavarga tables (Sun 0 … Saturn 6); `None` for the nodes.
fn bav_row(g: DashaLord) -> Option<usize> {
    match g {
        DashaLord::Sun => Some(0),
        DashaLord::Moon => Some(1),
        DashaLord::Mars => Some(2),
        DashaLord::Mercury => Some(3),
        DashaLord::Jupiter => Some(4),
        DashaLord::Venus => Some(5),
        DashaLord::Saturn => Some(6),
        DashaLord::Rahu | DashaLord::Ketu => None,
    }
}

fn sign_of(lon: f64) -> u8 {
    ((lon.rem_euclid(360.0) / 30.0).floor() as u8) % 12
}

/// House (1–12) that `to_sign` is, counted from `from_sign`.
fn house_between(from_sign: u8, to_sign: u8) -> u8 {
    ((i32::from(to_sign) - i32::from(from_sign)).rem_euclid(12) + 1) as u8
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        _ => format!("{h}th"),
    }
}

fn natal_moon_sidereal(natal: &Chart) -> f64 {
    natal
        .bodies
        .iter()
        .find(|b| b.name == "Moon")
        .map(|b| b.sidereal)
        .expect("a chart has a Moon")
}

/// Sidereal longitude of a gochara graha at `t`, degrees in `[0, 360)`; Rahu is the lunar
/// node of the engine-wide [`kshana_sky::lunar_node`] choice (the true node by default,
/// the same point the natal chart calls Rahu) and Ketu its opposite.
pub fn gochara_longitude(
    spk: &Spk,
    planet: DashaLord,
    ayanamsa: Ayanamsa,
    t: Instant,
) -> Result<f64, SpkError> {
    gochara_longitude_with(spk, planet, ayanamsa, lunar_node(), t)
}

/// [`gochara_longitude`] with the node named explicitly.
pub fn gochara_longitude_with(
    spk: &Spk,
    planet: DashaLord,
    ayanamsa: Ayanamsa,
    node: LunarNode,
    t: Instant,
) -> Result<f64, SpkError> {
    let tc = t.tt_centuries_since_j2000();
    let ay = ayanamsa_degrees(ayanamsa, tc);
    let tropical = match body_of(planet) {
        Some(body) => apparent(spk, body, t)?.longitude,
        None if planet == DashaLord::Rahu => node_longitude(spk, node, t)?,
        None => node_longitude(spk, node, t)? + 180.0,
    };
    Ok(sidereal(tropical, ay).rem_euclid(360.0))
}

/// The nine grahas' sidereal longitudes at `t`.
fn positions_at(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    t: Instant,
) -> Result<Vec<(DashaLord, f64)>, SpkError> {
    GOCHARA_PLANETS
        .iter()
        .map(|&g| gochara_longitude(spk, g, ayanamsa, t).map(|l| (g, l)))
        .collect()
}

// ---------------------------------------------------------------------------------------
// Vedha
// ---------------------------------------------------------------------------------------

/// Favourable houses from the natal Moon and the vedha house of each, as `(house, vedha)`.
///
/// Phaladeepika ch. 26 (Mantreswara; the wisdomlib.org text-and-translation, "Transits of
/// Planets \[Chapter 26\]"), one verse per graha, in order:
///
/// * Sun (26.3): auspicious in the 11th, 3rd, 10th and 6th unless the vedha places, the
///   5th, 9th, 4th and 12th respectively, hold a graha other than Saturn.
/// * Moon (26.4): good in the 7th, 1st, 6th, 11th, 10th and 3rd unless grahas other than
///   Mercury transit the 2nd, 5th, 12th, 8th, 4th and 9th respectively.
/// * Mars (26.5): good in the 3rd, 6th and 11th, vedha from the 12th, 9th and 5th.
/// * Mercury (26.6): auspicious in the 2nd, 4th, 6th, 8th, 10th and 11th unless the 5th,
///   3rd, 9th, 1st, 8th and 12th hold a graha other than the Moon.
/// * Jupiter (26.7): good in the 2nd, 11th, 9th, 5th and 7th if the 12th, 8th, 10th, 4th
///   and 3rd are void of grahas.
/// * Venus (26.8): good in the 1st, 2nd, 3rd, 4th, 5th, 8th, 9th, 11th and 12th, vedha
///   from the 8th, 7th, 1st, 10th, 9th, 5th, 11th, 6th and 3rd respectively.
/// * Saturn (26.9): as Mars, the 3rd, 6th and 11th with vedha from the 12th, 9th and 5th,
///   by a graha other than the Sun.
///
/// The Sun, Moon, Mercury and Jupiter rows were read from that translation; the Venus
/// row from the "Transit of Planets and Vedhas" table that quotes the same verse; the
/// Mars and Saturn pairing (3→12, 6→9, 11→5) is the one B. V. Raman tabulates in Hindu
/// Predictive Astrology ch. 34 (Gocharaphala). Rahu and Ketu are not in the text: the
/// Jagannatha Hora gochara notes give them Saturn's 3, 6, 11 "by common convention", and
/// this table gives them Saturn's vedhas with it.
pub fn vedha_table(planet: DashaLord) -> &'static [(u8, u8)] {
    match planet {
        DashaLord::Sun => &[(3, 9), (6, 12), (10, 4), (11, 5)],
        DashaLord::Moon => &[(1, 5), (3, 9), (6, 12), (7, 2), (10, 4), (11, 8)],
        DashaLord::Mars | DashaLord::Saturn | DashaLord::Rahu | DashaLord::Ketu => {
            &[(3, 12), (6, 9), (11, 5)]
        }
        DashaLord::Mercury => &[(2, 5), (4, 3), (6, 9), (8, 1), (10, 8), (11, 12)],
        DashaLord::Jupiter => &[(2, 12), (5, 4), (7, 3), (9, 10), (11, 8)],
        DashaLord::Venus => &[
            (1, 8),
            (2, 7),
            (3, 1),
            (4, 10),
            (5, 9),
            (8, 5),
            (9, 11),
            (11, 6),
            (12, 3),
        ],
    }
}

/// True for the two pairs that never obstruct each other: the Sun and Saturn, the Moon and
/// Mercury (Phaladeepika 26.3, 26.4, 26.6, 26.9: "other than Saturn", "other than Mercury",
/// "other than the Moon", "other than the Sun").
pub fn vedha_exempt(a: DashaLord, b: DashaLord) -> bool {
    matches!(
        (a, b),
        (DashaLord::Sun, DashaLord::Saturn)
            | (DashaLord::Saturn, DashaLord::Sun)
            | (DashaLord::Moon, DashaLord::Mercury)
            | (DashaLord::Mercury, DashaLord::Moon)
    )
}

/// The classical gochara result of a graha in a house from the natal Moon, with vedha.
#[derive(Clone, Debug, PartialEq)]
pub struct GocharaVerdict {
    /// True when the house is favourable for this graha and no graha obstructs it.
    pub favourable: bool,
    /// The graha obstructing an otherwise favourable house, if any.
    pub vedha_by: Option<DashaLord>,
    /// The verdict in words.
    pub words: String,
}

/// Judge `planet` transiting `house_from_moon` (1–12) given the other transiting grahas'
/// houses from the natal Moon in `occupants`. A favourable house is obstructed when any
/// other graha (not the planet itself, and not its [`vedha_exempt`] partner) stands in the
/// paired vedha house. An unfavourable house is reported as such with no vedha.
pub fn gochara_verdict(
    planet: DashaLord,
    house_from_moon: u8,
    occupants: &[(DashaLord, u8)],
) -> GocharaVerdict {
    let who = name(planet);
    let at = ordinal(house_from_moon);
    let Some(&(_, vedha)) = vedha_table(planet)
        .iter()
        .find(|(h, _)| *h == house_from_moon)
    else {
        return GocharaVerdict {
            favourable: false,
            vedha_by: None,
            words: format!(
                "{who} in the {at} from your Moon: not a favourable house for its transit (Phaladeepika 26)"
            ),
        };
    };
    let vedha_by = occupants
        .iter()
        .find(|(g, h)| *h == vedha && *g != planet && !vedha_exempt(planet, *g))
        .map(|(g, _)| *g);
    match vedha_by {
        Some(g) => GocharaVerdict {
            favourable: false,
            vedha_by: Some(g),
            words: format!(
                "{who} in the {at} from your Moon: a favourable house, but {} in the {} obstructs it (vedha)",
                name(g),
                ordinal(vedha)
            ),
        },
        None => GocharaVerdict {
            favourable: true,
            vedha_by: None,
            words: format!(
                "{who} in the {at} from your Moon: a favourable house, unobstructed (no graha in the {})",
                ordinal(vedha)
            ),
        },
    }
}

// ---------------------------------------------------------------------------------------
// Moorthi Nirnaya
// ---------------------------------------------------------------------------------------

/// Moorthi Nirnaya: the metal of a transit, from the transit Moon's house (counted from the
/// natal Moon) at the instant the graha entered its sign. Houses 1, 6, 11 give Swarna
/// (gold); 2, 5, 9 Rajata (silver); 3, 7, 10 Tamra (copper); 4, 8, 12 Loha (iron). Gold
/// and silver are good, copper middling, iron bad. (Sources agree on the grouping:
/// cosmicinsights.net "Moorti Nirnaya – the best way to judge transits"; astroshala.com
/// "Moorti Nirnaya: a traditional and authentic approach"; vijayalur.com "Moorti Nirnaya".)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moorthi {
    /// Gold: the Moon in the 1st, 6th or 11th from the natal Moon at the ingress.
    Swarna,
    /// Silver: the 2nd, 5th or 9th.
    Rajata,
    /// Copper: the 3rd, 7th or 10th.
    Tamra,
    /// Iron: the 4th, 8th or 12th.
    Loha,
}

impl Moorthi {
    /// The moorthi for the transit Moon's house (1–12) from the natal Moon.
    pub fn from_house(house: u8) -> Moorthi {
        match house {
            1 | 6 | 11 => Moorthi::Swarna,
            2 | 5 | 9 => Moorthi::Rajata,
            3 | 7 | 10 => Moorthi::Tamra,
            _ => Moorthi::Loha,
        }
    }

    /// Sanskrit name.
    pub fn name(self) -> &'static str {
        match self {
            Moorthi::Swarna => "Swarna",
            Moorthi::Rajata => "Rajata",
            Moorthi::Tamra => "Tamra",
            Moorthi::Loha => "Loha",
        }
    }

    /// The metal in English.
    pub fn metal(self) -> &'static str {
        match self {
            Moorthi::Swarna => "gold",
            Moorthi::Rajata => "silver",
            Moorthi::Tamra => "copper",
            Moorthi::Loha => "iron",
        }
    }

    /// Quality in `[0, 1]`: gold 1.0, silver 0.75, copper 0.4, iron 0.1.
    pub fn score(self) -> f64 {
        match self {
            Moorthi::Swarna => 1.0,
            Moorthi::Rajata => 0.75,
            Moorthi::Tamra => 0.4,
            Moorthi::Loha => 0.1,
        }
    }
}

/// Moorthi Nirnaya of a sign ingress at `ingress`: the transit Moon's house from the natal
/// Moon (`natal_moon_sidereal`, degrees) at that instant, classified by [`Moorthi::from_house`].
pub fn moorthi(
    spk: &Spk,
    natal_moon_sidereal: f64,
    ayanamsa: Ayanamsa,
    ingress: Instant,
) -> Result<Moorthi, SpkError> {
    let moon = gochara_longitude(spk, DashaLord::Moon, ayanamsa, ingress)?;
    Ok(Moorthi::from_house(house_between(
        sign_of(natal_moon_sidereal),
        sign_of(moon),
    )))
}

// ---------------------------------------------------------------------------------------
// Ashtakavarga transit and Kakshya
// ---------------------------------------------------------------------------------------

/// Bindus the transited `sign` (0 = Aries) holds for `planet`: `(bav, sav)`, the graha's
/// Bhinnashtakavarga count and the Sarvashtakavarga count. Four or more BAV bindus and
/// 28 or more SAV bindus (the mean of 337 over twelve signs) are the classical thresholds
/// (BPHS, Ashtakavargadhyaya). Rahu and Ketu own no Bhinnashtakavarga: their `bav` is 0.
pub fn transit_bindus(facts: &Facts, planet: DashaLord, sign: u8) -> (u8, u8) {
    let s = usize::from(sign % 12);
    let sav = sarva_for(facts)[s];
    let bav = bav_row(planet).map_or(0, |row| bhinna_for(facts)[row][s]);
    (bav, sav)
}

/// Lords of the eight kakshyas of a sign, in order from 0°: Saturn, Jupiter, Mars, the
/// Sun, Venus, Mercury, the Moon, the Lagna, each 3°45′ wide. (The order is the planets'
/// order of motion, slowest first, then the Lagna: Jagannatha Hora "Ashtakavarga
/// explained"; vedastro.org "Mastering Ashtakavarga part 15: transits".)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KakshyaLord {
    /// 0°00′–3°45′.
    Saturn,
    /// 3°45′–7°30′.
    Jupiter,
    /// 7°30′–11°15′.
    Mars,
    /// 11°15′–15°00′.
    Sun,
    /// 15°00′–18°45′.
    Venus,
    /// 18°45′–22°30′.
    Mercury,
    /// 22°30′–26°15′.
    Moon,
    /// 26°15′–30°00′.
    Lagna,
}

/// The kakshya lords in order from the start of a sign.
pub const KAKSHYA_LORDS: [KakshyaLord; 8] = [
    KakshyaLord::Saturn,
    KakshyaLord::Jupiter,
    KakshyaLord::Mars,
    KakshyaLord::Sun,
    KakshyaLord::Venus,
    KakshyaLord::Mercury,
    KakshyaLord::Moon,
    KakshyaLord::Lagna,
];

/// Width of one kakshya in degrees: 3°45′.
pub const KAKSHYA_DEG: f64 = 3.75;

impl KakshyaLord {
    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            KakshyaLord::Saturn => "Saturn",
            KakshyaLord::Jupiter => "Jupiter",
            KakshyaLord::Mars => "Mars",
            KakshyaLord::Sun => "Sun",
            KakshyaLord::Venus => "Venus",
            KakshyaLord::Mercury => "Mercury",
            KakshyaLord::Moon => "Moon",
            KakshyaLord::Lagna => "Lagna",
        }
    }

    /// Column of this lord in the Ashtakavarga reference order (Sun, Moon, Mars, Mercury,
    /// Jupiter, Venus, Saturn, Lagna).
    fn reference(self) -> usize {
        match self {
            KakshyaLord::Sun => 0,
            KakshyaLord::Moon => 1,
            KakshyaLord::Mars => 2,
            KakshyaLord::Mercury => 3,
            KakshyaLord::Jupiter => 4,
            KakshyaLord::Venus => 5,
            KakshyaLord::Saturn => 6,
            KakshyaLord::Lagna => 7,
        }
    }
}

/// Houses (1-based, counted from the reference) that receive a bindu from each graha,
/// indexed `[graha][reference]`: graha rows Sun, Moon, Mars, Mercury, Jupiter, Venus,
/// Saturn; reference columns Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Lagna.
/// This mirrors the benefic-position table of `kshana_vedic::ashtakavarga` (BPHS,
/// Ashtakavargadhyaya; row totals 48, 49, 39, 54, 56, 52, 39 = 337), kept here because
/// that crate exposes only per-sign totals and the kakshya needs each contributor; the
/// test `contributions_agree_with_kshana_vedic` pins it to the upstream table.
const CONTRIBUTIONS: [[&[u8]; 8]; 7] = [
    // Sun
    [
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[3, 6, 10, 11],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[3, 5, 6, 9, 10, 11, 12],
        &[5, 6, 9, 11],
        &[6, 7, 12],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[3, 4, 6, 10, 11, 12],
    ],
    // Moon
    [
        &[3, 6, 7, 8, 10, 11],
        &[1, 3, 6, 7, 10, 11],
        &[2, 3, 5, 6, 9, 10, 11],
        &[1, 3, 4, 5, 7, 8, 10, 11],
        &[1, 4, 7, 8, 10, 11, 12],
        &[3, 4, 5, 7, 9, 10, 11],
        &[3, 5, 6, 11],
        &[3, 6, 10, 11],
    ],
    // Mars
    [
        &[3, 5, 6, 10, 11],
        &[3, 6, 11],
        &[1, 2, 4, 7, 8, 10, 11],
        &[3, 5, 6, 11],
        &[6, 10, 11, 12],
        &[6, 8, 11, 12],
        &[1, 4, 7, 8, 9, 10, 11],
        &[1, 3, 6, 10, 11],
    ],
    // Mercury
    [
        &[5, 6, 9, 11, 12],
        &[2, 4, 6, 8, 10, 11],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[1, 3, 5, 6, 9, 10, 11, 12],
        &[6, 8, 11, 12],
        &[1, 2, 3, 4, 5, 8, 9, 11],
        &[1, 2, 4, 7, 8, 9, 10, 11],
        &[1, 2, 4, 6, 8, 10, 11],
    ],
    // Jupiter
    [
        &[1, 2, 3, 4, 7, 8, 9, 10, 11],
        &[2, 5, 7, 9, 11],
        &[1, 2, 4, 7, 8, 10, 11],
        &[1, 2, 4, 5, 6, 9, 10, 11],
        &[1, 2, 3, 4, 7, 8, 10, 11],
        &[2, 5, 6, 9, 10, 11],
        &[3, 5, 6, 12],
        &[1, 2, 4, 5, 6, 7, 9, 10, 11],
    ],
    // Venus
    [
        &[8, 11, 12],
        &[1, 2, 3, 4, 5, 8, 9, 11, 12],
        &[3, 5, 6, 9, 11, 12],
        &[3, 5, 6, 9, 11],
        &[5, 8, 9, 10, 11],
        &[1, 2, 3, 4, 5, 8, 9, 10, 11],
        &[3, 4, 5, 8, 9, 10, 11],
        &[1, 2, 3, 4, 5, 8, 9, 11],
    ],
    // Saturn
    [
        &[1, 2, 4, 7, 8, 10, 11],
        &[3, 6, 11],
        &[3, 5, 6, 10, 11, 12],
        &[6, 8, 9, 10, 11, 12],
        &[5, 6, 11, 12],
        &[6, 11, 12],
        &[3, 5, 6, 11],
        &[1, 3, 4, 6, 10, 11],
    ],
];

/// True when reference `r` gives graha row `g` a bindu in the sign that is `house` from it.
fn contributes(g: usize, r: usize, house: u8) -> bool {
    CONTRIBUTIONS[g][r].contains(&house)
}

/// Natal sign of an Ashtakavarga reference (column order Sun … Saturn, Lagna).
fn reference_sign(facts: &Facts, r: usize) -> u8 {
    let g = match r {
        0 => DashaLord::Sun,
        1 => DashaLord::Moon,
        2 => DashaLord::Mars,
        3 => DashaLord::Mercury,
        4 => DashaLord::Jupiter,
        5 => DashaLord::Venus,
        6 => DashaLord::Saturn,
        _ => return facts.lagna_sign,
    };
    facts.planet(g).sign
}

fn kakshya_cell(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / KAKSHYA_DEG).floor() as usize).min(95)
}

/// The kakshya a sidereal longitude falls in: `(sign 0–11, kakshya 0–7, lord)`.
pub fn kakshya_of(sidereal_longitude: f64) -> (u8, u8, KakshyaLord) {
    let c = kakshya_cell(sidereal_longitude);
    ((c / 8) as u8, (c % 8) as u8, KAKSHYA_LORDS[c % 8])
}

/// True when `lord` contributed a bindu to `planet`'s Bhinnashtakavarga in `sign`, so a
/// transit of `planet` through that lord's kakshya of `sign` gives its good (BPHS,
/// Ashtakavargadhyaya, the kakshya rule). Always false for Rahu and Ketu, which own no
/// Bhinnashtakavarga.
pub fn kakshya_favourable(facts: &Facts, planet: DashaLord, sign: u8, lord: KakshyaLord) -> bool {
    let Some(g) = bav_row(planet) else {
        return false;
    };
    let r = lord.reference();
    contributes(g, r, house_between(reference_sign(facts, r), sign % 12))
}

/// A graha's passage through one kakshya, with exact boundaries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KakshyaWindow {
    /// Entry (or `from` for the cell in progress).
    pub start: Instant,
    /// Exit (or `to` for the last cell).
    pub end: Instant,
    /// Sidereal sign, 0 = Aries.
    pub sign: u8,
    /// Kakshya within the sign, 0–7.
    pub kakshya: u8,
    /// Lord of the kakshya.
    pub kakshya_lord: KakshyaLord,
    /// True when the lord contributed a bindu to the graha's Bhinnashtakavarga in this sign.
    pub favourable: bool,
}

/// Scan step for the kakshya search, chosen so a step rarely holds more than one 3°45′
/// crossing (the finder bisects when it does).
fn kakshya_step(planet: DashaLord) -> i128 {
    match planet {
        DashaLord::Moon => 3 * 3_600 * NS_PER_SEC,
        DashaLord::Sun | DashaLord::Mercury | DashaLord::Venus => NS_PER_DAY,
        DashaLord::Mars => 2 * NS_PER_DAY,
        DashaLord::Jupiter => 5 * NS_PER_DAY,
        _ => 10 * NS_PER_DAY,
    }
}

/// Every kakshya `planet` passes through in `[from, to]`, contiguous, with each boundary
/// solved to the microsecond. The first window starts at `from` and the last ends at `to`.
/// Empty for Rahu and Ketu, which own no Bhinnashtakavarga to judge a kakshya by.
pub fn kakshya_windows(
    spk: &Spk,
    facts: &Facts,
    planet: DashaLord,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<KakshyaWindow>, SpkError> {
    if bav_row(planet).is_none() || from >= to {
        return Ok(Vec::new());
    }
    let f = |t: Instant| gochara_longitude(spk, planet, ayanamsa, t);
    let at_from = kakshya_cell(f(from)?);
    let crossings = uniform_crossings(&f, 96, from, to, kakshya_step(planet), TOL_NS)?;
    Ok(cells_to_windows(from, to, at_from, &crossings)
        .into_iter()
        .map(|(start, end, cell)| {
            let sign = (cell / 8) as u8;
            let lord = KAKSHYA_LORDS[cell % 8];
            KakshyaWindow {
                start,
                end,
                sign,
                kakshya: (cell % 8) as u8,
                kakshya_lord: lord,
                favourable: kakshya_favourable(facts, planet, sign, lord),
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------------------
// Crossing finder
// ---------------------------------------------------------------------------------------

/// Exact instants in `[from, to]` at which the angle `f(t)` (degrees) crosses a boundary
/// of the circle cut into `cells` equal cells, with the cell entered. Modelled on
/// `kshana_trace::crossings`: the scan steps `step_ns` at a time and solves a step that
/// holds one crossing with Brent's method; a step that holds more, or a turn, is bisected
/// until every piece holds at most one.
fn uniform_crossings<F>(
    f: &F,
    cells: usize,
    from: Instant,
    to: Instant,
    step_ns: i128,
    tol_ns: i128,
) -> Result<Vec<(Instant, usize)>, SpkError>
where
    F: Fn(Instant) -> Result<f64, SpkError>,
{
    let mut out = Vec::new();
    let mut a = from;
    let mut la = f(a)?;
    while a < to {
        let b = if a.add_ns(step_ns) > to {
            to
        } else {
            a.add_ns(step_ns)
        };
        let lb = f(b)?;
        uniform_piece(f, cells, a, la, b, lb, tol_ns, 0, &mut out)?;
        a = b;
        la = lb;
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn uniform_piece<F>(
    f: &F,
    cells: usize,
    a: Instant,
    la: f64,
    b: Instant,
    lb: f64,
    tol_ns: i128,
    depth: u32,
    out: &mut Vec<(Instant, usize)>,
) -> Result<(), SpkError>
where
    F: Fn(Instant) -> Result<f64, SpkError>,
{
    let width = 360.0 / cells as f64;
    let index = |l: f64| ((l.rem_euclid(360.0) / width).floor() as usize).min(cells - 1);
    let ia = index(la);
    let ib = index(lb);
    if ia == ib {
        return Ok(());
    }
    let forward = (ib + cells - ia) % cells;
    let backward = (ia + cells - ib) % cells;
    let motion = wrap180(lb - la);
    if (forward == 1 && motion > 0.0) || (backward == 1 && motion < 0.0) {
        // Direct motion enters cell `ib` across its lower edge; retrograde motion leaves
        // cell `ia` across its lower edge, entering `ib`.
        let boundary = if motion > 0.0 { ib } else { ia } as f64 * width;
        let g = |t: Instant| f(t).map(|l| wrap180(l - boundary)).unwrap_or(f64::NAN);
        if let Ok(root) = find_root(g, a, b, tol_ns) {
            out.push((root.t, ib));
        }
        return Ok(());
    }
    if depth > 40 || b.ns_since(a) <= 2 * tol_ns.max(1) {
        return Ok(());
    }
    let m = a.add_ns(b.ns_since(a) / 2);
    let lm = f(m)?;
    uniform_piece(f, cells, a, la, m, lm, tol_ns, depth + 1, out)?;
    uniform_piece(f, cells, m, lm, b, lb, tol_ns, depth + 1, out)
}

/// Turn a cell index in force at `from` and the crossings after it into contiguous
/// `(start, end, cell)` windows covering `[from, to]`.
fn cells_to_windows(
    from: Instant,
    to: Instant,
    at_from: usize,
    crossings: &[(Instant, usize)],
) -> Vec<(Instant, Instant, usize)> {
    let mut out = Vec::new();
    let mut start = from;
    let mut cell = at_from;
    for &(t, c) in crossings {
        if t <= from || t >= to || c == cell {
            continue;
        }
        out.push((start, t, cell));
        start = t;
        cell = c;
    }
    out.push((start, to, cell));
    out
}

// ---------------------------------------------------------------------------------------
// Transit windows and the score
// ---------------------------------------------------------------------------------------

/// A graha's stay in one sidereal sign, judged.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitWindow {
    /// The transiting graha.
    pub body: DashaLord,
    /// Sidereal sign, 0 = Aries.
    pub sign: u8,
    /// Start: the ingress, or `from` for the window already in progress.
    pub start: Instant,
    /// End: the next ingress, or `to` for the last window.
    pub end: Instant,
    /// House from the natal Moon, 1–12.
    pub house_from_moon: u8,
    /// House from the natal lagna (whole sign), 1–12.
    pub house_from_lagna: u8,
    /// Bhinnashtakavarga bindus of the graha in this sign (0 for the nodes).
    pub bav: u8,
    /// Sarvashtakavarga bindus of this sign.
    pub sav: u8,
    /// Classical verdict with vedha, judged on the other grahas' places at `start`.
    pub verdict: GocharaVerdict,
    /// Moorthi Nirnaya of the ingress that began this window; `None` when the window was
    /// already in progress at `from` and its ingress lies beyond the backward search.
    pub moorthi: Option<Moorthi>,
    /// Blended score in `[0, 1]`; see [`score_blend`].
    pub score: f64,
    /// The factors behind the score, in words.
    pub factors: Vec<String>,
}

/// The blend behind every score here. Weights: classical verdict 0.40 (favourable and
/// unobstructed 1.0, favourable but obstructed by vedha 0.4, unfavourable house 0.1);
/// Bhinnashtakavarga 0.25 (`bav / 8`, so the classical threshold of 4 is the midpoint);
/// Sarvashtakavarga 0.15 (`(sav − 20) / 16` clamped, so the mean of 28 is the midpoint
/// and 36 is full); Moorthi 0.10 ([`Moorthi::score`]); kakshya 0.10 (1 when the cell's
/// lord contributed a bindu, else 0). A term passed as `None` is left out and the rest
/// renormalised, so the result is always in `[0, 1]`.
pub fn score_blend(
    verdict: &GocharaVerdict,
    bav: Option<u8>,
    sav: u8,
    moorthi: Option<Moorthi>,
    kakshya: Option<bool>,
) -> f64 {
    let v = if verdict.favourable {
        1.0
    } else if verdict.vedha_by.is_some() {
        0.4
    } else {
        0.1
    };
    let mut num = 0.40 * v;
    let mut den = 0.40;
    if let Some(b) = bav {
        num += 0.25 * (f64::from(b) / 8.0).clamp(0.0, 1.0);
        den += 0.25;
    }
    num += 0.15 * ((f64::from(sav) - 20.0) / 16.0).clamp(0.0, 1.0);
    den += 0.15;
    if let Some(m) = moorthi {
        num += 0.10 * m.score();
        den += 0.10;
    }
    if let Some(k) = kakshya {
        num += 0.10 * if k { 1.0 } else { 0.0 };
        den += 0.10;
    }
    (num / den).clamp(0.0, 1.0)
}

/// The factors of a score in words, one per term in the blend.
#[allow(clippy::too_many_arguments)]
fn describe(
    planet: DashaLord,
    sign: u8,
    verdict: &GocharaVerdict,
    bav: Option<u8>,
    sav: u8,
    moorthi: Option<Moorthi>,
    kakshya: Option<(KakshyaLord, bool)>,
) -> Vec<String> {
    let who = name(planet);
    let sn = SIGN_NAMES[usize::from(sign % 12)];
    let mut out = vec![verdict.words.clone()];
    match bav {
        Some(b) if b >= 4 => out.push(format!(
            "Bhinnashtakavarga: {sn} holds {b} of {who}'s bindus (4 or more is good)"
        )),
        Some(b) => out.push(format!(
            "Bhinnashtakavarga: {sn} holds only {b} of {who}'s bindus (fewer than 4 is weak)"
        )),
        None => out.push(format!(
            "{who} owns no Bhinnashtakavarga; judged by the Sarvashtakavarga"
        )),
    }
    out.push(if sav >= 28 {
        format!("Sarvashtakavarga: {sn} holds {sav} bindus (28 or more is good)")
    } else {
        format!("Sarvashtakavarga: {sn} holds {sav} bindus (fewer than 28 withholds)")
    });
    if let Some(m) = moorthi {
        out.push(format!(
            "Moorthi Nirnaya: {} ({}) — the transit Moon's house from your Moon as {who} entered {sn}",
            m.name(),
            m.metal()
        ));
    }
    if let Some((lord, good)) = kakshya {
        out.push(if good {
            format!(
                "Kakshya: in {}'s kakshya of {sn}, which contributed a bindu to {who} here",
                lord.name()
            )
        } else {
            format!(
                "Kakshya: in {}'s kakshya of {sn}, which gave {who} no bindu here",
                lord.name()
            )
        });
    }
    out
}

/// How far before `from` to look for the ingress that began the window in progress: a
/// little more than the longest stay of each graha in one sign.
fn back_span_days(planet: DashaLord) -> i128 {
    match planet {
        DashaLord::Sun => 40,
        DashaLord::Moon => 4,
        DashaLord::Mercury => 120,
        DashaLord::Venus => 200,
        DashaLord::Mars => 300,
        DashaLord::Jupiter => 500,
        DashaLord::Saturn => 1_200,
        DashaLord::Rahu | DashaLord::Ketu => 700,
    }
}

/// Sidereal sign ingresses of a graha in `[from, to]`, as `(instant, sign entered)`.
/// The six planets use [`sidereal_ingresses`]; the Moon (which can cross a boundary
/// twice inside that function's ten-day step) and the nodes use the local finder.
fn ingresses(
    spk: &Spk,
    planet: DashaLord,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<(Instant, u8)>, SpkError> {
    ingresses_with(spk, planet, ayanamsa, lunar_node(), from, to)
}

/// Scan step for the true node. It is not monotonic: Meeus (*Astronomical Algorithms*
/// 2nd ed., ch. 47) gives true − mean = −1.4979° sin 2(D−F) − 0.1500° sin M
/// − 0.1226° sin 2D + 0.1176° sin 2F − 0.0801° sin 2(F−M′), whose fortnightly terms
/// make it run direct for a few days at a time, so it can cross one boundary three
/// times. Summing amplitude × ω² over those terms bounds its acceleration by
/// ≈ 0.049°/day². Between two samples 6 h apart, a back-and-forth pair of crossings
/// that leaves both samples on the same side reaches at most ½·0.049·(0.125 d)² ≈ 1.4″
/// past the boundary (a sliver of under six hours): every crossing that matters lands
/// in a step of its own, where it is bisected and solved.
const TRUE_NODE_STEP_NS: i128 = 6 * 3_600 * NS_PER_SEC;

/// [`ingresses`] with the node named explicitly.
fn ingresses_with(
    spk: &Spk,
    planet: DashaLord,
    ayanamsa: Ayanamsa,
    node: LunarNode,
    from: Instant,
    to: Instant,
) -> Result<Vec<(Instant, u8)>, SpkError> {
    match body_of(planet) {
        Some(body) if planet != DashaLord::Moon => {
            sidereal_ingresses(spk, body, ayanamsa, from, to)
        }
        _ => {
            let f = |t: Instant| gochara_longitude_with(spk, planet, ayanamsa, node, t);
            let step = if planet == DashaLord::Moon {
                6 * 3_600 * NS_PER_SEC
            } else if node == LunarNode::True {
                TRUE_NODE_STEP_NS
            } else {
                // the mean node is monotonic (−0.053°/day): one crossing per 10 days at most
                10 * NS_PER_DAY
            };
            Ok(uniform_crossings(&f, 12, from, to, step, TOL_NS)?
                .into_iter()
                .map(|(t, c)| (t, c as u8))
                .collect())
        }
    }
}

/// Every sign transit of the nine grahas over `[from, to]`, grouped by graha in
/// [`GOCHARA_PLANETS`] order and contiguous within each graha: the first window starts at
/// `from` with the sign in force then, each later one at an exact ingress, the last ends
/// at `to`. Rahu and Ketu move by the engine-wide node. Each window carries the classical verdict
/// (vedha judged on the other grahas' places at the window's start), the Ashtakavarga
/// bindus, the Moorthi of its ingress (searched back a little beyond one stay for the
/// window in progress) and the blended score of [`score_blend`].
pub fn transit_windows(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<TransitWindow>, SpkError> {
    let facts = Facts::from_chart(natal);
    let natal_moon = natal_moon_sidereal(natal);
    let moon_sign = sign_of(natal_moon);
    let lagna_sign = sign_of(natal.cusps_sidereal[0]);
    let mut out = Vec::new();
    if from >= to {
        return Ok(out);
    }
    for planet in GOCHARA_PLANETS {
        let back = from.add_ns(-back_span_days(planet) * NS_PER_DAY);
        let all = ingresses(spk, planet, ayanamsa, back, to)?;
        let sign_at_from = sign_of(gochara_longitude(spk, planet, ayanamsa, from)?);
        let first_ingress = all.iter().rev().find(|(t, _)| *t <= from).map(|(t, _)| *t);
        let later: Vec<(Instant, usize)> = all
            .iter()
            .filter(|(t, _)| *t > from)
            .map(|(t, s)| (*t, usize::from(*s)))
            .collect();
        let cells = cells_to_windows(from, to, usize::from(sign_at_from), &later);
        for (i, (start, end, cell)) in cells.into_iter().enumerate() {
            let sign = cell as u8;
            let ingress = if i == 0 { first_ingress } else { Some(start) };
            let moorthi = match ingress {
                Some(t) => Some(moorthi(spk, natal_moon, ayanamsa, t)?),
                None => None,
            };
            let occupants: Vec<(DashaLord, u8)> = positions_at(spk, ayanamsa, start)?
                .into_iter()
                .map(|(g, l)| (g, house_between(moon_sign, sign_of(l))))
                .collect();
            let house_from_moon = house_between(moon_sign, sign);
            let house_from_lagna = house_between(lagna_sign, sign);
            let verdict = gochara_verdict(planet, house_from_moon, &occupants);
            let (bav, sav) = transit_bindus(&facts, planet, sign);
            let bav_opt = bav_row(planet).map(|_| bav);
            let score = score_blend(&verdict, bav_opt, sav, moorthi, None);
            let factors = describe(planet, sign, &verdict, bav_opt, sav, moorthi, None);
            out.push(TransitWindow {
                body: planet,
                sign,
                start,
                end,
                house_from_moon,
                house_from_lagna,
                bav,
                sav,
                verdict,
                moorthi,
                score,
                factors,
            });
        }
    }
    Ok(out)
}

/// The sky and you, for one instant: each of the nine grahas judged where it stands at
/// `at` (classical verdict with vedha among the nine, Bhinnashtakavarga, Sarvashtakavarga
/// and the kakshya it is moving through; no Moorthi, which belongs to an ingress), blended
/// by [`score_blend`]. Returns the plain mean of the nine scores and one line per graha.
pub fn today_transit_score(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<(f64, Vec<String>), SpkError> {
    let facts = Facts::from_chart(natal);
    let moon_sign = sign_of(natal_moon_sidereal(natal));
    let positions = positions_at(spk, ayanamsa, at)?;
    let occupants: Vec<(DashaLord, u8)> = positions
        .iter()
        .map(|(g, l)| (*g, house_between(moon_sign, sign_of(*l))))
        .collect();
    let mut total = 0.0;
    let mut lines = Vec::with_capacity(positions.len());
    for (g, lon) in &positions {
        let sign = sign_of(*lon);
        let house = house_between(moon_sign, sign);
        let verdict = gochara_verdict(*g, house, &occupants);
        let (bav, sav) = transit_bindus(&facts, *g, sign);
        let bav_opt = bav_row(*g).map(|_| bav);
        let (_, _, lord) = kakshya_of(*lon);
        let kakshya = bav_row(*g).map(|_| kakshya_favourable(&facts, *g, sign, lord));
        let score = score_blend(&verdict, bav_opt, sav, None, kakshya);
        total += score;
        let factors = describe(
            *g,
            sign,
            &verdict,
            bav_opt,
            sav,
            None,
            kakshya.map(|k| (lord, k)),
        );
        lines.push(format!(
            "{} in {} ({} from your Moon), {score:.2}: {}",
            name(*g),
            SIGN_NAMES[usize::from(sign)],
            ordinal(house),
            factors.join("; ")
        ));
    }
    Ok((total / positions.len() as f64, lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_chart::{compute, ChartInput};
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;
    use kshana_vedic::{bhinna, AshtakavargaPositions};
    use DashaLord::*;

    fn fixture() -> Option<(Spk, Chart)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 10, 29, 17, 36, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: birth,
                observer: obs,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        Some((spk, ch))
    }

    fn utc(y: i64, m: u32, d: u32, h: u32) -> Instant {
        Utc::new(y, m, d, h, 0, 0, 0).unwrap().to_instant().unwrap()
    }

    #[test]
    fn vedha_table_matches_phaladeepika_26() {
        let expect: [(DashaLord, &[(u8, u8)]); 7] = [
            (Sun, &[(3, 9), (6, 12), (10, 4), (11, 5)]),
            (Moon, &[(1, 5), (3, 9), (6, 12), (7, 2), (10, 4), (11, 8)]),
            (Mars, &[(3, 12), (6, 9), (11, 5)]),
            (
                Mercury,
                &[(2, 5), (4, 3), (6, 9), (8, 1), (10, 8), (11, 12)],
            ),
            (Jupiter, &[(2, 12), (5, 4), (7, 3), (9, 10), (11, 8)]),
            (
                Venus,
                &[
                    (1, 8),
                    (2, 7),
                    (3, 1),
                    (4, 10),
                    (5, 9),
                    (8, 5),
                    (9, 11),
                    (11, 6),
                    (12, 3),
                ],
            ),
            (Saturn, &[(3, 12), (6, 9), (11, 5)]),
        ];
        for (g, pairs) in expect {
            assert_eq!(vedha_table(g), pairs, "{g:?}");
        }
        assert_eq!(vedha_table(Rahu), vedha_table(Saturn));
        assert_eq!(vedha_table(Ketu), vedha_table(Saturn));
        for g in GOCHARA_PLANETS {
            let t = vedha_table(g);
            let fav: Vec<u8> = t.iter().map(|p| p.0).collect();
            let ved: Vec<u8> = t.iter().map(|p| p.1).collect();
            for (h, v) in t {
                assert!((1..=12).contains(h) && (1..=12).contains(v));
                assert_ne!(h, v, "{g:?}: a house cannot be its own vedha");
            }
            let mut f2 = fav.clone();
            f2.sort_unstable();
            f2.dedup();
            assert_eq!(f2.len(), fav.len(), "{g:?}: favourable houses are distinct");
            let mut v2 = ved.clone();
            v2.sort_unstable();
            v2.dedup();
            assert_eq!(v2.len(), ved.len(), "{g:?}: vedha houses are distinct");
        }
    }

    #[test]
    fn vedha_is_obstructed_except_for_the_exempt_pairs_and_self() {
        let v = gochara_verdict(Sun, 3, &[(Saturn, 9)]);
        assert!(v.favourable && v.vedha_by.is_none(), "{}", v.words);
        let v = gochara_verdict(Sun, 3, &[(Mars, 9)]);
        assert!(!v.favourable);
        assert_eq!(v.vedha_by, Some(Mars));
        assert!(v.words.contains("Mars in the 9th"));
        let v = gochara_verdict(Saturn, 11, &[(Sun, 5)]);
        assert!(v.favourable);
        let v = gochara_verdict(Moon, 7, &[(Mercury, 2)]);
        assert!(v.favourable);
        let v = gochara_verdict(Mercury, 8, &[(Moon, 1)]);
        assert!(v.favourable);
        let v = gochara_verdict(Mercury, 8, &[(Jupiter, 1)]);
        assert_eq!(v.vedha_by, Some(Jupiter));
        let v = gochara_verdict(Venus, 1, &[(Venus, 8)]);
        assert!(v.favourable, "a graha never obstructs itself");
        let v = gochara_verdict(Jupiter, 8, &[(Mars, 1), (Saturn, 12)]);
        assert!(!v.favourable && v.vedha_by.is_none());
        assert!(v.words.contains("not a favourable house"));
        let v = gochara_verdict(Rahu, 6, &[(Ketu, 12), (Sun, 9)]);
        assert_eq!(v.vedha_by, Some(Sun));
        assert!(vedha_exempt(Sun, Saturn) && vedha_exempt(Mercury, Moon));
        assert!(!vedha_exempt(Sun, Mercury) && !vedha_exempt(Rahu, Ketu));
    }

    #[test]
    fn contributions_agree_with_kshana_vedic() {
        let sets: [[u8; 8]; 5] = [
            [0, 1, 2, 3, 4, 5, 6, 7],
            [6, 7, 3, 5, 3, 5, 11, 9],
            [11, 4, 8, 1, 2, 9, 0, 6],
            [5, 5, 5, 5, 5, 5, 5, 5],
            [3, 10, 0, 7, 11, 2, 8, 1],
        ];
        for signs in sets {
            let p = AshtakavargaPositions { signs };
            for g in 0..7 {
                let b = bhinna(&p, g);
                for s in 0..12u8 {
                    let n = (0..8)
                        .filter(|&r| contributes(g, r, house_between(signs[r], s)))
                        .count();
                    assert_eq!(n as u8, b[usize::from(s)], "graha {g} sign {s} {signs:?}");
                }
            }
        }
        for (g, rows) in CONTRIBUTIONS.iter().enumerate() {
            let total: usize = rows.iter().map(|r| r.len()).sum();
            assert_eq!(total as u32, kshana_vedic::ROW_TOTALS[g]);
        }
    }

    #[test]
    fn kakshya_lords_and_cells() {
        assert_eq!(kakshya_of(0.0), (0, 0, KakshyaLord::Saturn));
        assert_eq!(kakshya_of(3.75), (0, 1, KakshyaLord::Jupiter));
        assert_eq!(kakshya_of(29.99), (0, 7, KakshyaLord::Lagna));
        assert_eq!(kakshya_of(30.0), (1, 0, KakshyaLord::Saturn));
        assert_eq!(kakshya_of(359.999), (11, 7, KakshyaLord::Lagna));
        assert_eq!(kakshya_of(-1.0), (11, 7, KakshyaLord::Lagna));
        assert_eq!(KAKSHYA_LORDS.len(), 8);
        assert_eq!(KAKSHYA_DEG * 8.0, 30.0);
    }

    #[test]
    fn uniform_crossings_finds_every_cell_of_a_synthetic_mover() {
        // 13.2°/day like the Moon over 1.2 days from 0.01°: 15.84° = four 3°45′ boundaries.
        let from = Instant::j2000();
        let to = from.add_ns(NS_PER_DAY * 12 / 10);
        let f = |t: Instant| Ok(0.01 + 13.2 * (t.ns_since(from) as f64) / 86_400e9);
        let c = uniform_crossings(&f, 96, from, to, 2 * 3_600 * NS_PER_SEC, TOL_NS).unwrap();
        assert_eq!(c.len(), 4);
        for (i, (t, cell)) in c.iter().enumerate() {
            assert_eq!(*cell, i + 1);
            let want = (i as f64 + 1.0) * 3.75 - 0.01;
            let got = 13.2 * (t.ns_since(from) as f64) / 86_400e9;
            assert!((got - want).abs() < 1e-6, "{got} vs {want}");
        }
        // A coarse step holding several crossings is bisected, not skipped.
        let c2 = uniform_crossings(&f, 96, from, to, 2 * NS_PER_DAY, TOL_NS).unwrap();
        assert_eq!(c2.len(), 4);
        // Retrograde motion enters the lower cell.
        let g = |t: Instant| Ok(31.0 - 0.5 * (t.ns_since(from) as f64) / 86_400e9);
        let r = uniform_crossings(
            &g,
            12,
            from,
            from.add_ns(10 * NS_PER_DAY),
            NS_PER_DAY,
            TOL_NS,
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, 0);
        assert!((r[0].0.ns_since(from) as f64 / 86_400e9 - 2.0).abs() < 1e-6);
    }

    #[test]
    fn moorthi_follows_the_transit_moon() {
        assert_eq!(Moorthi::from_house(1), Moorthi::Swarna);
        assert_eq!(Moorthi::from_house(6), Moorthi::Swarna);
        assert_eq!(Moorthi::from_house(11), Moorthi::Swarna);
        assert_eq!(Moorthi::from_house(2), Moorthi::Rajata);
        assert_eq!(Moorthi::from_house(5), Moorthi::Rajata);
        assert_eq!(Moorthi::from_house(9), Moorthi::Rajata);
        assert_eq!(Moorthi::from_house(3), Moorthi::Tamra);
        assert_eq!(Moorthi::from_house(7), Moorthi::Tamra);
        assert_eq!(Moorthi::from_house(10), Moorthi::Tamra);
        assert_eq!(Moorthi::from_house(4), Moorthi::Loha);
        assert_eq!(Moorthi::from_house(8), Moorthi::Loha);
        assert_eq!(Moorthi::from_house(12), Moorthi::Loha);
        let Some((spk, ch)) = fixture() else {
            return;
        };
        let natal_moon = natal_moon_sidereal(&ch);
        for d in [1u32, 8, 15, 22] {
            let t = utc(2026, 10, d, 9);
            let m = moorthi(&spk, natal_moon, Ayanamsa::Lahiri, t).unwrap();
            let tm = gochara_longitude(&spk, Moon, Ayanamsa::Lahiri, t).unwrap();
            let h = house_between(sign_of(natal_moon), sign_of(tm));
            assert_eq!(m, Moorthi::from_house(h));
            assert!(matches!(
                m,
                Moorthi::Swarna | Moorthi::Rajata | Moorthi::Tamra | Moorthi::Loha
            ));
            assert!((0.0..=1.0).contains(&m.score()));
        }
    }

    #[test]
    fn moon_kakshya_windows_are_contiguous_and_each_3_45_wide() {
        let Some((spk, ch)) = fixture() else {
            return;
        };
        let facts = Facts::from_chart(&ch);
        let from = utc(2026, 10, 15, 9);
        let to = from.add_ns(2 * NS_PER_DAY);
        let w = kakshya_windows(&spk, &facts, Moon, Ayanamsa::Lahiri, from, to).unwrap();
        // ~26° in two days: six to eight cells
        assert!((6..=9).contains(&w.len()), "{}", w.len());
        assert_eq!(w[0].start, from);
        assert_eq!(w.last().unwrap().end, to);
        let lon = |t: Instant| gochara_longitude(&spk, Moon, Ayanamsa::Lahiri, t).unwrap();
        let cell = |x: &KakshyaWindow| usize::from(x.sign) * 8 + usize::from(x.kakshya);
        for pair in w.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
            assert_eq!((cell(&pair[0]) + 1) % 96, cell(&pair[1]));
        }
        for x in &w {
            let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
            assert_eq!(kakshya_of(lon(mid)), (x.sign, x.kakshya, x.kakshya_lord));
            assert_eq!(x.kakshya_lord, KAKSHYA_LORDS[usize::from(x.kakshya)]);
            assert_eq!(
                x.favourable,
                kakshya_favourable(&facts, Moon, x.sign, x.kakshya_lord)
            );
        }
        for x in &w[1..w.len() - 1] {
            let width = wrap180(lon(x.end) - lon(x.start));
            assert!((width - KAKSHYA_DEG).abs() < 1e-4, "{width}");
            // the boundaries themselves sit on a 3°45′ grid
            let edge = lon(x.start).rem_euclid(KAKSHYA_DEG);
            assert!(!(1e-4..=KAKSHYA_DEG - 1e-4).contains(&edge), "{edge}");
        }
        assert!(w.iter().any(|x| x.favourable) || w.iter().any(|x| !x.favourable));
        assert!(
            kakshya_windows(&spk, &facts, Rahu, Ayanamsa::Lahiri, from, to)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn transit_windows_over_2026_are_contiguous_and_scored() {
        let Some((spk, ch)) = fixture() else {
            return;
        };
        let from = utc(2026, 1, 1, 0);
        let to = utc(2027, 1, 1, 0);
        let all = transit_windows(&spk, &ch, Ayanamsa::Lahiri, from, to).unwrap();
        let moon_sign = sign_of(natal_moon_sidereal(&ch));
        let lagna_sign = sign_of(ch.cusps_sidereal[0]);
        for planet in GOCHARA_PLANETS {
            let w: Vec<&TransitWindow> = all.iter().filter(|x| x.body == planet).collect();
            assert!(!w.is_empty(), "{planet:?}");
            assert_eq!(w[0].start, from, "{planet:?}");
            assert_eq!(w.last().unwrap().end, to, "{planet:?}");
            for pair in w.windows(2) {
                assert_eq!(pair[0].end, pair[1].start, "{planet:?}");
                assert_ne!(pair[0].sign, pair[1].sign, "{planet:?}");
                assert!(
                    pair[1].moorthi.is_some(),
                    "{planet:?}: an ingress has a Moorthi"
                );
            }
            for x in &w {
                assert!((0.0..=1.0).contains(&x.score), "{planet:?} {}", x.score);
                assert!((1..=12).contains(&x.house_from_moon));
                assert!((1..=12).contains(&x.house_from_lagna));
                assert_eq!(x.house_from_moon, house_between(moon_sign, x.sign));
                assert_eq!(x.house_from_lagna, house_between(lagna_sign, x.sign));
                assert!(x.sign < 12);
                assert!(x.sav <= 56 && x.bav <= 8);
                assert!(x.factors.len() >= 3);
                let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
                let l = gochara_longitude(&spk, planet, Ayanamsa::Lahiri, mid).unwrap();
                assert_eq!(sign_of(l), x.sign, "{planet:?} mid-window sign");
                if matches!(planet, Rahu | Ketu) {
                    assert_eq!(x.bav, 0);
                }
            }
            match planet {
                Saturn => assert!(w.len() <= 2, "Saturn {}", w.len()),
                Jupiter => assert!(w.len() <= 3, "Jupiter {}", w.len()),
                Sun => assert!((12..=14).contains(&w.len()), "Sun {}", w.len()),
                Moon => assert!(w.len() > 100, "Moon {}", w.len()),
                Rahu | Ketu => assert!(w.len() <= 2, "{planet:?} {}", w.len()),
                _ => {}
            }
            // the window in progress at `from` found its ingress within the back span
            assert!(w[0].moorthi.is_some(), "{planet:?}");
        }
        // Rahu and Ketu are always opposite
        let r: Vec<&TransitWindow> = all.iter().filter(|x| x.body == Rahu).collect();
        let k: Vec<&TransitWindow> = all.iter().filter(|x| x.body == Ketu).collect();
        assert_eq!(r.len(), k.len());
        for (a, b) in r.iter().zip(k.iter()) {
            assert_eq!((a.sign + 6) % 12, b.sign);
            assert_eq!(a.start, b.start);
        }
    }

    fn utc_s(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> Instant {
        Utc::new(y, m, d, h, mi, s, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }

    /// Rahu's sidereal (Lahiri) sign ingresses 2023–2026, both nodes, against Swiss
    /// Ephemeris 2.10.03 (`swe_calc_ut(SE_TRUE_NODE | SE_MEAN_NODE, SEFLG_SWIEPH |
    /// SEFLG_SIDEREAL)` with `SE_SIDM_LAHIRI` and the semo_18/sepl_18 files, scanned
    /// hourly and bisected to 1 ms). The true node is sensitive to the Moon's velocity
    /// (≈ 1/sin 5° amplification): with the Moshier ephemeris instead, SE's true-node
    /// ingresses come 18 to 89 minutes later, so the files matter. Kshana (DE421) lands
    /// within seconds of SE's file ephemeris. The mean-node dates are the ones printed
    /// panchangas give: Rahu into Meena on 30 October 2023 and into Kumbha on
    /// 18 May 2025. The true node reaches each boundary 10 to 30 days apart from the mean.
    #[test]
    fn rahu_sign_ingresses_match_swiss_ephemeris_for_both_nodes() {
        let Some((spk, _)) = fixture() else {
            return;
        };
        let from = utc(2023, 1, 1, 0);
        let to = utc(2027, 1, 1, 0);
        let cases = [
            (
                LunarNode::True,
                [
                    (utc_s(2023, 11, 28, 20, 58, 36), 11u8),
                    (utc_s(2025, 5, 29, 18, 47, 37), 10),
                    (utc_s(2026, 11, 25, 12, 16, 7), 9),
                ],
                60.0,
            ),
            (
                LunarNode::Mean,
                [
                    (utc_s(2023, 10, 30, 11, 6, 58), 11u8),
                    (utc_s(2025, 5, 18, 14, 4, 33), 10),
                    (utc_s(2026, 12, 5, 17, 2, 10), 9),
                ],
                60.0,
            ),
        ];
        for (node, want, tol_s) in cases {
            let got = ingresses_with(&spk, Rahu, Ayanamsa::Lahiri, node, from, to).unwrap();
            assert_eq!(got.len(), want.len(), "{node:?}: {got:?}");
            for ((t, sign), (wt, wsign)) in got.iter().zip(want.iter()) {
                assert_eq!(sign, wsign, "{node:?}");
                let off = t.ns_since(*wt) as f64 / 1e9;
                eprintln!("{node:?} Rahu into sign {sign}: {off:+.0} s from Swiss Ephemeris");
                assert!(off.abs() < tol_s, "{node:?} sign {sign}: off by {off:.0} s");
            }
        }
    }

    /// The true node is not monotonic, so a scan over a whole year must find every
    /// nakshatra boundary it crosses, in either direction, with the longitude on the
    /// right side of each solved instant.
    #[test]
    fn true_node_nakshatra_crossings_are_all_found_both_ways() {
        let Some((spk, _)) = fixture() else {
            return;
        };
        let from = utc(2025, 1, 1, 0);
        let to = utc(2026, 1, 1, 0);
        let f =
            |t: Instant| gochara_longitude_with(&spk, Rahu, Ayanamsa::Lahiri, LunarNode::True, t);
        let got = uniform_crossings(&f, 27, from, to, TRUE_NODE_STEP_NS, TOL_NS).unwrap();
        // brute force: hourly samples, every change of nakshatra
        let cell = |l: f64| (l / (360.0 / 27.0)).floor() as usize;
        let mut brute = Vec::new();
        let mut t = from;
        let mut c = cell(f(t).unwrap());
        while t < to {
            let n = t.add_ns(3_600 * NS_PER_SEC);
            let cn = cell(f(n).unwrap());
            if cn != c {
                brute.push((n, cn));
            }
            t = n;
            c = cn;
        }
        assert_eq!(got.len(), brute.len(), "{got:?} vs {brute:?}");
        for ((t, c), (bt, bc)) in got.iter().zip(brute.iter()) {
            assert_eq!(c, bc);
            assert!(bt.ns_since(*t) >= 0 && bt.ns_since(*t) <= 3_600 * NS_PER_SEC);
            let before = cell(f(t.add_ns(-1_000_000)).unwrap());
            let after = cell(f(t.add_ns(1_000_000)).unwrap());
            assert_ne!(before, after);
            assert_eq!(after, *c);
        }
    }

    /// Natal Rahu and transit Rahu are the same point at the same instant, for either node.
    #[test]
    fn natal_and_transit_rahu_agree() {
        let Some((spk, ch)) = fixture() else {
            return;
        };
        let natal = ch.rahu().1;
        let transit = gochara_longitude(&spk, Rahu, Ayanamsa::Lahiri, ch.instant).unwrap();
        assert!(
            wrap180(natal - transit).abs() < 1e-9,
            "{natal} vs {transit}"
        );
        let ketu = gochara_longitude(&spk, Ketu, Ayanamsa::Lahiri, ch.instant).unwrap();
        assert!(wrap180(natal + 180.0 - ketu).abs() < 1e-9);
        for (node, natal) in [
            (LunarNode::True, ch.true_node.1),
            (LunarNode::Mean, ch.mean_node.1),
        ] {
            let t = gochara_longitude_with(&spk, Rahu, Ayanamsa::Lahiri, node, ch.instant).unwrap();
            assert!(wrap180(natal - t).abs() < 1e-9, "{node:?}: {natal} vs {t}");
        }
    }

    #[test]
    fn today_score_is_bounded_with_a_line_per_graha() {
        let Some((spk, ch)) = fixture() else {
            return;
        };
        let at = utc(2026, 10, 15, 9);
        let (s, lines) = today_transit_score(&spk, &ch, Ayanamsa::Lahiri, at).unwrap();
        assert!((0.0..=1.0).contains(&s), "{s}");
        assert_eq!(lines.len(), 9);
        for (g, line) in GOCHARA_PLANETS.iter().zip(lines.iter()) {
            assert!(line.starts_with(name(*g)), "{line}");
            assert!(line.contains("from your Moon"), "{line}");
            assert!(line.contains("Sarvashtakavarga"), "{line}");
        }
        assert!(lines[6].contains("Kakshya"), "{}", lines[6]);
        assert!(!lines[7].contains("Kakshya"), "{}", lines[7]);
        // the blend is bounded and monotone in its terms
        let good = GocharaVerdict {
            favourable: true,
            vedha_by: None,
            words: String::new(),
        };
        let bad = GocharaVerdict {
            favourable: false,
            vedha_by: None,
            words: String::new(),
        };
        assert_eq!(
            score_blend(&good, Some(8), 36, Some(Moorthi::Swarna), Some(true)),
            1.0
        );
        assert!(score_blend(&bad, Some(0), 20, Some(Moorthi::Loha), Some(false)) < 0.1);
        assert!(score_blend(&good, None, 28, None, None) > score_blend(&bad, None, 28, None, None));
        assert!(
            score_blend(&good, Some(6), 28, None, None)
                > score_blend(&good, Some(2), 28, None, None)
        );
    }
}
