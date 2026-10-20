//! The glossary: every term the app shows, in one or two plain sentences.

/// One term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// The term as shown.
    pub term: &'static str,
    /// Plain meaning.
    pub meaning: &'static str,
    /// Tradition: "Vedic", "Western", "Both", "Engine".
    pub tradition: &'static str,
}

const fn e(term: &'static str, meaning: &'static str, tradition: &'static str) -> GlossaryEntry {
    GlossaryEntry {
        term,
        meaning,
        tradition,
    }
}

/// All entries, alphabetical by term.
pub fn glossary() -> Vec<GlossaryEntry> {
    let mut v = vec![
        e("Antardasha", "A sub-period inside a mahadasha. Each mahadasha is split among the nine planets in the same order and proportion as the whole cycle.", "Vedic"),
        e("Ascendant", "The sign rising on the eastern horizon at the moment of birth. Called Lagna in Vedic astrology. It fixes the houses.", "Both"),
        e("Ashtakavarga", "A point system: each planet gives 0 or 1 point to each sign from eight reference points. A sign with many points is strong for transits and periods.", "Vedic"),
        e("Ashtakoota", "The eight-fold Moon matching used for marriage: Varna, Vashya, Tara, Yoni, Graha Maitri, Gana, Bhakoot, Nadi, scored out of 36.", "Vedic"),
        e("Aspect", "An angle between two planets. Western: conjunction 0°, sextile 60°, square 90°, trine 120°, opposition 180°. Vedic: planets aspect whole houses by count.", "Both"),
        e("Atmakaraka", "In Jaimini astrology, the planet with the highest degree in its sign. It stands for the self and the soul's purpose.", "Vedic"),
        e("Ayanamsa", "The gap between the tropical zodiac (anchored to the equinox) and the sidereal zodiac (anchored to the stars). Lahiri is the Indian government standard.", "Vedic"),
        e("Benefic", "A planet that tends to help: Jupiter, Venus, a waxing Moon, and Mercury when alone. Malefics are Saturn, Mars, the Sun, Rahu and Ketu.", "Vedic"),
        e("Bhava", "A house. The twelve divisions of the sky from the ascendant, each ruling an area of life.", "Vedic"),
        e("Cusp", "The exact starting degree of a house. KP astrology reads the sub-lord of the cusp to decide whether a matter is promised.", "Both"),
        e("Dasha", "A planetary period. The Vimshottari system gives each planet a fixed number of years in a 120-year cycle, starting from the Moon's star at birth.", "Vedic"),
        e("Debilitation", "The sign where a planet is weakest (for example the Sun in Libra). The opposite of exaltation.", "Both"),
        e("Delta T", "The difference between Earth-rotation time (UT1) and uniform atomic time (TT). The engine uses measured values, not a formula, where data exist.", "Engine"),
        e("Dosha", "A flaw in a chart or match, such as Mangal dosha (Mars in certain houses) or Nadi dosha. Most have stated cancellations.", "Vedic"),
        e("Ekadashi", "The eleventh tithi of each fortnight, a traditional fasting day. Observed on the day whose sunrise falls within it.", "Vedic"),
        e("Ephemeris", "The table of planetary positions. Kshana uses NASA JPL's DE440s kernel (1849 to 2150) directly, the same data used for spacecraft navigation.", "Engine"),
        e("Exaltation", "The sign where a planet is strongest (for example the Sun in Aries, the Moon in Taurus, Jupiter in Cancer).", "Both"),
        e("Gochara", "Transits: where the planets are now relative to your birth Moon and chart. Sade Sati is a gochara of Saturn.", "Vedic"),
        e("Hora", "A planetary hour. Day and night are each divided into twelve; each hour belongs to a planet in a fixed sequence from the day's ruler.", "Both"),
        e("House system", "The method of dividing the sky into twelve houses. Placidus is the Western default; whole sign and equal are the simplest; KP uses Placidus.", "Both"),
        e("Karaka", "A significator: the planet that naturally stands for a matter (Venus for marriage, Jupiter for children, Saturn for work).", "Vedic"),
        e("Karana", "Half a tithi. Eleven karanas cycle through the month; four of them are fixed to particular half-tithis.", "Vedic"),
        e("Kendra", "The angular houses 1, 4, 7 and 10, the pillars of a chart. Trikonas are 1, 5 and 9, the houses of fortune.", "Vedic"),
        e("Ketu", "The Moon's south node, where the Moon's path crosses the Sun's going south. Always opposite Rahu. Stands for detachment and the past.", "Vedic"),
        e("KP", "Krishnamurti Paddhati: a Vedic method that divides each star into nine unequal sub-parts and reads the sub-lord of each cusp. Precise and rule-based.", "Vedic"),
        e("Kundli", "The Vedic birth chart, drawn as a square (North or South Indian style) rather than a wheel.", "Vedic"),
        e("Lagna", "The ascendant. See Ascendant.", "Vedic"),
        e("Mahadasha", "A major planetary period, from 6 years (Sun) to 20 years (Venus).", "Vedic"),
        e("Mangal dosha", "Mars in the 1st, 2nd, 4th, 7th, 8th or 12th house from the ascendant, Moon or Venus. Read as strain in marriage unless cancelled.", "Vedic"),
        e("Muhurta", "A chosen auspicious moment for starting something. Judged from the tithi, star, day, hora and the Moon's standing for the person.", "Vedic"),
        e("Nadi", "One of three pulse groups of the 27 stars. Two Moons in the same Nadi is the heaviest matching flaw.", "Vedic"),
        e("Nakshatra", "One of 27 lunar stars, each 13°20′ wide. The Moon's star at birth sets your dasha, your temperament and your naming syllable.", "Vedic"),
        e("Navamsa", "The ninth divisional chart (D9), each sign split into nine. Read for marriage and for the inner strength of planets.", "Vedic"),
        e("Node", "Rahu and Ketu, where the Moon's orbit crosses the Sun's path. The true node wobbles; the mean node is its average. Both are shown.", "Both"),
        e("Pada", "A quarter of a nakshatra, 3°20′ wide. The four padas map onto the navamsa.", "Vedic"),
        e("Panchanga", "The five limbs of the Vedic day: tithi, vara (weekday), nakshatra, yoga and karana.", "Vedic"),
        e("Parashari", "The classical Vedic system from Parashara's text (BPHS): house lords, dignities, aspects and yogas.", "Vedic"),
        e("Parihara", "Remedy. In Kshana the first remedies are actions that match the cause; rituals are listed but never required.", "Vedic"),
        e("Prashna", "A question chart cast for the moment of asking. KP prashna uses a number from 1 to 249 chosen by the asker.", "Vedic"),
        e("Rahu", "The Moon's north node. Stands for hunger, the foreign, the new. Always opposite Ketu.", "Vedic"),
        e("Rahu kala", "A daily window of about ninety minutes ruled by Rahu, avoided for starting things. Its position depends on the weekday and the actual sunrise.", "Vedic"),
        e("Rectification", "Fixing an uncertain birth time from dated life events: the engine tests each candidate time against the events.", "Both"),
        e("Retrograde", "A planet appearing to move backwards against the stars. An optical effect of the orbits; read as turning inward or revisiting.", "Both"),
        e("Sade Sati", "The seven and a half years when Saturn passes through the sign before, over and after your birth Moon. Three phases of about 2½ years each.", "Vedic"),
        e("Sankranti", "The Sun entering a sidereal sign. Makara Sankranti (Capricorn) is the best known.", "Vedic"),
        e("Sidereal", "Measured against the fixed stars. The Vedic zodiac is sidereal; the Western zodiac is tropical.", "Both"),
        e("Sub-lord", "In KP, the lord of the sub-division of a star where a point falls. The sub-lord of a cusp decides whether the house delivers.", "Vedic"),
        e("Synastry", "Western compatibility: the aspects between one person's planets and another's.", "Western"),
        e("Tithi", "A lunar day: every 12° the Moon gains on the Sun. Thirty tithis make a month; the full Moon is the 15th, the new Moon the 30th.", "Vedic"),
        e("Transit", "Where a planet is now, placed in your birth chart. See Gochara.", "Both"),
        e("Tropical", "Measured from the spring equinox. The Western zodiac. Slowly drifts against the stars (precession).", "Western"),
        e("Varga", "A divisional chart. D1 is the birth chart; D9 the navamsa; D10 for career; D7 children; D4 property; D24 learning; D30 misfortune.", "Vedic"),
        e("Vimshottari", "The 120-year dasha system. See Dasha.", "Vedic"),
        e("Yoga (panchanga)", "One of 27 daily yogas from the sum of the Sun's and Moon's longitudes. Some are avoided for new beginnings.", "Vedic"),
        e("Yoga (chart)", "A planetary combination with a classical result, such as Gaja Kesari (Jupiter in a kendra from the Moon) or a Raja yoga.", "Vedic"),
        e("Yoni", "The animal nature of each nakshatra, used in matching for physical compatibility. Fourteen animals in seven pairs.", "Vedic"),
    ];
    v.sort_by(|a, b| a.term.cmp(b.term));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glossary_is_sorted_and_nonempty() {
        let g = glossary();
        assert!(g.len() >= 50);
        assert!(g.windows(2).all(|w| w[0].term <= w[1].term));
    }
}
