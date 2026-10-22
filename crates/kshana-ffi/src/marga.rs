//! The Prasna Marga suite and the Jaimini extras over the FFI: sphutas, ayurdaya,
//! Ashtamangala and the prasna Arudha, rasi drishti, argala, karakamsa and the padas,
//! dasha sandhi and chidra, Vimshopaka bala. Thin wrappers over `kshana_marga`; every
//! number comes from the kernel through the natal chart and `kshana_day`.

use kshana_marga as marga;
use kshana_rules::{Facts, Lang};
use kshana_sky::Observer;
use kshana_vedic::DashaLord;

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

fn g(l: DashaLord) -> String {
    format!("{l:?}")
}

fn err<E: std::fmt::Debug>(e: E) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

fn sign_name(s: u8, lang: Lang) -> String {
    if lang != Lang::En {
        return kshana_rules::yogas::words::rasi(lang, s as usize).to_string();
    }
    marga::SIGN_NAMES[(s % 12) as usize].to_string()
}

fn nakshatra_name(n: usize, lang: Lang) -> String {
    if lang != Lang::En {
        return kshana_rules::yogas::words::star(lang, n).to_string();
    }
    kshana_vedic::panchanga::NAKSHATRA_NAMES[n % 27].to_string()
}

fn parse_lord(name: &str) -> Result<DashaLord, KshanaError> {
    Ok(match name.trim().to_ascii_lowercase().as_str() {
        "sun" | "surya" => DashaLord::Sun,
        "moon" | "chandra" => DashaLord::Moon,
        "mars" | "kuja" | "mangala" => DashaLord::Mars,
        "mercury" | "budha" => DashaLord::Mercury,
        "jupiter" | "guru" | "brihaspati" => DashaLord::Jupiter,
        "venus" | "shukra" | "sukra" => DashaLord::Venus,
        "saturn" | "shani" | "sani" => DashaLord::Saturn,
        "rahu" => DashaLord::Rahu,
        "ketu" => DashaLord::Ketu,
        other => return Err(KshanaError::Engine(format!("unknown dasha lord: {other}"))),
    })
}

/// A sensitive point.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SphutaRecord {
    pub name: String,
    pub longitude: f64,
    pub sign: u8,
    pub sign_name: String,
    pub nakshatra: u32,
    pub nakshatra_name: String,
    pub pada: u8,
    pub lord: String,
    pub star_lord: String,
    pub words: Vec<String>,
}

impl SphutaRecord {
    fn of(s: &marga::Sphuta, lang: Lang) -> Self {
        SphutaRecord {
            name: s.name.to_string(),
            longitude: s.longitude,
            sign: s.sign,
            sign_name: sign_name(s.sign, lang),
            nakshatra: s.nakshatra as u32,
            nakshatra_name: nakshatra_name(s.nakshatra, lang),
            pada: s.pada,
            lord: g(s.lord),
            star_lord: g(s.star_lord),
            words: s.words.clone(),
        }
    }
}

/// The Yogi point and its planets.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct YogiRecord {
    pub point: SphutaRecord,
    pub yogi: String,
    pub sahayogi: String,
    pub avayogi_point: f64,
    pub avayogi: String,
    pub words: Vec<String>,
}

/// Every Prasna Marga sphuta of a chart, with the Gulika behind them.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SphutaReportRecord {
    pub gulika_convention: String,
    pub gulika_at: ExactInstant,
    pub gulika_night: bool,
    pub gulika_sidereal: f64,
    pub sphutas: Vec<SphutaRecord>,
    pub yogi: YogiRecord,
    pub bhrigu_bindu: SphutaRecord,
}

/// One harana or bharana.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AyurReductionRecord {
    pub name: String,
    pub factor: f64,
    pub years: f64,
    pub reason: String,
}

impl From<&marga::Reduction> for AyurReductionRecord {
    fn from(r: &marga::Reduction) -> Self {
        AyurReductionRecord {
            name: r.name.to_string(),
            factor: r.factor,
            years: r.years,
            reason: r.reason.clone(),
        }
    }
}

/// One graha's (or the lagna's) share.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AyurContributionRecord {
    pub graha: String,
    pub base_years: f64,
    pub years: f64,
    pub reductions: Vec<AyurReductionRecord>,
    pub words: Vec<String>,
}

