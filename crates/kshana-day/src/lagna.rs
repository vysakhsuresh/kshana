//! The BPHS sub-hour lagnas as longitude functions of time, with exact sign changes.
//!
//! BPHS ch. 5 (Santhanam; bphs.blogspot.com/2007/12/ch-5-special-lagnas.html;
//! barbarapijan.com "lagna vishesika"): with `t` the time elapsed since the solved
//! sunrise and `S0` the Sun's sidereal longitude at that sunrise,
//!
//! * Bhava lagna = S0 + 30° × t / 5 ghatis (2 h), i.e. 0.25°/min;
//! * Hora lagna = S0 + 30° × t / 2.5 ghatis (1 h), i.e. 0.5°/min;
//! * Ghati lagna = S0 + 30° × t / 1 ghati (24 min), i.e. 1.25°/min;
//! * Vighati lagna = S0 + 30° × t / 15 vighatis (6 min), i.e. 5°/min.
//!
//! Pranapada (BPHS ch. 3, Santhanam's worked example; PyJHora `pranapada_lagna`): the
//! elapsed time in vighatis divided by 15 gives signs; the result is added to the Sun's
//! longitude when the Sun is in a movable sign, to the ninth sign from the Sun (Sun +
//! 240°) in a fixed sign, and to the fifth sign from the Sun (Sun + 120°) in a dual
//! sign. The text adds to the Sun at birth; some practitioners use the Sun at sunrise,
//! so the anchor is a switch ([`SunAnchor`]), and the sign type is always judged from
//! the Sun at the instant. Pranapada in the 2nd, 4th, 5th, 9th, 10th or 11th from the
//! lagna marks an auspicious birth ([`pranapada_auspicious`]).
//!
//! Sree lagna (BPHS ch. 5; PyJHora `sree_lagna_from_moon_asc_longitudes`): the lagna
//! advanced by 360° times the fraction of the Moon's nakshatra already traversed.
//!
//! The ghati here is the fixed 24-minute ghati of the text (BPHS, Jagannatha Hora),
//! and every longitude is sidereal in the chosen ayanamsa. Sign changes are found with
//! `kshana_trace::crossings` on the longitude function and are roots solved to a
//! microsecond; a lagna that resets at sunrise is scanned one solar day at a time.

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{SolarDay, MICROSECOND_NS};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_SEC};
use kshana_trace::{crossings, Division};
use kshana_vedic::{nakshatra, Ayanamsa};

use crate::{ascendant_sidereal, sidereal_longitude, solar_days_covering, DayError};

/// The special lagnas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecialLagna {
    /// One sign per five ghatis from the sunrise Sun.
    Bhava,
    /// One sign per two and a half ghatis.
    Hora,
    /// One sign per ghati.
    Ghati,
    /// One sign per fifteen vighatis; the Pranapada before its sign-type offset.
    Vighati,
    /// Vighati lagna with the movable/fixed/dual offset from the Sun.
    Pranapada,
    /// Lagna plus the Moon's nakshatra fraction.
    Sree,
}

impl SpecialLagna {
    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            SpecialLagna::Bhava => "Bhava lagna",
            SpecialLagna::Hora => "Hora lagna",
            SpecialLagna::Ghati => "Ghati lagna",
            SpecialLagna::Vighati => "Vighati lagna",
            SpecialLagna::Pranapada => "Pranapada",
            SpecialLagna::Sree => "Sree lagna",
        }
    }

    /// Rate in degrees per minute of fixed-ghati time; `None` for Sree lagna, which is
    /// not a linear clock.
    pub fn degrees_per_minute(self) -> Option<f64> {
        match self {
            SpecialLagna::Bhava => Some(0.25),
            SpecialLagna::Hora => Some(0.5),
            SpecialLagna::Ghati => Some(1.25),
            SpecialLagna::Vighati | SpecialLagna::Pranapada => Some(5.0),
            SpecialLagna::Sree => None,
        }
    }

    /// Scan step for the sign-change search: well under half the time per sign.
    fn scan_step_ns(self) -> i128 {
        let minutes = match self {
            SpecialLagna::Bhava => 40,
            SpecialLagna::Hora => 20,
            SpecialLagna::Ghati => 8,
            SpecialLagna::Vighati | SpecialLagna::Pranapada => 2,
            SpecialLagna::Sree => 15,
        };
        minutes * 60 * NS_PER_SEC
    }
}

/// Which Sun the Pranapada is counted from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SunAnchor {
    /// The Sun at the instant itself (the text).
    AtInstant,
    /// The Sun at the day's sunrise (some practitioners; continuous with the other
    /// ghati-family lagnas).
    AtSunrise,
}

