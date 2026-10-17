//! The outlook in plain words: today, this week, this month, this year, for each life
//! and for the whole. Every sentence here is about the person's life (what goes well,
//! what asks patience, which day, what to do) and never names a planet. The astrology
//! that produced it travels alongside as `why`, for the tap that asks.

use crate::explain::explain;
use crate::feed::next_good;
use crate::forecast::{date_words, moment, series, slow_series, story, Band, ForecastError, Life, Natal, Story};
use crate::gochara::SaturnPhase;
use crate::varsha::year_chapter;
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

/// How far the outlook looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Span {
    Day,
    Week,
    Month,
    Year,
}

impl Span {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Span::Day => "day",
            Span::Week => "week",
            Span::Month => "month",
            Span::Year => "year",
        }
    }

    /// Parse a key.
    pub fn parse(s: &str) -> Option<Span> {
        [Span::Day, Span::Week, Span::Month, Span::Year].into_iter().find(|x| x.key() == s)
    }

    fn noun(self) -> &'static str {
        match self {
            Span::Day => "day",
            Span::Week => "week",
            Span::Month => "month",
            Span::Year => "year",
        }
    }
}

/// One life's outlook over the span.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeOutlook {
    /// Which life.
    pub life: Life,
    /// Mean score over the span, `[0, 1]`.
    pub score: f64,
    /// Band of the mean.
    pub band: Band,
    /// `rising`, `falling` or `steady`.
    pub trend: &'static str,
    /// What the span holds, in plain words with dates. No planets.
    pub words: String,
    /// What to do about it, one sentence.
    pub advice: String,
    /// The best day (or hour, for a day), when there is one.
    pub best_at: Option<Instant>,
    /// Its score.
    pub best_score: f64,
    /// The day (or hour) to go easy.
    pub care_at: Option<Instant>,
    /// Its score.
    pub care_score: f64,
    /// The astrology behind it, for the tap that asks.
    pub why: Vec<String>,
}

/// The whole outlook.
#[derive(Clone, Debug, PartialEq)]
pub struct Outlook {
    /// The span.
    pub span: Span,
    /// First instant.
    pub from: Instant,
    /// Last instant.
    pub to: Instant,
    /// Mean over all lives, `[0, 1]`.
    pub score: f64,
    /// Band.
    pub band: Band,
    /// Two or three sentences about the span as a whole. No planets.
    pub headline: String,
    /// Nine lives, strongest first.
    pub lives: Vec<LifeOutlook>,
    /// The season in plain words: the long stretch the span sits in, with its end date.
    pub season: String,
    /// The astrology behind the season.
    pub season_why: String,
}

