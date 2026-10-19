//! The Ashtamangala number of the Kerala prasna, and the Arudha, Veethi and Chatra
//! rasis with the lagna, Chandra and Arudha sphutas.
//!
//! # The number
//!
//! One hundred and eight cowries are divided by the astrologer into three heaps; each
//! heap is counted off in eights and the remainder kept, a remainder of zero counting
//! as eight; the three remainders, read left to right, make a three-digit number with
//! no digit above eight, the Ashtamangala number (Prasna Marga ch. IV; Wikipedia
//! "Ashtamangala prasnam"; shyamasundaradasa.com "Ashtamangala Deva Prasna" part 3).
//! The digits stand for the past, the present and the future, each digit for a planet:
//! 1 Sun, 2 Mars, 3 Jupiter, 4 Mercury, 5 Venus, 6 Saturn, 7 Moon, 8 Rahu; odd digits
//! are read as good, even as bad (the same sources; a tambula scheme with 1 Sun, 2 Moon
//! … 7 Saturn and no Rahu circulates for betel-leaf counts and is not this one).
//!
//! Convention ([`ashtamangala`]): the sign the number signals is the remainder of the
//! number by twelve counted from Aries, a zero remainder being Pisces, and its nakshatra
//! the remainder by twenty-seven from Ashwini, zero being Revati, as Kerala practice
//! reads a number into a rasi; the available translations of the text do not print
//! this step and it is marked as practice, not stanza. The deities and doshas each
//! planet signals ([`DIGIT_SIGNALS`]) follow the planet–deity correspondences of
//! Prasna Marga ch. XV (the deity chapter) as the Kerala Deva prasna applies them.
//!
//! # Arudha, Veethi and Chatra
//!
//! The Arudha rasi is the sign on which the querent's gold piece is placed (Prasna
//! Marga ch. IV and the notes of Raman's edition; the Arudha sphuta used here is the
//! lagna's degree carried into the Arudha sign, a convention). The Veethi rasi follows
//! the Sun: Sun in Taurus, Gemini, Cancer or Leo gives Aries; in Scorpio, Sagittarius,
//! Capricorn or Aquarius Gemini; in Pisces, Aries, Virgo or Libra Taurus (the three
//! paths, uttara, dakshina and madhya; Prasna Marga's table as the Chatra rasi section,
//! Raman p. 275, and sksastro.blogspot.com "Dagdha rasis" transcribe it). The Chatra
//! rasi is the sign reached by counting from the Veethi rasi as many signs as the lagna
//! is from the Arudha, both ends counted (Raman's example: Arudha Taurus, lagna Leo,
//! count four; from Veethi Taurus the fourth is Leo).

use kshana_chart::Chart;
use kshana_vedic::DashaLord;

use crate::sphuta::{make, Sphuta};
use crate::{body_sidereal, graha_name, norm360, sign_of, MargaError, SIGN_NAMES};

/// For each digit 1–8: the planet, the deity it signals and the dosha or matter it
/// points to in the Kerala Deva prasna (convention stated in the module notes).
pub const DIGIT_SIGNALS: [(DashaLord, &str, &str); 8] = [
    (
        DashaLord::Sun,
        "Siva",
        "the displeasure of the presiding deity, the father's line, authority",
    ),
    (
        DashaLord::Mars,
        "Subrahmanya and Bhadrakali",
        "fierce deities, fire, quarrels, enemies, blood",
    ),
    (
        DashaLord::Jupiter,
        "Brahma and the family deity (Vishnu in some readings)",
        "lapses towards the preceptor and Brahmins, the ancestral shrine",
    ),
    (
        DashaLord::Mercury,
        "Vishnu",
        "learning, trade, speech, vows left unfulfilled",
    ),
    (
        DashaLord::Venus,
        "Lakshmi and the Yakshi",
        "women, wealth, pleasures, the Yakshi's dosha",
    ),
    (
        DashaLord::Saturn,
        "Sasta (Ayyappa) and the Kirata",
        "the dead, pitru and preta dosha, servants, disease",
    ),
    (
        DashaLord::Moon,
        "Durga (Bhagavati)",
        "the mother, the mind, water, the Devi's displeasure",
    ),
    (
        DashaLord::Rahu,
        "the Nagas",
        "sarpa dosha, poison, deceit, the serpent grove",
    ),
];

/// Planet of a digit 1–8, `None` for any other digit.
pub fn digit_planet(d: u8) -> Option<DashaLord> {
    DIGIT_SIGNALS.get(d.checked_sub(1)? as usize).map(|s| s.0)
}

/// An Ashtamangala number read.
#[derive(Clone, Debug, PartialEq)]
pub struct Ashtamangala {
    /// The number.
    pub number: u32,
    /// Its decimal digits, most significant first.
    pub digits: Vec<u8>,
    /// The planet of each digit (`None` for a 0 or a 9).
    pub planets: Vec<Option<DashaLord>>,
    /// The sign the number signals, 0 = Aries.
    pub arudha_sign: u8,
    /// The nakshatra the number signals, 0 = Ashwini; `None` for zero.
    pub nakshatra: Option<usize>,
    /// The readings, in words.
    pub indications: Vec<String>,
}

