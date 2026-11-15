//! GNE property inventory and effective compiler-backed inspector values.
use laml::{
    geometry::{BoxNode, NodeShape},
    styles::{Element, StyleSheet, Value},
    syntax::{Diagram, NodeKind},
};
use std::collections::BTreeMap;

pub const NODE_PROPERTIES: &[&str] = &[
    "fill",
    "border-color",
    "border-width",
    "color",
    "base-color",
    "font-size",
    "padding",
    "width",
    "height",
    "min-width",
    "min-height",
    "corner-radius",
    "shape",
    "icon",
    "symbol-rendering",
    "icon-set",
    "display",
];
pub const RELATION_PROPERTIES: &[&str] = &["color", "width", "font-size", "base-color", "display"];
pub type RelationValues = BTreeMap<usize, BTreeMap<String, Property>>;
pub const PORT_PROPERTIES: &[&str] = &["fill", "border-color", "color", "base-color", "display"];
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Property {
    /// An absent value means there is no rendered/winning value, not an editor default.
    pub value: Option<String>,
    pub editable: bool,
    pub reason: Option<String>,
}
pub type Values = BTreeMap<String, BTreeMap<String, Property>>;

pub fn node_values(diagram: &Diagram, boxes: &[BoxNode], sheet: &StyleSheet) -> Values {
    diagram.nodes.iter().map(|node| {
        let scoped = sheet.for_node(diagram, &node.id);
        let element = Element::Node { id: &node.id, classes: &node.classes };
        let rendered = boxes.iter().find(|b| b.id == node.id);
        let properties = NODE_PROPERTIES.iter().map(|name| {
            let declared = scoped.get(&element, name).map(|d| string(&d.value));
            let value = rendered.and_then(|b| node_value(b, name)).or_else(|| declared.clone()).or_else(|| match *name {
                "display" => Some("block".into()),
                "symbol-rendering" => Some("text".into()),
                "base-color" if sheet.theme.modern() => Some(sheet.theme.default_base(if node.kind == NodeKind::Generic && rendered.is_some_and(|b| b.shape == NodeShape::Note) { NodeKind::Note } else { node.kind }).into()),
                _ => None,
            });
            let reason = match *name {
                "icon" if node.kind != NodeKind::Generic => Some("Additional icons apply only to generic nodes, not notes or structural elements."),
                "shape" if matches!(node.kind,NodeKind::Package|NodeKind::Junction) => Some("This structural element requires its notation-defined outline."),
                _ => None,
            };
            ((*name).into(), Property { value, editable: reason.is_none(), reason: reason.map(str::to_string) })
        }).collect();
        (node.id.clone(), properties)
    }).collect()
}
fn node_value(node: &BoxNode, name: &str) -> Option<String> {
    Some(match name {
        "fill" => node.fill.clone(),
        "border-color" => node.border_color.clone(),
        "color" => node.color.clone(),
        "border-width" => node.border_width.to_string(),
        "font-size" => node.font_size.to_string(),
        "padding" => node.padding.to_string(),
        "width" => node.width.to_string(),
        "height" => node.height.to_string(),
        "corner-radius" => node.radius.to_string(),
        "shape" => match node.shape {
            NodeShape::Rectangle => "rectangle",
            NodeShape::Ellipse => "ellipse",
            NodeShape::Diamond => "diamond",
            NodeShape::Note => "note",
        }
        .into(),
        _ => return None,
    })
}
pub fn port_values(diagram: &Diagram, boxes: &[BoxNode], sheet: &StyleSheet) -> Values {
    let mut result = Values::new();
    for node in &diagram.nodes {
        let scoped = sheet
            .for_node(diagram, &node.id)
            .with_node_ancestor(&node.id, &node.classes);
        for port in &node.ports {
            let id = port_id(&node.id, port);
            let element = Element::Port {
                id: &id,
                classes: &port.classes,
            };
            let rendered = boxes.iter().flat_map(|b| &b.ports).find(|p| p.id == id);
            let properties = PORT_PROPERTIES
                .iter()
                .map(|name| {
                    let value = rendered
                        .and_then(|p| match *name {
                            "fill" => Some(p.fill.clone()),
                            "border-color" => Some(p.border_color.clone()),
                            "color" => Some(p.color.clone()),
                            _ => None,
                        })
                        .or_else(|| scoped.get(&element, name).map(|d| string(&d.value)))
                        .or_else(|| (*name == "display").then(|| "block".into()));
                    (
                        (*name).into(),
                        Property {
                            value,
                            editable: true,
                            reason: None,
                        },
                    )
                })
                .collect();
            result.insert(id, properties);
        }
    }
    result
}
pub fn port_id(owner: &str, port: &laml::syntax::PortDecl) -> String {
    if port.explicit {
        port.id.clone()
    } else {
        format!("{owner}::{}", port.id)
    }
}
pub fn string(value: &Value) -> String {
    match value {
        Value::Number(n) => n.to_string(),
        Value::Color(s) | Value::Keyword(s) => s.clone(),
    }
}

/// Compare with compiler-normalized style values, not raw UI spelling (RGB, quotes, case).
pub fn matches(actual: Option<&str>, name: &str, requested: &str) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    let requested = requested.trim();
    if name == "icon" {
        return actual == requested;
    }
    if let (Ok(a), Ok(b)) = (actual.parse::<f64>(), requested.parse::<f64>()) {
        return (a - b).abs() < 0.0001;
    }
    actual.eq_ignore_ascii_case(requested)
}

pub fn source_value(name: &str, value: &str) -> String {
    if name == "icon" {
        crate::relation_edit::quote(value)
    } else {
        value.trim().into()
    }
}

pub fn relation_values(
    diagram: &Diagram,
    routes: &[laml::route::Route],
    sheet: &StyleSheet,
) -> RelationValues {
    diagram
        .relations
        .iter()
        .map(|r| {
            let element = Element::Relation {
                name: r.name.as_deref(),
                classes: &r.classes,
            };
            let route = routes.iter().find(|route| route.source_line == r.line);
            let values = RELATION_PROPERTIES
                .iter()
                .map(|name| {
                    let value = route
                        .and_then(|r| match *name {
                            "color" => Some(r.color.clone()),
                            "width" => Some(r.width.to_string()),
                            "font-size" => Some(r.font_size.to_string()),
                            _ => None,
                        })
                        .or_else(|| sheet.get(&element, name).map(|d| string(&d.value)))
                        .or_else(|| (*name == "display").then(|| "block".into()));
                    (
                        (*name).into(),
                        Property {
                            value,
                            editable: true,
                            reason: None,
                        },
                    )
                })
                .collect();
            (r.line, values)
        })
        .collect()
}

#[cfg(test)]
mod tests;
