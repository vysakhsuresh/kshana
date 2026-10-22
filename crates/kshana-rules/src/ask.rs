//! Ask: a plain answer to a question about one life, deduced from the birth chart,
//! the running periods and the sky ahead. The horary (Prashna) chart of the moment is
//! a witness, never a gate: a question is always answered.
//!
//! Each question is its own computation, because each asks something different:
//!
//! | question | what is weighed |
//! |---|---|
//! | Will it happen? | the promise: natal 0.35, running periods 0.30, the best of the next 180 days 0.20, the horary witness 0.15 |
//! | When? | timing: the nearest daily window at or above good, the next period level whose lord carries the houses, the first Moon window of the ruling planets; the score is how soon |
//! | Is now a good time? | the life score this second, the caution windows, the best hour left today |
//! | Should I? | the next seven days against the best stretch of the next 180, on the ground of the natal promise |
//! | Will it come back? | the slow line twelve months back against twenty-four ahead: a real dip, and whether the line climbs back to where it was |
//! | Will it be set right? | the first lasting lift (Explain) and the first month the slow line climbs a band |

use crate::areas::Area;
use crate::explain::explain;
use crate::facts::Facts;
use crate::forecast::{
    date_words, moment, natal_component_for, series, slow_series, story, Band, ForecastError, Life, Moment,
    Natal, Sample, Shift,
};
use crate::lang;
use crate::plain::Lang;
use crate::kp::{agreeing_levels, period_supports, timing};
use crate::prashna::{prashna, prashna_windows, QuestionShape, Verdict};
use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::dasha::{subdivide, vimshottari, YEAR_365_25_NS};
use kshana_vedic::{Ayanamsa, DashaPeriod};

/// What is being asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// Will it happen?
    WillItHappen,
    /// When?
    When,
    /// Is now a good time to act on it?
    GoodTimeNow,
    /// Should I take the step in front of me?
    ShouldI,
    /// Will it come back (a person, a thing, a position)?
    WillItReturn,
    /// Will they recover, will it be set right?
    WillItRecover,
}

impl Question {
    /// All, in display order.
    pub const ALL: [Question; 6] = [
        Question::WillItHappen,
        Question::When,
        Question::GoodTimeNow,
        Question::ShouldI,
        Question::WillItReturn,
        Question::WillItRecover,
    ];

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Question::WillItHappen => "will_it_happen",
            Question::When => "when",
            Question::GoodTimeNow => "good_time_now",
            Question::ShouldI => "should_i",
            Question::WillItReturn => "will_it_return",
            Question::WillItRecover => "will_it_recover",
        }
    }

    /// Parse a key.
    pub fn parse(s: &str) -> Option<Question> {
        Question::ALL.iter().copied().find(|q| q.key() == s)
    }

    /// The words on the button.
    pub fn words(self) -> &'static str {
        match self {
            Question::WillItHappen => "Will it happen?",
            Question::When => "When?",
            Question::GoodTimeNow => "Is now a good time?",
            Question::ShouldI => "Should I?",
            Question::WillItReturn => "Will it come back?",
            Question::WillItRecover => "Will it be set right?",
        }
    }

    /// The words on the button, in the reader's language.
    pub fn words_in(self, lang: Lang) -> &'static str {
        let t = match self {
            Question::WillItHappen => ["Will it happen?", "ഇത് നടക്കുമോ?", "क्या यह होगा?"],
            Question::When => ["When?", "എപ്പോൾ?", "कब?"],
            Question::GoodTimeNow => ["Is now a good time?", "ഇപ്പോൾ നല്ല സമയമാണോ?", "क्या अभी अच्छा समय है?"],
            Question::ShouldI => ["Should I?", "ഞാൻ ഇത് ചെയ്യണോ?", "क्या मुझे करना चाहिए?"],
            Question::WillItReturn => ["Will it come back?", "ഇത് തിരിച്ചുവരുമോ?", "क्या यह लौटेगा?"],
            Question::WillItRecover => ["Will it be set right?", "ഇത് ശരിയാകുമോ?", "क्या यह ठीक होगा?"],
        };
        lang.pick(t)
    }

    fn shape(self) -> QuestionShape {
        match self {
            Question::WillItHappen | Question::GoodTimeNow => QuestionShape::WillItHappen,
            Question::When => QuestionShape::When,
            Question::ShouldI => QuestionShape::Choice,
            Question::WillItReturn => QuestionShape::WillItReturn,
            Question::WillItRecover => QuestionShape::WillTheyRecover,
        }
    }
}

/// The answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    /// Which life.
    pub life: Life,
    /// What was asked.
    pub question: Question,
    /// The verdict in a few words.
    pub verdict: String,
    /// The verdict in English, whatever the reader's language: a stable phrase the
    /// pair reading compares ("Yes", "Not yet" …).
    pub verdict_en: String,
    /// Combined score in `[0, 1]`.
    pub score: f64,
    /// How much the sources agree, `[0, 1]`.
    pub confidence: f64,
    /// One to three sentences, with dates.
    pub headline: String,
    /// The window to act in, when there is one.
    pub when_start: Option<Instant>,
    /// End of that window.
    pub when_end: Option<Instant>,
    /// The window in words (without dates; the app adds them).
    pub when_words: String,
    /// The first Moon window from the horary chart's ruling planets, within 30 days.
    pub first_opening_start: Option<Instant>,
    /// Its end.
    pub first_opening_end: Option<Instant>,
    /// Plain reasons, strongest first.
    pub reasons: Vec<String>,
    /// Rule ids behind the reasons.
    pub rule_ids: Vec<&'static str>,
    /// The life score now, 0–1.
    pub now: f64,
    /// Mean of the 180-day series, 0–1.
    pub ahead_mean: f64,
    /// The best stretch's score, 0–1.
    pub ahead_best: f64,
    /// Shifts that matter in the period ahead.
    pub shifts: Vec<Shift>,
}

fn witness_score(v: Verdict) -> f64 {
    match v {
        Verdict::Yes => 1.0,
        Verdict::Delayed => 0.6,
        Verdict::Unclear => 0.5,
        Verdict::No => 0.2,
    }
}

fn agreement(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (1.0 - var.sqrt() * 2.5).clamp(0.3, 0.95)
}

