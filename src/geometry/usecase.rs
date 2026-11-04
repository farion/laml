//! Measured UML actor silhouette; its caption and connection hand have real bounds.
use super::{BoxNode, nodes};
use crate::styles::{Element, StyleSheet};
use crate::syntax::{DiagramType, NodeDecl};

// Actor proportions are independent of the width of its caption. The arms are
// 44 wide (22 per side), each leg is ~33 long and leans 15 degrees from vertical.
pub(crate) const ARM_HALF: f64 = 22.0;
pub(crate) const ARM_Y: f64 = 64.0;
pub(crate) const HIP_Y: f64 = 68.0;
pub(crate) const FOOT_Y: f64 = 100.0;
pub(crate) const FOOT_OFFSET: f64 = 8.5;

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
    let required = FOOT_Y + 10.0 + shape.font_size * 1.25 * shape.lines.len() as f64;
    shape.height = shape.height.max(required);
    if let Some(fixed) = shape.fixed_height
        && fixed < required
    {
        return Err("fixed actor height cannot fit stick figure and caption".into());
    }
    Ok(shape)
}
