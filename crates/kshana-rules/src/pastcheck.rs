//! Past checks: before predicting anything, ask the person about three to five real
//! stretches of their last six years that the reading marks most strongly, a dip or a
//! peak in a part of life, as plain yes-or-no questions. A reading that gets the past
//! right earns trust for the future; the answers also feed the person's record.
//!
//! The stretches come from the slow series (periods and slow movers, no daily weather),
//! sampled monthly: for each life the three-month window furthest below and above that
//! life's own six-year mean. The strongest go first, one per life, never overlapping in
//! time by more than half, with at least one good stretch when there is one.

use crate::forecast::{slow_series, ForecastError, Life, Natal};
use crate::lang::month_year;
use crate::plain::Lang;
use kshana_ephem::Spk;
use kshana_sky::Observer;
use kshana_time::{EopTable, Instant, NS_PER_DAY};
use kshana_vedic::Ayanamsa;

/// One question about the past.
#[derive(Clone, Debug, PartialEq)]
pub struct PastCheck {
    /// First day of the stretch.
    pub from: Instant,
    /// Last day of the stretch.
    pub to: Instant,
    /// Which life.
    pub life: Life,
    /// The question, in plain words.
    pub question: String,
    /// True for a good stretch, false for a hard one.
    pub good: bool,
    /// How far the stretch stood from the life's own average, in points.
    pub strength: f64,
}

