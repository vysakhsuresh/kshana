//! The consensus gate: dated windows where the independent ways of reading a chart
//! agree, said more specifically the more of them agree.
//!
//! # The witnesses
//!
//! Nine systems read each life at each instant (see [`System`]): the Vimshottari,
//! Yogini, Chara and Kalachakra periods, the KP cusp chain and the divisional chart (all
//! as [`crate::witness`] reads them for the birth-time rectification), the slow transits
//! from the Moon and the rising sign ([`crate::forecast`]'s gochara and bhava readings
//! without the Moon), the Tajika year (the Mudda periods of the year from one solar
//! return to the next and the lord of the return's rising sign, cast where the person
//! now lives), and the Nadi passages of Jupiter, Saturn and Rahu over the exact degrees
//! of the birth chart ([`crate::nadi`]). The forecast's slow score is left out: it is a
//! blend of the periods and the transits already counted, and counting it would count
//! them twice.
//!
//! # Agreement, as a measure
//!
//! Each system's reading of a life is sampled every three days from two years before
//! the start of the span to its end. That series is the life's own baseline: each
//! reading becomes a z-score against it. The readings are step functions (a period lord
//! holds for months, a Nadi passage is silent most of the time), so their spread is far
//! from normal and a mean-and-deviation score is lopsided: in trials on five charts it
//! called about three hard windows for every two good ones, and for one chart twenty
//! strong hard windows to no strong good one. The score is therefore the rank-based
//! normal score, `Φ⁻¹((below + ½ · equal + ½) / (n + 1))` (van der Waerden 1952, mid-ranks
//! for ties), which is symmetric over the baseline by construction; it is then bounded
//! softly as `B · tanh(z / B)` with `B = 1.5` (a bounded-influence score, Huber 1964, as
//! in [`crate::witness::Reading::zs`]). A system *leans* one way when its score is at
//! least [`LEAN`] = 0.5.
//!
//! The pooled lean is Stouffer's weighted Z with the systems' own correlation taken into
//! account: `Z = Σ wᵢ zᵢ / √(wᵀ C w)`, where `C` is the covariance of the bounded
//! distances over the baseline (Strube 1985, *Psychol. Bull.* 97, 334; Hartung 1999,
//! *Biometrical J.* 41, 849). The systems share the same Moon and the same lords, so they
//! are not independent; dividing by the pooled spread they actually show makes `Z` a
//! unit-variance score over the life's own baseline whatever their overlap. The weights
//! `wᵢ` ([`System::weight`]) are the rectification's, with the Vimshottari periods in the
//! slow score's place; the KP weight is scaled by the cusp's promise.
//!
//! # Tiers
//!
//! At each instant, with `d` the sign of `Z`, `agree` the systems leaning with `d`,
//! `against` those leaning the other way and `of` those with a reading:
//!
//! | tier | needs |
//! |------|-------|
//! | strong | `agree ≥ 5`, `agree ≥ 0.6 · of`, `against ≤ 1`, `|Z| ≥ 1.645` |
//! | likely | `agree ≥ 4`, `agree ≥ against + 3`, `|Z| ≥ 1.15` |
//! | possible | `agree ≥ 3`, `agree > against`, `|Z| ≥ 0.674` |
//!
//! Why these numbers. If the systems carried no signal and were independent, each
//! distance would be standard normal and would lean a given way with probability
//! `P(z ≥ 0.5) = 0.309`; five or more of eight leaning the same stated way would then
//! happen with probability 0.066, four or more with 0.21 ([`binomial_tail`] checks the
//! arithmetic). The count alone is therefore not enough, because the systems are
//! correlated and a count can be reached by chance a few times in two years. The pooled
//! `|Z|` thresholds are the two-sided 10 %, 25 % and 50 % points of the normal: a strong
//! window must sit in the furthest tenth of the life's own baseline *and* carry the count.
//! The test `strong_is_rare_over_the_baseline` holds this: strong instants are at most
//! a tenth of the baseline. A strong tier must also hold for two samples running (six
//! days), so a single spike is never called strong.
//!
//! # Words
//!
//! The more the systems agree, the more is said (see [`SureWindow::headline`]):
//! possible names the part of life and the month; likely adds the kind of event and the
//! dates; strong adds who is involved and the exact days. Only strong windows are said
//! firmly ("very likely", "the signs agree strongly"); nothing is ever said to be
//! certain. A hard window always carries humane words, when it eases, and a remedy.
//!
//! The kind of event and the person involved come from the houses the triggering lords
//! rule and occupy (whole signs from the rising sign): the running Vimshottari lords and
//! the Mudda lords, weighted by whether their system agrees, the natal planet a Nadi
//! passage falls on, and the houses Jupiter and Saturn are transiting. A good kind is
//! read from the life's own houses, a hard kind from the 6th, 8th and 12th and the life's
//! main house; the person from the heaviest other house, or from the natal planet a Nadi
//! passage falls on. The tables:
//!
//! | house | kind (good) | kind (hard) | who |
//! |------:|-------------|-------------|-----|
//! | 1 | a fresh start | tiredness, own health | you yourself |
//! | 2 | savings, family money, a promise kept | family money, harsh words | family at home |
//! | 3 | short trip, papers, a bold step | a tiring effort, papers | brother or sister, neighbour |
//! | 4 | home, land, vehicle | home, comfort, vehicle | mother, someone at home |
//! | 5 | a child, studies, a happy idea | worry over a child, a risky bet | a child, someone younger |
//! | 6 | a job won, a loan cleared, a rival beaten | dispute, debt, strain at work | colleague, someone you serve |
//! | 7 | a partner, an agreement | friction with a partner | partner, business partner |
//! | 8 | something sudden: inheritance, insurance | something sudden or hidden | the partner's family |
//! | 9 | luck, a blessing, a long journey | a plan that stalls, an elder | father, teacher, older family member |
//! | 10 | work and standing | pressure at work | boss, someone senior |
//! | 11 | a gain, a wish fulfilled, friends' help | a gain delayed | friend, elder sibling |
//! | 12 | travel abroad, a good expense, rest | expense, loss, poor sleep | someone far away or abroad |
//!
//! Sources for the house significations: Brihat Parashara Hora Shastra, the bhava
//! chapters (Santhanam's edition ch. 11–23, what each house signifies); Phaladeepika
//! ch. 1 (the house karakas: Sun for the father and the 9th, Moon the mother and the 4th,
//! Mars the siblings and the 3rd, Jupiter the children and the 5th, Venus the spouse and
//! the 7th); Uttara Kalamrita, Khanda 5 (the long lists of what each house holds: the 8th
//! inheritance and hidden things, the 12th expense and foreign lands, the 11th gains and
//! elder siblings). When a Nadi passage falls on a natal planet, the person is that
//! planet's relative (Brihat Jataka ch. 2 and Phaladeepika ch. 2 on the relatives each
//! graha stands for: Sun the father, Moon the mother, Mars a brother, Mercury a younger
//! relative or friend, Jupiter a teacher or child, Venus the spouse, Saturn an elder or
//! servants, Rahu a foreigner, Ketu a spiritual guide), because the Nadi reading takes the
//! natal planet as the person the event comes through.
//!
//! # Hours
//!
//! [`narrow`] finds, inside a window, the days when the Moon crosses the triggering
//! natal degrees and the days of the exact Nadi passages, and, for the best day, the
//! hours when the triggering planet's hora runs while the rising sign at the person's
//! present place stands in the triggering sign (the Prasna and Muhurta practice of
//! timing the event to the hour by the Moon, the hora lord and the lagna; Prasna Marga
//! ch. 15, Muhurta Chintamani on hora). Hours are given only when both agree on a block
//! of at least twenty minutes; otherwise days alone.

use crate::forecast::{bhava_component, gochara_items, lord_score, sky, ForecastError, Life, Natal, GRAHAS};
use crate::lang;
use crate::nadi::{self, NadiHit, NadiPoint, NadiTable};
use crate::parihara::parihara_from_facts;
use crate::plain::Lang;
use crate::remedy_timing::hora_lord_of;
use crate::reveal::par_map;
use crate::transit::returns;
use crate::witness::{ascendant, Reading, Witness};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_events::{hora_at, solar_day, solar_return, MICROSECOND_NS};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{ayanamsa_degrees, nakshatra, sign_lord, subdivide, vimshottari_levels, Ayanamsa, DashaLord, DashaPeriod};

/// Number of systems.
pub const NS: usize = 9;

/// A system leans when its distance from its own average is at least this many
/// standard deviations.
pub const LEAN: f64 = 0.5;

/// Soft bound on a distance, standard deviations.
pub const BOUND: f64 = 1.5;

/// Pooled-lean thresholds: the two-sided 10 %, 25 % and 50 % points of the normal.
pub const Z_STRONG: f64 = 1.645;
/// See [`Z_STRONG`].
pub const Z_LIKELY: f64 = 1.15;
/// See [`Z_STRONG`].
pub const Z_POSSIBLE: f64 = 0.674;

const MONTH_NS: i128 = (30.4375 * NS_PER_DAY as f64) as i128;
const GRID_NS: i128 = 3 * NS_PER_DAY;
const BASE_BACK_MONTHS: i128 = 24;
/// Edges are bisected to this.
const EDGE_NS: i128 = 2 * 60 * NS_PER_SEC;

/// One way of reading the chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum System {
    /// Vimshottari periods, three levels.
    Vimshottari,
    /// Yogini periods.
    Yogini,
    /// Chara (Jaimini) sign periods.
    Chara,
    /// Kalachakra sign periods.
    Kalachakra,
    /// KP cusp chain and timing funnel.
    Kp,
    /// The matter's divisional chart.
    Varga,
    /// Slow transits from the Moon and the rising sign.
    Transit,
    /// The Tajika year: Mudda periods and the year lord.
    Annual,
    /// Nadi passages over natal degrees and the Bhrigu Bindu.
    Nadi,
}

/// Every system, in index order.
pub const SYSTEMS: [System; NS] =
    [System::Vimshottari, System::Yogini, System::Chara, System::Kalachakra, System::Kp, System::Varga, System::Transit, System::Annual, System::Nadi];

impl System {
    /// Position in [`SYSTEMS`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            System::Vimshottari => "vimshottari",
            System::Yogini => "yogini",
            System::Chara => "chara",
            System::Kalachakra => "kalachakra",
            System::Kp => "kp",
            System::Varga => "varga",
            System::Transit => "gochara",
            System::Annual => "tajika",
            System::Nadi => "nadi",
        }
    }

    /// Parse a [`System::key`].
    pub fn parse(key: &str) -> Option<System> {
        SYSTEMS.iter().copied().find(|s| s.key() == key)
    }

    /// The system in an astrologer's words (expert view only).
    pub fn expert(self) -> &'static str {
        match self {
            System::Vimshottari => "Vimshottari dasha (maha, antar, pratyantar)",
            System::Yogini => "Yogini dasha",
            System::Chara => "Jaimini Chara dasha",
            System::Kalachakra => "Kalachakra dasha",
            System::Kp => "KP cusp sub-lord chain",
            System::Varga => "the matter's varga (D9/D10/D12/D7)",
            System::Transit => "Gochara from the Moon and the lagna",
            System::Annual => "Tajika varshaphala: Mudda dasha and varshesha",
            System::Nadi => "Bhrigu Nadi transits and the Bhrigu Bindu",
        }
    }

    /// The system in plain words.
    pub fn name(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            System::Vimshottari => ["the long-period reading", "നീണ്ട കാലഘട്ടങ്ങളുടെ വായന", "लंबे दौरों वाली पढ़त"],
            System::Yogini => ["the short-cycle reading", "ചെറിയ ചക്രങ്ങളുടെ വായന", "छोटे चक्रों वाली पढ़त"],
            System::Chara => ["the life-chapter reading", "ജീവിത അധ്യായങ്ങളുടെ വായന", "जीवन के अध्यायों वाली पढ़त"],
            System::Kalachakra => ["the wheel-of-time reading", "സമയചക്രത്തിന്റെ വായന", "समय-चक्र वाली पढ़त"],
            System::Kp => ["the fine-timing reading", "സൂക്ഷ്മ സമയ വായന", "बारीक समय वाली पढ़त"],
            System::Varga => ["the close-up reading for this part of life", "ഈ ജീവിതമേഖലയുടെ അടുത്തുനിന്നുള്ള വായന", "जीवन के इस हिस्से की बारीक पढ़त"],
            System::Transit => ["the sky right now", "ഇപ്പോഴത്തെ ആകാശം", "इस समय का आकाश"],
            System::Annual => ["your birthday-to-birthday year", "പിറന്നാൾ മുതൽ പിറന്നാൾ വരെയുള്ള വർഷം", "जन्मदिन से जन्मदिन तक का साल"],
            System::Nadi => ["the sensitive points of your birth", "ജനനസമയത്തെ സൂക്ഷ്മ ബിന്ദുക്കൾ", "जन्म के संवेदनशील बिंदु"],
        })
    }

    /// Weight in the pooled lean. The rectification's weights ([`Witness::weight`]),
    /// with Vimshottari at 1.0 in the place of the slow score left out here; the slow
    /// transits 0.6 (in the slow score they are 35 of its 80 parts, close to half the
    /// unit, raised a little because here they stand alone); the Tajika year 0.3, like
    /// Yogini (a compressed Vimshottari, two levels, read through the same lords); the
    /// Nadi passages 0.4 (silent most of the time, but the most specific when they speak).
    pub fn weight(self) -> f64 {
        match self {
            System::Vimshottari => 1.0,
            System::Yogini => Witness::Yogini.weight(),
            System::Chara => Witness::Chara.weight(),
            System::Kalachakra => Witness::Kalachakra.weight(),
            System::Kp => Witness::Kp.weight(),
            System::Varga => Witness::Varga.weight(),
            System::Transit => 0.6,
            System::Annual => 0.3,
            System::Nadi => 0.4,
        }
    }

    fn witness(self) -> Option<Witness> {
        Some(match self {
            System::Vimshottari => Witness::Vimshottari,
            System::Yogini => Witness::Yogini,
            System::Chara => Witness::Chara,
            System::Kalachakra => Witness::Kalachakra,
            System::Kp => Witness::Kp,
            System::Varga => Witness::Varga,
            _ => return None,
        })
    }
}

