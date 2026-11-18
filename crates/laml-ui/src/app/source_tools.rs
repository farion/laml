use super::*;
use laml_ui::{
    document::Edit,
    text_tools::{self, Action},
};
use std::ops::Range;

pub(super) struct SourceUi {
    pub context: source_context::State,
    pub line_numbers: bool,
    pub id: Option<egui::Id>,
    pub selection: Range<usize>,
    pub selection_revision: u64,
    pub jump: Option<Range<usize>>,
    pub diagnostic: Option<(u64, laml_ui::diagnostic::Diagnostic)>,
    pub find_open: bool,
    pub find: String,
    pub replace: String,
    pub case_sensitive: bool,
    pub focus_find: bool,
    pub auxiliary_ids: [Option<egui::Id>; 2],
    pub extra_drafts: Vec<egui::Id>,
    pub composing: bool,
    #[cfg(test)]
    pub rect: Option<egui::Rect>,
}
impl Default for SourceUi {
    fn default() -> Self {
        Self {
            context: Default::default(),
            line_numbers: true,
            id: None,
            selection: 0..0,
            selection_revision: 0,
            jump: None,
            diagnostic: None,
            find_open: false,
            find: String::new(),
            replace: String::new(),
            case_sensitive: true,
            focus_find: false,
            auxiliary_ids: [None, None],
            extra_drafts: Vec::new(),
            composing: false,
            #[cfg(test)]
            rect: None,
        }
    }
}
impl Editor {
    pub(super) fn history_shortcuts(&mut self, ctx: &egui::Context) {
        if dialogs::blocked(ctx)
            || self.inline_text.is_some()
            || self.graph.dialog_open()
            || self.deletion.is_some()
            || self.command_error.is_some()
        {
            return;
        }
        if ctx.memory(|m| m.focused()).is_some_and(|id| {
            self.source_ui.auxiliary_ids.contains(&Some(id))
                || self.source_ui.extra_drafts.contains(&id)
        }) {
            self.document.break_typing_group();
            return;
        }
        let redo = ctx.input_mut(|input| {
            let shift_z = input.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            );
            let y = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
            shift_z || y
        });
        let undo = ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
        if (redo && self.document.redo()) || (undo && self.document.undo()) {
            self.source_mutated();
        }
    }
    pub(super) fn source_mutated(&mut self) {
        self.inline_text = None;
        self.graph.cancel_gesture();
        self.remote.cancel();
        self.retry_after = None;
        self.graph.report_errors = false;
        self.graph.flash = None;
        self.changed = Instant::now();
        self.source_ui.diagnostic = None;
        self.advanced.completion = None;
        self.structure.proposal = None;
        self.source_ui.jump = None;
        self.error = None;
        self.command_error = None;
        self.pending_command = None;
        self.property_drafts.clear();
        self.membership = None;
        self.deletion = None;
        self.alignment = None;
        self.creation.drag = None;
        self.relation_drag = None;
        self.placement.cancel();
    }
    pub(super) fn apply_source_edits(
        &mut self,
        label: &str,
        edits: Vec<Edit>,
        selection: Option<Range<usize>>,
    ) {
        match self.document.apply(self.document.revision(), label, edits) {
            Ok(true) => {
                self.source_mutated();
                if let Some(range) = &selection {
                    self.source_ui.selection = range.clone();
                    self.source_ui.selection_revision = self.document.revision();
                }
                self.source_ui.jump = selection;
            }
            Err(error) => self.command_error = Some(format!("Source edit rejected: {error:?}")),
            _ => {}
        }
    }
    pub(super) fn source_action(&mut self, action: Action) {
        self.document.break_typing_group();
        if self.source_ui.selection_revision != self.document.revision() {
            self.command_error =
                Some("Source selection is out of date. Reselect before editing.".into());
            return;
        }
        match text_tools::command(
            self.document.source(),
            self.source_ui.selection.clone(),
            action,
        ) {
            Ok(change) => self.apply_source_edits(
                match action {
                    Action::Comment => "Toggle line comments",
                    Action::Indent => "Indent source",
                    Action::Outdent => "Outdent source",
                    Action::Newline => "Insert indented newline",
                },
                change.edits,
                Some(change.selection),
            ),
            Err(error) => self.command_error = Some(error),
        }
    }
    pub(super) fn find_next(&mut self, backward: bool) {
        self.document.break_typing_group();
        let ranges = text_tools::matches(
            self.document.source(),
            &self.source_ui.find,
            self.source_ui.case_sensitive,
        );
        let selected = &self.source_ui.selection;
        let range = if backward {
            ranges
                .iter()
                .rev()
                .find(|r| r.start < selected.start)
                .or_else(|| ranges.last())
        } else {
            ranges
                .iter()
                .find(|r| r.start >= selected.end && **r != *selected)
                .or_else(|| ranges.first())
        };
        if let Some(range) = range {
            self.source_ui.jump = Some(range.clone());
            self.source_ui.selection = range.clone();
            self.source_ui.selection_revision = self.document.revision();
        }
    }
    pub(super) fn replace_matches(&mut self, all: bool) {
        self.document.break_typing_group();
        if !all && self.source_ui.selection_revision != self.document.revision() {
            self.command_error = Some(
                "Source selection is out of date. Find the match again before replacing.".into(),
            );
            return;
        }
        let mut edits = text_tools::replacements(
            self.document.source(),
            &self.source_ui.find,
            &self.source_ui.replace,
            self.source_ui.case_sensitive,
        );
        if !all {
            edits.retain(|edit| edit.range == self.source_ui.selection);
        }
        let selection = edits
            .first()
            .map(|e| e.range.start..e.range.start + e.replacement.len());
        self.apply_source_edits(
            if all { "Replace all" } else { "Replace match" },
            edits,
            selection,
        );
    }
    pub(super) fn source_toolbar(&mut self, ui: &mut egui::Ui) {
        if !dialogs::blocked(ui.ctx()) {
            let ime_frame = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))));
            let composing = self.source_ui.composing || ime_frame;
            ui.input(|i| {
                for event in &i.events {
                    if let egui::Event::Ime(event) = event {
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                self.source_ui.composing = !text.is_empty()
                            }
                            egui::ImeEvent::Commit(_) => self.source_ui.composing = false,
                            _ => {}
                        }
                    }
                }
            });
            if composing {
                self.document.break_typing_group();
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            }
            let source_focused = ui.memory(|m| self.source_ui.id.is_some_and(|id| m.has_focus(id)));
            if source_focused
                && !composing
                && ui.input_mut(|i| {
                    i.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::F,
                    )
                })
            {
                self.format_source();
            }
            let find = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
            let replace = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::H));
            if find || replace {
                self.source_ui.find_open = true;
                self.source_ui.focus_find = true;
                self.document.break_typing_group();
            }
            let previous = ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F3));
            let next = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F3));
            if previous || next {
                self.find_next(previous);
            }
            let focused = ui.memory(|m| self.source_ui.id.is_some_and(|id| m.has_focus(id)));
            if focused
                && !composing
                && !find
                && !replace
                && !self.source_ui.focus_find
                && self.advanced.completion.is_none()
            {
                let comment =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Slash));
                let outdent =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab));
                let indent = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
                let enter =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                if comment {
                    self.source_action(Action::Comment);
                } else if outdent {
                    self.source_action(Action::Outdent);
                } else if indent {
                    self.source_action(Action::Indent);
                } else if enter {
                    self.source_action(Action::Newline);
                }
            }
        }
        let (entries, _) = self.document.history_usage();
        if entries == 0 && self.document.is_dirty() {
            ui.colored_label(
                egui::Color32::YELLOW,
                crate::i18n::tr("This edit exceeded the undo budget; no undo entry was retained."),
            );
        }
        if !self.source_ui.find_open {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::tr("Find"));
            let input =
                ui.add(egui::TextEdit::singleline(&mut self.source_ui.find).desired_width(110.0));
            self.source_ui.auxiliary_ids[0] = Some(input.id);
            if self.source_ui.focus_find {
                input.request_focus();
                self.source_ui.focus_find = false;
            }
            ui.checkbox(
                &mut self.source_ui.case_sensitive,
                crate::i18n::tr("Case sensitive"),
            );
            if ui.button(crate::i18n::tr("Previous")).clicked() {
                self.find_next(true);
            }
            if ui.button(crate::i18n::tr("Next")).clicked() {
                self.find_next(false);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::tr("Replace"));
            let input = ui
                .add(egui::TextEdit::singleline(&mut self.source_ui.replace).desired_width(110.0));
            self.source_ui.auxiliary_ids[1] = Some(input.id);
            if ui.button(crate::i18n::tr("Replace match")).clicked() {
                self.replace_matches(false);
            }
            if ui.button(crate::i18n::tr("Replace all")).clicked() {
                self.replace_matches(true);
            }
        });
        let count = text_tools::matches(
            self.document.source(),
            &self.source_ui.find,
            self.source_ui.case_sensitive,
        )
        .len();
        ui.small(crate::i18n::format("{count} literal matches · F3 / Shift+F3 · Unicode lowercase matching when case is ignored", &[("count", format!("{}", count))]));
    }
}
