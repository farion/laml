use super::Format;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
pub fn render(source: &str, path: &Path, format: Format) -> Result<Vec<u8>, String> {
    let preview = crate::preview::build(source, path)?;
    match format {
        Format::Svg => Ok(preview.svg.into_bytes()),
        Format::Png => laml::export::png(&preview.svg),
        Format::Pdf => laml::export::pdf(&preview.svg),
    }
}
pub fn target(path: &Path, format: Format, protected: &[PathBuf]) -> Result<PathBuf, String> {
    let path = crate::files::absolute_target(path)?;
    if path.extension().and_then(|s| s.to_str()) != Some(format.extension()) {
        return Err(format!("Choose a .{} output filename", format.extension()));
    }
    for original in protected {
        if path == *original
            || path
                .canonicalize()
                .ok()
                .zip(original.canonicalize().ok())
                .is_some_and(|(a, b)| a == b)
        {
            return Err("Export cannot replace the source or one of its dependencies".into());
        }
    }
    Ok(path)
}
pub fn snapshot(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.is_file() || meta.file_type().is_symlink() || meta.permissions().readonly() {
                return Err("Output must be a writable regular file, not a symlink".into());
            }
            if meta.len() > 128 * 1024 * 1024 {
                return Err("Existing output exceeds the 128 MiB confirmation limit".into());
            }
            fs::read(path).map(Some).map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
pub fn write(path: &Path, data: &[u8], expected: Option<&[u8]>) -> Result<(), String> {
    if snapshot(path)?.as_deref() != expected {
        return Err("Output changed since confirmation; export cancelled".into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    if let Ok(meta) = fs::metadata(path) {
        temp.as_file()
            .set_permissions(meta.permissions())
            .map_err(|e| e.to_string())?;
    }
    temp.write_all(data).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    if snapshot(path)?.as_deref() != expected {
        return Err("Output changed during export; cancelled".into());
    }
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
    fn all_formats_use_the_compiler_and_checked_writes_preserve_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("drawing.laml");
        let source = "semantics {\n A\n B\n A -> B\n}\n";
        for (format, signature) in [
            (Format::Svg, b"<svg".as_slice()),
            (Format::Png, b"\x89PNG".as_slice()),
            (Format::Pdf, b"%PDF".as_slice()),
        ] {
            let bytes = render(source, &path, format).unwrap();
            assert!(bytes.windows(signature.len()).any(|w| w == signature));
            let output = path.with_extension(format.extension());
            write(&output, &bytes, None).unwrap();
            assert!(write(&output, b"bad", None).is_err());
            assert_eq!(fs::read(&output).unwrap(), bytes);
        }
        let output = dir.path().join("existing.svg");
        fs::write(&output, "first").unwrap();
        let old = snapshot(&output).unwrap();
        fs::write(&output, "external").unwrap();
        assert!(write(&output, b"replace", old.as_deref()).is_err());
        assert_eq!(fs::read_to_string(output).unwrap(), "external");
    }
}
