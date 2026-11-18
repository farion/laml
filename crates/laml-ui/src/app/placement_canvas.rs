use super::*;
use laml::syntax::Placement;
mod root;

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

fn container_zones(rect: egui::Rect) -> [(Placement, egui::Rect); 4] {
    let gap = 4.0;
    let band = 22.0;
    [
        (
            Placement::Above,
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() - gap - band),
                egui::pos2(rect.right(), rect.top() - gap),
            ),
        ),
        (
            Placement::Below,
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() + gap),
                egui::pos2(rect.right(), rect.bottom() + gap + band),
            ),
        ),
        (
            Placement::Left,
            egui::Rect::from_min_max(
                egui::pos2(rect.left() - gap - band, rect.top()),
                egui::pos2(rect.left() - gap, rect.bottom()),
            ),
        ),
        (
            Placement::Right,
            egui::Rect::from_min_max(
                egui::pos2(rect.right() + gap, rect.top()),
                egui::pos2(rect.right() + gap + band, rect.bottom()),
            ),
        ),
    ]
}

fn without_header(zone: egui::Rect, header: egui::Rect) -> Vec<egui::Rect> {
    let cut = zone.intersect(header);
    if !cut.is_positive() {
        return vec![zone];
    }
    [
        egui::Rect::from_min_max(zone.min, egui::pos2(zone.right(), cut.top())),
        egui::Rect::from_min_max(egui::pos2(zone.left(), cut.bottom()), zone.max),
        egui::Rect::from_min_max(
            egui::pos2(zone.left(), cut.top()),
            egui::pos2(cut.left(), cut.bottom()),
        ),
        egui::Rect::from_min_max(
            egui::pos2(cut.right(), cut.top()),
            egui::pos2(zone.right(), cut.bottom()),
        ),
    ]
    .into_iter()
    .filter(|r| r.is_positive())
    .collect()
}

