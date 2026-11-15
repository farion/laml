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
    let (mut editor, ctx) = super::tests::ready();
    let script = Rc::new(RefCell::new(Script::default()));
    editor.workflow.service = Client::new(42, Box::new(Manual(script.clone()))).unwrap();
    (editor, ctx, script)
}
fn reply(script: &Rc<RefCell<Script>>, stamp: Stamp, result: Result<Reply, Failure>) {
    script
        .borrow_mut()
        .replies
        .push_back(codec::encode_response(&Response { stamp, result }).unwrap());
}
fn prepared(editor: &mut Editor, script: &Rc<RefCell<Script>>, exists: bool) {
    let stamp = script.borrow().requests[0].stamp;
    reply(
        script,
        stamp,
        Ok(Reply::ExportPrepared {
            ticket: 1,
            address: "server/out.svg".into(),
            exists,
        }),
    );
    editor.workflow_tick();
}
fn commit_stamp(script: &Rc<RefCell<Script>>) -> Stamp {
    script
        .borrow()
        .requests
        .iter()
        .find(|r| matches!(r.operation, Operation::CommitExport { .. }))
        .unwrap()
        .stamp
}
#[test]
fn typing_during_an_accepted_export_write_is_not_reported_as_a_cancelled_write() {
    let (mut editor, ctx, script) = editor();
    editor.start_export("server/out.svg".into(), Format::Svg);
    prepared(&mut editor, &script, false);
    assert!(editor.workflow.writing());
    let stamp = commit_stamp(&script);
    editor.document.replace_text("newer source".into()).unwrap();
    editor.request_tab_close(&ctx);
    assert!(!editor.allow_close);
    reply(&script, stamp, Ok(Reply::Address("server/out.svg".into())));
    editor.workflow_tick();
    editor.poll_file(&ctx);
    assert!(!editor.allow_close);
    assert_eq!(editor.document.source(), "newer source");
    assert!(editor.document.is_dirty());
    assert!(
        editor
            .workflow
            .status
            .as_ref()
            .unwrap()
            .contains("submitted revision")
    );
}
#[test]
fn export_write_failure_aborts_a_waiting_close_even_for_a_clean_document() {
    let (mut editor, ctx, script) = editor();
    editor.start_export("server/out.svg".into(), Format::Svg);
    prepared(&mut editor, &script, false);
    let stamp = commit_stamp(&script);
    editor.request_tab_close(&ctx);
    reply(
        &script,
        stamp,
        Err(Failure::backend("Output changed externally")),
    );
    editor.workflow_tick();
    editor.poll_file(&ctx);
    assert!(!editor.allow_close);
    assert!(editor.close_cancelled);
    assert!(editor.close_dialog_active());
    assert!(!editor.document.is_dirty());
}
#[test]
fn stale_preparation_is_cancelled_instead_of_committed() {
    let (mut editor, _, script) = editor();
    editor.start_export("server/out.svg".into(), Format::Svg);
    editor.document.replace_text("newer source".into()).unwrap();
    prepared(&mut editor, &script, false);
    assert!(
        !script
            .borrow()
            .requests
            .iter()
            .any(|r| matches!(r.operation, Operation::CommitExport { .. }))
    );
    let stamp = script
        .borrow()
        .requests
        .iter()
        .find(|r| matches!(r.operation, Operation::CancelExport { .. }))
        .unwrap()
        .stamp;
    reply(&script, stamp, Ok(Reply::Unit));
    editor.workflow_tick();
    assert!(!editor.workflow.busy());
    assert_eq!(editor.document.source(), "newer source");
}
