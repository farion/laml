//! Source-only context actions, using the original selection and one shared Undo.
use super::*;
use egui_phosphor::regular::{CLIPBOARD_TEXT, COPY, SCISSORS, TRASH};
use laml_ui::document::Edit;
use std::ops::Range;

#[derive(Clone, Debug)]
struct Snapshot {
    revision: u64,
    range: Range<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Copy,
    Paste,
    Cut,
    Delete,
}
struct Pending {
    selection: Snapshot,
    action: Action,
    result: mpsc::Receiver<Result<Option<String>, String>>,
}
#[derive(Default)]
pub(super) struct State {
    selection: Option<Snapshot>,
    pending: Option<Pending>,
}
impl Editor {
    pub(super) fn source_context_menu(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.secondary_clicked() {
            self.source_ui.context.selection = Some(Snapshot {
                revision: self.source_ui.selection_revision,
                range: self.source_ui.selection.clone(),
            });
        }
        let selection = self.source_ui.context.selection.clone();
        let valid = selection
            .as_ref()
            .is_some_and(|s| self.source_context_valid(s))
            && self.source_ui.context.pending.is_none();
        let selected = selection.as_ref().is_some_and(|s| !s.range.is_empty());
        let mut chosen = None;
        response.context_menu(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            ui.spacing_mut().interact_size.y = 32.0;
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.set_min_width(0.0);
            for (action, icon, label) in [
                (Action::Copy, COPY, "Copy"),
                (Action::Paste, CLIPBOARD_TEXT, "Paste"),
                (Action::Cut, SCISSORS, "Cut"),
                (Action::Delete, TRASH, "Delete"),
            ] {
                if graph_context::action(
                    ui,
                    valid && (selected || action == Action::Paste),
                    icon,
                    label,
                )
                .clicked()
                {
                    chosen = Some(action);
                    ui.close();
                }
            }
        });
        if let (Some(action), Some(selection)) = (chosen, selection) {
            self.source_context_action(ui.ctx(), selection, action);
        }
    }
    fn source_context_valid(&self, selection: &Snapshot) -> bool {
        selection.revision == self.document.revision()
            && self
                .document
                .source()
                .get(selection.range.clone())
                .is_some()
    }
    fn source_context_action(&mut self, ctx: &egui::Context, selection: Snapshot, action: Action) {
        if !self.source_context_valid(&selection) {
            return;
        }
        self.document.break_typing_group();
        self.source_ui.jump = Some(selection.range.clone());
        if let Some(id) = self.source_ui.id {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        if action == Action::Delete {
            self.source_context_replace(selection, "", "Delete source selection");
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if action == Action::Paste {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            } else {
                ctx.copy_text(self.document.source()[selection.range.clone()].into());
                if action == Action::Cut {
                    self.source_context_replace(selection, "", "Cut source selection");
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let write = (action != Action::Paste)
                .then(|| self.document.source()[selection.range.clone()].to_owned());
            self.source_ui.context.pending = Some(Pending {
                selection,
                action,
                result: crate::web_clipboard::text(write, ctx),
            });
        }
    }
    fn source_context_replace(&mut self, selection: Snapshot, text: &str, label: &str) {
        if !self.source_context_valid(&selection) {
            self.error_toast("Source changed while accessing the clipboard. Nothing was edited.");
            return;
        }
        let at = selection.range.start + text.len();
        self.apply_source_edits(
            label,
            vec![Edit {
                range: selection.range,
                replacement: text.into(),
            }],
            Some(at..at),
        );
    }
    pub(super) fn source_context_tick(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.source_ui.context.pending else {
            return;
        };
        let result = match pending.result.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Clipboard operation failed. Use the keyboard shortcut instead.".into())
            }
        };
        let pending = self.source_ui.context.pending.take().unwrap();
        match result {
            Ok(_) if !self.source_context_valid(&pending.selection) => self
                .error_toast("Source changed while accessing the clipboard. Nothing was edited."),
            Ok(Some(text)) if pending.action == Action::Paste => self.source_context_replace(
                pending.selection,
                &text.replace("\r\n", "\n"),
                "Paste source text",
            ),
            Ok(_) if pending.action == Action::Cut => {
                self.source_context_replace(pending.selection, "", "Cut source selection")
            }
            Ok(_) => {}
            Err(error) => self.error_toast(error),
        }
    }
}

#[cfg(test)]
mod tests;
