//! The day as a feed and the wall of countdowns: the next good minute for every life,
//! the exact instants the person is waiting for (Saturn's exit, the period's turn,
//! Jupiter's next house, the next sade sati, the year's close, the birth star's
//! return, the lunar days), and the observances of the coming month, each solved on
//! the kernel. Everything here is what a person checks again and again.

use crate::briefing::saturn_sentence_in;
use crate::plain::Lang;
use crate::forecast::{date_words, Life, Moment, Natal};
use crate::gochara::{favourable_from_moon, SaturnPhase};
use crate::transit::{kernel_end, next_sign_change, returns, sidereal_longitude, sign_changes};
use kshana_ephem::{Body, Spk};
use kshana_events::{nakshatra_boundaries, solar_return, tithi_boundaries, MICROSECOND_NS};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{ayanamsa_degrees, nakshatra, tithi, vimshottari_levels, Ayanamsa, DashaLord, Tithi};

/// The next hour that serves a life.
#[derive(Clone, Debug, PartialEq)]
pub struct GoodMinute {
    /// Which life.
    pub life: Life,
    /// Start (or `at` when the running hour already serves it).
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Hora fit, `[0, 1]`.
    pub fit: f64,
    /// Lord of that hour.
    pub lord: DashaLord,
    /// True when it is running now.
    pub now: bool,
}

/// The next hour at or above `floor` for each life, outside the cautions, from the
/// day's hours; `None` for a life when nothing is left today.
pub fn next_good(m: &Moment, at: Instant, floor: f64) -> Vec<Option<GoodMinute>> {
    Life::ALL
        .iter()
        .enumerate()
        .map(|(i, &life)| {
            m.hours
                .iter()
                .filter(|h| !h.caution && h.end > at && h.fit[i] >= floor)
                .min_by_key(|h| h.start.tai_ns())
                .map(|h| GoodMinute {
                    life,
                    start: if h.start <= at { at } else { h.start },
                    end: h.end,
                    fit: h.fit[i],
                    lord: h.lord,
                    now: h.start <= at,
                })
        })
        .collect()
}

/// A name for the day: "Sampat Shashthi, the Moon's day".
pub fn day_title(tara: usize, _t: &Tithi, day_lord: DashaLord) -> String {
    let day = match day_lord {
        DashaLord::Sun => "Sunday",
        DashaLord::Moon => "Monday",
        DashaLord::Mars => "Tuesday",
        DashaLord::Mercury => "Wednesday",
        DashaLord::Jupiter => "Thursday",
        DashaLord::Venus => "Friday",
        DashaLord::Saturn => "Saturday",
        _ => "Today",
    };
    format!("{day}, {}", crate::plain::day_short(tara))
}

/// One thing the person is waiting for.
#[derive(Clone, Debug, PartialEq)]
pub struct Countdown {
    /// `saturn`, `dasha`, `jupiter`, `rahu`, `year`, `star`, `moon`, `return`, `sadesati`.
    pub kind: &'static str,
    /// Short title.
    pub title: String,
    /// The exact instant.
    pub at: Instant,
    /// One sentence.
    pub words: String,
    /// True for a change the person will welcome.
    pub good: bool,
}

