//! The complete KP table and the Ashtakavarga reductions over the FFI, for the expert
//! pages (behind "Show the chart pages"). Nothing here is plain-language text: every
//! field is a chart term or a number.
//!
//! KP (K. S. Krishnamurti, KP Readers I–VI): every cusp and graha with its sign, star,
//! sub, sub-sub and sub-sub-sub lords; the significators of each house at the four
//! levels with Rahu/Ketu agency; the four-step chain and the cuspal interlink of every
//! cusp sub lord; the ruling planets of a moment and their overlap with each house's
//! significators. Ashtakavarga (BPHS; B. V. Raman, *Ashtakavarga System of
//! Prediction*): each graha's table raw, after trikona and ekadhipatya shodhana, its
//! rasi, graha and shodhya pindas, and the sensitive nakshatra and sign for transits.

use kshana_rules::kp::{
    cusp_links, house_significators, node_agency, ruling_planets, rp_overlap, weekday_lord,
    Step,
};
use kshana_rules::Facts;
use kshana_sky::Observer;
use kshana_time::{Instant, Utc};
use kshana_vedic::{kp_lords, DashaLord};

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

fn n(g: DashaLord) -> String {
    format!("{g:?}")
}

fn names(gs: &[DashaLord]) -> Vec<String> {
    gs.iter().map(|g| n(*g)).collect()
}

/// One step of the four-step chain.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpStepRecord {
    /// 1 planet, 2 its star lord, 3 its sub lord, 4 the sub lord's star lord.
    pub step: u8,
    pub planet: String,
    /// Houses occupied and owned (a node: also its sign lord's and conjoined grahas').
    pub houses: Vec<u8>,
}

fn steps(s: &[Step; 4]) -> Vec<KpStepRecord> {
    s.iter()
        .enumerate()
        .map(|(i, s)| KpStepRecord {
            step: (i + 1) as u8,
            planet: n(s.planet),
            houses: s.houses.clone(),
        })
        .collect()
}

/// A cusp or a graha with its chain of lords.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpPointRecord {
    /// "1" … "12" for cusps, the graha's name otherwise.
    pub point: String,
    pub sidereal_deg: f64,
    /// Placidus house holding it (the cusp's own number for cusps).
    pub house: u8,
    pub retrograde: bool,
    pub sign_lord: String,
    pub star_lord: String,
    pub sub_lord: String,
    pub sub_sub_lord: String,
    pub sub_sub_sub_lord: String,
    /// 1–249 number of the sub.
    pub sub_number: u16,
    /// Four-step chain: of the cusp's sub lord for a cusp, of the graha itself otherwise.
    pub steps: Vec<KpStepRecord>,
    /// For a cusp: the houses its sub lord signifies through its star lord (the
    /// decisive set). For a graha: every house it signifies at any level.
    pub signifies: Vec<u8>,
}

/// One house's significators at the four levels.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpHouseRecord {
    pub house: u8,
    /// A: in the star of an occupant.
    pub star_of_occupants: Vec<String>,
    /// B: occupants.
    pub occupants: Vec<String>,
    /// C: in the star of the owner.
    pub star_of_owner: Vec<String>,
    /// D: the owner.
    pub owner: String,
    /// Rahu/Ketu standing for a planet in A–D.
    pub node_agents: Vec<String>,
    /// Strongest first.
    pub ordered: Vec<String>,
    /// Ruling planets at the asked moment that are among `ordered`, in RP order.
    pub ruling: Vec<String>,
}

/// What a node stands for.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpNodeRecord {
    pub node: String,
    pub conjoined: Vec<String>,
    pub aspecting: Vec<String>,
    pub sign_lord: String,
    pub star_lord: String,
}

/// Ruling planets of a moment.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpRulingRecord {
    pub at: ExactInstant,
    pub day_lord: String,
    /// Moon's sign, star, sub lords.
    pub moon: Vec<String>,
    /// Lagna's sign, star, sub lords.
    pub lagna: Vec<String>,
    pub nodes: Vec<String>,
    /// Strongest first.
    pub ordered: Vec<String>,
    /// Ruling planets in the star of a retrograde planet.
    pub in_retro_star: Vec<String>,
}

