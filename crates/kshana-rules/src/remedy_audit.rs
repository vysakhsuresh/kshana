//! The parihara audit: every remedy the app can offer, in every language, checked for
//! the four promises of the trust plan.
//!
//! 1. **Deeds first.** In every list for a person, the first items are deeds, service,
//!    habits or a simple prayer, never a gemstone, a paid rite or a costly gift.
//! 2. **Nothing costing money unless chosen.** Whatever costs real money is marked
//!    `costs_money`, never comes before a free remedy, and a gemstone always says it is
//!    never necessary and only with a qualified astrologer's advice.
//! 3. **Nothing frightening.** No remedy threatens ("if you don't…", "or else", "will
//!    destroy", "curse", "black magic"), in English, Malayalam or Hindi.
//! 4. **Never a promise.** No remedy guarantees, removes all problems or changes a
//!    destiny.
//!
//! And every remedy keeps its source ([`crate::sources`]).

use crate::kundali::dosha_remedies;
use crate::parihara::{costs_money, life_table, table, RemedyKind};
use crate::plain::Lang;
use crate::remedy_tree::{branches, how_it_helps};
use crate::sources;
use kshana_vedic::DashaLord;

/// Threats, in the three languages (matched lower-case, as substrings).
const FEAR: [&[&str]; 3] = [
    &[
        "if you don't", "if you do not", "or else", "otherwise you will", "otherwise it will", "will destroy", "destroy you",
        "curse", "black magic", "evil eye", "punish", "doom", "disaster", "beware", "ruin your", "terrible", "you will suffer",
        "bad luck will",
    ],
    &["ശാപ", "കൂടോത്ര", "ദുർമന്ത്രവാദ", "നശിപ്പിക്കും", "നശിക്കും", "ചെയ്തില്ലെങ്കിൽ", "അല്ലാത്തപക്ഷം", "ദുരന്ത", "ശിക്ഷിക്കും", "കണ്ണേറ്"],
    &["शाप", "श्राप", "काला जादू", "नज़र दोष", "नष्ट", "बर्बाद", "वरना आप", "वरना नुकसान", "वरना बुरा", "नहीं तो", "अन्यथा", "सज़ा देगा", "विनाश", "टोना"],
];

/// Promises no remedy may make.
const PROMISE: [&[&str]; 3] = [
    &[
        "guarantee", "removes all", "remove all", "all your problems", "all problems", "change your destiny", "changes your destiny",
        "changes destiny", "rewrite your", "will fix", "will cure", "100%", "for sure", "will surely", "certainly", "miracle",
        "never fails", "never fail",
    ],
    &["ഉറപ്പായും", "ഗ്യാരണ്ടി", "എല്ലാ പ്രശ്നങ്ങളും", "വിധി മാറ്റും", "അത്ഭുത", "തീർച്ചയായും"],
    &["गारंटी", "पक्का असर", "पक्की गारंटी","सारी समस्याएँ", "सब समस्याएँ", "भाग्य बदल", "क़िस्मत बदल", "किस्मत बदल", "चमत्कार", "ज़रूर ठीक"],
];

fn idx(lang: Lang) -> usize {
    match lang {
        Lang::En => 0,
        Lang::Ml => 1,
        Lang::Hi => 2,
    }
}

/// The first threat or promise in `s`, checking English always (names and loan words)
/// and the reader's own language.
fn bad(lang: Lang, s: &str) -> Option<&'static str> {
    let low = s.to_lowercase();
    let mut lists = vec![FEAR[0], PROMISE[0]];
    if lang != Lang::En {
        lists.push(FEAR[idx(lang)]);
        lists.push(PROMISE[idx(lang)]);
    }
    lists.into_iter().flatten().copied().find(|w| low.contains(&w.to_lowercase()))
}

fn check(lang: Lang, what: &str, s: &str, hits: &mut Vec<String>) {
    if let Some(w) = bad(lang, s) {
        hits.push(format!("{lang:?} {what} [{w}]: {s}"));
    }
}

