//! CSS-like LSS selectors and contextual sequence visual parts.
use crate::syntax::MemberKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    All,
    AllNodes,
    AllRelations,
    AllMembers,
    AllPorts,
    AllLifelines,
    AllActivations,
    AllFragments,
    AllNotes,
    AllCoregions,
    AllContinuations,
    AllAnnotations,
    AllOrderings,
    AllGates,
    Class(String),
    NodeClass(String),
    RelationClass(String),
    MemberClass(String),
    PortClass(String),
    MemberKind(MemberKind),
    NodeId(String),
    MemberId(String),
    PortId(String),
    RelationId(String),
    LifelineId(String),
    ActivationId(String),
    FragmentId(String),
    FragmentOperator(String),
    NoteId(String),
    CoregionId(String),
    ContinuationId(String),
    AnnotationKind(String),
    GateId(String),
    NodeClassLifeline(String),
    NodeClassActivation(String),
    NodeClassCoregion(String),
    NodeIdLifeline(String),
    NodeIdActivation(String),
    NodeIdNamedActivation {
        owner: String,
        name: String,
    },
    NodeIdCoregion(String),
    NodeIdDescendantNode {
        owner: String,
        child: Option<String>,
    },
    NodeClassDescendantNode {
        class: String,
        child: Option<String>,
    },
    ScopedId {
        owner: String,
        child: Box<Selector>,
    },
    ScopedClass {
        class: String,
        child: Box<Selector>,
    },
}

impl Selector {
    pub(super) fn targets_node(&self) -> bool {
        match self {
            Self::AllNodes
            | Self::NodeId(_)
            | Self::NodeClass(_)
            | Self::NodeIdDescendantNode { .. }
            | Self::NodeClassDescendantNode { .. } => true,
            Self::ScopedId { child, .. } | Self::ScopedClass { child, .. } => child.targets_node(),
            _ => false,
        }
    }

    pub(super) fn targets_relation(&self) -> bool {
        match self {
            Self::AllRelations | Self::RelationId(_) | Self::RelationClass(_) => true,
            Self::ScopedId { child, .. } | Self::ScopedClass { child, .. } => {
                child.targets_relation()
            }
            _ => false,
        }
    }

    pub(super) fn targets_port(&self) -> bool {
        match self {
            Self::AllPorts | Self::PortId(_) | Self::PortClass(_) => true,
            Self::ScopedId { child, .. } | Self::ScopedClass { child, .. } => child.targets_port(),
            _ => false,
        }
    }

    pub(super) fn targets_member(&self) -> bool {
        match self {
            Self::AllMembers | Self::MemberId(_) | Self::MemberClass(_) | Self::MemberKind(_) => {
                true
            }
            Self::ScopedId { child, .. } | Self::ScopedClass { child, .. } => {
                child.targets_member()
            }
            _ => false,
        }
    }

