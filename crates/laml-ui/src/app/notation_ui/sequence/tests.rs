use super::*;
const SOURCE: &str =
    "settings {\n type: sequence\n}\nsemantics {\n A\n B\n B+\n A -> B: Work\n B-\n}\n";
pub(super) fn editor() -> (egui::Context, Editor, egui::Rect) {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    let mut editor = Editor::new(SOURCE.into(), None);
    let preview =
        laml_ui::preview::build(SOURCE, std::path::Path::new("sequence-ui.laml")).unwrap();
    let image = egui::Rect::from_min_size(
        egui::pos2(100.0, 100.0),
        egui::vec2(
            (preview.bounds.2 - preview.bounds.0) as f32,
            (preview.bounds.3 - preview.bounds.1) as f32,
        ),
    );
    editor.publish_preview(&ctx, preview);
    (ctx, editor, image)
}
pub(super) fn frame(
    ctx: &egui::Context,
    editor: &mut Editor,
    image: egui::Rect,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            let response = ui.interact(
                ui.max_rect(),
                ui.scope_id().with("sequence-test"),
                egui::Sense::click_and_drag(),
            );
            editor.sequence_canvas(ui, image, &response, false);
        },
    );
    output.textures_delta.clear();
    output
}
pub(super) fn pointer(at: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(at),
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        },
    ]
}
#[test]
fn participant_pluses_insert_directly_on_both_sides() {
    for left in [true, false] {
        let (ctx, mut editor, image) = editor();
        let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
        let at=scene.areas.iter().find(|a|matches!(&a.hit,Hit::ParticipantPlus {neighbor,left:side} if neighbor=="A" && *side==left)).unwrap().rect.center();
        frame(
            &ctx,
            &mut editor,
            image,
            vec![egui::Event::PointerMoved(at)],
        );
        frame(&ctx, &mut editor, image, pointer(at, true));
        frame(&ctx, &mut editor, image, pointer(at, false));
        assert!(
            editor.pending_command.is_some(),
            "Direct insertion submits preflight"
        );
        assert!(editor.notation_ui.sequence.picker.is_none());
        assert!(editor.notation_ui.sequence.insert.is_none());
        assert!(!dialogs::blocked(&ctx));
    }
}
#[test]
fn plus_and_sensitive_lifeline_start_have_different_actions() {
    let (ctx, mut editor, image) = editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let point = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap();
    assert!(matches!(scene.hit(point.at), Some(Hit::Origin(_))));
    let at = point.at + egui::vec2(22.0, 0.0);
    assert!(matches!(scene.hit(at), Some(Hit::Insert(_))));
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::PointerMoved(at)],
    );
    frame(&ctx, &mut editor, image, pointer(at, true));
    frame(&ctx, &mut editor, image, pointer(at, false));
    assert!(editor.notation_ui.sequence.insert.is_some());
    assert!(editor.notation_ui.sequence.gesture.is_none());
}
#[test]
fn escape_cancels_a_message_drag_without_any_source_change() {
    let (ctx, mut editor, image) = editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let at = scene
        .areas
        .iter()
        .find(|a| matches!(&a.hit,Hit::Origin(p) if p.dock.owner=="A"))
        .unwrap()
        .rect
        .center();
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::PointerMoved(at)],
    );
    frame(&ctx, &mut editor, image, pointer(at, true));
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::PointerMoved(at + egui::vec2(30.0, 0.0))],
    );
    assert!(editor.notation_ui.sequence.gesture.is_some());
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert!(editor.notation_ui.sequence.gesture.is_none());
    assert!(editor.notation_ui.sequence.pending_targets.is_none());
    assert!(editor.notation_ui.sequence.press.is_none());
    assert_eq!(editor.document.source(), SOURCE);
}
#[test]
fn nested_bar_has_its_own_edges_and_reconnection_destinations() {
    let source = SOURCE.replace(" A -> B: Work", " B+\n A -> B: Work\n B-");
    let preview = laml_ui::preview::build(&source, std::path::Path::new("nested.laml")).unwrap();
    let scene = Scene::new(
        &preview,
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(
                (preview.bounds.2 - preview.bounds.0) as f32,
                (preview.bounds.3 - preview.bounds.1) as f32,
            ),
        ),
    );
    assert_eq!(scene.bars.len(), 2);
    for bar in &scene.bars {
        for lower in [true, false] {
            assert!(scene.areas.iter().any(
                |a| matches!(&a.hit,Hit::Resize(b,l) if b.region.line==bar.region.line && *l==lower)
            ));
        }
    }
    let message = scene
        .regions
        .iter()
        .find(|r| r.kind == "message" && !r.path.is_empty())
        .unwrap();
    let destinations = proposals::destinations(
        &source,
        &scene,
        &Hit::Endpoint {
            message: message.clone(),
            source: false,
        },
        "->",
    );
    assert!(!destinations.is_empty());
    assert!(
        destinations.iter().all(|d| laml::sequence::parse(
            &edit::apply(&source, &d.proposal.edits).unwrap()
        )
        .is_ok())
    );
}

