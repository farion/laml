//! First geometry stage: measure rectangular nodes and satisfy hard rules.
//! Collision resolution is bounded. Complex routes may still fail explicitly.

mod constraints;
mod containers;
mod nodes;
mod ports;
mod visibility;

pub use constraints::GeometryError;
pub use nodes::{BoxNode, NodeShape};
pub(crate) use visibility::filtered as visible_diagram;

use crate::styles::StyleSheet;
use crate::syntax::{Diagram, NodeDecl};

pub(crate) fn scoped_styles(diagram: &Diagram, node: &NodeDecl, sheet: &StyleSheet) -> StyleSheet {
    let mut parents = Vec::new();
    let mut current = node.parent.as_deref();
    while let Some(id) = current {
        let Some(parent) = diagram.nodes.iter().find(|candidate| candidate.id == id) else {
            break;
        };
        parents.push(parent);
        current = parent.parent.as_deref();
    }
    let mut scoped = sheet.clone();
    for parent in parents.into_iter().rev() {
        scoped = scoped.with_ancestor(&parent.id, &parent.classes);
    }
    scoped
}

pub fn place(diagram: &Diagram, styles: &StyleSheet) -> Result<Vec<BoxNode>, GeometryError> {
    let visible = visible_diagram(diagram, styles)
        .map_err(|(line, message)| GeometryError { line, message })?;
    let diagram = &visible;
    let mut boxes: Vec<_> = diagram
        .nodes
        .iter()
        .map(|node| {
            nodes::measure_node(
                node,
                &scoped_styles(diagram, node, styles),
                diagram.settings.diagram_type,
                &diagram.port_layout,
            )
            .map_err(|message| GeometryError {
                line: node.line,
                message,
            })
        })
        .collect::<Result<_, _>>()?;
    for relation in &diagram.relations {
        for endpoint in [&relation.from, &relation.to] {
            if !boxes.iter().any(|node| node.has_endpoint(endpoint)) {
                return Err(GeometryError {
                    line: relation.line,
                    message: format!("visible relation targets hidden member '{endpoint}'"),
                });
            }
        }
    }
    for rule in &diagram.port_layout {
        if !boxes
            .iter()
            .any(|node| node.ports.iter().any(|port| port.id == rule.id))
        {
            return Err(GeometryError {
                line: rule.line,
                message: format!("mandatory port layout targets hidden port '{}'", rule.id),
            });
        }
    }
    containers::place(diagram, &mut boxes, styles)?;
    ports::position(diagram, &mut boxes);
    Ok(boxes)
}
