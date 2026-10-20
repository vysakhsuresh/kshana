//! The five dasha systems over the FFI: Vimshottari, Yogini, Ashtottari, Kalachakra and
//! Chara, each laid out from the natal chart with its level-1 periods, the period
//! running at an instant with that period's sub-periods, and its conventions in words.

use kshana_chart::Chart;
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::dashas::{
    ashtottari, ashtottari_applies, ashtottari_sub, chara, chara_direction, chara_sub,
    graha_from_name, graha_name, kalachakra_detail, kalachakra_sub, sign_index, yogini, yogini_sub,
    Period,
};
use kshana_vedic::{sign_lord, subdivide, vimshottari, DashaLord, DashaPeriod};

use crate::life::natal_chart;
use crate::{AyanamsaChoice, Engine, ExactInstant, KshanaError};

/// The systems, in the order [`dasha_system_names`] and `dasha_systems_at` use.
const SYSTEMS: [&str; 5] = ["vimshottari", "yogini", "ashtottari", "kalachakra", "chara"];

/// One period of any dasha system at any level.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct PeriodRecord {
    /// Graha, yogini or sign name.
    pub label: String,
    pub start: ExactInstant,
    pub end: ExactInstant,
    /// 1 = mahadasha, 2 = antardasha.
    pub level: u8,
}

/// One dasha system laid out from a birth.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct DashaSystemRecord {
    /// "vimshottari", "yogini", "ashtottari", "kalachakra" or "chara".
    pub system: String,
    /// Whether the system applies to this chart: the BPHS rule for Ashtottari (Rahu in a
    /// kendra or trikona from the lagna lord's sign and not in the lagna); always true
    /// for the others.
    pub applies: bool,
    /// The level-1 periods (mahadashas) from the running one at birth.
    pub periods: Vec<PeriodRecord>,
    /// The level-1 period containing the asked instant followed by all its level-2
    /// sub-periods; empty when the instant falls outside the laid-out periods.
    pub running: Vec<PeriodRecord>,
    /// The system's conventions, one per line.
    pub notes: Vec<String>,
}

fn rec(p: &Period) -> PeriodRecord {
    PeriodRecord {
        label: p.label.clone(),
        start: p.start.into(),
        end: p.end.into(),
        level: p.level,
    }
}

fn vim_period(p: DashaPeriod, level: u8) -> Period {
    Period {
        label: graha_name(p.lord).to_string(),
        start: p.start,
        end: p.end,
        level,
    }
}

/// Sign index (0 = Aries) of a sidereal longitude.
fn sign_of(lon: f64) -> usize {
    ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11)
}

