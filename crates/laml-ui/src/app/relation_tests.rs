use super::tests::{canvas_frame, wait};
use super::*;
use laml::{route::Point, syntax::EndPosition};
use laml_ui::relation_edit;

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
fn point(editor: &Editor, p: Point) -> egui::Pos2 {
    let rect = editor.creation.canvas_rect.unwrap();
    let (left, top, right, bottom) = editor.preview.as_ref().unwrap().bounds;
    egui::pos2(
        rect.left() + ((p.x - left) / (right - left)) as f32 * rect.width(),
        rect.top() + ((p.y - top) / (bottom - top)) as f32 * rect.height(),
    )
}
fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
const SOURCE: &str = "semantics {\n A\n B\n C\n @first A -> B : first\n A -> B : second\n}\n";

#[test]
fn parallel_arrows_are_selected_by_real_pointer_and_edit_independently() {
    let (mut editor, ctx) = ready(SOURCE);
    canvas_frame(&mut editor, &ctx, vec![]);
    let route = editor
        .preview
        .as_ref()
        .unwrap()
        .routes
        .iter()
        .find(|r| r.source_line == 6)
        .unwrap();
    let label = route.label.as_ref().unwrap();
    let pos = point(
        &editor,
        Point {
            x: label.x + label.width / 2.0,
            y: label.y + label.height / 2.0,
        },
    );
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(pos), button(pos, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![button(pos, false)]);
    assert_eq!(editor.selection.relations, [6]);
    assert_eq!(editor.source_jump, Some(6));
    editor.relation_edits("Label", |source, line| {
        relation_edit::label(source, line, "updated")
    });
    wait(&mut editor, &ctx, |e| e.document.revision() == 1);
    assert!(editor.document.source().contains("@first A -> B : first"));
    assert!(editor.document.source().contains("A -> B : \"updated\""));
    assert_eq!(editor.selection.relations, [6]);
    editor.document.undo();
    assert_eq!(editor.document.source(), SOURCE);
}

#[test]
fn endpoint_handle_drag_reconnects_with_independent_shift_release_side_choice() {
    for automatic in [false, true] {
        let (mut editor, ctx) = ready(SOURCE);
        editor.selection.click_relation(5, false);
        canvas_frame(&mut editor, &ctx, vec![]);
        let route = editor
            .preview
            .as_ref()
            .unwrap()
            .routes
            .iter()
            .find(|r| r.source_line == 5)
            .unwrap();
        let start = point(&editor, *route.points.last().unwrap());
        let c = editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "C")
            .unwrap();
        let target = point(
            &editor,
            Point {
                x: c.x + c.width / 2.0,
                y: c.y + c.height / 2.0,
            },
        );
        canvas_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        canvas_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
        assert_eq!(editor.relation_drag, Some((5, EndPosition::Target, 0)));
        canvas_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers {
                    shift: automatic,
                    ..Default::default()
                },
            }],
        );
        wait(&mut editor, &ctx, |e| e.document.revision() == 1);
        let expected = SOURCE.replace("@first A -> B", "@first A -> C");
        if automatic {
            assert_eq!(editor.document.source(), expected);
        } else {
            assert!(editor.document.source().starts_with(&expected));
        }
        let diagram = &editor.preview.as_ref().unwrap().diagram;
        assert!(diagram.layout.is_empty());
        assert_eq!(diagram.relation_end_layout.len(), usize::from(!automatic));
        if !automatic {
            let rule = &diagram.relation_end_layout[0];
            assert_eq!(rule.name, "first");
            assert_eq!(rule.end, EndPosition::Target);
            assert!(rule.side.is_some());
        }
        editor.document.undo();
        assert_eq!(editor.document.source(), SOURCE);
    }
}

#[test]
fn side_commands_remap_selection_when_layout_precedes_semantics() {
    let source = "layout {\n}\nsemantics {\n A\n B\n A -> B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click_relation(6, false);
    editor.relation_edits("Side", |source, line| {
        relation_edit::endpoint_side(source, line, EndPosition::Source, Some("right"))
    });
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.selection.relations, [7]);
    assert!(editor.document.source().contains("@Relation1 A -> B"));
    editor.relation_edits("Reset", |source, line| {
        relation_edit::endpoint_side(source, line, EndPosition::Source, None)
    });
    wait(&mut editor, &ctx, |e| e.document.revision() == 2);
    assert_eq!(editor.selection.relations, [6]);
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

#[test]
fn invalid_reconnect_and_stale_preflight_leave_source_unchanged() {
    let (mut editor, ctx) = ready(SOURCE);
    editor.selection.click_relation(5, false);
    editor.relation_edits("Reconnect", |source, line| {
        relation_edit::reconnect(source, line, EndPosition::Target, "Missing")
    });
    assert_eq!(editor.document.source(), SOURCE);
    assert!(editor.command_error.is_some());
    editor.relation_edits("Label", |source, line| {
        relation_edit::label(source, line, "obsolete")
    });
    editor
        .document
        .replace_loaded(SOURCE.replace("first", "different"));
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(1));
    assert!(!editor.document.source().contains("obsolete"));
    editor
        .document
        .replace_text("semantics {\n A -> Missing\n}\n".into())
        .unwrap();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.error.is_some());
    let old = editor.document.source().to_string();
    editor.relation_edits("Label", |source, line| {
        relation_edit::label(source, line, "blocked")
    });
    assert_eq!(editor.document.source(), old);
}

#[test]
fn multi_relation_styles_use_unique_names_and_undo_in_one_step() {
    let source = "styling {\n}\nsemantics {\n A\n B\n A -> B\n A -> B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click_relation(6, false);
    editor.selection.click_relation(7, true);
    editor.relation_property("color", Some("#123456"));
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.preview.as_ref().unwrap().routes.len(), 2);
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .routes
            .iter()
            .all(|r| r.color == "#123456")
    );
    assert!(editor.document.source().contains("@Relation1"));
    assert!(editor.document.source().contains("@Relation2"));
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}

#[test]
fn stale_picture_remains_selectable_but_its_source_positions_cannot_mutate_text() {
    let (mut editor, ctx) = ready(SOURCE);
    editor
        .document
        .replace_text("semantics {\n A -> Missing\n}\n".into())
        .unwrap();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.error.is_some());
    canvas_frame(&mut editor, &ctx, vec![]);
    let label = editor
        .preview
        .as_ref()
        .unwrap()
        .routes
        .iter()
        .find(|r| r.source_line == 5)
        .unwrap()
        .label
        .as_ref()
        .unwrap();
    let pos = point(
        &editor,
        Point {
            x: label.x + label.width / 2.0,
            y: label.y + label.height / 2.0,
        },
    );
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(pos), button(pos, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![button(pos, false)]);
    assert_eq!(editor.selection.relations, [5]);
    assert_eq!(editor.source_jump, None);
    let source = editor.document.source().to_string();
    editor.relation_edits("Blocked label", |source, line| {
        relation_edit::label(source, line, "blocked")
    });
    assert_eq!(editor.document.source(), source);
    assert!(!editor.can_edit_visually());
}
