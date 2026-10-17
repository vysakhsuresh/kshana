//! Exact sidereal motion of the grahas for the relief, couple and timeline layers:
//! when a graha next changes sign, every sign change in a range, and when it returns
//! to a natal longitude. Each answer is a solved root on the real sky (the apparent
//! longitude from the kernel, the ayanamsa of the instant, the osculating node for
//! Rahu and Ketu), not a mean-motion estimate.

use kshana_ephem::{Body, Spk};
use kshana_sky::{apparent_longitude, true_node_longitude};
use kshana_solver::{find_root, wrap180};
use kshana_time::{Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, Ayanamsa, DashaLord};

/// The kernel body behind a graha; `None` for the nodes.
fn body(g: DashaLord) -> Option<Body> {
    Some(match g {
        DashaLord::Sun => Body::Sun,
        DashaLord::Moon => Body::Moon,
        DashaLord::Mars => Body::MarsBarycenter,
        DashaLord::Mercury => Body::Mercury,
        DashaLord::Jupiter => Body::JupiterBarycenter,
        DashaLord::Venus => Body::Venus,
        DashaLord::Saturn => Body::SaturnBarycenter,
        DashaLord::Rahu | DashaLord::Ketu => return None,
    })
}

/// The last instant every planetary segment of the kernel covers, less a day of margin.
/// Scans must stop here: DE421 ends in 2053, DE440s in 2150.
pub fn kernel_end(spk: &Spk) -> Instant {
    let end_et = spk
        .segments()
        .iter()
        .map(|s| s.end_et)
        .fold(f64::INFINITY, f64::min);
    if !end_et.is_finite() {
        return Instant::j2000();
    }
    // ET (TDB) seconds past J2000; TDB − TT is under two milliseconds, well inside the margin.
    Instant::j2000().add_seconds_f64(end_et - 86_400.0)
}

/// The first instant every planetary segment covers, plus a day of margin.
pub fn kernel_start(spk: &Spk) -> Instant {
    let start_et = spk
        .segments()
        .iter()
        .map(|s| s.start_et)
        .fold(f64::NEG_INFINITY, f64::max);
    if !start_et.is_finite() {
        return Instant::j2000();
    }
    Instant::j2000().add_seconds_f64(start_et + 86_400.0)
}

/// Sidereal longitude of a graha at `t`, degrees in `[0, 360)`.
pub fn sidereal_longitude(spk: &Spk, ayanamsa: Ayanamsa, g: DashaLord, t: Instant) -> Option<f64> {
    let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
    let trop = match body(g) {
        Some(b) => apparent_longitude(spk, b, t).ok()?,
        None => {
            let n = true_node_longitude(spk, t).ok()?;
            if g == DashaLord::Ketu {
                n + 180.0
            } else {
                n
            }
        }
    };
    Some((trop - ay).rem_euclid(360.0))
}

/// A safe coarse step for scanning a graha's sign: well under half the shortest time it
/// can spend in a sign, so no sign is skipped.
pub fn scan_step(g: DashaLord) -> i128 {
    match g {
        DashaLord::Moon => 4 * 3_600 * NS_PER_SEC,
        DashaLord::Sun | DashaLord::Mercury | DashaLord::Venus => NS_PER_DAY,
        DashaLord::Mars => 2 * NS_PER_DAY,
        DashaLord::Jupiter => 10 * NS_PER_DAY,
        DashaLord::Saturn => 20 * NS_PER_DAY,
        DashaLord::Rahu | DashaLord::Ketu => 4 * NS_PER_DAY,
    }
}

/// How far ahead to look for a graha's next sign change before giving up.
pub fn horizon_days(g: DashaLord) -> i64 {
    match g {
        DashaLord::Moon => 4,
        DashaLord::Sun => 40,
        DashaLord::Mercury | DashaLord::Venus => 160,
        DashaLord::Mars => 300,
        DashaLord::Jupiter => 480,
        DashaLord::Saturn => 1_200,
        DashaLord::Rahu | DashaLord::Ketu => 720,
    }
}

/// One sign change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignChange {
    /// The exact instant.
    pub at: Instant,
    /// Sign left, 0 = Aries.
    pub from: u8,
    /// Sign entered.
    pub to: u8,
}

fn refine(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    g: DashaLord,
    a: Instant,
    b: Instant,
    boundary: f64,
) -> Option<Instant> {
    let f = |t: Instant| {
        sidereal_longitude(spk, ayanamsa, g, t)
            .map(|l| wrap180(l - boundary))
            .unwrap_or(f64::NAN)
    };
    find_root(f, a, b, NS_PER_SEC).ok().map(|r| r.t)
}

/// Every sign change of `g` in `[from, to]`, chronological, each solved to the second.
pub fn sign_changes(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    g: DashaLord,
    from: Instant,
    to: Instant,
) -> Vec<SignChange> {
    let step = scan_step(g);
    let mut out = Vec::new();
    let mut a = from;
    let Some(mut la) = sidereal_longitude(spk, ayanamsa, g, a) else {
        return out;
    };
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let Some(lb) = sidereal_longitude(spk, ayanamsa, g, b) else {
            break;
        };
        let sa = (la / 30.0).floor() as i64;
        let sb = (lb / 30.0).floor() as i64;
        if sa != sb {
            let forward = wrap180(lb - la) > 0.0;
            let boundary = if forward {
                sb as f64 * 30.0
            } else {
                sa as f64 * 30.0
            };
            if let Some(t) = refine(spk, ayanamsa, g, a, b, boundary) {
                out.push(SignChange {
                    at: t,
                    from: sa.rem_euclid(12) as u8,
                    to: sb.rem_euclid(12) as u8,
                });
            }
        }
        a = b;
        la = lb;
    }
    out
}

