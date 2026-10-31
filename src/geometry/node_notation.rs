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
        if matches!(kind, NodeKind::State | NodeKind::ActivityAction) {
            14.0
        } else {
            0.0
        },
    );
    if matches!(kind, NodeKind::State | NodeKind::ActivityAction) && !(8.0..=24.0).contains(&radius)
    {
        return Err("UML state/action requires visibly rounded corners (radius 8..24)".into());
    }
    if kind == NodeKind::ActivityObject && radius != 0.0 {
        return Err("UML object node must remain rectangular".into());
    }
    if matches!(
        kind,
        NodeKind::UseCase | NodeKind::UseCaseActor | NodeKind::UseCaseSystem
    ) && radius != 0.0
    {
        return Err("UML use-case notation does not allow a custom corner radius".into());
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
