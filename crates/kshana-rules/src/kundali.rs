//! Two charts together: the Ashtakoota match (Guna Milan), Kuja dosha, the Moon-to-Moon
//! synastry, and how the two stand at an instant and over the months ahead.
//!
//! The eight kootas are the classical tables of Muhurta Chintamani and the Kerala and
//! Tamil matching tradition, keyed to the two Moons (sign and nakshatra). Conventions
//! are stated where schools differ. Nothing here refuses a pair: a low total is read,
//! explained and dated like everything else.

use crate::ask::{answer, Answer, Question};
use crate::briefing::{briefing, Briefing};
use crate::explain::{explain, Relief};
use crate::forecast::{date_words, moment, slow_series, Band, ForecastError, Life, Natal, SIGN_NAMES};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::plain::{chapter_theme, long_test};
use crate::parashari::{dignity, natural_relation, Dignity};
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{nakshatra, sign_lord, Ayanamsa, DashaLord};

/// One koota.
#[derive(Clone, Debug, PartialEq)]
pub struct Koota {
    /// The classical name (Varna … Nadi): a key, never shown to the person.
    pub key: &'static str,
    /// The plain name the person reads.
    pub name: &'static str,
    /// Points given.
    pub points: f64,
    /// Points possible.
    pub max: f64,
    /// Why, in words.
    pub words: String,
    /// Source.
    pub source: &'static str,
}

/// The match of two birth charts.
#[derive(Clone, Debug, PartialEq)]
pub struct Kundali {
    /// Total out of 36.
    pub total: f64,
    /// The eight kootas, in the classical order.
    pub kootas: Vec<Koota>,
    /// `Excellent match`, `Very good`, `Good enough`, `Asks for care`, `Asks for much care`.
    pub verdict: String,
    /// One or two sentences.
    pub headline: String,
    /// Kuja dosha in the first chart.
    pub kuja_a: bool,
    /// Kuja dosha in the second chart.
    pub kuja_b: bool,
    /// Kuja dosha, in words.
    pub kuja_words: String,
    /// "Pisces · Revati" for the first Moon.
    pub moon_a: String,
    /// For the second Moon.
    pub moon_b: String,
    /// The second Moon's house from the first, 1–12.
    pub moon_distance: u8,
    /// What binds, in words.
    pub strengths: Vec<String>,
    /// What rubs, in words.
    pub frictions: Vec<String>,
    /// The classical remedies for the doshas the match carries.
    pub remedies: Vec<DoshaRemedy>,
    /// The verdict for two people already together: never a yes or no, only how much
    /// care the bond asks for.
    pub together_verdict: String,
    /// The headline for two people already together.
    pub together_headline: String,
    /// Every area that asks for care: what it brings, what each can do, the remedies
    /// and the chart reason on request.
    pub care: Vec<BondCare>,
}

/// A remedy for a dosha of the match, from the Vivaha tradition.
#[derive(Clone, Debug, PartialEq)]
pub struct DoshaRemedy {
    /// `Nadi`, `Bhakoot`, `Gana`, `Kuja`, `Graha Maitri`, `Yoni`.
    pub dosha: &'static str,
    /// What to do.
    pub title: &'static str,
    /// Why, in a sentence.
    pub why: &'static str,
    /// Where it comes from.
    pub source: &'static str,
    /// True for ritual items: supportive, never required.
    pub optional: bool,
}

const VIVAHA: &str = "Vivaha prakarana of Muhurta Chintamani and the Kerala and Tamil matching tradition";
const DHARMA_SINDHU: &str = "Dharma Sindhu, Nirnaya Sindhu (dosha parihara)";
const LAL_KITAB: &str = "Lal Kitab";

fn dosha_remedies(nadi: bool, bhakoot: bool, gana: bool, kuja_unbalanced: bool, maitri_low: bool, yoni_low: bool) -> Vec<DoshaRemedy> {
    let mut v = Vec::new();
    if nadi {
        v.push(DoshaRemedy { dosha: "Nadi", title: "Chant the Mahamrityunjaya mantra together, 108 times a day for 40 days", why: "Protects health and the coming of children, the two things this warning touches.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Nadi", title: "Feed and clothe the poor on a Monday, and give rice or grain", why: "The tradition pays this warning down in food and care given to others.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Nadi", title: "Keep the body's rhythm: shared meals, shared sleep hours, no all-nighters", why: "Two people with this warning tire the same way at the same time; a kept routine protects both.", source: "Dharma reading of the dosha", optional: false });
    }
    if bhakoot {
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Lakshmi-Narayana or Gauri-Shankara pooja on a Friday or Monday", why: "Worship of the divine couple is the tradition's way to reconcile two people whose fortunes pull apart.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Give water, rice and white cloth on Mondays, and keep a light Monday fast", why: "Calms the mind of the more anxious partner; a calm mind rides out the hard combination.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Bhakoot", title: "Decide money matters only after sleeping on them, and never during a quarrel", why: "This combination works through money, children and sudden turns; a one-night rule takes the heat out.", source: "Dharma reading of the dosha", optional: false });
    }
    if gana {
        v.push(DoshaRemedy { dosha: "Gana", title: "Gauri pooja, and Durga Saptashati read once a month (or heard)", why: "A gentle and a fiery temper under one roof: Gauri and Durga are the two tempers made one.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Gana", title: "A rule of words: no verdicts after nine at night, and the sharp word apologised for before sleep", why: "The clash of tempers shows as sharp words at the wrong hour; the remedy is a rule about hours and words.", source: "Dharma reading of the dosha", optional: false });
    }
    if kuja_unbalanced {
        v.push(DoshaRemedy { dosha: "Kuja", title: "Hanuman Chalisa on Tuesdays, and red lentils given", why: "Cools the heat, haste and quarrels one partner brings into the home.", source: DHARMA_SINDHU, optional: true });
        v.push(DoshaRemedy { dosha: "Kuja", title: "Mangala Gauri vrata on the Tuesdays of Shravana, or any four Tuesdays", why: "The fast the tradition prescribes for the hot-tempered partner, kept for the marriage.", source: VIVAHA, optional: false });
        v.push(DoshaRemedy { dosha: "Kuja", title: "Burn the heat with the hands: hard exercise, repairs, a kitchen garden", why: "Energy spent on effort is not spent on the partner.", source: LAL_KITAB, optional: false });
        v.push(DoshaRemedy { dosha: "Kuja", title: "The tradition also names a Kumbha vivaha (a symbolic first wedding) before the marriage; many families keep it, many do not", why: "Stated for completeness; it is a rite of the family's choosing, not a requirement.", source: VIVAHA, optional: true });
    }
    if maitri_low {
        v.push(DoshaRemedy { dosha: "Graha Maitri", title: "Each of you does one small kind act a week in the way the other likes, without being asked", why: "When minds pull apart, serving the other's way of seeing things brings them back together.", source: LAL_KITAB, optional: false });
        v.push(DoshaRemedy { dosha: "Graha Maitri", title: "Say the hard thing in writing first, then talk", why: "When the minds pull apart, the written word gives each mind its own hour to understand.", source: "Dharma reading of the dosha", optional: false });
    }
    if yoni_low {
        v.push(DoshaRemedy { dosha: "Yoni", title: "Rati-Kama or Uma-Maheshwara worship on a Friday", why: "For physical ease between the two of you; the tradition turns to the divine pair for it.", source: VIVAHA, optional: true });
        v.push(DoshaRemedy { dosha: "Yoni", title: "Unhurried time without a screen, at the same hour, twice a week", why: "Bodies and habits that must learn each other need kept time, not a rite.", source: "Dharma reading of the dosha", optional: false });
    }
    v
}

