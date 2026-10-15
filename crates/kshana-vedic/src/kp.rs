//! Krishnamurti Padhdhati: the 249 sub-divisions of the zodiac.
//!
//! Each nakshatra (13°20′) is ruled by a Vimshottari lord and divided into nine subs in
//! Vimshottari proportion (7, 20, 6, 10, 7, 18, 16, 19, 17 of 120), starting from the
//! nakshatra's own lord. The same rule nests again for the sub-sub. Longitudes are
//! sidereal degrees. All arithmetic is exact rational division of the span, so a sub
//! boundary is a precise longitude and can be solved for as an exact instant.

use crate::dasha::DashaLord;

/// Lords of a sidereal longitude at the sign, star, sub and sub-sub levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KpLords {
    /// Sign lord (traditional rulership).
    pub sign: DashaLord,
    /// Nakshatra (star) lord.
    pub star: DashaLord,
    /// Sub lord.
    pub sub: DashaLord,
    /// Sub-sub lord.
    pub sub_sub: DashaLord,
    /// 1-based index of the sub within the 249 (Ashwini-Ketu-Ketu = 1).
    pub sub_number: u16,
}

const ORDER: [DashaLord; 9] = DashaLord::ORDER;

fn sign_lord(sign_index: usize) -> DashaLord {
    match sign_index % 12 {
        0 | 7 => DashaLord::Mars,
        1 | 6 => DashaLord::Venus,
        2 | 5 => DashaLord::Mercury,
        3 => DashaLord::Moon,
        4 => DashaLord::Sun,
        8 | 11 => DashaLord::Jupiter,
        _ => DashaLord::Saturn,
    }
}

/// One of the 249 KP subs: sidereal bounds and lords.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sub {
    /// 1-based KP number.
    pub number: u16,
    /// Start longitude, sidereal degrees.
    pub start: f64,
    /// End longitude, sidereal degrees.
    pub end: f64,
    /// Star lord of the containing nakshatra.
    pub star: DashaLord,
    /// Sub lord.
    pub sub: DashaLord,
}

/// The 249-entry table: 27 × 9 Vimshottari-proportioned subs, with the six that straddle
/// a sign boundary split in two, in zodiacal order. Built on demand; 243 iterations.
pub fn sub_table() -> Vec<Sub> {
    let span = 360.0 / 27.0;
    let mut out = Vec::with_capacity(249);
    for nak in 0..27 {
        let star_idx = nak % 9;
        let mut acc = 0.0;
        for k in 0..9 {
            let lord_idx = (star_idx + k) % 9;
            let w = ORDER[lord_idx].years() as f64 / 120.0;
            let start = (nak as f64 + acc) * span;
            let end = (nak as f64 + acc + w) * span;
            // a sign boundary strictly inside splits the sub
            let next_sign = ((start / 30.0).floor() + 1.0) * 30.0;
            if next_sign < end - 1e-9 && next_sign > start + 1e-9 {
                out.push(Sub {
                    number: 0,
                    start,
                    end: next_sign,
                    star: ORDER[star_idx],
                    sub: ORDER[lord_idx],
                });
                out.push(Sub {
                    number: 0,
                    start: next_sign,
                    end,
                    star: ORDER[star_idx],
                    sub: ORDER[lord_idx],
                });
            } else {
                out.push(Sub {
                    number: 0,
                    start,
                    end,
                    star: ORDER[star_idx],
                    sub: ORDER[lord_idx],
                });
            }
            acc += w;
        }
    }
    for (i, s) in out.iter_mut().enumerate() {
        s.number = (i + 1) as u16;
    }
    out
}

/// The sub containing a sidereal longitude.
pub fn sub_of(sidereal_lon: f64) -> Sub {
    let l = sidereal_lon.rem_euclid(360.0);
    let table = sub_table();
    *table
        .iter()
        .find(|s| l >= s.start && l < s.end)
        .unwrap_or(&table[248])
}

