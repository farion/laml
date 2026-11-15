//! Additional typed diagram attributes augment the shared node/port inspectors.
use super::*;
impl Editor {
    pub(in crate::app) fn typed_node_attributes(&mut self, ui: &mut egui::Ui) {
        if self.notation() == Notation::Gne || self.selection.ids.len() != 1 {
            return;
        }
        let Some(node) = self
            .preview
            .as_ref()
            .and_then(|p| {
                p.diagram
                    .nodes
                    .iter()
                    .find(|n| n.id == self.selection.ids[0])
            })
            .cloned()
        else {
            return;
        };
        let fields: Vec<_> = edit::fields::of(self.document.source(), node.line)
            .into_iter()
            .filter(|f| {
                [
                    "Classifier",
                    "Interaction",
                    "Provides",
                    "Requires",
                    "Style-classes",
                ]
                .contains(&f.name.as_str())
            })
            .collect();
        if fields.is_empty() {
            return;
        }
        inspector_ui::category(ui, "Notation");
        self.typed_attributes(ui, node.line, fields);
    }
    pub(in crate::app) fn typed_port_attributes(&mut self, ui: &mut egui::Ui) {
        if self.selection.ports.len() != 1 {
            return;
        }
        let Some((node, port)) = self
            .preview
            .as_ref()
            .and_then(|p| laml_ui::port_edit::find(&p.diagram, &self.selection.ports[0]))
            .map(|(n, p)| (n.clone(), p.clone()))
        else {
            return;
        };
        if matches!(
            node.kind,
            laml::syntax::NodeKind::ComponentPart
                | laml::syntax::NodeKind::CompositePart
                | laml::syntax::NodeKind::CompositeRole
        ) {
            if let Some(classifier) = &node.part_type {
                ui.label(format!("Port declared by {classifier}"));
                if ui.button("Edit declaring classifier").clicked() {
                    self.selection.click(Some(classifier), false);
                    self.source_jump = self
                        .preview
                        .as_ref()
                        .and_then(|p| p.diagram.nodes.iter().find(|n| &n.id == classifier))
                        .map(|n| n.line);
                }
            }
            return;
        }
        let fields: Vec<_> = edit::fields::of(self.document.source(), port.line)
            .into_iter()
            .filter(|f| ["Provides", "Requires"].contains(&f.name.as_str()))
            .collect();
        if fields.is_empty() {
            return;
        }
        inspector_ui::category(ui, "Interface contract");
        self.typed_attributes(ui, port.line, fields);
    }
    fn typed_attributes(
        &mut self,
        ui: &mut egui::Ui,
        line: usize,
        fields: Vec<edit::fields::Field>,
    ) {
        for field in fields {
            let key = format!("typed-attribute-{line}-{}", field.name);
            let mut value = self
                .property_drafts
                .get(&key)
                .cloned()
                .unwrap_or_else(|| field.value.clone());
            let mut commit = false;
            ui.add_enabled_ui(self.can_edit_visually(), |ui| {
                inspector_ui::row(ui, &field.name, |ui| {
                    let f = inspector_ui::field(ui, &key, &mut value, false);
                    self.source_ui.extra_drafts.push(f.response.id);
                    commit = f.commit;
                });
            });
            self.property_drafts.insert(key, value.clone());
            if commit && value != field.value {
                match edit::fields::edit(self.document.source(), &field, &value) {
                    Ok(edits) => self.submit_command(Command {
                        edits,
                        label: "Change notation attribute".into(),
                        expectation: worker::Expectation::Notation {
                            line: None,
                            id: None,
                        },
                    }),
                    Err(e) => self.command_error = Some(e),
                }
            }
        }
    }
}
