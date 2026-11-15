//! Explicit saves, external-change detection and same-directory temporary writes.
use crate::document::Document;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct FileSession {
    pub path: Option<PathBuf>,
    baseline: String,
    backend: crate::backend::Handle,
}
#[derive(Debug, PartialEq, Eq)]
pub enum DiskStatus {
    Unchanged,
    Reloaded,
    Conflict,
}

impl FileSession {
    pub fn new(path: Option<PathBuf>, source: &str) -> Self {
        Self::with_backend(path, source, crate::backend::local())
    }
    pub fn with_backend(
        path: Option<PathBuf>,
        source: &str,
        backend: crate::backend::Handle,
    ) -> Self {
        Self {
            path: path.map(|path| backend.resolve_existing(&path).unwrap_or(path)),
            baseline: source.into(),
            backend,
        }
    }
    pub fn open(path: &Path) -> Result<(Self, String), String> {
        Self::open_with_backend(path, crate::backend::local())
    }
    pub fn open_with_backend(
        path: &Path,
        backend: crate::backend::Handle,
    ) -> Result<(Self, String), String> {
        let path = backend.resolve_existing(path)?;
        let source = backend.read_source(&path)?;
        Ok((
            Self {
                path: Some(path),
                baseline: source.clone(),
                backend,
            },
            source,
        ))
    }
    pub fn poll(&mut self, document: &mut Document, pending: bool) -> Result<DiskStatus, String> {
        let Some(path) = &self.path else {
            return Ok(DiskStatus::Unchanged);
        };
        let source = self.backend.read_source(path)?;
        if source == self.baseline {
            return Ok(DiskStatus::Unchanged);
        }
        if document.is_dirty() || pending {
            return Ok(DiskStatus::Conflict);
        }
        self.baseline = source.clone();
        document.replace_loaded(source);
        Ok(DiskStatus::Reloaded)
    }
    pub fn reload(&mut self, document: &mut Document) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No file is open")?;
        let source = self.backend.read_source(path)?;
        self.baseline = source.clone();
        document.replace_loaded(source);
        Ok(())
    }
    pub fn save(&mut self, document: &mut Document) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("Choose Save as for a new document")?;
        self.backend
            .write_source(path, document.source(), Some(&self.baseline))?;
        self.baseline = document.source().into();
        document.mark_saved();
        Ok(())
    }
    /// Existing targets require the snapshot explicitly confirmed by the user.
    pub fn save_as(
        &mut self,
        document: &mut Document,
        path: &Path,
        confirmed: Option<&str>,
    ) -> Result<(), String> {
        let path = self.backend.resolve_target(path)?;
        self.backend
            .write_source(&path, document.source(), confirmed)?;
        self.path = Some(path);
        self.baseline = document.source().into();
        document.mark_saved();
        document.invalidate(); // Relative style/font imports now have a different base.
        Ok(())
    }
}

pub fn absolute_target(path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("Expected a filename")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize().map_err(|e| e.to_string())?.join(name))
}

fn check(path: &Path, expected: Option<&str>) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(
                    "Save target must be a regular file, not a directory or symlink".into(),
                );
            }
            if metadata.permissions().readonly() {
                return Err("Save target is read-only".into());
            }
            let current = fs::read_to_string(path).map_err(|e| e.to_string())?;
            if expected != Some(current.as_str()) {
                return Err("File changed externally or target already exists. Reload or save to another path.".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && expected.is_none() => {}
        Err(error) => return Err(format!("Cannot safely save {}: {error}", path.display())),
    }
    Ok(())
}

