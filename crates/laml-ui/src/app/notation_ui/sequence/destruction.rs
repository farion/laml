//! Destruction X grips snap to another standalone destruction's final row.
use super::*;

#[derive(Clone)]
pub(super) struct Drag {
    owner: String,
    line: usize,
    revision: u64,
    x: f32,
}

impl Editor {
    pub(super) fn sequence_destructions(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        if self.notation_ui.sequence.destruction_drag.is_none() && self.notation_ui.sequence.interacting() {
            return false;
        }
        let Some(preview) = &self.preview else { return false; };
        let bounds = preview.bounds;
        let ends: Vec<_> = preview.regions.iter().filter(|r| r.kind == "destroyed-end").cloned().collect();
        let mut owned = false;
        for end in &ends {
            let center = canvas::point(rect, bounds, end.x + end.width / 2.0, end.y + end.height / 2.0);
            let response = ui.interact(egui::Rect::from_center_size(center, egui::Vec2::splat(24.0)),
                ui.scope_id().with(("destruction-grip", end.line, &end.id)), egui::Sense::click_and_drag());
            if response.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical); }
            owned |= response.hovered() || response.is_pointer_button_down_on();
            if response.clicked() { self.notation_select(end.line, Some(end.id.clone())); }
            if response.drag_started_by(egui::PointerButton::Primary) {
                let valid = laml::sequence::parse(self.document.source()).map_err(|e| e.message)
                    .and_then(|doc| laml::sequence::destruction_layout::standalone_line(&doc, &end.id));
                match valid {
                    Ok(line) if line == end.line => {
                        self.notation_ui.sequence.destruction_drag = Some(Drag {
                            owner: end.id.clone(), line, revision: self.document.revision(), x: center.x,
                        });
                    }
                    Ok(_) => self.command_error = Some("Choose a standalone destruction in this interaction".into()),
                    Err(error) => self.command_error = Some(error),
                }
            }
        }
        let Some(drag) = self.notation_ui.sequence.destruction_drag.clone() else { return owned; };
        if drag.revision != self.document.revision() || !self.can_edit_visually()
            || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.notation_ui.sequence.destruction_drag = None;
            return true;
        }
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else { return true; };
        let target = ends.iter().filter(|end| end.id != drag.owner).filter_map(|end| {
            let point = canvas::point(rect, bounds, end.x + end.width / 2.0, end.y + end.height / 2.0);
            ((pointer.y - point.y).abs() <= 14.0).then_some((end, point))
        }).min_by(|a, b| (pointer.y - a.1.y).abs().total_cmp(&(pointer.y - b.1.y).abs()));
        let candidate = target.map(|(end, _)| edit::sequence_destruction::align(self.document.source(), &drag.owner, &end.id));
        let valid = candidate.as_ref().is_some_and(Result::is_ok);
        let y = if valid {
            let target_id = &target.unwrap().0.id;
            let mut members = laml::sequence::parse(self.document.source()).ok()
                .and_then(|doc| laml::sequence::destruction_layout::groups(&doc).into_iter()
                    .find(|g| g.contains(target_id))).unwrap_or_else(|| vec![target_id.clone()]);
            members.push(drag.owner.clone());
            ends.iter().filter(|end| members.contains(&end.id)).map(|end|
                canvas::point(rect, bounds, end.x + end.width / 2.0, end.y + end.height / 2.0).y)
                .reduce(f32::max).unwrap_or(pointer.y)
        } else { pointer.y };
        let color = appearance::color(ui.ctx(), if valid { appearance::ACCENT } else { appearance::MUTED });
        let center = egui::pos2(drag.x, y);
        for direction in [-1.0, 1.0] {
            ui.painter().line_segment([center + egui::vec2(-7.0, -7.0 * direction),
                center + egui::vec2(7.0, 7.0 * direction)], egui::Stroke::new(2.0, color));
        }
        if valid && let Some((_, point)) = target {
            let aligned = egui::pos2(point.x, y);
            ui.painter().line_segment([center, aligned], egui::Stroke::new(1.0, color));
            ui.painter().circle_stroke(aligned, 13.0, egui::Stroke::new(2.0, color));
        }
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            self.notation_ui.sequence.destruction_drag = None;
            if let Some(candidate) = candidate {
                match candidate {
                    Ok(edits) if !edits.is_empty() => {
                        let line = laml_ui::relation_edit::remap_line(self.document.source(), drag.line, &edits);
                        self.submit_command(Command { edits, label: "Align destruction occurrences".into(),
                            expectation: worker::Expectation::Notation { line: Some(line), id: Some(drag.owner) } });
                    }
                    Ok(_) => {}
                    Err(error) => self.command_error = Some(error),
                }
            }
        } else if !ui.input(|i| i.pointer.primary_down()) {
            self.notation_ui.sequence.destruction_drag = None;
        }
        true
    }
}
