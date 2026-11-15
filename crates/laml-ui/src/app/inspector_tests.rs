use super::tests::wait;
use super::*;

pub(super) fn ready() -> (Editor, egui::Context) {
    let mut editor = Editor::new("semantics {\n A\n B\n A -> B\n}\n".into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    editor.selection.click(Some("A"), false);
    (editor, ctx)
}
fn render(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    editor.source_ui.extra_drafts.clear();
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(370.0, 1600.0),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.inspector(ui),
    );
    out.textures_delta.clear();
    out
}
fn enter(shift: bool) -> egui::Event {
    egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        },
    }
}
fn text(out: &egui::FullOutput) -> Vec<&str> {
    out.shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn tabs_have_flat_categories_no_elements_or_apply_and_left_aligned_attribute_captions() {
    let (mut editor, ctx) = ready();
    let general = render(&mut editor, &ctx, vec![]);
    let labels = text(&general);
    assert!(labels.contains(&"Identity") && labels.contains(&"Label") && labels.contains(&"Type"));
    assert!(!labels.iter().any(|s| s.starts_with("Apply")
        || *s == "Element"
        || *s == "Elements"
        || *s == "More actions"));
    editor.inspector_tab = inspector_ui::Tab::Style;
    let style = render(&mut editor, &ctx, vec![]);
    let labels = text(&style);
    assert!(labels.contains(&"Appearance") && labels.contains(&"Size & shape"));
    assert!(!labels.contains(&"Identity"));
    assert!(!labels.iter().any(|s| s.starts_with("Apply")));
    let x: Vec<_> = style
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if [
                    "Fill",
                    "Border",
                    "Stroke",
                    "Text",
                    "Base color",
                    "Corners",
                    "Width",
                    "Min. height",
                ]
                .contains(&t.galley.job.text.as_str()) =>
            {
                Some(t.pos.x)
            }
            _ => None,
        })
        .collect();
    assert_eq!(x.len(), 8);
    assert!(x.iter().all(|v| (*v - x[0]).abs() < 0.1));
}

#[test]
fn label_enter_commits_directly_once_and_invalid_style_keeps_original_source() {
    let (mut editor, ctx) = ready();
    let source = editor.document.source().to_owned();
    render(&mut editor, &ctx, vec![]);
    editor
        .property_drafts
        .insert("node-label".into(), "Checkout".into());
    let id = editor.source_ui.extra_drafts[0];
    ctx.memory_mut(|m| m.request_focus(id));
    render(&mut editor, &ctx, vec![enter(false)]);
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.preview.as_ref().unwrap().nodes[0].label, "Checkout");
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
    // Rebuild after undo before testing a different edit.
    editor.source_mutated();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| {
        e.rendered == Some(e.document.revision())
    });
    editor.inspector_tab = inspector_ui::Tab::Style;
    render(&mut editor, &ctx, vec![]);
    editor
        .property_drafts
        .insert("width".into(), "not-a-number".into());
    let id = editor.source_ui.extra_drafts[6];
    ctx.memory_mut(|m| m.request_focus(id));
    render(&mut editor, &ctx, vec![enter(false)]);
    assert!(editor.command_error.is_some());
    assert_eq!(editor.document.source(), source);
    assert!(editor.pending_command.is_none());
}

#[test]
fn label_grows_only_after_shift_enter_and_preserves_unicode_caret_and_padding() {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    let mut draft = "Grüße".to_string();
    let mut id = None;
    let mut height = 0.0;
    let mut commit = false;
    let mut field_rect = egui::Rect::NOTHING;
    let mut draw = |draft: &mut String, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let field = inspector_ui::field(ui, "label", draft, true);
                id = Some(field.response.id);
                height = field.response.rect.height();
                field_rect = field.response.rect;
                commit = field.commit;
            },
        );
        out.textures_delta.clear();
        (out, id.unwrap(), height, commit, field_rect)
    };
    let (out, id, one, _, rect) = draw(&mut draft, vec![]);
    assert!(one <= appearance::FIELD_HEIGHT + 3.0);
    let glyph = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Grüße" => Some(t.pos),
            _ => None,
        })
        .unwrap();
    assert!(glyph.x >= rect.left() + 9.0);
    ctx.memory_mut(|m| m.request_focus(id));
    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(2),
        )));
    state.store(&ctx, id);
    let (_, _, two, commit, _) = draw(&mut draft, vec![enter(true)]);
    assert_eq!(draft, "Gr\nüße");
    assert!(two > one);
    assert!(!commit);
    let (_, _, _, commit, _) = draw(&mut draft, vec![enter(false)]);
    assert!(commit);
    assert_eq!(draft, "Gr\nüße");
}

