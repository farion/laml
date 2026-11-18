use super::*;
use laml_ui::{
    source_assist::{self, snippet},
    syntax_tools,
};
#[derive(Default)]
pub(super) struct Advanced {
    pub completion: Option<(u64, source_assist::Completion)>,
    pub selected: Option<usize>,
    pub snippet: Option<snippet::Session>,
    pub anchor: Option<egui::Rect>,
    pub refresh: bool,
    pub scroll_selected: bool,
    #[cfg(test)]
    pub first_row: Option<egui::Rect>,
}
impl Editor {
    /// Runs before the ordinary source shortcuts and TextEdit consume input.
    pub(super) fn advanced_source(&mut self, ui: &mut egui::Ui) {
        if dialogs::blocked(ui.ctx()) {
            self.advanced.completion = None;
            return;
        }
        let focused = ui.memory(|m| self.source_ui.id.is_some_and(|id| m.has_focus(id)));
        if !focused
            || self.source_ui.composing
            || ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
        {
            self.advanced.completion = None;
            return;
        }
        if self
            .advanced
            .snippet
            .as_ref()
            .is_some_and(|s| s.revision != self.document.revision())
        {
            self.advanced.snippet = None;
        }
        if self
            .advanced
            .completion
            .as_ref()
            .is_some_and(|(r, _)| *r != self.document.revision())
        {
            self.advanced.completion = None;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Space)) {
            self.open_completion();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.advanced.completion = None;
            self.advanced.snippet = None;
            return;
        }
        let mut accepted = false;
        if let Some((_, candidates)) = &self.advanced.completion {
            let count = candidates.entries.len();
            if count > 0 {
                let down =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                if down {
                    self.advanced.scroll_selected = true;
                    self.advanced.selected =
                        Some(self.advanced.selected.map_or(0, |n| (n + 1) % count));
                }
                if up {
                    self.advanced.scroll_selected = true;
                    self.advanced.selected = Some(
                        self.advanced
                            .selected
                            .map_or(count - 1, |n| (n + count - 1) % count),
                    );
                }
                // Active snippet navigation takes precedence over incidental suggestions.
                let tab = self.advanced.snippet.is_none()
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                let enter = self.advanced.selected.is_some()
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                if tab || enter {
                    self.accept_completion(self.advanced.selected.unwrap_or(0));
                    accepted = true;
                }
            }
        }
        if !accepted
            && self.advanced.selected.is_none()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
        {
            self.advanced.completion = None;
        }
        if !accepted && self.advanced.snippet.is_some() {
            let backward = ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
            let forward = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
            if backward || forward {
                let selection = self
                    .advanced
                    .snippet
                    .as_mut()
                    .and_then(|s| s.step(backward));
                self.advanced.completion = None;
                if let Some(selection) = selection {
                    self.source_ui.jump = Some(selection);
                } else {
                    if let Some(s) = &self.advanced.snippet {
                        self.source_ui.jump = Some(s.end..s.end);
                    }
                    self.advanced.snippet = None;
                }
            }
        }
        if !accepted {
            self.source_pair_input(ui);
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
            return;
        }
        self.refresh_completion(true);
        self.document.break_typing_group();
    }
    pub(super) fn refresh_completion(&mut self, explicit: bool) {
        if !self.source_ui.selection.is_empty() || self.source_ui.composing {
            self.advanced.completion = None;
            return;
        }
        let result = source_assist::complete(
            self.document.source(),
            self.source_ui.selection.end,
            explicit,
        );
        self.advanced.completion =
            (!result.entries.is_empty()).then_some((self.document.revision(), result));
        self.advanced.selected = None;
    }
    pub(super) fn accept_completion(&mut self, n: usize) {
        let Some((revision, result)) = self.advanced.completion.take() else {
            return;
        };
        if revision != self.document.revision() {
            return;
        }
        let Some(entry) = result.entries.get(n) else {
            return;
        };
        let source = self.document.source();
        let line = source[..result.range.start]
            .rsplit('\n')
            .next()
            .unwrap_or("");
        let indent: String = line
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t'))
            .collect();
        let eol = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let expansion = snippet::expand(&entry.template, &indent, eol);
        let start = result.range.start;
        let caret = start + expansion.text.len();
        let selected = expansion
            .fields
            .first()
            .map_or(caret..caret, |r| start + r.start..start + r.end);
        let previous_session = self.advanced.snippet.take();
        let before = source.to_owned();
        self.apply_source_edits(
            "Complete source",
            vec![laml_ui::document::Edit {
                range: result.range,
                replacement: expansion.text.clone(),
            }],
            Some(selected),
        );
        self.update_source_snippet(previous_session, &before);
        if let Some(nested) = snippet::Session::new(&expansion, start, self.document.revision()) {
            if let Some(parent) = &mut self.advanced.snippet {
                parent
                    .fields
                    .splice(parent.active..parent.active + 1, nested.fields);
            } else {
                self.advanced.snippet = Some(nested);
            }
        }
        self.advanced.refresh = self.advanced.snippet.is_none();
    }
}
