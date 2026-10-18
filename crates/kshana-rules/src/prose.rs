//! The outlook in plain words: today, this week, this month, this year, for each life
//! and for the whole. Every sentence here is about the person's life (what goes well,
//! what asks patience, which day, what to do) and never names a planet. The astrology
//! that produced it travels alongside as `why`, for the tap that asks.

use crate::explain::explain;
use crate::feed::next_good;
use crate::forecast::{date_words, moment, series, slow_series, story, Band, ForecastError, Life, Natal, Story};
use crate::gochara::SaturnPhase;
use crate::varsha::year_chapter;
use crate::plain::Lang;
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
        (Home, Bright) => "Home and property move well: sign, build, shift.",
        (Home, Good) => "Property matters progress: measure, plan, fix.",
        (Home, Steady) => "Property matters are stable: maintain, do not expand.",
        (Home, Slow) => "Property matters drag: small delays with land, rent or repairs.",
        (Home, Hold) => "Property matters are on hold: no deals about land or a home.",
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
        (Home, Bright | Good) => "Move on the property matter you have been putting off.",
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

/// Open a life's sentence with its span, so today, this week, this month and this year
/// never read alike: "This week, love is slow: …".
fn span_open(lang: Lang, span: Span, core: &str) -> String {
    let opener = match (lang, span) {
        (Lang::En, Span::Day) => "Today",
        (Lang::En, Span::Week) => "This week",
        (Lang::En, Span::Month) => "This month",
        (Lang::En, Span::Year) => "Over the coming year",
        (Lang::Ml, Span::Day) => "ഇന്ന്",
        (Lang::Ml, Span::Week) => "ഈ ആഴ്ച",
        (Lang::Ml, Span::Month) => "ഈ മാസം",
        (Lang::Ml, Span::Year) => "വരുന്ന ഒരു വർഷം",
        (Lang::Hi, Span::Day) => "आज",
        (Lang::Hi, Span::Week) => "इस हफ़्ते",
        (Lang::Hi, Span::Month) => "इस महीने",
        (Lang::Hi, Span::Year) => "आने वाले साल में",
    };
    if lang != Lang::En {
        return format!("{opener}, {core}");
    }
    // Lower the first letter, except where the sentence opens with a name for a person.
    let mut c = core.chars();
    let body = match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    };
    format!("{opener}, {body}")
}

/// The top paragraph for the day: the day's quality and its unlucky stretch, if one is still ahead.
fn day_season(lang: Lang, m: &crate::forecast::Moment, at: Instant, lon: f64) -> String {
    let dq = crate::lang::day_quality(lang, m.tara);
    let w = m.windows.iter().find(|w| w.caution && w.end > at);
    let first = match lang {
        Lang::En => format!("Today is {}.", dq.trim_end_matches('.')),
        Lang::Ml => format!("ഇന്ന്: {}.", dq.trim_end_matches('.')),
        Lang::Hi => format!("आज: {}।", dq.trim_end_matches(['.', '।'])),
    };
    let second = match (lang, w) {
        (Lang::En, Some(w)) => format!(" The {} runs {} to {}: finish things then, begin nothing.", w.label.to_lowercase(), hm(w.start, lon), hm(w.end, lon)),
        (Lang::Ml, Some(w)) => format!(" {} {} മുതൽ {} വരെ: അപ്പോൾ ഒന്നും തുടങ്ങരുത്.", crate::lang::window_label(lang, &w.label), hm(w.start, lon), hm(w.end, lon)),
        (Lang::Hi, Some(w)) => format!(" {} {} से {} तक: तब कुछ नया शुरू न करें।", crate::lang::window_label(lang, &w.label), hm(w.start, lon), hm(w.end, lon)),
        (Lang::En, None) => " The day's unlucky stretches are behind you.".to_string(),
        (Lang::Ml, None) => " ഇന്നത്തെ നിർഭാഗ്യ സമയങ്ങൾ കഴിഞ്ഞു.".to_string(),
        (Lang::Hi, None) => " आज का अशुभ समय निकल चुका है।".to_string(),
    };
    first + &second
}

