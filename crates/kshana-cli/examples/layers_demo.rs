//! Time and print the explain, plan, together and book layers for a birth.
//! `cargo run --release -p kshana-cli --example layers_demo -- 1990-02-20T10:34:00 1997-09-22T18:42:00 2026-10-17T05:00:00`
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_rules::{appointment, couple, couple_ask, explain, kundali, timeline, Life, Natal, Purpose, Question};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc};
use kshana_vedic::Ayanamsa;

fn parse(s: &str) -> Instant {
    let (d, t) = s.split_once('T').unwrap();
    let mut d = d.split('-');
    let mut t = t.split(':');
    // the demo takes IST civil times
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
    .add_ns(-19_800 * 1_000_000_000)
}

fn fmt(i: Instant) -> String {
    let u = Utc::from_instant(i.add_ns(19_800 * 1_000_000_000));
    format!("{:04}-{:02}-{:02} {:02}:{:02}", u.year, u.month, u.day, u.hour, u.minute)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let birth = parse(args.get(1).map(String::as_str).unwrap_or("1990-02-20T10:34:00"));
    let other = parse(args.get(2).map(String::as_str).unwrap_or("1997-09-22T18:42:00"));
    let now = parse(args.get(3).map(String::as_str).unwrap_or("2026-10-17T05:00:00"));
    let spk = Spk::open(std::path::Path::new("data/de421.bsp")).unwrap();
    let eop = std::fs::read_to_string("data/finals2000A.all").ok().map(|s| EopTable::parse(&s));
    let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
    let a = Natal::compute(&spk, eop.as_ref(), birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
    let b = Natal::compute(&spk, eop.as_ref(), other, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();

    let t0 = std::time::Instant::now();
    let e = explain(&spk, eop.as_ref(), &a, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Love, now).unwrap();
    println!("== EXPLAIN love ({:?})\n{}", t0.elapsed(), e.headline);
    for c in e.causes.iter().take(4) {
        println!("  drag {:+.0}  {}  [{}]", c.pull * 100.0, c.what, c.until_words);
    }
    for c in e.helpers.iter().take(3) {
        println!("  help {:+.0}  {}", c.pull * 100.0, c.what);
    }
    for r in e.relief.iter().filter(|r| matches!(r.kind, "dasha" | "gochara")).take(6) {
        println!("  {} {:+.0}  {}", fmt(r.at), r.gain * 100.0, r.what);
    }
    if let Some(t) = &e.eases { println!("  eases: {}", t.words); }
    if let Some(t) = &e.peak { println!("  peak: {}", t.words); }

    let t0 = std::time::Instant::now();
    let ap = appointment(&spk, eop.as_ref(), &a, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Purpose::Meeting, now.add_ns(3 * 86_400 * 1_000_000_000 + 5 * 3_600 * 1_000_000_000)).unwrap();
    println!("\n== PLAN meeting ({:?})\n{} {:.0}  {}", t0.elapsed(), ap.verdict, ap.score * 100.0, ap.headline);
    for l in &ap.advice { println!("  · {l}"); }
    for s in &ap.better_today { println!("  better {} – {} {:?} {:.0}", fmt(s.start), fmt(s.end), s.lord, s.score * 100.0); }
    println!("  {}", ap.better_day_words);

    let t0 = std::time::Instant::now();
    let k = kundali(&a, &b, true);
    println!("\n== KUNDALI ({:?})\n{}", t0.elapsed(), k.headline);
    for x in &k.kootas { println!("  {:<13} {:>4}/{:<2} {}", x.name, x.points, x.max, x.words); }
    println!("  {}", k.kuja_words);

    let t0 = std::time::Instant::now();
    let c = couple(&spk, eop.as_ref(), &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, k.total, now).unwrap();
    println!("\n== COUPLE ({:?})\n{}", t0.elapsed(), c.headline);
    for n in &c.notes { println!("  · {n}"); }
    for r in &c.a_relief { println!("  A {} {:+.0} {}", fmt(r.at), r.gain * 100.0, r.what); }
    for r in &c.b_relief { println!("  B {} {:+.0} {}", fmt(r.at), r.gain * 100.0, r.what); }
    let line: Vec<String> = c.series.iter().step_by(3).map(|s| format!("{:.0}", s.3 * 100.0)).collect();
    println!("  together: {}", line.join(" "));

    let t0 = std::time::Instant::now();
    let q = couple_ask(&spk, eop.as_ref(), &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Question::WillItReturn, now).unwrap();
    println!("\n== COUPLE ASK ({:?})\n{}", t0.elapsed(), q.headline);

    let t0 = std::time::Instant::now();
    let tl = timeline(&spk, &a, Ayanamsa::Lahiri, 76.2673, now);
    println!("\n== BOOK ({:?})\n{}", t0.elapsed(), tl.headline);
    for s in &tl.saturn { println!("  {}", s.words); }
    for ch in &tl.chapters { println!("  {}{:<8} {} – {}  tone {:.0}  {}", if ch.current { "*" } else { " " }, format!("{:?}", ch.lord), fmt(ch.start), fmt(ch.end), ch.tone * 100.0, ch.words); }
    for r in &tl.returns { println!("  {}", r.words); }
}
