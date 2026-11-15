use super::*;
const SOURCE: &str = "semantics {\n A\n}\n";

fn open(app: &mut Workspace, path: PathBuf) {
    app.open_path(path);
    let end = Instant::now() + Duration::from_secs(10);
    while app.opening.is_some() {
        assert!(Instant::now() < end, "open timed out");
        app.poll_open();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn workspace() -> (Workspace, egui::Context) {
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    (Workspace::new(Editor::new(SOURCE.into(), None)), ctx)
}

fn frame(app: &mut Workspace, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1568.0, 975.0),
            )),
            events,
            ..Default::default()
        },
        |ui| app.tick(ui),
    );
    output.textures_delta.clear();
    output
}

fn click(app: &mut Workspace, ctx: &egui::Context, label: &str) {
    let mut output = frame(app, ctx, vec![]);
    for _ in 0..2 {
        output = frame(app, ctx, vec![]);
    }
    let point = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == label => {
                Some(t.pos + t.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing button: {label}"));
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        app,
        ctx,
        vec![egui::Event::PointerMoved(point), button(true)],
    );
    frame(app, ctx, vec![button(false)]);
}

#[test]
fn new_tabs_keep_independent_dirty_buffers_history_selection_and_zoom() {
    let (mut app, ctx) = workspace();
    let first = &mut app.tabs[0];
    first
        .document
        .replace_text(format!("{SOURCE}// first\n"))
        .unwrap();
    first.selection.ids.push("A".into());
    first.view.zoom = Some(1.5);
    first.view.pan = egui::vec2(4.0, 8.0);
    let first_id = first.id;
    app.new_tab();
    app.new_tab();
    assert_eq!(app.tabs.len(), 3);
    assert_ne!(app.tabs[1].path, app.tabs[2].path);
    app.tabs[1]
        .document
        .replace_text("semantics {\n B\n}\n".into())
        .unwrap();
    assert!(app.tabs[1].document.undo());
    assert_eq!(app.tabs[0].id, first_id);
    assert_eq!(app.tabs[0].view.zoom, Some(1.5));
    assert_eq!(app.tabs[0].view.pan, egui::vec2(4.0, 8.0));
    assert_eq!(app.tabs[0].selection.ids, ["A"]);
    assert!(app.tabs[0].document.is_dirty());
    assert!(app.tabs[0].document.undo());
    assert_eq!(app.tabs[0].document.source(), SOURCE);
    for tab in &mut app.tabs {
        let preview = laml_ui::preview::build(tab.document.source(), &tab.path).unwrap();
        tab.publish_preview(&ctx, preview);
    }
    assert_ne!(
        app.tabs[0].image.as_ref().unwrap().0,
        app.tabs[1].image.as_ref().unwrap().0
    );
}

#[test]
fn opening_an_open_file_selects_its_dirty_tab_and_failed_open_preserves_all_tabs() {
    let (mut app, _) = workspace();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.laml");
    std::fs::write(&path, SOURCE).unwrap();
    open(&mut app, path.clone());
    app.tabs[1]
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    app.active = 0;
    open(&mut app, dir.path().join(".").join("test.laml"));
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    assert!(app.tabs[1].document.source().contains("// mine"));
    assert_eq!(app.tabs[0].other_paths, [path.canonicalize().unwrap()]);
    open(&mut app, dir.path().join("missing.laml"));
    assert!(app.error.is_some());
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
}

#[test]
fn dirty_tab_close_can_cancel_or_discard_without_losing_other_documents() {
    let (mut app, ctx) = workspace();
    app.tabs[0]
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    app.new_tab();
    let other = app.tabs[1].id;
    app.request_close(0, &ctx);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 0);
    click(&mut app, &ctx, "Keep editing");
    assert!(app.waiting_close.is_none());
    assert_eq!(app.tabs.len(), 2);
    assert!(app.tabs[0].document.is_dirty());
    app.request_close(0, &ctx);
    click(&mut app, &ctx, "Discard and continue");
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.tabs[0].id, other);
}

