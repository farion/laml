use super::*;
mod browser_drop_tests;
mod root_tests;
const SOURCE: &str =
    "semantics {\n A\n B\n C\n D\n}\nlayout {\n A < B // keep comment\n B < C\n A above D\n}\n";

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.rendered != Some(0) {
        assert!(
            Instant::now() < deadline,
            "initial compile failed: {:?}",
            editor.error
        );
        editor.compile(&ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
    (editor, ctx)
}
fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
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
    output.textures_delta.clear();
}
fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}
fn point(editor: &Editor, id: &str, right_side: bool) -> egui::Pos2 {
    let rect = editor.creation.canvas_rect.unwrap();
    let preview = editor.preview.as_ref().unwrap();
    let (left, top, right, bottom) = preview.bounds;
    let node = preview.nodes.iter().find(|node| node.id == id).unwrap();
    let scale = rect.width() / (right - left) as f32;
    let width = node.width as f32 * scale;
    let height = node.height as f32 * scale;
    let offset = if right_side {
        width - 22.0_f32.min(width / 3.0).min(height / 3.0) / 2.0
    } else {
        // The center is now the automatic connection grip. Arrange from the body.
        width * 0.2
    };
    egui::pos2(
        rect.left() + (node.x - left) as f32 * scale + offset,
        rect.top() + ((node.y + node.height / 2.0 - top) / (bottom - top)) as f32 * rect.height(),
    )
}

