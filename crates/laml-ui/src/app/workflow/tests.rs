use super::*;

pub(super) fn ready() -> (Editor, egui::Context) {
    let mut editor = Editor::new("semantics {\n A\n}\n".into(), None);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    super::super::tests::wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
fn wait(editor: &mut Editor, done: impl Fn(&Editor) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !done(editor) {
        assert!(Instant::now() < end, "{:?}", editor.workflow.status);
        editor.workflow_tick();
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn export_rejects_stale_source_and_external_output_replacements() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, _) = ready();
    let output = dir.path().join("export.svg");
    editor.start_export(output.clone(), Format::Svg);
    editor.document.replace_text("newer text".into()).unwrap();
    wait(&mut editor, |e| !e.workflow.busy());
    assert!(!output.exists());
    assert!(
        editor
            .workflow
            .status
            .as_ref()
            .unwrap()
            .contains("cancelled")
    );
    let (mut editor, _) = ready();
    std::fs::write(&output, "original").unwrap();
    editor.start_export(output.clone(), Format::Svg);
    wait(&mut editor, |e| {
        e.workflow
            .export
            .as_ref()
            .is_some_and(|job| matches!(job.stage, Stage::Confirm { .. }))
    });
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "original");
    std::fs::write(&output, "external").unwrap();
    editor.confirm_export();
    wait(&mut editor, |e| !e.workflow.busy());
    assert_eq!(std::fs::read_to_string(output).unwrap(), "external");
    assert!(editor.workflow.status.as_ref().unwrap().contains("failed"));
}
#[test]
fn cancelling_a_confirmed_ticket_never_writes_and_close_can_continue() {
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, ctx) = ready();
    let output = dir.path().join("export.svg");
    std::fs::write(&output, "original").unwrap();
    editor.start_export(output.clone(), Format::Svg);
    wait(&mut editor, |e| {
        e.workflow
            .export
            .as_ref()
            .is_some_and(|job| matches!(job.stage, Stage::Confirm { .. }))
    });
    editor.request_tab_close(&ctx);
    assert!(!editor.allow_close);
    wait(&mut editor, |e| !e.workflow.busy());
    editor.poll_file(&ctx);
    assert!(editor.allow_close);
    assert_eq!(std::fs::read_to_string(output).unwrap(), "original");
}
#[test]
fn disconnect_completes_export_as_a_failure_without_writing() {
    use laml_ui::backend::{
        async_client::{Client, Transport},
        protocol::{Failure, FailureCode},
    };
    struct Closed;
    impl Transport for Closed {
        fn send(&mut self, _bytes: Vec<u8>) -> Result<(), Failure> {
            Ok(())
        }
        fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure> {
            Err(Failure::new(FailureCode::Disconnected, "Backend stopped"))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let (mut editor, _) = ready();
    editor.workflow.service = Client::new(99, Box::new(Closed)).unwrap();
    let output = dir.path().join("export.svg");
    editor.start_export(output.clone(), Format::Svg);
    editor.workflow_tick();
    assert!(!editor.workflow.busy());
    assert!(!output.exists());
    assert!(
        editor
            .workflow
            .status
            .as_ref()
            .unwrap()
            .contains("Backend stopped")
    );
}

#[test]
fn failed_dependency_service_prevents_ticket_commit_and_output() {
    let (mut editor, _, backend) = super::super::backend_tests::ready();
    backend
        .watch_failure
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let output = PathBuf::from("failed-export.svg");
    editor.start_export(output, Format::Svg);
    wait(&mut editor, |e| !e.workflow.busy());
    assert!(backend.outputs.lock().unwrap().is_empty());
    assert!(
        editor
            .workflow
            .status
            .as_ref()
            .unwrap()
            .contains("Service unavailable")
    );
}