    pub(super) fn parse(text: &str) -> Self {
        if let Some((parent, child)) = text.split_once(' ') {
            let child_node = child
                .strip_prefix("node[")
                .and_then(|name| name.strip_suffix(']'));
            if let Some(class) = parent.strip_prefix("node.") {
                if child == "node" || child_node.is_some() {
                    return Self::NodeClassDescendantNode {
                        class: class.into(),
                        child: child_node.map(str::to_string),
                    };
                }
                if !matches!(child, "lifeline" | "activation" | "coregion") {
                    return Self::ScopedClass {
                        class: class.into(),
                        child: Box::new(Self::parse(child)),
                    };
                }
                return if child == "lifeline" {
                    Self::NodeClassLifeline(class.into())
                } else if child == "activation" {
                    Self::NodeClassActivation(class.into())
                } else {
                    Self::NodeClassCoregion(class.into())
                };
            }
            let id = parent.trim_start_matches("node[").trim_end_matches(']');
            if child == "node" || child_node.is_some() {
                return Self::NodeIdDescendantNode {
                    owner: id.into(),
                    child: child_node.map(str::to_string),
                };
            }
            if let Some(name) = child
                .strip_prefix("activation[")
                .and_then(|name| name.strip_suffix(']'))
            {
                return Self::NodeIdNamedActivation {
                    owner: id.into(),
                    name: name.into(),
                };
            }
            if !matches!(child, "lifeline" | "activation" | "coregion") {
                return Self::ScopedId {
                    owner: id.into(),
                    child: Box::new(Self::parse(child)),
                };
            }
            return if child == "lifeline" {
                Self::NodeIdLifeline(id.into())
            } else if child == "activation" {
                Self::NodeIdActivation(id.into())
            } else {
                Self::NodeIdCoregion(id.into())
            };
        }
        if text == "*" {
            Self::All
        } else if let Some(class) = text.strip_prefix("node.") {
            Self::NodeClass(class.into())
        } else if let Some(class) = text.strip_prefix("relation.") {
            Self::RelationClass(class.into())
        } else if let Some(class) = text.strip_prefix("member.") {
            Self::MemberClass(class.into())
        } else if let Some(class) = text.strip_prefix("port.") {
            Self::PortClass(class.into())
        } else if let Some(class) = text.strip_prefix('.') {
            Self::Class(class.into())
        } else if let Some(id) = text.strip_prefix("node[") {
            Self::NodeId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("relation[") {
            Self::RelationId(id.trim_end_matches(']').into())
        } else if let Some(kind) = text.strip_prefix("member[kind=") {
            Self::MemberKind(match kind.trim_end_matches(']') {
                "field" => MemberKind::Field,
                "method" => MemberKind::Method,
                "slot" => MemberKind::Slot,
                "literal" => MemberKind::Literal,
                _ => MemberKind::Text,
            })
        } else if let Some(id) = text.strip_prefix("member[") {
            Self::MemberId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("port[") {
            Self::PortId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("lifeline[") {
            Self::LifelineId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("activation[") {
            Self::ActivationId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("fragment[") {
            let id = id.trim_end_matches(']');
            if matches!(
                id,
                "alt"
                    | "opt"
                    | "loop"
                    | "par"
                    | "break"
                    | "critical"
                    | "neg"
                    | "assert"
                    | "strict"
                    | "seq"
                    | "ignore"
                    | "consider"
                    | "ref"
            ) {
                Self::FragmentOperator(id.into())
            } else {
                Self::FragmentId(id.into())
            }
        } else if let Some(id) = text.strip_prefix("note[") {
            Self::NoteId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("coregion[") {
            Self::CoregionId(id.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("continuation[") {
            Self::ContinuationId(id.trim_end_matches(']').into())
        } else if let Some(kind) = text.strip_prefix("annotation[") {
            Self::AnnotationKind(kind.trim_end_matches(']').into())
        } else if let Some(id) = text.strip_prefix("gate[") {
            Self::GateId(id.trim_end_matches(']').into())
        } else {
            match text {
                "node" => Self::AllNodes,
                "relation" => Self::AllRelations,
                "member" => Self::AllMembers,
                "port" => Self::AllPorts,
                "lifeline" => Self::AllLifelines,
                "activation" => Self::AllActivations,
                "fragment" => Self::AllFragments,
                "note" => Self::AllNotes,
                "coregion" => Self::AllCoregions,
                "continuation" => Self::AllContinuations,
                "annotation" => Self::AllAnnotations,
                "ordering" => Self::AllOrderings,
                "gate" => Self::AllGates,
                _ => Self::AllRelations, // parser validated syntax
            }
        }
    }

    pub(super) fn specificity(&self) -> u8 {
        match self {
            Self::All
            | Self::AllNodes
            | Self::AllRelations
            | Self::AllMembers
            | Self::AllPorts
            | Self::AllLifelines
            | Self::AllActivations
            | Self::AllFragments
            | Self::AllNotes
            | Self::AllCoregions
            | Self::AllContinuations
            | Self::AllAnnotations
            | Self::AllOrderings
            | Self::AllGates => 0,
            Self::Class(_) => 1,
            Self::NodeClass(_)
            | Self::RelationClass(_)
            | Self::MemberClass(_)
            | Self::PortClass(_)
            | Self::MemberKind(_)
            | Self::NodeClassLifeline(_)
            | Self::NodeClassActivation(_)
            | Self::NodeClassCoregion(_)
            | Self::NodeClassDescendantNode { child: None, .. } => 2,
            Self::NodeClassDescendantNode { child: Some(_), .. } => 4,
            Self::FragmentOperator(_) => 2,
            Self::NodeId(_)
            | Self::RelationId(_)
            | Self::MemberId(_)
            | Self::PortId(_)
            | Self::LifelineId(_)
            | Self::ActivationId(_)
            | Self::FragmentId(_)
            | Self::NoteId(_)
            | Self::CoregionId(_)
            | Self::ContinuationId(_)
            | Self::AnnotationKind(_)
            | Self::GateId(_)
            | Self::NodeIdCoregion(_)
            | Self::NodeIdLifeline(_)
            | Self::NodeIdActivation(_) => 3,
            Self::NodeIdNamedActivation { .. } | Self::NodeIdDescendantNode { child: None, .. } => {
                4
            }
            Self::NodeIdDescendantNode { child: Some(_), .. } => 5,
            Self::ScopedId { child, .. } => 3 + child.specificity(),
            Self::ScopedClass { child, .. } => 2 + child.specificity(),
        }
    }

    pub(super) fn matches(&self, element: &Element<'_>, ancestors: &[String]) -> bool {
        match (self, element) {
            (Self::All, _) => true,
            (Self::AllNodes, Element::Node { .. })
            | (Self::AllRelations, Element::Relation { .. })
            | (Self::AllMembers, Element::Member { .. })
            | (Self::AllPorts, Element::Port { .. })
            | (Self::AllLifelines, Element::Lifeline { .. })
            | (Self::AllActivations, Element::Activation { .. })
            | (Self::AllFragments, Element::Fragment { .. })
            | (Self::AllNotes, Element::Note { .. })
            | (Self::AllCoregions, Element::Coregion { .. })
            | (Self::AllContinuations, Element::Continuation { .. })
            | (Self::AllAnnotations, Element::Annotation { .. })
            | (Self::AllOrderings, Element::Ordering)
            | (Self::AllGates, Element::Gate { .. }) => true,
            (
                Self::Class(class),
                Element::Node { classes, .. } | Element::Relation { classes, .. },
            ) => classes.iter().any(|c| c == class),
            (Self::NodeClass(class), Element::Node { classes, .. })
            | (Self::RelationClass(class), Element::Relation { classes, .. })
            | (Self::MemberClass(class), Element::Member { classes, .. })
            | (Self::PortClass(class), Element::Port { classes, .. }) => {
                classes.iter().any(|c| c == class)
            }
            (Self::NodeId(target), Element::Node { id, .. }) => target == id,
            (Self::MemberId(target), Element::Member { id: Some(id), .. }) => target == id,
            (Self::PortId(target), Element::Port { id, .. }) => target == id,
            (Self::MemberKind(target), Element::Member { kind, .. }) => target == kind,
            (Self::ScopedId { owner, child }, _) => {
                ancestors.iter().any(|ancestor| ancestor == owner)
                    && child.matches(element, ancestors)
            }
            (Self::ScopedClass { class, child }, _) => {
                ancestors
                    .iter()
                    .any(|ancestor| ancestor.starts_with(&format!(".{class}:")))
                    && child.matches(element, ancestors)
            }
            (Self::NodeIdDescendantNode { owner, child }, Element::Node { id, .. }) => {
                ancestors.iter().any(|ancestor| ancestor == owner)
                    && child.as_deref().is_none_or(|name| {
                        name == *id
                            || id
                                .strip_prefix(&format!("{owner}."))
                                .is_some_and(|relative| relative == name)
                    })
            }
            (Self::NodeClassDescendantNode { class, child }, Element::Node { id, .. }) => {
                ancestors
                    .iter()
                    .any(|ancestor| ancestor.starts_with(&format!(".{class}:")))
                    && child
                        .as_deref()
                        .is_none_or(|name| name == *id || id.rsplit('.').next() == Some(name))
            }
            (Self::RelationId(target), Element::Relation { name: Some(id), .. }) => target == id,
            (Self::FragmentOperator(target), Element::Fragment { operator, .. }) => {
                target == operator
            }
            (Self::FragmentId(target), Element::Fragment { id: Some(id), .. }) => target == id,
            (Self::AnnotationKind(target), Element::Annotation { kind }) => target == kind,
            (Self::GateId(target), Element::Gate { id }) => target == id,
            (Self::LifelineId(target), Element::Lifeline { id, .. })
            | (Self::ActivationId(target), Element::Activation { id, .. })
            | (Self::NoteId(target), Element::Note { id })
            | (Self::CoregionId(target), Element::Coregion { id, .. })
            | (Self::ContinuationId(target), Element::Continuation { id }) => target == id,
            (Self::NodeClassLifeline(class), Element::Lifeline { parent_classes, .. })
            | (Self::NodeClassActivation(class), Element::Activation { parent_classes, .. })
            | (Self::NodeClassCoregion(class), Element::Coregion { parent_classes, .. }) => {
                parent_classes.iter().any(|name| name == class)
                    || ancestors
                        .iter()
                        .any(|ancestor| ancestor.starts_with(&format!(".{class}:")))
            }
            (
                Self::NodeIdNamedActivation { owner, name },
                Element::Activation { id, local_id, .. },
            ) => {
                (owner == id || ancestors.iter().any(|ancestor| ancestor == owner))
                    && local_id.is_some_and(|actual| actual == name)
            }
            (Self::NodeIdLifeline(target), Element::Lifeline { id, .. })
            | (Self::NodeIdActivation(target), Element::Activation { id, .. })
            | (Self::NodeIdCoregion(target), Element::Coregion { id, .. }) => {
                target == id || ancestors.iter().any(|ancestor| ancestor == target)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Element<'a> {
    Node {
        id: &'a str,
        classes: &'a [String],
    },
    Relation {
        name: Option<&'a str>,
        classes: &'a [String],
    },
    Member {
        id: Option<&'a str>,
        classes: &'a [String],
        kind: MemberKind,
    },
    Port {
        id: &'a str,
        classes: &'a [String],
    },
    Lifeline {
        id: &'a str,
        parent_classes: &'a [String],
    },
    Activation {
        id: &'a str,
        local_id: Option<&'a str>,
        parent_classes: &'a [String],
    },
    Fragment {
        operator: &'a str,
        id: Option<&'a str>,
    },
    Note {
        id: &'a str,
    },
    Coregion {
        id: &'a str,
        parent_classes: &'a [String],
    },
    Continuation {
        id: &'a str,
    },
    Annotation {
        kind: &'a str,
    },
    Ordering,
    Gate {
        id: &'a str,
    },
}
