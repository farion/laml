//! Place children inside measured containers before solving the outer graph.
use super::{BoxNode, GeometryError, constraints};
mod package_lanes;
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
            let actor_usecase = diagram.settings.diagram_type
                == crate::syntax::DiagramType::UseCase
                && parent.is_none()
                && matches!(
                    (from.kind, to.kind),
                    (NodeKind::UseCaseActor, NodeKind::UseCase)
                        | (NodeKind::UseCase, NodeKind::UseCaseActor)
                );
            if relation.from != from.id
                || relation.to != to.id
                || (!actor_usecase
                    && (!matches!(
                        from.kind,
                        NodeKind::Generic | NodeKind::Package | NodeKind::Object
                    ) || from.kind != to.kind))
                || (from.kind == NodeKind::Package
                    && diagram.settings.diagram_type != crate::syntax::DiagramType::Package)
                || (parent.is_some() && from.kind == NodeKind::Package)
            {
                return None;
            }
            // For vertically stacked targets of an object fan-out, keep the
            // upper branch in its own lane and align the lower branch.
            if from.kind == NodeKind::Object
                && diagram.relations.iter().any(|other| {
                    other.from == relation.from
                        && other.to != relation.to
                        && diagram.layout.iter().any(|rule| {
                            (rule.first == to.id
                                && rule.second == other.to
                                && rule.placement == Placement::Above)
                                || (rule.first == other.to
                                    && rule.second == to.id
                                    && rule.placement == Placement::Below)
                        })
                })
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
            let anchor = |node: &BoxNode| {
                if actor_usecase && node.kind == NodeKind::UseCaseActor {
                    node.y + super::usecase::FIGURE_CENTER_Y
                } else {
                    center(node)
                }
            };
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
            // When an authored rule stacks the connected objects, the free
            // axis is horizontal: align their centres instead of pulling the
            // visitor aside through a left-to-right topology preference.
            let stacked = from.kind == NodeKind::Object
                && diagram.layout.iter().any(|rule| {
                    matches!(rule.placement, Placement::Above | Placement::Below)
                        && ((rule.first == from.id && rule.second == to.id)
                            || (rule.first == to.id && rule.second == from.id))
                });
            let x_center = |node: &BoxNode| node.x + node.width / 2.0;
            Some(ScopedAlignment {
                first: a,
                second: b,
                axis: if stacked {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                },
                delta: if stacked {
                    owner_a.width / 2.0 + x_center(from)
                        - x_center(owner_a)
                        - owner_b.width / 2.0
                        - x_center(to)
                        + x_center(owner_b)
                } else {
                    owner_a.height / 2.0 + anchor(from)
                        - center(owner_a)
                        - owner_b.height / 2.0
                        - anchor(to)
                        + center(owner_b)
                        - upper_clearance
                },
                line: 0,
            })
        })
        .collect()
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
    graph.attachments = diagram
        .attachments
        .iter()
        .filter(|attachment| {
            local.iter().any(|node| node.id == attachment.source)
                && graph
                    .relations
                    .iter()
                    .any(|relation| relation.name.as_deref() == Some(&attachment.target))
        })
        .cloned()
        .collect();
    let preferred = relation_alignments(diagram, boxes, &local, parent);
    constraints::place(&graph, &mut local, styles, &preferred)?;
    package_lanes::open_package_lanes(diagram, &mut local, parent);
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
