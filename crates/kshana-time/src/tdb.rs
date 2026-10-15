//! TDB − TT: the periodic relativistic correction between Terrestrial Time and
//! Barycentric Dynamical Time, the argument that JPL ephemerides are tabulated against.

/// TDB − TT in seconds for `t` Julian centuries of TT since J2000.0.
///
/// Truncated Fairhead & Bretagnon (1990) series, the form used in the Astronomical
/// Almanac. Accurate to about 30 µs. Milestone 2 replaces this with the full series
/// (SOFA `dtdb`) or, better, the ephemeris's own TT−TDB segment, which is consistent
/// with the planetary positions to the nanosecond.
pub fn tdb_minus_tt_seconds(t: f64) -> f64 {
    0.001_657 * (628.307_6 * t + 6.240_1).sin()
        + 0.000_022 * (575.338_5 * t + 4.297_0).sin()
        + 0.000_014 * (1_256.615_2 * t + 6.196_9).sin()
        + 0.000_005 * (606.977_7 * t + 4.021_2).sin()
        + 0.000_005 * (52.969_1 * t + 0.444_4).sin()
        + 0.000_002 * (21.329_9 * t + 5.543_1).sin()
        + 0.000_010 * t * (628.307_6 * t + 4.249_0).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_is_bounded() {
        // The correction never exceeds about ±1.7 ms.
        let mut max = 0.0f64;
        for i in 0..10_000 {
            let t = (i as f64 - 5_000.0) / 1_000.0;
            max = max.max(tdb_minus_tt_seconds(t).abs());
        }
        assert!(max < 0.001_8, "max {max}");
        assert!(max > 0.001_5, "max {max}");
    }
}
