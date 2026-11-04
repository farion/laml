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

pub(super) fn palette(
    kind: NodeKind,
    sheet: &StyleSheet,
    element: &Element<'_>,
) -> Option<crate::theme::Palette> {
    let base = match sheet.get(element, "base-color").map(|d| &d.value) {
        Some(Value::Color(value)) => Some(value.as_str()),
        _ => None,
    };
    let kind = if kind == NodeKind::Generic
        && matches!(sheet.get(element, "shape").map(|d| &d.value), Some(Value::Keyword(shape)) if shape == "note")
    {
        NodeKind::Note
    } else {
        kind
    };
    (sheet.theme.modern() || base.is_some()).then(|| {
        sheet
            .theme
            .colors(base.unwrap_or(sheet.theme.default_base(kind)))
    })
}

pub(super) fn text_paint(
    sheet: &StyleSheet,
    element: &Element<'_>,
    palette: Option<&crate::theme::Palette>,
) -> String {
    color(
        sheet,
        element,
        "color",
        palette.map_or(
            if sheet.theme.dark() {
                "#eef2f7"
            } else {
                "#17212e"
            },
            |p| p.text.as_str(),
        ),
    )
}

pub(super) fn fill_paint(
    kind: NodeKind,
    sheet: &StyleSheet,
    element: &Element<'_>,
    palette: Option<&crate::theme::Palette>,
) -> String {
    color(
        sheet,
        element,
        "fill",
        palette.map_or(
            if sheet.theme.dark() {
                "#181f2a"
            } else if kind == NodeKind::Note {
                "#fff8d6"
            } else {
                "#eff4fa"
            },
            |p| p.fill.as_str(),
        ),
    )
}

pub(super) fn accent_paint(palette: Option<&crate::theme::Palette>) -> String {
    palette
        .map_or("#395373", |paint| paint.border.as_str())
        .into()
}

pub(super) fn shape_border_paint(
    note: bool,
    sheet: &StyleSheet,
    element: &Element<'_>,
    palette: Option<&crate::theme::Palette>,
) -> String {
    match palette {
        Some(paint) if note && sheet.get(element, "border-color").is_none() => {
            paint.divider.clone()
        }
        _ => border_paint(sheet, element, palette),
    }
}

pub(super) fn border_paint(
    sheet: &StyleSheet,
    element: &Element<'_>,
    palette: Option<&crate::theme::Palette>,
) -> String {
    color(
        sheet,
        element,
        "border-color",
        palette.map_or(
            if sheet.theme.dark() {
                "#9eafc3"
            } else {
                "#395373"
            },
            |p| p.border.as_str(),
        ),
    )
}

pub(super) struct PortPaint {
    pub text: String,
    pub fill: String,
    pub border: String,
    pub explicit_fill: bool,
}

pub(super) fn port_paint(
    sheet: &StyleSheet,
    element: &Element<'_>,
    parent: Option<&crate::theme::Palette>,
    parent_text: &str,
) -> PortPaint {
    let base = match sheet.get(element, "base-color").map(|d| &d.value) {
        Some(Value::Color(value)) => Some(value.as_str()),
        _ => None,
    };
    let palette = base.map(|base| sheet.theme.colors(base));
    let paint = palette.as_ref().or(parent);
    PortPaint {
        text: color(
            sheet,
            element,
            "color",
            paint.map_or(parent_text, |p| p.text.as_str()),
        ),
        fill: color(
            sheet,
            element,
            "fill",
            paint.map_or(
                if sheet.theme.dark() {
                    "#181f2a"
                } else {
                    "#eff4fa"
                },
                |p| p.fill.as_str(),
            ),
        ),
        border: color(
            sheet,
            element,
            "border-color",
            paint.map_or(
                if sheet.theme.dark() {
                    "#9eafc3"
                } else {
                    "#395373"
                },
                |p| p.border.as_str(),
            ),
        ),
        explicit_fill: sheet.get(element, "fill").is_some(),
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
    if matches!(ty, DiagramType::Composite | DiagramType::Profile)
        && (matches!(styles.get(element,"border-color").map(|d| &d.value), Some(Value::Color(color)) if color=="none")
            || matches!(styles.get(element,"color").map(|d| &d.value), Some(Value::Color(color)) if color=="none")
            || super::nodes::number(styles, element, "border-width", 1.0) < 1.0)
    {
        return Err(format!(
            "UML {kind:?} frame must retain a visible outline and text"
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
        } else if styles.theme.modern()
            && !matches!(
                kind,
                NodeKind::InteractionUse
                    | NodeKind::ActivityObject
                    | NodeKind::UseCase
                    | NodeKind::UseCaseActor
                    | NodeKind::DeploymentNode
                    | NodeKind::Device
                    | NodeKind::ExecutionEnvironment
            )
        {
            8.0
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
    if matches!(kind, NodeKind::UseCase | NodeKind::UseCaseActor) && radius != 0.0 {
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
