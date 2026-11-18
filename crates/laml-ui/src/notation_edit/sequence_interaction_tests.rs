//! Contract cases spanning the new source actions and retained compiler rules.
use super::*;
use sequence_controls::{Boundary, Dock};
fn source(events: &str) -> String {
    format!(
        "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B(label=\"Worker\")\n{events}\n}}\n"
    )
}
#[test]
fn changing_to_creation_copies_participant_properties_and_keeps_comments() {
    let s = source(" A -> B: make // keep").replace('\n', "\r\n");
    let edits = sequence_lifetime::message_kind(&s, 7, true).unwrap();
    let result = apply(&s, &edits).unwrap();
    let d = laml::sequence::parse(&result).unwrap();
    let b = d.participants.iter().find(|p| p.id == "B").unwrap();
    assert_eq!(b.label, "Worker");
    assert!(b.created_line.is_some());
    assert!(result.contains("// keep\r\n"));
    assert!(!result.replace("\r\n", "").contains('\n'));
}
#[test]
fn creation_is_not_offered_when_target_already_has_an_earlier_message() {
    let s = source(" A -> B: first\n A -> B: later");
    assert!(sequence_lifetime::message_kind(&s, 8, true).is_err());
}
#[test]
fn deletion_is_not_offered_when_target_has_later_events() {
    let s = source(" A -> B: first\n A -> B: later");
    assert!(sequence_lifetime::message_kind(&s, 7, false).is_err());
}
#[test]
fn starting_a_self_message_does_not_rebind_its_sender_activation() {
    let s = source(" B+\n B-");
    let a = Dock {
        owner: "B".into(),
        before: 7,
        boundary: Some(Boundary::Start(7)),
    };
    let b = Dock {
        owner: "B".into(),
        before: 8,
        boundary: None,
    };
    let edits = sequence_controls::message(&s, &a, &b, "->").unwrap();
    let result = apply(&s, &edits).unwrap();
    let d = laml::sequence::parse(&result).unwrap();
    assert_eq!(laml::sequence::activation_layout::executions(&d).len(), 1);
    assert!(result.contains(" B+\n"));
    assert!(result.contains("B -> B"));
    assert!(!result.contains("B+ -> B"));
}
#[test]
fn qualified_lifelines_support_message_creation_and_activation_insertion() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inner\" {\n  A\n  B\n }\n}\n";
    let from = Dock {
        owner: "Root.A".into(),
        before: 0,
        boundary: None,
    };
    let to = Dock {
        owner: "Root.B".into(),
        before: 0,
        boundary: None,
    };
    let edits = sequence_controls::message(s, &from, &to, "->").unwrap();
    let result = apply(s, &edits).unwrap();
    let d = laml::sequence::parse(&result).unwrap();
    assert_eq!(
        d.decompositions[0]
            .diagram
            .events
            .iter()
            .filter(|e| matches!(e, laml::sequence::Event::Message(_)))
            .count(),
        1
    );
    let edits = sequence_scoped::insert(s, &to, sequence_insert::Action::Activate, "", "").unwrap();
    let result = apply(s, &edits).unwrap();
    let d = laml::sequence::parse(&result).unwrap();
    assert_eq!(
        laml::sequence::activation_layout::executions(&d.decompositions[0].diagram).len(),
        1
    );
}
#[test]
fn activation_deletion_removes_its_layout_references_but_keeps_messages() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n first: A+\n second: B+\n A ->> B\n B-\n A-\n}\nlayout {\n A.first+ ? B.second+ : align=center-y // keep comment\n}\n";
    let result = apply(s, &sequence_lifetime::activation(s, "A", 7).unwrap().edits).unwrap();
    assert!(result.contains("A ->> B"));
    assert!(laml::sequence::parse(&result).is_ok());
}
#[test]
fn destroyed_lifeline_cannot_receive_new_messages() {
    let s = source(" B!");
    let from = Dock {
        owner: "A".into(),
        before: 0,
        boundary: None,
    };
    let to = Dock {
        owner: "B".into(),
        before: 0,
        boundary: None,
    };
    assert!(sequence_controls::message(&s, &from, &to, "->").is_err());
}
#[test]
fn new_message_can_end_a_standalone_or_open_activation() {
    for open in [false, true] {
        let s = source(if open { " B+" } else { " B+\n B-" });
        let from = Dock {
            owner: "A".into(),
            before: if open { 0 } else { 8 },
            boundary: None,
        };
        let to = Dock {
            owner: "B".into(),
            before: if open { 0 } else { 8 },
            boundary: Some(Boundary::End {
                start: 7,
                end: if open { None } else { Some(8) },
            }),
        };
        let result = apply(
            &s,
            &sequence_controls::message(&s, &from, &to, "->").unwrap(),
        )
        .unwrap();
        assert!(result.contains(" B-\n"));
        assert!(result.contains(".receive ? B- : align=center-y; reference=second"));
        let doc = laml::sequence::parse(&result).unwrap();
        assert!(
            laml::sequence::activation_layout::executions(&doc)[0]
                .end
                .is_some()
        );
    }
}
#[test]
fn inserting_a_message_cannot_steal_an_existing_message_boundary() {
    let s = source(" A -> B+\n B-");
    let from = Dock {
        owner: "A".into(),
        before: 7,
        boundary: None,
    };
    let to = Dock {
        owner: "B".into(),
        before: 7,
        boundary: Some(Boundary::Start(7)),
    };
    let result = apply(
        &s,
        &sequence_controls::message(&s, &from, &to, "->").unwrap(),
    )
    .unwrap();
    assert!(result.contains(" A -> B+\n"));
    assert!(result.contains(".receive ? B+ : align=center-y; reference=second"));
    let doc = laml::sequence::parse(&result).unwrap();
    assert_eq!(laml::sequence::activation_layout::executions(&doc).len(), 1);
}
