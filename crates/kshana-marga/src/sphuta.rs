//! The Prasna Marga sensitive points (sphutas) and the readings the text attaches to
//! them.
//!
//! # Sources
//!
//! * Prasna Marga ch. VIII (B. V. Raman's translation, Part I, chs. I–XVI, Motilal
//!   Banarsidass), stanzas 1–5 for Trisphuta, Chatussphuta and Panchasphuta, 6–10 for
//!   the Prana, Deha and Mrityu sphutas, 15 for the navamsa ("multiply by nine")
//!   sphutas, 26–30 for the Trisphuta's sign and nakshatra readings. The stanza numbers
//!   are those of Raman's edition as the blog transcriptions of it number them
//!   (vedanga-jyotish.blogspot.com "Prasna II"; varahamihira.blogspot.com "Praśna II:
//!   Use of Trisphuta in praśna").
//! * Prasna Marga Part II, the chapter on progeny, for the Beeja and Kshetra sphutas
//!   (the longitude sums as every modern reading of the text gives them:
//!   eastrovedica.com lesson 52; planetarypositions.com "Beeja Sphuta, Kshetra
//!   Sphuta"); the text's own procedure (stanza 5, the expired portions of the planets'
//!   asterisms in ghatis) is older and not implemented.
//! * The Yogi point: Sun + Moon + 93°20′ (the start of Pushya), its nakshatra lord the
//!   Yogi, the sign lord the Sahayogi, the lord of the nakshatra 186°40′ on the Avayogi
//!   (hinducalculator.com "Yogi-Avayogi"; astrosaxena.in "Concept of Yogi, Avayogi";
//!   the convention Jagannatha Hora, Kala and Parashara's Light print).
//! * The Bhrigu Bindu: the midpoint of the Moon and Rahu (Bhrigu Nandi Nadi practice;
//!   blog.cosmicinsights.net "Secrets of Destiny point"; astroindus.com "Bhrigu Bindu
//!   calculator").
//!
//! # Gulika
//!
//! Gulika is taken from `kshana_day`: the sidereal ascendant at the **start** of
//! Saturn's part of the day (sunrise to sunset) or of the night, the eight parts being
//! lorded in weekday order from the weekday lord (BPHS ch. 3, 66–67). The start of the
//! part is Jagannatha Hora's default for Gulika ([`GulikaConvention::JhoraStart`]); the
//! Kerala thirty-ghati Mandi table of the Malayalam almanacs is the alternative
//! ([`GulikaConvention::KeralaMandi`]).

use kshana_chart::Chart;
use kshana_day::{
    ascendant_sidereal, mandi_instant_kerala, upagraha_longitude, KalaPoint, Upagraha,
};
use kshana_ephem::Spk;
use kshana_events::solar_day;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant};
use kshana_vedic::panchanga::NAKSHATRA_NAMES;
use kshana_vedic::{kp_lords, nakshatra, sign_lord, varga, Ayanamsa, DashaLord};

use crate::{body_sidereal, graha_name, norm360, sign_of, MargaError, SIGN_NAMES};

/// A sensitive point.
#[derive(Clone, Debug, PartialEq)]
pub struct Sphuta {
    /// Name.
    pub name: &'static str,
    /// Sidereal longitude, degrees in `[0, 360)`.
    pub longitude: f64,
    /// Sign index 0–11.
    pub sign: u8,
    /// Nakshatra index 0–26.
    pub nakshatra: usize,
    /// Pada 1–4.
    pub pada: u8,
    /// Lord of the sign.
    pub lord: DashaLord,
    /// Lord of the nakshatra (the Vimshottari lord).
    pub star_lord: DashaLord,
    /// The formula, the place, and the text's reading, in words.
    pub words: Vec<String>,
}

