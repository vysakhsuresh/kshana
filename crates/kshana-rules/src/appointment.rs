//! "I have something at this time. How is that time for me?"
//!
//! A purpose names the lives it touches and how much (an interview is career first,
//! money second); the forecast of the chosen instant gives each life its score; the
//! weighted sum is the reading. The limbs of the day are a gate: inside Rahu kala,
//! Yamaganda or Gulika the reading is capped at "shift it", inside Abhijit it is
//! lifted a little (Muhurta Chintamani). Then the engine looks for a better hour the
//! same day (by the planetary hours, outside the cautions) and a better day in the
//! next fortnight (by the daily series), so the person is never told only "no".

use crate::forecast::{date_words, moment, moment_in, series, Band, ForecastError, Hora, Life, Moment};
use crate::gochara::SaturnPhase;
use crate::plain::Lang;
use kshana_ephem::Spk;
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::{Ayanamsa, DashaLord};

/// What the appointment is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Purpose {
    Meeting,
    Interview,
    Exam,
    Journey,
    Deal,
    Doctor,
    Date,
    Family,
    Property,
    Launch,
    Anything,
}

impl Purpose {
    /// All, in display order.
    pub const ALL: [Purpose; 11] = [
        Purpose::Meeting,
        Purpose::Interview,
        Purpose::Exam,
        Purpose::Journey,
        Purpose::Deal,
        Purpose::Doctor,
        Purpose::Date,
        Purpose::Family,
        Purpose::Property,
        Purpose::Launch,
        Purpose::Anything,
    ];

