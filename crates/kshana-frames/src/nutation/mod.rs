//! Nutation: IAU 2000A (1 365 terms, ~0.1 mas), IAU 2000B (77 terms, ~1 mas over
//! 1995–2050) and the IAU 2006 rate adjustment that turns 2000A into 2006A.
//!
//! Formulas follow SOFA/ERFA `nut00a`, `nut00b` and `nut06a`; the coefficient tables
//! in `tables.rs` are generated verbatim from the ERFA sources.

pub mod tables;

use crate::ARCSEC;
use std::f64::consts::TAU;
use tables::{LUNISOLAR_2000A, LUNISOLAR_2000B, PLANETARY_2000A};

const TURNAS: f64 = 1_296_000.0;
/// 0.1 µas → radians.
const U2R: f64 = ARCSEC / 1e7;
const MAS2R: f64 = ARCSEC / 1e3;

fn turn(arcsec: f64) -> f64 {
    arcsec.rem_euclid(TURNAS) * ARCSEC
}

/// Mean anomaly of the Moon, l (IERS 2003).
pub fn fa_l(t: f64) -> f64 {
    turn(
        485_868.249_036
            + t * (1_717_915_923.217_8 + t * (31.879_2 + t * (0.051_635 + t * -0.000_244_70))),
    )
}
/// Mean anomaly of the Sun, l′.
pub fn fa_lp(t: f64) -> f64 {
    turn(
        1_287_104.793_05
            + t * (129_596_581.048_1 + t * (-0.553_2 + t * (0.000_136 + t * -0.000_011_49))),
    )
}
/// Mean longitude of the Moon minus that of the ascending node, F.
pub fn fa_f(t: f64) -> f64 {
    turn(
        335_779.526_232
            + t * (1_739_527_262.847_8 + t * (-12.751_2 + t * (-0.001_037 + t * 0.000_004_17))),
    )
}
/// Mean elongation of the Moon from the Sun, D.
pub fn fa_d(t: f64) -> f64 {
    turn(
        1_072_260.703_692
            + t * (1_602_961_601.209_0 + t * (-6.370_6 + t * (0.006_593 + t * -0.000_031_69))),
    )
}
/// Mean longitude of the Moon's ascending node, Ω.
pub fn fa_om(t: f64) -> f64 {
    turn(
        450_160.398_036
            + t * (-6_962_890.543_1 + t * (7.472_2 + t * (0.007_702 + t * -0.000_059_39))),
    )
}
fn fa_me(t: f64) -> f64 {
    (4.402_608_842 + 2_608.790_314_157_4 * t).rem_euclid(TAU)
}
fn fa_ve(t: f64) -> f64 {
    (3.176_146_697 + 1_021.328_554_621_1 * t).rem_euclid(TAU)
}
fn fa_e(t: f64) -> f64 {
    (1.753_470_314 + 628.307_584_999_1 * t).rem_euclid(TAU)
}
fn fa_ma(t: f64) -> f64 {
    (6.203_480_913 + 334.061_242_670_0 * t).rem_euclid(TAU)
}
fn fa_ju(t: f64) -> f64 {
    (0.599_546_497 + 52.969_096_264_1 * t).rem_euclid(TAU)
}
fn fa_sa(t: f64) -> f64 {
    (0.874_016_757 + 21.329_910_496_0 * t).rem_euclid(TAU)
}
fn fa_ur(t: f64) -> f64 {
    (5.481_293_872 + 7.478_159_856_7 * t).rem_euclid(TAU)
}
fn fa_pa(t: f64) -> f64 {
    (0.024_381_750 + 0.000_005_386_91 * t) * t
}

