//! Reference discovery remains useful while the current statement is incomplete.
use super::{Entry, context};
use crate::notation::Notation;
use std::collections::BTreeSet;
struct Scope {
    name: Option<String>,
    members: bool,
    note: bool,
}
pub(super) fn references(source: &str) -> Vec<Entry> {
    let clean = context::code(source);
    let ty = Notation::of(source);
    let mut names = BTreeSet::new();
    let mut stack: Vec<Scope> = vec![];
    let mut semantic = false;
    for line in clean.lines() {
        let line = line.trim();
        if line.starts_with("semantics {") {
            semantic = true;
            stack.push(Scope {
                name: None,
                members: false,
                note: false,
            });
            continue;
        }
        if line == "}" {
            stack.pop();
            if stack.is_empty() {
                semantic = false;
            }
            continue;
        }
        if !semantic || stack.last().is_some_and(|s| s.note) {
            continue;
        }
        let mut words = line.split_whitespace();
        let mut first = words.next().unwrap_or("");
        let explicit = first.strip_prefix('@');
        if explicit.is_some() {
            first = words.next().unwrap_or("");
        }
        if first == "abstract" || first == "external" {
            first = words.next().unwrap_or("");
        }
        let declaration = crate::element_edit::ElementKind::ALL
            .iter()
            .any(|k| !k.keyword().is_empty() && k.keyword().trim() == first)
            || [
                "person",
                "container",
                "boundary",
                "pool",
                "lane",
                "subprocess",
                "task",
                "userTask",
                "serviceTask",
                "sendTask",
                "receiveTask",
                "scriptTask",
                "manualTask",
                "businessRuleTask",
                "startEvent",
                "endEvent",
                "lifeline",
                "port",
            ]
            .contains(&first);
        let named_event = line.contains(':') && !line.contains("::") && ty == Notation::Sequence;
        let declared = explicit.unwrap_or_else(|| {
            if declaration {
                words.next().unwrap_or("")
            } else {
                first
            }
        });
        let name = declared
            .split(['(', ':', '{', '[', '<'])
            .next()
            .unwrap_or("")
            .trim_end_matches(['+', '-', '!']);
        let free_event = [
            "alt",
            "opt",
            "loop",
            "else",
            "and",
            "ref",
            "note",
            "at",
            "between",
            "order",
            "decompose",
            "coregion",
            "event",
            "par",
            "critical",
            "break",
            "neg",
            "assert",
            "strict",
            "seq",
            "ignore",
            "consider",
        ]
        .contains(&first);
        let member = stack.last().is_some_and(|s| s.members) && first != "port";
        let valid = !name.is_empty()
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            && name.chars().all(context::word)
            && !free_event
            && (!member || explicit.is_some())
            && (declaration || named_event || !line.contains(':') || explicit.is_some());
        if valid {
            let parents = stack
                .iter()
                .filter_map(|s| s.name.as_deref())
                .collect::<Vec<_>>();
            let id = if explicit.is_some() || parents.is_empty() {
                name.into()
            } else {
                format!("{}.{name}", parents.join("."))
            };
            names.insert(id);
        }
        if line.ends_with('{') {
            let scoped = valid && ty.graph() && (!ty.c4() || ty == Notation::C4Code);
            stack.push(Scope {
                name: scoped.then(|| name.into()),
                members: ["class", "interface", "enum", "datatype", "object"].contains(&first),
                note: first == "note",
            });
        }
    }
    names
        .into_iter()
        .map(|s| Entry::new(&s, "Existing source reference", &s))
        .collect()
}
pub(super) fn classes(source: &str) -> Vec<Entry> {
    let clean = context::code(source);
    let mut names = BTreeSet::new();
    for line in clean.lines() {
        if let Some((_, tail)) = line.split_once("style-classes=") {
            for name in tail
                .split([';', ')'])
                .next()
                .unwrap_or("")
                .split(',')
                .map(str::trim)
            {
                if !name.is_empty() {
                    names.insert(name.to_owned());
                }
            }
        }
        if line.trim_end().ends_with('{') {
            for part in line.split('.').skip(1) {
                let name: String = part
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
                    .collect();
                if !name.is_empty() {
                    names.insert(name);
                }
            }
        }
    }
    names
        .into_iter()
        .map(|s| Entry::new(&s, "Style class", &s))
        .collect()
}
pub(super) fn icons() -> Vec<Entry> {
    laml::bundled_icon_names()
        .map(|s| Entry::new(s, "Phosphor icon", &format!("\"{s}\"")))
        .collect()
}
