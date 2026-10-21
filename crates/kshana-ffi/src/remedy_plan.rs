//! Remedies with follow-through over the FFI: for every remedy of the remedy tree, the
//! exact first hour to begin it at the person's current place, how long to keep it, and
//! which hard stretch it is for.
//!
//! The first hour comes from `remedy_timing::TimingTable::best_start`: a daytime hora of
//! the influence's planet on its own weekday, clear of that day's Rahu kala, preferring a
//! day whose tithi or nakshatra is the planet's (or Pushya) and the waxing fortnight. The
//! length comes from `remedy_timing::keep_for`, which marks each count as a text's, a
//! custom's, or the app's plain default. The stretch is the tree cause's own span (from
//! the running period or the slow transit behind it).
//!
//! What the person reads: `title`, `start_words`, `keep_words`, `keep_basis_words`,
//! `until_words`, `for_stretch_words`, `why_now` and `reminder_text`, all in plain words
//! (English for now). `remedy_id`, `branch`, `keep_basis`, `keep_source` and `start_iso`
//! are for the app and the expert layer (see `plain_audit` below). Remedies are steadying
//! good acts begun at a good hour; nothing here promises they change what is coming.

use kshana_rules::remedy_timing::{keep_for, Keep, StartChoice, TimingTable, WEEKDAY_NAMES};
use kshana_rules::{Lang, Life};
use kshana_sky::Observer;
use kshana_time::{Instant, Utc, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::DashaLord;

use crate::{focus_of, lord_name, to_house_system, AyanamsaChoice, Engine, ExactInstant, HouseSystem, KshanaError};

/// One remedy's start, length and purpose.
#[derive(Clone, Debug, uniffi::Record)]
pub struct RemedyScheduleRecord {
    /// Stable key for the app's stores: `T:<life>:<influence>:<branch>:<index>`. Never shown.
    pub remedy_id: String,
    /// The remedy, as the tree words it.
    pub title: String,
    /// Index of its cause in the tree (strongest first).
    pub cause_index: u32,
    /// The tree branch key (`deeds`, `giving` …). Never shown.
    pub branch: String,
    /// Index of the remedy within its branch.
    pub item_index: u32,
    /// The first hour, exact.
    pub start: ExactInstant,
    /// Its start as local ISO 8601 with the offset given ("2026-10-24T07:12:31+05:30").
    pub start_iso: String,
    /// Its start in Unix epoch milliseconds (for alarms).
    pub start_ms: i64,
    /// The end of that first hour, epoch milliseconds.
    pub start_end_ms: i64,
    /// "Saturday 24 Oct, 7:13–8:10 am".
    pub start_words: String,
    /// Days from the first time to the end of the last.
    pub days: u32,
    /// How many times it is done.
    pub times: u32,
    /// Days between two times (1 daily, 7 weekly, 30 monthly, 0 for one act).
    pub every_days: u32,
    /// ISO weekday 1 (Monday) … 7 (Sunday) for a weekly remedy, 0 otherwise.
    pub weekday_iso: u8,
    /// "7 Saturdays in a row", "21 days in a row", "once, within the week".
    pub keep_words: String,
    /// `text`, `custom` or `default`. Never shown.
    pub keep_basis: String,
    /// The count's standing in plain words: "the count kept in temples and homes".
    pub keep_basis_words: String,
    /// Where the count comes from (expert).
    pub keep_source: String,
    /// The end of the last time, epoch milliseconds.
    pub until_ms: i64,
    /// "until Saturday 5 Dec".
    pub until_words: String,
    /// The hard stretch it answers: start and end in epoch milliseconds, when known.
    pub stretch_start_ms: Option<i64>,
    /// The end of that stretch, epoch milliseconds; none when no end is in sight.
    pub stretch_end_ms: Option<i64>,
    /// "For the delays and heavy duties on your career, easing around 3 Mar 2027."
    pub for_stretch_words: String,
    /// When the app should ask "did things ease?": the earlier of the stretch's end and
    /// the last day, never sooner than a week after the start. Epoch milliseconds.
    pub check_in_ms: i64,
    /// Why this hour, in plain words.
    pub why_now: String,
    /// What the start reminder says.
    pub reminder_text: String,
    /// Where the remedy, its length and its start come from: resolve with
    /// `source_lines`. Never shown as is.
    pub source_keys: Vec<String>,
}

/// Unix epoch milliseconds of an instant (a leap second reads as :59).
pub(crate) fn unix_ms(t: Instant) -> i64 {
    let u = Utc::from_instant(t);
    u.utc_day() * 86_400_000
        + u.hour as i64 * 3_600_000
        + u.minute as i64 * 60_000
        + u.second.min(59) as i64 * 1_000
        + (u.nanos / 1_000_000) as i64
}

fn local(t: Instant, offset_min: i32) -> Utc {
    Utc::from_instant(t.add_ns(offset_min as i128 * 60 * NS_PER_SEC))
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn weekday_of_local(u: &Utc) -> usize {
    (u.utc_day() + 4).rem_euclid(7) as usize
}

/// "Saturday 24 Oct".
fn day_words(t: Instant, offset_min: i32) -> String {
    let u = local(t, offset_min);
    format!("{} {} {}", WEEKDAY_NAMES[weekday_of_local(&u)], u.day, MONTHS[(u.month as usize).clamp(1, 12) - 1])
}

/// "24 Oct 2026".
fn date_words(t: Instant, offset_min: i32) -> String {
    let u = local(t, offset_min);
    format!("{} {} {}", u.day, MONTHS[(u.month as usize).clamp(1, 12) - 1], u.year)
}

/// (hour 1–12, minute, "am"/"pm") of a local time.
fn clock(u: &Utc) -> (u32, u32, &'static str) {
    let h = u.hour % 12;
    (if h == 0 { 12 } else { h }, u.minute, if u.hour < 12 { "am" } else { "pm" })
}

/// "7:13–8:10 am" or "11:40 am–12:38 pm": the start rounded up to the minute so the
/// person is never early, the end rounded down.
fn span_words(a: Instant, b: Instant, offset_min: i32) -> String {
    let up = a.add_ns(NS_PER_SEC * 60 - 1);
    let (ua, ub) = (local(up, offset_min), local(b, offset_min));
    let (ha, ma, pa) = clock(&ua);
    let (hb, mb, pb) = clock(&ub);
    if pa == pb {
        format!("{ha}:{ma:02}–{hb}:{mb:02} {pb}")
    } else {
        format!("{ha}:{ma:02} {pa}–{hb}:{mb:02} {pb}")
    }
}

fn iso(t: Instant, offset_min: i32) -> String {
    let u = local(t, offset_min);
    let sign = if offset_min < 0 { '-' } else { '+' };
    let o = offset_min.unsigned_abs();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        u.year,
        u.month,
        u.day,
        u.hour,
        u.minute,
        u.second.min(59),
        o / 60,
        o % 60
    )
}

fn plural_day(wd: usize) -> String {
    format!("{}s", WEEKDAY_NAMES[wd % 7])
}

/// The length in plain words.
fn keep_words(k: &Keep, wd: usize, on_weekday: bool) -> String {
    match (k.every, k.times) {
        (0, _) if k.days >= 28 => "once, within the month".into(),
        (0, _) if k.days >= 7 => "once, within the week".into(),
        (0, _) => "once".into(),
        (1, n) => format!("{n} days in a row"),
        (7, n) if on_weekday => format!("{n} {} in a row", plural_day(wd)),
        (7, n) => format!("once a week for {n} weeks"),
        (2, n) => format!("three times a week for {} weeks", n / 3),
        (_, n) => format!("once a month for {n} months"),
    }
}

fn basis_words(basis: &str) -> &'static str {
    match basis {
        "text" => "a length the old books give",
        "custom" => "the count kept in temples and homes",
        _ => "the tradition sets no count; this is long enough to notice a change",
    }
}

