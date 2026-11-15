//! Graph-local creation and deletion dialogs. No inspector proposals.
use super::*;
use laml_ui::{
    delete_edit,
    element_edit::{self, ElementKind},
};

pub(super) struct NodeDialog {
    pub revision: u64,
    pub kind: ElementKind,
    pub id: String,
    pub label: String,
    auto_id: bool,
    auto_label: bool,
    error: Option<String>,
}
pub(super) struct PortDialog {
    pub revision: u64,
    pub owner: String,
    pub id: String,
    pub name: String,
    pub side: String,
    error: Option<String>,
}
pub(super) struct DeleteDialog {
    pub revision: u64,
    pub ids: Vec<String>,
    pub plan: delete_edit::Deletion,
}

impl Editor {
    pub(super) fn open_node_dialog(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        self.graph.report_errors = true;
        match element_edit::add(
            self.document.source(),
            ElementKind::Rectangle,
            "",
            None,
            None,
        ) {
            Ok((id, _)) => {
                self.graph.node_dialog = Some(NodeDialog {
                    revision: self.document.revision(),
                    kind: ElementKind::Rectangle,
                    label: id.clone(),
                    id,
                    auto_id: true,
                    auto_label: true,
                    error: None,
                })
            }
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn open_port_dialog(&mut self, owner: &str, side: &str) {
        if !self.can_edit_visually() {
            return;
        }
        self.graph.report_errors = true;
        match laml_ui::port_edit::add(self.document.source(), owner, "") {
            Ok((id, _)) => {
                self.graph.port_dialog = Some(PortDialog {
                    revision: self.document.revision(),
                    owner: owner.into(),
                    name: id.clone(),
                    id,
                    side: side.into(),
                    error: None,
                })
            }
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn graph_delete(&mut self, ids: Vec<String>) {
        if !self.can_edit_visually() {
            return;
        }
        self.graph.report_errors = true;
        match delete_edit::plan(
            self.document.source(),
            &ids,
            &[],
            delete_edit::ContainerMode::WithContents,
        ) {
            Ok(plan) => {
                self.graph.deletion = Some(DeleteDialog {
                    revision: self.document.revision(),
                    ids,
                    plan,
                })
            }
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn submit_node_dialog(&mut self, dialog: &NodeDialog) -> Result<(), String> {
        if dialog.revision != self.document.revision() || !self.can_edit_visually() {
            return Err("Document changed. Close and reopen this dialog; source unchanged.".into());
        }
        let label = if dialog.kind == ElementKind::Junction {
            ""
        } else {
            &dialog.label
        };
        let edits =
            laml_ui::graph_edit::add_node(self.document.source(), dialog.kind, &dialog.id, label)?;
        self.submit_command(Command {
            edits,
            label: "Add node".into(),
            expectation: worker::Expectation::Element {
                id: dialog.id.clone(),
                parent: None,
                kind: dialog.kind,
            },
        });
        Ok(())
    }
    pub(super) fn submit_port_dialog(&mut self, dialog: &PortDialog) -> Result<(), String> {
        if dialog.revision != self.document.revision() || !self.can_edit_visually() {
            return Err("Document changed. Close and reopen this dialog; source unchanged.".into());
        }
        let edits = laml_ui::graph_edit::add_port(
            self.document.source(),
            &dialog.owner,
            &dialog.id,
            &dialog.name,
            &dialog.side,
        )?;
        self.submit_command(Command {
            edits,
            label: "Add port".into(),
            expectation: worker::Expectation::Port(dialog.id.clone()),
        });
        Ok(())
    }
    pub(super) fn confirm_graph_delete(&mut self) {
        let Some(dialog) = self.graph.deletion.take() else {
            return;
        };
        if dialog.revision != self.document.revision() || !self.can_edit_visually() {
            self.command_error = Some("Deletion is out of date. Source unchanged.".into());
            return;
        }
        self.submit_command(Command {
            edits: dialog.plan.edits,
            label: "Delete nodes".into(),
            expectation: worker::Expectation::Deleted {
                nodes: dialog.ids,
                ports: vec![],
            },
        });
    }
    pub(super) fn graph_windows(&mut self, ctx: &egui::Context) {
        if let Some(mut dialog) = self.graph.node_dialog.take() {
            let mut open = true;
            let mut submit = false;
            let mut cancel = false;
            let dismissed = dialogs::show(ctx, ("add-node", self.id), "Add node", 360.0, |ui| {
                let previous = dialog.kind;
                egui::ComboBox::from_id_salt("node-kind")
                    .selected_text(dialog.kind.title())
                    .show_ui(ui, |ui| {
                        for kind in ElementKind::ALL {
                            ui.selectable_value(&mut dialog.kind, kind, kind.title());
                        }
                    });
                if previous != dialog.kind
                    && dialog.auto_id
                    && let Ok((id, _)) =
                        element_edit::add(self.document.source(), dialog.kind, "", None, None)
                {
                    dialog.id = id;
                    if dialog.auto_label {
                        dialog.label = dialog.id.clone();
                    }
                }
                ui.label("ID");
                let input = ui.text_edit_singleline(&mut dialog.id);
                self.source_ui.extra_drafts.push(input.id);
                if input.changed() {
                    dialog.auto_id = false;
                    if dialog.auto_label {
                        dialog.label = dialog.id.clone();
                    }
                }
                ui.label("Label");
                let input = ui.add_enabled(
                    dialog.kind != ElementKind::Junction,
                    egui::TextEdit::singleline(&mut dialog.label),
                );
                self.source_ui.extra_drafts.push(input.id);
                if input.changed() {
                    dialog.auto_label = false;
                }
                if dialog.kind == ElementKind::Junction {
                    ui.small("Junctions are drawn without a label.");
                }
                if let Some(error) = &dialog.error {
                    command_errors::details(ui, error);
                }
                ui.horizontal(|ui| {
                    submit = ui
                        .add_enabled(self.can_edit_visually(), egui::Button::new("Add node"))
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if submit {
                match self.submit_node_dialog(&dialog) {
                    Ok(()) => open = false,
                    Err(e) => dialog.error = Some(e),
                }
            }
            if open && !cancel && !dismissed {
                self.graph.node_dialog = Some(dialog);
            }
        }
        if let Some(mut dialog) = self.graph.port_dialog.take() {
            let mut open = true;
            let mut submit = false;
            let mut cancel = false;
            let dismissed = dialogs::show(ctx, ("add-port", self.id), "Add port", 360.0, |ui| {
                ui.label(format!("Owner: {}", dialog.owner));
                ui.label("ID");
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut dialog.id).id);
                ui.label("Name");
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut dialog.name).id);
                egui::ComboBox::from_id_salt("port-side")
                    .selected_text(&dialog.side)
                    .show_ui(ui, |ui| {
                        for side in ["left", "right", "top", "bottom"] {
                            ui.selectable_value(&mut dialog.side, side.into(), side);
                        }
                    });
                if let Some(error) = &dialog.error {
                    command_errors::details(ui, error);
                }
                ui.horizontal(|ui| {
                    submit = ui
                        .add_enabled(self.can_edit_visually(), egui::Button::new("Add port"))
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if submit {
                match self.submit_port_dialog(&dialog) {
                    Ok(()) => open = false,
                    Err(e) => dialog.error = Some(e),
                }
            }
            if open && !cancel && !dismissed {
                self.graph.port_dialog = Some(dialog);
            }
        }
        if let Some(dialog) = &self.graph.deletion {
            let mut submit = false;
            let mut cancel = false;
            let dismissed = dialogs::show(
                ctx,
                ("graph-delete", self.id),
                "Delete nodes?",
                360.0,
                |ui| {
                    ui.label(
                        "The following nodes and their dependent connections will be removed:",
                    );
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            for affected in &dialog.plan.affected {
                                ui.label(affected);
                            }
                        });
                    ui.horizontal(|ui| {
                        submit = ui
                            .add_enabled(
                                dialog.revision == self.document.revision()
                                    && self.can_edit_visually(),
                                egui::Button::new("Delete nodes"),
                            )
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                    if dialog.revision != self.document.revision() {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Document changed; reopen the deletion preview.",
                        );
                    }
                },
            );
            if submit {
                self.confirm_graph_delete();
            } else if dismissed || cancel {
                self.graph.deletion = None;
            }
        }
        if self.graph.navigator {
            let dismissed = dialogs::show(ctx, ("elements", self.id), "Elements", 420.0, |ui| {
                self.element_tree(ui)
            });
            if dismissed {
                self.graph.navigator = false;
            }
        }
        if self.graph.addresses {
            let dismissed = dialogs::show(
                ctx,
                ("addresses", self.id),
                "Connect by address",
                460.0,
                |ui| self.address_controls(ui),
            );
            if dismissed || self.structure.proposal.is_some() {
                self.graph.addresses = false;
            }
        }
    }
}
