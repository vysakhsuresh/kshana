//! Why a life stands where it stands, who causes it, when it eases and what eases it.
//!
//! The forecast gives a life a score built from named components. This module opens
//! every component to its grahas: each running period lord, each transiting graha,
//! the Moon's star, the planetary hour and the limbs of the day become a *cause* with
//! a signed pull on the score and the exact instant it ends. For every cause that
//! ends within two years the engine recomputes the component with the graha moved on
//! (the next lord, the next sign, the next star, the next hour) and reports the exact
//! gain, so "what eases it" is a solved number, not a guess. A monthly slow series
//! (birth chart, periods, transits of the eight slow grahas) gives the turning points:
//! when the life first climbs a band, when it peaks, when it dips.

use crate::forecast::{
    abhijit_window, dasha_items, date_words, gochara_items, hora_component, kala_component,
    moment_in, natal_component, sky, slot_window, slow_scores, tara_component, weekday,
    Band, ForecastError, Hora, Life, Moment, Natal, GULIKA, YAMAGANDA,
};
use crate::gochara::{hora_lord, hora_words, tara_index, TARA_SCORE};
use crate::lang as tl;
use crate::plain::{cap, Lang};
use crate::transit::next_sign_change;
use kshana_ephem::{Body, Spk};
use kshana_events::{hora_at, nakshatra_boundaries, rahu_kala, solar_day, MICROSECOND_NS};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY, NS_PER_SEC};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{nakshatra, vimshottari_levels, Ayanamsa, DashaLord};

/// Something that pulls a life's score up or down right now.
#[derive(Clone, Debug, PartialEq)]
pub struct Cause {
    /// The graha behind it, when there is one.
    pub planet: Option<DashaLord>,
    /// `natal`, `dasha`, `gochara`, `tara`, `hora` or `kala`.
    pub kind: &'static str,
    /// What it is, in words.
    pub what: String,
    /// Its signed share of the score, in `[-0.5, 0.5]` (×100 for points).
    pub pull: f64,
    /// When it ends, if it ends.
    pub until: Option<Instant>,
    /// The end in words, with a date.
    pub until_words: String,
    /// Rule id.
    pub rule_id: &'static str,
}

/// A coming change and its exact effect on the score.
#[derive(Clone, Debug, PartialEq)]
pub struct Relief {
    /// When.
    pub at: Instant,
    /// What changes, in words.
    pub what: String,
    /// Signed change of the life score, in `[-1, 1]` (×100 for points).
    pub gain: f64,
    /// The graha that moves.
    pub planet: Option<DashaLord>,
    /// `dasha`, `gochara`, `tara`, `hora` or `kala`.
    pub kind: &'static str,
    /// True for a change that sets a season rather than a day: a mahadasha, antardasha
    /// or pratyantar boundary, or a sign change of any graha but the Moon.
    pub lasting: bool,
}

/// A turning point of the slow series.
#[derive(Clone, Debug, PartialEq)]
pub struct Turning {
    /// When.
    pub at: Instant,
    /// Slow score there, `[0, 1]`.
    pub score: f64,
    /// Band there.
    pub band: Band,
    /// In words, with the date.
    pub words: String,
}

/// The whole explanation of one life at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Explanation {
    /// Which life.
    pub life: Life,
    /// The instant.
    pub at: Instant,
    /// The life score now, `[0, 1]`.
    pub score: f64,
    /// Its band.
    pub band: Band,
    /// Two or three plain sentences with dates.
    pub headline: String,
    /// What drags, hardest first.
    pub causes: Vec<Cause>,
    /// What helps, strongest first.
    pub helpers: Vec<Cause>,
    /// Coming changes within two years, chronological.
    pub relief: Vec<Relief>,
    /// The first month the slow score climbs a band.
    pub eases: Option<Turning>,
    /// The best month of the next three years.
    pub peak: Option<Turning>,
    /// The first month the slow score drops a band.
    pub toughens: Option<Turning>,
    /// The slow score now.
    pub slow_now: f64,
    /// Thirty-six monthly slow samples from `at`.
    pub slow: Vec<(Instant, f64)>,
}

fn life_index(life: Life) -> usize {
    Life::ALL.iter().position(|l| *l == life).unwrap_or(0)
}

fn points(x: f64) -> String {
    let p = (x * 100.0).round() as i64;
    if p >= 0 {
        format!("+{p}")
    } else {
        format!("{p}")
    }
}

/// Pick the sentence for `lang`; each arm is built only when chosen.
macro_rules! say {
    ($lang:expr, $en:expr, $ml:expr, $hi:expr $(,)?) => {
        match $lang {
            Lang::En => $en,
            Lang::Ml => $ml,
            Lang::Hi => $hi,
        }
    };
}

/// A period level as a plain noun (see `plain::level_noun`).
fn level_noun(lang: Lang, level: usize) -> &'static str {
    let t = match level {
        0 => ["long chapter of your life", "ജീവിതത്തിലെ നീണ്ട അധ്യായം", "जीवन का लंबा अध्याय"],
        1 => ["stretch inside it", "അതിനുള്ളിലെ ഘട്ടം", "उसके भीतर का दौर"],
        2 => ["season of a few months", "ഏതാനും മാസത്തെ കാലം", "कुछ महीनों का समय"],
        3 => ["run of a few weeks", "ഏതാനും ആഴ്ചത്തെ ഘട്ടം", "कुछ हफ़्तों का दौर"],
        _ => ["run of a few days", "ഏതാനും ദിവസത്തെ ഘട്ടം", "कुछ दिनों का दौर"],
    };
    lang.pick(t)
}

