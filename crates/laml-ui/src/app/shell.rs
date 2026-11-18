//! Three-pane editor chrome based on the supplied 1568 × 975 reference.
use super::*;
mod compact_footer;
mod footer;

impl Editor {
    pub(super) fn shell(&mut self, ui: &mut egui::Ui) {
        self.responsive.layout(ui.available_width());
        let content_top = ui.available_rect_before_wrap().top();
        let footer = egui::Panel::bottom("document-status")
            .exact_size(40.0)
            .show_separator_line(false)
            .frame(
                appearance::panel_frame(appearance::color(ui.ctx(), appearance::HEADER))
                    .inner_margin(0),
            )
            .show(ui, |ui| ui.max_rect())
            .inner;
        let width = ui.available_width();
        if (width - self.shell_width).abs() > 1.0 {
            for salt in ["source-pane", "properties-pane"] {
                let id = ui.make_persistent_id(salt);
                ui.ctx()
                    .data_mut(|data| data.remove::<egui::containers::panel::PanelState>(id));
            }
            self.shell_width = width;
        }
        let mut left = ui.max_rect().left();
        let mut right = ui.max_rect().right();
        if !self.responsive.single {
            let panel = egui::Panel::left("source-pane")
                .resizable(true)
                .default_size((width * 0.25).round())
                .min_size((width * 0.25).min(280.0))
                .max_size(
                    (width
                        - if self.responsive.docked && !self.responsive.properties_hidden {
                            600.0
                        } else {
                            360.0
                        })
                    .max(240.0),
                )
                .frame(appearance::panel_frame(appearance::color(
                    ui.ctx(),
                    appearance::BACKGROUND,
                )))
                .show(ui, |ui| self.source_pane(ui));
            left = panel.response.rect.right();
        }
        if self.responsive.docked && !self.responsive.properties_hidden {
            let panel = egui::Panel::right("properties-pane")
                .resizable(true)
                .default_size((width * 0.20).round())
                .min_size((width * 0.20).min(270.0))
                .max_size((ui.available_width() - 340.0).max(240.0))
                .frame(appearance::panel_frame(appearance::color(
                    ui.ctx(),
                    appearance::PANEL,
                )))
                .show(ui, |ui| self.properties_pane(ui, false));
            right = panel.response.rect.left();
        }
        egui::CentralPanel::default()
            .frame(appearance::panel_frame(appearance::color(
                ui.ctx(),
                appearance::BACKGROUND,
            )))
            .show(ui, |ui| {
                if self.responsive.single && self.responsive.pane == responsive::Pane::Source {
                    self.source_pane(ui);
                } else {
                    let header = self.graph_heading(ui);
                    let area = egui::Rect::from_min_max(
                        egui::pos2(ui.max_rect().left(), header.bottom()),
                        ui.max_rect().max,
                    );
                    egui::Frame::new()
                        .inner_margin(12)
                        .show(ui, |ui| self.canvas(ui));
                    self.preview_loading(ui, area);
                }
            });
        self.footer(ui, footer, left, right);
        if !self.responsive.docked
            && self.responsive.flyout
            && (!self.responsive.single || self.responsive.pane == responsive::Pane::Diagram)
        {
            self.properties_flyout(ui, content_top, footer.top());
        }
    }

    fn source_pane(&mut self, ui: &mut egui::Ui) {
        self.source_heading(ui);
        egui::Frame::new()
            .fill(appearance::color(ui.ctx(), appearance::BACKGROUND))
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| self.source_editor(ui));
    }

    fn properties_pane(&mut self, ui: &mut egui::Ui, flyout: bool) {
        ui.spacing_mut().item_spacing.y = 0.0;
        if flyout {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(16, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::tr("Properties"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if workspace::icon(ui, egui_phosphor::regular::X, "Close").clicked() {
                                self.responsive.flyout = false;
                            }
                        });
                    });
                });
        } else {
            appearance::pane_heading(ui, "Properties");
        }
        self.inspector_tabs(ui);
        self.inspector(ui);
    }

    fn properties_flyout(&mut self, ui: &mut egui::Ui, top: f32, bottom: f32) {
        let screen = ui.ctx().content_rect();
        let width = 340.0_f32.min(screen.width() - 16.0).max(1.0);
        let rect = egui::Rect::from_min_max(
            egui::pos2(screen.right() - width, top.min(bottom)),
            egui::pos2(screen.right(), bottom),
        );
        egui::Area::new(ui.make_persistent_id("properties-flyout"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_width(width);
                ui.set_height(rect.height());
                ui.set_clip_rect(rect);
                appearance::panel_frame(appearance::color(ui.ctx(), appearance::PANEL))
                    .stroke(egui::Stroke::new(
                        1.0,
                        appearance::color(ui.ctx(), appearance::BORDER),
                    ))
                    .show(ui, |ui| {
                        ui.set_min_size(rect.size() - egui::vec2(2.0, 2.0));
                        self.properties_pane(ui, true);
                    });
            });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.responsive.flyout = false;
        }
    }

    fn inspector_tabs(&mut self, ui: &mut egui::Ui) {
        let (bar, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar, 0, appearance::color(ui.ctx(), appearance::HEADER));
        for (index, (tab, title)) in [
            (inspector_ui::Tab::General, "General"),
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
                crate::i18n::tr(title),
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
                ui.label(crate::i18n::tr("Layout & editing"));
                self.alignment_toolbar(ui);
                ui.separator();
                self.creation_toolbar(ui);
            });
        });
    }
}

#[cfg(test)]
mod tests;
