//! Ayanamsa: the offset between the tropical and sidereal zodiacs.
//!
//! Two kinds of definition, both matching Swiss Ephemeris 2.10.03 (`sweph.h`, table
//! `ayanamsa[]`, and `swi_get_ayanamsa_ex` in `sweph.c`):
//!
//! * **Fixed zero point** (Lahiri and its variants, Raman, Krishnamurti and its 291 CE
//!   variant, Fagan–Bradley, Yukteshwar, Surya Siddhanta). Each is a fixed direction on
//!   the sky: the sidereal zero point. Its tropical longitude at any date is the
//!   ayanamsa at that date. Kshana stores one anchor per ayanamsa (the tropical
//!   longitude of the zero point at a reference instant, on the true equinox and
//!   ecliptic of that instant) and carries that direction to the requested date with
//!   the same IAU 2006/2000A precession-nutation it uses for the planets. The result is
//!   the ayanamsa on the TRUE equinox of date, which is what must be subtracted from
//!   apparent (true-equinox) longitudes. Anchors are the Swiss Ephemeris 2.10.03 values
//!   at 2000-01-01T12:00:00 UTC (`swe_get_ayanamsa_ex_ut`, true equinox), recorded in
//!   `tests/golden/swisseph.json` and `tests/golden/swisseph_ayanamsa.json`; the
//!   comparison tests show the carried values agree with Swiss Ephemeris across
//!   1900–2050 to well under an arcsecond.
//! * **Star held fixed** (True Chitra, True Revati, True Pushya: `SE_SIDM_TRUE_CITRA`,
//!   `SE_SIDM_TRUE_REVATI`, `SE_SIDM_TRUE_PUSHYA`). The ayanamsa is the star's apparent
//!   geocentric longitude of date minus the longitude the definition gives it. The star
//!   is carried from its catalogue place with its full space motion (proper motion,
//!   parallax, radial velocity), seen from the Earth (annual parallax), deflected by the
//!   Sun and displaced by annual aberration, then rotated to the true ecliptic and
//!   equinox of date: what `swe_fixstar` does inside `swi_get_ayanamsa_ex`.

use kshana_frames::precession::{
    apply, apply_transpose, bias_precession_nutation_matrix, rotate_x,
};
use kshana_frames::{mean_obliquity_iau2006, nutation_iau2006a};

/// Named ayanamsa definitions. The Swiss Ephemeris mode each one reproduces is named in
/// its documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ayanamsa {
    /// Chitrapaksha, as adopted by the Indian Calendar Reform Committee (1955) and
    /// carried in the Indian Astronomical Ephemeris: `SE_SIDM_LAHIRI` (1), 23°15′00.658″
    /// on 1956-03-21 0h TT (IAE 1989 p. 556).
    Lahiri,
    /// B. V. Raman, *Hindu Predictive Astrology* (1938): `SE_SIDM_RAMAN` (3).
    Raman,
    /// K. S. Krishnamurti, *Reader 1* pp. 55–59: `SE_SIDM_KRISHNAMURTI` (5).
    Krishnamurti,
    /// Fagan–Bradley (Western sidereal), SVP 335°57′28.64″ at B1950:
    /// `SE_SIDM_FAGAN_BRADLEY` (0).
    FaganBradley,
    /// True Chitra: Spica (α Virginis) held at exactly 180° sidereal, from the star's
    /// actual apparent place: `SE_SIDM_TRUE_CITRA` (27).
    TrueChitra,
    /// True Revati: ζ Piscium held at 359°50′ sidereal: `SE_SIDM_TRUE_REVATI` (28).
    TrueRevati,
    /// True Pushya: δ Cancri (Asellus Australis) held at 106° sidereal (16° Cancer):
    /// `SE_SIDM_TRUE_PUSHYA` (29).
    TruePushya,
    /// Sri Yukteshwar, *The Holy Science* (1894), as defined by D. Cochrane:
    /// `SE_SIDM_YUKTESHWAR` (7), 21°03′ at J1900 (360° − 338.917778°).
    Yukteshwar,
    /// Surya Siddhanta: zero ayanamsa when the mean Sun entered Aries at the mean equinox
    /// on 21 March 499 CE, 7:30:31.57 UT (Ujjain noon): `SE_SIDM_SURYASIDDHANTA` (21).
    SuryaSiddhanta,
    /// Lahiri as first published, *Panchanga Darpan* / Calcutta ephemeris (1940):
    /// 22°26′45.50″ at J1900 with Newcomb precession: `SE_SIDM_LAHIRI_1940` (43).
    Lahiri1940,
    /// Lahiri's later definition (preface to his *Indian Ephemeris*, 1980): mean Sun at
    /// 0° sidereal and at the equinox in 285 CE: `SE_SIDM_LAHIRI_VP285` (44).
    LahiriVp285,
    /// The Calendar Reform Committee value before the 0.658″ IAE correction of 1985,
    /// 23°15′ on 1956-03-21 with Newcomb precession: `SE_SIDM_LAHIRI_ICRC` (46).
    LahiriIcrc,
    /// Krishnamurti from the mean equinox of 291 CE (D. Senthilathiban, *Study of KP
    /// Ayanamsa with Modern Precession Theories*, 2019): `SE_SIDM_KRISHNAMURTI_VP291`
    /// (45), often called the new KP ayanamsa.
    KrishnamurtiVp291,
}

