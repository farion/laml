use super::*;
use laml_ui::node_edit;

impl Editor {
    pub(super) fn object_fields(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids.is_empty()
            || (!self.selection.relations.is_empty() || !self.selection.ports.is_empty())
        {
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let selected: Vec<_> = self
            .selection
            .ids
            .iter()
            .filter_map(|id| preview.diagram.nodes.iter().find(|n| &n.id == id))
            .collect();
        let Some(first) = selected.first() else {
            return;
        };
        let value = first.label.clone().unwrap_or_else(|| first.id.clone());
        let mixed = selected
            .iter()
            .any(|n| n.label.as_deref().unwrap_or(&n.id) != value);
        let label_editable = selected.iter().all(|n| {
            !matches!(
                n.kind,
                laml::syntax::NodeKind::Object | laml::syntax::NodeKind::Junction
            ) && n.templates.is_empty()
        });
        let id = first.id.clone();
        let count = selected.len();
        inspector_ui::category(ui, "Identity");
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            ui.add_enabled_ui(label_editable, |ui| {
                inspector_ui::row(ui, "Label", |ui| {
                    let draft = self
                        .property_drafts
                        .entry("node-label".into())
                        .or_insert_with(|| if mixed { String::new() } else { value.clone() });
                    let field = inspector_ui::mixed_field(ui, "node-label", draft, true, mixed);
                    self.source_ui.extra_drafts.push(field.response.id);
                    let label = draft.clone();
                    if field.commit && (mixed || label != value) {
                        self.change_node_labels(&label);
                    }
                });
            });
            ui.add_enabled_ui(count == 1, |ui| {
                inspector_ui::row(ui, "ID", |ui| {
                    let draft = self
                        .property_drafts
                        .entry("node-id".into())
                        .or_insert_with(|| if count > 1 { String::new() } else { id.clone() });
                    if count > 1 { draft.clear(); }
                    let field = inspector_ui::mixed_field(ui, "node-id", draft, false, count > 1);
                    self.source_ui.extra_drafts.push(field.response.id);
                    let value = draft.clone();
                    if field.commit && value != id {
                        self.rename_node(&id, &value);
                    }
                });
            });
        });
    }
    pub(super) fn change_node_labels(&mut self, value: &str) {
        if !self.can_edit_visually()
            || (!self.selection.relations.is_empty() || !self.selection.ports.is_empty())
        {
            return;
        }
        let mut edits = Vec::new();
        for id in &self.selection.ids {
            match node_edit::label(self.document.source(), id, value) {
                Ok(next) => edits.extend(next),
                Err(error) => {
                    self.command_error = Some(error);
                    return;
                }
            }
        }
        self.submit_command(Command {
            edits,
            label: "Change node labels".into(),
            expectation: worker::Expectation::Nodes(self.selection.ids.clone()),
        });
    }
    pub(super) fn rename_node(&mut self, id: &str, value: &str) {
        if !self.can_edit_visually() {
            return;
        }
        match node_edit::rename_id(self.document.source(), id, value) {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: "Rename node ID".into(),
                expectation: worker::Expectation::Node(value.into()),
            }),
            Err(error) => self.command_error = Some(error),
        }
    }
}
