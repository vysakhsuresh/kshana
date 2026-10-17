//! Two charts together: the Ashtakoota match (Guna Milan), Kuja dosha, the Moon-to-Moon
//! synastry, and how the two stand at an instant and over the months ahead.
//!
//! The eight kootas are the classical tables of Muhurta Chintamani and the Kerala and
//! Tamil matching tradition, keyed to the two Moons (sign and nakshatra). Conventions
//! are stated where schools differ. Nothing here refuses a pair: a low total is read,
//! explained and dated like everything else.

use crate::ask::{answer, Answer, Question};
use crate::briefing::{briefing, Briefing};
use crate::explain::{explain, Relief};
use crate::forecast::{date_words, moment, slow_series, Band, ForecastError, Life, Natal, SIGN_NAMES};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::parashari::{dignity, natural_relation, Dignity};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, sign_lord, Ayanamsa, DashaLord};

/// One koota.
#[derive(Clone, Debug, PartialEq)]
pub struct Koota {
    /// Name.
    pub name: &'static str,
    /// Points given.
    pub points: f64,
    /// Points possible.
    pub max: f64,
    /// Why, in words.
    pub words: String,
    /// Source.
    pub source: &'static str,
}

/// The match of two birth charts.
#[derive(Clone, Debug, PartialEq)]
pub struct Kundali {
    /// Total out of 36.
    pub total: f64,
    /// The eight kootas, in the classical order.
    pub kootas: Vec<Koota>,
    /// `Excellent match`, `Very good`, `Good enough`, `Needs work`, `Hard match`.
    pub verdict: String,
    /// One or two sentences.
    pub headline: String,
    /// Kuja dosha in the first chart.
    pub kuja_a: bool,
    /// Kuja dosha in the second chart.
    pub kuja_b: bool,
    /// Kuja dosha, in words.
    pub kuja_words: String,
    /// "Pisces · Revati" for the first Moon.
    pub moon_a: String,
    /// For the second Moon.
    pub moon_b: String,
    /// The second Moon's house from the first, 1–12.
    pub moon_distance: u8,
    /// What binds, in words.
    pub strengths: Vec<String>,
    /// What rubs, in words.
    pub frictions: Vec<String>,
    /// The classical remedies for the doshas the match carries.
    pub remedies: Vec<DoshaRemedy>,
}

/// A remedy for a dosha of the match, from the Vivaha tradition.
#[derive(Clone, Debug, PartialEq)]
pub struct DoshaRemedy {
    /// `Nadi`, `Bhakoot`, `Gana`, `Kuja`, `Graha Maitri`, `Yoni`.
    pub dosha: &'static str,
    /// What to do.
    pub title: &'static str,
    /// Why, in a sentence.
    pub why: &'static str,
    /// Where it comes from.
    pub source: &'static str,
    /// True for ritual items: supportive, never required.
    pub optional: bool,
}

const VIVAHA: &str = "Vivaha prakarana of Muhurta Chintamani and the Kerala and Tamil matching tradition";
const DHARMA_SINDHU: &str = "Dharma Sindhu, Nirnaya Sindhu (dosha parihara)";
const LAL_KITAB: &str = "Lal Kitab";

fn dosha_remedies(nadi: bool, bhakoot: bool, gana: bool, kuja_unbalanced: bool, maitri_low: bool, yoni_low: bool) -> Vec<DoshaRemedy> {
    let mut v = Vec::new();
    if nadi {
        v.push(DoshaRemedy { dosha: "Nadi", title: "Mahamrityunjaya japa together, 108 a day for 40 days", why: "The Nadi dosha touches health and the line of children; the Mrityunjaya mantra is the classical shield for both.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Nadi", title: "Nadi dosha nivarana: feed and clothe the poor on a Monday, and give grain in the Moon's name", why: "The pulse (nadi) is the Moon's; its dosha is paid down in the Moon's currency, food and care.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Nadi", title: "Keep the body's rhythm: shared meals, shared sleep hours, no all-nighters", why: "Same-nadi pairs tire the same way at the same time; a kept routine is the daily remedy.", source: "Dharma reading of the dosha", optional: false });
    }
    if bhakoot {
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Lakshmi-Narayana or Gauri-Shankara pooja on a Friday or Monday", why: "The Bhakoot dosha is a quarrel of the two Moons; the divine couple is worshipped to reconcile them.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Strengthen the weaker Moon: water, rice and white cloth given on Mondays; a Monday fast kept lightly", why: "A sound Moon rides out a hard Moon-to-Moon angle; the texts treat the dosha as half-cancelled when the Moon is strong.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Decide money matters only after sleeping on them, and never during a quarrel", why: "The 2/12, 5/9 and 6/8 angles work through money, children and sudden turns; a one-night rule takes the heat out.", source: "Dharma reading of the dosha", optional: false });
    }
    if gana {
        v.push(DoshaRemedy { dosha: "Gana", title: "Gauri pooja, and Durga Saptashati read once a month (or heard)", why: "A gentle and a fierce temper under one roof: Gauri and Durga are the two tempers made one.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Gana", title: "A rule of words: no verdicts after nine at night, and the sharp word apologised for before sleep", why: "The Gana dosha shows as sharp words at the wrong hour; the remedy is a rule about hours and words.", source: "Dharma reading of the dosha", optional: false });
    }
    if kuja_unbalanced {
        v.push(DoshaRemedy { dosha: "Kuja", title: "Mangala shanti: Hanuman Chalisa or the Mangala stotra on Tuesdays, and red lentils given", why: "Mars in the houses of the home and the partner heats them; the shanti cools the graha that carries the dosha.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Kuja", title: "Mangala Gauri vrata on the Tuesdays of Shravana, or any four Tuesdays", why: "The vrata the tradition prescribes for the Mars-touched partner, kept for the marriage.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Kuja", title: "Spend Mars: hard exercise, repairs, a kitchen garden, anything that burns heat with the hands", why: "The dosha is unspent Mars; spent on effort it does not spend itself on the partner.", source: LAL_KITAB, optional: false });
        v.push(DoshaRemedy { dosha: "Kuja", title: "The tradition also names a Kumbha vivaha (a symbolic first wedding) before the marriage; many families keep it, many do not", why: "Stated for completeness; it is a rite of the family's choosing, not a requirement of the chart.", source: VIVAHA, optional: true });
    }
    if maitri_low {
        v.push(DoshaRemedy { dosha: "Graha Maitri", title: "Each of you keeps the other's Moon lord's day: a small act in its colour and grain", why: "Enemy Moon lords are made friends by serving each other's lord; the day, the colour and the grain are the lord's.", source: LAL_KITAB, optional: false });
        v.push(DoshaRemedy { dosha: "Graha Maitri", title: "Say the hard thing in writing first, then talk", why: "When the minds pull apart, the written word gives each mind its own hour to understand.", source: "Dharma reading of the dosha", optional: false });
    }
    if yoni_low {
        v.push(DoshaRemedy { dosha: "Yoni", title: "Rati-Kama or Uma-Maheshwara worship on a Friday", why: "The Yoni koota is the body's ease; the tradition turns to the divine pair for it.", source: VIVAHA, optional: true });
        v.push(DoshaRemedy { dosha: "Yoni", title: "Unhurried time without a screen, at the same hour, twice a week", why: "Enemy yonis need time to learn each other; the remedy is kept time, not a rite.", source: "Dharma reading of the dosha", optional: false });
    }
    v
}

