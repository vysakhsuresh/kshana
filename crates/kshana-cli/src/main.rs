//! `kshana chart --spk data/de421.bsp --utc 2026-10-15T09:02:07.418263 --lat 9.9312 --lon 76.2673`
//! `kshana next  --spk data/de421.bsp --utc 2026-10-15T00:00:00 --days 30 --tz +05:30`
//!
//! Prints the instant on every time scale, apparent tropical and sidereal longitudes,
//! Ascendant, Midheaven, Placidus cusps and the panchanga. This is the verification
//! harness: its output is what gets compared against Horizons and other ephemerides.

mod next;

use kshana_ephem::{Body, Spk};
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_truncated,
};
use kshana_houses::{cusps, HouseSystem};
use kshana_sky::{apparent, mean_node_longitude};
use kshana_time::{delta_t_for_year, ut1_jd_from_delta_t, EopTable, Instant, TwoPartJd, Utc};
use kshana_vedic::{ayanamsa_degrees, karana, nakshatra, sidereal, tithi, yoga, Ayanamsa};

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

fn parse_tz(s: &str) -> Option<i64> {
    let (sign, rest) = match s.chars().next()? {
        '+' => (1, &s[1..]),
        '-' => (-1, &s[1..]),
        _ => (1, s),
    };
    let (h, m) = rest.split_once(':').unwrap_or((rest, "0"));
    Some(sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?))
}