#[test]
fn empty_sequence_has_finite_canvas_bounds_for_the_add_participant_button() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n}\n";
    let p = laml_ui::preview::build(s, std::path::Path::new("empty.laml")).unwrap();
    assert!(p.bounds.2 > p.bounds.0 && p.bounds.3 > p.bounds.1);
    assert!(p.bounds.0.is_finite() && p.bounds.2.is_finite());
}

#[test]
fn insertion_plus_opens_a_local_nonmodal_menu_and_note_keeps_its_dock() {
    let (ctx, mut editor, image) = editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let point = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap()
        .clone();
    let at = point.at + egui::vec2(22.0, 0.0);
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::PointerMoved(at)],
    );
    frame(&ctx, &mut editor, image, pointer(at, true));
    frame(&ctx, &mut editor, image, pointer(at, false));
    let output = wait_for_menu(&ctx, &mut editor, image, "Note");
    let texts = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !texts.iter().any(|t| t.galley.job.text == "Add construct"),
        "No dialog title"
    );
    let note = texts
        .iter()
        .find(|t| t.galley.job.text.ends_with("  Note"))
        .expect("Note menu item");
    assert!(note.pos.x >= at.x - 10.0 && note.pos.x < at.x + 230.0);
    assert!(
        (note.pos.y - at.y).abs() < 350.0,
        "Menu must be attached to the plus, not centered"
    );
    assert!(!dialogs::blocked(&ctx));
    assert_eq!(editor.document.source(), SOURCE);
    let click = note.pos + note.galley.size() * 0.5;
    frame(&ctx, &mut editor, image, pointer(click, true));
    frame(&ctx, &mut editor, image, pointer(click, false));
    assert!(editor.notation_ui.sequence.insert.is_none());
    assert!(
        editor.pending_command.is_some(),
        "Note is inserted without a form"
    );
    assert!(editor.notation_ui.sequence.picker.is_none());
    assert!(!dialogs::blocked(&ctx));
}
#[test]
fn insertion_menu_dismisses_outside_or_on_escape_without_editing() {
    for escape in [false, true] {
        let (ctx, mut editor, image) = editor();
        let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
        let at = scene
            .areas
            .iter()
            .find(|a| matches!(a.hit, Hit::Insert(_)))
            .unwrap()
            .rect
            .center();
        frame(
            &ctx,
            &mut editor,
            image,
            vec![egui::Event::PointerMoved(at)],
        );
        frame(&ctx, &mut editor, image, pointer(at, true));
        frame(&ctx, &mut editor, image, pointer(at, false));
        frame(&ctx, &mut editor, image, vec![]);
        assert!(editor.notation_ui.sequence.insert.is_some());
        if escape {
            frame(
                &ctx,
                &mut editor,
                image,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
            );
        } else {
            let outside = at + egui::vec2(350.0, 200.0);
            frame(&ctx, &mut editor, image, pointer(outside, true));
            frame(&ctx, &mut editor, image, pointer(outside, false));
        }
        assert!(editor.notation_ui.sequence.insert.is_none());
        assert!(editor.notation_ui.sequence.picker.is_none());
        assert!(editor.notation_ui.sequence.press.is_none());
        assert_eq!(editor.document.source(), SOURCE);
    }
}

