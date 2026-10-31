//! Measured UML actor silhouette; its caption and connection hand have real bounds.
use super::{BoxNode, nodes};
use crate::styles::{Element, StyleSheet};
use crate::syntax::{DiagramType, NodeDecl};
pub(super) fn actor(node: &NodeDecl, sheet: &StyleSheet) -> Result<BoxNode, String> {
    if sheet
        .get(
            &Element::Node {
                id: &node.id,
                classes: &node.classes,
            },
            "fill",
        )
        .is_some()
    {
        return Err("actor stick figure has no fillable box".into());
    }
    let mut shape = nodes::measure_node(node, sheet, DiagramType::UseCase, &[])?;
    if shape.border_color == "none" || shape.border_width == 0.0 {
        return Err("actor stick figure requires a visible outline".into());
    }
    let required = 94.0 + shape.font_size * 1.25 * shape.lines.len().saturating_sub(1) as f64;
    shape.height = shape.height.max(required);
    if let Some(fixed) = shape.fixed_height
        && fixed < required
    {
        return Err("fixed actor height cannot fit stick figure and caption".into());
    }
    Ok(shape)
}
