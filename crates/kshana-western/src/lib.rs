//! Western astrology on top of exact longitudes.

#![forbid(unsafe_code)]

/// The classical and common modern aspects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Aspect {
    Conjunction,
    Opposition,
    Trine,
    Square,
    Sextile,
    Quincunx,
    Semisextile,
    Semisquare,
    Sesquiquadrate,
    Quintile,
    Biquintile,
}

impl Aspect {
    /// All aspects, major first.
    pub const ALL: [Aspect; 11] = [
        Aspect::Conjunction,
        Aspect::Opposition,
        Aspect::Trine,
        Aspect::Square,
        Aspect::Sextile,
        Aspect::Quincunx,
        Aspect::Semisextile,
        Aspect::Semisquare,
        Aspect::Sesquiquadrate,
        Aspect::Quintile,
        Aspect::Biquintile,
    ];

    /// Name in words.
    pub fn name(self) -> &'static str {
        match self {
            Aspect::Conjunction => "conjunction",
            Aspect::Opposition => "opposition",
            Aspect::Trine => "trine",
            Aspect::Square => "square",
            Aspect::Sextile => "sextile",
            Aspect::Quincunx => "quincunx",
            Aspect::Semisextile => "semisextile",
            Aspect::Semisquare => "semisquare",
            Aspect::Sesquiquadrate => "sesquiquadrate",
            Aspect::Quintile => "quintile",
            Aspect::Biquintile => "biquintile",
        }
    }

    /// The name as Malayalam and Hindi chart pages write it, `(Malayalam, Hindi)`: the
    /// Indian names where the angle has one (യോഗം / युति for the conjunction, കേന്ദ്രം /
    /// केंद्र for the square), the fraction of the circle for the minor aspects.
    pub const fn native_names(self) -> (&'static str, &'static str) {
        match self {
            Aspect::Conjunction => ("യോഗം", "युति"),
            Aspect::Opposition => ("സമസപ്തമം", "प्रतियुति"),
            Aspect::Trine => ("ത്രികോണം", "त्रिकोण"),
            Aspect::Square => ("കേന്ദ്രം", "केंद्र"),
            Aspect::Sextile => ("ഷഷ്ഠാംശം", "षष्ठांश"),
            Aspect::Quincunx => ("ക്വിൻകങ്ക്സ് (150°)", "क्विनकंक्स (150°)"),
            Aspect::Semisextile => ("അർധ ഷഷ്ഠാംശം", "अर्ध षष्ठांश"),
            Aspect::Semisquare => ("അർധ കേന്ദ്രം", "अर्ध केंद्र"),
            Aspect::Sesquiquadrate => ("സാർധ കേന്ദ്രം", "सार्ध केंद्र"),
            Aspect::Quintile => ("പഞ്ചമാംശം", "पंचमांश"),
            Aspect::Biquintile => ("ദ്വിപഞ്ചമാംശം", "द्विपंचमांश"),
        }
    }

    /// Exact angle in degrees.
    pub const fn angle(self) -> f64 {
        match self {
            Aspect::Conjunction => 0.0,
            Aspect::Opposition => 180.0,
            Aspect::Trine => 120.0,
            Aspect::Square => 90.0,
            Aspect::Sextile => 60.0,
            Aspect::Quincunx => 150.0,
            Aspect::Semisextile => 30.0,
            Aspect::Semisquare => 45.0,
            Aspect::Sesquiquadrate => 135.0,
            Aspect::Quintile => 72.0,
            Aspect::Biquintile => 144.0,
        }
    }

    /// True for the Ptolemaic five.
    pub const fn is_major(self) -> bool {
        matches!(
            self,
            Aspect::Conjunction
                | Aspect::Opposition
                | Aspect::Trine
                | Aspect::Square
                | Aspect::Sextile
        )
    }
}

/// A detected aspect between two bodies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectHit {
    /// Which aspect.
    pub aspect: Aspect,
    /// Signed orb in degrees: positive when the separation exceeds the exact angle.
    pub orb: f64,
    /// True when the faster body is closing on exactitude (needs speeds).
    pub applying: Option<bool>,
}

