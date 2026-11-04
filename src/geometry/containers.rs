//! Place children inside measured containers before solving the outer graph.
use super::{BoxNode, GeometryError, constraints};
use crate::styles::StyleSheet;
use crate::syntax::{
    Alignment, Axis, Diagram, FlowDirection, LayoutRule, NodeKind, Placement, ScopedAlignment,
};

fn err(line: usize, message: impl Into<String>) -> GeometryError {
    GeometryError {
        line,
        message: message.into(),
    }
}

fn direct_owner(diagram: &Diagram, endpoint: &str, parent: Option<&str>) -> Option<String> {
    let mut node = diagram
        .nodes
        .iter()
        .find(|node| node.has_endpoint(endpoint))?;
    loop {
        if node.parent.as_deref() == parent {
            return Some(node.id.clone());
        }
        node = diagram
            .nodes
            .iter()
            .find(|candidate| Some(candidate.id.as_str()) == node.parent.as_deref())?;
    }
}

fn descendant(diagram: &Diagram, child: &str, ancestor: &str) -> bool {
    let mut current = diagram
        .nodes
        .iter()
        .find(|node| node.id == child)
        .and_then(|node| node.parent.as_deref());
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = diagram
            .nodes
            .iter()
            .find(|node| node.id == parent)
            .and_then(|node| node.parent.as_deref());
    }
    false
}

fn shift(diagram: &Diagram, boxes: &mut [BoxNode], root: &str, dx: f64, dy: f64) {
    for node in boxes.iter_mut() {
        if node.id == root || descendant(diagram, &node.id, root) {
            node.x += dx;
            node.y += dy;
        }
    }
}

fn projected_alignment(
    rule: &LayoutRule,
    first: &str,
    second: &str,
    boxes: &[BoxNode],
) -> ScopedAlignment {
    let a = boxes
        .iter()
        .find(|node| node.id == rule.first)
        .expect("validated layout endpoint");
    let b = boxes
        .iter()
        .find(|node| node.id == rule.second)
        .expect("validated layout endpoint");
    let oa = boxes
        .iter()
        .find(|node| node.id == first)
        .expect("measured group owner");
    let ob = boxes
        .iter()
        .find(|node| node.id == second)
        .expect("measured group owner");
    let vertical = matches!(rule.placement, Placement::Left | Placement::Right);
    let extent = |node: &BoxNode| if vertical { node.height } else { node.width };
    let anchor = |node: &BoxNode| match rule.align.expect("alignment requested") {
        Alignment::Top | Alignment::Left => 0.0,
        Alignment::Center => extent(node) / 2.0,
        Alignment::Bottom | Alignment::Right => extent(node),
    };
    let coordinate = |node: &BoxNode| if vertical { node.y } else { node.x };
    ScopedAlignment {
        first: first.into(),
        second: second.into(),
        axis: if vertical {
            Axis::Vertical
        } else {
            Axis::Horizontal
        },
        delta: coordinate(a) - coordinate(oa) + anchor(a) - coordinate(b) + coordinate(ob)
            - anchor(b),
        line: rule.line,
    }
}

// A connection can join a child in this scope to a node outside its frame.
// Prefer aligning the visible endpoint centers, not the frame centers, when
// laying out the containing groups. This remains subordinate to layout rules.
fn relation_alignments(
    diagram: &Diagram,
    boxes: &[BoxNode],
    local: &[BoxNode],
    parent: Option<&str>,
) -> Vec<ScopedAlignment> {
    if !matches!(
        diagram.settings.direction,
        FlowDirection::LeftToRight | FlowDirection::RightToLeft
    ) {
        return Vec::new();
    }
    diagram
        .relations
        .iter()
        .filter_map(|relation| {
            let from = boxes
                .iter()
                .find(|node| node.has_endpoint(&relation.from))?;
            let to = boxes.iter().find(|node| node.has_endpoint(&relation.to))?;
            // A named port or member has its own measured attachment position.
            if relation.from != from.id
                || relation.to != to.id
                || !matches!(from.kind, NodeKind::Generic | NodeKind::Package)
                || (from.kind == NodeKind::Package
                    && diagram.settings.diagram_type != crate::syntax::DiagramType::Package)
                || from.kind != to.kind
                || (parent.is_some() && from.kind == NodeKind::Package)
            {
                return None;
            }
            let a = direct_owner(diagram, &relation.from, parent)?;
            let b = direct_owner(diagram, &relation.to, parent)?;
            if a == b {
                return None;
            }
            let owner_a = local.iter().find(|node| node.id == a)?;
            let owner_b = local.iter().find(|node| node.id == b)?;
            let center = |node: &BoxNode| node.y + node.height / 2.0;
            // An upper external package should not crowd a separate relation
            // from a lower sibling into another external package. A small
            // upward preference leaves the incoming horizontal stroke intact.
            let upper_clearance = if diagram.settings.diagram_type
                == crate::syntax::DiagramType::Package
                && parent.is_none()
                && diagram.layout.is_empty()
                && diagram
                    .nodes
                    .iter()
                    .any(|node| node.id == to.id && node.parent.is_none())
                && let Some(source_parent) = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == from.id)
                    .and_then(|node| node.parent.as_deref())
                && diagram.relations.iter().any(|other| {
                    other.from != relation.from
                        && other.to != relation.to
                        && diagram.nodes.iter().any(|node| {
                            node.id == other.from && node.parent.as_deref() == Some(source_parent)
                        })
                        && diagram
                            .nodes
                            .iter()
                            .any(|node| node.id == other.to && node.parent.is_none())
                        && boxes
                            .iter()
                            .any(|node| node.id == other.from && center(node) > center(from) + 16.0)
                }) {
                12.0
            } else {
                0.0
            };
            Some(ScopedAlignment {
                first: a,
                second: b,
                axis: Axis::Vertical,
                delta: owner_a.height / 2.0 + center(from)
                    - center(owner_a)
                    - owner_b.height / 2.0
                    - center(to)
                    + center(owner_b)
                    - upper_clearance,
                line: 0,
            })
        })
        .collect()
}

