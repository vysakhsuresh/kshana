//! Things nobody told the app: checkable facts read from the birth chart alone, each
//! said as a yes-or-no question ("Your chart says you are the first-born child in your
//! family. Is that right?"), and the birth time sharpened by every answer, together with
//! the answered stretches of the past and the events the person logged.
//!
//! The facts and where they come from (whole-sign houses from the rising sign, as the
//! Parashari texts count; [`crate::facts::Facts`] carries them):
//!
//! - **Birth order and siblings.** BPHS, the chapter on the Sahaja bhava, takes the
//!   co-born from the 3rd house, younger ones in particular, and the 11th for the elder
//!   (Phaladeepika's house significations agree), with Mars as their significator. A weak,
//!   badly placed 11th lord and Saturn or a node in the 11th read as few elder siblings
//!   (first-born); benefics in or aspecting the 3rd and 11th and strong lords as many.
//! - **Mother and father.** The 4th house and the Moon for the mother, the 9th and the
//!   Sun for the father (BPHS, the Sukha and Dharma bhava chapters). The planet that
//!   colours the house and its significator most (occupying, aspecting, owning, joined
//!   with) gives the parent's nature, in the plain traits the texts give each planet
//!   (Brihat Jataka ch. 2). Nothing about a parent's health or death is ever said.
//! - **A mark on the body, and which side.** Brihat Jataka ch. 5 (Janmakala-lakshana):
//!   the rising decanate divides the body in three (head and face, neck to navel, below
//!   the navel), the houses from the rising sign map to its parts (1 the top, 2 and 12
//!   the next part, … 7 the last), the houses below the horizon (2 to 6) the right side
//!   and those above it (8 to 12) the left. A malefic there marks it (Mars a scar, Saturn
//!   and the nodes a mole or dark mark, the Sun a burn mark); a benefic a light mole.
//!   The private parts of the third decanate are never named.
//! - **The years of moves, marriage and job changes.** The Vimshottari periods to the
//!   third level, each scored by how fully its lords signify the event's houses in the
//!   KP sense (marriage 2, 7, 11 against 1, 6, 10; a job 2, 6, 10, 11; a move 3, 4, 12),
//!   weighted like `kshana_ledger::retrodict` (the area's natal promise times the lords'
//!   share); the clearest calendar year, said with a year either side.
//!
//! Every fact carries how strongly the chart says it (only clear ones are said) and
//! whether it changes across the birth-time window: facts that rest on the rising sign,
//! its decanate or the cusps are evidence for the birth minute. [`refine_birth_pooled`]
//! pools them with the answered stretches and the logged events, per candidate minute,
//! on the same log-odds scale as [`crate::reveal::refine_birth_detail`]. A wrong guess
//! still helps, because it narrows down the birth time.

use crate::facts::Facts;
use crate::forecast::{Life, Natal};
use crate::kp::{promise, significators};
use crate::parashari::{aspected_houses, dignity, is_benefic};
use crate::plain::Lang;
use crate::reveal::{gain, judge, par_map, prepare, prob, suggested_shift, Answer, BirthFit, BirthRefine, Verdict, SUGGEST_LOG10_BAYES};
use crate::witness::{ascendant, boundaries, NW, SLOPE, WITNESSES};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::{subdivide, vimshottari, YEAR_365_25_NS};
use kshana_vedic::{Ayanamsa, DashaLord};

const YEAR_NS: i128 = 365 * NS_PER_DAY + NS_PER_DAY / 4;

/// What a fact is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    /// First-born or not.
    BirthOrder,
    /// How many brothers and sisters.
    Siblings,
    /// The mother's nature.
    Mother,
    /// The father's nature.
    Father,
    /// A mark on the body.
    Mark,
    /// The year of a move.
    Move,
    /// The year a marriage or lasting bond began.
    Marriage,
    /// The year of a new job.
    Job,
}

impl Topic {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Topic::BirthOrder => "birth_order",
            Topic::Siblings => "siblings",
            Topic::Mother => "mother",
            Topic::Father => "father",
            Topic::Mark => "mark",
            Topic::Move => "move",
            Topic::Marriage => "marriage",
            Topic::Job => "job",
        }
    }
}

/// Which side of the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Houses 8 to 12, above the horizon.
    Left,
    /// Houses 2 to 6, below it.
    Right,
    /// Houses 1 and 7, on the midline.
    Middle,
}

/// A dated event a fact is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YearEvent {
    /// A change of home.
    Move,
    /// A marriage or lasting bond.
    Marriage,
    /// A new job or a change of job.
    Job,
}

/// What a fact claims, in a form every candidate birth time can be judged against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Claim {
    /// `true`: the first-born; `false`: has an elder brother or sister.
    FirstBorn(bool),
    /// `true`: three or more brothers and sisters; `false`: at most one.
    ManySiblings(bool),
    /// The planet colouring the mother's nature.
    Mother(DashaLord),
    /// The planet colouring the father's nature.
    Father(DashaLord),
    /// A mark: the rising decanate (0–2), the position down the body (0–6), the side.
    Mark {
        /// Decanate of the rising degree.
        decanate: u8,
        /// 0 = the house of the rising sign, 1 = houses 2 and 12, … 6 = house 7.
        part: u8,
        /// Left, right or the midline.
        side: Side,
    },
    /// The calendar year (local) of an event, give or take one.
    Year {
        /// Which event.
        event: YearEvent,
        /// The year.
        year: i32,
    },
}

const LORDS: [DashaLord; 9] = [
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

fn gi(g: DashaLord) -> usize {
    LORDS.iter().position(|l| *l == g).unwrap_or(0)
}

fn lord_key(g: DashaLord) -> &'static str {
    ["sun", "moon", "mars", "mercury", "jupiter", "venus", "saturn", "rahu", "ketu"][gi(g)]
}

impl Claim {
    /// Stable key, stored with the answer and parsed back by [`Claim::parse`].
    pub fn key(&self) -> String {
        match *self {
            Claim::FirstBorn(true) => "first_born".into(),
            Claim::FirstBorn(false) => "elder_sibling".into(),
            Claim::ManySiblings(true) => "many_siblings".into(),
            Claim::ManySiblings(false) => "few_siblings".into(),
            Claim::Mother(g) => format!("mother:{}", lord_key(g)),
            Claim::Father(g) => format!("father:{}", lord_key(g)),
            Claim::Mark { decanate, part, side } => format!(
                "mark:{decanate}:{part}:{}",
                match side {
                    Side::Left => "left",
                    Side::Right => "right",
                    Side::Middle => "middle",
                }
            ),
            Claim::Year { event, year } => format!(
                "year:{}:{year}",
                match event {
                    YearEvent::Move => "move",
                    YearEvent::Marriage => "marriage",
                    YearEvent::Job => "job",
                }
            ),
        }
    }

    /// A key back to the claim; None for anything unknown.
    pub fn parse(key: &str) -> Option<Claim> {
        let parts: Vec<&str> = key.trim().split(':').collect();
        let lord = |s: &str| LORDS.iter().copied().find(|g| lord_key(*g) == s);
        match parts.as_slice() {
            ["first_born"] => Some(Claim::FirstBorn(true)),
            ["elder_sibling"] => Some(Claim::FirstBorn(false)),
            ["many_siblings"] => Some(Claim::ManySiblings(true)),
            ["few_siblings"] => Some(Claim::ManySiblings(false)),
            ["mother", g] => lord(g).map(Claim::Mother),
            ["father", g] => lord(g).map(Claim::Father),
            ["mark", d, p, s] => {
                let decanate: u8 = d.parse().ok().filter(|d| *d < 3)?;
                let part: u8 = p.parse().ok().filter(|p| *p < 7)?;
                let side = match *s {
                    "left" => Side::Left,
                    "right" => Side::Right,
                    "middle" => Side::Middle,
                    _ => return None,
                };
                Some(Claim::Mark { decanate, part, side })
            }
            ["year", e, y] => {
                let event = match *e {
                    "move" => YearEvent::Move,
                    "marriage" => YearEvent::Marriage,
                    "job" => YearEvent::Job,
                    _ => return None,
                };
                Some(Claim::Year { event, year: y.parse().ok()? })
            }
            _ => None,
        }
    }