/// The complete KP table.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct KpFullRecord {
    pub ayanamsa_deg: f64,
    pub cusps: Vec<KpPointRecord>,
    pub planets: Vec<KpPointRecord>,
    pub houses: Vec<KpHouseRecord>,
    pub nodes: Vec<KpNodeRecord>,
    pub ruling: KpRulingRecord,
    /// The conventions, one per line, with their sources.
    pub notes: Vec<String>,
}

/// One graha's Ashtakavarga through the reductions.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct AshtakaReductionRecord {
    pub planet: String,
    /// Bindus per sign, 0 = Aries, at each stage.
    pub bhinna: Vec<u8>,
    pub after_trikona: Vec<u8>,
    pub after_ekadhipatya: Vec<u8>,
    pub rasi_pinda: u32,
    pub graha_pinda: u32,
    pub shodhya_pinda: u32,
    /// Raw bindus in the 8th sign from the graha.
    pub eighth_bindus: u8,
    /// Shodhya pinda × eighth bindus.
    pub product: u32,
    /// Sensitive nakshatra, 0 = Ashwini (its trines +9, +18 too).
    pub sensitive_nakshatra: u8,
    /// Sensitive sign, 0 = Aries.
    pub sensitive_sign: u8,
}

/// Weekday (0 = Sunday) of the Vedic day containing `at`: the local civil date of the
/// sunrise that began it, local time by longitude. Falls back to the local civil date
/// of `at` where the Sun does not rise.
fn vedic_weekday(engine: &Engine, obs: &Observer, at: Instant) -> u32 {
    let start = kshana_events::solar_day(&engine.spk, engine.eop.as_ref(), obs, at)
        .ok()
        .flatten()
        .map(|d| d.sunrise)
        .unwrap_or(at);
    let local = start.add_ns((obs.longitude_deg / 15.0 * 3_600.0 * 1e9) as i128);
    (Utc::from_instant(local).utc_day() + 4).rem_euclid(7) as u32
}

fn point(
    label: String,
    lon: f64,
    house: u8,
    retrograde: bool,
    st: &[Step; 4],
    signifies: Vec<u8>,
) -> KpPointRecord {
    let k = kp_lords(lon);
    KpPointRecord {
        point: label,
        sidereal_deg: lon,
        house,
        retrograde,
        sign_lord: n(k.sign),
        star_lord: n(k.star),
        sub_lord: n(k.sub),
        sub_sub_lord: n(k.sub_sub),
        sub_sub_sub_lord: n(k.sub_sub_sub),
        sub_number: k.sub_number,
        steps: steps(st),
        signifies,
    }
}

const NOTES: &[&str] = &[
    "Placidus cusps; sub, sub-sub and sub-sub-sub lords divide each star in Vimshottari proportion from its own lord (Krishnamurti, KP Reader I).",
    "Significators: A in the star of an occupant, B occupant, C in the star of the owner, D owner, strongest first (KP Reader III).",
    "Rahu and Ketu stand for the grahas conjoined with them, then those aspecting them (Parashari aspects), then their sign lord, then their star lord; a node is placed before the planet it represents.",
    "Four steps: the planet, its star lord, its sub lord, the sub lord's star lord; houses are those occupied and owned. A cusp's sub lord decides the matter through its star lord's houses.",
    "Ruling planets: Lagna star, Lagna sign, Moon star, Moon sign, day lord (Krishnamurti's order, KP Reader VI), then Lagna and Moon sub lords; a node in the sign of, or with, a ruling planet joins it. The day runs sunrise to sunrise.",
];

