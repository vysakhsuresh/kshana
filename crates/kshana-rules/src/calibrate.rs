//! Calibration from the person's own record. Every window the app named can be
//! answered afterwards with one tap: it happened, or it did not. The record is scored
//! honestly (a window at or above good that came true is a hit; one below good that
//! did not happen is a hit too), and, once a life has five answers, the mean gap
//! between what happened and what was predicted becomes a small, bounded offset that
//! the screens show beside the score as "by your record". The core score is never
//! silently altered: the offset is shown, named and reversible.

use crate::forecast::Life;
use crate::plain::Lang;

/// One answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    /// Which life.
    pub life: Life,
    /// What the app predicted for the window, `[0, 1]`.
    pub predicted: f64,
    /// Whether it happened.
    pub happened: bool,
}

/// One life's record.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeCalibration {
    /// Which life.
    pub life: Life,
    /// Answers.
    pub n: usize,
    /// Hits.
    pub hits: usize,
    /// Hit rate in `[0, 1]` (0.5 when there are no answers).
    pub rate: f64,
    /// Offset to show beside the score, `[-0.08, 0.08]`; zero under five answers.
    pub offset: f64,
    /// In words.
    pub words: String,
}

/// The whole record.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    /// Per life, `Life::ALL` order.
    pub lives: Vec<LifeCalibration>,
    /// Answers in all.
    pub n: usize,
    /// Hits in all.
    pub hits: usize,
    /// In words.
    pub words: String,
}

/// Score a record.
pub fn calibrate(records: &[Record]) -> Calibration {
    calibrate_in(records, Lang::En)
}

fn life_words(lang: Lang, life: Life, hits: usize, n: usize, offset: f64) -> String {
    let en = life.name().to_ascii_lowercase();
    let l = crate::lang::life_name(lang, life);
    let pts = (offset.abs() * 100.0).round();
    match lang {
        Lang::En => {
            if n == 0 {
                format!("No answers yet for {en}.")
            } else if n < 5 {
                format!("{hits} of {n} right for {en}; five answers make a record.")
            } else if offset.abs() < 0.01 {
                format!("{hits} of {n} right for {en}: the app reads your {en} as it is.")
            } else if offset > 0.0 {
                format!("{hits} of {n} right for {en}: things went better than the app said, so your record lifts {en} by {:.0}.", offset * 100.0)
            } else {
                format!("{hits} of {n} right for {en}: things went harder than the app said, so your record lowers {en} by {:.0}.", -offset * 100.0)
            }
        }
        Lang::Ml => {
            if n == 0 {
                format!("{l}: ഇതുവരെ ഉത്തരങ്ങളൊന്നുമില്ല.")
            } else if n < 5 {
                format!("{l}: {n}-ൽ {hits} ശരിയായി; അഞ്ച് ഉത്തരങ്ങളാകുമ്പോൾ ഒരു രേഖയാകും.")
            } else if offset.abs() < 0.01 {
                format!("{l}: {n}-ൽ {hits} ശരിയായി; ആപ്പ് നിങ്ങളുടെ {l} ഉള്ളതുപോലെ തന്നെ വായിക്കുന്നു.")
            } else if offset > 0.0 {
                format!("{l}: {n}-ൽ {hits} ശരിയായി. ആപ്പ് പറഞ്ഞതിനേക്കാൾ നന്നായി കാര്യങ്ങൾ നടന്നു, അതിനാൽ നിങ്ങളുടെ രേഖ {l} {pts:.0} പോയിന്റ് ഉയർത്തുന്നു.")
            } else {
                format!("{l}: {n}-ൽ {hits} ശരിയായി. ആപ്പ് പറഞ്ഞതിനേക്കാൾ പ്രയാസമായാണ് കാര്യങ്ങൾ നടന്നത്, അതിനാൽ നിങ്ങളുടെ രേഖ {l} {pts:.0} പോയിന്റ് താഴ്ത്തുന്നു.")
            }
        }
        Lang::Hi => {
            if n == 0 {
                format!("{l}: अभी तक कोई जवाब नहीं।")
            } else if n < 5 {
                format!("{l}: {n} में से {hits} सही; पाँच जवाबों से एक रिकॉर्ड बनता है।")
            } else if offset.abs() < 0.01 {
                format!("{l}: {n} में से {hits} सही; ऐप आपके {l} को जैसा है वैसा ही पढ़ता है।")
            } else if offset > 0.0 {
                format!("{l}: {n} में से {hits} सही। बातें ऐप के कहे से बेहतर रहीं, इसलिए आपका रिकॉर्ड {l} को {pts:.0} अंक ऊपर करता है।")
            } else {
                format!("{l}: {n} में से {hits} सही। बातें ऐप के कहे से कठिन रहीं, इसलिए आपका रिकॉर्ड {l} को {pts:.0} अंक नीचे करता है।")
            }
        }
    }
}

