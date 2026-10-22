//! The life layer of the FFI: plain-words reading, life map, daily forecast, transits,
//! compatibility, calendar, karakas, naming and the glossary.

use kshana_chart::{compute, Chart, ChartInput};
use kshana_sky::Observer;
use kshana_time::Instant;
use kshana_vedic::DashaLord;

use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// The natal chart for a birth at a place (Placidus, no topocentric Moon).
pub(crate) fn natal_chart(
    engine: &Engine,
    birth: ExactInstant,
    latitude_deg: f64,
    longitude_deg: f64,
    ayanamsa: AyanamsaChoice,
) -> Result<Chart, KshanaError> {
    let input = ChartInput {
        instant: birth.into(),
        observer: Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        },
        system: kshana_houses::HouseSystem::Placidus,
        ayanamsa: ayanamsa.into(),
        topocentric_moon: false,
    };
    compute(&engine.spk, engine.eop.as_ref(), &input).map_err(|e| match e {
        kshana_chart::ChartError::Houses(kshana_houses::HouseError::PolarLatitude) => {
            KshanaError::PolarLatitude
        }
        other => KshanaError::Engine(format!("{other:?}")),
    })
}

fn lord(l: DashaLord) -> String {
    format!("{l:?}")
}

/// One sentence with its evidence.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SentenceRecord {
    pub text: String,
    pub why: Vec<String>,
    pub confidence: f64,
    pub rules: Vec<String>,
}

/// A titled group of sentences.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SectionRecord {
    /// Stable key, never shown: `who`, `now`, `next`, or the area's English name.
    pub key: String,
    /// For an area: `promised` or `earned`; empty otherwise. Never shown.
    pub mark: String,
    pub title: String,
    pub summary: String,
    pub sentences: Vec<SentenceRecord>,
}

fn section(s: kshana_speak::Section) -> SectionRecord {
    SectionRecord {
        key: s.key,
        mark: s.mark,
        title: s.title,
        summary: s.summary,
        sentences: s.sentences.into_iter().map(sentence).collect(),
    }
}

fn sentence(s: kshana_speak::Sentence) -> SentenceRecord {
    SentenceRecord {
        text: s.text,
        why: s.why,
        confidence: s.confidence,
        rules: s.rules,
    }
}

/// One year of the life map.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct YearRowRecord {
    pub age: u32,
    pub start: ExactInstant,
    pub maha: String,
    pub antar: String,
    /// Nine area scores in `AREAS` order: Career, Marriage, Children, Wealth, Health, Property, Education, Family, Foreign.
    pub areas: Vec<f64>,
}

/// A window of hours in a day.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct HourWindowRecord {
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub reason: String,
    pub avoid: bool,
}

/// The day's forecast for one person.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DailyRecord {
    pub sunrise: Option<ExactInstant>,
    pub sunset: Option<ExactInstant>,
    pub next_sunrise: Option<ExactInstant>,
    pub tara: String,
    pub tara_good: bool,
    pub chandra_house: u8,
    pub chandra_good: bool,
    pub moon_nakshatra: String,
    pub windows: Vec<HourWindowRecord>,
    pub headline: String,
}

/// One transit in force.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitRecord {
    pub body: String,
    pub natal: String,
    /// "over", "aspect" or "from_moon".
    pub kind: String,
    pub house: u8,
    pub longitude: f64,
    pub separation: f64,
}

/// Sade Sati status.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SadeSatiRecord {
    /// 0 none, 1 rising, 2 peak, 3 setting.
    pub phase: u8,
    pub saturn_sign: u8,
    pub moon_sign: u8,
    pub phase_start: Option<ExactInstant>,
    pub phase_end: Option<ExactInstant>,
    pub next_start: Option<ExactInstant>,
}

/// A coming sidereal ingress.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct IngressRecord {
    pub instant: ExactInstant,
    pub body: String,
    pub sign: u8,
}

/// Transits now and next.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct TransitReport {
    pub transits: Vec<TransitRecord>,
    pub sade_sati: SadeSatiRecord,
    pub upcoming: Vec<IngressRecord>,
}

/// One kuta score.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KutaRecord {
    pub name: String,
    pub points: f64,
    pub max: f64,
    pub detail: String,
}

