//! Parihara: remedies that fit the cause, ranked by fit and effort, actions first.
//!
//! The cause is located in the chart and the period: the weakest of the running period
//! lords, or the lord of the area's house. Each graha has a fixed set of remedies drawn
//! from the Jyotisha remedial tradition (Lal Kitab and temple practice for the ritual
//! items, dharma texts for the conduct items). Conduct, service and discipline are
//! always ranked above donation, mantra and pooja, and the ritual items are marked
//! optional: nothing in this module requires a purchase or a priest.

use crate::areas::Area;
use crate::facts::Facts;
use crate::parashari::{dignity, period_quality, Dignity};
use kshana_vedic::DashaLord;

/// What kind of remedy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum RemedyKind {
    Karma,
    Seva,
    Discipline,
    Daana,
    Mantra,
    Pooja,
}

/// One remedy.
#[derive(Clone, Debug, PartialEq)]
pub struct Remedy {
    /// Kind.
    pub kind: RemedyKind,
    /// Short title.
    pub title: String,
    /// Why this, in one or two sentences.
    pub why: String,
    /// Fit score in `[0, 1]`.
    pub fit: f64,
    /// Effort, in words.
    pub effort: &'static str,
    /// True for mantra and pooja: supportive, never required.
    pub optional: bool,
}

/// The remedy plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Parihara {
    /// The graha the plan addresses.
    pub planet: DashaLord,
    /// Why this graha, in words.
    pub cause: String,
    /// Severity in `[0, 1]`: 0 means nothing needs remedy.
    pub severity: f64,
    /// Remedies, strongest fit first.
    pub remedies: Vec<Remedy>,
}

fn table(g: DashaLord) -> Vec<(RemedyKind, &'static str, &'static str, &'static str)> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Karma, "Keep your word to those who depend on you", "The Sun is authority earned by reliability. Do what you said, when you said.", "daily"),
            (Seva, "Serve your father, or an elder man without family", "The Sun's karaka is the father; service to him is the direct remedy.", "weekly"),
            (Discipline, "Be up before sunrise for 40 days", "The Sun is strengthened by meeting it; the sunrise time is computed for your place.", "40 days"),
            (Daana, "Give wheat or jaggery on a Sunday, by hand", "Sun's grains, given in Surya hora, to someone who needs them.", "once"),
            (Mantra, "Aditya Hridayam on Sundays", "Supports the discipline above; not a substitute for it.", "if you wish"),
            (Pooja, "Surya arghya at sunrise", "Optional. Listed so you know it is not hidden.", "not required"),
        ],
        DashaLord::Moon => vec![
            (Karma, "Care for your mother, in person, this week", "The Moon is the mother and the mind; the remedy for a weak Moon is to tend to both.", "weekly"),
            (Seva, "Serve women and children who have no one", "The Moon's people. An hour of help, not money alone.", "weekly"),
            (Discipline, "Fixed sleep and waking times for 40 days", "A steady routine is the Moon's medicine.", "40 days"),
            (Daana, "Give rice, milk or white cloth on a Monday", "The Moon's offerings, in Chandra hora.", "once"),
            (Mantra, "Shiva or Chandra beeja on Mondays", "Optional support.", "if you wish"),
            (Pooja, "Abhisheka on a Monday", "Optional.", "not required"),
        ],
        DashaLord::Mars => vec![
            (Karma, "End one quarrel on your own initiative", "Mars rules conflict; a weak Mars is remedied by courage used for peace.", "this week"),
            (Seva, "Donate blood, or serve those who protect others", "Mars is blood and guardianship.", "once"),
            (Discipline, "No anger spoken aloud for 40 days", "Restraint is the discipline Mars respects.", "40 days"),
            (Daana, "Give red lentils on a Tuesday", "Mars's grain, in Mangala hora.", "once"),
            (Mantra, "Hanuman Chalisa on Tuesdays", "Optional support.", "if you wish"),
            (Pooja, "Mangala shanti", "Optional.", "not required"),
        ],
        DashaLord::Mercury => vec![
            (Karma, "Speak less and only what is true for a week", "Mercury is speech and trade; truthful, measured speech is its remedy.", "7 days"),
            (Seva, "Teach a child or an adult learner for an hour", "Mercury is learning passed on.", "weekly"),
            (Discipline, "No gossip, no idle scrolling, for 40 days", "Mercury weakens with scattered attention.", "40 days"),
            (Daana, "Give green moong on a Wednesday", "Mercury's grain, in Budha hora.", "once"),
            (Mantra, "Vishnu Sahasranama on Wednesdays", "Optional support.", "if you wish"),
            (Pooja, "Budha shanti", "Optional.", "not required"),
        ],
        DashaLord::Jupiter => vec![
            (Karma, "Honour a teacher you have neglected", "Jupiter is the guru; gratitude restores it.", "this week"),
            (Seva, "Feed students, or fund one student's books", "Jupiter is knowledge given freely.", "once"),
            (Discipline, "Read scripture or a serious book daily for 40 days", "Jupiter grows with study.", "40 days"),
            (Daana, "Give turmeric or chana dal on a Thursday", "Jupiter's offerings, in Guru hora.", "once"),
            (Mantra, "Guru beeja on Thursdays", "Optional support.", "if you wish"),
            (Pooja, "Guru shanti", "Optional.", "not required"),
        ],
        DashaLord::Venus => vec![
            (Karma, "Keep a promise made in a relationship", "Venus is bonds kept; keeping them is the remedy.", "this week"),
            (Seva, "Serve women, artists or the sick with your skill", "Venus is grace given.", "weekly"),
            (Discipline, "Simplify one indulgence for 40 days", "Venus weakens with excess.", "40 days"),
            (Daana, "Give curd or white sweets on a Friday", "Venus's offerings, in Shukra hora.", "once"),
            (Mantra, "Lakshmi or Shukra beeja on Fridays", "Optional support.", "if you wish"),
            (Pooja, "Shukra shanti", "Optional.", "not required"),
        ],
        DashaLord::Saturn => vec![
            (Karma, "Finish the work you have postponed longest", "Saturn pays completed duty and withholds for avoided duty.", "20 min · daily"),
            (Seva, "Serve the old or the labouring on Saturdays", "Saturn signifies the elderly and workers. An hour of real help.", "1 h · Saturdays"),
            (Discipline, "Rise before the Sun for 40 days", "Discipline is the Saturn remedy that needs no one else.", "40 days"),
            (Daana, "Give black til or a blanket on a Saturday", "By hand, in Shani hora, to a person who needs it.", "once"),
            (Mantra, "Hanuman Chalisa on Saturdays", "Optional. Supports the discipline above.", "if you wish"),
            (Pooja, "Shani shanti at a temple", "Optional. Nothing here requires it.", "not required"),
        ],
        DashaLord::Rahu => vec![
            (Karma, "Stop one shortcut or deception you have been running", "Rahu is the shortcut; refusing it is the remedy.", "this week"),
            (Seva, "Serve the outcast, the sick without family, the foreigner", "Rahu's people.", "weekly"),
            (Discipline, "No intoxicants for 40 days", "Rahu feeds on confusion.", "40 days"),
            (Daana, "Give mustard oil or blue cloth on a Saturday", "Rahu's offerings.", "once"),
            (Mantra, "Durga or Rahu beeja", "Optional support.", "if you wish"),
            (Pooja, "Rahu shanti", "Optional.", "not required"),
        ],
        DashaLord::Ketu => vec![
            (Karma, "Finish something you abandoned", "Ketu is detachment misapplied; completing restores it.", "this week"),
            (Seva, "Feed and care for stray dogs, or serve a renunciate", "Ketu's charges.", "weekly"),
            (Discipline, "Live simply for 40 days: one meal plain, one hour of silence", "Ketu rewards simplicity.", "40 days"),
            (Daana, "Give a blanket or sesame", "Ketu's offerings.", "once"),
            (Mantra, "Ganesha mantra", "Optional support.", "if you wish"),
            (Pooja, "Ketu shanti", "Optional.", "not required"),
        ],
    }
}

