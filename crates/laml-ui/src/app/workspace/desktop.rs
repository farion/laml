//! Native-only settings. Network/file operations never block the UI thread.
use super::*;
use laml::update::{self, Application, UpdateInfo};
use std::sync::mpsc::{self, Receiver};

enum Reply {
    Checked(UpdateInfo),
    Updated,
    Installed,
}
#[derive(Default)]
pub(super) struct State {
    task: Option<Receiver<Result<Reply, String>>>,
    update: Option<UpdateInfo>,
    updated: bool,
    installed: bool,
    error: Option<String>,
    replacing: bool,
}
impl State {
    fn start(
        &mut self,
        ctx: &egui::Context,
        action: impl FnOnce() -> Result<Reply, String> + Send + 'static,
    ) {
        if self.task.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.task = Some(rx);
        self.error = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(action());
            ctx.request_repaint();
        });
    }
    fn poll(&mut self) {
        let Some(rx) = &self.task else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Installation worker stopped unexpectedly".into()),
        };
        self.task = None;
        match result {
            Ok(Reply::Checked(info)) => self.update = Some(info),
            Ok(Reply::Updated) => self.updated = true,
            Ok(Reply::Installed) => {
                self.installed = true;
                self.replacing = false;
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn status(&self, ui: &mut egui::Ui) {
        if self.task.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(crate::i18n::tr("Working…"));
            });
        }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
    pub(super) fn updates(&mut self, ui: &mut egui::Ui) {
        self.poll();
        ui.heading(crate::i18n::tr("Updates"));
        ui.add_space(16.0);
        ui.label(format!("LAML Editor {}", env!("CARGO_PKG_VERSION")));
        ui.label(crate::i18n::tr(
            "Updates are downloaded only when you choose to install them.",
        ));
        if self.updated {
            ui.label(crate::i18n::tr(
                "Update prepared. Close LAML Editor and start it again to use the new version.",
            ));
        } else {
            let busy = self.task.is_some();
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(crate::i18n::tr("Check for updates")),
                )
                .clicked()
            {
                self.start(ui.ctx(), || {
                    update::check_update(Application::Editor).map(Reply::Checked)
                });
            }
            if let Some(info) = self.update.clone() {
                if info.available {
                    ui.label(format!(
                        "{} {}",
                        crate::i18n::tr("Available version:"),
                        info.tag
                    ));
                    if ui
                        .add_enabled(!busy, egui::Button::new(crate::i18n::tr("Install update")))
                        .clicked()
                    {
                        self.start(ui.ctx(), move || {
                            update::apply_update(Application::Editor, &info.tag)?;
                            Ok(Reply::Updated)
                        });
                    }
                } else {
                    ui.label(crate::i18n::tr("You are up to date."));
                }
            }
        }
        ui.add_space(8.0);
        self.status(ui);
    }
    pub(super) fn cli(&mut self, ui: &mut egui::Ui) {
        self.poll();
        ui.heading("CLI");
        ui.add_space(16.0);
        ui.label(crate::i18n::tr(
            "Install the matching LAML CLI for your user account.",
        ));
        let path = match update::cli_install_path() {
            Ok(path) => path,
            Err(error) => {
                ui.label(error);
                return;
            }
        };
        ui.label(crate::i18n::tr("Installation path:"));
        ui.add(
            egui::Label::new(path.display().to_string())
                .wrap()
                .selectable(true),
        );
        let exists = path.symlink_metadata().is_ok();
        ui.label(crate::i18n::tr(if exists {
            "A file already exists at this path."
        } else {
            "CLI is not installed at this path."
        }));
        if self.installed {
            ui.label(crate::i18n::tr("CLI installed successfully."));
        }
        let parent = path.parent().unwrap();
        if !std::env::var_os("PATH")
            .is_some_and(|value| std::env::split_paths(&value).any(|p| p == parent))
        {
            ui.label(crate::i18n::tr(
                "Add this directory to your PATH to run laml from a terminal:",
            ));
            ui.add(
                egui::Label::new(parent.display().to_string())
                    .wrap()
                    .selectable(true),
            );
        }
        let busy = self.task.is_some();
        let label = if exists {
            "Replace CLI…"
        } else {
            "Install CLI"
        };
        if ui
            .add_enabled(!busy, egui::Button::new(crate::i18n::tr(label)))
            .clicked()
        {
            if exists {
                self.replacing = true;
            } else {
                self.start(ui.ctx(), move || {
                    update::install_cli(&path, false)?;
                    Ok(Reply::Installed)
                });
            }
        }
        if self.replacing {
            ui.label(crate::i18n::tr(
                "Replace the existing file with the CLI matching this editor version?",
            ));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!busy, egui::Button::new(crate::i18n::tr("Replace CLI")))
                    .clicked()
                {
                    self.start(ui.ctx(), move || {
                        update::install_cli(&update::cli_install_path()?, true)?;
                        Ok(Reply::Installed)
                    });
                }
                if ui.button(crate::i18n::tr("Cancel")).clicked() {
                    self.replacing = false;
                }
            });
        }
        ui.add_space(8.0);
        self.status(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_do_not_start_downloads_when_opened() {
        let ctx = egui::Context::default();
        let mut state = State::default();
        let mut preferences = editor_theme::Preferences::default();
        for mut category in [settings::Category::Updates, settings::Category::Cli] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_width(600.0);
                settings::content(ui, &mut category, &mut preferences, &mut state);
            });
            output.textures_delta.clear();
            assert!(state.task.is_none());
            assert!(!state.updated && !state.installed);
        }
    }
    #[test]
    fn completed_tasks_are_polled_without_mutating_documents() {
        let (tx, rx) = mpsc::channel();
        let mut state = State {
            task: Some(rx),
            ..Default::default()
        };
        tx.send(Ok(Reply::Checked(UpdateInfo {
            tag: "v9.0.0".into(),
            available: true,
        })))
        .unwrap();
        state.poll();
        assert!(state.update.as_ref().unwrap().available);
        assert!(state.task.is_none());
        assert!(!state.updated);
        let (tx, rx) = mpsc::channel();
        state.task = Some(rx);
        tx.send(Err("checksum mismatch".into())).unwrap();
        state.poll();
        assert_eq!(state.error.as_deref(), Some("checksum mismatch"));
        assert!(!state.updated);
    }
}
