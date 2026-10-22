//! The observance calendar: Amavasya, Purnima, Ekadashi, Pradosha, Sankranti and the
//! festivals that fall from them, each with its exact instant and the civil day it is
//! observed on by the sunrise rule.

use kshana_ephem::{Body, Spk};
use kshana_events::{new_moons, sun_rise_set, tithi_boundaries, EventError, MICROSECOND_NS};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

use crate::transits::sidereal_ingresses;

/// A calendar observance.
#[derive(Clone, Debug, PartialEq)]
pub struct Observance {
    /// Name, e.g. "Ekadashi (Shukla)".
    pub name: &'static str,
    /// The exact instant the tithi or ingress begins.
    pub exact: Instant,
    /// Sunrise of the civil day on which it is observed (the day whose sunrise falls
    /// within the tithi), when a sunrise is found.
    pub observed_sunrise: Option<Instant>,
    /// Plain-words note.
    pub note: &'static str,
}

const SIGNS: [&str; 12] = [
    "Mesha",
    "Vrishabha",
    "Mithuna",
    "Karka",
    "Simha",
    "Kanya",
    "Tula",
    "Vrischika",
    "Dhanu",
    "Makara",
    "Kumbha",
    "Meena",
];

fn tithi_at(spk: &Spk, t: Instant) -> Result<u8, EventError> {
    let m = apparent(spk, Body::Moon, t)
        .map_err(EventError::Spk)?
        .longitude;
    let s = apparent(spk, Body::Sun, t)
        .map_err(EventError::Spk)?
        .longitude;
    Ok(((m - s).rem_euclid(360.0) / 12.0).floor() as u8 + 1)
}

/// Observances in `[from, from + days]` for an observer (sunrise rule at their place).
pub fn observances(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    from: Instant,
    days: u32,
) -> Result<Vec<Observance>, EventError> {
    let to = from.add_ns(days as i128 * NS_PER_DAY);
    let (rises, _) = sun_rise_set(
        spk,
        eop,
        obs,
        from.add_ns(-NS_PER_DAY),
        to.add_ns(2 * NS_PER_DAY),
        NS_PER_SEC,
    )?;
    let mut out = Vec::new();
    // Tithi-based observances: the instant the tithi begins, observed on the day whose
    // sunrise falls inside it.
    for e in tithi_boundaries(spk, from, to, MICROSECOND_NS)? {
        let start = e.root.t;
        let n = tithi_at(spk, start.add_ns(60 * NS_PER_SEC))?;
        let (name, note) = match n {
            11 => (
                "Ekadashi (Shukla)",
                "Fasting day of the waxing fortnight; observed at the sunrise within it.",
            ),
            26 => (
                "Ekadashi (Krishna)",
                "Fasting day of the waning fortnight; observed at the sunrise within it.",
            ),
            13 | 28 => (
                "Pradosha",
                "Trayodashi; the evening twilight is the observance window.",
            ),
            15 => (
                "Purnima",
                "Full Moon tithi; the exact full Moon is listed separately.",
            ),
            30 => (
                "Amavasya",
                "New Moon tithi; the exact new Moon is listed separately.",
            ),
            4 => (
                "Chaturthi (Shukla)",
                "Ganesha's tithi of the waxing fortnight.",
            ),
            19 => (
                "Sankashti Chaturthi",
                "Fourth tithi of the waning fortnight; observed at moonrise.",
            ),
            8 => ("Ashtami (Shukla)", "Durga's tithi of the waxing fortnight."),
            23 => ("Ashtami (Krishna)", "Kalashtami."),
            _ => continue,
        };
        let next_start = start.add_ns(30 * 3_600 * NS_PER_SEC);
        let observed_sunrise = rises
            .iter()
            .copied()
            .find(|r| *r >= start && *r < next_start && tithi_at(spk, *r).ok() == Some(n));
        out.push(Observance {
            name,
            exact: start,
            observed_sunrise,
            note,
        });
    }
    for e in new_moons(spk, from, to, MICROSECOND_NS)? {
        out.push(Observance {
            name: "New Moon (exact)",
            exact: e.root.t,
            observed_sunrise: None,
            note: "Sun and Moon at the same longitude.",
        });
    }
    for e in kshana_events::full_moons(spk, from, to, MICROSECOND_NS)? {
        out.push(Observance {
            name: "Full Moon (exact)",
            exact: e.root.t,
            observed_sunrise: None,
            note: "Moon opposite the Sun.",
        });
    }
    for (t, s) in sidereal_ingresses(spk, Body::Sun, ayanamsa, from, to).map_err(EventError::Spk)? {
        let name = match s {
            0 => "Mesha Sankranti",
            9 => "Makara Sankranti",
            3 => "Karka Sankranti",
            6 => "Tula Sankranti",
            _ => "Sankranti",
        };
        let _ = SIGNS[s as usize % 12];
        let observed_sunrise = rises.iter().copied().find(|r| *r >= t);
        out.push(Observance {
            name,
            exact: t,
            observed_sunrise,
            note: "The Sun enters a sidereal sign; the solar month begins.",
        });
    }
    out.sort_by_key(|o| o.exact);
    Ok(out)
}

