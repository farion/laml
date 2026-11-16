use super::*;

impl Editor {
    pub(in crate::app) fn file_windows(&mut self, ctx: &egui::Context) {
        if self.file_ui.busy() && !(cfg!(target_arch = "wasm32") && self.file_ui.picker.is_some()) {
            egui::Window::new(crate::i18n::tr("File operation"))
                .frame(dialogs::frame(ctx))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(if self.file_ui.writing() {
                            crate::i18n::tr("Saving submitted revision…")
                        } else {
                            crate::i18n::tr("Waiting for file operation…")
                        });
                    });
                    if !self.file_ui.writing()
                        && self.file_ui.picker.is_none()
                        && ui.button(crate::i18n::tr("Cancel")).clicked()
                    {
                        self.cancel_file_read();
                        self.file_ui.after_save = None;
                        self.file_ui.deferred = None;
                        self.close_cancelled = true;
                    }
                });
        }
        if self.file_ui.conflict || self.file_ui.error.is_some() {
            egui::Window::new(crate::i18n::tr("File status"))
                .frame(dialogs::frame(ctx))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .show(ctx, |ui| {
                    if self.file_ui.conflict {
                        ui.label(crate::i18n::tr(
                            "File changed externally. Local text has not been overwritten.",
                        ));
                    }
                    if let Some(error) = &self.file_ui.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, crate::i18n::message(error));
                    }
                    if ui.button(crate::i18n::tr("Reload from disk…")).clicked() {
                        self.request_action(Action::Reload, ctx);
                    }
                    if ui.button(crate::i18n::tr("Save a copy…")).clicked() {
                        self.show_file_dialog(Dialog::SaveAs);
                    }
                    if !self.file_ui.conflict && ui.button(crate::i18n::tr("Dismiss")).clicked() {
                        self.file_ui.error = None;
                    }
                });
        }
        if let Some(action) = self.file_ui.pending.clone() {
            let dismissed =
                dialogs::show(ctx, ("unsaved", self.id), "Unsaved changes", 420.0, |ui| {
                    ui.label(crate::i18n::tr("Save your changes before continuing?"));
                    if ui.button(crate::i18n::tr("Keep editing")).clicked() {
                        self.file_ui.pending = None;
                        self.close_cancelled = true;
                    }
                    if ui.button(crate::i18n::tr("Save and continue")).clicked() {
                        self.file_ui.pending = None;
                        self.file_ui.after_save = Some(action.clone());
                        self.save_file(ctx);
                    }
                    if ui.button(crate::i18n::tr("Discard and continue")).clicked() {
                        self.file_ui.pending = None;
                        self.file_ui.after_save = None;
                        self.perform(action.clone(), ctx);
                    }
                });
            if dismissed {
                self.file_ui.pending = None;
                self.close_cancelled = true;
            }
        }
        if let Some(dialog) = self.file_ui.dialog {
            let title = match dialog {
                Dialog::Open => "Open LAML file",
                Dialog::SaveAs => "Save LAML as",
            };
            let dismissed = dialogs::show(ctx, ("file-path", self.id), title, 480.0, |ui| {
                ui.label(crate::i18n::tr("File path"));
                self.source_ui
                    .extra_drafts
                    .push(ui.text_edit_singleline(&mut self.file_ui.path).id);
                if let Dialog::SaveAs = dialog {
                    ui.small(crate::i18n::tr("Relative style/font imports are resolved from the destination folder; imported files are not copied."));
                }
                if ui.button(crate::i18n::tr("Confirm")).clicked() {
                    let path = PathBuf::from(self.file_ui.path.trim());
                    match dialog {
                        Dialog::Open => {
                            self.file_ui.dialog = None;
                            self.request_action(Action::Open(path), ctx);
                        }
                        Dialog::SaveAs => self.confirm_save_as(path, ctx),
                    }
                }
                if ui.button(crate::i18n::tr("Cancel")).clicked() {
                    self.file_ui.dialog = None;
                    self.file_ui.after_save = None;
                    self.file_ui.overwrite = None;
                    self.close_cancelled = true;
                }
            });
            if dismissed {
                self.file_ui.dialog = None;
                self.file_ui.after_save = None;
                self.close_cancelled = true;
            }
        }
        if let Some((path, existing, revision)) = self.file_ui.overwrite.clone() {
            let dismissed = dialogs::show(
                ctx,
                ("overwrite", self.id),
                "Replace existing file?",
                420.0,
                |ui| {
                    ui.label(path.display().to_string());
                    if ui.button(crate::i18n::tr("Replace this file")).clicked() {
                        self.file_ui.overwrite = None;
                        if revision == self.document.revision() {
                            self.save_as(path.clone(), Some(&existing), ctx);
                        } else {
                            self.fail_file(
                                "Save confirmation expired because the document changed.",
                            );
                        }
                    }
                    if ui.button(crate::i18n::tr("Cancel")).clicked() {
                        self.file_ui.overwrite = None;
                        self.file_ui.after_save = None;
                        self.close_cancelled = true;
                    }
                },
            );
            if dismissed {
                self.file_ui.overwrite = None;
                self.file_ui.after_save = None;
                self.close_cancelled = true;
            }
        }
    }
}