/// How firmly a window is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Some signs point here: the part of life and the month only.
    Possible,
    /// Most signs agree: the kind of event and the dates.
    Likely,
    /// The signs agree strongly: who, what and the exact days.
    Strong,
}

impl Tier {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Tier::Possible => "possible",
            Tier::Likely => "likely",
            Tier::Strong => "strong",
        }
    }

    /// The tier in words.
    pub fn words(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Tier::Strong => ["The signs agree strongly: very likely.", "സൂചനകൾ ശക്തമായി യോജിക്കുന്നു: വളരെ സാധ്യത.", "संकेत मज़बूती से सहमत हैं: बहुत संभावना है।"],
            Tier::Likely => ["Most signs agree: likely.", "മിക്ക സൂചനകളും യോജിക്കുന്നു: സാധ്യതയുണ്ട്.", "ज़्यादातर संकेत सहमत हैं: संभावना है।"],
            Tier::Possible => ["Only some signs point here: possible.", "ചില സൂചനകൾ മാത്രം ഇങ്ങോട്ട്: സാധ്യമാണ്.", "केवल कुछ संकेत इस ओर हैं: संभव है।"],
        })
    }
}

/// The systems' reading of one life at one instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lean {
    /// Each system's bounded distance from its own average; None without a reading.
    pub zs: [Option<f64>; NS],
    /// The pooled lean, a unit-variance score over the baseline.
    pub z: f64,
    /// Systems leaning with the pooled lean.
    pub agree: usize,
    /// Systems leaning against it.
    pub against: usize,
    /// Systems with a reading.
    pub of: usize,
}

impl Lean {
    /// True when the pooled lean is favourable.
    pub fn good(&self) -> bool {
        self.z >= 0.0
    }

    /// The systems leaning with the pooled lean.
    pub fn agreeing(&self) -> Vec<System> {
        let d = if self.good() { 1.0 } else { -1.0 };
        SYSTEMS.iter().copied().filter(|s| self.zs[s.index()].map_or(false, |z| z * d >= LEAN)).collect()
    }
}

/// The tier a lean reaches, if any. Monotone: more agreeing systems, fewer against or a
/// larger pooled lean never lower it.
pub fn classify(agree: usize, against: usize, of: usize, z: f64) -> Option<Tier> {
    let a = z.abs();
    if agree >= 5 && agree as f64 >= 0.6 * of as f64 && against <= 1 && a >= Z_STRONG {
        Some(Tier::Strong)
    } else if agree >= 4 && agree >= against + 3 && a >= Z_LIKELY {
        Some(Tier::Likely)
    } else if agree >= 3 && agree > against && a >= Z_POSSIBLE {
        Some(Tier::Possible)
    } else {
        None
    }
}

/// `P(X ≥ k)` for `X ~ Binomial(n, p)`, for the tier arithmetic in the module notes.
pub fn binomial_tail(n: usize, k: usize, p: f64) -> f64 {
    let choose = |n: usize, r: usize| -> f64 { (0..r).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64) };
    (k..=n).map(|r| choose(n, r) * p.powi(r as i32) * (1.0 - p).powi((n - r) as i32)).sum()
}

fn li_of(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

fn gi(g: DashaLord) -> usize {
    GRAHAS.iter().position(|x| *x == g).unwrap_or(0)
}

/// One Tajika year.
#[derive(Clone, Debug)]
struct Year {
    start: Instant,
    end: Instant,
    /// The lord of the return's rising sign, cast where the person lives now.
    lord: DashaLord,
    /// Mudda periods.
    mudda: Vec<DashaPeriod>,
}

/// Mudda periods of a year (K. S. Charak, *A Textbook of Varshaphala*, ch. V; the rule as
/// `kshana-tajika`'s `mudda` implements it, repeated here because that crate builds on
/// this one): the natal Moon's lord advanced one lord per completed year, its share
/// reduced by the natal balance, the elapsed remainder closing the year.
fn mudda(natal_moon: f64, start: Instant, end: Instant, completed: u32) -> Vec<DashaPeriod> {
    let total = end.ns_since(start);
    let n = nakshatra(natal_moon);
    let first = DashaLord::ORDER[(n.index + completed as usize) % 9];
    let fi = DashaLord::ORDER.iter().position(|l| *l == first).unwrap_or(0);
    let mut shares: Vec<(DashaLord, f64)> = vec![(first, first.years() as f64 * (1.0 - n.fraction))];
    for k in 1..9 {
        let l = DashaLord::ORDER[(fi + k) % 9];
        shares.push((l, l.years() as f64));
    }
    if n.fraction > 0.0 {
        shares.push((first, first.years() as f64 * n.fraction));
    }
    let len = shares.len();
    let mut out = Vec::with_capacity(len);
    let (mut t, mut cum) = (start, 0.0);
    for (i, (lord, share)) in shares.into_iter().enumerate() {
        cum += share;
        let e = if i + 1 == len { end } else { start.add_ns((total as f64 * cum / 120.0).round() as i128) };
        out.push(DashaPeriod { lord, start: t, end: e });
        t = e;
    }
    out
}

type Vals = [[f64; 9]; NS];

/// A person's chart prepared for reading the systems at any instant.
pub struct Consensus<'a> {
    spk: &'a Spk,
    eop: Option<&'a EopTable>,
    natal: &'a Natal,
    obs: Observer,
    here: Observer,
    ayanamsa: Ayanamsa,
    reading: Reading<'a>,
    lord_tab: [[f64; 9]; 9],
    years: Vec<Year>,
    nadi: NadiTable,
    hits: Vec<NadiHit>,
    /// The person's own multiplier on each system's say, earned from their sealed
    /// outcomes ([`crate::integrity::consensus_multipliers`]); all 1 by default.
    mult: [f64; NS],
}

impl<'a> Consensus<'a> {
    /// Prepare `natal` (born where `obs` stands; living now where `here` stands) for
    /// readings in `[from, to]`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(spk: &'a Spk, eop: Option<&'a EopTable>, natal: &'a Natal, obs: &Observer, here: &Observer, ayanamsa: Ayanamsa, from: Instant, to: Instant) -> Self {
        let asc = ascendant(spk, eop, natal.birth, obs, ayanamsa).map(|a| a.0).unwrap_or(natal.facts.cusps[0]);
        let lord_tab = std::array::from_fn(|li| std::array::from_fn(|g| lord_score(natal, Life::ALL[li], GRAHAS[g]).0));
        // The Tajika years covering the span, cast at the present place.
        let sun_sid = natal.facts.planet(DashaLord::Sun).sidereal;
        let sun_trop = (sun_sid + ayanamsa_degrees(ayanamsa, natal.birth.tt_centuries_since_j2000())).rem_euclid(360.0);
        let (y0, y1) = (Utc::from_instant(from).year - 1, Utc::from_instant(to).year + 1);
        let rets: Vec<Instant> = (y0..=y1).filter_map(|y| solar_return(spk, sun_trop, natal.birth, y, MICROSECOND_NS).ok().flatten()).filter(|r| *r > natal.birth).collect();
        let years = rets
            .windows(2)
            .map(|w| {
                let completed = (w[0].ns_since(natal.birth) as f64 / (365.25 * NS_PER_DAY as f64)).round().max(0.0) as u32;
                let lord = compute(spk, eop, &ChartInput { instant: w[0], observer: *here, system: HouseSystem::WholeSign, ayanamsa, topocentric_moon: false })
                    .map(|ch| sign_lord(((ch.cusps_sidereal[0] / 30.0).floor() as usize) % 12))
                    .unwrap_or(DashaLord::Sun);
                Year { start: w[0], end: w[1], lord, mudda: mudda(natal.moon_sidereal, w[0], w[1], completed) }
            })
            .collect();
        let hits = nadi::hits(spk, ayanamsa, natal, from.add_ns(-90 * NS_PER_DAY), to.add_ns(90 * NS_PER_DAY));
        Consensus { spk, eop, natal, obs: *obs, here: *here, ayanamsa, reading: Reading::new(natal, asc), lord_tab, years, nadi: NadiTable::new(natal), hits, mult: [1.0; NS] }
    }

    /// Every system's reading of every life at `t`, `[system][life]`; NaN where a system
    /// has nothing to say.
    fn eval(&self, t: Instant) -> Option<Vals> {
        let s = sky(self.spk, self.eop, self.natal, &self.obs, HouseSystem::WholeSign, self.ayanamsa, t).ok()?;
        let w = self.reading.values(&s, t);
        let mut out = [[f64::NAN; 9]; NS];
        for sys in SYSTEMS {
            if let Some(wi) = sys.witness() {
                out[sys.index()] = w[wi.index()];
            }
        }
        let signs = s.signs();
        let year = self.years.iter().find(|y| y.start <= t && t < y.end);
        let mudda = year.and_then(|y| {
            let m = y.mudda.iter().find(|p| p.start <= t && t < p.end)?;
            let a = subdivide(*m).into_iter().find(|p| p.start <= t && t < p.end);
            Some((m.lord, a.map(|p| p.lord)))
        });
        let movers = [DashaLord::Jupiter, DashaLord::Saturn, DashaLord::Rahu].map(|g| s.facts.planet(g).sidereal);
        let nadi = self.nadi.values(movers);
        for (li, &life) in Life::ALL.iter().enumerate() {
            // The slow transits, as the slow score reads them, without the Moon.
            let (items, den) = gochara_items(self.natal, &s.from_moon, &signs, life);
            let den = den - items[1].relevance;
            let num: f64 = items.iter().map(|i| i.contrib).sum::<f64>() - items[1].contrib;
            let g = (0.5 + 0.5 * if den > 0.0 { num / den } else { 0.0 }).clamp(0.0, 1.0);
            let b = bhava_component(self.natal, &signs, life, false).score;
            out[System::Transit.index()][li] = 0.5 * g + 0.5 * b;
            if let (Some(y), Some((m, a))) = (year, mudda) {
                let tab = &self.lord_tab[li];
                let periods = match a {
                    Some(a) => (0.35 * tab[gi(m)] + 0.30 * tab[gi(a)]) / 0.65,
                    None => tab[gi(m)],
                };
                out[System::Annual.index()][li] = 0.7 * periods + 0.3 * tab[gi(y.lord)];
            }
            out[System::Nadi.index()][li] = nadi[li];
        }
        Some(out)
    }

    fn weight(&self, s: System, life: Life) -> f64 {
        self.mult[s.index()] * s.weight() * if s == System::Kp { self.reading.conf(Witness::Kp.index(), life) } else { 1.0 }
    }
}