/// One part of the match that asks for care, read for two people who share (or will
/// share) a life: what it brings day to day, what each can do for the other, and the
/// remedies, deeds first and prayer as a choice. Nothing here says "do not marry": it
/// says where to take care and how.
#[derive(Clone, Debug, PartialEq)]
pub struct BondCare {
    /// The engine's key (`Nadi`, `Bhakoot`, `Gana`, `Kuja`, `Graha Maitri`, `Yoni`,
    /// `Tara`, `Vashya`, `Varna`, `Venus`); never shown.
    pub key: &'static str,
    /// The plain name of the area ("Health and children").
    pub area: String,
    /// What it tends to bring when two such people live together, in plain words.
    pub living: String,
    /// What each of you can do for the other. "The first person" and "The second
    /// person" are replaced by names on the screen.
    pub care: Vec<String>,
    /// The remedies for this area: deeds and giving first, prayer and rites marked
    /// optional.
    pub remedies: Vec<DoshaRemedy>,
    /// The chart reason, for the person who taps "how this helps": which part of the
    /// two charts causes it and why the remedies ease it. Names planets; shown only on
    /// request.
    pub how_it_helps: String,
}

/// A prayer of the person's own faith, offered for every area so no one is left with
/// only one tradition's rites.
fn own_faith(dosha: &'static str, title: &'static str, why: &'static str) -> DoshaRemedy {
    DoshaRemedy { dosha, title, why, source: "Faith-neutral prayer", optional: true }
}

