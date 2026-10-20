//! Kala bala: temporal strength (BPHS ch. 27, slokas 8–20; Raman, *Graha and Bhava
//! Balas*, ch. on Kala bala). Nine parts: Nathonnata, Paksha, Tribhaga, the four lords
//! of time (Abda, Masa, Vara, Hora), Ayana and Yuddha.

use crate::tables::{disc_diameter, lord_by_name, weekday_lord, TARA};
use kshana_chart::Chart;
use kshana_events::{hora_at, vara_name, SolarDay};
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_SEC};
use kshana_vedic::DashaLord;

/// The parts of Kala bala, virupas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KalaParts {
    /// Nathonnata (day–night) bala.
    pub nathonnata: f64,
    /// Paksha (lunar fortnight) bala; the Moon's is doubled.
    pub paksha: f64,
    /// Tribhaga (third of the day or night) bala.
    pub tribhaga: f64,
    /// Abda bala: 15 to the lord of the year.
    pub abda: f64,
    /// Masa bala: 30 to the lord of the month.
    pub masa: f64,
    /// Vara bala: 45 to the lord of the weekday.
    pub vara: f64,
    /// Hora bala: 60 to the lord of the planetary hour.
    pub hora: f64,
    /// Ayana (declination) bala; the Sun's is doubled.
    pub ayana: f64,
    /// Yuddha bala: the war adjustment, positive for a victor, negative for the vanquished.
    pub yuddha: f64,
    /// All nine added.
    pub total: f64,
}

impl KalaParts {
    /// The sum before the war adjustment.
    pub fn before_war(&self) -> f64 {
        self.nathonnata
            + self.paksha
            + self.tribhaga
            + self.abda
            + self.masa
            + self.vara
            + self.hora
            + self.ayana
    }
}

/// The four lords of time at the birth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeLords {
    /// Abdadhipati: lord of the 360-day savana year.
    pub year: DashaLord,
    /// Masadhipati: lord of the 30-day savana month.
    pub month: DashaLord,
    /// Varadhipati: lord of the weekday, which runs sunrise to sunrise.
    pub day: DashaLord,
    /// Horadhipati: lord of the planetary hour.
    pub hora: DashaLord,
    /// Days elapsed since the Kali epoch on the civil day of the sunrise that began the
    /// vara, so that `kali_ahargana % 7` counts from that Friday.
    pub kali_ahargana: i64,
}

/// JDN of 1970-01-01.
const JDN_UNIX_EPOCH: i64 = 2_440_588;
/// JDN of Friday 18 February 3102 BCE (proleptic Julian), the first civil day of the
/// Kali yuga in the sunrise (audayika) reckoning; the midnight (ardharatrika) epoch of
/// JD 588465.5 is the start of the same day.
const JDN_KALI_EPOCH: i64 = 588_466;
/// Weekday of the Kali epoch, 0 = Sunday: a Friday.
const KALI_EPOCH_WEEKDAY: i64 = 5;

