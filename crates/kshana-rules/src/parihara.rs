//! Parihara: remedies that fit the cause, ranked by fit and effort, actions first.
//!
//! The cause is located in the chart and the period: the weakest of the running period
//! lords, or the lord of the area's house. Each graha carries a dictionary of remedies
//! drawn from the Jyotisha remedial tradition: conduct and service from the dharma
//! reading of each graha's signification; fasts (vrata), donations (daana), japa
//! counts, deities and temples from Brihat Parashara Hora Shastra (Graha shanti),
//! the Puranic graha-daana lists, Lal Kitab and Kerala and Tamil temple practice.
//! When a life area is known, its own remedies are added. Conduct, service and
//! discipline always rank above fast, donation, mantra and pooja, and the ritual items
//! are marked optional: nothing in this module requires a purchase or a priest.

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
    Vrata,
    Mantra,
    Pooja,
    /// Food given to the graha's people.
    Anna,
    /// Creatures fed: crows, cows, dogs, ants, fish, birds.
    Jeeva,
    /// Water, trees and places tended.
    Prakriti,
    /// Colours worn and kept from, small things carried.
    Vastra,
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
    /// The day to begin or keep it, in words ("Saturday", "any day").
    pub day: &'static str,
    /// Where it comes from.
    pub source: &'static str,
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
    /// The graha's deity, day, metal and grain in one line, for the header.
    pub signature: String,
}

