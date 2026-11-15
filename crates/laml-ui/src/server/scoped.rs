//! Restricted backend; no compiler call sees the live host workspace path.
use super::{
    stage::Stage,
    workspace::{SOURCE_LIMIT, Workspace, extension},
};
use crate::{
    backend::{Backend, DependencyWatch, Presentation},
    diagnostic::Diagnostic,
    output::Format,
    preview::Preview,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
pub struct Scoped {
    pub workspace: Arc<Workspace>,
    prepared: Mutex<Option<(String, PathBuf, Arc<Stage>)>>,
}
impl std::fmt::Debug for Scoped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedWorkspaceBackend")
    }
}
impl Scoped {
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Self {
            workspace,
            prepared: Mutex::new(None),
        }
    }
    fn stage(&self, source: &str, path: &Path, refresh: bool) -> Result<Arc<Stage>, Diagnostic> {
        let mut current = self
            .prepared
            .lock()
            .map_err(|_| Diagnostic::from("Compiler state unavailable"))?;
        if !refresh
            && let Some((text, address, stage)) = &*current
            && text == source
            && address == path
        {
            return Ok(stage.clone());
        }
        let stage = Arc::new(Stage::new(&self.workspace, source, path)?);
        *current = Some((source.into(), path.into(), stage.clone()));
        Ok(stage)
    }
}
struct Watch {
    workspace: Arc<Workspace>,
    paths: Vec<PathBuf>,
    hashes: Vec<Option<[u8; 32]>>,
    error: Option<String>,
}
impl DependencyWatch for Watch {
    fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
    fn changed(&self) -> Result<bool, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        for (path, hash) in self.paths.iter().zip(&self.hashes) {
            let actual = self
                .workspace
                .read(path, super::workspace::FILE_LIMIT)?
                .map(|bytes| Sha256::digest(bytes).into());
            if &actual != hash {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
impl Backend for Scoped {
    fn compile(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
    ) -> Result<Preview, Diagnostic> {
        let stage = self.stage(source, path, true)?;
        let result = crate::backend::LocalBackend.compile(source, &stage.path, presentation);
        stage.translate(result).map_err(Diagnostic::from)?
    }
    fn compile_relations(
        &self, source: &str, path: &Path, presentation: Presentation, previous: &Preview,
    ) -> Result<Preview, Diagnostic> {
        let stage = self.stage(source, path, true)?;
        let result = crate::backend::LocalBackend.compile_relations(
            source, &stage.path, presentation, previous,
        );
        stage.translate(result).map_err(Diagnostic::from)?
    }
    fn without_route(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        line: usize,
    ) -> Result<String, String> {
        let stage = self.stage(source, path, false).map_err(|e| e.to_string())?;
        crate::backend::LocalBackend
            .without_route(preview, source, &stage.path, line)
            .map(|s| stage.text(s))
            .map_err(|s| stage.text(s))
    }
    fn route_overlay(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        route: &laml::route::Route,
        bounds: (f64, f64, f64, f64),
    ) -> Result<String, String> {
        let stage = self.stage(source, path, false).map_err(|e| e.to_string())?;
        crate::backend::LocalBackend
            .route_overlay(preview, source, &stage.path, route, bounds)
            .map(|s| stage.text(s))
            .map_err(|s| stage.text(s))
    }
    fn source_targets(
        &self,
        diagram: &laml::syntax::Diagram,
        _path: &Path,
        line: usize,
    ) -> Option<crate::selection::Selection> {
        crate::source_link::targets_in_diagram(diagram, line)
    }
    fn render_export(&self, source: &str, path: &Path, format: Format) -> Result<Vec<u8>, String> {
        let stage = self.stage(source, path, false).map_err(|e| e.to_string())?;
        crate::backend::LocalBackend
            .render_export(source, &stage.path, format)
            .map_err(|s| stage.text(s))
    }
    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String> {
        self.workspace
            .resolve(path, true)
            .map(|p| super::workspace::wire(&p))
    }
    fn resolve_target(&self, path: &Path) -> Result<PathBuf, String> {
        self.workspace
            .resolve(path, false)
            .map(|p| super::workspace::wire(&p))
    }
    fn read_source(&self, path: &Path) -> Result<String, String> {
        self.source_snapshot(path)?
            .ok_or("Workspace source does not exist".into())
    }
    fn source_snapshot(&self, path: &Path) -> Result<Option<String>, String> {
        extension(path, &["laml", "lamlstyle"])?;
        self.workspace
            .read(path, SOURCE_LIMIT)?
            .map(|bytes| String::from_utf8(bytes).map_err(|_| "Source must be UTF-8".into()))
            .transpose()
    }
    fn write_source(
        &self,
        path: &Path,
        source: &str,
        expected: Option<&str>,
    ) -> Result<(), String> {
        extension(path, &["laml", "lamlstyle"])?;
        if self.workspace.limits && source.len() > SOURCE_LIMIT {
            return Err("Source exceeds host size limit".into());
        }
        self.workspace
            .write(path, source.as_bytes(), expected.map(str::as_bytes))
    }
    fn export_target(
        &self,
        path: &Path,
        format: Format,
        protected: &[PathBuf],
    ) -> Result<PathBuf, String> {
        let mut path = path.to_owned();
        if path.extension().is_none() {
            path.set_extension(format.extension());
        }
        extension(&path, &[format.extension()])?;
        let path = self.workspace.resolve(&path, false)?;
        if protected.contains(&path) {
            return Err("Export cannot replace source or dependencies".into());
        }
        Ok(super::workspace::wire(&path))
    }
    fn output_snapshot(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        extension(path, &["svg", "png", "pdf"])?;
        self.workspace.read(path, 128 * 1024 * 1024)
    }
    fn write_output(
        &self,
        path: &Path,
        data: &[u8],
        expected: Option<&[u8]>,
    ) -> Result<(), String> {
        extension(path, &["svg", "png", "pdf"])?;
        self.workspace.write(path, data, expected)
    }
    fn watch_dependencies(&self, source: &str, path: &Path) -> Box<dyn DependencyWatch> {
        let mut watch = Watch {
            workspace: self.workspace.clone(),
            paths: vec![],
            hashes: vec![],
            error: None,
        };
        match self.stage(source, path, true) {
            Ok(stage) => {
                watch.paths = stage
                    .dependencies
                    .iter()
                    .map(|p| super::workspace::wire(p))
                    .collect();
                for path in &watch.paths {
                    match std::fs::read(stage.temp.path().join(path)) {
                        Ok(bytes) => watch.hashes.push(Some(Sha256::digest(bytes).into())),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            watch.hashes.push(None)
                        }
                        Err(_) => {
                            watch.error = Some("Could not read private staged dependency".into());
                            break;
                        }
                    }
                }
            }
            Err(e) => watch.error = Some(e.to_string()),
        }
        Box::new(watch)
    }
}
