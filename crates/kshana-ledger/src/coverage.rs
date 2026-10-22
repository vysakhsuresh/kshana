//! The exact chance baseline: how much of a life the engine would have called
//! "supported" anyway.
//!
//! The fit of [`score_at`](crate::score_at) is a function of time whose dasha part
//! changes only at Vimshottari boundaries, which are integer arithmetic on the birth
//! instant. A [`FitProfile`] therefore cuts a span at every mahadasha, antardasha and
//! pratyantardasha boundary from [`dasha_transitions`] and holds one [`FitSegment`]
//! per cell. Within a cell the first three funnel levels are fixed; the fourth and
//! fifth cycle through the nine lords in proportion to their years, so the share of
//! the cell during which they support is `f = Σ years of supporting lords / 120` for
//! the fourth and `f²` for both, exactly, with no sampling. The Parashari factor
//! depends on the first two lords only and is constant per cell. The transit factor
//! (Jupiter's and Saturn's houses from the Moon) is the one part sampled: once per
//! cell at its midpoint, which the docs of every number derived from it state.
//!
//! From the profile come the coverage `C` (the fraction of the span with fit at or
//! above a threshold), the mean fit (what an instant nobody logged scores on
//! average) and the fit at any instant by one binary search, which is what the
//! time-shift test and the birth-time posterior need by the thousand.
//!
//! The chance line for a tally of events then follows from the Poisson-binomial
//! distribution of hits under independence: `E = Σ C_e`, `V = Σ C_e (1 − C_e)`, and the
//! exact tail `P[K ≥ H]` from the product of the polynomials `(1 − C_e + C_e x)`.

use crate::event::{EventKind, LedgerError, LifeEvent};
use crate::score::{
    combine_fit, parashari_factor, score_event, transit_factor, transit_sample, LordTable,
    TransitSample, FUNNEL_DEPTH,
};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::{area, Area, Facts, Lang};
use kshana_time::Instant;
use kshana_trace::dasha_transitions;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari_levels, Ayanamsa, DashaLord};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

/// The fit at or above which an event counts as supported, the same 0.5 that
/// [`crate::Calibration::support_rate`] uses.
pub const SUPPORT_THRESHOLD: f64 = 0.5;
/// Coverage above which the test cannot fail for an area: its events are shown but
/// left out of the headline tally.
pub const CANNOT_FAIL_COVERAGE: f64 = 0.7;
/// The Vimshottari depth at which the profile is cut: 3, the pratyantardasha. Levels
/// 4 and 5 are handled analytically within each cell.
pub const PROFILE_DEPTH: usize = 3;

/// One cell of a [`FitProfile`]: a stretch during which the first three Vimshottari
/// lords do not change.
#[derive(Clone, Debug, PartialEq)]
pub struct FitSegment {
    /// Exact start.
    pub start: Instant,
    /// Exact end (the next cell's start).
    pub end: Instant,
    /// The lords running, mahadasha first, down to the pratyantar; shorter outside
    /// the 120-year cycle from birth.
    pub chain: Vec<DashaLord>,
    /// Consecutive supporting levels from the mahadasha among those in `chain`.
    pub agreeing: usize,
    /// `Σ years of supporting lords / 120`: the share of the cell during which the
    /// sookshma lord supports, and the square of it the share during which the prana
    /// lord supports as well.
    pub year_fraction: f64,
    /// The Parashari factor, oriented to the kind; constant over the cell.
    pub parashari: f64,
    /// The transit factor, sampled at the cell's midpoint.
    pub transit: f64,
    /// The sky at the midpoint.
    pub sample: TransitSample,
    /// Mean fit over the cell, exact for the dasha and Parashari parts.
    pub mean_fit: f64,
}

impl FitSegment {
    /// Length in nanoseconds.
    pub fn duration_ns(&self) -> i128 {
        self.end.ns_since(self.start)
    }

    /// Whether levels four and five can add to the agreement: all three cell lords
    /// support.
    fn deeper_possible(&self) -> bool {
        self.chain.len() == PROFILE_DEPTH && self.agreeing == PROFILE_DEPTH
    }

    /// The fit when `agree` consecutive levels support.
    pub fn fit_for_agreeing(&self, agree: usize) -> f64 {
        combine_fit(
            agree.min(FUNNEL_DEPTH) as f64 / FUNNEL_DEPTH as f64,
            self.parashari,
            self.transit,
        )
    }

    /// The share of the cell during which the fit is at or above `threshold`:
    /// 0 or 1 when the first three levels already fix the agreement, else 1, `f` or
    /// `f²` by which of the agreements 3, 4 and 5 reach the threshold.
    pub fn fraction_at_least(&self, threshold: f64) -> f64 {
        if !self.deeper_possible() {
            return if self.fit_for_agreeing(self.agreeing) >= threshold {
                1.0
            } else {
                0.0
            };
        }
        let f = self.year_fraction;
        if self.fit_for_agreeing(3) >= threshold {
            1.0
        } else if self.fit_for_agreeing(4) >= threshold {
            f
        } else if self.fit_for_agreeing(5) >= threshold {
            f * f
        } else {
            0.0
        }
    }

