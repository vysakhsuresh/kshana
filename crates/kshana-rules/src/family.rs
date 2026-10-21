//! Family proof: one person's event read in the charts of the people around them.
//!
//! A child's wedding shows in the parents' charts too; a father's change of work shows
//! in his children's. When the person's own chart and one or more family charts point
//! to the same weeks for the same event, the reading is much stronger than any one
//! chart alone. No astrologer has time to cross-check every chart in a family this
//! way; the engine does it for every pair it can place.
//!
//! # The principle: bhavat bhavam
//!
//! BPHS (Bhava-vivechana; the karaka chapters) gives each relative a house of the
//! native's chart: the 9th is the father, the 4th the mother, the 7th the spouse, the
//! 5th the children, the 3rd the younger and the 11th the elder siblings. The rule of
//! *bhavat bhavam* ("the house from the house", used throughout BPHS and spelt out in
//! Phaladeepika ch. 15) then reads any matter of that relative by counting from their
//! house as if it were their rising sign: the father's career is the 10th from the 9th,
//! the 6th of the native's chart; a child's marriage is the 7th from the 5th, the 11th.
//!
//! The engine does this literally: it turns the relative's chart so the house that
//! stands for the person becomes the first ([`rotate`]), and then scores the event with
//! the same slow forecast score the person's own chart uses (birth promise, running
//! periods, the slow transits and the timing of the houses; see
//! [`crate::forecast::slow_series`]). The relative's own periods drive the timing: it
//! is their chart that is read, through the person's house.
//!
//! | the person is, to the reader | house of the person | marriage (7) | child (5) | career (10) | home (4) | gains (11) | abroad (12) | health (1) |
//! |------------------------------|--------------------:|------:|------:|------:|------:|------:|------:|------:|
//! | the reader's child           | 5  | 11 | 9  | 2  | 8  | 3  | 4  | 5  |
//! | the reader's father          | 9  | 3  | 1  | 6  | 12 | 7  | 8  | 9  |
//! | the reader's mother          | 4  | 10 | 8  | 1  | 7  | 2  | 3  | 4  |
//! | the reader's spouse          | 7  | 1  | 11 | 4  | 10 | 5  | 6  | 7  |
//! | the reader's younger sibling | 3  | 9  | 7  | 12 | 6  | 1  | 2  | 3  |
//! | the reader's elder sibling   | 11 | 5  | 3  | 8  | 2  | 9  | 10 | 11 |
//!
//! (A parent of unknown gender is read from the 9th, the house of parents and elders.)
//!
//! # Pooling
//!
//! The horizon is sampled every half month. Each chart's reading of the event is a slow
//! score in `[0, 1]`; it *agrees* with an opening when it stands above both its own
//! level over the horizon and the absolute good line, the same bands the Future view
//! uses. The pooled score adds, on the log-odds scale, each confirming chart's lift
//! over its own level, at weight [`RELATIVE_WEIGHT`] (a derived reading is weaker than
//! a chart's own), to the person's own score ([`pool`]). A window is reported only
//! where the person's own chart and at least one family chart agree for at least a
//! month; its edges are then found to the day.
//!
//! Honesty: the words say "likely", "very likely", "three charts agree"; never certain.
//! A hard window is said as care ("a time to take extra care of his health") with what
//! helps, and death is never read.

use crate::facts::Facts;
use crate::forecast::{date_words, sky, slow_scores, ForecastError, Life, Natal, Sky, GRAHAS};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa};

/// Weight of a family chart's lift against the person's own reading, on log-odds.
pub const RELATIVE_WEIGHT: f64 = 0.6;

/// The step of the horizon: half a month.
pub const STEP_NS: i128 = (15.21875 * NS_PER_DAY as f64) as i128;

/// How a relative stands to the main person, as the app stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Kin {
    Father,
    Mother,
    /// Spouse or partner.
    Spouse,
    Child,
    Sibling,
}

impl Kin {
    /// A stored relation word and the gender it implies: "Father", "mother", "Spouse",
    /// "Partner", "husband", "wife", "Child", "son", "daughter", "Sibling", "brother",
    /// "sister". Friends and others are not family charts here.
    pub fn parse(s: &str) -> Option<(Kin, Option<bool>)> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "father" | "dad" => (Kin::Father, Some(true)),
            "mother" | "mom" | "mum" => (Kin::Mother, Some(false)),
            "spouse" | "partner" => (Kin::Spouse, None),
            "husband" => (Kin::Spouse, Some(true)),
            "wife" => (Kin::Spouse, Some(false)),
            "child" => (Kin::Child, None),
            "son" => (Kin::Child, Some(true)),
            "daughter" => (Kin::Child, Some(false)),
            "sibling" => (Kin::Sibling, None),
            "brother" => (Kin::Sibling, Some(true)),
            "sister" => (Kin::Sibling, Some(false)),
            _ => return None,
        })
    }

    /// "your father", "your wife", "your child"…
    pub fn words(self, male: Option<bool>) -> &'static str {
        match (self, male) {
            (Kin::Father, _) => "your father",
            (Kin::Mother, _) => "your mother",
            (Kin::Spouse, Some(true)) => "your husband",
            (Kin::Spouse, Some(false)) => "your wife",
            (Kin::Spouse, None) => "your partner",
            (Kin::Child, Some(true)) => "your son",
            (Kin::Child, Some(false)) => "your daughter",
            (Kin::Child, None) => "your child",
            (Kin::Sibling, Some(true)) => "your brother",
            (Kin::Sibling, Some(false)) => "your sister",
            (Kin::Sibling, None) => "your sibling",
        }
    }
}