pub(crate) fn make(
    name: &'static str,
    longitude: f64,
    formula: String,
    mut words: Vec<String>,
) -> Sphuta {
    let l = norm360(longitude);
    let n = nakshatra(l);
    let sign = sign_of(l);
    let star_lord = kp_lords(l).star;
    let lord = sign_lord(sign as usize);
    words.insert(
        0,
        format!(
            "{name} = {formula} = {:.4}°: {:.2}° {}, {} pada {}, sign lord {}, star lord {}",
            l,
            l.rem_euclid(30.0),
            SIGN_NAMES[sign as usize],
            NAKSHATRA_NAMES[n.index],
            n.pada,
            graha_name(lord),
            graha_name(star_lord)
        ),
    );
    Sphuta {
        name,
        longitude: l,
        sign,
        nakshatra: n.index,
        pada: n.pada,
        lord,
        star_lord,
        words,
    }
}

/// Events the text attaches to the Trisphuta's nakshatra, by trine group: group `i`
/// holds nakshatras `i`, `i + 9` and `i + 18` (0 = Ashwini, Magha, Mula). Prasna Marga
/// VIII lists nine events; the five transcribed here are those the available
/// translations give in full, the other four are left `None` rather than guessed.
pub const TRISPHUTA_TRINE_EVENTS: [Option<&str>; 9] = [
    Some("a causeless quarrel within three days"),
    Some("a poor old Brahmin turning up within three days"),
    Some("fear from fire, or a conflagration"),
    None,
    None,
    None,
    None,
    Some("the death of a quadruped"),
    Some("a servant falling from a tree, a lamp put out by an insect, or news of a death"),
];

/// The text's readings of a Trisphuta longitude (Prasna Marga VIII.26–30): the sign
/// rule (Cancer, Scorpio and Pisces bring intense trouble, most in Pisces, least in
/// Cancer), the last-navamsa timing of those three signs (a year, a month, a day), and
/// the nakshatra trine's confirming event. These are the text's words about a prasna
/// on illness; the app shows them as the text's, never as a verdict.
pub fn trisphuta_words(longitude: f64) -> Vec<String> {
    let l = norm360(longitude);
    let sign = sign_of(l);
    let in_sign = l.rem_euclid(30.0);
    let last_navamsa = in_sign >= 30.0 - 30.0 / 9.0;
    let mut w = Vec::new();
    match sign {
        3 | 7 | 11 => {
            let (how, when) = match sign {
                3 => ("least", "a year"),
                7 => ("more", "a month"),
                _ => ("most", "a day"),
            };
            w.push(format!(
                "Prasna Marga VIII.28: Trisphuta in {} is read as intense trouble ({} of the three signs Cancer, Scorpio, Pisces)",
                SIGN_NAMES[sign as usize], how
            ));
            if last_navamsa {
                w.push(format!(
                    "Prasna Marga VIII.29: in the last navamsa of {} the text's timing is {when}",
                    SIGN_NAMES[sign as usize]
                ));
            }
        }
        _ => w.push(format!(
            "Prasna Marga VIII.28 names Cancer, Scorpio and Pisces as the Trisphuta signs of intense trouble; {} is not one of them",
            SIGN_NAMES[sign as usize]
        )),
    }
    let n = nakshatra(l).index;
    let group = n % 9;
    match TRISPHUTA_TRINE_EVENTS[group] {
        Some(ev) => w.push(format!(
            "Prasna Marga VIII: Trisphuta in {} (trine of {}, {} and {}): the confirming event the text names is {ev}",
            NAKSHATRA_NAMES[n], NAKSHATRA_NAMES[group], NAKSHATRA_NAMES[group + 9], NAKSHATRA_NAMES[group + 18]
        )),
        None => w.push(format!(
            "Prasna Marga VIII: Trisphuta in {} (trine of {}, {} and {}); the text's confirming event for this trine is not transcribed in the available translations",
            NAKSHATRA_NAMES[n], NAKSHATRA_NAMES[group], NAKSHATRA_NAMES[group + 9], NAKSHATRA_NAMES[group + 18]
        )),
    }
    w
}

/// Trisphuta = Lagna + Moon + Gulika (Prasna Marga VIII.1–3).
pub fn trisphuta(lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    let l = norm360(lagna + moon + gulika);
    make(
        "Trisphuta",
        l,
        format!("Lagna {lagna:.4} + Moon {moon:.4} + Gulika {gulika:.4}"),
        trisphuta_words(l),
    )
}

