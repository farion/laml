use super::*;

const SOURCE: &str = "semantics {\n A\n}\n";

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        editor.poll_file(ctx);
        if editor.file_ui.task.is_none() && !editor.file_ui.busy() {
            break;
        }
        assert!(Instant::now() < end, "{:?}", editor.file_ui.error);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn polling_reloads_clean_source_and_rejects_old_worker_output() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.laml");
    std::fs::write(&path, SOURCE).unwrap();
    let mut editor = Editor::new(SOURCE.into(), Some(path.clone()));
    let ctx = egui::Context::default();
    let old = laml_ui::preview::build(SOURCE, &path).unwrap();
    let newer = "semantics {\n B\n}\n";
    std::fs::write(&path, newer).unwrap();
    editor.file_ui.checked = Instant::now() - Duration::from_secs(2);
    editor.file_events(&ctx);
    settle(&mut editor, &ctx);
    assert_eq!(editor.document.source(), newer);
    assert_eq!(editor.document.revision(), 1);
    struct Precomputed {
        preview: Option<Preview>,
        replies: std::collections::VecDeque<Vec<u8>>,
    }
    impl laml_ui::backend::async_client::Transport for Precomputed {
        fn send(&mut self, bytes: Vec<u8>) -> Result<(), laml_ui::backend::protocol::Failure> {
            use laml_ui::backend::protocol::{Reply, Response, codec};
            let request = codec::decode_request(&bytes)?;
            if let Some(preview) = self.preview.take() {
                self.replies.push_back(codec::encode_response(&Response {
                    stamp: request.stamp,
                    result: Ok(Reply::Compiled(Ok(Box::new(preview)))),
                })?);
            }
            Ok(())
        }
        fn poll(&mut self) -> Result<Option<Vec<u8>>, laml_ui::backend::protocol::Failure> {
            Ok(self.replies.pop_front())
        }
    }
    let service = laml_ui::backend::async_client::Client::new(
        9,
        Box::new(Precomputed {
            preview: Some(old),
            replies: Default::default(),
        }),
    )
    .unwrap();
    editor.compiler = worker::Client::with_client(editor.id, service);
    editor
        .compiler
        .send((0, SOURCE.into(), path, None))
        .unwrap();
    editor.compile(&ctx);
    assert!(editor.preview.is_none());
}

#[test]
fn opening_another_file_with_local_edits_requires_a_decision() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("other.laml");
    std::fs::write(&path, "semantics {\n Other\n}\n").unwrap();
    let mut editor = Editor::new(SOURCE.into(), None);
    let ctx = egui::Context::default();
    editor
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    editor.request_action(Action::Open(path.clone()), &ctx);
    assert!(matches!(editor.file_ui.pending, Some(Action::Open(_))));
    assert!(editor.document.source().contains("// mine"));
    // Explicit discard is the action the confirmation button dispatches.
    editor.perform(Action::Open(path), &ctx);
    settle(&mut editor, &ctx);
    assert!(editor.document.source().contains("Other"));
    assert!(!editor.document.is_dirty());
    assert!(!editor.document.undo());
}

#[test]
fn dirty_external_change_stays_in_memory_until_explicit_reload() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.laml");
    std::fs::write(&path, SOURCE).unwrap();
    let mut editor = Editor::new(SOURCE.into(), Some(path.clone()));
    let ctx = egui::Context::default();
    editor
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    let theirs = format!("{SOURCE}// theirs\n");
    std::fs::write(&path, &theirs).unwrap();
    editor.file_ui.checked = Instant::now() - Duration::from_secs(2);
    editor.file_events(&ctx);
    settle(&mut editor, &ctx);
    assert!(editor.file_ui.conflict);
    editor.save_file(&ctx);
    settle(&mut editor, &ctx);
    assert!(editor.file_ui.error.is_some());
    assert!(editor.document.source().contains("// mine"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), theirs);
    editor.perform(Action::Reload, &ctx);
    settle(&mut editor, &ctx);
    assert_eq!(editor.document.source(), theirs);
}

