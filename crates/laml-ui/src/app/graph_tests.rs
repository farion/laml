use super::tests::{canvas_frame, wait};
use super::*;
use laml::syntax::{FreeAlignment, NodeKind};

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
fn changed(editor: &mut Editor, ctx: &egui::Context, revision: u64) {
    wait(editor, ctx, |e| {
        e.document.revision() == revision || e.pending_command.is_none()
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.document.revision(), revision);
}
fn header_frame(
    editor: &mut Editor,
    ctx: &egui::Context,
    width: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            editor.graph_heading(ui);
            editor.canvas(ui);
            editor.graph_windows(ui.ctx());
        },
    );
    out.textures_delta.clear();
    out
}
fn click(editor: &mut Editor, ctx: &egui::Context, width: f32, label: &str) {
    let mut out = header_frame(editor, ctx, width, vec![]);
    for _ in 0..2 {
        out = header_frame(editor, ctx, width, vec![]);
    }
    let point = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.job.text == label
                    || t.galley.job.text.ends_with(&format!("  {label}")) =>
            {
                Some(t.pos + t.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing button {label}"));
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    header_frame(
        editor,
        ctx,
        width,
        vec![egui::Event::PointerMoved(point), button(true)],
    );
    header_frame(editor, ctx, width, vec![button(false)]);
}

#[test]
fn add_dialog_prefills_type_id_label_and_validates_before_one_atomic_creation() {
    let source = "semantics {\n A\n}\n";
    let (mut editor, ctx) = ready(source);
    click(&mut editor, &ctx, 600.0, egui_phosphor::regular::PLUS);
    let mut dialog = editor.graph.node_dialog.take().unwrap();
    assert_eq!(dialog.kind, laml_ui::element_edit::ElementKind::Rectangle);
    assert!(!dialog.id.is_empty());
    assert_eq!(dialog.id, dialog.label);
    dialog.id = "A".into();
    assert!(editor.submit_node_dialog(&dialog).is_err());
    assert_eq!(editor.document.source(), source);
    dialog.id = "CustomClass".into();
    dialog.label = "Public API".into();
    dialog.kind = laml_ui::element_edit::ElementKind::Class;
    editor.submit_node_dialog(&dialog).unwrap();
    changed(&mut editor, &ctx, 1);
    let node = editor
        .preview
        .as_ref()
        .unwrap()
        .diagram
        .nodes
        .iter()
        .find(|n| n.id == "CustomClass")
        .unwrap();
    assert_eq!(node.kind, NodeKind::Class);
    assert_eq!(node.label.as_deref(), Some("Public API"));
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(!editor.document.undo());
}

#[test]
fn stale_dialog_and_cancel_never_modify_the_source() {
    let (mut editor, ctx) = ready("semantics {\n A\n}\n");
    editor.open_node_dialog();
    let dialog = editor.graph.node_dialog.take().unwrap();
    editor
        .document
        .replace_text("semantics {\n A\n B\n}\n".into())
        .unwrap();
    assert!(editor.submit_node_dialog(&dialog).is_err());
    assert_eq!(editor.document.revision(), 1);
    let (mut editor, ctx2) = ready("semantics {\n A\n}\n");
    editor.open_node_dialog();
    click(&mut editor, &ctx2, 600.0, "Cancel");
    assert!(editor.graph.node_dialog.is_none());
    assert_eq!(editor.document.revision(), 0);
    drop(ctx);
}

#[test]
fn port_dialog_preserves_selection_and_commits_id_name_and_side_together() {
    let source = "semantics {\n A\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.ids = vec!["A".into(), "B".into()];
    editor.open_port_dialog("A", "top");
    assert_eq!(editor.selection.ids, ["A", "B"]);
    let mut dialog = editor.graph.port_dialog.take().unwrap();
    dialog.id = "Boundary".into();
    dialog.name = "Requests".into();
    editor.submit_port_dialog(&dialog).unwrap();
    changed(&mut editor, &ctx, 1);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(
        laml_ui::port_edit::find(diagram, "Boundary")
            .unwrap()
            .1
            .label,
        "Requests"
    );
    assert_eq!(diagram.port_layout[0].side, laml::syntax::PortSide::Top);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}

#[test]
fn all_six_header_alignments_apply_directly_without_inspector_proposals() {
    for (_, _, align) in graph_header::ALIGNMENTS {
        let source = "semantics {\n A\n B\n C\n}\nlayout {\n A < B\n B above C\n}\n";
        let (mut editor, ctx) = ready(source);
        editor.selection.ids = vec!["A".into(), "B".into(), "C".into()];
        editor.align_selected(align);
        assert!(editor.alignment.is_none());
        assert!(editor.pending_command.is_some());
        changed(&mut editor, &ctx, 1);
        assert_eq!(editor.selection.ids, ["A", "B", "C"]);
        let values: Vec<_> = editor
            .selection
            .ids
            .iter()
            .map(|id| {
                let n = editor
                    .preview
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find(|n| &n.id == id)
                    .unwrap();
                match align {
                    FreeAlignment::Left => n.x,
                    FreeAlignment::Right => n.x + n.width,
                    FreeAlignment::CenterX => n.x + n.width / 2.0,
                    FreeAlignment::Top => n.y,
                    FreeAlignment::Bottom => n.y + n.height,
                    FreeAlignment::CenterY => n.y + n.height / 2.0,
                    _ => unreachable!(),
                }
            })
            .collect();
        assert!(values.iter().all(|v| (v - values[2]).abs() < 0.1));
        editor.document.undo();
        assert_eq!(editor.document.source(), source);
    }
}

#[test]
fn graphical_header_is_responsive_and_alignment_buttons_only_appear_for_nodes() {
    let (mut editor, ctx) = ready("semantics {\n A\n B\n}\n");
    for width in [260.0, 324.0, 348.0, 550.0, 800.0] {
        for count in [0, 1, 2] {
            editor.selection.ids = vec!["A".into(), "B".into()][..count].to_vec();
            let out = header_frame(&mut editor, &ctx, width, vec![]);
            for s in &out.shapes {
                if let egui::Shape::Text(t) = &s.shape {
                    assert!(
                        t.pos.x + t.galley.size().x <= width - 13.0,
                        "header exceeds {width}: {}",
                        t.galley.job.text
                    );
                    assert!(
                        !["Connect", "Add port", "Apply alignment", "Layout & editing"]
                            .contains(&t.galley.job.text.as_str())
                    );
                    if count < 2 {
                        assert!(
                            !graph_header::ALIGNMENTS
                                .iter()
                                .any(|(_, icon, _)| *icon == t.galley.job.text)
                        );
                    }
                }
            }
        }
    }
    editor.selection.ids = vec!["A".into(), "B".into()];
    click(
        &mut editor,
        &ctx,
        800.0,
        egui_phosphor::regular::ALIGN_LEFT_SIMPLE,
    );
    assert!(editor.alignment.is_none());
    changed(&mut editor, &ctx, 1);
}

#[test]
fn context_menu_does_not_change_selection_and_has_two_distinct_delete_actions() {
    let source = "semantics {\n A\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.ids = vec!["A".into(), "B".into()];
    canvas_frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let point = connection_geometry::node_rect(
        preview,
        editor.creation.canvas_rect.unwrap(),
        &preview.nodes[0],
    )
    .center();
    let event = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(point), event(true)],
    );
    canvas_frame(&mut editor, &ctx, vec![event(false)]);
    assert_eq!(editor.selection.ids, ["A", "B"]);
    assert!(editor.graph.context.is_some());
    let mut menu = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            ..Default::default()
        },
        |ui| editor.canvas(ui),
    );
    menu.textures_delta.clear();
    let labels: Vec<_> = menu
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(
                t.galley
                    .job
                    .text
                    .split_once("  ")
                    .map_or(t.galley.job.text.as_str(), |(_, label)| label),
            ),
            _ => None,
        })
        .collect();
    assert!(labels.contains(&"Add port…"));
    assert!(labels.contains(&"Delete this node…"));
    assert!(labels.contains(&"Delete selection…"));
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.starts_with("Delete "))
            .count(),
        2
    );
    // Deleting one of the selected nodes must retain the other selection.
    editor.graph_delete(vec!["A".into()]);
    assert_eq!(editor.selection.ids, ["A", "B"]);
    editor.confirm_graph_delete();
    changed(&mut editor, &ctx, 1);
    assert_eq!(editor.selection.ids, ["B"]);
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}

