//! First geometry stage: measure rectangular nodes and satisfy hard rules.
//! Collision resolution is bounded. Complex routes may still fail explicitly.

mod constraints;
mod nodes;
mod visibility;

pub use constraints::GeometryError;
pub use nodes::{BoxNode, NodeShape};
pub(crate) use visibility::filtered as visible_diagram;

use crate::styles::StyleSheet;
use crate::syntax::Diagram;

pub fn place(diagram: &Diagram, styles: &StyleSheet) -> Result<Vec<BoxNode>, GeometryError> {
    let visible = visible_diagram(diagram, styles)
        .map_err(|(line, message)| GeometryError { line, message })?;
    let diagram = &visible;
    let mut boxes: Vec<_> = diagram
        .nodes
        .iter()
        .map(|node| {
            nodes::measure_node(node, styles, diagram.settings.diagram_type).map_err(|message| {
                GeometryError {
                    line: node.line,
                    message,
                }
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
    constraints::place(diagram, &mut boxes, styles)?;
    Ok(boxes)
}
