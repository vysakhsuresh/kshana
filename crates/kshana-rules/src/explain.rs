//! Why a life stands where it stands, who causes it, when it eases and what eases it.
//!
//! The forecast gives a life a score built from named components. This module opens
//! every component to its grahas: each running period lord, each transiting graha,
//! the Moon's star, the planetary hour and the limbs of the day become a *cause* with
//! a signed pull on the score and the exact instant it ends. For every cause that
//! ends within two years the engine recomputes the component with the graha moved on
//! (the next lord, the next sign, the next star, the next hour) and reports the exact
//! gain, so "what eases it" is a solved number, not a guess. A monthly slow series
//! (birth chart, periods, transits of the eight slow grahas) gives the turning points:
//! when the life first climbs a band, when it peaks, when it dips.

use crate::forecast::{
    abhijit_window, dasha_items, date_words, gochara_items, hora_component, kala_component,
    moment, natal_component, sky, slot_window, slow_scores, tara_component, weekday,
    Band, ForecastError, Hora, Life, Moment, Natal, GULIKA, YAMAGANDA,
};
use crate::gochara::{hora_lord, hora_words, tara_index, TARA_SCORE};
use crate::plain::{birth_base, cap, chapter_theme, day_quality, hour_nature, influence_span, level_noun, long_test, pressure, support};
use crate::transit::next_sign_change;
use kshana_ephem::{Body, Spk};
use kshana_events::{hora_at, nakshatra_boundaries, rahu_kala, solar_day, MICROSECOND_NS};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{nakshatra, vimshottari_levels, Ayanamsa, DashaLord};

/// Something that pulls a life's score up or down right now.
#[derive(Clone, Debug, PartialEq)]
pub struct Cause {
    /// The graha behind it, when there is one.
    pub planet: Option<DashaLord>,
    /// `natal`, `dasha`, `gochara`, `tara`, `hora` or `kala`.
    pub kind: &'static str,
    /// What it is, in words.
    pub what: String,
    /// Its signed share of the score, in `[-0.5, 0.5]` (×100 for points).
    pub pull: f64,
    /// When it ends, if it ends.
    pub until: Option<Instant>,
    /// The end in words, with a date.
    pub until_words: String,
    /// Rule id.
    pub rule_id: &'static str,
}

/// A coming change and its exact effect on the score.
#[derive(Clone, Debug, PartialEq)]
pub struct Relief {
    /// When.
    pub at: Instant,
    /// What changes, in words.
    pub what: String,
    /// Signed change of the life score, in `[-1, 1]` (×100 for points).
    pub gain: f64,
    /// The graha that moves.
    pub planet: Option<DashaLord>,
    /// `dasha`, `gochara`, `tara`, `hora` or `kala`.
    pub kind: &'static str,
    /// True for a change that sets a season rather than a day: a mahadasha, antardasha
    /// or pratyantar boundary, or a sign change of any graha but the Moon.
    pub lasting: bool,
}

/// A turning point of the slow series.
#[derive(Clone, Debug, PartialEq)]
pub struct Turning {
    /// When.
    pub at: Instant,
    /// Slow score there, `[0, 1]`.
    pub score: f64,
    /// Band there.
    pub band: Band,
    /// In words, with the date.
    pub words: String,
}

/// The whole explanation of one life at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Explanation {
    /// Which life.
    pub life: Life,
    /// The instant.
    pub at: Instant,
    /// The life score now, `[0, 1]`.
    pub score: f64,
    /// Its band.
    pub band: Band,
    /// Two or three plain sentences with dates.
    pub headline: String,
    /// What drags, hardest first.
    pub causes: Vec<Cause>,
    /// What helps, strongest first.
    pub helpers: Vec<Cause>,
    /// Coming changes within two years, chronological.
    pub relief: Vec<Relief>,
    /// The first month the slow score climbs a band.
    pub eases: Option<Turning>,
    /// The best month of the next three years.
    pub peak: Option<Turning>,
    /// The first month the slow score drops a band.
    pub toughens: Option<Turning>,
    /// The slow score now.
    pub slow_now: f64,
    /// Thirty-six monthly slow samples from `at`.
    pub slow: Vec<(Instant, f64)>,
}

