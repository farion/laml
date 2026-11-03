//! BPMN drawing notation and rendering. No BPMN-XML import or process simulation.
mod geometry;
mod model;
mod parser;
mod render;
mod style;
mod type_sizes;
pub use model::{Document, Edge, EdgeKind, FlowPortRule, Kind, LabelOrientation, Node, Pool, Side};
pub use parser::{is_bpmn_source, parse};
pub use render::{render, render_with_styles};
pub use style::validate_styles;
pub fn validate_geometry(doc: &Document) -> Result<(), String> {
    let geom = geometry::place(doc)?;
    render::validate_attachments(doc, &geom)
}
