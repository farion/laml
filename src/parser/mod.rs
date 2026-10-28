//! Parser for the documented, line-oriented first language subset.
//! This produces an AST; style imports, layout feasibility, and SVG are later phases.

use crate::RelationKind;
use crate::lex::{
    find_arrow, quoted_string, split_once_top_level, split_top_level, valid_identifier,
    without_comment,
};
use crate::syntax::{
    Diagram, FlowDirection, LabelPlacement, NodeDecl, Property, RelationDecl, StyleEntry, StyleRule,
};

mod layout;
use layout::{parse_layout, parse_relation_label};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

enum Section {
    Top,
    Settings,
    Semantics,
    Layout,
    Styling,
    StyleRule(StyleRule),
}

pub fn parse(source: &str) -> Result<Diagram, ParseError> {
    parse_document(source, false)
}

/// Parse a standalone LSS file containing only one `styling { ... }` section.
pub fn parse_stylesheet(source: &str) -> Result<Vec<StyleEntry>, ParseError> {
    Ok(parse_document(source, true)?.styling)
}

fn parse_document(source: &str, stylesheet: bool) -> Result<Diagram, ParseError> {
    let mut diagram = Diagram::default();
    let mut section = Section::Top;
    let mut seen = [false; 4];
    let mut seen_settings = [false; 3];
    for (index, original) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(original).trim();
        if text.is_empty() {
            continue;
        }
        section = match section {
            Section::Top => {
                let (kind, next) = match text {
                    "settings {" => (0, Section::Settings),
                    "semantics {" => (1, Section::Semantics),
                    "layout {" => (2, Section::Layout),
                    "styling {" => (3, Section::Styling),
                    _ => return Err(error(line, "expected a section heading followed by '{'")),
                };
                if stylesheet && kind != 3 {
                    return Err(error(line, "stylesheet may only contain styling"));
                }
                if seen[kind] {
                    return Err(error(line, "duplicate section"));
                }
                seen[kind] = true;
                next
            }
            Section::Settings => {
                if text == "}" {
                    Section::Top
                } else {
                    parse_setting(text, line, &mut diagram, &mut seen_settings)?;
                    Section::Settings
                }
            }
            Section::Semantics => {
                if text == "}" {
                    Section::Top
                } else {
                    parse_semantic(text, line, &mut diagram)?;
                    Section::Semantics
                }
            }
            Section::Layout => {
                if text == "}" {
                    Section::Top
                } else {
                    if text.starts_with("relation[") {
                        diagram
                            .relation_layout
                            .push(parse_relation_label(text, line)?);
                    } else {
                        diagram.layout.push(parse_layout(text, line)?);
                    }
                    Section::Layout
                }
            }
            Section::Styling => {
                if text == "}" {
                    Section::Top
                } else if let Some(path) = text.strip_prefix("import ") {
                    let path = quoted_string(path.trim()).map_err(|msg| error(line, msg))?;
                    if path.is_empty() {
                        return Err(error(line, "style import path cannot be empty"));
                    }
                    diagram.styling.push(StyleEntry::Import { path, line });
                    Section::Styling
                } else if let Some(selector) = text.strip_suffix('{') {
                    let selector = selector.trim();
                    validate_selector(selector, line)?;
                    Section::StyleRule(StyleRule {
                        selector: selector.into(),
                        properties: Vec::new(),
                        line,
                    })
                } else {
                    return Err(error(line, "expected style import, selector block, or '}'"));
                }
            }
            Section::StyleRule(mut rule) => {
                if text == "}" {
                    diagram.styling.push(StyleEntry::Rule(rule));
                    Section::Styling
                } else {
                    let (name, value) = split_once_top_level(text, ':')
                        .ok_or_else(|| error(line, "expected style property: value"))?;
                    let (name, value) = (name.trim(), value.trim());
                    if name.is_empty()
                        || value.is_empty()
                        || !name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                    {
                        return Err(error(line, "invalid style property"));
                    }
                    if rule.properties.iter().any(|property| property.name == name) {
                        return Err(error(line, format!("duplicate style property '{name}'")));
                    }
                    rule.properties.push(Property {
                        name: name.into(),
                        value: value.into(),
                        line,
                    });
                    Section::StyleRule(rule)
                }
            }
        };
    }
    if !matches!(section, Section::Top) {
        return Err(error(
            source.lines().count().max(1),
            "unclosed section or style rule",
        ));
    }
    if stylesheet && !seen[3] {
        return Err(error(1, "missing styling section"));
    }
    if !stylesheet && !seen[1] {
        return Err(error(1, "missing semantics section"));
    }
    validate_references(&diagram)?;
    Ok(diagram)
}