impl Editor {
    pub(super) fn placement_canvas(
        &mut self,
        ui: &mut egui::Ui,
        canvas: egui::Rect,
        response: &egui::Response,
    ) {
        if self.placement.released {
            return;
        }
        if self.relation_drag.is_some()
            || self.creation.connect
            || self.graph.insertion.is_some()
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
                self.placement.drag_origin = Some(origin);
                self.selection.click(Some(&node.id), false);
                self.property_drafts.clear();
                self.command_error = None;
                self.source_jump = Some(node.line);
            }
        }
        let Some((from, revision)) = self.placement.drag.clone() else {
            return;
        };
        let Some(moved) = preview.diagram.nodes.iter().find(|n| n.id == from) else {
            return;
        };
        let mut targets = Vec::new();
        for node in &preview.nodes {
            if node.id == from {
                continue;
            }
            let Some(semantic) = preview.diagram.nodes.iter().find(|n| n.id == node.id) else {
                continue;
            };
            let rect = egui::Rect::from_min_max(
                to_screen(node.x, node.y),
                to_screen(node.x + node.width, node.y + node.height),
            );
            if semantic.parent == moved.parent {
                let areas = if laml_ui::notation::container_kind(semantic.kind) {
                    container_zones(rect)
                } else {
                    zones(rect)
                };
                for (side, zone) in areas {
                    let key = placement::Key {
                        revision,
                        from: from.clone(),
                        target: node.id.clone(),
                        side: Some(side),
                    };
                    if self.placement.allowed(&key, self.document.source()) {
                        targets.push((node.id.clone(), Some(side), zone));
                    }
                }
            }
            let key = placement::Key {
                revision,
                from: from.clone(),
                target: node.id.clone(),
                side: None,
            };
            if laml_ui::notation::container_kind(semantic.kind)
                && let Some((x, y, right, bottom)) = node.container_body()
                && self.placement.allowed(&key, self.document.source())
            {
                targets.push((
                    node.id.clone(),
                    None,
                    egui::Rect::from_min_max(to_screen(x, y), to_screen(right, bottom)),
                ));
            }
        }
        // A nested header is not an implicit membership target of its parent.
        // Cut it out of both painting and picking, including overlapping side areas.
        for node in preview.nodes.iter().filter(|n| n.id != from) {
            if let Some((_, body_y, _, _)) = node.container_body() {
                let header = egui::Rect::from_min_max(
                    to_screen(node.x, node.y),
                    to_screen(node.x + node.width, body_y),
                );
                targets = targets
                    .into_iter()
                    .flat_map(|(id, side, zone)| {
                        without_header(zone, header)
                            .into_iter()
                            .map(move |r| (id.clone(), side, r))
                    })
                    .collect();
            }
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered = pointer.and_then(|pointer| {
            targets
                .iter()
                .filter(|(_, _, zone)| zone.contains(pointer))
                .min_by(|a, b| a.2.area().total_cmp(&b.2.area()))
        });
        let mut key = hovered.map(|(target, side, _)| placement::Key {
            revision,
            from: from.clone(),
            target: target.clone(),
            side: *side,
        });
        if key.is_none()
            && moved.parent.is_some()
            && let Some(pointer) =
                pointer.filter(|p| ui.clip_rect().contains(*p) && response.rect.contains(*p))
            && root::empty(preview, canvas, pointer)
        {
            let root = placement::Key::root(revision, from.clone());
            if self.placement.allowed(&root, self.document.source()) {
                key = Some(root);
            }
        }
        self.placement
            .hover(key, self.document.source(), &self.path);
        if (matches!(&self.placement.plan, Some(Err(_)))
            || matches!(&self.placement.compiled, Some(Err(_))))
            && let Some(key) = self.placement.key.clone()
            && !self.placement.rejected.contains(&key)
        {
            self.placement.rejected.push(key);
        }
        if let Some(pointer) = pointer
            && let Some(key) = self.placement.key.as_ref().filter(|k| k.is_root())
            && !self.placement.rejected.contains(key)
        {
            root::feedback(ui, pointer, self.placement.ready());
        }
        for (target, side, zone) in &targets {
            let key = placement::Key {
                revision,
                from: from.clone(),
                target: target.clone(),
                side: *side,
            };
            if self.placement.rejected.contains(&key) {
                continue;
            }
            let active = self
                .placement
                .key
                .as_ref()
                .is_some_and(|key| key.target == *target && key.side == *side);
            let color = if active {
                if self.placement.ready() {
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
                color.gamma_multiply(if active { 0.48 } else { 0.23 }),
            );
            ui.painter().rect_stroke(
                *zone,
                2.0,
                egui::Stroke::new(if active { 3.0 } else { 1.5 }, color),
                egui::StrokeKind::Inside,
            );
            if side.is_none()
                && zone.width() > 70.0
                && zone.height() > 26.0
                && !targets
                    .iter()
                    .any(|(id, s, r)| id == target && s == side && r.area() > zone.area())
            {
                ui.painter().text(
                    zone.center(),
                    egui::Align2::CENTER_CENTER,
                    crate::i18n::tr("Move into container"),
                    egui::FontId::proportional(12.0),
                    appearance::color(ui.ctx(), appearance::TEXT),
                );
            }
        }
        if let Some(pointer) = pointer
            && let Some(node) = preview.nodes.iter().find(|n| n.id == from)
        {
            let rect = egui::Rect::from_min_max(
                to_screen(node.x, node.y),
                to_screen(node.x + node.width, node.y + node.height),
            );
            let origin = self.placement.drag_origin.unwrap_or(rect.center());
            let ghost = rect.translate(pointer - origin);
            let color = appearance::color(ui.ctx(), appearance::ACCENT);
            ui.painter()
                .rect_filled(ghost, 4.0, color.gamma_multiply(0.10));
            ui.painter().rect_stroke(
                ghost,
                4.0,
                egui::Stroke::new(2.0, color),
                egui::StrokeKind::Inside,
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
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
    fn directional_container_targets_are_outside_the_frame() {
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(200.0, 150.0));
        for (_, zone) in container_zones(rect) {
            assert!(!rect.intersects(zone));
            assert!(zone.is_positive());
        }
    }
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
