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
use kshana_vedic::panchanga::TITHI_NAMES;
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
    /// The moment's name: "Sampat under the Moon's hour, waxing".
    pub title: String,
    /// The day's name: "Sampat Shashthi, the Moon's day".
    pub day_title: String,
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

    /// The natal Moon's sidereal sign, 0 = Aries.
    pub fn moon_sign(&self) -> i64 {
        (self.moon_sidereal / 30.0).floor() as i64
    }

    /// The birth star, 0 = Ashwini.
    pub fn janma(&self) -> usize {
        nakshatra(self.moon_sidereal).index
    }
}

/// Errors.
#[derive(Debug)]
pub enum ForecastError {
    /// The chart could not be computed.
    Chart(String),
}

pub(crate) const GRAHAS: [DashaLord; 9] = [
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

#[allow(dead_code)]
pub(crate) const LEVEL_NAMES: [&str; 5] = ["mahadasha", "antardasha", "pratyantar", "sookshma", "prana"];
pub(crate) const LEVEL_WEIGHTS: [f64; 5] = [0.35, 0.30, 0.20, 0.10, 0.05];

/// "3 Jun 2027" for an instant, on the local mean date at `longitude_deg`.
pub(crate) fn date_words(t: Instant, longitude_deg: f64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let local = t.add_ns((longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    let u = Utc::from_instant(local);
    format!("{} {} {}", u.day, MONTHS[(u.month as usize - 1).min(11)], u.year)
}

/// The universally auspicious stars (Muhurta Chintamani), 0-based.
pub(crate) const GOOD_STARS: [usize; 15] = [0, 3, 4, 6, 7, 11, 12, 13, 14, 16, 20, 21, 22, 25, 26];
/// Yamaganda and Gulika slots (1-based eighth of the daytime) by weekday Sun..Sat.
pub(crate) const YAMAGANDA: [usize; 7] = [5, 4, 3, 2, 1, 7, 6];
pub(crate) const GULIKA: [usize; 7] = [7, 6, 5, 4, 3, 2, 1];

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

#[allow(dead_code)]
pub(crate) fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

pub(crate) fn weekday(day: &SolarDay, obs: &Observer) -> u32 {
    let local = day
        .sunrise
        .add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

pub(crate) fn slot_window(day: &SolarDay, slot: usize) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise) / 8;
    (
        day.sunrise.add_ns((slot as i128 - 1) * len),
        day.sunrise.add_ns(slot as i128 * len),
    )
}

pub(crate) fn abhijit_window(day: &SolarDay) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise);
    let noon = day.sunrise.add_ns(len / 2);
    (noon.add_ns(-len / 30), noon.add_ns(len / 30))
}

fn in_window(t: Instant, w: (Instant, Instant)) -> bool {
    t >= w.0 && t < w.1
}

/// The sky of the moment, as facts, plus each graha's house from the natal Moon.
pub(crate) struct Sky {
    pub(crate) facts: Facts,
    pub(crate) from_moon: [u8; 9],
    pub(crate) moon_sidereal: f64,
    pub(crate) sun_sidereal: f64,
    pub(crate) ayanamsa_deg: f64,
}

impl Sky {
    /// Each graha's sidereal sign, Sun..Ketu.
    pub(crate) fn signs(&self) -> [u8; 9] {
        std::array::from_fn(|i| self.facts.planet(GRAHAS[i]).sign)
    }
}

pub(crate) fn sky(
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
    natal_component_for(natal, life.area(), &life.name().to_ascii_lowercase())
}

