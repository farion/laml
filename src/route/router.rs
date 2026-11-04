//! Route relations with notation-aware feasibility checks and bounded candidate ranking.
use super::{
    Route, RouteError, attachments, bridges, candidates, edge_style, geometry::route_is_clear,
    parallel, qualifiers, ranking, route_labels, self_loops, side_rules,
};
use crate::{geometry::BoxNode, styles::StyleSheet, syntax::Diagram};

pub fn route(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
) -> Result<Vec<Route>, RouteError> {
    let stable = route_pass(diagram, boxes, sheet, false)?;
    // UML-specialized relations can require notation-specific arms and docking;
    // preserve those checked candidates instead of optimizing them as generic lines.
    if !matches!(
        diagram.settings.diagram_type,
        crate::syntax::DiagramType::Generic | crate::syntax::DiagramType::Package
    ) || diagram.nodes.iter().any(|node| {
        node.classes.iter().any(|class| {
            matches!(
                class.as_str(),
                "c4Person" | "c4System" | "c4Container" | "c4Component"
            )
        })
    }) {
        return Ok(stable);
    }
    if let Ok(ranked) = route_pass(diagram, boxes, sheet, true)
        && ranking::visibly_clear(&ranked, boxes)
        && diagram.relations.iter().enumerate().all(|(i, relation)| {
            relation.from != relation.to || ranked[i].points == stable[i].points
        })
        && (if diagram.settings.allow_diagonal {
            ranking::better_than(&ranked, &stable)
        } else {
            ranking::better_than_orthogonal(&ranked, &stable)
        })
    {
        return Ok(ranked);
    }
    Ok(stable)
}