/// Smallest separation between two longitudes, `[0, 180]`.
pub fn separation(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

/// Find the tightest aspect within `max_orb` degrees between longitudes `a` and `b`.
/// Pass daily speeds to learn whether it is applying.
pub fn aspect_between(
    a: f64,
    b: f64,
    speed_a: Option<f64>,
    speed_b: Option<f64>,
    max_orb: f64,
) -> Option<AspectHit> {
    let sep = separation(a, b);
    let mut best: Option<AspectHit> = None;
    for asp in Aspect::ALL {
        let orb = sep - asp.angle();
        if orb.abs() <= max_orb && best.map_or(true, |h| orb.abs() < h.orb.abs()) {
            let applying = match (speed_a, speed_b) {
                (Some(sa), Some(sb)) => {
                    // separation one day later
                    let next = separation(a + sa, b + sb);
                    Some((next - asp.angle()).abs() < orb.abs())
                }
                _ => None,
            };
            best = Some(AspectHit {
                aspect: asp,
                orb,
                applying,
            });
        }
    }
    best
}

/// The twelve signs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Sign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

impl Sign {
    /// Sign containing a longitude.
    pub fn of(lon: f64) -> Sign {
        const ALL: [Sign; 12] = [
            Sign::Aries,
            Sign::Taurus,
            Sign::Gemini,
            Sign::Cancer,
            Sign::Leo,
            Sign::Virgo,
            Sign::Libra,
            Sign::Scorpio,
            Sign::Sagittarius,
            Sign::Capricorn,
            Sign::Aquarius,
            Sign::Pisces,
        ];
        ALL[((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)]
    }

    /// Traditional domicile ruler.
    pub const fn traditional_ruler(self) -> Planet {
        match self {
            Sign::Aries | Sign::Scorpio => Planet::Mars,
            Sign::Taurus | Sign::Libra => Planet::Venus,
            Sign::Gemini | Sign::Virgo => Planet::Mercury,
            Sign::Cancer => Planet::Moon,
            Sign::Leo => Planet::Sun,
            Sign::Sagittarius | Sign::Pisces => Planet::Jupiter,
            Sign::Capricorn | Sign::Aquarius => Planet::Saturn,
        }
    }

    /// Planet exalted here, if any (traditional seven).
    pub const fn exaltation(self) -> Option<Planet> {
        match self {
            Sign::Aries => Some(Planet::Sun),
            Sign::Taurus => Some(Planet::Moon),
            Sign::Cancer => Some(Planet::Jupiter),
            Sign::Virgo => Some(Planet::Mercury),
            Sign::Libra => Some(Planet::Saturn),
            Sign::Capricorn => Some(Planet::Mars),
            Sign::Pisces => Some(Planet::Venus),
            _ => None,
        }
    }
}

/// The traditional seven plus the moderns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Planet {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
}

/// Day chart if the Sun is above the horizon. `sun_altitude` in degrees is the honest
/// test; the longitude shortcut (Sun in houses 7–12) is wrong near the horizon.
pub fn is_day_chart(sun_altitude_deg: f64) -> bool {
    sun_altitude_deg > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_wraps() {
        assert_eq!(separation(10.0, 350.0), 20.0);
        assert_eq!(separation(0.0, 180.0), 180.0);
    }

    #[test]
    fn finds_the_tightest_aspect() {
        let h = aspect_between(135.2, 1.4, Some(0.6), Some(0.0), 8.0).unwrap();
        assert_eq!(h.aspect, Aspect::Sesquiquadrate);
        assert!((h.orb - (-1.2)).abs() < 1e-9);
        let h = aspect_between(100.0, 10.5, Some(0.3), Some(0.0), 3.0).unwrap();
        assert_eq!(h.aspect, Aspect::Square);
        assert_eq!(h.applying, Some(true));
        assert!(aspect_between(100.0, 50.0, None, None, 3.0).is_none());
    }

    #[test]
    fn signs_and_dignities() {
        assert_eq!(Sign::of(202.1), Sign::Libra);
        assert_eq!(Sign::of(359.99), Sign::Pisces);
        assert_eq!(Sign::Libra.traditional_ruler(), Planet::Venus);
        assert_eq!(Sign::Libra.exaltation(), Some(Planet::Saturn));
    }
}
