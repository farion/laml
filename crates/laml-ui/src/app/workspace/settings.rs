//! Application settings navigation, shared by native and browser editors.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Category {
    #[default]
    Language,
    Theming,
}

pub(super) fn content(
    ui: &mut egui::Ui,
    category: &mut Category,
    preferences: &mut editor_theme::Preferences,
) {
    let width = ui.available_width();
    // Include the shared title row and 16-point frame padding in the 4:3 size.
    let height = ((width + 32.0) * 0.75 - 86.0).max(120.0);
    let sidebar = (width * 0.25).clamp(90.0, 150.0);
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.set_height(height);
            ui.allocate_ui_with_layout(
                egui::vec2(sidebar, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_height(height);
                    for (value, label) in [
                        (Category::Language, "Language"),
                        (Category::Theming, "Theming"),
                    ] {
                        if ui
                            .add_sized(
                                [sidebar, 32.0],
                                egui::Button::selectable(*category == value, "")
                                    .left_text(crate::i18n::tr(label)),
                            )
                            .clicked()
                        {
                            *category = value;
                        }
                    }
                },
            );
            ui.separator();
            ui.vertical(|ui| {
                ui.set_min_height(height);
                ui.set_width(ui.available_width());
                match category {
                    Category::Language => {
                        ui.heading(crate::i18n::tr("Language"));
                        ui.add_space(16.0);
                        ui.label(crate::i18n::tr("Language"));
                        egui::ComboBox::from_id_salt("editor-language")
                            .width(140.0)
                            .selected_text(preferences.language.label())
                            .show_ui(ui, |ui| {
                                for language in [
                                    crate::i18n::Language::Auto,
                                    crate::i18n::Language::English,
                                    crate::i18n::Language::German,
                                ] {
                                    ui.selectable_value(
                                        &mut preferences.language,
                                        language,
                                        language.label(),
                                    );
                                }
                            });
                    }
                    Category::Theming => {
                        ui.heading(crate::i18n::tr("Theming"));
                        ui.add_space(16.0);
                        ui.label(crate::i18n::tr("Editor theme"));
                        egui::ComboBox::from_id_salt("editor-theme")
                            .width(140.0)
                            .selected_text(preferences.editor_theme.label())
                            .show_ui(ui, |ui| {
                                for theme in [
                                    editor_theme::EditorTheme::Auto,
                                    editor_theme::EditorTheme::Dark,
                                    editor_theme::EditorTheme::Light,
                                ] {
                                    ui.selectable_value(
                                        &mut preferences.editor_theme,
                                        theme,
                                        theme.label(),
                                    );
                                }
                            });
                        ui.add_space(16.0);
                        ui.checkbox(
                            &mut preferences.fit_preview,
                            crate::i18n::tr("Ignore LAML theme and fit to editor theme"),
                        );
                    }
                }
            });
        },
    );
}
