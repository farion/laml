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
