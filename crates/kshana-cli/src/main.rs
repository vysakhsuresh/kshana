//! `kshana chart --spk data/de421.bsp --utc 2026-10-15T09:02:07.418263 --lat 9.9312 --lon 76.2673`
//! `kshana next  --spk data/de421.bsp --utc 2026-10-15T00:00:00 --days 30 --tz +05:30`
//! `kshana areas --spk data/de421.bsp --utc <birth UTC> --lat .. --lon ..`
//! `kshana prashna --spk data/de421.bsp --utc <now UTC> --lat .. --lon .. --area Career [--number 1..249] [--tz +05:30]`
//! `kshana rectify --spk data/de421.bsp --utc <birth center UTC> --lat .. --lon .. --window-min 15 --event Career=2013-07-01T04:00:00 --event Marriage=2018-11-22T05:00:00 [--tz +05:30]`
//! `kshana muhurta --spk data/de421.bsp --utc <start UTC> --days 7 --lat .. --lon .. --activity Marriage [--natal-moon <sidereal deg>] [--tz +05:30]`
//!
//! The verification harness. `chart` prints everything `kshana-chart` computes; `next`
//! lists upcoming exact instants from `kshana-events`. Nothing here does astronomy.

mod next;

use kshana_chart::{compute, compute_horary, Chart, ChartInput};
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
    // round to 0.01″ first so 59.999″ never prints as 60.00″
    let total = (deg.rem_euclid(360.0) * 360_000.0).round() / 360_000.0;
    let sign = total.rem_euclid(30.0);
    let d = sign.floor();
    let m = ((sign - d) * 60.0).floor();
    let s = ((sign - d) * 60.0 - m) * 60.0;
    let deg = total;
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

fn print_areas(ch: &Chart) {
    use kshana_rules::{assess, promise, yogas, Facts, AREAS};
    let facts = Facts::from_chart(ch);
    println!("KSHANA · life areas · KP promise (KS-K-0001) and Parashari assessment (KS-B-*)");
    for a in AREAS {
        let p = promise(&facts, a);
        let b = assess(&facts, a);
        let agree = p.promised == (b.score >= 0.5);
        println!();
        println!(
            "{:<10} KP {} ({:.2}) · Parashari {:.2} · {}",
            a.name,
            if p.promised {
                "promised"
            } else {
                "not promised"
            },
            p.strength,
            b.score,
            if agree {
                "both traditions agree"
            } else {
                "traditions disagree"
            }
        );
        for f in &p.hit.factors {
            println!("    KP · {f}");
        }
        for h in &b.hits {
            for f in &h.factors {
                println!("    {} · {f}", h.rule_id);
            }
        }
    }
    println!();
    let ys = yogas(&facts);
    if ys.is_empty() {
        println!("Yogas: none of the detected set");
    } else {
        println!("Yogas:");
        for y in ys {
            println!(
                "  {:<18} {}  [{}] {}",
                y.name,
                y.hit.factors.join("; "),
                y.hit.rule_id,
                y.hit.source
            );
        }
    }
    println!();
    println!("Significators (house: level 1 star-of-occupant … 4 owner) · dignity · rasi house");
    for pl in &facts.planets {
        let sig = kshana_rules::significators(&facts, pl.graha);
        let list: Vec<String> = sig
            .iter()
            .map(|s| format!("{}:{}", s.house, s.level as u8))
            .collect();
        println!(
            "  {:<8} {:?} · rasi {:>2} · star {:?} · sub {:?} · {}",
            format!("{:?}", pl.graha),
            kshana_rules::dignity(pl.graha, pl.sidereal),
            pl.rasi_house,
            pl.star_lord,
            pl.sub_lord,
            list.join(" ")
        );
    }
}

