//! "Where this comes from" over the FFI: the book, chapter and verse (or the custom, or
//! the app's own method) behind a claim, from the keys the records carry
//! (`source_keys`). See `kshana_rules::sources` for the registry and its numbering.
//!
//! Every field here is shown only behind the "Where this comes from" tap and names the
//! books by their titles, so the plain-words audits leave `source_keys` and these
//! records out (book titles such as "Brihat Parashara Hora Shastra" are the point here).

use crate::Engine;
use kshana_rules::sources::cite;
use kshana_rules::Lang;

/// One source, said in the reader's language.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SourceRecord {
    /// Stable key ("bphs.dashas", "lal_kitab", "kshana.consensus", "yoga.gaja_kesari").
    pub key: String,
    /// "text" (a classical or standard text), "folk" (a later folk text), "custom"
    /// (temple or household custom, not a text) or "kshana" (the app's own method).
    pub kind: String,
    /// The book, the custom, or "Kshana's own method".
    pub book: String,
    /// "ch. 46", "chs. 51–60", or the chapter's title; none when not checked.
    pub chapter: Option<String>,
    /// "v. 55–60", only where the engine's own tables cite a verse.
    pub verse: Option<String>,
    /// What the source says that the rule uses, in one line.
    pub note: String,
    /// The citation in one line: "Brihat Parashara Hora Shastra, ch. 46".
    pub line: String,
}

/// Keys as strings, for the records.
pub(crate) fn keys(k: &[&str]) -> Vec<String> {
    k.iter().map(|s| s.to_string()).collect()
}

/// The sources behind `keys` (from any record's `source_keys`), each once, in order,
/// in `lang` ("en", "ml", "hi"). Unknown keys are skipped. Needs no ephemeris.
#[uniffi::export]
pub fn source_lines(keys: Vec<String>, lang: String) -> Vec<SourceRecord> {
    cite(&keys, Lang::parse(&lang))
        .into_iter()
        .map(|c| SourceRecord { key: c.key, kind: c.kind.to_string(), book: c.book, chapter: c.chapter, verse: c.verse, note: c.note, line: c.line })
        .collect()
}

#[uniffi::export]
impl Engine {
    /// The same as [`source_lines`], on the engine.
    pub fn sources(&self, keys: Vec<String>, lang: String) -> Vec<SourceRecord> {
        source_lines(keys, lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_resolve_to_lines_in_three_languages() {
        for lang in ["en", "ml", "hi"] {
            let s = source_lines(vec!["life.dasha".into(), "kshana.consensus".into(), "custom.temple".into(), "lal_kitab".into(), "nope".into()], lang.into());
            assert_eq!(s.len(), 6, "{s:#?}");
            assert!(s.iter().all(|r| !r.line.is_empty() && ["text", "folk", "custom", "kshana"].contains(&r.kind.as_str())));
        }
        let en = source_lines(vec!["bphs.dashas".into()], "en".into());
        assert_eq!(en[0].line, "Brihat Parashara Hora Shastra, ch. 46");
        assert_eq!(en[0].chapter.as_deref(), Some("ch. 46"));
    }
}
