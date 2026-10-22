//! Shadbala, Bhava bala and avasthas over the FFI.

use kshana_bala::words;
use kshana_sky::Observer;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// One graha's six-fold strength in virupas.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct GrahaBalaRecord {
    /// English key ("Mars"); `graha_name` is what to show.
    pub graha: String,
    pub graha_name: String,
    /// The motion's name and meaning in the reader's language (`motion` is the key).
    pub motion_name: Option<String>,
    pub motion_meaning: Option<String>,
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
    pub lord_name: String,
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
    /// What to show: the graha and the three states in the reader's language, and
    /// their one-line meanings joined with " · ". The fields above are English keys.
    pub graha_name: String,
    pub baladi_name: String,
    pub jagradadi_name: String,
    pub deeptadi_name: String,
    pub meaning: String,
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
    /// Shadbala, Bhava bala and avasthas of the natal chart. `words`, `wars`, the time
    /// lords, `strongest`, `weakest` and the `*_name` / meaning fields are in `lang`;
    /// `graha`, `lord`, `motion` and the avastha fields stay English keys.
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
        let s = kshana_bala::shadbala_in(&self.spk, self.eop.as_ref(), &obs, &ch, ayanamsa.into(), lang)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let g = |l: kshana_vedic::DashaLord| format!("{l:?}");
        let name = |l: kshana_vedic::DashaLord| words::graha(lang, l).to_string();
        Ok(ShadbalaRecord {
            grahas: s
                .grahas
                .iter()
                .map(|b| GrahaBalaRecord {
                    graha: g(b.graha),
                    graha_name: name(b.graha),
                    motion_name: b.motion.map(|m| m.name_in(lang).to_string()),
                    motion_meaning: b.motion.map(|m| m.meaning_in(lang).to_string()),
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
                    lord_name: name(b.lord),
                    lord_bala: b.lord_bala,
                    dig: b.dig,
                    drishti: b.drishti,
                    total: b.total,
                    rupas: b.rupas,
                    ratio: b.ratio,
                })
                .collect(),
            year_lord: name(s.lords.year),
            month_lord: name(s.lords.month),
            day_lord: name(s.lords.day),
            hora_lord: name(s.lords.hora),
            wars: s
                .wars
                .iter()
                .map(|w| {
                    let (win, lose, sep, vir) = (name(w.winner), name(w.loser), w.separation_deg, w.virupas);
                    match lang {
                        kshana_rules::Lang::En => format!("{win} defeats {lose} ({sep:.2}° apart, {vir:.1} virupas)"),
                        kshana_rules::Lang::Ml => format!("ഗ്രഹയുദ്ധം: {win} ജയിക്കുന്നു, {lose} തോൽക്കുന്നു ({sep:.2}° അകലം, {vir:.1} വിരൂപ)"),
                        kshana_rules::Lang::Hi => format!("ग्रहयुद्ध: {win} विजयी, {lose} पराजित ({sep:.2}° की दूरी, {vir:.1} विरूप)"),
                    }
                })
                .collect(),
            strongest: name(s.strongest),
            weakest: name(s.weakest),
            avasthas: kshana_bala::avasthas(&ch)
                .iter()
                .map(|a| AvasthaRecord {
                    graha: g(a.graha),
                    baladi: format!("{:?}", a.baladi),
                    jagradadi: format!("{:?}", a.jagradadi),
                    deeptadi: format!("{:?}", a.deeptadi),
                    graha_name: name(a.graha),
                    baladi_name: a.baladi.name_in(lang).to_string(),
                    jagradadi_name: a.jagradadi.name_in(lang).to_string(),
                    deeptadi_name: a.deeptadi.name_in(lang).to_string(),
                    meaning: [a.baladi.meaning_in(lang), a.jagradadi.meaning_in(lang), a.deeptadi.meaning_in(lang)].join(" · "),
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
        assert!(s.grahas[0].words[0].starts_with("Sun at "), "{}", s.grahas[0].words[0]);

        // Malayalam and Hindi: every shown string is in the script; the keys stay English.
        let latin = |t: &str| {
            t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.len() >= 3)
        };
        for lang in ["ml", "hi"] {
            let r = engine
                .shadbala(birth, 9.9312, 76.2673, AyanamsaChoice::Lahiri, lang.into())
                .unwrap();
            let mut shown: Vec<String> = Vec::new();
            for g in &r.grahas {
                shown.extend(g.words.iter().cloned());
                shown.push(g.graha_name.clone());
                shown.extend(g.motion_name.iter().cloned());
                shown.extend(g.motion_meaning.iter().cloned());
            }
            for b in &r.bhavas {
                shown.push(b.lord_name.clone());
            }
            for a in &r.avasthas {
                shown.extend([a.graha_name.clone(), a.baladi_name.clone(), a.jagradadi_name.clone(), a.deeptadi_name.clone(), a.meaning.clone()]);
            }
            shown.extend(r.wars.iter().cloned());
            shown.extend([r.year_lord.clone(), r.month_lord.clone(), r.day_lord.clone(), r.hora_lord.clone(), r.strongest.clone(), r.weakest.clone()]);
            for t in &shown {
                assert!(!latin(t), "{lang}: {t}");
            }
            assert_eq!(r.grahas[0].graha, "Sun");
            assert_eq!(r.avasthas[0].graha, s.avasthas[0].graha);
            assert_eq!(r.grahas.iter().map(|g| g.motion.clone()).collect::<Vec<_>>(), s.grahas.iter().map(|g| g.motion.clone()).collect::<Vec<_>>());
        }
    }
}
