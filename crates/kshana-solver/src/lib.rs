//! Root finding on [`Instant`]s.
//!
//! Every "exact moment" in Kshana (an ingress, a station, a return, a tithi boundary,
//! an eclipse contact) is the root of some smooth function of time. This crate finds
//! that root with Brent's method, working in integer nanoseconds so the answer is an
//! exact [`Instant`], and reports how tightly it was bracketed.
//!
//! The bracket `[a, b]` must be at most about 100 days wide so that nanosecond offsets
//! stay exactly representable in `f64` during interpolation.

#![forbid(unsafe_code)]

use kshana_time::Instant;

/// Widest bracket (in ns) for which every nanosecond offset is an exact `f64`.
pub const MAX_BRACKET_NS: i128 = 1 << 53;

/// A found root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Root {
    /// The instant at which the function crosses zero, to within `half_width_ns`.
    pub t: Instant,
    /// Half the width of the final bracket, in nanoseconds.
    pub half_width_ns: i128,
    /// Function evaluations used.
    pub evaluations: u32,
}

/// Why no root was returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverError {
    /// `f(a)` and `f(b)` have the same sign, so no root is bracketed.
    NotBracketed,
    /// The bracket is wider than [`MAX_BRACKET_NS`].
    BracketTooWide,
    /// The iteration budget ran out before reaching the tolerance.
    NoConvergence,
    /// The function returned NaN.
    NotANumber,
}

/// Find `t` in `[a, b]` with `f(t) = 0`, to within `tol_ns` nanoseconds.
///
/// Brent's method: bisection safety with secant / inverse-quadratic speed.
/// For the smooth functions astrology needs, a 1 µs answer in a one-day bracket
/// takes about 8–12 evaluations.
// `evals` counts function calls, two of which happen before the loop.
#[allow(clippy::explicit_counter_loop)]
pub fn find_root<F>(mut f: F, a: Instant, b: Instant, tol_ns: i128) -> Result<Root, SolverError>
where
    F: FnMut(Instant) -> f64,
{
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    let span = b.ns_since(a);
    if span > MAX_BRACKET_NS {
        return Err(SolverError::BracketTooWide);
    }
    let tol = tol_ns.max(1) as f64;
    let origin = a;
    let at = |x: f64| origin.add_ns(x.round() as i128);

    let mut xa = 0.0f64;
    let mut xb = span as f64;
    let mut fa = f(at(xa));
    let mut fb = f(at(xb));
    let mut evals = 2u32;
    if fa.is_nan() || fb.is_nan() {
        return Err(SolverError::NotANumber);
    }
    if fa == 0.0 {
        return Ok(Root {
            t: at(xa),
            half_width_ns: 0,
            evaluations: evals,
        });
    }
    if fb == 0.0 {
        return Ok(Root {
            t: at(xb),
            half_width_ns: 0,
            evaluations: evals,
        });
    }
    if (fa > 0.0) == (fb > 0.0) {
        return Err(SolverError::NotBracketed);
    }

    let mut xc = xa;
    let mut fc = fa;
    let mut d = xb - xa;
    let mut e = d;

    for _ in 0..200 {
        if (fb > 0.0) == (fc > 0.0) {
            xc = xa;
            fc = fa;
            d = xb - xa;
            e = d;
        }
        if fc.abs() < fb.abs() {
            xa = xb;
            xb = xc;
            xc = xa;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let m = 0.5 * (xc - xb);
        if m.abs() <= tol || fb == 0.0 {
            // Final answer: the better of the bracket ends, with the bracket half-width.
            // An exact zero is an exact answer, whatever the bracket still spans.
            let half = if fb == 0.0 {
                0.0
            } else {
                (xc - xb).abs() * 0.5
            };
            return Ok(Root {
                t: at(xb),
                half_width_ns: half.ceil() as i128,
                evaluations: evals,
            });
        }
        if e.abs() >= tol && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if xa == xc {
                p = 2.0 * m * s;
                q = 1.0 - s;
            } else {
                let qq = fa / fc;
                let r = fb / fc;
                p = s * (2.0 * m * qq * (qq - r) - (xb - xa) * (r - 1.0));
                q = (qq - 1.0) * (r - 1.0) * (s - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if 2.0 * p < (3.0 * m * q - (tol * q).abs()).min((e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = m;
                e = m;
            }
        } else {
            d = m;
            e = m;
        }
        xa = xb;
        fa = fb;
        if d.abs() > tol {
            xb += d;
        } else {
            xb += if m > 0.0 { tol } else { -tol };
        }
        fb = f(at(xb));
        evals += 1;
        if fb.is_nan() {
            return Err(SolverError::NotANumber);
        }
    }
    Err(SolverError::NoConvergence)
}

/// Signed angular difference `a − b` wrapped into `(−180°, 180°]`.
///
/// Use this to turn "body reaches longitude L" into a sign-changing function:
/// `|t| wrap180(lon(t) − L)`.
pub fn wrap180(deg: f64) -> f64 {
    let mut x = deg % 360.0;
    if x <= -180.0 {
        x += 360.0;
    } else if x > 180.0 {
        x -= 360.0;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_SEC;

    #[test]
    fn linear_root_is_exact_to_the_nanosecond() {
        let a = Instant::j2000();
        let b = a.add_ns(86_400 * NS_PER_SEC);
        let target = a.add_ns(12_345 * NS_PER_SEC + 678_901_234);
        let f = |t: Instant| t.ns_since(target) as f64;
        let r = find_root(f, a, b, 1).unwrap();
        assert!(
            r.t.ns_since(target).abs() <= 1,
            "off by {}",
            r.t.ns_since(target)
        );
        assert!(r.evaluations < 20);
    }

    #[test]
    fn sine_crossing_to_a_microsecond() {
        // f(t) = sin(2π · seconds / 86400) crosses zero at 43200 s.
        let a = Instant::j2000().add_ns(1_000 * NS_PER_SEC);
        let b = Instant::j2000().add_ns(80_000 * NS_PER_SEC);
        let f = |t: Instant| {
            let s = t.ns_since(Instant::j2000()) as f64 / NS_PER_SEC as f64;
            (2.0 * std::f64::consts::PI * s / 86_400.0).sin()
        };
        let r = find_root(f, a, b, 1_000).unwrap();
        let want = Instant::j2000().add_ns(43_200 * NS_PER_SEC);
        assert!(
            r.t.ns_since(want).abs() <= 1_000,
            "off by {} ns",
            r.t.ns_since(want)
        );
        assert!(r.half_width_ns <= 1_000);
    }

    #[test]
    fn rejects_unbracketed_and_too_wide() {
        let a = Instant::j2000();
        let b = a.add_ns(10 * NS_PER_SEC);
        assert_eq!(
            find_root(|_| 1.0, a, b, 1).unwrap_err(),
            SolverError::NotBracketed
        );
        let c = a.add_ns(MAX_BRACKET_NS + 1);
        assert_eq!(
            find_root(|_| 1.0, a, c, 1).unwrap_err(),
            SolverError::BracketTooWide
        );
    }

    #[test]
    fn wrap() {
        assert_eq!(wrap180(0.0), 0.0);
        assert_eq!(wrap180(190.0), -170.0);
        assert_eq!(wrap180(-190.0), 170.0);
        assert_eq!(wrap180(180.0), 180.0);
        assert_eq!(wrap180(540.0), 180.0);
    }
}
