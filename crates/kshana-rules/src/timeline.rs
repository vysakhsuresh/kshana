//! The book of a life: its chapters and pages (the Vimshottari mahadashas and the
//! antardashas of the running one), Saturn's trials (every sade sati and dhaiya from
//! birth, solved to the second), Jupiter's passages over the Moon's houses, and the
//! returns of Saturn, Jupiter and Rahu to their birth places. Past, present and future
//! on one line, each with a tone per life from the birth chart.

use crate::forecast::{date_words, lord_score, Band, Life, Natal};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::transit::{kernel_end, kernel_start, returns, sidereal_longitude, sign_changes};
use kshana_ephem::Spk;
use kshana_time::{Instant, NS_PER_DAY};
use kshana_vedic::dasha::{subdivide, vimshottari, YEAR_365_25_NS};
use kshana_vedic::{Ayanamsa, DashaLord};

/// One chapter (mahadasha) or page (antardasha).
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    /// Lord.
    pub lord: DashaLord,
    /// 1 = mahadasha, 2 = antardasha.
    pub level: u8,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Age at the start, years.
    pub age: f64,
    /// Overall tone, `[0, 1]`.
    pub tone: f64,
    /// Band of the tone.
    pub band: Band,
    /// Tone per life, `Life::ALL` order.
    pub lives: Vec<f64>,
    /// The life it serves best.
    pub best: Life,
    /// The life it tests most.
    pub weak: Life,
    /// Plain words.
    pub words: String,
    /// True when `at` falls inside it.
    pub current: bool,
}

/// A span of Saturn over the Moon's testing houses.
#[derive(Clone, Debug, PartialEq)]
pub struct SaturnSpan {
    /// `sade sati`, `ardhashtama` or `ashtama Shani`.
    pub kind: &'static str,
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Age at the start.
    pub age: f64,
    /// Every phase change inside it: `(instant, words)`.
    pub phases: Vec<(Instant, String)>,
    /// True when `at` falls inside it.
    pub current: bool,
    /// Plain words with dates.
    pub words: String,
}

/// Jupiter in one house from the Moon.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Sign.
    pub sign: u8,
    /// House from the natal Moon.
    pub house: u8,
    /// Favourable by the transit tables.
    pub favourable: bool,
    /// True when `at` falls inside it.
    pub current: bool,
    /// Plain words.
    pub words: String,
}

/// A graha back at its birth place.
#[derive(Clone, Debug, PartialEq)]
pub struct Return {
    /// Which graha.
    pub graha: DashaLord,
    /// First, second, third return.
    pub nth: u8,
    /// When.
    pub at: Instant,
    /// Age then.
    pub age: f64,
    /// Plain words.
    pub words: String,
}

/// The book.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    /// Birth.
    pub birth: Instant,
    /// The instant the book is opened at.
    pub at: Instant,
    /// Age now, years.
    pub age: f64,
    /// The mahadashas of the 120-year cycle.
    pub chapters: Vec<Chapter>,
    /// The antardashas of the running mahadasha.
    pub pages: Vec<Chapter>,
    /// Saturn's trials from birth to age 100.
    pub saturn: Vec<SaturnSpan>,
    /// Jupiter's passages, three years back to twelve ahead.
    pub jupiter: Vec<Passage>,
    /// Returns of Saturn, Jupiter and Rahu, chronological.
    pub returns: Vec<Return>,
    /// One or two sentences about now.
    pub headline: String,
    /// The last instant the ephemeris on this device can see; Saturn's spans and the
    /// returns stop here.
    pub horizon: Instant,
}

fn years(from: Instant, to: Instant) -> f64 {
    to.ns_since(from) as f64 / (365.25 * NS_PER_DAY as f64)
}

