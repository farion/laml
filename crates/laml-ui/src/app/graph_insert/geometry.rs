//! Keep plus controls separate from arrow grips, ports and rendered paths.
use super::*;
pub(super) fn positions(
    preview: &Preview,
    canvas: egui::Rect,
    node: &laml::geometry::BoxNode,
) -> Vec<(Placement, egui::Pos2)> {
    let rect = connection_geometry::node_rect(preview, canvas, node);
    let mut obstacles = Vec::new();
    for other in &preview.nodes {
        let r = connection_geometry::node_rect(preview, canvas, other);
        if other.id != node.id && !r.contains_rect(rect) {
            obstacles.push(r.expand(2.0));
        }
        for p in &other.ports {
            let at = connection_geometry::screen(preview, canvas, p.x, p.y);
            obstacles.push(egui::Rect::from_center_size(at, egui::Vec2::splat(16.0)));
        }
    }
    for route in &preview.route_paths {
        for path in &route.paths {
            for pair in path.windows(2) {
                let a = connection_geometry::screen(preview, canvas, pair[0].x, pair[0].y);
                let b = connection_geometry::screen(preview, canvas, pair[1].x, pair[1].y);
                obstacles.push(egui::Rect::from_two_pos(a, b).expand(3.0));
            }
        }
        for label in &route.labels {
            let a = connection_geometry::screen(preview, canvas, label.x, label.y);
            let b = connection_geometry::screen(
                preview,
                canvas,
                label.x + label.width,
                label.y + label.height,
            );
            obstacles.push(egui::Rect::from_two_pos(a, b).expand(3.0));
        }
    }
    for route in &preview.routes {
        for end in [
            laml::syntax::EndPosition::Source,
            laml::syntax::EndPosition::Target,
        ] {
            if let Some(p) = connection_geometry::endpoint_grip(preview, canvas, route, end) {
                obstacles.push(egui::Rect::from_center_size(p, egui::Vec2::splat(22.0)));
            }
        }
    }
    let mut placed = Vec::new();
    let mut result = Vec::new();
    for (side, at) in super::pluses(rect) {
        let horizontal = matches!(side, Placement::Above | Placement::Below);
        let mut candidates = vec![at];
        for r in &obstacles {
            if horizontal {
                candidates.extend([
                    egui::pos2(r.left() - 14.0, at.y),
                    egui::pos2(r.right() + 14.0, at.y),
                ]);
            } else {
                candidates.extend([
                    egui::pos2(at.x, r.top() - 14.0),
                    egui::pos2(at.x, r.bottom() + 14.0),
                ]);
            }
        }
        candidates.sort_by(|a, b| a.distance_sq(at).total_cmp(&b.distance_sq(at)));
        let chosen = candidates
            .into_iter()
            .find(|p| {
                // Keep controls reachable through the owning node's hover envelope.
                if !rect.expand(39.0).contains(*p) {
                    return false;
                }
                let area = egui::Rect::from_center_size(*p, egui::Vec2::splat(22.0));
                !obstacles.iter().chain(&placed).any(|r| r.intersects(area))
            })
            .unwrap_or(at);
        placed.push(egui::Rect::from_center_size(
            chosen,
            egui::Vec2::splat(24.0),
        ));
        result.push((side, chosen));
    }
    result
}