#[uniffi::export]
impl Engine {
    /// The complete KP table of a natal chart, with the ruling planets at `at` for the
    /// place `(now_latitude_deg, now_longitude_deg)` and their overlap with each
    /// house's significators.
    #[allow(clippy::too_many_arguments)]
    pub fn kp_full(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
        now_latitude_deg: f64,
        now_longitude_deg: f64,
    ) -> Result<KpFullRecord, KshanaError> {
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        let moment = natal_chart(self, at, now_latitude_deg, now_longitude_deg, ayanamsa)?;
        let mf = Facts::from_chart(&moment);
        let obs = Observer {
            latitude_deg: now_latitude_deg,
            longitude_deg: now_longitude_deg,
            height_m: 0.0,
        };
        let rp = ruling_planets(&mf, weekday_lord(vedic_weekday(self, &obs, at.into())));
        let cusps = cusp_links(&f)
            .iter()
            .map(|c| {
                point(
                    c.house.to_string(),
                    c.sidereal,
                    c.house,
                    false,
                    &c.steps,
                    c.through_star.clone(),
                )
            })
            .collect();
        let planets = f
            .planets
            .iter()
            .map(|p| {
                let sig: Vec<u8> = kshana_rules::significators(&f, p.graha)
                    .iter()
                    .map(|s| s.house)
                    .collect();
                point(
                    n(p.graha),
                    p.sidereal,
                    p.house,
                    p.retrograde,
                    &kshana_rules::kp::four_step(&f, p.graha),
                    sig,
                )
            })
            .collect();
        let houses = (1..=12u8)
            .map(|h| {
                let s = house_significators(&f, h);
                KpHouseRecord {
                    house: h,
                    star_of_occupants: names(&s.star_of_occupants),
                    occupants: names(&s.occupants),
                    star_of_owner: names(&s.star_of_owner),
                    owner: n(s.owner),
                    node_agents: names(&s.node_agents),
                    ordered: names(&s.ordered),
                    ruling: names(&rp_overlap(&f, &rp, &[h])),
                }
            })
            .collect();
        let nodes = [DashaLord::Rahu, DashaLord::Ketu]
            .iter()
            .map(|&g| {
                let a = node_agency(&f, g);
                KpNodeRecord {
                    node: n(g),
                    conjoined: names(&a.conjoined),
                    aspecting: names(&a.aspecting),
                    sign_lord: n(a.sign_lord),
                    star_lord: n(a.star_lord),
                }
            })
            .collect();
        Ok(KpFullRecord {
            ayanamsa_deg: ch.ayanamsa_deg,
            cusps,
            planets,
            houses,
            nodes,
            ruling: KpRulingRecord {
                at,
                day_lord: n(rp.day_lord),
                moon: names(&rp.moon),
                lagna: names(&rp.lagna),
                nodes: names(&rp.nodes),
                ordered: names(&rp.ordered),
                in_retro_star: names(&rp.in_retro_star),
            },
            notes: NOTES.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Ashtakavarga of a natal chart through trikona and ekadhipatya shodhana, with
    /// the pindas and sensitive points, Sun through Saturn.
    pub fn ashtakavarga_reductions(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
    ) -> Result<Vec<AshtakaReductionRecord>, KshanaError> {
        use DashaLord::*;
        let ch = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        let f = Facts::from_chart(&ch);
        let order = [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn];
        let p = kshana_vedic::AshtakavargaPositions {
            signs: [
                f.planet(Sun).sign,
                f.planet(Moon).sign,
                f.planet(Mars).sign,
                f.planet(Mercury).sign,
                f.planet(Jupiter).sign,
                f.planet(Venus).sign,
                f.planet(Saturn).sign,
                f.lagna_sign,
            ],
        };
        Ok(order
            .iter()
            .enumerate()
            .map(|(g, &lord)| {
                let r = kshana_vedic::reduce(&p, g);
                AshtakaReductionRecord {
                    planet: n(lord),
                    bhinna: r.bhinna.to_vec(),
                    after_trikona: r.trikona.to_vec(),
                    after_ekadhipatya: r.ekadhipatya.to_vec(),
                    rasi_pinda: r.pinda.rasi,
                    graha_pinda: r.pinda.graha,
                    shodhya_pinda: r.pinda.shodhya,
                    eighth_bindus: r.eighth_bindus,
                    product: r.product,
                    sensitive_nakshatra: r.nakshatra,
                    sensitive_sign: r.sign,
                }
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
            month: 2,
            day: 20,
            hour: 5,
            minute: 4,
            second: 0,
            nanos: 0,
        })
        .unwrap()
    }

    #[test]
    fn kp_full_crosses_the_boundary() {
        let Some(e) = engine() else { return };
        let (lat, lon) = (9.9312, 76.2673);
        let t = e
            .kp_full(birth(), lat, lon, AyanamsaChoice::Krishnamurti, birth(), lat, lon)
            .unwrap();
        assert_eq!(t.cusps.len(), 12);
        assert_eq!(t.planets.len(), 9);
        assert_eq!(t.houses.len(), 12);
        let c1 = &t.cusps[0];
        assert_eq!(
            (c1.sign_lord.as_str(), c1.star_lord.as_str(), c1.sub_lord.as_str(), c1.sub_sub_lord.as_str()),
            ("Mars", "Ketu", "Mercury", "Saturn")
        );
        assert_eq!(c1.sub_number, 9);
        assert_eq!(c1.signifies, vec![4, 8]);
        // Tuesday after sunrise: Mars rules the day
        assert_eq!(t.ruling.day_lord, "Mars");
        assert_eq!(t.ruling.ordered, vec!["Ketu", "Mars", "Jupiter", "Rahu", "Mercury", "Moon"]);
        assert_eq!(t.houses[9].ordered, vec!["Sun", "Jupiter", "Ketu", "Mercury", "Rahu", "Saturn"]);
        assert_eq!(t.houses[9].ruling, vec!["Ketu", "Jupiter", "Rahu", "Mercury"]);
        let sun = &t.planets[0];
        assert_eq!((sun.star_lord.as_str(), sun.sub_lord.as_str(), sun.sub_sub_sub_lord.as_str()), ("Rahu", "Rahu", "Moon"));
        assert!(!t.notes.is_empty());
    }

    #[test]
    fn day_lord_turns_at_sunrise_not_midnight() {
        let Some(e) = engine() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        // 1990-02-20 23:30 UTC = Wednesday 05:00 IST, before sunrise: still Tuesday
        let pre = instant_from_utc(UtcFields { year: 1990, month: 2, day: 20, hour: 23, minute: 30, second: 0, nanos: 0 }).unwrap();
        assert_eq!(vedic_weekday(&e, &obs, pre.into()), 2);
        // 1990-02-21 03:00 UTC = Wednesday 08:30 IST
        let post = instant_from_utc(UtcFields { year: 1990, month: 2, day: 21, hour: 3, minute: 0, second: 0, nanos: 0 }).unwrap();
        assert_eq!(vedic_weekday(&e, &obs, post.into()), 3);
    }

    #[test]
    fn ashtakavarga_reductions_cross_the_boundary() {
        let Some(e) = engine() else { return };
        let r = e
            .ashtakavarga_reductions(birth(), 9.9312, 76.2673, AyanamsaChoice::Krishnamurti)
            .unwrap();
        assert_eq!(r.len(), 7);
        let totals: Vec<u32> = r.iter().map(|x| x.bhinna.iter().map(|&b| b as u32).sum()).collect();
        assert_eq!(totals, kshana_vedic::ROW_TOTALS.to_vec());
        for x in &r {
            for i in 0..12 {
                assert!(x.after_ekadhipatya[i] <= x.after_trikona[i] && x.after_trikona[i] <= x.bhinna[i]);
            }
            assert_eq!(x.shodhya_pinda, x.rasi_pinda + x.graha_pinda);
        }
        // Saturn's table, worked by hand from BPHS's Saturn row with Sun Aq, Moon,
        // Mars, Venus, Saturn Sg, Mercury Cp, Jupiter Ge, Lagna Ar:
        let s = &r[6];
        assert_eq!(s.bhinna, vec![4, 6, 2, 1, 2, 4, 6, 5, 2, 1, 5, 1]);
        // fire 4,2,2 −2; earth 6,4,1 −1; air 2,6,5 −2; water 1,5,1 −1
        assert_eq!(s.after_trikona, vec![2, 5, 0, 0, 0, 3, 4, 4, 0, 0, 3, 0]);
        // Aries/Scorpio both empty 2/4 → 2/2; Taurus/Libra both empty 5/4 → 4/4;
        // the Mercury, Jupiter and Saturn pairs each hold a zero → untouched
        assert_eq!(s.after_ekadhipatya, vec![2, 4, 0, 0, 0, 3, 4, 2, 0, 0, 3, 0]);
        // 2·7 + 4·10 + 3·5 + 4·7 + 2·8 + 3·11 = 146; only the Sun's sign keeps bindus: 3·5
        assert_eq!((s.rasi_pinda, s.graha_pinda, s.shodhya_pinda), (146, 15, 161));
        // 1 bindu in Cancer, 8th from Saturn: 161 → 161 mod 27 = 26 (Uttara Bhadrapada),
        // 161 mod 12 = 5 (Leo)
        assert_eq!((s.eighth_bindus, s.product, s.sensitive_nakshatra, s.sensitive_sign), (1, 161, 25, 4));
    }
}
