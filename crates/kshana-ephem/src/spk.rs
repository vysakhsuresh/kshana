//! SPK segment evaluation and body chaining.

use crate::chebyshev::eval_with_derivative;
use crate::daf::{Daf, DafError, Summary};
use kshana_time::{Instant, TwoPartJd};

/// Bodies Kshana charts, with their NAIF ids as stored in DE kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Body {
    SolarSystemBarycenter,
    Sun,
    Mercury,
    Venus,
    EarthMoonBarycenter,
    Earth,
    Moon,
    MarsBarycenter,
    JupiterBarycenter,
    SaturnBarycenter,
    UranusBarycenter,
    NeptuneBarycenter,
    PlutoBarycenter,
}

impl Body {
    /// NAIF id.
    pub const fn naif(self) -> i32 {
        match self {
            Body::SolarSystemBarycenter => 0,
            Body::Sun => 10,
            Body::Mercury => 1,
            Body::Venus => 2,
            Body::EarthMoonBarycenter => 3,
            Body::Earth => 399,
            Body::Moon => 301,
            Body::MarsBarycenter => 4,
            Body::JupiterBarycenter => 5,
            Body::SaturnBarycenter => 6,
            Body::UranusBarycenter => 7,
            Body::NeptuneBarycenter => 8,
            Body::PlutoBarycenter => 9,
        }
    }
}

/// Position (km) and velocity (km/s) in the kernel's inertial frame (ICRF/J2000).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Position in km.
    pub position: [f64; 3],
    /// Velocity in km/s.
    pub velocity: [f64; 3],
}

/// Why a state could not be produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpkError {
    /// File-level problem.
    Daf(DafError),
    /// The instant is outside every matching segment.
    OutOfRange,
    /// No chain of segments links target to centre.
    NoPath,
    /// Segment type not supported.
    UnsupportedType(i32),
}

impl From<DafError> for SpkError {
    fn from(e: DafError) -> Self {
        SpkError::Daf(e)
    }
}

/// An opened SPK kernel.
pub struct Spk {
    daf: Daf,
}