/// Mangal dosha of one chart.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct MangalRecord {
    pub from_lagna: u8,
    pub from_moon: u8,
    pub from_venus: u8,
    pub present: bool,
    pub cancellations: Vec<String>,
    pub effective: bool,
}

/// One synastry aspect.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SynastryRecord {
    pub a: String,
    pub b: String,
    pub aspect: String,
    pub orb: f64,
    pub tone: i8,
    pub weight: f64,
}

/// A full compatibility report.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct CompatibilityRecord {
    pub kutas: Vec<KutaRecord>,
    pub total: f64,
    pub verdict: String,
    pub nadi_dosha: bool,
    pub bhakoot_dosha: bool,
    pub mangal_a: MangalRecord,
    pub mangal_b: MangalRecord,
    pub synastry: Vec<SynastryRecord>,
    pub sentences: Vec<SentenceRecord>,
}

/// A calendar observance.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ObservanceRecord {
    pub name: String,
    pub key: String,
    pub exact: ExactInstant,
    pub observed_sunrise: Option<ExactInstant>,
    pub note: String,
}

/// One Jaimini karaka.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KarakaRecord {
    pub rank: u8,
    pub name: String,
    pub graha: String,
    pub degree_in_sign: f64,
    pub meaning: String,
}

/// Naming syllable.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct NamingRecord {
    pub nakshatra: String,
    pub pada: u8,
    pub syllable: String,
}

/// Glossary entry.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct GlossaryRecord {
    pub term: String,
    pub meaning: String,
    pub tradition: String,
    pub tradition_key: String,
}

fn mangal(m: &kshana_rules::MangalDosha) -> MangalRecord {
    MangalRecord {
        from_lagna: m.from_lagna,
        from_moon: m.from_moon,
        from_venus: m.from_venus,
        present: m.present,
        cancellations: m.cancellations.clone(),
        effective: m.effective,
    }
}

