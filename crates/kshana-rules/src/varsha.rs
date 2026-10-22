//! The year's chapter: from one solar return to the next. The return is the exact
//! instant the Sun stands again at its natal tropical longitude, solved on the kernel;
//! the chart of that instant, cast where the person is, gives the year its rising
//! sign and its lord (convention: the lord of the return lagna, the simplest Tajika
//! choice of varshesha); twelve monthly slow samples give each life its tone for the
//! year.

use crate::forecast::{date_words, slow_series, Band, ForecastError, Life, Natal};
use crate::plain::Lang;

use kshana_chart::{compute, ChartInput};
use kshana_ephem::Spk;
use kshana_events::{solar_return, MICROSECOND_NS};
use kshana_houses::HouseSystem;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, Utc, NS_PER_DAY};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{ayanamsa_degrees, nakshatra, sign_lord, Ayanamsa, DashaLord};

/// One year of a life, birthday to birthday.
#[derive(Clone, Debug, PartialEq)]
pub struct YearChapter {
    /// The solar return that opens the year.
    pub start: Instant,
    /// The next return.
    pub end: Instant,
    /// Age completed at the start.
    pub age: u32,
    /// Rising sign of the return chart, 0 = Aries.
    pub lagna_sign: u8,
    /// The Moon's star in the return chart.
    pub moon_star: String,
    /// The year's lord.
    pub lord: DashaLord,
    /// Tone per life for the year, `Life::ALL` order, `[0, 1]`.
    pub tones: Vec<f64>,
    /// Overall tone.
    pub tone: f64,
    /// Band of the tone.
    pub band: Band,
    /// The life the year gives most to.
    pub best: Life,
    /// The life it tests most.
    pub weak: Life,
    /// Monthly samples: `(instant, nine scores)`.
    pub months: Vec<(Instant, Vec<f64>)>,
    /// One or two sentences.
    pub headline: String,
    /// The year lord's theme.
    pub theme: String,
}