#[test]
fn alignment_guides_extend_past_selection_and_common_axes_remain_subtle() {
    let (mut editor, ctx) = ready("semantics {\n A\n B\n}\n");
    editor.selection.ids = vec!["A".into(), "B".into()];
    editor.graph.hover_align = Some(FreeAlignment::Left);
    let draw = |editor: &mut Editor| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events: vec![egui::Event::PointerMoved(egui::pos2(-10.0, -10.0))],
                ..Default::default()
            },
            |ui| {
                editor.canvas(ui);
                editor.graph.hover_align = Some(FreeAlignment::Left);
                editor.alignment_guides(ui, editor.creation.canvas_rect.unwrap());
            },
        );
        out.textures_delta.clear();
        out
    };
    let out = draw(&mut editor);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let reference = connection_geometry::node_rect(
        preview,
        canvas,
        preview.nodes.iter().find(|n| n.id == "B").unwrap(),
    );
    assert!(out.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::LineSegment {points,stroke} if stroke.color==appearance::SELECTED.gamma_multiply(0.5) && stroke.width==1.0
            && (points[0].x-reference.left()).abs()<0.01 && points[0].x==points[1].x && points[0].y<reference.top())));
    editor.align_selected(FreeAlignment::Left);
    changed(&mut editor, &ctx, 1);
    editor.graph.hover_align = None;
    editor.graph.flash = None;
    let out = draw(&mut editor);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::LineSegment {stroke,..} if stroke.color==appearance::SELECTED.gamma_multiply(0.22))));
    assert!(editor.alignment.is_none());
}
