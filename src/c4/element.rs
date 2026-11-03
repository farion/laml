use super::{Level, attr, err, parse_attributes, quote};
use crate::lex::valid_identifier;
use crate::parser::ParseError;

pub(super) fn element(
    text: &str,
    line: usize,
    level: Level,
    scope: &str,
    parent_kind: Option<&str>,
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
    let owner = kind == level.owner() && id == scope && !external;
    let enclosing_system =
        level == Level::Component && kind == "system" && !external && parent_kind.is_none();
    if block != (owner && level != Level::Context || enclosing_system) {
        return Err(err(
            line,
            "only the C4 scope or its enclosing system may contain a block",
        ));
    }
    let valid = match (level, parent_kind) {
        (Level::Context, None) => owner || kind == "person" || kind == "system" && external,
        (Level::Container, None) => owner || kind == "person" || kind == "system" && external,
        (Level::Container, Some("system")) => kind == "container" && !external,
        (Level::Component, None) => {
            owner
                || enclosing_system
                || kind == "container"
                || kind == "person"
                || kind == "system" && external
        }
        (Level::Component, Some("system")) => kind == "container" && !external,
        (Level::Component, Some("container")) => kind == "component" && !external,
        (Level::Code, None) => owner,
        _ => false,
    };
    if !valid || kind == "system" && id == scope && external {
        return Err(err(
            line,
            "element is not permitted at this C4 level or scope",
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
        format!("{label}\n[{tag}: {tech}]\n{description}")
    } else {
        format!("{label}\n[{tag}]\n{description}")
    };
    let appearance = attr(&attrs, "appearance");
    if appearance
        .is_some_and(|a| !matches!(a, "browser" | "database" | "bucket" | "folder" | "terminal"))
        || appearance.is_some() && kind != "container"
    {
        return Err(err(
            line,
            "appearance requires a container and browser|database|bucket|folder|terminal",
        ));
    }
    let mut classes = vec![match kind {
        "person" => "c4Person",
        "system" => "c4System",
        "container" => "c4Container",
        _ => "c4Component",
    }];
    if external {
        classes.push("c4External");
    }
    if block {
        classes.push("c4Boundary");
    }
    if let Some(shape) = appearance {
        classes.push(match shape {
            "browser" => "c4Browser",
            "database" => "c4Database",
            "bucket" => "c4Bucket",
            "folder" => "c4Folder",
            _ => "c4Terminal",
        });
    }
    let mut extras = format!("label={}", quote(&display));
    if let Some(user_classes) = attr(&attrs, "style-classes") {
        extras.push_str(&format!(
            "; style-classes={},{}",
            classes.join(","),
            user_classes
        ));
    } else {
        extras.push_str(&format!("; style-classes={}", classes.join(",")));
    }
    let output = if block {
        format!("package {id}({extras}) {{")
    } else {
        format!("{id}({extras})")
    };
    Ok((output, id.into(), owner))
}

fn capitalize(kind: &str) -> &str {
    match kind {
        "person" => "Person",
        "system" => "Software System",
        "container" => "Container",
        _ => "Component",
    }
}