/// TT Julian centuries since J2000.0 of the anchor instant (2000-01-01T12:00:00 UTC).
const ANCHOR_T: f64 = 0.000_742_870_4 / 36_525.0;

/// A catalogue star: ICRS place at epoch J2000.0 and its space motion.
#[derive(Clone, Copy, Debug)]
struct Star {
    /// Right ascension, degrees.
    ra_deg: f64,
    /// Declination, degrees.
    dec_deg: f64,
    /// Proper motion in right ascension times cos δ, milliarcseconds per year.
    pm_ra_cosdec_mas: f64,
    /// Proper motion in declination, milliarcseconds per year.
    pm_dec_mas: f64,
    /// Radial velocity, km/s.
    rv_km_s: f64,
    /// Parallax, milliarcseconds.
    parallax_mas: f64,
    /// Sidereal longitude the definition gives the star, degrees.
    sidereal_deg: f64,
}

const fn hms(h: f64, m: f64, s: f64) -> f64 {
    (h + m / 60.0 + s / 3_600.0) * 15.0
}

const fn dms(sign: f64, d: f64, m: f64, s: f64) -> f64 {
    sign * (d + m / 60.0 + s / 3_600.0)
}

// Catalogue values exactly as Swiss Ephemeris 2.10.03 carries them in `sefstars.txt`
// ("ICRS" records, from SIMBAD: positions and proper motions from the Hipparcos new
// reduction, F. van Leeuwen 2007, A&A 474, 653; radial velocities from SIMBAD), so the
// star-based ayanamsas reproduce SE's.

/// Spica, α Vir (HIP 65474).
const SPICA: Star = Star {
    ra_deg: hms(13.0, 25.0, 11.579_37),
    dec_deg: dms(-1.0, 11.0, 9.0, 40.750_1),
    pm_ra_cosdec_mas: -42.35,
    pm_dec_mas: -30.67,
    rv_km_s: 1.0,
    parallax_mas: 13.06,
    sidereal_deg: 180.0,
};

/// ζ Psc (HIP 5737), Revati's junction star, at 359°50′ (Surya Siddhanta's polar
/// longitude, the value SE uses).
const ZETA_PISCIUM: Star = Star {
    ra_deg: hms(1.0, 13.0, 43.887_35),
    dec_deg: dms(1.0, 7.0, 34.0, 31.274_5),
    pm_ra_cosdec_mas: 145.0,
    pm_dec_mas: -55.69,
    rv_km_s: 15.0,
    parallax_mas: 18.76,
    sidereal_deg: 359.0 + 50.0 / 60.0,
};

/// δ Cnc, Asellus Australis (HIP 42911), Pushya's junction star, at 106°.
const DELTA_CANCRI: Star = Star {
    ra_deg: hms(8.0, 44.0, 41.099_21),
    dec_deg: dms(1.0, 18.0, 9.0, 15.503_4),
    pm_ra_cosdec_mas: -17.67,
    pm_dec_mas: -229.26,
    rv_km_s: 17.14,
    parallax_mas: 24.98,
    sidereal_deg: 106.0,
};

