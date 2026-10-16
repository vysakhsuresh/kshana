//! Print the forecast for a birth at the present moment and a 30-day series.
//! `cargo run --release -p kshana-cli --example forecast_demo -- 1990-02-20T05:04:00 2026-10-16T05:00:00`
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_rules::{moment, series, story, Life, Natal};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

fn parse(s: &str) -> Instant {
    let (d, t) = s.split_once('T').unwrap();
    let mut d = d.split('-');
    let mut t = t.split(':');
    Utc::new(
        d.next().unwrap().parse().unwrap(),
        d.next().unwrap().parse().unwrap(),
        d.next().unwrap().parse().unwrap(),
        t.next().unwrap().parse().unwrap(),
        t.next().unwrap().parse().unwrap(),
        t.next().unwrap_or("0").parse().unwrap(),
        0,
    )
    .unwrap()
    .to_instant()
    .unwrap()
}

fn fmt(i: Instant) -> String {
    let u = Utc::from_instant(i.add_ns(19_800 * 1_000_000_000));
    format!("{:02}/{:02} {:02}:{:02}", u.day, u.month, u.hour, u.minute)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let birth = parse(args.get(1).map(String::as_str).unwrap_or("1990-02-20T05:04:00"));
    let now = parse(args.get(2).map(String::as_str).unwrap_or("2026-10-16T05:00:00"));
    let spk = Spk::open(std::path::Path::new("data/de421.bsp")).unwrap();
    let eop = std::fs::read_to_string("data/finals2000A.all")
        .ok()
        .map(|s| EopTable::parse(&s));
    let obs = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };
    let input = ChartInput {
        instant: birth,
        observer: obs,
        system: HouseSystem::Placidus,
        ayanamsa: Ayanamsa::Lahiri,
        topocentric_moon: false,
    };
    let ch = compute(&spk, eop.as_ref(), &input).unwrap();
    let natal = Natal::new(kshana_rules::Facts::from_chart(&ch), birth);

    let t0 = std::time::Instant::now();
    let m = moment(&spk, eop.as_ref(), &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, now).unwrap();
    println!("moment in {:?}", t0.elapsed());
    println!("{} · overall {:.0} · {}", m.headline, m.overall * 100.0, m.moon_star);
    println!("{} · Moon in the {} from your Moon · {}", m.tara_words, m.moon_from_moon, m.saturn.words());
    if let Some(h) = &m.hora {
        println!("hora {:?} {} → {} · {}", h.lord, fmt(h.start), fmt(h.end), h.words);
    }
    println!("dasha: {}", m.dasha.iter().map(|p| format!("{:?}", p.lord)).collect::<Vec<_>>().join(" > "));
    for l in &m.lives {
        println!("  {:<9} {:>3}  {:<7} {}", l.life.name(), (l.score * 100.0).round(), l.band.word(), l.headline);
        println!("            because {}", l.because);
        for c in &l.components {
            println!("            · {:<7} {:.2} × {:.2}  {}", c.key, c.score, c.weight, c.note);
        }
    }
    println!("windows:");
    for w in &m.windows {
        println!("  {} {} → {}{}", w.label, fmt(w.start), fmt(w.end), if w.caution { " (caution)" } else { "" });
    }
    println!("shifts:");
    for s in m.shifts.iter().take(14) {
        println!("  {} {:<6} {}", fmt(s.at), s.kind, s.what);
    }

    let t0 = std::time::Instant::now();
    let samples = series(&spk, eop.as_ref(), &natal, &obs, Ayanamsa::Lahiri, now, NS_PER_DAY, 30).unwrap();
    println!("series(30) in {:?}", t0.elapsed());
    for life in Life::ALL {
        let st = story(life, &samples);
        let idx = Life::ALL.iter().position(|l| *l == life).unwrap();
        let spark: String = samples.iter().map(|s| {
            let v = s.scores[idx];
            ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][((v * 7.999) as usize).min(7)]
        }).collect();
        println!("  {:<9} {} mean {:.0} {} · {}", life.name(), spark, st.mean * 100.0, st.trend, st.trend_words);
        if let Some((a, b, v)) = st.best { println!("            best {} → {} ({:.0})", fmt(a), fmt(b), v * 100.0); }
        if let Some((a, b, v)) = st.low { println!("            low  {} → {} ({:.0})", fmt(a), fmt(b), v * 100.0); }
        for s in &st.shifts { println!("            {} {}", fmt(s.at), s.what); }
    }
}