    /// What the fact is about.
    pub fn topic(&self) -> Topic {
        match self {
            Claim::FirstBorn(_) => Topic::BirthOrder,
            Claim::ManySiblings(_) => Topic::Siblings,
            Claim::Mother(_) => Topic::Mother,
            Claim::Father(_) => Topic::Father,
            Claim::Mark { .. } => Topic::Mark,
            Claim::Year { event: YearEvent::Move, .. } => Topic::Move,
            Claim::Year { event: YearEvent::Marriage, .. } => Topic::Marriage,
            Claim::Year { event: YearEvent::Job, .. } => Topic::Job,
        }
    }
}

/// One fact read from the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartFact {
    /// What it claims.
    pub claim: Claim,
    /// The question, in plain words in the reader's language.
    pub statement: String,
    /// How strongly the recorded chart says it, 0–1 (only clear ones are said).
    pub strength: f64,
    /// True when the claim changes across the birth-time window, so the answer tells
    /// nearby birth times apart.
    pub sensitive: bool,
    /// The chart reason, in an astrologer's words (expert view only).
    pub why_expert: String,
    /// The text it rests on.
    pub source: &'static str,
}

/// One answered fact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FactAnswer {
    /// The claim answered.
    pub claim: Claim,
    /// True for Yes, false for No.
    pub yes: bool,
}

/// One dated event the person logged themselves (the diary's life events, the ledger).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Logged {
    /// When it happened.
    pub at: Instant,
    /// Which life.
    pub life: Life,
    /// True for a good event (a start, a gain, a milestone), false for a hard one.
    pub good: bool,
}

/// What one candidate chart says, prepared once.
pub(crate) struct View<'a> {
    facts: &'a Facts,
    asc: f64,
    moon: f64,
    birth: Instant,
    lon: f64,
}

impl<'a> View<'a> {
    pub(crate) fn new(natal: &'a Natal, asc: f64, lon: f64) -> Self {
        View { facts: &natal.facts, asc, moon: natal.moon_sidereal, birth: natal.birth, lon }
    }

    fn lord_of(&self, h: u8) -> DashaLord {
        self.facts.rasi_lords[((h as usize) + 11) % 12]
    }

    fn aspects(&self, g: DashaLord, h: u8) -> bool {
        let p = self.facts.planet(g);
        aspected_houses(g, p.rasi_house).contains(&h)
    }
}

/// How well a house of the co-born (3 or 11) is kept: its lord's dignity and placement,
/// benefics in or aspecting it, Saturn and the nodes in it.
fn sibling_house(v: &View, h: u8) -> f64 {
    let f = v.facts;
    let lord = v.lord_of(h);
    let lp = f.planet(lord);
    let mut s = (dignity(lord, lp.sidereal).score() - 0.45) * 1.2;
    if [6, 8, 12].contains(&lp.rasi_house) {
        s -= 0.4;
    }
    if [1, 4, 7, 10, 5, 9].contains(&lp.rasi_house) || lp.rasi_house == h {
        s += 0.2;
    }
    for g in f.rasi_occupants(h) {
        s += match g {
            _ if is_benefic(f, g) => 0.45,
            DashaLord::Saturn => -0.55,
            DashaLord::Rahu | DashaLord::Ketu => -0.35,
            DashaLord::Sun => -0.3,
            DashaLord::Mars => 0.1,
            _ => 0.0,
        };
    }
    for p in &f.planets {
        if p.rasi_house != h && v.aspects(p.graha, h) {
            s += match p.graha {
                g if is_benefic(f, g) => 0.25,
                DashaLord::Saturn => -0.25,
                _ => -0.05,
            };
        }
    }
    s
}

fn mars_term(v: &View) -> f64 {
    let m = v.facts.planet(DashaLord::Mars);
    dignity(DashaLord::Mars, m.sidereal).score() - 0.45
}

/// Evidence of elder siblings: positive many, negative none (first-born).
fn elder(v: &View) -> f64 {
    sibling_house(v, 11) + 0.3 * mars_term(v)
}

/// Evidence of many siblings: positive many, negative few.
fn many(v: &View) -> f64 {
    let f = v.facts;
    let occupied = (f.rasi_occupants(3).len() + f.rasi_occupants(11).len()) as f64;
    let mars_in = matches!(f.planet(DashaLord::Mars).rasi_house, 3 | 11) as u8 as f64;
    0.5 * (sibling_house(v, 3) + sibling_house(v, 11)) + 0.2 * (occupied - 1.0) + 0.5 * mars_term(v) + 0.2 * mars_in
}

/// How much each planet colours a parent: the house (4 or 9) and its significator
/// (Moon or Sun). The nodes are left out: the texts give them no plain parent trait.
fn parent(v: &View, h: u8, karaka: DashaLord) -> [f64; 9] {
    let f = v.facts;
    let mut w = [0.0; 9];
    let k = f.planet(karaka);
    for p in &f.planets {
        let i = gi(p.graha);
        if p.rasi_house == h {
            w[i] += 1.0;
        }
        if p.rasi_house != h && v.aspects(p.graha, h) {
            w[i] += 0.6;
        }
        if p.graha != karaka {
            if p.sign == k.sign {
                w[i] += 0.8;
            } else if v.aspects(p.graha, k.rasi_house) {
                w[i] += 0.5;
            }
        }
    }
    w[gi(v.lord_of(h))] += 0.5;
    w[gi(karaka)] += 0.6 + 0.4 * (dignity(karaka, k.sidereal).score() - 0.45);
    w[7] = 0.0;
    w[8] = 0.0;
    w
}

fn parent_lead(w: &[f64; 9]) -> (usize, f64) {
    let mut order: Vec<usize> = (0..7).collect();
    order.sort_by(|a, b| w[*b].partial_cmp(&w[*a]).unwrap_or(std::cmp::Ordering::Equal));
    (order[0], w[order[0]] - w[order[1]])
}