    fn compute_mean_fit(&self) -> f64 {
        if !self.deeper_possible() {
            return self.fit_for_agreeing(self.agreeing);
        }
        let f = self.year_fraction;
        // P[agree = 3] = 1 − f, P[agree = 4] = f (1 − f), P[agree = 5] = f².
        (1.0 - f) * self.fit_for_agreeing(3)
            + f * (1.0 - f) * self.fit_for_agreeing(4)
            + f * f * self.fit_for_agreeing(5)
    }
}

/// The fit of one area and kind as a piecewise function of time over a span.
#[derive(Clone, Debug, PartialEq)]
pub struct FitProfile {
    /// The area.
    pub area: Area,
    /// The kind whose orientation the fits carry.
    pub kind: EventKind,
    /// Start of the span (clipped to the birth instant).
    pub from: Instant,
    /// End of the span.
    pub to: Instant,
    /// The cells, contiguous and in order; empty when the span holds no time after birth.
    pub segments: Vec<FitSegment>,
    table: LordTable,
    moon_sidereal: f64,
    birth: Instant,
}

impl FitProfile {
    /// Length of the span in nanoseconds.
    pub fn span_ns(&self) -> i128 {
        self.to.ns_since(self.from)
    }

    /// Index of the cell containing `at`, if any.
    fn segment_index(&self, at: Instant) -> Option<usize> {
        if self.segments.is_empty() || at < self.from || at >= self.to {
            return None;
        }
        let i = self.segments.partition_point(|s| s.start <= at);
        (i > 0).then_some(i - 1)
    }

    /// The fit at an instant inside the span: the exact five-level agreement at
    /// `at`, the cell's Parashari factor and its sampled transit. `None` outside.
    pub fn fit_at(&self, at: Instant) -> Option<f64> {
        let seg = &self.segments[self.segment_index(at)?];
        let chain: Vec<DashaLord> = vimshottari_levels(
            self.moon_sidereal,
            self.birth,
            YEAR_365_25_NS,
            at,
            FUNNEL_DEPTH,
        )
        .iter()
        .map(|p| p.lord)
        .collect();
        Some(seg.fit_for_agreeing(self.table.agreeing(&chain)))
    }

    /// Mean fit over the whole span, weighted by cell length: what an instant nobody
    /// logged scores on average. 0 for an empty profile.
    pub fn mean_fit(&self) -> f64 {
        self.mean_fit_over(self.from, self.to).unwrap_or(0.0)
    }

    /// Mean fit over `[start, end)`, weighted by overlap; `None` when the window
    /// does not overlap the span.
    pub fn mean_fit_over(&self, start: Instant, end: Instant) -> Option<f64> {
        let (start, end) = if end < start {
            (end, start)
        } else {
            (start, end)
        };
        let mut num = 0.0;
        let mut den: i128 = 0;
        for s in &self.segments {
            let lo = s.start.max(start);
            let hi = s.end.min(end);
            if hi > lo {
                let d = hi.ns_since(lo);
                num += d as f64 * s.mean_fit;
                den += d;
            }
        }
        (den > 0).then(|| num / den as f64)
    }

    /// The fraction of the span during which the fit is at or above `threshold`.
    /// 0 for an empty profile.
    pub fn coverage(&self, threshold: f64) -> f64 {
        self.coverage_within(self.from, self.to, threshold)
            .unwrap_or(0.0)
    }

    /// The fraction of `[start, end)` during which the fit is at or above
    /// `threshold`; `None` when the window does not overlap the span.
    pub fn coverage_within(&self, start: Instant, end: Instant, threshold: f64) -> Option<f64> {
        let (start, end) = if end < start {
            (end, start)
        } else {
            (start, end)
        };
        let mut num = 0.0;
        let mut den: i128 = 0;
        for s in &self.segments {
            let lo = s.start.max(start);
            let hi = s.end.min(end);
            if hi > lo {
                let d = hi.ns_since(lo);
                num += d as f64 * s.fraction_at_least(threshold);
                den += d;
            }
        }
        (den > 0).then(|| num / den as f64)
    }

    /// The transit samples of the cells with their bounds, for reuse by a chart cast
    /// at a nearby birth time.
    pub fn transit_table(&self) -> TransitTable {
        TransitTable {
            rows: self
                .segments
                .iter()
                .map(|s| (s.start, s.end, s.sample))
                .collect(),
        }
    }
}

/// A piecewise table of transit samples by instant, taken from one chart's profile
/// and looked up for another cast minutes or hours away: the natal Moon moves under
/// 0.6° per hour, so Jupiter's and Saturn's houses from it are the same except at a
/// sign edge, and the table spares a birth-time scan one ephemeris call per cell per
/// candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitTable {
    rows: Vec<(Instant, Instant, TransitSample)>,
}

