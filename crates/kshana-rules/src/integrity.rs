//! Keeping the record honest and learning from it honestly.
//!
//! * **Blind answers.** When a sealed prediction's dates have passed, the person is
//!   first asked, without seeing what was sealed, whether anything happened in that part
//!   of life between those dates, and which way ([`blind_question`], [`BlindAnswer`]).
//!   Only then is the sealed statement shown. The answer is turned into Yes / Close / No
//!   by [`blind_outcome`]: the same direction is a Yes, "some of both" a Close, the
//!   other direction or nothing a No.
//! * **Each way of reading earns its own hit rate** ([`system_rates`]): over the
//!   person's resolved sealed predictions, each way of reading that voted for the
//!   prediction is credited its points (Yes 1, Close ½, No 0). The rate is shrunk toward
//!   a shipped prior ([`system_prior`], from `integrity_prior.tsv`, starting at an even
//!   chance with the strength of 20 answers): `(points + m·s) / (votes + s)`, the mean of
//!   a Beta(m·s + points, (1−m)·s + votes − points) posterior. A way of reading moves the
//!   agreement vote only after [`MIN_VOTES_TO_WEIGHT`] votes, by at most ×0.5–×1.5
//!   ([`MULT_RANGE`]), and never so far that one holds more than [`MAX_SHARE`] of the
//!   whole say unless it already did ([`consensus_multipliers`]).
//! * **A number only when earned** ([`confidence_words`]): "about 7 in 10" beside a
//!   firm prediction only once [`MIN_FOR_NUMBER`] sealed predictions of that firmness
//!   have been answered, for the person or in the shipped pooled record (said as such);
//!   words alone before that.
//! * **Kept weeks against lapsed weeks** ([`remedy_weeks`], [`remedy_compare`]): the
//!   weeks a remedy was kept beside the weeks it lapsed, by the person's own record of
//!   good and hard things, with small numbers said to be small, and a gentle word to try
//!   another remedy when, after [`REMEDY_MIN_WEEKS`] weeks, keeping it made no
//!   difference.
//!
//! Only sealed predictions, answered after their dates and scored first, feed any of
//! this: nothing here ever touches the answers about the past that tune the birth time.

use crate::consensus::{System, NS, SYSTEMS};
use crate::forecast::Life;
use crate::lang::{date, life_name, month_year};
use crate::plain::Lang;
use kshana_time::{Instant, NS_PER_DAY};

// ---------------------------------------------------------------------------------
// Parts of life, whatever the feed calls them.

/// The part of life an area key names: a life key ("Money"), the ledger's areas
/// ("Wealth", "Marriage", …) or the family proof's keys ("gains", "abroad", …).
pub fn life_of_any(area: &str) -> Option<Life> {
    if let Some(l) = Life::parse(area.trim()) {
        return Some(l);
    }
    Some(match area.trim().to_ascii_lowercase().as_str() {
        "marriage" | "partner" | "relationship" => Life::Love,
        "wealth" | "gains" | "finance" => Life::Money,
        "property" | "house" => Life::Home,
        "education" => Life::Study,
        "foreign" | "abroad" => Life::Travel,
        "child" | "kids" => Life::Children,
        "work" | "job" => Life::Career,
        _ => return None,
    })
}

fn about(lang: Lang, life: Life) -> String {
    match lang {
        Lang::En => life_name(lang, life).to_lowercase(),
        _ => life_name(lang, life).to_string(),
    }
}

// ---------------------------------------------------------------------------------
// Blind answers.

/// What the person says happened, before seeing the sealed words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlindAnswer {
    /// Something good.
    Good,
    /// Something hard.
    Hard,
    /// Some of both.
    Mixed,
    /// Nothing much.
    Nothing,
    /// Not sure.
    Unsure,
}

impl BlindAnswer {
    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            BlindAnswer::Good => "good",
            BlindAnswer::Hard => "hard",
            BlindAnswer::Mixed => "mixed",
            BlindAnswer::Nothing => "nothing",
            BlindAnswer::Unsure => "unsure",
        }
    }

    /// From a key.
    pub fn parse(s: &str) -> Option<BlindAnswer> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "good" => BlindAnswer::Good,
            "hard" => BlindAnswer::Hard,
            "mixed" | "both" => BlindAnswer::Mixed,
            "nothing" | "none" => BlindAnswer::Nothing,
            "unsure" | "not sure" => BlindAnswer::Unsure,
            _ => return None,
        })
    }

    /// The button's words.
    pub fn label(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            BlindAnswer::Good => ["Something good", "നല്ലതെന്തോ", "कुछ अच्छा"],
            BlindAnswer::Hard => ["Something hard", "പ്രയാസമുള്ളതെന്തോ", "कुछ कठिन"],
            BlindAnswer::Mixed => ["Some of both", "രണ്ടും കുറച്ച്", "दोनों थोड़ा-थोड़ा"],
            BlindAnswer::Nothing => ["Nothing much", "കാര്യമായി ഒന്നുമില്ല", "कुछ खास नहीं"],
            BlindAnswer::Unsure => ["Not sure", "ഉറപ്പില്ല", "पक्का नहीं"],
        })
    }
}

