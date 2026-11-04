use laml::{geometry::place, parser::parse, route::route, styles::StyleSheet};

#[test]
fn orthogonal_is_default_and_free_routing_is_opt_in() {
    let source = include_str!("../examples/package-architecture.laml");
    let orthogonal = parse(source).unwrap();
    assert!(!orthogonal.settings.allow_diagonal);
    let boxes = place(&orthogonal, &StyleSheet::default()).unwrap();
    let routed = route(&orthogonal, &boxes, &StyleSheet::default()).unwrap();
    assert!(
        routed
            .iter()
            .flat_map(|r| r.points.windows(2))
            .all(|p| { (p[0].x - p[1].x).abs() < 1e-4 || (p[0].y - p[1].y).abs() < 1e-4 })
    );
    let legacy =
        parse(&source.replace("type: package", "type: package\nallow-diagonal: true")).unwrap();
    assert!(legacy.settings.allow_diagonal);
    let boxes = place(&legacy, &StyleSheet::default()).unwrap();
    let routes = route(&legacy, &boxes, &StyleSheet::default()).unwrap();
    assert!(
        routes[0]
            .points
            .windows(2)
            .any(|p| { (p[0].x - p[1].x).abs() > 1.0 && (p[0].y - p[1].y).abs() > 1.0 })
    );
}

#[test]
fn strict_mode_reaches_c4_and_named_relation_attachments() {
    for source in [
        include_str!("../examples/class-basic.laml"),
        include_str!("../examples/showcase-generic.laml"),
    ] {
        let diagram = parse(source).unwrap();
        let sheet = StyleSheet::default();
        let boxes = place(&diagram, &sheet).unwrap();
        let routes = route(&diagram, &boxes, &sheet).unwrap();
        assert!(
            routes
                .iter()
                .flat_map(|r| r.points.windows(2))
                .all(|p| { (p[0].x - p[1].x).abs() < 1e-4 || (p[0].y - p[1].y).abs() < 1e-4 })
        );
    }
    let diagram = laml::c4::parse(include_str!("../examples/c4-banking-container.laml")).unwrap();
    assert!(!diagram.settings.allow_diagonal);
    let sheet = laml::styles::load(
        &diagram,
        std::path::Path::new("examples/c4-banking-container.laml"),
    )
    .unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert!(
        routes
            .iter()
            .flat_map(|r| r.points.windows(2))
            .all(|p| { (p[0].x - p[1].x).abs() < 1e-4 || (p[0].y - p[1].y).abs() < 1e-4 })
    );
}

#[test]
fn sequence_keeps_receive_time_without_a_diagonal_message() {
    let source = include_str!("../examples/sequence-coregion.laml");
    let strict = laml::sequence::parse(source).unwrap();
    let svg = laml::sequence::render::render(&strict, laml::svg::TextMode::System).unwrap();
    let first = svg
        .lines()
        .find(|s| s.contains("data-message=\"first\""))
        .unwrap();
    assert!(first.contains(" H") && first.contains(" V"), "{first}");
    let elbow: f64 = first
        .split(" H")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let second = svg
        .lines()
        .find(|s| s.contains("data-message=\"second\""))
        .unwrap();
    let second_start: f64 = second
        .split("d=\"M")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        elbow < second_start,
        "time leg must not cross the later message or lie on B's lifeline"
    );
    let legacy_source = source.replace("type: sequence", "type: sequence\nallow-diagonal: true");
    let free = laml::sequence::parse(&legacy_source).unwrap();
    let svg = laml::sequence::render::render(&free, laml::svg::TextMode::System).unwrap();
    let old = svg
        .lines()
        .find(|s| s.contains("data-message=\"first\""))
        .unwrap();
    assert!(old.contains(" L") && !old.contains(" V"), "{old}");
}

#[test]
fn sequence_annotations_obey_the_connector_setting() {
    let source = include_str!("../examples/sequence-annotations.laml");
    let strict = laml::sequence::parse(source).unwrap();
    let svg = laml::sequence::render::render(&strict, laml::svg::TextMode::System).unwrap();
    let observation = svg
        .lines()
        .find(|line| {
            line.contains("data-annotation=\"time-observation\"") && line.contains("<path")
        })
        .unwrap();
    assert!(
        observation.contains(" V") && observation.contains(" h"),
        "{observation}"
    );
    let free_source = source.replace("type: sequence", "type: sequence\nallow-diagonal: true");
    let free = laml::sequence::parse(&free_source).unwrap();
    let svg = laml::sequence::render::render(&free, laml::svg::TextMode::System).unwrap();
    let observation = svg
        .lines()
        .find(|line| {
            line.contains("data-annotation=\"time-observation\"") && line.contains("<path")
        })
        .unwrap();
    assert!(observation.contains(" l"), "{observation}");
}

#[test]
fn independent_renderers_accept_the_same_switch() {
    let bpmn = include_str!("../examples/bpmn-basic.laml");
    assert!(!laml::bpmn::parse(bpmn).unwrap().allow_diagonal);
    assert!(
        laml::bpmn::parse(&bpmn.replace("type: bpmn", "type: bpmn\nallow-diagonal: true"))
            .unwrap()
            .allow_diagonal
    );
    let timing = include_str!("../examples/timing-basic.laml");
    assert!(!laml::timing::parse(timing).unwrap().allow_diagonal);
    assert!(
        laml::timing::parse(&timing.replace("type: timing", "type: timing\nallow-diagonal: true"))
            .unwrap()
            .allow_diagonal
    );
}

#[test]
fn the_switch_is_validated_in_every_parser() {
    let sources = [
        "settings {\ntype: activity\nallow-diagonal: maybe\n}\nsemantics {\ninitial Start\nactivityFinal End\ncontrol Start --> End\n}",
        "settings {\ntype: bpmn\nallow-diagonal: maybe\n}\nsemantics {\nStart --> End\n}",
        "settings {\ntype: sequence\nallow-diagonal: maybe\n}\nsemantics {\nparticipant A\nparticipant B\nA -> B : go\n}",
        "settings {\ntype: timing\nallow-diagonal: maybe\n}\nsemantics {\nlifeline User : Idle\n}\nlayout {\nend: 3\n}",
    ];
    assert!(
        parse(sources[0])
            .unwrap_err()
            .message
            .contains("allow-diagonal")
    );
    assert!(
        laml::bpmn::parse(sources[1])
            .unwrap_err()
            .contains("allow-diagonal")
    );
    assert!(
        laml::sequence::parse(sources[2])
            .unwrap_err()
            .message
            .contains("allow-diagonal")
    );
    assert!(
        laml::timing::parse(sources[3])
            .unwrap_err()
            .message
            .contains("allow-diagonal")
    );
}