/// Build the remedy plan. `period_lords` are the running dasha lords, maha first; `area`
/// narrows the cause to the matter at hand.
pub fn parihara(facts: &Facts, area: Option<Area>, period_lords: &[DashaLord]) -> Parihara {
    // Candidate causes with severity
    let mut candidates: Vec<(DashaLord, f64, String)> = Vec::new();
    for (i, &l) in period_lords.iter().enumerate() {
        let (q, why) = match area {
            Some(a) => period_quality(facts, a, l),
            None => {
                let p = facts.planet(l);
                let d = dignity(l, p.sidereal);
                (
                    d.score(),
                    vec![format!("{l:?} is {d:?} in rasi house {}", p.rasi_house)],
                )
            }
        };
        let level = ["mahadasha", "antardasha", "pratyantar", "sookshma", "prana"]
            .get(i)
            .copied()
            .unwrap_or("period");
        candidates.push((
            l,
            1.0 - q,
            format!("{l:?}, the running {level} lord: {}", why.join("; ")),
        ));
    }
    if let Some(a) = area {
        let lord = facts.rasi_lords[(a.primary - 1) as usize];
        let p = facts.planet(lord);
        let d = dignity(lord, p.sidereal);
        let sev = 1.0 - d.score();
        candidates.push((
            lord,
            sev,
            format!(
                "{lord:?}, lord of the {}th house of {}, is {d:?} in rasi house {}",
                a.primary, a.name, p.rasi_house
            ),
        ));
    }
    let (planet, severity, cause) = candidates
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((DashaLord::Saturn, 0.0, "no running period supplied".into()));
    let cause = if severity < 0.25 {
        format!("{cause}. Nothing here is hostile. The period withholds only what duty leaves undone; no remedy beyond your own effort is required.")
    } else {
        cause
    };
    let p = facts.planet(planet);
    let debilitated = dignity(planet, p.sidereal) == Dignity::Debilitated;
    let mut remedies: Vec<Remedy> = table(planet)
        .into_iter()
        .map(|(kind, title, why, effort)| {
            let base = match kind {
                RemedyKind::Karma => 0.95,
                RemedyKind::Seva => 0.85,
                RemedyKind::Discipline => 0.85,
                RemedyKind::Daana => 0.65,
                RemedyKind::Mantra => 0.45,
                RemedyKind::Pooja => 0.3,
            };
            let fit = (base * (0.5 + 0.5 * severity)
                + if debilitated && kind == RemedyKind::Daana {
                    0.05
                } else {
                    0.0
                })
            .clamp(0.0, 1.0);
            Remedy {
                kind,
                title: title.into(),
                why: why.into(),
                fit,
                effort,
                optional: matches!(kind, RemedyKind::Mantra | RemedyKind::Pooja),
            }
        })
        .collect();
    remedies.sort_by(|a, b| {
        b.fit
            .partial_cmp(&a.fit)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Parihara {
        planet,
        cause,
        severity,
        remedies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_graha_has_a_karma_first_plan() {
        for g in DashaLord::ORDER {
            let t = table(g);
            assert_eq!(t.len(), 6);
            assert_eq!(t[0].0, RemedyKind::Karma);
            assert!(
                t.iter()
                    .filter(|r| matches!(r.0, RemedyKind::Mantra | RemedyKind::Pooja))
                    .count()
                    == 2
            );
        }
    }
}