/// The wall of countdowns from `at`, soonest first.
pub fn countdowns(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Vec<Countdown> {
    countdowns_in(Lang::En, spk, eop, natal, obs, ayanamsa, at)
}

/// [`countdowns`] with every title and sentence in `lang`.
pub fn countdowns_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Vec<Countdown> {
    let _ = eop;
    let lon = obs.longitude_deg;
    let stop = if lang == Lang::Hi { "।" } else { "." };
    let dl = |t: Instant| crate::lang::date(lang, t, lon);
    let pick = |t: [&str; 3]| lang.pick(t).to_string();
    let horizon = kernel_end(spk);
    let mut v: Vec<Countdown> = Vec::new();
    let ms = natal.moon_sign();
    let house_of = |sign: u8| ((sign as i64 - ms).rem_euclid(12) + 1) as u8;

    // Saturn: the end of the running phase, or the start of the next.
    let sat_sign = sidereal_longitude(spk, ayanamsa, DashaLord::Saturn, at)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let phase = SaturnPhase::from_house(house_of(sat_sign));
    let (words, until) = saturn_sentence_in(lang, spk, ayanamsa, natal, phase, at, lon);
    if let Some(t) = until {
        v.push(Countdown {
            kind: "saturn",
            title: if phase == SaturnPhase::Clear {
                pick(["A long season turns", "ഒരു നീണ്ട കാലം മാറുന്നു", "एक लंबा दौर बदलता है"])
            } else {
                pick(["Your testing time ends", "നിങ്ങളുടെ പരീക്ഷണകാലം അവസാനിക്കുന്നു", "आपका कठिन समय ख़त्म होता है"])
            },
            at: t,
            words: format!("{words}{stop}"),
            good: phase != SaturnPhase::Clear,
        });
    }
    // The next sade sati, when clear of it now.
    if !matches!(phase, SaturnPhase::SadeSatiRising | SaturnPhase::SadeSatiPeak | SaturnPhase::SadeSatiSetting) {
        let to = at.add_ns(32 * 365 * NS_PER_DAY).min(horizon);
        if let Some(c) = sign_changes(spk, ayanamsa, DashaLord::Saturn, at, to)
            .into_iter()
            .find(|c| house_of(c.to) == 12)
        {
            v.push(Countdown {
                kind: "sadesati",
                title: pick(["Next long testing period", "അടുത്ത നീണ്ട പരീക്ഷണകാലം", "अगला लंबा कठिन दौर"]),
                at: c.at,
                words: match lang {
                    Lang::En => format!("From {} a long testing period of about seven and a half years begins: expenses rise, sleep gets lighter and patience is asked. Save money before then.", date_words(c.at, lon)),
                    Lang::Ml => format!("{} മുതൽ ഏകദേശം ഏഴര വർഷം നീളുന്ന ഒരു പരീക്ഷണകാലം തുടങ്ങും: ചെലവ് കൂടും, ഉറക്കം കുറയും, ക്ഷമ വേണ്ടിവരും. അതിനുമുമ്പ് പണം കരുതിവയ്ക്കുക.", dl(c.at)),
                    Lang::Hi => format!("{} से सात साल से कुछ ज़्यादा का एक लंबा कठिन दौर शुरू होगा: खर्च बढ़ेगा, नींद हल्की होगी और धैर्य माँगा जाएगा। उससे पहले पैसा बचा लें।", dl(c.at)),
                },
                good: false,
            });
        }
    }
    // The periods.
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    for (i, p) in levels.iter().enumerate().take(3) {
        let name = ["chapter of your life", "stretch of your life", "shorter phase of a few months"][i];
        let idx = DashaLord::ORDER.iter().position(|l| *l == p.lord).unwrap_or(0);
        let next = DashaLord::ORDER[(idx + 1) % 9];
        let (th, tn) = (crate::lang::chapter_theme(lang, p.lord), crate::lang::chapter_theme(lang, next));
        let (title, words) = match lang {
            Lang::En => (
                format!("This {name} ends"),
                format!(
                    "Until {} this {name} is about {}. Then {} come forward.",
                    date_words(p.end, lon),
                    crate::plain::chapter_theme(p.lord),
                    crate::plain::chapter_theme(next)
                ),
            ),
            Lang::Ml => {
                let name = ["ജീവിതത്തിലെ ഈ അധ്യായം", "ജീവിതത്തിലെ ഈ ഘട്ടം", "ഏതാനും മാസത്തെ ഈ ചെറിയ ഘട്ടം"][i];
                (
                    format!("{name} അവസാനിക്കുന്നു"),
                    format!("{} വരെ {name} {th} എന്നിവയെക്കുറിച്ചാണ്. പിന്നെ {tn} എന്നിവ മുന്നോട്ടുവരും.", dl(p.end)),
                )
            }
            Lang::Hi => {
                let name = ["जीवन का यह अध्याय", "जीवन का यह दौर", "कुछ महीनों का यह छोटा दौर"][i];
                (
                    format!("{name} ख़त्म होता है"),
                    format!("{} तक {name} {th} के बारे में है। फिर {tn} आगे आते हैं।", dl(p.end)),
                )
            }
        };
        v.push(Countdown {
            kind: "dasha",
            title,
            at: p.end,
            words,
            good: true,
        });
    }
    // Jupiter's and Rahu's next houses.
    for g in [DashaLord::Jupiter, DashaLord::Rahu] {
        if let Some(c) = next_sign_change(spk, ayanamsa, g, at) {
            let h = house_of(c.to);
            let fav = favourable_from_moon(g).contains(&h);
            v.push(Countdown {
                kind: if g == DashaLord::Jupiter { "jupiter" } else { "rahu" },
                title: match (g, fav) {
                    (DashaLord::Jupiter, true) => pick(["A year of growth begins", "വളർച്ചയുടെ ഒരു വർഷം തുടങ്ങുന്നു", "तरक्की का एक साल शुरू"]),
                    (DashaLord::Jupiter, false) => pick(["Growth slows for a year", "ഒരു വർഷത്തേക്ക് വളർച്ച മന്ദമാകുന്നു", "एक साल तक तरक्की धीमी"]),
                    (_, true) => pick(["A season of sudden openings", "പെട്ടെന്നുള്ള അവസരങ്ങളുടെ കാലം", "अचानक मौकों का दौर"]),
                    _ => pick(["A confusing season begins", "ആശയക്കുഴപ്പത്തിന്റെ ഒരു കാലം തുടങ്ങുന്നു", "उलझन भरा दौर शुरू"]),
                },
                at: c.at,
                words: match (lang, g, fav) {
                    (Lang::En, DashaLord::Jupiter, true) => format!("From {} for about a year, money, children, teachers and good advice come more easily. Start what you want to grow.", date_words(c.at, lon)),
                    (Lang::En, DashaLord::Jupiter, false) => format!("From {} for about a year, growth is slower: keep promises small and spending careful.", date_words(c.at, lon)),
                    (Lang::En, _, true) => format!("From {} for about a year and a half, bold moves, foreign links and new technology pay off.", date_words(c.at, lon)),
                    (Lang::En, _, false) => format!("From {} for about a year and a half, confusion and tempting shortcuts are around: check before you trust, and sign nothing in a hurry.", date_words(c.at, lon)),
                    (Lang::Ml, DashaLord::Jupiter, true) => format!("{} മുതൽ ഏകദേശം ഒരു വർഷം പണം, മക്കൾ, അധ്യാപകർ, നല്ല ഉപദേശം എന്നിവ എളുപ്പം വന്നുചേരും. വളരണമെന്ന് ആഗ്രഹിക്കുന്നത് ഇപ്പോൾ തുടങ്ങുക.", dl(c.at)),
                    (Lang::Ml, DashaLord::Jupiter, false) => format!("{} മുതൽ ഏകദേശം ഒരു വർഷം വളർച്ച മന്ദമാണ്: വാഗ്ദാനങ്ങൾ ചെറുതാക്കുക, ചെലവ് ശ്രദ്ധിക്കുക.", dl(c.at)),
                    (Lang::Ml, _, true) => format!("{} മുതൽ ഏകദേശം ഒന്നര വർഷം ധീരമായ നീക്കങ്ങൾക്കും വിദേശബന്ധങ്ങൾക്കും പുതിയ സാങ്കേതികവിദ്യയ്ക്കും ഫലം കിട്ടും.", dl(c.at)),
                    (Lang::Ml, _, false) => format!("{} മുതൽ ഏകദേശം ഒന്നര വർഷം ആശയക്കുഴപ്പവും എളുപ്പവഴികളുടെ പ്രലോഭനവും ചുറ്റുമുണ്ടാകും: വിശ്വസിക്കുംമുമ്പ് പരിശോധിക്കുക, ധൃതിയിൽ ഒന്നും ഒപ്പിടരുത്.", dl(c.at)),
                    (_, DashaLord::Jupiter, true) => format!("{} से लगभग एक साल तक पैसा, संतान, शिक्षक और अच्छी सलाह आसानी से मिलेंगे। जो बढ़ाना चाहते हैं, वह अभी शुरू करें।", dl(c.at)),
                    (_, DashaLord::Jupiter, false) => format!("{} से लगभग एक साल तक तरक्की धीमी रहेगी: वादे छोटे रखें और खर्च सोच-समझकर करें।", dl(c.at)),
                    (_, _, true) => format!("{} से लगभग डेढ़ साल तक हिम्मत भरे कदम, विदेश से जुड़ाव और नई तकनीक फ़ायदा देंगे।", dl(c.at)),
                    (_, _, false) => format!("{} से लगभग डेढ़ साल तक उलझन और शॉर्टकट का लालच आसपास रहेगा: भरोसा करने से पहले जाँचें और जल्दबाज़ी में किसी चीज़ पर दस्तख़त न करें।", dl(c.at)),
                },
                good: fav,
            });
        }
    }
    // The year's close: the next solar return.
    let sun_sid = natal.facts.planet(DashaLord::Sun).sidereal;
    let sun_trop = (sun_sid + ayanamsa_degrees(ayanamsa, natal.birth.tt_centuries_since_j2000())).rem_euclid(360.0);
    let year = Utc::from_instant(at).year;
    for y in [year, year + 1] {
        if let Ok(Some(r)) = solar_return(spk, sun_trop, natal.birth, y, MICROSECOND_NS) {
            if r > at {
                let age = ((r.ns_since(natal.birth)) as f64 / (365.25 * NS_PER_DAY as f64)).round() as i64;
                v.push(Countdown {
                    kind: "year",
                    title: pick(["Your new year begins", "നിങ്ങളുടെ പുതിയ വർഷം തുടങ്ങുന്നു", "आपका नया साल शुरू"]),
                    at: r,
                    words: match lang {
                        Lang::En => format!("Your {} year begins on {}, to the minute: a good day to set the year's intentions.", ordinal_year(age + 1), date_words(r, lon)),
                        Lang::Ml => format!("{}-ന്, കൃത്യം ആ നിമിഷം, നിങ്ങളുടെ ജീവിതത്തിലെ {}-ാം വർഷം തുടങ്ങും: ഈ വർഷത്തെ ലക്ഷ്യങ്ങൾ മനസ്സിൽ ഉറപ്പിക്കാൻ നല്ല ദിവസം.", dl(r), age + 1),
                        Lang::Hi => format!("{} को, ठीक उसी पल, आपके जीवन का {} साल शुरू होगा: साल के संकल्प तय करने का अच्छा दिन।", dl(r), crate::forecast::hi_ordinal((age + 1).max(1) as usize)),
                    },
                    good: true,
                });
                break;
            }
        }
    }
    // The birth star's return and the lunar days, within forty days.
    let to40 = at.add_ns(40 * NS_PER_DAY);
    if let Ok(ev) = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, to40, MICROSECOND_NS) {
        let cur = sidereal_longitude(spk, ayanamsa, DashaLord::Moon, at).map(|l| nakshatra(l).index).unwrap_or(0);
        for (i, e) in ev.iter().enumerate() {
            if (cur + 1 + i) % 27 == natal.janma() {
                v.push(Countdown {
                    kind: "star",
                    title: pick(["Your lucky day of the month", "മാസത്തിലെ നിങ്ങളുടെ ഭാഗ്യദിവസം", "महीने का आपका शुभ दिन"]),
                    at: e.root.t,
                    words: match lang {
                        Lang::En => format!("Your own day of the month comes on {}: begin what matters; the month's letter is written that day.", date_words(e.root.t, lon)),
                        Lang::Ml => format!("മാസത്തിലെ നിങ്ങളുടെ സ്വന്തം ദിവസം {}-ന് വരും: പ്രധാനപ്പെട്ടത് തുടങ്ങുക; ഈ മാസത്തെ കത്ത് അന്നാണ് എഴുതുന്നത്.", dl(e.root.t)),
                        Lang::Hi => format!("महीने का आपका अपना दिन {} को आएगा: ज़रूरी काम शुरू करें; महीने की चिट्ठी उसी दिन लिखी जाती है।", dl(e.root.t)),
                    },
                    good: true,
                });
                break;
            }
        }
    }
    if let Ok(ev) = tithi_boundaries(spk, at, to40, MICROSECOND_NS) {
        let (m, s) = (
            sidereal_longitude(spk, ayanamsa, DashaLord::Moon, at).unwrap_or(0.0),
            sidereal_longitude(spk, ayanamsa, DashaLord::Sun, at).unwrap_or(0.0),
        );
        let cur = tithi(m, s).number as usize;
        let mut seen_full = false;
        let mut seen_new = false;
        for (i, e) in ev.iter().enumerate() {
            let n = (cur + i) % 30 + 1;
            if n == 15 && !seen_full {
                seen_full = true;
                let words = match lang {
                    Lang::En => format!("Pournami begins {}.", date_words(e.root.t, lon)),
                    Lang::Ml => format!("പൗർണമി {}-ന് തുടങ്ങും.", dl(e.root.t)),
                    Lang::Hi => format!("पूर्णिमा {} को शुरू होगी।", dl(e.root.t)),
                };
                v.push(Countdown { kind: "moon", title: pick(["Full Moon", "വെളുത്തവാവ്", "पूर्णिमा"]), at: e.root.t, words, good: true });
            }
            if n == 30 && !seen_new {
                seen_new = true;
                let words = match lang {
                    Lang::En => format!("Amavasya begins {}: a day for the ancestors; begin nothing new.", date_words(e.root.t, lon)),
                    Lang::Ml => format!("അമാവാസി {}-ന് തുടങ്ങും: പിതൃക്കളെ ഓർക്കാനുള്ള ദിവസം; പുതുതായി ഒന്നും തുടങ്ങരുത്.", dl(e.root.t)),
                    Lang::Hi => format!("अमावस्या {} को शुरू होगी: पितरों को याद करने का दिन; कुछ नया शुरू न करें।", dl(e.root.t)),
                };
                v.push(Countdown { kind: "moon", title: pick(["New Moon", "കറുത്തവാവ്", "अमावस्या"]), at: e.root.t, words, good: false });
            }
        }
    }
    // Returns of Jupiter and Saturn within the horizon.
    for g in [DashaLord::Jupiter, DashaLord::Saturn] {
        if let Some(natal_lon) = sidereal_longitude(spk, ayanamsa, g, natal.birth) {
            let to = at.add_ns(if g == DashaLord::Jupiter { 13 } else { 31 } * 365 * NS_PER_DAY).min(horizon);
            if let Some(t) = returns(spk, ayanamsa, g, natal_lon, at, to).first() {
                let age = (t.ns_since(natal.birth)) as f64 / (365.25 * NS_PER_DAY as f64);
                v.push(Countdown {
                    kind: "return",
                    title: if g == DashaLord::Jupiter {
                        pick(["Your twelve-year turning point", "നിങ്ങളുടെ പന്ത്രണ്ടു വർഷത്തെ വഴിത്തിരിവ്", "आपका बारह साल का मोड़"])
                    } else {
                        pick(["Your thirty-year reckoning", "നിങ്ങളുടെ മുപ്പതു വർഷത്തെ കണക്കെടുപ്പ്", "आपका तीस साल का हिसाब"])
                    },
                    at: *t,
                    words: match (lang, g == DashaLord::Jupiter) {
                        (Lang::En, true) => format!("Around {} (age {:.0}) a twelve-year cycle of growth completes and a new one begins: a good year to start what you want to grow.", date_words(*t, lon), age),
                        (Lang::En, false) => format!("Around {} (age {:.0}) a thirty-year cycle closes: life asks you to grow up in one area, career, marriage or home. Effort made then lasts for decades.", date_words(*t, lon), age),
                        (Lang::Ml, true) => format!("{}-നോടടുത്ത് ({:.0} വയസ്സിൽ) വളർച്ചയുടെ പന്ത്രണ്ടു വർഷത്തെ ഒരു ചക്രം പൂർത്തിയായി പുതിയത് തുടങ്ങും: വളരണമെന്ന് ആഗ്രഹിക്കുന്നത് തുടങ്ങാൻ നല്ല വർഷം.", dl(*t), age),
                        (Lang::Ml, false) => format!("{}-നോടടുത്ത് ({:.0} വയസ്സിൽ) മുപ്പതു വർഷത്തെ ഒരു ചക്രം അടയും: ജോലി, വിവാഹം, വീട് ഇവയിലൊന്നിൽ പക്വത നേടാൻ ജീവിതം ആവശ്യപ്പെടും. അപ്പോൾ ചെയ്യുന്ന അധ്വാനം പതിറ്റാണ്ടുകളോളം നിലനിൽക്കും.", dl(*t), age),
                        (_, true) => format!("{} के आसपास ({:.0} की उम्र में) तरक्की का बारह साल का एक चक्र पूरा होकर नया शुरू होगा: जो बढ़ाना चाहते हैं, उसे शुरू करने का अच्छा साल।", dl(*t), age),
                        (_, false) => format!("{} के आसपास ({:.0} की उम्र में) तीस साल का एक चक्र पूरा होगा: ज़िंदगी काम, शादी या घर में से किसी एक में परिपक्व होने को कहेगी। तब की गई मेहनत दशकों तक टिकती है।", dl(*t), age),
                    },
                    good: g == DashaLord::Jupiter,
                });
            }
        }
    }
    v.retain(|c| c.at > at);
    v.sort_by(|a, b| a.at.cmp(&b.at));
    v
}