/// How long a passing influence lasts. English gives the bare span ("a few weeks", the
/// caller adds "for"); Malayalam and Hindi give the whole phrase ("for a few weeks").
fn influence_span(lang: Lang, l: DashaLord) -> &'static str {
    if lang == Lang::En {
        return crate::plain::influence_span(l);
    }
    let t = match l {
        DashaLord::Moon => ["", "രണ്ടുമൂന്നു ദിവസത്തേക്ക്", "दो-एक दिन के लिए"],
        DashaLord::Sun | DashaLord::Mercury | DashaLord::Venus => ["", "ഏതാനും ആഴ്ചകളിലേക്ക്", "कुछ हफ़्तों के लिए"],
        DashaLord::Mars => ["", "ഏകദേശം ആറാഴ്ചത്തേക്ക്", "लगभग छह हफ़्तों के लिए"],
        DashaLord::Jupiter => ["", "ഏകദേശം ഒരു വർഷത്തേക്ക്", "लगभग एक साल के लिए"],
        DashaLord::Saturn => ["", "രണ്ടു വർഷത്തിലേറെ", "दो साल से ज़्यादा के लिए"],
        DashaLord::Rahu | DashaLord::Ketu => ["", "ഏകദേശം പതിനെട്ടു മാസത്തേക്ക്", "लगभग अठारह महीनों के लिए"],
    };
    lang.pick(t)
}

/// What the person was born with for a part of life (see `plain::birth_base`).
fn birth_base(lang: Lang, name: &str, score: f64) -> String {
    match lang {
        Lang::En => crate::plain::birth_base(name, score),
        Lang::Ml if score >= 0.6 => format!("{name} കാര്യത്തിൽ ജന്മനാ നിങ്ങൾക്ക് നല്ല കരുത്തുണ്ട്: മോശം സമയങ്ങളിൽ നിന്ന് അത് നന്നായി കരകയറും"),
        Lang::Ml if score >= 0.45 => format!("{name} കാര്യത്തിൽ ജന്മനാ ശരാശരി അടിത്തറയാണ്: ഇടുന്ന അധ്വാനത്തിനനുസരിച്ച് ഫലം വരും"),
        Lang::Ml => format!("{name} ജന്മനാ ഒരു ദുർബല ഭാഗമാണ്: ജീവിതകാലം മുഴുവൻ സ്ഥിരമായ ശ്രദ്ധ വേണം"),
        Lang::Hi if score >= 0.6 => format!("{name} में आप जन्म से ही मज़बूत हैं: बुरे दौर से यह अच्छी तरह उबरता है"),
        Lang::Hi if score >= 0.45 => format!("{name} में जन्म से औसत आधार है: जितनी मेहनत, उतना फल"),
        Lang::Hi => format!("{name} जन्म से एक कमज़ोर पक्ष है: इसे जीवन भर लगातार देखभाल चाहिए"),
    }
}

/// "is good now", said of a life in Malayalam or Hindi.
fn band_state(lang: Lang, b: Band) -> &'static str {
    let t = match b {
        Band::Bright => ["bright", "തിളക്കത്തിലാണ്", "चमकते दौर में है"],
        Band::Good => ["good", "നല്ല നിലയിലാണ്", "अच्छे दौर में है"],
        Band::Steady => ["steady", "സ്ഥിരമായ നിലയിലാണ്", "स्थिर दौर में है"],
        Band::Slow => ["slow", "മന്ദഗതിയിലാണ്", "धीमे दौर में है"],
        Band::Hold => ["hold", "കാത്തിരിപ്പിന്റെ നിലയിലാണ്", "ठहराव के दौर में है"],
    };
    lang.pick(t)
}

/// The band a life moves into: "നല്ല നിലയിലേക്ക്", "अच्छे दौर में".
fn band_to(lang: Lang, b: Band) -> &'static str {
    let t = match b {
        Band::Bright => ["bright", "തിളക്കമുള്ള നിലയിലേക്ക്", "चमकते दौर में"],
        Band::Good => ["good", "നല്ല നിലയിലേക്ക്", "अच्छे दौर में"],
        Band::Steady => ["steady", "സ്ഥിരമായ നിലയിലേക്ക്", "स्थिर दौर में"],
        Band::Slow => ["slow", "മന്ദമായ നിലയിലേക്ക്", "धीमे दौर में"],
        Band::Hold => ["hold", "കാത്തിരിപ്പിന്റെ നിലയിലേക്ക്", "ठहराव के दौर में"],
    };
    lang.pick(t)
}

/// Explain `life` at `at` for a person born as `natal` describes, standing at `obs`.
#[allow(clippy::too_many_arguments)]
pub fn explain(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    at: Instant,
) -> Result<Explanation, ForecastError> {
    explain_in(Lang::En, spk, eop, natal, obs, system, ayanamsa, life, at)
}