#[test]
fn arrow_paints_before_target_reply_and_a_fast_release_is_not_lost() {
    use laml_ui::backend::{
        async_client::{Client, Transport},
        protocol::{self, Operation, Reply, Response},
    };
    use laml_ui::notation_edit::sequence_targets::{Action, Target};
    struct Deferred(std::sync::mpsc::Receiver<Vec<u8>>);
    impl Transport for Deferred {
        fn send(&mut self, _: Vec<u8>) -> Result<(), protocol::Failure> {
            Ok(())
        }
        fn poll(&mut self) -> Result<Option<Vec<u8>>, protocol::Failure> {
            Ok(self.0.try_recv().ok())
        }
    }
    let (ctx, mut editor, image) = editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let origin = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap()
        .clone();
    let target = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "B" && matches!(p.dock.boundary, Some(Boundary::Start(_))))
        .unwrap()
        .clone();
    let action = Action::Create {
        origin: origin.dock.clone(),
        operator: "->".into(),
    };
    let targets = vec![Target {
        dock: target.dock.clone(),
        same_row: false,
        straighten: false,
    }];
    let (tx, rx) = std::sync::mpsc::channel();
    let mut service = Client::new(77, Box::new(Deferred(rx))).unwrap();
    let stamp = service
        .submit(
            editor.id,
            editor.document.revision(),
            Operation::SequenceTargets {
                source: SOURCE.into(),
                address: "diagram.laml".into(),
                presentation: laml_ui::backend::Presentation::Document,
                action: action.clone(),
                targets: targets.clone(),
            },
        )
        .unwrap();
    editor.notation_ui.sequence.revision = Some(editor.document.revision());
    editor.notation_ui.sequence.pending_targets = Some(message_targets::Pending {
        service,
        stamp,
        points: vec![target.clone()],
        action: action.clone(),
    });
    editor.notation_ui.sequence.gesture = Some(gestures::Gesture {
        hit: Hit::Origin(origin),
        destinations: vec![],
        hot: None,
    });
    frame(&ctx, &mut editor, image, pointer(target.at, true));
    let output = frame(&ctx, &mut editor, image, vec![]);
    assert!(editor.notation_ui.sequence.pending_targets.is_some());
    assert!(
        editor
            .notation_ui
            .sequence
            .gesture
            .as_ref()
            .unwrap()
            .destinations
            .is_empty()
    );
    assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Path(p) if p.points.len()>=2 && p.points.last().unwrap().distance(target.at)<0.01)),"Arrow must already be painted with no backend reply");
    frame(&ctx, &mut editor, image, pointer(target.at, false));
    assert_eq!(editor.notation_ui.sequence.released, Some(target.at));
    assert_eq!(editor.document.source(), SOURCE);
    let choices = laml_ui::notation_edit::sequence_targets::build(
        laml_ui::backend::local().as_ref(),
        SOURCE,
        std::path::Path::new("diagram.laml"),
        laml_ui::backend::Presentation::Document,
        &action,
        &targets,
    );
    assert!(!choices.is_empty());
    tx.send(
        protocol::codec::encode_response(&Response {
            stamp,
            result: Ok(Reply::SequenceTargets(choices)),
        })
        .unwrap(),
    )
    .unwrap();
    frame(&ctx, &mut editor, image, vec![]);
    assert!(editor.notation_ui.sequence.gesture.is_none());
    assert!(editor.notation_ui.sequence.pending_targets.is_none());
    assert!(editor.notation_ui.sequence.released.is_none());
    assert!(
        editor.compiler.busy(),
        "Validated fast drop must reach ordinary revision-bound command preflight"
    );
}

pub(super) fn wait_for_menu(
    ctx: &egui::Context,
    editor: &mut Editor,
    image: egui::Rect,
    title: &str,
) -> egui::FullOutput {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let output = frame(ctx, editor, image, vec![]);
        if output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.ends_with(&format!("  {title}")))) { return output; }
        assert!(
            std::time::Instant::now() < deadline,
            "Timed out waiting for {title}: expanded={}, insert={}, texts={:?}",
            editor.notation_ui.sequence.expanded,
            editor.notation_ui.sequence.insert.is_some(),
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(&t.galley.job.text),
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
