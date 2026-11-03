//! C4 source adapter: four checked zoom levels using the existing graph and UML renderers.
//! The adapter preserves source line numbers and does not maintain a cross-file model.
use crate::lex::{
    quoted_string, split_once_top_level, split_top_level, valid_identifier, without_comment,
};
use crate::parser::{self, ParseError};
use crate::syntax::{Diagram, DiagramType};
mod element;
mod relation;
mod style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Context,
    Container,
    Component,
    Code,
}

impl Level {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "c4-context" => Some(Self::Context),
            "c4-container" => Some(Self::Container),
            "c4-component" => Some(Self::Component),
            "c4-code" => Some(Self::Code),
            _ => None,
        }
    }
    fn owner(self) -> &'static str {
        match self {
            Self::Context | Self::Container => "system",
            Self::Component => "container",
            Self::Code => "component",
        }
    }
}

fn err(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

pub fn is_c4_source(source: &str) -> bool {
    let mut settings = false;
    for raw in source.lines() {
        let text = without_comment(raw).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings
            && text
                .strip_prefix("type:")
                .is_some_and(|s| Level::parse(s.trim()).is_some())
        {
            return true;
        }
    }
    false
}

type Attributes<'a> = Vec<(&'a str, String)>;

fn parse_attributes(text: &str, line: usize) -> Result<(&str, Attributes<'_>), ParseError> {
    let Some((name, tail)) = text.split_once('(') else {
        return Ok((text.trim(), Vec::new()));
    };
    let content = tail
        .strip_suffix(')')
        .ok_or_else(|| err(line, "C4 attributes must end in ')'"))?;
    let mut attrs = Vec::new();
    for item in split_top_level(content, ';') {
        let (key, value) = split_once_top_level(item, '=')
            .ok_or_else(|| err(line, "expected C4 attribute=value"))?;
        let key = key.trim();
        if !matches!(
            key,
            "label" | "description" | "technology" | "style-classes" | "appearance"
        ) || attrs.iter().any(|(old, _)| *old == key)
        {
            return Err(err(
                line,
                format!("unknown or duplicate C4 attribute '{key}'"),
            ));
        }
        let value = if key == "style-classes" {
            value.trim().to_string()
        } else {
            quoted_string(value.trim()).map_err(|e| err(line, e))?
        };
        if value.trim().is_empty() {
            return Err(err(line, format!("empty C4 attribute '{key}'")));
        }
        attrs.push((key, value));
    }
    Ok((name.trim(), attrs))
}

fn attr<'a>(attrs: &'a [(&str, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.as_str())
}

fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

