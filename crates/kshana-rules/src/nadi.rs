//! Event-hitting transits in the Nadi manner: the slow movers over the exact degrees of
//! the birth chart.
//!
//! * **Bhrigu Bindu.** The midpoint of the natal Rahu and Moon, counted from Rahu to
//!   the Moon in the order of the signs (`BB = Rahu + ((Moon − Rahu) mod 360) / 2`).
//!   The point and its use ("when Jupiter, Saturn or Rahu transits the Bhrigu Bindu, an
//!   event of the house it falls in comes") belong to the Bhrigu Nadi tradition as
//!   popularised by R. G. Rao in his Nadi writings; the Rahu-to-Moon direction is the
//!   one common software follows. The exact rule for an arc over 180° differs between
//!   teachers; this one is stated and tested below.
//! * **Transits over natal degrees and their trines.** In the Bhrigu Nadi method (as
//!   taught by R. G. Rao and in C. S. Patel's Nadi writings) Jupiter's passage over a
//!   natal planet, or over the 5th and 9th from it (its trines, which Nadi reads as one
//!   with the planet), brings out what that planet signifies; Saturn's passage does the
//!   same for work, duty and the slow results. Here the passage is taken at the exact
//!   degree (the natal longitude and that longitude ± 120°), to the second.
//! * **Returns.** Jupiter's return to its natal degree (about every 12 years) and
//!   Saturn's (about every 29.5 years) are the strongest of these hits for the planet's
//!   own matters; Rahu's passage over the natal Moon and over the Bhrigu Bindu completes
//!   the set.
//!
//! [`hits`] finds every exact crossing in a span, each solved to the second on the real
//! sky (apparent longitude, the ayanamsa of the instant, the osculating node), with the
//! same scan-and-bisect as [`crate::transit::returns`]. [`NadiTable`] turns the current
//! positions of Jupiter, Saturn and Rahu into a reading per life, so the hits count as one
//! more witness in [`crate::consensus`].

use crate::forecast::{lord_score, Life, Natal, GRAHAS};
use crate::transit::sidereal_longitude;
use kshana_ephem::Spk;
use kshana_solver::{find_root, wrap180};
use kshana_time::{Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{Ayanamsa, DashaLord};

/// Orb of a hit, degrees: the reading of a passage falls linearly to nothing at this
/// distance from the exact degree. Three degrees keeps Jupiter's hit to about five weeks
/// and Saturn's to about three months at their usual speeds, the spans Nadi readers
/// quote for "the year of the transit narrowed to the month".
pub const ORB_DEG: f64 = 3.0;

/// The midpoint of natal Rahu and the Moon, from Rahu to the Moon in sign order.
pub fn bhrigu_bindu(rahu: f64, moon: f64) -> f64 {
    (rahu + (moon - rahu).rem_euclid(360.0) / 2.0).rem_euclid(360.0)
}

/// A sensitive point of the birth chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NadiPoint {
    /// The Bhrigu Bindu.
    BhriguBindu,
    /// A natal planet's own degree.
    Planet(DashaLord),
    /// The 5th or 9th from a natal planet (its degree ± 120°).
    Trine(DashaLord),
}

impl NadiPoint {
    /// The natal planet behind the point (the Moon for the Bhrigu Bindu, as the point is
    /// read for the mind and the matters of the house it falls in).
    pub fn graha(self) -> Option<DashaLord> {
        match self {
            NadiPoint::BhriguBindu => None,
            NadiPoint::Planet(g) | NadiPoint::Trine(g) => Some(g),
        }
    }

    /// The point in an astrologer's words (expert view only).
    pub fn expert(self) -> String {
        match self {
            NadiPoint::BhriguBindu => "the Bhrigu Bindu".into(),
            NadiPoint::Planet(g) => format!("natal {g:?}"),
            NadiPoint::Trine(g) => format!("the trine of natal {g:?}"),
        }
    }
}

/// What kind of passage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitKind {
    /// A planet back over its own natal degree.
    Return,
    /// A planet over another natal planet's degree, or over the Bhrigu Bindu.
    Conjunction,
    /// A planet over the trine of a natal planet.
    Trine,
}

/// One exact passage of a slow mover over a sensitive point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NadiHit {
    /// Jupiter, Saturn or Rahu.
    pub transit: DashaLord,
    /// The point crossed.
    pub point: NadiPoint,
    /// The point's sidereal longitude, degrees.
    pub lon: f64,
    /// The exact instant, to the second.
    pub at: Instant,
    /// What kind of passage.
    pub kind: HitKind,
    /// True when the mover is going backwards at the crossing (a retrograde pass).
    pub backwards: bool,
}

