//! The day's clocks over the FFI: muhurtas, choghadiya, ghatis, upagrahas, special
//! lagnas, the Moon's windows and Pancha Pakshi.

use kshana_day::{GhatiConvention, SpecialLagna, SunAnchor};
use kshana_events::solar_day;
use kshana_sky::Observer;
use kshana_time::Instant;

use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// A named window of the day.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DayWindowRecord {
    pub kind: String,
    pub name: String,
    pub lord: String,
    pub night: bool,
    pub index: u8,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub favourable: bool,
}

/// The day's clocks at one instant.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DayClockRecord {
    pub sunrise: ExactInstant,
    pub sunset: ExactInstant,
    pub next_sunrise: ExactInstant,
    pub weekday_lord: String,
    pub ghati: u32,
    pub vighati: u32,
    pub vipala: f64,
    pub ghati_fixed: u32,
    pub vighati_fixed: u32,
    pub muhurta: Option<DayWindowRecord>,
    pub choghadiya: Option<DayWindowRecord>,
    pub abhijit_start: ExactInstant,
    pub abhijit_end: ExactInstant,
    pub brahma_start: ExactInstant,
    pub brahma_end: ExactInstant,
    pub muhurtas: Vec<DayWindowRecord>,
    pub choghadiyas: Vec<DayWindowRecord>,
    pub upagrahas: Vec<UpagrahaRecord>,
    pub special_lagnas: Vec<SpecialLagnaRecord>,
}

/// One upagraha's longitude.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct UpagrahaRecord {
    pub name: String,
    pub point: String,
    pub night: bool,
    pub at: ExactInstant,
    pub sidereal: f64,
}

/// One special lagna's longitude.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SpecialLagnaRecord {
    pub name: String,
    pub key: String,
    pub sidereal: f64,
}

/// A labelled window of the Moon.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct LunarWindowRecord {
    pub kind: String,
    pub label: String,
    pub index: u32,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub favourable: bool,
}

/// One Pancha Pakshi window.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PakshiRecord {
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub night: bool,
    pub yama: u8,
    pub bird: String,
    pub activity: String,
    pub strength: f64,
    pub favourable: bool,
}

fn obs(latitude_deg: f64, longitude_deg: f64) -> Observer {
    Observer {
        latitude_deg,
        longitude_deg,
        height_m: 0.0,
    }
}

fn err<E: std::fmt::Debug>(e: E) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

#[uniffi::export]
impl Engine {
    /// The day's clocks for the solar day containing `at` at a place.
    pub fn day_clock(
        &self,
        at: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<DayClockRecord, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let w = |s: &str| kshana_day::word_in(s, lang);
        let o = obs(latitude_deg, longitude_deg);
        let at_i: Instant = at.into();
        let day = solar_day(&self.spk, self.eop.as_ref(), &o, at_i)
            .map_err(err)?
            .ok_or_else(|| KshanaError::Engine("the Sun does not rise here today".into()))?;
        let (g, v, p) = kshana_day::ghati_vighati(&day, at_i, GhatiConvention::SolarDay);
        let (gf, vf, _) = kshana_day::ghati_vighati(&day, at_i, GhatiConvention::Fixed);
        let mw = |m: &kshana_day::MuhurtaWindow| DayWindowRecord {
            kind: "muhurta".into(),
            name: w(m.muhurta.name),
            lord: w(m.muhurta.alias.unwrap_or("")),
            night: m.night,
            index: m.muhurta.number,
            start: m.start.into(),
            end: m.end.into(),
            favourable: m.auspicious,
        };
        let cw = |c: &kshana_day::ChoghadiyaWindow| DayWindowRecord {
            kind: "choghadiya".into(),
            name: w(c.kind.name()),
            lord: w(c.kind.lord()),
            night: c.night,
            index: c.slot,
            start: c.start.into(),
            end: c.end.into(),
            favourable: c.favourable,
        };
        let (a0, a1) = kshana_day::abhijit(&day);
        let (b0, b1) = kshana_day::brahma_muhurta(&day);
        let ay = ayanamsa.into();
        let ctx = kshana_day::LagnaContext {
            spk: &self.spk,
            eop: self.eop.as_ref(),
            obs: &o,
            ayanamsa: ay,
            anchor: SunAnchor::AtInstant,
        };
        let mut special = Vec::new();
        for which in [
            SpecialLagna::Bhava,
            SpecialLagna::Hora,
            SpecialLagna::Ghati,
            SpecialLagna::Vighati,
            SpecialLagna::Pranapada,
            SpecialLagna::Sree,
        ] {
            if let Ok(l) = kshana_day::special_lagna_at(&ctx, which, &day, at_i) {
                special.push(SpecialLagnaRecord {
                    name: w(which.name()),
                    key: which.name().to_string(),
                    sidereal: l,
                });
            }
        }
        Ok(DayClockRecord {
            sunrise: day.sunrise.into(),
            sunset: day.sunset.into(),
            next_sunrise: day.next_sunrise.into(),
            weekday_lord: w(kshana_day::WEEKDAY_LORDS[kshana_day::weekday(&day, &o) as usize % 7]),
            ghati: g,
            vighati: v,
            vipala: p,
            ghati_fixed: gf,
            vighati_fixed: vf,
            muhurta: kshana_day::muhurta_at(&day, &o, at_i).as_ref().map(mw),
            choghadiya: kshana_day::choghadiya_at(&day, &o, at_i).as_ref().map(cw),
            abhijit_start: a0.into(),
            abhijit_end: a1.into(),
            brahma_start: b0.into(),
            brahma_end: b1.into(),
            muhurtas: kshana_day::muhurtas(&day, &o).iter().map(mw).collect(),
            choghadiyas: kshana_day::choghadiya(&day, &o).iter().map(cw).collect(),
            upagrahas: kshana_day::upagraha_longitudes(self.eop.as_ref(), &o, &day, at_i, ay)
                .iter()
                .map(|u| UpagrahaRecord {
                    name: w(u.which.name()),
                    point: w(&format!("{:?}", u.point)),
                    night: u.night,
                    at: u.at.into(),
                    sidereal: u.sidereal,
                })
                .collect(),
            special_lagnas: special,
        })
    }

