use super::*;
use laml_ui::delete_edit::{self, ContainerMode, Deletion};

type SelectionSnapshot = (Vec<String>, Vec<usize>, Vec<String>);
pub(super) struct Proposal {
    revision: u64,
    selection: SelectionSnapshot,
    ids: Vec<String>,
    ports: Vec<String>,
    plan: Deletion,
}
impl Editor {
    fn delete_selection_snapshot(&self) -> SelectionSnapshot {
        (
            self.selection.ids.clone(),
            self.selection.relations.clone(),
            self.selection.ports.clone(),
        )
    }
    pub(super) fn has_selection(&self) -> bool {
        self.notation_ui
            .selected
            .as_ref()
            .is_some_and(|s| s.2 == self.document.revision())
            || !self.selection.ids.is_empty()
            || !self.selection.relations.is_empty()
            || !self.selection.ports.is_empty()
    }
    pub(super) fn selected_container(&self) -> bool {
        self.selection.ids.len() == 1
            && self.selection.ports.is_empty()
            && self.selection.relations.is_empty()
            && self.preview.as_ref().is_some_and(|p| {
                p.diagram
                    .nodes
                    .iter()
                    .any(|n| n.parent.as_ref() == self.selection.ids.first())
            })
    }
    pub(super) fn deletion_shortcuts(&mut self, ctx: &egui::Context) {
        if !self.can_edit_visually()
            || !self.has_selection()
            || dialogs::blocked(ctx)
            || self.graph.dialog_open()
            || self.deletion.is_some()
            || self.close_dialog_active()
            || self.command_error.is_some()
            || self.structure.proposal.is_some()
            || self.graph.gesture.is_some()
            || self.placement.drag.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            return;
        }
        if ctx
            .memory(|m| m.focused())
            .is_some_and(|id| Some(id) != self.view.canvas_id)
        {
            return;
        }
        let key = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key: key @ (egui::Key::Delete | egui::Key::Backspace),
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if modifiers.is_none() => Some(*key),
                _ => None,
            })
        });
        if let Some(key) = key
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
        {
            if let Some((line, id, revision)) = self
                .notation_ui
                .selected
                .clone()
                .filter(|s| s.2 == self.document.revision())
            {
                let _ = revision;
                self.notation_delete(line, id.as_deref(), false);
            } else {
                self.propose_delete(ContainerMode::WithContents);
            }
        }
    }
    pub(super) fn deletion_window(&mut self, ctx: &egui::Context) {
        if self.deletion.as_ref().is_some_and(|p| {
            p.revision != self.document.revision()
                || p.selection != self.delete_selection_snapshot()
        }) {
            self.deletion = None;
        }
        let Some(proposal) = &self.deletion else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        let dismissed = dialogs::show(
            ctx,
            ("delete-selection", self.id),
            "Delete selection?",
            440.0,
            |ui| {
                ui.label(crate::i18n::tr(
                    "These elements and dependent connections will be removed:",
                ));
                egui::ScrollArea::vertical()
                    .id_salt("deletion-preview")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for description in &proposal.plan.affected {
                            ui.label(description);
                        }
                    });
                ui.horizontal(|ui| {
                    confirm = ui
                        .add_enabled(
                            self.can_edit_visually(),
                            egui::Button::new(crate::i18n::tr("Delete")),
                        )
                        .clicked();
                    cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
                });
            },
        );
        if confirm {
            self.confirm_delete();
        } else if cancel || dismissed {
            self.deletion = None;
        }
    }
    pub(super) fn propose_delete(&mut self, mode: ContainerMode) {
        self.propose_delete_targets(
            self.selection.ids.clone(),
            self.selection.relations.clone(),
            self.selection.ports.clone(),
            mode,
        );
    }
    pub(super) fn propose_delete_targets(
        &mut self,
        ids: Vec<String>,
        relations: Vec<usize>,
        ports: Vec<String>,
        mode: ContainerMode,
    ) {
        if !self.can_edit_visually() || (ids.is_empty() && relations.is_empty() && ports.is_empty())
        {
            return;
        }
        match delete_edit::plan_with_ports(self.document.source(), &ids, &relations, &ports, mode) {
            Ok(plan) => {
                self.command_error = None;
                self.deletion = Some(Proposal {
                    revision: self.document.revision(),
                    selection: self.delete_selection_snapshot(),
                    ids,
                    ports,
                    plan,
                });
            }
            Err(error) => {
                self.deletion = None;
                self.command_error = Some(error);
            }
        }
    }
    pub(super) fn confirm_delete(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        let Some(proposal) = self.deletion.take() else {
            return;
        };
        if proposal.revision != self.document.revision()
            || proposal.selection != self.delete_selection_snapshot()
        {
            self.command_error = Some("Deletion proposal is out of date. Source unchanged.".into());
            return;
        }
        self.submit_command(Command {
            edits: proposal.plan.edits,
            label: "Delete selection".into(),
            expectation: worker::Expectation::Deleted {
                nodes: proposal.ids,
                ports: proposal.ports,
            },
        });
    }
}