    /// Stable key.
    pub fn key(self) -> &'static str {
        match self {
            Purpose::Meeting => "meeting",
            Purpose::Interview => "interview",
            Purpose::Exam => "exam",
            Purpose::Journey => "journey",
            Purpose::Deal => "deal",
            Purpose::Doctor => "doctor",
            Purpose::Date => "date",
            Purpose::Family => "family",
            Purpose::Property => "property",
            Purpose::Launch => "launch",
            Purpose::Anything => "anything",
        }
    }

    /// Parse a key.
    pub fn parse(s: &str) -> Option<Purpose> {
        Purpose::ALL.iter().copied().find(|p| p.key() == s)
    }

    /// The words on the chip.
    pub fn words(self) -> &'static str {
        match self {
            Purpose::Meeting => "Meeting someone",
            Purpose::Interview => "Interview, pitch",
            Purpose::Exam => "Exam, test",
            Purpose::Journey => "Journey",
            Purpose::Deal => "Deal, signing, purchase",
            Purpose::Doctor => "Doctor, treatment",
            Purpose::Date => "Date, proposal",
            Purpose::Family => "Family matter",
            Purpose::Property => "Home, land, rent",
            Purpose::Launch => "Starting something new",
            Purpose::Anything => "Anything at all",
        }
    }

    /// The purpose as a phrase inside a sentence: "the sky carries {phrase}".
    pub fn phrase(self) -> &'static str {
        match self {
            Purpose::Meeting => "a meeting",
            Purpose::Interview => "an interview or a pitch",
            Purpose::Exam => "an exam",
            Purpose::Journey => "a journey",
            Purpose::Deal => "a deal or a signing",
            Purpose::Doctor => "a visit to the doctor",
            Purpose::Date => "a date or a proposal",
            Purpose::Family => "a family matter",
            Purpose::Property => "a home or land matter",
            Purpose::Launch => "a new start",
            Purpose::Anything => "what you have planned",
        }
    }

    /// The words on the chip, in `lang`.
    pub fn words_in(self, lang: Lang) -> &'static str {
        let t = match self {
            Purpose::Meeting => ["Meeting someone", "ഒരാളെ കാണാൻ", "किसी से मिलना"],
            Purpose::Interview => ["Interview, pitch", "ഇന്റർവ്യൂ", "इंटरव्यू"],
            Purpose::Exam => ["Exam, test", "പരീക്ഷ", "परीक्षा"],
            Purpose::Journey => ["Journey", "യാത്ര", "यात्रा"],
            Purpose::Deal => ["Deal, signing, purchase", "കരാർ, ഒപ്പ്, വാങ്ങൽ", "सौदा, हस्ताक्षर, खरीद"],
            Purpose::Doctor => ["Doctor, treatment", "ഡോക്ടർ, ചികിത്സ", "डॉक्टर, इलाज"],
            Purpose::Date => ["Date, proposal", "പ്രണയം, വിവാഹാലോചന", "मुलाक़ात, प्रस्ताव"],
            Purpose::Family => ["Family matter", "കുടുംബകാര്യം", "पारिवारिक बात"],
            Purpose::Property => ["Home, land, rent", "വീട്, ഭൂമി, വാടക", "घर, ज़मीन, किराया"],
            Purpose::Launch => ["Starting something new", "പുതിയ തുടക്കം", "नई शुरुआत"],
            Purpose::Anything => ["Anything at all", "എന്തും", "कुछ भी"],
        };
        lang.pick(t)
    }

    /// The purpose as a phrase inside a sentence, in `lang`.
    pub fn phrase_in(self, lang: Lang) -> &'static str {
        let t = match self {
            Purpose::Meeting => ["a meeting", "ഈ കൂടിക്കാഴ്ച", "यह मुलाक़ात"],
            Purpose::Interview => ["an interview or a pitch", "ഇന്റർവ്യൂവോ അവതരണമോ", "इंटरव्यू या प्रस्तुति"],
            Purpose::Exam => ["an exam", "പരീക്ഷ", "परीक्षा"],
            Purpose::Journey => ["a journey", "യാത്ര", "यात्रा"],
            Purpose::Deal => ["a deal or a signing", "ഇടപാടോ ഒപ്പിടലോ", "सौदा या हस्ताक्षर"],
            Purpose::Doctor => ["a visit to the doctor", "ഡോക്ടറെ കാണുന്നത്", "डॉक्टर से मिलना"],
            Purpose::Date => ["a date or a proposal", "പ്രിയപ്പെട്ടയാളുമായുള്ള കൂടിക്കാഴ്ചയോ ആലോചനയോ", "मुलाक़ात या प्रस्ताव"],
            Purpose::Family => ["a family matter", "കുടുംബകാര്യം", "पारिवारिक मामला"],
            Purpose::Property => ["a home or land matter", "വീടിന്റെയോ ഭൂമിയുടെയോ കാര്യം", "घर या ज़मीन का मामला"],
            Purpose::Launch => ["a new start", "പുതിയ തുടക്കം", "नई शुरुआत"],
            Purpose::Anything => ["what you have planned", "നിങ്ങൾ ഉദ്ദേശിച്ച കാര്യം", "आपकी योजना"],
        };
        lang.pick(t)
    }

    /// The lives it touches and their weights (sum 1).
    pub fn lives(self) -> Vec<(Life, f64)> {
        use Life::*;
        match self {
            Purpose::Meeting => vec![(Career, 0.30), (Money, 0.20), (Love, 0.20), (Family, 0.15), (Travel, 0.15)],
            Purpose::Interview => vec![(Career, 0.70), (Money, 0.20), (Study, 0.10)],
            Purpose::Exam => vec![(Study, 0.70), (Career, 0.20), (Health, 0.10)],
            Purpose::Journey => vec![(Travel, 0.60), (Health, 0.20), (Money, 0.10), (Family, 0.10)],
            Purpose::Deal => vec![(Money, 0.50), (Career, 0.30), (Home, 0.20)],
            Purpose::Doctor => vec![(Health, 0.70), (Family, 0.15), (Money, 0.15)],
            Purpose::Date => vec![(Love, 0.70), (Family, 0.15), (Travel, 0.15)],
            Purpose::Family => vec![(Family, 0.60), (Love, 0.20), (Children, 0.20)],
            Purpose::Property => vec![(Home, 0.60), (Money, 0.30), (Family, 0.10)],
            Purpose::Launch => vec![(Career, 0.30), (Money, 0.30), (Study, 0.10), (Travel, 0.10), (Home, 0.10), (Health, 0.10)],
            Purpose::Anything => Life::ALL.iter().map(|&l| (l, 1.0 / 9.0)).collect(),
        }
    }
}

/// A better hour.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Hora lord.
    pub lord: DashaLord,
    /// Fit for the purpose, `[0, 1]`.
    pub score: f64,
}

/// The reading of one scheduled time for one purpose.
#[derive(Clone, Debug, PartialEq)]
pub struct Appointment {
    /// The purpose.
    pub purpose: Purpose,
    /// The time asked about.
    pub at: Instant,
    /// Combined score, `[0, 1]`.
    pub score: f64,
    /// Band.
    pub band: Band,
    /// `Go`, `Go, with care`, `Shift it if you can`, `Better at another time`.
    pub verdict: String,
    /// One or two sentences.
    pub headline: String,
    /// Plain lines, most useful first.
    pub advice: Vec<String>,
    /// `(life, weight, score)` for the lives the purpose touches, heaviest first.
    pub lives: Vec<(Life, f64, f64)>,
    /// The planetary hour then.
    pub hora: Option<Hora>,
    /// The caution window the time falls in, if any.
    pub caution: Option<String>,
    /// True inside Abhijit.
    pub abhijit: bool,
    /// Tara then.
    pub tara: usize,
    /// Better hours the same day, outside the cautions, best first (up to three).
    pub better_today: Vec<Slot>,
    /// A better day within a fortnight, when one clearly exists: `(midday, score)`.
    pub better_day: Option<(Instant, f64)>,
    /// The better day in words.
    pub better_day_words: String,
    /// The full forecast of the instant.
    pub moment: Moment,
}

