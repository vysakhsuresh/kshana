//! Birth-time rectification from dated events.
//!
//! Two rankings live here.
//!
//! [`rectify`]: the person names a few things that happened and when (a marriage, a
//! move, a loss, a job); every candidate birth time inside the stated window is cast and
//! scored by how well its periods and slow sky explain those days: a good event should
//! fall on a high slow score for its life with the Vimshottari funnel carrying the life's
//! houses, a hard one on a low score with the funnel silent. The best-fitting times are
//! returned with what changes between them (rising sign, Moon's star, the dasha at
//! birth), so the choice is informed, not blind.
//!
//! [`rectify_by_areas`]: the birth window is scanned in steps. For every candidate
//! instant the chart is cast and each known event is scored by how well that chart
//! explains it: the KP promise of the event's area, the number of Vimshottari levels
//! whose lords signify the area at the event's date, and the Parashari quality of the
//! running antardasha lord. The candidate's score is the mean over events. Candidates are
//! returned sorted, each with the Ascendant, its KP sub-lord (the quantity rectification
//! actually fixes) and the per-event reasoning, so an expert sees why one time is
//! preferred over another.
//!
//! Both are a ranking of hypotheses, not a measurement: with few events many times score
//! alike, and the screen says so.

use crate::areas::Area;
use crate::facts::Facts;
use crate::forecast::{sky, slow_scores, Life, Natal};
use crate::kp::{agreeing_levels, promise, timing};
use crate::lang;
use crate::parashari::period_quality;
use crate::plain::Lang;
use kshana_chart::{compute, ChartError, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, vimshottari_levels, Ayanamsa, DashaLord};

/// Something that happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifeEvent {
    /// When.
    pub at: Instant,
    /// Which life it belongs to.
    pub life: Life,
    /// True for a good event, false for a hard one.
    pub good: bool,
}

/// One candidate birth time.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The birth instant.
    pub birth: Instant,
    /// Fit in `[0, 1]`.
    pub fit: f64,
    /// Rising sign, 0 = Aries.
    pub lagna_sign: u8,
    /// The Moon's star, 0 = Ashwini.
    pub moon_star: usize,
    /// The mahadasha running at birth.
    pub dasha_at_birth: DashaLord,
    /// How many events it explains (fit ≥ 0.5 each).
    pub explained: usize,
    /// In words.
    pub words: String,
}

/// Score `window_start..=window_end` at up to `max_candidates` evenly spaced times.
#[allow(clippy::too_many_arguments)]
pub fn rectify(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    window_start: Instant,
    window_end: Instant,
    events: &[LifeEvent],
    max_candidates: usize,
) -> Vec<Candidate> {
    rectify_in(spk, eop, obs, system, ayanamsa, window_start, window_end, events, max_candidates, Lang::En)
}

