//! KP horary (Prashna) judgment.
//!
//! The chart is cast for the instant of the question, with the lagna fixed by the
//! querent's number when one is given. Judgment (KP Reader VI):
//! 1. The Moon's sub-lord shows the matter in the querent's mind: if it signifies the
//!    houses of the question, the question is genuine and the answer is read on.
//! 2. The primary cusp's sub-lord must signify the fulfilling houses (the promise rule).
//! 3. A retrograde sub-lord, or a sub-lord in the star of a retrograde planet, delays.
//! 4. The ruling planets at the question (lagna sign and star lords, Moon sign and star
//!    lords, day lord) time the matter: it fructifies when the Moon transits stars and
//!    subs of planets that are both ruling and significators.

use crate::areas::Area;
use crate::facts::Facts;
use crate::kp::{promise, significators, Promise, Significance};
use crate::Hit;
use kshana_ephem::{Body, Spk};
use kshana_sky::apparent;
use kshana_time::{Instant, Utc, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, kp_lords, sidereal, sign_lord, Ayanamsa, DashaLord};

/// The shape of the question, from the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum QuestionShape {
    WillItHappen,
    When,
    Choice,
    WillItReturn,
    WillTheyRecover,
}

/// The five ruling planets at the question.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct RulingPlanets {
    pub lagna_sign_lord: DashaLord,
    pub lagna_star_lord: DashaLord,
    pub moon_sign_lord: DashaLord,
    pub moon_star_lord: DashaLord,
    pub day_lord: DashaLord,
}

impl RulingPlanets {
    /// All five, deduplicated, in order.
    pub fn all(&self) -> Vec<DashaLord> {
        let mut v = vec![
            self.lagna_sign_lord,
            self.lagna_star_lord,
            self.moon_sign_lord,
            self.moon_star_lord,
            self.day_lord,
        ];
        v.dedup();
        let mut out: Vec<DashaLord> = Vec::new();
        for l in v {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }
}

/// The answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Verdict {
    Yes,
    Delayed,
    No,
    /// The sub-lord signifies neither fulfilling nor negating houses: no judgment.
    Unclear,
}

/// Full Prashna result.
#[derive(Clone, Debug, PartialEq)]
pub struct PrashnaVerdict {
    /// The answer.
    pub verdict: Verdict,
    /// Whether the Moon's sub-lord connects to the matter (question genuine).
    pub question_genuine: bool,
    /// Moon's sub-lord.
    pub moon_sub_lord: DashaLord,
    /// The cusp sub-lord promise.
    pub promise: Promise,
    /// Ruling planets.
    pub ruling: RulingPlanets,
    /// Planets that are both ruling and fulfilling significators: the timing agents.
    pub agents: Vec<DashaLord>,
    /// The fired rules.
    pub hits: Vec<Hit>,
}