fn verdict_words(score: f64, caution: Option<&str>) -> (&'static str, Band) {
    let b = Band::of(score);
    if caution.is_some() {
        return ("Shift it if you can", b);
    }
    (
        if score >= 0.62 {
            "Go"
        } else if score >= 0.50 {
            "Go, with care"
        } else if score >= 0.40 {
            "Shift it if you can"
        } else {
            "Better at another time"
        },
        b,
    )
}

/// A verdict (one of the English keys of `verdict_words`) in `lang`.
pub fn verdict_in(lang: Lang, verdict: &str) -> String {
    let t = match verdict {
        "Go" => ["Go", "പോകാം", "जाइए"],
        "Go, with care" => ["Go, with care", "പോകാം, ശ്രദ്ധയോടെ", "जाइए, पर सावधानी से"],
        "Shift it if you can" => ["Shift it if you can", "കഴിയുമെങ്കിൽ സമയം മാറ്റുക", "हो सके तो समय बदलें"],
        "Better at another time" => ["Better at another time", "മറ്റൊരു സമയമാണ് നല്ലത്", "किसी और समय बेहतर"],
        _ => return verdict.to_string(),
    };
    lang.pick(t).to_string()
}

/// How a life stands at a band, as the end of a sentence about that life (Malayalam and
/// Hindi only; English keeps its own wording).
fn band_state(lang: Lang, b: Band) -> &'static str {
    let t = match b {
        Band::Bright => ["bright", "നന്നായി തെളിഞ്ഞുനിൽക്കുന്നു", "बहुत अच्छी है"],
        Band::Good => ["good", "നല്ല നിലയിലാണ്", "अच्छी है"],
        Band::Steady => ["steady", "സമനിലയിലാണ്", "सामान्य है"],
        Band::Slow => ["slow", "മന്ദഗതിയിലാണ്", "धीमी है"],
        Band::Hold => ["hold", "കാത്തിരിപ്പിലാണ്", "थमी हुई है"],
    };
    lang.pick(t)
}

/// Read `at` for `purpose`.
#[allow(clippy::too_many_arguments)]
pub fn appointment(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    at: Instant,
) -> Result<Appointment, ForecastError> {
    appointment_in(Lang::En, spk, eop, natal, obs, system, ayanamsa, purpose, at)
}

/// [`appointment`] with every sentence in `lang`. The verdict is in `lang` too; the
/// caution is the moment's own window label.
#[allow(clippy::too_many_arguments)]
pub fn appointment_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    at: Instant,
) -> Result<Appointment, ForecastError> {
    if lang == Lang::En {
        return appointment_en(spk, eop, natal, obs, system, ayanamsa, purpose, at);
    }
    let m = moment_in(lang, spk, eop, natal, obs, system, ayanamsa, at)?;
    let mut a = appointment_from(spk, eop, natal, obs, ayanamsa, purpose, at, m)?;
    localise(lang, &mut a, obs.longitude_deg);
    Ok(a)
}

#[allow(clippy::too_many_arguments)]
fn appointment_en(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    at: Instant,
) -> Result<Appointment, ForecastError> {
    let m = moment(spk, eop, natal, obs, system, ayanamsa, at)?;
    appointment_from(spk, eop, natal, obs, ayanamsa, purpose, at, m)
}

