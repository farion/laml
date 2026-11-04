//! Generic straight paths and named-port normal exits.
use super::{
    Point,
    geometry::{center, port},
};
use crate::geometry::{BoxNode, NodeShape};
use crate::syntax::PortSide;

pub(super) fn forced_port(node: &BoxNode, side: PortSide) -> Point {
    let c = center(node);
    let towards = match side {
        PortSide::Left => Point {
            x: c.x - 1000.0,
            y: c.y,
        },
        PortSide::Right => Point {
            x: c.x + 1000.0,
            y: c.y,
        },
        PortSide::Top => Point {
            x: c.x,
            y: c.y - 1000.0,
        },
        PortSide::Bottom => Point {
            x: c.x,
            y: c.y + 1000.0,
        },
    };
    port(node, towards)
}

// Prefer a horizontal stroke when compatible visible contours overlap in Y.
// A diagonal centre-to-centre line is not the only straight route between them.
pub(super) fn horizontal_connection(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    let horizontal_box = |node: &BoxNode| {
        matches!(
            node.kind,
            crate::syntax::NodeKind::Generic
                | crate::syntax::NodeKind::Package
                | crate::syntax::NodeKind::Object
        ) && node.shape != NodeShape::Note
    };
    let actor_usecase = matches!(
        (a.kind, b.kind),
        (
            crate::syntax::NodeKind::UseCaseActor,
            crate::syntax::NodeKind::UseCase
        ) | (
            crate::syntax::NodeKind::UseCase,
            crate::syntax::NodeKind::UseCaseActor
        )
    );
    if !actor_usecase && (!horizontal_box(a) || !horizontal_box(b)) {
        return None;
    }
    let (left, right, reverse) = if a.x + a.width < b.x {
        (a, b, false)
    } else if b.x + b.width < a.x {
        (b, a, true)
    } else {
        return None;
    };
    let solid_top = |node: &BoxNode| {
        if node.kind == crate::syntax::NodeKind::Package && !node.c4_boundary {
            node.y
                + crate::geometry::package_outline::tab(node).map_or(14.0, |(_, height)| height)
                + 4.0
        } else {
            node.y + 4.0
        }
    };
    let (top, bottom, y) = if actor_usecase {
        let actor = if left.kind == crate::syntax::NodeKind::UseCaseActor {
            left
        } else {
            right
        };
        let usecase = if left.kind == crate::syntax::NodeKind::UseCase {
            left
        } else {
            right
        };
        let y = actor.y + crate::geometry::usecase::FIGURE_CENTER_Y;
        (usecase.y + 4.0, usecase.y + usecase.height - 4.0, y)
    } else {
        let top = solid_top(left).max(solid_top(right));
        let bottom = (left.y + left.height - 4.0).min(right.y + right.height - 4.0);
        (top, bottom, center(left).y)
    };
    if y < top || y > bottom {
        return None;
    }
    let y = if actor_usecase {
        y
    } else {
        y.clamp(top, bottom)
    };
    let contour = |node: &BoxNode, right_side: bool| {
        let center = center(node);
        if node.kind == crate::syntax::NodeKind::UseCaseActor {
            return Point {
                x: center.x
                    + if right_side {
                        crate::geometry::usecase::DOCK_RX
                    } else {
                        -crate::geometry::usecase::DOCK_RX
                    },
                y,
            };
        }
        let ry = node.height / 2.0;
        let ratio = ((y - center.y) / ry).abs();
        let width = match node.shape {
            NodeShape::Rectangle => node.width / 2.0,
            NodeShape::Ellipse => node.width / 2.0 * (1.0 - ratio * ratio).max(0.0).sqrt(),
            NodeShape::Diamond => node.width / 2.0 * (1.0 - ratio).max(0.0),
            NodeShape::Note => unreachable!(),
        };
        Point {
            x: center.x + if right_side { width } else { -width },
            y,
        }
    };
    let points = vec![contour(left, true), contour(right, false)];
    Some(if reverse {
        points.into_iter().rev().collect()
    } else {
        points
    })
}

