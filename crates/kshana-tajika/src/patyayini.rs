//! Patyayini dasha: the annual chart's own dasha, cut by the krishamsas of the lagna and
//! the seven grahas.
//!
//! Rule implemented (Tajika school, *Tajika Neelakanthi*; K. S. Charak, *A Textbook of
//! Varshaphala*, the Patyayini dasha chapter; B. V. Raman, *Varshaphal*; coded the same
//! way in PyJHora `src/jhora/horoscope/dhasa/annual/patyayini.py`):
//!
//! 1. The krishamsa of each of the eight — the annual lagna and Sun … Saturn; Rahu and
//!    Ketu take no part — is its longitude within its sign, 0°–30°.
//! 2. They are arranged in ascending order of krishamsa. The first one's patyamsa is its
//!    own krishamsa; every later one's patyamsa is its krishamsa minus the previous one's.
//!    The patyamsas therefore add up to the largest krishamsa.
//! 3. Each dasha, in that order from the least krishamsa, gets the share
//!    `patyamsa / Σ patyamsa` of the year.
//! 4. Antardashas: within each dasha, the eight in the same order starting from the
//!    dasha's own lord (cyclically), each with the same share `patyamsa / Σ patyamsa` of
//!    the dasha (PyJHora's ordering).
//!
//! Conventions: the year is the interval from this sidereal solar return to the next, as
//! for the Mudda dasha in this crate (PyJHora uses a fixed average year; Charak 360
//! days). Equal krishamsas keep the lagna-then-[`SEVEN`] order (a stable sort) and give a
//! zero-length period, which is kept so the eight always appear.

use kshana_time::Instant;
use kshana_vedic::DashaLord;

use crate::sky::AnnualSky;
use crate::tables::{within_sign, SEVEN};

/// A Patyayini lord: the annual lagna or one of the seven grahas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatyayiniLord {
    /// The annual ascendant.
    Lagna,
    /// One of Sun … Saturn.
    Graha(DashaLord),
}

/// One antardasha of a Patyayini period.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatyayiniSub {
    /// Lord of the sub-period.
    pub lord: PatyayiniLord,
    /// Exact start.
    pub start: Instant,
    /// Exact end.
    pub end: Instant,
}

/// One Patyayini dasha with its antardashas.
#[derive(Clone, Debug, PartialEq)]
pub struct PatyayiniPeriod {
    /// Lord of the period.
    pub lord: PatyayiniLord,
    /// Its krishamsa: degrees within its sign.
    pub krishamsa: f64,
    /// Its patyamsa: krishamsa minus the previous (smaller) one's.
    pub patyamsa: f64,
    /// Exact start.
    pub start: Instant,
    /// Exact end (= the next period's start).
    pub end: Instant,
    /// The eight antardashas from the period's own lord.
    pub antardashas: Vec<PatyayiniSub>,
}

/// The eight `(lord, krishamsa, patyamsa)` in dasha order (ascending krishamsa).
pub fn patyamsas(sky: &AnnualSky) -> Vec<(PatyayiniLord, f64, f64)> {
    let mut k: Vec<(PatyayiniLord, f64)> = Vec::with_capacity(8);
    k.push((PatyayiniLord::Lagna, within_sign(sky.lagna)));
    for (i, g) in SEVEN.iter().enumerate() {
        k.push((PatyayiniLord::Graha(*g), within_sign(sky.longitudes[i])));
    }
    k.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut prev = 0.0;
    k.into_iter()
        .map(|(l, x)| {
            let p = x - prev;
            prev = x;
            (l, x, p)
        })
        .collect()
}

/// Cut `[start, end)` by cumulative weights; the last piece ends exactly at `end`.
fn cut(start: Instant, end: Instant, weights: &[f64]) -> Vec<(Instant, Instant)> {
    let total = end.ns_since(start) as f64;
    let sum: f64 = weights.iter().sum();
    let mut out = Vec::with_capacity(weights.len());
    let mut t = start;
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        acc += w;
        let e = if i + 1 == weights.len() {
            end
        } else {
            start.add_ns((total * acc / sum).round() as i128)
        };
        out.push((t, e));
        t = e;
    }
    out
}

