//! The circular time-shift surrogate test: a p-value that keeps the shape of a life.
//!
//! Real lives cluster their events, in the twenties and thirties and around each
//! other, so a null that scatters them independently is too easy to beat. The
//! surrogate keeps every gap between events and moves the whole set by one offset
//! `u`, wrapping around the span: `t ↦ from + (t − from + u) mod T`. Rescoring the
//! shifted set against the same [`FitProfile`](crate::FitProfile) asks whether the
//! periods would have explained this same life just as well had it happened at
//! another time. With `k` offsets spread evenly over the span, deterministic, the
//! p-value is `(1 + #{shifted mean ≥ real mean}) / (1 + k)`, the real configuration
//! counting as one of the draws (Phipson and Smyth's correction, as in elephant's
//! surrogates). Every fit is one binary search in the profile, so ten thousand shifts
//! of forty events run in well under a second.

use crate::coverage::{ProfileSet, TransitCache};
use crate::event::{LedgerError, LifeEvent};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::Facts;
use kshana_time::Instant;
use kshana_vedic::Ayanamsa;

/// The outcome of a time-shift test.
#[derive(Clone, Debug, PartialEq)]
pub struct SurrogateResult {
    /// `(1 + #{shifted mean ≥ real mean}) / (1 + shifts)`.
    pub p: f64,
    /// Mean fit of the events where they are.
    pub real_mean: f64,
    /// Mean fit of the events under each offset, in offset order.
    pub shifted_means: Vec<f64>,
    /// Offsets used.
    pub shifts: usize,
    /// Events scored.
    pub n: usize,
    /// The result in words.
    pub words: String,
}

/// Move `at` by `offset_ns` within `[from, to)`, wrapping around.
pub fn circular_shift(at: Instant, from: Instant, to: Instant, offset_ns: i128) -> Instant {
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let span = to.ns_since(from);
    if span <= 0 {
        return from;
    }
    from.add_ns((at.ns_since(from) + offset_ns).rem_euclid(span))
}

/// The time-shift test of a ledger over `[from, to]` with `shifts` offsets
/// `T·j/(shifts + 1)`, `j = 1..=shifts`. Events outside the span are wrapped into
/// it before scoring, so give the span the events fall in. Refused when the span
/// holds no time after birth.
#[allow(clippy::too_many_arguments)]
pub fn time_shift_pvalue(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    events: &[LifeEvent],
    from: Instant,
    to: Instant,
    shifts: usize,
) -> Result<SurrogateResult, LedgerError> {
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let from = from.max(natal.instant);
    if to <= from {
        return Err(LedgerError::Refused(
            "the span holds no time after birth, so there is nothing to shift".into(),
        ));
    }
    if events.is_empty() {
        return Err(LedgerError::Refused("no events to shift".into()));
    }
    let facts = Facts::from_chart(natal);
    let mut cache = TransitCache::new(spk, natal, ayanamsa);
    let set = ProfileSet::build(&mut cache, &facts, events, from, to)?;
    let span = to.ns_since(from);
    let mean_at = |offset: i128| -> f64 {
        events
            .iter()
            .map(|ev| {
                let t = circular_shift(ev.at, from, to, offset);
                set.get(ev).fit_at(t).unwrap_or(0.0)
            })
            .sum::<f64>()
            / events.len() as f64
    };
    let real_mean = mean_at(0);
    let shifted_means: Vec<f64> = (1..=shifts)
        .map(|j| mean_at(span * j as i128 / (shifts as i128 + 1)))
        .collect();
    let at_least = shifted_means.iter().filter(|m| **m >= real_mean).count();
    let p = (1 + at_least) as f64 / (1 + shifts) as f64;
    let words = if shifts == 0 {
        format!(
            "The {} events score a mean fit of {real_mean:.2} where they are; no shifts were asked for, so there is no p-value.",
            events.len()
        )
    } else {
        let best = shifted_means
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let shifted_avg = shifted_means.iter().sum::<f64>() / shifts as f64;
        let verdict = if p <= 0.05 {
            "few other placements of this life score as well: the periods fit where the events are"
        } else if p <= 0.2 {
            "some other placements score as well: suggestive, not proof"
        } else {
            "many other placements of the same life score as well: the periods would have explained it wherever it fell"
        };
        format!(
            "The {} events score a mean fit of {real_mean:.2} where they are; moved together by {shifts} offsets around the span they average {shifted_avg:.2} (best {best:.2}), and {at_least} of {shifts} shifts score at least as well: p = {p:.3}, so {verdict}.",
            events.len()
        )
    };
    Ok(SurrogateResult {
        p,
        real_mean,
        shifted_means,
        shifts,
        n: events.len(),
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;
    use crate::fixture::{kochi, noon};
    use kshana_time::NS_PER_DAY;

    #[test]
    fn circular_shift_wraps() {
        let from = Instant::j2000();
        let to = from.add_ns(10 * NS_PER_DAY);
        let t = from.add_ns(8 * NS_PER_DAY);
        assert_eq!(circular_shift(t, from, to, 0), t);
        assert_eq!(
            circular_shift(t, from, to, 3 * NS_PER_DAY),
            from.add_ns(NS_PER_DAY)
        );
        assert_eq!(
            circular_shift(t, from, to, -9 * NS_PER_DAY),
            from.add_ns(9 * NS_PER_DAY)
        );
        assert_eq!(
            circular_shift(to.add_ns(NS_PER_DAY), from, to, 0),
            from.add_ns(NS_PER_DAY)
        );
        assert_eq!(
            circular_shift(t, to, from, 3 * NS_PER_DAY),
            from.add_ns(NS_PER_DAY)
        );
        assert_eq!(circular_shift(t, from, from, 5), from);
    }

    #[test]
    fn kernel_shift_test_gives_a_probability() {
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
        ];
        let r = time_shift_pvalue(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &events,
            noon(2005, 1, 1),
            noon(2026, 1, 1),
            200,
        )
        .unwrap();
        assert_eq!(r.shifted_means.len(), 200);
        assert_eq!(r.n, 3);
        assert!((0.0..=1.0).contains(&r.p), "{}", r.p);
        assert!(r.p >= 1.0 / 201.0);
        assert!((0.0..=1.0).contains(&r.real_mean));
        assert!(r.shifted_means.iter().all(|m| (0.0..=1.0).contains(m)));
        let at_least = r
            .shifted_means
            .iter()
            .filter(|m| **m >= r.real_mean)
            .count();
        assert!((r.p - (1 + at_least) as f64 / 201.0).abs() < 1e-12);
        assert!(r.words.contains("p = "), "{}", r.words);
        // Deterministic.
        let again = time_shift_pvalue(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &events,
            noon(2005, 1, 1),
            noon(2026, 1, 1),
            200,
        )
        .unwrap();
        assert_eq!(again, r);
        assert!(matches!(
            time_shift_pvalue(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                &events,
                noon(1980, 1, 1),
                noon(1985, 1, 1),
                10
            ),
            Err(LedgerError::Refused(_))
        ));
        assert!(matches!(
            time_shift_pvalue(
                &spk,
                &ch,
                Ayanamsa::Lahiri,
                &[],
                noon(2005, 1, 1),
                noon(2026, 1, 1),
                10
            ),
            Err(LedgerError::Refused(_))
        ));
        let none = time_shift_pvalue(
            &spk,
            &ch,
            Ayanamsa::Lahiri,
            &events,
            noon(2005, 1, 1),
            noon(2026, 1, 1),
            0,
        )
        .unwrap();
        assert_eq!(none.p, 1.0);
        assert!(none.words.contains("no shifts"));
    }
}
