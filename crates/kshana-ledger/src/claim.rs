//! Sealed claims: any dated prediction the app makes, written down and fingerprinted
//! before its window opens, then answered Yes, Close or No once the window has closed.
//!
//! [`crate::SealedForecast`] seals one of the ledger's eight areas and six event kinds
//! with the probability computed by the ledger's own fit. A [`SealedClaim`] is the
//! general form for everything else the app says about a dated window: the forecast's
//! openings and pressures, the stretch of the coming weeks, a window several readings
//! agree on. The caller supplies `{area, kind, good, window, probability, statement,
//! question}`; the claim's `id` is the SHA-256 of the canonical JSON of all of it and of
//! the instant it was sealed, so [`SealedClaim::verify`] exposes any later edit, and a
//! claim whose window has already opened is refused. Its `key` is the SHA-256 of what
//! is claimed (area, kind, good, the window's days) without the instant or the
//! probability, so the same prediction is never sealed twice ([`select_new`]).
//!
//! A resolution (Yes = right, Close = half right, No = wrong) is appended later and is
//! not part of the id.

use crate::event::{iso_utc, parse_iso_utc, LedgerError};
use crate::hash::sha256_hex;
use crate::json::{self, Value};
use kshana_time::{Instant, NS_PER_DAY};

/// How a sealed claim turned out, by the person's own answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It came as said.
    Yes,
    /// Something like it, or near that time.
    Close,
    /// It did not come.
    No,
}

impl Outcome {
    /// Stable key: "yes", "close" or "no".
    pub fn key(self) -> &'static str {
        match self {
            Outcome::Yes => "yes",
            Outcome::Close => "close",
            Outcome::No => "no",
        }
    }

    /// From a key; anything else (a "not sure") is none.
    pub fn parse(s: &str) -> Option<Outcome> {
        match s.trim().to_ascii_lowercase().as_str() {
            "yes" => Some(Outcome::Yes),
            "close" => Some(Outcome::Close),
            "no" => Some(Outcome::No),
            _ => None,
        }
    }

    /// Points on the scoreboard: 1, ½ or 0.
    pub fn points(self) -> f64 {
        match self {
            Outcome::Yes => 1.0,
            Outcome::Close => 0.5,
            Outcome::No => 0.0,
        }
    }
}

/// A dated prediction, sealed.
#[derive(Clone, Debug, PartialEq)]
pub struct SealedClaim {
    /// SHA-256 (hex) of the canonical JSON of every field below but `resolved`.
    pub id: String,
    /// SHA-256 (hex) of what is claimed, without when it was sealed: the same
    /// prediction sealed twice has the same key.
    pub key: String,
    /// When it was sealed.
    pub created_at: Instant,
    /// What produced it: "forecast", "weeks", "consensus", …
    pub source: String,
    /// The part of life it is about (the caller's key, e.g. "Money").
    pub area: String,
    /// What kind of claim ("opening", "pressure", …).
    pub kind: String,
    /// True if the stretch was said to be good.
    pub good: bool,
    /// Start of the window, after `created_at`.
    pub window_start: Instant,
    /// End of the window.
    pub window_end: Instant,
    /// The probability the app gave that it comes as said, `[0, 1]`.
    pub probability: f64,
    /// The prediction in the person's words.
    pub statement: String,
    /// The yes/no question that resolves it.
    pub question: String,
    /// `(outcome, when answered)` once answered.
    pub resolved: Option<(Outcome, Instant)>,
}

/// The day of an instant (UTC), the unit the key compares windows in.
fn day_of(t: Instant) -> String {
    iso_utc(t).chars().take(10).collect()
}

/// The key of a claim: what is claimed, without when.
pub fn claim_key(area: &str, kind: &str, good: bool, start: Instant, end: Instant) -> String {
    sha256_hex(&format!("kshana-claim-key-v1|{}|{}|{}|{}|{}", area.trim().to_ascii_lowercase(), kind.trim().to_ascii_lowercase(), good, day_of(start), day_of(end)))
}

