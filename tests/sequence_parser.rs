use laml::sequence::{Event, MessageKind, is_sequence_source, parse};

#[test]
fn sequence_type_is_explicit_and_does_not_match_comments_or_labels() {
    assert!(!is_sequence_source(
        "semantics {\nA(label=\"type: sequence\")\n}\n"
    ));
    assert!(!is_sequence_source("settings {\n// type: sequence\n}\n"));
    assert!(is_sequence_source(
        "settings {\ntype: sequence\n}\nsemantics {\nA\n}\n"
    ));
}

#[test]
fn parses_lifelines_messages_activations_and_short_reply() {
    let diagram = parse("settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI(label=\"Orders API\")\nrequest: Client -> API+ : GET /orders\n<-- : 200 OK\nClient ->> API : event\n}\n").unwrap();
    assert_eq!(diagram.participants[1].label, "Orders API");
    assert_eq!(diagram.events.len(), 5);
    assert!(
        matches!(&diagram.events[0], Event::Message(m) if m.name.as_deref() == Some("request") && m.kind == MessageKind::Sync && m.to == "API")
    );
    assert!(
        matches!(&diagram.events[1], Event::Activate { participant, .. } if participant == "API")
    );
    assert!(
        matches!(&diagram.events[2], Event::Message(m) if m.kind == MessageKind::Reply && m.from == "API" && m.to == "Client")
    );
    assert!(
        matches!(&diagram.events[3], Event::Deactivate { participant, .. } if participant == "API")
    );
    assert!(matches!(&diagram.events[4], Event::Message(m) if m.kind == MessageKind::Async));
}

#[test]
fn named_and_unnamed_messages_accept_existing_relation_class_syntax() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest(style-classes=protocol): A -> B : call\n(style-classes=protocol): A ->> B : event\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        matches!(&diagram.events[0], Event::Message(m) if m.name.as_deref() == Some("request") && m.classes == ["protocol"])
    );
    assert!(
        matches!(&diagram.events[1], Event::Message(m) if m.name.is_none() && m.classes == ["protocol"])
    );
}

#[test]
fn standalone_activation_and_reverse_call_are_preserved() {
    let diagram = parse("settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nAPI+\nClient <- API : request\nAPI-\n}\n").unwrap();
    assert!(
        matches!(&diagram.events[0], Event::Activate { participant, .. } if participant == "API")
    );
    assert!(matches!(&diagram.events[1], Event::Message(m) if m.from == "API" && m.to == "Client"));
    assert!(
        matches!(&diagram.events[2], Event::Deactivate { participant, .. } if participant == "API")
    );
}

#[test]
fn missing_reply_is_valid_but_extra_minus_and_unbound_reply_are_errors() {
    let prefix = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\n";
    assert!(parse(&format!("{prefix}Client -> API : fire\n}}\n")).is_ok());
    let error = parse(&format!("{prefix}API-\n}}\n")).unwrap_err();
    assert_eq!(error.line, 7);
    assert!(error.message.contains("unmatched"));
    let error = parse(&format!("{prefix}<-- : reply\n}}\n")).unwrap_err();
    assert_eq!(error.line, 7);
    assert!(error.message.contains("no open call"));
}

#[test]
fn visible_time_and_state_require_an_earlier_named_message_endpoint() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nrequest: Client -> API : GET\nat[request.receive] : time-constraint=\"t <= 200 ms\"\nat[request.send] : state=\"Ready\"\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram
            .events
            .iter()
            .filter(|event| matches!(event, Event::Annotation(_)))
            .count(),
        2
    );
    let unknown = source.replace("request.receive", "missing.receive");
    assert!(
        parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown earlier named message")
    );
    let invalid = source.replace("request.receive", "request.middle");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("send or receive")
    );
}

