use super::*;
fn apply(source: &str, id: &str, parent: Option<&str>) -> String {
    let plan = plan(source, id, parent).unwrap();
    assert!(!plan.changes.is_empty());
    let mut doc = crate::document::Document::new(source.into());
    doc.apply(0, "Membership", plan.edits).unwrap();
    let updated = doc.source().into();
    doc.undo();
    assert_eq!(doc.source(), source);
    updated
}
#[test]
fn moves_leaf_into_and_out_of_package_preserving_comments_and_crlf() {
    let source = "// ä keep\r\nsemantics {\r\n A(label=\"A\") // node\r\n B\r\n package P {\r\n }\r\n A -> B : \"A to B\" // edge\r\n}\r\nstyling {\r\n node[A] {\r\n fill: #ffffff\r\n }\r\n}\r\n";
    let updated = apply(source, "A", Some("P"));
    assert!(updated.contains("P.A -> B : \"A to B\" // edge\r\n"));
    assert!(updated.contains("node[P.A]"));
    assert!(updated.contains(" A(label=\"A\") // node\r\n"));
    assert!(!updated.replace("\r\n", "").contains('\n'));
    let moved = apply(&updated, "P.A", None);
    let diagram = node_edit::diagram(&moved).unwrap();
    assert!(
        diagram
            .nodes
            .iter()
            .any(|n| n.id == "A" && n.parent.is_none())
    );
    assert!(moved.contains("A -> B : \"A to B\" // edge"));
}
#[test]
fn nested_packages_rebase_descendants_ports_and_all_layout_rules() {
    let source = "semantics {\n package P {\n A {\n port Out\n }\n B\n A::Out -> B\n }\n package Q {\n }\n X\n P.A -> X\n}\nlayout {\n P.A < P.B\n port[P.A::Out] : side=right\n}\n";
    let updated = apply(source, "P", Some("Q"));
    assert!(updated.contains("Q.P.A::Out -> Q.P.B"));
    assert!(updated.contains("Q.P.A -> X"));
    assert!(updated.contains("Q.P.A < Q.P.B"));
    assert!(updated.contains("port[Q.P.A::Out]"));
    let diagram = node_edit::diagram(&updated).unwrap();
    assert_eq!(diagram.relations.len(), 2);
    assert_eq!(diagram.layout.len(), 1);
}
#[test]
fn explicit_ids_stay_global_and_relative_context_styles_stay_in_old_scope() {
    let source = "semantics {\n package P {\n @Svc Service\n }\n package Q {\n }\n X\n Svc -> X\n}\nstyling {\n node[P] node[Svc] {\n fill: #ffffff\n }\n}\n";
    let updated = apply(source, "Svc", Some("Q"));
    assert!(updated.contains("@Svc Service"));
    assert!(updated.contains("Svc -> X"));
    assert!(updated.contains("node[P] node[Svc]"));
}
#[test]
fn collisions_cycles_and_reference_capture_are_rejected() {
    let source = "semantics {\n A\n package P {\n A\n package Nested {\n }\n }\n}\n";
    assert!(plan(source, "A", Some("P")).is_err());
    assert!(plan(source, "P", Some("P.Nested")).is_err());
    let source = "semantics {\n B\n package P {\n A\n A -> B\n }\n package Q {\n B\n }\n}\n";
    // Moving P into Q changes the unqualified root B reference. No absolute root
    // escape exists in this syntax, so reject rather than changing the relation.
    assert!(plan(source, "P", Some("Q")).is_err());
}
#[test]
fn membership_does_not_delete_incompatible_mandatory_layout() {
    let source = "semantics {\n A\n B\n package P {\n }\n}\nlayout {\n A ? B : align=center\n}\n";
    let result = plan(source, "A", Some("P"));
    if let Ok(plan) = result {
        let updated = node_edit::candidate(source, &plan.edits).unwrap();
        assert!(updated.contains("P.A ? B : align=center"));
        assert_eq!(node_edit::diagram(&updated).unwrap().alignments.len(), 1);
    }
}
