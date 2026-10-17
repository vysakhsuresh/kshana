//! The other ways to answer a graha: food given to its people, creatures fed, water
//! and trees tended, colours worn and kept from. Each graha has its own people and
//! its own creatures in the Puranic daana lists and the temple tradition (the crow for
//! Saturn and the ancestors, the cow for Jupiter and Venus, the dog for Ketu and
//! Bhairava, the ant and the fish for Rahu and Ketu, the peepal for Saturn and
//! Jupiter, the tulasi for Venus and Mercury). Nothing here needs a priest or a
//! purchase beyond a meal.

use crate::parihara::{RemedyKind, Row};
use kshana_vedic::DashaLord;

const PURANA: &str = "Puranic graha-daana lists (Matsya, Yajnavalkya, Garuda)";
const LAL: &str = "Lal Kitab";
const TEMPLE: &str = "Temple practice (Navagraha kshetras, Kerala and Tamil Nadu)";
const DHARMA: &str = "Dharma reading of the graha's signification";

/// Food, creatures, water, trees and colour for a graha.
pub fn alternatives(g: DashaLord) -> Vec<Row> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Anna, "Feed wheat roti and jaggery to men who labour in the sun", "The Sun's grain to the Sun's people: masons, road workers, farmers at midday.", "once a week", "Sunday", PURANA),
            (Anna, "A hot breakfast for a watchman or a driver who was up before dawn", "The Sun is service rendered before it rises.", "once", "Sunday", DHARMA),
            (Jeeva, "Feed a cow with wheat flour and jaggery at sunrise", "The cow fed in Surya hora is the oldest Sun daana.", "weekly", "Sunday", TEMPLE),
            (Prakriti, "Offer water to the rising Sun from a copper vessel, and water a plant with what remains", "Arghya given, then not wasted: the Sun and the earth both fed.", "daily", "any day", TEMPLE),
            (Vastra, "Wear something red, orange or copper on Sundays; keep black off the body that day", "The Sun's colours worn on its day; its enemy's colour kept away.", "weekly", "Sunday", LAL),
            (Seva, "Visit your father, or an old man alone, and sit an hour without a phone", "An hour of attention is the Sun's true gift.", "weekly", "Sunday", DHARMA),
            (Anna, "Sweet made of wheat and ghee, given to a temple or shared at work", "The Sun's naivedya, shared.", "once", "Sunday", TEMPLE),
        ],
        DashaLord::Moon => vec![
            (Anna, "Rice kheer or milk sweets to women and children in need", "The Moon's food, white and sweet, to the Moon's people.", "once a week", "Monday", PURANA),
            (Anna, "A glass of milk or buttermilk to anyone who serves your house", "The Moon is the nourisher; small daily giving of milk steadies it.", "daily", "any day", DHARMA),
            (Jeeva, "Feed a white cow, or birds with rice at dawn", "The Moon's creatures fed in Chandra hora.", "weekly", "Monday", TEMPLE),
            (Prakriti, "Pour water at the root of a tulasi or a jasmine each morning", "Water and the white flower are the Moon's.", "daily", "any day", TEMPLE),
            (Prakriti, "Sit by water, a river, a tank, the sea, for twenty minutes on a Monday", "The Moon is the tides; being near water is the cheapest Moon remedy there is.", "weekly", "Monday", DHARMA),
            (Vastra, "Wear white or silver on Mondays; sleep in light colours", "The Moon's colours on its day.", "weekly", "Monday", LAL),
            (Anna, "Cook rice yourself and feed your mother, or a mother", "The Moon is the mother fed by her child.", "once", "Monday", DHARMA),
        ],
        DashaLord::Mars => vec![
            (Anna, "Red lentil dal and rice to labourers or soldiers, served hot", "Mars's grain to Mars's people, by your own hand.", "once a week", "Tuesday", PURANA),
            (Anna, "Jaggery and roasted gram to boys who play or work hard", "Mars is youth and heat; feed it well and it fights for you.", "once", "Tuesday", TEMPLE),
            (Jeeva, "Feed monkeys, or a red cow, with jaggery and gram on a Tuesday", "Hanuman's creatures.", "weekly", "Tuesday", TEMPLE),
            (Prakriti, "Plant a neem or a pomegranate and keep it alive", "Mars is land; a tree kept alive on land is Mars kept.", "once", "Tuesday", DHARMA),
            (Vastra, "Wear red or coral on Tuesdays; keep knives and tools clean and in their place", "Mars's colour on its day; order among its instruments.", "weekly", "Tuesday", LAL),
            (Seva, "Carry something heavy for someone who cannot", "Strength spent for another is Mars at its best.", "weekly", "any day", DHARMA),
            (Anna, "Sweet pongal or red rice to a Subrahmanya or Hanuman temple, then shared", "Mars's naivedya.", "once", "Tuesday", TEMPLE),
        ],
        DashaLord::Mercury => vec![
            (Anna, "Green moong, spinach dishes or fruit to students and clerks", "Mercury's grain to Mercury's people.", "once a week", "Wednesday", PURANA),
            (Anna, "Feed a child who is studying, and ask about the lesson", "Mercury is the student; listening is part of the meal.", "weekly", "Wednesday", DHARMA),
            (Jeeva, "Feed green moong to cows or grain to parrots and small birds", "Mercury's creatures.", "weekly", "Wednesday", TEMPLE),
            (Prakriti, "Grow something green on a windowsill and keep it alive", "Mercury is growth you can see; tending it tends the mind.", "once", "Wednesday", DHARMA),
            (Vastra, "Wear green on Wednesdays; keep a clean notebook and pen", "Mercury's colour on its day and its instruments in order.", "weekly", "Wednesday", LAL),
            (Seva, "Teach one person one thing they wanted to learn, for free", "Mercury is the exchange of knowledge.", "once", "any day", DHARMA),
            (Anna, "Give books or school supplies with a packet of sweets", "Mercury's daana in the old lists was the book.", "once", "Wednesday", PURANA),
        ],
        DashaLord::Jupiter => vec![
            (Anna, "Chana dal, bananas and ghee sweets to teachers, priests or the old", "Jupiter's grain to Jupiter's people.", "once a week", "Thursday", PURANA),
            (Anna, "Pay for one child's midday meals for a month", "Jupiter is children fed and taught.", "once", "Thursday", DHARMA),
            (Jeeva, "Feed cows bananas and chana on a Thursday", "The cow is Jupiter's creature.", "weekly", "Thursday", TEMPLE),
            (Prakriti, "Water a peepal tree on Thursdays; plant one if you can", "The peepal is Jupiter's and Saturn's tree both.", "weekly", "Thursday", TEMPLE),
            (Vastra, "Wear yellow or saffron on Thursdays; a turmeric mark on the forehead", "Jupiter's colour on its day.", "weekly", "Thursday", LAL),
            (Seva, "Visit a teacher you once had and thank them in person", "Jupiter is the guru; gratitude is its food.", "once", "Thursday", DHARMA),
            (Anna, "Kesari or chana sundal to a Dakshinamurti or Vishnu temple, then shared", "Jupiter's naivedya.", "once", "Thursday", TEMPLE),
        ],
        DashaLord::Venus => vec![
            (Anna, "Curd rice, white sweets or perfume to young women and to artists", "Venus's food and gifts to Venus's people.", "once a week", "Friday", PURANA),
            (Anna, "Feed a couple or a bride-to-be a meal you cooked", "Venus is partnership fed.", "once", "Friday", DHARMA),
            (Jeeva, "Feed a white cow curd and rice; keep water for birds in summer", "Venus's creatures and comfort.", "weekly", "Friday", TEMPLE),
            (Prakriti, "Plant or tend jasmine, rose or tulasi; keep fresh flowers at home on Fridays", "Flowers are Venus's; a house with flowers is a house Venus visits.", "weekly", "Friday", DHARMA),
            (Vastra, "Wear white or pastel on Fridays; bathe with a little milk or rose water", "Venus's colour and comfort on its day.", "weekly", "Friday", LAL),
            (Seva, "Help pay for a poor girl's wedding or schooling", "The classical Venus daana.", "once", "Friday", PURANA),
            (Anna, "Sweet rice to a Lakshmi temple, then shared with women at home", "Venus's naivedya.", "once", "Friday", TEMPLE),
        ],
        DashaLord::Saturn => vec![
            (Anna, "Sesame rice or black-gram dishes to labourers, sweepers and the old", "Saturn's grain to Saturn's people, the ones who do the hard work unseen.", "once a week", "Saturday", PURANA),
            (Anna, "Feed the disabled, the imprisoned or the dying through those who serve them", "Saturn rules the forgotten; feeding them is the direct remedy.", "once", "Saturday", DHARMA),
            (Jeeva, "Feed crows cooked rice with sesame before your own breakfast", "The crow is Saturn's and the ancestors'; fed first, it carries the debt.", "daily", "start Saturday", TEMPLE),
            (Jeeva, "Feed a black dog, or a stray, on Saturdays", "Saturn's and Bhairava's creature.", "weekly", "Saturday", TEMPLE),
            (Prakriti, "Pour water at the root of a peepal at dusk on Saturday and walk round it seven times", "The peepal is Saturn's tree; water given to it is given to Saturn.", "weekly", "Saturday", TEMPLE),
            (Anna, "Give sesame oil, or an iron vessel of it, to a Shani or Hanuman temple", "Oil is Saturn's offering; it is poured, not kept.", "once", "Saturday", TEMPLE),
            (Vastra, "Wear dark blue or black on Saturdays; keep iron on the body (a plain ring or kada)", "Saturn's colours and metal on its day.", "weekly", "Saturday", LAL),
            (Seva, "Clean a public place, a well, a temple yard, with your own hands", "Humble work is Saturn's worship.", "once", "Saturday", DHARMA),
        ],
        DashaLord::Rahu => vec![
            (Anna, "Feed the outcast: lepers, sweepers, those no one feeds, with your own hands", "Rahu is the one outside the circle; feeding him there is the classical Rahu daana.", "once a week", "Saturday", PURANA),
            (Anna, "Feed a stranger from another land or faith", "Rahu is the foreigner; a meal across the border calms him.", "once", "any day", DHARMA),
            (Anna, "Give blankets or warm food to people sleeping on the street at night", "Rahu rules the night and the forgotten.", "once", "Saturday", DHARMA),
            (Jeeva, "Feed seven grains to birds, and flour to ants, in the morning", "Rahu's creatures, small and many.", "daily", "start Saturday", LAL),
            (Jeeva, "Feed fish in a tank or river with flour balls", "The creatures of deep water are Rahu's.", "weekly", "Saturday", LAL),
            (Prakriti, "Keep coconut or a coconut tree; give a coconut to flowing water on a Saturday", "Rahu's fruit, returned to Rahu's element.", "once", "Saturday", LAL),
            (Vastra, "Wear smoke-blue or grey on Saturdays; keep a clean doorstep and a lit lamp at dusk", "Rahu is confusion; a swept threshold and a lamp is order against it.", "weekly", "Saturday", LAL),
            (Seva, "Serve at a Durga temple, or sweep it, on a Saturday", "Durga is Rahu's deity.", "once", "Saturday", TEMPLE),
        ],
        DashaLord::Ketu => vec![
            (Anna, "Feed dogs, and the people who feed dogs", "The dog is Ketu's; the feeder of dogs is Ketu's person.", "once a week", "Tuesday", TEMPLE),
            (Anna, "Sesame and jaggery, or kichdi of mixed grains, to the wandering poor", "Ketu is the renunciate; feed the one who owns nothing.", "once", "Tuesday", PURANA),
            (Jeeva, "Feed ants with sugar and flour; feed a stray dog daily", "Ketu's creatures, the smallest and the loyal.", "daily", "start Tuesday", LAL),
            (Prakriti, "Plant a banana or a tulasi; sit in silence under a tree for ten minutes", "Ketu is the inner road; a tree and silence are its temple.", "weekly", "Tuesday", DHARMA),
            (Vastra, "Wear mixed or earth colours on Tuesdays; keep nothing on the body you do not need that day", "Ketu's colours and its lesson of less.", "weekly", "Tuesday", LAL),
            (Seva, "Serve at a Ganapati temple, or help someone finish a thing they could not finish", "Ganapati, Ketu's deity, removes the block.", "once", "Tuesday", TEMPLE),
            (Anna, "Modak or sesame sweets to a Ganapati temple, then shared with children", "Ketu's naivedya.", "once", "Tuesday", TEMPLE),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_graha_has_food_creatures_and_colour() {
        for g in DashaLord::ORDER {
            let v = alternatives(g);
            assert!(v.len() >= 7, "{g:?}");
            assert!(v.iter().any(|r| r.0 == RemedyKind::Anna), "{g:?} food");
            assert!(v.iter().any(|r| r.0 == RemedyKind::Jeeva), "{g:?} creatures");
            assert!(v.iter().any(|r| r.0 == RemedyKind::Vastra), "{g:?} colour");
        }
    }
}
