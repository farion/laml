use super::*;
use crate::document::Document;
fn apply(source: &str, ids: &[&str], relations: &[usize], mode: ContainerMode) -> String {
    let plan = plan(
        source,
        &ids.iter().map(|id| (*id).into()).collect::<Vec<_>>(),
        relations,
        mode,
    )
    .unwrap();
    assert!(!plan.affected.is_empty());
    let mut doc = Document::new(source.into());
    doc.apply(0, "Delete", plan.edits).unwrap();
    let updated = doc.source().to_string();
    doc.undo();
    assert_eq!(doc.source(), source);
    node_edit::diagram(&updated).unwrap();
    updated
}
#[test]
fn deletes_node_incident_edges_attachments_layout_and_local_styles_atomically() {
    let source = "// untouched\r\nsemantics {\r\n A\r\n AB\r\n C\r\n note N \"keep A\"\r\n @r A -> AB // edge comment\r\n AB -> C\r\n N .. relation[r]\r\n}\r\nlayout {\r\n A < AB // rule comment\r\n AB < C\r\n relation[r].source : side=right\r\n}\r\nstyling {\r\n node[A] {\r\n fill: #ffffff\r\n }\r\n node[AB] {\r\n fill: #000000\r\n }\r\n}\r\n";
    let updated = apply(source, &["A"], &[], ContainerMode::WithContents);
    assert!(updated.starts_with("// untouched\r\n"));
    assert!(updated.contains("note N \"keep A\""));
    assert!(updated.contains("AB -> C"));
    assert!(updated.contains("AB < C"));
    assert!(updated.contains("// edge comment\r\n"));
    assert!(updated.contains("// rule comment\r\n"));
    assert!(!updated.contains("relation[r]"));
    assert!(!updated.contains("node[A]"));
    assert!(updated.contains("node[AB]"));
}
#[test]
fn deletes_only_selected_parallel_relation_and_preserves_nodes() {
    let source = "semantics {\n A\n B\n @one A -> B\n @two A -> B\n}\n";
    assert_eq!(
        apply(source, &[], &[4], ContainerMode::WithContents),
        source.replace(" @one A -> B\n", "")
    );
}
#[test]
fn nested_container_deletion_covers_default_explicit_children_and_ports() {
    let source = "semantics {\n package P {\n A {\n port out\n }\n @Global B\n A::out -> Global\n }\n X\n P.A::out -> X\n Global -> X\n}\n";
    let updated = apply(source, &["P", "P.A"], &[], ContainerMode::WithContents);
    assert_eq!(node_edit::diagram(&updated).unwrap().nodes.len(), 1);
    assert!(updated.contains(" X\n"));
}
#[test]
fn unwrap_preserves_contents_and_rebases_qualified_references() {
    let source = "semantics {\n package P {\n A\n B\n A -> B\n }\n X\n P.A -> X\n}\nlayout {\n P.A < P.B\n}\nstyling {\n node[P.A] {\n fill: #ffffff\n }\n}\n";
    let updated = apply(source, &["P"], &[], ContainerMode::KeepContents);
    let diagram = node_edit::diagram(&updated).unwrap();
    assert_eq!(
        diagram
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "X"]
    );
    assert!(updated.contains("A -> B"));
    assert!(updated.contains("A -> X"));
    assert!(updated.contains("A < B"));
    assert!(updated.contains("node[A]"));
}
#[test]
fn unwrap_collision_is_rejected_without_changing_source() {
    let source = "semantics {\n A\n package P {\n A\n }\n}\n";
    assert!(plan(source, &["P".into()], &[], ContainerMode::KeepContents).is_err());
}

#[test]
fn deleting_root_node_keeps_unrelated_contextual_and_port_style_rules() {
    let source = "semantics {\n A\n package P {\n A\n }\n}\nstyling {\n node[P] node[A] {\n fill: #ffffff\n }\n port[A] {\n fill: #ffffff\n }\n}\n";
    let updated = apply(source, &["A"], &[], ContainerMode::WithContents);
    assert!(updated.contains("node[P] node[A]"));
    assert!(updated.contains("port[A]"));
}

#[test]
fn selecting_an_attachment_deletes_only_the_attachment_not_its_endpoints() {
    let source = "semantics {\n A\n B\n note N \"annotation\"\n @r A -> B\n N .. relation[r]\n}\n";
    let plan = super::plan(source, &[], &[6], ContainerMode::WithContents).unwrap();
    let updated = crate::node_edit::candidate(source, &plan.edits).unwrap();
    let diagram = crate::node_edit::diagram(&updated).unwrap();
    assert_eq!(diagram.nodes.len(), 3);
    assert_eq!(diagram.relations.len(), 1);
    assert!(diagram.attachments.is_empty());
}
