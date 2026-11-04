//! Axis-aligned connector candidates. Feasibility and label checks remain in the router.
use super::{Point, Route, preferred::forced_port};
use crate::geometry::{BoxNode, NodeShape};
use crate::syntax::PortSide;

fn endpoints(
    node: &BoxNode,
    member: Option<&str>,
    side: Option<PortSide>,
    primary: PortSide,
) -> Vec<Point> {
    if let Some(name) = member
        && let Some(named) = node.ports.iter().find(|p| p.id == name)
    {
        return vec![Point {
            x: named.x,
            y: named.y,
        }];
    }
    let sides = [
        PortSide::Right,
        PortSide::Left,
        PortSide::Bottom,
        PortSide::Top,
    ];
    sides
        .into_iter()
        .filter(|s| side.is_none_or(|required| required == *s))
        .flat_map(|s| {
            let centered = forced_port(node, s);
            let mut ports = vec![centered];
            if s == primary
                && node.shape == NodeShape::Rectangle
                && node.width >= 36.0
                && node.height >= 36.0
            {
                for shift in [-12.0, 12.0] {
                    let p = match s {
                        PortSide::Right | PortSide::Left => Point {
                            x: centered.x,
                            y: centered.y + shift,
                        },
                        PortSide::Top | PortSide::Bottom => Point {
                            x: centered.x + shift,
                            y: centered.y,
                        },
                    };
                    let valid = match s {
                        PortSide::Right | PortSide::Left => {
                            p.y >= node.y + 6.0 && p.y <= node.y + node.height - 6.0
                        }
                        PortSide::Top | PortSide::Bottom => {
                            p.x >= node.x + 6.0 && p.x <= node.x + node.width - 6.0
                        }
                    };
                    if valid && node.kind != crate::syntax::NodeKind::Package {
                        ports.push(p);
                    } else if valid && node.kind == crate::syntax::NodeKind::Package {
                        // The tab is above the visible body's lateral edges.
                        let Some((tab_width, tab_height)) =
                            crate::geometry::package_outline::tab(node).ok()
                        else {
                            continue;
                        };
                        let on_contour = match s {
                            PortSide::Right | PortSide::Left => p.y >= node.y + tab_height + 6.0,
                            PortSide::Top => p.x >= node.x + tab_width + 6.0,
                            PortSide::Bottom => true,
                        };
                        if on_contour {
                            ports.push(p);
                        }
                    }
                }
            }
            ports
        })
        .collect()
}

pub(super) fn candidates(
    a: &BoxNode,
    b: &BoxNode,
    source_member: Option<&str>,
    target_member: Option<&str>,
    source_side: Option<PortSide>,
    target_side: Option<PortSide>,
    previous: &[Route],
) -> Vec<Vec<Point>> {
    let mut result = Vec::new();
    let ca = super::geometry::center(a);
    let cb = super::geometry::center(b);
    let horizontal = (cb.x - ca.x).abs() >= (cb.y - ca.y).abs();
    let source_primary = if horizontal {
        if cb.x > ca.x {
            PortSide::Right
        } else {
            PortSide::Left
        }
    } else if cb.y > ca.y {
        PortSide::Bottom
    } else {
        PortSide::Top
    };
    let target_primary = match source_primary {
        PortSide::Right => PortSide::Left,
        PortSide::Left => PortSide::Right,
        PortSide::Top => PortSide::Bottom,
        PortSide::Bottom => PortSide::Top,
    };
    let from = endpoints(a, source_member, source_side, source_primary);
    let to = endpoints(b, target_member, target_side, target_primary);
    let outer_x = previous
        .iter()
        .flat_map(|route| route.points.iter().map(|p| p.x))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(x), hi.max(x))
        });
    let outer_y = previous
        .iter()
        .flat_map(|route| route.points.iter().map(|p| p.y))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| {
            (lo.min(y), hi.max(y))
        });
    for start in from {
        for &end in &to {
            if (start.x - end.x).abs() < 0.01 || (start.y - end.y).abs() < 0.01 {
                result.push(vec![start, end]);
                continue;
            }
            result.push(vec![
                start,
                Point {
                    x: end.x,
                    y: start.y,
                },
                end,
            ]);
            result.push(vec![
                start,
                Point {
                    x: start.x,
                    y: end.y,
                },
                end,
            ]);
            let between_x = (start.x + end.x) / 2.0;
            let between_y = (start.y + end.y) / 2.0;
            result.push(vec![
                start,
                Point {
                    x: between_x,
                    y: start.y,
                },
                Point {
                    x: between_x,
                    y: end.y,
                },
                end,
            ]);
            result.push(vec![
                start,
                Point {
                    x: start.x,
                    y: between_y,
                },
                Point {
                    x: end.x,
                    y: between_y,
                },
                end,
            ]);
            // A route that shares a source with an earlier connection may need
            // to pass outside that connection before returning to its target.
            // The node-only tracks cannot clear a long existing route.
            if outer_x.0.is_finite() {
                for track in [outer_x.0 - 64.0, outer_x.1 + 64.0] {
                    result.push(vec![
                        start,
                        Point {
                            x: track,
                            y: start.y,
                        },
                        Point { x: track, y: end.y },
                        end,
                    ]);
                }
                for track in [outer_y.0 - 64.0, outer_y.1 + 64.0] {
                    result.push(vec![
                        start,
                        Point {
                            x: start.x,
                            y: track,
                        },
                        Point { x: end.x, y: track },
                        end,
                    ]);
                }
            }
            for node in [a, b] {
                for track in [node.x - 24.0, node.x + node.width + 24.0] {
                    result.push(vec![
                        start,
                        Point {
                            x: track,
                            y: start.y,
                        },
                        Point { x: track, y: end.y },
                        end,
                    ]);
                }
                for track in [node.y - 24.0, node.y + node.height + 24.0] {
                    result.push(vec![
                        start,
                        Point {
                            x: start.x,
                            y: track,
                        },
                        Point { x: end.x, y: track },
                        end,
                    ]);
                }
            }
        }
    }
    result
}