#[test]
fn save_and_close_only_saves_the_requested_tab() {
    let (mut app, ctx) = workspace();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.laml");
    std::fs::write(&path, SOURCE).unwrap();
    open(&mut app, path.clone());
    let source = format!("{SOURCE}// saved\n");
    app.tabs[1].document.replace_text(source.clone()).unwrap();
    app.tabs[0]
        .document
        .replace_text(format!("{SOURCE}// unrelated\n"))
        .unwrap();
    app.request_close(1, &ctx);
    click(&mut app, &ctx, "Save and continue");
    let end = Instant::now() + Duration::from_secs(10);
    while app.tabs.len() == 2 {
        assert!(Instant::now() < end, "save/close timed out");
        frame(&mut app, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    assert_eq!(app.tabs.len(), 1);
    assert!(app.tabs[0].document.is_dirty());
}

#[test]
fn application_close_checks_inactive_dirty_tabs_and_cancel_stops_closing() {
    let (mut app, ctx) = workspace();
    app.tabs[0]
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    app.new_tab();
    app.closing_all = true;
    app.close_all_tick(&ctx); // clean active tab closes first
    app.close_all_tick(&ctx); // inactive dirty tab must be confirmed
    assert_eq!(app.tabs.len(), 1);
    assert!(app.waiting_close.is_some());
    assert!(!app.allow_close);
    click(&mut app, &ctx, "Keep editing");
    assert!(!app.closing_all);
    assert!(!app.allow_close);
    assert!(app.tabs[0].document.is_dirty());
    app.closing_all = true;
    app.close_all_tick(&ctx);
    click(&mut app, &ctx, "Discard and continue");
    assert!(app.allow_close);
    assert!(app.tabs.is_empty());
}

#[test]
fn closing_last_clean_tab_creates_a_new_untitled_document() {
    let (mut app, ctx) = workspace();
    let old = app.tabs[0].id;
    app.request_close(0, &ctx);
    assert_eq!(app.tabs.len(), 1);
    assert_ne!(app.tabs[0].id, old);
    assert!(!app.allow_close);
    assert!(app.tabs[0].file.path.is_none());
}

#[test]
fn global_chrome_has_phosphor_actions_tabs_and_a_settings_window() {
    let (mut app, ctx) = workspace();
    app.new_tab();
    let output = frame(&mut app, &ctx, vec![]);
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    for label in [
        "LAML Editor",
        PLUS,
        FOLDER_OPEN,
        GEAR_SIX,
        X,
        "untitled.laml",
        "untitled-2.laml",
    ] {
        assert!(texts.contains(&label), "missing {label}");
    }
    for shape in &output.shapes {
        if let egui::Shape::Text(text) = &shape.shape
            && [PLUS, FOLDER_OPEN, GEAR_SIX, X].contains(&text.galley.job.text.as_str())
        {
            assert_eq!(
                text.galley.job.sections[0].format.font_id.family,
                egui::FontFamily::Name("phosphor".into())
            );
        }
    }
    click(&mut app, &ctx, GEAR_SIX);
    assert!(app.settings_open);
    let output = frame(&mut app, &ctx, vec![]);
    assert!(output.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Text(t) if t.galley.job.text == "Settings")));
    assert_eq!(app.tabs.len(), 2);
}

#[test]
fn title_new_button_and_file_tabs_dispatch_to_the_right_document() {
    let (mut app, ctx) = workspace();
    app.tabs[0]
        .document
        .replace_text(format!("{SOURCE}// mine\n"))
        .unwrap();
    click(&mut app, &ctx, PLUS);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    assert!(app.tabs[0].document.is_dirty());
    click(&mut app, &ctx, "untitled.laml •");
    assert_eq!(app.active, 0);
    assert!(app.tabs[0].document.source().contains("// mine"));
    // The first X belongs to the first tab; it must prompt for that tab.
    click(&mut app, &ctx, X);
    assert_eq!(app.waiting_close, Some(app.tabs[0].id));
    click(&mut app, &ctx, "Keep editing");
    assert_eq!(app.tabs.len(), 2);
    assert!(app.waiting_close.is_none());
}

