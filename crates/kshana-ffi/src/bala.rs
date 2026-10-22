//! Shadbala, Bhava bala and avasthas over the FFI.

use kshana_sky::Observer;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// One graha's six-fold strength in virupas.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct GrahaBalaRecord {
    pub graha: String,
    pub uchcha: f64,
    pub saptavargaja: f64,
    pub ojhayugma: f64,
    pub kendradi: f64,
    pub drekkana: f64,
    pub sthana: f64,
    pub dig: f64,
    pub nathonnata: f64,
    pub paksha: f64,
    pub tribhaga: f64,
    pub abda: f64,
    pub masa: f64,
    pub vara: f64,
    pub hora: f64,
    pub ayana: f64,
    pub yuddha: f64,
    pub kala: f64,
    pub chesta: f64,
    pub motion: Option<String>,
    pub naisargika: f64,
    pub drik: f64,
    pub total_virupas: f64,
    pub rupas: f64,
    pub required: f64,
    pub ratio: f64,
    pub ishta: f64,
    pub kashta: f64,
    pub words: Vec<String>,
}

/// One house's strength.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct BhavaBalaRecord {
    pub house: u8,
    pub lord: String,
    pub lord_bala: f64,
    pub dig: f64,
    pub drishti: f64,
    pub total: f64,
    pub rupas: f64,
    pub ratio: f64,
}

/// One graha's avasthas.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AvasthaRecord {
    pub graha: String,
    pub baladi: String,
    pub jagradadi: String,
    pub deeptadi: String,
    pub baladi_key: String,
    pub jagradadi_key: String,
    pub deeptadi_key: String,
}

/// The whole strength report.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ShadbalaRecord {
    pub grahas: Vec<GrahaBalaRecord>,
    pub bhavas: Vec<BhavaBalaRecord>,
    pub year_lord: String,
    pub month_lord: String,
    pub day_lord: String,
    pub hora_lord: String,
    pub wars: Vec<String>,
    pub strongest: String,
    pub weakest: String,
    pub avasthas: Vec<AvasthaRecord>,
}

#[uniffi::export]
impl Engine {
    /// Shadbala, Bhava bala and avasthas of the natal chart.
    pub fn shadbala(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<ShadbalaRecord, KshanaError> {
        let lang = kshana_rules::Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let obs = Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        };
        let s = kshana_bala::shadbala(&self.spk, self.eop.as_ref(), &obs, &ch, ayanamsa.into())
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let g = |l: kshana_vedic::DashaLord| kshana_speak::lexicon::graha(l, lang).to_string();
        Ok(ShadbalaRecord {
            grahas: s
                .grahas
                .iter()
                .map(|b| GrahaBalaRecord {
                    graha: g(b.graha),
                    uchcha: b.sthana.uchcha,
                    saptavargaja: b.sthana.saptavargaja,
                    ojhayugma: b.sthana.ojhayugma,
                    kendradi: b.sthana.kendradi,
                    drekkana: b.sthana.drekkana,
                    sthana: b.sthana.total,
                    dig: b.dig,
                    nathonnata: b.kala.nathonnata,
                    paksha: b.kala.paksha,
                    tribhaga: b.kala.tribhaga,
                    abda: b.kala.abda,
                    masa: b.kala.masa,
                    vara: b.kala.vara,
                    hora: b.kala.hora,
                    ayana: b.kala.ayana,
                    yuddha: b.kala.yuddha,
                    kala: b.kala.total,
                    chesta: b.chesta,
                    motion: b.motion.map(|m| m.name().to_string()),
                    naisargika: b.naisargika,
                    drik: b.drik,
                    total_virupas: b.total_virupas,
                    rupas: b.rupas,
                    required: b.required,
                    ratio: b.ratio,
                    ishta: b.ishta,
                    kashta: b.kashta,
                    words: b.words.clone(),
                })
                .collect(),
            bhavas: s
                .bhavas
                .iter()
                .map(|b| BhavaBalaRecord {
                    house: b.house,
                    lord: g(b.lord),
                    lord_bala: b.lord_bala,
                    dig: b.dig,
                    drishti: b.drishti,
                    total: b.total,
                    rupas: b.rupas,
                    ratio: b.ratio,
                })
                .collect(),
            year_lord: g(s.lords.year),
            month_lord: g(s.lords.month),
            day_lord: g(s.lords.day),
            hora_lord: g(s.lords.hora),
            wars: s
                .wars
                .iter()
                .map(|w| {
                    match lang {
                        kshana_rules::Lang::En => format!(
                            "{:?} defeats {:?} ({:.2}° apart, {:.1} virupas)",
                            w.winner, w.loser, w.separation_deg, w.virupas
                        ),
                        kshana_rules::Lang::Ml => format!(
                            "{} {}-നെ ജയിക്കുന്നു ({:.2}° അകലം, {:.1} വിരൂപ)",
                            g(w.winner), g(w.loser), w.separation_deg, w.virupas
                        ),
                        kshana_rules::Lang::Hi => format!(
                            "{} {} को हराता है ({:.2}° दूरी, {:.1} विरूप)",
                            g(w.winner), g(w.loser), w.separation_deg, w.virupas
                        ),
                    }
                })
                .collect(),
            strongest: g(s.strongest),
            weakest: g(s.weakest),
            avasthas: kshana_bala::avasthas(&ch)
                .iter()
                .map(|a| AvasthaRecord {
                    graha: g(a.graha),
                    baladi: a.baladi.name_in(lang).to_string(),
                    jagradadi: a.jagradadi.name_in(lang).to_string(),
                    deeptadi: a.deeptadi.name_in(lang).to_string(),
                    baladi_key: format!("{:?}", a.baladi),
                    jagradadi_key: format!("{:?}", a.jagradadi),
                    deeptadi_key: format!("{:?}", a.deeptadi),
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
    fn shadbala_crosses_the_boundary() {
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
        let s = engine
            .shadbala(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(s.grahas.len(), 7);
        assert_eq!(s.bhavas.len(), 12);
        assert!(s
            .grahas
            .iter()
            .all(|g| g.total_virupas > 0.0 && g.ratio > 0.0));
        assert_eq!(s.avasthas.len(), 7);
    }
}