/// Sidereal longitude of the Sun at `t`.
pub fn sun_sidereal_at(spk: &Spk, t: Instant, ayanamsa: Ayanamsa) -> Result<f64, SpkError> {
    sidereal_longitude(spk, Body::Sun, t, ayanamsa)
}

/// A ghati-family lagna: `sun_at_sunrise + rate × minutes since sunrise`, degrees in
/// `[0, 360)`. `rate_deg_per_min` is [`SpecialLagna::degrees_per_minute`].
pub fn ghati_family_lagna(
    rate_deg_per_min: f64,
    sun_at_sunrise: f64,
    day: &SolarDay,
    at: Instant,
) -> f64 {
    let minutes = at.ns_since(day.sunrise) as f64 / (60.0 * NS_PER_SEC as f64);
    (sun_at_sunrise + rate_deg_per_min * minutes).rem_euclid(360.0)
}

/// Sign-type offset of the Pranapada for a Sun in sign `sun_sidereal`: 0° movable,
/// 240° fixed, 120° dual.
pub fn pranapada_offset(sun_sidereal: f64) -> f64 {
    match ((sun_sidereal.rem_euclid(360.0) / 30.0).floor() as usize) % 3 {
        0 => 0.0,
        1 => 240.0,
        _ => 120.0,
    }
}

/// Pranapada: `base_sun + 5° × minutes since sunrise + offset(sun_at_instant)`, degrees
/// in `[0, 360)`. `base_sun` is the Sun at the instant or at sunrise by the anchor
/// chosen; `sun_at_instant` decides the sign type.
pub fn pranapada(sun_at_instant: f64, base_sun: f64, day: &SolarDay, at: Instant) -> f64 {
    let minutes = at.ns_since(day.sunrise) as f64 / (60.0 * NS_PER_SEC as f64);
    (base_sun + 5.0 * minutes + pranapada_offset(sun_at_instant)).rem_euclid(360.0)
}

/// Pranapada at `at` from the ephemeris.
pub fn pranapada_at(
    spk: &Spk,
    day: &SolarDay,
    at: Instant,
    ayanamsa: Ayanamsa,
    anchor: SunAnchor,
) -> Result<f64, SpkError> {
    let sun = sun_sidereal_at(spk, at, ayanamsa)?;
    let base = match anchor {
        SunAnchor::AtInstant => sun,
        SunAnchor::AtSunrise => sun_sidereal_at(spk, day.sunrise, ayanamsa)?,
    };
    Ok(pranapada(sun, base, day, at))
}

/// True when the Pranapada falls in the 2nd, 4th, 5th, 9th, 10th or 11th sign from the
/// lagna (both sidereal degrees).
pub fn pranapada_auspicious(pranapada_sidereal: f64, lagna_sidereal: f64) -> bool {
    let pp = (pranapada_sidereal.rem_euclid(360.0) / 30.0).floor() as i64;
    let lg = (lagna_sidereal.rem_euclid(360.0) / 30.0).floor() as i64;
    let house = (pp - lg).rem_euclid(12) + 1;
    matches!(house, 2 | 4 | 5 | 9 | 10 | 11)
}

/// Sree lagna from the sidereal ascendant and Moon, degrees in `[0, 360)`.
pub fn sree_lagna(ascendant_sidereal: f64, moon_sidereal: f64) -> f64 {
    (ascendant_sidereal + 360.0 * nakshatra(moon_sidereal).fraction).rem_euclid(360.0)
}

/// Sree lagna at `at` from the ephemeris and the place.
pub fn sree_lagna_at(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<f64, SpkError> {
    let moon = sidereal_longitude(spk, Body::Moon, at, ayanamsa)?;
    Ok(sree_lagna(ascendant_sidereal(eop, obs, at, ayanamsa), moon))
}

/// What the lagna functions need.
#[derive(Clone, Copy)]
pub struct LagnaContext<'a> {
    /// Ephemeris.
    pub spk: &'a Spk,
    /// Earth orientation, if available.
    pub eop: Option<&'a EopTable>,
    /// Place.
    pub obs: &'a Observer,
    /// Sidereal zodiac.
    pub ayanamsa: Ayanamsa,
    /// Sun anchor for the Pranapada.
    pub anchor: SunAnchor,
}

/// One sign change of a special lagna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LagnaChange {
    /// Which lagna.
    pub which: SpecialLagna,
    /// Exact instant.
    pub at: Instant,
    /// Half-width of the solved bracket, nanoseconds.
    pub half_width_ns: i128,
    /// Sign entered, 0 = Aries.
    pub sign: usize,
}

