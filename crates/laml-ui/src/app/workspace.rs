//! Application-level chrome and independent, long-lived document tabs.
use super::*;
use egui_phosphor::regular::{FOLDER_OPEN, GEAR_SIX, PLUS, QUESTION, X};

mod io;
#[cfg(target_arch = "wasm32")]
mod library;
mod settings;
mod new_diagram;
mod recovery;
mod welcome;
mod help;
#[cfg(test)]
mod tests;

pub(super) enum Action {
    New,
    Open(PathBuf),
}

pub struct Workspace {
    #[cfg(target_arch = "wasm32")]
    library: library::Library,
    backend: laml_ui::services::Services,
    service: laml_ui::backend::async_client::Client,
    opening: Option<laml_ui::backend::protocol::Stamp>,
    pub(super) tabs: Vec<Editor>,
    pub(super) preferences: editor_theme::Preferences,
    pub(super) editor_theme: egui::Theme,
    active: usize,
    next_untitled: usize,
    settings_open: bool,
    welcome_open: bool,
    help_open: bool,
    recovery: recovery::State,
    new_diagram: Option<new_diagram::Picker>,
    settings_category: settings::Category,
    reveal_active: bool,
    closing_all: bool,
    waiting_close: Option<u64>,
    allow_close: bool,
    error: Option<String>,
}

impl Workspace {
    pub fn new(mut editor: Editor) -> Self {
        editor.managed = true;
        let next_untitled = if editor.file.path.is_none() { 2 } else { 1 };
        let preferences = editor_theme::Preferences::load();
        Self {
            #[cfg(target_arch = "wasm32")]
            library: Default::default(),
            backend: editor.backend.clone(),
            service: editor.backend.client(),
            opening: None,
            tabs: vec![editor],
            preferences,
            welcome_open: !preferences.welcome_seen,
            help_open: false,
            editor_theme: egui::Theme::Dark,
            active: 0,
            next_untitled,
            settings_open: false,
            recovery: Default::default(),
            new_diagram: None,
            settings_category: settings::Category::default(),
            reveal_active: true,
            closing_all: false,
            waiting_close: None,
            allow_close: false,
            error: None,
        }
    }

    fn add_tab(&mut self, mut editor: Editor) {
        editor.managed = true;
        editor.recovery.records.clear();
        self.tabs.push(editor);
        self.active = self.tabs.len() - 1;
        self.reveal_active = true;
        self.sync_paths();
    }

    fn sync_paths(&mut self) {
        let paths: Vec<_> = self
            .tabs
            .iter()
            .flat_map(|tab| {
                tab.file
                    .path
                    .iter()
                    .chain(tab.file_ui.reserved_path())
                    .cloned()
                    .map(|path| (tab.id, path))
            })
            .collect();
        for tab in &mut self.tabs {
            tab.other_paths = paths
                .iter()
                .filter(|(id, _)| *id != tab.id)
                .map(|(_, path)| path.clone())
                .collect();
        }
    }

    fn new_tab(&mut self) {
        let mut editor =
            Editor::with_services(editor_theme::new_source(self.editor_theme), None, self.backend.clone());
        editor.path = PathBuf::from(if self.next_untitled == 1 {
            "untitled.laml".into()
        } else {
            format!("untitled-{}.laml", self.next_untitled)
        });
        self.next_untitled += 1;
        self.add_tab(editor);
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() { self.library_placeholder(); }
    }

    fn request_close(&mut self, index: usize, ctx: &egui::Context) {
        if self.waiting_close.is_some() {
            return;
        }
        self.active = index;
        self.reveal_active = true;
        #[cfg(target_arch = "wasm32")]
        if laml_ui::services::browser_mode() {
            if !crate::browser_storage::safe_to_close() {
                self.error = Some(crate::browser_storage::storage_status()); return;
            }
            self.tabs[index].allow_close = true;
            self.waiting_close = Some(self.tabs[index].id);
            self.finish_close(ctx); return;
        }
        let editor = &mut self.tabs[index];
        self.waiting_close = Some(editor.id);
        editor.request_tab_close(ctx);
        self.finish_close(ctx);
    }

