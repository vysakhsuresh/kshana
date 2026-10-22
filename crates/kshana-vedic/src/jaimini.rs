//! Jaimini chara karakas: the seven movable significators, ranked by degree in sign.
//!
//! Convention stated: the seven-karaka scheme (Sun through Saturn, Rahu excluded), the
//! planet with the highest degree in its sign being Atmakaraka. The eight-karaka scheme
//! with Rahu (BPHS ch. 32) is offered as [`chara_karakas_8`]. Also here: all twelve
//! arudha padas ([`arudha_padas`]) and rasi drishti ([`sign_aspects`]).

use crate::{sign_lord, DashaLord};

/// The seven chara karakas in rank order.
pub const KARAKA_NAMES: [&str; 7] = [
    "Atmakaraka",
    "Amatyakaraka",
    "Bhratrikaraka",
    "Matrikaraka",
    "Putrakaraka",
    "Gnatikaraka",
    "Darakaraka",
];

/// What each karaka signifies, in plain words.
pub const KARAKA_MEANINGS: [&str; 7] = [
    "the self, the soul's purpose",
    "career, counsel, the work one is known for",
    "siblings, courage, teachers",
    "mother, home, learning",
    "children, students, creativity",
    "rivals, obstacles, illness",
    "spouse, partnerships",
];

/// The karaka names as Kerala astrologers and Hindi panchangs write them: index 0 is
/// Malayalam, 1 Hindi; rank order as [`KARAKA_NAMES_8`] (Pitrikaraka at rank 4).
const KARAKA_NAMES_8_IN: [[&str; 8]; 2] = [
    [
        "ആത്മകാരകൻ",
        "അമാത്യകാരകൻ",
        "ഭ്രാതൃകാരകൻ",
        "മാതൃകാരകൻ",
        "പിതൃകാരകൻ",
        "പുത്രകാരകൻ",
        "ജ്ഞാതികാരകൻ",
        "ദാരകാരകൻ",
    ],
    ["आत्मकारक", "अमात्यकारक", "भ्रातृकारक", "मातृकारक", "पितृकारक", "पुत्रकारक", "ज्ञातिकारक", "दारकारक"],
];

/// [`KARAKA_MEANINGS`] in Malayalam (index 0) and Hindi (index 1).
const KARAKA_MEANINGS_IN: [[&str; 7]; 2] = [
    [
        "സ്വന്തം വ്യക്തിത്വം, ജീവിതലക്ഷ്യം",
        "തൊഴിൽ, ഉപദേശം, ഒരാൾ അറിയപ്പെടുന്ന ജോലി",
        "സഹോദരങ്ങൾ, ധൈര്യം, ഗുരുക്കന്മാർ",
        "അമ്മ, വീട്, പഠനം",
        "മക്കൾ, ശിഷ്യർ, സർഗ്ഗശക്തി",
        "എതിരാളികൾ, തടസ്സങ്ങൾ, രോഗം",
        "ജീവിതപങ്കാളി, പങ്കാളിത്തങ്ങൾ",
    ],
    [
        "स्वयं, आत्मा का उद्देश्य",
        "करियर, सलाह, वह काम जिससे व्यक्ति जाना जाता है",
        "भाई-बहन, साहस, गुरु",
        "माता, घर, विद्या",
        "संतान, शिष्य, रचनात्मकता",
        "प्रतिद्वंद्वी, बाधाएँ, रोग",
        "जीवनसाथी, साझेदारी",
    ],
];

fn lang_index(code: &str) -> Option<usize> {
    match code.trim().to_ascii_lowercase().as_str() {
        "ml" => Some(0),
        "hi" => Some(1),
        _ => None,
    }
}

/// A seven-scheme karaka's name (rank 0 = Atmakaraka) for a language code ("en",
/// "ml", "hi"); English for anything else.
pub fn karaka_name_in(rank: usize, code: &str) -> &'static str {
    let rank = rank.min(6);
    match lang_index(code) {
        // the seven scheme skips Pitrikaraka
        Some(l) => KARAKA_NAMES_8_IN[l][if rank >= 4 { rank + 1 } else { rank }],
        None => KARAKA_NAMES[rank],
    }
}

/// An eight-scheme karaka's name (rank 0 = Atmakaraka … 7 = Darakaraka) for a
/// language code; English for anything else.
pub fn karaka_name_8_in(rank: usize, code: &str) -> &'static str {
    let rank = rank.min(7);
    match lang_index(code) {
        Some(l) => KARAKA_NAMES_8_IN[l][rank],
        None => KARAKA_NAMES_8[rank],
    }
}