/// Kali ahargana of the vara that began at `day.sunrise`: the civil date is taken in the
/// local mean time of the place.
pub fn kali_ahargana(day: &SolarDay, obs: &Observer) -> i64 {
    let offset_ns = (obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128;
    let local = day.sunrise.add_ns(offset_ns);
    Utc::from_instant(local).utc_day() + JDN_UNIX_EPOCH - JDN_KALI_EPOCH
}

/// The lords of the year, month, weekday and hora.
///
/// BPHS ch. 27 and Varahamihira's rule (the *Brihat Samhita* reckoning of the savana
/// year lord) count the days elapsed from the Kali epoch: the year is 360 days and the
/// month 30, and the lord of each is the lord of the weekday on which it began. Since
/// 360 days are 51 weeks and 3 days, the year lord moves three weekdays per year; since
/// 30 days are 4 weeks and 2 days, the month lord moves two per month: year lord =
/// weekday(Friday + 3 × ⌊A/360⌋), month lord = weekday(Friday + 2 × ⌊A/30⌋), `A` the Kali
/// ahargana. Raman's tables and JHora's port use a modern epoch and the same "×3" and
/// "×2" steps, which shifts the year boundaries; the Kali reckoning is the one every
/// text describes. The weekday lord and the hora lord come from the sunrise-to-sunrise
/// day of `kshana_events`.
pub fn time_lords(day: &SolarDay, obs: &Observer, at: Instant) -> TimeLords {
    let a = kali_ahargana(day, obs);
    let lord_of = |steps: i64| weekday_lord((KALI_EPOCH_WEEKDAY + steps).rem_euclid(7) as u8);
    TimeLords {
        year: lord_of(3 * a.div_euclid(360)),
        month: lord_of(2 * a.div_euclid(30)),
        day: lord_by_name(vara_name(day, obs)).unwrap_or_else(|| lord_of(a)),
        hora: lord_by_name(hora_at(day, obs, at).0).unwrap_or(DashaLord::Sun),
        kali_ahargana: a,
    }
}

/// Local hour angle of the true Sun, degrees in (−180, 180]: 0 at apparent noon, ±180 at
/// apparent midnight.
pub fn sun_hour_angle_deg(chart: &Chart) -> f64 {
    let Some(sun) = chart.bodies.iter().find(|b| b.name == "Sun") else {
        return 0.0;
    };
    let (sl, cl) = sun.tropical.to_radians().sin_cos();
    let (sb, cb) = sun.latitude.to_radians().sin_cos();
    let (se, ce) = chart.obliquity_deg.to_radians().sin_cos();
    let ra = (cb * sl * ce - sb * se).atan2(cb * cl).to_degrees();
    (chart.lst_deg - ra + 180.0).rem_euclid(360.0) - 180.0
}

/// Declination, degrees, of a point at tropical longitude `lon`, ecliptic latitude `lat`
/// and true obliquity `obliquity`, all degrees.
pub fn declination_deg(lon: f64, lat: f64, obliquity: f64) -> f64 {
    let (sl, _) = lon.to_radians().sin_cos();
    let (sb, cb) = lat.to_radians().sin_cos();
    let (se, ce) = obliquity.to_radians().sin_cos();
    (sb * ce + cb * se * sl).asin().to_degrees()
}

/// Nathonnata bala (BPHS 27.8–9; Raman). The classical rule measures the birth from
/// midnight in ghatis (Unnata), Nata being its complement to 30 ghatis; the Moon, Mars
/// and Saturn take twice the Nata, the Sun, Jupiter and Venus twice the Unnata, and
/// Mercury always 60. Here the Sun's hour angle `h` stands for the apparent time from
/// noon: the Sun's group gets (180 − |h|)/3, the Moon's |h|/3, so that midnight gives the
/// night grahas 60 and noon the day grahas 60.
pub fn nathonnata_bala(g: DashaLord, sun_hour_angle_deg: f64) -> f64 {
    let h = sun_hour_angle_deg.abs().min(180.0);
    match g {
        DashaLord::Mercury => 60.0,
        DashaLord::Sun | DashaLord::Jupiter | DashaLord::Venus => (180.0 - h) / 3.0,
        DashaLord::Moon | DashaLord::Mars | DashaLord::Saturn => h / 3.0,
        _ => 0.0,
    }
}

/// Paksha bala (BPHS 27.10–11; Raman): the Moon's elongation from the Sun, folded to at
/// most 180°, over three is the Paksha bala of a benefic; 60 less that of a malefic; the
/// Moon's own is doubled. `benefic` is the graha's standing in this chart (the Moon
/// counts as a benefic while waxing, Mercury unless joined with malefics, as
/// `kshana_rules::is_benefic` decides).
pub fn paksha_bala(g: DashaLord, elongation_deg: f64, benefic: bool) -> f64 {
    let e = elongation_deg.rem_euclid(360.0);
    let e = if e > 180.0 { 360.0 - e } else { e };
    let base = e / 3.0;
    let v = if benefic { base } else { 60.0 - base };
    if g == DashaLord::Moon {
        2.0 * v
    } else {
        v
    }
}

/// Tribhaga bala (BPHS 27.12; Raman): the day from sunrise to sunset and the night from
/// sunset to the next sunrise are each cut in three; Mercury rules the first third of
/// the day, the Sun the second, Saturn the third; the Moon the first third of the night,
/// Venus the second, Mars the third; each gets 60 in its own third and Jupiter 60 always.
pub fn tribhaga_bala(g: DashaLord, day: &SolarDay, at: Instant) -> f64 {
    if g == DashaLord::Jupiter {
        return 60.0;
    }
    let (start, span, lords) = if at < day.sunset {
        (
            day.sunrise,
            day.sunset.ns_since(day.sunrise),
            [DashaLord::Mercury, DashaLord::Sun, DashaLord::Saturn],
        )
    } else {
        (
            day.sunset,
            day.next_sunrise.ns_since(day.sunset),
            [DashaLord::Moon, DashaLord::Venus, DashaLord::Mars],
        )
    };
    let third = (at.ns_since(start) * 3 / span.max(1)).clamp(0, 2) as usize;
    if lords[third] == g {
        60.0
    } else {
        0.0
    }
}

/// Ayana bala (BPHS 27.13–16; Raman): with the declination `δ` in degrees, the strength
/// is (24 + x) × 60/48 where x = +δ for the Sun, Mars, Jupiter and Venus (strong north),
/// −δ for the Moon and Saturn (strong south) and |δ| for Mercury (always added); the
/// Sun's result is doubled (BPHS; Raman; JHora). The result is held to 0–60 before the
/// doubling, since the Moon's declination can pass 24°.
pub fn ayana_bala(g: DashaLord, declination_deg: f64) -> f64 {
    let x = match g {
        DashaLord::Sun | DashaLord::Mars | DashaLord::Jupiter | DashaLord::Venus => declination_deg,
        DashaLord::Moon | DashaLord::Saturn => -declination_deg,
        DashaLord::Mercury => declination_deg.abs(),
        _ => return 0.0,
    };
    let v = ((24.0 + x) * 1.25).clamp(0.0, 60.0);
    if g == DashaLord::Sun {
        2.0 * v
    } else {
        v
    }
}

/// A planetary war.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct War {
    /// The victor.
    pub winner: DashaLord,
    /// The vanquished.
    pub loser: DashaLord,
    /// Separation in longitude, degrees.
    pub separation_deg: f64,
    /// Virupas moved from the loser to the winner.
    pub virupas: f64,
}