#[uniffi::export]
impl Engine {
    /// The plain-words reading: who you are, right now, each life area, the years ahead.
    pub fn plain_reading(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        lang: String,
    ) -> Result<Vec<SectionRecord>, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        Ok(
            kshana_speak::reading_in(kshana_rules::Lang::parse(&lang), &self.spk, &ch, at.into(), ayanamsa.into())
                .into_iter()
                .map(section)
                .collect(),
        )
    }

    /// The life map: every year from birth to `years`, the periods and area scores.
    pub fn life_map(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        years: u32,
    ) -> Result<Vec<YearRowRecord>, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        Ok(kshana_forecast::life_map(&ch, years.min(120))
            .into_iter()
            .map(|r| YearRowRecord {
                age: r.age,
                start: r.start.into(),
                maha: lord(r.maha),
                antar: lord(r.antar),
                areas: r.areas.to_vec(),
            })
            .collect())
    }

    /// The day for one person at the place they are now.
    #[allow(clippy::too_many_arguments)]
    pub fn daily(
        &self,
        birth: ExactInstant,
        birth_latitude_deg: f64,
        birth_longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        here_latitude_deg: f64,
        here_longitude_deg: f64,
        lang: String,
    ) -> Result<DailyRecord, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ch = natal_chart(
            self,
            birth,
            birth_latitude_deg,
            birth_longitude_deg,
            ayanamsa,
        )?;
        let obs = Observer {
            latitude_deg: here_latitude_deg,
            longitude_deg: here_longitude_deg,
            height_m: 0.0,
        };
        let d = kshana_forecast::daily(
            &self.spk,
            self.eop.as_ref(),
            &obs,
            &ch,
            at.into(),
            ayanamsa.into(),
            lang,
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(DailyRecord {
            sunrise: d.day.as_ref().map(|x| x.sunrise.into()),
            sunset: d.day.as_ref().map(|x| x.sunset.into()),
            next_sunrise: d.day.as_ref().map(|x| x.next_sunrise.into()),
            tara: d.tara.0,
            tara_good: d.tara.1,
            chandra_house: d.chandra_bala.0,
            chandra_good: d.chandra_bala.1,
            moon_nakshatra: d.moon_nakshatra,
            windows: d
                .windows
                .into_iter()
                .map(|w| HourWindowRecord {
                    start: w.start.into(),
                    end: w.end.into(),
                    reason: w.reason,
                    avoid: w.avoid,
                })
                .collect(),
            headline: d.headline,
        })
    }

    /// Transits in force, Sade Sati, and the ingresses of the next `days`.
    pub fn transits(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        days: u32,
    ) -> Result<TransitReport, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let ay: kshana_vedic::Ayanamsa = ayanamsa.into();
        let at_i: Instant = at.into();
        let err = |e: kshana_ephem::SpkError| KshanaError::Engine(format!("{e:?}"));
        let ts = kshana_forecast::transits_at(&self.spk, &ch, at_i, ay).map_err(err)?;
        let ss = kshana_forecast::sade_sati(&self.spk, &ch, at_i, ay).map_err(err)?;
        let up = kshana_forecast::upcoming_transits(&self.spk, ay, at_i, days.min(3660) as i64)
            .map_err(err)?;
        Ok(TransitReport {
            transits: ts
                .into_iter()
                .map(|t| TransitRecord {
                    body: t.body.to_string(),
                    natal: t.natal.to_string(),
                    kind: match t.kind {
                        kshana_forecast::TransitKind::OverNatal => "over",
                        kshana_forecast::TransitKind::AspectingNatal => "aspect",
                        kshana_forecast::TransitKind::FromMoon => "from_moon",
                    }
                    .to_string(),
                    house: t.house,
                    longitude: t.longitude,
                    separation: t.separation,
                })
                .collect(),
            sade_sati: SadeSatiRecord {
                phase: ss.phase,
                saturn_sign: ss.saturn_sign,
                moon_sign: ss.moon_sign,
                phase_start: ss.phase_start.map(Into::into),
                phase_end: ss.phase_end.map(Into::into),
                next_start: ss.next_start.map(Into::into),
            },
            upcoming: up
                .into_iter()
                .map(|(t, b, s)| IngressRecord {
                    instant: t.into(),
                    body: b.to_string(),
                    sign: s,
                })
                .collect(),
        })
    }

    /// Compatibility of two charts: Ashtakoota, Mangal dosha both sides, synastry, words.
    #[allow(clippy::too_many_arguments)]
    pub fn compatibility(
        &self,
        birth_a: ExactInstant,
        latitude_a: f64,
        longitude_a: f64,
        birth_b: ExactInstant,
        latitude_b: f64,
        longitude_b: f64,
        ayanamsa: AyanamsaChoice,
    ) -> Result<CompatibilityRecord, KshanaError> {
        let a = natal_chart(self, birth_a, latitude_a, longitude_a, ayanamsa)?;
        let b = natal_chart(self, birth_b, latitude_b, longitude_b, ayanamsa)?;
        let ash = kshana_rules::ashtakoota(a.bodies[1].sidereal, b.bodies[1].sidereal);
        let fa = kshana_rules::Facts::from_chart(&a);
        let fb = kshana_rules::Facts::from_chart(&b);
        let ma = kshana_rules::mangal_dosha(&fa);
        let mb = kshana_rules::mangal_dosha(&fb);
        let syn = kshana_rules::synastry(&a, &b, 6.0);
        let words = kshana_speak::compatibility(&ash, &ma, &mb, &syn);
        Ok(CompatibilityRecord {
            kutas: ash
                .kutas
                .iter()
                .map(|k| KutaRecord {
                    name: k.name.to_string(),
                    points: k.points,
                    max: k.max,
                    detail: k.detail.clone(),
                })
                .collect(),
            total: ash.total,
            verdict: ash.verdict.to_string(),
            nadi_dosha: ash.nadi_dosha,
            bhakoot_dosha: ash.bhakoot_dosha,
            mangal_a: mangal(&ma),
            mangal_b: mangal(&mb),
            synastry: syn
                .into_iter()
                .take(24)
                .map(|x| SynastryRecord {
                    a: x.a.to_string(),
                    b: x.b.to_string(),
                    aspect: x.aspect.to_string(),
                    orb: x.orb,
                    tone: x.tone,
                    weight: x.weight,
                })
                .collect(),
            sentences: words.into_iter().map(sentence).collect(),
        })
    }

    /// The observance calendar for `days` from `from` at a place.
    pub fn calendar(
        &self,
        from: ExactInstant,
        days: u32,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<Vec<ObservanceRecord>, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let obs = Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        };
        let o = kshana_forecast::observances(
            &self.spk,
            self.eop.as_ref(),
            &obs,
            ayanamsa.into(),
            from.into(),
            days.min(400),
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(o.into_iter()
            .map(|x| ObservanceRecord {
                name: kshana_forecast::observance_words(x.name, lang).to_string(),
                key: x.name.to_string(),
                exact: x.exact.into(),
                observed_sunrise: x.observed_sunrise.map(Into::into),
                note: kshana_forecast::observance_words(x.note, lang).to_string(),
            })
            .collect())
    }

    /// Jaimini chara karakas of a chart.
    pub fn karakas(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<Vec<KarakaRecord>, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let sid: [f64; 7] = std::array::from_fn(|i| ch.bodies[i].sidereal);
        Ok(kshana_vedic::chara_karakas(sid)
            .iter()
            .map(|k| KarakaRecord {
                rank: k.rank as u8,
                name: kshana_vedic::jaimini::karaka_name_in(k.rank, &lang).to_string(),
                graha: lord(k.graha),
                degree_in_sign: k.degree_in_sign,
                meaning: kshana_vedic::jaimini::karaka_meaning_in(k.rank, &lang).to_string(),
            })
            .collect())
    }
}

