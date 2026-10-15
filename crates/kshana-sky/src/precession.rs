//! IAU 2006 bias-precession (Fukushima–Williams angles) and the rotation that takes an
//! ICRF vector to the true equator and equinox of date.

use kshana_frames::ARCSEC;

/// 3×3 rotation matrix as rows.
pub type Mat3 = [[f64; 3]; 3];

/// Fukushima–Williams bias-precession angles (γ̄, φ̄, ψ̄) in radians for `t` TT centuries
/// since J2000.0 (IAU 2006, SOFA `pfw06`). Frame bias is included.
pub fn fw_angles(t: f64) -> (f64, f64, f64) {
    let gamb = (-0.052_928
        + t * (10.556_378
            + t * (0.493_204_4
                + t * (-0.000_312_38 + t * (-0.000_002_788 + t * 0.000_000_026_0)))))
        * ARCSEC;
    let phib = (84_381.412_819
        + t * (-46.811_016
            + t * (0.051_126_8
                + t * (0.000_532_89 + t * (-0.000_000_440 + t * -0.000_000_017_6)))))
        * ARCSEC;
    let psib = (-0.041_775
        + t * (5_038.481_484
            + t * (1.558_417_5
                + t * (-0.000_185_22 + t * (-0.000_026_452 + t * -0.000_000_014_8)))))
        * ARCSEC;
    (gamb, phib, psib)
}

fn identity() -> Mat3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

/// Premultiply `m` by a rotation of `theta` about the x-axis (SOFA `rx`).
fn rx(theta: f64, m: &mut Mat3) {
    let (s, c) = theta.sin_cos();
    for col in 0..3 {
        let a = m[1][col];
        let b = m[2][col];
        m[1][col] = c * a + s * b;
        m[2][col] = -s * a + c * b;
    }
}

/// Premultiply `m` by a rotation of `theta` about the z-axis (SOFA `rz`).
fn rz(theta: f64, m: &mut Mat3) {
    let (s, c) = theta.sin_cos();
    for col in 0..3 {
        let a = m[0][col];
        let b = m[1][col];
        m[0][col] = c * a + s * b;
        m[1][col] = -s * a + c * b;
    }
}

/// Bias-precession-nutation matrix ICRF → true equator and equinox of date
/// (SOFA `fw2m` applied to the `pfw06` angles plus nutation, as in `pnm06a`).
pub fn bias_precession_nutation_matrix(t: f64, dpsi: f64, deps: f64) -> Mat3 {
    let (gamb, phib, psib) = fw_angles(t);
    let eps_a = kshana_frames::mean_obliquity_iau2006(t);
    let mut m = identity();
    rz(gamb, &mut m);
    rx(phib, &mut m);
    rz(-(psib + dpsi), &mut m);
    rx(-(eps_a + deps), &mut m);
    m
}

/// Apply a matrix to a vector.
pub fn apply(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Rotate a vector about the x-axis by `theta` (equator → ecliptic when `theta` is the
/// obliquity).
pub fn rotate_x(theta: f64, v: [f64; 3]) -> [f64; 3] {
    let (s, c) = theta.sin_cos();
    [v[0], c * v[1] + s * v[2], -s * v[1] + c * v[2]]
}

/// Apply the transpose (inverse) of a rotation matrix to a vector.
pub fn apply_transpose(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_orthonormal() {
        let m = bias_precession_nutation_matrix(0.268, 1e-5, 2e-5);
        for i in 0..3 {
            for j in 0..3 {
                let dot: f64 = (0..3).map(|k| m[i][k] * m[j][k]).sum();
                let want = if i == j { 1.0 } else { 0.0 };
                assert!((dot - want).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn at_j2000_only_the_frame_bias_remains() {
        // With no nutation the J2000 matrix should be within ~0.1″ of identity.
        let m = bias_precession_nutation_matrix(0.0, 0.0, 0.0);
        for i in 0..3 {
            for j in 0..3 {
                let want = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (m[i][j] - want).abs() < 0.2 * ARCSEC,
                    "m[{i}][{j}] = {}",
                    m[i][j]
                );
            }
        }
    }

    #[test]
    fn precession_moves_the_equinox_about_fifty_arcsec_per_year() {
        // The x-axis of date, seen in ICRF, should drift ~50.3″/yr in longitude.
        let a = bias_precession_nutation_matrix(0.0, 0.0, 0.0);
        let b = bias_precession_nutation_matrix(0.01, 0.0, 0.0);
        let ex = [1.0, 0.0, 0.0];
        let pa = apply(&a, ex);
        let pb = apply(&b, ex);
        let ang = (pa[0] * pb[0] + pa[1] * pb[1] + pa[2] * pb[2])
            .clamp(-1.0, 1.0)
            .acos();
        // ~50.3″ in longitude ≈ 46.1″ along the equator-ish great circle
        assert!(
            ang / ARCSEC > 40.0 && ang / ARCSEC < 55.0,
            "{}",
            ang / ARCSEC
        );
    }
}