#[test]
fn background_compilation_never_publishes_into_another_tab() {
    let (mut app, ctx) = workspace();
    app.tabs[0]
        .document
        .replace_text("semantics {\n First\n}\n".into())
        .unwrap();
    app.tabs[0].changed = Instant::now() - Duration::from_secs(1);
    app.new_tab();
    app.tabs[1]
        .document
        .replace_text("semantics {\n Second\n}\n".into())
        .unwrap();
    app.tabs[1].changed = Instant::now() - Duration::from_secs(1);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app
        .tabs
        .iter()
        .any(|t| t.rendered != Some(t.document.revision()))
    {
        assert!(Instant::now() < deadline);
        frame(&mut app, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.tabs[0].preview.as_ref().unwrap().nodes[0].id, "First");
    assert_eq!(app.tabs[1].preview.as_ref().unwrap().nodes[0].id, "Second");
    assert_ne!(
        app.tabs[0].image.as_ref().unwrap().0,
        app.tabs[1].image.as_ref().unwrap().0
    );
}

#[test]
fn title_icons_are_centered_and_title_has_no_separator() {
    let (mut app, ctx) = workspace();
    for _ in 0..3 {
        let output = frame(&mut app, &ctx, vec![]);
        let mut icons = 0;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape
                && text.pos.y < 44.0
                && [PLUS, FOLDER_OPEN, GEAR_SIX].contains(&text.galley.job.text.as_str())
            {
                let center = text.pos.y + text.galley.size().y / 2.0;
                assert!(
                    (center - 22.0).abs() < 0.6,
                    "icon center at {center}, expected 22"
                );
                icons += 1;
            }
            if let egui::Shape::LineSegment { points, .. } = &shape.shape {
                assert!(
                    !(points[0].y == points[1].y && (points[0].y - 44.0).abs() < 1.0),
                    "no separator between title and tabs"
                );
            }
        }
        assert_eq!(icons, 3);
    }
}

#[test]
fn phosphor_icons_use_standard_hover_background() {
    let (_, ctx) = workspace();
    ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.0);
    let mut button = egui::Rect::NOTHING;
    for hovered in [false, true, false] {
        let pointer = if hovered {
            button.center()
        } else {
            egui::pos2(800.0, 600.0)
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events: vec![egui::Event::PointerMoved(pointer)],
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    button = icon(ui, GEAR_SIX, "Settings").rect;
                });
            },
        );
        output.textures_delta.clear();
        let hover = ctx
            .style_of(egui::Theme::Dark)
            .visuals
            .widgets
            .hovered
            .weak_bg_fill;
        let highlighted = output.shapes.iter().any(|shape| {
            matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect.contains(button.center()) && rect.fill == hover)
        });
        assert_eq!(highlighted, hovered);
    }
}

#[test]
fn tabs_reuse_the_injected_backend_including_after_the_last_tab_closes() {
    use super::super::backend_tests::{self, MemoryBackend};
    let (editor, ctx, backend) = backend_tests::ready();
    let mut workspace = Workspace::new(editor);
    let path = MemoryBackend::address(std::path::Path::new("remote.laml"));
    backend
        .sources
        .lock()
        .unwrap()
        .insert(path.clone(), SOURCE.into());
    open(&mut workspace, PathBuf::from("remote.laml"));
    assert_eq!(workspace.tabs.len(), 2);
    assert_eq!(workspace.tabs[1].file.path, Some(path.clone()));
    assert!(std::sync::Arc::ptr_eq(
        workspace.tabs[0].backend.native_handle(),
        workspace.tabs[1].backend.native_handle()
    ));
    open(&mut workspace, path);
    assert_eq!(workspace.tabs.len(), 2);
    workspace.new_tab();
    assert!(std::sync::Arc::ptr_eq(
        workspace.tabs[0].backend.native_handle(),
        workspace.tabs[2].backend.native_handle()
    ));
    workspace.tabs.clear();
    workspace.finish_close(&ctx);
    assert_eq!(workspace.tabs.len(), 1);
    assert!(std::sync::Arc::ptr_eq(
        workspace.backend.native_handle(),
        workspace.tabs[0].backend.native_handle()
    ));
}
