//! Jaimini strength of signs and of co-lords, shared by the Narayana, Sthira, Shoola and
//! Drig dashas.
//!
//! # Sources
//!
//! The rules are those of P. V. R. Narasimha Rao, *Vedic Astrology: An Integrated
//! Approach* (Sagar, 2000), chapter "Strength of Rasis and Grahas" (the Jaimini rules of
//! *Upadesa Sutras* 2.3, as taught by Sanjay Rath), in the order PyJHora implements them
//! (`src/jhora/horoscope/chart/house.py`: `stronger_rasi`,
//! `stronger_rasi_from_planet_positions`, `stronger_planet`, `_stronger_planet_new`,
//! `house_owner_from_planet_positions`, `brahma`), which is written to reproduce
//! Jagannatha Hora.
//!
//! Stronger of two signs ([`stronger_rasi`]), first rule that decides:
//! 1. the sign holding more grahas (the lagna is not a graha);
//! 2. the sign conjoined or aspected (rasi drishti) by more of Jupiter, Mercury and the
//!    sign's own lord (each counts once for conjoining and once for aspecting);
//! 3. the sign holding an exalted graha when the other holds none;
//! 4. the sign whose lord sits in a sign of the other oddity (odd ↔ even);
//! 5. by nature: dual > fixed > movable;
//! 6. the sign whose lord (co-lords resolved by [`sign_lord`]) has advanced further in
//!    its own sign; on equality the second sign.
//!
//! In rules 2 and 4 the lord is the traditional one (Mars for Scorpio, Saturn for
//! Aquarius), as PyJHora's `const.house_owners`. Convention (deviation): PyJHora rule 2
//! compares the lord's *graha index* with the sign index (`[p_to_h[Me], p_to_h[Ju],
//! lord].count(rasi)`), which is an indexing slip; the intended "lord placed in the sign"
//! is used here.
//!
//! Stronger of two grahas ([`stronger_graha`]), first rule that decides:
//! 0. (co-lords of Scorpio Mars/Ketu or Aquarius Saturn/Rahu only) if one co-lord
//!    occupies the co-owned sign and the other does not, the *other* is the stronger
//!    (it is the one that "carries" the sign's significations elsewhere);
//! 1. the graha conjoined by more grahas;
//! 2. the graha conjoined or aspected (rasi drishti) by more of Jupiter, Mercury and its
//!    dispositor (the traditional lord of the sign it occupies);
//! 3. the exalted graha when the other is not exalted;
//! 4. by the nature of the occupied sign: dual > fixed > movable;
//! 5a. (when judging a co-lord during dasha calculation) the co-lord that gives the
//!     sign the longer Narayana dasha;
//! 5b. the graha further advanced in its sign; on equality the first graha.
//!
//! Conventions (deviations from PyJHora, stated): PyJHora's rule 1 counts the lagna as a
//! co-occupant and its rule 3 compares the second graha with itself (so it never fires
//! for the second); both are treated as slips and the textual rule is used. PyJHora rule
//! 5a evaluates the Narayana length of the sign the co-lord *occupies*; here it is the
//! length the co-owned sign would get with that co-lord as its lord, which is what PVR's
//! text ("the co-lord giving the longer dasa") describes.
//!
//! Rasi drishti (Jaimini, *Upadesa Sutras* 1.1.2–1.1.4): a movable sign aspects the
//! fixed signs except the one next to it, a fixed sign the movable signs except the one
//! next to it, a dual sign the other three dual signs.
//!
//! Dignity table ([`DIGNITY`]): PyJHora `const.house_strengths_of_planets` (PVR's
//! dignity table; Rahu exalted in Taurus and Gemini, debilitated in Scorpio and
//! Sagittarius; Ketu the reverse; Mercury's Virgo counted as own sign, not exaltation).
//!
//! Graha indices throughout: 0 Sun, 1 Moon, 2 Mars, 3 Mercury, 4 Jupiter, 5 Venus,
//! 6 Saturn, 7 Rahu, 8 Ketu.

