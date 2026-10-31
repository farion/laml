//! Parsed language model. Geometry and SVG rendering are separate phases.

use crate::RelationKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelPlacement {
    #[default]
    Near,
    Along,
    Inline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiagramType {
    #[default]
    Generic,
    Class,
    Object,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeKind {
    #[default]
    Generic,
    Class,
    Interface,
    Enumeration,
    DataType,
    Package,
    Association,
    AssociationClass,
    Junction,
    /// Render-only interface realization ball; never accepted as a source declaration.
    Lollipop,
    Object,
    Note,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberKind {
    Text,
    Field,
    Method,
    Slot,
    Literal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodePart {
    Separator,
    Member {
        id: Option<String>,
        default_id: Option<String>,
        classes: Vec<String>,
        text: String,
        kind: MemberKind,
        abstract_member: bool,
        static_member: bool,
        line: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortDecl {
    pub id: String,
    pub label: String,
    pub explicit: bool,
    pub classes: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub direction: FlowDirection,
    pub strict_case: bool,
    pub relation_label: LabelPlacement,
    pub diagram_type: DiagramType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDecl {
    pub id: String,
    pub label: Option<String>,
    pub classes: Vec<String>,
    pub stereotypes: Vec<String>,
    pub kind: NodeKind,
    pub abstract_node: bool,
    pub visibility: Option<char>,
    pub templates: Vec<String>,
    /// Raw lines for a note block; finalized into `label` when its block closes.
    pub note_lines: Option<Vec<String>>,
    pub parts: Vec<NodePart>,
    pub ports: Vec<PortDecl>,
    pub parent: Option<String>,
    pub line: usize,
}

impl NodeDecl {
    pub fn matches_port(&self, port: &PortDecl, target: &str) -> bool {
        if port.explicit {
            port.id == target
        } else {
            target.strip_prefix(&format!("{}::", self.id)) == Some(port.id.as_str())
        }
    }

    pub fn matches_member(&self, part: &NodePart, target: &str) -> bool {
        match part {
            NodePart::Member { id: Some(id), .. } => id == target,
            NodePart::Member {
                id: None,
                default_id: Some(name),
                ..
            } => target.strip_prefix(&format!("{}::", self.id)) == Some(name.as_str()),
            _ => false,
        }
    }

    pub fn has_endpoint(&self, target: &str) -> bool {
        self.id == target
            || self
                .parts
                .iter()
                .any(|part| self.matches_member(part, target))
            || self
                .ports
                .iter()
                .any(|port| self.matches_port(port, target))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EndSymbol {
    #[default]
    None,
    Arrow,
    Triangle,
    OpenDiamond,
    FilledDiamond,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeAppearance {
    pub start: EndSymbol,
    pub end: EndSymbol,
    pub dashed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EndAnnotation {
    pub multiplicity: Option<String>,
    pub role: Option<String>,
    pub qualifier: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDecl {
    pub name: Option<String>,
    pub classes: Vec<String>,
    pub stereotypes: Vec<String>,
    pub binding: Option<Vec<(String, String)>>,
    /// The semantic source; `A <- B` is normalized to `from=B, to=A`.
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub appearance: EdgeAppearance,
    pub source_end: EndAnnotation,
    pub target_end: EndAnnotation,
    pub label: Option<String>,
    pub scope: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeAttachment {
    pub name: Option<String>,
    pub classes: Vec<String>,
    pub source: String,
    pub target: String,
    pub scope: Option<String>,
    pub line: usize,
}

impl RelationDecl {
    pub fn display_label(&self) -> Option<String> {
        if let Some(bindings) = &self.binding {
            return Some(format!(
                "«bind» {}",
                bindings
                    .iter()
                    .map(|(formal, actual)| format!("{formal}->{actual}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if self.stereotypes.is_empty() {
            return self.label.clone();
        }
        let marker = self
            .stereotypes
            .iter()
            .map(|name| format!("«{name}»"))
            .collect::<Vec<_>>()
            .join(" ");
        Some(match &self.label {
            Some(label) => format!("{marker}: {label}"),
            None => marker,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Left,
    Right,
    Above,
    Below,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Top,
    Center,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutRule {
    pub first: String,
    pub second: String,
    pub placement: Placement,
    pub align: Option<Alignment>,
    pub gap: Option<f64>,
    pub line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScopedAlignment {
    pub first: String,
    pub second: String,
    pub axis: Axis,
    pub delta: f64,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationLabelRule {
    pub name: String,
    pub placement: LabelPlacement,
    pub line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndSide {
    Above,
    Below,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndPosition {
    Source,
    Target,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortLayoutRule {
    pub id: String,
    pub side: PortSide,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationEndRule {
    pub name: String,
    pub end: EndPosition,
    pub side: Option<PortSide>,
    pub multiplicity: Option<EndSide>,
    pub role: Option<EndSide>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleRule {
    pub selector: String,
    pub properties: Vec<Property>,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleEntry {
    Import { path: String, line: usize },
    Rule(StyleRule),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diagram {
    pub settings: Settings,
    pub nodes: Vec<NodeDecl>,
    pub relations: Vec<RelationDecl>,
    pub attachments: Vec<EdgeAttachment>,
    pub layout: Vec<LayoutRule>,
    pub scoped_alignments: Vec<ScopedAlignment>,
    pub relation_layout: Vec<RelationLabelRule>,
    pub relation_end_layout: Vec<RelationEndRule>,
    pub port_layout: Vec<PortLayoutRule>,
    /// Source order is preserved so later matching style rules can override earlier ones.
    /// Imports are parsed but not resolved by this parser.
    pub styling: Vec<StyleEntry>,
}