/// The words of a reading in Malayalam or Hindi, rebuilt from its numbers and the
/// moment (already in `lang`).
fn localise(lang: Lang, a: &mut Appointment, lon: f64) {
    use crate::lang as l;
    let idx = |x: Life| Life::ALL.iter().position(|y| *y == x).unwrap_or(0);
    let m = &a.moment;
    let raw: f64 = a.lives.iter().map(|(_, w, s)| w * s).sum::<f64>() / a.lives.iter().map(|(_, w, _)| w).sum::<f64>();
    let top = a.lives.first().map(|(l, _, _)| *l).unwrap_or(Life::Career);
    let top_ls = &m.lives[idx(top)];
    let weak = a
        .lives
        .iter()
        .min_by(|x, y| x.2.partial_cmp(&y.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(l, _, s)| (*l, *s));
    let tn = l::life_name(lang, top);
    let st = band_state(lang, top_ls.band);
    let ph = a.purpose.phrase_in(lang);
    let pw = a.purpose.words_in(lang);
    a.caution = a.caution.as_ref().map(|c| l::window_label(lang, c));
    let ml = lang == Lang::Ml;
    a.headline = match (a.caution.as_deref(), a.verdict.as_str()) {
        (Some(c), _) => {
            if ml {
                format!("{pw}: ഈ സമയം ദിവസത്തിലെ {c} ആണ്. അപ്പോൾ തുടങ്ങിവച്ചത് തീർക്കാം, പുതിയതൊന്നും തുടങ്ങരുത്: കഴിയുമെങ്കിൽ ആ നേരം കഴിഞ്ഞ് തുടങ്ങുക, അല്ലെങ്കിൽ താഴെ കാണുന്ന ഏതെങ്കിലും സമയം എടുക്കുക.")
            } else {
                format!("{pw}: यह समय दिन के {c} में पड़ता है। उस समय शुरू किया काम निपटाएँ, नया कुछ शुरू न करें: हो सके तो वह समय बीतने के बाद शुरू करें, या नीचे दिए किसी समय को चुनें।")
            }
        }
        (None, "Go") => {
            if ml {
                format!("ധൈര്യമായി പോകൂ. ആ സമയത്ത് {tn} {st}; {ph} നന്നായി നടക്കാൻ സമയം അനുകൂലമാണ്. {}", top_ls.headline)
            } else {
                format!("बेझिझक जाइए। उस समय {tn} की स्थिति {st}, और समय {ph} के पक्ष में है। {}", top_ls.headline)
            }
        }
        (None, "Go, with care") => {
            let watch = weak
                .filter(|(_, s)| *s < 0.44)
                .map(|(w, _)| {
                    if ml {
                        format!(" {} ശ്രദ്ധിക്കുക.", l::life_name(lang, w))
                    } else {
                        format!(" {} पर ध्यान दें।", l::life_name(lang, w))
                    }
                })
                .unwrap_or_default();
            if ml {
                format!("പോകാം, ശ്രദ്ധയോടെ. അപ്പോൾ {tn} {st}; {ph} നടക്കും, പക്ഷേ പതിവിലും കൂടുതൽ തയ്യാറെടുക്കുക.{watch}")
            } else {
                format!("जाइए, पर सावधानी से। उस समय {tn} की स्थिति {st}; {ph} हो जाएगा, पर सामान्य से ज़्यादा तैयारी करें।{watch}")
            }
        }
        (None, "Shift it if you can") => {
            if ml {
                format!("കഴിയുമെങ്കിൽ സമയം മാറ്റുക. ആ സമയത്ത് {tn} {st}, {ph} ആശ്രയിക്കുന്നത് അതിനെയാണ്. താഴെയുള്ള സമയങ്ങളും ദിവസങ്ങളും ഇതിന് കൂടുതൽ നല്ലതാണ്.")
            } else {
                format!("हो सके तो समय बदल दें। उस समय {tn} की स्थिति {st}, और {ph} इसी पर टिका है। नीचे दिए समय और दिन इसके लिए बेहतर हैं।")
            }
        }
        _ => {
            if ml {
                format!("മറ്റൊരു സമയമാണ് നല്ലത്. അപ്പോൾ {tn} കാത്തിരിപ്പിലാണ്, ആ ഭാരം {ph} ചുമക്കേണ്ടിവരും. താഴെയുള്ള ഏതെങ്കിലും സമയം ഇതിന് കൂടുതൽ നല്ലതാണ്.")
            } else {
                format!("किसी और समय बेहतर रहेगा। उस समय {tn} की स्थिति थमी हुई है, और {ph} पर उसका बोझ पड़ेगा। नीचे दिया कोई समय इसके लिए बेहतर है।")
            }
        }
    };

    let mut advice: Vec<String> = Vec::new();
    if let Some(h) = &m.hora {
        let fit = top.hora_affinity(h.lord);
        let tail = if fit >= 0.7 {
            lang.pick(["", " · {}: യോജിച്ച സമയം", " · {} के लिए ठीक"]).replace("{}", tn)
        } else if fit <= 0.3 {
            lang.pick(["", " · {}: പറ്റിയ സമയമല്ല", " · {} के लिए सही समय नहीं"]).replace("{}", tn)
        } else {
            String::new()
        };
        let lead = lang.pick(["", "ആ മണിക്കൂർ: ", "वह घंटा: "]);
        advice.push(format!("{lead}{}{tail}", l::hour_nature(lang, h.lord)));
    }
    if a.abhijit {
        advice.push(lang.pick(["", "ഇത് ദിവസത്തിലെ ഭാഗ്യ നിമിഷങ്ങളിലാണ്. കൃത്യം അപ്പോൾ തുടങ്ങുക.", "यह दिन की शुभ घड़ी में पड़ता है। ठीक उसी समय शुरू करें।"]).to_string());
    }
    for w in &m.windows {
        if w.caution && w.start > a.at && w.start.ns_since(a.at) < 3 * 3_600 * kshana_time::NS_PER_SEC {
            let label = l::window_label(lang, &w.label);
            advice.push(if ml {
                format!("അധികം വൈകാതെ ദിവസത്തിലെ {label} തുടങ്ങും; അതിനുമുമ്പ് തീർക്കുക, അതിലേക്ക് നീളാൻ അനുവദിക്കരുത്.")
            } else {
                format!("थोड़ी ही देर बाद दिन का {label} शुरू होगा; उससे पहले निपटा लें, उसमें खिंचने न दें।")
            });
        }
    }
    let dq = l::day_quality(lang, m.tara);
    advice.push(if ml { format!("നിങ്ങൾക്ക് ഇത് {dq}.") } else { format!("आपके लिए यह {dq}।") });
    if m.moon_from_moon == 8 {
        advice.push(lang.pick(["", "മാസത്തിലെ നിങ്ങളുടെ ഏറ്റവും സൂക്ഷിക്കേണ്ട ദിവസമാണിത്: മനസ്സ് താഴും, തെറ്റിദ്ധാരണകൾ എളുപ്പം ഉണ്ടാകും. കാര്യങ്ങൾ ചുരുക്കവും ലളിതവുമാക്കുക.", "यह महीने का आपका सबसे नाज़ुक दिन है: मन उदास रहता है और ग़लतफ़हमियाँ आसानी से होती हैं। बात छोटी और सरल रखें।"]).to_string());
    }
    if m.saturn != SaturnPhase::Clear {
        let lt = l::long_test(lang, m.saturn);
        advice.push(if ml {
            format!("ഇപ്പോൾ നടക്കുന്നത്: {lt}. വിചാരിച്ചതിലും കൂടുതൽ സമയം എടുക്കുമെന്ന് കരുതിയിരിക്കുക.")
        } else {
            format!("अभी आपके लिए यह समय है: {lt}। काम में सोचे से ज़्यादा समय लग सकता है।")
        });
    }
    for (x, _, s) in &a.lives {
        if *s < 0.44 && *x != top {
            advice.push(format!("{}: {}", l::life_name(lang, *x), m.lives[idx(*x)].headline));
        }
    }
    if let Some((_, s)) = weak {
        if s >= 0.56 && a.lives.len() > 1 {
            advice.push(lang.pick(["", "ഇത് ബാധിക്കുന്ന എല്ലാ കാര്യങ്ങളും ആ സമയത്ത് നല്ല നിലയിലോ അതിലും മെച്ചത്തിലോ ആണ്.", "इससे जुड़े सभी पहलू उस समय अच्छे या उससे बेहतर हैं।"]).to_string());
        }
    }
    a.advice = advice;
    a.better_day_words = match a.better_day {
        Some((t, v)) => {
            let d = l::date(lang, t, lon);
            if ml {
                format!("മാറ്റാൻ കഴിയുമെങ്കിൽ: ഇതിന് {d} {:.0} കാണിക്കുന്നു; നിങ്ങൾ തിരഞ്ഞെടുത്ത സമയത്ത് {:.0} മാത്രം.", v * 100.0, raw * 100.0)
            } else {
                format!("अगर बदल सकें तो: इसके लिए {d} को {:.0} अंक मिलते हैं, आपके चुने समय पर {:.0}।", v * 100.0, raw * 100.0)
            }
        }
        None => String::new(),
    };
    a.verdict = verdict_in(lang, &a.verdict);
}

#[allow(clippy::too_many_arguments)]
fn appointment_from(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    at: Instant,
    m: Moment,
) -> Result<Appointment, ForecastError> {
    let weights = purpose.lives();
    let idx = |l: Life| Life::ALL.iter().position(|x| *x == l).unwrap_or(0);
    let mut lives: Vec<(Life, f64, f64)> = weights
        .iter()
        .map(|&(l, w)| (l, w, m.lives[idx(l)].score))
        .collect();
    lives.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let raw: f64 = lives.iter().map(|(_, w, s)| w * s).sum::<f64>() / lives.iter().map(|(_, w, _)| w).sum::<f64>();

    let caution = m
        .windows
        .iter()
        .find(|w| w.caution && at >= w.start && at < w.end)
        .map(|w| w.label.clone());
    let abhijit = m
        .windows
        .iter()
        .any(|w| !w.caution && at >= w.start && at < w.end);
    let mut score = raw;
    if caution.is_some() {
        score = score.min(0.40);
    }
    if abhijit {
        score = (score + 0.05).min(1.0);
    }
    let (verdict, band) = verdict_words(score, caution.as_deref());

    // Better hours the same day, by the planetary hour's fit for the purpose.
    let fit_of = |h: &crate::forecast::HourSlot| -> f64 {
        lives.iter().map(|(l, w, _)| w * h.fit[idx(*l)]).sum::<f64>()
            / lives.iter().map(|(_, w, _)| w).sum::<f64>()
    };
    let current_fit = m
        .hours
        .iter()
        .find(|h| at >= h.start && at < h.end)
        .map(fit_of)
        .unwrap_or(0.5);
    let mut better_today: Vec<Slot> = m
        .hours
        .iter()
        .filter(|h| !h.caution && !(at >= h.start && at < h.end))
        .map(|h| Slot {
            start: h.start,
            end: h.end,
            lord: h.lord,
            score: fit_of(h),
        })
        .filter(|s| s.score > current_fit + 0.08 || caution.is_some())
        .collect();
    better_today.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    better_today.truncate(3);
    better_today.sort_by(|a, b| a.start.cmp(&b.start));

    // A better day within a fortnight.
    let samples = series(spk, eop, natal, obs, ayanamsa, at.add_ns(NS_PER_DAY), NS_PER_DAY, 14)?;
    let day_score = |s: &crate::forecast::Sample| -> f64 {
        lives.iter().map(|(l, w, _)| w * s.scores[idx(*l)]).sum::<f64>()
            / lives.iter().map(|(_, w, _)| w).sum::<f64>()
    };
    let best_day = samples
        .iter()
        .map(|s| (s.at, day_score(s)))
        .fold(None, |acc: Option<(Instant, f64)>, (t, v)| match acc {
            Some((_, bv)) if bv >= v => acc,
            _ => Some((t, v)),
        });
    let better_day = best_day.filter(|(_, v)| *v >= raw + 0.08 || (caution.is_some() && *v >= raw));
    let better_day_words = match better_day {
        Some((t, v)) => format!(
            "If it can move: {} reads {:.0} for this, against {:.0} at the time you have.",
            date_words(t, obs.longitude_deg),
            v * 100.0,
            raw * 100.0
        ),
        None => String::new(),
    };

    // Words.
    let top = lives.first().map(|(l, _, _)| *l).unwrap_or(Life::Career);
    let top_ls = &m.lives[idx(top)];
    let weak = lives
        .iter()
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(l, _, s)| (*l, *s));
    let pw = purpose.words();
    let headline = match (caution.as_deref(), verdict) {
        (Some(c), _) => {
            let c = c.to_lowercase();
            format!(
                "{} falls inside the day's {c}. Finish things then, do not begin them: start once the {c} is over if you can, or take one of the hours below.",
                crate::plain::cap(pw)
            )
        }
        (None, "Go") => format!(
            "Go. {} is {} at that hour and the sky carries {}. {}",
            top.name(),
            top_ls.band.word(),
            purpose.phrase(),
            top_ls.headline
        ),
        (None, "Go, with care") => format!(
            "Go, with care. {} is {} then; {} will work, but prepare more than usual.{}",
            top.name(),
            top_ls.band.word(),
            purpose.phrase(),
            weak.filter(|(_, s)| *s < 0.44)
                .map(|(l, _)| format!(" Watch {}.", l.name().to_ascii_lowercase()))
                .unwrap_or_default()
        ),
        (None, "Shift it if you can") => format!(
            "Shift it if you can. {} is {} at that hour, and that is what {} leans on. The hours and days below serve it better.",
            top.name(),
            top_ls.band.word(),
            purpose.phrase()
        ),
        _ => format!(
            "Better at another time. {} is on hold then, and {} would carry that weight. One of the times below serves it better.",
            top.name(),
            purpose.phrase()
        ),
    };

    let mut advice: Vec<String> = Vec::new();
    if let Some(h) = &m.hora {
        let fit = top.hora_affinity(h.lord);
        advice.push(format!(
            "That hour is {}{}",
            crate::plain::hour_nature(h.lord),
            if fit >= 0.7 {
                format!(" · suits {}", top.name().to_ascii_lowercase())
            } else if fit <= 0.3 {
                format!(" · not an hour for {}", top.name().to_ascii_lowercase())
            } else {
                String::new()
            }
        ));
    }
    if abhijit {
        advice.push("It falls in the day's lucky minutes. Begin exactly then.".into());
    }
    for w in &m.windows {
        if w.caution && w.start > at && w.start.ns_since(at) < 3 * 3_600 * kshana_time::NS_PER_SEC {
            advice.push(format!(
                "The day's {} begins soon after; finish before it, do not let it spill in.",
                w.label.to_lowercase()
            ));
        }
    }
    advice.push(format!("For you it is {}.", crate::plain::day_quality(m.tara)));
    if m.moon_from_moon == 8 {
        advice.push("It is your most sensitive day of the month: moods run low and misunderstandings come easily. Keep it short and simple.".into());
    }
    if m.saturn != SaturnPhase::Clear {
        advice.push(format!(
            "You are in {}. Expect it to take longer than it should.",
            crate::plain::long_test(m.saturn)
        ));
    }
    for (l, _, s) in &lives {
        let ls = &m.lives[idx(*l)];
        if *s < 0.44 && *l != top {
            advice.push(format!("{}: {}", l.name(), ls.headline));
        }
    }
    if let Some((_, s)) = weak {
        if s >= 0.56 && lives.len() > 1 {
            advice.push("Every life this touches is good or better at that hour.".into());
        }
    }

    Ok(Appointment {
        purpose,
        at,
        score,
        band,
        verdict: verdict.to_string(),
        headline,
        advice,
        lives,
        hora: m.hora.clone(),
        caution,
        abhijit,
        tara: m.tara,
        better_today,
        better_day,
        better_day_words,
        moment: m,
    })
}

