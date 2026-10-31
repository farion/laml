//! Measured classifier headings; semantic UML keywords are not decorative icons.
use crate::syntax::{DiagramType, NodeDecl, NodeKind};

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