/// What a seven-scheme karaka signifies, for a language code; English for anything
/// else.
pub fn karaka_meaning_in(rank: usize, code: &str) -> &'static str {
    let rank = rank.min(6);
    match lang_index(code) {
        Some(l) => KARAKA_MEANINGS_IN[l][rank],
        None => KARAKA_MEANINGS[rank],
    }
}

/// One karaka assignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Karaka {
    /// Rank 0 = Atmakaraka … 6 = Darakaraka.
    pub rank: usize,
    /// The graha.
    pub graha: DashaLord,
    /// Degree within its sign, `[0, 30)`.
    pub degree_in_sign: f64,
}

/// Rank the seven planets (sidereal longitudes in the order Sun, Moon, Mars, Mercury,
/// Jupiter, Venus, Saturn) by degree in sign, highest first.
pub fn chara_karakas(sidereal: [f64; 7]) -> [Karaka; 7] {
    const ORDER: [DashaLord; 7] = [
        DashaLord::Sun,
        DashaLord::Moon,
        DashaLord::Mars,
        DashaLord::Mercury,
        DashaLord::Jupiter,
        DashaLord::Venus,
        DashaLord::Saturn,
    ];
    let mut v: Vec<(DashaLord, f64)> = ORDER
        .iter()
        .zip(sidereal.iter())
        .map(|(g, l)| (*g, l.rem_euclid(30.0)))
        .collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    std::array::from_fn(|i| Karaka {
        rank: i,
        graha: v[i].0,
        degree_in_sign: v[i].1,
    })
}

/// Arudha pada of a house: count from the house to its lord, then the same distance
/// again; if that lands on the house itself or its 7th, take the 10th from there.
/// `house_sign` and `lord_sign` are sidereal sign indices 0–11.
pub fn arudha(house_sign: usize, lord_sign: usize) -> usize {
    let d = (lord_sign + 12 - house_sign) % 12;
    let pada = (lord_sign + d) % 12;
    if pada == house_sign || pada == (house_sign + 6) % 12 {
        (pada + 9) % 12
    } else {
        pada
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_degree_is_atmakaraka() {
        let k = chara_karakas([10.0, 45.0, 89.0, 120.5, 150.0, 199.9, 240.0]);
        assert_eq!(k[0].graha, DashaLord::Mars);
        assert_eq!(k[6].graha, DashaLord::Saturn);
        assert!((k[1].degree_in_sign - 19.9).abs() < 1e-9);
    }

    #[test]
    fn karaka_words_in_three_languages() {
        for r in 0..7 {
            assert_eq!(karaka_name_in(r, "en"), KARAKA_NAMES[r]);
            assert_eq!(karaka_meaning_in(r, "en"), KARAKA_MEANINGS[r]);
            for code in ["ml", "hi"] {
                for s in [karaka_name_in(r, code), karaka_meaning_in(r, code)] {
                    assert!(!s.chars().any(|c| c.is_ascii_alphabetic()), "{code}: {s}");
                }
            }
        }
        assert_eq!(karaka_name_in(4, "hi"), "पुत्रकारक");
        assert_eq!(karaka_name_8_in(4, "ml"), "പിതൃകാരകൻ");
    }

    #[test]
    fn arudha_exceptions() {
        assert_eq!(arudha(0, 0), 9);
        assert_eq!(arudha(0, 6), 9);
        assert_eq!(arudha(0, 2), 4);
    }
}

// ---------------------------------------------------------------------------------
// The eight-karaka scheme, all twelve padas and sign aspects.
// ---------------------------------------------------------------------------------

/// The eight chara karakas in rank order (BPHS ch. 32, Santhanam: Atma, Amatya,
/// Bhratri, Matri, Pitri, Putra, Gnati and Dara, Rahu included as the eighth graha).
pub const KARAKA_NAMES_8: [&str; 8] = [
    "Atmakaraka",
    "Amatyakaraka",
    "Bhratrikaraka",
    "Matrikaraka",
    "Pitrikaraka",
    "Putrakaraka",
    "Gnatikaraka",
    "Darakaraka",
];

/// Which karaka scheme to rank by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarakaScheme {
    /// Sun to Saturn, Rahu excluded (Jaimini Sutras 1.1 as B. Suryanarain Rao and most
    /// modern readers apply them).
    Seven,
    /// Sun to Saturn and Rahu (BPHS ch. 32, Santhanam; Sanjay Rath's school). Rahu,
    /// always moving backwards, is ranked by the degrees still to run in its sign.
    Eight,
}

