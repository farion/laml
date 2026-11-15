use super::*;

const SOURCE: &str = "// Preserve this comment\nsemantics {\n A\n B\n}\n";

pub(super) fn wait_files(editor: &mut Editor, ctx: &egui::Context) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        editor.poll_file(ctx);
        if !editor.file_ui.busy() {
            break;
        }
        assert!(Instant::now() < end, "File operation timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn ready() -> (Editor, egui::Context) {
    let mut editor = Editor::new(SOURCE.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.rendered == Some(0));
    (editor, ctx)
}

pub(super) fn wait(editor: &mut Editor, ctx: &egui::Context, done: impl Fn(&Editor) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(editor) {
        assert!(
            Instant::now() < deadline,
            "worker timed out: {:?}",
            editor.error
        );
        editor.compile(ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn visual_multi_edit_renders_commits_and_undoes_as_one_step() {
    let (mut editor, ctx) = ready();
    editor.selection.ids = vec!["A".into(), "B".into()];
    editor.request_style("fill", Some("#123456".into()));
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
    assert_eq!(editor.rendered, Some(1));
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .all(|node| node.fill == "#123456")
    );
    assert!(editor.document.source().starts_with(SOURCE));
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), SOURCE);
    assert!(editor.document.redo());
    assert_eq!(editor.document.source().matches("fill: #123456").count(), 2);
}

#[test]
fn resetting_override_restores_rendered_default_and_is_undoable() {
    let (mut editor, ctx) = ready();
    let initial_fill = editor.preview.as_ref().unwrap().nodes[0].fill.clone();
    editor.selection.ids = vec!["A".into()];
    editor.request_style("fill", Some("#123456".into()));
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
    editor.request_style("fill", None);
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 2);
    assert_eq!(editor.preview.as_ref().unwrap().nodes[0].fill, initial_fill);
    assert!(!editor.document.source().contains("fill:"));
    editor.document.undo();
    assert!(editor.document.source().contains("fill: #123456"));
}

#[test]
fn typing_during_preflight_cancels_visual_change() {
    let (mut editor, ctx) = ready();
    editor.selection.ids = vec!["A".into()];
    editor.request_style("fill", Some("#123456".into()));
    let source = format!("{SOURCE}// newer text\n");
    editor.document.replace_text(source.clone()).unwrap();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.rendered == Some(1));
    assert_eq!(editor.document.source(), source);
    assert!(
        !editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.fill == "#123456")
    );
}

#[test]
fn invalid_source_keeps_last_picture_and_recovery_clears_warning() {
    let (mut editor, ctx) = ready();
    let old_image = editor.image.clone();
    editor
        .document
        .replace_text("semantics {\n A -> Missing\n}\n".into())
        .unwrap();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.error.is_some());
    assert_eq!(editor.image, old_image);
    assert_eq!(editor.rendered, Some(0));
    editor.selection.ids = vec!["A".into()];
    editor.request_style("fill", Some("#123456".into()));
    assert!(editor.pending_command.is_none());
    editor.document.undo();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.rendered == Some(2));
    assert!(editor.error.is_none());
}

#[test]
fn creation_commands_commit_and_undo_without_adding_layout_rules() {
    let (mut editor, ctx) = ready();
    editor.create_node("Created visually");
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
    assert_eq!(editor.selection.ids, ["Node1"]);
    editor.create_relation("A", "Node1");
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 2);
    let graph = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(graph.relations.len(), 1);
    assert_eq!(graph.relations[0].to, "Node1");
    assert!(graph.layout.is_empty());
    editor.document.undo();
    assert!(!editor.document.source().contains("@Relation1"));
    editor.document.undo();
    assert_eq!(editor.document.source(), SOURCE);
}

#[test]
fn an_empty_diagram_can_receive_its_first_node() {
    let mut editor = Editor::new("semantics {\n}\n".into(), None);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.rendered == Some(0));
    editor.create_node("First");
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
    assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), 1);
}

pub(super) fn canvas_frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
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

#[test]
fn actual_pointer_drag_creates_a_relation() {
    let (mut editor, ctx) = ready();
    editor.creation.connect = true;
    canvas_frame(&mut editor, &ctx, vec![]);
    let rect = editor.creation.canvas_rect.unwrap();
    let preview = editor.preview.as_ref().unwrap();
    let (left, top, right, bottom) = preview.bounds;
    let point = |id: &str| {
        let node = preview.nodes.iter().find(|node| node.id == id).unwrap();
        egui::pos2(
            rect.left()
                + ((node.x + node.width / 2.0 - left) / (right - left)) as f32 * rect.width(),
            rect.top()
                + ((node.y + node.height / 2.0 - top) / (bottom - top)) as f32 * rect.height(),
        )
    };
    let a = point("A");
    let b = point("B");
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(a), button(a, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(b)]);
    assert!(editor.creation.drag.is_some());
    canvas_frame(&mut editor, &ctx, vec![button(b, false)]);
    assert_eq!(editor.pending_command, Some(0));
    wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.relations[0].from,
        "A"
    );
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.relations[0].to,
        "B"
    );
}

#[test]
fn three_panes_render_without_native_window() {
    let (mut editor, ctx) = ready();
    editor.selection.ids = vec!["A".into(), "B".into()];
    for _ in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| {
                ui.columns(3, |columns| {
                    editor.source_editor(&mut columns[0]);
                    editor.canvas(&mut columns[1]);
                    editor.inspector(&mut columns[2]);
                });
            },
        );
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
    assert_eq!(editor.document.source(), SOURCE);
}

#[test]
fn three_pointer_drags_between_same_fixed_sides_are_accepted_and_undoable() {
    let (mut editor, ctx) = ready();
    for count in 1..=3 {
        editor.creation.connect = false;
        canvas_frame(&mut editor, &ctx, vec![]);
        let rect = editor.creation.canvas_rect.unwrap();
        let preview = editor.preview.as_ref().unwrap();
        let point = |id: &str, source: bool| {
            let node = preview.nodes.iter().find(|node| node.id == id).unwrap();
            super::connection_geometry::handles(preview, rect, node, source)
                .into_iter()
                .find(|h| h.dock.side == Some(if source { "right" } else { "left" }))
                .unwrap()
                .point
        };
        let (a, b) = (point("A", true), point("B", false));
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        canvas_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(a), button(a, true)],
        );
        canvas_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(b)]);
        assert!(editor.graph.gesture.is_some());
        canvas_frame(&mut editor, &ctx, vec![button(b, false)]);
        wait(&mut editor, &ctx, |e| e.document.revision() == count);
        assert!(editor.error.is_none(), "{:?}", editor.error);
        assert_eq!(
            editor.preview.as_ref().unwrap().routes.len(),
            count as usize
        );
    }
    let preview = editor.preview.as_ref().unwrap();
    assert_eq!(preview.diagram.relation_end_layout.len(), 6);
    for a in 0..3 {
        for b in a + 1..3 {
            assert_ne!(preview.routes[a].points, preview.routes[b].points);
        }
    }
    assert!(editor.document.undo());
    let source = editor.document.source();
    assert_eq!(laml::parser::parse(source).unwrap().relations.len(), 2);
}