/// `which` at `at`, within `day` (the day is unused for Sree lagna).
pub fn special_lagna_at(
    ctx: &LagnaContext<'_>,
    which: SpecialLagna,
    day: &SolarDay,
    at: Instant,
) -> Result<f64, DayError> {
    match which {
        SpecialLagna::Sree => Ok(sree_lagna_at(ctx.spk, ctx.eop, ctx.obs, at, ctx.ayanamsa)?),
        SpecialLagna::Pranapada => Ok(pranapada_at(ctx.spk, day, at, ctx.ayanamsa, ctx.anchor)?),
        _ => {
            let rate = which.degrees_per_minute().unwrap_or(0.0);
            let s0 = sun_sidereal_at(ctx.spk, day.sunrise, ctx.ayanamsa)?;
            Ok(ghati_family_lagna(rate, s0, day, at))
        }
    }
}

/// Every instant in `[from, to)` at which `which` enters a sign, each a root solved to
/// a microsecond, in time order.
pub fn lagna_sign_changes(
    ctx: &LagnaContext<'_>,
    which: SpecialLagna,
    from: Instant,
    to: Instant,
) -> Result<Vec<LagnaChange>, DayError> {
    let step = which.scan_step_ns();
    let mut out = Vec::new();
    let push = |out: &mut Vec<LagnaChange>, cs: Vec<kshana_trace::Crossing>| {
        out.extend(cs.into_iter().map(|c| LagnaChange {
            which,
            at: c.at,
            half_width_ns: c.half_width_ns,
            sign: c.entered,
        }));
    };
    if which == SpecialLagna::Sree {
        let f = |t: Instant| sree_lagna_at(ctx.spk, ctx.eop, ctx.obs, t, ctx.ayanamsa);
        push(
            &mut out,
            crossings(f, Division::Sign, from, to, step, MICROSECOND_NS)?,
        );
        return Ok(out);
    }
    for day in solar_days_covering(ctx.spk, ctx.eop, ctx.obs, from, to)? {
        let a = day.sunrise.max(from);
        let b = day.next_sunrise.min(to);
        if b <= a {
            continue;
        }
        if which == SpecialLagna::Pranapada {
            let f = |t: Instant| pranapada_at(ctx.spk, &day, t, ctx.ayanamsa, ctx.anchor);
            push(
                &mut out,
                crossings(f, Division::Sign, a, b, step, MICROSECOND_NS)?,
            );
        } else {
            let rate = which.degrees_per_minute().unwrap_or(0.0);
            let s0 = sun_sidereal_at(ctx.spk, day.sunrise, ctx.ayanamsa)?;
            let f = |t: Instant| Ok(ghati_family_lagna(rate, s0, &day, t));
            push(
                &mut out,
                crossings(f, Division::Sign, a, b, step, MICROSECOND_NS)?,
            );
        }
    }
    out.sort_by_key(|c| c.at);
    out.retain(|c| c.at >= from && c.at < to);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GHATI_NS;
    use crate::fixture::{kernel, utc, KOCHI};
    use kshana_events::solar_day;
    use kshana_solver::wrap180;
    use kshana_time::NS_PER_DAY;

    fn fake_day() -> SolarDay {
        let sunrise = utc(2026, 10, 15, 0, 50, 0);
        SolarDay {
            sunrise,
            sunset: sunrise.add_ns(12 * 3_600 * NS_PER_SEC),
            next_sunrise: sunrise.add_ns(NS_PER_DAY),
        }
    }

    #[test]
    fn ghati_family_rates() {
        let d = fake_day();
        let s0 = 100.0;
        let g = SpecialLagna::Ghati.degrees_per_minute().unwrap();
        assert!((ghati_family_lagna(g, s0, &d, d.sunrise.add_ns(GHATI_NS)) - 130.0).abs() < 1e-9);
        let h = SpecialLagna::Hora.degrees_per_minute().unwrap();
        assert!(
            (ghati_family_lagna(h, s0, &d, d.sunrise.add_ns(3_600 * NS_PER_SEC)) - 130.0).abs()
                < 1e-9
        );
        let b = SpecialLagna::Bhava.degrees_per_minute().unwrap();
        assert!(
            (ghati_family_lagna(b, s0, &d, d.sunrise.add_ns(2 * 3_600 * NS_PER_SEC)) - 130.0).abs()
                < 1e-9
        );
        let v = SpecialLagna::Vighati.degrees_per_minute().unwrap();
        assert!(
            (ghati_family_lagna(v, s0, &d, d.sunrise.add_ns(6 * 60 * NS_PER_SEC)) - 130.0).abs()
                < 1e-9
        );
        assert!(SpecialLagna::Sree.degrees_per_minute().is_none());
    }

    #[test]
    fn pranapada_formula_spot_check() {
        let d = fake_day();
        // Sun at 40° (Taurus, fixed): twelve minutes = 30 vighatis = 2 signs, plus the 9th
        let at = d.sunrise.add_ns(12 * 60 * NS_PER_SEC);
        assert!((pranapada(40.0, 40.0, &d, at) - 340.0).abs() < 1e-9);
        // movable sign: no offset; dual: fifth from the Sun
        assert!((pranapada(10.0, 10.0, &d, at) - 70.0).abs() < 1e-9);
        assert!((pranapada(70.0, 70.0, &d, at) - 250.0).abs() < 1e-9);
        // 1 vighati (24 s) = 2°
        assert!((pranapada(10.0, 10.0, &d, d.sunrise.add_ns(24 * NS_PER_SEC)) - 12.0).abs() < 1e-9);
        assert_eq!(pranapada_offset(359.9), 120.0);
        assert!(pranapada_auspicious(45.0, 10.0)); // 2nd
        assert!(!pranapada_auspicious(10.0, 10.0)); // 1st
        assert!(pranapada_auspicious(310.0, 10.0)); // 11th
        assert!(!pranapada_auspicious(220.0, 10.0)); // 8th
    }

    #[test]
    fn sree_lagna_adds_the_nakshatra_fraction() {
        // Moon at the midpoint of Ashwini: lagna + 180°
        assert!((sree_lagna(100.0, 20.0 / 3.0) - 280.0).abs() < 1e-9);
        assert!((sree_lagna(350.0, 0.0) - 350.0).abs() < 1e-9);
    }

    #[test]
    fn sign_changes_of_the_ghati_lagna_are_24_minutes_apart() {
        let Some(spk) = kernel() else { return };
        let ctx = LagnaContext {
            spk: &spk,
            eop: None,
            obs: &KOCHI,
            ayanamsa: Ayanamsa::Lahiri,
            anchor: SunAnchor::AtSunrise,
        };
        let at = utc(2026, 10, 15, 9, 0, 0);
        let day = solar_day(&spk, None, &KOCHI, at).unwrap().unwrap();
        let ch =
            lagna_sign_changes(&ctx, SpecialLagna::Ghati, day.sunrise, day.next_sunrise).unwrap();
        assert!(ch.len() == 59 || ch.len() == 60, "{}", ch.len());
        for w in ch.windows(2) {
            assert!(
                (w[1].at.ns_since(w[0].at) - GHATI_NS).abs() <= 2_000,
                "{:?}",
                w
            );
            assert_eq!(w[1].sign, (w[0].sign + 1) % 12);
        }
        // each change sits on a sign boundary of the function
        for c in ch.iter().take(5) {
            let l = special_lagna_at(&ctx, SpecialLagna::Ghati, &day, c.at).unwrap();
            assert!(
                wrap180(l - 30.0 * c.sign as f64).abs() < 1e-6,
                "{l} vs sign {}",
                c.sign
            );
            assert!(c.half_width_ns <= MICROSECOND_NS);
        }
        // the Pranapada clock changes sign every six minutes
        let pp = lagna_sign_changes(
            &ctx,
            SpecialLagna::Pranapada,
            day.sunrise,
            day.sunrise.add_ns(3 * 3_600 * NS_PER_SEC),
        )
        .unwrap();
        assert!(pp.len() == 30 || pp.len() == 31, "{}", pp.len());
        for w in pp.windows(2) {
            assert!((w[1].at.ns_since(w[0].at) - 6 * 60 * NS_PER_SEC).abs() <= 2_000);
        }
        // the Bhava lagna once every two hours, continuing across the day
        let bl = lagna_sign_changes(&ctx, SpecialLagna::Bhava, at, at.add_ns(NS_PER_DAY)).unwrap();
        assert!((11..=13).contains(&bl.len()), "{}", bl.len());
        assert!(bl.windows(2).all(|w| w[0].at < w[1].at));
    }

    #[test]
    fn sree_lagna_sign_changes_sit_on_boundaries() {
        let Some(spk) = kernel() else { return };
        let ctx = LagnaContext {
            spk: &spk,
            eop: None,
            obs: &KOCHI,
            ayanamsa: Ayanamsa::Lahiri,
            anchor: SunAnchor::AtInstant,
        };
        let from = utc(2026, 10, 15, 0, 0, 0);
        let ch =
            lagna_sign_changes(&ctx, SpecialLagna::Sree, from, from.add_ns(NS_PER_DAY)).unwrap();
        // lagna 360°/day plus the nakshatra fraction ~360°/day: about 24 changes
        assert!((18..=30).contains(&ch.len()), "{}", ch.len());
        let day = solar_day(&spk, None, &KOCHI, from.add_ns(12 * 3_600 * NS_PER_SEC))
            .unwrap()
            .unwrap();
        for c in &ch {
            let l = special_lagna_at(&ctx, SpecialLagna::Sree, &day, c.at).unwrap();
            assert!(
                wrap180(l - 30.0 * c.sign as f64).abs() < 1e-5,
                "{l} vs sign {}",
                c.sign
            );
            assert!(c.half_width_ns <= MICROSECOND_NS);
        }
    }
}