impl From<&marga::Contribution> for AyurContributionRecord {
    fn from(c: &marga::Contribution) -> Self {
        AyurContributionRecord {
            graha: c.graha.to_string(),
            base_years: c.base_years,
            years: c.years,
            reductions: c.reductions.iter().map(Into::into).collect(),
            words: c.words.clone(),
        }
    }
}

/// One method's reckoning inside an averaged result.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AyurComponentRecord {
    pub method: String,
    pub years: f64,
    pub raw_years: f64,
    pub contributions: Vec<AyurContributionRecord>,
    pub krurodaya: Option<AyurReductionRecord>,
    pub words: Vec<String>,
}

/// The longevity exercise. `caveat` is the sentence the app must show with it.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AyurdayaRecord {
    pub method: String,
    pub years: f64,
    pub raw_years: f64,
    pub selection: String,
    pub contributions: Vec<AyurContributionRecord>,
    pub krurodaya: Option<AyurReductionRecord>,
    pub words: Vec<String>,
    pub caveat: String,
    pub components: Vec<AyurComponentRecord>,
}

/// An Ashtamangala number read.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AshtamangalaRecord {
    pub number: u32,
    pub digits: Vec<u8>,
    pub planets: Vec<Option<String>>,
    pub arudha_sign: u8,
    pub arudha_sign_name: String,
    pub nakshatra: Option<u32>,
    pub nakshatra_name: Option<String>,
    pub indications: Vec<String>,
}

/// The Kerala prasna's sphutas and derived signs.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PrasnaSphutaRecord {
    pub lagna: SphutaRecord,
    pub chandra: SphutaRecord,
    pub arudha: SphutaRecord,
    pub arudha_sign: u8,
    pub veethi_sign: u8,
    pub veethi_sign_name: String,
    pub chatra_sign: u8,
    pub chatra_sign_name: String,
    pub words: Vec<String>,
}

/// A sign's rasi drishti.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct RasiDrishtiRecord {
    pub sign: u8,
    pub sign_name: String,
    pub aspects: Vec<u8>,
}

/// One argala house with its obstruction.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ArgalaPairRecord {
    pub argala_house: u8,
    pub virodha_house: u8,
    pub secondary: bool,
    pub by: Vec<String>,
    pub obstructed_by: Vec<String>,
    pub net: i8,
}

/// Argala on one house.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ArgalaRecord {
    pub house: u8,
    pub sign: u8,
    pub sign_name: String,
    pub by: Vec<String>,
    pub obstructed_by: Vec<String>,
    pub net: i8,
    pub pairs: Vec<ArgalaPairRecord>,
    pub words: Vec<String>,
}

/// The karakamsa.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KarakamsaRecord {
    pub atmakaraka: String,
    pub degree_in_sign: f64,
    pub sign: u8,
    pub sign_name: String,
    pub navamsa_lagna: u8,
    pub occupants: Vec<String>,
    pub words: Vec<String>,
}

/// An arudha pada.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PadaRecord {
    pub name: String,
    pub house: u8,
    pub sign: u8,
    pub sign_name: String,
    pub lord: String,
    pub rasi_house: u8,
    pub second: Vec<String>,
    pub eleventh: Vec<String>,
    pub seventh: Vec<String>,
    pub words: Vec<String>,
}

impl PadaRecord {
    fn of(p: &marga::Pada, lang: Lang) -> Self {
        PadaRecord {
            name: p.name.clone(),
            house: p.house,
            sign: p.sign,
            sign_name: sign_name(p.sign, lang),
            lord: g(p.lord),
            rasi_house: p.rasi_house,
            second: p.second.iter().map(|x| g(*x)).collect(),
            eleventh: p.eleventh.iter().map(|x| g(*x)).collect(),
            seventh: p.seventh.iter().map(|x| g(*x)).collect(),
            words: p.words.clone(),
        }
    }
}

/// The Jaimini extras of a chart.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct JaiminiRecord {
    pub rasi_drishti: Vec<RasiDrishtiRecord>,
    pub argala: Vec<ArgalaRecord>,
    pub karakamsa: KarakamsaRecord,
    pub pada_lagna: PadaRecord,
    pub upapada: PadaRecord,
}

/// One dasha junction window.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SandhiRecord {
    pub level: u8,
    pub from_lord: String,
    pub to_lord: String,
    pub boundary: ExactInstant,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub rule: String,
}