fn life_index(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

fn points(x: f64) -> String {
    let p = (x * 100.0).round() as i64;
    if p >= 0 {
        format!("+{p}")
    } else {
        format!("{p}")
    }
}

/// Explain `life` at `at` for a person born as `natal` describes, standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn explain(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    at: Instant,
) -> Result<Explanation, ForecastError> {
    let idx = life_index(life);
    let name = life.name().to_ascii_lowercase();
    let lon = obs.longitude_deg;
    let m: Moment = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let ls = &m.lives[idx];
    let sky_now = sky(spk, eop, natal, obs, system, ayanamsa, at)?;
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 5);
    let two_years = at.add_ns(730 * NS_PER_DAY);
    let total_weight: f64 = ls.components.iter().map(|c| c.weight).sum();
    let norm = |w: f64| if total_weight > 0.0 { w / total_weight } else { w };

    let mut causes: Vec<Cause> = Vec::new();
    let mut relief: Vec<Relief> = Vec::new();

    // 1. The birth chart.
    let n = natal_component(natal, life);
    causes.push(Cause {
        planet: Some(natal.facts.cusp_sub_lords[(life.area().primary - 1) as usize]),
        kind: "natal",
        what: birth_base(&name, n.score),
        pull: norm(n.weight) * (n.score - 0.5),
        until: None,
        until_words: "for life: every season builds on it, and the remedies work on it".into(),
        rule_id: "KS-N-0001",
    });

    // 2. The running periods, level by level. When several levels end at the same
    // instant (a deeper level always ends with its parent) the change is one event.
    let items = dasha_items(natal, life, &levels);
    let lw: f64 = items.iter().map(|i| i.weight).sum();
    let mut ends_seen: Vec<Instant> = Vec::new();
    for it in &items {
        let share = if lw > 0.0 { it.weight / lw } else { 0.0 };
        causes.push(Cause {
            planet: Some(it.lord),
            kind: "dasha",
            what: if it.score >= 0.5 {
                format!(
                    "the {} (about {}) brings {} to your {name}",
                    level_noun(it.level),
                    chapter_theme(it.lord),
                    support(it.lord)
                )
            } else {
                format!(
                    "the {} (about {}) brings {} to your {name}",
                    level_noun(it.level),
                    chapter_theme(it.lord),
                    pressure(it.lord)
                )
            },
            pull: norm(0.30) * share * (it.score - 0.5),
            until: Some(it.end),
            until_words: format!("this {} ends on {}", level_noun(it.level), date_words(it.end, lon)),
            rule_id: "KS-F-0001",
        });
        if it.end <= two_years && !ends_seen.contains(&it.end) {
            ends_seen.push(it.end);
            // The whole chain after this level ends: the exact dasha component then.
            let after = it.end.add_ns(NS_PER_SEC);
            let next_levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, after, 5);
            let now_c = dasha_component_score(natal, life, &levels);
            let then_c = dasha_component_score(natal, life, &next_levels);
            let next_lord = next_levels.get(it.level).map(|p| p.lord);
            if let Some(nl) = next_lord {
                let gain = norm(0.30) * (then_c - now_c);
                let (s, _houses) = crate::forecast::lord_score(natal, life, nl);
                relief.push(Relief {
                    at: it.end,
                    what: format!(
                        "a new {} begins, about {}: {}",
                        level_noun(it.level),
                        chapter_theme(nl),
                        if gain >= 0.01 {
                            format!("better for your {name}, with {}", support(nl))
                        } else if gain <= -0.01 {
                            format!("harder for your {name}, with {}", pressure(nl))
                        } else if s >= 0.5 {
                            format!("about the same for your {name}, with {}", support(nl))
                        } else {
                            format!("about the same for your {name}; watch for {}", pressure(nl))
                        }
                    ),
                    gain,
                    planet: Some(nl),
                    kind: "dasha",
                    lasting: it.level <= 2,
                });
            }
        }
    }

    // 3. The transits, graha by graha.
    let signs = sky_now.signs();
    let (gitems, den) = gochara_items(natal, &sky_now.from_moon, &signs, life);
    let g_now = gochara_score(&gitems, den);
    for (i, it) in gitems.iter().enumerate() {
        if it.contrib == 0.0 && it.relevance < 0.5 {
            continue;
        }
        let pull = if den > 0.0 { norm(0.20) * 0.5 * it.contrib / den } else { 0.0 };
        let change = next_sign_change(spk, ayanamsa, it.graha, at);
        let (until, until_words) = match change {
            Some(c) => (Some(c.at), format!("until {}", date_words(c.at, lon))),
            None => (None, "for longer than two years".to_string()),
        };
        let what_now = if it.saturn_phase != crate::gochara::SaturnPhase::Clear {
            format!("{} for you, touching your {name}", long_test(it.saturn_phase))
        } else if it.favourable && it.obstructed {
            format!("help for your {name} is on its way but partly blocked")
        } else if it.favourable || pull > 0.0 {
            format!("{} for your {name}, for {}", support(it.graha), influence_span(it.graha))
        } else if it.adverse || pull < 0.0 {
            format!("{} around your {name}, for {}", pressure(it.graha), influence_span(it.graha))
        } else {
            format!("a neutral spell for your {name}, for {}", influence_span(it.graha))
        };
        if pull.abs() >= 0.002 || it.graha == DashaLord::Saturn || it.graha == DashaLord::Jupiter {
            causes.push(Cause {
                planet: Some(it.graha),
                kind: "gochara",
                what: what_now,
                pull,
                until,
                until_words,
                rule_id: "KS-G-0001",
            });
        }
        if let Some(c) = change {
            if c.at <= two_years {
                let mut fm = sky_now.from_moon;
                let mut sg = signs;
                fm[i] = ((c.to as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                sg[i] = c.to;
                let (then_items, then_den) = gochara_items(natal, &fm, &sg, life);
                let g_then = gochara_score(&then_items, then_den);
                let gain = norm(0.20) * (g_then - g_now);
                let then_it = &then_items[i];
                let what = if it.saturn_phase != crate::gochara::SaturnPhase::Clear
                    && then_it.saturn_phase == crate::gochara::SaturnPhase::Clear
                {
                    format!("{} ends", cap(long_test(it.saturn_phase)))
                } else if then_it.saturn_phase != crate::gochara::SaturnPhase::Clear && it.saturn_phase == crate::gochara::SaturnPhase::Clear {
                    format!("{} begins", cap(long_test(then_it.saturn_phase)))
                } else if then_it.saturn_phase != crate::gochara::SaturnPhase::Clear {
                    format!("{} goes on, in a new form", cap(long_test(then_it.saturn_phase)))
                } else if then_it.favourable && then_it.obstructed {
                    format!("help for your {name} arrives, though partly blocked, for {}", influence_span(it.graha))
                } else if then_it.favourable {
                    format!("{} arrives for your {name}, for {}", cap(support(it.graha)), influence_span(it.graha))
                } else if then_it.adverse {
                    format!("{} set in around your {name}, for {}", cap(pressure(it.graha)), influence_span(it.graha))
                } else if gain > 0.0 {
                    format!("a pressure on your {name} lifts")
                } else {
                    format!("a help to your {name} fades")
                };
                if gain.abs() >= 0.005 || it.graha == DashaLord::Saturn {
                    relief.push(Relief {
                        at: c.at,
                        what,
                        gain,
                        planet: Some(it.graha),
                        kind: "gochara",
                        lasting: it.graha != DashaLord::Moon,
                    });
                }
            }
        }
    }

    // 4. Tara: the Moon's star counted from the birth star.
    let (t_now, tara_c) = tara_component(natal, &sky_now);
    let star_end = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, at.add_ns(2 * NS_PER_DAY), MICROSECOND_NS)
        .ok()
        .and_then(|v| v.first().map(|e| e.root.t));
    let cur_star = nakshatra(sky_now.moon_sidereal).index;
    let next_star = (cur_star + 1) % 27;
    let t_next = tara_index(natal.janma(), next_star);
    causes.push(Cause {
        planet: Some(DashaLord::Moon),
        kind: "tara",
        what: format!("today is {}", day_quality(t_now)),
        pull: norm(tara_c.weight) * (tara_c.score - 0.5),
        until: star_end,
        until_words: match star_end {
            Some(e) => format!("until the day's mood turns, {}", date_words(e, lon)),
            None => String::new(),
        },
        rule_id: "KS-T-0001",
    });
    if let Some(e) = star_end {
        relief.push(Relief {
            at: e,
            what: format!("the day's mood turns: {}", day_quality(t_next)),
            gain: norm(tara_c.weight) * (TARA_SCORE[t_next] - TARA_SCORE[t_now]),
            planet: Some(DashaLord::Moon),
            kind: "tara",
            lasting: false,
        });
    }

    // 5. The planetary hour and the limbs of the day.
    let day = solar_day(spk, eop, obs, at).ok().flatten();
    if let (Some(h), Some(d)) = (&m.hora, &day) {
        let hc = hora_component(natal, life, h);
        causes.push(Cause {
            planet: Some(h.lord),
            kind: "hora",
            what: if hc.score < 0.5 {
                format!("this hour does not favour your {}: it is {}", life.name().to_ascii_lowercase(), hour_nature(h.lord))
            } else {
                format!("this hour helps your {}: it is {}", life.name().to_ascii_lowercase(), hour_nature(h.lord))
            },
            pull: norm(hc.weight) * (hc.score - 0.5),
            until: Some(h.end),
            until_words: "until the hour turns".into(),
            rule_id: "KS-H-0001",
        });
        if h.end < d.next_sunrise {
            let (nname, ns, ne) = hora_at(d, obs, h.end.add_ns(NS_PER_SEC));
            if let Some(nl) = hora_lord(nname) {
                let nh = Hora {
                    lord: nl,
                    start: ns,
                    end: ne,
                    words: hora_words(nl),
                };
                let nc = hora_component(natal, life, &nh);
                relief.push(Relief {
                    at: h.end,
                    what: format!("the next hour is {}", hour_nature(nl)),
                    gain: norm(hc.weight) * (nc.score - hc.score),
                    planet: Some(nl),
                    kind: "hora",
                    lasting: false,
                });
            }
        }
    }
    let kc = kala_component(&sky_now, day.as_ref(), obs, at);
    let mut kala_until = None;
    let mut kala_words = String::new();
    if let Some(d) = &day {
        let wd = weekday(d, obs) as usize;
        let windows = [
            ("the unlucky stretch", rahu_kala(d, obs)),
            ("the risky stretch", slot_window(d, YAMAGANDA[wd])),
            ("the heavy stretch", slot_window(d, GULIKA[wd])),
            ("the lucky minutes", abhijit_window(d)),
        ];
        for (label, (s, e)) in windows {
            if at >= s && at < e {
                kala_until = Some(e);
                kala_words = format!("until {label} closes");
                let after = kala_component(&sky_now, Some(d), obs, e.add_ns(NS_PER_SEC));
                relief.push(Relief {
                    at: e,
                    what: format!("{label} closes"),
                    gain: norm(kc.weight) * (after.score - kc.score),
                    planet: None,
                    kind: "kala",
                    lasting: false,
                });
            }
        }
    }
    causes.push(Cause {
        planet: None,
        kind: "kala",
        what: if kc.score < 0.45 {
            "an unlucky stretch of the day is running: do not start anything new in it".to_string()
        } else if kc.score > 0.55 {
            "the lucky minutes of the day are running: a good time to begin".to_string()
        } else {
            "no special stretch of the day is running".to_string()
        },
        pull: norm(kc.weight) * (kc.score - 0.5),
        until: kala_until,
        until_words: kala_words,
        rule_id: "KS-M-0001",
    });

    // Order: what drags, what helps.
    let mut helpers: Vec<Cause> = causes.iter().filter(|c| c.pull > 0.0).cloned().collect();
    helpers.sort_by(|a, b| b.pull.partial_cmp(&a.pull).unwrap_or(std::cmp::Ordering::Equal));
    causes.retain(|c| c.pull <= 0.0);
    causes.sort_by(|a, b| a.pull.partial_cmp(&b.pull).unwrap_or(std::cmp::Ordering::Equal));
    relief.retain(|r| r.at > at);
    relief.sort_by(|a, b| a.at.cmp(&b.at));

    // The slow series: thirty-six months.
    let slow_now = slow_scores(natal, &sky_now, &levels[..levels.len().min(3)])[idx];
    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let slow_all = crate::forecast::slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(month), month, 36)?;
    let slow: Vec<(Instant, f64)> = slow_all.iter().map(|(t, v)| (*t, v[idx])).collect();
    // The season is judged against the better of the slow band and the band the person
    // sees now, so "climbs" always names a band above what is on the screen.
    let band_now = Band::of(slow_now).max(ls.band);
    let band_floor = Band::of(slow_now).min(ls.band);
    let turning = |t: Instant, s: f64, verb: &str| Turning {
        at: t,
        score: s,
        band: Band::of(s),
        words: format!("{} {} to {} from {}", life.name(), verb, Band::of(s).word(), date_words(t, lon)),
    };
    let eases = slow
        .iter()
        .find(|(_, s)| Band::of(*s) > band_now)
        .map(|&(t, s)| turning(t, s, "climbs"));
    let toughens = slow
        .iter()
        .find(|(_, s)| Band::of(*s) < band_floor)
        .map(|&(t, s)| turning(t, s, "slips"));
    let peak = slow
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, &(t, s)| match acc {
            Some((_, bs)) if bs >= s => acc,
            _ => Some((t, s)),
        })
        .filter(|(_, s)| *s > slow_now + 0.02)
        .map(|(t, s)| Turning {
            at: t,
            score: s,
            band: Band::of(s),
            words: format!("{} peaks around {} at {:.0}", life.name(), date_words(t, lon), s * 100.0),
        });

    // Words.
    let mut headline = String::new();
    let lasting = |c: &Cause| matches!(c.kind, "natal" | "dasha" | "gochara");
    if ls.band >= Band::Good {
        let top = helpers.iter().find(|c| lasting(c)).or(helpers.first());
        headline.push_str(&format!("{} is {} now", life.name(), ls.band.word()));
        if let Some(t) = top {
            headline.push_str(&format!(": {}", t.what));
        }
        headline.push('.');
        if let Some(t) = &toughens {
            headline.push_str(&format!(" Make the most of it; it slips to {} from {}.", t.band.word(), date_words(t.at, lon)));
        } else if let Some(p) = &peak {
            headline.push_str(&format!(" It peaks around {}.", date_words(p.at, lon)));
        }
    } else {
        let drags: Vec<&Cause> = causes.iter().filter(|c| lasting(c)).take(2).collect();
        headline.push_str(&format!("{} is {} now", life.name(), ls.band.word()));
        if !drags.is_empty() {
            headline.push_str(" because ");
            headline.push_str(&drags.iter().map(|c| c.what.as_str()).collect::<Vec<_>>().join(", and "));
        }
        headline.push('.');
        if let Some(r) = relief.iter().find(|r| r.gain >= 0.02 && r.lasting) {
            headline.push_str(&format!(" The first real lift comes on {}: {} ({} points).", date_words(r.at, lon), r.what, points(r.gain)));
        }
        if let Some(e) = &eases {
            headline.push_str(&format!(" {} climbs to {} from {}.", life.name(), e.band.word(), date_words(e.at, lon)));
        } else if ls.band <= Band::Slow {
            headline.push_str(" The slow stretch holds through the next three years; the remedies are the lever.");
        }
    }

    Ok(Explanation {
        life,
        at,
        score: ls.score,
        band: ls.band,
        headline,
        causes,
        helpers,
        relief,
        eases,
        peak,
        toughens,
        slow_now,
        slow,
    })
}

