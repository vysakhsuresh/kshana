//! Muhurta: the best windows for an activity in a date range at a place.
//!
//! The day is scanned in five-minute steps. Each step is scored by the classical
//! considerations (Muhurta Chintamani, Muhurta Martanda, and the panchanga tradition
//! as practised): the five limbs, the day's inauspicious periods (Rahu kala, Yamaganda,
//! Gulika kala), the planetary hour, the rising sign and its lord, Abhijit, and, when
//! the person's natal Moon is known, Tara bala and Chandra bala. Hard prohibitions
//! zero a step; everything else adds. Contiguous good steps become windows, ranked.
//! Every window lists the factors that made it and the cautions that remain.

use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{hora_at, rahu_kala, solar_day, SolarDay};
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_iau2006a,
};
use kshana_houses::angles;
use kshana_sky::{apparent_longitude, Observer};
use kshana_time::{EopTable, Instant, Utc, NS_PER_SEC};
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES, YOGA_NAMES};
use kshana_vedic::{
    ayanamsa_degrees, karana, nakshatra, sidereal, sign_lord, tithi, yoga, Ayanamsa, DashaLord,
};

/// What the time is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Activity {
    Marriage,
    Travel,
    Business,
    Housewarming,
    Learning,
    Medical,
    General,
}

impl Activity {
    /// Parse a name (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "marriage" | "wedding" => Activity::Marriage,
            "travel" | "journey" => Activity::Travel,
            "business" | "venture" | "work" => Activity::Business,
            "housewarming" | "home" | "grihapravesh" => Activity::Housewarming,
            "learning" | "study" | "education" | "vidyarambha" => Activity::Learning,
            "medical" | "treatment" | "surgery" => Activity::Medical,
            "general" | "any" => Activity::General,
            _ => return None,
        })
    }

    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            Activity::Marriage => "Marriage",
            Activity::Travel => "Travel",
            Activity::Business => "Business",
            Activity::Housewarming => "Housewarming",
            Activity::Learning => "Learning",
            Activity::Medical => "Medical",
            Activity::General => "General",
        }
    }

    /// Nakshatras (0-based) the tradition names for the activity.
    fn nakshatras(self) -> &'static [usize] {
        match self {
            // Rohini, Mrigashira, Magha, U.Phalguni, Hasta, Swati, Anuradha, Mula, U.Ashadha, U.Bhadrapada, Revati
            Activity::Marriage => &[3, 4, 9, 11, 12, 14, 16, 18, 20, 25, 26],
            // Ashwini, Mrigashira, Punarvasu, Pushya, Hasta, Anuradha, Shravana, Dhanishta, Revati
            Activity::Travel => &[0, 4, 6, 7, 12, 16, 21, 22, 26],
            // Ashwini, Rohini, Mrigashira, Pushya, U.Phalguni, Hasta, Chitra, Anuradha, U.Ashadha, Shravana, U.Bhadrapada, Revati
            Activity::Business => &[0, 3, 4, 7, 11, 12, 13, 16, 20, 21, 25, 26],
            // Rohini, Mrigashira, Pushya, U.Phalguni, Chitra, Anuradha, U.Ashadha, U.Bhadrapada, Revati
            Activity::Housewarming => &[3, 4, 7, 11, 13, 16, 20, 25, 26],
            // Ashwini, Punarvasu, Pushya, Hasta, Chitra, Swati, Anuradha, Shravana, Revati
            Activity::Learning => &[0, 6, 7, 12, 13, 14, 16, 21, 26],
            // Ashwini, Rohini, Mrigashira, Punarvasu, Pushya, U.Phalguni, Hasta, Chitra, Swati, Anuradha, U.Ashadha, Shravana, Dhanishta, Shatabhisha, U.Bhadrapada, Revati
            Activity::Medical => &[0, 3, 4, 6, 7, 11, 12, 13, 14, 16, 20, 21, 22, 23, 25, 26],
            // the universally auspicious set
            Activity::General => &[0, 3, 4, 6, 7, 11, 12, 13, 14, 16, 20, 21, 22, 25, 26],
        }
    }

    /// Weekday lords the tradition prefers (Sun=0 … Sat=6).
    fn good_days(self) -> &'static [u32] {
        match self {
            Activity::Marriage => &[1, 3, 4, 5],
            Activity::Travel => &[1, 3, 4, 5],
            Activity::Business => &[0, 1, 3, 4, 5],
            Activity::Housewarming => &[1, 3, 4, 5],
            Activity::Learning => &[0, 1, 3, 4, 5],
            Activity::Medical => &[1, 3, 4, 5],
            Activity::General => &[0, 1, 3, 4, 5],
        }
    }

    /// Hora lords that suit the activity.
    fn good_horas(self) -> &'static [&'static str] {
        match self {
            Activity::Marriage => &["Venus", "Jupiter", "Moon", "Mercury"],
            Activity::Travel => &["Moon", "Mercury", "Jupiter", "Venus"],
            Activity::Business => &["Mercury", "Jupiter", "Venus", "Sun"],
            Activity::Housewarming => &["Jupiter", "Venus", "Moon"],
            Activity::Learning => &["Mercury", "Jupiter", "Venus"],
            Activity::Medical => &["Jupiter", "Sun", "Moon", "Venus"],
            Activity::General => &["Jupiter", "Venus", "Mercury", "Moon"],
        }
    }

    /// Whether a fixed rising sign is preferred (permanence) or a movable one (motion).
    fn lagna_preference(self) -> Option<u8> {
        match self {
            Activity::Marriage | Activity::Housewarming | Activity::Business => Some(1), // fixed
            Activity::Travel => Some(0),                                                 // movable
            _ => None,
        }
    }
}

