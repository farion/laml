use super::*;
fn apply(source: &str, edits: Vec<Edit>) -> String {
    node_edit::candidate(source, &edits).unwrap()
}
#[test]
fn palette_builds_every_kind_from_an_empty_document() {
    for kind in ElementKind::available("") {
        let (id, edits) = add("", kind, "Hello \"world\"", None, None).unwrap();
        let source = apply("", edits);
        let preview = crate::preview::build(&source, std::path::Path::new("test.laml")).unwrap();
        assert_eq!(preview.diagram.nodes[0].id, id);
        assert_eq!(preview.nodes.len(), 1);
        if let Some(shape) = kind.shape() {
            assert_eq!(
                preview.node_properties[&id]["shape"].value.as_deref(),
                Some(shape)
            );
        }
        assert_eq!(preview.diagram.nodes[0].kind, kind.node_kind());
        assert_eq!(source.matches("semantics {").count(), 1);
    }
}
#[test]
fn inserts_nested_elements_with_unique_ids_relative_rules_and_crlf() {
    let source = "// keep\r\nsemantics {\r\n package P {\r\n Node1\r\n }\r\n}\r\n";
    let (id, edits) = add(
        source,
        ElementKind::Diamond,
        "Choice",
        Some("P"),
        Some(("P.Node1", Placement::Below)),
    )
    .unwrap();
    assert_eq!(id, "P.Node2");
    let updated = apply(source, edits);
    assert!(updated.starts_with("// keep\r\n"));
    assert!(updated.contains("P.Node2 below P.Node1"));
    assert_eq!(updated.replace("\r\n", "").matches('\n').count(), 0);
    let preview = crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
    assert_eq!(
        preview
            .diagram
            .nodes
            .iter()
            .find(|n| n.id == id)
            .unwrap()
            .parent
            .as_deref(),
        Some("P")
    );
    assert!(
        add(
            source,
            ElementKind::Rectangle,
            "",
            None,
            Some(("P.Node1", Placement::Left))
        )
        .is_err()
    );
}
#[test]
fn creating_notes_does_not_confuse_literal_note_braces_with_container_structure() {
    let source = "semantics {\n note N {\n literal {\n }\n}\n";
    let (_, edits) = add(source, ElementKind::Package, "", None, None).unwrap();
    let updated = apply(source, edits);
    assert!(updated.starts_with("semantics {\n note N {\n literal {\n }\n"));
    assert_eq!(node_edit::diagram(&updated).unwrap().nodes.len(), 2);
}

#[test]
fn nested_creation_does_not_capture_references_to_ancestor_defaults() {
    let source =
        "semantics {\n package Outer {\n Node1\n package Inner {\n B\n Node1 -> B\n }\n }\n}\n";
    let (id, edits) = add(
        source,
        ElementKind::Rectangle,
        "",
        Some("Outer.Inner"),
        None,
    )
    .unwrap();
    assert_eq!(id, "Outer.Inner.Node2");
    let updated = apply(source, edits);
    assert_eq!(
        node_edit::diagram(&updated).unwrap().relations[0].from,
        "Outer.Node1"
    );
}