/// Naming syllable for a sidereal Moon longitude.
#[uniffi::export]
pub fn naming(moon_sidereal: f64, lang: String) -> NamingRecord {
    let lang = kshana_rules::Lang::parse(&lang);
    let (n, p, s) = kshana_vedic::naming_for(moon_sidereal);
    let (ml, hi) = kshana_vedic::syllable_native(kshana_vedic::nakshatra(moon_sidereal).index, p);
    NamingRecord {
        nakshatra: kshana_speak::lexicon::name(n, lang).to_string(),
        pada: p,
        syllable: lang.pick([s, ml, hi]).to_string(),
    }
}

/// The glossary.
#[uniffi::export]
pub fn glossary(lang: String) -> Vec<GlossaryRecord> {
    let lang = kshana_rules::Lang::parse(&lang);
    kshana_speak::glossary::glossary_in(lang)
        .into_iter()
        .map(|g| GlossaryRecord {
            term: g.term.to_string(),
            meaning: g.meaning.to_string(),
            tradition: kshana_speak::glossary::tradition_name(g.tradition, lang).to_string(),
            tradition_key: g.tradition.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn life_layer_through_the_boundary() {
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
        let birth = u(1990, 10, 29, 17, 36);
        let at = u(2026, 10, 15, 9, 0);
        let r = engine
            .plain_reading(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, at, "en".into())
            .unwrap();
        assert_eq!(r.len(), 2 + kshana_rules::AREAS.len() + 1);
        let lm = engine
            .life_map(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, 60)
            .unwrap();
        assert_eq!(lm.len(), 61);
        let d = engine
            .daily(
                birth,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                at,
                12.97,
                77.59,
                "hi".into(),
            )
            .unwrap();
        assert!(d.sunrise.is_some() && !d.windows.is_empty());
        let t = engine
            .transits(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, at, 90)
            .unwrap();
        assert!(!t.transits.is_empty() && !t.upcoming.is_empty());
        let c = engine
            .compatibility(
                birth,
                9.9312,
                76.2673,
                u(1992, 4, 3, 6, 15),
                12.97,
                77.59,
                AyanamsaChoice::Lahiri,
            )
            .unwrap();
        assert_eq!(c.kutas.len(), 8);
        assert!(c.total <= 36.0 && !c.sentences.is_empty());
        let cal = engine
            .calendar(at, 40, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "ml".into())
            .unwrap();
        assert!(cal.len() >= 8);
        let k = engine
            .karakas(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(k.len(), 7);
        assert_eq!(naming(0.5, "en".into()).syllable, "Chu");
        assert_eq!(naming(0.5, "hi".into()).syllable, "चू");
        assert_eq!(naming(0.5, "ml".into()).nakshatra, "അശ്വതി");
        assert!(glossary("en".into()).len() >= 50);
    }
}