/// What the person whose event is read is, to the chart that reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Role {
    /// The person's own chart.
    Myself,
    Child,
    Father,
    Mother,
    /// A parent of unknown gender.
    Parent,
    Spouse,
    YoungerSibling,
    ElderSibling,
}

impl Role {
    /// The house of the reader's chart that stands for the person (BPHS).
    pub fn house(self) -> u8 {
        match self {
            Role::Myself => 1,
            Role::Child => 5,
            Role::Father | Role::Parent => 9,
            Role::Mother => 4,
            Role::Spouse => 7,
            Role::YoungerSibling => 3,
            Role::ElderSibling => 11,
        }
    }

    /// The role as an astrologer says it (expert view only).
    pub fn expert(self) -> &'static str {
        match self {
            Role::Myself => "own chart",
            Role::Child => "5th, children",
            Role::Father => "9th, father",
            Role::Mother => "4th, mother",
            Role::Parent => "9th, parents",
            Role::Spouse => "7th, spouse",
            Role::YoungerSibling => "3rd, younger siblings",
            Role::ElderSibling => "11th, elder siblings",
        }
    }
}

/// Bhavat bhavam: the house `h` counted from house `base` (both 1–12), as a house of
/// the reader's chart. `derived_house(5, 7) = 11`: a child's marriage in a parent's chart.
pub fn derived_house(base: u8, h: u8) -> u8 {
    ((base as u32 - 1 + h as u32 - 1) % 12 + 1) as u8
}

/// The chart turned so its house `k` becomes the first: every house number, cusp,
/// house lord and the rising sign move by `k − 1`; the planets stay where they are. An
/// area read on the turned chart is the area counted from house `k` (bhavat bhavam).
pub fn rotate(facts: &Facts, k: u8) -> Facts {
    let s = (k.clamp(1, 12) - 1) as usize;
    if s == 0 {
        return facts.clone();
    }
    let turn = |h: u8| ((h as i32 - 1 - s as i32).rem_euclid(12) + 1) as u8;
    let mut f = facts.clone();
    f.cusps = std::array::from_fn(|i| facts.cusps[(i + s) % 12]);
    f.cusp_sub_lords = std::array::from_fn(|i| facts.cusp_sub_lords[(i + s) % 12]);
    f.cusp_star_lords = std::array::from_fn(|i| facts.cusp_star_lords[(i + s) % 12]);
    f.house_lords = std::array::from_fn(|i| facts.house_lords[(i + s) % 12]);
    f.rasi_lords = std::array::from_fn(|i| facts.rasi_lords[(i + s) % 12]);
    f.lagna_sign = ((facts.lagna_sign as usize + s) % 12) as u8;
    for p in &mut f.planets {
        p.house = turn(p.house);
        p.rasi_house = turn(p.rasi_house);
        p.owns = p.owns.iter().map(|&h| turn(h)).collect();
        p.owns.sort_unstable();
    }
    f
}

/// The events family charts can confirm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Event {
    Marriage,
    Child,
    Career,
    Home,
    Gains,
    Abroad,
    Health,
}

impl Event {
    /// All of them, in display order.
    pub const ALL: [Event; 7] = [Event::Marriage, Event::Child, Event::Career, Event::Home, Event::Gains, Event::Abroad, Event::Health];

    /// The forecast life behind it.
    pub fn life(self) -> Life {
        match self {
            Event::Marriage => Life::Love,
            Event::Child => Life::Children,
            Event::Career => Life::Career,
            Event::Home => Life::Home,
            Event::Gains => Life::Money,
            Event::Abroad => Life::Travel,
            Event::Health => Life::Health,
        }
    }

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Event::Marriage => "marriage",
            Event::Child => "child",
            Event::Career => "career",
            Event::Home => "home",
            Event::Gains => "gains",
            Event::Abroad => "abroad",
            Event::Health => "health",
        }
    }

    /// The event's main house counted from the person's own rising sign.
    pub fn house(self) -> u8 {
        self.life().area().primary
    }

    /// Whether the event is read for a good window (an opening) or a hard one (a time
    /// of care) at this age, in years. Marriage and children only in the years they
    /// happen; work and money from adulthood; health care at any age.
    fn applies(self, good: bool, age: f64) -> bool {
        match (self, good) {
            (Event::Marriage, true) => (20.0..=60.0).contains(&age),
            (Event::Child, true) => (21.0..=46.0).contains(&age),
            (Event::Career, _) => (18.0..=72.0).contains(&age),
            (Event::Home, true) => age >= 21.0,
            (Event::Gains, _) => age >= 18.0,
            (Event::Abroad, true) => age >= 16.0,
            (Event::Health, false) => age >= 0.0,
            _ => false,
        }
    }
}

