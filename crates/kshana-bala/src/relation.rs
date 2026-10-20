//! Natural, temporary and compound (panchadha) relationships between the grahas.
//!
//! BPHS ch. 3 (slokas 55–60 in Santhanam's numbering): the natural relationship is
//! fixed; the temporary one is read from the chart, a graha in the 2nd, 3rd, 4th, 10th,
//! 11th or 12th sign from another being its temporary friend and one in the 1st, 5th,
//! 6th, 7th, 8th or 9th its temporary enemy. The two combine into five grades: natural
//! friend and temporary friend make an *adhi-mitra* (great friend), neutral and
//! temporary friend a *mitra*, friend with temporary enemy or enemy with temporary
//! friend a *sama* (neutral), neutral with temporary enemy a *satru* (enemy), enemy with
//! temporary enemy an *adhi-satru* (great enemy).

use kshana_rules::parashari::natural_relation;
use kshana_vedic::DashaLord;

/// Compound (panchadha) relationship of one graha to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    /// Great enemy.
    AdhiSatru,
    /// Enemy.
    Satru,
    /// Neutral.
    Sama,
    /// Friend.
    Mitra,
    /// Great friend.
    AdhiMitra,
}

impl Relation {
    /// Saptavargaja value for a graha in a sign owned by a lord of this relation:
    /// great friend 22.5, friend 15, neutral 7.5, enemy 3.75, great enemy 1.875 (Raman,
    /// *Graha and Bhava Balas*, Sthana bala; JHora, PyJHora `_sapthavargaja_bala_2`).
    /// Santhanam's BPHS 27.2–4 prints 20, 15, 10, 4, 2 for the same grades; the halving
    /// series that Raman and JHora use is kept.
    pub fn saptavargaja_virupas(self) -> f64 {
        match self {
            Relation::AdhiMitra => 22.5,
            Relation::Mitra => 15.0,
            Relation::Sama => 7.5,
            Relation::Satru => 3.75,
            Relation::AdhiSatru => 1.875,
        }
    }

    /// Word for the relation.
    pub fn name(self) -> &'static str {
        match self {
            Relation::AdhiMitra => "great friend",
            Relation::Mitra => "friend",
            Relation::Sama => "neutral",
            Relation::Satru => "enemy",
            Relation::AdhiSatru => "great enemy",
        }
    }
}

/// Temporary (tatkalika) relationship from the rasi positions: +1 when `other` stands in
/// the 2nd, 3rd, 4th, 10th, 11th or 12th sign from `sign`, −1 otherwise.
pub fn temporary_relation(sign: u8, other_sign: u8) -> i8 {
    let from = (other_sign as i32 - sign as i32).rem_euclid(12) + 1;
    if matches!(from, 2 | 3 | 4 | 10 | 11 | 12) {
        1
    } else {
        -1
    }
}

/// Compound relationship of `a` towards `b`, given their rasi signs.
pub fn compound_relation(a: DashaLord, b: DashaLord, sign_a: u8, sign_b: u8) -> Relation {
    let natural = natural_relation(a, b);
    let temporary = temporary_relation(sign_a, sign_b);
    match natural + temporary {
        2 => Relation::AdhiMitra,
        1 => Relation::Mitra,
        0 => Relation::Sama,
        -1 => Relation::Satru,
        _ => Relation::AdhiSatru,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_vedic::DashaLord::*;

    #[test]
    fn temporary_friends_are_the_six_houses() {
        assert_eq!(temporary_relation(0, 1), 1); // 2nd
        assert_eq!(temporary_relation(0, 3), 1); // 4th
        assert_eq!(temporary_relation(0, 4), -1); // 5th
        assert_eq!(temporary_relation(0, 9), 1); // 10th
        assert_eq!(temporary_relation(0, 11), 1); // 12th
        assert_eq!(temporary_relation(0, 0), -1); // same sign
        assert_eq!(temporary_relation(0, 6), -1); // 7th
    }

    #[test]
    fn compound_grades() {
        // Sun and Jupiter are natural friends: with Jupiter in the 2nd from the Sun → great friend
        assert_eq!(compound_relation(Sun, Jupiter, 0, 1), Relation::AdhiMitra);
        // in the 7th → neutral
        assert_eq!(compound_relation(Sun, Jupiter, 0, 6), Relation::Sama);
        // Sun and Saturn natural enemies: in the 7th → great enemy; in the 3rd → neutral
        assert_eq!(compound_relation(Sun, Saturn, 0, 6), Relation::AdhiSatru);
        assert_eq!(compound_relation(Sun, Saturn, 0, 2), Relation::Sama);
        // Sun and Mercury natural neutrals: 11th → friend, 5th → enemy
        assert_eq!(compound_relation(Sun, Mercury, 0, 10), Relation::Mitra);
        assert_eq!(compound_relation(Sun, Mercury, 0, 4), Relation::Satru);
        assert_eq!(Relation::AdhiMitra.saptavargaja_virupas(), 22.5);
        assert_eq!(Relation::AdhiSatru.saptavargaja_virupas(), 1.875);
    }
}
