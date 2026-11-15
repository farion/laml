//! Authoring recipes use existing compiler syntax; no editor-only language.
use crate::notation::Notation;
#[derive(Clone, Copy)]
pub struct Recipe {
    pub title: &'static str,
    pub section: &'static str,
    pub body: &'static str,
    pub layout: &'static str,
}
const fn r(title: &'static str, section: &'static str, body: &'static str) -> Recipe {
    Recipe {
        title,
        section,
        body,
        layout: "",
    }
}
const fn bundle(title: &'static str, body: &'static str, layout: &'static str) -> Recipe {
    Recipe {
        title,
        section: "semantics",
        body,
        layout,
    }
}
mod extended;
pub fn recipes(ty: Notation) -> Vec<Recipe> {
    use Notation::*;
    let mut out = match ty {
        Sequence => vec![
            r("Participant", "semantics", "{id}(label={label:q})"),
            r(
                "Synchronous message",
                "semantics",
                "{from} -> {to} : {label:q}",
            ),
            r(
                "Asynchronous message",
                "semantics",
                "{from} ->> {to} : {label:q}",
            ),
            r("Reply", "semantics", "<-- : {label:q}"),
            r(
                "Call with activation",
                "semantics",
                "{from} -> {to}+ : {label:q}\n<-- : {reply:q}",
            ),
            r(
                "Activation",
                "semantics",
                "{id}: {participant}+ : {label:q}\n{id}: {participant}-",
            ),
            r(
                "Create participant",
                "semantics",
                "{from} -> {id}+ : {label:q}",
            ),
            r("Destroy participant", "semantics", "{participant}!"),
            r(
                "Alternative",
                "semantics",
                "alt {condition:q} {\n    {from} ->> {to} : {label:q}\n}\nelse {otherwise:q} {\n    {from} ->> {to} : {alternative:q}\n}",
            ),
            r(
                "Loop",
                "semantics",
                "loop(min={minimum}; max={maximum}; guard={condition:q}) {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Optional fragment",
                "semantics",
                "opt {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Parallel fragment",
                "semantics",
                "par {condition:q} {\n    {from} ->> {to} : {label:q}\n}\nand {otherwise:q} {\n    {from} ->> {to} : {alternative:q}\n}",
            ),
            r(
                "Critical fragment",
                "semantics",
                "critical {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Break fragment",
                "semantics",
                "break {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Negative fragment",
                "semantics",
                "neg {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Assertion fragment",
                "semantics",
                "assert {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Strict fragment",
                "semantics",
                "strict {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r(
                "Weak sequencing fragment",
                "semantics",
                "seq {condition:q} {\n    {from} ->> {to} : {label:q}\n}",
            ),
            r("Interaction reference", "semantics", "ref {label:q}"),
            r("Participant order", "layout", "{from} < {to} : gap={gap}"),
        ],
        Timing => vec![
            r(
                "Lifeline",
                "semantics",
                "lifeline {id}(of={classifier}) : {state}",
            ),
            r(
                "State change",
                "semantics",
                "state {participant} at {time} : {state}",
            ),
            r(
                "Event",
                "semantics",
                "event {participant} at {time} : {label}",
            ),
            r("Ruler tick", "layout", "tick {time}"),
            r("Time scale", "layout", "scale: {scale}"),
            r("Lane spacing", "layout", "lane-gap: {gap}"),
            r("Timeline end", "layout", "end: {time}"),
        ],
        Bpmn => vec![
            r("Task", "semantics", "{id}(label={label:q})"),
            r("User task", "semantics", "userTask {id}(label={label:q})"),
            r(
                "Service task",
                "semantics",
                "serviceTask {id}(label={label:q})",
            ),
            r("Send task", "semantics", "sendTask {id}(label={label:q})"),
            r(
                "Receive task",
                "semantics",
                "receiveTask {id}(label={label:q})",
            ),
            r(
                "Script task",
                "semantics",
                "scriptTask {id}(label={label:q})",
            ),
            r(
                "Manual task",
                "semantics",
                "manualTask {id}(label={label:q})",
            ),
            r(
                "Start event",
                "semantics",
                "startEvent {id}(label={label:q})",
            ),
            r("End event", "semantics", "endEvent {id}(label={label:q})"),
            r(
                "Message start",
                "semantics",
                "messageStart {id}(label={label:q})",
            ),
            r(
                "Timer start",
                "semantics",
                "timerStart {id}(label={label:q})",
            ),
            r(
                "Message end",
                "semantics",
                "messageEnd {id}(label={label:q})",
            ),
            r("Error end", "semantics", "errorEnd {id}(label={label:q})"),
            r(
                "Terminate end",
                "semantics",
                "terminateEnd {id}(label={label:q})",
            ),
            r(
                "Intermediate timer",
                "semantics",
                "intermediateTimer {id}(label={label:q})",
            ),
            r(
                "Intermediate message",
                "semantics",
                "intermediateMessage {id}(label={label:q})",
            ),
            r("Exclusive gateway", "semantics", "<X>@{id}"),
            r("Parallel gateway", "semantics", "<+>@{id}"),
            r("Inclusive gateway", "semantics", "<O>@{id}"),
            r("Event-based gateway", "semantics", "<event>@{id}"),
            r("Sequence flow", "semantics", "{from} --> {to} : {label}"),
            r("Message flow", "semantics", "{from} ~~> {to} : {label}"),
            r("Data association", "semantics", "{from} ..> {to}"),
            r("Annotation association", "semantics", "{from} .. {to}"),
            r("Pool", "semantics", "pool {id}({label:q}) {\n}"),
            r("Lane", "semantics", "lane {id}({label:q}) {\n}"),
            r("Expanded subprocess", "semantics", "{id}({label:q}) {\n}"),
            r("Collapsed subprocess", "semantics", "+{id}({label:q})"),
            r("Data object", "semantics", "data {id}({label:q})"),
            r("Data store", "semantics", "store {id}({label:q})"),
            r("Annotation", "semantics", "note {id}({label:q})"),
            r(
                "Boundary timer",
                "semantics",
                "{from} -(timer@{id}:{label:q})-> {to}",
            ),
            r("Relative placement", "layout", "{from} < {to}"),
        ],
        C4Context | C4Container | C4Component => vec![
            r(
                "Person",
                "semantics",
                "person {id}(label={label:q}; description={description:q})",
            ),
            r(
                "External person",
                "semantics",
                "external person {id}(label={label:q}; description={description:q})",
            ),
            r(
                "External system",
                "semantics",
                "external system {id}(label={label:q}; description={description:q})",
            ),
            r("Relationship", "semantics", "{from} -> {to} : {label:q}"),
        ],
        _ => vec![],
    };
    if matches!(ty, C4Container | C4Component) {
        out.push(r("Container", "semantics", "container {id}(label={label:q}; description={description:q}; technology={technology:q})"));
    }
    if ty == C4Component {
        out.push(r("Component", "semantics", "component {id}(label={label:q}; description={description:q}; technology={technology:q})"));
        out.push(r(
            "Enclosing system",
            "semantics",
            "system {id}(label={label:q}; description={description:q}) {\n}",
        ));
    }
    if matches!(ty, Gne | Class | C4Code | Composite) {
        out.extend([
            r("Attribute", "semantics", "+{name}: {classifier}"),
            r("Operation", "semantics", "+{name}(): {classifier}"),
        ]);
    }
    if matches!(ty, Gne | Class | C4Code | Component | Composite) {
        out.push(r("Port", "semantics", "port {id}"));
        if matches!(ty, Component | Composite) {
            out.retain(|r| r.title != "Port");
            out.extend([
                r(
                    "Required interface port",
                    "semantics",
                    "port {id}(requires={classifier})",
                ),
                r(
                    "Provided interface port",
                    "semantics",
                    "port {id}(provides={classifier})",
                ),
            ]);
        }
    }
    if ty.graph() {
        out.push(r(
            "Relative placement",
            "layout",
            "{from} < {to} : gap={gap}",
        ));
    }
    out.extend(extended::recipes(ty));
    out.push(r(
        "Style rule",
        "styling",
        "{selector} {\n    {property}: {value}\n}",
    ));
    out
}
/// Placeholder fields, in stable appearance order. Braces used by LAML blocks
/// are not placeholders and are copied verbatim.
pub fn parameters(recipe: Recipe) -> Vec<String> {
    let mut out = Vec::new();
    let combined = format!("{}\n{}", recipe.body, recipe.layout);
    let mut tail = combined.as_str();
    while let Some(at) = tail.find('{') {
        tail = &tail[at + 1..];
        let Some(end) = tail.find('}') else {
            break;
        };
        let key = tail[..end].trim_end_matches(":q");
        if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !key.is_empty() {
            if !out.iter().any(|s| s == key) {
                out.push(key.into());
            }
            tail = &tail[end + 1..];
        }
    }
    out
}
pub fn instantiate(
    recipe: Recipe,
    values: &std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    let mut out = String::new();
    let mut tail = recipe.body;
    while let Some(at) = tail.find('{') {
        out.push_str(&tail[..at]);
        tail = &tail[at..];
        if let Some(end) = tail.find('}') {
            let placeholder = &tail[1..end];
            let (name, quoted) = placeholder
                .strip_suffix(":q")
                .map_or((placeholder, false), |n| (n, true));
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                let value = values.get(name).ok_or_else(|| format!("Missing {name}"))?;
                if quoted {
                    out.push_str(&crate::relation_edit::quote(value));
                } else {
                    if value.contains(['\n', '\r', '{', '}']) {
                        return Err(format!("Enter one {name} value"));
                    }
                    out.push_str(value);
                }
                tail = &tail[end + 1..];
                continue;
            }
        }
        out.push('{');
        tail = &tail[1..];
    }
    out.push_str(tail);
    Ok(out)
}