/// Every mark the chart shows: (decanate, part, side, strength, the planet making it),
/// strongest first.
fn marks(v: &View) -> Vec<(u8, u8, Side, f64, DashaLord)> {
    let f = v.facts;
    let decanate = ((v.asc.rem_euclid(30.0)) / 10.0).floor().min(2.0) as u8;
    let mut out: Vec<(u8, u8, Side, f64, DashaLord)> = Vec::new();
    for p in &f.planets {
        let h = p.rasi_house;
        let part = match h {
            1 => 0,
            7 => 6,
            h if h < 7 => h - 1,
            h => 13 - h,
        };
        if part_words(decanate, part).is_none() {
            continue;
        }
        let side = match h {
            1 | 7 => Side::Middle,
            2..=6 => Side::Right,
            _ => Side::Left,
        };
        let base = match p.graha {
            DashaLord::Mars => 1.0,
            DashaLord::Saturn => 0.9,
            DashaLord::Sun | DashaLord::Rahu | DashaLord::Ketu => 0.7,
            g if is_benefic(f, g) => 0.35,
            _ => 0.5,
        };
        // A fixed sign or the planet's own: a mark from birth, the texts say.
        let fixed = matches!(p.sign, 1 | 4 | 7 | 10) || matches!(dignity(p.graha, p.sidereal), crate::parashari::Dignity::Own);
        let s: f64 = if fixed { base + 0.1 } else { base };
        out.push((decanate, part, side, s.min(1.0), p.graha));
    }
    out.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// The houses an event's periods must signify, and those that deny it.
fn event_houses(e: YearEvent) -> (&'static [u8], &'static [u8], &'static str, u32) {
    match e {
        YearEvent::Marriage => (&[2, 7, 11], &[1, 6, 10], "Marriage", 21),
        YearEvent::Job => (&[2, 6, 10, 11], &[5, 9], "Career", 19),
        YearEvent::Move => (&[3, 4, 12], &[], "Property", 16),
    }
}

fn local_year(t: Instant, lon: f64) -> i32 {
    Utc::from_instant(t.add_ns((lon / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128)).year as i32
}

/// Per local calendar year from the event's earliest age to `now`, the mean score of
/// the periods running (sampled every fortnight).
fn year_scores(v: &View, e: YearEvent, now: Instant) -> Vec<(i32, f64)> {
    let f = v.facts;
    let (want, deny, area_name, min_age) = event_houses(e);
    let Some(area) = crate::areas::area(area_name) else { return Vec::new() };
    let promised = promise(f, area).strength.clamp(0.0, 1.0);
    let link: [f64; 9] = std::array::from_fn(|i| {
        let sig = significators(f, LORDS[i]);
        let w = |hs: &[u8]| -> f64 {
            if hs.is_empty() {
                return 0.0;
            }
            hs.iter().map(|h| sig.iter().filter(|s| s.house == *h).map(|s| s.level.weight()).fold(0.0, f64::max)).sum::<f64>() / hs.len() as f64
        };
        (w(want) - 0.5 * w(deny)).max(0.0)
    });
    let from = v.birth.add_ns(min_age as i128 * YEAR_NS);
    if from >= now {
        return Vec::new();
    }
    // Third-level periods overlapping [from, now], with their score.
    let mut spans: Vec<(Instant, Instant, f64)> = Vec::new();
    for m in vimshottari(v.moon, v.birth, YEAR_365_25_NS) {
        if m.end <= from || m.start > now {
            continue;
        }
        for a in subdivide(m) {
            if a.end <= from || a.start > now {
                continue;
            }
            for p in subdivide(a) {
                if p.end <= from || p.start > now {
                    continue;
                }
                let s = (0.5 + 0.5 * promised) * (0.35 * link[gi(m.lord)] + 0.45 * link[gi(a.lord)] + 0.2 * link[gi(p.lord)]);
                spans.push((p.start, p.end, s));
            }
        }
    }
    let step = 15 * NS_PER_DAY;
    let mut acc: Vec<(i32, f64, u32)> = Vec::new();
    let mut t = from;
    let mut j = 0usize;
    while t < now {
        while j + 1 < spans.len() && spans[j].1 <= t {
            j += 1;
        }
        if let Some(sp) = spans.get(j) {
            let y = local_year(t, v.lon);
            match acc.last_mut() {
                Some(last) if last.0 == y => {
                    last.1 += sp.2;
                    last.2 += 1;
                }
                _ => acc.push((y, sp.2, 1)),
            }
        }
        t = t.add_ns(step);
    }
    acc.into_iter().filter(|a| a.2 >= 6).map(|(y, s, n)| (y, s / n as f64)).collect()
}

/// The best year and its lead over every year outside it ± 2, and the candidate's
/// best within ± 1 of `year` against the best outside ± 2.
fn year_lead(ys: &[(i32, f64)], year: i32) -> f64 {
    let near = ys.iter().filter(|(y, _)| (y - year).abs() <= 1).map(|x| x.1).fold(f64::NEG_INFINITY, f64::max);
    let far = ys.iter().filter(|(y, _)| (y - year).abs() > 2).map(|x| x.1).fold(f64::NEG_INFINITY, f64::max);
    if !near.is_finite() {
        return -1.0;
    }
    if !far.is_finite() {
        return 0.0;
    }
    near - far
}

/// How strongly a candidate chart says the claim, −1 (it says the opposite) to 1.
pub(crate) fn support(claim: &Claim, v: &View, now: Instant) -> f64 {
    match *claim {
        Claim::FirstBorn(first) => {
            let e = (elder(v) / 0.8).tanh();
            if first {
                -e
            } else {
                e
            }
        }
        Claim::ManySiblings(m) => {
            let e = (many(v) / 0.8).tanh();
            if m {
                e
            } else {
                -e
            }
        }
        Claim::Mother(g) | Claim::Father(g) => {
            let w = if matches!(claim, Claim::Mother(_)) { parent(v, 4, DashaLord::Moon) } else { parent(v, 9, DashaLord::Sun) };
            let i = gi(g);
            let best_other = (0..7).filter(|j| *j != i).map(|j| w[j]).fold(f64::NEG_INFINITY, f64::max);
            ((w[i] - best_other) / 0.6).tanh()
        }
        Claim::Mark { decanate, part, side } => {
            let ms = marks(v);
            if let Some(m) = ms.iter().find(|m| m.0 == decanate && m.1 == part && m.2 == side) {
                m.3
            } else if ms.first().map(|m| m.3 >= 0.9).unwrap_or(false) {
                -0.5
            } else {
                -0.2
            }
        }
        Claim::Year { event, year } => (year_lead(&year_scores(v, event, now), year) / 0.06).tanh(),
    }
}

/// The chance a candidate gives to a Yes: never surer than 80 % either way, since no
/// single fact from a chart is better than that.
fn p_yes(s: f64) -> f64 {
    (0.5 + 0.3 * s).clamp(0.2, 0.8)
}

const SAYS: [&str; 3] = ["Your chart says", "ജനനസമയം അനുസരിച്ച്,", "आपके जन्म-समय के अनुसार,"];
const RIGHT: [&str; 3] = ["Is that right?", "ശരിയാണോ?", "क्या यह सही है?"];

fn wrap(lang: Lang, body: &str) -> String {
    match lang {
        Lang::En => format!("{} {body} {}", SAYS[0], RIGHT[0]),
        Lang::Ml => format!("{} {body} {}", SAYS[1], RIGHT[1]),
        Lang::Hi => format!("{} {body} {}", SAYS[2], RIGHT[2]),
    }
}

/// A part of the body: English, Malayalam (a locative phrase), Hindi (with "पर"), and
/// whether the Hindi side word is feminine. `{s}` is the side. None for the private parts.
fn part_words(decanate: u8, part: u8) -> Option<(&'static str, &'static str, &'static str, bool)> {
    Some(match (decanate, part) {
        (0, 0) => ("your head", "തലയിൽ", "आपके सिर पर", false),
        (0, 1) => ("your {s} eye", "{s}കണ്ണിന്റെ ഭാഗത്ത്", "आपकी {s} आँख पर", true),
        (0, 2) => ("your {s} ear", "{s}ചെവിയുടെ ഭാഗത്ത്", "आपके {s} कान पर", false),
        (0, 3) => ("the {s} side of your nose", "മൂക്കിന്റെ {s}വശത്ത്", "आपकी नाक के {s} ओर", true),
        (0, 4) => ("your {s} cheek", "{s}കവിളിൽ", "आपके {s} गाल पर", false),
        (0, 5) => ("the {s} side of your chin", "താടിയുടെ {s}വശത്ത്", "आपकी ठुड्डी के {s} ओर", true),
        (0, 6) => ("your mouth", "വായയുടെ ഭാഗത്ത്", "आपके मुँह पर", false),
        (1, 0) => ("your neck", "കഴുത്തിൽ", "आपकी गर्दन पर", true),
        (1, 1) => ("your {s} shoulder", "{s}തോളിൽ", "आपके {s} कंधे पर", false),
        (1, 2) => ("your {s} arm", "{s}കൈയിൽ", "आपकी {s} बाँह पर", true),
        (1, 3) => ("your {s} side, by the ribs", "{s}വശത്ത് വാരിയെല്ലിനടുത്ത്", "आपकी {s} पसलियों पर", true),
        (1, 4) => ("the {s} side of your chest", "നെഞ്ചിന്റെ {s}വശത്ത്", "आपकी छाती के {s} ओर", true),
        (1, 5) => ("the {s} side of your belly", "വയറിന്റെ {s}വശത്ത്", "आपके पेट के {s} ओर", true),
        (1, 6) => ("your navel", "പൊക്കിളിനടുത്ത്", "आपकी नाभि पर", true),
        (2, 0) => ("your lower belly", "അടിവയറ്റിൽ", "आपके पेट के निचले हिस्से पर", false),
        (2, 3) => ("your {s} thigh", "{s}തുടയിൽ", "आपकी {s} जाँघ पर", true),
        (2, 4) => ("your {s} knee", "{s}കാൽമുട്ടിൽ", "आपके {s} घुटने पर", false),
        (2, 5) => ("your {s} lower leg", "{s}കാലിന്റെ താഴ്ഭാഗത്ത്", "आपकी {s} पिंडली पर", true),
        (2, 6) => ("your feet", "പാദങ്ങളിൽ", "आपके पैरों पर", false),
        _ => return None,
    })
}

/// The plain trait each planet gives a parent (Brihat Jataka ch. 2's natures, said the
/// way a person would describe their mother or father). English and Malayalam
/// describe the person; Hindi describes the nature (स्वभाव, so one form fits both).
fn trait_words(g: DashaLord) -> [&'static str; 3] {
    match g {
        DashaLord::Sun => ["proud and dignified, someone used to taking charge", "അഭിമാനവും അന്തസ്സുമുള്ള, കാര്യങ്ങൾ മുന്നിൽ നിന്ന് നയിക്കുന്ന", "स्वाभिमानी, गरिमामय और आगे बढ़कर ज़िम्मेदारी संभालने वाला"],
        DashaLord::Moon => ["caring and emotional, deeply attached to home and family", "സ്നേഹവും കരുതലുമുള്ള, വീടിനോടും കുടുംബത്തോടും വളരെ അടുപ്പമുള്ള", "ममतामय, भावुक और घर-परिवार से गहरा जुड़ा हुआ"],
        DashaLord::Mars => ["quick, outspoken and hard-working, with a quick temper", "ചുറുചുറുക്കുള്ള, തുറന്നു പറയുന്ന, കഠിനാധ്വാനിയായ, പെട്ടെന്ന് ദേഷ്യം വരുന്ന", "फुर्तीला, साफ़ बोलने वाला और मेहनती, पर जल्दी ग़ुस्सा होने वाला"],
        DashaLord::Mercury => ["talkative, clever and practical, good with words or numbers", "നന്നായി സംസാരിക്കുന്ന, ബുദ്ധിയും പ്രായോഗികതയുമുള്ള", "बातूनी, चतुर और व्यावहारिक, शब्दों या हिसाब में तेज़"],
        DashaLord::Jupiter => ["kind, wise and respected, someone others go to for advice", "ദയയും അറിവുമുള്ള, മറ്റുള്ളവർ ഉപദേശം തേടിച്ചെല്ലുന്ന, എല്ലാവരും ബഹുമാനിക്കുന്ന", "दयालु, समझदार और सम्मानित, जिससे लोग सलाह लेने आते हैं"],
        DashaLord::Venus => ["gentle and graceful, fond of beauty, comfort and good company", "സൗമ്യതയും ചാരുതയുമുള്ള, ഭംഗിയും സുഖവും നല്ല കൂട്ടും ഇഷ്ടപ്പെടുന്ന", "कोमल और सुरुचिपूर्ण, सुंदरता, आराम और अच्छी संगत पसंद करने वाला"],
        _ => ["serious, disciplined and hard-working, not one to show feelings easily", "ഗൗരവവും അച്ചടക്കവുമുള്ള, കഠിനാധ്വാനം ചെയ്യുന്ന, വികാരങ്ങൾ എളുപ്പം പുറത്തു കാണിക്കാത്ത", "गंभीर, अनुशासित और मेहनती, जो भावनाएँ आसानी से नहीं दिखाता"],
    }
}

/// The question for a claim, in plain words.
pub fn statement(lang: Lang, claim: &Claim) -> String {
    let li = match lang {
        Lang::En => 0,
        Lang::Ml => 1,
        Lang::Hi => 2,
    };
    let body: String = match *claim {
        Claim::FirstBorn(true) => ["you are the first-born child in your family.", "നിങ്ങൾ വീട്ടിലെ ആദ്യത്തെ കുട്ടിയാണ്.", "आप घर की पहली संतान हैं।"][li].into(),
        Claim::FirstBorn(false) => ["you have an elder brother or sister.", "നിങ്ങൾക്ക് ഒരു ചേട്ടനോ ചേച്ചിയോ ഉണ്ട്.", "आपके एक बड़े भाई या बड़ी बहन हैं।"][li].into(),
        Claim::ManySiblings(true) => ["you have three or more brothers and sisters.", "നിങ്ങൾക്ക് മൂന്നോ അതിലധികമോ സഹോദരങ്ങളുണ്ട്.", "आपके तीन या उससे ज़्यादा भाई-बहन हैं।"][li].into(),
        Claim::ManySiblings(false) => ["you have at most one brother or sister.", "നിങ്ങൾക്ക് പരമാവധി ഒരു സഹോദരനോ സഹോദരിയോ മാത്രമേ ഉള്ളൂ.", "आपके ज़्यादा से ज़्यादा एक भाई या बहन है।"][li].into(),
        Claim::Mother(g) | Claim::Father(g) => {
            let mother = matches!(claim, Claim::Mother(_));
            let t = trait_words(g)[li];
            match lang {
                Lang::En => format!("your {} is {t}.", if mother { "mother" } else { "father" }),
                Lang::Ml => format!("നിങ്ങളുടെ {} {t} ഒരാളാണ്.", if mother { "അമ്മ" } else { "അച്ഛൻ" }),
                Lang::Hi => format!("{} का स्वभाव {t} है।", if mother { "आपकी माँ" } else { "आपके पिता" }),
            }
        }
        Claim::Mark { decanate, part, side } => {
            let (en, ml, hi, fem) = part_words(decanate, part).unwrap_or(("your body", "ശരീരത്തിൽ", "आपके शरीर पर", false));
            let s = match (side, lang, fem) {
                (Side::Middle, Lang::En, _) => "",
                (Side::Left, Lang::En, _) => "left",
                (Side::Right, Lang::En, _) => "right",
                (Side::Middle, Lang::Ml, _) => "",
                (Side::Left, Lang::Ml, _) => "ഇടത് ",
                (Side::Right, Lang::Ml, _) => "വലത് ",
                (Side::Middle, Lang::Hi, _) => "",
                (Side::Left, Lang::Hi, false) => "बाएँ",
                (Side::Right, Lang::Hi, false) => "दाएँ",
                (Side::Left, Lang::Hi, true) => "बाईं",
                (Side::Right, Lang::Hi, true) => "दाईं",
            };
            let fill = |t: &str| if s.is_empty() { t.replace("{s} ", "").replace("{s}", "") } else { t.replace("{s}", s) };
            match lang {
                Lang::En => format!("you have a mole, birthmark or small scar on or near {}.", fill(en)),
                Lang::Ml => format!("നിങ്ങളുടെ {} ഒരു മറുകോ ജന്മചിഹ്നമോ ചെറിയ പാടോ ഉണ്ട്.", fill(ml)),
                Lang::Hi => format!("{} या उसके पास एक तिल, जन्म-चिह्न या छोटा निशान है।", fill(hi)),
            }
        }
        Claim::Year { event, year } => {
            let (a, b) = (year - 1, year + 1);
            match (event, lang) {
                (YearEvent::Move, Lang::En) => format!("you moved home, or changed the place you live, around {year} (between {a} and {b})."),
                (YearEvent::Marriage, Lang::En) => format!("a marriage or a lasting relationship began for you around {year} (between {a} and {b})."),
                (YearEvent::Job, Lang::En) => format!("you started a new job or changed jobs around {year} (between {a} and {b})."),
                (YearEvent::Move, Lang::Ml) => format!("ഏകദേശം {year}-ൽ ({a} മുതൽ {b} വരെയുള്ള കാലത്ത്) നിങ്ങൾ വീടോ താമസസ്ഥലമോ മാറി."),
                (YearEvent::Marriage, Lang::Ml) => format!("ഏകദേശം {year}-ൽ ({a} മുതൽ {b} വരെയുള്ള കാലത്ത്) നിങ്ങളുടെ ജീവിതത്തിൽ ഒരു വിവാഹമോ നീണ്ടുനിൽക്കുന്ന ബന്ധമോ തുടങ്ങി."),
                (YearEvent::Job, Lang::Ml) => format!("ഏകദേശം {year}-ൽ ({a} മുതൽ {b} വരെയുള്ള കാലത്ത്) നിങ്ങൾ പുതിയ ജോലിയിൽ ചേരുകയോ ജോലി മാറുകയോ ചെയ്തു."),
                (YearEvent::Move, Lang::Hi) => format!("लगभग {year} में ({a} से {b} के बीच) आपने घर या रहने की जगह बदली।"),
                (YearEvent::Marriage, Lang::Hi) => format!("लगभग {year} में ({a} से {b} के बीच) आपके जीवन में कोई विवाह या लंबा रिश्ता शुरू हुआ।"),
                (YearEvent::Job, Lang::Hi) => format!("लगभग {year} में ({a} से {b} के बीच) आपने नई नौकरी शुरू की या नौकरी बदली।"),
            }
        }
    };
    wrap(lang, &body)
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

/// The facts the recorded chart says clearly, before `asked` (keys already asked are
/// skipped), at most one per topic, strongest first within the order the app shows
/// them (family, the body, then the years). `window_min` is how far the birth time may
/// be off: each fact is also judged at nine times across it, to mark it sensitive.
#[allow(clippy::too_many_arguments)]
pub fn chart_facts(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    lang: Lang,
    at: Instant,
    window_min: f64,
    asked: &[String],
) -> Vec<ChartFact> {
    let asc = ascendant(spk, eop, natal.birth, obs, ayanamsa).map(|a| a.0).unwrap_or(natal.facts.cusps[0]);
    let lon = obs.longitude_deg;
    let v = View::new(natal, asc, lon);
    let f = &natal.facts;

    // (claim, strength, why)
    let mut found: Vec<(Claim, f64, String, &'static str)> = Vec::new();
    let e = elder(&v);
    let lord11 = v.lord_of(11);
    let why11 = format!(
        "11th from lagna: lord {:?} in the {} ({:?}); occupants {:?}",
        lord11,
        ordinal(f.planet(lord11).rasi_house),
        dignity(lord11, f.planet(lord11).sidereal),
        f.rasi_occupants(11)
    );
    if e.abs() >= 0.55 {
        found.push((
            Claim::FirstBorn(e < 0.0),
            (e.abs() / 0.8).tanh(),
            why11.clone(),
            lang.pick([
                "BPHS, Sahaja bhava (the 11th for the elder co-born); Mars the karaka",
                "BPHS, സഹജഭാവം (മൂത്ത സഹോദരങ്ങൾക്ക് 11-ാം ഭാവം); ചൊവ്വ കാരകൻ",
                "BPHS, सहज भाव (बड़े भाई-बहनों के लिए 11वाँ भाव); मंगल कारक",
            ]),
        ));
    }
    let m = many(&v);
    if m.abs() >= 0.6 {
        found.push((
            Claim::ManySiblings(m > 0.0),
            (m.abs() / 0.8).tanh(),
            format!("3rd and 11th from lagna: occupants {:?} and {:?}; Mars {:?}", f.rasi_occupants(3), f.rasi_occupants(11), dignity(DashaLord::Mars, f.planet(DashaLord::Mars).sidereal)),
            lang.pick([
                "BPHS, Sahaja bhava (the 3rd and 11th); Mars the karaka of the co-born",
                "BPHS, സഹജഭാവം (3-ഉം 11-ഉം ഭാവങ്ങൾ); സഹോദരങ്ങളുടെ കാരകൻ ചൊവ്വ",
                "BPHS, सहज भाव (तीसरा और 11वाँ भाव); भाई-बहनों का कारक मंगल",
            ]),
        ));
    }
    for (h, karaka, mother) in [(4u8, DashaLord::Moon, true), (9u8, DashaLord::Sun, false)] {
        let w = parent(&v, h, karaka);
        let (top, lead) = parent_lead(&w);
        if w[top] >= 1.2 && lead >= 0.4 {
            let g = LORDS[top];
            found.push((
                if mother { Claim::Mother(g) } else { Claim::Father(g) },
                (lead / 0.6).tanh(),
                format!("{} from lagna and {:?}: {:?} colours them most ({:.1} against {:.1})", ordinal(h), karaka, g, w[top], w[top] - lead),
                if mother {
                    lang.pick([
                        "BPHS, Sukha bhava (the 4th, the mother); Moon the karaka; natures from Brihat Jataka ch. 2",
                        "BPHS, സുഖഭാവം (4-ാം ഭാവം, അമ്മ); ചന്ദ്രൻ കാരകൻ; സ്വഭാവങ്ങൾ ബൃഹജ്ജാതകം അധ്യായം 2-ൽ നിന്ന്",
                        "BPHS, सुख भाव (चौथा भाव, माँ); चंद्र कारक; स्वभाव बृहज्जातक अध्याय 2 से",
                    ])
                } else {
                    lang.pick([
                        "BPHS, Dharma bhava (the 9th, the father); Sun the karaka; natures from Brihat Jataka ch. 2",
                        "BPHS, ധർമ്മഭാവം (9-ാം ഭാവം, അച്ഛൻ); സൂര്യൻ കാരകൻ; സ്വഭാവങ്ങൾ ബൃഹജ്ജാതകം അധ്യായം 2-ൽ നിന്ന്",
                        "BPHS, धर्म भाव (नौवाँ भाव, पिता); सूर्य कारक; स्वभाव बृहज्जातक अध्याय 2 से",
                    ])
                },
            ));
        }
    }
    if let Some(&(d, part, side, s, g)) = marks(&v).first() {
        if s >= 0.9 {
            found.push((
                Claim::Mark { decanate: d, part, side },
                s,
                format!("rising decanate {}; {:?} in the {}", d + 1, g, ordinal(f.planet(g).rasi_house)),
                lang.pick([
                    "Brihat Jataka ch. 5 (Janmakala-lakshana): the decanate's body, the visible half left",
                    "ബൃഹജ്ജാതകം അധ്യായം 5 (ജന്മകാലലക്ഷണം): ദ്രേക്കാണത്തിന്റെ ശരീരഭാഗം, കാണുന്ന പകുതി ഇടത്",
                    "बृहज्जातक अध्याय 5 (जन्मकाल-लक्षण): द्रेष्काण का अंग, दिखने वाला आधा बायाँ",
                ]),
            ));
        }
    }
    for e in [YearEvent::Move, YearEvent::Marriage, YearEvent::Job] {
        let ys = year_scores(&v, e, at);
        let Some(&(best, top)) = ys.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)) else { continue };
        // The year either side of the last year is not yet over.
        if best >= local_year(at, lon) {
            continue;
        }
        let lead = year_lead(&ys, best);
        if top >= 0.15 && lead >= 0.04 {
            found.push((Claim::Year { event: e, year: best }, (lead / 0.06).tanh().min(1.0), format!("third-level periods best in {best}: {top:.2}, ahead by {lead:.2}"), lang.pick([
                "KP significators of the event's houses in the Vimshottari periods; the natal promise as in the ledger's retrodiction",
                "വിംശോത്തരി കാലങ്ങളിൽ സംഭവത്തിന്റെ ഭാവങ്ങളുടെ KP സൂചകങ്ങൾ; ജനനസമയത്തെ വാഗ്ദാനം, രേഖയിലെ പിൻവായനയിലെ പോലെ",
                "विंशोत्तरी दशाओं में घटना के भावों के KP सूचक; जन्म का वादा, रिकॉर्ड की पिछली पढ़त की तरह",
            ])));
        }
    }

    // Sensitivity: the claim judged at nine times across the window.
    let w = window_min.clamp(0.0, 180.0);
    let shifts: Vec<f64> = (-4..=4).map(|k| w * k as f64 / 4.0).collect();
    let others: Vec<Option<(Natal, f64)>> = par_map(&shifts, |s| {
        if s.abs() < 1e-9 {
            return None;
        }
        let b = natal.birth.add_ns((s * 60.0 * NS_PER_SEC as f64) as i128);
        let n = Natal::compute(spk, eop, b, obs, system, ayanamsa).ok()?;
        let a = ascendant(spk, eop, b, obs, ayanamsa).map(|a| a.0).unwrap_or(n.facts.cusps[0]);
        Some((n, a))
    });
    let mut out: Vec<ChartFact> = Vec::new();
    for (claim, strength, why, source) in found {
        if asked.iter().any(|k| *k == claim.key()) || out.iter().any(|x| x.claim.topic() == claim.topic()) {
            continue;
        }
        let here = support(&claim, &v, at);
        let mut lo = here;
        let mut hi = here;
        for (n, a) in others.iter().flatten() {
            let s = support(&claim, &View::new(n, *a, lon), at);
            lo = lo.min(s);
            hi = hi.max(s);
        }
        out.push(ChartFact { statement: statement(lang, &claim), claim, strength: strength.clamp(0.0, 1.0), sensitive: hi - lo >= 0.25, why_expert: why, source });
    }
    out
}

/// A logged event as an answered stretch: the three months around it, said Yes.
pub fn logged_answer(e: &Logged) -> Answer {
    let half = 45 * NS_PER_DAY;
    Answer { from: e.at.add_ns(-half), to: e.at.add_ns(half), life: e.life, good: e.good, verdict: Verdict::Yes }
}

/// The life an area or life name stands for: the app's lives ("Love") and the ledger's
/// areas ("Marriage", "Wealth", "Property", "Education", "Foreign").
pub fn life_of(name: &str) -> Option<Life> {
    Life::parse(name).or_else(|| match name.trim().to_ascii_lowercase().as_str() {
        "marriage" => Some(Life::Love),
        "wealth" => Some(Life::Money),
        "property" => Some(Life::Home),
        "education" => Some(Life::Study),
        "foreign" => Some(Life::Travel),
        _ => None,
    })
}

/// The pooled birth-time belief, with the stretch of times holding most of it.
#[derive(Clone, Debug, PartialEq)]
pub struct BirthNow {
    /// The refinement over every piece of evidence (candidates, Bayes factor, the
    /// chart's boundaries and their bracket), as [`crate::reveal::refine_birth_detail`].
    pub refine: BirthRefine,
    /// Hull of the highest-density set holding [`HPD_MASS`] of the belief, as cell
    /// edges: the stretch of birth times the evidence points to.
    pub hpd: Option<(Instant, Instant)>,
    /// Separate runs inside that set (1 = one stretch).
    pub hpd_runs: usize,
    /// Step between candidates, minutes.
    pub step_min: f64,
    /// Answered stretches used.
    pub stretches: usize,
    /// Answered facts used (only those that change across the window count).
    pub facts: usize,
    /// Answered facts left out because every time in the window says the same.
    pub facts_flat: usize,
    /// Logged events used.
    pub logged: usize,
    /// The time to suggest, when the evidence is strong and clearly prefers it.
    pub suggested: Option<BirthFit>,
}

/// Share of the belief inside [`BirthNow::hpd`].
pub const HPD_MASS: f64 = 0.8;

/// Every birth time within ±`window_min` minutes, in steps of `step_min` (widened to
/// keep at most 61), scored against every piece of evidence at once: the answered
/// stretches (all seven witnesses of [`crate::witness`]), the logged events (each read
/// as the three months around it said Yes) and the answered facts (each candidate chart
/// judged afresh: a Yes scores `ln p`, a No `ln(1 − p)`, `p` within 20–80 %). Facts
/// every candidate says alike carry nothing about the minute and are left out, so they
/// cannot inflate the evidence. A mild Gaussian prior (σ = the window, at least four
/// minutes) keeps the recorded time unless the evidence clearly prefers another; the
/// app suggests, the person decides.
#[allow(clippy::too_many_arguments)]
pub fn refine_birth_pooled(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    birth: Instant,
    window_min: f64,
    step_min: f64,
    answers: &[Answer],
    facts: &[FactAnswer],
    logged: &[Logged],
    at: Instant,
    with_boundaries: bool,
) -> BirthNow {
    let mut all: Vec<Answer> = answers.to_vec();
    all.extend(logged.iter().map(logged_answer));
    let empty = |step: f64| BirthNow {
        refine: BirthRefine { fits: Vec::new(), log10_bayes: 0.0, boundaries: Vec::new(), bracket: None, d60_in_bracket: 0 },
        hpd: None,
        hpd_runs: 0,
        step_min: step,
        stretches: answers.len(),
        facts: 0,
        facts_flat: facts.len(),
        logged: logged.len(),
        suggested: None,
    };
    let window_min = window_min.clamp(0.0, 180.0);
    let step = step_min.max(window_min / 30.0).max(0.5);
    if all.is_empty() && facts.is_empty() {
        return empty(step);
    }
    let n_side = (window_min / step).floor() as i64;
    let sigma = window_min.max(4.0);
    let ks: Vec<i64> = (-n_side..=n_side).collect();
    let lon = obs.longitude_deg;
    type Scored = (f64, f64, Vec<f64>, BirthFit);
    let (scored, bounds): (Vec<Scored>, _) = std::thread::scope(|s| {
        let bounds = with_boundaries.then(|| s.spawn(|| boundaries(spk, eop, obs, ayanamsa, birth, window_min)));
        let prep = if all.is_empty() { None } else { prepare(spk, eop, obs, system, ayanamsa, birth, &all) };
        let scored: Vec<Scored> = if !all.is_empty() && prep.is_none() {
            Vec::new()
        } else {
            par_map(&ks, |&k| {
                let shift = k as f64 * step;
                let b = birth.add_ns((shift * 60.0 * NS_PER_SEC as f64) as i128);
                let mut like = 0.0;
                let mut explained = 0;
                let mut by_witness = [0.0; NW];
                let (mut lagna_sign, mut moon_star) = (0u8, 0usize);
                if let Some(prep) = prep.as_ref() {
                    let j = judge(spk, eop, obs, system, ayanamsa, b, prep, &all)?;
                    lagna_sign = j.lagna_sign;
                    moon_star = j.moon_star;
                    for (a, z) in all.iter().zip(&j.zs) {
                        let logit: f64 = WITNESSES.iter().map(|w| w.weight() * z[w.index()].unwrap_or(0.0)).sum::<f64>() * SLOPE;
                        like += gain(a.verdict, prob(logit));
                        for (w, zw) in z.iter().enumerate() {
                            if let Some(zw) = zw {
                                by_witness[w] += gain(a.verdict, prob(SLOPE * zw));
                            }
                        }
                        let hit = match a.verdict {
                            Verdict::Yes => logit > 0.48,
                            Verdict::Close => logit > 0.0,
                            Verdict::No => logit < 0.48,
                        };
                        if hit {
                            explained += 1;
                        }
                    }
                }
                let mut ps = Vec::with_capacity(facts.len());
                if !facts.is_empty() {
                    let natal = Natal::compute(spk, eop, b, obs, system, ayanamsa).ok()?;
                    let asc = ascendant(spk, eop, b, obs, ayanamsa).map(|a| a.0).unwrap_or(natal.facts.cusps[0]);
                    if prep.is_none() {
                        lagna_sign = natal.facts.lagna_sign;
                        moon_star = natal.janma();
                    }
                    let v = View::new(&natal, asc, lon);
                    for fa in facts {
                        ps.push(p_yes(support(&fa.claim, &v, at)));
                    }
                }
                let prior = -0.5 * (shift / sigma) * (shift / sigma);
                Some((prior, like, ps, BirthFit { birth: b, shift_min: shift, prob: 0.0, explained, lagna_sign, moon_star, by_witness }))
            })
            .into_iter()
            .flatten()
            .collect()
        };
        (scored, bounds.and_then(|h| h.join().ok()))
    });
    if scored.is_empty() {
        return empty(step);
    }
    // Facts every candidate judges alike say nothing about the minute.
    let keep: Vec<bool> = (0..facts.len())
        .map(|i| {
            let lo = scored.iter().map(|x| x.2[i]).fold(f64::INFINITY, f64::min);
            let hi = scored.iter().map(|x| x.2[i]).fold(f64::NEG_INFINITY, f64::max);
            hi - lo >= 0.02
        })
        .collect();
    let used = keep.iter().filter(|k| **k).count();
    let scored: Vec<(f64, f64, BirthFit)> = scored
        .into_iter()
        .map(|(prior, mut like, ps, mut fit)| {
            for (i, fa) in facts.iter().enumerate() {
                if keep[i] {
                    let v = if fa.yes { Verdict::Yes } else { Verdict::No };
                    like += gain(v, ps[i]);
                    if (ps[i] > 0.5) == fa.yes {
                        fit.explained += 1;
                    }
                }
            }
            (prior, like, fit)
        })
        .collect();
    let lse = |v: Vec<f64>| -> f64 {
        let top = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        top + v.iter().map(|x| (x - top).exp()).sum::<f64>().ln()
    };
    let log_prior_norm = lse(scored.iter().map(|x| x.0).collect());
    let log_post_norm = lse(scored.iter().map(|x| x.0 + x.1).collect());
    let log_bf = log_post_norm - log_prior_norm;
    let mut fits: Vec<BirthFit> = scored
        .into_iter()
        .map(|(p, l, mut f)| {
            f.prob = (p + l - log_post_norm).exp();
            f
        })
        .collect();
    // The highest-density set, on the grid in time order.
    let step_ns = (step * 60.0 * NS_PER_SEC as f64) as i128;
    let mut by_time = fits.clone();
    by_time.sort_by(|a, b| a.shift_min.partial_cmp(&b.shift_min).unwrap_or(std::cmp::Ordering::Equal));
    let mut order: Vec<usize> = (0..by_time.len()).collect();
    order.sort_by(|a, b| by_time[*b].prob.partial_cmp(&by_time[*a].prob).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(b)));
    let mut chosen = Vec::new();
    let mut acc = 0.0;
    for i in order {
        chosen.push(i);
        acc += by_time[i].prob;
        if acc >= HPD_MASS - 1e-12 {
            break;
        }
    }
    chosen.sort_unstable();
    let hpd_runs = if chosen.is_empty() { 0 } else { 1 + chosen.windows(2).filter(|w| w[1] != w[0] + 1).count() };
    let hpd = match (chosen.first(), chosen.last()) {
        (Some(a), Some(b)) => Some((by_time[*a].birth.add_ns(-step_ns / 2), by_time[*b].birth.add_ns(step_ns / 2))),
        _ => None,
    };

    fits.sort_by(|a, b| b.prob.partial_cmp(&a.prob).unwrap_or(std::cmp::Ordering::Equal));
    let (boundaries, d60) = bounds.unwrap_or_default();
    let (bracket, d60_in_bracket) = match (with_boundaries, fits.first()) {
        (true, Some(best)) => {
            let edge = |m: f64| birth.add_ns((m * 60.0 * NS_PER_SEC as f64) as i128);
            let lo = boundaries.iter().filter(|b| b.shift_min <= best.shift_min).map(|b| b.at).next_back().unwrap_or(edge(-window_min));
            let hi = boundaries.iter().find(|b| b.shift_min > best.shift_min).map(|b| b.at).unwrap_or(edge(window_min));
            (Some((lo, hi)), d60.iter().filter(|t| **t >= lo && **t < hi).count())
        }
        _ => (None, 0),
    };
    let refine = BirthRefine { fits, log10_bayes: log_bf / std::f64::consts::LN_10, boundaries, bracket, d60_in_bracket };
    let counted = all.len() + used;
    let suggested = if refine.log10_bayes >= SUGGEST_LOG10_BAYES { suggested_shift(&refine.fits, counted) } else { None };
    BirthNow { refine, hpd, hpd_runs, step_min: step, stretches: answers.len(), facts: used, facts_flat: facts.len() - used, logged: logged.len(), suggested }
}

/// "10:33" at a fixed offset from UTC, in minutes.
fn clock(t: Instant, utc_offset_min: i32) -> String {
    let u = Utc::from_instant(t.add_ns(utc_offset_min as i128 * 60 * NS_PER_SEC));
    format!("{:02}:{:02}", u.hour, u.minute)
}

/// One plain sentence on where the evidence places the birth: "Your 31 answers so far
/// place your birth between 10:33 and 10:36." Times are read at `utc_offset_min` (the
/// birth clock's offset). Only strong evidence (20 to 1 against answers at random)
/// places it; weaker evidence is said to agree with, or lean from, the recorded time.
pub fn birth_now_words(lang: Lang, r: &BirthNow, recorded: Instant, utc_offset_min: i32) -> String {
    let n = r.stretches + r.facts + r.logged + r.facts_flat;
    let rec = clock(recorded, utc_offset_min);
    if n == 0 || r.refine.fits.is_empty() {
        return match lang {
            Lang::En => format!("Answer a few questions and I will check your birth time, {rec}, against your own life."),
            Lang::Ml => format!("കുറച്ച് ചോദ്യങ്ങൾക്ക് ഉത്തരം തരൂ; നിങ്ങളുടെ ജനനസമയം, {rec}, നിങ്ങളുടെ ജീവിതവുമായി ഒത്തുനോക്കാം."),
            Lang::Hi => format!("कुछ सवालों के जवाब दीजिए, फिर मैं आपके जन्म-समय {rec} को आपके अपने जीवन से मिलाकर देखूँगा।"),
        };
    }
    let strong = r.refine.log10_bayes >= SUGGEST_LOG10_BAYES;
    match (r.hpd, strong) {
        (Some((a, b)), true) => {
            let (a, b) = (clock(a, utc_offset_min), clock(b, utc_offset_min));
            if a == b {
                match lang {
                    Lang::En => format!("Your {n} answers so far place your birth at about {a}."),
                    Lang::Ml => format!("ഇതുവരെയുള്ള നിങ്ങളുടെ {n} ഉത്തരങ്ങൾ അനുസരിച്ച് ജനനം ഏകദേശം {a}-ന് ആണ്."),
                    Lang::Hi => format!("अब तक के आपके {n} जवाब आपके जन्म को लगभग {a} पर रखते हैं।"),
                }
            } else {
                match lang {
                    Lang::En => format!("Your {n} answers so far place your birth between {a} and {b}."),
                    Lang::Ml => format!("ഇതുവരെയുള്ള നിങ്ങളുടെ {n} ഉത്തരങ്ങൾ അനുസരിച്ച് ജനനം {a}-നും {b}-നും ഇടയിലാണ്."),
                    Lang::Hi => format!("अब तक के आपके {n} जवाब आपके जन्म को {a} और {b} के बीच रखते हैं।"),
                }
            }
        }
        _ => match lang {
            Lang::En => format!("Your {n} answers so far do not yet point clearly away from {rec}. More answers make this surer."),
            Lang::Ml => format!("ഇതുവരെയുള്ള നിങ്ങളുടെ {n} ഉത്തരങ്ങൾ {rec}-ൽ നിന്ന് വ്യക്തമായി മാറുന്നില്ല. കൂടുതൽ ഉത്തരങ്ങൾ ഇത് ഉറപ്പിക്കും."),
            Lang::Hi => format!("अब तक के आपके {n} जवाब {rec} से साफ़ तौर पर अलग नहीं ले जाते। और जवाब इसे पक्का करेंगे।"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn setup() -> Option<(Spk, Observer, Natal, Instant)> {
        let spk = kernel()?;
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        Some((spk, obs, natal, at))
    }

    fn view_at(spk: &Spk, obs: &Observer, b: Instant) -> (Natal, f64) {
        let n = Natal::compute(spk, None, b, obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let a = ascendant(spk, None, b, obs, Ayanamsa::Lahiri).unwrap().0;
        (n, a)
    }

    /// Every mark a chart can show: the midline parts on the midline, the rest on a side.
    fn every_mark() -> Vec<Claim> {
        let mut v = Vec::new();
        for d in 0..3u8 {
            for p in 0..7u8 {
                if part_words(d, p).is_some() {
                    let sides: &[Side] = if p == 0 || p == 6 { &[Side::Middle] } else { &[Side::Left, Side::Right] };
                    for side in sides {
                        v.push(Claim::Mark { decanate: d, part: p, side: *side });
                    }
                }
            }
        }
        v
    }

    #[test]
    fn keys_round_trip() {
        let claims = [
            Claim::FirstBorn(true),
            Claim::FirstBorn(false),
            Claim::ManySiblings(true),
            Claim::ManySiblings(false),
            Claim::Mother(DashaLord::Saturn),
            Claim::Father(DashaLord::Jupiter),
            Claim::Mark { decanate: 1, part: 2, side: Side::Left },
            Claim::Mark { decanate: 0, part: 0, side: Side::Middle },
            Claim::Year { event: YearEvent::Marriage, year: 2016 },
        ];
        for c in claims {
            assert_eq!(Claim::parse(&c.key()), Some(c), "{}", c.key());
        }
        assert_eq!(Claim::parse("mark:3:0:left"), None);
        assert_eq!(Claim::parse("nonsense"), None);
    }

    #[test]
    fn every_statement_is_plain_and_asks() {
        let mut claims = vec![Claim::FirstBorn(true), Claim::FirstBorn(false), Claim::ManySiblings(true), Claim::ManySiblings(false)];
        for g in &LORDS[..7] {
            claims.push(Claim::Mother(*g));
            claims.push(Claim::Father(*g));
        }
        claims.extend(every_mark());
        for e in [YearEvent::Move, YearEvent::Marriage, YearEvent::Job] {
            claims.push(Claim::Year { event: e, year: 2015 });
        }
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for c in &claims {
                let s = statement(lang, c);
                assert_eq!(jargon_in(lang, &s), None, "{s}");
                assert!(!s.contains("{s}") && !s.contains("  "), "{s}");
                if lang == Lang::En {
                    assert!(s.starts_with("Your chart says") && s.ends_with("Is that right?"), "{s}");
                    for w in ["guaranteed", "certain", "fate", "death", "died", "ill"] {
                        assert!(!s.split(|c: char| !c.is_alphanumeric()).any(|x| x.eq_ignore_ascii_case(w)), "{s}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_chart_states_clear_facts_and_marks_the_time_sensitive_ones() {
        let Some((spk, obs, _, at)) = setup() else { return };
        let mut total = 0;
        let mut sensitive = 0;
        let mut topics_seen = std::collections::HashSet::new();
        for (y, mo, d, h) in [(1990, 2, 20, 5), (1985, 7, 3, 12), (1978, 11, 30, 22), (1995, 5, 14, 2), (2001, 9, 9, 16)] {
            let birth = Utc::new(y, mo, d, h, 4, 0, 0).unwrap().to_instant().unwrap();
            let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
            let t = std::time::Instant::now();
            let fs = chart_facts(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &[]);
            eprintln!("{y}: {} facts in {:?}", fs.len(), t.elapsed());
            assert!(!fs.is_empty(), "every chart says something clearly");
            for f in &fs {
                eprintln!("  [{:.2}{}] {} — {} ({})", f.strength, if f.sensitive { ", sensitive" } else { "" }, f.statement, f.why_expert, f.source);
                assert_eq!(jargon_in(Lang::En, &f.statement), None);
                assert!(f.strength > 0.3);
                topics_seen.insert(f.claim.topic());
                sensitive += f.sensitive as usize;
            }
            total += fs.len();
            let topics: std::collections::HashSet<_> = fs.iter().map(|f| f.claim.topic()).collect();
            assert_eq!(topics.len(), fs.len(), "one per topic");
            // Asked facts are not asked again.
            let asked: Vec<String> = fs.iter().map(|f| f.claim.key()).collect();
            let again = chart_facts(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 30.0, &asked);
            assert!(again.iter().all(|f| !asked.contains(&f.claim.key())));
        }
        assert!(total >= 8 && sensitive >= 3 && topics_seen.len() >= 4, "{total} facts, {sensitive} sensitive, topics {topics_seen:?}");
    }

    #[test]
    fn a_mark_moves_when_the_rising_degree_crosses_a_decanate() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        // Find the next decanate boundary of the rising degree, minute by minute.
        let (_, a0) = view_at(&spk, &obs, natal.birth);
        let d0 = (a0.rem_euclid(30.0) / 10.0).floor() as i32;
        let mut cross = None;
        for m in 1..=90 {
            let b = natal.birth.add_ns(m as i128 * 60 * NS_PER_SEC);
            let (_, a) = view_at(&spk, &obs, b);
            if (a.rem_euclid(30.0) / 10.0).floor() as i32 != d0 {
                cross = Some(m);
                break;
            }
        }
        let m = cross.expect("a decanate boundary within ninety minutes");
        let before = natal.birth.add_ns((m as i128 - 2) * 60 * NS_PER_SEC);
        let after = natal.birth.add_ns((m as i128 + 2) * 60 * NS_PER_SEC);
        let (nb, ab) = view_at(&spk, &obs, before);
        let (na, aa) = view_at(&spk, &obs, after);
        let mb = marks(&View::new(&nb, ab, obs.longitude_deg));
        let ma = marks(&View::new(&na, aa, obs.longitude_deg));
        let top = mb.iter().find(|x| x.3 >= 0.7).copied().expect("a mark before the boundary");
        let claim = Claim::Mark { decanate: top.0, part: top.1, side: top.2 };
        let sb = support(&claim, &View::new(&nb, ab, obs.longitude_deg), at);
        let sa = support(&claim, &View::new(&na, aa, obs.longitude_deg), at);
        eprintln!("boundary at +{m} min: before {mb:?} ({sb:.2}), after {ma:?} ({sa:.2})");
        assert!(sb > 0.5 && sa < 0.0, "the claim flips across the boundary: {sb} → {sa}");
    }

    #[test]
    fn consistent_fact_answers_narrow_the_birth_time() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        // A person truly born 20 minutes later answers every sensitive fact as their own
        // chart says it.
        let truth = natal.birth.add_ns(20 * 60 * NS_PER_SEC);
        let (nt, atru) = view_at(&spk, &obs, truth);
        let vt = View::new(&nt, atru, obs.longitude_deg);
        let mut claims: Vec<Claim> = every_mark();
        claims.extend([Claim::FirstBorn(true), Claim::ManySiblings(true)]);
        for g in &LORDS[..7] {
            claims.push(Claim::Mother(*g));
            claims.push(Claim::Father(*g));
        }
        let answers: Vec<FactAnswer> = claims.iter().map(|c| FactAnswer { claim: *c, yes: support(c, &vt, at) > 0.0 }).collect();
        let r = refine_birth_pooled(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, 60.0, 2.0, &[], &answers, &[], at, false);
        let mean: f64 = r.refine.fits.iter().map(|f| f.prob * f.shift_min).sum();
        let (lo, hi) = r.hpd.expect("an interval");
        let width = hi.ns_since(lo) as f64 / 60.0 / NS_PER_SEC as f64;
        eprintln!("facts used {} (flat {}), best {:+.0}, mean {mean:+.1}, hpd width {width:.0} min in {} runs, log10 BF {:.2}", r.facts, r.facts_flat, r.refine.fits[0].shift_min, r.hpd_runs, r.refine.log10_bayes);
        assert!(r.facts > 0);
        assert!(mean > 5.0, "belief should move toward +20: {mean:+.1}");
        assert!(width < 120.0, "the interval narrows from the two-hour window: {width}");
        assert!(r.refine.log10_bayes > 0.0);
        let total: f64 = r.refine.fits.iter().map(|f| f.prob).sum();
        assert!((total - 1.0).abs() < 1e-6);
        let words = birth_now_words(Lang::En, &r, natal.birth, 330);
        eprintln!("{words}");
        assert_eq!(jargon_in(Lang::En, &words), None);
    }

    #[test]
    fn logged_events_move_the_belief() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        // Events as the chart born 25 minutes later marks them most: its strongest
        // stretches, logged as having happened.
        let truth = natal.birth.add_ns(25 * 60 * NS_PER_SEC);
        let nt = Natal::compute(&spk, None, truth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let qs = crate::reveal::past_reveal(&spk, None, &nt, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Lang::En, at, 0.0, &[], 8, false).unwrap();
        let logged: Vec<Logged> = qs.iter().map(|q| Logged { at: q.from.add_ns(q.to.ns_since(q.from) / 2), life: q.life, good: q.good }).collect();
        assert!(!logged.is_empty());
        let none = refine_birth_pooled(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, 60.0, 2.0, &[], &[], &[], at, false);
        assert!(none.refine.fits.is_empty());
        let r = refine_birth_pooled(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, natal.birth, 60.0, 2.0, &[], &[], &logged, at, false);
        let mean: f64 = r.refine.fits.iter().map(|f| f.prob * f.shift_min).sum();
        let at_zero = r.refine.fits.iter().find(|f| f.shift_min.abs() < 1e-9).map(|f| f.prob).unwrap_or(0.0);
        let flat = 1.0 / r.refine.fits.len() as f64;
        eprintln!("{} logged: best {:+.0}, mean {mean:+.1}, log10 BF {:.2}", r.logged, r.refine.fits[0].shift_min, r.refine.log10_bayes);
        assert_eq!(r.logged, logged.len());
        assert!(mean > 0.0, "logged events pull toward the chart that marks them: {mean:+.1}");
        assert!((r.refine.fits[0].prob - at_zero).abs() > 1e-6 || (at_zero - flat).abs() > 1e-6, "the events moved the belief");
    }
}
