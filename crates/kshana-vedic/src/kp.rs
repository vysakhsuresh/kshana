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
    /// Sub-sub-sub lord (the fourth nesting; KP software such as KP StarOne shows it).
    pub sub_sub_sub: DashaLord,
}

const ORDER: [DashaLord; 9] = DashaLord::ORDER;

/// Traditional ruler of a sign by 0-based index (Aries = 0).
pub fn sign_lord(sign_index: usize) -> DashaLord {
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

/// Arc unit for the nested lords: 1/36 arc-second. In it a nakshatra is 1 728 000
/// units, a sub `14 400·y₁`, a sub-sub `120·y₁·y₂` and a sub-sub-sub `y₁·y₂·y₃`
/// (y = Vimshottari years, Σy = 120), so every boundary down to the fourth level is an
/// exact integer and no floating-point drift can move a lord across a boundary.
pub const ARC_UNITS_PER_DEG: i64 = 3600 * 36;
const NAK_UNITS: i64 = 48_000 * 36;

/// One level of the nested KP division at a longitude: its lord and its exact
/// `[start, end)` bounds in sidereal degrees (unsplit by sign boundaries).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainLevel {
    /// Lord of this division.
    pub lord: DashaLord,
    /// Start, sidereal degrees.
    pub start: f64,
    /// End, sidereal degrees.
    pub end: f64,
}

/// The nested KP lords of a sidereal longitude: `[star, sub, sub-sub, sub-sub-sub]`,
/// each with its bounds. Each level divides its parent in Vimshottari proportion
/// (7, 20, 6, 10, 7, 18, 16, 19, 17 of 120) starting from the parent's own lord
/// (K. S. Krishnamurti, *KP Reader I: Casting the Horoscope*, the sub table; the
/// sub-sub as in KP Reader VI and KP StarOne). The nakshatra itself never splits at a
/// sign boundary, so the sub-sub cycle is not reset at 30° multiples.
pub fn lord_chain(sidereal_lon: f64) -> [ChainLevel; 4] {
    let total = 360 * ARC_UNITS_PER_DEG;
    let u = ((sidereal_lon.rem_euclid(360.0) * ARC_UNITS_PER_DEG as f64).floor() as i64)
        .clamp(0, total - 1);
    let nak = u / NAK_UNITS;
    let mut lord_idx = (nak % 9) as usize;
    let mut start = nak * NAK_UNITS;
    let mut span = NAK_UNITS;
    let deg = |x: i64| x as f64 / ARC_UNITS_PER_DEG as f64;
    let mut out = [ChainLevel {
        lord: ORDER[lord_idx],
        start: deg(start),
        end: deg(start + span),
    }; 4];
    for level in out.iter_mut().skip(1) {
        let mut acc = start;
        for k in 0..9 {
            let idx = (lord_idx + k) % 9;
            let w = span * ORDER[idx].years() as i64 / 120;
            if u < acc + w || k == 8 {
                lord_idx = idx;
                start = acc;
                span = w;
                break;
            }
            acc += w;
        }
        *level = ChainLevel {
            lord: ORDER[lord_idx],
            start: deg(start),
            end: deg(start + span),
        };
    }
    out
}

/// KP lords for a sidereal longitude in degrees.
pub fn kp_lords(sidereal_lon: f64) -> KpLords {
    let l = sidereal_lon.rem_euclid(360.0);
    let s = sub_of(l);
    let chain = lord_chain(l);
    KpLords {
        sign: sign_lord((l / 30.0).floor() as usize),
        star: s.star,
        sub: s.sub,
        sub_sub: chain[2].lord,
        sub_number: s.number,
        sub_sub_sub: chain[3].lord,
    }
}

