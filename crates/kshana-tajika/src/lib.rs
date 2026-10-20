//! Tajika varshaphala: the sidereal solar return (Varsha Pravesha) and what the Tajika
//! school reads from it — the Muntha, the five office-bearers and the lord of the year
//! by Panchavargeeya bala, the Sahams, the Tajika aspects and yogas, and the Mudda dasha.
//!
//! Conventions, each sourced in its module:
//!
//! * The year opens at the instant the Sun's **sidereal** longitude returns to its natal
//!   value, solved to a microsecond ([`sidereal_solar_return`]). The annual chart is cast
//!   for that instant at the place supplied (the person's current place), with Placidus
//!   cusps, in the same ayanamsa as the natal chart.
//! * A return is "by day" when the Sun's centre is above the standard rising/setting
//!   altitude at that place and instant.
//! * The Muntha is the natal lagna advanced one sign per completed year, keeping the
//!   natal degree inside the sign; its house is read from the annual cusps.
//! * The five office-bearers (panchadhikari) are the lords of the Muntha sign, the natal
//!   lagna sign and the annual lagna sign, the lord of the sign of the Sun by day or the
//!   Moon by night (Dina-Ratri pati), and the Tri-rashi lord of the annual lagna. They
//!   are scored by [`panchavargeeya`] bala and the year lord is chosen by Charak's chain
//!   ([`choose_varshesha`]): strongest among those of five vishwas or more that aspect
//!   the annual lagna, ties by number of offices, the Moon passed over unless alone or
//!   extraordinarily strong, the Muntha lord when nobody qualifies.
//! * Sahams follow the Jagannatha Hora / Raman list, cross-checked with Charak
//!   ([`SAHAMS`]); Tajika aspects, orbs and yogas follow Charak ch. X
//!   ([`tajika_yogas`]); the Mudda dasha follows Charak ch. V ([`mudda`]).
//!
//! Sources: K. S. Charak, *A Textbook of Varshaphala* (UMA Publications), which
//! systematises the *Tajika Neelakanthi* of Nilakantha (1587); B. V. Raman, *Varshaphal
//! or the Hindu Progressed Horoscope*; Jagannatha Hora and its Python port PyJHora
//! (`saham.py`, `tajaka.py`, `mudda.py`); the teistro-sdk Tajika design notes for the
//! Neelakanthi's Hadda table and the worked checks against Charak's examples.

#![forbid(unsafe_code)]

mod bala;
mod mudda;
mod patyayini;
mod saham;
mod sky;
mod tables;
mod yoga;

pub use bala::{grade, panchavargeeya, relation, Panchavargeeya, Relation};
pub use mudda::{mudda, mudda_start_lord, MuddaPeriod};
pub use patyayini::{patyamsas, patyayini, PatyayiniLord, PatyayiniPeriod, PatyayiniSub};
pub use saham::{between, combine, sahams, Saham, SahamFormula, Term, PUNYA, SAHAMS, SASTRA};
pub use sky::{house_of, AnnualSky};
pub use tables::{
    deep_debilitation, deeptamsa, drekkana_lord, graha_name, hadda_lord, navamsa_lord, sign_index,
    speed_rank, trirashi_lord, within_sign, CHALDEAN_DECANS, HADDA, SEVEN, SIGN_NAMES, TRIRASHI,
};
pub use yoga::{
    adhikara, aspects_by_sign, drishti, link, pair_orb, tajika_yogas, Adhikara, Drishti, Link,
    LinkKind, TajikaYoga, YogaKind,
};

use kshana_chart::{compute, Chart, ChartError, ChartInput};
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{sun_altitude, SUN_RISE_SET_ALT};
use kshana_houses::HouseSystem;
use kshana_sky::{apparent, Observer};
use kshana_solver::{find_root, wrap180, SolverError};
use kshana_time::{EopTable, Instant, Utc, UtcError, NS_PER_DAY};
use kshana_vedic::{ayanamsa_degrees, sidereal, sign_lord, Ayanamsa, DashaLord};

