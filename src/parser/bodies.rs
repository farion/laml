//! Graph node declarations and source-ordered compartments.
use super::{ParseError, attributes, classes, error};
use crate::lex::{quoted_string, valid_identifier, valid_reference};
use crate::syntax::{Diagram, DiagramType, NodeDecl, NodeKind, NodePart};

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
    if !super::node_kinds::allowed(kind, diagram.settings.diagram_type) {
        return Err(error(
            line,
            "element kind is not supported by this UML diagram type",
        ));
    }
    super::deployment::validate_node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    super::state::validate_node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    super::activity::validate_node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    super::composite::node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    super::profile::node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    crate::interaction_overview::validate_node(
        diagram,
        kind,
        parent,
        block,
        visibility,
        &stereotypes,
        line,
    )?;
    super::usecase::node(diagram, kind, parent, block, visibility, &stereotypes, line)?;
    super::package_diagram::node(diagram, kind, parent, visibility, &stereotypes, line)?;
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
    if kind == NodeKind::Package
        && !block
        && !matches!(
            diagram.settings.diagram_type,
            DiagramType::Package | DiagramType::Profile
        )
    {
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
        } else if matches!(
            kind,
            NodeKind::StateInitial
                | NodeKind::StateFinal
                | NodeKind::StateChoice
                | NodeKind::ActivityInitial
                | NodeKind::ActivityFinal
                | NodeKind::ActivityFlowFinal
                | NodeKind::ActivityDecision
                | NodeKind::ActivityMerge
                | NodeKind::ActivityFork
                | NodeKind::ActivityJoin
        ) {
            Some(String::new())
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
            "ref" if kind == NodeKind::InteractionUse && valid_identifier(value.trim()) => {
                if result.label.is_some() {
                    return Err(error(line, "interaction-use ref cannot also have a label"));
                }
                result.label = Some(value.trim().into());
            }
            "of" if matches!(
                kind,
                NodeKind::ComponentPart | NodeKind::CompositePart | NodeKind::CompositeRole
            ) && valid_reference(value.trim()) =>
            {
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
    if kind == NodeKind::InteractionUse && result.label.is_none() {
        return Err(error(line, "interaction-use requires ref=InteractionName"));
    }
    if matches!(
        kind,
        NodeKind::ComponentPart | NodeKind::CompositePart | NodeKind::CompositeRole
    ) && result.part_type.is_none()
    {
        return Err(error(line, "part requires of=Component"));
    }
    if kind == NodeKind::Object {
        if anonymous && classifier.is_none() {
            return Err(error(line, "anonymous object requires of=Classifier"));
        }
        let name = result.label.take().unwrap_or(name.clone());
        result.label = Some(if let Some(ty) = classifier {
            format!("{name}:{ty}")
        } else {
            name
        });
    } else if !templates.is_empty() {
        let shown = result.label.take().unwrap_or(name.clone());
        result.label = Some(format!(
            "{shown}<{}>",
            template_signature.as_deref().unwrap_or_default()
        ));
    } else if result.label.is_none() && result.id != name {
        result.label = Some(name.clone());
    }
    super::profile::decorate(&mut result, &name, line)?;
    diagram.nodes.push(result);
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