/// [`rectify`], its words in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn rectify_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    window_start: Instant,
    window_end: Instant,
    events: &[LifeEvent],
    max_candidates: usize,
    lang: Lang,
) -> Vec<Candidate> {
    let span = window_end.ns_since(window_start).max(0);
    let n = if span == 0 { 1 } else { max_candidates.clamp(3, 241) };
    let step = if n > 1 { span / (n as i128 - 1) } else { 0 };
    // The sky of each event is the same for every candidate: cast it once.
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let birth = window_start.add_ns(step * k as i128);
        let Ok(natal) = Natal::compute(spk, eop, birth, obs, system, ayanamsa) else {
            continue;
        };
        let mut total = 0.0;
        let mut explained = 0;
        for e in events {
            let Ok(s) = sky(spk, eop, &natal, obs, HouseSystem::WholeSign, ayanamsa, e.at) else {
                continue;
            };
            let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, e.at, 3);
            let idx = Life::ALL.iter().position(|l| *l == e.life).unwrap_or(0);
            let slow = slow_scores(&natal, &s, &levels)[idx];
            let funnel = timing(&natal.facts, e.life.area(), natal.moon_sidereal, natal.birth, e.at, 3);
            let agree = agreeing_levels(&funnel) as f64 / 3.0;
            let f = if e.good {
                0.6 * slow + 0.4 * agree
            } else {
                0.6 * (1.0 - slow) + 0.4 * (1.0 - agree)
            };
            if f >= 0.5 {
                explained += 1;
            }
            total += f;
        }
        let fit = if events.is_empty() { 0.5 } else { total / events.len() as f64 };
        let lagna_sign = natal.facts.lagna_sign;
        let moon_star = natal.janma();
        let dasha_at_birth = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, natal.birth.add_ns(NS_PER_SEC), 1)
            .first()
            .map(|p| p.lord)
            .unwrap_or(DashaLord::Moon);
        out.push(Candidate {
            birth,
            fit,
            lagna_sign,
            moon_star,
            dasha_at_birth,
            explained,
            words: {
                let (e, n, p) = (explained, events.len(), fit * 100.0);
                match lang {
                    Lang::En => format!("This birth time explains {e} of the {n} things you marked, a {p:.0}% fit"),
                    Lang::Ml => format!("നിങ്ങൾ അടയാളപ്പെടുത്തിയ {n} കാര്യങ്ങളിൽ {e} എണ്ണം ഈ ജനനസമയം വിശദീകരിക്കുന്നു, {p:.0}% ചേർച്ച"),
                    Lang::Hi => format!("आपकी बताई {n} बातों में से {e} को यह जन्म-समय समझाता है, {p:.0}% मेल"),
                }
            },
        });
    }
    out.sort_by(|a, b| b.fit.partial_cmp(&a.fit).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// The distinct rising signs among the candidates, best fit first, with the best
/// candidate of each: what the window actually lets vary.
pub fn by_rising_sign(candidates: &[Candidate]) -> Vec<Candidate> {
    let mut seen: Vec<u8> = Vec::new();
    let mut v = Vec::new();
    for c in candidates {
        if !seen.contains(&c.lagna_sign) {
            seen.push(c.lagna_sign);
            v.push(c.clone());
        }
    }
    v
}

#[allow(dead_code)]
fn star_name(i: usize) -> &'static str {
    NAKSHATRA_NAMES[nakshatra(i as f64 * 360.0 / 27.0 + 1.0).index]
}

/// A dated life event in one area (the expert, KP-based ranking).
#[derive(Clone, Debug, PartialEq)]
pub struct AreaEvent {
    /// Area of life.
    pub area: Area,
    /// When it happened.
    pub at: Instant,
    /// What it was, in the user's words.
    pub label: String,
}

/// How one candidate explains one event.
#[derive(Clone, Debug, PartialEq)]
pub struct EventScore {
    /// The event's label.
    pub label: String,
    /// Score in `[0, 1]`.
    pub score: f64,
    /// Reasons.
    pub factors: Vec<String>,
}

/// One candidate birth time from [`rectify_by_areas`].
#[derive(Clone, Debug, PartialEq)]
pub struct AreaCandidate {
    /// The candidate instant.
    pub birth: Instant,
    /// Mean score over events, `[0, 1]`.
    pub score: f64,
    /// Sidereal Ascendant, degrees.
    pub asc_sidereal: f64,
    /// KP sub-lord of the Ascendant.
    pub asc_sub_lord: DashaLord,
    /// Per-event detail.
    pub per_event: Vec<EventScore>,
}

