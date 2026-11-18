use laml::{
    sequence::{self, Event},
    styles::StyleSheet,
    svg::TextMode,
};

fn source(events: &str) -> String {
    format!("settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n{events}\n}}\n")
}

#[test]
fn forward_and_reverse_messages_change_both_ends_independently() {
    for events in [
        " A+ -> B+: both\n A-\n B-",
        " B+ <- A+: both\n A-\n B-",
        " A+ ->> B+: both\n A-\n B-",
        " B+ <<- A+: both\n A-\n B-",
    ] {
        let doc = sequence::parse(&source(events)).unwrap();
        assert!(
            matches!(&doc.events[1], Event::Activate { participant, at_send: true, .. } if participant == "A")
        );
        assert!(
            matches!(&doc.events[2], Event::Activate { participant, at_send: false, .. } if participant == "B")
        );
        let (_, regions) =
            sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
                .unwrap();
        let bars = regions
            .iter()
            .filter(|r| r.kind == "sequence-execution")
            .collect::<Vec<_>>();
        assert_eq!(bars.len(), 2);
        assert!((bars[0].y - bars[1].y).abs() < 0.01);
    }
}

#[test]
fn both_endings_and_mixed_markers_keep_their_individual_pairs() {
    for events in [
        " A+ -> B+\n A- -> B-",
        " A+\n A- -> B+\n B-",
        " A+ -> B+\n A- <-- B-",
    ] {
        let doc = sequence::parse(&source(events)).unwrap();
        assert_eq!(
            doc.events
                .iter()
                .filter(|e| matches!(e, Event::Activate { .. }))
                .count(),
            2
        );
        assert_eq!(
            doc.events
                .iter()
                .filter(|e| matches!(e, Event::Deactivate { .. }))
                .count(),
            2
        );
        sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
            .unwrap();
    }
}

#[test]
fn receiver_creation_and_deletion_preserve_sender_changes() {
    let create = source(" A+ -> New+\n A-\n New-");
    let doc = sequence::parse(&create).unwrap();
    assert!(
        doc.participants
            .iter()
            .find(|p| p.id == "New")
            .unwrap()
            .created_line
            .is_some()
    );
    let doc = sequence::parse(&source(" A+ -> B+\n A- -> B!")).unwrap();
    assert!(
        doc.participants
            .iter()
            .find(|p| p.id == "B")
            .unwrap()
            .destroyed_line
            .is_some()
    );
    assert!(sequence::parse(&source(" A+ -> ?+")).is_err());
    assert!(sequence::parse(&source(" A! -> B+")).is_err());
}

#[test]
fn self_messages_retain_separate_send_receive_edge_heights() {
    let doc = sequence::parse(&source(" A+ -> A+\n A- -> A-")).unwrap();
    let (_, regions) =
        sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
            .unwrap();
    let bars = regions
        .iter()
        .filter(|r| r.kind == "sequence-execution")
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 2);
    assert!((bars[0].y - bars[1].y).abs() >= 24.0);
}
