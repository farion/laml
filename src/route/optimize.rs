//! Compare complete, feasible route sets, including alternative allocation orders.
use super::{Route, RouteError, pass_cache::PassCache, ranking, routing_order};
use crate::{geometry::BoxNode, styles::StyleSheet, syntax::Diagram};

pub(super) fn route(
    diagram: &Diagram,
    boxes: &[BoxNode],
    sheet: &StyleSheet,
    stable: Result<Vec<Route>, RouteError>,
    cache: &mut PassCache,
) -> Result<Vec<Route>, RouteError> {
    let (mut best, mut failure) = match stable {
        Ok(routes) => (Some(routes), None),
        Err(error) => (None, Some(error)),
    };
    let original: Vec<_> = (0..diagram.relations.len()).collect();
    let mut orders = vec![original];
    // Self-loop lane ranks and their notation remain source-order owned.
    if diagram.relations.iter().all(|r| r.from != r.to) {
        orders.extend(routing_order::alternatives(diagram, boxes));
    }
    for order in orders {
        match cache.run(diagram, boxes, sheet, Some(usize::MAX), &order, false) {
            Ok(routes) => {
                if best.as_ref().is_none_or(|old| {
                    ranking::no_worse_clearance(&routes, old, boxes)
                        && if diagram.settings.diagram_type == crate::syntax::DiagramType::Package {
                            ranking::better_than_packages(&routes, old)
                        } else {
                            ranking::better_than_orthogonal(&routes, old)
                        }
                }) {
                    best = Some(routes);
                }
            }
            Err(error) => {
                if failure.as_ref().is_none_or(|old| error.line > old.line) {
                    failure = Some(error);
                }
            }
        }
    }
    // Mixing ranked prefixes with stable suffixes can rescue a crowded later
    // label/bridge without giving up the improved earlier connection.
    if best.is_none() {
        let order: Vec<_> = (0..diagram.relations.len()).collect();
        for cutoff in 0..diagram.relations.len().min(12) {
            if let Ok(routes) = cache.run(diagram, boxes, sheet, Some(cutoff), &order, false) {
                best = Some(routes);
                break;
            }
        }
    }
    best.ok_or_else(|| failure.expect("at least one route pass failed"))
}
