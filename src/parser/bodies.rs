//! Graph node declarations and source-ordered compartments.
use super::{ParseError, attributes, classes, error};
use crate::lex::{quoted_string, valid_identifier, valid_reference};
use crate::syntax::{Diagram, DiagramType, MemberKind, NodeDecl, NodeKind, NodePart};

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
    let (source, abstract_node, visibility) = super::node_modifiers::take(source, line)?;
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
    let (kind, name) = super::node_kinds::classify(
        source,
        note.as_ref().map(|(name, _)| *name),
        anonymous,
        diagram.settings.diagram_type,
    );
    if (diagram.settings.diagram_type == DiagramType::Class
        && matches!(kind, NodeKind::Object | NodeKind::Junction))
        || (diagram.settings.diagram_type == DiagramType::Object
            && !matches!(kind, NodeKind::Object | NodeKind::Note))
        || (diagram.settings.diagram_type == DiagramType::Component
            && !matches!(
                kind,
                NodeKind::Component
                    | NodeKind::ComponentPart
                    | NodeKind::Interface
                    | NodeKind::Package
                    | NodeKind::Note
            ))
        || (diagram.settings.diagram_type == DiagramType::Deployment
            && !matches!(
                kind,
                NodeKind::DeploymentNode
                    | NodeKind::Device
                    | NodeKind::ExecutionEnvironment
                    | NodeKind::Artifact
                    | NodeKind::Component
                    | NodeKind::Package
                    | NodeKind::Note
            ))
        || (!matches!(
            diagram.settings.diagram_type,
            DiagramType::Component | DiagramType::Deployment
        ) && kind == NodeKind::Component)
        || (diagram.settings.diagram_type != DiagramType::Deployment
            && matches!(
                kind,
                NodeKind::DeploymentNode
                    | NodeKind::Device
                    | NodeKind::ExecutionEnvironment
                    | NodeKind::Artifact
            ))
        || (diagram.settings.diagram_type != DiagramType::Component
            && kind == NodeKind::ComponentPart)
    {
        return Err(error(
            line,
            "element kind is not supported by this UML diagram type",
        ));
    }
    super::deployment::validate_node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
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
    if kind == NodeKind::ComponentPart && block {
        return Err(error(line, "part cannot have a body"));
    }
    if kind == NodeKind::Package && !block {
        return Err(error(line, "package requires a nested declaration block"));
    }
    if visibility.is_some() && matches!(kind, NodeKind::Object | NodeKind::Note) {
        return Err(error(line, "visibility is not valid on an object or note"));
    }
    if diagram.settings.diagram_type != DiagramType::Generic
        && abstract_node
        && !matches!(
            kind,
            NodeKind::Class
                | NodeKind::Component
                | NodeKind::AssociationClass
                | NodeKind::Interface
                | NodeKind::Enumeration
                | NodeKind::DataType
        )
    {
        return Err(error(line, "abstract is valid only on a UML classifier"));
    }
    if anonymous && (kind != NodeKind::Object || explicit_id.is_none()) {
        return Err(error(line, "anonymous object requires an explicit @id"));
    }
    let (prepared, templates, template_signature) = if kind == NodeKind::Class {
        super::template_formals::parse(name, line)?
    } else {
        (name.to_string(), Vec::new(), None)
    };
    let (name, attrs) = attributes(&prepared, line)?;
    if name.is_empty() && !anonymous && !matches!(kind, NodeKind::Association | NodeKind::Junction)
    {
        return Err(error(line, "node name required"));
    }
    let explicit = explicit_id.is_some();
    let mut result = NodeDecl {
        id: explicit_id
            .unwrap_or_else(|| parent.map_or(name.clone(), |path| format!("{path}.{name}"))),
        explicit,
        label: if matches!(kind, NodeKind::Association | NodeKind::Junction) {
            Some(name.clone())
        } else {
            note.map(|(_, text)| text)
        },
        classes: Vec::new(),
        stereotypes,
        kind,
        part_type: None,
        abstract_node,
        visibility,
        templates: templates.clone(),
        template_signature: template_signature.clone(),
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
            "of" if kind == NodeKind::ComponentPart && valid_reference(value.trim()) => {
                result.part_type = Some(value.trim().into());
            }
            "of" if kind == NodeKind::Object && valid_reference(value.trim()) => {
                classifier = Some(value.trim().to_string());
            }
            "of" if kind == NodeKind::Object => {
                return Err(error(
                    line,
                    "object classifier must be a valid qualified name",
                ));
            }
            _ => return Err(error(line, format!("unknown node attribute '{key}'"))),
        }
    }
    if kind == NodeKind::ComponentPart && result.part_type.is_none() {
        return Err(error(line, "part requires of=Component"));
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
        result.label = Some(format!(
            "{shown}<{}>",
            template_signature.as_deref().unwrap_or_default()
        ));
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