/// Chatussphuta = Trisphuta + Sun (Prasna Marga VIII.4).
pub fn chatussphuta(lagna: f64, moon: f64, gulika: f64, sun: f64) -> Sphuta {
    make(
        "Chatussphuta",
        lagna + moon + gulika + sun,
        format!("Trisphuta + Sun {sun:.4}"),
        vec!["Prasna Marga VIII.4: the Trisphuta with the Sun added; read with the Trisphuta and the Panchasphuta for the course of an illness".into()],
    )
}

/// Panchasphuta = Chatussphuta + Rahu (Prasna Marga VIII.5).
pub fn panchasphuta(lagna: f64, moon: f64, gulika: f64, sun: f64, rahu: f64) -> Sphuta {
    make(
        "Panchasphuta",
        lagna + moon + gulika + sun + rahu,
        format!("Chatussphuta + Rahu {rahu:.4}"),
        vec!["Prasna Marga VIII.5: the Chatussphuta with Rahu added".into()],
    )
}

/// Pranasphuta = Lagna × 5 + Gulika (Prasna Marga VIII.6).
pub fn pranasphuta(lagna: f64, gulika: f64) -> Sphuta {
    make(
        "Pranasphuta",
        5.0 * lagna + gulika,
        format!("5 × Lagna {lagna:.4} + Gulika {gulika:.4}"),
        vec![prana_deha_mrityu_reading()],
    )
}

/// Dehasphuta = Moon × 8 + Gulika (Prasna Marga VIII.7).
pub fn dehasphuta(moon: f64, gulika: f64) -> Sphuta {
    make(
        "Dehasphuta",
        8.0 * moon + gulika,
        format!("8 × Moon {moon:.4} + Gulika {gulika:.4}"),
        vec![prana_deha_mrityu_reading()],
    )
}

/// Mrityusphuta = Gulika × 7 + Sun (Prasna Marga VIII.8).
pub fn mrityusphuta(gulika: f64, sun: f64) -> Sphuta {
    make(
        "Mrityusphuta",
        7.0 * gulika + sun,
        format!("7 × Gulika {gulika:.4} + Sun {sun:.4}"),
        vec![prana_deha_mrityu_reading()],
    )
}

fn prana_deha_mrityu_reading() -> String {
    "Prasna Marga VIII.6–10 reads the Prana, Deha and Mrityu sphutas together: the transit of malefics, or of the Mrityusphuta's lord, over the Prana and Deha sphutas marks the crisis of an illness, and benefics there mark relief; the app shows the points and the text's words, not a verdict".into()
}

/// Sookshma (subtle) Trisphuta = Trisphuta × 9, the Trisphuta's navamsa longitude
/// (Prasna Marga VIII.15: "multiply the Lagna, Moon, Gulika and Trisphuta by nine").
///
/// Convention: the text's "multiply by nine and drop the signs" is the navamsa
/// longitude; summing the navamsa longitudes of the Lagna, Moon and Gulika separately
/// gives the same point modulo 360°, so the two readings agree.
pub fn sookshma_trisphuta(lagna: f64, moon: f64, gulika: f64) -> Sphuta {
    let t = norm360(lagna + moon + gulika);
    make(
        "Sookshma Trisphuta",
        9.0 * t,
        format!("9 × Trisphuta {t:.4}"),
        vec!["Prasna Marga VIII.15–16: sickness is read to begin on the day of the nakshatra of the Trisphuta navamsa; the text continues with the next nakshatra days for its stages".into()],
    )
}

fn parity_words(name: &str, longitude: f64, wants_odd: bool) -> Vec<String> {
    let rasi_odd = sign_of(longitude) % 2 == 0;
    let nav_odd = sign_of(varga(longitude, 9)) % 2 == 0;
    let want = if wants_odd { "odd" } else { "even" };
    let fits = |b: bool| {
        if b == wants_odd {
            "fits"
        } else {
            "does not fit"
        }
    };
    vec![
        format!(
            "{name}: rasi {} ({}), navamsa {} ({}); the text asks for an {want} sign and an {want} navamsa",
            SIGN_NAMES[sign_of(longitude) as usize],
            fits(rasi_odd),
            SIGN_NAMES[sign_of(varga(longitude, 9)) as usize],
            fits(nav_odd)
        ),
        if rasi_odd == wants_odd && nav_odd == wants_odd {
            "both the sign and the navamsa fit: the text's favourable configuration".into()
        } else if rasi_odd == wants_odd || nav_odd == wants_odd {
            "one of the two fits: a middling configuration in the text".into()
        } else {
            "neither fits: the text's unfavourable configuration; the text adds that benefics' aspects or conjunction repair it".into()
        },
    ]
}

