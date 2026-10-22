//! The lore of the planetary hour, and a name for every moment.
//!
//! Each hora lord carries a deity, a colour, a direction, a metal, a beeja mantra and
//! what the hour is for, from the Navagraha tradition (Agama beeja mantras, the
//! Navagraha dik and dhatu tables, Hora shastra). The moment's name is a short title
//! woven from its tara, its hour and the Moon's phase, so an instant reads like an
//! entry in a calendar of omens.


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
            deity: "Shiva, and Rama",
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
            deity: "Parvati",
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
            deity: "Dakshinamurti",
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
            deity: "Ayyappa, and Hanuman",
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

/// [`hora_lore`] in the reader's language. The mantra stays Sanskrit, written in the
/// reader's script; the source stays as it is (it is resolved, never shown as is).
pub fn hora_lore_in(lang: crate::plain::Lang, lord: DashaLord) -> HoraLore {
    let en = hora_lore(lord);
    let t: [[&'static str; 7]; 2] = match lord {
        DashaLord::Sun => [
            ["ശിവൻ, ശ്രീരാമൻ", "ചെമ്പിന്റെ ചുവപ്പ്", "കിഴക്ക്", "ചെമ്പ്", "ഓം ഹ്രാം ഹ്രീം ഹ്രൗം സഃ സൂര്യായ നമഃ", "അധികാരം, അപേക്ഷകൾ, ചികിത്സ, ഔദ്യോഗികമായ എന്തും", "മുതിർന്നവരോടും സർക്കാരിനോടുമുള്ള വഴക്ക്"],
            ["शिव और राम", "ताँबई लाल", "पूर्व", "ताँबा", "ॐ ह्रां ह्रीं ह्रौं सः सूर्याय नमः", "अधिकार, अर्ज़ियाँ, इलाज, हर सरकारी काम", "बड़ों और सरकार से झगड़ा"],
        ],
        DashaLord::Moon => [
            ["പാർവതി", "വെള്ള", "വടക്കുപടിഞ്ഞാറ്", "വെള്ളി", "ഓം ശ്രാം ശ്രീം ശ്രൗം സഃ ചന്ദ്രായ നമഃ", "കുടുംബം, യാത്ര, വെള്ളം, നടീൽ, മനസ്സിന്റെ കാര്യങ്ങൾ", "കടുത്ത വാക്കുകളും മനസ്സ് മോശമായിരിക്കുമ്പോൾ എടുക്കുന്ന തീരുമാനങ്ങളും"],
            ["पार्वती", "सफ़ेद", "उत्तर-पश्चिम", "चाँदी", "ॐ श्रां श्रीं श्रौं सः चन्द्राय नमः", "परिवार, यात्रा, पानी, बुआई, दिल के मामले", "कड़वे बोल और मूड में लिए फ़ैसले"],
        ],
        DashaLord::Mars => [
            ["സുബ്രഹ്മണ്യൻ, ഹനുമാൻ", "ചുവപ്പ്", "തെക്ക്", "ചെമ്പ്", "ഓം ക്രാം ക്രീം ക്രൗം സഃ ഭൗമായ നമഃ", "അധ്വാനം, ധൈര്യം, ശസ്ത്രക്രിയ, ഭൂമി, ജയിക്കേണ്ട ഒരു പോരാട്ടം", "പുതിയ പങ്കാളിത്തവും ദേഷ്യവും"],
            ["सुब्रह्मण्य और हनुमान", "लाल", "दक्षिण", "ताँबा", "ॐ क्रां क्रीं क्रौं सः भौमाय नमः", "मेहनत, हिम्मत, ऑपरेशन, ज़मीन, वह लड़ाई जो जीतनी ही है", "नई साझेदारी और गुस्सा"],
        ],
        DashaLord::Mercury => [
            ["വിഷ്ണു", "പച്ച", "വടക്ക്", "ഓട്", "ഓം ബ്രാം ബ്രീം ബ്രൗം സഃ ബുധായ നമഃ", "കച്ചവടം, എഴുത്ത്, പഠനം, സന്ദേശങ്ങൾ, ഇടപാടുകൾ", "ധൃതിയിൽ ഒപ്പിടൽ"],
            ["विष्णु", "हरा", "उत्तर", "काँसा", "ॐ ब्रां ब्रीं ब्रौं सः बुधाय नमः", "व्यापार, लिखना, पढ़ाई, संदेश और सौदे", "जल्दबाज़ी में दस्तख़त"],
        ],
        DashaLord::Jupiter => [
            ["ദക്ഷിണാമൂർത്തി", "മഞ്ഞ", "വടക്കുകിഴക്ക്", "സ്വർണം", "ഓം ഗ്രാം ഗ്രീം ഗ്രൗം സഃ ഗുരവേ നമഃ", "പഠനം, പണം, മക്കൾ, അധ്യാപകർ, നല്ല തുടക്കങ്ങൾ", "അഹങ്കാരവും പാലിക്കാനാകാത്ത വാഗ്ദാനങ്ങളും"],
            ["दक्षिणामूर्ति", "पीला", "उत्तर-पूर्व", "सोना", "ॐ ग्रां ग्रीं ग्रौं सः गुरवे नमः", "पढ़ाई, पैसा, संतान, शिक्षक और शुभ शुरुआत", "घमंड और ऐसे वादे जो निभ न सकें"],
        ],
        DashaLord::Venus => [
            ["ലക്ഷ്മി", "തിളങ്ങുന്ന വെള്ള", "തെക്കുകിഴക്ക്", "വെള്ളി", "ഓം ദ്രാം ദ്രീം ദ്രൗം സഃ ശുക്രായ നമഃ", "പ്രണയം, കല, വാങ്ങലുകൾ, സന്തോഷം, പങ്കാളിത്തം", "പ്രിയപ്പെട്ടയാളുമായുള്ള വഴക്ക്"],
            ["लक्ष्मी", "चमकीला सफ़ेद", "दक्षिण-पूर्व", "चाँदी", "ॐ द्रां द्रीं द्रौं सः शुक्राय नमः", "प्रेम, कला, ख़रीदारी, सुख और साझेदारी", "अपने प्रिय से झगड़ा"],
        ],
        DashaLord::Saturn => [
            ["അയ്യപ്പൻ, ഹനുമാൻ", "നീലകലർന്ന കറുപ്പ്", "പടിഞ്ഞാറ്", "ഇരുമ്പ്", "ഓം പ്രാം പ്രീം പ്രൗം സഃ ശനൈശ്ചരായ നമഃ", "പതിവുജോലി, സേവനം, മുതിർന്നവർ, അധ്വാനം, ക്ഷമ", "തുടക്കങ്ങൾ, കടം, കുറുക്കുവഴികൾ"],
            ["अय्यप्पा और हनुमान", "नीला-काला", "पश्चिम", "लोहा", "ॐ प्रां प्रीं प्रौं सः शनैश्चराय नमः", "रोज़ का काम, सेवा, बुज़ुर्ग, मेहनत और धैर्य", "नई शुरुआत, कर्ज़ और शॉर्टकट"],
        ],
        DashaLord::Rahu => [
            ["ദുർഗ", "പുകനിറം", "തെക്കുപടിഞ്ഞാറ്", "ഈയം", "ഓം ഭ്രാം ഭ്രീം ഭ്രൗം സഃ രാഹവേ നമഃ", "വിദേശവും അസാധാരണവുമായ കാര്യങ്ങൾ", "ചതി"],
            ["दुर्गा", "धुएँ जैसा", "दक्षिण-पश्चिम", "सीसा", "ॐ भ्रां भ्रीं भ्रौं सः राहवे नमः", "विदेश और अनोखे काम", "धोखा"],
        ],
        DashaLord::Ketu => [
            ["ഗണപതി", "പലനിറം", "വടക്കുപടിഞ്ഞാറ്", "ഈയം", "ഓം സ്രാം സ്രീം സ്രൗം സഃ കേതവേ നമഃ", "വിട്ടുകൊടുക്കലും ഉള്ളിലേക്കുള്ള നോട്ടവും", "തുടക്കങ്ങൾ"],
            ["गणेश", "चितकबरा", "उत्तर-पश्चिम", "सीसा", "ॐ स्रां स्रीं स्रौं सः केतवे नमः", "छोड़ना और भीतर का काम", "नई शुरुआत"],
        ],
    };
    let row = match lang {
        crate::plain::Lang::En => return en,
        crate::plain::Lang::Ml => t[0],
        crate::plain::Lang::Hi => t[1],
    };
    HoraLore {
        lord,
        deity: row[0],
        colour: row[1],
        direction: row[2],
        metal: row[3],
        mantra: row[4],
        for_: row[5],
        avoid: row[6],
        source: en.source,
    }
}

/// [`moment_title`] in the reader's language.
pub fn moment_title_in(lang: crate::plain::Lang, tara: usize, hora: Option<DashaLord>) -> String {
    match lang {
        crate::plain::Lang::En => moment_title(tara, hora, true, ""),
        _ => crate::lang::moment_title(lang, tara, hora),
    }
}

/// A name for the moment in plain words: "A giving day · an hour for money and blessings".
/// The arguments stay as the engine knows them; the words never name them.
pub fn moment_title(tara: usize, hora: Option<DashaLord>, _waxing: bool, _star: &str) -> String {
    let day = crate::plain::cap(crate::plain::day_short(tara));
    match hora {
        Some(l) => format!("{day} · {}", crate::plain::hour_short(l)),
        None => day,
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
        assert_eq!(moment_title(1, Some(DashaLord::Moon), true, "Mula"), "A giving day · an hour for home");
        assert_eq!(moment_title(8, None, true, "Revati"), "One of your best days");
        for l in DashaLord::ORDER {
            let h = hora_lore(l);
            for s in [h.deity, h.colour, h.direction, h.metal, h.for_, h.avoid] {
                assert_eq!(crate::plain::jargon(s), None, "{s}");
            }
            for t in 0..9 {
                assert_eq!(crate::plain::jargon(&moment_title(t, Some(l), true, "Mula")), None);
            }
        }
    }
}