fn band_words(life: Life, band: Band, span: Span) -> &'static str {
    use Band::*;
    use Life::*;
    match (life, band) {
        (Love, Bright) => "Love is lit up: closeness comes easily and what you say lands well.",
        (Love, Good) => "Love goes well: a warm, open stretch for the one who matters.",
        (Love, Steady) => "Love is even: nothing pushes, nothing pulls.",
        (Love, Slow) => "Love is slow: feelings run close to the surface and need room.",
        (Love, Hold) => "Love is on hold: settle nothing of the heart in this stretch.",
        (Career, Bright) => "Work moves your way: ask, pitch, apply, and be seen.",
        (Career, Good) => "Work is productive: finish what earns notice.",
        (Career, Steady) => "Work is steady: clear the list, make no big bets.",
        (Career, Slow) => "Work is slow: effort outweighs reward for now.",
        (Career, Hold) => "Work is on hold: do not sign, resign or confront.",
        (Money, Bright) => "Money flows in: a fine stretch to receive and to invest with care.",
        (Money, Good) => "Money is good: accounts balance and what is owed comes.",
        (Money, Steady) => "Money is level: spend as planned, no more.",
        (Money, Slow) => "Money is tight: expenses rise before income does.",
        (Money, Hold) => "Money is on hold: no loans, no lending, no big purchases.",
        (Health, Bright) => "Energy is high: body and mind are in step.",
        (Health, Good) => "Health is good: keep meals and sleep regular and it stays so.",
        (Health, Steady) => "Health is steady: do not overdo it.",
        (Health, Slow) => "Energy dips: rest early and eat light.",
        (Health, Hold) => "Health asks care: avoid risk, strain and late nights.",
        (Family, Bright) => "Home shines: a stretch to gather and to mend.",
        (Family, Good) => "Home is good: elders and children both respond well.",
        (Family, Steady) => "Home is calm: keep the routines.",
        (Family, Slow) => "Home needs patience: tempers run short.",
        (Family, Hold) => "Home asks for peace: decide nothing lasting there.",
        (Study, Bright) => "The mind is sharp: learn, write, sit the test.",
        (Study, Good) => "Study goes well: a clear-headed stretch.",
        (Study, Steady) => "Study is steady: practice rather than new topics.",
        (Study, Slow) => "Concentration wanders: short sessions, more breaks.",
        (Study, Hold) => "Study is on hold: cramming will not stick.",
        (Home, Bright) => "House and property move well: sign, build, shift.",
        (Home, Good) => "House matters progress: measure, plan, fix.",
        (Home, Steady) => "House matters are stable: maintain, do not expand.",
        (Home, Slow) => "House matters drag: small delays with land, rent or repairs.",
        (Home, Hold) => "House matters are on hold: no deals about property.",
        (Travel, Bright) => "Roads open: travel and plans abroad go smoothly.",
        (Travel, Good) => "Travel is good: short trips and paperwork go through.",
        (Travel, Steady) => "Travel is plain: allow the usual margin.",
        (Travel, Slow) => "Travel is slow: delays on the way, keep plans flexible.",
        (Travel, Hold) => "Travel is on hold: do not start a long journey.",
        (Children, Bright) => "Joy from children: their milestones are blessed.",
        (Children, Good) => "Children do well: teach, play, listen.",
        (Children, Steady) => "An even stretch for the children.",
        (Children, Slow) => "Children need extra patience.",
        (Children, Hold) => "Be gentle and watchful with the children; decide nothing big about them.",
    }
    .trim_end_matches(|c: char| c == ' ')
    .pipe(|s| {
        let _ = span;
        s
    })
}

trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

fn advice_words(life: Life, band: Band) -> &'static str {
    use Band::*;
    use Life::*;
    match (life, band) {
        (Love, Bright | Good) => "Say the thing you have been holding; reach out first.",
        (Love, Steady) => "Keep it simple and kind.",
        (Love, Slow | Hold) => "Listen more than you speak, and hold the big conversations for the better days below.",
        (Career, Bright | Good) => "Put your name forward; this is when authority listens.",
        (Career, Steady) => "Keep your head down; the gains are quiet ones.",
        (Career, Slow | Hold) => "Double-check before you send, and pace yourself.",
        (Money, Bright | Good) => "Collect what is owed and close the deal.",
        (Money, Steady) => "Keep the budget.",
        (Money, Slow | Hold) => "Watch the wallet: no impulse buys, no new debt.",
        (Health, Bright | Good) => "Start the healthy habit now; it will hold.",
        (Health, Steady) => "Keep the routine.",
        (Health, Slow | Hold) => "Sleep early, eat light, mind the small aches.",
        (Family, Bright | Good) => "Make the call; gather the people.",
        (Family, Steady) => "Let it stay calm.",
        (Family, Slow | Hold) => "Do not take the bait; patience over answers.",
        (Study, Bright | Good) => "Take the hard chapter now.",
        (Study, Steady) => "Practice over new material.",
        (Study, Slow | Hold) => "Re-read before you submit; short sessions.",
        (Home, Bright | Good) => "Move on the house matter you have been putting off.",
        (Home, Steady) => "Maintain; do not expand.",
        (Home, Slow | Hold) => "Check every document twice; wait for the better days below.",
        (Travel, Bright | Good) => "Book it; go.",
        (Travel, Steady) => "Leave a little early.",
        (Travel, Slow | Hold) => "Keep plans flexible and carry patience.",
        (Children, Bright | Good) => "Give them the time; it pays now.",
        (Children, Steady) => "Keep the routine.",
        (Children, Slow | Hold) => "Stay close and go gently.",
    }
}