/// The same for any KP house group (a topic of the question dictionary), named `name`.
pub fn natal_component_for(natal: &Natal, area: Area, name: &str) -> Component {
    let p = crate::kp::promise(&natal.facts, area);
    let b = crate::parashari::assess(&natal.facts, area);
    let kp = (0.5 + 0.5 * p.net.clamp(-1.0, 1.0)).clamp(0.0, 1.0);
    let score = (0.5 * kp + 0.5 * b.score).clamp(0.0, 1.0);
    let n = crate::plain::cap(&crate::plain::subject(name));
    let note = match (p.promised, b.score >= 0.5) {
        (true, true) => format!("{n} is clearly promised in your life; it comes to you more easily than to most"),
        (true, false) => format!("{n} is promised in your life, though it arrives with some effort"),
        (false, true) => format!("{n} is possible for you with steady effort; it is not handed over"),
        (false, false) => format!("{n} is hard-won in your life; patience and effort decide it"),
    };
    Component {
        key: "natal",
        score,
        weight: 0.15,
        note,
        rule_id: "KS-N-0001",
    }
}

/// One running period lord's bearing on a life.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DashaItem {
    pub(crate) lord: DashaLord,
    /// 0 = mahadasha … 4 = prana.
    pub(crate) level: usize,
    /// The lord's score for the life in `[0, 1]`: half KP signification, half Parashari quality.
    pub(crate) score: f64,
    /// Level weight.
    pub(crate) weight: f64,
    pub(crate) words: String,
    pub(crate) start: Instant,
    pub(crate) end: Instant,
}