/// Each system's baseline per life (its sorted readings), and the pooled spread.
struct Base {
    /// `[system][life]`: the baseline readings, sorted; empty when the system does not
    /// vary for the life.
    cols: Vec<Vec<Vec<f64>>>,
    norm: [f64; 9],
    w: [[f64; NS]; 9],
}

fn bound(z: f64) -> f64 {
    BOUND * (z / BOUND).tanh()
}

/// The inverse of the standard normal distribution, `Φ⁻¹(p)`, by Acklam's rational
/// approximation (P. J. Acklam 2003; relative error under 1.2 × 10⁻⁹, far finer than
/// a rank needs).
pub fn probit(p: f64) -> f64 {
    const A: [f64; 6] = [-3.969683028665376e1, 2.209460984245205e2, -2.759285104469687e2, 1.383577518672690e2, -3.066479806614716e1, 2.506628277459239];
    const B: [f64; 5] = [-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2, 6.680131188771972e1, -1.328068155288572e1];
    const C: [f64; 6] = [-7.784894002430293e-3, -3.223964580411365e-1, -2.400758277161838, -2.549732539343734, 4.374664141464968, 2.938163982698783];
    const D: [f64; 4] = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416];
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    let lo = 0.02425;
    if p < lo {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - lo {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        -probit(1.0 - p)
    }
}

impl Base {
    fn new(c: &Consensus, vals: &[Vals]) -> Self {
        let cols: Vec<Vec<Vec<f64>>> = (0..NS)
            .map(|s| {
                (0..9)
                    .map(|li| {
                        let mut col: Vec<f64> = vals.iter().map(|v| v[s][li]).filter(|x| x.is_finite()).collect();
                        col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let varies = col.len() >= 3 && col[col.len() - 1] - col[0] > 1e-6;
                        if varies {
                            col
                        } else {
                            Vec::new()
                        }
                    })
                    .collect()
            })
            .collect();
        let w: [[f64; NS]; 9] = std::array::from_fn(|li| std::array::from_fn(|s| c.weight(SYSTEMS[s], Life::ALL[li])));
        let mut b = Base { cols, norm: [1.0; 9], w };
        for li in 0..9 {
            // The pooled spread √(wᵀ C w) of the bounded distances over the baseline.
            let rows: Vec<[f64; NS]> = vals.iter().map(|v| std::array::from_fn(|s| b.z(v, s, li).unwrap_or(0.0))).collect();
            let n = rows.len().max(1) as f64;
            let mu: [f64; NS] = std::array::from_fn(|s| rows.iter().map(|r| r[s]).sum::<f64>() / n);
            let mut var = 0.0;
            for i in 0..NS {
                for j in 0..NS {
                    let cov = rows.iter().map(|r| (r[i] - mu[i]) * (r[j] - mu[j])).sum::<f64>() / n;
                    var += b.w[li][i] * b.w[li][j] * cov;
                }
            }
            b.norm[li] = var.sqrt().max(1e-6);
        }
        b
    }

    /// The reading's normal score against the baseline: `Φ⁻¹((below + ½ · equal + ½) /
    /// (n + 1))` (van der Waerden's mid-rank normal scores), softly bounded.
    fn z(&self, v: &Vals, s: usize, li: usize) -> Option<f64> {
        let x = v[s][li];
        let col = &self.cols[s][li];
        if col.is_empty() || !x.is_finite() {
            return None;
        }
        let eps = 1e-12;
        let below = col.partition_point(|y| *y < x - eps);
        let upto = col.partition_point(|y| *y <= x + eps);
        let p = (below as f64 + 0.5 * (upto - below) as f64 + 0.5) / (col.len() as f64 + 1.0);
        Some(bound(probit(p)))
    }

    fn lean(&self, v: &Vals, li: usize) -> Lean {
        let zs: [Option<f64>; NS] = std::array::from_fn(|s| self.z(v, s, li));
        let num: f64 = (0..NS).map(|s| self.w[li][s] * zs[s].unwrap_or(0.0)).sum();
        let z = num / self.norm[li];
        let d = if z >= 0.0 { 1.0 } else { -1.0 };
        let agree = zs.iter().flatten().filter(|x| *x * d >= LEAN).count();
        let against = zs.iter().flatten().filter(|x| *x * d <= -LEAN).count();
        let of = zs.iter().flatten().count();
        Lean { zs, z, agree, against, of }
    }
}

fn tier_of(l: &Lean) -> Option<Tier> {
    classify(l.agree, l.against, l.of, l.z)
}

/// One day inside a window that stands out, with its hours when they are sharp.
#[derive(Clone, Debug, PartialEq)]
pub struct BestDay {
    /// Local midnight that starts the day.
    pub start: Instant,
    /// The next local midnight.
    pub end: Instant,
    /// "Mon 16 Nov".
    pub label: String,
    /// The likely hours: from, to, and "morning (about 9–11 am)".
    pub hours: Option<(Instant, Instant, String)>,
}

/// What a window is narrowed to.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Narrowing {
    /// The days, in date order.
    pub days: Vec<BestDay>,
    /// "Most likely Mon 16 Nov, morning (about 9–11 am)." or "Most likely on 16 or 18
    /// Nov."; empty when nothing stands out.
    pub line: String,
    /// The hours line alone, when sharp.
    pub hours_line: Option<String>,
}

/// One window where the systems agree.
#[derive(Clone, Debug, PartialEq)]
pub struct SureWindow {
    /// The part of life.
    pub life: Life,
    /// Other parts of life whose windows share these dates and direction (said once).
    pub also: Vec<Life>,
    /// Start, bisected to two minutes (may be before the span's start when the window
    /// is already running).
    pub start: Instant,
    /// End, bisected to two minutes.
    pub end: Instant,
    /// The instant of the strongest pooled lean.
    pub peak: Instant,
    /// How firmly it is said.
    pub tier: Tier,
    /// Favourable or hard.
    pub good: bool,
    /// Systems agreeing at the peak.
    pub agree: usize,
    /// Systems leaning against at the peak.
    pub against: usize,
    /// Systems with a reading at the peak.
    pub of: usize,
    /// The pooled lean at the peak, a unit-variance score.
    pub z: f64,
    /// `Φ(|z|)` approximated as `logistic(1.6 |z|)`: how far into the life's own range.
    pub strength: f64,
    /// The agreeing systems.
    pub systems: Vec<System>,
    /// The house that sets the kind of event, 1–12.
    pub kind_house: u8,
    /// The house that sets who is involved, 1–12.
    pub who_house: u8,
    /// The natal planet a Nadi passage falls on, when it names the person.
    pub who_karaka: Option<DashaLord>,
    /// The triggering lords, strongest first, with their weight.
    pub triggers: Vec<(DashaLord, f64)>,
    /// The Nadi passages in or around the window bearing on this life.
    pub hits: Vec<NadiHit>,
    /// The kind of event in words (said from likely up).
    pub kind: String,
    /// Who is involved in words (said only when strong).
    pub who: String,
    /// "14–19 Nov 2026".
    pub date_line: String,
    /// The headline.
    pub headline: String,
    /// More words: how sure, through what, why, and for a hard window when it eases.
    pub detail: String,
    /// "6 of 8 ways of reading agree."
    pub sure_line: String,
    /// What helps, for a hard window (always present then).
    pub remedy_hint: Option<String>,
    /// A self-contained statement with both dates, to seal.
    pub seal_text: String,
    /// The best days (and hours, when sharp) inside the window.
    pub narrowing: Narrowing,
    /// The triggers in an astrologer's words (expert view only).
    pub expert: String,
}

// ---------------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------------

const MONTHS_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_EN_FULL: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_ML: [&str; 12] = ["ജനുവരി", "ഫെബ്രുവരി", "മാർച്ച്", "ഏപ്രിൽ", "മേയ്", "ജൂൺ", "ജൂലൈ", "ഓഗസ്റ്റ്", "സെപ്റ്റംബർ", "ഒക്ടോബർ", "നവംബർ", "ഡിസംബർ"];
const MONTHS_HI: [&str; 12] = ["जनवरी", "फ़रवरी", "मार्च", "अप्रैल", "मई", "जून", "जुलाई", "अगस्त", "सितंबर", "अक्टूबर", "नवंबर", "दिसंबर"];

fn local(t: Instant, off_min: i32) -> Utc {
    Utc::from_instant(t.add_ns(off_min as i128 * 60 * NS_PER_SEC))
}

fn mi(u: &Utc) -> usize {
    (u.month as usize).clamp(1, 12) - 1
}

fn full_date(lang: Lang, u: &Utc) -> String {
    let m = mi(u);
    match lang {
        Lang::En => format!("{} {} {}", u.day, MONTHS_EN[m], u.year),
        Lang::Ml => format!("{} {} {}", u.year, MONTHS_ML[m], u.day),
        Lang::Hi => format!("{} {} {}", u.day, MONTHS_HI[m], u.year),
    }
}

fn month_words(lang: Lang, u: &Utc) -> String {
    let m = mi(u);
    match lang {
        Lang::En => format!("{} {}", MONTHS_EN_FULL[m], u.year),
        Lang::Ml => format!("{} {}", u.year, MONTHS_ML[m]),
        Lang::Hi => format!("{} {}", MONTHS_HI[m], u.year),
    }
}

/// The last instant inside a window, so an end at local midnight names the day before.
fn last_day(end: Instant) -> Instant {
    end.add_ns(-NS_PER_SEC)
}

/// "Between 14 and 19 Nov 2026", "From now until 19 Nov 2026", "On 14 Nov 2026".
fn between(lang: Lang, start: Instant, end: Instant, now: Instant, off: i32) -> String {
    let b = local(last_day(end), off);
    if start <= now {
        return match lang {
            Lang::En => format!("From now until {}", full_date(lang, &b)),
            Lang::Ml => format!("ഇപ്പോൾ മുതൽ {} വരെ", full_date(lang, &b)),
            Lang::Hi => format!("अभी से {} तक", full_date(lang, &b)),
        };
    }
    let a = local(start, off);
    let same_day = (a.year, a.month, a.day) == (b.year, b.month, b.day);
    match lang {
        Lang::En if same_day => format!("On {}", full_date(lang, &a)),
        Lang::En if (a.year, a.month) == (b.year, b.month) => format!("Between {} and {} {} {}", a.day, b.day, MONTHS_EN[mi(&b)], b.year),
        Lang::En if a.year == b.year => format!("Between {} {} and {} {} {}", a.day, MONTHS_EN[mi(&a)], b.day, MONTHS_EN[mi(&b)], b.year),
        Lang::En => format!("Between {} and {}", full_date(lang, &a), full_date(lang, &b)),
        Lang::Ml if same_day => format!("{}-ന്", full_date(lang, &a)),
        Lang::Ml => format!("{} മുതൽ {} വരെ", full_date(lang, &a), full_date(lang, &b)),
        Lang::Hi if same_day => format!("{} को", full_date(lang, &a)),
        Lang::Hi => format!("{} से {} के बीच", full_date(lang, &a), full_date(lang, &b)),
    }
}

/// "14–19 Nov 2026", the big date on the card.
fn date_line(lang: Lang, start: Instant, end: Instant, now: Instant, off: i32, tier: Tier, peak: Instant) -> String {
    if tier == Tier::Possible {
        return month_words(lang, &local(peak, off));
    }
    let b = local(last_day(end), off);
    if start <= now {
        return match lang {
            Lang::En => format!("Now – {}", full_date(lang, &b)),
            Lang::Ml => format!("ഇപ്പോൾ – {}", full_date(lang, &b)),
            Lang::Hi => format!("अभी – {}", full_date(lang, &b)),
        };
    }
    let a = local(start, off);
    if (a.year, a.month, a.day) == (b.year, b.month, b.day) {
        return full_date(lang, &a);
    }
    match lang {
        Lang::En if (a.year, a.month) == (b.year, b.month) => format!("{}–{} {} {}", a.day, b.day, MONTHS_EN[mi(&b)], b.year),
        Lang::En if a.year == b.year => format!("{} {} – {} {} {}", a.day, MONTHS_EN[mi(&a)], b.day, MONTHS_EN[mi(&b)], b.year),
        _ => format!("{} – {}", full_date(lang, &a), full_date(lang, &b)),
    }
}

