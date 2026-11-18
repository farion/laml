use super::*;
const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+: first // right start\n A -> B: request\n A+ // left start\n A -> B: later\n A-\n A <-- B-: response\n}\n";

#[test]
fn screenshot_left_start_can_join_right_start_on_the_first_arrow() {
    for source in [SOURCE.to_string(), SOURCE.replace('\n', "\r\n")] {
        let (edits, selected) =
            super::super::sequence_align::snap_to_occurrence(&source, "A", 9, 9, false, 7, false)
                .unwrap();
        let candidate = apply(&source, &edits).unwrap();
        assert!(candidate.contains("A+ -> B+: first // right start"));
        assert!(candidate.contains("// left start"));
        let doc = laml::sequence::parse(&candidate).unwrap();
        let pairs = executions(&doc);
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs.iter().find(|e| e.owner == "A").unwrap().start,
            selected
        );
        assert_eq!(
            pairs.iter().find(|e| e.owner == "B").unwrap().start,
            selected
        );
        let preview = crate::preview::build(&candidate, std::path::Path::new("dual.laml")).unwrap();
        let bars = preview
            .regions
            .iter()
            .filter(|r| r.kind == "sequence-execution")
            .collect::<Vec<_>>();
        assert!((bars[0].y - bars[1].y).abs() < 0.01);
        let mut document = crate::document::Document::new(source.clone());
        document.apply(0, "Resize activation", edits).unwrap();
        assert!(document.undo());
        assert_eq!(document.source(), source);
        assert!(!document.undo());
    }
}

#[test]
fn resizing_either_edge_away_keeps_the_other_endpoint_marker() {
    let (edits, _) =
        super::super::sequence_align::snap_to_occurrence(SOURCE, "A", 9, 9, false, 7, false)
            .unwrap();
    let dual = apply(SOURCE, &edits).unwrap();
    for (owner, expected) in [("A", "A -> B+: first"), ("B", "A+ -> B: first")] {
        let (edits, _) =
            super::super::sequence_align::resize(&dual, owner, 7, 7, false, false, 8).unwrap();
        let candidate = apply(&dual, &edits).unwrap();
        assert!(candidate.contains(expected), "{candidate}");
        assert_eq!(
            executions(&laml::sequence::parse(&candidate).unwrap()).len(),
            2
        );
    }
}

#[test]
fn lower_edge_can_join_an_existing_end_at_the_other_endpoint() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A+ -> B+\n A -> B-: close right\n A-\n}\n";
    let (edits, _) =
        super::super::sequence_align::snap_to_occurrence(source, "A", 7, 9, true, 8, false)
            .unwrap();
    let candidate = apply(source, &edits).unwrap();
    assert!(candidate.contains("A- -> B-: close right"));
    let pairs = executions(&laml::sequence::parse(&candidate).unwrap());
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].end, pairs[1].end);
}
