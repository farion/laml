use super::*;

impl Editor {
    pub(super) fn request_action(&mut self, action: Action, ctx: &egui::Context) {
        if self.file_ui.deferred.is_some() {
            return;
        }
        if self.file_ui.writing() || self.file_ui.picker.is_some() || self.workflow.busy() {
            self.close_cancelled = false;
            self.file_ui.deferred = Some(action);
            self.workflow.cancel_export();
            return;
        }
        self.cancel_file_read();
        if self.managed {
            match &action {
                Action::New => {
                    self.workspace_action = Some(workspace::Action::New);
                    return;
                }
                Action::Open(path) => {
                    self.workspace_action = Some(workspace::Action::Open(path.clone()));
                    return;
                }
                _ => {}
            }
        }
        self.close_cancelled = false;
        self.file_ui.after_save = None;
        self.file_ui.dialog = None;
        self.file_ui.overwrite = None;
        if self.document.is_dirty() || self.pending_command == Some(self.document.revision()) {
            self.file_ui.pending = Some(action);
        } else {
            self.perform(action, ctx);
        }
    }
    pub(super) fn perform(&mut self, action: Action, ctx: &egui::Context) {
        if self.file_ui.writing() || self.workflow.busy() {
            self.file_ui.deferred = Some(action);
            self.workflow.cancel_export();
            return;
        }
        self.cancel_file_read();
        match action {
            Action::New => {
                if let Err(error) = self.recovery.clear() {
                    self.fail_file(error);
                    return;
                }
                let source = editor_theme::new_source(self.editor_theme);
                self.document.replace_loaded(source.clone());
                self.file = FileState::new(None, &source);
                self.reset_view(ctx);
                self.file_ui.error = None;
                self.file_ui.conflict = false;
            }
            Action::Open(path) => self.read_file(path, Kind::Open),
            Action::Reload => match &self.file.path {
                Some(path) => self.read_file(path.clone(), Kind::Reload),
                None => self.fail_file("No file is open"),
            },
            Action::Recover(record) => {
                if let Some(path) = &record.path {
                    self.read_file(path.clone(), Kind::Recover(record));
                } else {
                    self.apply_recovery(None, record, ctx);
                }
            }
            Action::Close => {
                if let Err(error) = self.recovery.clear() {
                    self.fail_file(error);
                    return;
                }
                if let Some(store) = &mut self.recovery.store {
                    store.closed = true;
                }
                self.allow_close = true;
                if !self.managed {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
    fn read_file(&mut self, path: PathBuf, kind: Kind) {
        match protocol::address(&path) {
            Ok(address) => self.start_file_task(kind, Operation::OpenSource { address }),
            Err(error) => self.fail_file(error),
        }
    }
    pub(in crate::app) fn apply_recovery(
        &mut self,
        loaded: Option<(PathBuf, String)>,
        record: laml_ui::recovery::Record,
        ctx: &egui::Context,
    ) {
        let (path, baseline) =
            loaded.map_or((None, String::new()), |(path, text)| (Some(path), text));
        self.file = FileState::new(path, &baseline);
        self.document.replace_loaded(baseline);
        let _ = self.document.replace_text(record.source.clone());
        if self.file.path.is_none() { self.document.mark_unsaved(); }
        self.reset_view(ctx);
        if self.file.path.is_none() {
            self.recovery.origin = record.path.clone();
            if let Some(path) = &record.path {
                self.path = path.clone();
            }
        }
        self.recovery.recovered = true;
        self.recovery.restoring = Some(record.clone());
        if self.checkpoint() {
            self.finish_recovery();
        } else {
            #[cfg(target_arch = "wasm32")]
            if self.recovery.error.is_none() {
                self.recovery.dismissed.insert(record.file.clone());
                self.recovery.records.retain(|r| r.file != record.file);
            }
        }
        self.file_ui.error = None;
        self.file_ui.conflict = false;
    }
    pub(in crate::app) fn save_file(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() { self.download_laml(); return; }
        if self.file_ui.busy() {
            return;
        }
        if self.pending_command == Some(self.document.revision()) {
            self.fail_file("Wait for the pending visual change before saving.");
            return;
        }
        if let Some(path) = &self.file.path {
            self.send_file_save(path.clone(), Some(self.file.baseline().into()));
        } else {
            self.browse_file(Dialog::SaveAs, ctx);
        }
    }
    pub(super) fn saved(&mut self, was_current: bool, ctx: &egui::Context) {
        if self.document.is_dirty() {
            self.checkpoint();
        } else if let Err(error) = self.recovery.clear() {
            self.recovery.error = Some(error);
        }
        self.file_ui.error = None;
        self.file_ui.conflict = false;
        self.file_ui.dialog = None;
        self.file_ui.overwrite = None;
        self.path = self
            .file
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.laml"));
        if let Some(action) = self.file_ui.after_save.take() {
            if was_current && !self.document.is_dirty() && self.pending_command.is_none() {
                self.perform(action, ctx);
            } else {
                self.fail_file("Saved the submitted revision. Newer changes remain unsaved; continuing was cancelled.");
            }
        }
    }
    pub(in crate::app) fn save_as(
        &mut self,
        path: PathBuf,
        expected: Option<&str>,
        _ctx: &egui::Context,
    ) {
        if self.file_ui.busy() {
            return;
        }
        if self.pending_command == Some(self.document.revision()) {
            self.fail_file("Wait for the pending visual change before saving.");
            return;
        }
        self.inspect_save_as(path, Inspect::Confirmed(expected.map(str::to_owned)));
    }
}
