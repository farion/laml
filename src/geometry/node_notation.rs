//! Measured classifier headings; semantic UML keywords are not decorative icons.
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{DiagramType, NodeDecl, NodeKind};

pub(super) fn color(
    sheet: &StyleSheet,
    element: &Element<'_>,
    property: &str,
    default: &str,
) -> String {
    match sheet
        .get(element, property)
        .map(|declaration| &declaration.value)
    {
        Some(Value::Color(color)) => color.clone(),
        _ => default.into(),
    }
}

pub(super) fn validate_ref_font(kind: NodeKind, size: f64) -> Result<(), String> {
    if kind == NodeKind::InteractionUse && size > 22.0 {
        return Err("interaction-use text cannot overlap its ref header".into());
    }
    Ok(())
}

pub(super) fn validate_ref_width(kind: NodeKind, width: f64) -> Result<(), String> {
    if kind == NodeKind::InteractionUse && width < 60.0 {
        return Err("interaction-use ref tab requires at least 60 units of width".into());
    }
    Ok(())
}

pub(super) fn ref_height(kind: NodeKind, natural: f64) -> f64 {
    if kind == NodeKind::InteractionUse {
        natural.max(66.0)
    } else {
        natural
    }
}

pub(super) fn validate_ref_height(
    kind: NodeKind,
    height: f64,
    required: f64,
) -> Result<(), String> {
    if kind == NodeKind::InteractionUse && height < required {
        return Err("interaction-use frame must fit its ref tab and text".into());
    }
    Ok(())
}

pub(super) fn validate_composite_border(
    kind: NodeKind,
    ty: DiagramType,
    styles: &StyleSheet,
    element: &Element<'_>,
) -> Result<(), String> {
    if ty == DiagramType::Composite
        && (matches!(styles.get(element,"border-color").map(|d| &d.value), Some(Value::Color(color)) if color=="none")
            || matches!(styles.get(element,"color").map(|d| &d.value), Some(Value::Color(color)) if color=="none")
            || super::nodes::number(styles, element, "border-width", 1.0) < 1.0)
    {
        return Err(format!(
            "composite {kind:?} frame must retain a visible outline"
        ));
    }
    Ok(())
}

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
    if kind == NodeKind::InteractionUse && radius != 0.0 {
        return Err("UML ref frame must have square corners".into());
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
