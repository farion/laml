use super::tests::{canvas_frame, wait};
use super::*;
use laml_ui::{delete_edit::ContainerMode, element_edit::ElementKind};

fn ready(source: &str) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    (editor, ctx)
}
fn changed(editor: &mut Editor, ctx: &egui::Context, revision: u64) {
    wait(editor, ctx, |e| {
        e.document.revision() == revision
            || (e.pending_command.is_none() && e.command_error.is_some())
    });
    assert_eq!(editor.command_error, None);
    assert_eq!(editor.document.revision(), revision);
}
#[test]
fn palette_creates_packages_nested_shapes_and_relative_insertions() {
    let source = "semantics {\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.creation.palette.kind = ElementKind::Package;
    editor.create_element();
    changed(&mut editor, &ctx, 1);
    assert_eq!(editor.selection.ids, ["Group1"]);
    editor.creation.palette.parent = "Group1".into();
    editor.creation.palette.kind = ElementKind::Ellipse;
    editor.creation.label = "Service".into();
    editor.create_element();
    changed(&mut editor, &ctx, 2);
    assert_eq!(editor.selection.ids, ["Group1.Node1"]);
    editor.creation.palette.kind = ElementKind::Diamond;
    editor.creation.palette.relative = 4;
    editor.creation.label = "Choice".into();
    editor.create_element();
    changed(&mut editor, &ctx, 3);
    let diagram = &editor.preview.as_ref().unwrap().diagram;
    assert_eq!(diagram.nodes.len(), 3);
    assert_eq!(diagram.layout[0].first, "Group1.Node2");
    assert_eq!(diagram.layout[0].second, "Group1.Node1");
    editor.document.undo();
    editor.document.undo();
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn expanded_properties_are_compiler_verified_and_hidden_nodes_can_be_restored() {
    let (mut editor, ctx) = ready("semantics {\n A(label=\"Sample\")\n}\n");
    editor.selection.click(Some("A"), false);
    let changes = [
        ("fill", "rgb(18, 52, 86)", "#123456"),
        ("border-color", "#654321", "#654321"),
        ("color", "#112233", "#112233"),
        ("border-width", "3", "3"),
        ("font-size", "18", "18"),
        ("padding", "20", "20"),
        ("corner-radius", "5", "5"),
        ("min-width", "160", "160"),
        ("min-height", "100", "100"),
        ("width", "300", "300"),
        ("height", "200", "200"),
        ("shape", "ellipse", "ellipse"),
        ("icon", "folder", "folder"),
        ("icon-set", "phosphor", "phosphor"),
        ("symbol-rendering", "icons", "icons"),
        ("base-color", "#008855", "#008855"),
        ("display", "none", "none"),
        ("display", "block", "block"),
    ];
    for (i, (name, value, expected)) in changes.iter().enumerate() {
        editor.request_style(name, Some((*value).into()));
        changed(&mut editor, &ctx, i as u64 + 1);
        assert_eq!(
            editor.preview.as_ref().unwrap().node_properties["A"][*name]
                .value
                .as_deref(),
            Some(*expected),
            "{name}"
        );
        assert_eq!(editor.selection.ids, ["A"]);
    }
    assert_eq!(editor.preview.as_ref().unwrap().nodes.len(), 1);
}
#[test]
fn shadowed_or_notation_incompatible_properties_do_not_change_source() {
    let source = "semantics {\n package P {\n A\n }\n}\nstyling {\n node[P] node[A] {\n padding: 40\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click(Some("P.A"), false);
    editor.request_style("padding", Some("5".into()));
    wait(&mut editor, &ctx, |e| {
        e.command_error.is_some() && e.pending_command.is_none()
    });
    assert_eq!(editor.document.source(), source);
    editor.selection.click(Some("P"), false);
    editor.request_style("shape", Some("diamond".into()));
    assert_eq!(editor.document.source(), source);
}
#[test]
fn port_workflow_creates_connects_styles_renames_and_deletes_atomically() {
    let source = "semantics {\n A\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.create_port("A", "Out");
    changed(&mut editor, &ctx, 1);
    assert_eq!(editor.selection.ports, ["Port1"]);
    editor.request_port_style("fill", Some("rgb(18,52,86)"));
    changed(&mut editor, &ctx, 2);
    editor.change_port_sides(Some("top"));
    changed(&mut editor, &ctx, 3);
    editor.change_port_labels("Output");
    changed(&mut editor, &ctx, 4);
    editor.create_relation("Port1", "B");
    changed(&mut editor, &ctx, 5);
    editor.selection.click_port("Port1", false);
    editor.rename_port("Port1", "NewPort");
    changed(&mut editor, &ctx, 6);
    assert_eq!(editor.selection.ports, ["NewPort"]);
    assert!(editor.document.source().contains("NewPort -> B"));
    editor.propose_delete(ContainerMode::WithContents);
    assert!(editor.deletion.is_some());
    editor.confirm_delete();
    changed(&mut editor, &ctx, 7);
    assert_eq!(editor.preview.as_ref().unwrap().diagram.nodes.len(), 2);
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .relations
            .is_empty()
    );
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .nodes
            .iter()
            .all(|n| n.ports.is_empty())
    );
    editor.document.undo();
    assert!(editor.document.source().contains("NewPort -> B"));
    for _ in 0..6 {
        editor.document.undo();
    }
    assert_eq!(editor.document.source(), source);
}
fn screen(editor: &Editor, x: f64, y: f64) -> egui::Pos2 {
    let rect = editor.creation.canvas_rect.unwrap();
    let (left, top, right, bottom) = editor.preview.as_ref().unwrap().bounds;
    egui::pos2(
        rect.left() + ((x - left) / (right - left)) as f32 * rect.width(),
        rect.top() + ((y - top) / (bottom - top)) as f32 * rect.height(),
    )
}
fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}
#[test]
fn real_pointer_connects_and_selects_a_port_without_dragging_its_owner() {
    let source = "semantics {\n A {\n @P port Out\n }\n B\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.creation.connect = true;
    canvas_frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let p = &preview.nodes.iter().find(|n| n.id == "A").unwrap().ports[0];
    let start = screen(&editor, p.x, p.y);
    let b = preview.nodes.iter().find(|n| n.id == "B").unwrap();
    let target = screen(&editor, b.x + b.width / 2.0, b.y + b.height / 2.0);
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start), button(start, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    canvas_frame(&mut editor, &ctx, vec![button(target, false)]);
    changed(&mut editor, &ctx, 1);
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.relations[0].from,
        "P"
    );
    assert!(editor.preview.as_ref().unwrap().diagram.layout.is_empty());
    editor.creation.connect = false;
    canvas_frame(&mut editor, &ctx, vec![]);
    let p = &editor
        .preview
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == "A")
        .unwrap()
        .ports[0];
    let pos = screen(&editor, p.x, p.y);
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(pos), button(pos, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![button(pos, false)]);
    assert_eq!(editor.selection.ports, ["P"]);
    assert!(editor.selection.ids.is_empty());
}
#[test]
fn hidden_relations_are_restorable_from_their_properties() {
    let (mut editor, ctx) = ready("semantics {\n A\n B\n A -> B : request\n}\n");
    editor.selection.click_relation(4, false);
    editor.relation_property("display", Some("none"));
    changed(&mut editor, &ctx, 1);
    assert!(editor.preview.as_ref().unwrap().routes.is_empty());
    assert_eq!(editor.selection.relations, [4]);
    editor.relation_property("display", None);
    changed(&mut editor, &ctx, 2);
    assert_eq!(editor.preview.as_ref().unwrap().routes.len(), 1);
    editor.change_label_modes(Some("inline"));
    changed(&mut editor, &ctx, 3);
    assert_eq!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .relation_layout
            .len(),
        1
    );
}

