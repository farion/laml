//! Compact file-manager presentation; persistence stays in the library controller.
use super::*;
use egui_phosphor::regular::{
    ARROW_COUNTER_CLOCKWISE, DOTS_THREE, DOWNLOAD_SIMPLE, FILE_CODE, FOLDER, FOLDER_OPEN,
    MAGNIFYING_GLASS, PENCIL_SIMPLE, TRASH, UPLOAD_SIMPLE, X,
};

pub(super) enum Command {
    Open(String),
    Rename(String, String),
    Download(String),
    Trash(String),
    Restore(String),
    Delete(String),
    Clear,
    Empty,
    Import,
}
fn glyph(icon: &str) -> egui::RichText {
    egui::RichText::new(icon).font(egui::FontId::new(
        18.0,
        egui::FontFamily::Name("phosphor".into()),
    ))
}
fn tool(ui: &mut egui::Ui, enabled: bool, icon: &str, label: &str) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(glyph(icon)).min_size(egui::vec2(30.0, 30.0)),
    )
    .on_hover_text(crate::i18n::tr(&(label)))
    .clicked()
}
fn menu_item(ui: &mut egui::Ui, enabled: bool, icon: &str, label: &str) -> bool {
    let mut text = egui::text::LayoutJob::default();
    let color = ui.visuals().text_color();
    text.append(
        &format!("{icon}  "),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
            color,
            ..Default::default()
        },
    );
    text.append(
        crate::i18n::tr(label),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(14.0),
            color,
            ..Default::default()
        },
    );
    let clicked = ui.add_enabled(enabled, egui::Button::new(text)).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
