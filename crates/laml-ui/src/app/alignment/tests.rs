use super::*;
const SOURCE: &str = "semantics {\n A\n B\n C\n}\n";
fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |editor| editor.rendered == Some(0));
    editor.selection.ids = vec!["A".into(), "B".into()];
    (editor, ctx)
}
fn wait(editor: &mut Editor, ctx: &egui::Context, done: impl Fn(&Editor) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(editor) {
        assert!(
            Instant::now() < deadline,
            "timed out: {:?} {:?}",
            editor.error,
            editor.command_error
        );
        editor.compile(ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn seven_alignment_actions_preflight_commit_and_share_history() {
    for align in [
        FreeAlignment::Left,
        FreeAlignment::Right,
        FreeAlignment::Top,
        FreeAlignment::Bottom,
        FreeAlignment::CenterX,
        FreeAlignment::CenterY,
        FreeAlignment::Center,
    ] {
        let (mut editor, ctx) = ready(SOURCE);
        editor.propose_alignment(align);
        assert!(editor.alignment.is_some());
        assert_eq!(editor.document.source(), SOURCE);
        editor.apply_alignment();
        wait(&mut editor, &ctx, |editor| editor.document.revision() == 1);
        let graph = &editor.preview.as_ref().unwrap().diagram;
        assert_eq!(graph.alignments.len(), 1);
        assert_eq!(graph.alignments[0].align, align);
        assert_eq!(graph.alignments[0].second, "B");
        assert_eq!(editor.rendered, Some(1));
        editor.document.undo();
        assert_eq!(editor.document.source(), SOURCE);
    }
}
#[test]
fn untouched_perpendicular_constraint_is_not_removed_to_make_alignment_fit() {
    let source = "semantics {\n A\n B\n}\nlayout {\n A above B : align=left\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.propose_alignment(FreeAlignment::Top);
    editor.apply_alignment();
    wait(&mut editor, &ctx, |editor| editor.command_error.is_some());
    assert_eq!(editor.document.source(), source);
    assert_eq!(editor.document.revision(), 0);
}
#[test]
fn edits_or_selection_changes_invalidate_unconfirmed_proposals() {
    let (mut editor, _) = ready(SOURCE);
    editor.propose_alignment(FreeAlignment::Left);
    editor.selection.ids = vec!["A".into(), "C".into()];
    editor.apply_alignment();
    assert_eq!(editor.document.source(), SOURCE);
    editor.propose_alignment(FreeAlignment::Center);
    let newer = format!("{SOURCE}// typed\n");
    editor.document.replace_text(newer.clone()).unwrap();
    editor.apply_alignment();
    assert_eq!(editor.document.source(), newer);
}
#[test]
fn alignment_controls_and_confirmation_render_headlessly() {
    let (mut editor, ctx) = ready(SOURCE);
    editor.propose_alignment(FreeAlignment::Center);
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        editor.alignment_toolbar(ui);
        editor.alignment_confirmation(ui);
    });
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    assert_eq!(editor.document.source(), SOURCE);
}
