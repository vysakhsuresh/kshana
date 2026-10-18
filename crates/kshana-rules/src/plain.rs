//! Plain words: the one vocabulary the person reads. The engine computes with grahas,
//! houses, periods and stars; the person reads only what those mean for their life:
//! what will happen, when, how strongly, and what to do. Every user-facing sentence in
//! the crate is built from this module, and `jargon` is the test that holds the line.

use crate::gochara::SaturnPhase;
use kshana_vedic::DashaLord;

/// Words a person without astrology would not understand. None of them may appear in
/// a sentence meant for the person. Matched whole-word, case-insensitive.
pub const JARGON: &[&str] = &[
    // the grahas, in English and the common Indian names
    "sun", "moon", "mars", "mercury", "jupiter", "venus", "saturn", "rahu", "ketu", "uranus", "neptune", "pluto",
    "surya", "chandra", "mangal", "kuja", "budha", "shukra", "shani", "sani", "graha", "grahas", "planet", "planets",
    // the frame
    "house", "houses", "bhava", "lagna", "ascendant", "rising sign", "cusp", "sub-lord", "sublord",
    "dasha", "dashas", "mahadasha", "antardasha", "bhukti", "antara", "pratyantar", "vimshottari",
    "transit", "transits", "gochara", "retrograde", "aspect", "aspects", "conjunction", "exalted", "debilitated",
    "nakshatra", "nakshatras", "tara", "hora", "tithi", "karana", "koota", "kootas", "ashtakoota", "ayanamsa",
    "sade sati", "sadesati", "ardhashtama", "ashtama", "dhaiya", "kantaka", "dosha", "manglik", "natal", "zodiac", "sidereal",
    "kp", "significator", "ashtakavarga", "bindu", "bindus", "varga", "navamsa", "rasi", "rashi",
    // the twelve signs
    "aries", "taurus", "gemini", "cancer", "leo", "virgo", "libra", "scorpio", "sagittarius", "capricorn", "aquarius", "pisces",
    // the tara names
    "janma", "sampat", "vipat", "kshema", "pratyak", "pratyari", "sadhana", "sadhaka", "naidhana", "mitra",
];

/// Phrases that contain a jargon word but are plain to anyone.
const ALLOWED: &[&str] = &["full moon", "new moon", "moonrise", "sunrise", "sunset", "sunday", "monday"];

/// The first jargon word in `s`, if any.
pub fn jargon(s: &str) -> Option<&'static str> {
    let mut low = s.to_lowercase();
    for a in ALLOWED {
        low = low.replace(a, " ");
    }
    let bytes = low.as_bytes();
    for w in JARGON {
        let mut from = 0;
        while let Some(i) = low[from..].find(w) {
            let at = from + i;
            let end = at + w.len();
            let before = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
            let after = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
            if before && after {
                return Some(w);
            }
            from = at + 1;
        }
    }
    None
}

/// What a long chapter of life (a major period) is about, as a short noun phrase.
pub fn chapter_theme(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "standing, recognition and your relationship with authority",
        DashaLord::Moon => "home, family, feelings and peace of mind",
        DashaLord::Mars => "effort, property, courage and competition",
        DashaLord::Mercury => "learning, business, communication and paperwork",
        DashaLord::Jupiter => "growth, money, children, teachers and faith",
        DashaLord::Venus => "love, marriage, comfort, vehicles and the good things of life",
        DashaLord::Saturn => "work, duty, discipline and slow but lasting gains",
        DashaLord::Rahu => "ambition, foreign links, technology and sudden turns",
        DashaLord::Ketu => "letting go, inner growth and quiet endings",
    }
}

/// What such a chapter tends to bring, in one or two sentences.
pub fn chapter_prediction(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "You are seen more. Promotions, government work and dealings with seniors decide a lot; pride and quarrels with a father or a boss are the risk.",
        DashaLord::Moon => "Life turns towards home and the people you care for. A move, a change in the household or a new caring role is likely, and your moods run closer to the surface.",
        DashaLord::Mars => "Energy is high and things move by force of will. Property, land, sport and competition come forward; anger, accidents and disputes are the risk.",
        DashaLord::Mercury => "A season of learning, trade and talk. Studies, contracts, writing and small businesses do well; scattered effort and nervous worry are the risk.",
        DashaLord::Jupiter => "Things grow. Money, children, marriage, teachers and good advice arrive; over-trust and over-spending are the risk.",
        DashaLord::Venus => "Comfort and affection rise. Love, marriage, a vehicle, a nicer home and creative work come forward; indulgence and spending are the risk.",
        DashaLord::Saturn => "Rewards come slowly but they last. Hard work, responsibility and patience are asked of you; delays, fatigue and loneliness are the risk.",
        DashaLord::Rahu => "Big wants and sudden openings. Foreign travel, technology, unusual people and quick rises are possible; confusion, shortcuts and anxiety are the risk.",
        DashaLord::Ketu => "Old things fall away. Spiritual interest, research and a quieter life come forward; losses, detachment and lack of direction are the risk.",
    }
}

