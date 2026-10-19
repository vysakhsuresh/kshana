//! The karma gate: what a remedy may claim, decided before any remedy is shown.
//!
//! Two taxonomies, both computed from the chart and the running periods:
//!
//! **(A) Sanchita, prarabdha, kriyamana** (the Vedantic store, the portion ripening
//! in this life, and what is being made now). The commentators agree a remedy cannot
//! touch sanchita and only negotiates with prarabdha; kriyamana is entirely in the
//! person's hands. Sivananda says prarabdha can be changed by right action, other
//! acharyas say it cannot; this module states the disagreement and does not pick a
//! side. The chart is a map of prarabdha, so the kind in play is read from the
//! period: a cause whose planet rules a running period is prarabdha ripening now; a
//! natal affliction no period brings is sanchita, stored; a sound planet with no
//! period pressing is kriyamana, where only present conduct counts.
//!
//! **(B) Dridha and adridha** (Prasna Marga ch. 1, stanza 101 ff., B. V. Raman's
//! translation): results flow from dridha (fixed, from deliberate action) and adridha
//! (not fixed, from unintentional action); later commentators add dridha-adridha
//! (mixed). The chart test (ch. 15): the evil karma is dridha or adridha according as
//! the indicator planet stands in the house of the Moon or of the Sun, the Kerala
//! guide reading "house" as the sign or the hora. The hora of a sign follows BPHS
//! ch. 7: in an odd sign the first fifteen degrees are the Sun's hora and the second
//! the Moon's; in an even sign the reverse.
//!
//! The concurrence count is a practitioner extension, not in the text: an affliction
//! shown by the natal yoga, the running dasha and the current transit together is
//! dridha; by one alone, adridha; by two, mixed. The rule used here, so the label is a
//! function of exact facts: with `m` the Moon test, `s` the Sun test and `c` the
//! concurrence out of three, the class is dridha when `m` holds and `c = 3`, adridha
//! when `s` holds and `c ≤ 1`, and dridha-adridha otherwise.
//!
//! What each class lets a remedy claim (the remedy ladder, finding "what each rung may
//! claim"): dridha is endured with discipline, the texts promising no relief and asking
//! for grace and patience; dridha-adridha is softened by remedy after intense and
//! prolonged effort (the full purascharana, repeated vows); adridha is changed by action
//! and "can be destroyed totally" by parihara.

use crate::areas::Area;
use crate::facts::Facts;
use crate::parashari::{dignity, is_benefic, Dignity};
use kshana_vedic::DashaLord;

/// Dridha (fixed), dridha-adridha (mixed) or adridha (not fixed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaClass {
    /// Fixed: deliberate karma; endured with discipline, no relief promised.
    Dridha,
    /// Mixed: softened by remedy after intense and prolonged effort.
    DridhaAdridha,
    /// Not fixed: changed by action; the texts say it can be destroyed totally.
    Adridha,
}

impl KarmaClass {
    /// Sanskrit label.
    pub fn name(self) -> &'static str {
        match self {
            KarmaClass::Dridha => "dridha",
            KarmaClass::DridhaAdridha => "dridha-adridha",
            KarmaClass::Adridha => "adridha",
        }
    }

    /// What the texts allow a remedy to claim for this class, in one line.
    pub fn may_claim(self) -> &'static str {
        match self {
            KarmaClass::Dridha => "endured with discipline: the texts do not promise relief for this; they ask for grace and patience, and conduct is offered as dignity, not as a bargain",
            KarmaClass::DridhaAdridha => "softened by remedy: the texts allow relief after intense and prolonged effort, the full dose repeated (whole purascharana, vows kept to their count)",
            KarmaClass::Adridha => "changed by action: the texts say this can be destroyed totally by parihara, and new karma is entirely in your hands",
        }
    }
}

/// Which store the cause is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaKind {
    /// Stored, not ripening in any running period.
    Sanchita,
    /// Ripening now: the planet rules a running period.
    Prarabdha,
    /// Being made now: nothing from the past presses; only present conduct counts.
    Kriyamana,
}

impl KarmaKind {
    /// Sanskrit label.
    pub fn name(self) -> &'static str {
        match self {
            KarmaKind::Sanchita => "sanchita",
            KarmaKind::Prarabdha => "prarabdha",
            KarmaKind::Kriyamana => "kriyamana",
        }
    }
}

