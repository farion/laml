//! GNE palette creates language elements and optional relative layout intent.
use crate::{
    document::Edit, node_edit, relation_edit::quote, source::SourceIndex, source_patch, style_edit,
};
use laml::syntax::{Diagram, NodeKind, Placement};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementKind {
    #[default]
    Rectangle,
    Ellipse,
    Diamond,
    Note,
    Package,
    Class,
    Interface,
    Enumeration,
    DataType,
    Object,
    Junction,
    Association,
    AssociationClass,
    Component,
    Part,
    Structure,
    CompositePart,
    Role,
    Profile,
    Stereotype,
    Metaclass,
    DeploymentNode,
    Device,
    Environment,
    Artifact,
    State,
    StateInitial,
    StateFinal,
    Choice,
    Action,
    ActivityObject,
    ActivityInitial,
    ActivityFinal,
    FlowFinal,
    Decision,
    Merge,
    Fork,
    Join,
    Actor,
    UseCase,
    System,
    Interaction,
}
impl ElementKind {
    pub const ALL: [Self; 42] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Diamond,
        Self::Note,
        Self::Package,
        Self::Class,
        Self::Interface,
        Self::Enumeration,
        Self::DataType,
        Self::Object,
        Self::Junction,
        Self::Association,
        Self::AssociationClass,
        Self::Component,
        Self::Part,
        Self::Structure,
        Self::CompositePart,
        Self::Role,
        Self::Profile,
        Self::Stereotype,
        Self::Metaclass,
        Self::DeploymentNode,
        Self::Device,
        Self::Environment,
        Self::Artifact,
        Self::State,
        Self::StateInitial,
        Self::StateFinal,
        Self::Choice,
        Self::Action,
        Self::ActivityObject,
        Self::ActivityInitial,
        Self::ActivityFinal,
        Self::FlowFinal,
        Self::Decision,
        Self::Merge,
        Self::Fork,
        Self::Join,
        Self::Actor,
        Self::UseCase,
        Self::System,
        Self::Interaction,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Diamond => "Diamond",
            Self::Note => "Note",
            Self::Package => "Package",
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Enumeration => "Enumeration",
            Self::DataType => "Data type",
            Self::Object => "Object",
            Self::Junction => "Junction",
            Self::Association => "N-ary association",
            Self::AssociationClass => "Association class",
            Self::Component => "Component",
            Self::Part => "Component part",
            Self::Structure => "Composite structure",
            Self::CompositePart => "Composite part",
            Self::Role => "Role",
            Self::Profile => "Profile",
            Self::Stereotype => "Stereotype",
            Self::Metaclass => "Metaclass",
            Self::DeploymentNode => "Deployment node",
            Self::Device => "Device",
            Self::Environment => "Execution environment",
            Self::Artifact => "Artifact",
            Self::State => "State",
            Self::StateInitial => "Initial state",
            Self::StateFinal => "Final state",
            Self::Choice => "Choice",
            Self::Action => "Action",
            Self::ActivityObject => "Object node",
            Self::ActivityInitial => "Initial node",
            Self::ActivityFinal => "Activity final",
            Self::FlowFinal => "Flow final",
            Self::Decision => "Decision",
            Self::Merge => "Merge",
            Self::Fork => "Fork",
            Self::Join => "Join",
            Self::Actor => "Actor",
            Self::UseCase => "Use case",
            Self::System => "System boundary",
            Self::Interaction => "Interaction use",
        }
    }
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Rectangle => "",
            Self::Ellipse => "",
            Self::Diamond => "",
            Self::Note => "note ",
            Self::Package => "package ",
            Self::Class => "class ",
            Self::Interface => "interface ",
            Self::Enumeration => "enum ",
            Self::DataType => "datatype ",
            Self::Object => "object ",
            Self::Junction => "junction ",
            Self::Association => "association ",
            Self::AssociationClass => "association class ",
            Self::Component => "component ",
            Self::Part => "part ",
            Self::Structure => "structure ",
            Self::CompositePart => "part ",
            Self::Role => "role ",
            Self::Profile => "profile ",
            Self::Stereotype => "stereotype ",
            Self::Metaclass => "metaclass ",
            Self::DeploymentNode => "node ",
            Self::Device => "device ",
            Self::Environment => "executionEnvironment ",
            Self::Artifact => "artifact ",
            Self::State => "state ",
            Self::StateInitial => "initial ",
            Self::StateFinal => "final ",
            Self::Choice => "choice ",
            Self::Action => "action ",
            Self::ActivityObject => "objectNode ",
            Self::ActivityInitial => "initial ",
            Self::ActivityFinal => "activityFinal ",
            Self::FlowFinal => "flowFinal ",
            Self::Decision => "decision ",
            Self::Merge => "merge ",
            Self::Fork => "fork ",
            Self::Join => "join ",
            Self::Actor => "actor ",
            Self::UseCase => "usecase ",
            Self::System => "system ",
            Self::Interaction => "interaction-use ",
        }
    }
    pub fn node_kind(self) -> NodeKind {
        match self {
            Self::Rectangle => NodeKind::Generic,
            Self::Ellipse => NodeKind::Generic,
            Self::Diamond => NodeKind::Generic,
            Self::Note => NodeKind::Note,
            Self::Package => NodeKind::Package,
            Self::Class => NodeKind::Class,
            Self::Interface => NodeKind::Interface,
            Self::Enumeration => NodeKind::Enumeration,
            Self::DataType => NodeKind::DataType,
            Self::Object => NodeKind::Object,
            Self::Junction => NodeKind::Junction,
            Self::Association => NodeKind::Association,
            Self::AssociationClass => NodeKind::AssociationClass,
            Self::Component => NodeKind::Component,
            Self::Part => NodeKind::ComponentPart,
            Self::Structure => NodeKind::CompositeStructure,
            Self::CompositePart => NodeKind::CompositePart,
            Self::Role => NodeKind::CompositeRole,
            Self::Profile => NodeKind::Profile,
            Self::Stereotype => NodeKind::Stereotype,
            Self::Metaclass => NodeKind::Metaclass,
            Self::DeploymentNode => NodeKind::DeploymentNode,
            Self::Device => NodeKind::Device,
            Self::Environment => NodeKind::ExecutionEnvironment,
            Self::Artifact => NodeKind::Artifact,
            Self::State => NodeKind::State,
            Self::StateInitial => NodeKind::StateInitial,
            Self::StateFinal => NodeKind::StateFinal,
            Self::Choice => NodeKind::StateChoice,
            Self::Action => NodeKind::ActivityAction,
            Self::ActivityObject => NodeKind::ActivityObject,
            Self::ActivityInitial => NodeKind::ActivityInitial,
            Self::ActivityFinal => NodeKind::ActivityFinal,
            Self::FlowFinal => NodeKind::ActivityFlowFinal,
            Self::Decision => NodeKind::ActivityDecision,
            Self::Merge => NodeKind::ActivityMerge,
            Self::Fork => NodeKind::ActivityFork,
            Self::Join => NodeKind::ActivityJoin,
            Self::Actor => NodeKind::UseCaseActor,
            Self::UseCase => NodeKind::UseCase,
            Self::System => NodeKind::UseCaseSystem,
            Self::Interaction => NodeKind::InteractionUse,
        }
    }
    pub fn shape(self) -> Option<&'static str> {
        match self {
            Self::Rectangle => Some("rectangle"),
            Self::Ellipse => Some("ellipse"),
            Self::Diamond => Some("diamond"),
            Self::Note => Some("note"),
            _ => None,
        }
    }
    pub fn available(source: &str) -> Vec<Self> {
        let notation = crate::notation::Notation::of(source);
        let Ok(graph) = crate::notation::parse_graph(source)
            .or_else(|_| crate::notation::parse_graph(&notation.starter(false)))
        else {
            return vec![];
        };
        if crate::notation::Notation::of(source).c4()
            && crate::notation::Notation::of(source) != crate::notation::Notation::C4Code
        {
            return vec![];
        }
        Self::ALL
            .into_iter()
            .filter(|k| {
                if notation == crate::notation::Notation::Gne
                    && !matches!(
                        k,
                        Self::Rectangle
                            | Self::Ellipse
                            | Self::Diamond
                            | Self::Note
                            | Self::Package
                            | Self::Class
                            | Self::Interface
                            | Self::Enumeration
                            | Self::DataType
                            | Self::Object
                            | Self::Junction
                    )
                {
                    return false;
                }
                if matches!(k, Self::Association | Self::AssociationClass) {
                    return false;
                }
                (k.node_kind() != NodeKind::Generic
                    || graph.settings.diagram_type == laml::syntax::DiagramType::Generic)
                    && laml::parser::allows_node_kind(k.node_kind(), graph.settings.diagram_type)
            })
            .collect()
    }
}

