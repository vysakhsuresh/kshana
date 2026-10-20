//! More dasha systems over the FFI (expert layer): the Jaimini rasi dashas Narayana,
//! Sthira, Shoola and Drig; the BPHS ch. 46 conditional nakshatra dashas, each with its
//! "applies to this chart" rule; Patyayini for the Tajika annual chart; and the
//! nadiamsa ordinal of the D150. The rules and their sources are documented in
//! `kshana_vedic::dashas::{narayana, sthira, shoola, drig, conditional}`,
//! `kshana_tajika::patyayini` and `kshana_vedic::varga`.
//!
//! Every record here is expert data for the chart pages; none of it is shown on the
//! plain-language screens.

use kshana_chart::Chart;
use kshana_sky::Observer;
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::dashas::{
    conditional, conditional_applies, conditional_sub, drig, drig_sub, graha_name, narayana,
    narayana_sub, shoola, shoola_sub, sthira, sthira_sub, AntarRule, ChartFacts, Conditional,
    Period,
};
use kshana_vedic::DashaLord;

use crate::dashas::{DashaSystemRecord, PeriodRecord};
use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// The systems [`Engine::dasha_system_more`] accepts, in the order
/// [`Engine::dasha_systems_more_at`] returns them.
const MORE: [&str; 12] = [
    "narayana",
    "sthira",
    "shoola",
    "shoola-lagna",
    "drig",
    "shodasottari",
    "dwadasottari",
    "panchottari",
    "shatabdika",
    "chaturaseeti-sama",
    "dwisaptati-sama",
    "shashtihayani",
];

const YEAR_NOTE: &str =
    "Years of 365.25 days; every boundary is exact (the balance at birth is rounded to a nanosecond once).";

fn rec(p: &Period) -> PeriodRecord {
    PeriodRecord {
        label: p.label.clone(),
        start: p.start.into(),
        end: p.end.into(),
        level: p.level,
    }
}

/// Sidereal longitude of a graha: Rahu is the true node, Ketu opposite it.
fn graha_sidereal(natal: &Chart, g: DashaLord) -> Result<f64, KshanaError> {
    match g {
        DashaLord::Rahu => Ok(natal.true_node.1),
        DashaLord::Ketu => Ok((natal.true_node.1 + 180.0).rem_euclid(360.0)),
        other => {
            let name = graha_name(other);
            natal
                .bodies
                .iter()
                .find(|b| b.name == name)
                .map(|b| b.sidereal)
                .ok_or_else(|| KshanaError::Engine(format!("chart has no {name}")))
        }
    }
}

/// The nine grahas Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu.
fn nine(natal: &Chart) -> Result<[f64; 9], KshanaError> {
    use DashaLord::*;
    let order = [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu];
    let mut out = [0.0; 9];
    for (slot, g) in out.iter_mut().zip(order) {
        *slot = graha_sidereal(natal, g)?;
    }
    Ok(out)
}

fn conditional_of(name: &str) -> Option<Conditional> {
    Conditional::ALL
        .into_iter()
        .find(|c| c.name().eq_ignore_ascii_case(name))
}

/// The applicability rule of a conditional dasha in words (BPHS ch. 46).
fn applies_note(c: Conditional) -> &'static str {
    match c {
        Conditional::Shodasottari => "Applies (BPHS 46.24-26) when the lagna is in the Moon's hora in the dark fortnight or the Sun's hora in the bright fortnight.",
        Conditional::Dwadasottari => "Applies (BPHS 46.27-28) when the lagna's navamsa is a sign of Venus.",
        Conditional::Panchottari => "Applies (BPHS 46.29-31, as JHora reads it) when the lagna's dwadasamsa is Cancer.",
        Conditional::Shatabdika => "Applies (BPHS 46.32-34) when the lagna is vargottama (same sign in rasi and navamsa).",
        Conditional::ChaturaseetiSama => "Applies (BPHS 46.35-36) when the lord of the 10th is in the 10th.",
        Conditional::DwisaptatiSama => "Applies (BPHS 46.37-39) when the lagna lord is in the 7th or the 7th lord in the lagna.",
        Conditional::Shashtihayani => "Applies (BPHS 46.40-41) when the Sun is in the lagna.",
    }
}

