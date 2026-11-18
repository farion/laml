use super::*;
use laml::sequence::{Event, occurrence_layout::Endpoint};
use sequence_controls::{Boundary, Dock};
fn source() -> String {
    "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n work: B+ : \"Keep label\" // keep start\n request: A -> B: request // keep request\n B- // keep end\n}\n".into()
}
fn line(s: &str, needle: &str) -> usize {
    s.lines().position(|l| l.contains(needle)).unwrap() + 1
}
fn dock(s: &str, lower: bool) -> Dock {
    Dock {
        owner: "B".into(),
        before: if lower {
            line(s, "B- //")
        } else {
            line(s, "work: B+")
        },
        boundary: Some(if lower {
            Boundary::End {
                start: line(s, "work: B+"),
                end: Some(line(s, "B- //")),
            }
        } else {
            Boundary::Start(line(s, "work: B+"))
        }),
    }
}
fn render(s: &str) {
    let p = crate::preview::build(s, std::path::Path::new("docking.laml"));
    assert!(p.is_ok(), "{}\n{s}", p.unwrap_err());
}
#[test]
fn named_labelled_and_occupied_docks_preserve_source_and_undo() {
    for lower in [false, true] {
        let s = source().replace('\n', "\r\n");
        let target = dock(&s, lower);
        let from = Dock {
            owner: "C".into(),
            before: target.before,
            boundary: None,
        };
        let (edits, selected) =
            sequence_controls::message_with_line(&s, &from, &target, "->>").unwrap();
        let result = apply(&s, &edits).unwrap();
        assert!(result.contains("work: B+ : \"Keep label\" // keep start\r\n"));
        assert!(result.contains("request: A -> B: request // keep request\r\n"));
        assert!(result.contains("B- // keep end\r\n"));
        assert!(!result.replace("\r\n", "").contains('\n'));
        let d = laml::sequence::parse(&result).unwrap();
        assert_eq!(d.occurrence_layout.len(), 2);
        assert!(d.events.iter().any(
            |e| matches!(e,Event::Message(m) if m.line==selected && m.from=="C" && m.to=="B")
        ));
        render(&result);
        let mut document = crate::document::Document::new(s.clone());
        document.apply(0, "Dock message", edits).unwrap();
        assert!(document.undo());
        assert_eq!(document.source(), s);
    }
}
#[test]
fn reconnect_replaces_only_the_selected_endpoint_constraint_and_can_detach_to_gap() {
    let s = source();
    let target = dock(&s, false);
    let (edits, selected) =
        sequence_controls::reconnect(&s, line(&s, "request:"), false, &target, false).unwrap();
    let top = apply(&s, &edits).unwrap();
    render(&top);
    assert!(
        sequence_controls::reconnect(&top, selected, false, &dock(&top, false), false)
            .unwrap()
            .0
            .is_empty()
    );
    let (edits, selected) =
        sequence_controls::reconnect(&top, selected, false, &dock(&top, true), false).unwrap();
    let bottom = apply(&top, &edits).unwrap();
    render(&bottom);
    let d = laml::sequence::parse(&bottom).unwrap();
    assert_eq!(d.occurrence_layout.len(), 1);
    assert!(matches!(
        d.occurrence_layout[0].second,
        Endpoint::Activation { lower: true, .. }
    ));
    let gap = Dock {
        owner: "B".into(),
        before: 0,
        boundary: None,
    };
    let (edits, _) = sequence_controls::reconnect(&bottom, selected, false, &gap, false).unwrap();
    let free = apply(&bottom, &edits).unwrap();
    render(&free);
    assert!(
        laml::sequence::parse(&free)
            .unwrap()
            .occurrence_layout
            .is_empty()
    );
}
#[test]
fn resizing_or_deleting_a_bar_releases_its_docks_without_removing_messages() {
    let s = source();
    let target = dock(&s, false);
    let (edits, _) =
        sequence_controls::reconnect(&s, line(&s, "request:"), false, &target, false).unwrap();
    let s = apply(&s, &edits).unwrap();
    let (edits, _) = sequence_align::resize(
        &s,
        "B",
        line(&s, "work: B+"),
        line(&s, "work: B+"),
        false,
        false,
        line(&s, "B- //"),
    )
    .unwrap();
    let resized = apply(&s, &edits).unwrap();
    assert!(
        laml::sequence::parse(&resized)
            .unwrap()
            .occurrence_layout
            .is_empty()
    );
    assert!(resized.contains("request: A -> B"));
    let deleted = apply(
        &s,
        &sequence_lifetime::activation(&s, "B", line(&s, "work: B+"))
            .unwrap()
            .edits,
    )
    .unwrap();
    render(&deleted);
    assert!(deleted.contains("request: A -> B"));
    assert!(
        laml::sequence::parse(&deleted)
            .unwrap()
            .occurrence_layout
            .is_empty()
    );
}
#[test]
fn scoped_reconnection_preserves_existing_rules_comments_and_qualification() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n Root\n Other\n decompose Root \"Inside\" {\n  A\n  B\n  work: B+\n  request: A -> B\n  B-\n }\n}\nlayout {\n Root < Other : gap=99 // unrelated\n Root.request.receive ? Root.B.work+ : align=center-y; reference=second // keep dock comment\n}\n";
    let dock = Dock {
        owner: "Root.B".into(),
        before: line(s, "  B-"),
        boundary: Some(Boundary::End {
            start: line(s, "work: B+"),
            end: Some(line(s, "  B-")),
        }),
    };
    let (edits, _) =
        sequence_controls::reconnect(s, line(s, "request:"), false, &dock, false).unwrap();
    let result = apply(s, &edits).unwrap();
    assert!(result.contains("Root < Other : gap=99 // unrelated"));
    assert!(result.contains("// keep dock comment"));
    assert!(!result.contains("Root.Root"));
    let d = laml::sequence::parse(&result).unwrap();
    assert_eq!(d.decompositions[0].diagram.occurrence_layout.len(), 1);
    render(&result);
}
#[test]
fn renaming_an_activation_updates_mixed_references() {
    let s = source();
    let target = dock(&s, false);
    let (edits, _) =
        sequence_controls::reconnect(&s, line(&s, "request:"), false, &target, false).unwrap();
    let s = apply(&s, &edits).unwrap();
    let field = fields::of(&s, line(&s, "work: B+"))
        .into_iter()
        .find(|f| f.name == "Name / reference")
        .unwrap();
    let result = apply(&s, &fields::edit(&s, &field, "renamed").unwrap()).unwrap();
    assert!(result.contains("B.renamed+"));
    render(&result);
}
#[test]
fn renaming_or_deleting_a_bound_call_updates_its_activation_docks() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n call: A -> B+\n extra: C ->> B\n B-\n}\nlayout {\n extra.receive ? B.call+ : align=center-y; reference=second\n}\n";
    let renamed = apply(s, &rename(s, "call", line(s, "call:"), "renamed").unwrap()).unwrap();
    assert!(renamed.contains("B.renamed+"));
    render(&renamed);
    let deleted = apply(
        s,
        &delete(s, line(s, "call:"), Some("call"), false)
            .unwrap()
            .edits,
    )
    .unwrap();
    render(&deleted);
    assert!(deleted.contains("extra: C ->> B"));
    assert!(
        laml::sequence::parse(&deleted)
            .unwrap()
            .occurrence_layout
            .is_empty()
    );
}
#[test]
fn moving_a_message_row_releases_only_layout_docks_and_preserves_comments() {
    let s = source();
    let (edits, _) =
        sequence_controls::reconnect(&s, line(&s, "request:"), false, &dock(&s, false), false)
            .unwrap();
    let s = apply(&s, &edits).unwrap();
    let (edits, _) = sequence_scoped::move_event(&s, line(&s, "request:"), 0, None).unwrap();
    let result = apply(&s, &edits).unwrap();
    render(&result);
    assert!(result.contains("// keep request"));
    assert!(
        laml::sequence::parse(&result)
            .unwrap()
            .occurrence_layout
            .is_empty()
    );
}
