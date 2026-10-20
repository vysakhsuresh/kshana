//! Retrodiction: what the chart said about each antardasha of a span, before the
//! person logs anything. Each area's strength is the KP promise of the natal chart
//! times the mean Parashari quality of the maha and antar lords, so an area that is
//! not promised stays at zero however good the period; Jupiter's and Saturn's houses
//! from the Moon at mid-period give the sky's part in words.

use crate::event::{month_year, LedgerError};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_forecast::transits_at;
use kshana_rules::{period_quality, promise, Facts, AREAS};

/// How many areas a reading scores, one per entry of [`AREAS`].
pub const N_AREAS: usize = AREAS.len();
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{subdivide, vimshottari, Ayanamsa, DashaLord};

/// One antardasha read from the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodReading {
    /// Exact start of the antardasha.
    pub start: Instant,
    /// Exact end (the next antardasha's start).
    pub end: Instant,
    /// Mahadasha lord.
    pub maha: DashaLord,
    /// Antardasha lord.
    pub antar: DashaLord,
    /// Per area in [`AREAS`] order: natal promise strength × mean period quality of
    /// the two lords, in `[0, 1]`.
    pub area_strengths: [f64; N_AREAS],
    /// Up to three area names with strength above zero, strongest first.
    pub top_areas: Vec<String>,
    /// Jupiter's house from the natal Moon at mid-period, 1–12.
    pub jupiter_from_moon: u8,
    /// Saturn's house from the natal Moon at mid-period, 1–12.
    pub saturn_from_moon: u8,
    /// Sade Sati phase at mid-period: 0 none, 1 rising, 2 peak, 3 setting.
    pub sade_sati_phase: u8,
    /// The reading in words.
    pub words: String,
}

fn ordinal(h: u8) -> String {
    match h {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

/// Every antardasha overlapping `[from, to]`, in order, with full (unclipped) bounds,
/// so consecutive readings share their boundary instants.
pub fn retrodict(
    spk: &Spk,
    natal: &Chart,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<PeriodReading>, LedgerError> {
    let (from, to) = if to < from { (to, from) } else { (from, to) };
    let facts = Facts::from_chart(natal);
    let promised: [f64; N_AREAS] = AREAS.map(|a| promise(&facts, a).strength);
    let mut out = Vec::new();
    for m in vimshottari(natal.bodies[1].sidereal, natal.instant, YEAR_365_25_NS) {
        if m.end <= from || m.start > to {
            continue;
        }
        let maha_q: [f64; N_AREAS] = AREAS.map(|a| period_quality(&facts, a, m.lord).0);
        for p in subdivide(m) {
            if p.end <= from || p.start > to {
                continue;
            }
            let antar_q: [f64; N_AREAS] = AREAS.map(|a| period_quality(&facts, a, p.lord).0);
            let area_strengths: [f64; N_AREAS] = std::array::from_fn(|i| {
                (promised[i] * 0.5 * (maha_q[i] + antar_q[i])).clamp(0.0, 1.0)
            });
            let mut order: Vec<usize> = (0..N_AREAS).collect();
            order.sort_by(|a, b| {
                area_strengths[*b]
                    .partial_cmp(&area_strengths[*a])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let top_areas: Vec<String> = order
                .iter()
                .filter(|i| area_strengths[**i] > 0.0)
                .take(3)
                .map(|i| AREAS[*i].name.to_string())
                .collect();
            let mid = p.start.add_ns(p.end.ns_since(p.start) / 2);
            let transits = transits_at(spk, natal, mid, ayanamsa)?;
            let house_from_moon = |body: &str| {
                transits
                    .iter()
                    .find(|t| t.body == body && t.natal == "Moon")
                    .map_or(0, |t| t.house)
            };
            let jupiter_from_moon = house_from_moon("Jupiter");
            let saturn_from_moon = house_from_moon("Saturn");
            let sade_sati_phase = match saturn_from_moon {
                12 => 1,
                1 => 2,
                2 => 3,
                _ => 0,
            };

            let mut words = format!(
                "{:?}'s sub-period in {:?}'s mahadasha, {} to {}: ",
                p.lord,
                m.lord,
                month_year(p.start),
                month_year(p.end)
            );
            if top_areas.is_empty() {
                words
                    .push_str("no area was both promised by the cusps and carried by these lords.");
            } else {
                let named: Vec<String> = order
                    .iter()
                    .filter(|i| area_strengths[**i] > 0.0)
                    .take(3)
                    .map(|i| format!("{} ({:.2})", AREAS[*i].name, area_strengths[*i]))
                    .collect();
                let weakest = order[7];
                words.push_str(&format!(
                    "{} stood out; {} ({:.2}) was weakest.",
                    named.join(", "),
                    AREAS[weakest].name,
                    area_strengths[weakest]
                ));
            }
            words.push_str(&format!(
                " At mid-period Jupiter was in the {} and Saturn in the {} from your Moon",
                ordinal(jupiter_from_moon),
                ordinal(saturn_from_moon)
            ));
            if sade_sati_phase != 0 {
                words.push_str(&format!(", Sade Sati phase {sade_sati_phase} of 3"));
            }
            words.push('.');

            out.push(PeriodReading {
                start: p.start,
                end: p.end,
                maha: m.lord,
                antar: p.lord,
                area_strengths,
                top_areas,
                jupiter_from_moon,
                saturn_from_moon,
                sade_sati_phase,
                words,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kochi, noon};

    #[test]
    fn readings_cover_the_range_contiguously() {
        let Some((spk, ch)) = kochi() else { return };
        let from = noon(2010, 1, 1);
        let to = noon(2020, 1, 1);
        let r = retrodict(&spk, &ch, Ayanamsa::Lahiri, from, to).unwrap();
        assert!(r.len() >= 4, "{}", r.len());
        assert!(r[0].start <= from && r[0].end > from);
        assert!(r.last().unwrap().end >= to && r.last().unwrap().start <= to);
        for w in r.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        for p in &r {
            assert!(p.start < p.end);
            assert!(p.area_strengths.iter().all(|s| (0.0..=1.0).contains(s)));
            assert!(p.top_areas.len() <= 3);
            for w in p.top_areas.windows(2) {
                let i = AREAS.iter().position(|a| a.name == w[0]).unwrap();
                let j = AREAS.iter().position(|a| a.name == w[1]).unwrap();
                assert!(p.area_strengths[i] >= p.area_strengths[j]);
            }
            assert!((1..=12).contains(&p.jupiter_from_moon));
            assert!((1..=12).contains(&p.saturn_from_moon));
            assert!(p.words.contains("sub-period in"), "{}", p.words);
            assert!(p.words.contains("from your Moon"), "{}", p.words);
        }
        // Ten years hold at least two mahadasha lords' worth of sub-periods or one long one.
        assert!(r.iter().any(|p| p.maha != p.antar));
        let swapped = retrodict(&spk, &ch, Ayanamsa::Lahiri, to, from).unwrap();
        assert_eq!(swapped, r);
    }
}