impl TransitTable {
    /// The sample of the row containing `at`, or the nearest row's when `at` lies
    /// outside the table; `None` for an empty table.
    pub fn lookup(&self, at: Instant) -> Option<TransitSample> {
        if self.rows.is_empty() {
            return None;
        }
        let i = self.rows.partition_point(|r| r.0 <= at);
        Some(self.rows[i.saturating_sub(1)].2)
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Build a profile with the sky sampled by `sampler` at each cell's midpoint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fit_profile_with(
    facts: &Facts,
    moon_sidereal: f64,
    birth: Instant,
    a: Area,
    kind: EventKind,
    from: Instant,
    to: Instant,
    sampler: &mut dyn FnMut(Instant) -> Result<TransitSample, LedgerError>,
) -> Result<FitProfile, LedgerError> {
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let from = from.max(birth);
    let table = LordTable::new(facts, a, kind.is_adverse());
    let mut profile = FitProfile {
        area: a,
        kind,
        from,
        to,
        segments: Vec::new(),
        table,
        moon_sidereal,
        birth,
    };
    if to <= from {
        return Ok(profile);
    }
    let mut bounds = vec![from];
    for t in dasha_transitions(
        moon_sidereal,
        birth,
        YEAR_365_25_NS,
        PROFILE_DEPTH,
        from,
        to,
    ) {
        if t.at > from && t.at < to && bounds.last() != Some(&t.at) {
            bounds.push(t.at);
        }
    }
    bounds.push(to);
    let f = profile.table.year_fraction();
    for w in bounds.windows(2) {
        let (start, end) = (w[0], w[1]);
        if end <= start {
            continue;
        }
        let mid = start.add_ns(end.ns_since(start) / 2);
        let chain: Vec<DashaLord> =
            vimshottari_levels(moon_sidereal, birth, YEAR_365_25_NS, mid, PROFILE_DEPTH)
                .iter()
                .map(|p| p.lord)
                .collect();
        let agreeing = profile.table.agreeing(&chain);
        let (parashari, _) = parashari_factor(facts, a, kind, &chain);
        let sample = sampler(mid)?;
        let transit = transit_factor(&sample, a, kind);
        let mut seg = FitSegment {
            start,
            end,
            chain,
            agreeing,
            year_fraction: f,
            parashari,
            transit,
            sample,
            mean_fit: 0.0,
        };
        seg.mean_fit = seg.compute_mean_fit();
        profile.segments.push(seg);
    }
    Ok(profile)
}

/// A memo of transit samples by instant for one chart, so the profiles of several
/// areas, which share their cell boundaries, share their ephemeris calls.
pub(crate) struct TransitCache<'a> {
    spk: &'a Spk,
    natal: &'a Chart,
    ayanamsa: Ayanamsa,
    memo: HashMap<i128, TransitSample>,
}

impl<'a> TransitCache<'a> {
    pub(crate) fn new(spk: &'a Spk, natal: &'a Chart, ayanamsa: Ayanamsa) -> Self {
        Self {
            spk,
            natal,
            ayanamsa,
            memo: HashMap::new(),
        }
    }

    pub(crate) fn sample(&mut self, at: Instant) -> Result<TransitSample, LedgerError> {
        if let Some(s) = self.memo.get(&at.tai_ns()) {
            return Ok(*s);
        }
        let s = transit_sample(self.spk, self.natal, self.ayanamsa, at)?;
        self.memo.insert(at.tai_ns(), s);
        Ok(s)
    }

    /// Profile for an area name and kind over a span, through this cache.
    pub(crate) fn profile(
        &mut self,
        facts: &Facts,
        area_name: &str,
        kind: EventKind,
        from: Instant,
        to: Instant,
    ) -> Result<FitProfile, LedgerError> {
        let a = area(area_name).ok_or_else(|| LedgerError::UnknownArea(area_name.to_string()))?;
        let moon = self.natal.bodies[1].sidereal;
        let birth = self.natal.instant;
        fit_profile_with(facts, moon, birth, a, kind, from, to, &mut |t| {
            self.sample(t)
        })
    }
}

/// Profiles for every (area, orientation) a ledger needs, built once each.
pub(crate) struct ProfileSet {
    profiles: HashMap<(String, bool), FitProfile>,
}

impl ProfileSet {
    pub(crate) fn build(
        cache: &mut TransitCache<'_>,
        facts: &Facts,
        events: &[LifeEvent],
        from: Instant,
        to: Instant,
    ) -> Result<Self, LedgerError> {
        let mut profiles = HashMap::new();
        for ev in events {
            let key = (ev.area.clone(), ev.kind.is_adverse());
            if let Entry::Vacant(slot) = profiles.entry(key) {
                slot.insert(cache.profile(facts, &ev.area, ev.kind, from, to)?);
            }
        }
        Ok(Self { profiles })
    }

    pub(crate) fn get(&self, ev: &LifeEvent) -> &FitProfile {
        &self.profiles[&(ev.area.clone(), ev.kind.is_adverse())]
    }
}

/// The fit of an area and kind as a piecewise function of time over `[from, to]`,
/// clipped to start no earlier than birth.
pub fn fit_profile(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    area_name: &str,
    kind: EventKind,
    from: Instant,
    to: Instant,
) -> Result<FitProfile, LedgerError> {
    let facts = Facts::from_chart(natal);
    let mut cache = TransitCache::new(spk, natal, ayanamsa);
    cache.profile(&facts, area_name, kind, from, to)
}

/// How much of a span the engine would have called supported for an area and kind.
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    /// The fraction of the span with fit at or above `threshold`, in `[0, 1]`.
    pub fraction: f64,
    /// Pratyantar cells scored (one transit sample each).
    pub periods_scored: usize,
    /// The threshold used.
    pub threshold: f64,
    /// Mean fit over the span.
    pub mean_fit: f64,
    /// `fraction > CANNOT_FAIL_COVERAGE`: the test cannot fail for this area.
    pub cannot_fail: bool,
    /// The result in words.
    pub words: String,
}