/// Solver tolerance for the return: one microsecond.
pub const MICROSECOND_NS: i128 = 1_000;

/// Mean sidereal motion of the Sun, degrees per day; used only to aim the bracket.
const MEAN_MOTION: f64 = 360.0 / 365.256_363;

/// Vishwa bala below which an office-bearer cannot hold the year (Charak).
pub const WEAK_BELOW: f64 = 5.0;

/// Vishwa bala from which the Moon is "extraordinarily strong" and may hold the year
/// (Adawal's Parakrami grade; the texts give no figure).
pub const MOON_RULES_FROM: f64 = 15.0;

/// Why a varshaphala could not be computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TajikaError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Chart failure (houses undefined at the place, kernel range).
    Chart(ChartError),
    /// The return could not be bracketed or solved.
    Solver(SolverError),
    /// The calendar year could not be expressed as an instant.
    Time(UtcError),
    /// The year asked for is before the birth year.
    BeforeBirth,
}

impl From<SpkError> for TajikaError {
    fn from(e: SpkError) -> Self {
        TajikaError::Spk(e)
    }
}
impl From<ChartError> for TajikaError {
    fn from(e: ChartError) -> Self {
        TajikaError::Chart(e)
    }
}
impl From<SolverError> for TajikaError {
    fn from(e: SolverError) -> Self {
        TajikaError::Solver(e)
    }
}
impl From<UtcError> for TajikaError {
    fn from(e: UtcError) -> Self {
        TajikaError::Time(e)
    }
}

/// The instant in calendar `year` (UTC) when the Sun's sidereal longitude, in the given
/// ayanamsa, returns to `natal_sun_sidereal`, to one microsecond.
///
/// The date is aimed from the Sun's position on 1 January by its mean motion, corrected
/// twice with the local rate, and the root of
/// `wrap180(sidereal(apparent Sun, ayanamsa(t)) − natal)` is then bracketed ±3 days
/// around that aim and solved by Brent's method. The Sun's sidereal longitude is
/// monotonic over the bracket, so the root is unique. For a birthday within hours of
/// the turn of the year the return found may fall just inside the neighbouring calendar
/// year, since the sidereal year is not the civil one.
pub fn sidereal_solar_return(
    spk: &Spk,
    natal_sun_sidereal: f64,
    ayanamsa: Ayanamsa,
    year: i64,
) -> Result<Instant, TajikaError> {
    let natal = natal_sun_sidereal.rem_euclid(360.0);
    let f = |t: Instant| -> Result<f64, SpkError> {
        let a = apparent(spk, Body::Sun, t)?;
        let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
        Ok(wrap180(sidereal(a.longitude, ay) - natal))
    };
    let jan1 = Utc::new(year, 1, 1, 0, 0, 0, 0)?.to_instant()?;
    let f0 = f(jan1)?;
    let days = (-f0).rem_euclid(360.0) / MEAN_MOTION;
    let mut anchor = jan1.add_ns((days * NS_PER_DAY as f64).round() as i128);
    for _ in 0..2 {
        let fa = f(anchor)?;
        anchor = anchor.add_ns((-fa / MEAN_MOTION * NS_PER_DAY as f64).round() as i128);
    }
    let a = anchor.add_ns(-3 * NS_PER_DAY);
    let b = anchor.add_ns(3 * NS_PER_DAY);
    let root = find_root(|t| f(t).unwrap_or(f64::NAN), a, b, MICROSECOND_NS)?;
    Ok(root.t)
}

/// The five offices whose holders contend for the year.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Office {
    /// Lord of the Muntha's sign (Munthesha).
    MunthaLord,
    /// Lord of the natal lagna's sign (Janma lagnesha).
    NatalLagnaLord,
    /// Lord of the annual lagna's sign (Varsha lagnesha).
    AnnualLagnaLord,
    /// Lord of the Sun's sign by day, of the Moon's sign by night (Dina-Ratri pati).
    DinaRatriLord,
    /// Tri-rashi lord of the annual lagna (Tri-rashi pati).
    TrirashiLord,
}

