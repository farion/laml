//! GNE creation commands: no hidden coordinates and no automatic layout constraints.
use crate::{document::Edit, source::SourceIndex, style_edit::code};
use laml::syntax::{Diagram, StyleEntry};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Directed,
    Bidirectional,
    Undirected,
}
impl Connection {
    pub fn operator(self) -> &'static str {
        match self {
            Self::Directed => "->",
            Self::Bidirectional => "<->",
            Self::Undirected => "--",
        }
    }
}

fn parse(source: &str) -> Result<Diagram, String> {
    if source.trim().is_empty() {
        return Ok(Diagram::default());
    }
    let diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    Ok(diagram)
}

pub(crate) fn fresh_id(diagram: &Diagram, prefix: &str) -> String {
    let occupied: BTreeSet<&str> = diagram
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        // New global addresses must not capture references to scoped defaults.
        .chain(
            diagram
                .nodes
                .iter()
                .map(|node| node.id.rsplit('.').next().unwrap()),
        )
        .chain(
            diagram
                .relations
                .iter()
                .filter_map(|edge| edge.name.as_deref()),
        )
        .chain(
            diagram
                .attachments
                .iter()
                .filter_map(|edge| edge.name.as_deref()),
        )
        .chain(
            diagram
                .nodes
                .iter()
                .flat_map(|node| node.ports.iter().map(|port| port.id.as_str())),
        )
        .chain(diagram.nodes.iter().flat_map(|node| {
            node.parts.iter().filter_map(|part| match part {
                laml::syntax::NodePart::Member { id: Some(id), .. } => Some(id.as_str()),
                _ => None,
            })
        }))
        .collect();
    (1..)
        .map(|i| format!("{prefix}{i}"))
        .find(|id| !occupied.contains(id.as_str()))
        .unwrap()
}
fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    )
}

pub fn add_node(source: &str, label: &str) -> Result<(String, Vec<Edit>), String> {
    let diagram = parse(source)?;
    let id = fresh_id(&diagram, "Node");
    let declaration = if label.is_empty() {
        id.clone()
    } else {
        format!("{id}(label={})", quote(label))
    };
    Ok((id, vec![insert_semantic(source, &diagram, &declaration)?]))
}

pub fn add_relation(
    source: &str,
    from: &str,
    to: &str,
    kind: Connection,
) -> Result<(String, Vec<Edit>), String> {
    add_relation_operator(source, from, to, kind.operator(), None)
}
pub fn add_relation_operator(source: &str, from: &str, to: &str, operator: &str, label: Option<&str>) -> Result<(String, Vec<Edit>), String> {
    let diagram = parse(source)?;
    for id in [from, to] {
        if !diagram.nodes.iter().any(|node| node.has_endpoint(id)) {
            return Err(format!("Unknown node: {id}"));
        }
    }
    let prefix = if diagram.settings.strict_case {
        "relation"
    } else {
        "Relation"
    };
    let id = fresh_id(&diagram, prefix);
    let relation = crate::notation::Notation::of(source).relation(operator).ok_or("Choose a relationship supported by this diagram")?;
    let mut declaration = format!("@{id} {}{from} {} {to}", relation.prefix, relation.operator);
    if !relation.stereotype.is_empty() { declaration.push_str(&format!(" <<{}>>", relation.stereotype)); }
    if let Some(label) = label.filter(|s| !s.is_empty()) { declaration.push_str(&format!(" : {}", quote(label))); }
    Ok((id, vec![insert_semantic(source, &diagram, &declaration)?]))
}

fn insert_semantic(source: &str, diagram: &Diagram, declaration: &str) -> Result<Edit, String> {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let index = SourceIndex::new(source);
    // Count only compiler-recognized blocks. Note text ending in '{' is not a block.
    let openings: BTreeSet<usize> = diagram
        .nodes
        .iter()
        .map(|node| node.line)
        .chain(diagram.styling.iter().filter_map(|entry| match entry {
            StyleEntry::Rule(rule) => Some(rule.line),
            _ => None,
        }))
        .collect();
    let mut depth = 0;
    let mut semantics = false;
    for (i, line) in source.lines().enumerate() {
        let text = code(line).trim();
        if depth == 0
            && matches!(
                text,
                "semantics {" | "settings {" | "layout {" | "styling {"
            )
        {
            semantics = text == "semantics {";
            depth = 1;
        } else if text == "}" {
            if semantics && depth == 1 {
                let position = index
                    .line(i + 1)
                    .ok_or("Missing semantic block boundary")?
                    .start;
                return Ok(Edit {
                    range: position..position,
                    replacement: format!("    {declaration}{newline}"),
                });
            }
            depth -= 1;
        } else if openings.contains(&(i + 1)) && text.ends_with('{') {
            depth += 1;
        }
    }
    if semantics {
        return Err("Cannot locate the semantic block boundary".into());
    }
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        newline
    };
    Ok(Edit {
        range: source.len()..source.len(),
        replacement: format!(
            "{separator}{newline}semantics {{{newline}    {declaration}{newline}}}{newline}"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    fn apply(source: &str, edits: Vec<Edit>) -> String {
        let mut doc = Document::new(source.into());
        doc.apply(0, "Create", edits).unwrap();
        doc.source().into()
    }
    #[test]
    fn creates_escaped_label_and_collision_free_id_without_layout() {
        let source = "// keep\r\nsemantics {\r\n Node1\r\n}\r\n";
        let (id, edits) = add_node(source, "Hello \"world\"\nGrün").unwrap();
        assert_eq!(id, "Node2");
        let updated = apply(source, edits);
        let diagram = parse(&updated).unwrap();
        assert_eq!(
            diagram.nodes[1].label.as_deref(),
            Some("Hello \"world\"\nGrün")
        );
        assert!(updated.starts_with("// keep\r\n"));
        assert!(diagram.layout.is_empty());
        assert_eq!(
            updated.replace("    Node2(label=\"Hello \\\"world\\\"\\nGrün\")\r\n", ""),
            source
        );
    }
    #[test]
    fn adds_named_relation_for_each_operator_without_placement() {
        let source = "semantics {\n A\n B\n}\n";
        for kind in [
            Connection::Directed,
            Connection::Bidirectional,
            Connection::Undirected,
        ] {
            let (id, edits) = add_relation(source, "A", "B", kind).unwrap();
            let diagram = parse(&apply(source, edits)).unwrap();
            assert_eq!(diagram.relations[0].name.as_deref(), Some(id.as_str()));
            assert!(diagram.layout.is_empty());
        }
        assert!(add_relation(source, "A", "Missing", Connection::Directed).is_err());
    }
    #[test]
    fn creates_first_semantics_and_inserts_outside_note_body() {
        let (_, edits) = add_node("", "First").unwrap();
        assert_eq!(parse(&apply("", edits)).unwrap().nodes.len(), 1);
        let source = "semantics {\n note N {\n example {\n }\n}\n";
        let (_, edits) = add_node(source, "Second").unwrap();
        let updated = apply(source, edits);
        let diagram = parse(&updated).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert!(updated.starts_with("semantics {\n note N {\n example {\n }\n"));
    }
    #[test]
    fn node_ids_do_not_collide_with_relation_ids() {
        let source = "semantics {\n A\n B\n @Node1 A -> B\n}\n";
        assert_eq!(add_node(source, "").unwrap().0, "Node2");
    }
}