/// How one lord scores for a life, regardless of level.
pub(crate) fn lord_score(natal: &Natal, life: Life, lord: DashaLord) -> (f64, String) {
    let area = life.area();
    let sup = period_supports(&natal.facts, area, lord);
    let kp = sup
        .iter()
        .map(|s| s.level.weight())
        .fold(0.0_f64, f64::max);
    let (pq, _) = period_quality(&natal.facts, area, lord);
    let houses = sup
        .iter()
        .map(|s| s.house.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    (0.5 * kp + 0.5 * pq, houses)
}

pub(crate) fn dasha_items(natal: &Natal, life: Life, levels: &[DashaPeriod]) -> Vec<DashaItem> {
    levels
        .iter()
        .enumerate()
        .take(5)
        .map(|(i, p)| {
            let (s, houses) = lord_score(natal, life, p.lord);
            let level = ["chapter of your life", "stretch of your life", "shorter phase of a few months", "phase of a few weeks", "phase of a few days"][i.min(4)];
            let name = life.name().to_ascii_lowercase();
            let words = if houses.is_empty() {
                format!("this {level} leaves {name} to run on its own")
            } else if s >= 0.5 {
                format!("this {level} brings {} to {name}", crate::plain::support(p.lord))
            } else {
                format!("this {level} brings {} to {name}", crate::plain::pressure(p.lord))
            };
            DashaItem {
                lord: p.lord,
                level: i,
                score: s,
                weight: LEVEL_WEIGHTS[i],
                words,
                start: p.start,
                end: p.end,
            }
        })
        .collect()
}

fn dasha_component(natal: &Natal, life: Life, levels: &[DashaPeriod]) -> Component {
    let items = dasha_items(natal, life, levels);
    let mut num = 0.0;
    let mut den = 0.0;
    let mut best: Option<(f64, String)> = None;
    let mut worst: Option<(f64, String)> = None;
    for it in &items {
        num += it.weight * it.score;
        den += it.weight;
        let v = it.weight * (it.score - 0.5);
        if best.as_ref().map_or(true, |b| v > b.0) {
            best = Some((v, it.words.clone()));
        }
        if worst.as_ref().map_or(true, |b| v < b.0) {
            worst = Some((v, it.words.clone()));
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

/// One transiting graha's bearing on a life.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GocharaItem {
    pub(crate) graha: DashaLord,
    /// House from the natal Moon, 1–12.
    pub(crate) house: u8,
    /// Relevance of the graha to the life, after the ownership bonus.
    pub(crate) relevance: f64,
    /// `relevance × value × ashtakavarga factor`, in `[-1.25, 1.25]`.
    pub(crate) contrib: f64,
    pub(crate) words: String,
    pub(crate) saturn_phase: SaturnPhase,
    pub(crate) favourable: bool,
    pub(crate) adverse: bool,
    pub(crate) obstructed: bool,
}

/// Every graha's transit bearing on a life, given each graha's house from the natal Moon
/// and its sidereal sign (Sun..Ketu). Returns the items and the sum of relevances.
pub(crate) fn gochara_items(
    natal: &Natal,
    from_moon: &[u8; 9],
    signs: &[u8; 9],
    life: Life,
) -> (Vec<GocharaItem>, f64) {
    let mut items = Vec::with_capacity(9);
    let mut den = 0.0;
    for (i, &g) in GRAHAS.iter().enumerate() {
        let h = from_moon[i];
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
                    && GRAHAS[..7]
                        .iter()
                        .enumerate()
                        .any(|(j, &p)| p != g && !vedha_exempt(g, p) && from_moon[j] == vh)
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
            Some(k) => match natal.bav[k][signs[i] as usize % 12] {
                b if b >= 5 => 1.25,
                4 => 1.0,
                _ => 0.6,
            },
            None => 1.0,
        };
        let contrib = r * v * av;
        den += r;
        let words = if saturn_phase != SaturnPhase::Clear {
            format!("{} is running", saturn_phase.words())
        } else if g == DashaLord::Moon && h == 8 {
            "these are the two heaviest days of your month: moods dip and plans slip".to_string()
        } else if fav && obstructed {
            format!("a season of {} is held back for now", crate::plain::support(g))
        } else if fav {
            format!("the present season brings {}", crate::plain::support(g))
        } else if adverse {
            format!("the present season brings {}", crate::plain::pressure(g))
        } else {
            "a neutral season, neither helping nor hindering".to_string()
        };
        items.push(GocharaItem {
            graha: g,
            house: h,
            relevance: r,
            contrib,
            words,
            saturn_phase,
            favourable: fav,
            adverse,
            obstructed,
        });
    }
    (items, den)
}

fn gochara_component(natal: &Natal, sky: &Sky, life: Life) -> Component {
    let (items, den) = gochara_items(natal, &sky.from_moon, &sky.signs(), life);
    let mut num = 0.0;
    let mut best: Option<(f64, String)> = None;
    let mut worst: Option<(f64, String)> = None;
    for it in &items {
        num += it.contrib;
        if best.as_ref().map_or(true, |b| it.contrib > b.0) {
            best = Some((it.contrib, it.words.clone()));
        }
        if worst.as_ref().map_or(true, |b| it.contrib < b.0) {
            worst = Some((it.contrib, it.words.clone()));
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

/// How much each graha's passage through a house is felt, Sun..Ketu: the Moon for the
/// rhythm of days, Jupiter and Saturn for the shape of a year.
const BHAVA_WEIGHT: [f64; 9] = [0.5, 1.0, 0.7, 0.5, 1.0, 0.7, 1.0, 0.6, 0.5];

/// The life's own timing: the grahas passing through the houses that carry this part of
/// life, counted from the rising sign at birth (Parashari bhava gochara). A natural
/// benefic (Moon, Mercury, Jupiter, Venus) crossing them nourishes it; a natural malefic
/// (Sun, Mars, Saturn, Rahu, Ketu) presses on it, most of all on the main house. Unlike
/// the transits counted from the Moon, this differs from one life to the next, so each
/// life has its own good and hard days. `with_moon` is false for the slow, monthly view.
pub(crate) fn bhava_component(natal: &Natal, signs: &[u8; 9], life: Life, with_moon: bool) -> Component {
    let area = life.area();
    let lagna = natal.facts.lagna_sign as i64;
    let mut num = 0.0;
    let mut den = 0.0;
    for (i, &g) in GRAHAS.iter().enumerate() {
        if g == DashaLord::Moon && !with_moon {
            continue;
        }
        let h = ((signs[i] as i64 - lagna).rem_euclid(12) + 1) as u8;
        let main = h == area.primary;
        let in_group = main || area.group.contains(&h);
        let in_neg = area.negating.contains(&h);
        if !in_group && !in_neg {
            continue;
        }
        let benefic = matches!(g, DashaLord::Moon | DashaLord::Mercury | DashaLord::Jupiter | DashaLord::Venus);
        let w = BHAVA_WEIGHT[i];
        let v = match (in_group, benefic) {
            (true, true) => if main { 1.0 } else { 0.6 },
            (true, false) => if main { -1.0 } else { -0.4 },
            (false, true) => -0.3,
            (false, false) => 0.2,
        };
        num += w * v;
        den += w;
    }
    let den = den.max(2.0);
    let score = (0.5 + 0.5 * num / den).clamp(0.0, 1.0);
    let who = crate::plain::subject(life.name()).to_ascii_lowercase();
    let note = if score >= 0.58 {
        format!("the timing in your own birth chart favours {who} just now")
    } else if score <= 0.42 {
        format!("{who} is going through a slow patch of its own in your birth chart")
    } else {
        format!("nothing in your own birth chart pushes {who} either way just now")
    };
    Component { key: "bhava", score, weight: 0.22, note, rule_id: "KS-B-0001" }
}

pub(crate) fn tara_component(natal: &Natal, sky: &Sky) -> (usize, Component) {
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

pub(crate) fn hora_component(natal: &Natal, life: Life, hora: &Hora) -> Component {
    let p = natal.facts.planet(hora.lord);
    let d = dignity(hora.lord, p.sidereal).score();
    let score = (life.hora_affinity(hora.lord) * (0.5 + 0.5 * d)).clamp(0.0, 1.0);
    Component {
        key: "hora",
        score,
        weight: 0.13,
        note: format!("this is {}", crate::plain::hour_nature(hora.lord)),
        rule_id: "KS-H-0001",
    }
}

pub(crate) fn kala_component(sky: &Sky, day: Option<&SolarDay>, obs: &Observer, at: Instant) -> Component {
    let n = nakshatra(sky.moon_sidereal);
    let ti = tithi(sky.moon_sidereal, sky.sun_sidereal);
    let k = karana(sky.moon_sidereal, sky.sun_sidereal);
    let mut score: f64 = 0.7;
    let mut notes: Vec<String> = Vec::new();
    if let Some(d) = day {
        let wd = weekday(d, obs) as usize;
        if in_window(at, rahu_kala(d, obs)) {
            score = 0.1;
            notes.push("the day's unlucky stretch: finish what you started, begin nothing".into());
        } else if in_window(at, slot_window(d, YAMAGANDA[wd])) {
            score = 0.15;
            notes.push("a risky stretch of the day: pause, do not start".into());
        } else if in_window(at, slot_window(d, GULIKA[wd])) {
            score = 0.2;
            notes.push("a quiet stretch of the day: let things stand".into());
        } else if in_window(at, abhijit_window(d)) {
            score = 1.0;
            notes.push("the day's lucky minutes: begin what matters".into());
        }
    }
    if GOOD_STARS.contains(&n.index) {
        score += 0.15;
        notes.push("the calendar marks today as lucky for beginnings".into());
    }
    if matches!(ti.number % 15, 4 | 9 | 14) || ti.number == 30 {
        score -= 0.2;
        notes.push("the calendar marks today as an empty day: avoid starting new things".into());
    }
    if k.name == "Vishti" {
        score -= 0.2;
        notes.push("a blocked half-day on the calendar: avoid starting new things".into());
    }
    if notes.is_empty() {
        notes.push("an ordinary day on the calendar".into());
    }
    Component {
        key: "kala",
        score: score.clamp(0.0, 1.0),
        weight: 0.10,
        note: notes.join(" · "),
        rule_id: "KS-M-0001",
    }
}

pub(crate) fn combine(components: &[Component]) -> f64 {
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
            "A strong time for the home: repairs, purchase, rent.",
        ],
        (Life::Home, Band::Good) => &[
            "Good for work on the home and paperwork about property.",
            "Home projects progress. Measure, plan, fix.",
        ],
        (Life::Home, Band::Steady) => &[
            "Home matters are stable. Maintain, do not expand.",
            "A quiet day at home.",
        ],
        (Life::Home, Band::Slow) => &[
            "Expect small delays with land, rent or repairs.",
            "Check documents twice before anything about property.",
        ],
        (Life::Home, Band::Hold) => &[
            "No property deals today. Wait for a better hour.",
            "Hold moves, purchases and agreements about the home.",
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
        s.push_str(" · ");
        s.push_str(crate::plain::hour_short(h.lord));
    }
    s.push_str(" · ");
    s.push_str(crate::plain::day_short(tara));
    s
}

/// The calendar side of the moment in plain words: a caution window, the winning
/// minutes, or the day's own quality.
pub(crate) fn kala_plain(sky: &Sky, day: Option<&SolarDay>, obs: &Observer, at: Instant) -> String {
    if let Some(d) = day {
        let wd = weekday(d, obs) as usize;
        if in_window(at, rahu_kala(d, obs)) {
            return "this is the day's unlucky window: finish what you started, begin nothing".into();
        }
        if in_window(at, slot_window(d, YAMAGANDA[wd])) || in_window(at, slot_window(d, GULIKA[wd])) {
            return "this is a stalled window of the day: let things stand for now".into();
        }
        if in_window(at, abhijit_window(d)) {
            return "these are the day's winning minutes: begin what matters".into();
        }
    }
    let n = nakshatra(sky.moon_sidereal);
    let ti = tithi(sky.moon_sidereal, sky.sun_sidereal);
    let k = karana(sky.moon_sidereal, sky.sun_sidereal);
    if matches!(ti.number % 15, 4 | 9 | 14) || ti.number == 30 || k.name == "Vishti" {
        "the calendar marks today as an empty day: avoid starting new things".into()
    } else if GOOD_STARS.contains(&n.index) {
        "the calendar marks today as a lucky day for beginnings".into()
    } else {
        "an ordinary day on the calendar".into()
    }
}

/// Why a life stands where it does, in plain words: the strongest pull among the
/// components, said as what it means for the person. Never names the astrology.
#[allow(clippy::too_many_arguments)]
fn plain_because(natal: &Natal, life: Life, comps: &[Component], levels: &[DashaPeriod], sky: &Sky, hora: Option<&Hora>, tara: usize, kala: &str) -> String {
    let name = life.name().to_ascii_lowercase();
    let Some(c) = comps.iter().max_by(|a, b| {
        let fa = (a.score - 0.5).abs() * a.weight;
        let fb = (b.score - 0.5).abs() * b.weight;
        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return String::new();
    };
    let good = c.score >= 0.5;
    match c.key {
        "natal" => {
            if good {
                format!("{name} comes naturally to you; your birth promise for it is strong")
            } else {
                format!("{name} has always asked more effort of you than of most")
            }
        }
        "dasha" => {
            let items = dasha_items(natal, life, levels);
            let pick = if good {
                items.iter().max_by(|a, b| (a.weight * (a.score - 0.5)).partial_cmp(&(b.weight * (b.score - 0.5))).unwrap_or(std::cmp::Ordering::Equal))
            } else {
                items.iter().min_by(|a, b| (a.weight * (a.score - 0.5)).partial_cmp(&(b.weight * (b.score - 0.5))).unwrap_or(std::cmp::Ordering::Equal))
            };
            match pick {
                Some(it) if good => format!("this stretch of your life brings {} to {name}", crate::plain::support(it.lord)),
                Some(it) => format!("this stretch of your life brings {} to {name}", crate::plain::pressure(it.lord)),
                None => String::new(),
            }
        }
        "gochara" => {
            let (items, _) = gochara_items(natal, &sky.from_moon, &sky.signs(), life);
            let pick = if good {
                items.iter().max_by(|a, b| a.contrib.partial_cmp(&b.contrib).unwrap_or(std::cmp::Ordering::Equal))
            } else {
                items.iter().min_by(|a, b| a.contrib.partial_cmp(&b.contrib).unwrap_or(std::cmp::Ordering::Equal))
            };
            match pick {
                Some(it) if it.saturn_phase != SaturnPhase::Clear => format!("{} is running", crate::plain::long_test(it.saturn_phase)),
                Some(it) if it.graha == DashaLord::Moon && it.house == 8 => "these two days are your heaviest of the month: moods dip and plans slip".to_string(),
                Some(it) if good => format!("the present season brings {}", crate::plain::support(it.graha)),
                Some(it) => format!("the present season brings {} to {name}", crate::plain::pressure(it.graha)),
                None => String::new(),
            }
        }
        "tara" => format!("today is {}", crate::plain::day_quality(tara)),
        "hora" => match hora {
            Some(h) if good => format!("this is {}", crate::plain::hour_nature(h.lord)),
            Some(_) => format!("this hour does not suit {name}; a better one comes later today"),
            None => String::new(),
        },
        _ => kala.to_string(),
    }
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
    let kala_words = kala_plain(&sky, day.as_ref(), obs, at);
    let seed = (at.tai_ns() / NS_PER_DAY) as u64;

    let lives: Vec<LifeScore> = Life::ALL
        .iter()
        .map(|&life| {
            let mut comps = vec![
                natal_component(natal, life),
                dasha_component(natal, life, &levels),
                gochara_component(natal, &sky, life),
                bhava_component(natal, &sky.signs(), life, true),
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
                because: plain_because(natal, life, &comps, &levels, &sky, hora.as_ref(), tara, &kala_words),
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
            label: "Unlucky stretch".into(),
            start: rs,
            end: re,
            caution: true,
        });
        let (ys, ye) = slot_window(d, YAMAGANDA[wd]);
        windows.push(Window {
            label: "Risky stretch".into(),
            start: ys,
            end: ye,
            caution: true,
        });
        let (gs, ge) = slot_window(d, GULIKA[wd]);
        windows.push(Window {
            label: "Quiet stretch".into(),
            start: gs,
            end: ge,
            caution: true,
        });
        let (as_, ae) = abhijit_window(d);
        windows.push(Window {
            label: "Lucky minutes".into(),
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
            let what = match hora_lord(next) {
                Some(l) => format!("{} begins", crate::plain::cap(crate::plain::hour_short(l))),
                None => "A new hour begins".to_string(),
            };
            shifts.push(Shift {
                at: e,
                what,
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
                what: format!("The day turns: {}", crate::plain::day_short(t)),
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
                    "The lunar day changes to {} ({})",
                    TITHI_NAMES[(n - 1) % 15],
                    if n <= 15 { "waxing" } else { "waning" }
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
                    what: match h {
                        8 => "Two heavy days begin: moods dip; keep plans small".to_string(),
                        1 | 3 | 6 | 7 | 10 | 11 => "Two lighter days begin: the mood lifts".to_string(),
                        _ => "The mood of the next two days changes".to_string(),
                    },
                    kind: "sign",
                });
            }
        }
    }
    for p in levels.iter().skip(2) {
        if p.end > at && p.end <= horizon {
            // the lord that follows at this level
            let idx = DashaLord::ORDER.iter().position(|l| *l == p.lord).unwrap_or(0);
            let next = DashaLord::ORDER[(idx + 1) % 9];
            shifts.push(Shift {
                at: p.end,
                what: format!("A new phase of a few days begins, about {}", crate::plain::chapter_theme(next)),
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
    let title = crate::hora::moment_title(
        tara,
        hora.as_ref().map(|h| h.lord),
        sky.facts.moon_phase_deg < 180.0,
        nakshatra(sky.moon_sidereal).name(),
    );
    let day_title = crate::feed::day_title(
        tara,
        &tithi(sky.moon_sidereal, sky.sun_sidereal),
        crate::prashna::day_lord(at, obs.longitude_deg),
    );
    Ok(Moment {
        hours,
        at,
        overall,
        band,
        headline: moment_words(band, hora.as_ref(), tara),
        title,
        day_title,
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
                    bhava_component(natal, &sky.signs(), life, true),
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

/// The slow score of every life at `at`: birth chart, running periods (three levels)
/// and the transits of the eight grahas other than the Moon, renormalised. It leaves
/// out everything that turns within a month, so a monthly sample of it reads as the
/// shape of a year rather than the noise of a day.
pub(crate) fn slow_scores(natal: &Natal, sky: &Sky, levels: &[DashaPeriod]) -> Vec<f64> {
    let signs = sky.signs();
    Life::ALL
        .iter()
        .map(|&life| {
            let n = natal_component(natal, life);
            let d = dasha_component(natal, life, levels);
            let (items, den) = gochara_items(natal, &sky.from_moon, &signs, life);
            let moon = &items[1];
            let den = den - moon.relevance;
            let num: f64 = items.iter().map(|i| i.contrib).sum::<f64>() - moon.contrib;
            let raw = if den > 0.0 { num / den } else { 0.0 };
            let g = (0.5 + 0.5 * raw).clamp(0.0, 1.0);
            let b = bhava_component(natal, &signs, life, false).score;
            ((0.15 * n.score + 0.30 * d.score + 0.20 * g + 0.15 * b) / 0.80).clamp(0.0, 1.0)
        })
        .collect()
}

/// `count` monthly slow samples (see [`slow_scores`]) from `from`: `(instant, nine scores)`.
#[allow(clippy::too_many_arguments)]
pub fn slow_series(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    from: Instant,
    step_ns: i128,
    count: usize,
) -> Result<Vec<(Instant, Vec<f64>)>, ForecastError> {
    let mut out = Vec::with_capacity(count);
    for k in 0..count {
        let at = from.add_ns(step_ns * k as i128);
        let sky = sky(spk, eop, natal, obs, HouseSystem::WholeSign, ayanamsa, at)?;
        let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
        out.push((at, slow_scores(natal, &sky, &levels)));
    }
    Ok(out)
}

impl Natal {
    /// Compute the birth chart and derive what the forecast needs from it.
    pub fn compute(
        spk: &Spk,
        eop: Option<&EopTable>,
        birth: Instant,
        obs: &Observer,
        system: HouseSystem,
        ayanamsa: Ayanamsa,
    ) -> Result<Natal, ForecastError> {
        let ch = compute(
            spk,
            eop,
            &ChartInput {
                instant: birth,
                observer: *obs,
                system,
                ayanamsa,
                topocentric_moon: false,
            },
        )
        .map_err(|e| ForecastError::Chart(format!("{e:?}")))?;
        Ok(Natal::new(Facts::from_chart(&ch), birth))
    }
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
                    what: format!(
                        "A new {} begins, about {}",
                        ["chapter of your life", "stretch of your life", "shorter phase of a few months", "phase of a few weeks", "phase of a few days"][lvl.min(4)],
                        crate::plain::chapter_theme(*b)
                    ),
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
                let name = life.name().to_ascii_lowercase();
                let what = if favourable_from_moon(g).contains(&h) {
                    format!("A helpful season for {name} begins: {}", crate::plain::support(g))
                } else if adverse_from_moon(g).contains(&h) {
                    format!("A harder season for {name} begins: {}", crate::plain::pressure(g))
                } else {
                    format!("The season for {name} turns neutral")
                };
                shifts.push(Shift {
                    at: q.at,
                    what,
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
            assert_eq!(l.components.len(), 7, "hora present at Kochi");
            let w: f64 = l.components.iter().map(|c| c.weight).sum();
            assert!((w - 1.22).abs() < 1e-9, "weights {w}");
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
