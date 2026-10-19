//! Where the time goes for one day: `cargo run --release -p kshana-trace --example profile`.
use kshana_ephem::Spk;
use kshana_sky::Observer;
use kshana_time::{Utc, NS_PER_DAY};
use kshana_trace::{trace, Division, Person, TraceOptions};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::Ayanamsa;

fn main() {
    let spk = Spk::open(std::path::Path::new("data/de421.bsp")).expect("kernel");
    let who = Person {
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
    };
    let from = Utc::new(2026, 10, 16, 0, 0, 0, 0)
        .unwrap()
        .to_instant()
        .unwrap();
    let to = from.add_ns(7 * NS_PER_DAY);
    let variants: [(&str, TraceOptions); 6] = [
        (
            "dasha only d9",
            TraceOptions {
                dasha_depth: 9,
                moon_finest: None,
                planet_finest: None,
                panchanga: false,
                horas: false,
                lagna_finest: None,
            },
        ),
        (
            "moon to sub",
            TraceOptions {
                dasha_depth: 0,
                moon_finest: Some(Division::Sub),
                planet_finest: None,
                panchanga: false,
                horas: false,
                lagna_finest: None,
            },
        ),
        (
            "moon to subsub",
            TraceOptions {
                dasha_depth: 0,
                moon_finest: Some(Division::SubSub),
                planet_finest: None,
                panchanga: false,
                horas: false,
                lagna_finest: None,
            },
        ),
        (
            "planets to sub",
            TraceOptions {
                dasha_depth: 0,
                moon_finest: None,
                planet_finest: Some(Division::Sub),
                panchanga: false,
                horas: false,
                lagna_finest: None,
            },
        ),
        (
            "panchanga",
            TraceOptions {
                dasha_depth: 0,
                moon_finest: None,
                planet_finest: None,
                panchanga: true,
                horas: false,
                lagna_finest: None,
            },
        ),
        (
            "horas",
            TraceOptions {
                dasha_depth: 0,
                moon_finest: None,
                planet_finest: None,
                panchanga: false,
                horas: true,
                lagna_finest: None,
            },
        ),
    ];
    for (label, o) in variants {
        let t0 = std::time::Instant::now();
        let t = trace(&spk, None, &who, from, to, o).unwrap();
        println!(
            "{label:>16}: {:>6} transitions in {:?} for 7 days",
            t.len(),
            t0.elapsed()
        );
    }
    let t0 = std::time::Instant::now();
    let n = 2000;
    for i in 0..n {
        let _ = kshana_sky::apparent(
            &spk,
            kshana_ephem::Body::Moon,
            from.add_ns(i * 1_000_000_000),
        )
        .unwrap();
    }
    println!("apparent(Moon): {:?} each", t0.elapsed() / n as u32);
}
