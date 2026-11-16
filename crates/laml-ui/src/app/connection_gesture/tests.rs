use super::*;
use crate::app::tests::wait;
use connection_geometry::{self as geometry, Dock};
use laml::syntax::{EndPosition, PortSide};

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    frame(&mut editor, &ctx, vec![]);
    (editor, ctx)
}
fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.canvas(ui),
    );
    out.textures_delta.clear();
    out
}
fn button(point: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
fn grip(editor: &Editor, id: &str, side: Option<&str>, source: bool) -> egui::Pos2 {
    let preview = editor.preview.as_ref().unwrap();
    let node = preview.nodes.iter().find(|n| n.id == id).unwrap();
    geometry::handles(preview, editor.creation.canvas_rect.unwrap(), node, source)
        .into_iter()
        .find(|h| h.dock.side == side)
        .unwrap()
        .point
}
fn body(editor: &Editor, id: &str) -> egui::Pos2 {
    let preview = editor.preview.as_ref().unwrap();
    let node = preview.nodes.iter().find(|n| n.id == id).unwrap();
    geometry::node_rect(preview, editor.creation.canvas_rect.unwrap(), node).center()
}
fn begin(
    editor: &mut Editor,
    ctx: &egui::Context,
    from: egui::Pos2,
    to: egui::Pos2,
) -> egui::FullOutput {
    begin_with_shift(editor, ctx, from, to, false)
}
fn begin_with_shift(
    editor: &mut Editor,
    ctx: &egui::Context,
    from: egui::Pos2,
    to: egui::Pos2,
    shift: bool,
) -> egui::FullOutput {
    frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(from),
            shifted_button(from, true, shift),
        ],
    );
    let out = frame(editor, ctx, vec![egui::Event::PointerMoved(to)]);
    assert!(
        editor.graph.gesture.is_some(),
        "connection gesture must own pointer"
    );
    assert!(editor.placement.drag.is_none());
    out
}
fn shifted_button(point: egui::Pos2, pressed: bool, shift: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers {
            shift,
            ..Default::default()
        },
    }
}
fn drop_at(editor: &mut Editor, ctx: &egui::Context, point: egui::Pos2, revision: u64) {
    drop_with_shift(editor, ctx, point, revision, false);
}
fn drop_with_shift(
    editor: &mut Editor,
    ctx: &egui::Context,
    point: egui::Pos2,
    revision: u64,
    shift: bool,
) {
    frame(editor, ctx, vec![shifted_button(point, false, shift)]);
    assert!(editor.graph.gesture.is_none());
    wait(editor, ctx, |e| {
        e.document.revision() == revision || e.pending_command.is_none()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.document.revision(), revision);
}

#[test]
fn shift_start_and_body_drop_leave_both_sides_automatic_and_undo_once() {
    let source = "semantics {\n A\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    let from = grip(&editor, "A", Some("right"), true);
    let to = body(&editor, "B");
    begin_with_shift(&mut editor, &ctx, from, to, true);
    assert_eq!(editor.document.source(), source);
    drop_with_shift(&mut editor, &ctx, to, 1, true);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(diagram.relations[0].from, "A");
    assert_eq!(diagram.relations[0].to, "B");
    assert!(diagram.relation_end_layout.is_empty());
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
    assert!(!editor.document.undo());
}

#[test]
fn all_four_source_buttons_can_set_a_side_while_the_target_stays_automatic() {
    for side in ["left", "right", "top", "bottom"] {
        let (mut editor, ctx) = ready("semantics {\n A\n B\n}\n");
        let from = grip(&editor, "A", Some(side), true);
        let to = body(&editor, "B");
        begin(&mut editor, &ctx, from, to);
        drop_with_shift(&mut editor, &ctx, to, 1, true);
        let rules = &editor.preview.as_ref().unwrap().diagram.relation_end_layout;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].end, EndPosition::Source);
        let actual = match rules[0].side.unwrap() {
            PortSide::Left => "left",
            PortSide::Right => "right",
            PortSide::Top => "top",
            PortSide::Bottom => "bottom",
        };
        assert_eq!(actual, side);
    }
}

#[test]
fn target_dock_fixes_the_target_side_and_type_is_frozen_at_drag_start() {
    let (mut editor, ctx) = ready("semantics {\n A\n B\n}\n");
    editor.graph.operator = "..>".into();
    let from = grip(&editor, "A", Some("right"), true);
    let to = grip(&editor, "B", Some("top"), false);
    begin_with_shift(&mut editor, &ctx, from, to, true);
    editor.graph.operator = "--".into();
    drop_at(&mut editor, &ctx, to, 1);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(diagram.relation_end_layout[0].end, EndPosition::Target);
    assert_eq!(diagram.relation_end_layout[0].side, Some(PortSide::Top));
    assert_eq!(
        laml_ui::relation_edit::operator(editor.document.source(), diagram.relations[0].line)
            .unwrap(),
        "..>"
    );
}

