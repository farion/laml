use super::*;
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
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn click(editor: &mut Editor, ctx: &egui::Context, point: egui::Pos2) {
    frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    frame(
        editor,
        ctx,
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}
fn setup() -> (Editor, egui::Context, egui::Pos2) {
    let source = "semantics {\n A(label=\"Alpha\")\n B\n A -> B: Flow\n}\n";
    setup_text(source, "Alpha")
}
fn setup_text(source: &str, label: &str) -> (Editor, egui::Context, egui::Pos2) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    for _ in 0..3 {
        frame(&mut editor, &ctx, vec![]);
    }
    let p = editor.preview.as_ref().unwrap();
    let t = p.texts.iter().find(|t| t.text == label).unwrap();
    let image = editor.creation.canvas_rect.unwrap();
    let scale = image.width() / (p.bounds.2 - p.bounds.0) as f32;
    let point = image.min
        + egui::vec2(
            (t.x + t.width / 2.0 - p.bounds.0) as f32,
            (t.y + t.height / 2.0 - p.bounds.1) as f32,
        ) * scale;
    (editor, ctx, point)
}
fn open(editor: &mut Editor, ctx: &egui::Context, point: egui::Pos2) {
    click(editor, ctx, point);
    assert!(
        editor.inline_text.is_none(),
        "Single click must not edit text"
    );
    click(editor, ctx, point);
    assert!(
        editor.inline_text.is_some(),
        "Double click opens inline text"
    );
    frame(editor, ctx, vec![]);
}
fn finish(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..200 {
        editor.compile(ctx);
        if editor.pending_command.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(editor.command_error.is_none(), "{:?}", editor.command_error);
    assert!(editor.pending_command.is_none());
}
#[test]
fn inline_text_escape_discards_without_changing_source() {
    let (mut editor, ctx, point) = setup();
    let original = editor.document.source().to_string();
    open(&mut editor, &ctx, point);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Changed".into())]);
    assert_eq!(editor.inline_text.as_ref().unwrap().value, "Changed");
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(editor.inline_text.is_none());
    assert_eq!(editor.document.source(), original);
    assert_eq!(editor.document.revision(), 0);
}
#[test]
fn inline_text_enter_commits_and_shift_enter_inserts_newline() {
    let (mut editor, ctx, point) = setup();
    let original = editor.document.source().to_string();
    open(&mut editor, &ctx, point);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Changed".into())]);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::SHIFT)],
    );
    assert_eq!(editor.inline_text.as_ref().unwrap().value, "Changed\n");
    assert_eq!(editor.document.source(), original);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Second".into())]);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    finish(&mut editor, &ctx);
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.nodes[0]
            .label
            .as_deref(),
        Some("Changed\nSecond")
    );
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), original);
}
#[test]
fn inline_text_click_outside_commits() {
    let (mut editor, ctx, point) = setup();
    open(&mut editor, &ctx, point);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Outside".into())]);
    click(&mut editor, &ctx, egui::pos2(15.0, 15.0));
    finish(&mut editor, &ctx);
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.nodes[0]
            .label
            .as_deref(),
        Some("Outside")
    );
}
#[test]
fn inline_text_arrow_shift_enter_is_ignored_and_enter_preserves_endpoints() {
    let (mut editor, ctx, _) = setup();
    let preview = editor.preview.as_ref().unwrap();
    let text = preview.texts.iter().find(|t| t.text == "Flow").unwrap();
    let image = editor.creation.canvas_rect.unwrap();
    let scale = image.width() / (preview.bounds.2 - preview.bounds.0) as f32;
    let point = image.min
        + egui::vec2(
            (text.x + text.width / 2.0 - preview.bounds.0) as f32,
            (text.y + text.height / 2.0 - preview.bounds.1) as f32,
        ) * scale;
    open(&mut editor, &ctx, point);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Caption".into())]);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::SHIFT)],
    );
    assert_eq!(editor.inline_text.as_ref().unwrap().value, "Caption");
    assert!(editor.pending_command.is_none());
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    finish(&mut editor, &ctx);
    let relation = &editor.preview.as_ref().unwrap().diagram.relations[0];
    assert_eq!(relation.label.as_deref(), Some("Caption"));
    assert_eq!((&*relation.from, &*relation.to), ("A", "B"));
}

#[test]
fn inline_text_stale_draft_cannot_overwrite_newer_source() {
    let (mut editor, ctx, point) = setup();
    open(&mut editor, &ctx, point);
    let draft = editor.inline_text.take().unwrap();
    editor.apply_source_edits(
        "Newer",
        vec![laml_ui::document::Edit {
            range: 0..0,
            replacement: "// newer\n".into(),
        }],
        None,
    );
    editor.confirm_inline_text(Draft {
        value: "Old".into(),
        ..draft
    });
    assert!(editor.pending_command.is_none());
    assert!(editor.document.source().starts_with("// newer\n"));
    assert!(editor.document.source().contains("Alpha"));
}
#[test]
fn sequence_fragment_header_and_condition_double_click_edit_only_the_condition() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n Block: opt \"ready\" { // keep\n A ->> B: Work\n }\n}\n";
    for label in ["opt", "[ready]"] {
        let (mut editor, ctx, point) = setup_text(source, label);
        open(&mut editor, &ctx, point);
        assert_eq!(editor.inline_text.as_ref().unwrap().value, "ready");
        assert!(!editor.inline_text.as_ref().unwrap().text.multiline);
        frame(&mut editor, &ctx, vec![egui::Event::Text("changed".into())]);
        frame(
            &mut editor,
            &ctx,
            vec![key(egui::Key::Enter, egui::Modifiers::SHIFT)],
        );
        assert_eq!(editor.inline_text.as_ref().unwrap().value, "changed");
        frame(
            &mut editor,
            &ctx,
            vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
        );
        finish(&mut editor, &ctx);
        assert!(
            editor
                .document
                .source()
                .contains("Block: opt \"changed\" { // keep")
        );
        assert!(editor.document.source().contains(" A ->> B: Work"));
        assert!(editor.document.undo());
        assert_eq!(editor.document.source(), source);
        assert!(!editor.document.undo());
    }
}