impl Ayanamsa {
    /// Every variant.
    pub const ALL: [Ayanamsa; 13] = [
        Ayanamsa::Lahiri,
        Ayanamsa::Raman,
        Ayanamsa::Krishnamurti,
        Ayanamsa::FaganBradley,
        Ayanamsa::TrueChitra,
        Ayanamsa::TrueRevati,
        Ayanamsa::TruePushya,
        Ayanamsa::Yukteshwar,
        Ayanamsa::SuryaSiddhanta,
        Ayanamsa::Lahiri1940,
        Ayanamsa::LahiriVp285,
        Ayanamsa::LahiriIcrc,
        Ayanamsa::KrishnamurtiVp291,
    ];

    /// The Swiss Ephemeris `SE_SIDM_*` number this definition reproduces.
    pub fn swiss_ephemeris_mode(self) -> i32 {
        match self {
            Ayanamsa::FaganBradley => 0,
            Ayanamsa::Lahiri => 1,
            Ayanamsa::Raman => 3,
            Ayanamsa::Krishnamurti => 5,
            Ayanamsa::Yukteshwar => 7,
            Ayanamsa::SuryaSiddhanta => 21,
            Ayanamsa::TrueChitra => 27,
            Ayanamsa::TrueRevati => 28,
            Ayanamsa::TruePushya => 29,
            Ayanamsa::Lahiri1940 => 43,
            Ayanamsa::LahiriVp285 => 44,
            Ayanamsa::KrishnamurtiVp291 => 45,
            Ayanamsa::LahiriIcrc => 46,
        }
    }

    /// The star a star-based ayanamsa holds fixed.
    fn star(self) -> Option<&'static Star> {
        match self {
            Ayanamsa::TrueChitra => Some(&SPICA),
            Ayanamsa::TrueRevati => Some(&ZETA_PISCIUM),
            Ayanamsa::TruePushya => Some(&DELTA_CANCRI),
            _ => None,
        }
    }

    /// Tropical longitude of the sidereal zero point at the anchor instant, degrees,
    /// true equinox of date: Swiss Ephemeris 2.10.03 `swe_get_ayanamsa_ex_ut` at
    /// 2000-01-01T12:00:00 UTC. Star-based definitions have no anchor.
    fn anchor_deg(self) -> f64 {
        match self {
            Ayanamsa::Lahiri => 23.853_222_486_194_408,
            Ayanamsa::Raman => 22.406_921_172_812_087,
            Ayanamsa::Krishnamurti => 23.756_370_172_812_08,
            Ayanamsa::FaganBradley => 24.736_430_126_920_492,
            Ayanamsa::Yukteshwar => 22.474_933_160_294,
            Ayanamsa::SuryaSiddhanta => 20.891_188_985_102,
            Ayanamsa::Lahiri1940 => 23.838_453_392_812,
            Ayanamsa::LahiriVp285 => 23.859_611_363_129,
            Ayanamsa::LahiriIcrc => 23.852_919_148_771,
            Ayanamsa::KrishnamurtiVp291 => 23.776_495_117_402,
            // not used to compute: SE (SWIEPH) values at the anchor, checked by the tests
            Ayanamsa::TrueChitra => 23.836_148_147_794,
            Ayanamsa::TrueRevati => 20.041_291_824_979,
            Ayanamsa::TruePushya => 22.723_221_853_187,
        }
    }
}

/// Unit vector of an ecliptic-of-date longitude (latitude 0) in the GCRS, for TT
/// centuries `t`.
fn zero_point_gcrs(lon_deg: f64, t: f64) -> [f64; 3] {
    let (dpsi, deps) = nutation_iau2006a(t);
    let eps = mean_obliquity_iau2006(t) + deps;
    let (s, c) = lon_deg.to_radians().sin_cos();
    let ecl = [c, s, 0.0];
    let eq = rotate_x(-eps, ecl);
    let m = bias_precession_nutation_matrix(t, dpsi, deps);
    apply_transpose(&m, eq)
}

/// The GCRS zero point of each fixed ayanamsa, computed once per process.
fn zero_point_cached(which: Ayanamsa) -> [f64; 3] {
    use std::sync::OnceLock;
    static POINTS: OnceLock<Vec<(Ayanamsa, [f64; 3])>> = OnceLock::new();
    let all = POINTS.get_or_init(|| {
        Ayanamsa::ALL
            .iter()
            .map(|a| (*a, zero_point_gcrs(a.anchor_deg(), ANCHOR_T)))
            .collect()
    });
    all.iter()
        .find(|(a, _)| *a == which)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| zero_point_gcrs(which.anchor_deg(), ANCHOR_T))
}

