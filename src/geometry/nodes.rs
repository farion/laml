use super::ports::BoxPort;
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{DiagramType, NodeDecl, NodeKind, NodePart, PortLayoutRule, PortSide};
use crate::text::{ellipsize, measure, measure_italic};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,
    Ellipse,
    Diamond,
    Note,
}

#[derive(Clone, Debug)]
pub struct RowStyle {
    pub font_size: f64,
    pub color: String,
    pub icon: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BoxNode {
    pub id: String,
    pub label: String,
    pub template: Option<String>,
    pub lines: Vec<String>,
    pub icons: Vec<String>,
    pub body: Vec<NodePart>,
    pub body_styles: Vec<RowStyle>,
    pub ports: Vec<BoxPort>,
    pub port_start: f64,
    pub top_band: f64,
    pub bottom_band: f64,
    pub abstract_node: bool,
    pub kind: NodeKind,
    pub shape: NodeShape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fixed_width: Option<f64>,
    pub fixed_height: Option<f64>,
    pub font_size: f64,
    pub padding: f64,
    pub fill: String,
    pub color: String,
    pub border_color: String,
    pub border_width: f64,
    pub radius: f64,
    pub line: usize,
}

impl BoxNode {
    pub fn contains_package_content(&self, x: f64, y: f64, width: f64, height: f64) -> bool {
        self.kind == NodeKind::Package
            && x >= self.x + 1.0
            && x + width <= self.x + self.width - 1.0
            && y >= self.y + self.padding + self.font_size * 1.25 * self.lines.len() as f64 + 4.0
            && y + height <= self.y + self.height - 1.0
    }

    pub fn matches_member(&self, part: &NodePart, target: &str) -> bool {
        match part {
            NodePart::Member { id: Some(id), .. } => id == target,
            NodePart::Member {
                id: None,
                default_id: Some(name),
                ..
            } => target.strip_prefix(&format!("{}::", self.id)) == Some(name.as_str()),
            _ => false,
        }
    }

