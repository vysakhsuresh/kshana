//! Independent witnesses that judge a candidate birth time against a person's answers,
//! and the moments inside a birth-time window where the chart itself changes.
//!
//! Every witness reads one life at one instant for one candidate chart and returns a
//! number that is high when that life is favoured. [`crate::reveal::refine_birth`] turns
//! each witness's reading of an answered stretch into a distance from that life's own
//! average (in standard deviations, like the slow score), and pools the witnesses on the
//! log-odds scale with the weights of [`Witness::weight`]. [`crate::learn`] uses the same
//! readings, one witness at a time, to see which technique fits a person's past best.
//!
//! The witnesses:
//!
//! * [`Witness::Slow`]: the forecast's slow score (birth chart, three Vimshottari levels
//!   and the slow movers), the reading the past stretches were chosen from.
//! * [`Witness::Vimshottari`]: the Vimshottari periods alone (three levels), each lord
//!   read by [`crate::forecast::lord_score`]: half KP signification of the matter's
//!   houses, half Parashari quality of the lord for the matter.
//! * [`Witness::Yogini`]: the Yogini mahadasha and antardasha, read through the graha
//!   each yogini stands for (BPHS Yogini-dasha chapter; Mangala = Moon … Sankata = Rahu),
//!   with the same lord reading.
//! * [`Witness::Chara`]: K. N. Rao's Chara dasha (mahadasha and antardasha signs). A sign
//!   period is read half by the house the sign is from the rising sign (a house of the
//!   matter favours it, a house that negates it weighs against) and half by the sign's
//!   lord (Rao, *Predicting through Jaimini's Chara Dasha*: the dasha sign and its lord
//!   are judged for the houses they hold and rule).
//! * [`Witness::Kalachakra`]: the Kalachakra sign periods (BPHS Kalachakra-dasha
//!   chapter), read like Chara. Its row depends on the Moon's quarter-star, so it only
//!   discriminates across a quarter-star boundary.
//! * [`Witness::Kp`]: Krishnamurti Padhdhati. The running Vimshottari dasha, bhukti and
//!   antara lords' net support for the matter's houses ([`crate::kp::funnel_strength`]),
//!   70 %, and their agreement with the matter's cusp: whether each lord signifies a
//!   fulfilling house that the cusp's sub-lord or sub-sub-lord also signifies, 30 %
//!   (KP Reader III/IV: the event comes in the joint periods of the significators of
//!   the houses the cusp sub-lord signifies). The promise of the cusp (sub-lord 60 %,
//!   sub-sub-lord 40 %, [`crate::kp::promise`]) sets how far the witness is trusted:
//!   confidence `0.5 + 0.5 × promise`.
//! * [`Witness::Varga`]: the divisional chart of the matter: D9 for love and marriage,
//!   D10 for career, D12 for family and parents, D7 for children (BPHS ch. 6–7, the
//!   uses of the sixteen vargas). Each running Vimshottari lord is read by its house from
//!   the varga's rising sign (kendra or trikona 1.0, 2nd or 11th 0.75, 3rd 0.5, 6th, 8th
//!   or 12th 0.0; BPHS ch. 34 on the house classes), 70 %, and by whether it rules the
//!   matter's house of that varga (7th of D9, 10th of D10, 4th and 9th of D12, 5th of
//!   D7), 30 %. The varga rising sign changes every 10 to 17 minutes, so this witness
//!   splits candidates the main chart cannot.

use crate::forecast::{lord_score, slow_scores, Life, Natal, Sky, GRAHAS, LEVEL_WEIGHTS};
use crate::kp::{promise, significators, support_score};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::dashas::{
    chara, chara_direction, chara_sub, kalachakra_detail, kalachakra_sub, period_at, sign_index, yogini, yogini_sub, KalachakraDasha, Period,
    Yogini,
};
use kshana_vedic::{kp_lords, nakshatra, sign_lord, varga, vimshottari_levels, Ayanamsa, DashaLord};

/// Number of witnesses.
pub const NW: usize = 7;

