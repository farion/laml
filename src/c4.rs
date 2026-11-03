//! C4 source adapter: four checked zoom levels using the existing graph and UML renderers.
//! The adapter preserves source line numbers and does not maintain a cross-file model.
use crate::lex::{
    quoted_string, split_once_top_level, split_top_level, valid_identifier, without_comment,
};
use crate::parser::{self, ParseError};
use crate::syntax::{Diagram, DiagramType};

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
            "label" | "description" | "technology" | "style-classes"
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

fn element(
    text: &str,
    line: usize,
    level: Level,
    scope: &str,
    inside: bool,
    block: bool,
) -> Result<(String, String, bool), ParseError> {
    let (kind, tail, external) = if let Some(tail) = text.strip_prefix("external system ") {
        ("system", tail, true)
    } else if let Some(tail) = text.strip_prefix("external person ") {
        ("person", tail, true)
    } else if let Some(tail) = text.strip_prefix("person ") {
        ("person", tail, false)
    } else if let Some(tail) = text.strip_prefix("system ") {
        ("system", tail, false)
    } else if let Some(tail) = text.strip_prefix("container ") {
        ("container", tail, false)
    } else if let Some(tail) = text.strip_prefix("component ") {
        ("component", tail, false)
    } else {
        return Err(err(line, "expected a C4 element declaration"));
    };
    let (id, attrs) = parse_attributes(tail, line)?;
    if !valid_identifier(id) {
        return Err(err(line, "invalid C4 element identifier"));
    }
    if level == Level::Code && !(kind == "component" && id == scope) {
        return Err(err(
            line,
            "c4-code declares code elements only within the scoped component",
        ));
    }
    if block != (kind == level.owner() && id == scope && level != Level::Context) {
        return Err(err(line, "only the C4 scope may contain an element block"));
    }
    if inside {
        if !(matches!(level, Level::Container) && kind == "container"
            || matches!(level, Level::Component) && kind == "component"
            || matches!(level, Level::Code) && kind == "component" && id == scope)
        {
            return Err(err(line, "element is not permitted inside this C4 scope"));
        }
    } else if kind == level.owner() && id == scope {
        if external {
            return Err(err(line, "C4 scope cannot be external"));
        }
    } else if !matches!(kind, "person" | "system" | "container")
        || kind == "container" && level != Level::Component
    {
        return Err(err(line, "unsupported supporting element at this C4 level"));
    } else if kind == "container" && !inside && level == Level::Component && !external {
        // A container outside the scoped container is another container of the system.
    } else if kind == "system" && !external {
        return Err(err(
            line,
            "supporting systems must be declared 'external system'",
        ));
    }
    if level == Level::Context
        && (kind != "system" || id != scope)
        && kind != "person"
        && kind != "system"
    {
        return Err(err(line, "context supports only people and systems"));
    }
    if level == Level::Context && block {
        return Err(err(line, "context scope is a system box, not a boundary"));
    }
    if level == Level::Container && inside && kind != "container"
        || level == Level::Component && inside && kind != "component"
    {
        return Err(err(line, "wrong C4 element inside scope"));
    }
    if level == Level::Code && inside {
        return Err(err(
            line,
            "c4-code contains code elements, not nested components",
        ));
    }
    let description =
        attr(&attrs, "description").ok_or_else(|| err(line, "C4 element requires description"))?;
    let technology = attr(&attrs, "technology");
    if matches!(kind, "container" | "component") && technology.is_none() {
        return Err(err(line, "C4 container/component requires technology"));
    }
    if matches!(kind, "person" | "system") && technology.is_some() {
        return Err(err(line, "technology belongs to containers and components"));
    }
    let label = attr(&attrs, "label").unwrap_or(id);
    let tag = if external {
        format!("External {}", capitalize(kind))
    } else {
        capitalize(kind).to_string()
    };
    let display = if let Some(tech) = technology {
        format!("{label} [{tag}: {tech}]\n{description}")
    } else {
        format!("{label} [{tag}]\n{description}")
    };
    let mut extras = format!("label={}", quote(&display));
    if let Some(classes) = attr(&attrs, "style-classes") {
        extras.push_str(&format!("; style-classes={classes}"));
    }
    let owner = kind == level.owner() && id == scope;
    let output = if block {
        format!("package {id}({extras}) {{")
    } else {
        format!("{id}({extras})")
    };
    Ok((output, id.into(), owner))
}

fn relation_with_technology(text: &str, line: usize) -> Result<String, ParseError> {
    let (head, body) = text
        .split_once(char::is_whitespace)
        .ok_or_else(|| err(line, "C4 relation needs endpoints"))?;
    let (name, attrs) = parse_attributes(head, line)?;
    if !name.starts_with('@')
        || !valid_identifier(&name[1..])
        || attrs.len() != 1
        || attrs[0].0 != "technology"
    {
        return Err(err(line, "relation accepts only @id(technology=\"...\")"));
    }
    let (ends, label) = split_once_top_level(body, ':')
        .ok_or_else(|| err(line, "C4 relation with technology needs a label"))?;
    if label.trim().is_empty() {
        return Err(err(line, "C4 relation needs a label"));
    }
    Ok(format!("{name} {ends}: {} ({})", label.trim(), attrs[0].1))
}

fn capitalize(kind: &str) -> &str {
    match kind {
        "person" => "Person",
        "system" => "Software System",
        "container" => "Container",
        _ => "Component",
    }
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
    let mut in_scope = false;
    let mut class_body = false;
    let mut owner_count = 0;
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
                } else if in_scope {
                    in_scope = false;
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
                    let (replacement, id, owner) =
                        element(head, line, level, &scope, in_scope, block)?;
                    if !declared.insert(if in_scope {
                        format!("{scope}.{id}")
                    } else {
                        id
                    }) {
                        return Err(err(line, "duplicate C4 element"));
                    }
                    if owner {
                        owner_count += 1;
                    }
                    if block {
                        in_scope = true;
                    }
                    out = replacement;
                } else if text.starts_with('@')
                    && text.contains(" -> ")
                    && text.contains("(technology=")
                {
                    out = relation_with_technology(text, line)?;
                } else if level == Level::Code
                    && in_scope
                    && (head.starts_with("class ")
                        || head.starts_with("interface ")
                        || head.starts_with("enum ")
                        || head.starts_with("datatype "))
                {
                    // UML classifier declarations and member blocks use the class parser.
                    class_body = block;
                } else if !text.contains(" -> ")
                    && !text.starts_with('@')
                    && !(level == Level::Code && in_scope && crate::lex::find_arrow(text).is_some())
                {
                    return Err(err(line, "unknown C4 element or relation"));
                }
            }
        }
        transformed.push_str(&out);
        transformed.push('\n');
    }
    if owner_count != 1 {
        return Err(err(
            1,
            "C4 diagram requires exactly one declared scope element",
        ));
    }
    let diagram = parser::parse_document(&transformed, false)?;
    if level == Level::Code
        && !diagram
            .nodes
            .iter()
            .any(|n| n.parent.as_deref() == Some(scope.as_str()))
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
            .any(|n| n.parent.as_deref() == Some(scope.as_str()))
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
            let belongs = |id: &str| id == scope || id.starts_with(&format!("{scope}."));
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
    Ok(diagram)
}