/// The top paragraph for a week or a month: its best and hardest day across your life,
/// what those days are like, and for a month the shorter phase it sits in.
fn stretch_season(lang: Lang, span: Span, samples: &[crate::forecast::Sample], levels: &[kshana_vedic::DashaPeriod], lon: f64, focus: &[Life]) -> String {
    let ld = |t: Instant| crate::lang::date(lang, t, lon);
    let idx: Vec<usize> = Life::ALL.iter().enumerate().filter(|(_, l)| focus.is_empty() || focus.contains(l)).map(|(i, _)| i).collect();
    let mean = |s: &crate::forecast::Sample| idx.iter().map(|&i| s.scores[i]).sum::<f64>() / idx.len().max(1) as f64;
    let best = samples.iter().max_by(|a, b| mean(a).partial_cmp(&mean(b)).unwrap_or(std::cmp::Ordering::Equal));
    let worst = samples.iter().min_by(|a, b| mean(a).partial_cmp(&mean(b)).unwrap_or(std::cmp::Ordering::Equal));
    let mut s = String::new();
    if span == Span::Month {
        if let Some(p) = levels.get(2) {
            let th = crate::lang::chapter_theme(lang, p.lord);
            // When the phase ends inside the month, say what comes after it.
            let ends_soon = samples.last().map(|l| p.end < l.at).unwrap_or(false);
            let idx = kshana_vedic::DashaLord::ORDER.iter().position(|l| *l == p.lord).unwrap_or(0);
            let next = crate::lang::chapter_theme(lang, kshana_vedic::DashaLord::ORDER[(idx + 1) % 9]);
            s.push_str(&match (lang, ends_soon) {
                (Lang::En, false) => format!("Until {} a shorter phase puts {th} at the front. ", ld(p.end)),
                (Lang::En, true) => format!("Until {} a shorter phase puts {th} at the front; after that, {next} come forward. ", ld(p.end)),
                (Lang::Ml, false) => format!("{} വരെ ഒരു ചെറിയ ഘട്ടം {th} എന്നിവയെ മുന്നിലേക്ക് കൊണ്ടുവരുന്നു. ", ld(p.end)),
                (Lang::Ml, true) => format!("{} വരെ ഒരു ചെറിയ ഘട്ടം {th} എന്നിവയെ മുന്നിലേക്ക് കൊണ്ടുവരുന്നു; അതിനുശേഷം {next} മുന്നിലേക്ക് വരും. ", ld(p.end)),
                (Lang::Hi, false) => format!("{} तक एक छोटा दौर {th} को आगे लाता है। ", ld(p.end)),
                (Lang::Hi, true) => format!("{} तक एक छोटा दौर {th} को आगे लाता है; उसके बाद {next} आगे आएँगे। ", ld(p.end)),
            });
        }
    }
    if let (Some(b), Some(w)) = (best, worst) {
        let noun = crate::lang::span_noun(lang, span.key());
        let bq = crate::lang::day_short(lang, b.tara);
        let wq = crate::lang::day_short(lang, w.tara);
        s.push_str(&match lang {
            Lang::En => format!(
                "Taking every part of your life together, your best day this {} is {}, {}: put the important things there. Go gently on {}, {}.",
                span.noun(), ld(b.at), bq, ld(w.at), wq
            ),
            Lang::Ml => format!("ഈ {noun} നിങ്ങളുടെ ഏറ്റവും നല്ല ദിവസം {} ആണ് ({bq}): പ്രധാന കാര്യങ്ങൾ അന്ന് ചെയ്യുക. {} സൂക്ഷിക്കുക ({wq}).", ld(b.at), ld(w.at)),
            Lang::Hi => format!("इस {noun} आपका सबसे अच्छा दिन {} है ({bq}): ज़रूरी काम उसी दिन रखें। {} को सँभलकर चलें ({wq})।", ld(b.at), ld(w.at)),
        });
    }
    s
}

fn day_name_words(t: Instant, lon: f64, span: Span) -> String {
    match span {
        Span::Day => String::new(),
        _ => date_words(t, lon),
    }
}

/// Saturn's phase in plain words, without the planet.
pub fn saturn_plain(phase: SaturnPhase) -> &'static str {
    crate::plain::long_test(phase)
}

/// The outlook for `span` from `at`, in English.
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
    outlook_in(spk, eop, natal, obs, system, ayanamsa, span, at, Lang::En, &[])
}