fn attributes(text: &str, line: usize) -> Result<(String, Vec<(String, String)>), ParseError> {
    let text = text.trim();
    let (id, body) = if let Some(open) = text.find('(') {
        let body = text[open + 1..]
            .strip_suffix(')')
            .ok_or_else(|| error(line, "attribute list must end with ')'"))?;
        (text[..open].trim(), Some(body))
    } else {
        (text, None)
    };
    if !id.is_empty() && !valid_identifier(id) {
        return Err(error(line, format!("invalid identifier '{id}'")));
    }
    let mut result = Vec::new();
    if let Some(body) = body {
        for entry in split_top_level(body, ';') {
            let (key, value) = split_once_top_level(entry.trim(), '=')
                .ok_or_else(|| error(line, "expected attribute=value"))?;
            let (key, value) = (key.trim(), value.trim());
            if !valid_identifier(key) || value.is_empty() {
                return Err(error(line, "invalid attribute"));
            }
            if result
                .iter()
                .any(|(previous, _): &(String, String)| previous == key)
            {
                return Err(error(line, format!("duplicate attribute '{key}'")));
            }
            result.push((key.into(), value.into()));
        }
    }
    Ok((id.into(), result))
}

fn classes(value: &str, line: usize) -> Result<Vec<String>, ParseError> {
    value
        .split(',')
        .map(|name| {
            let name = name.trim();
            if !valid_identifier(name) {
                Err(error(line, "invalid class name"))
            } else {
                Ok(name.into())
            }
        })
        .collect()
}

fn parse_semantic(text: &str, line: usize, diagram: &mut Diagram) -> Result<(), ParseError> {
    // Prefixes end at a colon before the arrow; labels begin at a colon after it.
    let arrow_at = find_arrow(text);
    if arrow_at.is_none() {
        let (id, attrs) = attributes(text, line)?;
        if id.is_empty() {
            return Err(error(line, "node identifier required"));
        }
        let mut node = NodeDecl {
            id,
            label: None,
            classes: Vec::new(),
            line,
        };
        for (key, value) in attrs {
            match key.as_str() {
                "label" => {
                    node.label = Some(quoted_string(&value).map_err(|msg| error(line, msg))?)
                }
                "classes" => node.classes = classes(&value, line)?,
                _ => return Err(error(line, format!("unknown node attribute '{key}'"))),
            }
        }
        diagram.nodes.push(node);
        return Ok(());
    }
    let arrow_at = arrow_at.expect("checked above");
    let (name, attrs, body) = match split_once_top_level(text, ':') {
        Some((prefix, rest)) if prefix.len() < arrow_at => {
            let (name, attrs) = attributes(prefix, line)?;
            (
                if name.is_empty() { None } else { Some(name) },
                attrs,
                rest.trim(),
            )
        }
        _ => (None, Vec::new(), text),
    };
    let mut relation_classes = Vec::new();
    for (key, value) in attrs {
        if key != "classes" {
            return Err(error(line, format!("unknown relation attribute '{key}'")));
        }
        relation_classes = classes(&value, line)?;
    }
    let (endpoints, label) = match split_once_top_level(body, ':') {
        Some((endpoints, label)) => {
            let label = label.trim();
            if label.is_empty() {
                return Err(error(line, "empty relation label"));
            }
            let label = if label.starts_with('"') {
                quoted_string(label).map_err(|msg| error(line, msg))?
            } else {
                label.into()
            };
            if label.trim().is_empty() {
                return Err(error(line, "empty relation label"));
            }
            (endpoints.trim(), Some(label))
        }
        None => (body.trim(), None),
    };
    let tokens: Vec<_> = endpoints.split_whitespace().collect();
    if tokens.len() != 3 || !valid_identifier(tokens[0]) || !valid_identifier(tokens[2]) {
        return Err(error(line, "expected relation: A -> B"));
    }
    let (from, to, kind) = match tokens[1] {
        "->" => (tokens[0], tokens[2], RelationKind::Directed),
        "<-" => (tokens[2], tokens[0], RelationKind::Directed),
        "<->" => (tokens[0], tokens[2], RelationKind::Bidirectional),
        _ => return Err(error(line, "expected ->, <-, or <->")),
    };
    diagram.relations.push(RelationDecl {
        name,
        classes: relation_classes,
        from: from.into(),
        to: to.into(),
        kind,
        label,
        line,
    });
    Ok(())
}

