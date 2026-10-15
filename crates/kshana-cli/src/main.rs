//! `kshana chart --spk data/de421.bsp --utc 2026-10-15T09:02:07.418263 --lat 9.9312 --lon 76.2673`
//! `kshana next  --spk data/de421.bsp --utc 2026-10-15T00:00:00 --days 30 --tz +05:30`
//!
//! The verification harness. `chart` prints everything `kshana-chart` computes; `next`
//! lists upcoming exact instants from `kshana-events`. Nothing here does astronomy.

mod next;

use kshana_chart::{compute, Chart, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Utc};
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES, YOGA_NAMES};
use kshana_vedic::Ayanamsa;

fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args();
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
    }
    None
}

fn parse_utc(s: &str) -> Option<Utc> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let mo: u32 = d.next()?.parse().ok()?;
    let da: u32 = d.next()?.parse().ok()?;
    let mut t = time.trim_end_matches('Z').split(':');
    let h: u32 = t.next()?.parse().ok()?;
    let mi: u32 = t.next()?.parse().ok()?;
    let sec = t.next().unwrap_or("0");
    let (s_int, frac) = sec.split_once('.').unwrap_or((sec, ""));
    let s: u32 = s_int.parse().ok()?;
    let mut ns = 0u32;
    let mut scale = 100_000_000u32;
    for ch in frac.chars().take(9) {
        ns += ch.to_digit(10)? * scale;
        scale /= 10;
    }
    Utc::new(y, mo, da, h, mi, s, ns).ok()
}

fn parse_tz(s: &str) -> Option<i64> {
    let (sign, rest) = match s.chars().next()? {
        '+' => (1, &s[1..]),
        '-' => (-1, &s[1..]),
        _ => (1, s),
    };
    let (h, m) = rest.split_once(':').unwrap_or((rest, "0"));
    Some(sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?))
}

fn dms(deg: f64) -> String {
    let sign = deg.rem_euclid(30.0);
    let d = sign.floor();
    let m = ((sign - d) * 60.0).floor();
    let s = ((sign - d) * 60.0 - m) * 60.0;
    const SIGNS: [&str; 12] = [
        "Ari", "Tau", "Gem", "Can", "Leo", "Vir", "Lib", "Sco", "Sag", "Cap", "Aqu", "Pis",
    ];
    let idx = ((deg.rem_euclid(360.0) / 30.0).floor() as usize).min(11);
    format!("{:>2}°{:02}′{:05.2}″ {}", d, m, s, SIGNS[idx])
}

fn print_chart(utc: &Utc, ch: &Chart, lat: f64, lon: f64) {
    println!("KSHANA · chart harness");
    println!(
        "UTC   {:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}",
        utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second, utc.nanos
    );
    println!("TAI   {} ns since 1958", ch.instant.tai_ns());
    println!("TT    JD {:.1} + {:.12}", ch.tt.jd1, ch.tt.jd2);
    println!(
        "UT1   JD {:.1} + {:.12}   ({})",
        ch.ut1.jd1, ch.ut1.jd2, ch.provenance.ut1_source
    );
    println!(
        "LST   {:.6}° at {lat}°N {lon}°E · obliquity {:.6}°",
        ch.lst_deg, ch.obliquity_deg
    );
    println!(
        "Ayanamsa {:.6}°  = {}",
        ch.ayanamsa_deg,
        dms(ch.ayanamsa_deg)
    );
    println!();
    println!(
        "{:<9} {:>16}  {:>16}  {:>8}  {:>8}  {:>3}  {:<15} KP star·sub·sub²",
        "body", "tropical", "sidereal", "lat", "°/day", "H", "nakshatra"
    );
    for b in &ch.bodies {
        println!(
            "{:<9} {:>16}  {:>16}  {:>+8.4}  {:>+8.4}  {:>3}  {:<15} {:?}·{:?}·{:?}",
            b.name,
            dms(b.tropical),
            dms(b.sidereal),
            b.latitude,
            b.speed,
            b.house,
            format!("{} {}", NAKSHATRA_NAMES[b.nakshatra.0], b.nakshatra.1),
            b.kp.0,
            b.kp.1,
            b.kp.2
        );
    }
    println!(
        "{:<9} {:>16}  {:>16}",
        "Rahu(m)",
        dms(ch.mean_node.0),
        dms(ch.mean_node.1)
    );
    println!(
        "{:<9} {:>16}  {:>16}",
        "Rahu(t)",
        dms(ch.true_node.0),
        dms(ch.true_node.1)
    );
    println!();
    println!("Asc {}   MC {}", dms(ch.cusps[0]), dms(ch.cusps[9]));
    for (i, cusp) in ch.cusps.iter().enumerate() {
        print!("{:>2}:{}  ", i + 1, dms(*cusp));
        if i % 3 == 2 {
            println!();
        }
    }
    println!();
    let p = &ch.panchanga;
    println!(
        "Panchanga: {} pada {} ({:.1}% done) · {} {} ({:.1}%) · {} ({:.1}%) · {}",
        NAKSHATRA_NAMES[p.nakshatra.0],
        p.nakshatra.1,
        p.nakshatra.2 * 100.0,
        if p.tithi.0 <= 15 { "Shukla" } else { "Krishna" },
        TITHI_NAMES[((p.tithi.0 - 1) % 15) as usize],
        p.tithi.1 * 100.0,
        YOGA_NAMES[p.yoga.0],
        p.yoga.1 * 100.0,
        p.karana.1
    );
    println!();
    println!(
        "Provenance: {} · {} · {}",
        ch.provenance.engine, ch.provenance.kernel, ch.provenance.models
    );
}

fn main() {
    let sub = std::env::args().nth(1).unwrap_or_default();
    let spk_path = arg("--spk").unwrap_or_else(|| "data/de421.bsp".into());
    let utc = arg("--utc").and_then(|s| parse_utc(&s)).unwrap_or_else(|| {
        eprintln!("usage: kshana chart --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS.ffffff --lat <deg> --lon <deg east> [--system placidus|equal|whole|porphyry] [--topo]");
        eprintln!("       kshana next  --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS --days <n> [--tz +05:30]");
        std::process::exit(2);
    });
    let spk = Spk::open(std::path::Path::new(&spk_path)).unwrap_or_else(|e| {
        eprintln!("cannot open {spk_path}: {e:?}");
        std::process::exit(1);
    });
    if sub == "next" {
        let days: i64 = arg("--days").and_then(|s| s.parse().ok()).unwrap_or(30);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        next::run(&spk, utc.to_instant().expect("instant"), days, tz);
        return;
    }
    let lat: f64 = arg("--lat").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let lon: f64 = arg("--lon").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let system = match arg("--system").as_deref() {
        Some("equal") => HouseSystem::Equal,
        Some("whole") => HouseSystem::WholeSign,
        Some("porphyry") => HouseSystem::Porphyry,
        _ => HouseSystem::Placidus,
    };
    let eop = std::fs::read_to_string("data/finals2000A.all")
        .ok()
        .map(|s| EopTable::parse(&s));
    let input = ChartInput {
        instant: utc.to_instant().expect("instant"),
        observer: Observer {
            latitude_deg: lat,
            longitude_deg: lon,
            height_m: 0.0,
        },
        system,
        ayanamsa: Ayanamsa::Lahiri,
        topocentric_moon: std::env::args().any(|a| a == "--topo"),
    };
    match compute(&spk, eop.as_ref(), &input) {
        Ok(ch) => print_chart(&utc, &ch, lat, lon),
        Err(e) => {
            eprintln!("chart: {e:?}");
            std::process::exit(1);
        }
    }
}