#[test]
fn the_fear_and_promise_lists_catch_what_they_should() {
    assert_eq!(bad(Lang::En, "If you don't do this, the dosha will destroy you"), Some("if you don't"));
    assert_eq!(bad(Lang::En, "This removes all problems"), Some("removes all"));
    assert_eq!(bad(Lang::Ml, "ഇത് ചെയ്തില്ലെങ്കിൽ ദോഷം"), Some("ചെയ്തില്ലെങ്കിൽ"));
    assert_eq!(bad(Lang::Hi, "यह उपाय करें वरना नुकसान होगा"), Some("वरना नुकसान"));
    assert_eq!(bad(Lang::Hi, "भाग्य बदल देगा"), Some("भाग्य बदल"));
    assert_eq!(bad(Lang::En, "Feed a stray dog every day"), None);
}

#[test]
fn every_tree_remedy_is_calm_promises_nothing_and_keeps_its_source() {
    let mut hits = Vec::new();
    for lang in [Lang::En, Lang::Ml, Lang::Hi] {
        for g in DashaLord::ORDER {
            for b in branches(lang, g) {
                check(lang, "branch.title", &b.title, &mut hits);
                check(lang, "branch.intro", &b.intro, &mut hits);
                for it in &b.items {
                    for (f, s) in [("title", &it.title), ("why", &it.why), ("how", &it.how), ("day", &it.day), ("effort", &it.effort)] {
                        check(lang, &format!("{g:?}.{}.{f}", b.key), s, &mut hits);
                    }
                    assert!(!it.source_keys.is_empty() && it.source_keys.iter().all(|k| sources::known(k)), "{g:?} {}: {:?}", it.title, it.source_keys);
                    assert!(!sources::cite(&it.source_keys, lang).is_empty());
                }
            }
            for src in ["dasha", "gochara", "natal"] {
                check(Lang::En, "how_it_helps", &how_it_helps(g, src, None), &mut hits);
            }
        }
    }
    assert!(hits.is_empty(), "{hits:#?}");
}

#[test]
fn deeds_come_first_and_money_only_when_chosen_in_the_tree() {
    for lang in [Lang::En, Lang::Ml, Lang::Hi] {
        for g in DashaLord::ORDER {
            let b = branches(lang, g);
            // The first branch is deeds, and its first items are free deeds.
            assert_eq!(b[0].key, "deeds");
            assert!(b[0].items.len() >= 3 && b[0].items.iter().all(|i| !i.costs_money), "{g:?}");
            // The tradition (rites, fasts, chants) is last.
            assert_eq!(b.last().unwrap().key, "tradition");
            // The first remedy offered is free, and in every branch nothing that costs
            // money comes before a free remedy.
            let all: Vec<_> = b.iter().flat_map(|x| x.items.iter()).collect();
            assert!(!all[0].costs_money);
            for x in &b {
                assert!(x.items.windows(2).all(|w| !w[0].costs_money || w[1].costs_money), "{g:?} {}", x.key);
                assert!(x.items.iter().any(|i| !i.costs_money), "{g:?} {} has no free remedy", x.key);
            }
            // The tradition's intro says a gemstone is never necessary, and only with advice.
            let intro = &b.last().unwrap().intro;
            let (never, advice) = match lang {
                Lang::En => ("never necessary", "qualified astrologer's advice"),
                Lang::Ml => ("ഒരിക്കലും ആവശ്യമില്ല", "യോഗ്യനായ ജ്യോതിഷിയുടെ ഉപദേശ"),
                Lang::Hi => ("कभी ज़रूरी नहीं", "योग्य ज्योतिषी की सलाह"),
            };
            assert!(intro.contains(never) && intro.contains(advice), "{lang:?}: {intro}");
        }
    }
    // The paid rites are marked: the full temple rites and school fees.
    let en: Vec<_> = DashaLord::ORDER.iter().flat_map(|g| branches(Lang::En, *g)).flat_map(|b| b.items).collect();
    assert!(en.iter().any(|i| i.costs_money && i.title.contains("full rite")));
    assert!(en.iter().any(|i| i.costs_money && i.title.contains("school")));
    assert!(en.iter().filter(|i| i.costs_money).count() < en.len() / 5, "most remedies are free");
}

