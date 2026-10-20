//! Divisional charts (vargas): the one canonical implementation every crate uses.
//!
//! Each function maps a sidereal longitude to the longitude in the divisional chart:
//! the varga sign index times 30 plus the position inside that sign, so the same
//! drawing code renders every varga. Sign indices are 0 = Aries.
//!
//! # Sources and conventions
//!
//! * BPHS ch. 6 (*Shodashavarga*), verses as numbered in R. Santhanam's and G. C.
//!   Sharma's editions, for D1–D4, D7, D9, D10, D12, D16, D20, D24, D27, D30, D40,
//!   D45, D60.
//! * Where BPHS is silent (D5, D6, D8, D11, D81, D108, D144, D150) the convention is
//!   Jagannatha Hora's default ("traditional Parasara") method, as encoded in PyJHora
//!   `src/jhora/horoscope/chart/charts.py` (`panchamsa_chart`, `shashthamsa_chart`,
//!   `ashtamsa_chart`, `rudramsa_chart`, `nava_navamsa_chart`, `ashtotharamsa_chart`,
//!   `dwadas_dwadasamsa_chart`, `nadiamsa_chart`) with the defaults in
//!   `src/jhora/const.py` (`d5_chart_method_default` … `d150_chart_method_default`).
//!   Each rule below names the variant chosen.
//! * The part of a sign is `floor(in_sign × n / 30)`, computed by multiplying first so
//!   that a longitude exactly on a boundary lands in the later part, as in PyJHora's
//!   `int(long // (30/n))` (which agrees except for float noise at boundaries).
//!
//! Before this module was made canonical, D40 and D45 fell through to the generic
//! "count from the sign" rule here, while `kshana-marga` carried the correct BPHS rules
//! for its Vimshopaka; both now come from this one function.

/// The vargas with a specific rule here (every other `n` counts from the sign itself).
pub const SUPPORTED_VARGAS: [u32; 24] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 24, 27, 30, 40, 45, 60, 81, 108, 144, 150,
];

/// Sign (0–11), part (0..n) and fraction through the part of a longitude in the D-n
/// division of its sign.
fn sign_and_part(sidereal: f64, n: u32) -> (u32, u32, f64) {
    let l = sidereal.rem_euclid(360.0);
    let sign = ((l / 30.0).floor() as u32).min(11);
    let in_sign = (l - sign as f64 * 30.0).clamp(0.0, 30.0);
    let scaled = in_sign * n as f64 / 30.0;
    let part = (scaled.floor() as u32).min(n - 1);
    let frac = (scaled - part as f64).clamp(0.0, 1.0 - f64::EPSILON);
    (sign, part, frac)
}

fn out(varga_sign: u32, frac: f64) -> f64 {
    (varga_sign % 12) as f64 * 30.0 + frac * 30.0
}

/// Movable (0), fixed (1) or dual (2) quality of a sign.
const fn quality(sign: u32) -> u32 {
    sign % 3
}

