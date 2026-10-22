//! Chesta bala: motional strength.
//!
//! BPHS 27.21–25 names eight states of motion and their strengths: Vakra (retrograde)
//! 60, Anuvakra (retrograde and entered the previous sign) 30, Vikala (motionless) 15,
//! Manda (slow) 30, Mandatara (slower) 15, Sama (even) 7.5, Chara (fast) 45, Atichara
//! (fast and entered the next sign) 30. The Sun and the Moon have no Chesta bala of
//! their own: the Sun takes its Ayana bala and the Moon its Paksha bala (BPHS 27.21;
//! Raman).
//!
//! # Convention
//!
//! The states are graded from the apparent daily motion `v` against the graha's mean
//! daily motion `m` ([`crate::tables::mean_motion`]), with no mean-longitude theory:
//!
//! * `v < 0`: Vakra, or Anuvakra when the graha has crossed back into the previous sign
//!   since it turned retrograde (found by stepping the ephemeris back a day at a time
//!   while the motion stays retrograde);
//! * `|v| < m / 10`: Vikala;
//! * `m / 10 ≤ v < m / 2`: Mandatara;
//! * `m / 2 ≤ v < 0.9 m`: Manda;
//! * `0.9 m ≤ v ≤ 1.1 m`: Sama;
//! * `v > 1.1 m`: Chara, or Atichara when the graha has crossed into a new sign since the
//!   motion last fell to the mean (the same backward walk while `v > m`).
//!
//! The bands are this crate's; the texts give only the words. Raman and Jagannatha Hora
//! compute instead a *chesta kendra* from the mean Sun and the planet's mean longitude
//! (Raman's "seeghrochcha" method, PyJHora `_cheshta_bala_new`); that formula is offered
//! as [`chesta_kendra_bala`] for a caller who has the mean longitudes, and its result is
//! continuous from 0 to 60 rather than one of the eight steps.

use crate::BalaError;
use kshana_rules::Lang;
use kshana_ephem::{Body, Spk};
use kshana_sky::apparent;
use kshana_time::{Instant, NS_PER_DAY};
use kshana_vedic::{ayanamsa_degrees, Ayanamsa, DashaLord};

/// The eight motional states of BPHS 27.21–23.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Retrograde.
    Vakra,
    /// Retrograde and gone back into the previous sign.
    Anuvakra,
    /// Motionless (at a station).
    Vikala,
    /// Slower than the mean.
    Manda,
    /// Much slower than the mean.
    Mandatara,
    /// Even with the mean.
    Sama,
    /// Faster than the mean.
    Chara,
    /// Fast and gone on into the next sign.
    Atichara,
}

impl Motion {
    /// Virupas for the state (BPHS 27.24–25).
    pub fn virupas(self) -> f64 {
        match self {
            Motion::Vakra => 60.0,
            Motion::Anuvakra => 30.0,
            Motion::Vikala => 15.0,
            Motion::Manda => 30.0,
            Motion::Mandatara => 15.0,
            Motion::Sama => 7.5,
            Motion::Chara => 45.0,
            Motion::Atichara => 30.0,
        }
    }

    /// Sanskrit name.
    pub fn name(self) -> &'static str {
        match self {
            Motion::Vakra => "Vakra",
            Motion::Anuvakra => "Anuvakra",
            Motion::Vikala => "Vikala",
            Motion::Manda => "Manda",
            Motion::Mandatara => "Mandatara",
            Motion::Sama => "Sama",
            Motion::Chara => "Chara",
            Motion::Atichara => "Atichara",
        }
    }

    /// The state's name in the reader's language: "Vakra", "വക്രം", "वक्र".
    pub fn name_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Motion::Vakra => ["Vakra", "വക്രം", "वक्र"],
            Motion::Anuvakra => ["Anuvakra", "അനുവക്രം", "अनुवक्र"],
            Motion::Vikala => ["Vikala", "വികലം", "विकल"],
            Motion::Manda => ["Manda", "മന്ദം", "मंद"],
            Motion::Mandatara => ["Mandatara", "മന്ദതരം", "मंदतर"],
            Motion::Sama => ["Sama", "സമം", "सम"],
            Motion::Chara => ["Chara", "ചരം", "चर"],
            Motion::Atichara => ["Atichara", "അതിചാരം", "अतिचार"],
        })
    }

    /// What the state is, in a few words.
    pub fn meaning_in(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Motion::Vakra => ["retrograde", "വക്രഗതി", "वक्री"],
            Motion::Anuvakra => ["retrograde into the previous sign", "മുൻരാശിയിലേക്കു വക്രിച്ചത്", "पिछली राशि में वक्री"],
            Motion::Vikala => ["stationary", "നിശ്ചലം", "स्थिर"],
            Motion::Manda => ["slow", "മന്ദഗതി", "धीमी गति"],
            Motion::Mandatara => ["very slow", "വളരെ മന്ദഗതി", "बहुत धीमी गति"],
            Motion::Sama => ["even", "സമഗതി", "सम गति"],
            Motion::Chara => ["fast", "ശീഘ്രഗതി", "तेज़ गति"],
            Motion::Atichara => ["very fast", "അതിശീഘ്രഗതി", "बहुत तेज़ गति"],
        })
    }
}

