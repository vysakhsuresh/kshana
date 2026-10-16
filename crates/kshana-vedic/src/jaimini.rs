//! Jaimini chara karakas: the seven movable significators, ranked by degree in sign.
//!
//! Convention stated: the seven-karaka scheme (Sun through Saturn, Rahu excluded), the
//! planet with the highest degree in its sign being Atmakaraka. The eight-karaka scheme
//! with Rahu is a known alternative; this module does not use it.

use crate::DashaLord;

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
    fn arudha_exceptions() {
        assert_eq!(arudha(0, 0), 9);
        assert_eq!(arudha(0, 6), 9);
        assert_eq!(arudha(0, 2), 4);
    }
}