const MC: &str = "Muhurta Chintamani, Vivaha prakarana; Kerala and Tamil matching tables";

/// Varna by Moon sign: 3 Brahmin (water), 2 Kshatriya (fire), 1 Vaishya (earth), 0 Shudra (air).
fn varna(sign: u8) -> u8 {
    match sign % 12 {
        3 | 7 | 11 => 3,
        0 | 4 | 8 => 2,
        1 | 5 | 9 => 1,
        _ => 0,
    }
}

const VARNA_NAMES: [&str; 4] = ["Shudra", "Vaishya", "Kshatriya", "Brahmin"];

/// Vashya class by Moon sign and half: 0 Chatushpada, 1 Manava, 2 Jalachara, 3 Vanachara, 4 Keeta.
fn vashya(sidereal: f64) -> usize {
    let sign = ((sidereal / 30.0).floor() as usize) % 12;
    let second_half = sidereal.rem_euclid(30.0) >= 15.0;
    match sign {
        0 | 1 => 0,
        2 | 5 | 6 | 10 => 1,
        3 | 11 => 2,
        4 => 3,
        7 => 4,
        8 => {
            if second_half {
                0
            } else {
                1
            }
        }
        _ => {
            // Capricorn: first half quadruped, second half water
            if second_half {
                2
            } else {
                0
            }
        }
    }
}

const VASHYA_NAMES: [&str; 5] = ["Chatushpada", "Manava", "Jalachara", "Vanachara", "Keeta"];

/// Vashya points (convention: the common symmetric table).
fn vashya_points(a: usize, b: usize) -> f64 {
    const T: [[f64; 5]; 5] = [
        [2.0, 1.0, 1.0, 0.0, 1.0],
        [1.0, 2.0, 0.5, 0.5, 0.0],
        [1.0, 0.5, 2.0, 0.0, 1.0],
        [0.0, 0.5, 0.0, 2.0, 0.0],
        [1.0, 0.0, 1.0, 0.0, 2.0],
    ];
    T[a][b]
}

/// Yoni by nakshatra: 0 Horse, 1 Elephant, 2 Sheep, 3 Serpent, 4 Dog, 5 Cat, 6 Rat, 7 Cow,
/// 8 Buffalo, 9 Tiger, 10 Deer, 11 Monkey, 12 Mongoose, 13 Lion.
const YONI: [usize; 27] = [0, 1, 2, 3, 3, 4, 5, 2, 5, 6, 6, 7, 8, 9, 8, 9, 10, 10, 4, 11, 12, 11, 13, 0, 13, 7, 1];
const YONI_NAMES: [&str; 14] = [
    "Horse", "Elephant", "Sheep", "Serpent", "Dog", "Cat", "Rat", "Cow", "Buffalo", "Tiger", "Deer", "Monkey",
    "Mongoose", "Lion",
];
/// The classical 14 × 14 yoni table, 4 = same, 0 = sworn enemies.
const YONI_POINTS: [[u8; 14]; 14] = [
    [4, 2, 2, 3, 2, 2, 2, 1, 0, 1, 3, 3, 2, 1],
    [2, 4, 3, 3, 2, 2, 2, 2, 3, 1, 2, 3, 2, 0],
    [2, 3, 4, 2, 1, 2, 1, 3, 3, 1, 2, 0, 3, 1],
    [3, 3, 2, 4, 2, 1, 1, 1, 1, 2, 2, 2, 0, 2],
    [2, 2, 1, 2, 4, 2, 1, 2, 2, 1, 0, 2, 1, 1],
    [2, 2, 2, 1, 2, 4, 0, 2, 2, 1, 3, 3, 2, 1],
    [2, 2, 1, 1, 1, 0, 4, 2, 2, 2, 2, 2, 1, 2],
    [1, 2, 3, 1, 2, 2, 2, 4, 3, 0, 3, 2, 2, 1],
    [0, 3, 3, 1, 2, 2, 2, 3, 4, 1, 2, 2, 2, 1],
    [1, 1, 1, 2, 1, 1, 2, 0, 1, 4, 1, 1, 2, 1],
    [3, 2, 2, 2, 0, 3, 2, 3, 2, 1, 4, 2, 2, 1],
    [3, 3, 0, 2, 2, 3, 2, 2, 2, 1, 2, 4, 3, 2],
    [2, 2, 3, 0, 1, 2, 1, 2, 2, 2, 2, 3, 4, 2],
    [1, 0, 1, 2, 1, 1, 2, 1, 1, 1, 1, 2, 2, 4],
];

