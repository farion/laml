//! Freeze authorized source dependencies before calling any native compiler IO.
use super::workspace::{FILE_LIMIT, SOURCE_LIMIT, Workspace, extension, relative};
use crate::diagnostic::Diagnostic;
use laml::syntax::StyleEntry;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub struct Stage {
    pub temp: tempfile::TempDir,
    pub path: PathBuf,
    pub dependencies: Vec<PathBuf>,
}
impl Stage {
    pub fn new(workspace: &Workspace, source: &str, address: &Path) -> Result<Self, Diagnostic> {
        if workspace.limits && source.len() > SOURCE_LIMIT {
            return Err("Source exceeds host size limit".into());
        }
        let address = workspace
            .resolve(address, false)
            .map_err(Diagnostic::from)?;
        extension(&address, &["laml", "lamlstyle"]).map_err(Diagnostic::from)?;
        // Parse first: invalid settings must not reach with_fonts before authorization.
        let (diagram, nodes, relations) = crate::notation::inspect_at(source, &address)?;
        if workspace.limits && (nodes > 512 || relations > 2048) {
            return Err("Diagram exceeds host element limit".into());
        }
        let temp = tempfile::tempdir()
            .map_err(|_| Diagnostic::from("Could not create private compiler staging directory"))?;
        let path = temp.path().join(&address);
        std::fs::create_dir_all(path.parent().unwrap())
            .map_err(|_| Diagnostic::from("Could not prepare compiler staging directory"))?;
        let mut stage = Self {
            temp,
            path,
            dependencies: vec![],
        };
        let mut seen = BTreeSet::new();
        let mut total = source.len();
        let base = address.parent().unwrap_or(Path::new(""));
        for font in [
            diagram.settings.font_file,
            diagram.settings.font_italic_file,
        ]
        .into_iter()
        .flatten()
        {
            let path = relative(&base.join(font)).map_err(Diagnostic::from)?;
            extension(&path, &["ttf", "otf"]).map_err(Diagnostic::from)?;
            stage.copy(workspace, &path, &mut seen, &mut total)?;
        }
        stage.styles(workspace, &diagram.styling, base, &mut seen, &mut total, 0)?;
        Ok(stage)
    }
    fn copy(
        &mut self,
        workspace: &Workspace,
        path: &Path,
        seen: &mut BTreeSet<PathBuf>,
        total: &mut usize,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        if !seen.insert(path.to_owned()) {
            return Ok(None);
        }
        if workspace.limits && seen.len() > 128 {
            return Err("Too many compiler dependencies".into());
        }
        let bytes = workspace.read(path, FILE_LIMIT).map_err(Diagnostic::from)?;
        self.dependencies.push(path.to_owned());
        if let Some(bytes) = &bytes {
            *total += bytes.len();
            if workspace.limits && *total > 64 * 1024 * 1024 {
                return Err("Compiler dependencies exceed host size limit".into());
            }
            let target = self.temp.path().join(path);
            std::fs::create_dir_all(target.parent().unwrap())
                .and_then(|()| std::fs::write(target, bytes))
                .map_err(|_| Diagnostic::from("Could not stage compiler dependency"))?;
        }
        Ok(bytes)
    }
    fn styles(
        &mut self,
        workspace: &Workspace,
        entries: &[StyleEntry],
        base: &Path,
        seen: &mut BTreeSet<PathBuf>,
        total: &mut usize,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if workspace.limits && depth > 32 {
            return Err("Stylesheet import depth exceeds host limit".into());
        }
        for entry in entries {
            if let StyleEntry::Import { path, line } = entry {
                let path = relative(&base.join(path)).map_err(Diagnostic::from)?;
                extension(&path, &["lamlstyle"]).map_err(Diagnostic::from)?;
                if let Some(bytes) = self.copy(workspace, &path, seen, total)? {
                    let text = String::from_utf8(bytes).map_err(|_| {
                        Diagnostic::in_file(path.clone(), *line, "Stylesheet must be UTF-8")
                    })?;
                    let imported = laml::parser::parse_stylesheet(&text)
                        .map_err(|e| Diagnostic::in_file(path.clone(), e.line, e.message))?;
                    self.styles(
                        workspace,
                        &imported,
                        path.parent().unwrap_or(Path::new("")),
                        seen,
                        total,
                        depth + 1,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn prefixes(&self) -> Vec<String> {
        let mut roots = vec![self.temp.path().to_owned()];
        if let Ok(canonical) = self.temp.path().canonicalize() {
            roots.push(canonical);
        }
        let mut prefixes: Vec<_> = roots
            .into_iter()
            .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
            .collect();
        // On macOS, /var/... can be a suffix of canonical /private/var/....
        // Remove the longer canonical root first, or an absolute /private/... remains.
        prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
        prefixes.dedup();
        prefixes
    }
    pub fn text(&self, mut text: String) -> String {
        for prefix in self.prefixes() {
            text = text.replace(&prefix, "");
        }
        text
    }
    pub fn translate<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        value: T,
    ) -> Result<T, String> {
        let mut value =
            serde_json::to_value(value).map_err(|_| "Could not translate compiler model")?;
        let prefixes = self.prefixes();
        fn visit(value: &mut serde_json::Value, prefixes: &[String]) {
            match value {
                serde_json::Value::String(text) => {
                    let path = prefixes.iter().any(|prefix| text.starts_with(prefix));
                    for prefix in prefixes {
                        *text = text.replace(prefix, "");
                    }
                    if path {
                        *text = text.replace('\\', "/");
                    }
                }
                serde_json::Value::Array(items) => {
                    for item in items {
                        visit(item, prefixes)
                    }
                }
                serde_json::Value::Object(items) => {
                    for item in items.values_mut() {
                        visit(item, prefixes)
                    }
                }
                _ => {}
            }
        }
        visit(&mut value, &prefixes);
        serde_json::from_value(value).map_err(|_| "Could not translate compiler model".into())
    }
}
