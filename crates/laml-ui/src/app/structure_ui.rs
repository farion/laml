use super::*;
use laml_ui::{document::Edit, structure};
#[derive(Default)]
pub(super) struct StructureUi {
    owner: String,
    original: Option<(String, Option<String>)>,
    body: String,
    pub proposal: Option<Proposal>,
    from: String,
    to: String,
    note: String,
    relation: Option<usize>,
}
pub(super) struct Proposal {
    revision: u64,
    selection: (Vec<String>, Vec<String>, Vec<usize>),
    edits: Vec<Edit>,
    owner: Option<String>,
    label: String,
}
impl Editor {
    pub(super) fn structured_controls(&mut self, ui: &mut egui::Ui) {
        let enabled = self.can_edit_visually();
        let node = self.preview.as_ref().and_then(|p| {
            if self.selection.ids.len() == 1
                && self.selection.ports.is_empty()
                && self.selection.relations.is_empty()
            {
                p.diagram
                    .nodes
                    .iter()
                    .find(|n| n.id == self.selection.ids[0])
                    .cloned()
            } else {
                None
            }
        });
        if let Some(node) = node
            && let Ok(content) = structure::content(self.document.source(), &node.id)
        {
            let current = (content.header.clone(), content.body.clone());
            if self.structure.owner != node.id || self.structure.original.as_ref() != Some(&current)
            {
                self.structure.owner = node.id.clone();
                self.structure.body = content.body.clone().unwrap_or_default();
                self.structure.original = Some(current);
            }
            ui.add_enabled_ui(enabled, |ui| {
                if content.body.is_some() {
                    inspector_ui::category(ui, "Members / compartments");
                    let field = inspector_ui::row(ui, "Body", |ui| {
                        inspector_ui::field(ui, "structured-body", &mut self.structure.body, true)
                    });
                    self.source_ui.extra_drafts.push(field.response.id);
                    if field.commit && content.body.as_deref() != Some(self.structure.body.as_str())
                    {
                        self.submit_structure(
                            "Edit node body",
                            Some(node.id.clone()),
                            structure::body(self.document.source(), &node.id, &self.structure.body),
                        );
                    }
                    inspector_ui::row(ui, "Insert", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (name, text) in [
                                ("Field", "    field +value: String"),
                                ("Method", "    method +run()"),
                                ("Text", "    text \"Text\""),
                                ("Slot", "    value = 0"),
                                ("Literal", "    VALUE"),
                                ("Separator", "    --"),
                            ] {
                                if ui
                                    .add_enabled(
                                        match node.kind {
                                            laml::syntax::NodeKind::Object => {
                                                matches!(name, "Slot" | "Text" | "Separator")
                                            }
                                            laml::syntax::NodeKind::Enumeration => {
                                                matches!(name, "Literal" | "Text" | "Separator")
                                            }
                                            _ => !matches!(name, "Slot" | "Literal"),
                                        },
                                        egui::Button::new(name),
                                    )
                                    .clicked()
                                {
                                    if !self.structure.body.ends_with('\n')
                                        && !self.structure.body.is_empty()
                                    {
                                        self.structure.body.push('\n');
                                    }
                                    self.structure.body.push_str(text);
                                    self.submit_structure(
                                        "Add member",
                                        Some(node.id.clone()),
                                        structure::body(
                                            self.document.source(),
                                            &node.id,
                                            &self.structure.body,
                                        ),
                                    );
                                }
                            }
                        });
                    });
                }
            });
        }
    }
    pub(super) fn address_controls(&mut self, ui: &mut egui::Ui) {
        let Some(diagram) = self.preview.as_ref().map(|p| p.diagram.clone()) else {
            return;
        };
        let endpoints = structure::endpoints(&diagram);
        for (name, draft) in [
            ("From", &mut self.structure.from),
            ("To", &mut self.structure.to),
        ] {
            egui::ComboBox::from_id_salt(name)
                .selected_text(if draft.is_empty() {
                    name
                } else {
                    draft.as_str()
                })
                .show_ui(ui, |ui| {
                    for id in &endpoints {
                        ui.selectable_value(draft, id.clone(), id);
                    }
                });
        }
        if ui.button("Connect these addresses").clicked() {
            let a = self.structure.from.clone();
            let b = self.structure.to.clone();
            self.create_relation(&a, &b);
        }
        egui::ComboBox::from_id_salt("attachment-note")
            .selected_text(if self.structure.note.is_empty() {
                "Note"
            } else {
                &self.structure.note
            })
            .show_ui(ui, |ui| {
                for n in diagram
                    .nodes
                    .iter()
                    .filter(|n| n.kind == laml::syntax::NodeKind::Note)
                {
                    ui.selectable_value(&mut self.structure.note, n.id.clone(), &n.id);
                }
            });
        egui::ComboBox::from_id_salt("attachment-relation")
            .selected_text(
                self.structure
                    .relation
                    .map_or_else(|| "Relation".into(), |l| format!("Line {l}")),
            )
            .show_ui(ui, |ui| {
                for r in &diagram.relations {
                    ui.selectable_value(
                        &mut self.structure.relation,
                        Some(r.line),
                        format!("{} → {} ({})", r.from, r.to, r.line),
                    );
                }
            });
        if ui.button("Attach note").clicked()
            && let Some(line) = self.structure.relation
        {
            let result = structure::attach(self.document.source(), &self.structure.note, line);
            self.submit_structure("Attach note", None, result);
        }
        for a in &diagram.attachments {
            if ui
                .button(format!("Remove {} — relation[{}]…", a.source, a.target))
                .clicked()
            {
                self.propose_structure(
                    "Remove note attachment",
                    None,
                    structure::remove_attachment(self.document.source(), a.line),
                );
            }
        }
    }
    pub(super) fn submit_structure(
        &mut self,
        label: &str,
        owner: Option<String>,
        edits: Result<Vec<Edit>, String>,
    ) {
        match edits {
            Ok(edits) => self.submit_command(Command {
                edits,
                label: label.into(),
                expectation: worker::Expectation::Nodes(owner.into_iter().collect()),
            }),
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn propose_structure(
        &mut self,
        label: &str,
        owner: Option<String>,
        edits: Result<Vec<Edit>, String>,
    ) {
        match edits {
            Ok(edits) => {
                self.structure.proposal = Some(Proposal {
                    revision: self.document.revision(),
                    selection: (
                        self.selection.ids.clone(),
                        self.selection.ports.clone(),
                        self.selection.relations.clone(),
                    ),
                    edits,
                    owner,
                    label: label.into(),
                })
            }
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn structure_window(&mut self, ctx: &egui::Context) {
        let Some(p) = &self.structure.proposal else {
            return;
        };
        if p.revision != self.document.revision()
            || p.selection
                != (
                    self.selection.ids.clone(),
                    self.selection.ports.clone(),
                    self.selection.relations.clone(),
                )
        {
            self.structure.proposal = None;
            return;
        }
        let mut confirm = false;
        let mut cancel = false;
        let dismissed = dialogs::show(
            ctx,
            ("source-proposal", self.id),
            "Source change preview",
            520.0,
            |ui| {
                ui.label(&p.label);
                ui.small("Referenced endpoints are preserved. Compiler layout/routing is checked before applying.");
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for edit in &p.edits {
                            ui.label("Before");
                            ui.code(
                                self.document.source()[edit.range.clone()]
                                    .chars()
                                    .take(4000)
                                    .collect::<String>(),
                            );
                            ui.label("After (up to 4000 characters)");
                            ui.code(edit.replacement.chars().take(4000).collect::<String>());
                        }
                    });
                if ui.button("Confirm change").clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            },
        );
        if confirm {
            let p = self.structure.proposal.take().unwrap();
            self.submit_structure(&p.label, p.owner, Ok(p.edits));
        } else if cancel || dismissed {
            self.structure.proposal = None;
        }
    }
}

mod members_ui;

#[cfg(test)]
mod tests;