/// Every area of the match that asks for care, heaviest first.
#[allow(clippy::too_many_arguments)]
fn bond_care(
    kootas: &[Koota],
    all: &[DoshaRemedy],
    a: &Natal,
    b: &Natal,
    na: usize,
    nb: usize,
    d: u8,
    la: DashaLord,
    lb: DashaLord,
    ka: bool,
    kb: bool,
    vd: u8,
) -> Vec<BondCare> {
    let pts = |key: &str| kootas.iter().find(|k| k.key == key).map(|k| k.points).unwrap_or(0.0);
    let take = |key: &'static str| -> Vec<DoshaRemedy> { all.iter().filter(|r| r.dosha == key).cloned().collect() };
    let mut out = Vec::new();

    // Nadi: the heaviest.
    if pts("Nadi") == 0.0 {
        let mut r = take("Nadi");
        r.push(own_faith("Nadi", "Pray together, in your own faith, for each other's health, once a week", "Holding the other's health in mind each week keeps the care in the habit."));
        out.push(BondCare {
            key: "Nadi",
            area: "Health and children".into(),
            living: "You two have the same kind of body and rhythm. You tend to tire, fall ill and recover at the same times, so it is easy for both of you to be low together. The care this asks for is about health, and about pregnancy and children if you plan them. It is a call for care, not a sentence: many couples with this live long, healthy lives together.".into(),
            care: vec![
                "Look after each other's health without waiting to be asked: when one of you is unwell, the other takes over, even when tired.".into(),
                "Keep shared meal and sleep times, and do not both work late in the same week.".into(),
                "A yearly health check for both of you, on the same day, as a small ritual of care.".into(),
                "If you plan a child, see a doctor early and keep every check-up; the old warning is about taking care, not about fate.".into(),
            ],
            remedies: r,
            how_it_helps: format!(
                "Nadi compares your two birth-stars ({} and {}). Both fall in the {} Nadi, which the old texts tie to the same body type (one of vata, pitta and kapha), so the same strains reach both of you. The Moon rules the body's fluids, health and the mind. Food and clothes given to the poor on a Monday, the Moon's day, is the tradition's way to strengthen the Moon: care given to others comes back as health. The Mahamrityunjaya prayer is addressed to Shiva as the giver of health and long life, the deity the tradition sets over this warning. Keeping a shared rhythm is the practical side of the same remedy.",
                NAKSHATRA_NAMES[na], NAKSHATRA_NAMES[nb], NADI_NAMES[NADI[na] as usize]
            ),
        });
    }

    // Bhakoot: shared fortunes.
    if pts("Bhakoot") == 0.0 {
        let other = match (14 - d as i32).rem_euclid(12) { 0 => 12, x => x };
        let (living, care): (&str, Vec<&str>) = match d {
            2 | 12 => (
                "Money and words are where you two rub. One of you tends to save while the other spends, and words said about money cut deeper than either of you means.",
                vec![
                    "Keep one shared money plan, and a small private allowance each, so neither feels watched.",
                    "Never decide about money during a quarrel; sleep on it and talk in the morning.",
                    "Praise in front of others, correct only in private.",
                ],
            ),
            5 | 9 => (
                "Children, beliefs and plans for the future are where you two rub. You may see faith, teachers or the raising of children differently.",
                vec![
                    "Respect each other's faith and teachers, even where you do not share them.",
                    "Decide matters about children together, never one over the other's head.",
                    "Once a month, talk about the future you each want, and listen to the end.",
                ],
            ),
            _ => (
                "Health, debts and sudden turns are where you two rub. When one of you has a hard time, the other is pulled into it.",
                vec![
                    "Stand by each other in illness and in money trouble; never keep a debt secret.",
                    "Keep some savings for sudden turns, so a shock does not become a quarrel.",
                    "When one of you is down, the other stays steady instead of sinking too.",
                ],
            ),
        };
        let mut r = take("Bhakoot");
        r.push(own_faith("Bhakoot", "Pray together, in your own faith, for the household, on a Friday", "Asking together for the home's good joins two sets of hopes into one."));
        out.push(BondCare {
            key: "Bhakoot",
            area: "Shared fortunes".into(),
            living: format!("{living} This marks where to take care, and it eases with the steps below."),
            care: care.into_iter().map(String::from).collect(),
            remedies: r,
            how_it_helps: format!(
                "Bhakoot measures how far apart the two Moon signs are: yours sit {d} and {other} signs from each other. The texts call this distance hard because each Moon falls in a house of loss, conflict or change from the other. The Moon sign carries a person's mind and fortune, so two fortunes pull in different ways. Water, rice and white cloth given on a Monday, and a light Monday fast, strengthen and calm the Moon. Worship of the divine couple (Lakshmi-Narayana or Gauri-Shankara) is the tradition's way to join two fortunes into one. The money and talk rules work on the place where the strain shows."
            ),
        });
    }

    // Kuja: heat in one chart only.
    if ka != kb {
        let (who, other_who, n) = if ka { ("The first person", "The second person", a) } else { ("The second person", "The first person", b) };
        let mars = n.facts.planet(DashaLord::Mars);
        let from_moon = ((mars.sign as i64 - n.moon_sign()).rem_euclid(12) + 1) as u8;
        let mut r = take("Kuja");
        r.push(own_faith("Kuja", "Pray, in your own faith, for patience, on a Tuesday", "A weekly prayer for patience names the thing to watch and asks for help with it."));
        out.push(BondCare {
            key: "Kuja",
            area: "Heat and quarrels".into(),
            living: format!("{who} brings more drive, haste and temper into the home. Quarrels can flare fast and turn hurtful, and {} may feel run over. Used well, the same fire is energy that protects and builds the home.", other_who.to_lowercase()),
            care: vec![
                format!("{who}: spend the heat on work, exercise and repairs; never make a big decision while angry."),
                format!("{other_who}: do not answer heat with heat. Let it cool, then talk."),
                "Both: one rule, that a quarrel is ended, with an apology from whoever started it, before you sleep.".into(),
            ],
            remedies: r,
            how_it_helps: format!(
                "In {}'s chart, Mars sits in house {} from the rising sign and {} from the Moon: the classic Mars (Kuja) dosha, held by only one of you. Mars is the planet of fire, courage and conflict. Hanuman is the picture of strength under discipline, and red lentils are Mars' grain: given on a Tuesday, Mars' day, they turn Mars' fire into service. Hard physical work uses up the same energy on purpose, so less of it reaches the partner. When both partners carry it, it balances; here it does not, so the remedies matter.",
                if ka { "the first person" } else { "the second person" },
                mars.rasi_house,
                from_moon
            ),
        });
    }

    // Gana: temperament.
    if pts("Gana") <= 1.0 {
        let mut r = take("Gana");
        r.push(own_faith("Gana", "Pray, each in your own faith, for a gentle tongue, every evening", "A short evening prayer ends the day softly, at the very hour when sharp words come."));
        out.push(BondCare {
            key: "Gana",
            area: "Temperament".into(),
            living: "One of you is easy-going and the other quick to anger. Sharp words come, often late in the evening or when tired, and the gentler one may go quiet and store the hurt. Neither temper is wrong; they need a few rules to live together well.".into(),
            care: vec![
                "The quicker one: pause before replying, or walk away for ten minutes rather than say the sharp thing.".into(),
                "The gentler one: say what hurt the same day, calmly, instead of storing it.".into(),
                "Both: no serious talks after nine at night, and an apology before sleep.".into(),
            ],
            remedies: r,
            how_it_helps: format!(
                "Gana sorts the 27 birth-stars into three temperaments: gentle (Deva), human (Manushya) and fiery (Rakshasa). Your birth-stars, {} and {}, are {} and {}. The Moon is the mind, and its star sets the temper. Gauri (the gentle mother) and Durga (the fierce one) are two faces of one Goddess, so their worship is the tradition's picture of two tempers made one. The rule about hours and words works directly where the friction shows.",
                NAKSHATRA_NAMES[na], NAKSHATRA_NAMES[nb], GANA_NAMES[GANA[na] as usize], GANA_NAMES[GANA[nb] as usize]
            ),
        });
    }

    // Graha Maitri: minds.
    if pts("Graha Maitri") <= 1.0 {
        let mut r = take("Graha Maitri");
        r.push(own_faith("Graha Maitri", "Pray, in your own faith, for understanding before a hard talk", "A moment of quiet before talking turns a fight into a conversation."));
        out.push(BondCare {
            key: "Graha Maitri",
            area: "Meeting of minds".into(),
            living: "You think in different ways. What is obvious to one of you can seem strange to the other, so it is easy to feel misunderstood. Learned, the difference becomes a strength: two ways of seeing are wiser than one.".into(),
            care: vec![
                "Ask \"what did you mean?\" before reacting.".into(),
                "Write the hard thing down first, then talk about it.".into(),
                "Once a week, do one thing the other's way, without comment.".into(),
            ],
            remedies: r,
            how_it_helps: format!(
                "Graha Maitri compares the lords of your two Moon signs: {la:?} and {lb:?}, which are not friends by nature. The lord of the Moon sign colours how each mind works, so the two minds pull apart. Doing small things the other's way, and kind acts on the weekday of the other's lord ({} and {}), please both lords and bring the minds closer.",
                weekday_of(lb),
                weekday_of(la)
            ),
        });
    }

    // Yoni: physical ease.
    if pts("Yoni") <= 1.0 {
        let mut r = take("Yoni");
        r.push(own_faith("Yoni", "Pray, in your own faith, for tenderness between you, on a Friday", "Asking for tenderness keeps it in mind for the week."));
        out.push(BondCare {
            key: "Yoni",
            area: "Physical ease".into(),
            living: "Your bodies, rhythms and habits are different: one of you may want closeness when the other wants rest. This can feel like rejection when it is only difference. With patience it becomes ease.".into(),
            care: vec![
                "Talk about what each of you likes, kindly and without blame.".into(),
                "Keep unhurried time together, without phones, twice a week.".into(),
                "Never use closeness as a reward or a punishment.".into(),
            ],
            remedies: r,
            how_it_helps: format!(
                "Yoni pairs each birth-star with an animal nature; yours are the {} and the {}, not natural companions in the classical table. Venus rules pleasure and closeness, and Friday is Venus' day: worship of the divine couple on a Friday is the tradition's way to bring ease. Kept, gentle time does on purpose what the stars do not do by themselves.",
                YONI_NAMES[YONI[na]].to_lowercase(),
                YONI_NAMES[YONI[nb]].to_lowercase()
            ),
        });
    }

    // Tara: luck together.
    if pts("Tara") == 0.0 {
        out.push(BondCare {
            key: "Tara",
            area: "Luck together".into(),
            living: "Big decisions made together in a hurry tend to go wrong for you two. Made calmly, they go well.".into(),
            care: vec![
                "Before a big step, each of you thinks it over alone first, then you decide together.".into(),
                "Begin big things on a good day; the app marks them under Future.".into(),
            ],
            remedies: vec![
                DoshaRemedy { dosha: "Tara", title: "Give food to someone in need on the day you begin anything big together", why: "Starting with a gift turns a shared beginning into a blessed one.", source: VIVAHA, optional: false },
                own_faith("Tara", "Pray together, in your own faith, before a big step", "A shared moment of quiet slows the hurry that causes the trouble."),
            ],
            how_it_helps: "Tara counts from one birth-star to the other in groups of nine. Yours fall on the 3rd, 5th or 7th of the group (the stars of danger, obstacle and loss) both ways. Giving food on a day of beginnings is the classical remedy for a bad tara, and a calm, unhurried start avoids the clash the count warns of.".into(),
        });
    }

    // Vashya: mutual pull.
    if pts("Vashya") == 0.0 {
        out.push(BondCare {
            key: "Vashya",
            area: "Mutual pull".into(),
            living: "Neither of you naturally gives way to the other, so closeness has to be built on purpose rather than left to happen.".into(),
            care: vec![
                "Take turns choosing: this week one decides, next week the other.".into(),
                "Say thank you for small things, every day.".into(),
            ],
            remedies: vec![
                DoshaRemedy { dosha: "Vashya", title: "Do one task a week together that neither of you leads, like cooking or a walk", why: "Shared work without a leader builds the pull the match does not give by itself.", source: "Dharma reading of the dosha", optional: false },
                own_faith("Vashya", "Pray, in your own faith, for each other by name, once a week", "Praying for someone by name keeps them close."),
            ],
            how_it_helps: "Vashya sorts the Moon signs into five kinds (four-footed, human, water, wild and insect) and asks which kind is drawn to the other. Yours are not drawn either way. The remedy is to build the pull on purpose, which is what shared work does.".into(),
        });
    }

    // Venus: taste in love and comfort.
    if matches!(vd, 6 | 8) {
        out.push(BondCare {
            key: "Venus",
            area: "Love and comfort".into(),
            living: "What pleases one of you in love and comfort can puzzle the other. Gifts and gestures may miss, not for lack of love but for lack of knowing.".into(),
            care: vec![
                "Ask what pleases the other; do not assume.".into(),
                "Each of you names one small thing a week that made you feel loved.".into(),
            ],
            remedies: vec![
                DoshaRemedy { dosha: "Venus", title: "On a Friday, give sweets or clothes to a woman in need", why: "Giving comfort to others is the tradition's way to bring comfort home.", source: "Graha daana for Venus", optional: false },
                own_faith("Venus", "Pray, in your own faith, in thanks for each other, on a Friday", "Thankfulness is what keeps affection noticed."),
            ],
            how_it_helps: format!("Your two Venus signs sit {vd} signs apart (a 6-8 distance). Venus rules love, comfort and taste, so your tastes differ. Friday is Venus' day; giving comfort to women on a Friday is the classical gift for Venus, and asking instead of assuming does the same work in daily life."),
        });
    }
    out
}

fn weekday_of(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "Sunday",
        DashaLord::Moon => "Monday",
        DashaLord::Mars => "Tuesday",
        DashaLord::Mercury => "Wednesday",
        DashaLord::Jupiter => "Thursday",
        DashaLord::Venus => "Friday",
        DashaLord::Saturn | DashaLord::Rahu => "Saturday",
        DashaLord::Ketu => "Tuesday",
    }
}