/// Ayanamsa in degrees at `t` Julian centuries of TT since J2000.0, on the true
/// equinox and ecliptic of date.
pub fn ayanamsa_degrees(which: Ayanamsa, t: f64) -> f64 {
    let (dpsi, deps) = kshana_frames::nutation_cached(t);
    let eps = mean_obliquity_iau2006(t) + deps;
    let m = bias_precession_nutation_matrix(t, dpsi, deps);
    if let Some(star) = which.star() {
        let lon = star_apparent_longitude(star, t, &m, eps, dpsi);
        return (lon - star.sidereal_deg).rem_euclid(360.0);
    }
    let v = zero_point_cached(which);
    let eq = apply(&m, v);
    let ecl = rotate_x(eps, eq);
    ecl[1].atan2(ecl[0]).to_degrees().rem_euclid(360.0)
}

/// Astronomical unit, km (IAU 2012 B2).
const AU_KM: f64 = 149_597_870.7;
/// Speed of light, AU per day (c = 299 792.458 km/s, IAU 2012 au).
const C_AU_DAY: f64 = 299_792.458 * 86_400.0 / AU_KM;
/// Schwarzschild radius of the Sun, AU: 2GM☉/(c² au) (ERFA `ERFA_SRS`).
const SRS_AU: f64 = 1.974_125_743_36e-8;
/// AU per parsec: 648 000/π.
const AU_PER_PARSEC: f64 = 648_000.0 / std::f64::consts::PI;
/// km/s to AU/day.
const KM_S_TO_AU_DAY: f64 = 86_400.0 / AU_KM;