/// `(kind, title, why, effort, day, source)`.
pub type Row = (RemedyKind, &'static str, &'static str, &'static str, &'static str, &'static str);

const DHARMA: &str = "Dharma reading of the graha's signification";
const BPHS: &str = "BPHS, Graha shanti adhyaya";
const PURANA: &str = "Puranic graha-daana lists (Matsya, Yajnavalkya)";
const LAL: &str = "Lal Kitab";
const TEMPLE: &str = "Temple practice (Navagraha kshetras, Kerala and Tamil Nadu)";
const MUHURTA: &str = "Vrata tradition (Muhurta and Dharma Sindhu)";

/// The graha's own dictionary.
pub fn table(g: DashaLord) -> Vec<Row> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Karma, "Keep your word to those who depend on you", "The Sun is authority earned by reliability. Do what you said, when you said.", "daily", "any day", DHARMA),
            (Karma, "Mend things with your father, or honour his memory", "The Sun is the father; a strained bond with him is the Sun's wound.", "this week", "Sunday", DHARMA),
            (Karma, "Take responsibility for one thing you have been blaming others for", "The Sun grows with ownership and shrinks with excuses.", "this week", "any day", DHARMA),
            (Seva, "Serve your father, or an elder man without family", "The Sun's karaka is the father; service to him is the direct remedy.", "weekly", "Sunday", DHARMA),
            (Seva, "Help someone get a government paper or a right they are owed", "The Sun is the state and justice; clearing another's path clears yours.", "once", "any day", DHARMA),
            (Discipline, "Be up before sunrise for 40 days", "The Sun is strengthened by meeting it; the sunrise time is computed for your place.", "40 days", "start Sunday", MUHURTA),
            (Discipline, "Twelve rounds of Surya namaskara at dawn", "The body's salute to the Sun, done daily, steadies heart, spine and will.", "20 min · daily", "any day", TEMPLE),
            (Daana, "Give wheat or jaggery on a Sunday, by hand", "The Sun's grains, given in Surya hora, to someone who needs them.", "once", "Sunday", PURANA),
            (Daana, "Give a copper vessel or red cloth to a man who works in the sun", "Copper and red are the Sun's; a labourer under the Sun is its person.", "once", "Sunday", PURANA),
            (Vrata, "Sunday fast: one meal, no salt, before sunset", "The Sun's vrata, kept for 12 Sundays, is the classical shanti.", "12 Sundays", "Sunday", BPHS),
            (Mantra, "Aditya Hridayam on Sundays", "Supports the discipline above; not a substitute for it.", "if you wish", "Sunday", TEMPLE),
            (Mantra, "Om hraam hreem hraum sah Suryaya namah · 7,000 in all", "The Sun's beeja; Parashara gives the count. A mala a day finishes it in two months.", "if you wish", "start Sunday", BPHS),
            (Pooja, "Surya arghya at sunrise: water from a copper vessel, facing east", "Optional. Listed so you know it is not hidden.", "not required", "any day", TEMPLE),
            (Pooja, "Shiva or Surya temple on a Sunday; Suryanar Kovil for the full shanti", "Optional. The Sun's kshetra among the nine.", "not required", "Sunday", TEMPLE),
        ],
        DashaLord::Moon => vec![
            (Karma, "Care for your mother, in person, this week", "The Moon is the mother and the mind; the remedy for a weak Moon is to tend to both.", "weekly", "Monday", DHARMA),
            (Karma, "Say one true thing you have been holding in, gently", "A troubled Moon hides feeling until it floods; speaking it early is the cure.", "this week", "any day", DHARMA),
            (Karma, "Forgive one person in your own mind, without telling them", "The Moon carries grudges as tides; letting one go lightens all of them.", "once", "full moon", DHARMA),
            (Seva, "Serve women and children who have no one", "The Moon's people. An hour of help, not money alone.", "weekly", "Monday", DHARMA),
            (Seva, "Feed and comfort someone who is grieving", "The Moon is the comforter; comforting another steadies your own mind.", "once", "any day", DHARMA),
            (Discipline, "Fixed sleep and waking times for 40 days", "A steady routine is the Moon's medicine.", "40 days", "start Monday", MUHURTA),
            (Discipline, "Ten minutes of still breathing each night, no screen after it", "The Moon is the mind; a quiet close to the day settles it.", "10 min · nightly", "any day", DHARMA),
            (Daana, "Give rice, milk or white cloth on a Monday", "The Moon's offerings, in Chandra hora.", "once", "Monday", PURANA),
            (Daana, "Give silver (even a small coin) or a conch to a temple or a woman elder", "Silver and the conch are the Moon's.", "once", "Monday", PURANA),
            (Vrata, "Somavara vrata: Mondays, one meal after sunset, for 16 Mondays", "The Moon's fast, kept for Shiva, is the classical cure for a shaken mind.", "16 Mondays", "Monday", MUHURTA),
            (Mantra, "Om shraam shreem shraum sah Chandraya namah · 11,000 in all", "The Moon's beeja; Parashara's count.", "if you wish", "start Monday", BPHS),
            (Mantra, "Shiva Panchakshari on Mondays", "Optional support; Shiva wears the Moon.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Abhisheka with milk on a Monday", "Optional.", "not required", "Monday", TEMPLE),
            (Pooja, "Gauri or Parvati temple; Thingalur for the full shanti", "Optional. The Moon's kshetra.", "not required", "Monday", TEMPLE),
        ],
        DashaLord::Mars => vec![
            (Karma, "End one quarrel on your own initiative", "Mars rules conflict; a weak Mars is remedied by courage used for peace.", "this week", "Tuesday", DHARMA),
            (Karma, "Stand up, once, for someone weaker than you", "Mars is the protector; protecting restores it.", "this week", "any day", DHARMA),
            (Karma, "Repay a brother or a sibling what you owe, in money or in time", "Mars is the brothers; a debt there is Mars's debt.", "this month", "Tuesday", DHARMA),
            (Seva, "Donate blood, or serve those who protect others", "Mars is blood and guardianship.", "once", "any day", DHARMA),
            (Seva, "Cook and serve a hot meal to labourers or soldiers", "Mars's people, fed by your own hands.", "once", "Tuesday", TEMPLE),
            (Discipline, "No anger spoken aloud for 40 days", "Restraint is the discipline Mars respects.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Hard physical work or sport, 30 minutes, until you sweat", "Mars that is not spent in effort is spent in temper.", "30 min · daily", "any day", DHARMA),
            (Daana, "Give red lentils on a Tuesday", "Mars's grain, in Mangala hora.", "once", "Tuesday", PURANA),
            (Daana, "Give red cloth, copper or coral-coloured sweets to a young man who serves", "Mars's colour and metal.", "once", "Tuesday", PURANA),
            (Vrata, "Tuesday fast with one meal of plain food; Mangala Gauri for women", "The Mars vrata, kept 21 Tuesdays, for courage and for marriage.", "21 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Tuesdays", "Optional support.", "if you wish", "Tuesday", TEMPLE),
            (Mantra, "Om kraam kreem kraum sah Bhaumaya namah · 10,000 in all", "The Mars beeja; Parashara's count.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Subrahmanya or Hanuman temple on a Tuesday", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Mangala shanti; Vaitheeswaran Koil for the full rite", "Optional. The Mars kshetra.", "not required", "Tuesday", TEMPLE),
        ],
        DashaLord::Mercury => vec![
            (Karma, "Speak less and only what is true for a week", "Mercury is speech and trade; truthful, measured speech is its remedy.", "7 days", "Wednesday", DHARMA),
            (Karma, "Settle one account you have let slide, to the rupee", "Mercury is accounts; a loose ledger is a loose Mercury.", "this week", "Wednesday", DHARMA),
            (Karma, "Return a borrowed book, tool or favour", "Mercury is exchange kept even.", "this week", "any day", LAL),
            (Seva, "Teach a child or an adult learner for an hour", "Mercury is learning passed on.", "weekly", "Wednesday", DHARMA),
            (Seva, "Help a small trader or a student with paperwork or accounts", "Mercury's people.", "once", "any day", DHARMA),
            (Discipline, "No gossip, no idle scrolling, for 40 days", "Mercury weakens with scattered attention.", "40 days", "start Wednesday", DHARMA),
            (Discipline, "Write one page by hand every morning", "Mercury is the hand and the word; writing daily sharpens both.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give green moong on a Wednesday", "Mercury's grain, in Budha hora.", "once", "Wednesday", PURANA),
            (Daana, "Give green cloth, a bronze vessel or books to a student", "Mercury's colour, metal and gift.", "once", "Wednesday", PURANA),
            (Vrata, "Wednesday fast, one meal, with Vishnu in mind", "The Mercury vrata.", "7 Wednesdays", "Wednesday", MUHURTA),
            (Mantra, "Vishnu Sahasranama on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Mantra, "Om braam breem braum sah Budhaya namah · 9,000 in all", "The Mercury beeja; Parashara's count.", "if you wish", "start Wednesday", BPHS),
            (Pooja, "Vishnu or Krishna temple on a Wednesday; tulasi water", "Optional.", "not required", "Wednesday", TEMPLE),
            (Pooja, "Budha shanti; Thiruvenkadu for the full rite", "Optional. The Mercury kshetra.", "not required", "Wednesday", TEMPLE),
        ],
        DashaLord::Jupiter => vec![
            (Karma, "Honour a teacher you have neglected", "Jupiter is the guru; gratitude restores it.", "this week", "Thursday", DHARMA),
            (Karma, "Keep one promise made to a child", "Jupiter is children and the keeping of faith with them.", "this week", "any day", DHARMA),
            (Karma, "Give advice only when asked, and then fully", "Jupiter weakens when wisdom is forced or withheld.", "40 days", "any day", DHARMA),
            (Seva, "Feed students, or fund one student's books", "Jupiter is knowledge given freely.", "once", "Thursday", DHARMA),
            (Seva, "Serve a priest, a teacher or an elder scholar without being asked", "Jupiter's people.", "weekly", "Thursday", TEMPLE),
            (Discipline, "Read scripture or a serious book daily for 40 days", "Jupiter grows with study.", "40 days", "start Thursday", DHARMA),
            (Discipline, "No lie, not even a small one, for 40 days", "Jupiter is truth; the discipline is exact.", "40 days", "any day", DHARMA),
            (Daana, "Give turmeric or chana dal on a Thursday", "Jupiter's offerings, in Guru hora.", "once", "Thursday", PURANA),
            (Daana, "Give yellow cloth, a book or (if you can) a little gold to a teacher", "Jupiter's colour, gift and metal.", "once", "Thursday", PURANA),
            (Vrata, "Thursday fast: yellow food once, no salt, Guru in mind", "The Jupiter vrata, 16 Thursdays, for marriage, children and wisdom.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Om graam greem graum sah Gurave namah · 19,000 in all", "The Jupiter beeja; Parashara's count.", "if you wish", "start Thursday", BPHS),
            (Mantra, "Guru stotra or Dakshinamurthy stotra on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Dakshinamurthy or Vishnu temple on a Thursday", "Optional.", "not required", "Thursday", TEMPLE),
            (Pooja, "Guru shanti; Alangudi for the full rite", "Optional. The Jupiter kshetra.", "not required", "Thursday", TEMPLE),
        ],
        DashaLord::Venus => vec![
            (Karma, "Keep a promise made in a relationship", "Venus is bonds kept; keeping them is the remedy.", "this week", "Friday", DHARMA),
            (Karma, "Say thank you, specifically, to the person who makes your home", "Venus is the comfort others give; naming it feeds it.", "this week", "Friday", DHARMA),
            (Karma, "Make one thing beautiful: a corner, a meal, a letter", "Venus is beauty made, not bought.", "this week", "any day", DHARMA),
            (Seva, "Serve women, artists or the sick with your skill", "Venus is grace given.", "weekly", "Friday", DHARMA),
            (Seva, "Help a couple who are struggling, or a girl's education", "Venus's people.", "once", "Friday", DHARMA),
            (Discipline, "Simplify one indulgence for 40 days", "Venus weakens with excess.", "40 days", "start Friday", DHARMA),
            (Discipline, "Clean clothes, clean room, clean words, for 40 days", "Venus is refinement in small things.", "40 days", "any day", LAL),
            (Daana, "Give curd or white sweets on a Friday", "Venus's offerings, in Shukra hora.", "once", "Friday", PURANA),
            (Daana, "Give white cloth, rice, ghee or silver to a woman in need", "Venus's gifts.", "once", "Friday", PURANA),
            (Vrata, "Friday fast with white food once; Vaibhava Lakshmi vrata", "The Venus vrata, for marriage, comfort and the arts.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Om draam dreem draum sah Shukraya namah · 16,000 in all", "The Venus beeja; Parashara's count.", "if you wish", "start Friday", BPHS),
            (Mantra, "Sri Suktam or Lakshmi Ashtottara on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi or Devi temple on a Friday", "Optional.", "not required", "Friday", TEMPLE),
            (Pooja, "Shukra shanti; Kanjanur for the full rite", "Optional. The Venus kshetra.", "not required", "Friday", TEMPLE),
        ],
        DashaLord::Saturn => vec![
            (Karma, "Finish the work you have postponed longest", "Saturn pays completed duty and withholds for avoided duty.", "20 min · daily", "any day", DHARMA),
            (Karma, "Pay every worker and helper on time, in full, this month", "Saturn is the labourer; his unpaid wage is Saturn's grievance.", "this month", "Saturday", DHARMA),
            (Karma, "Visit someone old who is alone", "Saturn is age and loneliness; sitting with it is the remedy.", "weekly", "Saturday", DHARMA),
            (Seva, "Serve the old or the labouring on Saturdays", "Saturn signifies the elderly and workers. An hour of real help.", "1 h · Saturdays", "Saturday", DHARMA),
            (Seva, "Clean a public place, a temple floor or a shared stair, yourself", "Saturn is humble work done without witness.", "once", "Saturday", TEMPLE),
            (Discipline, "Rise before the Sun for 40 days", "Discipline is the Saturn remedy that needs no one else.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Walk instead of ride for one errand a day", "Saturn is patience in the body.", "daily", "any day", LAL),
            (Daana, "Give black til or a blanket on a Saturday", "By hand, in Shani hora, to a person who needs it.", "once", "Saturday", PURANA),
            (Daana, "Give iron (a tool, a pan) or mustard oil to a labourer", "Saturn's metal and oil.", "once", "Saturday", PURANA),
            (Daana, "Feed crows and a black dog on Saturdays", "Saturn's creatures, in temple practice.", "Saturdays", "Saturday", TEMPLE),
            (Vrata, "Saturday fast, one meal after sunset, for 19 Saturdays", "The Saturn vrata through sade sati and dhaiya.", "19 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays", "Optional. Supports the discipline above.", "if you wish", "Saturday", TEMPLE),
            (Mantra, "Om praam preem praum sah Shanaischaraya namah · 23,000 in all", "The Saturn beeja; Parashara's count.", "if you wish", "start Saturday", BPHS),
            (Pooja, "Til-oil lamp to Shani or Hanuman on a Saturday", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Shani shanti; Thirunallar for the full rite", "Optional. The Saturn kshetra.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Rahu => vec![
            (Karma, "Stop one shortcut or deception you have been running", "Rahu is the shortcut; refusing it is the remedy.", "this week", "any day", DHARMA),
            (Karma, "Tell the one person you have been avoiding the plain truth", "Rahu is the fog between people; clearing it clears Rahu.", "this week", "Saturday", DHARMA),
            (Karma, "Leave one obsession alone for a week: the feed, the rival, the fear", "Rahu is appetite without end.", "7 days", "any day", DHARMA),
            (Seva, "Serve the outcast, the sick without family, the foreigner", "Rahu's people.", "weekly", "Saturday", DHARMA),
            (Seva, "Help a migrant worker or a stranger far from home", "Rahu is the foreign and the displaced.", "once", "any day", DHARMA),
            (Discipline, "No intoxicants for 40 days", "Rahu feeds on confusion.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Eat only what you have cooked or seen cooked, for 21 days", "Rahu is the unknown ingredient.", "21 days", "any day", LAL),
            (Daana, "Give mustard oil or blue cloth on a Saturday", "Rahu's offerings.", "once", "Saturday", PURANA),
            (Daana, "Give a coconut, lead or a dark blanket; let a coconut float in running water", "Rahu's gifts in temple and Lal Kitab practice.", "once", "Saturday", LAL),
            (Vrata, "Saturday fast with Durga in mind, 18 Saturdays", "The Rahu vrata.", "18 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Om bhraam bhreem bhraum sah Rahave namah · 18,000 in all", "The Rahu beeja; Parashara's count.", "if you wish", "start Saturday", BPHS),
            (Mantra, "Durga Saptashati or Durga mantra on Saturdays", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga temple; Rahu kala pooja at a Devi shrine", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Rahu shanti; Thirunageswaram for the full rite", "Optional. The Rahu kshetra.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Ketu => vec![
            (Karma, "Finish something you abandoned", "Ketu is detachment misapplied; completing restores it.", "this week", "any day", DHARMA),
            (Karma, "Apologise for one thing you walked away from", "Ketu leaves without a word; the remedy is the word.", "this week", "Tuesday", DHARMA),
            (Karma, "Give away ten things you do not use", "Ketu is release done rightly.", "this week", "any day", DHARMA),
            (Seva, "Feed and care for stray dogs, or serve a renunciate", "Ketu's charges.", "weekly", "Tuesday", DHARMA),
            (Seva, "Sit with the dying or the very ill, in silence", "Ketu is the threshold; keeping it with another is its highest service.", "once", "any day", DHARMA),
            (Discipline, "Live simply for 40 days: one meal plain, one hour of silence", "Ketu rewards simplicity.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Ten minutes of meditation on the breath, daily", "Ketu is moksha; the breath is its door.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give a blanket or sesame", "Ketu's offerings.", "once", "Tuesday", PURANA),
            (Daana, "Give a multicoloured cloth or a lamp to a temple of Ganesha", "Ketu's gifts.", "once", "Tuesday", TEMPLE),
            (Vrata, "Tuesday fast with Ganesha in mind; Sankashti Chaturthi", "The Ketu vrata.", "7 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Ganesha mantra · Om gam Ganapataye namah", "Optional support.", "if you wish", "any day", TEMPLE),
            (Mantra, "Om sraam sreem sraum sah Ketave namah · 17,000 in all", "The Ketu beeja; Parashara's count.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Ganesha temple on a Tuesday or Chaturthi", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Ketu shanti; Keezhperumpallam for the full rite", "Optional. The Ketu kshetra.", "not required", "Tuesday", TEMPLE),
        ],
    }
}

/// Remedies for a life area, added when the matter is known.
pub fn life_table(area_name: &str) -> Vec<Row> {
    use RemedyKind::*;
    match area_name {
        "Marriage" => vec![
            (Karma, "Treat your partner's family as your own this month", "The 7th is the partner and the 2nd their family; marriage is remedied where it is strained.", "this month", "any day", DHARMA),
            (Karma, "Settle an old hurt with a former partner in your own mind", "The 7th house carries every bond; one left bleeding weakens the next.", "once", "Friday", DHARMA),
            (Seva, "Help with a wedding you are not part of; give to a girl's marriage", "Marriage given to another returns.", "once", "Friday", TEMPLE),
            (Vrata, "Swayamvara Parvathi or Mangala Gauri vrata on Tuesdays or Fridays", "The classical vrata for marriage and its harmony.", "16 weeks", "Tuesday", MUHURTA),
            (Mantra, "Swayamvara Parvathi mantra, or Katyayani for women, Shiva for men", "Optional support, from the marriage tradition.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Umamaheshwara pooja; a visit to a Parvati or Subrahmanya temple as a couple", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Career" => vec![
            (Karma, "Do the part of your work you have been avoiding, first thing", "The 10th is karma itself; the avoided task is the blocked house.", "daily", "any day", DHARMA),
            (Karma, "Give your superior the respect you expect from those under you", "The 10th is authority both ways.", "this week", "Sunday", DHARMA),
            (Seva, "Mentor someone junior without credit", "The 10th grows when it lifts the 6th.", "weekly", "any day", DHARMA),
            (Discipline, "Twelve Surya namaskara before work", "The Sun is the career karaka; meet it before you meet the day.", "15 min · daily", "Sunday", TEMPLE),
            (Vrata, "Sunday or Saturday fast according to the cause above", "Sun for recognition, Saturn for stability of work.", "12 weeks", "Sunday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays; Aditya Hridayam on Sundays", "Optional support.", "if you wish", "Saturday", TEMPLE),
        ],
        "Wealth" => vec![
            (Karma, "Write down every rupee in and out for 40 days", "The 2nd is what is kept; the unwatched account leaks.", "5 min · daily", "any day", DHARMA),
            (Karma, "Repay the oldest small debt, and lend nothing for 40 days", "The 11th gives to one who does not scatter.", "40 days", "any day", LAL),
            (Seva, "Give one tenth of a windfall away before you spend it", "Wealth kept whole is wealth that stays.", "when it comes", "any day", DHARMA),
            (Seva, "Annadanam: feed people on a day of the month, however few", "Food given is the oldest Lakshmi remedy.", "monthly", "Friday", TEMPLE),
            (Vrata, "Vaibhava Lakshmi vrata on Fridays; Ekadashi fast", "The vrata for steady wealth.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Sri Suktam or Kanakadhara stotra on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi-Kubera pooja; lamp at the house door at dusk", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Health" => vec![
            (Karma, "Keep the appointment, take the medicine, finish the course", "The 1st is the body; the first remedy is the one the physician gave.", "as prescribed", "any day", DHARMA),
            (Discipline, "Sleep by ten, up before the Sun, for 40 days", "The Sun and the Moon are health; their rhythm is the body's.", "40 days", "any day", DHARMA),
            (Discipline, "A plain diet without stimulants for 21 days", "The 6th is disease; it feeds on excess.", "21 days", "any day", LAL),
            (Seva, "Give medicines or a day of care to someone ill and poor", "Health given returns as health.", "once", "any day", DHARMA),
            (Mantra, "Mahamrityunjaya mantra, 108 a day", "The classical mantra for health and long life.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Dhanvantari or Shiva abhisheka; Subrahmanya on Tuesdays for recovery", "Optional.", "not required", "Monday", TEMPLE),
        ],
        "Family" => vec![
            (Karma, "Eat one meal a day with your family, phones away", "The 2nd and 4th are kutumba and home; the shared meal is their rite.", "daily", "any day", DHARMA),
            (Karma, "Call or visit your mother; keep her comfort before your plans", "The 4th is the mother; its remedy is her ease.", "weekly", "Monday", DHARMA),
            (Seva, "Serve your elders and your in-laws without being asked", "Family given to returns as family.", "weekly", "any day", DHARMA),
            (Vrata, "Pitru tarpana on Amavasya; Somavara vrata", "The ancestors steady the house.", "monthly", "Monday", MUHURTA),
            (Mantra, "Shiva-Parvati or Gauri mantra on Mondays", "Optional support.", "if you wish", "Monday", TEMPLE),
            (Pooja, "A lamp at home at dusk, lit by the family together", "Optional, and the simplest.", "not required", "any day", TEMPLE),
        ],
        "Education" => vec![
            (Karma, "Study the hardest subject first, at the same hour daily", "The 4th and 5th are learning; the fixed hour is their discipline.", "daily", "Wednesday", DHARMA),
            (Seva, "Teach a junior what you have learned, free", "Knowledge given is knowledge fixed.", "weekly", "Wednesday", DHARMA),
            (Discipline, "Gayatri at sunrise, 11 times, for 40 days", "The classical discipline for the mind and the voice.", "40 days", "any day", TEMPLE),
            (Daana, "Give books or school fees for one child", "Jupiter and Mercury are education; their daana is the book.", "once", "Thursday", PURANA),
            (Mantra, "Saraswati or Hayagriva mantra on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Pooja, "Saraswati pooja before an exam; Dakshinamurthy on Thursdays", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        "Property" => vec![
            (Karma, "Repair what is broken in the house before buying anything new", "The 4th is the home as it is; its neglect blocks its growth.", "this month", "Tuesday", DHARMA),
            (Karma, "Clear the north-east corner of the home and keep it light", "Vastu's rule for the 4th house.", "once", "any day", TEMPLE),
            (Seva, "Help someone keep or find a roof: rent, a repair, a reference", "Home given returns as home.", "once", "any day", DHARMA),
            (Daana, "Give red lentils and a copper vessel on a Tuesday", "Mars is property; his daana.", "once", "Tuesday", PURANA),
            (Mantra, "Hanuman Chalisa on Tuesdays; Bhumi sukta before a deal", "Optional support.", "if you wish", "Tuesday", TEMPLE),
            (Pooja, "Vastu or Bhumi pooja before building; Ganesha before signing", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
        "Foreign" => vec![
            (Karma, "Set your papers in order: passport, documents, debts, before you plan the journey", "The 12th and 9th are distance; disorder at home follows you abroad.", "this month", "any day", DHARMA),
            (Seva, "Help a traveller or a stranger far from home", "Rahu and the 12th are the foreign; serving them opens the road.", "once", "Saturday", DHARMA),
            (Daana, "Give footwear or an umbrella to someone who walks far", "The traveller's daana.", "once", "Wednesday", TEMPLE),
            (Discipline, "No intoxicants and plain food for 21 days before a long journey", "Rahu rules the journey; keep him clear.", "21 days", "any day", LAL),
            (Mantra, "Hanuman or Ganesha before setting out; Durga for Rahu", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga pooja on a Saturday; a lamp before the journey", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        "Children" => vec![
            (Karma, "Keep every promise made to a child this month", "The 5th is children and faith kept with them.", "this month", "Thursday", DHARMA),
            (Seva, "Feed children, or pay one child's schooling", "The 5th grows when children are served.", "once", "Thursday", DHARMA),
            (Seva, "Serve a teacher or an old scholar", "Jupiter, the karaka of children, is the guru.", "weekly", "Thursday", DHARMA),
            (Vrata, "Putrada Ekadashi; Thursday fast for Jupiter", "The classical vrata for children.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Santana Gopala mantra; Guru beeja on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Santana Gopala or Krishna pooja; Alangudi for Jupiter", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        _ => Vec::new(),
    }
}

/// The graha's deity, day, metal, grain and colour, in one line.
pub fn signature(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Sun · Shiva and Surya · Sunday · copper · wheat · red and saffron",
        DashaLord::Moon => "Moon · Parvati · Monday · silver · rice · white",
        DashaLord::Mars => "Mars · Subrahmanya and Hanuman · Tuesday · copper · red lentil · red",
        DashaLord::Mercury => "Mercury · Vishnu · Wednesday · bronze · green moong · green",
        DashaLord::Jupiter => "Jupiter · Dakshinamurthy and Brahma · Thursday · gold · chana dal · yellow",
        DashaLord::Venus => "Venus · Lakshmi · Friday · silver · rice and curd · white",
        DashaLord::Saturn => "Saturn · Shani, Hanuman and Yama · Saturday · iron · black til · black and blue",
        DashaLord::Rahu => "Rahu · Durga · Saturday · lead · mustard and urad · smoke blue",
        DashaLord::Ketu => "Ketu · Ganesha · Tuesday · (no metal) · sesame · many colours",
    }
}

fn kind_base(kind: RemedyKind) -> f64 {
    match kind {
        RemedyKind::Karma => 0.95,
        RemedyKind::Seva => 0.85,
        RemedyKind::Discipline => 0.85,
        RemedyKind::Daana => 0.65,
        RemedyKind::Vrata => 0.55,
        RemedyKind::Mantra => 0.45,
        RemedyKind::Pooja => 0.3,
        RemedyKind::Anna => 0.82,
        RemedyKind::Jeeva => 0.78,
        RemedyKind::Prakriti => 0.72,
        RemedyKind::Vastra => 0.5,
    }
}

/// Build the remedy plan. `period_lords` are the running dasha lords, maha first; `area`
/// narrows the cause to the matter at hand and adds the matter's own remedies.
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
    let make = |rows: Vec<Row>, bonus: f64| -> Vec<Remedy> {
        rows.into_iter()
            .map(|(kind, title, why, effort, day, source)| {
                let fit = (kind_base(kind) * (0.5 + 0.5 * severity)
                    + bonus
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
                    day,
                    source,
                    optional: matches!(kind, RemedyKind::Mantra | RemedyKind::Pooja),
                }
            })
            .collect()
    };
    let mut remedies = make(table(planet), 0.0);
    // The other ways: food to the graha's people, its creatures fed, water and trees,
    // colour. They sit with the daana and seva of the main table.
    remedies.extend(make(crate::daana::alternatives(planet), -0.01));
    if let Some(a) = area {
        // The matter's own remedies sit just under the graha's: same kind, a shade less fit.
        remedies.extend(make(life_table(a.name), -0.03));
    }
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
        signature: signature(planet).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_graha_has_a_karma_first_dictionary() {
        for g in DashaLord::ORDER {
            let t = table(g);
            assert!(t.len() >= 14, "{g:?} has {}", t.len());
            assert_eq!(t[0].0, RemedyKind::Karma);
            for kind in [
                RemedyKind::Karma,
                RemedyKind::Seva,
                RemedyKind::Discipline,
                RemedyKind::Daana,
                RemedyKind::Vrata,
                RemedyKind::Mantra,
                RemedyKind::Pooja,
            ] {
                assert!(t.iter().any(|r| r.0 == kind), "{g:?} lacks {kind:?}");
            }
            assert!(!signature(g).is_empty());
        }
    }

    #[test]
    fn every_area_has_its_own_remedies() {
        for a in crate::areas::AREAS {
            let t = life_table(a.name);
            assert!(t.len() >= 6, "{} has {}", a.name, t.len());
            assert!(t.iter().any(|r| !matches!(r.0, RemedyKind::Mantra | RemedyKind::Pooja)));
        }
    }
}
