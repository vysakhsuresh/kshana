//! Exact windows of the transiting Moon: Gandanta, Panchaka, Bhadra, the named
//! inauspicious yogas, tara bala and chandra bala.
//!
//! Every edge is the Moon (or the Moon–Sun elongation, or the Moon + Sun sum) crossing
//! a fixed sidereal boundary, found with `kshana_trace::crossings` and solved to a
//! microsecond. The scan runs a margin beyond the requested range so that a window
//! already open at `from` or still open at `to` is reported with both edges exact;
//! windows are returned with their true edges, which may lie outside `[from, to]`.
//!
//! Definitions:
//!
//! * Gandanta: the Moon within 3°20′ of the water–fire junctions, i.e. the last pada of
//!   Cancer, Scorpio or Pisces (Ashlesha, Jyeshtha, Revati pada 4) or the first pada
//!   of Leo, Sagittarius or Aries (Magha, Mula, Ashwini pada 1); each window spans both
//!   padas (research brief; Muhurta Chintamani).
//! * Panchaka: the Moon from the third pada of Dhanishta, which begins at 300°
//!   (0° Aquarius), to the end of Revati at 360° (shubhpanchang.com/panchang/panchak,
//!   dekhopanchang.com/en/learn/panchak, r-astro.com/panchak).
//! * Bhadra: the Vishti karana, every seventh movable karana, ordinals 7, 14, 21, 28,
//!   35, 42, 49 and 56 of the sixty half-tithis counted from Kimstughna as ordinal 0
//!   (`kshana_vedic::karana`).
//! * Inauspicious yogas: Vishkambha, Atiganda, Shula, Ganda, Vyaghata, Vajra,
//!   Vyatipata, Parigha and Vaidhriti (Muhurta Chintamani; varahamihira.blogspot.com
//!   "yogas in muhurtha"); the whole yoga is reported, the texts' partial-ghati
//!   exemptions are not applied.
//! * Tara bala: the transit Moon's nakshatra counted from the natal nakshatra, cycle of
//!   nine (Janma, Sampat, Vipat, Kshema, Pratyak, Sadhana, Naidhana, Mitra, Parama
//!   Mitra); Vipat, Pratyak and Naidhana are unfavourable (panchangbodh.com/tara-bala;
//!   the same rule as `kshana_forecast::daily`).
//! * Chandra bala: the transit Moon's sign counted from the natal Moon's; the 1st, 3rd,
//!   6th, 7th, 10th and 11th are favourable, the 8th is Chandrashtama.

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{EventError, MICROSECOND_NS};
use kshana_sky::apparent;
use kshana_time::{Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_trace::{crossings, Division};
use kshana_vedic::panchanga::YOGA_NAMES;
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

/// A window with a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelledWindow {
    /// Exact start.
    pub start: Instant,
    /// Exact end.
    pub end: Instant,
    /// Half-width of the start's solved bracket, nanoseconds.
    pub start_half_width_ns: i128,
    /// Half-width of the end's solved bracket, nanoseconds.
    pub end_half_width_ns: i128,
    /// What the window is.
    pub label: &'static str,
    /// Cell index of the division at the window's start (pada, sign, karana, yoga or
    /// nakshatra index as the function documents).
    pub index: usize,
    /// Good to act in.
    pub favourable: bool,
}

const HOUR: i128 = 3_600 * NS_PER_SEC;

/// Windows of consecutive cells of `division` that carry the same label under `f`,
/// scanning `margin_ns` beyond both ends so that every window overlapping
/// `[from, to]` has both edges solved.
fn cell_windows<F, L>(
    f: F,
    division: Division,
    from: Instant,
    to: Instant,
    step_ns: i128,
    margin_ns: i128,
    label: L,
) -> Result<Vec<LabelledWindow>, EventError>
where
    F: Fn(Instant) -> Result<f64, SpkError>,
    L: Fn(usize) -> Option<(&'static str, bool)>,
{
    let scan_from = from.add_ns(-margin_ns);
    let mut scan_to = to.add_ns(margin_ns);
    let mut cs = crossings(&f, division, scan_from, scan_to, step_ns, MICROSECOND_NS)?;
    let mut out = Vec::new();
    let first = division.index(f(scan_from)?);
    let mut open = label(first).map(|l| (scan_from, 0i128, first, l));
    let mut i = 0;
    let mut extensions = 0;
    loop {
        if i == cs.len() {
            // Still open past the scan and overlapping the range: scan further, at most
            // eight more margins (a window longer than that would not be one of these).
            match open {
                Some((s, ..)) if s < to && extensions < 8 => {
                    extensions += 1;
                    let more = crossings(
                        &f,
                        division,
                        scan_to,
                        scan_to.add_ns(margin_ns),
                        step_ns,
                        MICROSECOND_NS,
                    )?;
                    scan_to = scan_to.add_ns(margin_ns);
                    if more.is_empty() {
                        continue;
                    }
                    cs.extend(more);
                }
                _ => break,
            }
        }
        let c = cs[i];
        i += 1;
        let next = label(c.entered);
        match (open, next) {
            (Some((_, _, _, l)), Some(nl)) if nl == l => {}
            (Some((s, shw, idx, l)), _) => {
                out.push(LabelledWindow {
                    start: s,
                    end: c.at,
                    start_half_width_ns: shw,
                    end_half_width_ns: c.half_width_ns,
                    label: l.0,
                    index: idx,
                    favourable: l.1,
                });
                open = next.map(|nl| (c.at, c.half_width_ns, c.entered, nl));
            }
            (None, Some(nl)) => open = Some((c.at, c.half_width_ns, c.entered, nl)),
            (None, None) => {}
        }
    }
    out.retain(|w| w.end > from && w.start < to);
    Ok(out)
}

fn moon_sidereal(spk: &Spk, ayanamsa: Ayanamsa) -> impl Fn(Instant) -> Result<f64, SpkError> + '_ {
    move |t| {
        let a = apparent(spk, Body::Moon, t)?;
        Ok(sidereal(
            a.longitude,
            ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000()),
        ))
    }
}