/// The event of a part of life, good or hard.
pub fn event(lang: Lang, life: Life, good: bool) -> &'static str {
    let t = match (life, good) {
        (Life::Love, true) => ["a warm turn in love", "പ്രണയത്തിൽ ഒരു നല്ല വഴിത്തിരിവ്", "प्रेम में एक अच्छा मोड़"],
        (Life::Love, false) => ["strain in love", "പ്രണയത്തിൽ ഒരു ഉലച്ചിൽ", "प्रेम में खिंचाव"],
        (Life::Career, true) => ["good news at work", "ജോലിയിൽ ഒരു നല്ല വാർത്ത", "काम में अच्छी ख़बर"],
        (Life::Career, false) => ["pressure at work", "ജോലിയിൽ സമ്മർദ്ദം", "काम का दबाव"],
        (Life::Money, true) => ["money news", "പണത്തെക്കുറിച്ച് ഒരു നല്ല വാർത്ത", "पैसे की अच्छी ख़बर"],
        (Life::Money, false) => ["a money strain", "പണത്തിന് ഒരു ഞെരുക്കം", "पैसे की तंगी"],
        (Life::Health, true) => ["a lift in health and energy", "ആരോഗ്യത്തിലും ഉന്മേഷത്തിലും ഉയർച്ച", "सेहत और ऊर्जा में सुधार"],
        (Life::Health, false) => ["a dip in health", "ആരോഗ്യത്തിൽ ഒരു ക്ഷീണം", "सेहत में गिरावट"],
        (Life::Family, true) => ["a happy family matter", "കുടുംബത്തിൽ ഒരു സന്തോഷം", "परिवार में कोई ख़ुशी"],
        (Life::Family, false) => ["a strain in the family", "കുടുംബത്തിൽ ഒരു അസ്വസ്ഥത", "परिवार में तनाव"],
        (Life::Study, true) => ["a step forward in studies", "പഠനത്തിൽ ഒരു മുന്നേറ്റം", "पढ़ाई में एक कदम आगे"],
        (Life::Study, false) => ["studies slowing down", "പഠനം മന്ദഗതിയിലാകുന്നത്", "पढ़ाई का धीमा पड़ना"],
        (Life::Home, true) => ["news about home", "വീടിനെക്കുറിച്ച് ഒരു നല്ല വാർത്ത", "घर से जुड़ी अच्छी ख़बर"],
        (Life::Home, false) => ["trouble at home", "വീട്ടിൽ ഒരു പ്രശ്നം", "घर में परेशानी"],
        (Life::Travel, true) => ["a journey or a move", "ഒരു യാത്രയോ താമസം മാറലോ", "कोई यात्रा या जगह बदलना"],
        (Life::Travel, false) => ["travel hitches", "യാത്രയിൽ തടസ്സങ്ങൾ", "यात्रा में अड़चनें"],
        (Life::Children, true) => ["good news about children", "മക്കളെക്കുറിച്ച് ഒരു നല്ല വാർത്ത", "संतान से जुड़ी अच्छी ख़बर"],
        (Life::Children, false) => ["worry about children", "മക്കളെച്ചൊല്ലി ഒരു ആധി", "संतान को लेकर चिंता"],
    };
    lang.pick(t)
}

/// The kind of event a house brings, good or hard (the table in the module notes).
pub fn kind_words(lang: Lang, house: u8, good: bool) -> &'static str {
    let good_t: [[&str; 3]; 12] = [
        ["a fresh start of your own", "നിങ്ങളുടെ തന്നെ ഒരു പുതിയ തുടക്കം", "आपकी अपनी एक नई शुरुआत"],
        ["savings, family money or a promise kept", "സമ്പാദ്യം, കുടുംബത്തിന്റെ പണം, അല്ലെങ്കിൽ പാലിച്ച ഒരു വാക്ക്", "बचत, परिवार का पैसा या निभाया गया वादा"],
        ["a short trip, papers signed or a bold step of your own", "ഒരു ചെറുയാത്ര, ഒപ്പിട്ട രേഖകൾ, അല്ലെങ്കിൽ നിങ്ങളുടെ ധീരമായ ഒരു ചുവട്", "छोटी यात्रा, काग़ज़ों पर दस्तख़त या आपका कोई साहसी कदम"],
        ["home, land or a vehicle", "വീട്, ഭൂമി അല്ലെങ്കിൽ വാഹനം", "घर, ज़मीन या वाहन"],
        ["a child, studies or a happy idea", "ഒരു കുട്ടി, പഠനം, അല്ലെങ്കിൽ ഒരു നല്ല ആശയം", "बच्चा, पढ़ाई या कोई अच्छा विचार"],
        ["a job won, a loan cleared or a rival beaten", "കിട്ടുന്ന ഒരു ജോലി, തീരുന്ന ഒരു കടം, അല്ലെങ്കിൽ എതിരാളിക്കു മേൽ ജയം", "मिली हुई नौकरी, चुकता क़र्ज़ या विरोधी पर जीत"],
        ["a partner or an agreement", "ഒരു പങ്കാളി അല്ലെങ്കിൽ ഒരു കരാർ", "जीवनसाथी, साझेदार या कोई समझौता"],
        ["something sudden: an inheritance, insurance or a secret that helps", "പെട്ടെന്നുള്ള ഒന്ന്: പൈതൃകസ്വത്ത്, ഇൻഷുറൻസ്, അല്ലെങ്കിൽ സഹായിക്കുന്ന ഒരു രഹസ്യം", "कुछ अचानक: विरासत, बीमा या कोई काम आने वाला राज़"],
        ["luck, a blessing or a long journey", "ഭാഗ്യം, ഒരു അനുഗ്രഹം, അല്ലെങ്കിൽ ഒരു ദീർഘയാത്ര", "भाग्य, कोई आशीर्वाद या लंबी यात्रा"],
        ["work and standing: a role, a rise or recognition", "ജോലിയും നിലയും: ഒരു ചുമതല, ഉയർച്ച, അല്ലെങ്കിൽ അംഗീകാരം", "काम और प्रतिष्ठा: कोई ज़िम्मेदारी, तरक़्क़ी या पहचान"],
        ["a gain, a wish fulfilled or help from friends", "ഒരു നേട്ടം, നിറവേറുന്ന ഒരു ആഗ്രഹം, അല്ലെങ്കിൽ സുഹൃത്തുക്കളുടെ സഹായം", "कोई लाभ, पूरी हुई इच्छा या दोस्तों की मदद"],
        ["travel abroad, a good expense or rest", "വിദേശയാത്ര, നല്ല ഒരു ചെലവ്, അല്ലെങ്കിൽ വിശ്രമം", "विदेश यात्रा, कोई अच्छा ख़र्च या आराम"],
    ];
    let hard_t: [[&str; 3]; 12] = [
        ["tiredness and your own health", "ക്ഷീണവും നിങ്ങളുടെ ആരോഗ്യവും", "थकान और आपकी अपनी सेहत"],
        ["family money or harsh words at home", "കുടുംബത്തിന്റെ പണം, അല്ലെങ്കിൽ വീട്ടിലെ കടുത്ത വാക്കുകൾ", "परिवार का पैसा या घर में कड़वे शब्द"],
        ["a tiring effort, papers or a short trip", "തളർത്തുന്ന ഒരു ശ്രമം, രേഖകൾ, അല്ലെങ്കിൽ ഒരു ചെറുയാത്ര", "थकाने वाली मेहनत, काग़ज़ात या छोटी यात्रा"],
        ["home, comfort or a vehicle", "വീട്, സുഖസൗകര്യം, അല്ലെങ്കിൽ വാഹനം", "घर, सुख-सुविधा या वाहन"],
        ["worry over a child, studies or a risky bet", "ഒരു കുട്ടിയെയോ പഠനത്തെയോ ഒരു സാഹസത്തെയോ ചൊല്ലിയുള്ള ആധി", "बच्चे, पढ़ाई या किसी जोखिम भरे दाँव की चिंता"],
        ["a dispute, a debt or a strain at work", "ഒരു തർക്കം, ഒരു കടം, അല്ലെങ്കിൽ ജോലിയിലെ ഭാരം", "कोई विवाद, क़र्ज़ या काम का बोझ"],
        ["friction with a partner or over an agreement", "പങ്കാളിയുമായോ ഒരു കരാറിനെച്ചൊല്ലിയോ ഉരസൽ", "साथी से या किसी समझौते पर खटपट"],
        ["something sudden or hidden coming to light", "പെട്ടെന്നുള്ളതോ മറഞ്ഞിരുന്നതോ ആയ ഒന്ന് വെളിച്ചത്തു വരുന്നത്", "कुछ अचानक या छिपा हुआ सामने आना"],
        ["a plan that stalls or a disagreement with an elder", "നീളുന്ന ഒരു പദ്ധതി, അല്ലെങ്കിൽ ഒരു മുതിർന്നയാളുമായുള്ള അഭിപ്രായഭിന്നത", "अटकी हुई कोई योजना या किसी बड़े से मतभेद"],
        ["pressure at work or with someone senior", "ജോലിയിലോ മേലധികാരിയുമായോ ഉള്ള സമ്മർദ്ദം", "काम पर या किसी वरिष्ठ के साथ दबाव"],
        ["a gain that is delayed or a friend who lets you down", "വൈകുന്ന ഒരു നേട്ടം, അല്ലെങ്കിൽ നിരാശപ്പെടുത്തുന്ന ഒരു സുഹൃത്ത്", "देर से आने वाला लाभ या साथ न देने वाला दोस्त"],
        ["an expense, a loss or poor sleep", "ഒരു ചെലവ്, ഒരു നഷ്ടം, അല്ലെങ്കിൽ ഉറക്കക്കുറവ്", "कोई ख़र्च, नुक़सान या कम नींद"],
    ];
    let i = (house.clamp(1, 12) - 1) as usize;
    lang.pick(if good { good_t[i] } else { hard_t[i] })
}

/// Who a house stands for (the table in the module notes).
pub fn who_of_house(lang: Lang, house: u8) -> &'static str {
    let t: [[&str; 3]; 12] = [
        ["you yourself", "നിങ്ങൾ തന്നെ", "आप ख़ुद"],
        ["someone in your family at home", "വീട്ടിലെ ഒരു കുടുംബാംഗം", "घर में परिवार का कोई सदस्य"],
        ["a brother or sister, or a neighbour", "ഒരു സഹോദരനോ സഹോദരിയോ, അല്ലെങ്കിൽ ഒരു അയൽക്കാരൻ", "कोई भाई या बहन, या पड़ोसी"],
        ["your mother, or someone at home", "അമ്മ, അല്ലെങ്കിൽ വീട്ടിലുള്ള ഒരാൾ", "आपकी माँ, या घर का कोई व्यक्ति"],
        ["a child, or someone younger you guide", "ഒരു കുട്ടി, അല്ലെങ്കിൽ നിങ്ങൾ വഴികാട്ടുന്ന ഒരു ഇളയയാൾ", "कोई बच्चा, या कोई छोटा जिसे आप राह दिखाते हैं"],
        ["a colleague, or someone you serve", "ഒരു സഹപ്രവർത്തകൻ, അല്ലെങ്കിൽ നിങ്ങൾ സേവിക്കുന്ന ഒരാൾ", "कोई सहकर्मी, या जिसकी आप सेवा करते हैं"],
        ["your partner, or a business partner", "നിങ്ങളുടെ പങ്കാളി, അല്ലെങ്കിൽ ഒരു ബിസിനസ് പങ്കാളി", "आपका जीवनसाथी, या कोई व्यापारिक साझेदार"],
        ["your partner's family", "പങ്കാളിയുടെ കുടുംബം", "जीवनसाथी का परिवार"],
        ["your father, a teacher or an older family member", "അച്ഛൻ, ഒരു അധ്യാപകൻ, അല്ലെങ്കിൽ കുടുംബത്തിലെ ഒരു മുതിർന്നയാൾ", "आपके पिता, कोई शिक्षक या परिवार का कोई बड़ा"],
        ["your boss, or someone senior at work", "നിങ്ങളുടെ മേലധികാരി, അല്ലെങ്കിൽ ജോലിയിലെ ഒരു മുതിർന്നയാൾ", "आपके बॉस, या काम पर कोई वरिष्ठ"],
        ["a friend, or an elder brother or sister", "ഒരു സുഹൃത്ത്, അല്ലെങ്കിൽ ഒരു മൂത്ത സഹോദരനോ സഹോദരിയോ", "कोई दोस्त, या बड़ा भाई या बहन"],
        ["someone far away or abroad", "ദൂരെയോ വിദേശത്തോ ഉള്ള ഒരാൾ", "दूर या विदेश में रहने वाला कोई"],
    ];
    lang.pick(t[(house.clamp(1, 12) - 1) as usize])
}

