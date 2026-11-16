//! Client-side editor language. Never translates source, identifiers or compiler protocol data.
use std::{cell::Cell, collections::BTreeMap, sync::OnceLock};

mod locale;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    Auto,
    English,
    German,
}
impl Language {
    pub fn label(self) -> &'static str {
        tr(match self {
            Self::Auto => "Auto",
            Self::English => "English",
            Self::German => "German",
        })
    }
    pub fn resolve(self, languages: &[String]) -> Self {
        if self != Self::Auto {
            return self;
        }
        for language in languages {
            let base = language.split(['-', '_', '.', '@']).next().unwrap_or("");
            if base.eq_ignore_ascii_case("de") {
                return Self::German;
            }
            if base.eq_ignore_ascii_case("en") || base == "C" || base == "POSIX" {
                return Self::English;
            }
        }
        Self::English
    }
    pub fn apply(self) {
        let resolved = self.resolve(&locale::languages());
        ACTIVE.with(|active| {
            if active.replace(resolved) != resolved {
                locale::set_document_language(resolved);
            }
        });
    }
}
thread_local! {
    // Language belongs to the UI thread/client, never the shared compiler server.
    static ACTIVE: Cell<Language> = const { Cell::new(Language::English) };
}
pub fn current() -> Language {
    ACTIVE.with(Cell::get)
}
#[cfg(test)]
pub(crate) fn scoped_language(language: Language) -> impl Drop {
    struct Reset(Language);
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|active| active.set(self.0));
        }
    }
    Reset(ACTIVE.with(|active| active.replace(language)))
}
fn catalog() -> &'static BTreeMap<String, String> {
    static CATALOG: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("i18n/de.json")).expect("German UI catalog")
    })
}
/// Only use for interface-owned text, not document names or user-authored labels.
pub fn tr(text: &str) -> &str {
    if current() == Language::German {
        catalog().get(text).map_or(text, String::as_str)
    } else {
        text
    }
}
/// Formats translated UI templates while leaving every inserted value untouched.
pub fn format(template: &str, values: &[(&str, String)]) -> String {
    let template = tr(template);
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|end| start + end) else {
            output.push_str(&rest[start..]);
            return output;
        };
        let name = &rest[start + 1..end];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            output.push_str(value);
        } else {
            output.push_str(&rest[start..=end]);
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}
/// Translate diagnostics at the UI boundary; retain the original compiler detail.
/// Dynamic fields are copied verbatim, never recursively translated or rewritten.
pub fn message(text: &str) -> String {
    let exact = tr(text);
    if exact != text || current() != Language::German {
        return exact.to_owned();
    }
    for (template, translated) in catalog() {
        if !template.contains('{') {
            continue;
        }
        if let Some(values) = capture(template, text) {
            return format(translated, &values);
        }
    }
    text.to_owned()
}
fn capture<'a>(template: &'a str, text: &str) -> Option<Vec<(&'a str, String)>> {
    let mut rest = text;
    let mut pattern = template;
    let mut values = Vec::new();
    while let Some(start) = pattern.find('{') {
        rest = rest.strip_prefix(&pattern[..start])?;
        let end = pattern[start..].find('}')? + start;
        let name = &pattern[start + 1..end];
        pattern = &pattern[end + 1..];
        let literal = pattern.split('{').next()?;
        let size = if literal.is_empty() {
            if pattern.is_empty() {
                rest.len()
            } else {
                return None;
            }
        } else if !pattern.contains('{') {
            rest.strip_suffix(literal)?.len()
        } else {
            rest.find(literal)?
        };
        values.push((name, rest[..size].to_owned()));
        rest = &rest[size..];
    }
    (rest == pattern).then_some(values)
}
