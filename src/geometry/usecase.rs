//! Measured UML actor silhouette; its caption and connection hand have real bounds.
use super::{BoxNode, nodes};
use crate::styles::{Element, StyleSheet};
use crate::syntax::{DiagramType, NodeDecl};

// The silhouette is independent of caption width. Each arm is 26 long,
// each leg ~39; the legs open to a 75-degree included angle.
// Arms sit about two thirds of the way UP from the feet, just below the head.
pub(crate) const ARM_HALF: f64 = 26.0;
pub(crate) const ARM_Y: f64 = 34.0;
pub(crate) const HIP_Y: f64 = 60.0;
pub(crate) const FOOT_Y: f64 = 91.0;
pub(crate) const FOOT_OFFSET: f64 = 24.0;
pub(crate) const HEAD_CENTER_Y: f64 = 15.0;
pub(crate) const HEAD_RADIUS: f64 = 12.5;
// Invisible docking contour around the figure, not around its caption.
pub(crate) const FIGURE_CENTER_Y: f64 = 47.5;
pub(crate) const DOCK_RX: f64 = 50.0;
pub(crate) const DOCK_RY: f64 = 50.0;

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
    if shape.width < DOCK_RX * 2.0 {
        return Err("actor width cannot fit the figure's connection contour".into());
    }
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
