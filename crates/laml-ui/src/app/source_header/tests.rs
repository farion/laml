use super::*;

fn ready() -> (Editor, egui::Context) {
    let source = "semantics {\n A\n B\n}\n";
    let mut editor = Editor::new(source.into(), Some(PathBuf::from("header-test.laml")));
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let preview = laml_ui::preview::build(source, &editor.path).unwrap();
    editor.publish_preview(&ctx, preview);
    (editor, ctx)
}

fn frame(
    editor: &mut Editor,
    ctx: &egui::Context,
    width: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            // Document actions live in the global titlebar, not Source.
            if width >= 900.0 {
                ui.horizontal(|ui| {
                    editor.document_title_actions(ui);
                    editor.export_controls(ui);
                });
            }
            editor.source_heading(ui);
            editor.source_editor(ui);
            editor.export_windows(ui.ctx());
        },
    );
    output.textures_delta.clear();
    output
}

fn click(editor: &mut Editor, ctx: &egui::Context, width: f32, label: &str) {
    let mut output = frame(editor, ctx, width, vec![]);
    for _ in 0..2 {
        output = frame(editor, ctx, width, vec![]);
    }
    let point = output
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
        .unwrap_or_else(|| panic!("missing action {label}"));
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        editor,
        ctx,
        width,
        vec![egui::Event::PointerMoved(point), button(true)],
    );
    frame(editor, ctx, width, vec![button(false)]);
}

#[test]
fn header_is_single_and_responsive_without_clipping_or_old_actions() {
    let (mut editor, ctx) = ready();
    for width in [280.0, 324.0, 366.0, 574.0, 800.0, 1000.0] {
        for _ in 0..3 {
            let output = frame(&mut editor, &ctx, width, vec![]);
            let mut source_labels = 0;
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape {
                    let label = text.galley.job.text.as_str();
                    source_labels += usize::from(label == "Source");
                    assert_ne!(label, "header-test.laml");
                    assert!(
                        ![
                            "New",
                            "Open…",
                            "Path…",
                            "Save path…",
                            "Recovery…",
                            "Complete…",
                            "Toggle line comments"
                        ]
                        .contains(&label)
                    );
                    let source_top = if width >= 900.0 { 42.0 } else { 0.0 };
                    if text.pos.y >= source_top
                        && text.pos.y < source_top + appearance::HEADER_HEIGHT
                    {
                        assert!(text.pos.x >= 14.0);
                        assert!(
                            text.pos.x + text.galley.size().x <= width - 13.0,
                            "{label} exceeds header width {width}: {:?}",
                            text.pos
                        );
                    }
                }
                if let egui::Shape::Rect(rect) = &shape.shape
                    && rect.rect.width() < 150.0
                    && rect.rect.center().y >= if width >= 900.0 { 42.0 } else { 0.0 }
                    && rect.rect.center().y
                        < if width >= 900.0 { 42.0 } else { 0.0 } + appearance::HEADER_HEIGHT
                {
                    assert!(
                        rect.rect.right() <= width - 13.0,
                        "button exceeds width {width}: {:?}",
                        rect.rect
                    );
                    assert!(shape.clip_rect.contains_rect(rect.rect));
                }
            }
            assert_eq!(source_labels, 1);
        }
    }
}

#[test]
fn narrow_header_overflow_contains_hidden_actions_with_icon_and_text() {
    let (mut editor, ctx) = ready();
    click(&mut editor, &ctx, 140.0, egui_phosphor::regular::DOTS_THREE);
    let output = frame(&mut editor, &ctx, 140.0, vec![]);
    for label in ["Find / replace…", "Format source"] {
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if t.galley.job.text == label || t.galley.job.text.ends_with(&format!("  {label}"))
        )), "missing overflow entry {label}");
    }
    for label in ["Find / replace…", "Format source"] {
        let text = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text.ends_with(&format!("  {label}")) => {
                    Some(t)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            text.galley.job.sections[0].format.font_id.family,
            egui::FontFamily::Name("phosphor".into())
        );
        assert_eq!(
            text.galley.job.sections[1].format.font_id.family,
            egui::FontFamily::Proportional
        );
    }
    click(&mut editor, &ctx, 140.0, "Find / replace…");
    assert!(editor.source_ui.find_open);
}