/// Gana by nakshatra: 0 Deva, 1 Manushya, 2 Rakshasa.
const GANA: [u8; 27] = [0, 1, 2, 1, 0, 1, 0, 0, 2, 2, 1, 1, 0, 2, 0, 2, 0, 2, 2, 1, 1, 0, 2, 2, 1, 1, 0];
const GANA_NAMES: [&str; 3] = ["Deva", "Manushya", "Rakshasa"];

/// Nadi by nakshatra: 0 Aadi, 1 Madhya, 2 Antya.
const NADI: [u8; 27] = [0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2];
const NADI_NAMES: [&str; 3] = ["Aadi", "Madhya", "Antya"];

fn moon_words(sidereal: f64) -> String {
    let n = nakshatra(sidereal);
    format!("{} · {}", SIGN_NAMES[((sidereal / 30.0).floor() as usize) % 12], NAKSHATRA_NAMES[n.index])
}

/// Kuja dosha: Mars in 1, 2, 4, 7, 8 or 12 from the lagna or from the Moon (convention:
/// both counted, the 2nd included as in the South). Cancelled when Mars is in its own
/// or exaltation sign.
fn kuja(n: &Natal) -> (bool, String) {
    let mars = n.facts.planet(DashaLord::Mars);
    let from_lagna = mars.rasi_house;
    let from_moon = ((mars.sign as i64 - n.moon_sign()).rem_euclid(12) + 1) as u8;
    let bad = [1u8, 2, 4, 7, 8, 12];
    let hit_l = bad.contains(&from_lagna);
    let hit_m = bad.contains(&from_moon);
    let d = dignity(DashaLord::Mars, mars.sidereal);
    let cancelled = matches!(d, Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted);
    if !(hit_l || hit_m) {
        return (false, format!("Mars in the {} from the lagna and the {} from the Moon: no Kuja dosha", crate::forecast::ordinal(from_lagna), crate::forecast::ordinal(from_moon)));
    }
    if cancelled {
        return (
            false,
            format!(
                "Mars in the {} from the lagna, but {:?} in {}: the dosha is cancelled",
                crate::forecast::ordinal(if hit_l { from_lagna } else { from_moon }),
                d,
                SIGN_NAMES[mars.sign as usize]
            ),
        );
    }
    (
        true,
        format!(
            "Kuja dosha: Mars in the {} from the {}",
            crate::forecast::ordinal(if hit_l { from_lagna } else { from_moon }),
            if hit_l { "lagna" } else { "Moon" }
        ),
    )
}