/// The outlook for `span` from `at`, in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn outlook_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    span: Span,
    at: Instant,
    lang: Lang,
    focus: &[Life],
) -> Result<Outlook, ForecastError> {
    let in_focus = |l: Life| focus.is_empty() || focus.contains(&l);
    let ld = |t: Instant| crate::lang::date(lang, t, obs.longitude_deg);
    let bw = |life: Life, band: Band| -> String {
        let core = crate::lang::band_sentence(lang, life, band).map(|s| s.to_string()).unwrap_or_else(|| band_words(life, band, span).to_string());
        span_open(lang, span, &core)
    };
    let aw = |life: Life, band: Band| -> String { crate::lang::advice_sentence(lang, life, band).map(|s| s.to_string()).unwrap_or_else(|| advice_words(life, band).to_string()) };
    let lon = obs.longitude_deg;
    let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let season_why = m.saturn.expert().to_string();
    let levels = &m.dasha;
    let long_season = {
        let (test, _) = crate::briefing::saturn_sentence_in(lang, spk, ayanamsa, natal, m.saturn, at, lon);
        let stop = if lang == Lang::Hi { "।" } else { "." };
        match levels.get(1) {
            Some(p) => {
                let th = crate::lang::chapter_theme(lang, p.lord);
                match lang {
                    Lang::En => format!("Until {} the most movement in your life is in {th}. {test}.", ld(p.end)),
                    Lang::Ml => format!("{} വരെ ജീവിതത്തിൽ ഏറ്റവും കൂടുതൽ ചലനം {th} എന്നിവയിലാണ്. {test}.", ld(p.end)),
                    Lang::Hi => format!("{} तक ज़िंदगी में सबसे ज़्यादा हलचल {th} में है। {test}।", ld(p.end)),
                }
            }
            None => format!("{test}{stop}"),
        }
    };

    let (from, to, lives, season): (Instant, Instant, Vec<LifeOutlook>, String) = match span {
        Span::Day => {
            let good = next_good(&m, at, 0.56);
            let lives = Life::ALL
                .iter()
                .enumerate()
                .map(|(i, &life)| {
                    let ls = &m.lives[i];
                    let g = good[i].as_ref();
                    let caution = m.windows.iter().find(|w| w.caution && w.start > at).map(|w| w.start);
                    let mut words = format!("{} ", bw(life, ls.band));
                    match g {
                        Some(g) if g.now => words.push_str(&match lang {
                            Lang::En => format!("This hour serves it, until {}.", hm(g.end, lon)),
                            Lang::Ml => format!("ഈ മണിക്കൂർ ഇതിന് അനുകൂലമാണ്, {} വരെ.", hm(g.end, lon)),
                            Lang::Hi => format!("यह घंटा इसके लिए अच्छा है, {} तक।", hm(g.end, lon)),
                        }),
                        Some(g) => words.push_str(&match lang {
                            Lang::En => format!("Your best hour for it comes at {}.", hm(g.start, lon)),
                            Lang::Ml => format!("ഇതിനുള്ള നിങ്ങളുടെ ഏറ്റവും നല്ല മണിക്കൂർ {}-ന് വരും.", hm(g.start, lon)),
                            Lang::Hi => format!("इसके लिए आपका सबसे अच्छा घंटा {} बजे आएगा।", hm(g.start, lon)),
                        }),
                        None => words.push_str(lang.pick([
                            "No strong hour for it is left today; tomorrow's sunrise starts a new set.",
                            "ഇന്ന് ഇതിന് നല്ല മണിക്കൂർ ബാക്കിയില്ല; നാളെ സൂര്യോദയത്തോടെ പുതിയവ തുടങ്ങും.",
                            "आज इसके लिए कोई अच्छा घंटा नहीं बचा; कल सूर्योदय से नए शुरू होंगे।",
                        ])),
                    }
                    LifeOutlook {
                        life,
                        score: ls.score,
                        band: ls.band,
                        trend: "steady",
                        words: words.trim().to_string(),
                        advice: aw(life, ls.band),
                        best_at: g.map(|x| x.start),
                        best_score: g.map(|x| x.fit).unwrap_or(0.0),
                        care_at: caution,
                        care_score: 0.0,
                        why: ls.components.iter().map(|c| c.note.clone()).collect(),
                    }
                })
                .collect();
            (at, m.sunrise.map(|s| s.add_ns(NS_PER_DAY)).unwrap_or(at.add_ns(NS_PER_DAY)), lives, day_season(lang, &m, at, lon))
        }
        Span::Week | Span::Month => {
            let n = if span == Span::Week { 7 } else { 30 };
            let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, n)?;
            let lives = Life::ALL
                .iter()
                .map(|&life| {
                    let st: Story = story(life, &samples);
                    let band = Band::of(st.mean);
                    let mut words = bw(life, band);
                    let nn = crate::lang::span_noun(lang, span.key());
                    match st.trend {
                        "rising" => words.push_str(&match lang {
                            Lang::En => format!(" It improves as the {} goes on.", span.noun()),
                            Lang::Ml => format!(" {nn} മുന്നോട്ട് പോകുന്തോറും ഇത് മെച്ചപ്പെടും."),
                            Lang::Hi => format!(" जैसे-जैसे {nn} आगे बढ़ेगा, यह बेहतर होगा।"),
                        }),
                        "falling" => words.push_str(lang.pick([
                            " It starts stronger than it ends; act early.",
                            " തുടക്കം അവസാനത്തേക്കാൾ ശക്തമാണ്; നേരത്തെ പ്രവർത്തിക്കുക.",
                            " शुरुआत अंत से मज़बूत है; जल्दी कदम उठाएँ।",
                        ])),
                        _ => {}
                    }
                    if let Some((a, b, v)) = st.best {
                        if v >= 0.56 {
                            let range = b.ns_since(a) > NS_PER_DAY;
                            words.push_str(&match lang {
                                Lang::En => format!(" Best: {}{}.", day_name_words(a, lon, span), if range { format!(" to {}", date_words(b, lon)) } else { String::new() }),
                                Lang::Ml => if range { format!(" ഏറ്റവും നല്ലത്: {} മുതൽ {} വരെ.", ld(a), ld(b)) } else { format!(" ഏറ്റവും നല്ലത്: {}.", ld(a)) },
                                Lang::Hi => if range { format!(" सबसे अच्छा: {} से {} तक।", ld(a), ld(b)) } else { format!(" सबसे अच्छा: {}।", ld(a)) },
                            });
                        }
                    }
                    if let Some((a, _, v)) = st.low {
                        if v < 0.44 {
                            words.push_str(&match lang {
                                Lang::En => format!(" Go easy around {}.", date_words(a, lon)),
                                Lang::Ml => format!(" {}-ഓടെ സൂക്ഷിക്കുക.", ld(a)),
                                Lang::Hi => format!(" {} के आसपास सँभलकर चलें।", ld(a)),
                            });
                        }
                    }
                    let why: Vec<String> = st.shifts.iter().map(|s| format!("{}: {}", date_words(s.at, lon), s.what)).collect();
                    LifeOutlook {
                        life,
                        score: st.mean,
                        band,
                        trend: st.trend,
                        words,
                        advice: aw(life, band),
                        best_at: st.best.map(|b| b.0),
                        best_score: st.best.map(|b| b.2).unwrap_or(0.0),
                        care_at: st.low.map(|b| b.0),
                        care_score: st.low.map(|b| b.2).unwrap_or(0.0),
                        why,
                    }
                })
                .collect();
            (at, at.add_ns(n as i128 * NS_PER_DAY), lives, stretch_season(lang, span, &samples, levels, lon, focus))
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
                    let mut words = bw(life, band);
                    match trend {
                        "rising" => words.push_str(lang.pick([
                            " The year improves as it goes.",
                            " വർഷം മുന്നോട്ട് പോകുന്തോറും മെച്ചപ്പെടും.",
                            " साल आगे बढ़ने के साथ बेहतर होगा।",
                        ])),
                        "falling" => words.push_str(lang.pick([
                            " The first half is the stronger; move early.",
                            " ആദ്യപകുതിയാണ് ശക്തം; നേരത്തെ നീങ്ങുക.",
                            " पहला आधा ज़्यादा मज़बूत है; जल्दी कदम उठाएँ।",
                        ])),
                        _ => {}
                    }
                    if let (Some(b), Some(l)) = (best, low) {
                        words.push_str(&match lang {
                            Lang::En => format!(" Best month around {}; go easy around {}.", date_words(b.0, lon), date_words(l.0, lon)),
                            Lang::Ml => format!(" ഏറ്റവും നല്ല മാസം {}-ഓടെ; {}-ഓടെ സൂക്ഷിക്കുക.", ld(b.0), ld(l.0)),
                            Lang::Hi => format!(" सबसे अच्छा महीना {} के आसपास; {} के आसपास सँभलें।", ld(b.0), ld(l.0)),
                        });
                    }
                    let mut why = Vec::new();
                    if weak_two.contains(&i) && band <= Band::Steady {
                        if let Ok(e) = explain(spk, eop, natal, obs, system, ayanamsa, life, at) {
                            if let Some(t) = &e.eases {
                                words.push_str(&match lang {
                                    Lang::En => format!(" It eases from {}.", date_words(t.at, lon)),
                                    Lang::Ml => format!(" {} മുതൽ ഇത് അയയും.", ld(t.at)),
                                    Lang::Hi => format!(" {} से इसमें राहत मिलेगी।", ld(t.at)),
                                });
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
                        advice: aw(life, band),
                        best_at: best.map(|b| b.0),
                        best_score: best.map(|b| b.1).unwrap_or(0.0),
                        care_at: low.map(|b| b.0),
                        care_score: low.map(|b| b.1).unwrap_or(0.0),
                        why,
                    }
                })
                .collect();
            let _ = &yc;
            let (f, t) = (at, at.add_ns(12 * month));
            (f, t, lives, long_season.clone())
        }
    };

    let mut lives = lives;
    lives.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    // The summary speaks only of the parts of life the person chose; the rest are listed below it.
    let chosen: Vec<&LifeOutlook> = lives.iter().filter(|l| in_focus(l.life)).collect();
    let chosen = if chosen.is_empty() { lives.iter().collect::<Vec<_>>() } else { chosen };
    let score = chosen.iter().map(|l| l.score).sum::<f64>() / chosen.len() as f64;
    let band = Band::of(score);
    let top = chosen[0];
    let low = chosen[chosen.len() - 1];
    let noun = span.noun();
    let nn = crate::lang::span_noun(lang, span.key());
    let lead = match (lang, band) {
        (Lang::En, Band::Bright) => format!("A bright {noun}."),
        (Lang::En, Band::Good) => format!("A good {noun}."),
        (Lang::En, Band::Steady) => format!("An even {noun}."),
        (Lang::En, Band::Slow) => format!("A slow {noun}."),
        (Lang::En, Band::Hold) => format!("A {noun} to hold still."),
        (Lang::Ml, Band::Bright) => format!("തിളക്കമുള്ള {nn}."),
        (Lang::Ml, Band::Good) => format!("നല്ല {nn}."),
        (Lang::Ml, Band::Steady) => format!("സമനിലയുള്ള {nn}."),
        (Lang::Ml, Band::Slow) => format!("മന്ദമായ {nn}."),
        (Lang::Ml, Band::Hold) => format!("നിശ്ചലമായി നിൽക്കേണ്ട {nn}."),
        (Lang::Hi, Band::Bright) => format!("उज्ज्वल {nn}।"),
        (Lang::Hi, Band::Good) => format!("अच्छा {nn}।"),
        (Lang::Hi, Band::Steady) => format!("बराबर {nn}।"),
        (Lang::Hi, Band::Slow) => format!("धीमा {nn}।"),
        (Lang::Hi, Band::Hold) => format!("रुककर चलने वाला {nn}।"),
    };
    let best = top.best_at.filter(|_| span != Span::Day);
    let care = low.care_at.filter(|_| span != Span::Day);
    let mut headline = match lang {
        Lang::En => format!(
            "{lead} {} carries it{}; the most patience is needed for {}{}.",
            top.life.name(),
            best.map(|t| format!(", best around {}", date_words(t, lon))).unwrap_or_default(),
            low.life.name().to_ascii_lowercase(),
            care.map(|t| format!(", go easy around {}", date_words(t, lon))).unwrap_or_default()
        ),
        Lang::Ml => format!(
            "{lead} {} ആണ് ഏറ്റവും ശക്തം{}; ഏറ്റവും ക്ഷമ വേണ്ടത് {} എന്നതിനാണ്{}.",
            crate::lang::life_name(lang, top.life),
            best.map(|t| format!(", ഏറ്റവും നല്ലത് {}-ഓടെ", ld(t))).unwrap_or_default(),
            crate::lang::life_name(lang, low.life),
            care.map(|t| format!(", {}-ഓടെ സൂക്ഷിക്കുക", ld(t))).unwrap_or_default()
        ),
        Lang::Hi => format!(
            "{lead} {} सबसे मज़बूत है{}; सबसे ज़्यादा धैर्य {} में चाहिए{}।",
            crate::lang::life_name(lang, top.life),
            best.map(|t| format!(", सबसे अच्छा {} के आसपास", ld(t))).unwrap_or_default(),
            crate::lang::life_name(lang, low.life),
            care.map(|t| format!(", {} के आसपास सँभलें", ld(t))).unwrap_or_default()
        ),
    };
    let rising: Vec<String> = chosen.iter().filter(|l| l.trend == "rising").map(|l| crate::lang::life_lower(lang, l.life)).collect();
    if !rising.is_empty() && span != Span::Day {
        headline.push_str(&match lang {
            Lang::En => format!(" Improving as the {noun} goes: {}.", rising.join(", ")),
            Lang::Ml => format!(" {nn} മുന്നോട്ട് പോകുന്തോറും മെച്ചപ്പെടുന്നവ: {}.", rising.join(", ")),
            Lang::Hi => format!(" {nn} के साथ बेहतर होते जाएँगे: {}।", rising.join(", ")),
        });
    }
    if span == Span::Day {
        if let Some(g) = next_good(&m, at, 0.56).into_iter().flatten().filter(|g| !g.now && in_focus(g.life)).min_by_key(|g| g.start.tai_ns()) {
            headline.push_str(&match lang {
                Lang::En => format!(" The next good hour comes at {} for {}.", hm(g.start, lon), g.life.name().to_ascii_lowercase()),
                Lang::Ml => format!(" അടുത്ത നല്ല മണിക്കൂർ {}-ന്, {} എന്നതിന്.", hm(g.start, lon), crate::lang::life_name(lang, g.life)),
                Lang::Hi => format!(" अगला अच्छा घंटा {} बजे, {} के लिए।", hm(g.start, lon), crate::lang::life_name(lang, g.life)),
            });
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
        for span in [Span::Day, Span::Week, Span::Month, Span::Year] {
            let o = outlook(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, span, at).unwrap();
            assert_eq!(o.lives.len(), 9);
            assert!(!o.headline.is_empty());
            assert_eq!(crate::plain::jargon(&o.headline), None, "{span:?} headline: {}", o.headline);
            assert_eq!(crate::plain::jargon(&o.season), None, "{span:?} season: {}", o.season);
            for l in &o.lives {
                assert_eq!(crate::plain::jargon(&l.words), None, "{span:?} {:?}: {}", l.life, l.words);
                assert_eq!(crate::plain::jargon(&l.advice), None, "{span:?} {:?}: {}", l.life, l.advice);
            }
            assert!(o.lives.windows(2).all(|w| w[0].score >= w[1].score));
            assert!(o.lives.iter().all(|l| !l.why.is_empty() || span == Span::Year));
        }
    }
}