fn file_menu(ui: &mut egui::Ui, record: &Record, safe: bool) -> Option<Command> {
    ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
    let id = &record.id;
    let mut action = None;
    if record.trashed {
        if menu_item(ui, safe, ARROW_COUNTER_CLOCKWISE, "Restore") {
            action = Some(Command::Restore(id.clone()));
        }
        ui.add(egui::Separator::default().spacing(8.0));
        if menu_item(ui, safe, TRASH, "Delete permanently…") {
            action = Some(Command::Delete(id.clone()));
        }
    } else {
        if menu_item(ui, true, FOLDER_OPEN, "Open") {
            action = Some(Command::Open(id.clone()));
        }
        if menu_item(ui, safe, PENCIL_SIMPLE, "Rename…") {
            action = Some(Command::Rename(id.clone(), record.name.clone()));
        }
        if menu_item(ui, true, DOWNLOAD_SIMPLE, "Download .laml") {
            action = Some(Command::Download(id.clone()));
        }
        ui.add(egui::Separator::default().spacing(8.0));
        if menu_item(ui, safe, TRASH, "Move to trash") {
            action = Some(Command::Trash(id.clone()));
        }
    }
    action
}
impl Library {
    pub(super) fn manager(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        safe: bool,
    ) -> Option<Command> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(glyph(MAGNIFYING_GLASS));
            // Reserve the real icon-button widths, including their padding, so
            // this row cannot widen the body beyond the dialog header.
            let tool_width = (18.0 + 2.0 * ui.spacing().button_padding.x)
                .max(ui.spacing().interact_size.x)
                .max(30.0);
            let reserved = 2.0 * (tool_width + ui.spacing().item_spacing.x);
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(crate::i18n::tr("Search files…"))
                    .desired_width((ui.available_width() - reserved).max(100.0)),
            );
            if tool(ui, !self.query.is_empty(), X, "Clear search") {
                self.query.clear();
            }
            if tool(ui, true, UPLOAD_SIMPLE, "Import .laml…") {
                action = Some(Command::Import);
            }
        });
        ui.add_space(8.0);
        let height = (ui.ctx().content_rect().height() - 300.0).clamp(100.0, 240.0);
        let query = self.query.trim().to_lowercase();
        let mut rows: Vec<_> = snapshot
            .records
            .iter()
            .filter(|r| r.trashed == self.trash && r.name.to_lowercase().contains(&query))
            .collect();
        rows.sort_by_cached_key(|r| (r.name.to_lowercase(), r.id.clone()));
        let selected = rows
            .iter()
            .copied()
            .find(|r| self.selected.as_deref() == Some(r.id.as_str()));
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), height + 38.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.set_max_height(height + 38.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(180.0, height + 38.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(180.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (trash, icon, title) in
                            [(false, FOLDER, "Documents"), (true, TRASH, "Trash")]
                        {
                            let count = snapshot
                                .records
                                .iter()
                                .filter(|r| r.trashed == trash)
                                .count();
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(180.0, 36.0), egui::Sense::click());
                            let selected = self.trash == trash;
                            if selected || response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    0.0,
                                    if selected {
                                        ui.visuals().selection.bg_fill
                                    } else {
                                        ui.visuals().widgets.hovered.bg_fill
                                    },
                                );
                            }
                            let color = if selected {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.painter().text(
                                egui::pos2(rect.left() + 12.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                icon,
                                egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
                                color,
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 42.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                crate::i18n::tr(&(title)),
                                egui::FontId::proportional(15.0),
                                color,
                            );
                            ui.painter().text(
                                egui::pos2(rect.right() - 12.0, rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                count.to_string(),
                                egui::FontId::proportional(15.0),
                                color,
                            );
                            if response.clicked() {
                                self.trash = trash;
                                self.selected = None;
                            }
                        }
                    },
                );
                let (divider, _) =
                    ui.allocate_exact_size(egui::vec2(1.0, height + 38.0), egui::Sense::hover());
                ui.painter().line_segment(
                    [divider.center_top(), divider.center_bottom()],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.weak(crate::i18n::tr("Name"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.weak(crate::i18n::tr("Diagram type"));
                        });
                    });
                    ui.separator();
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("library-files")
                                .auto_shrink([false, false])
                                .min_scrolled_height(height)
                                .max_height(height)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    if rows.is_empty() {
                                        ui.add_space(24.0);
                                        ui.vertical_centered(|ui| {
                                            ui.label(glyph(if self.trash {
                                                TRASH
                                            } else {
                                                FOLDER_OPEN
                                            }));
                                            ui.weak(if !query.is_empty() {
                                                crate::i18n::tr("No matching files.")
                                            } else if self.trash {
                                                crate::i18n::tr("Trash is empty.")
                                            } else {
                                                crate::i18n::tr("No documents yet.")
                                            });
                                        });
                                    }
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    for record in &rows {
                                        ui.push_id(&record.id, |ui| {
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(ui.available_width(), 36.0),
                                                egui::Sense::click(),
                                            );
                                            let selected =
                                                self.selected.as_deref() == Some(&record.id);
                                            if selected || response.hovered() {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    3.0,
                                                    if selected {
                                                        ui.visuals().selection.bg_fill
                                                    } else {
                                                        ui.visuals().widgets.hovered.bg_fill
                                                    },
                                                );
                                            }
                                            let color = if selected {
                                                ui.visuals().selection.stroke.color
                                            } else {
                                                ui.visuals().text_color()
                                            };
                                            ui.painter().text(
                                                egui::pos2(rect.left() + 10.0, rect.center().y),
                                                egui::Align2::LEFT_CENTER,
                                                FILE_CODE,
                                                egui::FontId::new(
                                                    19.0,
                                                    egui::FontFamily::Name("phosphor".into()),
                                                ),
                                                color,
                                            );
                                            let text_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.left() + 38.0, rect.top()),
                                                egui::pos2(rect.right() - 140.0, rect.bottom()),
                                            );
                                            ui.painter().with_clip_rect(text_rect).text(
                                                text_rect.left_center(),
                                                egui::Align2::LEFT_CENTER,
                                                &record.name,
                                                egui::FontId::proportional(14.0),
                                                color,
                                            );
                                            let notation =
                                                laml_ui::notation::Notation::of(&record.source);
                                            ui.painter().with_clip_rect(rect).text(
                                                egui::pos2(rect.right() - 10.0, rect.center().y),
                                                egui::Align2::RIGHT_CENTER,
                                                crate::i18n::tr(&(notation.title())),
                                                egui::FontId::proportional(12.0),
                                                ui.visuals().weak_text_color(),
                                            );
                                            if response.clicked() || response.secondary_clicked() {
                                                self.selected = Some(record.id.clone());
                                            }
                                            if response.double_clicked() && !self.trash {
                                                action = Some(Command::Open(record.id.clone()));
                                            }
                                            response.context_menu(|ui| {
                                                if let Some(command) = file_menu(ui, record, safe) {
                                                    action = Some(command);
                                                }
                                            });
                                        });
                                    }
                                });
                        });
                });
            },
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.weak(format!(
                "{} {}",
                rows.len(),
                crate::i18n::tr(if rows.len() == 1 { "file" } else { "files" })
            ));
            let (divider, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
            ui.painter().line_segment(
                [divider.center_top(), divider.center_bottom()],
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if self.trash {
                if tool(
                    ui,
                    safe && selected.is_some(),
                    ARROW_COUNTER_CLOCKWISE,
                    "Restore selected file",
                ) {
                    action = selected.map(|r| Command::Restore(r.id.clone()));
                }
                if tool(
                    ui,
                    safe && selected.is_some(),
                    TRASH,
                    "Delete selected file permanently…",
                ) {
                    action = selected.map(|r| Command::Delete(r.id.clone()));
                }
            } else {
                if tool(ui, selected.is_some(), FOLDER_OPEN, "Open selected file") {
                    action = selected.map(|r| Command::Open(r.id.clone()));
                }
                if tool(
                    ui,
                    safe && selected.is_some(),
                    PENCIL_SIMPLE,
                    "Rename selected file…",
                ) {
                    action = selected.map(|r| Command::Rename(r.id.clone(), r.name.clone()));
                }
                if tool(
                    ui,
                    selected.is_some(),
                    DOWNLOAD_SIMPLE,
                    "Download selected file",
                ) {
                    action = selected.map(|r| Command::Download(r.id.clone()));
                }
                if tool(
                    ui,
                    safe && selected.is_some(),
                    TRASH,
                    "Move selected file to trash",
                ) {
                    action = selected.map(|r| Command::Trash(r.id.clone()));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(crate::i18n::tr("Close")).clicked() {
                    self.open = false;
                }
                ui.menu_button(glyph(DOTS_THREE), |ui| {
                    ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
                    if self.trash && menu_item(ui, safe, TRASH, "Empty trash…") {
                        action = Some(Command::Empty);
                    }
                    if menu_item(ui, safe, TRASH, "Delete everything…") {
                        action = Some(Command::Clear);
                    }
                });
            });
        });
        action
    }
}
