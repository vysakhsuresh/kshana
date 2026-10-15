//! The five limbs: tithi, nakshatra, yoga, karana (and vara, which is a date).
//! All inputs are sidereal longitudes in degrees unless stated.

/// Names of the 27 nakshatras in order from Ashwini.
pub const NAKSHATRA_NAMES: [&str; 27] = [
    "Ashwini",
    "Bharani",
    "Krittika",
    "Rohini",
    "Mrigashira",
    "Ardra",
    "Punarvasu",
    "Pushya",
    "Ashlesha",
    "Magha",
    "Purva Phalguni",
    "Uttara Phalguni",
    "Hasta",
    "Chitra",
    "Swati",
    "Vishakha",
    "Anuradha",
    "Jyeshtha",
    "Mula",
    "Purva Ashadha",
    "Uttara Ashadha",
    "Shravana",
    "Dhanishta",
    "Shatabhisha",
    "Purva Bhadrapada",
    "Uttara Bhadrapada",
    "Revati",
];

/// Names of the 30 tithis, Shukla 1–15 then Krishna 1–15.
pub const TITHI_NAMES: [&str; 15] = [
    "Pratipada",
    "Dwitiya",
    "Tritiya",
    "Chaturthi",
    "Panchami",
    "Shashthi",
    "Saptami",
    "Ashtami",
    "Navami",
    "Dashami",
    "Ekadashi",
    "Dwadashi",
    "Trayodashi",
    "Chaturdashi",
    "Purnima/Amavasya",
];

/// Names of the 27 yogas.
pub const YOGA_NAMES: [&str; 27] = [
    "Vishkambha",
    "Priti",
    "Ayushman",
    "Saubhagya",
    "Shobhana",
    "Atiganda",
    "Sukarma",
    "Dhriti",
    "Shula",
    "Ganda",
    "Vriddhi",
    "Dhruva",
    "Vyaghata",
    "Harshana",
    "Vajra",
    "Siddhi",
    "Vyatipata",
    "Variyan",
    "Parigha",
    "Shiva",
    "Siddha",
    "Sadhya",
    "Shubha",
    "Shukla",
    "Brahma",
    "Indra",
    "Vaidhriti",
];

const MOVABLE_KARANAS: [&str; 7] = [
    "Bava", "Balava", "Kaulava", "Taitila", "Gara", "Vanija", "Vishti",
];

/// Span of one nakshatra in degrees: 13°20′.
pub const NAKSHATRA_SPAN: f64 = 360.0 / 27.0;

/// A nakshatra position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nakshatra {
    /// 0-based index, 0 = Ashwini.
    pub index: usize,
    /// Pada 1–4.
    pub pada: u8,
    /// Fraction of the nakshatra already traversed, `[0, 1)`.
    pub fraction: f64,
}

impl Nakshatra {
    /// Name of this nakshatra.
    pub fn name(&self) -> &'static str {
        NAKSHATRA_NAMES[self.index]
    }
}

/// Nakshatra of a sidereal longitude.
pub fn nakshatra(sidereal_lon: f64) -> Nakshatra {
    let l = sidereal_lon.rem_euclid(360.0);
    let pos = l / NAKSHATRA_SPAN;
    let index = (pos.floor() as usize).min(26);
    let fraction = pos - index as f64;
    let pada = ((fraction * 4.0).floor() as u8 + 1).min(4);
    Nakshatra {
        index,
        pada,
        fraction,
    }
}

/// A tithi position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tithi {
    /// 1–30; 1–15 Shukla paksha, 16–30 Krishna paksha.
    pub number: u8,
    /// Fraction of the tithi elapsed, `[0, 1)`.
    pub fraction: f64,
}

impl Tithi {
    /// True in the bright half.
    pub fn is_shukla(&self) -> bool {
        self.number <= 15
    }
    /// Name within the paksha.
    pub fn name(&self) -> &'static str {
        TITHI_NAMES[((self.number - 1) % 15) as usize]
    }
}

/// Tithi from the Moon − Sun elongation (degrees, any zodiac: the difference cancels
/// the ayanamsa).
pub fn tithi(moon_lon: f64, sun_lon: f64) -> Tithi {
    let e = (moon_lon - sun_lon).rem_euclid(360.0) / 12.0;
    let n = (e.floor() as u8).min(29);
    Tithi {
        number: n + 1,
        fraction: e - n as f64,
    }
}

/// A yoga position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Yoga {
    /// 0-based index, 0 = Vishkambha.
    pub index: usize,
    /// Fraction elapsed.
    pub fraction: f64,
}

impl Yoga {
    /// Name of this yoga.
    pub fn name(&self) -> &'static str {
        YOGA_NAMES[self.index]
    }
}

/// Yoga from sidereal Sun and Moon longitudes: their sum in units of 13°20′.
pub fn yoga(moon_sid: f64, sun_sid: f64) -> Yoga {
    let s = (moon_sid + sun_sid).rem_euclid(360.0) / NAKSHATRA_SPAN;
    let index = (s.floor() as usize).min(26);
    Yoga {
        index,
        fraction: s - index as f64,
    }
}

/// A karana (half-tithi).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Karana {
    /// 0–59 within the lunar month.
    pub ordinal: u8,
    /// Its name; four fixed karanas sit at the month's ends, seven movable ones cycle.
    pub name: &'static str,
}

/// Karana from the Moon − Sun elongation.
pub fn karana(moon_lon: f64, sun_lon: f64) -> Karana {
    let e = (moon_lon - sun_lon).rem_euclid(360.0) / 6.0;
    let k = (e.floor() as u8).min(59);
    let name = match k {
        0 => "Kimstughna",
        57 => "Shakuni",
        58 => "Chatushpada",
        59 => "Naga",
        n => MOVABLE_KARANAS[((n - 1) % 7) as usize],
    };
    Karana { ordinal: k, name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mula_pada_one() {
        let n = nakshatra(243.2);
        assert_eq!(n.name(), "Mula");
        assert_eq!(n.pada, 1);
    }

    #[test]
    fn boundaries_are_exact() {
        assert_eq!(nakshatra(0.0).index, 0);
        assert_eq!(nakshatra(NAKSHATRA_SPAN).index, 1);
        assert_eq!(nakshatra(359.999_999).index, 26);
        assert_eq!(nakshatra(360.0).index, 0);
    }

    #[test]
    fn shukla_shashthi() {
        let t = tithi(267.4, 202.1);
        assert_eq!(t.number, 6);
        assert!(t.is_shukla());
        assert_eq!(t.name(), "Shashthi");
    }

    #[test]
    fn amavasya_and_purnima() {
        assert_eq!(tithi(10.0, 10.0).number, 1);
        assert_eq!(tithi(359.9, 0.0).number, 30);
        assert_eq!(tithi(180.0, 0.0).number, 16);
        assert_eq!(tithi(179.9, 0.0).number, 15);
    }

    #[test]
    fn karana_cycle() {
        assert_eq!(karana(0.0, 0.0).name, "Kimstughna");
        assert_eq!(karana(6.0, 0.0).name, "Bava");
        assert_eq!(karana(42.0, 0.0).name, "Vishti");
        assert_eq!(karana(48.0, 0.0).name, "Bava");
        assert_eq!(karana(54.0, 0.0).name, "Balava");
        assert_eq!(karana(354.0, 0.0).name, "Naga");
    }
}
