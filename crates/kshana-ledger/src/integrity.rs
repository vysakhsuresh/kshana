//! The record kept honest: what the person logged, matched to what was sealed only
//! *after* they wrote it, and the short record shown on the About page.
//!
//! **Blind logging.** The person writes what happened (the part of life, the day, good
//! or hard, a line) without seeing any prediction. Only after it is saved does
//! [`match_logged`] look for sealed claims it bears on: the same part of life (whatever
//! the feed calls it, see [`kshana_rules::integrity::life_of_any`]), sealed before that
//! day, with the day inside the claim's window widened by [`BLIND_GRACE_DAYS`] on each
//! side. A match in the claim's own direction `agrees`; one the other way does not. A
//! claim sealed after the day is never matched: it could not have foreseen it.

use crate::claim::SealedClaim;
use crate::scoreboard::{Scoreboard, MIN_TO_JUDGE};
use kshana_rules::integrity::life_of_any;
use kshana_rules::Lang;
use kshana_time::{Instant, NS_PER_DAY};

/// Days on each side of a sealed window within which a logged event still bears on it.
pub const BLIND_GRACE_DAYS: i64 = 3;

/// A sealed claim a logged event bears on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlindMatch {
    /// Index into the claims passed.
    pub index: usize,
    /// The event went the way the claim said.
    pub agrees: bool,
    /// The day falls inside the window itself, not only in the grace days.
    pub inside: bool,
}

/// The sealed claims a logged event (part of life `area`, `good` or hard, on `at`)
/// bears on, with `grace_days` either side of each window: agreeing ones first, then
/// those it falls inside, then by window start.
pub fn match_logged(claims: &[SealedClaim], area: &str, good: bool, at: Instant, grace_days: i64) -> Vec<BlindMatch> {
    let Some(life) = life_of_any(area) else { return Vec::new() };
    let grace = grace_days.max(0) as i128 * NS_PER_DAY;
    let mut out: Vec<BlindMatch> = claims
        .iter()
        .enumerate()
        .filter(|(_, c)| life_of_any(&c.area) == Some(life))
        .filter(|(_, c)| c.created_at <= at)
        .filter(|(_, c)| c.window_start.add_ns(-grace) <= at && at <= c.window_end.add_ns(grace))
        .map(|(i, c)| BlindMatch { index: i, agrees: c.good == good, inside: c.window_start <= at && at <= c.window_end })
        .collect();
    out.sort_by_key(|m| (!m.agrees, !m.inside, claims[m.index].window_start));
    out
}

/// The short record for the About page.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordCard {
    /// "For you, Kshana has been right 7 times out of 9." or "Too early to tell: 2 sealed
    /// predictions resolved so far."
    pub headline: String,
    /// The chance line and how many are sealed and waiting.
    pub detail: String,
    /// Fewer than [`MIN_TO_JUDGE`] answered: no verdict is given.
    pub early: bool,
    /// Sealed in all.
    pub sealed: usize,
    /// Answered.
    pub resolved: usize,
}