impl Office {
    /// The five offices in the order the texts list them.
    pub const ALL: [Office; 5] = [
        Office::MunthaLord,
        Office::NatalLagnaLord,
        Office::AnnualLagnaLord,
        Office::DinaRatriLord,
        Office::TrirashiLord,
    ];

    /// Name of the office.
    pub fn name(self) -> &'static str {
        match self {
            Office::MunthaLord => "Muntha lord",
            Office::NatalLagnaLord => "natal lagna lord",
            Office::AnnualLagnaLord => "annual lagna lord",
            Office::DinaRatriLord => "Dina-Ratri lord",
            Office::TrirashiLord => "Tri-rashi lord",
        }
    }
}

/// One contender for the year, with every office it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The planet.
    pub lord: DashaLord,
    /// Offices it holds, in [`Office::ALL`] order.
    pub offices: Vec<Office>,
    /// Its five-fold strength.
    pub bala: Panchavargeeya,
    /// Its Vishwa bala out of 20 (`bala.vishwa`, repeated for convenience).
    pub vishwa: f64,
    /// Whether it aspects the annual lagna sign in the Tajika sense.
    pub aspects_lagna: bool,
}

/// The year's reading.
#[derive(Clone, Debug, PartialEq)]
pub struct Varshaphala {
    /// Calendar year of the return.
    pub year: i64,
    /// Completed years of age at the return.
    pub age: u32,
    /// The Varsha Pravesha: the sidereal solar return.
    pub instant: Instant,
    /// The next return; the year runs from `instant` to here.
    pub next_return: Instant,
    /// True when the return is by day at the place.
    pub day: bool,
    /// The annual chart (Placidus, at the place supplied).
    pub chart: Chart,
    /// The seven grahas, lagna and cusps as the rules read them.
    pub sky: AnnualSky,
    /// Muntha `(sign 0–11, house 1–12)`.
    pub muntha: (usize, u8),
    /// Muntha longitude, sidereal.
    pub muntha_longitude: f64,
    /// Lord of the Muntha's sign.
    pub muntha_lord: DashaLord,
    /// Five-fold strength of each of the seven, in [`SEVEN`] order.
    pub balas: Vec<Panchavargeeya>,
    /// The office-bearers, one entry per distinct planet.
    pub candidates: Vec<Candidate>,
    /// Lord of the year.
    pub varshesha: DashaLord,
    /// How the year lord was chosen, step by step.
    pub reasoning: Vec<String>,
    /// The thirty-five sahams.
    pub sahams: Vec<Saham>,
    /// Tajika yogas in the annual chart.
    pub yogas: Vec<TajikaYoga>,
    /// Mudda dasha for the year, with antardashas.
    pub mudda: Vec<MuddaPeriod>,
}

/// The five office-bearers of a return, in [`Office::ALL`] order.
pub fn office_bearers(
    natal_lagna_sign: usize,
    muntha_sign: usize,
    sky: &AnnualSky,
    day: bool,
) -> Vec<(Office, DashaLord)> {
    let luminary = if day { DashaLord::Sun } else { DashaLord::Moon };
    vec![
        (Office::MunthaLord, sign_lord(muntha_sign)),
        (Office::NatalLagnaLord, sign_lord(natal_lagna_sign)),
        (Office::AnnualLagnaLord, sign_lord(sky.lagna_sign())),
        (
            Office::DinaRatriLord,
            sign_lord(sky.sign(luminary).unwrap_or(0)),
        ),
        (Office::TrirashiLord, trirashi_lord(sky.lagna_sign(), day)),
    ]
}

/// Merge office-bearers into one candidate per planet, scored and checked for aspect to
/// the annual lagna.
pub fn candidates(sky: &AnnualSky, bearers: &[(Office, DashaLord)]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    for (office, lord) in bearers {
        if let Some(c) = out.iter_mut().find(|c| c.lord == *lord) {
            c.offices.push(*office);
            continue;
        }
        let Some(bala) = panchavargeeya(sky, *lord) else {
            continue;
        };
        let aspects_lagna = sky
            .sign(*lord)
            .is_some_and(|s| aspects_by_sign(s, sky.lagna_sign()));
        out.push(Candidate {
            lord: *lord,
            offices: vec![*office],
            bala,
            vishwa: bala.vishwa,
            aspects_lagna,
        });
    }
    out
}