/// One witness to a birth time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Witness {
    /// The forecast's slow score.
    Slow,
    /// Vimshottari periods alone.
    Vimshottari,
    /// Yogini periods.
    Yogini,
    /// Chara (Jaimini) sign periods.
    Chara,
    /// Kalachakra sign periods.
    Kalachakra,
    /// KP cusp sub-lord chain and timing funnel.
    Kp,
    /// The matter's divisional chart.
    Varga,
}

/// Every witness, in index order.
pub const WITNESSES: [Witness; NW] = [Witness::Slow, Witness::Vimshottari, Witness::Yogini, Witness::Chara, Witness::Kalachakra, Witness::Kp, Witness::Varga];

/// Slope of the logistic that turns a distance in standard deviations into a
/// probability: `logistic(1.6 z)` is within 0.02 of the normal `Φ(z)` everywhere (the
/// logit–probit scale factor 1.6–1.7; Amemiya 1981, *J. Econ. Lit.* 19, 1483).
pub const SLOPE: f64 = 1.6;

/// Soft bound on a witness's distance, in standard deviations. A person answers whether
/// something happened, not how much, so a stretch counts as marked or not: beyond about
/// one standard deviation more distance adds little (a bounded-influence score, Huber
/// 1964, *Ann. Math. Statist.* 35, 73). It also undoes a selection bias: the stretches are
/// asked where the recorded time's reading stands furthest out, so an unbounded score
/// would credit the recorded time for having been the one the questions were picked
/// from (the winner's curse; Zöllner & Pritchard 2007, *Am. J. Hum. Genet.* 80, 605).
pub const MARK_SCALE: f64 = 1.0;

impl Witness {
    /// Position in [`WITNESSES`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Witness::Slow => "slow",
            Witness::Vimshottari => "vimshottari",
            Witness::Yogini => "yogini",
            Witness::Chara => "chara",
            Witness::Kalachakra => "kalachakra",
            Witness::Kp => "kp",
            Witness::Varga => "varga",
        }
    }

    /// Weight in the rectification's pooled log-odds.
    ///
    /// The answers are pooled on the log-odds scale, `logit p = Σ wᵢ · 1.6 · zᵢ`
    /// (logarithmic opinion pooling, Genest & Zidek 1986, *Statist. Sci.* 1, 114; a total
    /// weight above one extremises, as recommended for partly independent forecasters by
    /// Satopää et al. 2014, *Int. J. Forecasting* 30, 344). Each answer keeps a proper
    /// predictive distribution, so the evidence against random answering stays honest
    /// (see [`crate::reveal::BirthRefine::log10_bayes`]). The weights:
    ///
    /// * Slow 1.0: the unit, the reading the stretches were chosen from.
    /// * Vimshottari 0.0: its periods are already 30 of the slow score's 80 parts;
    ///   counting them twice would double their say. Kept for the per-person comparison.
    /// * Yogini 0.30 and Chara 0.30: independent timing systems, two levels each; the
    ///   texts use them to confirm Vimshottari (Rao asks for an event to show in both
    ///   Vimshottari and Chara), so each counts for under a third of the main reading.
    /// * Kalachakra 0.15: as Chara, halved, because its row only changes at a
    ///   quarter-star boundary of the Moon.
    /// * KP 0.5: shares the Vimshottari timing with the slow score, but its cusp sub-lord
    ///   chain is the most birth-time sensitive factor of all (a cusp's sub changes every
    ///   2–6 minutes of birth time); half weight for the shared timing.
    /// * Varga 0.35: four lives only, and two-level house reading.
    pub fn weight(self) -> f64 {
        match self {
            Witness::Slow => 1.0,
            Witness::Vimshottari => 0.0,
            Witness::Yogini => 0.30,
            Witness::Chara => 0.30,
            Witness::Kalachakra => 0.15,
            Witness::Kp => 0.5,
            Witness::Varga => 0.35,
        }
    }
}

fn gi(g: DashaLord) -> usize {
    GRAHAS.iter().position(|x| *x == g).unwrap_or(0)
}