/// An observance's name or note in the reader's language, `(English, ml, hi)`.
const WORDS: [(&str, &str, &str); 25] = [
    ("Ekadashi (Shukla)", "ഏകാദശി (ശുക്ലപക്ഷം)", "एकादशी (शुक्ल)"),
    ("Ekadashi (Krishna)", "ഏകാദശി (കൃഷ്ണപക്ഷം)", "एकादशी (कृष्ण)"),
    ("Pradosha", "പ്രദോഷം", "प्रदोष"),
    ("Purnima", "പൗർണമി (വെളുത്തവാവ്)", "पूर्णिमा"),
    ("Amavasya", "അമാവാസി (കറുത്തവാവ്)", "अमावस्या"),
    ("Chaturthi (Shukla)", "ചതുർത്ഥി (ശുക്ലപക്ഷം)", "चतुर्थी (शुक्ल)"),
    ("Sankashti Chaturthi", "സങ്കഷ്ടി ചതുർത്ഥി", "संकष्टी चतुर्थी"),
    ("Ashtami (Shukla)", "അഷ്ടമി (ശുക്ലപക്ഷം)", "अष्टमी (शुक्ल)"),
    ("Ashtami (Krishna)", "അഷ്ടമി (കൃഷ്ണപക്ഷം)", "अष्टमी (कृष्ण)"),
    ("New Moon (exact)", "കറുത്തവാവ് (കൃത്യസമയം)", "अमावस्या (ठीक क्षण)"),
    ("Full Moon (exact)", "വെളുത്തവാവ് (കൃത്യസമയം)", "पूर्णिमा (ठीक क्षण)"),
    ("Mesha Sankranti", "മേടസംക്രമം (വിഷു)", "मेष संक्रांति"),
    ("Makara Sankranti", "മകരസംക്രമം", "मकर संक्रांति"),
    ("Karka Sankranti", "കർക്കടകസംക്രമം", "कर्क संक्रांति"),
    ("Tula Sankranti", "തുലാസംക്രമം", "तुला संक्रांति"),
    ("Sankranti", "സംക്രമം", "संक्रांति"),
    ("Fasting day of the waxing fortnight; observed at the sunrise within it.", "ശുക്ലപക്ഷത്തിലെ വ്രതദിനം; ഈ തിഥിയിൽ വരുന്ന സൂര്യോദയത്തിന്റെ ദിവസം ആചരിക്കുന്നു.", "शुक्ल पक्ष का व्रत; जिस दिन सूर्योदय इस तिथि में हो, उस दिन रखा जाता है।"),
    ("Fasting day of the waning fortnight; observed at the sunrise within it.", "കൃഷ്ണപക്ഷത്തിലെ വ്രതദിനം; ഈ തിഥിയിൽ വരുന്ന സൂര്യോദയത്തിന്റെ ദിവസം ആചരിക്കുന്നു.", "कृष्ण पक्ष का व्रत; जिस दिन सूर्योदय इस तिथि में हो, उस दिन रखा जाता है।"),
    ("Trayodashi; the evening twilight is the observance window.", "ത്രയോദശി; സന്ധ്യാസമയമാണ് ആചരണ സമയം.", "त्रयोदशी; शाम का गोधूलि समय पूजा का समय है।"),
    ("Full Moon tithi; the exact full Moon is listed separately.", "പൗർണമി തിഥി; വെളുത്തവാവിന്റെ കൃത്യസമയം വേറെ കൊടുത്തിട്ടുണ്ട്.", "पूर्णिमा तिथि; पूर्ण चंद्र का ठीक क्षण अलग दिया गया है।"),
    ("New Moon tithi; the exact new Moon is listed separately.", "അമാവാസി തിഥി; കറുത്തവാവിന്റെ കൃത്യസമയം വേറെ കൊടുത്തിട്ടുണ്ട്.", "अमावस्या तिथि; अमावस्या का ठीक क्षण अलग दिया गया है।"),
    ("Ganesha's tithi of the waxing fortnight.", "ശുക്ലപക്ഷത്തിലെ ഗണപതിയുടെ തിഥി.", "शुक्ल पक्ष में गणेश की तिथि।"),
    ("Fourth tithi of the waning fortnight; observed at moonrise.", "കൃഷ്ണപക്ഷത്തിലെ നാലാം തിഥി; ചന്ദ്രോദയത്തിൽ ആചരിക്കുന്നു.", "कृष्ण पक्ष की चौथी तिथि; चंद्रोदय पर मनाई जाती है।"),
    ("Durga's tithi of the waxing fortnight.", "ശുക്ലപക്ഷത്തിലെ ദേവിയുടെ (ദുർഗ) തിഥി.", "शुक्ल पक्ष में दुर्गा की तिथि।"),
    ("Kalashtami.", "കാലാഷ്ടമി.", "कालाष्टमी।"),
];

