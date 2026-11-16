use super::*;
use laml::syntax::FreeAlignment;
use laml_ui::alignment_edit::AlignmentPlan;
#[cfg(test)]
mod tests;

pub(super) struct Proposal {
    revision: u64,
    ids: Vec<String>,
    align: FreeAlignment,
    plan: AlignmentPlan,
}
impl Editor {
    #[cfg(test)]
    pub(super) fn alignment_toolbar(&mut self, ui: &mut egui::Ui) {
        let enabled = self.can_edit_visually()
            && self.selection.relations.is_empty()
            && self.selection.ports.is_empty()
            && self.selection.ids.len() >= 2;
        if self.alignment.as_ref().is_some_and(|proposal| {
            proposal.revision != self.document.revision() || proposal.ids != self.selection.ids
        }) {
            self.alignment = None;
        }
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(crate::i18n::tr("Align:"));
                for (label, align) in [
                    ("Left", FreeAlignment::Left),
                    ("Right", FreeAlignment::Right),
                    ("Top", FreeAlignment::Top),
                    ("Bottom", FreeAlignment::Bottom),
                    ("Center X", FreeAlignment::CenterX),
                    ("Center Y", FreeAlignment::CenterY),
                    ("Center · overlap", FreeAlignment::Center),
                ] {
                    if ui.button(crate::i18n::tr(label)).clicked() {
                        self.propose_alignment(align);
                    }
                }
                if let Some(reference) = self.selection.ids.last() {
                    ui.label(crate::i18n::format(
                        "Reference: {reference}",
                        &[("reference", reference.to_string())],
                    ));
                }
            });
        });
    }
    pub(super) fn propose_alignment(&mut self, align: FreeAlignment) {
        if !self.can_edit_visually() {
            return;
        }
        match laml_ui::alignment_edit::plan(self.document.source(), &self.selection.ids, align) {
            Ok(plan) => {
                self.command_error = None;
                self.alignment = Some(Proposal {
                    revision: self.document.revision(),
                    ids: self.selection.ids.clone(),
                    align,
                    plan,
                });
            }
            Err(error) => {
                self.alignment = None;
                self.command_error = Some(error);
            }
        }
    }
    #[cfg(test)]
    pub(super) fn alignment_confirmation(&mut self, ui: &mut egui::Ui) {
        let Some(proposal) = &self.alignment else {
            return;
        };
        if proposal.revision != self.document.revision() || proposal.ids != self.selection.ids {
            self.alignment = None;
            return;
        }
        ui.heading(crate::i18n::tr("Alignment proposal"));
        if proposal.align == FreeAlignment::Center {
            ui.colored_label(
                egui::Color32::YELLOW,
                crate::i18n::tr("Shared center intentionally overlaps these nodes."),
            );
        }
        for rule in &proposal.plan.removed {
            ui.label(crate::i18n::format(
                "Remove: {rule}",
                &[("rule", rule.to_string())],
            ));
        }
        for rule in &proposal.plan.preserved {
            ui.label(crate::i18n::format(
                "Preserve: {rule}",
                &[("rule", rule.to_string())],
            ));
        }
        for rule in &proposal.plan.added {
            ui.label(crate::i18n::format(
                "Add: {rule}",
                &[("rule", rule.to_string())],
            ));
        }
        if proposal.plan.edits.is_empty() {
            ui.label(crate::i18n::tr(
                "These alignment rules are already present.",
            ));
        }
        ui.add_enabled_ui(self.can_edit_visually(), |ui| {
            if ui.button(crate::i18n::tr("Apply alignment")).clicked() {
                self.apply_alignment();
            }
        });
        if ui.button(crate::i18n::tr("Cancel alignment")).clicked() {
            self.alignment = None;
        }
        ui.separator();
    }
    pub(super) fn apply_alignment(&mut self) {
        if !self.can_edit_visually() {
            return;
        }
        let Some(proposal) = self.alignment.take() else {
            return;
        };
        if proposal.revision != self.document.revision() || proposal.ids != self.selection.ids {
            return;
        }
        self.submit_command(Command {
            edits: proposal.plan.edits,
            label: "Align nodes".into(),
            expectation: worker::Expectation::Alignment {
                ids: proposal.ids,
                align: proposal.align,
            },
        });
    }
}