impl Spk {
    /// Parse a kernel from bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SpkError> {
        Ok(Self {
            daf: Daf::parse(bytes)?,
        })
    }

    /// Open a kernel file.
    pub fn open(path: &std::path::Path) -> Result<Self, SpkError> {
        let bytes = std::fs::read(path).map_err(|_| SpkError::Daf(DafError::Truncated))?;
        Self::from_bytes(bytes)
    }

    /// Segment summaries.
    pub fn segments(&self) -> &[Summary] {
        &self.daf.summaries
    }

    /// TDB seconds past J2000 for an instant, from its two-part TDB Julian Date.
    pub fn et_of(i: Instant) -> f64 {
        let TwoPartJd { jd1, jd2 } = i.tdb_jd();
        ((jd1 - 2_451_545.0) + jd2) * 86_400.0
    }

    /// State of `target` relative to `center` at TDB seconds past J2000.
    ///
    /// Each body is followed up its chain of centres (Earth → EMB → SSB, say) until the
    /// two chains meet; the result is the difference of the two partial sums.
    pub fn state(&self, target: Body, center: Body, et: f64) -> Result<State, SpkError> {
        let (t_nodes, t_states) = self.chain(target.naif(), et)?;
        let (c_nodes, c_states) = self.chain(center.naif(), et)?;
        let (ti, ci) = t_nodes
            .iter()
            .enumerate()
            .find_map(|(ti, n)| c_nodes.iter().position(|m| m == n).map(|ci| (ti, ci)))
            .ok_or(SpkError::NoPath)?;
        let mut out = State {
            position: [0.0; 3],
            velocity: [0.0; 3],
        };
        for s in &t_states[..ti] {
            for k in 0..3 {
                out.position[k] += s.position[k];
                out.velocity[k] += s.velocity[k];
            }
        }
        for s in &c_states[..ci] {
            for k in 0..3 {
                out.position[k] -= s.position[k];
                out.velocity[k] -= s.velocity[k];
            }
        }
        Ok(out)
    }

    /// Nodes `[id, centre(id), centre(centre(id)), …]` and the state of each node
    /// relative to the next, following segments until a body has none.
    fn chain(&self, start: i32, et: f64) -> Result<(Vec<i32>, Vec<State>), SpkError> {
        let mut nodes = vec![start];
        let mut states = Vec::new();
        let mut id = start;
        for _ in 0..8 {
            let has_any = self.daf.summaries.iter().any(|s| s.target == id);
            if !has_any {
                break;
            }
            let seg = self
                .daf
                .summaries
                .iter()
                .find(|s| s.target == id && s.start_et <= et && et <= s.end_et)
                .ok_or(SpkError::OutOfRange)?;
            states.push(self.eval_segment(seg, et)?);
            id = seg.center;
            nodes.push(id);
        }
        Ok((nodes, states))
    }

    fn eval_segment(&self, seg: &Summary, et: f64) -> Result<State, SpkError> {
        match seg.data_type {
            2 | 3 => {}
            t => return Err(SpkError::UnsupportedType(t)),
        }
        let end = seg.end_addr as usize;
        let init = self.daf.double(end - 3);
        let intlen = self.daf.double(end - 2);
        let rsize = self.daf.double(end - 1) as usize;
        let nrec = self.daf.double(end) as usize;
        let mut idx = ((et - init) / intlen).floor();
        if idx < 0.0 {
            idx = 0.0;
        }
        let idx = (idx as usize).min(nrec - 1);
        let rec_addr = seg.start_addr as usize + idx * rsize;
        let rec = self.daf.doubles(rec_addr, rsize);
        let mid = rec[0];
        let radius = rec[1];
        let tau = (et - mid) / radius;
        let ncoef = if seg.data_type == 2 {
            (rsize - 2) / 3
        } else {
            (rsize - 2) / 6
        };
        let mut st = State {
            position: [0.0; 3],
            velocity: [0.0; 3],
        };
        for k in 0..3 {
            let c = &rec[2 + k * ncoef..2 + (k + 1) * ncoef];
            let (p, dp) = eval_with_derivative(c, tau);
            st.position[k] = p;
            st.velocity[k] = dp / radius;
        }
        if seg.data_type == 3 {
            for k in 0..3 {
                let c = &rec[2 + (3 + k) * ncoef..2 + (4 + k) * ncoef];
                st.velocity[k] = eval_with_derivative(c, tau).0;
            }
        }
        Ok(st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daf::tests::synthetic;

    #[test]
    fn evaluates_a_synthetic_type2_segment() {
        // x = 1 + 2τ, y = 3 + 4τ, z = 5 + 6τ over et ∈ [0, 100], so τ = (et − 50)/50
        let bytes = synthetic([&[1.0, 2.0], &[3.0, 4.0], &[5.0, 6.0]], 0.0, 100.0);
        let spk = Spk::from_bytes(bytes).unwrap();
        let st = spk
            .state(Body::Earth, Body::EarthMoonBarycenter, 75.0)
            .unwrap();
        assert!((st.position[0] - 2.0).abs() < 1e-12);
        assert!((st.position[1] - 5.0).abs() < 1e-12);
        assert!((st.position[2] - 8.0).abs() < 1e-12);
        // velocity = dcoef/radius
        assert!((st.velocity[0] - 2.0 / 50.0).abs() < 1e-12);
        assert_eq!(
            spk.state(Body::Earth, Body::EarthMoonBarycenter, 150.0)
                .unwrap_err(),
            SpkError::OutOfRange
        );
        assert_eq!(
            spk.state(Body::Moon, Body::EarthMoonBarycenter, 50.0)
                .unwrap_err(),
            SpkError::NoPath
        );
    }

    #[test]
    fn et_of_j2000_is_zero_to_tdb_tolerance() {
        let et = Spk::et_of(Instant::j2000());
        // TDB − TT at J2000 is about −0.07 ms
        assert!(et.abs() < 0.002, "{et}");
    }

    fn kernel(name: &str) -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data")
            .join(name);
        if !p.exists() {
            eprintln!("skipped: data/{name} not present (scripts/fetch-ephemeris.sh)");
            return None;
        }
        Some(Spk::open(&p).unwrap())
    }

    /// Angle in milliarcseconds between two position vectors.
    fn mas_between(a: [f64; 3], b: [f64; 3]) -> f64 {
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let c = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();
        let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        c.atan2(d).to_degrees() * 3_600_000.0
    }

    /// DE440s (the app's kernel, JPL `planets/bsp/de440s.bsp`, SHA-256 c1c7feea…6a49f2)
    /// read through the same segment code as DE421 gives the same sky. Both kernels carry
    /// one Type 2 segment per body (DE440s: 1849-12-26 … 2150-01-22, no 499 Mars body
    /// segment, which Kshana does not use). Measured geocentric differences over 1990
    /// (the two fits differ, not the reader; sampled every 5 days): Sun 0.28 mas, Moon
    /// 0.49 mas, Mars 0.30 mas, Saturn 0.73 mas, Jupiter 18 mas (DE440 adds Juno ranging),
    /// all far under the 0.1″ the charts print.
    #[test]
    fn de440s_matches_de421_and_spans_1849_to_2150() {
        let (Some(old), Some(new)) = (kernel("de421.bsp"), kernel("de440s.bsp")) else {
            return;
        };
        let year = 365.25 * 86_400.0;
        let et0 = -10.0 * year; // 1990.0
        let mut worst = [0.0f64; 5];
        let bodies = [
            Body::Sun,
            Body::Moon,
            Body::MarsBarycenter,
            Body::JupiterBarycenter,
            Body::SaturnBarycenter,
        ];
        for k in 0..73 {
            let et = et0 + k as f64 * 5.0 * 86_400.0;
            for (i, b) in bodies.iter().enumerate() {
                let a = old.state(*b, Body::Earth, et).unwrap().position;
                let n = new.state(*b, Body::Earth, et).unwrap().position;
                worst[i] = worst[i].max(mas_between(a, n));
            }
        }
        eprintln!("DE421 vs DE440s over 1990, mas: Sun {:.3} Moon {:.3} Mars {:.3} Jupiter {:.3} Saturn {:.3}",
            worst[0], worst[1], worst[2], worst[3], worst[4]);
        assert!(
            worst[0] < 0.5 && worst[1] < 1.0 && worst[2] < 1.0,
            "{worst:?}"
        );
        assert!(worst[3] < 25.0 && worst[4] < 2.0, "{worst:?}");

        // The span DE421 lacks: an 1850 birth and a 2149 forecast evaluate in DE440s.
        for y in [1850.0, 1899.0, 2060.0, 2149.0] {
            let et = (y - 2000.0) * year;
            assert!(
                old.state(Body::Moon, Body::Earth, et).is_err() || (1900.0..2053.0).contains(&y)
            );
            let st = new.state(Body::Sun, Body::Earth, et).unwrap();
            let r = (st.position.iter().map(|x| x * x).sum::<f64>()).sqrt() / 149_597_870.7;
            assert!((0.98..1.02).contains(&r), "{y}: {r} au");
        }
        let (start, end) = new
            .segments()
            .iter()
            .fold((f64::NEG_INFINITY, f64::INFINITY), |(s, e), g| {
                (s.max(g.start_et), e.min(g.end_et))
            });
        assert_eq!((start, end), (-4_734_072_000.0, 4_735_368_000.0));
    }

    /// Real-kernel smoke test: `KSHANA_SPK=data/de440s.bsp cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_kernel_earth_sun_distance_is_one_au() {
        let path = std::env::var("KSHANA_SPK").expect("KSHANA_SPK");
        let mut p = std::path::PathBuf::from(&path);
        if p.is_relative() {
            // cargo runs tests from the crate directory; resolve against the workspace root
            p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(&path);
        }
        let spk = Spk::open(&p).unwrap_or_else(|e| panic!("open {}: {e:?}", p.display()));
        let st = spk.state(Body::Earth, Body::Sun, 0.0).unwrap();
        let r = (st.position[0].powi(2) + st.position[1].powi(2) + st.position[2].powi(2)).sqrt();
        // Earth–Sun distance on 2000-01-01 ≈ 0.9833 au (perihelion is Jan 3)
        assert!((r / 149_597_870.7 - 0.983_3).abs() < 0.001, "{r} km");
    }
}
