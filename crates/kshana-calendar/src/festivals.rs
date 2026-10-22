//! The festival calendar, derived from rules rather than a list of dates.
//!
//! A lunar festival is a tithi of a (non-adhika) amanta month plus a time-of-day
//! condition (kala). When the tithi touches two civil days, the day is the one on
//! which the tithi covers the kala; the kala windows divide the day and night as the
//! *Dharmasindhu* and *Nirnaya Sindhu* do, and as Drik Panchang states them:
//!
//! * sunrise (udaya): the tithi prevailing at sunrise; a tithi that holds no sunrise
//!   (kshaya) is kept on the day in which it runs;
//! * purvahna: the first half of the daytime;
//! * madhyahna and aparahna: the third and fourth fifths of the daytime (the fivefold
//!   division: pratah, sangava, madhyahna, aparahna, sayahna, three muhurtas each);
//! * pradosha: three night-muhurtas (one fifth of the night) from sunset;
//! * nishita: the eighth of the fifteen night-muhurtas, about local midnight;
//! * moonrise: the tithi prevailing at moonrise;
//! * arunodaya: four ghatis (96 minutes) before sunrise.
//!
//! When the tithi covers the window on both days, the day with the larger share wins
//! and equal shares go to the first day, except where a text names the second (Diwali:
//! "dinadvaye pradoshavyaptau para", Nirnaya Sindhu II). Festivals are not held in an
//! adhika month (Nirnaya Sindhu I, malamasa); the monthly observances are.
//!
//! Solar festivals follow the regional solar-month rules in `solar`; Onam is
//! Thiruvonam (Shravana nakshatra) in Chingam.

use crate::astro::Span;
use crate::lunar::LunarMonth;
use crate::names::{ekadashi_name, LUNAR_MONTHS};
use crate::solar::SolarRule;
use crate::{Calendar, CalendarError, CivilDay, Date};
use kshana_time::{Instant, NS_PER_SEC};

/// One ghati: 24 minutes.
const GHATI_NS: i128 = 24 * 60 * NS_PER_SEC;

/// A festival or observance on a civil day.
#[derive(Clone, Debug, PartialEq)]
pub struct Festival {
    /// Stable key, e.g. "diwali", "ekadashi".
    pub key: &'static str,
    /// Display name, e.g. "Devshayani Ekadashi".
    pub name: String,
    /// The civil date it is kept on (local date of the day's sunrise).
    pub date: Date,
    /// That day's sunrise.
    pub day_start: Instant,
    /// The next sunrise.
    pub day_end: Instant,
    /// When the governing tithi, star or sankranti begins.
    pub begins: Instant,
    /// When the governing tithi or star ends.
    pub ends: Option<Instant>,
    /// "tithi", "sun" or "star".
    pub kind: &'static str,
    /// What to do, in plain words.
    pub what: &'static str,
    /// The rule that fixed the day (expert text).
    pub rule: String,
}

/// Time-of-day conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kala {
    /// At sunrise.
    Sunrise,
    /// First half of the daytime.
    Purvahna,
    /// Third fifth of the daytime.
    Madhyahna,
    /// Fourth fifth of the daytime.
    Aparahna,
    /// One fifth of the night from sunset.
    Pradosha,
    /// The eighth night-muhurta.
    Nishita,
    /// At moonrise.
    Moonrise,
}

impl Kala {
    fn label(self) -> &'static str {
        match self {
            Kala::Sunrise => "at sunrise (udaya)",
            Kala::Purvahna => "in the forenoon (purvahna)",
            Kala::Madhyahna => "at midday (madhyahna)",
            Kala::Aparahna => "in the afternoon (aparahna)",
            Kala::Pradosha => "at dusk (pradosha)",
            Kala::Nishita => "at midnight (nishita)",
            Kala::Moonrise => "at moonrise",
        }
    }
}

/// The window `[a, b]` of a kala on a civil day (`None` for sunrise and moonrise,
/// which are instants).
pub fn window(day: &CivilDay, k: Kala) -> Option<(Instant, Instant)> {
    let (d, n) = (day.day_ns(), day.night_ns());
    let (r, s) = (day.sunrise, day.sunset);
    match k {
        Kala::Purvahna => Some((r, r.add_ns(d / 2))),
        Kala::Madhyahna => Some((r.add_ns(d * 2 / 5), r.add_ns(d * 3 / 5))),
        Kala::Aparahna => Some((r.add_ns(d * 3 / 5), r.add_ns(d * 4 / 5))),
        Kala::Pradosha => Some((s, s.add_ns(n / 5))),
        Kala::Nishita => Some((s.add_ns(n * 7 / 15), s.add_ns(n * 8 / 15))),
        Kala::Sunrise | Kala::Moonrise => None,
    }
}

struct LunarRule {
    key: &'static str,
    name: &'static str,
    /// Amanta month, 1 = Chaitra.
    month: u8,
    /// 1–30.
    tithi: u8,
    kala: Kala,
    later_on_tie: bool,
    what: &'static str,
}

