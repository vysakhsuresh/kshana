//! The specificity discount: closing the garden of forking paths in event matching.
//!
//! An event's area is fixed at entry from the taxonomy in [`AREAS`], never chosen
//! after seeing which houses would fit. But the taxonomy itself overlaps: a lord that
//! signifies house 2 or 11 supports Career, Marriage, Children and Wealth at once, so
//! a Career hit on those houses is a hit that several other labels would also have
//! claimed. The rule here, after kpastro's specificity weight for a lord that
//! signifies many houses, counts for an area the expected number of *other* areas a
//! hit on one of its tested houses would also have counted for,
//! `n_eq = Σ_{B ≠ A} |S_A ∩ S_B| / |S_A|` (the tested set `S` is the group for a
//! favourable kind and the negating houses for an adverse one), and discounts the
//! fit to `fit / (1 + n_eq)`: the fit per equivalent label.
//!
//! The discounted fit is a weight for ranking and comparing areas, not a
//! probability: the support threshold and the coverage line work on the plain fit,
//! and the coverage already prices in how often a broad area fires by chance.

use crate::event::{EventKind, LedgerError, LifeEvent};
use crate::score::{score_event, EventScore};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::{area, Area, AREAS};
use kshana_vedic::Ayanamsa;

/// The houses an event of this kind in this area is tested against: the group for a
/// favourable kind, the negating houses for an adverse one.
pub fn tested_houses(a: Area, kind: EventKind) -> &'static [u8] {
    if kind.is_adverse() {
        a.negating
    } else {
        a.group
    }
}

/// The expected number of other areas a hit on one of this area's tested houses
/// would also have counted for: `Σ_{B ≠ A} |S_A ∩ S_B| / |S_A|`.
pub fn equivalent_areas(a: Area, kind: EventKind) -> f64 {
    let own = tested_houses(a, kind);
    if own.is_empty() {
        return 0.0;
    }
    AREAS
        .iter()
        .filter(|b| b.name != a.name)
        .map(|b| {
            let other = tested_houses(*b, kind);
            own.iter().filter(|h| other.contains(h)).count() as f64
        })
        .sum::<f64>()
        / own.len() as f64
}

/// `1 / (1 + equivalent_areas)`: 1 for an area no other label shares a tested house
/// with, smaller the more labels would have claimed the same hit.
pub fn specificity_factor(a: Area, kind: EventKind) -> f64 {
    1.0 / (1.0 + equivalent_areas(a, kind))
}

/// An event score with its specificity discount.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscountedScore {
    /// The plain score.
    pub score: EventScore,
    /// [`specificity_factor`] of the event's area and kind.
    pub factor: f64,
    /// [`equivalent_areas`] of the event's area and kind.
    pub equivalent_areas: f64,
    /// `score.fit × factor`.
    pub fit: f64,
    /// The discount in words.
    pub words: String,
}

/// Score an event and discount its fit by the specificity of its area.
pub fn score_event_discounted(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    ev: &LifeEvent,
) -> Result<DiscountedScore, LedgerError> {
    let a = area(&ev.area).ok_or_else(|| LedgerError::UnknownArea(ev.area.clone()))?;
    let score = score_event(spk, natal, ayanamsa, ev)?;
    let n_eq = equivalent_areas(a, ev.kind);
    let factor = specificity_factor(a, ev.kind);
    let fit = score.fit * factor;
    let houses = tested_houses(a, ev.kind);
    let words = format!(
        "A hit on {}'s tested houses {houses:?} would also have counted for {n_eq:.2} other areas on average, so the fit {:.2} is discounted by {factor:.2} to {fit:.2}: the fit per equivalent label, not a probability.",
        a.name, score.fit
    );
    Ok(DiscountedScore {
        score,
        factor,
        equivalent_areas: n_eq,
        fit,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};

    #[test]
    fn factors_follow_the_taxonomy() {
        let career = area("Career").unwrap();
        // Career {2, 6, 10, 11}: Marriage 2/4, Children 2/4, Wealth 3/4, Health 1/4,
        // Property 1/4, Education 1/4, Family 2/4, Foreign 0 → 3 other areas.
        assert!((equivalent_areas(career, EventKind::Gain) - 3.0).abs() < 1e-12);
        assert!((specificity_factor(career, EventKind::Gain) - 1.0 / 4.0).abs() < 1e-12);
        let foreign = area("Foreign").unwrap();
        // Foreign {3, 9, 12}: Education shares 9, Property 12 → 2/3.
        assert!((equivalent_areas(foreign, EventKind::Began) - 2.0 / 3.0).abs() < 1e-12);
        // Foreign negating {4}: Children {1, 4, 10} shares it → 1.
        assert!((equivalent_areas(foreign, EventKind::Loss) - 1.0).abs() < 1e-12);
        assert!((specificity_factor(foreign, EventKind::Crisis) - 0.5).abs() < 1e-12);
        for a in AREAS {
            for k in EventKind::ALL {
                let f = specificity_factor(a, k);
                assert!(f > 0.0 && f <= 1.0, "{} {k:?}: {f}", a.name);
                assert!(!tested_houses(a, k).is_empty());
            }
        }
    }

    #[test]
    fn kernel_discount_scales_the_fit() {
        let Some((spk, ch)) = kochi() else { return };
        let ev = LifeEvent::new(noon(2013, 7, 1), "Career", EventKind::Began, 0.9, "job").unwrap();
        let d = score_event_discounted(&spk, &ch, Ayanamsa::Lahiri, &ev).unwrap();
        assert!((d.factor - 1.0 / 4.0).abs() < 1e-12);
        assert!((d.fit - d.score.fit * d.factor).abs() < 1e-12);
        assert!(d.fit <= d.score.fit);
        assert!(d.words.contains("discounted by"), "{}", d.words);
        let mut bad = ev.clone();
        bad.area = "Pets".into();
        assert!(score_event_discounted(&spk, &ch, Ayanamsa::Lahiri, &bad).is_err());
    }
}
