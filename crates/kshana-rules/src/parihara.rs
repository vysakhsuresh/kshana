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
/// The graha's own dictionary. Titles say what to do; the why says what it eases in
/// the person's life. No planet is named: the graha is the engine's, the life is theirs.
pub fn table(g: DashaLord) -> Vec<Row> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Karma, "Keep your word to those who depend on you", "Respect and standing are earned by reliability. Do what you said, when you said it.", "daily", "any day", DHARMA),
            (Karma, "Mend things with your father, or honour his memory", "A strained bond with your father blocks recognition; mending it opens doors with seniors too.", "this week", "Sunday", DHARMA),
            (Karma, "Take responsibility for one thing you have been blaming others for", "Confidence and standing grow with ownership and shrink with excuses.", "this week", "any day", DHARMA),
            (Seva, "Serve your father, or an elder man without family", "Eases friction with bosses, officials and anyone in authority over you.", "weekly", "Sunday", DHARMA),
            (Seva, "Help someone get a government paper or a right they are owed", "Clearing another's path with officials clears your own: approvals and paperwork move.", "once", "any day", DHARMA),
            (Discipline, "Be up before sunrise for 40 days", "Steadies energy, confidence and willpower; the sunrise time is computed for your place.", "40 days", "start Sunday", MUHURTA),
            (Discipline, "Twelve rounds of Suryanamaskar at dawn", "Done daily, it steadies heart, spine and will, and lifts low confidence.", "20 min · daily", "any day", TEMPLE),
            (Daana, "Give wheat or jaggery on a Sunday, by hand", "Eases trouble with authority, government and self-respect.", "once", "Sunday", PURANA),
            (Daana, "Give a copper vessel or red cloth to a man who works under the open sky", "Eases trouble with authority, father and reputation.", "once", "Sunday", PURANA),
            (Vrata, "Sunday fast: one meal, no salt, before sunset", "Kept for 12 Sundays, the classical fast for standing, health of the heart and the eyes.", "12 Sundays", "Sunday", BPHS),
            (Mantra, "Aditya Hridayam on Sundays", "Supports the discipline above; not a substitute for it.", "if you wish", "Sunday", TEMPLE),
            (Mantra, "Om hraam hreem hraum sah Suryaya namah · 7,000 in all", "The classical chant for confidence and recognition. A mala a day finishes it in two months.", "if you wish", "start Sunday", BPHS),
            (Pooja, "Arghya at sunrise: water offered from a copper vessel, facing east", "Optional. Listed so you know it is not hidden.", "not required", "any day", TEMPLE),
            (Pooja, "Shiva temple on a Sunday; Suryanar Kovil for the full rite", "Optional.", "not required", "Sunday", TEMPLE),
        ],
        DashaLord::Moon => vec![
            (Karma, "Care for your mother, in person, this week", "A restless mind and an unsettled home calm down when the bond with your mother is tended.", "weekly", "Monday", DHARMA),
            (Karma, "Say one true thing you have been holding in, gently", "Feelings held back build up until they flood; speaking early prevents the outburst.", "this week", "any day", DHARMA),
            (Karma, "Forgive one person in your own mind, without telling them", "Grudges weigh on sleep and mood; letting one go lightens all of them.", "once", "full moon", DHARMA),
            (Seva, "Serve women and children who have no one", "Calms worry and brings goodwill from people around you. An hour of help, not money alone.", "weekly", "Monday", DHARMA),
            (Seva, "Feed and comfort someone who is grieving", "Comforting another steadies your own mind.", "once", "any day", DHARMA),
            (Discipline, "Fixed sleep and waking times for 40 days", "A steady routine is the best medicine for mood swings and poor sleep.", "40 days", "start Monday", MUHURTA),
            (Discipline, "Ten minutes of still breathing each night, no screen after it", "A quiet close to the day settles an anxious mind.", "10 min · nightly", "any day", DHARMA),
            (Daana, "Give rice, milk or white cloth on a Monday", "Calms worry, mood swings and trouble at home.", "once", "Monday", PURANA),
            (Daana, "Give silver (even a small coin) or a conch to a temple or a woman elder", "Brings peace of mind and the blessing of the mother.", "once", "Monday", PURANA),
            (Vrata, "Somavara vrata: Mondays, one meal after sunset, for 16 Mondays", "Kept for Shiva, the classical fast for a shaken mind.", "16 Mondays", "Monday", MUHURTA),
            (Mantra, "Om shraam shreem shraum sah Chandraya namah · 11,000 in all", "The classical chant for a calm mind.", "if you wish", "start Monday", BPHS),
            (Mantra, "Shiva Panchakshari on Mondays", "Optional support for peace of mind.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Abhisheka with milk on a Monday", "Optional.", "not required", "Monday", TEMPLE),
            (Pooja, "Gauri or Parvati temple; Thingalur for the full rite", "Optional.", "not required", "Monday", TEMPLE),
        ],
        DashaLord::Mars => vec![
            (Karma, "End one quarrel on your own initiative", "Anger and conflict ease when courage is used for peace.", "this week", "Tuesday", DHARMA),
            (Karma, "Stand up, once, for someone weaker than you", "Turns restless energy into protection; quarrels around you drop.", "this week", "any day", DHARMA),
            (Karma, "Repay a brother or a sibling what you owe, in money or in time", "Debts between siblings feed disputes over money and property.", "this month", "Tuesday", DHARMA),
            (Seva, "Donate blood, or serve those who protect others", "Lowers the risk of injuries, accidents and fights.", "once", "any day", DHARMA),
            (Seva, "Cook and serve a hot meal to labourers or soldiers", "Cools heat and haste, and eases property and land troubles.", "once", "Tuesday", TEMPLE),
            (Discipline, "No anger spoken aloud for 40 days", "Restraint is the surest remedy for quarrels and their costs.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Hard physical work or sport, 30 minutes, until you sweat", "Energy not spent in effort comes out as temper.", "30 min · daily", "any day", DHARMA),
            (Daana, "Give red lentils on a Tuesday", "Cools anger, quarrels and accidents; helps property matters.", "once", "Tuesday", PURANA),
            (Daana, "Give red cloth, copper or red sweets to a young man who serves", "Eases disputes, debts and land troubles.", "once", "Tuesday", PURANA),
            (Vrata, "Tuesday fast with one meal of plain food; Mangala Gauri for women", "Kept 21 Tuesdays, for courage and for a calmer marriage.", "21 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Tuesdays", "Optional support for courage and protection.", "if you wish", "Tuesday", TEMPLE),
            (Mantra, "Om kraam kreem kraum sah Bhaumaya namah · 10,000 in all", "The classical chant for courage and calm.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Subrahmanya or Hanuman temple on a Tuesday", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Vaitheeswaran Koil for the full rite", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
        DashaLord::Mercury => vec![
            (Karma, "Speak less and only what is true for a week", "Misunderstandings, broken deals and gossip ease when speech is measured.", "7 days", "Wednesday", DHARMA),
            (Karma, "Settle one account you have let slide, to the rupee", "A loose ledger brings money mix-ups; a clean one ends them.", "this week", "Wednesday", DHARMA),
            (Karma, "Return a borrowed book, tool or favour", "Keeps exchanges even, so trade and talks flow.", "this week", "any day", LAL),
            (Seva, "Teach a child or an adult learner for an hour", "Sharpens your own studies and thinking.", "weekly", "Wednesday", DHARMA),
            (Seva, "Help a small trader or a student with paperwork or accounts", "Clears confusion in your own business and papers.", "once", "any day", DHARMA),
            (Discipline, "No gossip, no idle scrolling, for 40 days", "Scattered attention is the root of forgetfulness and nervous worry.", "40 days", "start Wednesday", DHARMA),
            (Discipline, "Write one page by hand every morning", "Writing daily sharpens memory, speech and focus.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give green moong on a Wednesday", "Clears confusion in talk, studies, trade and paperwork.", "once", "Wednesday", PURANA),
            (Daana, "Give green cloth, a bronze vessel or books to a student", "Helps exams, contracts and business.", "once", "Wednesday", PURANA),
            (Vrata, "Wednesday fast, one meal, with Vishnu in mind", "For clear thinking, studies and trade.", "7 Wednesdays", "Wednesday", MUHURTA),
            (Mantra, "Vishnu Sahasranama on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Mantra, "Om braam breem braum sah Budhaya namah · 9,000 in all", "The classical chant for intellect and speech.", "if you wish", "start Wednesday", BPHS),
            (Pooja, "Vishnu or Krishna temple on a Wednesday; tulasi water", "Optional.", "not required", "Wednesday", TEMPLE),
            (Pooja, "Thiruvenkadu for the full rite", "Optional.", "not required", "Wednesday", TEMPLE),
        ],
        DashaLord::Jupiter => vec![
            (Karma, "Honour a teacher you have neglected", "Gratitude to teachers brings back good advice and lucky breaks.", "this week", "Thursday", DHARMA),
            (Karma, "Keep one promise made to a child", "Faith kept with children brings their blessings and good news about them.", "this week", "any day", DHARMA),
            (Karma, "Give advice only when asked, and then fully", "Luck and respect shrink when wisdom is forced or withheld.", "40 days", "any day", DHARMA),
            (Seva, "Feed students, or fund one student's books", "Knowledge given freely returns as growth in money and status.", "once", "Thursday", DHARMA),
            (Seva, "Serve a priest, a teacher or an elder scholar without being asked", "Brings guidance at the moment you need it.", "weekly", "Thursday", TEMPLE),
            (Discipline, "Read scripture or a serious book daily for 40 days", "Steadies judgement, so money and family decisions go right.", "40 days", "start Thursday", DHARMA),
            (Discipline, "No lie, not even a small one, for 40 days", "Truthfulness is the fastest way to win back trust and luck.", "40 days", "any day", DHARMA),
            (Daana, "Give turmeric or chana dal on a Thursday", "Brings back luck, savings and the blessing of children.", "once", "Thursday", PURANA),
            (Daana, "Give yellow cloth, a book or (if you can) a little gold to a teacher", "For growth in money, marriage and wisdom.", "once", "Thursday", PURANA),
            (Vrata, "Thursday fast: yellow food once, no salt", "Kept 16 Thursdays, for marriage, children and wisdom.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Om graam greem graum sah Gurave namah · 19,000 in all", "The classical chant for wisdom and good fortune.", "if you wish", "start Thursday", BPHS),
            (Mantra, "Dakshinamurthy stotra on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Dakshinamurthy or Vishnu temple on a Thursday", "Optional.", "not required", "Thursday", TEMPLE),
            (Pooja, "Alangudi for the full rite", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        DashaLord::Venus => vec![
            (Karma, "Keep a promise made in a relationship", "Love and trust grow back when promises are kept.", "this week", "Friday", DHARMA),
            (Karma, "Say thank you, specifically, to the person who makes your home", "Naming the comfort others give brings warmth back into the home.", "this week", "Friday", DHARMA),
            (Karma, "Make one thing beautiful: a corner, a meal, a letter", "Beauty made, not bought, restores joy and attraction.", "this week", "any day", DHARMA),
            (Seva, "Serve women, artists or the sick with your skill", "Brings grace and ease into your own relationships.", "weekly", "Friday", DHARMA),
            (Seva, "Help a couple who are struggling, or a girl's education", "Harmony given to others returns to your own marriage.", "once", "Friday", DHARMA),
            (Discipline, "Simplify one indulgence for 40 days", "Overspending and indulgence drain love and money alike.", "40 days", "start Friday", DHARMA),
            (Discipline, "Clean clothes, clean room, clean words, for 40 days", "Small refinements bring back comfort and attraction.", "40 days", "any day", LAL),
            (Daana, "Give curd or white sweets on a Friday", "Restores love, comfort and harmony in marriage.", "once", "Friday", PURANA),
            (Daana, "Give white cloth, rice, ghee or silver to a woman in need", "For a happier marriage and comfort at home.", "once", "Friday", PURANA),
            (Vrata, "Friday fast with white food once; Vaibhava Lakshmi vrata", "For marriage, comfort and the arts.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Om draam dreem draum sah Shukraya namah · 16,000 in all", "The classical chant for love and comfort.", "if you wish", "start Friday", BPHS),
            (Mantra, "Sri Suktam or Lakshmi Ashtottara on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi or Devi temple on a Friday", "Optional.", "not required", "Friday", TEMPLE),
            (Pooja, "Kanjanur for the full rite", "Optional.", "not required", "Friday", TEMPLE),
        ],
        DashaLord::Saturn => vec![
            (Karma, "Finish the work you have postponed longest", "Delays lift when duty is completed rather than avoided.", "20 min · daily", "any day", DHARMA),
            (Karma, "Pay every worker and helper on time, in full, this month", "An unpaid wage is the surest way to invite delays and losses.", "this month", "Saturday", DHARMA),
            (Karma, "Visit someone old who is alone", "Eases loneliness and heaviness in your own life.", "weekly", "Saturday", DHARMA),
            (Seva, "Serve the old or the labouring on Saturdays", "Eases delays, debts and tiredness at work. An hour of real help.", "1 h · Saturdays", "Saturday", DHARMA),
            (Seva, "Clean a public place, a temple floor or a shared stair, yourself", "Humble work done without witness lightens a heavy period.", "once", "Saturday", TEMPLE),
            (Discipline, "Rise before sunrise for 40 days", "Discipline is the remedy for delay that needs no one else.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Walk instead of ride for one errand a day", "Builds the patience a slow period asks for.", "daily", "any day", LAL),
            (Daana, "Give black til or a blanket on a Saturday", "Eases delays, debts and heavy duty; give by hand to a person who needs it.", "once", "Saturday", PURANA),
            (Daana, "Give iron (a tool, a pan) or mustard oil to a labourer", "Eases job troubles, chronic aches and slow progress.", "once", "Saturday", PURANA),
            (Daana, "Feed crows and a black dog on Saturdays", "Eases delays, debts and tiredness at work.", "Saturdays", "Saturday", TEMPLE),
            (Vrata, "Saturday fast, one meal after sunset, for 19 Saturdays", "The classical fast through a long testing period.", "19 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays", "Optional. Supports the discipline above.", "if you wish", "Saturday", TEMPLE),
            (Mantra, "Om praam preem praum sah Shanaischaraya namah · 23,000 in all", "The classical chant for patience and protection in hard times.", "if you wish", "start Saturday", BPHS),
            (Pooja, "Til-oil lamp to Hanuman on a Saturday", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Thirunallar for the full rite", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Rahu => vec![
            (Karma, "Stop one shortcut or deception you have been running", "Shortcuts are where sudden trouble comes from; dropping one closes the door on it.", "this week", "any day", DHARMA),
            (Karma, "Tell the one person you have been avoiding the plain truth", "Clears the fog and suspicion between people.", "this week", "Saturday", DHARMA),
            (Karma, "Leave one obsession alone for a week: the feed, the rival, the fear", "Anxiety feeds on appetite without end; starving it calms the mind.", "7 days", "any day", DHARMA),
            (Seva, "Serve the outcast, the sick without family, the foreigner", "Eases confusion, fear and trouble from strangers.", "weekly", "Saturday", DHARMA),
            (Seva, "Help a migrant worker or a stranger far from home", "Opens foreign and unusual opportunities, safely.", "once", "any day", DHARMA),
            (Discipline, "No intoxicants for 40 days", "Confusion and bad decisions feed on intoxication.", "40 days", "start Saturday", DHARMA),
            (Discipline, "Eat only what you have cooked or seen cooked, for 21 days", "Guards against hidden harm and sudden illness.", "21 days", "any day", LAL),
            (Daana, "Give mustard oil or blue cloth on a Saturday", "Clears confusion, anxiety and sudden setbacks.", "once", "Saturday", PURANA),
            (Daana, "Give a coconut or a dark blanket; let a coconut float in running water", "Eases fear, cheating and sudden trouble.", "once", "Saturday", LAL),
            (Vrata, "Saturday fast with Durga in mind, 18 Saturdays", "For protection from fear and sudden trouble.", "18 Saturdays", "Saturday", MUHURTA),
            (Mantra, "Om bhraam bhreem bhraum sah Rahave namah · 18,000 in all", "The classical chant against fear and confusion.", "if you wish", "start Saturday", BPHS),
            (Mantra, "Durga Saptashati or Durga mantra on Saturdays", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga temple on a Saturday evening", "Optional.", "not required", "Saturday", TEMPLE),
            (Pooja, "Thirunageswaram for the full rite", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        DashaLord::Ketu => vec![
            (Karma, "Finish something you abandoned", "Drift and loss of interest end when one thing is completed.", "this week", "any day", DHARMA),
            (Karma, "Apologise for one thing you walked away from", "Mends the silent breaks that leave you feeling cut off.", "this week", "Tuesday", DHARMA),
            (Karma, "Give away ten things you do not use", "Letting go on purpose prevents losses that come uninvited.", "this week", "any day", DHARMA),
            (Seva, "Feed and care for stray dogs, or serve a renunciate", "Eases sudden losses and a sense of emptiness.", "weekly", "Tuesday", DHARMA),
            (Seva, "Sit with the dying or the very ill, in silence", "Brings clarity and calm in a confusing time.", "once", "any day", DHARMA),
            (Discipline, "Live simply for 40 days: one meal plain, one hour of silence", "Simplicity turns restlessness into insight.", "40 days", "start Tuesday", DHARMA),
            (Discipline, "Ten minutes of meditation on the breath, daily", "Gives direction when you feel lost.", "10 min · daily", "any day", DHARMA),
            (Daana, "Give a blanket or sesame", "Eases losses, drift and lack of direction.", "once", "Tuesday", PURANA),
            (Daana, "Give a multicoloured cloth or a lamp to a temple of Ganesha", "Clears obstacles and sudden breaks.", "once", "Tuesday", TEMPLE),
            (Vrata, "Tuesday fast with Ganesha in mind; Sankashti Chaturthi", "For the removal of obstacles.", "7 Tuesdays", "Tuesday", MUHURTA),
            (Mantra, "Ganesha mantra · Om gam Ganapataye namah", "Optional support.", "if you wish", "any day", TEMPLE),
            (Mantra, "Om sraam sreem sraum sah Ketave namah · 17,000 in all", "The classical chant for clarity and release.", "if you wish", "start Tuesday", BPHS),
            (Pooja, "Ganesha temple on a Tuesday or Chaturthi", "Optional.", "not required", "Tuesday", TEMPLE),
            (Pooja, "Keezhperumpallam for the full rite", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
    }
}

/// Remedies for a life area, added when the matter is known.
pub fn life_table(area_name: &str) -> Vec<Row> {
    use RemedyKind::*;
    match area_name {
        "Marriage" => vec![
            (Karma, "Treat your partner's family as your own this month", "Marriages are strained most often through the in-laws; this is where it mends.", "this month", "any day", DHARMA),
            (Karma, "Settle an old hurt with a former partner in your own mind", "A bond left bleeding weakens the next one.", "once", "Friday", DHARMA),
            (Seva, "Help with a wedding you are not part of; give to a girl's marriage", "Marriage given to another returns.", "once", "Friday", TEMPLE),
            (Vrata, "Swayamvara Parvathi or Mangala Gauri vrata on Tuesdays or Fridays", "The classical fast for marriage and its harmony.", "16 weeks", "Tuesday", MUHURTA),
            (Mantra, "Swayamvara Parvathi mantra, or Katyayani for women, Shiva for men", "Optional support, from the marriage tradition.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Umamaheshwara pooja; a visit to a Parvati or Subrahmanya temple as a couple", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Career" => vec![
            (Karma, "Do the part of your work you have been avoiding, first thing", "The avoided task is the blocked promotion.", "daily", "any day", DHARMA),
            (Karma, "Give your superior the respect you expect from those under you", "Authority flows both ways; respect given upward is returned.", "this week", "Sunday", DHARMA),
            (Seva, "Mentor someone junior without credit", "Lifting others at work raises your own standing.", "weekly", "any day", DHARMA),
            (Discipline, "Twelve rounds of Suryanamaskar before work", "Meet the day with energy before you meet your work.", "15 min · daily", "Sunday", TEMPLE),
            (Vrata, "Sunday fast for recognition, or Saturday fast for stability at work", "Choose by what you need: to be seen, or to be secure.", "12 weeks", "Sunday", MUHURTA),
            (Mantra, "Hanuman Chalisa on Saturdays; Aditya Hridayam on Sundays", "Optional support.", "if you wish", "Saturday", TEMPLE),
        ],
        "Wealth" => vec![
            (Karma, "Write down every rupee in and out for 40 days", "Money leaks where it is not watched.", "5 min · daily", "any day", DHARMA),
            (Karma, "Repay the oldest small debt, and lend nothing for 40 days", "Gains come to one who does not scatter.", "40 days", "any day", LAL),
            (Seva, "Give one tenth of a windfall away before you spend it", "Wealth shared at once is wealth that stays.", "when it comes", "any day", DHARMA),
            (Seva, "Annadanam: feed people on a day of the month, however few", "Food given is the oldest remedy for money.", "monthly", "Friday", TEMPLE),
            (Vrata, "Vaibhava Lakshmi vrata on Fridays; Ekadashi fast", "The fast for steady wealth.", "11 Fridays", "Friday", MUHURTA),
            (Mantra, "Sri Suktam or Kanakadhara stotra on Fridays", "Optional support.", "if you wish", "Friday", TEMPLE),
            (Pooja, "Lakshmi-Kubera pooja; a lamp at the front door at dusk", "Optional.", "not required", "Friday", TEMPLE),
        ],
        "Health" => vec![
            (Karma, "Keep the appointment, take the medicine, finish the course", "The first remedy is the one the doctor gave.", "as prescribed", "any day", DHARMA),
            (Discipline, "Sleep by ten, up before sunrise, for 40 days", "The body heals on a steady rhythm.", "40 days", "any day", DHARMA),
            (Discipline, "A plain diet without stimulants for 21 days", "Illness feeds on excess.", "21 days", "any day", LAL),
            (Seva, "Give medicines or a day of care to someone ill and poor", "Health given returns as health.", "once", "any day", DHARMA),
            (Mantra, "Mahamrityunjaya mantra, 108 a day", "The classical mantra for health and long life.", "if you wish", "Monday", TEMPLE),
            (Pooja, "Dhanvantari or Shiva abhisheka; Subrahmanya on Tuesdays for recovery", "Optional.", "not required", "Monday", TEMPLE),
        ],
        "Family" => vec![
            (Karma, "Eat one meal a day with your family, phones away", "The shared meal is what holds a family together.", "daily", "any day", DHARMA),
            (Karma, "Call or visit your mother; keep her comfort before your plans", "Peace at home begins with the mother's ease.", "weekly", "Monday", DHARMA),
            (Seva, "Serve your elders and your in-laws without being asked", "Family given to returns as family.", "weekly", "any day", DHARMA),
            (Vrata, "Pitru tarpana on Amavasya; Somavara vrata", "The ancestors steady the home.", "monthly", "Monday", MUHURTA),
            (Mantra, "Shiva-Parvati or Gauri mantra on Mondays", "Optional support.", "if you wish", "Monday", TEMPLE),
            (Pooja, "A lamp at home at dusk, lit by the family together", "Optional, and the simplest.", "not required", "any day", TEMPLE),
        ],
        "Education" => vec![
            (Karma, "Study the hardest subject first, at the same hour daily", "A fixed hour is the discipline that wins exams.", "daily", "Wednesday", DHARMA),
            (Seva, "Teach a junior what you have learned, free", "Knowledge given is knowledge fixed.", "weekly", "Wednesday", DHARMA),
            (Discipline, "Gayatri at sunrise, 11 times, for 40 days", "The classical discipline for the mind and the voice.", "40 days", "any day", TEMPLE),
            (Daana, "Give books or school fees for one child", "Learning given opens your own.", "once", "Thursday", PURANA),
            (Mantra, "Saraswati or Hayagriva mantra on Wednesdays", "Optional support.", "if you wish", "Wednesday", TEMPLE),
            (Pooja, "Saraswati pooja before an exam; Dakshinamurthy on Thursdays", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        "Property" => vec![
            (Karma, "Repair what is broken at home before buying anything new", "A neglected home blocks a new one.", "this month", "Tuesday", DHARMA),
            (Karma, "Clear the north-east corner of the home and keep it light", "Vastu's first rule for a settled home.", "once", "any day", TEMPLE),
            (Seva, "Help someone keep or find a roof: rent, a repair, a reference", "Home given returns as home.", "once", "any day", DHARMA),
            (Daana, "Give red lentils and a copper vessel on a Tuesday", "The classical gift for land and property matters.", "once", "Tuesday", PURANA),
            (Mantra, "Hanuman Chalisa on Tuesdays; Bhumi sukta before a deal", "Optional support.", "if you wish", "Tuesday", TEMPLE),
            (Pooja, "Vastu or Bhumi pooja before building; Ganesha before signing", "Optional.", "not required", "Tuesday", TEMPLE),
        ],
        "Foreign" => vec![
            (Karma, "Set your papers in order: passport, documents, debts, before you plan the journey", "Disorder at home follows you abroad.", "this month", "any day", DHARMA),
            (Seva, "Help a traveller or a stranger far from home", "Serving travellers opens the road for you.", "once", "Saturday", DHARMA),
            (Daana, "Give footwear or an umbrella to someone who walks far", "The traveller's gift.", "once", "Wednesday", TEMPLE),
            (Discipline, "No intoxicants and plain food for 21 days before a long journey", "Keeps the journey clear of confusion and mishap.", "21 days", "any day", LAL),
            (Mantra, "Hanuman or Ganesha before setting out; Durga for protection", "Optional support.", "if you wish", "Saturday", TEMPLE),
            (Pooja, "Durga pooja on a Saturday; a lamp before the journey", "Optional.", "not required", "Saturday", TEMPLE),
        ],
        "Children" => vec![
            (Karma, "Keep every promise made to a child this month", "Faith kept with children brings their blessings.", "this month", "Thursday", DHARMA),
            (Seva, "Feed children, or pay one child's schooling", "Children served bring children's joy.", "once", "Thursday", DHARMA),
            (Seva, "Serve a teacher or an old scholar", "The teacher's blessing is the classical blessing for children.", "weekly", "Thursday", DHARMA),
            (Vrata, "Putrada Ekadashi; Thursday fast", "The classical fast for children.", "16 Thursdays", "Thursday", MUHURTA),
            (Mantra, "Santana Gopala mantra on Thursdays", "Optional support.", "if you wish", "Thursday", TEMPLE),
            (Pooja, "Santana Gopala or Krishna pooja; Alangudi for the full rite", "Optional.", "not required", "Thursday", TEMPLE),
        ],
        _ => Vec::new(),
    }
}

/// The remedy's deity, day, metal, grain and colour, in one line. No planet named.
pub fn signature(g: DashaLord) -> &'static str {
    match g {
        DashaLord::Sun => "Shiva and Aditya · Sunday · copper · wheat · red and saffron",
        DashaLord::Moon => "Parvati · Monday · silver · rice · white",
        DashaLord::Mars => "Subrahmanya and Hanuman · Tuesday · copper · red lentil · red",
        DashaLord::Mercury => "Vishnu · Wednesday · bronze · green moong · green",
        DashaLord::Jupiter => "Dakshinamurthy · Thursday · gold · chana dal · yellow",
        DashaLord::Venus => "Lakshmi · Friday · silver · rice and curd · white",
        DashaLord::Saturn => "Hanuman · Saturday · iron · black til · black and blue",
        DashaLord::Rahu => "Durga · Saturday · lead · mustard and urad · smoke blue",
        DashaLord::Ketu => "Ganesha · Tuesday · sesame · many colours",
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
    let matter = area.map(|a| a.name.to_ascii_lowercase());
    for (i, &l) in period_lords.iter().enumerate() {
        let (q, _why) = match area {
            Some(a) => period_quality(facts, a, l),
            None => {
                let p = facts.planet(l);
                let d = dignity(l, p.sidereal);
                (d.score(), Vec::new())
            }
        };
        candidates.push((
            l,
            1.0 - q,
            format!(
                "The {} you are in brings {}{}",
                crate::plain::level_noun(i),
                crate::plain::pressure(l),
                matter.as_ref().map(|m| format!(" to your {m}")).unwrap_or_default()
            ),
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
                "From birth your {} is open to {}",
                a.name.to_ascii_lowercase(),
                crate::plain::pressure(lord)
            ),
        ));
    }
    let (planet, severity, cause) = candidates
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((DashaLord::Saturn, 0.0, "Nothing particular weighs on you".into()));
    let cause = if severity < 0.25 {
        format!("{cause}, but only lightly. Nothing serious is against you: your own effort is the remedy, and the list below is extra help.")
    } else {
        format!("{cause}. The remedies below are chosen to ease exactly that.")
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
    fn every_remedy_speaks_plainly() {
        use crate::plain::jargon;
        let mut rows: Vec<Row> = Vec::new();
        for g in DashaLord::ORDER {
            rows.extend(table(g));
            rows.extend(crate::daana::alternatives(g));
            assert_eq!(jargon(signature(g)), None, "{}", signature(g));
        }
        for a in crate::areas::AREAS {
            rows.extend(life_table(a.name));
        }
        for r in &rows {
            assert_eq!(jargon(r.1), None, "title: {}", r.1);
            assert_eq!(jargon(r.2), None, "why: {}", r.2);
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