/// One chart of the family. Index 0 of a family is the main person, whose `kin` is None.
#[derive(Clone, Debug)]
pub struct Member {
    /// The birth chart.
    pub natal: Natal,
    /// How they stand to the main person; None for the main person.
    pub kin: Option<Kin>,
    /// Male, female, or not known.
    pub male: Option<bool>,
}

impl Member {
    /// "you", "your father"…
    pub fn words(&self) -> &'static str {
        self.kin.map_or("you", |k| k.words(self.male))
    }

    fn possessive(&self) -> &'static str {
        match (self.kin, self.male) {
            (None, _) => "your",
            (_, Some(true)) => "his",
            (_, Some(false)) => "her",
            (_, None) => "their",
        }
    }

    fn chart_words(&self) -> String {
        match self.kin {
            None => "your own chart".into(),
            Some(k) => format!("{}'s chart", k.words(self.male)),
        }
    }
}

/// What `members[x]` is to `members[r]`, when the family tree places them: parents to
/// each other, parents and siblings, the main person's spouse and children, children
/// among themselves. In-laws and grandparents are not read.
pub fn role_of(members: &[Member], x: usize, r: usize) -> Option<Role> {
    if x == r {
        return Some(Role::Myself);
    }
    let (mx, mr) = (members.get(x)?, members.get(r)?);
    let sib = || if mx.natal.birth < mr.natal.birth { Role::ElderSibling } else { Role::YoungerSibling };
    let parent = |male: Option<bool>| match male {
        Some(true) => Role::Father,
        Some(false) => Role::Mother,
        None => Role::Parent,
    };
    use Kin::*;
    Some(match (mx.kin, mr.kin) {
        (Some(k), None) => match k {
            Father => Role::Father,
            Mother => Role::Mother,
            Spouse => Role::Spouse,
            Child => Role::Child,
            Sibling => sib(),
        },
        (None, Some(k)) => match k {
            Father | Mother => Role::Child,
            Spouse => Role::Spouse,
            Child => parent(mx.male),
            Sibling => sib(),
        },
        (Some(a), Some(b)) => match (a, b) {
            (Father, Mother) | (Mother, Father) => Role::Spouse,
            (Father, Sibling) => Role::Father,
            (Mother, Sibling) => Role::Mother,
            (Sibling, Father) | (Sibling, Mother) => Role::Child,
            (Sibling, Sibling) | (Child, Child) => sib(),
            (Spouse, Child) => parent(mx.male.or(members[0].male.map(|m| !m))),
            (Child, Spouse) => Role::Child,
            _ => return None,
        },
        (None, None) => return None,
    })
}

/// The pooled score: the person's own score on log-odds, plus each confirming chart's
/// lift over its own level (`(score, level)`) at [`RELATIVE_WEIGHT`].
pub fn pool(own: f64, others: &[(f64, f64)]) -> f64 {
    let logit = |p: f64| {
        let p = p.clamp(1e-4, 1.0 - 1e-4);
        (p / (1.0 - p)).ln()
    };
    let l = logit(own) + others.iter().map(|&(s, m)| RELATIVE_WEIGHT * (logit(s) - logit(m))).sum::<f64>();
    1.0 / (1.0 + (-l).exp())
}

/// The good and hard lines of a series: above its own level and the absolute good band
/// for an opening, below both for a time of care (as the Future view draws them).
pub fn bounds(series: &[f64]) -> (f64, f64, f64) {
    let mean = series.iter().sum::<f64>() / series.len().max(1) as f64;
    ((mean + 0.05).max(0.53), (mean - 0.05).min(0.44), mean)
}

/// One chart's reading of the event over the horizon.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// Index of the chart in the family.
    pub chart: usize,
    /// The house of that chart read as the person's first.
    pub house: u8,
    /// The slow score at each step.
    pub scores: Vec<f64>,
}

/// A run of steps where the person's own reading and at least one family reading agree.
#[derive(Clone, Debug, PartialEq)]
pub struct Agreement {
    /// First step inside.
    pub from: usize,
    /// One past the last step inside.
    pub to: usize,
    /// Charts (family indices) that agree for at least half the run.
    pub confirmers: Vec<usize>,
    /// Pooled score at the peak step.
    pub pooled: f64,
    /// The person's own score at the peak step.
    pub own: f64,
    /// The peak step.
    pub peak: usize,
}

