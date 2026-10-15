//! Earth rotation time. UT1 is measured, not computed, so it carries an uncertainty.
//!
//! Two routes to UT1 exist:
//! * `UT1 = UTC + (UT1 − UTC)` with the daily value from IERS Bulletin A (±0.1 ms for
//!   the measured past, growing for predictions). This is the route Kshana uses in
//!   production; the data file is loaded at runtime and stamped in the provenance.
//! * `UT1 = TT − ΔT` with ΔT from historical tables and models. This is the route for
//!   dates before 1962 and the fallback when no bulletin is loaded.

use crate::instant::{Instant, TwoPartJd};

/// ΔT = TT − UT1 with its one-sigma uncertainty, both in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaT {
    /// TT − UT1 in seconds.
    pub seconds: f64,
    /// One-sigma uncertainty in seconds.
    pub sigma: f64,
}

/// UT1 − UTC in seconds with its uncertainty, as published in IERS Bulletin A.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ut1 {
    /// UT1 − UTC in seconds.
    pub ut1_minus_utc: f64,
    /// One-sigma uncertainty in seconds.
    pub sigma: f64,
}

/// SCAFFOLD TABLE. Yearly ΔT at January 1, seconds, 1973–2025, rounded to 0.01 s.
/// Values follow the published IERS/USNO series; they are placeholders until the
/// `finals2000A` loader lands and must not be treated as authoritative.
const DELTA_T_TABLE: &[(i32, f64)] = &[
    (1973, 43.47),
    (1974, 44.48),
    (1975, 45.48),
    (1976, 46.46),
    (1977, 47.52),
    (1978, 48.53),
    (1979, 49.59),
    (1980, 50.54),
    (1981, 51.38),
    (1982, 52.17),
    (1983, 52.96),
    (1984, 53.79),
    (1985, 54.34),
    (1986, 54.87),
    (1987, 55.32),
    (1988, 55.82),
    (1989, 56.30),
    (1990, 56.86),
    (1991, 57.57),
    (1992, 58.31),
    (1993, 59.12),
    (1994, 59.98),
    (1995, 60.78),
    (1996, 61.63),
    (1997, 62.29),
    (1998, 62.97),
    (1999, 63.47),
    (2000, 63.83),
    (2001, 64.09),
    (2002, 64.30),
    (2003, 64.47),
    (2004, 64.57),
    (2005, 64.69),
    (2006, 64.85),
    (2007, 65.15),
    (2008, 65.46),
    (2009, 65.78),
    (2010, 66.07),
    (2011, 66.32),
    (2012, 66.60),
    (2013, 66.91),
    (2014, 67.28),
    (2015, 67.64),
    (2016, 68.10),
    (2017, 68.59),
    (2018, 68.97),
    (2019, 69.22),
    (2020, 69.36),
    (2021, 69.36),
    (2022, 69.29),
    (2023, 69.20),
    (2024, 69.17),
    (2025, 69.18),
];

