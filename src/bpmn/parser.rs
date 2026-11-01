//! Line-oriented BPMN source parser. Explicitly reject constructs not yet drawable.
use crate::lex::split_top_level;

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
    pub direction: crate::syntax::FlowDirection,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub pools: Vec<Pool>,
    pub layout: Vec<(String, String, String)>,
    pub styling: Vec<crate::syntax::StyleEntry>,
}
#[derive(Clone, Debug, Default)]
struct Context {
    pool: Option<String>,
    lane: Option<String>,
    parent: Option<String>,
}
fn error(line: usize, msg: &str) -> String {
    format!("{line}: {msg}")
}
fn identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn unquote(value: &str, line: usize) -> Result<String, String> {
    let text = value
        .trim()
        .strip_prefix("label=")
        .unwrap_or(value.trim())
        .trim();
    if !text.starts_with('"') || !text.ends_with('"') || text.len() < 2 {
        return Err(error(line, "expected quoted text"));
    }
    let mut out = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                _ => return Err(error(line, "invalid escape in text")),
            }
        } else if c == '"' {
            return Err(error(line, "unescaped quote in text"));
        } else {
            out.push(c);
        }
    }
    Ok(out)
}
fn token_end(s: &str) -> usize {
    let (mut quote, mut escape, mut depth) = (false, false, 0usize);
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
        } else if quote && c == '\\' {
            escape = true;
        } else if c == '"' {
            quote = !quote;
        } else if !quote {
            match c {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ if depth == 0 && c.is_whitespace() => return i,
                _ => {}
            }
        }
    }
    s.len()
}
fn name_args(text: &str, line: usize) -> Result<(String, Option<String>, Option<String>), String> {
    let text = text.trim();
    let (id, args) = if let Some(open) = text.find('(') {
        if !text.ends_with(')') {
            return Err(error(line, "unclosed attributes"));
        }
        (&text[..open], Some(&text[open + 1..text.len() - 1]))
    } else {
        (text, None)
    };
    if !identifier(id) {
        return Err(error(line, "invalid BPMN identifier"));
    }
    let (mut label, mut icon) = (None, None);
    if let Some(args) = args {
        for arg in split_top_level(args, ';') {
            let arg = arg.trim();
            if let Some(value) = arg.strip_prefix("icon=") {
                if icon.is_some() {
                    return Err(error(line, "duplicate icon"));
                }
                icon = Some(unquote(value, line)?);
            } else if label.is_none() {
                label = Some(unquote(arg, line)?);
            } else {
                return Err(error(line, "duplicate or unsupported attribute"));
            }
        }
    }
    Ok((id.into(), label, icon))
}
fn qualified(id: &str, ctx: &Context) -> String {
    let mut parts = Vec::new();
    if let Some(pool) = &ctx.pool {
        parts.push(pool.as_str());
    }
    if let Some(parent) = &ctx.parent {
        parts.push(parent.as_str());
    }
    parts.push(id);
    parts.join(".")
}
fn resolve(doc: &Document, input: &str, ctx: &Context) -> Option<String> {
    let candidate = if input.contains('.') {
        input.into()
    } else {
        qualified(input, ctx)
    };
    if doc.nodes.iter().any(|n| n.id == candidate) {
        return Some(candidate);
    }
    if ctx.parent.is_some() {
        let outer = Context {
            parent: None,
            ..ctx.clone()
        };
        let candidate = qualified(input, &outer);
        if doc.nodes.iter().any(|n| n.id == candidate) {
            return Some(candidate);
        }
    }
    None
}
fn define(
    doc: &mut Document,
    text: &str,
    kind: Kind,
    marker: Option<&str>,
    ctx: &Context,
    line: usize,
) -> Result<String, String> {
    let (name, label, icon) = name_args(text, line)?;
    let id = qualified(&name, ctx);
    if let Some(node) = doc.nodes.iter().find(|n| n.id == id) {
        if node.kind != kind
            || label.is_some()
            || icon.is_some()
            || node.lane != ctx.lane
            || node.parent != ctx.parent
        {
            return Err(error(line, "conflicting BPMN node declaration"));
        }
        return Ok(id);
    }
    if icon
        .as_ref()
        .is_some_and(|i| crate::svg_icons::source(i).is_none())
    {
        return Err(error(line, "unknown bundled Phosphor icon"));
    }
    if icon.is_some()
        && !matches!(
            kind,
            Kind::Task
                | Kind::UserTask
                | Kind::ServiceTask
                | Kind::SendTask
                | Kind::ReceiveTask
                | Kind::ScriptTask
                | Kind::ManualTask
                | Kind::Collapsed
                | Kind::Subprocess
        )
    {
        return Err(error(
            line,
            "free icon not allowed on BPMN event, gateway or data",
        ));
    }
    let default_label = if matches!(
        kind,
        Kind::Start
            | Kind::End
            | Kind::Intermediate
            | Kind::Boundary
            | Kind::Exclusive
            | Kind::Parallel
            | Kind::Inclusive
            | Kind::EventBased
    ) {
        String::new()
    } else {
        name
    };
    doc.nodes.push(Node {
        id: id.clone(),
        label: label.unwrap_or(default_label),
        kind,
        icon,
        marker: marker.map(str::to_owned),
        pool: ctx.pool.clone(),
        lane: ctx.lane.clone(),
        parent: ctx.parent.clone(),
        attached: None,
        interrupting: true,
    });
    Ok(id)
}
fn endpoint(doc: &mut Document, text: &str, ctx: &Context, line: usize) -> Result<String, String> {
    let text = text.trim();
    let typed: &[(&str, Kind, Option<&str>)] = &[
        ("startEvent ", Kind::Start, None),
        ("endEvent ", Kind::End, None),
        ("messageStart ", Kind::Start, Some("message")),
        ("timerStart ", Kind::Start, Some("timer")),
        ("messageEnd ", Kind::End, Some("message")),
        ("errorEnd ", Kind::End, Some("error")),
        ("terminateEnd ", Kind::End, Some("terminate")),
        ("intermediateTimer ", Kind::Intermediate, Some("timer")),
        ("intermediateMessage ", Kind::Intermediate, Some("message")),
        ("userTask ", Kind::UserTask, None),
        ("serviceTask ", Kind::ServiceTask, None),
        ("sendTask ", Kind::SendTask, None),
        ("receiveTask ", Kind::ReceiveTask, None),
        ("scriptTask ", Kind::ScriptTask, None),
        ("manualTask ", Kind::ManualTask, None),
        ("data ", Kind::Data, None),
        ("store ", Kind::Store, None),
        ("note ", Kind::Annotation, None),
        ("<X>@", Kind::Exclusive, None),
        ("<+>@", Kind::Parallel, None),
        ("<O>@", Kind::Inclusive, None),
        ("<event>@", Kind::EventBased, None),
        ("+", Kind::Collapsed, None),
    ];
    for &(prefix, kind, marker) in typed {
        if let Some(rest) = text.strip_prefix(prefix) {
            return define(doc, rest, kind, marker, ctx, line);
        }
    }
    if identifier(text) || text.split('.').all(identifier) {
        if let Some(id) = resolve(doc, text, ctx) {
            return Ok(id);
        }
        if text.contains('.') {
            return Err(error(line, "unknown qualified BPMN reference"));
        }
    }
    define(doc, text, Kind::Task, None, ctx, line)
}
fn add_edge(
    doc: &mut Document,
    from: String,
    to: String,
    kind: EdgeKind,
    label: Option<String>,
    line: usize,
) -> Result<(), String> {
    let a = doc.nodes.iter().find(|n| n.id == from).unwrap();
    let b = doc.nodes.iter().find(|n| n.id == to).unwrap();
    if from == to {
        return Err(error(line, "self flow is not a BPMN connection"));
    }
    match kind {
        EdgeKind::Sequence => {
            if a.pool != b.pool || a.parent != b.parent {
                return Err(error(
                    line,
                    "sequence flow cannot cross pool or subprocess boundary",
                ));
            }
            if a.kind == Kind::End
                || matches!(b.kind, Kind::Start | Kind::Boundary)
                || matches!(a.kind, Kind::Data | Kind::Store | Kind::Annotation)
                || matches!(b.kind, Kind::Data | Kind::Store | Kind::Annotation)
            {
                return Err(error(line, "invalid sequence-flow endpoint"));
            }
            if matches!(a.kind, Kind::Parallel | Kind::EventBased) && label.is_some() {
                return Err(error(
                    line,
                    "parallel and event-based gateways do not have conditional branches",
                ));
            }
        }
        EdgeKind::Message => {
            if a.pool.is_none()
                || b.pool.is_none()
                || a.pool == b.pool
                || a.parent.is_some()
                || b.parent.is_some()
            {
                return Err(error(
                    line,
                    "message flow requires separate pools and visible endpoints",
                ));
            }
            if matches!(a.kind, Kind::Data | Kind::Store | Kind::Annotation)
                || matches!(b.kind, Kind::Data | Kind::Store | Kind::Annotation)
            {
                return Err(error(line, "message flow cannot connect data objects"));
            }
        }
        EdgeKind::Association => {
            if a.pool != b.pool
                || a.parent != b.parent
                || (a.kind == Kind::Annotation) == (b.kind == Kind::Annotation)
            {
                return Err(error(
                    line,
                    "text annotation association needs one note in the same scope",
                ));
            }
        }
        EdgeKind::Data => {
            if a.kind == Kind::Annotation || b.kind == Kind::Annotation {
                return Err(error(line, "text annotation needs '..' without arrow"));
            }
            if a.pool != b.pool
                || a.parent != b.parent
                || !(matches!(a.kind, Kind::Data | Kind::Store)
                    || matches!(b.kind, Kind::Data | Kind::Store))
            {
                return Err(error(
                    line,
                    "data association needs data and stays within its scope",
                ));
            }
        }
    }
    doc.edges.push(Edge {
        from,
        to,
        kind,
        label,
    });
    Ok(())
}
mod document;
mod flow;
pub use document::{is_bpmn_source, parse};