use super::sign_of;

/// Graha index of the Sun.
pub const SUN: usize = 0;
/// Graha index of Mars.
pub const MARS: usize = 2;
/// Graha index of Mercury.
pub const MERCURY: usize = 3;
/// Graha index of Jupiter.
pub const JUPITER: usize = 4;
/// Graha index of Saturn.
pub const SATURN: usize = 6;
/// Graha index of Rahu.
pub const RAHU: usize = 7;
/// Graha index of Ketu.
pub const KETU: usize = 8;

/// Dignity value: debilitated.
pub const DEBILITATED: u8 = 0;
/// Dignity value: enemy's sign.
pub const ENEMY: u8 = 1;
/// Dignity value: neutral sign.
pub const NEUTRAL: u8 = 2;
/// Dignity value: friend's sign.
pub const FRIEND: u8 = 3;
/// Dignity value: exalted.
pub const EXALTED: u8 = 4;
/// Dignity value: own sign.
pub const OWN: u8 = 5;

/// Dignity of each graha (rows Sun..Ketu) in each sign (columns Aries..Pisces), PyJHora
/// `const.house_strengths_of_planets`.
pub const DIGNITY: [[u8; 12]; 9] = [
    // Ar Ta Ge Cn Le Vi Li Sc Sg Cp Aq Pi
    [4, 1, 2, 2, 5, 2, 0, 3, 3, 1, 1, 3], // Sun
    [2, 4, 3, 5, 3, 3, 2, 0, 2, 2, 2, 2], // Moon
    [5, 2, 1, 0, 3, 1, 2, 5, 3, 4, 2, 3], // Mars
    [2, 3, 5, 1, 3, 5, 3, 2, 2, 2, 2, 0], // Mercury
    [3, 1, 1, 4, 3, 1, 1, 3, 5, 0, 2, 5], // Jupiter
    [2, 5, 3, 1, 1, 0, 5, 2, 3, 3, 3, 4], // Venus
    [0, 3, 3, 1, 1, 3, 4, 1, 2, 5, 5, 2], // Saturn
    [1, 4, 4, 1, 1, 3, 3, 0, 0, 3, 1, 3], // Rahu
    [1, 0, 0, 1, 1, 3, 3, 4, 4, 3, 1, 3], // Ketu
];

/// Traditional lord of each sign (graha index): Mars for Scorpio, Saturn for Aquarius.
pub const TRADITIONAL_LORD: [usize; 12] = [2, 5, 3, 1, 0, 3, 5, 2, 4, 6, 6, 4];

/// True for the movable signs Aries, Cancer, Libra, Capricorn.
pub const fn is_movable(sign: usize) -> bool {
    sign % 3 == 0
}

/// True for the fixed signs Taurus, Leo, Scorpio, Aquarius.
pub const fn is_fixed(sign: usize) -> bool {
    sign % 3 == 1
}

/// True for the dual signs Gemini, Virgo, Sagittarius, Pisces.
pub const fn is_dual(sign: usize) -> bool {
    sign % 3 == 2
}

/// True for the odd signs Aries, Gemini, Leo, Libra, Sagittarius, Aquarius.
pub const fn is_odd_sign(sign: usize) -> bool {
    sign % 2 == 0
}

/// The three signs `sign` aspects by rasi drishti, in zodiacal order after it.
pub fn rasi_aspects(sign: usize) -> [usize; 3] {
    let s = sign % 12;
    let mut out = [0usize; 3];
    let mut k = 0;
    for step in 1..12 {
        let t = (s + step) % 12;
        let aspected = if is_movable(s) {
            is_fixed(t) && step != 1 && step != 11
        } else if is_fixed(s) {
            is_movable(t) && step != 1 && step != 11
        } else {
            is_dual(t)
        };
        if aspected {
            out[k] = t;
            k += 1;
        }
    }
    out
}

/// True when `a` and `b` aspect each other by rasi drishti (the relation is mutual).
pub fn rasi_aspects_sign(a: usize, b: usize) -> bool {
    rasi_aspects(a).contains(&(b % 12))
}