/// The yearly lunar festivals. Months are amanta; the purnimanta name of a dark-half
/// festival is one month later (Janmashtami is "Bhadrapada Krishna Ashtami" in the
/// North, "Shravana Krishna Ashtami" in amanta almanacs).
const YEARLY: &[LunarRule] = &[
    LunarRule { key: "ugadi", name: "Ugadi / Gudi Padwa", month: 1, tithi: 1, kala: Kala::Sunrise, later_on_tie: false,
        what: "The lunar new year: a fresh start, a bitter-sweet dish for the year's mixed days, and the year's almanac read aloud." },
    LunarRule { key: "chaitra_navaratri", name: "Chaitra Navaratri begins", month: 1, tithi: 1, kala: Kala::Sunrise, later_on_tie: false,
        what: "Nine nights of the Goddess in spring: a lamp each evening and a lighter diet." },
    LunarRule { key: "rama_navami", name: "Rama Navami", month: 1, tithi: 9, kala: Kala::Madhyahna, later_on_tie: false,
        what: "Rama's birthday, kept at midday: a fast till noon, the story read, sweet water and fruit shared." },
    LunarRule { key: "akshaya_tritiya", name: "Akshaya Tritiya", month: 2, tithi: 3, kala: Kala::Purvahna, later_on_tie: false,
        what: "What is begun or given today is said never to run out: a good day to start, buy gold or give food." },
    LunarRule { key: "guru_purnima", name: "Guru Purnima", month: 4, tithi: 15, kala: Kala::Sunrise, later_on_tie: false,
        what: "The teachers' day: thank and honour those who taught you." },
    LunarRule { key: "raksha_bandhan", name: "Raksha Bandhan", month: 5, tithi: 15, kala: Kala::Aparahna, later_on_tie: false,
        what: "Sisters tie a protective thread on brothers' wrists; brothers promise care in return." },
    LunarRule { key: "janmashtami", name: "Krishna Janmashtami", month: 5, tithi: 23, kala: Kala::Nishita, later_on_tie: false,
        what: "Krishna's birth at midnight: a fast through the day, songs, and the fast broken after midnight." },
    LunarRule { key: "ganesh_chaturthi", name: "Ganesh Chaturthi", month: 6, tithi: 4, kala: Kala::Madhyahna, later_on_tie: false,
        what: "Ganesha is welcomed home at midday; begin the new and clear what blocks you." },
    LunarRule { key: "navaratri", name: "Navaratri begins", month: 7, tithi: 1, kala: Kala::Sunrise, later_on_tie: false,
        what: "Nine nights of the Goddess: the pot is set up in the morning and a lamp kept each evening." },
    LunarRule { key: "dussehra", name: "Dussehra / Vijayadashami", month: 7, tithi: 10, kala: Kala::Aparahna, later_on_tie: false,
        what: "Victory of good over evil: begin learning, honour your tools and books, and make peace." },
    LunarRule { key: "karva_chauth", name: "Karva Chauth", month: 7, tithi: 19, kala: Kala::Moonrise, later_on_tie: false,
        what: "Wives fast from dawn for their husbands and break the fast at moonrise." },
    LunarRule { key: "diwali", name: "Diwali (Lakshmi Puja)", month: 7, tithi: 30, kala: Kala::Pradosha, later_on_tie: true,
        what: "The festival of lights: lamps at dusk, Lakshmi welcomed, the home cleaned and accounts begun afresh." },
    LunarRule { key: "vasant_panchami", name: "Vasant Panchami", month: 11, tithi: 5, kala: Kala::Purvahna, later_on_tie: false,
        what: "Saraswati's day at the start of spring: children begin letters, books and instruments are honoured." },
    LunarRule { key: "maha_shivaratri", name: "Maha Shivaratri", month: 11, tithi: 29, kala: Kala::Nishita, later_on_tie: false,
        what: "The great night of Shiva: a fast, a lamp and a night's vigil, at least till midnight." },
    LunarRule { key: "holika_dahan", name: "Holika Dahan", month: 12, tithi: 15, kala: Kala::Pradosha, later_on_tie: false,
        what: "The bonfire at dusk on the eve of Holi: old grudges and the year's waste go into the fire." },
];

/// Keys of the observances that recur every month (as opposed to the yearly
/// festivals and the solar new years).
pub const MONTHLY_KEYS: &[&str] = &[
    "ekadashi",
    "ekadashi_vaishnava",
    "pradosham",
    "sankashti",
    "masik_shivaratri",
    "purnima",
    "amavasya",
    "vinayaka_chaturthi",
];

/// The rule text of a lunar festival.
fn rule_text(month: Option<u8>, tithi: u8, kala: Kala) -> String {
    let t = crate::names::tithi_full_name(tithi);
    match month {
        Some(m) => format!("{} {t} (amanta) {}", LUNAR_MONTHS[(m - 1) as usize], kala.label()),
        None => format!("{t} {}", kala.label()),
    }
}

/// Days (indices into `cal.days`) that the span touches.
fn touched(cal: &Calendar, span: &Span) -> Vec<usize> {
    (0..cal.days.len())
        .filter(|&i| span.overlap(cal.days[i].sunrise, cal.days[i].next_sunrise) > 0)
        .collect()
}

/// The sunrise (udaya) rule: the first day whose sunrise lies in the span; a span
/// that holds no sunrise is kept on the day it runs in.
fn by_sunrise(cal: &Calendar, span: &Span) -> Option<usize> {
    let t = touched(cal, span);
    t.iter()
        .copied()
        .find(|&i| span.contains(cal.days[i].sunrise))
        .or_else(|| t.iter().copied().find(|&i| span.start >= cal.days[i].sunrise && span.end <= cal.days[i].next_sunrise))
}

fn by_kala(cal: &Calendar, span: &Span, kala: Kala, later_on_tie: bool) -> Result<Option<usize>, CalendarError> {
    let t = touched(cal, span);
    let mut best: Option<(usize, i128)> = None;
    match kala {
        Kala::Sunrise => return Ok(by_sunrise(cal, span)),
        Kala::Moonrise => {
            for &i in &t {
                if let Some(m) = cal.moonrise(&cal.days[i])? {
                    if span.contains(m) {
                        return Ok(Some(i));
                    }
                }
            }
            return Ok(by_sunrise(cal, span));
        }
        _ => {
            for &i in &t {
                let (a, b) = window(&cal.days[i], kala).expect("a window");
                let o = span.overlap(a, b);
                if o == 0 {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((_, bo)) => o > bo || (o == bo && later_on_tie),
                };
                if better {
                    best = Some((i, o));
                }
            }
        }
    }
    Ok(best.map(|b| b.0).or_else(|| by_sunrise(cal, span)))
}