#[cfg(test)]
mod tests {
    use super::*;
    fn wrap(body: &str) -> String {
        format!("settings {{\ntype: bpmn\n}}\nsemantics {{\n{body}\n}}")
    }
    #[test]
    fn split_join_and_labels() {
        let d = parse(&wrap("startEvent Start\nendEvent End\nStart --> Review(\"Review\")\nReview <X>@Decision Yes:yes ; No:no\nYes --> <X>@Merge --> End\nNo --> Merge")).unwrap();
        assert_eq!(
            d.nodes.iter().filter(|n| n.kind == Kind::Exclusive).count(),
            2
        );
        assert_eq!(d.edges.len(), 7);
        assert_eq!(d.edges[2].label.as_deref(), Some("yes"));
    }
    #[test]
    fn pools_lanes_and_messages() {
        let d = parse(&wrap("pool Customer {\nstartEvent Start\nStart --> Send\n}\npool Office {\nlane Sales {\nmessageStart Received\nReceived --> Review\n}\n}\nCustomer.Send ~~> Office.Received : application")).unwrap();
        assert_eq!(d.pools.len(), 2);
        assert_eq!(d.edges.last().unwrap().kind, EdgeKind::Message);
        assert!(
            d.nodes
                .iter()
                .any(|n| n.id == "Office.Review" && n.lane.as_deref() == Some("Sales"))
        );
        assert!(parse(&wrap("pool P {\nA ~~> B\n}")).is_err());
    }
    #[test]
    fn subprocess_boundary_data_and_icon() {
        let d = parse(&wrap("pool P {\nstartEvent Start\nStart --> Review(label=\"Check\"; icon=\"folder\")\nReview -(timer@Deadline:\"2 days\")-> Escalate\ndata Paper\nPaper ..> Review\n+Check(\"Collapsed\")\nOpen(\"Expanded\") {\nstartEvent InnerStart\nendEvent InnerEnd\nInnerStart --> InnerEnd\n}\n}")).unwrap();
        assert_eq!(
            d.nodes.iter().filter(|n| n.kind == Kind::Boundary).count(),
            1
        );
        assert!(d.nodes.iter().any(|n| n.kind == Kind::Subprocess));
        assert!(
            crate::bpmn::render(&d, crate::svg::TextMode::System)
                .unwrap()
                .contains(crate::export_metadata::MARKER)
        );
    }
    #[test]
    fn mandatory_layout_conflicts_fail_and_stay_in_lane() {
        let mut d = parse(&wrap("pool P {\nlane L {\nA --> B\n}\n}")).unwrap();
        d.layout.push(("A".into(), "<".into(), "B".into()));
        d.layout.push(("B".into(), "<".into(), "A".into()));
        assert!(crate::bpmn::validate_geometry(&d).is_err());
        d.layout.pop();
        d.layout.push(("A".into(), "below".into(), "B".into()));
        assert!(crate::bpmn::validate_geometry(&d).is_ok());
        let mut other = parse(&wrap("pool P {\nlane One {\nA\n}\nlane Two {\nB\n}\n}")).unwrap();
        other.layout.push(("A".into(), "<".into(), "B".into()));
        assert!(crate::bpmn::validate_geometry(&other).is_err());
    }
    #[test]
    fn marker_like_text_inside_a_label_is_not_a_gateway() {
        let d = parse(&wrap(
            "startEvent Start\nStart --> Review(\"Check <X> marker\")\nReview --> endEvent Done",
        ))
        .unwrap();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
    }
    #[test]
    fn event_gateway_and_noninterrupting_boundary_have_checked_meaning() {
        let d=parse(&wrap("startEvent Start\nintermediateTimer Tick\nreceiveTask Message\nStart <event>@Wait Tick ; Message\nMessage -(message+@Notice:\"incoming\")-> FollowUp")).unwrap();
        let boundary = d.nodes.iter().find(|n| n.kind == Kind::Boundary).unwrap();
        assert!(!boundary.interrupting);
        assert_eq!(boundary.marker.as_deref(), Some("message"));
        assert!(parse(&wrap("A <event> B ; C")).is_err());
        assert!(parse(&wrap("A <+> B:yes ; C:no")).is_err());
    }
    #[test]
    fn reject_semantic_crossings_and_redefinitions() {
        assert!(parse(&wrap("pool A {\nX\n}\npool B {\nY\n}\nA.X --> B.Y")).is_err());
        assert!(parse(&wrap("A --> B\nB(\"redefined\") --> C")).is_err());
        assert!(parse(&wrap("A <event> B:yes ; C:no")).is_err());
    }
}