// A fallback straight connection uses the figure's invisible contour and the
// use-case ellipse. The actor caption and its visible arms are not ports.
pub(super) fn actor_usecase_straight(a: &BoxNode, b: &BoxNode) -> Option<Vec<Point>> {
    let (actor, usecase, reverse) = match (a.kind, b.kind) {
        (crate::syntax::NodeKind::UseCaseActor, crate::syntax::NodeKind::UseCase) => (a, b, false),
        (crate::syntax::NodeKind::UseCase, crate::syntax::NodeKind::UseCaseActor) => (b, a, true),
        _ => return None,
    };
    let hand = port(actor, center(usecase));
    let points = vec![hand, port(usecase, hand)];
    Some(if reverse {
        points.into_iter().rev().collect()
    } else {
        points
    })
}

// For a mostly lateral actor/use-case relation obstructed by another use case,
// leave the figure from its side before routing around that obstacle. The
// generic exterior candidates might otherwise pick the actor's head first.
pub(super) fn actor_side_detours(a: &BoxNode, b: &BoxNode, boxes: &[BoxNode]) -> Vec<Vec<Point>> {
    let (actor, usecase, actor_at_start) = match (a.kind, b.kind) {
        (crate::syntax::NodeKind::UseCaseActor, crate::syntax::NodeKind::UseCase) => (a, b, true),
        (crate::syntax::NodeKind::UseCase, crate::syntax::NodeKind::UseCaseActor) => (b, a, false),
        _ => return Vec::new(),
    };
    let (ac, uc) = (center(actor), center(usecase));
    if (uc.x - ac.x).abs() < (uc.y - ac.y).abs() {
        return Vec::new(); // A genuinely vertical connection may use the head.
    }
    let rightward = uc.x > ac.x;
    let mut options: Vec<_> = super::detours::corridors(a, b, boxes, false)
        .into_iter()
        .filter(|points| {
            if points.len() < 4 {
                return false;
            }
            let (at, next) = if actor_at_start {
                (points[0], points[1])
            } else {
                (*points.last().unwrap(), points[points.len() - 2])
            };
            (at.y - ac.y).abs() < 1e-6
                && (next.y - at.y).abs() < 1e-6
                && if rightward {
                    next.x > at.x
                } else {
                    next.x < at.x
                }
        })
        .collect();
    // Below the use-case row is preferable here: it keeps the title area and
    // the actor's head clear. Route feasibility still decides the outcome.
    let bottom = usecase.y + usecase.height;
    options.sort_by(|a, b| {
        let score = |points: &Vec<Point>| {
            let below = points.iter().any(|p| p.y > bottom + 8.0);
            let length: f64 = points
                .windows(2)
                .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
                .sum();
            (!below, length)
        };
        let (a_below, a_length) = score(a);
        let (b_below, b_length) = score(b);
        a_below.cmp(&b_below).then(a_length.total_cmp(&b_length))
    });
    // Prefer an elbow outside the subject frame, then a crossing with room
    // below the use-case row. Generic obstacle tracks can hug its left edge.
    if let Some(frame) = boxes.iter().find(|node| {
        node.kind == crate::syntax::NodeKind::UseCaseSystem
            && usecase.x >= node.x
            && usecase.x + usecase.width <= node.x + node.width
    }) {
        let y = bottom + 14.0;
        let side = forced_port(
            actor,
            if rightward {
                PortSide::Right
            } else {
                PortSide::Left
            },
        );
        let x = if rightward {
            (side.x + 24.0).min(frame.x - 12.0)
        } else {
            (side.x - 24.0).max(frame.x + frame.width + 12.0)
        };
        if y < frame.y + frame.height - 14.0 && (x - side.x) * (uc.x - ac.x) > 0.0 {
            let bottom_port = port(
                usecase,
                Point {
                    x: uc.x,
                    y: y + 100.0,
                },
            );
            let mut path = vec![
                side,
                Point { x, y: side.y },
                Point { x, y },
                Point { x: uc.x, y },
                bottom_port,
            ];
            if !actor_at_start {
                path.reverse();
            }
            options.insert(0, path);
        }
    }
    options
}