/// Score a chart against the events.
fn score_chart(facts: &Facts, moon: f64, birth: Instant, events: &[AreaEvent], lang: Lang) -> Vec<EventScore> {
    events
        .iter()
        .map(|e| {
            let mut factors = Vec::new();
            let p = promise(facts, e.area);
            let funnel = timing(facts, e.area, moon, birth, e.at, 3);
            let agree = agreeing_levels(&funnel);
            let mut s = 0.0;
            let area = lang::area_name(lang, e.area.name);
            let primary = e.area.primary;
            if p.promised {
                s += 0.3;
                let houses = p.fulfilling.iter().map(|x| x.house).collect::<Vec<_>>();
                let sub = lang::graha_name(lang, p.sub_lord);
                let list = houses.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(", ");
                factors.push(match lang {
                    Lang::En => format!("{} promised: {:?} as {}th sub-lord signifies {:?}", e.area.name, p.sub_lord, primary, houses),
                    Lang::Ml => format!("{area} വാഗ്ദാനമുണ്ട്: {primary}-ാം ഭാവത്തിന്റെ ഉപനാഥനായ {sub} {list} ഭാവങ്ങളെ സൂചിപ്പിക്കുന്നു"),
                    Lang::Hi => format!("{area} का वादा है: {primary}वें भाव का उप-स्वामी {sub} भाव {list} का सूचक है"),
                });
            } else {
                let sub = lang::graha_name(lang, p.sub_lord);
                factors.push(match lang {
                    Lang::En => format!("{} not promised by the {}th sub-lord {:?}", e.area.name, primary, p.sub_lord),
                    Lang::Ml => format!("{primary}-ാം ഭാവത്തിന്റെ ഉപനാഥനായ {sub} {area} വാഗ്ദാനം ചെയ്യുന്നില്ല"),
                    Lang::Hi => format!("{primary}वें भाव का उप-स्वामी {sub} {area} का वादा नहीं करता"),
                });
            }
            s += 0.1 * p.strength;
            s += 0.4 * agree as f64 / 3.0;
            if let Some(l) = funnel.first() {
                let lord = lang::graha_name(lang, l.period.lord);
                factors.push(match lang {
                    Lang::En => format!("at the event: {:?} maha, {} of 3 levels signify the area", l.period.lord, agree),
                    Lang::Ml => format!("സംഭവസമയത്ത്: {lord} മഹാദശ; 3 തലങ്ങളിൽ {agree} എണ്ണം ഈ മേഖലയെ സൂചിപ്പിക്കുന്നു"),
                    Lang::Hi => format!("घटना के समय: {lord} महादशा; 3 में से {agree} स्तर इस क्षेत्र के सूचक हैं"),
                });
            }
            if let Some(antar) = funnel.get(1) {
                let (q, why) = period_quality(facts, e.area, antar.period.lord);
                s += 0.2 * q;
                let lord = lang::graha_name(lang, antar.period.lord);
                factors.push(match lang {
                    Lang::En => format!("antardasha {:?}: {}", antar.period.lord, why.join("; ")),
                    // The reasons behind the period's quality are English rule text;
                    // the expert line keeps the period and its verdict in the reader's
                    // words.
                    Lang::Ml => format!("{lord} അന്തർദശ: {}", if q >= 0.5 { "ഈ മേഖലയ്ക്ക് അനുകൂലം" } else { "ഈ മേഖലയ്ക്ക് അത്ര അനുകൂലമല്ല" }),
                    Lang::Hi => format!("{lord} अंतर्दशा: {}", if q >= 0.5 { "इस क्षेत्र के अनुकूल" } else { "इस क्षेत्र के लिए कम अनुकूल" }),
                });
            }
            EventScore {
                label: e.label.clone(),
                score: s.clamp(0.0, 1.0),
                factors,
            }
        })
        .collect()
}

/// Scan `[center − half_window, center + half_window]` in `step` nanoseconds and rank
/// the candidate birth instants by how well they explain the events.
#[allow(clippy::too_many_arguments)]
pub fn rectify_by_areas(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    center: Instant,
    half_window_ns: i128,
    step_ns: i128,
    events: &[AreaEvent],
) -> Result<Vec<AreaCandidate>, ChartError> {
    rectify_by_areas_in(spk, eop, obs, ayanamsa, center, half_window_ns, step_ns, events, Lang::En)
}