/// Rank the eight karakas. `sidereal` is Sun, Moon, Mars, Mercury, Jupiter, Venus,
/// Saturn, Rahu. Rahu's degree is counted backwards from the end of its sign
/// (30° − degree in sign, BPHS ch. 32); its `degree_in_sign` holds that counted value,
/// the one the ranking used.
pub fn chara_karakas_8(sidereal: [f64; 8]) -> [Karaka; 8] {
    const ORDER: [DashaLord; 8] = [
        DashaLord::Sun,
        DashaLord::Moon,
        DashaLord::Mars,
        DashaLord::Mercury,
        DashaLord::Jupiter,
        DashaLord::Venus,
        DashaLord::Saturn,
        DashaLord::Rahu,
    ];
    let mut v: Vec<(DashaLord, f64)> = ORDER
        .iter()
        .zip(sidereal.iter())
        .map(|(g, l)| {
            let d = l.rem_euclid(30.0);
            (*g, if *g == DashaLord::Rahu { 30.0 - d } else { d })
        })
        .collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    std::array::from_fn(|i| Karaka {
        rank: i,
        graha: v[i].0,
        degree_in_sign: v[i].1,
    })
}

/// Index of a graha in the Sun..Saturn order, `None` for the nodes.
fn seven_index(g: DashaLord) -> Option<usize> {
    match g {
        DashaLord::Sun => Some(0),
        DashaLord::Moon => Some(1),
        DashaLord::Mars => Some(2),
        DashaLord::Mercury => Some(3),
        DashaLord::Jupiter => Some(4),
        DashaLord::Venus => Some(5),
        DashaLord::Saturn => Some(6),
        _ => None,
    }
}

/// The twelve arudha padas A1..A12 (index 0 = A1, the Pada lagna; index 11 = A12, the
/// Upapada), as sign indices 0 = Aries.
///
/// BPHS ch. 29 (Santhanam) and Jaimini Sutras 1.3 (B. Suryanarain Rao): count from the
/// house to its lord and as many again from the lord; if the pada falls in the house
/// itself take the 10th from it, if it falls in the 7th from the house take the 10th
/// from there (the 4th from the house). Each sign has its one BPHS ch. 3 lord (Mars
/// for Scorpio, Saturn for Aquarius); the Ketu and Rahu co-lordship some Jaimini
/// schools use is not applied. `planet_signs`: Sun, Moon, Mars, Mercury, Jupiter,
/// Venus, Saturn.
pub fn arudha_padas(lagna_sign: usize, planet_signs: &[usize; 7]) -> [usize; 12] {
    std::array::from_fn(|i| {
        let house_sign = (lagna_sign + i) % 12;
        let lord = sign_lord(house_sign);
        let lord_sign = planet_signs[seven_index(lord).expect("sign lords are Sun..Saturn")];
        arudha(house_sign, lord_sign % 12)
    })
}

/// Signs aspected by a sign under rasi drishti (Jaimini Sutras 1.1; BPHS ch. 8): a
/// movable sign aspects the three fixed signs other than the one beside it, a fixed
/// sign the three movable signs other than the one beside it, a dual sign the other
/// three duals. The aspect is mutual. 0 = Aries. (`kshana_marga::rasi_drishti` is the
/// same rule on `u8`.)
pub fn sign_aspects(sign: usize) -> [usize; 3] {
    let s = sign % 12;
    let pick = |set: [usize; 4], skip: usize| -> [usize; 3] {
        let v: Vec<usize> = set.into_iter().filter(|x| *x != skip).collect();
        [v[0], v[1], v[2]]
    };
    match s % 3 {
        0 => pick([1, 4, 7, 10], (s + 1) % 12),
        1 => pick([0, 3, 6, 9], (s + 11) % 12),
        _ => pick([2, 5, 8, 11], s),
    }
}

/// True when sign `a` aspects sign `b` by rasi drishti.
pub fn signs_aspect(a: usize, b: usize) -> bool {
    sign_aspects(a).contains(&(b % 12))
}

