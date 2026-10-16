//! The forecast layer: what this moment, this day and the coming weeks hold for nine
//! plain life areas, scored from the natal chart and the sky of the moment, and said
//! in plain words. Every score is a weighted sum of named components, each a classical
//! rule with a citation, so the app can always show why.
//!
//! Components of a life score at an instant:
//!
//! | key       | weight | rule |
//! |-----------|--------|------|
//! | `dasha`   | 0.35 | the five running Vimshottari lords, KP signification of the area's houses (Reader III) and Parashari period quality (BPHS, Dasha phala) |
//! | `gochara` | 0.25 | transits counted from the natal Moon with vedha (Brihat Samhita 104, Phaladeepika 26), filtered by Bhinnashtakavarga bindus (BPHS), Saturn's sade sati and dhaiya |
//! | `tara`    | 0.15 | Tara bala of the Moon's star from the birth star (Muhurta Chintamani) |
//! | `hora`    | 0.15 | the planetary hour's lord and what it suits, weighed by that lord's natal dignity |
//! | `kala`    | 0.10 | the limbs of the day: Rahu kala, Yamaganda, Gulika, Abhijit, rikta tithis, Vishti, the auspicious stars |
//!
//! A series sample (one per day or fortnight) uses only the first three, renormalised.

use crate::areas::{area, Area};
use crate::facts::Facts;
use crate::gochara::*;
use crate::kp::period_supports;
use crate::parashari::{bhinna_for, dignity, period_quality};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::{Body, Spk};
use kshana_events::{
    hora_at, longitude_crossings, nakshatra_boundaries, rahu_kala, solar_day, tithi_boundaries,
    SolarDay, MICROSECOND_NS,
};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES};
use kshana_vedic::{karana, nakshatra, tithi, vimshottari_levels, Ayanamsa, DashaLord, DashaPeriod};

/// Sign names, 0 = Aries.
pub const SIGN_NAMES: [&str; 12] = [
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

/// The nine life areas as a person names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Life {
    Love,
    Family,
    Career,
    Money,
    Health,
    Study,
    Home,
    Travel,
    Children,
}

impl Life {
    /// All nine, in display order.
    pub const ALL: [Life; 9] = [
        Life::Love,
        Life::Career,
        Life::Money,
        Life::Health,
        Life::Family,
        Life::Study,
        Life::Home,
        Life::Travel,
        Life::Children,
    ];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Life::Love => "Love",
            Life::Family => "Family",
            Life::Career => "Career",
            Life::Money => "Money",
            Life::Health => "Health",
            Life::Study => "Study",
            Life::Home => "Home",
            Life::Travel => "Travel",
            Life::Children => "Children",
        }
    }

    /// Parse a display name.
    pub fn parse(s: &str) -> Option<Life> {
        Life::ALL
            .iter()
            .copied()
            .find(|l| l.name().eq_ignore_ascii_case(s))
    }

    /// The KP/Parashari area behind it.
    pub fn area(self) -> Area {
        let name = match self {
            Life::Love => "Marriage",
            Life::Family => "Family",
            Life::Career => "Career",
            Life::Money => "Wealth",
            Life::Health => "Health",
            Life::Study => "Education",
            Life::Home => "Property",
            Life::Travel => "Foreign",
            Life::Children => "Children",
        };
        area(name).expect("every life maps to an area")
    }

    /// How much a graha's transit bears on this life (natural karakatva), in `[0, 1]`.
    pub fn relevance(self, g: DashaLord) -> f64 {
        use DashaLord::*;
        let t: [f64; 9] = match self {
            //          Sun  Moon Mars Merc Jup  Ven  Sat  Rahu Ketu
            Life::Love => [0.3, 0.6, 0.4, 0.3, 0.5, 1.0, 0.4, 0.3, 0.3],
            Life::Family => [0.4, 1.0, 0.4, 0.3, 0.6, 0.5, 0.4, 0.3, 0.3],
            Life::Career => [0.8, 0.3, 0.6, 0.6, 0.6, 0.3, 1.0, 0.5, 0.3],
            Life::Money => [0.3, 0.4, 0.3, 0.6, 1.0, 0.6, 0.5, 0.5, 0.2],
            Life::Health => [1.0, 0.7, 0.7, 0.3, 0.4, 0.3, 0.8, 0.5, 0.5],
            Life::Study => [0.3, 0.4, 0.3, 1.0, 0.9, 0.4, 0.4, 0.3, 0.4],
            Life::Home => [0.3, 0.7, 1.0, 0.3, 0.5, 0.6, 0.5, 0.3, 0.3],
            Life::Travel => [0.3, 0.8, 0.4, 0.6, 0.4, 0.4, 0.5, 0.8, 0.5],
            Life::Children => [0.4, 0.5, 0.4, 0.3, 1.0, 0.5, 0.4, 0.3, 0.4],
        };
        let i = match g {
            Sun => 0,
            Moon => 1,
            Mars => 2,
            Mercury => 3,
            Jupiter => 4,
            Venus => 5,
            Saturn => 6,
            Rahu => 7,
            Ketu => 8,
        };
        t[i]
    }

    /// How well a planetary hour's lord suits this life, in `[0, 1]`.
    pub fn hora_affinity(self, g: DashaLord) -> f64 {
        use DashaLord::*;
        let t: [f64; 7] = match self {
            //          Sun  Moon Mars Merc Jup  Ven  Sat
            Life::Love => [0.4, 0.8, 0.2, 0.5, 0.7, 1.0, 0.1],
            Life::Family => [0.5, 1.0, 0.3, 0.5, 0.8, 0.8, 0.2],
            Life::Career => [1.0, 0.4, 0.6, 0.8, 0.8, 0.4, 0.6],
            Life::Money => [0.5, 0.5, 0.3, 0.9, 1.0, 0.7, 0.3],
            Life::Health => [0.9, 0.6, 0.3, 0.5, 0.9, 0.6, 0.2],
            Life::Study => [0.5, 0.5, 0.3, 1.0, 1.0, 0.6, 0.4],
            Life::Home => [0.5, 0.8, 0.5, 0.5, 0.9, 0.9, 0.3],
            Life::Travel => [0.5, 1.0, 0.3, 0.9, 0.7, 0.7, 0.2],
            Life::Children => [0.5, 0.8, 0.3, 0.5, 1.0, 0.8, 0.2],
        };
        match g {
            Sun => t[0],
            Moon => t[1],
            Mars => t[2],
            Mercury => t[3],
            Jupiter => t[4],
            Venus => t[5],
            Saturn => t[6],
            _ => 0.3,
        }
    }
}

