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

/// [`SYLLABLES`] in Malayalam script, cell for cell.
pub const SYLLABLES_ML: [[&str; 4]; 27] = [
    ["ചു", "ചേ", "ചോ", "ല"],
    ["ലി", "ലു", "ലേ", "ലോ"],
    ["അ", "ഇ", "ഉ", "എ"],
    ["ഒ", "വ", "വി", "വു"],
    ["വേ", "വോ", "ക", "കി"],
    ["കു", "ഘ", "ങ", "ഛ"],
    ["കേ", "കോ", "ഹ", "ഹി"],
    ["ഹു", "ഹേ", "ഹോ", "ഡ"],
    ["ഡി", "ഡു", "ഡേ", "ഡോ"],
    ["മ", "മി", "മു", "മേ"],
    ["മോ", "ട", "ടി", "ടു"],
    ["ടേ", "ടോ", "പ", "പി"],
    ["പു", "ഷ", "ണ", "ഠ"],
    ["പേ", "പോ", "ര", "രി"],
    ["രു", "രേ", "രോ", "ത"],
    ["തി", "തു", "തേ", "തോ"],
    ["ന", "നി", "നു", "നേ"],
    ["നോ", "യ", "യി", "യു"],
    ["യേ", "യോ", "ഭ", "ഭി"],
    ["ഭു", "ധ", "ഫ", "ഢ"],
    ["ഭേ", "ഭോ", "ജ", "ജി"],
    ["ജു", "ജേ", "ജോ", "ഘ"],
    ["ഗ", "ഗി", "ഗു", "ഗേ"],
    ["ഗോ", "സ", "സി", "സു"],
    ["സേ", "സോ", "ദ", "ദി"],
    ["ദു", "ഥ", "ഝ", "ദ"],
    ["ദേ", "ദോ", "ച", "ചി"],
];

/// [`SYLLABLES`] in Devanagari, cell for cell, as Hindi naming charts print them.
pub const SYLLABLES_HI: [[&str; 4]; 27] = [
    ["चू", "चे", "चो", "ला"],
    ["ली", "लू", "ले", "लो"],
    ["अ", "ई", "उ", "ए"],
    ["ओ", "वा", "वी", "वू"],
    ["वे", "वो", "का", "की"],
    ["कू", "घ", "ङ", "छ"],
    ["के", "को", "हा", "ही"],
    ["हू", "हे", "हो", "डा"],
    ["डी", "डू", "डे", "डो"],
    ["मा", "मी", "मू", "मे"],
    ["मो", "टा", "टी", "टू"],
    ["टे", "टो", "पा", "पी"],
    ["पू", "ष", "ण", "ठ"],
    ["पे", "पो", "रा", "री"],
    ["रू", "रे", "रो", "ता"],
    ["ती", "तू", "ते", "तो"],
    ["ना", "नी", "नू", "ने"],
    ["नो", "या", "यी", "यू"],
    ["ये", "यो", "भा", "भी"],
    ["भू", "धा", "फा", "ढा"],
    ["भे", "भो", "जा", "जी"],
    ["जू", "जे", "जो", "घा"],
    ["गा", "गी", "गू", "गे"],
    ["गो", "सा", "सी", "सू"],
    ["से", "सो", "दा", "दी"],
    ["दू", "थ", "झ", "दा"],
    ["दे", "दो", "चा", "ची"],
];

/// The syllable in `(Malayalam, Hindi)` script for a nakshatra index (0–26) and pada (1–4).
pub fn syllable_native(nakshatra: usize, pada: u8) -> (&'static str, &'static str) {
    let (n, p) = (nakshatra.min(26), (pada.clamp(1, 4) - 1) as usize);
    (SYLLABLES_ML[n][p], SYLLABLES_HI[n][p])
}

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