/// A rasi chart reduced to what the Jaimini rules read: the lagna sign and each graha's
/// sign and degrees within the sign.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasiChart {
    /// Lagna sign, 0 = Aries.
    pub lagna: usize,
    /// Sign of each graha, Sun..Ketu.
    pub sign: [usize; 9],
    /// Degrees within the sign of each graha.
    pub deg: [f64; 9],
}

impl RasiChart {
    /// From sidereal longitudes: the lagna and the nine grahas Sun, Moon, Mars, Mercury,
    /// Jupiter, Venus, Saturn, Rahu, Ketu.
    pub fn new(lagna_sidereal: f64, grahas: &[f64; 9]) -> Self {
        Self {
            lagna: sign_of(lagna_sidereal),
            sign: std::array::from_fn(|i| sign_of(grahas[i])),
            deg: std::array::from_fn(|i| grahas[i].rem_euclid(360.0).rem_euclid(30.0)),
        }
    }

    /// Number of grahas in `sign`.
    pub fn occupants(&self, sign: usize) -> usize {
        self.sign.iter().filter(|s| **s == sign % 12).count()
    }

    /// True when graha `g` is exalted.
    pub fn exalted(&self, g: usize) -> bool {
        DIGNITY[g][self.sign[g]] == EXALTED
    }

    /// True when graha `g` is debilitated.
    pub fn debilitated(&self, g: usize) -> bool {
        DIGNITY[g][self.sign[g]] == DEBILITATED
    }

    /// How many of Jupiter, Mercury and `lord` occupy or aspect (rasi drishti) `sign`;
    /// each graha counts once for occupying and once for aspecting.
    fn jupiter_mercury_lord_count(&self, sign: usize, lord: usize) -> usize {
        [JUPITER, MERCURY, lord]
            .iter()
            .map(|g| {
                let s = self.sign[*g];
                usize::from(s == sign) + usize::from(rasi_aspects_sign(s, sign))
            })
            .sum()
    }
}

/// Natural strength rank of a sign: dual 3, fixed 2, movable 1.
const fn nature_rank(sign: usize) -> u8 {
    if is_dual(sign) {
        3
    } else if is_fixed(sign) {
        2
    } else {
        1
    }
}

/// The co-lord pair of Scorpio (Mars, Ketu) or Aquarius (Saturn, Rahu), traditional lord
/// first.
pub const fn co_lords(sign: usize) -> Option<(usize, usize)> {
    match sign % 12 {
        7 => Some((MARS, KETU)),
        10 => Some((SATURN, RAHU)),
        _ => None,
    }
}

/// Stronger of grahas `a` and `b` by the rules in the module notes. `co_owned` is the
/// sign when `a`, `b` are its co-lords (enables rule 0, and rule 5a when `during_dasha`).
pub fn stronger_graha(
    chart: &RasiChart,
    a: usize,
    b: usize,
    co_owned: Option<usize>,
    during_dasha: bool,
) -> usize {
    if a == b {
        return a;
    }
    let (sa, sb) = (chart.sign[a], chart.sign[b]);
    // Rule 0: a co-lord in the co-owned sign yields to the one outside it.
    if let Some(s) = co_owned {
        if sa == s && sb != s {
            return b;
        }
        if sb == s && sa != s {
            return a;
        }
    }
    // Rule 1: conjoined by more grahas.
    let (ca, cb) = (chart.occupants(sa) - 1, chart.occupants(sb) - 1);
    if ca != cb {
        return if ca > cb { a } else { b };
    }
    // Rule 2: Jupiter, Mercury and the dispositor conjoining or aspecting.
    let ja = chart.jupiter_mercury_lord_count(sa, TRADITIONAL_LORD[sa]);
    let jb = chart.jupiter_mercury_lord_count(sb, TRADITIONAL_LORD[sb]);
    if ja != jb {
        return if ja > jb { a } else { b };
    }
    // Rule 3: exaltation.
    let (ea, eb) = (chart.exalted(a), chart.exalted(b));
    if ea != eb {
        return if ea { a } else { b };
    }
    // Rule 4: dual > fixed > movable.
    let (na, nb) = (nature_rank(sa), nature_rank(sb));
    if na != nb {
        return if na > nb { a } else { b };
    }
    // Rule 5a: the longer Narayana dasha for the co-owned sign.
    if let (Some(s), true) = (co_owned, during_dasha) {
        let (ya, yb) = (
            super::narayana::years_with_lord(chart, s, a),
            super::narayana::years_with_lord(chart, s, b),
        );
        if ya != yb {
            return if ya > yb { a } else { b };
        }
    }
    // Rule 5b: further advanced in its sign.
    if chart.deg[b] > chart.deg[a] {
        b
    } else {
        a
    }
}