/// Five bands a score falls in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Band {
    Hold,
    Slow,
    Steady,
    Good,
    Bright,
}

impl Band {
    /// Band for a score in `[0, 1]`.
    pub fn of(score: f64) -> Band {
        if score >= 0.70 {
            Band::Bright
        } else if score >= 0.56 {
            Band::Good
        } else if score >= 0.44 {
            Band::Steady
        } else if score >= 0.32 {
            Band::Slow
        } else {
            Band::Hold
        }
    }

    /// One word.
    pub fn word(self) -> &'static str {
        match self {
            Band::Bright => "bright",
            Band::Good => "good",
            Band::Steady => "steady",
            Band::Slow => "slow",
            Band::Hold => "hold",
        }
    }
}

/// One scored component.
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    /// `dasha`, `gochara`, `tara`, `hora` or `kala`.
    pub key: &'static str,
    /// Score in `[0, 1]`.
    pub score: f64,
    /// Weight used.
    pub weight: f64,
    /// Why, in words.
    pub note: String,
    /// Rule id.
    pub rule_id: &'static str,
}

/// One life area at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeScore {
    /// Which life.
    pub life: Life,
    /// Score in `[0, 1]`.
    pub score: f64,
    /// Band.
    pub band: Band,
    /// One plain sentence.
    pub headline: String,
    /// The strongest reason, in words.
    pub because: String,
    /// Every component.
    pub components: Vec<Component>,
}

/// A named window of the day.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    /// Name.
    pub label: String,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// True when it asks for caution rather than action.
    pub caution: bool,
}

/// Something that changes at an instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Shift {
    /// When.
    pub at: Instant,
    /// What, in words.
    pub what: String,
    /// Kind: `hora`, `star`, `tithi`, `sign`, `dasha`, `kala`, `sun`.
    pub kind: &'static str,
}

/// The planetary hour.
#[derive(Clone, Debug, PartialEq)]
pub struct Hora {
    /// Lord.
    pub lord: DashaLord,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// What it suits.
    pub words: &'static str,
}

/// One planetary hour of the day with its fit for each life.
#[derive(Clone, Debug, PartialEq)]
pub struct HourSlot {
    /// Lord.
    pub lord: DashaLord,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Hora component score per life, `Life::ALL` order.
    pub fit: Vec<f64>,
    /// True when the slot overlaps Rahu kala, Yamaganda or Gulika.
    pub caution: bool,
}

/// The whole picture at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Moment {
    /// The twenty-four planetary hours of the day, sunrise to sunrise.
    pub hours: Vec<HourSlot>,
    /// The instant.
    pub at: Instant,
    /// Mean of the life scores.
    pub overall: f64,
    /// Band of the mean.
    pub band: Band,
    /// One sentence for the moment.
    pub headline: String,
    /// Nine lives, in `Life::ALL` order.
    pub lives: Vec<LifeScore>,
    /// The planetary hour, when the Sun rises and sets here.
    pub hora: Option<Hora>,
    /// The Moon's star now.
    pub moon_star: String,
    /// Tara index 0–8 and words.
    pub tara: usize,
    /// Tara in words.
    pub tara_words: &'static str,
    /// The Moon's sign counted from the natal Moon, 1–12.
    pub moon_from_moon: u8,
    /// Saturn's phase relative to the natal Moon.
    pub saturn: SaturnPhase,
    /// Running dasha lords, maha first.
    pub dasha: Vec<DashaPeriod>,
    /// Named windows of the day, chronological.
    pub windows: Vec<Window>,
    /// Upcoming shifts within about a day, chronological.
    pub shifts: Vec<Shift>,
    /// Sunrise and sunset of the day, when defined.
    pub sunrise: Option<Instant>,
    /// Sunset.
    pub sunset: Option<Instant>,
}

/// One sample of a series.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// When.
    pub at: Instant,
    /// Scores in `Life::ALL` order.
    pub scores: Vec<f64>,
    /// Running dasha lords, maha first (three levels).
    pub dasha: Vec<DashaLord>,
    /// Each graha's house from the natal Moon, Sun..Ketu.
    pub houses_from_moon: [u8; 9],
    /// Tara index.
    pub tara: usize,
}

/// A horizon story for one life.
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    /// Which life.
    pub life: Life,
    /// `rising`, `falling` or `steady`.
    pub trend: &'static str,
    /// One sentence for the trend.
    pub trend_words: String,
    /// Mean score over the horizon.
    pub mean: f64,
    /// Best stretch: start, end, score.
    pub best: Option<(Instant, Instant, f64)>,
    /// Lowest stretch: start, end, score.
    pub low: Option<(Instant, Instant, f64)>,
    /// Shifts that matter for this life, chronological.
    pub shifts: Vec<Shift>,
}

/// What the forecast needs from the birth chart.
#[derive(Clone, Debug)]
pub struct Natal {
    /// Facts of the birth chart.
    pub facts: Facts,
    /// Natal Moon, sidereal degrees.
    pub moon_sidereal: f64,
    /// Birth instant.
    pub birth: Instant,
    /// Bhinnashtakavarga tables, Sun..Saturn.
    pub bav: [[u8; 12]; 7],
}

impl Natal {
    /// From the birth chart's facts.
    pub fn new(facts: Facts, birth: Instant) -> Self {
        let moon_sidereal = facts.planet(DashaLord::Moon).sidereal;
        let bav = bhinna_for(&facts);
        Self {
            facts,
            moon_sidereal,
            birth,
            bav,
        }
    }

    fn moon_sign(&self) -> i64 {
        (self.moon_sidereal / 30.0).floor() as i64
    }

    fn janma(&self) -> usize {
        nakshatra(self.moon_sidereal).index
    }
}

/// Errors.
#[derive(Debug)]
pub enum ForecastError {
    /// The chart could not be computed.
    Chart(String),
}