// In a package frame, a right-hand child need not sit directly in the row of
// its incoming sibling when that blocks another sibling's outward relation.
// Favor the related lower row within that space, so its outgoing route stays
// closer to its target than to unrelated nodes in the upper row. Never move
// an authored position.
fn open_package_lanes(diagram: &Diagram, local: &mut [BoxNode], parent: Option<&str>) {
    if diagram.settings.diagram_type != crate::syntax::DiagramType::Package
        || parent.is_none()
        || !diagram.layout.is_empty()
    {
        return;
    }
    let parent = parent.unwrap();
    let center = |node: &BoxNode| node.y + node.height / 2.0;
    for right in 0..local.len() {
        if local[right].kind != NodeKind::Package
            || local[right].is_container
            || diagram
                .layout
                .iter()
                .any(|rule| rule.first == local[right].id || rule.second == local[right].id)
        {
            continue;
        }
        let candidate = (0..local.len())
            .filter(|&i| {
                local[i].kind == NodeKind::Package
                    && local[i].x + local[i].width + 16.0 < local[right].x
            })
            .flat_map(|upper| (0..local.len()).map(move |lower| (upper, lower)))
            .find(|&(upper, lower)| {
                if upper == lower
                    || (local[upper].x - local[lower].x).abs() > 4.0
                    || center(&local[upper]) + local[upper].height / 2.0
                        >= center(&local[lower]) - local[lower].height / 2.0 + 4.0
                {
                    return false;
                }
                let joined = diagram.relations.iter().any(|relation| {
                    relation.from == local[lower].id && relation.to == local[right].id
                });
                let outward = diagram.relations.iter().any(|relation| {
                    relation.from == local[upper].id
                        && direct_owner(diagram, &relation.to, Some(parent)).is_none()
                });
                joined && outward
            });
        if let Some((upper, lower)) = candidate {
            let proposed = center(&local[upper]) * 0.38 + center(&local[lower]) * 0.62
                - local[right].height / 2.0;
            let clear = local.iter().enumerate().all(|(i, node)| {
                i == right
                    || node.x + node.width + 8.0 <= local[right].x
                    || local[right].x + local[right].width + 8.0 <= node.x
                    || proposed + local[right].height + 8.0 <= node.y
                    || node.y + node.height + 8.0 <= proposed
            });
            if clear {
                local[right].y = proposed;
            }
        }
    }
}