/// Lord of `sign`, with the co-lords of Scorpio and Aquarius resolved by
/// [`stronger_graha`]. `during_dasha` enables rule 5a (PyJHora `check_during_dhasa`).
pub fn sign_lord(chart: &RasiChart, sign: usize, during_dasha: bool) -> usize {
    let sign = sign % 12;
    match co_lords(sign) {
        Some((a, b)) => stronger_graha(chart, a, b, Some(sign), during_dasha),
        None => TRADITIONAL_LORD[sign],
    }
}

/// Stronger of signs `r1` and `r2` by the six rules in the module notes.
pub fn stronger_rasi(chart: &RasiChart, r1: usize, r2: usize) -> usize {
    let (r1, r2) = (r1 % 12, r2 % 12);
    if r1 == r2 {
        return r1;
    }
    // Rule 1: more grahas.
    let (o1, o2) = (chart.occupants(r1), chart.occupants(r2));
    if o1 != o2 {
        return if o1 > o2 { r1 } else { r2 };
    }
    // Rule 2: Jupiter, Mercury and the sign's lord conjoining or aspecting.
    let (l1, l2) = (TRADITIONAL_LORD[r1], TRADITIONAL_LORD[r2]);
    let (j1, j2) = (
        chart.jupiter_mercury_lord_count(r1, l1),
        chart.jupiter_mercury_lord_count(r2, l2),
    );
    if j1 != j2 {
        return if j1 > j2 { r1 } else { r2 };
    }
    // Rule 3: holds an exalted graha.
    let ex = |r: usize| (0..9).any(|g| chart.sign[g] == r && chart.exalted(g));
    let (e1, e2) = (ex(r1), ex(r2));
    if e1 != e2 {
        return if e1 { r1 } else { r2 };
    }
    // Rule 4: lord in a sign of the other oddity.
    let other = |r: usize, l: usize| is_odd_sign(r) != is_odd_sign(chart.sign[l]);
    let (d1, d2) = (other(r1, l1), other(r2, l2));
    if d1 != d2 {
        return if d1 { r1 } else { r2 };
    }
    // Rule 5: dual > fixed > movable.
    let (n1, n2) = (nature_rank(r1), nature_rank(r2));
    if n1 != n2 {
        return if n1 > n2 { r1 } else { r2 };
    }
    // Rule 6: lord further advanced in its sign.
    let (g1, g2) = (sign_lord(chart, r1, false), sign_lord(chart, r2, false));
    if chart.deg[g1] > chart.deg[g2] {
        r1
    } else {
        r2
    }
}

