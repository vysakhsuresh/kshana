//! Print the six answers for one life. `cargo run -p kshana-cli --example ask_demo -- Love`
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_rules::{answer, Life, Natal, Question};
use kshana_sky::Observer;
use kshana_time::{EopTable, Utc};
use kshana_vedic::Ayanamsa;

fn main() {
    let life = std::env::args().nth(1).and_then(|s| Life::parse(&s)).unwrap_or(Life::Love);
    let spk = Spk::open(std::path::Path::new("data/de421.bsp")).unwrap();
    let eop = std::fs::read_to_string("data/finals2000A.all").ok().map(|s| EopTable::parse(&s));
    let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
    let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
    let at = Utc::new(2026, 10, 17, 6, 0, 0, 0).unwrap().to_instant().unwrap();
    let natal = Natal::compute(&spk, eop.as_ref(), birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
    for q in Question::ALL {
        let t0 = std::time::Instant::now();
        let a = answer(&spk, eop.as_ref(), &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, life, q, at).unwrap();
        println!("\n{} ({:?})\n  {} · {:.0} · {:.0}% sure\n  {}", q.words(), t0.elapsed(), a.verdict, a.score * 100.0, a.confidence * 100.0, a.headline);
    }
}