/// Grade a motion. `crossed_sign` says whether the graha changed sign during the current
/// retrograde (if `speed < 0`) or fast (if `speed > 1.1 × mean`) spell; it is ignored for
/// the other states.
pub fn motion_state(speed: f64, mean_motion: f64, crossed_sign: bool) -> Motion {
    let r = speed / mean_motion;
    if speed < 0.0 {
        if r.abs() < 0.1 {
            Motion::Vikala
        } else if crossed_sign {
            Motion::Anuvakra
        } else {
            Motion::Vakra
        }
    } else if r < 0.1 {
        Motion::Vikala
    } else if r < 0.5 {
        Motion::Mandatara
    } else if r < 0.9 {
        Motion::Manda
    } else if r <= 1.1 {
        Motion::Sama
    } else if crossed_sign {
        Motion::Atichara
    } else {
        Motion::Chara
    }
}

/// Raman's chesta kendra formula as JHora implements it: the seeghrochcha less the mean
/// of the mean and true longitudes, folded to at most 180°, over three. For Mars,
/// Jupiter and Saturn the seeghrochcha is the mean Sun and `mean` the planet's mean
/// longitude; for Mercury and Venus the seeghrochcha is the planet's own mean longitude
/// and `mean` is the mean Sun (Raman, *Graha and Bhava Balas*, Chesta bala; PyJHora
/// `_cheshta_bala_new`). All three in the same zodiac, degrees.
pub fn chesta_kendra_bala(seeghrochcha: f64, mean: f64, true_longitude: f64) -> f64 {
    let m = mean.rem_euclid(360.0);
    let t = true_longitude.rem_euclid(360.0);
    // mean of two longitudes along the shorter arc
    let half = ((t - m + 180.0).rem_euclid(360.0) - 180.0) / 2.0;
    let average = (m + half).rem_euclid(360.0);
    let kendra = (seeghrochcha - average).rem_euclid(360.0);
    let kendra = if kendra > 180.0 {
        360.0 - kendra
    } else {
        kendra
    };
    kendra / 3.0
}

fn body_of(g: DashaLord) -> Option<Body> {
    Some(match g {
        DashaLord::Mars => Body::MarsBarycenter,
        DashaLord::Mercury => Body::Mercury,
        DashaLord::Jupiter => Body::JupiterBarycenter,
        DashaLord::Venus => Body::Venus,
        DashaLord::Saturn => Body::SaturnBarycenter,
        _ => return None,
    })
}

fn sidereal_sign(tropical: f64, t: Instant, ay: Ayanamsa) -> u8 {
    let s = (tropical - ayanamsa_degrees(ay, t.tt_centuries_since_j2000())).rem_euclid(360.0);
    (s / 30.0).floor() as u8
}

/// Walk back a day at a time while `spell(speed)` holds (at most `max_days`) and report
/// whether the sidereal sign differed from `sign_now` at any step.
fn crossed_sign_in_spell(
    spk: &Spk,
    body: Body,
    t: Instant,
    ay: Ayanamsa,
    sign_now: u8,
    spell: impl Fn(f64) -> bool,
    max_days: u32,
) -> Result<bool, BalaError> {
    for k in 1..=max_days {
        let tk = t.add_ns(-(k as i128) * NS_PER_DAY);
        let a = apparent(spk, body, tk)?;
        if !spell(a.speed) {
            return Ok(false);
        }
        if sidereal_sign(a.longitude, tk, ay) != sign_now {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Motional state and Chesta bala of one of the five planets at `t`, from its apparent
/// daily motion `speed` and sidereal longitude. `None` for the Sun, the Moon and the
/// nodes, whose Chesta bala is taken elsewhere.
pub fn chesta_of(
    spk: &Spk,
    g: DashaLord,
    sidereal: f64,
    speed: f64,
    t: Instant,
    ay: Ayanamsa,
) -> Result<Option<(Motion, f64)>, BalaError> {
    let (Some(body), Some(m)) = (body_of(g), crate::tables::mean_motion(g)) else {
        return Ok(None);
    };
    let sign_now = (sidereal.rem_euclid(360.0) / 30.0).floor() as u8;
    let crossed = if speed < 0.0 {
        crossed_sign_in_spell(spk, body, t, ay, sign_now, |v| v < 0.0, 200)?
    } else if speed > 1.1 * m {
        crossed_sign_in_spell(spk, body, t, ay, sign_now, |v| v > m, 400)?
    } else {
        false
    };
    let state = motion_state(speed, m, crossed);
    Ok(Some((state, state.virupas())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_by_speed_ratio() {
        let m = 0.524;
        assert_eq!(motion_state(-0.3, m, false), Motion::Vakra);
        assert_eq!(motion_state(-0.3, m, true), Motion::Anuvakra);
        assert_eq!(motion_state(-0.01, m, false), Motion::Vikala);
        assert_eq!(motion_state(0.02, m, false), Motion::Vikala);
        assert_eq!(motion_state(0.2, m, false), Motion::Mandatara);
        assert_eq!(motion_state(0.4, m, false), Motion::Manda);
        assert_eq!(motion_state(0.524, m, false), Motion::Sama);
        assert_eq!(motion_state(0.7, m, false), Motion::Chara);
        assert_eq!(motion_state(0.7, m, true), Motion::Atichara);
        assert_eq!(Motion::Vakra.virupas(), 60.0);
        assert_eq!(Motion::Sama.virupas(), 7.5);
    }

    #[test]
    fn chesta_kendra_folds_and_divides_by_three() {
        // Raman-style example: Mars mean 266.34, true 229.50, seeghrochcha (mean Sun) 181.23
        // average 247.92, kendra 66.69 → 22.23
        let v = chesta_kendra_bala(181.23, 266.34, 229.50);
        assert!((v - 22.23).abs() < 0.01, "{v}");
        // a kendra over 180 is folded
        assert!((chesta_kendra_bala(0.0, 200.0, 200.0) - 160.0 / 3.0).abs() < 1e-9);
        assert_eq!(chesta_kendra_bala(100.0, 100.0, 100.0), 0.0);
    }
}
