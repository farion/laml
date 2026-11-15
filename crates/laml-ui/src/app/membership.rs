use super::*;
use laml_ui::membership_edit;

pub(super) struct Proposal {
    revision: u64,
    selected: String,
    parent: Option<String>,
    plan: membership_edit::Move,
}
impl Editor {
    pub(super) fn membership_controls(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids.len() != 1
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            self.membership = None;
            if !self.selection.ids.is_empty() {
                inspector_ui::row(ui, "Container", |ui| {
                    ui.add_enabled(false, egui::Button::new("Select one node"))
                        .on_disabled_hover_text(
                            "Membership changes require exactly one selected node.",
                        );
                });
            }
            return;
        }
        let selected = self.selection.ids[0].clone();
        if self
            .membership
            .as_ref()
            .is_some_and(|p| p.revision != self.document.revision() || p.selected != selected)
        {
            self.membership = None;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let current = preview
            .diagram
            .nodes
            .iter()
            .find(|n| n.id == selected)
            .and_then(|n| n.parent.clone())
            .unwrap_or_default();
        let packages: Vec<_> = preview
            .diagram
            .nodes
            .iter()
            .filter(|n| laml_ui::notation::container_kind(n.kind))
            .map(|n| n.id.clone())
            .collect();

        let mut target = current.clone();
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            inspector_ui::row(ui, "Container", |ui| {
                egui::ComboBox::from_id_salt("membership-target")
                    .width(ui.available_width())
                    .selected_text(if target.is_empty() { "Root" } else { &target })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut target, String::new(), "Root");
                        for parent in packages {
                            let result = membership_edit::plan(
                                self.document.source(),
                                &selected,
                                Some(&parent),
                            );
                            let response = ui.add_enabled(
                                parent == current || result.is_ok(),
                                egui::Button::selectable(target == parent, &parent),
                            );
                            if let Err(error) = result {
                                response.clone().on_disabled_hover_text(error);
                            }
                            if response.clicked() {
                                target = parent;
                            }
                        }
                    });
            });
        });
        if target != current {
            self.propose_membership((!target.is_empty()).then_some(target.as_str()));
            self.confirm_membership();
        }
    }
    pub(super) fn propose_membership(&mut self, parent: Option<&str>) {
        if !self.can_edit_visually()
            || self.selection.ids.len() != 1
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            return;
        }
        let selected = self.selection.ids[0].clone();
        match membership_edit::plan(self.document.source(), &selected, parent) {
            Ok(plan) => {
                self.membership = Some(Proposal {
                    revision: self.document.revision(),
                    selected,
                    parent: parent.map(str::to_string),
                    plan,
                });
                self.command_error = None;
            }
            Err(error) => {
                self.membership = None;
                self.command_error = Some(error);
            }
        }
    }
    pub(super) fn confirm_membership(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        let Some(proposal) = self.membership.take() else {
            return;
        };
        if proposal.revision != self.document.revision()
            || self.selection.ids != [proposal.selected]
            || !self.selection.ports.is_empty()
            || !self.selection.relations.is_empty()
        {
            self.command_error =
                Some("Membership proposal is out of date. Source unchanged.".into());
            return;
        }
        self.submit_command(Command {
            edits: proposal.plan.edits,
            label: "Move container membership".into(),
            expectation: worker::Expectation::Membership {
                id: proposal.plan.id,
                parent: proposal.parent,
            },
        });
    }
}