pub(super) fn named_port_candidates(
    a: &BoxNode,
    b: &BoxNode,
    source: Option<(Point, PortSide)>,
    target: Option<(Point, PortSide)>,
) -> Vec<Vec<Point>> {
    let side_towards = |node: &BoxNode, point: Point| {
        if point.x < node.x {
            PortSide::Left
        } else if point.x > node.x + node.width {
            PortSide::Right
        } else if point.y < node.y {
            PortSide::Top
        } else {
            PortSide::Bottom
        }
    };
    let start = source.map_or_else(
        || forced_port(a, side_towards(a, target.map(|p| p.0).unwrap_or(center(b)))),
        |p| p.0,
    );
    let end = target.map_or_else(
        || forced_port(b, side_towards(b, source.map(|p| p.0).unwrap_or(center(a)))),
        |p| p.0,
    );
    let lead = |point: Point, side: PortSide| match side {
        PortSide::Left => Point {
            x: point.x - 18.0,
            ..point
        },
        PortSide::Right => Point {
            x: point.x + 18.0,
            ..point
        },
        PortSide::Top => Point {
            y: point.y - 18.0,
            ..point
        },
        PortSide::Bottom => Point {
            y: point.y + 18.0,
            ..point
        },
    };
    let first = source.map_or(start, |port| lead(start, port.1));
    let last = target.map_or(end, |port| lead(end, port.1));
    [0.5, 0.25, 0.75]
        .into_iter()
        .map(|fraction| {
            let x = first.x + (last.x - first.x) * fraction;
            let mut points = vec![start, first];
            if (first.y - last.y).abs() > 1e-6 {
                points.extend([Point { x, y: first.y }, Point { x, y: last.y }]);
            }
            points.extend([last, end]);
            points.dedup_by(|p, q| (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6);
            points
        })
        .collect()
}

pub(super) fn normal_at_port(points: &[Point], side: PortSide, source: bool) -> bool {
    if points.len() < 2 {
        return false;
    }
    let (at, next) = if source {
        (points[0], points[1])
    } else {
        (*points.last().unwrap(), points[points.len() - 2])
    };
    match side {
        PortSide::Left => (next.y - at.y).abs() < 1e-6 && next.x < at.x - 1e-6,
        PortSide::Right => (next.y - at.y).abs() < 1e-6 && next.x > at.x + 1e-6,
        PortSide::Top => (next.x - at.x).abs() < 1e-6 && next.y < at.y - 1e-6,
        PortSide::Bottom => (next.x - at.x).abs() < 1e-6 && next.y > at.y + 1e-6,
    }
}

#[cfg(test)]
mod candidate_tests {
    use super::super::geometry::route_is_clear;
    use super::super::*;
    use crate::geometry::place;
    use crate::parser::parse;
    use crate::styles::StyleSheet;

    #[test]
    fn shifted_ports_keep_bent_detours_orthogonal_on_both_axes() {
        for rule in [
            "A < B : align=center; gap=220",
            "A above B : align=center; gap=220",
        ] {
            let source = format!("semantics {{\nA\nB\n}}\nlayout {{\n{rule}\n}}\n");
            let diagram = parse(&source).unwrap();
            let boxes = place(&diagram, &StyleSheet::default()).unwrap();
            let options = candidates(&diagram, 0, 0, 1, &boxes, &[], 12.0, false);
            let valid: Vec<_> = options
                .iter()
                .filter(|points| route_is_clear(points, &boxes, &[]))
                .collect();
            assert!(valid.iter().any(|points| points.len() == 4), "{rule}");
            for points in valid {
                assert!(
                    points
                        .windows(2)
                        .all(|pair| (pair[0].x - pair[1].x).abs() < 1e-6
                            || (pair[0].y - pair[1].y).abs() < 1e-6),
                    "{rule}: {points:?}"
                );
            }
        }
    }
}