#[cfg(test)]
mod lang_tests {
    use super::*;
    use crate::plain::jargon_in;
    use kshana_time::Utc;

    #[test]
    fn every_span_speaks_plainly_in_malayalam_and_hindi() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for lang in [Lang::Ml, Lang::Hi] {
            for span in [Span::Day, Span::Week, Span::Month, Span::Year] {
                let o = outlook_in(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, span, at, lang, &[]).unwrap();
                for s in [&o.headline, &o.season] {
                    assert_eq!(jargon_in(lang, s), None, "{lang:?} {span:?}: {s}");
                    assert!(!s.contains("Love") && !s.contains("Career"), "{lang:?} {span:?} English left: {s}");
                }
                for l in &o.lives {
                    assert_eq!(jargon_in(lang, &l.words), None, "{lang:?} {span:?}: {}", l.words);
                    assert_eq!(jargon_in(lang, &l.advice), None, "{lang:?} {span:?}: {}", l.advice);
                }
            }
        }
    }
}

#[cfg(test)]
mod span_tests {
    use super::*;
    use kshana_time::Utc;

    /// Today, this week, this month and this year must each say their own thing: their
    /// own dates, their own headline, and their own words for each life.
    #[test]
    fn the_four_spans_say_different_things() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(spk) = Spk::open(&p) else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 18, 10, 0, 0, 0).unwrap().to_instant().unwrap();
        let mut seen: Vec<(Span, String, String)> = Vec::new();
        for span in [Span::Day, Span::Week, Span::Month, Span::Year] {
            let o = outlook(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, span, at).unwrap();
            println!("=== {span:?} {} → {} · {:.0}\n{}\n{}", date_words(o.from, 76.27), date_words(o.to, 76.27), o.score * 100.0, o.headline, o.season);
            for l in &o.lives {
                println!("  {:?} {:.0} {}: {} | {}", l.life, l.score * 100.0, l.trend, l.words, l.advice);
            }
            let words: String = o.lives.iter().map(|l| l.words.clone()).collect::<Vec<_>>().join("|");
            seen.push((span, o.headline.clone(), words));
        }
        for i in 0..seen.len() {
            for j in i + 1..seen.len() {
                assert_ne!(seen[i].1, seen[j].1, "{:?} and {:?} share a headline", seen[i].0, seen[j].0);
                assert_ne!(seen[i].2, seen[j].2, "{:?} and {:?} share every life's words", seen[i].0, seen[j].0);
            }
        }
    }
}