impl SealedClaim {
    fn canonical_fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("key", Value::Str(self.key.clone())),
            ("created_at", Value::Str(iso_utc(self.created_at))),
            ("source", Value::Str(self.source.clone())),
            ("area", Value::Str(self.area.clone())),
            ("kind", Value::Str(self.kind.clone())),
            ("good", Value::Bool(self.good)),
            ("window_start", Value::Str(iso_utc(self.window_start))),
            ("window_end", Value::Str(iso_utc(self.window_end))),
            ("probability", Value::Num(self.probability)),
            ("statement", Value::Str(self.statement.clone())),
            ("question", Value::Str(self.question.clone())),
        ]
    }

    /// The id the content deserves.
    pub fn content_id(&self) -> String {
        sha256_hex(&json::write_object(&self.canonical_fields()))
    }

    /// Whether the id and the key still match the content.
    pub fn verify(&self) -> bool {
        self.id == self.content_id() && self.key == claim_key(&self.area, &self.kind, self.good, self.window_start, self.window_end)
    }

    /// Whether the window has closed by `at`.
    pub fn is_closed(&self, at: Instant) -> bool {
        self.window_end <= at
    }

    /// Whether `other` claims the same thing: the same key, or the same area and
    /// direction over overlapping windows.
    pub fn same_claim(&self, area: &str, good: bool, start: Instant, end: Instant, key: &str) -> bool {
        self.key == key || (self.area.eq_ignore_ascii_case(area) && self.good == good && self.window_start < end && start < self.window_end)
    }

    /// One line of JSON, no newline.
    pub fn to_json_line(&self) -> String {
        let mut fields = vec![("id", Value::Str(self.id.clone()))];
        fields.extend(self.canonical_fields());
        match self.resolved {
            Some((o, when)) => {
                fields.push(("outcome", Value::Str(o.key().into())));
                fields.push(("resolved_at", Value::Str(iso_utc(when))));
            }
            None => {
                fields.push(("outcome", Value::Null));
                fields.push(("resolved_at", Value::Null));
            }
        }
        json::write_object(&fields)
    }

    /// Read one line written by [`SealedClaim::to_json_line`]; the id is read as stored
    /// (call [`SealedClaim::verify`]).
    pub fn from_json_line(line: &str) -> Result<Self, LedgerError> {
        let fields = json::parse_object(line).map_err(LedgerError::BadLine)?;
        let get = |k: &str| fields.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        let text = |k: &str| match get(k) {
            Some(Value::Str(s)) => Ok(s.clone()),
            _ => Err(LedgerError::BadLine(format!("missing \"{k}\""))),
        };
        let when = |k: &str| parse_iso_utc(&text(k)?);
        let probability = match get("probability") {
            Some(Value::Num(n)) => *n,
            _ => return Err(LedgerError::BadLine("missing \"probability\"".into())),
        };
        let good = matches!(get("good"), Some(Value::Bool(true)));
        let resolved = match (get("outcome"), get("resolved_at")) {
            (Some(Value::Str(o)), Some(Value::Str(s))) => Outcome::parse(o).map(|o| parse_iso_utc(s).map(|t| (o, t))).transpose()?,
            _ => None,
        };
        Ok(Self {
            id: text("id")?,
            key: text("key").unwrap_or_default(),
            created_at: when("created_at")?,
            source: text("source").unwrap_or_default(),
            area: text("area")?,
            kind: text("kind").unwrap_or_default(),
            good,
            window_start: when("window_start")?,
            window_end: when("window_end")?,
            probability,
            statement: text("statement").unwrap_or_default(),
            question: text("question").unwrap_or_default(),
            resolved,
        })
    }
}

