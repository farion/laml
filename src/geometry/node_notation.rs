//! Measured classifier headings; semantic UML keywords are not decorative icons.
use crate::styles::{Element, StyleSheet};
use crate::syntax::{DiagramType, NodeDecl, NodeKind};

pub(super) fn corner_radius(
    kind: NodeKind,
    styles: &StyleSheet,
    element: &Element<'_>,
) -> Result<f64, String> {
    let radius = super::nodes::number(
        styles,
        element,
        "corner-radius",
        if kind == NodeKind::State { 14.0 } else { 0.0 },
    );
    if kind == NodeKind::State && !(8.0..=24.0).contains(&radius) {
        return Err("UML state requires visibly rounded corners (radius 8..24)".into());
    }
    Ok(radius)
}

pub(super) fn header(node: &NodeDecl, name: String, diagram_type: DiagramType) -> String {
    // The bundled regular face need not support italic synthesis in every exporter.
    // UML permits an explicit abstract annotation as a font-independent fallback.
    let name = if matches!(diagram_type, DiagramType::Class | DiagramType::Component)
        && node.abstract_node
    {
        format!("{name} {{abstract}}")
    } else {
        name
    };
    let label = match node.kind {
        NodeKind::Component => format!("«component»\n{name}"),
        NodeKind::Device => format!("«device»\n{name}"),
        NodeKind::ExecutionEnvironment => format!("«executionEnvironment»\n{name}"),
        NodeKind::Artifact => format!("«artifact»\n{name}"),
        NodeKind::Interface => format!("«interface»\n{name}"),
        NodeKind::Enumeration => format!("«enumeration»\n{name}"),
        NodeKind::DataType => format!("«dataType»\n{name}"),
        _ => name,
    };
    if node.stereotypes.is_empty() {
        label
    } else {
        format!(
            "{}\n{label}",
            node.stereotypes
                .iter()
                .map(|name| format!("«{name}»"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}
