//! Participant heads drag horizontally with an insertion marker, never a connector.
use super::*;
#[derive(Clone)]
pub(super) struct Drag {
    id: String,
    line: usize,
    revision: u64,
    origin: egui::Pos2,
    shape: egui::Rect,
    started: bool,
    last_target: Option<Option<String>>,
    candidate: Option<Result<Vec<laml_ui::document::Edit>, String>>,
}
impl Editor {
    pub(super) fn sequence_participants(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        if self.notation_ui.sequence.participant_drag.is_none() && self.notation_ui.sequence.interacting() { return false; }
        let Some(preview) = &self.preview else { return false; };
        let bounds = preview.bounds;
        let mut heads = Vec::new();
        for r in preview.regions.iter().filter(|r| r.kind == "participant" && !r.id.contains('.')) {
            if heads.iter().any(|(id, _, _)| id == &r.id) { continue; }
            let shape = egui::Rect::from_min_max(canvas::point(rect, bounds, r.x, r.y),
                canvas::point(rect, bounds, r.x+r.width, r.y+r.height));
            heads.push((r.id.clone(), r.line, shape));
        }
        heads.sort_by(|a,b| a.2.center().x.total_cmp(&b.2.center().x));
        if self.notation_ui.sequence.participant_drag.is_none() {
            for (id, line, shape) in &heads {
                let response = ui.interact(*shape, ui.scope_id().with(("participant-order", id)), egui::Sense::click_and_drag());
                if response.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                if response.clicked() { self.notation_select(*line, Some(id.clone())); }
                if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
                    self.notation_select(*line, Some(id.clone()));
                    self.notation_ui.drag = None;
                    self.notation_ui.sequence.participant_drag = Some(Drag { id: id.clone(), line: *line,
                        revision: self.document.revision(), origin: ui.input(|i| i.pointer.press_origin()).unwrap_or(shape.center()),
                        shape: *shape, started: false, last_target: None, candidate: None });
                    break;
                }
            }
        }
        let Some(mut drag) = self.notation_ui.sequence.participant_drag.take() else { return false; };
        if drag.revision != self.document.revision() || ui.input(|i| i.key_pressed(egui::Key::Escape)) { return true; }
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else { return true; };
        drag.started |= (pointer.x-drag.origin.x).abs() >= 6.0;
        let within = ui.clip_rect().contains(pointer);
        if drag.started && within {
            let others: Vec<_> = heads.iter().filter(|(id, _, _)| id != &drag.id).collect();
            if !others.is_empty() {
                let at = others.iter().position(|(_, _, r)| pointer.x < r.center().x).unwrap_or(others.len());
                let target = others.get(at).map(|(id, _, _)| id.clone());
                if drag.last_target.as_ref() != Some(&target) {
                    drag.candidate = Some(edit::sequence_participants::reorder(self.document.source(), &drag.id, target.as_deref()));
                    drag.last_target = Some(target);
                }
                let x = if at == 0 { others[0].2.left() - 14.0 }
                    else if at == others.len() { others[at-1].2.right() + 14.0 }
                    else { (others[at-1].2.right()+others[at].2.left()) / 2.0 };
                let color = appearance::color(ui.ctx(), appearance::ACCENT);
                ui.painter().line_segment([egui::pos2(x, drag.shape.top()-8.0), egui::pos2(x, drag.shape.bottom()+8.0)], egui::Stroke::new(3.0, color));
                let ghost = drag.shape.translate(egui::vec2(pointer.x-drag.origin.x, 0.0));
                ui.painter().rect_stroke(ghost, 4.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Outside);
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            if drag.started && within && let Some(candidate) = drag.candidate {
                match candidate {
                    Ok(edits) if !edits.is_empty() => {
                        let line = laml_ui::relation_edit::remap_line(self.document.source(), drag.line, &edits);
                        self.submit_command(Command { edits, label: "Reorder participants".into(),
                            expectation: worker::Expectation::Notation { line: Some(line), id: Some(drag.id) } });
                    }
                    Ok(_) => {}
                    Err(error) => self.command_error = Some(error),
                }
            }
        } else if ui.input(|i| i.pointer.primary_down()) { self.notation_ui.sequence.participant_drag = Some(drag); }
        true
    }
}
