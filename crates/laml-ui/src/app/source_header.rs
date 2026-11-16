//! One source header; document names belong to the application tab bar.
use super::*;
use egui_phosphor::regular::{
    ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, CODE, FLOPPY_DISK, MAGNIFYING_GLASS, PENCIL_SIMPLE,
};

#[cfg(test)]
mod tests;

impl Editor {
    pub(super) fn source_heading(&mut self, ui: &mut egui::Ui) {
        let heading = egui::Frame::new()
            .fill(appearance::HEADER)
            .inner_margin(egui::Margin::symmetric(14, 6))
            .show(ui, |ui| {
                ui.set_min_height(appearance::HEADER_HEIGHT - 12.0);
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(crate::i18n::tr("Source"));
                    self.source_header_actions(ui);
                });
            });
        ui.painter().hline(
            heading.response.rect.x_range(),
            heading.response.rect.bottom(),
            egui::Stroke::new(1.0, appearance::BORDER),
        );
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn download_laml(&self) {
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        crate::browser_storage::download(
            &name,
            self.document.source().as_bytes(),
            "text/plain;charset=utf-8",
        );
    }
    pub(super) fn document_title_actions(&mut self, ui: &mut egui::Ui) {
        let can_save = self.pending_command != Some(self.document.revision())
            && !self.file_ui.busy()
            && !self.workflow.writing();
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            if workspace::icon(
                ui,
                egui_phosphor::regular::DOWNLOAD_SIMPLE,
                "Download LAML (Ctrl/Cmd+S)",
            )
            .clicked()
            {
                self.download_laml();
            }
        }
        if !laml_ui::services::browser_mode() {
            ui.add_enabled_ui(can_save, |ui| {
                if workspace::icon(ui, FLOPPY_DISK, "Save (Ctrl/Cmd+S)").clicked() {
                    self.save_file(ui.ctx());
                }
                let save_as = workspace::icon(ui, FLOPPY_DISK, "Save as… (Ctrl/Cmd+Shift+S)");
                let mark = egui::Rect::from_center_size(
                    save_as.rect.right_bottom() - egui::vec2(7.0, 7.0),
                    egui::vec2(14.0, 14.0),
                );
                ui.painter().rect_filled(mark, 2.0, appearance::HEADER);
                ui.painter().text(
                    mark.center(),
                    egui::Align2::CENTER_CENTER,
                    PENCIL_SIMPLE,
                    egui::FontId::new(14.0, egui::FontFamily::Name("phosphor".into())),
                    if can_save {
                        appearance::TEXT
                    } else {
                        appearance::MUTED
                    },
                );
                if save_as.clicked() {
                    self.save_as_dialog(ui.ctx());
                }
            });
        }
        ui.separator();
        ui.add_enabled_ui(self.document.undo_label().is_some(), |ui| {
            if workspace::icon(ui, ARROW_COUNTER_CLOCKWISE, "Undo (Ctrl/Cmd+Z)").clicked()
                && self.document.undo()
            {
                self.source_mutated();
            }
        });
        ui.add_enabled_ui(self.document.can_redo(), |ui| {
            if workspace::icon(ui, ARROW_CLOCKWISE, "Redo (Ctrl/Cmd+Shift+Z)").clicked()
                && self.document.redo()
            {
                self.source_mutated();
            }
        });
    }

    fn source_header_actions(&mut self, ui: &mut egui::Ui) {
        let items = [
            toolbar::Item {
                label: "Find / replace…",
                icon: Some(MAGNIFYING_GLASS),
                enabled: true,
                separator_before: false,
            },
            toolbar::Item {
                label: "Format source",
                icon: Some(CODE),
                enabled: !self.source_ui.composing,
                separator_before: false,
            },
        ];
        let mut clicked = None;
        toolbar::show_right(ui, &items, |ui, index, in_menu| {
            if toolbar::button(ui, &items[index], in_menu).clicked() {
                clicked = Some(index);
                if in_menu {
                    ui.close();
                }
            }
        });
        match clicked {
            Some(0) => {
                self.source_ui.find_open = !self.source_ui.find_open;
                self.source_ui.focus_find = self.source_ui.find_open;
                self.document.break_typing_group();
            }
            Some(1) => self.format_source(),
            _ => {}
        }
    }
}