/// The runs, at least `min_len` steps long, where `own` is inside its line (above for
/// `good`, below otherwise) and at least one of `others` is inside its own.
pub fn agree_windows(own: &[f64], others: &[Track], good: bool, min_len: usize) -> Vec<Agreement> {
    let n = own.len();
    let (ohi, olo, _) = bounds(own);
    let lines: Vec<(f64, f64, f64)> = others.iter().map(|t| bounds(&t.scores)).collect();
    let inside = |v: f64, (hi, lo, _): (f64, f64, f64)| if good { v >= hi } else { v <= lo };
    let agreeing = |k: usize| -> Vec<usize> {
        (0..others.len()).filter(|&j| others[j].scores.get(k).is_some_and(|&v| inside(v, lines[j]))).collect()
    };
    let on = |k: usize| inside(own[k], (ohi, olo, 0.0)) && !agreeing(k).is_empty();
    let mut out = Vec::new();
    let mut k = 0;
    while k < n {
        if !on(k) {
            k += 1;
            continue;
        }
        let from = k;
        while k < n && on(k) {
            k += 1;
        }
        if k - from < min_len {
            continue;
        }
        let mut count = vec![0usize; others.len()];
        for s in from..k {
            for j in agreeing(s) {
                count[j] += 1;
            }
        }
        let half = (k - from).div_ceil(2);
        let confirm: Vec<usize> = (0..others.len()).filter(|&j| count[j] >= half).collect();
        let confirm = if confirm.is_empty() { vec![(0..others.len()).max_by_key(|&j| count[j]).unwrap_or(0)] } else { confirm };
        let pooled_at = |s: usize| {
            let o: Vec<(f64, f64)> = confirm.iter().map(|&j| (others[j].scores[s], lines[j].2)).collect();
            pool(own[s], &o)
        };
        let peak = (from..k)
            .max_by(|&a, &b| {
                let (pa, pb) = (pooled_at(a), pooled_at(b));
                let c = if good { pa.total_cmp(&pb) } else { pb.total_cmp(&pa) };
                c.then(b.cmp(&a))
            })
            .unwrap_or(from);
        out.push(Agreement {
            from,
            to: k,
            confirmers: confirm.iter().map(|&j| others[j].chart).collect(),
            pooled: pooled_at(peak),
            own: own[peak],
            peak,
        });
    }
    out
}

/// One window of an event in a person's life, confirmed by family charts.
#[derive(Clone, Debug, PartialEq)]
pub struct ProofWindow {
    /// Family index of the person whose event it is (0 = the main person).
    pub person: usize,
    /// The event.
    pub event: Event,
    /// An opening (true) or a time of care (false).
    pub good: bool,
    /// Start, to the day.
    pub start: Instant,
    /// End, to the day.
    pub end: Instant,
    /// The step where the pooled reading is strongest.
    pub peak: Instant,
    /// Family indices of the charts that agree with the person's own.
    pub confirmed_by: Vec<usize>,
    /// The person's own score at the peak, `[0, 1]`.
    pub own: f64,
    /// The pooled score at the peak, `[0, 1]`.
    pub pooled: f64,
    /// "For your father: a likely time for a change or a step up at work."
    pub headline: String,
    /// Dates and which charts agree, in plain words.
    pub detail: String,
    /// "Your own chart and your mother's chart agree."
    pub confirmed_line: String,
    /// "Three charts agree: very likely."
    pub strength_words: String,
    /// What helps, in plain words.
    pub remedy_hint: String,
    /// Each confirming chart as an astrologer reads it (expert view only), e.g.
    /// "your father's chart: 9th, father; career from the 6th (10th from the 9th)".
    pub expert: Vec<String>,
}

/// Everything the family's charts confirm in the horizon.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FamilyProof {
    /// The main person's events, each confirmed by at least one family chart.
    pub about_you: Vec<ProofWindow>,
    /// Each relative's events that the main person's own chart confirms.
    pub about_family: Vec<ProofWindow>,
    /// Family charts that could be placed in the tree and read.
    pub charts_read: usize,
}

fn count_word(n: usize) -> &'static str {
    ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"].get(n).copied().unwrap_or("many")
}

fn phrase(event: Event, good: bool, pos: &str) -> String {
    match (event, good) {
        (Event::Marriage, _) => "a likely time for a wedding or a big step in a relationship".into(),
        (Event::Child, _) => "a likely time for a child to arrive".into(),
        (Event::Career, true) => "a likely time for a change or a step up at work".into(),
        (Event::Career, false) => "a slower time at work that asks for patience".into(),
        (Event::Home, _) => "a likely time to move home or settle a property".into(),
        (Event::Gains, true) => "a likely time for money to come in".into(),
        (Event::Gains, false) => "a time to be careful with money".into(),
        (Event::Abroad, _) => "a likely time for travel abroad".into(),
        (Event::Health, _) => format!("a time to take extra care of {pos} health"),
    }
}