/// [`rectify_by_areas`], its reasons in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn rectify_by_areas_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    center: Instant,
    half_window_ns: i128,
    step_ns: i128,
    events: &[AreaEvent],
    lang: Lang,
) -> Result<Vec<AreaCandidate>, ChartError> {
    let step = step_ns.max(1_000_000_000);
    let mut out = Vec::new();
    let mut t = center.add_ns(-half_window_ns);
    let end = center.add_ns(half_window_ns);
    while t <= end {
        let input = ChartInput {
            instant: t,
            observer: *obs,
            system: HouseSystem::Placidus,
            ayanamsa,
            topocentric_moon: false,
        };
        let ch = compute(spk, eop, &input)?;
        let facts = Facts::from_chart(&ch);
        let moon = ch.bodies[1].sidereal;
        let per_event = score_chart(&facts, moon, t, events, lang);
        let score = if per_event.is_empty() {
            0.0
        } else {
            per_event.iter().map(|e| e.score).sum::<f64>() / per_event.len() as f64
        };
        out.push(AreaCandidate {
            birth: t,
            score,
            asc_sidereal: ch.cusps_sidereal[0],
            asc_sub_lord: facts.cusp_sub_lords[0],
            per_event,
        });
        t = t.add_ns(step);
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.birth.cmp(&b.birth))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::area;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    fn observer() -> Observer {
        Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        }
    }

    #[test]
    fn candidates_are_ranked_and_distinct_rising_signs_are_found() {
        let Some(spk) = kernel() else { return };
        let obs = observer();
        let centre = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let events = [
            LifeEvent { at: Utc::new(2018, 11, 12, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Love, good: true },
            LifeEvent { at: Utc::new(2025, 7, 17, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Love, good: false },
            LifeEvent { at: Utc::new(2021, 3, 1, 6, 0, 0, 0).unwrap().to_instant().unwrap(), life: Life::Career, good: true },
        ];
        let v = rectify(&spk, None, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, centre.add_ns(-3 * 3600 * NS_PER_SEC), centre.add_ns(3 * 3600 * NS_PER_SEC), &events, 37);
        assert_eq!(v.len(), 37);
        assert!(v.windows(2).all(|w| w[0].fit >= w[1].fit));
        assert!(v.iter().all(|c| (0.0..=1.0).contains(&c.fit)));
        let signs = by_rising_sign(&v);
        assert!(signs.len() >= 3, "six hours of window cross at least three rising signs: {}", signs.len());
    }

    #[test]
    fn area_candidates_cover_the_window_and_are_ranked() {
        let Some(spk) = kernel() else { return };
        let obs = observer();
        let center = Utc::new(1990, 10, 29, 17, 36, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let events = vec![
            AreaEvent {
                area: area("Career").unwrap(),
                at: Utc::new(2013, 7, 1, 4, 0, 0, 0)
                    .unwrap()
                    .to_instant()
                    .unwrap(),
                label: "first job".into(),
            },
            AreaEvent {
                area: area("Marriage").unwrap(),
                at: Utc::new(2018, 11, 22, 5, 0, 0, 0)
                    .unwrap()
                    .to_instant()
                    .unwrap(),
                label: "wedding".into(),
            },
        ];
        let c = rectify_by_areas(
            &spk,
            None,
            &obs,
            Ayanamsa::Lahiri,
            center,
            15 * 60 * NS_PER_SEC,
            60 * NS_PER_SEC,
            &events,
        )
        .unwrap();
        assert_eq!(c.len(), 31);
        for w in c.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
        for cand in &c {
            assert_eq!(cand.per_event.len(), 2);
            assert!((0.0..=1.0).contains(&cand.score));
            assert!(cand.per_event.iter().all(|e| !e.factors.is_empty()));
        }
        // the ascendant moves about 0.25° per minute at this latitude
        let first = c.iter().min_by_key(|x| x.birth).unwrap();
        let last = c.iter().max_by_key(|x| x.birth).unwrap();
        let moved = (last.asc_sidereal - first.asc_sidereal).rem_euclid(360.0);
        assert!(moved > 5.0 && moved < 10.0, "{moved}");
    }
}
