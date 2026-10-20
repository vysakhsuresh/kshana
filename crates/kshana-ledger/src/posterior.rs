//! A real posterior over birth time, in place of a ranking by mean score.
//!
//! For each candidate birth instant on a grid across the stated window the chart is
//! cast and every logged event is read through that chart's exact
//! [`FitProfile`](crate::FitProfile): the fit at the event, `fit_e(t)`, and the mean
//! fit of the event's area over the lived span, `C_e(t)`, which is what an instant
//! nobody logged scores under that chart. The likelihood is the product over events
//! of `max(fit_e, floor) / max(C_e, floor)`: a chart is rewarded for explaining an
//! event only in proportion to how rarely it would explain a random instant, so a
//! chart that explains everything gains nothing. This is the continuous form of the
//! research's `q / C` likelihood ratio, with the engine's own fit in place of an
//! assumed sensitivity `q`; it avoids the staircase a threshold would make of the
//! posterior. The prior is uniform over the window; the posterior is the normalised
//! likelihood over the grid, and the mode, the mean and the 50 % and 90 %
//! highest-density sets are reported as exact instants, each set as its hull with the
//! number of separate runs it holds (the comb, when there is one).
//!
//! Cost: one chart per candidate, so the grid is capped at [`MAX_CANDIDATES`] and
//! the step widened to fit. The transit factor of every candidate is read from the
//! centre chart's [`TransitTable`](crate::TransitTable), since the natal Moon moves
//! under 0.6° per hour and the Moon-relative houses of Jupiter and Saturn do not
//! change across a window of hours except at a sign edge. Dasha boundaries, which
//! do move (hours per second of birth error at the deeper levels), are recomputed
//! exactly per candidate. Rodden rates every rectified time C; the words say so.

use crate::coverage::{fit_profile_with, TransitCache, TransitTable};
use crate::event::{iso_utc, LedgerError, LifeEvent};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_rules::{area, Facts};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_vedic::Ayanamsa;
use std::collections::HashMap;

/// The most candidate instants a scan evaluates; a finer request widens the step.
pub const MAX_CANDIDATES: usize = 400;
/// Floor under every fit and coverage in the likelihood ratio, so one zero cannot
/// veto a candidate.
pub const LIKELIHOOD_FLOOR: f64 = 0.05;

/// The posterior over birth time.
#[derive(Clone, Debug, PartialEq)]
pub struct Posterior {
    /// Candidate instants with their posterior mass, in time order; the masses sum to 1.
    pub grid: Vec<(Instant, f64)>,
    /// Grid step in nanoseconds (each candidate stands for a cell of this width).
    pub step_ns: i128,
    /// Log-likelihood of each candidate, in grid order.
    pub log_likelihood: Vec<f64>,
    /// The candidate with the most mass (the earliest on a tie).
    pub map: Instant,
    /// The posterior mean.
    pub mean: Instant,
    /// Hull of the 50 % highest-density set, as cell edges.
    pub hpd50: (Instant, Instant),
    /// Hull of the 90 % highest-density set, as cell edges.
    pub hpd90: (Instant, Instant),
    /// Separate runs in the 50 % set (1 = one interval).
    pub hpd50_runs: usize,
    /// Separate runs in the 90 % set.
    pub hpd90_runs: usize,
    /// Events used.
    pub events: usize,
    /// The result in words.
    pub words: String,
}