/// Match two charts. `a_is_groom` orients the two one-directional kootas (Varna, and the
/// Deva–Manushya exception of Gana).
pub fn kundali(a: &Natal, b: &Natal, a_is_groom: bool) -> Kundali {
    let ma = a.moon_sidereal;
    let mb = b.moon_sidereal;
    let sa = ((ma / 30.0).floor() as u8) % 12;
    let sb = ((mb / 30.0).floor() as u8) % 12;
    let na = nakshatra(ma).index;
    let nb = nakshatra(mb).index;
    let (g, br) = if a_is_groom { (sa, sb) } else { (sb, sa) };
    let mut kootas = Vec::with_capacity(8);

    // 1. Varna
    let (va, vb) = (varna(g), varna(br));
    let p = if va >= vb { 1.0 } else { 0.0 };
    kootas.push(Koota {
        name: "Varna",
        points: p,
        max: 1.0,
        words: format!(
            "{} and {}: {}",
            VARNA_NAMES[varna(sa) as usize],
            VARNA_NAMES[varna(sb) as usize],
            if p > 0.0 { "the temperaments sit in the right order" } else { "the order of temperaments is reversed (convention: groom's varna not below the bride's)" }
        ),
        source: MC,
    });

    // 2. Vashya
    let (xa, xb) = (vashya(ma), vashya(mb));
    let p = vashya_points(xa, xb);
    kootas.push(Koota {
        name: "Vashya",
        points: p,
        max: 2.0,
        words: format!(
            "{} and {}: {}",
            VASHYA_NAMES[xa],
            VASHYA_NAMES[xb],
            match p {
                x if x >= 2.0 => "the same kind; each holds the other easily",
                x if x >= 1.0 => "different kinds that get on",
                x if x > 0.0 => "a pull in one direction only",
                _ => "kinds that do not hold each other",
            }
        ),
        source: MC,
    });

    // 3. Tara
    let ta = (nb + 27 - na) % 27 % 9;
    let tb = (na + 27 - nb) % 27 % 9;
    let good = |t: usize| !matches!(t, 2 | 4 | 6);
    let p = (if good(ta) { 1.5 } else { 0.0 }) + (if good(tb) { 1.5 } else { 0.0 });
    kootas.push(Koota {
        name: "Tara",
        points: p,
        max: 3.0,
        words: format!(
            "their star is your {} tara, yours is their {}: {}",
            crate::gochara::TARA_NAMES[ta],
            crate::gochara::TARA_NAMES[tb],
            if p >= 3.0 { "both stars bless each other" } else if p > 0.0 { "one direction is blessed, the other tests" } else { "the stars test each other both ways" }
        ),
        source: MC,
    });

    // 4. Yoni
    let (ya, yb) = (YONI[na], YONI[nb]);
    let p = YONI_POINTS[ya][yb] as f64;
    kootas.push(Koota {
        name: "Yoni",
        points: p,
        max: 4.0,
        words: format!(
            "{} and {}: {}",
            YONI_NAMES[ya],
            YONI_NAMES[yb],
            match p as u8 {
                4 => "the same nature; deep physical ease",
                3 => "friendly natures",
                2 => "neutral natures; ease comes with time",
                1 => "natures that must learn each other",
                _ => "sworn enemies in the old tables; the body needs patience",
            }
        ),
        source: MC,
    });

    // 5. Graha Maitri
    let (la, lb) = (sign_lord(sa as usize), sign_lord(sb as usize));
    let p = if la == lb {
        5.0
    } else {
        let r1 = natural_relation(la, lb);
        let r2 = natural_relation(lb, la);
        match (r1, r2) {
            (1, 1) => 5.0,
            (1, 0) | (0, 1) => 4.0,
            (0, 0) => 3.0,
            (1, -1) | (-1, 1) => 1.0,
            (0, -1) | (-1, 0) => 0.5,
            _ => 0.0,
        }
    };
    kootas.push(Koota {
        name: "Graha Maitri",
        points: p,
        max: 5.0,
        words: format!(
            "Moon lords {la:?} and {lb:?}: {}",
            match p {
                x if x >= 5.0 => "friends; the minds meet",
                x if x >= 4.0 => "friend and neutral; the minds meet most days",
                x if x >= 3.0 => "neutral to each other; respect rather than instinct",
                x if x >= 1.0 => "a friend to an enemy; one gives more",
                x if x > 0.0 => "neutral to an enemy; the minds pull apart",
                _ => "enemies; the minds pull apart",
            }
        ),
        source: "BPHS, Naisargika maitri; Muhurta Chintamani",
    });

    // 6. Gana
    let (ga, gb) = (GANA[na], GANA[nb]);
    let (gg, gbr) = if a_is_groom { (ga, gb) } else { (gb, ga) };
    let p = match (gg, gbr) {
        (x, y) if x == y => 6.0,
        (0, 1) => 6.0,
        (1, 0) => 5.0,
        (0, 2) | (2, 0) => 1.0,
        _ => 0.0,
    };
    kootas.push(Koota {
        name: "Gana",
        points: p,
        max: 6.0,
        words: format!(
            "{} and {}: {}",
            GANA_NAMES[ga as usize],
            GANA_NAMES[gb as usize],
            match p as u8 {
                6 | 5 => "the same temper, or a gentle one with a human one",
                1 => "a gentle temper with a fierce one; sharp words at times",
                _ => "a human temper with a fierce one; the old tables warn",
            }
        ),
        source: MC,
    });

    // 7. Bhakoot
    let d = ((sb as i64 - sa as i64).rem_euclid(12) + 1) as u8;
    let dosha = matches!(d, 2 | 12 | 5 | 9 | 6 | 8);
    let friends = la == lb || (natural_relation(la, lb) == 1 && natural_relation(lb, la) == 1);
    let p = if !dosha || friends { 7.0 } else { 0.0 };
    kootas.push(Koota {
        name: "Bhakoot",
        points: p,
        max: 7.0,
        words: format!(
            "Moons {} apart ({}/{}): {}",
            crate::forecast::ordinal(d),
            d,
            ((1 - d as i64).rem_euclid(12) + 1),
            if !dosha {
                "a flowing distance; money, health and children find room"
            } else if friends {
                "a hard distance, cancelled because the Moon lords are friends"
            } else {
                match d {
                    2 | 12 => "the 2/12 pair: money and words need care",
                    5 | 9 => "the 5/9 pair: children and faith need care",
                    _ => "the 6/8 pair: health and sudden turns need care",
                }
            }
        ),
        source: MC,
    });

    // 8. Nadi
    let (da, db) = (NADI[na], NADI[nb]);
    let same_nadi = da == db;
    let cancelled = same_nadi && ((sa == sb && na != nb) || (na == nb && sa != sb));
    let p = if !same_nadi || cancelled { 8.0 } else { 0.0 };
    kootas.push(Koota {
        name: "Nadi",
        points: p,
        max: 8.0,
        words: format!(
            "{} and {} nadi: {}",
            NADI_NAMES[da as usize],
            NADI_NAMES[db as usize],
            if !same_nadi {
                "different pulses; the bodies and the line of children are protected"
            } else if cancelled {
                "the same pulse, cancelled by the Moons' sign and star (the classical exception)"
            } else {
                "the same pulse: the old tables call this the heaviest dosha; health and children ask for remedy"
            }
        ),
        source: MC,
    });

    let total: f64 = kootas.iter().map(|k| k.points).sum();
    let verdict = if total >= 28.0 {
        "Excellent match"
    } else if total >= 24.0 {
        "Very good"
    } else if total >= 18.0 {
        "Good enough"
    } else if total >= 13.0 {
        "Needs work"
    } else {
        "Hard match"
    };

    let (ka, kwa) = kuja(a);
    let (kb, kwb) = kuja(b);
    let kuja_words = if ka && kb {
        "Both charts carry Kuja dosha, so it is balanced: Mars meets Mars and neither is burned.".to_string()
    } else if ka || kb {
        format!(
            "{} in {} chart. The other chart does not carry it, so Mars asks for a remedy, or a Mars-strong chart beside it.",
            if ka { &kwa } else { &kwb },
            if ka { "the first" } else { "the second" }
        )
    } else {
        format!("{kwa}. {kwb}.")
    };

    // The heaviest kootas speak first.
    let mut strengths = Vec::new();
    let mut frictions = Vec::new();
    let mut by_weight: Vec<&Koota> = kootas.iter().collect();
    by_weight.sort_by(|a, b| b.max.partial_cmp(&a.max).unwrap_or(std::cmp::Ordering::Equal));
    for k in by_weight {
        if k.points >= k.max * 0.75 && k.max >= 2.0 {
            strengths.push(format!("{}: {}", k.name, k.words));
        } else if k.points <= k.max * 0.25 {
            frictions.push(format!("{}: {}", k.name, k.words));
        }
    }
    // Moon-to-Moon synastry beyond the kootas.
    match d {
        7 => strengths.push("Your Moons face each other across the 7th: the classical bond of partners.".into()),
        1 => strengths.push("Your Moons share a sign: you feel the same weather on the same day.".into()),
        3 | 11 => strengths.push(format!("Their Moon is in your {}: friendship and gain flow easily between you.", crate::forecast::ordinal(d))),
        _ => {}
    }
    if favourable_from_moon(DashaLord::Moon).contains(&d) && d != 1 && d != 7 {
        strengths.push("Each Moon sits in a house the other's transit tables call favourable.".into());
    }
    let venus_a = a.facts.planet(DashaLord::Venus).sign as i64;
    let venus_b = b.facts.planet(DashaLord::Venus).sign as i64;
    let vd = ((venus_b - venus_a).rem_euclid(12) + 1) as u8;
    if matches!(vd, 1 | 5 | 7 | 9) {
        strengths.push(format!("Your Venuses stand {} apart: the same taste in love and comfort.", if vd == 1 { "a sign".to_string() } else { format!("{} signs", vd - 1) }));
    } else if matches!(vd, 6 | 8) {
        frictions.push("Your Venuses stand 6/8 apart: what pleases one puzzles the other.".into());
    }

    let headline = format!(
        "{verdict}: {total:.1} of 36. {}{}",
        if strengths.is_empty() {
            String::new()
        } else {
            format!("Strongest, {}. ", strengths[0].replacen(": ", ": ", 1))
        },
        if frictions.is_empty() {
            "Nothing in the tables stands against you.".to_string()
        } else {
            format!("Hardest, {}.", frictions[0].trim_end_matches('.'))
        }
    );

    let points_of = |name: &str| kootas.iter().find(|k| k.name == name).map(|k| k.points).unwrap_or(0.0);
    let remedies = dosha_remedies(
        points_of("Nadi") == 0.0,
        points_of("Bhakoot") == 0.0,
        points_of("Gana") <= 1.0,
        ka != kb,
        points_of("Graha Maitri") <= 1.0,
        points_of("Yoni") <= 1.0,
    );

    Kundali {
        total,
        kootas,
        verdict: verdict.to_string(),
        headline,
        kuja_a: ka,
        kuja_b: kb,
        kuja_words,
        moon_a: moon_words(ma),
        moon_b: moon_words(mb),
        moon_distance: d,
        strengths,
        frictions,
        remedies,
    }
}

