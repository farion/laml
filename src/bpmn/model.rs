//! BPMN drawing elements, flows and explicit attachment overrides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Task,
    UserTask,
    ServiceTask,
    SendTask,
    ReceiveTask,
    ScriptTask,
    ManualTask,
    Start,
    End,
    Intermediate,
    Boundary,
    Exclusive,
    Parallel,
    Inclusive,
    EventBased,
    Subprocess,
    Collapsed,
    Data,
    Store,
    Annotation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Sequence,
    Message,
    Data,
    Association,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub icon: Option<String>,
    pub marker: Option<String>,
    pub pool: Option<String>,
    pub lane: Option<String>,
    pub parent: Option<String>,
    pub attached: Option<String>,
    pub interrupting: bool,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: EdgeKind,
}
#[derive(Clone, Debug)]
pub struct Lane {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug)]
pub struct Pool {
    pub id: String,
    pub label: String,
    pub lanes: Vec<Lane>,
}
#[derive(Clone, Debug, Default)]
pub struct Document {
    pub theme: crate::theme::Theme,
    pub direction: crate::syntax::FlowDirection,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub pools: Vec<Pool>,
    pub layout: Vec<(String, String, String)>,
    pub styling: Vec<crate::syntax::StyleEntry>,
    pub flow_layout: Vec<FlowPortRule>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelOrientation {
    Auto,
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug)]
pub struct FlowPortRule {
    pub from: String,
    pub to: String,
    pub from_side: Option<Side>,
    pub to_side: Option<Side>,
    pub label_orientation: Option<LabelOrientation>,
}
