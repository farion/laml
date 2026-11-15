use super::*;
use crate::{document::Document, preview};
use std::path::Path;

fn apply(source: &str, edits: Vec<Edit>) -> Document {
    let mut doc = Document::new(source.into());
    doc.apply(0, "Graph gesture", edits).unwrap();
    doc
}

#[test]
fn named_nodes_keep_crlf_comments_shapes_and_atomic_history() {
    let source = "// Grüße\r\nsemantics {\r\n A // keep\r\n}\r\n";
    for kind in element_edit::ElementKind::available(source) {
        let edits = add_node(source, kind, "Custom", "Display label").unwrap();
        let mut doc = apply(source, edits);
        let model = node_edit::diagram(doc.source()).unwrap();
        assert!(
            model
                .nodes
                .iter()
                .any(|n| n.id == "Custom" && n.kind == kind.node_kind())
        );
        assert!(doc.source().contains("A // keep\r\n"));
        assert!(!doc.source().replace("\r\n", "").contains('\n'));
        preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
        assert!(doc.undo());
        assert_eq!(doc.source(), source);
        assert!(!doc.undo());
    }
}

#[test]
fn node_and_port_ids_cannot_collide_and_ports_have_explicit_sides() {
    let source = "semantics {\n A\n B\n @r A -> B\n}\n";
    assert!(add_node(source, element_edit::ElementKind::Rectangle, "A", "Other").is_err());
    assert!(
        add_node(
            source,
            element_edit::ElementKind::Rectangle,
            "bad id",
            "Other"
        )
        .is_err()
    );
    let edits = add_port(source, "A", "Boundary", "Requests", "top").unwrap();
    let mut doc = apply(source, edits);
    let model = node_edit::diagram(doc.source()).unwrap();
    let (owner, port) = port_edit::find(&model, "Boundary").unwrap();
    assert_eq!(owner.id, "A");
    assert_eq!(port.label, "Requests");
    assert_eq!(model.port_layout[0].side, laml::syntax::PortSide::Top);
    assert!(add_port(doc.source(), "B", "Boundary", "Other", "right").is_err());
    preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
    doc.undo();
    assert_eq!(doc.source(), source);
}

#[test]
fn new_relations_cover_types_self_loops_parallel_paths_and_side_combinations() {
    let source = "semantics {\n A\n B\n}\n";
    for operator in OPERATORS {
        for (from_side, to_side) in [
            (None, None),
            (Some("right"), None),
            (None, Some("top")),
            (Some("left"), Some("bottom")),
        ] {
            let (id, edits) = add_relation(source, "A", "B", operator, from_side, to_side).unwrap();
            let mut doc = apply(source, edits);
            let model = node_edit::diagram(doc.source()).unwrap();
            assert!(
                model
                    .relations
                    .iter()
                    .any(|r| r.name.as_ref() == Some(&id) && r.from == "A" && r.to == "B")
            );
            assert_eq!(
                model.relation_end_layout.len(),
                usize::from(from_side.is_some()) + usize::from(to_side.is_some())
            );
            preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
            doc.undo();
            assert_eq!(doc.source(), source);
        }
    }
    let (_, edits) = add_relation(source, "A", "A", "->", None, None).unwrap();
    let doc = apply(source, edits);
    preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
    let (_, edits) = add_relation(source, "A", "B", "->", None, None).unwrap();
    let doc = apply(source, edits);
    let (_, edits) = add_relation(doc.source(), "A", "B", "--", None, None).unwrap();
    let doc = apply(doc.source(), edits);
    assert_eq!(node_edit::diagram(doc.source()).unwrap().relations.len(), 2);
    preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
}

#[test]
fn reconnect_handles_reversed_operators_early_layout_end_labels_and_reset() {
    let source = "layout {\n relation[r].source : side=left; role=below // keep role\n relation[r].target : side=right\n}\nsemantics {\n A\n B\n C\n @r A <- /owner B : \"message\" // label\n}\nstyling {\n relation[r] {\n color: #123456\n }\n}\n";
    let original = node_edit::diagram(source).unwrap();
    let line = original.relations[0].line;
    let (id, edits) = reconnect(source, line, EndPosition::Source, "C", None).unwrap();
    assert_eq!(id, "r");
    let mut doc = apply(source, edits);
    let updated = node_edit::diagram(doc.source()).unwrap();
    assert_eq!(updated.relations[0].from, "C");
    assert_eq!(updated.relations[0].to, "A");
    assert_eq!(updated.relations[0].label.as_deref(), Some("message"));
    assert!(doc.source().contains("role=below // keep role"));
    assert!(doc.source().contains("color: #123456"));
    assert!(doc.source().contains("target : side=right"));
    assert!(!doc.source().contains("side=left"));
    doc.undo();
    assert_eq!(doc.source(), source);
}

#[test]
fn reconnection_to_ports_is_not_replaced_by_attachment_to_their_owner() {
    let source = "semantics {\n A {\n @P port Requests\n }\n B\n @r B -> A\n}\n";
    let model = node_edit::diagram(source).unwrap();
    let (_, edits) = reconnect(
        source,
        model.relations[0].line,
        EndPosition::Target,
        "P",
        None,
    )
    .unwrap();
    let doc = apply(source, edits);
    assert_eq!(
        node_edit::diagram(doc.source()).unwrap().relations[0].to,
        "P"
    );
    preview::build_dark(doc.source(), Path::new("test.laml")).unwrap();
}