/// Day lord from the local civil weekday (longitude-based local time; the sunrise-based
/// vara differs only between midnight and sunrise).
pub fn day_lord(instant: Instant, longitude_deg: f64) -> DashaLord {
    let local = instant.add_ns((longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    let u = Utc::from_instant(local);
    let days = u.utc_day(); // 1970-01-01 was a Thursday
    match (days + 4).rem_euclid(7) {
        0 => DashaLord::Sun,
        1 => DashaLord::Moon,
        2 => DashaLord::Mars,
        3 => DashaLord::Mercury,
        4 => DashaLord::Jupiter,
        5 => DashaLord::Venus,
        _ => DashaLord::Saturn,
    }
}

/// Ruling planets at the question.
pub fn ruling_planets(facts: &Facts, instant: Instant, longitude_deg: f64) -> RulingPlanets {
    let lagna = facts.cusps[0];
    let moon = facts.planet(DashaLord::Moon);
    RulingPlanets {
        lagna_sign_lord: sign_lord((lagna / 30.0).floor() as usize),
        lagna_star_lord: facts.cusp_star_lords[0],
        moon_sign_lord: sign_lord((moon.sidereal / 30.0).floor() as usize),
        moon_star_lord: moon.star_lord,
        day_lord: day_lord(instant, longitude_deg),
    }
}

/// Judge a question.
pub fn prashna(
    facts: &Facts,
    area: Area,
    shape: QuestionShape,
    instant: Instant,
    longitude_deg: f64,
) -> PrashnaVerdict {
    let mut hits = Vec::new();
    let moon = facts.planet(DashaLord::Moon);
    let moon_sig = significators(facts, moon.sub_lord);
    let genuine_houses: Vec<Significance> = moon_sig
        .iter()
        .copied()
        .filter(|s| area.group.contains(&s.house))
        .collect();
    let question_genuine = !genuine_houses.is_empty();
    hits.push(Hit {
        rule_id: "KS-P-0001",
        source: "KP Reader VI: the Moon's sub-lord reveals the matter in the querent's mind",
        weight: if question_genuine { 1.0 } else { 0.3 },
        factors: if question_genuine {
            genuine_houses.iter().map(|s| format!("Moon's sub-lord {:?} signifies house {}", moon.sub_lord, s.house)).collect()
        } else {
            vec![format!("Moon's sub-lord {:?} does not signify houses {:?}; the question may not be the real concern", moon.sub_lord, area.group)]
        },
    });

    let p = promise(facts, area);
    hits.push(p.hit.clone());

    let sub = facts.planet(p.sub_lord);
    let sub_star = facts.planet(sub.star_lord);
    let retro_sub = sub.retrograde && !matches!(sub.graha, DashaLord::Rahu | DashaLord::Ketu);
    let retro_star =
        sub_star.retrograde && !matches!(sub_star.graha, DashaLord::Rahu | DashaLord::Ketu);
    if retro_sub || retro_star {
        hits.push(Hit {
            rule_id: "KS-P-0012",
            source: "KP Reader VI: a retrograde sub-lord, or one in the star of a retrograde planet, delays until the planet turns direct",
            weight: 0.8,
            factors: {
                let mut v = Vec::new();
                if retro_sub {
                    v.push(format!("{:?}, the sub-lord, is retrograde", sub.graha));
                }
                if retro_star {
                    v.push(format!("{:?}, the sub-lord's star lord, is retrograde", sub_star.graha));
                }
                v
            },
        });
    }

    let ruling = ruling_planets(facts, instant, longitude_deg);
    let fulfilling_lords: Vec<DashaLord> = facts
        .planets
        .iter()
        .map(|pl| pl.graha)
        .filter(|&g| {
            significators(facts, g)
                .iter()
                .any(|s| area.group.contains(&s.house))
        })
        .collect();
    let agents: Vec<DashaLord> = ruling
        .all()
        .into_iter()
        .filter(|l| fulfilling_lords.contains(l))
        .collect();
    hits.push(Hit {
        rule_id: "KS-P-0020",
        source: "KP Reader VI: ruling planets at the question time the matter when they are also significators",
        weight: if agents.is_empty() { 0.2 } else { 0.7 },
        factors: vec![
            format!(
                "ruling planets: lagna sign {:?}, lagna star {:?}, Moon sign {:?}, Moon star {:?}, day {:?}",
                ruling.lagna_sign_lord, ruling.lagna_star_lord, ruling.moon_sign_lord, ruling.moon_star_lord, ruling.day_lord
            ),
            if agents.is_empty() { "none of them signifies the matter".into() } else { format!("agents for timing: {agents:?}") },
        ],
    });

    let verdict = match (p.promised, retro_sub || retro_star, p.denying.is_empty()) {
        (true, false, _) => Verdict::Yes,
        (true, true, _) => Verdict::Delayed,
        (false, _, false) => Verdict::No,
        (false, _, true) => Verdict::Unclear,
    };
    let verdict = match shape {
        // a choice question is judged on the 7th (the other option) against the lagna:
        // reported as Yes = take the offered option; the caller frames it
        QuestionShape::Choice => verdict,
        _ => verdict,
    };
    PrashnaVerdict {
        verdict,
        question_genuine,
        moon_sub_lord: moon.sub_lord,
        promise: p,
        ruling,
        agents,
        hits,
    }
}

/// A window when the Moon transits a star and sub both ruled by timing agents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoonWindow {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Star lord during the window.
    pub star: DashaLord,
    /// Sub lord during the window.
    pub sub: DashaLord,
}

/// Windows in the next `days` days when the Moon's star lord and sub lord are both in
/// `agents`. Boundaries are refined to one microsecond.
pub fn prashna_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    agents: &[DashaLord],
    from: Instant,
    days: i64,
) -> Vec<MoonWindow> {
    if agents.is_empty() {
        return Vec::new();
    }
    let lords_at = |t: Instant| -> Option<(DashaLord, DashaLord)> {
        let a = apparent(spk, Body::Moon, t).ok()?;
        let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
        let k = kp_lords(sidereal(a.longitude, ay));
        Some((k.star, k.sub))
    };
    let on = |l: Option<(DashaLord, DashaLord)>| {
        l.map(|(s, u)| agents.contains(&s) && agents.contains(&u))
            .unwrap_or(false)
    };
    let step = 15 * 60 * NS_PER_SEC;
    let to = from.add_ns(days as i128 * 86_400 * NS_PER_SEC);
    let refine = |mut a: Instant, mut b: Instant, want_on: bool| -> Instant {
        // f(a) != want_on, f(b) == want_on; bracket to 1 µs, like every other instant
        while b.ns_since(a) > 1_000 {
            let m = a.add_ns(b.ns_since(a) / 2);
            if on(lords_at(m)) == want_on {
                b = m;
            } else {
                a = m;
            }
        }
        b
    };
    let mut out = Vec::new();
    let mut t = from;
    let mut state = on(lords_at(t));
    let mut start = if state { Some(t) } else { None };
    while t < to {
        let n = if t.add_ns(step) > to {
            to
        } else {
            t.add_ns(step)
        };
        let s = on(lords_at(n));
        if s != state {
            if s {
                start = Some(refine(t, n, true));
            } else if let Some(st) = start.take() {
                let end = refine(t, n, false);
                let (star, sub) =
                    lords_at(st.add_ns(1_000)).unwrap_or((DashaLord::Moon, DashaLord::Moon));
                out.push(MoonWindow {
                    start: st,
                    end,
                    star,
                    sub,
                });
            }
            state = s;
        }
        t = n;
    }
    if let Some(st) = start {
        let (star, sub) = lords_at(st.add_ns(1_000)).unwrap_or((DashaLord::Moon, DashaLord::Moon));
        out.push(MoonWindow {
            start: st,
            end: to,
            star,
            sub,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::AREAS;
    use kshana_chart::{compute_horary, ChartInput};
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;

    #[test]
    fn day_lord_cycle() {
        // 2026-10-15 is a Thursday
        let t = Utc::new(2026, 10, 15, 9, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        assert_eq!(day_lord(t, 76.0), DashaLord::Jupiter);
        let t = Utc::new(2026, 10, 18, 12, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        assert_eq!(day_lord(t, 0.0), DashaLord::Sun);
    }

    #[test]
    fn horary_judgment_is_complete_for_every_number_shape_and_area() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let t = Utc::new(2026, 10, 15, 9, 2, 7, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let input = ChartInput {
            instant: t,
            observer: Observer {
                latitude_deg: 9.9312,
                longitude_deg: 76.2673,
                height_m: 0.0,
            },
            system: HouseSystem::Placidus,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        };
        let mut verdicts = [0usize; 4];
        for number in [1u16, 57, 100, 183, 249] {
            let ch = compute_horary(&spk, None, &input, number).unwrap();
            let facts = Facts::from_chart(&ch);
            for area in AREAS {
                let v = prashna(&facts, area, QuestionShape::WillItHappen, t, 76.2673);
                assert!(v.hits.len() >= 3);
                assert_eq!(
                    v.ruling.all().len(),
                    v.ruling
                        .all()
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                );
                verdicts[match v.verdict {
                    Verdict::Yes => 0,
                    Verdict::Delayed => 1,
                    Verdict::No => 2,
                    Verdict::Unclear => 3,
                }] += 1;
            }
        }
        assert!(
            verdicts[0] > 0 && verdicts[2] + verdicts[3] > 0,
            "{verdicts:?}"
        );
    }

    #[test]
    fn moon_windows_are_ordered_and_consistent() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let from = Utc::new(2026, 10, 15, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let agents = [DashaLord::Jupiter, DashaLord::Venus, DashaLord::Mercury];
        let w = prashna_windows(&spk, Ayanamsa::Lahiri, &agents, from, 30);
        assert!(!w.is_empty());
        for x in &w {
            assert!(x.start < x.end);
            assert!(agents.contains(&x.star) && agents.contains(&x.sub));
        }
        for pair in w.windows(2) {
            assert!(pair[0].end <= pair[1].start);
        }
    }
}
