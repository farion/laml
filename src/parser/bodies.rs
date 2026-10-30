//! Graph node declarations and source-ordered compartments.
use super::{ParseError, attributes, classes, error};
use crate::lex::{quoted_string, valid_identifier};
use crate::syntax::{Diagram, DiagramType, MemberKind, NodeDecl, NodeKind, NodePart};

pub(super) fn node(text: &str, line: usize, diagram: &mut Diagram) -> Result<(), ParseError> {
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
    let abstract_node = source.starts_with("abstract ");
    let source = source.strip_prefix("abstract ").unwrap_or(source);
    let note = if let Some(tail) = source.strip_prefix("note ") {
        let at = tail
            .find('"')
            .ok_or_else(|| error(line, "note requires quoted text"))?;
        let value = quoted_string(tail[at..].trim()).map_err(|msg| error(line, msg))?;
        Some((tail[..at].trim(), value))
    } else {
        None
    };
    let (kind, name) = if let Some((name, _)) = &note {
        (NodeKind::Note, *name)
    } else if let Some(name) = source.strip_prefix("class ") {
        (NodeKind::Class, name)
    } else if let Some(name) = source.strip_prefix("interface ") {
        (NodeKind::Interface, name)
    } else if let Some(name) = source.strip_prefix("enum ") {
        (NodeKind::Enumeration, name)
    } else if let Some(name) = source.strip_prefix("datatype ") {
        (NodeKind::DataType, name)
    } else if let Some(name) = source.strip_prefix("object ") {
        (NodeKind::Object, name)
    } else {
        match diagram.settings.diagram_type {
            DiagramType::Generic => (NodeKind::Generic, source),
            DiagramType::Class => (NodeKind::Class, source),
            DiagramType::Object => (NodeKind::Object, source),
        }
    };
    if (diagram.settings.diagram_type == DiagramType::Class && kind == NodeKind::Object)
        || (diagram.settings.diagram_type == DiagramType::Object
            && !matches!(kind, NodeKind::Object | NodeKind::Note))
    {
        return Err(error(
            line,
            "element kind is not supported by this UML diagram type",
        ));
    }
    let (name, attrs) = attributes(name, line)?;
    if name.is_empty() {
        return Err(error(line, "node name required"));
    }
    let mut result = NodeDecl {
        id: explicit_id.unwrap_or_else(|| name.clone()),
        label: note.map(|(_, text)| text),
        classes: Vec::new(),
        kind,
        abstract_node,
        parts: Vec::new(),
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
        let name = result.label.take().unwrap_or(name);
        result.label = Some(if let Some(ty) = classifier {
            format!("{name}:{ty}")
        } else {
            name
        });
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
    let (id, mut content) = if let Some(rest) = text.strip_prefix('@') {
        let (id, content) = rest
            .split_once(char::is_whitespace)
            .ok_or_else(|| error(line, "expected member after @id"))?;
        if !valid_identifier(id) {
            return Err(error(line, "invalid explicit member ID"));
        }
        (Some(id.to_string()), content.trim())
    } else {
        (None, text)
    };
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
    if diagram_type == DiagramType::Class {
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
    node.parts.push(NodePart::Member {
        id,
        text: literal.unwrap_or_else(|| content.to_string()),
        kind,
        abstract_member,
        static_member,
        line,
    });
    Ok(())
}

pub(super) fn close(
    line: usize,
    node: &NodeDecl,
    diagram_type: DiagramType,
) -> Result<(), ParseError> {
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