/// IAU 2000A nutation (Δψ, Δε) in radians for `t` TT centuries since J2000.0.
pub fn nutation_iau2000a(t: f64) -> (f64, f64) {
    // Luni-solar
    let el = fa_l(t);
    let elp = turn(
        1_287_104.793_05
            + t * (129_596_581.048_1 + t * (-0.553_2 + t * (0.000_136 + t * -0.000_011_49))),
    );
    let f = fa_f(t);
    let d = turn(
        1_072_260.703_69
            + t * (1_602_961_601.209_0 + t * (-6.370_6 + t * (0.006_593 + t * -0.000_031_69))),
    );
    let om = fa_om(t);
    let (mut dp, mut de) = (0.0, 0.0);
    for term in LUNISOLAR_2000A.iter().rev() {
        let n = &term.n;
        let arg = (n[0] as f64 * el
            + n[1] as f64 * elp
            + n[2] as f64 * f
            + n[3] as f64 * d
            + n[4] as f64 * om)
            .rem_euclid(TAU);
        let (s, c) = arg.sin_cos();
        let k = &term.c;
        dp += (k[0] + k[1] * t) * s + k[2] * c;
        de += (k[3] + k[4] * t) * c + k[5] * s;
    }
    let dpsi_ls = dp * U2R;
    let deps_ls = de * U2R;

    // Planetary (with the MHB2000 simplified luni-solar arguments)
    let al = (2.355_555_98 + 8_328.691_426_955_4 * t).rem_euclid(TAU);
    let af = (1.627_905_234 + 8_433.466_158_131 * t).rem_euclid(TAU);
    let ad = (5.198_466_741 + 7_771.377_146_812_1 * t).rem_euclid(TAU);
    let aom = (2.182_439_20 - 33.757_045 * t).rem_euclid(TAU);
    let apa = fa_pa(t);
    let (alme, alve, alea, alma, alju, alsa, alur) = (
        fa_me(t),
        fa_ve(t),
        fa_e(t),
        fa_ma(t),
        fa_ju(t),
        fa_sa(t),
        fa_ur(t),
    );
    let alne = (5.321_159_000 + 3.812_777_400_0 * t).rem_euclid(TAU);
    let (mut dp, mut de) = (0.0, 0.0);
    for term in PLANETARY_2000A.iter().rev() {
        let n = &term.n;
        let arg = (n[0] as f64 * al
            + n[1] as f64 * af
            + n[2] as f64 * ad
            + n[3] as f64 * aom
            + n[4] as f64 * alme
            + n[5] as f64 * alve
            + n[6] as f64 * alea
            + n[7] as f64 * alma
            + n[8] as f64 * alju
            + n[9] as f64 * alsa
            + n[10] as f64 * alur
            + n[11] as f64 * alne
            + n[12] as f64 * apa)
            .rem_euclid(TAU);
        let (s, c) = arg.sin_cos();
        let k = &term.c;
        dp += k[0] as f64 * s + k[1] as f64 * c;
        de += k[2] as f64 * s + k[3] as f64 * c;
    }
    (dpsi_ls + dp * U2R, deps_ls + de * U2R)
}

/// IAU 2000B nutation (77 terms plus fixed planetary offsets), ~1 mas, cheap.
pub fn nutation_iau2000b(t: f64) -> (f64, f64) {
    let el = turn(485_868.249_036 + 1_717_915_923.217_8 * t);
    let elp = turn(1_287_104.793_05 + 129_596_581.048_1 * t);
    let f = turn(335_779.526_232 + 1_739_527_262.847_8 * t);
    let d = turn(1_072_260.703_69 + 1_602_961_601.209_0 * t);
    let om = turn(450_160.398_036 - 6_962_890.543_1 * t);
    let (mut dp, mut de) = (0.0, 0.0);
    for term in LUNISOLAR_2000B.iter().rev() {
        let n = &term.n;
        let arg = (n[0] as f64 * el
            + n[1] as f64 * elp
            + n[2] as f64 * f
            + n[3] as f64 * d
            + n[4] as f64 * om)
            .rem_euclid(TAU);
        let (s, c) = arg.sin_cos();
        let k = &term.c;
        dp += (k[0] + k[1] * t) * s + k[2] * c;
        de += (k[3] + k[4] * t) * c + k[5] * s;
    }
    (dp * U2R - 0.135 * MAS2R, de * U2R + 0.388 * MAS2R)
}

/// IAU 2006A nutation: 2000A with the P03 rate adjustment (SOFA `nut06a`).
pub fn nutation_iau2006a(t: f64) -> (f64, f64) {
    let fj2 = -2.7774e-6 * t;
    let (dp, de) = nutation_iau2000a(t);
    (dp + dp * (0.4697e-6 + fj2), de + de * fj2)
}
