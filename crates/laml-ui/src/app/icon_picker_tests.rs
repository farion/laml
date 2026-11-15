use super::inspector_tests::ready;
use super::tests::wait;
use super::*;

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1568.0, 975.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            editor.file_events(ui.ctx());
            editor.history_shortcuts(ui.ctx());
            editor.shell(ui);
            editor.icon_picker_window(ui.ctx());
            editor.command_error_window(ui.ctx());
        },
    );
    out.textures_delta.clear();
    out
}
#[test]
fn picker_has_real_icon_tiles_search_and_visible_confirmation() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    editor.open_icon_picker(vec!["A".into()], None);
    let mut out = frame(&mut editor, &ctx, vec![]);
    for _ in 0..4 {
        out = frame(&mut editor, &ctx, vec![]);
    }
    assert!(dialogs::blocked(&ctx));
    for label in ["Cancel", "Use icon", "acorn", "Architecture"] {
        let (shape, text) = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => Some((s, t)),
                _ => None,
            })
            .unwrap();
        let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
        assert!(
            shape.clip_rect.contains_rect(rect),
            "{label}: rect {rect:?}, clip {:?}",
            shape.clip_rect
        );
    }
    assert!(out.shapes.iter().any(
        |s| matches!(&s.shape,egui::Shape::Mesh(m) if m.texture_id!=egui::TextureId::default())
    ));
    assert_eq!(editor.document.revision(), 0);
}

fn text_center(out: &egui::FullOutput, label: &str) -> egui::Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == label => {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing text: {label}"))
}
fn click(editor: &mut Editor, ctx: &egui::Context, pos: egui::Pos2) -> egui::FullOutput {
    frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
    );
    frame(
        editor,
        ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    )
}
#[test]
fn selecting_a_tile_then_confirming_uses_the_actual_icon_and_one_undo() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    let source = editor.document.source().to_owned();
    editor.open_icon_picker(vec!["A".into()], None);
    frame(&mut editor, &ctx, vec![]);
    let out = frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, text_center(&out, "acorn"));
    assert_eq!(
        editor
            .graph
            .icon_picker
            .as_ref()
            .unwrap()
            .selected
            .as_deref(),
        Some("acorn")
    );
    assert_eq!(editor.document.source(), source);
    let out = frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, text_center(&out, "Use icon"));
    assert!(editor.graph.icon_picker.is_none());
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.id == "A" && node.icons.iter().any(|icon| icon == "acorn"))
    );
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
}
#[test]
fn search_uses_the_entire_catalogue_and_categories_and_empty_results() {
    let (mut editor, _) = ready();
    editor.open_icon_picker(vec!["A".into()], None);
    let picker = editor.graph.icon_picker.as_mut().unwrap();
    assert_eq!(picker.filtered().len(), laml::bundled_icon_names().count());
    for name in picker.filtered() {
        assert!(laml::bundled_icon_svg(name).unwrap().contains("<svg"));
    }
    picker.query = "  YOUTUBE  logo ".into();
    assert_eq!(picker.filtered(), vec!["youtube-logo"]);
    picker.query = "cloud".into();
    picker.category = icon_picker::Category::Architecture;
    let names = picker.filtered();
    assert!(!names.is_empty());
    assert!(names.iter().all(|name| name.contains("cloud")));
    picker.query = "there-is-no-such-icon".into();
    assert!(picker.filtered().is_empty());
}
#[test]
fn picker_cancel_and_stale_submission_cannot_change_source_or_other_selections() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    let source = editor.document.source().to_owned();
    editor.open_icon_picker(vec!["A".into()], Some("cloud".into()));
    frame(&mut editor, &ctx, vec![]);
    let out = frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, text_center(&out, "Cancel"));
    assert!(editor.graph.icon_picker.is_none());
    assert_eq!(editor.document.source(), source);
    editor.open_icon_picker(vec!["A".into()], Some("cloud".into()));
    let picker = editor.graph.icon_picker.take().unwrap();
    editor.selection.click(Some("B"), false);
    assert!(editor.submit_icon_picker(&picker).is_err());
    assert_eq!(editor.document.source(), source);
    editor.selection.click(Some("A"), false);
    editor
        .document
        .replace_text(format!("// changed\n{source}"))
        .unwrap();
    let changed = editor.document.source().to_owned();
    assert!(editor.submit_icon_picker(&picker).is_err());
    assert_eq!(editor.document.source(), changed);
    assert!(editor.pending_command.is_none());
}
#[test]
fn access_icons_automatically_set_phosphor_for_all_targets_atomically() {
    let source = "// Grüße bleiben erhalten\r\nsemantics {\r\n +class A {\r\n +run()\r\n -id: Int\r\n }\r\n B\r\n}\r\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    editor.selection.ids = vec!["A".into(), "B".into()];
    editor.request_style("symbol-rendering", Some("icons".into()));
    assert_eq!(editor.document.source(), source);
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    let updated = editor.document.source().to_owned();
    assert!(updated.starts_with(source));
    assert_eq!(updated.matches("icon-set: phosphor").count(), 2);
    assert_eq!(updated.matches("symbol-rendering: icons").count(), 2);
    assert!(
        editor
            .preview
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.id == "A" && node.icons.iter().any(|icon| icon == "globe"))
    );
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
}
#[test]
fn validation_errors_are_modal_and_never_inspector_or_canvas_banners() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    let source = editor.document.source().to_owned();
    editor.request_style("width", Some("not-a-number".into()));
    let error = editor.command_error.clone().unwrap();
    // Rendering only the document shell must not paint the error at either pane top.
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
    assert!(
        !out.shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==error))
    );
    frame(&mut editor, &ctx, vec![]);
    let out = frame(&mut editor, &ctx, vec![]);
    assert!(dialogs::blocked(&ctx));
    text_center(&out, "Change could not be applied");
    assert_eq!(
        out.shapes
            .iter()
            .filter(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==error))
            .count(),
        1
    );
    assert_eq!(editor.document.source(), source);
}