    fn finish_close(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.waiting_close {
            let index = self.tabs.iter().position(|tab| tab.id == id).unwrap();
            if self.tabs[index].close_cancelled {
                self.tabs[index].close_cancelled = false;
                self.waiting_close = None;
                self.closing_all = false;
                return;
            }
            if !self.tabs[index].allow_close {
                return;
            }
            self.tabs.remove(index);
            self.sync_paths();
            self.active = index.min(self.tabs.len().saturating_sub(1));
            self.reveal_active = true;
            self.waiting_close = None;
        }
        if self.tabs.is_empty() {
            if self.closing_all {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.new_tab();
            }
        }
    }

    fn close_all_tick(&mut self, ctx: &egui::Context) {
        self.finish_close(ctx);
        if self.closing_all && !self.allow_close && self.waiting_close.is_none() {
            self.request_close(self.active, ctx);
        }
    }

    fn chrome(&mut self, ui: &mut egui::Ui) {
        let locked = self.welcome_open || self.opening.is_some()
            || self.waiting_close.is_some()
            || self
                .tabs
                .get(self.active)
                .is_some_and(Editor::close_dialog_active);
        let mut action = None;
        egui::Panel::top("global-title")
            .exact_size(44.0)
            .show_separator_line(false)
            .frame(
                appearance::panel_frame(appearance::HEADER)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "LAML Editor",
                    egui::FontId::proportional(16.0),
                    appearance::TEXT,
                );
                ui.horizontal_centered(|ui| {
                    ui.add_enabled_ui(!locked, |ui| {
                        ui.horizontal(|ui| {
                            if !laml_ui::services::browser_mode() { self.recovery_button(ui); }
                            #[cfg(target_arch = "wasm32")]
                            if laml_ui::services::browser_mode() && icon(ui, egui_phosphor::regular::BOOKS, "Browser library").clicked() { self.library.open = true; }
                            if icon(ui, PLUS, "New LAML (Ctrl/Cmd+N)").clicked() {
                                action = Some(Action::New);
                            }
                            if icon(ui, FOLDER_OPEN, "Open LAML (Ctrl/Cmd+O)").clicked() {
                                self.tabs[self.active].open_tab(ui.ctx());
                            }
                            ui.separator();
                            self.tabs[self.active].document_title_actions(ui);
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon(ui, QUESTION, "Help").clicked() {
                            self.help_open = true;
                        }
                        if icon(ui, GEAR_SIX, "Settings").clicked() {
                            self.settings_open = true;
                        }
                        ui.add_enabled_ui(!locked, |ui| {
                            self.tabs[self.active].export_controls(ui);
                        });
                    });
                });
            });
        let mut selected = None;
        let mut closed = None;
        let reveal = std::mem::take(&mut self.reveal_active);
        egui::Panel::top("document-tabs")
            .exact_size(38.0)
            .frame(appearance::panel_frame(appearance::HEADER))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.add_enabled_ui(!locked, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("open-files")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.0;
                                for (index, editor) in self.tabs.iter().enumerate() {
                                    ui.push_id(editor.id, |ui| {
                                        let active = index == self.active;
                                        let response = egui::Frame::new()
                                            .fill(if active {
                                                appearance::PANEL
                                            } else {
                                                appearance::HEADER
                                            })
                                            .inner_margin(egui::Margin::symmetric(8, 2))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 6.0;
                                                    let name = editor
                                                        .path
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy();
                                                    let label = format!(
                                                        "{name}{}{}",
                                                        if editor.recovery.recovered && editor.file.path.is_none() { " (recovered)" } else { "" },
                                                        if !laml_ui::services::browser_mode() && editor.document.is_dirty() {
                                                            " •"
                                                        } else {
                                                            ""
                                                        }
                                                    );
                                                    if ui
                                                        .add(egui::Button::new(label).frame(false))
                                                        .on_hover_text(
                                                            editor.path.display().to_string(),
                                                        )
                                                        .clicked()
                                                    {
                                                        selected = Some(index);
                                                    }
                                                    if icon(ui, X, "Close file").clicked() {
                                                        closed = Some(index);
                                                    }
                                                });
                                            });
                                        if active {
                                            if reveal {
                                                response
                                                    .response
                                                    .scroll_to_me(Some(egui::Align::Center));
                                            }
                                            ui.painter().hline(
                                                response.response.rect.x_range(),
                                                response.response.rect.bottom() - 1.0,
                                                egui::Stroke::new(2.0, appearance::ACCENT),
                                            );
                                        }
                                    });
                                }
                            });
                        });
                });
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    egui::Stroke::new(1.0, appearance::BORDER),
                );
            });
        if let Some(index) = selected {
            self.active = index;
            self.reveal_active = true;
        }
        if let Some(index) = closed {
            self.request_close(index, ui.ctx());
        }
        if let Some(Action::New) = action {
            self.new_diagram = Some(Default::default());
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.welcome_open || self.help_open || self.settings_open || dialogs::blocked(ctx) {
            return;
        }
        if self.opening.is_some()
            || self.waiting_close.is_some()
            || self.tabs[self.active].close_dialog_active()
        {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)) {
            self.request_close(self.active, ctx);
        }
        let previous = ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::Tab,
            )
        });
        let next = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab));
        if previous || next {
            self.reveal_active = true;
        }
        if previous {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        } else if next {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    fn windows(&mut self, ctx: &egui::Context) {
        self.new_diagram_window(ctx);
        if self.settings_open {
            let dismissed = dialogs::show(ctx, "global-settings", "Settings", 640.0, |ui| {
                let before = self.preferences;
                settings::content(ui, &mut self.settings_category, &mut self.preferences);
                if before != self.preferences {
                    if let Err(error) = self.preferences.save() {
                        self.error = Some(format!("Could not save editor settings: {error}"));
                    }
                    self.sync_editor_theme(ctx);
                }
            });
            if dismissed {
                self.settings_open = false;
            }
        }
        if let Some(error) = self.error.clone() {
            let dismissed =
                dialogs::show(ctx, "open-file-error", "Could not open file", 420.0, |ui| {
                    ui.label(error);
                    if ui.button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            if dismissed {
                self.error = None;
            }
        }
    }

    fn tick(&mut self, ui: &mut egui::Ui) {
        #[cfg(target_arch = "wasm32")]
        self.library_tick();
        self.sync_editor_theme(ui.ctx());
        #[cfg(target_arch = "wasm32")]
        crate::web::set_dirty(self.tabs.iter().any(|e| e.document.is_dirty()));
        #[cfg(target_arch = "wasm32")]
        if let Some(tab) = self.tabs.get(self.active) {
            crate::web::set_document(
                tab.path
                    .file_name()
                    .and_then(|p| p.to_str())
                    .unwrap_or("Untitled"),
                tab.rendered == Some(tab.document.revision()) && tab.view.image_ready,
                if laml_ui::services::browser_mode() { !crate::browser_storage::safe_to_close() } else { tab.document.is_dirty() },
                tab.document.revision(),
                self.recovery_count(),
            );
        }
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.closing_all = true;
            if let Some(stamp) = self.opening.take() {
                self.service.forget(stamp);
            }
        }
        self.poll_open();
        self.close_all_tick(&ctx);
        if self.allow_close {
            return;
        }
        self.sync_paths();
        self.chrome(ui);
        self.shortcuts(&ctx);
        self.sync_editor_theme(&ctx);
        for (index, editor) in self.tabs.iter_mut().enumerate() {
            if index != self.active {
                editor.workflow_tick();
                editor.compile(&ctx);
                editor.recovery_tick();
                editor.poll_file(&ctx);
            }
        }
        let editor = &mut self.tabs[self.active];
        ui.push_id(editor.id, |ui| {
            // The document owns its text cursor, inspector, view, dialogs and worker.
            editor.document_ui(ui);
        });
        if let Some(action) = self.tabs[self.active].workspace_action.take() {
            match action {
                Action::New => self.new_diagram = Some(Default::default()),
                Action::Open(path) => self.open_path(path),
            }
        }
        self.finish_close(&ctx);
        if !laml_ui::services::browser_mode() { self.recovery_tick(); }
        #[cfg(target_arch = "wasm32")]
        self.library_tick();
        if self.welcome_open && !self.closing_all {
            self.welcome_window(&ctx);
        } else if self.help_open && !self.closing_all {
            self.help_window(&ctx);
        } else {
            self.windows(&ctx);
            if !laml_ui::services::browser_mode() { self.recovery_window(&ctx); }
            #[cfg(target_arch = "wasm32")]
            self.library_window(&ctx);
        }
    }
}

pub(super) fn icon(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    ui.add_sized(
        [32.0, 32.0],
        egui::Button::new(egui::RichText::new(glyph).font(egui::FontId::new(
            19.0,
            egui::FontFamily::Name("phosphor".into()),
        )))
        .frame_when_inactive(false),
    )
    .on_hover_text(tooltip)
}

impl eframe::App for Workspace {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.tick(ui);
    }
}