const GRAHAS: [DashaLord; 9] = [
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

const LEVEL_NAMES: [&str; 5] = ["mahadasha", "antardasha", "pratyantar", "sookshma", "prana"];
const LEVEL_WEIGHTS: [f64; 5] = [0.35, 0.30, 0.20, 0.10, 0.05];

/// The universally auspicious stars (Muhurta Chintamani), 0-based.
const GOOD_STARS: [usize; 15] = [0, 3, 4, 6, 7, 11, 12, 13, 14, 16, 20, 21, 22, 25, 26];
/// Yamaganda and Gulika slots (1-based eighth of the daytime) by weekday Sun..Sat.
const YAMAGANDA: [usize; 7] = [5, 4, 3, 2, 1, 7, 6];
const GULIKA: [usize; 7] = [7, 6, 5, 4, 3, 2, 1];

fn bav_index(g: DashaLord) -> Option<usize> {
    Some(match g {
        DashaLord::Sun => 0,
        DashaLord::Moon => 1,
        DashaLord::Mars => 2,
        DashaLord::Mercury => 3,
        DashaLord::Jupiter => 4,
        DashaLord::Venus => 5,
        DashaLord::Saturn => 6,
        _ => return None,
    })
}

pub(crate) fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn weekday(day: &SolarDay, obs: &Observer) -> u32 {
    let local = day
        .sunrise
        .add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

fn slot_window(day: &SolarDay, slot: usize) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise) / 8;
    (
        day.sunrise.add_ns((slot as i128 - 1) * len),
        day.sunrise.add_ns(slot as i128 * len),
    )
}

fn abhijit_window(day: &SolarDay) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise);
    let noon = day.sunrise.add_ns(len / 2);
    (noon.add_ns(-len / 30), noon.add_ns(len / 30))
}

fn in_window(t: Instant, w: (Instant, Instant)) -> bool {
    t >= w.0 && t < w.1
}

/// The sky of the moment, as facts, plus each graha's house from the natal Moon.
struct Sky {
    facts: Facts,
    from_moon: [u8; 9],
    moon_sidereal: f64,
    sun_sidereal: f64,
    ayanamsa_deg: f64,
}