/// What the caller wants sealed.
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimInput {
    /// What produced it.
    pub source: String,
    /// The part of life.
    pub area: String,
    /// The kind of claim.
    pub kind: String,
    /// True if said to be good.
    pub good: bool,
    /// Window start.
    pub window_start: Instant,
    /// Window end.
    pub window_end: Instant,
    /// Probability that it comes as said.
    pub probability: f64,
    /// The prediction.
    pub statement: String,
    /// The question that resolves it.
    pub question: String,
}

/// Seal a claim at `created_at`. Refused when the window is empty, has already opened,
/// the probability is not a number in `[0, 1]`, or there is nothing to ask.
pub fn seal_claim(input: &ClaimInput, created_at: Instant) -> Result<SealedClaim, LedgerError> {
    let (start, end) = (input.window_start, input.window_end);
    if end <= start {
        return Err(LedgerError::Refused("the window is empty".into()));
    }
    if start <= created_at {
        return Err(LedgerError::Refused(format!(
            "the window opens at {} but the claim is being sealed at {}: a claim about a window already open cannot be sealed",
            iso_utc(start),
            iso_utc(created_at)
        )));
    }
    if !(0.0..=1.0).contains(&input.probability) {
        return Err(LedgerError::Refused(format!("probability {} is not between 0 and 1", input.probability)));
    }
    if input.area.trim().is_empty() || input.question.trim().is_empty() {
        return Err(LedgerError::Refused("a claim needs a part of life and a question".into()));
    }
    let mut c = SealedClaim {
        id: String::new(),
        key: claim_key(&input.area, &input.kind, input.good, start, end),
        created_at,
        source: input.source.trim().to_string(),
        area: input.area.trim().to_string(),
        kind: input.kind.trim().to_string(),
        good: input.good,
        window_start: start,
        window_end: end,
        probability: input.probability,
        statement: input.statement.trim().to_string(),
        question: input.question.trim().to_string(),
        resolved: None,
    };
    c.id = c.content_id();
    Ok(c)
}

/// Record the person's answer.
pub fn resolve_claim(claim: &SealedClaim, outcome: Outcome, when: Instant) -> SealedClaim {
    SealedClaim { resolved: Some((outcome, when)), ..claim.clone() }
}

/// Fewest days between sealing and a window's opening, so nothing is sealed on the eve.
pub const MIN_LEAD_DAYS: i128 = 1;