    pub fn has_endpoint(&self, target: &str) -> bool {
        self.id == target
            || self
                .body
                .iter()
                .any(|part| self.matches_member(part, target))
            || self.ports.iter().any(|port| port.id == target)
    }
}

pub(super) fn number(
    sheet: &StyleSheet,
    element: &Element<'_>,
    property: &str,
    default: f64,
) -> f64 {
    match sheet
        .get(element, property)
        .map(|declaration| &declaration.value)
    {
        Some(Value::Number(number)) => *number,
        _ => default,
    }
}

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

pub fn measure_node(
    node: &NodeDecl,
    styles: &StyleSheet,
    diagram_type: DiagramType,
    port_rules: &[PortLayoutRule],
) -> Result<BoxNode, String> {
    let element = Element::Node {
        id: &node.id,
        classes: &node.classes,
    };
    let shape = match styles.get(&element, "shape").map(|d| &d.value) {
        Some(Value::Keyword(value)) => match value.as_str() {
            "rectangle" => NodeShape::Rectangle,
            "ellipse" => NodeShape::Ellipse,
            "diamond" => NodeShape::Diamond,
            "note" => NodeShape::Note,
            _ => return Err("unknown node shape".into()),
        },
        _ if node.kind == NodeKind::Note => NodeShape::Note,
        _ => NodeShape::Rectangle,
    };
    if diagram_type != DiagramType::Generic
        && shape
            != if node.kind == NodeKind::Note {
                NodeShape::Note
            } else {
                NodeShape::Rectangle
            }
    {
        return Err("UML node form cannot be replaced by an LSS shape".into());
    }
    let font_size = number(styles, &element, "font-size", 14.0);
    let text_color = color(styles, &element, "color", "#17212e");
    let icons_requested = matches!(styles.get(&element, "symbol-rendering").map(|d| &d.value),
        Some(Value::Keyword(mode)) if mode == "icons");
    if icons_requested
        && !matches!(styles.get(&element, "icon-set").map(|d| &d.value),
        Some(Value::Keyword(set)) if set == "phosphor")
    {
        return Err("symbol-rendering: icons requires icon-set: phosphor".into());
    }
    let mut icons = Vec::new();
    let visibility_icon = if icons_requested {
        node.visibility.and_then(crate::svg_icons::visibility)
    } else {
        None
    };
    if let Some(icon) = visibility_icon {
        icons.push(icon.into());
    }
    if node.kind == NodeKind::Package && !icons.is_empty() {
        return Err("package visibility icons need a notation-safe tab layout".into());
    }
    if let Some(Value::Keyword(icon)) = styles.get(&element, "icon").map(|d| &d.value) {
        if diagram_type != DiagramType::Generic || node.kind != NodeKind::Generic {
            return Err("additional icons are not valid on this UML or structural node".into());
        }
        icons.push(icon.clone());
    }
    let icon_space = if icons.is_empty() {
        0.0
    } else {
        (font_size + 4.0) * icons.len() as f64
    };
    let (mut body, body_styles) =
        super::members::visible_body(node, styles, font_size, &text_color)?;
    let scoped = styles.with_ancestor(&node.id, &node.classes);
    let ports: Vec<_> = node.ports.iter().filter_map(|port| {
        let id = if port.explicit { port.id.clone() } else { format!("{}::{}", node.id, port.id) };
        let element = Element::Port { id: &id, classes: &port.classes };
        if matches!(scoped.get(&element, "display").map(|d| &d.value), Some(Value::Keyword(mode)) if mode == "none") {
            return None;
        }
        Some(BoxPort {
            label: port.label.clone(),
            side: port_rules.iter().find(|rule| rule.id == id).map_or(PortSide::Right, |rule| rule.side),
            x: 0.0,
            y: 0.0,
            color: color(&scoped, &element, "color", &text_color),
            fill: color(&scoped, &element, "fill", "#eff4fa"),
            border_color: color(&scoped, &element, "border-color", "#395373"),
            id,
        })
    }).collect();
    if node.kind == NodeKind::Package && shape != NodeShape::Rectangle {
        return Err("package container requires a rectangular frame".into());
    }
    if matches!(shape, NodeShape::Diamond | NodeShape::Ellipse) && !body.is_empty() {
        return Err(
            "nonrectangular GNE node with compartments needs supported inset geometry".into(),
        );
    }
    let padding = number(styles, &element, "padding", 12.0);
    let mut name = node.label.clone().unwrap_or_else(|| node.id.clone());
    let template = if diagram_type == DiagramType::Class
        && node.kind == NodeKind::Class
        && !node.templates.is_empty()
    {
        if !ports.is_empty() {
            return Err(
                "UML template corner and boundary ports need separate collision geometry".into(),
            );
        }
        let suffix = format!("<{}>", node.templates.join(", "));
        name = name
            .strip_suffix(&suffix)
            .ok_or("template heading does not match parameters")?
            .into();
        Some(node.templates.join(", "))
    } else {
        None
    };
    let name = match node.visibility {
        Some(marker) if visibility_icon.is_none() => format!("{marker}{name}"),
        _ => name,
    };
    // The bundled regular face need not support italic synthesis in every exporter.
    // UML permits an explicit abstract annotation as a font-independent fallback.
    let name = if diagram_type == DiagramType::Class && node.abstract_node {
        format!("{name} {{abstract}}")
    } else {
        name
    };
    let label = match node.kind {
        NodeKind::Interface => format!("«interface»\n{name}"),
        NodeKind::Enumeration => format!("«enumeration»\n{name}"),
        NodeKind::DataType => format!("«dataType»\n{name}"),
        _ => name,
    };
    let label = if node.stereotypes.is_empty() {
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
    if let Some(last) = original.last() {
        let width = if node.abstract_node {
            measure_italic(last, font_size)?
        } else {
            measure(last, font_size)?
        };
        longest = longest.max(width + icon_space);
    }
    if node.kind == NodeKind::Package {
        longest += 12.0; // leave room for the folder-tab shoulder
    }
    for (part, row) in body.iter().zip(&body_styles) {
        if let NodePart::Member {
            text,
            abstract_member,
            ..
        } = part
        {
            let shown = super::members::visible_text(text, row);
            longest = longest.max(
                if *abstract_member {
                    measure_italic(shown, row.font_size)?
                } else {
                    measure(shown, row.font_size)?
                } + if row.icon.is_some() {
                    row.font_size + 4.0
                } else {
                    0.0
                },
            );
        }
    }
    for port in &ports {
        longest = longest.max(measure(&port.label, font_size)? + 16.0);
    }
    if let Some(parameters) = &template {
        longest = longest.max(measure(parameters, font_size)? + 16.0);
    }
    for side in [PortSide::Top, PortSide::Bottom] {
        let mut count = 0;
        let mut widest: f64 = 0.0;
        for port in ports.iter().filter(|port| port.side == side) {
            count += 1;
            widest = widest.max(measure(&port.label, font_size)?);
        }
        if count > 0 {
            // Ports are equally spaced: reserve an entire label-width plus a gap
            // per slot, not merely the sum of widths across the whole box.
            longest = longest.max((count + 1) as f64 * (widest + 8.0));
        }
    }
    let top_band = if ports.iter().any(|port| port.side == PortSide::Top) {
        line_height + 8.0
    } else if template.is_some() {
        line_height + 12.0
    } else {
        0.0
    };
    let bottom_band = if ports.iter().any(|port| port.side == PortSide::Bottom) {
        line_height + 8.0
    } else {
        0.0
    };
    let side_port_count = ports
        .iter()
        .filter(|port| matches!(port.side, PortSide::Left | PortSide::Right))
        .count();
    let width_factor = match shape {
        NodeShape::Rectangle | NodeShape::Note => 1.0,
        NodeShape::Ellipse => 1.6,
        NodeShape::Diamond => 2.0,
    };
    let width = fixed_width.unwrap_or_else(|| {
        explicit_min_width
            .unwrap_or(100.0)
            .max((longest + padding * 2.0) * width_factor)
    });
    let body_height: f64 = body
        .iter()
        .zip(&body_styles)
        .map(|(part, row)| match part {
            NodePart::Member { .. } => row.font_size * 1.25,
            NodePart::Separator => 8.0,
        })
        .sum();
    let natural_height = line_height
        * original.len() as f64
        * if matches!(shape, NodeShape::Ellipse | NodeShape::Diamond) {
            1.7
        } else {
            1.0
        }
        + body_height
        + if body.is_empty() { 0.0 } else { 4.0 }
        + top_band
        + bottom_band
        + if side_port_count == 0 {
            0.0
        } else {
            4.0 + side_port_count as f64 * line_height
        }
        + padding * 2.0;
    let height =
        fixed_height.unwrap_or_else(|| explicit_min_height.unwrap_or(50.0).max(natural_height));
    let available_width = width / width_factor - padding * 2.0;
    let available_height = height - padding * 2.0;
    if available_width < 0.0 || available_height < line_height {
        return Err("fixed node box cannot fit even one line of text and padding".into());
    }
    if (diagram_type != DiagramType::Generic
        || node.kind == NodeKind::Package
        || matches!(shape, NodeShape::Diamond | NodeShape::Ellipse))
        && (longest > available_width + 0.01 || natural_height > height + 0.01)
    {
        return Err("fixed node box would hide text or violate its shape boundary".into());
    }
    // A body is structural: it is not safe to silently omit addressable rows.
    if !body.is_empty() && natural_height > height + 0.01 {
        return Err("fixed node box cannot fit its member rows".into());
    }
    if !ports.is_empty() && (natural_height > height + 0.01 || longest > available_width + 0.01) {
        return Err("fixed node box cannot fit its visible ports and labels".into());
    }
    let visible = original
        .len()
        .min((available_height / line_height).floor() as usize);
    let vertical_overflow = visible < original.len();
    let mut lines = Vec::new();
    for (index, text) in original[..visible].iter().enumerate() {
        let force = vertical_overflow && index + 1 == visible;
        let space = if index + 1 == visible {
            icon_space
        } else {
            0.0
        };
        lines.push(ellipsize(text, available_width - space, font_size, force)?);
    }
    if diagram_type == DiagramType::Generic {
        for (part, row) in body.iter_mut().zip(&body_styles) {
            if let NodePart::Member { text, .. } = part {
                if row.icon.is_some() {
                    let marker = text.chars().next().expect("visible icon needs glyph");
                    let shown = super::members::visible_text(text, row);
                    *text = format!(
                        "{marker}{}",
                        ellipsize(
                            shown,
                            available_width - row.font_size - 4.0,
                            row.font_size,
                            false
                        )?
                    );
                } else {
                    *text = ellipsize(text, available_width, row.font_size, false)?;
                }
            }
        }
    }
    let port_start = padding
        + top_band
        + line_height * original.len() as f64
        + body_height
        + if body.is_empty() { 0.0 } else { 4.0 }
        + 4.0;
    Ok(BoxNode {
        id: node.id.clone(),
        label,
        template,
        lines,
        icons,
        body,
        body_styles,
        port_start,
        top_band,
        bottom_band,
        ports,
        abstract_node: node.abstract_node,
        kind: node.kind,
        shape,
        x: 0.0,
        y: 0.0,
        width,
        height,
        fixed_width,
        fixed_height,
        font_size,
        padding,
        fill: color(
            styles,
            &element,
            "fill",
            if node.kind == NodeKind::Note {
                "#fff8d6"
            } else {
                "#eff4fa"
            },
        ),
        color: text_color,
        border_color: color(styles, &element, "border-color", "#395373"),
        border_width: number(styles, &element, "border-width", 1.0),
        radius: number(styles, &element, "corner-radius", 0.0),
        line: node.line,
    })
}
