//! Inline completion surface and delimiter input; no renderer/backend requests.
use super::*;
mod popup;
#[cfg(test)]
mod tests;
impl Editor {
    pub(super) fn source_pair_input(&mut self, ui: &mut egui::Ui) {
        if self.source_ui.selection_revision != self.document.revision() {
            return;
        }
        let selection = self.source_ui.selection.clone();
        let source = self.document.source();
        let action = ui.input(|i| {
            if i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Paste(_)
                        | egui::Event::Ime(_)
                        | egui::Event::Cut
                        | egui::Event::PointerButton { .. }
                )
            }) {
                return None;
            }
            let text_events: Vec<_> = i
                .events
                .iter()
                .enumerate()
                .filter_map(|(n, e)| {
                    if let egui::Event::Text(t) = e {
                        Some((n, t.as_str()))
                    } else {
                        None
                    }
                })
                .collect();
            // Let TextEdit process navigation/editing-key batches in their original
            // order rather than applying a delimiter against yesterday's caret.
            let editing_key = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        pressed: true,
                        key: egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::Home
                            | egui::Key::End
                            | egui::Key::Enter
                            | egui::Key::Delete
                            | egui::Key::Backspace,
                        ..
                    }
                )
            });
            if !text_events.is_empty() {
                if editing_key || i.modifiers.command || i.modifiers.ctrl && !i.modifiers.alt {
                    return None;
                }
                let mut candidate = source.to_owned();
                let mut selected = selection.clone();
                let mut assisted = false;
                for (_, text) in &text_events {
                    for c in text.chars() {
                        if let Some(change) =
                            laml_ui::source_assist::pairs::typed(&candidate, selected.clone(), c)
                        {
                            for edit in change.edits.iter().rev() {
                                candidate.replace_range(edit.range.clone(), &edit.replacement);
                            }
                            selected = change.selection;
                            assisted = true;
                        } else {
                            candidate.replace_range(selected.clone(), &c.to_string());
                            let at = selected.start + c.len_utf8();
                            selected = at..at;
                        }
                    }
                }
                return assisted.then(|| {
                    (
                        text_events.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
                        laml_ui::text_tools::Change {
                            edits: laml_ui::graph_edit::difference(source, &candidate),
                            selection: selected,
                        },
                    )
                });
            }
            i.events.iter().enumerate().find_map(|(index, event)| {
                if let egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                    && modifiers.is_none()
                {
                    laml_ui::source_assist::pairs::backspace(source, selection.clone())
                        .map(|change| (vec![index], change))
                } else {
                    None
                }
            })
        });
        if let Some((indices, change)) = action {
            ui.input_mut(|i| {
                for index in indices.into_iter().rev() {
                    i.events.remove(index);
                }
            });
            if change.edits.is_empty() {
                self.source_ui.jump = Some(change.selection);
                self.advanced.completion = None;
            } else {
                let before = self.document.source().to_owned();
                let session = self.advanced.snippet.take();
                self.apply_source_edits("Insert delimiter", change.edits, Some(change.selection));
                self.update_source_snippet(session, &before);
                self.advanced.refresh = true;
            }
        }
    }
    pub(super) fn update_source_snippet(
        &mut self,
        session: Option<laml_ui::source_assist::snippet::Session>,
        before: &str,
    ) {
        self.advanced.snippet = session.and_then(|mut s| {
            s.update(before, self.document.source(), self.document.revision())
                .then_some(s)
        });
    }
}