/// One recommended window.
#[derive(Clone, Debug, PartialEq)]
pub struct MuhurtaWindow {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Mean score in `[0, 1]`.
    pub score: f64,
    /// Rising sign index at the start (sidereal), 0 = Aries.
    pub lagna_sign: u8,
    /// Tithi, nakshatra, yoga, hora at the start.
    pub tithi: String,
    /// Nakshatra name.
    pub nakshatra: String,
    /// Yoga name.
    pub yoga: String,
    /// Hora lord.
    pub hora: String,
    /// Contains Abhijit.
    pub abhijit: bool,
    /// Why it scores.
    pub factors: Vec<String>,
    /// What still argues against it.
    pub cautions: Vec<String>,
}

struct Step {
    t: Instant,
    score: f64,
    lagna_sign: u8,
    tithi: String,
    nakshatra: String,
    yoga: String,
    hora: String,
    abhijit: bool,
    factors: Vec<String>,
    cautions: Vec<String>,
}

fn ordinal(n: i64) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        n => format!("{n}th"),
    }
}

fn weekday(day: &SolarDay, obs: &Observer) -> u32 {
    let local = day
        .sunrise
        .add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

/// Yamaganda and Gulika slots (1-based eighth of the daytime) by weekday Sun..Sat.
const YAMAGANDA: [usize; 7] = [5, 4, 3, 2, 1, 7, 6];
const GULIKA: [usize; 7] = [7, 6, 5, 4, 3, 2, 1];

fn slot_window(day: &SolarDay, slot: usize) -> (Instant, Instant) {
    let len = day.sunset.ns_since(day.sunrise) / 8;
    (
        day.sunrise.add_ns((slot as i128 - 1) * len),
        day.sunrise.add_ns(slot as i128 * len),
    )
}

fn abhijit_window(day: &SolarDay) -> (Instant, Instant) {
    // the eighth of fifteen daytime muhurtas: centred on local noon, one fifteenth of the day long
    let len = day.sunset.ns_since(day.sunrise);
    let noon = day.sunrise.add_ns(len / 2);
    (noon.add_ns(-len / 30), noon.add_ns(len / 30))
}

fn in_window(t: Instant, w: (Instant, Instant)) -> bool {
    t >= w.0 && t < w.1
}

#[allow(clippy::too_many_arguments)]
fn score_step(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    activity: Activity,
    ayanamsa: Ayanamsa,
    natal_moon_sidereal: Option<f64>,
    day: &SolarDay,
    t: Instant,
) -> Result<Step, SpkError> {
    let tc = t.tt_centuries_since_j2000();
    let ay = ayanamsa_degrees(ayanamsa, tc);
    let sun = sidereal(apparent_longitude(spk, Body::Sun, t)?, ay);
    let moon = sidereal(apparent_longitude(spk, Body::Moon, t)?, ay);
    let n = nakshatra(moon);
    let ti = tithi(moon, sun);
    let y = yoga(moon, sun);
    let k = karana(moon, sun);
    let wd = weekday(day, obs);
    let (hora, _, _) = hora_at(day, obs, t);

    // rising sign (sidereal) and its lord's placement in the sky of the moment
    let (ut1, _) = kshana_chart::ut1_for(t, eop);
    let era = earth_rotation_angle(ut1);
    let lst = local_sidereal_time(gast(era, tc), obs.longitude_deg.to_radians());
    let (_, deps) = nutation_iau2006a(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let asc = sidereal(angles(lst, eps, obs.latitude_deg.to_radians()).asc, ay);
    let lagna_sign = ((asc / 30.0).floor() as usize % 12) as u8;

    let mut score: f64 = 0.0;
    let mut factors = Vec::new();
    let mut cautions = Vec::new();
    let mut blocked = false;

    // Hard prohibitions
    if in_window(t, rahu_kala(day, obs)) {
        blocked = true;
        cautions.push("Rahu kala".into());
    }
    if in_window(t, slot_window(day, YAMAGANDA[wd as usize])) {
        blocked = true;
        cautions.push("Yamaganda".into());
    }
    if in_window(t, slot_window(day, GULIKA[wd as usize])) {
        blocked = true;
        cautions.push("Gulika kala".into());
    }
    if k.name == "Vishti" {
        blocked = true;
        cautions.push("Vishti (Bhadra) karana".into());
    }
    if matches!(y.index, 16 | 26) {
        blocked = true;
        cautions.push(format!("{} yoga", y.name()));
    }
    if matches!(ti.number % 15, 4 | 9 | 14) || ti.number == 30 {
        blocked = true;
        cautions.push(format!("{} tithi (rikta or amavasya)", ti.name()));
    }
    if let Some(nm) = natal_moon_sidereal {
        let janma = nakshatra(nm).index;
        let count = (n.index + 27 - janma) % 27 % 9 + 1; // 1..9 tara
        if matches!(count, 3 | 5 | 7) {
            blocked = true;
            cautions.push(format!(
                "Tara bala: {} ({})",
                [
                    "Janma",
                    "Sampat",
                    "Vipat",
                    "Kshema",
                    "Pratyak",
                    "Sadhana",
                    "Naidhana",
                    "Mitra",
                    "Parama Mitra"
                ][count - 1],
                n.name()
            ));
        }
        let from_natal =
            ((moon / 30.0).floor() as i64 - (nm / 30.0).floor() as i64).rem_euclid(12) + 1;
        if from_natal == 8 {
            blocked = true;
            cautions.push("Chandrashtama: Moon in the 8th sign from your natal Moon".into());
        } else if matches!(from_natal, 1 | 3 | 6 | 7 | 10 | 11) {
            score += 0.10;
            factors.push(format!(
                "Chandra bala: Moon in the {} from your natal Moon",
                ordinal(from_natal)
            ));
        }
    }

    // Positives
    if activity.nakshatras().contains(&n.index) {
        score += 0.35;
        factors.push(format!(
            "{} is a {} nakshatra",
            n.name(),
            activity.name().to_ascii_lowercase()
        ));
    } else {
        cautions.push(format!(
            "{} is not among the nakshatras named for {}",
            n.name(),
            activity.name().to_ascii_lowercase()
        ));
    }
    let tclass = match ti.number % 15 {
        1 | 6 | 11 => "Nanda",
        2 | 7 | 12 => "Bhadra",
        3 | 8 | 13 => "Jaya",
        5 | 10 | 0 => "Purna",
        _ => "Rikta",
    };
    if tclass != "Rikta" && ti.number != 30 {
        score += 0.15;
        factors.push(format!(
            "{} {} tithi ({tclass})",
            if ti.is_shukla() { "Shukla" } else { "Krishna" },
            ti.name()
        ));
    }
    if activity.good_days().contains(&wd) {
        score += 0.10;
        factors.push(format!(
            "{} suits the activity",
            [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday"
            ][wd as usize]
        ));
    } else {
        cautions.push(format!(
            "{} is not preferred for this",
            [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday"
            ][wd as usize]
        ));
    }
    if activity.good_horas().contains(&hora) {
        score += 0.15;
        factors.push(format!("{hora} hora"));
    }
    if !matches!(y.index, 0 | 5 | 8 | 9 | 12 | 14 | 16 | 18 | 26) {
        score += 0.05;
        factors.push(format!("{} yoga", y.name()));
    } else if !matches!(y.index, 16 | 26) {
        cautions.push(format!("{} yoga is avoided by some", y.name()));
    }
    let abhijit = in_window(t, abhijit_window(day));
    if abhijit {
        score += 0.15;
        factors.push("Abhijit muhurta".into());
    }
    if let Some(pref) = activity.lagna_preference() {
        if lagna_sign % 3 == pref {
            score += 0.05;
            factors.push(format!(
                "{} rising sign",
                if pref == 1 { "fixed" } else { "movable" }
            ));
        }
    }
    // lagna lord: in the sky of the moment, not in the 6th, 8th or 12th from the lagna
    let lord = sign_lord(lagna_sign as usize);
    let lord_body = match lord {
        DashaLord::Sun => Some(Body::Sun),
        DashaLord::Moon => Some(Body::Moon),
        DashaLord::Mars => Some(Body::MarsBarycenter),
        DashaLord::Mercury => Some(Body::Mercury),
        DashaLord::Jupiter => Some(Body::JupiterBarycenter),
        DashaLord::Venus => Some(Body::Venus),
        DashaLord::Saturn => Some(Body::SaturnBarycenter),
        _ => None,
    };
    if let Some(b) = lord_body {
        let l = sidereal(apparent_longitude(spk, b, t)?, ay);
        let house = ((l / 30.0).floor() as i64 - lagna_sign as i64).rem_euclid(12) + 1;
        if matches!(house, 6 | 8 | 12) {
            cautions.push(format!(
                "lagna lord {lord:?} is in the {house}th from the rising sign"
            ));
        } else {
            score += 0.05;
            factors.push(format!(
                "lagna lord {lord:?} is well placed ({})",
                ordinal(house)
            ));
        }
    }

    Ok(Step {
        t,
        score: if blocked { 0.0 } else { score.min(1.0) },
        lagna_sign,
        tithi: format!(
            "{} {}",
            if ti.is_shukla() { "Shukla" } else { "Krishna" },
            TITHI_NAMES[((ti.number - 1) % 15) as usize]
        ),
        nakshatra: format!("{} {}", NAKSHATRA_NAMES[n.index], n.pada),
        yoga: YOGA_NAMES[y.index].to_string(),
        hora: hora.to_string(),
        abhijit,
        factors,
        cautions,
    })
}

/// Find the best windows for `activity` in `days` days from `from`. Steps of five
/// minutes; windows are runs of steps scoring at least `threshold`; at most `limit`
/// windows are returned, best first.
#[allow(clippy::too_many_arguments)]
pub fn muhurta(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    activity: Activity,
    ayanamsa: Ayanamsa,
    natal_moon_sidereal: Option<f64>,
    from: Instant,
    days: i64,
    threshold: f64,
    limit: usize,
) -> Result<Vec<MuhurtaWindow>, SpkError> {
    let step = 5 * 60 * NS_PER_SEC;
    let to = from.add_ns(days as i128 * 86_400 * NS_PER_SEC);
    let mut windows: Vec<MuhurtaWindow> = Vec::new();
    let mut run: Vec<Step> = Vec::new();
    let mut prev_bad: Option<Instant> = None;
    let mut t = from;
    let mut day: Option<SolarDay> = None;

    // Is `t` a good step? Used to refine window edges to the second.
    let good_at = |t: Instant, day: &SolarDay| -> bool {
        score_step(
            spk,
            eop,
            obs,
            activity,
            ayanamsa,
            natal_moon_sidereal,
            day,
            t,
        )
        .map(|s| s.score >= threshold)
        .unwrap_or(false)
    };
    // Bisect between a good instant and a bad instant to one second.
    let refine = |good: Instant, bad: Instant, day: &SolarDay| -> Instant {
        let (mut g, mut b) = (good, bad);
        while (b.ns_since(g)).abs() > NS_PER_SEC {
            let m = g.add_ns(b.ns_since(g) / 2);
            if good_at(m, day) {
                g = m;
            } else {
                b = m;
            }
        }
        g
    };
    let flush = |run: &mut Vec<Step>,
                 windows: &mut Vec<MuhurtaWindow>,
                 prev_bad: Option<Instant>,
                 next_bad: Option<Instant>,
                 day: &SolarDay| {
        if run.is_empty() {
            return;
        }
        let best = run
            .iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let mean = run.iter().map(|s| s.score).sum::<f64>() / run.len() as f64;
        let start = match prev_bad {
            Some(pb) => refine(run[0].t, pb, day),
            None => run[0].t,
        };
        let last = run.last().unwrap().t;
        let end = match next_bad {
            Some(nb) => refine(last, nb, day),
            None => last.add_ns(step),
        };
        windows.push(MuhurtaWindow {
            start,
            end,
            score: mean,
            lagna_sign: best.lagna_sign,
            tithi: best.tithi.clone(),
            nakshatra: best.nakshatra.clone(),
            yoga: best.yoga.clone(),
            hora: best.hora.clone(),
            abhijit: run.iter().any(|s| s.abhijit),
            factors: best.factors.clone(),
            cautions: best.cautions.clone(),
        });
        run.clear();
    };
    while t < to {
        let need_day = match &day {
            Some(d) => !(d.sunrise <= t && t < d.next_sunrise),
            None => true,
        };
        if need_day {
            if let Some(d) = &day {
                flush(&mut run, &mut windows, prev_bad, None, d);
            }
            day = solar_day(spk, eop, obs, t).ok().flatten();
            prev_bad = None;
        }
        let Some(d) = &day else {
            t = t.add_ns(step);
            continue;
        };
        let s = score_step(spk, eop, obs, activity, ayanamsa, natal_moon_sidereal, d, t)?;
        if s.score >= threshold {
            run.push(s);
        } else {
            flush(&mut run, &mut windows, prev_bad, Some(t), d);
            prev_bad = Some(t);
        }
        t = t.add_ns(step);
    }
    if let Some(d) = &day {
        flush(&mut run, &mut windows, prev_bad, None, d);
    }
    windows.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.start.cmp(&b.start))
    });
    windows.truncate(limit);
    Ok(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activities_parse_and_name() {
        assert_eq!(Activity::parse("Wedding"), Some(Activity::Marriage));
        assert_eq!(
            Activity::parse("grihapravesh"),
            Some(Activity::Housewarming)
        );
        assert_eq!(Activity::parse("nothing"), None);
        assert_eq!(Activity::Travel.name(), "Travel");
    }

    #[test]
    fn windows_are_ordered_scored_and_explained() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let from = Utc::new(2026, 10, 15, 0, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let w = muhurta(
            &spk,
            None,
            &obs,
            Activity::Business,
            Ayanamsa::Lahiri,
            Some(243.2),
            from,
            7,
            0.5,
            10,
        )
        .unwrap();
        assert!(!w.is_empty());
        for x in &w {
            assert!(x.start < x.end);
            assert!(x.score >= 0.5 && x.score <= 1.0);
            assert!(!x.factors.is_empty());
        }
        for pair in w.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
        // a window never overlaps Rahu kala
        for x in &w {
            let d = solar_day(&spk, None, &obs, x.start).unwrap().unwrap();
            let (rs, re) = rahu_kala(&d, &obs);
            assert!(
                x.end <= rs || x.start >= re,
                "window {:?}–{:?} overlaps Rahu kala {:?}–{:?} (sunrise {:?})",
                Utc::from_instant(x.start),
                Utc::from_instant(x.end),
                Utc::from_instant(rs),
                Utc::from_instant(re),
                Utc::from_instant(d.sunrise)
            );
        }
    }
}
