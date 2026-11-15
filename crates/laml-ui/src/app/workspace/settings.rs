//! Application settings navigation, shared by native and browser editors.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Category {
    #[default]
    Theming,
    About,
}

pub(super) fn content(ui: &mut egui::Ui, category: &mut Category, preferences: &mut editor_theme::Preferences) {
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
                    for (value, label) in
                        [(Category::Theming, "Theming"), (Category::About, "About")]
                    {
                        if ui
                            .add_sized(
                                [sidebar, 32.0],
                                egui::Button::selectable(*category == value, "").left_text(label),
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
                    Category::Theming => {
                        ui.heading("Theming");
                        ui.add_space(16.0);
                        ui.label("Editor theme");
                        egui::ComboBox::from_id_salt("editor-theme")
                            .selected_text(preferences.editor_theme.label())
                            .show_ui(ui, |ui| {
                                for theme in [editor_theme::EditorTheme::Auto,
                                    editor_theme::EditorTheme::Dark, editor_theme::EditorTheme::Light] {
                                    ui.selectable_value(&mut preferences.editor_theme, theme, theme.label());
                                }
                            });
                        ui.add_space(16.0);
                        ui.checkbox(&mut preferences.fit_preview, "Ignore LAML theme and fit to editor theme");
                    }
                    Category::About => {
                        ui.heading(format!("LAML Editor {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(16.0);
                        ui.hyperlink("https://github.com/farion/laml");
                        ui.add_space(16.0);
                        ui.label("LAML License MIT");
                        ui.label("Editor License GPL");
                        ui.add_space(16.0);
                        ui.label("Maintainer: Frieder Reinhold<reinhold@trigon-media.com>");
                    }
                }
            });
        },
    );
}
