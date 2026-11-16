use super::*;
use crate::i18n::{Language, scoped_language};

fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
    fn collect(shape: &egui::Shape, output: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => output.push((
                text.galley.job.text.clone(),
                egui::Rect::from_min_size(text.pos, text.galley.size()),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, output)),
            _ => {}
        }
    }
    let mut output = Vec::new();
    for shape in shapes {
        collect(&shape.shape, &mut output);
    }
    output
}
#[test]
fn legacy_preferences_default_to_auto_and_manual_language_roundtrips() {
    let prefs: editor_theme::Preferences =
        serde_json::from_str(r#"{"editor_theme":"Light","fit_preview":true,"welcome_seen":true}"#)
            .unwrap();
    assert_eq!(prefs.language, Language::Auto);
    assert!(prefs.welcome_seen && prefs.fit_preview);
    let prefs = editor_theme::Preferences {
        language: Language::German,
        ..prefs
    };
    let saved = serde_json::to_string(&prefs).unwrap();
    let loaded: editor_theme::Preferences = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded, prefs);
}
#[test]
fn settings_show_language_before_theming_and_localized_choices() {
    let ctx = egui::Context::default();
    Editor::install_appearance(&ctx);
    let _language = scoped_language(Language::German);
    let mut category = settings::Category::default();
    assert!(category == settings::Category::Language);
    let mut prefs = editor_theme::Preferences::default();
    let mut draw = |events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 650.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                ui.set_width(600.0);
                settings::content(ui, &mut category, &mut prefs);
            },
        );
        output.textures_delta.clear();
        output
    };
    draw(vec![]);
    let output = draw(vec![]);
    let labels = texts(&output.shapes);
    let language = labels
        .iter()
        .find(|(s, _)| s == "Sprache")
        .unwrap_or_else(|| {
            panic!(
                "settings labels: {labels:?}, language: {:?}",
                crate::i18n::current()
            )
        })
        .1;
    let theming = labels.iter().find(|(s, _)| s == "Theming").unwrap().1;
    assert!(language.top() < theming.top());
    let auto = labels.iter().find(|(s, _)| s == "Auto").unwrap().1.center();
    for pressed in [true, false] {
        draw(vec![
            egui::Event::PointerMoved(auto),
            egui::Event::PointerButton {
                pos: auto,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }
    let output = draw(vec![]);
    let labels = texts(&output.shapes);
    assert!(labels.iter().any(|(s, _)| s == "Englisch"));
    let german = labels
        .iter()
        .find(|(s, _)| s == "Deutsch")
        .unwrap()
        .1
        .center();
    for pressed in [true, false] {
        draw(vec![
            egui::Event::PointerMoved(german),
            egui::Event::PointerButton {
                pos: german,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }
    assert_eq!(prefs.language, Language::German);
}
#[test]
fn language_switch_does_not_change_document_undo_or_diagram_theme() {
    let _language = scoped_language(Language::English);
    let ctx = egui::Context::default();
    let source = "settings {\n theme: plain\n}\nsemantics {\n Class(label=\"Source\")\n}\n";
    let mut workspace = Workspace::new(Editor::new(source.into(), None));
    workspace.preferences.language = Language::English;
    workspace.sync_editor_theme(&ctx);
    let revision = workspace.tabs[0].document.revision();
    workspace.preferences.language = Language::German;
    workspace.sync_editor_theme(&ctx);
    assert_eq!(crate::i18n::tr("Source"), "Quelltext");
    assert_eq!(workspace.tabs[0].document.source(), source);
    assert_eq!(workspace.tabs[0].document.revision(), revision);
    assert!(!workspace.tabs[0].document.is_dirty());
    assert!(workspace.tabs[0].document.undo_label().is_none());
    assert_eq!(
        workspace.tabs[0].compiler.presentation,
        laml_ui::backend::Presentation::Document
    );
    workspace.preferences.language = Language::English;
    workspace.sync_editor_theme(&ctx);
    assert_eq!(crate::i18n::tr("Source"), "Source");
}
