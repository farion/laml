use super::*;
use laml::syntax::EndPosition;
use laml_ui::relation_selection::{endpoint, hit};

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
        for route in &preview.routes {
            if !self.selection.relations.contains(&route.source_line)
                || moving == Some(route.source_line)
            {
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
            if self.selection.relations.len() == 1
                && self.selection.ids.is_empty()
                && self.selection.ports.is_empty()
                && preview
                    .diagram
                    .relations
                    .iter()
                    .any(|r| r.line == route.source_line)
            {
                for end in [EndPosition::Source, EndPosition::Target] {
                    if let Some(point) = endpoint(route, end) {
                        ui.painter().circle_filled(
                            screen(point),
                            connection_geometry::endpoint_radius(preview, rect),
                            egui::Color32::LIGHT_BLUE,
                        );
                    }
                }
            }
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