/// One chidra lord with its reason.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct ChidraRecord {
    pub lord: String,
    pub reason: String,
}

/// One graha's Vimshopaka bala.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct VimshopakaRecord {
    pub graha: String,
    pub shadvarga: f64,
    pub saptavarga: f64,
    pub dashavarga: f64,
    pub shodashavarga: f64,
    pub grade: String,
    pub words: Vec<String>,
}

fn obs(latitude_deg: f64, longitude_deg: f64) -> Observer {
    Observer {
        latitude_deg,
        longitude_deg,
        height_m: 0.0,
    }
}

fn yogi_record(y: &marga::Yogi, lang: Lang) -> YogiRecord {
    YogiRecord {
        point: SphutaRecord::of(&y.point, lang),
        yogi: g(y.yogi),
        sahayogi: g(y.sahayogi),
        avayogi_point: y.avayogi_point,
        avayogi: g(y.avayogi),
        words: y.words.clone(),
    }
}

fn component(a: &marga::Ayurdaya) -> AyurComponentRecord {
    AyurComponentRecord {
        method: a.method.name().to_string(),
        years: a.years,
        raw_years: a.raw_years,
        contributions: a.contributions.iter().map(Into::into).collect(),
        krurodaya: a.krurodaya.as_ref().map(Into::into),
        words: a.words.clone(),
    }
}

/// Read an Ashtamangala number (the three cowrie remainders, digits 1–8).
#[uniffi::export]
pub fn ashtamangala_number(number: u32, lang: String) -> AshtamangalaRecord {
    let lang = Lang::parse(&lang);
    let a = marga::ashtamangala::ashtamangala_in(number, lang);
    AshtamangalaRecord {
        number: a.number,
        digits: a.digits.clone(),
        planets: a.planets.iter().map(|p| p.map(g)).collect(),
        arudha_sign: a.arudha_sign,
        arudha_sign_name: sign_name(a.arudha_sign, lang),
        nakshatra: a.nakshatra.map(|n| n as u32),
        nakshatra_name: a.nakshatra.map(|n| nakshatra_name(n, lang)),
        indications: a.indications,
    }
}