fn sky(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<Sky, ForecastError> {
    let input = ChartInput {
        instant: at,
        observer: *obs,
        system,
        ayanamsa,
        topocentric_moon: false,
    };
    let ch = compute(spk, eop, &input).map_err(|e| ForecastError::Chart(format!("{e:?}")))?;
    let facts = Facts::from_chart(&ch);
    let ms = natal.moon_sign();
    let from_moon: [u8; 9] = std::array::from_fn(|i| {
        let s = facts.planet(GRAHAS[i]).sign as i64;
        ((s - ms).rem_euclid(12) + 1) as u8
    });
    Ok(Sky {
        moon_sidereal: facts.planet(DashaLord::Moon).sidereal,
        sun_sidereal: facts.planet(DashaLord::Sun).sidereal,
        facts,
        from_moon,
        ayanamsa_deg: ch.ayanamsa_deg,
    })
}

// ---------------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------------

/// What the birth chart itself promises for the life: the KP cusp sub-lord net and the
/// Parashari assessment, half each. Constant over time; it sets each life's baseline.
pub fn natal_component(natal: &Natal, life: Life) -> Component {
    let area = life.area();
    let p = crate::kp::promise(&natal.facts, area);
    let b = crate::parashari::assess(&natal.facts, area);
    let kp = (0.5 + 0.5 * p.net.clamp(-1.0, 1.0)).clamp(0.0, 1.0);
    let score = (0.5 * kp + 0.5 * b.score).clamp(0.0, 1.0);
    let name = life.name().to_ascii_lowercase();
    let houses = |v: &[crate::kp::Significance]| {
        v.iter()
            .map(|s| s.house.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let note = match (p.promised, b.score >= 0.5) {
        (true, true) => format!(
            "your birth chart promises {name}: {:?}, sub-lord of the {}, signifies houses {} and the Parashari reading agrees ({:.0}%)",
            p.sub_lord,
            crate::forecast::ordinal(area.primary),
            houses(&p.fulfilling),
            b.score * 100.0
        ),
        (true, false) => format!(
            "your birth chart promises {name} by KP ({:?} signifies {}) though the Parashari reading is cautious ({:.0}%)",
            p.sub_lord,
            houses(&p.fulfilling),
            b.score * 100.0
        ),
        (false, true) => format!(
            "your birth chart carries {name} through Parashari ({:.0}%) while the KP sub-lord {:?} leans to houses {}",
            b.score * 100.0,
            p.sub_lord,
            if p.denying.is_empty() { "outside the matter".to_string() } else { houses(&p.denying) }
        ),
        (false, false) => format!(
            "your birth chart makes {name} hard-won: {:?}, sub-lord of the {}, signifies {} and the Parashari reading is {:.0}%",
            p.sub_lord,
            crate::forecast::ordinal(area.primary),
            if p.denying.is_empty() { "none of its houses".to_string() } else { format!("the negating houses {}", houses(&p.denying)) },
            b.score * 100.0
        ),
    };
    Component {
        key: "natal",
        score,
        weight: 0.15,
        note,
        rule_id: "KS-N-0001",
    }
}

fn dasha_component(natal: &Natal, life: Life, levels: &[DashaPeriod]) -> Component {
    let area = life.area();
    let mut num = 0.0;
    let mut den = 0.0;
    let mut best: Option<(f64, String)> = None;
    let mut worst: Option<(f64, String)> = None;
    for (i, p) in levels.iter().enumerate().take(5) {
        let w = LEVEL_WEIGHTS[i];
        let sup = period_supports(&natal.facts, area, p.lord);
        let kp = sup
            .iter()
            .map(|s| s.level.weight())
            .fold(0.0_f64, f64::max);
        let (pq, _) = period_quality(&natal.facts, area, p.lord);
        let s = 0.5 * kp + 0.5 * pq;
        num += w * s;
        den += w;
        let words = if sup.is_empty() {
            format!(
                "{:?} {} does not touch your {} houses",
                p.lord,
                LEVEL_NAMES[i],
                life.name().to_ascii_lowercase()
            )
        } else {
            format!(
                "{:?} {} carries your {} houses ({})",
                p.lord,
                LEVEL_NAMES[i],
                life.name().to_ascii_lowercase(),
                sup.iter()
                    .map(|s| s.house.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let v = w * (s - 0.5);
        if best.as_ref().map_or(true, |b| v > b.0) {
            best = Some((v, words.clone()));
        }
        if worst.as_ref().map_or(true, |b| v < b.0) {
            worst = Some((v, words));
        }
    }
    let score = if den > 0.0 { num / den } else { 0.5 };
    let note = match (best, worst) {
        (Some(b), Some(w)) if score >= 0.5 => {
            if b.0 > 0.0 {
                b.1
            } else {
                w.1
            }
        }
        (Some(_), Some(w)) => w.1,
        _ => "no running period".into(),
    };
    Component {
        key: "dasha",
        score: score.clamp(0.0, 1.0),
        weight: 0.30,
        note,
        rule_id: "KS-F-0001",
    }
}

fn gochara_component(natal: &Natal, sky: &Sky, life: Life) -> Component {
    let mut num = 0.0;
    let mut den = 0.0;
    let mut best: Option<(f64, String)> = None;
    let mut worst: Option<(f64, String)> = None;
    for (i, &g) in GRAHAS.iter().enumerate() {
        let h = sky.from_moon[i];
        let mut r = life.relevance(g);
        // A graha that owns one of the area's houses in the birth chart matters more.
        let owns_area = natal
            .facts
            .planet(g)
            .owns
            .iter()
            .any(|o| life.area().group.contains(o));
        if owns_area {
            r = (r + 0.3).min(1.0);
        }
        let fav = favourable_from_moon(g).contains(&h);
        let adverse = adverse_from_moon(g).contains(&h);
        let obstructed = fav
            && vedha_pairs(g).iter().any(|&(fh, vh)| {
                fh == h
                    && GRAHAS[..7].iter().enumerate().any(|(j, &p)| {
                        p != g && !vedha_exempt(g, p) && sky.from_moon[j] == vh
                    })
            });
        let saturn_phase = if g == DashaLord::Saturn {
            SaturnPhase::from_house(h)
        } else {
            SaturnPhase::Clear
        };
        let mut v: f64 = if fav && !obstructed {
            1.0
        } else if fav {
            0.2
        } else if adverse {
            -1.0
        } else {
            0.0
        };
        if saturn_phase != SaturnPhase::Clear && v >= 0.0 {
            v = -0.6;
        }
        // Chandrashtama: the Moon in the 8th from itself weighs on every life.
        if g == DashaLord::Moon && h == 8 {
            v = -1.0;
            r = r.max(0.8);
        }
        let av = match bav_index(g) {
            Some(k) => match natal.bav[k][sky.facts.planet(g).sign as usize] {
                b if b >= 5 => 1.25,
                4 => 1.0,
                _ => 0.6,
            },
            None => 1.0,
        };
        let contrib = r * v * av;
        num += contrib;
        den += r;
        let words = if saturn_phase != SaturnPhase::Clear {
            saturn_phase.words().to_string()
        } else if g == DashaLord::Moon && h == 8 {
            "Chandrashtama: the Moon is in the 8th from your Moon".to_string()
        } else {
            format!(
                "{:?} in the {} from your Moon{}",
                g,
                ordinal(h),
                if obstructed {
                    " (obstructed by vedha)"
                } else if fav {
                    " (favourable)"
                } else if adverse {
                    " (testing)"
                } else {
                    ""
                }
            )
        };
        if best.as_ref().map_or(true, |b| contrib > b.0) {
            best = Some((contrib, words.clone()));
        }
        if worst.as_ref().map_or(true, |b| contrib < b.0) {
            worst = Some((contrib, words));
        }
    }
    let raw = if den > 0.0 { num / den } else { 0.0 };
    let score = (0.5 + 0.5 * raw).clamp(0.0, 1.0);
    let note = match (best, worst) {
        (Some(b), Some(w)) => {
            if score >= 0.5 && b.0 > 0.0 {
                b.1
            } else if w.0 < 0.0 {
                w.1
            } else {
                b.1
            }
        }
        _ => String::new(),
    };
    Component {
        key: "gochara",
        score,
        weight: 0.20,
        note,
        rule_id: "KS-G-0001",
    }
}

fn tara_component(natal: &Natal, sky: &Sky) -> (usize, Component) {
    let t = tara_index(natal.janma(), nakshatra(sky.moon_sidereal).index);
    (
        t,
        Component {
            key: "tara",
            score: TARA_SCORE[t],
            weight: 0.12,
            note: tara_words(t).to_string(),
            rule_id: "KS-T-0001",
        },
    )
}

fn hora_component(natal: &Natal, life: Life, hora: &Hora) -> Component {
    let p = natal.facts.planet(hora.lord);
    let d = dignity(hora.lord, p.sidereal).score();
    let score = (life.hora_affinity(hora.lord) * (0.5 + 0.5 * d)).clamp(0.0, 1.0);
    Component {
        key: "hora",
        score,
        weight: 0.13,
        note: format!("{:?} hora: {}", hora.lord, hora.words),
        rule_id: "KS-H-0001",
    }
}

fn kala_component(sky: &Sky, day: Option<&SolarDay>, obs: &Observer, at: Instant) -> Component {
    let n = nakshatra(sky.moon_sidereal);
    let ti = tithi(sky.moon_sidereal, sky.sun_sidereal);
    let k = karana(sky.moon_sidereal, sky.sun_sidereal);
    let mut score: f64 = 0.7;
    let mut notes: Vec<String> = Vec::new();
    if let Some(d) = day {
        let wd = weekday(d, obs) as usize;
        if in_window(at, rahu_kala(d, obs)) {
            score = 0.1;
            notes.push("Rahu kala: finish, do not begin".into());
        } else if in_window(at, slot_window(d, YAMAGANDA[wd])) {
            score = 0.15;
            notes.push("Yamaganda: a pause, not a start".into());
        } else if in_window(at, slot_window(d, GULIKA[wd])) {
            score = 0.2;
            notes.push("Gulika kala: let things stand".into());
        } else if in_window(at, abhijit_window(d)) {
            score = 1.0;
            notes.push("Abhijit muhurta: the day's winning minutes".into());
        }
    }
    if GOOD_STARS.contains(&n.index) {
        score += 0.15;
        notes.push(format!("{} is an auspicious star", n.name()));
    }
    if matches!(ti.number % 15, 4 | 9 | 14) || ti.number == 30 {
        score -= 0.2;
        notes.push(format!("{} is a rikta tithi", ti.name()));
    }
    if k.name == "Vishti" {
        score -= 0.2;
        notes.push("Vishti (Bhadra) karana".into());
    }
    if notes.is_empty() {
        notes.push(format!(
            "{} {} tithi, {} star",
            if ti.is_shukla() { "Shukla" } else { "Krishna" },
            TITHI_NAMES[((ti.number - 1) % 15) as usize],
            n.name()
        ));
    }
    Component {
        key: "kala",
        score: score.clamp(0.0, 1.0),
        weight: 0.10,
        note: notes.join(" · "),
        rule_id: "KS-M-0001",
    }
}

fn combine(components: &[Component]) -> f64 {
    let den: f64 = components.iter().map(|c| c.weight).sum();
    if den <= 0.0 {
        return 0.5;
    }
    (components.iter().map(|c| c.weight * c.score).sum::<f64>() / den).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------------

/// One plain sentence for a life at a band. Several variants; `seed` picks one so the
/// wording changes from day to day but stays put within a day.
pub fn headline(life: Life, band: Band, seed: u64) -> &'static str {
    let v: &[&str] = match (life, band) {
        (Life::Love, Band::Bright) => &[
            "Love is lit up. Say the thing you have been holding.",
            "A warm, open window for the heart. Reach out first.",
        ],
        (Life::Love, Band::Good) => &[
            "Good hours for closeness. Small gestures land well.",
            "The heart is well placed. Make time for the one who matters.",
        ],
        (Life::Love, Band::Steady) => &[
            "Love is even. Keep it simple and kind.",
            "Nothing pushes, nothing pulls. Enjoy the quiet.",
        ],
        (Life::Love, Band::Slow) => &[
            "Give love some room. Listen more than you speak.",
            "Feelings run close to the surface. Go gently.",
        ],
        (Life::Love, Band::Hold) => &[
            "Hold the big conversations for later.",
            "Not the hour to settle matters of the heart. Wait.",
        ],
        (Life::Family, Band::Bright) => &[
            "Home shines. A good time to gather and to mend.",
            "Family bonds are strong now. Make the call.",
        ],
        (Life::Family, Band::Good) => &[
            "Good for family time and small decisions at home.",
            "Elders and children both respond well now.",
        ],
        (Life::Family, Band::Steady) => &[
            "Home is calm. Keep routines and let it stay so.",
            "A plain, settled stretch at home.",
        ],
        (Life::Family, Band::Slow) => &[
            "Tempers at home run short. Do not take the bait.",
            "Family needs patience more than answers right now.",
        ],
        (Life::Family, Band::Hold) => &[
            "Avoid family disputes and big household moves.",
            "Keep the peace at home; decide nothing lasting today.",
        ],
        (Life::Career, Band::Bright) => &[
            "Work moves your way. Ask, pitch, apply.",
            "Authority listens now. Put your name forward.",
        ],
        (Life::Career, Band::Good) => &[
            "A productive stretch. Finish what earns notice.",
            "Good for meetings, interviews and plain hard work.",
        ],
        (Life::Career, Band::Steady) => &[
            "Work is steady. Clear the list, make no big bets.",
            "Keep your head down; the gains are quiet ones.",
        ],
        (Life::Career, Band::Slow) => &[
            "Delays at work. Double-check before you send.",
            "Effort outweighs reward for now. Pace yourself.",
        ],
        (Life::Career, Band::Hold) => &[
            "Do not sign, resign or confront today.",
            "Hold career moves. The ground is not firm.",
        ],
        (Life::Money, Band::Bright) => &[
            "Money flows in. A fine time to receive and to invest wisely.",
            "Gains are favoured. Close the deal.",
        ],
        (Life::Money, Band::Good) => &[
            "Good for earning and sensible spending.",
            "Accounts balance well now. Collect what is owed.",
        ],
        (Life::Money, Band::Steady) => &[
            "Money is level. Spend as planned, no more.",
            "Neither windfall nor leak. Keep the budget.",
        ],
        (Life::Money, Band::Slow) => &[
            "Watch the wallet. Avoid loans and impulse buys.",
            "Expenses rise before income does. Wait.",
        ],
        (Life::Money, Band::Hold) => &[
            "No big purchases, no lending, no new debt.",
            "Hold your money close today.",
        ],
        (Life::Health, Band::Bright) => &[
            "Energy is high. A good day to start a healthy habit.",
            "Body and mind are in step. Move, breathe, rest well.",
        ],
        (Life::Health, Band::Good) => &[
            "Good vitality. Keep meals and sleep regular.",
            "A sound stretch for check-ups and recovery.",
        ],
        (Life::Health, Band::Steady) => &[
            "Health is steady. Do not overdo it.",
            "An ordinary day for the body. Keep the routine.",
        ],
        (Life::Health, Band::Slow) => &[
            "Energy dips. Rest early and eat light.",
            "Mind the small aches before they grow.",
        ],
        (Life::Health, Band::Hold) => &[
            "Take care today. Avoid risk, strain and late nights.",
            "Slow down and protect your rest.",
        ],
        (Life::Study, Band::Bright) => &[
            "The mind is sharp. Learn, write, sit the test.",
            "Ideas come easily. Study the hard chapter now.",
        ],
        (Life::Study, Band::Good) => &[
            "Good focus. Revise, read, ask questions.",
            "A clear-headed stretch for learning.",
        ],
        (Life::Study, Band::Steady) => &[
            "Steady attention. Practice rather than new topics.",
            "Routine study pays; nothing brilliant, nothing lost.",
        ],
        (Life::Study, Band::Slow) => &[
            "Concentration wanders. Short sessions, more breaks.",
            "Details slip now. Re-read before you submit.",
        ],
        (Life::Study, Band::Hold) => &[
            "Not the hour for exams or decisions about courses.",
            "Let the mind rest; cramming will not stick.",
        ],
        (Life::Home, Band::Bright) => &[
            "Property and home matters move well. Sign, build, shift.",
            "A strong time for the house: repairs, purchase, rent.",
        ],
        (Life::Home, Band::Good) => &[
            "Good for house work and paperwork about property.",
            "Home projects progress. Measure, plan, fix.",
        ],
        (Life::Home, Band::Steady) => &[
            "Home matters are stable. Maintain, do not expand.",
            "A quiet day for the house.",
        ],
        (Life::Home, Band::Slow) => &[
            "Expect small delays with land, rent or repairs.",
            "Check documents twice before anything about property.",
        ],
        (Life::Home, Band::Hold) => &[
            "No property deals today. Wait for a better hour.",
            "Hold moves, purchases and agreements about the house.",
        ],
        (Life::Travel, Band::Bright) => &[
            "Journeys go smoothly. Book it, go.",
            "Roads open. A fine time to travel or to plan abroad.",
        ],
        (Life::Travel, Band::Good) => &[
            "Good for short trips and travel paperwork.",
            "Movement is favoured. Leave a little early.",
        ],
        (Life::Travel, Band::Steady) => &[
            "Travel is plain. Allow the usual margin.",
            "Nothing special on the road either way.",
        ],
        (Life::Travel, Band::Slow) => &[
            "Delays on the way. Carry patience and a charger.",
            "Not ideal for long journeys. Keep plans flexible.",
        ],
        (Life::Travel, Band::Hold) => &[
            "Avoid starting a long journey now.",
            "Postpone travel decisions; the timing is against them.",
        ],
        (Life::Children, Band::Bright) => &[
            "Joy from children. A good time for their milestones.",
            "Matters of children are blessed now.",
        ],
        (Life::Children, Band::Good) => &[
            "Good for time with children and their plans.",
            "Children respond well; teach, play, listen.",
        ],
        (Life::Children, Band::Steady) => &[
            "An even stretch for children. Keep the routine.",
            "Nothing pressing with the young ones.",
        ],
        (Life::Children, Band::Slow) => &[
            "Children need extra patience now.",
            "Small worries about the young ones; stay close.",
        ],
        (Life::Children, Band::Hold) => &[
            "Avoid big decisions about children today.",
            "Be gentle and watchful with the young ones.",
        ],
    };
    v[(seed as usize) % v.len()]
}

fn moment_words(band: Band, hora: Option<&Hora>, tara: usize) -> String {
    let lead = match band {
        Band::Bright => "A bright stretch",
        Band::Good => "A good stretch",
        Band::Steady => "An even stretch",
        Band::Slow => "A slow stretch",
        Band::Hold => "A stretch to hold still",
    };
    let mut s = lead.to_string();
    if let Some(h) = hora {
        s.push_str(&format!(" · {:?} hour", h.lord));
    }
    s.push_str(" · ");
    s.push_str(TARA_NAMES[tara]);
    s.push_str(" day");
    s
}

fn because(components: &[Component]) -> String {
    components
        .iter()
        .max_by(|a, b| {
            let fa = (a.score - 0.5).abs() * a.weight;
            let fb = (b.score - 0.5).abs() * b.weight;
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|c| c.note.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------------
// The moment
// ---------------------------------------------------------------------------------

/// Everything for one instant at a place. `system` is the house system for the sky of
/// the moment (the birth chart's facts are already in `natal`).
#[allow(clippy::too_many_arguments)]
pub fn moment(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<Moment, ForecastError> {
    let sky = sky(spk, eop, natal, obs, system, ayanamsa, at)?;
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 5);
    let day = solar_day(spk, eop, obs, at).ok().flatten();
    let hora = day.as_ref().and_then(|d| {
        let (name, s, e) = hora_at(d, obs, at);
        hora_lord(name).map(|lord| Hora {
            lord,
            start: s,
            end: e,
            words: hora_words(lord),
        })
    });
    let (tara, tara_c) = tara_component(natal, &sky);
    let kala_c = kala_component(&sky, day.as_ref(), obs, at);
    let seed = (at.tai_ns() / NS_PER_DAY) as u64;

    let lives: Vec<LifeScore> = Life::ALL
        .iter()
        .map(|&life| {
            let mut comps = vec![
                natal_component(natal, life),
                dasha_component(natal, life, &levels),
                gochara_component(natal, &sky, life),
                tara_c.clone(),
            ];
            if let Some(h) = &hora {
                comps.push(hora_component(natal, life, h));
            }
            comps.push(kala_c.clone());
            let score = combine(&comps);
            let band = Band::of(score);
            LifeScore {
                life,
                score,
                band,
                headline: headline(life, band, seed).to_string(),
                because: because(&comps),
                components: comps,
            }
        })
        .collect();
    let overall = lives.iter().map(|l| l.score).sum::<f64>() / lives.len() as f64;
    let band = Band::of(overall);

    // Windows of the day.
    let mut windows = Vec::new();
    if let Some(d) = &day {
        let wd = weekday(d, obs) as usize;
        let (rs, re) = rahu_kala(d, obs);
        windows.push(Window {
            label: "Rahu kala".into(),
            start: rs,
            end: re,
            caution: true,
        });
        let (ys, ye) = slot_window(d, YAMAGANDA[wd]);
        windows.push(Window {
            label: "Yamaganda".into(),
            start: ys,
            end: ye,
            caution: true,
        });
        let (gs, ge) = slot_window(d, GULIKA[wd]);
        windows.push(Window {
            label: "Gulika kala".into(),
            start: gs,
            end: ge,
            caution: true,
        });
        let (as_, ae) = abhijit_window(d);
        windows.push(Window {
            label: "Abhijit muhurta".into(),
            start: as_,
            end: ae,
            caution: false,
        });
        windows.sort_by(|a, b| a.start.cmp(&b.start));
    }

    // Upcoming shifts.
    let horizon = at.add_ns(NS_PER_DAY + 6 * 3_600 * NS_PER_SEC);
    let mut shifts: Vec<Shift> = Vec::new();
    if let Some(d) = &day {
        // every hora boundary until the next sunrise
        let mut t = at;
        for _ in 0..26 {
            let (_, _, e) = hora_at(d, obs, t);
            if e <= t || e >= d.next_sunrise {
                break;
            }
            let (next, _, _) = hora_at(d, obs, e.add_ns(NS_PER_SEC));
            shifts.push(Shift {
                at: e,
                what: format!("{next} hour begins"),
                kind: "hora",
            });
            t = e.add_ns(NS_PER_SEC);
        }
        for w in &windows {
            if w.start > at {
                shifts.push(Shift {
                    at: w.start,
                    what: format!("{} begins", w.label),
                    kind: "kala",
                });
            }
            if w.end > at {
                shifts.push(Shift {
                    at: w.end,
                    what: format!("{} ends", w.label),
                    kind: "kala",
                });
            }
        }
        if d.sunset > at {
            shifts.push(Shift {
                at: d.sunset,
                what: "Sunset".into(),
                kind: "sun",
            });
        }
        shifts.push(Shift {
            at: d.next_sunrise,
            what: "Sunrise: a new day begins".into(),
            kind: "sun",
        });
    }
    if let Ok(v) = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, horizon, MICROSECOND_NS) {
        let cur = nakshatra(sky.moon_sidereal).index;
        for (i, e) in v.iter().enumerate().take(2) {
            let next = (cur + 1 + i) % 27;
            let t = tara_index(natal.janma(), next);
            shifts.push(Shift {
                at: e.root.t,
                what: format!("Moon enters {} · {} tara", NAKSHATRA_NAMES[next], TARA_NAMES[t]),
                kind: "star",
            });
        }
    }
    if let Ok(v) = tithi_boundaries(spk, at, horizon, MICROSECOND_NS) {
        let cur = tithi(sky.moon_sidereal, sky.sun_sidereal).number;
        for (i, e) in v.iter().enumerate().take(2) {
            let n = (cur as usize + i) % 30 + 1;
            shifts.push(Shift {
                at: e.root.t,
                what: format!(
                    "{} {} tithi begins",
                    if n <= 15 { "Shukla" } else { "Krishna" },
                    TITHI_NAMES[(n - 1) % 15]
                ),
                kind: "tithi",
            });
        }
    }
    {
        let next_sign = ((sky.moon_sidereal / 30.0).floor() as i64 + 1).rem_euclid(12);
        let target = (next_sign as f64 * 30.0 + sky.ayanamsa_deg).rem_euclid(360.0);
        if let Ok(v) = longitude_crossings(
            spk,
            Body::Moon,
            target,
            at,
            at.add_ns(3 * NS_PER_DAY),
            MICROSECOND_NS,
        ) {
            if let Some(e) = v.first() {
                let h = ((next_sign - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                shifts.push(Shift {
                    at: e.root.t,
                    what: format!(
                        "Moon enters {} · the {} from your Moon",
                        SIGN_NAMES[next_sign as usize],
                        ordinal(h)
                    ),
                    kind: "sign",
                });
            }
        }
    }
    for (i, p) in levels.iter().enumerate().skip(2) {
        if p.end > at && p.end <= horizon {
            // the lord that follows at this level
            let idx = DashaLord::ORDER.iter().position(|l| *l == p.lord).unwrap_or(0);
            let next = DashaLord::ORDER[(idx + 1) % 9];
            shifts.push(Shift {
                at: p.end,
                what: format!("{next:?} {} begins", LEVEL_NAMES[i]),
                kind: "dasha",
            });
        }
    }
    shifts.retain(|s| s.at > at && s.at <= horizon);
    shifts.sort_by(|a, b| a.at.cmp(&b.at));

    let moon_star = {
        let n = nakshatra(sky.moon_sidereal);
        format!("{} {}", n.name(), n.pada)
    };
    // The day's twenty-four planetary hours and how each suits each life.
    let mut hours = Vec::new();
    if let Some(d) = &day {
        let cautions: Vec<(Instant, Instant)> = windows
            .iter()
            .filter(|w| w.caution)
            .map(|w| (w.start, w.end))
            .collect();
        let mut t = d.sunrise.add_ns(NS_PER_SEC);
        for _ in 0..24 {
            let (name, s, e) = hora_at(d, obs, t);
            if e <= s {
                break;
            }
            if let Some(lord) = hora_lord(name) {
                let h = Hora {
                    lord,
                    start: s,
                    end: e,
                    words: hora_words(lord),
                };
                hours.push(HourSlot {
                    lord,
                    start: s,
                    end: e,
                    fit: Life::ALL
                        .iter()
                        .map(|&l| hora_component(natal, l, &h).score)
                        .collect(),
                    caution: cautions.iter().any(|&(cs, ce)| s < ce && e > cs),
                });
            }
            if e >= d.next_sunrise {
                break;
            }
            t = e.add_ns(NS_PER_SEC);
        }
    }
    Ok(Moment {
        hours,
        at,
        overall,
        band,
        headline: moment_words(band, hora.as_ref(), tara),
        lives,
        hora,
        moon_star,
        tara,
        tara_words: tara_words(tara),
        moon_from_moon: sky.from_moon[1],
        saturn: SaturnPhase::from_house(sky.from_moon[6]),
        dasha: levels,
        windows,
        shifts,
        sunrise: day.as_ref().map(|d| d.sunrise),
        sunset: day.as_ref().map(|d| d.sunset),
    })
}

// ---------------------------------------------------------------------------------
// Series and stories
// ---------------------------------------------------------------------------------

/// `count` samples from `from`, `step_ns` apart, with the day-level components only.
#[allow(clippy::too_many_arguments)]
pub fn series(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    from: Instant,
    step_ns: i128,
    count: usize,
) -> Result<Vec<Sample>, ForecastError> {
    let mut out = Vec::with_capacity(count);
    for k in 0..count {
        let at = from.add_ns(step_ns * k as i128);
        let sky = sky(spk, eop, natal, obs, HouseSystem::WholeSign, ayanamsa, at)?;
        let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
        let (tara, tara_c) = tara_component(natal, &sky);
        let scores = Life::ALL
            .iter()
            .map(|&life| {
                combine(&[
                    natal_component(natal, life),
                    dasha_component(natal, life, &levels),
                    gochara_component(natal, &sky, life),
                    tara_c.clone(),
                ])
            })
            .collect();
        out.push(Sample {
            at,
            scores,
            dasha: levels.iter().map(|p| p.lord).collect(),
            houses_from_moon: sky.from_moon,
            tara,
        });
    }
    Ok(out)
}

/// The story of one life over a series.
pub fn story(life: Life, samples: &[Sample]) -> Story {
    let idx = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
    let n = samples.len();
    if n == 0 {
        return Story {
            life,
            trend: "steady",
            trend_words: String::new(),
            mean: 0.5,
            best: None,
            low: None,
            shifts: Vec::new(),
        };
    }
    let s: Vec<f64> = samples.iter().map(|x| x.scores[idx]).collect();
    let mean = s.iter().sum::<f64>() / n as f64;
    let third = (n / 3).max(1);
    let head = s[..third].iter().sum::<f64>() / third as f64;
    let tail = s[n - third..].iter().sum::<f64>() / third as f64;
    let trend = if tail - head > 0.06 {
        "rising"
    } else if head - tail > 0.06 {
        "falling"
    } else {
        "steady"
    };
    let run = |pick_max: bool| -> Option<(Instant, Instant, f64)> {
        let (i, v) = s
            .iter()
            .enumerate()
            .fold(None, |acc: Option<(usize, f64)>, (i, &v)| match acc {
                None => Some((i, v)),
                Some((_, bv)) if (pick_max && v > bv) || (!pick_max && v < bv) => Some((i, v)),
                a => a,
            })?;
        let ok = |x: f64| {
            if pick_max {
                x >= v - 0.05
            } else {
                x <= v + 0.05
            }
        };
        let mut a = i;
        while a > 0 && ok(s[a - 1]) {
            a -= 1;
        }
        let mut b = i;
        while b + 1 < n && ok(s[b + 1]) {
            b += 1;
        }
        Some((samples[a].at, samples[b].at, v))
    };
    let best = run(true);
    let low = run(false);

    let mut shifts = Vec::new();
    for w in samples.windows(2) {
        let (p, q) = (&w[0], &w[1]);
        for (lvl, (a, b)) in p.dasha.iter().zip(q.dasha.iter()).enumerate() {
            if a != b {
                shifts.push(Shift {
                    at: q.at,
                    what: format!("{b:?} {} begins", LEVEL_NAMES[lvl]),
                    kind: "dasha",
                });
                break;
            }
        }
        for (i, &g) in GRAHAS.iter().enumerate() {
            if p.houses_from_moon[i] != q.houses_from_moon[i]
                && life.relevance(g) >= 0.5
                && g != DashaLord::Moon
            {
                let h = q.houses_from_moon[i];
                let tone = if favourable_from_moon(g).contains(&h) {
                    "favourable"
                } else if adverse_from_moon(g).contains(&h) {
                    "testing"
                } else {
                    "neutral"
                };
                shifts.push(Shift {
                    at: q.at,
                    what: format!(
                        "{:?} moves into the {} from your Moon ({tone} for {})",
                        g,
                        ordinal(h),
                        life.name().to_ascii_lowercase()
                    ),
                    kind: "sign",
                });
            }
        }
    }
    shifts.truncate(6);
    let trend_words = match (trend, Band::of(mean)) {
        ("rising", _) => format!("{} improves as the period goes on.", life.name()),
        ("falling", _) => format!("{} starts stronger than it ends; act early.", life.name()),
        (_, Band::Bright) | (_, Band::Good) => {
            format!("{} holds a good level throughout.", life.name())
        }
        (_, Band::Steady) => format!("{} stays even; no big swings.", life.name()),
        _ => format!("{} stays slow; patience is the work.", life.name()),
    };
    Story {
        life,
        trend,
        trend_words,
        mean,
        best,
        low,
        shifts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn natal(spk: &Spk) -> (Natal, Observer) {
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            spk,
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
        (Natal::new(Facts::from_chart(&ch), birth), obs)
    }

    #[test]
    fn every_life_is_scored_and_explained() {
        let Some(spk) = kernel() else { return };
        let (natal, obs) = natal(&spk);
        let at = Utc::new(2026, 10, 16, 5, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let m = moment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
        assert_eq!(m.lives.len(), 9);
        for l in &m.lives {
            assert!((0.0..=1.0).contains(&l.score), "{:?}", l);
            assert!(!l.headline.is_empty());
            assert!(!l.because.is_empty());
            assert_eq!(l.components.len(), 6, "hora present at Kochi");
            let w: f64 = l.components.iter().map(|c| c.weight).sum();
            assert!((w - 1.0).abs() < 1e-9);
        }
        assert!(!m.shifts.is_empty());
        assert!(m.shifts.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(m.shifts.iter().all(|s| s.at > at));
        assert_eq!(m.hours.len(), 24);
        assert_eq!(m.windows.len(), 4);
        assert!(m.hora.is_some());
    }

    #[test]
    fn series_and_story_are_consistent() {
        let Some(spk) = kernel() else { return };
        let (natal, obs) = natal(&spk);
        let from = Utc::new(2026, 10, 16, 5, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let s = series(&spk, None, &natal, &obs, Ayanamsa::Lahiri, from, NS_PER_DAY, 10).unwrap();
        assert_eq!(s.len(), 10);
        for life in Life::ALL {
            let st = story(life, &s);
            assert!((0.0..=1.0).contains(&st.mean));
            let (a, b, v) = st.best.unwrap();
            assert!(a <= b && (0.0..=1.0).contains(&v));
            assert!(matches!(st.trend, "rising" | "falling" | "steady"));
        }
    }

    #[test]
    fn tara_counts_from_the_birth_star() {
        assert_eq!(tara_index(18, 18), 0); // Janma
        assert_eq!(tara_index(18, 19), 1); // Sampat
        assert_eq!(tara_index(18, 26), 8); // Parama Mitra
        assert_eq!(tara_index(18, 0), 0); // tenth star: Janma again
        assert_eq!(tara_index(26, 0), 1);
    }

    #[test]
    fn bands_are_monotone() {
        assert_eq!(Band::of(0.9), Band::Bright);
        assert_eq!(Band::of(0.6), Band::Good);
        assert_eq!(Band::of(0.5), Band::Steady);
        assert_eq!(Band::of(0.35), Band::Slow);
        assert_eq!(Band::of(0.1), Band::Hold);
        assert!(Band::Bright > Band::Hold);
    }
}
