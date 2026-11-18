use super::*;
fn frame(editor: &mut Editor, ctx: &egui::Context, mut events: Vec<egui::Event>) {
    let time = ctx.input(|i| i.time) + 0.1;
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
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut output = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |ui| {
            editor.history_shortcuts(ui.ctx());
            editor.source_editor(ui);
        },
    );
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
}
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn ready(source: &str, selection: std::ops::Range<usize>) -> (Editor, egui::Context) {
    let mut editor = Editor::new(source.into(), None);
    let ctx = egui::Context::default();
    editor.source_ui.jump = Some(selection);
    frame(&mut editor, &ctx, vec![]);
    (editor, ctx)
}
#[test]
fn short_source_editor_fills_available_height_including_horizontal_overflow() {
    for source in [
        String::new(),
        "semantics {}".into(),
        format!("// {}", "wide ".repeat(200)),
    ] {
        let mut editor = Editor::new(source.clone(), None);
        let ctx = egui::Context::default();
        for height in [400.0, 900.0] {
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(600.0, height),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let bottom = ui.max_rect().bottom();
                        editor.source_editor(ui);
                        let rect = editor.source_ui.rect.unwrap();
                        assert!(
                            rect.bottom() >= bottom - 20.0,
                            "editor {rect:?} does not fill height to {bottom}"
                        );
                        assert!(
                            ui.min_rect().bottom() <= bottom + 1.0,
                            "source pane must not overflow its available height"
                        );
                    },
                );
                output.textures_delta.clear();
            }
        }
        assert_eq!(editor.document.source(), source);
        assert_eq!(editor.document.revision(), 0);
    }
}

#[test]
fn source_clipboard_copy_cut_paste_preserves_unicode_and_shared_undo() {
    let source = "// Grüße 日本語\n";
    let (mut editor, ctx) = ready(source, 3..source.len() - 1);
    let revision = editor.document.revision();
    let mut copied = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Copy],
            ..Default::default()
        },
        |ui| editor.source_editor(ui),
    );
    for command in &output.platform_output.commands {
        if let egui::OutputCommand::CopyText(text) = command {
            copied = Some(text.clone());
        }
    }
    output.textures_delta.clear();
    assert_eq!(copied.as_deref(), Some("Grüße 日本語"));
    assert_eq!(editor.document.revision(), revision);
    frame(&mut editor, &ctx, vec![egui::Event::Cut]);
    assert_eq!(editor.document.source(), "// \n");
    frame(&mut editor, &ctx, vec![egui::Event::Paste(copied.unwrap())]);
    assert_eq!(editor.document.source(), source);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), "// \n");
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), source);
}

#[test]
fn real_source_typing_has_shared_grouped_undo_without_native_snapshot_history() {
    let (mut editor, ctx) = ready("// ", 3..3);
    frame(&mut editor, &ctx, vec![egui::Event::Text("Gr".into())]);
    frame(&mut editor, &ctx, vec![egui::Event::Text("ün".into())]);
    assert_eq!(editor.document.source(), "// Grün");
    assert_eq!(editor.document.history_usage().0, 1);
    let state = egui::text_edit::TextEditState::load(&ctx, editor.source_ui.id.unwrap()).unwrap();
    assert!(!state.undoer().has_undo(&(
        state.cursor.char_range().unwrap(),
        editor.document.source().into()
    )));
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), "// ");
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Y, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), "// Grün");
}
#[test]
fn enter_followed_by_text_in_one_frame_uses_the_new_caret_and_preserves_crlf() {
    let source = "semantics {}\r\n";
    let (mut editor, ctx) = ready(source, 11..11);
    frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::Enter, egui::Modifiers::NONE),
            egui::Event::Text("A".into()),
        ],
    );
    assert_eq!(editor.document.source(), "semantics {\r\n    A\r\n}\r\n");
    assert_eq!(editor.document.history_usage().0, 2);
    editor.document.undo();
    assert_eq!(editor.document.source(), "semantics {\r\n    \r\n}\r\n");
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
}
#[test]
fn actual_tab_outdent_and_comment_shortcuts_preserve_selected_lines() {
    let source = "ä\r\nB\r\n";
    let (mut editor, ctx) = ready(source, 0..source.len());
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert_eq!(editor.document.source(), "    ä\r\n    B\r\n");
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::SHIFT)],
    );
    assert_eq!(editor.document.source(), source);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Slash, egui::Modifiers::COMMAND)],
    );
    assert_eq!(
        editor.document.source(),
        "// ä\r\n// B\r\n",
        "selection {:?}, focused {:?}, error {:?}",
        editor.source_ui.selection,
        ctx.memory(|m| m.focused()),
        editor.command_error
    );
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Slash, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), source);
}
#[test]
fn find_wraps_in_both_directions_and_replace_all_is_one_shared_undo() {
    let source = "ä Ä ä";
    let (mut editor, ctx) = ready(source, 0..0);
    editor.source_ui.find = "ä".into();
    editor.source_ui.case_sensitive = false;
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::F3, egui::Modifiers::NONE)],
    );
    assert_eq!(editor.source_ui.selection, 0..2);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::F3, egui::Modifiers::NONE)],
    );
    assert_eq!(editor.source_ui.selection, 3..5);
    editor.source_ui.replace = "Blue".into();
    editor.replace_matches(false);
    frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.document.source(), "ä Blue ä");
    editor.document.undo();
    editor.source_mutated();
    editor.replace_matches(true);
    frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.document.source(), "Blue Blue Blue");
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
    editor.source_ui.selection = 0..0;
    editor.find_next(true);
    assert_eq!(editor.source_ui.jump, Some(6..8));
}
#[test]
fn stale_replacement_selection_is_not_reused_against_different_text() {
    let source = "A B";
    let (mut editor, ctx) = ready(source, 0..1);
    editor.source_ui.find = "A".into();
    editor.source_ui.replace = "C".into();
    editor.document.replace_text("Z B".into()).unwrap();
    editor.replace_matches(false);
    frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.document.source(), "Z B");
}
#[test]
fn diagnostic_ranges_are_revision_bound_and_safe_during_unicode_input() {
    let (mut editor, ctx) = ready("A\nBroken", 0..0);
    editor.source_ui.diagnostic = Some((
        0,
        laml_ui::diagnostic::Diagnostic::at(&editor.path, 2, "bad"),
    ));
    frame(&mut editor, &ctx, vec![]);
    // Old diagnostic byte 2 becomes the middle of this new UTF-8 character.
    frame(&mut editor, &ctx, vec![egui::Event::Text("中".into())]);
    assert_eq!(editor.document.source(), "中A\nBroken");
    assert!(editor.source_ui.diagnostic.is_none());
}
#[test]
fn source_replace_cancels_inflight_visual_transaction_and_compiles_new_text() {
    let source = "semantics {\n A\n}\n";
    let (mut editor, ctx) = ready(source, 0..0);
    editor.changed = Instant::now() - Duration::from_secs(1);
    super::tests::wait(&mut editor, &ctx, |e| e.rendered == Some(0));
    editor.selection.click(Some("A"), false);
    editor.request_style("fill", Some("#123456".into()));
    editor.source_ui.find = "A".into();
    editor.source_ui.replace = "B".into();
    editor.replace_matches(true);
    editor.changed = Instant::now() - Duration::from_secs(1);
    super::tests::wait(&mut editor, &ctx, |e| e.rendered == Some(1));
    assert_eq!(editor.document.source(), source.replace('A', "B"));
    assert!(editor.pending_command.is_none());
    assert_eq!(editor.preview.as_ref().unwrap().nodes[0].id, "B");
}