/// Every answer, in the order the buttons show them.
pub const BLIND_ANSWERS: [BlindAnswer; 5] = [BlindAnswer::Good, BlindAnswer::Hard, BlindAnswer::Mixed, BlindAnswer::Nothing, BlindAnswer::Unsure];

/// The question asked before the sealed words are shown: "Did anything happen around
/// money between 14 Nov 2026 and 19 Nov 2026? Good, hard, or nothing much?" Days for a window of
/// six weeks or less, months for a longer one.
pub fn blind_question(lang: Lang, life: Life, from: Instant, to: Instant, lon: f64) -> String {
    let short = to.ns_since(from) <= 45 * NS_PER_DAY;
    let (a, b) = if short { (date(lang, from, lon), date(lang, to, lon)) } else { (month_year(lang, from, lon), month_year(lang, to, lon)) };
    let what = about(lang, life);
    let when = if a == b {
        match lang {
            Lang::En => format!("in {a}"),
            Lang::Ml => format!("{a}-ൽ"),
            Lang::Hi => format!("{a} में"),
        }
    } else {
        match lang {
            Lang::En => format!("between {a} and {b}"),
            Lang::Ml => format!("{a} മുതൽ {b} വരെ"),
            Lang::Hi => format!("{a} से {b} के बीच"),
        }
    };
    match lang {
        Lang::En => format!("Did anything happen around {what} {when}? Good, hard, or nothing much?"),
        Lang::Ml => format!("{when} {what} സംബന്ധിച്ച് എന്തെങ്കിലും സംഭവിച്ചോ? നല്ലതോ, പ്രയാസമുള്ളതോ, അതോ കാര്യമായി ഒന്നുമില്ലേ?"),
        Lang::Hi => format!("{when} {what} को लेकर कुछ हुआ? अच्छा, कठिन, या कुछ खास नहीं?"),
    }
}

/// The outcome key ("yes", "close", "no") a blind answer gives a claim that said the
/// stretch would be good (`claim_good`) or hard; none for "not sure".
pub fn blind_outcome(claim_good: bool, a: BlindAnswer) -> Option<&'static str> {
    match a {
        BlindAnswer::Unsure => None,
        BlindAnswer::Mixed => Some("close"),
        BlindAnswer::Good => Some(if claim_good { "yes" } else { "no" }),
        BlindAnswer::Hard => Some(if claim_good { "no" } else { "yes" }),
        BlindAnswer::Nothing => Some("no"),
    }
}

/// What the app says once the sealed words are shown: "This was sealed on 21 Oct 2026: …"
/// and how it turned out, misses as plainly as hits, a hard result never left alone.
pub fn blind_reveal(lang: Lang, sealed_at: Instant, statement: &str, claim_good: bool, a: BlindAnswer, lon: f64) -> String {
    let d = date(lang, sealed_at, lon);
    let first = match lang {
        Lang::En => format!("This was sealed on {d}: “{statement}”"),
        Lang::Ml => format!("ഇത് {d}-ന് മുദ്രവെച്ചതാണ്: “{statement}”"),
        Lang::Hi => format!("यह {d} को सील किया गया था: “{statement}”"),
    };
    let then = match (blind_outcome(claim_good, a), claim_good) {
        (None, _) => lang.pick(["You were not sure, so it is left out of the score.", "ഉറപ്പില്ലാത്തതിനാൽ ഇത് സ്കോറിൽ എണ്ണുന്നില്ല.", "आप पक्के नहीं थे, इसलिए यह स्कोर से बाहर रखा गया।"]),
        (Some("yes"), true) => lang.pick(["It came as sealed. That goes on your score.", "മുദ്രവെച്ചതുപോലെ വന്നു. അത് സ്കോറിൽ ചേർക്കുന്നു.", "जैसा सील किया था, वैसा ही हुआ। यह स्कोर में जुड़ता है।"]),
        (Some("yes"), false) => lang.pick([
            "It came as sealed. That goes on your score. A hard stretch passes, and what helps can ease the next one.",
            "മുദ്രവെച്ചതുപോലെ വന്നു. അത് സ്കോറിൽ ചേർക്കുന്നു. പ്രയാസമുള്ള കാലം കടന്നുപോകും; സഹായിക്കുന്നവ അടുത്തതിനെ ലഘൂകരിക്കും.",
            "जैसा सील किया था, वैसा ही हुआ। यह स्कोर में जुड़ता है। कठिन समय बीत जाता है, और जो मदद करता है वह अगले को हल्का कर सकता है।",
        ]),
        (Some("close"), _) => lang.pick(["Some of both: counted as half.", "രണ്ടും കുറച്ച്: പകുതിയായി എണ്ണുന്നു.", "दोनों थोड़ा-थोड़ा: आधा गिना गया।"]),
        (Some(_), true) => lang.pick([
            "It did not come as sealed. That goes on your score as plainly as a hit, and what helps is still there for the weeks ahead.",
            "മുദ്രവെച്ചതുപോലെ വന്നില്ല. ഒരു ശരി പോലെ തന്നെ വ്യക്തമായി അത് സ്കോറിൽ ചേർക്കുന്നു; വരും ആഴ്ചകൾക്ക് സഹായിക്കുന്നവ ഇപ്പോഴും ഉണ്ട്.",
            "जैसा सील किया था, वैसा नहीं हुआ। यह स्कोर में उतनी ही साफ़ गिनी जाती है जितनी सही बात, और आगे के हफ़्तों के लिए मदद अब भी है।",
        ]),
        (Some(_), false) => lang.pick([
            "The hard stretch did not come as sealed, which is good news. It goes on your score honestly, as a miss.",
            "പ്രയാസമുള്ള കാലം മുദ്രവെച്ചതുപോലെ വന്നില്ല; അത് നല്ല വാർത്തയാണ്. അത് സത്യസന്ധമായി ഒരു തെറ്റായി സ്കോറിൽ ചേർക്കുന്നു.",
            "कठिन समय जैसा सील किया था वैसा नहीं आया, यह अच्छी खबर है। यह ईमानदारी से एक चूक के रूप में स्कोर में जुड़ता है।",
        ]),
    };
    format!("{first} {then}")
}