fn helps(event: Event, good: bool) -> &'static str {
    match (event, good) {
        (Event::Marriage, _) => "Talk openly as a family, and set the important day with care.",
        (Event::Child, _) => "Rest, eat well and keep the home calm; teaching a child something you know is an old way of blessing this time.",
        (Event::Career, true) => "Ask, apply and show your work; finish one postponed duty so you begin the new one clear.",
        (Event::Career, false) => "Finish one postponed duty each week, keep your word at work, and help someone who works with their hands.",
        (Event::Home, _) => "Check the papers twice, and feed someone in the new place when you move.",
        (Event::Gains, true) => "Share a small part of what comes in; it keeps the flow open.",
        (Event::Gains, false) => "Avoid lending and big purchases on impulse, repay one small debt, and share a little of what you have.",
        (Event::Abroad, _) => "Keep the documents ready early and call home often while away.",
        (Event::Health, _) => "Keep regular sleep and meals, see a doctor early for anything that lingers, and serve someone who is unwell; it eases the time for the whole family.",
    }
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn join_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// The sky for another natal from one already computed: the same positions, the houses
/// from that natal Moon.
fn resky(base: &Sky, natal: &Natal) -> Sky {
    let ms = natal.moon_sign();
    let facts = base.facts.clone();
    let from_moon: [u8; 9] = std::array::from_fn(|i| ((facts.planet(GRAHAS[i]).sign as i64 - ms).rem_euclid(12) + 1) as u8);
    Sky { from_moon, moon_sidereal: base.moon_sidereal, sun_sidereal: base.sun_sidereal, ayanamsa_deg: base.ayanamsa_deg, facts }
}

/// The readers of the family: per chart and per house read as first, the turned natal.
struct Readers {
    turned: Vec<Vec<(u8, Natal)>>,
}

impl Readers {
    fn new(members: &[Member]) -> Self {
        let houses = [1u8, 3, 4, 5, 7, 9, 11];
        Readers {
            turned: members.iter().map(|m| houses.iter().map(|&k| (k, Natal::new(rotate(&m.natal.facts, k), m.natal.birth))).collect()).collect(),
        }
    }

    fn natal(&self, chart: usize, house: u8) -> &Natal {
        &self.turned[chart].iter().find(|(k, _)| *k == house).expect("a house the family reads").1
    }
}