const MC: &str = "Muhurta Chintamani, Vivaha prakarana; Kerala and Tamil matching tables";

/// Varna by Moon sign: 3 Brahmin (water), 2 Kshatriya (fire), 1 Vaishya (earth), 0 Shudra (air).
fn varna(sign: u8) -> u8 {
    match sign % 12 {
        3 | 7 | 11 => 3,
        0 | 4 | 8 => 2,
        1 | 5 | 9 => 1,
        _ => 0,
    }
}

#[allow(dead_code)] // the classical names, kept for the expert layer
const VARNA_NAMES: [&str; 4] = ["Shudra", "Vaishya", "Kshatriya", "Brahmin"];

/// Vashya class by Moon sign and half: 0 Chatushpada, 1 Manava, 2 Jalachara, 3 Vanachara, 4 Keeta.
fn vashya(sidereal: f64) -> usize {
    let sign = ((sidereal / 30.0).floor() as usize) % 12;
    let second_half = sidereal.rem_euclid(30.0) >= 15.0;
    match sign {
        0 | 1 => 0,
        2 | 5 | 6 | 10 => 1,
        3 | 11 => 2,
        4 => 3,
        7 => 4,
        8 => {
            if second_half {
                0
            } else {
                1
            }
        }
        _ => {
            // Capricorn: first half quadruped, second half water
            if second_half {
                2
            } else {
                0
            }
        }
    }
}

#[allow(dead_code)] // the classical names, kept for the expert layer
const VASHYA_NAMES: [&str; 5] = ["Chatushpada", "Manava", "Jalachara", "Vanachara", "Keeta"];

/// Vashya points (convention: the common symmetric table).
fn vashya_points(a: usize, b: usize) -> f64 {
    const T: [[f64; 5]; 5] = [
        [2.0, 1.0, 1.0, 0.0, 1.0],
        [1.0, 2.0, 0.5, 0.5, 0.0],
        [1.0, 0.5, 2.0, 0.0, 1.0],
        [0.0, 0.5, 0.0, 2.0, 0.0],
        [1.0, 0.0, 1.0, 0.0, 2.0],
    ];
    T[a][b]
}

/// Yoni by nakshatra: 0 Horse, 1 Elephant, 2 Sheep, 3 Serpent, 4 Dog, 5 Cat, 6 Rat, 7 Cow,
/// 8 Buffalo, 9 Tiger, 10 Deer, 11 Monkey, 12 Mongoose, 13 Lion.
const YONI: [usize; 27] = [0, 1, 2, 3, 3, 4, 5, 2, 5, 6, 6, 7, 8, 9, 8, 9, 10, 10, 4, 11, 12, 11, 13, 0, 13, 7, 1];
#[allow(dead_code)] // the classical names, kept for the expert layer
const YONI_NAMES: [&str; 14] = [
    "Horse", "Elephant", "Sheep", "Serpent", "Dog", "Cat", "Rat", "Cow", "Buffalo", "Tiger", "Deer", "Monkey",
    "Mongoose", "Lion",
];
/// The classical 14 × 14 yoni table, 4 = same, 0 = sworn enemies.
const YONI_POINTS: [[u8; 14]; 14] = [
    [4, 2, 2, 3, 2, 2, 2, 1, 0, 1, 3, 3, 2, 1],
    [2, 4, 3, 3, 2, 2, 2, 2, 3, 1, 2, 3, 2, 0],
    [2, 3, 4, 2, 1, 2, 1, 3, 3, 1, 2, 0, 3, 1],
    [3, 3, 2, 4, 2, 1, 1, 1, 1, 2, 2, 2, 0, 2],
    [2, 2, 1, 2, 4, 2, 1, 2, 2, 1, 0, 2, 1, 1],
    [2, 2, 2, 1, 2, 4, 0, 2, 2, 1, 3, 3, 2, 1],
    [2, 2, 1, 1, 1, 0, 4, 2, 2, 2, 2, 2, 1, 2],
    [1, 2, 3, 1, 2, 2, 2, 4, 3, 0, 3, 2, 2, 1],
    [0, 3, 3, 1, 2, 2, 2, 3, 4, 1, 2, 2, 2, 1],
    [1, 1, 1, 2, 1, 1, 2, 0, 1, 4, 1, 1, 2, 1],
    [3, 2, 2, 2, 0, 3, 2, 3, 2, 1, 4, 2, 2, 1],
    [3, 3, 0, 2, 2, 3, 2, 2, 2, 1, 2, 4, 3, 2],
    [2, 2, 3, 0, 1, 2, 1, 2, 2, 2, 2, 3, 4, 2],
    [1, 0, 1, 2, 1, 1, 2, 1, 1, 1, 1, 2, 2, 4],
];

/// Gana by nakshatra: 0 Deva, 1 Manushya, 2 Rakshasa.
const GANA: [u8; 27] = [0, 1, 2, 1, 0, 1, 0, 0, 2, 2, 1, 1, 0, 2, 0, 2, 0, 2, 2, 1, 1, 0, 2, 2, 1, 1, 0];
#[allow(dead_code)] // the classical names, kept for the expert layer
const GANA_NAMES: [&str; 3] = ["Deva", "Manushya", "Rakshasa"];

/// Nadi by nakshatra: 0 Aadi, 1 Madhya, 2 Antya.
const NADI: [u8; 27] = [0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0, 1, 2];
#[allow(dead_code)] // the classical names, kept for the expert layer
const NADI_NAMES: [&str; 3] = ["Aadi", "Madhya", "Antya"];

fn moon_words(sidereal: f64) -> String {
    let n = nakshatra(sidereal);
    format!("{} · {}", SIGN_NAMES[((sidereal / 30.0).floor() as usize) % 12], NAKSHATRA_NAMES[n.index])
}

/// Kuja dosha: Mars in 1, 2, 4, 7, 8 or 12 from the lagna or from the Moon (convention:
/// both counted, the 2nd included as in the South). Cancelled when Mars is in its own
/// or exaltation sign.
fn kuja(n: &Natal) -> (bool, String) {
    let mars = n.facts.planet(DashaLord::Mars);
    let from_lagna = mars.rasi_house;
    let from_moon = ((mars.sign as i64 - n.moon_sign()).rem_euclid(12) + 1) as u8;
    let bad = [1u8, 2, 4, 7, 8, 12];
    let hit_l = bad.contains(&from_lagna);
    let hit_m = bad.contains(&from_moon);
    let d = dignity(DashaLord::Mars, mars.sidereal);
    let cancelled = matches!(d, Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted);
    if !(hit_l || hit_m) {
        return (false, "no sign of heat in married life".to_string());
    }
    if cancelled {
        return (false, "a sign of heat in married life, cancelled by its own strength".to_string());
    }
    (true, "a sign of heat, haste and quarrels in married life".to_string())
}