fn day_name_words(t: Instant, lon: f64, span: Span) -> String {
    match span {
        Span::Day => String::new(),
        _ => date_words(t, lon),
    }
}

/// Saturn's phase in plain words, without the planet.
pub fn saturn_plain(phase: SaturnPhase) -> &'static str {
    match phase {
        SaturnPhase::SadeSatiRising => "the long seven-and-a-half-year test (sade sati) has begun: work doubles, rewards come late",
        SaturnPhase::SadeSatiPeak => "the long seven-and-a-half-year test (sade sati) is at its peak: the mind is heavy and the ground shifts",
        SaturnPhase::SadeSatiSetting => "the last part of the long test (sade sati): what is true stays, the rest falls away",
        SaturnPhase::DhaiyaFourth => "a testing stretch for home and peace of mind (ardhashtama)",
        SaturnPhase::DhaiyaEighth => "a testing stretch for health and sudden turns (ashtama Shani)",
        SaturnPhase::Clear => "no long test running",
    }
}

/// The outlook for `span` from `at`.
#[allow(clippy::too_many_arguments)]
pub fn outlook(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    span: Span,
    at: Instant,
) -> Result<Outlook, ForecastError> {
    let lon = obs.longitude_deg;
    let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let season_why = m.saturn.words().to_string();
    let levels = &m.dasha;
    let season = {
        let antar_end = levels.get(1).map(|p| date_words(p.end, lon));
        let sat = saturn_plain(m.saturn);
        match (m.saturn, antar_end) {
            (SaturnPhase::Clear, Some(e)) => format!("The season you are in runs to {e}; no long test is on you."),
            (_, Some(e)) => format!("The season you are in runs to {e}, and {sat}."),
            (SaturnPhase::Clear, None) => "No long test is on you.".to_string(),
            (_, None) => format!("Right now {sat}."),
        }
    };

    let (from, to, lives): (Instant, Instant, Vec<LifeOutlook>) = match span {
        Span::Day => {
            let good = next_good(&m, at, 0.56);
            let lives = Life::ALL
                .iter()
                .enumerate()
                .map(|(i, &life)| {
                    let ls = &m.lives[i];
                    let g = good[i].as_ref();
                    let caution = m.windows.iter().find(|w| w.caution && w.start > at).map(|w| w.start);
                    let mut words = format!("{} {}", band_words(life, ls.band, span), "");
                    match g {
                        Some(g) if g.now => words.push_str(&format!("This hour serves it, until {}.", crate::forecast::date_words(g.end, lon).pipe(|_| hm(g.end, lon)))),
                        Some(g) => words.push_str(&format!("Your best hour for it comes at {}.", hm(g.start, lon))),
                        None => words.push_str("No strong hour for it is left today; tomorrow's sunrise starts a new set."),
                    }
                    LifeOutlook {
                        life,
                        score: ls.score,
                        band: ls.band,
                        trend: "steady",
                        words: words.trim().to_string(),
                        advice: advice_words(life, ls.band).to_string(),
                        best_at: g.map(|x| x.start),
                        best_score: g.map(|x| x.fit).unwrap_or(0.0),
                        care_at: caution,
                        care_score: 0.0,
                        why: ls.components.iter().map(|c| c.note.clone()).collect(),
                    }
                })
                .collect();
            (at, m.sunrise.map(|s| s.add_ns(NS_PER_DAY)).unwrap_or(at.add_ns(NS_PER_DAY)), lives)
        }
        Span::Week | Span::Month => {
            let n = if span == Span::Week { 7 } else { 30 };
            let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, n)?;
            let lives = Life::ALL
                .iter()
                .map(|&life| {
                    let st: Story = story(life, &samples);
                    let band = Band::of(st.mean);
                    let mut words = band_words(life, band, span).to_string();
                    match st.trend {
                        "rising" => words.push_str(&format!(" It improves as the {} goes on.", span.noun())),
                        "falling" => words.push_str(" It starts stronger than it ends; act early."),
                        _ => {}
                    }
                    if let Some((a, b, v)) = st.best {
                        if v >= 0.56 {
                            words.push_str(&format!(" Best: {}{}.", day_name_words(a, lon, span), if b.ns_since(a) > NS_PER_DAY { format!(" to {}", date_words(b, lon)) } else { String::new() }));
                        }
                    }
                    if let Some((a, _, v)) = st.low {
                        if v < 0.44 {
                            words.push_str(&format!(" Go easy around {}.", date_words(a, lon)));
                        }
                    }
                    let why: Vec<String> = st.shifts.iter().map(|s| format!("{}: {}", date_words(s.at, lon), s.what)).collect();
                    LifeOutlook {
                        life,
                        score: st.mean,
                        band,
                        trend: st.trend,
                        words,
                        advice: advice_words(life, band).to_string(),
                        best_at: st.best.map(|b| b.0),
                        best_score: st.best.map(|b| b.2).unwrap_or(0.0),
                        care_at: st.low.map(|b| b.0),
                        care_score: st.low.map(|b| b.2).unwrap_or(0.0),
                        why,
                    }
                })
                .collect();
            (at, at.add_ns(n as i128 * NS_PER_DAY), lives)
        }
        Span::Year => {
            let yc = year_chapter(spk, eop, natal, obs, system, ayanamsa, at).ok();
            let month = (30.4375 * NS_PER_DAY as f64) as i128;
            let slow = slow_series(spk, eop, natal, obs, ayanamsa, at, month, 13)?;
            // The two lives that most need it get the exact turning dates from Explain.
            let mut means: Vec<(usize, f64)> = (0..9).map(|i| (i, slow.iter().map(|(_, v)| v[i]).sum::<f64>() / slow.len() as f64)).collect();
            means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let weak_two: Vec<usize> = means.iter().take(2).map(|x| x.0).collect();
            let lives = Life::ALL
                .iter()
                .enumerate()
                .map(|(i, &life)| {
                    let vals: Vec<(Instant, f64)> = slow.iter().map(|(t, v)| (*t, v[i])).collect();
                    let mean = vals.iter().map(|x| x.1).sum::<f64>() / vals.len() as f64;
                    let band = Band::of(mean);
                    let head = vals[..4].iter().map(|x| x.1).sum::<f64>() / 4.0;
                    let tail = vals[9..].iter().map(|x| x.1).sum::<f64>() / 4.0;
                    let trend = if tail - head > 0.05 { "rising" } else if head - tail > 0.05 { "falling" } else { "steady" };
                    let best = vals.iter().cloned().fold(None, |a: Option<(Instant, f64)>, x| if a.map_or(true, |a| x.1 > a.1) { Some(x) } else { a });
                    let low = vals.iter().cloned().fold(None, |a: Option<(Instant, f64)>, x| if a.map_or(true, |a| x.1 < a.1) { Some(x) } else { a });
                    let mut words = band_words(life, band, span).to_string();
                    match trend {
                        "rising" => words.push_str(" The year improves as it goes."),
                        "falling" => words.push_str(" The first half is the stronger; move early."),
                        _ => {}
                    }
                    if let (Some(b), Some(l)) = (best, low) {
                        words.push_str(&format!(" Best month around {}; go easy around {}.", date_words(b.0, lon), date_words(l.0, lon)));
                    }
                    let mut why = Vec::new();
                    if weak_two.contains(&i) && band <= Band::Steady {
                        if let Ok(e) = explain(spk, eop, natal, obs, system, ayanamsa, life, at) {
                            if let Some(t) = &e.eases {
                                words.push_str(&format!(" It eases from {}.", date_words(t.at, lon)));
                            }
                            for c in e.causes.iter().take(2) {
                                why.push(format!("{}{}", c.what, if c.until_words.is_empty() { String::new() } else { format!(", {}", c.until_words) }));
                            }
                            for r in e.relief.iter().filter(|r| r.lasting).take(3) {
                                why.push(format!("{}: {} ({:+.0})", date_words(r.at, lon), r.what, r.gain * 100.0));
                            }
                        }
                    }
                    LifeOutlook {
                        life,
                        score: mean,
                        band,
                        trend,
                        words,
                        advice: advice_words(life, band).to_string(),
                        best_at: best.map(|b| b.0),
                        best_score: best.map(|b| b.1).unwrap_or(0.0),
                        care_at: low.map(|b| b.0),
                        care_score: low.map(|b| b.1).unwrap_or(0.0),
                        why,
                    }
                })
                .collect();
            let (f, t) = yc.as_ref().map(|y| (y.start, y.end)).unwrap_or((at, at.add_ns(365 * NS_PER_DAY)));
            (f, t, lives)
        }
    };

    let mut lives = lives;
    lives.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let score = lives.iter().map(|l| l.score).sum::<f64>() / lives.len() as f64;
    let band = Band::of(score);
    let top = &lives[0];
    let low = &lives[lives.len() - 1];
    let noun = span.noun();
    let lead = match band {
        Band::Bright => format!("A bright {noun}."),
        Band::Good => format!("A good {noun}."),
        Band::Steady => format!("An even {noun}."),
        Band::Slow => format!("A slow {noun}."),
        Band::Hold => format!("A {noun} to hold still."),
    };
    let mut headline = format!(
        "{lead} {} carries it{}; {} asks the most patience{}.",
        top.life.name(),
        top.best_at
            .filter(|_| span != Span::Day)
            .map(|t| format!(", best around {}", date_words(t, lon)))
            .unwrap_or_default(),
        low.life.name().to_ascii_lowercase(),
        low.care_at
            .filter(|_| span != Span::Day)
            .map(|t| format!(", go easy around {}", date_words(t, lon)))
            .unwrap_or_default()
    );
    let rising: Vec<&str> = lives.iter().filter(|l| l.trend == "rising").map(|l| l.life.name()).collect();
    if !rising.is_empty() && span != Span::Day {
        headline.push_str(&format!(" Improving as the {noun} goes: {}.", rising.join(", ").to_ascii_lowercase()));
    }
    if span == Span::Day {
        if let Some(g) = next_good(&m, at, 0.56).into_iter().flatten().filter(|g| !g.now).min_by_key(|g| g.start.tai_ns()) {
            headline.push_str(&format!(" The next good hour comes at {} for {}.", hm(g.start, lon), g.life.name().to_ascii_lowercase()));
        }
    }

    Ok(Outlook {
        span,
        from,
        to,
        score,
        band,
        headline,
        lives,
        season,
        season_why,
    })
}

