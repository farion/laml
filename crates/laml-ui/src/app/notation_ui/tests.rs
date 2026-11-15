use super::*;
#[test]
fn specialized_inspectors_and_creation_dialogs_draw_without_context_reentry() {
    for source in [include_str!("../../../../../examples/sequence-notes.laml"), include_str!("../../../../../examples/bpmn-basic.laml"), include_str!("../../../../../examples/timing-basic.laml"), include_str!("../../../../../examples/c4-context.laml")] {
        let ctx = egui::Context::default();
        appearance::install(&ctx); egui_extras::install_image_loaders(&ctx);
        let mut editor = Editor::new(source.into(), None);
        let preview = laml_ui::preview::build(source, std::path::Path::new("ui.laml")).unwrap();
        editor.publish_preview(&ctx, preview);
        let regions = editor.preview.as_ref().unwrap().regions.clone();
        let mut targets: Vec<_> = regions.into_iter().map(|r| (r.line, Some(r.id))).collect();
        if targets.is_empty() { targets.extend(editor.preview.as_ref().unwrap().diagram.nodes.iter().map(|n| (n.line, Some(n.id.clone())))); }
        for (line, id) in targets {
            editor.notation_select(line, id);
            for tab in [inspector_ui::Tab::General, inspector_ui::Tab::Style] {
                editor.inspector_tab = tab;
                let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(440.0, 900.0))), ..Default::default() }, |ui| editor.inspector(ui));
                out.textures_delta.clear();
            }
        }
        editor.open_construct_dialog();
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), ..Default::default() }, |ui| editor.notation_windows(ui.ctx()));
        out.textures_delta.clear();
    }
}
#[test]
fn invalid_text_keeps_the_last_valid_notation_until_compilation_succeeds() {
    let ctx = egui::Context::default();
    let source = laml_ui::notation::Notation::Sequence.starter(false);
    let mut editor = Editor::new(source.clone(), None);
    let preview = laml_ui::preview::build(&source, std::path::Path::new("ui.laml")).unwrap();
    editor.publish_preview(&ctx, preview);
    editor.document.replace_text("settings {\n type: unfinished\n}\n".into()).unwrap();
    assert_eq!(editor.notation(), laml_ui::notation::Notation::Sequence);
    assert!(!editor.can_edit_visually());
}
