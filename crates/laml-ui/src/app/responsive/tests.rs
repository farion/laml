use super::*;

fn ready() -> (Editor, egui::Context) {
    let source = "semantics {\n A\n B\n A -> B : request\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    editor.publish_preview(&ctx, laml_ui::preview::build(source, &editor.path).unwrap());
    (editor, ctx)
}
fn frame(
    e: &mut Editor,
    ctx: &egui::Context,
    width: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ui| e.shell(ui),
    );
    output.textures_delta.clear();
    output
}
fn text(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.job.text == label => {
            Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
        }
        _ => None,
    })
}
fn click(e: &mut Editor, ctx: &egui::Context, width: f32, at: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            e,
            ctx,
            width,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn narrow_header_switches_views_without_mutating_the_document() {
    let (mut e, ctx) = ready();
    let original = e.document.source().to_string();
    for width in [320.0, 390.0, 600.0] {
        e.responsive.pane = Pane::Diagram;
        for _ in 0..3 {
            frame(&mut e, &ctx, width, vec![]);
        }
        let output = frame(&mut e, &ctx, width, vec![]);
        let heading = text(&output, "Diagram").expect("pane selector");
        click(&mut e, &ctx, width, heading.center());
        let output = frame(&mut e, &ctx, width, vec![]);
        let source = text(&output, "Source").expect("source menu entry");
        click(&mut e, &ctx, width, source.center());
        let output = frame(&mut e, &ctx, width, vec![]);
        assert_eq!(e.responsive.pane, Pane::Source);
        assert!(text(&output, "Source").is_some());
        assert!(text(&output, "Diagram").is_none());
        assert_eq!(e.document.source(), original);
        assert_eq!(e.document.revision(), 0);
    }
}
#[test]
fn flyout_fits_small_screens_and_closes_without_touching_source() {
    let (mut e, ctx) = ready();
    for width in [320.0, 390.0, 800.0] {
        e.responsive.flyout = true;
        for _ in 0..3 {
            frame(&mut e, &ctx, width, vec![]);
        }
        let output = frame(&mut e, &ctx, width, vec![]);
        let close = text(&output, egui_phosphor::regular::X).expect("flyout close");
        assert!(
            close.right() <= width && close.left() >= 0.0,
            "{width}: {close:?}"
        );
        assert!(text(&output, "Properties").is_some());
        click(&mut e, &ctx, width, close.center());
        assert!(!e.responsive.flyout);
        assert_eq!(e.document.revision(), 0);
    }
}
#[test]
fn touch_controls_follow_selection_not_a_stale_hover() {
    let (mut e, _) = ready();
    let preview = e.preview.as_ref().unwrap();
    let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
    let b = preview.nodes.iter().find(|n| n.id == "B").unwrap();
    let pointer = connection_geometry::node_rect(preview, canvas, b).center();
    e.selection.ids = vec!["A".into()];
    e.responsive.touch = true;
    assert_eq!(
        e.graph_control_node(preview, canvas, Some(pointer))
            .unwrap()
            .id,
        "A"
    );
    assert_eq!(e.graph_control_node(preview, canvas, None).unwrap().id, "A");
    e.selection.ids.clear();
    assert!(
        e.graph_control_node(preview, canvas, Some(pointer))
            .is_none()
    );
    e.responsive.touch = false;
    assert_eq!(
        e.graph_control_node(preview, canvas, Some(pointer))
            .unwrap()
            .id,
        "B"
    );
}
