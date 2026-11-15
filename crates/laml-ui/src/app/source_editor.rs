use super::*;
use egui::text::{CCursor, CCursorRange, LayoutJob};

impl Editor {
    pub(super) fn source_editor(&mut self, ui: &mut egui::Ui) {
        self.source_toolbar(ui);
        self.advanced_source(ui);
        let mut text = self.document.source().to_owned();
        let explicit_jump = self.source_ui.jump.take();
        let focus = explicit_jump.is_some();
        let jump = explicit_jump
            .or_else(|| {
                self.source_jump
                    .take()
                    .and_then(|line| laml_ui::source::SourceIndex::new(&text).line(line))
            })
            .filter(|r| {
                r.start <= r.end
                    && r.end <= text.len()
                    && text.is_char_boundary(r.start)
                    && text.is_char_boundary(r.end)
            });
        if let Some(range) = &jump
            && let Some(id) = self.source_ui.id
        {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            state.cursor.set_char_range(Some(char_range(&text, range)));
            state.store(ui.ctx(), id);
            if focus {
                ui.memory_mut(|m| m.request_focus(id));
            }
        }
        let diagnostic = self
            .source_ui
            .diagnostic
            .as_ref()
            .filter(|(r, _)| *r == self.document.revision())
            .and_then(|(_, d)| d.range(&text, &self.path));
        if let Some(range) = &diagnostic
            && ui.small_button("Go to reported line").clicked()
        {
            self.source_ui.jump = Some(range.clone());
        }
        let pair = laml_ui::syntax_tools::matching_pair(&text, self.source_ui.selection.end);
        let diagnosed_source = (diagnostic.is_some() || pair.is_some()).then(|| text.clone());
        let navigation = ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, egui::Event::PointerButton { .. })
                    || matches!(
                        e,
                        egui::Event::Key {
                            key: egui::Key::ArrowLeft
                                | egui::Key::ArrowRight
                                | egui::Key::ArrowUp
                                | egui::Key::ArrowDown
                                | egui::Key::Home
                                | egui::Key::End
                                | egui::Key::PageUp
                                | egui::Key::PageDown,
                            ..
                        }
                    )
            })
        });
        if navigation {
            self.document.break_typing_group();
        }
        let pasted = ui.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Paste(_) | egui::Event::Cut))
        });
        egui::ScrollArea::both().id_salt("source").show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let digits = (text.lines().count().max(1).ilog10() + 1) as f32;
                let (gutter, _) = ui.allocate_exact_size(
                    egui::vec2(digits * 9.0 + 16.0, 0.0),
                    egui::Sense::hover(),
                );
                let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _width: f32| {
                    ui.fonts_mut(|fonts| {
                        let mut job = highlight(buffer.as_str(), ui.visuals().text_color());
                        if diagnosed_source.as_deref() == Some(buffer.as_str())
                            && let Some(range) = &diagnostic
                        {
                            mark(&mut job, range, true);
                        }
                        if diagnosed_source.as_deref() == Some(buffer.as_str())
                            && let Some((a, b)) = &pair
                        {
                            mark(&mut job, a, false);
                            mark(&mut job, b, false);
                        }
                        fonts.layout_job(job)
                    })
                };
                let mut output = egui::TextEdit::multiline(&mut text)
                    .id_salt("source-text")
                    .code_editor()
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .show(ui);
                self.source_ui.id = Some(output.response.id);
                #[cfg(test)]
                {
                    self.source_ui.rect = Some(output.response.rect);
                }
                if output.response.changed() {
                    let result = if pasted {
                        self.document.replace_text(text.clone())
                    } else {
                        self.document.replace_typed(
                            text.clone(),
                            Duration::try_from_secs_f64(ui.input(|i| i.time)).unwrap_or_default(),
                        )
                    };
                    match result {
                        Ok(true) => self.source_mutated(),
                        Err(error) => self.error = Some(format!("Edit failed: {error:?}")),
                        _ => {}
                    }
                }
                if !output.response.changed()
                    && let Some(range) = &jump
                {
                    let range = char_range(&text, range);
                    output.state.cursor.set_char_range(Some(range));
                    if focus && !output.response.has_focus() {
                        output.response.request_focus();
                    }
                    ui.scroll_to_rect(
                        output
                            .galley
                            .pos_from_cursor(range.primary)
                            .translate(output.galley_pos.to_vec2()),
                        Some(egui::Align::Center),
                    );
                }
                let cursor = output.state.cursor.char_range();
                if let Some(cursor) = cursor {
                    let sorted = cursor.as_sorted_char_range();
                    let selection = byte_offset(&text, sorted.start.into())
                        ..byte_offset(&text, sorted.end.into());
                    if !output.response.changed() && selection != self.source_ui.selection {
                        self.document.break_typing_group();
                    }
                    self.source_ui.selection = selection;
                    self.source_ui.selection_revision = self.document.revision();
                    if jump.is_none()
                        && output.response.has_focus()
                        && self.rendered == Some(self.document.revision())
                    {
                        let line = laml_ui::source::SourceIndex::new(&text)
                            .line_at(byte_offset(&text, cursor.primary.index.into()))
                            .unwrap_or(1);
                        self.select_source_line(line);
                    }
                }
                if !output.response.has_focus() {
                    self.document.break_typing_group();
                    self.source_ui.composing = false;
                }
                if ui.memory(|m| m.has_focus(output.response.id)) {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            output.response.id,
                            egui::EventFilter {
                                tab: true,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: false,
                            },
                        )
                    });
                }
                // Only Document owns source undo. Do not retain egui's duplicate
                // full-buffer undo snapshots alongside the bounded delta history.
                output.state.clear_undoer();
                output.state.store(ui.ctx(), output.response.id);
                let index = laml_ui::source::SourceIndex::new(&text);
                let mut offsets = vec![0];
                offsets.extend(
                    text.chars()
                        .enumerate()
                        .filter_map(|(i, c)| (c == '\n').then_some(i + 1)),
                );
                for (line, offset) in offsets.into_iter().enumerate() {
                    let position = output.galley.pos_from_cursor(CCursor::new(offset));
                    let point = egui::pos2(gutter.right(), output.galley_pos.y + position.top());
                    if point.y + position.height() < ui.clip_rect().top()
                        || point.y > ui.clip_rect().bottom()
                    {
                        continue;
                    }
                    let marked = !output.response.changed()
                        && diagnostic.as_ref().is_some_and(|d| {
                            index.line(line + 1).is_some_and(|r| r.start == d.start)
                        });
                    ui.painter().text(
                        point,
                        egui::Align2::RIGHT_TOP,
                        (line + 1).to_string(),
                        egui::FontId::monospace(14.0),
                        if marked {
                            egui::Color32::LIGHT_RED
                        } else {
                            ui.visuals().weak_text_color()
                        },
                    );
                }
            });
        });
    }
    fn select_source_line(&mut self, line: usize) {
        if self.specialized() {
            let id = self.preview.as_ref().and_then(|p| p.regions.iter().find(|r| r.line == line)).map(|r| r.id.clone());
            if laml_ui::notation_edit::record(self.document.source(), line).is_ok() {
                self.notation_ui.selected = Some((line, id, self.document.revision()));
                self.property_drafts.clear();
            }
            return;
        }
        self.notation_ui.selected = None;
        let Some(preview) = &self.preview else {
            return;
        };
        if let Some(selection) = laml_ui::source_link::targets_in_diagram(&preview.diagram, line) {
            self.selection = selection;
            self.property_drafts.clear();
            return;
        }
        let selected = if let Some(node) = preview.diagram.nodes.iter().find(|n| n.line == line) {
            if self.selection.ids == [node.id.clone()] {
                false
            } else {
                self.selection.click(Some(&node.id), false);
                true
            }
        } else if let Some(relation) = preview.diagram.relations.iter().find(|r| r.line == line) {
            if self.selection.relations == [line] {
                false
            } else {
                self.selection.click_relation(relation.line, false);
                true
            }
        } else if let Some(id) = preview.diagram.nodes.iter().find_map(|n| {
            n.ports
                .iter()
                .find(|p| p.line == line)
                .map(|p| laml_ui::properties::port_id(&n.id, p))
        }) {
            if self.selection.ports == [id.clone()] {
                false
            } else {
                self.selection.click_port(&id, false);
                true
            }
        } else {
            false
        };
        if selected {
            self.property_drafts.clear();
        }
    }
}
fn byte_offset(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(i, _)| i)
}
fn char_range(text: &str, range: &std::ops::Range<usize>) -> CCursorRange {
    CCursorRange::two(
        CCursor::new(text[..range.start].chars().count()),
        CCursor::new(text[..range.end].chars().count()),
    )
}
fn mark(job: &mut LayoutJob, range: &std::ops::Range<usize>, diagnostic: bool) {
    job.sections = std::mem::take(&mut job.sections)
        .into_iter()
        .flat_map(|section| {
            let a = usize::from(section.byte_range.start);
            let b = usize::from(section.byte_range.end);
            let mut cuts = vec![a, b];
            for edge in [range.start, range.end] {
                if edge > a && edge < b {
                    cuts.push(edge);
                }
            }
            cuts.sort_unstable();
            cuts.windows(2)
                .map(|part| {
                    let mut section = section.clone();
                    section.byte_range = part[0].into()..part[1].into();
                    if part[0] != a {
                        section.leading_space = 0.0;
                    }
                    if part[0] >= range.start && part[1] <= range.end {
                        if diagnostic {
                            section.format.underline =
                                egui::Stroke::new(1.5, egui::Color32::LIGHT_RED);
                        }
                        section.format.background = if diagnostic {
                            egui::Color32::from_rgba_unmultiplied(180, 20, 20, 30)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(20, 150, 180, 80)
                        };
                    }
                    section
                })
                .collect::<Vec<_>>()
        })
        .collect();
}