fn print_prashna(spk: &Spk, ch: &Chart, area_name: &str, number: Option<u16>, lon: f64, tz: i64) {
    use kshana_rules::{area, prashna, prashna_windows, Facts, QuestionShape};
    let Some(a) = area(area_name) else {
        eprintln!("unknown area {area_name}; one of Career, Marriage, Children, Wealth, Health, Property, Education, Foreign");
        std::process::exit(2);
    };
    let facts = Facts::from_chart(ch);
    let v = prashna(&facts, a, QuestionShape::WillItHappen, ch.instant, lon);
    println!(
        "KSHANA · prashna · {} · {}",
        a.name,
        match number {
            Some(n) => format!("KP number {n}"),
            None => "lagna of the moment".into(),
        }
    );
    println!(
        "lagna {} sidereal · Moon {} sidereal",
        dms(ch.cusps_sidereal[0]),
        dms(ch.bodies[1].sidereal)
    );
    println!();
    println!(
        "VERDICT: {:?}{}",
        v.verdict,
        if v.question_genuine {
            ""
        } else {
            "  (question not shown by the Moon's sub-lord)"
        }
    );
    for h in &v.hits {
        println!("  [{}] {}", h.rule_id, h.source);
        for f in &h.factors {
            println!("      · {f}");
        }
    }
    println!();
    let agents = v.agents.clone();
    if agents.is_empty() {
        println!("No ruling planet signifies the matter: no timing window from this chart.");
        return;
    }
    let windows = prashna_windows(spk, Ayanamsa::Lahiri, &agents, ch.instant, 45);
    println!(
        "Moon transits through stars and subs of {:?} in the next 45 days:",
        agents
    );
    for w in windows.iter().take(12) {
        println!(
            "  {} → {}   star {:?} · sub {:?}",
            next::fmt_local(w.start, tz),
            next::fmt_local(w.end, tz),
            w.star,
            w.sub
        );
    }
}

/// Now, from the system clock, to the nanosecond (leap seconds via the UTC table).
fn now_instant() -> kshana_time::Instant {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs() as i64;
    let (y, m, day) = kshana_time::civil_from_days(secs.div_euclid(86_400));
    let sod = secs.rem_euclid(86_400) as u32;
    Utc::new(
        y,
        m,
        day,
        sod / 3600,
        (sod / 60) % 60,
        sod % 60,
        d.subsec_nanos(),
    )
    .and_then(|u| u.to_instant())
    .expect("system clock within the UTC table")
}

