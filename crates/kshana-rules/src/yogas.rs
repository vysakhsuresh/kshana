//! The classical yogas: a catalogue of 103 combinations from Brihat Parashara Hora
//! Shastra, Brihat Jataka, Phaladeepika, Saravali and the Jaimini Sutras, each with the
//! exact condition it tests, a strength, the dasha lords that bring it to life, and a
//! plain-language meaning for the person.
//!
//! [`crate::parashari::yogas`] stays as the short list the older screens use; this
//! module is the full catalogue.
//!
//! # Conventions (stated once, used everywhere)
//!
//! * Houses are rasi houses (whole signs from the lagna sign), as BPHS counts them.
//! * Graha drishti is [`crate::parashari::aspected_houses`]: every graha the 7th; Mars
//!   4 and 8, Jupiter 5 and 9, Saturn 3 and 10 as well (BPHS ch. 26).
//! * Natural benefics are Jupiter, Venus, the waxing Moon and an unafflicted Mercury
//!   ([`crate::parashari::is_benefic`], BPHS ch. 3); Sun, Mars, Saturn, Rahu and Ketu
//!   are malefics. The Nabhasa yogas use the seven grahas only (BPHS ch. 35).
//! * "Mutual kendras" means the two are 1, 4, 7 or 10 signs from each other, the
//!   same sign included (the kendra count starts at 1).
//! * "Strong" for a lord a yoga asks to be strong (balavan): not debilitated, not in
//!   an enemy's sign, not combust, not in the 6th, 8th or 12th. The texts leave the
//!   measure open; this is the minimal reading B. V. Raman uses in *Three Hundred
//!   Important Combinations*.
//! * Combustion orbs are Surya Siddhanta ch. 9 as the BPHS commentaries use them:
//!   Moon 12°, Mars 17°, Mercury 14° (12° retrograde), Jupiter 11°, Venus 10° (8°
//!   retrograde), Saturn 15° (the same table as `kshana_bala::tables::combustion_orb`).
//! * Where the texts give a house-occupancy pattern ("all planets in the 1st to the
//!   4th"), every house named must hold at least one planet and no planet may be
//!   elsewhere: the strict reading, which avoids false positives.
//!
//! # Strength
//!
//! Each participating graha is scored by [`graha_strength`]: dignity
//! ([`Dignity::score`]), halved when combust; +0.1 in a kendra or trikona, −0.15 in a
//! dusthana; ±0.06 per benefic or malefic conjoining or aspecting it; ±0.1 at most
//! from the Sarvashtakavarga bindus of its sign around the mean of 28 (BPHS ch. 72).
//! A yoga's strength is the mean over its grahas. For a difficult yoga, strength is how
//! strongly the difficulty shows: `1 − 0.6 × mean`, since weak grahas make it worse.
//!
//! # Activation
//!
//! A yoga gives its results in the dashas of the grahas that form it (BPHS ch. 39 on
//! raja yogas: "in the dasha of the planets causing the yoga"). The Nabhasa yogas give
//! theirs throughout life (BPHS ch. 35), and are marked `lifelong`.

use crate::facts::{Facts, PlanetFact};
use crate::parashari::{aspected_houses, dignity, exaltation, is_benefic, sarva_for, Dignity};
use kshana_vedic::{chara_karakas, sign_lord, signs_aspect, varga, DashaLord};
use DashaLord::*;

/// Which family a yoga belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The 32 Nabhasa yogas (BPHS ch. 35; Brihat Jataka ch. 12).
    Nabhasa,
    /// Yogas counted from the Moon (BPHS ch. 37; Brihat Jataka ch. 13).
    Lunar,
    /// Yogas counted from the Sun (BPHS ch. 38).
    Solar,
    /// Pancha Mahapurusha (BPHS ch. 75).
    Mahapurusha,
    /// Raja yogas (BPHS ch. 34, 39; Brihat Jataka ch. 11).
    Raja,
    /// Dhana yogas (BPHS ch. 41).
    Dhana,
    /// Parivartana (Phaladeepika ch. 6).
    Parivartana,
    /// Viparita raja yogas (Phaladeepika ch. 6).
    Viparita,
    /// The named yogas of BPHS ch. 36 and their kin.
    Named,
    /// Difficult combinations (BPHS ch. 9, 42; Phaladeepika ch. 6).
    Hardship,
    /// Pravrajya, renunciation (Brihat Jataka ch. 15; Phaladeepika ch. 27).
    Pravrajya,
    /// Jaimini raja yogas between chara karakas.
    Jaimini,
}

impl Group {
    /// A stable key.
    pub fn key(self) -> &'static str {
        match self {
            Group::Nabhasa => "nabhasa",
            Group::Lunar => "lunar",
            Group::Solar => "solar",
            Group::Mahapurusha => "mahapurusha",
            Group::Raja => "raja",
            Group::Dhana => "dhana",
            Group::Parivartana => "parivartana",
            Group::Viparita => "viparita",
            Group::Named => "named",
            Group::Hardship => "hardship",
            Group::Pravrajya => "pravrajya",
            Group::Jaimini => "jaimini",
        }
    }
}

/// One catalogue entry.
#[derive(Debug, PartialEq)]
pub struct YogaDef {
    /// Stable key, e.g. `gaja_kesari`.
    pub key: &'static str,
    /// Expert name, e.g. "Gaja Kesari".
    pub name: &'static str,
    /// Family.
    pub group: Group,
    /// Text and chapter.
    pub source: &'static str,
    /// The condition, in the texts' terms (expert).
    pub rule: &'static str,
    /// A short plain title for the person.
    pub title: &'static str,
    /// What it means for the person, in plain words.
    pub meaning: &'static str,
    /// True for a gift, false for a difficulty.
    pub good: bool,
    /// For a difficulty: humane words and what helps. Empty for a gift.
    pub care: &'static str,
}

macro_rules! y {
    ($key:literal, $name:literal, $group:ident, $source:literal, $rule:literal, $title:literal, $meaning:literal) => {
        YogaDef { key: $key, name: $name, group: Group::$group, source: $source, rule: $rule, title: $title, meaning: $meaning, good: true, care: "" }
    };
    ($key:literal, $name:literal, $group:ident, $source:literal, $rule:literal, $title:literal, $meaning:literal, care: $care:literal) => {
        YogaDef { key: $key, name: $name, group: Group::$group, source: $source, rule: $rule, title: $title, meaning: $meaning, good: false, care: $care }
    };
}


