//! Identity-bearing spans, excluding visible text, attribute keys and CSS values.
use super::*;
pub(super) fn spans(text: &str, section: &str, timing: bool) -> Vec<Range<usize>> {
    let tokens = fields::tokens(text);
    let raw: Vec<_> = tokens.iter().map(|r| &text[r.clone()]).collect();
    if section == "settings" && raw.first() != Some(&"scope") { return vec![]; }
    if section == "semantics" && matches!(raw.first().copied(), Some("entry" | "exit" | "do")) { return vec![]; }
    let arrow = raw.iter().position(|s| crate::relation_edit::OPERATORS.contains(s) || ["->>", "~~>"].contains(s));
    let label = arrow.and_then(|op| raw.iter().enumerate().skip(op + 1).find(|(_, s)| **s == ":").map(|(i, _)| i));
    let colon = raw.iter().position(|s| *s == ":");
    let mut bracket = 0; let mut attribute: Option<&str> = None; let mut result = Vec::new();
    for (i, span) in tokens.iter().enumerate() {
        let value = raw[i];
        if value == "[" { bracket += 1; continue; }
        if value == "]" { bracket -= 1; continue; }
        if label.is_some_and(|at| i >= at) { break; }
        if timing && section == "semantics" && colon.is_some_and(|at| i >= at) { break; }
        if section == "layout" && colon.is_some_and(|at| i >= at) { break; }
        if section == "styling" && bracket == 0 { continue; }
        if matches!(value, ";" | ")") { attribute = None; continue; }
        if raw.get(i + 1) == Some(&"=") { attribute = Some(value); continue; }
        if attribute.is_some_and(|key| !matches!(key, "of" | "ref" | "requires" | "provides")) { continue; }
        if value.starts_with('"') || (raw.get(i + 1) == Some(&":") && section != "semantics"
            && !(section == "layout" && value.ends_with(['!', '+', '-']))) { continue; }
        if raw.get(i + 1) == Some(&"{") && section != "semantics" { continue; }
        let keyword = ["external", "person", "system", "container", "component", "class", "interface", "enum", "object", "datatype", "package", "profile", "metaclass", "stereotype", "state", "initial", "final", "choice", "action", "decision", "merge", "fork", "join", "interaction-use", "lifeline", "event", "pool", "lane", "decompose", "port", "part", "structure", "control", "message", "note", "coregion", "order", "between", "at", "flow", "tick", "startEvent", "endEvent", "userTask", "serviceTask", "sendTask", "receiveTask", "scriptTask", "manualTask", "messageStart", "timerStart", "messageEnd", "errorEnd", "terminateEnd", "intermediateTimer", "intermediateMessage", "data", "store", "entry", "exit", "do"];
        if (i == 0 || (i == 1 && raw[0] == "external")) && raw.len() > i + 1 && keyword.contains(&value) && !arrow.is_some_and(|op| op == i + 1) { continue; }
        let mut begin = None;
        for (at, ch) in value.char_indices().chain(std::iter::once((value.len(), ' '))) {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':') { begin.get_or_insert(at); }
            else if let Some(begin) = begin.take() { result.push(span.start + begin..span.start + at); }
        }
    }
    result
}
pub(super) fn scope(source: &str, record: &Record, notation: crate::notation::Notation) -> String {
    let all = records(source); let mut parent = record.parent;
    let bpmn = if notation == crate::notation::Notation::Bpmn { laml::bpmn::parse(source).ok() } else { None };
    let mut decomposition = Vec::new();
    while let Some(line) = parent {
        let Some(r) = all.iter().find(|r| r.line == line) else { break; };
        if let Some(doc) = &bpmn {
            if let Some(node) = doc.nodes.iter().find(|n| n.line == line && n.kind == laml::bpmn::Kind::Subprocess) { return node.id.clone(); }
            if let Some(pool) = doc.pools.iter().find(|p| p.line == line) { return pool.id.clone(); }
        }
        if notation == crate::notation::Notation::Sequence && r.heading.starts_with("decompose ") {
            if let Some(id) = r.heading.split_whitespace().nth(1) { decomposition.push(id); }
        }
        parent = r.parent;
    }
    decomposition.reverse(); decomposition.join(".")
}
