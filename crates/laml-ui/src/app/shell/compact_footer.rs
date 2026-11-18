//! Small footer keeps status visible and places complete action labels in a menu.
use super::*;
impl Editor {
    pub(super) fn compact_footer(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter().hline(
            rect.x_range(),
            rect.top(),
            egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
        );
        let mut footer = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("compact-footer")
                .max_rect(rect.shrink2(egui::vec2(12.0, 4.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let response = workspace::icon(
            &mut footer,
            egui_phosphor::regular::DOTS_THREE,
            "More actions",
        );
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(220.0);
            ui.menu_button(
                toolbar::menu_text(ui, "Zoom", egui_phosphor::regular::MAGNIFYING_GLASS),
                |ui| {
                    self.view_menu(ui);
                },
            );
            if self.notation() != laml_ui::notation::Notation::Timing {
                ui.menu_button(
                    toolbar::menu_text(
                        ui,
                        "Default Relation Type",
                        egui_phosphor::regular::ARROW_RIGHT,
                    ),
                    |ui| {
                        self.arrow_type_control(ui, ui.available_width().clamp(160.0, 270.0));
                    },
                );
            }
            ui.separator();
            ui.label(crate::i18n::tr("LAML · UTF-8"));
            if !laml_ui::services::browser_mode() && self.document.is_dirty() {
                ui.label(crate::i18n::tr("Unsaved changes"));
            }
            #[cfg(target_arch = "wasm32")]
            if laml_ui::services::browser_mode() {
                ui.label(crate::i18n::message(
                    &crate::browser_storage::storage_status(),
                ));
            }
        });
        footer.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            footer_status::show(ui, self);
        });
    }
}
