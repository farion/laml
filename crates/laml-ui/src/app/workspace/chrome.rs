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
                    crate::i18n::tr("LAML Editor"),
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
                            if let Some(editor) = self.tabs.get_mut(self.active) {
                                ui.separator();
                                editor.document_title_actions(ui);
                            }
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon(ui, egui_phosphor::regular::INFO, "About").clicked() {
                            self.about_open = true;
                        }
                        if icon(ui, QUESTION, "Help").clicked() {
                            self.help_open = true;
                        }
                        if icon(ui, GEAR_SIX, "Settings").clicked() {
                            self.settings_open = true;
                        }
                        let download = ui.button(crate::i18n::tr("Download LAML for local use"));
                        paint_download_glow(ui, &download);
                        if download.clicked() {
                            self.download.show(ui.ctx());
                        }
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

/// A short, softly fading comet follows the actual rounded button border.
fn paint_download_glow(ui: &egui::Ui, response: &egui::Response) {
    const PERIOD: f64 = 4.0;
    const LAP: f64 = 2.6;
    let phase = ui.input(|input| input.time).rem_euclid(PERIOD);
    if phase >= LAP {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(PERIOD - phase));
        return;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(30));
    let rect = response.rect.shrink(0.5);
    let radius = f32::from(ui.style().interact(response).corner_radius.nw)
        .min(rect.height() / 2.0)
        .min(rect.width() / 2.0);
    let straight_x = rect.width() - 2.0 * radius;
    let straight_y = rect.height() - 2.0 * radius;
    let arc = std::f32::consts::FRAC_PI_2 * radius;
    let perimeter = 2.0 * (straight_x + straight_y) + 4.0 * arc;
    if perimeter <= 0.0 {
        return;
    }
    let point = |distance: f32| {
        let mut distance = distance.rem_euclid(perimeter);
        let centers = [
            egui::pos2(rect.right() - radius, rect.top() + radius),
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            egui::pos2(rect.left() + radius, rect.top() + radius),
        ];
        let starts = [
            egui::pos2(rect.left() + radius, rect.top()),
            egui::pos2(rect.right(), rect.top() + radius),
            egui::pos2(rect.right() - radius, rect.bottom()),
            egui::pos2(rect.left(), rect.bottom() - radius),
        ];
        let directions = [
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, 1.0),
            egui::vec2(-1.0, 0.0),
            egui::vec2(0.0, -1.0),
        ];
        for side in 0..4 {
            let length = if side % 2 == 0 {
                straight_x
            } else {
                straight_y
            };
            if distance <= length {
                return starts[side] + directions[side] * distance;
            }
            distance -= length;
            if distance <= arc && radius > 0.0 {
                let angle = (side as f32 - 1.0) * std::f32::consts::FRAC_PI_2 + distance / radius;
                return centers[side] + egui::vec2(angle.cos(), angle.sin()) * radius;
            }
            distance -= arc;
        }
        starts[0]
    };
    let fade = ((phase / 0.18).min((LAP - phase) / 0.3).min(1.0)) as f32;
    let head = (phase / LAP) as f32 * perimeter;
    let color = ui.visuals().selection.stroke.color;
    let painter = ui
        .painter()
        .with_clip_rect(response.rect.expand(6.0).intersect(ui.clip_rect()));
    for index in (0..24).rev() {
        let strength = (1.0 - index as f32 / 24.0).powi(2) * fade;
        let position = point(head - index as f32 * 1.4);
        painter.circle_filled(position, 4.5, color.gamma_multiply(0.035 * strength));
        painter.circle_filled(position, 2.5, color.gamma_multiply(0.12 * strength));
        painter.circle_filled(position, 1.0, color.gamma_multiply(0.8 * strength));
    }
    painter.circle_filled(point(head), 1.4, color.gamma_multiply(fade));
}