#[test]
fn wide_header_shows_all_actions_and_history_controls_work() {
    let (mut editor, ctx) = ready();
    let original = editor.document.source().to_string();
    let edited = format!("{original}// edited\n");
    editor.document.replace_text(edited.clone()).unwrap();
    let output = frame(&mut editor, &ctx, 1000.0, vec![]);
    for label in [
        FLOPPY_DISK,
        ARROW_COUNTER_CLOCKWISE,
        ARROW_CLOCKWISE,
        egui_phosphor::regular::EXPORT,
        MAGNIFYING_GLASS,
        CODE,
    ] {
        assert!(
            output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if t.galley.job.text == label)),
            "missing {label}"
        );
    }
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Text(t) if t.galley.job.text == egui_phosphor::regular::DOTS_THREE)));
    click(&mut editor, &ctx, 1000.0, ARROW_COUNTER_CLOCKWISE);
    assert_eq!(editor.document.source(), original);
    click(&mut editor, &ctx, 1000.0, ARROW_CLOCKWISE);
    assert_eq!(editor.document.source(), edited);
    click(&mut editor, &ctx, 1000.0, CODE);
    assert!(editor.document.source().contains("    A"));
    click(&mut editor, &ctx, 1000.0, MAGNIFYING_GLASS);
    assert!(editor.source_ui.find_open);
}

#[test]
fn line_numbers_toggle_from_header_and_overflow_without_editing_source() {
    let (mut editor, ctx) = ready();
    let original = editor.document.source().to_owned();
    assert!(editor.source_ui.line_numbers);
    click(&mut editor, &ctx, 1000.0, LIST_NUMBERS);
    assert!(!editor.source_ui.line_numbers);
    click(&mut editor, &ctx, 1000.0, LIST_NUMBERS);
    assert!(editor.source_ui.line_numbers);
    click(&mut editor, &ctx, 140.0, egui_phosphor::regular::DOTS_THREE);
    click(&mut editor, &ctx, 140.0, "Hide line numbers");
    assert!(!editor.source_ui.line_numbers);
    click(&mut editor, &ctx, 140.0, egui_phosphor::regular::DOTS_THREE);
    click(&mut editor, &ctx, 140.0, "Show line numbers");
    assert!(editor.source_ui.line_numbers);
    assert_eq!(editor.document.source(), original);
    assert_eq!(editor.document.revision(), 0);
    assert_eq!(editor.document.history_usage().0, 0);
}

#[test]
fn save_icon_writes_only_the_current_document() {
    let (mut editor, ctx) = ready();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("current.laml");
    let original = editor.document.source().to_owned();
    std::fs::write(&path, &original).unwrap();
    editor.file = laml_ui::file_state::FileState::new(Some(path.clone()), &original);
    editor.path = path.clone();
    let changed = format!("{original}// saved with icon\n");
    editor.document.replace_text(changed.clone()).unwrap();
    click(&mut editor, &ctx, 1000.0, FLOPPY_DISK);
    super::super::tests::wait_files(&mut editor, &ctx);
    assert_eq!(std::fs::read_to_string(path).unwrap(), changed);
    assert!(!editor.document.is_dirty());
}

#[test]
fn global_export_action_opens_the_format_dialog() {
    let (mut editor, ctx) = ready();
    click(&mut editor, &ctx, 1000.0, egui_phosphor::regular::EXPORT);
    let output = frame(&mut editor, &ctx, 1000.0, vec![]);
    for format in [
        "SVG · Vector image",
        "PNG · Raster image",
        "PDF · Vector document",
    ] {
        assert!(
            output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if t.galley.job.text == format)),
            "missing {format}"
        );
    }
}
