//! Western and Hellenistic timing over the FFI: secondary progressions and their exact
//! contacts, solar arc directions, profections, firdaria, zodiacal releasing, lunar
//! returns and exact transits, all thin wrappers over `kshana-directions`.

use kshana_chart::BODIES;
use kshana_directions::{
    firdaria, is_day_at, lots, lunar_returns, natal_points, profections,
    progressed_aspect_instants, progressed_instant, progressions, solar_arc_hits, transit_hits,
    zodiacal_releasing_from, DirectionsError,
};
use kshana_ephem::Body;
use kshana_sky::{apparent, Observer};
use kshana_time::{Instant, NS_PER_DAY};
use kshana_western::{Aspect, Sign};

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// Longest transit window, in days.
const TRANSIT_MAX_DAYS: i128 = 800;

/// One secondary progressed body.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ProgressedBodyRecord {
    pub name: String,
    /// Progressed tropical longitude of date, degrees.
    pub longitude: f64,
    /// Degrees per ephemeris day, which is degrees per year of life.
    pub speed: f64,
}

/// The secondary progressed chart at one real instant.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ProgressedRecord {
    /// The ephemeris instant whose sky this is: birth + elapsed / 365.2422.
    pub instant: ExactInstant,
    /// Progressed bodies in chart order.
    pub bodies: Vec<ProgressedBodyRecord>,
    /// Progressed Sun − natal Sun, degrees.
    pub solar_arc: f64,
    /// Progressed Midheaven: natal MC + solar arc.
    pub mc: f64,
    /// Progressed Ascendant for that MC at the natal latitude.
    pub asc: f64,
    /// Progressed Moon − progressed Sun, degrees.
    pub moon_phase_deg: f64,
    /// Rudhyar's lunation phase: "New", "Crescent", "First Quarter", "Gibbous", "Full",
    /// "Disseminating", "Last Quarter" or "Balsamic".
    pub lunation_phase: String,
}

/// A progressed or directed point at an exact aspect to a natal point.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DirectionHitRecord {
    /// The real instant, exact to a microsecond.
    pub at: ExactInstant,
    /// "progressed" (secondary progressed Sun or Moon) or "solar_arc".
    pub kind: String,
    /// The moving point: "Sun" or "Moon" for progressions; any natal point for solar arc.
    pub moving: String,
    /// The natal point reached: a body name, "Ascendant" or "MC".
    pub natal: String,
    /// "conjunction", "opposition", "trine", "square" or "sextile".
    pub aspect: String,
    /// Retrograde motion of the progressed body at that instant; `None` for solar arc,
    /// whose arc only grows.
    pub retrograde: Option<bool>,
}

/// The profected year and month in force at an instant.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ProfectionRecord {
    /// Completed years of life: solar returns since birth.
    pub age: u32,
    pub year_sign: String,
    /// Traditional domicile ruler of the year sign.
    pub year_lord: String,
    pub year_start: ExactInstant,
    pub year_end: ExactInstant,
    /// Month of the profected year, 0–11.
    pub month: u8,
    pub month_sign: String,
    pub month_lord: String,
    pub month_start: ExactInstant,
    pub month_end: ExactInstant,
}

/// One firdaria period: a planetary sub-period or an undivided node period.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct FirdariaRecord {
    /// Lord of the major period.
    pub lord: String,
    /// Lord of the sub-period; `None` for the node periods.
    pub sub: Option<String>,
    pub start: ExactInstant,
    pub end: ExactInstant,
}

/// One zodiacal releasing period.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ZrRecord {
    /// 1 (general) to 4.
    pub level: u8,
    pub sign: String,
    pub start: ExactInstant,
    pub end: ExactInstant,
    /// True when the period begins with the loosing of the bond.
    pub loosing: bool,
    /// True when the sign is angular (1st, 4th, 7th, 10th) from the Lot of Fortune.
    pub peak: bool,
}

/// A transiting body at an exact aspect to a natal point.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitHitRecord {
    pub at: ExactInstant,
    /// Chart name of the transiting body.
    pub body: String,
    /// The natal point hit: a body name, "Ascendant" or "MC".
    pub natal_point: String,
    /// "conjunction", "opposition", "trine", "square" or "sextile".
    pub aspect: String,
    pub retrograde: bool,
    /// Half-width of the solver's final bracket, nanoseconds.
    pub half_width_ns: u64,
}

fn err(e: DirectionsError) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

fn observer(latitude_deg: f64, longitude_deg: f64) -> Observer {
    Observer {
        latitude_deg,
        longitude_deg,
        height_m: 0.0,
    }
}

