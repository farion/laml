use super::*;

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.source_editor(ui),
    );
    output.textures_delta.clear();
    output
}
fn ready() -> (Editor, egui::Context, Snapshot) {
    let source = "// Grüße 日本語\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    let selection = Snapshot {
        revision: 0,
        range: 3..source.len() - 1,
    };
    editor.source_ui.jump = Some(selection.range.clone());
    frame(&mut editor, &ctx, vec![]);
    (editor, ctx, selection)
}
fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
fn label(shapes: &[egui::epaint::ClippedShape], title: &str) -> egui::Pos2 {
    shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text().ends_with(title) => {
                Some(t.pos + egui::vec2(30.0, 8.0))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing menu item {title}"))
}
#[test]
fn actual_right_click_menu_keeps_selection_and_deletes_in_one_undo() {
    let (mut editor, ctx, selection) = ready();
    let source = editor.document.source().to_owned();
    let pos = editor.source_ui.rect.unwrap().center();
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            pointer(pos, egui::PointerButton::Secondary, true),
        ],
    );
    frame(
        &mut editor,
        &ctx,
        vec![pointer(pos, egui::PointerButton::Secondary, false)],
    );
    let output = frame(&mut editor, &ctx, vec![]);
    assert_eq!(
        editor.source_ui.context.selection.as_ref().unwrap().range,
        selection.range
    );
    for title in ["Copy", "Paste", "Cut", "Delete"] {
        label(&output.shapes, title);
    }
    let at = label(&output.shapes, "Delete");
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(at),
            pointer(at, egui::PointerButton::Primary, true),
        ],
    );
    frame(
        &mut editor,
        &ctx,
        vec![pointer(at, egui::PointerButton::Primary, false)],
    );
    assert_eq!(editor.document.source(), "// \n");
    assert_eq!(editor.document.history_usage().0, 1);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn native_copy_is_read_only_and_cut_uses_the_original_unicode_selection() {
    let (mut editor, ctx, selection) = ready();
    let source = editor.document.source().to_owned();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        editor.source_context_action(ui.ctx(), selection.clone(), Action::Copy);
    });
    assert!(
        output
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == "Grüße 日本語"))
    );
    output.textures_delta.clear();
    assert_eq!(editor.document.source(), source);
    assert_eq!(editor.document.revision(), 0);
    editor.source_context_action(&ctx, selection, Action::Cut);
    assert_eq!(editor.document.source(), "// \n");
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn native_menu_paste_requests_the_clipboard_and_replaces_the_saved_selection() {
    let (mut editor, ctx, selection) = ready();
    let source = editor.document.source().to_owned();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        editor.source_context_action(ui.ctx(), selection.clone(), Action::Paste);
    });
    assert!(output.viewport_output.values().any(|v| {
        v.commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::RequestPaste))
    }));
    output.textures_delta.clear();
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Paste("replacement".into())],
    );
    assert_eq!(editor.document.source(), "// replacement\n");
    assert_eq!(editor.document.history_usage().0, 1);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}

#[test]
fn denied_cut_never_removes_the_original_text() {
    let (mut editor, ctx, selection) = ready();
    let source = editor.document.source().to_owned();
    let (sender, result) = mpsc::channel();
    editor.source_ui.context.pending = Some(Pending {
        selection,
        action: Action::Cut,
        result,
    });
    sender
        .send(Err(
            "Clipboard operation failed. Use the keyboard shortcut instead.".into(),
        ))
        .unwrap();
    editor.source_context_tick(&ctx);
    assert_eq!(editor.document.source(), source);
    assert_eq!(editor.document.revision(), 0);
    assert!(editor.graph.toast.is_some());
}
#[test]
fn delayed_paste_rejects_a_changed_document() {
    let (mut editor, ctx, selection) = ready();
    let (sender, result) = mpsc::channel();
    editor.source_ui.context.pending = Some(Pending {
        selection,
        action: Action::Paste,
        result,
    });
    editor.document.replace_text("// newer".into()).unwrap();
    sender.send(Ok(Some("obsolete paste".into()))).unwrap();
    editor.source_context_tick(&ctx);
    assert_eq!(editor.document.source(), "// newer");
}
#[test]
fn asynchronous_paste_is_one_undo_and_normalizes_browser_newlines() {
    let (mut editor, ctx, selection) = ready();
    let source = editor.document.source().to_owned();
    let (sender, result) = mpsc::channel();
    editor.source_ui.context.pending = Some(Pending {
        selection,
        action: Action::Paste,
        result,
    });
    sender.send(Ok(Some("a\r\nb".into()))).unwrap();
    editor.source_context_tick(&ctx);
    assert_eq!(editor.document.source(), "// a\nb\n");
    assert_eq!(editor.document.history_usage().0, 1);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