/// The tithi spans with `index`, each with the amanta month it lies in.
fn spans_of(cal: &Calendar, index: u8) -> Vec<(Span, LunarMonth)> {
    cal.tithis
        .iter()
        .filter(|s| s.index == index)
        .filter_map(|s| cal.month_at(s.start.add_ns(s.end.ns_since(s.start) / 2)).map(|m| (*s, m)))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn push(
    out: &mut Vec<Festival>,
    cal: &Calendar,
    i: usize,
    key: &'static str,
    name: String,
    begins: Instant,
    ends: Option<Instant>,
    kind: &'static str,
    what: &'static str,
    rule: String,
) {
    let d = &cal.days[i];
    out.push(Festival { key, name, date: d.date, day_start: d.sunrise, day_end: d.next_sunrise, begins, ends, kind, what, rule });
}

/// Every festival and observance on the requested days of `cal`, in date order.
pub fn festivals(cal: &Calendar) -> Result<Vec<Festival>, CalendarError> {
    let mut out = Vec::new();
    // Yearly lunar festivals.
    for r in YEARLY {
        for (span, m) in spans_of(cal, r.tithi) {
            if m.adhika || m.number != r.month {
                continue;
            }
            let rule = rule_text(Some(r.month), r.tithi, r.kala);
            if r.key == "raksha_bandhan" {
                // Nirnaya Sindhu: Shravana Purnima that holds sunrise for at least six
                // ghatis is kept that day (the afternoon of the eve is usually under
                // Bhadra, the Vishti half-tithi that opens a Purnima); otherwise the
                // afternoon rule.
                let day = by_sunrise(cal, &span).filter(|&i| {
                    let r0 = cal.days[i].sunrise;
                    span.contains(r0) && span.end.ns_since(r0) >= 6 * GHATI_NS
                });
                let day = match day {
                    Some(d) => Some(d),
                    None => by_kala(cal, &span, r.kala, false)?,
                };
                if let Some(i) = day {
                    push(&mut out, cal, i, r.key, r.name.into(), span.start, Some(span.end), "tithi", r.what,
                        format!("{rule}; Shravana Purnima at sunrise for six ghatis takes precedence (Nirnaya Sindhu)"));
                }
                continue;
            }
            if let Some(i) = by_kala(cal, &span, r.kala, r.later_on_tie)? {
                push(&mut out, cal, i, r.key, r.name.into(), span.start, Some(span.end), "tithi", r.what, rule.clone());
                if r.key == "holika_dahan" && i + 1 < cal.days.len() {
                    push(&mut out, cal, i + 1, "holi", "Holi".into(), span.start, Some(span.end), "tithi",
                        "The festival of colours, the morning after the bonfire: play, forgive and visit.",
                        "The day after Holika Dahan (Chaitra Krishna Pratipada, purnimanta)".into());
                }
            }
        }
    }
    // Monthly observances.
    for (paksha_tithi, shukla) in [(11u8, true), (26u8, false)] {
        for (span, m) in spans_of(cal, paksha_tithi) {
            let Some(smarta) = by_sunrise(cal, &span) else { continue };
            let name = format!("{} Ekadashi", ekadashi_name(m.number, shukla, m.adhika));
            push(&mut out, cal, smarta, "ekadashi", name.clone(), span.start, Some(span.end), "tithi",
                "A fast for Vishnu: no grain, the mind kept light, the fast broken the next morning.",
                format!("{} at sunrise (Smarta)", crate::names::tithi_full_name(paksha_tithi)));
            // Vaishnava rule (Hari Bhakti Vilasa 12–13): an Ekadashi touched by
            // Dashami at arunodaya, 4 ghatis before sunrise, is "viddha" and the
            // fast moves to the next day; so does one that holds two sunrises.
            let d = &cal.days[smarta];
            let aruna = d.sunrise.add_ns(-4 * GHATI_NS);
            let viddha = !span.contains(aruna);
            let two = cal.days.get(smarta + 1).is_some_and(|n| span.contains(n.sunrise));
            if (viddha || two) && smarta + 1 < cal.days.len() {
                push(&mut out, cal, smarta + 1, "ekadashi_vaishnava", format!("{name} (Vaishnava)"), span.start, Some(span.end), "tithi",
                    "The same fast as kept by Vaishnava families, one day later this time.",
                    "Ekadashi free of Dashami at arunodaya, 4 ghatis before sunrise (Hari Bhakti Vilasa)".into());
            }
        }
    }
    let monthly: [(&str, &str, u8, Kala, &str); 7] = [
        ("pradosham", "Pradosham", 13, Kala::Pradosha, "The dusk hours are Shiva's: light a lamp and make an apology; it eases delays."),
        ("pradosham", "Pradosham", 28, Kala::Pradosha, "The dusk hours are Shiva's: light a lamp; it eases delays."),
        ("sankashti", "Sankashti Chaturthi", 19, Kala::Moonrise, "Ganesha's fast, broken after moonrise: finish what was left undone; obstacles ease."),
        ("masik_shivaratri", "Masik Shivaratri", 29, Kala::Nishita, "The month's night of Shiva: a lamp kept through the night, or at least till midnight."),
        ("purnima", "Purnima", 15, Kala::Sunrise, "Full moon: honour your mother, feed women and children, sit by water."),
        ("amavasya", "Amavasya", 30, Kala::Sunrise, "The ancestors' day: give food in their name and begin nothing new."),
        ("vinayaka_chaturthi", "Vinayaka Chaturthi", 4, Kala::Madhyahna, "Ganesha's day of the bright fortnight, kept at midday."),
    ];
    for (key, name, tithi, kala, what) in monthly {
        for (span, m) in spans_of(cal, tithi) {
            // In Magha the monthly Shivaratri is the Maha Shivaratri.
            if key == "masik_shivaratri" && m.number == 11 && !m.adhika {
                continue;
            }
            if let Some(i) = by_kala(cal, &span, kala, false)? {
                push(&mut out, cal, i, key, name.into(), span.start, Some(span.end), "tithi", what, rule_text(None, tithi, kala));
            }
        }
    }
    // Solar festivals: (key, name, sign entered, regional rule, what, rule).
    let solar: [(&str, &str, u8, SolarRule, &str, &str); 6] = [
        ("makar_sankranti", "Makar Sankranti", 9, SolarRule::Tamil,
            "The turn towards longer days: til and jaggery shared, a bath at dawn, gifts to those in need.",
            "Sun's entry into sidereal Makara; a night ingress is kept the next day (Dharmasindhu, punya kala)"),
        ("pongal", "Pongal", 9, SolarRule::Tamil, "The harvest festival: rice boiled over in a new pot, thanks given to the land and cattle.",
            "Thai 1, the Tamil rule: Makara sankranti before sunset → same day"),
        ("puthandu", "Puthandu (Tamil New Year)", 0, SolarRule::Tamil, "The Tamil new year: a tray of fruit, flowers and a mirror seen first in the morning.",
            "Chithirai 1, the Tamil rule: Mesha sankranti before sunset → same day"),
        ("vishu", "Vishu", 0, SolarRule::Kerala, "Kerala's new year: the Vishukkani seen first at dawn, and coins given to the young.",
            "Medam 1, the Kerala rule: Mesha sankranti in the first 3/5 of the daytime → same day"),
        ("pohela_boishakh", "Pohela Boishakh (Bengali New Year)", 0, SolarRule::Bengal, "The Bengali new year: new account books opened, sweets shared.",
            "Boishakh 1, the Bengal rule: Mesha sankranti before midnight → next day"),
        ("pana_sankranti", "Pana Sankranti (Odia New Year)", 0, SolarRule::Odisha, "The Odia new year: a cooling drink shared and water given to the thirsty.",
            "Baisakha 1, the Odisha rule: the civil day of the Mesha sankranti"),
    ];
    for (key, name, sign, rule, what, why) in solar {
        for (s, d) in cal.solar_month_starts(sign, rule) {
            if let Some(i) = cal.index_of(d) {
                push(&mut out, cal, i, key, name.into(), s.at, None, "sun", what, why.into());
            }
        }
    }
    // Onam: Thiruvonam (Shravana, the 22nd nakshatra) in Chingam. The day is the
    // first in Chingam on which the star still holds six nazhikas (2 h 24 min) after
    // sunrise, the Kerala rule for star-days; failing that, the day it holds sunrise.
    for (_, start) in cal.solar_month_starts(4, SolarRule::Kerala) {
        let Some(first) = cal.index_of(start) else { continue };
        let end = cal.solar_month_starts(5, SolarRule::Kerala).into_iter().map(|x| x.1).find(|d| *d > start);
        let in_month = |i: usize| i < cal.days.len() && end.map_or(true, |e| cal.days[i].date < e) && cal.days[i].date >= start;
        let holds = |i: usize, after: i128| cal.star_span_at(cal.days[i].sunrise.add_ns(after)).filter(|s| s.index == 21);
        let pick = (first..cal.days.len()).take_while(|&i| in_month(i)).find_map(|i| holds(i, 6 * GHATI_NS).map(|s| (i, s))).or_else(|| {
            (first..cal.days.len()).take_while(|&i| in_month(i)).find_map(|i| holds(i, 0).map(|s| (i, s)))
        });
        if let Some((i, s)) = pick {
            push(&mut out, cal, i, "onam", "Onam (Thiruvonam)".into(), s.start, Some(s.end), "star",
                "Kerala's harvest festival: a flower carpet at the door, a feast on a banana leaf, the family together.",
                "Shravana (Thiruvonam) nakshatra in Chingam, holding six nazhikas after sunrise".into());
        }
    }
    let (a, b) = (cal.from, cal.from.plus(cal.count as i64));
    out.retain(|f| f.date >= a && f.date < b);
    out.sort_by(|x, y| x.date.cmp(&y.date).then(x.begins.cmp(&y.begins)).then(x.key.cmp(y.key)));
    out.dedup_by(|x, y| x.key == y.key && x.date == y.date);
    Ok(out)
}

/// The name and the plain words of every festival by key, `[en, ml, hi]`, as people say
/// them in Kerala and in Hindi. The English is exactly what [`festivals`] writes (the
/// Ekadashis are named apart: see [`localise`]).
const WORDS: &[(&str, [&str; 3], [&str; 3])] = &[
    ("ugadi", ["Ugadi / Gudi Padwa", "യുഗാദി", "गुड़ी पड़वा / उगादी"],
        ["The lunar new year: a fresh start, a bitter-sweet dish for the year's mixed days, and the year's almanac read aloud.",
         "ചാന്ദ്രവർഷത്തിന്റെ പുതുവർഷം: ഒരു പുതിയ തുടക്കം, വർഷത്തിലെ ഇടകലർന്ന ദിവസങ്ങൾക്കായി കയ്പും മധുരവുമുള്ള ഒരു വിഭവം, വർഷത്തെ പഞ്ചാംഗം ഉറക്കെ വായിക്കൽ.",
         "चांद्र नववर्ष: एक नई शुरुआत, साल के मिले-जुले दिनों के लिए खट्टा-मीठा व्यंजन, और साल का पंचांग पढ़कर सुनाया जाता है।"]),
    ("chaitra_navaratri", ["Chaitra Navaratri begins", "ചൈത്ര നവരാത്രി തുടങ്ങുന്നു", "चैत्र नवरात्रि आरंभ"],
        ["Nine nights of the Goddess in spring: a lamp each evening and a lighter diet.",
         "വസന്തകാലത്ത് ദേവിയുടെ ഒൻപത് രാത്രികൾ: എല്ലാ സന്ധ്യക്കും ഒരു വിളക്ക്, ലഘുവായ ഭക്ഷണം.",
         "वसंत में देवी की नौ रातें: हर शाम एक दीया और हल्का भोजन।"]),
    ("rama_navami", ["Rama Navami", "ശ്രീരാമനവമി", "राम नवमी"],
        ["Rama's birthday, kept at midday: a fast till noon, the story read, sweet water and fruit shared.",
         "ശ്രീരാമന്റെ ജന്മദിനം, ഉച്ചയ്ക്ക് ആചരിക്കുന്നു: ഉച്ചവരെ ഉപവാസം, രാമകഥ വായന, പാനകവും പഴങ്ങളും പങ്കുവയ്ക്കൽ.",
         "श्रीराम का जन्मदिन, दोपहर में मनाया जाता है: दोपहर तक उपवास, रामकथा का पाठ, शरबत और फल बाँटे जाते हैं।"]),
    ("akshaya_tritiya", ["Akshaya Tritiya", "അക്ഷയതൃതീയ", "अक्षय तृतीया"],
        ["What is begun or given today is said never to run out: a good day to start, buy gold or give food.",
         "ഇന്ന് തുടങ്ങുന്നതും കൊടുക്കുന്നതും ഒരിക്കലും തീരില്ല എന്നാണ് വിശ്വാസം: തുടങ്ങാനും സ്വർണം വാങ്ങാനും അന്നദാനത്തിനും നല്ല ദിവസം.",
         "मान्यता है कि आज शुरू किया या दिया हुआ कभी ख़त्म नहीं होता: शुरुआत करने, सोना ख़रीदने या अन्न दान के लिए अच्छा दिन।"]),
    ("guru_purnima", ["Guru Purnima", "വ്യാസപൂർണിമ", "गुरुपूर्णिमा"],
        ["The teachers' day: thank and honour those who taught you.",
         "അധ്യാപകരുടെ ദിവസം: നിങ്ങളെ പഠിപ്പിച്ചവരോട് നന്ദി പറയുക, അവരെ ആദരിക്കുക.",
         "शिक्षकों का दिन: जिन्होंने आपको सिखाया, उनका धन्यवाद करें और उन्हें सम्मान दें।"]),
    ("raksha_bandhan", ["Raksha Bandhan", "രക്ഷാബന്ധൻ", "रक्षाबंधन"],
        ["Sisters tie a protective thread on brothers' wrists; brothers promise care in return.",
         "സഹോദരിമാർ സഹോദരന്മാരുടെ കൈയിൽ രക്ഷാച്ചരട് കെട്ടുന്നു; സഹോദരന്മാർ തിരികെ കരുതൽ വാഗ്ദാനം ചെയ്യുന്നു.",
         "बहनें भाइयों की कलाई पर रक्षा सूत्र बाँधती हैं; भाई बदले में रक्षा का वचन देते हैं।"]),
    ("janmashtami", ["Krishna Janmashtami", "ശ്രീകൃഷ്ണ ജന്മാഷ്ടമി", "कृष्ण जन्माष्टमी"],
        ["Krishna's birth at midnight: a fast through the day, songs, and the fast broken after midnight.",
         "അർധരാത്രിയിലെ ശ്രീകൃഷ്ണജനനം: പകൽ മുഴുവൻ ഉപവാസം, ഭജന, അർധരാത്രി കഴിഞ്ഞ് ഉപവാസം മുറിക്കൽ.",
         "आधी रात को श्रीकृष्ण का जन्म: दिन भर उपवास, भजन, और आधी रात के बाद व्रत खोलना।"]),
    ("ganesh_chaturthi", ["Ganesh Chaturthi", "വിനായക ചതുർത്ഥി", "गणेश चतुर्थी"],
        ["Ganesha is welcomed home at midday; begin the new and clear what blocks you.",
         "ഉച്ചയ്ക്ക് ഗണപതിയെ വീട്ടിലേക്ക് സ്വാഗതം ചെയ്യുന്നു; പുതിയത് തുടങ്ങുക, തടസ്സമായതെല്ലാം നീക്കുക.",
         "दोपहर में गणेश जी का घर में स्वागत होता है; नया शुरू करें और जो अड़चन है उसे हटाएँ।"]),
    ("navaratri", ["Navaratri begins", "നവരാത്രി തുടങ്ങുന്നു", "नवरात्रि आरंभ"],
        ["Nine nights of the Goddess: the pot is set up in the morning and a lamp kept each evening.",
         "ദേവിയുടെ ഒൻപത് രാത്രികൾ: രാവിലെ കലശം വയ്ക്കുന്നു, എല്ലാ സന്ധ്യക്കും വിളക്ക് കൊളുത്തുന്നു.",
         "देवी की नौ रातें: सुबह कलश स्थापना होती है और हर शाम दीया जलाया जाता है।"]),
    ("dussehra", ["Dussehra / Vijayadashami", "വിജയദശമി", "दशहरा / विजयादशमी"],
        ["Victory of good over evil: begin learning, honour your tools and books, and make peace.",
         "തിന്മയുടെ മേൽ നന്മയുടെ വിജയം: വിദ്യാരംഭം കുറിക്കുക, പണിയായുധങ്ങളെയും പുസ്തകങ്ങളെയും ആദരിക്കുക, പിണക്കങ്ങൾ തീർക്കുക.",
         "बुराई पर अच्छाई की जीत: पढ़ाई शुरू करें, अपने औज़ारों और किताबों का पूजन करें, और मेल-मिलाप करें।"]),
    ("karva_chauth", ["Karva Chauth", "കർവാ ചൗത്ത്", "करवा चौथ"],
        ["Wives fast from dawn for their husbands and break the fast at moonrise.",
         "ഭാര്യമാർ ഭർത്താക്കന്മാർക്കായി പുലർച്ചെ മുതൽ ഉപവസിക്കുന്നു, ചന്ദ്രോദയത്തിൽ ഉപവാസം മുറിക്കുന്നു.",
         "पत्नियाँ पति के लिए भोर से व्रत रखती हैं और चाँद निकलने पर व्रत खोलती हैं।"]),
    ("diwali", ["Diwali (Lakshmi Puja)", "ദീപാവലി (ലക്ഷ്മീപൂജ)", "दीपावली (लक्ष्मी पूजा)"],
        ["The festival of lights: lamps at dusk, Lakshmi welcomed, the home cleaned and accounts begun afresh.",
         "ദീപങ്ങളുടെ ഉത്സവം: സന്ധ്യക്ക് വിളക്കുകൾ, ലക്ഷ്മീദേവിക്ക് സ്വാഗതം, വീട് വൃത്തിയാക്കൽ, കണക്കുകൾ പുതുതായി തുടങ്ങൽ.",
         "रोशनी का त्योहार: शाम को दीये, लक्ष्मी जी का स्वागत, घर की सफ़ाई और नए बही-खाते।"]),
    ("vasant_panchami", ["Vasant Panchami", "വസന്തപഞ്ചമി", "वसंत पंचमी"],
        ["Saraswati's day at the start of spring: children begin letters, books and instruments are honoured.",
         "വസന്തത്തിന്റെ തുടക്കത്തിലെ സരസ്വതീദിനം: കുട്ടികൾ അക്ഷരം കുറിക്കുന്നു, പുസ്തകങ്ങളും വാദ്യങ്ങളും ആദരിക്കപ്പെടുന്നു.",
         "वसंत की शुरुआत में सरस्वती का दिन: बच्चे अक्षर सीखना शुरू करते हैं, किताबों और वाद्यों का पूजन होता है।"]),
    ("maha_shivaratri", ["Maha Shivaratri", "ശിവരാത്രി", "महाशिवरात्रि"],
        ["The great night of Shiva: a fast, a lamp and a night's vigil, at least till midnight.",
         "ശിവന്റെ മഹാരാത്രി: ഉപവാസം, വിളക്ക്, ഉറക്കമൊഴിയൽ, കുറഞ്ഞത് അർധരാത്രി വരെ.",
         "शिव की महान रात: उपवास, दीया और रात भर जागरण, कम से कम आधी रात तक।"]),
    ("holika_dahan", ["Holika Dahan", "ഹോളികാ ദഹനം", "होलिका दहन"],
        ["The bonfire at dusk on the eve of Holi: old grudges and the year's waste go into the fire.",
         "ഹോളിയുടെ തലേന്ന് സന്ധ്യക്കുള്ള അഗ്നി: പഴയ പിണക്കങ്ങളും വർഷത്തെ മാലിന്യവും തീയിലേക്ക്.",
         "होली की पूर्व संध्या पर अलाव: पुराने गिले-शिकवे और साल का कचरा आग में।"]),
    ("holi", ["Holi", "ഹോളി", "होली"],
        ["The festival of colours, the morning after the bonfire: play, forgive and visit.",
         "അഗ്നിക്ക് പിറ്റേന്ന് രാവിലെ നിറങ്ങളുടെ ഉത്സവം: കളിക്കുക, ക്ഷമിക്കുക, സന്ദർശിക്കുക.",
         "अलाव के अगले दिन सुबह रंगों का त्योहार: खेलें, माफ़ करें और मिलने जाएँ।"]),
    ("pradosham", ["Pradosham", "പ്രദോഷം", "प्रदोष व्रत"],
        ["The dusk hours are Shiva's: light a lamp and make an apology; it eases delays.",
         "സന്ധ്യാസമയം ശിവന്റേതാണ്: ഒരു വിളക്ക് കൊളുത്തുക, ഒരു ക്ഷമ ചോദിക്കുക; താമസങ്ങൾ അയയും.",
         "शाम का समय शिव का है: दीया जलाएँ और किसी से माफ़ी माँगें; रुकावटें कम होती हैं।"]),
    ("sankashti", ["Sankashti Chaturthi", "സങ്കടഹര ചതുർത്ഥി", "संकष्टी चतुर्थी"],
        ["Ganesha's fast, broken after moonrise: finish what was left undone; obstacles ease.",
         "ഗണപതിക്കുള്ള വ്രതം, ചന്ദ്രോദയത്തിന് ശേഷം മുറിക്കുന്നു: ബാക്കിവച്ചത് പൂർത്തിയാക്കുക; തടസ്സങ്ങൾ നീങ്ങും.",
         "गणेश जी का व्रत, चाँद निकलने के बाद खोला जाता है: अधूरा काम पूरा करें; अड़चनें कम होती हैं।"]),
    ("masik_shivaratri", ["Masik Shivaratri", "മാസശിവരാത്രി", "मासिक शिवरात्रि"],
        ["The month's night of Shiva: a lamp kept through the night, or at least till midnight.",
         "മാസത്തിലെ ശിവരാത്രി: രാത്രി മുഴുവൻ, അല്ലെങ്കിൽ കുറഞ്ഞത് അർധരാത്രി വരെ, ഒരു വിളക്ക്.",
         "महीने की शिवरात्रि: रात भर, या कम से कम आधी रात तक, एक दीया।"]),
    ("purnima", ["Purnima", "പൗർണമി", "पूर्णिमा"],
        ["Full moon: honour your mother, feed women and children, sit by water.",
         "വെളുത്തവാവ്: അമ്മയെ ആദരിക്കുക, സ്ത്രീകൾക്കും കുട്ടികൾക്കും ഭക്ഷണം കൊടുക്കുക, വെള്ളത്തിനരികെ ഇരിക്കുക.",
         "पूरा चाँद: अपनी माँ का सम्मान करें, स्त्रियों और बच्चों को भोजन कराएँ, पानी के पास बैठें।"]),
    ("amavasya", ["Amavasya", "കറുത്തവാവ്", "अमावस्या"],
        ["The ancestors' day: give food in their name and begin nothing new.",
         "പിതൃക്കളുടെ ദിവസം: അവരുടെ പേരിൽ അന്നദാനം ചെയ്യുക, പുതിയതൊന്നും തുടങ്ങരുത്.",
         "पितरों का दिन: उनके नाम पर अन्न दान करें और कुछ नया शुरू न करें।"]),
    ("vinayaka_chaturthi", ["Vinayaka Chaturthi", "വിനായക ചതുർത്ഥി", "विनायक चतुर्थी"],
        ["Ganesha's day of the bright fortnight, kept at midday.",
         "വെളുത്ത പക്ഷത്തിലെ ഗണപതിദിനം, ഉച്ചയ്ക്ക് ആചരിക്കുന്നു.",
         "शुक्ल पक्ष का गणेश जी का दिन, दोपहर में मनाया जाता है।"]),
    ("makar_sankranti", ["Makar Sankranti", "മകരസംക്രാന്തി", "मकर संक्रांति"],
        ["The turn towards longer days: til and jaggery shared, a bath at dawn, gifts to those in need.",
         "പകൽ നീളുന്നതിലേക്കുള്ള തിരിവ്: എള്ളും ശർക്കരയും പങ്കുവയ്ക്കൽ, പുലർച്ചെ കുളി, ആവശ്യക്കാർക്ക് ദാനം.",
         "लंबे दिनों की ओर मोड़: तिल-गुड़ बाँटना, भोर में स्नान, ज़रूरतमंदों को दान।"]),
    ("pongal", ["Pongal", "പൊങ്കൽ", "पोंगल"],
        ["The harvest festival: rice boiled over in a new pot, thanks given to the land and cattle.",
         "വിളവെടുപ്പുത്സവം: പുതിയ കലത്തിൽ അരി തിളച്ചുതൂവുന്നു, മണ്ണിനും കന്നുകാലികൾക്കും നന്ദി.",
         "फ़सल का त्योहार: नए बर्तन में चावल उफनता है, धरती और पशुओं का धन्यवाद।"]),
    ("puthandu", ["Puthandu (Tamil New Year)", "പുത്താണ്ട് (തമിഴ് പുതുവർഷം)", "पुथांडु (तमिल नववर्ष)"],
        ["The Tamil new year: a tray of fruit, flowers and a mirror seen first in the morning.",
         "തമിഴ് പുതുവർഷം: രാവിലെ ആദ്യം കാണുന്നത് പഴങ്ങളും പൂക്കളും കണ്ണാടിയും വച്ച ഒരു താലം.",
         "तमिल नववर्ष: सुबह सबसे पहले फल, फूल और दर्पण से सजी थाली देखी जाती है।"]),
    ("vishu", ["Vishu", "വിഷു", "विषु"],
        ["Kerala's new year: the Vishukkani seen first at dawn, and coins given to the young.",
         "കേരളത്തിന്റെ പുതുവർഷം: പുലർച്ചെ ആദ്യം വിഷുക്കണി കാണൽ, കുട്ടികൾക്ക് വിഷുക്കൈനീട്ടം.",
         "केरल का नववर्ष: भोर में सबसे पहले विषुक्कणि का दर्शन, और बच्चों को सिक्के दिए जाते हैं।"]),
    ("pohela_boishakh", ["Pohela Boishakh (Bengali New Year)", "പൊയ്‌ല ബൈശാഖ് (ബംഗാളി പുതുവർഷം)", "पोइला बैसाख (बंगाली नववर्ष)"],
        ["The Bengali new year: new account books opened, sweets shared.",
         "ബംഗാളി പുതുവർഷം: പുതിയ കണക്കുപുസ്തകങ്ങൾ തുറക്കുന്നു, മധുരം പങ്കുവയ്ക്കുന്നു.",
         "बंगाली नववर्ष: नए बही-खाते खुलते हैं, मिठाई बाँटी जाती है।"]),
    ("pana_sankranti", ["Pana Sankranti (Odia New Year)", "പണ സംക്രാന്തി (ഒഡിയ പുതുവർഷം)", "पना संक्रांति (ओड़िया नववर्ष)"],
        ["The Odia new year: a cooling drink shared and water given to the thirsty.",
         "ഒഡിയ പുതുവർഷം: തണുത്ത പാനീയം പങ്കുവയ്ക്കൽ, ദാഹിക്കുന്നവർക്ക് വെള്ളം.",
         "ओड़िया नववर्ष: ठंडा पना बाँटा जाता है और प्यासों को पानी पिलाया जाता है।"]),
    ("onam", ["Onam (Thiruvonam)", "തിരുവോണം", "ओणम (तिरुवोणम)"],
        ["Kerala's harvest festival: a flower carpet at the door, a feast on a banana leaf, the family together.",
         "കേരളത്തിന്റെ വിളവെടുപ്പുത്സവം: വാതിൽക്കൽ പൂക്കളം, ഇലയിൽ ഓണസദ്യ, കുടുംബം ഒന്നിച്ച്.",
         "केरल का फ़सल त्योहार: दरवाज़े पर फूलों की रंगोली, केले के पत्ते पर भोज, पूरा परिवार साथ।"]),
];

/// The name and the plain words of a festival in the reader's language (`"en"`, `"ml"`,
/// `"hi"`; anything else is English). English returns the festival's own text unchanged.
pub fn localise(f: &Festival, lang: &str) -> (String, &'static str) {
    let l = crate::names::lang_index(lang);
    if l == 0 {
        return (f.name.clone(), f.what);
    }
    if f.key == "ekadashi" || f.key == "ekadashi_vaishnava" {
        let base = f.name.trim_end_matches(" (Vaishnava)").trim_end_matches(" Ekadashi");
        let own = crate::names::ekadashi_name_in(base, lang).unwrap_or(base);
        let name = match (l, f.key == "ekadashi_vaishnava") {
            (1, false) => format!("{own} ഏകാദശി"),
            (1, true) => format!("{own} ഏകാദശി (വൈഷ്ണവർ)"),
            (_, false) => format!("{own} एकादशी"),
            (_, true) => format!("{own} एकादशी (वैष्णव)"),
        };
        let what = if f.key == "ekadashi" {
            ["", "വിഷ്ണുവിനായുള്ള വ്രതം: അരിയാഹാരം ഇല്ല, മനസ്സ് ലഘുവായി, പിറ്റേന്ന് രാവിലെ വ്രതം മുറിക്കും.", "विष्णु का व्रत: अन्न नहीं, मन हल्का रखें, अगली सुबह व्रत खोलें।"][l]
        } else {
            ["", "വൈഷ്ണവ കുടുംബങ്ങൾ നോക്കുന്ന അതേ വ്രതം, ഇത്തവണ ഒരു ദിവസം കഴിഞ്ഞ്.", "वैष्णव परिवारों का वही व्रत, इस बार एक दिन बाद।"][l]
        };
        return (name, what);
    }
    // Pradosham has two plain lines (the bright fortnight's asks for an apology too).
    if f.key == "pradosham" && f.what.ends_with("light a lamp; it eases delays.") {
        let w = ["", "സന്ധ്യാസമയം ശിവന്റേതാണ്: ഒരു വിളക്ക് കൊളുത്തുക; താമസങ്ങൾ അയയും.", "शाम का समय शिव का है: दीया जलाएँ; रुकावटें कम होती हैं।"][l];
        return (["", "പ്രദോഷം", "प्रदोष व्रत"][l].to_string(), w);
    }
    match WORDS.iter().find(|(k, _, _)| *k == f.key) {
        Some((_, n, w)) => (n[l].to_string(), w[l]),
        None => (f.name.clone(), f.what),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;
    use crate::{Config, Date};
    use kshana_sky::Observer;

    fn on(spk: &kshana_ephem::Spk, obs: Observer, from: Date, days: u32) -> Vec<Festival> {
        let cal = Calendar::new(spk, None, obs, Config::india(), from, days).unwrap();
        festivals(&cal).unwrap()
    }

    fn date_of(f: &[Festival], key: &str) -> Vec<Date> {
        f.iter().filter(|x| x.key == key).map(|x| x.date).collect()
    }

    /// Dates as published by Drik Panchang for New Delhi (and the regional city named),
    /// 2025–2026. Only dates that are not in dispute are pinned.
    #[test]
    fn delhi_2025_matches_the_published_panchang() {
        let Some(spk) = kernel() else { return };
        let f = on(&spk, DELHI, Date::new(2025, 1, 1), 365);
        let d = |m, day| Date::new(2025, m, day);
        assert_eq!(date_of(&f, "makar_sankranti"), vec![d(1, 14)]);
        assert_eq!(date_of(&f, "maha_shivaratri"), vec![d(2, 26)]);
        assert_eq!(date_of(&f, "holika_dahan"), vec![d(3, 13)]);
        assert_eq!(date_of(&f, "holi"), vec![d(3, 14)]);
        assert_eq!(date_of(&f, "ugadi"), vec![d(3, 30)]);
        assert_eq!(date_of(&f, "rama_navami"), vec![d(4, 6)]);
        assert_eq!(date_of(&f, "akshaya_tritiya"), vec![d(4, 30)]);
        assert_eq!(date_of(&f, "guru_purnima"), vec![d(7, 10)]);
        assert_eq!(date_of(&f, "raksha_bandhan"), vec![d(8, 9)]);
        assert_eq!(date_of(&f, "ganesh_chaturthi"), vec![d(8, 27)]);
        assert_eq!(date_of(&f, "navaratri"), vec![d(9, 22)]);
        assert_eq!(date_of(&f, "dussehra"), vec![d(10, 2)]);
        assert_eq!(date_of(&f, "karva_chauth"), vec![d(10, 10)]);
        assert_eq!(date_of(&f, "diwali"), vec![d(10, 20)]);
        assert!(f.iter().any(|x| x.date == d(7, 6) && x.name == "Devshayani Ekadashi"), "{:?}", date_of(&f, "ekadashi"));
        // 24 Ekadashis in a year without an adhika month (sometimes 25 when one
        // straddles the year's ends).
        let n = date_of(&f, "ekadashi").len();
        assert!((24..=25).contains(&n), "{n}");
        assert!((24..=26).contains(&date_of(&f, "pradosham").len()));
        assert!((11..=12).contains(&date_of(&f, "masik_shivaratri").len()));
    }

    #[test]
    fn delhi_2026_matches_the_published_panchang() {
        let Some(spk) = kernel() else { return };
        let f = on(&spk, DELHI, Date::new(2026, 1, 1), 120);
        assert_eq!(date_of(&f, "makar_sankranti"), vec![Date::new(2026, 1, 14)]);
        assert_eq!(date_of(&f, "maha_shivaratri"), vec![Date::new(2026, 2, 15)]);
        let m = on(&spk, MUMBAI, Date::new(2026, 3, 10), 20);
        assert_eq!(date_of(&m, "ugadi"), vec![Date::new(2026, 3, 19)]);
    }

    #[test]
    fn kerala_and_tamil_festivals() {
        let Some(spk) = kernel() else { return };
        let k = on(&spk, KOCHI, Date::new(2025, 4, 1), 200);
        assert_eq!(date_of(&k, "vishu"), vec![Date::new(2025, 4, 14)]);
        assert_eq!(date_of(&k, "onam"), vec![Date::new(2025, 9, 5)]);
        let k = on(&spk, KOCHI, Date::new(2026, 4, 1), 30);
        assert_eq!(date_of(&k, "vishu"), vec![Date::new(2026, 4, 14)]);
        let c = on(&spk, CHENNAI, Date::new(2025, 1, 1), 120);
        assert_eq!(date_of(&c, "pongal"), vec![Date::new(2025, 1, 14)]);
        assert_eq!(date_of(&c, "puthandu"), vec![Date::new(2025, 4, 14)]);
    }

    #[test]
    fn no_festival_in_the_adhika_month() {
        let Some(spk) = kernel() else { return };
        // Adhika Jyeshtha 2026 (17 May – 15 June): its Ekadashis are Padmini and
        // Parama, and no yearly festival falls in it.
        let f = on(&spk, DELHI, Date::new(2026, 5, 17), 29);
        let ek: Vec<&str> = f.iter().filter(|x| x.key == "ekadashi").map(|x| x.name.as_str()).collect();
        assert_eq!(ek, vec!["Padmini Ekadashi", "Parama Ekadashi"]);
        assert!(f.iter().all(|x| !YEARLY.iter().any(|r| r.key == x.key)));
    }

    #[test]
    fn every_festival_has_its_words_in_three_languages() {
        let Some(spk) = kernel() else { return };
        let mut seen = std::collections::BTreeSet::new();
        for (obs, from) in [(DELHI, Date::new(2025, 1, 1)), (KOCHI, Date::new(2025, 1, 1)), (CHENNAI, Date::new(2025, 1, 1))] {
            for f in on(&spk, obs, from, 366) {
                assert_eq!(localise(&f, "en"), (f.name.clone(), f.what));
                if !f.key.starts_with("ekadashi") && !(f.key == "pradosham" && f.what.ends_with("light a lamp; it eases delays.")) {
                    let row = WORDS.iter().find(|(k, _, _)| *k == f.key).unwrap_or_else(|| panic!("no words for {}", f.key));
                    assert_eq!((row.1[0], row.2[0]), (f.name.as_str(), f.what), "{}", f.key);
                }
                for lang in ["ml", "hi"] {
                    let (n, w) = localise(&f, lang);
                    assert!(!n.is_empty() && !w.is_empty(), "{} {lang}", f.key);
                    assert!(!n.chars().any(|c| c.is_ascii_alphabetic()), "{lang}: {n}");
                    assert!(!w.chars().any(|c| c.is_ascii_alphabetic()), "{lang}: {w}");
                }
                seen.insert(f.key);
            }
        }
        assert!(seen.contains("onam") && seen.contains("vishu") && seen.contains("diwali"));
    }
}