/// Sub-sub lord: the Vimshottari-proportioned division of the sub, starting from the
/// sub lord itself. Uses the unsplit sub so a sign boundary does not reset the cycle.
fn sub_sub_lord(sidereal_lon: f64) -> DashaLord {
    let l = sidereal_lon.rem_euclid(360.0);
    let span = 360.0 / 27.0;
    let nak = ((l / span).floor() as usize).min(26);
    let star_idx = nak % 9;
    let in_nak = l / span - nak as f64;
    let mut acc = 0.0;
    for k in 0..9 {
        let lord_idx = (star_idx + k) % 9;
        let w = ORDER[lord_idx].years() as f64 / 120.0;
        if in_nak < acc + w || k == 8 {
            let in_sub = ((in_nak - acc) / w).clamp(0.0, 0.999_999_999_999);
            let mut acc2 = 0.0;
            for j in 0..9 {
                let idx2 = (lord_idx + j) % 9;
                let w2 = ORDER[idx2].years() as f64 / 120.0;
                if in_sub < acc2 + w2 || j == 8 {
                    return ORDER[idx2];
                }
                acc2 += w2;
            }
        }
        acc += w;
    }
    unreachable!()
}

/// KP lords for a sidereal longitude in degrees.
pub fn kp_lords(sidereal_lon: f64) -> KpLords {
    let l = sidereal_lon.rem_euclid(360.0);
    let s = sub_of(l);
    KpLords {
        sign: sign_lord((l / 30.0).floor() as usize),
        star: s.star,
        sub: s.sub,
        sub_sub: sub_sub_lord(l),
        sub_number: s.number,
    }
}

/// Sidereal longitude bounds `[start, end)` of the (sign-split) sub containing the longitude.
pub fn sub_bounds(sidereal_lon: f64) -> (f64, f64) {
    let s = sub_of(sidereal_lon);
    (s.start, s.end)
}

/// Horary number 1–249 → the sidereal longitude of the start of that sub, the KP
/// convention for a Prashna lagna.
pub fn horary_longitude(number: u16) -> Option<f64> {
    if !(1..=249).contains(&number) {
        return None;
    }
    Some(sub_table()[number as usize - 1].start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ashwini_starts_with_ketu_ketu() {
        let k = kp_lords(0.0);
        assert_eq!(
            (k.sign, k.star, k.sub, k.sub_sub, k.sub_number),
            (
                DashaLord::Mars,
                DashaLord::Ketu,
                DashaLord::Ketu,
                DashaLord::Ketu,
                1
            )
        );
    }

    #[test]
    fn sub_widths_follow_vimshottari_proportion() {
        // Ashwini: Ketu sub = 7/120 of 13°20′ = 0°46′40″
        let (a, b) = sub_bounds(0.1);
        assert!((a - 0.0).abs() < 1e-12);
        assert!((b - 13.333_333_333_333_334 * 7.0 / 120.0).abs() < 1e-9);
        // the next sub is Venus
        assert_eq!(kp_lords(b + 1e-6).sub, DashaLord::Venus);
        assert_eq!(kp_lords(b + 1e-6).sub_number, 2);
    }

    #[test]
    fn two_hundred_forty_nine_subs_tile_the_zodiac() {
        let t = sub_table();
        assert_eq!(t.len(), 249);
        assert_eq!(t[0].start, 0.0);
        assert!((t[248].end - 360.0).abs() < 1e-9);
        for w in t.windows(2) {
            assert!((w[0].end - w[1].start).abs() < 1e-9);
            assert!(w[0].end > w[0].start);
        }
        // the first split sub: Rahu sub of Krittika straddles 30°
        let at_30 = t
            .iter()
            .position(|s| (s.start - 30.0).abs() < 1e-9)
            .unwrap();
        assert_eq!(t[at_30].sub, DashaLord::Rahu);
        assert_eq!(t[at_30 - 1].sub, DashaLord::Rahu);
        assert_eq!(kp_lords(359.999).sub_number, 249);
    }

    #[test]
    fn horary_numbers_round_trip() {
        for n in 1..=249u16 {
            let l = horary_longitude(n).unwrap();
            assert_eq!(kp_lords(l + 1e-7).sub_number, n, "number {n} lon {l}");
        }
        assert!(horary_longitude(0).is_none());
        assert!(horary_longitude(250).is_none());
    }

    #[test]
    fn mula_is_ketu_star() {
        let k = kp_lords(243.2);
        assert_eq!(k.star, DashaLord::Ketu);
        assert_eq!(k.sign, DashaLord::Jupiter);
    }
}