/// ΔT for a decimal year. Linear interpolation inside the table; outside it, the
/// Espenak & Meeus (2006) polynomials with a widening sigma. Scaffold quality.
pub fn delta_t_for_year(year: f64) -> DeltaT {
    let first = DELTA_T_TABLE[0];
    let last = DELTA_T_TABLE[DELTA_T_TABLE.len() - 1];
    if year >= first.0 as f64 && year <= last.0 as f64 {
        let idx = ((year - first.0 as f64).floor() as usize).min(DELTA_T_TABLE.len() - 2);
        let (y0, v0) = DELTA_T_TABLE[idx];
        let (_, v1) = DELTA_T_TABLE[idx + 1];
        let f = year - y0 as f64;
        return DeltaT {
            seconds: v0 + (v1 - v0) * f,
            sigma: 0.01,
        };
    }
    if year > last.0 as f64 {
        // Short-range extrapolation: hold the recent trend (ΔT has been nearly flat
        // since 2020). Sigma grows roughly 0.3 s per year of extrapolation.
        let dy = year - last.0 as f64;
        return DeltaT {
            seconds: last.1 + 0.05 * dy,
            sigma: 0.05 + 0.3 * dy,
        };
    }
    // Historical polynomials (Espenak & Meeus 2006), the −500 … 1973 span only.
    let y = year;
    let (seconds, sigma) = if y >= 1961.0 {
        let t = y - 1975.0;
        (45.45 + 1.067 * t - t * t / 260.0 - t * t * t / 718.0, 0.1)
    } else if y >= 1941.0 {
        let t = y - 1950.0;
        (29.07 + 0.407 * t - t * t / 233.0 + t * t * t / 2547.0, 0.3)
    } else if y >= 1920.0 {
        let t = y - 1920.0;
        (
            21.20 + 0.84493 * t - 0.076100 * t * t + 0.0020936 * t * t * t,
            0.5,
        )
    } else if y >= 1900.0 {
        let t = y - 1900.0;
        (
            -2.79 + 1.494119 * t - 0.0598939 * t * t + 0.0061966 * t * t * t
                - 0.000197 * t * t * t * t,
            1.0,
        )
    } else if y >= 1860.0 {
        let t = y - 1860.0;
        (
            7.62 + 0.5737 * t - 0.251754 * t * t + 0.01680668 * t * t * t
                - 0.0004473624 * t * t * t * t
                + t * t * t * t * t / 233174.0,
            2.0,
        )
    } else if y >= 1800.0 {
        let t = y - 1800.0;
        (
            13.72 - 0.332447 * t + 0.0068612 * t * t + 0.0041116 * t * t * t
                - 0.00037436 * t * t * t * t
                + 0.0000121272 * t * t * t * t * t
                - 0.0000001699 * t * t * t * t * t * t
                + 0.000000000875 * t * t * t * t * t * t * t,
            5.0,
        )
    } else if y >= 1700.0 {
        let t = y - 1700.0;
        (
            8.83 + 0.1603 * t - 0.0059285 * t * t + 0.00013336 * t * t * t
                - t * t * t * t / 1174000.0,
            10.0,
        )
    } else if y >= 1600.0 {
        let t = y - 1600.0;
        (
            120.0 - 0.9808 * t - 0.01532 * t * t + t * t * t / 7129.0,
            20.0,
        )
    } else if y >= 500.0 {
        let u = (y - 1000.0) / 100.0;
        (
            1574.2 - 556.01 * u + 71.23472 * u * u + 0.319781 * u * u * u
                - 0.8503463 * u * u * u * u
                - 0.005050998 * u * u * u * u * u
                + 0.0083572073 * u * u * u * u * u * u,
            60.0,
        )
    } else {
        let u = (y - 1820.0) / 100.0;
        (-20.0 + 32.0 * u * u, 120.0 + 2.0 * (1820.0 - y).abs())
    };
    DeltaT { seconds, sigma }
}

/// UT1 as a two-part Julian Date from an instant and a ΔT value.
pub fn ut1_jd_from_delta_t(i: Instant, dt: DeltaT) -> TwoPartJd {
    let tt = i.tt_jd();
    TwoPartJd {
        jd1: tt.jd1,
        jd2: tt.jd2 - dt.seconds / 86_400.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_monotonic_in_year() {
        for w in DELTA_T_TABLE.windows(2) {
            assert_eq!(w[1].0, w[0].0 + 1);
        }
    }

    #[test]
    fn interpolates_and_extrapolates_sanely() {
        let d = delta_t_for_year(2000.0);
        assert!((d.seconds - 63.83).abs() < 1e-9);
        let d = delta_t_for_year(2026.8);
        assert!(d.seconds > 69.0 && d.seconds < 69.5, "{d:?}");
        assert!(d.sigma > 0.5, "{d:?}");
        let d = delta_t_for_year(1900.0);
        assert!((d.seconds - (-2.79)).abs() < 1e-9);
    }
}
