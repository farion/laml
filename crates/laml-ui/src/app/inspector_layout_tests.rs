//! Mockup geometry: full-bleed categories, dark tabs and a single value column.
use super::inspector_tests::ready;
use super::*;

fn rects(shapes: &[egui::epaint::ClippedShape]) -> Vec<egui::epaint::RectShape> {
    fn collect(s: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
        match s {
            egui::Shape::Rect(r) => out.push(r.clone()),
            egui::Shape::Vec(v) => {
                for s in v {
                    collect(s, out)
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    for s in shapes {
        collect(&s.shape, &mut out);
    }
    out
}

#[test]
fn node_port_and_relation_keys_and_values_share_columns_and_categories_touch_both_edges() {
    for width in [270.0, 370.0, 600.0] {
        for kind in ["node", "relation", "port"] {
            for tab in [inspector_ui::Tab::General, inspector_ui::Tab::Style] {
                let (mut editor, ctx) = ready();
                if kind == "port" {
                    editor
                        .document
                        .replace_text("semantics {\n A {\n @P port Out\n }\n B\n}\n".into())
                        .unwrap();
                    editor.source_mutated();
                    editor.changed = Instant::now() - Duration::from_secs(1);
                    super::tests::wait(&mut editor, &ctx, |e| {
                        e.rendered == Some(e.document.revision())
                    });
                    editor.selection.click_port("P", false);
                }
                let source = editor.document.source().to_owned();
                let revision = editor.document.revision();
                if kind == "relation" {
                    editor.selection.click_relation(
                        editor.preview.as_ref().unwrap().routes[0].source_line,
                        false,
                    );
                }
                editor.inspector_tab = tab;
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 1600.0),
                        )),
                        ..Default::default()
                    },
                    |ui| editor.inspector(ui),
                );
                out.textures_delta.clear();
                assert!(!editor.source_ui.extra_drafts.is_empty());
                let left = f32::from(inspector_ui::INSET) + inspector_ui::KEY_WIDTH + 10.0;
                for id in &editor.source_ui.extra_drafts {
                    let rect = ctx.read_response(*id).unwrap().rect;
                    assert!(
                        (rect.left() - left).abs() < 1.0,
                        "{kind} {tab:?} width {width}: {rect:?}"
                    );
                    assert!(
                        rect.right() <= width - f32::from(inspector_ui::INSET) + 1.0,
                        "field overflows: {rect:?}"
                    );
                }
                let bars: Vec<_> = rects(&out.shapes)
                    .into_iter()
                    .filter(|r| {
                        r.fill == appearance::HEADER.gamma_multiply(0.45) && r.rect.height() == 30.0
                    })
                    .collect();
                assert!(!bars.is_empty());
                for bar in bars {
                    assert!(bar.rect.left().abs() < 1.0);
                    assert!(
                        (bar.rect.right() - width).abs() <= 1.0,
                        "{kind} {tab:?} width {width}: {:?}",
                        bar.rect
                    );
                }
                assert_eq!(editor.document.revision(), revision);
                assert_eq!(editor.document.source(), source);
            }
        }
    }
}

#[test]
fn empty_selection_hint_has_padding_below_tabs_in_both_views() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    editor.selection.click(None, false);
    for tab in [inspector_ui::Tab::General, inspector_ui::Tab::Style] {
        editor.inspector_tab = tab;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1568.0, 975.0),
                )),
                ..Default::default()
            },
            |ui| editor.shell(ui),
        );
        out.textures_delta.clear();
        let strip = rects(&out.shapes)
            .into_iter()
            .find(|r| {
                r.fill == appearance::HEADER && r.rect.height() == 40.0 && r.rect.left() > 1000.0
            })
            .unwrap()
            .rect;
        let hint = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.job.text == "Select an element in the diagram." =>
                {
                    Some(t)
                }
                _ => None,
            })
            .unwrap();
        assert!((hint.pos.y - strip.bottom() - f32::from(inspector_ui::INSET)).abs() < 1.0);
        assert!((hint.pos.x - strip.left() - f32::from(inspector_ui::INSET)).abs() < 1.0);
        assert_eq!(editor.document.revision(), 0);
    }
}

#[test]
fn tabs_have_a_dark_full_width_strip_and_a_distinct_active_panel() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    for tab in [inspector_ui::Tab::General, inspector_ui::Tab::Style] {
        editor.inspector_tab = tab;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1568.0, 975.0),
                )),
                ..Default::default()
            },
            |ui| editor.shell(ui),
        );
        out.textures_delta.clear();
        let shapes = rects(&out.shapes);
        let strip = shapes
            .iter()
            .find(|r| {
                r.fill == appearance::HEADER && r.rect.height() == 40.0 && r.rect.left() > 1000.0
            })
            .unwrap()
            .rect;
        let active = shapes
            .iter()
            .find(|r| {
                r.fill == appearance::PANEL
                    && r.rect.height() == 40.0
                    && r.rect.left() >= strip.left()
            })
            .unwrap()
            .rect;
        assert!((active.width() * 2.0 - strip.width()).abs() < 1.0);
        assert!(
            (active.left()
                - if tab == inspector_ui::Tab::General {
                    strip.left()
                } else {
                    strip.center().x
                })
            .abs()
                < 1.0
        );
        for label in ["Allgemein", "Style"] {
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => Some(t),
                    _ => None,
                })
                .unwrap();
            let selected = (label == "Allgemein") == (tab == inspector_ui::Tab::General);
            assert_eq!(
                text.galley.job.sections[0].format.color,
                if selected {
                    appearance::TEXT
                } else {
                    appearance::MUTED
                }
            );
        }
    }
}
