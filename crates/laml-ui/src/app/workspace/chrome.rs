//! Global actions and document tabs, including the empty workspace.
use super::*;

impl Workspace {
    pub(super) fn chrome(&mut self, ui: &mut egui::Ui) {
        let locked = self.welcome_open
            || self.opening.is_some()
            || self.file_picker.is_some()
            || self.waiting_close.is_some()
            || self
                .tabs
                .get(self.active)
                .is_some_and(Editor::close_dialog_active);
        let mut action = None;
        egui::Panel::top("global-title")
            .exact_size(44.0)
            .show_separator_line(false)
            .frame(
                appearance::panel_frame(appearance::HEADER)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "LAML Editor",
                    egui::FontId::proportional(16.0),
                    appearance::TEXT,
                );
                ui.horizontal_centered(|ui| {
                    ui.add_enabled_ui(!locked, |ui| {
                        ui.horizontal(|ui| {
                            if !laml_ui::services::browser_mode() {
                                self.recovery_button(ui);
                            }
                            #[cfg(target_arch = "wasm32")]
                            if laml_ui::services::browser_mode()
                                && icon(ui, egui_phosphor::regular::BOOKS, "Browser library")
                                    .clicked()
                            {
                                self.library.open = true;
                            }
                            if icon(ui, PLUS, "New LAML (Ctrl/Cmd+N)").clicked() {
                                action = Some(Action::New);
                            }
                            if icon(ui, FOLDER_OPEN, "Open LAML (Ctrl/Cmd+O)").clicked() {
                                self.open_dialog(ui.ctx());
                            }
                            ui.separator();
                            if let Some(editor) = self.tabs.get_mut(self.active) {
                                editor.document_title_actions(ui);
                            }
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon(ui, QUESTION, "Help").clicked() {
                            self.help_open = true;
                        }
                        if icon(ui, GEAR_SIX, "Settings").clicked() {
                            self.settings_open = true;
                        }
                        ui.add_enabled_ui(!locked, |ui| {
                            if let Some(editor) = self.tabs.get_mut(self.active) {
                                editor.export_controls(ui);
                            }
                        });
                    });
                });
            });
        if let Some(Action::New) = action {
            self.begin_new_diagram();
        }
        if self.tabs.is_empty() {
            return;
        }
        let mut selected = None;
        let mut closed = None;
        let reveal = std::mem::take(&mut self.reveal_active);
        egui::Panel::top("document-tabs")
            .exact_size(38.0)
            .frame(appearance::panel_frame(appearance::HEADER))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.add_enabled_ui(!locked, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("open-files")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.0;
                                for (index, editor) in self.tabs.iter().enumerate() {
                                    ui.push_id(editor.id, |ui| {
                                        let active = index == self.active;
                                        let response = egui::Frame::new()
                                            .fill(if active {
                                                appearance::PANEL
                                            } else {
                                                appearance::HEADER
                                            })
                                            .inner_margin(egui::Margin::symmetric(8, 2))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 6.0;
                                                    let name = editor
                                                        .path
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy();
                                                    let label = format!(
                                                        "{name}{}{}",
                                                        if editor.recovery.recovered
                                                            && editor.file.path.is_none()
                                                        {
                                                            " (recovered)"
                                                        } else {
                                                            ""
                                                        },
                                                        if !laml_ui::services::browser_mode()
                                                            && editor.document.is_dirty()
                                                        {
                                                            " •"
                                                        } else {
                                                            ""
                                                        }
                                                    );
                                                    if ui
                                                        .add(egui::Button::new(label).frame(false))
                                                        .on_hover_text(
                                                            editor.path.display().to_string(),
                                                        )
                                                        .clicked()
                                                    {
                                                        selected = Some(index);
                                                    }
                                                    if icon(ui, X, "Close file").clicked() {
                                                        closed = Some(index);
                                                    }
                                                });
                                            });
                                        if active {
                                            if reveal {
                                                response
                                                    .response
                                                    .scroll_to_me(Some(egui::Align::Center));
                                            }
                                            ui.painter().hline(
                                                response.response.rect.x_range(),
                                                response.response.rect.bottom() - 1.0,
                                                egui::Stroke::new(2.0, appearance::ACCENT),
                                            );
                                        }
                                    });
                                }
                            });
                        });
                });
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    egui::Stroke::new(1.0, appearance::BORDER),
                );
            });
        if let Some(index) = selected {
            self.active = index;
            self.reveal_active = true;
        }
        if let Some(index) = closed {
            self.request_close(index, ui.ctx());
        }
    }
}