/// Brahma (the "creator" graha of Sthira dasha), PVR ch. on Brahma/Rudra/Maheswara and
/// PyJHora `house.brahma`: among the lords of the 6th, 8th and 12th from the stronger of
/// the lagna and the 7th (Rahu and Ketu excluded), score one point each for a friendly,
/// own or exalted sign, an odd sign, and a place in the first seven houses from that
/// stronger sign; the stronger ([`stronger_graha`]) of the two best scorers (ties keep
/// the 6th, 8th, 12th order) is Brahma.
pub fn brahma(chart: &RasiChart) -> usize {
    let sp = stronger_rasi(chart, chart.lagna, chart.lagna + 6);
    let mut lords: Vec<usize> = Vec::with_capacity(3);
    for h in [5, 7, 11] {
        let l = sign_lord(chart, sp + h, false);
        if l != RAHU && l != KETU && !lords.contains(&l) {
            lords.push(l);
        }
    }
    let score = |g: usize| {
        let s = chart.sign[g];
        u8::from(DIGNITY[g][s] >= FRIEND)
            + u8::from(is_odd_sign(s))
            + u8::from((s + 12 - sp) % 12 < 7)
    };
    let mut scored: Vec<(usize, u8)> = lords.iter().map(|g| (*g, score(*g))).collect();
    scored.sort_by(|x, y| y.1.cmp(&x.1)); // stable: ties keep the 6th/8th/12th order
    match scored.as_slice() {
        [] => SUN, // unreachable: at most two of the three lords can be nodes
        [(g, _)] => *g,
        [(a, _), (b, _), ..] => stronger_graha(chart, *a, *b, None, false),
    }
}

