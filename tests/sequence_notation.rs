use laml::{sequence, svg::TextMode};

#[test]
fn lifeline_head_displays_selector_type_and_self_as_visible_text() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nself\nAPI(selector=\"u\"; type=\"Service\")\nself -> API : ping\n}\n";
    let diagram = sequence::parse(source).unwrap();
    assert_eq!(diagram.participants[0].label, "self");
    assert_eq!(diagram.participants[1].label, "API[u]: Service");
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("API[u]: Service"));
    assert!(sequence::parse(&source.replace("selector=\"u\"", "selector=\"\"")).is_err());
    assert!(sequence::parse(&source.replace("self\n", "self(type=\"Widget\")\n")).is_err());
}

#[test]
fn deletion_message_ends_at_destruction_cross_without_arrowhead() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nClient -> API! : delete\n}\n";
    let diagram = sequence::parse(source).unwrap();
    assert!(diagram.events.iter().any(
        |e| matches!(e, sequence::Event::Message(m) if m.kind == sequence::MessageKind::Delete)
    ));
    assert!(diagram.participants[1].destroyed_line.is_some());
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-destroy=\"API\""));
    assert!(
        !svg.lines()
            .find(|line| line.contains("data-message=\"\""))
            .unwrap()
            .contains("marker-end")
    );
    assert!(sequence::parse(&source.replace("Client -> API!", "Client -> Missing!")).is_err());
    assert!(
        sequence::parse(&source.replace(
            "Client -> API! : delete",
            "Client -> API! : delete\nAPI -> Client : late"
        ))
        .is_err()
    );
}

#[test]
fn leftward_creation_and_deletion_mark_the_receiving_lifeline() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nspawn(kind=create): Worker(label=\"Worker: Job\")+ <- Client : allocate\nWorker! <- Client : release\n}\nlayout {\nWorker < Client\n}\n";
    let diagram = sequence::parse(source).unwrap();
    assert_eq!(
        diagram
            .participants
            .iter()
            .find(|p| p.id == "Worker")
            .unwrap()
            .label,
        "Worker: Job"
    );
    assert!(
        diagram
            .events
            .iter()
            .any(|e| matches!(e, sequence::Event::Message(m)
        if m.kind == sequence::MessageKind::Create && m.from == "Client" && m.to == "Worker"))
    );
    assert!(
        diagram
            .events
            .iter()
            .any(|e| matches!(e, sequence::Event::Message(m)
        if m.kind == sequence::MessageKind::Delete && m.from == "Client" && m.to == "Worker"))
    );
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-destroy=\"Worker\""));
    assert!(sequence::parse(&source.replace("Worker! <- Client", "Worker! -> Client")).is_err());
}

#[test]
fn atomic_action_has_visible_box_and_link_to_receive_occurrence() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B : read\nat[request.receive] : action=\"Read attribute\"\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-annotation=\"action\""));
    assert!(svg.contains("data-action-link=\"true\""));
    assert!(svg.contains("Read attribute"));
    assert!(
        sequence::parse(&source.replace("request.receive", "request.send"))
            .unwrap_err()
            .message
            .contains("receive occurrence")
    );
}

#[test]
fn optional_semantic_check_only_disables_non_geometric_bound_check() {
    let source = "settings {\ntype: sequence\nsemantic-check: off\n}\nsemantics {\nA\nloop(min=5; max=2) {\nA ->> A : retry\n}\n}\n";
    assert!(sequence::parse(source).is_ok());
    assert!(
        sequence::parse(&source.replace("semantic-check: off", "semantic-check: on"))
            .unwrap_err()
            .message
            .contains("at least min")
    );
    let nested = "settings {\ntype: sequence\nsemantic-check: off\n}\nsemantics {\nA\ndecompose A \"Inner\" {\nB\nloop(min=5; max=2) {\nB ->> B : retry\n}\n}\n}\n";
    assert!(sequence::parse(nested).is_ok());
    assert!(sequence::parse(&source.replace("A ->> A : retry", "A ->> Unknown : retry")).is_err());
    assert!(
        sequence::parse(&source.replace("semantic-check: off", "semantic-check: nope")).is_err()
    );
}