/// [`explain`] with every sentence in `lang`.
#[allow(clippy::too_many_arguments)]
pub fn explain_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    life: Life,
    at: Instant,
) -> Result<Explanation, ForecastError> {
    let idx = life_index(life);
    // The life inside a sentence: lower-case English, the plain name otherwise.
    let name = tl::life_lower(lang, life);
    let lname = tl::life_name(lang, life);
    let lon = obs.longitude_deg;
    let date_words = |t: Instant, lon: f64| -> String {
        match lang {
            Lang::En => date_words(t, lon),
            _ => tl::date(lang, t, lon),
        }
    };
    let chapter_theme = |l: DashaLord| tl::chapter_theme(lang, l);
    let support = |l: DashaLord| tl::support(lang, l);
    let pressure = |l: DashaLord| tl::pressure(lang, l);
    let long_test = |p: crate::gochara::SaturnPhase| tl::long_test(lang, p);
    let day_quality = |t: usize| tl::day_quality(lang, t);
    let hour_nature = |l: DashaLord| tl::hour_nature(lang, l);
    let m: Moment = moment_in(lang, spk, eop, natal, obs, system, ayanamsa, at)?;
    let ls = &m.lives[idx];
    let sky_now = sky(spk, eop, natal, obs, system, ayanamsa, at)?;
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 5);
    let two_years = at.add_ns(730 * NS_PER_DAY);
    let total_weight: f64 = ls.components.iter().map(|c| c.weight).sum();
    let norm = |w: f64| if total_weight > 0.0 { w / total_weight } else { w };

    let mut causes: Vec<Cause> = Vec::new();
    let mut relief: Vec<Relief> = Vec::new();

    // 1. The birth chart.
    let n = natal_component(natal, life);
    causes.push(Cause {
        planet: Some(natal.facts.cusp_sub_lords[(life.area().primary - 1) as usize]),
        kind: "natal",
        what: birth_base(lang, &name, n.score),
        pull: norm(n.weight) * (n.score - 0.5),
        until: None,
        until_words: lang
            .pick([
                "for life: every season builds on it, and the remedies work on it",
                "ജീവിതകാലം മുഴുവൻ: ഓരോ കാലവും ഇതിന്മേലാണ് പണിയുന്നത്, പരിഹാരങ്ങൾ ഇതിലാണ് പ്രവർത്തിക്കുന്നത്",
                "जीवन भर: हर दौर इसी पर टिकता है, और उपाय इसी पर काम करते हैं",
            ])
            .into(),
        rule_id: "KS-N-0001",
    });

    // 2. The running periods, level by level. When several levels end at the same
    // instant (a deeper level always ends with its parent) the change is one event.
    let items = dasha_items(natal, life, &levels);
    let lw: f64 = items.iter().map(|i| i.weight).sum();
    let mut ends_seen: Vec<Instant> = Vec::new();
    for it in &items {
        let share = if lw > 0.0 { it.weight / lw } else { 0.0 };
        causes.push(Cause {
            planet: Some(it.lord),
            kind: "dasha",
            what: {
                let ln = level_noun(lang, it.level);
                let th = chapter_theme(it.lord);
                let x = if it.score >= 0.5 { support(it.lord) } else { pressure(it.lord) };
                say!(
                    lang,
                    format!("the {ln} (about {th}) brings {x} to your {name}"),
                    format!("{ln} ({th}) നിങ്ങളുടെ {name} കാര്യത്തിൽ {x} കൊണ്ടുവരുന്നു"),
                    format!("{ln} ({th} से जुड़ा) {name} के मामले में {x} लाता है"),
                )
            },
            pull: norm(0.30) * share * (it.score - 0.5),
            until: Some(it.end),
            until_words: {
                let ln = level_noun(lang, it.level);
                let d = date_words(it.end, lon);
                say!(
                    lang,
                    format!("this {ln} ends on {d}"),
                    format!("ഈ {ln} {d}-ന് അവസാനിക്കും"),
                    format!("यह {ln} {d} को ख़त्म होगा"),
                )
            },
            rule_id: "KS-F-0001",
        });
        if it.end <= two_years && !ends_seen.contains(&it.end) {
            ends_seen.push(it.end);
            // The whole chain after this level ends: the exact dasha component then.
            let after = it.end.add_ns(NS_PER_SEC);
            let next_levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, after, 5);
            let now_c = dasha_component_score(natal, life, &levels);
            let then_c = dasha_component_score(natal, life, &next_levels);
            let next_lord = next_levels.get(it.level).map(|p| p.lord);
            if let Some(nl) = next_lord {
                let gain = norm(0.30) * (then_c - now_c);
                let (s, _houses) = crate::forecast::lord_score(natal, life, nl);
                relief.push(Relief {
                    at: it.end,
                    what: {
                        let ln = level_noun(lang, it.level);
                        let th = chapter_theme(nl);
                        let (sp, pr) = (support(nl), pressure(nl));
                        let tail = if gain >= 0.01 {
                            say!(
                                lang,
                                format!("better for your {name}, with {sp}"),
                                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് നല്ലത്; {sp} ഒപ്പമുണ്ടാകും"),
                                format!("{name} के लिए बेहतर, {sp} के साथ"),
                            )
                        } else if gain <= -0.01 {
                            say!(
                                lang,
                                format!("harder for your {name}, with {pr}"),
                                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് കുറച്ചുകൂടി കഠിനം; {pr} വരാം"),
                                format!("{name} के लिए कठिन, {pr} के साथ"),
                            )
                        } else if s >= 0.5 {
                            say!(
                                lang,
                                format!("about the same for your {name}, with {sp}"),
                                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് ഏകദേശം ഇതുപോലെ തന്നെ; {sp} ഒപ്പമുണ്ടാകും"),
                                format!("{name} के लिए लगभग वैसा ही, {sp} के साथ"),
                            )
                        } else {
                            say!(
                                lang,
                                format!("about the same for your {name}; watch for {pr}"),
                                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് ഏകദേശം ഇതുപോലെ തന്നെ; {pr} ശ്രദ്ധിക്കുക"),
                                format!("{name} के लिए लगभग वैसा ही; {pr} से सावधान रहें"),
                            )
                        };
                        say!(
                            lang,
                            format!("a new {ln} begins, about {th}: {tail}"),
                            format!("പുതിയൊരു ഘട്ടം തുടങ്ങുന്നു ({ln}), {th} എന്നിവയുടെ കാലം: {tail}"),
                            format!("नया दौर शुरू होता है ({ln}), {th} से जुड़ा: {tail}"),
                        )
                    },
                    gain,
                    planet: Some(nl),
                    kind: "dasha",
                    lasting: it.level <= 2,
                });
            }
        }
    }

    // 3. The transits, graha by graha.
    let signs = sky_now.signs();
    let (gitems, den) = gochara_items(natal, &sky_now.from_moon, &signs, life);
    let g_now = gochara_score(&gitems, den);
    for (i, it) in gitems.iter().enumerate() {
        if it.contrib == 0.0 && it.relevance < 0.5 {
            continue;
        }
        let pull = if den > 0.0 { norm(0.20) * 0.5 * it.contrib / den } else { 0.0 };
        let change = next_sign_change(spk, ayanamsa, it.graha, at);
        let (until, until_words) = match change {
            Some(c) => {
                let d = date_words(c.at, lon);
                (Some(c.at), say!(lang, format!("until {d}"), format!("{d} വരെ"), format!("{d} तक")))
            }
            None => (None, lang.pick(["for longer than two years", "രണ്ടു വർഷത്തിലേറെ", "दो साल से ज़्यादा"]).to_string()),
        };
        let span = influence_span(lang, it.graha);
        let what_now = if it.saturn_phase != crate::gochara::SaturnPhase::Clear {
            let lt = long_test(it.saturn_phase);
            say!(
                lang,
                format!("{lt} for you, touching your {name}"),
                format!("നിങ്ങൾക്ക് {lt}; ഇത് {name} കാര്യത്തെയും തൊടുന്നു"),
                format!("आपके लिए {lt}, जिसका असर {name} पर भी है"),
            )
        } else if it.favourable && it.obstructed {
            say!(
                lang,
                format!("help for your {name} is on its way but partly blocked"),
                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് സഹായം വരുന്നുണ്ട്, പക്ഷേ ഭാഗികമായി തടസ്സപ്പെട്ടിരിക്കുന്നു"),
                format!("{name} के लिए मदद रास्ते में है, पर कुछ रुकावट के साथ"),
            )
        } else if it.favourable || pull > 0.0 {
            let sp = support(it.graha);
            say!(
                lang,
                format!("{sp} for your {name}, for {span}"),
                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് {sp}, {span}"),
                format!("{name} के लिए {sp}, {span}"),
            )
        } else if it.adverse || pull < 0.0 {
            let pr = pressure(it.graha);
            say!(
                lang,
                format!("{pr} around your {name}, for {span}"),
                format!("നിങ്ങളുടെ {name} കാര്യത്തിൽ {pr}, {span}"),
                format!("{name} के मामले में {pr}, {span}"),
            )
        } else {
            say!(
                lang,
                format!("a neutral spell for your {name}, for {span}"),
                format!("നിങ്ങളുടെ {name} കാര്യത്തിന് ഗുണവും ദോഷവുമില്ലാത്ത കാലം, {span}"),
                format!("{name} के लिए न अच्छा न बुरा समय, {span}"),
            )
        };
        if pull.abs() >= 0.002 || it.graha == DashaLord::Saturn || it.graha == DashaLord::Jupiter {
            causes.push(Cause {
                planet: Some(it.graha),
                kind: "gochara",
                what: what_now,
                pull,
                until,
                until_words,
                rule_id: "KS-G-0001",
            });
        }
        if let Some(c) = change {
            if c.at <= two_years {
                let mut fm = sky_now.from_moon;
                let mut sg = signs;
                fm[i] = ((c.to as i64 - natal.moon_sign()).rem_euclid(12) + 1) as u8;
                sg[i] = c.to;
                let (then_items, then_den) = gochara_items(natal, &fm, &sg, life);
                let g_then = gochara_score(&then_items, then_den);
                let gain = norm(0.20) * (g_then - g_now);
                let then_it = &then_items[i];
                let what = if it.saturn_phase != crate::gochara::SaturnPhase::Clear
                    && then_it.saturn_phase == crate::gochara::SaturnPhase::Clear
                {
                    let lt = long_test(it.saturn_phase);
                    say!(lang, format!("{} ends", cap(lt)), format!("{lt} അവസാനിക്കുന്നു"), format!("{lt}: ख़त्म"))
                } else if then_it.saturn_phase != crate::gochara::SaturnPhase::Clear && it.saturn_phase == crate::gochara::SaturnPhase::Clear {
                    let lt = long_test(then_it.saturn_phase);
                    say!(lang, format!("{} begins", cap(lt)), format!("{lt} തുടങ്ങുന്നു"), format!("{lt}: शुरू"))
                } else if then_it.saturn_phase != crate::gochara::SaturnPhase::Clear {
                    let lt = long_test(then_it.saturn_phase);
                    say!(
                        lang,
                        format!("{} goes on, in a new form", cap(lt)),
                        format!("{lt} പുതിയ രൂപത്തിൽ തുടരുന്നു"),
                        format!("{lt}: नए रूप में जारी"),
                    )
                } else if then_it.favourable && then_it.obstructed {
                    say!(
                        lang,
                        format!("help for your {name} arrives, though partly blocked, for {span}"),
                        format!("നിങ്ങളുടെ {name} കാര്യത്തിന് സഹായം എത്തുന്നു, ഭാഗികമായ തടസ്സത്തോടെ, {span}"),
                        format!("{name} के लिए मदद आती है, कुछ रुकावट के साथ, {span}"),
                    )
                } else if then_it.favourable {
                    let sp = support(it.graha);
                    say!(
                        lang,
                        format!("{} arrives for your {name}, for {span}", cap(sp)),
                        format!("നിങ്ങളുടെ {name} കാര്യത്തിന് {sp} എത്തുന്നു, {span}"),
                        format!("{name} के लिए {sp} का समय शुरू, {span}"),
                    )
                } else if then_it.adverse {
                    let pr = pressure(it.graha);
                    say!(
                        lang,
                        format!("{} set in around your {name}, for {span}", cap(pr)),
                        format!("നിങ്ങളുടെ {name} കാര്യത്തിൽ {pr} തുടങ്ങുന്നു, {span}"),
                        format!("{name} के मामले में {pr} का समय शुरू, {span}"),
                    )
                } else if gain > 0.0 {
                    say!(
                        lang,
                        format!("a pressure on your {name} lifts"),
                        format!("നിങ്ങളുടെ {name} കാര്യത്തിലെ ഒരു ഭാരം നീങ്ങുന്നു"),
                        format!("{name} पर से एक दबाव हटता है"),
                    )
                } else {
                    say!(
                        lang,
                        format!("a help to your {name} fades"),
                        format!("നിങ്ങളുടെ {name} കാര്യത്തിനുള്ള ഒരു സഹായം മങ്ങുന്നു"),
                        format!("{name} को मिल रही एक मदद कम होती है"),
                    )
                };
                if gain.abs() >= 0.005 || it.graha == DashaLord::Saturn {
                    relief.push(Relief {
                        at: c.at,
                        what,
                        gain,
                        planet: Some(it.graha),
                        kind: "gochara",
                        lasting: it.graha != DashaLord::Moon,
                    });
                }
            }
        }
    }

    // 4. Tara: the Moon's star counted from the birth star.
    let (t_now, tara_c) = tara_component(natal, &sky_now);
    let star_end = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, at.add_ns(2 * NS_PER_DAY), MICROSECOND_NS)
        .ok()
        .and_then(|v| v.first().map(|e| e.root.t));
    let cur_star = nakshatra(sky_now.moon_sidereal).index;
    let next_star = (cur_star + 1) % 27;
    let t_next = tara_index(natal.janma(), next_star);
    causes.push(Cause {
        planet: Some(DashaLord::Moon),
        kind: "tara",
        what: {
            let q = day_quality(t_now);
            say!(lang, format!("today is {q}"), format!("ഇന്ന് {q}"), format!("आज {q}"))
        },
        pull: norm(tara_c.weight) * (tara_c.score - 0.5),
        until: star_end,
        until_words: match star_end {
            Some(e) => {
                let d = date_words(e, lon);
                say!(
                    lang,
                    format!("until the day's mood turns, {d}"),
                    format!("ദിവസത്തിന്റെ നില മാറുന്നതുവരെ, {d}"),
                    format!("दिन का मिज़ाज बदलने तक, {d}"),
                )
            }
            None => String::new(),
        },
        rule_id: "KS-T-0001",
    });
    if let Some(e) = star_end {
        relief.push(Relief {
            at: e,
            what: {
                let q = day_quality(t_next);
                say!(
                    lang,
                    format!("the day's mood turns: {q}"),
                    format!("ദിവസത്തിന്റെ നില മാറുന്നു: {q}"),
                    format!("दिन का मिज़ाज बदलता है: {q}"),
                )
            },
            gain: norm(tara_c.weight) * (TARA_SCORE[t_next] - TARA_SCORE[t_now]),
            planet: Some(DashaLord::Moon),
            kind: "tara",
            lasting: false,
        });
    }

    // 4b. The life's own timing: what passes through its own houses in the birth chart.
    if let Some(bc) = ls.components.iter().find(|c| c.key == "bhava") {
        causes.push(Cause {
            planet: None,
            kind: "bhava",
            what: bc.note.clone(),
            pull: norm(bc.weight) * (bc.score - 0.5),
            until: None,
            until_words: String::new(),
            rule_id: bc.rule_id,
        });
    }

    // 5. The planetary hour and the limbs of the day.
    let day = solar_day(spk, eop, obs, at).ok().flatten();
    if let (Some(h), Some(d)) = (&m.hora, &day) {
        let hc = hora_component(natal, life, h);
        causes.push(Cause {
            planet: Some(h.lord),
            kind: "hora",
            what: {
                let hn = hour_nature(h.lord);
                if hc.score < 0.5 {
                    say!(
                        lang,
                        format!("this hour does not favour your {name}: it is {hn}"),
                        format!("ഈ മണിക്കൂർ നിങ്ങളുടെ {name} കാര്യത്തിന് അനുകൂലമല്ല: ഇത് {hn}"),
                        format!("यह घंटा {name} के लिए अनुकूल नहीं: यह {hn}"),
                    )
                } else {
                    say!(
                        lang,
                        format!("this hour helps your {name}: it is {hn}"),
                        format!("ഈ മണിക്കൂർ നിങ്ങളുടെ {name} കാര്യത്തിന് സഹായമാണ്: ഇത് {hn}"),
                        format!("यह घंटा {name} के लिए मददगार है: यह {hn}"),
                    )
                }
            },
            pull: norm(hc.weight) * (hc.score - 0.5),
            until: Some(h.end),
            until_words: lang.pick(["until the hour turns", "ഈ മണിക്കൂർ മാറുന്നതുവരെ", "घंटा बदलने तक"]).into(),
            rule_id: "KS-H-0001",
        });
        if h.end < d.next_sunrise {
            let (nname, ns, ne) = hora_at(d, obs, h.end.add_ns(NS_PER_SEC));
            if let Some(nl) = hora_lord(nname) {
                let nh = Hora {
                    lord: nl,
                    start: ns,
                    end: ne,
                    words: hora_words(nl),
                };
                let nc = hora_component(natal, life, &nh);
                relief.push(Relief {
                    at: h.end,
                    what: {
                        let hn = hour_nature(nl);
                        say!(lang, format!("the next hour is {hn}"), format!("അടുത്ത മണിക്കൂർ: {hn}"), format!("अगला घंटा: {hn}"))
                    },
                    gain: norm(hc.weight) * (nc.score - hc.score),
                    planet: Some(nl),
                    kind: "hora",
                    lasting: false,
                });
            }
        }
    }
    let kc = kala_component(&sky_now, day.as_ref(), obs, at);
    let mut kala_until = None;
    let mut kala_words = String::new();
    if let Some(d) = &day {
        let wd = weekday(d, obs) as usize;
        let windows = [
            (lang.pick(["the unlucky stretch", "നിർഭാഗ്യ സമയം", "अशुभ समय"]), rahu_kala(d, obs)),
            (lang.pick(["the risky stretch", "ജാഗ്രതാ സമയം", "जोखिम का समय"]), slot_window(d, YAMAGANDA[wd])),
            (lang.pick(["the quiet stretch", "ശാന്തമായ സമയം", "शांत समय"]), slot_window(d, GULIKA[wd])),
            (lang.pick(["the lucky minutes", "ഭാഗ്യ നിമിഷങ്ങൾ", "शुभ घड़ी"]), abhijit_window(d)),
        ];
        for (label, (s, e)) in windows {
            if at >= s && at < e {
                kala_until = Some(e);
                kala_words = say!(
                    lang,
                    format!("until {label} closes"),
                    format!("{label} കഴിയുന്നതുവരെ"),
                    format!("{label} ख़त्म होने तक"),
                );
                let after = kala_component(&sky_now, Some(d), obs, e.add_ns(NS_PER_SEC));
                relief.push(Relief {
                    at: e,
                    what: say!(lang, format!("{label} closes"), format!("{label} കഴിയുന്നു"), format!("{label} ख़त्म")),
                    gain: norm(kc.weight) * (after.score - kc.score),
                    planet: None,
                    kind: "kala",
                    lasting: false,
                });
            }
        }
    }
    causes.push(Cause {
        planet: None,
        kind: "kala",
        what: if kc.score < 0.45 {
            lang.pick([
                "an unlucky stretch of the day is running: do not start anything new in it",
                "ദിവസത്തിലെ ഒരു നിർഭാഗ്യ സമയമാണ് ഇപ്പോൾ: ഇതിൽ പുതിയതൊന്നും തുടങ്ങരുത്",
                "दिन का एक अशुभ समय चल रहा है: इसमें कुछ नया शुरू न करें",
            ])
            .to_string()
        } else if kc.score > 0.55 {
            lang.pick([
                "the lucky minutes of the day are running: a good time to begin",
                "ദിവസത്തിലെ ഭാഗ്യ നിമിഷങ്ങളാണ് ഇപ്പോൾ: തുടങ്ങാൻ നല്ല സമയം",
                "दिन की शुभ घड़ी चल रही है: शुरुआत के लिए अच्छा समय",
            ])
            .to_string()
        } else {
            lang.pick([
                "no special stretch of the day is running",
                "ദിവസത്തിലെ പ്രത്യേക സമയമൊന്നും ഇപ്പോൾ ഇല്ല",
                "दिन का कोई ख़ास समय अभी नहीं चल रहा",
            ])
            .to_string()
        },
        pull: norm(kc.weight) * (kc.score - 0.5),
        until: kala_until,
        until_words: kala_words,
        rule_id: "KS-M-0001",
    });

    // Order: what drags, what helps.
    let mut helpers: Vec<Cause> = causes.iter().filter(|c| c.pull > 0.0).cloned().collect();
    helpers.sort_by(|a, b| b.pull.partial_cmp(&a.pull).unwrap_or(std::cmp::Ordering::Equal));
    causes.retain(|c| c.pull <= 0.0);
    causes.sort_by(|a, b| a.pull.partial_cmp(&b.pull).unwrap_or(std::cmp::Ordering::Equal));
    relief.retain(|r| r.at > at);
    relief.sort_by(|a, b| a.at.cmp(&b.at));

    // The slow series: thirty-six months.
    let slow_now = slow_scores(natal, &sky_now, &levels[..levels.len().min(3)])[idx];
    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let slow_all = crate::forecast::slow_series(spk, eop, natal, obs, ayanamsa, at.add_ns(month), month, 36)?;
    let slow: Vec<(Instant, f64)> = slow_all.iter().map(|(t, v)| (*t, v[idx])).collect();
    // The season is judged against the better of the slow band and the band the person
    // sees now, so "climbs" always names a band above what is on the screen.
    let band_now = Band::of(slow_now).max(ls.band);
    let band_floor = Band::of(slow_now).min(ls.band);
    let turning = |t: Instant, s: f64, up: bool| {
        let d = date_words(t, lon);
        let b = Band::of(s);
        Turning {
            at: t,
            score: s,
            band: b,
            words: match (lang, up) {
                (Lang::En, _) => format!("{} {} to {} from {}", life.name(), if up { "climbs" } else { "slips" }, b.word(), d),
                (Lang::Ml, true) => format!("{lname}: {d} മുതൽ {} ഉയരും", band_to(lang, b)),
                (Lang::Ml, false) => format!("{lname}: {d} മുതൽ {} താഴും", band_to(lang, b)),
                (Lang::Hi, true) => format!("{lname}: {d} से ऊपर उठकर {}", band_to(lang, b)),
                (Lang::Hi, false) => format!("{lname}: {d} से नीचे आकर {}", band_to(lang, b)),
            },
        }
    };
    let eases = slow
        .iter()
        .find(|(_, s)| Band::of(*s) > band_now)
        .map(|&(t, s)| turning(t, s, true));
    let toughens = slow
        .iter()
        .find(|(_, s)| Band::of(*s) < band_floor)
        .map(|&(t, s)| turning(t, s, false));
    let peak = slow
        .iter()
        .fold(None, |acc: Option<(Instant, f64)>, &(t, s)| match acc {
            Some((_, bs)) if bs >= s => acc,
            _ => Some((t, s)),
        })
        .filter(|(_, s)| *s > slow_now + 0.02)
        .map(|(t, s)| Turning {
            at: t,
            score: s,
            band: Band::of(s),
            words: {
                let d = date_words(t, lon);
                let n = format!("{:.0}", s * 100.0);
                say!(
                    lang,
                    format!("{} peaks around {} at {}", life.name(), d, n),
                    format!("{lname}: ഏറ്റവും നല്ല സമയം {d} ഓടെ ({n})"),
                    format!("{lname}: सबसे अच्छा समय {d} के आसपास ({n})"),
                )
            },
        });

    // Words.
    let mut headline = String::new();
    let lasting = |c: &Cause| matches!(c.kind, "natal" | "dasha" | "gochara");
    // "Love is good now" / "പ്രണയം ഇപ്പോൾ നല്ല നിലയിലാണ്" / "प्रेम अभी अच्छे दौर में है".
    let is_now = match lang {
        Lang::En => format!("{} is {} now", crate::plain::subject(life.name()), ls.band.word()),
        Lang::Ml => format!("{lname} ഇപ്പോൾ {}", band_state(lang, ls.band)),
        Lang::Hi => format!("{lname} अभी {}", band_state(lang, ls.band)),
    };
    let stop = if lang == Lang::Hi { '।' } else { '.' };
    if ls.band >= Band::Good {
        let top = helpers.iter().find(|c| lasting(c)).or(helpers.first());
        headline.push_str(&is_now);
        if let Some(t) = top {
            headline.push_str(&format!(": {}", t.what));
        }
        headline.push(stop);
        if let Some(t) = &toughens {
            let d = date_words(t.at, lon);
            headline.push_str(&say!(
                lang,
                format!(" Make the most of it; it slips to {} from {}.", t.band.word(), d),
                format!(" ഈ സമയം പരമാവധി ഉപയോഗിക്കുക; {d} മുതൽ {} താഴും.", band_to(lang, t.band)),
                format!(" इसका पूरा फ़ायदा उठाएँ; {d} से यह {} आ जाएगा।", band_to(lang, t.band)),
            ));
        } else if let Some(p) = &peak {
            let d = date_words(p.at, lon);
            headline.push_str(&say!(
                lang,
                format!(" It peaks around {d}."),
                format!(" {d} ഓടെ ഇത് ഏറ്റവും ഉയരത്തിലെത്തും."),
                format!(" {d} के आसपास यह सबसे ऊँचे स्तर पर होगा।"),
            ));
        }
    } else {
        let drags: Vec<&Cause> = causes.iter().filter(|c| lasting(c)).take(2).collect();
        headline.push_str(&is_now);
        if !drags.is_empty() {
            let list = drags.iter().map(|c| c.what.as_str()).collect::<Vec<_>>();
            headline.push_str(&say!(
                lang,
                format!(" because {}", list.join(", and ")),
                format!(", കാരണം {}", list.join("; കൂടാതെ ")),
                format!(", क्योंकि {}", list.join(", और ")),
            ));
        }
        headline.push(stop);
        if let Some(r) = relief.iter().find(|r| r.gain >= 0.02 && r.lasting) {
            let d = date_words(r.at, lon);
            let p = points(r.gain);
            headline.push_str(&say!(
                lang,
                format!(" The first real lift comes on {}: {} ({} points).", d, r.what, p),
                format!(" ആദ്യത്തെ യഥാർഥ ഉയർച്ച {d}-ന്: {} ({p} പോയിന്റ്).", r.what),
                format!(" पहला असली सुधार {d} को: {} ({p} अंक)।", r.what),
            ));
        }
        if let Some(e) = &eases {
            let d = date_words(e.at, lon);
            headline.push_str(&say!(
                lang,
                format!(" {} climbs to {} from {}.", life.name(), e.band.word(), d),
                format!(" {d} മുതൽ {lname} {} ഉയരും.", band_to(lang, e.band)),
                format!(" {d} से {lname} का समय ऊपर उठकर {} आएगा।", band_to(lang, e.band)),
            ));
        } else if ls.band <= Band::Slow {
            headline.push_str(lang.pick([
                " The slow stretch holds through the next three years; the remedies are the lever.",
                " ഈ മന്ദകാലം അടുത്ത മൂന്നു വർഷവും തുടരും; പരിഹാരങ്ങളാണ് ഇവിടെ വഴി.",
                " यह धीमा दौर अगले तीन साल चलेगा; उपाय ही असली सहारा हैं।",
            ]));
        }
    }

    Ok(Explanation {
        life,
        at,
        score: ls.score,
        band: ls.band,
        headline,
        causes,
        helpers,
        relief,
        eases,
        peak,
        toughens,
        slow_now,
        slow,
    })
}