/// Charak's chain for the lord of the year (ch. VII, following the *Tajika Neelakanthi*,
/// Varshatantra vv. 11–12), with each step written into `reasoning`:
///
/// 1. If every office-bearer is below [`WEAK_BELOW`] vishwas, the Muntha lord takes the
///    year.
/// 2. Only office-bearers of five vishwas or more that aspect the annual lagna sign
///    (any of the eight Tajika aspect positions) qualify; if none does, the Muntha lord
///    takes the year.
/// 3. The Moon, "mild by nature", is passed over when another qualifies, unless it is
///    extraordinarily strong ([`MOON_RULES_FROM`]).
/// 4. The strongest by Vishwa bala wins; a tie goes to the planet holding more offices;
///    an outright tie to the Muntha lord.
pub fn choose_varshesha(
    candidates: &[Candidate],
    muntha_lord: DashaLord,
    reasoning: &mut Vec<String>,
) -> DashaLord {
    if candidates.iter().all(|c| c.vishwa < WEAK_BELOW) {
        reasoning.push(format!(
            "every office-bearer is below {WEAK_BELOW:.0} vishwas; the Muntha lord {} takes the year",
            graha_name(muntha_lord)
        ));
        return muntha_lord;
    }
    let mut q: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.vishwa >= WEAK_BELOW && c.aspects_lagna)
        .collect();
    for c in candidates
        .iter()
        .filter(|c| !(c.vishwa >= WEAK_BELOW && c.aspects_lagna))
    {
        reasoning.push(format!(
            "{} is set aside: {}",
            graha_name(c.lord),
            if c.vishwa < WEAK_BELOW {
                format!("only {:.2} vishwas", c.vishwa)
            } else {
                "it does not aspect the annual lagna".to_string()
            }
        ));
    }
    if q.is_empty() {
        reasoning.push(format!(
            "no office-bearer of {WEAK_BELOW:.0} vishwas or more aspects the annual lagna; the Muntha lord {} takes the year",
            graha_name(muntha_lord)
        ));
        return muntha_lord;
    }
    q.sort_by(|a, b| {
        b.vishwa
            .partial_cmp(&a.vishwa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.offices.len().cmp(&a.offices.len()))
    });
    if q[0].lord == DashaLord::Moon && q.len() > 1 && q[0].vishwa < MOON_RULES_FROM {
        reasoning.push(format!(
            "the Moon, though strongest at {:.2} vishwas, is mild by nature and steps down (it would rule only from {MOON_RULES_FROM:.0})",
            q[0].vishwa
        ));
        q.remove(0);
    }
    let best = q[0];
    let tied: Vec<&Candidate> = q
        .iter()
        .copied()
        .filter(|c| (c.vishwa - best.vishwa).abs() < 1e-9)
        .collect();
    if tied.len() == 1 {
        reasoning.push(format!(
            "{} is the strongest office-bearer that aspects the annual lagna, {:.2} vishwas ({}); it is the lord of the year",
            graha_name(best.lord),
            best.vishwa,
            grade(best.vishwa)
        ));
        return best.lord;
    }
    let most = tied.iter().map(|c| c.offices.len()).max().unwrap_or(0);
    let holders: Vec<&Candidate> = tied
        .iter()
        .copied()
        .filter(|c| c.offices.len() == most)
        .collect();
    if holders.len() == 1 {
        reasoning.push(format!(
            "{} tie at {:.2} vishwas; {} holds more offices ({}) and is the lord of the year",
            tied.iter()
                .map(|c| graha_name(c.lord))
                .collect::<Vec<_>>()
                .join(", "),
            best.vishwa,
            graha_name(holders[0].lord),
            most
        ));
        return holders[0].lord;
    }
    reasoning.push(format!(
        "{} tie outright at {:.2} vishwas with {} offices each; the Muntha lord {} takes the year",
        holders
            .iter()
            .map(|c| graha_name(c.lord))
            .collect::<Vec<_>>()
            .join(", "),
        best.vishwa,
        most,
        graha_name(muntha_lord)
    ));
    muntha_lord
}