fn li_of(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

/// The divisional chart and its matter houses for a life (BPHS ch. 6: navamsa for the
/// spouse, dashamsa for deeds and power, dwadashamsa for the parents, saptamsa for
/// children); none for the other lives.
pub fn varga_of(life: Life) -> Option<(u32, &'static [u8])> {
    match life {
        Life::Love => Some((9, &[7])),
        Life::Career => Some((10, &[10])),
        Life::Family => Some((12, &[4, 9])),
        Life::Children => Some((7, &[5])),
        _ => None,
    }
}

/// Quality of a house counted from a rising sign, 1–12 (BPHS ch. 34: kendras and
/// trikonas good, 6, 8, 12 dusthanas; 2 and 11 gain, 3 mild).
fn house_quality(h: u8) -> f64 {
    match h {
        1 | 4 | 5 | 7 | 9 | 10 => 1.0,
        2 | 11 => 0.75,
        3 => 0.5,
        _ => 0.0,
    }
}

fn sign_of(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

/// The sidereal ascendant of a birth: the first cusp of the equal houses, which is the
/// ascendant at every latitude.
pub fn ascendant(spk: &Spk, eop: Option<&EopTable>, birth: Instant, obs: &Observer, ayanamsa: Ayanamsa) -> Option<(f64, f64)> {
    let ch = compute(spk, eop, &ChartInput { instant: birth, observer: *obs, system: HouseSystem::Equal, ayanamsa, topocentric_moon: false }).ok()?;
    let moon = ch.bodies.iter().find(|b| b.name == "Moon")?.sidereal;
    Some((ch.cusps_sidereal[0], moon))
}

/// What a candidate chart says, prepared once so every instant is cheap to read.
pub(crate) struct Reading<'a> {
    pub(crate) natal: &'a Natal,
    /// Life × graha: the forecast's lord reading.
    lord_tab: [[f64; 9]; 9],
    /// Life × graha: KP net support.
    kp_tab: [[f64; 9]; 9],
    /// Life × graha: 1 when the graha signifies a fulfilling house the cusp's sub-lord or
    /// sub-sub-lord also signifies.
    kp_chain: [[f64; 9]; 9],
    /// Life: trust in the KP witness from the cusp's promise.
    pub(crate) kp_conf: [f64; 9],
    /// Life × graha: divisional reading (NaN where the life has no varga).
    varga_tab: [[f64; 9]; 9],
    /// Life × sign: sign-period reading.
    sign_tab: [[f64; 12]; 9],
    yogini: Vec<Period>,
    chara: Vec<Period>,
    kala: KalachakraDasha,
}

impl<'a> Reading<'a> {
    /// Prepare the reading of `natal`, whose sidereal ascendant is `asc`.
    pub(crate) fn new(natal: &'a Natal, asc: f64) -> Self {
        let f = &natal.facts;
        let mut lord_tab = [[0.0; 9]; 9];
        let mut kp_tab = [[0.0; 9]; 9];
        let mut kp_chain = [[0.0; 9]; 9];
        let mut kp_conf = [1.0; 9];
        let mut varga_tab = [[f64::NAN; 9]; 9];
        let mut sign_tab = [[0.5; 12]; 9];
        let lagna = f.lagna_sign as usize;
        for (li, &life) in Life::ALL.iter().enumerate() {
            let area = life.area();
            for (g, &graha) in GRAHAS.iter().enumerate() {
                lord_tab[li][g] = lord_score(natal, life, graha).0;
                kp_tab[li][g] = support_score(f, area, graha).0;
            }
            // The cusp's sub-lord and sub-sub-lord, and the fulfilling houses they signify.
            let cusp = f.cusps[(area.primary - 1) as usize];
            let k = kp_lords(cusp);
            let p_sub = promise(f, area);
            let ss_sig = significators(f, k.sub_sub);
            let ss_f: f64 = ss_sig.iter().filter(|s| area.group.contains(&s.house)).map(|s| s.level.weight()).sum::<f64>() / area.group.len() as f64;
            let ss_d: f64 = ss_sig.iter().filter(|s| area.negating.contains(&s.house)).map(|s| s.level.weight()).sum::<f64>() / area.negating.len().max(1) as f64;
            let ss_strength = (ss_f - 0.5 * ss_d).clamp(0.0, 1.0);
            kp_conf[li] = 0.5 + 0.5 * (0.6 * p_sub.strength + 0.4 * ss_strength);
            let mut chain = [false; 13];
            for s in significators(f, k.sub).into_iter().chain(ss_sig) {
                if area.group.contains(&s.house) {
                    chain[s.house as usize] = true;
                }
            }
            for (g, &graha) in GRAHAS.iter().enumerate() {
                kp_chain[li][g] = if significators(f, graha).iter().any(|s| chain[s.house as usize]) { 1.0 } else { 0.0 };
            }
            // Sign periods: the sign's house from the rising sign, and its lord.
            for (s, slot) in sign_tab[li].iter_mut().enumerate() {
                let h = ((s + 12 - lagna) % 12 + 1) as u8;
                let house = if area.group.contains(&h) {
                    1.0
                } else if area.negating.contains(&h) {
                    0.0
                } else {
                    0.5
                };
                *slot = 0.5 * house + 0.5 * lord_tab[li][gi(sign_lord(s))];
            }
            if let Some((n, houses)) = varga_of(life) {
                let vl = sign_of(varga(asc, n));
                let rulers: Vec<DashaLord> = houses.iter().map(|h| sign_lord((vl + *h as usize - 1) % 12)).collect();
                for p in f.planets.iter() {
                    let vs = sign_of(varga(p.sidereal, n));
                    let h = ((vs + 12 - vl) % 12 + 1) as u8;
                    let rules = if rulers.contains(&p.graha) { 1.0 } else { 0.0 };
                    varga_tab[li][gi(p.graha)] = 0.7 * house_quality(h) + 0.3 * rules;
                }
            }
        }
        let grahas: [f64; 9] = std::array::from_fn(|i| f.planet(GRAHAS[i]).sidereal);
        Reading {
            natal,
            lord_tab,
            kp_tab,
            kp_chain,
            kp_conf,
            varga_tab,
            sign_tab,
            yogini: yogini(natal.moon_sidereal, natal.birth, YEAR_365_25_NS),
            chara: chara(asc, &grahas, natal.birth, YEAR_365_25_NS),
            kala: kalachakra_detail(natal.moon_sidereal, natal.birth, YEAR_365_25_NS),
        }
    }

    /// Every witness's reading of every life at `t`, `[witness][life]`; NaN where a
    /// witness has nothing to say. `sky` must be this chart's sky (see [`rebased`]).
    pub(crate) fn values(&self, sky: &Sky, t: Instant) -> [[f64; 9]; NW] {
        let n = self.natal;
        let mut out = [[f64::NAN; 9]; NW];
        let levels = vimshottari_levels(n.moon_sidereal, n.birth, YEAR_365_25_NS, t, 3);
        let slow = slow_scores(n, sky, &levels);
        let lw: Vec<f64> = (0..levels.len()).map(|i| LEVEL_WEIGHTS[i]).collect();
        let lw_sum: f64 = lw.iter().sum::<f64>().max(1e-12);
        // Two-level systems: maha and antar with the first two level weights.
        let two = |lords: &[usize], tab: &dyn Fn(usize) -> f64| -> f64 {
            let w = [LEVEL_WEIGHTS[0], LEVEL_WEIGHTS[1]];
            let mut num = 0.0;
            let mut den = 0.0;
            for (i, &l) in lords.iter().enumerate().take(2) {
                num += w[i] * tab(l);
                den += w[i];
            }
            if den > 0.0 {
                num / den
            } else {
                f64::NAN
            }
        };
        let yog: Vec<usize> = period_at(&self.yogini, t)
            .map(|m| {
                let mut v = vec![Yogini::from_name(&m.label).map(|y| gi(y.graha())).unwrap_or(0)];
                if let Some(a) = period_at(&yogini_sub(m), t) {
                    v.push(Yogini::from_name(&a.label).map(|y| gi(y.graha())).unwrap_or(0));
                }
                v
            })
            .unwrap_or_default();
        let chr: Vec<usize> = period_at(&self.chara, t)
            .and_then(|m| {
                let s = sign_index(&m.label)?;
                let mut v = vec![s];
                if let Some(a) = period_at(&chara_sub(m, chara_direction(s)), t) {
                    v.extend(sign_index(&a.label));
                }
                Some(v)
            })
            .unwrap_or_default();
        let kal: Vec<usize> = self
            .kala
            .periods
            .iter()
            .position(|p| p.start <= t && t < p.end)
            .and_then(|k| {
                let m = &self.kala.periods[k];
                let s = sign_index(&m.label)?;
                let pos = (self.kala.first_position + k) % 9;
                let mut v = vec![s];
                if let Some(a) = period_at(&kalachakra_sub(m, &self.kala.row, pos), t) {
                    v.extend(sign_index(&a.label));
                }
                Some(v)
            })
            .unwrap_or_default();
        for li in 0..9 {
            out[Witness::Slow.index()][li] = slow[li];
            let lvl = |tab: &[[f64; 9]; 9]| -> f64 { levels.iter().enumerate().map(|(i, p)| lw[i] * tab[li][gi(p.lord)]).sum::<f64>() / lw_sum };
            out[Witness::Vimshottari.index()][li] = lvl(&self.lord_tab);
            if !yog.is_empty() {
                out[Witness::Yogini.index()][li] = two(&yog, &|g| self.lord_tab[li][g]);
            }
            if !chr.is_empty() {
                out[Witness::Chara.index()][li] = two(&chr, &|s| self.sign_tab[li][s]);
            }
            if !kal.is_empty() {
                out[Witness::Kalachakra.index()][li] = two(&kal, &|s| self.sign_tab[li][s]);
            }
            out[Witness::Kp.index()][li] = 0.7 * lvl(&self.kp_tab) + 0.3 * lvl(&self.kp_chain);
            if !self.varga_tab[li][0].is_nan() {
                out[Witness::Varga.index()][li] = lvl(&self.varga_tab);
            }
        }
        out
    }

    /// How far each witness's reading of `life` over the answered months stands from
    /// its own average over the base months, in standard deviations, signed so that
    /// positive agrees with the stretch as it was said (`good` or hard), bounded softly
    /// as `S · tanh(z / S)` with `S` = [`MARK_SCALE`]. None where the witness has no
    /// reading or does not vary.
    pub(crate) fn zs(base: &[[[f64; 9]; NW]], inside: &[[[f64; 9]; NW]], life: Life, good: bool) -> [Option<f64>; NW] {
        let li = li_of(life);
        std::array::from_fn(|w| {
            let col: Vec<f64> = base.iter().map(|v| v[w][li]).filter(|x| x.is_finite()).collect();
            let ins: Vec<f64> = inside.iter().map(|v| v[w][li]).filter(|x| x.is_finite()).collect();
            if col.len() < 3 || ins.is_empty() {
                return None;
            }
            let m = col.iter().sum::<f64>() / col.len() as f64;
            let sd = (col.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / col.len() as f64).sqrt();
            if sd < 1e-3 {
                return None;
            }
            let z = (ins.iter().sum::<f64>() / ins.len() as f64 - m) / sd;
            let z = z * if good { 1.0 } else { -1.0 };
            Some(MARK_SCALE * (z / MARK_SCALE).tanh())
        })
    }

    /// The pooled reading of every rolling `window`-month stretch of a monthly series,
    /// per life: `Σ wᵢ · trustᵢ · zᵢ` with each witness's distance taken from its own
    /// average over the whole series and bounded as in [`Reading::zs`] (the log-odds of
    /// the rectification's pooled reading, over [`SLOPE`]).
    pub(crate) fn pooled_rolling(&self, vals: &[[[f64; 9]; NW]], window: usize) -> Vec<Vec<f64>> {
        let n = vals.len();
        (0..9)
            .map(|li| {
                if n < window {
                    return Vec::new();
                }
                let mut out = vec![0.0; n - window + 1];
                for w in WITNESSES {
                    let weight = w.weight() * self.conf(w.index(), Life::ALL[li]);
                    if weight == 0.0 {
                        continue;
                    }
                    let col: Vec<f64> = vals.iter().map(|v| v[w.index()][li]).collect();
                    let fin: Vec<f64> = col.iter().copied().filter(|x| x.is_finite()).collect();
                    if fin.len() < 3 {
                        continue;
                    }
                    let m = fin.iter().sum::<f64>() / fin.len() as f64;
                    let sd = (fin.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / fin.len() as f64).sqrt();
                    if sd < 1e-3 {
                        continue;
                    }
                    for (i, o) in out.iter_mut().enumerate() {
                        let part: Vec<f64> = col[i..i + window].iter().copied().filter(|x| x.is_finite()).collect();
                        if part.is_empty() {
                            continue;
                        }
                        let z = (part.iter().sum::<f64>() / part.len() as f64 - m) / sd;
                        *o += weight * MARK_SCALE * (z / MARK_SCALE).tanh();
                    }
                }
                out
            })
            .collect()
    }

    /// Trust in witness `w` for `life` (the KP promise; 1 for the others).
    pub(crate) fn conf(&self, w: usize, life: Life) -> f64 {
        if w == Witness::Kp.index() {
            self.kp_conf[li_of(life)]
        } else {
            1.0
        }
    }
}

/// `sky` re-read for `natal`: only the houses from the natal Moon depend on the birth.
pub(crate) fn rebased(sky: &Sky, natal: &Natal) -> Option<Sky> {
    let ms = natal.moon_sign();
    let from_moon: [u8; 9] = std::array::from_fn(|i| {
        let s = sky.facts.planet(GRAHAS[i]).sign as i64;
        ((s - ms).rem_euclid(12) + 1) as u8
    });
    (from_moon != sky.from_moon).then(|| Sky { facts: sky.facts.clone(), from_moon, moon_sidereal: sky.moon_sidereal, sun_sidereal: sky.sun_sidereal, ayanamsa_deg: sky.ayanamsa_deg })
}

/// What changes at a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoundaryKind {
    /// The rising sign (0 = Aries).
    RisingSign,
    /// The Moon's quarter-star, 0–107 (star × 4 + quarter).
    MoonPada,
    /// The navamsa (D9) rising sign.
    NavamsaRising,
    /// The KP sub-lord of the ascendant (graha index Sun = 0 … Ketu = 8).
    KpRisingSub,
}