/// Degrees, minutes and seconds (rounded to the nearest second) of a longitude within
/// its sign, for comparing against printed KP tables.
pub fn dms_in_sign(lon: f64) -> (u8, u8, u8) {
    let s = (lon.rem_euclid(360.0) * 3600.0).round() as i64 % (30 * 3600);
    ((s / 3600) as u8, ((s / 60) % 60) as u8, (s % 60) as u8)
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

    /// Boundaries of the 249 table as printed in KP Reader I (Krishnamurti), sign
    /// degree-minute-second, with star and sub lords.
    #[test]
    fn sub_table_matches_the_printed_kp_table() {
        use DashaLord::*;
        let t = sub_table();
        // (number, sign 0-based, start dms, end dms, star, sub)
        let rows: [(u16, u8, (u8, u8, u8), (u8, u8, u8), DashaLord, DashaLord); 12] = [
            (1, 0, (0, 0, 0), (0, 46, 40), Ketu, Ketu),
            (2, 0, (0, 46, 40), (3, 0, 0), Ketu, Venus),
            (9, 0, (11, 26, 40), (13, 20, 0), Ketu, Mercury),
            (10, 0, (13, 20, 0), (15, 33, 20), Venus, Venus),
            (18, 0, (25, 53, 20), (26, 40, 0), Venus, Ketu),
            (22, 0, (29, 13, 20), (0, 0, 0), Sun, Rahu),
            (23, 1, (0, 0, 0), (1, 13, 20), Sun, Rahu),
            // Punarvasu's Moon sub straddles Gemini/Cancer
            (62, 2, (29, 26, 40), (0, 0, 0), Jupiter, Moon),
            (63, 3, (0, 0, 0), (0, 33, 20), Jupiter, Moon),
            // Uttara Phalguni's Rahu sub straddles Leo/Virgo
            (105, 4, (29, 13, 20), (0, 0, 0), Sun, Rahu),
            (106, 5, (0, 0, 0), (1, 13, 20), Sun, Rahu),
            (249, 11, (27, 53, 20), (0, 0, 0), Mercury, Saturn),
        ];
        for (n, sign, a, b, star, sub) in rows {
            let s = t[n as usize - 1];
            assert_eq!(s.number, n);
            assert_eq!((s.start / 30.0 + 1e-9).floor() as u8, sign, "sub {n} sign");
            assert_eq!(dms_in_sign(s.start), a, "sub {n} start");
            assert_eq!(dms_in_sign(s.end), b, "sub {n} end");
            assert_eq!((s.star, s.sub), (star, sub), "sub {n} lords");
        }
        // exactly six subs straddle a sign: at 30, 90, 150, 210, 270, 330
        let split: Vec<i64> = t
            .windows(2)
            .filter(|w| w[0].sub == w[1].sub && w[0].star == w[1].star)
            .map(|w| w[1].start.round() as i64)
            .collect();
        assert_eq!(split, vec![30, 90, 150, 210, 270, 330]);
    }

    #[test]
    fn nested_lords_are_exact_and_tile() {
        use DashaLord::*;
        // Ketu-Ketu-Ketu sub-sub: 7·7/120 of the Ketu sub = 0°2′43⅓″; Venus next to 0°10′30″
        let c = lord_chain(0.0);
        assert_eq!([c[0].lord, c[1].lord, c[2].lord, c[3].lord], [Ketu, Ketu, Ketu, Ketu]);
        assert!((c[2].end * 3600.0 - 163.0 - 1.0 / 3.0).abs() < 1e-9);
        let v = lord_chain(c[2].end + 1e-9);
        assert_eq!(v[2].lord, Venus);
        assert!((v[2].end * 3600.0 - 630.0).abs() < 1e-9);
        // sub-sub-sub of K-K-K = 7·7·7/36 arc-seconds
        assert!((c[3].end * 3600.0 - 343.0 / 36.0).abs() < 1e-9);
        // the sub-sub level tiles every sub, starting with the sub lord
        // (the second half of a sign-split sub starts mid-sub, so it is skipped)
        let t = sub_table();
        for (i, s) in t.iter().enumerate() {
            let first = lord_chain(s.start + 1e-9);
            assert_eq!(first[1].lord, s.sub);
            let split_tail = i > 0 && t[i - 1].sub == s.sub && t[i - 1].star == s.star;
            if !split_tail {
                assert_eq!(first[2].lord, s.sub, "sub-sub starts from the sub lord");
                assert!((first[1].start - s.start).abs() < 1e-9);
            }
        }
        // the sign split does not restart the sub-sub cycle: Gemini 29°59′59″ and
        // Cancer 0°00′01″ both sit in the Moon sub of Punarvasu
        let a = lord_chain(90.0 - 1.0 / 3600.0);
        let b = lord_chain(90.0 + 1.0 / 3600.0);
        assert_eq!((a[1].lord, b[1].lord), (Moon, Moon));
        assert_eq!(a[1].start, b[1].start);
        // kp_lords agrees with the chain everywhere away from boundaries
        for i in 0..3600 {
            let l = i as f64 * 0.1 + 0.000_37;
            let k = kp_lords(l);
            let ch = lord_chain(l);
            assert_eq!((k.star, k.sub, k.sub_sub, k.sub_sub_sub), (ch[0].lord, ch[1].lord, ch[2].lord, ch[3].lord), "{l}");
        }
    }

    /// KP horary (Reader VI): number n puts the ascendant at the first degree of the
    /// n-th sub. 1 = Aries 0°, 23 = Taurus 0°, 249 = Pisces 27°53′20″.
    #[test]
    fn horary_numbers_land_on_printed_degrees() {
        assert_eq!(horary_longitude(1), Some(0.0));
        assert!((horary_longitude(23).unwrap() - 30.0).abs() < 1e-9);
        assert_eq!(dms_in_sign(horary_longitude(249).unwrap()), (27, 53, 20));
        assert_eq!(dms_in_sign(horary_longitude(2).unwrap()), (0, 46, 40));
    }

    #[test]
    fn mula_is_ketu_star() {
        let k = kp_lords(243.2);
        assert_eq!(k.star, DashaLord::Ketu);
        assert_eq!(k.sign, DashaLord::Jupiter);
    }
}