/// Gandanta windows overlapping `[from, to]`: the Moon in the last pada of Cancer,
/// Scorpio or Pisces or the first pada of Leo, Sagittarius or Aries. `index` is the
/// pada entered at the window's start (35, 71 or 107).
pub fn gandanta_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    cell_windows(
        moon_sidereal(spk, ayanamsa),
        Division::Pada,
        from,
        to,
        HOUR,
        NS_PER_DAY,
        |p| match p {
            35 | 36 => Some(("Cancer–Leo gandanta", false)),
            71 | 72 => Some(("Scorpio–Sagittarius gandanta", false)),
            107 | 0 => Some(("Pisces–Aries gandanta", false)),
            _ => None,
        },
    )
}

/// Panchaka windows overlapping `[from, to]`: the Moon from 0° Aquarius (Dhanishta
/// pada 3) to the end of Revati. `index` is the sign entered at the start (10).
pub fn panchaka_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    cell_windows(
        moon_sidereal(spk, ayanamsa),
        Division::Sign,
        from,
        to,
        2 * HOUR,
        6 * NS_PER_DAY,
        |s| (s == 10 || s == 11).then_some(("Panchaka", false)),
    )
}

/// Bhadra (Vishti karana) windows overlapping `[from, to]`. `index` is the karana
/// ordinal (7, 14, … 56).
pub fn bhadra_windows(
    spk: &Spk,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    let elong = |t: Instant| {
        let m = apparent(spk, Body::Moon, t)?.longitude;
        let s = apparent(spk, Body::Sun, t)?.longitude;
        Ok((m - s).rem_euclid(360.0))
    };
    cell_windows(
        elong,
        Division::Karana,
        from,
        to,
        2 * HOUR,
        NS_PER_DAY,
        |k| ((1..=56).contains(&k) && (k - 1) % 7 == 6).then_some(("Vishti (Bhadra)", false)),
    )
}

/// The yogas Muhurta Chintamani counts as inauspicious, as indices into
/// `kshana_vedic::panchanga::YOGA_NAMES`: Vishkambha, Atiganda, Shula, Ganda,
/// Vyaghata, Vajra, Vyatipata, Parigha, Vaidhriti.
pub const INAUSPICIOUS_YOGAS: [usize; 9] = [0, 5, 8, 9, 12, 14, 16, 18, 26];

/// Windows of the yogas listed in `which` (indices 0–26) overlapping `[from, to]`,
/// each labelled with the yoga's name and marked unfavourable. `index` is the yoga.
pub fn yoga_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
    which: &[usize],
) -> Result<Vec<LabelledWindow>, EventError> {
    let sum = |t: Instant| {
        let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
        let m = sidereal(apparent(spk, Body::Moon, t)?.longitude, ay);
        let s = sidereal(apparent(spk, Body::Sun, t)?.longitude, ay);
        Ok((m + s).rem_euclid(360.0))
    };
    cell_windows(
        sum,
        Division::Yoga,
        from,
        to,
        2 * HOUR,
        2 * NS_PER_DAY,
        |y| which.contains(&y).then_some((YOGA_NAMES[y % 27], false)),
    )
}

/// Windows of the nine inauspicious yogas overlapping `[from, to]`.
pub fn special_yoga_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    yoga_windows(spk, ayanamsa, from, to, &INAUSPICIOUS_YOGAS)
}

/// The nine taras in order from the natal nakshatra.
pub const TARA_NAMES: [&str; 9] = [
    "Janma",
    "Sampat",
    "Vipat",
    "Kshema",
    "Pratyak",
    "Sadhana",
    "Naidhana",
    "Mitra",
    "Parama Mitra",
];

