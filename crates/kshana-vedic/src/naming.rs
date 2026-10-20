//! Naming syllables (namakshara) for each nakshatra pada, as used across India.

use crate::panchanga::NAKSHATRA_NAMES;

/// The four syllables of each nakshatra, pada 1–4.
pub const SYLLABLES: [[&str; 4]; 27] = [
    ["Chu", "Che", "Cho", "La"],
    ["Li", "Lu", "Le", "Lo"],
    ["A", "I", "U", "E"],
    ["O", "Va", "Vi", "Vu"],
    ["Ve", "Vo", "Ka", "Ki"],
    ["Ku", "Gha", "Nga", "Chha"],
    ["Ke", "Ko", "Ha", "Hi"],
    ["Hu", "He", "Ho", "Da"],
    ["Di", "Du", "De", "Do"],
    ["Ma", "Mi", "Mu", "Me"],
    ["Mo", "Ta", "Ti", "Tu"],
    ["Te", "To", "Pa", "Pi"],
    ["Pu", "Sha", "Na", "Tha"],
    ["Pe", "Po", "Ra", "Ri"],
    ["Ru", "Re", "Ro", "Ta"],
    ["Ti", "Tu", "Te", "To"],
    ["Na", "Ni", "Nu", "Ne"],
    ["No", "Ya", "Yi", "Yu"],
    ["Ye", "Yo", "Bha", "Bhi"],
    ["Bhu", "Dha", "Pha", "Dha"],
    ["Bhe", "Bho", "Ja", "Ji"],
    ["Ju", "Je", "Jo", "Gha"],
    ["Ga", "Gi", "Gu", "Ge"],
    ["Go", "Sa", "Si", "Su"],
    ["Se", "So", "Da", "Di"],
    ["Du", "Tha", "Jha", "Da"],
    ["De", "Do", "Cha", "Chi"],
];

/// The syllable for a nakshatra index (0–26) and pada (1–4).
pub fn syllable(nakshatra: usize, pada: u8) -> &'static str {
    SYLLABLES[nakshatra.min(26)][(pada.clamp(1, 4) - 1) as usize]
}

/// Name and syllable for a sidereal Moon longitude.
pub fn naming_for(moon_sidereal: f64) -> (&'static str, u8, &'static str) {
    let n = crate::nakshatra(moon_sidereal);
    (NAKSHATRA_NAMES[n.index], n.pada, syllable(n.index, n.pada))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_complete() {
        for row in SYLLABLES.iter() {
            for s in row {
                assert!(!s.is_empty());
            }
        }
        assert_eq!(naming_for(0.5), ("Ashwini", 1, "Chu"));
        assert_eq!(naming_for(359.0), ("Revati", 4, "Chi"));
    }
}
