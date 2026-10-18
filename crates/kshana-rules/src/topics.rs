//! The question dictionary: the matters people actually ask about, each a KP house
//! group of its own (KP Reader VI, the horary tables), hung under one of the nine lives.
//! A topic answers with the same six questions as a life, with the topic's houses in
//! the promise and the timing funnel and the life's daily sky behind them.

use crate::areas::Area;
use crate::forecast::Life;

/// One matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topic {
    /// Stable key.
    pub key: &'static str,
    /// The words on the chip.
    pub words: &'static str,
    /// Which life it hangs under.
    pub life: Life,
    /// Its KP houses.
    pub area: Area,
}

macro_rules! topic {
    ($key:literal, $words:literal, $life:ident, $name:literal, $primary:literal, [$($g:literal),*], [$($n:literal),*], $varga:literal) => {
        Topic {
            key: $key,
            words: $words,
            life: Life::$life,
            area: Area {
                name: $name,
                primary: $primary,
                group: &[$($g),*],
                negating: &[$($n),*],
                varga: $varga,
            },
        }
    };
}

/// Every topic, grouped by life in display order.
pub const TOPICS: &[Topic] = &[
    // Love
    topic!("marriage", "Marriage", Love, "Marriage", 7, [2, 7, 11], [1, 6, 10], "D9"),
    topic!("new_love", "A new relationship", Love, "Romance", 5, [5, 7, 11], [4, 6, 10], "D9"),
    topic!("reunion", "Getting back together", Love, "Reunion", 7, [2, 7, 11], [1, 6, 10], "D9"),
    topic!("second_marriage", "A second marriage", Love, "Second marriage", 9, [2, 9, 11], [1, 8, 10], "D9"),
    topic!("separation", "A separation ending", Love, "Separation", 6, [1, 6, 10], [2, 7, 11], "D9"),
    topic!("in_laws", "Peace with in-laws", Love, "In-laws", 2, [2, 7, 11], [1, 6, 10], "D9"),
    // Career
    topic!("new_job", "A new job", Career, "Job", 10, [2, 6, 10], [1, 5, 9], "D10"),
    topic!("promotion", "A promotion", Career, "Promotion", 10, [2, 6, 10, 11], [1, 5, 9], "D10"),
    topic!("transfer", "A transfer", Career, "Transfer", 10, [3, 10, 12], [2, 9, 11], "D10"),
    topic!("own_business", "A business of my own", Career, "Business", 10, [2, 7, 10, 11], [1, 6, 9], "D10"),
    topic!("government_post", "A government post", Career, "Government post", 10, [6, 10, 11], [5, 9], "D10"),
    topic!("business_partner", "A partner in business", Career, "Business partner", 7, [7, 10, 11], [6, 9], "D10"),
    topic!("foreign_job", "A job abroad", Career, "Job abroad", 10, [9, 10, 12], [8, 11], "D10"),
    topic!("court_case", "Winning a court case or dispute", Career, "Litigation", 6, [6, 11], [5, 12], "D1"),
    // Money
    topic!("loan", "Getting a loan", Money, "Loan", 6, [6, 11], [5, 10], "D2"),
    topic!("debt_cleared", "Clearing a debt", Money, "Debt", 12, [2, 11, 12], [1, 10], "D2"),
    topic!("investment", "An investment paying off", Money, "Investment", 5, [2, 5, 11], [1, 4, 10], "D2"),
    topic!("inheritance", "An inheritance", Money, "Inheritance", 8, [2, 8, 11], [1, 7, 10], "D2"),
    topic!("windfall", "A windfall", Money, "Windfall", 11, [2, 5, 8, 11], [1, 4, 7, 10], "D2"),
    topic!("sale", "Selling something", Money, "Sale", 3, [2, 3, 10, 11], [1, 9], "D2"),
    // Health
    topic!("recovery", "Recovering from illness", Health, "Recovery", 1, [1, 5, 11], [6, 8, 12], "D30"),
    topic!("surgery", "A surgery going well", Health, "Surgery", 1, [1, 5, 11], [6, 8, 12], "D30"),
    topic!("peace_of_mind", "Peace of mind", Health, "Peace of mind", 4, [1, 4, 5], [3, 12], "D30"),
    topic!("mother_health", "A mother's health", Health, "Mother's health", 4, [2, 4, 8], [1, 3, 7], "D30"),
    topic!("father_health", "A father's health", Health, "Father's health", 9, [1, 7, 9], [6, 8, 12], "D30"),
    // Family
    topic!("home_peace", "Peace at home", Family, "Family", 2, [2, 4, 11], [1, 3, 10], "D4"),
    topic!("sibling", "A brother or sister", Family, "Sibling", 3, [3, 11], [2, 10], "D4"),
    topic!("mother", "Mother", Family, "Mother", 4, [4, 11], [3, 10], "D4"),
    topic!("father", "Father", Family, "Father", 9, [9, 11], [8, 10], "D4"),
    topic!("moving_home", "Moving home", Family, "Moving", 4, [3, 4, 12], [2, 11], "D4"),
    // Study
    topic!("exam", "An exam", Study, "Exam", 4, [4, 9, 11], [3, 8, 10], "D24"),
    topic!("admission", "An admission", Study, "Admission", 9, [4, 9, 11], [3, 8, 10], "D24"),
    topic!("competitive_exam", "A competitive exam", Study, "Competitive exam", 6, [4, 6, 9, 11], [3, 5, 8, 10], "D24"),
    topic!("study_abroad", "Studying abroad", Study, "Study abroad", 12, [4, 9, 12], [3, 8, 11], "D24"),
    topic!("scholarship", "A scholarship", Study, "Scholarship", 11, [2, 4, 11], [1, 3, 10], "D24"),
    // Home
    topic!("buy_house", "Buying a home or land", Home, "Property", 4, [4, 11, 12], [3, 10], "D4"),
    topic!("build_house", "Building a home", Home, "Building", 4, [4, 11], [3, 10], "D4"),
    topic!("sell_house", "Selling a home or land", Home, "Selling property", 3, [3, 10], [2, 9], "D4"),
    topic!("rent", "A place to rent", Home, "Renting", 4, [4, 11], [3, 10], "D4"),
    topic!("vehicle", "A vehicle", Home, "Vehicle", 4, [2, 4, 11], [1, 3, 10], "D4"),
    // Travel
    topic!("visa", "A visa", Travel, "Visa", 12, [3, 9, 12], [2, 8, 11], "D9"),
    topic!("journey_abroad", "A journey abroad", Travel, "Journey abroad", 9, [3, 9, 12], [2, 8, 11], "D9"),
    topic!("settle_abroad", "Settling abroad", Travel, "Settling abroad", 12, [3, 9, 12], [2, 8, 11], "D9"),
    topic!("pilgrimage", "A pilgrimage", Travel, "Pilgrimage", 9, [9, 12], [8, 11], "D9"),
    topic!("return_home", "Coming back home", Travel, "Return home", 4, [2, 4, 11], [1, 3, 10], "D9"),
    // Children
    topic!("child_birth", "A child", Children, "Children", 5, [2, 5, 11], [1, 4, 10], "D7"),
    topic!("child_marriage", "A child's marriage", Children, "Child's marriage", 11, [3, 6, 11], [2, 5, 10], "D7"),
    topic!("child_studies", "A child's studies", Children, "Child's studies", 8, [1, 3, 8], [2, 7, 12], "D7"),
    topic!("child_health", "A child's health", Children, "Child's health", 5, [3, 5, 9], [2, 4, 8], "D7"),
    topic!("adoption", "An adoption", Children, "Adoption", 5, [2, 5, 11], [1, 4, 10], "D7"),
];

/// Topics under a life, in order.
pub fn topics_for(life: Life) -> Vec<&'static Topic> {
    TOPICS.iter().filter(|t| t.life == life).collect()
}

/// Look a topic up by key.
pub fn topic(key: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_life_has_topics_with_sound_house_groups() {
        for life in Life::ALL {
            let v = topics_for(life);
            assert!(v.len() >= 5, "{life:?}");
        }
        let mut keys: Vec<&str> = TOPICS.iter().map(|t| t.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), TOPICS.len(), "duplicate topic key");
        for t in TOPICS {
            assert_eq!(crate::plain::jargon(t.words), None, "{}", t.words);
        }
        for t in TOPICS {
            assert!(t.area.group.contains(&t.area.primary), "{}: primary outside the group", t.key);
            assert!((1..=12).contains(&t.area.primary));
            for h in t.area.group.iter().chain(t.area.negating.iter()) {
                assert!((1..=12).contains(h), "{}", t.key);
            }
            assert!(t.area.group.iter().all(|h| !t.area.negating.contains(h)), "{}: a house both fulfils and negates", t.key);
        }
    }
}
