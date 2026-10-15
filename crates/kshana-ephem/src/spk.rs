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
