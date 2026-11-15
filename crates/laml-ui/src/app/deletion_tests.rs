use super::tests::wait;
use super::*;
use laml_ui::delete_edit::ContainerMode;

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
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
            editor.canvas(ui);
            editor.deletion_shortcuts(ui.ctx());
            editor.graph_windows(ui.ctx());
            editor.deletion_window(ui.ctx());
        },
    );
    out.textures_delta.clear();
    out
}
fn text_center(out: &egui::FullOutput, label: &str) -> egui::Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.job.text == label
                    || t.galley.job.text.ends_with(&format!("  {label}")) =>
            {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing: {label}"))
}
fn click(editor: &mut Editor, ctx: &egui::Context, pos: egui::Pos2, button: egui::PointerButton) {
    for pressed in [true, false] {
        frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn right_click_on_arrow_opens_its_menu_and_confirmed_deletion_preserves_nodes() {
    let source = "semantics {\n A\n B\n A -> B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("B"), false);
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let line = preview.routes[0].source_line;
    let segment = preview.route_paths[0].paths[0]
        .windows(2)
        .max_by(|a, b| {
            let len = |p: &[laml::route::Point]| (p[0].x - p[1].x).hypot(p[0].y - p[1].y);
            len(a).total_cmp(&len(b))
        })
        .unwrap();
    let point = connection_geometry::screen(
        preview,
        canvas,
        (segment[0].x + segment[1].x) / 2.0,
        (segment[0].y + segment[1].y) / 2.0,
    );
    click(&mut editor, &ctx, point, egui::PointerButton::Secondary);
    assert!(
        matches!(editor.graph.context,Some((graph_context::Target::Relation(l),_,_)) if l==line)
    );
    assert_eq!(editor.selection.ids, ["B"]);
    let out = frame(&mut editor, &ctx, vec![]);
    click(
        &mut editor,
        &ctx,
        text_center(&out, "Delete this arrow…"),
        egui::PointerButton::Primary,
    );
    assert!(editor.deletion.is_some());
    assert_eq!(editor.document.source(), source);
    frame(&mut editor, &ctx, vec![]);
    let out = frame(&mut editor, &ctx, vec![]);
    text_center(&out, "Delete selection?");
    click(
        &mut editor,
        &ctx,
        text_center(&out, "Delete"),
        egui::PointerButton::Primary,
    );
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert!(editor.preview.as_ref().unwrap().routes.is_empty());
    assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), 2);
    assert_eq!(editor.selection.ids, ["B"]);
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
}
#[test]
fn port_and_label_picking_uses_the_same_visible_geometry_as_left_click() {
    let (mut editor, ctx) = ready("semantics {\n A {\n @P port Out\n }\n B\n A -> B: request\n}\n");
    frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let port = &preview.nodes.iter().find(|n| n.id == "A").unwrap().ports[0];
    let point = connection_geometry::screen(preview, canvas, port.x, port.y);
    assert_eq!(
        graph_context::hit(preview, canvas, point),
        Some(graph_context::Target::Port("P".into()))
    );
    let route = &preview.route_paths[0];
    let label = &route.labels[0];
    let point = connection_geometry::screen(preview, canvas, label.x, label.y);
    assert_eq!(
        graph_context::hit(preview, canvas, point),
        Some(graph_context::Target::Relation(route.line))
    );
}
#[test]
fn delete_and_backspace_open_confirmation_and_escape_cancels_without_editing() {
    for delete in [egui::Key::Delete, egui::Key::Backspace] {
        let source = "semantics {\n A\n B\n A -> B\n}\n";
        let (mut editor, ctx) = ready(source);
        editor.selection.click(Some("A"), false);
        frame(&mut editor, &ctx, vec![]);
        frame(&mut editor, &ctx, vec![key(delete)]);
        assert!(editor.deletion.is_some());
        assert_eq!(editor.document.source(), source);
        frame(&mut editor, &ctx, vec![]);
        frame(&mut editor, &ctx, vec![key(egui::Key::Escape)]);
        assert!(editor.deletion.is_none());
        assert_eq!(editor.document.source(), source);
    }
}
#[test]
fn text_input_focus_protects_the_graphical_selection_from_delete_shortcuts() {
    let (mut editor, ctx) = ready("semantics {\n A\n}\n");
    editor.selection.click(Some("A"), false);
    let mut text = "abc".to_owned();
    for events in [
        vec![],
        vec![key(egui::Key::Backspace)],
        vec![key(egui::Key::Delete)],
    ] {
        let mut out = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                let field = ui.text_edit_singleline(&mut text);
                field.request_focus();
                editor.deletion_shortcuts(ui.ctx());
            },
        );
        out.textures_delta.clear();
        assert!(editor.deletion.is_none());
    }
    assert_eq!(editor.document.revision(), 0);
}
#[test]
fn mixed_selection_deletion_is_one_transaction_and_never_lives_in_properties() {
    let source = "semantics {\n A\n B\n C {\n @P port Out\n }\n @r B -> C\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.ids = vec!["A".into()];
    editor.selection.ports = vec!["P".into()];
    editor.selection.relations = vec![editor.preview.as_ref().unwrap().diagram.relations[0].line];
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| editor.inspector(ui));
    out.textures_delta.clear();
    assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="Actions"||t.galley.job.text.starts_with("Delete selection"))));
    editor.propose_delete(ContainerMode::WithContents);
    assert_eq!(editor.document.source(), source);
    editor.confirm_delete();
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    let preview = editor.preview.as_ref().unwrap();
    assert_eq!(preview.nodes.len(), 2);
    assert!(preview.routes.is_empty());
    assert!(preview.nodes.iter().all(|n| n.ports.is_empty()));
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
}
#[test]
fn declaration_is_replaced_by_checked_semantic_fields_with_one_undo() {
    let source = "semantics {\n @Stable abstract +class A<T> {\n +run()\n }\n B\n Stable -> B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("Stable"), false);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| editor.inspector(ui));
    out.textures_delta.clear();
    let labels: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    for key in ["Type", "Visibility", "Abstract", "Templates"] {
        assert!(labels.contains(&key), "Missing {key}");
    }
    for key in ["Declaration", "Definition", "Actions", "Delete selection…"] {
        assert!(!labels.contains(&key));
    }
    editor.change_declaration(
        "Stable",
        laml_ui::declaration_fields::Change::Visibility(Some('-')),
    );
    assert_eq!(editor.document.source(), source);
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    let node = &editor.preview.as_ref().unwrap().diagram.nodes[0];
    assert_eq!(node.visibility, Some('-'));
    assert!(node.abstract_node);
    assert_eq!(node.templates, ["T"]);
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
}
