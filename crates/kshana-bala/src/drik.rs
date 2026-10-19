//! Drik bala: the strength a point gains or loses from the aspects falling on it
//! (BPHS 27.18–19; Raman, *Graha and Bhava Balas*, ch. on Drik bala).
//!
//! Every graha's sputa drishti on the point is found from the arc between them
//! ([`crate::tables::sputa_drishti`]); the aspects of benefics are added, those of
//! malefics subtracted, and a quarter of the net is the Drik bala. Rahu and Ketu throw
//! no aspect in this reckoning. Raman's worked chart gives the seven values 15.86,
//! −21.73, 0.95, 15.64, −16.04, 18.47, 7.21, all within the ±60 this formula allows.

use crate::tables::sputa_drishti;
use kshana_vedic::DashaLord;

/// One aspecting graha: who, its sidereal longitude, and whether it counts as a benefic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aspector {
    /// The graha.
    pub graha: DashaLord,
    /// Sidereal longitude, degrees.
    pub sidereal: f64,
    /// Natural benefic in this chart (Mercury and the Moon are conditional).
    pub benefic: bool,
}

/// The aspects on a point: `(benefic sum, malefic sum)` in virupas. A graha does not
/// aspect itself, so pass `exclude` for a graha's own longitude.
pub fn drishti_pinda(point: f64, aspectors: &[Aspector], exclude: Option<DashaLord>) -> (f64, f64) {
    let mut benefic = 0.0;
    let mut malefic = 0.0;
    for a in aspectors {
        if Some(a.graha) == exclude {
            continue;
        }
        let d = (point - a.sidereal).rem_euclid(360.0);
        let v = sputa_drishti(a.graha, d);
        if a.benefic {
            benefic += v;
        } else {
            malefic += v;
        }
    }
    (benefic, malefic)
}

/// Drik bala of a point: a quarter of the benefic aspects less a quarter of the malefic.
pub fn drik_bala(point: f64, aspectors: &[Aspector], exclude: Option<DashaLord>) -> f64 {
    let (b, m) = drishti_pinda(point, aspectors, exclude);
    (b - m) / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_of_the_net_aspect() {
        // Jupiter (benefic) exactly opposite: 60; Saturn (malefic) 90° behind: 45
        let asp = [
            Aspector {
                graha: DashaLord::Jupiter,
                sidereal: 0.0,
                benefic: true,
            },
            Aspector {
                graha: DashaLord::Saturn,
                sidereal: 90.0,
                benefic: false,
            },
            Aspector {
                graha: DashaLord::Sun,
                sidereal: 180.0,
                benefic: false,
            },
        ];
        let (b, m) = drishti_pinda(180.0, &asp, Some(DashaLord::Sun));
        assert_eq!(b, 60.0);
        assert_eq!(m, 45.0);
        assert!((drik_bala(180.0, &asp, Some(DashaLord::Sun)) - 3.75).abs() < 1e-12);
    }
}