fn dasha_component_score(natal: &Natal, life: Life, levels: &[kshana_vedic::DashaPeriod]) -> f64 {
    let items = dasha_items(natal, life, levels);
    let den: f64 = items.iter().map(|i| i.weight).sum();
    if den <= 0.0 {
        return 0.5;
    }
    (items.iter().map(|i| i.weight * i.score).sum::<f64>() / den).clamp(0.0, 1.0)
}

fn gochara_score(items: &[crate::forecast::GocharaItem], den: f64) -> f64 {
    let num: f64 = items.iter().map(|i| i.contrib).sum();
    let raw = if den > 0.0 { num / den } else { 0.0 };
    (0.5 + 0.5 * raw).clamp(0.0, 1.0)
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
    fn every_cause_has_words_and_the_relief_is_exact_and_ordered() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let e = explain(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Love, at).unwrap();
        assert!(!e.headline.is_empty());
        assert!(e.causes.len() + e.helpers.len() >= 8, "{} {}", e.causes.len(), e.helpers.len());
        for c in e.causes.iter().chain(e.helpers.iter()) {
            assert!(!c.what.is_empty());
            assert!(c.pull.abs() <= 0.5);
        }
        assert!(e.causes.iter().all(|c| c.pull <= 0.0));
        assert!(e.helpers.iter().all(|c| c.pull > 0.0));
        // the pulls sum to the score minus one half
        let total: f64 = e.causes.iter().chain(e.helpers.iter()).map(|c| c.pull).sum();
        assert!((total - (e.score - 0.5)).abs() < 0.02, "{total} vs {}", e.score - 0.5);
        assert!(!e.relief.is_empty());
        assert!(e.relief.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(e.relief.iter().all(|r| r.at > at && r.gain.abs() <= 1.0));
        // Saturn's next sign change is always reported
        assert!(e.relief.iter().any(|r| r.planet == Some(DashaLord::Saturn)));
        assert_eq!(e.slow.len(), 36);
        assert!((0.0..=1.0).contains(&e.slow_now));
        use crate::plain::jargon;
        for c in e.causes.iter().chain(e.helpers.iter()) {
            assert_eq!(jargon(&c.what), None, "{}", c.what);
            assert_eq!(jargon(&c.until_words), None, "{}", c.until_words);
        }
        for r in &e.relief {
            assert_eq!(jargon(&r.what), None, "{}", r.what);
        }
        for t in [&e.eases, &e.peak, &e.toughens].into_iter().flatten() {
            assert_eq!(jargon(&t.words), None, "{}", t.words);
        }
    }
}