/// The trouble an influence brings when it presses on a part of life, as a noun phrase.
pub fn pressure(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "clashes with authority and hurt pride",
        DashaLord::Moon => "mood swings, worry and restless sleep",
        DashaLord::Mars => "heat, quarrels, haste and injuries",
        DashaLord::Mercury => "mix-ups in talk, paperwork and accounts",
        DashaLord::Jupiter => "over-promising, over-trust and too much spending",
        DashaLord::Venus => "indulgence, jealousy and spending on comfort",
        DashaLord::Saturn => "delays, heavy duty and tiredness",
        DashaLord::Rahu => "restlessness, confusion and tempting shortcuts",
        DashaLord::Ketu => "detachment, losses and loss of interest",
    }
}

/// The help an influence brings when it supports a part of life, as a noun phrase.
pub fn support(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "recognition and confidence",
        DashaLord::Moon => "calm, goodwill and public support",
        DashaLord::Mars => "energy, courage and drive",
        DashaLord::Mercury => "clear thinking, good talks and deals",
        DashaLord::Jupiter => "luck, wise advice and growth",
        DashaLord::Venus => "affection, comfort and harmony",
        DashaLord::Saturn => "steady effort that pays off",
        DashaLord::Rahu => "sudden openings and bold ambition",
        DashaLord::Ketu => "insight and a clean break from what no longer serves",
    }
}

/// What an hour is good for (the planetary hour, never named).
pub fn hour_nature(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "a good hour to meet officials, seniors and anyone who can approve",
        DashaLord::Moon => "a good hour for family, travel, food and caring work",
        DashaLord::Mars => "a good hour for hard work, sport and repairs; keep away from arguments",
        DashaLord::Mercury => "a good hour for calls, messages, study, writing and trade",
        DashaLord::Jupiter => "a good hour for money, advice, learning, prayer and good beginnings",
        DashaLord::Venus => "a good hour for love, shopping, beauty, art and making peace",
        DashaLord::Saturn => "an hour for routine work, cleaning and patience; start nothing new",
        DashaLord::Rahu | DashaLord::Ketu => "an uncertain hour; finish rather than begin",
    }
}

/// A short name for the hour, for a title: "an hour for money".
pub fn hour_short(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "an hour for authority",
        DashaLord::Moon => "an hour for home",
        DashaLord::Mars => "an hour for effort",
        DashaLord::Mercury => "an hour for talk and trade",
        DashaLord::Jupiter => "an hour for money and blessings",
        DashaLord::Venus => "an hour for love and comfort",
        DashaLord::Saturn => "a slow hour",
        DashaLord::Rahu | DashaLord::Ketu => "an uncertain hour",
    }
}

/// The quality of a day for the person (from the day's star counted from the birth star).
pub fn day_quality(tara: usize) -> &'static str {
    match tara % 9 {
        0 => "a sensitive day: keep things light and avoid big risks",
        1 => "a day that gives: gains and good news come easier",
        2 => "a day to avoid risk: postpone big decisions and travel",
        3 => "a safe, settled day: good for steady work and health",
        4 => "a day of small obstacles: expect delays and keep patience",
        5 => "a day that rewards effort: push what matters",
        6 => "a day to hold back: rest, and do not sign or start",
        7 => "a friendly day: people help you",
        _ => "one of your best days: begin what matters",
    }
}

/// A two-word name for the day's quality, for titles: "a giving day".
pub fn day_short(tara: usize) -> &'static str {
    match tara % 9 {
        0 => "a sensitive day",
        1 => "a giving day",
        2 => "a risky day",
        3 => "a settled day",
        4 => "a day of small delays",
        5 => "a day for effort",
        6 => "a day to hold back",
        7 => "a friendly day",
        _ => "one of your best days",
    }
}

/// Whether the day's quality is kind (giving, settled, effort, friendly, best).
pub fn day_is_kind(tara: usize) -> bool {
    matches!(tara % 9, 1 | 3 | 5 | 7 | 8)
}

