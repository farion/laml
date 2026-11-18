use super::*;
const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A -> B: Work\n second: B -> A: Done\n Frame1: opt \"condition\" {\n }\n Note1: note \"Text\"\n Continue1: continuation \"Continue\"\n}\nlayout {\n Note1 over A\n Continue1 spans A,B\n}\n";
#[test]
fn every_non_message_recipe_has_a_renderable_direct_insertion() {
    let editor = Editor::new(SOURCE.into(), None);
    for recipe in recipes() {
        let mut values = insertion::defaults(&editor, recipe);
        for (key, value) in [
            ("id", "Element1"),
            ("participant", "B"),
            ("from", "A"),
            ("to", "B"),
            ("frame", "Frame1"),
            ("note", "Note1"),
            ("continuation", "Continue1"),
            ("occurrence", "@7.receive"),
            ("first", "@7.send"),
            ("second", "@8.receive"),
            ("gap", "24"),
        ] {
            values.insert(key.into(), value.into());
        }
        if matches!(recipe.title, "Coregion" | "Occurrence order") {
            values.insert("first".into(), "@7.receive".into());
            values.insert("second".into(), "@8.send".into());
        }
        let source = if recipe.title == "Occurrence order" {
            SOURCE.replace(
                " Frame1:",
                " coregion B {\n first.receive\n second.send\n }\n Frame1:",
            )
        } else {
            SOURCE.into()
        };
        let edits = edit::sequence_create::create(&source, recipe, &values, 0, Some("B"))
            .unwrap_or_else(|e| panic!("{}: {e}", recipe.title));
        let candidate = edit::apply(&source, &edits).unwrap();
        laml_ui::preview::build(&candidate, std::path::Path::new("direct.laml"))
            .unwrap_or_else(|e| panic!("{}: {e}\n{candidate}", recipe.title));
    }
}
#[test]
fn anonymous_occurrences_are_named_only_in_the_completed_atomic_edit() {
    let source = SOURCE.replace("first: ", "").replace("second: ", "");
    let recipe = recipes()
        .into_iter()
        .find(|r| r.title == "Duration constraint")
        .unwrap();
    let mut values = Editor::new(source.clone(), None).construct_defaults(recipe);
    values.insert("first".into(), "@7.send".into());
    values.insert("second".into(), "@8.receive".into());
    let edits = edit::sequence_create::create(&source, recipe, &values, 0, Some("A")).unwrap();
    let candidate = edit::apply(&source, &edits).unwrap();
    assert!(candidate.contains("Message1: A -> B: Work"));
    assert!(candidate.contains("Message2: B -> A: Done"));
    assert!(candidate.contains("between[Message1.send,Message2.receive]"));
    let mut document = laml_ui::document::Document::new(source.clone());
    document
        .apply(document.revision(), "Insert annotation", edits)
        .unwrap();
    document.undo();
    assert_eq!(document.source(), source);
}
#[test]
fn direct_insertion_keeps_operand_and_decomposition_scope() {
    let recipe = recipes()
        .into_iter()
        .find(|r| r.title == "Optional fragment")
        .unwrap();
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n decompose A \"Inside\" {\n  Child\n  Other\n  alt \"condition\" {\n   Child ->> Other\n  }\n  else \"else\" {\n  }\n }\n}\n";
    let values = Editor::new(source.into(), None).construct_defaults(recipe);
    let edits =
        edit::sequence_create::create(source, recipe, &values, 10, Some("A.Child")).unwrap();
    let candidate = edit::apply(source, &edits).unwrap();
    assert!(candidate.find("opt ").unwrap() < candidate.find("Child ->>").unwrap());
    laml_ui::preview::build(&candidate, std::path::Path::new("nested.laml")).unwrap();
}
#[test]
fn graphical_reference_selection_is_nonmodal_and_escape_does_not_name_or_edit() {
    use super::tests::{editor, frame};
    let (ctx, mut editor, image) = editor();
    frame(&ctx, &mut editor, image, vec![]);
    let recipe = recipes()
        .into_iter()
        .find(|r| r.title == "Duration constraint")
        .unwrap();
    let original = editor.document.source().to_owned();
    editor.sequence_begin_insert(recipe, None);
    frame(&ctx, &mut editor, image, vec![]);
    assert!(editor.notation_ui.sequence.picker.is_some());
    assert!(!dialogs::blocked(&ctx));
    assert_eq!(editor.document.source(), original);
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert!(editor.notation_ui.sequence.picker.is_none());
    assert!(editor.pending_command.is_none());
    assert_eq!(editor.document.source(), original);
}
#[test]
fn context_menu_does_not_offer_the_insertion_catalog() {
    use super::tests::{editor, frame};
    let (ctx, mut editor, image) = editor();
    let at = image.center();
    frame(
        &ctx,
        &mut editor,
        image,
        vec![egui::Event::PointerMoved(at)],
    );
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut editor,
            image,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    let output = frame(&ctx, &mut editor, image, vec![]);
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.ends_with("Elements…")));
    assert!(!texts.iter().any(|t| {
        [
            "Participant",
            "Activation",
            "Note",
            "Fragments",
            "Annotations",
            "Layout",
            "Add construct…",
            "Synchronous message",
        ]
        .iter()
        .any(|title| t.ends_with(title))
    }));
    assert!(editor.notation_ui.create.is_none());
}
#[test]
fn sequence_header_retains_elements_but_has_no_plus() {
    let (ctx, mut editor, _) = super::tests::editor();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        editor.graph_heading(ui);
    });
    output.textures_delta.clear();
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"Elements…"), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t.contains(egui_phosphor::regular::PLUS) || t.contains("Add construct"))
    );
}

