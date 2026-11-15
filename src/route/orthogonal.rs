//! Axis-aligned connector candidates. Feasibility and label checks remain in the router.
use super::{Point, Route, preferred::forced_port};
use crate::geometry::{BoxNode, NodeShape};
use crate::syntax::PortSide;

pub(super) fn endpoints(
    node: &BoxNode,
    member: Option<&str>,
    side: Option<PortSide>,
    primary: PortSide,
    explore: bool,
    other: Point,
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
            if explore && node.shape == NodeShape::Rectangle {
                if node.kind == crate::syntax::NodeKind::Package
                    && !node.c4_boundary
                    && let Ok((w, h)) = crate::geometry::package_outline::tab(node)
                    && s == PortSide::Top
                    && w >= 24.0
                    && node.width - w >= 24.0
                {
                    for x in [
                        node.x + w / 2.0,
                        other.x.clamp(node.x + 12.0, node.x + w - 12.0),
                    ] {
                        ports.push(Point { x, y: node.y });
                    }
                    ports.push(Point {
                        x: other.x.clamp(node.x + w + 12.0, node.x + node.width - 12.0),
                        y: node.y + h,
                    });
                } else if node.kind != crate::syntax::NodeKind::Package {
                    let projected = match s {
                        PortSide::Top | PortSide::Bottom if node.width >= 24.0 => Some(Point {
                            x: other.x.clamp(node.x + 12.0, node.x + node.width - 12.0),
                            y: centered.y,
                        }),
                        PortSide::Left | PortSide::Right if node.height >= 24.0 => Some(Point {
                            x: centered.x,
                            y: other.y.clamp(node.y + 12.0, node.y + node.height - 12.0),
                        }),
                        _ => None,
                    };
                    ports.extend(projected);
                }
            }
            if (s == primary || explore)
                && node.shape == NodeShape::Rectangle
                && node.width >= 36.0
                && node.height >= 36.0
            {
                let mut shifts = vec![-12.0, 12.0];
                if explore {
                    let half = match s {
                        PortSide::Left | PortSide::Right => node.height / 2.0,
                        PortSide::Top | PortSide::Bottom => node.width / 2.0,
                    };
                    shifts.extend([12.0 - half, half - 12.0]);
                }
                for shift in shifts {
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

#[allow(clippy::too_many_arguments)] // Both endpoints, mandatory sides and current obstacles define candidates.
pub(super) fn candidates(
    a: &BoxNode,
    b: &BoxNode,
    source_member: Option<&str>,
    target_member: Option<&str>,
    source_side: Option<PortSide>,
    target_side: Option<PortSide>,
    previous: &[Route],
    obstacles: Option<&[BoxNode]>,
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
    let from = endpoints(
        a,
        source_member,
        source_side,
        source_primary,
        obstacles.is_some(),
        cb,
    );
    let to = endpoints(
        b,
        target_member,
        target_side,
        target_primary,
        obstacles.is_some(),
        ca,
    );
    let tracks = obstacles.map(|boxes| {
        (
            super::detours::tracks(boxes, true, (ca.x + cb.x) / 2.0, true),
            super::detours::tracks(boxes, false, (ca.y + cb.y) / 2.0, true),
        )
    });
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
    let contours = Contours::new(a, b);
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
            if let Some((xs, ys)) = &tracks {
                // Mixed faces need a local corridor too: an L path alone can be
                // blocked even though a short right-to-top path exists nearby.
                let horizontal_start =
                    (start.x - a.x).abs() < 0.01 || (start.x - a.x - a.width).abs() < 0.01;
                let horizontal_end =
                    (end.x - b.x).abs() < 0.01 || (end.x - b.x - b.width).abs() < 0.01;
                if horizontal_start != horizontal_end {
                    for &x in xs {
                        for &y in ys {
                            let path = if horizontal_start {
                                vec![
                                    start,
                                    Point { x, y: start.y },
                                    Point { x, y },
                                    Point { x: end.x, y },
                                    end,
                                ]
                            } else {
                                vec![
                                    start,
                                    Point { x: start.x, y },
                                    Point { x, y },
                                    Point { x, y: end.y },
                                    end,
                                ]
                            };
                            if contours.normal(&path, false) {
                                result.push(path);
                            }
                        }
                    }
                } else {
                    for &track in if horizontal_start { xs } else { ys } {
                        result.push(if horizontal_start {
                            vec![
                                start,
                                Point {
                                    x: track,
                                    y: start.y,
                                },
                                Point { x: track, y: end.y },
                                end,
                            ]
                        } else {
                            vec![
                                start,
                                Point {
                                    x: start.x,
                                    y: track,
                                },
                                Point { x: end.x, y: track },
                                end,
                            ]
                        });
                    }
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
fn valid_departure(
    node: &BoxNode,
    port: Point,
    outside: Point,
    allow_slant: bool,
    visible_top: Option<(f64, f64)>,
) -> bool {
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

pub(super) struct Contours<'a> {
    a: &'a BoxNode,
    b: &'a BoxNode,
    tabs: [Option<(f64, f64)>; 2],
}
impl<'a> Contours<'a> {
    pub(super) fn new(a: &'a BoxNode, b: &'a BoxNode) -> Self {
        let tabs = [a, b].map(|n| {
            if n.kind == crate::syntax::NodeKind::Package && !n.c4_boundary {
                crate::geometry::package_outline::tab(n).ok()
            } else {
                None
            }
        });
        Self { a, b, tabs }
    }
    pub(super) fn normal(&self, points: &[Point], slant: bool) -> bool {
        points.len() >= 2
            && valid_departure(self.a, points[0], points[1], slant, self.tabs[0])
            && valid_departure(
                self.b,
                *points.last().unwrap(),
                points[points.len() - 2],
                slant,
                self.tabs[1],
            )
    }
}
#[cfg(test)]
pub(super) fn normal_ends(points: &[Point], a: &BoxNode, b: &BoxNode) -> bool {
    Contours::new(a, b).normal(points, false)
}
#[cfg(test)]
pub(super) fn non_tangent_ends(points: &[Point], a: &BoxNode, b: &BoxNode) -> bool {
    Contours::new(a, b).normal(points, true)
}

pub(super) fn axis_aligned(points: &[Point]) -> bool {
    points
        .windows(2)
        .all(|p| (p[0].x - p[1].x).abs() < 1e-4 || (p[0].y - p[1].y).abs() < 1e-4)
}

#[cfg(test)]
#[path = "orthogonal_tests.rs"]
mod tests;