/// One of the best slots in the days ahead for a purpose.
#[derive(Clone, Debug, PartialEq)]
pub struct DaySlot {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Hora lord.
    pub lord: DashaLord,
    /// Combined score, `[0, 1]`: half the hour's fit, half the day's reading for the purpose.
    pub score: f64,
    /// The day's reading alone.
    pub day_score: f64,
}

/// The best `limit` hours for `purpose` in the next `days`, outside the cautions,
/// scored half by the planetary hour and half by the day.
#[allow(clippy::too_many_arguments)]
pub fn best_slots(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &crate::forecast::Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    from: Instant,
    days: usize,
    limit: usize,
) -> Result<Vec<DaySlot>, ForecastError> {
    let weights = purpose.lives();
    let idx = |l: Life| Life::ALL.iter().position(|x| *x == l).unwrap_or(0);
    let wsum: f64 = weights.iter().map(|(_, w)| w).sum();
    let samples = series(spk, eop, natal, obs, ayanamsa, from, NS_PER_DAY, days.max(1))?;
    let mut out: Vec<DaySlot> = Vec::new();
    for (d, s) in samples.iter().enumerate() {
        let day_score = weights.iter().map(|(l, w)| w * s.scores[idx(*l)]).sum::<f64>() / wsum;
        // The hours of that day: the moment at the sample instant carries them.
        let m = moment(spk, eop, natal, obs, system, ayanamsa, from.add_ns(d as i128 * NS_PER_DAY + 6 * 3_600 * kshana_time::NS_PER_SEC))?;
        for h in &m.hours {
            if h.caution || h.end <= from {
                continue;
            }
            let fit = weights.iter().map(|(l, w)| w * h.fit[idx(*l)]).sum::<f64>() / wsum;
            out.push(DaySlot {
                start: h.start.max(from),
                end: h.end,
                lord: h.lord,
                score: 0.5 * fit + 0.5 * day_score,
                day_score,
            });
        }
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(limit);
    out.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(out)
}

/// An hour that suits several people at once.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupSlot {
    /// Start.
    pub start: Instant,
    /// End.
    pub end: Instant,
    /// Combined score, `[0, 1]`: weighted towards the weakest person so no one is left out.
    pub score: f64,
    /// Each person's own score for the hour, `[0, 1]`, in the order given.
    pub each: Vec<f64>,
    /// Index of the person the hour suits best.
    pub best_for: usize,
    /// Index of the person the hour suits least.
    pub weakest_for: usize,
    /// In plain words.
    pub words: String,
}