/// Yuddha bala (BPHS 27.20; Raman, Kala bala; JHora): two of the five tara grahas within
/// one degree of longitude are at war. The difference of their strengths gathered so far
/// (`strength` gives each graha's Sthana, Dig and Kala bala before the war) divided by
/// the difference of their disc diameters is added to the victor and taken from the
/// vanquished. The victor is the graha further north in ecliptic latitude (BPHS; Sripati);
/// Raman notes the alternative reading that the one with the lesser longitude wins.
/// `positions` are `(graha, sidereal longitude, ecliptic latitude)`.
pub fn wars(positions: &[(DashaLord, f64, f64)], strength: impl Fn(DashaLord) -> f64) -> Vec<War> {
    let mut out = Vec::new();
    let find = |g: DashaLord| positions.iter().find(|(p, _, _)| *p == g).copied();
    for (i, a) in TARA.iter().enumerate() {
        for b in TARA.iter().skip(i + 1) {
            let (Some((_, la, ba)), Some((_, lb, bb))) = (find(*a), find(*b)) else {
                continue;
            };
            let sep = (la - lb).rem_euclid(360.0);
            let sep = if sep > 180.0 { 360.0 - sep } else { sep };
            if sep >= 1.0 {
                continue;
            }
            let (Some(da), Some(db)) = (disc_diameter(*a), disc_diameter(*b)) else {
                continue;
            };
            let virupas = (strength(*a) - strength(*b)).abs() / (da - db).abs();
            let (winner, loser) = if ba >= bb { (*a, *b) } else { (*b, *a) };
            out.push(War {
                winner,
                loser,
                separation_deg: sep,
                virupas,
            });
        }
    }
    out
}

