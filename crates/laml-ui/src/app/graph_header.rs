use super::*;
use egui_phosphor::regular::*;
use laml::syntax::FreeAlignment;

pub(super) const ALIGNMENTS: [(&str, &str, FreeAlignment); 6] = [
    ("Align left", ALIGN_LEFT_SIMPLE, FreeAlignment::Left),
    (
        "Center horizontally",
        ALIGN_CENTER_HORIZONTAL_SIMPLE,
        FreeAlignment::CenterX,
    ),
    ("Align right", ALIGN_RIGHT_SIMPLE, FreeAlignment::Right),
    ("Align top", ALIGN_TOP_SIMPLE, FreeAlignment::Top),
    (
        "Center vertically",
        ALIGN_CENTER_VERTICAL_SIMPLE,
        FreeAlignment::CenterY,
    ),
    ("Align bottom", ALIGN_BOTTOM_SIMPLE, FreeAlignment::Bottom),
];

impl Editor {
    pub(super) fn graph_heading(&mut self, ui: &mut egui::Ui) -> egui::Rect {
        let header = egui::Frame::new()
            .fill(appearance::color(ui.ctx(), appearance::HEADER))
            .inner_margin(egui::Margin::symmetric(14, 6))
            .show(ui, |ui| {
                ui.set_min_height(appearance::HEADER_HEIGHT - 12.0);
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    self.pane_title(ui, responsive::Pane::Diagram);
                    if ui.available_width() > 430.0 {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(crate::i18n::tr(self.notation().title()))
                                    .color(appearance::color(ui.ctx(), appearance::MUTED)),
                            )
                            .truncate(),
                        );
                    }
                    let mut items = vec![toolbar::Item {
                        label: "Add node",
                        icon: Some(PLUS),
                        enabled: self.can_edit_visually()
                            && !self.graph.dialog_open()
                            && self.graph.gesture.is_none(),
                        separator_before: false,
                    }];
                    if self.notation() != laml_ui::notation::Notation::Gne {
                        items.push(toolbar::Item {
                            label: "Elements…",
                            icon: None,
                            enabled: true,
                            separator_before: false,
                        });
                        if self.notation().graph() {
                            items.push(toolbar::Item {
                                label: "Add construct…",
                                icon: None,
                                enabled: items[0].enabled,
                                separator_before: false,
                            });
                        }
                    }
                    let sequence = self.notation() == laml_ui::notation::Notation::Sequence;
                    if sequence {
                        items.remove(0);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.properties_button(ui);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            toolbar::show_right(ui, &items, |ui, index, in_menu| {
                                if toolbar::button(ui, &items[index], in_menu).clicked() {
                                    match if sequence { index + 1 } else { index } {
                                        0 => self.open_node_dialog(),
                                        1 => self.notation_ui.navigator = true,
                                        _ => self.open_construct_dialog(),
                                    }
                                    if in_menu {
                                        ui.close();
                                    }
                                }
                            });
                        });
                    });
                });
            });
        ui.painter().hline(
            header.response.rect.x_range(),
            header.response.rect.bottom(),
            egui::Stroke::new(1.0, appearance::color(ui.ctx(), appearance::BORDER)),
        );
        header.response.rect
    }

    pub(super) fn alignment_overlay(&mut self, ui: &egui::Ui, viewport: egui::Rect) -> bool {
        self.graph.hover_align = None;
        if self.specialized()
            || self.selection.ids.len() < 2
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            return false;
        }
        let enabled =
            self.can_edit_visually() && !self.graph.dialog_open() && self.graph.gesture.is_none();
        let mut clicked = None;
        let overlay = egui::Area::new(ui.make_persistent_id("alignment-overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(viewport.center_top() + egui::vec2(0.0, 10.0))
            .pivot(egui::Align2::CENTER_TOP)
            .movable(false)
            .show(ui.ctx(), |ui| {
                appearance::floating_frame(ui.ctx()).show(ui, |ui| {
                    ui.spacing_mut().interact_size = egui::Vec2::splat(24.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for (label, icon, align) in ALIGNMENTS {
                            let item = toolbar::Item {
                                label,
                                icon: Some(icon),
                                enabled,
                                separator_before: false,
                            };
                            let response = toolbar::button(ui, &item, false);
                            if response.hovered() {
                                self.graph.hover_align = Some(align);
                            }
                            if response.clicked() {
                                clicked = Some(align);
                            }
                        }
                    });
                });
            });
        if let Some(align) = clicked {
            self.align_selected(align);
        }
        ui.input(|input| input.pointer.hover_pos())
            .is_some_and(|point| overlay.response.rect.contains(point))
    }

    pub(super) fn align_selected(&mut self, align: FreeAlignment) {
        if self.specialized()
            || self.selection.ids.len() < 2
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            return;
        }
        self.graph.report_errors = true;
        self.propose_alignment(align);
        if self.alignment.is_some() {
            self.apply_alignment();
            self.graph.flash =
                (self.pending_command == Some(self.document.revision())).then(|| {
                    alignment_guides::Flash {
                        align,
                        ids: self.selection.ids.clone(),
                        revision: self.document.revision(),
                        started: None,
                    }
                });
        }
    }
}