fn system_record(natal: &Chart, name: &str, at: Instant) -> Result<DashaSystemRecord, KshanaError> {
    let birth = natal.instant;
    let year_ns = YEAR_365_25_NS;
    let lagna = natal.cusps_sidereal[0];
    let grahas = nine(natal)?;
    let moon = grahas[1];
    let mut applies = true;
    let periods: Vec<Period>;
    let sub: Box<dyn Fn(&Period) -> Vec<Period>>;
    let mut notes: Vec<String>;
    match name {
        "narayana" => {
            periods = narayana(lagna, &grahas, birth, year_ns);
            sub = Box::new(move |p| narayana_sub(p, lagna, &grahas));
            notes = vec![
                "Narayana (Jaimini; P. V. R. Narasimha Rao and Sanjay Rath, as JHora): from the stronger of the lagna and the 7th; odd signs run zodiacally, even signs backward; movable signs in order, fixed every sixth, dual by kendras and trines; Saturn or Ketu in the first sign change the order.".into(),
                "Years: count to the lord's sign (forward for odd-footed, backward for even-footed) minus one; lord at home 12; exalted +1, debilitated -1. Second cycle: 12 minus the first.".into(),
                "Antardashas: twelve equal, from the stronger of the signs holding the dasha sign's lord and the 7th lord.".into(),
            ];
        }
        "sthira" => {
            periods = sthira(lagna, &grahas, birth, year_ns);
            sub = Box::new(sthira_sub);
            notes = vec![
                "Sthira (Jaimini): from the sign of Brahma, zodiacally; movable signs 7 years, fixed 8, dual 9 (96 years).".into(),
                "Antardashas: twelve equal.".into(),
            ];
        }
        "shoola" | "shoola-lagna" => {
            let house = if name == "shoola" { 2 } else { 1 };
            periods = shoola(lagna, &grahas, house, birth, year_ns);
            sub = Box::new(move |p| shoola_sub(p, lagna, &grahas));
            notes = vec![
                if house == 2 {
                    "Shoola (Jaimini): from the stronger of the 2nd and the 8th, zodiacally, 9 years each (108 years).".into()
                } else {
                    "Shoola (Jaimini, JHora default): from the stronger of the lagna and the 7th, zodiacally, 9 years each (108 years).".into()
                },
                "Antardashas: twelve equal.".into(),
            ];
        }
        "drig" => {
            periods = drig(lagna, &grahas, birth, year_ns);
            sub = Box::new(move |p| drig_sub(p, lagna, &grahas));
            notes = vec![
                "Drig (Jaimini): the 9th, 10th and 11th, each followed by the signs it aspects by rasi drishti; years as in Narayana, two cycles.".into(),
                "Antardashas: twelve equal.".into(),
            ];
        }
        other => {
            let Some(c) = conditional_of(other) else {
                return Err(KshanaError::Engine(format!(
                    "unknown dasha system {other:?}; one of {}",
                    MORE.join(", ")
                )));
            };
            periods = conditional(c, moon, birth, year_ns);
            sub = Box::new(move |p| conditional_sub(c, p, AntarRule::Equal));
            applies = conditional_applies(c, &ChartFacts { lagna, grahas });
            let lords: Vec<String> = c
                .lords()
                .iter()
                .zip(c.years())
                .map(|(l, y)| format!("{} {y}", graha_name(*l)))
                .collect();
            notes = vec![
                format!("{}: {} ({} years).", c.name(), lords.join(", "), c.cycle_years()),
                applies_note(c).into(),
                "Balance at birth: the untraversed part of the Moon's nakshatra times the first lord's years (BPHS 46).".into(),
                "Antardashas: equal shares in cycle order from the period's own lord (BPHS is silent; PyJHora's rule).".into(),
            ];
        }
    }
    notes.push(
        "Rahu is the true node and Ketu its opposite; positions sidereal by the chosen ayanamsa."
            .into(),
    );
    notes.push(YEAR_NOTE.into());
    let mut running = Vec::new();
    if let Some(k) = periods.iter().position(|p| p.start <= at && at < p.end) {
        running.push(rec(&periods[k]));
        running.extend(sub(&periods[k]).iter().map(rec));
    }
    Ok(DashaSystemRecord {
        system: name.to_string(),
        applies,
        periods: periods.iter().map(rec).collect(),
        running,
        notes,
    })
}

/// One antardasha of a Patyayini period.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PatyayiniSubRecord {
    /// "Lagna" or a graha name.
    pub lord: String,
    pub start: ExactInstant,
    pub end: ExactInstant,
}

/// One Patyayini period of the Tajika year.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PatyayiniRecord {
    /// "Lagna" or a graha name.
    pub lord: String,
    /// Degrees within its sign in the annual chart.
    pub krishamsa: f64,
    /// Its krishamsa minus the previous one's: its share of the year in degrees.
    pub patyamsa: f64,
    pub start: ExactInstant,
    pub end: ExactInstant,
    pub antardashas: Vec<PatyayiniSubRecord>,
}

fn patyayini_lord(l: kshana_tajika::PatyayiniLord) -> String {
    match l {
        kshana_tajika::PatyayiniLord::Lagna => "Lagna".into(),
        kshana_tajika::PatyayiniLord::Graha(g) => graha_name(g).into(),
    }
}

