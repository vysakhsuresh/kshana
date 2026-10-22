//! Tajika varshaphala over the FFI: the year ahead from the exact sidereal solar return.

use kshana_sky::Observer;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// One Panchavargeeya bala row.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PanchavargeeyaRecord {
    pub planet: String,
    pub kshetra: f64,
    pub uchcha: f64,
    pub hadda: f64,
    pub drekkana: f64,
    pub navamsa: f64,
    pub total: f64,
    pub vishwa: f64,
}

/// One year-lord candidate.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct YearLordCandidateRecord {
    pub lord: String,
    pub offices: Vec<String>,
    pub vishwa: f64,
    pub aspects_lagna: bool,
}

/// One saham.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SahamRecord {
    pub name: String,
    pub key: String,
    pub longitude: f64,
    pub house: u8,
    pub meaning: String,
}

/// One Tajika yoga.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TajikaYogaRecord {
    pub kind: String,
    pub name: String,
    pub planets: Vec<String>,
    pub exact: bool,
    pub words: String,
}

/// One Mudda period with its antardashas.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct MuddaRecord {
    pub lord: String,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub antar_lords: Vec<String>,
    pub antar_starts: Vec<ExactInstant>,
    pub antar_ends: Vec<ExactInstant>,
}

/// One planet of the annual sky.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AnnualBodyRecord {
    pub name: String,
    pub sidereal: f64,
    pub house: u8,
    pub speed: f64,
}

/// The year ahead.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct VarshaphalaRecord {
    pub year: i64,
    pub age: u32,
    pub instant: ExactInstant,
    pub next_return: ExactInstant,
    pub day: bool,
    pub lagna_sidereal: f64,
    pub cusps_sidereal: Vec<f64>,
    pub bodies: Vec<AnnualBodyRecord>,
    pub muntha_sign: u8,
    pub muntha_house: u8,
    pub muntha_longitude: f64,
    pub muntha_lord: String,
    pub balas: Vec<PanchavargeeyaRecord>,
    pub candidates: Vec<YearLordCandidateRecord>,
    pub varshesha: String,
    pub reasoning: Vec<String>,
    pub sahams: Vec<SahamRecord>,
    pub yogas: Vec<TajikaYogaRecord>,
    pub mudda: Vec<MuddaRecord>,
}

#[uniffi::export]
impl Engine {
    /// The Tajika annual chart for calendar `year`, cast at the person's current place.
    #[allow(clippy::too_many_arguments)]
    pub fn varshaphala(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        year: i64,
        here_latitude_deg: f64,
        here_longitude_deg: f64,
        lang: String,
    ) -> Result<VarshaphalaRecord, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let obs = Observer {
            latitude_deg: here_latitude_deg,
            longitude_deg: here_longitude_deg,
            height_m: 0.0,
        };
        let v = kshana_tajika::varshaphala(
            &self.spk,
            self.eop.as_ref(),
            &natal,
            &obs,
            ayanamsa.into(),
            year,
            lang,
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let g = |l: kshana_vedic::DashaLord| kshana_speak::lexicon::graha(l, lang).to_string();
        Ok(VarshaphalaRecord {
            year: v.year,
            age: v.age,
            instant: v.instant.into(),
            next_return: v.next_return.into(),
            day: v.day,
            lagna_sidereal: v.sky.lagna,
            cusps_sidereal: v.chart.cusps_sidereal.to_vec(),
            bodies: v
                .chart
                .bodies
                .iter()
                .map(|b| AnnualBodyRecord {
                    name: kshana_speak::lexicon::name(b.name, lang).to_string(),
                    sidereal: b.sidereal,
                    house: b.house,
                    speed: b.speed,
                })
                .collect(),
            muntha_sign: v.muntha.0 as u8,
            muntha_house: v.muntha.1,
            muntha_longitude: v.muntha_longitude,
            muntha_lord: g(v.muntha_lord),
            balas: v
                .balas
                .iter()
                .map(|b| PanchavargeeyaRecord {
                    planet: g(b.planet),
                    kshetra: b.kshetra,
                    uchcha: b.uchcha,
                    hadda: b.hadda,
                    drekkana: b.drekkana,
                    navamsa: b.navamsa,
                    total: b.total,
                    vishwa: b.vishwa,
                })
                .collect(),
            candidates: v
                .candidates
                .iter()
                .map(|c| YearLordCandidateRecord {
                    lord: g(c.lord),
                    offices: c.offices.iter().map(|o| o.name_in(lang).to_string()).collect(),
                    vishwa: c.vishwa,
                    aspects_lagna: c.aspects_lagna,
                })
                .collect(),
            varshesha: g(v.varshesha),
            reasoning: v.reasoning.clone(),
            sahams: v
                .sahams
                .iter()
                .map(|s| SahamRecord {
                    name: kshana_tajika::saham_in(s.name, lang).0.to_string(),
                    key: s.name.to_string(),
                    longitude: s.longitude,
                    house: s.house,
                    meaning: kshana_tajika::saham_in(s.name, lang).1.to_string(),
                })
                .collect(),
            yogas: v
                .yogas
                .iter()
                .map(|y| TajikaYogaRecord {
                    kind: format!("{:?}", y.kind),
                    name: y.name.to_string(),
                    planets: y.planets.iter().map(|p| g(*p)).collect(),
                    exact: y.exact,
                    words: y.words.clone(),
                })
                .collect(),
            mudda: v
                .mudda
                .iter()
                .map(|m| MuddaRecord {
                    lord: g(m.lord),
                    start: m.start.into(),
                    end: m.end.into(),
                    antar_lords: m.antardashas.iter().map(|p| g(p.lord)).collect(),
                    antar_starts: m.antardashas.iter().map(|p| p.start.into()).collect(),
                    antar_ends: m.antardashas.iter().map(|p| p.end.into()).collect(),
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn varshaphala_crosses_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let birth = instant_from_utc(UtcFields {
            year: 1990,
            month: 10,
            day: 29,
            hour: 12,
            minute: 6,
            second: 0,
            nanos: 0,
        })
        .unwrap();
        let v = engine
            .varshaphala(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                2026,
                12.97,
                77.59,
                "en".into(),
            )
            .unwrap();
        assert_eq!(v.age, 36);
        assert_eq!(v.bodies.len(), 10);
        assert!(v.sahams.len() >= 30);
        assert!(!v.mudda.is_empty() && !v.reasoning.is_empty());
        assert!(v.candidates.iter().any(|c| c.lord == v.varshesha));
    }
}
