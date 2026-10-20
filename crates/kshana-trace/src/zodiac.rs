//! Divisions of the sidereal zodiac that a body can cross: sign, nakshatra, pada, KP sub,
//! KP sub-sub, and the lunar phases (tithi, karana, yoga) as divisions of an angle.

use kshana_vedic::{sub_table, DashaLord};

/// A division of 360° into cells with exact boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Division {
    /// 12 signs of 30°.
    Sign,
    /// 27 nakshatras of 13°20′.
    Nakshatra,
    /// 108 padas of 3°20′.
    Pada,
    /// 249 KP subs (Vimshottari-proportioned within each nakshatra, sign-split).
    Sub,
    /// 2241 KP sub-subs (each of the 249 subs split again in Vimshottari proportion).
    SubSub,
    /// 30 tithis of 12° of Moon − Sun elongation.
    Tithi,
    /// 60 karanas of 6° of elongation.
    Karana,
    /// 27 yogas of 13°20′ of Moon + Sun.
    Yoga,
}

impl Division {
    /// Name in words.
    pub fn name(self) -> &'static str {
        match self {
            Division::Sign => "sign",
            Division::Nakshatra => "nakshatra",
            Division::Pada => "pada",
            Division::Sub => "sub",
            Division::SubSub => "sub-sub",
            Division::Tithi => "tithi",
            Division::Karana => "karana",
            Division::Yoga => "yoga",
        }
    }

    /// The cell boundaries `(start, end)` in degrees, in order, covering `[0, 360)`.
    pub fn bounds(self) -> Vec<(f64, f64)> {
        match self {
            Division::Sign => uniform(12),
            Division::Nakshatra | Division::Yoga => uniform(27),
            Division::Pada => uniform(108),
            Division::Tithi => uniform(30),
            Division::Karana => uniform(60),
            Division::Sub => sub_bounds().to_vec(),
            Division::SubSub => subsub_bounds().to_vec(),
        }
    }

    /// Index of the cell containing `deg` (any real degrees).
    pub fn index(self, deg: f64) -> usize {
        let l = deg.rem_euclid(360.0);
        match self {
            Division::Sign => ((l / 30.0).floor() as usize).min(11),
            Division::Nakshatra | Division::Yoga => ((l / (360.0 / 27.0)).floor() as usize).min(26),
            Division::Pada => ((l / (360.0 / 108.0)).floor() as usize).min(107),
            Division::Tithi => ((l / 12.0).floor() as usize).min(29),
            Division::Karana => ((l / 6.0).floor() as usize).min(59),
            Division::Sub | Division::SubSub => {
                let b: &[(f64, f64)] = if self == Division::Sub {
                    sub_bounds()
                } else {
                    subsub_bounds()
                };
                // binary search on starts
                let mut lo = 0usize;
                let mut hi = b.len();
                while hi - lo > 1 {
                    let mid = (lo + hi) / 2;
                    if b[mid].0 <= l {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                lo
            }
        }
    }

    /// Number of cells.
    pub fn len(self) -> usize {
        match self {
            Division::Sign => 12,
            Division::Nakshatra | Division::Yoga => 27,
            Division::Pada => 108,
            Division::Sub => 249,
            Division::SubSub => 2241,
            Division::Tithi => 30,
            Division::Karana => 60,
        }
    }

    /// Always false; present so `len` reads naturally beside it.
    pub fn is_empty(self) -> bool {
        false
    }
}

/// The 249 sub bounds, built once per process.
fn sub_bounds() -> &'static [(f64, f64)] {
    static T: std::sync::OnceLock<Vec<(f64, f64)>> = std::sync::OnceLock::new();
    T.get_or_init(|| sub_table().iter().map(|s| (s.start, s.end)).collect())
}

/// The 2241 sub-sub bounds, built once per process.
fn subsub_bounds() -> &'static [(f64, f64)] {
    static T: std::sync::OnceLock<Vec<(f64, f64)>> = std::sync::OnceLock::new();
    T.get_or_init(|| subsub_table().iter().map(|s| (s.start, s.end)).collect())
}

fn uniform(n: usize) -> Vec<(f64, f64)> {
    let w = 360.0 / n as f64;
    (0..n).map(|i| (i as f64 * w, (i + 1) as f64 * w)).collect()
}

/// One KP sub-sub: a sub split again in Vimshottari proportion from the sub lord.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubSub {
    /// Start, sidereal degrees.
    pub start: f64,
    /// End, sidereal degrees.
    pub end: f64,
    /// Star lord.
    pub star: DashaLord,
    /// Sub lord.
    pub sub: DashaLord,
    /// Sub-sub lord.
    pub subsub: DashaLord,
    /// The 1-based KP sub number this belongs to.
    pub sub_number: u16,
}

/// The 2241-entry sub-sub table (249 × 9), built on demand. A sub split at a sign
/// boundary keeps the proportion of its own (partial) width, the convention used by KP
/// software that lists sub-subs within each listed sub.
pub fn subsub_table() -> Vec<SubSub> {
    let subs = sub_table();
    let mut out = Vec::with_capacity(2241);
    for s in subs {
        let width = s.end - s.start;
        let start_idx = DashaLord::ORDER
            .iter()
            .position(|l| *l == s.sub)
            .unwrap_or(0);
        let mut acc = 0.0;
        for k in 0..9 {
            let lord = DashaLord::ORDER[(start_idx + k) % 9];
            let w = lord.years() as f64 / 120.0;
            let a = s.start + width * acc;
            let b = if k == 8 {
                s.end
            } else {
                s.start + width * (acc + w)
            };
            out.push(SubSub {
                start: a,
                end: b,
                star: s.star,
                sub: s.sub,
                subsub: lord,
                sub_number: s.number,
            });
            acc += w;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_tile_the_circle() {
        for d in [
            Division::Sign,
            Division::Nakshatra,
            Division::Pada,
            Division::Sub,
            Division::SubSub,
            Division::Tithi,
            Division::Karana,
            Division::Yoga,
        ] {
            let b = d.bounds();
            assert_eq!(b.len(), d.len());
            assert!((b[0].0).abs() < 1e-9);
            assert!((b[b.len() - 1].1 - 360.0).abs() < 1e-6, "{:?}", d);
            for w in b.windows(2) {
                assert!((w[0].1 - w[1].0).abs() < 1e-9, "{:?} gap", d);
                assert!(w[0].1 > w[0].0);
            }
            // index agrees with the bounds at cell midpoints
            for (i, (a, e)) in b.iter().enumerate() {
                assert_eq!(d.index(0.5 * (a + e)), i, "{:?} cell {}", d, i);
            }
        }
    }

    #[test]
    fn subsub_lords_follow_the_sub() {
        let t = subsub_table();
        assert_eq!(t[0].star, DashaLord::Ketu);
        assert_eq!(t[0].sub, DashaLord::Ketu);
        assert_eq!(t[0].subsub, DashaLord::Ketu);
        assert_eq!(t[1].subsub, DashaLord::Venus);
        assert_eq!(t[8].subsub, DashaLord::Mercury);
        assert_eq!(t[9].sub, DashaLord::Venus);
        assert_eq!(t[9].subsub, DashaLord::Venus);
    }
}
