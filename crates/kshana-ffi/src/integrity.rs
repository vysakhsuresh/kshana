//! The record kept honest, over the FFI: blind answers to sealed predictions, logged
//! events matched to what was sealed only after they are saved, each way of reading's
//! own hit rate (and its say in the agreement vote), a confidence number only when
//! earned, kept weeks set beside lapsed weeks for a remedy, and the short record for the
//! About page.
//!
//! What the person reads: every `words`, `question`, `label`, `reveal`, `headline`,
//! `detail`, `suggestion`, `how_it_works` and `statement`. Keys (`key`, `id`, `line`,
//! `outcome`, `tier`, `standing`) are for the app (see `plain_audit` below).
//!
//! The hold-out rule holds throughout: only sealed predictions, answered after their
//! dates, feed the score, the hit rates and the confidence numbers. Nothing here reads
//! the answers about the past that tune the birth time.

use kshana_ledger::{Outcome, SealedClaim};
use kshana_rules::integrity::{self as ig, BlindAnswer};
use kshana_rules::Lang;
use kshana_sky::Observer;

use crate::consensus::{window_record, SureWindowRecord};
use crate::trust::{claim_record, score_items, ClaimRecord, ScoreItemInput};
use crate::{AyanamsaChoice, Engine, ExactInstant, HouseSystem, KshanaError};

fn err<E: std::fmt::Display>(e: E) -> KshanaError {
    KshanaError::Engine(e.to_string())
}

// ---------------------------------------------------------------------------------
// Blind answers.

/// One answer button.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BlindAnswerRecord {
    /// "good", "hard", "mixed", "nothing" or "unsure".
    pub key: String,
    /// The button's words.
    pub label: String,
}

/// The answer buttons for a blind question, in order.
#[uniffi::export]
pub fn blind_answers(lang: String) -> Vec<BlindAnswerRecord> {
    let lang = Lang::parse(&lang);
    ig::BLIND_ANSWERS.iter().map(|a| BlindAnswerRecord { key: a.key().into(), label: a.label(lang).into() }).collect()
}

/// The part of life's key ("Money") for any feed's area word ("Wealth", "gains",
/// "Marriage", "abroad", …); none when it names no part of life.
#[uniffi::export]
pub fn life_key_of(area: String) -> Option<String> {
    ig::life_of_any(&area).map(|l| l.name().to_string())
}

/// The question asked before a sealed claim's words are shown: "Did anything happen
/// around money between 14 Nov and 19 Nov? Good, hard, or nothing much?"; empty for an
/// unknown part of life.
#[uniffi::export]
pub fn blind_question(lang: String, area: String, from: ExactInstant, to: ExactInstant, lon: f64) -> String {
    ig::life_of_any(&area).map(|life| ig::blind_question(Lang::parse(&lang), life, from.into(), to.into(), lon)).unwrap_or_default()
}

/// A blind answer applied to a stored claim.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BlindResolveRecord {
    /// The answered claim, whose `line` replaces the stored one; none for "not sure".
    pub claim: Option<ClaimRecord>,
    /// "yes", "close", "no", or "unsure".
    pub outcome: String,
    /// What is shown now: "This was sealed on 21 Oct 2026: “…”" and how it turned out.
    pub reveal: String,
}

/// Answer a stored claim blind: the person's `answer` ("good", "hard", "mixed",
/// "nothing", "unsure") to [`blind_question`] decides Yes / Close / No against what was
/// sealed, and only then are the sealed words revealed.
#[uniffi::export]
pub fn blind_resolve(line: String, answer: String, when: ExactInstant, lang: String, lon: f64) -> Result<BlindResolveRecord, KshanaError> {
    let c = SealedClaim::from_json_line(&line).map_err(err)?;
    let a = BlindAnswer::parse(&answer).ok_or_else(|| KshanaError::Engine(format!("unknown answer {answer}")))?;
    let lang = Lang::parse(&lang);
    let reveal = ig::blind_reveal(lang, c.created_at, &c.statement, c.good, a, lon);
    let outcome = ig::blind_outcome(c.good, a);
    let claim = match outcome.and_then(Outcome::parse) {
        Some(o) => Some(claim_record(&kshana_ledger::resolve_claim(&c, o, when.into()))),
        None => None,
    };
    Ok(BlindResolveRecord { claim, outcome: outcome.unwrap_or("unsure").into(), reveal })
}