impl NadiHit {
    /// The hit in an astrologer's words (expert view only).
    pub fn expert(&self) -> String {
        let k = match self.kind {
            HitKind::Return => "return over",
            HitKind::Conjunction => "over",
            HitKind::Trine => "over",
        };
        format!("{:?} {k} {}{}", self.transit, self.point.expert(), if self.backwards { " (retrograde pass)" } else { "" })
    }
}

/// The movers this module follows.
pub const MOVERS: [DashaLord; 3] = [DashaLord::Jupiter, DashaLord::Saturn, DashaLord::Rahu];

/// The sensitive points each mover is followed over: every natal planet's degree and the
/// Bhrigu Bindu; for Jupiter and Saturn also the two trines of every natal planet (Rahu
/// is read over degrees only, as the Nadi texts use it).
pub fn targets(natal: &Natal, mover: DashaLord) -> Vec<(NadiPoint, f64)> {
    let f = &natal.facts;
    let mut v = vec![(NadiPoint::BhriguBindu, bhrigu_bindu(f.planet(DashaLord::Rahu).sidereal, f.planet(DashaLord::Moon).sidereal))];
    for g in GRAHAS {
        let lon = f.planet(g).sidereal;
        v.push((NadiPoint::Planet(g), lon));
        if mover != DashaLord::Rahu && !matches!(g, DashaLord::Rahu | DashaLord::Ketu) {
            v.push((NadiPoint::Trine(g), (lon + 120.0).rem_euclid(360.0)));
            v.push((NadiPoint::Trine(g), (lon + 240.0).rem_euclid(360.0)));
        }
    }
    v
}

fn step(g: DashaLord) -> i128 {
    // Half of `transit::scan_step`: a crossing near a station is not stepped over.
    match g {
        DashaLord::Jupiter => 5 * NS_PER_DAY,
        DashaLord::Saturn => 10 * NS_PER_DAY,
        _ => 3 * NS_PER_DAY,
    }
}

/// Every exact passage of Jupiter, Saturn and Rahu over the sensitive points of `natal`
/// in `[from, to]`, in time order.
pub fn hits(spk: &Spk, ayanamsa: Ayanamsa, natal: &Natal, from: Instant, to: Instant) -> Vec<NadiHit> {
    let mut out = Vec::new();
    for mover in MOVERS {
        let tg = targets(natal, mover);
        let st = step(mover);
        let mut a = from;
        let Some(mut la) = sidereal_longitude(spk, ayanamsa, mover, a) else { continue };
        while a < to {
            let b = if a.add_ns(st) > to { to } else { a.add_ns(st) };
            let Some(lb) = sidereal_longitude(spk, ayanamsa, mover, b) else { break };
            for &(point, lon) in &tg {
                let fa = wrap180(la - lon);
                let fb = wrap180(lb - lon);
                if (fa > 0.0) == (fb > 0.0) || (fa - fb).abs() >= 180.0 {
                    continue;
                }
                let f = |t: Instant| sidereal_longitude(spk, ayanamsa, mover, t).map(|l| wrap180(l - lon)).unwrap_or(f64::NAN);
                let Ok(r) = find_root(f, a, b, NS_PER_SEC) else { continue };
                let kind = match point {
                    NadiPoint::Planet(g) if g == mover => HitKind::Return,
                    NadiPoint::Trine(_) => HitKind::Trine,
                    _ => HitKind::Conjunction,
                };
                out.push(NadiHit { transit: mover, point, lon, at: r.t, kind, backwards: fb < fa });
            }
            a = b;
            la = lb;
        }
    }
    out.sort_by_key(|h| h.at);
    out
}

/// How much a sensitive point bears on a life, `[0, 1]`: the natal planet's natural
/// signification for the life ([`Life::relevance`]), raised to 1 when it rules or sits in
/// one of the life's houses (whole signs from the rising sign); a trine counts 0.6 of
/// the planet; the Bhrigu Bindu 0.8 for every life, 1 for the life whose house it falls in.
pub fn relevance(natal: &Natal, point: NadiPoint, life: Life) -> f64 {
    let f = &natal.facts;
    let area = life.area();
    let planet_rel = |g: DashaLord| {
        let p = f.planet(g);
        let rules = (0..12).any(|i| f.rasi_lords[i] == g && area.group.contains(&((i + 1) as u8)));
        let sits = area.group.contains(&p.rasi_house);
        if rules || sits {
            1.0
        } else {
            life.relevance(g)
        }
    };
    match point {
        NadiPoint::Planet(g) => planet_rel(g),
        NadiPoint::Trine(g) => 0.6 * planet_rel(g),
        NadiPoint::BhriguBindu => {
            let bb = bhrigu_bindu(f.planet(DashaLord::Rahu).sidereal, f.planet(DashaLord::Moon).sidereal);
            let h = (((bb / 30.0).floor() as i64 - f.lagna_sign as i64).rem_euclid(12) + 1) as u8;
            if area.group.contains(&h) {
                1.0
            } else {
                0.8
            }
        }
    }
}

