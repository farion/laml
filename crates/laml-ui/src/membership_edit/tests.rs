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
fn bare_package_destination_opens_body_and_keeps_header_comment() {
    let source = "settings {\r\n type: package\r\n}\r\nsemantics {\r\n package A\r\n package P // keep\r\n}\r\n";
    let updated = apply(source, "A", Some("P"));
    assert!(updated.contains("package P { // keep\r\n"));
    assert!(!updated.replace("\r\n", "").contains('\n'));
    let diagram = node_edit::diagram(&updated).unwrap();
    assert!(
        diagram
            .nodes
            .iter()
            .any(|n| n.id == "P.A" && n.parent.as_deref() == Some("P"))
    );
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
fn screenshot_node3_outside_rule_conflicts_with_container_membership() {
    let source = include_str!("../../../../tests/fixtures/gne-node3-container-drop.laml");
    let moved = plan(source, "Node3", Some("Group1")).unwrap();
    let candidate = node_edit::candidate(source, &moved.edits).unwrap();
    let diagram = node_edit::diagram(&candidate).unwrap();
    let styles = laml::styles::load(&diagram, std::path::Path::new("screenshot.laml")).unwrap();
    let error = laml::geometry::place(&diagram, &styles).unwrap_err();
    assert!(
        error
            .message
            .contains("cannot place a child outside its own container"),
        "{error:?}"
    );
}
#[test]
fn body_drop_replaces_only_direct_outside_constraints_and_preserves_comments() {
    let source = "semantics {\r\n @A A\r\n B\r\n package P {\r\n }\r\n A -> B\r\n}\r\nlayout {\r\n A below P // outside\r\n P > A\r\n B below P // keep\r\n}\r\n";
    let moved = plan_drop(source, "A", "P").unwrap();
    let mut doc = crate::document::Document::new(source.into());
    doc.apply(0, "Body drop", moved.edits).unwrap();
    let updated = doc.source();
    assert!(updated.contains("// outside\r\n"));
    assert!(updated.contains("B below P // keep\r\n"));
    assert!(updated.contains(" A -> B\r\n"));
    assert!(!updated.replace("\r\n", "").contains('\n'));
    let diagram = node_edit::diagram(updated).unwrap();
    assert_eq!(diagram.layout.len(), 1);
    assert_eq!(diagram.relations.len(), 1);
    assert!(
        diagram
            .nodes
            .iter()
            .any(|n| n.id == "A" && n.parent.as_deref() == Some("P"))
    );
    doc.undo();
    assert_eq!(doc.source(), source);
}
#[test]
fn screenshot_body_drop_renders_and_keeps_other_rules_and_relations() {
    let source = include_str!("../../../../tests/fixtures/gne-node3-container-drop.laml");
    let moved = plan_drop(source, "Node3", "Group1").unwrap();
    let candidate = node_edit::candidate(source, &moved.edits).unwrap();
    let preview =
        crate::preview::build(&candidate, std::path::Path::new("screenshot.laml")).unwrap();
    assert!(
        preview
            .diagram
            .nodes
            .iter()
            .any(|n| n.id == "Node3" && n.parent.as_deref() == Some("Group1"))
    );
    assert_eq!(preview.diagram.relations.len(), 6);
    assert!(candidate.contains("Group1 above Browser"));
    assert!(candidate.contains("Node2 ? Browser : align=left"));
    assert!(candidate.contains("relation[Relation4].target : side=bottom"));
    assert!(!candidate.contains("Node3 below Group1"));
}
#[test]
fn screenshot_browser_body_drop_renders_with_relation_to_its_package() {
    let source = include_str!("../../../../tests/fixtures/gne-browser-container-drop.laml");
    let moved = plan_drop(source, "Browser", "Group1").unwrap();
    let candidate = node_edit::candidate(source, &moved.edits).unwrap();
    let preview =
        crate::preview::build(&candidate, std::path::Path::new("browser-drop.laml")).unwrap();
    assert!(
        preview
            .diagram
            .nodes
            .iter()
            .any(|n| n.id == "Group1.Browser" && n.parent.as_deref() == Some("Group1"))
    );
    assert_eq!(preview.diagram.relations.len(), 6);
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