fn main() {
    let sub = std::env::args().nth(1).unwrap_or_default();
    let spk_path = arg("--spk").unwrap_or_else(|| "data/de421.bsp".into());
    let utc = arg("--utc").and_then(|s| parse_utc(&s)).unwrap_or_else(|| {
        eprintln!("usage: kshana chart --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS.ffffff --lat <deg> --lon <deg east>");
        eprintln!("       kshana next  --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS --days <n> [--tz +05:30]");
        std::process::exit(2);
    });
    if sub == "next" {
        let spk = Spk::open(std::path::Path::new(&spk_path)).unwrap_or_else(|e| {
            eprintln!("cannot open {spk_path}: {e:?}");
            std::process::exit(1);
        });
        let days: i64 = arg("--days").and_then(|s| s.parse().ok()).unwrap_or(30);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        next::run(&spk, utc.to_instant().expect("instant"), days, tz);
        return;
    }
    let lat: f64 = arg("--lat").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let lon: f64 = arg("--lon").and_then(|s| s.parse().ok()).unwrap_or(0.0);

    let spk = match Spk::open(std::path::Path::new(&spk_path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot open {spk_path}: {e:?}");
            std::process::exit(1);
        }
    };
    let t: Instant = utc.to_instant().expect("instant");
    let tt = t.tt_jd();
    let tc = t.tt_centuries_since_j2000();

    // UT1: Bulletin A if present, else ΔT.
    let eop = std::fs::read_to_string("data/finals2000A.all")
        .ok()
        .map(|s| EopTable::parse(&s));
    let mjd_utc = {
        let d = utc.utc_day() as f64 + 40_587.0;
        d + (utc.hour as f64 * 3_600.0
            + utc.minute as f64 * 60.0
            + utc.second as f64
            + utc.nanos as f64 * 1e-9)
            / 86_400.0
    };
    let (ut1, ut1_source) = match eop.as_ref().and_then(|e| e.ut1_minus_utc(mjd_utc)) {
        Some(u) => {
            let utc_jd = TwoPartJd {
                jd1: 2_400_000.5 + mjd_utc.floor(),
                jd2: mjd_utc.fract(),
            };
            (
                TwoPartJd {
                    jd1: utc_jd.jd1,
                    jd2: utc_jd.jd2 + u.ut1_minus_utc / 86_400.0,
                },
                format!("IERS UT1−UTC {:+.4} s ±{:.4}", u.ut1_minus_utc, u.sigma),
            )
        }
        None => {
            let year = utc.year as f64 + (utc.month as f64 - 1.0) / 12.0;
            let dt = delta_t_for_year(year);
            (
                ut1_jd_from_delta_t(t, dt),
                format!("ΔT {:.3} s ±{:.2} (table)", dt.seconds, dt.sigma),
            )
        }
    };

    println!("KSHANA · chart harness");
    println!(
        "UTC   {:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}",
        utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second, utc.nanos
    );
    println!("TAI   {} ns since 1958", t.tai_ns());
    println!("TT    JD {:.1} + {:.12}", tt.jd1, tt.jd2);
    println!("UT1   JD {:.1} + {:.12}   ({ut1_source})", ut1.jd1, ut1.jd2);

    let era = earth_rotation_angle(ut1);
    let g = gast(era, tc);
    let lst = local_sidereal_time(g, lon.to_radians());
    let (_, deps) = nutation_truncated(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    println!(
        "LST   {:.6}° at {lat}°N {lon}°E · obliquity {:.6}°",
        lst.to_degrees(),
        eps.to_degrees()
    );

    let ay = ayanamsa_degrees(Ayanamsa::Lahiri, tc);
    println!("Ayanamsa (Lahiri) {:.6}°  = {}", ay, dms(ay));
    println!();
    println!(
        "{:<9} {:>16}  {:>16}  {:>9}  {:>8}",
        "body", "tropical", "sidereal", "lat", "°/day"
    );
    let bodies = [
        ("Sun", Body::Sun),
        ("Moon", Body::Moon),
        ("Mercury", Body::Mercury),
        ("Venus", Body::Venus),
        ("Mars", Body::MarsBarycenter),
        ("Jupiter", Body::JupiterBarycenter),
        ("Saturn", Body::SaturnBarycenter),
        ("Uranus", Body::UranusBarycenter),
        ("Neptune", Body::NeptuneBarycenter),
        ("Pluto", Body::PlutoBarycenter),
    ];
    let mut sun_lon = 0.0;
    let mut moon_lon = 0.0;
    for (name, body) in bodies {
        match apparent(&spk, body, t) {
            Ok(a) => {
                if name == "Sun" {
                    sun_lon = a.longitude;
                }
                if name == "Moon" {
                    moon_lon = a.longitude;
                }
                println!(
                    "{:<9} {:>16}  {:>16}  {:>+9.4}  {:>+8.4}",
                    name,
                    dms(a.longitude),
                    dms(sidereal(a.longitude, ay)),
                    a.latitude,
                    a.speed
                );
            }
            Err(e) => println!("{name:<9} error {e:?}"),
        }
    }
    let node = mean_node_longitude(tc);
    println!(
        "{:<9} {:>16}  {:>16}  {:>9}  {:>8}",
        "Rahu(m)",
        dms(node),
        dms(sidereal(node, ay)),
        "",
        ""
    );
    println!();
    match cusps(HouseSystem::Placidus, lst, eps, lat.to_radians()) {
        Ok(c) => {
            println!("Asc {}   MC {}   (Placidus)", dms(c[0]), dms(c[9]));
            for (i, cusp) in c.iter().enumerate() {
                print!("{:>2}:{}  ", i + 1, dms(*cusp));
                if i % 3 == 2 {
                    println!();
                }
            }
        }
        Err(e) => println!("houses: {e:?}"),
    }
    println!();
    let ms = sidereal(moon_lon, ay);
    let ss = sidereal(sun_lon, ay);
    let n = nakshatra(ms);
    let ti = tithi(ms, ss);
    let y = yoga(ms, ss);
    let k = karana(ms, ss);
    println!(
        "Panchanga: {} pada {} ({:.1}% done) · {} {} ({:.1}%) · {} ({:.1}%) · {}",
        n.name(),
        n.pada,
        n.fraction * 100.0,
        if ti.is_shukla() { "Shukla" } else { "Krishna" },
        ti.name(),
        ti.fraction * 100.0,
        y.name(),
        y.fraction * 100.0,
        k.name
    );
}