/// Who a natal planet stands for, when a passage over it names the person.
pub fn who_of_planet(lang: Lang, g: DashaLord) -> &'static str {
    lang.pick(match g {
        DashaLord::Sun => ["your father, or a senior official", "അച്ഛൻ, അല്ലെങ്കിൽ ഒരു ഉയർന്ന ഉദ്യോഗസ്ഥൻ", "आपके पिता, या कोई बड़ा अधिकारी"],
        DashaLord::Moon => ["your mother, or an older woman in the family", "അമ്മ, അല്ലെങ്കിൽ കുടുംബത്തിലെ ഒരു മുതിർന്ന സ്ത്രീ", "आपकी माँ, या परिवार की कोई बड़ी महिला"],
        DashaLord::Mars => ["a brother, or someone in uniform", "ഒരു സഹോദരൻ, അല്ലെങ്കിൽ യൂണിഫോമിലുള്ള ഒരാൾ", "कोई भाई, या वर्दी वाला कोई व्यक्ति"],
        DashaLord::Mercury => ["a younger relative, a friend or a colleague", "ഒരു ഇളയ ബന്ധു, സുഹൃത്ത്, അല്ലെങ്കിൽ സഹപ്രവർത്തകൻ", "कोई छोटा रिश्तेदार, दोस्त या सहकर्मी"],
        DashaLord::Jupiter => ["a teacher, an elder or a child", "ഒരു അധ്യാപകൻ, ഒരു മുതിർന്നയാൾ, അല്ലെങ്കിൽ ഒരു കുട്ടി", "कोई शिक्षक, कोई बड़ा या कोई बच्चा"],
        DashaLord::Venus => ["your spouse or partner, or a woman close to you", "ജീവിതപങ്കാളി, അല്ലെങ്കിൽ അടുപ്പമുള്ള ഒരു സ്ത്രീ", "आपका जीवनसाथी, या कोई क़रीबी महिला"],
        DashaLord::Saturn => ["an older person, or people who work for you", "ഒരു പ്രായമായ ആൾ, അല്ലെങ്കിൽ നിങ്ങൾക്കായി ജോലി ചെയ്യുന്നവർ", "कोई बुज़ुर्ग, या आपके लिए काम करने वाले लोग"],
        DashaLord::Rahu => ["a stranger, or someone from abroad", "ഒരു അപരിചിതൻ, അല്ലെങ്കിൽ വിദേശത്തുനിന്നുള്ള ഒരാൾ", "कोई अजनबी, या विदेश से जुड़ा कोई"],
        DashaLord::Ketu => ["a guide, or someone on a spiritual path", "ഒരു വഴികാട്ടി, അല്ലെങ്കിൽ ആത്മീയ വഴിയിലുള്ള ഒരാൾ", "कोई मार्गदर्शक, या आध्यात्मिक राह पर चलने वाला कोई"],
    })
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn sure_line(lang: Lang, agree: usize, of: usize) -> String {
    match lang {
        Lang::En => format!("{agree} of {of} ways of reading agree."),
        Lang::Ml => format!("{of} വായനകളിൽ {agree} എണ്ണം യോജിക്കുന്നു."),
        Lang::Hi => format!("{of} में से {agree} पढ़तें सहमत हैं।"),
    }
}

// ---------------------------------------------------------------------------------
// Triggers
// ---------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct Triggers {
    lords: Vec<(DashaLord, f64)>,
    houses: [f64; 13],
    /// Nadi points in orb bearing this way: (mover, point, closeness × |weight|).
    nadi: Vec<(DashaLord, NadiPoint, f64)>,
    vim: Vec<DashaLord>,
}

impl<'a> Consensus<'a> {
    /// What triggers `life` at `t` in direction `good`; `agree` says which systems lean
    /// that way (all, when unknown).
    fn triggers(&self, t: Instant, life: Life, good: bool, agree: Option<&[System]>) -> Triggers {
        let f = &self.natal.facts;
        let agrees = |s: System| agree.map_or(true, |a| a.contains(&s));
        let mut lords: Vec<(DashaLord, f64)> = Vec::new();
        let mut add = |g: DashaLord, w: f64| match lords.iter_mut().find(|x| x.0 == g) {
            Some(x) => x.1 += w,
            None => lords.push((g, w)),
        };
        let levels = vimshottari_levels(self.natal.moon_sidereal, self.natal.birth, YEAR_365_25_NS, t, 3);
        let vf = if agrees(System::Vimshottari) { 1.0 } else { 0.3 };
        for (i, p) in levels.iter().enumerate() {
            // The shorter the period, the more it says about this window in particular:
            // the long chapter colours years, the shortest level these weeks.
            add(p.lord, [0.15, 0.30, 0.40][i.min(2)] * vf);
        }
        if let Some(y) = self.years.iter().find(|y| y.start <= t && t < y.end) {
            let af = if agrees(System::Annual) { 1.0 } else { 0.3 };
            if let Some(m) = y.mudda.iter().find(|p| p.start <= t && t < p.end) {
                add(m.lord, 0.25 * af);
                if let Some(a) = subdivide(*m).into_iter().find(|p| p.start <= t && t < p.end) {
                    add(a.lord, 0.2 * af);
                }
            }
        }
        let mut houses = [0.0; 13];
        let mut nadi_v = Vec::new();
        if let Ok(s) = sky(self.spk, self.eop, self.natal, &self.obs, HouseSystem::WholeSign, self.ayanamsa, t) {
            let movers = [DashaLord::Jupiter, DashaLord::Saturn, DashaLord::Rahu].map(|g| s.facts.planet(g).sidereal);
            if agrees(System::Nadi) {
                for (mover, point, close, w) in self.nadi.in_orb(movers, life) {
                    if (w > 0.0) == good && w.abs() > 0.05 {
                        let v = close * w.abs();
                        nadi_v.push((mover, point, v));
                        match point.graha() {
                            Some(g) => add(g, 0.6 * v),
                            None => {
                                let h = (((nadi::bhrigu_bindu(f.planet(DashaLord::Rahu).sidereal, f.planet(DashaLord::Moon).sidereal) / 30.0).floor() as i64 - f.lagna_sign as i64).rem_euclid(12) + 1) as usize;
                                houses[h] += 0.6 * v;
                            }
                        }
                    }
                }
            }
            if agrees(System::Transit) {
                for g in [DashaLord::Jupiter, DashaLord::Saturn] {
                    let h = ((s.facts.planet(g).sign as i64 - f.lagna_sign as i64).rem_euclid(12) + 1) as usize;
                    houses[h] += 0.2;
                }
            }
        }
        for &(g, w) in &lords {
            for (i, l) in f.rasi_lords.iter().enumerate() {
                if *l == g {
                    houses[i + 1] += w;
                }
            }
            let p = f.planet(g);
            houses[p.rasi_house as usize] += 0.7 * w;
            if matches!(g, DashaLord::Rahu | DashaLord::Ketu) {
                // A node gives the results of its sign's lord.
                let disp = sign_lord(p.sign as usize);
                for (i, l) in f.rasi_lords.iter().enumerate() {
                    if *l == disp {
                        houses[i + 1] += 0.5 * w;
                    }
                }
            }
        }
        lords.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        nadi_v.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        Triggers { lords, houses, nadi: nadi_v, vim: levels.iter().map(|p| p.lord).collect() }
    }
}

/// The house that sets the kind: the heaviest among the life's own houses (good), or
/// among the three hard houses (6, 8, 12, the dusthanas of BPHS ch. 34) and the life's
/// own main house (hard); the life's main house if none carries weight. The KP negating
/// houses deny a matter, but what they stand for (the 10th for children, the 5th for a
/// career) is not what a person meets in a hard stretch, so they set no words.
fn kind_house(tr: &Triggers, life: Life, good: bool) -> u8 {
    let area = life.area();
    let set: Vec<u8> = if good { area.group.to_vec() } else { vec![6, 8, 12, area.primary] };
    set.iter().copied().filter(|h| tr.houses[*h as usize] > 0.0).max_by(|a, b| tr.houses[*a as usize].partial_cmp(&tr.houses[*b as usize]).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(area.primary)
}

/// The house that sets who: the heaviest house other than the 1st and the kind house;
/// the kind house when nothing else carries weight.
fn who_house(tr: &Triggers, kind: u8) -> u8 {
    (2..=12u8)
        .filter(|h| *h != kind && tr.houses[*h as usize] > 0.0)
        .max_by(|a, b| tr.houses[*a as usize].partial_cmp(&tr.houses[*b as usize]).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(kind)
}

// ---------------------------------------------------------------------------------
// Narrowing
// ---------------------------------------------------------------------------------

fn day_bounds(u: &Utc, off: i32) -> Option<(Instant, Instant)> {
    let mid = Utc::new(u.year, u.month, u.day, 0, 0, 0, 0).ok()?.to_instant().ok()?.add_ns(-(off as i128) * 60 * NS_PER_SEC);
    Some((mid, mid.add_ns(NS_PER_DAY)))
}

fn day_label(lang: Lang, u: &Utc) -> String {
    let wd = ((u.utc_day() + 4).rem_euclid(7)) as usize;
    let m = mi(u);
    match lang {
        Lang::En => format!("{} {} {}", lang::weekday(lang, wd), u.day, MONTHS_EN[m]),
        Lang::Ml => format!("{} {}, {}", MONTHS_ML[m], u.day, lang::weekday(lang, wd)),
        Lang::Hi => format!("{} {}, {}", u.day, MONTHS_HI[m], lang::weekday(lang, wd)),
    }
}

fn hour_words(lang: Lang, a: &Utc, b_hour: u32) -> String {
    let h0 = a.hour;
    let h1 = b_hour;
    let part = |h: u32| -> &'static str {
        let t = match h {
            5..=11 => ["morning", "രാവിലെ", "सुबह"],
            12..=16 => ["afternoon", "ഉച്ചകഴിഞ്ഞ്", "दोपहर"],
            17..=20 => ["evening", "വൈകുന്നേരം", "शाम"],
            _ => ["night", "രാത്രി", "रात"],
        };
        lang.pick(t)
    };
    match lang {
        Lang::En => {
            let ap = |h: u32| if h % 24 < 12 { "am" } else { "pm" };
            let h12 = |h: u32| match h % 12 {
                0 => 12,
                x => x,
            };
            let span = if ap(h0) == ap(h1) { format!("{}–{} {}", h12(h0), h12(h1), ap(h1)) } else { format!("{} {}–{} {}", h12(h0), ap(h0), h12(h1), ap(h1)) };
            format!("{} (about {span})", part(h0))
        }
        Lang::Ml => format!("{} (ഏകദേശം {:02}:00–{:02}:00)", part(h0), h0, h1),
        Lang::Hi => format!("{} (लगभग {:02}:00–{:02}:00)", part(h0), h0, h1),
    }
}

/// Narrow `[start, end]` for `life` to its likely days, and the hours of the best day
/// when `hours` and they are sharp. Days: the Moon over the triggering natal degrees (the
/// two strongest triggering lords) and over the Nadi points being passed, each crossing
/// weighted by its trigger; the day of an exact Nadi passage counts most; the weekday of
/// the strongest lord adds a little. Hours: the strongest lord's hora at the present
/// place, inside the stretch when the rising sign there stands in that lord's natal sign,
/// between 6 am and 10 pm; given only when the two overlap for twenty minutes or more.
#[allow(clippy::too_many_arguments)]
fn narrow_with(c: &Consensus, life: Life, good: bool, start: Instant, end: Instant, now: Instant, off: i32, lang: Lang, tr: &Triggers, hours: bool) -> Narrowing {
    let f = &c.natal.facts;
    let a = start.max(now);
    if end <= a {
        return Narrowing::default();
    }
    let total: f64 = tr.lords.iter().take(2).map(|x| x.1).sum::<f64>().max(1e-9);
    let mut points: Vec<(f64, f64)> = tr.lords.iter().take(2).map(|(g, w)| (f.planet(*g).sidereal, w / total)).collect();
    let bb = nadi::bhrigu_bindu(f.planet(DashaLord::Rahu).sidereal, f.planet(DashaLord::Moon).sidereal);
    for (_, p, v) in tr.nadi.iter().take(2) {
        let lon = match p.graha() {
            Some(g) => f.planet(g).sidereal,
            None => bb,
        };
        points.push((lon, 0.8 + v.min(1.0)));
    }
    let mut days: Vec<((i64, u32, u32), f64)> = Vec::new();
    let mut bump = |u: &Utc, w: f64, only_existing: bool| {
        let k = (u.year, u.month, u.day);
        match days.iter_mut().find(|d| d.0 == k) {
            Some(d) => d.1 += w,
            None if !only_existing => days.push((k, w)),
            None => {}
        }
    };
    for (lon, w) in &points {
        for t in returns(c.spk, c.ayanamsa, DashaLord::Moon, *lon, a, end) {
            bump(&local(t, off), *w, false);
        }
    }
    let li = li_of(life);
    for h in &c.hits {
        if h.at < a || h.at >= end {
            continue;
        }
        let w = c.nadi_weight(h, li);
        if (w > 0.0) == good && w.abs() > 0.05 {
            bump(&local(h.at, off), 1.5, false);
        }
    }
    if let Some(wd) = tr.lords.first().and_then(|l| lang::weekday_of_lord(l.0)) {
        for d in days.iter_mut() {
            if let Some(u) = Utc::new(d.0 .0, d.0 .1, d.0 .2, 12, 0, 0, 0).ok() {
                if ((u.utc_day() + 4).rem_euclid(7)) as usize == wd {
                    d.1 += 0.25;
                }
            }
        }
    }
    days.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal).then(x.0.cmp(&y.0)));
    days.truncate(3);
    let top = days.first().map(|d| d.0);
    days.sort_by_key(|d| d.0);
    let mut out = Narrowing::default();
    for (k, _) in &days {
        let Ok(u) = Utc::new(k.0, k.1, k.2, 12, 0, 0, 0) else { continue };
        let Some((s, e)) = day_bounds(&u, off) else { continue };
        let mut bd = BestDay { start: s, end: e, label: day_label(lang, &u), hours: None };
        if hours && Some(*k) == top {
            bd.hours = tr.lords.first().and_then(|l| sharp_hours(c, l.0, s, off, lang));
        }
        out.days.push(bd);
    }
    if let Some(d) = out.days.iter().find(|d| d.hours.is_some()) {
        let h = d.hours.as_ref().map(|h| h.2.clone()).unwrap_or_default();
        out.line = match lang {
            Lang::En => format!("Most likely {}, {h}.", d.label),
            Lang::Ml => format!("മിക്കവാറും {}, {h}.", d.label),
            Lang::Hi => format!("सबसे संभवतः {}, {h}।", d.label),
        };
        out.hours_line = Some(h);
    } else if !out.days.is_empty() {
        let labels: Vec<&str> = out.days.iter().map(|d| d.label.as_str()).collect();
        let list = match (lang, labels.len()) {
            (_, 1) => labels[0].to_string(),
            (Lang::En, n) => format!("{} or {}", labels[..n - 1].join(", "), labels[n - 1]),
            (Lang::Ml, n) => format!("{} അല്ലെങ്കിൽ {}", labels[..n - 1].join(", "), labels[n - 1]),
            (Lang::Hi, n) => format!("{} या {}", labels[..n - 1].join(", "), labels[n - 1]),
        };
        out.line = match lang {
            Lang::En => format!("Most likely on {list}."),
            Lang::Ml => format!("മിക്കവാറും {list} ദിവസങ്ങളിൽ."),
            Lang::Hi => format!("सबसे संभवतः {list} को।"),
        };
    }
    out
}