/// Beeja sphuta = Sun + Venus + Jupiter, the male factor (Prasna Marga Part II, the
/// chapter on progeny): favourable in an odd sign and an odd navamsa.
pub fn beeja_sphuta(sun: f64, venus: f64, jupiter: f64) -> Sphuta {
    let l = norm360(sun + venus + jupiter);
    make(
        "Beeja sphuta",
        l,
        format!("Sun {sun:.4} + Venus {venus:.4} + Jupiter {jupiter:.4}"),
        parity_words("Beeja", l, true),
    )
}

/// Kshetra sphuta = Moon + Mars + Jupiter, the female factor (Prasna Marga Part II,
/// the chapter on progeny): favourable in an even sign and an even navamsa.
///
/// Convention: every reading of the text consulted sums the Moon, Mars and Jupiter
/// (Jupiter being the karaka of children in both sphutas); a Moon + Mars + Venus form
/// circulates in some software and is not used here.
pub fn kshetra_sphuta(moon: f64, mars: f64, jupiter: f64) -> Sphuta {
    let l = norm360(moon + mars + jupiter);
    make(
        "Kshetra sphuta",
        l,
        format!("Moon {moon:.4} + Mars {mars:.4} + Jupiter {jupiter:.4}"),
        parity_words("Kshetra", l, false),
    )
}

/// The Yogi point and its three planets.
#[derive(Clone, Debug, PartialEq)]
pub struct Yogi {
    /// Sun + Moon + 93°20′.
    pub point: Sphuta,
    /// Lord of the Yogi point's nakshatra.
    pub yogi: DashaLord,
    /// Lord of the Yogi point's sign (the duplicate Yogi).
    pub sahayogi: DashaLord,
    /// The Avayogi point: Yogi point + 186°40′.
    pub avayogi_point: f64,
    /// Lord of the Avayogi point's nakshatra.
    pub avayogi: DashaLord,
    /// The reading, in words.
    pub words: Vec<String>,
}

/// The Yogi point (Sun + Moon + 93°20′) with the Yogi (its nakshatra lord), the
/// Sahayogi (its sign lord) and the Avayogi (the nakshatra lord 186°40′ on).
///
/// Convention: 186°40′ is fourteen nakshatras, and since the Vimshottari lords repeat
/// every nine, the Avayogi is also the lord of the sixth nakshatra counted from the
/// Yogi's; the two descriptions in circulation name the same planet. A few authors take
/// the sign lord of the Avayogi point instead; that is not done here.
pub fn yogi(sun: f64, moon: f64) -> Yogi {
    let l = norm360(sun + moon + 93.0 + 20.0 / 60.0);
    let point = make(
        "Yogi point",
        l,
        format!("Sun {sun:.4} + Moon {moon:.4} + 93°20′"),
        Vec::new(),
    );
    let yogi = point.star_lord;
    let sahayogi = point.lord;
    let avayogi_point = norm360(l + 186.0 + 40.0 / 60.0);
    let avayogi = kp_lords(avayogi_point).star;
    let words = vec![
        format!(
            "Yogi {}: the lord of {}; its periods and transits are read as the chart's prosperity",
            graha_name(yogi),
            NAKSHATRA_NAMES[point.nakshatra]
        ),
        format!(
            "Sahayogi {}: the lord of {}, the Yogi's helper{}",
            graha_name(sahayogi),
            SIGN_NAMES[point.sign as usize],
            if sahayogi == yogi { " (the same planet)" } else { "" }
        ),
        format!(
            "Avayogi {}: the lord of {} at {:.4}° (Yogi point + 186°40′, the sixth nakshatra's lord); its periods and transits are read as obstruction{}",
            graha_name(avayogi),
            NAKSHATRA_NAMES[nakshatra(avayogi_point).index],
            avayogi_point,
            if avayogi == sahayogi {
                "; it is also the Sahayogi, so the reading is mixed"
            } else {
                ""
            }
        ),
    ];
    Yogi {
        point,
        yogi,
        sahayogi,
        avayogi_point,
        avayogi,
        words,
    }
}

