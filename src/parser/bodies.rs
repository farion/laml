//! Graph node declarations and source-ordered compartments.
use super::{ParseError, attributes, classes, error};
use crate::lex::{quoted_string, valid_identifier};
use crate::syntax::{Diagram, DiagramType, MemberKind, NodeDecl, NodeKind, NodePart, PortDecl};

pub(super) fn node(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    block: bool,
    parent: Option<&str>,
) -> Result<(), ParseError> {
    let mut source = text.trim();
    let explicit_id = if let Some(rest) = source.strip_prefix('@') {
        let (id, tail) = rest
            .split_once(char::is_whitespace)
            .ok_or_else(|| error(line, "expected a name after @id"))?;
        if !valid_identifier(id) {
            return Err(error(line, "invalid explicit node ID"));
        }
        source = tail.trim();
        Some(id.to_string())
    } else {
        None
    };
    let (source, stereotypes) =
        super::stereotypes::suffix(source, line, diagram.settings.diagram_type)?;
    let abstract_node = source.starts_with("abstract ");
    let source = source.strip_prefix("abstract ").unwrap_or(source);
    let visibility = source
        .chars()
        .next()
        .filter(|ch| matches!(ch, '+' | '-' | '#' | '~'));
    let source = if visibility.is_some() {
        source[1..].trim_start()
    } else {
        source
    };
    let anonymous = source.starts_with("anonymous ");
    let source = source.strip_prefix("anonymous ").unwrap_or(source);
    let note = if let Some(tail) = source.strip_prefix("note ") {
        if let Some(at) = tail.find('"') {
            if block {
                return Err(error(line, "note block cannot also have inline text"));
            }
            let value = quoted_string(tail[at..].trim()).map_err(|msg| error(line, msg))?;
            Some((tail[..at].trim(), value))
        } else if block {
            Some((tail.trim(), String::new()))
        } else {
            return Err(error(line, "note requires quoted text or a text block"));
        }
    } else {
        None
    };
    let (kind, name) = if let Some((name, _)) = &note {
        (NodeKind::Note, *name)
    } else if let Some(name) = source.strip_prefix("association class ") {
        (NodeKind::AssociationClass, name)
    } else if source == "association" {
        (NodeKind::Association, "")
    } else if source == "junction" {
        (NodeKind::Junction, "")
    } else if let Some(name) = source.strip_prefix("association ") {
        (NodeKind::Association, name)
    } else if let Some(name) = source.strip_prefix("junction ") {
        (NodeKind::Junction, name)
    } else if let Some(name) = source.strip_prefix("package ") {
        (NodeKind::Package, name)
    } else if let Some(name) = source.strip_prefix("class ") {
        (NodeKind::Class, name)
    } else if let Some(name) = source.strip_prefix("interface ") {
        (NodeKind::Interface, name)
    } else if let Some(name) = source.strip_prefix("enum ") {
        (NodeKind::Enumeration, name)
    } else if let Some(name) = source.strip_prefix("datatype ") {
        (NodeKind::DataType, name)
    } else if anonymous && source.starts_with("object(") {
        (NodeKind::Object, &source["object".len()..])
    } else if let Some(name) = source.strip_prefix("object ") {
        (NodeKind::Object, name)
    } else {
        match diagram.settings.diagram_type {
            DiagramType::Generic => (NodeKind::Generic, source),
            DiagramType::Class => (NodeKind::Class, source),
            DiagramType::Object => (NodeKind::Object, source),
        }
    };
    if (diagram.settings.diagram_type == DiagramType::Class
        && matches!(kind, NodeKind::Object | NodeKind::Junction))
        || (diagram.settings.diagram_type == DiagramType::Object
            && !matches!(kind, NodeKind::Object | NodeKind::Note))
    {
        return Err(error(
            line,
            "element kind is not supported by this UML diagram type",
        ));
    }
    if kind == NodeKind::AssociationClass && diagram.settings.diagram_type != DiagramType::Class {
        return Err(error(line, "association class requires type: class"));
    }
    if kind == NodeKind::Association && diagram.settings.diagram_type != DiagramType::Class {
        return Err(error(line, "association junction requires type: class"));
    }
    if matches!(kind, NodeKind::Association | NodeKind::Junction)
        && (block
            || (name.is_empty() && explicit_id.is_none())
            || abstract_node
            || visibility.is_some()
            || !stereotypes.is_empty())
    {
        return Err(error(
            line,
            "unnamed junction needs @id and cannot have a body, visibility or stereotype",
        ));
    }
    if kind == NodeKind::Package && !block {
        return Err(error(line, "package requires a nested declaration block"));
    }
    if visibility.is_some() && matches!(kind, NodeKind::Object | NodeKind::Note) {
        return Err(error(line, "visibility is not valid on an object or note"));
    }
    if anonymous && (kind != NodeKind::Object || explicit_id.is_none()) {
        return Err(error(line, "anonymous object requires an explicit @id"));
    }
    let (prepared, templates) = if kind == NodeKind::Class && name.contains('<') {
        let open = name.find('<').expect("contains checked");
        let close = name[open + 1..]
            .find('>')
            .ok_or_else(|| error(line, "unclosed template parameters"))?
            + open
            + 1;
        let list = &name[open + 1..close];
        let names: Vec<_> = list.split(',').map(str::trim).collect();
        if names.is_empty()
            || names.iter().any(|name| !valid_identifier(name))
            || names
                .iter()
                .enumerate()
                .any(|(index, name)| names[..index].contains(name))
        {
            return Err(error(line, "invalid or duplicate template parameter"));
        }
        let suffix = &name[close + 1..];
        if !suffix.is_empty() && !suffix.starts_with('(') {
            return Err(error(line, "unexpected text after template parameters"));
        }
        let mut prepared = name[..open].to_string();
        prepared.push_str(suffix);
        (
            prepared,
            names.into_iter().map(str::to_string).collect::<Vec<_>>(),
        )
    } else {
        (name.to_string(), Vec::new())
    };
    let (name, attrs) = attributes(&prepared, line)?;
    if name.is_empty() && !anonymous && !matches!(kind, NodeKind::Association | NodeKind::Junction)
    {
        return Err(error(line, "node name required"));
    }
    let mut result = NodeDecl {
        id: explicit_id
            .unwrap_or_else(|| parent.map_or(name.clone(), |path| format!("{path}.{name}"))),
        label: if matches!(kind, NodeKind::Association | NodeKind::Junction) {
            Some(name.clone())
        } else {
            note.map(|(_, text)| text)
        },
        classes: Vec::new(),
        stereotypes,
        kind,
        abstract_node,
        visibility,
        templates: templates.clone(),
        note_lines: if block && kind == NodeKind::Note {
            Some(Vec::new())
        } else {
            None
        },
        parts: Vec::new(),
        ports: Vec::new(),
        parent: parent.map(str::to_string),
        line,
    };
    let mut classifier = None;
    for (key, value) in attrs {
        match key.as_str() {
            "label" if result.label.is_none() => {
                result.label = Some(quoted_string(&value).map_err(|msg| error(line, msg))?)
            }
            "style-classes" => result.classes = classes(&value, line)?,
            "of" if kind == NodeKind::Object && !value.trim().is_empty() => {
                classifier = Some(value.trim().to_string());
            }
            _ => return Err(error(line, format!("unknown node attribute '{key}'"))),
        }
    }
    if kind == NodeKind::Object {
        if anonymous && classifier.is_none() {
            return Err(error(line, "anonymous object requires of=Classifier"));
        }
        let name = result.label.take().unwrap_or(name);
        result.label = Some(if let Some(ty) = classifier {
            format!("{name}:{ty}")
        } else {
            name
        });
    } else if !templates.is_empty() {
        let shown = result.label.take().unwrap_or(name);
        result.label = Some(format!("{shown}<{}>", templates.join(", ")));
    } else if result.label.is_none() && result.id != name {
        result.label = Some(name);
    }
    diagram.nodes.push(result);
    Ok(())
}

