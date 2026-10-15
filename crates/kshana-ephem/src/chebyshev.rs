//! Chebyshev series evaluation (Clenshaw recurrence) with the derivative.

/// Evaluate `Σ c_k T_k(τ)` and its derivative with respect to τ, for τ in `[-1, 1]`.
pub fn eval_with_derivative(coeffs: &[f64], tau: f64) -> (f64, f64) {
    let n = coeffs.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    // Value via Clenshaw.
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    let two_tau = 2.0 * tau;
    for k in (1..n).rev() {
        let b0 = coeffs[k] + two_tau * b1 - b2;
        b2 = b1;
        b1 = b0;
    }
    let value = coeffs[0] + tau * b1 - b2;

    // Derivative via the recurrence T'_k = 2 τ T'_{k-1} - T'_{k-2} + 2 T_{k-1}.
    let mut t0 = 1.0;
    let mut t1 = tau;
    let mut d0 = 0.0;
    let mut d1 = 1.0;
    let mut deriv = if n > 1 { coeffs[1] * d1 } else { 0.0 };
    for &c in coeffs.iter().skip(2) {
        let t2 = two_tau * t1 - t0;
        let d2 = two_tau * d1 - d0 + 2.0 * t1;
        deriv += c * d2;
        t0 = t1;
        t1 = t2;
        d0 = d1;
        d1 = d2;
    }
    (value, deriv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_direct_polynomial() {
        // T0=1, T1=x, T2=2x²−1, T3=4x³−3x
        let c = [1.0, 2.0, 3.0, 4.0];
        for i in 0..=20 {
            let x = -1.0 + i as f64 / 10.0;
            let want =
                1.0 + 2.0 * x + 3.0 * (2.0 * x * x - 1.0) + 4.0 * (4.0 * x * x * x - 3.0 * x);
            let dwant = 2.0 + 3.0 * 4.0 * x + 4.0 * (12.0 * x * x - 3.0);
            let (v, d) = eval_with_derivative(&c, x);
            assert!((v - want).abs() < 1e-12, "x={x} v={v} want={want}");
            assert!((d - dwant).abs() < 1e-11, "x={x} d={d} want={dwant}");
        }
    }
}
