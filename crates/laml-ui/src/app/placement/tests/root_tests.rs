use super::*;

fn drop_at(editor: &mut Editor, ctx: &egui::Context, id: &str, target: egui::Pos2) {
    let from = point(editor, id, false);
    frame(
        editor,
        ctx,
        vec![egui::Event::PointerMoved(from), button(from, true)],
    );
    frame(editor, ctx, vec![egui::Event::PointerMoved(target)]);
    assert!(
        editor.placement.key.is_some(),
        "Drop target must be available"
    );
    frame(editor, ctx, vec![button(target, false)]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.released {
        assert!(Instant::now() < deadline);
        frame(editor, ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(editor.document.revision(), 1, "{:?}", editor.command_error);
}
fn blank(editor: &Editor) -> egui::Pos2 {
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    [
        egui::pos2(980.0, 680.0),
        egui::pos2(20.0, 20.0),
        egui::pos2(980.0, 20.0),
    ]
    .into_iter()
    .find(|p| {
        preview
            .nodes
            .iter()
            .all(|n| !connection_geometry::node_rect(preview, canvas, n).contains(*p))
    })
    .unwrap()
}
#[test]
fn empty_canvas_drop_moves_to_root_preserving_relations_and_undo() {
    let source = "semantics {\n package P {\n A // keep\n }\n B\n P.A -> B : call\n}\n";
    let (mut editor, ctx) = ready(source);
    frame(&mut editor, &ctx, vec![]);
    let target = blank(&editor);
    drop_at(&mut editor, &ctx, "P.A", target);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert!(
        diagram
            .nodes
            .iter()
            .any(|n| n.id == "A" && n.parent.is_none())
    );
    assert_eq!(diagram.relations[0].from, "A");
    assert!(editor.document.source().contains("A // keep"));
    assert_eq!(editor.selection.ids, ["A"]);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn sibling_container_body_drop_moves_membership_directly() {
    let source = "semantics {\n package P {\n @A A\n }\n package Q {\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let q = preview.nodes.iter().find(|n| n.id == "Q").unwrap();
    let (x, y, right, bottom) = q.container_body().unwrap();
    let target =
        connection_geometry::screen(preview, canvas, (x + right) / 2.0, (y + bottom) / 2.0);
    drop_at(&mut editor, &ctx, "A", target);
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .nodes
            .iter()
            .any(|n| n.id == "A" && n.parent.as_deref() == Some("Q"))
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn root_drop_cannot_overwrite_a_root_identifier_collision() {
    let source = "semantics {\n A\n package P {\n A\n }\n}\n";
    let (mut editor, _) = ready(source);
    assert!(
        !editor
            .placement
            .allowed(&Key::root(0, "P.A".into()), source)
    );
    assert_eq!(editor.document.source(), source);
}