#[test]
fn every_parihara_row_is_calm_promises_nothing_and_is_sourced() {
    let mut hits = Vec::new();
    let mut rows = Vec::new();
    for g in DashaLord::ORDER {
        let t = table(g);
        // Deeds first in every planet's own dictionary, and nothing that costs money
        // before the first free remedy.
        assert_eq!(t[0].0, RemedyKind::Karma, "{g:?}");
        assert!(!costs_money(t[0].0, t[0].1));
        assert!(t.iter().filter(|r| r.0 == RemedyKind::Karma && !costs_money(r.0, r.1)).count() >= 3, "{g:?}");
        rows.extend(t);
        rows.extend(crate::daana::alternatives(g));
    }
    for a in crate::areas::AREAS {
        rows.extend(life_table(a.name));
    }
    for r in &rows {
        check(Lang::En, "parihara.title", r.1, &mut hits);
        check(Lang::En, "parihara.why", r.2, &mut hits);
        assert!(!sources::of_remedy(r.0, r.5).is_empty());
    }
    // The paid rites and precious gifts are marked.
    assert!(rows.iter().any(|r| costs_money(r.0, r.1) && r.1.contains("full rite")));
    assert!(rows.iter().any(|r| costs_money(r.0, r.1) && r.1.contains("gold")));
    assert!(costs_money(RemedyKind::Gem, "ruby (information only)"));
    for r in dosha_remedies(true, true, true, true, true, true) {
        check(Lang::En, "match.title", r.title, &mut hits);
        check(Lang::En, "match.why", r.why, &mut hits);
        assert!(!sources::of_remedy(if r.optional { RemedyKind::Pooja } else { RemedyKind::Karma }, r.source).is_empty());
    }
    assert!(hits.is_empty(), "{hits:#?}");
}

#[test]
fn the_plans_for_a_real_chart_put_deeds_first_and_money_last() {
    use crate::areas::AREAS;
    use crate::facts::Facts;
    use crate::parihara::{parihara, parihara_from_facts};
    use kshana_chart::{compute, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;
    use kshana_vedic::Ayanamsa;

    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
    let Ok(spk) = Spk::open(&p) else { return };
    let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
    let at = Utc::new(2026, 10, 21, 6, 0, 0, 0).unwrap().to_instant().unwrap();
    let mut hits = Vec::new();
    for (y, mo, d, h) in [(1990, 10, 29, 23), (1985, 3, 2, 4), (2001, 7, 18, 13)] {
        let t = Utc::new(y, mo, d, h, 6, 0, 0).unwrap().to_instant().unwrap();
        let ch = compute(&spk, None, &ChartInput { instant: t, observer: obs, system: HouseSystem::Placidus, ayanamsa: Ayanamsa::Lahiri, topocentric_moon: false }).unwrap();
        for area in std::iter::once(None).chain(AREAS.iter().map(Some)) {
            let area = area.copied();
            let full = parihara(&spk, None, &obs, &ch, Ayanamsa::Lahiri, at, area).unwrap();
            let quick = parihara_from_facts(&Facts::from_chart(&ch), area, &[]);
            for plan in [&full, &quick] {
                check(Lang::En, "plain", &plan.plain, &mut hits);
                let first = &plan.remedies[0];
                assert!(matches!(first.kind, RemedyKind::Karma | RemedyKind::Seva | RemedyKind::Discipline), "{:?} first: {}", first.kind, first.title);
                assert!(plan.remedies.iter().take(3).all(|r| !r.costs_money));
                assert!(plan.remedies.windows(2).all(|w| !w[0].costs_money || w[1].costs_money), "money before a free remedy");
                for r in &plan.remedies {
                    check(Lang::En, "remedy.title", &r.title, &mut hits);
                    check(Lang::En, "remedy.why", &r.why, &mut hits);
                    assert!(!r.source_keys.is_empty() && r.source_keys.iter().all(|k| sources::source(k).is_some()), "{}", r.title);
                    if r.kind == RemedyKind::Gem {
                        assert!(r.costs_money && r.why.contains("never necessary"), "{}", r.why);
                    }
                    if matches!(r.kind, RemedyKind::Pooja) && r.title.contains("full rite") {
                        assert!(r.costs_money);
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "{hits:#?}");
}