fn highlight(text: &str, normal: egui::Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut i = 0;
    while i < text.len() {
        let start = i;
        let ch = text[i..].chars().next().unwrap();
        let color = if text[i..].starts_with("//")
            && (i == 0
                || text[..i]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace))
        {
            i += text[i..].find('\n').unwrap_or(text.len() - i);
            egui::Color32::from_rgb(130, 150, 130)
        } else if ch == '"' {
            i += 1;
            let mut escaped = false;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                i += c.len_utf8();
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                }
            }
            egui::Color32::from_rgb(210, 180, 110)
        } else if ch.is_alphabetic() || ch == '_' {
            i += ch.len_utf8();
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                if !(c.is_alphanumeric() || c == '_' || c == '-') {
                    break;
                }
                i += c.len_utf8();
            }
            match &text[start..i] {
                "semantics" | "settings" | "layout" | "styling" | "import" | "above" | "below"
                | "node" => egui::Color32::from_rgb(115, 190, 230),
                _ => normal,
            }
        } else {
            i += ch.len_utf8();
            if "{}()[]:;<>-=?".contains(ch) {
                egui::Color32::from_rgb(180, 150, 220)
            } else {
                normal
            }
        };
        let color = if normal.r() < 128 {
            match color {
                _ if color == egui::Color32::from_rgb(130, 150, 130) => egui::Color32::from_rgb(70, 110, 70),
                _ if color == egui::Color32::from_rgb(210, 180, 110) => egui::Color32::from_rgb(140, 90, 20),
                _ if color == egui::Color32::from_rgb(115, 190, 230) => egui::Color32::from_rgb(25, 95, 155),
                _ if color == egui::Color32::from_rgb(180, 150, 220) => egui::Color32::from_rgb(120, 65, 165),
                _ => color,
            }
        } else { color };
        job.append(
            &text[start..i],
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color,
                ..Default::default()
            },
        );
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn highlighting_preserves_unicode_comments_and_incomplete_strings() {
        for source in [
            "A(label=\"Grün // ok\") // comment\r\n",
            "semantics {\n A(label=\"unfinished",
            "// comment",
            "",
        ] {
            assert_eq!(highlight(source, egui::Color32::WHITE).text, source);
        }
    }
}
