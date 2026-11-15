//! Capability-relative host IO. The compiler only receives private staged files.
use cap_std::fs::{Dir, OpenOptions};
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
pub const SOURCE_LIMIT: usize = 4 * 1024 * 1024;
pub const FILE_LIMIT: usize = 32 * 1024 * 1024;
#[derive(Debug)]
pub struct Workspace {
    pub dir: Dir,
    pub limits: bool,
}
fn io(error: std::io::Error) -> String {
    format!("Workspace operation failed: {}", error.kind())
}
pub fn relative(path: &Path) -> Result<PathBuf, String> {
    let text = path.to_str().ok_or("Workspace path must be UTF-8")?;
    if text.is_empty()
        || (((!cfg!(windows) && text.contains('\\')) || text.contains(':'))
            || text.chars().any(char::is_control))
    {
        return Err("Invalid workspace path".into());
    }
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::Normal(p) if !p.to_string_lossy().starts_with('.') => out.push(p),
            _ => {
                return Err(
                    "Use a relative workspace path without hidden or parent components".into(),
                );
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err("Missing workspace file name".into());
    }
    Ok(out)
}
pub fn extension(path: &Path, allowed: &[&str]) -> Result<(), String> {
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| allowed.contains(&s))
    {
        Ok(())
    } else {
        Err("File type is not permitted for this operation".into())
    }
}
impl Workspace {
    pub fn open(root: &Path) -> Result<Self, String> {
        Ok(Self {
            dir: Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(io)?,
            limits: false,
        })
    }
    pub fn with_limits(mut self, limits: bool) -> Self {
        self.limits = limits;
        self
    }
    pub fn resolve(&self, path: &Path, existing: bool) -> Result<PathBuf, String> {
        let path = relative(path)?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let canonical_parent = self
            .dir
            .canonicalize(parent)
            .map_err(io)?
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect::<PathBuf>();
        let expected_parent = if parent == Path::new(".") {
            PathBuf::new()
        } else {
            parent.to_owned()
        };
        if canonical_parent != expected_parent {
            return Err("Symlinked workspace paths are not permitted".into());
        }
        match self.dir.canonicalize(&path) {
            Ok(canonical) if canonical == path => Ok(path),
            Ok(_) => Err("Symlinked workspace files are not permitted".into()),
            Err(e) if !existing && e.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(e) => Err(io(e)),
        }
    }
    pub fn read(&self, path: &Path, limit: usize) -> Result<Option<Vec<u8>>, String> {
        let limit = if self.limits { limit } else { usize::MAX };
        let path = self.resolve(path, false)?;
        match self.dir.metadata(&path) {
            Ok(m) if !m.is_file() => return Err("Workspace target is not a regular file".into()),
            Ok(m) if m.len() > limit as u64 => {
                return Err("Workspace file exceeds size limit".into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(e)),
            _ => {}
        }
        let file = self.dir.open(&path).map_err(io)?;
        if !file.metadata().map_err(io)?.is_file() {
            return Err("Workspace target is not a regular file".into());
        }
        let mut bytes = vec![];
        file.take((limit as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(io)?;
        if bytes.len() > limit {
            return Err("Workspace file exceeds size limit".into());
        }
        Ok(Some(bytes))
    }
    pub fn write(&self, path: &Path, data: &[u8], expected: Option<&[u8]>) -> Result<(), String> {
        if self.limits && data.len() > 128 * 1024 * 1024 {
            return Err("Output exceeds size limit".into());
        }
        let path = self.resolve(path, false)?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        // Keep this directory capability open throughout snapshot check and rename.
        let dir = self.dir.open_dir(parent).map_err(io)?;
        let name = Path::new(path.file_name().ok_or("Missing file name")?);
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| "Random source unavailable")?;
        let temp = format!(
            ".laml-write-{}",
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let result = (|| {
            let mut file = dir
                .open_with(&temp, OpenOptions::new().write(true).create_new(true))
                .map_err(io)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(cap_std::fs::Permissions::from_std(
                    std::fs::Permissions::from_mode(0o600),
                ))
                .map_err(io)?;
            }
            if let Ok(meta) = dir.metadata(name) {
                if meta.permissions().readonly() {
                    return Err("Target is read-only".into());
                }
                file.set_permissions(meta.permissions()).map_err(io)?;
            }
            file.write_all(data).map_err(io)?;
            file.sync_all().map_err(io)?;
            drop(file);
            if dir.metadata(name).is_ok_and(|m| m.permissions().readonly()) {
                return Err("Target became read-only".into());
            }
            // Snapshot against the pinned directory rather than re-opening its parent.
            let pinned = Workspace {
                dir: dir.try_clone().map_err(io)?,
                limits: self.limits,
            };
            if pinned.read(name, 128 * 1024 * 1024)?.as_deref() != expected {
                return Err("File changed since confirmation; nothing was written".into());
            }
            if expected.is_none() {
                dir.hard_link(&temp, &dir, name).map_err(io)?;
            } else {
                dir.rename(&temp, &dir, name).map_err(io)?;
            }
            Ok(())
        })();
        let _ = dir.remove_file(&temp);
        result
    }
}

/// Portable namespace spelling, never a host absolute path.
pub fn wire(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}
