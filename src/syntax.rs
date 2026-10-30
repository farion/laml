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
        classes: Vec<String>,
        text: String,
        kind: MemberKind,
        abstract_member: bool,
        static_member: bool,
        line: usize,
    },
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
    pub kind: NodeKind,
    pub abstract_node: bool,
    pub templates: Vec<String>,
    pub parts: Vec<NodePart>,
    pub line: usize,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationDecl {
    pub name: Option<String>,
    pub classes: Vec<String>,
    /// The semantic source; `A <- B` is normalized to `from=B, to=A`.
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub appearance: EdgeAppearance,
    pub source_end: EndAnnotation,
    pub target_end: EndAnnotation,
    pub label: Option<String>,
    pub line: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationLabelRule {
    pub name: String,
    pub placement: LabelPlacement,
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
    pub layout: Vec<LayoutRule>,
    pub relation_layout: Vec<RelationLabelRule>,
    /// Source order is preserved so later matching style rules can override earlier ones.
    /// Imports are parsed but not resolved by this parser.
    pub styling: Vec<StyleEntry>,
}