/// Two people at one instant and over the months around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Couple {
    /// The instant.
    pub at: Instant,
    /// The first person's Love score now.
    pub a_love: f64,
    /// The second person's.
    pub b_love: f64,
    /// The pair's score now, `[0, 1]`: the two Loves and the match.
    pub together: f64,
    /// Band of `together`.
    pub band: Band,
    /// Two or three sentences with dates.
    pub headline: String,
    /// Saturn relative to each Moon.
    pub a_saturn: SaturnPhase,
    /// For the second person.
    pub b_saturn: SaturnPhase,
    /// Plain notes about the two skies.
    pub notes: Vec<String>,
    /// The first month the pair climbs a band: `(instant, score)`.
    pub eases: Option<(Instant, f64)>,
    /// The best month of the window.
    pub peak: Option<(Instant, f64)>,
    /// The lowest month of the window.
    pub low: Option<(Instant, f64)>,
    /// Monthly slow samples, twelve months back to twenty-four ahead: `(at, a, b, together)`.
    pub series: Vec<(Instant, f64, f64, f64)>,
    /// What lifts the first person's love next.
    pub a_relief: Vec<Relief>,
    /// What lifts the second person's.
    pub b_relief: Vec<Relief>,
    /// What drags the first person's love now, in words.
    pub a_cause: String,
    /// The second's.
    pub b_cause: String,
    /// The first person's briefing: Moon, periods, Saturn, the moment, the first lift.
    pub a_briefing: Briefing,
    /// The second's.
    pub b_briefing: Briefing,
    /// The pair's own paragraphs: what the two seasons mean together.
    pub together_words: Vec<String>,
}

/// The engine speaks to the person whose chart it reads; the second chart's sentences
/// are turned to the third person so a screen can show both side by side.
pub fn theirs(s: &str) -> String {
    s.replace("Your birth Moon", "Their birth Moon")
        .replace("You are in", "They are in")
        .replace("your Moon", "their Moon")
        .replace("your love", "their love")
        .replace("your birth", "their birth")
        .replace("your strongest", "their strongest")
        .replace("your chart", "their chart")
        .replace("You ", "They ")
        .replace(" you ", " them ")
        .replace("your ", "their ")
}

fn third_person_briefing(b: &mut Briefing) {
    b.saturn_words = theirs(&b.saturn_words);
    for p in b.paragraphs.iter_mut() {
        *p = theirs(p);
    }
    for c in b.circled.iter_mut() {
        c.words = theirs(&c.words);
    }
    if let Some(r) = b.remedy.as_mut() {
        r.cause = theirs(&r.cause);
    }
}

