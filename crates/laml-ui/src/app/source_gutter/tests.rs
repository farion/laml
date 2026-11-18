use super::*;

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.source_editor(ui),
    );
    out.textures_delta.clear();
    out
}
fn text_shape<'a>(
    out: &'a egui::FullOutput,
    text: &str,
) -> (&'a egui::epaint::TextShape, egui::Rect) {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some((t, s.clip_rect)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing painted text {text}"))
}
fn scroll(editor: &mut Editor, ctx: &egui::Context, delta: egui::Vec2) -> egui::FullOutput {
    frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(egui::pos2(250.0, 250.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                delta,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut out = frame(editor, ctx, vec![]);
    for _ in 0..25 {
        out = frame(editor, ctx, vec![]);
    }
    out
}
#[test]
fn horizontal_scroll_moves_only_code_and_vertical_scroll_keeps_rows_aligned() {
    let source = (0..60)
        .map(|_| format!("// {}\n", "wide ".repeat(100)))
        .collect::<String>();
    let mut editor = Editor::new(source.clone(), None);
    let ctx = egui::Context::default();
    let before = frame(&mut editor, &ctx, vec![]);
    let number_x = text_shape(&before, "1").0.pos.x;
    let code_x = text_shape(&before, &source).0.pos.x;
    let code_clip = text_shape(&before, &source).1;
    let horizontal = scroll(&mut editor, &ctx, egui::vec2(-200.0, 0.0));
    assert!((text_shape(&horizontal, "1").0.pos.x - number_x).abs() < 0.01);
    assert!(text_shape(&horizontal, &source).0.pos.x < code_x - 50.0);
    assert!((text_shape(&horizontal, &source).1.left() - code_clip.left()).abs() < 0.01);
    assert!(text_shape(&horizontal, "1").1.right() <= code_clip.left());
    let row_y = text_shape(&horizontal, "10").0.pos.y;
    let code_y = text_shape(&horizontal, &source).0.pos.y;
    let vertical = scroll(&mut editor, &ctx, egui::vec2(0.0, -80.0));
    let row_delta = text_shape(&vertical, "10").0.pos.y - row_y;
    let code_delta = text_shape(&vertical, &source).0.pos.y - code_y;
    assert!(row_delta < -20.0);
    assert!((row_delta - code_delta).abs() < 0.01);
    assert_eq!(editor.document.source(), source);
    assert_eq!(editor.document.revision(), 0);
}
#[test]
fn hiding_numbers_reclaims_width_without_changing_source_cursor_or_undo() {
    let source = "// Grüße\n// second\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    editor.source_ui.jump = Some(3..9);
    let before = frame(&mut editor, &ctx, vec![]);
    let x = text_shape(&before, source).0.pos.x;
    let id = editor.source_ui.id.unwrap();
    let cursor = egui::TextEdit::load_state(&ctx, id)
        .unwrap()
        .cursor
        .char_range();
    editor.source_ui.line_numbers = false;
    let after = frame(&mut editor, &ctx, vec![]);
    assert!(text_shape(&after, source).0.pos.x < x - 20.0);
    assert!(!after.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "1" || t.galley.text() == "2")));
    assert_eq!(editor.source_ui.id, Some(id));
    assert_eq!(
        egui::TextEdit::load_state(&ctx, id)
            .unwrap()
            .cursor
            .char_range(),
        cursor
    );
    assert_eq!(editor.document.revision(), 0);
    assert_eq!(editor.document.history_usage().0, 0);
    editor.source_ui.line_numbers = true;
    let restored = frame(&mut editor, &ctx, vec![]);
    assert!((text_shape(&restored, source).0.pos.x - x).abs() < 0.01);
    assert_eq!(editor.document.source(), source);
}
#[test]
fn trailing_empty_row_is_included_when_reserving_digit_width() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(Default::default(), |ui| {
        let gutter = Gutter::reserve(ui, &"\n".repeat(9), true).unwrap();
        assert_eq!(gutter.rect.width(), 2.0 * 9.0 + 24.0);
    });
    out.textures_delta.clear();
}
