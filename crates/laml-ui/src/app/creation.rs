use super::*;
use laml_ui::semantic_edit::{self, Connection};

#[derive(Default)]
pub(super) struct Creation {
    pub connect: bool,
    pub kind: Connection,
    pub drag: Option<(String, u64)>,
    #[cfg(test)]
    pub label: String,
    #[cfg(test)]
    pub palette: palette::Palette,
    #[cfg(test)]
    pub canvas_rect: Option<egui::Rect>,
}

impl Editor {
    pub(super) fn can_edit_visually(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() && !crate::browser_storage::available() {
            return false;
        }
        self.error.is_none()
            && self.rendered == Some(self.document.revision())
            && self.pending_command != Some(self.document.revision())
    }
    #[cfg(test)]
    pub(super) fn creation_toolbar(&mut self, ui: &mut egui::Ui) {
        let enabled = self.can_edit_visually();
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("New node:");
                ui.add_enabled(
                    self.creation.palette.kind != laml_ui::element_edit::ElementKind::Junction,
                    egui::TextEdit::singleline(&mut self.creation.label)
                        .hint_text("Label (optional)")
                        .desired_width(160.0),
                );
                self.palette_controls(ui);
                ui.separator();
                if ui
                    .selectable_label(!self.creation.connect, "Select / arrange")
                    .clicked()
                {
                    self.creation.connect = false;
                    self.creation.drag = None;
                }
                if ui
                    .selectable_label(self.creation.connect, "Connect")
                    .clicked()
                {
                    self.creation.connect = true;
                    self.placement.cancel();
                }
                egui::ComboBox::from_id_salt("connection-type")
                    .selected_text(self.creation.kind.operator())
                    .show_ui(ui, |ui| {
                        for kind in [
                            Connection::Directed,
                            Connection::Bidirectional,
                            Connection::Undirected,
                        ] {
                            ui.selectable_value(&mut self.creation.kind, kind, kind.operator());
                        }
                    });
            });
        });
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| self.port_creation_controls(ui));
        });
        if self.creation.connect {
            ui.small(
                "Drag from one node to another. Connection sides remain automatic; Escape cancels.",
            );
        } else {
            ui.small("Drag a node onto a side of another node. Wait for a green zone before releasing; Escape cancels.");
        }
    }
    #[cfg(test)]
    pub(super) fn create_node(&mut self, label: &str) {
        if !self.can_edit_visually() {
            return;
        }
        match laml_ui::element_edit::add(
            self.document.source(),
            laml_ui::element_edit::ElementKind::Rectangle,
            label,
            None,
            None,
        ) {
            Ok((id, edits)) => self.submit_command(Command {
                edits,
                label: "Add node".into(),
                expectation: worker::Expectation::Element {
                    id,
                    parent: None,
                    kind: laml_ui::element_edit::ElementKind::Rectangle,
                },
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn create_relation(&mut self, from: &str, to: &str) {
        if !self.can_edit_visually() {
            return;
        }
        match semantic_edit::add_relation(self.document.source(), from, to, self.creation.kind) {
            Ok((id, edits)) => self.submit_command(Command {
                edits,
                label: "Add relation".into(),
                expectation: worker::Expectation::Relation(id),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn submit_command(&mut self, mut command: Command) {
        if !self.can_edit_visually() {
            return;
        }
        let revision = self.document.revision();
        let mut candidate = self.document.clone();
        match candidate.apply(revision, command.label.clone(), command.edits.clone()) {
            Ok(false) => return,
            Err(error) => {
                self.command_error = Some(format!("Edit failed: {error:?}"));
                return;
            }
            Ok(true) => {}
        }
        match &mut command.expectation {
            worker::Expectation::RelationEdit(lines)
            | worker::Expectation::RelationStyle { lines, .. } => {
                for line in lines {
                    *line = laml_ui::relation_edit::remap_line(
                        self.document.source(),
                        *line,
                        &command.edits,
                    );
                }
            }
            _ => {}
        }
        match self.compiler.send((
            revision,
            candidate.source().into(),
            self.path.clone(),
            Some(command),
        )) {
            Ok(()) => {
                self.pending_command = Some(revision);
                self.command_error = None;
            }
            Err(error) => self.command_error = Some(error.to_string()),
        }
    }
}
