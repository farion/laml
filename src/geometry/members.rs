//! Member visibility, icon selection and measured row styling.
use super::nodes::{RowStyle, color, number};
use crate::styles::{Element, StyleSheet, Value};
use crate::syntax::{NodeDecl, NodePart};

pub(crate) fn visible_text<'a>(text: &'a str, row: &RowStyle) -> &'a str {
    if row.icon.is_some() {
        text.get(1..).unwrap_or("")
    } else {
        text
    }
}

pub(super) fn visible_body(
    node: &NodeDecl,
    styles: &StyleSheet,
    size: f64,
    default_color: &str,
) -> Result<(Vec<NodePart>, Vec<RowStyle>), String> {
    let scoped = styles.with_ancestor(&node.id, &node.classes);
    let mut visible = Vec::new();
    let mut rows = Vec::new();
    for part in &node.parts {
        match part {
            NodePart::Separator => {
                visible.push(NodePart::Separator);
                rows.push(RowStyle {
                    font_size: size,
                    color: default_color.into(),
                    icon: None,
                });
            }
            NodePart::Member {
                id,
                default_id,
                classes,
                kind,
                ..
            } => {
                let scoped_id = id.clone().or_else(|| {
                    default_id
                        .as_ref()
                        .map(|name| format!("{}::{name}", node.id))
                });
                let element = Element::Member {
                    id: scoped_id.as_deref(),
                    classes,
                    kind: *kind,
                };
                if matches!(scoped.get(&element, "display").map(|d| &d.value), Some(Value::Keyword(mode)) if mode == "none")
                {
                    continue;
                }
                visible.push(part.clone());
                let icons_requested = matches!(scoped.get(&element, "symbol-rendering").map(|d| &d.value),
                    Some(Value::Keyword(mode)) if mode == "icons");
                if icons_requested
                    && !matches!(scoped.get(&element, "icon-set").map(|d| &d.value),
                    Some(Value::Keyword(set)) if set == "phosphor")
                {
                    return Err("symbol-rendering: icons requires icon-set: phosphor".into());
                }
                let icon = if icons_requested {
                    let text = match part {
                        NodePart::Member { text, .. } => text,
                        _ => unreachable!(),
                    };
                    text.chars()
                        .next()
                        .and_then(crate::svg_icons::visibility)
                        .map(str::to_string)
                } else {
                    None
                };
                rows.push(RowStyle {
                    font_size: number(&scoped, &element, "font-size", size),
                    color: color(&scoped, &element, "color", default_color),
                    icon,
                });
            }
        }
    }
    let mut body = Vec::new();
    let mut styled = Vec::new();
    for (index, part) in visible.iter().enumerate() {
        if matches!(part, NodePart::Separator)
            && (body.is_empty()
                || matches!(body.last(), Some(NodePart::Separator))
                || !visible[index + 1..]
                    .iter()
                    .any(|p| matches!(p, NodePart::Member { .. })))
        {
            continue;
        }
        body.push(part.clone());
        styled.push(rows[index].clone());
    }
    while matches!(body.last(), Some(NodePart::Separator)) {
        body.pop();
        styled.pop();
    }
    Ok((body, styled))
}
