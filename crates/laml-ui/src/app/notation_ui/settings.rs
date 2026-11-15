//! Document settings remain available even when a value is not written yet.
use super::*;
impl Editor {
    pub(super) fn notation_document_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            if self.inspector_tab == inspector_ui::Tab::Style {
                inspector_ui::category(ui, "Document theme");
                self.notation_setting_choice(
                    ui,
                    "theme",
                    "Theme",
                    "modern",
                    &[
                        ("modern", "Modern"),
                        ("modern-dark", "Modern dark"),
                        ("modern-mono", "Modern mono"),
                        ("modern-mono-dark", "Modern mono dark"),
                        ("plain", "Plain"),
                        ("plain-dark", "Plain dark"),
                    ],
                );
                self.notation_setting_choice(
                    ui,
                    "background",
                    "Background",
                    "opaque",
                    &[("opaque", "Opaque"), ("transparent", "Transparent")],
                );
                inspector_ui::category(ui, "Fonts");
                self.notation_setting_text(ui, "font-file", "Regular file", "", true);
                self.notation_setting_text(ui, "font-italic-file", "Italic file", "", true);
            } else {
                inspector_ui::category(ui, "Document layout");
                self.notation_setting_choice(
                    ui,
                    "allow-diagonal",
                    "Diagonal",
                    "false",
                    &[("false", "Orthogonal"), ("true", "Allow diagonal")],
                );
                match self.notation() {
                    Notation::Bpmn => self.notation_setting_choice(
                        ui,
                        "direction",
                        "Direction",
                        "left-to-right",
                        &[
                            ("left-to-right", "Left to right"),
                            ("top-to-bottom", "Top to bottom"),
                        ],
                    ),
                    Notation::Sequence => self.notation_setting_choice(
                        ui,
                        "semantic-check",
                        "Loop bounds",
                        "on",
                        &[("on", "Check bounds"), ("off", "Do not check bounds")],
                    ),
                    Notation::Timing => {
                        self.notation_setting_text(
                            ui,
                            "interaction",
                            "Interaction",
                            "Interaction",
                            false,
                        );
                        self.notation_setting_text(ui, "time-unit", "Time unit", "", false);
                    }
                    ty if ty.c4() => {
                        self.notation_setting_text(ui, "scope", "Scope", "", false);
                        self.notation_setting_choice(
                            ui,
                            "direction",
                            "Direction",
                            "left-to-right",
                            &[
                                ("left-to-right", "Left to right"),
                                ("right-to-left", "Right to left"),
                                ("top-to-bottom", "Top to bottom"),
                                ("bottom-to-top", "Bottom to top"),
                            ],
                        );
                        self.notation_setting_choice(
                            ui,
                            "relation-label",
                            "Label place",
                            "near",
                            &[("near", "Near"), ("along", "Along"), ("inline", "Inline")],
                        );
                    }
                    _ => {}
                }
            }
        });
    }
    fn authored_setting(&self, key: &str, default: &str) -> String {
        edit::records(self.document.source())
            .into_iter()
            .find(|r| {
                r.section == "settings"
                    && r.heading
                        .split_once(':')
                        .is_some_and(|(name, _)| name.trim() == key)
            })
            .and_then(|r| {
                edit::fields::of(self.document.source(), r.line)
                    .into_iter()
                    .next()
                    .map(|f| f.value)
            })
            .unwrap_or_else(|| default.into())
    }
    fn notation_setting_choice(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        title: &str,
        default: &str,
        choices: &[(&str, &str)],
    ) {
        let original = self.authored_setting(key, default);
        let mut value = original.clone();
        let mut reset = false;
        inspector_ui::row(ui, title, |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 34.0).max(40.0);
                egui::ComboBox::from_id_salt(("notation-setting", key))
                    .width(width)
                    .selected_text(
                        choices
                            .iter()
                            .find(|(v, _)| *v == value)
                            .map_or(value.as_str(), |(_, title)| *title),
                    )
                    .show_ui(ui, |ui| {
                        for (v, title) in choices {
                            ui.selectable_value(&mut value, (*v).into(), *title);
                        }
                    });
                reset = appearance::property_action(ui, true).clicked();
            });
        });
        if reset || value != original {
            self.notation_change_setting(key, (!reset).then_some(value.as_str()));
        }
    }
    fn notation_setting_text(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        title: &str,
        default: &str,
        quoted: bool,
    ) {
        let original = self.authored_setting(key, default);
        let draft_key = format!("notation-document-{key}");
        let mut value = self
            .property_drafts
            .get(&draft_key)
            .cloned()
            .unwrap_or_else(|| original.clone());
        let mut commit = false;
        let mut reset = false;
        inspector_ui::row(ui, title, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut value)
                        .desired_width((ui.available_width() - 34.0).max(40.0)),
                );
                self.source_ui.extra_drafts.push(response.id);
                commit = response.lost_focus()
                    || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                reset = appearance::property_action(ui, true).clicked();
            });
        });
        self.property_drafts.insert(draft_key, value.clone());
        if reset || (commit && value != original) {
            let source_value = if quoted {
                laml_ui::relation_edit::quote(&value)
            } else {
                value.clone()
            };
            self.notation_change_setting(
                key,
                (!reset && !value.trim().is_empty()).then_some(source_value.as_str()),
            );
        }
    }
    fn notation_change_setting(&mut self, key: &str, value: Option<&str>) {
        match edit::setting(self.document.source(), key, value) {
            Ok(edits) => self.notation_submit("Change document setting", edits, None),
            Err(e) => self.command_error = Some(e),
        }
    }
}
