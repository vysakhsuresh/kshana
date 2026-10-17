//! Calibration from the person's own record. Every window the app named can be
//! answered afterwards with one tap: it happened, or it did not. The record is scored
//! honestly (a window at or above good that came true is a hit; one below good that
//! did not happen is a hit too), and, once a life has five answers, the mean gap
//! between what happened and what was predicted becomes a small, bounded offset that
//! the screens show beside the score as "by your record". The core score is never
//! silently altered: the offset is shown, named and reversible.

use crate::forecast::Life;

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
        let words = if n == 0 {
            format!("No answers yet for {}.", life.name().to_ascii_lowercase())
        } else if n < 5 {
            format!("{} of {} right for {}; five answers make a record.", hits, n, life.name().to_ascii_lowercase())
        } else if offset.abs() < 0.01 {
            format!("{} of {} right for {}: the app reads your {} as it is.", hits, n, life.name().to_ascii_lowercase(), life.name().to_ascii_lowercase())
        } else if offset > 0.0 {
            format!("{} of {} right for {}: things went better than the app said, so your record lifts {} by {:.0}.", hits, n, life.name().to_ascii_lowercase(), life.name().to_ascii_lowercase(), offset * 100.0)
        } else {
            format!("{} of {} right for {}: things went harder than the app said, so your record lowers {} by {:.0}.", hits, n, life.name().to_ascii_lowercase(), life.name().to_ascii_lowercase(), -offset * 100.0)
        };
        n_all += n;
        hits_all += hits;
        lives.push(LifeCalibration { life, n, hits, rate, offset, words });
    }
    let words = if n_all == 0 {
        "No record yet. When a window the app named has passed, say whether it happened; five answers for a life make a record.".to_string()
    } else {
        format!(
            "{} of {} windows you judged came out as the app said ({:.0}%).",
            hits_all,
            n_all,
            100.0 * hits_all as f64 / n_all as f64
        )
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