#[test]
fn pointer_reorder_shows_rule_changes_then_commits_one_undo_step() {
    let (mut editor, ctx) = ready(SOURCE);
    frame(&mut editor, &ctx, vec![]);
    let a = point(&editor, "A", false);
    let target = point(&editor, "C", true);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(a), button(a, true)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    assert_eq!(
        editor.placement.key.as_ref().unwrap().side,
        Some(Placement::Right)
    );
    assert_eq!(
        editor
            .placement
            .plan
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .removed,
        ["A < B"]
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !editor.placement.ready() {
        assert!(Instant::now() < deadline, "hover validation timed out");
        frame(&mut editor, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    let hover = editor
        .placement
        .compiled
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(hover.diagram.settings.theme, laml::theme::Theme::Modern);
    assert!(!hover.diagram.settings.transparent);
    assert!(hover.svg.contains("#ffffff"));
    assert_eq!(editor.document.source(), SOURCE);
    frame(&mut editor, &ctx, vec![button(target, false)]);
    assert_eq!(editor.document.revision(), 1);
    assert_eq!(editor.rendered, Some(1));
    let preview = editor.preview.as_ref().unwrap();
    assert_eq!(preview.diagram.settings.theme, laml::theme::Theme::Modern);
    assert!(!preview.diagram.settings.transparent);
    assert!(editor.document.source().contains(" B < C\n A above D\n"));
    assert!(editor.document.source().contains("A > C"));
    let nodes = &editor.preview.as_ref().unwrap().nodes;
    let a = nodes.iter().find(|n| n.id == "A").unwrap();
    let c = nodes.iter().find(|n| n.id == "C").unwrap();
    assert!(a.x >= c.x + c.width);
    editor.document.undo();
    assert_eq!(editor.document.source(), SOURCE);
}

#[test]
fn move_changes_alignment_but_preserves_other_axis_separation() {
    let source = "semantics {\n A\n B\n}\nlayout {\n A above B : align=center\n}\n";
    let (mut editor, ctx) = ready(source);
    let key = Key {
        revision: 0,
        from: "A".into(),
        target: "B".into(),
        side: Some(Placement::Right),
    };
    editor.placement.drag = Some(("A".into(), 0));
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.compiled.is_none() {
        assert!(Instant::now() < deadline);
        editor
            .placement
            .hover(Some(key.clone()), source, &editor.path);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(editor.placement.ready());
    editor.finish_placement(&ctx);
    assert!(editor.document.source().contains(" A above B\n"));
    assert!(editor.document.source().contains("A > B"));
    assert!(!editor.document.source().contains("align=center"));
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}

#[test]
fn releasing_before_validation_waits_without_changing_source_immediately() {
    let (mut editor, ctx) = ready(SOURCE);
    editor.placement.drag = Some(("A".into(), 0));
    editor.placement.hover(
        Some(Key {
            revision: 0,
            from: "A".into(),
            target: "C".into(),
            side: Some(Placement::Right),
        }),
        SOURCE,
        &editor.path,
    );
    editor.finish_placement(&ctx);
    assert_eq!(editor.document.source(), SOURCE);
    assert_eq!(editor.document.revision(), 0);
}

#[test]
fn stale_validated_drop_cannot_overwrite_new_text() {
    let (mut editor, ctx) = ready(SOURCE);
    let key = Key {
        revision: 0,
        from: "A".into(),
        target: "C".into(),
        side: Some(Placement::Right),
    };
    editor.placement.drag = Some(("A".into(), 0));
    editor.placement.hover(Some(key), SOURCE, &editor.path);
    let proposed = editor
        .placement
        .plan
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .source
        .clone();
    editor.placement.compiled = Some(laml_ui::preview::build(&proposed, &editor.path));
    assert!(editor.placement.ready());
    let newer = format!("{SOURCE}// edited while hovering\n");
    editor.document.replace_text(newer.clone()).unwrap();
    editor.finish_placement(&ctx);
    assert_eq!(editor.document.source(), newer);
}

#[test]
fn early_release_commits_after_validation_without_holding_the_pointer() {
    let (mut editor, ctx) = ready(SOURCE);
    editor.placement.drag = Some(("A".into(), 0));
    editor.placement.hover(
        Some(Key {
            revision: 0,
            from: "A".into(),
            target: "C".into(),
            side: Some(Placement::Right),
        }),
        SOURCE,
        &editor.path,
    );
    editor.finish_placement(&ctx);
    assert!(editor.placement.released);
    assert!(editor.placement.drag.is_none());
    assert_eq!(editor.document.source(), SOURCE);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.released {
        assert!(Instant::now() < deadline, "Pending drop did not finish");
        editor.poll_placement(&ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(editor.document.revision(), 1);
    assert!(editor.document.source().contains("A > C"));
    editor.document.undo();
    assert_eq!(editor.document.source(), SOURCE);
}
#[test]
fn container_body_accepts_pointer_drop_but_header_is_not_a_target() {
    for contents in ["", "  B\n"] {
        let source = format!("semantics {{\n A\n package P {{\n{contents} }}\n}}\n");
        let (mut editor, ctx) = ready(&source);
        frame(&mut editor, &ctx, vec![]);
        let a = point(&editor, "A", false);
        let preview = editor.preview.as_ref().unwrap();
        let canvas = editor.creation.canvas_rect.unwrap();
        let p = preview.nodes.iter().find(|n| n.id == "P").unwrap();
        let (x, y, right, _) = p.container_body().unwrap();
        // Just inside the body: the old arbitrary shrunken center excluded it.
        let target = connection_geometry::screen(preview, canvas, x + (right - x) * 0.8, y + 3.0);
        let header = connection_geometry::screen(preview, canvas, p.x + p.width / 2.0, p.y + 5.0);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(a), button(a, true)],
        );
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(header)]);
        assert!(editor.placement.drag.is_some());
        assert!(
            editor.placement.key.is_none(),
            "Package header must not be a drop target"
        );
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
        let key = editor
            .placement
            .key
            .as_ref()
            .expect("Whole body is a membership target");
        assert_eq!(key.target, "P");
        assert!(key.side.is_none());
        frame(&mut editor, &ctx, vec![button(target, false)]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while editor.placement.released {
            assert!(Instant::now() < deadline);
            frame(&mut editor, &ctx, vec![]);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(editor.document.revision(), 1, "{:?}", editor.command_error);
        assert!(
            editor
                .preview
                .as_ref()
                .unwrap()
                .diagram
                .nodes
                .iter()
                .any(|n| n.id == "P.A" && n.parent.as_deref() == Some("P"))
        );
        editor.document.undo();
        assert_eq!(editor.document.source(), source);
    }
}

#[test]
fn nested_package_header_does_not_drop_into_outer_container() {
    let source = "semantics {\n A\n package P {\n package Nested {\n }\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    frame(&mut editor, &ctx, vec![]);
    let a = point(&editor, "A", false);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let p = preview.nodes.iter().find(|n| n.id == "P.Nested").unwrap();
    let header = connection_geometry::screen(preview, canvas, p.x + p.width / 2.0, p.y + 5.0);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(a), button(a, true)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(header)]);
    assert!(editor.placement.drag.is_some());
    assert!(editor.placement.key.is_none());
    assert_eq!(editor.document.revision(), 0);
}

#[test]
fn invalid_membership_targets_are_filtered_without_compilation() {
    let source = "semantics {\n A\n package P {\n A\n }\n}\n";
    let (mut editor, _) = ready(source);
    let key = Key {
        revision: 0,
        from: "A".into(),
        target: "P".into(),
        side: None,
    };
    assert!(!editor.placement.allowed(&key, source));
    let key = Key {
        from: "P".into(),
        target: "P.A".into(),
        ..key
    };
    assert!(!editor.placement.allowed(&key, source));
}

#[test]
fn node3_body_drop_supersedes_its_outside_rule_in_one_undo() {
    let source = "semantics {\n package Group1(label=\"Package\") {\n }\n @Node3 class Node3(label=\"Node3\")\n}\nlayout {\n Node3 below Group1\n}\n";
    let (mut editor, ctx) = ready(source);
    frame(&mut editor, &ctx, vec![]);
    let from = point(&editor, "Node3", false);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let p = preview.nodes.iter().find(|n| n.id == "Group1").unwrap();
    let (x, y, right, bottom) = p.container_body().unwrap();
    let target =
        connection_geometry::screen(preview, canvas, (x + right) / 2.0, (y + bottom) / 2.0);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(from), button(from, true)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    assert_eq!(editor.placement.key.as_ref().unwrap().target, "Group1");
    frame(&mut editor, &ctx, vec![button(target, false)]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.released {
        assert!(Instant::now() < deadline);
        frame(&mut editor, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(editor.document.revision(), 1);
    assert!(!editor.document.source().contains("Node3 below Group1"));
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .nodes
            .iter()
            .any(|n| n.id == "Node3" && n.parent.as_deref() == Some("Group1"))
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn container_center_is_a_separate_membership_transaction() {
    let source = "semantics {\n A\n package P {\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.placement.drag = Some(("A".into(), 0));
    editor.placement.hover(
        Some(Key {
            revision: 0,
            from: "A".into(),
            target: "P".into(),
            side: None,
        }),
        source,
        &editor.path,
    );
    assert_eq!(editor.placement.moved_id.as_deref(), Some("P.A"));
    editor.finish_placement(&ctx);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.released {
        assert!(Instant::now() < deadline);
        editor.poll_placement(&ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(editor.selection.ids, ["P.A"]);
    assert_eq!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .nodes
            .iter()
            .find(|n| n.id == "P.A")
            .unwrap()
            .parent
            .as_deref(),
        Some("P")
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