/// Sidereal signs of the slow transits at the instant of judgement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitSigns {
    /// Saturn's sidereal sign, 0 = Aries.
    pub saturn: u8,
    /// Jupiter's sidereal sign.
    pub jupiter: u8,
    /// Rahu's sidereal sign (true node).
    pub rahu: u8,
    /// Ketu's sidereal sign.
    pub ketu: u8,
}

/// The verdict: class, kind in play, the evidence and what a remedy may claim.
#[derive(Clone, Debug, PartialEq)]
pub struct KarmaVerdict {
    /// Dridha, dridha-adridha or adridha.
    pub class: KarmaClass,
    /// Sanchita, prarabdha or kriyamana.
    pub kind_in_play: KarmaKind,
    /// Which tests fired, in words the evidence drawer can show.
    pub basis: Vec<String>,
    /// What the texts let a remedy claim for this class.
    pub may_claim: &'static str,
    /// Concurrence out of three (natal yoga, running dasha, current transit).
    pub concurrence: u8,
    /// The text behind the rule.
    pub source: &'static str,
    /// Stable rule id.
    pub rule_id: &'static str,
}

/// Source line for every verdict.
pub const SOURCE: &str = "Prasna Marga ch. 1 st. 101 ff. and ch. 15 (Raman): dridha or adridha as the indicator stands in the Moon's or the Sun's sign or hora; BPHS ch. 7 (hora of a sign); the three-fold concurrence of yoga, dasha and transit is practitioner convention, not the text; sanchita, prarabdha and kriyamana per the Vedantic commentators, who disagree on whether prarabdha can be changed";

/// The Sun's or the Moon's hora of a sidereal longitude (BPHS ch. 7): odd signs give
/// the Sun the first half and the Moon the second; even signs the reverse.
pub fn hora_lord_of(sidereal: f64) -> DashaLord {
    let sign = (sidereal.rem_euclid(360.0) / 30.0).floor() as u32;
    let first_half = sidereal.rem_euclid(30.0) < 15.0;
    let odd = sign % 2 == 0; // Aries is the first (odd) sign
    if odd == first_half {
        DashaLord::Sun
    } else {
        DashaLord::Moon
    }
}

fn is_dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}

fn sign_name(s: u8) -> &'static str {
    [
        "Aries",
        "Taurus",
        "Gemini",
        "Cancer",
        "Leo",
        "Virgo",
        "Libra",
        "Scorpio",
        "Sagittarius",
        "Capricorn",
        "Aquarius",
        "Pisces",
    ][s as usize % 12]
}

