//! A dated life event and its one-line JSON form.
//!
//! The line is `{"at":"2019-03-04T09:30:00Z","area":"Career","kind":"Began","weight":0.8,"note":"changed jobs"}`:
//! the instant as an ISO-8601 UTC string (nanoseconds kept when present), the area as
//! one of the [`AREAS`] names, the kind as one of the six [`EventKind`] names,
//! the importance in `[0, 1]` and the person's own words. Unknown keys are ignored on
//! reading, `weight` defaults to 1 and `note` to empty.

use crate::json::{self, Value};
use kshana_chart::ChartError;
use kshana_ephem::SpkError;
use kshana_rules::{area, AREAS};
use kshana_time::{days_from_civil, Instant, Utc, NS_PER_SEC};
use std::fmt;

/// Why the ledger refused something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    /// Not one of the [`AREAS`].
    UnknownArea(String),
    /// Not one of the six [`EventKind`]s.
    UnknownKind(String),
    /// A JSON line or an instant that could not be read; the text says what was wrong.
    BadLine(String),
    /// The ephemeris could not supply a position (usually: the instant lies outside the kernel).
    Ephemeris(SpkError),
    /// A chart could not be cast (a birth-time scan: polar latitude or kernel range).
    Chart(ChartError),
    /// The ledger refused on principle; the text says why (a forecast sealed after
    /// its window opened, a test with nothing to test).
    Refused(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::UnknownArea(a) => {
                let names: Vec<&str> = AREAS.iter().map(|x| x.name).collect();
                write!(
                    f,
                    "unknown area {a:?}; expected one of {}",
                    names.join(", ")
                )
            }
            LedgerError::UnknownKind(k) => {
                let names: Vec<&str> = EventKind::ALL.iter().map(|x| x.name()).collect();
                write!(
                    f,
                    "unknown kind {k:?}; expected one of {}",
                    names.join(", ")
                )
            }
            LedgerError::BadLine(why) => write!(f, "bad ledger line: {why}"),
            LedgerError::Ephemeris(e) => write!(f, "ephemeris: {e:?}"),
            LedgerError::Chart(e) => write!(f, "chart: {e:?}"),
            LedgerError::Refused(why) => write!(f, "refused: {why}"),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<SpkError> for LedgerError {
    fn from(e: SpkError) -> Self {
        LedgerError::Ephemeris(e)
    }
}

impl From<ChartError> for LedgerError {
    fn from(e: ChartError) -> Self {
        LedgerError::Chart(e)
    }
}

/// What kind of thing happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Something started: a job, a marriage, a course, a stay abroad.
    Began,
    /// Something ended.
    Ended,
    /// A gain: a promotion, an inheritance, a child born, a recovery.
    Gain,
    /// A loss: a job lost, money lost, a separation, a bereavement.
    Loss,
    /// A milestone: a wedding, a degree, a house bought.
    Milestone,
    /// A crisis: an illness, an accident, a breakdown, a lawsuit.
    Crisis,
}

impl EventKind {
    /// The six kinds.
    pub const ALL: [EventKind; 6] = [
        EventKind::Began,
        EventKind::Ended,
        EventKind::Gain,
        EventKind::Loss,
        EventKind::Milestone,
        EventKind::Crisis,
    ];

    /// The name written in a JSON line.
    pub fn name(self) -> &'static str {
        match self {
            EventKind::Began => "Began",
            EventKind::Ended => "Ended",
            EventKind::Gain => "Gain",
            EventKind::Loss => "Loss",
            EventKind::Milestone => "Milestone",
            EventKind::Crisis => "Crisis",
        }
    }

    /// Look a kind up by name, ignoring case.
    pub fn from_name(name: &str) -> Option<EventKind> {
        EventKind::ALL
            .iter()
            .copied()
            .find(|k| k.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Loss and Crisis: for these, the chart "supports" the event when the running
    /// periods were adverse for the area, so every factor is scored for adversity.
    pub fn is_adverse(self) -> bool {
        matches!(self, EventKind::Loss | EventKind::Crisis)
    }
}

/// A dated life event in one area.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeEvent {
    /// When it happened.
    pub at: Instant,
    /// One of the [`AREAS`] names, as spelt there.
    pub area: String,
    /// What kind of thing it was.
    pub kind: EventKind,
    /// Importance in `[0, 1]`; carried for the app, not used by the scoring.
    pub weight: f64,
    /// The person's own words.
    pub note: String,
}