impl BoundaryKind {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            BoundaryKind::RisingSign => "rising_sign",
            BoundaryKind::MoonPada => "moon_pada",
            BoundaryKind::NavamsaRising => "d9_rising",
            BoundaryKind::KpRisingSub => "kp_rising_sub",
        }
    }
}

/// A birth time inside the window where something in the chart changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boundary {
    /// The exact moment, to the second.
    pub at: Instant,
    /// Minutes from the recorded time.
    pub shift_min: f64,
    /// What changes.
    pub kind: BoundaryKind,
    /// Its value just before (see [`BoundaryKind`]).
    pub before: u32,
    /// Its value just after.
    pub after: u32,
}

/// The markers of a birth instant: rising sign, Moon quarter-star, D9 rising sign, the
/// ascendant's KP sub-lord (a sub split by a sign boundary keeps its lord, so only a
/// real change of lord counts), D60 rising sign.
fn markers(spk: &Spk, eop: Option<&EopTable>, obs: &Observer, ayanamsa: Ayanamsa, t: Instant) -> Option<[u32; 5]> {
    let (asc, moon) = ascendant(spk, eop, t, obs, ayanamsa)?;
    let nk = nakshatra(moon);
    Some([sign_of(asc) as u32, (nk.index * 4 + nk.pada as usize - 1) as u32, sign_of(varga(asc, 9)) as u32, gi(kp_lords(asc).sub) as u32, sign_of(varga(asc, 60)) as u32])
}

