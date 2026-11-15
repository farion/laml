use super::*;
fn apply(s: &str, edits: Vec<Edit>) -> String {
    node_edit::candidate(s, &edits).unwrap()
}
#[test]
fn body_editor_creates_and_edits_members_without_touching_comments_or_crlf() {
    let source = "semantics {\r\n class A // keep header\r\n B\r\n}\r\n";
    let updated = apply(
        source,
        body(
            source,
            "A",
            "    field +value: String\n    --\n    method +run()\n",
        )
        .unwrap(),
    );
    assert!(updated.contains("class A { // keep header\r\n    field"));
    assert!(!updated.replace("\r\n", "").contains('\n'));
    let p = crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
    assert_eq!(p.diagram.nodes[0].parts.len(), 3);
    let changed = apply(
        &updated,
        header(&updated, "A", "abstract class A<T> {").unwrap(),
    );
    assert!(crate::preview::build(&changed, std::path::Path::new("test.laml")).is_ok());
    assert!(header(source, "A", "class Changed").is_err());
}
#[test]
fn member_address_assignment_preserves_edges_and_styles_then_allows_name_changes() {
    let source = "semantics {\n class C {\n field +name: String\n }\n Other\n C::name -> Other\n}\nstyling {\n member[C::name] {\n color: #123456\n }\n}\n";
    let updated = apply(source, assign_member_id(source, "C", 3).unwrap());
    assert!(updated.contains("@Member1 field +name"));
    assert!(updated.contains("Member1 -> Other"));
    assert!(updated.contains("member[Member1]"));
    let updated = apply(
        &updated,
        body(&updated, "C", "    @Member1 field +renamed: String").unwrap(),
    );
    let preview = crate::preview::build(&updated, std::path::Path::new("test.laml")).unwrap();
    assert_eq!(preview.diagram.relations[0].from, "Member1");
    assert_eq!(
        member_value(&preview, "C", "Member1", "color"),
        Some("#123456".into())
    );
    assert!(body(source, "C", "    field +changed: String").is_err());
}
#[test]
fn notes_can_attach_to_unnamed_arrows_and_detach_without_deleting_the_note() {
    let source = "semantics {\n A\n B\n A -> B\n note N \"Why\"\n}\n";
    let attached = apply(source, attach(source, "N", 4).unwrap());
    let p = crate::preview::build(&attached, std::path::Path::new("test.laml")).unwrap();
    assert_eq!(p.diagram.attachments.len(), 1);
    assert!(p.diagram.relations[0].name.is_some());
    let removed = apply(
        &attached,
        remove_attachment(&attached, p.diagram.attachments[0].line).unwrap(),
    );
    let p = crate::preview::build(&removed, std::path::Path::new("test.laml")).unwrap();
    assert!(p.diagram.attachments.is_empty());
    assert_eq!(p.diagram.nodes.len(), 3);
}

#[test]
fn body_input_cannot_escape_into_global_styling_or_layout() {
    let source = "semantics {\n class C {\n field x\n }\n}\n";
    assert!(
        body(
            source,
            "C",
            "field x\n}\n}\nstyling {\n node[C] {\n color: #ff0000"
        )
        .is_err()
    );
    assert!(
        body(
            source,
            "C",
            "field x\n}\n}\nlayout {\n C ? C : align=center\n//"
        )
        .is_err()
    );
}
