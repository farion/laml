//! Route-space picking independent of route-vector/source-vector ordering.
use crate::preview::Preview;
use laml::{
    route::{Point, Route},
    syntax::EndPosition,
};

pub fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
    };
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

pub fn hit(preview: &Preview, x: f64, y: f64, tolerance: f64) -> Option<usize> {
    let p = Point { x, y };
    preview
        .routes
        .iter()
        .filter(|route| {
            preview
                .diagram
                .relations
                .iter()
                .any(|r| r.line == route.source_line)
        })
        .filter_map(|route| {
            let mut distance = preview
                .route_paths
                .iter()
                .filter(|p| p.line == route.source_line)
                .flat_map(|p| &p.paths)
                .flat_map(|path| path.windows(2))
                .map(|pair| segment_distance(p, pair[0], pair[1]))
                .fold(f64::INFINITY, f64::min);
            for label in preview
                .route_paths
                .iter()
                .filter(|p| p.line == route.source_line)
                .flat_map(|p| &p.labels)
            {
                let angle = -label.angle.to_radians();
                let dx = x - label.x;
                let dy = y - label.y;
                let lx = dx * angle.cos() - dy * angle.sin();
                let ly = dx * angle.sin() + dy * angle.cos();
                if lx.abs() <= label.width / 2.0 && ly.abs() <= label.height / 2.0 {
                    distance = 0.0;
                }
            }
            (distance <= tolerance).then_some((route.source_line, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(line, _)| line)
}

pub fn endpoint(route: &Route, end: EndPosition) -> Option<Point> {
    if end == EndPosition::Source {
        route.points.first().copied()
    } else {
        route.points.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_parallel_arrows_loops_and_survives_route_reordering() {
        let source = "semantics {\n A\n B\n @first A -> B\n A -> B\n A -> A\n}\n";
        let mut preview = crate::preview::build(source, std::path::Path::new("test.laml")).unwrap();
        preview.routes.reverse();
        for route in &preview.routes {
            assert!(
                preview
                    .diagram
                    .relations
                    .iter()
                    .any(|r| r.line == route.source_line)
            );
            let picked = route.points.windows(2).any(|pair| {
                let x = (pair[0].x + pair[1].x) / 2.0;
                let y = (pair[0].y + pair[1].y) / 2.0;
                hit(&preview, x, y, 0.01) == Some(route.source_line)
            });
            assert!(picked, "unpickable route at line {}", route.source_line);
        }
    }
    #[test]
    fn picking_uses_visible_bridge_curves_and_excludes_underpass_gaps() {
        let mut preview = crate::preview::build(
            "semantics {\n A\n B\n A -> B\n}\n",
            std::path::Path::new("test.laml"),
        )
        .unwrap();
        let route = &mut preview.routes[0];
        route.points = vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 }];
        route.label = None;
        route.bridges = vec![laml::route::Bridge {
            segment: 0,
            distance: 60.0,
            radius: 6.0,
            height: 5.0,
        }];
        route.gaps = vec![laml::route::Gap {
            segment: 0,
            distance: 20.0,
            half: 5.0,
        }];
        preview.route_paths[0].paths =
            laml::svg::route_pick_paths(route, &preview.nodes, preview.diagram.settings.theme)
                .unwrap();
        preview.route_paths[0].labels.clear();
        assert_eq!(hit(&preview, 60.0, -5.0, 0.1), Some(4));
        assert_eq!(hit(&preview, 20.0, 0.0, 1.0), None);
    }
    #[test]
    fn visibility_filtering_does_not_reassign_source_identity() {
        let source = "semantics {\n A\n B\n C\n @hidden A -> B\n @visible B -> C\n}\nstyling {\n relation[hidden] {\n display: none\n }\n}\n";
        let preview = crate::preview::build(source, std::path::Path::new("test.laml")).unwrap();
        assert_eq!(preview.diagram.relations.len(), 2);
        assert_eq!(preview.routes.len(), 1);
        assert_eq!(preview.routes[0].source_line, 6);
        let path = &preview.route_paths[0].paths[0];
        let p = path[path.len() / 2];
        assert_eq!(hit(&preview, p.x, p.y, 0.1), Some(6));
    }
}
