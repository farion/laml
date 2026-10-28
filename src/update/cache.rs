//! One network check per user per day. Normal commands only read a tiny file.

use super::{VERSION, latest, target, version};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DAY: u64 = 24 * 60 * 60;

#[derive(Default)]
struct State {
    checked: u64,
    latest: Option<String>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn due(checked: u64, now: u64) -> bool {
    checked == 0 || now < checked || now - checked >= DAY
}
fn cache_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("laml").join("cache"))
    } else {
        if let Some(path) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from)
            && path.is_absolute()
        {
            return Some(path.join("laml"));
        }
        std::env::var_os("HOME").map(|home| {
            let home = PathBuf::from(home);
            if cfg!(target_os = "macos") {
                home.join("Library/Caches/laml")
            } else {
                home.join(".cache/laml")
            }
        })
    }
}
fn read(path: &Path) -> State {
    let Ok(bytes) = fs::read(path) else {
        return State::default();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return State::default();
    };
    State {
        checked: value["checked"].as_u64().unwrap_or(0),
        latest: value["latest"].as_str().map(str::to_owned),
    }
}
fn write(path: &Path, state: &State) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let bytes =
        serde_json::to_vec(&serde_json::json!({"checked":state.checked,"latest":state.latest}))?;
    fs::write(&temporary, bytes)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}
fn lock(path: &Path) -> bool {
    if OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .is_ok()
    {
        return true;
    }
    // An interrupted checker cannot block updates indefinitely.
    let stale = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .is_some_and(|age| age > Duration::from_secs(120));
    stale
        && fs::remove_file(path).is_ok()
        && OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .is_ok()
}

fn notice(state: &State) -> Option<String> {
    let latest = state.latest.as_deref()?;
    (version(latest)? > version(VERSION).expect("package version")).then(|| {
        format!("LAML update available: {latest} (installed: v{VERSION}). Run `laml update`.")
    })
}

pub(super) fn notify_in_background() {
    if std::env::var_os("LAML_NO_UPDATE_CHECK").is_some() || target().is_none() {
        return;
    }
    let Some(dir) = cache_dir() else { return };
    let path = dir.join("update.json");
    let mut state = read(&path);
    if let Some(message) = notice(&state) {
        eprintln!("{message}");
    }
    let timestamp = now();
    if !due(state.checked, timestamp) || fs::create_dir_all(&dir).is_err() {
        return;
    }
    let lock_path = dir.join("update.lock");
    if !lock(&lock_path) {
        return;
    }
    state.checked = timestamp;
    if write(&path, &state).is_err() {
        let _ = fs::remove_file(lock_path);
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        let _ = fs::remove_file(lock_path);
        return;
    };
    let mut child = Command::new(exe);
    child
        .arg("__update-check")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x00000008 | 0x08000000); // detached, no console window
    }
    if child.spawn().is_err() {
        let _ = fs::remove_file(lock_path);
    }
}

pub(super) fn refresh() {
    let Some(dir) = cache_dir() else { return };
    let path = dir.join("update.json");
    let lock_path = dir.join("update.lock");
    let mut state = read(&path);
    if let Ok(release) = latest() {
        state.latest = release.map(|release| release.tag);
    }
    state.checked = now();
    let _ = write(&path, &state);
    let _ = fs::remove_file(lock_path);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refresh_is_never_due_more_than_once_per_day() {
        assert!(due(0, 100));
        assert!(!due(100, 100 + DAY - 1));
        assert!(due(100, 100 + DAY));
        assert!(due(200, 100));
    }
    #[test]
    fn a_newer_cached_version_is_announced_without_network() {
        let mut state = State {
            checked: 100,
            latest: Some("v9.9.9".into()),
        };
        assert!(notice(&state).unwrap().contains("laml update"));
        state.latest = Some(format!("v{VERSION}"));
        assert!(notice(&state).is_none());
    }
}