/// The Bhrigu Bindu: the midpoint of the Moon and Rahu on the shorter arc.
///
/// Convention: the plain average of two longitudes picks the midpoint of the longer arc
/// whenever the two straddle 0° Aries; the shorter-arc midpoint is taken here, as
/// blog.cosmicinsights.net and astroindus.com describe. With the Moon at 10° and Rahu
/// at 300° this gives 335°, not 155°.
pub fn bhrigu_bindu(moon: f64, rahu: f64) -> Sphuta {
    let d = (rahu - moon).rem_euclid(360.0);
    let half = if d > 180.0 {
        (d - 360.0) / 2.0
    } else {
        d / 2.0
    };
    let l = norm360(moon + half);
    make(
        "Bhrigu Bindu",
        l,
        format!("midpoint of Moon {moon:.4} and Rahu {rahu:.4} on the shorter arc"),
        vec![
            "the midpoint of the Moon and Rahu on the shorter arc (convention stated); Bhrigu Nandi Nadi practice watches the transits of Jupiter, Saturn and the nodes over this point and its trines".into(),
        ],
    )
}

/// Which instant of Saturn's part gives Gulika.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GulikaConvention {
    /// The start of Saturn's part of the day or night (Jagannatha Hora's default).
    JhoraStart,
    /// The Kerala thirty-ghati Mandi table (Sun 26, Mon 22, … of the day; 10, 6, … of
    /// the night).
    KeralaMandi,
}

/// Gulika's longitude and where it came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GulikaPosition {
    /// Convention used.
    pub convention: GulikaConvention,
    /// The instant of the kala point.
    pub at: Instant,
    /// True when taken from the night's parts.
    pub night: bool,
    /// Sidereal longitude of the ascendant at that instant, degrees.
    pub sidereal: f64,
}

/// Gulika for a birth or question at `at`: the sidereal ascendant at the start of
/// Saturn's part (Jagannatha Hora's default), in the day's parts when `at` is before
/// sunset and in the night's parts otherwise.
pub fn gulika_longitude(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    ayanamsa: Ayanamsa,
) -> Result<GulikaPosition, MargaError> {
    gulika_longitude_with(spk, eop, obs, at, ayanamsa, GulikaConvention::JhoraStart)
}

/// Gulika under a stated convention.
pub fn gulika_longitude_with(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    at: Instant,
    ayanamsa: Ayanamsa,
    convention: GulikaConvention,
) -> Result<GulikaPosition, MargaError> {
    let day = solar_day(spk, eop, obs, at)?.ok_or(MargaError::NoSolarDay)?;
    let night = at >= day.sunset;
    Ok(match convention {
        GulikaConvention::JhoraStart => {
            let u = upagraha_longitude(
                eop,
                obs,
                &day,
                Upagraha::Gulika,
                KalaPoint::Start,
                at,
                ayanamsa,
            );
            GulikaPosition {
                convention,
                at: u.at,
                night: u.night,
                sidereal: u.sidereal,
            }
        }
        GulikaConvention::KeralaMandi => {
            let t = mandi_instant_kerala(&day, obs, night);
            GulikaPosition {
                convention,
                at: t,
                night,
                sidereal: ascendant_sidereal(eop, obs, t, ayanamsa),
            }
        }
    })
}