fn ordinal_year(n: i64) -> String {
    let s = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// An observance of the coming days.
#[derive(Clone, Debug, PartialEq)]
pub struct Festival {
    /// Name.
    pub name: &'static str,
    /// When it begins (the tithi's start, the ingress, the star's entry).
    pub at: Instant,
    /// When it ends, for a tithi.
    pub end: Option<Instant>,
    /// What to do.
    pub what: &'static str,
    /// `tithi`, `sun`, `star`.
    pub kind: &'static str,
}

/// The observances in the next `days`: Ekadashi, Pradosham, Pournami, Amavasya,
/// Sankashti Chaturthi, masik Shivaratri, Shukla Shashthi, Durga Ashtami, the Sun's
/// sankranti, and the Moon's return to the birth star.
pub fn festivals(spk: &Spk, natal: &Natal, ayanamsa: Ayanamsa, at: Instant, days: i64) -> Vec<Festival> {
    festivals_in(crate::plain::Lang::En, spk, natal, ayanamsa, at, days)
}

/// [`festivals`] with the names and the plain words in `lang`.
pub fn festivals_in(lang: crate::plain::Lang, spk: &Spk, natal: &Natal, ayanamsa: Ayanamsa, at: Instant, days: i64) -> Vec<Festival> {
    let to = at.add_ns(days as i128 * NS_PER_DAY);
    let mut v = Vec::new();
    if let Ok(ev) = tithi_boundaries(spk, at, to, MICROSECOND_NS) {
        let (m, s) = (
            sidereal_longitude(spk, ayanamsa, DashaLord::Moon, at).unwrap_or(0.0),
            sidereal_longitude(spk, ayanamsa, DashaLord::Sun, at).unwrap_or(0.0),
        );
        let cur = tithi(m, s).number as usize;
        for (i, e) in ev.iter().enumerate() {
            let n = (cur + i) % 30 + 1;
            let end = ev.get(i + 1).map(|x| x.root.t);
            let f = |name: [&'static str; 3], what: [&'static str; 3]| Festival { name: lang.pick(name), at: e.root.t, end, what: lang.pick(what), kind: "tithi" };
            match n {
                6 => v.push(f(
                    ["Shukla Shashthi", "ഷഷ്ഠി", "स्कंद षष्ठी"],
                    [
                        "Subrahmanya's day: a fast or a light meal, and courage spent for someone.",
                        "സുബ്രഹ്മണ്യന്റെ ദിവസം: ഒരു വ്രതം അല്ലെങ്കിൽ ലഘുഭക്ഷണം, ഒപ്പം ആർക്കെങ്കിലും വേണ്ടി കാണിക്കുന്ന ധൈര്യം.",
                        "सुब्रह्मण्य का दिन: व्रत या हल्का भोजन, और किसी के लिए दिखाई गई हिम्मत।",
                    ],
                )),
                8 => v.push(f(
                    ["Durga Ashtami", "ദുർഗാഷ്ടമി", "दुर्गा अष्टमी"],
                    [
                        "Durga's day: feed people outside your circle; it clears fear and confusion.",
                        "ദുർഗയുടെ ദിവസം: നിങ്ങളുടെ വേണ്ടപ്പെട്ടവർക്ക് പുറത്തുള്ളവർക്ക് ഭക്ഷണം കൊടുക്കുക; ഭയവും ആശയക്കുഴപ്പവും മാറും.",
                        "दुर्गा का दिन: अपने दायरे से बाहर के लोगों को भोजन कराएँ; डर और उलझन दूर होती है।",
                    ],
                )),
                11 => v.push(f(
                    ["Shukla Ekadashi", "വെളുത്ത പക്ഷ ഏകാദശി", "शुक्ल एकादशी"],
                    [
                        "Vishnu's fast: no grain, the mind kept light, a debt forgiven.",
                        "വിഷ്ണുവിനായുള്ള വ്രതം: അരിയാഹാരം ഇല്ല, മനസ്സ് ലഘുവായി, ഒരു കടം പൊറുത്തുകൊടുക്കുക.",
                        "विष्णु का व्रत: अन्न नहीं, मन हल्का, और किसी का एक कर्ज़ माफ़।",
                    ],
                )),
                13 => v.push(f(
                    ["Pradosham", "പ്രദോഷം", "प्रदोष व्रत"],
                    [
                        "The hour before sunset is Shiva's: light a lamp and make an apology; it eases delays.",
                        "സൂര്യാസ്തമയത്തിന് മുമ്പുള്ള നേരം ശിവന്റേതാണ്: ഒരു വിളക്ക് കൊളുത്തുക, ഒരു ക്ഷമ ചോദിക്കുക; താമസങ്ങൾ അയയും.",
                        "सूर्यास्त से पहले का समय शिव का है: दीया जलाएँ और किसी से माफ़ी माँगें; रुकावटें कम होती हैं।",
                    ],
                )),
                15 => v.push(f(
                    ["Pournami", "പൗർണമി", "पूर्णिमा"],
                    [
                        "Full moon: honour your mother, feed women and children, sit by water.",
                        "വെളുത്തവാവ്: അമ്മയെ ആദരിക്കുക, സ്ത്രീകൾക്കും കുട്ടികൾക്കും ഭക്ഷണം കൊടുക്കുക, വെള്ളത്തിനരികെ ഇരിക്കുക.",
                        "पूरा चाँद: अपनी माँ का सम्मान करें, स्त्रियों और बच्चों को भोजन कराएँ, पानी के पास बैठें।",
                    ],
                )),
                19 => v.push(f(
                    ["Sankashti Chaturthi", "സങ്കടഹര ചതുർത്ഥി", "संकष्टी चतुर्थी"],
                    [
                        "Ganapati's day after moonrise: finish what was left undone; obstacles ease.",
                        "ചന്ദ്രോദയത്തിന് ശേഷം ഗണപതിയുടെ നേരം: ബാക്കിവച്ചത് പൂർത്തിയാക്കുക; തടസ്സങ്ങൾ നീങ്ങും.",
                        "चाँद निकलने के बाद गणेश जी का समय: अधूरा काम पूरा करें; अड़चनें कम होती हैं।",
                    ],
                )),
                26 => v.push(f(
                    ["Krishna Ekadashi", "കറുത്ത പക്ഷ ഏകാദശി", "कृष्ण एकादशी"],
                    [
                        "Vishnu's fast: no grain, the mind kept light.",
                        "വിഷ്ണുവിനായുള്ള വ്രതം: അരിയാഹാരം ഇല്ല, മനസ്സ് ലഘുവായി.",
                        "विष्णु का व्रत: अन्न नहीं, मन हल्का।",
                    ],
                )),
                28 => v.push(f(
                    ["Pradosham", "പ്രദോഷം", "प्रदोष व्रत"],
                    [
                        "The hour before sunset is Shiva's: light a lamp; it eases delays.",
                        "സൂര്യാസ്തമയത്തിന് മുമ്പുള്ള നേരം ശിവന്റേതാണ്: ഒരു വിളക്ക് കൊളുത്തുക; താമസങ്ങൾ അയയും.",
                        "सूर्यास्त से पहले का समय शिव का है: दीया जलाएँ; रुकावटें कम होती हैं।",
                    ],
                )),
                29 => v.push(f(
                    ["Masik Shivaratri", "മാസശിവരാത്രി", "मासिक शिवरात्रि"],
                    [
                        "The month's night of Shiva: a lamp kept through the night, or the first watch.",
                        "മാസത്തിലെ ശിവരാത്രി: രാത്രി മുഴുവൻ, അല്ലെങ്കിൽ ആദ്യയാമം വരെ, ഒരു വിളക്ക്.",
                        "महीने की शिवरात्रि: रात भर, या कम से कम पहले पहर तक, एक दीया।",
                    ],
                )),
                30 => v.push(f(
                    ["Amavasya", "കറുത്തവാവ്", "अमावस्या"],
                    [
                        "The ancestors' day: feed crows first, give food in their name, begin nothing new.",
                        "പിതൃക്കളുടെ ദിവസം: ആദ്യം കാക്കയ്ക്ക് ചോറ് കൊടുക്കുക, അവരുടെ പേരിൽ അന്നദാനം ചെയ്യുക, പുതിയതൊന്നും തുടങ്ങരുത്.",
                        "पितरों का दिन: पहले कौओं को खिलाएँ, उनके नाम पर अन्न दान करें, कुछ नया शुरू न करें।",
                    ],
                )),
                _ => {}
            }
        }
    }
    if let Some(c) = next_sign_change(spk, ayanamsa, DashaLord::Sun, at) {
        if c.at <= to {
            v.push(Festival {
                name: lang.pick(["Sankranti", "സംക്രമം", "संक्रांति"]),
                at: c.at,
                end: None,
                what: lang.pick([
                    "The solar month turns: the hours around it are for giving, not beginning.",
                    "മാസം മാറുന്നു: ആ നേരത്തിന് ചുറ്റുമുള്ള മണിക്കൂറുകൾ ദാനത്തിനുള്ളതാണ്, തുടക്കങ്ങൾക്കല്ല.",
                    "सौर महीना बदलता है: उसके आसपास के घंटे दान के लिए हैं, शुरुआत के लिए नहीं।",
                ]),
                kind: "sun",
            });
        }
    }
    if let Ok(ev) = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, to, MICROSECOND_NS) {
        let cur = sidereal_longitude(spk, ayanamsa, DashaLord::Moon, at).map(|l| nakshatra(l).index).unwrap_or(0);
        for (i, e) in ev.iter().enumerate() {
            if (cur + 1 + i) % 27 == natal.janma() {
                v.push(Festival {
                    name: lang.pick(["Your own day", "നിങ്ങളുടെ സ്വന്തം നാൾ", "आपका अपना दिन"]),
                    at: e.root.t,
                    end: ev.get(i + 1).map(|x| x.root.t),
                    what: lang.pick([
                        "Your lucky day of the month: begin what matters, read the month's letter, visit your family deity.",
                        "മാസത്തിലെ നിങ്ങളുടെ ഭാഗ്യദിനം: പ്രധാനപ്പെട്ടത് തുടങ്ങുക, മാസത്തെ കത്ത് വായിക്കുക, കുടുംബക്ഷേത്രത്തിൽ പോവുക.",
                        "महीने का आपका शुभ दिन: जो ज़रूरी है वह शुरू करें, महीने की चिट्ठी पढ़ें, अपने कुलदेवता के दर्शन करें।",
                    ]),
                    kind: "star",
                });
            }
        }
    }
    v.sort_by(|a, b| a.at.cmp(&b.at));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::moment;
    use kshana_houses::HouseSystem;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_wall_and_the_feed_are_ordered_and_full() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let t0 = std::time::Instant::now();
        let w = countdowns(&spk, None, &natal, &obs, Ayanamsa::Lahiri, at);
        eprintln!("countdowns in {:?}", t0.elapsed());
        assert!(w.len() >= 8, "{}", w.len());
        assert!(w.windows(2).all(|x| x[0].at <= x[1].at));
        assert!(w.iter().all(|c| c.at > at && !c.words.is_empty()));
        assert!(w.iter().any(|c| c.kind == "saturn"));
        assert!(w.iter().any(|c| c.kind == "star"));
        assert!(w.iter().any(|c| c.kind == "year"));
        let f = festivals(&spk, &natal, Ayanamsa::Lahiri, at, 35);
        assert!(f.iter().any(|x| x.name.contains("Ekadashi")));
        assert!(f.iter().any(|x| x.name == "Amavasya") && f.iter().any(|x| x.name == "Pournami"));
        assert!(f.iter().any(|x| x.name == "Your own day"));
        assert!(f.windows(2).all(|x| x[0].at <= x[1].at));
        let m = moment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
        let g = next_good(&m, at, 0.6);
        assert_eq!(g.len(), 9);
        assert!(g.iter().flatten().all(|x| x.end > at && x.fit >= 0.6));
        assert!(!m.day_title.is_empty());
    }
}
