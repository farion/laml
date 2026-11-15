//! Local source/font/style dependency snapshots, including empty and missing imports.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
#[derive(Default)]
pub struct Watch {
    pub paths: Vec<PathBuf>,
    stamps: Vec<Option<(u64, SystemTime)>>,
    root: PathBuf,
}
fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c.as_os_str()),
        }
    }
    out
}
fn stamp(path: &Path, root: &Path) -> Option<(u64, SystemTime)> {
    let canonical = path.canonicalize().ok()?;
    if !canonical.starts_with(root) {
        return None;
    }
    let m = fs::metadata(canonical).ok()?;
    Some((m.len(), m.modified().ok()?))
}
impl Watch {
    pub fn new(source: &str, path: &Path) -> Self {
        let root = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .canonicalize();
        let Ok(root) = root else {
            return Self::default();
        };
        let mut paths = BTreeSet::new();
        if let Ok((diagram, _, _)) = crate::notation::inspect(source) {
            for font in [
                diagram.settings.font_file,
                diagram.settings.font_italic_file,
            ]
            .into_iter()
            .flatten()
            {
                let p = normalized(&root.join(font));
                if p.starts_with(&root) {
                    paths.insert(p);
                }
            }
            fn imports(
                entries: &[laml::syntax::StyleEntry],
                base: &Path,
                root: &Path,
                seen: &mut BTreeSet<PathBuf>,
                depth: usize,
            ) {
                if depth > 32 {
                    return;
                }
                for e in entries {
                    if let laml::syntax::StyleEntry::Import { path, .. } = e {
                        let p = normalized(&base.join(path));
                        if !p.starts_with(root) || !seen.insert(p.clone()) {
                            continue;
                        }
                        if let Ok(canonical) = p.canonicalize()
                            && canonical.starts_with(root)
                            && fs::metadata(&canonical).is_ok_and(|m| m.len() <= 16 * 1024 * 1024)
                            && let Ok(text) = fs::read_to_string(&canonical)
                            && let Ok(entries) = laml::parser::parse_stylesheet(&text)
                        {
                            imports(
                                &entries,
                                canonical.parent().unwrap_or(root),
                                root,
                                seen,
                                depth + 1,
                            );
                        }
                    }
                }
            }
            imports(&diagram.styling, &root, &root, &mut paths, 0);
        }
        let paths: Vec<_> = paths.into_iter().collect();
        let stamps = paths.iter().map(|p| stamp(p, &root)).collect();
        Self {
            paths,
            stamps,
            root,
        }
    }
    pub fn changed(&self) -> bool {
        self.paths
            .iter()
            .zip(&self.stamps)
            .any(|(p, old)| stamp(p, &self.root) != *old)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_empty_nested_and_missing_imports_and_fonts() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join("a.lamlstyle"),
            "styling {\n import \"empty.lamlstyle\"\n}\n",
        )
        .unwrap();
        fs::write(d.path().join("empty.lamlstyle"), "styling {\n}\n").unwrap();
        let source = "settings {\n font-file: \"font.ttf\"\n}\nsemantics {\n A\n}\nstyling {\n import \"a.lamlstyle\"\n}\n";
        let watch = Watch::new(source, &d.path().join("diagram.laml"));
        assert_eq!(watch.paths.len(), 3);
        assert!(!watch.changed());
        fs::write(d.path().join("font.ttf"), b"new").unwrap();
        assert!(watch.changed());
    }
}