/// What a blind answer makes of any sealed prediction said to be `good` or hard,
/// sealed at `sealed_at` with `statement`: the outcome key ("yes", "close", "no" or
/// "unsure") and the words shown once it is revealed. For seals that are not stored as
/// claim lines (the stretch sealed on the past-check page).
#[uniffi::export]
pub fn blind_judge(good: bool, answer: String, sealed_at: ExactInstant, statement: String, lang: String, lon: f64) -> Result<BlindResolveRecord, KshanaError> {
    let a = BlindAnswer::parse(&answer).ok_or_else(|| KshanaError::Engine(format!("unknown answer {answer}")))?;
    let reveal = ig::blind_reveal(Lang::parse(&lang), sealed_at.into(), &statement, good, a, lon);
    Ok(BlindResolveRecord { claim: None, outcome: ig::blind_outcome(good, a).unwrap_or("unsure").into(), reveal })
}

/// A sealed claim a logged event bears on.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BlindMatchRecord {
    /// The claim.
    pub claim: ClaimRecord,
    /// The event went the way the claim said.
    pub agrees: bool,
    /// The day falls inside the window itself, not only in the grace days.
    pub inside: bool,
    /// "This was sealed on 21 Oct 2026: “…”", and whether its dates are still running.
    pub words: String,
}

/// After a logged event is saved (part of life `area`, `good` or hard, on `at`), the
/// stored claims (`lines`) it bears on: the same part of life, sealed before that day,
/// the day within the window or [`kshana_ledger::BLIND_GRACE_DAYS`] of it. Agreeing
/// ones first. Unreadable lines are skipped.
#[uniffi::export]
pub fn match_logged_event(lines: Vec<String>, area: String, good: bool, at: ExactInstant, now: ExactInstant, lang: String, lon: f64) -> Vec<BlindMatchRecord> {
    let claims: Vec<SealedClaim> = lines.iter().filter_map(|l| SealedClaim::from_json_line(l).ok()).collect();
    let lang = Lang::parse(&lang);
    let now: kshana_time::Instant = now.into();
    kshana_ledger::match_logged(&claims, &area, good, at.into(), kshana_ledger::BLIND_GRACE_DAYS)
        .into_iter()
        .map(|m| {
            let c = &claims[m.index];
            let d = kshana_rules::lang::date(lang, c.created_at, lon);
            let sealed = match lang {
                Lang::En => format!("This was sealed on {d}: “{}”", c.statement),
                Lang::Ml => format!("ഇത് {d}-ന് മുദ്രവെച്ചതാണ്: “{}”", c.statement),
                Lang::Hi => format!("यह {d} को सील किया गया था: “{}”", c.statement),
            };
            let tail = if c.resolved.is_some() {
                lang.pick(["It has already been answered.", "ഇതിന് ഇതിനകം ഉത്തരം നൽകി.", "इसका जवाब पहले ही दिया जा चुका है।"])
            } else if c.window_end > now {
                lang.pick(["Its dates are still running; you will be asked once they pass.", "ഇതിന്റെ തീയതികൾ ഇപ്പോഴും തുടരുന്നു; അവ കഴിഞ്ഞാൽ ചോദിക്കും.", "इसकी तारीखें अभी चल रही हैं; बीतने पर पूछा जाएगा।"])
            } else if m.agrees {
                lang.pick(["What you wrote went the way it said.", "നിങ്ങൾ എഴുതിയത് പറഞ്ഞതുപോലെ പോയി.", "आपने जो लिखा वह कहे अनुसार गया।"])
            } else {
                lang.pick(["What you wrote went the other way.", "നിങ്ങൾ എഴുതിയത് മറുവഴിക്ക് പോയി.", "आपने जो लिखा वह दूसरी तरफ़ गया।"])
            };
            BlindMatchRecord { claim: claim_record(c), agrees: m.agrees, inside: m.inside, words: format!("{sealed} {tail}") }
        })
        .collect()
}

// ---------------------------------------------------------------------------------
// Each way of reading earns its own hit rate.

/// One resolved sealed prediction a way of reading voted for.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SystemVoteInput {
    /// The way of reading's key ("vimshottari"; a `system_keys` entry of a window).
    pub key: String,
    /// The prediction's answer: "yes", "close" or "no" (anything else is left out).
    pub outcome: String,
}