#[test]
fn both_header_action_groups_end_at_the_right_margin_at_multiple_widths() {
    for width in [300.0, 520.0, 1000.0] {
        for diagram in [false, true] {
            let (mut editor, ctx) = ready();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 160.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    if diagram {
                        editor.graph_heading(ui)
                    } else {
                        editor.source_heading(ui)
                    }
                },
            );
            out.textures_delta.clear();
            let rects: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.rect.height() > 25.0
                            && r.rect.height() < 40.0
                            && r.rect.width() > 20.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .collect();
            let last = rects.iter().map(|r| r.right()).reduce(f32::max).unwrap();
            assert!(
                (last - (width - 14.0)).abs() < 6.0,
                "width {width}, diagram {diagram}, right {last}"
            );
        }
    }
}

#[test]
fn connection_grip_geometry_and_hit_areas_scale_with_the_canvas_not_the_ui() {
    let (editor, _) = ready();
    let preview = editor.preview.as_ref().unwrap();
    let node = &preview.nodes[0];
    let size = egui::vec2(
        (preview.bounds.2 - preview.bounds.0) as f32,
        (preview.bounds.3 - preview.bounds.1) as f32,
    );
    let small = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let large = egui::Rect::from_min_size(egui::Pos2::ZERO, size * 3.0);
    let a = connection_geometry::handles(preview, small, node, true);
    let b = connection_geometry::handles(preview, large, node, true);
    for (a, b) in a.iter().zip(&b) {
        assert!((b.radius - a.radius * 3.0).abs() < 0.01);
        assert!((b.point.to_vec2() - a.point.to_vec2() * 3.0).length() < 0.01);
    }
    let right = &b[1];
    let pointer = right.point + egui::vec2(0.0, right.radius * 0.8);
    assert_eq!(
        connection_geometry::source_at(preview, large, pointer)
            .unwrap()
            .side,
        Some("right")
    );
}

#[test]
fn a_node_modal_blocks_new_document_and_document_undo_shortcuts() {
    let (mut editor, ctx) = ready();
    editor.managed = true;
    editor.open_node_dialog();
    for events in [
        vec![],
        vec![egui::Event::Key {
            key: egui::Key::N,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    ] {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                editor.file_events(ui.ctx());
                editor.history_shortcuts(ui.ctx());
                editor.graph_windows(ui.ctx());
            },
        );
        out.textures_delta.clear();
    }
    assert!(editor.workspace_action.is_none());
    assert_eq!(editor.document.revision(), 0);
    assert!(editor.graph.node_dialog.is_some());
}

#[test]
fn choosing_a_shape_commits_without_any_apply_button() {
    let (mut editor, ctx) = ready();
    let source = editor.document.source().to_owned();
    editor.inspector_tab = inspector_ui::Tab::Style;
    let center = |out: &egui::FullOutput, label: &str| {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap()
    };
    let out = render(&mut editor, &ctx, vec![]);
    let button = center(&out, "rectangle");
    for pressed in [true, false] {
        render(
            &mut editor,
            &ctx,
            vec![
                egui::Event::PointerMoved(button),
                egui::Event::PointerButton {
                    pos: button,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let out = render(&mut editor, &ctx, vec![]);
    let choice = center(&out, "ellipse");
    for pressed in [true, false] {
        render(
            &mut editor,
            &ctx,
            vec![
                egui::Event::PointerMoved(choice),
                egui::Event::PointerButton {
                    pos: choice,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "A")
            .unwrap()
            .shape,
        laml::geometry::NodeShape::Ellipse
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
