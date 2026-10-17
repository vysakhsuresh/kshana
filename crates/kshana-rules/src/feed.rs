//! The day as a feed and the wall of countdowns: the next good minute for every life,
//! the exact instants the person is waiting for (Saturn's exit, the period's turn,
//! Jupiter's next house, the next sade sati, the year's close, the birth star's
//! return, the lunar days), and the observances of the coming month, each solved on
//! the kernel. Everything here is what a person checks again and again.

use crate::briefing::saturn_sentence;
use crate::forecast::{date_words, Life, Moment, Natal, SIGN_NAMES};
use crate::gochara::{favourable_from_moon, SaturnPhase, TARA_NAMES};
use crate::transit::{kernel_end, next_sign_change, returns, sidereal_longitude, sign_changes};
use kshana_ephem::{Body, Spk};
use kshana_events::{nakshatra_boundaries, solar_return, tithi_boundaries, MICROSECOND_NS};
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY};
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::panchanga::{NAKSHATRA_NAMES, TITHI_NAMES};
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
pub fn day_title(tara: usize, t: &Tithi, day_lord: DashaLord) -> String {
    let lord = match day_lord {
        DashaLord::Sun => "the Sun's day",
        DashaLord::Moon => "the Moon's day",
        DashaLord::Mars => "Mars's day",
        DashaLord::Mercury => "Mercury's day",
        DashaLord::Jupiter => "Jupiter's day",
        DashaLord::Venus => "Venus's day",
        DashaLord::Saturn => "Saturn's day",
        _ => "the day",
    };
    format!(
        "{} {}, {}",
        TARA_NAMES[tara.min(8)],
        TITHI_NAMES[((t.number - 1) % 15) as usize],
        lord
    )
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
    let _ = eop;
    let lon = obs.longitude_deg;
    let horizon = kernel_end(spk);
    let mut v: Vec<Countdown> = Vec::new();
    let ms = natal.moon_sign();
    let house_of = |sign: u8| ((sign as i64 - ms).rem_euclid(12) + 1) as u8;

    // Saturn: the end of the running phase, or the start of the next.
    let sat_sign = sidereal_longitude(spk, ayanamsa, DashaLord::Saturn, at)
        .map(|l| ((l / 30.0).floor() as u8) % 12)
        .unwrap_or(0);
    let phase = SaturnPhase::from_house(house_of(sat_sign));
    let (words, until) = saturn_sentence(spk, ayanamsa, natal, phase, at, lon);
    if let Some(t) = until {
        v.push(Countdown {
            kind: "saturn",
            title: if phase == SaturnPhase::Clear { "Saturn moves".into() } else { "Saturn's trial ends".into() },
            at: t,
            words,
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
                title: "Next sade sati begins".into(),
                at: c.at,
                words: format!("Saturn enters {}, the 12th from your Moon, on {}: the next sade sati opens.", SIGN_NAMES[c.to as usize], date_words(c.at, lon)),
                good: false,
            });
        }
    }
    // The periods.
    let levels = vimshottari_levels(natal.moon_sidereal, natal.birth, YEAR_365_25_NS, at, 3);
    for (i, p) in levels.iter().enumerate().take(3) {
        let name = ["mahadasha", "antardasha", "pratyantar"][i];
        let idx = DashaLord::ORDER.iter().position(|l| *l == p.lord).unwrap_or(0);
        let next = DashaLord::ORDER[(idx + 1) % 9];
        v.push(Countdown {
            kind: "dasha",
            title: format!("{:?} {name} ends", p.lord),
            at: p.end,
            words: format!("{:?} {name} runs until {}; {next:?} follows.", p.lord, date_words(p.end, lon)),
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
                title: format!("{g:?} enters {}", SIGN_NAMES[c.to as usize]),
                at: c.at,
                words: format!(
                    "{g:?} enters {}, the {} from your Moon, on {}: {}.",
                    SIGN_NAMES[c.to as usize],
                    crate::forecast::ordinal(h),
                    date_words(c.at, lon),
                    if fav { "a giving house" } else { "a house to work through" }
                ),
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
                    title: "Your solar return".into(),
                    at: r,
                    words: format!("The Sun returns to its birth place on {}: your {} year opens.", date_words(r, lon), ordinal_year(age + 1)),
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
                    title: format!("Moon returns to {}", NAKSHATRA_NAMES[natal.janma()]),
                    at: e.root.t,
                    words: format!("Your birth star comes round on {}: a day to begin what matters, and the month's letter is written.", date_words(e.root.t, lon)),
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
                v.push(Countdown { kind: "moon", title: "Full Moon".into(), at: e.root.t, words: format!("Pournami begins {}.", date_words(e.root.t, lon)), good: true });
            }
            if n == 30 && !seen_new {
                seen_new = true;
                v.push(Countdown { kind: "moon", title: "New Moon".into(), at: e.root.t, words: format!("Amavasya begins {}: the ancestors' day.", date_words(e.root.t, lon)), good: false });
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
                    title: format!("{g:?} return"),
                    at: *t,
                    words: format!("{g:?} stands again where it stood at your birth on {} (age {:.1}).", date_words(*t, lon), age),
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
            let f = |name: &'static str, what: &'static str| Festival { name, at: e.root.t, end, what, kind: "tithi" };
            match n {
                6 => v.push(f("Shukla Shashthi", "Subrahmanya's day: a fast or a light meal, and courage spent for someone.")),
                8 => v.push(f("Durga Ashtami", "Durga's day: Rahu's remedies land best; feed those outside the circle.")),
                11 => v.push(f("Shukla Ekadashi", "Vishnu's fast: no grain, the mind kept light, a debt forgiven.")),
                13 => v.push(f("Pradosham", "The hour before sunset is Shiva's: a lamp, Saturn's remedies, an apology made.")),
                15 => v.push(f("Pournami", "Full Moon: the mother and the mind; feed women and children, sit by water.")),
                19 => v.push(f("Sankashti Chaturthi", "Ganapati's day after moonrise: Ketu's remedies; finish what was left undone.")),
                26 => v.push(f("Krishna Ekadashi", "Vishnu's fast: no grain, the mind kept light.")),
                28 => v.push(f("Pradosham", "The hour before sunset is Shiva's: a lamp, Saturn's remedies.")),
                29 => v.push(f("Masik Shivaratri", "The month's night of Shiva: a lamp kept through the night, or the first watch.")),
                30 => v.push(f("Amavasya", "The ancestors' day: feed crows first, give food in their name, begin nothing new.")),
                _ => {}
            }
        }
    }
    if let Some(c) = next_sign_change(spk, ayanamsa, DashaLord::Sun, at) {
        if c.at <= to {
            v.push(Festival {
                name: "Sankranti",
                at: c.at,
                end: None,
                what: "The Sun changes sign: the sixteen ghatis around it are for giving, not beginning.",
                kind: "sun",
            });
        }
    }
    if let Ok(ev) = nakshatra_boundaries(spk, Body::Moon, ayanamsa, at, to, MICROSECOND_NS) {
        let cur = sidereal_longitude(spk, ayanamsa, DashaLord::Moon, at).map(|l| nakshatra(l).index).unwrap_or(0);
        for (i, e) in ev.iter().enumerate() {
            if (cur + 1 + i) % 27 == natal.janma() {
                v.push(Festival {
                    name: "Your birth star",
                    at: e.root.t,
                    end: ev.get(i + 1).map(|x| x.root.t),
                    what: "The Moon in your own star: a day for beginnings, the monthly letter, and your own deity.",
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
        assert!(f.iter().any(|x| x.name == "Your birth star"));
        assert!(f.windows(2).all(|x| x[0].at <= x[1].at));
        let m = moment(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
        let g = next_good(&m, at, 0.6);
        assert_eq!(g.len(), 9);
        assert!(g.iter().flatten().all(|x| x.end > at && x.fit >= 0.6));
        assert!(!m.day_title.is_empty());
    }
}
