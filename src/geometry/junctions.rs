//! Small unlabeled junctions are not ordinary diamond-shaped text nodes.
use super::{BoxNode, NodeShape};
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{NodeDecl, NodeKind};
use crate::text::measure as text_width;

pub(super) fn measure(node: &NodeDecl, sheet: &StyleSheet) -> Result<BoxNode, String> {
    let element = Element::Node {
        id: &node.id,
        classes: &node.classes,
    };
    if [
        "shape",
        "width",
        "height",
        "min-width",
        "min-height",
        "icon",
    ]
    .iter()
    .any(|key| sheet.get(&element, key).is_some())
    {
        return Err("junction diamond is a fixed UML/GNE primitive, not a resizable node".into());
    }
    let fill = match sheet.get(&element, "fill").map(|d| &d.value) {
        Some(Value::Color(value)) => value.clone(),
        _ => "#ffffff".into(),
    };
    let color = match sheet.get(&element, "color").map(|d| &d.value) {
        Some(Value::Color(value)) => value.clone(),
        _ => "#17212e".into(),
    };
    let border_color = match sheet.get(&element, "border-color").map(|d| &d.value) {
        Some(Value::Color(value)) => value.clone(),
        _ => "#395373".into(),
    };
    let border_width = match sheet.get(&element, "border-width").map(|d| &d.value) {
        Some(Value::Number(value)) => *value,
        _ => 1.0,
    };
    debug_assert!(matches!(
        node.kind,
        NodeKind::Association | NodeKind::Junction | NodeKind::Lollipop
    ));
    let label = node.label.clone().unwrap_or_default();
    let base = if node.kind == NodeKind::Lollipop {
        20.0
    } else {
        24.0
    };
    let width = if label.is_empty() {
        base
    } else {
        base + 12.0 + text_width(&label, 14.0)?
    };
    let height = if label.is_empty() {
        base
    } else {
        base.max(14.0 * 1.25 + 8.0)
    };
    Ok(BoxNode {
        id: node.id.clone(),
        label,
        lines: Vec::new(),
        icons: Vec::new(),
        body: Vec::new(),
        body_styles: Vec::new(),
        ports: Vec::new(),
        port_start: 0.0,
        top_band: 0.0,
        bottom_band: 0.0,
        abstract_node: false,
        kind: node.kind,
        shape: if node.kind == NodeKind::Lollipop {
            NodeShape::Ellipse
        } else {
            NodeShape::Diamond
        },
        x: 0.0,
        y: 0.0,
        width,
        height,
        fixed_width: None,
        fixed_height: None,
        font_size: 14.0,
        padding: 0.0,
        fill,
        color,
        border_color,
        border_width,
        radius: 0.0,
        line: node.line,
    })
}