fn percent(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

/// Coverage of an area and kind over `[from, to]` at a threshold.
#[allow(clippy::too_many_arguments)]
pub fn coverage(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    area_name: &str,
    kind: EventKind,
    from: Instant,
    to: Instant,
    threshold: f64,
) -> Result<Coverage, LedgerError> {
    let p = fit_profile(spk, natal, ayanamsa, area_name, kind, from, to)?;
    Ok(coverage_of(&p, threshold))
}

/// Coverage read from a profile already built.
pub fn coverage_of(p: &FitProfile, threshold: f64) -> Coverage {
    coverage_of_in(p, threshold, Lang::En)
}

/// [`coverage_of`], its words in the reader's language.
pub fn coverage_of_in(p: &FitProfile, threshold: f64, lang: Lang) -> Coverage {
    let fraction = p.coverage(threshold);
    let mean_fit = p.mean_fit();
    let cannot_fail = fraction > CANNOT_FAIL_COVERAGE;
    let orientation = if p.kind.is_adverse() {
        "adverse"
    } else {
        "favourable"
    };
    let area = kshana_rules::lang::area_name(lang, p.area.name);
    let (cells, frac) = (p.segments.len(), percent(fraction));
    let words = if lang == Lang::Ml {
        let o = if p.kind.is_adverse() { "പ്രതികൂല" } else { "അനുകൂല" };
        if p.segments.is_empty() {
            format!("ഈ കാലയളവിൽ ജനനത്തിനു ശേഷമുള്ള സമയമില്ല, അതിനാൽ {area} ({o}) എന്നതിന് കവറേജില്ല.")
        } else if cannot_fail {
            format!("നിങ്ങളുടെ ജാതകത്തിൽ {area} എന്നതിന് ഈ പരിശോധന തോൽക്കില്ല: {o} വായന കാലയളവിന്റെ {frac} സമയത്ത് കുറഞ്ഞത് {threshold:.2} നേടി ({cells} കൃത്യ പ്രത്യന്തര ഖണ്ഡങ്ങൾ, ശരാശരി ചേർച്ച {mean_fit:.2}; ഗോചരഭാഗം ഓരോ ഖണ്ഡത്തിലും ഒരിക്കൽ അളന്നു).")
        } else {
            format!("{area} ({o}) കാലയളവിന്റെ {frac} സമയത്ത് കുറഞ്ഞത് {threshold:.2} നേടി ({cells} കൃത്യ പ്രത്യന്തര ഖണ്ഡങ്ങൾ, ശരാശരി ചേർച്ച {mean_fit:.2}; ഗോചരഭാഗം ഓരോ ഖണ്ഡത്തിലും ഒരിക്കൽ അളന്നു), അതിനാൽ ഏതെങ്കിലുമൊരു നിമിഷത്തിന് ഏകദേശം {frac} സമയം പിന്തുണ കിട്ടും.")
        }
    } else if lang == Lang::Hi {
        let o = if p.kind.is_adverse() { "प्रतिकूल" } else { "अनुकूल" };
        if p.segments.is_empty() {
            format!("इस अवधि में जन्म के बाद का कोई समय नहीं है, इसलिए {area} ({o}) का कोई कवरेज नहीं।")
        } else if cannot_fail {
            format!("आपकी कुंडली में {area} के लिए यह परीक्षा विफल नहीं हो सकती: {o} पढ़त अवधि के {frac} समय में कम से कम {threshold:.2} रही ({cells} सटीक प्रत्यंतर खंड, औसत मेल {mean_fit:.2}; गोचर वाला हिस्सा हर खंड में एक बार मापा गया)।")
        } else {
            format!("{area} ({o}) अवधि के {frac} समय में कम से कम {threshold:.2} रहा ({cells} सटीक प्रत्यंतर खंड, औसत मेल {mean_fit:.2}; गोचर वाला हिस्सा हर खंड में एक बार मापा गया), इसलिए किसी भी पल को लगभग {frac} समय समर्थन मिलता है।")
        }
    } else if p.segments.is_empty() {
        format!(
            "The span holds no time after birth, so {} ({orientation}) has no coverage.",
            p.area.name
        )
    } else if cannot_fail {
        format!(
            "This test cannot fail for {} in your chart: the {orientation} reading scored at least {threshold:.2} during {} of the span ({} exact pratyantar cells, mean fit {mean_fit:.2}; the transit part sampled once per cell).",
            p.area.name,
            percent(fraction),
            p.segments.len()
        )
    } else {
        format!(
            "{} ({orientation}) scored at least {threshold:.2} during {} of the span ({} exact pratyantar cells, mean fit {mean_fit:.2}; the transit part sampled once per cell), so a random instant is supported about {} of the time.",
            p.area.name,
            percent(fraction),
            p.segments.len(),
            percent(fraction)
        )
    };
    Coverage {
        fraction,
        periods_scored: p.segments.len(),
        threshold,
        mean_fit,
        cannot_fail,
        words,
    }
}

/// The distribution of the number of hits among independent events with their own
/// chances: the Poisson-binomial distribution.
#[derive(Clone, Debug, PartialEq)]
pub struct PoissonBinomial {
    /// `pmf[k]` = probability of exactly `k` hits, `k = 0..=n`.
    pub pmf: Vec<f64>,
    /// `Σ p_e`.
    pub mean: f64,
    /// `Σ p_e (1 − p_e)`.
    pub variance: f64,
}

impl PoissonBinomial {
    /// From the per-event chances, each clamped to `[0, 1]`. The mass function is
    /// the product of the polynomials `(1 − p_e + p_e x)`.
    pub fn new(chances: &[f64]) -> Self {
        let mut pmf = vec![1.0];
        let mut mean = 0.0;
        let mut variance = 0.0;
        for p in chances {
            let p = if p.is_finite() {
                p.clamp(0.0, 1.0)
            } else {
                0.0
            };
            mean += p;
            variance += p * (1.0 - p);
            let mut next = vec![0.0; pmf.len() + 1];
            for (k, mass) in pmf.iter().enumerate() {
                next[k] += mass * (1.0 - p);
                next[k + 1] += mass * p;
            }
            pmf = next;
        }
        Self {
            pmf,
            mean,
            variance,
        }
    }

    /// Standard deviation.
    pub fn sd(&self) -> f64 {
        self.variance.sqrt()
    }

    /// `P[K ≥ k]`.
    pub fn p_at_least(&self, k: usize) -> f64 {
        self.pmf.iter().skip(k).sum::<f64>().clamp(0.0, 1.0)
    }

    /// `P[K ≤ k]`.
    pub fn p_at_most(&self, k: usize) -> f64 {
        self.pmf.iter().take(k + 1).sum::<f64>().clamp(0.0, 1.0)
    }
}

/// The Poisson-binomial expectation of hits for events with the given coverages.
pub fn expected_hits(coverages: &[f64]) -> PoissonBinomial {
    PoissonBinomial::new(coverages)
}

/// "about 1 time in N" for a probability, or "almost always"/"almost never".
pub fn chance_words(p: f64) -> String {
    chance_words_in(p, Lang::En)
}

/// [`chance_words`] in the reader's language.
pub fn chance_words_in(p: f64, lang: Lang) -> String {
    if lang != Lang::En {
        return if p.is_nan() || p <= 0.0 {
            lang.pick(["", "യാദൃച്ഛികമായി ഏതാണ്ട് ഒരിക്കലുമില്ല", "संयोग से लगभग कभी नहीं"]).to_string()
        } else if p >= 0.5 {
            lang.pick(["", "യാദൃച്ഛികമായി പകുതിയിലേറെ തവണ", "संयोग से आधे से ज़्यादा बार"]).to_string()
        } else {
            let n = (1.0 / p).round();
            match lang {
                Lang::Ml => format!("യാദൃച്ഛികമായി ഏകദേശം {n:.0}-ൽ ഒരിക്കൽ"),
                _ => format!("संयोग से लगभग {n:.0} में 1 बार"),
            }
        };
    }
    if p.is_nan() || p <= 0.0 {
        "almost never by chance".to_string()
    } else if p >= 0.5 {
        "more often than not by chance".to_string()
    } else {
        format!("about 1 time in {:.0} by chance", (1.0 / p).round())
    }
}

/// One event in a tally.
#[derive(Clone, Debug, PartialEq)]
pub struct EventCoverage {
    /// The area.
    pub area: String,
    /// The kind.
    pub kind: EventKind,
    /// When.
    pub at: Instant,
    /// The fit read from the profile at the instant (exact dasha levels, the
    /// transit from the cell's midpoint); the exact score when the instant lies
    /// outside the span.
    pub fit: f64,
    /// The fit of [`score_event`] at the exact instant.
    pub exact_fit: f64,
    /// Coverage of the event's area and orientation over the span.
    pub coverage: f64,
    /// `fit ≥ threshold`.
    pub supported: bool,
    /// The area's coverage exceeds [`CANNOT_FAIL_COVERAGE`]: shown, not counted.
    pub cannot_fail: bool,
}

/// The headline tally with its chance line.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    /// Events supported, among those counted.
    pub supported: usize,
    /// Events counted (logged events less those excluded).
    pub total: usize,
    /// Expected hits by chance, `Σ C_e` over the counted events.
    pub expected: f64,
    /// Standard deviation of the chance hits, `√Σ C_e (1 − C_e)`.
    pub sd: f64,
    /// Exact Poisson-binomial `P[K ≥ supported]`; 1 when nothing is counted.
    pub p_value: f64,
    /// Events left out of the headline because their area cannot fail.
    pub excluded: usize,
    /// The threshold used.
    pub threshold: f64,
    /// Every event, counted or not.
    pub per_event: Vec<EventCoverage>,
    /// Coverage of each (area, orientation) used, in words.
    pub coverage_words: Vec<String>,
    /// The headline in words.
    pub words: String,
}