pub(crate) fn write_checked(
    path: &Path,
    source: &str,
    expected: Option<&str>,
) -> Result<(), String> {
    check(path, expected)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    if let Ok(metadata) = fs::metadata(path) {
        temp.as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
    }
    temp.write_all(source.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    check(path, expected)?;
    if expected.is_some() {
        temp.persist(path).map_err(|e| e.error.to_string())?;
    } else {
        temp.persist_noclobber(path)
            .map_err(|e| e.error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_roundtrips_crlf_and_preserves_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.laml");
        fs::write(&path, "old\r\n").unwrap();
        let (mut session, source) = FileSession::open(&path).unwrap();
        let mut doc = Document::new(source);
        doc.replace_text("new\r\n".into()).unwrap();
        session.save(&mut doc).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(fs::read_to_string(path).unwrap(), "new\r\n");
        doc.undo();
        assert_eq!(doc.source(), "old\r\n");
        assert!(doc.is_dirty());
    }
    #[test]
    fn clean_reload_but_dirty_or_pending_conflicts_and_refuses_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.laml");
        fs::write(&path, "one").unwrap();
        let (mut session, source) = FileSession::open(&path).unwrap();
        let mut doc = Document::new(source);
        fs::write(&path, "two").unwrap();
        assert_eq!(session.poll(&mut doc, true).unwrap(), DiskStatus::Conflict);
        assert_eq!(session.poll(&mut doc, false).unwrap(), DiskStatus::Reloaded);
        assert_eq!(doc.source(), "two");
        doc.replace_text("mine".into()).unwrap();
        fs::write(&path, "theirs").unwrap();
        assert_eq!(session.poll(&mut doc, false).unwrap(), DiskStatus::Conflict);
        assert!(session.save(&mut doc).is_err());
        assert_eq!(doc.source(), "mine");
        assert_eq!(fs::read_to_string(&path).unwrap(), "theirs");
    }
    #[test]
    fn save_as_needs_confirmation_and_rechecks_its_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("target.laml");
        let mut session = FileSession::new(None, "mine");
        let mut doc = Document::new("mine".into());
        fs::write(&path, "other").unwrap();
        assert!(session.save_as(&mut doc, &path, None).is_err());
        fs::write(&path, "newer").unwrap();
        assert!(session.save_as(&mut doc, &path, Some("other")).is_err());
        session.save_as(&mut doc, &path, Some("newer")).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
    }
    #[test]
    fn missing_original_is_not_silently_recreated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.laml");
        fs::write(&path, "old").unwrap();
        let (mut session, source) = FileSession::open(&path).unwrap();
        let mut doc = Document::new(source);
        fs::remove_file(&path).unwrap();
        assert!(session.save(&mut doc).is_err());
        assert!(!path.exists());
    }
    #[cfg(unix)]
    #[test]
    fn save_as_never_replaces_a_symlink_target() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.laml");
        let link = dir.path().join("link.laml");
        fs::write(&original, "original").unwrap();
        std::os::unix::fs::symlink(&original, &link).unwrap();
        let mut session = FileSession::new(None, "mine");
        let mut doc = Document::new("mine".into());
        assert!(session.save_as(&mut doc, &link, Some("original")).is_err());
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn read_only_target_is_not_replaced_via_directory_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readonly.laml");
        fs::write(&path, "original").unwrap();
        let (mut session, source) = FileSession::open(&path).unwrap();
        let mut doc = Document::new(source);
        doc.replace_text("edited".into()).unwrap();
        let permissions = fs::metadata(&path).unwrap().permissions();
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = session.save(&mut doc);
        fs::set_permissions(&path, permissions).unwrap();
        assert!(result.is_err());
        assert!(doc.is_dirty());
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
    }

    #[test]
    fn new_file_can_be_saved_and_parent_failures_do_not_mark_clean() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.laml");
        let mut session = FileSession::new(None, "");
        let mut doc = Document::new(String::new());
        doc.replace_text("edited".into()).unwrap();
        assert!(
            session
                .save_as(&mut doc, &dir.path().join("absent/new.laml"), None)
                .is_err()
        );
        assert!(doc.is_dirty());
        session.save_as(&mut doc, &path, None).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "edited");
    }
}