/// The best hours for `purpose` for everyone in `people` at once, in the next `days`:
/// each person's own hour reading, combined as 0.6 × the weakest + 0.4 × the mean, so a
/// date that is great for one and poor for another ranks below one that is good for all.
#[allow(clippy::too_many_arguments)]
pub fn best_slots_together(
    spk: &Spk,
    eop: Option<&EopTable>,
    people: &[crate::forecast::Natal],
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    purpose: Purpose,
    from: Instant,
    days: usize,
    limit: usize,
    lang: crate::plain::Lang,
) -> Result<Vec<GroupSlot>, ForecastError> {
    if people.is_empty() {
        return Ok(Vec::new());
    }
    let mut each: Vec<Vec<DaySlot>> = Vec::with_capacity(people.len());
    for n in people {
        each.push(best_slots(spk, eop, n, obs, system, ayanamsa, purpose, from, days, usize::MAX)?);
    }
    let mut out: Vec<GroupSlot> = Vec::new();
    for s in &each[0] {
        let mut scores = vec![s.score];
        for other in &each[1..] {
            match other.iter().find(|o| o.start == s.start) {
                Some(o) => scores.push(o.score),
                None => break,
            }
        }
        if scores.len() != people.len() {
            continue;
        }
        let min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let best_for = scores.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|x| x.0).unwrap_or(0);
        let weakest_for = scores.iter().enumerate().min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|x| x.0).unwrap_or(0);
        let words = lang
            .pick(if min >= 0.56 {
                ["Good for everyone.", "എല്ലാവർക്കും നല്ലത്.", "सबके लिए अच्छा।"]
            } else if min >= 0.44 {
                ["Fair for everyone; no one is set back.", "എല്ലാവർക്കും തരക്കേടില്ല; ആർക്കും തിരിച്ചടിയില്ല.", "सबके लिए ठीक; किसी को नुकसान नहीं।"]
            } else {
                ["Good for most; weaker for one of you.", "മിക്കവർക്കും നല്ലത്; ഒരാൾക്ക് അൽപം ദുർബലം.", "ज़्यादातर के लिए अच्छा; एक के लिए कमज़ोर।"]
            })
            .to_string();
        out.push(GroupSlot { start: s.start, end: s.end, score: 0.6 * min + 0.4 * mean, each: scores, best_for, weakest_for, words });
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(limit);
    out.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::Natal;

    #[test]
    fn hours_together_suit_everyone() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let a = Natal::compute(&spk, None, kshana_time::Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap(), &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let b = Natal::compute(&spk, None, kshana_time::Utc::new(1997, 9, 22, 13, 12, 0, 0).unwrap().to_instant().unwrap(), &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = kshana_time::Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let s = best_slots_together(&spk, None, &[a, b], &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Purpose::ALL[0], at, 7, 5, crate::plain::Lang::En).unwrap();
        assert!(!s.is_empty() && s.len() <= 5);
        for x in &s {
            assert_eq!(x.each.len(), 2);
            assert!(x.score <= x.each.iter().cloned().fold(0.0, f64::max) + 1e-9);
            assert!(x.start >= at || x.end > at);
        }
    }
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn every_purpose_is_read_and_never_left_without_an_alternative_when_poor() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 20, 2, 0, 0, 0).unwrap().to_instant().unwrap();
        for p in Purpose::ALL {
            let a = appointment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, p, at).unwrap();
            assert!((0.0..=1.0).contains(&a.score));
            assert!(!a.verdict.is_empty() && !a.headline.is_empty());
            assert!(!a.advice.is_empty());
            for s in std::iter::once(&a.headline).chain(a.advice.iter()).chain(a.caution.iter()).chain(std::iter::once(&a.better_day_words)) {
                assert_eq!(crate::plain::jargon(s), None, "{p:?}: {s}");
            }
            let w: f64 = a.lives.iter().map(|x| x.1).sum();
            assert!((w - 1.0).abs() < 1e-9);
            if a.score < 0.50 {
                assert!(!a.better_today.is_empty() || a.better_day.is_some(), "{:?}", a.verdict);
            }
        }
        // The same in Malayalam and Hindi: no astrology words, no English left, and the
        // numbers unchanged.
        for lang in [Lang::Ml, Lang::Hi] {
            for p in Purpose::ALL {
                let en = appointment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, p, at).unwrap();
                let a = appointment_in(lang, &spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, p, at).unwrap();
                assert_eq!(a.score, en.score);
                assert_eq!(a.verdict, verdict_in(lang, &en.verdict));
                for s in std::iter::once(&a.headline).chain(a.advice.iter()).chain(std::iter::once(&a.better_day_words)).chain(std::iter::once(&a.verdict)) {
                    assert_eq!(crate::plain::jargon_in(lang, s), None, "{lang:?} {p:?}: {s}");
                }
                assert!(!a.headline.is_ascii() && a.advice.iter().all(|s| !s.is_ascii()), "{lang:?} {p:?}: {:?}", a.advice);
            }
        }
        // Rahu kala on a Tuesday in Kochi runs about 15:00–16:30 local; a time inside it is capped.
        let rk = Utc::new(2026, 10, 20, 10, 0, 0, 0).unwrap().to_instant().unwrap();
        let a = appointment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Purpose::Deal, rk).unwrap();
        assert!(a.caution.is_some(), "{:?}", a.moment.windows);
        assert!(a.score <= 0.45);
        assert_eq!(a.verdict, "Shift it if you can");
        let slots = best_slots(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, Purpose::Interview, at, 7, 3).unwrap();
        assert_eq!(slots.len(), 3);
        assert!(slots.windows(2).all(|w| w[0].start <= w[1].start));
        assert!(slots.iter().all(|s| s.start >= at && (0.0..=1.0).contains(&s.score)));
    }
}
