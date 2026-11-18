//! Dialogs do not introduce a second connection mode.
use super::*;
impl Editor {
    pub(in crate::app) fn sequence_insertion_menu_open(&self) -> bool {
        self.notation_ui.sequence.insert.is_some()
    }
    /// Same menu styling/placement as other diagram plus controls, not a dialog.
    pub(super) fn sequence_insert_menu(&mut self, ui: &mut egui::Ui, scene: &Scene) {
        let Some(point) = self.notation_ui.sequence.insert.take() else {
            return;
        };
        let anchor = scene
            .areas
            .iter()
            .find_map(|area| match &area.hit {
                Hit::Insert(p) if p.dock == point.dock => Some(area.rect),
                _ => None,
            })
            .unwrap_or_else(|| {
                egui::Rect::from_center_size(
                    point.at + egui::vec2(22.0, 0.0),
                    egui::Vec2::splat(20.0),
                )
            });
        painting::icon(ui, anchor.center(), egui_phosphor::regular::PLUS, true);
        let mut open = true;
        egui::Popup::new(
            ui.make_persistent_id(("sequence-add", self.id)),
            ui.ctx().clone(),
            anchor,
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .style(egui::containers::menu::menu_style)
        .gap(0.0)
        .open_bool(&mut open)
        .show(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            self.sequence_add_menu(ui, &point);
        });
        if !open {
            self.notation_ui.sequence.expanded = false;
        }
        if open && self.notation_ui.sequence.picker.is_none() && self.pending_command.is_none() {
            self.notation_ui.sequence.insert = Some(point);
        }
    }
    pub(in crate::app) fn sequence_window(&mut self, ctx: &egui::Context) {
        if let Some(options) = self.notation_ui.sequence.alternatives.take() {
            let mut selected = None;
            let dismissed = dialogs::show(
                ctx,
                ("sequence-message-type", self.id),
                "Choose relationship",
                380.0,
                |ui| {
                    ui.label(crate::i18n::tr(
                        "The default cannot connect these endpoints with the current properties.",
                    ));
                    for (index, (key, _)) in options.iter().enumerate() {
                        let title = laml_ui::notation::Notation::Sequence
                            .relation_types()
                            .into_iter()
                            .find(|r| r.key == key)
                            .map_or(key.as_str(), |r| r.title);
                        if actions::menu_button(ui, title) {
                            selected = Some(index);
                        }
                    }
                },
            );
            if let Some(i) = selected {
                self.sequence_apply(options[i].1.clone());
            } else if !dismissed {
                self.notation_ui.sequence.alternatives = Some(options);
            }
        }
    }
}
