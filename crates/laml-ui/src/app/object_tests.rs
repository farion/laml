use super::tests::wait;
use super::*;
use laml_ui::delete_edit::ContainerMode;

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
#[test]
fn node_labels_and_ids_are_separate_atomic_transactions() {
    let source = "semantics {\n A\n B\n A -> B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("A"), false);
    editor.change_node_labels("Friendly label");
    wait(&mut editor, &ctx, |e| e.document.revision() == 1);
    assert_eq!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "A")
            .unwrap()
            .label,
        "Friendly label"
    );
    editor.rename_node("A", "Alpha");
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 2 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.selection.ids, ["Alpha"]);
    assert!(editor.document.source().contains("Alpha -> B"));
    assert_eq!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "Alpha")
            .unwrap()
            .label,
        "Friendly label"
    );
    editor.document.undo();
    assert!(
        editor
            .document
            .source()
            .contains("A(label=\"Friendly label\")")
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn deletion_requires_confirmation_and_keeps_one_undo_step() {
    let source = "semantics {\n A\n B\n A -> B\n}\nlayout {\n A < B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("A"), false);
    editor.propose_delete(ContainerMode::WithContents);
    assert!(editor.deletion.is_some());
    assert_eq!(editor.document.source(), source);
    editor.confirm_delete();
    wait(&mut editor, &ctx, |e| e.document.revision() == 1);
    assert!(editor.selection.ids.is_empty());
    assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), 1);
    assert!(editor.preview.as_ref().unwrap().routes.is_empty());
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn deletion_preview_cannot_outlive_text_or_selection_changes() {
    let source = "semantics {\n A\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("A"), false);
    editor.propose_delete(ContainerMode::WithContents);
    editor.selection.click(Some("B"), false);
    editor.confirm_delete();
    assert_eq!(editor.document.source(), source);
    assert!(editor.pending_command.is_none());
    editor.propose_delete(ContainerMode::WithContents);
    editor
        .document
        .replace_text(format!("{source}// newer\n"))
        .unwrap();
    editor.confirm_delete();
    assert!(editor.document.source().ends_with("// newer\n"));
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(1));
    assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), 2);
}
#[test]
fn container_modes_remove_or_retain_contents_and_preflight_geometry() {
    let source = "semantics {\n package P {\n A\n B\n A -> B\n }\n}\n";
    for (mode, remaining) in [
        (ContainerMode::WithContents, 0),
        (ContainerMode::KeepContents, 2),
    ] {
        let (mut editor, ctx) = ready(source);
        editor.selection.click(Some("P"), false);
        editor.propose_delete(mode);
        editor.confirm_delete();
        wait(&mut editor, &ctx, |e| {
            e.document.revision() == 1 || e.command_error.is_some()
        });
        assert_eq!(editor.command_error, None);
        assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), remaining);
        editor.document.undo();
        assert_eq!(editor.document.source(), source);
    }
}
#[test]
fn object_and_relation_controls_render_without_native_window() {
    let (mut editor, ctx) = ready("semantics {\n A\n B\n A -> B\n}\n");
    for relation in [false, true] {
        if relation {
            editor.selection.click_relation(4, false);
        } else {
            editor.selection.click(Some("A"), false);
        }
        editor.propose_delete(ContainerMode::WithContents);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 1000.0),
                )),
                ..Default::default()
            },
            |ui| editor.inspector(ui),
        );
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