/// The record card from the scoreboard (whose headline already counts only sealed
/// forward predictions), the number sealed in all and the number still waiting.
pub fn record_card(lang: Lang, board: &Scoreboard, sealed: usize, waiting: usize) -> RecordCard {
    let resolved = board.total;
    let early = resolved < MIN_TO_JUDGE;
    let headline = if early {
        match (lang, resolved) {
            (Lang::En, 1) => "Too early to tell: 1 sealed prediction resolved so far.".to_string(),
            (Lang::En, n) => format!("Too early to tell: {n} sealed predictions resolved so far."),
            (Lang::Ml, n) => format!("പറയാൻ ഇനിയും നേരത്തെയാണ്: ഇതുവരെ {n} മുദ്രവെച്ച പ്രവചനങ്ങൾക്ക് ഉത്തരം കിട്ടി."),
            (Lang::Hi, n) => format!("अभी कहना जल्दी है: अब तक {n} सील की गई भविष्यवाणियों का जवाब मिला है।"),
        }
    } else {
        board.headline.clone()
    };
    let counts = match lang {
        Lang::En => format!("{sealed} sealed in all, {waiting} still to come. Only predictions sealed before their dates count, misses as plainly as hits."),
        Lang::Ml => format!("ആകെ {sealed} മുദ്രവെച്ചു, {waiting} ഇനി വരാനുണ്ട്. തീയതികൾക്ക് മുമ്പ് മുദ്രവെച്ചവ മാത്രം എണ്ണുന്നു; ശരികൾ പോലെ തന്നെ തെറ്റുകളും."),
        Lang::Hi => format!("कुल {sealed} सील की गईं, {waiting} अभी आनी हैं। सिर्फ़ तारीखों से पहले सील की गई बातें गिनी जाती हैं, चूकें भी उतनी ही साफ़ जितनी सही बातें।"),
    };
    let detail = if early || board.chance_line.is_empty() { counts } else { format!("{} {counts}", board.chance_line) };
    RecordCard { headline, detail, early, sealed, resolved }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::{seal_claim, ClaimInput};
    use crate::scoreboard::{scoreboard, ScoreItem, ScoreSource};
    use kshana_rules::plain::jargon_in;
    use kshana_time::Utc;

    fn day(y: i64, m: u32, d: u32) -> Instant {
        Utc::new(y, m, d, 6, 0, 0, 0).unwrap().to_instant().unwrap()
    }

    fn claim(area: &str, good: bool, from: Instant, to: Instant, sealed: Instant) -> SealedClaim {
        seal_claim(
            &ClaimInput {
                source: "test".into(),
                area: area.into(),
                kind: "opening".into(),
                good,
                window_start: from,
                window_end: to,
                probability: 0.7,
                statement: "Money news is likely.".into(),
                question: "Did it come?".into(),
            },
            sealed,
        )
        .unwrap()
    }

    #[test]
    fn a_logged_event_matches_only_claims_sealed_before_it_in_the_same_part_of_life() {
        let sealed = day(2026, 10, 1);
        let claims = vec![
            claim("Money", true, day(2026, 11, 14), day(2026, 11, 19), sealed),
            claim("Career", true, day(2026, 11, 14), day(2026, 11, 19), sealed),
            claim("Money", false, day(2026, 11, 10), day(2026, 11, 30), sealed),
            // Sealed after the event: never matched.
            claim("Money", true, day(2026, 12, 1), day(2026, 12, 5), day(2026, 11, 20)),
        ];
        // Inside the first window, good: it agrees with the first claim; the hard one
        // over the same days is matched as not agreeing.
        let m = match_logged(&claims, "Money", true, day(2026, 11, 16), BLIND_GRACE_DAYS);
        assert_eq!(m, vec![BlindMatch { index: 0, agrees: true, inside: true }, BlindMatch { index: 2, agrees: false, inside: true }]);
        // The ledger's own area words match too.
        assert_eq!(match_logged(&claims, "Wealth", true, day(2026, 11, 16), BLIND_GRACE_DAYS).len(), 2);
        // Within the grace days, but outside the window.
        let m = match_logged(&claims, "Money", true, day(2026, 11, 21), BLIND_GRACE_DAYS);
        assert_eq!(m[0], BlindMatch { index: 0, agrees: true, inside: false });
        // Beyond the grace days: nothing from the first claim.
        assert!(match_logged(&claims, "Money", true, day(2026, 11, 23), BLIND_GRACE_DAYS).iter().all(|x| x.index != 0));
        // The claim sealed later is never matched, even inside its window.
        assert!(match_logged(&claims, "Money", true, day(2026, 11, 18), 20).iter().all(|x| x.index != 3));
        assert!(match_logged(&claims, "Money", true, day(2026, 12, 2), BLIND_GRACE_DAYS).iter().any(|x| x.index == 3));
        // An unknown area matches nothing.
        assert!(match_logged(&claims, "Weather", true, day(2026, 11, 16), BLIND_GRACE_DAYS).is_empty());
    }

    #[test]
    fn the_record_card_waits_for_five_answers_before_any_verdict() {
        let two: Vec<ScoreItem> = (0..2).map(|_| ScoreItem { source: ScoreSource::Sealed, points: 1.0, chance: 0.5, used_for_fitting: false }).collect();
        let c = record_card(Lang::En, &scoreboard(&two, Lang::En), 6, 4);
        assert!(c.early);
        assert_eq!(c.headline, "Too early to tell: 2 sealed predictions resolved so far.");
        assert!(c.detail.starts_with("6 sealed in all, 4 still to come."));
        // Answers about the past never make the record look further along.
        let mut more = two.clone();
        for _ in 0..30 {
            more.push(ScoreItem { source: ScoreSource::Past, points: 1.0, chance: 0.5, used_for_fitting: true });
        }
        assert_eq!(record_card(Lang::En, &scoreboard(&more, Lang::En), 6, 4).headline, c.headline);
        let seven: Vec<ScoreItem> = (0..7).map(|i| ScoreItem { source: ScoreSource::Sealed, points: (i % 2) as f64, chance: 0.5, used_for_fitting: false }).collect();
        let c = record_card(Lang::En, &scoreboard(&seven, Lang::En), 9, 2);
        assert!(!c.early);
        assert_eq!(c.headline, "For you, Kshana has been right 3 times out of 7.");
        assert!(c.detail.contains("no better than guessing"), "{}", c.detail);
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for items in [&two, &seven, &vec![]] {
                let c = record_card(lang, &scoreboard(items, lang), 1, 0);
                for t in [&c.headline, &c.detail] {
                    assert_eq!(jargon_in(lang, t), None, "{t}");
                }
            }
        }
    }
}
