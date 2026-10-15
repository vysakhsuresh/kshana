//! `kshana next`: upcoming exact instants.

use kshana_ephem::{Body, Spk};
use kshana_events::{
    full_moons, nakshatra_boundaries, new_moons, sign_ingresses, stations, tithi_boundaries, Event,
    MICROSECOND_NS,
};
use kshana_time::{Instant, Utc, NS_PER_SEC};
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES};
use kshana_vedic::Ayanamsa;

struct Line {
    t: Instant,
    what: String,
    detail: String,
}

pub fn fmt_local(t: Instant, tz_minutes: i64) -> String {
    let u = Utc::from_instant(t.add_ns(tz_minutes as i128 * 60 * NS_PER_SEC));
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
        u.year,
        u.month,
        u.day,
        u.hour,
        u.minute,
        u.second,
        u.nanos / 1_000
    )
}

const SIGNS: [&str; 12] = [
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

/// Print upcoming events from `from` for `days` days.
pub fn run(spk: &Spk, from: Instant, days: i64, tz_minutes: i64) {
    let to = from.add_ns(days as i128 * 86_400 * NS_PER_SEC);
    let mut lines: Vec<Line> = Vec::new();
    let push =
        |lines: &mut Vec<Line>, evs: Vec<Event>, what: &str, detail: &dyn Fn(&Event) -> String| {
            for e in evs {
                lines.push(Line {
                    t: e.root.t,
                    what: what.to_string(),
                    detail: detail(&e),
                });
            }
        };
    let bodies = [
        ("Sun", Body::Sun),
        ("Moon", Body::Moon),
        ("Mercury", Body::Mercury),
        ("Venus", Body::Venus),
        ("Mars", Body::MarsBarycenter),
        ("Jupiter", Body::JupiterBarycenter),
        ("Saturn", Body::SaturnBarycenter),
    ];
    for (name, body) in bodies {
        if let Ok(evs) = sign_ingresses(spk, body, from, to, MICROSECOND_NS) {
            push(&mut lines, evs, &format!("{name} ingress"), &|e| {
                let lon = kshana_sky::apparent(spk, body, e.root.t.add_ns(1_000_000_000))
                    .map(|a| a.longitude)
                    .unwrap_or(0.0);
                format!(
                    "enters {} (tropical)",
                    SIGNS[((lon / 30.0).floor() as usize) % 12]
                )
            });
        }
        if name != "Sun" && name != "Moon" {
            if let Ok(evs) = stations(spk, body, from, to, MICROSECOND_NS) {
                push(&mut lines, evs, &format!("{name} station"), &|e| {
                    let s = kshana_sky::apparent(spk, body, e.root.t.add_ns(86_400 * NS_PER_SEC))
                        .map(|a| a.speed)
                        .unwrap_or(0.0);
                    if s < 0.0 {
                        "turns retrograde".into()
                    } else {
                        "turns direct".into()
                    }
                });
            }
        }
    }
    if let Ok(evs) = new_moons(spk, from, to, MICROSECOND_NS) {
        push(&mut lines, evs, "New Moon", &|_| {
            "Amavasya ends · Shukla Pratipada begins".into()
        });
    }
    if let Ok(evs) = full_moons(spk, from, to, MICROSECOND_NS) {
        push(&mut lines, evs, "Full Moon", &|_| {
            "Purnima ends · Krishna Pratipada begins".into()
        });
    }
    if let Ok(evs) = tithi_boundaries(spk, from, to, MICROSECOND_NS) {
        push(&mut lines, evs, "Tithi", &|e| {
            let t = e.root.t.add_ns(60 * NS_PER_SEC);
            let m = kshana_sky::apparent(spk, Body::Moon, t)
                .map(|a| a.longitude)
                .unwrap_or(0.0);
            let s = kshana_sky::apparent(spk, Body::Sun, t)
                .map(|a| a.longitude)
                .unwrap_or(0.0);
            let n = ((m - s).rem_euclid(360.0) / 12.0).floor() as usize;
            format!(
                "{} {} begins",
                if n < 15 { "Shukla" } else { "Krishna" },
                TITHI_NAMES[n % 15]
            )
        });
    }
    if let Ok(evs) =
        nakshatra_boundaries(spk, Body::Moon, Ayanamsa::Lahiri, from, to, MICROSECOND_NS)
    {
        push(&mut lines, evs, "Moon nakshatra", &|e| {
            let t = e.root.t.add_ns(60 * NS_PER_SEC);
            let a = kshana_sky::apparent(spk, Body::Moon, t)
                .map(|a| a.longitude)
                .unwrap_or(0.0);
            let ay = kshana_vedic::ayanamsa_degrees(Ayanamsa::Lahiri, t.tt_centuries_since_j2000());
            let idx = ((kshana_vedic::sidereal(a, ay) / kshana_vedic::NAKSHATRA_SPAN).floor()
                as usize)
                % 27;
            format!("enters {} (Lahiri)", NAKSHATRA_NAMES[idx])
        });
    }
    lines.sort_by_key(|l| l.t);
    let sign = if tz_minutes < 0 { '-' } else { '+' };
    println!(
        "KSHANA · next {days} days · local UTC{sign}{:02}:{:02} · solved to 1 µs",
        tz_minutes.abs() / 60,
        tz_minutes.abs() % 60
    );
    println!("{:<26}  {:<16}  detail", "local time", "event");
    for l in lines {
        println!(
            "{:<26}  {:<16}  {}",
            fmt_local(l.t, tz_minutes),
            l.what,
            l.detail
        );
    }
}
