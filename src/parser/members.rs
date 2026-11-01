//! Typed node body members and visible ports.
use super::{ParseError, attributes, classes, error};
use crate::lex::{quoted_string, valid_identifier};
use crate::syntax::{Diagram, DiagramType, MemberKind, NodeKind, NodePart};

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
                && node.kind != NodeKind::Component
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
    if super::port_bodies::parse(
        content,
        line,
        &mut diagram.nodes[index],
        diagram_type,
        &id,
        &member_classes,
    )? {
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
    let mut forced = None;
    let mut abstract_member = false;
    let mut static_member = false;
    loop {
        if let Some(rest) = content.strip_prefix("field ") {
            if forced.replace(MemberKind::Field).is_some() {
                return Err(error(line, "duplicate or conflicting member kind"));
            }
            content = rest;
        } else if let Some(rest) = content.strip_prefix("method ") {
            if forced.replace(MemberKind::Method).is_some() {
                return Err(error(line, "duplicate or conflicting member kind"));
            }
            content = rest;
        } else if let Some(rest) = content.strip_prefix("abstract ") {
            if abstract_member {
                return Err(error(line, "duplicate abstract member modifier"));
            }
            abstract_member = true;
            content = rest;
        } else if let Some(rest) = content.strip_prefix("static ") {
            if static_member {
                return Err(error(line, "duplicate static member modifier"));
            }
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
    if diagram_type == DiagramType::Object && kind == MemberKind::Slot {
        let slot = content
            .split_once('=')
            .expect("validated object slot")
            .0
            .trim();
        if node.parts.iter().any(|part| {
            matches!(part, NodePart::Member {
            kind: MemberKind::Slot, text, ..
        } if text.split_once('=').is_some_and(|(prior, _)| prior.trim() == slot))
        }) {
            return Err(error(line, format!("duplicate UML object slot '{slot}'")));
        }
    }
    if matches!(diagram_type, DiagramType::Class | DiagramType::Component) {
        if node.kind == NodeKind::Interface && content.starts_with(['-', '#', '~']) {
            return Err(error(line, "UML interface features must be public"));
        }
        if node.kind == NodeKind::Enumeration && node.parts.iter().any(|part| matches!(part,NodePart::Member {kind:MemberKind::Literal,text,..} if text==content)) {
            return Err(error(line, "duplicate UML enumeration literal"));
        }
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
    let shown = literal.unwrap_or_else(|| content.to_string());
    let shown = if matches!(diagram_type, DiagramType::Class | DiagramType::Component) {
        super::member_notation::normalize(&shown, kind, line)?
    } else {
        shown
    };
    node.parts.push(NodePart::Member {
        id,
        default_id,
        classes: member_classes,
        text: shown,
        kind,
        abstract_member,
        static_member,
        line,
    });
    Ok(())
}
