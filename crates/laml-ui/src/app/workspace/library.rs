//! Browser-only document library. Source, history and editing remain in Editor.
use super::*;
use crate::browser_storage as storage;
use std::collections::BTreeMap;
#[derive(Clone, serde::Deserialize)]
struct Record { id: String, name: String, source: String, trashed: bool }
#[derive(Default, serde::Deserialize)]
struct Snapshot { records: Vec<Record>, notice: String, error: String }
#[derive(serde::Deserialize)]
struct Imported { name: String, source: String }
#[derive(serde::Deserialize)]
struct Completed { action: String, id: String, name: String }
struct Link { id: String, revision: u64, name: String, persisted: bool }
#[derive(Default)]
pub(super) struct Library {
    links: BTreeMap<u64, Link>,
    pub open: bool,
    trash: bool,
    rename: Option<(String, String)>,
    confirm: Option<(String, String)>,
    remembered: String,
}
impl Workspace {
    pub(crate) fn load_browser_tabs(&mut self) {
        let ids: Vec<String> = serde_json::from_str(&storage::startup_tabs()).unwrap_or_default();
        let mut restored = false;
        for id in ids {
            if let Ok(record) = serde_json::from_str::<Record>(&storage::open_record(&id)) {
                if !restored { self.tabs.clear(); restored = true; }
                self.open_library_record(record);
            }
        }
        if !restored { self.library_placeholder(); }
        self.library_tick();
    }
    pub(super) fn library_placeholder(&mut self) {
        if let Some(tab) = self.tabs.get(self.active) {
            self.library.links.insert(tab.id, Link { id: storage::new_id(), revision: tab.document.revision(),
                name: tab.path.to_string_lossy().into_owned(), persisted: false });
        }
    }
    fn open_library_record(&mut self, record: Record) {
        if let Some(index) = self.tabs.iter().position(|tab|
            self.library.links.get(&tab.id).is_some_and(|link| link.id == record.id)) {
            self.active = index; self.reveal_active = true; return;
        }
        let mut editor = Editor::with_services(record.source, None, self.backend.clone());
        editor.path = record.name.clone().into();
        self.library.links.insert(editor.id, Link { id: record.id, revision: editor.document.revision(), name: record.name, persisted: true });
        self.add_tab(editor);
    }
    pub(super) fn library_tick(&mut self) {
        if !laml_ui::services::browser_mode() { return; }
        for action in serde_json::from_str::<Vec<Completed>>(&storage::take_actions()).unwrap_or_default() {
            if action.action == "rename" {
                for tab in &mut self.tabs {
                    if let Some(link) = self.library.links.get_mut(&tab.id).filter(|l| l.id == action.id) {
                        link.name = action.name.clone(); tab.path = action.name.clone().into();
                    }
                }
            } else if matches!(action.action.as_str(), "trash" | "delete" | "clear") {
                self.tabs.retain(|tab| !self.library.links.get(&tab.id).is_some_and(|l|
                    action.action == "clear" || l.id == action.id));
                if self.tabs.is_empty() { self.new_tab(); }
                self.active = self.active.min(self.tabs.len() - 1);
            }
        }
        for item in serde_json::from_str::<Vec<Imported>>(&storage::take_imports()).unwrap_or_default() {
            self.open_library_record(Record { id: storage::new_id(), name: item.name,
                source: item.source, trashed: false });
            if let Some(tab) = self.tabs.get(self.active) {
                if let Some(link) = self.library.links.get_mut(&tab.id) { link.revision = u64::MAX; }
            }
        }
        for tab in &self.tabs {
            let link = self.library.links.entry(tab.id).or_insert_with(|| Link {
                id: storage::new_id(), revision: u64::MAX, persisted: false, name: tab.path.file_name()
                    .unwrap_or_default().to_string_lossy().into_owned(),
            });
            if link.revision != tab.document.revision() {
                storage::save_document(&link.id, &link.name, tab.document.source());
                link.revision = tab.document.revision();
                link.persisted = true;
            }
        }
        self.library.links.retain(|key, _| self.tabs.iter().any(|t| t.id == *key));
        let ids: Vec<_> = self.tabs.iter().filter_map(|t| self.library.links.get(&t.id).filter(|l| l.persisted).map(|l| &l.id)).collect();
        let json = serde_json::to_string(&ids).unwrap_or_default();
        if self.library.remembered != json { storage::remember_tabs(&json); self.library.remembered = json; }
    }
    pub(super) fn library_window(&mut self, ctx: &egui::Context) {
        if !self.library.open { return; }
        let snapshot: Snapshot = serde_json::from_str(&storage::snapshot()).unwrap_or_default();
        let mut open = None;
        let mut action = None;
        let safe = storage::safe_to_close();
        let dismissed = dialogs::show(ctx, "browser-library", "Browser library", 620.0, |ui| {
            ui.label("Documents are stored only in this browser, not on the server. Clearing browser data deletes them. Download important diagrams as backups.");
            if !snapshot.error.is_empty() { ui.colored_label(egui::Color32::LIGHT_RED, &snapshot.error); }
            if !snapshot.notice.is_empty() { ui.label(&snapshot.notice); }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.library.trash, false, "Documents");
                ui.selectable_value(&mut self.library.trash, true, "Trash");
                if ui.button("Import .laml…").clicked() { storage::import_laml(); }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                let rows: Vec<_> = snapshot.records.iter().filter(|r| r.trashed == self.library.trash).collect();
                if rows.is_empty() { ui.label(if self.library.trash { "Trash is empty." } else { "No documents." }); }
                for record in rows {
                    ui.push_id(&record.id, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(&record.name);
                            if self.library.trash {
                                if ui.add_enabled(safe, egui::Button::new("Restore")).clicked() { action = Some(("restore", record.id.clone(), String::new())); }
                                if ui.add_enabled(safe, egui::Button::new("Delete permanently…")).clicked() {
                                    self.library.confirm = Some(("delete".into(), record.id.clone()));
                                }
                            } else {
                                if ui.button("Open").clicked() { open = Some(record.id.clone()); }
                                if ui.add_enabled(safe, egui::Button::new("Rename…")).clicked() { self.library.rename = Some((record.id.clone(), record.name.clone())); }
                                if ui.button("Download").clicked() { storage::download(&record.name, record.source.as_bytes(), "text/plain;charset=utf-8"); }
                                if ui.add_enabled(safe, egui::Button::new("Move to trash")).clicked() { action = Some(("trash", record.id.clone(), String::new())); }
                            }
                        });
                        ui.separator();
                    });
                }
            });
            ui.add_space(12.0);
            if let Some((id, name)) = &mut self.library.rename {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(name);
                    if ui.add_enabled(safe && !name.trim().is_empty(), egui::Button::new("Rename")).clicked() {
                        let mut name = name.trim().replace(['/', '\\'], "-");
                        if !name.ends_with(".laml") { name.push_str(".laml"); }
                        action = Some(("rename", id.clone(), name));
                    }
                });
            }
            if let Some((kind, id)) = self.library.confirm.clone() {
                ui.colored_label(egui::Color32::LIGHT_RED, if kind == "clear" {
                    "Permanently delete ALL documents and trash from this browser? This cannot be undone."
                } else { "Permanently delete the selected trash? This cannot be undone." });
                ui.horizontal(|ui| {
                    if ui.add_enabled(safe, egui::Button::new("Delete permanently")).clicked() {
                        action = Some((if kind == "clear" { "clear" } else if kind == "empty" { "empty" } else { "delete" }, id, String::new()));
                        self.library.confirm = None;
                    }
                    if ui.button("Cancel").clicked() { self.library.confirm = None; }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                if self.library.trash && ui.add_enabled(safe, egui::Button::new("Empty trash…")).clicked() { self.library.confirm = Some(("empty".into(), String::new())); }
                if ui.add_enabled(safe, egui::Button::new("Delete everything…")).clicked() { self.library.confirm = Some(("clear".into(), String::new())); }
                if ui.button("Close").clicked() { self.library.open = false; }
            });
        });
        if let Some((kind, id, name)) = action { storage::library_action(kind, &id, &name); self.library.rename = None; }
        if let Some(id) = open {
            if let Ok(record) = serde_json::from_str(&storage::open_record(&id)) { self.open_library_record(record); self.library.open = false; }
        }
        if dismissed { self.library.open = false; }
    }
}
