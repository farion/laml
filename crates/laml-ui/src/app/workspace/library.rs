//! Browser-only document library. Source, history and editing remain in Editor.
use super::*;
#[path = "library_confirm.rs"]
mod confirm;
#[path = "library_view.rs"]
mod view;
use crate::browser_storage as storage;
use std::collections::BTreeMap;
use view::Command;
#[derive(Clone, serde::Deserialize)]
struct Record {
    id: String,
    name: String,
    source: String,
    trashed: bool,
}
#[derive(Default, serde::Deserialize)]
struct Snapshot {
    records: Vec<Record>,
    notice: String,
    error: String,
}
#[derive(serde::Deserialize)]
struct Imported {
    name: String,
    source: String,
}
#[derive(serde::Deserialize)]
struct Completed {
    action: String,
    id: String,
    name: String,
}
struct Link {
    id: String,
    revision: u64,
    name: String,
    persisted: bool,
}
#[derive(Default)]
pub(super) struct Library {
    links: BTreeMap<u64, Link>,
    pub open: bool,
    trash: bool,
    query: String,
    selected: Option<String>,
    rename: Option<(String, String)>,
    confirm: Option<(String, String)>,
    remembered: String,
}
impl Workspace {
    pub(super) fn library_has_name(&self, name: &str) -> bool {
        serde_json::from_str::<Snapshot>(&storage::snapshot())
            .unwrap_or_default()
            .records
            .iter()
            .any(|record| record.name == name)
    }

