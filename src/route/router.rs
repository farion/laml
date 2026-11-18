//! Route relations with notation-aware feasibility checks and bounded candidate ranking.
use super::{
    Route, RouteError, attachments, bridges, candidates, edge_style, geometry::route_is_clear,
    parallel, pass_cache::PassCache, qualifiers, ranking, route_labels, self_loops, side_rules,
};
use crate::{geometry::BoxNode, styles::StyleSheet, syntax::Diagram};

fn furthest_failure(stable: RouteError, alternative: RouteError) -> RouteError {
    if alternative.line > stable.line {
        alternative
    } else {
        stable
    }
}

pub fn route(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
) -> Result<Vec<Route>, RouteError> {
    if let Some(routes) = super::corridor::route(diagram, boxes, sheet) {
        return Ok(routes);
    }
    let mut cache = PassCache::default();
    let failure = match route_fast(diagram, boxes, sheet, &mut cache) {
        Ok(routes) => return Ok(routes),
        Err(error) => error,
    };
    if !matches!(
        diagram.settings.diagram_type,
        crate::syntax::DiagramType::Generic | crate::syntax::DiagramType::Package
    ) {
        return Err(failure);
    }
    let visible = crate::geometry::visible_diagram(diagram, sheet)
        .map_err(|(line, message)| RouteError { line, message })?;
    let order: Vec<_> = (0..visible.relations.len()).collect();
    let mut orders = vec![order];
    // An earlier edge can occupy a self-loop's mandatory docking corridor.
    // Retry allocation order too: loop lane ranks are computed from declaration
    // indices below, and returned routes are restored to declaration order.
    orders.extend(super::routing_order::alternatives(&visible, boxes));
    for order in orders {
        if let Ok(routes) = cache.run(&visible, boxes, sheet, Some(usize::MAX), &order, true) {
            return Ok(routes);
        }
    }
    Err(failure)
}

fn route_fast(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    cache: &mut PassCache,
) -> Result<Vec<Route>, RouteError> {
    let visible = crate::geometry::visible_diagram(diagram, sheet)
        .map_err(|(line, message)| RouteError { line, message })?;
    let diagram = &visible;
    let stable = cache.pass(diagram, boxes, sheet, None);
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
        if stable.is_err()
            && !diagram.settings.allow_diagonal
            && diagram.settings.diagram_type == crate::syntax::DiagramType::Generic
        {
            return super::optimize::route(diagram, boxes, sheet, stable, cache);
        }
        return stable;
    }
    if !diagram.settings.allow_diagonal {
        return super::optimize::route(diagram, boxes, sheet, stable, cache);
    }
    // A locally feasible early candidate can block a later relation. If the
    // stable pass fails, retry from scratch with crossing-aware candidate order.
    let stable = match stable {
        Ok(routes) => routes,
        Err(error) => {
            let mut failure = error;
            if let Ok(routes) = cache.pass(diagram, boxes, sheet, Some(usize::MAX))
                && ranking::visibly_clear(&routes, boxes)
            {
                return Ok(routes);
            }
            let first_blocked = diagram
                .relations
                .iter()
                .position(|relation| relation.line == failure.line)
                .unwrap_or(0);
            for cutoff in first_blocked..diagram.relations.len().min(first_blocked + 12) {
                match cache.pass(diagram, boxes, sheet, Some(cutoff)) {
                    Ok(routes) if ranking::visibly_clear(&routes, boxes) => return Ok(routes),
                    Ok(_) => {}
                    Err(alternative) => failure = furthest_failure(failure, alternative),
                }
            }
            return Err(failure);
        }
    };
    let fully_ranked = cache.pass(diagram, boxes, sheet, Some(usize::MAX));
    let fully_ranked_failed = fully_ranked.is_err();
    if let Ok(ranked) = fully_ranked
        && ranking::no_worse_clearance(&ranked, &stable, boxes)
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
    // Ranking every edge can make a later edge impossible even though ranking
    // only the first few edges improves a needlessly long stable route.
    if fully_ranked_failed {
        for cutoff in 0..diagram.relations.len().min(3) {
            if let Ok(candidate) = cache.pass(diagram, boxes, sheet, Some(cutoff))
                && ranking::no_worse_clearance(&candidate, &stable, boxes)
                && diagram.relations.iter().enumerate().all(|(i, relation)| {
                    relation.from != relation.to || candidate[i].points == stable[i].points
                })
                && (if diagram.settings.allow_diagonal {
                    ranking::better_than(&candidate, &stable)
                } else {
                    ranking::better_than_orthogonal(&candidate, &stable)
                })
            {
                return Ok(candidate);
            }
        }
    }
    Ok(stable)
}