#[cfg(test)]
mod full_tests {
    use super::*;
    use crate::varga;

    #[test]
    fn eight_karakas_count_rahu_backwards() {
        // Sun 10.5°, Moon 17.2°, Mars 5.9°, Mercury 8.0°, Jupiter 20.3°, Venus 23.1°,
        // Saturn 1.4°, Rahu at 24° Taurus → 30 − 24 = 6.0°.
        let lon = [10.5, 47.2, 95.9, 128.0, 200.3, 263.1, 331.4, 54.0];
        let k = chara_karakas_8(lon);
        let order: Vec<DashaLord> = k.iter().map(|x| x.graha).collect();
        use DashaLord::*;
        assert_eq!(order, vec![Venus, Jupiter, Moon, Sun, Mercury, Rahu, Mars, Saturn]);
        assert!((k[5].degree_in_sign - 6.0).abs() < 1e-9);
        // The seven-karaka scheme on the same chart drops Rahu and closes the gap.
        let k7 = chara_karakas([lon[0], lon[1], lon[2], lon[3], lon[4], lon[5], lon[6]]);
        let order7: Vec<DashaLord> = k7.iter().map(|x| x.graha).collect();
        assert_eq!(order7, vec![Venus, Jupiter, Moon, Sun, Mercury, Mars, Saturn]);
        // Rahu early in its sign is a high karaka: 2° Taurus counts as 28°.
        let mut lon2 = lon;
        lon2[7] = 32.0;
        assert_eq!(chara_karakas_8(lon2)[0].graha, Rahu);
        assert_eq!(KARAKA_NAMES_8[4], "Pitrikaraka");
    }

    #[test]
    fn twelve_padas_hand_checked() {
        // Aries lagna; Sun Gemini, Moon Aquarius, Mars Capricorn, Mercury Leo, Jupiter
        // Taurus, Venus Cancer, Saturn Pisces (worked by hand, BPHS ch. 29):
        // A1 Aries→Mars in Cap (10th): 10th from Cap = Libra, the 7th → Cancer.
        // A2 Taurus→Venus Cancer (3rd) → Virgo. A3 Gemini→Mercury Leo → Libra.
        // A4 Cancer→Moon Aquarius (8th) → Virgo. A5 Leo→Sun Gemini (11th) → Aries.
        // A6 Virgo→Mercury Leo (12th) → Cancer. A7 Libra→Venus Cancer (10th) → Aries,
        // the 7th from Libra → Capricorn. A8 Scorpio→Mars Cap → Pisces.
        // A9 Sagittarius→Jupiter Taurus (6th) → Libra. A10 Capricorn→Saturn Pisces →
        // Taurus. A11 Aquarius→Saturn Pisces → Aries. A12 Pisces→Jupiter Taurus → Cancer.
        let signs = [2, 10, 9, 4, 1, 3, 11];
        assert_eq!(arudha_padas(0, &signs), [3, 5, 6, 5, 0, 3, 9, 11, 6, 1, 0, 3]);
        // The exceptions: lord in the house itself or the 7th → the 10th from the house;
        // lord in the 4th or the 10th → the 4th from the house.
        assert_eq!(arudha(4, 4), 1);
        assert_eq!(arudha(4, 10), 1);
        assert_eq!(arudha(4, 7), 7);
        assert_eq!(arudha(4, 1), 7);
        // No pada ever lands in its own house or the 7th from it.
        for h in 0..12 {
            for l in 0..12 {
                let p = arudha(h, l);
                assert!(p != h && p != (h + 6) % 12, "{h} {l} {p}");
            }
        }
    }

    #[test]
    fn sign_aspects_and_karakamsa_navamsa() {
        assert_eq!(sign_aspects(0), [4, 7, 10]);
        assert_eq!(sign_aspects(1), [3, 6, 9]);
        assert_eq!(sign_aspects(8), [2, 5, 11]);
        for a in 0..12 {
            for b in sign_aspects(a) {
                assert!(signs_aspect(b, a));
            }
        }
        // Karakamsa is the Atmakaraka's navamsa: 15° Gemini (dual, counted from the 5th,
        // Libra), the 5th part → Aquarius.
        assert_eq!((varga(75.0, 9) / 30.0).floor() as usize, 10);
        // 23.1° Sagittarius (dual, from Aries), the 7th part → Libra.
        assert_eq!((varga(263.1, 9) / 30.0).floor() as usize, 6);
    }
}
