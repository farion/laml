//! BPMN drawing notation and rendering. No BPMN-XML import or process simulation.
mod geometry;
mod parser;
mod render;
mod style;
pub use parser::{Document, Edge, EdgeKind, Kind, Node, Pool, is_bpmn_source, parse};
pub use render::{render, render_with_styles};
pub use style::validate_styles;
pub fn validate_geometry(doc: &Document) -> Result<(), String> {
    geometry::place(doc).map(|_| ())
}
