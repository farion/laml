use super::*;
use crate::document::Document;
fn pick(source: &str, label: &str) -> (Preview, Text) {
    let preview = crate::preview::build(source, std::path::Path::new("inline.laml")).unwrap();
    let region = preview
        .texts
        .iter()
        .find(|t| t.text == label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", preview.texts));
    let picked = hit(
        &preview,
        source,
        region.x + region.width / 2.0,
        region.y + region.height / 2.0,
    )
    .unwrap_or_else(|| panic!("Uneditable {label}: {:?}", preview.texts));
    (preview, picked)
}
fn replace(source: &str, picked: &Text, value: &str) -> Document {
    let mut document = Document::new(source.into());
    document
        .apply(0, "Inline text", picked.edits(source, value).unwrap())
        .unwrap();
    document
}
#[test]
fn nodes_and_arrows_preserve_ids_comments_crlf_and_share_one_undo_step() {
    let source = "// retain\r\nsemantics {\r\n A(label=\"Alpha\") // unchanged\r\n B\r\n @r A -> B: \"Flow\" // arrow\r\n}\r\n";
    for (label, value) in [("Alpha", "Änderung\nSecond line"), ("Flow", "New Flow")] {
        let (_, picked) = pick(source, label);
        assert_eq!(picked.multiline, label == "Alpha");
        let mut document = replace(source, &picked, value);
        let preview =
            crate::preview::build(document.source(), std::path::Path::new("inline.laml")).unwrap();
        assert_eq!(preview.diagram.nodes[0].id, "A");
        assert_eq!(preview.diagram.relations[0].from, "A");
        assert!(document.source().contains("// unchanged\r\n"));
        assert!(document.source().contains("// arrow\r\n"));
        assert!(document.undo());
        assert_eq!(document.source(), source);
        assert!(!document.undo());
    }
}
#[test]
fn whitespace_is_not_a_text_target_and_repeated_labels_use_their_own_nodes() {
    let source = "semantics {\n A(label=\"Same\")\n B(label=\"Same\")\n A -> B\n}\n";
    let (preview, _) = pick(source, "Same");
    assert!(hit(&preview, source, -100.0, -100.0).is_none());
    let mut ids = vec![];
    for region in preview.texts.iter().filter(|t| t.text == "Same") {
        let text = hit(
            &preview,
            source,
            region.x + region.width / 2.0,
            region.y + region.height / 2.0,
        )
        .unwrap();
        let Target::Node(id) = text.target else {
            panic!("Expected node")
        };
        ids.push(id);
    }
    ids.sort();
    assert_eq!(ids, ["A", "B"]);
}
#[test]
fn sequence_labels_and_default_participant_labels_do_not_rename_references() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B: Request\n}\n";
    for label in ["A", "Request"] {
        let (_, picked) = pick(source, label);
        let document = replace(source, &picked, "Edited\nSecond");
        let parsed = laml::sequence::parse(document.source()).unwrap();
        assert_eq!(parsed.participants[0].id, "A");
        assert!(parsed.events.iter().any(
            |event| matches!(event, laml::sequence::Event::Message(message) if message.from == "A")
        ));
    }
}
#[test]
fn bpmn_inline_nodes_and_flow_labels_target_only_the_selected_semantics() {
    let source = "settings {\n type: bpmn\n}\nsemantics {\n A(\"Task\") --> B(\"Other\"): yes\n}\n";
    for label in ["Task", "yes"] {
        let (_, picked) = pick(source, label);
        let document = replace(source, &picked, "Changed");
        let doc = laml::bpmn::parse(document.source()).unwrap();
        assert_eq!(doc.edges[0].from, "A");
        assert_eq!(doc.edges[0].to, "B");
        assert_eq!(doc.nodes[1].label, "Other");
    }
}
#[test]
fn empty_sequence_labels_have_no_pill_and_arrows_remain_inline_editable() {
    for suffix in ["", ": \"\"", ": \"   \""] {
        let source = format!(
            "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n A -> B{suffix} // keep\n}}\n"
        );
        let preview = crate::preview::build(&source, std::path::Path::new("inline.laml")).unwrap();
        assert!(!preview.svg.contains("data-message-pill"));
        let message = preview
            .regions
            .iter()
            .find(|r| r.kind == "message" && !r.path.is_empty())
            .unwrap();
        let a = message.path[0];
        let b = message.path[1];
        let picked = hit(&preview, &source, (a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0).unwrap();
        let mut document = replace(&source, &picked, "New\nLabel");
        let parsed = laml::sequence::parse(document.source()).unwrap();
        assert!(parsed.events.iter().any(|e| matches!(e, laml::sequence::Event::Message(m) if m.label.as_deref() == Some("New\nLabel"))));
        assert!(document.source().contains("// keep"));
        assert!(document.undo());
        assert_eq!(document.source(), source);
    }
}

#[test]
fn sequence_fragment_conditions_edit_raw_text_not_rendered_brackets() {
    for operator in [
        "opt", "alt", "par", "loop", "critical", "break", "neg", "assert", "strict", "seq",
        "ignore", "consider",
    ] {
        let source = format!(
            "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n block: {operator} \"ready\" {{ // preserve header\n A ->> B: Work\n }}\n}}\n"
        );
        let label = if matches!(operator, "ignore" | "consider") {
            format!("{operator} {{ready}}")
        } else {
            "[ready]".into()
        };
        let (_, picked) = pick(&source, &label);
        assert_eq!(picked.value, "ready");
        assert!(!picked.multiline);
        assert!(matches!(picked.target, Target::SequenceGuard(_)));
        let mut document = replace(&source, &picked, "Menge > 0 && status == \"bereit\"");
        assert!(document.source().contains("// preserve header"));
        assert!(document.source().contains(" A ->> B: Work"));
        assert!(document.source().contains(&format!(
            "block: {operator} \"Menge > 0 && status == \\\"bereit\\\"\""
        )));
        crate::preview::build(document.source(), std::path::Path::new("condition.laml")).unwrap();
        assert!(document.undo());
        assert_eq!(document.source(), source);
        assert!(!document.undo());
        assert!(picked.edits(&source, "unsupported\nline").is_err());
    }
}
#[test]
fn sequence_branch_conditions_and_nested_identical_guards_use_exact_source_lines() {
    let source = "settings {\r\n type: sequence\r\n}\r\nsemantics {\r\n A\r\n B\r\n outer: alt \"same\" { // outer\r\n inner: opt \"same\" { // inner\r\n A ->> B\r\n }\r\n }\r\n else \"same\" { // branch\r\n B ->> A\r\n }\r\n}\r\n";
    let preview = crate::preview::build(source, std::path::Path::new("conditions.laml")).unwrap();
    let labels = preview
        .texts
        .iter()
        .filter(|t| t.text == "[same]")
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 3);
    let mut lines = vec![];
    for label in labels {
        let picked = hit(
            &preview,
            source,
            label.x + label.width / 2.0,
            label.y + label.height / 2.0,
        )
        .unwrap();
        lines.push(picked.line);
        let document = replace(source, &picked, "changed");
        assert_eq!(document.source().matches("\"changed\"").count(), 1);
        assert_eq!(document.source().matches("\"same\"").count(), 2);
        assert!(document.source().contains("// outer\r\n"));
        assert!(document.source().contains("// inner\r\n"));
        assert!(document.source().contains("// branch\r\n"));
        assert!(!document.source().replace("\r\n", "").contains('\n'));
        crate::preview::build(document.source(), std::path::Path::new("conditions.laml")).unwrap();
    }
    lines.sort();
    assert_eq!(lines, [7, 8, 12]);
}
#[test]
fn loop_guard_editing_preserves_bounds_and_empty_headers_can_add_conditions() {
    for header in [
        "opt {",
        "loop(min=1; max=3) {",
        "loop(min=\"low\"; max=\"high\"; guard=\"ready\") {",
    ] {
        let source = format!(
            "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n {header} // keep\n A ->> B\n }}\n}}\n"
        );
        let label = if header.contains("guard=") {
            "[ready]"
        } else if header.starts_with("opt") {
            "opt"
        } else {
            "loop(1,3)"
        };
        let (_, picked) = pick(&source, label);
        let document = replace(&source, &picked, "condition");
        assert!(document.source().contains("// keep"));
        if header.contains("min=") {
            assert!(document.source().contains(if header.contains("low") {
                "min=\"low\"; max=\"high\""
            } else {
                "min=1; max=3"
            }));
            assert!(document.source().contains("guard=\"condition\""));
        }
        crate::preview::build(
            document.source(),
            std::path::Path::new("loop-condition.laml"),
        )
        .unwrap();
    }
}
#[test]
fn decomposed_fragment_text_keeps_qualified_source_addressing() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inside\" {\n A\n B\n opt \"ready\" {\n A ->> B\n }\n }\n}\n";
    let (_, picked) = pick(source, "[ready]");
    assert_eq!(picked.line, 9);
    let document = replace(source, &picked, "changed");
    assert!(document.source().contains("opt \"changed\""));
    crate::preview::build(document.source(), std::path::Path::new("decomposed.laml")).unwrap();
}
