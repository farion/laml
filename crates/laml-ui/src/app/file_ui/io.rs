use super::*;
use std::task::Poll;

impl Editor {
    pub(super) fn cancel_file_read(&mut self) {
        if !self.file_ui.writing()
            && let Some(task) = self.file_ui.task.take()
        {
            self.file_ui.service.forget(task.stamp);
        }
    }
    pub(super) fn start_file_task(&mut self, kind: Kind, operation: Operation) {
        if self.file_ui.writing() {
            self.fail_file("Wait for the current save to finish.");
            return;
        }
        self.cancel_file_read();
        match self
            .file_ui
            .service
            .submit(self.id, self.document.revision(), operation)
        {
            Ok(stamp) => {
                self.file_ui.task = Some(Task {
                    stamp,
                    origin: self.file.path.clone(),
                    kind,
                })
            }
            Err(error) => self.fail_file(error),
        }
    }
    pub(in crate::app) fn poll_file(&mut self, ctx: &egui::Context) {
        let picked =
            self.file_ui
                .picker
                .as_mut()
                .and_then(|(dialog, picker)| match picker.poll(ctx) {
                    Poll::Ready(path) => Some((*dialog, path)),
                    Poll::Pending => None,
                });
        if let Some((dialog, path)) = picked {
            self.file_ui.picker = None;
            if self.file_ui.deferred.is_some() && path.is_some() {
                // A file transition requested while the platform picker was open
                // supersedes the picker result, never the current document.
            } else {
                match (dialog, path) {
                    (Dialog::Open, Some(path)) => self.request_action(Action::Open(path), ctx),
                    (Dialog::SaveAs, Some(path)) => self.confirm_save_as(path, ctx),
                    (_, None) => {
                        self.file_ui.after_save = None;
                        self.file_ui.deferred = None;
                        self.close_cancelled = true;
                    }
                }
            }
        }
        while let Some(response) = self.file_ui.service.poll() {
            if !self
                .file_ui
                .task
                .as_ref()
                .is_some_and(|t| t.stamp == response.stamp)
            {
                continue;
            }
            let task = self.file_ui.task.take().expect("Matching file request");
            if task.origin != self.file.path {
                continue;
            }
            if !matches!(task.kind, Kind::Save { .. })
                && task.stamp.revision != self.document.revision()
            {
                if !matches!(task.kind, Kind::Poll) {
                    self.fail_file("File operation cancelled: the document changed while waiting.");
                }
                continue;
            }
            match response.result {
                Ok(reply) => self.apply_file_reply(task, reply, ctx),
                Err(_) if matches!(task.kind, Kind::Recover(_)) => {
                    if let Kind::Recover(record) = task.kind {
                        self.apply_recovery(None, record, ctx);
                    }
                }
                Err(error) => {
                    let poll = matches!(task.kind, Kind::Poll);
                    if poll {
                        self.file_ui.conflict = true;
                    }
                    if !(poll && self.file_ui.error.is_some() && !self.file_ui.poll_error) {
                        self.fail_file(error);
                        self.file_ui.poll_error = poll;
                    }
                }
            }
        }
        if !self.file_ui.busy()
            && !self.workflow.busy()
            && let Some(action) = self.file_ui.deferred.take()
        {
            self.request_action(action, ctx);
        }
        if self
            .file_ui
            .overwrite
            .as_ref()
            .is_some_and(|(_, _, revision)| *revision != self.document.revision())
        {
            self.file_ui.overwrite = None;
            self.fail_file(
                "Save confirmation expired because the document changed. Choose Save as again.",
            );
        }
        if self.file_ui.checked.elapsed() >= Duration::from_secs(1)
            && self.file_ui.task.is_none()
            && !self.file_ui.busy()
            && self.file_ui.overwrite.is_none()
            && self.file_ui.pending.is_none()
            && let Some(path) = &self.file.path
        {
            self.file_ui.checked = Instant::now();
            match protocol::address(path) {
                Ok(address) => self.start_file_task(Kind::Poll, Operation::OpenSource { address }),
                Err(error) => self.fail_file(error),
            }
        }
    }
    fn apply_file_reply(&mut self, task: Task, reply: Reply, ctx: &egui::Context) {
        match (task.kind, reply) {
            (Kind::Poll, Reply::OpenedSource { source, .. }) => {
                if self.file_ui.poll_error {
                    self.file_ui.error = None;
                    self.file_ui.poll_error = false;
                }
                let pending = self.pending_command == Some(self.document.revision());
                match self.file.observe(source, &mut self.document, pending) {
                    DiskStatus::Reloaded => {
                        self.reset_view(ctx);
                        self.file_ui.conflict = false;
                        self.file_ui.error = None;
                    }
                    DiskStatus::Conflict => self.file_ui.conflict = true,
                    DiskStatus::Unchanged => self.file_ui.conflict = false,
                }
            }
            (Kind::Open | Kind::Reload, Reply::OpenedSource { address, source }) => {
                if let Err(error) = self.recovery.clear() {
                    self.fail_file(error);
                    return;
                }
                self.file = FileState::new(Some(address.into()), &source);
                self.document.replace_loaded(source);
                self.reset_view(ctx);
                self.file_ui.error = None;
                self.file_ui.conflict = false;
            }
            (Kind::Recover(record), Reply::OpenedSource { address, source }) => {
                self.apply_recovery(Some((address.into(), source)), record, ctx);
            }
            (Kind::Inspect(intent), Reply::SourceSnapshot { address, source }) => {
                let path = PathBuf::from(address);
                if self.other_paths.contains(&path) {
                    self.fail_file(
                        "This file is already open in another tab. Choose another path.",
                    );
                    return;
                }
                match intent {
                    Inspect::Ask => {
                        if let Some(source) = source {
                            self.file_ui.overwrite = Some((path, source, task.stamp.revision));
                        } else {
                            self.send_file_save(path, None);
                        }
                    }
                    Inspect::Confirmed(expected) if expected == source => {
                        self.send_file_save(path, expected)
                    }
                    Inspect::Confirmed(_) => {
                        self.fail_file("File changed since confirmation. Choose Save as again.")
                    }
                }
            }
            (Kind::Save { source, .. }, Reply::Address(address)) => {
                let was_current = task.stamp.revision == self.document.revision();
                self.file
                    .acknowledge_save(address.into(), source, &mut self.document);
                let now = Instant::now();
                self.changed = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
                self.saved(was_current, ctx);
            }
            _ => self.fail_file("Unexpected file service response"),
        }
    }
    pub(super) fn confirm_save_as(&mut self, path: PathBuf, _ctx: &egui::Context) {
        self.inspect_save_as(path, Inspect::Ask);
    }
    pub(super) fn inspect_save_as(&mut self, path: PathBuf, intent: Inspect) {
        self.file_ui.dialog = None;
        match protocol::address(&path) {
            Ok(address) => {
                self.start_file_task(Kind::Inspect(intent), Operation::InspectSource { address })
            }
            Err(error) => self.fail_file(error),
        }
    }
    pub(super) fn send_file_save(&mut self, path: PathBuf, expected: Option<String>) {
        if self.other_paths.contains(&path) && self.file.path.as_ref() != Some(&path) {
            self.fail_file("This file is already open in another tab. Choose another path.");
            return;
        }
        if self.pending_command == Some(self.document.revision()) {
            self.fail_file("Wait for the pending visual change before saving.");
            return;
        }
        let source = self.document.source().to_string();
        match protocol::address(&path) {
            Ok(address) => self.start_file_task(
                Kind::Save {
                    source: source.clone(),
                    path,
                },
                Operation::SaveSource {
                    address,
                    source,
                    expected,
                },
            ),
            Err(error) => self.fail_file(error),
        }
    }
}