#[test]
fn creation_sort_and_reply_arrow_style_are_explicit_visible_properties() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nspawn(kind=create): A -> B+ : create\nreply(arrow=filled): A <-- B : done\n}\n";
    let diagram = parse(source).unwrap();
    assert!(matches!(&diagram.events[0], Event::Message(m) if m.kind == MessageKind::Create));
    assert!(
        matches!(&diagram.events[2], Event::Message(m) if m.kind == MessageKind::Reply && m.filled_reply)
    );
    let bad = source.replace("spawn(kind=create)", "spawn(kind=call)");
    assert!(parse(&bad).is_err());
    let bad = source.replace("A -> B+", "A -> B");
    assert!(parse(&bad).is_err());
    let bad = source.replace(
        "reply(arrow=filled): A <-- B",
        "reply(arrow=filled): A -> B",
    );
    assert!(parse(&bad).is_err());
    let shorthand = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nA -> B+\nr(arrow=filled): <-- : done\n}\n";
    assert!(
        parse(shorthand)
            .unwrap()
            .events
            .iter()
            .any(|e| matches!(e, Event::Message(m) if m.filled_reply))
    );
}

#[test]
fn invariant_annotation_is_a_visible_constraint_at_a_named_endpoint() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B\nat[request.receive] : invariant=\"ready && active\"\n}\n";
    let diagram = parse(source).unwrap();
    assert!(diagram.events.iter().any(|event| matches!(event,
        Event::Annotation(a) if a.kind == laml::sequence::AnnotationKind::Invariant)));
    assert!(parse(&source.replace("request.receive", "unknown.receive")).is_err());
}

#[test]
fn general_ordering_connects_named_ends_and_rejects_cycles() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nfirst: A -> B : one\nsecond: B -> A : two\norder[first.receive,second.send]\n}\n";
    assert!(
        parse(source)
            .unwrap()
            .events
            .iter()
            .any(|event| matches!(event, Event::Ordering(_)))
    );
    let cycle = source.replace(
        "order[first.receive,second.send]",
        "order[first.receive,second.send]\norder[second.send,first.receive]",
    );
    assert!(
        parse(&cycle)
            .unwrap_err()
            .message
            .contains("ordering cycle")
    );
    let reverse_message = source.replace(
        "order[first.receive,second.send]",
        "order[first.receive,first.send]",
    );
    assert!(
        parse(&reverse_message)
            .unwrap_err()
            .message
            .contains("ordering cycle")
    );
}

#[test]
fn time_and_duration_forms_require_the_right_number_of_anchors() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B : request\nreply: A <-- B : reply\nat[request.receive] : time-observation=\"t1\"\nbetween[request.send,reply.receive] : duration-observation=\"elapsed\"\nbetween[request.receive,reply.send] : duration-constraint=\"<= 50 ms\"\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram
            .events
            .iter()
            .filter(|event| matches!(event, Event::Annotation(_)))
            .count(),
        3
    );
    assert!(
        parse(&source.replace("duration-observation", "time-observation"))
            .unwrap_err()
            .message
            .contains("does not match")
    );
    let unknown = source.replace("reply.receive", "missing.receive");
    assert!(
        parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown earlier")
    );
}

#[test]
fn named_gates_require_an_enclosing_frame() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nref \"Orders\" {\nAPI -> gate[auftrag] : request\ngate[ergebnis] -> API : result\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert!(matches!(&diagram.events[1], Event::Message(m) if m.to == "gate[auftrag]"));
    assert!(matches!(&diagram.events[2], Event::Message(m) if m.from == "gate[ergebnis]"));
    let outside = source
        .replace("ref \"Orders\" {\n", "")
        .replace("}\n}\n", "}\n");
    assert!(
        parse(&outside)
            .unwrap_err()
            .message
            .contains("enclosing frame")
    );
    let invalid = source.replace("gate[auftrag]", "gate[bad-name]");
    assert!(parse(&invalid).is_err());
}

#[test]
fn question_mark_marks_only_unknown_message_ends() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nAPI\n? -> API : found\nAPI -> ? : lost\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.participants.len(), 1);
    assert!(matches!(&diagram.events[0], Event::Message(m) if m.from == "?" && m.to == "API"));
    assert!(matches!(&diagram.events[1], Event::Message(m) if m.from == "API" && m.to == "?"));
    let invalid = source.replace("? -> API", "? -> ?");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("at least one known")
    );
    let invalid = source.replace("API -> ?", "API -> ?+");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("activation marker")
    );
}