/// Kochi test chart, 1990-02-20 05:04 UTC, 9.9312° N 76.2673° E, Lahiri, sidereal
/// degrees from this engine's DE421 pipeline (`kshana-chart`, the same chart pinned in
/// its `kochi_1990_02_20_birth_chart_and_dasha_balance` test; mean node): lagna 13°14′
/// Aries; Sun 7°34′ Aquarius, Moon 4°30′ Sagittarius, Mars 21°58′ Sagittarius, Mercury
/// 17°55′ Capricorn, Jupiter 7°07′ Gemini, Venus 29°48′ Sagittarius, Saturn 27°33′
/// Sagittarius, Rahu 22°05′ Capricorn, Ketu 22°05′ Cancer.
#[cfg(test)]
pub(crate) const KOCHI_LAGNA: f64 = 13.235_302;
/// Kochi grahas, Sun..Ketu (see [`KOCHI_LAGNA`]).
#[cfg(test)]
pub(crate) const KOCHI_GRAHAS: [f64; 9] = [
    307.572_601,
    244.505_682,
    261.967_589,
    287.920_751,
    67.120_337,
    269.803_134,
    267.550_317,
    292.079_937,
    112.079_937,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kochi_strengths_by_hand() {
        let c = RasiChart::new(KOCHI_LAGNA, &KOCHI_GRAHAS);
        // Aries vs Libra: no occupants (1), no Jupiter/Mercury/lord contact (2: Aries is
        // aspected from Leo, Scorpio, Aquarius; Libra from Taurus, Leo, Aquarius — Mars,
        // Venus, Jupiter, Mercury are in Sagittarius, Sagittarius, Gemini, Capricorn), no
        // exalted graha (3), both lords in an odd sign like their own (4), both movable
        // (5); rule 6: Venus 29°48′ beats Mars 21°58′ → Libra.
        assert_eq!(stronger_rasi(&c, 0, 6), 6);
        // Scorpio: neither co-lord in it; Mars has three companions, Ketu none → Mars.
        assert_eq!(sign_lord(&c, 7, false), MARS);
        // Aquarius: Saturn three companions, Rahu one (Mercury) → Saturn.
        assert_eq!(sign_lord(&c, 10, false), SATURN);
        // Taurus vs Scorpio (2nd and 8th): both empty; each aspected by Mercury from
        // Capricorn (1 each); no exalted graha; both lords in an odd sign (4 ties); both
        // fixed; rule 6: Venus 29°48′ over Mars 21°58′ → Taurus.
        assert_eq!(stronger_rasi(&c, 1, 7), 1);
        // Brahma: lords of the 6th, 8th, 12th from Libra are Jupiter (Pisces), Venus
        // (Taurus), Mercury (Virgo). Scores: Jupiter in Gemini 0+1+0 = 1, Venus in
        // Sagittarius (friend, odd, 3rd from Libra) 3, Mercury in Capricorn 0+0+1 = 1.
        // Best two Venus, Jupiter; Venus has three companions → Brahma is Venus.
        assert_eq!(brahma(&c), 5);
    }

    #[test]
    fn rasi_drishti_by_nature() {
        assert_eq!(rasi_aspects(0), [4, 7, 10]); // Aries: Leo, Scorpio, Aquarius
        assert_eq!(rasi_aspects(1), [3, 6, 9]); // Taurus: Cancer, Libra, Capricorn
        assert_eq!(rasi_aspects(9), [1, 4, 7]); // Capricorn: Taurus, Leo, Scorpio
        assert_eq!(rasi_aspects(10), [0, 3, 6]); // Aquarius: Aries, Cancer, Libra
        assert_eq!(rasi_aspects(8), [11, 2, 5]); // Sagittarius: the other duals
        for a in 0..12 {
            for b in 0..12 {
                assert_eq!(rasi_aspects_sign(a, b), rasi_aspects_sign(b, a), "{a} {b}");
            }
        }
    }

    #[test]
    fn stronger_rasi_rules_in_order() {
        // Rule 1: Sun and Moon in Leo, nothing in Aquarius → Leo.
        let mut g = [125.0, 130.0, 10.0, 70.0, 250.0, 40.0, 280.0, 100.0, 280.0];
        let c = RasiChart::new(0.0, &g);
        assert_eq!(stronger_rasi(&c, 4, 10), 4);
        assert_eq!(stronger_rasi(&c, 10, 4), 4);
        // Rule 2: both empty. Gemini: Jupiter, Mercury (Libra) and its lord Mercury
        // neither occupy nor aspect it → 0. Leo aspects Aries, Libra, Capricorn: Jupiter
        // and Mercury in Libra and the lord Sun in Aries → 3. Leo wins.
        g = [5.0, 5.0, 5.0, 185.0, 185.0, 185.0, 5.0, 95.0, 275.0];
        let c = RasiChart::new(0.0, &g);
        assert_eq!(stronger_rasi(&c, 2, 4), 4);
        // Rule 5: Gemini (dual) over Libra (movable)? Libra holds three grahas → rule 1.
        assert_eq!(stronger_rasi(&c, 2, 6), 6);
        // Rule 5 proper: empty Virgo (dual) vs empty Taurus (fixed). Virgo: Mercury, its
        // lord, and Jupiter in Libra → 0. Taurus aspects Cancer, Libra, Capricorn: Jupiter,
        // Mercury and its lord Venus in Libra → 3. Taurus wins at rule 2.
        assert_eq!(stronger_rasi(&c, 5, 1), 1);
        // Rule 5 proper: every graha in Cancer, which aspects Taurus, Scorpio, Aquarius.
        // Leo and Sagittarius: rules 1–3 give 0 each; rule 4: both lords (Sun, Jupiter)
        // sit in an even sign while Leo and Sagittarius are odd → tie; rule 5: the dual
        // Sagittarius wins.
        let all_cn = [95.0; 9];
        let c = RasiChart::new(0.0, &all_cn);
        assert_eq!(stronger_rasi(&c, 4, 8), 8);
        assert_eq!(stronger_rasi(&c, 8, 4), 8);
    }

    #[test]
    fn co_lord_rule_zero_and_conjunction() {
        // Mars in Scorpio, Ketu in Taurus → Ketu is the lord to use.
        let g = [0.0, 0.0, 215.0, 0.0, 0.0, 0.0, 0.0, 225.0, 45.0];
        let c = RasiChart::new(0.0, &g);
        assert_eq!(sign_lord(&c, 7, false), KETU);
        // Neither in Aquarius; Saturn joined by four grahas → Saturn.
        let g = [5.0, 6.0, 7.0, 8.0, 100.0, 9.0, 10.0, 150.0, 330.0];
        let c = RasiChart::new(0.0, &g);
        assert_eq!(sign_lord(&c, 10, false), SATURN);
    }
}