/// Score a record, its words in the reader's language.
pub fn calibrate_in(records: &[Record], lang: Lang) -> Calibration {
    let mut lives = Vec::with_capacity(9);
    let mut n_all = 0;
    let mut hits_all = 0;
    for life in Life::ALL {
        let rs: Vec<&Record> = records.iter().filter(|r| r.life == life).collect();
        let n = rs.len();
        let hits = rs.iter().filter(|r| (r.predicted >= 0.56) == r.happened).count();
        let rate = if n == 0 { 0.5 } else { hits as f64 / n as f64 };
        let bias = if n == 0 {
            0.0
        } else {
            rs.iter().map(|r| (if r.happened { 1.0 } else { 0.0 }) - r.predicted).sum::<f64>() / n as f64
        };
        let offset = if n >= 5 { (0.5 * bias).clamp(-0.08, 0.08) } else { 0.0 };
        let words = life_words(lang, life, hits, n, offset);
        n_all += n;
        hits_all += hits;
        lives.push(LifeCalibration { life, n, hits, rate, offset, words });
    }
    let words = if n_all == 0 {
        lang.pick([
            "No record yet. When a window the app named has passed, say whether it happened; five answers for a life make a record.",
            "ഇതുവരെ രേഖയൊന്നുമില്ല. ആപ്പ് പറഞ്ഞ ഒരു സമയം കഴിഞ്ഞാൽ അത് നടന്നോ എന്ന് പറയൂ; ഒരു കാര്യത്തിന് അഞ്ച് ഉത്തരങ്ങളാകുമ്പോൾ ഒരു രേഖയാകും.",
            "अभी कोई रिकॉर्ड नहीं। ऐप का बताया कोई समय बीत जाए, तो बताइए कि वह हुआ या नहीं; किसी एक बात के पाँच जवाबों से रिकॉर्ड बनता है।",
        ])
        .to_string()
    } else {
        let pct = 100.0 * hits_all as f64 / n_all as f64;
        match lang {
            Lang::En => format!("{hits_all} of {n_all} windows you judged came out as the app said ({pct:.0}%)."),
            Lang::Ml => format!("നിങ്ങൾ വിലയിരുത്തിയ {n_all} സമയങ്ങളിൽ {hits_all} എണ്ണം ആപ്പ് പറഞ്ഞതുപോലെ നടന്നു ({pct:.0}%)."),
            Lang::Hi => format!("आपने जिन {n_all} समयों को परखा, उनमें से {hits_all} ऐप के कहे जैसे निकले ({pct:.0}%)."),
        }
    };
    Calibration { lives, n: n_all, hits: hits_all, words }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_and_offsets_are_counted_honestly() {
        let mut rs = Vec::new();
        for _ in 0..6 {
            rs.push(Record { life: Life::Love, predicted: 0.45, happened: true });
        }
        rs.push(Record { life: Life::Career, predicted: 0.7, happened: true });
        rs.push(Record { life: Life::Career, predicted: 0.7, happened: false });
        let c = calibrate(&rs);
        assert_eq!(c.n, 8);
        let love = &c.lives[0];
        assert_eq!((love.n, love.hits), (6, 0));
        assert!(love.offset > 0.07 && love.offset <= 0.08, "{}", love.offset);
        let career = &c.lives[1];
        assert_eq!((career.n, career.hits), (2, 1));
        assert_eq!(career.offset, 0.0);
        assert!(c.words.contains("1 of 8"));
    }
}
