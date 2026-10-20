//! The life trace: a person's life as a sequence of exact instants at which some
//! astrological state changes. Dasha levels down to minutes, every planet's entry
//! into a sign, nakshatra, pada, KP sub and sub-sub (the nodes by their true
//! longitude), every tithi, karana and yoga,
//! every hora, sunrise and sunset. Each instant is a solved root or exact integer
//! arithmetic; none is interpolated.
//!
//! The trace is generated lazily for a requested window so a phone can hold a day,
//! a month or a year of it, and `snapshot` answers "what is running right now" at any
//! instant without a scan.

#![forbid(unsafe_code)]

pub mod crossings;
pub mod dasha;
pub mod lagna;
pub mod zodiac;

use kshana_ephem::{Body, Spk};
use kshana_events::{hora_at, solar_day, sun_rise_set, EventError, SolarDay};
use kshana_sky::{apparent, true_node_longitude, Observer};
use kshana_time::{EopTable, Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES, YOGA_NAMES};
use kshana_vedic::{ayanamsa_degrees, karana, sidereal, sub_table, Ayanamsa, DashaLord};

pub use crossings::{crossings, Crossing};
pub use dasha::{chain_at, dasha_transitions, DashaTransition, LEVEL_NAMES, MAX_DEPTH};
pub use lagna::{
    ascendant_sidereal, dasha_shift_per_second_ns, first_lord_years, meaningful_depth,
};
pub use zodiac::{subsub_table, Division, SubSub};

/// Sign names.
pub const SIGN_NAMES: [&str; 12] = [
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
    "Aquarius",
    "Pisces",
];

/// What changed.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// A Vimshottari period at `level` began; `chain` is the full chain after it.
    Dasha {
        /// 1 = mahadasha … 9.
        level: u8,
        /// Lords from level 1 down.
        chain: Vec<DashaLord>,
        /// End of the new period.
        until: Instant,
    },
    /// A body entered a cell of a zodiac division.
    Enter {
        /// Body name.
        body: &'static str,
        /// Which division.
        division: Division,
        /// Cell entered.
        index: usize,
        /// Direct (true) or retrograde (false).
        direct: bool,
    },
    /// A tithi began (1–30).
    Tithi(u8),
    /// A karana began (ordinal 0–59).
    Karana(u8),
    /// A yoga began (0–26).
    Yoga(u8),
    /// A planetary hour began.
    Hora(&'static str),
    /// Sunrise.
    Sunrise,
    /// Sunset.
    Sunset,
}

/// One transition.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// Exact instant.
    pub at: Instant,
    /// Half-width of the solved bracket in nanoseconds (0 for exact arithmetic).
    pub half_width_ns: i128,
    /// What changed.
    pub change: Change,
}

impl Transition {
    /// The transition in words.
    pub fn describe(&self) -> String {
        describe(&self.change)
    }

    /// How far the real sky's event may lie from this instant, in nanoseconds, from
    /// the ephemeris and model limits in `docs/TRACE.md` (not the solver, which is
    /// `half_width_ns`). Dasha boundaries are exact arithmetic and return 0; their
    /// sensitivity to the recorded birth time is a separate number
    /// ([`dasha_shift_per_second_ns`]).
    pub fn physical_uncertainty_ns(&self) -> i128 {
        physical_uncertainty_ns_at(&self.change, self.at)
    }

    /// How well the civil label (UTC or a zone) of this instant can be printed today,
    /// in seconds: the leap-second floor of `kshana_time::civil_label_uncertainty_s`.
    pub fn civil_label_uncertainty_s(&self) -> f64 {
        kshana_time::civil_label_uncertainty_s(self.at)
    }
}

/// [`physical_uncertainty_ns`] widened by the Earth-rotation (ΔT) uncertainty at the
/// instant for everything that depends on the rotating Earth: sunrise, horas and the
/// rising degree. Within the IERS table that term is a few milliseconds; decades ahead
/// it grows to seconds, and the ascendant's sub-sub then means less than it prints.
pub fn physical_uncertainty_ns_at(c: &Change, at: Instant) -> i128 {
    let base = physical_uncertainty_ns(c);
    let rotation_bound = matches!(
        c,
        Change::Hora(_) | Change::Sunrise | Change::Sunset | Change::Enter { body: "Lagna", .. }
    );
    if !rotation_bound {
        return base;
    }
    let year = 2000.0 + at.tt_centuries_since_j2000() * 100.0;
    let sigma_s = kshana_time::delta_t_for_year(year).sigma;
    base + (sigma_s * 1e9) as i128
}

