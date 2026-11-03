//! Visible UML package silhouette: tab, shoulder and body.
use super::Point;
use crate::geometry::BoxNode;

pub(super) fn port(node: &BoxNode, towards: Point) -> Option<Point> {
    let (tab_width, tab_height) = crate::geometry::package_outline::tab(node).ok()?;
    let (x, y, right, bottom) = (node.x, node.y, node.x + node.width, node.y + node.height);
    let shoulder = x + tab_width;
    let body_top = y + tab_height;
    let edges = [
        (Point { x, y }, Point { x: shoulder, y }),
        (
            Point { x: shoulder, y },
            Point {
                x: shoulder,
                y: body_top,
            },
        ),
        (
            Point {
                x: shoulder,
                y: body_top,
            },
            Point {
                x: right,
                y: body_top,
            },
        ),
        (
            Point {
                x: right,
                y: body_top,
            },
            Point {
                x: right,
                y: bottom,
            },
        ),
        (
            Point {
                x: right,
                y: bottom,
            },
            Point { x, y: bottom },
        ),
        (Point { x, y: bottom }, Point { x, y }),
    ];
    let c = super::geometry::center(node);
    let (dx, dy) = (towards.x - c.x, towards.y - c.y);
    edges
        .into_iter()
        .filter_map(|(a, b)| {
            let t = if (a.x - b.x).abs() < 1e-9 {
                (dx.abs() > 1e-9).then(|| (a.x - c.x) / dx)?
            } else {
                (dy.abs() > 1e-9).then(|| (a.y - c.y) / dy)?
            };
            let hit = Point {
                x: c.x + t * dx,
                y: c.y + t * dy,
            };
            (t > 1e-9
                && hit.x >= a.x.min(b.x) - 1e-6
                && hit.x <= a.x.max(b.x) + 1e-6
                && hit.y >= a.y.min(b.y) - 1e-6
                && hit.y <= a.y.max(b.y) + 1e-6)
                .then_some((t, hit))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, hit)| hit)
}

pub(super) fn intersects(a: Point, b: Point, node: &BoxNode) -> bool {
    let (tab_width, tab_height) = match crate::geometry::package_outline::tab(node) {
        Ok(dimensions) => dimensions,
        Err(_) => return true,
    };
    super::labels::segment_hits_rect(a, b, node.x, node.y, tab_width, tab_height)
        || super::labels::segment_hits_rect(
            a,
            b,
            node.x,
            node.y + tab_height,
            node.width,
            node.height - tab_height,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_uses_visible_package_body_and_not_empty_tab_corner() {
        let diagram =
            crate::parser::parse("settings {\ntype: package\n}\nsemantics {\npackage A\n}\n")
                .unwrap();
        let boxes =
            crate::geometry::place(&diagram, &crate::styles::StyleSheet::default()).unwrap();
        let n = &boxes[0];
        let center_x = n.x + n.width / 2.0;
        let hit = port(
            n,
            Point {
                x: center_x,
                y: n.y - 50.0,
            },
        )
        .unwrap();
        assert!((hit.x - center_x).abs() < 1e-6);
        assert!((hit.y - n.y - 14.0).abs() < 1e-6);
        let empty = Point {
            x: center_x,
            y: n.y + 7.0,
        };
        assert!(!intersects(
            Point {
                x: center_x,
                y: n.y - 5.0
            },
            empty,
            n
        ));
        assert!(intersects(
            Point {
                x: center_x,
                y: n.y - 5.0
            },
            Point {
                x: center_x,
                y: n.y + 20.0
            },
            n
        ));
    }
}
