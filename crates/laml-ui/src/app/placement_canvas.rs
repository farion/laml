use super::*;
use laml::syntax::Placement;

fn zones(rect: egui::Rect) -> [(Placement, egui::Rect); 4] {
    let band = 22.0_f32.min(rect.width() / 3.0).min(rect.height() / 3.0);
    [
        (
            Placement::Above,
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + band)),
        ),
        (
            Placement::Below,
            egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - band), rect.max),
        ),
        (
            Placement::Left,
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + band),
                egui::pos2(rect.left() + band, rect.bottom() - band),
            ),
        ),
        (
            Placement::Right,
            egui::Rect::from_min_max(
                egui::pos2(rect.right() - band, rect.top() + band),
                egui::pos2(rect.right(), rect.bottom() - band),
            ),
        ),
    ]
}

impl Editor {
    pub(super) fn placement_canvas(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        response: &egui::Response,
    ) {
        if self.relation_drag.is_some()
            || self.creation.connect
            || !self.can_edit_visually()
            || ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.placement.cancel();
            return;
        }
        if self
            .placement
            .drag
            .as_ref()
            .is_some_and(|(_, revision)| *revision != self.document.revision())
        {
            self.placement.cancel();
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let (left, top, right, bottom) = preview.bounds;
        let to_screen = |x: f64, y: f64| {
            egui::pos2(
                canvas.left() + ((x - left) / (right - left)) as f32 * canvas.width(),
                canvas.top() + ((y - top) / (bottom - top)) as f32 * canvas.height(),
            )
        };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let x = left + f64::from((origin.x - canvas.left()) / canvas.width()) * (right - left);
            let y = top + f64::from((origin.y - canvas.top()) / canvas.height()) * (bottom - top);
            if let Some(node) = laml_ui::selection::hit_node(&preview.nodes, x, y) {
                self.placement.drag = Some((node.id.clone(), self.document.revision()));
                self.selection.click(Some(&node.id), false);
                self.property_drafts.clear();
                self.command_error = None;
                self.source_jump = Some(node.line);
            }
        }
        let Some((from, revision)) = self.placement.drag.clone() else {
            return;
        };
        let mut targets = Vec::new();
        for node in &preview.nodes {
            if node.id == from {
                continue;
            }
            let rect = egui::Rect::from_min_max(
                to_screen(node.x, node.y),
                to_screen(node.x + node.width, node.y + node.height),
            );
            for (side, zone) in zones(rect) {
                targets.push((node.id.clone(), side, zone));
            }
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered = pointer.and_then(|pointer| {
            targets
                .iter()
                .filter(|(_, _, zone)| zone.contains(pointer))
                .min_by(|a, b| a.2.area().total_cmp(&b.2.area()))
        });
        let key = hovered.map(|(target, side, _)| placement::Key {
            revision,
            from: from.clone(),
            target: target.clone(),
            side: *side,
        });
        self.placement
            .hover(key, self.document.source(), &self.path);
        for (target, side, zone) in &targets {
            let active = self
                .placement
                .key
                .as_ref()
                .is_some_and(|key| key.target == *target && key.side == *side);
            let color = if active {
                if matches!(&self.placement.plan, Some(Err(_)))
                    || matches!(&self.placement.compiled, Some(Err(_)))
                {
                    egui::Color32::LIGHT_RED
                } else if self.placement.ready() {
                    egui::Color32::LIGHT_GREEN
                } else {
                    egui::Color32::YELLOW
                }
            } else {
                egui::Color32::LIGHT_BLUE
            };
            ui.painter().rect_filled(
                *zone,
                2.0,
                color.gamma_multiply(if active { 0.30 } else { 0.12 }),
            );
            ui.painter().rect_stroke(
                *zone,
                2.0,
                egui::Stroke::new(if active { 2.0 } else { 1.0 }, color),
                egui::StrokeKind::Inside,
            );
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            self.finish_placement(ui.ctx());
        } else if !ui.input(|i| i.pointer.primary_down()) {
            self.placement.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zones_have_distinct_centers_and_stay_within_the_node() {
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(120.0, 60.0));
        let zones = zones(rect);
        for (_, zone) in zones {
            assert!(rect.contains_rect(zone));
            assert_eq!(
                zones
                    .iter()
                    .filter(|(_, other)| other.contains(zone.center()))
                    .count(),
                1
            );
            assert!(zone.is_positive());
        }
    }
}