#[test]
fn endpoint_handle_reconnects_to_an_existing_named_port() {
    let source = "semantics {\n A {\n @P port Out\n }\n B {\n @Q port In\n }\n C\n @r A -> C\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click_relation(9, false);
    canvas_frame(&mut editor, &ctx, vec![]);
    let preview = editor.preview.as_ref().unwrap();
    let endpoint = *preview
        .routes
        .iter()
        .find(|r| r.source_line == 9)
        .unwrap()
        .points
        .last()
        .unwrap();
    let q = &preview.nodes.iter().find(|n| n.id == "B").unwrap().ports[0];
    let start = screen(&editor, endpoint.x, endpoint.y);
    let target = screen(&editor, q.x, q.y);
    canvas_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start), button(start, true)],
    );
    canvas_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    canvas_frame(&mut editor, &ctx, vec![button(target, false)]);
    changed(&mut editor, &ctx, 1);
    assert_eq!(
        editor.preview.as_ref().unwrap().diagram.relations[0].to,
        "Q"
    );
    assert_eq!(
        editor.document.source(),
        source.replace("@r A -> C", "@r A -> Q")
    );
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .diagram
            .port_layout
            .is_empty()
    );
}
#[test]
fn multi_port_style_reset_exposes_different_inherited_values() {
    let source = "semantics {\n A {\n @P port Out(style-classes=red)\n }\n B {\n @Q port In(style-classes=blue)\n }\n}\nstyling {\n port.red {\n fill: #ff0000\n }\n port.blue {\n fill: #0000ff\n }\n}\n";
    let (mut editor, ctx) = ready(source);
    editor.selection.click_port("P", false);
    editor.selection.click_port("Q", true);
    editor.request_port_style("fill", Some("#123456"));
    changed(&mut editor, &ctx, 1);
    editor.request_port_style("fill", None);
    changed(&mut editor, &ctx, 2);
    let props = &editor.preview.as_ref().unwrap().port_properties;
    assert_eq!(props["P"]["fill"].value.as_deref(), Some("#ff0000"));
    assert_eq!(props["Q"]["fill"].value.as_deref(), Some("#0000ff"));
    editor.document.undo();
    assert_eq!(editor.document.source().matches("fill: #123456").count(), 2);
}

#[test]
fn zoomed_and_panned_canvas_keeps_picking_in_compiler_coordinates() {
    let source = "semantics {\n A\n B\n A -> B\n}\n";
    let (mut e, ctx) = ready(source);
    canvas_frame(&mut e, &ctx, vec![]);
    e.view.zoom_request = Some(1.15);
    e.view.pan = egui::vec2(20.0, 5.0);
    canvas_frame(&mut e, &ctx, vec![]);
    let a = &e.preview.as_ref().unwrap().nodes[0];
    let p = screen(&e, a.x, a.y);
    assert!(e.creation.canvas_rect.unwrap().contains(p));
    canvas_frame(
        &mut e,
        &ctx,
        vec![egui::Event::PointerMoved(p), button(p, true)],
    );
    canvas_frame(&mut e, &ctx, vec![button(p, false)]);
    assert_eq!(e.selection.ids, ["A"]);
    assert_eq!(e.document.source(), source);
    assert_eq!(e.document.revision(), 0);
}
