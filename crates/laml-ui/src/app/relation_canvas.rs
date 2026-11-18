use super::*;
use laml::syntax::EndPosition;
use laml_ui::relation_selection::hit;

impl Editor {
    /// Paint persistent selection and pick visible routes. Endpoint dragging is
    /// owned by the graph gesture controller, not by inspector state.
    pub(super) fn relation_canvas(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        navigation: bool,
    ) -> bool {
        let Some(preview) = &self.preview else {
            return false;
        };
        let (left, top, right, _) = preview.bounds;
        let scale = rect.width() as f64 / (right - left);
        let screen = |point: laml::route::Point| {
            connection_geometry::screen(preview, rect, point.x, point.y)
        };
        let moving = self.graph.gesture.as_ref().and_then(|g| match g.kind {
            connection_gesture::Kind::Reconnect { line, .. } => Some(line),
            _ => None,
        });
        let hovered = if navigation {
            None
        } else {
            ui.input(|i| i.pointer.hover_pos())
                .filter(|p| ui.clip_rect().contains(*p))
                .and_then(|p| {
                    let x = left + f64::from(p.x - rect.left()) / scale;
                    let y = top + f64::from(p.y - rect.top()) / scale;
                    hit(preview, x, y, 7.0 / scale)
                })
        };
        for route in &preview.routes {
            let selected = self.selection.relations.contains(&route.source_line);
            let hover = hovered == Some(route.source_line);
            if (!selected && !hover) || moving == Some(route.source_line) {
                continue;
            }
            for path in preview
                .route_paths
                .iter()
                .filter(|p| p.line == route.source_line)
                .flat_map(|p| &p.paths)
            {
                ui.painter().add(egui::Shape::line(
                    path.iter().copied().map(screen).collect(),
                    egui::Stroke::new(
                        (route.width as f32 * scale as f32 + 3.0).max(3.0),
                        egui::Color32::from_rgba_unmultiplied(80, 170, 255, 100),
                    ),
                ));
            }
            if (hover
                || (self.selection.relations.len() == 1
                    && self.selection.ids.is_empty()
                    && self.selection.ports.is_empty()))
                && preview
                    .diagram
                    .relations
                    .iter()
                    .any(|r| r.line == route.source_line)
            {
                for end in [EndPosition::Source, EndPosition::Target] {
                    if let Some(point) =
                        connection_geometry::endpoint_grip(preview, rect, route, end)
                    {
                        ui.painter().circle_filled(
                            point,
                            connection_geometry::endpoint_radius(preview, rect),
                            egui::Color32::LIGHT_BLUE,
                        );
                    }
                }
            }
        }
        if !navigation
            && self.graph.gesture.is_none()
            && let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
            && let Some((line, end)) = connection_geometry::endpoint_at(preview, rect, pointer)
            && let Some(route) = preview.routes.iter().find(|r| r.source_line == line)
            && let Some(point) = connection_geometry::endpoint_grip(preview, rect, route, end)
        {
            let color = appearance::color(ui.ctx(), appearance::ACCENT);
            ui.painter().circle_filled(
                point,
                connection_geometry::endpoint_radius(preview, rect),
                color,
            );
            ui.painter().circle_stroke(
                point,
                connection_geometry::endpoint_radius(preview, rect) + 4.0,
                egui::Stroke::new(2.0, color),
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if !navigation
            && response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let x = left + f64::from(pointer.x - rect.left()) / scale;
            let y = top + f64::from(pointer.y - rect.top()) / scale;
            if laml_ui::selection::hit_node(&preview.nodes, x, y).is_none()
                && laml_ui::selection::hit_port(&preview.nodes, x, y, (7.0 / scale).max(5.0))
                    .is_none()
                && let Some(line) = hit(preview, x, y, 7.0 / scale)
            {
                self.selection
                    .click_relation(line, ui.input(|i| i.modifiers.shift));
                self.property_drafts.clear();
                if self.rendered == Some(self.document.revision()) {
                    self.source_jump = Some(line);
                }
                return true;
            }
        }
        false
    }
}
