//! IERS Earth-orientation parameters from `finals2000A.all` (Bulletin A format).
//!
//! Each line is one UTC day. Fixed columns (1-based): 8–15 MJD, 17 IERS/prediction
//! flag for polar motion, 19–27 x, 38–46 y (arcsec), 58 flag for UT1−UTC, 59–68
//! UT1−UTC (s), 69–78 its error. Lines with a blank UT1 field are skipped.

use crate::delta_t::Ut1;

/// One day of Earth-orientation data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EopRow {
    /// Modified Julian Date (UTC) of the day.
    pub mjd: f64,
    /// UT1 − UTC in seconds.
    pub ut1_minus_utc: f64,
    /// One-sigma uncertainty in seconds, where given.
    pub sigma: f64,
    /// True for an IERS-measured value, false for a Bulletin A prediction.
    pub measured: bool,
    /// Polar motion x, arcsec.
    pub pm_x: f64,
    /// Polar motion y, arcsec.
    pub pm_y: f64,
}

/// A loaded table.
#[derive(Clone, Debug, Default)]
pub struct EopTable {
    rows: Vec<EopRow>,
}

impl EopTable {
    /// Parse the text of `finals2000A.all`.
    pub fn parse(text: &str) -> Self {
        let mut rows = Vec::new();
        for line in text.lines() {
            if line.len() < 68 {
                continue;
            }
            let field = |a: usize, b: usize| line.get(a..b).map(str::trim).unwrap_or("");
            let mjd: f64 = match field(7, 15).parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let ut1: f64 = match field(58, 68).parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let sigma = field(68, 78).parse().unwrap_or(0.0);
            let measured = field(57, 58) == "I";
            let pm_x = field(18, 27).parse().unwrap_or(0.0);
            let pm_y = field(37, 46).parse().unwrap_or(0.0);
            rows.push(EopRow {
                mjd,
                ut1_minus_utc: ut1,
                sigma,
                measured,
                pm_x,
                pm_y,
            });
        }
        Self { rows }
    }

    /// Number of days loaded.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when nothing was loaded.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Inclusive MJD range covered.
    pub fn range(&self) -> Option<(f64, f64)> {
        Some((self.rows.first()?.mjd, self.rows.last()?.mjd))
    }

    /// UT1 − UTC at a UTC MJD by linear interpolation between daily values.
    /// Returns `None` outside the table.
    pub fn ut1_minus_utc(&self, mjd_utc: f64) -> Option<Ut1> {
        let first = self.rows.first()?.mjd;
        if mjd_utc < first {
            return None;
        }
        let i = ((mjd_utc - first).floor() as usize).min(self.rows.len().saturating_sub(1));
        let a = self.rows.get(i)?;
        let b = self.rows.get(i + 1).unwrap_or(a);
        if mjd_utc > b.mjd + 1e-9 {
            return None;
        }
        let f = if b.mjd > a.mjd {
            (mjd_utc - a.mjd) / (b.mjd - a.mjd)
        } else {
            0.0
        };
        // Leap seconds make UT1−UTC jump by a whole second between rows; interpolate
        // the continuous part only.
        let mut d = b.ut1_minus_utc - a.ut1_minus_utc;
        if d.abs() > 0.5 {
            d -= d.round();
        }
        let sigma = a
            .sigma
            .max(b.sigma)
            .max(if a.measured && b.measured { 0.0 } else { 0.001 });
        Some(Ut1 {
            ut1_minus_utc: a.ut1_minus_utc + d * f,
            sigma,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        " 0 1 1 51544.00 I  0.043301 0.000092  0.377867 0.000099  I 0.3554779 0.0000099  0.9333 0.0076  I    -0.135    0.315    -0.204    0.298   .043190   .377700   .3554990    -0.252    -0.119  \n",
        " 0 1 2 51545.00 I  0.043541 0.000058  0.377638 0.000073  I 0.3546013 0.0000119  0.8165 0.0074  I     0.121    0.294    -0.258    0.340   .043480   .377510   .3546330    -0.111    -0.079  \n\";\n",
    );

    #[test]
    fn parses_bulletin_a_lines() {
        let t = EopTable::parse(SAMPLE);
        assert_eq!(t.len(), 2);
        let u = t.ut1_minus_utc(51544.5).unwrap();
        assert!((u.ut1_minus_utc - 0.3550396).abs() < 1e-7, "{u:?}");
        assert!(u.sigma < 2e-5, "{u:?}");
        assert!(t.ut1_minus_utc(51543.0).is_none());
        assert!(t.ut1_minus_utc(51546.0).is_none());
    }

    /// Real file: `cargo test -p kshana-time -- --ignored` with data/finals2000A.all present.
    #[test]
    #[ignore]
    fn real_file_j2000_value() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/finals2000A.all");
        let text = std::fs::read_to_string(p).expect("data/finals2000A.all");
        let t = EopTable::parse(&text);
        assert!(t.len() > 10_000, "{}", t.len());
        let u = t.ut1_minus_utc(51544.0).unwrap();
        assert!((u.ut1_minus_utc - 0.3555).abs() < 0.001, "{u:?}");
    }
}