/// What a phase of a long test brings, as the line in the book.
fn phase_words(ph: SaturnPhase) -> &'static str {
    match ph {
        SaturnPhase::SadeSatiRising => "The first part begins: expenses rise and sleep gets lighter",
        SaturnPhase::SadeSatiPeak => "The hardest part begins: the mind feels heavy and work doubles",
        SaturnPhase::SadeSatiSetting => "The last part begins: the pressure starts to ease",
        SaturnPhase::DhaiyaFourth => "It begins: home and peace of mind are tested",
        SaturnPhase::DhaiyaEighth => "It begins: health and sudden changes need care",
        SaturnPhase::Clear => "The pressure ends",
    }
}

#[allow(dead_code)]
pub(crate) fn lord_theme(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "years of standing and authority: the father, the state, the self",
        DashaLord::Moon => "years of home and mind: mother, feeling, the public",
        DashaLord::Mars => "years of effort, property and courage; keep the temper",
        DashaLord::Mercury => "years of learning, trade and talk",
        DashaLord::Jupiter => "years of growth: teachers, children, faith and money",
        DashaLord::Venus => "years of love, comfort, art and partnership",
        DashaLord::Saturn => "years of work and duty; slow, lasting gains",
        DashaLord::Rahu => "years of ambition, foreign things and sudden turns",
        DashaLord::Ketu => "years of letting go, insight and the inner road",
    }
}