/// The year containing `at` for `natal`, read where `obs` stands.
pub fn year_chapter(
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<YearChapter, ForecastError> {
    year_chapter_in(Lang::En, spk, eop, natal, obs, system, ayanamsa, at)
}

/// [`year_chapter`] with its sentences in `lang`.
#[allow(clippy::too_many_arguments)]
pub fn year_chapter_in(
    lang: Lang,
    spk: &Spk,
    eop: Option<&EopTable>,
    natal: &Natal,
    obs: &Observer,
    system: HouseSystem,
    ayanamsa: Ayanamsa,
    at: Instant,
) -> Result<YearChapter, ForecastError> {
    let sun_sid = natal.facts.planet(DashaLord::Sun).sidereal;
    let sun_trop = (sun_sid + ayanamsa_degrees(ayanamsa, natal.birth.tt_centuries_since_j2000())).rem_euclid(360.0);
    let year = Utc::from_instant(at).year;
    let ret = |y: i64| solar_return(spk, sun_trop, natal.birth, y, MICROSECOND_NS).ok().flatten();
    let (start, end) = match ret(year) {
        Some(r) if r <= at => (r, ret(year + 1)),
        _ => (
            ret(year - 1).ok_or_else(|| ForecastError::Chart("no solar return".into()))?,
            ret(year),
        ),
    };
    let end = end.unwrap_or(start.add_ns(365 * NS_PER_DAY + NS_PER_DAY / 4));
    let ch = compute(
        spk,
        eop,
        &ChartInput {
            instant: start,
            observer: *obs,
            system,
            ayanamsa,
            topocentric_moon: false,
        },
    )
    .map_err(|e| ForecastError::Chart(format!("{e:?}")))?;
    let lagna_sign = ((ch.cusps_sidereal[0] / 30.0).floor() as usize % 12) as u8;
    let moon = ch.bodies[1].sidereal;
    let n = nakshatra(moon);
    let moon_star = format!("{} {}", NAKSHATRA_NAMES[n.index], n.pada);
    let lord = sign_lord(lagna_sign as usize);
    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let months = slow_series(spk, eop, natal, obs, ayanamsa, start.add_ns(month / 2), month, 12)?;
    let tones: Vec<f64> = (0..9)
        .map(|i| months.iter().map(|(_, v)| v[i]).sum::<f64>() / months.len().max(1) as f64)
        .collect();
    let tone = tones.iter().sum::<f64>() / 9.0;
    let (bi, _) = tones.iter().enumerate().fold((0, f64::MIN), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    let (wi, _) = tones.iter().enumerate().fold((0, f64::MAX), |a, (i, &v)| if v < a.1 { (i, v) } else { a });
    let age = (start.ns_since(natal.birth) as f64 / (365.25 * NS_PER_DAY as f64)).round() as u32;
    let band = Band::of(tone);
    let lon = obs.longitude_deg;
    let (th, pr) = (crate::lang::chapter_theme(lang, lord), crate::lang::chapter_prediction(lang, lord));
    let theme = match lang {
        Lang::En => format!("This year leans towards {th}. {pr}"),
        Lang::Ml => format!("ഈ വർഷം {th} എന്നിവയിലേക്ക് ചായുന്നു. {pr}"),
        Lang::Hi => format!("यह साल {th} की ओर झुका है। {pr}"),
    };
    let bix = match band {
        Band::Bright => 0,
        Band::Good => 1,
        Band::Steady => 2,
        Band::Slow => 3,
        Band::Hold => 4,
    };
    let d = |t: Instant| crate::lang::date(lang, t, lon);
    let (bn, wn) = (crate::lang::life_name(lang, Life::ALL[bi]), crate::lang::life_name(lang, Life::ALL[wi]));
    let headline = match lang {
        Lang::En => {
            let kind = ["a bright year", "a good year", "an even year", "a slow year", "a year to hold still"][bix];
            format!(
                "Your {} year runs from {} to {}. Overall it is {kind}: {} is its gift, and the most patience is needed for {}.",
                ordinal_age(age + 1),
                date_words(start, lon),
                date_words(end, lon),
                Life::ALL[bi].name(),
                Life::ALL[wi].name().to_ascii_lowercase()
            )
        }
        Lang::Ml => {
            let kind = ["തിളക്കമുള്ള വർഷം", "നല്ല വർഷം", "സമനിലയുള്ള വർഷം", "മന്ദമായ വർഷം", "അനങ്ങാതെ നിൽക്കേണ്ട വർഷം"][bix];
            format!(
                "നിങ്ങളുടെ ജീവിതത്തിലെ {}-ാം വർഷം {} മുതൽ {} വരെ. മൊത്തത്തിൽ ഇത് {kind} ആണ്: ഇതിന്റെ സമ്മാനം {bn}; ഏറ്റവും ക്ഷമ വേണ്ടത് {}.",
                age + 1,
                d(start),
                d(end),
                crate::forecast::life_in(lang, Life::ALL[wi])
            )
        }
        Lang::Hi => {
            let kind = ["चमकदार साल", "अच्छा साल", "बराबर साल", "धीमा साल", "ठहरने का साल"][bix];
            format!(
                "आपके जीवन का {} साल {} से {} तक है। कुल मिलाकर यह {kind} है: इसका तोहफ़ा {bn} है, और सबसे ज़्यादा धैर्य {wn} में चाहिए।",
                crate::forecast::hi_ordinal(age as usize + 1),
                d(start),
                d(end)
            )
        }
    };
    Ok(YearChapter {
        start,
        end,
        age,
        lagna_sign,
        moon_star,
        lord,
        tones,
        tone,
        band,
        best: Life::ALL[bi],
        weak: Life::ALL[wi],
        months,
        headline,
        theme,
    })
}

fn ordinal_age(n: u32) -> String {
    let s = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_year_runs_birthday_to_birthday() {
        let Some(spk) = kernel() else { return };
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        let y = year_chapter(&spk, None, &natal, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri, at).unwrap();
        assert!(y.start <= at && at < y.end);
        let s = Utc::from_instant(y.start);
        assert_eq!((s.year, s.month), (2026, 2));
        assert!((19..=21).contains(&s.day));
        let days = y.end.ns_since(y.start) as f64 / NS_PER_DAY as f64;
        assert!((days - 365.25).abs() < 1.0, "{days}");
        assert_eq!(y.age, 36);
        assert_eq!(y.months.len(), 12);
        assert!(y.headline.contains("37th year"));
        assert_eq!(crate::plain::jargon(&y.headline), None, "{}", y.headline);
        assert_eq!(crate::plain::jargon(&y.theme), None, "{}", y.theme);
    }
}