fn parse_setting(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    seen: &mut [bool; 3],
) -> Result<(), ParseError> {
    let (key, value) =
        split_once_top_level(text, ':').ok_or_else(|| error(line, "expected setting: value"))?;
    let value = value.trim();
    let slot = match key.trim() {
        "direction" => 0,
        "naming-convention" => 1,
        "relation-label" => 2,
        _ => return Err(error(line, format!("unknown setting '{}'", key.trim()))),
    };
    if seen[slot] {
        return Err(error(line, format!("duplicate setting '{}'", key.trim())));
    }
    seen[slot] = true;
    match key.trim() {
        "direction" => {
            diagram.settings.direction = match value {
                "left-to-right" => FlowDirection::LeftToRight,
                "right-to-left" => FlowDirection::RightToLeft,
                "top-to-bottom" => FlowDirection::TopToBottom,
                "bottom-to-top" => FlowDirection::BottomToTop,
                _ => return Err(error(line, "invalid diagram direction")),
            }
        }
        "naming-convention" => {
            if value != "uppercase-nodes-lowercase-relations" {
                return Err(error(line, "invalid naming convention"));
            }
            diagram.settings.strict_case = true;
        }
        "relation-label" => {
            diagram.settings.relation_label = match value {
                "near" => LabelPlacement::Near,
                "along" => LabelPlacement::Along,
                "inline" => LabelPlacement::Inline,
                _ => return Err(error(line, "invalid relation-label setting")),
            }
        }
        _ => return Err(error(line, format!("unknown setting '{}'", key.trim()))),
    }
    Ok(())
}

fn validate_selector(selector: &str, line: usize) -> Result<(), ParseError> {
    let valid = if let Some(class) = selector
        .strip_prefix("node.")
        .or_else(|| selector.strip_prefix("relation."))
        .or_else(|| selector.strip_prefix('.'))
    {
        valid_identifier(class)
    } else if let Some(id) = selector
        .strip_prefix("node[")
        .or_else(|| selector.strip_prefix("relation["))
    {
        id.strip_suffix(']').is_some_and(valid_identifier)
    } else {
        selector == "node" || selector == "relation"
    };
    if valid {
        Ok(())
    } else {
        Err(error(line, "invalid style selector"))
    }
}

fn validate_references(diagram: &Diagram) -> Result<(), ParseError> {
    for (index, node) in diagram.nodes.iter().enumerate() {
        if diagram.nodes[..index]
            .iter()
            .any(|other| other.id == node.id)
        {
            return Err(error(node.line, format!("duplicate node '{}'", node.id)));
        }
        if diagram.settings.strict_case && !node.id.starts_with(|ch: char| ch.is_ascii_uppercase())
        {
            return Err(error(node.line, "node identifier must start uppercase"));
        }
    }
    for (index, relation) in diagram.relations.iter().enumerate() {
        if let Some(name) = &relation.name {
            if diagram.relations[..index]
                .iter()
                .any(|other| other.name.as_ref() == Some(name))
            {
                return Err(error(relation.line, format!("duplicate relation '{name}'")));
            }
            if diagram.settings.strict_case && !name.starts_with(|ch: char| ch.is_ascii_lowercase())
            {
                return Err(error(relation.line, "relation name must start lowercase"));
            }
        }
        for endpoint in [&relation.from, &relation.to] {
            if !diagram.nodes.iter().any(|node| &node.id == endpoint) {
                return Err(error(relation.line, format!("unknown node '{endpoint}'")));
            }
        }
    }
    for rule in &diagram.layout {
        for id in [&rule.first, &rule.second] {
            if !diagram.nodes.iter().any(|node| &node.id == id) {
                return Err(error(rule.line, format!("unknown node '{id}'")));
            }
        }
    }
    for (index, rule) in diagram.relation_layout.iter().enumerate() {
        let target = diagram
            .relations
            .iter()
            .find(|relation| relation.name.as_deref() == Some(rule.name.as_str()));
        let Some(target) = target else {
            return Err(error(
                rule.line,
                format!("unknown named relation '{}'", rule.name),
            ));
        };
        if target.label.is_none() {
            return Err(error(rule.line, "relation has no label to place"));
        }
        if diagram.relation_layout[..index]
            .iter()
            .any(|prior| prior.name == rule.name)
        {
            return Err(error(
                rule.line,
                format!("duplicate relation layout for '{}'", rule.name),
            ));
        }
    }
    Ok(())
}