/// The block of the day starting at local midnight `day0` when `lord`'s hora runs and
/// the rising sign at the present place is `lord`'s natal sign, between 6 am and 10 pm;
/// None unless it lasts twenty minutes.
fn sharp_hours(c: &Consensus, lord: DashaLord, day0: Instant, off: i32, lang: Lang) -> Option<(Instant, Instant, String)> {
    let sign = c.natal.facts.planet(lord).sign as usize;
    let hl = format!("{:?}", hora_lord_of(lord));
    let from = day0.add_ns(6 * 3_600 * NS_PER_SEC);
    let to = day0.add_ns(22 * 3_600 * NS_PER_SEC);
    // The day's horas of the lord.
    let noon = day0.add_ns(12 * 3_600 * NS_PER_SEC);
    let day = solar_day(c.spk, c.eop, &c.here, noon).ok().flatten()?;
    let mut slots = Vec::new();
    let mut t = day.sunrise;
    while t < day.next_sunrise {
        let (name, a, b) = hora_at(&day, &c.here, t);
        if name == hl {
            slots.push((a, b));
        }
        t = b.add_ns(NS_PER_SEC);
    }
    // The stretches when the rising sign there is the lord's natal sign, to ten minutes.
    let step = 10 * 60 * NS_PER_SEC;
    let times: Vec<Instant> = (0..=((to.ns_since(from)) / step)).map(|k| from.add_ns(k * step)).collect();
    let inside: Vec<bool> = par_map(&times, |t| ascendant(c.spk, c.eop, *t, &c.here, c.ayanamsa).map_or(false, |a| ((a.0 / 30.0).floor() as usize) % 12 == sign));
    let mut rising: Vec<(Instant, Instant)> = Vec::new();
    for (i, &on) in inside.iter().enumerate() {
        if !on {
            continue;
        }
        let (a, b) = (times[i], times[i].add_ns(step));
        match rising.last_mut() {
            Some(last) if last.1 == a => last.1 = b,
            _ => rising.push((a, b)),
        }
    }
    let mut best: Option<(Instant, Instant)> = None;
    for (sa, sb) in &slots {
        for (ra, rb) in &rising {
            let a = (*sa).max(*ra).max(from);
            let b = (*sb).min(*rb).min(to);
            if b > a && best.map_or(true, |x| b.ns_since(a) > x.1.ns_since(x.0)) {
                best = Some((a, b));
            }
        }
    }
    let (a, b) = best?;
    if b.ns_since(a) < 20 * 60 * NS_PER_SEC {
        return None;
    }
    let ua = local(a, off);
    let ub = local(b, off);
    let h1 = if ub.minute > 0 || ub.second > 0 { ub.hour + 1 } else { ub.hour };
    let h1 = h1.max(ua.hour + 1);
    Some((a, b, hour_words(lang, &ua, h1)))
}

impl<'a> Consensus<'a> {
    fn nadi_weight(&self, h: &NadiHit, li: usize) -> f64 {
        nadi::relevance(self.natal, h.point, Life::ALL[li]) * nadi::polarity(self.natal, h.transit, h.kind, Life::ALL[li])
    }
}

// ---------------------------------------------------------------------------------
// The scan
// ---------------------------------------------------------------------------------

/// The windows of the next `months` months from `from` where the systems agree, for
/// each of `lives` (every life when empty), strongest first, at most `max`. `obs` is the
/// birth place, `here` where the person lives now (the Tajika year is cast there and the
/// hours are read there), `utc_offset_min` that place's clock for the words.
#[allow(clippy::too_many_arguments)]
pub fn sure_windows(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    here: &Observer,
    ayanamsa: Ayanamsa,
    lang: Lang,
    utc_offset_min: i32,
    from: Instant,
    months: u32,
    lives: &[Life],
    max: usize,
) -> Result<Vec<SureWindow>, ForecastError> {
    sure_windows_trusted(spk, eop, natal, obs, here, ayanamsa, lang, utc_offset_min, from, months, lives, max, &[1.0; NS])
}