// For box-like nodes, both terminal strokes must be normal to a visible
// side; following an edge into an arrowhead is never a valid connector.
fn valid_departure(node: &BoxNode, port: Point, outside: Point, allow_slant: bool) -> bool {
    if node.shape != NodeShape::Rectangle
        || !matches!(
            node.kind,
            crate::syntax::NodeKind::Generic
                | crate::syntax::NodeKind::Package
                | crate::syntax::NodeKind::Class
                | crate::syntax::NodeKind::Object
                | crate::syntax::NodeKind::UseCaseSystem
        )
    {
        return true;
    }
    let (dx, dy) = (outside.x - port.x, outside.y - port.y);
    let epsilon = 0.01;
    let normal = |axial: f64, lateral: f64| {
        if allow_slant {
            let distance = axial.hypot(lateral);
            distance >= 20.0 - epsilon && axial >= distance * 0.25
        } else {
            axial >= 20.0 - epsilon && lateral.abs() < epsilon
        }
    };
    let visible_top = if node.kind == crate::syntax::NodeKind::Package && !node.c4_boundary {
        crate::geometry::package_outline::tab(node).ok()
    } else {
        None
    };
    let left = (port.x - node.x).abs() < epsilon && normal(-dx, dy);
    let right = (port.x - node.x - node.width).abs() < epsilon
        && visible_top.is_none_or(|(_, tab_height)| port.y >= node.y + tab_height - epsilon)
        && normal(dx, dy);
    let bottom = (port.y - node.y - node.height).abs() < epsilon && normal(dy, dx);
    let top = visible_top.map_or((port.y - node.y).abs() < epsilon, |(tab_w, tab_h)| {
        ((port.y - node.y).abs() < epsilon && port.x <= node.x + tab_w + epsilon)
            || ((port.y - node.y - tab_h).abs() < epsilon && port.x >= node.x + tab_w - epsilon)
    }) && normal(-dy, dx);
    left || right || top || bottom
}

pub(super) fn normal_ends(points: &[Point], a: &BoxNode, b: &BoxNode) -> bool {
    if points.len() < 2 {
        return false;
    }
    valid_departure(a, points[0], points[1], false)
        && valid_departure(b, *points.last().unwrap(), points[points.len() - 2], false)
}

pub(super) fn non_tangent_ends(points: &[Point], a: &BoxNode, b: &BoxNode) -> bool {
    points.len() >= 2
        && valid_departure(a, points[0], points[1], true)
        && valid_departure(b, *points.last().unwrap(), points[points.len() - 2], true)
}

pub(super) fn axis_aligned(points: &[Point]) -> bool {
    points
        .windows(2)
        .all(|p| (p[0].x - p[1].x).abs() < 1e-4 || (p[0].y - p[1].y).abs() < 1e-4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::place, parser::parse, styles::StyleSheet};

    #[test]
    fn rectangular_connectors_require_normal_departure_and_arrival() {
        let diagram =
            parse("semantics {\nA\nB\nA -> B\n}\nlayout {\nA < B : align=center; gap=100\n}\n")
                .unwrap();
        let boxes = place(&diagram, &StyleSheet::default()).unwrap();
        let (a, b) = (&boxes[0], &boxes[1]);
        let y = a.y + a.height / 2.0;
        let start = Point {
            x: a.x + a.width,
            y,
        };
        let end = Point { x: b.x, y };
        let valid = [
            start,
            Point {
                x: start.x + 20.0,
                y,
            },
            Point { x: end.x - 20.0, y },
            end,
        ];
        assert!(normal_ends(&valid, a, b));
        let tangent = [
            start,
            Point {
                x: start.x,
                y: y + 30.0,
            },
            end,
        ];
        assert!(!normal_ends(&tangent, a, b));
        assert!(!non_tangent_ends(&tangent, a, b));
        let diagonal = [
            start,
            Point {
                x: start.x + 28.0,
                y: y + 15.0,
            },
            end,
        ];
        assert!(non_tangent_ends(&diagonal, a, b));
        let short = [
            start,
            Point {
                x: start.x + 19.0,
                y,
            },
            end,
        ];
        assert!(!normal_ends(&short, a, b));
        let tangent_arrival = [
            start,
            Point {
                x: end.x,
                y: y + 30.0,
            },
            end,
        ];
        assert!(!normal_ends(&tangent_arrival, a, b));
        assert!(!non_tangent_ends(&tangent_arrival, a, b));
    }

    #[test]
    fn offers_a_corridor_beyond_an_existing_long_route() {
        let diagram = parse(
            "semantics {\nA\nB\nC\nA -> B\n}\nlayout {\nA < B : align=center; gap=180\nB above C : gap=80\n}\n",
        )
        .unwrap();
        let sheet = StyleSheet::default();
        let boxes = place(&diagram, &sheet).unwrap();
        let previous = super::super::route(&diagram, &boxes, &sheet).unwrap();
        let outer_x = previous[0]
            .points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max)
            + 64.0;
        let options = candidates(&boxes[0], &boxes[2], None, None, None, None, &previous);
        assert!(options.iter().any(|path| {
            axis_aligned(path) && path.iter().any(|point| (point.x - outer_x).abs() < 1e-6)
        }));
    }
}