/// Why this hour.
fn why_now(c: &StartChoice, offset_min: i32) -> String {
    let wd = WEEKDAY_NAMES[c.weekday as usize % 7];
    let part = if c.hora.hora_index == 1 { "the first hour after sunrise" } else { "a good hour" };
    let mut s = format!("It begins in {part} on {wd}, the day kept for this kind of help.");
    match (c.tithi_supports, c.star_supports) {
        (true, true) => s.push_str(" The date in the old calendar and the day's star both favour a start."),
        (true, false) => s.push_str(" The date in the old calendar favours a start."),
        (false, true) => s.push_str(" The day's star favours a start."),
        _ => {}
    }
    if c.waxing {
        s.push_str(" It falls in the bright half of the month, when vows are begun.");
    }
    if c.weeks_later > 0 {
        s.push_str(&format!(
            " It waits {} for the {wd} that suits best.",
            if c.weeks_later == 1 { "a week" } else { "two weeks" }
        ));
    }
    s.push_str(&format!(
        " It keeps clear of {}, the part of that day the tradition avoids for new starts.",
        span_words(c.rahu_kala.0, c.rahu_kala.1, offset_min)
    ));
    s
}

/// The stretch in plain words.
fn stretch_words(lord: DashaLord, kind: &str, life: Life, until: Option<Instant>, offset_min: i32) -> String {
    let p = kshana_rules::plain::pressure(lord);
    let life = life.name().to_ascii_lowercase();
    match until {
        Some(u) => format!("For the {p} weighing on your {life} now, easing around {}.", date_words(u, offset_min)),
        None if kind == "natal" => format!("For the {p} your {life} has carried from birth; keep it as long as it steadies you."),
        None => format!("For the {p} weighing on your {life}; no end is in sight within two years, so this is what you can do."),
    }
}

