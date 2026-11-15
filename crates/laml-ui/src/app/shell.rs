//! Three-pane editor chrome based on the supplied 1568 × 975 reference.
use super::*;

impl Editor {
    pub(super) fn shell(&mut self, ui: &mut egui::Ui) {
        let frame = appearance::panel_frame(appearance::color(ui.ctx(), appearance::PANEL));
        let footer = egui::Panel::bottom("document-status")
            .exact_size(40.0)
            // We paint the border ourselves; the panel's separator would consume
            // one pixel of the 24-point button row and clip its lower edge.
            .show_separator_line(false)
            .frame(
                appearance::panel_frame(appearance::color(ui.ctx(), appearance::HEADER))
                    .inner_margin(0),
            )
            .show(ui, |ui| ui.max_rect())
            .inner;
        let width = ui.available_width();
        if (width - self.shell_width).abs() > 1.0 {
            // The native window manager can change the initial viewport after its
            // first frame. Reapply the reference proportions at the actual size.
            for salt in ["source-pane", "properties-pane"] {
                let id = ui.make_persistent_id(salt);
                ui.ctx()
                    .data_mut(|data| data.remove::<egui::containers::panel::PanelState>(id));
            }
            self.shell_width = width;
        }
        egui::Panel::left("source-pane")
            .resizable(true)
            .default_size((width * 0.25).round())
            .min_size((width * 0.25).min(280.0))
            .frame(appearance::panel_frame(appearance::color(
                ui.ctx(),
                appearance::BACKGROUND,
            )))
            .show(ui, |ui| {
                self.source_heading(ui);
                egui::Frame::new()
                    .fill(appearance::color(ui.ctx(), appearance::BACKGROUND))
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| self.source_editor(ui));
            });
        egui::Panel::right("properties-pane")
            .resizable(true)
            .default_size((width * 0.20).round())
            .min_size((width * 0.20).min(270.0))
            .frame(frame)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                appearance::pane_heading(ui, "Properties");
                self.inspector_tabs(ui);
                self.inspector(ui);
            });
        egui::CentralPanel::default()
            .frame(appearance::panel_frame(appearance::color(
                ui.ctx(),
                appearance::BACKGROUND,
            )))
            .show(ui, |ui| {
                let header = self.graph_heading(ui);
                let diagram_area = egui::Rect::from_min_max(
                    egui::pos2(ui.max_rect().left(), header.bottom()),
                    ui.max_rect().max,
                );
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 12,
                        bottom: 12,
                    })
                    .show(ui, |ui| {
                        self.canvas(ui);
                    });
                self.preview_loading(ui, diagram_area);
            });
        let source = egui::containers::panel::PanelState::load(
            ui.ctx(),
            ui.make_persistent_id("source-pane"),
        )
        .unwrap()
        .outer_rect;
        let properties = egui::containers::panel::PanelState::load(
            ui.ctx(),
            ui.make_persistent_id("properties-pane"),
        )
        .unwrap()
        .outer_rect;
        self.footer(ui, footer, source.right(), properties.left());
    }

    fn footer(&mut self, ui: &mut egui::Ui, rect: egui::Rect, left: f32, right: f32) {
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
                    "Default Relation Type".into(),
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
                arrow.label("Default Relation Type");
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
            "LAML · UTF-8",
        );
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            let status = crate::browser_storage::storage_status();
            let color = if crate::browser_storage::storage_failed() {
                egui::Color32::LIGHT_RED
            } else {
                appearance::color(ui.ctx(), appearance::TEXT)
            };
            info.add(egui::Label::new(egui::RichText::new(&status).color(color)).truncate())
                .on_hover_text(&status);
        }
        if !laml_ui::services::browser_mode() && self.document.is_dirty() {
            info.label("Unsaved changes");
        }
    }

    fn inspector_tabs(&mut self, ui: &mut egui::Ui) {
        let (bar, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar, 0, appearance::color(ui.ctx(), appearance::HEADER));
        for (index, (tab, title)) in [
            (inspector_ui::Tab::General, "Allgemein"),
            (inspector_ui::Tab::Style, "Style"),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = egui::Rect::from_min_size(
                bar.min + egui::vec2(bar.width() / 2.0 * index as f32, 0.0),
                egui::vec2(bar.width() / 2.0, bar.height()),
            );
            let response = ui.interact(
                rect,
                ui.make_persistent_id(("inspector-tab", index)),
                egui::Sense::click(),
            );
            let active = self.inspector_tab == tab;
            if active {
                ui.painter()
                    .rect_filled(rect, 0, appearance::color(ui.ctx(), appearance::PANEL));
            } else if response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    0,
                    appearance::color(ui.ctx(), appearance::BACKGROUND),
                );
            }
            ui.painter().text(
                rect.left_center() + egui::vec2(16.0, 0.0),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(15.0),
                if active {
                    appearance::color(ui.ctx(), appearance::TEXT)
                } else {
                    appearance::color(ui.ctx(), appearance::MUTED)
                },
            );
            if active {
                ui.painter().hline(
                    rect.x_range(),
                    rect.top(),
                    egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
                );
                for x in [rect.left(), rect.right()] {
                    ui.painter().vline(
                        x,
                        rect.y_range(),
                        egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
                    );
                }
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    egui::Stroke::new(2.0, appearance::color(ui.ctx(), appearance::ACCENT)),
                );
            }
            if response.clicked() {
                self.inspector_next_tab = Some(tab);
            }
        }
    }

    #[cfg(test)]
    fn canvas_tools(&mut self, ui: &mut egui::Ui, viewport: egui::Rect) -> egui::Rect {
        let mut tools = ui.new_child(egui::UiBuilder::new().id_salt("canvas-toolbars").max_rect(
            egui::Rect::from_min_size(
                viewport.min + egui::vec2(12.0, 10.0),
                egui::vec2(viewport.width() - 24.0, 136.0),
            ),
        ));
        tools.set_clip_rect(viewport);
        appearance::floating_frame(ui.ctx()).show(&mut tools, |ui| self.layout_tools(ui));
        tools.min_rect()
    }

    #[cfg(test)]
    fn layout_tools(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let button_width = ((ui.available_width() - 70.0) / 2.0).clamp(80.0, 120.0);
            let enabled = self.can_edit_visually()
                && self.selection.ids.len() >= 2
                && self.selection.relations.is_empty()
                && self.selection.ports.is_empty();
            if appearance::alignment_button(ui, enabled, true, button_width).clicked() {
                self.propose_alignment(laml::syntax::FreeAlignment::CenterY);
            }
            if appearance::alignment_button(ui, enabled, false, button_width).clicked() {
                self.propose_alignment(laml::syntax::FreeAlignment::CenterX);
            }
            ui.menu_button("···", |ui| {
                ui.set_max_width(400.0);
                ui.label("Layout & editing");
                self.alignment_toolbar(ui);
                ui.separator();
                self.creation_toolbar(ui);
            });
        });
    }
}

#[cfg(test)]
mod tests;