/// Match two charts. `a_is_groom` orients the two one-directional kootas (Varna, and the
/// Deva–Manushya exception of Gana).
pub fn kundali(a: &Natal, b: &Natal, a_is_groom: bool) -> Kundali {
    let ma = a.moon_sidereal;
    let mb = b.moon_sidereal;
    let sa = ((ma / 30.0).floor() as u8) % 12;
    let sb = ((mb / 30.0).floor() as u8) % 12;
    let na = nakshatra(ma).index;
    let nb = nakshatra(mb).index;
    let (g, br) = if a_is_groom { (sa, sb) } else { (sb, sa) };
    let mut kootas = Vec::with_capacity(8);

    // 1. Varna
    let (va, vb) = (varna(g), varna(br));
    let p = if va >= vb { 1.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Varna",
        name: "Outlook on life",
        points: p,
        max: 1.0,
        words: if p > 0.0 {
            "your ways of seeing life fit together; who leads and who supports comes naturally".to_string()
        } else {
            "your ways of seeing life sit in the opposite order to the tradition's; agree early who decides what".to_string()
        },
        source: MC,
    });

    // 2. Vashya
    let (xa, xb) = (vashya(ma), vashya(mb));
    let p = vashya_points(xa, xb);
    kootas.push(Koota {
        key: "Vashya",
        name: "Mutual pull",
        points: p,
        max: 2.0,
        words: {
            match p {
                x if x >= 2.0 => "you are drawn to each other equally; each holds the other easily",
                x if x >= 1.0 => "you are different but get on; the pull is there on both sides",
                x if x > 0.0 => "one of you is drawn more than the other; balance it with attention",
                _ => "neither naturally holds the other; closeness has to be built on purpose",
            }
            .to_string()
        },
        source: MC,
    });

    // 3. Tara
    let ta = (nb + 27 - na) % 27 % 9;
    let tb = (na + 27 - nb) % 27 % 9;
    let good = |t: usize| !matches!(t, 2 | 4 | 6);
    let p = (if good(ta) { 1.5 } else { 0.0 }) + (if good(tb) { 1.5 } else { 0.0 });
    kootas.push(Koota {
        key: "Tara",
        name: "Luck together",
        points: p,
        max: 3.0,
        words: if p >= 3.0 {
            "you bring each other luck: good things tend to happen when you are together"
        } else if p > 0.0 {
            "one of you brings the other luck; the other brings tests"
        } else {
            "you test each other's luck; big decisions go better when made calmly, apart first"
        }
        .to_string(),
        source: MC,
    });

    // 4. Yoni
    let (ya, yb) = (YONI[na], YONI[nb]);
    let p = YONI_POINTS[ya][yb] as f64;
    kootas.push(Koota {
        key: "Yoni",
        name: "Physical ease",
        points: p,
        max: 4.0,
        words: {
            match p as u8 {
                4 => "deep physical ease and closeness",
                3 => "easy, friendly closeness",
                2 => "physical ease comes with time",
                1 => "your bodies and habits must learn each other",
                _ => "physical closeness needs patience and gentleness",
            }
            .to_string()
        },
        source: MC,
    });

    // 5. Graha Maitri
    let (la, lb) = (sign_lord(sa as usize), sign_lord(sb as usize));
    let p = if la == lb {
        5.0
    } else {
        let r1 = natural_relation(la, lb);
        let r2 = natural_relation(lb, la);
        match (r1, r2) {
            (1, 1) => 5.0,
            (1, 0) | (0, 1) => 4.0,
            (0, 0) => 3.0,
            (1, -1) | (-1, 1) => 1.0,
            (0, -1) | (-1, 0) => 0.5,
            _ => 0.0,
        }
    };
    kootas.push(Koota {
        key: "Graha Maitri",
        name: "Meeting of minds",
        points: p,
        max: 5.0,
        words: match p {
            x if x >= 5.0 => "your minds meet: you understand each other without many words",
            x if x >= 4.0 => "your minds meet most days",
            x if x >= 3.0 => "you respect each other's thinking more than you share it",
            x if x >= 1.0 => "one of you gives more understanding than the other",
            _ => "your minds pull in different directions; talk things through slowly",
        }
        .to_string(),
        source: "BPHS, Naisargika maitri; Muhurta Chintamani",
    });

    // 6. Gana
    let (ga, gb) = (GANA[na], GANA[nb]);
    let (gg, gbr) = if a_is_groom { (ga, gb) } else { (gb, ga) };
    let p = match (gg, gbr) {
        (x, y) if x == y => 6.0,
        (0, 1) => 6.0,
        (1, 0) => 5.0,
        (0, 2) | (2, 0) => 1.0,
        _ => 0.0,
    };
    kootas.push(Koota {
        key: "Gana",
        name: "Temperament",
        points: p,
        max: 6.0,
        words: {
            match p as u8 {
                6 | 5 => "your tempers suit each other",
                1 => "one of you is gentle and the other fiery; sharp words come at times",
                _ => "one of you is easy-going and the other fiery; anger needs a rule between you",
            }
            .to_string()
        },
        source: MC,
    });

    // 7. Bhakoot
    let d = ((sb as i64 - sa as i64).rem_euclid(12) + 1) as u8;
    let dosha = matches!(d, 2 | 12 | 5 | 9 | 6 | 8);
    let friends = la == lb || (natural_relation(la, lb) == 1 && natural_relation(lb, la) == 1);
    let p = if !dosha || friends { 7.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Bhakoot",
        name: "Shared fortunes",
        points: p,
        max: 7.0,
        words: if !dosha {
            "money, health and children flow well between you"
        } else if friends {
            "a hard combination for shared fortunes, softened because your natures are friendly"
        } else {
            match d {
                2 | 12 => "money and harsh words need care between you",
                5 | 9 => "children and beliefs need care between you",
                _ => "health and sudden turns need care between you",
            }
        }
        .to_string(),
        source: MC,
    });

    // 8. Nadi
    let (da, db) = (NADI[na], NADI[nb]);
    let same_nadi = da == db;
    let cancelled = same_nadi && ((sa == sb && na != nb) || (na == nb && sa != sb));
    let p = if !same_nadi || cancelled { 8.0 } else { 0.0 };
    kootas.push(Koota {
        key: "Nadi",
        name: "Health and children",
        points: p,
        max: 8.0,
        words: {
            if !same_nadi {
                "your bodies balance each other; health and children are protected"
            } else if cancelled {
                "a classic warning about health and children, cancelled in your case"
            } else {
                "the most serious warning in matching: health and children need care and the remedies below"
            }
            .to_string()
        },
        source: MC,
    });

    let total: f64 = kootas.iter().map(|k| k.points).sum();
    let verdict = if total >= 28.0 {
        "Excellent match"
    } else if total >= 24.0 {
        "Very good"
    } else if total >= 18.0 {
        "Good enough"
    } else if total >= 13.0 {
        "Asks for care"
    } else {
        "Asks for much care"
    };

    let (ka, kwa) = kuja(a);
    let (kb, kwb) = kuja(b);
    let kuja_words = if ka && kb {
        "Both of you carry the same sign of heat in married life, so it balances out: neither burns the other.".to_string()
    } else if ka || kb {
        format!(
            "The {} person carries {}; the other does not. Expect sharper quarrels than usual; the remedies below cool it.",
            if ka { "first" } else { "second" },
            if ka { &kwa } else { &kwb }
        )
    } else {
        format!("First person: {kwa}. Second person: {kwb}.")
    };

    // The heaviest kootas speak first.
    let mut strengths = Vec::new();
    let mut frictions = Vec::new();
    let mut by_weight: Vec<&Koota> = kootas.iter().collect();
    by_weight.sort_by(|a, b| b.max.partial_cmp(&a.max).unwrap_or(std::cmp::Ordering::Equal));
    for k in by_weight {
        if k.points >= k.max * 0.75 && k.max >= 2.0 {
            strengths.push(format!("{}: {}", k.name, k.words));
        } else if k.points <= k.max * 0.25 {
            frictions.push(format!("{}: {}", k.name, k.words));
        }
    }
    // Moon-to-Moon synastry beyond the kootas.
    match d {
        7 => strengths.push("Emotionally you complete each other: the classic bond of life partners.".into()),
        1 => strengths.push("You feel the same moods on the same days: easy to understand, but low days come together.".into()),
        3 | 11 => strengths.push("Friendship and gain flow easily between you.".into()),
        _ => {}
    }
    if favourable_from_moon(DashaLord::Moon).contains(&d) && d != 1 && d != 7 {
        strengths.push("Your moods lift each other more often than they drag.".into());
    }
    let venus_a = a.facts.planet(DashaLord::Venus).sign as i64;
    let venus_b = b.facts.planet(DashaLord::Venus).sign as i64;
    let vd = ((venus_b - venus_a).rem_euclid(12) + 1) as u8;
    if matches!(vd, 1 | 5 | 7 | 9) {
        strengths.push("You share the same taste in love, comfort and pleasure.".into());
    } else if matches!(vd, 6 | 8) {
        frictions.push("What pleases one of you in love and comfort puzzles the other: ask, do not assume.".into());
    }

    let headline = format!(
        "{verdict}: {total:.1} of 36. {}{}",
        if strengths.is_empty() {
            String::new()
        } else {
            format!("Strongest, {}. ", strengths[0].replacen(": ", ": ", 1))
        },
        if frictions.is_empty() {
            "Nothing in the matching stands against you.".to_string()
        } else {
            format!("Hardest, {}.", frictions[0].trim_end_matches('.'))
        }
    );

    let points_of = |name: &str| kootas.iter().find(|k| k.key == name).map(|k| k.points).unwrap_or(0.0);
    let remedies = dosha_remedies(
        points_of("Nadi") == 0.0,
        points_of("Bhakoot") == 0.0,
        points_of("Gana") <= 1.0,
        ka != kb,
        points_of("Graha Maitri") <= 1.0,
        points_of("Yoni") <= 1.0,
    );

    let care = bond_care(&kootas, &remedies, a, b, na, nb, d, la, lb, ka, kb, vd);
    let together_verdict = if total >= 28.0 {
        "A naturally strong bond"
    } else if total >= 24.0 {
        "A strong bond"
    } else if total >= 18.0 {
        "A good bond, with places to tend"
    } else {
        "A bond that grows with care"
    }
    .to_string();
    let together_headline = format!(
        "You already share a life, so this is not a yes or a no. Matching was made for choosing a partner; once two people are together, what matters is how they care for each other. {}{}",
        if strengths.is_empty() {
            String::new()
        } else {
            format!("Your bond is naturally strong in {}. ", strengths[0].split(':').next().unwrap_or("").to_lowercase())
        },
        match care.len() {
            0 => "Nothing in the match stands against you.".to_string(),
            1 => "One part of it asks for care; below is what it brings, what each of you can do for the other, and what helps.".to_string(),
            n => format!("{n} parts of it ask for care; below is what each brings, what each of you can do for the other, and what helps."),
        }
    );

    Kundali {
        together_verdict,
        together_headline,
        care,
        total,
        kootas,
        verdict: verdict.to_string(),
        headline,
        kuja_a: ka,
        kuja_b: kb,
        kuja_words,
        moon_a: moon_words(ma),
        moon_b: moon_words(mb),
        moon_distance: d,
        strengths,
        frictions,
        remedies,
    }
}