fn together_of(a: f64, b: f64, match_total: f64) -> f64 {
    (0.65 * (a + b) / 2.0 + 0.35 * (match_total / 36.0)).clamp(0.0, 1.0)
}

/// The pair at `at`, both standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn couple(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    match_total: f64,
    at: Instant,
) -> Result<Couple, ForecastError> {
    let lon = obs.longitude_deg;
    let ma = moment(spk, eop, a, obs, system, ayanamsa, at)?;
    let mb = moment(spk, eop, b, obs, system, ayanamsa, at)?;
    let a_love = ma.lives[0].score;
    let b_love = mb.lives[0].score;
    let together = together_of(a_love, b_love, match_total);
    let band = Band::of(together);

    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let start = at.add_ns(-12 * month);
    let sa = slow_series(spk, eop, a, obs, ayanamsa, start, month, 37)?;
    let sb = slow_series(spk, eop, b, obs, ayanamsa, start, month, 37)?;
    let series: Vec<(Instant, f64, f64, f64)> = sa
        .iter()
        .zip(sb.iter())
        .map(|((t, va), (_, vb))| (*t, va[0], vb[0], together_of(va[0], vb[0], match_total)))
        .collect();
    let now_slow = series
        .iter()
        .min_by_key(|s| (s.0.ns_since(at)).abs())
        .map(|s| s.3)
        .unwrap_or(together);
    let band_slow = Band::of(now_slow).max(band);
    let ahead: Vec<&(Instant, f64, f64, f64)> = series.iter().filter(|s| s.0 > at).collect();
    let eases = ahead
        .iter()
        .find(|s| Band::of(s.3) > band_slow)
        .map(|s| (s.0, s.3));
    let peak = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v >= s.3 => acc,
            _ => Some((s.0, s.3)),
        });
    let low = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v <= s.3 => acc,
            _ => Some((s.0, s.3)),
        });

    let ea = explain(spk, eop, a, obs, system, ayanamsa, Life::Love, at)?;
    let eb = explain(spk, eop, b, obs, system, ayanamsa, Life::Love, at)?;
    let lifts = |e: &crate::explain::Explanation| -> Vec<Relief> {
        let mut v: Vec<Relief> = e
            .relief
            .iter()
            .filter(|r| r.gain >= 0.01 && r.lasting)
            .cloned()
            .collect();
        v.truncate(3);
        v
    };
    let a_relief = lifts(&ea);
    let mut b_relief = lifts(&eb);
    for r in b_relief.iter_mut() {
        r.what = theirs(&r.what);
    }
    let lasting = |e: &crate::explain::Explanation| -> String {
        e.causes
            .iter()
            .find(|c| matches!(c.kind, "dasha" | "gochara"))
            .map(|c| c.what.clone())
            .unwrap_or_default()
    };
    let a_cause = lasting(&ea);
    let b_cause = theirs(&lasting(&eb));

    let mut notes = Vec::new();
    match (ma.saturn, mb.saturn) {
        (SaturnPhase::Clear, SaturnPhase::Clear) => notes.push("Saturn is clear of both Moons.".into()),
        (x, SaturnPhase::Clear) => notes.push(format!("Saturn is testing the first chart ({}), not the second. The weight is one-sided; the other can carry.", x.words())),
        (SaturnPhase::Clear, y) => notes.push(format!("Saturn is testing the second chart ({}), not the first. The weight is one-sided; the other can carry.", y.words())),
        (x, y) => notes.push(format!("Saturn is testing both Moons at once ({}; {}). The old texts call this a shared trial: it passes, and it binds those who stay.", x.words(), y.words())),
    }
    let jup_a = ma.lives[0].components.iter().find(|c| c.key == "gochara").map(|c| c.note.clone()).unwrap_or_default();
    if !jup_a.is_empty() {
        notes.push(format!("First chart, transits for love: {jup_a}."));
    }
    let jup_b = mb.lives[0].components.iter().find(|c| c.key == "gochara").map(|c| theirs(&c.note)).unwrap_or_default();
    if !jup_b.is_empty() {
        notes.push(format!("Second chart, transits for love: {jup_b}."));
    }
    notes.push(format!(
        "Running periods: {} · {}",
        ma.dasha.iter().take(3).map(|p| format!("{:?}", p.lord)).collect::<Vec<_>>().join(" › "),
        mb.dasha.iter().take(3).map(|p| format!("{:?}", p.lord)).collect::<Vec<_>>().join(" › ")
    ));

    let mut headline = format!(
        "Together you read {:.0} now ({}): love is {:.0} in the first chart and {:.0} in the second.",
        together * 100.0,
        band.word(),
        a_love * 100.0,
        b_love * 100.0
    );
    if band <= Band::Steady {
        let worst = if a_love <= b_love { &a_cause } else { &b_cause };
        if !worst.is_empty() {
            headline.push_str(&format!(" The heavier side: {worst}."));
        }
        let first = a_relief.iter().chain(b_relief.iter()).min_by_key(|r| r.at.tai_ns());
        if let Some(r) = first {
            headline.push_str(&format!(" The first lift comes on {}: {}.", date_words(r.at, lon), r.what));
        }
        if let Some((t, s)) = eases {
            headline.push_str(&format!(" The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)));
        } else if let Some((t, _)) = peak {
            headline.push_str(&format!(" The best month in the next two years is around {}.", date_words(t, lon)));
        }
    } else if let Some((t, s)) = low {
        if Band::of(s) < Band::of(now_slow).min(band) {
            headline.push_str(&format!(" Hold what you have around {}, the lowest month ahead.", date_words(t, lon)));
        }
    }

    // Each person's briefing for love, and the pair's own paragraphs.
    let a_briefing = briefing(spk, eop, a, obs, system, ayanamsa, &[Life::Love], at)?;
    let mut b_briefing = briefing(spk, eop, b, obs, system, ayanamsa, &[Life::Love], at)?;
    third_person_briefing(&mut b_briefing);
    let mut together_words = Vec::new();
    together_words.push(format!(
        "Two seasons run at once. The first chart is in {}; the second in {}. {} {}",
        a_briefing.dasha,
        b_briefing.dasha,
        match (ma.saturn, mb.saturn) {
            (SaturnPhase::Clear, SaturnPhase::Clear) => "Saturn tests neither Moon now.".to_string(),
            (x, SaturnPhase::Clear) => format!("Saturn tests the first chart's Moon ({}); the second is free of it, and can carry.", x.words().split(':').next().unwrap_or("")),
            (SaturnPhase::Clear, y) => format!("Saturn tests the second chart's Moon ({}); the first is free of it, and can carry.", y.words().split(':').next().unwrap_or("")),
            (x, y) => format!("Saturn tests both Moons ({}; {}): a shared trial, which binds those who stay.", x.words().split(':').next().unwrap_or(""), y.words().split(':').next().unwrap_or("")),
        },
        match (a_briefing.saturn_until, b_briefing.saturn_until) {
            (Some(ta), _) if ma.saturn != SaturnPhase::Clear => format!("The first chart's Saturn phase ends {}.", date_words(ta, lon)),
            (_, Some(tb)) if mb.saturn != SaturnPhase::Clear => format!("The second chart's Saturn phase ends {}.", date_words(tb, lon)),
            _ => String::new(),
        }
    ));
    let d = ((b.moon_sign() - a.moon_sign()).rem_euclid(12) + 1) as u8;
    together_words.push(format!(
        "Your Moons stand {} apart ({}). Love reads {:.0} in the first chart and {:.0} in the second this second; the pair, with the match of {:.1} of 36 folded in, reads {:.0}. {}",
        crate::forecast::ordinal(d),
        match d {
            1 => "the same sign: the same weather on the same day",
            7 => "facing each other: the classical angle of partners",
            3 | 11 => "a friendly angle: ease and gain flow between you",
            2 | 12 => "the 2/12 angle: money and words need care",
            5 | 9 => "the 5/9 angle: children and faith need care",
            6 | 8 => "the 6/8 angle: health and sudden turns need care",
            _ => "a square angle: effort, not ease, but it builds",
        },
        a_love * 100.0,
        b_love * 100.0,
        match_total,
        together * 100.0,
        if a_love < b_love {
            format!("The wait is on the first chart's side: {}.", a_cause)
        } else if b_love < a_love {
            format!("The wait is on the second chart's side: {}.", b_cause)
        } else {
            "The two charts carry the same weight today.".to_string()
        }
    ));
    let first_lift = a_relief.iter().map(|r| (r, "first")).chain(b_relief.iter().map(|r| (r, "second"))).min_by_key(|(r, _)| r.at.tai_ns());
    if let Some((r, who)) = first_lift {
        together_words.push(format!(
            "The first lasting lift for the pair comes on {}, in the {who} chart: {} ({:+.0} points). {}",
            date_words(r.at, lon),
            r.what,
            r.gain * 100.0,
            match (eases, peak) {
                (Some((t, s)), _) => format!("The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)),
                (None, Some((t, _))) => format!("The best month in the next two years is around {}.", date_words(t, lon)),
                _ => String::new(),
            }
        ));
    }

    Ok(Couple {
        at,
        a_love,
        b_love,
        together,
        band,
        headline,
        a_saturn: ma.saturn,
        b_saturn: mb.saturn,
        notes,
        eases,
        peak,
        low,
        series,
        a_relief,
        b_relief,
        a_cause,
        b_cause,
        a_briefing,
        b_briefing,
        together_words,
    })
}

/// The talk window: the first hour in the next `days` when both charts read well for
/// love at once (both hora fits at or above 0.55, outside the cautions, both daily
/// love scores at or above 0.5). `(start, end, mean fit)`.
#[allow(clippy::too_many_arguments)]
pub fn talk_window(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    from: Instant,
    days: usize,
) -> Result<Option<(Instant, Instant, f64)>, ForecastError> {
    let sa = crate::forecast::series(spk, eop, a, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    let sb = crate::forecast::series(spk, eop, b, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    for d in 0..days.max(1) {
        if sa[d].scores[0] < 0.5 || sb[d].scores[0] < 0.5 {
            continue;
        }
        let t = from.add_ns(d as i128 * NS_PER_DAY + 6 * 3_600 * kshana_time::NS_PER_SEC);
        let ma = moment(spk, eop, a, obs, system, ayanamsa, t)?;
        let mb = moment(spk, eop, b, obs, system, ayanamsa, t)?;
        for (ha, hb) in ma.hours.iter().zip(mb.hours.iter()) {
            if ha.caution || ha.end <= from {
                continue;
            }
            if ha.fit[0] >= 0.55 && hb.fit[0] >= 0.55 {
                return Ok(Some((ha.start.max(from), ha.end, (ha.fit[0] + hb.fit[0]) / 2.0)));
            }
        }
    }
    Ok(None)
}

/// A question about the pair, answered from both charts.
#[derive(Clone, Debug, PartialEq)]
pub struct CoupleAnswer {
    /// The first chart's answer about love.
    pub a: Answer,
    /// The second chart's.
    pub b: Answer,
    /// Combined verdict.
    pub verdict: String,
    /// Combined score, `[0, 1]`.
    pub score: f64,
    /// Combined confidence.
    pub confidence: f64,
    /// Two sentences.
    pub headline: String,
    /// The window both charts open, or the earlier of the two.
    pub when_start: Option<Instant>,
    /// Its end.
    pub when_end: Option<Instant>,
}

/// Ask `question` about love for both charts and read them together.
#[allow(clippy::too_many_arguments)]
pub fn couple_ask(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    question: Question,
    at: Instant,
) -> Result<CoupleAnswer, ForecastError> {
    let aa = answer(spk, eop, a, obs, system, ayanamsa, Life::Love, question, at)?;
    let ab = answer(spk, eop, b, obs, system, ayanamsa, Life::Love, question, at)?;
    let score = (0.5 * aa.score + 0.5 * ab.score).clamp(0.0, 1.0);
    let confidence = ((aa.confidence + ab.confidence) / 2.0 - (aa.score - ab.score).abs() * 0.5).clamp(0.3, 0.95);
    // Each chart has answered the question in its own terms; the pair's verdict says
    // whether the two agree, and on whose side the wait is.
    let yes = |v: &str| {
        v.starts_with("Yes")
            || v.starts_with("Take it")
            || v.starts_with("Go ahead")
            || v.starts_with("Within")
            || v.starts_with("Nothing has left")
    };
    let (ya, yb) = (yes(&aa.verdict), yes(&ab.verdict));
    let verdict: String = match (ya, yb, question) {
        (true, true, Question::WillItReturn) => "Yes, you come back together".into(),
        (true, true, Question::WillItRecover) => "Yes, it mends between you".into(),
        (true, true, Question::ShouldI) => "Take the step, both of you".into(),
        (true, true, Question::GoodTimeNow) => "Now serves you both".into(),
        (true, true, Question::When) => format!("Both charts open: {}", if aa.when_start.map(|t| t.tai_ns()) <= ab.when_start.map(|t| t.tai_ns()) { ab.verdict.to_ascii_lowercase() } else { aa.verdict.to_ascii_lowercase() }),
        (true, true, _) => "Yes, for both of you".into(),
        (true, false, _) => "Your chart says yes; theirs waits".into(),
        (false, true, _) => "Their chart says yes; yours waits".into(),
        (false, false, Question::WillItReturn) => "Not as it was, for either of you".into(),
        (false, false, Question::GoodTimeNow) => "Not this hour, for either of you".into(),
        (false, false, Question::When) => "Neither chart opens soon".into(),
        (false, false, _) => "Not yet, for either of you".into(),
    };
    let (ws, we) = match (aa.when_start, aa.when_end, ab.when_start, ab.when_end) {
        (Some(s1), Some(e1), Some(s2), Some(e2)) => {
            let s = s1.max(s2);
            let e = e1.min(e2);
            if s <= e {
                (Some(s), Some(e))
            } else if s1 <= s2 {
                (Some(s1), Some(e1))
            } else {
                (Some(s2), Some(e2))
            }
        }
        (Some(s1), Some(e1), _, _) => (Some(s1), Some(e1)),
        (_, _, Some(s2), Some(e2)) => (Some(s2), Some(e2)),
        _ => (None, None),
    };
    let headline = format!(
        "{verdict}. Your chart says \"{}\" ({:.0}); theirs says \"{}\" ({:.0}). {}",
        aa.verdict,
        aa.score * 100.0,
        ab.verdict,
        ab.score * 100.0,
        if (aa.score - ab.score).abs() > 0.15 {
            if aa.score > ab.score {
                "The wait is on their side: their period has to open first."
            } else {
                "The wait is on your side: your period has to open first."
            }
        } else {
            "The two charts agree."
        }
    );
    Ok(CoupleAnswer {
        a: aa,
        b: ab,
        verdict,
        score,
        confidence,
        headline,
        when_start: ws,
        when_end: we,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn tables_are_well_formed() {
        for row in YONI_POINTS {
            assert_eq!(row.len(), 14);
        }
        for i in 0..14 {
            assert_eq!(YONI_POINTS[i][i], 4);
            for j in 0..14 {
                assert_eq!(YONI_POINTS[i][j], YONI_POINTS[j][i], "yoni {i} {j}");
            }
        }
        assert_eq!(GANA.iter().filter(|g| **g == 0).count(), 9);
        assert_eq!(GANA.iter().filter(|g| **g == 1).count(), 9);
        assert_eq!(NADI.iter().filter(|g| **g == 0).count(), 9);
        for a in 0..5 {
            for b in 0..5 {
                assert_eq!(vashya_points(a, b), vashya_points(b, a));
            }
        }
    }

    #[test]
    fn a_pair_is_matched_and_read_over_time() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let ba = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let bb = Utc::new(1992, 8, 11, 14, 30, 0, 0).unwrap().to_instant().unwrap();
        let a = Natal::compute(&spk, None, ba, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let b = Natal::compute(&spk, None, bb, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let k = kundali(&a, &b, true);
        assert_eq!(k.kootas.len(), 8);
        assert!((0.0..=36.0).contains(&k.total));
        let max: f64 = k.kootas.iter().map(|k| k.max).sum();
        assert_eq!(max, 36.0);
        assert!(!k.headline.is_empty() && !k.kuja_words.is_empty());
        // matching a chart with itself gives every symmetric koota its maximum except Nadi
        let s = kundali(&a, &a, true);
        assert!(s.kootas.iter().filter(|k| k.name != "Nadi").all(|k| k.points == k.max), "{:?}", s.kootas);
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let c = couple(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, k.total, at).unwrap();
        assert_eq!(c.series.len(), 37);
        assert!((0.0..=1.0).contains(&c.together));
        assert!(!c.headline.is_empty() && !c.notes.is_empty());
        let q = couple_ask(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Question::WillItReturn, at).unwrap();
        assert!(!q.verdict.is_empty());
        assert!((0.3..=0.95).contains(&q.confidence));
    }
}