#[test]
fn ports_are_precise_start_and_drop_targets_not_their_owners() {
    for from_port in [false, true] {
        let source = "semantics {\n A {\n @P port Out\n }\n B {\n @Q port In\n }\n}\n";
        let (mut editor, ctx) = ready(source);
        let preview = editor.preview.as_ref().unwrap();
        let canvas = editor.creation.canvas_rect.unwrap();
        let port = |id: &str| {
            let p = preview
                .nodes
                .iter()
                .flat_map(|n| &n.ports)
                .find(|p| p.id == id)
                .unwrap();
            geometry::screen(preview, canvas, p.x, p.y)
        };
        let from = if from_port {
            port("P")
        } else {
            grip(&editor, "A", Some("right"), true)
        };
        let to = port("Q");
        begin_with_shift(&mut editor, &ctx, from, to, true);
        drop_at(&mut editor, &ctx, to, 1);
        let relation = &editor.preview.as_ref().unwrap().diagram.relations[0];
        assert_eq!(relation.from, if from_port { "P" } else { "A" });
        assert_eq!(relation.to, "Q");
        assert!(
            editor
                .preview
                .as_ref()
                .unwrap()
                .diagram
                .relation_end_layout
                .is_empty()
        );
    }
}

#[test]
fn self_connection_is_allowed_and_does_not_become_a_node_move() {
    let (mut editor, ctx) = ready("semantics {\n A\n}\n");
    let from = grip(&editor, "A", Some("right"), true);
    let to = grip(&editor, "A", Some("bottom"), false);
    begin(&mut editor, &ctx, from, to);
    drop_at(&mut editor, &ctx, to, 1);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(diagram.relations[0].from, "A");
    assert_eq!(diagram.relations[0].to, "A");
    assert!(diagram.layout.is_empty());
}