/// All the Prasna Marga sphutas of a chart, given Gulika: Trisphuta, Chatussphuta,
/// Panchasphuta, Pranasphuta, Dehasphuta, Mrityusphuta, Sookshma Trisphuta, Beeja,
/// Kshetra, the Yogi point and the Bhrigu Bindu, in that order. Rahu is the true node.
pub fn sphutas(chart: &Chart, gulika: f64) -> Result<Vec<Sphuta>, MargaError> {
    let lagna = chart.cusps_sidereal[0];
    let sun = body_sidereal(chart, "Sun")?;
    let moon = body_sidereal(chart, "Moon")?;
    let mars = body_sidereal(chart, "Mars")?;
    let jupiter = body_sidereal(chart, "Jupiter")?;
    let venus = body_sidereal(chart, "Venus")?;
    let rahu = chart.true_node.1;
    Ok(vec![
        trisphuta(lagna, moon, gulika),
        chatussphuta(lagna, moon, gulika, sun),
        panchasphuta(lagna, moon, gulika, sun, rahu),
        pranasphuta(lagna, gulika),
        dehasphuta(moon, gulika),
        mrityusphuta(gulika, sun),
        sookshma_trisphuta(lagna, moon, gulika),
        beeja_sphuta(sun, venus, jupiter),
        kshetra_sphuta(moon, mars, jupiter),
        yogi(sun, moon).point,
        bhrigu_bindu(moon, rahu),
    ])
}

/// The sphutas of a chart with the Gulika that produced them.
#[derive(Clone, Debug, PartialEq)]
pub struct SphutaReport {
    /// Gulika.
    pub gulika: GulikaPosition,
    /// The eleven sphutas of [`sphutas`].
    pub sphutas: Vec<Sphuta>,
    /// The Yogi point and its planets.
    pub yogi: Yogi,
    /// The Bhrigu Bindu.
    pub bhrigu_bindu: Sphuta,
}