/// See [`Transition::physical_uncertainty_ns`].
pub fn physical_uncertainty_ns(c: &Change) -> i128 {
    const S: i128 = 1_000_000_000;
    match c {
        Change::Dasha { .. } => 0,
        Change::Enter { body, .. } => match *body {
            "Moon" => S / 3,
            "Sun" => 4 * S,
            "Mercury" | "Venus" => 20 * S,
            "Mars" => 60 * S,
            "Jupiter" => 210 * S,
            "Saturn" => 540 * S,
            "Rahu" | "Ketu" => 2 * S,
            "Lagna" => 20 * S,
            _ => 60 * S,
        },
        Change::Tithi(_) | Change::Karana(_) | Change::Yoga(_) => S / 3,
        Change::Hora(_) | Change::Sunrise | Change::Sunset => 20 * S,
    }
}

/// Words for a change.
pub fn describe(c: &Change) -> String {
    match c {
        Change::Dasha { level, chain, .. } => {
            let lords: Vec<String> = chain.iter().map(|l| format!("{l:?}")).collect();
            format!(
                "{} begins: {}",
                LEVEL_NAMES[(*level as usize).clamp(1, 9) - 1],
                lords.join(" › ")
            )
        }
        Change::Enter {
            body,
            division,
            index,
            direct,
        } => {
            let motion = if *direct { "enters" } else { "slips back into" };
            match division {
                Division::Sign => format!("{body} {motion} {}", SIGN_NAMES[*index % 12]),
                Division::Nakshatra => format!("{body} {motion} {}", NAKSHATRA_NAMES[*index % 27]),
                Division::Pada => format!(
                    "{body} {motion} {} pada {}",
                    NAKSHATRA_NAMES[(*index / 4) % 27],
                    index % 4 + 1
                ),
                Division::Sub => {
                    let t = sub_table();
                    let s = t[*index % t.len()];
                    format!(
                        "{body} {motion} the sub of {:?} in {} (KP {})",
                        s.sub,
                        NAKSHATRA_NAMES[((s.start + 1e-6) / (360.0 / 27.0)).floor() as usize % 27],
                        s.number
                    )
                }
                Division::SubSub => {
                    let t = subsub_table();
                    let s = t[*index % t.len()];
                    format!(
                        "{body} {motion} the sub-sub of {:?} under {:?} in the star of {:?}",
                        s.subsub, s.sub, s.star
                    )
                }
                Division::Tithi | Division::Karana | Division::Yoga => {
                    format!("{body} {motion} cell {index} of the {}", division.name())
                }
            }
        }
        Change::Tithi(n) => {
            let n = (*n).clamp(1, 30);
            let paksha = if n <= 15 { "Shukla" } else { "Krishna" };
            let name = if n == 30 {
                "Amavasya"
            } else if n == 15 {
                "Purnima"
            } else {
                TITHI_NAMES[((n - 1) % 15) as usize]
            };
            format!("{paksha} {name} begins")
        }
        Change::Karana(k) => format!("Karana {} begins", karana_name(*k)),
        Change::Yoga(y) => format!("{} yoga begins", YOGA_NAMES[(*y as usize) % 27]),
        Change::Hora(l) => format!("{l} hora begins"),
        Change::Sunrise => "Sunrise".to_string(),
        Change::Sunset => "Sunset".to_string(),
    }
}

fn karana_name(k: u8) -> &'static str {
    karana(6.0 * k as f64 + 1.0, 0.0).name
}

/// Which parts of the trace to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceOptions {
    /// Dasha depth 1–9 (0 = none).
    pub dasha_depth: usize,
    /// Finest zodiac division for the Moon.
    pub moon_finest: Option<Division>,
    /// Finest zodiac division for the other grahas (Sun … Saturn, mean nodes).
    pub planet_finest: Option<Division>,
    /// Tithi, karana and yoga boundaries.
    pub panchanga: bool,
    /// Horas, sunrise, sunset (needs an observer).
    pub horas: bool,
    /// Finest zodiac division for the rising degree (the ascendant clock).
    pub lagna_finest: Option<Division>,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            dasha_depth: 5,
            moon_finest: Some(Division::SubSub),
            planet_finest: Some(Division::Sub),
            panchanga: true,
            horas: true,
            lagna_finest: Some(Division::Nakshatra),
        }
    }
}

