use super::*;
use laml_ui::recovery::{Record, Store};
pub(super) struct RecoveryUi {
    pub store: Option<Store>,
    pub records: Vec<Record>,
    pub error: Option<String>,
    pub origin: Option<PathBuf>,
    pub restoring: Option<Record>,
    pub dismissed: std::collections::BTreeSet<PathBuf>,
    pub recovered: bool,
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
            Ok(store) if !laml_ui::services::browser_mode() => (Some(store), None),
            Ok(_) => (None, None),
            Err(e) => (None, Some(e)),
        };
        let records = Vec::new();
        Self {
            store,
            error,
            records,
            origin: None,
            restoring: None,
            dismissed: Default::default(),
            recovered: false,
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
        self.recovered = false;
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
                    self.recovery.error = None;
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
            if let Some(store) = &self.recovery.store {
                match store.discard_confirmed(&record) {
                    Ok(true) => {
                        self.recovery.error = None;
                    }
                    result => {
                        if let Err(error) = result {
                            self.recovery.error = Some(error);
                        }
                        self.recovery.restoring = Some(record);
                        return;
                    }
                }
            }
            self.recovery.dismissed.insert(record.file.clone());
            self.recovery.records.retain(|r| r.file != record.file);
        }
    }
    pub(super) fn recovery_tick(&mut self) {
        if self.recovery.checked.elapsed() >= Duration::from_secs(1) {
            self.recovery.checked = Instant::now();
            if (self.recovery.last.as_ref() != Some(&(self.document.revision(), self.path.clone()))
                || self.recovery.restoring.is_some())
                && self.checkpoint()
            {
                self.finish_recovery();
            }
        }
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.checkpoint();
    }
}
