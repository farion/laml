//! Private, atomic recovery snapshots. Per-session file locks exclude live editors.
pub use crate::recovery_record::Record;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
pub struct Store {
    root: PathBuf,
    lease: tempfile::NamedTempFile,
    pub closed: bool,
    limits: bool,
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Recovery directory cannot be a symlink".into());
        }
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let lease = tempfile::Builder::new()
            .prefix("session-")
            .suffix(".lock")
            .tempfile_in(&root)
            .map_err(|e| e.to_string())?;
        lease.as_file().try_lock().map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            lease,
            closed: false,
            limits: true,
        })
    }
    pub fn with_limits(mut self, limits: bool) -> Self {
        self.limits = limits;
        self
    }
    fn own(&self) -> PathBuf {
        self.lease.path().with_extension("json")
    }
    pub fn sync(&self, path: Option<&Path>, source: &str, dirty: bool) -> Result<(), String> {
        if self.closed {
            return Ok(());
        }
        if !dirty {
            return self.clear();
        }
        if self.limits && source.len() > 32 * 1024 * 1024 {
            return Err("Buffer exceeds the 32 MiB recovery limit; save explicitly.".into());
        }
        // Path bytes must round-trip: never silently restore a lossy filename.
        let path = path
            .map(|p| p.to_str().ok_or("Recovery requires a Unicode file path"))
            .transpose()?;
        let json = serde_json::json!({"version":1,"path":path,"source":source});
        let bytes = serde_json::to_vec(&json).map_err(|e| e.to_string())?;
        if self.limits && bytes.len() > 64 * 1024 * 1024 {
            return Err("Encoded recovery snapshot exceeds 64 MiB; save explicitly.".into());
        }
        let mut temp = tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(self.own()).map_err(|e| e.error.to_string())?;
        Ok(())
    }
    pub fn clear(&self) -> Result<(), String> {
        match fs::remove_file(self.own()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn abandoned(file: &Path) -> bool {
        let lock = file.with_extension("lock");
        match File::open(lock) {
            Ok(f) => f.try_lock().is_ok(),
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        }
    }
    pub fn pending(&self) -> Result<Vec<Record>, String> {
        let mut records = vec![];
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|s| s != "json") || !Self::abandoned(&path) {
                continue;
            }
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if !meta.is_file() || (self.limits && meta.len() > 64 * 1024 * 1024) {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                continue;
            };
            if json["version"] != 1 {
                continue;
            }
            if let Some(source) = json["source"].as_str() {
                records.push(Record {
                    file: path,
                    path: json["path"].as_str().map(PathBuf::from),
                    source: source.into(),
                });
            }
        }
        records.sort_by(|a, b| a.file.cmp(&b.file));
        Ok(records)
    }
    pub fn discard(&self, record: &Record) -> Result<(), String> {
        if record.file.parent() != Some(self.root.as_path()) || !Self::abandoned(&record.file) {
            return Err("Recovery snapshot is no longer available".into());
        }
        fs::remove_file(&record.file).map_err(|e| e.to_string())
    }
}
pub fn default_directory() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LAML_UI_STATE_DIR") {
        return Some(PathBuf::from(path).join("recovery"));
    }
    #[cfg(windows)]
    {
        return std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("LAML/recovery"));
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Application Support/LAML/recovery"))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
            .map(|p| p.join("laml-ui/recovery"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_sessions_are_not_recovered_and_snapshots_are_private_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::open(dir.path().into()).unwrap();
        a.sync(Some(Path::new("/tmp/ä.laml")), "Grün\r\n", true)
            .unwrap();
        let b = Store::open(dir.path().into()).unwrap();
        assert!(b.pending().unwrap().is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(a.own()).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(a);
        let records = b.pending().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "Grün\r\n");
        b.discard(&records[0]).unwrap();
        assert!(b.pending().unwrap().is_empty());
        b.sync(None, "new", true).unwrap();
        b.sync(None, "new", false).unwrap();
        assert!(!b.own().exists());
    }
}
