//! The annual sky as the Tajika rules read it: sidereal longitudes of the seven grahas,
//! the lagna and the twelve cusps, lifted out of a [`Chart`].

use crate::tables::{graha_name, sign_index, speed_rank, SEVEN};
use kshana_chart::Chart;
use kshana_vedic::DashaLord;

/// What every Tajika rule reads from the annual chart. Everything is sidereal.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnualSky {
    /// Sidereal ascendant, degrees.
    pub lagna: f64,
    /// Sidereal cusps, index 0 = house 1.
    pub cusps: [f64; 12],
    /// Sidereal longitudes of the seven grahas in [`SEVEN`] order.
    pub longitudes: [f64; 7],
    /// Daily motion of the seven in degrees per day (negative when retrograde).
    pub speeds: [f64; 7],
}

impl AnnualSky {
    /// Lift the seven grahas, the lagna and the cusps out of a chart.
    pub fn from_chart(chart: &Chart) -> Self {
        let mut longitudes = [0.0; 7];
        let mut speeds = [0.0; 7];
        for (i, g) in SEVEN.iter().enumerate() {
            if let Some(b) = chart.bodies.iter().find(|b| b.name == graha_name(*g)) {
                longitudes[i] = b.sidereal;
                speeds[i] = b.speed;
            }
        }
        Self {
            lagna: chart.cusps_sidereal[0],
            cusps: chart.cusps_sidereal,
            longitudes,
            speeds,
        }
    }

    /// Sidereal longitude of a graha, `None` for the nodes.
    pub fn longitude(&self, g: DashaLord) -> Option<f64> {
        speed_rank(g).map(|i| self.longitudes[i])
    }

    /// Daily motion of a graha, `None` for the nodes.
    pub fn speed(&self, g: DashaLord) -> Option<f64> {
        speed_rank(g).map(|i| self.speeds[i])
    }

    /// Sign index 0–11 of a graha, `None` for the nodes.
    pub fn sign(&self, g: DashaLord) -> Option<usize> {
        self.longitude(g).map(sign_index)
    }

    /// Sign index of the lagna.
    pub fn lagna_sign(&self) -> usize {
        sign_index(self.lagna)
    }

    /// House 1–12 of a sidereal longitude by this chart's cusps.
    pub fn house(&self, lon: f64) -> u8 {
        house_of(lon, &self.cusps)
    }

    /// House 1–12 of a sign counted from the lagna sign (whole-sign reckoning, which is
    /// how the Tajika yogas place planets).
    pub fn sign_house(&self, sign: usize) -> u8 {
        ((sign % 12 + 12 - self.lagna_sign()) % 12 + 1) as u8
    }
}

/// House 1–12 of a longitude by twelve cusps in the same zodiac.
pub fn house_of(lon: f64, cusps: &[f64; 12]) -> u8 {
    for (i, &a) in cusps.iter().enumerate() {
        let b = cusps[(i + 1) % 12];
        let span = (b - a).rem_euclid(360.0);
        let off = (lon - a).rem_euclid(360.0);
        if off < span {
            return (i + 1) as u8;
        }
    }
    12
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Equal houses from a given lagna, planets as given (SEVEN order).
    pub(crate) fn synthetic(lagna: f64, longitudes: [f64; 7]) -> AnnualSky {
        AnnualSky {
            lagna,
            cusps: std::array::from_fn(|i| (lagna + 30.0 * i as f64).rem_euclid(360.0)),
            longitudes,
            speeds: [13.0, 1.4, 1.2, 1.0, 0.5, 0.08, 0.03],
        }
    }

    #[test]
    fn houses_follow_the_cusps() {
        let sky = synthetic(100.0, [0.0; 7]);
        assert_eq!(sky.house(100.0), 1);
        assert_eq!(sky.house(129.9), 1);
        assert_eq!(sky.house(130.0), 2);
        assert_eq!(sky.house(99.9), 12);
        assert_eq!(sky.sign_house(3), 1);
        assert_eq!(sky.sign_house(2), 12);
    }
}