/// Read an Ashtamangala number.
pub fn ashtamangala(number: u32) -> Ashtamangala {
    let digits: Vec<u8> = number.to_string().bytes().map(|b| b - b'0').collect();
    let planets: Vec<Option<DashaLord>> = digits.iter().map(|d| digit_planet(*d)).collect();
    let r12 = (number % 12) as u8;
    let arudha_sign = (r12 + 11) % 12;
    let nakshatra = if number == 0 {
        None
    } else {
        Some(((number % 27) as usize + 26) % 27)
    };
    let mut indications = Vec::new();
    let labels: &[&str] = if digits.len() == 3 {
        &["past", "present", "future"]
    } else {
        &[]
    };
    for (i, (d, p)) in digits.iter().zip(planets.iter()).enumerate() {
        let label = labels
            .get(i)
            .map(|l| l.to_string())
            .unwrap_or_else(|| format!("digit {}", i + 1));
        match p {
            Some(g) => {
                let (_, deity, signal) = DIGIT_SIGNALS[(*d - 1) as usize];
                indications.push(format!(
                    "{label}: {d} is {} ({}); deity {deity}; signals {signal}",
                    graha_name(*g),
                    if d % 2 == 1 {
                        "odd, read as good"
                    } else {
                        "even, read as bad"
                    }
                ));
            }
            None => indications.push(format!(
                "{label}: {d} is not an Ashtamangala digit (the cowrie remainders run 1–8)"
            )),
        }
    }
    if digits.len() == 3 {
        let good = digits.iter().filter(|d| *d % 2 == 1).count();
        indications.push(format!(
            "{good} of the three digits are odd: {}",
            match good {
                3 => "the number is wholly favourable",
                2 => "favourable on the whole",
                1 => "unfavourable on the whole",
                _ => "the number is wholly unfavourable",
            }
        ));
    }
    indications.push(format!(
        "remainder by twelve {r12}: the number signals {} (practice: remainder counted from Aries, zero is Pisces)",
        SIGN_NAMES[arudha_sign as usize]
    ));
    if let Some(n) = nakshatra {
        indications.push(format!(
            "remainder by twenty-seven {}: {} (practice: counted from Ashwini, zero is Revati)",
            number % 27,
            kshana_vedic::panchanga::NAKSHATRA_NAMES[n]
        ));
    }
    Ashtamangala {
        number,
        digits,
        planets,
        arudha_sign,
        nakshatra,
        indications,
    }
}

/// Veethi rasi from the Sun's sign: Aries for the Sun in Taurus–Leo, Gemini for
/// Scorpio–Aquarius, Taurus for Pisces, Aries, Virgo and Libra.
pub fn veethi_rasi(sun_sign: u8) -> u8 {
    match sun_sign % 12 {
        1..=4 => 0,
        7..=10 => 2,
        _ => 1,
    }
}

/// Chatra rasi: count from the Arudha to the lagna, both ends included, and take that
/// many from the Veethi rasi of the Sun's sign.
pub fn chatra_rasi(arudha_sign: u8, lagna_sign: u8, sun_sign: u8) -> u8 {
    let n = (lagna_sign as i32 - arudha_sign as i32).rem_euclid(12) as u8 + 1;
    (veethi_rasi(sun_sign) + n - 1) % 12
}

/// The Kerala prasna's three sphutas and two derived signs.
#[derive(Clone, Debug, PartialEq)]
pub struct PrasnaSphutas {
    /// The lagna.
    pub lagna: Sphuta,
    /// The Moon.
    pub chandra: Sphuta,
    /// The Arudha: the lagna's degree in the Arudha sign.
    pub arudha: Sphuta,
    /// The Arudha sign given, 0 = Aries.
    pub arudha_sign: u8,
    /// The Veethi rasi of the Sun's sign.
    pub veethi_sign: u8,
    /// The Chatra rasi.
    pub chatra_sign: u8,
    /// The derivation, in words.
    pub words: Vec<String>,
}

