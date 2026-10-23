//! Interface languages. Strings in the code stay English and are the lookup keys; a catalog per
//! language (`*.tsv`, format in `ko.tsv`) maps them to display text when the UI is drawn. Command
//! ids, document text, file names, the control channel, the CLI and MCP never see translated
//! text, so scripts and agents are unaffected. A string without a translation shows in English.
//!
//! Only English and Korean exist. The PPM build (`ppm-host` feature) always starts in Korean,
//! whatever the browser's language; every other build stays English. This is WordCraft's system
//! (`wordcraft/crates/ui-egui/src/i18n`) without the language setting and the operating-system
//! language match, which the PPM build must not have.
//!
//! # Looking strings up
//! - [`tl!`](crate::tl) / [`t`]: a string in the current language. [`tr`]: in a given one.
//! - [`location`]: a ribbon location such as `Home › Font`, segment by segment.
//! - [`fmt`]: fill `{name}` placeholders after a lookup; translators may reorder them.

mod catalog;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::sync::OnceLock;

use catalog::Catalog;

/// One supported interface language.
pub struct LangInfo {
    /// BCP 47 code, lowercase.
    pub code: &'static str,
    /// Catalog file contents (empty for the built-in English).
    pub source: &'static str,
    catalog: OnceLock<Catalog>,
}

/// The registry. English first: it is the fallback and the source language.
static LANGUAGES: [LangInfo; 2] = [
    LangInfo { code: "en", source: "", catalog: OnceLock::new() },
    LangInfo { code: "ko", source: include_str!("ko.tsv"), catalog: OnceLock::new() },
];

impl LangInfo {
    fn catalog(&self) -> &Catalog {
        self.catalog.get_or_init(|| {
            let (catalog, errors) = Catalog::parse(self.source);
            for error in errors {
                log::warn!("{} interface catalog: {error}; that entry shows in English", self.code);
            }
            catalog
        })
    }
}

/// A language the UI can be shown in (a handle into [`LANGUAGES`]).
#[derive(Clone, Copy)]
pub struct Lang(&'static LangInfo);

impl std::fmt::Debug for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lang({})", self.0.code)
    }
}

impl PartialEq for Lang {
    fn eq(&self, other: &Self) -> bool {
        self.0.code == other.0.code
    }
}

impl Eq for Lang {}

impl Lang {
    pub const EN: Lang = Lang(&LANGUAGES[0]);
    pub const KO: Lang = Lang(&LANGUAGES[1]);

    pub fn code(self) -> &'static str {
        self.0.code
    }

    fn catalog(self) -> &'static Catalog {
        self.0.catalog()
    }
}

/// The language a session starts in: Korean in the PPM build (`host_build`), English otherwise.
pub fn default_lang(host_build: bool) -> Lang {
    if host_build { Lang::KO } else { Lang::EN }
}

thread_local! {
    // Per thread: parallel test harnesses must not change each other's language. `None` until
    // somebody asks or sets, then the build's default.
    static CURRENT: Cell<Option<Lang>> = const { Cell::new(None) };
}

/// Set the language the UI is drawn in (this thread).
pub fn set_current(lang: Lang) {
    CURRENT.set(Some(lang));
}

/// The language the UI is drawn in: what [`set_current`] chose, else the build's default
/// ([`default_lang`]).
pub fn current() -> Lang {
    CURRENT.get().unwrap_or_else(|| default_lang(deckcraft_engine::hostlock::HOST_BUILD))
}

/// Does `lang` have a catalog entry for this string? (English never does: it is the source.)
pub fn has(lang: Lang, s: &str) -> bool {
    lang.catalog().plain(s).is_some()
}

/// `s` in the current language ([`tr`] with [`current`]).
pub fn t(s: &str) -> &str {
    tr(current(), s)
}

/// `s` in `lang`; strings without a translation come back unchanged.
pub fn tr(lang: Lang, s: &str) -> &str {
    lang.catalog().plain(s).unwrap_or(s)
}

/// A ribbon location (`Home › Font`) in the current language, segment by segment.
pub fn location(loc: &str) -> String {
    loc.split(" › ").map(t).collect::<Vec<_>>().join(" › ")
}

/// Fill `{name}` placeholders in one pass. Unknown placeholders stay as written, and inserted
/// values are never read as templates again, so a file name containing `{n}` stays intact.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once('{') {
        out.push_str(before);
        let Some((name, after_close)) = after_open.split_once('}') else {
            out.push('{');
            out.push_str(after_open);
            return out;
        };
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = after_close;
    }
    out.push_str(rest);
    out
}