pub fn create(
    source: &str,
    recipe: Recipe,
    values: &std::collections::BTreeMap<String, String>,
    parent: Option<usize>,
    before: Option<usize>,
) -> Result<Vec<crate::document::Edit>, String> {
    let first_pool = recipe.title == "Pool"
        && crate::notation::Notation::of(source) == Notation::Bpmn
        && laml::bpmn::parse(source)?.pools.is_empty();
    if first_pool || recipe.title == "Wrap root process in pool" {
        return super::bpmn::wrap_root_process(
            source,
            values.get("id").ok_or("Missing pool ID")?,
            values.get("label").ok_or("Missing pool label")?,
        );
    }
    let mut body = instantiate(recipe, values)?;
    let mut replacement = Vec::new();
    let inserts_flow = recipe.title.ends_with("on flow")
        && crate::notation::Notation::of(source) == Notation::InteractionOverview;
    if inserts_flow {
        let graph = crate::notation::parse_graph(source).map_err(|e| e.message)?;
        let flow = graph
            .relations
            .iter()
            .find(|r| Some(&r.from) == values.get("from") && Some(&r.to) == values.get("to"))
            .ok_or("Choose the endpoints of an existing flow")?;
        let span = crate::relation_edit::endpoint_spans(source, flow.line)?;
        replacement.push(crate::document::Edit {
            range: span.to,
            replacement: values.get("id").ok_or("Missing ID")?.clone(),
        });
        let first_leg = format!(
            "{} --> {}",
            values.get("from").unwrap(),
            values.get("id").unwrap()
        );
        body = body
            .lines()
            .filter(|line| *line != first_leg)
            .collect::<Vec<_>>()
            .join("\n");
    }
    let mut edits = super::insert(source, recipe.section, parent, before, &body)?;
    edits.extend(replacement);
    if !recipe.layout.is_empty() {
        let layout_recipe = Recipe {
            body: recipe.layout,
            layout: "",
            ..recipe
        };
        let layout = instantiate(layout_recipe, values)?;
        edits.extend(super::insert(source, "layout", None, None, &layout)?);
    }
    let edits = crate::element_edit::merge_insertions(edits);
    if let Ok(before) = crate::notation::parse_graph(source) {
        let candidate = super::apply(source, &edits)?;
        let after = crate::notation::parse_graph(&candidate).map_err(|e| e.message)?;
        for old in &before.relations {
            let line = crate::relation_edit::remap_line(source, old.line, &edits);
            if after
                .relations
                .iter()
                .find(|r| r.line == line)
                .is_none_or(|new| {
                    new.from != old.from
                        || (new.to != old.to
                            && !(inserts_flow
                                && Some(&old.from) == values.get("from")
                                && Some(&old.to) == values.get("to")
                                && Some(&new.to) == values.get("id")))
                })
            {
                return Err("This new construct would capture an existing relationship reference; choose a different ID".into());
            }
        }
    }
    Ok(edits)
}