/// The prasna sphutas of a chart with the Arudha sign the gold piece marked.
pub fn prasna_sphutas(chart: &Chart, arudha_sign: u8) -> Result<PrasnaSphutas, MargaError> {
    let arudha_sign = arudha_sign % 12;
    let lagna_lon = chart.cusps_sidereal[0];
    let moon = body_sidereal(chart, "Moon")?;
    let sun = body_sidereal(chart, "Sun")?;
    let lagna_sign = sign_of(lagna_lon);
    let sun_sign = sign_of(sun);
    let arudha_lon = norm360(arudha_sign as f64 * 30.0 + lagna_lon.rem_euclid(30.0));
    let veethi_sign = veethi_rasi(sun_sign);
    let chatra_sign = chatra_rasi(arudha_sign, lagna_sign, sun_sign);
    let n = (lagna_sign as i32 - arudha_sign as i32).rem_euclid(12) + 1;
    let words = vec![
        format!(
            "Arudha {}: the sign the gold piece marked; its sphuta carries the lagna's degree (convention)",
            SIGN_NAMES[arudha_sign as usize]
        ),
        format!(
            "Veethi {}: the Sun in {} walks that path",
            SIGN_NAMES[veethi_sign as usize], SIGN_NAMES[sun_sign as usize]
        ),
        format!(
            "Chatra {}: from Arudha {} to lagna {} is {n} counting both ends; the {n}th from Veethi {}",
            SIGN_NAMES[chatra_sign as usize],
            SIGN_NAMES[arudha_sign as usize],
            SIGN_NAMES[lagna_sign as usize],
            SIGN_NAMES[veethi_sign as usize]
        ),
        "Prasna Marga reads the Arudha and the Chatra rasi beside the lagna and the Moon; wherever the lagna is named, Chandra lagna is read as well".into(),
    ];
    Ok(PrasnaSphutas {
        lagna: make("Lagna", lagna_lon, "the rising degree".into(), Vec::new()),
        chandra: make("Chandra", moon, "the Moon".into(), Vec::new()),
        arudha: make(
            "Arudha",
            arudha_lon,
            format!(
                "Arudha sign {} + the lagna's degree",
                SIGN_NAMES[arudha_sign as usize]
            ),
            Vec::new(),
        ),
        arudha_sign,
        veethi_sign,
        chatra_sign,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_chart};

    #[test]
    fn digits_and_planets() {
        let a = ashtamangala(831);
        assert_eq!(a.digits, vec![8, 3, 1]);
        assert_eq!(
            a.planets,
            vec![
                Some(DashaLord::Rahu),
                Some(DashaLord::Jupiter),
                Some(DashaLord::Sun)
            ]
        );
        assert!(a.indications[0].contains("past") && a.indications[0].contains("Rahu"));
        assert!(a.indications[2].contains("odd, read as good"));
        assert!(a.indications.iter().any(|s| s.contains("2 of the three")));
        // 831 mod 12 = 3 → Gemini; 831 mod 27 = 21 → Uttara Ashadha (index 20)
        assert_eq!(a.arudha_sign, 2);
        assert_eq!(a.nakshatra, Some(20));
        assert_eq!(digit_planet(0), None);
        assert_eq!(digit_planet(9), None);
        assert_eq!(digit_planet(7), Some(DashaLord::Moon));
        // remainder zero is Pisces and Revati
        let z = ashtamangala(108);
        assert_eq!(z.arudha_sign, 11);
        assert_eq!(z.nakshatra, Some(26));
        let n = ashtamangala(0);
        assert_eq!(n.nakshatra, None);
        assert!(n.indications[0].contains("not an Ashtamangala digit"));
        assert!(ashtamangala(888).indications.iter().all(|s| !s.is_empty()));
    }

    #[test]
    fn veethi_and_chatra_follow_ramans_example() {
        assert_eq!(veethi_rasi(1), 0); // Sun in Taurus → Aries
        assert_eq!(veethi_rasi(4), 0);
        assert_eq!(veethi_rasi(7), 2); // Scorpio → Gemini
        assert_eq!(veethi_rasi(10), 2);
        assert_eq!(veethi_rasi(11), 1); // Pisces → Taurus
        assert_eq!(veethi_rasi(0), 1);
        assert_eq!(veethi_rasi(5), 1);
        assert_eq!(veethi_rasi(6), 1);
        // Arudha Taurus, lagna Leo, Sun in Aries (Veethi Taurus): count 4 → Leo
        assert_eq!(chatra_rasi(1, 4, 0), 4);
        // Arudha = lagna: count 1 → the Veethi rasi itself
        assert_eq!(chatra_rasi(3, 3, 2), 0);
        // wrap: Arudha Pisces, lagna Aries, Sun in Scorpio (Veethi Gemini): count 2 → Cancer
        assert_eq!(chatra_rasi(11, 0, 7), 3);
    }

    #[test]
    fn kochi_prasna_sphutas() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let p = prasna_sphutas(&ch, 5).unwrap();
        assert_eq!(p.arudha_sign, 5);
        assert_eq!(p.arudha.sign, 5);
        assert!(
            (p.arudha.longitude.rem_euclid(30.0) - p.lagna.longitude.rem_euclid(30.0)).abs() < 1e-9
        );
        assert_eq!(p.lagna.longitude, ch.cusps_sidereal[0]);
        assert_eq!(p.chandra.longitude, ch.bodies[1].sidereal);
        assert_eq!(p.veethi_sign, veethi_rasi(sign_of(ch.bodies[0].sidereal)));
        assert_eq!(
            p.chatra_sign,
            chatra_rasi(5, p.lagna.sign, sign_of(ch.bodies[0].sidereal))
        );
        assert_eq!(p.words.len(), 4);
        assert!(p.words.iter().all(|w| !w.is_empty()));
    }
}