/// The catalogue.
pub static CATALOG: &[YogaDef] = &[
    // ---- Nabhasa: Ashraya (BPHS ch. 35 v. 7-8; Brihat Jataka 12.2) ----
    y!("rajju", "Rajju", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in movable signs", "Drawn to travel and movement", "You do best on the move. Travel, a change of place and work that keeps you going bring your luck; standing still too long makes you restless."),
    y!("musala", "Musala", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in fixed signs", "Steady and set in your ways", "You are firm, proud of your word and slow to change. People trust your steadiness, and you build things that last."),
    y!("nala", "Nala", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in dual signs", "Many skills, many sides", "You adapt easily and are good at more than one thing. You can work with many kinds of people; the risk is spreading yourself thin."),
    // ---- Nabhasa: Dala ----
    y!("mala", "Mala (Srik)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "natural benefics in three kendras, no malefic in a kendra", "A life with comforts", "Comforts, good company and pleasures come to you fairly easily. People enjoy your presence."),
    y!("sarpa", "Sarpa", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "malefics in three kendras, no benefic in a kendra", "A harder road early on", "Life asks more effort of you than of most, and kindness is not always returned at first.", care: "This is a call to build patience and good habits. Steady service and a calm tongue turn it around over the years."),
    // ---- Nabhasa: Akriti ----
    y!("gada", "Gada", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in two successive kendras (1-4, 4-7, 7-10 or 10-1)", "Wealth through steady effort", "You work hard and earn well. Learning, skill and a strong will bring money and respect."),
    y!("nabhasa_shakata", "Shakata (Nabhasa)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 1st and 7th", "Ups and downs in earning", "Earnings rise and fall like a turning wheel. Work with vehicles, transport or your own hands suits you.", care: "Save in the good years for the lean ones. A steady routine and simple living keep the wheel turning your way."),
    y!("vihaga", "Vihaga (Pakshi)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 4th and 10th", "A messenger and traveller", "You move between people and places, carrying news, goods or ideas. Travel and errands for others shape your work."),
    y!("shringataka", "Shringataka", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 1st, 5th and 9th", "Happier with the years", "You are liked by many. Quarrels of youth give way to a happier, more settled life as the years go on."),
    y!("hala", "Hala", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in one of the triads 2-6-10, 3-7-11 or 4-8-12", "Close to the land", "You do well with land, food or steady hands-on work, and you share what you have."),
    y!("vajra", "Vajra", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "benefics in the 1st and 7th, malefics in the 4th and 10th", "Easy at the start and end of life", "Your early and later years are the easiest; the middle years ask for courage. You are brave and pleasant to look at."),
    y!("yava", "Yava", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "malefics in the 1st and 7th, benefics in the 4th and 10th", "A bright middle life", "The middle of your life is the happiest and most settled. You keep to your principles and give freely."),
    y!("kamala", "Kamala (Padma)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the four kendras, each occupied", "Respect and a lasting good name", "You are respected, live with dignity and do many good deeds. Your name lasts."),
    y!("vapi", "Vapi", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the four panapharas or the four apoklimas, each occupied", "A careful saver", "You build savings and are careful with money. Comfort comes from what you put aside."),
    y!("yupa", "Yupa", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 1st to 4th, each occupied", "Devoted to good causes", "You give time to worthy causes, keep your word, and find meaning in ritual and service."),
    y!("shara", "Shara (Ishu)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 4th to 7th, each occupied", "Skilled and sharp", "You are skilled with tools and craft. You can be hard on others; used well, that sharpness becomes expertise."),
    y!("shakti", "Shakti", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 7th to 10th, each occupied", "Success after a slow start", "Early life can feel short of money and ease, but you keep going. Persistence is your strength.", care: "Hold on to steady work and good friends. The effort you put in now is what pays later."),
    y!("danda", "Danda", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the 10th to 1st, each occupied", "Learning to stand alone", "At times you may feel let down by people close to you, and you learn to rely on yourself.", care: "Keep your circle of friends wide and keep serving others. Kindness given freely comes back from unexpected places."),
    y!("nauka", "Nauka", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the seven houses from the 1st, each occupied", "Earning through trade and travel", "You can do well through trade, travel over water or work linked with water. Fame comes, though happiness is not always steady."),
    y!("kuta", "Kuta", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the seven houses from the 4th, each occupied", "Restless under rules", "You may chafe at rules and be tempted to bend the truth when pressed.", care: "Choose honest work and people you can trust. A clean name is the strongest protection you have."),
    y!("chhatra", "Chhatra", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the seven houses from the 7th, each occupied", "A shelter for others", "You look after many people. Your later years are especially happy and comfortable."),
    y!("chapa", "Chapa (Dhanus)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the seven houses from the 10th, each occupied", "Brave and far-roaming", "You are brave and self-reliant, and you may travel far. The middle of life brings the most comfort."),
    y!("ardha_chandra", "Ardha Chandra", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in seven successive houses starting from a panapara or apoklima, each occupied", "A leader people follow", "You lead well and earn the favour of those above you."),
    y!("chakra", "Chakra", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the six odd houses from the 1st, each occupied", "Respected by the powerful", "People with influence respect you, and you can rise to a high place."),
    y!("samudra", "Samudra", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "all seven grahas in the six even houses from the 2nd, each occupied", "Wealth like the sea", "Wealth, comforts and a generous heart. You live well and share with many."),
    // ---- Nabhasa: Sankhya (only when no other Nabhasa yoga forms; BPHS ch. 35) ----
    y!("vallaki", "Vallaki (Veena)", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in seven signs", "Many friends and the arts", "You love music and the arts, have many friends and live comfortably."),
    y!("daama", "Daama", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in six signs", "Generous and helpful", "You are generous, help many people and are well known for it."),
    y!("pasha", "Pasha", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in five signs", "Ties that bind", "You gather people around you and work hard to keep them; duties can feel like ties.", care: "Keep your commitments light enough to carry, and share the load. Asking for help is a strength."),
    y!("kedara", "Kedara", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in four signs", "Useful to many", "You are useful to many people, honest and steady; land or practical work brings income."),
    y!("shoola", "Shoola", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in three signs", "Quick to act", "You can be quick to anger and quick to act, and life tests your temper.", care: "Hard physical work, sport and a pause before you speak turn this fire into strength."),
    y!("yuga", "Yuga", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in two signs", "A lean stretch", "Money can be scarce for long spells, and you may feel outside the crowd.", care: "Steady work, simple needs and good company help a lot. A trade or skill learned well is your best friend."),
    y!("gola", "Gola", Nabhasa, "BPHS ch. 35 (Nabhasa yogas); Brihat Jataka ch. 12", "the seven grahas in one sign", "Hard going at times", "There can be long stretches of struggle with money and learning.", care: "A skill, steady effort and a kind circle around you change the picture. This is a starting point, not a sentence."),
    // ---- Lunar (BPHS ch. 37; Brihat Jataka ch. 13; Phaladeepika ch. 6) ----
    y!("sunapha", "Sunapha", Lunar, "BPHS ch. 37; Brihat Jataka ch. 13; Saravali ch. 13", "a graha other than the Sun and the nodes in the 2nd from the Moon, none in the 12th", "Self-made", "What you have, you earn yourself. You are clever, well spoken and build your own wealth."),
    y!("anapha", "Anapha", Lunar, "BPHS ch. 37; Brihat Jataka ch. 13; Saravali ch. 13", "a graha other than the Sun and the nodes in the 12th from the Moon, none in the 2nd", "Healthy and well liked", "You carry yourself well, enjoy good health and a good name, and live free of want."),
    y!("durudhura", "Durudhura", Lunar, "BPHS ch. 37; Brihat Jataka ch. 13; Saravali ch. 13", "grahas other than the Sun and the nodes in both the 2nd and 12th from the Moon", "Supported on all sides", "Help comes from many directions. You enjoy comforts and the goodwill of others, and you are generous."),
    y!("kemadruma", "Kemadruma", Lunar, "BPHS ch. 37; Brihat Jataka ch. 13; Phaladeepika ch. 6", "no graha other than the Sun and the nodes in the 2nd or 12th from the Moon, and no cancellation", "Times of feeling alone", "There may be times when you feel alone or short of support, and your moods can run low.", care: "Stay connected: call family, keep friends close and help others. Caring for your mother or a mother figure, and a calm daily routine, are the old remedies, and they work."),
    y!("kemadruma_bhanga", "Kemadruma Bhanga", Lunar, "BPHS ch. 37; Phaladeepika ch. 6; Saravali ch. 13", "Kemadruma cancelled: a graha in a kendra from the lagna or the Moon, a graha with the Moon, or Jupiter's aspect on the Moon", "Help arrives when needed", "Lonely spells may come, but help arrives in time. They make you self-reliant and strong."),
    y!("adhi", "Adhi (from the Moon)", Lunar, "BPHS ch. 37; Phaladeepika ch. 6", "at least two natural benefics in the 6th, 7th and 8th from the Moon, no malefic there", "Rising to a place of trust", "You can rise to a position of trust and authority. Comfort, a good name and success over rivals come with it."),
    y!("chandra_mangala", "Chandra-Mangala", Lunar, "Phaladeepika ch. 6; B. V. Raman, Three Hundred Important Combinations", "the Moon and Mars in one sign", "Earning through drive", "You make money through drive and enterprise; trade, property and business suit you. Keep a cool head with family."),
    y!("vasumati", "Vasumati", Lunar, "Phaladeepika ch. 6", "Mercury, Jupiter and Venus all in upachayas (3, 6, 10, 11) from the Moon or the lagna", "Wealth that keeps growing", "Your wealth grows steadily over the years. You are rarely without what you need."),
    y!("shakata", "Shakata", Lunar, "BPHS ch. 36; Phaladeepika ch. 6", "Jupiter in the 6th, 8th or 12th from the Moon, not cancelled by Jupiter in a kendra from the lagna", "Ups and downs of fortune", "Fortune rises and falls. A good spell can be followed by a slow one, and you may need to start again more than once.", care: "Save in good spells and keep your teachers close. Generosity and steady effort smooth the wheel."),
    y!("gaja_kesari", "Gaja Kesari", Lunar, "BPHS ch. 36; Phaladeepika ch. 6", "Jupiter in a kendra from the Moon, not debilitated, not combust, not in an enemy's sign (or in a kendra from the lagna joined or aspected by a benefic, BPHS)", "Wisdom and a good name", "You are wise, respected and remembered for good deeds. People listen to you, and wealth comes with honour."),
    // ---- Solar (BPHS ch. 38) ----
    y!("vesi", "Vesi", Solar, "BPHS ch. 38; Saravali ch. 13", "a graha other than the Moon and the nodes in the 2nd from the Sun, none in the 12th", "Truthful and balanced", "You are truthful, even-minded and hard working. Comfort grows in the later part of life."),
    y!("vasi", "Vasi", Solar, "BPHS ch. 38; Saravali ch. 13", "a graha other than the Moon and the nodes in the 12th from the Sun, none in the 2nd", "Skilful and giving", "You are skilful and generous, and well liked by those in charge."),
    y!("ubhayachari", "Ubhayachari", Solar, "BPHS ch. 38; Saravali ch. 13", "grahas other than the Moon and the nodes in both the 2nd and 12th from the Sun", "Well rounded", "You are capable in many fields, well spoken and comfortable, and people look up to you."),
    y!("budhaditya", "Budha-Aditya", Solar, "B. V. Raman, Three Hundred Important Combinations (Budha-Aditya)", "the Sun and Mercury in one sign; weak when Mercury is combust", "A sharp mind", "You think clearly, learn quickly and speak well. Study and skill bring respect."),
    // ---- Pancha Mahapurusha (BPHS ch. 75; Phaladeepika ch. 6) ----
    y!("ruchaka", "Ruchaka", Mahapurusha, "BPHS ch. 75; Phaladeepika ch. 6", "Mars in its own or exaltation sign in a kendra from the lagna", "Courage and command", "You are brave, strong and able to lead. You win in contests and may command others."),
    y!("bhadra", "Bhadra", Mahapurusha, "BPHS ch. 75; Phaladeepika ch. 6", "Mercury in its own or exaltation sign in a kendra from the lagna", "Brilliant with words", "You are clever, articulate and learned. Business, writing and teaching suit you."),
    y!("hamsa", "Hamsa", Mahapurusha, "BPHS ch. 75; Phaladeepika ch. 6", "Jupiter in its own or exaltation sign in a kendra from the lagna", "Goodness and wisdom", "You are good-hearted, wise and respected. People trust your counsel."),
    y!("malavya", "Malavya", Mahapurusha, "BPHS ch. 75; Phaladeepika ch. 6", "Venus in its own or exaltation sign in a kendra from the lagna", "Grace and comfort", "You enjoy beauty, comfort and a happy family life. Art and refinement come naturally."),
    y!("shasha", "Shasha", Mahapurusha, "BPHS ch. 75; Phaladeepika ch. 6", "Saturn in its own or exaltation sign in a kendra from the lagna", "Authority through hard work", "You rise through discipline and service and can hold real authority. Many people come to trust you."),
    // ---- Raja (BPHS ch. 34, 39; Brihat Jataka ch. 11; Phaladeepika ch. 7) ----
    y!("raja", "Raja (kendra and trikona lords)", Raja, "BPHS ch. 39", "a kendra lord and a trikona lord conjoined, in mutual aspect or exchanging signs", "A rise in life", "You have the makings of a rise in standing: success, authority and respect, most of all in the years this is active."),
    y!("dharma_karmadhipati", "Dharma-Karmadhipati", Raja, "BPHS ch. 39", "the 9th and 10th lords conjoined, in mutual aspect or exchanging signs", "Work that is also your calling", "Your work and your sense of purpose line up. Good fortune and career rise together."),
    y!("yogakaraka", "Yogakaraka", Raja, "BPHS ch. 34", "one graha owns both a kendra (4, 7, 10) and a trikona (5, 9)", "One strong helper", "One strong influence in your life brings both luck and achievement; the years it rules are among your best."),
    y!("neecha_bhanga", "Neecha Bhanga Raja", Raja, "Phaladeepika ch. 7; BPHS ch. 39; Saravali", "a debilitated graha whose fall is cancelled by one or more of the classical rules", "Weakness turned to strength", "Something that starts as a weakness becomes your strength. Early setbacks teach you what later lifts you high."),
    y!("three_exalted", "Raja (three exalted)", Raja, "Brihat Jataka ch. 11", "three or more grahas exalted or in moolatrikona", "Born to rise", "Several strong supports in your life point to a rise in position and lasting success."),
    y!("vargottama_moon", "Raja (vargottama Moon)", Raja, "Brihat Jataka ch. 11", "the Moon vargottama (same sign in the rasi and the navamsa) and aspected by four or more grahas", "A steady mind, widely supported", "Your mind is steady and many kinds of people support you; this can bring a place of leadership."),
    // ---- Dhana (BPHS ch. 41) ----
    y!("dhana", "Dhana", Dhana, "BPHS ch. 41", "the lord of the 2nd or 11th conjoined, in mutual aspect or exchanging signs with the lord of the 1st, 2nd, 5th, 9th or 11th", "Money comes", "There is a clear promise of wealth. Savings, gains and income grow, most of all in the years this is active."),
    // ---- Parivartana (Phaladeepika ch. 6) ----
    y!("maha_parivartana", "Maha Parivartana", Parivartana, "Phaladeepika ch. 6", "lords of two houses exchange signs, neither house being 3, 6, 8 or 12", "Two good things that feed each other", "Two good parts of your life support each other: wealth, comfort, honour and a happy family come with it."),
    y!("khala_parivartana", "Khala Parivartana", Parivartana, "Phaladeepika ch. 6", "the 3rd lord exchanges signs with the lord of a house other than 6, 8 or 12", "Swings of fortune", "Your fortunes can swing between good and difficult spells, and your moods with them.", care: "Steady routines and saving in good times keep the swings small. Courage used well turns this into success."),
    y!("dainya_parivartana", "Dainya Parivartana", Parivartana, "Phaladeepika ch. 6", "the lord of the 6th, 8th or 12th exchanges signs with the lord of another house", "Struggle that teaches", "Some things in life come only through struggle, and there can be spells of worry or debt.", care: "Avoid shortcuts and debt, help those who struggle, and trust slow progress. Difficulties here often turn into unusual gains later."),
    // ---- Viparita (Phaladeepika ch. 6; Uttara Kalamrita ch. 4) ----
    y!("harsha", "Harsha (Viparita)", Viparita, "Phaladeepika ch. 6; Uttara Kalamrita ch. 4", "the 6th lord in the 6th, 8th or 12th", "Winning over troubles", "Rivals, illness and trouble do not hold you down for long. You stay healthy, happy and win disputes."),
    y!("sarala", "Sarala (Viparita)", Viparita, "Phaladeepika ch. 6; Uttara Kalamrita ch. 4", "the 8th lord in the 6th, 8th or 12th", "Fearless and long-lived", "You face danger and hardship without fear, live long and come out ahead. Learning and wealth follow."),
    y!("vimala", "Vimala (Viparita)", Viparita, "Phaladeepika ch. 6; Uttara Kalamrita ch. 4", "the 12th lord in the 6th, 8th or 12th", "Good with money, free in spirit", "You spend wisely, save well and live independently, with a good character and a happy life."),
    // ---- Named yogas of BPHS ch. 36 and kin ----
    y!("lagnadhi", "Lagnadhi", Named, "BPHS ch. 36", "at least two natural benefics in the 6th, 7th and 8th from the lagna, no malefic there", "A good and learned life", "You live well, learn deeply and are respected as a good person."),
    y!("lakshmi", "Lakshmi", Named, "BPHS ch. 36", "the 9th lord in its own, moolatrikona or exaltation sign in a kendra or trikona, and the lagna lord strong", "Fortune and grace", "Wealth, beauty, a good name and a generous heart. Fortune favours you."),
    y!("saraswati", "Saraswati", Named, "Phaladeepika ch. 6", "Jupiter, Venus and Mercury in kendras, trikonas or the 2nd, Jupiter in its own, a friend's or its exaltation sign", "A gift for learning", "You are learned, skilful with words and the arts, and respected for your knowledge."),
    y!("amala", "Amala", Named, "BPHS ch. 36; Phaladeepika ch. 6", "only natural benefics in the 10th from the lagna or the Moon", "A spotless name", "Your good name lasts. You are honest, prosperous and admired for your character."),
    y!("parvata", "Parvata", Named, "BPHS ch. 36", "benefics in the kendras with no malefic there, the 6th and 8th empty or holding benefics only; or the lords of the 1st and 12th in mutual kendras", "Lasting prosperity", "You are prosperous, generous and well known, a leader in your community."),
    y!("kahala", "Kahala", Named, "BPHS ch. 36", "the 4th and 9th lords in mutual kendras, the lagna lord strong", "Bold and able", "You are bold, energetic and able to lead others; you may head a group or a place."),
    y!("chamara", "Chamara", Named, "BPHS ch. 36", "the lagna lord exalted in a kendra and aspected by Jupiter; or two benefics together in the 1st, 7th, 9th or 10th", "Honoured for wisdom", "You are honoured, eloquent and learned. People in high places seek your advice."),
    y!("shankha", "Shankha", Named, "BPHS ch. 36", "the 5th and 6th lords in mutual kendras, the lagna lord strong; or the lords of the 1st and 10th together in a movable sign, the 9th lord strong", "A good and long life", "You are kind and live long and well, with a happy family and comforts."),
    y!("bheri", "Bheri", Named, "BPHS ch. 36", "Venus, Jupiter and the lagna lord in mutual kendras, the 9th lord strong; or all seven grahas in the 1st, 2nd, 7th and 12th, the 9th lord strong", "Good health and good fortune", "You enjoy health, comfort, a happy family and a good name. Your later years are prosperous."),
    y!("mridanga", "Mridanga", Named, "BPHS ch. 36; B. V. Raman, Three Hundred Important Combinations", "the lord of the sign holding the lord of an exalted graha's navamsa is in a kendra or trikona in its own or exaltation sign, the lagna lord strong", "Rising high", "You rise to a high position and enjoy respect and comfort."),
    y!("shrinatha", "Shrinatha", Named, "BPHS ch. 36; B. V. Raman, Three Hundred Important Combinations", "the 7th lord exalted in the 10th, the 10th lord with the 9th lord", "Blessed in family and fortune", "You live in comfort with the joy of a good partner and children, and you are generous and devoted."),
    y!("khadga", "Khadga", Named, "BPHS ch. 36", "the 2nd lord in the 9th and the 9th lord in the 2nd, the lagna lord in a kendra or trikona", "Wealth through knowledge", "Wealth comes through knowledge, skill and your own effort. You are clever and fortunate."),
    y!("kalanidhi", "Kalanidhi", Named, "BPHS ch. 36", "Jupiter in the 2nd or 5th, joined or aspected by both Mercury and Venus", "Gifted and respected", "You are gifted, well mannered and respected; wealth and honour come to you."),
    y!("matsya", "Matsya", Named, "BPHS ch. 36", "benefics only in the 1st and 9th, benefics and malefics in the 5th, malefics only in the 4th and 8th", "Kind and knowledgeable", "You are kind and knowledgeable, and known for your good deeds."),
    y!("kurma", "Kurma", Named, "BPHS ch. 36", "benefics only in the 5th, 6th and 7th; malefics only in the 1st, 3rd and 11th, each in its own, moolatrikona or exaltation sign", "Happy and well known", "You are happy, well known and helpful to others, with strength of character."),
    y!("chatussagara", "Chatussagara", Named, "Saravali (as B. V. Raman, Three Hundred Important Combinations gives it)", "all four kendras occupied", "Known far and wide", "Your name and wealth spread far. You live comfortably and enjoy a good reputation."),
    y!("shubha_kartari", "Shubha Kartari", Named, "BPHS ch. 36 (Subha yoga); Phaladeepika ch. 6", "benefics only in both the 2nd and the 12th", "Protected on both sides", "Good influences surround you. You are healthy, well spoken and protected from trouble."),
    y!("papa_kartari", "Papa Kartari", Hardship, "BPHS ch. 36 (Asubha yoga); Phaladeepika ch. 6", "malefics only in both the 2nd and the 12th", "Pressed on both sides", "You may feel hemmed in by pressures at times, with others asking a lot of you.", care: "Protect your rest and choose your company with care. Quiet time each day and generosity ease the pressure."),
    y!("hari", "Hari", Named, "BPHS ch. 36", "benefics in the 2nd, 12th and 8th from the 2nd lord", "Wealth and learning", "You are learned, wealthy and happy, with children who do well."),
    y!("hara", "Hara", Named, "BPHS ch. 36", "benefics in the 4th, 9th and 8th from the 7th lord", "Happiness and learning", "You are happy, learned and prosperous, and well respected."),
    y!("brahma", "Brahma", Named, "BPHS ch. 36", "benefics in the 4th, 10th and 11th from the lagna lord", "Long life and learning", "You live long, learn deeply and do good for many."),
    y!("guru_mangala", "Guru-Mangala", Named, "B. V. Raman, Three Hundred Important Combinations", "Jupiter and Mars in one sign or in the 7th from each other", "Courage guided by wisdom", "You combine courage with good judgement. Property, leadership and righteous causes suit you."),
    y!("bhaskara", "Bhaskara", Named, "B. V. Raman, Three Hundred Important Combinations", "Mercury in the 2nd from the Sun, the Moon in the 11th from Mercury, Jupiter in the 5th or 9th from the Moon", "Brave and learned", "You are brave, learned and well regarded, with a gift for subtle subjects."),
    y!("indra", "Indra", Named, "B. V. Raman, Three Hundred Important Combinations", "the 5th and 11th lords exchange signs, the Moon in the 5th", "Fame and courage", "You become well known for bold action and enjoy comfort and influence."),
    y!("marud", "Marud", Named, "B. V. Raman, Three Hundred Important Combinations", "Jupiter in the 5th or 9th from Venus, the Moon in the 5th from Jupiter, the Sun in a kendra from the Moon", "A generous voice", "You are eloquent, generous and successful in trade, with a big heart."),
    y!("budha", "Budha", Named, "B. V. Raman, Three Hundred Important Combinations", "Jupiter in the lagna, the Moon in a kendra, Rahu in the 2nd from the Moon, the Sun and Mars in the 3rd from Rahu", "Learned and respected", "You are learned, respected and favoured by people in power."),
    y!("siva", "Siva", Named, "B. V. Raman, Three Hundred Important Combinations", "the 5th lord in the 9th, the 9th lord in the 10th, the 10th lord in the 5th", "A wise guide", "You are wise and successful, and become a guide or leader for others."),
    y!("jaya", "Jaya", Named, "B. V. Raman, Three Hundred Important Combinations", "the 6th lord debilitated and the 10th lord exalted", "Victory over rivals", "You defeat rivals, succeed in your efforts and live long and happily."),
    y!("vidyut", "Vidyut", Named, "B. V. Raman, Three Hundred Important Combinations", "the 11th lord exalted, joined with Venus, in a kendra from the lagna lord", "Generous with gains", "You gain wealth and give it freely; you can rise to a position of trust."),
    y!("sareera_soukhya", "Sareera Soukhya", Named, "B. V. Raman, Three Hundred Important Combinations", "the lagna lord, Jupiter or Venus in a kendra", "Good health", "You enjoy good health, a long life and a strong body."),
    // ---- Hardship (Phaladeepika ch. 6; BPHS ch. 9, 42) ----
    y!("duryoga", "Duryoga", Hardship, "Phaladeepika ch. 6", "the 10th lord in the 6th, 8th or 12th", "Effort without credit", "Your work may not always be seen or rewarded as it should be.", care: "Keep a record of what you do, find work you believe in, and serve without waiting for praise. Recognition often comes late, but it comes."),
    y!("daridra", "Daridra", Hardship, "Phaladeepika ch. 6; BPHS ch. 42", "the 11th lord in the 6th, 8th or 12th", "Money needs care", "Money may slip away and gains can be slow.", care: "Keep a budget, avoid lending and quick schemes, and give a little regularly to those in need; it steadies the flow."),
    y!("kala_sarpa", "Kala Sarpa", Hardship, "Later texts, not in BPHS: all seven grahas on one side of the Rahu-Ketu axis", "all seven grahas between Rahu and Ketu", "Sudden turns, then a steady rise", "Life may move in sudden turns, with delays before things settle.", care: "Patience is the key. Steady work, prayer and helping others smooth the way, and many with this go on to great things."),
    y!("balarishta", "Balarishta", Hardship, "BPHS ch. 9 (Arishta), ch. 10 (Arishta bhanga)", "the Moon in the 6th, 8th or 12th aspected by a malefic and by no benefic, with no cancellation", "Early health needed care", "The first years of life called for extra care with health.", care: "If you are reading this as an adult, that chapter is behind you. Regular check-ups and good sleep keep the habit of care alive, and it serves you well."),
    y!("arishta_bhanga", "Arishta Bhanga", Hardship, "BPHS ch. 10", "an early-health arishta cancelled by a strong Jupiter in a kendra, a strong lagna lord in a kendra, or a benefic's aspect on the Moon", "Protected in early life", "Early health troubles were turned aside; you were well protected."),
    // ---- Pravrajya (Brihat Jataka ch. 15; Phaladeepika ch. 27) ----
    y!("pravrajya", "Pravrajya", Pravrajya, "Brihat Jataka ch. 15; Phaladeepika ch. 27", "four or more grahas in one sign, the strongest of them not combust", "A pull towards the spiritual", "You feel a strong pull towards a spiritual, simple life and may serve a higher cause. It does not mean leaving the world; it means you find meaning beyond it."),
    y!("pravrajya_saturn", "Pravrajya (Saturn on the Moon's lord)", Pravrajya, "Brihat Jataka ch. 15", "the lord of the Moon's sign aspected by Saturn alone", "A quiet inner search", "Your inner life matters deeply to you. Times of solitude and study bring you peace."),
    // ---- Jaimini ----
    y!("jaimini_ak_amk", "Jaimini Raja (Atmakaraka and Amatyakaraka)", Jaimini, "Jaimini Upadesa Sutras, raja yoga sutras (Sanjay Rath's translation; Iranganti Rangacharya)", "the Atmakaraka and the Amatyakaraka conjoined, in mutual rasi drishti or exchanging signs", "The self and the work agree", "What you want from life and the work you do pull in the same direction. Success and standing come through your career."),
    y!("jaimini_ak_pk", "Jaimini Raja (Atmakaraka and Putrakaraka)", Jaimini, "Jaimini Upadesa Sutras, raja yoga sutras (Sanjay Rath's translation; Iranganti Rangacharya)", "the Atmakaraka and the Putrakaraka conjoined, in mutual rasi drishti or exchanging signs", "Creative power", "Your creativity and your deepest aims work together; children, students or creative work bring you standing."),
];

/// A catalogue entry by key.
pub fn def(key: &str) -> &'static YogaDef {
    CATALOG
        .iter()
        .find(|d| d.key == key)
        .unwrap_or_else(|| panic!("yoga key {key} is not in the catalogue"))
}

/// A yoga found in a chart.
#[derive(Clone, Debug, PartialEq)]
pub struct YogaFound {
    /// The catalogue entry.
    pub def: &'static YogaDef,
    /// What fired it, in expert terms.
    pub factors: Vec<String>,
    /// Strength in `[0, 1]` (see the module notes).
    pub strength: f64,
    /// The grahas that form it.
    pub grahas: Vec<DashaLord>,
    /// The dasha lords whose periods bring it to life.
    pub activators: Vec<DashaLord>,
    /// True when it works throughout life rather than in particular periods.
    pub lifelong: bool,
}

const SEVEN: [DashaLord; 7] = [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn];
const NINE: [DashaLord; 9] = [Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu];
const KENDRA: [u8; 4] = [1, 4, 7, 10];

fn kendra(h: u8) -> bool {
    KENDRA.contains(&h)
}
fn trikona(h: u8) -> bool {
    matches!(h, 1 | 5 | 9)
}
fn dusthana(h: u8) -> bool {
    matches!(h, 6 | 8 | 12)
}
/// Count from sign `a` to sign `b`, 1 = the same sign.
fn from(a: u8, b: u8) -> u8 {
    ((b as i32 - a as i32).rem_euclid(12) + 1) as u8
}
/// The houses of a run of `len` successive houses starting at `start`.
fn run(start: u8, len: u8) -> Vec<u8> {
    (0..len).map(|i| (start - 1 + i) % 12 + 1).collect()
}

/// Combustion orb (Surya Siddhanta ch. 9; see the module notes).
fn combustion_orb(g: DashaLord, retrograde: bool) -> Option<f64> {
    Some(match (g, retrograde) {
        (Moon, _) => 12.0,
        (Mars, _) => 17.0,
        (Mercury, false) => 14.0,
        (Mercury, true) => 12.0,
        (Jupiter, _) => 11.0,
        (Venus, false) => 10.0,
        (Venus, true) => 8.0,
        (Saturn, _) => 15.0,
        _ => return None,
    })
}

struct Ctx<'a> {
    f: &'a Facts,
    sarva: [u8; 12],
}

impl<'a> Ctx<'a> {
    fn p(&self, g: DashaLord) -> &PlanetFact {
        self.f.planet(g)
    }
    fn sign(&self, g: DashaLord) -> u8 {
        self.p(g).sign
    }
    fn house(&self, g: DashaLord) -> u8 {
        self.p(g).rasi_house
    }
    fn lord(&self, h: u8) -> DashaLord {
        self.f.rasi_lords[((h - 1) % 12) as usize]
    }
    fn dig(&self, g: DashaLord) -> Dignity {
        dignity(g, self.p(g).sidereal)
    }
    fn benefic(&self, g: DashaLord) -> bool {
        !matches!(g, Rahu | Ketu) && is_benefic(self.f, g)
    }
    fn combust(&self, g: DashaLord) -> bool {
        let p = self.p(g);
        match combustion_orb(g, p.retrograde) {
            Some(orb) => {
                let d = (p.sidereal - self.p(Sun).sidereal).rem_euclid(360.0);
                d.min(360.0 - d) < orb
            }
            None => false,
        }
    }
    /// Grahas in a rasi house; `nodes` includes Rahu and Ketu.
    fn in_house(&self, h: u8, nodes: bool) -> Vec<DashaLord> {
        let set: &[DashaLord] = if nodes { &NINE } else { &SEVEN };
        set.iter().copied().filter(|g| self.house(*g) == h).collect()
    }
    /// Grahas in a sign counted from another sign.
    fn in_sign(&self, s: u8, nodes: bool) -> Vec<DashaLord> {
        let set: &[DashaLord] = if nodes { &NINE } else { &SEVEN };
        set.iter().copied().filter(|g| self.sign(*g) == s).collect()
    }
    /// `a` casts graha drishti on `b`.
    fn aspects(&self, a: DashaLord, b: DashaLord) -> bool {
        a != b && aspected_houses(a, self.house(a)).contains(&self.house(b))
    }
    fn conj(&self, a: DashaLord, b: DashaLord) -> bool {
        a != b && self.sign(a) == self.sign(b)
    }
    fn exchange(&self, a: DashaLord, b: DashaLord) -> bool {
        a != b && sign_lord(self.sign(a) as usize) == b && sign_lord(self.sign(b) as usize) == a
    }
    /// Conjunction, exchange or mutual aspect (BPHS ch. 39's three relations).
    fn connected(&self, a: DashaLord, b: DashaLord) -> Option<&'static str> {
        if a == b {
            None
        } else if self.conj(a, b) {
            Some("conjoined")
        } else if self.exchange(a, b) {
            Some("exchanging signs")
        } else if self.aspects(a, b) && self.aspects(b, a) {
            Some("in mutual aspect")
        } else {
            None
        }
    }
    fn mutual_kendra(&self, a: DashaLord, b: DashaLord) -> bool {
        kendra(from(self.sign(a), self.sign(b)))
    }
    /// The minimal "strong" (balavan) of the module notes.
    fn strong(&self, g: DashaLord) -> bool {
        !matches!(self.dig(g), Dignity::Debilitated | Dignity::Enemy)
            && !self.combust(g)
            && !dusthana(self.house(g))
    }
    fn own_or_exalted(&self, g: DashaLord) -> bool {
        matches!(self.dig(g), Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted)
    }
    fn strength(&self, g: DashaLord) -> f64 {
        graha_strength_in(self, g)
    }
    fn mean(&self, gs: &[DashaLord]) -> f64 {
        if gs.is_empty() {
            return 0.5;
        }
        gs.iter().map(|g| self.strength(*g)).sum::<f64>() / gs.len() as f64
    }
    fn name(&self, g: DashaLord) -> String {
        format!("{g:?}")
    }
}

fn graha_strength_in(c: &Ctx, g: DashaLord) -> f64 {
    let p = c.p(g);
    let mut s = match g {
        Rahu | Ketu => 0.5,
        _ => c.dig(g).score(),
    };
    if c.combust(g) {
        s *= 0.5;
    }
    if kendra(p.rasi_house) || trikona(p.rasi_house) {
        s += 0.1;
    } else if dusthana(p.rasi_house) {
        s -= 0.15;
    }
    for q in NINE {
        if q != g && (c.conj(q, g) || c.aspects(q, g)) {
            s += if c.benefic(q) { 0.06 } else { -0.06 };
        }
    }
    s += ((c.sarva[p.sign as usize] as f64 - 28.0) / 40.0).clamp(-0.1, 0.1);
    s.clamp(0.0, 1.0)
}

/// Strength of one graha for yoga purposes, `[0, 1]` (see the module notes).
pub fn graha_strength(facts: &Facts, g: DashaLord) -> f64 {
    let c = Ctx { f: facts, sarva: sarva_for(facts) };
    graha_strength_in(&c, g)
}

struct Out<'c, 'a> {
    c: &'c Ctx<'a>,
    v: Vec<YogaFound>,
}

impl Out<'_, '_> {
    fn push(&mut self, key: &str, factors: Vec<String>, grahas: Vec<DashaLord>, activators: Vec<DashaLord>) {
        let d = def(key);
        let m = self.c.mean(&grahas);
        let strength = if d.good { m } else { (1.0 - 0.6 * m).clamp(0.0, 1.0) };
        self.push_with(key, factors, grahas, activators, strength, false);
    }
    fn push_with(&mut self, key: &str, factors: Vec<String>, grahas: Vec<DashaLord>, mut activators: Vec<DashaLord>, strength: f64, lifelong: bool) {
        let mut seen = Vec::new();
        activators.retain(|g| {
            let fresh = !seen.contains(g);
            seen.push(*g);
            fresh
        });
        self.v.push(YogaFound {
            def: def(key),
            factors,
            strength: (strength.clamp(0.0, 1.0) * 1000.0).round() / 1000.0,
            grahas,
            activators,
            lifelong,
        });
    }
}

/// Every catalogued yoga present in the chart.
pub fn chart_yogas(facts: &Facts) -> Vec<YogaFound> {
    let c = Ctx { f: facts, sarva: sarva_for(facts) };
    let mut o = Out { c: &c, v: Vec::new() };
    nabhasa(&mut o);
    lunar(&mut o);
    solar(&mut o);
    mahapurusha(&mut o);
    raja(&mut o);
    dhana(&mut o);
    parivartana(&mut o);
    viparita(&mut o);
    neecha_bhanga(&mut o);
    named(&mut o);
    hardship(&mut o);
    pravrajya(&mut o);
    jaimini(&mut o);
    o.v
}

// ---------------------------------------------------------------------------------
// Nabhasa (BPHS ch. 35)
// ---------------------------------------------------------------------------------

fn nabhasa(o: &mut Out) {
    let c = o.c;
    let hs: Vec<(DashaLord, u8)> = SEVEN.iter().map(|g| (*g, c.house(*g))).collect();
    let all_in = |set: &[u8]| hs.iter().all(|(_, h)| set.contains(h));
    let fills = |set: &[u8]| all_in(set) && set.iter().all(|s| hs.iter().any(|(_, h)| h == s));
    let strength = c.mean(&SEVEN);
    let mut found: Vec<(&str, String)> = Vec::new();

    // Ashraya: the modality of the signs (BPHS ch. 35 v. 7).
    for (key, m, label) in [("rajju", 0u8, "movable"), ("musala", 1, "fixed"), ("nala", 2, "dual")] {
        if SEVEN.iter().all(|g| c.sign(*g) % 3 == m) {
            found.push((key, format!("all seven grahas are in {label} signs")));
        }
    }
    // Dala: Mala and Sarpa (BPHS ch. 35 v. 8).
    let ben_k = KENDRA.iter().filter(|k| hs.iter().any(|(g, h)| h == *k && c.benefic(*g))).count();
    let mal_k = KENDRA.iter().filter(|k| hs.iter().any(|(g, h)| h == *k && !c.benefic(*g))).count();
    if ben_k >= 3 && mal_k == 0 {
        found.push(("mala", format!("benefics hold {ben_k} kendras and no malefic is in a kendra")));
    }
    if mal_k >= 3 && ben_k == 0 {
        found.push(("sarpa", format!("malefics hold {mal_k} kendras and no benefic is in a kendra")));
    }
    // Akriti (BPHS ch. 35 v. 9-17).
    for (a, b) in [(1u8, 4u8), (4, 7), (7, 10), (10, 1)] {
        if fills(&[a, b]) {
            found.push(("gada", format!("all seven grahas in the {a} and {b} houses")));
        }
    }
    let simple: [(&str, &[u8]); 13] = [
        ("nabhasa_shakata", &[1, 7]),
        ("vihaga", &[4, 10]),
        ("shringataka", &[1, 5, 9]),
        ("hala", &[2, 6, 10]),
        ("hala", &[3, 7, 11]),
        ("hala", &[4, 8, 12]),
        ("kamala", &[1, 4, 7, 10]),
        ("vapi", &[2, 5, 8, 11]),
        ("vapi", &[3, 6, 9, 12]),
        ("chakra", &[1, 3, 5, 7, 9, 11]),
        ("samudra", &[2, 4, 6, 8, 10, 12]),
        ("yupa", &[1, 2, 3, 4]),
        ("shara", &[4, 5, 6, 7]),
    ];
    for (key, set) in simple {
        if fills(set) {
            found.push((key, format!("all seven grahas fill houses {set:?}")));
        }
    }
    for (key, start, len) in [("shakti", 7u8, 4u8), ("danda", 10, 4), ("nauka", 1, 7), ("kuta", 4, 7), ("chhatra", 7, 7), ("chapa", 10, 7)] {
        let set = run(start, len);
        if fills(&set) {
            found.push((key, format!("all seven grahas fill houses {set:?}")));
        }
    }
    for start in [2u8, 3, 5, 6, 8, 9, 11, 12] {
        let set = run(start, 7);
        if fills(&set) {
            found.push(("ardha_chandra", format!("all seven grahas fill the seven houses from the {start}")));
        }
    }
    // Vajra and Yava: benefics and malefics split between the kendra pairs.
    let bens: Vec<u8> = hs.iter().filter(|(g, _)| c.benefic(*g)).map(|x| x.1).collect();
    let mals: Vec<u8> = hs.iter().filter(|(g, _)| !c.benefic(*g)).map(|x| x.1).collect();
    let split = |b: &[u8], m: &[u8]| {
        !bens.is_empty() && !mals.is_empty() && bens.iter().all(|h| b.contains(h)) && mals.iter().all(|h| m.contains(h))
            && b.iter().all(|h| bens.contains(h)) && m.iter().all(|h| mals.contains(h))
    };
    if split(&[1, 7], &[4, 10]) {
        found.push(("vajra", "benefics in the 1st and 7th, malefics in the 4th and 10th".into()));
    }
    if split(&[4, 10], &[1, 7]) {
        found.push(("yava", "malefics in the 1st and 7th, benefics in the 4th and 10th".into()));
    }
    // Sankhya: only when no other Nabhasa yoga forms (BPHS ch. 35 v. 18).
    if found.is_empty() {
        let mut signs: Vec<u8> = SEVEN.iter().map(|g| c.sign(*g)).collect();
        signs.sort_unstable();
        signs.dedup();
        let key = ["gola", "yuga", "shoola", "kedara", "pasha", "daama", "vallaki"][signs.len() - 1];
        found.push((key, format!("the seven grahas occupy {} signs", signs.len())));
    }
    for (key, why) in found {
        let d = def(key);
        let s = if d.good { strength } else { 1.0 - 0.6 * strength };
        o.push_with(key, vec![why], SEVEN.to_vec(), Vec::new(), s, true);
    }
}

// ---------------------------------------------------------------------------------
// Lunar (BPHS ch. 37) and solar (BPHS ch. 38)
// ---------------------------------------------------------------------------------

fn lunar(o: &mut Out) {
    let c = o.c;
    let m = c.sign(Moon);
    let five = [Mars, Mercury, Jupiter, Venus, Saturn];
    let second: Vec<DashaLord> = five.iter().copied().filter(|g| from(m, c.sign(*g)) == 2).collect();
    let twelfth: Vec<DashaLord> = five.iter().copied().filter(|g| from(m, c.sign(*g)) == 12).collect();
    let names = |v: &[DashaLord]| v.iter().map(|g| c.name(*g)).collect::<Vec<_>>().join(", ");
    let with_moon = |v: &[DashaLord]| {
        let mut g = vec![Moon];
        g.extend_from_slice(v);
        g
    };
    match (second.is_empty(), twelfth.is_empty()) {
        (false, true) => {
            let g = with_moon(&second);
            o.push("sunapha", vec![format!("{} in the 2nd from the Moon, nothing in the 12th", names(&second))], g.clone(), g);
        }
        (true, false) => {
            let g = with_moon(&twelfth);
            o.push("anapha", vec![format!("{} in the 12th from the Moon, nothing in the 2nd", names(&twelfth))], g.clone(), g);
        }
        (false, false) => {
            let mut g = with_moon(&second);
            g.extend(twelfth.iter().copied());
            o.push("durudhura", vec![format!("{} in the 2nd and {} in the 12th from the Moon", names(&second), names(&twelfth))], g.clone(), g);
        }
        (true, true) => {
            // Kemadruma bhanga (BPHS ch. 37; Phaladeepika ch. 6; Saravali ch. 13).
            let mut cancels = Vec::new();
            let lagna_k: Vec<DashaLord> = SEVEN.iter().copied().filter(|g| *g != Moon && kendra(c.house(*g))).collect();
            if !lagna_k.is_empty() {
                cancels.push(format!("{} in a kendra from the lagna", names(&lagna_k)));
            }
            let moon_k: Vec<DashaLord> = SEVEN.iter().copied().filter(|g| matches!(from(m, c.sign(*g)), 4 | 7 | 10)).collect();
            if !moon_k.is_empty() {
                cancels.push(format!("{} in a kendra from the Moon", names(&moon_k)));
            }
            let with: Vec<DashaLord> = SEVEN.iter().copied().filter(|g| c.conj(*g, Moon)).collect();
            if !with.is_empty() {
                cancels.push(format!("{} with the Moon", names(&with)));
            }
            if c.aspects(Jupiter, Moon) {
                cancels.push("Jupiter aspects the Moon".into());
            }
            if cancels.is_empty() {
                o.push("kemadruma", vec!["no graha other than the Sun in the 2nd or 12th from the Moon, and none of the cancellations".into()], vec![Moon], vec![Moon]);
            } else {
                let f = cancels.iter().map(|x| format!("Kemadruma is cancelled: {x}")).collect();
                o.push("kemadruma_bhanga", f, vec![Moon], vec![Moon]);
            }
        }
    }
    // Adhi from the Moon (BPHS ch. 37) and from the lagna (Lagnadhi, BPHS ch. 36).
    adhi(o, "adhi", m, "the Moon");
    adhi(o, "lagnadhi", c.f.lagna_sign, "the lagna");
    if c.conj(Moon, Mars) {
        o.push("chandra_mangala", vec!["the Moon and Mars share a sign".into()], vec![Moon, Mars], vec![Moon, Mars]);
    }
    let trio = [Mercury, Jupiter, Venus];
    for (base, label) in [(m, "the Moon"), (c.f.lagna_sign, "the lagna")] {
        if trio.iter().all(|g| matches!(from(base, c.sign(*g)), 3 | 6 | 10 | 11)) {
            o.push("vasumati", vec![format!("Mercury, Jupiter and Venus all in upachayas from {label}")], trio.to_vec(), trio.to_vec());
            break;
        }
    }
    let jm = from(m, c.sign(Jupiter));
    if dusthana(jm) && !kendra(c.house(Jupiter)) {
        o.push("shakata", vec![format!("Jupiter is {jm} signs from the Moon and not in a kendra from the lagna")], vec![Jupiter, Moon], vec![Jupiter, Moon]);
    }
    // Gaja Kesari (BPHS ch. 36; Phaladeepika ch. 6).
    let jd = c.dig(Jupiter);
    let sound = !matches!(jd, Dignity::Debilitated | Dignity::Enemy) && !c.combust(Jupiter);
    let ben_with: Vec<DashaLord> = [Mercury, Venus, Moon]
        .into_iter()
        .filter(|g| c.benefic(*g) && (c.conj(*g, Jupiter) || c.aspects(*g, Jupiter)))
        .collect();
    if sound && kendra(jm) {
        let mut f = vec![format!("Jupiter is in the {jm} from the Moon, {jd:?}, not combust")];
        let mut s = c.mean(&[Jupiter, Moon]);
        if ben_with.is_empty() {
            s *= 0.85;
        } else {
            f.push(format!("Jupiter is joined or aspected by {}", names(&ben_with)));
        }
        o.push_with("gaja_kesari", f, vec![Jupiter, Moon], vec![Jupiter, Moon], s, false);
    } else if sound && kendra(c.house(Jupiter)) && !ben_with.is_empty() {
        let f = vec![format!("Jupiter in a kendra from the lagna, joined or aspected by {} (BPHS ch. 36)", names(&ben_with))];
        o.push("gaja_kesari", f, vec![Jupiter], vec![Jupiter]);
    }
}

fn adhi(o: &mut Out, key: &str, base: u8, label: &str) {
    let c = o.c;
    let ben: Vec<DashaLord> = [Mercury, Jupiter, Venus]
        .into_iter()
        .filter(|g| c.benefic(*g) && matches!(from(base, c.sign(*g)), 6 | 7 | 8))
        .collect();
    let mal = NINE.iter().any(|g| *g != Moon && !c.benefic(*g) && matches!(from(base, c.sign(*g)), 6 | 7 | 8))
        || (key == "lagnadhi" && !c.benefic(Moon) && matches!(from(base, c.sign(Moon)), 6 | 7 | 8));
    if ben.len() >= 2 && !mal {
        let places: Vec<u8> = ben.iter().map(|g| from(base, c.sign(*g))).collect();
        let mut distinct = places.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let full = distinct.len() == 3;
        let f = vec![format!(
            "{} in the {:?} from {label}, no malefic there{}",
            ben.iter().map(|g| c.name(*g)).collect::<Vec<_>>().join(", "),
            places,
            if full { "; all three places filled (the full yoga)" } else { "" }
        )];
        let s = c.mean(&ben) * if full { 1.0 } else { 0.85 };
        o.push_with(key, f, ben.clone(), ben, s, false);
    }
}

fn solar(o: &mut Out) {
    let c = o.c;
    let s = c.sign(Sun);
    let five = [Mars, Mercury, Jupiter, Venus, Saturn];
    let second: Vec<DashaLord> = five.iter().copied().filter(|g| from(s, c.sign(*g)) == 2).collect();
    let twelfth: Vec<DashaLord> = five.iter().copied().filter(|g| from(s, c.sign(*g)) == 12).collect();
    let names = |v: &[DashaLord]| v.iter().map(|g| c.name(*g)).collect::<Vec<_>>().join(", ");
    let mut g = vec![Sun];
    g.extend(second.iter().copied());
    g.extend(twelfth.iter().copied());
    match (second.is_empty(), twelfth.is_empty()) {
        (false, true) => o.push("vesi", vec![format!("{} in the 2nd from the Sun", names(&second))], g.clone(), g),
        (true, false) => o.push("vasi", vec![format!("{} in the 12th from the Sun", names(&twelfth))], g.clone(), g),
        (false, false) => o.push("ubhayachari", vec![format!("{} in the 2nd and {} in the 12th from the Sun", names(&second), names(&twelfth))], g.clone(), g),
        (true, true) => {}
    }
    if c.conj(Sun, Mercury) {
        let comb = c.combust(Mercury);
        let st = c.mean(&[Sun, Mercury]) * if comb { 0.6 } else { 1.0 };
        let f = vec![format!("Sun and Mercury share a sign{}", if comb { "; Mercury is combust, which weakens it" } else { "" })];
        o.push_with("budhaditya", f, vec![Sun, Mercury], vec![Sun, Mercury], st, false);
    }
}

// ---------------------------------------------------------------------------------
// Mahapurusha (BPHS ch. 75), Raja (ch. 34, 39), Dhana (ch. 41)
// ---------------------------------------------------------------------------------

fn mahapurusha(o: &mut Out) {
    let c = o.c;
    for (g, key) in [(Mars, "ruchaka"), (Mercury, "bhadra"), (Jupiter, "hamsa"), (Venus, "malavya"), (Saturn, "shasha")] {
        if kendra(c.house(g)) && c.own_or_exalted(g) {
            let mut f = vec![format!("{g:?} is {:?} in house {}", c.dig(g), c.house(g))];
            let mut s = c.strength(g);
            // Commonly read caveat: joined by the Sun or Moon the yoga gives less.
            let lum: Vec<DashaLord> = [Sun, Moon].into_iter().filter(|l| c.conj(*l, g)).collect();
            if !lum.is_empty() {
                s *= 0.8;
                f.push(format!("joined by {lum:?}, which lessens it"));
            }
            if c.combust(g) {
                f.push("combust".into());
            }
            o.push_with(key, f, vec![g], vec![g], s, false);
        }
    }
}

fn raja(o: &mut Out) {
    let c = o.c;
    let mut seen: Vec<(DashaLord, DashaLord)> = Vec::new();
    for kh in KENDRA {
        for th in [1u8, 5, 9] {
            if kh == th {
                continue;
            }
            let (kl, tl) = (c.lord(kh), c.lord(th));
            if kl == tl || seen.iter().any(|p| *p == (kl, tl) || *p == (tl, kl)) {
                continue;
            }
            let Some(how) = c.connected(kl, tl) else { continue };
            seen.push((kl, tl));
            let nine_ten = {
                let (l9, l10) = (c.lord(9), c.lord(10));
                (kl == l10 && tl == l9) || (kl == l9 && tl == l10)
            };
            let key = if nine_ten { "dharma_karmadhipati" } else { "raja" };
            let mut s = c.mean(&[kl, tl]);
            // A lord that also rules the 8th or 12th gives the yoga less purely
            // (BPHS ch. 34, the lords of 8 and 12 as functional malefics).
            let tainted = [kl, tl].iter().any(|g| [8u8, 12].iter().any(|h| c.lord(*h) == *g && c.lord(1) != *g));
            if tainted {
                s *= 0.8;
            }
            o.push_with(key, vec![format!("{kl:?} (lord of {kh}) and {tl:?} (lord of {th}) are {how}{}", if tainted { "; one also rules the 8th or 12th" } else { "" })], vec![kl, tl], vec![kl, tl], s, false);
        }
    }
    // Yogakaraka (BPHS ch. 34).
    for g in SEVEN {
        let owns: Vec<u8> = (1..=12u8).filter(|h| c.lord(*h) == g).collect();
        if owns.iter().any(|h| matches!(h, 4 | 7 | 10)) && owns.iter().any(|h| matches!(h, 5 | 9)) {
            o.push("yogakaraka", vec![format!("{g:?} rules houses {owns:?}, a kendra and a trikona")], vec![g], vec![g]);
        }
    }
    // Brihat Jataka ch. 11: three or more grahas exalted or in moolatrikona.
    let high: Vec<DashaLord> = SEVEN.iter().copied().filter(|g| matches!(c.dig(*g), Dignity::Exalted | Dignity::Moolatrikona)).collect();
    if high.len() >= 3 {
        o.push("three_exalted", vec![format!("{high:?} are exalted or in moolatrikona")], high.clone(), high);
    }
    // Brihat Jataka ch. 11: a vargottama Moon aspected by four or more grahas.
    let moon_d9 = (varga(c.p(Moon).sidereal, 9) / 30.0).floor() as u8 % 12;
    if moon_d9 == c.sign(Moon) {
        let by: Vec<DashaLord> = [Sun, Mars, Mercury, Jupiter, Venus, Saturn].into_iter().filter(|g| c.aspects(*g, Moon)).collect();
        if by.len() >= 4 {
            let mut g = vec![Moon];
            g.extend(by.iter().copied());
            o.push("vargottama_moon", vec![format!("the Moon is vargottama and aspected by {by:?}")], g.clone(), g);
        }
    }
}

fn dhana(o: &mut Out) {
    let c = o.c;
    let mut seen: Vec<(DashaLord, DashaLord)> = Vec::new();
    for a in [2u8, 11] {
        for b in [1u8, 2, 5, 9, 11] {
            if a == b {
                continue;
            }
            let (la, lb) = (c.lord(a), c.lord(b));
            if la == lb || seen.iter().any(|p| *p == (la, lb) || *p == (lb, la)) {
                continue;
            }
            if let Some(how) = c.connected(la, lb) {
                seen.push((la, lb));
                o.push("dhana", vec![format!("{la:?} (lord of {a}) and {lb:?} (lord of {b}) are {how}")], vec![la, lb], vec![la, lb]);
            }
        }
    }
}

// ---------------------------------------------------------------------------------
// Parivartana, Viparita (Phaladeepika ch. 6), Neecha Bhanga (Phaladeepika ch. 7)
// ---------------------------------------------------------------------------------

fn parivartana(o: &mut Out) {
    let c = o.c;
    for (i, p) in SEVEN.iter().enumerate() {
        for q in &SEVEN[i + 1..] {
            if !c.exchange(*p, *q) {
                continue;
            }
            // p sits in a house q rules and q in a house p rules.
            let (a, b) = (c.house(*q), c.house(*p));
            let key = if dusthana(a) || dusthana(b) {
                "dainya_parivartana"
            } else if a == 3 || b == 3 {
                "khala_parivartana"
            } else {
                "maha_parivartana"
            };
            o.push(key, vec![format!("{p:?} and {q:?} exchange signs, joining houses {} and {}", a.min(b), a.max(b))], vec![*p, *q], vec![*p, *q]);
        }
    }
}

fn viparita(o: &mut Out) {
    let c = o.c;
    for (h, key) in [(6u8, "harsha"), (8, "sarala"), (12, "vimala")] {
        let l = c.lord(h);
        let at = c.house(l);
        if !dusthana(at) {
            continue;
        }
        let mut f = vec![format!("{l:?}, lord of {h}, is in house {at}")];
        let mut s = c.strength(l).max(0.4);
        if l == c.lord(1) {
            s *= 0.5;
            f.push("it also rules the lagna, whose lord in a dusthana harms the self: much weakened".into());
        }
        // Uttara Kalamrita ch. 4: the yoga is purest unjoined by other lords.
        let joined: Vec<DashaLord> = SEVEN
            .iter()
            .copied()
            .filter(|g| c.conj(*g, l) && (1..=12u8).any(|x| !dusthana(x) && c.lord(x) == *g))
            .collect();
        if !joined.is_empty() {
            s *= 0.8;
            f.push(format!("joined by {joined:?}, lords of other houses, which dilutes it"));
        }
        o.push_with(key, f, vec![l], vec![l], s, false);
    }
}

/// The planet exalted in a sign, if any.
fn exalted_in(sign: u8) -> Option<DashaLord> {
    SEVEN.into_iter().find(|g| exaltation(*g).map(|e| e.0) == Some(sign))
}

fn neecha_bhanga(o: &mut Out) {
    let c = o.c;
    let m = c.sign(Moon);
    for g in SEVEN {
        if c.dig(g) != Dignity::Debilitated {
            continue;
        }
        let s = c.sign(g);
        let disp = sign_lord(s as usize);
        let ex_sign = exaltation(g).expect("seven grahas exalt").0;
        let ex_lord = sign_lord(ex_sign as usize);
        let ex_here = exalted_in(s);
        // In a kendra from the lagna or from the Moon; the Moon is not counted as in
        // a kendra from itself.
        let in_k = |x: DashaLord| kendra(c.house(x)) || (x != Moon && kendra(from(m, c.sign(x))));
        let mut rules = Vec::new();
        if in_k(disp) {
            rules.push(format!("its dispositor {disp:?} is in a kendra from the lagna or the Moon (Phaladeepika ch. 7)"));
        }
        if let Some(e) = ex_here {
            if e != g && in_k(e) {
                rules.push(format!("{e:?}, exalted in this sign, is in a kendra from the lagna or the Moon (Phaladeepika ch. 7)"));
            }
            if e != g && (c.conj(e, g) || c.aspects(e, g)) {
                rules.push(format!("{e:?}, exalted in this sign, joins or aspects it (Saravali)"));
            }
        }
        if ex_lord != g && in_k(ex_lord) {
            rules.push(format!("{ex_lord:?}, lord of its exaltation sign, is in a kendra from the lagna or the Moon (BPHS)"));
        }
        if ex_lord != disp && c.mutual_kendra(disp, ex_lord) && disp != g && ex_lord != g {
            rules.push(format!("its dispositor {disp:?} and its exaltation lord {ex_lord:?} are in mutual kendras (Phaladeepika ch. 7)"));
        }
        if disp != g && (c.conj(disp, g) || c.aspects(disp, g)) {
            rules.push(format!("its dispositor {disp:?} joins or aspects it (BPHS)"));
        }
        if c.exchange(g, disp) {
            rules.push(format!("it exchanges signs with its dispositor {disp:?}"));
        }
        let d9 = (varga(c.p(g).sidereal, 9) / 30.0).floor() as u8 % 12;
        if d9 == ex_sign {
            rules.push("it is exalted in the navamsa (Phaladeepika ch. 7)".into());
        }
        if rules.is_empty() {
            continue;
        }
        let n = rules.len() as f64;
        let s = (0.35 + 0.15 * n).min(1.0) * (0.7 + 0.3 * c.strength(disp));
        let mut f = vec![format!("{g:?} is debilitated in house {}", c.house(g))];
        f.extend(rules);
        o.push_with("neecha_bhanga", f, vec![g, disp], vec![g, disp], s, false);
    }
}

// ---------------------------------------------------------------------------------
// The named yogas (BPHS ch. 36 and kin)
// ---------------------------------------------------------------------------------

fn named(o: &mut Out) {
    let c = o.c;
    let l1 = c.lord(1);
    let names = |v: &[DashaLord]| v.iter().map(|g| c.name(*g)).collect::<Vec<_>>().join(", ");
    let only_benefics = |h: u8| {
        let v = c.in_house(h, true);
        !v.is_empty() && v.iter().all(|g| c.benefic(*g))
    };
    let only_malefics = |h: u8| {
        let v = c.in_house(h, true);
        !v.is_empty() && v.iter().all(|g| !c.benefic(*g))
    };
    let ben_at_sign = |s: u8| c.in_sign(s, false).iter().any(|g| c.benefic(*g));

    // Lakshmi (BPHS ch. 36).
    let l9 = c.lord(9);
    if c.own_or_exalted(l9) && (kendra(c.house(l9)) || trikona(c.house(l9))) && c.strong(l1) {
        o.push("lakshmi", vec![format!("9th lord {l9:?} is {:?} in house {}; lagna lord {l1:?} is strong", c.dig(l9), c.house(l9))], vec![l9, l1], vec![l9, l1]);
    }
    // Saraswati (Phaladeepika ch. 6).
    let good_place = |g: DashaLord| matches!(c.house(g), 1 | 2 | 4 | 5 | 7 | 9 | 10);
    if [Jupiter, Venus, Mercury].iter().all(|g| good_place(*g))
        && matches!(c.dig(Jupiter), Dignity::Own | Dignity::Moolatrikona | Dignity::Exalted | Dignity::Friend)
    {
        o.push("saraswati", vec![format!("Jupiter, Venus and Mercury in houses {}, {}, {}; Jupiter {:?}", c.house(Jupiter), c.house(Venus), c.house(Mercury), c.dig(Jupiter))], vec![Jupiter, Venus, Mercury], vec![Jupiter, Venus, Mercury]);
    }
    // Amala (BPHS ch. 36).
    if only_benefics(10) {
        let v = c.in_house(10, true);
        o.push("amala", vec![format!("only benefics ({}) in the 10th from the lagna", names(&v))], v.clone(), v);
    } else {
        let s10 = (c.sign(Moon) + 9) % 12;
        let v = c.in_sign(s10, true);
        if !v.is_empty() && v.iter().all(|g| c.benefic(*g)) {
            o.push("amala", vec![format!("only benefics ({}) in the 10th from the Moon", names(&v))], v.clone(), v);
        }
    }
    // Parvata (BPHS ch. 36).
    let kv: Vec<DashaLord> = KENDRA.iter().flat_map(|h| c.in_house(*h, true)).collect();
    let clean_68 = [6u8, 8].iter().all(|h| c.in_house(*h, true).iter().all(|g| c.benefic(*g)));
    if !kv.is_empty() && kv.iter().all(|g| c.benefic(*g)) && clean_68 {
        o.push("parvata", vec![format!("only benefics ({}) in the kendras; the 6th and 8th empty or benefic", names(&kv))], kv.clone(), kv.clone());
    } else {
        let l12 = c.lord(12);
        if l12 != l1 && c.mutual_kendra(l1, l12) {
            o.push("parvata", vec![format!("lords of the 1st ({l1:?}) and 12th ({l12:?}) in mutual kendras")], vec![l1, l12], vec![l1, l12]);
        }
    }
    // Kahala (BPHS ch. 36).
    let (l4, l5, l6, l7, l10, l2) = (c.lord(4), c.lord(5), c.lord(6), c.lord(7), c.lord(10), c.lord(2));
    if l4 != l9 && c.mutual_kendra(l4, l9) && c.strong(l1) {
        o.push("kahala", vec![format!("4th lord {l4:?} and 9th lord {l9:?} in mutual kendras; lagna lord strong")], vec![l4, l9, l1], vec![l4, l9]);
    }
    // Chamara (BPHS ch. 36).
    if c.dig(l1) == Dignity::Exalted && kendra(c.house(l1)) && c.aspects(Jupiter, l1) {
        o.push("chamara", vec![format!("lagna lord {l1:?} exalted in house {} and aspected by Jupiter", c.house(l1))], vec![l1, Jupiter], vec![l1, Jupiter]);
    } else {
        for h in [1u8, 7, 9, 10] {
            let b: Vec<DashaLord> = c.in_house(h, false).into_iter().filter(|g| c.benefic(*g)).collect();
            if b.len() >= 2 {
                o.push("chamara", vec![format!("benefics {} together in house {h}", names(&b))], b.clone(), b);
                break;
            }
        }
    }
    // Shankha (BPHS ch. 36).
    if l5 != l6 && c.mutual_kendra(l5, l6) && c.strong(l1) {
        o.push("shankha", vec![format!("5th lord {l5:?} and 6th lord {l6:?} in mutual kendras; lagna lord strong")], vec![l5, l6, l1], vec![l5, l6]);
    } else if l1 != l10 && c.conj(l1, l10) && c.sign(l1) % 3 == 0 && c.strong(l9) {
        o.push("shankha", vec![format!("lords of the 1st ({l1:?}) and 10th ({l10:?}) together in a movable sign; 9th lord strong")], vec![l1, l10, l9], vec![l1, l10]);
    }
    // Bheri (BPHS ch. 36).
    let trio = [Venus, Jupiter, l1];
    let mutual = trio.iter().all(|a| trio.iter().all(|b| c.mutual_kendra(*a, *b)));
    if mutual && c.strong(l9) {
        o.push("bheri", vec![format!("Venus, Jupiter and lagna lord {l1:?} in mutual kendras; 9th lord {l9:?} strong")], vec![Venus, Jupiter, l1], vec![Venus, Jupiter, l1]);
    } else if SEVEN.iter().all(|g| matches!(c.house(*g), 1 | 2 | 7 | 12)) && c.strong(l9) {
        o.push("bheri", vec!["all seven grahas in the 1st, 2nd, 7th and 12th; 9th lord strong".into()], vec![l9], vec![l9]);
    }
    // Mridanga (BPHS ch. 36, Raman's reading).
    if c.strong(l1) {
        for e in SEVEN.into_iter().filter(|g| c.dig(*g) == Dignity::Exalted) {
            let n = (varga(c.p(e).sidereal, 9) / 30.0).floor() as usize % 12;
            let nl = sign_lord(n);
            let l2x = sign_lord(c.sign(nl) as usize);
            if (kendra(c.house(l2x)) || trikona(c.house(l2x))) && c.own_or_exalted(l2x) {
                o.push("mridanga", vec![format!("{e:?} exalted; its navamsa lord {nl:?} sits in a sign of {l2x:?}, which is {:?} in house {}; lagna lord strong", c.dig(l2x), c.house(l2x))], vec![e, l2x, l1], vec![l2x, e]);
                break;
            }
        }
    }
    // Shrinatha.
    if c.house(l7) == 10 && c.dig(l7) == Dignity::Exalted && l10 != l9 && c.conj(l10, l9) {
        o.push("shrinatha", vec![format!("7th lord {l7:?} exalted in the 10th; 10th lord {l10:?} with 9th lord {l9:?}")], vec![l7, l10, l9], vec![l7, l10, l9]);
    }
    // Khadga.
    if l2 != l9 && c.house(l2) == 9 && c.house(l9) == 2 && (kendra(c.house(l1)) || trikona(c.house(l1))) {
        o.push("khadga", vec![format!("2nd lord {l2:?} in the 9th and 9th lord {l9:?} in the 2nd; lagna lord in house {}", c.house(l1))], vec![l2, l9, l1], vec![l2, l9]);
    }
    // Kalanidhi.
    if matches!(c.house(Jupiter), 2 | 5) && [Mercury, Venus].iter().all(|g| c.conj(*g, Jupiter) || c.aspects(*g, Jupiter)) {
        o.push("kalanidhi", vec![format!("Jupiter in house {} joined or aspected by Mercury and Venus", c.house(Jupiter))], vec![Jupiter, Mercury, Venus], vec![Jupiter, Mercury, Venus]);
    }
    // Matsya.
    let five = c.in_house(5, true);
    if only_benefics(1) && only_benefics(9) && five.iter().any(|g| c.benefic(*g)) && five.iter().any(|g| !c.benefic(*g)) && only_malefics(4) && only_malefics(8) {
        o.push("matsya", vec!["benefics in the 1st and 9th, mixed in the 5th, malefics in the 4th and 8th".into()], c.in_house(9, true), c.in_house(9, true));
    }
    // Kurma.
    let mal_dig = |h: u8| {
        let v = c.in_house(h, true);
        !v.is_empty() && v.iter().all(|g| !c.benefic(*g) && !matches!(g, Rahu | Ketu) && c.own_or_exalted(*g))
    };
    if [5u8, 6, 7].iter().all(|h| only_benefics(*h)) && [1u8, 3, 11].iter().all(|h| mal_dig(*h)) {
        let g: Vec<DashaLord> = [5u8, 6, 7].iter().flat_map(|h| c.in_house(*h, false)).collect();
        o.push("kurma", vec!["benefics in the 5th, 6th and 7th; dignified malefics in the 1st, 3rd and 11th".into()], g.clone(), g);
    }
    // Chatussagara.
    if KENDRA.iter().all(|h| !c.in_house(*h, true).is_empty()) {
        o.push("chatussagara", vec!["all four kendras are occupied".into()], kv.clone(), kv.clone());
    }
    // Shubha and Papa kartari on the lagna.
    if only_benefics(2) && only_benefics(12) {
        let mut g = c.in_house(2, true);
        g.extend(c.in_house(12, true));
        o.push("shubha_kartari", vec!["benefics only in the 2nd and the 12th".into()], g.clone(), g);
    }
    // Hari, Hara, Brahma (BPHS ch. 36).
    for (key, l, places) in [("hari", l2, [2u8, 12, 8]), ("hara", l7, [4, 9, 8]), ("brahma", l1, [4, 10, 11])] {
        let base = c.sign(l);
        if places.iter().all(|p| ben_at_sign((base + p - 1) % 12)) {
            o.push(key, vec![format!("benefics in the {places:?} from {l:?}")], vec![l], vec![l, Jupiter, Venus]);
        }
    }
    // Guru-Mangala.
    if matches!(from(c.sign(Jupiter), c.sign(Mars)), 1 | 7) {
        o.push("guru_mangala", vec!["Jupiter and Mars in one sign or opposite".into()], vec![Jupiter, Mars], vec![Jupiter, Mars]);
    }
    // Bhaskara.
    let me = c.sign(Mercury);
    if from(c.sign(Sun), me) == 2 && from(me, c.sign(Moon)) == 11 && matches!(from(c.sign(Moon), c.sign(Jupiter)), 5 | 9) {
        o.push("bhaskara", vec!["Mercury 2nd from the Sun, the Moon 11th from Mercury, Jupiter in a trine from the Moon".into()], vec![Sun, Mercury, Moon, Jupiter], vec![Sun, Mercury, Moon, Jupiter]);
    }
    // Indra.
    let l11 = c.lord(11);
    if c.exchange(l5, l11) && c.house(Moon) == 5 {
        o.push("indra", vec![format!("5th lord {l5:?} and 11th lord {l11:?} exchange signs; the Moon in the 5th")], vec![l5, l11, Moon], vec![l5, l11, Moon]);
    }
    // Marud.
    if matches!(from(c.sign(Venus), c.sign(Jupiter)), 5 | 9) && from(c.sign(Jupiter), c.sign(Moon)) == 5 && kendra(from(c.sign(Moon), c.sign(Sun))) {
        o.push("marud", vec!["Jupiter in a trine from Venus, the Moon 5th from Jupiter, the Sun in a kendra from the Moon".into()], vec![Jupiter, Venus, Moon, Sun], vec![Jupiter, Venus, Moon]);
    }
    // Budha.
    let rs = c.sign(Rahu);
    if c.house(Jupiter) == 1 && kendra(c.house(Moon)) && from(c.sign(Moon), rs) == 2 && from(rs, c.sign(Sun)) == 3 && from(rs, c.sign(Mars)) == 3 {
        o.push("budha", vec!["Jupiter in the lagna, the Moon in a kendra, Rahu 2nd from the Moon, the Sun and Mars 3rd from Rahu".into()], vec![Jupiter, Moon, Rahu, Sun, Mars], vec![Jupiter, Moon]);
    }
    // Siva.
    if c.house(l5) == 9 && c.house(l9) == 10 && c.house(l10) == 5 && l5 != l9 && l9 != l10 {
        o.push("siva", vec![format!("5th lord {l5:?} in the 9th, 9th lord {l9:?} in the 10th, 10th lord {l10:?} in the 5th")], vec![l5, l9, l10], vec![l5, l9, l10]);
    }
    // Jaya.
    if c.dig(l6) == Dignity::Debilitated && c.dig(l10) == Dignity::Exalted {
        o.push("jaya", vec![format!("6th lord {l6:?} debilitated and 10th lord {l10:?} exalted")], vec![l10], vec![l10, l6]);
    }
    // Vidyut.
    if c.dig(l11) == Dignity::Exalted && l11 != Venus && c.conj(l11, Venus) && kendra(from(c.sign(l1), c.sign(l11))) {
        o.push("vidyut", vec![format!("11th lord {l11:?} exalted, with Venus, in a kendra from lagna lord {l1:?}")], vec![l11, Venus], vec![l11, Venus]);
    }
    // Sareera Soukhya.
    let k: Vec<DashaLord> = {
        let mut v = vec![l1, Jupiter, Venus];
        v.dedup();
        v.into_iter().filter(|g| kendra(c.house(*g))).collect()
    };
    if !k.is_empty() {
        o.push("sareera_soukhya", vec![format!("{} in a kendra", names(&k))], k.clone(), k);
    }
}

// ---------------------------------------------------------------------------------
// Hardship, Pravrajya, Jaimini
// ---------------------------------------------------------------------------------

fn hardship(o: &mut Out) {
    let c = o.c;
    let only_malefics = |h: u8| {
        let v = c.in_house(h, true);
        !v.is_empty() && v.iter().all(|g| !c.benefic(*g))
    };
    if only_malefics(2) && only_malefics(12) {
        let mut g = c.in_house(2, true);
        g.extend(c.in_house(12, true));
        o.push("papa_kartari", vec!["malefics only in the 2nd and the 12th".into()], vec![c.lord(1)], g);
    }
    for (h, key) in [(10u8, "duryoga"), (11, "daridra")] {
        let l = c.lord(h);
        let at = c.house(l);
        if dusthana(at) {
            let mut f = vec![format!("{l:?}, lord of {h}, is in house {at}")];
            let mut s = 1.0 - 0.6 * c.strength(l);
            if c.own_or_exalted(l) {
                s *= 0.6;
                f.push("it is dignified there, which softens it".into());
            }
            o.push_with(key, f, vec![l], vec![l], s, false);
        }
    }
    // Kala Sarpa (later texts).
    let rahu = c.p(Rahu).sidereal;
    let sides: Vec<bool> = SEVEN.iter().map(|g| (c.p(*g).sidereal - rahu).rem_euclid(360.0) < 180.0).collect();
    if sides.iter().all(|b| *b) || sides.iter().all(|b| !*b) {
        o.push("kala_sarpa", vec!["all seven grahas lie on one side of the Rahu-Ketu axis".into()], vec![Rahu, Ketu], vec![Rahu, Ketu]);
    }
    // Balarishta (BPHS ch. 9) and its cancellation (ch. 10).
    if dusthana(c.house(Moon)) {
        let mal: Vec<DashaLord> = [Sun, Mars, Saturn, Rahu, Ketu].into_iter().filter(|g| c.aspects(*g, Moon)).collect();
        let ben: Vec<DashaLord> = [Jupiter, Venus, Mercury].into_iter().filter(|g| c.benefic(*g) && (c.aspects(*g, Moon) || c.conj(*g, Moon))).collect();
        if !mal.is_empty() {
            let mut cancels = Vec::new();
            if kendra(c.house(Jupiter)) && c.strong(Jupiter) {
                cancels.push("a strong Jupiter in a kendra".to_string());
            }
            let l1 = c.lord(1);
            if kendra(c.house(l1)) && c.strong(l1) {
                cancels.push(format!("a strong lagna lord {l1:?} in a kendra"));
            }
            if !ben.is_empty() {
                cancels.push(format!("benefic aspect or company on the Moon from {ben:?}"));
            }
            let base = format!("the Moon in house {} aspected by {mal:?}", c.house(Moon));
            if cancels.is_empty() {
                o.push("balarishta", vec![base], vec![Moon], vec![Moon]);
            } else {
                let mut f = vec![base];
                f.extend(cancels.into_iter().map(|x| format!("cancelled by {x}")));
                o.push_with("arishta_bhanga", f, vec![Moon], vec![Moon], 0.6, false);
            }
        }
    }
}

fn pravrajya(o: &mut Out) {
    let c = o.c;
    for s in 0..12u8 {
        let v = c.in_sign(s, false);
        if v.len() < 4 {
            continue;
        }
        let strongest = *v
            .iter()
            .max_by(|a, b| c.strength(**a).partial_cmp(&c.strength(**b)).unwrap_or(std::cmp::Ordering::Equal))
            .expect("non-empty");
        if strongest != Sun && c.combust(strongest) {
            continue;
        }
        o.push("pravrajya", vec![format!("{v:?} together in one sign; the strongest, {strongest:?}, is not combust")], v.clone(), vec![strongest]);
    }
    let d = sign_lord(c.sign(Moon) as usize);
    if d != Saturn && c.aspects(Saturn, d) {
        let others = SEVEN.iter().any(|g| *g != Saturn && *g != d && c.aspects(*g, d));
        if !others {
            o.push("pravrajya_saturn", vec![format!("{d:?}, lord of the Moon's sign, is aspected by Saturn and by no other graha")], vec![d, Saturn], vec![Saturn, d]);
        }
    }
}

fn jaimini(o: &mut Out) {
    let c = o.c;
    let lon = |g: DashaLord| c.p(g).sidereal;
    let k = chara_karakas([lon(Sun), lon(Moon), lon(Mars), lon(Mercury), lon(Jupiter), lon(Venus), lon(Saturn)]);
    let ak = k[0].graha;
    for (key, other, label) in [("jaimini_ak_amk", k[1].graha, "Amatyakaraka"), ("jaimini_ak_pk", k[4].graha, "Putrakaraka")] {
        let how = if c.conj(ak, other) {
            Some("conjoined")
        } else if c.exchange(ak, other) {
            Some("exchanging signs")
        } else if signs_aspect(c.sign(ak) as usize, c.sign(other) as usize) {
            Some("in mutual rasi drishti")
        } else {
            None
        };
        if let Some(how) = how {
            o.push(key, vec![format!("Atmakaraka {ak:?} and {label} {other:?} are {how} (seven-karaka scheme)")], vec![ak, other], vec![ak, other]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::PlanetFact;

    /// A hand-built chart: lagna sign and the nine sidereal longitudes, Sun..Ketu.
    fn mk(lagna: u8, lons: [f64; 9]) -> Facts {
        let rasi_lords: [DashaLord; 12] = std::array::from_fn(|i| sign_lord((lagna as usize + i) % 12));
        let planets = NINE
            .iter()
            .zip(lons.iter())
            .map(|(g, l)| {
                let sign = ((l / 30.0).floor() as usize % 12) as u8;
                let rasi_house = ((sign as i32 - lagna as i32).rem_euclid(12) + 1) as u8;
                PlanetFact {
                    graha: *g,
                    sidereal: *l,
                    house: rasi_house,
                    rasi_house,
                    sign,
                    star_lord: Moon,
                    sub_lord: Moon,
                    owns: (1..=12u8).filter(|h| rasi_lords[(*h - 1) as usize] == *g && !matches!(g, Rahu | Ketu)).collect(),
                    retrograde: matches!(g, Rahu | Ketu),
                }
            })
            .collect();
        Facts {
            planets,
            cusps: std::array::from_fn(|i| ((lagna as usize + i) % 12) as f64 * 30.0 + 5.0),
            cusp_sub_lords: [Moon; 12],
            cusp_star_lords: [Moon; 12],
            house_lords: rasi_lords,
            lagna_sign: lagna,
            rasi_lords,
            moon_phase_deg: (lons[1] - lons[0]).rem_euclid(360.0),
        }
    }

    /// Sign index and degree to a longitude.
    fn at(sign: u8, deg: f64) -> f64 {
        sign as f64 * 30.0 + deg
    }

    fn keys(f: &Facts) -> Vec<&'static str> {
        chart_yogas(f).iter().map(|y| y.def.key).collect()
    }
    fn has(f: &Facts, k: &str) -> bool {
        keys(f).contains(&k)
    }
    fn found<'a>(v: &'a [YogaFound], k: &str) -> &'a YogaFound {
        v.iter().find(|y| y.def.key == k).unwrap_or_else(|| panic!("{k} not found"))
    }

    // Signs: 0 Ari 1 Tau 2 Gem 3 Can 4 Leo 5 Vir 6 Lib 7 Sco 8 Sag 9 Cap 10 Aqu 11 Pis

    #[test]
    fn catalogue_is_complete_and_plain() {
        assert!(CATALOG.len() >= 100, "{}", CATALOG.len());
        let nabhasa = CATALOG.iter().filter(|d| d.group == Group::Nabhasa).count();
        assert_eq!(nabhasa, 32);
        let mut ks: Vec<&str> = CATALOG.iter().map(|d| d.key).collect();
        ks.sort_unstable();
        let n = ks.len();
        ks.dedup();
        assert_eq!(ks.len(), n, "duplicate keys");
        for d in CATALOG {
            assert!(!d.source.is_empty() && !d.rule.is_empty(), "{}", d.key);
            for (field, text) in [("title", d.title), ("meaning", d.meaning), ("care", d.care)] {
                assert_eq!(crate::plain::jargon(text), None, "{}.{field}: {text}", d.key);
            }
            assert_eq!(d.good, d.care.is_empty(), "{}: a difficulty needs care words", d.key);
        }
    }

    /// Cancer lagna. Jupiter exalted in Cancer (1st) with the Moon; Mars exalted in
    /// Capricorn (7th); Saturn exalted in Libra (4th); Venus in Taurus (11th); Sun in
    /// Aries (10th, exalted); Mercury in Pisces (9th, debilitated); Rahu Gemini.
    fn royal() -> Facts {
        mk(3, [at(0, 10.0), at(3, 3.0), at(9, 28.0), at(11, 15.0), at(3, 5.0), at(1, 20.0), at(6, 20.0), at(2, 10.0), at(8, 10.0)])
    }

    #[test]
    fn mahapurusha_and_gaja_kesari() {
        let f = royal();
        let k = keys(&f);
        assert!(k.contains(&"hamsa"), "{k:?}");
        assert!(k.contains(&"ruchaka"), "{k:?}");
        assert!(k.contains(&"shasha"), "{k:?}");
        assert!(!k.contains(&"malavya"), "{k:?}"); // Venus own sign but in the 11th
        assert!(!k.contains(&"bhadra"), "{k:?}"); // Mercury debilitated
        assert!(k.contains(&"gaja_kesari"), "{k:?}");
        assert!(k.contains(&"three_exalted"), "{k:?}");
        let ys = chart_yogas(&f);
        let h = found(&ys, "hamsa");
        // Hamsa is lessened by the Moon in Jupiter's sign.
        assert!(h.factors.iter().any(|x| x.contains("lessens")));
        assert_eq!(h.activators, vec![Jupiter]);
        assert!(ys.iter().all(|y| (0.0..=1.0).contains(&y.strength)));
    }

    #[test]
    fn gaja_kesari_negative_cases() {
        // Jupiter debilitated in Capricorn, the Moon in Cancer (7th from it): no yoga.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(5, 10.0), at(5, 20.0), at(9, 10.0), at(6, 10.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(!has(&f, "gaja_kesari"));
        // Jupiter 2nd from the Moon: no yoga.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(5, 10.0), at(5, 20.0), at(4, 25.0), at(6, 10.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(!has(&f, "gaja_kesari"));
        // Jupiter combust in a kendra from the Moon: no yoga.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(5, 10.0), at(1, 2.0), at(0, 14.0), at(10, 10.0), at(10, 20.0), at(1, 1.0), at(7, 1.0)]);
        assert!(!has(&f, "gaja_kesari"));
    }

    #[test]
    fn chandra_yogas_and_kemadruma() {
        // Aries lagna, Moon in Leo (5th), Mars in Virgo (2nd from the Moon), nothing in Cancer.
        let base = [at(0, 10.0), at(4, 10.0), at(5, 10.0), at(0, 20.0), at(8, 5.0), at(1, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)];
        let f = mk(0, base);
        assert!(has(&f, "sunapha") && !has(&f, "anapha") && !has(&f, "durudhura"));
        let mut b = base;
        b[2] = at(3, 10.0); // Mars to Cancer: 12th from the Moon
        let f = mk(0, b);
        assert!(has(&f, "anapha") && !has(&f, "sunapha"));
        b[6] = at(5, 10.0); // Saturn to Virgo: both sides
        let f = mk(0, b);
        assert!(has(&f, "durudhura"));
        // Kemadruma with no cancellation: Libra lagna; Moon in Aries (7th... a kendra
        // from the lagna is fine, the Moon itself does not cancel); every other graha
        // in a panapara or apoklima from both lagna and Moon; Jupiter not aspecting.
        // Lagna Libra (6). Moon Aries (house 7). Sun Gemini (9), Mars Gemini (9),
        // Mercury Gemini (9), Venus Taurus? (2nd from the Moon: no). Use Leo (11, 5th from
        // the Moon), Jupiter Virgo (12, 6th from the Moon: aspects 4, 6, 8 → houses 3, 5, 7
        // from lagna... Jupiter in 12 aspects 4, 6, 8: not the Moon's 7th). Saturn
        // Sagittarius (3, 9th from the Moon).
        let f = mk(6, [at(2, 10.0), at(0, 10.0), at(2, 20.0), at(2, 25.0), at(5, 10.0), at(4, 10.0), at(8, 10.0), at(1, 1.0), at(7, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"kemadruma"), "{k:?}");
        assert!(!k.contains(&"kemadruma_bhanga"));
        // Move Saturn into Capricorn: a kendra from both the lagna (4th) and the Moon (10th).
        let f = mk(6, [at(2, 10.0), at(0, 10.0), at(2, 20.0), at(2, 25.0), at(5, 10.0), at(4, 10.0), at(9, 10.0), at(1, 1.0), at(7, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"kemadruma_bhanga") && !k.contains(&"kemadruma"), "{k:?}");
    }

    #[test]
    fn solar_yogas() {
        // Sun Leo, Mars Virgo (2nd), nothing in Cancer except the Moon (excluded).
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(5, 10.0), at(4, 25.0), at(8, 5.0), at(6, 10.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"vesi") && !k.contains(&"vasi") && !k.contains(&"ubhayachari"), "{k:?}");
        assert!(k.contains(&"budhaditya"));
        // Mercury 15° from the Sun, same sign: not combust (orb 14°), full strength.
        let ys = chart_yogas(&f);
        assert!(!found(&ys, "budhaditya").factors[0].contains("combust"));
        // Venus to Cancer (12th from the Sun) → Ubhayachari.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(5, 10.0), at(4, 25.0), at(8, 5.0), at(3, 20.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(has(&f, "ubhayachari"));
    }

    #[test]
    fn raja_dharma_karma_and_yogakaraka() {
        // Taurus lagna: Saturn rules 9 and 10 (yogakaraka). Mars (7th lord) with Sun
        // (4th lord) in Leo... use Venus (lagna lord) with Mercury (5th lord) in Gemini.
        let f = mk(1, [at(4, 10.0), at(8, 10.0), at(10, 10.0), at(2, 15.0), at(11, 10.0), at(2, 5.0), at(6, 10.0), at(0, 1.0), at(6, 1.0)]);
        let ys = chart_yogas(&f);
        let k: Vec<&str> = ys.iter().map(|y| y.def.key).collect();
        assert!(k.contains(&"yogakaraka"), "{k:?}");
        let r = ys.iter().find(|y| y.def.key == "raja" && y.grahas.contains(&Venus) && y.grahas.contains(&Mercury)).expect("1st and 5th lords conjoined");
        assert!(r.factors[0].contains("conjoined"));
        // Aries lagna: Jupiter (9th lord) and Saturn (10th lord) exchanging: Jupiter in
        // Capricorn, Saturn in Sagittarius.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(0, 10.0), at(5, 20.0), at(9, 10.0), at(1, 10.0), at(8, 10.0), at(2, 1.0), at(8, 1.0)]);
        let ys = chart_yogas(&f);
        let d = found(&ys, "dharma_karmadhipati");
        assert!(d.factors[0].contains("exchanging"));
        assert!(!ys.iter().any(|y| y.def.key == "raja" && y.grahas.contains(&Jupiter) && y.grahas.contains(&Saturn)));
        // Mutual aspect: Sun (5th lord) in Aries and Saturn (10th lord) in Libra.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(1, 10.0), at(11, 20.0), at(2, 10.0), at(10, 10.0), at(6, 10.0), at(2, 1.0), at(8, 1.0)]);
        let ys = chart_yogas(&f);
        assert!(ys.iter().any(|y| y.def.key == "raja" && y.factors[0].contains("mutual aspect") && y.grahas.contains(&Sun) && y.grahas.contains(&Saturn)));
        // Negative: Aries lagna, no kendra and trikona lords related.
        let f = mk(0, [at(1, 10.0), at(5, 10.0), at(10, 10.0), at(2, 2.0), at(7, 10.0), at(11, 10.0), at(3, 15.0), at(2, 25.0), at(8, 25.0)]);
        assert!(!has(&f, "raja") && !has(&f, "dharma_karmadhipati"), "{:?}", keys(&f));
    }

    #[test]
    fn parivartana_three_kinds() {
        // Aries lagna. Mars in Taurus (2nd) and Venus in Aries (1st): 1 and 2 → Maha.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(1, 10.0), at(5, 20.0), at(8, 10.0), at(0, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "maha_parivartana"));
        // Mercury (3rd and 6th lord) in Aries?  Use Mercury in Leo (5th) and the Sun in
        // Gemini (3rd): houses 3 and 5 → Khala.
        let f = mk(0, [at(2, 10.0), at(3, 10.0), at(0, 10.0), at(4, 20.0), at(8, 10.0), at(1, 10.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"khala_parivartana") && !k.contains(&"maha_parivartana"), "{k:?}");
        // Jupiter (9, 12) in Scorpio (8th) and Mars in Sagittarius (9th): houses 8 and 9 → Dainya.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(8, 10.0), at(5, 20.0), at(7, 10.0), at(1, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"dainya_parivartana"), "{k:?}");
        assert!(!k.contains(&"maha_parivartana") && !k.contains(&"khala_parivartana"));
    }

    #[test]
    fn viparita_harsha_sarala_vimala() {
        // Aries lagna: 6th lord Mercury in Scorpio (8th) → Harsha; 8th lord Mars is the
        // lagna lord → Sarala much weakened when in a dusthana; 12th lord Jupiter in Virgo
        // (6th) → Vimala.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(1, 10.0), at(7, 20.0), at(5, 10.0), at(9, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"harsha") && k.contains(&"vimala") && !k.contains(&"sarala"), "{k:?}");
        // Negative: the 6th lord in a kendra.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(1, 10.0), at(3, 20.0), at(8, 10.0), at(9, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(!has(&f, "harsha") && !has(&f, "vimala"));
    }

    #[test]
    fn neecha_bhanga_rules() {
        // Aries lagna, Sun debilitated in Libra (7th). Its dispositor Venus in Cancer
        // (4th, a kendra) → cancelled; Saturn (exalted in Libra) in Capricorn (10th) too.
        let f = mk(0, [at(6, 10.0), at(1, 10.0), at(4, 10.0), at(7, 2.0), at(8, 10.0), at(3, 10.0), at(9, 10.0), at(2, 1.0), at(8, 1.0)]);
        let ys = chart_yogas(&f);
        let n = found(&ys, "neecha_bhanga");
        assert!(n.factors.iter().any(|x| x.contains("dispositor Venus is in a kendra")), "{:?}", n.factors);
        assert!(n.factors.iter().any(|x| x.contains("Saturn, exalted in this sign")), "{:?}", n.factors);
        // Negative. Taurus lagna and a Taurus Moon, so the kendras are the fixed signs
        // from both. Sun debilitated at 11° Libra (navamsa Capricorn, not Aries; house 6).
        // Dispositor Venus in Gemini, exaltation lord Mars in Aries (11 signs apart, not
        // mutual kendras), Saturn (exalted in Libra) in Pisces: none in a kendra from the
        // lagna or the Moon, none joining or aspecting the Sun, no exchange.
        let f = mk(1, [at(6, 11.0), at(1, 10.0), at(0, 10.0), at(6, 20.0), at(2, 25.0), at(2, 15.0), at(11, 10.0), at(3, 1.0), at(9, 1.0)]);
        assert!(!has(&f, "neecha_bhanga"), "{:?}", chart_yogas(&f).iter().filter(|y| y.def.key == "neecha_bhanga").map(|y| y.factors.clone()).collect::<Vec<_>>());
    }

    #[test]
    fn nabhasa_patterns() {
        // Yupa: all seven in the 1st to 4th, each filled (Aries lagna).
        let f = mk(0, [at(0, 10.0), at(1, 10.0), at(2, 10.0), at(3, 10.0), at(0, 20.0), at(1, 20.0), at(2, 20.0), at(5, 1.0), at(11, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"yupa"), "{k:?}");
        assert!(!k.iter().any(|x| ["vallaki", "daama", "pasha", "kedara", "shoola", "yuga", "gola"].contains(x)));
        // Kamala: the four kendras, each filled.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(6, 10.0), at(9, 10.0), at(0, 20.0), at(3, 20.0), at(9, 20.0), at(5, 1.0), at(11, 1.0)]);
        assert!(has(&f, "kamala"));
        // Gada: 1st and 4th only.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(0, 15.0), at(3, 20.0), at(0, 20.0), at(3, 25.0), at(0, 25.0), at(5, 1.0), at(11, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"gada") && !k.contains(&"kamala"), "{k:?}");
        // Shakata (Nabhasa): 1st and 7th.
        let f = mk(0, [at(0, 10.0), at(6, 10.0), at(0, 15.0), at(6, 20.0), at(0, 20.0), at(6, 25.0), at(0, 25.0), at(5, 1.0), at(11, 1.0)]);
        assert!(has(&f, "nabhasa_shakata"));
        // Gola: all in one sign. Yuga: two signs (but these also form Ashraya? Taurus and
        // Leo are both fixed → Musala, so Sankhya does not apply).
        let f = mk(0, [at(1, 10.0), at(1, 11.0), at(1, 12.0), at(1, 13.0), at(1, 14.0), at(1, 15.0), at(1, 16.0), at(5, 1.0), at(11, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"musala") && !k.contains(&"gola"), "{k:?}");
        // Seven signs, nothing else: Vallaki. Aries lagna; grahas in 1, 2, 3, 5, 6, 9, 11.
        let f = mk(0, [at(0, 10.0), at(1, 10.0), at(2, 10.0), at(4, 10.0), at(5, 10.0), at(8, 10.0), at(10, 10.0), at(3, 1.0), at(9, 1.0)]);
        let ys = chart_yogas(&f);
        let v = found(&ys, "vallaki");
        assert!(v.lifelong && v.activators.is_empty());
        // Rajju: all movable; Sankhya suppressed.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(6, 10.0), at(9, 10.0), at(0, 20.0), at(3, 20.0), at(6, 20.0), at(5, 1.0), at(11, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"rajju") && !k.contains(&"pasha"), "{k:?}");
    }

    #[test]
    fn lakshmi_amala_saraswati() {
        // Aries lagna. 9th lord Jupiter in Sagittarius (own, 9th); lagna lord Mars in
        // Capricorn (exalted, 10th) → Lakshmi. Venus alone in the 10th? Mars is there,
        // so Amala must come from the Moon: Moon in Gemini, 10th from it Pisces holds
        // Mercury? Mercury is debilitated there; benefic unless joined by a malefic.
        let f = mk(0, [at(4, 10.0), at(2, 10.0), at(9, 10.0), at(11, 10.0), at(8, 5.0), at(10, 10.0), at(6, 10.0), at(1, 1.0), at(7, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"lakshmi"), "{k:?}");
        assert!(k.contains(&"amala"), "{k:?}");
        // Saraswati: Jupiter own in Sagittarius (9th), Venus in Taurus (2nd), Mercury in
        // Gemini (3rd) → no (the 3rd is not allowed).
        assert!(!k.contains(&"saraswati"));
        let f = mk(0, [at(4, 10.0), at(2, 10.0), at(9, 10.0), at(4, 25.0), at(8, 5.0), at(1, 10.0), at(6, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(has(&f, "saraswati"), "{:?}", keys(&f));
    }

    #[test]
    fn shakata_and_its_cancellation() {
        // Moon in Aries, Jupiter in Virgo (6th from the Moon); Aries lagna → Jupiter in the
        // 6th from the lagna: Shakata stands.
        let f = mk(0, [at(4, 10.0), at(0, 10.0), at(9, 10.0), at(4, 25.0), at(5, 5.0), at(10, 10.0), at(6, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(has(&f, "shakata"));
        // Gemini lagna: Jupiter in Virgo is in the 4th from the lagna → cancelled.
        let f = mk(2, [at(4, 10.0), at(0, 10.0), at(9, 10.0), at(4, 25.0), at(5, 5.0), at(10, 10.0), at(6, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(!has(&f, "shakata"));
    }

    #[test]
    fn adhi_vasumati_kalanidhi() {
        // Moon in Aries; Mercury Virgo (6th), Venus Libra (7th), Jupiter Scorpio (8th);
        // Sun in Leo, Mercury alone in Virgo (benefic); nodes in Cancer and Capricorn.
        let f = mk(0, [at(4, 10.0), at(0, 10.0), at(9, 10.0), at(5, 10.0), at(7, 5.0), at(6, 10.0), at(10, 10.0), at(3, 1.0), at(9, 1.0)]);
        let ys = chart_yogas(&f);
        let a = found(&ys, "adhi");
        assert!(a.factors[0].contains("full yoga"));
        assert_eq!(a.activators, vec![Mercury, Jupiter, Venus]);
        // Vasumati from the Moon in Aries: Mercury Gemini (3rd), Jupiter Virgo (6th),
        // Venus Capricorn (10th).
        let f = mk(0, [at(1, 10.0), at(0, 10.0), at(8, 10.0), at(2, 10.0), at(5, 5.0), at(9, 10.0), at(4, 10.0), at(3, 1.0), at(9, 1.0)]);
        assert!(has(&f, "vasumati"));
        // Kalanidhi: Aries lagna, Jupiter in Taurus (2nd), Mercury with it, Venus in
        // Scorpio aspecting it.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(9, 10.0), at(1, 25.0), at(1, 5.0), at(7, 10.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "kalanidhi"));
    }

    #[test]
    fn adhi_blocked_by_a_malefic() {
        // As above but Ketu sits in Scorpio, the 8th from an Aries Moon: blocked.
        let f = mk(0, [at(4, 10.0), at(0, 10.0), at(9, 10.0), at(5, 10.0), at(7, 5.0), at(6, 10.0), at(10, 10.0), at(1, 1.0), at(7, 1.0)]);
        assert!(!has(&f, "adhi"));
        // Only one benefic in the three places: no Adhi.
        let f = mk(0, [at(4, 10.0), at(0, 10.0), at(9, 10.0), at(4, 20.0), at(7, 5.0), at(1, 10.0), at(10, 10.0), at(3, 1.0), at(9, 1.0)]);
        assert!(!has(&f, "adhi"));
    }

    #[test]
    fn kartari_and_chatussagara() {
        // Aries lagna: Venus in Taurus (2nd), Jupiter in Pisces (12th) → Shubha kartari.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(9, 10.0), at(5, 10.0), at(11, 5.0), at(1, 10.0), at(6, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "shubha_kartari") && !has(&f, "papa_kartari"));
        // Saturn in Taurus and Mars in Pisces → Papa kartari.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(11, 10.0), at(5, 10.0), at(8, 5.0), at(4, 25.0), at(1, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "papa_kartari") && !has(&f, "shubha_kartari"));
        // Chatussagara: Aries lagna with grahas in 1, 4, 7, 10.
        let f = mk(0, [at(0, 10.0), at(3, 10.0), at(6, 10.0), at(9, 10.0), at(8, 5.0), at(4, 25.0), at(1, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "chatussagara"));
    }

    #[test]
    fn hardship_and_pravrajya() {
        // Aries lagna: 10th lord Saturn in Virgo (6th) → Duryoga; 11th lord Saturn too → Daridra.
        let f = mk(0, [at(4, 10.0), at(3, 10.0), at(9, 10.0), at(5, 10.0), at(8, 5.0), at(4, 25.0), at(5, 20.0), at(2, 1.0), at(8, 1.0)]);
        let k = keys(&f);
        assert!(k.contains(&"duryoga") && k.contains(&"daridra"), "{k:?}");
        // Pravrajya: four grahas in Sagittarius, Jupiter (own) strongest, not combust;
        // the Sun is far away.
        let f = mk(0, [at(4, 10.0), at(8, 10.0), at(8, 12.0), at(8, 14.0), at(8, 5.0), at(5, 25.0), at(1, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "pravrajya"), "{:?}", keys(&f));
        // Only three together: no Pravrajya.
        let f = mk(0, [at(4, 10.0), at(8, 10.0), at(8, 12.0), at(9, 14.0), at(8, 5.0), at(5, 25.0), at(1, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(!has(&f, "pravrajya"));
    }

    #[test]
    fn kala_sarpa_and_its_absence() {
        let f = mk(0, [at(1, 10.0), at(2, 10.0), at(3, 10.0), at(4, 10.0), at(5, 5.0), at(1, 25.0), at(4, 20.0), at(0, 1.0), at(6, 1.0)]);
        assert!(has(&f, "kala_sarpa"));
        let f = mk(0, [at(1, 10.0), at(2, 10.0), at(9, 10.0), at(4, 10.0), at(5, 5.0), at(1, 25.0), at(4, 20.0), at(0, 1.0), at(6, 1.0)]);
        assert!(!has(&f, "kala_sarpa"));
    }

    #[test]
    fn jaimini_raja() {
        // Venus 29° (Atmakaraka) in Aries, Jupiter 28° (Amatyakaraka) in Leo: movable
        // Aries aspects fixed Leo → AK-AmK raja yoga.
        let f = mk(0, [at(1, 10.0), at(2, 5.0), at(3, 3.0), at(4, 2.0), at(4, 28.0), at(0, 29.0), at(6, 1.0), at(2, 1.0), at(8, 1.0)]);
        assert!(has(&f, "jaimini_ak_amk"));
        // Jupiter to Taurus instead: Aries does not aspect the sign beside it.
        let f = mk(0, [at(1, 10.0), at(2, 5.0), at(3, 3.0), at(4, 2.0), at(1, 28.0), at(0, 29.0), at(6, 1.0), at(2, 1.0), at(8, 1.0)]);
        assert!(!has(&f, "jaimini_ak_amk"));
    }

    #[test]
    fn strength_reflects_dignity_and_combustion() {
        // Exalted Jupiter in the lagna is stronger than debilitated Jupiter in the 6th.
        let a = mk(3, [at(0, 10.0), at(1, 10.0), at(9, 10.0), at(2, 10.0), at(3, 5.0), at(1, 20.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        let b = mk(4, [at(0, 10.0), at(1, 10.0), at(9, 10.0), at(2, 10.0), at(9, 5.0), at(1, 20.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(graha_strength(&a, Jupiter) > 0.8);
        assert!(graha_strength(&b, Jupiter) < 0.2);
        // Combust Venus is weaker than the same Venus far from the Sun.
        let c1 = mk(0, [at(11, 25.0), at(1, 10.0), at(9, 10.0), at(2, 10.0), at(3, 5.0), at(11, 27.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        let c2 = mk(0, [at(9, 25.0), at(1, 10.0), at(9, 10.0), at(2, 10.0), at(3, 5.0), at(11, 27.0), at(10, 10.0), at(2, 1.0), at(8, 1.0)]);
        assert!(graha_strength(&c1, Venus) < graha_strength(&c2, Venus));
    }
}