/// The lean of a mover's passage for a life, `[-1, 1]`: half its nature (Jupiter
/// expands, +0.6; Saturn tests, −0.4; Rahu unsettles, −0.2; on a return Jupiter +0.8 and
/// Saturn −0.6, the twelve- and thirty-year turning points), half its own standing for
/// the life in the birth chart (the forecast's lord reading, `2q − 1`).
pub fn polarity(natal: &Natal, mover: DashaLord, kind: HitKind, life: Life) -> f64 {
    let nature = match (mover, kind) {
        (DashaLord::Jupiter, HitKind::Return) => 0.8,
        (DashaLord::Saturn, HitKind::Return) => -0.6,
        (DashaLord::Jupiter, _) => 0.6,
        (DashaLord::Saturn, _) => -0.4,
        _ => -0.2,
    };
    let q = lord_score(natal, life, mover).0;
    0.5 * nature + 0.5 * (2.0 * q - 1.0)
}

/// The passages of a birth chart prepared once, so each instant reads cheaply.
#[derive(Clone, Debug)]
pub struct NadiTable {
    /// Per mover (Jupiter, Saturn, Rahu): `(point, lon, kind, weight per life)`, the
    /// weight being relevance × polarity.
    rows: [Vec<(NadiPoint, f64, [f64; 9])>; 3],
}

impl NadiTable {
    /// Prepare the table for `natal`.
    pub fn new(natal: &Natal) -> Self {
        let rows = std::array::from_fn(|m| {
            let mover = MOVERS[m];
            targets(natal, mover)
                .into_iter()
                .map(|(point, lon)| {
                    let kind = match point {
                        NadiPoint::Planet(g) if g == mover => HitKind::Return,
                        NadiPoint::Trine(_) => HitKind::Trine,
                        _ => HitKind::Conjunction,
                    };
                    let w = std::array::from_fn(|li| {
                        let life = Life::ALL[li];
                        relevance(natal, point, life) * polarity(natal, mover, kind, life)
                    });
                    (point, lon, w)
                })
                .collect()
        });
        NadiTable { rows }
    }

    /// The reading of every life (in [`Life::ALL`] order) with Jupiter, Saturn and Rahu at
    /// `movers` (sidereal degrees): the sum over points within [`ORB_DEG`] of
    /// `(1 − distance / orb) × weight`. Zero when nothing is in orb.
    pub fn values(&self, movers: [f64; 3]) -> [f64; 9] {
        let mut out = [0.0; 9];
        for (m, rows) in self.rows.iter().enumerate() {
            for (_, lon, w) in rows {
                let d = wrap180(movers[m] - lon).abs();
                if d >= ORB_DEG {
                    continue;
                }
                let k = 1.0 - d / ORB_DEG;
                for li in 0..9 {
                    out[li] += k * w[li];
                }
            }
        }
        out
    }

    /// The points in orb with Jupiter, Saturn and Rahu at `movers`, strongest bearing on
    /// `life` first: `(mover, point, closeness 0–1, weight for the life)`.
    pub fn in_orb(&self, movers: [f64; 3], life: Life) -> Vec<(DashaLord, NadiPoint, f64, f64)> {
        let li = Life::ALL.iter().position(|l| *l == life).unwrap_or(0);
        let mut v = Vec::new();
        for (m, rows) in self.rows.iter().enumerate() {
            for (point, lon, w) in rows {
                let d = wrap180(movers[m] - lon).abs();
                if d < ORB_DEG {
                    v.push((MOVERS[m], *point, 1.0 - d / ORB_DEG, w[li]));
                }
            }
        }
        v.sort_by(|a, b| (b.2 * b.3.abs()).partial_cmp(&(a.2 * a.3.abs())).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;

    fn setup() -> Option<(Spk, Natal)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).ok()?;
        Some((spk, natal))
    }