#[uniffi::export]
impl Engine {
    /// For every remedy of the remedy tree (same arguments as `remedy_tree`, plus the
    /// offset of the person's clock from UTC in minutes, IST = 330): the exact first hour
    /// to begin it at the current place `lat`/`lon` (the planet's weekday, its daytime
    /// hora, never in Rahu kala, a supportive date or star preferred), how long to keep
    /// it, and which hard stretch it answers. In tree order: cause, branch, item.
    #[allow(clippy::too_many_arguments)]
    pub fn remedy_schedule(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        lat: f64,
        lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        life: Option<String>,
        at: ExactInstant,
        lang: String,
        focus: Vec<String>,
        utc_offset_minutes: i32,
    ) -> Result<Vec<RemedyScheduleRecord>, KshanaError> {
        let offset = utc_offset_minutes.clamp(-14 * 60, 14 * 60);
        let natal = self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?;
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let at_i: Instant = at.into();
        let lang = Lang::parse(&lang);
        let life = life.as_deref().and_then(Life::parse);
        let tree = kshana_rules::remedy_tree(
            &self.spk,
            self.eop.as_ref(),
            &natal,
            &obs,
            to_house_system(system),
            ayanamsa.into(),
            life,
            lang,
            at_i,
            &focus_of(&focus),
        )
        .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let table = TimingTable::compute(&self.spk, self.eop.as_ref(), &obs, ayanamsa.into(), at_i, 35)
            .map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let any_day = kshana_rules::lang::any_day(lang);
        let mut out = Vec::new();
        for (ci, c) in tree.causes.iter().enumerate() {
            let Some(start) = table.best_start(&obs, c.key, at_i, 3) else { continue };
            let s = start.hora.start;
            let wd = start.weekday as usize;
            let stretch = stretch_words(c.key, c.source, tree.life, c.until, offset);
            let why = why_now(&start, offset);
            for b in &c.branches {
                for (ii, r) in b.items.iter().enumerate() {
                    let on_weekday = r.day != any_day;
                    let k = keep_for(kshana_rules::remedy_tree::effort_key(&r.effort), b.key, c.key, on_weekday);
                    let until = s.add_ns(k.days as i128 * NS_PER_DAY);
                    let last = s.add_ns((k.days.max(1) as i128 - 1) * NS_PER_DAY);
                    let until_ms = unix_ms(until);
                    let start_ms = unix_ms(s);
                    let stretch_end_ms = c.until.map(unix_ms);
                    let check = stretch_end_ms.map_or(until_ms, |e| e.min(until_ms)).max(start_ms + 7 * 86_400_000);
                    let kw = keep_words(&k, wd, on_weekday);
                    out.push(RemedyScheduleRecord {
                        remedy_id: format!("T:{}:{}:{}:{ii}", tree.life.name(), lord_name(c.key), b.key),
                        title: r.title.clone(),
                        cause_index: ci as u32,
                        branch: b.key.to_string(),
                        item_index: ii as u32,
                        start: s.into(),
                        start_iso: iso(s, offset),
                        start_ms,
                        start_end_ms: unix_ms(start.hora.end),
                        start_words: format!("{}, {}", day_words(s, offset), span_words(s, start.hora.end, offset)),
                        days: k.days,
                        times: k.times,
                        every_days: k.every,
                        weekday_iso: if k.every == 7 && on_weekday { if wd == 0 { 7 } else { wd as u8 } } else { 0 },
                        keep_basis: k.basis.to_string(),
                        keep_basis_words: if k.times <= 1 { "one act; no count is needed".to_string() } else { basis_words(k.basis).to_string() },
                        keep_source: k.source.to_string(),
                        until_ms,
                        until_words: match (k.times, k.days) {
                            (0 | 1, 0 | 1) => format!("on {}", day_words(last, offset)),
                            (0 | 1, _) => format!("by {}", day_words(last, offset)),
                            _ => format!("until {}", day_words(last, offset)),
                        },
                        stretch_start_ms: c.since.map(unix_ms),
                        stretch_end_ms,
                        for_stretch_words: stretch.clone(),
                        check_in_ms: check,
                        why_now: why.clone(),
                        reminder_text: format!("Time to begin: {}. Keep it {kw}.", r.title.trim_end_matches('.')),
                        keep_words: kw,
                        source_keys: r
                            .source_keys
                            .iter()
                            .copied()
                            .chain([kshana_rules::sources::of_keep(k.source), "kshana.timing"])
                            .map(String::from)
                            .collect(),
                    });
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};
    use kshana_events::{hora_at, rahu_kala, solar_day};

    /// Fields of [`RemedyScheduleRecord`] the common screens never show.
    const NOT_SHOWN: &[&str] = &["remedy_id", "branch", "keep_basis", "keep_source", "start_iso", "source_keys"];

    fn engine() -> std::sync::Arc<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Engine::open(p.to_string_lossy().into_owned(), None).expect("the kernel is needed for this test")
    }

    fn check(field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(kshana_rules::Lang::En, v) {
            hits.push(format!("{field} [{w}]: {v}"));
        }
        for bad in [
            "guarantee", "removes all", "change your destiny", "changes destiny", "rewrite", "will fix",
            // Nothing frightening either.
            "if you don't", "if you do not", "or else", "destroy", "curse", "black magic", "evil eye", "punish",
        ] {
            if v.to_lowercase().contains(bad) {
                hits.push(format!("{field} promises too much [{bad}]: {v}"));
            }
        }
    }

    fn at(y: i64, m: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: m, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn schedules(e: &Engine, life: Option<&str>, now: ExactInstant, lat: f64, lon: f64) -> Vec<RemedyScheduleRecord> {
        let b = at(1990, 2, 20, 5);
        e.remedy_schedule(b, 9.9312, 76.2673, lat, lon, HouseSystem::Placidus, AyanamsaChoice::Lahiri, life.map(Into::into), now, "en".into(), vec![], 330)
            .unwrap()
    }

    #[test]
    fn remedy_schedule_words_name_no_astrology() {
        let e = engine();
        let mut hits = Vec::new();
        for life in [None, Some("Love"), Some("Career"), Some("Health"), Some("Money")] {
            for r in schedules(&e, life, at(2026, 10, 21, 6), 28.6139, 77.2090) {
                for (f, v) in [
                    ("title", r.title.as_str()),
                    ("start_words", &r.start_words),
                    ("keep_words", &r.keep_words),
                    ("keep_basis_words", &r.keep_basis_words),
                    ("until_words", &r.until_words),
                    ("for_stretch_words", &r.for_stretch_words),
                    ("why_now", &r.why_now),
                    ("reminder_text", &r.reminder_text),
                    ("remedy_id", &r.remedy_id),
                    ("keep_source", &r.keep_source),
                ] {
                    check(f, v, &mut hits);
                }
            }
        }
        assert!(hits.is_empty(), "{hits:#?}");
    }

    #[test]
    fn every_start_is_on_the_planets_day_in_its_hour_and_never_in_rahu_kala() {
        let e = engine();
        // Born at Kochi, living in Delhi: the start is solved for Delhi.
        let (lat, lon) = (28.6139, 77.2090);
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let now = at(2026, 10, 21, 6);
        let tree = e.remedy_tree(at(1990, 2, 20, 5), 9.9312, 76.2673, lat, lon, HouseSystem::Placidus, AyanamsaChoice::Lahiri, None, now.clone(), "en".into(), vec![]).unwrap();
        let rs = schedules(&e, None, now.clone(), lat, lon);
        let items: usize = tree.causes.iter().map(|c| c.branches.iter().map(|b| b.items.len()).sum::<usize>()).sum();
        assert_eq!(rs.len(), items, "one schedule per tree remedy");
        let now_i: Instant = now.into();
        let tol = 10 * NS_PER_SEC / 1_000;
        for r in &rs {
            let cause = &tree.causes[r.cause_index as usize];
            assert_eq!(r.title, cause.branches.iter().find(|b| b.key == r.branch).unwrap().items[r.item_index as usize].title);
            let s: Instant = r.start.clone().into();
            assert!(s > now_i, "{}", r.start_words);
            assert!(s.ns_since(now_i) < 22 * NS_PER_DAY, "within three weeks: {}", r.start_words);
            let probe = s.add_ns(NS_PER_SEC * 60);
            let d = solar_day(&e.spk, None, &obs, probe).unwrap().unwrap();
            let lord = match cause.key.as_str() {
                "Rahu" => "Saturn",
                "Ketu" => "Mars",
                k => k,
            };
            let wd = kshana_rules::remedy_timing::weekday_at(d.sunrise, &obs);
            let want = kshana_rules::remedy_timing::weekday_of(match cause.key.as_str() {
                "Sun" => DashaLord::Sun,
                "Moon" => DashaLord::Moon,
                "Mars" => DashaLord::Mars,
                "Mercury" => DashaLord::Mercury,
                "Jupiter" => DashaLord::Jupiter,
                "Venus" => DashaLord::Venus,
                "Saturn" => DashaLord::Saturn,
                "Rahu" => DashaLord::Rahu,
                _ => DashaLord::Ketu,
            });
            assert_eq!(wd, want, "{}: {}", cause.key, r.start_words);
            let (hl, hs, _) = hora_at(&d, &obs, probe);
            assert_eq!(hl, lord, "{}", r.start_words);
            assert!(hs.ns_since(s).abs() < tol);
            assert!(s >= d.sunrise.add_ns(-tol) && s < d.sunset, "by day: {}", r.start_words);
            let (a, b) = rahu_kala(&d, &obs);
            let end = s.add_ns((r.start_end_ms - r.start_ms) as i128 * 1_000_000);
            assert!(end <= a || s >= b, "inside Rahu kala: {}", r.start_words);
            assert!(r.start_words.starts_with(&r.start_words.split(' ').next().unwrap().to_string()));
            assert!(r.check_in_ms >= r.start_ms + 7 * 86_400_000);
            assert!(r.until_ms > r.start_ms);
        }
    }

    #[test]
    fn durations_follow_the_counts() {
        use kshana_rules::remedy_timing::keep_for;
        let k = keep_for("weekly", "deeds", DashaLord::Saturn, true);
        assert_eq!((k.times, k.days, k.every, k.basis), (7, 43, 7, "custom"));
        assert_eq!(keep_words(&k, 6, true), "7 Saturdays in a row");
        let k = keep_for("weekly", "giving", DashaLord::Moon, true);
        assert_eq!((k.times, k.days), (16, 106));
        assert_eq!(keep_words(&k, 1, true), "16 Mondays in a row");
        let k = keep_for("weekly", "giving", DashaLord::Rahu, true);
        assert_eq!((k.times, k.basis), (8, "default"));
        let k = keep_for("daily", "deeds", DashaLord::Mars, false);
        assert_eq!((k.times, k.days, k.basis), (21, 21, "custom"));
        let k = keep_for("daily", "tradition", DashaLord::Mars, false);
        assert_eq!((k.times, k.days), (48, 48));
        let k = keep_for("40", "habits", DashaLord::Sun, false);
        assert_eq!((k.days, k.basis), (40, "text"));
        let k = keep_for("week", "deeds", DashaLord::Sun, false);
        assert_eq!(keep_words(&k, 0, false), "once, within the week");
        assert_eq!(keep_words(&keep_for("monthly", "nature", DashaLord::Sun, false), 0, false), "once a month for 3 months");
        assert_eq!(keep_words(&keep_for("3w", "habits", DashaLord::Sun, false), 0, false), "three times a week for 8 weeks");
        // Every effort in every tree branch maps to a known key, in all three languages.
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            for g in DashaLord::ORDER {
                for b in kshana_rules::remedy_tree::branches(lang, g) {
                    for it in &b.items {
                        let key = kshana_rules::remedy_tree::effort_key(&it.effort);
                        assert!(key != "once" || ["once", "ഒരിക്കൽ", "एक बार"].contains(&it.effort.as_str()), "{lang:?} {g:?}: {}", it.effort);
                    }
                }
            }
        }
        // The schedule's words for a weekly remedy agree with its count and end date.
        let e = engine();
        for r in schedules(&e, Some("Career"), at(2026, 10, 21, 6), 9.9312, 76.2673) {
            if r.every_days == 7 && r.weekday_iso > 0 {
                assert!(r.keep_words.contains(&format!("{} ", r.times)), "{}", r.keep_words);
                assert_eq!(r.days, (r.times - 1) * 7 + 1);
                assert!(r.until_words.starts_with("until "));
            }
        }
    }

    /// `cargo test -p kshana-ffi show_schedule -- --ignored --nocapture` prints a sample.
    #[test]
    #[ignore]
    fn show_schedule() {
        let e = engine();
        for r in schedules(&e, Some("Career"), at(2026, 10, 21, 6), 28.6139, 77.2090) {
            println!("{} | {} | {} {} ({}) | {}\n  {}\n  {}\n", r.title, r.start_words, r.keep_words, r.until_words, r.keep_basis_words, r.for_stretch_words, r.why_now, r.remedy_id);
        }
    }

    #[test]
    fn words_read_right() {
        let t: Instant = at(2026, 10, 24, 1).into(); // 06:30 IST
        assert_eq!(day_words(t, 330), "Saturday 24 Oct");
        assert_eq!(span_words(t, t.add_ns(58 * 60 * NS_PER_SEC), 330), "6:30–7:28 am");
        let noon: Instant = at(2026, 10, 24, 6).into(); // 11:30 IST
        assert_eq!(span_words(noon, noon.add_ns(60 * 60 * NS_PER_SEC), 330), "11:30 am–12:30 pm");
        assert_eq!(iso(t, 330), "2026-10-24T06:30:00+05:30");
        assert_eq!(unix_ms(t), 1_792_803_600_000);
    }
}