/// The Ptolemaic five.
fn major_aspects() -> Vec<Aspect> {
    Aspect::ALL
        .iter()
        .copied()
        .filter(|a| a.is_major())
        .collect()
}

fn sign_name(s: Sign) -> String {
    format!("{s:?}")
}

#[uniffi::export]
impl Engine {
    /// The secondary progressed chart at `at`: a day for a tropical year, angles by the
    /// solar-arc MC with the Ascendant at the natal latitude.
    pub fn progressions(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
    ) -> Result<ProgressedRecord, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let p = progressions(&self.spk, &natal, latitude_deg, at.into()).map_err(err)?;
        Ok(ProgressedRecord {
            instant: p.instant.into(),
            bodies: p
                .bodies
                .iter()
                .map(|b| ProgressedBodyRecord {
                    name: b.name.to_string(),
                    longitude: b.longitude,
                    speed: b.speed,
                })
                .collect(),
            solar_arc: p.solar_arc,
            mc: p.mc,
            asc: p.asc,
            moon_phase_deg: p.moon_phase_deg,
            lunation_phase: p.lunation_phase.to_string(),
        })
    }

    /// Exact progressed contacts in `[from, to]` (clamped to the birth): the secondary
    /// progressed Sun and Moon in the Ptolemaic aspects to every natal point (bodies,
    /// Ascendant, MC), plus every solar arc direction to a natal point. Sorted by time.
    pub fn progressed_hits(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
    ) -> Result<Vec<DirectionHitRecord>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let from_i = Instant::from(from).max(natal.instant);
        let to_i: Instant = to.into();
        let mut out = Vec::new();
        let points = natal_points(&natal);
        for (name, body) in [("Sun", Body::Sun), ("Moon", Body::Moon)] {
            for p in &points {
                for aspect in major_aspects() {
                    let hits = progressed_aspect_instants(
                        &self.spk,
                        &natal,
                        body,
                        p.longitude,
                        aspect,
                        from_i,
                        to_i,
                    )
                    .map_err(err)?;
                    for t in hits {
                        let speed = apparent(&self.spk, body, progressed_instant(natal.instant, t))
                            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?
                            .speed;
                        out.push(DirectionHitRecord {
                            at: t.into(),
                            kind: "progressed".into(),
                            moving: name.into(),
                            natal: p.name.into(),
                            aspect: aspect.name().into(),
                            retrograde: Some(speed < 0.0),
                        });
                    }
                }
            }
        }
        for h in solar_arc_hits(&self.spk, &natal, from_i, to_i).map_err(err)? {
            out.push(DirectionHitRecord {
                at: h.at.into(),
                kind: "solar_arc".into(),
                moving: h.directed.into(),
                natal: h.natal.into(),
                aspect: h.aspect.name().into(),
                retrograde: None,
            });
        }
        out.sort_by_key(|h| (h.at.tai_seconds, h.at.nanos));
        Ok(out)
    }

    /// Annual and monthly profections at `at`: whole signs from the natal Ascendant,
    /// one a year between exact solar returns, twelve equal months within the year.
    pub fn profections(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
    ) -> Result<ProfectionRecord, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let p = profections(&self.spk, &natal, at.into()).map_err(err)?;
        Ok(ProfectionRecord {
            age: p.age,
            year_sign: sign_name(p.year_sign),
            year_lord: format!("{:?}", p.year_lord),
            year_start: p.year_start.into(),
            year_end: p.year_end.into(),
            month: p.month,
            month_sign: sign_name(p.month_sign),
            month_lord: format!("{:?}", p.month_lord),
            month_start: p.month_start.into(),
            month_end: p.month_end.into(),
        })
    }

    /// The 75-year firdaria cycle from birth, sub-periods expanded: 51 contiguous
    /// periods of 365.25-day years. Sect is the Sun's altitude at birth at the given
    /// place. The ayanamsa does not enter (the technique is tropical) and is accepted
    /// only so that every timing call takes the same arguments.
    pub fn firdaria(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
    ) -> Result<Vec<FirdariaRecord>, KshanaError> {
        let _ = ayanamsa;
        let obs = observer(latitude_deg, longitude_deg);
        let is_day = is_day_at(&self.spk, self.eop.as_ref(), &obs, birth.into()).map_err(err)?;
        Ok(firdaria(birth.into(), is_day)
            .into_iter()
            .map(|p| FirdariaRecord {
                lord: p.lord.name().to_string(),
                sub: p.sub.map(|s| s.name().to_string()),
                start: p.start.into(),
                end: p.end.into(),
            })
            .collect())
    }

    /// Zodiacal releasing from the Lot of Fortune (`lot` = "fortune") or of Spirit
    /// ("spirit"), `levels` deep (1–4), every period starting before `until`, in
    /// pre-order (each parent before its sub-periods). The lots come from the natal
    /// Ascendant, Sun and Moon by sect; peaks are counted from Fortune for both lots,
    /// as Valens does.
    #[allow(clippy::too_many_arguments)]
    pub fn zodiacal_releasing(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lot: String,
        levels: u8,
        until: ExactInstant,
    ) -> Result<Vec<ZrRecord>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let obs = observer(latitude_deg, longitude_deg);
        let is_day = is_day_at(&self.spk, self.eop.as_ref(), &obs, natal.instant).map_err(err)?;
        let l = lots(
            natal.cusps[0],
            natal.bodies[0].tropical,
            natal.bodies[1].tropical,
            is_day,
        );
        let from = match lot.to_ascii_lowercase().as_str() {
            "fortune" => l.fortune,
            "spirit" => l.spirit,
            other => {
                return Err(KshanaError::Engine(format!(
                    "unknown lot {other:?}: use \"fortune\" or \"spirit\""
                )))
            }
        };
        Ok(zodiacal_releasing_from(
            Sign::of(from),
            Sign::of(l.fortune),
            natal.instant,
            levels,
            until.into(),
        )
        .into_iter()
        .map(|p| ZrRecord {
            level: p.level,
            sign: sign_name(p.sign),
            start: p.start.into(),
            end: p.end.into(),
            loosing: p.loosing,
            peak: p.peak,
        })
        .collect())
    }

    /// Exact instants in `[from, to]` when the Moon returns to its natal tropical
    /// longitude (about every 27.32 days).
    pub fn lunar_returns(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
    ) -> Result<Vec<ExactInstant>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        Ok(
            lunar_returns(&self.spk, natal.bodies[1].tropical, from.into(), to.into())
                .map_err(err)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    /// Exact transit contacts in `[from, to]` (at most 800 days): each of `bodies`
    /// (chart names such as "Saturn"; an empty list means the classical seven) in the
    /// Ptolemaic aspects to every natal point (bodies, Ascendant, MC). Sorted by time.
    #[allow(clippy::too_many_arguments)]
    pub fn transit_hits(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
        bodies: Vec<String>,
    ) -> Result<Vec<TransitHitRecord>, KshanaError> {
        let from_i: Instant = from.into();
        let to_i: Instant = to.into();
        if to_i.ns_since(from_i) > TRANSIT_MAX_DAYS * NS_PER_DAY {
            return Err(KshanaError::Engine(format!(
                "transit windows are at most {TRANSIT_MAX_DAYS} days; ask for the next window"
            )));
        }
        let moving: Vec<Body> = if bodies.is_empty() {
            BODIES.iter().take(7).map(|(_, b)| *b).collect()
        } else {
            bodies
                .iter()
                .map(|n| {
                    BODIES
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case(n))
                        .map(|(_, b)| *b)
                        .ok_or_else(|| KshanaError::Engine(format!("unknown body {n:?}")))
                })
                .collect::<Result<_, _>>()?
        };
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let hits = transit_hits(&self.spk, &natal, &moving, &major_aspects(), from_i, to_i)
            .map_err(err)?;
        Ok(hits
            .into_iter()
            .map(|h| TransitHitRecord {
                at: h.at.into(),
                body: h.body_name.into(),
                natal_point: h.natal_point.into(),
                aspect: h.aspect.name().into(),
                retrograde: h.retrograde,
                half_width_ns: h.half_width_ns.max(0) as u64,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    fn key(i: ExactInstant) -> (i64, u32) {
        (i.tai_seconds, i.nanos)
    }

    fn u(y: i64, mo: u32, d: u32, h: u32, mi: u32) -> ExactInstant {
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
    }

    #[test]
    fn directions_cross_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let birth = u(1990, 10, 29, 12, 6);
        let (lat, lon) = (9.9312, 76.2673);
        let ay = AyanamsaChoice::Lahiri;
        let at = u(2026, 10, 16, 0, 0);

        // progressions: about 36 years, so about 36 days and 36 degrees of arc
        let pr = engine.progressions(birth, lat, lon, ay, at).unwrap();
        assert_eq!(pr.bodies.len(), 10);
        assert_eq!(pr.bodies[0].name, "Sun");
        let days = (pr.instant.tai_seconds - birth.tai_seconds) as f64 / 86_400.0;
        assert!((35.0..37.0).contains(&days), "{days}");
        assert!((34.0..38.0).contains(&pr.solar_arc), "{}", pr.solar_arc);
        assert!((0.0..360.0).contains(&pr.mc) && (0.0..360.0).contains(&pr.asc));
        assert!((0.0..360.0).contains(&pr.moon_phase_deg));
        assert!(!pr.lunation_phase.is_empty());

        // progressed and directed contacts over three years
        let hits = engine
            .progressed_hits(
                birth,
                lat,
                lon,
                ay,
                u(2024, 1, 1, 0, 0),
                u(2027, 1, 1, 0, 0),
            )
            .unwrap();
        assert!(!hits.is_empty());
        assert!(hits.windows(2).all(|w| key(w[0].at) <= key(w[1].at)));
        assert!(hits
            .iter()
            .any(|h| h.kind == "progressed" && h.moving == "Moon"));
        assert!(hits.iter().any(|h| h.kind == "solar_arc"));
        assert!(hits
            .iter()
            .all(|h| (h.kind == "solar_arc") == h.retrograde.is_none()));
        assert!(hits
            .iter()
            .all(|h| !h.natal.is_empty() && !h.aspect.is_empty()));

        // profections: the 36th birthday is 2026-10-29, so still age 35 in mid October
        let pf = engine.profections(birth, lat, lon, ay, at).unwrap();
        assert!((35..=36).contains(&pf.age), "{}", pf.age);
        assert!(pf.month <= 11);
        assert!(key(pf.year_start) <= key(at) && key(at) < key(pf.year_end));
        assert!(key(pf.month_start) <= key(at) && key(at) < key(pf.month_end));
        assert!(!pf.year_sign.is_empty() && !pf.year_lord.is_empty());
        assert!(!pf.month_sign.is_empty() && !pf.month_lord.is_empty());

        // firdaria: a day birth, 51 contiguous periods
        let f = engine.firdaria(birth, lat, lon, ay).unwrap();
        assert_eq!(f.len(), 51);
        assert_eq!(f[0].lord, "Sun");
        assert_eq!(f[0].sub.as_deref(), Some("Sun"));
        assert_eq!(key(f[0].start), key(birth));
        assert!(f.windows(2).all(|w| key(w[0].end) == key(w[1].start)));
        assert_eq!(f.iter().filter(|p| p.sub.is_none()).count(), 2);

        // zodiacal releasing from both lots
        let until = u(2030, 1, 1, 0, 0);
        let zr = engine
            .zodiacal_releasing(birth, lat, lon, ay, "fortune".into(), 2, until)
            .unwrap();
        assert!(zr.iter().any(|p| p.level == 1));
        assert!(zr.iter().any(|p| p.level == 2));
        assert_eq!(key(zr[0].start), key(birth));
        assert!(zr.windows(2).all(|w| key(w[0].start) <= key(w[1].start)));
        let l1: Vec<&ZrRecord> = zr.iter().filter(|p| p.level == 1).collect();
        assert!(l1.windows(2).all(|w| key(w[0].end) == key(w[1].start)));
        assert!(l1[0].peak);
        let spirit = engine
            .zodiacal_releasing(birth, lat, lon, ay, "spirit".into(), 1, until)
            .unwrap();
        assert!(!spirit.is_empty());
        assert!(engine
            .zodiacal_releasing(birth, lat, lon, ay, "fate".into(), 1, until)
            .is_err());

        // lunar returns: three or four in a hundred days
        let lr = engine
            .lunar_returns(
                birth,
                lat,
                lon,
                ay,
                u(2026, 1, 1, 0, 0),
                u(2026, 4, 11, 0, 0),
            )
            .unwrap();
        assert!((3..=4).contains(&lr.len()), "{}", lr.len());
        assert!(lr.windows(2).all(|w| key(w[0]) < key(w[1])));

        // transits: Saturn over two years, some of them retrograde
        let th = engine
            .transit_hits(
                birth,
                lat,
                lon,
                ay,
                u(2024, 1, 1, 0, 0),
                u(2026, 1, 1, 0, 0),
                vec!["Saturn".into()],
            )
            .unwrap();
        assert!(!th.is_empty());
        assert!(th.windows(2).all(|w| key(w[0].at) <= key(w[1].at)));
        assert!(th
            .iter()
            .all(|h| h.body == "Saturn" && h.half_width_ns <= 1_000));
        assert!(th.iter().any(|h| h.retrograde));
        assert!(engine
            .transit_hits(
                birth,
                lat,
                lon,
                ay,
                u(2020, 1, 1, 0, 0),
                u(2026, 1, 1, 0, 0),
                vec![]
            )
            .is_err());
        assert!(engine
            .transit_hits(
                birth,
                lat,
                lon,
                ay,
                u(2025, 1, 1, 0, 0),
                u(2025, 2, 1, 0, 0),
                vec!["Vulcan".into()]
            )
            .is_err());
    }
}