/// "16:37" at the place's mean time.
fn hm(t: Instant, lon: f64) -> String {
    let local = t.add_ns((lon / 15.0 * 3_600.0 * kshana_time::NS_PER_SEC as f64) as i128);
    let u = kshana_time::Utc::from_instant(local);
    format!("{:02}:{:02}", u.hour, u.minute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn every_span_speaks_plainly() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let planets = ["Sun ", "Moon ", "Mars", "Mercury", "Jupiter", "Venus", "Saturn", "Rahu", "Ketu", "dasha", "hora", "tara"];
        for span in [Span::Day, Span::Week, Span::Month, Span::Year] {
            let o = outlook(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, span, at).unwrap();
            assert_eq!(o.lives.len(), 9);
            assert!(!o.headline.is_empty());
            for p in planets {
                assert!(!o.headline.contains(p), "{span:?} headline names {p}: {}", o.headline);
                for l in &o.lives {
                    assert!(!l.words.contains(p), "{span:?} {:?} words name {p}: {}", l.life, l.words);
                    assert!(!l.advice.contains(p));
                }
            }
            assert!(o.lives.windows(2).all(|w| w[0].score >= w[1].score));
            assert!(o.lives.iter().all(|l| !l.why.is_empty() || span == Span::Year));
        }
    }
}
