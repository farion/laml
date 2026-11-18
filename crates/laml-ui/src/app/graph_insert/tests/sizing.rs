use super::*;

fn show(
    editor: &mut Editor,
    ctx: &egui::Context,
    height: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, height),
            )),
            events,
            ..Default::default()
        },
        |ui| editor.graph_insert_menu(ui),
    );
    out.textures_delta.clear();
    out
}
fn text<'a>(
    out: &'a egui::FullOutput,
    label: &str,
) -> Option<(&'a egui::epaint::TextShape, egui::Rect)> {
    out.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t)
            if t.galley.text() == label || t.galley.text().ends_with(&format!("  {label}")) =>
        {
            Some((t, s.clip_rect))
        }
        _ => None,
    })
}
fn menu() -> (Editor, egui::Context) {
    let source = "semantics {\n A\n}\n";
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    appearance::install(&ctx);
    egui_extras::install_image_loaders(&ctx);
    let choices = graph_insert::items(source);
    editor.graph.insertion = Some(Menu {
        revision: 0,
        from: "A".into(),
        side: Placement::Right,
        at: egui::pos2(200.0, 500.0),
        expanded: false,
        inside: false,
        choices,
    });
    (editor, ctx)
}
#[test]
fn more_expands_to_natural_height_instead_of_the_old_420_point_limit() {
    let (mut editor, ctx) = menu();
    let mut out = show(&mut editor, &ctx, 1300.0, vec![]);
    for _ in 0..5 {
        out = show(&mut editor, &ctx, 1300.0, vec![]);
    }
    let (more, _) = text(&out, "More…").expect("compact menu must offer More");
    let pos = more.pos + more.galley.size() / 2.0;
    for pressed in [true, false] {
        out = show(
            &mut editor,
            &ctx,
            1300.0,
            vec![egui::Event::PointerMoved(pos), button(pos, pressed)],
        );
    }
    assert!(editor.graph.insertion.as_ref().unwrap().expanded);
    for _ in 0..6 {
        out = show(&mut editor, &ctx, 1300.0, vec![]);
    }
    let choices = &editor.graph.insertion.as_ref().unwrap().choices;
    let first = text(&out, choices[0].title()).unwrap().0.pos.y;
    let mut bottom = first;
    for item in choices {
        let (label, clip) = text(&out, item.title())
            .unwrap_or_else(|| panic!("{} must not require scrolling", item.title()));
        let bounds = egui::Rect::from_min_size(label.pos, label.galley.size());
        assert!(
            clip.expand(0.5).contains_rect(bounds),
            "{} clipped despite available screen space: {bounds:?} / {clip:?}",
            item.title()
        );
        bottom = bottom.max(bounds.bottom());
    }
    assert!(
        bottom - first > 420.0,
        "fixture must exceed the old menu cap, got {} entries over {} points",
        choices.len(),
        bottom - first
    );
    assert_eq!(editor.document.revision(), 0);
}
#[test]
fn expanded_menu_scrolls_only_when_the_screen_is_too_short() {
    let (mut editor, ctx) = menu();
    let menu = editor.graph.insertion.as_mut().unwrap();
    menu.expanded = true;
    menu.at.y = 160.0;
    let first = menu.choices[0].title();
    let last = menu.choices.last().unwrap().title();
    let mut out = show(&mut editor, &ctx, 300.0, vec![]);
    for _ in 0..6 {
        out = show(&mut editor, &ctx, 300.0, vec![]);
    }
    let (top, clip) = text(&out, first).unwrap();
    assert!(clip.height() < 300.0);
    let pos = egui::pos2(top.pos.x + 40.0, clip.center().y);
    show(
        &mut editor,
        &ctx,
        300.0,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -2000.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..40 {
        out = show(&mut editor, &ctx, 300.0, vec![]);
    }
    let (label, clip) = text(&out, last).expect("last item must be reachable by scrolling");
    let bounds = egui::Rect::from_min_size(label.pos, label.galley.size());
    assert!(clip.expand(0.5).contains_rect(bounds));
    assert!(bounds.top() >= 0.0 && bounds.bottom() <= 300.0);
    assert_eq!(editor.document.revision(), 0);
}
