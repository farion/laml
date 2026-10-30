use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{DiagramType, NodeDecl, NodeKind, NodePart};
use crate::text::{ellipsize, measure};

#[derive(Clone, Debug)]
pub struct BoxNode {
    pub id: String,
    pub label: String,
    pub lines: Vec<String>,
    pub body: Vec<NodePart>,
    pub abstract_node: bool,
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_size: f64,
    pub padding: f64,
    pub fill: String,
    pub color: String,
    pub border_color: String,
    pub border_width: f64,
    pub radius: f64,
    pub line: usize,
}

fn number(sheet: &StyleSheet, element: &Element<'_>, property: &str, default: f64) -> f64 {
    match sheet
        .get(element, property)
        .map(|declaration| &declaration.value)
    {
        Some(Value::Number(number)) => *number,
        _ => default,
    }
}

fn color(sheet: &StyleSheet, element: &Element<'_>, property: &str, default: &str) -> String {
    match sheet
        .get(element, property)
        .map(|declaration| &declaration.value)
    {
        Some(Value::Color(color)) => color.clone(),
        _ => default.into(),
    }
}

pub fn measure_node(
    node: &NodeDecl,
    styles: &StyleSheet,
    diagram_type: DiagramType,
) -> Result<BoxNode, String> {
    let element = Element::Node {
        id: &node.id,
        classes: &node.classes,
    };
    let font_size = number(styles, &element, "font-size", 14.0);
    let padding = number(styles, &element, "padding", 12.0);
    let name = node.label.clone().unwrap_or_else(|| node.id.clone());
    let label = match node.kind {
        NodeKind::Interface => format!("«interface»\n{name}"),
        NodeKind::Enumeration => format!("«enumeration»\n{name}"),
        NodeKind::DataType => format!("«dataType»\n{name}"),
        _ => name,
    };
    let original: Vec<&str> = label.split('\n').collect();
    let line_height = font_size * 1.25;
    let fixed_width = styles.get(&element, "width").and_then(|d| match d.value {
        Value::Number(n) => Some(n),
        _ => None,
    });
    let fixed_height = styles.get(&element, "height").and_then(|d| match d.value {
        Value::Number(n) => Some(n),
        _ => None,
    });
    let explicit_min_width = styles
        .get(&element, "min-width")
        .and_then(|d| match d.value {
            Value::Number(n) => Some(n),
            _ => None,
        });
    let explicit_min_height = styles
        .get(&element, "min-height")
        .and_then(|d| match d.value {
            Value::Number(n) => Some(n),
            _ => None,
        });
    if let (Some(min), Some(fixed)) = (explicit_min_width, fixed_width)
        && min > fixed
    {
        return Err("styled width is less than styled min-width".into());
    }
    if let (Some(min), Some(fixed)) = (explicit_min_height, fixed_height)
        && min > fixed
    {
        return Err("styled height is less than styled min-height".into());
    }
    let mut longest = original
        .iter()
        .try_fold(0.0_f64, |max, text| -> Result<f64, String> {
            Ok(max.max(measure(text, font_size)?))
        })?;
    for part in &node.parts {
        if let NodePart::Member { text, .. } = part {
            longest = longest.max(measure(text, font_size)?);
        }
    }
    let width = fixed_width.unwrap_or_else(|| {
        explicit_min_width
            .unwrap_or(100.0)
            .max(longest + padding * 2.0)
    });
    let body_height: f64 = node
        .parts
        .iter()
        .map(|part| match part {
            NodePart::Member { .. } => line_height,
            NodePart::Separator => 8.0,
        })
        .sum();
    let natural_height = line_height * original.len() as f64
        + body_height
        + if node.parts.is_empty() { 0.0 } else { 4.0 }
        + padding * 2.0;
    let height =
        fixed_height.unwrap_or_else(|| explicit_min_height.unwrap_or(50.0).max(natural_height));
    let available_width = width - padding * 2.0;
    let available_height = height - padding * 2.0;
    if available_width < 0.0 || available_height < line_height {
        return Err("fixed node box cannot fit even one line of text and padding".into());
    }
    if diagram_type != DiagramType::Generic
        && (longest > available_width + 0.01 || natural_height > height + 0.01)
    {
        return Err("fixed UML node box would hide its name or members".into());
    }
    // A body is structural: it is not safe to silently omit addressable rows.
    if !node.parts.is_empty() && natural_height > height + 0.01 {
        return Err("fixed node box cannot fit its member rows".into());
    }
    let visible = original
        .len()
        .min((available_height / line_height).floor() as usize);
    let vertical_overflow = visible < original.len();
    let mut lines = Vec::new();
    for (index, text) in original[..visible].iter().enumerate() {
        let force = vertical_overflow && index + 1 == visible;
        lines.push(ellipsize(text, available_width, font_size, force)?);
    }
    let mut body = node.parts.clone();
    if diagram_type == DiagramType::Generic {
        for part in &mut body {
            if let NodePart::Member { text, .. } = part {
                *text = ellipsize(text, available_width, font_size, false)?;
            }
        }
    }
    Ok(BoxNode {
        id: node.id.clone(),
        label,
        lines,
        body,
        abstract_node: node.abstract_node,
        kind: node.kind,
        x: 0.0,
        y: 0.0,
        width,
        height,
        font_size,
        padding,
        fill: color(styles, &element, "fill", "#eff4fa"),
        color: color(styles, &element, "color", "#17212e"),
        border_color: color(styles, &element, "border-color", "#395373"),
        border_width: number(styles, &element, "border-width", 1.0),
        radius: number(styles, &element, "corner-radius", 0.0),
        line: node.line,
    })
}
