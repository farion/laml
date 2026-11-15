//! Editor services. UI state stays in the frontend; machine-local work lives here.
//!
//! Backend is the trusted blocking service boundary. `protocol` and
//! `async_client` provide owned messages and nonblocking frontend requests.
//! A network host still needs address scoping and session authorization.
use crate::{diagnostic::Diagnostic, output::Format, preview::Preview};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

pub mod async_client;
#[cfg(not(target_arch = "wasm32"))]
pub mod jobs;
#[cfg(not(target_arch = "wasm32"))]
mod local;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_transport;
pub mod protocol;
#[cfg(target_arch = "wasm32")]
pub mod web_transport;
#[cfg(not(target_arch = "wasm32"))]
pub use local::LocalBackend;
#[cfg(test)]
mod tests;

pub type Handle = Arc<dyn Backend>;

#[cfg(not(target_arch = "wasm32"))]
pub fn local() -> Handle {
    Arc::new(LocalBackend)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    /// Preview override: modern-dark with a transparent canvas.
    Editor,
    /// Preview override: modern (light) with a transparent canvas.
    EditorLight,
    /// Use the document's own theme, as for exports.
    Document,
}

impl Presentation {
    pub fn theme_override(self) -> Option<laml::theme::Theme> {
        match self {
            Self::Editor => Some(laml::theme::Theme::ModernDark),
            Self::EditorLight => Some(laml::theme::Theme::Modern),
            Self::Document => None,
        }
    }
}

/// Backend-owned dependency state. Frontends never inspect local timestamps.
pub trait DependencyWatch: Send {
    fn paths(&self) -> &[PathBuf];
    fn changed(&self) -> Result<bool, String>;
}

/// Blocking service implementation. Compilation and export rendering use a job
/// executor. Native dialogs/recovery are host adapters,
/// not backend operations, and are deliberately not part of this interface.
///
/// Checked writes retain the existing snapshot/no-clobber contract. Paths are
/// backend addresses: frontends must not canonicalize them on their own machine.
pub trait Backend: Send + Sync + Debug {
    fn compile(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
    ) -> Result<Preview, Diagnostic>;
    /// Reuse a server-owned preview only when node/layout inputs are unchanged.
    /// Backends without incremental compilation retain the full compiler path.
    fn compile_relations(
        &self,
        source: &str,
        path: &Path,
        presentation: Presentation,
        previous: &Preview,
    ) -> Result<Preview, Diagnostic> {
        let _ = previous;
        self.compile(source, path, presentation)
    }
    fn without_route(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        line: usize,
    ) -> Result<String, String>;
    fn route_overlay(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        route: &laml::route::Route,
        bounds: (f64, f64, f64, f64),
    ) -> Result<String, String>;
    fn source_targets(
        &self,
        diagram: &laml::syntax::Diagram,
        path: &Path,
        line: usize,
    ) -> Option<crate::selection::Selection>;
    fn render_export(&self, source: &str, path: &Path, format: Format) -> Result<Vec<u8>, String>;
    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String>;
    fn resolve_target(&self, path: &Path) -> Result<PathBuf, String>;
    fn read_source(&self, path: &Path) -> Result<String, String>;
    /// None means absent, not unreadable. Errors must not become permission to overwrite.
    fn source_snapshot(&self, path: &Path) -> Result<Option<String>, String>;
    fn write_source(&self, path: &Path, source: &str, expected: Option<&str>)
    -> Result<(), String>;
    fn export_target(
        &self,
        path: &Path,
        format: Format,
        protected: &[PathBuf],
    ) -> Result<PathBuf, String>;
    fn output_snapshot(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;
    fn write_output(&self, path: &Path, data: &[u8], expected: Option<&[u8]>)
    -> Result<(), String>;
    fn watch_dependencies(&self, source: &str, path: &Path) -> Box<dyn DependencyWatch>;
}
