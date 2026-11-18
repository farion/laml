//! Native implementation, preserving the compiler and existing checked IO paths.
use super::*;
use crate::{dependencies::Watch, output, preview};

#[derive(Debug, Default)]
pub struct LocalBackend;

impl Backend for LocalBackend {
    fn compile(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
    ) -> Result<Preview, Diagnostic> {
        preview::build_presented(source, path, presentation)
    }
    fn sequence_targets(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
        action: &crate::notation_edit::sequence_targets::Action,
        targets: &[crate::notation_edit::sequence_targets::Target],
    ) -> Vec<crate::notation_edit::sequence_targets::Choice> {
        crate::notation_edit::sequence_target_geometry::build(
            source,
            path,
            presentation,
            action,
            targets,
        )
    }
    fn sequence_options(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
        queries: &[crate::notation_edit::sequence_options::Query],
        references: bool,
    ) -> Vec<crate::notation_edit::sequence_options::Choice> {
        crate::notation_edit::sequence_target_geometry::with_geometry(
            source,
            path,
            presentation,
            |geometry| {
                crate::notation_edit::sequence_options::build_with(
                    source,
                    queries,
                    references,
                    |text| geometry(text).is_some(),
                )
            },
        )
        .unwrap_or_default()
    }
    fn compile_relations(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
        previous: &Preview,
    ) -> Result<Preview, Diagnostic> {
        preview::build_relations(source, path, presentation.theme_override(), previous)
    }
    fn without_route(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        line: usize,
    ) -> Result<String, String> {
        preview::without_route(preview, source, path, line)
    }
    fn route_overlay(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        route: &laml::route::Route,
        bounds: (f64, f64, f64, f64),
    ) -> Result<String, String> {
        crate::gesture_preview::render(preview, source, path, route, bounds)
    }
    fn source_targets(
        &self,
        diagram: &laml::syntax::Diagram,
        path: &Path,
        line: usize,
    ) -> Option<crate::selection::Selection> {
        crate::source_link::targets(diagram, path, line)
    }
    fn render_export(&self, source: &str, path: &Path, format: Format) -> Result<Vec<u8>, String> {
        output::render(source, path, format)
    }
    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String> {
        path.canonicalize().map_err(|e| e.to_string())
    }
    fn resolve_target(&self, path: &Path) -> Result<PathBuf, String> {
        crate::files::absolute_target(path)
    }
    fn read_source(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
    }
    fn source_snapshot(&self, path: &Path) -> Result<Option<String>, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }
    fn write_source(
        &self,
        path: &Path,
        source: &str,
        expected: Option<&str>,
    ) -> Result<(), String> {
        crate::files::write_checked(path, source, expected)
    }
    fn export_target(
        &self,
        path: &Path,
        format: Format,
        protected: &[PathBuf],
    ) -> Result<PathBuf, String> {
        output::target(path, format, protected)
    }
    fn output_snapshot(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        output::snapshot(path)
    }
    fn write_output(
        &self,
        path: &Path,
        data: &[u8],
        expected: Option<&[u8]>,
    ) -> Result<(), String> {
        output::write(path, data, expected)
    }
    fn watch_dependencies(&self, source: &str, path: &Path) -> Box<dyn DependencyWatch> {
        Box::new(Watch::new(source, path))
    }
}

impl DependencyWatch for Watch {
    fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
    fn changed(&self) -> Result<bool, String> {
        Ok(self.changed())
    }
}