#[test]
fn lss_annotation_selector_styles_action_shape_and_link() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B : read\nat[request.receive] : action=\"Read\"\n}\nstyling {\nannotation[action] {\ncolor: \"#913200\"\nfill: \"#ffeddd\"\nfont-size: 21\nwidth: 2\n}\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg = sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let shape = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"action\""))
        .unwrap();
    let link = svg
        .lines()
        .find(|line| line.contains("data-action-link=\"true\""))
        .unwrap();
    assert!(shape.contains("fill=\"#ffeddd\""));
    assert!(link.contains("stroke=\"#913200\""));
    assert!(svg.contains("font-size=\"21.00\""));
}

#[test]
fn lss_ordering_styles_both_path_and_arrowhead() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nfirst: A -> B\nsecond: B -> A\norder[first.receive,second.send]\n}\nstyling {\nordering {\ncolor: \"#551188\"\nwidth: 2\n}\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg = sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let order = svg
        .lines()
        .find(|line| line.contains("data-general-ordering=\"true\""))
        .unwrap();
    assert!(order.contains("stroke=\"#551188\""));
    assert!(order.contains("stroke-width=\"2.00\""));
    assert!(svg.contains("fill=\"#551188\""));
}

#[test]
fn large_styled_action_reserves_space_before_following_message() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nfirst: A -> B : one\nat[first.receive] : action=\"Read\"\nnext: B -> A : two\n}\nstyling {\nannotation[action] {\nfont-size: 80\n}\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/sequence-basic.laml"
        )),
    )
    .unwrap();
    let svg = sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let action = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"action\""))
        .unwrap();
    let attr = |name: &str| -> f64 {
        action
            .split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let next = svg
        .lines()
        .find(|line| line.contains("data-message=\"next\""))
        .unwrap();
    let y: f64 = next
        .split(" d=\"M")
        .nth(1)
        .unwrap()
        .split(',')
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(y > attr("y") + attr("height") + 10.0);
}

#[test]
fn general_ordering_can_link_named_execution_boundaries_to_messages() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nwork: B+ : \"Read\"\ncall: A -> B : query\nwork: B-\norder[execution(B,work).start,call.receive]\norder[call.receive,execution(B,work).finish]\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert_eq!(svg.matches("data-general-ordering=\"true\"").count(), 2);
    let cycle = source.replace(
        "order[execution(B,work).start,call.receive]",
        "order[execution(B,work).finish,execution(B,work).start]",
    );
    assert!(
        sequence::parse(&cycle)
            .unwrap_err()
            .message
            .contains("ordering cycle")
    );
    let unknown = source.replace("execution(B,work).start", "execution(B,missing).start");
    assert!(
        sequence::parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown earlier execution boundary")
    );
    let early_finish = source
        .replace(
            "work: B-\n",
            "order[call.receive,execution(B,work).finish]\nwork: B-\n",
        )
        .replace("order[call.receive,execution(B,work).finish]\n}\n", "}\n");
    assert!(
        sequence::parse(&early_finish)
            .unwrap_err()
            .message
            .contains("unknown earlier execution boundary")
    );
}

#[test]
fn timing_annotations_can_anchor_named_execution_boundaries() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nwork: A+ : \"Compute\"\nwork: A-\nat[execution(A,work).start] : time-observation=\"t0\"\nat[execution(A,work).finish] : time-constraint=\"t < 30\"\nbetween[execution(A,work).start,execution(A,work).finish] : duration-observation=\"elapsed\"\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-annotation=\"time-observation\""));
    assert!(svg.contains("data-annotation=\"time-constraint\""));
    assert!(svg.contains("data-annotation=\"duration-observation\""));
    let bar = svg
        .lines()
        .find(|line| line.contains("data-activation-id=\"work\""))
        .unwrap();
    let elapsed = svg
        .lines()
        .find(|line| line.contains(">elapsed</text>"))
        .unwrap();
    let value = |line: &str, name: &str| -> f64 {
        line.split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(value(elapsed, "x") > value(bar, "x") + value(bar, "width"));
    let absent = source.replace("execution(A,work).finish", "execution(A,missing).finish");
    assert!(sequence::parse(&absent).is_err());
}
