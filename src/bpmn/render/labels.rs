//! Place labels against their own routed line, not against the longest leg.
//! All edges are routed first so candidates can avoid every other line, node,
//! pool/lane border and previously placed label.
use super::super::geometry::BoxAt;
use super::orthogonal::{Point, Prior};
use super::{Document, Geometry, escape};
use crate::bpmn::{Kind, LabelOrientation};
use std::fmt::Write;

#[derive(Clone, Copy)]
struct Label {
    x: f64,
    y: f64,
    vertical: bool,
    box_at: BoxAt,
    cost: f64,
}
fn clashes(a: BoxAt, b: BoxAt, margin: f64) -> bool {
    (a.x - b.x).abs() < (a.w + b.w) / 2.0 + margin && (a.y - b.y).abs() < (a.h + b.h) / 2.0 + margin
}
fn segment_hits(a: Point, b: Point, rect: BoxAt, margin: f64) -> bool {
    let left = rect.x - rect.w / 2.0 - margin;
    let right = rect.x + rect.w / 2.0 + margin;
    let top = rect.y - rect.h / 2.0 - margin;
    let bottom = rect.y + rect.h / 2.0 + margin;
    if (a.0 - b.0).abs() < 0.05 {
        a.0 > left && a.0 < right && a.1.max(b.1) > top && a.1.min(b.1) < bottom
    } else {
        a.1 > top && a.1 < bottom && a.0.max(b.0) > left && a.0.min(b.0) < right
    }
}
fn clear(
    doc: &Document,
    geom: &Geometry,
    routes: &[Prior],
    edge: usize,
    box_at: BoxAt,
    placed: &[BoxAt],
) -> bool {
    if doc
        .nodes
        .iter()
        .enumerate()
        .any(|(i, _)| clashes(box_at, geom.nodes[i], 4.0))
        || placed.iter().any(|b| clashes(box_at, *b, 7.0))
    {
        return false;
    }
    for pool in &geom.pools {
        if [pool.y, pool.y + 30.0, pool.y + pool.h].iter().any(|y| {
            (box_at.y - y).abs() < box_at.h / 2.0 + 4.0
                && box_at.x + box_at.w / 2.0 > pool.x
                && box_at.x - box_at.w / 2.0 < pool.x + pool.w
        }) {
            return false;
        }
        if [pool.x, pool.x + pool.w].iter().any(|x| {
            (box_at.x - x).abs() < box_at.w / 2.0 + 4.0
                && box_at.y + box_at.h / 2.0 > pool.y
                && box_at.y - box_at.h / 2.0 < pool.y + pool.h
        }) {
            return false;
        }
        if doc.direction == crate::syntax::FlowDirection::TopToBottom {
            if pool.bands.iter().any(|band| {
                (box_at.x - band.y).abs() < box_at.w / 2.0 + 4.0
                    && box_at.y + box_at.h / 2.0 > pool.y + 30.0
                    && box_at.y - box_at.h / 2.0 < pool.y + pool.h
            }) {
                return false;
            }
        } else if pool.bands.iter().any(|band| {
            (box_at.y - band.y).abs() < box_at.h / 2.0 + 4.0
                && box_at.x + box_at.w / 2.0 > pool.x
                && box_at.x - box_at.w / 2.0 < pool.x + pool.w
        }) {
            return false;
        }
    }
    !routes.iter().enumerate().any(|(i, route)| {
        i != edge
            && route
                .points
                .windows(2)
                .any(|leg| segment_hits(leg[0], leg[1], box_at, 3.0))
    })
}
fn near_other_line(routes: &[Prior], edge: usize, rect: BoxAt) -> bool {
    routes.iter().enumerate().any(|(i, route)| {
        i != edge
            && route.points.windows(2).any(|leg| {
                let (a, b) = (leg[0], leg[1]);
                if (a.0 - b.0).abs() < 0.05 {
                    rect.y - rect.h / 2.0 < b.1.max(a.1)
                        && rect.y + rect.h / 2.0 > a.1.min(b.1)
                        && (a.0 - rect.x).abs() - rect.w / 2.0 < 24.0
                } else {
                    rect.x - rect.w / 2.0 < a.0.max(b.0)
                        && rect.x + rect.w / 2.0 > a.0.min(b.0)
                        && (a.1 - rect.y).abs() - rect.h / 2.0 < 24.0
                }
            })
    })
}
fn close_to_related_note(doc: &Document, geom: &Geometry, index: usize, box_at: BoxAt) -> bool {
    let Some(source) = doc.nodes.iter().find(|n| n.id == doc.edges[index].from) else {
        return false;
    };
    if source.kind != Kind::Boundary {
        return false;
    }
    let Some(owner) = source.attached.as_deref() else {
        return false;
    };
    doc.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == Kind::Annotation)
        .any(|(i, n)| {
            let related = doc.edges.iter().any(|e| {
                e.kind == crate::bpmn::EdgeKind::Association
                    && ((e.from == n.id && e.to == owner) || (e.to == n.id && e.from == owner))
            });
            let note = geom.nodes[i];
            related
                && (box_at.x - note.x).abs() < (box_at.w + note.w) / 2.0 + 40.0
                && (box_at.y - note.y).abs() < (box_at.h + note.h) / 2.0 + 10.0
        })
}
fn choose(
    doc: &Document,
    geom: &Geometry,
    routes: &[Prior],
    index: usize,
    name: &str,
    orientation: LabelOrientation,
    placed: &[BoxAt],
) -> Result<Label, String> {
    let width = crate::text::measure(name, 12.0).unwrap_or(40.0).max(1.0);
    let points = &routes[index].points;
    let route_length: f64 = points
        .windows(2)
        .map(|leg| (leg[1].0 - leg[0].0).abs() + (leg[1].1 - leg[0].1).abs())
        .sum();
    let source = doc
        .nodes
        .iter()
        .find(|n| n.id == doc.edges[index].from)
        .unwrap();
    let near_source = matches!(
        source.kind,
        Kind::Boundary | Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
    );
    let mut best: Option<Label> = None;
    let mut traveled = 0.0;
    for leg in points.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        let horizontal = (a.1 - b.1).abs() < 0.05;
        let length = (a.0 - b.0).abs() + (a.1 - b.1).abs();
        if length < 15.0 {
            traveled += length;
            continue;
        }
        let modes: &[bool] = match orientation {
            LabelOrientation::Horizontal => &[false],
            LabelOrientation::Vertical => &[true],
            LabelOrientation::Auto => &[false, true],
        };
        for &vertical in modes {
            if vertical && horizontal {
                continue;
            }
            // A vertical label reads along a vertical line. Horizontal text can
            // sit either above a horizontal run or beside a vertical one.
            let half = if horizontal || vertical {
                width / 2.0 + 9.0
            } else {
                12.0
            };
            if length < 2.0 * half {
                continue;
            }
            for along in [
                half,
                half + 16.0,
                half + 32.0,
                half + 40.0,
                half + 56.0,
                half + 72.0,
                length * 0.25,
                length * 0.5,
                length * 0.6,
                length * 0.75,
            ] {
                if along < half - 0.1 || along > length - half + 0.1 {
                    continue;
                }
                let mid = (
                    a.0 + (b.0 - a.0) * along / length,
                    a.1 + (b.1 - a.1) * along / length,
                );
                for sign in [-1.0, 1.0] {
                    let (x, y, box_at) = if vertical {
                        let x = mid.0 + sign * 13.0;
                        (
                            x,
                            mid.1,
                            BoxAt {
                                x,
                                y: mid.1,
                                w: 14.0,
                                h: width + 6.0,
                            },
                        )
                    } else if horizontal {
                        let center_y = mid.1 + sign * 13.0;
                        let baseline = if sign < 0.0 {
                            mid.1 - 8.0
                        } else {
                            mid.1 + 18.0
                        };
                        (
                            mid.0,
                            baseline,
                            BoxAt {
                                x: mid.0,
                                y: center_y,
                                w: width + 6.0,
                                h: 14.0,
                            },
                        )
                    } else {
                        let x = mid.0 + sign * (width / 2.0 + 10.0);
                        (
                            x,
                            mid.1 + 4.0,
                            BoxAt {
                                x,
                                y: mid.1,
                                w: width + 6.0,
                                h: 16.0,
                            },
                        )
                    };
                    if !clear(doc, geom, routes, index, box_at, placed) {
                        continue;
                    }
                    let progress = traveled + along;
                    if !near_source
                        && (progress < route_length.min(220.0) / 4.0
                            || route_length - progress < route_length.min(220.0) / 4.0)
                    {
                        continue;
                    }
                    let distance = if near_source {
                        progress * 3.0
                    } else {
                        (progress - route_length / 2.0).abs()
                    };
                    let cost = distance
                        + if close_to_related_note(doc, geom, index, box_at) {
                            400.0
                        } else {
                            0.0
                        }
                        + if source.kind == Kind::Boundary && !horizontal && !vertical {
                            65.0
                        } else {
                            0.0
                        }
                        + if near_other_line(routes, index, box_at) {
                            170.0
                        } else {
                            0.0
                        }
                        + if vertical { 12.0 } else { 0.0 }
                        + if !horizontal && !vertical { 5.0 } else { 0.0 };
                    let candidate = Label {
                        x,
                        y,
                        vertical,
                        box_at,
                        cost,
                    };
                    if best.is_none_or(|current| cost < current.cost - 0.1) {
                        best = Some(candidate);
                    }
                }
            }
        }
        traveled += length;
    }
    best.ok_or_else(|| {
        format!(
            "no clear placement for BPMN label '{name}' on {} -> {}",
            doc.edges[index].from, doc.edges[index].to
        )
    })
}
pub(super) fn render(
    svg: &mut String,
    doc: &Document,
    geom: &Geometry,
    routes: &[Prior],
) -> Result<(), String> {
    let mut labels = Vec::new();
    for (index, edge) in doc.edges.iter().enumerate() {
        let source = doc.nodes.iter().find(|n| n.id == edge.from).unwrap();
        if let Some(name) = edge.label.as_ref().or_else(|| {
            (source.kind == Kind::Boundary && !source.label.is_empty()).then_some(&source.label)
        }) {
            let orientation = doc
                .flow_layout
                .iter()
                .find(|r| r.from == edge.from && r.to == edge.to)
                .and_then(|rule| rule.label_orientation)
                .unwrap_or(LabelOrientation::Auto);
            labels.push((index, name.as_str(), orientation));
        }
    }
    labels.sort_by_key(|(i, _, _)| {
        let source = &doc
            .nodes
            .iter()
            .find(|n| n.id == doc.edges[*i].from)
            .unwrap();
        if matches!(
            source.kind,
            Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
        ) {
            0
        } else if source.kind == Kind::Boundary {
            1
        } else {
            2
        }
    });
    let mut placed = Vec::new();
    for (index, name, orientation) in labels {
        let candidate = choose(doc, geom, routes, index, name, orientation, &placed)?;
        let (x, y) = (candidate.x, candidate.y);
        let transform = if candidate.vertical {
            format!(" transform=\"rotate(-90 {x:.1} {y:.1})\"")
        } else {
            String::new()
        };
        writeln!(svg,"<text class=\"bpmn-flow-label\" x=\"{x:.1}\" y=\"{y:.1}\"{transform} text-anchor=\"middle\" font-family=\"Inter\" font-size=\"12\" fill=\"#182330\">{}</text>",escape(name)).unwrap();
        placed.push(candidate.box_at);
    }
    Ok(())
}