type T3 = [&'static str; 3];

fn rest(life: Life, good: bool) -> T3 {
    use Life::*;
    match (life, good) {
        (Love, false) => ["was there strain, distance or a break in love or marriage?", "പ്രണയത്തിലോ വിവാഹത്തിലോ ഉലച്ചിലോ അകൽച്ചയോ വേർപിരിയലോ ഉണ്ടായോ?", "क्या प्रेम या विवाह में तनाव, दूरी या टूटन आई?"],
        (Love, true) => ["did love bring something good: a new bond, a marriage, or real closeness?", "പ്രണയം എന്തെങ്കിലും നല്ലത് കൊണ്ടുവന്നോ: പുതിയ ബന്ധം, വിവാഹം, അല്ലെങ്കിൽ യഥാർത്ഥ അടുപ്പം?", "क्या प्रेम कुछ अच्छा लाया: नया रिश्ता, विवाह, या सच्ची नज़दीकी?"],
        (Career, false) => ["was work hard: a setback, a bad boss, a lost job or effort that went unrewarded?", "ജോലി കഠിനമായിരുന്നോ: ഒരു തിരിച്ചടി, മോശം മേലധികാരി, ജോലി നഷ്ടം, അല്ലെങ്കിൽ ഫലം കിട്ടാത്ത അധ്വാനം?", "क्या काम कठिन रहा: कोई झटका, बुरा बॉस, नौकरी जाना या बिना फल की मेहनत?"],
        (Career, true) => ["did work move up: a new job, a promotion, or recognition?", "ജോലിയിൽ ഉയർച്ചയുണ്ടായോ: പുതിയ ജോലി, സ്ഥാനക്കയറ്റം, അല്ലെങ്കിൽ അംഗീകാരം?", "क्या काम में तरक्की हुई: नई नौकरी, पदोन्नति या पहचान?"],
        (Money, false) => ["was money tight: losses, debts or expenses bigger than income?", "പണം ഞെരുക്കത്തിലായിരുന്നോ: നഷ്ടം, കടം, അല്ലെങ്കിൽ വരുമാനത്തേക്കാൾ വലിയ ചെലവ്?", "क्या पैसा तंग रहा: नुकसान, कर्ज़ या आमदनी से बड़ा ख़र्च?"],
        (Money, true) => ["did money come in well: a raise, a gain or a good deal?", "പണം നന്നായി വന്നോ: ശമ്പളവർധന, ലാഭം, അല്ലെങ്കിൽ നല്ല ഇടപാട്?", "क्या पैसा अच्छा आया: वेतन वृद्धि, लाभ या अच्छा सौदा?"],
        (Health, false) => ["was there an illness, an injury or a long spell of low energy?", "അസുഖമോ പരിക്കോ നീണ്ട ക്ഷീണകാലമോ ഉണ്ടായോ?", "क्या कोई बीमारी, चोट या लंबी कमज़ोरी रही?"],
        (Health, true) => ["were you unusually well and full of energy?", "പതിവിലും നല്ല ആരോഗ്യവും ഊർജവും ഉണ്ടായിരുന്നോ?", "क्या आप असामान्य रूप से स्वस्थ और ऊर्जा से भरे रहे?"],
        (Family, false) => ["was there trouble at home: quarrels, an elder's illness, or someone moving away?", "വീട്ടിൽ പ്രശ്നമുണ്ടായോ: വഴക്ക്, മുതിർന്നവരുടെ അസുഖം, അല്ലെങ്കിൽ ആരെങ്കിലും അകന്നുപോകൽ?", "क्या घर में परेशानी रही: झगड़े, किसी बड़े की बीमारी, या किसी का दूर जाना?"],
        (Family, true) => ["was home a happy place: a celebration, a reunion or a new member?", "വീട് സന്തോഷമുള്ള ഇടമായിരുന്നോ: ഒരു ആഘോഷം, ഒത്തുചേരൽ, അല്ലെങ്കിൽ പുതിയ അംഗം?", "क्या घर ख़ुशियों भरा रहा: कोई उत्सव, मिलन या नया सदस्य?"],
        (Study, false) => ["did studies or exams go badly, or did focus fall apart?", "പഠനമോ പരീക്ഷയോ മോശമായോ, ശ്രദ്ധ തകർന്നോ?", "क्या पढ़ाई या परीक्षा ख़राब गई, या ध्यान बिखर गया?"],
        (Study, true) => ["did you pass, qualify or learn something that changed your path?", "നിങ്ങൾ ജയിച്ചോ, യോഗ്യത നേടിയോ, അല്ലെങ്കിൽ വഴി മാറ്റിയ എന്തെങ്കിലും പഠിച്ചോ?", "क्या आप पास हुए, योग्यता पाई, या कुछ ऐसा सीखा जिसने राह बदली?"],
        (Home, false) => ["were there problems with your home, land, rent or a move?", "വീട്, ഭൂമി, വാടക, അല്ലെങ്കിൽ താമസംമാറ്റം എന്നിവയിൽ പ്രശ്നങ്ങളുണ്ടായോ?", "क्या घर, ज़मीन, किराए या शिफ़्टिंग में परेशानी रही?"],
        (Home, true) => ["did you buy, build or move into a better home?", "നിങ്ങൾ മെച്ചപ്പെട്ട ഒരു വീട് വാങ്ങുകയോ പണിയുകയോ അതിലേക്ക് മാറുകയോ ചെയ്തോ?", "क्या आपने बेहतर घर ख़रीदा, बनाया या उसमें गए?"],
        (Travel, false) => ["did a journey go wrong, or did plans to travel or move abroad fall through?", "ഒരു യാത്ര പിഴച്ചോ, അല്ലെങ്കിൽ യാത്രയ്ക്കോ വിദേശത്തേക്ക് പോകാനോ ഉള്ള പദ്ധതി മുടങ്ങിയോ?", "क्या कोई यात्रा बिगड़ी, या यात्रा या विदेश जाने की योजना अटक गई?"],
        (Travel, true) => ["did you travel far, or did a move abroad work out?", "നിങ്ങൾ ദൂരയാത്ര ചെയ്തോ, അല്ലെങ്കിൽ വിദേശത്തേക്കുള്ള നീക്കം വിജയിച്ചോ?", "क्या आपने दूर की यात्रा की, या विदेश जाना सफल रहा?"],
        (Children, false) => ["did a child have a hard time, or was there worry about having children?", "ഒരു കുട്ടിക്ക് കഷ്ടകാലമുണ്ടായോ, അല്ലെങ്കിൽ മക്കളുണ്ടാകുന്നതിനെക്കുറിച്ച് ആശങ്കയുണ്ടായോ?", "क्या किसी बच्चे का समय कठिन रहा, या संतान को लेकर चिंता रही?"],
        (Children, true) => ["was there joy from children: a birth, a milestone, a success?", "മക്കളിൽ നിന്ന് സന്തോഷമുണ്ടായോ: ഒരു ജനനം, ഒരു നാഴികക്കല്ല്, ഒരു വിജയം?", "क्या बच्चों से ख़ुशी मिली: जन्म, कोई उपलब्धि, कोई सफलता?"],
    }
}

/// The question for a stretch, in the reader's language.
pub fn question(lang: Lang, life: Life, good: bool, from: Instant, to: Instant, lon: f64) -> String {
    let a = month_year(lang, from, lon);
    let b = month_year(lang, to, lon);
    let r = lang.pick(rest(life, good));
    match lang {
        Lang::En => format!("Between {a} and {b}, {r}"),
        Lang::Ml => format!("{a} മുതൽ {b} വരെ, {r}"),
        Lang::Hi => format!("{a} से {b} के बीच, {r}"),
    }
}

/// Three to five questions about the six years before `at`.
#[allow(clippy::too_many_arguments)]
pub fn past_checks(spk: &Spk, eop: Option<&EopTable>, natal: &Natal, obs: &Observer, ayanamsa: Ayanamsa, lang: Lang, at: Instant) -> Result<Vec<PastCheck>, ForecastError> {
    let month = (30.4375 * NS_PER_DAY as f64) as i128;
    let n = 72usize;
    let start = at.add_ns(-(n as i128) * month);
    // Never before the person was born plus a few years.
    let start = if start < natal.birth.add_ns(5 * 365 * NS_PER_DAY) { natal.birth.add_ns(5 * 365 * NS_PER_DAY) } else { start };
    if start >= at {
        return Ok(Vec::new());
    }
    let count = ((at.ns_since(start) / month) as usize).clamp(6, n);
    let s = slow_series(spk, eop, natal, obs, ayanamsa, start, month, count)?;
    let w = 3usize;
    let mut cands: Vec<(f64, usize, usize, bool)> = Vec::new(); // (strength, life, first index, good)
    for (li, _) in Life::ALL.iter().enumerate() {
        let vals: Vec<f64> = s.iter().map(|(_, v)| v[li]).collect();
        if vals.len() < w + 1 {
            continue;
        }
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let means: Vec<f64> = (0..=vals.len() - w).map(|i| vals[i..i + w].iter().sum::<f64>() / w as f64).collect();
        if let Some((i, v)) = means.iter().enumerate().min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            cands.push(((mean - v) * 100.0, li, i, false));
        }
        if let Some((i, v)) = means.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            cands.push(((v - mean) * 100.0, li, i, true));
        }
    }
    cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut chosen: Vec<(f64, usize, usize, bool)> = Vec::new();
    let overlaps = |a: usize, b: usize| (a as i64 - b as i64).unsigned_abs() < 2;
    // At least one good stretch first, when there is a real one.
    if let Some(g) = cands.iter().find(|c| c.3 && c.0 >= 2.0) {
        chosen.push(*g);
    }
    for c in &cands {
        if chosen.len() >= 5 {
            break;
        }
        if c.0 < 2.0 && chosen.len() >= 3 {
            break;
        }
        if chosen.iter().any(|x| x.1 == c.1 || overlaps(x.2, c.2)) {
            continue;
        }
        chosen.push(*c);
    }
    chosen.sort_by_key(|c| c.2);
    let lon = obs.longitude_deg;
    Ok(chosen
        .into_iter()
        .map(|(strength, li, i, good)| {
            let from = s[i].0;
            let to = s[(i + w - 1).min(s.len() - 1)].0.add_ns(month - NS_PER_DAY);
            let life = Life::ALL[li];
            PastCheck { from, to, life, question: question(lang, life, good, from, to, lon), good, strength }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::jargon_in;
    use kshana_houses::HouseSystem;
    use kshana_time::Utc;

    fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    #[test]
    fn the_past_is_asked_about_plainly_one_life_at_a_time() {
        let Some(spk) = kernel() else { return };
        let obs = Observer { latitude_deg: 9.9312, longitude_deg: 76.2673, height_m: 0.0 };
        let birth = Utc::new(1990, 2, 20, 5, 4, 0, 0).unwrap().to_instant().unwrap();
        let natal = Natal::compute(&spk, None, birth, &obs, HouseSystem::Placidus, Ayanamsa::Lahiri).unwrap();
        let at = Utc::new(2026, 10, 17, 5, 0, 0, 0).unwrap().to_instant().unwrap();
        for lang in [Lang::En, Lang::Ml, Lang::Hi] {
            let c = past_checks(&spk, None, &natal, &obs, Ayanamsa::Lahiri, lang, at).unwrap();
            assert!((3..=5).contains(&c.len()), "{lang:?}: {c:?}");
            for x in &c {
                assert!(x.from < x.to && x.to <= at.add_ns(31 * NS_PER_DAY));
                assert_eq!(jargon_in(lang, &x.question), None, "{}", x.question);
            }
            let lives: std::collections::HashSet<_> = c.iter().map(|x| x.life).collect();
            assert_eq!(lives.len(), c.len());
        }
    }
}