#[uniffi::export]
impl Engine {
    /// The Prasna Marga sphutas of the chart at `at`: Trisphuta, Chatussphuta,
    /// Panchasphuta, Prana, Deha and Mrityu sphutas, Sookshma Trisphuta, Beeja and
    /// Kshetra, the Yogi point and the Bhrigu Bindu, with Gulika from the solar day.
    pub fn prasna_marga_sphutas(
        &self,
        at: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<SphutaReportRecord, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, at, latitude_deg, longitude_deg, ayanamsa)?;
        let o = obs(latitude_deg, longitude_deg);
        let r = marga::sphuta::sphuta_report_in(&self.spk, self.eop.as_ref(), &o, &ch, ayanamsa.into(), lang)
            .map_err(err)?;
        Ok(SphutaReportRecord {
            gulika_convention: format!("{:?}", r.gulika.convention),
            gulika_at: r.gulika.at.into(),
            gulika_night: r.gulika.night,
            gulika_sidereal: r.gulika.sidereal,
            sphutas: r.sphutas.iter().map(|s| SphutaRecord::of(s, lang)).collect(),
            yogi: yogi_record(&r.yogi, lang),
            bhrigu_bindu: SphutaRecord::of(&r.bhrigu_bindu, lang),
        })
    }

    /// The classical ayurdaya of the natal chart, the method chosen by the Shadbala
    /// strengths of the lagna, the Sun and the Moon. The record's `caveat` must be
    /// shown with it: this is the text's arithmetic, not a prediction.
    pub fn ayurdaya_classical(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
    ) -> Result<AyurdayaRecord, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let o = obs(latitude_deg, longitude_deg);
        let a =
            marga::ayurdaya_with_shadbala(&self.spk, self.eop.as_ref(), &o, &ch, ayanamsa.into())
                .map_err(err)?;
        Ok(AyurdayaRecord {
            method: a.method.name().to_string(),
            years: a.years,
            raw_years: a.raw_years,
            selection: a.selection.clone(),
            contributions: a.contributions.iter().map(Into::into).collect(),
            krurodaya: a.krurodaya.as_ref().map(Into::into),
            words: a.words.clone(),
            caveat: marga::LONGEVITY_CAVEAT.to_string(),
            components: a.components.iter().map(component).collect(),
        })
    }

    /// The Kerala prasna's lagna, Chandra and Arudha sphutas with the Veethi and
    /// Chatra rasis, for a question at `at` with the gold piece on `arudha_sign`
    /// (0 = Aries).
    pub fn prasna_arudha_sphutas(
        &self,
        at: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        arudha_sign: u8,
        lang: String,
    ) -> Result<PrasnaSphutaRecord, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, at, latitude_deg, longitude_deg, ayanamsa)?;
        let p = marga::ashtamangala::prasna_sphutas_in(&ch, arudha_sign, lang).map_err(err)?;
        Ok(PrasnaSphutaRecord {
            lagna: SphutaRecord::of(&p.lagna, lang),
            chandra: SphutaRecord::of(&p.chandra, lang),
            arudha: SphutaRecord::of(&p.arudha, lang),
            arudha_sign: p.arudha_sign,
            veethi_sign: p.veethi_sign,
            veethi_sign_name: sign_name(p.veethi_sign, lang),
            chatra_sign: p.chatra_sign,
            chatra_sign_name: sign_name(p.chatra_sign, lang),
            words: p.words,
        })
    }

    /// Rasi drishti, argala on the twelve houses, the karakamsa, the Pada lagna and
    /// the Upapada of the natal chart.
    pub fn jaimini_extras(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<JaiminiRecord, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        let k = marga::jaimini::karakamsa_in(&f, lang);
        Ok(JaiminiRecord {
            rasi_drishti: (0..12u8)
                .map(|s| RasiDrishtiRecord {
                    sign: s,
                    sign_name: sign_name(s, lang),
                    aspects: marga::rasi_drishti(s),
                })
                .collect(),
            argala: marga::jaimini::argala_in(&f, lang)
                .iter()
                .map(|a| ArgalaRecord {
                    house: a.house,
                    sign: a.sign,
                    sign_name: sign_name(a.sign, lang),
                    by: a.by.iter().map(|x| g(*x)).collect(),
                    obstructed_by: a.obstructed_by.iter().map(|x| g(*x)).collect(),
                    net: a.net,
                    pairs: a
                        .pairs
                        .iter()
                        .map(|p| ArgalaPairRecord {
                            argala_house: p.argala_house,
                            virodha_house: p.virodha_house,
                            secondary: p.secondary,
                            by: p.by.iter().map(|x| g(*x)).collect(),
                            obstructed_by: p.obstructed_by.iter().map(|x| g(*x)).collect(),
                            net: p.net,
                        })
                        .collect(),
                    words: a.words.clone(),
                })
                .collect(),
            karakamsa: KarakamsaRecord {
                atmakaraka: g(k.atmakaraka),
                degree_in_sign: k.degree_in_sign,
                sign: k.sign,
                sign_name: sign_name(k.sign, lang),
                navamsa_lagna: k.navamsa_lagna,
                occupants: k.occupants.iter().map(|x| g(*x)).collect(),
                words: k.words.clone(),
            },
            pada_lagna: PadaRecord::of(&marga::jaimini::pada_of_in(&f, 1, lang), lang),
            upapada: PadaRecord::of(&marga::jaimini::pada_of_in(&f, 12, lang), lang),
        })
    }

    /// Vimshottari sandhi windows (level 1 and 2) whose boundary lies in `[from, to]`,
    /// under the last-and-first sub-period rule, or a fraction of each adjoining
    /// period when `fraction` is given (0.125 for an eighth).
    #[allow(clippy::too_many_arguments)]
    pub fn dasha_sandhi_windows(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        to: ExactInstant,
        fraction: Option<f64>,
        lang: String,
    ) -> Result<Vec<SandhiRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let moon = ch.bodies[1].sidereal;
        let rule = match fraction {
            Some(f) => marga::SandhiRule::Fraction(f),
            None => marga::SandhiRule::SubPeriod,
        };
        Ok(marga::sandhi_windows_with(
            moon,
            birth.into(),
            from.into(),
            to.into(),
            rule,
            kshana_vedic::dasha::YEAR_365_25_NS,
        )
        .iter()
        .map(|s| SandhiRecord {
            level: s.level,
            from_lord: g(s.from_lord),
            to_lord: g(s.to_lord),
            boundary: s.boundary.into(),
            start: s.start.into(),
            end: s.end.into(),
            rule: s.rule.name_in(lang).to_string(),
        })
        .collect())
    }

    /// The chidra lords of a dasha lord (named "Sun" … "Ketu") in the natal chart.
    pub fn dasha_chidra_lords(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        dasha_lord: String,
        lang: String,
    ) -> Result<Vec<ChidraRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        let lord = parse_lord(&dasha_lord)?;
        Ok(marga::sandhi::chidra_lords_in(&f, lord, lang)
            .into_iter()
            .map(|(l, reason)| ChidraRecord { lord: g(l), reason })
            .collect())
    }

    /// Vimshopaka bala of the seven grahas in the four varga schemes.
    pub fn vimshopaka_bala(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        lang: String,
    ) -> Result<Vec<VimshopakaRecord>, KshanaError> {
        let lang = Lang::parse(&lang);
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        Ok(marga::vimshopaka::vimshopaka_in(&ch, lang)
            .map_err(err)?
            .iter()
            .map(|v| VimshopakaRecord {
                graha: g(v.graha),
                shadvarga: v.shadvarga,
                saptavarga: v.saptavarga,
                dashavarga: v.dashavarga,
                shodashavarga: v.shodashavarga,
                grade: v.grade.to_string(),
                words: v.words.clone(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    fn engine() -> Option<std::sync::Arc<Engine>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return None;
        }
        Engine::open(p.to_string_lossy().into_owned(), None).ok()
    }

    fn birth() -> ExactInstant {
        instant_from_utc(UtcFields {
            year: 1990,
            month: 10,
            day: 29,
            hour: 12,
            minute: 6,
            second: 0,
            nanos: 0,
        })
        .unwrap()
    }

    #[test]
    fn marga_crosses_the_boundary() {
        let Some(e) = engine() else { return };
        let b = birth();
        let s = e
            .prasna_marga_sphutas(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(s.sphutas.len(), 11);
        assert_eq!(s.sphutas[0].name, "Trisphuta");
        assert!(s.sphutas.iter().all(|x| !x.words.is_empty()));
        assert!(!s.yogi.yogi.is_empty());
        let a = e
            .ayurdaya_classical(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri)
            .unwrap();
        assert!((0.0..=120.0).contains(&a.years));
        assert_eq!(a.caveat, kshana_marga::LONGEVITY_CAVEAT);
        assert_eq!(a.words[0], a.caveat);
        let p = e
            .prasna_arudha_sphutas(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, 5, "en".into())
            .unwrap();
        assert_eq!(p.arudha_sign, 5);
        assert_eq!(p.arudha.sign, 5);
        let j = e
            .jaimini_extras(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(j.rasi_drishti.len(), 12);
        assert_eq!(j.argala.len(), 12);
        assert_eq!(j.pada_lagna.name, "Pada lagna");
        let to = instant_from_utc(UtcFields {
            year: 2030,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        })
        .unwrap();
        let w = e
            .dasha_sandhi_windows(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, b, to, None, "en".into())
            .unwrap();
        assert!(w.iter().any(|x| x.level == 1));
        assert!(w.iter().all(|x| x.rule == "last and first sub-periods"));
        let w8 = e
            .dasha_sandhi_windows(
                b,
                9.9312,
                76.2673,
                AyanamsaChoice::Lahiri,
                b,
                to,
                Some(0.125),
                "en".into(),
            )
            .unwrap();
        assert_eq!(w.len(), w8.len());
        let c = e
            .dasha_chidra_lords(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "Venus".into(), "en".into())
            .unwrap();
        assert!(c.len() >= 5);
        assert!(e
            .dasha_chidra_lords(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "Pluto".into(), "en".into())
            .is_err());
        let v = e
            .vimshopaka_bala(b, 9.9312, 76.2673, AyanamsaChoice::Lahiri, "en".into())
            .unwrap();
        assert_eq!(v.len(), 7);
        assert!(v.iter().all(|x| x.shodashavarga <= 20.0));
    }

    #[test]
    fn ashtamangala_is_pure() {
        let a = ashtamangala_number(831, "en".into());
        assert_eq!(a.digits, vec![8, 3, 1]);
        assert_eq!(a.planets[0].as_deref(), Some("Rahu"));
        assert_eq!(a.arudha_sign_name, "Gemini");
        assert_eq!(a.nakshatra_name.as_deref(), Some("Uttara Ashadha"));
        assert!(parse_lord("shani").is_ok());
    }
}