fn route_pass(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    ranked: bool,
) -> Result<Vec<Route>, RouteError> {
    let visible = crate::geometry::visible_diagram(diagram, sheet)
        .map_err(|(line, message)| RouteError { line, message })?;
    let diagram = &visible;
    let mut routed = Vec::new();
    for (index, relation) in diagram.relations.iter().enumerate() {
        let owner = |id: &str| {
            boxes
                .iter()
                .position(|b| b.has_endpoint(id))
                .expect("parsed relation endpoint")
        };
        let from = owner(&relation.from);
        let to = owner(&relation.to);
        if from == to && (relation.from != boxes[from].id || relation.to != boxes[to].id) {
            return Err(RouteError {
                line: relation.line,
                message: format!(
                    "relation '{}' ({} -> {}) connects two members of one node; use separate nodes or an exterior self-relation",
                    relation.name.as_deref().unwrap_or("unnamed"),
                    relation.from,
                    relation.to
                ),
            });
        }
        let (color, width, font_size) = edge_style::style(relation, sheet);
        if matches!(
            diagram.settings.diagram_type,
            crate::syntax::DiagramType::Composite | crate::syntax::DiagramType::Profile
        ) && (color == "none" || width < 1.0)
        {
            return Err(RouteError {
                line: relation.line,
                message: "composite connector stroke must remain visible".into(),
            });
        }
        let mut options = if from == to {
            let rank = diagram.relations[..index]
                .iter()
                .filter(|r| r.from == relation.from && r.to == relation.to)
                .count();
            if rank >= 4 {
                return Err(RouteError {
                    line: relation.line,
                    message: format!(
                        "relation '{}' ({} -> {}): more than four self-relations need distinct loop ports; use fewer loops or separate nodes",
                        relation.name.as_deref().unwrap_or("unnamed"),
                        relation.from,
                        relation.to
                    ),
                });
            }
            side_rules::loop_options(
                diagram,
                index,
                &boxes[from],
                self_loops::candidates(&boxes[from], rank),
            )
        } else {
            let offset =
                parallel::offset(diagram, index, boxes, from, to, sheet).map_err(|message| {
                    RouteError {
                        line: relation.line,
                        message,
                    }
                })?;
            candidates(diagram, index, from, to, boxes, &routed, offset, ranked)
        };
        if ranked && from != to {
            ranking::order(
                &mut options,
                &routed,
                boxes,
                from,
                to,
                !diagram.settings.allow_diagonal
                    && diagram.settings.diagram_type == crate::syntax::DiagramType::Package,
                !diagram.settings.allow_diagonal,
            );
        }
        let label_rule = diagram
            .relation_layout
            .iter()
            .find(|rule| relation.name.as_deref() == Some(rule.name.as_str()));
        let end_rules = [
            crate::syntax::EndPosition::Source,
            crate::syntax::EndPosition::Target,
        ]
        .map(|end| {
            diagram.relation_end_layout.iter().find(|rule| {
                relation.name.as_deref() == Some(rule.name.as_str()) && rule.end == end
            })
        });
        let placement = label_rule.map_or(diagram.settings.relation_label, |rule| rule.placement);
        let label_line = label_rule.map_or(relation.line, |rule| rule.line);
        let mut choice = None;
        let mut label_error = None;
        for mut points in options {
            let qualifier_boxes = match qualifiers::attach(
                relation,
                &mut points,
                boxes,
                from,
                to,
                font_size,
                &routed,
            ) {
                Ok(boxes) => boxes,
                Err(message) => {
                    label_error = Some(message);
                    continue;
                }
            };
            if !diagram.settings.allow_diagonal && !super::orthogonal::axis_aligned(&points) {
                continue;
            }
            if !diagram.settings.allow_diagonal
                && diagram.settings.diagram_type == crate::syntax::DiagramType::Composite
                && boxes
                    .iter()
                    .filter(|node| {
                        node.kind == crate::syntax::NodeKind::CompositeStructure
                            && node.is_container
                    })
                    .any(|node| {
                        crate::geometry::composite::internal_label_rect(node).is_ok_and(
                            |(left, top, right, bottom)| {
                                points.windows(2).any(|p| {
                                    super::labels::segment_hits_rect(
                                        p[0],
                                        p[1],
                                        left,
                                        top,
                                        right - left,
                                        bottom - top,
                                    )
                                })
                            },
                        )
                    })
            {
                continue;
            }
            if !route_is_clear(&points, boxes, &routed) {
                continue;
            }
            let Some(plan) = bridges::plan(&points, boxes, &routed, from, to, width) else {
                continue;
            };
            if qualifier_boxes
                .iter()
                .any(|q| !plan.avoids_region(&points, q.x, q.y, q.width, q.height))
            {
                label_error = Some("qualifier conflicts with required bridge".into());
                continue;
            }
            let (label, end_labels, label_gap) = match route_labels::select(
                relation,
                &points,
                boxes,
                &routed,
                &plan,
                &qualifier_boxes,
                placement,
                font_size,
                width,
                end_rules,
                diagram.settings.diagram_type == crate::syntax::DiagramType::Package
                    && sheet.theme.modern(),
            ) {
                Ok(labels) => labels,
                Err(message) => {
                    label_error = Some(message);
                    continue;
                }
            };
            choice = Some((points, qualifier_boxes, label, end_labels, label_gap, plan));
            break;
        }
        let (points, qualifier_boxes, label, end_labels, label_gap, plan) =
            choice.ok_or_else(|| {
                let label_failed = label_error.is_some();
                let reason = label_error.unwrap_or_else(|| {
                    if diagram.settings.allow_diagonal {
                        "no safe path around nodes, other connections, and crossings"
                    } else {
                        "no safe orthogonal path around nodes, other connections, and crossings"
                    }.into()
                });
                let identity = relation.name.as_deref().unwrap_or("unnamed");
                let advice = if let Some(name) = relation.name.as_deref() {
                    format!(
                        "Increase the gap between the connected elements or set relation[{name}].source : side=left (or choose a suitable target side)"
                    )
                } else {
                    "Increase the gap between the connected elements or name the relation to select its attachment sides".into()
                };
                RouteError {
                    line: if label_failed && label_rule.is_some() {
                        label_line
                    } else {
                        relation.line
                    },
                    message: format!(
                        "relation '{identity}' ({} -> {}): {reason}. {advice}",
                        relation.from, relation.to
                    ),
                }
            })?;
        let new_bridges = plan.apply(&mut routed);
        routed.push(Route {
            points,
            qualifiers: qualifier_boxes,
            bridges: new_bridges,
            gaps: label_gap.into_iter().collect(),
            label,
            end_labels,
            from,
            to,
            color,
            width,
            font_size,
            bidirectional: relation.kind == crate::RelationKind::Bidirectional,
            marker_end: relation.kind != crate::RelationKind::Undirected,
            appearance: relation.appearance,
        });
    }
    attachments::route_attachments(diagram, boxes, sheet, &mut routed)?;
    Ok(routed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::place, parser::parse};

    #[test]
    fn picks_shorter_clear_corridor_over_first_external_detour() {
        let source = "semantics {\nA\nB\nC\nA -> C\n}\nlayout {\nA < B : align=center; gap=90\nB < C : align=center; gap=90\n}\n";
        let diagram = parse(source).unwrap();
        let sheet = StyleSheet::default();
        let boxes = place(&diagram, &sheet).unwrap();
        let stable = route_pass(&diagram, &boxes, &sheet, false).unwrap();
        let chosen = route(&diagram, &boxes, &sheet).unwrap();
        assert!(
            ranking::better_than(&chosen, &stable),
            "bounded auto-routing should shorten a safe detour"
        );
        assert!(ranking::visibly_clear(&chosen, &boxes));
    }
}