/// Notes not in [`WORDS`] because they are long: `(English, ml, hi)`.
const MORE_WORDS: [(&str, &str, &str); 3] = [
    ("Sun and Moon at the same longitude.", "സൂര്യനും ചന്ദ്രനും ഒരേ രേഖാംശത്തിൽ.", "सूर्य और चंद्र एक ही भोगांश पर।"),
    ("Moon opposite the Sun.", "ചന്ദ്രൻ സൂര്യന് നേരെ എതിരിൽ.", "चंद्र सूर्य के ठीक सामने।"),
    ("The Sun enters a sidereal sign; the solar month begins.", "സൂര്യൻ ഒരു നിരയന രാശിയിൽ പ്രവേശിക്കുന്നു; സൗരമാസം തുടങ്ങുന്നു.", "सूर्य किसी निरयन राशि में प्रवेश करता है; सौर मास शुरू होता है।"),
];

/// An observance's name or note (as [`Observance`] carries them) in the reader's language;
/// English and unknown text come back unchanged.
pub fn observance_words(text: &'static str, lang: kshana_rules::Lang) -> &'static str {
    WORDS
        .iter()
        .chain(MORE_WORDS.iter())
        .find(|w| w.0 == text)
        .map_or(text, |w| lang.pick([w.0, w.1, w.2]))
}

/// Sidereal sign name (Sanskrit) of a tropical longitude at `t`.
pub fn sidereal_sign_name(tropical: f64, ayanamsa: Ayanamsa, t: Instant) -> &'static str {
    let ay = ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000());
    SIGNS[(sidereal(tropical, ay) / 30.0).floor() as usize % 12]
}
