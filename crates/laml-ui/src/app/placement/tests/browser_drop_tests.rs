use super::*;

#[test]
fn screenshot_browser_pointer_drop_into_its_relation_target_commits_and_undoes() {
    let source = include_str!("../../../../../../tests/fixtures/gne-browser-container-drop.laml");
    let (mut editor, ctx) = ready(source);
    frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let package = preview.nodes.iter().find(|n| n.id == "Group1").unwrap();
    let (x, _, _, bottom) = package.container_body().unwrap();
    let target = connection_geometry::screen(preview, canvas, x + 12.0, bottom - 12.0);
    let from = point(&editor, "Browser", false);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(from), button(from, true)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    let key = editor.placement.key.as_ref().expect("package body target");
    assert_eq!(key.target, "Group1");
    assert_eq!(key.side, None);
    // Early release must wait for the shared compiler rather than cancel the move.
    frame(&mut editor, &ctx, vec![button(target, false)]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while editor.placement.released {
        assert!(Instant::now() < deadline, "drop validation timed out");
        frame(&mut editor, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(editor.document.revision(), 1, "{:?}", editor.command_error);
    assert_eq!(editor.rendered, Some(1));
    let preview = editor.preview.as_ref().unwrap();
    assert!(
        preview
            .diagram
            .nodes
            .iter()
            .any(|n| n.id == "Group1.Browser" && n.parent.as_deref() == Some("Group1"))
    );
    assert_eq!(preview.diagram.relations.len(), 6);
    assert!(
        editor
            .document
            .source()
            .contains("@Relation5 Group1.Browser -> Group1")
    );
    assert!(
        editor
            .document
            .source()
            .contains("Node2 ? Group1.Browser : align=left")
    );
    assert!(!editor.document.source().contains("Group1 above"));
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
