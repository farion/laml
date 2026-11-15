use super::tests::wait;
use super::*;
fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
#[test]
fn membership_proposal_commits_only_after_confirmation_and_undoes_once() {
    let source =
        "semantics {\n package P {\n @Svc Service\n }\n package Q {\n }\n X\n Svc -> X\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("Svc"), false);
    editor.propose_membership(Some("Q"));
    assert!(editor.membership.is_some(), "{:?}", editor.command_error);
    assert_eq!(editor.document.source(), source);
    editor.confirm_membership();
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.selection.ids, ["Svc"]);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(
        diagram
            .nodes
            .iter()
            .find(|n| n.id == "Svc")
            .unwrap()
            .parent
            .as_deref(),
        Some("Q")
    );
    assert_eq!(diagram.relations.len(), 1);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn stale_membership_proposal_cannot_overwrite_new_text() {
    let source = "semantics {\n A\n package P {\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("A"), false);
    editor.propose_membership(Some("P"));
    editor
        .document
        .replace_text(format!("{source}// newer\n"))
        .unwrap();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(1));
    editor.confirm_membership();
    assert_eq!(editor.document.source(), format!("{source}// newer\n"));
    assert!(editor.pending_command.is_none());
}
#[test]
fn invalid_membership_never_deletes_the_conflicting_constraints() {
    let source = "semantics {\n A\n B\n package P {\n }\n}\nlayout {\n A ? B : align=center\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("A"), false);
    editor.propose_membership(Some("P"));
    if editor.membership.is_some() {
        editor.confirm_membership();
        wait(&mut editor, &ctx, |e| {
            e.command_error.is_some() && e.pending_command.is_none()
        });
    }
    assert!(editor.command_error.is_some());
    assert_eq!(editor.document.source(), source);
}
#[test]
fn expanded_palette_port_and_membership_ui_render_headlessly() {
    let source = "semantics {\n A {\n @P port Out\n }\n package Group {\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    for port in [false, true] {
        if port {
            editor.selection.click_port("P", false);
        } else {
            editor.selection.click(Some("A"), false);
            editor.propose_membership(Some("Group"));
        }
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1100.0),
                )),
                ..Default::default()
            },
            |ui| {
                editor.creation_toolbar(ui);
                ui.columns(3, |cols| {
                    editor.source_editor(&mut cols[0]);
                    editor.canvas(&mut cols[1]);
                    editor.inspector(&mut cols[2]);
                });
            },
        );
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
