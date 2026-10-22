//! Vimshottari transitions at any depth. Level 1 is the mahadasha; each level divides
//! the one above in the same proportion, so level 9 periods last minutes. Every
//! boundary is exact arithmetic on the birth instant (integer nanoseconds), never a
//! float.

use kshana_time::Instant;
use kshana_vedic::{subdivide, vimshottari, DashaLord, DashaPeriod};

/// Names of the levels, 1-based.
pub const LEVEL_NAMES: [&str; 9] = [
    "Mahadasha",
    "Antardasha",
    "Pratyantardasha",
    "Sookshmadasha",
    "Pranadasha",
    "Dehadasha",
    "Level 7",
    "Level 8",
    "Level 9",
];

/// Name of level `level` (1-based) in the reader's language: the Kerala names (ദശ,
/// അപഹാരം, ഛിദ്രം) and the Hindi panchang names (महादशा, अंतर्दशा, प्रत्यंतर्दशा).
pub fn level_name_in(level: usize, lang: kshana_rules::Lang) -> &'static str {
    const NAMES: [[&str; 3]; 9] = [
        ["Mahadasha", "മഹാദശ", "महादशा"],
        ["Antardasha", "അപഹാരം", "अंतर्दशा"],
        ["Pratyantardasha", "ഛിദ്രം", "प्रत्यंतर्दशा"],
        ["Sookshmadasha", "സൂക്ഷ്മദശ", "सूक्ष्म दशा"],
        ["Pranadasha", "പ്രാണദശ", "प्राण दशा"],
        ["Dehadasha", "ദേഹദശ", "देह दशा"],
        ["Level 7", "ഏഴാം തലം", "सातवाँ स्तर"],
        ["Level 8", "എട്ടാം തലം", "आठवाँ स्तर"],
        ["Level 9", "ഒമ്പതാം തലം", "नौवाँ स्तर"],
    ];
    lang.pick(NAMES[level.clamp(1, 9) - 1])
}

/// The deepest level offered.
pub const MAX_DEPTH: usize = 9;

/// A transition: at `at`, the chain of lords from level 1 down to `level` becomes `chain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashaTransition {
    /// Exact instant.
    pub at: Instant,
    /// The level that changes (1 = mahadasha). Every deeper level changes too.
    pub level: u8,
    /// Lords from level 1 to `level` after the transition.
    pub chain: Vec<DashaLord>,
    /// End of the new period at `level`.
    pub until: Instant,
}

/// Every transition at levels `1..=depth` with an instant in `[from, to)`.
pub fn dasha_transitions(
    moon_sidereal: f64,
    birth: Instant,
    year_ns: i128,
    depth: usize,
    from: Instant,
    to: Instant,
) -> Vec<DashaTransition> {
    let depth = depth.clamp(1, MAX_DEPTH);
    let mut out = Vec::new();
    let mahas = vimshottari(moon_sidereal, birth, year_ns);
    descend(&mahas, 1, depth, &[], birth, from, to, &mut out);
    out.sort_by_key(|t| t.at);
    out
}

#[allow(clippy::too_many_arguments)]
fn descend(
    periods: &[DashaPeriod],
    level: usize,
    depth: usize,
    prefix: &[DashaLord],
    birth: Instant,
    from: Instant,
    to: Instant,
    out: &mut Vec<DashaTransition>,
) {
    for p in periods {
        if p.end <= from || p.start >= to {
            continue;
        }
        let mut chain = prefix.to_vec();
        chain.push(p.lord);
        if p.start >= from && p.start < to && p.start > birth {
            out.push(DashaTransition {
                at: p.start,
                level: level as u8,
                chain: chain.clone(),
                until: p.end,
            });
        }
        if level < depth {
            descend(
                &subdivide(*p),
                level + 1,
                depth,
                &chain,
                birth,
                from,
                to,
                out,
            );
        }
    }
}

/// The chain of periods containing `at` down to `depth`, with exact bounds.
pub fn chain_at(
    moon_sidereal: f64,
    birth: Instant,
    year_ns: i128,
    depth: usize,
    at: Instant,
) -> Vec<DashaPeriod> {
    kshana_vedic::vimshottari_levels(moon_sidereal, birth, year_ns, at, depth.clamp(1, MAX_DEPTH))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_time::NS_PER_DAY;
    use kshana_vedic::dasha::YEAR_365_25_NS;

    #[test]
    fn deep_levels_nest_and_count() {
        let birth = Instant::j2000();
        let from = birth.add_ns(30 * 365 * NS_PER_DAY);
        let to = from.add_ns(NS_PER_DAY);
        let t = dasha_transitions(316.344, birth, YEAR_365_25_NS, 9, from, to);
        // level 9 periods are minutes long: a day holds hundreds of transitions
        assert!(t.len() > 100, "{}", t.len());
        assert!(t.windows(2).all(|w| w[0].at <= w[1].at));
        // a transition at level k changes exactly the levels >= k, and the chain prefix
        // agrees with the chain found by chain_at at that instant
        for x in t.iter().take(50) {
            let c = chain_at(316.344, birth, YEAR_365_25_NS, x.level as usize, x.at);
            let lords: Vec<_> = c.iter().map(|p| p.lord).collect();
            assert_eq!(lords, x.chain);
            assert_eq!(c.last().unwrap().start, x.at);
            assert_eq!(c.last().unwrap().end, x.until);
        }
    }

    #[test]
    fn level_one_boundaries_are_the_mahadasha_starts() {
        let birth = Instant::j2000();
        let to = birth.add_ns(120 * YEAR_365_25_NS);
        let t = dasha_transitions(316.344, birth, YEAR_365_25_NS, 1, birth, to);
        let m = vimshottari(316.344, birth, YEAR_365_25_NS);
        assert_eq!(t.len(), 8);
        for (x, p) in t.iter().zip(m.iter().skip(1)) {
            assert_eq!(x.at, p.start);
            assert_eq!(x.chain, vec![p.lord]);
        }
    }
}
