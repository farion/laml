//! Transient canvas text draft; only confirmation submits one compiler-checked patch.
use super::*;
use laml_ui::inline_text::Text;
#[cfg(test)]
mod tests;

pub(super) struct Draft {
    text: Text,
    value: String,
    revision: u64,
    focus: bool,
}
impl Editor {
    pub(super) fn inline_text_canvas(
        &mut self,
        ui: &mut egui::Ui,
        viewport: egui::Rect,
        image: egui::Rect,
        response: &egui::Response,
    ) -> bool {
        if self.inline_text.is_some()
            && (self.pending_command == Some(self.document.revision())
                || self.command_error.is_some()
                || dialogs::blocked(ui.ctx()))
        {
            return true;
        }
        if self
            .inline_text
            .as_ref()
            .is_some_and(|d| d.revision != self.document.revision())
            || !self.can_edit_visually()
        {
            self.inline_text = None;
            return false;
        }
        if self.inline_text.is_none()
            && response.double_clicked_by(egui::PointerButton::Primary)
            && let Some(pointer) = response.interact_pointer_pos()
            && let Some(preview) = &self.preview
        {
            let scale = f64::from(image.width()) / (preview.bounds.2 - preview.bounds.0);
            let x = preview.bounds.0 + f64::from(pointer.x - image.left()) / scale;
            let y = preview.bounds.1 + f64::from(pointer.y - image.top()) / scale;
            if let Some(text) = laml_ui::inline_text::hit(preview, self.document.source(), x, y) {
                self.graph.cancel_gesture();
                self.placement.cancel();
                self.creation.drag = None;
                self.notation_ui.drag = None;
                self.relation_drag = None;
                self.document.break_typing_group();
                self.inline_text = Some(Draft {
                    value: text.value.clone(),
                    text,
                    revision: self.document.revision(),
                    focus: true,
                });
            }
        }
        let Some(mut draft) = self.inline_text.take() else {
            return false;
        };
        let id = egui::Id::unique(("inline-text", self.id));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            ui.memory_mut(|m| m.surrender_focus(id));
            return true;
        }
        let preview = self.preview.as_ref().unwrap();
        let scale = image.width() / (preview.bounds.2 - preview.bounds.0) as f32;
        let b = draft.text.bounds;
        let origin = image.min
            + egui::vec2(
                (b.0 - preview.bounds.0) as f32,
                (b.1 - preview.bounds.1) as f32,
            ) * scale
            - egui::vec2(6.0, 4.0);
        let width =
            (((b.2 - b.0) as f32 * scale + 24.0).max(180.0)).min(viewport.width().max(40.0));
        let height = (22.0 * draft.value.lines().count().clamp(1, 12) as f32 + 12.0)
            .min(viewport.height().max(24.0));
        let pos = egui::pos2(
            origin.x.clamp(
                viewport.left(),
                (viewport.right() - width).max(viewport.left()),
            ),
            origin.y.clamp(
                viewport.top(),
                (viewport.bottom() - height).max(viewport.top()),
            ),
        );
        let first = draft.focus;
        if !draft.text.multiline {
            ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter));
        }
        let enter = !first
            && ui.input_mut(|i| {
                let mut pressed = false;
                i.events.retain(|event| {
                    let matched = matches!(event, egui::Event::Key {
                    key: egui::Key::Enter, pressed: true, modifiers, ..
                } if modifiers.is_none());
                    pressed |= matched;
                    !matched
                });
                pressed
            });
        let mut commit = false;
        egui::Area::new(id.with("overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(viewport);
                ui.set_width(width);
                let edit = if draft.text.multiline {
                    egui::TextEdit::multiline(&mut draft.value)
                        .desired_rows(1)
                        .return_key(Some(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        )))
                } else {
                    egui::TextEdit::singleline(&mut draft.value)
                };
                let field = ui.add_sized(
                    egui::vec2(width, height),
                    edit.id(id)
                        .desired_width(width)
                        .font(egui::TextStyle::Body)
                        .margin(egui::Margin::symmetric(6, 4)),
                );
                self.source_ui.extra_drafts.push(id);
                if first {
                    field.request_focus();
                    let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(draft.value.chars().count()),
                        )));
                    state.store(ui.ctx(), id);
                } else {
                    commit = enter
                        || field.lost_focus()
                        || ui.input(|i| {
                            i.pointer.any_pressed()
                                && i.pointer
                                    .interact_pos()
                                    .is_some_and(|p| !field.rect.contains(p))
                        });
                }
            });
        draft.focus = false;
        if commit {
            ui.memory_mut(|m| m.surrender_focus(id));
            self.confirm_inline_text(draft);
        } else {
            self.inline_text = Some(draft);
        }
        true
    }
    fn confirm_inline_text(&mut self, mut draft: Draft) {
        if draft.value == draft.text.value || draft.revision != self.document.revision() {
            return;
        }
        match draft.text.edits(self.document.source(), &draft.value) {
            Ok(edits) => {
                self.submit_command(Command {
                    edits,
                    label: "Edit diagram text".into(),
                    expectation: worker::Expectation::Notation {
                        line: None,
                        id: None,
                    },
                });
                if self.pending_command.is_some() || self.command_error.is_some() {
                    draft.focus = true;
                    self.inline_text = Some(draft);
                }
            }
            Err(error) => {
                self.command_error = Some(error);
                self.inline_text = Some(draft);
            }
        }
    }
}
