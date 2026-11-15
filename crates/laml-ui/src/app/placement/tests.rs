use super::*;
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
        Placement::Right
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
    assert_eq!(
        preview.diagram.settings.theme,
        laml::theme::Theme::Modern
    );
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
        side: Placement::Right,
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
fn releasing_before_validation_never_changes_source() {
    let (mut editor, ctx) = ready(SOURCE);
    editor.placement.drag = Some(("A".into(), 0));
    editor.placement.hover(
        Some(Key {
            revision: 0,
            from: "A".into(),
            target: "C".into(),
            side: Placement::Right,
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
        side: Placement::Right,
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