/// One way of reading's own record for the person.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SystemRateRecord {
    /// Its key (expert view only).
    pub key: String,
    /// Resolved sealed predictions it voted for.
    pub votes: u32,
    /// Points on them.
    pub points: f64,
    /// The rate shrunk toward the shipped prior.
    pub rate: f64,
    /// Whether it has enough votes to move the agreement vote.
    pub weighted: bool,
    /// Its multiplier in the agreement vote (1 until weighted).
    pub multiplier: f64,
    /// "the long-period reading: right 8 of 12 for you so far; …"
    pub words: String,
}

fn votes_of(votes: &[SystemVoteInput]) -> Vec<(String, f64)> {
    votes.iter().filter_map(|v| Some((v.key.clone(), Outcome::parse(&v.outcome)?.points()))).collect()
}

/// Every way of reading's own hit rate from the votes, with its say in the agreement
/// vote.
#[uniffi::export]
pub fn system_hit_rates(votes: Vec<SystemVoteInput>, lang: String) -> Vec<SystemRateRecord> {
    let lang = Lang::parse(&lang);
    let rates = ig::system_rates(&votes_of(&votes));
    let m = ig::consensus_multipliers(&rates);
    rates
        .iter()
        .map(|r| SystemRateRecord {
            key: r.key.clone(),
            votes: r.votes as u32,
            points: r.points,
            rate: r.rate,
            weighted: r.weighted,
            multiplier: kshana_rules::consensus::System::parse(&r.key).map(|s| m[s.index()]).unwrap_or(1.0),
            words: ig::system_rate_words(lang, r),
        })
        .collect()
}

#[uniffi::export]
impl Engine {
    /// `sure_windows`, with each way of reading's say adjusted by its own record for the
    /// person (`votes`: one per resolved sealed prediction it voted for). Identical to
    /// `sure_windows` until a way of reading has ten votes.
    #[allow(clippy::too_many_arguments)]
    pub fn sure_windows_trusted(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        here_lat: f64,
        here_lon: f64,
        utc_offset_minutes: i32,
        from: ExactInstant,
        months: u32,
        areas: Vec<String>,
        max: u32,
        lang: String,
        votes: Vec<SystemVoteInput>,
    ) -> Result<Vec<SureWindowRecord>, KshanaError> {
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let obs = Observer { latitude_deg: birth_lat, longitude_deg: birth_lon, height_m: 0.0 };
        let here = Observer { latitude_deg: here_lat, longitude_deg: here_lon, height_m: 0.0 };
        let lang = Lang::parse(&lang);
        let mult = ig::consensus_multipliers(&ig::system_rates(&votes_of(&votes)));
        let lives: Vec<kshana_rules::Life> = areas.iter().filter_map(|a| kshana_rules::Life::parse(a)).collect();
        let w = kshana_rules::consensus::sure_windows_trusted(
            &self.spk,
            self.eop.as_ref(),
            &natal,
            &obs,
            &here,
            ayanamsa.into(),
            lang,
            utc_offset_minutes.clamp(-14 * 60, 14 * 60),
            from.into(),
            months,
            &lives,
            max.clamp(1, 200) as usize,
            &mult,
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        Ok(w.iter().map(|x| window_record(x, lang)).collect())
    }
}

// ---------------------------------------------------------------------------------
// A number only when earned.

/// How sure a firm prediction is.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ConfidenceRecord {
    /// Whether "about 7 in 10" is said.
    pub number_shown: bool,
    /// The rate behind it (0 when none is shown).
    pub rate: f64,
    /// Outcomes it rests on.
    pub n: u32,
    /// The number is the shipped pooled record's, not the person's own.
    pub from_everyone: bool,
    /// The line.
    pub words: String,
}

/// The confidence line for a firm prediction of `tier` ("strong", "likely",
/// "possible"), given the person's answered sealed predictions of that tier (`points`
/// over `n`, Yes 1, Close ½): a number only from 20 answered, else words.
#[uniffi::export]
pub fn confidence_line(tier: String, points: f64, n: u32, lang: String) -> ConfidenceRecord {
    let c = ig::confidence_words(Lang::parse(&lang), &tier, points, n as usize);
    ConfidenceRecord { number_shown: c.number_shown, rate: c.rate, n: c.n as u32, from_everyone: c.from_everyone, words: c.words }
}

/// The tier of a sealed prediction from the probability it was sealed with.
#[uniffi::export]
pub fn tier_of_probability(probability: f64) -> String {
    ig::tier_of_probability(probability).into()
}

// ---------------------------------------------------------------------------------
// Kept weeks against lapsed weeks.

