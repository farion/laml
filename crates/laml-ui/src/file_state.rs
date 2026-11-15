//! Portable file identity and acknowledged disk baseline. No filesystem calls.
use crate::document::Document;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileState {
    pub path: Option<PathBuf>,
    baseline: String,
}
#[derive(Debug, PartialEq, Eq)]
pub enum DiskStatus {
    Unchanged,
    Reloaded,
    Conflict,
}
impl FileState {
    /// The host/service resolves the address before supplying it here.
    pub fn new(path: Option<PathBuf>, source: &str) -> Self {
        Self {
            path,
            baseline: source.into(),
        }
    }
    pub fn baseline(&self) -> &str {
        &self.baseline
    }
    pub fn observe(
        &mut self,
        source: String,
        document: &mut Document,
        pending: bool,
    ) -> DiskStatus {
        if source == self.baseline {
            return DiskStatus::Unchanged;
        }
        if document.is_dirty() || pending {
            return DiskStatus::Conflict;
        }
        self.baseline = source.clone();
        document.replace_loaded(source);
        DiskStatus::Reloaded
    }
    /// A save acknowledges the submitted snapshot, never whatever newer source
    /// the user has typed while the write was running. Undo history is retained.
    pub fn acknowledge_save(&mut self, path: PathBuf, snapshot: String, document: &mut Document) {
        let changed_path = self.path.as_ref() != Some(&path);
        self.path = Some(path);
        self.baseline = snapshot.clone();
        document.mark_saved_snapshot(snapshot);
        if changed_path {
            document.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_save_preserves_newer_text_dirty_state_and_history() {
        let mut document = Document::new("old".into());
        let mut file = FileState::new(Some("a.laml".into()), "old");
        document.replace_text("submitted".into()).unwrap();
        document.replace_text("newer".into()).unwrap();
        let revision = document.revision();
        file.acknowledge_save("a.laml".into(), "submitted".into(), &mut document);
        assert_eq!(document.revision(), revision);
        assert_eq!(document.source(), "newer");
        assert!(document.is_dirty());
        assert!(document.undo());
        assert_eq!(document.source(), "submitted");
        assert!(!document.is_dirty());
    }
    #[test]
    fn save_as_invalidates_relative_dependencies_without_replacing_newer_source() {
        let mut document = Document::new("old".into());
        let mut file = FileState::new(None, "old");
        document.replace_text("newer".into()).unwrap();
        file.acknowledge_save("copy.laml".into(), "old".into(), &mut document);
        assert_eq!(document.revision(), 2);
        assert_eq!(document.source(), "newer");
        assert_eq!(file.baseline(), "old");
        assert!(document.is_dirty());
    }
}
