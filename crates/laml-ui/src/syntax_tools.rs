//! Source-aware formatting, delimiter matching and completion for the GNE editor.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use std::{collections::BTreeSet, ops::Range};
#[derive(Default)]
pub struct Scan {
    pub pairs: Vec<(usize, usize)>,
    pub opaque: Vec<Range<usize>>,
    pub braces: Vec<(usize, char)>,
    pub sections: Vec<(usize, String)>,
}
pub fn scan(source: &str) -> Scan {
    let mut out = Scan::default();
    let mut stack: Vec<(usize, char)> = vec![];
    let mut note = false;
    let index = SourceIndex::new(source);
    for (n, line) in source.lines().enumerate() {
        let start = index.line(n + 1).unwrap().start;
        let text = code(line);
        let trimmed = text.trim();
        if note && trimmed != "}" {
            out.opaque.push(start..start + line.len() + 1);
            continue;
        }
        if note {
            note = false;
        }
        if stack.is_empty() && trimmed.ends_with('{') {
            out.sections
                .push((start, trimmed.trim_end_matches('{').trim().into()));
        }
        let mut quote = None;
        let mut escape = false;
        let mut depth = 0usize;
        let relation = ["->", "<-", "--", "..>"].iter().any(|s| text.contains(s));
        for (i, c) in text.char_indices() {
            if let Some(q) = quote {
                if escape {
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == '"' {
                    out.opaque.push(start + q..start + i + 1);
                    quote = None;
                }
                continue;
            }
            if c == '"' {
                quote = Some(i);
                continue;
            }
            if relation
                && c == ':'
                && ["->", "<-", "--", "..>"]
                    .iter()
                    .any(|op| text[..i].contains(op))
                && depth == 0
                && !text[i..].starts_with("::")
                && !text[..i].ends_with(':')
            {
                out.opaque.push(start + i + 1..start + line.len() + 1);
                break;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth = depth.saturating_sub(1);
            }
            if "{[(".contains(c) {
                stack.push((start + i, c));
            } else if "}])".contains(c)
                && let Some((a, open)) = stack.pop()
            {
                if matches!((open, c), ('{', '}') | ('[', ']') | ('(', ')')) {
                    out.pairs.push((a, start + i));
                } else {
                    stack.clear();
                }
            }
            if matches!(c, '{' | '}') {
                out.braces.push((start + i, c));
            }
        }
        if let Some(q) = quote {
            out.opaque.push(start + q..start + line.len() + 1);
        }
        if text.len() < line.len() {
            out.opaque.push(start + text.len()..start + line.len() + 1);
        }
        let parts = crate::relation_edit::tokens(trimmed);
        if trimmed.ends_with('{') && parts.iter().take(2).any(|r| &trimmed[r.clone()] == "note") {
            note = true;
        }
    }
    out
}
pub fn matching_pair(source: &str, caret: usize) -> Option<(Range<usize>, Range<usize>)> {
    if caret > source.len() || !source.is_char_boundary(caret) {
        return None;
    }
    let scan = scan(source);
    let positions = [
        Some(caret),
        source[..caret].char_indices().next_back().map(|(i, _)| i),
    ];
    for p in positions.into_iter().flatten() {
        if let Some((a, b)) = scan.pairs.iter().find(|(a, b)| *a == p || *b == p) {
            return Some((*a..*a + 1, *b..*b + 1));
        }
    }
    None
}
/// Explicit leading-whitespace formatter. No note text, line breaks or tokens change.
pub fn format(source: &str) -> Result<Vec<Edit>, String> {
    let before = crate::notation::inspect(source)?;
    let scan = scan(source);
    let index = SourceIndex::new(source);
    let mut depth = 0usize;
    let mut edits = vec![];
    for n in 1..=index.line_count() {
        let range = index.line(n).unwrap();
        let line = &source[range.clone()];
        let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
        let protected = scan
            .opaque
            .iter()
            .any(|r| r.start == range.start && r.end >= range.end);
        let closing = scan
            .braces
            .iter()
            .any(|(p, c)| *p == range.start + indent && *c == '}');
        if !line.trim().is_empty() && !protected {
            let replacement = "    ".repeat(depth.saturating_sub(usize::from(closing)));
            if line[..indent] != replacement {
                edits.push(Edit {
                    range: range.start..range.start + indent,
                    replacement,
                });
            }
        }
        for (_, c) in scan.braces.iter().filter(|(p, _)| range.contains(p)) {
            if *c == '{' {
                depth += 1;
            } else {
                depth = depth.saturating_sub(1);
            }
        }
    }
    let candidate = crate::node_edit::candidate(source, &edits)?;
    if crate::notation::inspect(&candidate)? != before {
        return Err("Formatting would change compiler semantics; source unchanged".into());
    }
    Ok(edits)
}
#[derive(Clone, Debug)]
pub struct Completion {
    pub range: Range<usize>,
    pub values: Vec<String>,
}
pub fn complete(source: &str, caret: usize, diagram: Option<&laml::syntax::Diagram>) -> Completion {
    let empty = || Completion {
        range: caret..caret,
        values: vec![],
    };
    if caret > source.len() || !source.is_char_boundary(caret) {
        return empty();
    }
    let scanned = scan(source);
    if scanned.opaque.iter().any(|r| r.contains(&caret)) {
        return empty();
    }
    let start = source[..caret]
        .char_indices()
        .rev()
        .take_while(|(i, c)| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '-' | '.')
                || *c == ':' && (source[*i..].starts_with("::") || source[..*i].ends_with(':'))
        })
        .last()
        .map_or(caret, |(i, _)| i);
    let prefix = &source[start..caret];
    let line = source[..start].rsplit('\n').next().unwrap_or("").trim();
    let mut opens = vec![];
    for (position, c) in scanned.braces.iter().filter(|(p, _)| *p < caret) {
        if *c == '{' {
            opens.push(*position);
        } else {
            opens.pop();
        }
    }
    let heading = |position: usize| source[..position].rsplit('\n').next().unwrap_or("").trim();
    let section = opens.first().map(|p| heading(*p)).unwrap_or("");
    let body_heading = opens.last().map(|p| heading(*p)).unwrap_or("");
    let nested_body = opens.len() > 1
        && !crate::relation_edit::tokens(body_heading)
            .iter()
            .take(2)
            .any(|r| &body_heading[r.clone()] == "package");
    let mut values = BTreeSet::new();
    let mut add = |items: &[&str]| values.extend(items.iter().map(|s| s.to_string()));
    let notation = crate::notation::Notation::of(source);
    if section == "settings" {
        if let Some((key, _)) = line.split_once(':') {
            match key.trim() {
                "type" => {
                    let types: Vec<_> = crate::notation::Notation::ALL
                        .into_iter()
                        .filter(|n| *n != crate::notation::Notation::Gne)
                        .map(|n| n.key())
                        .collect();
                    add(&types);
                }
                "semantic-check" => add(&["on", "off"]),
                "direction" => add(&[
                    "left-to-right",
                    "right-to-left",
                    "top-to-bottom",
                    "bottom-to-top",
                ]),
                "theme" => add(&[
                    "modern",
                    "plain",
                    "modern-mono",
                    "modern-dark",
                    "plain-dark",
                    "modern-mono-dark",
                ]),
                "relation-label" => add(&["near", "along", "inline"]),
                "background" => add(&["opaque", "transparent"]),
                "allow-diagonal" => add(&["true", "false"]),
                _ => {}
            }
        } else {
            add(&[
                "direction:",
                "naming-convention:",
                "relation-label:",
                "theme:",
                "background:",
                "font-file:",
                "font-italic-file:",
                "allow-diagonal:",
            ]);
        }
    } else if section == "styling" && opens.len() == 1 {
        add(&[
            "import",
            "node",
            "port",
            "relation",
            "member",
            "compartment",
        ]);
    } else if section == "styling" {
        if let Some((key, _)) = line.split_once(':') {
            match key.trim() {
                "shape" => add(&["rectangle", "ellipse", "diamond", "note"]),
                "display" => add(&["block", "none"]),
                "symbol-rendering" => add(&["text", "icons"]),
                "icon-set" => add(&["phosphor"]),
                _ => {}
            }
        } else {
            add(&[
                "fill:",
                "border-color:",
                "border-width:",
                "color:",
                "base-color:",
                "font-size:",
                "padding:",
                "width:",
                "height:",
                "min-width:",
                "min-height:",
                "corner-radius:",
                "shape:",
                "icon:",
                "display:",
                "symbol-rendering:",
                "icon-set:",
                "node",
                "port",
                "relation",
            ]);
        }
    } else if section == "layout" {
        add(&[
            "above", "below", "align=", "center", "center-x", "center-y", "left", "right", "top",
            "bottom", "side=", "label=", "gap=",
        ]);
    } else if section == "semantics" {
        if notation != crate::notation::Notation::Gne {
            let mut words = Vec::new();
            if notation.graph() && !notation.c4() {
                words.extend(
                    crate::element_edit::ElementKind::available(source)
                        .into_iter()
                        .map(|k| k.keyword().trim()),
                );
            }
            for recipe in crate::notation_edit::catalog::recipes(notation)
                .into_iter()
                .filter(|r| r.section == "semantics")
            {
                if let Some(word) = recipe
                    .body
                    .split_whitespace()
                    .next()
                    .filter(|s| !s.contains('{'))
                {
                    words.push(word);
                }
            }
            add(&words);
        } else {
            if nested_body {
                if body_heading.contains("enum ") {
                    add(&["VALUE"]);
                } else if body_heading.contains("object ") {
                    add(&["value = ", "text"]);
                } else {
                    add(&[
                        "field", "method", "text", "static", "abstract", "port", "--",
                    ]);
                }
            } else {
                add(&[
                    "package",
                    "note",
                    "class",
                    "interface",
                    "enum",
                    "datatype",
                    "object",
                    "junction",
                ]);
            }
        }
    } else {
        add(&["semantics {", "settings {", "layout {", "styling {"]);
    }
    if matches!(section, "semantics" | "layout" | "styling")
        && let Some(diagram) = diagram
    {
        for node in &diagram.nodes {
            values.insert(node.id.clone());
            for port in &node.ports {
                values.insert(crate::properties::port_id(&node.id, port));
            }
        }
        for relation in &diagram.relations {
            if let Some(name) = &relation.name {
                values.insert(format!("relation[{name}]"));
            }
        }
    }
    if section == "settings" && !line.contains(':') {
        values.insert("type:".into());
        match notation {
            crate::notation::Notation::Sequence => {
                values.retain(|v| {
                    !["direction:", "relation-label:", "naming-convention:"].contains(&v.as_str())
                });
                values.insert("semantic-check:".into());
            }
            crate::notation::Notation::Timing => {
                values.retain(|v| {
                    !["direction:", "relation-label:", "naming-convention:"].contains(&v.as_str())
                });
                values.extend(["interaction:", "time-unit:"].map(str::to_owned));
            }
            crate::notation::Notation::Bpmn => {
                values.retain(|v| !["relation-label:", "naming-convention:"].contains(&v.as_str()));
            }
            crate::notation::Notation::Communication
            | crate::notation::Notation::InteractionOverview => {
                values.insert("interaction:".into());
            }
            ty if ty.c4() => {
                values.insert("scope:".into());
            }
            _ => {}
        }
    }
    if matches!(section, "semantics" | "layout" | "styling") {
        match notation {
            crate::notation::Notation::Bpmn => {
                if let Ok(doc) = laml::bpmn::parse(source) {
                    values.extend(doc.nodes.into_iter().map(|n| n.id));
                }
            }
            crate::notation::Notation::Sequence => {
                if let Ok(doc) = laml::sequence::parse(source) {
                    values.extend(doc.participants.into_iter().map(|n| n.id));
                }
            }
            crate::notation::Notation::Timing => {
                if let Ok(doc) = laml::timing::parse(source) {
                    values.extend(doc.lifelines.into_iter().map(|n| n.id));
                }
            }
            _ => {}
        }
    }
    Completion {
        range: start..caret,
        values: values
            .into_iter()
            .filter(|v| v.starts_with(prefix) && v != prefix)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notes_strings_comments_and_plain_relation_labels_are_opaque() {
        let s = "semantics {\n note N {\n raw { [ ( // preserve\n }\n A(label=\"(ignored)\")\n B\n A -> B : {ignored\n}\n";
        let result = scan(s);
        assert_eq!(result.pairs.len(), 3);
        let p = s.find("raw").unwrap();
        assert!(complete(s, p + 3, None).values.is_empty());
        assert!(matching_pair(s, s.find("ignored").unwrap()).is_none());
        assert!(matching_pair(s, s.find('{').unwrap()).is_some());
    }
    #[test]
    fn format_preserves_free_note_text_comments_unicode_and_crlf() {
        let s = "// keep\r\nsemantics {\r\nA\r\n note N {\r\n  Ä // { literal\r\n }\r\n}\r\n";
        let edited = crate::node_edit::candidate(s, &format(s).unwrap()).unwrap();
        assert!(edited.contains("    A\r\n    note N {\r\n  Ä // { literal\r\n    }\r\n"));
        assert_eq!(
            crate::node_edit::diagram(s).unwrap(),
            crate::node_edit::diagram(&edited).unwrap()
        );
        assert!(format(&edited).unwrap().is_empty());
        assert!(format("semantics {\n A -> Missing\n}").is_err());
    }
    #[test]
    fn context_completion_and_matching_work_in_incomplete_source() {
        assert!(
            complete("settings {\n direction: lef", 26, None)
                .values
                .contains(&"left-to-right".into())
        );
        let s = "styling {\n node {\n font-s";
        assert!(
            complete(s, s.len(), None)
                .values
                .contains(&"font-size:".into())
        );
        let s = "semantics {\n A(label=\"unfinished";
        assert!(complete(s, s.len(), None).values.is_empty());
        assert!(matching_pair("{[(])}", 3).is_none());
    }
}

#[cfg(test)]
mod corpus {
    use super::*;
    #[test]
    fn formatting_preserves_the_delivered_gne_example_models() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut checked = 0;
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "laml") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            if crate::node_edit::diagram(&source).is_err() {
                continue;
            }
            let edits = format(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let candidate = crate::node_edit::candidate(&source, &edits).unwrap();
            assert!(format(&candidate).unwrap().is_empty(), "{}", path.display());
            checked += 1;
        }
        assert!(checked >= 5);
    }
}