/// One thing the person recorded on a day: a logged event, or a sealed prediction they
/// answered (a good stretch that came, or a hard one that did not, is good).
#[derive(Clone, Debug, uniffi::Record)]
pub struct DaySignalInput {
    /// Days since 1970-01-01.
    pub day: i64,
    /// Good or hard.
    pub good: bool,
}

/// A remedy's kept weeks beside its lapsed weeks.
#[derive(Clone, Debug, uniffi::Record)]
pub struct RemedyCompareRecord {
    /// Finished weeks.
    pub weeks: u32,
    /// Kept weeks with something recorded.
    pub kept_weeks: u32,
    /// Of those, the ones that went well.
    pub kept_well: u32,
    /// Lapsed weeks with something recorded.
    pub lapsed_weeks: u32,
    /// Of those, the ones that went well.
    pub lapsed_well: u32,
    /// "helps", "no_difference" or "too_early".
    pub standing: String,
    /// "Weeks you kept it: 4 of 6 went well by your own record; weeks you didn't: 1 of 3. Small numbers, not proof."
    pub words: String,
    /// A gentle word to try another remedy, after eight weeks of no difference; else empty.
    pub suggestion: String,
}

/// Set a remedy's kept weeks beside its lapsed weeks: `due_days` and `done_days` (days
/// since 1970-01-01) from its practice ticks, `signals` the person's own record, `today`
/// the current day.
#[uniffi::export]
pub fn remedy_kept_compare(due_days: Vec<i64>, done_days: Vec<i64>, signals: Vec<DaySignalInput>, today: i64, lang: String) -> RemedyCompareRecord {
    let sig: Vec<(i64, bool)> = signals.iter().map(|s| (s.day, s.good)).collect();
    let weeks = ig::remedy_weeks(&due_days, &done_days, &sig, today);
    let c = ig::remedy_compare(Lang::parse(&lang), &weeks);
    RemedyCompareRecord {
        weeks: weeks.len() as u32,
        kept_weeks: c.kept_weeks as u32,
        kept_well: c.kept_well as u32,
        lapsed_weeks: c.lapsed_weeks as u32,
        lapsed_well: c.lapsed_well as u32,
        standing: c.standing.into(),
        words: c.words,
        suggestion: c.suggestion,
    }
}

// ---------------------------------------------------------------------------------
// The record for the About page.

/// Kshana's record for one person.
#[derive(Clone, Debug, uniffi::Record)]
pub struct RecordCardRecord {
    /// "For you, Kshana has been right 7 times out of 9." or "Too early to tell: 2
    /// sealed predictions resolved so far."
    pub headline: String,
    /// The chance line and how many are sealed and waiting.
    pub detail: String,
    /// Fewer than five answered: no verdict.
    pub early: bool,
    /// Sealed in all.
    pub sealed: u32,
    /// Answered (counted in the score).
    pub resolved: u32,
    /// Better than guessing, with enough answers.
    pub beats_chance: bool,
}

