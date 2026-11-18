use super::*;

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.toast_overlay(ui.ctx()),
    );
    output.textures_delta.clear();
    output
}
fn ring(output: &egui::FullOutput) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Circle(c) if (c.radius - 11.5).abs() < 0.01 => Some(c.center),
            _ => None,
        })
        .expect("countdown ring")
}
#[test]
fn countdown_tracks_the_same_five_second_deadline() {
    assert_eq!(remaining(Duration::ZERO), 1.0);
    assert_eq!(remaining(Duration::from_millis(2500)), 0.5);
    assert_eq!(remaining(TIMEOUT), 0.0);
    assert_eq!(remaining(Duration::from_secs(20)), 0.0);
    let center = egui::pos2(30.0, 40.0);
    let full = countdown(center, 11.5, 1.0);
    let half = countdown(center, 11.5, 0.5);
    assert!(full.len() > half.len());
    assert!((full[0].x - center.x).abs() < 0.001);
    assert!((full[0].y - center.y + 11.5).abs() < 0.001);
    assert!((half[0].y - center.y - 11.5).abs() < 0.001);
    for point in full.iter().chain(&half) {
        assert!((point.distance(center) - 11.5).abs() < 0.001);
    }
}
#[test]
fn rendered_close_is_an_icon_with_a_clickable_countdown_not_a_text_cross() {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let mut editor = Editor::new("semantics {}".into(), None);
    editor.error_toast("Test error");
    for _ in 0..3 {
        frame(&mut editor, &ctx, vec![]);
    }
    let output = frame(&mut editor, &ctx, vec![]);
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "×" || t.galley.text() == "X")));
    assert!(
        output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(rect) if rect.brush.is_some())),
        "SVG close icon must render"
    );
    let pos = ring(&output);
    let since = editor.graph.toast.as_ref().unwrap().since;
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(pos)]);
    assert_eq!(editor.graph.toast.as_ref().unwrap().since, since);
    for pressed in [true, false] {
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    assert!(editor.graph.toast.is_none());
}
#[test]
fn timeout_removes_toast_and_new_error_gets_a_fresh_countdown() {
    let ctx = egui::Context::default();
    let mut editor = Editor::new("semantics {}".into(), None);
    editor.error_toast("Old error");
    editor.graph.toast.as_mut().unwrap().since = Instant::now() - Duration::from_secs(6);
    frame(&mut editor, &ctx, vec![]);
    assert!(editor.graph.toast.is_none());
    editor.error_toast("New error");
    let toast = editor.graph.toast.as_ref().unwrap();
    assert_eq!(toast.message, "New error");
    assert!(remaining(toast.since.elapsed()) > 0.9);
    assert_eq!(editor.document.revision(), 0);
}
