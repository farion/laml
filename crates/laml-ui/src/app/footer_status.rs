//! Neutral progress messages with an activity indicator; diagnostics stay distinct.
use super::*;

impl Editor {
    pub(super) fn preview_updating(&self) -> bool {
        self.compiler.busy()
            || (self.error.is_none()
                && self.retry_after.is_none()
                && self.submitted != Some(self.document.revision()))
    }
}

pub(super) fn show(ui: &mut egui::Ui, editor: &Editor) {
    let preview_busy = editor.preview_updating();
    let operation_busy = editor.workflow.processing();
    let busy = preview_busy || operation_busy || editor.file_ui.writing();
    let current = editor.rendered == Some(editor.document.revision());
    let recovery_error = editor.recovery.error.as_ref().map(|error| {
        if editor.recovery.restoring.is_some() {
            format!("Recovery backup pending: {error}. Your work is open and the original draft is retained. Retrying automatically.")
        } else {
            format!("Recovery backup failed: {error}. Save your work explicitly.")
        }
    });
    #[cfg(target_arch = "wasm32")]
    let connection = if laml_ui::services::browser_mode() { crate::browser_storage::connection_status() } else { String::new() };
    #[cfg(not(target_arch = "wasm32"))]
    let connection = String::new();
    let text = if !connection.is_empty() { &connection } else if let Some(error) = recovery_error.as_deref() {
        error
    } else if preview_busy {
        "Updating preview…"
    } else {
        editor
            .error
            .as_deref()
            .or(editor.workflow.status.as_deref())
            .unwrap_or(if current {
                "Preview up to date"
            } else {
                "Preview out of date · visual editing locked"
            })
    };
    if busy {
        ui.add(egui::Spinner::new().size(14.0).color(appearance::color(ui.ctx(), appearance::TEXT)));
    }
    let color = if !connection.is_empty() || recovery_error.is_some() || (editor.error.is_some() && !preview_busy) {
        egui::Color32::LIGHT_RED
    } else {
        appearance::color(ui.ctx(), appearance::TEXT)
    };
    ui.add(egui::Label::new(egui::RichText::new(text).color(color)).truncate())
        .on_hover_text(text);
}