fn scope(
    diagram: &Diagram,
    boxes: &mut [BoxNode],
    styles: &StyleSheet,
    parent: Option<&str>,
) -> Result<(), GeometryError> {
    let children: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| node.parent.as_deref() == parent)
        .map(|node| node.id.clone())
        .collect();
    for child in &children {
        if diagram.nodes.iter().any(|node| {
            node.id == *child
                && (matches!(
                    node.kind,
                    NodeKind::Package
                        | NodeKind::Profile
                        | NodeKind::Component
                        | NodeKind::CompositeStructure
                        | NodeKind::State
                        | NodeKind::UseCaseSystem
                ) || node.kind.is_deployment_target())
                && diagram
                    .nodes
                    .iter()
                    .any(|part| part.parent.as_deref() == Some(child.as_str()))
        }) {
            scope(diagram, boxes, styles, Some(child))?;
        }
    }
    let mut local: Vec<BoxNode> = children
        .iter()
        .map(|id| {
            boxes
                .iter()
                .find(|node| node.id == *id)
                .expect("measured child")
                .clone()
        })
        .collect();
    let mut graph = Diagram {
        settings: diagram.settings.clone(),
        ..Default::default()
    };
    for rule in &diagram.layout {
        let (Some(a), Some(b)) = (
            direct_owner(diagram, &rule.first, parent),
            direct_owner(diagram, &rule.second, parent),
        ) else {
            continue;
        };
        if a == b {
            continue;
        }
        let mut mapped = rule.clone();
        if rule.align.is_some() && (a != rule.first || b != rule.second) {
            graph
                .scoped_alignments
                .push(projected_alignment(rule, &a, &b, boxes));
            mapped.align = None;
        }
        mapped.first = a;
        mapped.second = b;
        graph.layout.push(mapped);
    }
    graph.relations = diagram
        .relations
        .iter()
        .filter_map(|relation| {
            let a = direct_owner(diagram, &relation.from, parent)?;
            let b = direct_owner(diagram, &relation.to, parent)?;
            if a == b {
                return None;
            }
            // An implicit link into a nested package need not put that
            // package to the right of its peer. Stacking the two siblings
            // keeps the child-to-child connection axial through the frame.
            if diagram.settings.diagram_type == crate::syntax::DiagramType::Class
                && relation.to != b
                && local
                    .iter()
                    .any(|node| node.id == b && node.kind == NodeKind::Package)
                && !graph
                    .layout
                    .iter()
                    .any(|r| (r.first == a && r.second == b) || (r.first == b && r.second == a))
            {
                return None;
            }
            let mut mapped = relation.clone();
            mapped.from = a;
            mapped.to = b;
            Some(mapped)
        })
        .collect();
    let preferred = relation_alignments(diagram, boxes, &local, parent);
    constraints::place(&graph, &mut local, styles, &preferred)?;
    open_package_lanes(diagram, &mut local, parent);
    if local.is_empty() {
        return Ok(());
    }
    let min_x = local
        .iter()
        .map(|node| node.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = local
        .iter()
        .map(|node| node.y)
        .fold(f64::INFINITY, f64::min);
    let (base_x, base_y) = if let Some(parent) = parent {
        let box_parent = boxes
            .iter()
            .find(|node| node.id == parent)
            .expect("container measured");
        (
            box_parent.padding + 24.0,
            if box_parent.c4_boundary {
                box_parent.padding + 34.0
            } else if box_parent.kind == NodeKind::CompositeStructure {
                super::composite::header_height(box_parent) + 48.0
            } else if matches!(box_parent.kind, NodeKind::Component | NodeKind::State)
                || box_parent.kind.is_deployment_target()
            {
                box_parent.height + 24.0
            } else {
                box_parent.padding
                    + box_parent.font_size * 1.25 * box_parent.lines.len() as f64
                    + 36.0
            },
        )
    } else {
        (min_x, min_y)
    };
    for node in &local {
        let x = node.x - min_x + base_x;
        let y = node.y - min_y + base_y;
        let existing = boxes
            .iter()
            .find(|b| b.id == node.id)
            .expect("child measured");
        shift(diagram, boxes, &node.id, x - existing.x, y - existing.y);
    }
    if let Some(parent) = parent {
        let index = boxes
            .iter()
            .position(|node| node.id == parent)
            .expect("container measured");
        let right = children
            .iter()
            .map(|id| boxes.iter().find(|node| node.id == *id).unwrap())
            .map(|node| node.x + node.width)
            .fold(0.0, f64::max);
        let bottom = children
            .iter()
            .map(|id| boxes.iter().find(|node| node.id == *id).unwrap())
            .map(|node| node.y + node.height)
            .fold(0.0, f64::max);
        let required_width = right + boxes[index].padding + 24.0;
        let required_height = bottom + boxes[index].padding + 24.0;
        if boxes[index]
            .fixed_width
            .is_some_and(|width| width + 1e-6 < required_width)
            || boxes[index]
                .fixed_height
                .is_some_and(|height| height + 1e-6 < required_height)
        {
            return Err(err(
                boxes[index].line,
                "fixed container cannot fit its child nodes",
            ));
        }
        boxes[index].width = boxes[index].width.max(required_width);
        boxes[index].height = boxes[index].height.max(required_height);
    }
    Ok(())
}

pub(super) fn place(
    diagram: &Diagram,
    boxes: &mut [BoxNode],
    styles: &StyleSheet,
) -> Result<(), GeometryError> {
    for rule in &diagram.layout {
        let a = diagram
            .nodes
            .iter()
            .find(|node| node.id == rule.first)
            .expect("checked layout node");
        let b = diagram
            .nodes
            .iter()
            .find(|node| node.id == rule.second)
            .expect("checked layout node");
        if descendant(diagram, &a.id, &b.id) || descendant(diagram, &b.id, &a.id) {
            return Err(err(
                rule.line,
                "mandatory separation cannot place a child outside its own container",
            ));
        }
    }
    scope(diagram, boxes, styles, None)
}
