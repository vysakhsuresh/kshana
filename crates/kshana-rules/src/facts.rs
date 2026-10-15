//! Facts derived from a chart. Sidereal, with the chart's own cusps.

use kshana_chart::Chart;
use kshana_vedic::{kp_lords, sign_lord, DashaLord};

/// One of the nine grahas as the rules see it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetFact {
    /// Which graha.
    pub graha: DashaLord,
    /// Sidereal longitude, degrees.
    pub sidereal: f64,
    /// House occupied (1–12) by the chart's cusps.
    pub house: u8,
    /// Star (nakshatra) lord.
    pub star_lord: DashaLord,
    /// Sub lord.
    pub sub_lord: DashaLord,
    /// Houses owned (by sign on cusp). Empty for Rahu and Ketu.
    pub owns: Vec<u8>,
    /// Retrograde.
    pub retrograde: bool,
}

/// Everything the rules may look at.
#[derive(Clone, Debug, PartialEq)]
pub struct Facts {
    /// The nine grahas: Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu.
    pub planets: Vec<PlanetFact>,
    /// Sidereal cusps, index 0 = house 1.
    pub cusps: [f64; 12],
    /// Sub-lord of each cusp.
    pub cusp_sub_lords: [DashaLord; 12],
    /// Star-lord of each cusp.
    pub cusp_star_lords: [DashaLord; 12],
    /// Lord (sign ruler) of each house.
    pub house_lords: [DashaLord; 12],
}

fn house_of(lon: f64, cusps: &[f64; 12]) -> u8 {
    for i in 0..12 {
        let a = cusps[i];
        let b = cusps[(i + 1) % 12];
        let span = (b - a).rem_euclid(360.0);
        let off = (lon - a).rem_euclid(360.0);
        if off < span {
            return (i + 1) as u8;
        }
    }
    12
}

impl Facts {
    /// Derive the facts from a chart.
    pub fn from_chart(chart: &Chart) -> Self {
        let cusps = chart.cusps_sidereal;
        let house_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((cusps[i] / 30.0).floor() as usize));
        let owns_of = |g: DashaLord| -> Vec<u8> {
            (0..12)
                .filter(|&i| house_lords[i] == g)
                .map(|i| (i + 1) as u8)
                .collect()
        };
        let mut planets = Vec::with_capacity(9);
        let mut add = |graha: DashaLord, sidereal: f64, retrograde: bool| {
            let k = kp_lords(sidereal);
            planets.push(PlanetFact {
                graha,
                sidereal,
                house: house_of(sidereal, &cusps),
                star_lord: k.star,
                sub_lord: k.sub,
                owns: match graha {
                    DashaLord::Rahu | DashaLord::Ketu => Vec::new(),
                    g => owns_of(g),
                },
                retrograde,
            });
        };
        let find = |name: &str| chart.bodies.iter().find(|b| b.name == name).expect("body");
        for (name, graha) in [
            ("Sun", DashaLord::Sun),
            ("Moon", DashaLord::Moon),
            ("Mars", DashaLord::Mars),
            ("Mercury", DashaLord::Mercury),
            ("Jupiter", DashaLord::Jupiter),
            ("Venus", DashaLord::Venus),
            ("Saturn", DashaLord::Saturn),
        ] {
            let b = find(name);
            add(graha, b.sidereal, b.speed < 0.0);
        }
        let rahu = chart.true_node.1;
        add(DashaLord::Rahu, rahu, true);
        add(DashaLord::Ketu, (rahu + 180.0).rem_euclid(360.0), true);

        let cusp_sub_lords: [DashaLord; 12] = std::array::from_fn(|i| kp_lords(cusps[i]).sub);
        let cusp_star_lords: [DashaLord; 12] = std::array::from_fn(|i| kp_lords(cusps[i]).star);
        Self {
            planets,
            cusps,
            cusp_sub_lords,
            cusp_star_lords,
            house_lords,
        }
    }

    /// The fact record for a graha.
    pub fn planet(&self, g: DashaLord) -> &PlanetFact {
        self.planets
            .iter()
            .find(|p| p.graha == g)
            .expect("all nine present")
    }

    /// Grahas occupying a house.
    pub fn occupants(&self, house: u8) -> Vec<DashaLord> {
        self.planets
            .iter()
            .filter(|p| p.house == house)
            .map(|p| p.graha)
            .collect()
    }
}
