use super::*;
use laml_ui::syntax_tools;
#[derive(Default)]
pub(super) struct Advanced {
    pub completion: Option<(u64, syntax_tools::Completion)>,
    selected: usize,
}
impl Editor {
    pub(super) fn advanced_source(&mut self, ui: &mut egui::Ui) {
        if dialogs::blocked(ui.ctx()) {
            return;
        }
        let focused = ui.memory(|m| self.source_ui.id.is_some_and(|id| m.has_focus(id)));
        let complete =
            focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Space));
        let format = focused
            && ui.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::F,
                )
            });
        if complete {
            self.open_completion();
        }
        if format {
            self.format_source();
        }
        let Some((revision, candidates)) = &self.advanced.completion else {
            return;
        };
        if *revision != self.document.revision() {
            self.advanced.completion = None;
            return;
        }
        let count = candidates.values.len();
        let down = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
        if down && count > 0 {
            self.advanced.selected = (self.advanced.selected + 1) % count;
        }
        if up && count > 0 {
            self.advanced.selected = (self.advanced.selected + count - 1) % count;
        }
        let accept = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let cancel = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let mut picked = accept
            .then_some(self.advanced.selected)
            .filter(|_| count > 0);
        let mut open = !cancel;
        egui::Window::new("Source completion")
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                if count == 0 {
                    ui.label("No matching completions in this source context.");
                }
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (n, value) in candidates.values.iter().enumerate() {
                            if ui
                                .selectable_label(n == self.advanced.selected, value)
                                .clicked()
                            {
                                picked = Some(n);
                            }
                        }
                    });
            });
        if let Some(n) = picked {
            self.accept_completion(n);
        } else if !open {
            self.advanced.completion = None;
        }
    }
    pub(super) fn format_source(&mut self) {
        if self.source_ui.composing {
            self.command_error = Some("Finish text composition before formatting.".into());
            return;
        }
        match syntax_tools::format(self.document.source()) {
            Ok(edits) => self.apply_source_edits("Format source", edits, None),
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn open_completion(&mut self) {
        if self.source_ui.selection_revision != self.document.revision() {
            self.command_error = Some("Source caret is out of date; reselect it.".into());
            return;
        }
        let result = syntax_tools::complete(
            self.document.source(),
            self.source_ui.selection.end,
            self.preview.as_ref().map(|p| &p.diagram),
        );
        self.advanced.completion = Some((self.document.revision(), result));
        self.advanced.selected = 0;
        self.document.break_typing_group();
    }
    pub(super) fn accept_completion(&mut self, n: usize) {
        let Some((revision, result)) = self.advanced.completion.take() else {
            return;
        };
        if revision != self.document.revision() {
            return;
        }
        let Some(value) = result.values.get(n) else {
            return;
        };
        let caret = result.range.start + value.len();
        self.apply_source_edits(
            "Complete source",
            vec![laml_ui::document::Edit {
                range: result.range,
                replacement: value.clone(),
            }],
            Some(caret..caret),
        );
    }
}
