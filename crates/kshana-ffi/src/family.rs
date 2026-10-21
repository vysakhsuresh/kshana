//! Family proof over the FFI: the windows of a person's life that their own chart and
//! at least one family chart agree on (see `kshana_rules::family` for the bhavat
//! bhavam reading and the pooling).
//!
//! What the person reads: `relation`, `headline`, `detail`, `confirmed_line`,
//! `confirmed_by`, `strength_words`, `remedy_hint` and `plain`. The names (`person`,
//! `confirmed_names`, `not_read`) are what the family typed; `area` is a key and
//! `expert` the astrologer's reading, shown only behind the chart-pages setting (see
//! `plain_audit` below).

use kshana_rules::family::{family_proof, Kin, Member, ProofWindow};
use kshana_sky::Observer;

use crate::{AyanamsaChoice, Engine, ExactInstant, HouseSystem, KshanaError};

/// A relative for the family proof.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FamilyMemberInput {
    /// The name the family uses.
    pub name: String,
    /// Their relation to the main person, as stored: "Father", "Mother", "Spouse",
    /// "Partner", "Child", "Sibling", or "husband", "wife", "son", "daughter",
    /// "brother", "sister". "Friend" and "Other" are not read.
    pub relation: String,
    /// Birth instant.
    pub birth: ExactInstant,
    /// Birth latitude.
    pub lat: f64,
    /// Birth longitude.
    pub lon: f64,
}

/// One window confirmed by more than one chart.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FamilyProofRecord {
    /// The name of the person whose event it is ("" for the main person).
    pub person: String,
    /// Who it is, plainly: "you", "your father", "your daughter".
    pub relation: String,
    /// Key: "marriage", "child", "career", "home", "gains", "abroad" or "health".
    pub area: String,
    /// Start, to the day.
    pub start: ExactInstant,
    /// End, to the day.
    pub end: ExactInstant,
    /// Where the reading is strongest.
    pub peak: ExactInstant,
    /// "For your father: a likely time for a change or a step up at work."
    pub headline: String,
    /// Dates, the peak and which charts agree.
    pub detail: String,
    /// "Your own chart shows it, and your mother's chart points to the same weeks."
    pub confirmed_line: String,
    /// The confirming charts' people, plainly: "you", "your mother".
    pub confirmed_by: Vec<String>,
    /// Their names, in the same order ("" for the main person), for the initials.
    pub confirmed_names: Vec<String>,
    /// The pooled reading at the peak, 0–100 (for the good windows high is strong,
    /// for the hard ones low is).
    pub strength: f64,
    /// "Three charts agree: very likely."
    pub strength_words: String,
    /// A time of care rather than an opening.
    pub is_hard: bool,
    /// What helps, plainly.
    pub remedy_hint: String,
    /// Expert only: how each confirming chart reads it.
    pub expert: Vec<String>,
    /// Where this comes from (the house of each relative, counting from it, the
    /// periods, and the app's cross-check). Resolve with `source_lines`; never shown as is.
    pub source_keys: Vec<String>,
}

/// The family proof for the main person.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FamilyProofResult {
    /// Your events, each confirmed by at least one family chart, by start.
    pub about_you: Vec<FamilyProofRecord>,
    /// What your family's charts say: each relative's events that your chart confirms.
    pub about_family: Vec<FamilyProofRecord>,
    /// Family charts read.
    pub charts_read: u32,
    /// Names whose relation the family proof cannot read (friends, others).
    pub not_read: Vec<String>,
    /// One plain line on the whole: "Your family's charts agree on 4 times in the next
    /// 12 months."
    pub plain: String,
}

#[uniffi::export]
impl Engine {
    /// Family proof for the next `months` (3–36) from `from`: windows for the main
    /// person (born at `birth`, `birth_lat`/`birth_lon`; `main_is_male` when known)
    /// and for each relative, where the person's own chart and at least one family
    /// chart point to the same weeks. Dates are read on the local date at `lon`.
    #[allow(clippy::too_many_arguments)]
    pub fn family_proof(
        &self,
        birth: ExactInstant,
        birth_lat: f64,
        birth_lon: f64,
        main_is_male: Option<bool>,
        relatives: Vec<FamilyMemberInput>,
        lat: f64,
        lon: f64,
        system: HouseSystem,
        ayanamsa: AyanamsaChoice,
        from: ExactInstant,
        months: u32,
    ) -> Result<FamilyProofResult, KshanaError> {
        let mut members = vec![Member { natal: self.natal(birth, birth_lat, birth_lon, system, ayanamsa)?, kin: None, male: main_is_male }];
        let mut names = vec![String::new()];
        let mut not_read = Vec::new();
        for r in &relatives {
            match Kin::parse(&r.relation) {
                Some((kin, male)) => {
                    members.push(Member { natal: self.natal(r.birth, r.lat, r.lon, system, ayanamsa)?, kin: Some(kin), male });
                    names.push(r.name.clone());
                }
                None => not_read.push(r.name.clone()),
            }
        }
        let obs = Observer { latitude_deg: lat, longitude_deg: lon, height_m: 0.0 };
        let p = family_proof(&self.spk, self.eop.as_ref(), &members, &obs, ayanamsa.into(), from.into(), months as usize).map_err(|e| KshanaError::Engine(format!("{e:?}")))?;
        let record = |w: &ProofWindow| FamilyProofRecord {
            person: names[w.person].clone(),
            relation: members[w.person].words().to_string(),
            area: w.event.key().to_string(),
            start: w.start.into(),
            end: w.end.into(),
            peak: w.peak.into(),
            headline: w.headline.clone(),
            detail: w.detail.clone(),
            confirmed_line: w.confirmed_line.clone(),
            confirmed_by: w.confirmed_by.iter().map(|&r| members[r].words().to_string()).collect(),
            confirmed_names: w.confirmed_by.iter().map(|&r| names[r].clone()).collect(),
            strength: w.pooled * 100.0,
            strength_words: w.strength_words.clone(),
            is_hard: !w.good,
            remedy_hint: w.remedy_hint.clone(),
            expert: w.expert.clone(),
            source_keys: vec!["family".into()],
        };
        let about_you: Vec<FamilyProofRecord> = p.about_you.iter().map(record).collect();
        let about_family: Vec<FamilyProofRecord> = p.about_family.iter().map(record).collect();
        let total = about_you.len() + about_family.len();
        let months = (months as usize).clamp(3, 36);
        let plain = if members.len() < 2 {
            "Add your family's births and the app will look for the times their charts and yours agree on.".to_string()
        } else if total == 0 {
            format!("In the next {months} months no single time stands out in more than one chart; the family's charts each tell their own story for now.")
        } else if total == 1 {
            format!("In the next {months} months your family's charts agree on one time.")
        } else {
            format!("In the next {months} months your family's charts agree on {total} times.")
        };
        Ok(FamilyProofResult { about_you, about_family, charts_read: p.charts_read as u32, not_read, plain })
    }
}