#[test]
fn escape_empty_drop_and_stale_revision_do_not_mutate_source() {
    for scenario in 0..3 {
        let source = "semantics {\n A\n B\n}\n";
        let (mut editor, ctx) = ready(source);
        let from = grip(&editor, "A", Some("right"), true);
        let to = body(&editor, "B");
        begin(&mut editor, &ctx, from, to);
        match scenario {
            0 => {
                frame(
                    &mut editor,
                    &ctx,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            1 => {
                let empty = egui::pos2(10.0, 10.0);
                frame(
                    &mut editor,
                    &ctx,
                    vec![egui::Event::PointerMoved(empty), button(empty, false)],
                );
            }
            _ => {
                editor
                    .document
                    .replace_text(format!("{source}// newer\n"))
                    .unwrap();
                editor.source_mutated();
                frame(&mut editor, &ctx, vec![button(to, false)]);
            }
        }
        assert!(editor.graph.gesture.is_none());
        assert!(editor.pending_command.is_none());
        assert_eq!(
            editor.document.source(),
            if scenario == 2 {
                format!("{source}// newer\n")
            } else {
                source.into()
            }
        );
    }
}

#[test]
fn either_endpoint_can_move_and_original_arrow_is_removed_during_drag() {
    for end in [EndPosition::Source, EndPosition::Target] {
        let source = "semantics {\n A\n B\n C\n @r A -> B : request\n}\nlayout {\n relation[r].source : side=right\n}\n";
        let (mut editor, ctx) = ready(source);
        let preview = editor.preview.as_ref().unwrap();
        let line = preview.routes[0].source_line;
        let p = laml_ui::relation_selection::endpoint(&preview.routes[0], end).unwrap();
        let from = geometry::screen(preview, editor.creation.canvas_rect.unwrap(), p.x, p.y);
        let to = body(&editor, "C");
        editor.selection.click_relation(line, false);
        begin(&mut editor, &ctx, from, to);
        let end_wait = Instant::now() + Duration::from_secs(10);
        while editor.graph.drag_image.is_none() {
            editor.poll_remote(&ctx);
            assert!(Instant::now() < end_wait, "{:?}", editor.command_error);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(editor.graph.drag_image.is_some());
        assert!(editor.relation_drag.is_some());
        assert_eq!(editor.preview.as_ref().unwrap().routes.len(), 1);
        assert_eq!(editor.document.source(), source);
        let g = editor.graph.gesture.as_ref().unwrap();
        assert_eq!(
            g.route.label.as_ref().map(|l| l.text.as_str()),
            Some("request")
        );
        drop_with_shift(&mut editor, &ctx, to, 1, true);
        assert!(editor.graph.drag_image.is_none());
        let diagram = &editor.preview.as_ref().unwrap().diagram;
        let relation = &diagram.relations[0];
        assert_eq!(
            relation.from,
            if end == EndPosition::Source { "C" } else { "A" }
        );
        assert_eq!(
            relation.to,
            if end == EndPosition::Target { "C" } else { "B" }
        );
        assert_eq!(relation.label.as_deref(), Some("request"));
        if end == EndPosition::Source {
            assert!(diagram.relation_end_layout.is_empty());
        }
        editor.document.undo();
        assert_eq!(editor.document.source(), source);
    }
}

#[test]
fn shift_at_press_and_release_independently_controls_each_end() {
    for start_auto in [false, true] {
        for end_auto in [false, true] {
            let (mut editor, ctx) = ready("semantics {\n A\n B\n}\n");
            let preview = editor.preview.as_ref().unwrap();
            let canvas = editor.creation.canvas_rect.unwrap();
            assert_eq!(
                geometry::handles(preview, canvas, &preview.nodes[0], true).len(),
                4
            );
            assert!(geometry::source_at(preview, canvas, body(&editor, "A")).is_none());
            let from = grip(&editor, "A", Some("right"), true);
            let to = grip(&editor, "B", Some("left"), false);
            // The motion frame has Shift released: the press choice must stay latched.
            begin_with_shift(&mut editor, &ctx, from, to, start_auto);
            let Kind::New { from, .. } = &editor.graph.gesture.as_ref().unwrap().kind else {
                panic!("New arrow expected")
            };
            assert_eq!(from.side.is_none(), start_auto);
            drop_with_shift(&mut editor, &ctx, to, 1, end_auto);
            let rules = &editor.preview.as_ref().unwrap().diagram.relation_end_layout;
            assert_eq!(
                rules
                    .iter()
                    .find(|r| r.end == EndPosition::Source)
                    .and_then(|r| r.side),
                (!start_auto).then_some(PortSide::Right)
            );
            assert_eq!(
                rules
                    .iter()
                    .find(|r| r.end == EndPosition::Target)
                    .and_then(|r| r.side),
                (!end_auto).then_some(PortSide::Left)
            );
        }
    }
}

#[test]
fn body_drop_is_fixed_by_default_and_shift_preserves_named_ports() {
    let (editor, _) = ready("semantics {\n A\n B {\n @P port Input\n }\n}\n");
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let body = body(&editor, "B");
    assert!(
        geometry::target_at(preview, canvas, body, false)
            .unwrap()
            .side
            .is_some()
    );
    assert!(
        geometry::target_at(preview, canvas, body, true)
            .unwrap()
            .side
            .is_none()
    );
    let port = preview
        .nodes
        .iter()
        .flat_map(|n| &n.ports)
        .find(|p| p.id == "P")
        .unwrap();
    let point = geometry::screen(preview, canvas, port.x, port.y);
    assert_eq!(
        geometry::target_at(preview, canvas, point, true)
            .unwrap()
            .id,
        "P"
    );
}

#[test]
fn stale_drop_targets_are_not_applied_even_when_called_directly() {
    let (mut editor, _) = ready("semantics {\n A\n B\n}\n");
    let gesture = Gesture {
        revision: 0,
        canvas: egui::Rect::NOTHING,
        kind: Kind::New {
            from: Dock {
                id: "A".into(),
                owner: "A".into(),
                side: None,
                point: egui::Pos2::ZERO,
            },
            operator: "->".into(),
        },
        start: egui::Pos2::ZERO,
        end: egui::Pos2::ZERO,
        start_direction: egui::Vec2::ZERO,
        end_direction: egui::Vec2::ZERO,
        route: laml_ui::gesture_preview::draft_route(
            editor.preview.as_ref().unwrap(),
            &laml::parser::parse("semantics {\n A\n B\n A -> B\n}\n")
                .unwrap()
                .relations[0],
        ),
    };
    editor
        .document
        .replace_text("semantics {\n A\n B\n C\n}\n".into())
        .unwrap();
    editor.commit_connection(
        &gesture,
        &Dock {
            id: "B".into(),
            owner: "B".into(),
            side: None,
            point: egui::Pos2::ZERO,
        },
    );
    assert!(editor.pending_command.is_none());
    assert_eq!(editor.document.revision(), 1);
}