/// Every boundary within ±`window_min` minutes of `birth`, in time order, each found to
/// the second by bisection between one-minute samples; and every D60 (shashtiamsha)
/// rising-sign change, reported as instants only. BPHS counts D60 the finest varga, but
/// it changes every two minutes or so, finer than nearly any recorded birth time, so it
/// is a flag and never a witness.
pub fn boundaries(spk: &Spk, eop: Option<&EopTable>, obs: &Observer, ayanamsa: Ayanamsa, birth: Instant, window_min: f64) -> (Vec<Boundary>, Vec<Instant>) {
    let n = window_min.clamp(0.0, 180.0).floor() as i64;
    let at = |s: f64| birth.add_ns((s * 60.0 * NS_PER_SEC as f64) as i128);
    let mins: Vec<i64> = (-n..=n).collect();
    let samples: Vec<(f64, Option<[u32; 5]>)> = crate::reveal::par_map(&mins, |&m| (m as f64, markers(spk, eop, obs, ayanamsa, at(m as f64))));
    let mut out = Vec::new();
    let mut d60 = Vec::new();
    let kinds = [BoundaryKind::RisingSign, BoundaryKind::MoonPada, BoundaryKind::NavamsaRising, BoundaryKind::KpRisingSub];
    for w in samples.windows(2) {
        let ((s0, Some(m0)), (s1, Some(m1))) = (w[0], w[1]) else { continue };
        for j in 0..5 {
            if m0[j] == m1[j] {
                continue;
            }
            // Bisect to one second.
            let (mut lo, mut hi) = (s0, s1);
            while (hi - lo) * 60.0 > 1.0 {
                let mid = 0.5 * (lo + hi);
                match markers(spk, eop, obs, ayanamsa, at(mid)) {
                    Some(m) if m[j] == m0[j] => lo = mid,
                    Some(_) => hi = mid,
                    None => break,
                }
            }
            if j == 4 {
                d60.push(at(hi));
                continue;
            }
            out.push(Boundary { at: at(hi), shift_min: hi, kind: kinds[j], before: m0[j], after: m1[j] });
        }
    }
    out.sort_by(|a, b| a.shift_min.partial_cmp(&b.shift_min).unwrap_or(std::cmp::Ordering::Equal));
    (out, d60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reveal::{gain, prob, Verdict};
    use kshana_time::Utc;

    #[test]
    fn a_pooled_answer_is_a_fair_bet_against_chance() {
        // Under answers given at random (Yes or No, half each) the likelihood ratio of
        // any pooled prediction against chance averages exactly 1, so the Bayes factor
        // is a fair test (expectation 1; Markov bounds its tail).
        for logit in [-5.0, -1.3, 0.0, 0.48, 2.0, 7.0] {
            let p = prob(logit);
            let e = 0.5 * gain(Verdict::Yes, p).exp() + 0.5 * gain(Verdict::No, p).exp();
            assert!((e - 1.0).abs() < 1e-12, "{logit}: {e}");
        }
        // The weights as documented.
        let w: Vec<f64> = WITNESSES.iter().map(|w| w.weight()).collect();
        assert_eq!(w, vec![1.0, 0.0, 0.30, 0.30, 0.15, 0.5, 0.35]);
        assert_eq!(varga_of(Life::Love), Some((9, &[7u8][..])));
        assert_eq!(varga_of(Life::Family), Some((12, &[4u8, 9][..])));
        assert_eq!(varga_of(Life::Money), None);
        // The soft bound: tanh(1) at one standard deviation, never past MARK_SCALE.
        assert!((MARK_SCALE * (1.0f64 / MARK_SCALE).tanh() - 0.761_594_155_955_764_9).abs() < 1e-15);
    }

    #[test]
    fn each_boundary_is_where_its_marker_changes() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let t = std::time::Instant::now();
        let (b, d60) = boundaries(&spk, None, &obs, Ayanamsa::Lahiri, birth, 60.0);
        eprintln!("{} boundaries, {} D60 changes in {:?}", b.len(), d60.len(), t.elapsed());
        for x in &b {
            eprintln!("{:+7.2} min {:>14} {} -> {}", x.shift_min, x.kind.key(), x.before, x.after);
        }
        // Over two hours the ascendant runs some 30°: several navamsas (3°20′ each) and
        // KP subs, and a D60 change every half degree.
        assert!(b.iter().filter(|x| x.kind == BoundaryKind::NavamsaRising).count() >= 7);
        assert!(b.iter().filter(|x| x.kind == BoundaryKind::KpRisingSub).count() >= 7);
        assert!(d60.len() >= 30);
        let sec = NS_PER_SEC as i128;
        for x in &b {
            let j = match x.kind {
                BoundaryKind::RisingSign => 0,
                BoundaryKind::MoonPada => 1,
                BoundaryKind::NavamsaRising => 2,
                BoundaryKind::KpRisingSub => 3,
            };
            let m0 = markers(&spk, None, &obs, Ayanamsa::Lahiri, x.at.add_ns(-2 * sec)).unwrap();
            let m1 = markers(&spk, None, &obs, Ayanamsa::Lahiri, x.at.add_ns(sec)).unwrap();
            assert_eq!((m0[j], m1[j]), (x.before, x.after), "{x:?}");
            assert_ne!(x.before, x.after);
        }
    }
}
