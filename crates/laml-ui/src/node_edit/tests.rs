use super::*;
fn apply(source: &str, edits: Vec<Edit>) -> String {
    let updated = candidate(source, &edits).unwrap();
    diagram(&updated).unwrap();
    updated
}
#[test]
fn node_label_changes_do_not_rename_ids_or_touch_other_attributes() {
    let source = "// ä\r\nsemantics {\r\n A(style-classes=accent; label=\"A, old\") // keep\r\n B\r\n A -> B : A\r\n}\r\n";
    let updated = apply(source, label(source, "A", "New \"label\"").unwrap());
    assert_eq!(
        updated,
        source.replace("\"A, old\"", "\"New \\\"label\\\"\"")
    );
    let updated = apply(source, label(source, "B", "Label").unwrap());
    assert_eq!(updated, source.replace(" B\r\n", " B(label=\"Label\")\r\n"));
}
#[test]
fn note_labels_support_inline_and_block_text_with_literal_braces() {
    let source = "semantics {\n note N {\n example {\n }\n note M \"old\" // keep\n}\n";
    let updated = apply(source, label(source, "N", "new\ntext").unwrap());
    assert!(updated.contains("note N {\n    new\n    text\n }"));
    assert_eq!(
        apply(source, label(source, "M", "new").unwrap()),
        source.replace("\"old\"", "\"new\"")
    );
    let changed = apply(source, label(source, "N", "bad\n}").unwrap());
    assert_eq!(
        diagram(&changed)
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.id == "N")
            .unwrap()
            .label
            .as_deref(),
        Some("bad\n}")
    );
}
#[test]
fn renames_exact_resolved_references_not_labels_comments_or_longer_ids() {
    let source = "semantics {\n A\n AB(label=\"A\")\n @r A -> AB : \"A -> AB\" // A\n}\nlayout {\n A < AB\n A ? AB : align=center-y\n}\nstyling {\n node[A] {\n fill: #ffffff\n }\n}\n";
    let updated = apply(source, rename_id(source, "A", "Alpha").unwrap());
    assert!(updated.contains("@Alpha A\n"));
    assert!(updated.contains("AB(label=\"A\")"));
    assert!(updated.contains("@r Alpha -> AB : \"A -> AB\" // A"));
    assert!(updated.contains("Alpha < AB"));
    assert!(updated.contains("Alpha ? AB"));
    assert!(updated.contains("node[Alpha]"));
    assert!(rename_id(source, "A", "AB").is_err());
}
#[test]
fn renames_scoped_node_and_container_with_ports_and_contextual_styles() {
    let source = "semantics {\n package P {\n A {\n port out\n }\n B\n A::out -> B\n }\n X\n P.A -> X\n}\nlayout {\n P.A < P.B\n port[P.A::out] : side=right\n}\nstyling {\n node[P] node[A] {\n fill: #ffffff\n }\n node[P.A] {\n border-width: 2\n }\n}\n";
    let updated = apply(source, rename_id(source, "P.A", "Renamed").unwrap());
    assert!(updated.contains("Renamed::out -> B"));
    assert!(updated.contains("Renamed -> X"));
    assert!(updated.contains("node[P] node[Renamed]"));
    assert!(updated.contains("port[Renamed::out]"));
    let updated = apply(source, rename_id(source, "P", "Group").unwrap());
    assert!(updated.contains("@Group package P"));
    assert!(updated.contains("Group.A::out -> Group.B"));
    assert!(updated.contains("node[Group] node[A]"));
    assert!(updated.contains("node[Group.A]"));
}
#[test]
fn rejects_global_shadowing_of_unrelated_scoped_references() {
    let source = "semantics {\n A\n package P {\n X\n Y\n X -> Y\n }\n}\n";
    assert!(rename_id(source, "A", "X").is_err());
}

#[test]
fn renaming_root_node_does_not_rewrite_an_unrelated_relative_style_selector() {
    let source = "semantics {\n A\n package P {\n A\n }\n}\nstyling {\n node[P] node[A] {\n fill: #ffffff\n }\n port[A] {\n fill: #ffffff\n }\n}\n";
    let updated = apply(source, rename_id(source, "A", "Alpha").unwrap());
    assert!(updated.contains("node[P] node[A]"));
    assert!(updated.contains("port[A]"));
}

#[test]
fn rename_rejects_relative_selector_that_would_gain_unrelated_targets() {
    let source = "semantics {\n package P {\n A\n Z\n }\n}\nstyling {\n node[P] node[A] {\n fill: #ffffff\n }\n}\n";
    assert!(rename_id(source, "P.A", "Z").is_err());
}

#[test]
fn note_labels_with_comments_and_styled_inline_notes_roundtrip_exactly() {
    for source in [
        "semantics {\n note N { // header\n original\n }\n}\n",
        "semantics {\n note N(style-classes=red) \"old\" // header\n}\n",
    ] {
        let updated = apply(source, label(source, "N", "literal // comment\n}").unwrap());
        let d = diagram(&updated).unwrap();
        assert_eq!(d.nodes[0].label.as_deref(), Some("literal // comment\n}"));
        assert!(updated.contains("// header"));
    }
}

#[test]
fn note_label_edits_keep_body_and_closing_annotations() {
    let source = "semantics {\n note N { // heading\n old // body annotation\n // standalone\n } // closing\n}\n";
    for value in ["new label", "literal // text\n}", ""] {
        let updated = apply(source, label(source, "N", value).unwrap());
        for comment in [
            "// heading",
            "// body annotation",
            "// standalone",
            "// closing",
        ] {
            assert!(updated.contains(comment), "{updated}");
        }
        assert_eq!(
            diagram(&updated).unwrap().nodes[0].label.as_deref(),
            Some(value)
        );
    }
}