#[test]
fn recovery_uses_current_disk_as_baseline_without_overwriting_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.laml");
    let root = dir.path().join("recovery");
    let recovered = "semantics {\n Recovered\n}\n";
    let disk = "semantics {\n DiskChanged\n}\n";
    let writer = laml_ui::recovery::Store::open(root.clone()).unwrap();
    writer.sync(Some(&path), recovered, true).unwrap();
    drop(writer);
    std::fs::write(&path, disk).unwrap();
    let store = laml_ui::recovery::Store::open(root).unwrap();
    let record = store.pending().unwrap().remove(0);
    let mut editor = Editor::new(disk.into(), Some(path.clone()));
    editor.recovery.store = Some(store);
    let ctx = egui::Context::default();
    editor.request_recovery(record.clone(), &ctx);
    settle(&mut editor, &ctx);
    assert_eq!(editor.document.source(), recovered);
    assert!(editor.document.is_dirty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), disk);
    assert!(!record.file.exists());
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), disk);
    assert!(!editor.document.is_dirty());
}
#[test]
fn recovery_cannot_silently_replace_an_already_dirty_buffer() {
    let mut editor = Editor::new("initial".into(), None);
    editor.document.replace_text("mine".into()).unwrap();
    let ctx = egui::Context::default();
    editor.request_recovery(
        laml_ui::recovery::Record {
            file: "unused.json".into(),
            path: None,
            source: "recovered".into(),
        },
        &ctx,
    );
    assert_eq!(editor.document.source(), "mine");
    assert!(matches!(editor.file_ui.pending, Some(Action::Recover(_))));
}

#[test]
fn managed_new_and_open_are_nondestructive_workspace_requests() {
    let ctx = egui::Context::default();
    let mut editor = Editor::new(SOURCE.into(), None);
    editor.managed = true;
    editor
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    editor.new_tab(&ctx);
    assert!(matches!(
        editor.workspace_action.take(),
        Some(workspace::Action::New)
    ));
    let path = PathBuf::from("other.laml");
    editor.request_action(Action::Open(path.clone()), &ctx);
    assert!(
        matches!(editor.workspace_action.take(), Some(workspace::Action::Open(p)) if p == path)
    );
    assert!(editor.file_ui.pending.is_none());
    assert!(editor.document.source().contains("// mine"));
    assert!(editor.document.is_dirty());
}

#[test]
fn save_as_cannot_overwrite_a_document_open_in_another_tab() {
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other.laml");
    std::fs::write(&path, SOURCE).unwrap();
    let mut editor = Editor::new("semantics {\n Other\n}\n".into(), None);
    editor.managed = true;
    editor.other_paths.push(path.canonicalize().unwrap());
    editor.save_as(path.clone(), Some(SOURCE), &ctx);
    settle(&mut editor, &ctx);
    assert!(
        editor
            .file_ui
            .error
            .as_deref()
            .unwrap()
            .contains("another tab")
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), SOURCE);
    assert!(editor.file.path.is_none());
}

#[test]
fn failed_save_aborts_application_close_without_discarding_source() {
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("changed.laml");
    std::fs::write(&path, SOURCE).unwrap();
    let mut editor = Editor::new(SOURCE.into(), Some(path.clone()));
    editor.managed = true;
    editor
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    std::fs::write(&path, format!("{SOURCE}// theirs\n")).unwrap();
    editor.request_tab_close(&ctx);
    editor.file_ui.pending = None;
    editor.file_ui.after_save = Some(Action::Close);
    editor.save_file(&ctx);
    settle(&mut editor, &ctx);
    assert!(editor.close_cancelled);
    assert!(!editor.allow_close);
    assert!(editor.document.is_dirty());
    assert!(editor.document.source().contains("// mine"));
}
