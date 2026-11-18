//! In-memory service proves frontend injection without client-side file access.
use super::*;
use laml_ui::backend::{Backend, DependencyWatch, LocalBackend, Presentation};
use laml_ui::{diagnostic::Diagnostic, output::Format};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

pub(super) const SOURCE: &str = "semantics {\n A\n B\n}\n";

#[derive(Debug, Default)]
pub(super) struct MemoryBackend {
    pub sources: Mutex<BTreeMap<PathBuf, String>>,
    pub outputs: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    pub calls: Mutex<Vec<&'static str>>,
    pub watch_failure: Arc<std::sync::atomic::AtomicBool>,
}
impl MemoryBackend {
    fn record(&self, operation: &'static str) {
        self.calls.lock().unwrap().push(operation);
    }
    pub fn address(path: &Path) -> PathBuf {
        if path.starts_with("server-only") {
            path.into()
        } else {
            Path::new("server-only").join(path)
        }
    }
}
struct NoDependencies(Arc<std::sync::atomic::AtomicBool>);
impl DependencyWatch for NoDependencies {
    fn paths(&self) -> &[PathBuf] {
        &[]
    }
    fn changed(&self) -> Result<bool, String> {
        if self.0.load(std::sync::atomic::Ordering::Relaxed) {
            Err("Service unavailable".into())
        } else {
            Ok(false)
        }
    }
}
impl Backend for MemoryBackend {
    fn compile(
        &self,
        source: &str,
        _path: &Path,
        presentation: Presentation,
    ) -> Result<Preview, Diagnostic> {
        self.record("compile");
        LocalBackend.compile(source, Path::new("test.laml"), presentation)
    }
    fn without_route(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        line: usize,
    ) -> Result<String, String> {
        self.record("without_route");
        LocalBackend.without_route(preview, source, path, line)
    }
    fn route_overlay(
        &self,
        preview: &Preview,
        source: &str,
        path: &Path,
        route: &laml::route::Route,
        bounds: (f64, f64, f64, f64),
    ) -> Result<String, String> {
        self.record("route_overlay");
        LocalBackend.route_overlay(preview, source, path, route, bounds)
    }
    fn source_targets(
        &self,
        diagram: &laml::syntax::Diagram,
        path: &Path,
        line: usize,
    ) -> Option<laml_ui::selection::Selection> {
        self.record("source_targets");
        LocalBackend.source_targets(diagram, path, line)
    }
    fn render_export(
        &self,
        _source: &str,
        _path: &Path,
        _format: Format,
    ) -> Result<Vec<u8>, String> {
        self.record("export");
        Ok(b"backend-rendered".to_vec())
    }
    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String> {
        self.record("resolve_existing");
        let path = Self::address(path);
        self.sources
            .lock()
            .unwrap()
            .contains_key(&path)
            .then_some(path)
            .ok_or("Not found on server".into())
    }
    fn resolve_target(&self, path: &Path) -> Result<PathBuf, String> {
        self.record("resolve_target");
        Ok(Self::address(path))
    }
    fn read_source(&self, path: &Path) -> Result<String, String> {
        self.record("read");
        self.source_snapshot(path)?
            .ok_or("Not found on server".into())
    }
    fn source_snapshot(&self, path: &Path) -> Result<Option<String>, String> {
        self.record("source_snapshot");
        Ok(self
            .sources
            .lock()
            .unwrap()
            .get(&Self::address(path))
            .cloned())
    }
    fn write_source(
        &self,
        path: &Path,
        source: &str,
        expected: Option<&str>,
    ) -> Result<(), String> {
        self.record("write_source");
        let mut files = self.sources.lock().unwrap();
        let path = Self::address(path);
        if files.get(&path).map(String::as_str) != expected {
            return Err("Server conflict".into());
        }
        files.insert(path, source.into());
        Ok(())
    }
    fn export_target(
        &self,
        path: &Path,
        _format: Format,
        protected: &[PathBuf],
    ) -> Result<PathBuf, String> {
        self.record("export_target");
        let path = Self::address(path);
        if protected.contains(&path) {
            return Err("Protected source".into());
        }
        Ok(path)
    }
    fn output_snapshot(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        self.record("output_snapshot");
        Ok(self
            .outputs
            .lock()
            .unwrap()
            .get(&Self::address(path))
            .cloned())
    }
    fn write_output(
        &self,
        path: &Path,
        data: &[u8],
        expected: Option<&[u8]>,
    ) -> Result<(), String> {
        self.record("write_output");
        let mut files = self.outputs.lock().unwrap();
        let path = Self::address(path);
        if files.get(&path).map(Vec::as_slice) != expected {
            return Err("Server output conflict".into());
        }
        files.insert(path, data.into());
        Ok(())
    }
    fn watch_dependencies(&self, _source: &str, _path: &Path) -> Box<dyn DependencyWatch> {
        self.record("watch");
        Box::new(NoDependencies(self.watch_failure.clone()))
    }
}

