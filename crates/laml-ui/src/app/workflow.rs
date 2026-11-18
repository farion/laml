//! Asynchronous dependency watching and backend-owned export tickets.
use super::*;
use laml_ui::{
    backend::{
        async_client::Client,
        protocol::{self, Operation, Reply, Stamp},
    },
    output::Format,
};
#[cfg(test)]
mod async_tests;
mod io;
#[cfg(test)]
mod tests;

enum Stage {
    Preparing(Stamp),
    Confirm { ticket: u64, path: String },
    Committing(Stamp),
    Cancelling(Stamp),
}
struct Export {
    revision: u64,
    origin: PathBuf,
    stage: Stage,
    cancel: bool,
}
pub(super) struct Workflow {
    service: Client,
    document: u64,
    watch: Option<u64>,
    watch_request: Option<Stamp>,
    poll_request: Option<Stamp>,
    revision: Option<u64>,
    path: PathBuf,
    checked: Instant,
    retry_after: Option<Instant>,
    export: Option<Export>,
    picker: Option<(Format, host_dialogs::Picker, egui::Context)>,
    dismiss_picker: bool,
    export_open: bool,
    export_format: Format,
    pub status: Option<String>,
}
impl Workflow {
    pub fn new(document: u64, backend: laml_ui::services::Services) -> Self {
        Self {
            service: backend.client(),
            document,
            watch: None,
            watch_request: None,
            poll_request: None,
            revision: None,
            path: PathBuf::new(),
            checked: Instant::now(),
            retry_after: None,
            export: None,
            picker: None,
            dismiss_picker: false,
            export_open: false,
            export_format: Format::Svg,
            status: None,
        }
    }
    pub fn invalidate_watch(&mut self) {
        for stamp in [self.watch_request.take(), self.poll_request.take()]
            .into_iter()
            .flatten()
        {
            self.service.forget(stamp);
        }
        self.watch = None;
        self.revision = None;
    }
    pub fn busy(&self) -> bool {
        self.export.is_some() || self.picker.is_some()
    }
    pub fn processing(&self) -> bool {
        self.export.as_ref().is_some_and(|job| {
            matches!(
                job.stage,
                Stage::Preparing(_) | Stage::Committing(_) | Stage::Cancelling(_)
            )
        })
    }
    pub fn writing(&self) -> bool {
        self.export
            .as_ref()
            .is_some_and(|job| matches!(job.stage, Stage::Committing(_)))
    }
    pub fn cancel_export(&mut self) {
        if self.picker.is_some() {
            self.dismiss_picker = true;
        }
        if let Some(export) = &mut self.export {
            export.cancel = true;
        }
    }
}
impl Editor {
    pub(super) fn export_error(&mut self, error: impl ToString) {
        let message = format!("Export failed: {}", error.to_string());
        self.workflow.export = None;
        self.fail_pending_file_action(&message);
        self.workflow.status = Some(message);
    }
    pub(super) fn start_export(&mut self, path: PathBuf, format: Format) {
        if self.workflow.busy() {
            return;
        }
        if !self.can_edit_visually() || self.file_ui.busy() {
            self.workflow.status = Some(
                "Build valid current source and finish file operations before exporting.".into(),
            );
            return;
        }
        let operation = protocol::address(&path).and_then(|address| {
            if laml_ui::services::browser_mode() {
                return Ok(Operation::RenderExport {
                    source: self.document.source().into(),
                    address: protocol::address(&self.path)?,
                    format,
                });
            }
            Ok(Operation::BeginExport {
                address,
                source_address: protocol::address(&self.path)?,
                source: self.document.source().into(),
                format,
            })
        });
        let result = operation.and_then(|op| {
            self.workflow
                .service
                .submit(self.id, self.document.revision(), op)
        });
        match result {
            Ok(stamp) => {
                self.workflow.export_format = format;
                self.workflow.export = Some(Export {
                    revision: self.document.revision(),
                    origin: self.path.clone(),
                    stage: Stage::Preparing(stamp),
                    cancel: false,
                });
                self.workflow.status = Some("Preparing export…".into());
            }
            Err(error) => self.export_error(error),
        }
    }
    pub(super) fn open_export_dialog(&mut self) {
        self.workflow.export_open = true;
    }
    #[cfg(test)]
    pub(super) fn export_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(
            self.can_edit_visually() && !self.workflow.busy() && !self.file_ui.busy(),
            |ui| {
                if workspace::icon(ui, egui_phosphor::regular::EXPORT, "Export…").clicked() {
                    self.workflow.export_open = true;
                }
            },
        );
    }
    pub(super) fn export_windows(&mut self, ctx: &egui::Context) {
        if self.workflow.export_open {
            let dismissed =
                dialogs::show(ctx, ("export-options", self.id), "Export", 360.0, |ui| {
                    ui.label(crate::i18n::tr("Format"));
                    for (format, label) in [
                        (Format::Svg, "SVG · Vector image"),
                        (Format::Png, "PNG · Raster image"),
                        (Format::Pdf, "PDF · Vector document"),
                    ] {
                        if ui
                            .radio(
                                self.workflow.export_format.extension() == format.extension(),
                                label,
                            )
                            .clicked()
                        {
                            self.workflow.export_format = format;
                        }
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let enabled = self.can_edit_visually()
                            && !self.workflow.busy()
                            && !self.file_ui.busy();
                        if ui
                            .add_enabled(enabled, egui::Button::new(crate::i18n::tr("Export…")))
                            .clicked()
                        {
                            let format = self.workflow.export_format;
                            let name = self.path.file_stem().unwrap_or_default().to_string_lossy();
                            if laml_ui::services::browser_mode() {
                                self.start_export(
                                    PathBuf::from(format!("{name}.{}", format.extension())),
                                    format,
                                );
                            } else {
                                self.workflow.picker = Some((
                                    format,
                                    host_dialogs::Picker::save(
                                        format!("{name}.{}", format.extension()),
                                        format.extension(),
                                    ),
                                    ctx.clone(),
                                ));
                            }
                            self.workflow.export_open = false;
                        }
                        if ui.button(crate::i18n::tr("Cancel")).clicked() {
                            self.workflow.export_open = false;
                        }
                    });
                });
            if dismissed {
                self.workflow.export_open = false;
            }
        }
        let path = self
            .workflow
            .export
            .as_ref()
            .and_then(|job| match &job.stage {
                Stage::Confirm { path, .. } => Some(path.clone()),
                _ => None,
            });
        if let Some(path) = path {
            let dismissed = dialogs::show(
                ctx,
                ("export-overwrite", self.id),
                "Replace existing output?",
                440.0,
                |ui| {
                    ui.label(path);
                    ui.small(crate::i18n::tr(
                        "The checked output snapshot will be revalidated before writing.",
                    ));
                    if ui.button(crate::i18n::tr("Replace this output")).clicked() {
                        self.confirm_export();
                    }
                    if ui.button(crate::i18n::tr("Cancel")).clicked() {
                        self.workflow.cancel_export();
                    }
                },
            );
            if dismissed {
                self.workflow.cancel_export();
            }
        }
    }
    pub(super) fn confirm_export(&mut self) {
        let Some(job) = &self.workflow.export else {
            return;
        };
        let Stage::Confirm { ticket, .. } = job.stage else {
            return;
        };
        let current =
            job.revision == self.document.revision() && job.origin == self.path && !job.cancel;
        let operation = if current {
            Operation::CommitExport { ticket }
        } else {
            Operation::CancelExport { ticket }
        };
        match self
            .workflow
            .service
            .submit(self.id, job.revision, operation)
        {
            Ok(stamp) => {
                let job = self.workflow.export.as_mut().expect("Active export");
                job.stage = if current {
                    Stage::Committing(stamp)
                } else {
                    Stage::Cancelling(stamp)
                };
                self.workflow.status = Some(
                    if current {
                        "Writing checked export…"
                    } else {
                        "Cancelling outdated export…"
                    }
                    .into(),
                );
            }
            Err(error) => self.export_error(error),
        }
    }
}