/// Include the real viewport gate: calling sequence_canvas directly misses this bug.
fn clipped_frame(
    ctx: &egui::Context,
    editor: &mut Editor,
    image: egui::Rect,
    viewport: egui::Rect,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(viewport));
            canvas.set_clip_rect(viewport);
            let response = canvas.interact(
                viewport,
                canvas.make_persistent_id("clipped-canvas"),
                egui::Sense::click_and_drag(),
            );
            let navigation = editor.viewport_input(&mut canvas, viewport, &response, image.size());
            editor.sequence_canvas(&mut canvas, image, &response, navigation);
        },
    );
    output.textures_delta.clear();
    output
}
fn menu_position(output: &egui::FullOutput, title: &str) -> Option<egui::Pos2> {
    output.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.job.text.ends_with(&format!("  {title}")) => {
            Some(t.pos + t.galley.size() * 0.5)
        }
        _ => None,
    })
}
fn wait_clipped(
    ctx: &egui::Context,
    editor: &mut Editor,
    image: egui::Rect,
    viewport: egui::Rect,
    title: &str,
) -> egui::FullOutput {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let output = clipped_frame(ctx, editor, image, viewport, vec![]);
        if menu_position(&output, title).is_some() {
            return output;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Timed out waiting for {title}"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
fn click_clipped(
    ctx: &egui::Context,
    editor: &mut Editor,
    image: egui::Rect,
    viewport: egui::Rect,
    at: egui::Pos2,
) {
    clipped_frame(
        ctx,
        editor,
        image,
        viewport,
        vec![egui::Event::PointerMoved(at)],
    );
    for pressed in [true, false] {
        clipped_frame(
            ctx,
            editor,
            image,
            viewport,
            super::tests::pointer(at, pressed),
        );
    }
}
#[test]
fn canvas_menus_and_submenus_remain_clickable_outside_the_canvas() {
    for context in [false, true] {
        for submenu in [false, true] {
            let (ctx, mut editor, image) = super::tests::editor();
            let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
            let point = scene
                .docks
                .iter()
                .find(|p| p.dock.owner == "B" && p.dock.boundary.is_none())
                .unwrap();
            let at = if context {
                let r = scene
                    .regions
                    .iter()
                    .find(|r| r.kind == "message" && !r.path.is_empty())
                    .unwrap();
                scene.point((r.path[0].0 + r.path[1].0) / 2.0, r.path[0].1)
            } else {
                point.at + egui::vec2(22.0, 0.0)
            };
            let viewport =
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(at.x + 8.0, 850.0));
            clipped_frame(
                &ctx,
                &mut editor,
                image,
                viewport,
                vec![egui::Event::PointerMoved(at)],
            );
            if context {
                for pressed in [true, false] {
                    clipped_frame(
                        &ctx,
                        &mut editor,
                        image,
                        viewport,
                        vec![egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Secondary,
                            pressed,
                            modifiers: Default::default(),
                        }],
                    );
                }
            } else {
                editor.notation_ui.sequence.insert = Some(point.clone());
            }
            let title = match (context, submenu) {
                (true, true) => "Wrap in fragment",
                (true, false) => "Elements…",
                (false, true) => "More…",
                _ => "Note",
            };
            let output = wait_clipped(&ctx, &mut editor, image, viewport, title);
            let mut item = menu_position(&output, title).unwrap();
            assert!(!viewport.contains(item));
            if !context && submenu {
                click_clipped(&ctx, &mut editor, image, viewport, item);
                let output = wait_clipped(&ctx, &mut editor, image, viewport, "Fragments");
                assert!(
                    editor.notation_ui.sequence.expanded,
                    "More expands instead of dismissing the popup"
                );
                item = menu_position(&output, "Fragments").unwrap();
            }
            let target = if submenu {
                clipped_frame(
                    &ctx,
                    &mut editor,
                    image,
                    viewport,
                    vec![egui::Event::PointerMoved(item)],
                );
                let title = if context { "Loop" } else { "Critical fragment" };
                let output = wait_clipped(&ctx, &mut editor, image, viewport, title);
                menu_position(&output, title).unwrap()
            } else {
                item
            };
            assert!(!viewport.contains(target));
            click_clipped(&ctx, &mut editor, image, viewport, target);
            if context && !submenu {
                assert!(editor.notation_ui.navigator);
            } else {
                assert!(
                    editor.pending_command.is_some(),
                    "Outside click must apply its action: context={context}, submenu={submenu}"
                );
            }
        }
    }
}
#[test]
fn plus_starts_compact_and_more_only_expands_the_current_menu() {
    use super::tests::{editor, frame, pointer, wait_for_menu};
    let (ctx, mut editor, image) = editor();
    frame(&ctx, &mut editor, image, vec![]);
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let point = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap();
    editor.notation_ui.sequence.insert = Some(point.clone());
    let output = wait_for_menu(&ctx, &mut editor, image, "More…");
    assert!(menu_position(&output, "Note").is_some());
    assert!(menu_position(&output, "Participant").is_none());
    assert!(menu_position(&output, "Fragments").is_none());
    let more = menu_position(&output, "More…").unwrap();
    frame(&ctx, &mut editor, image, pointer(more, true));
    frame(&ctx, &mut editor, image, pointer(more, false));
    let output = wait_for_menu(&ctx, &mut editor, image, "Fragments");
    assert!(menu_position(&output, "Participant").is_some());
    assert!(editor.notation_ui.sequence.insert.is_some());
    assert!(!editor.document.is_dirty());
    assert!(editor.pending_command.is_none());
}