#[uniffi::export]
impl Engine {
    /// One of the further dasha systems (see [`more_dasha_system_names`]) laid out from
    /// the natal chart, with the period running at `at` and its sub-periods. `applies`
    /// is the BPHS ch. 46 condition for the conditional nakshatra dashas and true for
    /// the rasi dashas.
    pub fn dasha_system_more(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        system: String,
        at: ExactInstant,
    ) -> Result<DashaSystemRecord, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        system_record(&natal, &system.to_ascii_lowercase(), at.into())
    }

    /// All the further dasha systems at `at`, in the order of [`more_dasha_system_names`].
    pub fn dasha_systems_more_at(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
    ) -> Result<Vec<DashaSystemRecord>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        MORE.iter()
            .map(|s| system_record(&natal, s, at.into()))
            .collect()
    }

    /// Patyayini dasha of the Tajika year for calendar `year` (annual chart cast at the
    /// person's current place): the lagna and Sun … Saturn ordered by krishamsa, each
    /// period its patyamsa's share of the year from this solar return to the next.
    #[allow(clippy::too_many_arguments)]
    pub fn patyayini(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        year: i64,
        here_latitude_deg: f64,
        here_longitude_deg: f64,
    ) -> Result<Vec<PatyayiniRecord>, KshanaError> {
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
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(kshana_tajika::patyayini(&v.sky, v.instant, v.next_return)
            .into_iter()
            .map(|p| PatyayiniRecord {
                lord: patyayini_lord(p.lord),
                krishamsa: p.krishamsa,
                patyamsa: p.patyamsa,
                start: p.start.into(),
                end: p.end.into(),
                antardashas: p
                    .antardashas
                    .iter()
                    .map(|s| PatyayiniSubRecord {
                        lord: patyayini_lord(s.lord),
                        start: s.start.into(),
                        end: s.end.into(),
                    })
                    .collect(),
            })
            .collect())
    }
}

/// Names of the systems `Engine::dasha_system_more` accepts.
#[uniffi::export]
pub fn more_dasha_system_names() -> Vec<String> {
    MORE.iter().map(|s| s.to_string()).collect()
}

/// Ordinal 1–150 of the named nadiamsa (Chandra Kala Nadi order: movable signs from the
/// first, fixed from the last, dual from the 76th) for a sidereal longitude.
#[uniffi::export]
pub fn nadiamsa_ordinal(sidereal: f64) -> u8 {
    kshana_vedic::nadiamsa_ordinal(sidereal)
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

    /// The Kochi chart (1990-02-20 05:04 UTC): first periods as pinned by hand in the
    /// kshana-vedic tests, through the whole FFI path.
    #[test]
    fn kochi_more_systems() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let birth = u(1990, 2, 20, 5, 4);
        let (lat, lon) = (9.9312, 76.2673);
        let ay = AyanamsaChoice::Lahiri;
        let at = u(2026, 10, 16, 0, 0);
        let all = engine.dasha_systems_more_at(birth, lat, lon, ay, at).unwrap();
        assert_eq!(all.len(), MORE.len());
        for (s, name) in all.iter().zip(MORE) {
            assert_eq!(s.system, name);
            assert!(!s.periods.is_empty(), "{name}");
            assert!(s.periods.windows(2).all(|w| key(w[0].end) == key(w[1].start)), "{name}");
            assert!(key(s.periods[0].start) <= key(birth), "{name}");
            assert!(s.running.len() > 1, "{name}");
            assert_eq!(key(s.running[1].start), key(s.running[0].start), "{name}");
            assert_eq!(key(s.running.last().unwrap().end), key(s.running[0].end), "{name}");
            assert!(s.notes.len() >= 3, "{name}");
        }
        let first = |n: &str| {
            all.iter()
                .find(|s| s.system == n)
                .map(|s| (s.periods[0].label.clone(), s.periods[1].label.clone(), s.applies))
                .unwrap()
        };
        assert_eq!(first("narayana"), ("Libra".into(), "Scorpio".into(), true));
        assert_eq!(first("sthira"), ("Sagittarius".into(), "Capricorn".into(), true));
        assert_eq!(first("shoola"), ("Taurus".into(), "Gemini".into(), true));
        assert_eq!(first("shoola-lagna").0, "Libra");
        assert_eq!(first("drig"), ("Sagittarius".into(), "Pisces".into(), true));
        // Conditional dashas on Kochi (hand-checked in kshana-vedic): first lords, and the
        // five that do not apply (navamsa lagna Cancer, Sun's hora in the dark
        // fortnight, dwadasamsa Virgo, not vargottama, Sun in Aquarius).
        assert_eq!(first("dwadasottari"), ("Sun".into(), "Jupiter".into(), false));
        assert_eq!(first("shodasottari"), ("Saturn".into(), "Ketu".into(), false));
        assert_eq!(first("panchottari"), ("Saturn".into(), "Mars".into(), false));
        assert_eq!(first("shatabdika"), ("Mars".into(), "Saturn".into(), false));
        assert_eq!(first("shashtihayani"), ("Venus".into(), "Saturn".into(), false));
        assert_eq!(first("chaturaseeti-sama").0, "Jupiter");
        assert_eq!(first("dwisaptati-sama").0, "Sun");
        assert!(engine
            .dasha_system_more(birth, lat, lon, ay, "vimshottari".into(), at)
            .is_err());

        let pt = engine.patyayini(birth, lat, lon, ay, 2026, lat, lon).unwrap();
        assert_eq!(pt.len(), 8);
        assert!(pt.windows(2).all(|w| key(w[0].end) == key(w[1].start)));
        assert!(pt.windows(2).all(|w| w[0].krishamsa <= w[1].krishamsa));
        assert!(pt.iter().all(|p| p.antardashas.len() == 8));
        assert_eq!(nadiamsa_ordinal(13.235_310), 67);
    }
}