/// The long test (Saturn counted from the birth Moon) as a short name. Never the
/// Sanskrit; never the planet.
pub fn long_test(phase: SaturnPhase) -> &'static str {
    match phase {
        SaturnPhase::SadeSatiRising => "the start of a long testing period",
        SaturnPhase::SadeSatiPeak => "the hardest part of a long testing period",
        SaturnPhase::SadeSatiSetting => "the last part of a long testing period",
        SaturnPhase::DhaiyaFourth => "a testing time for home and peace of mind",
        SaturnPhase::DhaiyaEighth => "a testing time for health and sudden changes",
        SaturnPhase::Clear => "no long test running",
    }
}

/// What the long test brings, in plain sentences.
pub fn long_test_prediction(phase: SaturnPhase) -> &'static str {
    match phase {
        SaturnPhase::SadeSatiRising => "Expenses rise, sleep gets lighter and old arrangements start to loosen. Save money and avoid big loans.",
        SaturnPhase::SadeSatiPeak => "The mind feels heavy, work doubles and rewards come late. Health and relationships need care; do not take shortcuts.",
        SaturnPhase::SadeSatiSetting => "Pressure on money and family speech eases step by step. What is true stays with you; the rest falls away.",
        SaturnPhase::DhaiyaFourth => "Home, mother, property and vehicles ask for attention, and peace of mind is harder to hold. Distance or friction at home is common; keep talking and do not sell or buy property in haste.",
        SaturnPhase::DhaiyaEighth => "Sudden changes, health scares and hidden problems are more likely. Get check-ups, drive carefully and keep money in reserve.",
        SaturnPhase::Clear => "No long pressure is on you; effort pays at its normal rate.",
    }
}

/// Capitalise the first letter.
pub fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A period level as a plain noun: 0 the long chapter, 1 the stretch inside it, 2 the
/// season, 3 these weeks, 4 these days.
pub fn level_noun(level: usize) -> &'static str {
    match level {
        0 => "long chapter of your life",
        1 => "stretch inside it",
        2 => "season of a few months",
        3 => "run of a few weeks",
        _ => "run of a few days",
    }
}

/// What the person was born with for a part of life (`name`, lower case), from the
/// birth-chart score in `[0, 1]`.
pub fn birth_base(name: &str, score: f64) -> String {
    if score >= 0.6 {
        format!("you were born with real strength in {name}: it recovers well from bad patches")
    } else if score >= 0.45 {
        format!("you were born with an average base for {name}: results follow the effort you put in")
    } else {
        format!("{name} is a weaker point from birth: it needs steady care all your life")
    }
}

/// Roughly how long a passing influence lasts, by the moving body behind it.
pub fn influence_span(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Moon => "a couple of days",
        DashaLord::Sun | DashaLord::Mercury | DashaLord::Venus => "a few weeks",
        DashaLord::Mars => "about six weeks",
        DashaLord::Jupiter => "about a year",
        DashaLord::Saturn => "over two years",
        DashaLord::Rahu | DashaLord::Ketu => "about eighteen months",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vocabulary_is_itself_plain() {
        for l in [
            DashaLord::Sun,
            DashaLord::Moon,
            DashaLord::Mars,
            DashaLord::Mercury,
            DashaLord::Jupiter,
            DashaLord::Venus,
            DashaLord::Saturn,
            DashaLord::Rahu,
            DashaLord::Ketu,
        ] {
            for s in [chapter_theme(l), chapter_prediction(l), pressure(l), support(l), hour_nature(l), hour_short(l)] {
                assert_eq!(jargon(s), None, "{s}");
            }
        }
        for t in 0..9 {
            assert_eq!(jargon(day_quality(t)), None);
            assert_eq!(jargon(day_short(t)), None);
        }
        for p in [
            SaturnPhase::SadeSatiRising,
            SaturnPhase::SadeSatiPeak,
            SaturnPhase::SadeSatiSetting,
            SaturnPhase::DhaiyaFourth,
            SaturnPhase::DhaiyaEighth,
            SaturnPhase::Clear,
        ] {
            assert_eq!(jargon(long_test(p)), None);
            assert_eq!(jargon(long_test_prediction(p)), None);
        }
    }

    #[test]
    fn the_checker_finds_whole_words_only() {
        assert_eq!(jargon("Saturn in the 4th house"), Some("saturn"));
        assert_eq!(jargon("Sunrise on Sunday, a full moon"), None);
        assert_eq!(jargon("a household chore"), None);
        assert_eq!(jargon("your lordship"), None);
        assert_eq!(jargon("the Moon mahadasha"), Some("moon"));
    }
}