/// The next sign change of `g` after `from`, within its horizon.
pub fn next_sign_change(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    g: DashaLord,
    from: Instant,
) -> Option<SignChange> {
    let to = from.add_ns(horizon_days(g) as i128 * NS_PER_DAY);
    let step = scan_step(g);
    let mut a = from;
    let mut la = sidereal_longitude(spk, ayanamsa, g, a)?;
    while a < to {
        let b = a.add_ns(step);
        let lb = sidereal_longitude(spk, ayanamsa, g, b)?;
        let sa = (la / 30.0).floor() as i64;
        let sb = (lb / 30.0).floor() as i64;
        if sa != sb {
            let forward = wrap180(lb - la) > 0.0;
            let boundary = if forward {
                sb as f64 * 30.0
            } else {
                sa as f64 * 30.0
            };
            let t = refine(spk, ayanamsa, g, a, b, boundary)?;
            return Some(SignChange {
                at: t,
                from: sa.rem_euclid(12) as u8,
                to: sb.rem_euclid(12) as u8,
            });
        }
        a = b;
        la = lb;
    }
    None
}

/// Instants in `[from, to]` when `g` returns to the sidereal longitude `target_deg`
/// (a return to its natal place when `target_deg` is the natal longitude). Retrograde
/// loops can give three crossings close together; all are returned.
pub fn returns(
    spk: &Spk,
    ayanamsa: Ayanamsa,
    g: DashaLord,
    target_deg: f64,
    from: Instant,
    to: Instant,
) -> Vec<Instant> {
    let step = scan_step(g);
    let mut out = Vec::new();
    let mut a = from;
    let Some(mut fa) = sidereal_longitude(spk, ayanamsa, g, a).map(|l| wrap180(l - target_deg)) else {
        return out;
    };
    while a < to {
        let b = if a.add_ns(step) > to {
            to
        } else {
            a.add_ns(step)
        };
        let Some(fb) = sidereal_longitude(spk, ayanamsa, g, b).map(|l| wrap180(l - target_deg)) else {
            break;
        };
        if (fa > 0.0) != (fb > 0.0) && (fa - fb).abs() < 180.0 {
            let f = |t: Instant| {
                sidereal_longitude(spk, ayanamsa, g, t)
                    .map(|l| wrap180(l - target_deg))
                    .unwrap_or(f64::NAN)
            };
            if let Ok(r) = find_root(f, a, b, NS_PER_SEC) {
                out.push(r.t);
            }
        }
        a = b;
        fa = fb;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn saturn_changes_sign_about_every_two_and_a_half_years() {
        let Some(spk) = kernel() else { return };
        let from = Utc::new(2020, 1, 1, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let to = Utc::new(2030, 1, 1, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let v = sign_changes(&spk, Ayanamsa::Lahiri, DashaLord::Saturn, from, to);
        // Saturn: Capricorn 2020, Aquarius 2022–23 (with a retrograde return), Pisces
        // 2025, Aries 2027–28 (retrograde return), Taurus 2029 (retrograde return): ten.
        assert!(v.len() >= 8 && v.len() <= 11, "{v:?}");
        assert_eq!((v[0].from, v[0].to), (8, 9));
        for w in v.windows(2) {
            assert!(w[0].at < w[1].at);
        }
        for c in &v {
            let l = sidereal_longitude(&spk, Ayanamsa::Lahiri, DashaLord::Saturn, c.at).unwrap();
            let off = l.rem_euclid(30.0);
            assert!(off.min(30.0 - off) < 1e-3, "{l}");
        }
        let n = next_sign_change(&spk, Ayanamsa::Lahiri, DashaLord::Saturn, from).unwrap();
        assert_eq!(n.at, v[0].at);
    }

    #[test]
    fn the_moon_enters_a_new_sign_within_three_days() {
        let Some(spk) = kernel() else { return };
        let from = Utc::new(2026, 10, 17, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let n = next_sign_change(&spk, Ayanamsa::Lahiri, DashaLord::Moon, from).unwrap();
        assert!(n.at > from && n.at.ns_since(from) < 3 * NS_PER_DAY);
        assert_eq!((n.from + 1) % 12, n.to);
    }

    #[test]
    fn rahu_moves_backwards_and_jupiter_returns_every_twelve_years() {
        let Some(spk) = kernel() else { return };
        let from = Utc::new(2026, 1, 1, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let r = next_sign_change(&spk, Ayanamsa::Lahiri, DashaLord::Rahu, from).unwrap();
        assert_eq!((r.from + 11) % 12, r.to, "{r:?}");
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = sidereal_longitude(&spk, Ayanamsa::Lahiri, DashaLord::Jupiter, birth).unwrap();
        let to = birth.add_ns(40 * 365 * NS_PER_DAY);
        let v = returns(
            &spk,
            Ayanamsa::Lahiri,
            DashaLord::Jupiter,
            natal,
            birth.add_ns(NS_PER_DAY),
            to,
        );
        // three returns in forty years, each possibly tripled by a retrograde loop, plus
        // any re-crossing of the natal degree in the weeks after birth
        assert!(v.len() >= 3 && v.len() <= 12, "{}", v.len());
        let first_years = v
            .iter()
            .map(|t| t.ns_since(birth) as f64 / (365.25 * NS_PER_DAY as f64))
            .find(|y| *y > 2.0)
            .unwrap();
        assert!((first_years - 11.86).abs() < 1.2, "{first_years}");
    }
}