fn dasha_component_score(natal: &Natal, life: Life, levels: &[kshana_vedic::DashaPeriod]) -> f64 {
    let items = dasha_items(natal, life, levels);
    let den: f64 = items.iter().map(|i| i.weight).sum();
    if den <= 0.0 {
        return 0.5;
    }
    (items.iter().map(|i| i.weight * i.score).sum::<f64>() / den).clamp(0.0, 1.0)
}

fn gochara_score(items: &[crate::forecast::GocharaItem], den: f64) -> f64 {
    let num: f64 = items.iter().map(|i| i.contrib).sum();
    let raw = if den > 0.0 { num / den } else { 0.0 };
    (0.5 + 0.5 * raw).clamp(0.0, 1.0)
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
    fn every_cause_has_words_and_the_relief_is_exact_and_ordered() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let e = explain(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Love, at).unwrap();
        assert!(!e.headline.is_empty());
        assert!(e.causes.len() + e.helpers.len() >= 8, "{} {}", e.causes.len(), e.helpers.len());
        for c in e.causes.iter().chain(e.helpers.iter()) {
            assert!(!c.what.is_empty());
            assert!(c.pull.abs() <= 0.5);
        }
        assert!(e.causes.iter().all(|c| c.pull <= 0.0));
        assert!(e.helpers.iter().all(|c| c.pull > 0.0));
        // the pulls sum to the score minus one half
        let total: f64 = e.causes.iter().chain(e.helpers.iter()).map(|c| c.pull).sum();
        assert!((total - (e.score - 0.5)).abs() < 0.02, "{total} vs {}", e.score - 0.5);
        assert!(!e.relief.is_empty());
        assert!(e.relief.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(e.relief.iter().all(|r| r.at > at && r.gain.abs() <= 1.0));
        // Saturn's next sign change is always reported
        assert!(e.relief.iter().any(|r| r.planet == Some(DashaLord::Saturn)));
        assert_eq!(e.slow.len(), 36);
        assert!((0.0..=1.0).contains(&e.slow_now));
        use crate::plain::jargon;
        for c in e.causes.iter().chain(e.helpers.iter()) {
            assert_eq!(jargon(&c.what), None, "{}", c.what);
            assert_eq!(jargon(&c.until_words), None, "{}", c.until_words);
        }
        for r in &e.relief {
            assert_eq!(jargon(&r.what), None, "{}", r.what);
        }
        for t in [&e.eases, &e.peak, &e.toughens].into_iter().flatten() {
            assert_eq!(jargon(&t.words), None, "{}", t.words);
        }
    }

    #[test]
    fn the_explanation_reads_plain_in_malayalam_and_hindi() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let en = explain(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Life::Money, at).unwrap();
        for lang in [Lang::Ml, Lang::Hi] {
            for life in [Life::Love, Life::Money, Life::Health] {
                let e = explain_in(lang, &spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, life, at).unwrap();
                let mut texts = vec![e.headline.clone()];
                for c in e.causes.iter().chain(e.helpers.iter()) {
                    texts.push(c.what.clone());
                    texts.push(c.until_words.clone());
                }
                texts.extend(e.relief.iter().map(|r| r.what.clone()));
                texts.extend([&e.eases, &e.peak, &e.toughens].into_iter().flatten().map(|t| t.words.clone()));
                for s in texts {
                    assert_eq!(crate::plain::jargon_in(lang, &s), None, "{lang:?}: {s}");
                    assert!(!s.contains("your") && !s.contains(" the "), "{lang:?} English left: {s}");
                }
            }
        }
        // the English path is unchanged by the language plumbing
        assert!(en.headline.starts_with("Money is "));
    }
}
