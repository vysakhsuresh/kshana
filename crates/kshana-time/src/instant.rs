//! The exact instant type and its float projections.

use crate::tdb::tdb_minus_tt_seconds;

/// Nanoseconds in one SI second.
pub const NS_PER_SEC: i128 = 1_000_000_000;
/// Nanoseconds in one day of 86 400 SI seconds.
pub const NS_PER_DAY: i128 = 86_400 * NS_PER_SEC;
/// TT − TAI = 32.184 s exactly, by definition.
pub const TT_MINUS_TAI_NS: i128 = 32_184_000_000;

/// TAI nanoseconds elapsed from the TAI epoch (1958-01-01T00:00:00 TAI) to J2000.0 TT
/// (2000-01-01T12:00:00 TT = 2000-01-01T11:59:27.816 TAI).
///
/// 15 340 days × 86 400 s + 11 h 59 m 27.816 s.
pub(crate) const J2000_TAI_NS: i128 = 15_340 * NS_PER_DAY + 43_167_816_000_000;

/// Julian Date of J2000.0 (TT).
pub const JD_J2000: f64 = 2_451_545.0;

/// One instant of time, exact to the nanosecond, on the TAI scale.
///
/// Internally this is an `i128` count of nanoseconds since 1958-01-01T00:00:00 TAI.
/// The range covers the whole of DE441 (−13 200 to +17 191) with room to spare,
/// and integer arithmetic means no rounding ever occurs inside the time layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    tai_ns: i128,
}

impl Instant {
    /// Build from raw TAI nanoseconds since the TAI epoch.
    pub const fn from_tai_ns(tai_ns: i128) -> Self {
        Self { tai_ns }
    }

    /// Raw TAI nanoseconds since the TAI epoch.
    pub const fn tai_ns(self) -> i128 {
        self.tai_ns
    }

    /// Nanoseconds on the TT scale since the TAI epoch label (TT = TAI + 32.184 s).
    pub const fn tt_ns(self) -> i128 {
        self.tai_ns + TT_MINUS_TAI_NS
    }

    /// The J2000.0 epoch (2000-01-01T12:00:00 TT).
    pub const fn j2000() -> Self {
        Self {
            tai_ns: J2000_TAI_NS,
        }
    }

    /// Shift by a signed number of nanoseconds.
    pub const fn add_ns(self, ns: i128) -> Self {
        Self {
            tai_ns: self.tai_ns + ns,
        }
    }

    /// Shift by a signed number of SI seconds given as a float (sub-nanosecond part is
    /// rounded). Use [`Instant::add_ns`] when the offset is already exact.
    pub fn add_seconds_f64(self, seconds: f64) -> Self {
        let ns = (seconds * NS_PER_SEC as f64).round() as i128;
        self.add_ns(ns)
    }

    /// Signed difference `self − other` in nanoseconds.
    pub const fn ns_since(self, other: Instant) -> i128 {
        self.tai_ns - other.tai_ns
    }

    /// TT elapsed since J2000.0 in SI seconds, as a float. Resolution near the present
    /// is about 0.1 µs; prefer the two-part JD or integer forms where it matters.
    pub fn tt_seconds_since_j2000(self) -> f64 {
        (self.tai_ns - J2000_TAI_NS) as f64 / NS_PER_SEC as f64
    }

    /// TT as a two-part Julian Date. `jd1` is a whole-number-plus-half day and `jd2`
    /// lies in `[0, 1)`, so the pair carries picosecond resolution.
    pub fn tt_jd(self) -> TwoPartJd {
        // Whole days since J2000.0 TT, then the fraction of the current day.
        let since = self.tai_ns - J2000_TAI_NS;
        let days = since.div_euclid(NS_PER_DAY);
        let rem = since.rem_euclid(NS_PER_DAY);
        TwoPartJd {
            jd1: JD_J2000 + days as f64,
            jd2: rem as f64 / NS_PER_DAY as f64,
        }
    }

    /// TDB as a two-part Julian Date, using the truncated TDB − TT series.
    pub fn tdb_jd(self) -> TwoPartJd {
        let tt = self.tt_jd();
        let t_centuries = tt.centuries_since_j2000();
        let dt = tdb_minus_tt_seconds(t_centuries);
        TwoPartJd {
            jd1: tt.jd1,
            jd2: tt.jd2 + dt / 86_400.0,
        }
    }

    /// Julian centuries of TT since J2000.0.
    pub fn tt_centuries_since_j2000(self) -> f64 {
        self.tt_jd().centuries_since_j2000()
    }
}

/// A Julian Date held as the sum of two floats, `jd1 + jd2`, the SOFA convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoPartJd {
    /// The large part (normally a half-integer day).
    pub jd1: f64,
    /// The small part (normally in `[0, 1)`).
    pub jd2: f64,
}

impl TwoPartJd {
    /// Julian centuries since J2000.0, computed to keep the small part small.
    pub fn centuries_since_j2000(self) -> f64 {
        ((self.jd1 - JD_J2000) + self.jd2) / 36_525.0
    }

    /// Collapse to one float. Loses resolution; for display only.
    pub fn as_f64(self) -> f64 {
        self.jd1 + self.jd2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_is_exact() {
        let jd = Instant::j2000().tt_jd();
        assert_eq!(jd.jd1, JD_J2000);
        assert_eq!(jd.jd2, 0.0);
    }

    #[test]
    fn one_nanosecond_is_representable() {
        let a = Instant::j2000();
        let b = a.add_ns(1);
        assert_ne!(a, b);
        assert_eq!(b.ns_since(a), 1);
        // and it survives into the two-part JD
        let jd = b.tt_jd();
        assert!(jd.jd2 > 0.0 && jd.jd2 < 1e-13);
    }

    #[test]
    fn day_boundary_splits_cleanly() {
        let i = Instant::j2000().add_ns(NS_PER_DAY * 3 + 500);
        let jd = i.tt_jd();
        assert_eq!(jd.jd1, JD_J2000 + 3.0);
        assert!((jd.jd2 - 500.0 / NS_PER_DAY as f64).abs() < 1e-18);
    }

    #[test]
    fn negative_offsets_use_euclid_split() {
        let i = Instant::j2000().add_ns(-1);
        let jd = i.tt_jd();
        assert_eq!(jd.jd1, JD_J2000 - 1.0);
        assert!(jd.jd2 > 0.999_999_999_9);
    }
}
