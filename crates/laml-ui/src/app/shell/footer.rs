//! Footer actions adapt without clipping status or controls.
use super::*;
impl Editor {
    pub(super) fn footer(&mut self, ui: &mut egui::Ui, rect: egui::Rect, left: f32, right: f32) {
        if rect.width() < 960.0 || self.responsive.properties_hidden {
            self.compact_footer(ui, rect);
            return;
        }
        ui.painter().hline(
            rect.x_range(),
            rect.top(),
            egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
        );
        let rect = rect.shrink2(egui::vec2(0.0, 8.0));
        let source_rect = egui::Rect::from_min_max(
            rect.min + egui::vec2(14.0, 0.0),
            egui::pos2(left - 8.0, rect.bottom()),
        );
        let mut status = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("footer-status")
                .max_rect(source_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        status.set_clip_rect(source_rect);
        footer_status::show(&mut status, self);
        let text_color = appearance::color(ui.ctx(), appearance::TEXT);
        let label_width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(
                    crate::i18n::tr("Default Relation Type").into(),
                    egui::FontId::proportional(15.0),
                    text_color,
                )
                .size()
                .x
        });
        let has_relations = self.notation() != laml_ui::notation::Notation::Timing;
        let center_width = right - left;
        let picker_width = 270.0_f32.min((center_width - 180.0).max(100.0));
        let show_relation_label = center_width >= label_width + 270.0 + 192.0;
        let relation_width = if has_relations {
            picker_width
                + if show_relation_label {
                    label_width + 12.0
                } else {
                    0.0
                }
        } else {
            0.0
        };
        let zoom_rect = egui::Rect::from_min_max(
            egui::pos2(left + 12.0, rect.top()),
            egui::pos2(
                (right - relation_width - 20.0).max(left + 40.0),
                rect.bottom(),
            ),
        );
        let mut zoom = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("footer-zoom")
                .max_rect(zoom_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        zoom.set_clip_rect(zoom_rect);
        self.view_controls(&mut zoom);
        let arrow_rect = egui::Rect::from_min_max(
            egui::pos2(right - relation_width - 12.0, rect.top()),
            egui::pos2(right - 12.0, rect.bottom()),
        );
        let mut arrow = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("footer-arrow-type")
                .max_rect(arrow_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        arrow.set_clip_rect(arrow_rect);
        arrow.spacing_mut().interact_size.y = 24.0;
        arrow.spacing_mut().button_padding.y = 2.0;
        if has_relations {
            self.arrow_type_control(&mut arrow, picker_width);
            if show_relation_label {
                arrow.label(crate::i18n::tr("Default Relation Type"));
            }
        }
        let info_rect = egui::Rect::from_min_max(
            egui::pos2(right + 8.0, rect.top()),
            rect.max - egui::vec2(14.0, 0.0),
        );
        let mut info = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("footer-info")
                .max_rect(info_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        info.set_clip_rect(info_rect);
        info.colored_label(
            appearance::color(ui.ctx(), appearance::MUTED),
            crate::i18n::tr("LAML · UTF-8"),
        );
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            let status = crate::browser_storage::storage_status();
            let color = if crate::browser_storage::storage_failed() {
                egui::Color32::LIGHT_RED
            } else {
                appearance::color(ui.ctx(), appearance::TEXT)
            };
            info.add(
                egui::Label::new(
                    egui::RichText::new(crate::i18n::message(&(&status))).color(color),
                )
                .truncate(),
            )
            .on_hover_text(crate::i18n::message(&(&status)));
        }
        if !laml_ui::services::browser_mode() && self.document.is_dirty() {
            info.label(crate::i18n::tr("Unsaved changes"));
        }
    }
}
