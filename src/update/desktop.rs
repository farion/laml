//! Explicit native update/install operations. No GUI and no automatic downloads.
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Application {
    Cli,
    Editor,
}
impl Application {
    fn name(self) -> &'static str {
        match self {
            Self::Cli => "laml",
            Self::Editor => "laml-ui",
        }
    }
    fn member(self, tag: &str, target: &str) -> String {
        let name = self.name();
        let root = format!("{name}-{tag}-{target}");
        if self == Self::Editor && target.contains("apple-darwin") {
            format!("{root}/LAML Editor.app/Contents/MacOS/laml-ui")
        } else {
            format!(
                "{root}/{name}{}",
                if target.contains("windows") {
                    ".exe"
                } else {
                    ""
                }
            )
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub tag: String,
    pub available: bool,
}

/// Check metadata only. Installation is a separate explicit operation.
pub fn check_update(app: Application) -> Result<UpdateInfo, String> {
    let target = target().ok_or("no release binary for this OS/architecture")?;
    let release = latest()?.ok_or("no published LAML release is available")?;
    release.asset(&archive_name(app, &release.tag, target))?;
    release.asset("SHA256SUMS")?;
    Ok(UpdateInfo {
        available: release.version > version(VERSION).ok_or("invalid running version")?,
        tag: release.tag,
    })
}

fn archive_name(app: Application, tag: &str, target: &str) -> String {
    format!(
        "{}-{tag}-{target}.{}",
        app.name(),
        if target.contains("windows") {
            "zip"
        } else {
            "tar.gz"
        }
    )
}

fn release_at(tag: &str) -> Result<Release, String> {
    version(tag).ok_or("invalid release version")?;
    if !tag.starts_with('v') {
        return Err("release tag must start with v".into());
    }
    let bytes = fetch(
        &format!("https://api.github.com/repos/farion/laml/releases/tags/{tag}"),
        1_000_000,
        Duration::from_secs(10),
    )?;
    let release = parse_release(&bytes)?;
    if release.tag != tag {
        return Err("release version mismatch".into());
    }
    Ok(release)
}

fn binary(app: Application, release: &Release) -> Result<Vec<u8>, String> {
    let target = target().ok_or("no release binary for this OS/architecture")?;
    let name = archive_name(app, &release.tag, target);
    let archive = release.asset(&name)?;
    let bytes = fetch(&archive.url, 512_000_000, Duration::from_secs(120))?;
    let sums = fetch(
        &release.asset("SHA256SUMS")?.url,
        65_536,
        Duration::from_secs(10),
    )?;
    download::verify(&bytes, &sums, &name, archive.digest.as_deref())?;
    download::unpack(&bytes, &app.member(&release.tag, target))
}

/// Install exactly the version the user checked, never a silently changed latest.
pub fn apply_update(app: Application, tag: &str) -> Result<(), String> {
    if version(tag).ok_or("invalid release version")?
        <= version(VERSION).ok_or("invalid running version")?
    {
        return Err("update must be newer than the running version".into());
    }
    let release = release_at(tag)?;
    let bytes = binary(app, &release)?;
    let current = std::env::current_exe().map_err(|e| format!("cannot locate executable: {e}"))?;
    #[cfg(unix)]
    {
        download::replace_unix(&current, &bytes)
    }
    #[cfg(windows)]
    {
        download::replace_windows(&current, &bytes, app == Application::Editor)
    }
}

/// Per-user installation, without elevation or modification of shell profiles.
pub fn cli_install_path() -> Result<PathBuf, String> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("LAML/bin/laml.exe"))
            .ok_or("LOCALAPPDATA is not set".into())
    } else {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join(".local/bin/laml"))
            .ok_or("HOME is not set".into())
    }
}

/// Download the matching CLI release. Existing files require explicit replacement.
pub fn install_cli(destination: &Path, replace: bool) -> Result<(), String> {
    if destination != cli_install_path()? {
        return Err("unexpected CLI installation path".into());
    }
    let bytes = binary(Application::Cli, &release_at(&format!("v{VERSION}"))?)?;
    install_file(destination, &bytes, replace)
}

fn install_file(destination: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    use std::fs;
    if let Ok(metadata) = fs::symlink_metadata(destination) {
        if !replace {
            return Err("CLI already exists; confirm replacement".into());
        }
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("refusing to replace a directory or symbolic link".into());
        }
    }
    let parent = destination
        .parent()
        .ok_or("installation path has no parent")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create installation directory: {e}"))?;
    // A private same-directory temporary file; publish only a complete executable.
    let staged = destination.with_file_name(format!(".laml-install-{}", std::process::id()));
    let mut owned = false;
    let result = (|| {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|e| format!("cannot stage CLI: {e}"))?;
        owned = true;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if replace {
            #[cfg(windows)]
            {
                return replace_installed_windows(&staged, destination);
            }
            #[cfg(not(windows))]
            {
                fs::rename(&staged, destination).map_err(|e| e.to_string())?;
            }
        } else {
            // No-clobber publication, including when another install raced us.
            fs::hard_link(&staged, destination).map_err(|e| format!("cannot install CLI: {e}"))?;
        }
        Ok(())
    })();
    if owned {
        let _ = fs::remove_file(&staged);
    }
    result
}

#[cfg(windows)]
fn replace_installed_windows(staged: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(old: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let old: Vec<_> = staged.as_os_str().encode_wide().chain(Some(0)).collect();
    let new: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    if unsafe { MoveFileExW(old.as_ptr(), new.as_ptr(), 1 | 8) } == 0 {
        Err(format!(
            "cannot replace CLI (close running CLI processes): {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn updates_refuse_old_versions_before_any_download() {
        assert!(apply_update(Application::Editor, VERSION).is_err());
        assert!(apply_update(Application::Cli, "v0.0.0").is_err());
        assert!(apply_update(Application::Editor, "not-a-version").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn cli_install_rejects_symlinks_even_with_replace_confirmation() {
        let dir = std::env::temp_dir().join(format!("laml-cli-symlink-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("original");
        let link = dir.join("laml");
        std::fs::write(&original, b"original").unwrap();
        std::os::unix::fs::symlink(&original, &link).unwrap();
        assert!(install_file(&link, b"replacement", true).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn archive_paths_cover_all_release_platforms() {
        for target in [
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ] {
            for app in [Application::Cli, Application::Editor] {
                let member = app.member("v1.2.3", target);
                assert!(member.starts_with(&format!("{}-v1.2.3-{target}/", app.name())));
                assert_eq!(member.ends_with(".exe"), target.contains("windows"));
                assert_eq!(
                    member.contains(".app/"),
                    app == Application::Editor && target.contains("apple")
                );
            }
        }
    }
    #[test]
    fn local_install_is_complete_and_never_silently_overwrites() {
        let dir = std::env::temp_dir().join(format!("laml-cli-install-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("laml");
        install_file(&path, b"first", false).unwrap();
        assert!(install_file(&path, b"second", false).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        install_file(&path, b"second", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
