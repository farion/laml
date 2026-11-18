use laml::{authoring::Region, sequence, styles::StyleSheet, svg::TextMode};
fn source(events: &str, layout: &str) -> String {
    format!(
        "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n C\n{events}\n}}\nlayout {{\n{layout}\n}}\n"
    )
}
fn render(s: &str) -> Result<Vec<Region>, String> {
    let d = sequence::parse(s).map_err(|e| e.message)?;
    sequence::render::render_for_editor(&d, &StyleSheet::default(), TextMode::System)
        .map(|(_, r)| r)
}
fn occurrence(regions: &[Region], s: &str, name: &str, receive: bool) -> f64 {
    let d = sequence::parse(s).unwrap();
    let line = d
        .events
        .iter()
        .find_map(|e| match e {
            sequence::Event::Message(m) if m.name.as_deref() == Some(name) => Some(m.line),
            _ => None,
        })
        .unwrap();
    regions
        .iter()
        .find(|r| {
            r.line == line
                && r.kind
                    == if receive {
                        "sequence-receive"
                    } else {
                        "sequence-send"
                    }
        })
        .unwrap()
        .y
}
fn bar(regions: &[Region], owner: &str) -> (f64, f64) {
    let b = regions
        .iter()
        .find(|r| r.kind == "sequence-execution" && r.id == owner)
        .unwrap();
    (b.y, b.y + b.height)
}
#[test]
fn named_labelled_activation_stays_stationary_at_both_message_docks() {
    let events = " work: B+ : \"Keep this label\"\n request: A -> B\n B-";
    let baseline = bar(&render(&source(events, "")).unwrap(), "B");
    for lower in [false, true] {
        let s = source(
            events,
            &format!(
                " request.receive ? B.work{} : align=center-y; reference=second",
                if lower { '-' } else { '+' }
            ),
        );
        let r = render(&s).unwrap();
        assert_eq!(bar(&r, "B"), baseline);
        assert_eq!(
            occurrence(&r, &s, "request", true),
            if lower { baseline.1 } else { baseline.0 }
        );
        assert!(occurrence(&r, &s, "request", false) <= occurrence(&r, &s, "request", true));
        let d = sequence::parse(&s).unwrap();
        assert!(d.events.iter().any(|e|matches!(e,sequence::Event::Activate {id:Some(id),label:Some(label),..} if id=="work" && label=="Keep this label")));
    }
}
#[test]
fn docking_to_an_occupied_boundary_keeps_the_original_call_in_place() {
    let events = " call: A -> B+\n extra: C -> B\n B-";
    let baseline = source(events, "");
    let base = render(&baseline).unwrap();
    let s = source(
        events,
        " extra.receive ? B.call+ : align=center-y; reference=second",
    );
    let r = render(&s).unwrap();
    assert_eq!(bar(&r, "B"), bar(&base, "B"));
    assert_eq!(
        occurrence(&r, &s, "call", true),
        occurrence(&base, &baseline, "call", true)
    );
    assert_eq!(occurrence(&r, &s, "extra", true), bar(&r, "B").0);
}
#[test]
fn independent_sender_and_receiver_constraints_make_a_bent_message() {
    let events = " first: A+\n second: B+\n request: A -> B\n B-\n A-";
    let s = source(
        events,
        " request.send ? A.first+ : align=center-y; reference=second\n request.receive ? B.second- : align=center-y; reference=second",
    );
    let r = render(&s).unwrap();
    assert_eq!(occurrence(&r, &s, "request", false), bar(&r, "A").0);
    assert_eq!(occurrence(&r, &s, "request", true), bar(&r, "B").1);
    assert!(occurrence(&r, &s, "request", true) > occurrence(&r, &s, "request", false));
    assert!(r.iter().any(|r| r.kind == "message" && r.path.len() > 2));
}
#[test]
fn self_message_has_distinct_send_and_receive_binding_heights() {
    let s = source(
        " work: B+\n self: B -> B\n B-",
        " self.send ? B.work+ : align=center-y; reference=second\n self.receive ? B.work- : align=center-y; reference=second",
    );
    let r = render(&s).unwrap();
    assert_eq!(occurrence(&r, &s, "self", false), bar(&r, "B").0);
    assert_eq!(occurrence(&r, &s, "self", true), bar(&r, "B").1);
}
#[test]
fn conflicting_references_and_cycles_fail_instead_of_overriding_rules() {
    let s = source(
        " first: B+\n second: B+\n request: A -> B\n B-\n B-",
        " request.receive ? B.first+ : align=center-y; reference=second\n request.receive ? B.second+ : align=center-y; reference=second",
    );
    assert!(render(&s).unwrap_err().contains("Conflicting"));
    let s = source(
        " first: A -> B+\n second: C -> B+\n B-\n B-",
        " first.receive ? B.second+ : align=center-y; reference=second\n second.receive ? B.first+ : align=center-y; reference=second",
    );
    assert!(render(&s).unwrap_err().contains("Cyclic"));
}
#[test]
fn unknown_or_open_references_and_cross_operand_rules_fail_early() {
    for layout in [
        " missing.receive ? B.work+ : align=center-y",
        " request.bad ? B.work+ : align=center-y",
        " request.receive ? B.work- : align=center-y",
    ] {
        assert!(sequence::parse(&source(" work: B+\n request: A -> B", layout)).is_err());
    }
    let s = source(
        " alt \"one\" {\n work: B+\n B-\n }\n else \"two\" {\n request: A -> B\n }",
        " request.receive ? B.work+ : align=center-y",
    );
    assert!(sequence::parse(&s).unwrap_err().message.contains("operand"));
}
#[test]
fn mixed_rules_route_into_inline_decompositions() {
    let s = "settings {\n type: sequence\n}\nsemantics {\n Root\n decompose Root \"Inside\" {\n A\n B\n work: B+\n request: A -> B\n B-\n }\n}\nlayout {\n Root.request.receive ? Root.B.work+ : align=center-y; reference=second\n}\n";
    let r = render(s).unwrap();
    let b = r
        .iter()
        .find(|r| r.kind == "sequence-execution" && r.id == "Root.B")
        .unwrap();
    let end = r
        .iter()
        .find(|r| r.kind == "sequence-receive" && r.id == "Root.B")
        .unwrap();
    assert_eq!(end.y, b.y);
}
#[test]
fn reversed_spelling_and_default_anchor_keep_the_activation_fixed() {
    let events = " work: B+\n request: A -> B\n B-";
    let b = bar(&render(&source(events, "")).unwrap(), "B");
    let s = source(events, " B.work+ ? request.receive : align=center-y");
    let r = render(&s).unwrap();
    assert_eq!(bar(&r, "B"), b);
    assert_eq!(occurrence(&r, &s, "request", true), b.0);
}
#[test]
fn explicit_upward_messages_nested_escape_and_lifetime_violations_are_rejected() {
    let s = source(
        " first: A+\n second: B+\n request: A -> B\n B-\n A-",
        " request.send ? A.first- : align=center-y; reference=second\n request.receive ? B.second+ : align=center-y; reference=second",
    );
    assert!(render(&s).unwrap_err().contains("upward"));
    let s = source(
        " early: A ->> B\n outer: B+\n inner: B+\n B-\n B-",
        " B.inner+ ? early.receive : align=center-y; reference=second",
    );
    assert!(render(&s).unwrap_err().contains("outside its parent"));
    let s = source(
        " outer: A+\n work: B+\n request: C -> B\n B!\n A-",
        " request.receive ? A.outer- : align=center-y; reference=second",
    );
    assert!(
        render(&s)
            .unwrap_err()
            .contains("after participant destruction")
    );
}