/// The record card: the headline score (sealed predictions only) against chance, how
/// many are sealed and still to come.
#[uniffi::export]
pub fn record_card(items: Vec<ScoreItemInput>, sealed: u32, waiting: u32, lang: String) -> RecordCardRecord {
    let lang = Lang::parse(&lang);
    let b = kshana_ledger::scoreboard(&score_items(&items), lang);
    let c = kshana_ledger::record_card(lang, &b, sealed as usize, waiting as usize);
    RecordCardRecord { headline: c.headline, detail: c.detail, early: c.early, sealed: c.sealed as u32, resolved: c.resolved as u32, beats_chance: b.beats_chance }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::trust::{seal_claim, ClaimInputRecord};
    use crate::{instant_from_utc, UtcFields};

    fn at(y: i64, mo: u32, d: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: 5, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn check(lang: Lang, what: &str, v: &str, hits: &mut Vec<String>) {
        if let Some(w) = kshana_rules::plain::jargon_in(lang, v) {
            hits.push(format!("{what} [{w}]: {v}"));
        }
        for banned in ["guarantee", "always right", "fate", "destiny"] {
            if v.to_lowercase().contains(banned) {
                hits.push(format!("{what} [promise]: {v}"));
            }
        }
    }

    #[test]
    fn the_record_speaks_plainly_and_answers_blind() {
        let now = at(2026, 10, 21);
        let mut hits = Vec::new();
        for lang in ["en", "ml", "hi"] {
            let l = Lang::parse(lang);
            let feed = |good: bool| ClaimInputRecord {
                source: "consensus".into(),
                area: "Money".into(),
                kind: "opening".into(),
                good,
                window_start: at(2026, 11, 14),
                window_end: at(2026, 11, 19),
                probability: 0.8,
                statement: "Money news is likely.".into(),
                question: String::new(),
            };
            let good = seal_claim(feed(true), now, lang.into(), 76.27).unwrap();
            let hard = seal_claim(feed(false), now, lang.into(), 76.27).unwrap();
            for b in blind_answers(lang.into()) {
                check(l, "label", &b.label, &mut hits);
                for c in [&good, &hard] {
                    let r = blind_resolve(c.line.clone(), b.key.clone(), at(2026, 11, 20), lang.into(), 76.27).unwrap();
                    check(l, "reveal", &r.reveal, &mut hits);
                    assert_eq!(r.claim.is_some(), b.key != "unsure");
                    if let Some(x) = &r.claim {
                        assert!(x.verified && x.id == c.id && x.outcome == r.outcome);
                    }
                }
            }
            let q = blind_question(lang.into(), "Money".into(), good.window_start, good.window_end, 76.27);
            check(l, "question", &q, &mut hits);
            assert!(!q.is_empty() && !q.contains("Money news"));
            // Matching after a logged event.
            let m = match_logged_event(vec![good.line.clone(), hard.line.clone()], "Money".into(), true, at(2026, 11, 16), now, lang.into(), 76.27);
            assert_eq!(m.len(), 2);
            assert!(m[0].agrees && m[0].claim.id == good.id && !m[1].agrees);
            for x in &m {
                check(l, "match", &x.words, &mut hits);
            }
            let m = match_logged_event(vec![good.line.clone()], "Money".into(), true, at(2026, 11, 16), at(2026, 12, 1), lang.into(), 76.27);
            check(l, "match", &m[0].words, &mut hits);
            // Hit rates, confidence and remedies.
            let votes: Vec<SystemVoteInput> = (0..12).map(|i| SystemVoteInput { key: if i % 2 == 0 { "yogini".into() } else { "nadi".into() }, outcome: "yes".into() }).collect();
            for r in system_hit_rates(votes, lang.into()) {
                check(l, "rate", &r.words, &mut hits);
            }
            for t in ["strong", "likely", "possible"] {
                for n in [3u32, 30] {
                    let c = confidence_line(t.into(), n as f64 * 0.7, n, lang.into());
                    assert_eq!(c.number_shown, n >= 20);
                    check(l, "confidence", &c.words, &mut hits);
                }
            }
            let due: Vec<i64> = (0..70).collect();
            let sig: Vec<DaySignalInput> = (0..10).map(|w| DaySignalInput { day: w * 7 + 2, good: w < 5 }).collect();
            let r = remedy_kept_compare(due, (0..35).collect(), sig, 90, lang.into());
            assert_eq!(r.weeks, 10);
            check(l, "remedy", &r.words, &mut hits);
            check(l, "remedy", &r.suggestion, &mut hits);
            // The record card.
            let items: Vec<ScoreItemInput> = (0..2).map(|_| ScoreItemInput { source: "sealed".into(), outcome: "yes".into(), chance: 0.5, used_for_fitting: false }).collect();
            let mut with_past = items.clone();
            for _ in 0..20 {
                with_past.push(ScoreItemInput { source: "past".into(), outcome: "yes".into(), chance: 0.5, used_for_fitting: false });
            }
            let c = record_card(with_past, 5, 3, lang.into());
            assert!(c.early && c.resolved == 2, "past answers never make the record look further along");
            check(l, "record", &c.headline, &mut hits);
            check(l, "record", &c.detail, &mut hits);
            if lang == "en" {
                assert_eq!(c.headline, "Too early to tell: 2 sealed predictions resolved so far.");
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }

    #[test]
    fn trusted_windows_match_the_usual_ones_until_a_way_of_reading_has_ten_votes() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let birth = at(1990, 2, 20);
        let (la, lo) = (9.9312, 76.2673);
        let now = at(2026, 10, 21);
        let usual = engine.sure_windows(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, la, lo, 330, now, 6, vec!["Money".into()], 10, "en".into()).unwrap();
        let few: Vec<SystemVoteInput> = (0..9).map(|_| SystemVoteInput { key: "vimshottari".into(), outcome: "no".into() }).collect();
        let same = engine.sure_windows_trusted(birth, la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, la, lo, 330, now, 6, vec!["Money".into()], 10, "en".into(), few).unwrap();
        assert_eq!(format!("{usual:?}"), format!("{same:?}"));
    }
}