/// Classify the karma behind a located cause: the planet `lord` afflicting the area's
/// house (or, with no area, the house it occupies), given the running period lords
/// (maha first) and, when known, the slow transits at the instant.
pub fn classify(
    facts: &Facts,
    lord: DashaLord,
    area: Option<Area>,
    running: &[DashaLord],
    transits: Option<&TransitSigns>,
) -> KarmaVerdict {
    let p = facts.planet(lord);
    let house = area.map(|a| a.primary).unwrap_or(p.rasi_house);
    let house_sign = ((facts.lagna_sign as u16 + house as u16 - 1) % 12) as u8;
    let house_lord = facts.rasi_lords[(house - 1) as usize];
    let mut basis = Vec::new();

    // (B) the Moon / Sun test: sign first, then the hora of the sign
    let (moon_test, sun_test) = match p.sign {
        3 => {
            basis.push(format!("{lord:?} stands in Cancer, the Moon's sign: the Prasna Marga test reads the karma as fixed (dridha-leaning)"));
            (true, false)
        }
        4 => {
            basis.push(format!("{lord:?} stands in Leo, the Sun's sign: the Prasna Marga test reads the karma as not fixed (adridha-leaning)"));
            (false, true)
        }
        s => {
            let h = hora_lord_of(p.sidereal);
            let half = if p.sidereal.rem_euclid(30.0) < 15.0 {
                "first"
            } else {
                "second"
            };
            basis.push(format!(
                "{lord:?} at {:.2}° of {} stands in the {half} half, the {h:?}'s hora of the sign (BPHS ch. 7): the Prasna Marga test reads the karma as {}",
                p.sidereal.rem_euclid(30.0),
                sign_name(s),
                if h == DashaLord::Moon { "fixed (dridha-leaning)" } else { "not fixed (adridha-leaning)" }
            ));
            (h == DashaLord::Moon, h == DashaLord::Sun)
        }
    };

    // concurrence 1: the natal affliction holds
    let d = dignity(lord, p.sidereal);
    let owns_dusthana = p.owns.iter().any(|h| is_dusthana(*h));
    let owns_trikona = p.owns.iter().any(|h| matches!(h, 1 | 5 | 9));
    let with_malefic: Vec<DashaLord> = facts
        .planets
        .iter()
        .filter(|q| q.graha != lord && q.sign == p.sign && !is_benefic(facts, q.graha))
        .map(|q| q.graha)
        .collect();
    let mut natal_reasons = Vec::new();
    if d <= Dignity::Enemy {
        natal_reasons.push(format!("{lord:?} is {d:?} by dignity"));
    }
    if is_dusthana(p.rasi_house) {
        natal_reasons.push(format!(
            "{lord:?} sits in rasi house {}, a dusthana",
            p.rasi_house
        ));
    }
    if owns_dusthana && !owns_trikona {
        natal_reasons.push(format!(
            "{lord:?} owns a dusthana ({:?}) and no trikona: a functional malefic for this lagna",
            p.owns
        ));
    }
    if !with_malefic.is_empty() {
        natal_reasons.push(format!("{lord:?} is conjoined with {with_malefic:?}"));
    }
    let natal = !natal_reasons.is_empty();
    if natal {
        basis.push(format!("natal yoga holds: {}", natal_reasons.join("; ")));
    } else {
        basis.push(format!(
            "no natal affliction: {lord:?} is {d:?} in rasi house {} with no malefic beside it",
            p.rasi_house
        ));
    }

    // concurrence 2: the running dasha
    let runs = running.contains(&lord);
    let house_lord_runs = running.contains(&house_lord);
    if runs {
        basis.push(format!("running dasha: {lord:?} rules a running period"));
    } else if house_lord_runs {
        basis.push(format!(
            "running dasha: {house_lord:?}, lord of the {house}th, rules a running period"
        ));
    } else {
        basis.push(format!(
            "no running period of {lord:?} or of the {house}th lord {house_lord:?}"
        ));
    }
    let running_hit = runs || house_lord_runs;

    // concurrence 3: the slow transits over the planet or the house
    let transit_hit = match transits {
        Some(t) => {
            let mut hits = Vec::new();
            for (name, s) in [
                ("Saturn", t.saturn),
                ("Jupiter", t.jupiter),
                ("Rahu", t.rahu),
                ("Ketu", t.ketu),
            ] {
                if s == p.sign {
                    hits.push(format!("{name} transits {}, over {lord:?}", sign_name(s)));
                } else if s == house_sign {
                    hits.push(format!(
                        "{name} transits {}, the {house}th house",
                        sign_name(s)
                    ));
                }
            }
            if hits.is_empty() {
                basis.push(
                    "no slow transit (Saturn, Jupiter, the nodes) over the planet or the house now"
                        .into(),
                );
                false
            } else {
                basis.push(format!("transit: {}", hits.join("; ")));
                true
            }
        }
        None => {
            basis.push(
                "transits not supplied: the transit leg of the concurrence is not counted".into(),
            );
            false
        }
    };

    let concurrence = natal as u8 + running_hit as u8 + transit_hit as u8;
    basis.push(format!(
        "concurrence {concurrence} of 3 (natal yoga, running dasha, transit)"
    ));

    let class = if moon_test && concurrence == 3 {
        KarmaClass::Dridha
    } else if sun_test && concurrence <= 1 {
        KarmaClass::Adridha
    } else {
        KarmaClass::DridhaAdridha
    };
    basis.push(format!(
        "rule: dridha when the Moon test holds and all three concur; adridha when the Sun test holds and at most one concurs; otherwise dridha-adridha → {}",
        class.name()
    ));

    let kind_in_play = if runs {
        basis.push(format!(
            "prarabdha: the period of {lord:?} is ripening it now"
        ));
        KarmaKind::Prarabdha
    } else if natal {
        basis.push(format!(
            "sanchita: the chart carries it but no running period of {lord:?} brings it now"
        ));
        KarmaKind::Sanchita
    } else {
        basis.push(
            "kriyamana: nothing from the past presses here; what you do now is what counts".into(),
        );
        KarmaKind::Kriyamana
    };

    KarmaVerdict {
        class,
        kind_in_play,
        basis,
        may_claim: class.may_claim(),
        concurrence,
        source: SOURCE,
        rule_id: "KS-R-KARMA-01",
    }
}

