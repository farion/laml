use super::*;
fn apply(source: &str, edits: Vec<Edit>) -> String {
    let updated = node_edit::candidate(source, &edits).unwrap();
    node_edit::diagram(&updated).unwrap();
    updated
}
#[test]
fn creates_ports_in_leaf_and_existing_body_without_losing_comments() {
    for declaration in [" A // keep\r\n", " A { // keep\r\n  +run()\r\n }\r\n"] {
        let source = format!("semantics {{\r\n{declaration}}}\r\n");
        let (id, edits) = add(&source, "A", "Input").unwrap();
        let updated = apply(&source, edits);
        assert!(updated.contains("// keep\r\n"));
        assert_eq!(
            find(&node_edit::diagram(&updated).unwrap(), &id)
                .unwrap()
                .1
                .label,
            "Input"
        );
        crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
    }
}
#[test]
fn port_name_edit_requires_an_explicit_id_and_updates_no_endpoint_references() {
    let source = "semantics {\n A {\n port Out\n }\n B\n A::Out -> B\n}\n";
    assert!(label(source, "A::Out", "Output").is_err());
    let named = apply(source, rename_id(source, "A::Out", "OutputPort").unwrap());
    assert!(named.contains("@OutputPort port Out"));
    assert!(named.contains("OutputPort -> B"));
    let updated = apply(&named, label(&named, "OutputPort", "Output").unwrap());
    assert!(updated.contains("@OutputPort port Output"));
    assert!(updated.contains("OutputPort -> B"));
    assert!(label(&named, "OutputPort", "Out\n@Evil port Extra").is_err());
    assert!(add(source, "A", "Out\n@Evil port Extra").is_err());
}
#[test]
fn sides_and_style_reset_preserve_comments_and_other_properties() {
    let source = "semantics {\n A {\n @P port In\n }\n}\nlayout {\n port[P] : side=left // keep\n}\nstyling {\n port[P] {\n fill: #ffffff // fill comment\n color: #000000\n }\n}\n";
    let updated = apply(source, side(source, &["P".into()], None).unwrap());
    assert!(updated.contains("// keep\n"));
    assert!(!updated.contains("side=left"));
    let updated = apply(
        source,
        property(source, &["P".into()], "fill", None).unwrap(),
    );
    assert!(updated.contains("// fill comment"));
    assert!(updated.contains("color: #000000"));
}

#[test]
fn explicit_port_rename_preserves_decorator_classes_and_comments() {
    let source = "semantics {\n A {\n @P(style-classes=red) port Out // keep\n }\n B\n P -> B\n}\n";
    let updated = apply(source, rename_id(source, "P", "Renamed").unwrap());
    assert!(updated.contains("@Renamed(style-classes=red) port Out // keep"));
    assert_eq!(
        find(&node_edit::diagram(&updated).unwrap(), "Renamed")
            .unwrap()
            .1
            .classes,
        ["red"]
    );
}

#[test]
fn generated_global_port_ids_do_not_capture_scoped_default_nodes() {
    let source = "semantics {\n A\n package P {\n Port1\n B\n Port1 -> B\n }\n}\n";
    let (id, edits) = add(source, "A", "Output").unwrap();
    assert_eq!(id, "Port2");
    let updated = apply(source, edits);
    assert_eq!(
        node_edit::diagram(&updated).unwrap().relations[0].from,
        "P.Port1"
    );
}

#[test]
fn port_definition_edits_class_lists_without_changing_identity_or_references() {
    let source = "semantics {\n A {\n @P(style-classes=old) port Out // keep\n }\n B\n P -> B\n}\n";
    let updated = apply(
        source,
        definition(
            source,
            "P",
            "@P(style-classes=first) port Out(style-classes=second,third)",
        )
        .unwrap(),
    );
    let d = node_edit::diagram(&updated).unwrap();
    assert_eq!(find(&d, "P").unwrap().1.classes, ["second", "third"]);
    assert_eq!(d.relations[0].from, "P");
    assert!(updated.contains("// keep"));
    assert!(definition(source, "P", "@Other port Out").is_err());
    assert!(definition(source, "P", "@P port Out\nB").is_err());
}