/// [`sure_windows`] with the person's own multiplier on each system's say, in
/// [`SYSTEMS`] order ([`crate::integrity::consensus_multipliers`], earned only from
/// their resolved sealed predictions); all ones reproduce [`sure_windows`] exactly.
#[allow(clippy::too_many_arguments)]
pub fn sure_windows_trusted(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    here: &Observer,
    ayanamsa: Ayanamsa,
    lang: Lang,
    utc_offset_min: i32,
    from: Instant,
    months: u32,
    lives: &[Life],
    max: usize,
    mult: &[f64; NS],
) -> Result<Vec<SureWindow>, ForecastError> {
    let months = months.clamp(1, 60) as i128;
    let to = from.add_ns(months * MONTH_NS);
    let base_from = from.add_ns(-BASE_BACK_MONTHS * MONTH_NS).max(natal.birth.add_ns(NS_PER_DAY));
    let mut c = Consensus::new(spk, eop, natal, obs, here, ayanamsa, base_from, to);
    c.mult = mult.map(|m| if m.is_finite() { m.clamp(crate::integrity::MULT_RANGE.0, crate::integrity::MULT_RANGE.1) } else { 1.0 });
    let n = (to.ns_since(base_from) / GRID_NS) as usize + 1;
    let grid: Vec<Instant> = (0..n).map(|k| base_from.add_ns(k as i128 * GRID_NS)).collect();
    let vals: Vec<Option<Vals>> = par_map(&grid, |t| c.eval(*t));
    if vals.iter().any(|v| v.is_none()) {
        return Err(ForecastError::Chart("a sample of the sky could not be read".into()));
    }
    let vals: Vec<Vals> = vals.into_iter().flatten().collect();
    let base = Base::new(&c, &vals);
    let lives: Vec<Life> = if lives.is_empty() { Life::ALL.to_vec() } else { lives.to_vec() };
    let first_ahead = grid.iter().position(|t| *t >= from).unwrap_or(grid.len());

    // (life, tier, good, i0, i1 inclusive, peak index, |z| at peak)
    let mut found: Vec<(Life, Tier, bool, usize, usize, usize, f64)> = Vec::new();
    for &life in &lives {
        let li = li_of(life);
        let leans: Vec<Lean> = vals.iter().map(|v| base.lean(v, li)).collect();
        let tiers: Vec<Option<(Tier, bool)>> = leans.iter().map(|l| tier_of(l).map(|t| (t, l.good()))).collect();
        let mut i = 0;
        while i < n {
            let Some((_, good)) = tiers[i] else {
                i += 1;
                continue;
            };
            let mut j = i;
            while j + 1 < n && tiers[j + 1].map_or(false, |t| t.1 == good) {
                j += 1;
            }
            // The run is i..=j. Its tier: the highest held two samples running.
            if j >= first_ahead && j > i {
                let held = |t: Tier| (i..j).any(|k| tiers[k].map_or(false, |x| x.0 >= t) && tiers[k + 1].map_or(false, |x| x.0 >= t));
                if let Some(tier) = [Tier::Strong, Tier::Likely, Tier::Possible].into_iter().find(|t| held(*t)) {
                    // Every core at that tier (single-sample dips merged).
                    let mut k = i;
                    while k <= j {
                        if !tiers[k].map_or(false, |x| x.0 >= tier) {
                            k += 1;
                            continue;
                        }
                        let mut e = k;
                        while e + 1 <= j && (tiers[e + 1].map_or(false, |x| x.0 >= tier) || (e + 2 <= j && tiers[e + 2].map_or(false, |x| x.0 >= tier))) {
                            e += 1;
                        }
                        while !tiers[e].map_or(false, |x| x.0 >= tier) {
                            e -= 1;
                        }
                        if e > k && e >= first_ahead {
                            let lo = k.max(first_ahead.min(e));
                            let peak = (lo..=e).max_by(|a, b| leans[*a].z.abs().partial_cmp(&leans[*b].z.abs()).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(lo);
                            found.push((life, tier, good, k, e, peak, leans[peak].z.abs()));
                        }
                        k = e + 1;
                    }
                }
            }
            i = j + 1;
        }
    }
    found.sort_by(|a, b| b.1.cmp(&a.1).then(b.6.partial_cmp(&a.6).unwrap_or(std::cmp::Ordering::Equal)));
    // One stretch, one card: when the same running periods move several parts of life
    // at once, the windows share their dates. The strongest is kept and names the others,
    // so a hard stretch is said once, not seven times.
    let mut kept: Vec<((Life, Tier, bool, usize, usize, usize, f64), Vec<Life>)> = Vec::new();
    for f in found {
        // Two possible windows of one life within a month of each other are one month's lean.
        if let Some((k, _)) = kept.iter_mut().find(|(k, _)| k.0 == f.0 && k.2 == f.2 && k.1 == Tier::Possible && f.1 == Tier::Possible && f.3 <= k.4 + 10 && k.3 <= f.4 + 10) {
            k.3 = k.3.min(f.3);
            k.4 = k.4.max(f.4);
            continue;
        }
        let dup = kept.iter_mut().find(|(k, _)| {
            let (lo, hi) = (f.3.max(k.3), f.4.min(k.4));
            k.2 == f.2 && hi >= lo && (hi - lo + 1) * 2 >= (f.4 - f.3 + 1).min(k.4 - k.3 + 1)
        });
        match dup {
            Some((_, also)) if lives.len() > 1 => {
                if !also.contains(&f.0) {
                    also.push(f.0);
                }
            }
            _ => kept.push((f, Vec::new())),
        }
    }
    kept.truncate(max.max(1));

    let built: Vec<SureWindow> = par_map(&kept, |((life, tier, good, i0, i1, peak, _), also)| {
        let (life, tier, good, i0, i1, peak) = (*life, *tier, *good, *i0, *i1, *peak);
        let li = li_of(life);
        let pred = |t: Instant| c.eval(t).map_or(false, |v| {
            let l = base.lean(&v, li);
            tier_of(&l).map_or(false, |x| x >= tier) && l.good() == good
        });
        let bisect = |mut out: Instant, mut inn: Instant| {
            while (inn.ns_since(out)).abs() > EDGE_NS {
                let mid = out.add_ns(inn.ns_since(out) / 2);
                if pred(mid) {
                    inn = mid;
                } else {
                    out = mid;
                }
            }
            inn
        };
        // Exact edges for the likely and strong windows; the possible ones are said by
        // the month and keep the grid's edges.
        let start = if i0 == 0 { grid[0] } else if tier >= Tier::Likely { bisect(grid[i0 - 1], grid[i0]) } else { grid[i0] };
        let end = if i1 + 1 >= n { to } else if tier >= Tier::Likely { bisect(grid[i1 + 1], grid[i1]) } else { grid[i1] };
        let l = base.lean(&vals[peak], li);
        build(&c, life, tier, good, start, end, grid[peak], &l, from, utc_offset_min, lang, also)
    });
    let mut built = built;
    built.sort_by(|a, b| b.tier.cmp(&a.tier).then(a.start.cmp(&b.start)));
    Ok(built)
}

#[allow(clippy::too_many_arguments)]
fn build(c: &Consensus, life: Life, tier: Tier, good: bool, start: Instant, end: Instant, peak: Instant, l: &Lean, now: Instant, off: i32, lang: Lang, also: &[Life]) -> SureWindow {
    let systems = l.agreeing();
    let tr = c.triggers(peak, life, good, Some(&systems));
    let kh = kind_house(&tr, life, good);
    let karaka = tr.nadi.first().and_then(|n| n.1.graha());
    let wh = who_house(&tr, kh);
    let kind = kind_words(lang, kh, good).to_string();
    let who = match karaka {
        Some(g) => who_of_planet(lang, g).to_string(),
        None => who_of_house(lang, wh).to_string(),
    };
    let ev = event(lang, life, good);
    let btw = between(lang, start, end, now, off);
    let around = month_words(lang, &local(peak, off));
    let area = lang::life_lower(lang, life);
    let headline = match (tier, good, lang) {
        (Tier::Strong, true, Lang::En) => format!("{btw}, {ev}, involving {who}."),
        (Tier::Strong, true, Lang::Ml) => format!("{btw}, {who} ഉൾപ്പെടുന്ന {ev}."),
        (Tier::Strong, true, Lang::Hi) => format!("{btw}, {ev}, जिसमें {who} जुड़े होंगे।"),
        (Tier::Strong, false, Lang::En) => format!("{btw}, go gently: {ev}, involving {who}."),
        (Tier::Strong, false, Lang::Ml) => format!("{btw}, സാവധാനം നീങ്ങുക: {who} ഉൾപ്പെടുന്ന {ev}."),
        (Tier::Strong, false, Lang::Hi) => format!("{btw}, संभलकर चलें: {ev}, जिसमें {who} जुड़े हो सकते हैं।"),
        (Tier::Likely, true, Lang::En) => format!("{btw}, {ev}, most likely through {kind}."),
        (Tier::Likely, true, Lang::Ml) => format!("{btw}, {ev}, മിക്കവാറും {kind} വഴി."),
        (Tier::Likely, true, Lang::Hi) => format!("{btw}, {ev}, सबसे संभवतः {kind} के ज़रिए।"),
        (Tier::Likely, false, Lang::En) => format!("{btw}, the signs point to {ev}, around {kind}."),
        (Tier::Likely, false, Lang::Ml) => format!("{btw}, {kind} ചുറ്റിപ്പറ്റി {ev} വരാൻ സാധ്യത."),
        (Tier::Likely, false, Lang::Hi) => format!("{btw}, {kind} को लेकर {ev} की संभावना है।"),
        (Tier::Possible, true, Lang::En) => format!("{} matters may move around {around}.", cap_first(&area)),
        (Tier::Possible, true, Lang::Ml) => format!("{around} ഓടെ {area} കാര്യങ്ങൾ നീങ്ങിയേക്കാം."),
        (Tier::Possible, true, Lang::Hi) => format!("{around} के आसपास {area} से जुड़े मामले आगे बढ़ सकते हैं।"),
        (Tier::Possible, false, Lang::En) => format!("{} may ask for a little care around {around}.", cap_first(&area)),
        (Tier::Possible, false, Lang::Ml) => format!("{around} ഓടെ {area} കാര്യത്തിൽ അൽപം ശ്രദ്ധ വേണ്ടിവന്നേക്കാം."),
        (Tier::Possible, false, Lang::Hi) => format!("{around} के आसपास {area} पर थोड़ा ध्यान देना पड़ सकता है।"),
    };
    let sure = sure_line(lang, l.agree, l.of);
    let mut detail = format!("{sure} {}", tier.words(lang));
    if tier >= Tier::Strong {
        detail.push(' ');
        detail.push_str(&match (good, lang) {
            (true, Lang::En) => format!("Most likely it comes through {kind}."),
            (true, Lang::Ml) => format!("മിക്കവാറും ഇത് {kind} വഴിയാകും."),
            (true, Lang::Hi) => format!("सबसे संभवतः यह {kind} के ज़रिए आएगा।"),
            (false, Lang::En) => format!("It is most likely to touch {kind}."),
            (false, Lang::Ml) => format!("ഇത് മിക്കവാറും {kind} എന്നിവയെ ബാധിക്കും."),
            (false, Lang::Hi) => format!("इसका असर सबसे ज़्यादा {kind} पर हो सकता है।"),
        });
    }
    if tier >= Tier::Likely {
        if let Some(&(g, _)) = tr.lords.first() {
            let what = if good { lang::support(lang, g) } else { lang::pressure(lang, g) };
            detail.push(' ');
            detail.push_str(&match lang {
                Lang::En => format!("The stretch brings {what}."),
                Lang::Ml => format!("ഈ കാലം {what} കൊണ്ടുവരുന്നു."),
                Lang::Hi => format!("यह समय {what} लाता है।"),
            });
        }
    }
    if !also.is_empty() {
        let list: Vec<String> = also.iter().map(|a| lang::life_lower(lang, *a)).collect();
        let joined = match (lang, list.len()) {
            (_, 1) => list[0].clone(),
            (Lang::En, n) => format!("{} and {}", list[..n - 1].join(", "), list[n - 1]),
            (Lang::Ml, _) => list.join(", "),
            (Lang::Hi, n) => format!("{} और {}", list[..n - 1].join(", "), list[n - 1]),
        };
        detail.push(' ');
        detail.push_str(&match lang {
            Lang::En => format!("{} {} this stretch too.", cap_first(&joined), if list.len() == 1 { "shares" } else { "share" }),
            Lang::Ml => format!("{joined} എന്നിവയ്ക്കും ഇതേ കാലം ബാധകമാണ്."),
            Lang::Hi => format!("{joined} पर भी यही समय लागू है।"),
        });
    }
    let names: Vec<&str> = systems.iter().map(|s| s.name(lang)).collect();
    if !names.is_empty() {
        detail.push(' ');
        detail.push_str(&match lang {
            Lang::En => format!("Agreeing: {}.", names.join(", ")),
            Lang::Ml => format!("യോജിക്കുന്നവ: {}.", names.join(", ")),
            Lang::Hi => format!("सहमत: {}।", names.join(", ")),
        });
    }
    let remedy_hint = (!good).then(|| {
        let p = parihara_from_facts(&c.natal.facts, Some(life.area()), &tr.vim);
        let title = p.remedies.iter().find(|r| !r.optional).or(p.remedies.first()).map(|r| r.title.clone());
        match (lang, title) {
            (Lang::En, Some(t)) => format!("What helps: {}.", t.trim_end_matches('.')),
            (Lang::En, None) => "Small deeds of care help here: see What helps.".to_string(),
            (Lang::Ml, _) => "ചെറിയ നല്ല പ്രവൃത്തികൾ ഇവിടെ സഹായിക്കും: 'സഹായിക്കുന്നവ' കാണുക.".to_string(),
            (Lang::Hi, _) => "छोटे अच्छे काम यहाँ मदद करते हैं: 'क्या मदद करता है' देखें।".to_string(),
        }
    });
    if !good {
        let ends = full_date(lang, &local(last_day(end), off));
        detail.push(' ');
        detail.push_str(&match lang {
            Lang::En => format!("This is a stretch to go gently, not a verdict: it eases after {ends}."),
            Lang::Ml => format!("ഇത് സാവധാനം നീങ്ങേണ്ട ഒരു കാലം മാത്രമാണ്, ഒരു തീർപ്പല്ല: {ends} കഴിഞ്ഞാൽ അയയും."),
            Lang::Hi => format!("यह संभलकर चलने का समय है, कोई अंतिम फ़ैसला नहीं: {ends} के बाद यह हल्का पड़ेगा।"),
        });
        if let Some(r) = &remedy_hint {
            detail.push(' ');
            detail.push_str(r);
        }
    }
    let (a, b) = (full_date(lang, &local(start.max(now), off)), full_date(lang, &local(last_day(end), off)));
    let seal_text = match (tier, lang) {
        (Tier::Strong, Lang::En) => format!("Between {a} and {b}: {ev}, involving {who}."),
        (Tier::Strong, Lang::Ml) => format!("{a} മുതൽ {b} വരെ: {who} ഉൾപ്പെടുന്ന {ev}."),
        (Tier::Strong, Lang::Hi) => format!("{a} से {b} तक: {ev}, जिसमें {who} जुड़े हों।"),
        (_, Lang::En) => format!("Between {a} and {b}: {ev}."),
        (_, Lang::Ml) => format!("{a} മുതൽ {b} വരെ: {ev}."),
        (_, Lang::Hi) => format!("{a} से {b} तक: {ev}।"),
    };
    let narrowing = if tier >= Tier::Likely { narrow_with(c, life, good, start, end, now, off, lang, &tr, tier == Tier::Strong) } else { Narrowing::default() };
    let mut expert = format!(
        "z {:+.2}; {}; lords {}",
        l.z,
        systems.iter().map(|s| s.expert()).collect::<Vec<_>>().join(", "),
        tr.lords.iter().take(4).map(|(g, w)| format!("{g:?} {w:.2}")).collect::<Vec<_>>().join(", ")
    );
    for (m, p, v) in tr.nadi.iter().take(3) {
        expert.push_str(&format!("; {m:?} over {} ({v:.2})", p.expert()));
    }
    expert.push_str(&format!("; kind from house {kh}, who from {}", karaka.map(|g| format!("natal {g:?}")).unwrap_or_else(|| format!("house {wh}"))));
    let li = li_of(life);
    let hits: Vec<NadiHit> = c
        .hits
        .iter()
        .filter(|h| h.at >= start.add_ns(-10 * NS_PER_DAY) && h.at <= end.add_ns(10 * NS_PER_DAY))
        .filter(|h| {
            let w = c.nadi_weight(h, li);
            (w > 0.0) == good && w.abs() > 0.05
        })
        .copied()
        .collect();
    SureWindow {
        life,
        also: also.to_vec(),
        start,
        end,
        peak,
        tier,
        good,
        agree: l.agree,
        against: l.against,
        of: l.of,
        z: l.z,
        strength: 1.0 / (1.0 + (-1.6 * l.z.abs()).exp()),
        systems,
        kind_house: kh,
        who_house: wh,
        who_karaka: karaka,
        triggers: tr.lords.clone(),
        hits,
        kind: if tier >= Tier::Likely { kind } else { String::new() },
        who: if tier == Tier::Strong { who } else { String::new() },
        date_line: date_line(lang, start, end, now, off, tier, peak),
        headline,
        detail,
        sure_line: sure,
        remedy_hint,
        seal_text,
        narrowing,
        expert,
    }
}

/// Narrow any window of `life` (good or hard) to its likely days and, when sharp, the
/// hours of the best day, read where `here` stands. Given a window's `peak` and its
/// agreeing `systems` (from [`sure_windows`]) the triggers are read exactly as the window
/// read them, so the days match; without them, at the middle of the window with every
/// system taken as agreeing.
#[allow(clippy::too_many_arguments)]
pub fn sure_hours(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    here: &Observer,
    ayanamsa: Ayanamsa,
    lang: Lang,
    utc_offset_min: i32,
    life: Life,
    good: bool,
    start: Instant,
    end: Instant,
    now: Instant,
    peak: Option<Instant>,
    systems: Option<&[System]>,
) -> Narrowing {
    if end <= start {
        return Narrowing::default();
    }
    let c = Consensus::new(spk, eop, natal, obs, here, ayanamsa, start, end);
    let mid = peak.unwrap_or_else(|| start.max(now).add_ns(end.ns_since(start.max(now)) / 2));
    let tr = c.triggers(mid, life, good, systems);
    narrow_with(&c, life, good, start, end, now, utc_offset_min, lang, &tr, true)
}

/// The share of the baseline instants of `life` that reach each tier (strong, likely,
/// possible), for the calibration test and the expert page.
#[allow(clippy::too_many_arguments)]
pub fn tier_rates(spk: &Spk, eop: Option<&EopTable>, natal: &Natal, obs: &Observer, ayanamsa: Ayanamsa, from: Instant, months: u32, life: Life) -> [f64; 3] {
    let to = from.add_ns(months as i128 * MONTH_NS);
    let base_from = from.add_ns(-BASE_BACK_MONTHS * MONTH_NS);
    let c = Consensus::new(spk, eop, natal, obs, obs, ayanamsa, base_from, to);
    let n = (to.ns_since(base_from) / GRID_NS) as usize + 1;
    let grid: Vec<Instant> = (0..n).map(|k| base_from.add_ns(k as i128 * GRID_NS)).collect();
    let vals: Vec<Vals> = par_map(&grid, |t| c.eval(*t)).into_iter().flatten().collect();
    let base = Base::new(&c, &vals);
    let li = li_of(life);
    let mut r = [0.0; 3];
    for v in &vals {
        match tier_of(&base.lean(v, li)) {
            Some(Tier::Strong) => r[0] += 1.0,
            Some(Tier::Likely) => r[1] += 1.0,
            Some(Tier::Possible) => r[2] += 1.0,
            None => {}
        }
    }
    r.map(|x| x / vals.len().max(1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;

    #[test]
    fn the_tier_arithmetic_is_as_documented() {
        let p = 1.0 - 0.691_462; // P(z ≥ 0.5)
        assert!((binomial_tail(8, 5, p) - 0.066).abs() < 0.002, "{}", binomial_tail(8, 5, p));
        assert!((binomial_tail(8, 4, p) - 0.21).abs() < 0.01, "{}", binomial_tail(8, 4, p));
        assert!((binomial_tail(8, 0, p) - 1.0).abs() < 1e-12);
        // The weights as documented.
        let w: Vec<f64> = SYSTEMS.iter().map(|s| s.weight()).collect();
        assert_eq!(w, vec![1.0, 0.30, 0.30, 0.15, 0.5, 0.35, 0.6, 0.3, 0.4]);
    }

    #[test]
    fn the_tiers_are_monotone() {
        // More agreeing systems, fewer against, or a larger pooled lean never lower the tier.
        let rank = |t: Option<Tier>| t.map_or(0, |t| t as usize + 1);
        for of in 3..=9usize {
            for agree in 0..=of {
                for against in 0..=(of - agree) {
                    for zi in 0..40 {
                        let z = zi as f64 * 0.1;
                        let t = rank(classify(agree, against, of, z));
                        assert!(rank(classify(agree, against, of, z + 0.1)) >= t);
                        assert!(rank(classify(agree, against, of, -z)) == t, "the sign does not matter");
                        if agree + against < of {
                            assert!(rank(classify(agree + 1, against, of, z)) >= t, "{agree} {against} {of} {z}");
                        }
                        if against > 0 {
                            assert!(rank(classify(agree, against - 1, of, z)) >= t);
                        }
                    }
                }
            }
        }
        assert_eq!(classify(5, 0, 8, 1.7), Some(Tier::Strong));
        assert_eq!(classify(5, 0, 8, 1.6), Some(Tier::Likely));
        assert_eq!(classify(4, 2, 8, 3.0), Some(Tier::Possible));
        assert_eq!(classify(2, 0, 8, 3.0), None);
    }

    #[test]
    fn the_tables_are_plain_in_every_language() {
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for h in 1..=12u8 {
                for good in [true, false] {
                    assert_eq!(jargon_in(lang, kind_words(lang, h, good)), None, "{h} {good}");
                }
                assert_eq!(jargon_in(lang, who_of_house(lang, h)), None, "{h}");
            }
            for g in GRAHAS {
                assert_eq!(jargon_in(lang, who_of_planet(lang, g)), None, "{g:?}");
            }
            for life in Life::ALL {
                for good in [true, false] {
                    assert_eq!(jargon_in(lang, event(lang, life, good)), None);
                }
            }
            for s in SYSTEMS {
                assert_eq!(jargon_in(lang, s.name(lang)), None, "{s:?}");
            }
            for t in [Tier::Strong, Tier::Likely, Tier::Possible] {
                assert_eq!(jargon_in(lang, t.words(lang)), None);
            }
        }
    }

    fn setup() -> Option<(Spk, Observer, Natal, Instant)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).ok()?;
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        Some((spk, obs, natal, at))
    }

    #[test]
    fn windows_are_dated_agreed_and_plain() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let here = Observer { latitude_deg: 12.9716, longitude_deg: 77.5946, height_m: 0.0 };
        let t = std::time::Instant::now();
        let w = sure_windows(&spk, None, &natal, &obs, &here, Ayanamsa::Lahiri, Lang::En, 330, at, 24, &[], 30).unwrap();
        eprintln!("{} windows in {:?}", w.len(), t.elapsed());
        assert!(!w.is_empty());
        let to = at.add_ns(24 * MONTH_NS);
        for x in &w {
            eprintln!("[{} {} {}/{} z{:+.2}] {} | {} | {}\n    {}\n    {}\n    {}", x.tier.key(), x.life.name(), x.agree, x.of, x.z, x.date_line, x.headline, x.narrowing.line, x.detail, x.seal_text, x.expert);
            assert!(x.start < x.end && x.end > at && x.end <= to, "{x:?}");
            assert!(x.peak >= x.start.add_ns(-GRID_NS) && x.peak <= x.end.add_ns(GRID_NS));
            assert!(x.agree + x.against <= x.of && x.of <= NS);
            assert_eq!(x.systems.len(), x.agree);
            assert_eq!(classify(x.agree, x.against, x.of, x.z).map_or(false, |t| t >= x.tier), true, "{x:?}");
            if x.tier == Tier::Strong {
                assert!(x.agree >= 5 && !x.who.is_empty() && !x.kind.is_empty());
                assert!(x.detail.contains("very likely"));
            } else {
                assert!(x.who.is_empty() && !x.detail.contains("very likely"));
            }
            if !x.good {
                assert!(x.remedy_hint.is_some(), "a hard window always carries what helps");
                assert!(x.detail.contains("not a verdict"));
            }
            for s in [&x.headline, &x.detail, &x.sure_line, &x.seal_text, &x.date_line, &x.who, &x.kind, &x.narrowing.line] {
                assert_eq!(jargon_in(Lang::En, s), None, "{s}");
                for bad in ["guarantee", "certain", "definitely", "fate", "only god"] {
                    assert!(!s.to_lowercase().contains(bad), "{s}");
                }
            }
            for d in &x.narrowing.days {
                assert!(d.end > x.start.max(at) && d.start < x.end, "{d:?}");
                if let Some((a, b, _)) = &d.hours {
                    assert!(*a >= d.start && *b <= d.end && a < b);
                }
            }
        }
        // Strongest first.
        assert!(w.windows(2).all(|p| p[0].tier >= p[1].tier));
        // Narrowing a strong window again, from its peak and its agreeing systems, gives
        // the same days and hours.
        if let Some(s) = w.iter().find(|x| x.tier == Tier::Strong) {
            let n = sure_hours(&spk, None, &natal, &obs, &here, Ayanamsa::Lahiri, Lang::En, 330, s.life, s.good, s.start, s.end, at, Some(s.peak), Some(&s.systems));
            assert_eq!(n, s.narrowing);
        }
        // Deterministic.
        let again = sure_windows(&spk, None, &natal, &obs, &here, Ayanamsa::Lahiri, Lang::En, 330, at, 24, &[], 30).unwrap();
        assert_eq!(w, again);
        // Every language reads plainly.
        for lang in [Lang::Ml, Lang::Hi] {
            for x in sure_windows(&spk, None, &natal, &obs, &here, Ayanamsa::Lahiri, lang, 330, at, 24, &[Life::Money, Life::Career], 8).unwrap() {
                for s in [&x.headline, &x.detail, &x.sure_line, &x.seal_text, &x.date_line, &x.narrowing.line] {
                    assert_eq!(jargon_in(lang, s), None, "{s}");
                }
            }
        }
    }

    #[test]
    fn strong_is_rare_over_the_baseline() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        for life in [Life::Money, Life::Career, Life::Love] {
            let r = tier_rates(&spk, None, &natal, &obs, Ayanamsa::Lahiri, at, 24, life);
            eprintln!("{}: strong {:.3}, likely {:.3}, possible {:.3}", life.name(), r[0], r[1], r[2]);
            assert!(r[0] <= 0.10, "{r:?}");
            assert!(r[0] + r[1] + r[2] <= 0.6, "{r:?}");
        }
    }

    #[test]
    fn a_window_narrows_to_days_inside_it() {
        let Some((spk, obs, natal, at)) = setup() else { return };
        let start = at.add_ns(10 * NS_PER_DAY);
        let end = start.add_ns(20 * NS_PER_DAY);
        let n = sure_hours(&spk, None, &natal, &obs, &obs, Ayanamsa::Lahiri, Lang::En, 330, Life::Career, true, start, end, at, None, None);
        eprintln!("{n:?}");
        // The Moon crosses every degree once in 27⅓ days, so in twenty days it crosses at
        // least one of the two triggering degrees.
        assert!(!n.days.is_empty() && n.days.len() <= 3);
        assert!(!n.line.is_empty());
        assert_eq!(jargon_in(Lang::En, &n.line), None);
        for d in &n.days {
            assert!(d.end > start && d.start < end);
            assert_eq!(d.end.ns_since(d.start), NS_PER_DAY);
        }
        assert!(n.days.windows(2).all(|p| p[0].start < p[1].start));
    }

    /// The rank scores make every system symmetric over the person's own baseline, so the
    /// gate has no built-in lean toward hard or good news: across five people both kinds
    /// reach the firm tiers.
    #[test]
    fn the_gate_says_good_and_hard_alike() {
        let mut good_firm = 0;
        let mut hard_firm = 0;
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let at = Utc::new(2026, 10, 20, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for (y, m, d, h, lat, lon) in [(1990, 2, 20, 5, 9.93, 76.27), (1985, 7, 3, 14, 28.6, 77.2), (1972, 11, 30, 1, 19.07, 72.87), (2001, 4, 12, 20, 13.08, 80.27), (1995, 9, 9, 9, 22.57, 88.36)] {
            let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
            let birth = Utc::new(y, m, d, h, 0, 0, 0).unwrap().to_instant().unwrap();
            let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
            let w = sure_windows(&spk, None, &natal, &obs, &obs, Ayanamsa::Lahiri, Lang::En, 330, at, 24, &[], 200).unwrap();
            let c = |t: Tier, g: bool| w.iter().filter(|x| x.tier == t && x.good == g).count();
            eprintln!("{y}: strong +{} -{}, likely +{} -{}, possible +{} -{}", c(Tier::Strong, true), c(Tier::Strong, false), c(Tier::Likely, true), c(Tier::Likely, false), c(Tier::Possible, true), c(Tier::Possible, false));
            good_firm += c(Tier::Strong, true) + c(Tier::Likely, true);
            hard_firm += c(Tier::Strong, false) + c(Tier::Likely, false);
        }
        assert!(good_firm * 3 >= hard_firm && hard_firm * 3 >= good_firm, "{good_firm} {hard_firm}");
    }
}