/// Sidereal longitude of a graha in the natal chart: Rahu is the true node, Ketu is
/// opposite it.
fn graha_sidereal(natal: &Chart, g: DashaLord) -> Result<f64, KshanaError> {
    match g {
        DashaLord::Rahu => Ok(natal.rahu().1),
        DashaLord::Ketu => Ok((natal.rahu().1 + 180.0).rem_euclid(360.0)),
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

/// The nine grahas in Chara's input order: Sun, Moon, Mars, Mercury, Jupiter, Venus,
/// Saturn, Rahu, Ketu.
fn chara_grahas(natal: &Chart) -> Result<[f64; 9], KshanaError> {
    let order = [
        DashaLord::Sun,
        DashaLord::Moon,
        DashaLord::Mars,
        DashaLord::Mercury,
        DashaLord::Jupiter,
        DashaLord::Venus,
        DashaLord::Saturn,
        DashaLord::Rahu,
        DashaLord::Ketu,
    ];
    let mut out = [0.0; 9];
    for (slot, g) in out.iter_mut().zip(order) {
        *slot = graha_sidereal(natal, g)?;
    }
    Ok(out)
}

const YEAR_NOTE: &str =
    "Years of 365.25 days; the balance at birth is rounded to a nanosecond once and every later boundary is exact.";

/// Sub-periods of level-1 period number `k` (its index in the laid-out list).
type SubFn<'a> = Box<dyn Fn(usize, &Period) -> Vec<Period> + 'a>;

fn system_record(natal: &Chart, name: &str, at: Instant) -> Result<DashaSystemRecord, KshanaError> {
    let birth = natal.instant;
    let moon = graha_sidereal(natal, DashaLord::Moon)?;
    let lagna_sidereal = natal.cusps_sidereal[0];
    let year_ns = YEAR_365_25_NS;
    let mut applies = true;
    let periods: Vec<Period>;
    let sub: SubFn<'_>;
    let notes: Vec<&str>;
    match name {
        "vimshottari" => {
            periods = vimshottari(moon, birth, year_ns)
                .into_iter()
                .map(|p| vim_period(p, 1))
                .collect();
            sub = Box::new(|_, p: &Period| {
                let Some(lord) = graha_from_name(&p.label) else {
                    return Vec::new();
                };
                subdivide(DashaPeriod {
                    lord,
                    start: p.start,
                    end: p.end,
                })
                .into_iter()
                .map(|q| vim_period(q, 2))
                .collect()
            });
            notes = vec![
                "Vimshottari: Ketu 7, Venus 20, Sun 6, Moon 10, Mars 7, Rahu 18, Jupiter 16, Saturn 19, Mercury 17 (120 years), starting from the lord of the Moon's nakshatra.",
                "Balance at birth: the untraversed fraction of the Moon's nakshatra times the lord's years.",
                "Antardashas: the nine lords in cycle order from the period's own lord, proportioned by their years over 120.",
                YEAR_NOTE,
            ];
        }
        "yogini" => {
            periods = yogini(moon, birth, year_ns);
            sub = Box::new(|_, p: &Period| yogini_sub(p));
            notes = vec![
                "Yogini: Mangala 1 (Moon), Pingala 2 (Sun), Dhanya 3 (Jupiter), Bhramari 4 (Mars), Bhadrika 5 (Mercury), Ulka 6 (Saturn), Siddha 7 (Venus), Sankata 8 (Rahu); 36 years.",
                "Starting yogini (BPHS): (nakshatra number + 3) mod 8, 0 = Sankata; balance from the untraversed fraction of the Moon's nakshatra.",
                "Antardashas: the eight yoginis in cycle order from the period's own, proportioned by their years over 36 (BPHS; not PyJHora's equal eighths).",
                YEAR_NOTE,
            ];
        }
        "ashtottari" => {
            periods = ashtottari(moon, birth, year_ns);
            sub = Box::new(|_, p: &Period| ashtottari_sub(p));
            let lagna = sign_of(lagna_sidereal);
            let lord_sign = sign_of(graha_sidereal(natal, sign_lord(lagna))?);
            applies = ashtottari_applies(lagna, lord_sign, sign_of(natal.rahu().1));
            notes = vec![
                "Ashtottari: Sun 6, Moon 15, Mars 8, Mercury 17, Saturn 10, Jupiter 19, Rahu 12, Venus 21 (108 years); Ketu has no period.",
                "Nakshatra groups of 4 and 3 from Ardra (27-nakshatra scheme); balance from the Moon's place within its lord's whole group.",
                "Antardashas: the eight lords in cycle order from the period's own lord, proportioned by their years over 108 (JHora).",
                "Applies (BPHS 47.1-2) when Rahu is in a kendra or trikona from the sign of the lagna lord and not in the lagna; Rahu is the true node.",
                YEAR_NOTE,
            ];
        }
        "kalachakra" => {
            let d = kalachakra_detail(moon, birth, year_ns);
            let (row, first) = (d.row, d.first_position);
            periods = d.periods;
            sub = Box::new(move |k, p: &Period| kalachakra_sub(p, &row, (first + k) % 9));
            notes = vec![
                "Kalachakra: nine-sign rows per nakshatra pada (JHora tables, Revati savya as in BPHS); paramayus 100/85/83/86 savya, 86/83/85/100 apasavya.",
                "Sign years (BPHS 46): Aries 7, Taurus 16, Gemini 9, Cancer 21, Leo 5, Virgo 9, Libra 16, Scorpio 7, Sagittarius 10, Capricorn 4, Aquarius 4, Pisces 10.",
                "Balance (BPHS 46.93): the fraction of the Moon's pada traversed times the paramayus; the natal row repeats after its ninth sign.",
                "Antardashas: the natal row rotated to start from the period's sign, proportioned by sign years over the paramayus.",
                YEAR_NOTE,
            ];
        }
        "chara" => {
            let grahas = chara_grahas(natal)?;
            periods = chara(lagna_sidereal, &grahas, birth, year_ns);
            sub = Box::new(|_, p: &Period| {
                sign_index(&p.label)
                    .map(|s| chara_sub(p, chara_direction(s)))
                    .unwrap_or_default()
            });
            notes = vec![
                "Chara (Jaimini, K. N. Rao): twelve signs from the lagna, direct when the ninth sign from the lagna is odd-footed, reverse otherwise.",
                "Years: count from the sign to its lord's sign (direct for odd-footed signs, reverse for even-footed) minus one; lord in its own sign 12; exalted +1, debilitated -1; Scorpio and Aquarius by their co-lords.",
                "Rahu is the true node and Ketu its opposite; all positions sidereal by the chosen ayanamsa.",
                "Antardashas: twelve equal, from the sign after the period's sign in that sign's own direction, ending on the sign itself.",
                YEAR_NOTE,
            ];
        }
        other => {
            return Err(KshanaError::Engine(format!(
                "unknown dasha system {other:?}; one of {}",
                SYSTEMS.join(", ")
            )))
        }
    }
    let mut running = Vec::new();
    if let Some(k) = periods.iter().position(|p| p.start <= at && at < p.end) {
        running.push(rec(&periods[k]));
        running.extend(sub(k, &periods[k]).iter().map(rec));
    }
    Ok(DashaSystemRecord {
        system: name.to_string(),
        applies,
        periods: periods.iter().map(rec).collect(),
        running,
        notes: notes.iter().map(|s| s.to_string()).collect(),
    })
}

#[uniffi::export]
impl Engine {
    /// One dasha system ("vimshottari", "yogini", "ashtottari", "kalachakra" or "chara")
    /// laid out from the natal chart, with the period running at `at` and its
    /// sub-periods.
    pub fn dasha_system(
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

    /// All five dasha systems at `at`, in the order of [`dasha_system_names`].
    pub fn dasha_systems_at(
        &self,
        birth: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
        ayanamsa: AyanamsaChoice,
        at: ExactInstant,
    ) -> Result<Vec<DashaSystemRecord>, KshanaError> {
        let natal = natal_chart(self, birth, latitude_deg, longitude_deg, ayanamsa)?;
        SYSTEMS
            .iter()
            .map(|s| system_record(&natal, s, at.into()))
            .collect()
    }
}

/// Names of the dasha systems `Engine::dasha_system` accepts, in the order
/// `Engine::dasha_systems_at` returns them.
#[uniffi::export]
pub fn dasha_system_names() -> Vec<String> {
    SYSTEMS.iter().map(|s| s.to_string()).collect()
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
    fn five_systems_cross_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let birth = u(1990, 10, 29, 12, 6);
        let (lat, lon) = (9.9312, 76.2673);
        let ay = AyanamsaChoice::Lahiri;
        let at = u(2026, 10, 16, 0, 0);

        let names = dasha_system_names();
        assert_eq!(names.len(), 5);
        let all = engine.dasha_systems_at(birth, lat, lon, ay, at).unwrap();
        assert_eq!(all.len(), 5);
        for (s, name) in all.iter().zip(&names) {
            assert_eq!(&s.system, name);
            assert!(!s.periods.is_empty(), "{name}");
            assert!(s
                .periods
                .iter()
                .all(|p| p.level == 1 && !p.label.is_empty()));
            assert!(s
                .periods
                .windows(2)
                .all(|w| key(w[0].end) == key(w[1].start)));
            assert!(key(s.periods[0].start) <= key(birth));
            assert!(s.running.len() > 1, "{name}");
            assert_eq!(s.running[0].level, 1);
            assert!(key(s.running[0].start) <= key(at) && key(at) < key(s.running[0].end));
            assert!(s.running[1..].iter().all(|p| p.level == 2));
            assert_eq!(key(s.running[1].start), key(s.running[0].start));
            assert_eq!(key(s.running.last().unwrap().end), key(s.running[0].end));
            assert!(s.notes.len() >= 4);
        }
        // one cycle of nine, eight, eight, nine and twelve
        assert_eq!(all[0].periods.len(), 9);
        assert_eq!(all[0].running.len(), 10);
        assert_eq!(all[1].running.len(), 9);
        assert_eq!(all[2].running.len(), 9);
        assert_eq!(all[3].running.len(), 10);
        assert_eq!(all[4].periods.len(), 12);
        assert_eq!(all[4].running.len(), 13);
        assert!(all.iter().enumerate().all(|(i, s)| i == 2 || s.applies));
        // the fixture Moon in Shatabhisha: Rahu, Jupiter, then Saturn running in 2026
        assert_eq!(all[0].periods[0].label, "Rahu");
        assert_eq!(all[0].running[0].label, "Saturn");

        let one = engine
            .dasha_system(birth, lat, lon, ay, "Vimshottari".into(), at)
            .unwrap();
        assert_eq!(one.system, "vimshottari");
        assert_eq!(one.running[0].label, all[0].running[0].label);
        assert!(engine
            .dasha_system(birth, lat, lon, ay, "narayana".into(), at)
            .is_err());
    }
}