/// What a remedy on rung `rung` (1 karma … 7 gem) may claim for a verdict, per the
/// ladder mapping: kriyamana → rungs 1–3 are the remedy; adridha → rungs 1–6 can
/// destroy it; dridha-adridha → rungs 1–6 with the full dose; dridha → rungs 1–3 as
/// endurance and dignity, 4–6 as devotion with no promise; rung 7 is information only.
pub fn may_claim(class: KarmaClass, kind: KarmaKind, rung: u8) -> &'static str {
    if rung >= 7 {
        return "information only: a gem strengthens its planet; nothing here recommends a purchase and no claim is made";
    }
    if kind == KarmaKind::Kriyamana {
        return if rung <= 3 {
            "the remedy itself: conduct, service and discipline are the only things that change new karma"
        } else {
            "support only: nothing from the past presses here, so this adds devotion, not relief"
        };
    }
    match (class, rung <= 3) {
        (KarmaClass::Dridha, true) => "endurance and dignity: the texts do not promise relief for this; they ask for grace and patience",
        (KarmaClass::Dridha, false) => "devotion: the texts do not promise relief for dridha karma; this is offered, not bargained",
        (KarmaClass::DridhaAdridha, true) => "the work itself; relief, the texts say, after intense and prolonged effort",
        (KarmaClass::DridhaAdridha, false) => "softens with the full dose repeated: the whole purascharana, vows kept to their count",
        (KarmaClass::Adridha, true) => "changes it: the texts say an adridha affliction can be destroyed totally",
        (KarmaClass::Adridha, false) => "the texts say it can be destroyed totally by parihara; this supports the conduct above",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PlanetFact;
    use kshana_vedic::sign_lord;
    use DashaLord::*;

    fn planet(
        graha: DashaLord,
        sidereal: f64,
        lagna_sign: u8,
        lords: &[DashaLord; 12],
    ) -> PlanetFact {
        let sign = ((sidereal / 30.0).floor() as usize % 12) as u8;
        let rasi_house = ((sign as i32 - lagna_sign as i32).rem_euclid(12) + 1) as u8;
        let owns = (0..12u8)
            .filter(|i| lords[*i as usize] == graha)
            .map(|i| i + 1)
            .collect();
        PlanetFact {
            graha,
            sidereal,
            house: rasi_house,
            rasi_house,
            sign,
            star_lord: Moon,
            sub_lord: Moon,
            owns,
            retrograde: false,
        }
    }

    /// Aries lagna. Saturn in Cancer (4th) at 20°: the Moon's sign; Mars in Leo 5° (5th,
    /// the Sun's sign); Venus in Scorpio 20° (8th, the Moon's hora of an odd sign);
    /// Jupiter in Taurus 10° (2nd, even sign first half = Moon's hora).
    fn synthetic() -> Facts {
        let lagna_sign = 0u8;
        let rasi_lords: [DashaLord; 12] =
            std::array::from_fn(|i| sign_lord((lagna_sign as usize + i) % 12));
        let cusps: [f64; 12] = std::array::from_fn(|i| (i as f64 * 30.0 + 5.0) % 360.0);
        let p = |g, lon| planet(g, lon, lagna_sign, &rasi_lords);
        Facts {
            planets: vec![
                p(Sun, 10.0),
                p(Moon, 100.0),
                p(Mars, 125.0),
                p(Mercury, 75.0),
                p(Jupiter, 40.0),
                p(Venus, 230.0),
                p(Saturn, 110.0),
                p(Rahu, 230.0),
                p(Ketu, 50.0),
            ],
            cusps,
            cusp_sub_lords: [Moon; 12],
            cusp_star_lords: [Moon; 12],
            house_lords: rasi_lords,
            lagna_sign,
            rasi_lords,
            moon_phase_deg: 90.0,
        }
    }

    #[test]
    fn hora_of_a_sign_follows_bphs_seven() {
        assert_eq!(hora_lord_of(5.0), Sun); // Aries first half
        assert_eq!(hora_lord_of(20.0), Moon); // Aries second half
        assert_eq!(hora_lord_of(35.0), Moon); // Taurus first half
        assert_eq!(hora_lord_of(50.0), Sun); // Taurus second half
        assert_eq!(hora_lord_of(230.0), Sun); // Scorpio is the 8th (even) sign: second half is the Sun's
        assert_eq!(hora_lord_of(215.0), Moon); // Scorpio first half
    }

    #[test]
    fn dridha_needs_the_moon_test_and_full_concurrence() {
        let f = synthetic();
        // Saturn in Cancer, Enemy (natal), running, Saturn transiting Cancer: all three
        let t = TransitSigns {
            saturn: 3,
            jupiter: 0,
            rahu: 0,
            ketu: 6,
        };
        let v = classify(&f, Saturn, None, &[Saturn, Moon], Some(&t));
        assert_eq!(v.concurrence, 3, "{:?}", v.basis);
        assert_eq!(v.class, KarmaClass::Dridha, "{:?}", v.basis);
        assert_eq!(v.kind_in_play, KarmaKind::Prarabdha);
        assert!(v.may_claim.contains("do not promise"));
        assert!(v.basis.iter().any(|b| b.contains("Cancer")));
        // same planet with nothing running and no transit: mixed (Moon test, c = 1)
        let v2 = classify(
            &f,
            Saturn,
            None,
            &[Jupiter],
            Some(&TransitSigns {
                saturn: 9,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v2.concurrence, 1);
        assert_eq!(v2.class, KarmaClass::DridhaAdridha);
        assert_eq!(v2.kind_in_play, KarmaKind::Sanchita);
    }

    #[test]
    fn adridha_needs_the_sun_test_and_little_concurrence() {
        let f = synthetic();
        // Mars in Leo: Sun's sign; Mars owns 1 and 8 (trikona too), Own? Mars in Leo is Friend;
        // not in a dusthana; Rahu not with it → no natal affliction; not running; no transit
        let v = classify(
            &f,
            Mars,
            None,
            &[Jupiter, Mercury],
            Some(&TransitSigns {
                saturn: 9,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v.concurrence, 0, "{:?}", v.basis);
        assert_eq!(v.class, KarmaClass::Adridha, "{:?}", v.basis);
        assert_eq!(v.kind_in_play, KarmaKind::Kriyamana);
        // with the house lord running and a transit, two concur: mixed
        let v2 = classify(
            &f,
            Mars,
            None,
            &[Sun],
            Some(&TransitSigns {
                saturn: 4,
                jupiter: 9,
                rahu: 9,
                ketu: 9,
            }),
        );
        assert_eq!(v2.concurrence, 2, "{:?}", v2.basis);
        assert_eq!(v2.class, KarmaClass::DridhaAdridha);
    }

    #[test]
    fn missing_transits_are_stated_not_assumed() {
        let f = synthetic();
        let v = classify(&f, Venus, Some(crate::areas::AREAS[1]), &[Venus], None);
        assert!(v.basis.iter().any(|b| b.contains("not supplied")));
        assert!(v.concurrence <= 2);
        assert_eq!(v.source, SOURCE);
    }

    #[test]
    fn rung_claims_follow_the_ladder() {
        assert!(may_claim(KarmaClass::Dridha, KarmaKind::Prarabdha, 1).contains("endurance"));
        assert!(may_claim(KarmaClass::Dridha, KarmaKind::Prarabdha, 5).contains("devotion"));
        assert!(
            may_claim(KarmaClass::Adridha, KarmaKind::Prarabdha, 2).contains("destroyed totally")
        );
        assert!(may_claim(KarmaClass::Adridha, KarmaKind::Kriyamana, 5).contains("support only"));
        assert!(may_claim(KarmaClass::DridhaAdridha, KarmaKind::Sanchita, 4).contains("full dose"));
        assert!(
            may_claim(KarmaClass::Adridha, KarmaKind::Prarabdha, 7).contains("information only")
        );
    }
}