/// Who the trace is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Person {
    /// Birth instant.
    pub birth: Instant,
    /// Natal Moon, sidereal degrees.
    pub moon_sidereal: f64,
    /// Where they are now (for horas and sunrise).
    pub observer: Observer,
    /// Ayanamsa.
    pub ayanamsa: Ayanamsa,
    /// Length of the dasha year in nanoseconds.
    pub year_ns: i128,
}

const GRAHAS: [(&str, Body, i128); 7] = [
    ("Sun", Body::Sun, 24 * 3_600),
    ("Moon", Body::Moon, 2 * 3_600),
    ("Mercury", Body::Mercury, 24 * 3_600),
    ("Venus", Body::Venus, 24 * 3_600),
    ("Mars", Body::MarsBarycenter, 2 * 24 * 3_600),
    ("Jupiter", Body::JupiterBarycenter, 5 * 24 * 3_600),
    ("Saturn", Body::SaturnBarycenter, 10 * 24 * 3_600),
];

fn divisions_up_to(finest: Division) -> Vec<Division> {
    let order = [
        Division::Sign,
        Division::Nakshatra,
        Division::Pada,
        Division::Sub,
        Division::SubSub,
    ];
    let k = order.iter().position(|d| *d == finest).unwrap_or(0);
    order[..=k].to_vec()
}