fn clean_weight(w: f64) -> f64 {
    if w.is_finite() {
        w.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

impl LifeEvent {
    /// Build an event, checking the area name (any case) and clamping the weight.
    pub fn new(
        at: Instant,
        area_name: &str,
        kind: EventKind,
        weight: f64,
        note: &str,
    ) -> Result<Self, LedgerError> {
        let a = area(area_name).ok_or_else(|| LedgerError::UnknownArea(area_name.to_string()))?;
        Ok(Self {
            at,
            area: a.name.to_string(),
            kind,
            weight: clean_weight(weight),
            note: note.to_string(),
        })
    }

    /// One line of JSON, no newline.
    pub fn to_json_line(&self) -> String {
        json::write_object(&[
            ("at", Value::Str(iso_utc(self.at))),
            ("area", Value::Str(self.area.clone())),
            ("kind", Value::Str(self.kind.name().to_string())),
            ("weight", Value::Num(clean_weight(self.weight))),
            ("note", Value::Str(self.note.clone())),
        ])
    }

    /// Read one line written by [`LifeEvent::to_json_line`] (or by hand in that shape).
    pub fn from_json_line(line: &str) -> Result<Self, LedgerError> {
        let fields = json::parse_object(line).map_err(LedgerError::BadLine)?;
        let get = |k: &str| fields.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        let at = match get("at") {
            Some(Value::Str(s)) => parse_iso_utc(s)?,
            Some(_) => {
                return Err(LedgerError::BadLine(
                    "\"at\" must be an ISO-8601 UTC string".into(),
                ))
            }
            None => return Err(LedgerError::BadLine("missing \"at\"".into())),
        };
        let area_name = match get("area") {
            Some(Value::Str(s)) => s.as_str(),
            _ => return Err(LedgerError::BadLine("missing \"area\"".into())),
        };
        let kind = match get("kind") {
            Some(Value::Str(s)) => {
                EventKind::from_name(s).ok_or_else(|| LedgerError::UnknownKind(s.clone()))?
            }
            _ => return Err(LedgerError::BadLine("missing \"kind\"".into())),
        };
        let weight = match get("weight") {
            Some(Value::Num(n)) => *n,
            None | Some(Value::Null) => 1.0,
            Some(_) => return Err(LedgerError::BadLine("\"weight\" must be a number".into())),
        };
        let note = match get("note") {
            Some(Value::Str(s)) => s.as_str(),
            None | Some(Value::Null) => "",
            Some(_) => return Err(LedgerError::BadLine("\"note\" must be a string".into())),
        };
        Self::new(at, area_name, kind, weight, note)
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `"Mar 2019"` for an instant, by its UTC date.
pub fn month_year(at: Instant) -> String {
    let u = Utc::from_instant(at);
    format!("{} {}", MONTHS[(u.month as usize).clamp(1, 12) - 1], u.year)
}

/// An instant as `YYYY-MM-DDTHH:MM:SSZ`, with `.nnnnnnnnn` before the `Z` when the
/// nanoseconds are not zero. Exact: [`parse_iso_utc`] gives the instant back.
pub fn iso_utc(at: Instant) -> String {
    let u = Utc::from_instant(at);
    let mut s = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        u.year, u.month, u.day, u.hour, u.minute, u.second
    );
    if u.nanos != 0 {
        s.push_str(&format!(".{:09}", u.nanos));
    }
    s.push('Z');
    s
}

/// The instant of a civil UTC value. Dates before 1972, which the modern UTC scale does
/// not cover, are labelled with TAI − UTC = 10 s, the inverse of [`Utc::from_instant`].
fn instant_of(u: &Utc) -> Instant {
    match u.to_instant() {
        Ok(i) => i,
        Err(_) => {
            let days = (u.utc_day() - days_from_civil(1958, 1, 1)) as i128;
            let s = days * 86_400
                + u.hour as i128 * 3_600
                + u.minute as i128 * 60
                + u.second as i128
                + 10;
            Instant::from_tai_ns(s * NS_PER_SEC + u.nanos as i128)
        }
    }
}

/// Read `YYYY-MM-DD`, optionally followed by `T` (or a space) and `HH:MM[:SS[.fraction]]`,
/// optionally ending in `Z`. The time is UTC; a date alone means midnight.
pub fn parse_iso_utc(text: &str) -> Result<Instant, LedgerError> {
    let s = text.trim();
    let bad = |why: &str| LedgerError::BadLine(format!("bad instant {s:?}: {why}"));
    let (date, time) = match s.find(['T', 't', ' ']) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    };
    let d: Vec<&str> = date.split('-').collect();
    if d.len() != 3 {
        return Err(bad("expected YYYY-MM-DD"));
    }
    let year: i64 = d[0].parse().map_err(|_| bad("year"))?;
    let month: u32 = d[1].parse().map_err(|_| bad("month"))?;
    let day: u32 = d[2].parse().map_err(|_| bad("day"))?;
    let time = time.strip_suffix(['Z', 'z']).unwrap_or(time);
    let (mut hour, mut minute, mut second, mut nanos) = (0u32, 0u32, 0u32, 0u32);
    if !time.is_empty() {
        let t: Vec<&str> = time.split(':').collect();
        if t.len() < 2 || t.len() > 3 {
            return Err(bad("expected HH:MM[:SS[.fraction]]"));
        }
        hour = t[0].parse().map_err(|_| bad("hour"))?;
        minute = t[1].parse().map_err(|_| bad("minute"))?;
        if let Some(sec) = t.get(2) {
            let (whole, frac) = match sec.find('.') {
                Some(i) => (&sec[..i], &sec[i + 1..]),
                None => (*sec, ""),
            };
            second = whole.parse().map_err(|_| bad("second"))?;
            if !frac.is_empty() {
                if frac.len() > 9 || !frac.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad("fraction must be 1 to 9 digits"));
                }
                let mut f = frac.to_string();
                while f.len() < 9 {
                    f.push('0');
                }
                nanos = f.parse().map_err(|_| bad("fraction"))?;
            }
        }
    }
    let u = Utc::new(year, month, day, hour, minute, second, nanos)
        .map_err(|e| bad(&format!("{e:?}")))?;
    Ok(instant_of(&u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;

    #[test]
    fn json_line_round_trips_with_awkward_text() {
        let at = Utc::new(2019, 3, 4, 9, 30, 15, 123_456_789)
            .unwrap()
            .to_instant()
            .unwrap();
        let ev = LifeEvent::new(
            at,
            "career",
            EventKind::Began,
            0.8,
            "changed jobs: \"Acme\" → Zed\\Co\n(second line)",
        )
        .unwrap();
        assert_eq!(ev.area, "Career");
        let line = ev.to_json_line();
        assert!(!line.contains('\n'));
        assert!(line.starts_with("{\"at\":\"2019-03-04T09:30:15.123456789Z\",\"area\":\"Career\",\"kind\":\"Began\",\"weight\":0.8,\"note\":"));
        let back = LifeEvent::from_json_line(&line).unwrap();
        assert_eq!(back, ev);
    }

    #[test]
    fn json_line_tolerates_hand_written_shapes() {
        let ev = LifeEvent::from_json_line(
            " { \"kind\" : \"crisis\", \"area\" : \"HEALTH\", \"at\" : \"2020-06-15\", \"extra\" : true } ",
        )
        .unwrap();
        assert_eq!(ev.area, "Health");
        assert_eq!(ev.kind, EventKind::Crisis);
        assert_eq!(ev.weight, 1.0);
        assert_eq!(ev.note, "");
        assert_eq!(iso_utc(ev.at), "2020-06-15T00:00:00Z");
        let w = LifeEvent::from_json_line(
            "{\"at\":\"2020-06-15T10:00Z\",\"area\":\"Wealth\",\"kind\":\"Gain\",\"weight\":7}",
        )
        .unwrap();
        assert_eq!(w.weight, 1.0);
        assert!(matches!(
            LifeEvent::from_json_line(
                "{\"at\":\"2020-06-15\",\"area\":\"Pets\",\"kind\":\"Gain\"}"
            ),
            Err(LedgerError::UnknownArea(_))
        ));
        assert!(matches!(
            LifeEvent::from_json_line(
                "{\"at\":\"2020-06-15\",\"area\":\"Wealth\",\"kind\":\"Win\"}"
            ),
            Err(LedgerError::UnknownKind(_))
        ));
        assert!(matches!(
            LifeEvent::from_json_line("{\"area\":\"Wealth\",\"kind\":\"Gain\"}"),
            Err(LedgerError::BadLine(_))
        ));
        assert!(matches!(
            LifeEvent::from_json_line(
                "{\"at\":\"yesterday\",\"area\":\"Wealth\",\"kind\":\"Gain\"}"
            ),
            Err(LedgerError::BadLine(_))
        ));
        assert!(LifeEvent::from_json_line("not json").is_err());
    }

    #[test]
    fn instants_before_1972_round_trip_through_the_label() {
        let old = Instant::j2000().add_ns(-40 * 365 * NS_PER_DAY - 12_345);
        let text = iso_utc(old);
        assert!(text.starts_with("1960-"));
        assert_eq!(parse_iso_utc(&text).unwrap(), old);
        let modern = Instant::j2000().add_ns(5 * NS_PER_DAY + 999_999_999);
        assert_eq!(parse_iso_utc(&iso_utc(modern)).unwrap(), modern);
        assert_eq!(
            parse_iso_utc("2016-12-31T23:59:60Z").unwrap(),
            parse_iso_utc("2017-01-01T00:00:00Z")
                .unwrap()
                .add_ns(-NS_PER_SEC)
        );
        assert_eq!(month_year(modern), "Jan 2000");
    }

    #[test]
    fn kinds_and_errors_have_words() {
        for k in EventKind::ALL {
            assert_eq!(EventKind::from_name(&k.name().to_lowercase()), Some(k));
        }
        assert!(EventKind::Loss.is_adverse() && EventKind::Crisis.is_adverse());
        assert!(!EventKind::Gain.is_adverse());
        let e = LedgerError::UnknownArea("Pets".into()).to_string();
        assert!(e.contains("Career") && e.contains("Foreign"));
        assert!(LedgerError::UnknownKind("Win".into())
            .to_string()
            .contains("Milestone"));
    }
}