/// Geocentric apparent longitude of a catalogue star, degrees, true ecliptic and equinox
/// of date. `m` is the GCRS → true-equator-of-date matrix at `t` and `eps` the true
/// obliquity (radians), `dpsi` the nutation in longitude (radians).
///
/// Space motion follows Swiss Ephemeris `fixstar_calc_from_struct`: the catalogue place
/// at J2000.0 with its distance from the parallax, moved linearly in Cartesian
/// coordinates by the velocity from the proper motions and radial velocity. The Earth's
/// heliocentric position and velocity come from the Sun's elliptic orbit (J. Meeus,
/// *Astronomical Algorithms* 2nd ed., ch. 25 low-accuracy Sun, ≈ 0.01°; ch. 23 for
/// the aberration terms with e and ϖ): this is used only for annual parallax (≤ 25 mas
/// for these stars), light deflection (≤ 0.3″ for Spica at its closest to the Sun) and
/// aberration (20.5″), where it costs under 0.01″. Deflection is ERFA `ld` with the Sun
/// as the deflector; aberration is first order in v/c (the second-order term is
/// < 0.002″).
fn star_apparent_longitude(star: &Star, t: f64, m: &[[f64; 3]; 3], eps: f64, dpsi: f64) -> f64 {
    let days = t * 36_525.0;
    // catalogue place and space motion, GCRS, AU and AU/day
    let (sa, ca) = star.ra_deg.to_radians().sin_cos();
    let (sd, cd) = star.dec_deg.to_radians().sin_cos();
    let mas = (1.0f64 / 3_600_000.0).to_radians();
    let dist = AU_PER_PARSEC / (star.parallax_mas / 1_000.0);
    let u = [cd * ca, cd * sa, sd];
    let east = [-sa, ca, 0.0];
    let north = [-sd * ca, -sd * sa, cd];
    let mu_a = star.pm_ra_cosdec_mas * mas / 365.25 * dist;
    let mu_d = star.pm_dec_mas * mas / 365.25 * dist;
    let vr = star.rv_km_s * KM_S_TO_AU_DAY;
    let mut p = [0.0; 3];
    for k in 0..3 {
        let v = mu_a * east[k] + mu_d * north[k] + vr * u[k];
        p[k] = dist * u[k] + v * days;
    }
    // to the true ecliptic and equinox of date
    let p = rotate_x(eps, apply(m, p));

    // the Sun as seen from the Earth (Meeus ch. 25), mean equinox → true (+Δψ)
    let l0 = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let manom = (357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t).to_radians();
    let e = 0.016_708_634 - 0.000_042_037 * t - 0.000_000_126_7 * t * t;
    let c = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * manom.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * manom).sin()
        + 0.000_289 * (3.0 * manom).sin();
    let sun_lon = (l0 + c).to_radians() + dpsi;
    let nu = manom + c.to_radians();
    let r = 1.000_001_018 * (1.0 - e * e) / (1.0 + e * nu.cos());
    let (ss, cs) = sun_lon.sin_cos();
    let earth = [-r * cs, -r * ss, 0.0];
    // annual parallax: from the Sun to the Earth
    let q = [p[0] - earth[0], p[1] - earth[1], p[2] - earth[2]];
    let qn = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    let mut d = [q[0] / qn, q[1] / qn, q[2] / qn];

    // light deflection by the Sun (ERFA ld, star at infinity: p = q)
    let eh = [-cs, -ss, 0.0]; // unit vector Sun → Earth
    let qe = d[0] * eh[0] + d[1] * eh[1] + d[2] * eh[2];
    let w = SRS_AU / r / (1.0 + qe).max(1e-9);
    for k in 0..3 {
        d[k] += w * (eh[k] - d[k] * qe);
    }

    // annual aberration: the Earth's orbital velocity (Meeus ch. 23: κ and the e, ϖ terms)
    let varpi = (102.937_35 + 1.719_46 * t + 0.000_46 * t * t).to_radians() + dpsi;
    // Earth's mean motion 0.9856091°/day and semi-major axis 1.000001018 AU, so that
    // v/c = n·a/(c·√(1−e²)) = 20.4955″, Meeus's constant of aberration κ
    let n = 0.985_609_1_f64.to_radians();
    let v_mag = n * 1.000_001_018 / (1.0 - e * e).sqrt();
    let (sp, cp) = varpi.sin_cos();
    let v = [
        v_mag * (ss - e * sp) / C_AU_DAY,
        v_mag * (-cs + e * cp) / C_AU_DAY,
        0.0,
    ];
    let dv = d[0] * v[0] + d[1] * v[1] + d[2] * v[2];
    let mut a = [0.0; 3];
    for k in 0..3 {
        a[k] = d[k] + v[k] - d[k] * dv;
    }
    a[1].atan2(a[0]).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_reproduces_itself() {
        for a in Ayanamsa::ALL {
            if a.star().is_some() {
                continue;
            }
            let v = ayanamsa_degrees(a, ANCHOR_T);
            assert!((v - a.anchor_deg()).abs() < 1e-9, "{a:?}: {v}");
        }
    }

    #[test]
    fn lahiri_in_2026_is_about_24_degrees_13_minutes() {
        let a = ayanamsa_degrees(Ayanamsa::Lahiri, 0.268);
        assert!(a > 24.22 && a < 24.24, "{a}");
    }

    #[test]
    fn rate_is_about_fifty_arcseconds_per_year_on_average() {
        // The true-equinox value carries nutation (±17″), so average over four centuries,
        // where the mean rate is general precession in longitude.
        let a0 = ayanamsa_degrees(Ayanamsa::Lahiri, -2.0);
        let a1 = ayanamsa_degrees(Ayanamsa::Lahiri, 2.0);
        let per_year = (a1 - a0) * 3_600.0 / 400.0;
        assert!((per_year - 50.29).abs() < 0.15, "{per_year}");
    }

    /// Each star-based ayanamsa puts its star where the definition says, and the value
    /// at the anchor instant is Swiss Ephemeris's (`swe_get_ayanamsa_ex_ut`, SWIEPH).
    #[test]
    fn star_based_ayanamsas_match_swiss_ephemeris_at_j2000() {
        for a in [
            Ayanamsa::TrueChitra,
            Ayanamsa::TrueRevati,
            Ayanamsa::TruePushya,
        ] {
            let v = ayanamsa_degrees(a, ANCHOR_T);
            let d = (v - a.anchor_deg()).abs() * 3_600.0;
            assert!(d < 0.05, "{a:?}: {v} vs {} ({d:.4}″)", a.anchor_deg());
        }
    }

    /// Pinned Swiss Ephemeris 2.10.03 values (`swe_get_ayanamsa_ex_ut`, SEFLG_SWIEPH,
    /// true equinox; `tests/golden/swisseph_ayanamsa.json`) at 1900-01-01 0h, 1950-01-01
    /// 0h, 2000-01-01 12h and 2026-01-01 0h UTC, given here by their TT Julian dates.
    #[test]
    fn pinned_swiss_ephemeris_values_1900_1950_2000_2026() {
        use Ayanamsa::*;
        let epochs = [
            2_415_020.499_976_958_6,
            2_433_282.500_334_859_8,
            2_451_545.000_742_870_4,
            2_461_041.500_800_740_9,
        ];
        let table: [(Ayanamsa, [f64; 4]); 13] = [
            (
                Lahiri,
                [
                    22.465_373_223_610,
                    23.157_807_576_200,
                    23.853_222_486_194,
                    24.221_809_904_410,
                ],
            ),
            (
                Raman,
                [
                    21.019_071_756_808,
                    21.711_506_194_418,
                    22.406_921_172_812,
                    22.775_508_619_993,
                ],
            ),
            (
                Krishnamurti,
                [
                    22.368_520_756_808,
                    23.060_955_194_418,
                    23.756_370_172_812,
                    24.124_957_619_993,
                ],
            ),
            (
                FaganBradley,
                [
                    23.348_580_846_269,
                    24.041_015_208_809,
                    24.736_430_126_920,
                    25.105_017_548_628,
                ],
            ),
            (
                TrueChitra,
                [
                    22.449_595_138_680,
                    23.141_359_636_072,
                    23.836_148_147_794,
                    24.204_412_581_716,
                ],
            ),
            (
                TrueRevati,
                [
                    18.650_140_827_402,
                    19.344_248_204_456,
                    20.041_291_824_979,
                    20.410_723_708_335,
                ],
            ),
            (
                TruePushya,
                [
                    21.334_223_488_396,
                    22.027_220_103_293,
                    22.723_221_853_187,
                    23.092_111_258_265,
                ],
            ),
            (
                Yukteshwar,
                [
                    21.087_083_744_290,
                    21.779_518_181_900,
                    22.474_933_160_294,
                    22.843_520_607_475,
                ],
            ),
            (
                SuryaSiddhanta,
                [
                    19.503_343_233_083,
                    20.195_776_053_844,
                    20.891_188_985_102,
                    21.259_775_197_960,
                ],
            ),
            (
                Lahiri1940,
                [
                    22.450_603_976_808,
                    23.143_038_414_418,
                    23.838_453_392_812,
                    24.207_040_839_993,
                ],
            ),
            (
                LahiriVp285,
                [
                    22.471_767_478_892,
                    23.164_199_398_819,
                    23.859_611_363_129,
                    24.228_197_047_272,
                ],
            ),
            (
                LahiriIcrc,
                [
                    22.465_069_886_187,
                    23.157_504_238_777,
                    23.852_919_148_771,
                    24.221_506_566_987,
                ],
            ),
            (
                KrishnamurtiVp291,
                [
                    22.388_651_176_031,
                    23.081_083_123_603,
                    23.776_495_117_402,
                    24.145_080_817_602,
                ],
            ),
        ];
        for (a, want) in table {
            for (jd, w) in epochs.iter().zip(want) {
                let t = (jd - 2_451_545.0) / 36_525.0;
                let d = (ayanamsa_degrees(a, t) - w).abs() * 3_600.0;
                assert!(d < 0.05, "{a:?} at JD {jd}: off by {d:.4}″");
            }
        }
    }

    /// True Chitra swings with annual aberration (±20″ over the year), unlike a fixed
    /// zero point, which only carries nutation.
    #[test]
    fn true_chitra_carries_annual_aberration() {
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for i in 0..24 {
            let t = 0.26 + (i as f64) / 24.0 / 100.0;
            let d = (ayanamsa_degrees(Ayanamsa::TrueChitra, t)
                - ayanamsa_degrees(Ayanamsa::Lahiri, t))
                * 3_600.0;
            lo = lo.min(d);
            hi = hi.max(d);
        }
        assert!(hi - lo > 30.0 && hi - lo < 45.0, "{lo} {hi}");
    }
}