    pub(crate) fn load_browser_tabs(&mut self) {
        let ids: Vec<String> = serde_json::from_str(&storage::startup_tabs()).unwrap_or_default();
        self.tabs.clear();
        self.library.links.clear();
        self.active = 0;
        for id in ids {
            if let Ok(record) = serde_json::from_str::<Record>(&storage::open_record(&id)) {
                self.open_library_record(record);
            }
        }
        self.library_tick();
    }
    fn open_library_record(&mut self, record: Record) {
        if let Some(index) = self.tabs.iter().position(|tab| {
            self.library
                .links
                .get(&tab.id)
                .is_some_and(|link| link.id == record.id)
        }) {
            self.active = index;
            self.reveal_active = true;
            return;
        }
        let mut editor = Editor::with_services(record.source, None, self.backend.clone());
        editor.path = record.name.clone().into();
        self.library.links.insert(
            editor.id,
            Link {
                id: record.id,
                revision: editor.document.revision(),
                name: record.name,
                persisted: true,
            },
        );
        self.add_tab(editor);
    }
    pub(super) fn library_tick(&mut self) {
        if !laml_ui::services::browser_mode() {
            return;
        }
        for action in
            serde_json::from_str::<Vec<Completed>>(&storage::take_actions()).unwrap_or_default()
        {
            if action.action == "rename" {
                for tab in &mut self.tabs {
                    if let Some(link) = self
                        .library
                        .links
                        .get_mut(&tab.id)
                        .filter(|l| l.id == action.id)
                    {
                        link.name = action.name.clone();
                        tab.path = action.name.clone().into();
                    }
                }
            } else if matches!(action.action.as_str(), "trash" | "delete" | "clear") {
                self.tabs.retain(|tab| {
                    !self
                        .library
                        .links
                        .get(&tab.id)
                        .is_some_and(|l| action.action == "clear" || l.id == action.id)
                });
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
        }
        for item in
            serde_json::from_str::<Vec<Imported>>(&storage::take_imports()).unwrap_or_default()
        {
            self.open_library_record(Record {
                id: storage::new_id(),
                name: item.name,
                source: item.source,
                trashed: false,
            });
            if let Some(tab) = self.tabs.get(self.active) {
                if let Some(link) = self.library.links.get_mut(&tab.id) {
                    link.revision = u64::MAX;
                }
            }
        }
        for tab in &self.tabs {
            let link = self.library.links.entry(tab.id).or_insert_with(|| Link {
                id: storage::new_id(),
                revision: u64::MAX,
                persisted: false,
                name: tab
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            });
            if link.revision != tab.document.revision() {
                storage::save_document(&link.id, &link.name, tab.document.source());
                link.revision = tab.document.revision();
                link.persisted = true;
            }
        }
        self.library
            .links
            .retain(|key, _| self.tabs.iter().any(|t| t.id == *key));
        let ids: Vec<_> = self
            .tabs
            .iter()
            .filter_map(|t| {
                self.library
                    .links
                    .get(&t.id)
                    .filter(|l| l.persisted)
                    .map(|l| &l.id)
            })
            .collect();
        let json = serde_json::to_string(&ids).unwrap_or_default();
        if self.library.remembered != json {
            storage::remember_tabs(&json);
            self.library.remembered = json;
        }
    }
    pub(super) fn library_window(&mut self, ctx: &egui::Context) {
        if !self.library.open {
            return;
        }
        let snapshot: Snapshot = serde_json::from_str(&storage::snapshot()).unwrap_or_default();
        let safe = storage::safe_to_close();
        let blocked = self.library.confirm.is_some() || self.library.rename.is_some();
        let mut command = None;
        let dismissed = dialogs::show(ctx, "browser-library", "Browser library", 760.0, |ui| {
            ui.add_enabled_ui(!blocked, |ui| {
                if !snapshot.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, crate::i18n::message(&(&snapshot.error))); }
                if !snapshot.notice.is_empty() { ui.label(crate::i18n::message(&(&snapshot.notice))); }
                command = self.library.manager(ui, &snapshot, safe);
                ui.add_space(8.0);
                ui.add(egui::Label::new(egui::RichText::new(
                    crate::i18n::tr("Stored only in this browser. Clearing browser data deletes these files. Download backups.")
                ).small().weak()).wrap());
            });
        });
        if let Some(command) = command {
            match command {
                Command::Open(id) => {
                    if let Ok(record) = serde_json::from_str(&storage::open_record(&id)) {
                        self.open_library_record(record);
                        self.library.open = false;
                    }
                }
                Command::Import => storage::import_laml(),
                Command::Rename(id, name) => self.library.rename = Some((id, name)),
                Command::Download(id) => {
                    if let Some(record) = snapshot.records.iter().find(|r| r.id == id) {
                        storage::download(
                            &record.name,
                            record.source.as_bytes(),
                            "text/plain;charset=utf-8",
                        );
                    }
                }
                Command::Trash(id) => storage::library_action("trash", &id, ""),
                Command::Restore(id) => storage::library_action("restore", &id, ""),
                Command::Delete(id) => self.library.confirm = Some(("delete".into(), id)),
                Command::Empty => self.library.confirm = Some(("empty".into(), String::new())),
                Command::Clear => self.library.confirm = Some(("clear".into(), String::new())),
            }
        }
        if dismissed && !blocked {
            self.library.open = false;
        }
        if self.library.open {
            self.library_confirmation(ctx, &snapshot, safe);
            self.library_rename(ctx, safe);
        }
    }
    fn library_rename(&mut self, ctx: &egui::Context, safe: bool) {
        let Some((id, mut name)) = self.library.rename.take() else {
            return;
        };
        let mut accepted = false;
        let mut cancelled = false;
        let dismissed = dialogs::show(ctx, "browser-library-rename", "Rename file", 380.0, |ui| {
            ui.label(crate::i18n::tr("File name"));
            let response = ui.text_edit_singleline(&mut name);
            let valid = !name.trim().is_empty()
                && !name.contains(['/', '\\'])
                && !name.chars().any(char::is_control)
                && !matches!(name.trim(), "." | "..");
            if !valid {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    crate::i18n::tr("Enter a file name, not a path."),
                );
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                accepted = ui
                    .add_enabled(safe && valid, egui::Button::new(crate::i18n::tr("Rename")))
                    .clicked()
                    || (safe
                        && valid
                        && response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                cancelled = ui.button(crate::i18n::tr("Cancel")).clicked();
            });
        });
        if accepted {
            let mut name = name.trim().to_string();
            if !name.to_ascii_lowercase().ends_with(".laml") {
                name.push_str(".laml");
            }
            storage::library_action("rename", &id, &name);
        } else if !cancelled && !dismissed {
            self.library.rename = Some((id, name));
        }
    }
}