#[test]
fn member_access_icons_set_their_own_dependency_and_render_in_one_transaction() {
    let source = "semantics {\n A {\n -id: Int\n }\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    editor.changed = Instant::now() - Duration::from_secs(1);
    wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    let edits =
        laml_ui::structure::member_style(source, "A", "A::id", "symbol-rendering", Some("icons"))
            .unwrap();
    editor.submit_command(Command {
        edits,
        label: "Member visibility icons".into(),
        expectation: worker::Expectation::MemberStyle {
            owner: "A".into(),
            address: "A::id".into(),
            name: "symbol-rendering".into(),
            value: Some("icons".into()),
        },
    });
    wait(&mut editor, &ctx, |e| {
        e.document.revision() == 1 || e.command_error.is_some()
    });
    assert_eq!(editor.command_error, None);
    assert!(editor.document.source().contains("icon-set: phosphor"));
    assert!(
        editor.preview.as_ref().unwrap().nodes[0]
            .body_styles
            .iter()
            .any(|row| row.icon.as_deref() == Some("lock"))
    );
    assert!(editor.document.undo());
    assert_eq!(editor.document.source(), source);
    assert!(editor.document.undo_label().is_none());
}

#[test]
fn picker_search_accepts_typing_and_escape_cancels_without_background_shortcuts() {
    let (mut editor, ctx) = ready();
    egui_extras::install_image_loaders(&ctx);
    editor.open_icon_picker(vec!["A".into()], None);
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![egui::Event::Text("cloud".into())]);
    assert_eq!(editor.graph.icon_picker.as_ref().unwrap().query, "cloud");
    let key = |key, modifiers| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::N, egui::Modifiers::COMMAND)],
    );
    assert!(editor.workspace_action.is_none());
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(editor.graph.icon_picker.is_none());
    assert_eq!(editor.document.revision(), 0);
}
