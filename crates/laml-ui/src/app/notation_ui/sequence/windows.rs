use super::*;
impl Editor {
    pub(in crate::app) fn sequence_window(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.notation_ui.sequence.dialog.take() else {
            return;
        };
        let title = match form.action {
            Action::Participant => "Add participant",
            Action::Note => "Add note",
            Action::SyncActivate => "Add call with activation",
            Action::Async => "Add asynchronous message",
            Action::Reply => "Add reply",
            _ => "Add synchronous call",
        };
        let mut submit = false;
        let mut cancel = false;
        let dismissed = dialogs::show(ctx, ("sequence-insert", self.id), title, 400.0, |ui| {
            if matches!(form.action, Action::Participant | Action::Note) {
                ui.label(crate::i18n::tr("ID"));
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut form.id).id);
            } else {
                ui.label(format!("{} → {}", form.anchor.owner, form.target));
            }
            if matches!(form.action, Action::Sync | Action::SyncActivate) {
                let mut activation =
                    form.action == Action::SyncActivate || form.anchor.activation_line.is_some();
                if ui
                    .add_enabled(
                        form.anchor.activation_line.is_none(),
                        egui::Checkbox::new(
                            &mut activation,
                            crate::i18n::tr("Start target activation"),
                        ),
                    )
                    .changed()
                {
                    form.action = if activation {
                        Action::SyncActivate
                    } else {
                        Action::Sync
                    };
                }
            }
            if form.anchor.activation_line.is_some() {
                ui.label(crate::i18n::tr(
                    "Uses the existing activation and keeps its end.",
                ));
            }
            ui.label(if form.action == Action::Note {
                crate::i18n::tr("Text")
            } else {
                crate::i18n::tr("Label")
            });
            self.source_ui
                .extra_drafts
                .push(ui.text_edit_multiline(&mut form.label).id);
            let label = if form.action == Action::Participant && form.label.trim().is_empty() {
                form.id.as_str()
            } else {
                form.label.as_str()
            };
            let edits =
                self.sequence_candidate(&form.anchor, form.action, &form.target, &form.id, label);
            if let Err(error) = &edits {
                ui.label(crate::i18n::message(error));
            }
            if let Some(error) = &form.error {
                command_errors::details(ui, error);
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                submit = ui
                    .add_enabled(
                        edits.is_ok()
                            && self.can_edit_visually()
                            && self.document.revision() == form.revision,
                        egui::Button::new(crate::i18n::tr("Add")),
                    )
                    .clicked();
                cancel = ui.button(crate::i18n::tr("Cancel")).clicked();
            });
            if form.revision != self.document.revision() {
                ui.label(crate::i18n::tr(
                    "Document changed. Reopen this action at the desired insertion point.",
                ));
            }
        });
        if submit {
            let label = if form.action == Action::Participant && form.label.trim().is_empty() {
                form.id.as_str()
            } else {
                form.label.as_str()
            };
            match self.sequence_candidate(&form.anchor, form.action, &form.target, &form.id, label)
            {
                Ok(edits) => {
                    self.notation_submit(title, edits, None);
                    return;
                }
                Err(error) => form.error = Some(error),
            }
        }
        if !cancel && !dismissed {
            self.notation_ui.sequence.dialog = Some(form);
        }
    }
}
