use super::*;
use std::task::Poll;

impl Editor {
    pub(in crate::app) fn workflow_tick(&mut self) {
        let picked = self
            .workflow
            .picker
            .as_mut()
            .and_then(|(format, picker, ctx)| match picker.poll(ctx) {
                Poll::Ready(path) => Some((*format, path)),
                Poll::Pending => None,
            });
        if let Some((format, path)) = picked {
            self.workflow.picker = None;
            if !std::mem::take(&mut self.workflow.dismiss_picker)
                && let Some(path) = path
            {
                self.start_export(path, format);
            }
        }
        while let Some(response) = self.workflow.service.poll() {
            if self.workflow.watch_request == Some(response.stamp) {
                self.workflow.watch_request = None;
                if response.stamp.revision == self.document.revision()
                    && self.workflow.path == self.path
                {
                    match response.result {
                        Ok(Reply::DependencyWatch { watch, .. }) => {
                            self.workflow.watch = Some(watch);
                            self.workflow.retry_after = None;
                        }
                        Err(error) => {
                            self.workflow.watch = None;
                            self.workflow.revision = None;
                            self.workflow.retry_after =
                                Some(Instant::now() + Duration::from_secs(2));
                            self.workflow.status = Some(format!("Dependency check failed: {error}"))
                        }
                        _ => {
                            self.workflow.status =
                                Some("Unexpected dependency service response".into())
                        }
                    }
                }
            } else if self.workflow.poll_request == Some(response.stamp) {
                self.workflow.poll_request = None;
                if response.stamp.revision != self.document.revision()
                    || self.workflow.path != self.path
                {
                    continue;
                }
                match response.result {
                    Ok(Reply::DependenciesChanged(true)) => {
                        self.document.invalidate();
                        self.source_mutated();
                        let now = Instant::now();
                        self.changed = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
                        self.workflow.invalidate_watch();
                    }
                    Ok(Reply::DependenciesChanged(false)) => {}
                    Err(error) => {
                        self.workflow.watch = None;
                        self.workflow.revision = None;
                        self.workflow.retry_after = Some(Instant::now() + Duration::from_secs(2));
                        self.workflow.status = Some(format!("Dependency check failed: {error}"));
                    }
                    _ => {
                        self.workflow.status = Some("Unexpected dependency service response".into())
                    }
                }
            } else {
                self.export_reply(response);
            }
        }
        if self.workflow.export.as_ref().is_some_and(|job| {
            !matches!(job.stage, Stage::Committing(_) | Stage::Cancelling(_))
                && (job.revision != self.document.revision()
                    || job.origin != self.path
                    || job.cancel)
        }) {
            self.workflow.cancel_export();
            if self
                .workflow
                .export
                .as_ref()
                .is_some_and(|job| matches!(job.stage, Stage::Confirm { .. }))
            {
                self.confirm_export();
            }
        }
        if laml_ui::services::browser_mode() { return; }
        let changed = self.workflow.revision != Some(self.document.revision())
            || self.workflow.path != self.path;
        if changed {
            self.workflow.invalidate_watch();
        }
        if self.workflow.watch_request.is_none()
            && self.workflow.poll_request.is_none()
            && self
                .workflow
                .retry_after
                .is_none_or(|at| Instant::now() >= at)
            && (changed || self.workflow.checked.elapsed() >= Duration::from_secs(1))
        {
            self.workflow.checked = Instant::now();
            let operation = if let Some(watch) = self.workflow.watch {
                Ok(Operation::DependenciesChanged { watch })
            } else {
                protocol::address(&self.path).map(|address| Operation::WatchDependencies {
                    source: self.document.source().into(),
                    address,
                })
            };
            let registering = self.workflow.watch.is_none();
            match operation.and_then(|op| {
                self.workflow
                    .service
                    .submit(self.workflow.document, self.document.revision(), op)
            }) {
                Ok(stamp) => {
                    self.workflow.path = self.path.clone();
                    self.workflow.revision = Some(self.document.revision());
                    if registering {
                        self.workflow.watch_request = Some(stamp);
                    } else {
                        self.workflow.poll_request = Some(stamp);
                    }
                }
                Err(error) => {
                    self.workflow.watch = None;
                    self.workflow.revision = None;
                    self.workflow.retry_after = Some(Instant::now() + Duration::from_secs(2));
                    self.workflow.status = Some(format!("Dependency check failed: {error}"))
                }
            }
        }
    }
    fn export_reply(&mut self, response: protocol::Response) {
        let matches = self
            .workflow
            .export
            .as_ref()
            .is_some_and(|job| match job.stage {
                Stage::Preparing(stamp) | Stage::Committing(stamp) | Stage::Cancelling(stamp) => {
                    stamp == response.stamp
                }
                _ => false,
            });
        if !matches {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            match response.result {
                Ok(Reply::Export(bytes)) => {
                    let format = self.workflow.export_format;
                    let origin = &self.workflow.export.as_ref().unwrap().origin;
                    let name = format!("{}.{}", origin.file_stem().unwrap_or_default().to_string_lossy(), format.extension());
                    let mime = match format { Format::Svg => "image/svg+xml", Format::Png => "image/png", Format::Pdf => "application/pdf" };
                    crate::browser_storage::download(&name, &bytes, mime);
                    self.workflow.export = None;
                    self.workflow.status = Some(format!("Downloaded {name}"));
                }
                Err(error) => self.export_error(error),
                _ => self.export_error("Unexpected download response"),
            }
            return;
        }
        if let Err(error) = response.result {
            self.export_error(error);
            return;
        }
        let job = self.workflow.export.as_mut().expect("Matching export");
        match (response.result.expect("Successful response"), &job.stage) {
            (
                Reply::ExportPrepared {
                    ticket,
                    address,
                    exists,
                },
                Stage::Preparing(_),
            ) => {
                job.stage = Stage::Confirm {
                    ticket,
                    path: address,
                };
                if !exists
                    || job.cancel
                    || job.revision != self.document.revision()
                    || job.origin != self.path
                {
                    self.confirm_export();
                } else {
                    self.workflow.status =
                        Some("Confirm replacement of the existing output.".into());
                }
            }
            (Reply::Address(address), Stage::Committing(_)) => {
                self.workflow.status = Some(
                    if job.revision == self.document.revision() && job.origin == self.path {
                        format!("Exported {address}")
                    } else {
                        format!(
                            "Exported submitted revision to {address}; document has newer changes."
                        )
                    },
                );
                self.workflow.export = None;
            }
            (Reply::Unit, Stage::Cancelling(_)) => {
                self.workflow.export = None;
                self.workflow.status =
                    Some("Export cancelled: source changed or operation was dismissed.".into());
            }
            _ => self.export_error("Unexpected export service response"),
        }
    }
}