/// Longitude in the D-n chart. Specific rules for every n in [`SUPPORTED_VARGAS`];
/// any other n uses the uniform "count from the same sign" rule.
pub fn varga(sidereal: f64, n: u32) -> f64 {
    if n <= 1 {
        return sidereal.rem_euclid(360.0);
    }
    // Composite vargas: a varga of a varga (JHora / PyJHora "traditional Parasara").
    match n {
        // D108 Ashtottaramsa = D12 of the D9 (PyJHora `ashtotharamsa_chart` method 1:
        // `mixed_chart(varga_factor_1=9, varga_factor_2=12)`).
        108 => return varga(varga(sidereal, 9), 12),
        // D144 Dwadas-dwadasamsa = D12 of the D12 (PyJHora `dwadas_dwadasamsa_chart`
        // method 1: `mixed_chart(12, 12)`).
        144 => return varga(varga(sidereal, 12), 12),
        _ => {}
    }
    let (sign, part, frac) = sign_and_part(sidereal, n);
    let odd = sign % 2 == 0; // Aries (0) is odd in the 1-based count
    let vs = match n {
        // Hora (BPHS 6.5–6): odd signs first half Sun (Leo), second half Moon (Cancer);
        // even signs reversed.
        2 => match (odd, part) {
            (true, 0) | (false, 1) => 4,
            _ => 3,
        },
        // Drekkana (BPHS 6.7–8): 1st, 5th, 9th from the sign.
        3 => sign + 4 * part,
        // Chaturthamsa (BPHS 6.9): 1st, 4th, 7th, 10th from the sign.
        4 => sign + 3 * part,
        // Panchamsa (not in BPHS 6; JHora default `PARASARA_TRADITIONAL`, PyJHora
        // `const.panchamsa_odd_signs = [0,10,8,2,6]`, `panchamsa_even_signs =
        // [1,5,11,9,7]`): the five parts go to the signs of Mars, Saturn, Jupiter,
        // Mercury, Venus — odd signs Aries, Aquarius, Sagittarius, Gemini, Libra; even
        // signs Taurus, Virgo, Pisces, Capricorn, Scorpio (the trimsamsa lords' order).
        5 => {
            const ODD: [u32; 5] = [0, 10, 8, 2, 6];
            const EVEN: [u32; 5] = [1, 5, 11, 9, 7];
            if odd {
                ODD[part as usize]
            } else {
                EVEN[part as usize]
            }
        }
        // Shashthamsa (not in BPHS 6; JHora default, PyJHora `shashthamsa_chart`
        // method 1): odd signs count from Aries, even signs from Libra.
        6 => (if odd { 0 } else { 6 }) + part,
        // Saptamsa (BPHS 6.10): odd from the sign, even from the 7th, forward
        // (PyJHora `d7_chart_method_default = PARASARA_EVEN_START_7TH_GO_FORWARD`).
        7 => (if odd { sign } else { sign + 6 }) + part,
        // Ashtamsa (not in BPHS 6; JHora default, PyJHora `ashtamsa_chart` method 1):
        // movable signs from Aries, fixed from Sagittarius, dual from Leo.
        8 => {
            (match quality(sign) {
                0 => 0,
                1 => 8,
                _ => 4,
            }) + part
        }
        // Navamsa (BPHS 6.11): movable from the sign, fixed from the 9th, dual from the
        // 5th (equivalently Aries / Capricorn / Libra / Cancer by element).
        9 => {
            (match quality(sign) {
                0 => sign,
                1 => sign + 8,
                _ => sign + 4,
            }) + part
        }
        // Dashamsa (BPHS 6.12): odd from the sign, even from the 9th.
        10 => (if odd { sign } else { sign + 8 }) + part,
        // Rudramsa (not in BPHS 6; JHora default `TRADITIONAL_PARASARA_SANJAY_RATH`,
        // PyJHora `rudramsa_chart`: `r = (12 - sign + l) % 12`): the first part of a
        // sign is the sign as far before Aries as the sign is after it (Aries → Aries,
        // Taurus → Pisces, Gemini → Aquarius …), then forward. B. V. Raman's
        // anti-zodiacal Ekadasamsa is not used.
        11 => (12 - sign) + part,
        // Dwadasamsa (BPHS 6.13): from the sign.
        12 => sign + part,
        // Shodasamsa (BPHS 6.14): movable from Aries, fixed from Leo, dual from
        // Sagittarius.
        16 => {
            (match quality(sign) {
                0 => 0,
                1 => 4,
                _ => 8,
            }) + part
        }
        // Vimsamsa (BPHS 6.15): movable from Aries, fixed from Sagittarius, dual from Leo.
        20 => {
            (match quality(sign) {
                0 => 0,
                1 => 8,
                _ => 4,
            }) + part
        }
        // Chaturvimsamsa / Siddhamsa (BPHS 6.16): odd from Leo, even from Cancer, forward.
        24 => (if odd { 4 } else { 3 }) + part,
        // Saptavimsamsa / Bhamsa (BPHS 6.17): fiery from Aries, earthy from Cancer, airy
        // from Libra, watery from Capricorn.
        27 => {
            (match sign % 4 {
                0 => 0,
                1 => 3,
                2 => 6,
                _ => 9,
            }) + part
        }
        // Trimsamsa (BPHS 6.18–19): unequal parts. Odd: Mars 5° (Aries), Saturn 5°
        // (Aquarius), Jupiter 8° (Sagittarius), Mercury 7° (Gemini), Venus 5° (Libra).
        // Even: Venus 5° (Taurus), Mercury 7° (Virgo), Jupiter 8° (Pisces), Saturn 5°
        // (Capricorn), Mars 5° (Scorpio).
        30 => {
            let l = sidereal.rem_euclid(360.0);
            let in_sign = l - ((l / 30.0).floor().min(11.0)) * 30.0;
            let table: [(f64, u32); 5] = if odd {
                [(5.0, 0), (10.0, 10), (18.0, 8), (25.0, 2), (30.0, 6)]
            } else {
                [(5.0, 1), (12.0, 5), (20.0, 11), (25.0, 9), (30.0, 7)]
            };
            let mut lo = 0.0;
            let mut chosen = (table[4].1, 1.0 - f64::EPSILON);
            for (hi, s) in table {
                if in_sign < hi {
                    chosen = (s, (in_sign - lo) / (hi - lo));
                    break;
                }
                lo = hi;
            }
            return out(chosen.0, chosen.1);
        }
        // Khavedamsa (BPHS 6.20): odd signs from Aries, even from Libra.
        40 => (if odd { 0 } else { 6 }) + part,
        // Akshavedamsa (BPHS 6.21): movable from Aries, fixed from Leo, dual from
        // Sagittarius.
        45 => {
            (match quality(sign) {
                0 => 0,
                1 => 4,
                _ => 8,
            }) + part
        }
        // Shashtiamsa (BPHS 6.22–23): from the sign itself (PyJHora
        // `PARASARA_TRADITIONAL_FROM_SIGN`).
        60 => sign + part,
        // Nava-navamsa (not in BPHS 6; JHora default
        // `PARASARA_TRADITIONAL_PARIVRITTI_CYCLIC`): the 81 parts run on cyclically from
        // Aries through the zodiac, `(81 × sign + part) mod 12` — identical to the
        // navamsa of the navamsa.
        81 => 81 * sign + part,
        // Nadiamsa (not in BPHS 6; JHora default `PVR_JHORA_METHOD_OWN_SIGN_UNIFORM_DIRECT`):
        // 150 equal parts of 12′, counted forward from the sign itself. The Chandra Kala
        // Nadi's own ordering of the 150 named nadiamsas is [`nadiamsa_ordinal`].
        150 => sign + part,
        _ => sign + part,
    };
    out(vs, frac)
}

