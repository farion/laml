use super::*;
const SOURCE: &str = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n A -> B+\n B+\n B -> B: self\n B-\n A <-- B-: response\n}\n";

fn snap(
    source: &str,
    start: usize,
    edge: usize,
    lower: bool,
    message: usize,
    receive: bool,
) -> (String, usize) {
    let (edits, selected) = super::super::sequence_align::snap_to_occurrence(
        source, "B", start, edge, lower, message, receive,
    )
    .unwrap();
    let candidate = apply(source, &edits).unwrap();
    let preview =
        crate::preview::build(&candidate, std::path::Path::new("self-snap.laml")).unwrap();
    let bar = preview
        .regions
        .iter()
        .find(|r| r.kind == "execution" && r.line == selected)
        .unwrap();
    let doc = laml::sequence::parse(&candidate).unwrap();
    let self_line = doc
        .events
        .iter()
        .find_map(|e| match e {
            Event::Message(m) if m.label.as_deref() == Some("self") => Some(m.line),
            _ => None,
        })
        .unwrap();
    let kind = if receive {
        "sequence-receive"
    } else {
        "sequence-send"
    };
    let endpoint = preview
        .regions
        .iter()
        .find(|r| r.kind == kind && r.line == self_line)
        .unwrap();
    let y = if lower { bar.y + bar.height } else { bar.y };
    assert!(
        (y - endpoint.y).abs() < 0.01,
        "edge {y}, {} {}",
        kind,
        endpoint.y
    );
    assert_eq!(executions(&doc).len(), 2);
    assert!(candidate.contains(": response"));
    let mut document = crate::document::Document::new(source.into());
    document
        .apply(0, "Resize nested activation", edits)
        .unwrap();
    assert!(document.undo());
    assert_eq!(document.source(), source);
    (candidate, selected)
}

#[test]
fn lone_open_activation_can_snap_to_send_then_receive_and_back() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n B\n B+\n B -> B\n}\n";
    let mut candidate = source.to_owned();
    let mut start = 6;
    let mut message = 7;
    for receive in [false, true, false] {
        let (edits, selected) = super::super::sequence_align::snap_to_occurrence(
            &candidate, "B", start, start, false, message, receive,
        )
        .unwrap();
        candidate = apply(&candidate, &edits).unwrap();
        start = selected;
        message = selected;
        let preview = crate::preview::build(&candidate, std::path::Path::new("lone.laml")).unwrap();
        let bar = preview
            .regions
            .iter()
            .find(|r| r.kind == "execution")
            .unwrap();
        let endpoint = preview
            .regions
            .iter()
            .find(|r| {
                r.kind
                    == if receive {
                        "sequence-receive"
                    } else {
                        "sequence-send"
                    }
            })
            .unwrap();
        assert!((bar.y - endpoint.y).abs() < 0.01, "{candidate}");
    }
}

#[test]
fn nested_upper_edge_can_snap_to_either_self_call_occurrence() {
    for receive in [false, true] {
        snap(SOURCE, 8, 8, false, 9, receive);
    }
}
#[test]
fn nested_lower_edge_can_snap_to_either_self_call_occurrence() {
    for receive in [false, true] {
        snap(SOURCE, 8, 10, true, 9, receive);
    }
}
#[test]
fn already_bound_self_call_can_switch_which_occurrence_anchors_its_start() {
    let (candidate, start) = snap(SOURCE, 8, 8, false, 9, true);
    snap(&candidate, start, start, false, start, false);
}