/// Generate the trace in `[from, to)`.
pub fn trace(
    spk: &Spk,
    eop: Option<&EopTable>,
    who: &Person,
    from: Instant,
    to: Instant,
    opts: TraceOptions,
) -> Result<Vec<Transition>, EventError> {
    let tol = 1_000; // 1 µs
    let mut out = Vec::new();
    if opts.dasha_depth > 0 {
        for d in dasha_transitions(
            who.moon_sidereal,
            who.birth,
            who.year_ns,
            opts.dasha_depth,
            from,
            to,
        ) {
            out.push(Transition {
                at: d.at,
                half_width_ns: 0,
                change: Change::Dasha {
                    level: d.level,
                    chain: d.chain,
                    until: d.until,
                },
            });
        }
    }
    let ay = who.ayanamsa;
    for (name, body, step_s) in GRAHAS {
        let finest = if name == "Moon" {
            opts.moon_finest
        } else {
            opts.planet_finest
        };
        let Some(finest) = finest else { continue };
        let f = |t: Instant| {
            apparent(spk, body, t).map(|a| {
                sidereal(
                    a.longitude,
                    ayanamsa_degrees(ay, t.tt_centuries_since_j2000()),
                )
            })
        };
        for div in divisions_up_to(finest) {
            // Finer cells need a finer step so bisection stays shallow.
            let step = match div {
                Division::Sub | Division::SubSub if name == "Moon" => 3_600,
                _ => step_s,
            };
            for c in crossings(f, div, from, to, step * NS_PER_SEC, tol)? {
                out.push(Transition {
                    at: c.at,
                    half_width_ns: c.half_width_ns,
                    change: Change::Enter {
                        body: name,
                        division: div,
                        index: c.entered,
                        direct: c.direct,
                    },
                });
            }
        }
    }
    if let Some(finest) = opts.planet_finest {
        // The true (osculating) node, as the chart's facts use it: it wobbles about the
        // mean node by up to 1.7° with a 173-day period and runs direct for short
        // spells, so it crosses subs both ways. Scanned daily.
        for (name, offset) in [("Rahu", 0.0), ("Ketu", 180.0)] {
            let f = |t: Instant| {
                let tc = t.tt_centuries_since_j2000();
                true_node_longitude(spk, t).map(|n| sidereal(n + offset, ayanamsa_degrees(ay, tc)))
            };
            for div in divisions_up_to(finest) {
                for c in crossings(f, div, from, to, 86_400 * NS_PER_SEC, tol)? {
                    out.push(Transition {
                        at: c.at,
                        half_width_ns: c.half_width_ns,
                        change: Change::Enter {
                            body: name,
                            division: div,
                            index: c.entered,
                            direct: c.direct,
                        },
                    });
                }
            }
        }
    }
    if let Some(finest) = opts.lagna_finest {
        let f = |t: Instant| Ok(ascendant_sidereal(eop, &who.observer, ay, t));
        for div in divisions_up_to(finest) {
            // The ascendant moves a sub-sub in seconds: step by a minute.
            let step = match div {
                Division::Sign | Division::Nakshatra => 600,
                Division::Pada => 120,
                _ => 60,
            };
            for c in crossings(f, div, from, to, step * NS_PER_SEC, tol)? {
                out.push(Transition {
                    at: c.at,
                    half_width_ns: c.half_width_ns,
                    change: Change::Enter {
                        body: "Lagna",
                        division: div,
                        index: c.entered,
                        direct: c.direct,
                    },
                });
            }
        }
    }
    if opts.panchanga {
        let elong = |t: Instant| {
            let m = apparent(spk, Body::Moon, t)?.longitude;
            let s = apparent(spk, Body::Sun, t)?.longitude;
            Ok((m - s).rem_euclid(360.0))
        };
        for c in crossings(
            elong,
            Division::Tithi,
            from,
            to,
            3 * 3_600 * NS_PER_SEC,
            tol,
        )? {
            out.push(Transition {
                at: c.at,
                half_width_ns: c.half_width_ns,
                change: Change::Tithi(c.entered as u8 + 1),
            });
        }
        for c in crossings(
            elong,
            Division::Karana,
            from,
            to,
            3 * 3_600 * NS_PER_SEC,
            tol,
        )? {
            out.push(Transition {
                at: c.at,
                half_width_ns: c.half_width_ns,
                change: Change::Karana(c.entered as u8),
            });
        }
        let sum = |t: Instant| {
            let tc = t.tt_centuries_since_j2000();
            let a = ayanamsa_degrees(ay, tc);
            let m = sidereal(apparent(spk, Body::Moon, t)?.longitude, a);
            let s = sidereal(apparent(spk, Body::Sun, t)?.longitude, a);
            Ok((m + s).rem_euclid(360.0))
        };
        for c in crossings(sum, Division::Yoga, from, to, 3 * 3_600 * NS_PER_SEC, tol)? {
            out.push(Transition {
                at: c.at,
                half_width_ns: c.half_width_ns,
                change: Change::Yoga(c.entered as u8),
            });
        }
    }
    if opts.horas {
        // One rise/set scan for the whole window (plus a day each side), then the horas
        // of every solar day that touches it.
        let (rises, sets) = sun_rise_set(
            spk,
            eop,
            &who.observer,
            from.add_ns(-NS_PER_DAY),
            to.add_ns(NS_PER_DAY),
            1_000_000,
        )?;
        for (i, &sunrise) in rises.iter().enumerate() {
            let Some(&next_sunrise) = rises.get(i + 1) else {
                break;
            };
            let Some(&sunset) = sets.iter().find(|s| **s > sunrise && **s < next_sunrise) else {
                continue;
            };
            if next_sunrise <= from || sunrise >= to {
                continue;
            }
            let day = SolarDay {
                sunrise,
                sunset,
                next_sunrise,
            };
            if sunrise >= from && sunrise < to {
                out.push(Transition {
                    at: sunrise,
                    half_width_ns: 1_000_000,
                    change: Change::Sunrise,
                });
            }
            if sunset >= from && sunset < to {
                out.push(Transition {
                    at: sunset,
                    half_width_ns: 1_000_000,
                    change: Change::Sunset,
                });
            }
            let day_len = sunset.ns_since(sunrise) / 12;
            let night_len = next_sunrise.ns_since(sunset) / 12;
            for k in 0..24 {
                let start = if k < 12 {
                    sunrise.add_ns(k * day_len)
                } else {
                    sunset.add_ns((k - 12) * night_len)
                };
                if start >= from && start < to {
                    let (lord, _, _) = hora_at(&day, &who.observer, start.add_ns(NS_PER_SEC));
                    out.push(Transition {
                        at: start,
                        half_width_ns: 1_000_000,
                        change: Change::Hora(lord),
                    });
                }
            }
        }
    }
    out.sort_by_key(|x| x.at);
    Ok(out)
}