#[test]
fn standalone_labeled_plus_creates_header_and_execution_only_with_plus() {
    let with_plus = "settings {\ntype: sequence\n}\nsemantics {\nAPI(label=\"Orders API\")+\n}\n";
    let diagram = parse(with_plus).unwrap();
    assert_eq!(diagram.participants[0].label, "Orders API");
    assert_eq!(diagram.participants[0].created_line, Some(5));
    assert!(
        matches!(&diagram.events[0], Event::Activate { participant, .. } if participant == "API")
    );
    let without_plus =
        parse(&with_plus.replace("API(label=\"Orders API\")+", "API(label=\"Orders API\")"))
            .unwrap();
    assert_eq!(without_plus.participants[0].created_line, None);
    assert!(without_plus.events.is_empty());
    let before_arrow = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI(label=\"Orders API\")\nClient -> API+ : start\n}\n";
    let diagram = parse(before_arrow).unwrap();
    assert_eq!(diagram.participants[1].label, "Orders API");
    assert!(
        matches!(&diagram.events[1], Event::Activate { participant, .. } if participant == "API")
    );
}

#[test]
fn first_plus_target_accepts_inline_display_label() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nClient -> API(label=\"Orders API\")+ : start\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.participants[1].id, "API");
    assert_eq!(diagram.participants[1].label, "Orders API");
    assert_eq!(diagram.participants[1].created_line, Some(6));
}

#[test]
fn exclamation_ends_lifeline_and_rejects_later_occurrences() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nClient\nClient -> API+ : start\nAPI!\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.participants[1].destroyed_line, Some(7));
    assert!(
        matches!(&diagram.events[2], Event::Destroy { participant, .. } if participant == "API")
    );
    let extra = source.replace("API!\n", "API!\nClient -> API : late\n");
    assert!(
        parse(&extra)
            .unwrap_err()
            .message
            .contains("already destroyed")
    );
    let duplicate = source.replace("API!\n", "API!\nAPI!\n");
    assert!(
        parse(&duplicate)
            .unwrap_err()
            .message
            .contains("already destroyed")
    );
    let unknown = source.replace("API!", "Missing!");
    assert!(
        parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown participant")
    );
}

#[test]
fn first_plus_opens_an_undeclared_lifeline_without_implicit_typos() {
    let arrow = "settings {\ntype: sequence\n}\nsemantics {\nClient\nClient -> API+ : start\n}\n";
    let diagram = parse(arrow).unwrap();
    assert_eq!(diagram.participants[1].created_line, Some(6));
    assert!(matches!(&diagram.events[0], Event::Message(m) if m.to == "API"));
    let standalone =
        "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI+\nClient -> API : hello\n}\n";
    assert_eq!(
        parse(standalone).unwrap().participants[1].created_line,
        Some(6)
    );
    let typo = arrow.replace("API+", "APi");
    assert!(
        parse(&typo)
            .unwrap_err()
            .message
            .contains("unknown participant")
    );
}

#[test]
fn short_reply_may_have_optional_name() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nClient -> API : request\nresult: <-- : done\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        matches!(&diagram.events[1], Event::Message(m) if m.name.as_deref() == Some("result") && m.from == "API" && m.to == "Client")
    );
}

#[test]
fn reverse_async_and_explicit_reply_leave_no_stale_short_reply_target() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nAPI <<- Client : notify\nClient -> API : request\nClient <-- API : answer\n<-- : extra\n}\n";
    let err = parse(source).unwrap_err();
    assert_eq!(err.line, 10);
    assert!(err.message.contains("no open call"));
    let valid = source.replace("<-- : extra\n", "");
    let diagram = parse(&valid).unwrap();
    assert!(
        matches!(&diagram.events[0], Event::Message(m) if m.kind == MessageKind::Async && m.from == "Client" && m.to == "API")
    );
}

