//! The life map: every year from birth, the running periods and how each area fares.

use kshana_chart::Chart;
use kshana_rules::{period_quality, Facts, AREAS};
use kshana_time::{Instant, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{vimshottari, DashaLord};

/// One year of a life.
#[derive(Clone, Debug, PartialEq)]
pub struct YearRow {
    /// Age at the start of the year.
    pub age: u32,
    /// Instant the year starts.
    pub start: Instant,
    /// Mahadasha lord.
    pub maha: DashaLord,
    /// Antardasha lord at mid-year.
    pub antar: DashaLord,
    /// Area scores in `[0, 1]`, in [`AREAS`] order: mean of the maha and antar lords' quality.
    pub areas: [f64; AREAS.len()],
}

/// Years `0..=years` of a life.
pub fn life_map(natal: &Chart, years: u32) -> Vec<YearRow> {
    let facts = Facts::from_chart(natal);
    let moon = natal.bodies[1].sidereal;
    let birth = natal.instant;
    let mahas = vimshottari(moon, birth, YEAR_365_25_NS);
    let mut rows = Vec::with_capacity(years as usize + 1);
    for age in 0..=years {
        let start = birth.add_ns(age as i128 * YEAR_365_25_NS);
        let mid = start.add_ns(182 * NS_PER_DAY);
        let Some(m) = mahas.iter().find(|p| p.start <= mid && mid < p.end) else {
            break;
        };
        let subs = kshana_vedic::subdivide(*m);
        let a = subs
            .iter()
            .find(|p| p.start <= mid && mid < p.end)
            .map(|p| p.lord)
            .unwrap_or(m.lord);
        let mut areas = [0.0; AREAS.len()];
        for (i, area) in AREAS.iter().enumerate() {
            let (q1, _) = period_quality(&facts, *area, m.lord);
            let (q2, _) = period_quality(&facts, *area, a);
            areas[i] = 0.5 * (q1 + q2);
        }
        rows.push(YearRow {
            age,
            start,
            maha: m.lord,
            antar: a,
            areas,
        });
    }
    rows
}
