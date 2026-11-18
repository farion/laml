use super::*;
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn frame(
    e: &mut Editor,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Vec<egui::epaint::ClippedShape> {
    let modifiers = events
        .iter()
        .find_map(|e| {
            if let egui::Event::Key { modifiers, .. } = e {
                Some(*modifiers)
            } else {
                None
            }
        })
        .unwrap_or_default();
    let mut events = events;
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut output = ctx.run_ui(
        egui::RawInput {
            events,
            time: Some(ctx.input(|i| i.time) + 0.1),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        },
        |ui| {
            e.history_shortcuts(ui.ctx());
            e.source_editor(ui);
        },
    );
    output.textures_delta.clear();
    output.shapes
}
fn ready(s: &str) -> (Editor, egui::Context) {
    let mut e = Editor::new(s.into(), None);
    let ctx = egui::Context::default();
    e.source_ui.jump = Some(s.len()..s.len());
    frame(&mut e, &ctx, vec![]);
    (e, ctx)
}
#[test]
fn typing_opens_inline_suggestions_and_tab_accepts_without_arrow_selection() {
    let (mut e, ctx) = ready("settings {\n    th");
    frame(&mut e, &ctx, vec![egui::Event::Text("e".into())]);
    assert!(e.advanced.completion.is_some());
    assert_eq!(e.advanced.selected, None);
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n    theme: ");
    assert!(
        e.advanced
            .completion
            .as_ref()
            .unwrap()
            .1
            .entries
            .iter()
            .any(|v| v.label == "modern")
    );
}
#[test]
fn unselected_enter_is_an_indented_newline_not_an_acceptance() {
    let (mut e, ctx) = ready("settings {\n    th");
    frame(&mut e, &ctx, vec![egui::Event::Text("e".into())]);
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n    the\n    ");
    assert!(e.advanced.completion.is_none());
}
#[test]
fn arrow_selection_allows_enter_and_escape_leaves_source_unchanged() {
    let s = "settings {\n direction: lef";
    let (mut e, ctx) = ready(s);
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Space, egui::Modifiers::CTRL)],
    );
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
    );
    assert_eq!(e.advanced.selected, Some(0));
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n direction: left-to-right");
    e.open_completion();
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(e.advanced.completion.is_none());
}
#[test]
fn delimiters_are_paired_skipped_removed_and_one_undo_step() {
    let (mut e, ctx) = ready("semantics {\n A");
    frame(&mut e, &ctx, vec![egui::Event::Text("(".into())]);
    assert_eq!(e.document.source(), "semantics {\n A()");
    assert!(e.advanced.completion.is_some());
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Backspace, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "semantics {\n A");
    assert!(e.document.undo());
    assert_eq!(e.document.source(), "semantics {\n A()");
    e.source_ui.jump = Some(e.document.source().len() - 1..e.document.source().len() - 1);
    frame(&mut e, &ctx, vec![]);
    let revision = e.document.revision();
    frame(&mut e, &ctx, vec![egui::Event::Text(")".into())]);
    assert_eq!(e.document.revision(), revision);
    assert_eq!(
        e.source_ui.selection,
        e.document.source().len()..e.document.source().len()
    );
}
#[test]
fn snippet_fields_support_tab_shift_tab_escape_unicode_and_shared_undo() {
    let (mut e, ctx) = ready("sett");
    e.open_completion();
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n    \n}");
    assert!(e.advanced.snippet.is_some());
    frame(
        &mut e,
        &ctx,
        vec![egui::Event::Text("theme: modern".into())],
    );
    assert!(e.advanced.snippet.is_some());
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::SHIFT)],
    );
    assert_eq!(
        &e.document.source()[e.source_ui.selection.clone()],
        "theme: modern"
    );
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(e.advanced.snippet.is_none());
}
#[test]
fn blank_lines_and_paste_do_not_open_automatic_completion() {
    let (mut e, ctx) = ready("semantics {\n ");
    assert!(e.advanced.completion.is_none());
    frame(&mut e, &ctx, vec![egui::Event::Paste("class Hello".into())]);
    assert!(e.advanced.completion.is_none());
}
#[test]
fn mouse_can_accept_a_suggestion_without_clicking_through_to_source() {
    let (mut e, ctx) = ready("settings {\n th");
    frame(&mut e, &ctx, vec![egui::Event::Text("e".into())]);
    frame(&mut e, &ctx, vec![]);
    let pos = e.advanced.first_row.expect("visible suggestion").center();
    frame(
        &mut e,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    frame(
        &mut e,
        &ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(e.document.source(), "settings {\n theme: ");
    frame(&mut e, &ctx, vec![]);
    assert!(ctx.memory(|m| m.has_focus(e.source_ui.id.unwrap())));
}
#[test]
fn same_frame_text_batches_pair_once_and_keep_the_final_caret() {
    let (mut e, ctx) = ready("semantics {\n A");
    frame(
        &mut e,
        &ctx,
        vec![
            egui::Event::Text("(".into()),
            egui::Event::Text("label=".into()),
            egui::Event::Text("\"".into()),
        ],
    );
    assert_eq!(e.document.source(), "semantics {\n A(label=\"\")");
    assert_eq!(&e.document.source()[e.source_ui.selection.end..], "\")");
    assert!(e.document.undo());
    assert_eq!(e.document.source(), "semantics {\n A");
}
#[test]
fn enter_between_braces_indents_and_last_placeholder_tab_exits_at_the_end() {
    let (mut e, ctx) = ready("settings ");
    frame(&mut e, &ctx, vec![egui::Event::Text("{".into())]);
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n    \n}");
    let (mut e, ctx) = ready("sett");
    e.open_completion();
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert!(e.advanced.snippet.is_none());
    assert_eq!(
        e.source_ui.selection,
        e.document.source().len()..e.document.source().len()
    );
}
#[test]
fn text_rows_sit_under_source_and_arrows_paint_a_visible_selection() {
    let (mut e, ctx) = ready("settings {\n    theme: ");
    e.open_completion();
    frame(&mut e, &ctx, vec![]);
    let shapes = frame(&mut e, &ctx, vec![]);
    let row = e.advanced.first_row.unwrap();
    let anchor = e.advanced.anchor.unwrap();
    assert!(row.top() >= anchor.bottom() && row.top() - anchor.bottom() <= 3.0);
    assert!(
        row.height() <= 22.0,
        "code-sized rows, not padded buttons: {row:?}"
    );
    let text = shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "modern" => Some(t),
            _ => None,
        })
        .expect("plain text suggestion");
    assert_eq!(
        text.galley.job.sections[0].format.font_id,
        egui::FontId::monospace(14.0)
    );
    let caret = e.source_ui.selection.clone();
    let shapes = frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
    );
    assert_eq!(e.advanced.selected, Some(0));
    assert_eq!(e.source_ui.selection, caret);
    assert!(ctx.memory(|m| m.has_focus(e.source_ui.id.unwrap())));
    let fill = ctx.global_style().visuals.selection.bg_fill;
    assert!(
        shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect == row && r.fill == fill)),
        "arrow selection must paint its row background"
    );
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
    );
    assert_eq!(e.advanced.selected, Some(1));
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::ArrowUp, egui::Modifiers::NONE)],
    );
    assert_eq!(e.advanced.selected, Some(0));
    frame(
        &mut e,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(e.document.source(), "settings {\n    theme: modern");
}