#[allow(clippy::too_many_arguments)] // Fixed search inputs plus its call-local prefix cache.
pub(super) fn route_in_order_mode(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    ranked_until: Option<usize>,
    order: &[usize],
    rescue: bool,
    seeds: Option<&std::collections::BTreeMap<usize, Vec<super::Point>>>,
    cache: &mut PassCache,
) -> Result<Vec<Route>, RouteError> {
    // The public entry point resolves visibility and notation exactly once.
    let mut routed = Vec::new();
    let mut prefix = Vec::new();
    for &index in order {
        let relation = &diagram.relations[index];
        let ranked = ranked_until.is_some_and(|cutoff| index <= cutoff);
        prefix.push((index, ranked));
        if let Some(previous) = cache.get(rescue, &prefix) {
            routed = previous;
            continue;
        }
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
        let terminal_gap = [relation.appearance.start, relation.appearance.end]
            .into_iter()
            .map(|symbol| super::terminal::minimum(symbol, width, sheet.theme.modern(), true))
            .fold(24.0_f64, f64::max);
        let mut options = if let Some(seeds) = seeds {
            seeds.get(&relation.line).cloned().into_iter().collect()
        } else if from == to {
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
        } else if cache.corridor {
            super::corridor::direct(diagram, index, boxes, &routed, terminal_gap)
        } else {
            let offset =
                parallel::offset(diagram, index, boxes, from, to, sheet).map_err(|message| {
                    RouteError {
                        line: relation.line,
                        message,
                    }
                })?;
            candidates(
                diagram,
                index,
                from,
                to,
                boxes,
                &routed,
                offset,
                ranked,
                terminal_gap,
            )
        };
        let obstacles = (!diagram.settings.allow_diagonal
            && matches!(
                diagram.settings.diagram_type,
                crate::syntax::DiagramType::Generic | crate::syntax::DiagramType::Package
            )
            && !diagram
                .nodes
                .iter()
                .any(|n| n.classes.iter().any(|c| c.starts_with("c4"))))
        .then(|| super::obstacles::Obstacles::new(boxes, from, to));
        if ranked && from != to {
            if let Some(check) = &obstacles {
                options.retain(|points| check.clear(points, &routed));
            }
            ranking::order(
                &mut options,
                &routed,
                boxes,
                from,
                to,
                !diagram.settings.allow_diagonal
                    && diagram.settings.diagram_type == crate::syntax::DiagramType::Package,
                !diagram.settings.allow_diagonal,
                obstacles
                    .as_ref()
                    .and_then(|_| super::fanout::preferred(diagram, index, boxes, from)),
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
        let extra: Box<dyn Iterator<Item = Vec<super::Point>> + '_> =
            if cache.corridor && from != to {
                Box::new(super::corridor::paths(
                    diagram,
                    index,
                    boxes,
                    &routed,
                    terminal_gap,
                    cache.crossing_free,
                ))
            } else {
                Box::new(
                    std::iter::once_with(|| {
                        if rescue {
                            super::rescue::candidates(diagram, index, boxes, &routed, terminal_gap)
                        } else {
                            Vec::new()
                        }
                    })
                    .flatten(),
                )
            };
        let contours = super::orthogonal::Contours::new(&boxes[from], &boxes[to]);
        for mut points in options.into_iter().chain(extra) {
            if cache.corridor && cache.crossing_free && super::corridor::crosses(&points, &routed) {
                continue;
            }
            // Seed replay and grid rescue must obey the same terminal directions
            // as ordinary candidates; collision clearance alone permits tangency.
            if matches!(
                diagram.settings.diagram_type,
                crate::syntax::DiagramType::Generic | crate::syntax::DiagramType::Package
            ) && !contours.normal(&points, diagram.settings.allow_diagonal)
            {
                continue;
            }
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
            if !super::terminal::clear(
                &points,
                relation,
                width,
                sheet.theme.modern(),
                &boxes[from],
                &boxes[to],
            ) {
                continue;
            }
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
            if !obstacles.as_ref().map_or_else(
                || route_is_clear(&points, boxes, &routed),
                |check| check.clear(&points, &routed),
            ) {
                continue;
            }
            let Some(plan) =
                bridges::plan(&points, boxes, &routed, from, to, width).or_else(|| {
                    rescue
                        .then(|| super::crossing_fallback::plan(&points, &routed, from, to, width))
                        .flatten()
                })
            else {
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
                        "routing search found no acceptable path around nodes, other connections, and crossings"
                    } else {
                        "routing search found no acceptable orthogonal path around nodes, other connections, and crossings"
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
            source_line: relation.line,
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
        cache.insert(rescue, &prefix, &routed);
    }
    // Bridge/gap plans are already attached to their routes; restore source
    // order before rendering or resolving relation attachments.
    let mut indexed: Vec<_> = order.iter().copied().zip(routed).collect();
    indexed.sort_by_key(|(index, _)| *index);
    let mut routed: Vec<_> = indexed.into_iter().map(|(_, route)| route).collect();
    attachments::route_attachments(diagram, boxes, sheet, &mut routed)?;
    Ok(routed)
}

#[cfg(test)]
#[path = "router_regression_tests.rs"]
mod regression_tests;

#[cfg(test)]
#[path = "router_tests.rs"]
mod tests;