/// Tara bala windows overlapping `[from, to]` for a natal Moon: one window per
/// nakshatra of the transit Moon, labelled with the tara and marked favourable unless
/// it is Vipat, Pratyak or Naidhana. `index` is the nakshatra (0 = Ashwini).
pub fn tara_bala_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    natal_moon_sidereal: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    let janma = Division::Nakshatra.index(natal_moon_sidereal);
    cell_windows(
        moon_sidereal(spk, ayanamsa),
        Division::Nakshatra,
        from,
        to,
        2 * HOUR,
        2 * NS_PER_DAY,
        move |n| {
            let count = (n + 27 - janma) % 27 % 9;
            Some((TARA_NAMES[count], !matches!(count, 2 | 4 | 6)))
        },
    )
}

/// Labels for the transit Moon's sign counted from the natal Moon's.
pub const CHANDRA_LABELS: [&str; 12] = [
    "Moon in the 1st from the natal Moon",
    "Moon in the 2nd from the natal Moon",
    "Moon in the 3rd from the natal Moon",
    "Moon in the 4th from the natal Moon",
    "Moon in the 5th from the natal Moon",
    "Moon in the 6th from the natal Moon",
    "Moon in the 7th from the natal Moon",
    "Chandrashtama: Moon in the 8th from the natal Moon",
    "Moon in the 9th from the natal Moon",
    "Moon in the 10th from the natal Moon",
    "Moon in the 11th from the natal Moon",
    "Moon in the 12th from the natal Moon",
];

