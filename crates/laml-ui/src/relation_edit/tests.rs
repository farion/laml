use super::*;
use crate::document::Document;
fn apply(source: &str, edits: Vec<Edit>) -> String {
    let mut doc = Document::new(source.into());
    doc.apply(0, "Edit relation", edits).unwrap();
    laml::parser::parse(doc.source()).unwrap();
    let result = doc.source().to_owned();
    if doc.undo() {
        assert_eq!(doc.source(), source);
    }
    result
}
#[test]
fn independently_edits_parallel_unnamed_relations_preserving_crlf_comments() {
    let source = "// ä keep\r\nsemantics {\r\n A\r\n B\r\n A  ->  B : \"first -> B\" // keep\r\n A -> B : second\r\n}\r\n";
    let updated = apply(source, label(source, 5, "Grün \"hi\"\nB").unwrap());
    assert!(updated.contains("A  ->  B : \"Grün \\\"hi\\\"\\nB\" // keep\r\n"));
    assert!(updated.contains("A -> B : second\r\n"));
    assert_eq!(
        apply(source, change_operator(source, 6, "<->").unwrap()),
        source.replace("A -> B : second", "A <-> B : second")
    );
    let updated = apply(source, label(source, 5, "").unwrap());
    assert!(updated.contains("A  ->  B  // keep"));
}
#[test]
fn resetting_side_keeps_other_end_properties_and_comments() {
    for attrs in [
        "side=right; role=below",
        "role=below; side=right",
        "multiplicity=above; side=right; role=below",
    ] {
        let source = format!(
            "semantics {{\n A\n B\n @request A \"1\"/owner -> B\n}}\nlayout {{\n relation[request].source : {attrs} // keep\n relation[request].target : side=left\n}}\n"
        );
        let updated = apply(
            &source,
            endpoint_side(&source, 4, EndPosition::Source, None).unwrap(),
        );
        assert!(!updated.contains("side=right"));
        assert!(updated.contains("role=below"));
        assert!(updated.contains("// keep"));
        assert!(updated.contains("relation[request].target : side=left"));
    }
}
#[test]
fn assigns_names_atomically_and_obeys_naming_convention() {
    let source = "settings {\n naming-convention: uppercase-nodes-lowercase-relations\n}\nsemantics {\n A\n B\n A -> B\n}\n";
    let updated = apply(
        source,
        endpoint_side(source, 7, EndPosition::Source, Some("right")).unwrap(),
    );
    assert!(updated.contains("@relation1 A -> B"));
    assert!(updated.contains("relation[relation1].source : side=right"));
    let updated = apply(
        source,
        property(source, 7, "color", Some("#123456")).unwrap(),
    );
    assert!(updated.contains("relation[relation1]"));
    assert!(updated.contains("color: #123456"));
}
#[test]
fn reconnect_targets_semantic_end_of_reversed_relation_and_preserves_adornments() {
    let source = "semantics {\n A\n B\n C\n @r A \"1\"/owner <-- /items B : \"B -> A\" // keep\n}\nlayout {\n relation[r].source : side=right\n}\n";
    let updated = apply(
        source,
        reconnect(source, 5, EndPosition::Source, "C").unwrap(),
    );
    assert_eq!(updated, source.replace("/items B :", "/items C :"));
    let updated = apply(
        source,
        reconnect(source, 5, EndPosition::Target, "C").unwrap(),
    );
    assert_eq!(updated, source.replace("@r A ", "@r C "));
    assert!(reconnect(source, 5, EndPosition::Source, "Missing").is_err());
}
#[test]
fn legacy_names_and_loop_labels_are_not_rewritten() {
    let source = "semantics {\n A\n loop(style-classes=accent): A -> A : \"x: y\"\n}\n";
    let updated = apply(source, label(source, 3, "updated").unwrap());
    assert_eq!(updated, source.replace("\"x: y\"", "\"updated\""));
}

#[test]
fn multi_endpoint_sides_name_each_relation_once_and_share_one_layout_section() {
    let source = "semantics {\n A\n B\n A -> B\n A -> B\n}\n";
    let updated = apply(
        source,
        endpoint_sides(source, &[4, 5], EndPosition::Source, Some("right")).unwrap(),
    );
    assert_eq!(updated.matches("layout {").count(), 1);
    assert!(updated.contains("@Relation1 A -> B"));
    assert!(updated.contains("@Relation2 A -> B"));
    assert!(updated.contains("relation[Relation1].source : side=right"));
    assert!(updated.contains("relation[Relation2].source : side=right"));
}

#[test]
fn full_definition_preserves_identity_neighbors_and_inline_comments() {
    let source = "semantics {\n A\n B\n @r A -- B // keep\n A -> B : other\n}\n";
    assert_eq!(definition_text(source, 4).unwrap(), "@r A -- B");
    let edits = definition(source, 4, "@r A \"1\"/owner -- \"*\"/items B : list").unwrap();
    let updated = crate::node_edit::candidate(source, &edits).unwrap();
    assert!(updated.contains("list // keep"));
    let d = laml::parser::parse(&updated).unwrap();
    assert_eq!(d.relations[0].source_end.role.as_deref(), Some("owner"));
    assert_eq!(d.relations[0].target_end.multiplicity.as_deref(), Some("*"));
    assert_eq!(d.relations[1].label.as_deref(), Some("other"));
    assert!(definition(source, 4, "@changed A -- B").is_err());
}
