//! Dedicated header properties; preserve name, IDs, attributes, comments and body.
use crate::{document::Edit, node_edit, source::SourceIndex, structure, style_edit::code};
use laml::syntax::NodeKind;
use std::ops::Range;

#[derive(Clone, Debug)]
pub enum Change {
    Kind(NodeKind),
    Visibility(Option<char>),
    Abstract(bool),
    Templates(String),
}
pub const KINDS: [NodeKind; 5] = [
    NodeKind::Generic,
    NodeKind::Class,
    NodeKind::Interface,
    NodeKind::Enumeration,
    NodeKind::DataType,
];
pub fn title(kind: NodeKind) -> String {
    match kind {
        NodeKind::Generic => "Node".into(),
        NodeKind::DataType => "Data type".into(),
        _ => format!("{kind:?}"),
    }
}
fn keyword(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Class => "class",
        NodeKind::Interface => "interface",
        NodeKind::Enumeration => "enum",
        NodeKind::DataType => "datatype",
        NodeKind::Object => "object",
        NodeKind::Package => "package",
        NodeKind::Note => "note",
        NodeKind::Junction => "junction",
        _ => "",
    }
}
struct Prefix {
    start: usize,
    kind: usize,
    visibility: Option<Range<usize>>,
    abstract_token: Option<Range<usize>>,
}
fn prefix(text: &str) -> Result<Prefix, String> {
    let mut at = text.len() - text.trim_start().len();
    if text[at..].starts_with('@') {
        at += text[at..]
            .find(char::is_whitespace)
            .ok_or("Missing declaration after ID")?;
        at += text[at..].len() - text[at..].trim_start().len();
    }
    let start = at;
    let mut visibility = None;
    let mut abstract_token = None;
    loop {
        let tail = &text[at..];
        if tail.starts_with("abstract") && tail[8..].starts_with(char::is_whitespace) {
            let from = at;
            at += 8;
            at += text[at..].len() - text[at..].trim_start().len();
            abstract_token = Some(from..at);
        } else if tail.starts_with(['+', '-', '#', '~']) {
            visibility = Some(at..at + 1);
            at += 1;
            at += text[at..].len() - text[at..].trim_start().len();
        } else {
            break;
        }
    }
    Ok(Prefix {
        start,
        kind: at,
        visibility,
        abstract_token,
    })
}
pub fn change(source: &str, id: &str, change: Change) -> Result<Vec<Edit>, String> {
    let diagram = node_edit::diagram(source)?;
    let node = diagram
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Missing node")?;
    let range = SourceIndex::new(source)
        .line(node.line)
        .ok_or("Missing declaration")?;
    let raw = &source[range];
    let mut text = code(raw).trim().to_owned();
    let p = prefix(&text)?;
    match &change {
        Change::Kind(kind) => {
            if !node.templates.is_empty() && *kind != NodeKind::Class {
                return Err("Remove template parameters before changing the element type.".into());
            }
            if !KINDS.contains(&node.kind) || !KINDS.contains(kind) {
                return Err("This structural element cannot change kind in Properties.".into());
            }
            let declared = keyword(node.kind);
            let old = if !declared.is_empty()
                && text[p.kind..].starts_with(declared)
                && text[p.kind + declared.len()..].starts_with(char::is_whitespace)
            {
                declared
            } else {
                ""
            };
            let new = keyword(*kind);
            if !old.is_empty()
                && (!text[p.kind..].starts_with(old)
                    || !text[p.kind + old.len()..].starts_with(char::is_whitespace))
            {
                return Err("Cannot locate element type".into());
            }
            let end = p.kind + old.len();
            if new.is_empty() {
                let end = end + text[end..].len() - text[end..].trim_start().len();
                text.replace_range(p.kind..end, "");
            } else if old.is_empty() {
                text.insert_str(p.kind, &format!("{new} "));
            } else {
                text.replace_range(p.kind..end, new);
            }
        }
        Change::Visibility(value) => {
            if value.is_some_and(|c| !matches!(c, '+' | '-' | '#' | '~')) {
                return Err("Unknown visibility".into());
            }
            if matches!(
                node.kind,
                NodeKind::Note | NodeKind::Object | NodeKind::Junction
            ) {
                return Err("This element has no access visibility.".into());
            }
            if let Some(range) = p.visibility {
                text.replace_range(range, &value.map_or(String::new(), |c| c.to_string()));
            } else if let Some(value) = value {
                text.insert_str(p.start, &format!("{value} "));
            }
        }
        Change::Abstract(value) => {
            if !KINDS.contains(&node.kind) {
                return Err("This element has no abstract modifier.".into());
            }
            if *value && p.abstract_token.is_none() {
                text.insert_str(p.start, "abstract ");
            } else if !value && let Some(range) = p.abstract_token {
                text.replace_range(range, "");
            }
        }
        Change::Templates(value) => {
            if node.kind != NodeKind::Class {
                return Err("Template parameters apply to classes.".into());
            }
            let value = value.trim();
            if value.contains(['<', '>', '\n', '\r', '{', '}']) {
                return Err(
                    "Enter template parameters only, without angle brackets or line breaks.".into(),
                );
            }
            let name = node_edit::node_head(&text, node)?.start;
            let end = name
                + text[name..]
                    .find(|c: char| c == '<' || c == '(' || c.is_whitespace())
                    .unwrap_or(text.len() - name);
            let range = if text[end..].starts_with('<') {
                end..end
                    + text[end..]
                        .find('>')
                        .ok_or("Unclosed template parameters")?
                    + 1
            } else {
                end..end
            };
            text.replace_range(
                range,
                &if value.is_empty() {
                    String::new()
                } else {
                    format!("<{value}>")
                },
            );
        }
    }
    let edits = structure::header(source, id, &text)?;
    let candidate = node_edit::diagram(&node_edit::candidate(source, &edits)?)?;
    let updated = candidate
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("Header change lost its original ID")?;
    let expected = match change {
        Change::Kind(kind) => updated.kind == kind,
        Change::Visibility(value) => updated.visibility == value,
        Change::Abstract(value) => updated.abstract_node == value,
        Change::Templates(_) => true,
    };
    if !expected {
        return Err("Header property could not be applied; source unchanged.".into());
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fields_preserve_explicit_ids_comments_unicode_crlf_body_and_references() {
        let source = "// Grüße\r\nsemantics {\r\n @Stable abstract +class A<T>(label=\"Name\") { // keep\r\n +run()\r\n }\r\n B\r\n Stable -> B\r\n}\r\n";
        for (field, expected) in [
            (Change::Visibility(Some('-')), "abstract -class"),
            (Change::Abstract(false), "@Stable +class"),
            (Change::Templates("X:Type".into()), "A<X:Type>"),
            (Change::Templates(String::new()), "A(label="),
        ] {
            let edits = change(source, "Stable", field).unwrap();
            let updated = node_edit::candidate(source, &edits).unwrap();
            assert!(updated.contains(expected), "{updated}");
            assert!(updated.contains("// keep\r\n +run()\r\n"));
            assert!(updated.contains("Stable -> B"));
            assert!(!updated.replace("\r\n", "").contains('\n'));
            crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
        }
    }
    #[test]
    fn kind_changes_are_checked_without_renaming_and_unsupported_conversions_fail() {
        let source = "semantics {\n A(label=\"Display\")\n B\n A -> B\n}\n";
        for kind in KINDS {
            let updated =
                node_edit::candidate(source, &change(source, "A", Change::Kind(kind)).unwrap())
                    .unwrap();
            let preview =
                crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
            assert_eq!(preview.diagram.nodes[0].kind, kind);
            assert!(updated.contains("A -> B"));
        }
        assert!(
            change(
                "semantics {\n class A<T>\n}\n",
                "A",
                Change::Kind(NodeKind::Generic)
            )
            .is_err()
        );
        assert!(change(source, "A", Change::Templates("T> {\n bad".into())).is_err());
    }
}