/// Which third of the day or night the instant falls in, for the words: `(is_day, third)`.
pub fn tribhaga_of(day: &SolarDay, at: Instant) -> (bool, usize) {
    let (start, span, is_day) = if at < day.sunset {
        (day.sunrise, day.sunset.ns_since(day.sunrise), true)
    } else {
        (day.sunset, day.next_sunrise.ns_since(day.sunset), false)
    };
    (
        is_day,
        (at.ns_since(start) * 3 / span.max(1)).clamp(0, 2) as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;
    use kshana_vedic::DashaLord::*;

    #[test]
    fn nathonnata_at_noon_and_midnight() {
        assert_eq!(nathonnata_bala(Sun, 0.0), 60.0);
        assert_eq!(nathonnata_bala(Moon, 0.0), 0.0);
        assert_eq!(nathonnata_bala(Sun, 180.0), 0.0);
        assert_eq!(nathonnata_bala(Saturn, -180.0), 60.0);
        assert_eq!(nathonnata_bala(Mercury, 37.0), 60.0);
        assert_eq!(nathonnata_bala(Jupiter, 90.0), 30.0);
    }

    #[test]
    fn paksha_doubles_the_moon_and_complements_malefics() {
        assert!((paksha_bala(Jupiter, 90.0, true) - 30.0).abs() < 1e-12);
        assert!((paksha_bala(Saturn, 90.0, false) - 30.0).abs() < 1e-12);
        assert!((paksha_bala(Venus, 270.0, true) - 30.0).abs() < 1e-12);
        assert!((paksha_bala(Moon, 180.0, true) - 120.0).abs() < 1e-12);
        assert!((paksha_bala(Moon, 60.0, true) - 40.0).abs() < 1e-12);
        assert!((paksha_bala(Mars, 180.0, false)).abs() < 1e-12);
    }

    #[test]
    fn ayana_by_declination() {
        assert!((ayana_bala(Jupiter, 23.5) - 59.375).abs() < 1e-9);
        assert!((ayana_bala(Moon, -10.75) - 43.4375).abs() < 1e-9);
        assert!((ayana_bala(Mercury, -9.0) - 41.25).abs() < 1e-9);
        assert!((ayana_bala(Sun, -8.75) - 2.0 * 19.0625).abs() < 1e-9);
        assert!((ayana_bala(Saturn, 0.0) - 30.0).abs() < 1e-9);
        assert_eq!(ayana_bala(Moon, 28.0), 0.0);
        assert!((declination_deg(90.0, 0.0, 23.44) - 23.44).abs() < 1e-9);
        assert!(declination_deg(0.0, 0.0, 23.44).abs() < 1e-12);
    }

    #[test]
    fn tribhaga_thirds() {
        let sunrise = Instant::j2000();
        let day = SolarDay {
            sunrise,
            sunset: sunrise.add_ns(12 * NS_PER_DAY / 24),
            next_sunrise: sunrise.add_ns(NS_PER_DAY),
        };
        let hour = NS_PER_DAY / 24;
        assert_eq!(tribhaga_bala(Mercury, &day, sunrise.add_ns(hour)), 60.0);
        assert_eq!(tribhaga_bala(Sun, &day, sunrise.add_ns(5 * hour)), 60.0);
        assert_eq!(tribhaga_bala(Saturn, &day, sunrise.add_ns(9 * hour)), 60.0);
        assert_eq!(tribhaga_bala(Moon, &day, sunrise.add_ns(13 * hour)), 60.0);
        assert_eq!(tribhaga_bala(Venus, &day, sunrise.add_ns(17 * hour)), 60.0);
        assert_eq!(tribhaga_bala(Mars, &day, sunrise.add_ns(21 * hour)), 60.0);
        assert_eq!(
            tribhaga_bala(Jupiter, &day, sunrise.add_ns(21 * hour)),
            60.0
        );
        assert_eq!(tribhaga_bala(Sun, &day, sunrise.add_ns(21 * hour)), 0.0);
        assert_eq!(tribhaga_of(&day, sunrise.add_ns(17 * hour)), (false, 1));
    }

    #[test]
    fn war_within_a_degree_goes_to_the_northern_graha() {
        // Mars and Mercury half a degree apart, Mercury further north
        let pos = [
            (Mars, 100.0, -1.0),
            (Mercury, 100.5, 0.5),
            (Jupiter, 200.0, 0.0),
        ];
        let w = wars(&pos, |g| if g == Mars { 300.0 } else { 280.0 });
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].winner, Mercury);
        assert_eq!(w[0].loser, Mars);
        // 20 / |9.4 − 6.6| = 7.142857
        assert!((w[0].virupas - 20.0 / 2.8).abs() < 1e-9);
        // no war at a degree or more, nor for the luminaries
        let pos = [(Mars, 100.0, 0.0), (Mercury, 101.0, 0.0), (Sun, 100.2, 0.0)];
        assert!(wars(&pos, |_| 1.0).is_empty());
    }
}