fn stamp(t: Instant) -> String {
    let u = Utc::from_instant(t);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        u.year, u.month, u.day, u.hour, u.minute, u.second
    )
}

/// The varshaphala for calendar `year`: the sidereal return of the natal Sun, the annual
/// chart at `obs` (Placidus), and everything read from it. `natal` must have been
/// computed with the same `ayanamsa`.
pub fn varshaphala(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Chart,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    year: i64,
) -> Result<Varshaphala, TajikaError> {
    let birth = Utc::from_instant(natal.instant);
    if year < birth.year {
        return Err(TajikaError::BeforeBirth);
    }
    let age = (year - birth.year) as u32;
    let natal_of = |name: &str| {
        natal
            .bodies
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.sidereal)
            .unwrap_or(0.0)
    };
    let natal_sun = natal_of("Sun");
    let natal_moon = natal_of("Moon");
    let instant = sidereal_solar_return(spk, natal_sun, ayanamsa, year)?;
    let next_return = sidereal_solar_return(spk, natal_sun, ayanamsa, year + 1)?;
    let chart = compute(
        spk,
        eop,
        &ChartInput {
            instant,
            observer: *obs,
            system: HouseSystem::Placidus,
            ayanamsa,
            topocentric_moon: false,
        },
    )?;
    let day = sun_altitude(spk, eop, obs, instant)? > SUN_RISE_SET_ALT;
    let sky = AnnualSky::from_chart(&chart);

    let natal_lagna = natal.cusps_sidereal[0];
    let natal_lagna_sign = sign_index(natal_lagna);
    let muntha_longitude = (natal_lagna + 30.0 * age as f64).rem_euclid(360.0);
    let muntha_sign = sign_index(muntha_longitude);
    let muntha_house = sky.house(muntha_longitude);
    let muntha_lord = sign_lord(muntha_sign);

    let bearers = office_bearers(natal_lagna_sign, muntha_sign, &sky, day);
    let candidates = candidates(&sky, &bearers);
    let balas: Vec<Panchavargeeya> = SEVEN
        .iter()
        .filter_map(|g| panchavargeeya(&sky, *g))
        .collect();

    let mut reasoning = vec![
        format!(
            "Varsha Pravesha {} for the {}th year, by {} at {:.4}°{} {:.4}°{}; annual lagna {} {:.2}°",
            stamp(instant),
            age + 1,
            if day { "day" } else { "night" },
            obs.latitude_deg.abs(),
            if obs.latitude_deg >= 0.0 { "N" } else { "S" },
            obs.longitude_deg.abs(),
            if obs.longitude_deg >= 0.0 { "E" } else { "W" },
            SIGN_NAMES[sky.lagna_sign()],
            within_sign(sky.lagna)
        ),
        format!(
            "Muntha in {} {:.2}° (house {}), lord {}",
            SIGN_NAMES[muntha_sign],
            within_sign(muntha_longitude),
            muntha_house,
            graha_name(muntha_lord)
        ),
    ];
    for c in &candidates {
        reasoning.push(format!(
            "{}: {}; {:.2} vishwas (Kshetra {:.1}, Uchcha {:.1}, Hadda {:.2}, Drekkana {:.1}, Navamsa {:.2}); {} the annual lagna from {}",
            graha_name(c.lord),
            c.offices
                .iter()
                .map(|o| o.name())
                .collect::<Vec<_>>()
                .join(", "),
            c.vishwa,
            c.bala.kshetra,
            c.bala.uchcha,
            c.bala.hadda,
            c.bala.drekkana,
            c.bala.navamsa,
            if c.aspects_lagna {
                "aspects"
            } else {
                "does not aspect"
            },
            SIGN_NAMES[sky.sign(c.lord).unwrap_or(0)]
        ));
    }
    let varshesha = choose_varshesha(&candidates, muntha_lord, &mut reasoning);

    Ok(Varshaphala {
        year,
        age,
        instant,
        next_return,
        day,
        sahams: sahams(&sky, day),
        yogas: tajika_yogas(&sky),
        mudda: mudda(natal_moon, instant, next_return, age),
        chart,
        sky,
        muntha: (muntha_sign, muntha_house),
        muntha_longitude,
        muntha_lord,
        balas,
        candidates,
        varshesha,
        reasoning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KOCHI: Observer = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };

    fn fixture() -> Option<(Spk, Chart)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let t = Utc::new(1990, 10, 29, 12, 6, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: t,
                observer: KOCHI,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        Some((spk, ch))
    }

    #[test]
    fn sidereal_return_lands_on_the_birthday_2000_to_2030() {
        let Some((spk, natal)) = fixture() else {
            return;
        };
        let natal_sun = natal.bodies[0].sidereal;
        for year in 2000..=2030 {
            let t = sidereal_solar_return(&spk, natal_sun, Ayanamsa::Lahiri, year).unwrap();
            let birthday = Utc::new(year, 10, 29, 12, 6, 0, 0)
                .unwrap()
                .to_instant()
                .unwrap();
            let off_days = t.ns_since(birthday) as f64 / NS_PER_DAY as f64;
            assert!(off_days.abs() <= 1.5, "{year}: {off_days} days off");
            let a = apparent(&spk, Body::Sun, t).unwrap();
            let s = sidereal(
                a.longitude,
                ayanamsa_degrees(Ayanamsa::Lahiri, t.tt_centuries_since_j2000()),
            );
            assert!(wrap180(s - natal_sun).abs() < 1e-6, "{year}");
        }
        // the birth year's return is the birth itself
        let t0 = sidereal_solar_return(&spk, natal_sun, Ayanamsa::Lahiri, 1990).unwrap();
        assert!(t0.ns_since(natal.instant).abs() < 10 * MICROSECOND_NS);
    }

    #[test]
    fn varshaphala_hangs_together() {
        let Some((spk, natal)) = fixture() else {
            return;
        };
        let v25 = varshaphala(&spk, None, &natal, &KOCHI, Ayanamsa::Lahiri, 2025).unwrap();
        let v26 = varshaphala(&spk, None, &natal, &KOCHI, Ayanamsa::Lahiri, 2026).unwrap();
        assert_eq!((v25.age, v26.age), (35, 36));
        // Muntha advances one sign a year from the natal lagna
        assert_eq!(
            v25.muntha.0,
            (sign_index(natal.cusps_sidereal[0]) + 35) % 12
        );
        assert_eq!((v25.muntha.0 + 1) % 12, v26.muntha.0);
        assert!((1..=12).contains(&v25.muntha.1));
        assert_eq!(v25.muntha_lord, sign_lord(v25.muntha.0));
        assert_eq!(v25.muntha.1, v25.sky.house(v25.muntha_longitude));
        // the annual Sun is the natal Sun
        assert!(wrap180(v25.chart.bodies[0].sidereal - natal.bodies[0].sidereal).abs() < 1e-6);
        // five offices over the candidates; the year lord is one of them
        assert_eq!(
            v25.candidates
                .iter()
                .map(|c| c.offices.len())
                .sum::<usize>(),
            5
        );
        assert!(v25.candidates.iter().any(|c| c.lord == v25.varshesha));
        assert!(v25.reasoning.len() >= 3);
        assert_eq!(v25.balas.len(), 7);
        for b in &v25.balas {
            assert!((0.0..=20.0).contains(&b.vishwa), "{b:?}");
        }
        // sahams
        assert_eq!(v25.sahams.len(), SAHAMS.len());
        for s in &v25.sahams {
            assert!((0.0..360.0).contains(&s.longitude), "{}", s.name);
            assert!((1..=12).contains(&s.house));
        }
        // mudda: contiguous, filling the year exactly, which ends at the next return
        assert_eq!(v25.next_return, v26.instant);
        assert_eq!(v25.mudda[0].start, v25.instant);
        assert_eq!(v25.mudda.last().unwrap().end, v25.next_return);
        for w in v25.mudda.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
        assert_eq!(
            v25.mudda[0].lord,
            mudda_start_lord(natal.bodies[1].sidereal, 35)
        );
        let year_days = v25.next_return.ns_since(v25.instant) as f64 / NS_PER_DAY as f64;
        assert!((year_days - 365.2564).abs() < 0.05, "{year_days}");
        // yogas carry words
        for y in &v25.yogas {
            assert!(!y.words.is_empty());
            assert!(y.planets.len() >= 2);
        }
    }

    #[test]
    fn year_before_birth_is_refused() {
        let Some((spk, natal)) = fixture() else {
            return;
        };
        assert_eq!(
            varshaphala(&spk, None, &natal, &KOCHI, Ayanamsa::Lahiri, 1989).err(),
            Some(TajikaError::BeforeBirth)
        );
    }

    fn cand(lord: DashaLord, offices: &[Office], vishwa: f64, aspects: bool) -> Candidate {
        Candidate {
            lord,
            offices: offices.to_vec(),
            bala: Panchavargeeya {
                planet: lord,
                kshetra: 0.0,
                uchcha: 0.0,
                hadda: 0.0,
                drekkana: 0.0,
                navamsa: 0.0,
                total: vishwa * 4.0,
                vishwa,
            },
            vishwa,
            aspects_lagna: aspects,
        }
    }

    #[test]
    fn the_year_lord_chain() {
        let mut r = Vec::new();
        // strongest without aspect loses to the strongest that aspects (Charak's example chart)
        let cs = [
            cand(DashaLord::Jupiter, &[Office::MunthaLord], 14.77, false),
            cand(DashaLord::Sun, &[Office::DinaRatriLord], 14.34, true),
            cand(
                DashaLord::Mars,
                &[Office::NatalLagnaLord, Office::AnnualLagnaLord],
                9.0,
                true,
            ),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Jupiter, &mut r),
            DashaLord::Sun
        );
        // nobody aspects → Muntha lord
        let cs = [
            cand(DashaLord::Jupiter, &[Office::MunthaLord], 14.0, false),
            cand(DashaLord::Sun, &[Office::DinaRatriLord], 12.0, false),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Jupiter, &mut r),
            DashaLord::Jupiter
        );
        // all weak → Muntha lord
        let cs = [cand(DashaLord::Sun, &[Office::DinaRatriLord], 4.0, true)];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Venus
        );
        // the Moon steps down for the next qualifier
        let cs = [
            cand(DashaLord::Moon, &[Office::AnnualLagnaLord], 12.0, true),
            cand(DashaLord::Venus, &[Office::MunthaLord], 9.0, true),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Venus
        );
        // but rules when extraordinarily strong, or alone
        let cs = [
            cand(DashaLord::Moon, &[Office::AnnualLagnaLord], 16.0, true),
            cand(DashaLord::Venus, &[Office::MunthaLord], 9.0, true),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Moon
        );
        let cs = [cand(DashaLord::Moon, &[Office::AnnualLagnaLord], 8.0, true)];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Moon
        );
        // ties: more offices, then the Muntha lord
        let cs = [
            cand(DashaLord::Sun, &[Office::DinaRatriLord], 10.0, true),
            cand(
                DashaLord::Mars,
                &[Office::NatalLagnaLord, Office::AnnualLagnaLord],
                10.0,
                true,
            ),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Mars
        );
        let cs = [
            cand(DashaLord::Sun, &[Office::DinaRatriLord], 10.0, true),
            cand(DashaLord::Mars, &[Office::NatalLagnaLord], 10.0, true),
        ];
        assert_eq!(
            choose_varshesha(&cs, DashaLord::Venus, &mut r),
            DashaLord::Venus
        );
        assert!(r.iter().any(|s| s.contains("lord of the year")));
    }
}