pub fn parse(source: &str) -> Result<Diagram, ParseError> {
    let mut level = None;
    let mut scope = None;
    let mut section = "";
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let t = without_comment(raw).trim();
        if t == "settings {" {
            section = "settings";
        } else if section == "settings" && t == "}" {
            section = "";
        } else if section == "settings" {
            if let Some(value) = t.strip_prefix("type:") {
                level = Level::parse(value.trim());
            }
            if let Some(value) = t.strip_prefix("scope:") {
                if scope.is_some() || !valid_identifier(value.trim()) {
                    return Err(err(line, "invalid or duplicate C4 scope"));
                }
                scope = Some(value.trim().to_string());
            }
        }
    }
    let level = level.ok_or_else(|| {
        err(
            1,
            "expected type: c4-context|c4-container|c4-component|c4-code",
        )
    })?;
    let scope = scope.ok_or_else(|| err(1, "C4 diagrams require settings scope: Identifier"))?;
    let mut transformed = String::new();
    let mut section = "";
    let mut containers: Vec<(String, &str)> = Vec::new();
    let mut class_body = false;
    let mut owner_count = 0;
    let mut enclosing_count = 0;
    let mut declared = std::collections::HashSet::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = without_comment(raw).trim();
        let mut out = raw.to_string();
        if text == "settings {" {
            section = "settings";
        } else if section == "settings" {
            if let Some(value) = text.strip_prefix("type:") {
                if Level::parse(value.trim()).is_none() {
                    return Err(err(line, "invalid C4 diagram type"));
                }
                out = if level == Level::Code {
                    "type: class".into()
                } else {
                    String::new()
                };
            } else if text.starts_with("scope:") {
                out.clear();
            } else if text == "}" {
                section = "";
            }
        } else if text == "semantics {" {
            section = "semantics";
        } else if section == "semantics" {
            if text == "}" {
                if class_body {
                    class_body = false;
                } else if !containers.is_empty() {
                    containers.pop();
                } else {
                    section = "";
                }
            } else if class_body {
                // UML classifier members are parsed by the existing class parser.
            } else if !text.is_empty() && !text.starts_with("//") {
                let block = text.ends_with('{');
                let head = if block {
                    text.trim_end_matches('{').trim()
                } else {
                    text
                };
                let c4_element = [
                    "person ",
                    "external person ",
                    "system ",
                    "external system ",
                    "container ",
                    "component ",
                ]
                .iter()
                .any(|prefix| head.starts_with(prefix));
                if c4_element {
                    let parent_kind = containers.last().map(|(_, kind)| *kind);
                    let (replacement, id, owner) =
                        element::element(head, line, level, &scope, parent_kind, block)?;
                    let parent_path = containers
                        .iter()
                        .map(|(id, _)| id.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    let qualified = if parent_path.is_empty() {
                        id.clone()
                    } else {
                        format!("{parent_path}.{id}")
                    };
                    if !declared.insert(qualified) {
                        return Err(err(line, "duplicate C4 element"));
                    }
                    if owner {
                        owner_count += 1;
                    }
                    if block {
                        let kind = if head.starts_with("system ") {
                            "system"
                        } else if head.starts_with("container ") {
                            "container"
                        } else {
                            "component"
                        };
                        if kind == "system" && level == Level::Component {
                            enclosing_count += 1;
                        }
                        containers.push((id, kind));
                    }
                    out = replacement;
                } else if text.starts_with('@')
                    && text.contains(" -> ")
                    && text.contains("(technology=")
                {
                    out = relation::relation_with_technology(text, line)?;
                } else if level == Level::Code
                    && containers
                        .last()
                        .is_some_and(|(id, kind)| id == &scope && *kind == "component")
                    && (head.starts_with("class ")
                        || head.starts_with("interface ")
                        || head.starts_with("enum ")
                        || head.starts_with("datatype "))
                {
                    // UML classifier declarations and member blocks use the class parser.
                    class_body = block;
                } else if !text.contains(" -> ")
                    && !text.starts_with('@')
                    && !(level == Level::Code
                        && containers.last().is_some_and(|(id, _)| id == &scope)
                        && crate::lex::find_arrow(text).is_some())
                {
                    return Err(err(line, "unknown C4 element or relation"));
                }
            }
        }
        transformed.push_str(&out);
        transformed.push('\n');
    }
    if owner_count != 1 || enclosing_count > 1 {
        return Err(err(
            1,
            "C4 diagram requires one scope and at most one enclosing system",
        ));
    }
    let mut diagram = parser::parse_document(&transformed, false)?;
    let scoped_id = diagram
        .nodes
        .iter()
        .find(|node| node.id == scope || node.id.ends_with(&format!(".{scope}")))
        .map(|node| node.id.clone())
        .ok_or_else(|| err(1, "C4 scope not found"))?;
    if enclosing_count == 1
        && !diagram.nodes.iter().any(|node| {
            node.id == scoped_id
                && node.parent.as_ref().is_some_and(|id| {
                    diagram
                        .nodes
                        .iter()
                        .any(|p| &p.id == id && p.classes.iter().any(|c| c == "c4System"))
                })
        })
    {
        return Err(err(
            1,
            "scoped container must be inside its enclosing system",
        ));
    }
    if level == Level::Code
        && !diagram
            .nodes
            .iter()
            .any(|n| n.parent.as_deref() == Some(scoped_id.as_str()))
    {
        return Err(err(
            1,
            "c4-code needs at least one code element inside its component",
        ));
    }
    if matches!(level, Level::Container | Level::Component)
        && !diagram
            .nodes
            .iter()
            .any(|n| n.parent.as_deref() == Some(scoped_id.as_str()))
    {
        return Err(err(1, "C4 scope must contain at least one element"));
    }
    if level != Level::Code {
        for relation in &diagram.relations {
            if relation.kind != crate::RelationKind::Directed
                || relation.label.as_deref().is_none_or(str::is_empty)
            {
                return Err(err(
                    relation.line,
                    "C4 relationships must be directed and labelled",
                ));
            }
            let belongs = |id: &str| id == scoped_id || id.starts_with(&format!("{scoped_id}."));
            if !belongs(&relation.from) && !belongs(&relation.to) {
                return Err(err(
                    relation.line,
                    "C4 relationship must touch the scoped element or its contents",
                ));
            }
        }
    }
    if level == Level::Code && diagram.settings.diagram_type != DiagramType::Class {
        return Err(err(1, "c4-code needs the class renderer"));
    }
    if level != Level::Code {
        for relation in &mut diagram.relations {
            relation.appearance.dashed = true;
        }
    }
    style::apply(&mut diagram)?;
    Ok(diagram)
}