pub fn add(
    source: &str,
    kind: ElementKind,
    label: &str,
    parent: Option<&str>,
    relative: Option<(&str, Placement)>,
) -> Result<(String, Vec<Edit>), String> {
    let diagram = if source.trim().is_empty() {
        Diagram::default()
    } else {
        node_edit::diagram(source)?
    };
    if let Some(parent) = parent
        && !diagram.nodes.iter().any(|n| {
            n.id == parent
                && node_edit::blocks(source, &diagram).is_ok_and(|b| b.contains_key(&n.line))
        })
    {
        return Err("Destination must be a container block".into());
    }
    let prefix = match kind {
        ElementKind::Package => "Group",
        ElementKind::Note => "Note",
        _ => "Node",
    };
    let (name, id) = (1..).map(|i| {
        let name = format!("{prefix}{i}");
        let id = parent.map_or_else(|| name.clone(), |p| format!("{p}.{name}"));
        (name, id)
    }).find(|(name, id)| !diagram.nodes.iter().any(|n| &n.id == id || &n.id == name || n.id.rsplit('.').next() == Some(name.as_str()))
        && !diagram.relations.iter().any(|r| r.name.as_ref() == Some(name))
        && !diagram.attachments.iter().any(|r| r.name.as_ref() == Some(name))
        && !diagram.nodes.iter().any(|n| n.ports.iter().any(|p| p.explicit && &p.id == name)
            || n.parts.iter().any(|p| matches!(p, laml::syntax::NodePart::Member { id: Some(id), .. } if id == name))))
        .unwrap();
    let newline = newline(source);
    let attrs = if label.is_empty() || kind == ElementKind::Junction {
        String::new()
    } else {
        format!("(label={})", quote(label))
    };
    let declaration = match kind {
        ElementKind::Package
        | ElementKind::Profile
        | ElementKind::System
        | ElementKind::Structure
        | ElementKind::DeploymentNode
        | ElementKind::Device
        | ElementKind::Environment => format!("{}{name}{attrs} {{{newline}}}", kind.keyword()),
        ElementKind::Part | ElementKind::CompositePart | ElementKind::Role => {
            let expected = if kind == ElementKind::Part {
                NodeKind::Component
            } else {
                NodeKind::Class
            };
            let classifier = diagram
                .nodes
                .iter()
                .find(|n| n.kind == expected)
                .ok_or("Create a matching classifier first")?;
            format!(
                "{}{name}(of={}; label={})",
                kind.keyword(),
                classifier.id,
                quote(label)
            )
        }
        ElementKind::Interaction => format!(
            "interaction-use {name}(ref=Interaction; label={})",
            quote(label)
        ),
        ElementKind::Note => format!(
            "note {name} {}",
            quote(if label.trim().is_empty() {
                "Note"
            } else {
                label
            })
        ),
        _ => format!("{}{name}{attrs}", kind.keyword()),
    };
    let mut edits = vec![insert(source, &diagram, parent, &declaration)?];
    if let Some(shape) = kind.shape() {
        edits.extend(style_edit::local_property(
            source,
            &[format!("node[{id}]")],
            "shape",
            Some(shape),
        )?);
    }
    if let Some((reference, placement)) = relative {
        let reference_node = diagram
            .nodes
            .iter()
            .find(|n| n.id == reference)
            .ok_or("Missing placement reference")?;
        if reference_node.parent.as_deref() != parent {
            return Err("Relative insertion requires the same container as its reference".into());
        }
        let operator = match placement {
            Placement::Left => "<",
            Placement::Right => ">",
            Placement::Above => "above",
            Placement::Below => "below",
        };
        edits.push(source_patch::insert_section(
            source,
            &diagram,
            "layout",
            &format!("{id} {operator} {reference}"),
        )?);
    }
    let edits = merge_insertions(edits);
    let candidate = node_edit::candidate(source, &edits)?;
    let updated = node_edit::diagram(&candidate)?;
    if diagram.relations.len() != updated.relations.len()
        || diagram
            .relations
            .iter()
            .zip(&updated.relations)
            .any(|(before, after)| before.from != after.from || before.to != after.to)
    {
        return Err(
            "Created element would capture an existing scoped reference. Source unchanged.".into(),
        );
    }
    if !updated.nodes.iter().any(|n| n.id == id) {
        return Err("Created node did not receive the intended ID".into());
    }
    Ok((id, edits))
}