// ---------------------------------------------------------------------------------
// The shipped prior.

/// The shipped prior file: per way of reading a mean and a strength, per tier the
/// pooled points and outcomes. Updated by the team from pooled sealed outcomes.
pub const PRIOR_TEXT: &str = include_str!("integrity_prior.tsv");

/// A way of reading's prior: its mean hit rate and its strength in pseudo-answers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prior {
    /// Prior mean, 0–1.
    pub mean: f64,
    /// Strength, in answers.
    pub strength: f64,
}

/// The default when a key is not in the file: an even chance worth twenty answers.
pub const DEFAULT_PRIOR: Prior = Prior { mean: 0.5, strength: 20.0 };

fn prior_rows() -> impl Iterator<Item = (&'static str, &'static str, f64, f64)> {
    PRIOR_TEXT.lines().filter_map(|l| {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            return None;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() != 4 {
            return None;
        }
        Some((f[0], f[1], f[2].parse().ok()?, f[3].parse().ok()?))
    })
}

/// The shipped prior for a way of reading by its key ("vimshottari"); the `*` row, then
/// [`DEFAULT_PRIOR`], for an unknown key.
pub fn system_prior(key: &str) -> Prior {
    let find = |k: &str| prior_rows().find(|r| r.0 == "system" && r.1 == k).map(|r| Prior { mean: r.2.clamp(0.01, 0.99), strength: r.3.max(1.0) });
    find(key).or_else(|| find("*")).unwrap_or(DEFAULT_PRIOR)
}

/// The shipped pooled record for a tier ("strong", "likely", "possible"): points scored
/// and outcomes resolved, across everyone. `(0, 0)` until the team ships pooled data.
pub fn tier_prior(tier: &str) -> (f64, usize) {
    prior_rows().find(|r| r.0 == "tier" && r.1 == tier).map(|r| (r.2.max(0.0), r.3.max(0.0) as usize)).unwrap_or((0.0, 0))
}

// ---------------------------------------------------------------------------------
// Each way of reading earns its own hit rate.

/// Fewest votes before a way of reading's own rate moves the agreement vote.
pub const MIN_VOTES_TO_WEIGHT: usize = 10;
/// The most a way of reading's say is multiplied or divided by.
pub const MULT_RANGE: (f64, f64) = (0.5, 1.5);
/// The largest share of the whole say a way of reading may be raised to.
pub const MAX_SHARE: f64 = 0.3;

/// One way of reading's standing for one person.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemRate {
    /// The key ("vimshottari").
    pub key: String,
    /// Resolved sealed predictions it voted for.
    pub votes: usize,
    /// Points scored on them.
    pub points: f64,
    /// The plain rate, points over votes (0.5 with none).
    pub raw: f64,
    /// The rate shrunk toward the shipped prior.
    pub rate: f64,
    /// Whether it has enough votes to move the agreement vote.
    pub weighted: bool,
}

/// Every way of reading's rate from `(key, points)` pairs, one per resolved sealed
/// prediction it voted for (Yes 1, Close ½, No 0). Sorted by key.
pub fn system_rates(votes: &[(String, f64)]) -> Vec<SystemRate> {
    let mut keys: Vec<&str> = votes.iter().map(|v| v.0.as_str()).collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .map(|k| {
            let mine: Vec<f64> = votes.iter().filter(|v| v.0 == k).map(|v| v.1.clamp(0.0, 1.0)).filter(|p| p.is_finite()).collect();
            let n = mine.len();
            let pts: f64 = mine.iter().sum();
            let p = system_prior(k);
            SystemRate {
                key: k.to_string(),
                votes: n,
                points: pts,
                raw: if n == 0 { 0.5 } else { pts / n as f64 },
                rate: (pts + p.mean * p.strength) / (n as f64 + p.strength),
                weighted: n >= MIN_VOTES_TO_WEIGHT,
            }
        })
        .collect()
}