fn life_index(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

/// Slow score of `life` for the natal at the sky.
fn score(natal: &Natal, base: &Sky, at: Instant, life: Life) -> f64 {
    let sky = resky(base, natal);
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    slow_scores(natal, &sky, &levels)[life_index(life)]
}

/// Whether the person is already married by what the family says: the main person
/// with a spouse in the family, the spouse, and the main person's parents.
fn married(members: &[Member], x: usize) -> bool {
    match members[x].kin {
        None => members.iter().any(|m| m.kin == Some(Kin::Spouse)),
        Some(Kin::Spouse) | Some(Kin::Father) | Some(Kin::Mother) => true,
        _ => false,
    }
}

/// Family proof over `months` from `from`: every event of every member that their own
/// chart and at least one family chart agree on. `members[0]` is the main person.
#[allow(clippy::too_many_arguments)]
pub fn family_proof(
    spk: &Spk,
    eop: Option<&EopTable>,
    members: &[Member],
    obs: &Observer,
    ayanamsa: Ayanamsa,
    from: Instant,
    months: usize,
) -> Result<FamilyProof, ForecastError> {
    let mut proof = FamilyProof::default();
    if members.len() < 2 {
        return Ok(proof);
    }
    let lon = obs.longitude_deg;
    let months = months.clamp(3, 36);
    let n = months * 2 + 1;
    let times: Vec<Instant> = (0..n).map(|k| from.add_ns(STEP_NS * k as i128)).collect();
    let horizon_end = from.add_ns(STEP_NS * (n as i128 - 1));
    let sky_at = |t: Instant| sky(spk, eop, &members[0].natal, obs, HouseSystem::WholeSign, ayanamsa, t);
    let skies: Vec<Sky> = times.iter().map(|&t| sky_at(t)).collect::<Result<_, _>>()?;
    let readers = Readers::new(members);

    // Every (chart, house) series: nine lives at each step.
    let mut series: std::collections::HashMap<(usize, u8), Vec<Vec<f64>>> = std::collections::HashMap::new();
    let mut series_of = |chart: usize, house: u8| -> Vec<Vec<f64>> {
        series
            .entry((chart, house))
            .or_insert_with(|| {
                let natal = readers.natal(chart, house);
                skies
                    .iter()
                    .zip(&times)
                    .map(|(s, &t)| {
                        let sky = resky(s, natal);
                        let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, t, 3);
                        slow_scores(natal, &sky, &levels)
                    })
                    .collect()
            })
            .clone()
    };

    let placed: Vec<bool> = (0..members.len()).map(|r| r == 0 || (0..members.len()).any(|x| x != r && role_of(members, x, r).is_some())).collect();
    proof.charts_read = placed.iter().skip(1).filter(|p| **p).count();

    for x in 0..members.len() {
        let readers_of_x: Vec<(usize, Role)> = (0..members.len()).filter(|&r| r != x).filter_map(|r| role_of(members, x, r).map(|role| (r, role))).collect();
        if readers_of_x.is_empty() {
            continue;
        }
        let own_all = series_of(x, 1);
        let reads: Vec<(usize, Role, Vec<Vec<f64>>)> = readers_of_x.iter().map(|&(r, role)| (r, role, series_of(r, role.house()))).collect();
        let birth = members[x].natal.birth;
        let age_at = |t: Instant| t.ns_since(birth) as f64 / YEAR_365_25_NS as f64;
        let mut found: Vec<ProofWindow> = Vec::new();
        for event in Event::ALL {
            let li = life_index(event.life());
            let own: Vec<f64> = own_all.iter().map(|v| v[li]).collect();
            let tracks: Vec<Track> = reads.iter().map(|(r, role, s)| Track { chart: *r, house: role.house(), scores: s.iter().map(|v| v[li]).collect() }).collect();
            for good in [true, false] {
                if event == Event::Marriage && married(members, x) {
                    continue;
                }
                for a in agree_windows(&own, &tracks, good, 2) {
                    if !event.applies(good, age_at(times[a.from])) {
                        continue;
                    }
                    if x != 0 && !a.confirmers.contains(&0) {
                        continue;
                    }
                    // The edges to the day: where the own reading and the confirming
                    // charts still agree, between the outside step and the inside one.
                    let (ohi, olo, _) = bounds(&own);
                    let conf_tracks: Vec<&Track> = tracks.iter().filter(|t| a.confirmers.contains(&t.chart)).collect();
                    let lines: Vec<(f64, f64, f64)> = conf_tracks.iter().map(|t| bounds(&t.scores)).collect();
                    let inside = |v: f64, hi: f64, lo: f64| if good { v >= hi } else { v <= lo };
                    let agrees_at = |t: Instant| -> Result<bool, ForecastError> {
                        let base = sky_at(t)?;
                        if !inside(score(readers.natal(x, 1), &base, t, event.life()), ohi, olo) {
                            return Ok(false);
                        }
                        for (tr, l) in conf_tracks.iter().zip(&lines) {
                            if inside(score(readers.natal(tr.chart, tr.house), &base, t, event.life()), l.0, l.1) {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    };
                    let edge = |out: Instant, inn: Instant| -> Result<Instant, ForecastError> {
                        let (mut o, mut i) = (out, inn);
                        while i.ns_since(o).abs() > NS_PER_DAY {
                            let mid = o.add_ns(i.ns_since(o) / 2);
                            if agrees_at(mid)? {
                                i = mid;
                            } else {
                                o = mid;
                            }
                        }
                        Ok(i)
                    };
                    let start = if a.from == 0 { times[0] } else { edge(times[a.from - 1], times[a.from])? };
                    let end = if a.to >= n { horizon_end } else { edge(times[a.to], times[a.to - 1])? };
                    found.push(words(members, x, event, good, start, end, times[a.peak], &a, lon));
                }
            }
        }
        // At most two windows per event, the strongest.
        found.sort_by(|a, b| (a.event as u8, good_rank(a)).cmp(&(b.event as u8, good_rank(b))).then(strength(b).total_cmp(&strength(a))));
        let mut kept: Vec<ProofWindow> = Vec::new();
        for w in found {
            if kept.iter().filter(|k| k.event == w.event && k.good == w.good).count() < 2 {
                kept.push(w);
            }
        }
        if x == 0 {
            proof.about_you.extend(kept);
        } else {
            proof.about_family.extend(kept);
        }
    }
    for list in [&mut proof.about_you, &mut proof.about_family] {
        list.sort_by(|a, b| a.start.cmp(&b.start).then(a.person.cmp(&b.person)).then((a.event as u8).cmp(&(b.event as u8))));
    }
    Ok(proof)
}

fn good_rank(w: &ProofWindow) -> u8 {
    u8::from(!w.good)
}

fn strength(w: &ProofWindow) -> f64 {
    let p = if w.good { w.pooled } else { 1.0 - w.pooled };
    p + 0.1 * w.confirmed_by.len() as f64
}

#[allow(clippy::too_many_arguments)]
fn words(members: &[Member], x: usize, event: Event, good: bool, start: Instant, end: Instant, peak: Instant, a: &Agreement, lon: f64) -> ProofWindow {
    let m = &members[x];
    let who = m.words();
    let headline = format!("For {who}: {}.", phrase(event, good, m.possessive()));
    let charts: Vec<String> = a.confirmers.iter().map(|&r| members[r].chart_words()).collect();
    let total = a.confirmers.len() + 1;
    let own_chart = m.chart_words();
    let confirmed_line = if x == 0 {
        format!("Your own chart shows it, and {} {} the same weeks.", join_and(&charts), if charts.len() == 1 { "points to" } else { "point to" })
    } else {
        format!("{} shows it, and {} {} the same weeks.", crate::plain::cap(&own_chart), join_and(&charts), if charts.len() == 1 { "points to" } else { "point to" })
    };
    let p = if good { a.pooled } else { 1.0 - a.pooled };
    let strength_words = match (good, total >= 3 || p >= 0.70) {
        (true, true) => format!("{} charts agree: very likely.", crate::plain::cap(count_word(total))),
        (true, false) => format!("{} charts agree: likely.", crate::plain::cap(count_word(total))),
        (false, true) => format!("{} charts agree: worth preparing for.", crate::plain::cap(count_word(total))),
        (false, false) => format!("{} charts agree: worth a little care.", crate::plain::cap(count_word(total))),
    };
    let dates = format!("From {} to {}", date_words(start, lon), date_words(end, lon));
    let peak_words = if good { format!("strongest around {}", date_words(peak, lon)) } else { format!("asking most care around {}", date_words(peak, lon)) };
    let detail = if good {
        format!("{dates}, {peak_words}. {confirmed_line}")
    } else {
        format!("{dates}, {peak_words}. {confirmed_line} Nothing here is fixed: with early care such a time passes lightly, and the family can carry it together.")
    };
    let expert = a
        .confirmers
        .iter()
        .map(|&r| {
            let role = role_of(members, x, r).unwrap_or(Role::Myself);
            let k = role.house();
            let h = event.house();
            format!(
                "{}: {} as first; {} from the {} ({} from the {})",
                members[r].chart_words(),
                role.expert(),
                event.key(),
                ordinal(derived_house(k, h)),
                ordinal(h),
                ordinal(k)
            )
        })
        .collect();
    ProofWindow {
        person: x,
        event,
        good,
        start,
        end,
        peak,
        confirmed_by: a.confirmers.clone(),
        own: a.own,
        pooled: a.pooled,
        headline,
        detail,
        confirmed_line,
        strength_words,
        remedy_hint: helps(event, good).to_string(),
        expert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    #[test]
    fn bhavat_bhavam_table() {
        // (the person to the reader, event) → the reader's house, as in the module table.
        let rows: [(Role, [u8; 7]); 6] = [
            (Role::Child, [11, 9, 2, 8, 3, 4, 5]),
            (Role::Father, [3, 1, 6, 12, 7, 8, 9]),
            (Role::Mother, [10, 8, 1, 7, 2, 3, 4]),
            (Role::Spouse, [1, 11, 4, 10, 5, 6, 7]),
            (Role::YoungerSibling, [9, 7, 12, 6, 1, 2, 3]),
            (Role::ElderSibling, [5, 3, 8, 2, 9, 10, 11]),
        ];
        for (role, want) in rows {
            for (e, w) in Event::ALL.iter().zip(want) {
                assert_eq!(derived_house(role.house(), e.house()), w, "{role:?} {e:?}");
            }
        }
        assert_eq!(derived_house(5, 7), 11);
        assert_eq!(derived_house(9, 10), 6);
        assert_eq!(derived_house(1, 7), 7);
    }

    fn chart(y: i64, mo: u32, d: u32, h: u32, spk: &Spk) -> Natal {
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(y, mo, d, h, 0, 0, 0).unwrap().to_instant().unwrap();
        Natal::compute(spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap()
    }

    fn spk() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_turned_chart_reads_from_the_relatives_house() {
        let Some(spk) = spk() else { return };
        let f = chart(1990, 2, 20, 5, &spk).facts;
        for k in 1..=12u8 {
            let t = rotate(&f, k);
            for (a, b) in f.planets.iter().zip(&t.planets) {
                assert_eq!(derived_house(k, b.house), a.house);
                assert_eq!(derived_house(k, b.rasi_house), a.rasi_house);
                assert_eq!(a.sidereal, b.sidereal);
            }
            for h in 1..=12u8 {
                let d = derived_house(k, h) as usize - 1;
                assert_eq!(t.cusp_sub_lords[h as usize - 1], f.cusp_sub_lords[d]);
                assert_eq!(t.house_lords[h as usize - 1], f.house_lords[d]);
            }
            assert_eq!(t.lagna_sign as usize, (f.lagna_sign as usize + k as usize - 1) % 12);
        }
        assert_eq!(rotate(&f, 1), f);
    }

    #[test]
    fn agreement_raises_the_pooled_score() {
        let own = 0.6;
        assert!((pool(own, &[]) - own).abs() < 1e-9);
        let one = pool(own, &[(0.65, 0.5)]);
        let two = pool(own, &[(0.65, 0.5), (0.62, 0.48)]);
        assert!(one > own && two > one, "{own} {one} {two}");
        // A chart sitting at its own level adds nothing; one below takes away.
        assert!((pool(own, &[(0.5, 0.5)]) - own).abs() < 1e-9);
        assert!(pool(own, &[(0.4, 0.5)]) < own);
    }

    #[test]
    fn three_synthetic_charts_overlap_where_expected() {
        // 25 steps. The own chart is high on steps 4–11; relative A on 6–14; relative B
        // on 9–12 and again 18–21, where the own chart is low. The agreement is 6–11,
        // confirmed by A throughout and by B on 9–11 (half the run): both confirm.
        let base = |hi: std::ops::Range<usize>| -> Vec<f64> { (0..25).map(|k| if hi.contains(&k) { 0.72 } else { 0.48 }).collect() };
        let own = base(4..12);
        let a = Track { chart: 1, house: 5, scores: base(6..15) };
        let b = Track { chart: 2, house: 9, scores: (0..25).map(|k| if (9..13).contains(&k) || (18..22).contains(&k) { 0.72 } else { 0.48 }).collect() };
        let w = agree_windows(&own, &[a.clone(), b.clone()], true, 2);
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!((w[0].from, w[0].to), (6, 12));
        assert_eq!(w[0].confirmers, vec![1, 2]);
        assert!(w[0].pooled > w[0].own);
        // With A alone the run is the same and A alone confirms, more weakly.
        let wa = agree_windows(&own, &[a], true, 2);
        assert_eq!((wa[0].from, wa[0].to), (6, 12));
        assert_eq!(wa[0].confirmers, vec![1]);
        assert!(w[0].pooled > wa[0].pooled);
        // The own chart alone confirms nothing; B's second high stretch is not the own's.
        assert!(agree_windows(&own, &[], true, 2).is_empty());
        assert!(agree_windows(&own, &[b], true, 2).iter().all(|x| x.to <= 12));
    }

    #[test]
    fn the_family_tree_places_each_pair() {
        let Some(spk) = spk() else { return };
        let me = Member { natal: chart(1990, 2, 20, 5, &spk), kin: None, male: Some(true) };
        let father = Member { natal: chart(1960, 6, 1, 3, &spk), kin: Some(Kin::Father), male: Some(true) };
        let mother = Member { natal: chart(1964, 9, 9, 8, &spk), kin: Some(Kin::Mother), male: Some(false) };
        let sister = Member { natal: chart(1993, 1, 5, 10, &spk), kin: Some(Kin::Sibling), male: Some(false) };
        let wife = Member { natal: chart(1992, 4, 4, 4, &spk), kin: Some(Kin::Spouse), male: Some(false) };
        let son = Member { natal: chart(2020, 7, 7, 7, &spk), kin: Some(Kin::Child), male: Some(true) };
        let f = [me, father, mother, sister, wife, son];
        assert_eq!(role_of(&f, 0, 1), Some(Role::Child)); // I am my father's child: his 5th
        assert_eq!(role_of(&f, 1, 0), Some(Role::Father)); // my father in my chart: my 9th
        assert_eq!(role_of(&f, 2, 0), Some(Role::Mother));
        assert_eq!(role_of(&f, 1, 2), Some(Role::Spouse));
        assert_eq!(role_of(&f, 3, 0), Some(Role::YoungerSibling));
        assert_eq!(role_of(&f, 0, 3), Some(Role::ElderSibling));
        assert_eq!(role_of(&f, 3, 1), Some(Role::Child));
        assert_eq!(role_of(&f, 0, 5), Some(Role::Father));
        assert_eq!(role_of(&f, 4, 5), Some(Role::Mother));
        assert_eq!(role_of(&f, 5, 4), Some(Role::Child));
        assert_eq!(role_of(&f, 1, 5), None); // grandparents are not read
        assert_eq!(role_of(&f, 4, 1), None); // nor in-laws
        assert!(married(&f, 0) && married(&f, 1) && !married(&f, 3));
    }

    #[test]
    fn a_real_family_is_confirmed_plainly_and_the_same_every_time() {
        let Some(spk) = spk() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let fam = vec![
            Member { natal: chart(1990, 2, 20, 5, &spk), kin: None, male: Some(false) },
            Member { natal: chart(1958, 11, 2, 1, &spk), kin: Some(Kin::Father), male: Some(true) },
            Member { natal: chart(1963, 3, 30, 12, &spk), kin: Some(Kin::Mother), male: Some(false) },
            Member { natal: chart(1995, 8, 15, 18, &spk), kin: Some(Kin::Sibling), male: Some(true) },
        ];
        let at = Utc::new(2026, 10, 21, 6, 0, 0, 0).unwrap().to_instant().unwrap();
        let p = family_proof(&spk, None, &fam, &obs, Ayanamsa::Lahiri, at, 18).unwrap();
        let q = family_proof(&spk, None, &fam, &obs, Ayanamsa::Lahiri, at, 18).unwrap();
        assert_eq!(p, q);
        assert_eq!(p.charts_read, 3);
        let all: Vec<&ProofWindow> = p.about_you.iter().chain(&p.about_family).collect();
        assert!(!all.is_empty());
        for w in &all {
            println!("{} | {} | {} | {}", w.headline, w.strength_words, w.detail, w.expert.join("; "));
            assert!(w.start < w.end && w.start <= w.peak && w.peak <= w.end, "{w:?}");
            assert!(!w.confirmed_by.is_empty() && !w.confirmed_by.contains(&w.person));
            for s in [&w.headline, &w.detail, &w.confirmed_line, &w.strength_words, &w.remedy_hint] {
                assert_eq!(crate::plain::jargon(s), None, "{s}");
                for bad in ["guarantee", "certain", "fate", "death", "die"] {
                    assert!(!s.to_lowercase().contains(bad), "{s}");
                }
            }
            if w.good {
                assert!(w.pooled >= w.own - 1e-9);
            } else {
                assert!(w.pooled <= w.own + 1e-9);
            }
        }
        assert!(p.about_family.iter().all(|w| w.confirmed_by.contains(&0) && w.person != 0));
        assert!(p.about_you.iter().all(|w| w.person == 0));
        // The parents are married to each other: no wedding windows for them.
        assert!(all.iter().all(|w| !(w.event == Event::Marriage && (w.person == 1 || w.person == 2))));
    }
}