    /// The Moon's labelled windows over `[from, to]`: "gandanta", "panchaka", "bhadra",
    /// "yoga" (the inauspicious yogas), "tara" and "chandra" (for a natal Moon).
    #[allow(clippy::too_many_arguments)]
    pub fn moon_windows(
        &self,
        kind: String,
        ayanamsa: AyanamsaChoice,
        natal_moon_sidereal: f64,
        from: ExactInstant,
        to: ExactInstant,
        lang: String,
    ) -> Result<Vec<LunarWindowRecord>, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ay = ayanamsa.into();
        let (f, t): (Instant, Instant) = (from.into(), to.into());
        if t.ns_since(f) > 400 * 86_400 * 1_000_000_000 {
            return Err(KshanaError::Engine(
                "windows cover at most 400 days per call".into(),
            ));
        }
        let w = match kind.as_str() {
            "gandanta" => kshana_day::gandanta_windows(&self.spk, ay, f, t),
            "panchaka" => kshana_day::panchaka_windows(&self.spk, ay, f, t),
            "bhadra" => kshana_day::bhadra_windows(&self.spk, f, t),
            "yoga" => kshana_day::special_yoga_windows(&self.spk, ay, f, t),
            "tara" => kshana_day::tara_bala_windows(&self.spk, ay, natal_moon_sidereal, f, t),
            "chandra" => kshana_day::chandra_bala_windows(&self.spk, ay, natal_moon_sidereal, f, t),
            other => return Err(KshanaError::Engine(format!("unknown window kind {other}"))),
        }
        .map_err(err)?;
        Ok(w.into_iter()
            .map(|x| LunarWindowRecord {
                kind: kind.clone(),
                label: kshana_day::word_in(x.label, lang),
                index: x.index as u32,
                start: x.start.into(),
                end: x.end.into(),
                favourable: x.favourable,
            })
            .collect())
    }

    /// Pancha Pakshi: the birth bird and the day's ten windows at a place. The activity
    /// tables could not be verified against a primary text; the engine marks them so.
    #[allow(clippy::too_many_arguments)]
    pub fn pancha_pakshi(
        &self,
        birth: ExactInstant,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        lang: String,
    ) -> Result<Vec<PakshiRecord>, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ay = ayanamsa.into();
        let bird = kshana_day::birth_bird_at(&self.spk, birth.into(), ay).map_err(err)?;
        let o = obs(latitude_deg, longitude_deg);
        let w =
            kshana_day::pancha_pakshi_windows(&self.spk, self.eop.as_ref(), &o, bird, at.into())
                .map_err(err)?;
        Ok(w.into_iter()
            .map(|x| PakshiRecord {
                start: x.start.into(),
                end: x.end.into(),
                night: x.night,
                yama: x.yama,
                bird: kshana_day::word_in(x.bird.name(), lang),
                activity: kshana_day::word_in(x.activity.name(), lang),
                strength: x.strength,
                favourable: x.favourable,
            })
            .collect())
    }
}

/// The birth bird's name for a sidereal Moon and the paksha.
#[uniffi::export]
pub fn birth_bird(moon_sidereal: f64, shukla: bool, lang: String) -> String {
    kshana_day::word_in(
        kshana_day::birth_bird(moon_sidereal, shukla).name(),
        kshana_rules::Lang::parse(&lang),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn day_clock_crosses_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let u = |y, mo, d, h, mi| {
            instant_from_utc(UtcFields {
                year: y,
                month: mo,
                day: d,
                hour: h,
                minute: mi,
                second: 0,
                nanos: 0,
            })
            .unwrap()
        };
        let at = u(2026, 10, 16, 3, 30);
        let d = engine
            .day_clock(at, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(d.muhurtas.len(), 30);
        assert_eq!(d.choghadiyas.len(), 16);
        assert!(d.muhurta.is_some() && d.choghadiya.is_some());
        assert_eq!(d.upagrahas.len(), 6);
        assert_eq!(d.special_lagnas.len(), 6);
        let g = engine
            .moon_windows(
                "gandanta".into(),
                AyanamsaChoice::Lahiri,
                0.0,
                at,
                u(2026, 11, 16, 0, 0),
                "en".into(),
            )
            .unwrap();
        assert!(!g.is_empty());
        let pk = engine
            .pancha_pakshi(
                u(1990, 10, 29, 12, 6),
                AyanamsaChoice::Lahiri,
                at,
                9.9312,
                76.2673,
                "ml".into(),
            )
            .unwrap();
        assert_eq!(pk.len(), 10);
        assert!(!birth_bird(316.344, true, "hi".into()).is_empty());
    }
}
