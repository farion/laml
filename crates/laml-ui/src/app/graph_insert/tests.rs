use super::*;
mod sizing;
#[test]
fn insertion_icons_are_unique_and_rectangle_is_not_a_fallback() {
    use laml_ui::element_edit::ElementKind;
    let mut icons = std::collections::BTreeSet::new();
    for kind in ElementKind::ALL {
        if let Some(icon) = menu_icon(Item::Element(kind)) {
            assert!(icons.insert(icon), "Duplicate icon for {kind:?}");
            if icon == egui_phosphor::regular::SQUARE {
                assert_eq!(kind, ElementKind::Rectangle);
            }
        }
    }
    assert_eq!(
        menu_icon(Item::Element(ElementKind::Rectangle)),
        Some(egui_phosphor::regular::SQUARE)
    );
    assert_eq!(menu_icon(Item::Element(ElementKind::Class)), None);
}
#[test]
fn entries_without_icons_keep_the_same_text_column() {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let job = label(ui, "Class", None);
        assert_eq!(job.text, crate::i18n::tr("Class"));
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.sections[0].leading_space, 28.0);
        let plain = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let plain_x = plain.rows[0].glyphs.last().unwrap().pos.x;
        for kind in laml_ui::element_edit::ElementKind::ALL {
            if let Some(icon) = menu_icon(Item::Element(kind)) {
                let job = label(ui, "Class", Some(icon));
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                let icon_x = galley.rows[0].glyphs.last().unwrap().pos.x;
                assert!(
                    (plain_x - icon_x).abs() < 0.1,
                    "{kind:?}: {plain_x} != {icon_x}"
                );
            }
        }
    });
    output.textures_delta.clear();
}
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
fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}
fn ready() -> (Editor, egui::Context) {
    let source = "semantics {\n A\n B\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    for _ in 0..3 {
        frame(&mut editor, &ctx, vec![]);
    }
    (editor, ctx)
}
#[test]
fn empty_package_plus_is_hover_only_and_menu_keeps_it_visible() {
    let source = "semantics {\n package P {\n }\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    for _ in 0..3 {
        frame(&mut editor, &ctx, vec![]);
    }
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let at = empty::centers(preview, canvas)[0].1;
    assert!(empty::hover_centers(preview, canvas, None).is_empty());
    assert!(empty::hover_centers(preview, canvas, Some(egui::pos2(-100.0, -100.0))).is_empty());
    assert_eq!(empty::hover_centers(preview, canvas, Some(at)).len(), 1);
    assert!(preview.nodes[0].width >= 200.0);
    assert!(preview.nodes[0].height >= 140.0);
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(at)]);
    frame(&mut editor, &ctx, vec![button(at, true)]);
    frame(&mut editor, &ctx, vec![button(at, false)]);
    let menu = editor
        .graph
        .insertion
        .as_ref()
        .expect("Empty body plus opens menu");
    assert!(menu.inside);
    assert!(!menu.choices.is_empty());
    assert_eq!(editor.document.revision(), 0);
    assert!(editor.graph.gesture.is_none());
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(egui::pos2(950.0, 650.0))],
    );
    assert!(
        editor
            .graph
            .insertion
            .as_ref()
            .is_some_and(|menu| menu.inside)
    );
}
#[test]
fn four_pluses_open_direct_menu_without_starting_a_gesture() {
    for side in [
        Placement::Left,
        Placement::Right,
        Placement::Above,
        Placement::Below,
    ] {
        let (mut editor, ctx) = ready();
        let preview = editor.preview.as_ref().unwrap();
        let canvas = editor.creation.canvas_rect.unwrap();
        let node = &preview.nodes[0];
        let center = connection_geometry::node_rect(preview, canvas, node).center();
        let at = geometry::positions(preview, canvas, node)
            .into_iter()
            .find(|(s, _)| *s == side)
            .unwrap()
            .1;
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(center)]);
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(at)]);
        frame(&mut editor, &ctx, vec![button(at, true)]);
        frame(&mut editor, &ctx, vec![button(at, false)]);
        let menu = editor
            .graph
            .insertion
            .as_ref()
            .expect("Plus opens the insertion menu directly");
        assert_eq!(menu.side, side);
        assert!(!menu.expanded);
        assert!(!menu.choices.is_empty());
        assert!(editor.graph.gesture.is_none());
        assert!(editor.placement.drag.is_none());
        assert_eq!(editor.document.revision(), 0);
    }
}
#[test]
fn nested_node_pluses_keep_priority_over_enclosing_packages() {
    for owner in ["Outer", "Inner"] {
        for side in [
            Placement::Left,
            Placement::Right,
            Placement::Above,
            Placement::Below,
        ] {
            let source = if owner == "Outer" {
                "semantics {\n package Outer {\n A\n }\n}\n"
            } else {
                "semantics {\n package Outer {\n package Inner {\n A\n }\n }\n}\n"
            };
            let mut editor = Editor::new(source.into(), None);
            let ctx = egui::Context::default();
            egui_extras::install_image_loaders(&ctx);
            let preview = laml_ui::preview::build(source, &editor.path).unwrap();
            editor.publish_preview(&ctx, preview);
            for _ in 0..3 {
                frame(&mut editor, &ctx, vec![]);
            }
            let preview = editor.preview.as_ref().unwrap();
            let canvas = editor.creation.canvas_rect.unwrap();
            let node = preview.nodes.iter().find(|n| n.label == "A").unwrap();
            let id = node.id.clone();
            let center = connection_geometry::node_rect(preview, canvas, node).center();
            let at = geometry::positions(preview, canvas, node)
                .into_iter()
                .find(|(s, _)| *s == side)
                .unwrap()
                .1;
            frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(center)]);
            // Walk through the gap, where the package body is the direct hit.
            for step in 1..=8 {
                let pos = center.lerp(at, step as f32 / 8.0);
                frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(pos)]);
                assert_eq!(
                    editor.graph.hover_node.as_deref(),
                    Some(id.as_str()),
                    "{owner} {side:?} step {step}"
                );
            }
            frame(&mut editor, &ctx, vec![button(at, true)]);
            frame(&mut editor, &ctx, vec![button(at, false)]);
            let menu = editor
                .graph
                .insertion
                .as_ref()
                .expect("Nested node plus opens menu");
            assert_eq!(menu.from, id);
            assert_eq!(menu.side, side);
            assert!(!menu.inside);
            assert!(editor.graph.gesture.is_none());
            assert!(editor.placement.drag.is_none());
            assert_eq!(editor.document.revision(), 0);
        }
    }
}
#[test]
fn automatic_origin_is_separate_from_node_movement() {
    let (mut editor, ctx) = ready();
    let preview = editor.preview.as_ref().unwrap();
    let canvas = editor.creation.canvas_rect.unwrap();
    let from = connection_geometry::handles(preview, canvas, &preview.nodes[0], true)
        .into_iter()
        .find(|h| h.dock.side.is_none())
        .unwrap()
        .point;
    let target = connection_geometry::node_rect(preview, canvas, &preview.nodes[1]).center();
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(from), button(from, true)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    assert!(
        matches!(&editor.graph.gesture.as_ref().unwrap().kind,connection_gesture::Kind::New{from,..} if from.side.is_none())
    );
    assert!(editor.placement.drag.is_none());
}
#[test]
fn a_used_node_dock_remains_available_for_new_connections() {
    let source = "semantics {\n A\n B\n A -> B\n}\n";
    let preview = laml_ui::preview::build(source, std::path::Path::new("test.laml")).unwrap();
    let canvas = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(
            (preview.bounds.2 - preview.bounds.0) as f32,
            (preview.bounds.3 - preview.bounds.1) as f32,
        ),
    );
    let node = preview.nodes.iter().find(|n| n.id == "A").unwrap();
    let dock = connection_geometry::handles(&preview, canvas, node, true)
        .into_iter()
        .find(|h| h.dock.side == Some("right"))
        .unwrap();
    assert!(connection_geometry::endpoint_at(&preview, canvas, dock.point).is_none());
    assert_eq!(
        connection_geometry::source_at(&preview, canvas, dock.point)
            .unwrap()
            .side,
        Some("right")
    );
}
#[test]
fn invalid_drop_toast_does_not_open_a_modal_or_change_source() {
    let (mut editor, ctx) = ready();
    let source = editor.document.source().to_owned();
    editor.placement.drag = Some(("A".into(), 0));
    editor.finish_placement(&ctx);
    assert!(editor.graph.toast.is_some());
    assert!(editor.command_error.is_none());
    assert_eq!(editor.document.source(), source);
    let mut output = ctx.run_ui(Default::default(), |ui| editor.toast_overlay(ui.ctx()));
    output.textures_delta.clear();
    assert!(!dialogs::blocked(&ctx));
}