pub(super) fn member(
    text: &str,
    line: usize,
    diagram: &mut Diagram,
    index: usize,
) -> Result<(), ParseError> {
    let diagram_type = diagram.settings.diagram_type;
    if text == "--" {
        let node = &mut diagram.nodes[index];
        if diagram_type != DiagramType::Generic {
            if node.kind != NodeKind::Class
                && node.kind != NodeKind::AssociationClass
                && node.kind != NodeKind::Interface
                && node.kind != NodeKind::DataType
            {
                return Err(error(line, "separator is not valid for this UML node"));
            }
            if node
                .parts
                .iter()
                .any(|part| matches!(part, NodePart::Separator))
                || node.parts.is_empty()
            {
                return Err(error(
                    line,
                    "UML attribute/operation separator must follow attributes",
                ));
            }
        }
        node.parts.push(NodePart::Separator);
        return Ok(());
    }
    let (id, member_classes, mut content) = if let Some(rest) = text.strip_prefix('@') {
        let (head, content) = rest
            .split_once(char::is_whitespace)
            .ok_or_else(|| error(line, "expected member after @id"))?;
        let (id, attrs) = attributes(head, line)?;
        if id.is_empty() {
            return Err(error(line, "invalid explicit member ID"));
        }
        let mut member_classes = Vec::new();
        for (key, value) in attrs {
            if key != "style-classes" {
                return Err(error(line, format!("unknown member attribute '{key}'")));
            }
            member_classes = classes(&value, line)?;
        }
        (Some(id), member_classes, content.trim())
    } else {
        (None, Vec::new(), text)
    };
    if let Some(port) = content.strip_prefix("port ") {
        if !matches!(
            diagram.nodes[index].kind,
            NodeKind::Generic | NodeKind::Class
        ) {
            return Err(error(
                line,
                "ports are supported only on generic or class nodes",
            ));
        }
        let (name, attrs) = attributes(port.trim(), line)?;
        if name.is_empty() {
            return Err(error(line, "port name required"));
        }
        let mut port_classes = member_classes;
        for (key, value) in attrs {
            if key != "style-classes" {
                return Err(error(line, format!("unknown port attribute '{key}'")));
            }
            port_classes = classes(&value, line)?;
        }
        diagram.nodes[index].ports.push(PortDecl {
            id: id.clone().unwrap_or_else(|| name.clone()),
            label: name,
            explicit: id.is_some(),
            classes: port_classes,
            line,
        });
        return Ok(());
    }
    if let Some(id) = &id
        && diagram.nodes.iter().any(|other| {
            other.parts.iter().any(|part| {
                matches!(part, NodePart::Member { id: Some(existing), .. } if existing == id)
            })
        })
    {
        return Err(error(line, format!("duplicate explicit member ID '{id}'")));
    }
    let node = &mut diagram.nodes[index];
    let literal = if let Some(value) = content.strip_prefix("text ") {
        Some(quoted_string(value.trim()).map_err(|msg| error(line, msg))?)
    } else {
        None
    };
    let forced = if let Some(rest) = content.strip_prefix("field ") {
        content = rest;
        Some(MemberKind::Field)
    } else if let Some(rest) = content.strip_prefix("method ") {
        content = rest;
        Some(MemberKind::Method)
    } else {
        None
    };
    let mut abstract_member = false;
    let mut static_member = false;
    loop {
        if let Some(rest) = content.strip_prefix("abstract ") {
            abstract_member = true;
            content = rest;
        } else if let Some(rest) = content.strip_prefix("static ") {
            static_member = true;
            content = rest;
        } else {
            break;
        }
    }
    let kind = if literal.is_some() {
        MemberKind::Text
    } else if node.kind == NodeKind::Object {
        let (name, value) = content
            .split_once('=')
            .ok_or_else(|| error(line, "object slot requires name = value"))?;
        if !valid_identifier(name.trim()) || value.trim().is_empty() {
            return Err(error(line, "invalid object slot"));
        }
        MemberKind::Slot
    } else if node.kind == NodeKind::Enumeration {
        if !valid_identifier(content) {
            return Err(error(line, "invalid enumeration literal"));
        }
        MemberKind::Literal
    } else {
        forced.unwrap_or_else(|| {
            if content.contains('(') && content.contains(')') {
                MemberKind::Method
            } else {
                MemberKind::Field
            }
        })
    };
    if diagram_type == DiagramType::Object
        && (kind != MemberKind::Slot || abstract_member || static_member || forced.is_some())
    {
        return Err(error(line, "object body accepts only name = value slots"));
    }
    if diagram_type == DiagramType::Class {
        if node.kind == NodeKind::Enumeration
            && (abstract_member || static_member || forced.is_some())
        {
            return Err(error(
                line,
                "enumeration literals cannot be abstract, static, field or method",
            ));
        }
        if kind == MemberKind::Field && abstract_member {
            return Err(error(
                line,
                "UML attributes cannot be abstract; mark the classifier or an operation abstract",
            ));
        }
        if kind == MemberKind::Field && (content.contains('(') || content.contains(')')) {
            return Err(error(
                line,
                "UML attribute cannot use operation parentheses",
            ));
        }
        if kind == MemberKind::Method && (!content.contains('(') || !content.contains(')')) {
            return Err(error(line, "UML operation requires parentheses"));
        }
        let separated = node
            .parts
            .iter()
            .any(|part| matches!(part, NodePart::Separator));
        let prior = node.parts.iter().find_map(|part| match part {
            NodePart::Member { kind, .. } => Some(*kind),
            _ => None,
        });
        if matches!(kind, MemberKind::Text | MemberKind::Slot) {
            return Err(error(line, "not a UML class member"));
        }
        if (separated && kind != MemberKind::Method)
            || (!separated && kind == MemberKind::Method && prior == Some(MemberKind::Field))
            || (!separated && kind == MemberKind::Field && prior == Some(MemberKind::Method))
        {
            return Err(error(
                line,
                "attribute and operation compartments must be separated",
            ));
        }
    }
    let default_id = if id.is_some() || kind == MemberKind::Text {
        None
    } else {
        let name = match kind {
            MemberKind::Slot => content.split_once('=').map(|(name, _)| name.trim()),
            MemberKind::Method => content
                .split_once('(')
                .and_then(|(head, _)| head.split_whitespace().last()),
            MemberKind::Field => content
                .split_once(':')
                .map(|(name, _)| name.trim())
                .or_else(|| content.split_whitespace().last()),
            MemberKind::Literal => Some(content.trim()),
            MemberKind::Text => None,
        };
        name.map(|name| name.trim_start_matches(['+', '-', '#', '~']))
            .filter(|name| valid_identifier(name))
            .map(str::to_string)
    };
    node.parts.push(NodePart::Member {
        id,
        default_id,
        classes: member_classes,
        text: literal.unwrap_or_else(|| content.to_string()),
        kind,
        abstract_member,
        static_member,
        line,
    });
    Ok(())
}

pub(super) fn note_line(text: &str, node: &mut NodeDecl) {
    if let Some(lines) = &mut node.note_lines {
        lines.push(text.into());
    }
}

pub(super) fn close(
    line: usize,
    node: &mut NodeDecl,
    diagram_type: DiagramType,
) -> Result<(), ParseError> {
    if let Some(lines) = node.note_lines.take() {
        if lines.iter().all(|line| line.trim().is_empty()) {
            return Err(error(line, "note block needs visible text"));
        }
        node.label = Some(lines.join("\n"));
    }
    if diagram_type != DiagramType::Generic
        && node
            .parts
            .last()
            .is_some_and(|part| matches!(part, NodePart::Separator))
    {
        return Err(error(line, "empty UML operation compartment"));
    }
    Ok(())
}
