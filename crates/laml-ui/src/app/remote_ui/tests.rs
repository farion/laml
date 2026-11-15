use super::*;
use laml_ui::backend::{
    async_client::Transport,
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
fn ready() -> (Editor, egui::Context, Rc<RefCell<Script>>) {
    let backend = std::sync::Arc::new(super::super::backend_tests::MemoryBackend::default());
    let mut editor =
        Editor::with_backend("semantics {\n A\n B\n A -> B\n}\n".into(), None, backend);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    super::super::tests::wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    let script = Rc::new(RefCell::new(Script::default()));
    editor.remote = Remote::new(Client::new(9, Box::new(Manual(script.clone()))).unwrap());
    let route = editor.preview.as_ref().unwrap().routes[0].clone();
    editor.graph.gesture = Some(connection_gesture::Gesture {
        revision: editor.document.revision(),
        canvas: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0)),
        kind: connection_gesture::Kind::Reconnect {
            line: route.source_line,
            end: laml::syntax::EndPosition::Target,
        },
        start: egui::Pos2::ZERO,
        end: egui::pos2(100.0, 100.0),
        start_direction: egui::Vec2::X,
        end_direction: -egui::Vec2::X,
        route,
    });
    (editor, ctx, script)
}
fn reply(script: &Rc<RefCell<Script>>, index: usize, reply: Reply) {
    let stamp = script.borrow().requests[index].stamp;
    script.borrow_mut().replies.push_back(
        codec::encode_response(&Response {
            stamp,
            result: Ok(reply),
        })
        .unwrap(),
    );
}
#[test]
fn late_base_after_cancel_never_hides_a_route_or_changes_source() {
    let (mut editor, ctx, script) = ready();
    let source = editor.document.source().to_owned();
    let line = editor.preview.as_ref().unwrap().routes[0].source_line;
    editor.request_gesture_base(line).unwrap();
    editor.end_connection(&ctx);
    reply(
        &script,
        0,
        Reply::GestureSvg {
            base: Some("late".into()),
            overlay: None,
        },
    );
    editor.poll_remote(&ctx);
    assert!(editor.graph.drag_image.is_none());
    assert!(editor.command_error.is_none());
    assert_eq!(editor.document.source(), source);
}
#[test]
fn stale_overlay_is_rejected_after_a_source_revision_change() {
    let (mut editor, ctx, script) = ready();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
    editor
        .remote
        .overlay(
            editor.id,
            editor.document.revision(),
            "frame".into(),
            rect,
            Operation::GestureRender {
                presentation: laml_ui::backend::Presentation::Document,
                source: editor.document.source().into(),
                address: "test.laml".into(),
                route: None,
                bounds: (0.0, 0.0, 400.0, 300.0),
                remove_line: None,
            },
        )
        .unwrap();
    editor.document.replace_text("newer buffer".into()).unwrap();
    reply(
        &script,
        0,
        Reply::GestureSvg {
            base: None,
            overlay: Some("late".into()),
        },
    );
    editor.poll_remote(&ctx);
    assert!(editor.graph.motion.uri.is_empty());
    assert_eq!(editor.document.source(), "newer buffer");
}
#[test]
fn accepted_base_and_overlay_can_arrive_out_of_order_without_touching_history() {
    let (mut editor, ctx, script) = ready();
    let line = editor.preview.as_ref().unwrap().routes[0].source_line;
    editor.request_gesture_base(line).unwrap();
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
    editor
        .remote
        .overlay(
            editor.id,
            editor.document.revision(),
            "frame".into(),
            rect,
            Operation::GestureRender {
                presentation: laml_ui::backend::Presentation::Document,
                source: editor.document.source().into(),
                address: "test.laml".into(),
                route: None,
                bounds: (0.0, 0.0, 400.0, 300.0),
                remove_line: None,
            },
        )
        .unwrap();
    reply(
        &script,
        1,
        Reply::GestureSvg {
            base: None,
            overlay: Some("overlay".into()),
        },
    );
    reply(
        &script,
        0,
        Reply::GestureSvg {
            base: Some("base".into()),
            overlay: None,
        },
    );
    editor.poll_remote(&ctx);
    assert!(editor.graph.drag_image.is_some());
    assert!(!editor.graph.motion.uri.is_empty());
    assert_eq!(editor.document.revision(), 0);
    assert_eq!(editor.document.history_usage().0, 0);
}
