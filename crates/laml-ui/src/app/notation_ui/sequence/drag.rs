//! Vertical event drags and execution-edge resizing use compiler source boundaries.
use super::*;
use laml::authoring::Region;

#[derive(Clone)]
pub(super) struct Drag {
    revision: u64,
    origin: egui::Pos2,
    started: bool,
    line: usize,
    selection_line: usize,
    id: String,
    close_open: bool,
    owner: Option<String>,
    last_alignment: Option<(usize, String)>,
    shape: egui::Rect,
    lower: Option<bool>,
    last_before: Option<usize>,
    candidate: Option<Result<(Vec<laml_ui::document::Edit>, usize), String>>,
}
fn screen(rect: egui::Rect, bounds: (f64, f64, f64, f64), r: &Region) -> egui::Rect {
    egui::Rect::from_min_max(
        canvas::point(rect, bounds, r.x, r.y),
        canvas::point(rect, bounds, r.x + r.width, r.y + r.height),
    )
}
impl Editor {
    pub(super) fn sequence_edge_drag(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        if self.notation_ui.sequence.event_drag.is_some() {
            return self.sequence_drag_active(ui, rect);
        }
        if self.notation_ui.sequence.interacting() {
            return false;
        }
        let Some(preview) = &self.preview else {
            return false;
        };
        let bounds = preview.bounds;
        let bars: Vec<_> = preview
            .regions
            .iter()
            .filter(|r| r.kind == "execution")
            .cloned()
            .collect();
        let message_ends: Vec<_> = preview
            .regions
            .iter()
            .filter(|r| matches!(r.kind.as_str(), "sequence-send" | "sequence-receive"))
            .map(|r| canvas::point(rect, bounds, r.x, r.y))
            .collect();
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let mut owned = false;
        for bar in bars {
            let shape = screen(rect, bounds, &bar);
            if !ui.clip_rect().intersects(shape) {
                continue;
            }
            let selected = self
                .notation_ui
                .selected
                .as_ref()
                .is_some_and(|s| s.0 == bar.line);
            let near = pointer.is_some_and(|p| shape.expand(8.0).contains(p));
            if !selected && !near {
                continue;
            }
            for lower in [false, true] {
                let center = if lower {
                    shape.center_bottom()
                } else {
                    shape.center_top()
                };
                // Both full edges, including the selection outline, are grips.
                // An actual message endpoint keeps its small circular hit area.
                if pointer.is_some_and(|p| {
                    message_ends.iter().any(|end| {
                        (end.y - center.y).abs() < 3.0
                            && (end.x - center.x).abs() <= shape.width() / 2.0 + 3.0
                            && end.distance(p) < 5.5
                    })
                }) {
                    continue;
                }
                let hit =
                    egui::Rect::from_center_size(center, egui::vec2(shape.width() + 12.0, 16.0));
                let response = ui.interact(
                    hit,
                    ui.scope_id()
                        .with(("execution-edge", bar.line, bar.y.to_bits(), lower)),
                    egui::Sense::click_and_drag(),
                );
                let color = appearance::color(ui.ctx(), appearance::ACCENT);
                ui.painter().line_segment(
                    [
                        egui::pos2(shape.left(), center.y),
                        egui::pos2(shape.right(), center.y),
                    ],
                    egui::Stroke::new(if response.hovered() { 3.0 } else { 2.0 }, color),
                );
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
                owned |= response.hovered() || response.is_pointer_button_down_on();
                if response.clicked() {
                    self.notation_select(bar.line, Some(bar.id.clone()));
                }
                if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
                    let owner = self
                        .preview
                        .as_ref()
                        .and_then(|p| {
                            p.regions.iter().find(|r| {
                                r.kind == "sequence-execution"
                                    && r.line == bar.line
                                    && (r.x - bar.x).abs() < 0.01
                                    && (r.y - bar.y).abs() < 0.01
                            })
                        })
                        .map(|r| r.id.clone());
                    self.notation_select(bar.line, Some(bar.id.clone()));
                    self.notation_ui.drag = None;
                    self.notation_ui.sequence.event_drag = Some(Drag {
                        revision: self.document.revision(),
                        origin: ui.input(|i| i.pointer.press_origin()).unwrap_or(center),
                        started: false,
                        line: if lower {
                            bar.end_line.unwrap_or(bar.line)
                        } else {
                            bar.line
                        },
                        selection_line: bar.line,
                        id: bar.id.clone(),
                        close_open: lower && bar.end_line.is_none(),
                        owner,
                        last_alignment: None,
                        shape,
                        lower: Some(lower),
                        last_before: None,
                        candidate: None,
                    });
                    return self.sequence_drag_active(ui, rect);
                }
            }
        }
        owned
    }
    pub(super) fn sequence_body_drag(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        if self.notation_ui.sequence.interacting() {
            return false;
        }
        let Some(preview) = &self.preview else {
            return false;
        };
        let Some(pointer) = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| ui.clip_rect().contains(*p))
        else {
            return false;
        };
        let bounds = preview.bounds;
        let scale = f64::from(rect.width()) / (bounds.2 - bounds.0);
        let x = bounds.0 + f64::from(pointer.x - rect.left()) / scale;
        let y = bounds.1 + f64::from(pointer.y - rect.top()) / scale;
        let Some(hit) = super::super::canvas::hit(&preview.regions, x, y, 7.0 / scale) else {
            return false;
        };
        if !matches!(
            hit.kind.as_str(),
            "message" | "note" | "continuation" | "annotation" | "frame" | "decomposition"
        ) {
            return false;
        }
        if matches!(hit.kind.as_str(), "frame" | "decomposition")
            && x > hit.x + 7.0 / scale
            && x < hit.x + hit.width - 7.0 / scale
            && y > hit.y + 28.0
            && y < hit.y + hit.height - 7.0 / scale
        {
            return false;
        }
        let hit = hit.clone();
        let shape = screen(rect, bounds, &hit);
        // Capture only the actual hover point, not the bounding box of a long
        // bent message or an empty fragment interior.
        let response = ui.interact(
            egui::Rect::from_center_size(pointer, egui::Vec2::splat(16.0)),
            ui.scope_id()
                .with(("sequence-event-drag", hit.line, &hit.id)),
            egui::Sense::click_and_drag(),
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if response.clicked() {
            self.notation_select(hit.line, Some(hit.id.clone()));
        }
        if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
            self.notation_select(hit.line, Some(hit.id.clone()));
            self.notation_ui.drag = None;
            self.notation_ui.sequence.event_drag = Some(Drag {
                revision: self.document.revision(),
                origin: ui.input(|i| i.pointer.press_origin()).unwrap_or(pointer),
                started: false,
                line: hit.line,
                selection_line: hit.line,
                id: hit.id,
                close_open: false,
                owner: None,
                last_alignment: None,
                shape,
                lower: None,
                last_before: None,
                candidate: None,
            });
            return self.sequence_drag_active(ui, rect);
        }
        response.hovered() || response.is_pointer_button_down_on()
    }
    fn sequence_drag_active(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        let Some(mut drag) = self.notation_ui.sequence.event_drag.take() else {
            return false;
        };
        if drag.revision != self.document.revision()
            || !self.can_edit_visually()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            return true;
        }
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else {
            return true;
        };
        drag.started |= pointer.distance(drag.origin) >= 6.0;
        if !drag.started {
            if ui.input(|i| i.pointer.primary_down()) {
                self.notation_ui.sequence.event_drag = Some(drag);
            }
            return true;
        }
        let Some(preview) = &self.preview else {
            return true;
        };
        let bounds = preview.bounds;
        let slot = ui
            .clip_rect()
            .contains(pointer)
            .then(|| {
                preview
                    .regions
                    .iter()
                    .filter(|r| r.kind == "sequence-slot" && !r.id.contains('.'))
                    .min_by(|a, b| {
                        (canvas::point(rect, bounds, a.x, a.y).y - pointer.y)
                            .abs()
                            .total_cmp(&(canvas::point(rect, bounds, b.x, b.y).y - pointer.y).abs())
                    })
            })
            .flatten();
        let aligned = drag.lower.and_then(|lower| {
            let owner = drag.owner.as_deref()?;
            if !ui.clip_rect().contains(pointer) {
                return None;
            }
            preview
                .regions
                .iter()
                .filter(|r| r.kind == "execution" && r.line != drag.selection_line)
                .filter_map(|r| {
                    let target_owner = preview
                        .regions
                        .iter()
                        .find(|a| {
                            a.kind == "sequence-execution"
                                && a.line == r.line
                                && (a.x - r.x).abs() < 0.01
                                && (a.y - r.y).abs() < 0.01
                        })?
                        .id
                        .clone();
                    if target_owner.contains('.') || owner.contains('.') {
                        return None;
                    }
                    let shape = screen(rect, bounds, r);
                    let y = if lower { shape.bottom() } else { shape.top() };
                    ((pointer.y - y).abs() <= 12.0).then_some((r.line, target_owner, y, shape))
                })
                .min_by(|a, b| (pointer.y - a.2).abs().total_cmp(&(pointer.y - b.2).abs()))
        });
        let mut guide = None;
        if let Some((line, owner, y, _)) = &aligned {
            let key = (*line, owner.clone());
            if drag.last_alignment.as_ref() != Some(&key) {
                drag.last_alignment = Some(key);
                drag.last_before = None;
                drag.candidate = Some(edit::sequence_align::align(
                    self.document.source(),
                    drag.owner.as_deref().unwrap(),
                    drag.selection_line,
                    owner,
                    *line,
                    drag.lower.unwrap(),
                ));
            }
            let y = *y;
            guide = Some(y);
        } else if let Some(slot) = slot {
            let was_aligned = drag.last_alignment.take().is_some();
            if drag.last_before != Some(slot.line) || was_aligned {
                drag.last_before = Some(slot.line);
                drag.candidate = Some(if let Some(lower) = drag.lower {
                    drag.owner
                        .as_deref()
                        .ok_or_else(|| "The activation owner could not be resolved".to_owned())
                        .and_then(|owner| {
                            edit::sequence_align::resize(
                                self.document.source(),
                                owner,
                                drag.selection_line,
                                drag.line,
                                lower,
                                drag.close_open,
                                slot.line,
                            )
                        })
                } else {
                    edit::sequence_drag::move_event(self.document.source(), drag.line, slot.line)
                });
            }
            guide = Some(canvas::point(rect, bounds, slot.x, slot.y).y);
        }
        if let Some(y) = guide {
            let valid = drag
                .candidate
                .as_ref()
                .is_some_and(|result| result.as_ref().is_ok_and(|(edits, _)| !edits.is_empty()));
            let color = appearance::color(
                ui.ctx(),
                if valid {
                    appearance::ACCENT
                } else {
                    appearance::MUTED
                },
            );
            if valid {
                ui.painter()
                    .hline(ui.clip_rect().x_range(), y, egui::Stroke::new(2.0, color));
                if let Some((_, _, _, target)) = &aligned {
                    ui.painter().line_segment(
                        [egui::pos2(target.left(), y), egui::pos2(target.right(), y)],
                        egui::Stroke::new(3.0, color),
                    );
                }
                let mut ghost = drag.shape;
                if let Some(lower) = drag.lower {
                    if lower {
                        ghost.max.y = y.max(ghost.min.y + 2.0);
                    } else {
                        ghost.min.y = y.min(ghost.max.y - 2.0);
                    }
                } else {
                    ghost = ghost.translate(egui::vec2(0.0, y - ghost.center().y));
                }
                ui.painter().rect_stroke(
                    ghost,
                    2.0,
                    egui::Stroke::new(1.5, color),
                    egui::StrokeKind::Outside,
                );
            } else {
                let unchanged = drag.candidate.as_ref().is_some_and(Result::is_ok);
                ui.painter().text(
                    pointer + egui::vec2(14.0, 18.0),
                    egui::Align2::LEFT_TOP,
                    if unchanged {
                        crate::i18n::tr("Already at this position")
                    } else {
                        crate::i18n::tr("Invalid event position")
                    },
                    egui::FontId::proportional(12.0),
                    color,
                );
            }
            ui.ctx().set_cursor_icon(if valid {
                egui::CursorIcon::ResizeVertical
            } else {
                egui::CursorIcon::NotAllowed
            });
        }
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            if guide.is_some()
                && let Some(candidate) = drag.candidate
            {
                match candidate {
                    Ok((edits, moved_line)) if !edits.is_empty() => {
                        let selected = moved_line;
                        let id = if drag.id.starts_with("line:") {
                            format!("line:{selected}")
                        } else {
                            drag.id
                        };
                        self.submit_command(Command {
                            edits,
                            label: if drag.lower.is_some() {
                                "Resize activation"
                            } else {
                                "Move sequence element"
                            }
                            .into(),
                            expectation: worker::Expectation::Notation {
                                line: Some(selected),
                                id: Some(id),
                            },
                        });
                    }
                    Ok(_) => {}
                    Err(error) => self.command_error = Some(error),
                }
            }
        } else if ui.input(|i| i.pointer.primary_down()) {
            self.notation_ui.sequence.event_drag = Some(drag);
        }
        true
    }
}
