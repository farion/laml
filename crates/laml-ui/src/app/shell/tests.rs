use super::*;
use egui_phosphor::regular::{MINUS, PLUS};

#[test]
fn floating_toolbars_fit_without_covering_the_diagram_at_small_sizes() {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    let mut editor = Editor::new("semantics {\n A\n}\n".into(), None);
    for width in [324.0, 348.0, 550.0, 620.0, 706.0] {
        let viewport = egui::Rect::from_min_size(egui::pos2(20.0, 80.0), egui::vec2(width, 800.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1568.0, 975.0),
                )),
                ..Default::default()
            },
            |ui| {
                let tools = editor.canvas_tools(ui, viewport);
                assert!(
                    tools.right() <= viewport.right(),
                    "toolbar {tools:?} exceeds {viewport:?}"
                );
                let reserved = 80.0;
                assert!(
                    tools.bottom() <= viewport.top() + reserved,
                    "toolbar {tools:?} exceeds reserved height"
                );
            },
        );
        output.textures_delta.clear();
    }
}

#[test]
fn zoom_controls_stay_in_a_compact_footer_under_the_preview() {
    let source = "semantics {\n A\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    for (width, pixel_scale) in [(1000.0, 1.0), (1568.0, 1.25), (2190.0, 1.5), (1568.0, 2.0)] {
        for _ in 0..3 {
            let mut bounds = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 975.0),
                    )),
                    viewports: std::iter::once((
                        egui::ViewportId::ROOT,
                        egui::ViewportInfo {
                            native_pixels_per_point: Some(pixel_scale),
                            ..Default::default()
                        },
                    ))
                    .collect(),
                    ..Default::default()
                },
                |ui| {
                    editor.shell(ui);
                    let panel = |name| {
                        egui::containers::panel::PanelState::load(&ctx, ui.make_persistent_id(name))
                            .unwrap()
                            .outer_rect
                    };
                    bounds = Some((
                        panel("document-status"),
                        panel("source-pane"),
                        panel("properties-pane"),
                    ));
                },
            );
            output.textures_delta.clear();
            let (footer, source, properties) = bounds.unwrap();
            assert_eq!(footer.height(), 40.0);
            assert!(
                source.top() < 40.0,
                "no application title strip above panes"
            );
            assert_eq!(source.top(), properties.top());
            assert!(
                output.shapes.iter().any(|shape| {
                    if let egui::Shape::LineSegment { points, stroke } = &shape.shape {
                        (points[0].x - source.left()).abs() < 1.0
                            && (points[1].x - source.right()).abs() <= 1.0
                            && (points[0].y - source.top() - appearance::HEADER_HEIGHT).abs() < 1.0
                            && points[0].y == points[1].y
                            && *stroke == egui::Stroke::new(1.0, appearance::BORDER)
                    } else {
                        false
                    }
                }),
                "source header must have a full-width bottom border"
            );
            let mut controls = Vec::new();
            for shape in &output.shapes {
                if let egui::Shape::Rect(button) = &shape.shape
                    && button.rect.width() < 100.0
                    && footer.contains(button.rect.center())
                    && button.rect.center().x > source.right()
                    && button.rect.center().x < properties.left()
                {
                    let content = footer.shrink2(egui::vec2(0.0, 8.0));
                    assert!(
                        content.contains_rect(button.rect),
                        "full button {:?} must fit footer content {content:?}",
                        button.rect
                    );
                    assert!(
                        shape.clip_rect.contains_rect(button.rect),
                        "button {:?} clipped by {:?}",
                        button.rect,
                        shape.clip_rect
                    );
                }
                if let egui::Shape::Text(text) = &shape.shape {
                    let label = text.galley.job.text.as_str();
                    assert_ne!(label, "LAML Editor");
                    if (["Fit", "100%", MINUS, PLUS].contains(&label) || label.ends_with('%'))
                        && (label != PLUS || text.pos.y >= footer.top())
                    {
                        assert!(
                            footer.contains(text.pos),
                            "{label} outside footer: {:?}",
                            text.pos
                        );
                        assert!(text.pos.y >= footer.top() + 8.0);
                        assert!(text.pos.y + text.galley.size().y <= footer.bottom() - 8.0);
                        assert!(text.pos.x >= source.right() + 12.0);
                        assert!(text.pos.x + text.galley.size().x <= properties.left());
                        if [MINUS, PLUS].contains(&label) {
                            assert_eq!(
                                text.galley.job.sections[0].format.font_id.family,
                                egui::FontFamily::Name("phosphor".into())
                            );
                            let center = text.pos + text.galley.size() / 2.0;
                            let button = output
                                .shapes
                                .iter()
                                .find_map(|shape| match &shape.shape {
                                    egui::Shape::Rect(button)
                                        if button.rect.width() < 100.0
                                            && button.rect.contains(center) =>
                                    {
                                        Some(button.rect)
                                    }
                                    _ => None,
                                })
                                .expect("icon has a button background");
                            assert_eq!(button.size(), egui::Vec2::splat(24.0));
                            assert!(button.center().distance(center) < 0.01);
                        }
                        controls.push(label);
                    }
                }
            }
            assert!(
                controls.last().is_some_and(|label| label.ends_with('%')),
                "width={width} scale={pixel_scale} controls={controls:?}"
            );
            if controls.len() == 5 {
                assert_eq!(&controls[..4], &["Fit", "100%", MINUS, PLUS]);
            } else {
                // Narrow diagram panes reserve room for the relation picker;
                // trailing zoom actions belong in the shared overflow menu.
                assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::Shape::Text(t) if t.galley.job.text == egui_phosphor::regular::DOTS_THREE
                        && footer.contains(t.pos))));
            }
            output.textures_delta.clear();
        }
    }
    assert_eq!(editor.document.revision(), 0);
}

#[test]
fn mockup_pane_proportions_survive_native_resize_and_selection() {
    let source = "semantics {\n A\n B\n @r A -> B : request\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    for (width, selection) in [(1568.0, 0), (2190.0, 1), (1000.0, 2), (1568.0, 1)] {
        editor.selection.ids.clear();
        editor.selection.relations.clear();
        if selection == 1 {
            editor.selection.ids.push("A".into());
        }
        if selection == 2 {
            editor.selection.relations.push(4);
        }
        let mut panels = None;
        for _ in 0..3 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 975.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let left = ui.make_persistent_id("source-pane");
                    let right = ui.make_persistent_id("properties-pane");
                    editor.shell(ui);
                    panels = Some((
                        egui::containers::panel::PanelState::load(&ctx, left)
                            .unwrap()
                            .outer_rect,
                        egui::containers::panel::PanelState::load(&ctx, right)
                            .unwrap()
                            .outer_rect,
                    ));
                },
            );
            output.textures_delta.clear();
        }
        let (left, right) = panels.unwrap();
        assert!(
            (left.width() - width * 0.25).abs() <= 2.0,
            "left {left:?}, width={width}"
        );
        assert!(
            (right.width() - width * 0.20).abs() <= 2.0,
            "right {right:?}, width={width}"
        );
        assert!(right.left() - left.right() > 300.0);
        assert_eq!(editor.document.source(), source);
        assert_eq!(editor.document.revision(), 0);
    }
}