/// The year's Patyayini dashas from `start` (this return) to `end` (the next), with
/// antardashas. Empty when every krishamsa is zero.
pub fn patyayini(sky: &AnnualSky, start: Instant, end: Instant) -> Vec<PatyayiniPeriod> {
    let rows = patyamsas(sky);
    let weights: Vec<f64> = rows.iter().map(|r| r.2).collect();
    if weights.iter().sum::<f64>() <= 0.0 {
        return Vec::new();
    }
    let n = rows.len();
    cut(start, end, &weights)
        .into_iter()
        .enumerate()
        .map(|(d, (s, e))| {
            let order: Vec<usize> = (0..n).map(|k| (d + k) % n).collect();
            let w: Vec<f64> = order.iter().map(|i| weights[*i]).collect();
            let antardashas = cut(s, e, &w)
                .into_iter()
                .zip(order)
                .map(|((a, b), i)| PatyayiniSub {
                    lord: rows[i].0,
                    start: a,
                    end: b,
                })
                .collect();
            PatyayiniPeriod {
                lord: rows[d].0,
                krishamsa: rows[d].1,
                patyamsa: rows[d].2,
                start: s,
                end: e,
                antardashas,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;

    /// Lagna 12° Leo; in [`SEVEN`] order (Moon, Mercury, Venus, Sun, Mars, Jupiter,
    /// Saturn) krishamsas 25°, 3°, 18°, 6°, 9°, 27°, 15°.
    fn sky() -> AnnualSky {
        let longitudes: Vec<f64> = SEVEN
            .iter()
            .map(|g| match g {
                DashaLord::Moon => 25.0 + 30.0,
                DashaLord::Mercury => 3.0 + 300.0,
                DashaLord::Venus => 18.0 + 90.0,
                DashaLord::Sun => 6.0 + 270.0,
                DashaLord::Mars => 9.0,
                DashaLord::Jupiter => 27.0 + 150.0,
                _ => 15.0 + 210.0, // Saturn
            })
            .collect();
        AnnualSky {
            lagna: 120.0 + 12.0,
            cusps: [0.0; 12],
            longitudes: longitudes.try_into().unwrap(),
            speeds: [0.0; 7],
        }
    }

    #[test]
    fn worked_order_and_shares() {
        // Ascending: Mercury 3, Sun 6, Mars 9, Lagna 12, Saturn 15, Venus 18, Moon 25,
        // Jupiter 27. Patyamsas 3, 3, 3, 3, 3, 3, 7, 2; their sum is 27, the largest.
        let p = patyamsas(&sky());
        let lords: Vec<PatyayiniLord> = p.iter().map(|r| r.0).collect();
        use DashaLord::*;
        use PatyayiniLord::*;
        assert_eq!(
            lords,
            vec![
                Graha(Mercury),
                Graha(Sun),
                Graha(Mars),
                Lagna,
                Graha(Saturn),
                Graha(Venus),
                Graha(Moon),
                Graha(Jupiter)
            ]
        );
        let pats: Vec<f64> = p.iter().map(|r| r.2).collect();
        let want = [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 7.0, 2.0];
        for (a, b) in pats.iter().zip(want) {
            assert!((a - b).abs() < 1e-9, "{pats:?}");
        }
        // A 365-day year: Mercury gets 3/27 of it = 40.555… days, the Moon 7/27 = 94.63.
        let start = Instant::j2000();
        let end = start.add_ns(365 * NS_PER_DAY);
        let v = patyayini(&sky(), start, end);
        assert_eq!(v.len(), 8);
        let days = |a: Instant, b: Instant| b.ns_since(a) as f64 / NS_PER_DAY as f64;
        assert!((days(v[0].start, v[0].end) - 365.0 * 3.0 / 27.0).abs() < 1e-6);
        assert!((days(v[6].start, v[6].end) - 365.0 * 7.0 / 27.0).abs() < 1e-6);
        assert_eq!(v[0].start, start);
        assert_eq!(v[7].end, end);
        for w in v.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }

    #[test]
    fn antardashas_start_from_the_own_lord() {
        let start = Instant::j2000();
        let end = start.add_ns(365 * NS_PER_DAY);
        let v = patyayini(&sky(), start, end);
        let moon = &v[6];
        assert_eq!(moon.lord, PatyayiniLord::Graha(DashaLord::Moon));
        let a = &moon.antardashas;
        assert_eq!(a.len(), 8);
        assert_eq!(a[0].lord, moon.lord);
        assert_eq!(a[1].lord, PatyayiniLord::Graha(DashaLord::Jupiter));
        assert_eq!(a[2].lord, PatyayiniLord::Graha(DashaLord::Mercury));
        assert_eq!(a[0].start, moon.start);
        assert_eq!(a[7].end, moon.end);
        for w in a.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        // Moon in Moon: 7/27 of 7/27 of the year
        let d = a[0].end.ns_since(a[0].start) as f64 / NS_PER_DAY as f64;
        assert!((d - 365.0 * 49.0 / 729.0).abs() < 1e-6, "{d}");
    }

    #[test]
    fn all_zero_krishamsas_give_nothing() {
        let s = AnnualSky {
            lagna: 30.0,
            cusps: [0.0; 12],
            longitudes: [0.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0],
            speeds: [0.0; 7],
        };
        let start = Instant::j2000();
        assert!(patyayini(&s, start, start.add_ns(NS_PER_DAY)).is_empty());
    }
}
