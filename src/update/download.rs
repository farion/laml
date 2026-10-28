//! Verify release assets and extract only the named executable.

use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

const MAX_BINARY: u64 = 50_000_000;

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(hex, "{byte:02x}").expect("formatting SHA-256");
    }
    hex
}

pub(super) fn verify(
    archive: &[u8],
    sums: &[u8],
    name: &str,
    digest: Option<&str>,
) -> Result<(), String> {
    let manifest = std::str::from_utf8(sums).map_err(|_| "invalid checksums encoding")?;
    let mut expected = None;
    for line in manifest.lines() {
        let mut columns = line.split_whitespace();
        let (Some(hash), Some(file)) = (columns.next(), columns.next()) else {
            continue;
        };
        if file.trim_start_matches('*') == name {
            if expected.is_some()
                || hash.len() != 64
                || !hash.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err("ambiguous or invalid archive checksum".into());
            }
            expected = Some(hash.to_ascii_lowercase());
        }
    }
    let expected = expected.ok_or("archive is missing from SHA256SUMS")?;
    let actual = sha256_hex(archive);
    if actual != expected {
        return Err("archive checksum mismatch; executable not changed".into());
    }
    if let Some(digest) = digest
        && digest
            .strip_prefix("sha256:")
            .is_none_or(|value| value.to_ascii_lowercase() != actual)
    {
        return Err("release asset digest does not match SHA256SUMS".into());
    }
    Ok(())
}

#[cfg(unix)]
fn unpack(archive: &[u8], member: &str) -> Result<Vec<u8>, String> {
    let gzip = flate2::read::GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(gzip);
    let mut found = None;
    for entry in tar
        .entries()
        .map_err(|e| format!("cannot read tar archive: {e}"))?
    {
        let mut entry = entry.map_err(|e| format!("invalid tar member: {e}"))?;
        if entry.path().map_err(|e| e.to_string())?.as_ref() == std::path::Path::new(member) {
            if found.is_some()
                || !entry.header().entry_type().is_file()
                || entry.size() > MAX_BINARY
            {
                return Err("invalid release executable in archive".into());
            }
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|e| format!("cannot extract executable: {e}"))?;
            found = Some(content);
        }
    }
    found
        .filter(|content| !content.is_empty())
        .ok_or("release archive contains no executable".into())
}

#[cfg(windows)]
fn unpack(archive: &[u8], member: &str) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|e| format!("cannot read zip archive: {e}"))?;
    let mut found = None;
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|e| format!("invalid zip member: {e}"))?;
        // PowerShell ZIP writers may use Windows path separators.
        if file
            .name()
            .map_err(|e| format!("invalid zip member name: {e}"))?
            .replace('\\', "/")
            != member
        {
            continue;
        }
        if found.is_some() || file.is_dir() || file.size() > MAX_BINARY {
            return Err("invalid release executable in archive".into());
        }
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|e| format!("cannot extract executable: {e}"))?;
        found = Some(content);
    }
    found
        .filter(|content| !content.is_empty())
        .ok_or("release archive contains no executable".into())
}

#[cfg(unix)]
pub(super) fn install(archive: &[u8], tag: &str, target: &str) -> Result<(), String> {
    let binary = unpack(archive, &format!("laml-{tag}-{target}/laml"))?;
    let current = std::env::current_exe().map_err(|e| format!("cannot locate executable: {e}"))?;
    replace_unix(&current, &binary)
}

#[cfg(unix)]
fn replace_unix(current: &std::path::Path, binary: &[u8]) -> Result<(), String> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let path = current.with_file_name(format!(".laml-update-{}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("cannot stage update next to {}: {e}", current.display()))?;
        file.write_all(binary).map_err(|e| e.to_string())?;
        let mode = fs::metadata(current)
            .map_err(|e| e.to_string())?
            .permissions()
            .mode();
        file.set_permissions(fs::Permissions::from_mode(mode | 0o111))
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&path, current).map_err(|e| format!("cannot replace {}: {e}", current.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result
}

#[cfg(windows)]
pub(super) fn install(archive: &[u8], tag: &str, target: &str) -> Result<(), String> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::process::{Command, Stdio};
    let binary = unpack(archive, &format!("laml-{tag}-{target}/laml.exe"))?;
    let current = std::env::current_exe().map_err(|e| format!("cannot locate executable: {e}"))?;
    let staged = current.with_file_name(format!(".laml-update-{}.exe", std::process::id()));
    let helper = current.with_file_name(format!(".laml-update-{}.ps1", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
            .map_err(|e| format!("cannot stage update next to {}: {e}", current.display()))?;
        file.write_all(&binary).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::write(&helper, include_str!("../../scripts/replace-windows.ps1"))
            .map_err(|e| e.to_string())?;
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&helper)
            .arg("-ParentPid")
            .arg(std::process::id().to_string())
            .arg("-Staged")
            .arg(&staged)
            .arg("-Destination")
            .arg(&current)
            .arg("-Helper")
            .arg(&helper)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot start Windows updater: {e}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staged);
        let _ = fs::remove_file(helper);
    }
    result
}

#[cfg(not(any(unix, windows)))]
pub(super) fn install(_: &[u8], _: &str, _: &str) -> Result<(), String> {
    Err("self-updates are not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn only_named_binary_is_extracted_and_replacement_is_atomic() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let member = "laml-v1.1.0-x86_64-unknown-linux-gnu/laml";
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_size(3);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, member, &b"new"[..])
            .unwrap();
        let archive = builder.into_inner().unwrap().finish().unwrap();
        assert_eq!(unpack(&archive, member).unwrap(), b"new");
        assert!(unpack(&archive, "another/laml").is_err());
        let dir = std::env::temp_dir().join(format!("laml-atomic-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("laml");
        fs::write(&dest, b"old").unwrap();
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o700)).unwrap();
        replace_unix(&dest, b"new").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert_ne!(fs::metadata(&dest).unwrap().permissions().mode() & 0o111, 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn extracts_only_the_named_windows_binary() {
        use std::io::Write;
        let member = "laml-v1.1.0-x86_64-pc-windows-msvc/laml.exe";
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(member, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"windows-binary").unwrap();
        let archive = writer.finish().unwrap().into_inner();
        assert_eq!(unpack(&archive, member).unwrap(), b"windows-binary");
        assert!(unpack(&archive, "other/laml.exe").is_err());
    }

    #[test]
    fn checksum_is_required_and_corruption_is_rejected() {
        let data = b"archive bytes";
        let hash = sha256_hex(data);
        let name = "laml-v1.1.0-x86_64-unknown-linux-gnu.tar.gz";
        let manifest = format!("{hash}  {name}\n");
        verify(
            data,
            manifest.as_bytes(),
            name,
            Some(&format!("sha256:{hash}")),
        )
        .unwrap();
        assert!(verify(b"bad", manifest.as_bytes(), name, None).is_err());
        assert!(verify(data, manifest.as_bytes(), "other.tar.gz", None).is_err());
    }
}
