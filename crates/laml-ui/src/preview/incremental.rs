//! Narrow fast path for relation notation changes; all other edits use a full build.
use super::Preview;
use laml::{geometry::BoxNode, route::Route, styles::StyleSheet, syntax::Diagram};

pub(super) fn geometry(
    diagram: &Diagram,
    sheet: &StyleSheet,
    previous: &Preview,
) -> Option<(Vec<BoxNode>, Vec<Route>)> {
    if !notation_only(diagram, &previous.diagram) || !diagram.attachments.is_empty() {
        return None;
    }
    // Placement is cheap relative to global routing. Verify current-source
    // geometry so automatic direction preferences and spacing cannot become
    // hidden layout history. Expanded previous layouts fall back to a full build.
    let boxes = laml::geometry::place(diagram, sheet).ok()?;
    if !same_geometry(&boxes, &previous.nodes) {
        return None;
    }
    let routes = laml::route::replay(diagram, &boxes, sheet, &previous.routes, &previous.diagram)
        .or_else(|_| laml::route::route(diagram, &boxes, sheet))
        .ok()?;
    Some((boxes, routes))
}

fn notation_only(diagram: &Diagram, old: &Diagram) -> bool {
    if diagram.relations.len() != old.relations.len() {
        return false;
    }
    let mut normalized = diagram.clone();
    for (new, old) in normalized.relations.iter_mut().zip(&old.relations) {
        if new.from != old.from || new.to != old.to {
            if new.from != old.to || new.to != old.from {
                return false;
            }
            std::mem::swap(&mut new.from, &mut new.to);
            std::mem::swap(&mut new.source_end, &mut new.target_end);
        }
        new.appearance = old.appearance;
        new.kind = old.kind;
    }
    normalized == *old
}

fn same_geometry(a: &[BoxNode], b: &[BoxNode]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.id == b.id
                && a.x == b.x
                && a.y == b.y
                && a.width == b.width
                && a.height == b.height
                && a.ports.len() == b.ports.len()
                && a.ports
                    .iter()
                    .zip(&b.ports)
                    .all(|(a, b)| a.id == b.id && a.x == b.x && a.y == b.y)
        })
}
