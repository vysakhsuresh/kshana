//! The lore of the planetary hour, and a name for every moment.
//!
//! Each hora lord carries a deity, a colour, a direction, a metal, a beeja mantra and
//! what the hour is for, from the Navagraha tradition (Agama beeja mantras, the
//! Navagraha dik and dhatu tables, Hora shastra). The moment's name is a short title
//! woven from its tara, its hour and the Moon's phase, so an instant reads like an
//! entry in a calendar of omens.

use crate::gochara::TARA_NAMES;
use kshana_vedic::DashaLord;

/// What an hour's lord carries.
#[derive(Clone, Debug, PartialEq)]
pub struct HoraLore {
    /// The lord.
    pub lord: DashaLord,
    /// Its deity.
    pub deity: &'static str,
    /// Its colour.
    pub colour: &'static str,
    /// Its direction.
    pub direction: &'static str,
    /// Its metal.
    pub metal: &'static str,
    /// Its beeja mantra, one line.
    pub mantra: &'static str,
    /// What the hour is for.
    pub for_: &'static str,
    /// What to keep away from in it.
    pub avoid: &'static str,
    /// Source.
    pub source: &'static str,
}

const SOURCE: &str = "Navagraha beeja mantras (Agama tradition); Navagraha dik and dhatu tables; Hora shastra";

/// The lore of a hora lord.
pub fn hora_lore(lord: DashaLord) -> HoraLore {
    match lord {
        DashaLord::Sun => HoraLore {
            lord,
            deity: "Surya",
            colour: "copper red",
            direction: "east",
            metal: "copper",
            mantra: "Om Hram Hreem Hraum Sah Suryaya Namah",
            for_: "authority, petitions, medicine, anything official",
            avoid: "quarrels with elders and the state",
            source: SOURCE,
        },
        DashaLord::Moon => HoraLore {
            lord,
            deity: "Chandra, and Parvati",
            colour: "white",
            direction: "north-west",
            metal: "silver",
            mantra: "Om Shram Shreem Shraum Sah Chandraya Namah",
            for_: "family, travel, water, planting, matters of the heart",
            avoid: "harsh words and decisions made in a mood",
            source: SOURCE,
        },
        DashaLord::Mars => HoraLore {
            lord,
            deity: "Subrahmanya, and Hanuman",
            colour: "red",
            direction: "south",
            metal: "copper",
            mantra: "Om Kram Kreem Kraum Sah Bhaumaya Namah",
            for_: "effort, courage, surgery, land, a fight you must win",
            avoid: "new partnerships and anger",
            source: SOURCE,
        },
        DashaLord::Mercury => HoraLore {
            lord,
            deity: "Vishnu",
            colour: "green",
            direction: "north",
            metal: "bronze",
            mantra: "Om Bram Breem Braum Sah Budhaya Namah",
            for_: "trade, writing, study, messages and deals",
            avoid: "signing in haste",
            source: SOURCE,
        },
        DashaLord::Jupiter => HoraLore {
            lord,
            deity: "Brihaspati, and Dakshinamurti",
            colour: "yellow",
            direction: "north-east",
            metal: "gold",
            mantra: "Om Gram Greem Graum Sah Gurave Namah",
            for_: "learning, money, children, teachers and good beginnings",
            avoid: "pride and promises you cannot keep",
            source: SOURCE,
        },
        DashaLord::Venus => HoraLore {
            lord,
            deity: "Lakshmi",
            colour: "white and bright",
            direction: "south-east",
            metal: "silver",
            mantra: "Om Dram Dreem Draum Sah Shukraya Namah",
            for_: "love, art, purchases, pleasure and partnership",
            avoid: "a quarrel with the one you love",
            source: SOURCE,
        },
        DashaLord::Saturn => HoraLore {
            lord,
            deity: "Shani, and Ayyappa",
            colour: "blue-black",
            direction: "west",
            metal: "iron",
            mantra: "Om Pram Preem Praum Sah Shanaischaraya Namah",
            for_: "routine, service, elders, labour and patience",
            avoid: "beginnings, loans and shortcuts",
            source: SOURCE,
        },
        DashaLord::Rahu => HoraLore {
            lord,
            deity: "Durga",
            colour: "smoke",
            direction: "south-west",
            metal: "lead",
            mantra: "Om Bhram Bhreem Bhraum Sah Rahave Namah",
            for_: "the foreign and the unusual",
            avoid: "deception",
            source: SOURCE,
        },
        DashaLord::Ketu => HoraLore {
            lord,
            deity: "Ganapati",
            colour: "variegated",
            direction: "north-west",
            metal: "lead",
            mantra: "Om Sram Sreem Sraum Sah Ketave Namah",
            for_: "letting go and inner work",
            avoid: "beginnings",
            source: SOURCE,
        },
    }
}

/// A name for the moment: "Sampat under the Moon's hour, waxing".
pub fn moment_title(tara: usize, hora: Option<DashaLord>, waxing: bool, star: &str) -> String {
    let phase = if waxing { "waxing" } else { "waning" };
    let t = TARA_NAMES[tara.min(8)];
    match hora {
        Some(l) => {
            let joiner = match l {
                DashaLord::Saturn | DashaLord::Mars => "in",
                _ => "under",
            };
            format!("{t} {joiner} {}'s hour, {phase}", lord_possessive(l))
        }
        None => format!("{t} in {star}, {phase}"),
    }
}

fn lord_possessive(l: DashaLord) -> &'static str {
    match l {
        DashaLord::Sun => "the Sun",
        DashaLord::Moon => "the Moon",
        DashaLord::Mars => "Mars",
        DashaLord::Mercury => "Mercury",
        DashaLord::Jupiter => "Jupiter",
        DashaLord::Venus => "Venus",
        DashaLord::Saturn => "Saturn",
        DashaLord::Rahu => "Rahu",
        DashaLord::Ketu => "Ketu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_lord_has_lore_and_moments_have_names() {
        for l in DashaLord::ORDER {
            let h = hora_lore(l);
            assert!(!h.deity.is_empty() && h.mantra.starts_with("Om "));
        }
        assert_eq!(
            moment_title(1, Some(DashaLord::Moon), true, "Mula"),
            "Sampat under the Moon's hour, waxing"
        );
        assert_eq!(moment_title(6, Some(DashaLord::Saturn), false, "Mula"), "Naidhana in Saturn's hour, waning");
        assert_eq!(moment_title(8, None, true, "Revati"), "Parama Mitra in Revati, waxing");
    }
}