pub(crate) fn newline(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
pub(crate) fn closing_start(source: &str, end: usize) -> usize {
    source[..end]
        .trim_end_matches(['\r', '\n'])
        .rfind('\n')
        .map_or(0, |i| i + 1)
}
pub(crate) fn insert(
    source: &str,
    diagram: &Diagram,
    parent: Option<&str>,
    declaration: &str,
) -> Result<Edit, String> {
    if let Some(parent) = parent {
        let node = diagram
            .nodes
            .iter()
            .find(|n| n.id == parent)
            .ok_or("Missing container")?;
        let blocks = node_edit::blocks(source, diagram)?;
        let block = blocks.get(&node.line).ok_or("Missing destination block")?;
        let position = closing_start(source, block.end);
        let range = SourceIndex::new(source)
            .line(node.line)
            .ok_or("Missing container line")?;
        let line = &source[range];
        let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
        let newline = newline(source);
        let content = declaration
            .replace("\r\n", "\n")
            .lines()
            .map(|line| format!("{indent}    {line}{newline}"))
            .collect();
        Ok(Edit {
            range: position..position,
            replacement: content,
        })
    } else {
        source_patch::insert_section(
            source,
            diagram,
            "semantics",
            &declaration.replace(newline(source), &format!("{}    ", newline(source))),
        )
    }
}

/// Distinct section creations can share EOF. Join them in transaction order.
pub(crate) fn merge_insertions(edits: Vec<Edit>) -> Vec<Edit> {
    let mut result: Vec<Edit> = Vec::new();
    for edit in edits {
        if edit.range.is_empty()
            && let Some(previous) = result.iter_mut().find(|old| old.range == edit.range)
        {
            previous.replacement.push_str(&edit.replacement);
        } else {
            result.push(edit);
        }
    }
    result
}
#[cfg(test)]
mod tests;
