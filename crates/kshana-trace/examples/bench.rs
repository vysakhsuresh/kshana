//! Time the trace for a day and a month in release mode: `cargo run --release -p kshana-trace --example bench`.
use kshana_ephem::Spk;
use kshana_sky::Observer;
use kshana_time::{Utc, NS_PER_DAY};
use kshana_trace::{trace, Person, TraceOptions};
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
    for (label, days, depth) in [
        ("day", 1, 9),
        ("week", 7, 7),
        ("month", 30, 5),
        ("year", 365, 3),
    ] {
        let t0 = std::time::Instant::now();
        let t = trace(
            &spk,
            None,
            &who,
            from,
            from.add_ns(days * NS_PER_DAY),
            TraceOptions {
                dasha_depth: depth,
                ..TraceOptions::default()
            },
        )
        .unwrap();
        println!(
            "{label:>6}: {:>7} transitions in {:?}",
            t.len(),
            t0.elapsed()
        );
    }
}
