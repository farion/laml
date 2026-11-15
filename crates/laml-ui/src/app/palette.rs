use super::*;
use laml::syntax::{NodeKind, Placement};
use laml_ui::element_edit::{self, ElementKind};

#[derive(Default)]
pub(super) struct Palette {
    pub kind: ElementKind,
    pub parent: String,
    pub relative: usize,
    pub port_name: String,
}
impl Editor {
    #[cfg(test)]
    pub(super) fn palette_controls(&mut self, ui: &mut egui::Ui) {
        let packages: Vec<_> = self
            .preview
            .as_ref()
            .into_iter()
            .flat_map(|p| &p.diagram.nodes)
            .filter(|n| n.kind == NodeKind::Package)
            .map(|n| n.id.clone())
            .collect();
        egui::ComboBox::from_id_salt("new-element-kind")
            .selected_text(self.creation.palette.kind.title())
            .show_ui(ui, |ui| {
                for kind in ElementKind::ALL {
                    ui.selectable_value(&mut self.creation.palette.kind, kind, kind.title());
                }
            });
        egui::ComboBox::from_id_salt("new-element-container")
            .selected_text(if self.creation.palette.parent.is_empty() {
                "Root"
            } else {
                &self.creation.palette.parent
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.creation.palette.parent, String::new(), "Root");
                for parent in packages {
                    ui.selectable_value(&mut self.creation.palette.parent, parent.clone(), &parent);
                }
            });
        let modes = [
            "Automatic",
            "Left of selection",
            "Right of selection",
            "Above selection",
            "Below selection",
        ];
        egui::ComboBox::from_id_salt("new-element-placement")
            .selected_text(modes[self.creation.palette.relative])
            .show_ui(ui, |ui| {
                for (i, title) in modes.iter().enumerate() {
                    ui.selectable_value(&mut self.creation.palette.relative, i, *title);
                }
            });
        if ui.button("Add element").clicked() {
            self.create_element();
        }
    }
    #[cfg(test)]
    pub(super) fn port_creation_controls(&mut self, ui: &mut egui::Ui) {
        let owner = self
            .selection
            .ids
            .first()
            .filter(|_| {
                self.selection.ids.len() == 1
                    && self.selection.ports.is_empty()
                    && self.selection.relations.is_empty()
            })
            .cloned();
        let applicable = owner.as_ref().is_some_and(|id| {
            self.preview.as_ref().is_some_and(|p| {
                p.diagram
                    .nodes
                    .iter()
                    .any(|n| &n.id == id && matches!(n.kind, NodeKind::Generic | NodeKind::Class))
            })
        });
        ui.add_enabled_ui(applicable, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.creation.palette.port_name)
                    .hint_text("Port name (identifier)")
                    .desired_width(160.0),
            );
            if ui.button("Add port to selected node").clicked()
                && let Some(owner) = owner
            {
                let name = self.creation.palette.port_name.clone();
                self.create_port(&owner, &name);
            }
        })
        .response
        .on_disabled_hover_text("Select one generic node or class to add a boundary port.");
    }
    pub(super) fn create_element(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        if self.creation.palette.kind == ElementKind::Rectangle
            && self.creation.palette.parent.is_empty()
            && self.creation.palette.relative == 0
        {
            let label = self.creation.label.clone();
            self.create_node(&label);
            return;
        }
        let palette = &self.creation.palette;
        let parent = (!palette.parent.is_empty()).then_some(palette.parent.as_str());
        let placement = match palette.relative {
            1 => Some(Placement::Left),
            2 => Some(Placement::Right),
            3 => Some(Placement::Above),
            4 => Some(Placement::Below),
            _ => None,
        };
        let reference = self.selection.ids.last();
        if placement.is_some()
            && (reference.is_none()
                || !self.selection.ports.is_empty()
                || !self.selection.relations.is_empty())
        {
            self.command_error = Some("Select a node as the relative insertion reference.".into());
            return;
        }
        match element_edit::add(
            self.document.source(),
            palette.kind,
            &self.creation.label,
            parent,
            placement.zip(reference).map(|(p, id)| (id.as_str(), p)),
        ) {
            Ok((id, edits)) => self.submit_command(Command {
                edits,
                label: "Add GNE element".into(),
                expectation: worker::Expectation::Element {
                    id,
                    parent: parent.map(str::to_string),
                    kind: palette.kind,
                },
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn create_port(&mut self, owner: &str, name: &str) {
        if !self.can_edit_visually() {
            return;
        }
        match laml_ui::port_edit::add(self.document.source(), owner, name) {
            Ok((id, edits)) => self.submit_command(Command {
                edits,
                label: "Add port".into(),
                expectation: worker::Expectation::Port(id),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
}
