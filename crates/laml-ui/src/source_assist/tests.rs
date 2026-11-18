use super::*;
fn labels(s: &str) -> Vec<String> {
    complete(s, s.len(), true)
        .entries
        .into_iter()
        .map(|e| e.label)
        .collect()
}
#[test]
fn section_settings_values_and_incomplete_source() {
    assert_eq!(labels("sett"), ["settings"]);
    assert!(labels("settings {\n dir").contains(&"direction:".into()));
    assert_eq!(labels("settings {\n direction: lef"), ["left-to-right"]);
    assert!(labels("settings {\n theme: modern-").contains(&"modern-dark".into()));
    assert!(complete("semantics {\n ", 14, false).entries.is_empty());
    assert!(labels("settings {\n type: ").len() >= 19);
}
#[test]
fn attributes_values_and_duplicates() {
    assert_eq!(labels("semantics {\n A(lab"), ["label="]);
    assert!(labels("semantics {\n A(label=\"Text\"; ").contains(&"style-classes=".into()));
    assert!(!labels("settings {\n theme: modern\n ").contains(&"theme:".into()));
    assert_eq!(
        labels("layout {\n A ? B : align=center-"),
        ["center-x", "center-y"]
    );
    assert_eq!(labels("styling {\n node {\n display: n"), ["none"]);
}
#[test]
fn comments_labels_and_strings_remain_quiet() {
    for source in [
        "// sett",
        "semantics {\n A(label=\"sett",
        "semantics {\n A -> B : sett",
        "semantics {\n note N {\n sett",
    ] {
        assert!(labels(source).is_empty(), "{source}");
    }
    assert!(labels("// unfinished \"\nsettings {\n th").contains(&"theme:".into()));
}
#[test]
fn operators_match_notation_and_have_explanations() {
    for (ty, op) in [
        (Notation::Sequence, "->>"),
        (Notation::Bpmn, "~~>"),
        (Notation::Class, "--|>"),
        (Notation::Component, "=="),
    ] {
        let s = format!("settings {{\n type: {}\n}}\nsemantics {{\n A ", ty.key());
        let result = complete(&s, s.len(), true);
        assert!(
            result
                .entries
                .iter()
                .any(|e| e.label == op && !e.detail.is_empty()),
            "{s}"
        );
    }
    let s = "settings {\n type: sequence\n}\nsemantics {\n A -";
    assert!(labels(s).contains(&"->>".into()));
    assert!(!labels(s).contains(&"--|>".into()));
}
#[test]
fn every_notation_has_constructs_even_when_incomplete() {
    for ty in Notation::ALL {
        let s = if ty == Notation::Gne {
            "semantics {\n ".into()
        } else {
            format!("settings {{\n type: {}\n}}\nsemantics {{\n ", ty.key())
        };
        assert!(!labels(&s).is_empty(), "{}", ty.key());
    }
}
#[test]
fn unicode_ranges_replace_whole_token_and_keep_suffix() {
    let s = "// Grüße\nsettings {\n direction: left-to-right\n}";
    let caret = s.find("left").unwrap() + 2;
    let c = complete(s, caret, true);
    assert_eq!(&s[c.range], "left-to-right");
    assert!(c.entries.iter().any(|e| e.label == "left-to-right"));
}
#[test]
fn references_survive_incomplete_buffer_and_are_qualified() {
    let s = "semantics {\n package Group {\n A\n }\n B\n B -> Gr";
    assert!(labels(s).contains(&"Group.A".into()));
}
#[test]
fn snippets_preserve_blocks_indentation_crlf_and_unicode_ranges() {
    let e = snippet::expand("class {id} {\n    {content}\n}", "  ", "\r\n");
    assert_eq!(e.text, "class NewElement {\r\n      \r\n  }");
    assert_eq!(&e.text[e.fields[0].clone()], "NewElement");
    let mut session = snippet::Session::new(&e, 0, 1).unwrap();
    let next = e.text.replacen("NewElement", "Über", 1);
    assert!(session.update(&e.text, &next, 2));
    assert_eq!(&next[session.selection()], "Über");
    assert!(session.step(false).unwrap().is_empty());
    assert!(session.step(false).is_none());
}
#[test]
fn pair_insertion_wrap_skip_delete_and_opaque_text() {
    let change = pairs::typed("日本語", 0..9, '(').unwrap();
    assert_eq!(change.edits[0].replacement, "(日本語)");
    assert_eq!(change.selection, 1..10);
    assert!(pairs::typed("// ", 3..3, '(').is_none());
    assert!(pairs::typed("A(label=\"hi", 11..11, '(').is_none());
    assert!(pairs::typed("()", 1..1, ')').unwrap().edits.is_empty());
    assert_eq!(pairs::backspace("[]", 1..1).unwrap().edits[0].range, 0..2);
    assert!(pairs::backspace("// []", 4..4).is_none());
}
#[test]
fn values_open_after_assignment_and_quoted_catalogs_do_not_duplicate_quotes() {
    let s = "layout {\n A ? B : align=";
    assert!(
        complete(s, s.len(), false)
            .entries
            .iter()
            .any(|e| e.label == "center-y")
    );
    let s = "styling {\n node {\n icon: \"dat\"";
    let result = complete(s, s.len() - 1, false);
    assert!(
        result
            .entries
            .iter()
            .any(|e| e.label == "database" && e.template == "database")
    );
    let s = "semantics {\n A(label=\"x(y)\"; style-classes=hot; ";
    assert!(!labels(s).contains(&"style-classes=".into()));
    let s = "settings {\n type: c4-container\n}\nsemantics {\n person P(";
    assert!(!labels(s).contains(&"appearance=".into()));
    let s = s.replace("person P", "container C");
    assert!(labels(&s).contains(&"appearance=".into()));
}
#[test]
fn member_bodies_and_references_do_not_invent_elements() {
    assert_eq!(labels("semantics {\n enum E {\n "), ["literal"]);
    assert!(!labels("semantics {\n interface I {\n ").contains(&"port".into()));
    let s = "semantics {\n class A {\n +field: String\n port output\n }\n note N {\n bogus\n }\n B\n B -> ";
    let names = labels(s);
    assert!(names.contains(&"A.output".into()));
    assert!(!names.contains(&"bogus".into()));
    assert!(!names.contains(&"port".into()));
}