/// The hull and run count of the highest-density set holding at least `mass` of a
/// grid of `(instant, probability)` cells of width `step_ns`.
pub fn hpd_interval(
    grid: &[(Instant, f64)],
    step_ns: i128,
    mass: f64,
) -> ((Instant, Instant), usize) {
    if grid.is_empty() {
        let t = Instant::j2000();
        return ((t, t), 0);
    }
    let mut order: Vec<usize> = (0..grid.len()).collect();
    order.sort_by(|a, b| {
        grid[*b]
            .1
            .partial_cmp(&grid[*a].1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    let mut chosen = Vec::new();
    let mut acc = 0.0;
    for i in order {
        chosen.push(i);
        acc += grid[i].1;
        if acc >= mass - 1e-12 {
            break;
        }
    }
    chosen.sort_unstable();
    let runs = 1 + chosen.windows(2).filter(|w| w[1] != w[0] + 1).count();
    let lo = grid[chosen[0]].0.add_ns(-step_ns / 2);
    let hi = grid[*chosen.last().unwrap()].0.add_ns(step_ns / 2);
    ((lo, hi), runs)
}

fn duration_words(ns: i128) -> String {
    let s = ns.max(0) / NS_PER_SEC;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    match (h, m) {
        (0, 0) => format!("{sec} s"),
        (0, _) => format!("{m} min {sec} s"),
        _ => format!("{h} h {m} min {sec} s"),
    }
}

/// The posterior over `[center − half_window_s, center + half_window_s]` in steps of
/// `step_s` seconds (at least 1 s, widened to keep at most [`MAX_CANDIDATES`]
/// candidates), from the logged events. Refused without events.
#[allow(clippy::too_many_arguments)]
pub fn birth_posterior(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    center: Instant,
    half_window_s: i64,
    step_s: i64,
    events: &[LifeEvent],
) -> Result<Posterior, LedgerError> {
    if events.is_empty() {
        return Err(LedgerError::Refused(
            "no dated events, so every birth time is as likely as any other".into(),
        ));
    }
    for ev in events {
        if area(&ev.area).is_none() {
            return Err(LedgerError::UnknownArea(ev.area.clone()));
        }
    }
    let half = (half_window_s.max(0) as i128) * NS_PER_SEC;
    let mut step = (step_s.max(1) as i128) * NS_PER_SEC;
    let cells = |step: i128| (2 * half / step) as usize + 1;
    if cells(step) > MAX_CANDIDATES {
        let slots = (MAX_CANDIDATES - 1) as i128;
        step = (2 * half + slots - 1) / slots;
    }
    let n = cells(step);
    let first = center.add_ns(-half);
    let last_event = events.iter().map(|e| e.at).max().unwrap_or(center);
    let span_end = last_event.max(first.add_ns(step));

    let input = |t: Instant| ChartInput {
        instant: t,
        observer: *obs,
        system,
        ayanamsa,
        topocentric_moon: false,
    };
    // The centre chart supplies the transit samples for every candidate.
    let centre_chart = compute(spk, eop, &input(center))?;
    let centre_facts = Facts::from_chart(&centre_chart);
    let mut cache = TransitCache::new(spk, &centre_chart, ayanamsa);
    let table: TransitTable = cache
        .profile(
            &centre_facts,
            &events[0].area,
            events[0].kind,
            first,
            span_end,
        )?
        .transit_table();
    let fallback = table.lookup(center);

    let mut grid = Vec::with_capacity(n);
    let mut log_likelihood = Vec::with_capacity(n);
    for j in 0..n {
        let t = first.add_ns(step * j as i128);
        let ch = compute(spk, eop, &input(t))?;
        let facts = Facts::from_chart(&ch);
        let moon = ch.bodies[1].sidereal;
        let mut profiles = HashMap::new();
        let mut ll = 0.0;
        for ev in events {
            let key = (ev.area.clone(), ev.kind.is_adverse());
            if !profiles.contains_key(&key) {
                let a = area(&ev.area).ok_or_else(|| LedgerError::UnknownArea(ev.area.clone()))?;
                let p = fit_profile_with(&facts, moon, t, a, ev.kind, t, span_end, &mut |at| {
                    table
                        .lookup(at)
                        .or(fallback)
                        .ok_or_else(|| LedgerError::Refused("no transit samples".into()))
                })?;
                profiles.insert(key.clone(), p);
            }
            let p = &profiles[&key];
            let fit = p.fit_at(ev.at).unwrap_or(LIKELIHOOD_FLOOR);
            let coverage = p.mean_fit();
            ll += fit.max(LIKELIHOOD_FLOOR).ln() - coverage.max(LIKELIHOOD_FLOOR).ln();
        }
        grid.push((t, 0.0));
        log_likelihood.push(ll);
    }
    let top = log_likelihood
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<f64> = log_likelihood.iter().map(|l| (l - top).exp()).collect();
    let total: f64 = weights.iter().sum();
    for (g, w) in grid.iter_mut().zip(&weights) {
        g.1 = w / total;
    }
    let map = grid
        .iter()
        .fold(grid[0], |best, g| if g.1 > best.1 { *g } else { best })
        .0;
    let mean_offset: f64 = grid
        .iter()
        .map(|(t, p)| p * t.ns_since(center) as f64)
        .sum();
    let mean = center.add_ns(mean_offset.round() as i128);
    let (hpd50, hpd50_runs) = hpd_interval(&grid, step, 0.5);
    let (hpd90, hpd90_runs) = hpd_interval(&grid, step, 0.9);
    let words = format!(
        "Scanned {n} candidate birth instants from {} to {} in steps of {} against {} dated event{}. Most likely: {}; posterior mean {}. Half the mass lies in {} to {} (width {}{}); 90% in {} to {} (width {}{}). A rectified time is rated C by Rodden's convention: keep the recorded time beside it.",
        iso_utc(first),
        iso_utc(first.add_ns(step * (n as i128 - 1))),
        duration_words(step),
        events.len(),
        if events.len() == 1 { "" } else { "s" },
        iso_utc(map),
        iso_utc(mean),
        iso_utc(hpd50.0),
        iso_utc(hpd50.1),
        duration_words(hpd50.1.ns_since(hpd50.0)),
        if hpd50_runs > 1 {
            format!(", in {hpd50_runs} separate runs: a comb, not one time")
        } else {
            String::new()
        },
        iso_utc(hpd90.0),
        iso_utc(hpd90.1),
        duration_words(hpd90.1.ns_since(hpd90.0)),
        if hpd90_runs > 1 {
            format!(", in {hpd90_runs} separate runs")
        } else {
            String::new()
        }
    );
    Ok(Posterior {
        grid,
        step_ns: step,
        log_likelihood,
        map,
        mean,
        hpd50,
        hpd90,
        hpd50_runs,
        hpd90_runs,
        events: events.len(),
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;
    use crate::fixture::{kochi, noon};

    #[test]
    fn hpd_sets_nest_and_count_runs() {
        let t0 = Instant::j2000();
        let step = 10 * NS_PER_SEC;
        let grid: Vec<(Instant, f64)> = [0.04, 0.3, 0.06, 0.05, 0.3, 0.2, 0.05]
            .iter()
            .enumerate()
            .map(|(i, p)| (t0.add_ns(step * i as i128), *p))
            .collect();
        let ((lo, hi), runs) = hpd_interval(&grid, step, 0.5);
        // 0.3 (index 1) + 0.3 (index 4) = 0.6 ≥ 0.5: two runs.
        assert_eq!(runs, 2);
        assert_eq!(lo, t0.add_ns(step - step / 2));
        assert_eq!(hi, t0.add_ns(4 * step + step / 2));
        let ((lo9, hi9), runs9) = hpd_interval(&grid, step, 0.9);
        // + 0.2 (index 5) = 0.8, + 0.06 (index 2) = 0.86, + 0.05 (index 3) = 0.91:
        // indices 1 to 5, one run.
        assert_eq!(runs9, 1);
        assert!(lo9 <= lo && hi9 >= hi);
        assert_eq!(lo9, lo);
        assert_eq!(hi9, t0.add_ns(5 * step + step / 2));
        assert_eq!(hpd_interval(&[], step, 0.5).1, 0);
        assert_eq!(duration_words(65 * NS_PER_SEC), "1 min 5 s");
        assert_eq!(duration_words(3725 * NS_PER_SEC), "1 h 2 min 5 s");
        assert_eq!(duration_words(9 * NS_PER_SEC), "9 s");
    }

    #[test]
    fn kernel_posterior_is_a_distribution_over_the_window() {
        let Some((spk, ch)) = kochi() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
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
        ];
        let p = birth_posterior(
            &spk,
            None,
            &obs,
            HouseSystem::Placidus,
            Ayanamsa::Lahiri,
            ch.instant,
            30 * 60,
            120,
            &events,
        )
        .unwrap();
        assert_eq!(p.grid.len(), 31);
        assert_eq!(p.step_ns, 120 * NS_PER_SEC);
        assert_eq!(p.log_likelihood.len(), 31);
        let sum: f64 = p.grid.iter().map(|g| g.1).sum();
        assert!((sum - 1.0).abs() < 1e-9, "{sum}");
        assert!(p.grid.iter().all(|g| g.1 >= 0.0 && g.1.is_finite()));
        assert!(p
            .grid
            .windows(2)
            .all(|w| w[1].0.ns_since(w[0].0) == p.step_ns));
        assert_eq!(p.grid[0].0, ch.instant.add_ns(-30 * 60 * NS_PER_SEC));
        assert_eq!(p.grid[30].0, ch.instant.add_ns(30 * 60 * NS_PER_SEC));
        let max_mass = p.grid.iter().map(|g| g.1).fold(0.0, f64::max);
        let map_mass = p.grid.iter().find(|g| g.0 == p.map).unwrap().1;
        assert_eq!(map_mass, max_mass);
        assert!(p.hpd90.0 <= p.hpd50.0 && p.hpd90.1 >= p.hpd50.1);
        assert!(p.hpd50.0 <= p.map && p.map <= p.hpd50.1);
        assert!(p.mean >= p.grid[0].0 && p.mean <= p.grid[30].0);
        assert!(p.hpd50_runs >= 1 && p.hpd90_runs >= 1);
        assert!(p.words.contains("rated C"), "{}", p.words);
        assert!(p.words.contains("31 candidate"), "{}", p.words);
        // The cap widens the step.
        let capped = birth_posterior(
            &spk,
            None,
            &obs,
            HouseSystem::Placidus,
            Ayanamsa::Lahiri,
            ch.instant,
            3600,
            1,
            &events[..1],
        )
        .unwrap();
        assert!(capped.grid.len() <= MAX_CANDIDATES);
        assert!(capped.step_ns >= 18 * NS_PER_SEC);
        assert!(matches!(
            birth_posterior(
                &spk,
                None,
                &obs,
                HouseSystem::Placidus,
                Ayanamsa::Lahiri,
                ch.instant,
                600,
                60,
                &[]
            ),
            Err(LedgerError::Refused(_))
        ));
        let mut bad = events[0].clone();
        bad.area = "Pets".into();
        assert!(matches!(
            birth_posterior(
                &spk,
                None,
                &obs,
                HouseSystem::Placidus,
                Ayanamsa::Lahiri,
                ch.instant,
                600,
                60,
                &[bad]
            ),
            Err(LedgerError::UnknownArea(_))
        ));
    }
}
