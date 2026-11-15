use super::*;
use laml_ui::recovery::{Record, Store};
pub(super) struct RecoveryUi {
    pub store: Option<Store>,
    pub records: Vec<Record>,
    pub error: Option<String>,
    pub origin: Option<PathBuf>,
    pub restoring: Option<Record>,
    pub dismissed: std::collections::BTreeSet<PathBuf>,
    last: Option<(u64, PathBuf)>,
    checked: Instant,
    #[cfg(test)]
    _temp: Option<tempfile::TempDir>,
}
impl Default for RecoveryUi {
    fn default() -> Self {
        #[cfg(test)]
        let temp = tempfile::tempdir().ok();
        #[cfg(test)]
        let directory = temp.as_ref().map(|d| d.path().join("recovery"));
        #[cfg(not(test))]
        let directory = laml_ui::recovery::default_directory();
        let result = directory
            .ok_or_else(|| "No local recovery directory available".to_string())
            .and_then(Store::open);
        let (store, error) = match result {
            Ok(store) => (Some(store), None),
            Err(e) => (None, Some(e)),
        };
        let records = store
            .as_ref()
            .and_then(|s| s.pending().ok())
            .unwrap_or_default();
        Self {
            store,
            error,
            records,
            origin: None,
            restoring: None,
            dismissed: Default::default(),
            last: None,
            checked: Instant::now(),
            #[cfg(test)]
            _temp: temp,
        }
    }
}
impl RecoveryUi {
    pub fn clear(&mut self) -> Result<(), String> {
        if let Some(store) = &self.store {
            store.clear()?;
        }
        if let Some(record) = self.restoring.take()
            && let Some(store) = &self.store
        {
            store.discard(&record)?;
        }
        self.last = None;
        self.origin = None;
        Ok(())
    }
}
impl Editor {
    pub(super) fn checkpoint(&mut self) -> bool {
        if let Some(store) = &self.recovery.store {
            let path = self
                .file
                .path
                .as_deref()
                .or(self.recovery.origin.as_deref());
            #[cfg(not(target_arch = "wasm32"))]
            let result = store
                .sync(path, self.document.source(), self.document.is_dirty())
                .map(|()| true);
            #[cfg(target_arch = "wasm32")]
            let result = store.checkpoint(path, self.document.source(), self.document.is_dirty());
            match result {
                Ok(true) => {
                    self.recovery.last = Some((self.document.revision(), self.path.clone()));
                    true
                }
                Ok(false) => false,
                Err(e) => {
                    self.recovery.error = Some(e);
                    false
                }
            }
        } else {
            false
        }
    }
    pub(super) fn finish_recovery(&mut self) {
        if let Some(record) = self.recovery.restoring.take() {
            if let Some(store) = &self.recovery.store
                && let Err(error) = store.discard(&record)
            {
                self.recovery.error = Some(error);
                self.recovery.restoring = Some(record);
                return;
            }
            self.recovery.dismissed.insert(record.file.clone());
            self.recovery.records.retain(|r| r.file != record.file);
        }
    }
    pub(super) fn recovery_tick(&mut self) {
        if self.recovery.checked.elapsed() >= Duration::from_secs(1) {
            self.recovery.checked = Instant::now();
            #[cfg(target_arch = "wasm32")]
            if let Some(store) = &self.recovery.store {
                match store.pending() {
                    Ok(records) => {
                        for record in records {
                            if !self.recovery.dismissed.contains(&record.file)
                                && !self.recovery.records.iter().any(|r| r.file == record.file)
                            {
                                self.recovery.records.push(record);
                            }
                        }
                    }
                    Err(e) => self.recovery.error = Some(e),
                }
            }
            if self.recovery.last.as_ref() != Some(&(self.document.revision(), self.path.clone()))
                && self.checkpoint()
            {
                self.finish_recovery();
            }
        }
    }
    pub(super) fn recovery_window(&mut self, ctx: &egui::Context) {
        if self.recovery.records.is_empty() && self.recovery.error.is_none() {
            return;
        }
        let mut restore = None;
        let mut discard = None;
        let mut dismiss = false;
        let dismissed = dialogs::show(
            ctx,
            ("recovery", self.id),
            if cfg!(target_arch = "wasm32") {
                "Server recovery"
            } else {
                "Local recovery"
            },
            520.0,
            |ui| {
                ui.label(if cfg!(target_arch="wasm32") {"Snapshots stay on the server. Restore loads a snapshot into the editor buffer."} else {"Snapshots stay on this computer. Restore loads a snapshot into the editor buffer."});
                if let Some(e) = &self.recovery.error {
                    ui.colored_label(egui::Color32::YELLOW, e);
                }
                for (n, record) in self.recovery.records.iter().enumerate() {
                    ui.group(|ui| {
                        ui.label(
                            record.path.as_ref().map_or_else(
                                || "Unsaved diagram".into(),
                                |p| p.display().to_string(),
                            ),
                        );
                        ui.small(format!("{} bytes", record.source.len()));
                        ui.horizontal(|ui| {
                            if ui.button("Restore…").clicked() {
                                restore = Some(n);
                            }
                            if ui.button("Discard snapshot").clicked() {
                                discard = Some(n);
                            }
                        });
                    });
                }
                if ui.button("Later / dismiss").clicked() {
                    dismiss = true;
                }
            },
        );
        if let Some(n) = restore {
            self.request_recovery(self.recovery.records[n].clone(), ctx);
        }
        if let Some(n) = discard
            && let Some(store) = &self.recovery.store
        {
            match store.discard(&self.recovery.records[n]) {
                Ok(()) => {
                    self.recovery
                        .dismissed
                        .insert(self.recovery.records[n].file.clone());
                    self.recovery.records.remove(n);
                }
                Err(e) => self.recovery.error = Some(e),
            }
        }
        if dismiss || dismissed {
            self.recovery
                .dismissed
                .extend(self.recovery.records.iter().map(|r| r.file.clone()));
            self.recovery.records.clear();
            self.recovery.error = None;
        }
    }
}
impl Drop for Editor {
    fn drop(&mut self) {
        self.checkpoint();
    }
}