    #[test]
    fn the_bindu_is_the_midpoint_from_rahu_to_the_moon() {
        assert!((bhrigu_bindu(40.0, 100.0) - 70.0).abs() < 1e-12);
        // Across Aries 0°: Rahu 300°, Moon 10° → the arc is 70°, midpoint 335°.
        assert!((bhrigu_bindu(300.0, 10.0) - 335.0).abs() < 1e-12);
        // Moon behind Rahu: the arc from Rahu runs the long way round (Rahu 100, Moon 40:
        // 300° of arc, midpoint 250°).
        assert!((bhrigu_bindu(100.0, 40.0) - 250.0).abs() < 1e-12);
        // Equidistant from both, on the forward arc from Rahu.
        for (r, m) in [(12.3, 211.9), (355.0, 3.0), (180.0, 179.0)] {
            let b = bhrigu_bindu(r, m);
            let fwd_rb = (b - r).rem_euclid(360.0);
            let fwd_bm = (m - b).rem_euclid(360.0);
            assert!((fwd_rb - fwd_bm).abs() < 1e-9, "{r} {m} {b}");
        }
        // The 1990 Kochi chart, from its own positions.
        let Some((_, natal)) = setup() else { return };
        let r = natal.facts.planet(DashaLord::Rahu).sidereal;
        let m = natal.facts.planet(DashaLord::Moon).sidereal;
        let b = bhrigu_bindu(r, m);
        eprintln!("Rahu {r:.3}, Moon {m:.3}, Bhrigu Bindu {b:.3}");
        assert!((wrap180(b - r) - wrap180(m - b)).abs() < 1e-9);
    }

    #[test]
    fn every_hit_is_an_exact_crossing_bracketed_in_time() {
        let Some((spk, natal)) = setup() else { return };
        let from = Utc::new(2026, 1, 1, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let to = Utc::new(2031, 1, 1, 0, 0, 0, 0).unwrap().to_instant().unwrap();
        let t = std::time::Instant::now();
        let h = hits(&spk, Ayanamsa::Lahiri, &natal, from, to);
        eprintln!("{} hits in {:?}", h.len(), t.elapsed());
        for x in &h {
            eprintln!("{:?} {}", Utc::from_instant(x.at), x.expert());
        }
        // Jupiter covers some 150° in five years: with 25 points around the circle it
        // crosses several; Saturn and Rahu add more.
        assert!(h.iter().filter(|x| x.transit == DashaLord::Jupiter).count() >= 3);
        assert!(h.iter().any(|x| x.transit == DashaLord::Saturn));
        assert!(h.iter().any(|x| x.transit == DashaLord::Rahu));
        for x in &h {
            assert!(x.at >= from && x.at <= to);
            // At the instant the mover stands on the point, and a day either side it
            // stands on opposite sides of it (the crossing is bracketed).
            let l = sidereal_longitude(&spk, Ayanamsa::Lahiri, x.transit, x.at).unwrap();
            assert!(wrap180(l - x.lon).abs() < 1e-3, "{x:?} {l}");
            let before = sidereal_longitude(&spk, Ayanamsa::Lahiri, x.transit, x.at.add_ns(-NS_PER_DAY)).unwrap();
            let after = sidereal_longitude(&spk, Ayanamsa::Lahiri, x.transit, x.at.add_ns(NS_PER_DAY)).unwrap();
            assert!(wrap180(before - x.lon) * wrap180(after - x.lon) < 0.0, "{x:?}");
        }
        assert!(h.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn jupiter_returns_about_every_twelve_years() {
        let Some((spk, natal)) = setup() else { return };
        let from = natal.birth.add_ns(300 * NS_PER_DAY);
        let to = natal.birth.add_ns(37 * 365 * NS_PER_DAY);
        let h: Vec<NadiHit> = hits(&spk, Ayanamsa::Lahiri, &natal, from, to).into_iter().filter(|x| x.kind == HitKind::Return && x.transit == DashaLord::Jupiter).collect();
        let years: Vec<f64> = h.iter().map(|x| x.at.ns_since(natal.birth) as f64 / (365.25 * NS_PER_DAY as f64)).collect();
        eprintln!("Jupiter returns at ages {years:?}");
        assert!(years.iter().any(|y| (y - 11.9).abs() < 1.2));
        assert!(years.iter().any(|y| (y - 23.7).abs() < 1.5));
        assert!(years.iter().any(|y| (y - 35.6).abs() < 1.5));
    }

    #[test]
    fn the_reading_is_zero_out_of_orb_and_peaks_on_the_point() {
        let Some((_, natal)) = setup() else { return };
        let t = NadiTable::new(&natal);
        let venus = natal.facts.planet(DashaLord::Venus).sidereal;
        // Park Saturn and Rahu far from every point by searching for a clear spot.
        let clear = (0..3600).map(|k| k as f64 * 0.1).find(|x| t.values([*x, *x, *x]).iter().all(|v| *v == 0.0)).expect("a clear degree");
        let at = |d: f64| t.values([venus + d, clear, clear])[0];
        let on = at(0.0);
        assert!(on != 0.0);
        if t.in_orb([venus, clear, clear], Life::Love).len() == 1 && t.in_orb([venus + 1.5, clear, clear], Life::Love).len() == 1 {
            // Linear in the distance: half the orb, half the reading.
            assert!((at(1.5) - 0.5 * on).abs() < 1e-9);
        }
        assert_eq!(t.in_orb([clear, clear, clear], Life::Love).len(), 0);
    }
}
