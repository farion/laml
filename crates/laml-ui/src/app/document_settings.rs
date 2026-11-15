//! Document properties shown when the canvas selection is empty.
use super::*;
use laml_ui::settings_edit;

impl Editor {
    pub(super) fn document_settings(&mut self, ui: &mut egui::Ui) {
        let Ok(diagram) = laml_ui::notation::parse_graph(self.document.source()) else {
            return;
        };
        let settings = diagram.settings;
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            if self.inspector_tab == inspector_ui::Tab::General {
                inspector_ui::category(ui, "Document layout");
                self.setting_choice(
                    ui,
                    &settings,
                    "direction",
                    "Direction",
                    &[
                        ("left-to-right", "Left to right"),
                        ("right-to-left", "Right to left"),
                        ("top-to-bottom", "Top to bottom"),
                        ("bottom-to-top", "Bottom to top"),
                    ],
                );
                self.setting_choice(
                    ui,
                    &settings,
                    "allow-diagonal",
                    "Diagonal",
                    &[("false", "Orthogonal"), ("true", "Allow diagonal")],
                );
                self.setting_choice(
                    ui,
                    &settings,
                    "relation-label",
                    "Label place",
                    &[("near", "Near"), ("along", "Along"), ("inline", "Inline")],
                );
                inspector_ui::category(ui, "Naming");
                self.setting_choice(
                    ui,
                    &settings,
                    "naming-convention",
                    "Convention",
                    &[
                        ("default", "Default"),
                        (
                            "uppercase-nodes-lowercase-relations",
                            "Uppercase nodes / lowercase relations",
                        ),
                    ],
                );
            } else {
                inspector_ui::category(ui, "Document theme");
                self.setting_choice(
                    ui,
                    &settings,
                    "theme",
                    "Theme",
                    &[
                        ("modern", "Modern"),
                        ("modern-dark", "Modern dark"),
                        ("modern-mono", "Modern mono"),
                        ("modern-mono-dark", "Modern mono dark"),
                        ("plain", "Plain"),
                        ("plain-dark", "Plain dark"),
                    ],
                );
                self.setting_choice(
                    ui,
                    &settings,
                    "background",
                    "Background",
                    &[("opaque", "Opaque"), ("transparent", "Transparent")],
                );
                inspector_ui::category(ui, "Fonts");
                for (name, title) in [
                    ("font-file", "Regular file"),
                    ("font-italic-file", "Italic file"),
                ] {
                    self.setting_text(ui, &settings, name, title);
                }
            }
        });
    }

    fn setting_choice(
        &mut self,
        ui: &mut egui::Ui,
        settings: &laml::syntax::Settings,
        name: &str,
        title: &str,
        choices: &[(&str, &str)],
    ) {
        let original = settings_edit::value(settings, name);
        let mut chosen = original.clone();
        let mut reset = false;
        ui.push_id(("document-setting", name), |ui| {
            inspector_ui::row(ui, title, |ui| {
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 36.0).max(48.0);
                    let title = choices
                        .iter()
                        .find(|(value, _)| *value == chosen)
                        .map_or(chosen.as_str(), |(_, title)| *title);
                    egui::ComboBox::from_id_salt("value")
                        .width(width)
                        .selected_text(title)
                        .show_ui(ui, |ui| {
                            for (value, title) in choices {
                                ui.selectable_value(&mut chosen, (*value).into(), *title);
                            }
                        });
                    reset = appearance::property_action(ui, true).clicked();
                });
            });
        });
        if reset || chosen != original {
            self.change_setting(name, (!reset).then_some(chosen.as_str()));
        }
    }

    fn setting_text(
        &mut self,
        ui: &mut egui::Ui,
        settings: &laml::syntax::Settings,
        name: &str,
        title: &str,
    ) {
        let original = settings_edit::value(settings, name);
        let draft = self
            .property_drafts
            .entry(format!("setting-{name}"))
            .or_insert_with(|| original.clone());
        let mut commit = false;
        let mut reset = false;
        inspector_ui::row(ui, title, |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 36.0).max(48.0);
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(width, appearance::FIELD_HEIGHT),
                    )),
                    |ui| {
                        let field =
                            inspector_ui::field(ui, ("document-setting", name), draft, false);
                        self.source_ui.extra_drafts.push(field.response.id);
                        commit = field.commit;
                    },
                );
                reset = appearance::property_action(ui, true).clicked();
            });
        });
        let value = draft.clone();
        if reset || (commit && value != original) {
            self.change_setting(name, (!reset).then_some(value.as_str()));
        }
    }

    fn change_setting(&mut self, name: &str, value: Option<&str>) {
        if !self.can_edit_visually() {
            return;
        }
        let result =
            settings_edit::property(self.document.source(), name, value).and_then(|edits| {
                let candidate = laml_ui::node_edit::candidate(self.document.source(), &edits)?;
                let expected = laml_ui::notation::parse_graph(&candidate)
                    .map_err(|e| e.message)?
                    .settings;
                Ok((edits, expected))
            });
        match result {
            Ok((edits, expected)) if !edits.is_empty() => self.submit_command(Command {
                edits,
                label: format!("Change document {name}"),
                expectation: worker::Expectation::Settings(expected),
            }),
            Ok(_) => {}
            Err(error) => self.command_error = Some(error),
        }
    }
}