pub(super) fn ready() -> (Editor, egui::Context, Arc<MemoryBackend>) {
    let backend = Arc::new(MemoryBackend::default());
    let mut editor = Editor::with_backend(SOURCE.into(), None, backend.clone());
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    tests::wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx, backend)
}

#[test]
fn injected_backend_handles_compile_preflight_export_and_file_conflicts() {
    let (mut editor, ctx, backend) = ready();
    editor.selection.ids = vec!["A".into()];
    editor.request_style("fill", Some("#123456".into()));
    tests::wait(&mut editor, &ctx, |e| e.document.revision() == 1);
    assert!(
        backend
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| **c == "compile")
            .count()
            >= 2
    );
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), SOURCE);
    editor.changed = Instant::now() - Duration::from_secs(1);
    tests::wait(&mut editor, &ctx, |e| e.rendered == Some(2));

    let path = MemoryBackend::address(Path::new("diagram.laml"));
    editor.save_as(path.clone(), None, &ctx);
    tests::wait_files(&mut editor, &ctx);
    assert_eq!(backend.sources.lock().unwrap()[&path], SOURCE);
    editor
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    backend
        .sources
        .lock()
        .unwrap()
        .insert(path.clone(), "external".into());
    editor.save_file(&ctx);
    tests::wait_files(&mut editor, &ctx);
    assert!(editor.document.is_dirty());
    assert_eq!(backend.sources.lock().unwrap()[&path], "external");

    // A separate ready editor exports bytes supplied by the service, not by a
    // hidden call to the local compiler or local output writer.
    let (mut editor, _, backend) = ready();
    editor.workflow_tick();
    editor.start_export(PathBuf::from("result.svg"), Format::Svg);
    let output = MemoryBackend::address(Path::new("result.svg"));
    let end = Instant::now() + Duration::from_secs(10);
    while !backend.outputs.lock().unwrap().contains_key(&output) {
        assert!(Instant::now() < end, "{:?}", editor.workflow.status);
        editor.workflow_tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        backend.outputs.lock().unwrap()[&output],
        b"backend-rendered"
    );
    for expected in [
        "watch",
        "export_target",
        "output_snapshot",
        "export",
        "write_output",
    ] {
        assert!(backend.calls.lock().unwrap().contains(&expected));
    }
}

#[test]
fn injected_file_session_keeps_backend_on_reload_save_as_and_clone() {
    use laml_ui::{
        document::Document,
        files::{DiskStatus, FileSession},
    };
    let backend = Arc::new(MemoryBackend::default());
    let original = MemoryBackend::address(Path::new("original.laml"));
    backend
        .sources
        .lock()
        .unwrap()
        .insert(original.clone(), SOURCE.into());
    let (mut file, source) =
        FileSession::open_with_backend(Path::new("original.laml"), backend.clone()).unwrap();
    let mut document = Document::new(source);
    backend
        .sources
        .lock()
        .unwrap()
        .insert(original.clone(), "changed".into());
    assert_eq!(
        file.poll(&mut document, false).unwrap(),
        DiskStatus::Reloaded
    );
    document.replace_text("mine".into()).unwrap();
    file.save(&mut document).unwrap();
    assert!(!document.is_dirty());
    let target = MemoryBackend::address(Path::new("copy.laml"));
    file.save_as(&mut document, Path::new("copy.laml"), None)
        .unwrap();
    assert_eq!(file.path, Some(target.clone()));
    let mut copy = file.clone();
    backend
        .sources
        .lock()
        .unwrap()
        .insert(target, "reloaded".into());
    copy.reload(&mut document).unwrap();
    assert_eq!(document.source(), "reloaded");
    backend.sources.lock().unwrap().remove(&original);
    assert!(FileSession::open_with_backend(&original, backend).is_err());
}

#[test]
fn placement_preflight_uses_the_injected_backend_and_commits_one_undo_step() {
    let (mut editor, ctx, backend) = ready();
    let calls = backend.calls.lock().unwrap().len();
    let key = placement::Key {
        revision: editor.document.revision(),
        from: "A".into(),
        target: "B".into(),
        side: Some(laml::syntax::Placement::Right),
    };
    let end = Instant::now() + Duration::from_secs(10);
    while !editor.placement.ready() {
        assert!(Instant::now() < end, "{:?}", editor.placement.compiled);
        editor
            .placement
            .hover(Some(key.clone()), editor.document.source(), &editor.path);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(backend.calls.lock().unwrap()[calls..].contains(&"compile"));
    assert_eq!(editor.document.source(), SOURCE);
    editor.finish_placement(&ctx);
    assert_eq!(editor.document.revision(), 1);
    assert_eq!(editor.rendered, Some(1));
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), SOURCE);
}