fn life_index(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

/// The first stretch of at least `min_len` samples at or above `floor`, nearest first;
/// falls back to the single best sample.
fn best_window(samples: &[Sample], idx: usize, floor: f64, min_len: usize) -> Option<(Instant, Instant, f64)> {
    let s: Vec<f64> = samples.iter().map(|x| x.scores[idx]).collect();
    let n = s.len();
    let mut i = 0;
    while i < n {
        if s[i] >= floor {
            let mut j = i;
            while j + 1 < n && s[j + 1] >= floor {
                j += 1;
            }
            if j + 1 - i >= min_len {
                let peak = s[i..=j].iter().cloned().fold(0.0_f64, f64::max);
                return Some((samples[i].at, samples[j].at, peak));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    story(Life::ALL[idx], samples).best
}

/// The next period (antardasha or pratyantar) that begins after `at` within two years
/// and whose lord carries the area's houses: `(period, level name)`.
fn next_carrying_period(natal: &Natal, area: Area, at: Instant) -> Option<(DashaPeriod, &'static str)> {
    let horizon = at.add_ns(730 * NS_PER_DAY);
    let mut best: Option<(DashaPeriod, &'static str)> = None;
    let mut consider = |p: DashaPeriod, level: &'static str| {
        if p.start > at && p.start <= horizon && !period_supports(&natal.facts, area, p.lord).is_empty() {
            if best.as_ref().map_or(true, |b| p.start < b.0.start) {
                best = Some((p, level));
            }
        }
    };
    for maha in vimshottari(natal.moon_sidereal, natal.birth, YEAR_365_25_NS) {
        if maha.end <= at || maha.start > horizon {
            continue;
        }
        for antar in subdivide(maha) {
            if antar.end <= at || antar.start > horizon {
                continue;
            }
            consider(antar, "antardasha");
            if antar.start <= at {
                for pr in subdivide(antar) {
                    consider(pr, "pratyantar");
                }
            }
        }
    }
    best
}

/// What the person was born with for a matter, in Malayalam or Hindi (the English is
/// `plain::birth_base`).
fn birth_base_in(lang: Lang, n: &str, score: f64) -> String {
    let t = if score >= 0.6 {
        [
            format!("{n}: ജന്മനാ നല്ല കരുത്തുണ്ട്; മോശം കാലങ്ങളിൽ നിന്ന് ഇത് നന്നായി കരകയറും"),
            format!("{n}: इसमें आपको जन्म से अच्छी ताक़त मिली है; बुरे दौर से यह अच्छी तरह उबरता है"),
        ]
    } else if score >= 0.45 {
        [
            format!("{n}: ജന്മനാ ഇടത്തരം അടിത്തറയാണ്; ഫലം നിങ്ങളുടെ പരിശ്രമത്തിനൊത്ത് വരും"),
            format!("{n}: इसमें जन्म से औसत नींव है; फल आपकी मेहनत के हिसाब से आता है"),
        ]
    } else {
        [
            format!("{n}: ജന്മനാ അല്പം ദുർബലമായ ഇടമാണ്; ജീവിതം മുഴുവൻ സ്ഥിരമായ ശ്രദ്ധ കൊടുത്താൽ ഇത് നന്നായി നിൽക്കും"),
            format!("{n}: यह जन्म से थोड़ा कमज़ोर पक्ष है; जीवन भर लगातार देखभाल से यह अच्छा टिकता है"),
        ]
    };
    let [ml, hi] = t;
    if lang == Lang::Hi { hi } else { ml }
}

fn months_between(a: Instant, b: Instant) -> f64 {
    b.ns_since(a) as f64 / (30.4375 * NS_PER_DAY as f64)
}

/// Answer a question about one life at `at`, for a person born as `natal` describes,
/// standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn answer(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    question: Question,
    at: Instant,
) -> Result<Answer, ForecastError> {
    answer_in(spk, eop, natal, obs, system, ayanamsa, life, question, at, Lang::En)
}

/// [`answer`] in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn answer_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    question: Question,
    at: Instant,
    lang: Lang,
) -> Result<Answer, ForecastError> {
    let label = Label { en: life.name().to_ascii_lowercase(), local: lang::life_name(lang, life).to_string() };
    answer_with(spk, eop, natal, obs, system, ayanamsa, life, life.area(), &label, question, at, lang)
}

/// Answer a question about a topic of the dictionary: the topic's houses carry the
/// promise and the timing, the life's daily sky carries the rest.
#[allow(clippy::too_many_arguments)]
pub fn answer_topic(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    topic: &crate::topics::Topic,
    question: Question,
    at: Instant,
) -> Result<Answer, ForecastError> {
    answer_topic_in(spk, eop, natal, obs, system, ayanamsa, topic, question, at, Lang::En)
}

/// [`answer_topic`] in the reader's language.
#[allow(clippy::too_many_arguments)]
pub fn answer_topic_in(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    topic: &crate::topics::Topic,
    question: Question,
    at: Instant,
    lang: Lang,
) -> Result<Answer, ForecastError> {
    let label = Label { en: topic.words.to_ascii_lowercase(), local: topic.words_in(lang).to_string() };
    answer_with(spk, eop, natal, obs, system, ayanamsa, topic.life, topic.area, &label, question, at, lang)
}

/// The matter asked about: the English label (lower case) the engine computes with,
/// and the reader's own words for it.
struct Label {
    en: String,
    local: String,
}

#[allow(clippy::too_many_arguments)]
fn answer_with(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    area: Area,
    label: &Label,
    question: Question,
    at: Instant,
    lang: Lang,
) -> Result<Answer, ForecastError> {
    let idx = life_index(life);
    let name = label.en.clone();
    // The reader's word for the matter (Malayalam and Hindi), and the sentence picker:
    // English stays exactly as it was; the others are written for a common reader, with
    // the matter set at the head of the sentence ("വിവാഹം: …") so no case ending is
    // forced onto it.
    let n = label.local.as_str();
    let tr = |t: [String; 3]| -> String {
        let [en, ml, hi] = t;
        match lang {
            Lang::En => en,
            Lang::Ml => ml,
            Lang::Hi => hi,
        }
    };
    let dw = |t: Instant| -> String {
        match lang {
            Lang::En => date_words(t, obs.longitude_deg),
            _ => lang::date(lang, t, obs.longitude_deg),
        }
    };
    let bw = |b: Band| -> String {
        match lang {
            Lang::En => b.word().to_string(),
            _ => lang::band_word(lang, b).to_string(),
        }
    };
    let title = {
        let name = if name.eq_ignore_ascii_case("children") { "having children".to_string() } else { name.clone() };
        let mut c = name.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    };
    let lon = obs.longitude_deg;

    // The common ground: the birth chart, the moment, the periods, the sky ahead and
    // the witness. Every question reads these differently.
    let natal_c = natal_component_for(natal, area, &name);
    let m: Moment = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    let now = m.lives[idx].score;
    let funnel = timing(&natal.facts, area, natal.moon_sidereal, natal.birth, at, 5);
    let agreeing = agreeing_levels(&funnel);
    let period = (0.6 * (agreeing as f64 / 5.0) + 0.4 * now).clamp(0.0, 1.0);
    let period_words = match agreeing {
        0 => tr([
            format!("the phase of life you are in does not favour {name} yet"),
            format!("{n}: നിങ്ങൾ ഇപ്പോൾ കടന്നുപോകുന്ന ജീവിതഘട്ടം ഇതിന് ഇനിയും അനുകൂലമല്ല"),
            format!("{n}: आप जिस जीवन-दौर में हैं, वह अभी इसके पक्ष में नहीं है"),
        ]),
        1 | 2 => tr([
            format!("the phase of life you are in partly favours {name}"),
            format!("{n}: ഇപ്പോഴത്തെ ജീവിതഘട്ടം ഭാഗികമായി ഇതിന് അനുകൂലമാണ്"),
            format!("{n}: आपका अभी का जीवन-दौर कुछ हद तक इसके पक्ष में है"),
        ]),
        _ => tr([
            format!("the phase of life you are in strongly favours {name}"),
            format!("{n}: ഇപ്പോഴത്തെ ജീവിതഘട്ടം ഇതിന് ശക്തമായി അനുകൂലമാണ്"),
            format!("{n}: आपका अभी का जीवन-दौर पूरी तरह इसके पक्ष में है"),
        ]),
    };
    let samples = series(spk, eop, natal, obs, ayanamsa, at, NS_PER_DAY, 180)?;
    let st = crate::forecast::story_in(lang, life, &samples);
    let ahead_best = st.best.map(|b| b.2).unwrap_or(st.mean);
    let ahead = (0.5 * st.mean + 0.5 * ahead_best).clamp(0.0, 1.0);

    let horary = compute(
        spk,
        eop,
        &ChartInput {
            instant: at,
            observer: *obs,
            system,
            ayanamsa,
            topocentric_moon: false,
        },
    )
    .map_err(|e| ForecastError::Chart(format!("{e:?}")))?;
    let hf = Facts::from_chart(&horary);
    let pv = prashna(&hf, area, question.shape(), at, obs.longitude_deg);
    let witness = witness_score(pv.verdict);
    let witness_words = match pv.verdict {
        Verdict::Yes => tr([
            format!("the very moment you asked also points to yes for {name}"),
            format!("{n}: നിങ്ങൾ ചോദിച്ച ആ നിമിഷവും 'അതെ' എന്നു തന്നെ സൂചിപ്പിക്കുന്നു"),
            format!("{n}: आपने जिस पल पूछा, वह भी हाँ की ओर इशारा करता है"),
        ]),
        Verdict::Delayed => tr([
            format!("the very moment you asked says yes for {name}, but late"),
            format!("{n}: ചോദിച്ച ആ നിമിഷം 'അതെ' എന്നു പറയുന്നു, പക്ഷേ വൈകിയേ നടക്കൂ"),
            format!("{n}: आपने जिस पल पूछा, वह हाँ कहता है, पर देर से"),
        ]),
        Verdict::Unclear => lang
            .pick([
                "the very moment you asked gives no clear sign",
                "നിങ്ങൾ ചോദിച്ച ആ നിമിഷം വ്യക്തമായ സൂചനയൊന്നും തരുന്നില്ല",
                "आपने जिस पल पूछा, वह कोई साफ़ संकेत नहीं देता",
            ])
            .to_string(),
        Verdict::No => tr([
            format!("the very moment you asked holds back on {name}"),
            format!("{n}: ചോദിച്ച ആ നിമിഷം ഇതിനെ അല്പം പിടിച്ചുനിർത്തുന്നു"),
            format!("{n}: आपने जिस पल पूछा, वह इसे अभी थामे रखता है"),
        ]),
    };
    let openings = prashna_windows(spk, ayanamsa, &pv.agents, at, 30);
    let first_opening = openings.first().map(|w| (w.start, w.end));

    let mut reasons: Vec<String> = Vec::new();
    let mut rule_ids: Vec<&'static str> = Vec::new();
    let push_common = |reasons: &mut Vec<String>, rule_ids: &mut Vec<&'static str>| {
        reasons.push(match lang {
            Lang::En => crate::plain::birth_base(&name, natal_c.score),
            _ => birth_base_in(lang, n, natal_c.score),
        });
        rule_ids.push("KS-N-0001");
        reasons.push(period_words.clone());
        rule_ids.push("KS-F-0001");
        let (mean, best) = (st.mean * 100.0, ahead_best * 100.0);
        reasons.push(tr([
            format!("the next six months average {mean:.0} for {name}, with a best stretch at {best:.0}"),
            format!("{n}: അടുത്ത ആറു മാസത്തെ ശരാശരി {mean:.0}; ഏറ്റവും നല്ല സമയം {best:.0}"),
            format!("{n}: अगले छह महीनों का औसत {mean:.0} है, और सबसे अच्छा दौर {best:.0} पर"),
        ]));
        rule_ids.push("KS-G-0001");
        reasons.push(witness_words.clone());
        rule_ids.push("KS-P-0001");
    };

    // The verdict in all three languages: the English one is kept for the pair reading.
    let (verdict3, score, confidence, headline, when, when_words): ([&'static str; 3], f64, f64, String, Option<(Instant, Instant)>, String) = match question {
        // ---------------------------------------------------------------- will it happen
        Question::WillItHappen => {
            push_common(&mut reasons, &mut rule_ids);
            let parts = [natal_c.score, period, ahead, witness];
            let score = (0.35 * natal_c.score + 0.30 * period + 0.20 * ahead + 0.15 * witness).clamp(0.0, 1.0);
            let floor = if score >= 0.62 { 0.56 } else { 0.50 };
            let w = best_window(&samples, idx, floor, 2).or(st.best);
            let (v, h) = if score >= 0.62 {
                (["Yes", "അതെ", "हाँ"], tr([
                    format!("Yes. {} is written in your birth chart and the phase you are in brings it forward. The dates below are when it shows first.", title),
                    format!("അതെ. {n}: ജനനസമയം അനുസരിച്ച് ഇത് നിങ്ങളുടെ ജീവിതത്തിലുണ്ട്, ഇപ്പോഴത്തെ ജീവിതഘട്ടം അതിനെ മുന്നോട്ടു കൊണ്ടുവരുന്നു. താഴെയുള്ള തീയതികളിലാണ് ഇത് ആദ്യം തെളിയുക."),
                    format!("हाँ। {n}: आपके जन्म-समय में यह लिखा है, और आपका अभी का दौर इसे आगे लाता है। नीचे की तारीखों में यह सबसे पहले दिखेगा।"),
                ]))
            } else if score >= 0.50 {
                (["Yes, with patience", "അതെ, ക്ഷമയോടെ", "हाँ, धैर्य के साथ"], tr([
                    format!("Yes, but not quickly. {} comes through the dates below and your effort inside them; the phase you are in helps only in part.", title),
                    format!("അതെ, പക്ഷേ പെട്ടെന്നല്ല. {n}: താഴെയുള്ള തീയതികളിലൂടെയും അവയ്ക്കുള്ളിലെ നിങ്ങളുടെ പരിശ്രമത്തിലൂടെയുമാണ് ഇത് വരിക; ഇപ്പോഴത്തെ ജീവിതഘട്ടം ഭാഗികമായേ സഹായിക്കൂ."),
                    format!("हाँ, पर जल्दी नहीं। {n}: यह नीचे की तारीखों में और उनमें आपकी मेहनत से आता है; अभी का दौर बस कुछ हद तक मदद करता है।"),
                ]))
            } else if score >= 0.40 {
                (["Not yet", "ഇപ്പോഴല്ല", "अभी नहीं"], tr([
                    format!("Not yet. The phase you are in does not favour {name}. The nearest opening is below; until then the remedies do the work."),
                    format!("ഇപ്പോഴല്ല. {n}: ഇപ്പോഴത്തെ ജീവിതഘട്ടം ഇതിന് അനുകൂലമല്ല. ഏറ്റവും അടുത്ത അവസരം താഴെയുണ്ട്; അതുവരെ പരിഹാരങ്ങൾ വഴി കാട്ടും."),
                    format!("अभी नहीं। {n}: आपका अभी का दौर इसके पक्ष में नहीं है। सबसे पास का मौका नीचे है; तब तक उपाय काम करते हैं।"),
                ]))
            } else {
                (["Not in this phase", "ഈ ഘട്ടത്തിൽ ഇല്ല", "इस दौर में नहीं"], tr([
                    format!("Not in this phase of life. {} needs remedies and time before it is worth asking again.", title),
                    format!("ഈ ജീവിതഘട്ടത്തിൽ ഇല്ല. {n}: വീണ്ടും ചോദിക്കുംമുമ്പ് പരിഹാരങ്ങളും കുറച്ചു സമയവും വേണം; അതിനുള്ള വഴികൾ താഴെയുണ്ട്."),
                    format!("जीवन के इस दौर में नहीं। {n}: फिर पूछने से पहले उपाय और कुछ समय चाहिए; उनके रास्ते नीचे हैं।"),
                ]))
            };
            let ww = if score >= 0.50 {
                ["the window to act in", "പ്രവർത്തിക്കേണ്ട സമയം", "काम करने का समय"]
            } else {
                ["the nearest opening", "ഏറ്റവും അടുത്ത അവസരം", "सबसे पास का मौका"]
            };
            (v, score, agreement(&parts), h, w.map(|x| (x.0, x.1)), lang.pick(ww).to_string())
        }

        // ------------------------------------------------------------------------- when
        Question::When => {
            push_common(&mut reasons, &mut rule_ids);
            let daily = best_window(&samples, idx, 0.56, 3);
            let period_next = next_carrying_period(natal, area, at);
            let slow = slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(30 * NS_PER_DAY), (30.4375 * NS_PER_DAY as f64) as i128, 24)?;
            let slow_now_band = Band::of(m.lives[idx].score);
            let climb = slow.iter().find(|(_, v)| Band::of(v[idx]) > slow_now_band).map(|(t, v)| (*t, v[idx]));
            if let Some((p, level)) = &period_next {
                let long = *level == "antardasha";
                let d = dw(p.start);
                reasons.push(tr([
                    format!(
                        "a {} that favours {name} begins on {}",
                        if long { "stretch of a year or more" } else { "season of a few months" },
                        date_words(p.start, lon)
                    ),
                    format!("{n}: ഇതിന് അനുകൂലമായ {} {d}-ന് തുടങ്ങുന്നു", if long { "ഒരു വർഷമോ അതിലധികമോ നീളുന്ന കാലം" } else { "ഏതാനും മാസത്തെ ഒരു കാലം" }),
                    format!("{n}: इसके पक्ष में {} {d} को शुरू होता है", if long { "एक साल या उससे लंबा दौर" } else { "कुछ महीनों का दौर" }),
                ]));
                rule_ids.push("KS-F-0001");
            }
            // The answer is the soonest of a sound daily window and a carrying period.
            let soonest: Option<(Instant, Instant, &str, f64)> = match (daily, period_next) {
                (Some((s, e, pk)), Some((p, _))) => {
                    if s <= p.start {
                        Some((s, e, "daily", pk))
                    } else {
                        Some((p.start, p.end.min(p.start.add_ns(90 * NS_PER_DAY)), "period", 0.6))
                    }
                }
                (Some((s, e, pk)), None) => Some((s, e, "daily", pk)),
                (None, Some((p, _))) => Some((p.start, p.end.min(p.start.add_ns(90 * NS_PER_DAY)), "period", 0.6)),
                (None, None) => None,
            };
            match soonest {
                Some((s, e, kind, pk)) => {
                    let months = months_between(at, s).max(0.0);
                    let score = if months < 1.0 { 0.85 } else if months < 3.0 { 0.7 } else if months < 6.0 { 0.55 } else if months < 12.0 { 0.42 } else { 0.32 };
                    let v = if months < 1.0 {
                        ["Within the month", "ഈ മാസത്തിനുള്ളിൽ", "इसी महीने के भीतर"]
                    } else if months < 3.0 {
                        ["Within three months", "മൂന്നു മാസത്തിനുള്ളിൽ", "तीन महीनों के भीतर"]
                    } else if months < 6.0 {
                        ["Within six months", "ആറു മാസത്തിനുള്ളിൽ", "छह महीनों के भीतर"]
                    } else if months < 12.0 {
                        ["Within the year", "ഒരു വർഷത്തിനുള്ളിൽ", "साल भर के भीतर"]
                    } else {
                        ["Beyond a year", "ഒരു വർഷത്തിനപ്പുറം", "एक साल के बाद"]
                    };
                    let (ds, de, pk) = (dw(s), dw(e), pk * 100.0);
                    let mut h = if kind == "daily" {
                        tr([
                            format!("{} opens {} to {}, your best stretch for it at {:.0} out of 100. That is the first time to act.", title, date_words(s, lon), date_words(e, lon), pk),
                            format!("{n}: {ds} മുതൽ {de} വരെ വഴി തുറക്കുന്നു; ഇതിനുള്ള നിങ്ങളുടെ ഏറ്റവും നല്ല സമയം, 100-ൽ {pk:.0}. പ്രവർത്തിക്കാനുള്ള ആദ്യ അവസരം അതാണ്."),
                            format!("{n}: {ds} से {de} तक रास्ता खुलता है; यह इसके लिए आपका सबसे अच्छा दौर है, 100 में {pk:.0}। काम करने का पहला मौका यही है।"),
                        ])
                    } else {
                        tr([
                            format!("The first stretch of life that truly favours {name} begins {}. Before it no strong days come; use the time to prepare.", date_words(s, lon)),
                            format!("{n}: ഇതിന് ശരിക്കും അനുകൂലമായ ആദ്യ ജീവിതകാലം {ds}-ന് തുടങ്ങുന്നു. അതിനുമുമ്പ് ശക്തമായ ദിവസങ്ങൾ വരില്ല; ആ സമയം ഒരുങ്ങാൻ ഉപയോഗിക്കുക."),
                            format!("{n}: इसके सच में पक्ष वाला पहला दौर {ds} को शुरू होता है। उससे पहले मज़बूत दिन नहीं आते; यह समय तैयारी में लगाएँ।"),
                        ])
                    };
                    if let Some((p, _level)) = next_carrying_period(natal, area, at) {
                        if kind == "daily" {
                            let d = dw(p.start);
                            h.push_str(&tr([
                                format!(" A longer supportive stretch begins {} and holds it in place.", date_words(p.start, lon)),
                                format!(" {d} മുതൽ കൂടുതൽ നീണ്ട ഒരു തുണക്കാലം തുടങ്ങി ഇതിനെ ഉറപ്പിച്ചുനിർത്തും."),
                                format!(" {d} से एक लंबा सहारा देने वाला दौर शुरू होकर इसे टिकाए रखता है।"),
                            ]));
                        }
                    }
                    if let Some((t, v2)) = climb {
                        let (b, d) = (bw(Band::of(v2)), dw(t));
                        h.push_str(&tr([
                            format!(" The season itself climbs to {} from {}.", Band::of(v2).word(), date_words(t, lon)),
                            format!(" {d} മുതൽ കാലം തന്നെ '{b}' നിലയിലേക്ക് ഉയരും."),
                            format!(" {d} से समय खुद '{b}' तक चढ़ता है।"),
                        ]));
                    }
                    (v, score, agreement(&[natal_c.score, period, ahead]).max(0.5), h, Some((s, e)), lang.pick(["the first opening", "ആദ്യ അവസരം", "पहला मौका"]).to_string())
                }
                None => {
                    let h = match climb {
                        Some((t, v2)) => {
                            let (b, d) = (bw(Band::of(v2)), dw(t));
                            tr([
                                format!("No strong stretch within six months, and no new phase that favours {name} within two years. Things turn {} from {}; until then the remedies do the work.", Band::of(v2).word().to_ascii_lowercase(), date_words(t, lon)),
                                format!("{n}: ആറു മാസത്തിനുള്ളിൽ ശക്തമായ കാലമില്ല, രണ്ടു വർഷത്തിനുള്ളിൽ അനുകൂലമായ പുതിയ ഘട്ടവുമില്ല. {d} മുതൽ കാര്യങ്ങൾ '{b}' ആകും; അതുവരെ പരിഹാരങ്ങൾ വഴി കാട്ടും."),
                                format!("{n}: छह महीनों में कोई मज़बूत दौर नहीं, और दो साल में कोई नया पक्ष वाला दौर नहीं। {d} से बातें '{b}' होती हैं; तब तक उपाय काम करते हैं।"),
                            ])
                        }
                        None => tr([
                            format!("Not within two years: no phase that favours {name} begins, and things hold where they are. Lean on the remedies, and ask again after them."),
                            format!("{n}: രണ്ടു വർഷത്തിനുള്ളിൽ ഇല്ല. അനുകൂലമായ ഘട്ടമൊന്നും തുടങ്ങുന്നില്ല, കാര്യങ്ങൾ ഇപ്പോഴുള്ളിടത്തു തന്നെ നിൽക്കും. പരിഹാരങ്ങളിൽ ആശ്രയിക്കുക, അവയ്ക്കുശേഷം വീണ്ടും ചോദിക്കുക."),
                            format!("{n}: दो साल के भीतर नहीं। इसके पक्ष में कोई दौर शुरू नहीं होता, और बातें जहाँ हैं वहीं टिकी रहती हैं। उपायों का सहारा लें, और उनके बाद फिर पूछें।"),
                        ]),
                    };
                    (["Not within two years", "രണ്ടു വർഷത്തിനുള്ളിൽ ഇല്ല", "दो साल के भीतर नहीं"], 0.25, 0.6, h, climb.map(|(t, _)| (t, t.add_ns(30 * NS_PER_DAY))), lang.pick(["the season's turn", "കാലം മാറുന്ന സമയം", "समय का मोड़"]).to_string())
                }
            }
        }

        // ------------------------------------------------------------- is now a good time
        Question::GoodTimeNow => {
            if let Some(h) = &m.hora {
                reasons.push(tr([
                    format!("this hour is {}", crate::plain::hour_nature(h.lord)),
                    format!("ഈ മണിക്കൂർ: {}", lang::hour_nature(lang, h.lord)),
                    format!("यह घंटा: {}", lang::hour_nature(lang, h.lord)),
                ]));
            } else {
                reasons.push(match lang {
                    Lang::En => m.lives[idx].because.clone(),
                    _ => lang::band_sentence(lang, life, m.lives[idx].band).unwrap_or_default().to_string(),
                });
            }
            rule_ids.push("KS-H-0001");
            let (x, b) = (now * 100.0, bw(m.lives[idx].band));
            reasons.push(tr([
                format!("{name} reads {:.0} this second ({})", now * 100.0, m.lives[idx].band.word()),
                format!("{n}: ഈ നിമിഷം {x:.0} ({b})"),
                format!("{n}: इस पल {x:.0} ({b})"),
            ]));
            rule_ids.push("KS-M-0001");
            push_common(&mut reasons, &mut rule_ids);
            let band = m.lives[idx].band;
            let hour_fit = m
                .hours
                .iter()
                .filter(|h| h.end > at && !h.caution)
                .map(|h| (h.start, h.end, h.fit[idx]))
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let in_caution = m.windows.iter().find(|w| w.caution && w.start <= at && at < w.end).map(|w| w.label.clone());
            if let Some(c) = in_caution {
                let end = m.windows.iter().find(|w| w.label == c).map(|w| w.end).unwrap_or(at);
                (
                    ["Not this minute", "ഈ നിമിഷമല്ല", "इस पल नहीं"],
                    now.min(0.4),
                    0.8,
                    lang.pick([
                        "Not this minute: an unlucky stretch of the day is running. Finish things, do not begin them; it closes shortly and the better hours after it are below.",
                        "ഈ നിമിഷമല്ല: ദിവസത്തിലെ ഒരു നിർഭാഗ്യ സമയം നടക്കുകയാണ്. കാര്യങ്ങൾ തീർക്കുക, പുതിയതു തുടങ്ങരുത്; അത് ഉടൻ കഴിയും, അതിനുശേഷമുള്ള നല്ല മണിക്കൂറുകൾ താഴെയുണ്ട്.",
                        "इस पल नहीं: दिन का एक अशुभ समय चल रहा है। काम पूरे करें, नया शुरू न करें; यह जल्दी ख़त्म होगा और उसके बाद के अच्छे घंटे नीचे हैं।",
                    ])
                    .to_string(),
                    Some((end, m.hora.as_ref().map(|h| h.end.max(end)).unwrap_or(end))),
                    lang.pick(["after the caution ends", "ജാഗ്രതാസമയം കഴിഞ്ഞ്", "सावधानी का समय ख़त्म होने के बाद"]).to_string(),
                )
            } else if now >= 0.56 {
                let local = lang::band_sentence(lang, life, band).unwrap_or_default();
                (
                    ["Go ahead", "മുന്നോട്ടു പോകാം", "आगे बढ़ें"],
                    now,
                    0.85,
                    tr([
                        format!("{} This hour serves it.", m.lives[idx].headline),
                        format!("{local} ഈ മണിക്കൂർ ഇതിന് ഉതകുന്നു."),
                        format!("{local} यह घंटा इसमें साथ देता है।"),
                    ]),
                    Some((at, m.hora.as_ref().map(|h| h.end).unwrap_or(at))),
                    lang.pick(["now, until the hour turns", "ഇപ്പോൾ, ഈ മണിക്കൂർ മാറുന്നതുവരെ", "अभी, घंटा बदलने तक"]).to_string(),
                )
            } else if let Some((s, e, f)) = hour_fit.filter(|h| h.2 >= 0.5 && h.0 > at) {
                let slow_day = band < Band::Steady;
                let fit = f * 100.0;
                (
                    ["Wait for the hour", "ആ മണിക്കൂറിനായി കാത്തിരിക്കുക", "उस घंटे का इंतज़ार करें"],
                    (now + f) / 2.0,
                    0.7,
                    tr([
                        format!("Not this minute. A better hour comes later today{}.", if band >= Band::Steady { "" } else { ", and the day itself is slow" }),
                        format!("ഈ നിമിഷമല്ല. ഇന്നു പിന്നീട് ഒരു നല്ല മണിക്കൂർ വരും{}.", if slow_day { "; ദിവസം തന്നെ അല്പം മന്ദമാണ്" } else { "" }),
                        format!("इस पल नहीं। आज बाद में एक बेहतर घंटा आता है{}।", if slow_day { ", और दिन खुद धीमा है" } else { "" }),
                    ]),
                    Some((s, e)),
                    tr([
                        format!("the best hour left today, fit {:.0}", f * 100.0),
                        format!("ഇന്നു ബാക്കിയുള്ളതിൽ ഏറ്റവും നല്ല മണിക്കൂർ, ചേർച്ച {fit:.0}"),
                        format!("आज बचा सबसे अच्छा घंटा, मेल {fit:.0}"),
                    ]),
                )
            } else {
                (
                    ["Not today", "ഇന്നല്ല", "आज नहीं"],
                    now.min(0.42),
                    0.7,
                    tr([
                        format!("Today does not serve {name}. Let it wait for the stretch below."),
                        format!("{n}: ഇന്ന് ഇതിന് ഉതകുന്ന ദിവസമല്ല. താഴെയുള്ള സമയം വരെ കാത്തിരിക്കട്ടെ."),
                        format!("{n}: आज का दिन इसके लिए ठीक नहीं। इसे नीचे दिए दौर तक रुकने दें।"),
                    ]),
                    st.best.map(|b| (b.0, b.1)),
                    lang.pick(["the next good stretch", "അടുത്ത നല്ല സമയം", "अगला अच्छा दौर"]).to_string(),
                )
            }
        }

        // -------------------------------------------------------------------- should I
        Question::ShouldI => {
            push_common(&mut reasons, &mut rule_ids);
            let week: f64 = samples.iter().take(7).map(|s| s.scores[idx]).sum::<f64>() / 7.0_f64.min(samples.len() as f64);
            let best = st.best;
            let (wk, hb) = (week * 100.0, ahead_best * 100.0);
            reasons.insert(0, tr([
                format!("the next seven days average {:.0} for {name}; the best stretch of the half-year reaches {:.0}", week * 100.0, ahead_best * 100.0),
                format!("{n}: അടുത്ത ഏഴു ദിവസത്തെ ശരാശരി {wk:.0}; ഈ ആറു മാസത്തെ ഏറ്റവും നല്ല സമയം {hb:.0} വരെ എത്തുന്നു"),
                format!("{n}: अगले सात दिनों का औसत {wk:.0} है; इस छमाही का सबसे अच्छा दौर {hb:.0} तक पहुँचता है"),
            ]));
            rule_ids.insert(0, "KS-G-0001");
            let ground = natal_c.score;
            let gr = ground * 100.0;
            if ground >= 0.5 && week >= ahead_best - 0.06 && week >= 0.5 {
                (
                    ["Take it now", "ഇപ്പോൾ തന്നെ ചെയ്യുക", "अभी कर लें"],
                    (0.5 * week + 0.3 * ground + 0.2 * witness).clamp(0.0, 1.0),
                    agreement(&[ground, week, witness]),
                    tr([
                        format!("Take it now. The next seven days ({:.0}) are as good as anything the half-year offers ({:.0}), and your birth chart favours {name}.", week * 100.0, ahead_best * 100.0),
                        format!("ഇപ്പോൾ തന്നെ ചെയ്യുക. {n}: അടുത്ത ഏഴു ദിവസം ({wk:.0}) ഈ ആറു മാസം തരുന്ന ഏതു സമയത്തോളവും ({hb:.0}) നല്ലതാണ്, ജനനസമയവും ഇതിന് അനുകൂലമാണ്."),
                        format!("अभी कर लें। {n}: अगले सात दिन ({wk:.0}) इस छमाही के किसी भी समय ({hb:.0}) जितने अच्छे हैं, और आपका जन्म-समय भी इसके पक्ष में है।"),
                    ]),
                    Some((at, at.add_ns(7 * NS_PER_DAY))),
                    lang.pick(["this week", "ഈ ആഴ്ച", "इस हफ़्ते"]).to_string(),
                )
            } else if ground >= 0.5 && best.map_or(false, |b| b.2 >= week + 0.06) {
                let b = best.unwrap();
                let (d, bp) = (dw(b.0), b.2 * 100.0);
                (
                    ["Take it, from the window", "ചെയ്യാം, ആ സമയം മുതൽ", "करें, उस समय से"],
                    (0.4 * b.2 + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0),
                    agreement(&[ground, b.2, witness]),
                    tr([
                        format!("Take it, but not this week. This week reads {:.0}; from {} the same step reads {:.0}. Prepare now, move then.", week * 100.0, date_words(b.0, lon), b.2 * 100.0),
                        format!("ചെയ്യാം, പക്ഷേ ഈ ആഴ്ചയല്ല. ഈ ആഴ്ചയുടെ നില {wk:.0}; {d} മുതൽ ഇതേ ചുവടിന് {bp:.0}. ഇപ്പോൾ ഒരുങ്ങുക, അപ്പോൾ നീങ്ങുക."),
                        format!("करें, पर इस हफ़्ते नहीं। इस हफ़्ते का अंक {wk:.0} है; {d} से वही कदम {bp:.0} पर है। अभी तैयारी करें, तब आगे बढ़ें।"),
                    ]),
                    Some((b.0, b.1)),
                    lang.pick(["the window to move in", "നീങ്ങേണ്ട സമയം", "आगे बढ़ने का समय"]).to_string(),
                )
            } else if ground < 0.45 && week < 0.5 {
                (
                    ["Hold", "കാത്തിരിക്കുക", "रुकें"],
                    (0.4 * week + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0),
                    agreement(&[ground, week, witness]),
                    tr([
                        format!("Hold. Your birth chart does not make {name} easy ({:.0}) and the coming week is slow ({:.0}). If you must move, keep the stakes small and the exit open.", ground * 100.0, week * 100.0),
                        format!("ഇപ്പോൾ കാത്തിരിക്കുക. {n}: ജനനസമയം അനുസരിച്ച് ഇത് അത്ര എളുപ്പമല്ല ({gr:.0}), വരുന്ന ആഴ്ചയും മന്ദമാണ് ({wk:.0}). നീങ്ങിയേ തീരൂ എങ്കിൽ, പന്തയം ചെറുതാക്കുക, പിന്മാറാനുള്ള വഴി തുറന്നുവയ്ക്കുക."),
                        format!("अभी रुकें। {n}: आपके जन्म-समय में यह आसान नहीं है ({gr:.0}) और आने वाला हफ़्ता धीमा है ({wk:.0})। अगर आगे बढ़ना ही हो, तो दाँव छोटा रखें और लौटने का रास्ता खुला रखें।"),
                    ]),
                    best.map(|b| (b.0, b.1)),
                    lang.pick(["the gentlest window", "ഏറ്റവും സൗമ്യമായ സമയം", "सबसे नरम समय"]).to_string(),
                )
            } else {
                let willing = ground >= 0.5;
                (
                    ["Take it, with small stakes", "ചെയ്യാം, ചെറിയ തോതിൽ", "करें, छोटे दाँव से"],
                    (0.4 * week + 0.4 * ground + 0.2 * witness).clamp(0.0, 1.0),
                    agreement(&[ground, week, witness]),
                    tr([
                        format!("Take it with small stakes. The week is fair ({:.0}) and your birth chart is {} on {name}; a step you can undo is the right size.", week * 100.0, if ground >= 0.5 { "willing" } else { "cautious" }),
                        format!("ചെറിയ തോതിൽ ചെയ്യുക. {n}: ആഴ്ച തരക്കേടില്ല ({wk:.0}), ജനനസമയം ഇതിൽ {}; തിരുത്താവുന്ന ഒരു ചുവടാണ് ശരിയായ വലിപ്പം.", if willing { "അനുകൂലമാണ്" } else { "കരുതലോടെയാണ്" }),
                        format!("छोटे दाँव से करें। {n}: हफ़्ता ठीक-ठाक है ({wk:.0}) और आपका जन्म-समय इस पर {} है; ऐसा कदम जिसे वापस लिया जा सके, सही आकार का है।", if willing { "राज़ी" } else { "सतर्क" }),
                    ]),
                    Some((at, at.add_ns(7 * NS_PER_DAY))),
                    lang.pick(["this week, lightly", "ഈ ആഴ്ച, ലഘുവായി", "इस हफ़्ते, हल्के से"]).to_string(),
                )
            }
        }

        // ------------------------------------------------------------- will it come back
        Question::WillItReturn => {
            push_common(&mut reasons, &mut rule_ids);
            let month = (30.4375 * NS_PER_DAY as f64) as i128;
            let slow = slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(-12 * month), month, 37)?;
            let past: Vec<f64> = slow.iter().take(12).map(|(_, v)| v[idx]).collect();
            let future: Vec<(Instant, f64)> = slow.iter().skip(13).map(|(t, v)| (*t, v[idx])).collect();
            let now_slow = slow.get(12).map(|(_, v)| v[idx]).unwrap_or(now);
            let past_peak = past.iter().cloned().fold(0.0_f64, f64::max);
            let (fut_t, fut_peak) = future.iter().cloned().fold((at, 0.0_f64), |a, (t, v)| if v > a.1 { (t, v) } else { a });
            let back_at = future.iter().find(|(_, v)| *v >= past_peak - 0.03).map(|(t, _)| *t);
            let (pp, ns, fp, fd) = (past_peak * 100.0, now_slow * 100.0, fut_peak * 100.0, dw(fut_t));
            reasons.insert(0, tr([
                format!("the long-term trend of {name} peaked at {:.0} in the past year, stands at {:.0} now, and reaches {:.0} around {}", past_peak * 100.0, now_slow * 100.0, fut_peak * 100.0, date_words(fut_t, lon)),
                format!("{n}: ദീർഘകാല പ്രവണത കഴിഞ്ഞ വർഷം {pp:.0} വരെ ഉയർന്നു, ഇപ്പോൾ {ns:.0}, {fd} ഓടെ {fp:.0} എത്തും"),
                format!("{n}: लंबा रुझान पिछले साल {pp:.0} तक पहुँचा, अभी {ns:.0} पर है, और {fd} के आसपास {fp:.0} तक जाता है"),
            ]));
            rule_ids.insert(0, "KS-G-0001");
            let dip = now_slow < past_peak - 0.05;
            if dip && back_at.is_some() {
                let t = back_at.unwrap();
                let months = months_between(at, t);
                // A yes is at least a half; nearer and fuller returns score higher.
                let score = (0.58 + 0.25 * (1.0 - months / 24.0).max(0.0) + 0.5 * (fut_peak - past_peak).clamp(-0.1, 0.1) + 0.1 * (witness - 0.5)).clamp(0.5, 0.9);
                let v = if months <= 12.0 {
                    ["Yes, it comes back", "അതെ, തിരിച്ചുവരും", "हाँ, यह लौटेगा"]
                } else {
                    ["Yes, in its season", "അതെ, അതിന്റെ കാലത്ത്", "हाँ, अपने समय पर"]
                };
                let d = dw(t);
                (
                    v,
                    score,
                    agreement(&[natal_c.score, fut_peak, witness]),
                    tr([
                        format!("Yes. {} fell from {:.0} to {:.0} in the long-term trend, and climbs back to where it was around {}. Keep the door open and do not chase; the return is the season's, not an argument's.", title, past_peak * 100.0, now_slow * 100.0, date_words(t, lon)),
                        format!("അതെ. {n}: ദീർഘകാല പ്രവണതയിൽ {pp:.0}-ൽ നിന്ന് {ns:.0}-ലേക്ക് താഴ്ന്നു, {d} ഓടെ പഴയ നിലയിലേക്ക് തിരിച്ചുകയറും. വാതിൽ തുറന്നുവയ്ക്കുക, പിന്നാലെ ഓടരുത്; മടക്കം കാലത്തിന്റേതാണ്, തർക്കത്തിന്റേതല്ല."),
                        format!("हाँ। {n}: लंबे रुझान में यह {pp:.0} से {ns:.0} पर आया, और {d} के आसपास फिर वहीं पहुँचता है। दरवाज़ा खुला रखें और पीछे न भागें; लौटना समय का काम है, बहस का नहीं।"),
                    ]),
                    Some((t, t.add_ns(60 * NS_PER_DAY))),
                    lang.pick(["when it is back where it was", "പഴയ നിലയിൽ തിരിച്ചെത്തുമ്പോൾ", "जब यह फिर वहीं पहुँचे"]).to_string(),
                )
            } else if dip {
                let score = (0.42 + 0.1 * (witness - 0.5)).clamp(0.3, 0.55);
                (
                    ["Not as it was", "പഴയതുപോലെയല്ല", "पहले जैसा नहीं"],
                    score,
                    agreement(&[natal_c.score, fut_peak, witness]),
                    tr([
                        format!("Not as it was. {} eases to {:.0} around {} but does not reach the {:.0} of the past year within two years. What comes back comes changed, or what comes is new.", title, fut_peak * 100.0, date_words(fut_t, lon), past_peak * 100.0),
                        format!("പഴയതുപോലെയല്ല. {n}: {fd} ഓടെ {fp:.0} വരെ മെച്ചപ്പെടും, പക്ഷേ രണ്ടു വർഷത്തിനുള്ളിൽ കഴിഞ്ഞ വർഷത്തെ {pp:.0} എത്തില്ല. തിരിച്ചുവരുന്നത് മാറിയ രൂപത്തിലാകും, അല്ലെങ്കിൽ വരുന്നത് പുതിയ ഒന്നാകും."),
                        format!("पहले जैसा नहीं। {n}: {fd} के आसपास यह {fp:.0} तक सुधरता है, पर दो साल में पिछले साल के {pp:.0} तक नहीं पहुँचता। जो लौटता है वह बदलकर लौटता है, या जो आता है वह नया होता है।"),
                    ]),
                    Some((fut_t, fut_t.add_ns(60 * NS_PER_DAY))),
                    lang.pick(["the best it reaches", "എത്തുന്ന ഏറ്റവും നല്ല നില", "सबसे ऊँचा जहाँ तक पहुँचे"]).to_string(),
                )
            } else {
                let score = (0.6 + 0.2 * (witness - 0.5)).clamp(0.45, 0.8);
                (
                    ["Nothing has truly left", "ഒന്നും ശരിക്കും വിട്ടുപോയിട്ടില്ല", "कुछ भी सच में गया नहीं है"],
                    score,
                    agreement(&[natal_c.score, now_slow, witness]),
                    tr([
                        format!("Nothing has truly left. The long-term trend of {name} stands at {:.0}, as high as any month of the past year ({:.0}). What you miss is within reach; the move is yours, and the window below is the time for it.", now_slow * 100.0, past_peak * 100.0),
                        format!("ഒന്നും ശരിക്കും വിട്ടുപോയിട്ടില്ല. {n}: ദീർഘകാല പ്രവണത {ns:.0}-ലാണ്, കഴിഞ്ഞ വർഷത്തെ ഏതു മാസത്തോളവും ഉയരത്തിൽ ({pp:.0}). നിങ്ങൾക്ക് നഷ്ടമായതായി തോന്നുന്നത് കൈയെത്തും ദൂരത്തുണ്ട്; നീക്കം നിങ്ങളുടേതാണ്, അതിനുള്ള സമയം താഴെയുണ്ട്."),
                        format!("कुछ भी सच में गया नहीं है। {n}: लंबा रुझान {ns:.0} पर है, पिछले साल के किसी भी महीने जितना ऊँचा ({pp:.0})। जिसकी कमी लगती है, वह पहुँच में है; पहल आपकी है, और उसका समय नीचे है।"),
                    ]),
                    best_window(&samples, idx, 0.56, 2).map(|w| (w.0, w.1)),
                    lang.pick(["the time to reach out", "ബന്ധപ്പെടാനുള്ള സമയം", "हाथ बढ़ाने का समय"]).to_string(),
                )
            }
        }

        // ------------------------------------------------------------ will it be set right
        Question::WillItRecover => {
            push_common(&mut reasons, &mut rule_ids);
            let e = explain(spk, eop, natal, obs, system, ayanamsa, life, at)?;
            let lift = e.relief.iter().find(|r| r.lasting && r.gain >= 0.015).cloned();
            // What lifts, in the reader's words: the help the influence brings (the
            // engine's own phrase names it, so it stays for English only).
            let lift_what = |r: &crate::explain::Relief| -> String {
                match (lang, r.planet) {
                    (Lang::En, _) => r.what.clone(),
                    (_, Some(l)) => lang::support(lang, l).to_string(),
                    (_, None) => String::new(),
                }
            };
            if let Some(r) = &lift {
                let (d, w, g) = (dw(r.at), lift_what(r), r.gain * 100.0);
                let w = if w.is_empty() { String::new() } else { format!(": {w}") };
                reasons.insert(0, tr([
                    format!("the first lasting lift for {name} comes on {}: {} ({:+.0} points)", date_words(r.at, lon), r.what, r.gain * 100.0),
                    format!("{n}: ആദ്യത്തെ നിലനിൽക്കുന്ന ഉയർച്ച {d}-ന് വരും{w} ({g:+.0} പോയിന്റ്)"),
                    format!("{n}: पहला टिकाऊ सुधार {d} को आता है{w} ({g:+.0} अंक)"),
                ]));
                rule_ids.insert(0, "KS-G-0001");
            }
            if let Some(c) = e.causes.iter().find(|c| matches!(c.kind, "dasha" | "gochara")) {
                let weight = c.planet.map(|l| lang::pressure(lang, l).to_string());
                let until = c.until.map(|u| dw(u));
                reasons.insert(0, tr([
                    format!("what weighs on {name} now: {}{}", c.what, if c.until_words.is_empty() { String::new() } else { format!(", {}", c.until_words) }),
                    format!(
                        "{n}: ഇപ്പോൾ ഭാരമാകുന്നത്: {}{}",
                        weight.clone().unwrap_or_else(|| "ഒരു ഞെരുക്കം".to_string()),
                        until.as_ref().map(|d| format!(", {d} വരെ")).unwrap_or_default()
                    ),
                    format!(
                        "{n}: अभी इस पर भारी है: {}{}",
                        weight.unwrap_or_else(|| "कुछ दबाव".to_string()),
                        until.as_ref().map(|d| format!(", {d} तक")).unwrap_or_default()
                    ),
                ]));
                rule_ids.insert(0, c.rule_id);
            }
            match (&e.eases, &lift) {
                (Some(t), _) => {
                    let months = months_between(at, t.at);
                    let score = (0.8 - months / 60.0).clamp(0.5, 0.85);
                    let soon = months <= 12.0;
                    let (b, d) = (bw(t.band), dw(t.at));
                    let mut h = tr([
                        format!("{} {} climbs to {} from {}", if soon { "Yes, it mends." } else { "It mends, slowly." }, title, t.band.word(), date_words(t.at, lon)),
                        format!("{} {n}: {d} മുതൽ '{b}' നിലയിലേക്ക് ഉയരും", if soon { "അതെ, ശരിയാകും." } else { "ശരിയാകും, പതുക്കെ." }),
                        format!("{} {n}: {d} से यह '{b}' तक चढ़ता है", if soon { "हाँ, यह ठीक होगा।" } else { "ठीक होगा, धीरे-धीरे।" }),
                    ]);
                    if let Some(r) = &lift {
                        let (d, w) = (dw(r.at), lift_what(r));
                        h.push_str(&tr([
                            format!("; the first lift comes earlier, on {}, when {}", date_words(r.at, lon), r.what),
                            format!("; ആദ്യ ഉയർച്ച അതിനു മുമ്പേ, {d}-ന് വരും{}", if w.is_empty() { String::new() } else { format!(" ({w})") }),
                            format!("; पहला सुधार उससे पहले, {d} को आता है{}", if w.is_empty() { String::new() } else { format!(" ({w})") }),
                        ]));
                    }
                    h.push_str(lang.pick([
                        ". Steady care until then; the season does the rest.",
                        ". അതുവരെ സ്ഥിരമായ ശ്രദ്ധ മതി; ബാക്കി കാലം ചെയ്തുകൊള്ളും.",
                        "। तब तक लगातार देखभाल रखें; बाकी समय कर देगा।",
                    ]));
                    (
                        if soon { ["Yes, it mends", "അതെ, ശരിയാകും", "हाँ, ठीक होगा"] } else { ["It mends, slowly", "ശരിയാകും, പതുക്കെ", "ठीक होगा, धीरे-धीरे"] },
                        score,
                        agreement(&[natal_c.score, ahead, witness]).max(0.55),
                        h,
                        Some((t.at, t.at.add_ns(60 * NS_PER_DAY))),
                        lang.pick(["when it climbs a band", "ഒരു പടി ഉയരുമ്പോൾ", "जब यह एक दर्जा ऊपर चढ़े"]).to_string(),
                    )
                }
                (None, Some(r)) => {
                    let score = (0.5 + r.gain).clamp(0.4, 0.6);
                    let (d, w) = (dw(r.at), lift_what(r));
                    let w = if w.is_empty() { String::new() } else { format!(" ({w})") };
                    (
                        ["It steadies, slowly", "പതുക്കെ സ്ഥിരമാകും", "धीरे-धीरे संभलेगा"],
                        score,
                        agreement(&[natal_c.score, ahead, witness]),
                        tr([
                            format!("It steadies rather than mends. The first lift comes on {} ({}), but the season holds its band for three years. Hold what you have; the remedy below is the lever.", date_words(r.at, lon), r.what),
                            format!("പൂർണ്ണമായി ശരിയാകുന്നതിനേക്കാൾ ഇത് സ്ഥിരമാകും. ആദ്യ ഉയർച്ച {d}-ന് വരും{w}, പക്ഷേ മൂന്നു വർഷത്തേക്ക് കാലം ഇതേ നിലയിൽ തുടരും. ഉള്ളത് കാത്തുസൂക്ഷിക്കുക; താഴെയുള്ള പരിഹാരമാണ് വഴി."),
                            format!("पूरी तरह ठीक होने से ज़्यादा यह संभलेगा। पहला सुधार {d} को आता है{w}, पर तीन साल तक समय इसी दर्जे पर रहता है। जो है उसे संभालें; नीचे दिया उपाय ही रास्ता है।"),
                        ]),
                        Some((r.at, r.at.add_ns(30 * NS_PER_DAY))),
                        lang.pick(["the first lift", "ആദ്യ ഉയർച്ച", "पहला सुधार"]).to_string(),
                    )
                }
                (None, None) => (
                    ["Not in this period", "ഈ കാലത്ത് ഇല്ല", "इस दौर में नहीं"],
                    0.3,
                    0.6,
                    tr([
                        format!("Not in this period. Nothing in the next two years lifts {name} lastingly and the season holds. This is where the remedy, not the sky, must do the work."),
                        format!("ഈ കാലത്ത് ഇല്ല. {n}: അടുത്ത രണ്ടു വർഷത്തിൽ ഇതിനെ സ്ഥിരമായി ഉയർത്തുന്ന ഒന്നുമില്ല, കാലം ഇതേ നിലയിൽ തുടരും. ഇവിടെ ആകാശമല്ല, പരിഹാരമാണ് പണിയെടുക്കേണ്ടത്; അത് താഴെയുണ്ട്."),
                        format!("इस दौर में नहीं। {n}: अगले दो साल में कुछ भी इसे टिकाऊ ढंग से नहीं उठाता, और समय वैसा ही रहता है। यहाँ आसमान नहीं, उपाय को काम करना है; वह नीचे है।"),
                    ]),
                    st.best.map(|b| (b.0, b.1)),
                    lang.pick(["the best the sky gives meanwhile", "അതിനിടെ കിട്ടുന്ന ഏറ്റവും നല്ല സമയം", "इस बीच का सबसे अच्छा समय"]).to_string(),
                ),
            }
        }
    };

    if pv.hits.iter().any(|h| h.rule_id == "KS-P-0012") {
        reasons.push(
            lang.pick([
                "the very moment you asked shows a hold-up: expect a delay before it moves",
                "നിങ്ങൾ ചോദിച്ച ആ നിമിഷം ഒരു തടസ്സം കാണിക്കുന്നു: നീങ്ങിത്തുടങ്ങുംമുമ്പ് അല്പം താമസം പ്രതീക്ഷിക്കുക",
                "आपने जिस पल पूछा, उसमें एक अड़चन दिखती है: आगे बढ़ने से पहले कुछ देरी की उम्मीद रखें",
            ])
            .to_string(),
        );
        rule_ids.push("KS-P-0012");
    }

    Ok(Answer {
        life,
        question,
        verdict_en: verdict3[0].to_string(),
        verdict: lang.pick(verdict3).to_string(),
        score,
        confidence,
        headline,
        when_start: when.map(|w| w.0),
        when_end: when.map(|w| w.1),
        when_words,
        first_opening_start: first_opening.map(|w| w.0),
        first_opening_end: first_opening.map(|w| w.1),
        reasons,
        rule_ids,
        now,
        ahead_mean: st.mean,
        ahead_best,
        shifts: st.shifts.clone(),
    })
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
    fn every_question_about_every_life_is_answered_and_the_answers_differ() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for life in [Life::Love, Life::Career, Life::Money] {
            let mut headlines = Vec::new();
            let mut scores = Vec::new();
            for q in Question::ALL {
                let a = answer(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, life, q, at).unwrap();
                assert!(!a.verdict.is_empty() && !a.headline.is_empty());
                assert!((0.0..=1.0).contains(&a.score), "{q:?} {}", a.score);
                assert!((0.3..=0.95).contains(&a.confidence));
                assert!(a.reasons.len() >= 4);
                for s in std::iter::once(&a.verdict).chain(std::iter::once(&a.headline)).chain(std::iter::once(&a.when_words)).chain(a.reasons.iter()) {
                    assert_eq!(crate::plain::jargon(s), None, "{q:?}: {s}");
                }
                if let (Some(s), Some(e)) = (a.when_start, a.when_end) {
                    assert!(s <= e);
                }
                headlines.push(a.headline);
                scores.push(a.score);
            }
            // six questions, six different headlines; and the numbers are not all one number
            let mut uniq = headlines.clone();
            uniq.sort();
            uniq.dedup();
            assert_eq!(uniq.len(), 6, "{headlines:?}");
            let distinct = scores.iter().map(|s| (s * 100.0).round() as i64).collect::<std::collections::BTreeSet<_>>();
            assert!(distinct.len() >= 3, "{scores:?}");
        }
    }

    /// A run of three or more Latin letters: English left in a Malayalam or Hindi answer.
    fn latin_word(s: &str) -> Option<String> {
        s.split(|c: char| !c.is_ascii_alphabetic()).find(|w| w.len() >= 3).map(String::from)
    }

    #[test]
    fn answers_in_malayalam_and_hindi_are_plain_and_whole() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let topics = [crate::topics::topic("marriage").unwrap(), crate::topics::topic("buy_house").unwrap(), crate::topics::topic("recovery").unwrap()];
        for lang in [Lang::Ml, Lang::Hi] {
            for q in Question::ALL {
                let mut answers = vec![answer_in(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Love, q, at, lang).unwrap()];
                for t in topics {
                    answers.push(answer_topic_in(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, t, q, at, lang).unwrap());
                }
                // The language changes the words, never the reading.
                let en = answer(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Love, q, at).unwrap();
                assert_eq!((answers[0].score, answers[0].when_start, &answers[0].verdict_en), (en.score, en.when_start, &en.verdict));
                for a in answers {
                    assert!(!a.verdict_en.is_empty() && a.verdict_en.is_ascii());
                    for s in std::iter::once(&a.verdict).chain(std::iter::once(&a.headline)).chain(std::iter::once(&a.when_words)).chain(a.reasons.iter()) {
                        assert_eq!(crate::plain::jargon_in(lang, s), None, "{lang:?} {q:?}: {s}");
                        assert_eq!(latin_word(s), None, "{lang:?} {q:?}: {s}");
                    }
                }
            }
            for q in Question::ALL {
                assert_eq!(latin_word(q.words_in(lang)), None);
            }
        }
    }
}