/// The state of every traced quantity at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Dasha chain with exact bounds, level 1 down.
    pub dashas: Vec<kshana_vedic::DashaPeriod>,
    /// For each graha: name, sidereal longitude, sign, nakshatra, pada, sub index, sub-sub index, direct.
    pub grahas: Vec<GrahaState>,
    /// Tithi number 1–30 and fraction elapsed.
    pub tithi: (u8, f64),
    /// Karana ordinal.
    pub karana: u8,
    /// Yoga index.
    pub yoga: u8,
    /// Hora lord with its exact bounds, when a solar day was found.
    pub hora: Option<(&'static str, Instant, Instant)>,
}

/// One graha's state.
#[derive(Clone, Debug, PartialEq)]
pub struct GrahaState {
    /// Name.
    pub name: &'static str,
    /// Sidereal longitude.
    pub sidereal: f64,
    /// Sign 0–11.
    pub sign: usize,
    /// Nakshatra 0–26.
    pub nakshatra: usize,
    /// Pada 1–4.
    pub pada: u8,
    /// Sub index 0–248 and its lords (star, sub).
    pub sub: (usize, DashaLord, DashaLord),
    /// Sub-sub index 0–2240 and its lord.
    pub subsub: (usize, DashaLord),
    /// Direct motion.
    pub direct: bool,
}