/// The multiplier on each way of reading's say in the agreement vote, in
/// [`SYSTEMS`] order: 1 until it has [`MIN_VOTES_TO_WEIGHT`] votes, then its shrunk rate
/// over its prior mean, held within [`MULT_RANGE`], and lowered where needed so no way
/// of reading is raised above [`MAX_SHARE`] of the whole say.
pub fn consensus_multipliers(rates: &[SystemRate]) -> [f64; NS] {
    let mut m = [1.0; NS];
    for (i, s) in SYSTEMS.iter().enumerate() {
        if let Some(r) = rates.iter().find(|r| r.key == s.key() && r.weighted) {
            let p = system_prior(&r.key);
            m[i] = (r.rate / p.mean).clamp(MULT_RANGE.0, MULT_RANGE.1);
        }
    }
    let base: [f64; NS] = std::array::from_fn(|i| SYSTEMS[i].weight());
    // A few passes settle the cap: lowering one raises nobody else's share.
    for _ in 0..8 {
        let total: f64 = (0..NS).map(|i| base[i] * m[i]).sum();
        let mut changed = false;
        for i in 0..NS {
            if m[i] > 1.0 && base[i] * m[i] > MAX_SHARE * total {
                let others = total - base[i] * m[i];
                let cap = (MAX_SHARE * others / (1.0 - MAX_SHARE)) / base[i];
                let new = cap.max(1.0).min(m[i]);
                if (new - m[i]).abs() > 1e-12 {
                    m[i] = new;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    m
}

/// The plain sentence for a way of reading's own record: "the long-period reading: right
/// 8 of 12 for you so far."
pub fn system_rate_words(lang: Lang, r: &SystemRate) -> String {
    let name = System::parse(&r.key).map(|s| s.name(lang)).unwrap_or(lang.pick(["another reading", "മറ്റൊരു വായന", "एक और पढ़त"]));
    let pts = crate::integrity::points(r.points);
    let tail = if r.weighted {
        lang.pick(["it now counts for a little more or less for you.", "ഇപ്പോൾ ഇത് നിങ്ങൾക്ക് അല്പം കൂടുതലോ കുറവോ കണക്കാക്കുന്നു.", "अब यह आपके लिए थोड़ा ज़्यादा या कम गिनी जाती है।"])
    } else {
        lang.pick(["too few yet to change anything.", "എന്തെങ്കിലും മാറ്റാൻ ഇനിയും കുറവ്.", "अभी कुछ बदलने के लिए कम।"])
    };
    match lang {
        Lang::En => format!("{name}: right {pts} of {} for you so far; {tail}", r.votes),
        Lang::Ml => format!("{name}: ഇതുവരെ {}-ൽ {pts} ശരി; {tail}", r.votes),
        Lang::Hi => format!("{name}: अब तक {} में से {pts} सही; {tail}", r.votes),
    }
}

fn points(x: f64) -> String {
    let whole = x.floor();
    if x - whole >= 0.25 {
        format!("{}½", whole as i64)
    } else {
        format!("{}", x.round() as i64)
    }
}

// ---------------------------------------------------------------------------------
// A number only when earned.

/// Fewest answered sealed predictions of a tier before a number is said.
pub const MIN_FOR_NUMBER: usize = 20;

/// The tier of a sealed prediction from the probability it was sealed with.
pub fn tier_of_probability(p: f64) -> &'static str {
    if p >= 0.75 {
        "strong"
    } else if p >= 0.6 {
        "likely"
    } else {
        "possible"
    }
}

/// How sure a firm prediction is, in words, with a number only when earned.
#[derive(Clone, Debug, PartialEq)]
pub struct Confidence {
    /// Whether "about 7 in 10" is said.
    pub number_shown: bool,
    /// The rate behind the number (0 when none is shown).
    pub rate: f64,
    /// Outcomes it rests on.
    pub n: usize,
    /// True when the number comes from the shipped pooled record, not the person's own.
    pub from_everyone: bool,
    /// The line.
    pub words: String,
}

fn tier_words(lang: Lang, tier: &str) -> &'static str {
    lang.pick(match tier {
        "strong" => ["Very likely.", "വളരെ സാധ്യത.", "बहुत संभावना है।"],
        "likely" => ["Likely.", "സാധ്യതയുണ്ട്.", "संभावना है।"],
        _ => ["Possible.", "സാധ്യമാണ്.", "हो सकता है।"],
    })
}

/// The confidence line for a firm prediction of `tier`, given the person's own answered
/// sealed predictions of that tier (`points` over `n`): their rate once `n` reaches
/// [`MIN_FOR_NUMBER`]; else the shipped pooled record when it does, said as such; else
/// words alone, with how many have been checked.
pub fn confidence_words(lang: Lang, tier: &str, points: f64, n: usize) -> Confidence {
    let (gp, gn) = tier_prior(tier);
    let base = tier_words(lang, tier);
    let ten = |r: f64| ((r * 10.0).round() as i64).clamp(0, 10);
    if n >= MIN_FOR_NUMBER {
        let r = (points / n as f64).clamp(0.0, 1.0);
        let k = ten(r);
        let words = match lang {
            Lang::En => format!("{base} For you, about {k} in 10 like this have come ({n} checked)."),
            Lang::Ml => format!("{base} നിങ്ങൾക്ക്, ഇതുപോലുള്ളവയിൽ ഏകദേശം 10-ൽ {k} എണ്ണം വന്നു ({n} എണ്ണം പരിശോധിച്ചു)."),
            Lang::Hi => format!("{base} आपके लिए, ऐसी 10 में से लगभग {k} बातें हुईं ({n} जाँची गईं)।"),
        };
        return Confidence { number_shown: true, rate: r, n, from_everyone: false, words };
    }
    if gn >= MIN_FOR_NUMBER {
        let r = (gp / gn as f64).clamp(0.0, 1.0);
        let k = ten(r);
        let words = match lang {
            Lang::En => format!("{base} Across everyone's checked predictions, about {k} in 10 like this have come; yours are still too few to say."),
            Lang::Ml => format!("{base} എല്ലാവരുടെയും പരിശോധിച്ച പ്രവചനങ്ങളിൽ, ഇതുപോലുള്ളവയിൽ ഏകദേശം 10-ൽ {k} എണ്ണം വന്നു; നിങ്ങളുടേത് പറയാൻ ഇനിയും കുറവാണ്."),
            Lang::Hi => format!("{base} सबकी जाँची गई भविष्यवाणियों में, ऐसी 10 में से लगभग {k} बातें हुईं; आपकी अभी कहने के लिए कम हैं।"),
        };
        return Confidence { number_shown: true, rate: r, n: gn, from_everyone: true, words };
    }
    let words = match lang {
        Lang::En => format!("{base} No number yet: one is shown only after {MIN_FOR_NUMBER} like this have been checked ({n} so far)."),
        Lang::Ml => format!("{base} ഇതുവരെ സംഖ്യയില്ല: ഇതുപോലുള്ള {MIN_FOR_NUMBER} എണ്ണം പരിശോധിച്ചശേഷം മാത്രമേ സംഖ്യ കാണിക്കൂ (ഇതുവരെ {n})."),
        Lang::Hi => format!("{base} अभी कोई संख्या नहीं: ऐसी {MIN_FOR_NUMBER} बातें जाँची जाने के बाद ही संख्या दिखती है (अब तक {n})।"),
    };
    Confidence { number_shown: false, rate: 0.0, n, from_everyone: false, words }
}

// ---------------------------------------------------------------------------------
// Kept weeks against lapsed weeks.

/// Weeks before a remedy that made no difference is gently questioned.
pub const REMEDY_MIN_WEEKS: usize = 8;
/// How much better the kept weeks must go, as a share, to say the remedy may help.
pub const REMEDY_DIFFERENCE: f64 = 0.15;

/// One finished week of a remedy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemedyWeek {
    /// First day (days since 1970-01-01).
    pub start_day: i64,
    /// Kept: done on at least half its due days.
    pub kept: bool,
    /// Good things the person recorded that week.
    pub good: u32,
    /// Hard things.
    pub hard: u32,
}

impl RemedyWeek {
    /// Whether the week has anything recorded at all.
    pub fn has_record(&self) -> bool {
        self.good + self.hard > 0
    }

    /// Whether it went well by the person's own record.
    pub fn went_well(&self) -> bool {
        self.good > self.hard
    }
}

/// The finished weeks of a remedy: seven-day blocks from its first due day, each with
/// at least one due day, ended by `today` (days since 1970-01-01). `signals` are the
/// person's own records, `(day, good)`: the things they logged and the sealed
/// predictions they answered (a good stretch that came, or a hard one that did not, is
/// good).
pub fn remedy_weeks(due_days: &[i64], done_days: &[i64], signals: &[(i64, bool)], today: i64) -> Vec<RemedyWeek> {
    let Some(&first) = due_days.iter().min() else { return Vec::new() };
    let last = *due_days.iter().max().unwrap_or(&first);
    let mut out = Vec::new();
    let mut s = first;
    while s <= last && s + 7 <= today {
        let e = s + 7;
        let due = due_days.iter().filter(|d| (s..e).contains(*d)).count();
        if due > 0 {
            let done = done_days.iter().filter(|d| (s..e).contains(*d) && due_days.contains(d)).count();
            let good = signals.iter().filter(|x| (s..e).contains(&x.0) && x.1).count() as u32;
            let hard = signals.iter().filter(|x| (s..e).contains(&x.0) && !x.1).count() as u32;
            out.push(RemedyWeek { start_day: s, kept: 2 * done >= due, good, hard });
        }
        s = e;
    }
    out
}

/// Where a remedy stands for the person.
#[derive(Clone, Debug, PartialEq)]
pub struct RemedyCompare {
    /// Kept weeks with something recorded.
    pub kept_weeks: usize,
    /// Of those, the ones that went well.
    pub kept_well: usize,
    /// Lapsed weeks with something recorded.
    pub lapsed_weeks: usize,
    /// Of those, the ones that went well.
    pub lapsed_well: usize,
    /// "helps" (kept weeks went clearly better), "no_difference" (after
    /// [`REMEDY_MIN_WEEKS`] weeks), or "too_early".
    pub standing: &'static str,
    /// The plain line.
    pub words: String,
    /// A gentle suggestion to try another from the list, when keeping it made no
    /// difference; empty otherwise.
    pub suggestion: String,
}

/// The kept weeks beside the lapsed weeks, honestly: small numbers said to be small,
/// never proof, and a kind word to try another remedy when keeping it made no difference.
pub fn remedy_compare(lang: Lang, weeks: &[RemedyWeek]) -> RemedyCompare {
    let with: Vec<&RemedyWeek> = weeks.iter().filter(|w| w.has_record()).collect();
    let kept: Vec<&&RemedyWeek> = with.iter().filter(|w| w.kept).collect();
    let lapsed: Vec<&&RemedyWeek> = with.iter().filter(|w| !w.kept).collect();
    let (kw, kg) = (kept.len(), kept.iter().filter(|w| w.went_well()).count());
    let (lw, lg) = (lapsed.len(), lapsed.iter().filter(|w| w.went_well()).count());
    let total = kw + lw;
    let comparable = kw >= 2 && lw >= 2;
    let diff = if comparable { kg as f64 / kw as f64 - lg as f64 / lw as f64 } else { 0.0 };
    let standing = if comparable && diff >= REMEDY_DIFFERENCE {
        "helps"
    } else if comparable && total >= REMEDY_MIN_WEEKS {
        "no_difference"
    } else {
        "too_early"
    };
    let small = if total < REMEDY_MIN_WEEKS {
        lang.pick(["Small numbers, not proof.", "ചെറിയ സംഖ്യകൾ, തെളിവല്ല.", "छोटी संख्याएँ, सबूत नहीं।"])
    } else {
        lang.pick(["Still not proof: many things move a week.", "ഇപ്പോഴും തെളിവല്ല: ഒരാഴ്ചയെ പലതും സ്വാധീനിക്കുന്നു.", "फिर भी सबूत नहीं: एक हफ़्ते पर कई चीज़ों का असर होता है।"])
    };
    let words = if total == 0 {
        lang.pick([
            "No weeks to compare yet. Note what happens, good or hard, and the weeks you kept it will be set beside the weeks you did not.",
            "താരതമ്യം ചെയ്യാൻ ഇതുവരെ ആഴ്ചകളില്ല. നല്ലതോ പ്രയാസമുള്ളതോ ആയി സംഭവിക്കുന്നത് കുറിച്ചുവെക്കൂ; പാലിച്ച ആഴ്ചകളും പാലിക്കാത്ത ആഴ്ചകളും അടുത്തടുത്ത് വെക്കാം.",
            "अभी तुलना के लिए हफ़्ते नहीं हैं। जो होता है, अच्छा या कठिन, लिखते रहिए, फिर जिन हफ़्तों में आपने इसे निभाया उन्हें बाकी हफ़्तों के साथ रखा जाएगा।",
        ])
        .to_string()
    } else if lw == 0 {
        match lang {
            Lang::En => format!("Weeks you kept it: {kg} of {kw} went well by your own record. You kept it every week, so there is nothing yet to compare with. {small}"),
            Lang::Ml => format!("പാലിച്ച ആഴ്ചകൾ: നിങ്ങളുടെ സ്വന്തം കുറിപ്പ് പ്രകാരം {kw}-ൽ {kg} നന്നായി. എല്ലാ ആഴ്ചയും പാലിച്ചതിനാൽ താരതമ്യം ചെയ്യാൻ ഇനിയും ഒന്നുമില്ല. {small}"),
            Lang::Hi => format!("जिन हफ़्तों में निभाया: आपके अपने हिसाब से {kw} में से {kg} अच्छे गए। आपने हर हफ़्ते निभाया, इसलिए अभी तुलना के लिए कुछ नहीं है। {small}"),
        }
    } else if kw == 0 {
        match lang {
            Lang::En => format!("Weeks you did not keep it: {lg} of {lw} went well by your own record. There is no kept week yet to compare with. {small}"),
            Lang::Ml => format!("പാലിക്കാത്ത ആഴ്ചകൾ: നിങ്ങളുടെ സ്വന്തം കുറിപ്പ് പ്രകാരം {lw}-ൽ {lg} നന്നായി. താരതമ്യം ചെയ്യാൻ പാലിച്ച ആഴ്ച ഇനിയും ഇല്ല. {small}"),
            Lang::Hi => format!("जिन हफ़्तों में नहीं निभाया: आपके अपने हिसाब से {lw} में से {lg} अच्छे गए। तुलना के लिए अभी कोई निभाया हुआ हफ़्ता नहीं है। {small}"),
        }
    } else {
        match lang {
            Lang::En => format!("Weeks you kept it: {kg} of {kw} went well by your own record; weeks you didn't: {lg} of {lw}. {small}"),
            Lang::Ml => format!("പാലിച്ച ആഴ്ചകൾ: നിങ്ങളുടെ സ്വന്തം കുറിപ്പ് പ്രകാരം {kw}-ൽ {kg} നന്നായി; പാലിക്കാത്തവ: {lw}-ൽ {lg}. {small}"),
            Lang::Hi => format!("जिन हफ़्तों में निभाया: आपके अपने हिसाब से {kw} में से {kg} अच्छे गए; जिनमें नहीं: {lw} में से {lg}। {small}"),
        }
    };
    let suggestion = if standing == "no_difference" {
        match lang {
            Lang::En => format!("After {total} weeks, the weeks you kept it went no better than the weeks you didn't. It may be worth trying another from the list; keep any that seem to help you."),
            Lang::Ml => format!("{total} ആഴ്ചകൾക്കു ശേഷവും, പാലിച്ച ആഴ്ചകൾ പാലിക്കാത്തവയെക്കാൾ മെച്ചമായില്ല. പട്ടികയിലെ മറ്റൊന്ന് ശ്രമിച്ചുനോക്കാം; സഹായിക്കുന്നതായി തോന്നുന്നവ തുടരൂ."),
            Lang::Hi => format!("{total} हफ़्तों के बाद भी, जिन हफ़्तों में निभाया वे बाकी हफ़्तों से बेहतर नहीं गए। सूची में से कोई और आज़माना ठीक रहेगा; जो मदद करते लगें, उन्हें जारी रखिए।"),
        }
    } else {
        String::new()
    };
    RemedyCompare { kept_weeks: kw, kept_well: kg, lapsed_weeks: lw, lapsed_well: lg, standing, words, suggestion }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;
    use kshana_time::Utc;

    fn at(y: i64, m: u32, d: u32) -> Instant {
        Utc::new(y, m, d, 6, 0, 0, 0).unwrap().to_instant().unwrap()
    }

    #[test]
    fn areas_from_every_feed_map_to_a_part_of_life() {
        assert_eq!(life_of_any("Money"), Some(Life::Money));
        assert_eq!(life_of_any("Wealth"), Some(Life::Money));
        assert_eq!(life_of_any("gains"), Some(Life::Money));
        assert_eq!(life_of_any("Marriage"), Some(Life::Love));
        assert_eq!(life_of_any("abroad"), Some(Life::Travel));
        assert_eq!(life_of_any("child"), Some(Life::Children));
        assert_eq!(life_of_any("nonsense"), None);
    }

    #[test]
    fn the_blind_question_hides_the_claim_and_the_answer_decides_it() {
        let q = blind_question(Lang::En, Life::Money, at(2026, 11, 14), at(2026, 11, 19), 76.27);
        assert_eq!(q, "Did anything happen around money between 14 Nov 2026 and 19 Nov 2026? Good, hard, or nothing much?");
        // The question says nothing about which way the sealed claim leaned.
        assert_eq!(blind_question(Lang::En, Life::Money, at(2026, 11, 14), at(2026, 11, 19), 76.27), q);
        assert_eq!(blind_outcome(true, BlindAnswer::Good), Some("yes"));
        assert_eq!(blind_outcome(true, BlindAnswer::Hard), Some("no"));
        assert_eq!(blind_outcome(true, BlindAnswer::Nothing), Some("no"));
        assert_eq!(blind_outcome(false, BlindAnswer::Hard), Some("yes"));
        assert_eq!(blind_outcome(false, BlindAnswer::Nothing), Some("no"));
        assert_eq!(blind_outcome(false, BlindAnswer::Mixed), Some("close"));
        assert_eq!(blind_outcome(true, BlindAnswer::Unsure), None);
        let r = blind_reveal(Lang::En, at(2026, 10, 21), "Money news is likely.", true, BlindAnswer::Good, 76.27);
        assert!(r.starts_with("This was sealed on 21 Oct 2026: “Money news is likely.” It came as sealed."), "{r}");
        for a in BLIND_ANSWERS {
            assert_eq!(BlindAnswer::parse(a.key()), Some(a));
        }
    }

    #[test]
    fn a_way_of_reading_earns_its_say_only_with_enough_votes_and_never_dominates() {
        // Nine votes, all right: shrunk, but not yet weighted.
        let nine: Vec<(String, f64)> = (0..9).map(|_| ("yogini".to_string(), 1.0)).collect();
        let r = system_rates(&nine);
        assert_eq!(r.len(), 1);
        assert!(!r[0].weighted);
        assert!((r[0].rate - (9.0 + 10.0) / 29.0).abs() < 1e-12);
        assert_eq!(consensus_multipliers(&r), [1.0; NS]);
        // Ten votes: now it moves, within the range.
        let ten: Vec<(String, f64)> = (0..10).map(|_| ("yogini".to_string(), 1.0)).collect();
        let r = system_rates(&ten);
        assert!(r[0].weighted);
        let m = consensus_multipliers(&r);
        let yi = System::Yogini.index();
        assert!(m[yi] > 1.0 && m[yi] <= MULT_RANGE.1, "{m:?}");
        for (i, x) in m.iter().enumerate() {
            if i != yi {
                assert_eq!(*x, 1.0);
            }
        }
        // A way of reading that keeps missing loses say, down to the floor at most.
        let miss: Vec<(String, f64)> = (0..200).map(|_| ("nadi".to_string(), 0.0)).collect();
        let m = consensus_multipliers(&system_rates(&miss));
        assert!((m[System::Nadi.index()] - MULT_RANGE.0).abs() < 1e-12);
        // Nobody is raised above the share cap.
        let all_right: Vec<(String, f64)> = (0..500).map(|_| ("vimshottari".to_string(), 1.0)).collect();
        let m = consensus_multipliers(&system_rates(&all_right));
        let total: f64 = SYSTEMS.iter().enumerate().map(|(i, s)| s.weight() * m[i]).sum();
        let vi = System::Vimshottari.index();
        let base_share = System::Vimshottari.weight() / SYSTEMS.iter().map(|s| s.weight()).sum::<f64>();
        assert!(System::Vimshottari.weight() * m[vi] <= MAX_SHARE.max(base_share) * total + 1e-9, "{m:?}");
        // The shipped prior parses, and an unknown key falls back to even odds.
        assert_eq!(system_prior("vimshottari"), Prior { mean: 0.5, strength: 20.0 });
        assert_eq!(system_prior("something-new"), Prior { mean: 0.5, strength: 20.0 });
        assert_eq!(tier_prior("strong"), (0.0, 0));
    }

    #[test]
    fn a_number_is_said_only_when_earned() {
        let c = confidence_words(Lang::En, "strong", 13.0, 19);
        assert!(!c.number_shown);
        assert!(c.words.starts_with("Very likely. No number yet"), "{}", c.words);
        let c = confidence_words(Lang::En, "strong", 14.0, 20);
        assert!(c.number_shown && !c.from_everyone);
        assert_eq!(c.words, "Very likely. For you, about 7 in 10 like this have come (20 checked).");
        assert_eq!(tier_of_probability(0.8), "strong");
        assert_eq!(tier_of_probability(0.65), "likely");
        assert_eq!(tier_of_probability(0.55), "possible");
    }

    #[test]
    fn kept_weeks_are_set_beside_lapsed_weeks_honestly() {
        // Daily remedy over 10 weeks from day 0; kept the first 6 weeks, lapsed the last 4.
        let due: Vec<i64> = (0..70).collect();
        let done: Vec<i64> = (0..42).collect();
        let mut sig = Vec::new();
        for w in 0..10 {
            sig.push((w * 7 + 1, w < 4 || w == 8));
        }
        let weeks = remedy_weeks(&due, &done, &sig, 100);
        assert_eq!(weeks.len(), 10);
        assert_eq!(weeks.iter().filter(|w| w.kept).count(), 6);
        let c = remedy_compare(Lang::En, &weeks);
        assert_eq!((c.kept_weeks, c.kept_well, c.lapsed_weeks, c.lapsed_well), (6, 4, 4, 1));
        assert_eq!(c.standing, "helps");
        assert_eq!(c.words, "Weeks you kept it: 4 of 6 went well by your own record; weeks you didn't: 1 of 4. Still not proof: many things move a week.");
        assert!(c.suggestion.is_empty());
        // Unfinished weeks are not counted.
        assert_eq!(remedy_weeks(&due, &done, &sig, 20).len(), 2);
        // No difference after enough weeks: a gentle word to try another.
        let mut sig = Vec::new();
        for w in 0..10 {
            sig.push((w * 7 + 1, [0, 1, 5, 6].contains(&w)));
        }
        let done: Vec<i64> = (0..35).collect();
        let c = remedy_compare(Lang::En, &remedy_weeks(&due, &done, &sig, 100));
        assert_eq!((c.kept_weeks, c.kept_well, c.lapsed_weeks, c.lapsed_well), (5, 2, 5, 2));
        assert_eq!(c.standing, "no_difference", "{c:?}");
        assert!(c.suggestion.starts_with("After 10 weeks"));
        // Few weeks: said to be small.
        let c = remedy_compare(Lang::En, &remedy_weeks(&due[..21], &done, &sig, 100));
        assert!(c.words.ends_with("Small numbers, not proof."), "{}", c.words);
        assert_eq!(c.standing, "too_early");
    }

    #[test]
    fn everything_here_speaks_plainly() {
        let mut hits = Vec::new();
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            let mut texts: Vec<String> = Vec::new();
            for life in Life::ALL {
                texts.push(blind_question(lang, life, at(2026, 11, 14), at(2026, 11, 19), 76.27));
                texts.push(blind_question(lang, life, at(2026, 11, 14), at(2027, 3, 19), 76.27));
            }
            for a in BLIND_ANSWERS {
                texts.push(a.label(lang).into());
                for g in [true, false] {
                    texts.push(blind_reveal(lang, at(2026, 10, 21), "Money news is likely.", g, a, 76.27));
                }
            }
            for t in ["strong", "likely", "possible"] {
                for n in [0, 25] {
                    texts.push(confidence_words(lang, t, n as f64 * 0.6, n).words);
                }
            }
            for k in ["vimshottari", "nadi", "unknown"] {
                for n in [3, 12] {
                    texts.push(system_rate_words(lang, &system_rates(&(0..n).map(|_| (k.to_string(), 1.0)).collect::<Vec<_>>())[0]));
                }
            }
            let due: Vec<i64> = (0..70).collect();
            for done in [(0..70).collect::<Vec<i64>>(), (0..35).collect(), vec![]] {
                let sig: Vec<(i64, bool)> = (0..10).map(|w| (w * 7 + 1, w % 2 == 0)).collect();
                let c = remedy_compare(lang, &remedy_weeks(&due, &done, &sig, 100));
                texts.push(c.words);
                texts.push(c.suggestion);
            }
            texts.push(remedy_compare(lang, &[]).words);
            for t in &texts {
                if let Some(w) = jargon_in(lang, t) {
                    hits.push(format!("[{w}] {t}"));
                }
                let low = t.to_lowercase();
                for banned in ["guarantee", "always right", "fate", "destiny", "proven", "cure"] {
                    if low.contains(banned) {
                        hits.push(format!("[promise] {t}"));
                    }
                }
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }
}
