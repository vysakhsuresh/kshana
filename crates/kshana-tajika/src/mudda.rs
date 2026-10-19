//! Mudda dasha (Varsha Vimshottari): the Vimshottari cycle compressed into one solar year.
//!
//! Rule implemented (K. S. Charak, *A Textbook of Varshaphala*, ch. V, Table V-2, pp.
//! 40–43, following the *Tajika Neelakanthi* appendix v. 1; the same rule is coded in
//! PyJHora's `mudda.py`):
//!
//! 1. The starting lord is the Vimshottari lord of the nakshatra reached by counting
//!    `completed years` nakshatras forward from the natal Moon's nakshatra; that is the
//!    natal lord advanced one lord per completed year. Charak states it as
//!    `(natal nakshatra number + completed years − 2) mod 9` into the order Venus, Sun,
//!    Moon, Mars, Rahu, Jupiter, Saturn, Mercury, Ketu, which is the same thing. For
//!    Charak's worked chart (Purva Phalguni, 40 years) it gives Rahu.
//! 2. The nine lords run in Vimshottari order with their years as shares of the year.
//!    The first lord's share is reduced by the fraction of the natal nakshatra the Moon
//!    had already traversed at birth (the natal balance, as Charak's example: Rahu 54
//!    days × 572/800 = 38.61), and the elapsed remainder of that lord closes the year, so
//!    there are ten periods when the balance is not nil.
//! 3. The year is the interval between this sidereal return and the next, not 360 civil
//!    days: Charak prints 360 days, the *Neelakanthi* commentary defines the year by the
//!    Sun's return, and this crate has the exact next return in hand.
//! 4. Antardashas: each period starts with its own lord and runs the cycle in
//!    Vimshottari proportion ([`kshana_vedic::subdivide`]); the two pieces of the split
//!    first lord are each subdivided in full ("compressed"), the reading neither source
//!    rules on.
//!
//! The alternative starting rule some texts give (count from the natal Moon's nakshatra
//! to the nakshatra of the return day) is not implemented; it is not the Charak or
//! Neelakanthi rule.

use kshana_time::Instant;
use kshana_vedic::{nakshatra, subdivide, DashaLord, DashaPeriod};

/// One Mudda period with its antardashas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MuddaPeriod {
    /// Lord of the period.
    pub lord: DashaLord,
    /// Exact start.
    pub start: Instant,
    /// Exact end (= the next period's start).
    pub end: Instant,
    /// Antardashas, in cycle order from the period's own lord.
    pub antardashas: Vec<DashaPeriod>,
}

/// Starting lord of the year's Mudda dasha: the natal Moon's Vimshottari lord advanced
/// one lord per completed year.
pub fn mudda_start_lord(natal_moon_sidereal: f64, completed_years: u32) -> DashaLord {
    let n = nakshatra(natal_moon_sidereal).index;
    DashaLord::ORDER[(n + completed_years as usize) % 9]
}

/// The year's Mudda periods from `start` (this return) to `end` (the next), with
/// antardashas. See the module documentation for the rule.
pub fn mudda(
    natal_moon_sidereal: f64,
    start: Instant,
    end: Instant,
    completed_years: u32,
) -> Vec<MuddaPeriod> {
    let total = end.ns_since(start);
    let first = mudda_start_lord(natal_moon_sidereal, completed_years);
    let fi = DashaLord::ORDER
        .iter()
        .position(|l| *l == first)
        .unwrap_or(0);
    let elapsed = nakshatra(natal_moon_sidereal).fraction;
    let mut shares: Vec<(DashaLord, f64)> = vec![(first, first.years() as f64 * (1.0 - elapsed))];
    for k in 1..9 {
        let l = DashaLord::ORDER[(fi + k) % 9];
        shares.push((l, l.years() as f64));
    }
    if elapsed > 0.0 {
        shares.push((first, first.years() as f64 * elapsed));
    }
    let n = shares.len();
    let mut out = Vec::with_capacity(n);
    let mut t = start;
    let mut cum = 0.0;
    for (i, (lord, share)) in shares.into_iter().enumerate() {
        cum += share;
        let e = if i + 1 == n {
            end
        } else {
            start.add_ns((total as f64 * cum / 120.0).round() as i128)
        };
        let p = DashaPeriod {
            lord,
            start: t,
            end: e,
        };
        out.push(MuddaPeriod {
            lord,
            start: t,
            end: e,
            antardashas: subdivide(p),
        });
        t = e;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;

    #[test]
    fn charak_worked_example_starts_with_rahu() {
        // Purva Phalguni is nakshatra 11 (index 10); 40 completed years → Rahu.
        let moon = 10.0 * 360.0 / 27.0 + 3.8;
        assert_eq!(mudda_start_lord(moon, 40), DashaLord::Rahu);
        // closed form from Charak: (years + n − 2) mod 9 into Venus, Sun, Moon, Mars, Rahu, …
        let charak = [
            DashaLord::Venus,
            DashaLord::Sun,
            DashaLord::Moon,
            DashaLord::Mars,
            DashaLord::Rahu,
            DashaLord::Jupiter,
            DashaLord::Saturn,
            DashaLord::Mercury,
            DashaLord::Ketu,
        ];
        for n in 1..=27usize {
            for years in 0..120u32 {
                let lon = (n - 1) as f64 * 360.0 / 27.0 + 1.0;
                let want = charak[(years as usize + n + 9 * 30 - 2) % 9];
                assert_eq!(mudda_start_lord(lon, years), want, "n {n} years {years}");
            }
        }
    }

    #[test]
    fn periods_are_contiguous_and_fill_the_year() {
        let start = Instant::j2000();
        let end = start.add_ns(365 * NS_PER_DAY + NS_PER_DAY / 4);
        let moon = 10.0 * 360.0 / 27.0 + 3.8; // 28.5% of the nakshatra elapsed
        let ps = mudda(moon, start, end, 40);
        assert_eq!(ps.len(), 10);
        assert_eq!(ps[0].lord, DashaLord::Rahu);
        assert_eq!(ps[9].lord, DashaLord::Rahu);
        assert_eq!(ps[0].start, start);
        assert_eq!(ps[9].end, end);
        for w in ps.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        // head + tail = Rahu's full share of 18/120
        let rahu = (ps[0].end.ns_since(ps[0].start) + ps[9].end.ns_since(ps[9].start)) as f64;
        let want = end.ns_since(start) as f64 * 18.0 / 120.0;
        assert!((rahu - want).abs() < 10.0);
        for p in &ps {
            assert_eq!(p.antardashas.len(), 9);
            assert_eq!(p.antardashas[0].lord, p.lord);
            assert_eq!(p.antardashas[0].start, p.start);
            assert_eq!(p.antardashas[8].end, p.end);
        }
    }
}