/// Everything running at `at`.
pub fn snapshot(
    spk: &Spk,
    eop: Option<&EopTable>,
    who: &Person,
    at: Instant,
    depth: usize,
) -> Result<Snapshot, EventError> {
    let tc = at.tt_centuries_since_j2000();
    let a = ayanamsa_degrees(who.ayanamsa, tc);
    let subs = sub_table();
    let subsubs = subsub_table();
    let mut grahas = Vec::with_capacity(9);
    let mut push = |name: &'static str, lon: f64, direct: bool| {
        let si = Division::Sub.index(lon);
        let ssi = Division::SubSub.index(lon);
        grahas.push(GrahaState {
            name,
            sidereal: lon,
            sign: Division::Sign.index(lon),
            nakshatra: Division::Nakshatra.index(lon),
            pada: (Division::Pada.index(lon) % 4) as u8 + 1,
            sub: (si, subs[si].star, subs[si].sub),
            subsub: (ssi, subsubs[ssi].subsub),
            direct,
        });
    };
    let mut moon = 0.0;
    let mut sun = 0.0;
    for (name, body, _) in GRAHAS {
        let ap = apparent(spk, body, at)?;
        let lon = sidereal(ap.longitude, a);
        if name == "Moon" {
            moon = lon;
        }
        if name == "Sun" {
            sun = lon;
        }
        push(name, lon, ap.speed >= 0.0);
    }
    let node = sidereal(true_node_longitude(spk, at)?, a);
    let node_later = sidereal(true_node_longitude(spk, at.add_ns(3_600 * NS_PER_SEC))?, a);
    let node_direct = kshana_solver::wrap180(node_later - node) > 0.0;
    push("Rahu", node, node_direct);
    push("Ketu", (node + 180.0).rem_euclid(360.0), node_direct);
    push(
        "Lagna",
        ascendant_sidereal(eop, &who.observer, who.ayanamsa, at),
        true,
    );
    let e = (moon - sun).rem_euclid(360.0);
    let hora = solar_day(spk, eop, &who.observer, at)?.map(|d| hora_at(&d, &who.observer, at));
    Ok(Snapshot {
        dashas: chain_at(who.moon_sidereal, who.birth, who.year_ns, depth, at),
        grahas,
        tithi: ((e / 12.0).floor() as u8 + 1, (e / 12.0).fract()),
        karana: (e / 6.0).floor() as u8,
        yoga: Division::Yoga.index((moon + sun).rem_euclid(360.0)) as u8,
        hora,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::{Utc, NS_PER_DAY};
    use kshana_vedic::dasha::YEAR_365_25_NS;

    fn person() -> Person {
        Person {
            birth: Utc::new(1990, 10, 29, 12, 6, 0, 0)
                .unwrap()
                .to_instant()
                .unwrap(),
            moon_sidereal: 316.344,
            observer: Observer {
                latitude_deg: 9.9312,
                longitude_deg: 76.2673,
                height_m: 0.0,
            },
            ayanamsa: Ayanamsa::Lahiri,
            year_ns: YEAR_365_25_NS,
        }
    }

    #[test]
    fn a_day_of_trace_is_dense_ordered_and_consistent_with_snapshots() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let who = person();
        let from = Utc::new(2026, 10, 16, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let to = from.add_ns(NS_PER_DAY);
        let opts = TraceOptions {
            dasha_depth: 7,
            lagna_finest: Some(Division::Sub),
            ..TraceOptions::default()
        };
        let t = trace(&spk, None, &who, from, to, opts).unwrap();
        assert!(t.windows(2).all(|w| w[0].at <= w[1].at));
        let moon_subsubs = t
            .iter()
            .filter(|x| {
                matches!(
                    x.change,
                    Change::Enter {
                        body: "Moon",
                        division: Division::SubSub,
                        ..
                    }
                )
            })
            .count();
        assert!((40..=140).contains(&moon_subsubs), "{moon_subsubs}");
        assert_eq!(
            t.iter()
                .filter(|x| matches!(x.change, Change::Sunrise))
                .count(),
            1
        );
        assert_eq!(
            t.iter()
                .filter(|x| matches!(x.change, Change::Hora(_)))
                .count(),
            24
        );
        assert!(t
            .iter()
            .any(|x| matches!(x.change, Change::Dasha { level: 6 | 7, .. })));
        for x in t
            .iter()
            .filter(|x| matches!(x.change, Change::Dasha { .. }))
            .take(10)
        {
            let Change::Dasha {
                level, ref chain, ..
            } = x.change
            else {
                unreachable!()
            };
            assert_eq!(chain.len(), level as usize);
            let c = chain_at(
                who.moon_sidereal,
                who.birth,
                who.year_ns,
                level as usize,
                x.at,
            );
            assert_eq!(c.last().unwrap().start, x.at);
        }
        // every Moon sub-sub entry agrees with a snapshot taken just after it
        for x in t
            .iter()
            .filter(|x| {
                matches!(
                    x.change,
                    Change::Enter {
                        body: "Moon",
                        division: Division::SubSub,
                        ..
                    }
                )
            })
            .take(20)
        {
            let Change::Enter { index, .. } = x.change else {
                unreachable!()
            };
            let s = snapshot(&spk, None, &who, x.at.add_ns(NS_PER_SEC), 5).unwrap();
            let moon = s.grahas.iter().find(|g| g.name == "Moon").unwrap();
            assert_eq!(moon.subsub.0, index);
            assert!(x.half_width_ns <= 1_000);
            assert!(!x.describe().is_empty());
        }
        // tithi transitions agree with the snapshot's tithi
        for x in t.iter().filter(|x| matches!(x.change, Change::Tithi(_))) {
            let Change::Tithi(n) = x.change else {
                unreachable!()
            };
            let s = snapshot(&spk, None, &who, x.at.add_ns(NS_PER_SEC), 1).unwrap();
            assert_eq!(s.tithi.0, n);
        }
    }

    #[test]
    fn words_cover_every_change() {
        let c = [
            Change::Dasha {
                level: 5,
                chain: vec![DashaLord::Saturn, DashaLord::Venus],
                until: Instant::j2000(),
            },
            Change::Enter {
                body: "Moon",
                division: Division::Sub,
                index: 20,
                direct: true,
            },
            Change::Enter {
                body: "Moon",
                division: Division::SubSub,
                index: 200,
                direct: true,
            },
            Change::Enter {
                body: "Mars",
                division: Division::Sign,
                index: 3,
                direct: false,
            },
            Change::Enter {
                body: "Mars",
                division: Division::Pada,
                index: 5,
                direct: true,
            },
            Change::Tithi(30),
            Change::Karana(0),
            Change::Yoga(26),
            Change::Hora("Saturn"),
            Change::Sunset,
        ];
        for x in c {
            assert!(!describe(&x).is_empty());
        }
        assert_eq!(describe(&Change::Tithi(15)), "Shukla Purnima begins");
        assert_eq!(describe(&Change::Tithi(30)), "Krishna Amavasya begins");
    }
}