#[test]
fn lane_rules_are_mandatory_and_stably_reorder_headers() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nC\nA\nB\n}\nlayout {\nB < A : gap=160\n}\n";
    let diagram = parse(source).unwrap();
    let ids: Vec<_> = diagram.participants.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["C", "B", "A"]);
    assert_eq!(diagram.layout[0].gap, 160.0);
    let reversed = source.replace("B < A", "A > B");
    assert_eq!(parse(&reversed).unwrap().participants, diagram.participants);
    let cycle = source.replace("B < A : gap=160", "B < A\nA < B");
    assert!(parse(&cycle).unwrap_err().message.contains("contradictory"));
    let unknown = source.replace("B < A", "Missing < A");
    assert!(
        parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown participant")
    );
}

#[test]
fn fragments_bind_short_replies_per_branch_and_reject_unbalanced_joins() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nA -> B+ : request\nalt \"ok\" {\n<-- : accepted\n}\nelse \"failed\" {\n<-- : rejected\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        diagram
            .events
            .iter()
            .any(|event| matches!(event, Event::FrameBranch { guard, .. } if guard == "failed"))
    );
    assert_eq!(
        diagram
            .events
            .iter()
            .filter(|event| matches!(event, Event::Message(m) if m.kind == MessageKind::Reply))
            .count(),
        2
    );
    let divergent = source
        .replace("A -> B+ : request", "A -> B : request")
        .replace("<-- : rejected", "B -> A : another call");
    assert!(parse(&divergent).is_ok());
    let ambiguous = divergent.replacen("}\n}\n", "}\n<-- : ambiguous\n}\n", 1);
    let error = parse(&ambiguous).unwrap_err();
    assert!(error.message.contains("no open call"));
}

#[test]
fn loop_bounds_are_structured_and_validated_without_evaluating_guard() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nloop(min=1; max=5; guard=\"retry if needed\") {\nA ->> B : attempt\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert!(
        matches!(&diagram.events[0], Event::FrameStart { operator, guard, .. } if operator == "loop(1,5)" && guard == "retry if needed")
    );
    let invalid = source.replace("max=5", "max=0");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("at least min")
    );
    let zero = source.replace("min=1; max=5", "min=0; max=0");
    assert!(parse(&zero).is_ok());
    let invalid = source.replace("min=1", "min=6");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("at least min")
    );
    let missing = source.replace("min=1; ", "");
    assert!(
        parse(&missing)
            .unwrap_err()
            .message
            .contains("requires min")
    );
    let symbolic = source.replace("min=1; max=5", "min=\"required\"; max=\"limit + 1\"");
    assert!(
        matches!(&parse(&symbolic).unwrap().events[0], Event::FrameStart { operator, .. } if operator == "loop(required,limit + 1)")
    );
}

#[test]
fn nested_frames_and_unclosed_blocks_are_detected() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nloop \"3 times\" {\nopt \"enabled\" {\nA ->> B : signal\n}\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram
            .events
            .iter()
            .filter(|event| matches!(event, Event::FrameStart { .. }))
            .count(),
        2
    );
    assert!(parse(&source.replace("}\n}\n}\n", "}\n}\n")).is_err());
}

#[test]
fn fragment_join_rejects_conditional_executions_without_inventing_an_end() {
    let divergent = "settings {\ntype: sequence\n}\nsemantics {\nA\nalt \"ok\" {\nA+\n}\nelse \"fail\" {\nA ->> A : retry\n}\nA-\n}\n";
    let err = parse(divergent).unwrap_err();
    assert!(err.message.contains("different open execution bars"));
    let balanced = divergent
        .replace("A+\n}", "A+\nA-\n}")
        .replace("}\nA-\n}\n", "}\n}\n");
    assert!(parse(&balanced).is_ok());

    let retained = "settings {\ntype: sequence\n}\nsemantics {\nA\nA+\nalt \"ok\" {\nA ->> A : one\n}\nelse \"fail\" {\nA ->> A : two\n}\nA-\n}\n";
    assert!(parse(retained).is_ok());
    let optional = "settings {\ntype: sequence\n}\nsemantics {\nA\nopt \"maybe\" {\nA+\n}\n}\n";
    assert!(
        parse(optional)
            .unwrap_err()
            .message
            .contains("different open execution bars")
    );
}