#[test]
fn find_field_undo_does_not_undo_the_document() {
    let (mut editor, ctx) = ready("A", 1..1);
    frame(&mut editor, &ctx, vec![egui::Event::Text("B".into())]);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::F, egui::Modifiers::COMMAND)],
    );
    frame(&mut editor, &ctx, vec![egui::Event::Text("query".into())]);
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.document.source(), "AB");
}
#[test]
fn ime_confirmation_enter_is_not_a_source_newline_command() {
    let (mut editor, ctx) = ready("", 0..0);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "中".into(),
            active_range_chars: None,
        })],
    );
    assert!(editor.source_ui.composing);
    frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::Enter, egui::Modifiers::NONE),
            egui::Event::Ime(egui::ImeEvent::Commit("中文".into())),
        ],
    );
    assert_eq!(editor.document.source(), "中文");
    assert!(!editor.source_ui.composing);
}

#[test]
fn commands_reject_a_selection_from_a_different_source_revision() {
    let (mut editor, ctx) = ready("A", 0..1);
    editor.document.replace_text("A B".into()).unwrap();
    editor.source_action(laml_ui::text_tools::Action::Comment);
    assert_eq!(editor.document.source(), "A B");
    assert!(editor.command_error.is_some());
    frame(&mut editor, &ctx, vec![]);
    editor.source_action(laml_ui::text_tools::Action::Comment);
    assert_eq!(editor.document.source(), "// A B");
}

#[test]
fn completion_acceptance_is_atomic_and_stale_candidates_are_rejected() {
    let source = "settings {\n direction: lef";
    let (mut editor, ctx) = ready(source, source.len()..source.len());
    editor.open_completion();
    assert!(editor.advanced.completion.is_some());
    frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert_eq!(
        editor.document.source(),
        "settings {\n direction: left-to-right"
    );
    editor.document.undo();
    assert_eq!(editor.document.source(), source);
    frame(&mut editor, &ctx, vec![]);
    editor.open_completion();
    editor.document.replace_text("newer".into()).unwrap();
    editor.accept_completion(0);
    assert_eq!(editor.document.source(), "newer");
}
#[test]
fn format_source_shortcut_is_not_mistaken_for_find() {
    let source = "semantics {\nA\n}\n";
    let (mut editor, ctx) = ready(source, 0..0);
    frame(
        &mut editor,
        &ctx,
        vec![key(
            egui::Key::F,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        )],
    );
    assert_eq!(editor.document.source(), "semantics {\n    A\n}\n");
    assert!(!editor.source_ui.find_open);
}

#[test]
fn layout_source_line_selects_both_nodes_not_an_unrelated_declaration() {
    let source = "semantics {\n A\n B\n}\nlayout {\n A < B\n}\n";
    let (mut e, ctx) = ready(source, 0..0);
    e.changed = Instant::now() - Duration::from_secs(1);
    super::tests::wait(&mut e, &ctx, |e| e.rendered == Some(0));
    e.source_ui.jump = Some(source.find("A <").unwrap()..source.find("A <").unwrap());
    frame(&mut e, &ctx, vec![]);
    frame(&mut e, &ctx, vec![]);
    assert_eq!(e.selection.ids, ["A", "B"]);
}
