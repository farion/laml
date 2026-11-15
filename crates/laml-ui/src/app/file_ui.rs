//! Frontend file intents and asynchronous service/picker state.
use super::*;
use laml_ui::{
    backend::{
        async_client::Client,
        protocol::{self, Operation, Reply, Stamp},
    },
    file_state::{DiskStatus, FileState},
};
mod actions;
#[cfg(test)]
mod async_tests;
mod io;
mod reset;
#[cfg(test)]
mod tests;
mod windows;

#[derive(Clone)]
enum Action {
    New,
    Open(PathBuf),
    Reload,
    Close,
    Recover(laml_ui::recovery::Record),
}
#[derive(Clone, Copy)]
enum Dialog {
    Open,
    SaveAs,
}
enum Inspect {
    Ask,
    Confirmed(Option<String>),
}
enum Kind {
    Poll,
    Open,
    Reload,
    Recover(laml_ui::recovery::Record),
    Inspect(Inspect),
    Save { source: String, path: PathBuf },
}
struct Task {
    stamp: Stamp,
    origin: Option<PathBuf>,
    kind: Kind,
}
pub(super) struct FileUi {
    service: Client,
    task: Option<Task>,
    picker: Option<(Dialog, host_dialogs::Picker)>,
    dialog: Option<Dialog>,
    path: String,
    pending: Option<Action>,
    after_save: Option<Action>,
    deferred: Option<Action>,
    overwrite: Option<(PathBuf, String, u64)>,
    error: Option<String>,
    conflict: bool,
    poll_error: bool,
    checked: Instant,
}
impl FileUi {
    pub fn new(backend: laml_ui::services::Services) -> Self {
        Self {
            service: backend.client(),
            task: None,
            picker: None,
            dialog: None,
            path: String::new(),
            pending: None,
            after_save: None,
            deferred: None,
            overwrite: None,
            error: None,
            conflict: false,
            poll_error: false,
            checked: Instant::now(),
        }
    }
    pub fn busy(&self) -> bool {
        self.picker.is_some()
            || self
                .task
                .as_ref()
                .is_some_and(|t| !matches!(t.kind, Kind::Poll))
    }
    pub fn writing(&self) -> bool {
        self.task
            .as_ref()
            .is_some_and(|t| matches!(t.kind, Kind::Save { .. }))
    }
    pub fn reserved_path(&self) -> Option<&PathBuf> {
        self.overwrite
            .as_ref()
            .map(|(path, _, _)| path)
            .or_else(|| {
                self.task.as_ref().and_then(|task| match &task.kind {
                    Kind::Save { path, .. } => Some(path),
                    _ => None,
                })
            })
    }
}
impl Editor {
    pub(super) fn file_events(&mut self, ctx: &egui::Context) {
        self.poll_file(ctx);
        if !self.managed && ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_action(Action::Close, ctx);
        }
        if dialogs::blocked(ctx)
            || self.graph.dialog_open()
            || self.deletion.is_some()
            || self.command_error.is_some()
            || self.file_ui.busy()
            || self.file_ui.deferred.is_some()
        {
            return;
        }
        let save_as = ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::S,
            )
        });
        let save = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let open = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O));
        let new = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N));
        if save_as {
            self.browse_file(Dialog::SaveAs, ctx);
        } else if save {
            self.save_file(ctx);
        }
        if open {
            self.browse_file(Dialog::Open, ctx);
        }
        if new {
            self.new_tab(ctx);
        }
    }
    pub(super) fn save_as_dialog(&mut self, ctx: &egui::Context) {
        self.browse_file(Dialog::SaveAs, ctx);
    }
    pub(super) fn request_recovery(
        &mut self,
        record: laml_ui::recovery::Record,
        ctx: &egui::Context,
    ) {
        self.request_action(Action::Recover(record), ctx);
    }
    pub(super) fn new_tab(&mut self, ctx: &egui::Context) {
        self.request_action(Action::New, ctx);
    }
    pub(super) fn open_tab(&mut self, ctx: &egui::Context) {
        self.browse_file(Dialog::Open, ctx);
    }
    pub(super) fn request_tab_close(&mut self, ctx: &egui::Context) {
        self.request_action(Action::Close, ctx);
    }
    pub(super) fn close_dialog_active(&self) -> bool {
        self.file_ui.busy()
            || self.file_ui.pending.is_some()
            || self.file_ui.deferred.is_some()
            || self.file_ui.after_save.is_some()
            || self.file_ui.dialog.is_some()
            || self.file_ui.overwrite.is_some()
            || self.file_ui.error.is_some()
            || self.workflow.writing()
    }
    fn browse_file(&mut self, dialog: Dialog, _ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            match dialog {
                Dialog::Open => crate::browser_storage::import_laml(),
                Dialog::SaveAs => self.download_laml(),
            }
            return;
        }
        if self.file_ui.busy() {
            return;
        }
        let picker = match dialog {
            Dialog::Open => host_dialogs::Picker::open(),
            Dialog::SaveAs => host_dialogs::Picker::save(
                self.path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                "laml",
            ),
        };
        self.file_ui.picker = Some((dialog, picker));
    }
    fn show_file_dialog(&mut self, dialog: Dialog) {
        self.file_ui.path = self.path.display().to_string();
        self.file_ui.dialog = Some(dialog);
        self.file_ui.error = None;
    }
    pub(super) fn fail_pending_file_action(&mut self, error: &str) {
        if self.file_ui.deferred.is_some() {
            self.fail_file(error);
        }
    }
    fn fail_file(&mut self, error: impl ToString) {
        self.file_ui.poll_error = false;
        self.file_ui.error = Some(error.to_string());
        self.file_ui.after_save = None;
        self.file_ui.deferred = None;
        self.close_cancelled = true;
    }
}