fn main() {
    let sub = std::env::args().nth(1).unwrap_or_default();
    let spk_path = arg("--spk").unwrap_or_else(|| "data/de421.bsp".into());
    let utc = arg("--utc").and_then(|s| parse_utc(&s)).unwrap_or_else(|| {
        eprintln!("usage: kshana chart --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS.ffffff --lat <deg> --lon <deg east> [--system placidus|equal|whole|porphyry] [--topo]");
        eprintln!("       kshana next  --spk <file.bsp> --utc YYYY-MM-DDTHH:MM:SS --days <n> [--tz +05:30]");
        eprintln!("       kshana life|areas|prashna|muhurta|rectify|match|calendar ... (see README)");
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
    if sub == "trace" {
        let days: i64 = arg("--days").and_then(|s| s.parse().ok()).unwrap_or(1);
        let depth: usize = arg("--depth").and_then(|s| s.parse().ok()).unwrap_or(5);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let moon: f64 = arg("--natal-moon").and_then(|s| s.parse().ok()).unwrap_or_else(|| {
            eprintln!("trace needs --natal-moon <sidereal deg> (from `kshana chart` of the birth) and --utc <window start>");
            std::process::exit(2);
        });
        let birth = arg("--birth")
            .and_then(|s| parse_utc(&s))
            .and_then(|u| u.to_instant().ok())
            .unwrap_or_else(|| {
                eprintln!("trace needs --birth YYYY-MM-DDTHH:MM:SS");
                std::process::exit(2);
            });
        let who = kshana_trace::Person {
            birth,
            moon_sidereal: moon,
            observer: input.observer,
            ayanamsa: Ayanamsa::Lahiri,
            year_ns: kshana_vedic::dasha::YEAR_365_25_NS,
        };
        let from = input.instant;
        let to = from.add_ns(days as i128 * 86_400 * 1_000_000_000);
        let opts = kshana_trace::TraceOptions {
            dasha_depth: depth,
            ..kshana_trace::TraceOptions::default()
        };
        let t = kshana_trace::trace(&spk, eop.as_ref(), &who, from, to, opts).unwrap_or_else(|e| {
            eprintln!("trace: {e:?}");
            std::process::exit(1);
        });
        println!(
            "KSHANA · life trace · {} transitions · local UTC{}",
            t.len(),
            arg("--tz").unwrap_or_default()
        );
        for x in &t {
            let u = Utc::from_instant(x.at.add_ns(tz as i128 * 60_000_000_000));
            println!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}  ±{:<6} {}",
                u.year,
                u.month,
                u.day,
                u.hour,
                u.minute,
                u.second,
                u.nanos / 1_000,
                format!("{}µs", x.half_width_ns / 1_000),
                x.describe()
            );
        }
        return;
    }
    if sub == "dashas" {
        use kshana_vedic::dashas::{self as ds, Period};
        use kshana_vedic::DashaLord as L;
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let at = arg("--at")
            .and_then(|s| parse_utc(&s))
            .and_then(|u| u.to_instant().ok())
            .unwrap_or_else(now_instant);
        let ch = compute(&spk, eop.as_ref(), &input).unwrap_or_else(|e| {
            eprintln!("dashas: {e:?}");
            std::process::exit(1);
        });
        let birth = ch.instant;
        let year_ns = kshana_vedic::dasha::YEAR_365_25_NS;
        let sign_of = |lon: f64| ((lon.rem_euclid(360.0) / 30.0).floor() as usize).min(11);
        let sidereal = |g: L| -> f64 {
            match g {
                L::Rahu => ch.true_node.1,
                L::Ketu => (ch.true_node.1 + 180.0).rem_euclid(360.0),
                other => {
                    let name = ds::graha_name(other);
                    ch.bodies
                        .iter()
                        .find(|b| b.name == name)
                        .map_or(0.0, |b| b.sidereal)
                }
            }
        };
        let moon = sidereal(L::Moon);
        let lagna = ch.cusps_sidereal[0];
        let grahas: [f64; 9] = [
            L::Sun,
            L::Moon,
            L::Mars,
            L::Mercury,
            L::Jupiter,
            L::Venus,
            L::Saturn,
            L::Rahu,
            L::Ketu,
        ]
        .map(sidereal);
        let applies = ds::ashtottari_applies(
            sign_of(lagna),
            sign_of(sidereal(kshana_vedic::sign_lord(sign_of(lagna)))),
            sign_of(ch.true_node.1),
        );
        let vim_period = |p: kshana_vedic::DashaPeriod, level: u8| Period {
            label: ds::graha_name(p.lord).to_string(),
            start: p.start,
            end: p.end,
            level,
        };
        let vim: Vec<Period> = kshana_vedic::vimshottari(moon, birth, year_ns)
            .into_iter()
            .map(|p| vim_period(p, 1))
            .collect();
        let kc = ds::kalachakra_detail(moon, birth, year_ns);
        let (row, first) = (kc.row, kc.first_position);
        type SubFn<'a> = Box<dyn Fn(usize, &Period) -> Vec<Period> + 'a>;
        let systems: Vec<(&str, Vec<Period>, SubFn<'_>)> = vec![
            (
                "Vimshottari",
                vim,
                Box::new(|_, p: &Period| {
                    let Some(lord) = ds::graha_from_name(&p.label) else {
                        return Vec::new();
                    };
                    kshana_vedic::subdivide(kshana_vedic::DashaPeriod {
                        lord,
                        start: p.start,
                        end: p.end,
                    })
                    .into_iter()
                    .map(|q| vim_period(q, 2))
                    .collect()
                }),
            ),
            (
                "Yogini",
                ds::yogini(moon, birth, year_ns),
                Box::new(|_, p: &Period| ds::yogini_sub(p)),
            ),
            (
                "Ashtottari",
                ds::ashtottari(moon, birth, year_ns),
                Box::new(|_, p: &Period| ds::ashtottari_sub(p)),
            ),
            (
                "Kalachakra",
                kc.periods,
                Box::new(move |k, p: &Period| ds::kalachakra_sub(p, &row, (first + k) % 9)),
            ),
            (
                "Chara",
                ds::chara(lagna, &grahas, birth, year_ns),
                Box::new(|_, p: &Period| {
                    ds::sign_index(&p.label)
                        .map(|s| ds::chara_sub(p, ds::chara_direction(s)))
                        .unwrap_or_default()
                }),
            ),
        ];
        let fmt = |t: kshana_time::Instant| {
            let u = Utc::from_instant(t.add_ns(tz as i128 * 60_000_000_000));
            format!("{:04}-{:02}-{:02}", u.year, u.month, u.day)
        };
        println!(
            "KSHANA · dashas at {} · birth {} · Moon {:.3}° sidereal · local UTC{}",
            fmt(at),
            fmt(birth),
            moon,
            arg("--tz").unwrap_or_default()
        );
        for (name, periods, sub) in &systems {
            let Some(k) = periods.iter().position(|p| p.start <= at && at < p.end) else {
                println!("{name:<12} no period covers this instant");
                continue;
            };
            let p1 = &periods[k];
            let subs = sub(k, p1);
            let flag = if *name == "Ashtottari" && !applies {
                "  (BPHS: does not apply to this chart)"
            } else {
                ""
            };
            match ds::period_at(&subs, at) {
                Some(p2) => println!(
                    "{name:<12} {:<12} {} – {}  ›  {:<12} {} – {}{flag}",
                    p1.label,
                    fmt(p1.start),
                    fmt(p1.end),
                    p2.label,
                    fmt(p2.start),
                    fmt(p2.end)
                ),
                None => println!(
                    "{name:<12} {:<12} {} – {}{flag}",
                    p1.label,
                    fmt(p1.start),
                    fmt(p1.end)
                ),
            }
        }
        return;
    }
    if sub == "varsha" {
        let year: i64 = arg("--year").and_then(|s| s.parse().ok()).unwrap_or(2026);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let natal = compute(&spk, eop.as_ref(), &input).unwrap_or_else(|e| {
            eprintln!("varsha: {e:?}");
            std::process::exit(1);
        });
        let v = kshana_tajika::varshaphala(
            &spk,
            eop.as_ref(),
            &natal,
            &input.observer,
            Ayanamsa::Lahiri,
            year,
        )
        .unwrap_or_else(|e| {
            eprintln!("varsha: {e:?}");
            std::process::exit(1);
        });
        let local = |i: kshana_time::Instant| {
            let u = Utc::from_instant(i.add_ns(tz as i128 * 60_000_000_000));
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
        };
        println!(
            "KSHANA · varshaphala {} · age {} · return {} ({})",
            v.year,
            v.age,
            local(v.instant),
            if v.day { "day" } else { "night" }
        );
        println!(
            "Muntha {} in house {} · lord {:?} · Varshesha {:?}",
            kshana_speak::SIGN_NAMES[v.muntha.0 % 12],
            v.muntha.1,
            v.muntha_lord,
            v.varshesha
        );
        for r in &v.reasoning {
            println!("  · {r}");
        }
        println!("Sahams:");
        for sh in v.sahams.iter().take(12) {
            println!(
                "  {:<14} {:>8.3}°  house {:<2} {}",
                sh.name, sh.longitude, sh.house, sh.meaning
            );
        }
        println!("Yogas:");
        for y in &v.yogas {
            println!("  {}", y.words);
        }
        println!("Mudda:");
        for m in &v.mudda {
            println!(
                "  {:<8} {} → {}",
                format!("{:?}", m.lord),
                local(m.start),
                local(m.end)
            );
        }
        return;
    }
    if sub == "directions" {
        let at = arg("--at")
            .and_then(|s| parse_utc(&s))
            .and_then(|u| u.to_instant().ok())
            .unwrap_or_else(now_instant);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let natal = compute(&spk, eop.as_ref(), &input).unwrap_or_else(|e| {
            eprintln!("directions: {e:?}");
            std::process::exit(1);
        });
        let local = |i: kshana_time::Instant| {
            let u = Utc::from_instant(i.add_ns(tz as i128 * 60_000_000_000));
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                u.year, u.month, u.day, u.hour, u.minute, u.second
            )
        };
        match kshana_directions::progressions(&spk, &natal, lat, at) {
            Ok(p) => {
                println!("Secondary progressions at {} (progressed instant {}): arc {:.4}°, MC {:.3}°, Asc {:.3}°, Moon phase {:.1}° ({})", local(at), local(p.instant), p.solar_arc, p.mc, p.asc, p.moon_phase_deg, p.lunation_phase);
                for b in &p.bodies {
                    println!("  {:<8} {:>9.4}°  {:+.4}°/d", b.name, b.longitude, b.speed);
                }
            }
            Err(e) => eprintln!("progressions: {e:?}"),
        }
        if let Ok(pf) = kshana_directions::profections(&spk, &natal, at) {
            println!(
                "Profection: age {} · year {:?} ({:?}) {} → {} · month {} {:?} ({:?})",
                pf.age,
                pf.year_sign,
                pf.year_lord,
                local(pf.year_start),
                local(pf.year_end),
                pf.month,
                pf.month_sign,
                pf.month_lord
            );
        }
        let day = kshana_directions::is_day_at(&spk, eop.as_ref(), &input.observer, natal.instant)
            .unwrap_or(true);
        if let Some(f) = kshana_directions::firdaria_at(natal.instant, day, at) {
            println!(
                "Firdaria: {} / {} · {} → {}",
                f.lord.name(),
                f.sub.map(|s| s.name()).unwrap_or("—"),
                local(f.start),
                local(f.end)
            );
        }
        return;
    }
    if sub == "life" {
        let ch = compute(&spk, eop.as_ref(), &input).unwrap_or_else(|e| {
            eprintln!("life: {e:?}");
            std::process::exit(1);
        });
        let at = arg("--at")
            .and_then(|s| parse_utc(&s))
            .and_then(|u| u.to_instant().ok())
            .unwrap_or_else(now_instant);
        for sec in kshana_speak::reading(&spk, &ch, at, Ayanamsa::Lahiri) {
            println!("\n== {} — {}", sec.title, sec.summary);
            for st in sec.sentences {
                println!("  {}  [{:.0}%]", st.text, st.confidence * 100.0);
                for w in st.why {
                    println!("      · {w}");
                }
            }
        }
        return;
    }
    if sub == "match" {
        let b_utc = arg("--utc2")
            .and_then(|s| parse_utc(&s))
            .unwrap_or_else(|| {
                eprintln!("match needs --utc2 YYYY-MM-DDTHH:MM:SS --lat2 <deg> --lon2 <deg east>");
                std::process::exit(2);
            });
        let lat2: f64 = arg("--lat2").and_then(|s| s.parse().ok()).unwrap_or(lat);
        let lon2: f64 = arg("--lon2").and_then(|s| s.parse().ok()).unwrap_or(lon);
        let input_b = ChartInput {
            instant: b_utc.to_instant().expect("instant"),
            observer: Observer {
                latitude_deg: lat2,
                longitude_deg: lon2,
                height_m: 0.0,
            },
            ..input
        };
        let (a, b) = match (
            compute(&spk, eop.as_ref(), &input),
            compute(&spk, eop.as_ref(), &input_b),
        ) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("match: {e:?}");
                std::process::exit(1);
            }
        };
        let ash = kshana_rules::ashtakoota(a.bodies[1].sidereal, b.bodies[1].sidereal);
        let fa = kshana_rules::Facts::from_chart(&a);
        let fb = kshana_rules::Facts::from_chart(&b);
        let ma = kshana_rules::mangal_dosha(&fa);
        let mb = kshana_rules::mangal_dosha(&fb);
        let syn = kshana_rules::synastry(&a, &b, 6.0);
        println!("Ashtakoota {:.1} / 36 — {}", ash.total, ash.verdict);
        for k in &ash.kutas {
            println!(
                "  {:<13} {:>4.1} / {:<3}  {}",
                k.name, k.points, k.max, k.detail
            );
        }
        for st in kshana_speak::compatibility(&ash, &ma, &mb, &syn) {
            println!("- {}", st.text);
        }
        return;
    }
    if sub == "calendar" {
        let days: u32 = arg("--days").and_then(|s| s.parse().ok()).unwrap_or(60);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let obs = input.observer;
        let list = kshana_forecast::observances(
            &spk,
            eop.as_ref(),
            &obs,
            Ayanamsa::Lahiri,
            input.instant,
            days,
        )
        .unwrap_or_else(|e| {
            eprintln!("calendar: {e:?}");
            std::process::exit(1);
        });
        for o in list {
            let local = Utc::from_instant(o.exact.add_ns(tz as i128 * 60_000_000_000));
            let observed = o
                .observed_sunrise
                .map(|s| {
                    let u = Utc::from_instant(s.add_ns(tz as i128 * 60_000_000_000));
                    format!("observed {:04}-{:02}-{:02}", u.year, u.month, u.day)
                })
                .unwrap_or_default();
            println!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  {:<22} {}",
                local.year,
                local.month,
                local.day,
                local.hour,
                local.minute,
                local.second,
                o.name,
                observed
            );
        }
        return;
    }
    if sub == "rectify" {
        use kshana_rules::{area, rectify, LifeEvent};
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let window_min: i128 = arg("--window-min")
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);
        let mut events = Vec::new();
        let args: Vec<String> = std::env::args().collect();
        for (i, a) in args.iter().enumerate() {
            if a == "--event" {
                if let Some(spec) = args.get(i + 1) {
                    if let Some((name, when)) = spec.split_once('=') {
                        match (area(name), parse_utc(when)) {
                            (Some(ar), Some(u)) => events.push(LifeEvent {
                                area: ar,
                                at: u.to_instant().expect("instant"),
                                label: format!("{name} {when}"),
                            }),
                            _ => eprintln!("ignored event {spec}"),
                        }
                    }
                }
            }
        }
        if events.is_empty() {
            eprintln!("give at least one --event Area=YYYY-MM-DDTHH:MM:SS");
            std::process::exit(2);
        }
        let obs = Observer {
            latitude_deg: lat,
            longitude_deg: lon,
            height_m: 0.0,
        };
        let center = utc.to_instant().expect("instant");
        let c = rectify(
            &spk,
            eop.as_ref(),
            &obs,
            Ayanamsa::Lahiri,
            center,
            window_min * 60 * kshana_time::NS_PER_SEC,
            60 * kshana_time::NS_PER_SEC,
            &events,
        )
        .unwrap_or_else(|e| {
            eprintln!("rectify: {e:?}");
            std::process::exit(1);
        });
        println!("KSHANA · rectification · ±{window_min} min around the given time · {} events · {} candidates", events.len(), c.len());
        for cand in c.iter().take(6) {
            println!();
            println!(
                "{}   score {:.3}   asc {} · sub-lord {:?}",
                next::fmt_local(cand.birth, tz),
                cand.score,
                dms(cand.asc_sidereal),
                cand.asc_sub_lord
            );
            for e in &cand.per_event {
                println!("  {:<24} {:.2}", e.label, e.score);
                for f in &e.factors {
                    println!("      · {f}");
                }
            }
        }
        return;
    }
    if sub == "muhurta" {
        use kshana_rules::{muhurta, Activity};
        let days: i64 = arg("--days").and_then(|s| s.parse().ok()).unwrap_or(7);
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let act = arg("--activity")
            .and_then(|s| Activity::parse(&s))
            .unwrap_or(Activity::General);
        let natal: Option<f64> = arg("--natal-moon").and_then(|s| s.parse().ok());
        let obs = Observer {
            latitude_deg: lat,
            longitude_deg: lon,
            height_m: 0.0,
        };
        let from = utc.to_instant().expect("instant");
        let w = muhurta(
            &spk,
            eop.as_ref(),
            &obs,
            act,
            Ayanamsa::Lahiri,
            natal,
            from,
            days,
            0.5,
            12,
        )
        .unwrap_or_else(|e| {
            eprintln!("muhurta: {e:?}");
            std::process::exit(1);
        });
        println!(
            "KSHANA · muhurta · {} · next {days} days at {lat}°N {lon}°E",
            act.name()
        );
        for x in &w {
            println!();
            println!(
                "{} → {}   score {:.2}{}",
                next::fmt_local(x.start, tz),
                next::fmt_local(x.end, tz),
                x.score,
                if x.abhijit { " · Abhijit" } else { "" }
            );
            println!(
                "  {} · {} · {} yoga · {} hora · lagna {}",
                x.tithi, x.nakshatra, x.yoga, x.hora, x.lagna_sign
            );
            for f in &x.factors {
                println!("    + {f}");
            }
            for c in &x.cautions {
                println!("    − {c}");
            }
        }
        if w.is_empty() {
            println!("no window scored above the threshold; widen the range");
        }
        return;
    }
    if sub == "prashna" {
        let number: Option<u16> = arg("--number").and_then(|s| s.parse().ok());
        let tz = arg("--tz").and_then(|s| parse_tz(&s)).unwrap_or(0);
        let area_name = arg("--area").unwrap_or_else(|| "Career".into());
        let ch = match number {
            Some(n) => compute_horary(&spk, eop.as_ref(), &input, n),
            None => compute(&spk, eop.as_ref(), &input),
        };
        match ch {
            Ok(ch) => print_prashna(&spk, &ch, &area_name, number, lon, tz),
            Err(e) => {
                eprintln!("prashna: {e:?}");
                std::process::exit(1);
            }
        }
        return;
    }
    match compute(&spk, eop.as_ref(), &input) {
        Ok(ch) if sub == "areas" => print_areas(&ch),
        Ok(ch) => print_chart(&utc, &ch, lat, lon),
        Err(e) => {
            eprintln!("chart: {e:?}");
            std::process::exit(1);
        }
    }
}
