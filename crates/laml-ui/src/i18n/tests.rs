use super::*;

struct Restore(Language);
impl Drop for Restore {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(self.0));
    }
}
fn language(language: Language) -> Restore {
    Restore(ACTIVE.with(|active| active.replace(language)))
}
#[test]
fn auto_respects_preference_order_regions_and_fallback() {
    let resolve = |tags: &[&str]| {
        Language::Auto.resolve(&tags.iter().map(|s| (*s).into()).collect::<Vec<_>>())
    };
    for tag in ["de", "de-DE", "de_AT.UTF-8", "DE-ch", "de_DE@euro"] {
        assert_eq!(resolve(&[tag]), Language::German);
    }
    assert_eq!(resolve(&["en-US", "de-DE"]), Language::English);
    assert_eq!(resolve(&["fr-FR", "de-DE", "en"]), Language::German);
    for tags in [
        vec![],
        vec!["fr-FR"],
        vec!["C.UTF-8"],
        vec!["POSIX"],
        vec!["debug"],
    ] {
        assert_eq!(resolve(&tags), Language::English);
    }
    assert_eq!(
        Language::German.resolve(&["en-US".into()]),
        Language::German
    );
    assert_eq!(
        Language::English.resolve(&["de-DE".into()]),
        Language::English
    );
}
#[test]
fn exact_translation_and_english_fallback() {
    let _restore = language(Language::German);
    assert_eq!(tr("Settings"), "Einstellungen");
    assert_eq!(tr("Language"), "Sprache");
    assert_eq!(tr("English"), "Englisch");
    assert_eq!(tr("German"), "Deutsch");
    assert_eq!(tr("BPMN"), "BPMN");
    assert_eq!(tr("a user-defined name"), "a user-defined name");
    ACTIVE.with(|a| a.set(Language::English));
    assert_eq!(tr("Settings"), "Settings");
}
#[test]
fn inserted_values_are_never_translated_or_reinterpreted() {
    let _restore = language(Language::German);
    assert_eq!(
        format(
            "Permanently delete {name}?",
            &[("name", "Source {error}".into())]
        ),
        "Source {error} endgültig löschen?"
    );
    assert_eq!(
        message("Permanently delete Source {error}?"),
        "Source {error} endgültig löschen?"
    );
    assert_eq!(
        format(
            "{0} of {1} selected",
            &[("0", "2".into()), ("1", "5".into())]
        ),
        "2 von 5 ausgewählt"
    );
    assert_eq!(
        message("Could not save editor settings: disk full"),
        "Editor-Einstellungen konnten nicht gespeichert werden: disk full"
    );
    assert_eq!(
        message("unknown compiler diagnostic 'Source'"),
        "unknown compiler diagnostic 'Source'"
    );
}
#[test]
fn catalog_templates_keep_all_placeholders() {
    fn fields(text: &str) -> Vec<&str> {
        let mut rest = text;
        let mut result = Vec::new();
        while let Some(start) = rest.find('{') {
            let end = rest[start..].find('}').unwrap() + start;
            result.push(&rest[start + 1..end]);
            rest = &rest[end + 1..];
        }
        result.sort_unstable();
        result
    }
    for (key, value) in catalog() {
        assert!(!value.is_empty(), "Empty German translation: {key}");
        assert_eq!(fields(key), fields(value), "Changed placeholders: {key}");
    }
}
#[test]
fn every_notation_and_semantic_relation_has_a_catalog_entry() {
    use crate::notation::Notation;
    for kind in crate::element_edit::ElementKind::ALL {
        assert!(
            catalog().contains_key(kind.title()),
            "Missing element title: {}",
            kind.title()
        );
    }
    for (_, title) in crate::notation_edit::bpmn::TYPES {
        assert!(
            catalog().contains_key(*title),
            "Missing BPMN title: {title}"
        );
    }
    for notation in Notation::ALL {
        for recipe in crate::notation_edit::catalog::recipes(notation) {
            assert!(
                catalog().contains_key(recipe.title),
                "Missing recipe title: {}",
                recipe.title
            );
            for parameter in crate::notation_edit::catalog::parameters(recipe) {
                let title = parameter.replace('_', " ");
                assert!(
                    catalog().contains_key(&title),
                    "Missing parameter title: {title}"
                );
            }
        }
        let title = notation.title();
        assert!(
            catalog().contains_key(title) || ["BPMN", "Container"].contains(&title),
            "Missing diagram title: {title}"
        );
        for relation in notation.relation_types() {
            assert!(
                catalog().contains_key(relation.title),
                "Missing relationship title: {}",
                relation.title
            );
        }
    }
}
