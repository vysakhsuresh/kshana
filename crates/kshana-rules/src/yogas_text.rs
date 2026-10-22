//! The yoga catalogue in Malayalam and Hindi. One row per catalogue key; the English
//! stays in [`super::CATALOG`]. Expert fields (`name`, `rule`, `source`) keep the
//! classical terms in each script; the plain fields (`title`, `meaning`, `care`) are
//! everyday speech and pass `plain::jargon_in`.

/// One yoga's words in one language. `care` is empty for a gift.
pub(crate) struct YogaText {
    pub name: &'static str,
    pub source: &'static str,
    pub rule: &'static str,
    pub title: &'static str,
    pub meaning: &'static str,
    pub care: &'static str,
}

/// `(key, Malayalam, Hindi)`.
pub(crate) static TEXT: &[(&str, YogaText, YogaText)] = &[];

/// The Malayalam and Hindi rows for a key.
pub(crate) fn text(key: &str) -> Option<(&'static YogaText, &'static YogaText)> {
    TEXT.iter().find(|r| r.0 == key).map(|r| (&r.1, &r.2))
}