/// Tally a ledger against its exact chance line over `[from, to]`.
pub fn tally(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    events: &[LifeEvent],
    from: Instant,
    to: Instant,
    threshold: f64,
) -> Result<Tally, LedgerError> {
    tally_in(spk, natal, ayanamsa, events, from, to, threshold, Lang::En)
}

/// [`tally`], its words in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn tally_in(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    events: &[LifeEvent],
    from: Instant,
    to: Instant,
    threshold: f64,
    lang: Lang,
) -> Result<Tally, LedgerError> {
    let facts = Facts::from_chart(natal);
    let mut cache = TransitCache::new(spk, natal, ayanamsa);
    let set = ProfileSet::build(&mut cache, &facts, events, from, to)?;
    let mut per_event = Vec::with_capacity(events.len());
    let mut coverages: HashMap<(String, bool), Coverage> = HashMap::new();
    for ev in events {
        let p = set.get(ev);
        let key = (ev.area.clone(), ev.kind.is_adverse());
        let c = coverages
            .entry(key)
            .or_insert_with(|| coverage_of_in(p, threshold, lang))
            .clone();
        let exact_fit = score_event(spk, natal, ayanamsa, ev)?.fit;
        let fit = p.fit_at(ev.at).unwrap_or(exact_fit);
        per_event.push(EventCoverage {
            area: ev.area.clone(),
            kind: ev.kind,
            at: ev.at,
            fit,
            exact_fit,
            coverage: c.fraction,
            supported: fit >= threshold,
            cannot_fail: c.cannot_fail,
        });
    }
    let counted: Vec<&EventCoverage> = per_event.iter().filter(|e| !e.cannot_fail).collect();
    let supported = counted.iter().filter(|e| e.supported).count();
    let total = counted.len();
    let excluded = per_event.len() - total;
    let pb = expected_hits(&counted.iter().map(|e| e.coverage).collect::<Vec<_>>());
    let p_value = if total == 0 {
        1.0
    } else {
        pb.p_at_least(supported)
    };
    let mut keys: Vec<&(String, bool)> = coverages.keys().collect();
    keys.sort();
    let coverage_words: Vec<String> = keys.iter().map(|k| coverages[*k].words.clone()).collect();
    if lang != Lang::En {
        let ml = lang == Lang::Ml;
        let mut words = if events.is_empty() {
            lang.pick(["", "തീയതിയുള്ള സംഭവങ്ങളൊന്നും ഇതുവരെ ചേർത്തിട്ടില്ല, അതിനാൽ എണ്ണാൻ ഒന്നുമില്ല.", "अभी कोई तारीख़ वाली घटना दर्ज नहीं, इसलिए गिनने को कुछ नहीं।"]).to_string()
        } else if total == 0 {
            let n = events.len();
            if ml {
                format!("തീയതിയുള്ള {n} സംഭവങ്ങളും നിങ്ങളുടെ ജാതകത്തിൽ ഈ പരിശോധന തോൽക്കാത്ത മേഖലകളിലാണ്, അതിനാൽ പ്രധാന ഫലമില്ല.")
            } else {
                format!("तारीख़ वाली सभी {n} घटनाएँ उन क्षेत्रों में हैं जहाँ आपकी कुंडली में यह परीक्षा विफल नहीं हो सकती, इसलिए कोई मुख्य नतीजा नहीं।")
            }
        } else {
            let (m, sd, cw) = (pb.mean, pb.sd(), chance_words_in(p_value, lang));
            if ml {
                format!("തീയതിയുള്ള {total} സംഭവങ്ങളിൽ {supported} എണ്ണത്തെ അപ്പോൾ നടന്നിരുന്ന ദശകൾ പിന്തുണച്ചു; യാദൃച്ഛികത മാത്രം {m:.0} ± {sd:.0} തരും; ഇത് {cw} സംഭവിക്കുന്നു (p = {p_value:.3}).")
            } else {
                format!("तारीख़ वाली {total} घटनाओं में से {supported} को उस समय चल रही दशाओं ने समर्थन दिया; सिर्फ़ संयोग {m:.0} ± {sd:.0} देता; ऐसा {cw} होता है (p = {p_value:.3})।")
            }
        };
        if excluded > 0 {
            let mut areas: Vec<String> = per_event.iter().filter(|e| e.cannot_fail).map(|e| kshana_rules::lang::area_name(lang, &e.area)).collect();
            areas.sort();
            areas.dedup();
            let pc = percent(CANNOT_FAIL_COVERAGE);
            if ml {
                words.push_str(&format!(" {} എന്നിവയിലെ {excluded} സംഭവങ്ങൾ പ്രധാന ഫലത്തിൽ നിന്ന് ഒഴിവാക്കി: നിങ്ങളുടെ ജാതകത്തിൽ ആ മേഖലകൾക്ക് ഈ പരിശോധന തോൽക്കില്ല (കവറേജ് {pc}-ൽ കൂടുതൽ).", areas.join(", ")));
            } else {
                words.push_str(&format!(" {} की {excluded} घटनाएँ मुख्य नतीजे से बाहर रखी गईं: आपकी कुंडली में उन क्षेत्रों के लिए यह परीक्षा विफल नहीं हो सकती (कवरेज {pc} से ऊपर)।", areas.join(" और ")));
            }
        }
        if total > 0 && total < 10 {
            words.push_str(lang.pick(["", " എണ്ണിയ സംഭവങ്ങൾ പത്തിൽ കുറവായതിനാൽ യാദൃച്ഛികതയുടെ പരിധി വിശാലമാണ്.", " दस से कम गिनी गई घटनाओं में संयोग की सीमा चौड़ी है।"]));
        }
        return Ok(Tally { supported, total, expected: pb.mean, sd: pb.sd(), p_value, excluded, threshold, per_event, coverage_words, words });
    }
    let mut words = if events.is_empty() {
        "No dated events logged yet, so there is nothing to tally.".to_string()
    } else if total == 0 {
        format!(
            "All {} dated events fall in areas this test cannot fail for in your chart, so there is no headline.",
            events.len()
        )
    } else {
        format!(
            "Of {total} dated event{}, the running periods supported {supported}; chance alone would give {:.0} ± {:.0}; this happens {} (p = {p_value:.3}).",
            if total == 1 { "" } else { "s" },
            pb.mean,
            pb.sd(),
            chance_words(p_value)
        )
    };
    if excluded > 0 {
        let mut areas: Vec<String> = per_event
            .iter()
            .filter(|e| e.cannot_fail)
            .map(|e| e.area.clone())
            .collect();
        areas.sort();
        areas.dedup();
        words.push_str(&format!(
            " {excluded} event{} in {} left out of the headline: this test cannot fail for {} in your chart (coverage above {}).",
            if excluded == 1 { " was" } else { "s were" },
            areas.join(" and "),
            if areas.len() == 1 { "that area" } else { "those areas" },
            percent(CANNOT_FAIL_COVERAGE)
        ));
    }
    if total > 0 && total < 10 {
        words.push_str(" With fewer than ten counted events the chance line is wide.");
    }
    Ok(Tally {
        supported,
        total,
        expected: pb.mean,
        sd: pb.sd(),
        p_value,
        excluded,
        threshold,
        per_event,
        coverage_words,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::baseline_fits_for;
    use crate::fixture::{kochi, noon};
    use crate::score::score_at;

    #[test]
    fn poisson_binomial_is_exact() {
        let pb = PoissonBinomial::new(&[0.5, 0.5, 0.5]);
        assert_eq!(pb.pmf.len(), 4);
        for (k, want) in [0.125, 0.375, 0.375, 0.125].iter().enumerate() {
            assert!((pb.pmf[k] - want).abs() < 1e-12);
        }
        assert!((pb.mean - 1.5).abs() < 1e-12);
        assert!((pb.variance - 0.75).abs() < 1e-12);
        assert!((pb.p_at_least(2) - 0.5).abs() < 1e-12);
        assert!((pb.p_at_most(0) - 0.125).abs() < 1e-12);
        assert!((pb.p_at_least(0) - 1.0).abs() < 1e-12);
        assert_eq!(pb.p_at_least(4), 0.0);
        let mixed = expected_hits(&[0.1, 0.9, 0.3, 2.0, -1.0, f64::NAN]);
        assert!((mixed.mean - 2.3).abs() < 1e-12);
        assert!((mixed.pmf.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((mixed.variance - (0.09 + 0.09 + 0.21)).abs() < 1e-12);
        assert!(chance_words(0.05).contains("1 time in 20"));
        assert!(chance_words(0.0).contains("almost never"));
        assert!(chance_words(0.7).contains("more often"));
    }

    #[test]
    fn kernel_coverage_matches_the_sampled_baseline() {
        let Some((spk, ch)) = kochi() else { return };
        let from = noon(1995, 1, 1);
        let to = noon(2025, 1, 1);
        for (area_name, kind) in [
            ("Career", EventKind::Began),
            ("Marriage", EventKind::Milestone),
            ("Health", EventKind::Crisis),
        ] {
            let c = coverage(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                area_name,
                kind,
                from,
                to,
                SUPPORT_THRESHOLD,
            )
            .unwrap();
            assert!((0.0..=1.0).contains(&c.fraction), "{c:?}");
            assert!((0.0..=1.0).contains(&c.mean_fit), "{c:?}");
            assert!(c.periods_scored >= 100, "{}", c.periods_scored);
            assert!(c.words.contains(area_name), "{}", c.words);
            let sampled =
                baseline_fits_for(&spk, &ch, Ayanamsa::Lahiri, area_name, kind, from, to, 200)
                    .unwrap();
            let sampled_fraction =
                sampled.iter().filter(|f| **f >= SUPPORT_THRESHOLD).count() as f64 / 200.0;
            assert!(
                (c.fraction - sampled_fraction).abs() < 0.15,
                "{area_name}: exact {} vs sampled {sampled_fraction}",
                c.fraction
            );
            let sampled_mean = sampled.iter().sum::<f64>() / 200.0;
            assert!(
                (c.mean_fit - sampled_mean).abs() < 0.1,
                "{area_name}: exact mean {} vs sampled {sampled_mean}",
                c.mean_fit
            );
        }
    }

    #[test]
    fn kernel_profile_is_contiguous_and_agrees_with_score_at() {
        let Some((spk, ch)) = kochi() else { return };
        let from = noon(2005, 1, 1);
        let to = noon(2015, 1, 1);
        let p = fit_profile(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            EventKind::Gain,
            from,
            to,
        )
        .unwrap();
        assert_eq!(p.from, from);
        assert_eq!(p.to, to);
        assert_eq!(p.segments.first().unwrap().start, from);
        assert_eq!(p.segments.last().unwrap().end, to);
        for w in p.segments.windows(2) {
            assert_eq!(w[0].end, w[1].start);
            // a new cell means a different pratyantar chain
            assert_ne!(w[0].chain, w[1].chain);
        }
        let facts = Facts::from_chart(&ch);
        let a = area("Career").unwrap();
        for s in &p.segments {
            assert_eq!(s.chain.len(), PROFILE_DEPTH);
            assert!((0.0..=1.0).contains(&s.mean_fit));
            assert!((0.0..=1.0).contains(&s.year_fraction));
            for thr in [0.0, 0.3, 0.5, 0.7, 1.01] {
                assert!((0.0..=1.0).contains(&s.fraction_at_least(thr)));
            }
            assert_eq!(s.fraction_at_least(0.0), 1.0);
            assert_eq!(s.fraction_at_least(1.01), 0.0);
            // the mean lies between the fits of the least and most agreement
            let lo = s.fit_for_agreeing(s.agreeing);
            let hi = s.fit_for_agreeing(5);
            assert!(s.mean_fit >= lo - 1e-12 && s.mean_fit <= hi + 1e-12);
            // at the midpoint the profile fit matches score_at except through the
            // transit, which the profile sampled at that very instant: so exactly.
            let mid = s.start.add_ns(s.duration_ns() / 2);
            let exact =
                score_at(&spk, &ch, &facts, Ayanamsa::Lahiri, a, EventKind::Gain, mid).unwrap();
            let fit = p.fit_at(mid).unwrap();
            assert!((fit - exact.fit).abs() < 1e-12, "{fit} vs {}", exact.fit);
            assert_eq!(s.sample.jupiter_from_moon, exact.jupiter_from_moon);
            assert_eq!(s.sample.saturn_from_moon, exact.saturn_from_moon);
        }
        assert!(p.fit_at(from.add_ns(-1)).is_none());
        assert!(p.fit_at(to).is_none());
        assert!(p.fit_at(from).is_some());
        let m = p.mean_fit();
        assert!((0.0..=1.0).contains(&m));
        assert!((p.mean_fit_over(from, to).unwrap() - m).abs() < 1e-12);
        assert!(p.mean_fit_over(to, to.add_ns(1)).is_none());
        let c_half = p.coverage(0.5);
        assert!(p.coverage(0.3) >= c_half && c_half >= p.coverage(0.7));
        assert_eq!(p.coverage(0.0), 1.0);
        assert!((p.coverage_within(from, to, 0.5).unwrap() - c_half).abs() < 1e-12);
        let t = p.transit_table();
        assert_eq!(t.len(), p.segments.len());
        assert_eq!(t.lookup(from.add_ns(-1)).unwrap(), p.segments[0].sample);
        assert_eq!(
            t.lookup(to.add_ns(1)).unwrap(),
            p.segments.last().unwrap().sample
        );
        // A span before birth is empty.
        let empty = fit_profile(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Career",
            EventKind::Gain,
            noon(1980, 1, 1),
            noon(1985, 1, 1),
        )
        .unwrap();
        assert!(empty.segments.is_empty());
        assert_eq!(empty.mean_fit(), 0.0);
        assert!(coverage_of(&empty, 0.5)
            .words
            .contains("no time after birth"));
        assert!(fit_profile(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            "Pets",
            EventKind::Gain,
            from,
            to
        )
        .is_err());
    }

    #[test]
    fn kernel_tally_reads_the_chance_line() {
        let Some((spk, ch)) = kochi() else { return };
        let events = vec![
            LifeEvent::new(noon(2013, 7, 1), "Career", EventKind::Began, 0.9, "job").unwrap(),
            LifeEvent::new(
                noon(2018, 11, 22),
                "Marriage",
                EventKind::Milestone,
                1.0,
                "wedding",
            )
            .unwrap(),
            LifeEvent::new(noon(2020, 6, 15), "Health", EventKind::Crisis, 0.7, "ill").unwrap(),
            LifeEvent::new(
                noon(2022, 3, 1),
                "Career",
                EventKind::Gain,
                0.5,
                "promotion",
            )
            .unwrap(),
        ];
        let t = tally(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &events,
            noon(2005, 1, 1),
            noon(2026, 1, 1),
            SUPPORT_THRESHOLD,
        )
        .unwrap();
        assert_eq!(t.per_event.len(), 4);
        assert_eq!(t.total + t.excluded, 4);
        assert!(t.supported <= t.total);
        let sum_cov: f64 = t
            .per_event
            .iter()
            .filter(|e| !e.cannot_fail)
            .map(|e| e.coverage)
            .sum();
        assert!((t.expected - sum_cov).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&t.p_value));
        assert!(t.sd >= 0.0);
        assert_eq!(t.coverage_words.len(), 3);
        assert!(!t.words.is_empty());
        for e in &t.per_event {
            assert!((0.0..=1.0).contains(&e.fit));
            assert!((0.0..=1.0).contains(&e.exact_fit));
            assert_eq!(e.supported, e.fit >= SUPPORT_THRESHOLD);
            assert_eq!(e.cannot_fail, e.coverage > CANNOT_FAIL_COVERAGE);
        }
        let none = tally(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &[],
            noon(2005, 1, 1),
            noon(2026, 1, 1),
            0.5,
        )
        .unwrap();
        assert_eq!(none.total, 0);
        assert_eq!(none.p_value, 1.0);
        assert!(none.words.contains("nothing to tally"));
    }
}
