//! Exact instants at which an angle crosses the boundaries of a [`Division`].
//!
//! The scan steps through the range, reads the cell index at each end of a step, and
//! when the index differs by one cell solves the boundary with Brent's method; when it
//! differs by more (a fast body over fine cells) the step is bisected until every piece
//! holds at most one crossing. Nothing is interpolated: every instant is a solved root.

use kshana_ephem::SpkError;
use kshana_events::EventError;
use kshana_solver::{find_root, wrap180};
use kshana_time::Instant;

use crate::zodiac::Division;

/// One crossing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    /// The solved instant.
    pub at: Instant,
    /// Half-width of the final bracket, nanoseconds.
    pub half_width_ns: i128,
    /// Cell entered (index into `division.bounds()`).
    pub entered: usize,
    /// True when moving forward in longitude (direct), false when retrograde.
    pub direct: bool,
}

/// Find every crossing of `division` by the angle `f(t)` (degrees) in `[from, to]`,
/// stepping `step_ns` at a time and solving to `tol_ns`.
pub fn crossings<F>(
    f: F,
    division: Division,
    from: Instant,
    to: Instant,
    step_ns: i128,
    tol_ns: i128,
) -> Result<Vec<Crossing>, EventError>
where
    F: Fn(Instant) -> Result<f64, SpkError>,
{
    let bounds = division.bounds();
    let n = bounds.len();
    let mut out = Vec::new();
    let mut a = from;
    let mut la = f(a)?;
    while a < to {
        let b = if a.add_ns(step_ns) > to {
            to
        } else {
            a.add_ns(step_ns)
        };
        let lb = f(b)?;
        piece(&f, &bounds, n, division, a, la, b, lb, tol_ns, 0, &mut out)?;
        a = b;
        la = lb;
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn piece<F>(
    f: &F,
    bounds: &[(f64, f64)],
    n: usize,
    division: Division,
    a: Instant,
    la: f64,
    b: Instant,
    lb: f64,
    tol_ns: i128,
    depth: u32,
    out: &mut Vec<Crossing>,
) -> Result<(), EventError>
where
    F: Fn(Instant) -> Result<f64, SpkError>,
{
    let ia = division.index(la);
    let ib = division.index(lb);
    if ia == ib {
        return Ok(());
    }
    let forward = (ib + n - ia) % n;
    let backward = (ia + n - ib) % n;
    let motion = wrap180(lb - la);
    if forward == 1 && motion > 0.0 {
        let boundary = bounds[ib].0;
        let root = find_root(
            |t| f(t).map(|l| wrap180(l - boundary)).unwrap_or(f64::NAN),
            a,
            b,
            tol_ns,
        )?;
        out.push(Crossing {
            at: root.t,
            half_width_ns: root.half_width_ns,
            entered: ib,
            direct: true,
        });
        return Ok(());
    }
    if backward == 1 && motion < 0.0 {
        let boundary = bounds[ia].0;
        let root = find_root(
            |t| f(t).map(|l| wrap180(l - boundary)).unwrap_or(f64::NAN),
            a,
            b,
            tol_ns,
        )?;
        out.push(Crossing {
            at: root.t,
            half_width_ns: root.half_width_ns,
            entered: ib,
            direct: false,
        });
        return Ok(());
    }
    // More than one cell, or a turn inside the piece: bisect.
    if depth > 40 || b.ns_since(a) <= 2 * tol_ns.max(1) {
        return Ok(());
    }
    let m = a.add_ns(b.ns_since(a) / 2);
    let lm = f(m)?;
    piece(f, bounds, n, division, a, la, m, lm, tol_ns, depth + 1, out)?;
    piece(f, bounds, n, division, m, lm, b, lb, tol_ns, depth + 1, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_SEC;

    #[test]
    fn a_uniform_motion_crosses_every_subsub_once() {
        // 13.2°/day like the Moon, through one nakshatra: 81 sub-sub boundaries + the
        // sub-sub cells entered. Start just inside Ashwini, run 1.2 days.
        let from = Instant::j2000();
        let to = from.add_ns(NS_PER_SEC * 86_400 * 12 / 10);
        let f = |t: Instant| Ok(0.01 + 13.2 * (t.ns_since(from) as f64) / 86_400e9);
        let c = crossings(f, Division::SubSub, from, to, 2 * 3_600 * NS_PER_SEC, 1_000).unwrap();
        // 13.2 × 1.2 = 15.84° covers Ashwini (13.33°) and 2.5° of Bharani
        let expected = Division::SubSub.index(15.85);
        assert_eq!(c.len(), expected);
        for (i, x) in c.iter().enumerate() {
            assert_eq!(x.entered, i + 1);
            assert!(x.direct);
            assert!(x.half_width_ns <= 1_000);
        }
        for w in c.windows(2) {
            assert!(w[0].at < w[1].at);
        }
    }

    #[test]
    fn retrograde_motion_enters_the_lower_cell() {
        let from = Instant::j2000();
        let to = from.add_ns(NS_PER_SEC * 86_400 * 10);
        let f = |t: Instant| Ok(31.0 - 0.5 * (t.ns_since(from) as f64) / 86_400e9);
        let c = crossings(f, Division::Sign, from, to, 86_400 * NS_PER_SEC, 1_000).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].entered, 0);
        assert!(!c[0].direct);
        let days = c[0].at.ns_since(from) as f64 / 86_400e9;
        assert!((days - 2.0).abs() < 1e-6);
    }
}