/// Two people at one instant and over the months around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Couple {
    /// The instant.
    pub at: Instant,
    /// The first person's Love score now.
    pub a_love: f64,
    /// The second person's.
    pub b_love: f64,
    /// The pair's score now, `[0, 1]`: the two Loves and the match.
    pub together: f64,
    /// Band of `together`.
    pub band: Band,
    /// Two or three sentences with dates.
    pub headline: String,
    /// Saturn relative to each Moon.
    pub a_saturn: SaturnPhase,
    /// For the second person.
    pub b_saturn: SaturnPhase,
    /// Plain notes about the two skies.
    pub notes: Vec<String>,
    /// The first month the pair climbs a band: `(instant, score)`.
    pub eases: Option<(Instant, f64)>,
    /// The best month of the window.
    pub peak: Option<(Instant, f64)>,
    /// The lowest month of the window.
    pub low: Option<(Instant, f64)>,
    /// Monthly slow samples, twelve months back to twenty-four ahead: `(at, a, b, together)`.
    pub series: Vec<(Instant, f64, f64, f64)>,
    /// What lifts the first person's love next.
    pub a_relief: Vec<Relief>,
    /// What lifts the second person's.
    pub b_relief: Vec<Relief>,
    /// What drags the first person's love now, in words.
    pub a_cause: String,
    /// The second's.
    pub b_cause: String,
    /// The first person's briefing: Moon, periods, Saturn, the moment, the first lift.
    pub a_briefing: Briefing,
    /// The second's.
    pub b_briefing: Briefing,
    /// The pair's own paragraphs: what the two seasons mean together.
    pub together_words: Vec<String>,
}

/// The engine speaks to the person whose chart it reads; the second chart's sentences
/// are turned to the third person so a screen can show both side by side.
pub fn theirs(s: &str) -> String {
    s.replace("Your birth Moon", "Their birth Moon")
        .replace("You are in", "They are in")
        .replace("your Moon", "their Moon")
        .replace("your love", "their love")
        .replace("your birth", "their birth")
        .replace("your strongest", "their strongest")
        .replace("your chart", "their chart")
        .replace("You ", "They ")
        .replace(" you ", " them ")
        .replace("your ", "their ")
}

fn third_person_briefing(b: &mut Briefing) {
    b.saturn_words = theirs(&b.saturn_words);
    for p in b.paragraphs.iter_mut() {
        *p = theirs(p);
    }
    for c in b.circled.iter_mut() {
        c.words = theirs(&c.words);
    }
    if let Some(r) = b.remedy.as_mut() {
        r.cause = theirs(&r.cause);
    }
}

fn together_of(a: f64, b: f64, match_total: f64) -> f64 {
    (0.65 * (a + b) / 2.0 + 0.35 * (match_total / 36.0)).clamp(0.0, 1.0)
}

