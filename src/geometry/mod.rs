//! First geometry stage: measure rectangular nodes and satisfy hard rules.
//! Collision resolution is bounded. Complex routes may still fail explicitly.

mod constraints;
mod nodes;

pub use constraints::GeometryError;
pub use nodes::{BoxNode, NodeShape};

use crate::styles::StyleSheet;
use crate::syntax::Diagram;

pub fn place(diagram: &Diagram, styles: &StyleSheet) -> Result<Vec<BoxNode>, GeometryError> {
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
    constraints::place(diagram, &mut boxes, styles)?;
    Ok(boxes)
}