#[cfg(test)]
mod plain_audit {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    /// Fields of these records the person does not read as sentences: what the family
    /// typed, keys, and the expert reading.
    const NOT_SHOWN: &[&str] = &["person", "confirmed_names", "not_read", "area", "expert", "source_keys"];

    fn at(y: i64, mo: u32, d: u32, h: u32) -> ExactInstant {
        instant_from_utc(UtcFields { year: y, month: mo, day: d, hour: h, minute: 0, second: 0, nanos: 0 }).unwrap()
    }

    fn check(what: &str, field: &str, v: &str, hits: &mut Vec<String>) {
        if NOT_SHOWN.contains(&field) {
            return;
        }
        if let Some(w) = kshana_rules::plain::jargon_in(kshana_rules::Lang::En, v) {
            hits.push(format!("{what}.{field} [{w}]: {v}"));
        }
        let low = v.to_lowercase();
        for bad in ["guarantee", "certain", "fate", "death", "will die", "dying"] {
            if low.contains(bad) {
                hits.push(format!("{what}.{field} [{bad}]: {v}"));
            }
        }
    }

    #[test]
    fn family_proof_names_no_astrology() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let Ok(engine) = Engine::open(p.to_string_lossy().into_owned(), None) else { return };
        let (la, lo) = (9.9312, 76.2673);
        let rel = |name: &str, relation: &str, b: ExactInstant| FamilyMemberInput { name: name.into(), relation: relation.into(), birth: b, lat: la, lon: lo };
        let family = vec![
            rel("Raghavan", "Father", at(1958, 11, 2, 1)),
            rel("Lakshmi", "Mother", at(1963, 3, 30, 12)),
            rel("Arun", "Sibling", at(1995, 8, 15, 18)),
            rel("Meera", "Spouse", at(1991, 6, 10, 9)),
            rel("Kiran", "Child", at(2019, 1, 12, 4)),
            rel("Joseph", "Friend", at(1990, 5, 5, 5)),
        ];
        let mut hits = Vec::new();
        let mut windows = 0;
        for male in [Some(true), Some(false), None] {
            let r = engine
                .family_proof(at(1990, 2, 20, 5), la, lo, male, family.clone(), la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, at(2026, 10, 21, 6), 24)
                .unwrap();
            assert_eq!(r.not_read, vec!["Joseph".to_string()]);
            assert_eq!(r.charts_read, 5);
            check("result", "plain", &r.plain, &mut hits);
            for w in r.about_you.iter().chain(&r.about_family) {
                windows += 1;
                eprintln!("{} | {} | {}", w.headline, w.strength_words, w.detail);
                assert!(w.confirmed_by.len() == w.confirmed_names.len() && !w.confirmed_by.is_empty());
                assert!(w.start.tai_seconds < w.end.tai_seconds);
                for (field, v) in [
                    ("relation", w.relation.as_str()),
                    ("headline", &w.headline),
                    ("detail", &w.detail),
                    ("confirmed_line", &w.confirmed_line),
                    ("strength_words", &w.strength_words),
                    ("remedy_hint", &w.remedy_hint),
                    ("person", &w.person),
                    ("area", &w.area),
                ] {
                    check("window", field, v, &mut hits);
                }
                for c in &w.confirmed_by {
                    check("window", "confirmed_by", c, &mut hits);
                }
                if w.is_hard {
                    assert!(!w.remedy_hint.is_empty());
                }
            }
            for w in &r.about_family {
                assert!(w.confirmed_by.iter().any(|c| c == "you"), "{w:?}");
            }
        }
        assert!(windows > 0);
        assert!(hits.is_empty(), "{hits:#?}");
        // No family: an invitation, nothing else.
        let r = engine.family_proof(at(1990, 2, 20, 5), la, lo, None, vec![], la, lo, HouseSystem::Placidus, AyanamsaChoice::Lahiri, at(2026, 10, 21, 6), 12).unwrap();
        assert!(r.about_you.is_empty() && r.about_family.is_empty());
        check("result", "plain", &r.plain, &mut hits);
        assert!(hits.is_empty(), "{hits:#?}");
    }
}
