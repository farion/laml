use super::*;
#[test]
fn feasibility_query_does_not_hide_valid_uncommon_constructs() {
    let (_, editor, image) = super::tests::editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let point = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap();
    let recipes = recipes();
    let queries = recipes
        .iter()
        .map(|r| insertion_options::query(&editor, *r, Some(point)))
        .collect::<Vec<_>>();
    let choices =
        edit::sequence_options::build_with(editor.document.source(), &queries, false, |_| true);
    let titles = choices
        .iter()
        .map(|c| recipes[c.index].title)
        .collect::<Vec<_>>();
    assert!(titles.contains(&"Participant"), "{titles:?}");
    assert!(titles.contains(&"Critical fragment"), "{titles:?}");
}
#[test]
fn palette_filters_impossible_actions_and_only_offers_completable_references() {
    let source =
        "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A -> B\n second: B -> A\n}\n";
    let ctx = egui::Context::default();
    let mut editor = Editor::new(source.into(), None);
    editor.publish_preview(
        &ctx,
        laml_ui::preview::build(source, std::path::Path::new("palette.laml")).unwrap(),
    );
    let point = Point {
        dock: Dock {
            owner: "B".into(),
            before: 7,
            boundary: None,
        },
        at: egui::Pos2::ZERO,
        same_row: false,
        straighten: false,
    };
    let recipes = recipes();
    let queries = recipes
        .iter()
        .map(|r| insertion_options::query(&editor, *r, Some(&point)))
        .collect::<Vec<_>>();
    let backend = laml_ui::backend::local();
    let choices = edit::sequence_options::build_with(source, &queries, false, |_| true);
    let titles = choices
        .iter()
        .map(|c| recipes[c.index].title)
        .collect::<Vec<_>>();
    assert!(titles.contains(&"Note"));
    assert!(
        !titles.contains(&"Destroy participant"),
        "B is used after this insertion point"
    );
    assert!(
        !titles.contains(&"Frame coverage"),
        "No frame exists to select"
    );
    assert!(
        !titles.contains(&"Note placement"),
        "No note exists to select"
    );
    for choice in choices.iter().filter(|c| c.edits.is_some()) {
        let candidate = edit::apply(source, choice.edits.as_ref().unwrap()).unwrap();
        laml_ui::preview::build(&candidate, std::path::Path::new("palette.laml")).unwrap();
    }
    let recipe = recipes.iter().find(|r| r.title == "Coregion").unwrap();
    let mut end = point.clone();
    end.dock.before = 0;
    let query = insertion_options::query(&editor, *recipe, Some(&end));
    let first = backend.sequence_options(
        source,
        std::path::Path::new("palette.laml"),
        editor.compiler.presentation,
        std::slice::from_ref(&query),
        true,
    );
    assert!(!first.is_empty());
    assert!(
        first[0]
            .next
            .iter()
            .all(|s| matches!(s.as_str(), "@7.receive" | "@8.send")),
        "Coregion on B must not offer A's endpoints: {:?}",
        first[0].next
    );
    for value in &first[0].next {
        let mut query = query.clone();
        let (key, _) = query.keys.remove(0);
        query.values.insert(key, value.clone());
        let next = backend.sequence_options(
            source,
            std::path::Path::new("palette.laml"),
            editor.compiler.presentation,
            &[query],
            true,
        );
        assert!(
            !next.is_empty(),
            "Every offered first reference must have a valid completion"
        );
        assert!(
            !next[0].next.contains(value),
            "Cannot select the same occurrence twice"
        );
    }
}
#[test]
fn compact_and_expanded_menus_are_ready_in_the_first_frame_without_backend_requests() {
    let (ctx, mut editor, image) = super::tests::editor();
    let scene = Scene::new(editor.preview.as_ref().unwrap(), image);
    let point = scene
        .docks
        .iter()
        .find(|p| p.dock.owner == "A" && p.dock.boundary.is_none())
        .unwrap();
    for expanded in [false, true] {
        editor.notation_ui.sequence.expanded = expanded;
        let start = std::time::Instant::now();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            editor.sequence_add_menu(ui, point)
        });
        output.textures_delta.clear();
        eprintln!(
            "{} palette local context: {:?}",
            if expanded { "Expanded" } else { "Compact" },
            start.elapsed()
        );
        assert!(
            editor.notation_ui.sequence.options.is_none(),
            "Opening or expanding the menu must not submit a backend preflight"
        );
        assert!(editor.notation_ui.sequence.menu.is_some());
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.ends_with("  Note"))), "No waiting frame or loading spinner");
        assert!(editor.pending_command.is_none());
        assert!(editor.command_error.is_none());
    }
}
