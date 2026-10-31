//! Parser for the documented, line-oriented first language subset.
//! This produces an AST; style imports, layout feasibility, and SVG are later phases.

use crate::lex::{
    find_arrow, quoted_string, split_once_top_level, split_top_level, valid_identifier,
    valid_reference, without_comment,
};
use crate::syntax::{
    Diagram, DiagramType, FlowDirection, LabelPlacement, Property, RelationDecl, StyleEntry,
    StyleRule,
};

mod bindings;
mod bodies;
mod ends;
mod graph_operators;
mod graph_relations;
mod layout;
mod references;
mod selectors;
mod stereotypes;
mod uml;
use layout::{parse_layout, parse_port_side, parse_relation_end, parse_relation_label};
use selectors::validate_selector;

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
    NodeBody(usize),
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

fn prescan_type(source: &str) -> DiagramType {
    let mut settings = false;
    for raw in source.lines() {
        let text = without_comment(raw).trim();
        if !settings {
            settings = text == "settings {";
        } else if text == "}" {
            settings = false;
        } else if let Some(value) = text.strip_prefix("type:") {
            return match value.trim() {
                "class" => DiagramType::Class,
                "object" => DiagramType::Object,
                _ => DiagramType::Generic,
            };
        }
    }
    DiagramType::Generic
}

fn parse_document(source: &str, stylesheet: bool) -> Result<Diagram, ParseError> {
    let mut diagram = Diagram::default();
    let mut section = Section::Top;
    let mut seen = [false; 4];
    let mut seen_settings = [false; 4];
    diagram.settings.diagram_type = prescan_type(source);
    for (index, original) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(original).trim();
        if text.is_empty() {
            if original.trim().is_empty()
                && let Section::NodeBody(index) = section
                && diagram.nodes[index].note_lines.is_some()
            {
                bodies::note_line("", &mut diagram.nodes[index]);
            }
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
                } else if let Some(header) = text.strip_suffix('{') {
                    bodies::node(header.trim(), line, &mut diagram, true, None)?;
                    Section::NodeBody(diagram.nodes.len() - 1)
                } else {
                    graph_relations::parse_semantic(text, line, &mut diagram, None)?;
                    Section::Semantics
                }
            }
            Section::NodeBody(index)
                if diagram.nodes[index].kind == crate::syntax::NodeKind::Package =>
            {
                let scope = diagram.nodes[index].id.clone();
                if text == "}" {
                    if let Some(parent) = &diagram.nodes[index].parent {
                        let outer = diagram
                            .nodes
                            .iter()
                            .position(|node| &node.id == parent)
                            .expect("validated container parent");
                        Section::NodeBody(outer)
                    } else {
                        Section::Semantics
                    }
                } else if let Some(header) = text.strip_suffix('{') {
                    bodies::node(header.trim(), line, &mut diagram, true, Some(&scope))?;
                    Section::NodeBody(diagram.nodes.len() - 1)
                } else {
                    graph_relations::parse_semantic(text, line, &mut diagram, Some(&scope))?;
                    Section::NodeBody(index)
                }
            }
            Section::NodeBody(index) => {
                if text == "}" {
                    bodies::close(
                        line,
                        &mut diagram.nodes[index],
                        diagram.settings.diagram_type,
                    )?;
                    Section::Semantics
                } else if diagram.nodes[index].note_lines.is_some() {
                    bodies::note_line(
                        if text == "\\}" { "}" } else { text },
                        &mut diagram.nodes[index],
                    );
                    Section::NodeBody(index)
                } else {
                    bodies::member(text, line, &mut diagram, index)?;
                    Section::NodeBody(index)
                }
            }
            Section::Layout => {
                if text == "}" {
                    Section::Top
                } else {
                    if text.starts_with("port[") {
                        diagram.port_layout.push(parse_port_side(text, line)?);
                    } else if text.starts_with("relation[") {
                        if text.contains("].source") || text.contains("].target") {
                            diagram
                                .relation_end_layout
                                .push(parse_relation_end(text, line)?);
                        } else {
                            diagram
                                .relation_layout
                                .push(parse_relation_label(text, line)?);
                        }
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
    references::resolve_and_validate(&mut diagram)?;
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
            if !(valid_identifier(key) || key == "style-classes") || value.is_empty() {
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

fn parse_setting(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    seen: &mut [bool; 4],
) -> Result<(), ParseError> {
    let (key, value) =
        split_once_top_level(text, ':').ok_or_else(|| error(line, "expected setting: value"))?;
    let value = value.trim();
    let slot = match key.trim() {
        "direction" => 0,
        "naming-convention" => 1,
        "relation-label" => 2,
        "type" => 3,
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
        "type" => {
            diagram.settings.diagram_type = match value {
                "class" => DiagramType::Class,
                "object" => DiagramType::Object,
                _ => return Err(error(line, "invalid graph diagram type")),
            }
        }
        _ => return Err(error(line, format!("unknown setting '{}'", key.trim()))),
    }
    Ok(())
}