fn chapter(natal: &Natal, lord: DashaLord, parent: Option<DashaLord>, level: u8, start: Instant, end: Instant, at: Instant) -> Chapter {
    let lives: Vec<f64> = Life::ALL
        .iter()
        .map(|&life| {
            let (s, _) = lord_score(natal, life, lord);
            match parent {
                Some(p) => 0.5 * s + 0.5 * lord_score(natal, life, p).0,
                None => s,
            }
        })
        .collect();
    let tone = lives.iter().sum::<f64>() / lives.len() as f64;
    let (bi, _) = lives.iter().enumerate().fold((0, f64::MIN), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    let (wi, _) = lives.iter().enumerate().fold((0, f64::MAX), |a, (i, &v)| if v < a.1 { (i, v) } else { a });
    let best = Life::ALL[bi];
    let weak = Life::ALL[wi];
    let band = Band::of(tone);
    let words = format!(
        "{} {}. {} for {}; {} asks patience.",
        if level == 1 { "A chapter about" } else { "A stretch about" },
        crate::plain::chapter_theme(lord),
        match band {
            Band::Bright => "Giving",
            Band::Good => "Good",
            Band::Steady => "Even",
            Band::Slow => "Slow",
            Band::Hold => "Patient",
        },
        best.name().to_ascii_lowercase(),
        crate::plain::subject(&weak.name().to_ascii_lowercase())
    );
    Chapter {
        lord,
        level,
        start,
        end,
        age: years(natal.birth, start),
        tone,
        band,
        lives,
        best,
        weak,
        words,
        current: start <= at && at < end,
    }
}

/// Open the book at `at`. `longitude_deg` only places the dates in words.
pub fn timeline(spk: &Spk, natal: &Natal, ayanamsa: Ayanamsa, longitude_deg: f64, at: Instant) -> Timeline {
    let birth = natal.birth;
    let mahas = vimshottari(natal.moon_sidereal, birth, YEAR_365_25_NS);
    let chapters: Vec<Chapter> = mahas
        .iter()
        .map(|p| chapter(natal, p.lord, None, 1, p.start.max(birth), p.end, at))
        .collect();
    let running = mahas.iter().copied().find(|p| p.start <= at && at < p.end);
    let pages: Vec<Chapter> = running
        .map(|p| {
            subdivide(p)
                .into_iter()
                .map(|q| chapter(natal, q.lord, Some(p.lord), 2, q.start.max(birth), q.end, at))
                .collect()
        })
        .unwrap_or_default();

    // Saturn from birth to age 100, or as far as the kernel on this device sees.
    let horizon = kernel_end(spk);
    let end100 = birth
        .add_ns((100.0 * 365.25 * NS_PER_DAY as f64) as i128)
        .min(horizon);
    let sat_changes = sign_changes(spk, ayanamsa, DashaLord::Saturn, birth, end100);
    let sat_sign0 = sidereal_longitude(spk, ayanamsa, DashaLord::Saturn, birth)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let ms = natal.moon_sign();
    let house_of = |sign: u8| ((sign as i64 - ms).rem_euclid(12) + 1) as u8;
    // Intervals of Saturn's sign: (start, end, sign).
    let mut intervals: Vec<(Instant, Instant, u8)> = Vec::new();
    let mut t = birth;
    let mut s = sat_sign0;
    for c in &sat_changes {
        intervals.push((t, c.at, s));
        t = c.at;
        s = c.to;
    }
    intervals.push((t, end100, s));
    let mut saturn: Vec<SaturnSpan> = Vec::new();
    let kind_of = |h: u8| -> Option<&'static str> {
        match h {
            12 | 1 | 2 => Some("sade sati"),
            4 => Some("ardhashtama"),
            8 => Some("ashtama Shani"),
            _ => None,
        }
    };
    let mut i = 0;
    while i < intervals.len() {
        let (st, _, sg) = intervals[i];
        let h = house_of(sg);
        if let Some(kind) = kind_of(h) {
            // extend while the kind holds (retrograde re-entries included)
            let mut j = i;
            let mut phases: Vec<(Instant, String)> = Vec::new();
            while j < intervals.len() && kind_of(house_of(intervals[j].2)) == Some(kind) {
                let hh = house_of(intervals[j].2);
                let ph = SaturnPhase::from_house(hh);
                phases.push((
                    intervals[j].0,
                    phase_words(ph).to_string(),
                ));
                j += 1;
            }
            let en = intervals[j - 1].1;
            let current = st <= at && at < en;
            let words = format!(
                "{} from {} to {} (age {:.0} to {:.0}){}",
                match kind {
                    "sade sati" => "A long testing period of about seven and a half years,",
                    "ardhashtama" => "A testing time for home and peace of mind,",
                    _ => "A testing time for health and sudden changes,",
                },
                date_words(st, longitude_deg),
                date_words(en, longitude_deg),
                years(birth, st),
                years(birth, en),
                if current {
                    format!(". You are in it now; it ends {}.", date_words(en, longitude_deg))
                } else if en <= at {
                    ". Passed.".to_string()
                } else {
                    ". Ahead.".to_string()
                }
            );
            saturn.push(SaturnSpan {
                kind,
                start: st,
                end: en,
                age: years(birth, st),
                phases,
                current,
                words,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    // A retrograde loop can carry Saturn back out of the span for a few months and in
    // again; that is one trial, not two. Merge same-kind spans separated by under 1.5 years.
    let mut merged: Vec<SaturnSpan> = Vec::new();
    for s in saturn {
        if let Some(last) = merged.last_mut() {
            if last.kind == s.kind && years(last.end, s.start) < 1.5 {
                let months = (years(last.end, s.start) * 12.0).round() as i64;
                last.phases.push((
                    last.end,
                    format!("The pressure lifts for about {months} month{}, then returns", if months == 1 { "" } else { "s" }),
                ));
                last.phases.extend(
                    s.phases
                        .into_iter()
                        .map(|(t, w)| (t, w.replacen(" begins", " resumes", 1))),
                );
                last.end = s.end;
                last.current = last.start <= at && at < last.end;
                last.words = format!(
                    "{} from {} to {} (age {:.0} to {:.0}){}",
                    match last.kind {
                        "sade sati" => "A long testing period of about seven and a half years,",
                        "ardhashtama" => "A testing time for home and peace of mind,",
                        _ => "A testing time for health and sudden changes,",
                    },
                    date_words(last.start, longitude_deg),
                    date_words(last.end, longitude_deg),
                    years(birth, last.start),
                    years(birth, last.end),
                    if last.current {
                        format!(". You are in it now; it ends {}.", date_words(last.end, longitude_deg))
                    } else if last.end <= at {
                        ". Passed.".to_string()
                    } else {
                        ". Ahead.".to_string()
                    }
                );
                continue;
            }
        }
        merged.push(s);
    }
    let saturn = merged;

    // Jupiter: three years back to twelve ahead.
    let jfrom = at.add_ns(-3 * 365 * NS_PER_DAY).max(kernel_start(spk));
    let jto = at.add_ns(12 * 365 * NS_PER_DAY).min(horizon);
    let jch = sign_changes(spk, ayanamsa, DashaLord::Jupiter, jfrom, jto);
    let jsign0 = sidereal_longitude(spk, ayanamsa, DashaLord::Jupiter, jfrom)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let mut jupiter = Vec::new();
    let mut t = jfrom;
    let mut s = jsign0;
    let mut push = |start: Instant, end: Instant, sign: u8| {
        let h = house_of(sign);
        let fav = favourable_from_moon(DashaLord::Jupiter).contains(&h);
        jupiter.push(Passage {
            start,
            end,
            sign,
            house: h,
            favourable: fav,
            current: start <= at && at < end,
            words: if fav {
                "A giving year: growth, help and good news come easier".to_string()
            } else {
                match h {
                    1 => "A year of expense and focus on yourself: spend with care",
                    4 | 8 | 12 => "A quiet year: build inside, not outside",
                    _ => "An even year for growth",
                }
                .to_string()
            },
        });
    };
    for c in &jch {
        push(t, c.at, s);
        t = c.at;
        s = c.to;
    }
    push(t, jto, s);

    // Returns.
    let mut rets: Vec<Return> = Vec::new();
    for g in [DashaLord::Saturn, DashaLord::Jupiter, DashaLord::Rahu] {
        let Some(natal_lon) = sidereal_longitude(spk, ayanamsa, g, birth) else { continue };
        // A graha re-crossing its natal degree in the months after birth is a loop, not a return.
        let v = returns(spk, ayanamsa, g, natal_lon, birth.add_ns(2 * 365 * NS_PER_DAY), end100);
        // A retrograde loop gives three crossings within months; count them as one return.
        let mut n = 0u8;
        let mut last: Option<Instant> = None;
        for t in v {
            if let Some(l) = last {
                if years(l, t) < 1.5 {
                    continue;
                }
            }
            n += 1;
            last = Some(t);
            let age = years(birth, t);
            rets.push(Return {
                graha: g,
                nth: n,
                at: t,
                age,
                words: format!(
                    "{} {}, {} (age {:.1}): {}",
                    match n {
                        1 => "First",
                        2 => "Second",
                        3 => "Third",
                        _ => "Fourth",
                    },
                    match g {
                        DashaLord::Saturn => "thirty-year reckoning",
                        DashaLord::Jupiter => "twelve-year renewal",
                        _ => "eighteen-year turn",
                    },
                    date_words(t, longitude_deg),
                    age,
                    match g {
                        DashaLord::Saturn => "the structure of your life is tested and rebuilt; what is true stays",
                        DashaLord::Jupiter => "a year of expansion, teachers and second chances",
                        _ => "an old hunger comes round, and a chance to answer it",
                    }
                ),
            });
        }
    }
    rets.sort_by(|a, b| a.at.cmp(&b.at));

    let age = years(birth, at);
    let cur = chapters.iter().find(|c| c.current);
    let page = pages.iter().find(|c| c.current);
    let sat_now = saturn.iter().find(|s| s.current);
    let next_ret = rets.iter().find(|r| r.at > at);
    let mut headline = match (cur, page) {
        (Some(c), Some(p)) => format!(
            "You are {:.1}. Until {} your life centres on {}. Until {} the most movement is in {}.",
            age,
            date_words(c.end, longitude_deg),
            crate::plain::chapter_theme(c.lord),
            date_words(p.end, longitude_deg),
            crate::plain::chapter_theme(p.lord)
        ),
        (Some(c), None) => format!("You are {:.1}. Until {} your life centres on {}.", age, date_words(c.end, longitude_deg), crate::plain::chapter_theme(c.lord)),
        _ => format!("You are {age:.1}."),
    };
    if let Some(s) = sat_now {
        headline.push(' ');
        headline.push_str(&s.words);
    }
    if let Some(r) = next_ret {
        headline.push_str(&format!(" Next: {}.", r.words.split(':').next().unwrap_or("")));
    }

    Timeline {
        birth,
        at,
        age,
        chapters,
        pages,
        saturn,
        jupiter,
        returns: rets,
        headline,
        horizon,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_book_has_chapters_trials_passages_and_returns() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let t0 = std::time::Instant::now();
        let tl = timeline(&spk, &natal, Ayanamsa::Lahiri, 76.2673, at);
        let dt = t0.elapsed();
        assert_eq!(tl.chapters.len(), 9);
        assert_eq!(tl.chapters.iter().filter(|c| c.current).count(), 1);
        assert_eq!(tl.pages.len(), 9);
        assert_eq!(tl.pages.iter().filter(|c| c.current).count(), 1);
        // Saturn's 29.5-year cycle gives three or four sade satis in a hundred years.
        let ss: Vec<&SaturnSpan> = tl.saturn.iter().filter(|s| s.kind == "sade sati").collect();
        assert!(ss.len() >= 3 && ss.len() <= 4, "{}", ss.len());
        let end100 = birth.add_ns((100.0 * 365.25 * NS_PER_DAY as f64) as i128);
        let whole = |s: &SaturnSpan| s.start > birth.add_ns(NS_PER_DAY) && s.end < end100.add_ns(-NS_PER_DAY);
        for s in ss.iter().filter(|s| whole(s)) {
            let len = years(s.start, s.end);
            assert!(len > 6.0 && len < 9.5, "sade sati {len} years: {}", s.words);
            assert!(s.phases.len() >= 3);
        }
        // and the ardhashtama and ashtama spans, each about 2.5 years
        for s in tl.saturn.iter().filter(|s| s.kind != "sade sati" && whole(s)) {
            let len = years(s.start, s.end);
            assert!(len > 1.5 && len < 4.0, "{} {len}", s.kind);
        }
        for s in &tl.saturn {
            eprintln!("{}", s.words);
        }
        assert!(tl.saturn.windows(2).all(|w| w[0].end <= w[1].start));
        assert!(tl.jupiter.len() >= 12 && tl.jupiter.len() <= 48, "{}", tl.jupiter.len());
        assert_eq!(tl.jupiter.iter().filter(|p| p.current).count(), 1);
        // The horizon is the kernel end (DE421 in tests: 2053; the app ships DE440s: 2150):
        // to 2053, two Saturn returns, five Jupiter, three Rahu for a 1990 birth.
        let seen = years(birth, tl.horizon);
        assert!(seen > 60.0, "{seen}");
        assert!(tl.returns.iter().all(|r| r.at <= tl.horizon));
        let sat_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Saturn).collect();
        assert_eq!(sat_ret.len(), (seen / 29.46) as usize, "{:?}", sat_ret);
        assert!((sat_ret[0].age - 29.5).abs() < 1.0, "{}", sat_ret[0].age);
        let jup_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Jupiter).collect();
        assert_eq!(jup_ret.len(), (seen / 11.86) as usize, "{:?}", jup_ret.iter().map(|r| r.age).collect::<Vec<_>>());
        assert!((jup_ret[0].age - 11.86).abs() < 1.0, "{}", jup_ret[0].age);
        let rahu_ret: Vec<&Return> = tl.returns.iter().filter(|r| r.graha == DashaLord::Rahu).collect();
        assert_eq!(rahu_ret.len(), (seen / 18.61) as usize, "{:?}", rahu_ret.iter().map(|r| r.age).collect::<Vec<_>>());
        assert!((rahu_ret[0].age - 18.6).abs() < 0.8, "{}", rahu_ret[0].age);
        assert!(!tl.headline.is_empty());
        eprintln!("timeline in {dt:?}");
    }
}