/// Chandra bala windows overlapping `[from, to]` for a natal Moon: one window per
/// sidereal sign of the transit Moon, favourable in the 1st, 3rd, 6th, 7th, 10th and
/// 11th from the natal Moon's sign. `index` is the sign (0 = Aries).
pub fn chandra_bala_windows(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    natal_moon_sidereal: f64,
    from: Instant,
    to: Instant,
) -> Result<Vec<LabelledWindow>, EventError> {
    let natal = Division::Sign.index(natal_moon_sidereal);
    cell_windows(
        moon_sidereal(spk, ayanamsa),
        Division::Sign,
        from,
        to,
        2 * HOUR,
        3 * NS_PER_DAY,
        move |s| {
            let house = (s + 12 - natal) % 12 + 1;
            Some((
                CHANDRA_LABELS[house - 1],
                matches!(house, 1 | 3 | 6 | 7 | 10 | 11),
            ))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, utc};
    use crate::sidereal_longitude;
    use kshana_solver::wrap180;
    use kshana_vedic::{karana, nakshatra, yoga};

    const LAHIRI: Ayanamsa = Ayanamsa::Lahiri;

    fn moon(spk: &Spk, t: Instant) -> f64 {
        sidereal_longitude(spk, Body::Moon, t, LAHIRI).unwrap()
    }

    #[test]
    fn gandanta_windows_span_three_degrees_twenty_each_side() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 1, 0, 0, 0);
        let to = from.add_ns(30 * NS_PER_DAY);
        let w = gandanta_windows(&spk, LAHIRI, from, to).unwrap();
        assert!((3..=4).contains(&w.len()), "{}", w.len());
        for x in &w {
            let a = moon(&spk, x.start);
            let b = moon(&spk, x.end);
            let junction = match x.index {
                35 => 120.0,
                71 => 240.0,
                107 => 0.0,
                i => panic!("index {i}"),
            };
            assert!(
                wrap180(a - (junction - 10.0 / 3.0)).abs() < 1e-6,
                "start {a}"
            );
            assert!(wrap180(b - (junction + 10.0 / 3.0)).abs() < 1e-6, "end {b}");
            let hours = x.end.ns_since(x.start) as f64 / 3.6e12;
            assert!((9.0..=16.0).contains(&hours), "{hours} h");
            assert!(!x.favourable);
            assert!(
                x.start_half_width_ns <= MICROSECOND_NS && x.end_half_width_ns <= MICROSECOND_NS
            );
        }
        assert!(w.windows(2).all(|p| p[0].end < p[1].start));
    }

    #[test]
    fn panchaka_runs_from_aquarius_to_the_end_of_revati() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 1, 0, 0, 0);
        let w = panchaka_windows(&spk, LAHIRI, from, from.add_ns(35 * NS_PER_DAY)).unwrap();
        assert!((1..=2).contains(&w.len()), "{}", w.len());
        for x in &w {
            assert_eq!(x.index, 10);
            assert!(wrap180(moon(&spk, x.start) - 300.0).abs() < 1e-6);
            assert!(wrap180(moon(&spk, x.end)).abs() < 1e-6);
            let days = x.end.ns_since(x.start) as f64 / NS_PER_DAY as f64;
            assert!((4.0..=6.0).contains(&days), "{days} d");
            assert_eq!(x.label, "Panchaka");
        }
        // a window open at `from` is reported with its true start before `from`
        let inside = w[0].start.add_ns(NS_PER_DAY);
        let again = panchaka_windows(&spk, LAHIRI, inside, inside.add_ns(NS_PER_DAY)).unwrap();
        assert_eq!(again.len(), 1);
        assert!(again[0].start.ns_since(w[0].start).abs() <= 2 * MICROSECOND_NS);
        assert!(again[0].end.ns_since(w[0].end).abs() <= 2 * MICROSECOND_NS);
        assert_eq!(again[0].label, w[0].label);
    }

    #[test]
    fn bhadra_windows_are_vishti_karanas() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 1, 0, 0, 0);
        let w = bhadra_windows(&spk, from, from.add_ns(30 * NS_PER_DAY)).unwrap();
        assert!((8..=9).contains(&w.len()), "{}", w.len());
        for x in &w {
            assert_eq!(x.index % 7, 0);
            let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
            let m = apparent(&spk, Body::Moon, mid).unwrap().longitude;
            let s = apparent(&spk, Body::Sun, mid).unwrap().longitude;
            let k = karana(m, s);
            assert_eq!(k.name, "Vishti");
            assert_eq!(k.ordinal as usize, x.index);
            let hours = x.end.ns_since(x.start) as f64 / 3.6e12;
            assert!((8.0..=18.0).contains(&hours), "{hours} h");
            // just before the start the karana is the previous one
            let before = karana(
                apparent(&spk, Body::Moon, x.start.add_ns(-NS_PER_SEC))
                    .unwrap()
                    .longitude,
                apparent(&spk, Body::Sun, x.start.add_ns(-NS_PER_SEC))
                    .unwrap()
                    .longitude,
            );
            assert_eq!(before.ordinal as usize + 1, x.index);
        }
    }

    #[test]
    fn special_yoga_windows_match_the_yoga_at_their_midpoint() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 1, 0, 0, 0);
        let w = special_yoga_windows(&spk, LAHIRI, from, from.add_ns(30 * NS_PER_DAY)).unwrap();
        assert!((9..=12).contains(&w.len()), "{}", w.len());
        for x in &w {
            assert!(INAUSPICIOUS_YOGAS.contains(&x.index));
            let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
            let m = sidereal_longitude(&spk, Body::Moon, mid, LAHIRI).unwrap();
            let s = sidereal_longitude(&spk, Body::Sun, mid, LAHIRI).unwrap();
            let y = yoga(m, s);
            assert_eq!(y.index, x.index);
            assert_eq!(y.name(), x.label);
        }
        let v = yoga_windows(&spk, LAHIRI, from, from.add_ns(30 * NS_PER_DAY), &[16]).unwrap();
        assert!((1..=2).contains(&v.len()));
        assert!(v.iter().all(|x| x.label == "Vyatipata"));
    }

    #[test]
    fn tara_and_chandra_bala_windows_tile_the_range() {
        let Some(spk) = kernel() else { return };
        let natal = 316.344; // Shatabhisha, Aquarius
        let from = utc(2026, 10, 1, 0, 0, 0);
        let to = from.add_ns(10 * NS_PER_DAY);
        let t = tara_bala_windows(&spk, LAHIRI, natal, from, to).unwrap();
        assert!((10..=12).contains(&t.len()), "{}", t.len());
        assert!(t[0].start <= from && t[t.len() - 1].end >= to);
        for p in t.windows(2) {
            assert_eq!(p[0].end, p[1].start);
            assert_eq!((p[0].index + 1) % 27, p[1].index);
        }
        let janma = nakshatra(natal).index;
        for x in &t {
            let mid = x.start.add_ns(x.end.ns_since(x.start) / 2);
            let n = nakshatra(moon(&spk, mid));
            assert_eq!(n.index, x.index);
            let count = (n.index + 27 - janma) % 27 % 9;
            assert_eq!(x.label, TARA_NAMES[count]);
            assert_eq!(x.favourable, !matches!(count, 2 | 4 | 6));
        }
        let to = from.add_ns(30 * NS_PER_DAY);
        let c = chandra_bala_windows(&spk, LAHIRI, natal, from, to).unwrap();
        assert!((12..=14).contains(&c.len()), "{}", c.len());
        for p in c.windows(2) {
            assert_eq!(p[0].end, p[1].start);
            assert_eq!((p[0].index + 1) % 12, p[1].index);
        }
        let eighth = c
            .iter()
            .find(|x| x.label.starts_with("Chandrashtama"))
            .unwrap();
        assert_eq!((eighth.index + 12 - 10) % 12, 7);
        assert!(!eighth.favourable);
        assert!(c.iter().filter(|x| x.favourable).count() >= 6);
    }
}