/// Seal at `at` up to `max_new` of `candidates` that no existing claim already makes
/// (same key, or the same area and direction over an overlapping window) and that
/// open at least [`MIN_LEAD_DAYS`] after `at`; the soonest window first (it is answered
/// soonest), then the furthest from an even chance. Candidates that would be refused
/// are skipped, never an error.
pub fn select_new(existing: &[SealedClaim], candidates: &[ClaimInput], at: Instant, max_new: usize) -> Vec<SealedClaim> {
    let mut order: Vec<&ClaimInput> = candidates.iter().filter(|c| c.window_start.ns_since(at) >= MIN_LEAD_DAYS * NS_PER_DAY).collect();
    order.sort_by(|a, b| {
        a.window_start
            .cmp(&b.window_start)
            .then((b.probability - 0.5).abs().partial_cmp(&(a.probability - 0.5).abs()).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut out: Vec<SealedClaim> = Vec::new();
    for c in order {
        if out.len() >= max_new {
            break;
        }
        let key = claim_key(&c.area, &c.kind, c.good, c.window_start, c.window_end);
        let taken = existing.iter().chain(out.iter()).any(|e| e.same_claim(&c.area, c.good, c.window_start, c.window_end, &key));
        if taken {
            continue;
        }
        if let Ok(s) = seal_claim(c, at) {
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn day(y: i64, m: u32, d: u32) -> Instant {
        Utc::new(y, m, d, 6, 0, 0, 0).unwrap().to_instant().unwrap()
    }

    fn input(area: &str, good: bool, a: Instant, b: Instant, p: f64) -> ClaimInput {
        ClaimInput {
            source: "forecast".into(),
            area: area.into(),
            kind: if good { "opening".into() } else { "pressure".into() },
            good,
            window_start: a,
            window_end: b,
            probability: p,
            statement: "Money improves.".into(),
            question: "Between 14 Nov 2026 and 19 Nov 2026, did money come in well?".into(),
        }
    }

    #[test]
    fn a_claim_is_sealed_verified_and_round_trips() {
        let at = day(2026, 10, 21);
        let c = seal_claim(&input("Money", true, day(2026, 11, 14), day(2026, 11, 19), 0.7), at).unwrap();
        assert_eq!(c.id.len(), 64);
        assert_eq!(c.key.len(), 64);
        assert!(c.verify());
        let line = c.to_json_line();
        assert!(line.ends_with("\"outcome\":null,\"resolved_at\":null}"), "{line}");
        let back = SealedClaim::from_json_line(&line).unwrap();
        assert_eq!(back, c);
        assert!(back.verify());
        // An edit to the claim shows.
        let mut t = back.clone();
        t.probability = 0.9;
        assert!(!t.verify());
        assert!(!SealedClaim::from_json_line(&line.replace("Money improves.", "Money falls.")).unwrap().verify());
        // A resolution keeps the id.
        let r = resolve_claim(&c, Outcome::Close, day(2026, 11, 20));
        assert!(r.verify());
        let rb = SealedClaim::from_json_line(&r.to_json_line()).unwrap();
        assert_eq!(rb.resolved, Some((Outcome::Close, day(2026, 11, 20))));
        assert!(rb.is_closed(day(2026, 11, 20)) && !rb.is_closed(day(2026, 11, 18)));
        // The key ignores when it was sealed and the probability.
        let again = seal_claim(&input("money", true, day(2026, 11, 14), day(2026, 11, 19), 0.6), day(2026, 10, 25)).unwrap();
        assert_eq!(again.key, c.key);
        assert_ne!(again.id, c.id);
    }

    #[test]
    fn sealing_is_refused_once_the_window_opens() {
        let at = day(2026, 11, 15);
        for i in [
            input("Money", true, day(2026, 11, 14), day(2026, 11, 19), 0.7),
            input("Money", true, day(2026, 11, 20), day(2026, 11, 20), 0.7),
            input("Money", true, day(2026, 11, 20), day(2026, 11, 25), f64::NAN),
            input("Money", true, day(2026, 11, 20), day(2026, 11, 25), 1.2),
            input(" ", true, day(2026, 11, 20), day(2026, 11, 25), 0.5),
        ] {
            assert!(matches!(seal_claim(&i, at), Err(LedgerError::Refused(_))), "{i:?}");
        }
    }

    #[test]
    fn select_new_never_seals_the_same_prediction_twice() {
        let at = day(2026, 10, 21);
        let a = input("Money", true, day(2026, 11, 14), day(2026, 11, 19), 0.7);
        let overlapping = input("Money", true, day(2026, 11, 16), day(2026, 12, 30), 0.65);
        let other_way = input("Money", false, day(2026, 11, 16), day(2026, 12, 30), 0.3);
        let too_soon = input("Love", true, day(2026, 10, 21), day(2026, 10, 30), 0.8);
        let later = input("Love", true, day(2027, 1, 1), day(2027, 2, 1), 0.8);
        let first = select_new(&[], &[later.clone(), a.clone(), overlapping.clone(), too_soon.clone()], at, 5);
        assert_eq!(first.len(), 2, "{first:?}");
        assert_eq!(first[0].area, "Money");
        assert_eq!(first[1].area, "Love");
        // Again a week later: nothing new but the other direction.
        let second = select_new(&first, &[a, overlapping, other_way, later, too_soon], day(2026, 10, 28), 5);
        assert_eq!(second.len(), 1);
        assert!(!second[0].good);
        // The cap holds.
        assert!(select_new(&[], &[input("Health", true, day(2027, 3, 1), day(2027, 4, 1), 0.6), input("Home", true, day(2027, 3, 1), day(2027, 4, 1), 0.6)], at, 1).len() == 1);
    }
}