/// Sign index 0–11 of a longitude in the D-n chart.
pub fn varga_sign(sidereal: f64, n: u32) -> u8 {
    ((varga(sidereal, n) / 30.0).floor() as u32 % 12) as u8
}

/// Ordinal 1–150 of the named nadiamsa (Vasudha, Vaishnavi, …) a longitude falls in,
/// by the Chandra Kala Nadi (Deva Keralam) rule: each sign holds 150 nadiamsas of 12′;
/// in movable signs they are counted from the first (1 → 150), in fixed signs from the
/// last (150 → 1), in dual signs from the middle (76 → 150, then 1 → 75). Source:
/// Chandra Kala Nadi, R. Santhanam's translation (Ranjan, 1993), introduction on the
/// nadiamsa; the same quality-wise ordering is PyJHora's `D150_CHART_METHOD` options 2–3
/// ("Movable: Aries, Fixed: Taurus [Rev], Dual: Gemini").
pub fn nadiamsa_ordinal(sidereal: f64) -> u8 {
    let (sign, part, _) = sign_and_part(sidereal, 150);
    let ord = match quality(sign) {
        0 => part + 1,
        1 => 150 - part,
        _ => (part + 75) % 150 + 1,
    };
    ord as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(l: f64) -> u32 {
        (l.rem_euclid(360.0) / 30.0).floor() as u32
    }

    /// Part boundaries of the D-n chart inside a sign, in degrees (unequal for D30).
    fn boundaries(n: u32) -> Vec<f64> {
        match n {
            30 => vec![5.0, 10.0, 18.0, 25.0], // odd-sign trimsamsa; even checked apart
            108 => (1..108).map(|k| k as f64 * 30.0 / 108.0).collect(),
            144 => (1..144).map(|k| k as f64 * 30.0 / 144.0).collect(),
            _ => (1..n).map(|k| k as f64 * 30.0 / n as f64).collect(),
        }
    }

    /// Reference rule written independently of `varga`: the varga sign of part `p` of
    /// sign `s` (0-based), straight from the rule statements cited above.
    fn reference(s: u32, p: u32, n: u32) -> u32 {
        let odd = s % 2 == 0;
        let q = s % 3;
        (match n {
            2 => {
                if (odd && p == 0) || (!odd && p == 1) {
                    4
                } else {
                    3
                }
            }
            3 => s + 4 * p,
            4 => s + 3 * p,
            5 => {
                if odd {
                    [0, 10, 8, 2, 6][p as usize]
                } else {
                    [1, 5, 11, 9, 7][p as usize]
                }
            }
            6 => p + if odd { 0 } else { 6 },
            7 => s + p + if odd { 0 } else { 6 },
            8 => p + [0, 8, 4][q as usize],
            9 => s + p + [0, 8, 4][q as usize],
            10 => s + p + if odd { 0 } else { 8 },
            11 => 12 - s + p,
            12 => s + p,
            16 => p + [0, 4, 8][q as usize],
            20 => p + [0, 8, 4][q as usize],
            24 => p + if odd { 4 } else { 3 },
            27 => p + [0, 3, 6, 9][(s % 4) as usize],
            40 => p + if odd { 0 } else { 6 },
            45 => p + [0, 4, 8][q as usize],
            60 => s + p,
            81 => 81 * s + p,
            150 => s + p,
            _ => unreachable!(),
        }) % 12
    }

    const EPS: f64 = 1e-9;
    const LAST: f64 = 29.0 + 59.0 / 60.0 + 59.0 / 3600.0; // 29°59′59″

    #[test]
    fn every_equal_part_varga_on_every_boundary_of_every_sign() {
        for n in [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 24, 27, 40, 45, 60, 81, 150] {
            for s in 0..12u32 {
                let base = s as f64 * 30.0;
                // 0° and 29°59′59″
                assert_eq!(sign(varga(base, n)), reference(s, 0, n), "D{n} sign {s} at 0°");
                assert_eq!(
                    sign(varga(base + LAST, n)),
                    reference(s, n - 1, n),
                    "D{n} sign {s} at 29°59′59″"
                );
                // just before and just after each internal boundary
                for (k, b) in boundaries(n).into_iter().enumerate() {
                    let k = k as u32;
                    assert_eq!(sign(varga(base + b - EPS, n)), reference(s, k, n), "D{n} s{s} before {b}");
                    assert_eq!(sign(varga(base + b + EPS, n)), reference(s, k + 1, n), "D{n} s{s} after {b}");
                }
                // midpoint of each part, and the position inside the varga sign
                for p in 0..n {
                    let mid = base + (p as f64 + 0.5) * 30.0 / n as f64;
                    let v = varga(mid, n);
                    assert_eq!(sign(v), reference(s, p, n), "D{n} s{s} part {p}");
                    assert!((v.rem_euclid(30.0) - 15.0).abs() < 1e-6, "D{n} s{s} p{p} {v}");
                }
            }
        }
    }

    #[test]
    fn trimsamsa_boundaries_odd_and_even() {
        // odd sign (Aries): Ar 0–5, Aq 5–10, Sg 10–18, Ge 18–25, Li 25–30
        let odd = [(0.0, 0), (5.0, 10), (10.0, 8), (18.0, 2), (25.0, 6)];
        // even sign (Taurus): Ta 0–5, Vi 5–12, Pi 12–20, Cp 20–25, Sc 25–30
        let even = [(0.0, 1), (5.0, 5), (12.0, 11), (20.0, 9), (25.0, 7)];
        for (base, table) in [(0.0, odd), (30.0, even)] {
            for (i, (lo, s)) in table.iter().enumerate() {
                assert_eq!(sign(varga(base + lo, 30)), *s, "{base}+{lo}");
                assert_eq!(sign(varga(base + lo + EPS, 30)), *s);
                if i > 0 {
                    assert_eq!(sign(varga(base + lo - EPS, 30)), table[i - 1].1, "{base}+{lo}-");
                }
            }
            assert_eq!(sign(varga(base + LAST, 30)), table[4].1);
        }
    }

    #[test]
    fn composite_vargas_d108_d144_on_boundaries() {
        // D108: D12 of D9; D144: D12 of D12. Check the composite against the two steps
        // done by hand from the cited rules at every part midpoint and just inside each
        // boundary.
        for (n, a) in [(108u32, 9u32), (144, 12)] {
            for s in 0..12u32 {
                let base = s as f64 * 30.0;
                for k in 0..n {
                    for off in [EPS, 0.5 * 30.0 / n as f64, 30.0 / n as f64 - EPS] {
                        let l = base + k as f64 * 30.0 / n as f64 + off;
                        let p1 = k / 12; // part of the first division
                        let p2 = k % 12; // part of the D12 inside it
                        let s1 = reference(s, p1, a);
                        let want = (s1 + p2) % 12;
                        assert_eq!(sign(varga(l, n)), want, "D{n} s{s} k{k} off{off}");
                    }
                }
                assert_eq!(sign(varga(base, n)), reference(reference(s, 0, a), 0, 12));
                assert_eq!(sign(varga(base + LAST, n)), reference(reference(s, a - 1, a), 11, 12));
            }
        }
    }

    #[test]
    fn nava_navamsa_is_the_navamsa_of_the_navamsa() {
        for i in 0..(12 * 81) {
            let l = (i as f64 + 0.37) * 30.0 / 81.0;
            assert_eq!(sign(varga(l, 81)), sign(varga(varga(l, 9), 9)), "{l}");
        }
    }

    #[test]
    fn nadiamsa_ordinals_by_quality() {
        assert_eq!(nadiamsa_ordinal(0.0), 1); // Aries, movable: first
        assert_eq!(nadiamsa_ordinal(LAST), 150);
        assert_eq!(nadiamsa_ordinal(30.0), 150); // Taurus, fixed: counted from the end
        assert_eq!(nadiamsa_ordinal(30.0 + LAST), 1);
        assert_eq!(nadiamsa_ordinal(60.0), 76); // Gemini, dual: from the middle
        assert_eq!(nadiamsa_ordinal(60.0 + 15.0 - EPS), 150);
        assert_eq!(nadiamsa_ordinal(60.0 + 15.0 + EPS), 1);
        assert_eq!(nadiamsa_ordinal(60.0 + LAST), 75);
        assert_eq!(nadiamsa_ordinal(0.2 - EPS), 1);
        assert_eq!(nadiamsa_ordinal(0.2 + EPS), 2);
        for i in 0..(12 * 150) {
            let o = nadiamsa_ordinal((i as f64 + 0.5) * 0.2);
            assert!((1..=150).contains(&o));
        }
    }

    /// Hand-checked table for the Kochi chart (1990-02-20 05:04 UTC, 9.9312° N
    /// 76.2673° E, Lahiri), sidereal longitudes from the DE421 pin in `kshana-chart`
    /// (`kochi_1990_02_20_birth_chart_and_dasha_balance`): Lagna 13.235310° (Aries),
    /// Sun 307.572599° (7.5726° Aquarius), Moon 244.505683° (4.5057° Sagittarius).
    /// Each cell was worked by hand: part = ⌊degree in sign × n / 30⌋, then the rule.
    /// E.g. D11 Sun: ⌊7.5726 × 11 / 30⌋ = 2, (12 − 10 + 2) mod 12 = 4 Leo; D108 Lagna:
    /// D9 Cancer at 29.118° → D12 part 11 → Cancer + 11 = Gemini; D144 Moon: D12
    /// Capricorn at 24.068° → part 9 → Capricorn + 9 = Libra.
    #[test]
    fn kochi_chart_table() {
        let (lagna, sun, moon) = (13.235_310, 307.572_599, 244.505_683);
        #[rustfmt::skip]
        let table: [(u32, [u32; 3]); 23] = [
            (2, [4, 4, 4]),    (3, [4, 10, 8]),   (4, [3, 1, 8]),    (5, [8, 10, 0]),
            (6, [2, 1, 0]),    (7, [3, 11, 9]),   (8, [3, 10, 5]),   (9, [3, 8, 1]),
            (10, [4, 0, 9]),   (11, [4, 4, 5]),   (12, [5, 1, 9]),   (16, [7, 8, 10]),
            (20, [8, 1, 7]),   (24, [2, 10, 7]),  (27, [11, 0, 4]),  (30, [8, 10, 0]),
            (40, [5, 10, 6]),  (45, [7, 3, 2]),   (60, [2, 1, 5]),   (81, [11, 2, 0]),
            (108, [2, 11, 5]), (144, [8, 1, 6]),  (150, [6, 11, 6]),
        ];
        for (n, want) in table {
            let got = [lagna, sun, moon].map(|l| sign(varga(l, n)));
            assert_eq!(got, want, "D{n}");
        }
        assert_eq!(
            [lagna, sun, moon].map(nadiamsa_ordinal),
            // Aries: 67th part; Aquarius (fixed): 38th part → 151 − 38 = 113; Sagittarius
            // (dual): 23rd part → 23 + 75 = 98
            [67, 113, 98]
        );
    }

    #[test]
    fn navamsa_rules() {
        assert_eq!(sign(varga(1.0, 9)), 0); // Aries first navamsa → Aries
        assert_eq!(sign(varga(29.9, 9)), 8); // Aries ninth navamsa → Sagittarius
        assert_eq!(sign(varga(45.0, 9)), 1); // 15° Taurus (fixed): 5th navamsa from Capricorn → Taurus
        assert_eq!(sign(varga(61.0, 9)), 6); // 1° Gemini (dual): from Libra
        for i in 0..108 {
            let l = i as f64 * 360.0 / 108.0 + 0.5;
            assert!((0.0..360.0).contains(&varga(l, 9)));
        }
    }

    #[test]
    fn khavedamsa_and_akshavedamsa() {
        assert_eq!(varga_sign(0.1, 40), 0); // Aries first part → Aries
        assert_eq!(varga_sign(29.9, 40), 3); // 40th part: Aries + 39 → Cancer
        assert_eq!(varga_sign(30.1, 40), 6); // Taurus (even) first part → Libra
        assert_eq!(varga_sign(0.1, 45), 0);
        assert_eq!(varga_sign(30.1, 45), 4); // fixed → Leo
        assert_eq!(varga_sign(60.1, 45), 8); // dual → Sagittarius
        assert_eq!(varga_sign(29.9, 45), (44 % 12) as u8);
    }

    #[test]
    fn hora_and_drekkana() {
        assert_eq!(sign(varga(10.0, 2)), 4); // odd sign, first half → Leo
        assert_eq!(sign(varga(20.0, 2)), 3); // odd sign, second half → Cancer
        assert_eq!(sign(varga(40.0, 2)), 3); // even sign, first half → Cancer
        assert_eq!(sign(varga(5.0, 3)), 0);
        assert_eq!(sign(varga(15.0, 3)), 4);
        assert_eq!(sign(varga(25.0, 3)), 8);
    }

    #[test]
    fn trimshamsha_parts() {
        assert_eq!(sign(varga(2.0, 30)), 0); // Aries 0–5 → Mars → Aries
        assert_eq!(sign(varga(7.0, 30)), 10); // 5–10 → Saturn → Aquarius
        assert_eq!(sign(varga(15.0, 30)), 8); // 10–18 → Jupiter → Sagittarius
        assert_eq!(sign(varga(32.0, 30)), 1); // Taurus 0–5 → Venus → Taurus
        assert_eq!(sign(varga(59.0, 30)), 7); // Taurus 25–30 → Mars → Scorpio
    }

    #[test]
    fn dashamsha_and_dwadashamsha() {
        assert_eq!(sign(varga(2.0, 10)), 0);
        assert_eq!(sign(varga(33.0, 10)), 10); // Taurus (even): 2nd dashamsha counted from Capricorn → Aquarius
        assert_eq!(sign(varga(27.5, 12)), 11); // Aries 12th dwadashamsha → Pisces
        assert_eq!(sign(varga(0.4, 60)), 0);
        assert_eq!(sign(varga(29.9, 60)), 11);
    }

    #[test]
    fn outputs_stay_in_range() {
        for n in SUPPORTED_VARGAS {
            for i in 0..3600 {
                let l = i as f64 * 0.1 - 0.05;
                let v = varga(l, n);
                assert!((0.0..360.0).contains(&v), "D{n} {l} → {v}");
            }
            assert!((0.0..360.0).contains(&varga(359.999_999_999, n)));
            assert!((0.0..360.0).contains(&varga(-1e-12, n)));
        }
    }
}