/// The pair at `at`, both standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn couple(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    match_total: f64,
    at: Instant,
) -> Result<Couple, ForecastError> {
    let lon = obs.longitude_deg;
    let ma = moment(spk, eop, a, obs, system, ayanamsa, at)?;
    let mb = moment(spk, eop, b, obs, system, ayanamsa, at)?;
    let a_love = ma.lives[0].score;
    let b_love = mb.lives[0].score;
    let together = together_of(a_love, b_love, match_total);
    let band = Band::of(together);

    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let start = at.add_ns(-12 * month);
    let sa = slow_series(spk, eop, a, obs, ayanamsa, start, month, 37)?;
    let sb = slow_series(spk, eop, b, obs, ayanamsa, start, month, 37)?;
    let series: Vec<(Instant, f64, f64, f64)> = sa
        .iter()
        .zip(sb.iter())
        .map(|((t, va), (_, vb))| (*t, va[0], vb[0], together_of(va[0], vb[0], match_total)))
        .collect();
    let now_slow = series
        .iter()
        .min_by_key(|s| (s.0.ns_since(at)).abs())
        .map(|s| s.3)
        .unwrap_or(together);
    let band_slow = Band::of(now_slow).max(band);
    let ahead: Vec<&(Instant, f64, f64, f64)> = series.iter().filter(|s| s.0 > at).collect();
    let eases = ahead
        .iter()
        .find(|s| Band::of(s.3) > band_slow)
        .map(|s| (s.0, s.3));
    let peak = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v >= s.3 => acc,
            _ => Some((s.0, s.3)),
        });
    let low = ahead
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, s| match acc {
            Some((_, v)) if v <= s.3 => acc,
            _ => Some((s.0, s.3)),
        });

    let ea = explain(spk, eop, a, obs, system, ayanamsa, Life::Love, at)?;
    let eb = explain(spk, eop, b, obs, system, ayanamsa, Life::Love, at)?;
    let lifts = |e: &crate::explain::Explanation| -> Vec<Relief> {
        let mut v: Vec<Relief> = e
            .relief
            .iter()
            .filter(|r| r.gain >= 0.01 && r.lasting)
            .cloned()
            .collect();
        v.truncate(3);
        v
    };
    let a_relief = lifts(&ea);
    let mut b_relief = lifts(&eb);
    for r in b_relief.iter_mut() {
        r.what = theirs(&r.what);
    }
    let lasting = |e: &crate::explain::Explanation| -> String {
        e.causes
            .iter()
            .find(|c| matches!(c.kind, "dasha" | "gochara"))
            .map(|c| c.what.clone())
            .unwrap_or_default()
    };
    let a_cause = lasting(&ea);
    let b_cause = theirs(&lasting(&eb));

    let mut notes = Vec::new();
    match (ma.saturn, mb.saturn) {
        (SaturnPhase::Clear, SaturnPhase::Clear) => notes.push("Neither of you is in a long testing period now.".into()),
        (x, SaturnPhase::Clear) => notes.push(format!(
            "The first person is in {}; the second is not. The weight is one-sided, so the other can carry the pair through it.",
            long_test(x)
        )),
        (SaturnPhase::Clear, y) => notes.push(format!(
            "The second person is in {}; the first is not. The weight is one-sided, so the other can carry the pair through it.",
            long_test(y)
        )),
        (x, y) => notes.push(format!(
            "Both of you are under pressure at once ({}; {}). A shared hard time passes, and it binds those who stay.",
            long_test(x),
            long_test(y)
        )),
    }
    if !a_cause.is_empty() {
        notes.push(format!("For the first person's love just now: {a_cause}."));
    }
    if !b_cause.is_empty() {
        notes.push(format!("For the second person's love just now: {b_cause}."));
    }
    if let (Some(pa), Some(pb)) = (ma.dasha.first(), mb.dasha.first()) {
        notes.push(format!(
            "The first person's life is centred on {}; the second's on {}.",
            chapter_theme(pa.lord),
            chapter_theme(pb.lord)
        ));
    }

    let mut headline = format!(
        "Together you score {:.0} now ({}): love is {:.0} for the first person and {:.0} for the second.",
        together * 100.0,
        band.word(),
        a_love * 100.0,
        b_love * 100.0
    );
    if band <= Band::Steady {
        let worst = if a_love <= b_love { &a_cause } else { &b_cause };
        if !worst.is_empty() {
            headline.push_str(&format!(" The heavier side: {worst}."));
        }
        let first = a_relief.iter().chain(b_relief.iter()).min_by_key(|r| r.at.tai_ns());
        if let Some(r) = first {
            headline.push_str(&format!(" The first lift comes on {}: {}.", date_words(r.at, lon), r.what));
        }
        if let Some((t, s)) = eases {
            headline.push_str(&format!(" The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)));
        } else if let Some((t, _)) = peak {
            headline.push_str(&format!(" The best month in the next two years is around {}.", date_words(t, lon)));
        }
    } else if let Some((t, s)) = low {
        if Band::of(s) < Band::of(now_slow).min(band) {
            headline.push_str(&format!(" Hold what you have around {}, the lowest month ahead.", date_words(t, lon)));
        }
    }

    // Each person's briefing for love, and the pair's own paragraphs.
    let a_briefing = briefing(spk, eop, a, obs, system, ayanamsa, &[Life::Love], at)?;
    let mut b_briefing = briefing(spk, eop, b, obs, system, ayanamsa, &[Life::Love], at)?;
    third_person_briefing(&mut b_briefing);
    let mut together_words = Vec::new();
    together_words.push(format!(
        "You are living two different seasons at once. {} {} {}",
        match (ma.dasha.first(), mb.dasha.first()) {
            (Some(pa), Some(pb)) => format!(
                "The first person's life is about {} until {}; the second's is about {} until {}.",
                chapter_theme(pa.lord),
                date_words(pa.end, lon),
                chapter_theme(pb.lord),
                date_words(pb.end, lon)
            ),
            _ => String::new(),
        },
        match (ma.saturn, mb.saturn) {
            (SaturnPhase::Clear, SaturnPhase::Clear) => "Neither of you is in a long testing period now.".to_string(),
            (x, SaturnPhase::Clear) => format!("The first person is in {}; the second is free of it and can carry the pair.", long_test(x)),
            (SaturnPhase::Clear, y) => format!("The second person is in {}; the first is free of it and can carry the pair.", long_test(y)),
            (x, y) => format!("Both of you are in a hard time ({}; {}): a shared trial, which binds those who stay.", long_test(x), long_test(y)),
        },
        match (a_briefing.saturn_until, b_briefing.saturn_until) {
            (Some(ta), _) if ma.saturn != SaturnPhase::Clear => format!("The first person's testing time ends {}.", date_words(ta, lon)),
            (_, Some(tb)) if mb.saturn != SaturnPhase::Clear => format!("The second person's testing time ends {}.", date_words(tb, lon)),
            _ => String::new(),
        }
    ));
    let d = ((b.moon_sign() - a.moon_sign()).rem_euclid(12) + 1) as u8;
    together_words.push(format!(
        "{} Love scores {:.0} for the first person and {:.0} for the second right now; as a pair, with your match of {:.1} out of 36 counted in, you score {:.0}. {}",
        match d {
            1 => "You feel the same moods on the same days.",
            7 => "Emotionally you complete each other, the classic bond of partners.",
            3 | 11 => "Friendship and gain flow easily between you.",
            2 | 12 => "Money and harsh words need care between you.",
            5 | 9 => "Children and beliefs need care between you.",
            6 | 8 => "Health and sudden turns need care between you.",
            _ => "Your bond takes effort rather than ease, but it builds.",
        },
        a_love * 100.0,
        b_love * 100.0,
        match_total,
        together * 100.0,
        if a_love < b_love {
            format!("The wait is on the first person's side: {}.", a_cause)
        } else if b_love < a_love {
            format!("The wait is on the second person's side: {}.", b_cause)
        } else {
            "You both carry the same weight today.".to_string()
        }
    ));
    let first_lift = a_relief.iter().map(|r| (r, "first")).chain(b_relief.iter().map(|r| (r, "second"))).min_by_key(|(r, _)| r.at.tai_ns());
    if let Some((r, who)) = first_lift {
        together_words.push(format!(
            "The first lasting lift for the pair comes on {}, on the {who} person's side: {} ({:+.0} points). {}",
            date_words(r.at, lon),
            r.what,
            r.gain * 100.0,
            match (eases, peak) {
                (Some((t, s)), _) => format!("The pair climbs to {} from {}.", Band::of(s).word(), date_words(t, lon)),
                (None, Some((t, _))) => format!("The best month in the next two years is around {}.", date_words(t, lon)),
                _ => String::new(),
            }
        ));
    }

    Ok(Couple {
        at,
        a_love,
        b_love,
        together,
        band,
        headline,
        a_saturn: ma.saturn,
        b_saturn: mb.saturn,
        notes,
        eases,
        peak,
        low,
        series,
        a_relief,
        b_relief,
        a_cause,
        b_cause,
        a_briefing,
        b_briefing,
        together_words,
    })
}

/// The talk window: the first hour in the next `days` when both charts read well for
/// love at once (both hora fits at or above 0.55, outside the cautions, both daily
/// love scores at or above 0.5). `(start, end, mean fit)`.
#[allow(clippy::too_many_arguments)]
pub fn talk_window(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    from: Instant,
    days: usize,
) -> Result<Option<(Instant, Instant, f64)>, ForecastError> {
    let sa = crate::forecast::series(spk, eop, a, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    let sb = crate::forecast::series(spk, eop, b, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    for d in 0..days.max(1) {
        if sa[d].scores[0] < 0.5 || sb[d].scores[0] < 0.5 {
            continue;
        }
        let t = from.add_ns(d as i128 * NS_PER_DAY + 6 * 3_600 * kshana_time::NS_PER_SEC);
        let ma = moment(spk, eop, a, obs, system, ayanamsa, t)?;
        let mb = moment(spk, eop, b, obs, system, ayanamsa, t)?;
        for (ha, hb) in ma.hours.iter().zip(mb.hours.iter()) {
            if ha.caution || ha.end <= from {
                continue;
            }
            if ha.fit[0] >= 0.55 && hb.fit[0] >= 0.55 {
                return Ok(Some((ha.start.max(from), ha.end, (ha.fit[0] + hb.fit[0]) / 2.0)));
            }
        }
    }
    Ok(None)
}

/// A question about the pair, answered from both charts.
#[derive(Clone, Debug, PartialEq)]
pub struct CoupleAnswer {
    /// The first chart's answer about love.
    pub a: Answer,
    /// The second chart's.
    pub b: Answer,
    /// Combined verdict.
    pub verdict: String,
    /// Combined score, `[0, 1]`.
    pub score: f64,
    /// Combined confidence.
    pub confidence: f64,
    /// Two sentences.
    pub headline: String,
    /// The window both charts open, or the earlier of the two.
    pub when_start: Option<Instant>,
    /// Its end.
    pub when_end: Option<Instant>,
}

/// Ask `question` about love for both charts and read them together.
#[allow(clippy::too_many_arguments)]
pub fn couple_ask(
    spk: &Spk,
    eop: Option<&EopTable>,
    a: &Natal,
    b: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    question: Question,
    at: Instant,
) -> Result<CoupleAnswer, ForecastError> {
    let aa = answer(spk, eop, a, obs, system, ayanamsa, Life::Love, question, at)?;
    let ab = answer(spk, eop, b, obs, system, ayanamsa, Life::Love, question, at)?;
    let score = (0.5 * aa.score + 0.5 * ab.score).clamp(0.0, 1.0);
    let confidence = ((aa.confidence + ab.confidence) / 2.0 - (aa.score - ab.score).abs() * 0.5).clamp(0.3, 0.95);
    // Each chart has answered the question in its own terms; the pair's verdict says
    // whether the two agree, and on whose side the wait is.
    let yes = |v: &str| {
        v.starts_with("Yes")
            || v.starts_with("Take it")
            || v.starts_with("Go ahead")
            || v.starts_with("Within")
            || v.starts_with("Nothing has truly left")
    };
    let (ya, yb) = (yes(&aa.verdict), yes(&ab.verdict));
    let verdict: String = match (ya, yb, question) {
        (true, true, Question::WillItReturn) => "Yes, you come back together".into(),
        (true, true, Question::WillItRecover) => "Yes, it mends between you".into(),
        (true, true, Question::ShouldI) => "Take the step, both of you".into(),
        (true, true, Question::GoodTimeNow) => "Now serves you both".into(),
        (true, true, Question::When) => format!("Both of you: {}", if aa.when_start.map(|t| t.tai_ns()) <= ab.when_start.map(|t| t.tai_ns()) { ab.verdict.to_ascii_lowercase() } else { aa.verdict.to_ascii_lowercase() }),
        (true, true, _) => "Yes, for both of you".into(),
        (true, false, _) => "Yes for you; they need more time".into(),
        (false, true, _) => "Yes for them; you need more time".into(),
        (false, false, Question::WillItReturn) => "Not as it was, for either of you".into(),
        (false, false, Question::GoodTimeNow) => "Not this hour, for either of you".into(),
        (false, false, Question::When) => "Not soon, for either of you".into(),
        (false, false, _) => "Not yet, for either of you".into(),
    };
    let (ws, we) = match (aa.when_start, aa.when_end, ab.when_start, ab.when_end) {
        (Some(s1), Some(e1), Some(s2), Some(e2)) => {
            let s = s1.max(s2);
            let e = e1.min(e2);
            if s <= e {
                (Some(s), Some(e))
            } else if s1 <= s2 {
                (Some(s1), Some(e1))
            } else {
                (Some(s2), Some(e2))
            }
        }
        (Some(s1), Some(e1), _, _) => (Some(s1), Some(e1)),
        (_, _, Some(s2), Some(e2)) => (Some(s2), Some(e2)),
        _ => (None, None),
    };
    let headline = format!(
        "{verdict}. For you: \"{}\" ({:.0}); for them: \"{}\" ({:.0}). {}",
        aa.verdict,
        aa.score * 100.0,
        ab.verdict,
        ab.score * 100.0,
        if (aa.score - ab.score).abs() > 0.15 {
            if aa.score > ab.score {
                "The wait is on their side: their time has to open first."
            } else {
                "The wait is on your side: your time has to open first."
            }
        } else {
            "You both read the same."
        }
    );
    Ok(CoupleAnswer {
        a: aa,
        b: ab,
        verdict,
        score,
        confidence,
        headline,
        when_start: ws,
        when_end: we,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn tables_are_well_formed() {
        for row in YONI_POINTS {
            assert_eq!(row.len(), 14);
        }
        for i in 0..14 {
            assert_eq!(YONI_POINTS[i][i], 4);
            for j in 0..14 {
                assert_eq!(YONI_POINTS[i][j], YONI_POINTS[j][i], "yoni {i} {j}");
            }
        }
        assert_eq!(GANA.iter().filter(|g| **g == 0).count(), 9);
        assert_eq!(GANA.iter().filter(|g| **g == 1).count(), 9);
        assert_eq!(NADI.iter().filter(|g| **g == 0).count(), 9);
        for a in 0..5 {
            for b in 0..5 {
                assert_eq!(vashya_points(a, b), vashya_points(b, a));
            }
        }
    }

    #[test]
    fn a_pair_is_matched_and_read_over_time() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let ba = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let bb = Utc::new(1992, 8, 11, 14, 30, 0, 0).unwrap().to_instant().unwrap();
        let a = Natal::compute(&spk, None, ba, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let b = Natal::compute(&spk, None, bb, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let k = kundali(&a, &b, true);
        assert_eq!(k.kootas.len(), 8);
        assert!((0.0..=36.0).contains(&k.total));
        let max: f64 = k.kootas.iter().map(|k| k.max).sum();
        assert_eq!(max, 36.0);
        assert!(!k.headline.is_empty() && !k.kuja_words.is_empty());
        // matching a chart with itself gives every symmetric koota its maximum except Nadi
        let s = kundali(&a, &a, true);
        assert!(s.kootas.iter().filter(|k| k.key != "Nadi").all(|k| k.points == k.max), "{:?}", s.kootas);
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let c = couple(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, k.total, at).unwrap();
        assert_eq!(c.series.len(), 37);
        assert!((0.0..=1.0).contains(&c.together));
        assert!(!c.headline.is_empty() && !c.notes.is_empty());
        let q = couple_ask(&spk, None, &a, &b, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Question::WillItReturn, at).unwrap();
        assert!(!q.verdict.is_empty());
        assert!((0.3..=0.95).contains(&q.confidence));
        use crate::plain::jargon;
        let mut said: Vec<String> = vec![k.headline.clone(), k.kuja_words.clone(), c.headline.clone(), q.headline.clone(), q.verdict.clone(), c.a_cause.clone(), c.b_cause.clone()];
        said.extend(k.kootas.iter().flat_map(|x| [x.name.to_string(), x.words.clone()]));
        said.extend(k.strengths.iter().chain(k.frictions.iter()).cloned());
        said.extend(k.remedies.iter().flat_map(|r| [r.title.to_string(), r.why.to_string()]));
        said.extend(c.notes.iter().chain(c.together_words.iter()).cloned());
        said.extend(c.a_relief.iter().chain(c.b_relief.iter()).map(|r| r.what.clone()));
        for s in &said {
            assert_eq!(jargon(s), None, "{s}");
        }
    }
}
