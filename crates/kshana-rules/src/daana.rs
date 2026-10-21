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

/// Food, creatures, water, trees and colour for a graha. The why says what it eases.
pub fn alternatives(g: DashaLord) -> Vec<Row> {
    use RemedyKind::*;
    match g {
        DashaLord::Sun => vec![
            (Anna, "Feed wheat roti and jaggery to men who labour outdoors in the heat of the day", "Masons, road workers, farmers at midday. Eases trouble with authority and lifts your standing.", "once a week", "Sunday", PURANA),
            (Anna, "A hot breakfast for a watchman or a driver who was up before dawn", "Service to those who serve before dawn returns as recognition.", "once", "Sunday", DHARMA),
            (Jeeva, "Feed a cow with wheat flour and jaggery at sunrise", "The oldest gift for confidence, health and favour from seniors.", "weekly", "Sunday", TEMPLE),
            (Prakriti, "Offer water at sunrise from a copper vessel, facing east, and water a plant with what remains", "Starts the day with intent; steadies confidence and energy.", "daily", "any day", TEMPLE),
            (Vastra, "Wear something red, orange or copper on Sundays; keep black off the body that day", "A small daily reminder of the confidence you are building.", "weekly", "Sunday", LAL),
            (Seva, "Visit your father, or an old man alone, and sit an hour without a phone", "An hour of attention mends the bond with father figures and bosses.", "weekly", "Sunday", DHARMA),
            (Anna, "Sweet made of wheat and ghee, given to a temple or shared at work", "Goodwill at work, offered and shared.", "once", "Sunday", TEMPLE),
        ],
        DashaLord::Moon => vec![
            (Anna, "Rice kheer or milk sweets to women and children in need", "Calms worry and brings peace at home.", "once a week", "Monday", PURANA),
            (Anna, "A glass of milk or buttermilk to anyone who serves your home", "Small daily giving steadies the mind and the household.", "daily", "any day", DHARMA),
            (Jeeva, "Feed a white cow, or birds with rice at dawn", "Eases mood swings and restless sleep.", "weekly", "Monday", TEMPLE),
            (Prakriti, "Pour water at the root of a tulasi or a jasmine each morning", "A calm morning habit that settles the mind.", "daily", "any day", TEMPLE),
            (Prakriti, "Sit by water, a river, a tank, the sea, for twenty minutes on a Monday", "The cheapest cure there is for an anxious, heavy mind.", "weekly", "Monday", DHARMA),
            (Vastra, "Wear white or silver on Mondays; sleep in light colours", "Light colours for a lighter mood.", "weekly", "Monday", LAL),
            (Anna, "Cook rice yourself and feed your mother, or a mother", "Brings the mother's blessing and peace at home.", "once", "Monday", DHARMA),
        ],
        DashaLord::Mars => vec![
            (Anna, "Red lentil dal and rice to labourers or soldiers, served hot, by your own hand", "Cools anger and quarrels; helps land and property matters.", "once a week", "Tuesday", PURANA),
            (Anna, "Jaggery and roasted gram to boys who play or work hard", "Turns restless energy into strength that works for you.", "once", "Tuesday", TEMPLE),
            (Jeeva, "Feed monkeys, or a red cow, with jaggery and gram on a Tuesday", "For courage, and safety on the road.", "weekly", "Tuesday", TEMPLE),
            (Prakriti, "Plant a neem or a pomegranate and keep it alive", "Steadies property and land affairs.", "once", "Tuesday", DHARMA),
            (Vastra, "Wear red or coral on Tuesdays; keep knives and tools clean and in their place", "Order among sharp things keeps you safe from cuts and slips.", "weekly", "Tuesday", LAL),
            (Seva, "Carry something heavy for someone who cannot", "Strength spent for another is strength that does not turn into temper.", "weekly", "any day", DHARMA),
            (Anna, "Sweet pongal or red rice to a Subrahmanya or Hanuman temple, then shared", "For courage and calm.", "once", "Tuesday", TEMPLE),
        ],
        DashaLord::Mercury => vec![
            (Anna, "Green moong, spinach dishes or fruit to students and clerks", "Clears confusion in studies, paperwork and trade.", "once a week", "Wednesday", PURANA),
            (Anna, "Feed a child who is studying, and ask about the lesson", "Sharpens your own thinking and memory.", "weekly", "Wednesday", DHARMA),
            (Jeeva, "Feed green moong to cows or grain to parrots and small birds", "For clear speech and good deals.", "weekly", "Wednesday", TEMPLE),
            (Prakriti, "Grow something green on a windowsill and keep it alive", "Tending something growing tends the mind.", "once", "Wednesday", DHARMA),
            (Vastra, "Wear green on Wednesdays; keep a clean notebook and pen", "Order in your tools brings order to your papers.", "weekly", "Wednesday", LAL),
            (Seva, "Teach one person one thing they wanted to learn, for free", "Knowledge shared returns as opportunity.", "once", "any day", DHARMA),
            (Anna, "Give books or school supplies with a packet of sweets", "The classical gift for exams and learning.", "once", "Wednesday", PURANA),
        ],
        DashaLord::Jupiter => vec![
            (Anna, "Chana dal, bananas and ghee sweets to teachers, priests or the old", "Brings back luck, good advice and savings.", "once a week", "Thursday", PURANA),
            (Anna, "Pay for one child's midday meals for a month", "For the blessing of children and growth in the home.", "once", "Thursday", DHARMA),
            (Jeeva, "Feed cows bananas and chana on a Thursday", "For good fortune and wise decisions.", "weekly", "Thursday", TEMPLE),
            (Prakriti, "Water a peepal tree on Thursdays; plant one if you can", "For steady growth in money and family.", "weekly", "Thursday", TEMPLE),
            (Vastra, "Wear yellow or saffron on Thursdays; a turmeric mark on the forehead", "A reminder of the faith and judgement you are building.", "weekly", "Thursday", LAL),
            (Seva, "Visit a teacher you once had and thank them in person", "Gratitude brings guidance back into your life.", "once", "Thursday", DHARMA),
            (Anna, "Kesari or chana sundal to a Dakshinamurti or Vishnu temple, then shared", "For blessings on money, marriage and children.", "once", "Thursday", TEMPLE),
        ],
        DashaLord::Venus => vec![
            (Anna, "Curd rice, white sweets or perfume to young women and to artists", "Restores love, comfort and harmony.", "once a week", "Friday", PURANA),
            (Anna, "Feed a couple or a bride-to-be a meal you cooked", "Harmony given to a couple returns to your own marriage.", "once", "Friday", DHARMA),
            (Jeeva, "Feed a white cow curd and rice; keep water for birds in summer", "For comfort and ease at home.", "weekly", "Friday", TEMPLE),
            (Prakriti, "Plant or tend jasmine, rose or tulasi; keep fresh flowers at home on Fridays", "A home with flowers is a warmer home; love follows.", "weekly", "Friday", DHARMA),
            (Vastra, "Wear white or pastel on Fridays; bathe with a little milk or rose water", "Care for yourself brings back attraction and ease.", "weekly", "Friday", LAL),
            (Seva, "Help pay for a poor girl's wedding or schooling", "The classical gift for a happy marriage.", "once", "Friday", PURANA),
            (Anna, "Sweet rice to a Lakshmi temple, then shared with women at home", "For comfort and plenty at home.", "once", "Friday", TEMPLE),
        ],
        DashaLord::Saturn => vec![
            (Anna, "Sesame rice or black-gram dishes to labourers, sweepers and the old", "The people who do the hard work unseen. Eases delays, debts and tiredness at work.", "once a week", "Saturday", PURANA),
            (Anna, "Feed the disabled, the imprisoned or those in hospice care through those who serve them", "Feeding the forgotten is the direct remedy for a heavy, slow period.", "once", "Saturday", DHARMA),
            (Jeeva, "Feed crows cooked rice with sesame before your own breakfast", "Eases delays and the weight of old family debts.", "daily", "start Saturday", TEMPLE),
            (Jeeva, "Feed a black dog, or a stray, on Saturdays", "Eases delays, debts and tiredness at work.", "weekly", "Saturday", TEMPLE),
            (Prakriti, "Pour water at the root of a peepal at dusk on Saturday and walk round it seven times", "For patience and protection through a hard stretch.", "weekly", "Saturday", TEMPLE),
            (Anna, "Give sesame oil, or an iron vessel of it, to a Hanuman temple", "Oil is poured, not kept: let go of what weighs on you.", "once", "Saturday", TEMPLE),
            (Vastra, "Wear dark blue or black on Saturdays; keep iron on the body (a plain ring or kada)", "A small daily reminder of the discipline a slow time asks for.", "weekly", "Saturday", LAL),
            (Seva, "Clean a public place, a well, a temple yard, with your own hands", "Humble work lightens a heavy period.", "once", "Saturday", DHARMA),
        ],
        DashaLord::Rahu => vec![
            (Anna, "Feed the outcast: lepers, sweepers, those no one feeds, with your own hands", "Clears confusion, fear and sudden trouble.", "once a week", "Saturday", PURANA),
            (Anna, "Feed a stranger from another land or faith", "Calms restlessness; opens foreign doors safely.", "once", "any day", DHARMA),
            (Anna, "Give blankets or warm food to people sleeping on the street at night", "Eases anxiety and fears that come at night.", "once", "Saturday", DHARMA),
            (Jeeva, "Feed seven grains to birds, and flour to ants, in the morning", "Small daily giving against confusion and scattered effort.", "daily", "start Saturday", LAL),
            (Jeeva, "Feed fish in a tank or river with flour balls", "Eases hidden worries and sudden setbacks.", "weekly", "Saturday", LAL),
            (Prakriti, "Give a coconut to flowing water on a Saturday", "Let the worry go with the water.", "once", "Saturday", LAL),
            (Vastra, "Wear smoke-blue or grey on Saturdays; keep a clean doorstep and a lit lamp at dusk", "A swept doorstep and a lamp bring order against confusion.", "weekly", "Saturday", LAL),
            (Seva, "Serve at a Durga temple, or sweep it, on a Saturday", "For protection from fear and deception.", "once", "Saturday", TEMPLE),
        ],
        DashaLord::Ketu => vec![
            (Anna, "Feed dogs, and the people who feed dogs", "Eases sudden losses and breaks.", "once a week", "Tuesday", TEMPLE),
            (Anna, "Sesame and jaggery, or kichdi of mixed grains, to the wandering poor", "Feeding the one who owns nothing eases losses and drift.", "once", "Tuesday", PURANA),
            (Jeeva, "Feed ants with sugar and flour; feed a stray dog daily", "Small, steady giving against obstacles and loss.", "daily", "start Tuesday", LAL),
            (Prakriti, "Plant a banana or a tulasi; sit in silence under a tree for ten minutes", "Gives direction when you feel lost.", "weekly", "Tuesday", DHARMA),
            (Vastra, "Wear mixed or earth colours on Tuesdays; keep nothing on the body you do not need that day", "The lesson of less, worn.", "weekly", "Tuesday", LAL),
            (Seva, "Serve at a Ganapati temple, or help someone finish a thing they could not finish", "Removes blocks from your own unfinished work.", "once", "Tuesday", TEMPLE),
            (Anna, "Modak or sesame sweets to a Ganapati temple, then shared with children", "For the removal of obstacles.", "once", "Tuesday", TEMPLE),
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
