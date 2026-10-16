//! Print what the Today and Right-time doors show for a sample birth, for design previews.
//! `cargo run -p kshana-forecast --example today`

use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_forecast::{daily, observances, sade_sati, transits_at, TransitKind};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_SEC};
use kshana_vedic::Ayanamsa;

fn local(i: Instant) -> String {
    let u = Utc::from_instant(i.add_ns(330 * 60 * NS_PER_SEC));
    format!("{:02}:{:02}:{:02}", u.hour, u.minute, u.second)
}
fn date(i: Instant) -> String {
    let u = Utc::from_instant(i.add_ns(330 * 60 * NS_PER_SEC));
    format!("{:04}-{:02}-{:02} {}", u.year, u.month, u.day, local(i))
}

fn main() {
    let spk = Spk::open(std::path::Path::new("data/de421.bsp")).expect("kernel");
    let obs = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };
    let birth = Utc::new(1990, 10, 29, 12, 6, 0, 0).unwrap().to_instant().unwrap();
    let ch = compute(
        &spk,
        None,
        &ChartInput {
            instant: birth,
            observer: obs,
            system: HouseSystem::Placidus,
            ayanamsa: Ayanamsa::Lahiri,
            topocentric_moon: false,
        },
    )
    .unwrap();
    let at = Utc::new(2026, 10, 16, 3, 30, 0, 0).unwrap().to_instant().unwrap();
    let d = daily(&spk, None, &obs, &ch, at, Ayanamsa::Lahiri).unwrap();
    println!("headline: {}", d.headline);
    if let Some(day) = &d.day {
        println!("sunrise {} sunset {} next {}", local(day.sunrise), local(day.sunset), local(day.next_sunrise));
    }
    println!("tara {:?} chandra {:?} moon nakshatra {}", d.tara, d.chandra_bala, d.moon_nakshatra);
    for w in &d.windows {
        println!("  {} {}–{}  {}", if w.avoid { "AVOID" } else { "good " }, local(w.start), local(w.end), w.reason);
    }
    let ss = sade_sati(&spk, &ch, at, Ayanamsa::Lahiri).unwrap();
    println!("sade sati phase {} end {:?} next {:?}", ss.phase, ss.phase_end.map(date), ss.next_start.map(date));
    for t in transits_at(&spk, &ch, at, Ayanamsa::Lahiri).unwrap() {
        if t.kind == TransitKind::FromMoon && matches!(t.body, "Jupiter" | "Saturn") {
            println!("{} in house {} from Moon", t.body, t.house);
        }
    }
    for o in observances(&spk, None, &obs, Ayanamsa::Lahiri, at, 20).unwrap().iter().take(8) {
        println!("cal {} {}", date(o.exact), o.name);
    }
}