/// Gulika from the sky, then every sphuta of the chart. `ayanamsa` must be the one the
/// chart was cast with.
pub fn sphuta_report(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
) -> Result<SphutaReport, MargaError> {
    let gulika = gulika_longitude(spk, eop, obs, chart.instant, ayanamsa)?;
    let all = sphutas(chart, gulika.sidereal)?;
    let sun = body_sidereal(chart, "Sun")?;
    let moon = body_sidereal(chart, "Moon")?;
    Ok(SphutaReport {
        gulika,
        yogi: yogi(sun, moon),
        bhrigu_bindu: bhrigu_bindu(moon, chart.true_node.1),
        sphutas: all,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_birth, kochi_chart, KOCHI};

    #[test]
    fn formulas_on_hand_values() {
        // Raman's worked example: 43°9′ + 18°29′ + 64°26′ = 126°4′
        let t = trisphuta(43.0 + 9.0 / 60.0, 18.0 + 29.0 / 60.0, 64.0 + 26.0 / 60.0);
        assert!(
            (t.longitude - (126.0 + 4.0 / 60.0)).abs() < 1e-9,
            "{}",
            t.longitude
        );
        assert_eq!(t.sign, 4);
        assert_eq!(t.lord, DashaLord::Sun);
        let c = chatussphuta(100.0, 50.0, 20.0, 300.0);
        assert!((c.longitude - 110.0).abs() < 1e-9);
        let p = panchasphuta(100.0, 50.0, 20.0, 300.0, 200.0);
        assert!((p.longitude - 310.0).abs() < 1e-9);
        assert!((pranasphuta(100.0, 20.0).longitude - 160.0).abs() < 1e-9);
        assert!((dehasphuta(50.0, 20.0).longitude - 60.0).abs() < 1e-9);
        assert!((mrityusphuta(20.0, 300.0).longitude - 80.0).abs() < 1e-9);
        // 9 × Trisphuta: 9 × 170 = 1530 → 90
        assert!((sookshma_trisphuta(100.0, 50.0, 20.0).longitude - 90.0).abs() < 1e-9);
        assert!((beeja_sphuta(10.0, 20.0, 30.0).longitude - 60.0).abs() < 1e-9);
        assert!((kshetra_sphuta(350.0, 20.0, 30.0).longitude - 40.0).abs() < 1e-9);
        for s in [t, c, p] {
            assert!(!s.words.is_empty());
            assert!(s.words[0].contains('='));
        }
    }

    #[test]
    fn yogi_point_lords() {
        // Sun 1°, Moon 0°: Yogi point 94°20′ in Pushya (Saturn); sign Cancer (Moon);
        // Avayogi point 281° in Shravana (Moon), whose lord is also that of Hasta, the
        // sixth nakshatra from Pushya
        let y = yogi(1.0, 0.0);
        assert!((y.point.longitude - (94.0 + 20.0 / 60.0)).abs() < 1e-9);
        assert_eq!(y.point.nakshatra, 7);
        assert_eq!(y.yogi, DashaLord::Saturn);
        assert_eq!(y.sahayogi, DashaLord::Moon);
        assert!((y.avayogi_point - 281.0).abs() < 1e-9);
        assert_eq!(nakshatra(y.avayogi_point).index, 21);
        assert_eq!(y.avayogi, DashaLord::Moon);
        // the sixth nakshatra from the Yogi's has the same lord as the Avayogi point's
        for (s, m) in [(12.3, 200.1), (359.0, 2.0), (150.0, 150.0), (77.7, 311.2)] {
            let y = yogi(s, m);
            let sixth = (y.point.nakshatra + 5) % 27;
            assert_eq!(DashaLord::ORDER[sixth % 9], y.avayogi);
        }
        assert_eq!(y.words.len(), 3);
    }

    #[test]
    fn bhrigu_bindu_takes_the_shorter_arc() {
        assert!((bhrigu_bindu(10.0, 300.0).longitude - 335.0).abs() < 1e-9);
        assert!((bhrigu_bindu(300.0, 10.0).longitude - 335.0).abs() < 1e-9);
        assert!((bhrigu_bindu(100.0, 200.0).longitude - 150.0).abs() < 1e-9);
        assert!((bhrigu_bindu(0.0, 180.0).longitude - 90.0).abs() < 1e-9);
    }

    #[test]
    fn trisphuta_readings_are_data() {
        let w = trisphuta_words(359.0); // last navamsa of Pisces, Revati (group 8)
        assert!(w.iter().any(|s| s.contains("Pisces") && s.contains("most")));
        assert!(w.iter().any(|s| s.contains("a day")));
        assert!(w.iter().any(|s| s.contains("news of a death")));
        let w = trisphuta_words(5.0); // Aries, Ashwini
        assert!(w.iter().any(|s| s.contains("not one of them")));
        assert!(w.iter().any(|s| s.contains("quarrel")));
        let w = trisphuta_words(100.0); // Cancer, Pushya (group 7)
        assert!(w.iter().any(|s| s.contains("least")));
        assert!(w.iter().any(|s| s.contains("quadruped")));
        assert_eq!(TRISPHUTA_TRINE_EVENTS.iter().flatten().count(), 5);
    }

    #[test]
    fn kochi_sphutas_from_the_kernel() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let r = sphuta_report(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert_eq!(r.gulika.convention, GulikaConvention::JhoraStart);
        // 12:06 UTC is 17:36 local, before sunset at Kochi in late October
        assert!(!r.gulika.night);
        assert!(r.gulika.at < kochi_birth());
        assert_eq!(r.sphutas.len(), 11);
        let lagna = ch.cusps_sidereal[0];
        let moon = ch.bodies[1].sidereal;
        let want = norm360(lagna + moon + r.gulika.sidereal);
        assert!((r.sphutas[0].longitude - want).abs() < 1e-9);
        for s in &r.sphutas {
            assert!((0.0..360.0).contains(&s.longitude));
            assert_eq!(s.sign, sign_of(s.longitude));
            assert!(!s.words.is_empty(), "{}", s.name);
            assert!(s.words.iter().all(|w| !w.is_empty()));
        }
        assert_eq!(r.sphutas[9].longitude, r.yogi.point.longitude);
        assert_eq!(r.sphutas[10].longitude, r.bhrigu_bindu.longitude);
        // the Kerala table gives a different instant and so a different longitude
        let k = gulika_longitude_with(
            &spk,
            None,
            &KOCHI,
            ch.instant,
            Ayanamsa::Lahiri,
            GulikaConvention::KeralaMandi,
        )
        .unwrap();
        assert_eq!(k.convention, GulikaConvention::KeralaMandi);
        assert_ne!(k.at, r.gulika.at);
    }
}
