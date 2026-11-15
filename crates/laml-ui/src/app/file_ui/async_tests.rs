use super::*;
use laml_ui::backend::{
    async_client::{Client, Transport},
    protocol::{Failure, Request, Response, codec},
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Default)]
struct Script {
    requests: Vec<Request>,
    replies: VecDeque<Vec<u8>>,
}
struct Manual(Rc<RefCell<Script>>);
impl Transport for Manual {
    fn send(&mut self, bytes: Vec<u8>) -> Result<(), Failure> {
        self.0
            .borrow_mut()
            .requests
            .push(codec::decode_request(&bytes)?);
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        Ok(self.0.borrow_mut().replies.pop_front())
    }
}
fn editor() -> (Editor, egui::Context, Rc<RefCell<Script>>) {
    let mut editor = Editor::new("semantics {\n A\n}\n".into(), None);
    editor.file = FileState::new(Some("server.laml".into()), editor.document.source());
    editor.path = "server.laml".into();
    let script = Rc::new(RefCell::new(Script::default()));
    editor.file_ui.service = Client::new(42, Box::new(Manual(script.clone()))).unwrap();
    (editor, egui::Context::default(), script)
}
fn reply(script: &Rc<RefCell<Script>>, index: usize, result: Result<Reply, Failure>) {
    let stamp = script.borrow().requests[index].stamp;
    script
        .borrow_mut()
        .replies
        .push_back(codec::encode_response(&Response { stamp, result }).unwrap());
}
#[test]
fn delayed_save_acknowledges_only_its_snapshot_and_keeps_newer_typing_dirty() {
    let (mut editor, ctx, script) = editor();
    editor.document.replace_text("submitted".into()).unwrap();
    editor.save_file(&ctx);
    assert!(editor.file_ui.writing());
    editor.document.replace_text("newer".into()).unwrap();
    reply(&script, 0, Ok(Reply::Address("server.laml".into())));
    editor.poll_file(&ctx);
    assert_eq!(editor.document.source(), "newer");
    assert!(editor.document.is_dirty());
    assert_eq!(editor.file.baseline(), "submitted");
    assert!(editor.document.undo());
    assert!(!editor.document.is_dirty());
}
#[test]
fn save_and_close_never_discards_text_typed_during_the_write() {
    let (mut editor, ctx, script) = editor();
    editor.document.replace_text("submitted".into()).unwrap();
    editor.file_ui.after_save = Some(Action::Close);
    editor.save_file(&ctx);
    editor.document.replace_text("newer".into()).unwrap();
    reply(&script, 0, Ok(Reply::Address("server.laml".into())));
    editor.poll_file(&ctx);
    assert!(!editor.allow_close);
    assert!(editor.close_cancelled);
    assert!(editor.document.is_dirty());
    assert_eq!(editor.document.source(), "newer");
}
#[test]
fn close_during_accepted_save_waits_and_rechecks_unsaved_changes() {
    let (mut editor, ctx, script) = editor();
    editor.document.replace_text("submitted".into()).unwrap();
    editor.save_file(&ctx);
    editor.request_action(Action::Close, &ctx);
    assert!(!editor.allow_close);
    assert!(editor.file_ui.deferred.is_some());
    editor.document.replace_text("newer".into()).unwrap();
    reply(&script, 0, Ok(Reply::Address("server.laml".into())));
    editor.poll_file(&ctx);
    assert!(matches!(editor.file_ui.pending, Some(Action::Close)));
    assert!(!editor.allow_close);
    assert_eq!(editor.document.source(), "newer");
}
#[test]
fn late_open_and_overwrite_inspection_cannot_replace_newer_source() {
    for inspect in [false, true] {
        let (mut editor, ctx, script) = editor();
        if inspect {
            editor.confirm_save_as("other.laml".into(), &ctx);
        } else {
            editor.perform(Action::Open("other.laml".into()), &ctx);
        }
        editor.document.replace_text("newer".into()).unwrap();
        let result = if inspect {
            Reply::SourceSnapshot {
                address: "other.laml".into(),
                source: Some("existing".into()),
            }
        } else {
            Reply::OpenedSource {
                address: "other.laml".into(),
                source: "external".into(),
            }
        };
        reply(&script, 0, Ok(result));
        editor.poll_file(&ctx);
        assert_eq!(editor.document.source(), "newer");
        assert_eq!(editor.file.path, Some("server.laml".into()));
        assert!(editor.file_ui.overwrite.is_none());
        assert_eq!(script.borrow().requests.len(), 1);
    }
}
#[test]
fn failed_save_never_marks_clean_or_runs_close_continuation() {
    let (mut editor, ctx, script) = editor();
    editor.document.replace_text("submitted".into()).unwrap();
    editor.file_ui.after_save = Some(Action::Close);
    editor.save_file(&ctx);
    reply(
        &script,
        0,
        Err(Failure::backend("Connection lost after save submission")),
    );
    editor.poll_file(&ctx);
    assert!(editor.document.is_dirty());
    assert!(!editor.allow_close);
    assert!(editor.file_ui.after_save.is_none());
    assert!(editor.file_ui.error.is_some());
}

#[test]
fn successful_poll_clears_only_its_transient_disconnect_error() {
    for foreground in [false, true] {
        let (mut editor, ctx, script) = editor();
        if foreground {
            editor.fail_file("Foreground save failure");
        }
        editor.start_file_task(
            Kind::Poll,
            Operation::OpenSource {
                address: "server.laml".into(),
            },
        );
        reply(&script, 0, Err(Failure::backend("Session disconnected")));
        editor.poll_file(&ctx);
        assert!(editor.file_ui.error.is_some());
        assert_eq!(editor.file_ui.poll_error, !foreground);
        editor.start_file_task(
            Kind::Poll,
            Operation::OpenSource {
                address: "server.laml".into(),
            },
        );
        reply(
            &script,
            1,
            Ok(Reply::OpenedSource {
                address: "server.laml".into(),
                source: editor.document.source().into(),
            }),
        );
        editor.poll_file(&ctx);
        assert!(!editor.file_ui.conflict);
        assert!(!editor.file_ui.poll_error);
        assert_eq!(
            editor.file_ui.error.as_deref(),
            if foreground {
                Some("Foreground save failure")
            } else {
                None
            }
        );
        assert_eq!(editor.file.path, Some("server.laml".into()));
    }
}
