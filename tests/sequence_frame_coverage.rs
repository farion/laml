use laml::{sequence, svg::TextMode};

#[test]
fn named_frame_coverage_is_mandatory_and_scopes_its_rectangle() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nC\nleft: alt \"yes\" {\nA ->> B : one\n}\nright: alt \"no\" {\nB ->> C : two\n}\n}\nlayout {\nleft covers A,B\nright covers B,C\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    let attr = |line: &str, name: &str| -> f64 {
        line.split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let left = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"left\""))
        .unwrap();
    let right = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"right\""))
        .unwrap();
    assert!(attr(left, "x") < attr(right, "x"));
    assert!(attr(left, "x") + attr(left, "width") < attr(right, "x") + attr(right, "width"));
    assert!(sequence::parse(&source.replace("left covers A,B", "left covers A,C")).is_err());
    assert!(sequence::parse(&source.replace("right covers B,C", "missing covers B,C")).is_err());
    assert!(sequence::parse(&source.replace("left covers A,B", "left covers A,Unknown")).is_err());
}

#[test]
fn scoped_ref_gate_meets_its_own_frame_not_the_global_diagram_boundary() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nWorker\nlookup: ref \"Lookup\" {\nAPI -> gate[result] : done\n}\n}\nlayout {\nlookup covers API\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    let attr = |line: &str, name: &str| -> f64 {
        line.split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let frame = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"lookup\""))
        .unwrap();
    let message = svg
        .lines()
        .find(|line| line.contains("data-message=\"\""))
        .unwrap();
    let x: f64 = message
        .split(" L")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (x - (attr(frame, "x") + attr(frame, "width"))).abs() < 0.02
            || (x - attr(frame, "x")).abs() < 0.02
    );
}

#[test]
fn inline_decomposition_can_scope_its_own_named_frame_from_root_layout() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\ndecompose API \"Internals\" {\nCache\nDB\nlocal: alt \"yes\" {\nCache ->> DB : query\n}\n}\n}\nlayout {\nAPI.local covers API.Cache,API.DB\n}\n";
    let diagram = sequence::parse(source).unwrap();
    assert_eq!(
        diagram.decompositions[0].diagram.frame_layout[0].id,
        "local"
    );
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-fragment-id=\"local\""));
}

#[test]
fn named_gate_has_own_css_like_style() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nref \"Lookup\" {\nAPI -> gate[result] : done\n}\n}\nstyling {\ngate[result] {\ncolor: \"#994411\"\nwidth: 3\nfont-size: 22\n}\n}\n";
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
    let mark = svg
        .lines()
        .find(|line| line.contains("data-gate=\"result\""))
        .unwrap();
    assert!(mark.contains("stroke=\"#994411\""));
    assert!(mark.contains("stroke-width=\"3.00\""));
    assert!(svg.contains("font-size=\"22.00\""));
}

#[test]
fn one_line_ref_uses_fragment_lss_and_reserves_label_height() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nref \"Some interaction\"\nA ->> A : after\n}\nstyling {\nfragment[ref] {\ncolor: \"#771122\"\nborder-color: \"#771122\"\nfont-size: 28\n}\n}\n";
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
    assert!(svg.contains("Some interaction"));
    assert!(svg.contains("font-size=\"28.00\""));
    assert!(svg.contains("stroke=\"#771122\""));
}

#[test]
fn long_styled_reference_label_fits_inside_the_svg_canvas() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nref \"A very long interaction name (input, out result): return value\"\n}\nstyling {\nfragment[ref] {\nfont-size: 30\n}\n}\n";
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
    let root = svg.lines().next().unwrap();
    let canvas: f64 = root
        .split(" width=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let text = svg
        .lines()
        .find(|line| line.contains("A very long interaction name"))
        .unwrap();
    let x: f64 = text
        .split(" x=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        x + laml::text::measure(
            "A very long interaction name (input, out result): return value",
            30.0
        )
        .unwrap()
            < canvas
    );
}

#[test]
fn css_ancestor_scopes_an_annotation_inside_one_decomposition() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nWorker\ndecompose API \"One\" {\nA\nB\nread: A -> B\nat[read.receive] : action=\"Audit\"\n}\ndecompose Worker \"Two\" {\nA\nB\nread: A -> B\nat[read.receive] : action=\"Audit\"\n}\n}\nstyling {\nnode[API] annotation[action] {\nfill: \"#bbddff\"\n}\n}\n";
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
    let actions: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-annotation=\"action\""))
        .collect();
    assert_eq!(actions.len(), 2);
    assert!(actions[0].contains("fill=\"#bbddff\""));
    assert!(actions[1].contains("fill=\"white\""));
}

#[test]
fn combined_fragment_gate_meets_its_own_boundary() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nC\nbranch: alt \"x\" {\nB -> gate[out] : signal\n}\n}\nlayout {\nbranch covers B\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    let frame = svg
        .lines()
        .find(|line| line.contains("data-fragment-id=\"branch\""))
        .unwrap();
    let message = svg
        .lines()
        .find(|line| line.contains("data-message=\"\""))
        .unwrap();
    let attr = |name: &str| -> f64 {
        frame
            .split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let x: f64 = message
        .split(" L")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((x - attr("x")).abs() < 0.02 || (x - attr("x") - attr("width")).abs() < 0.02);
    assert!(
        sequence::parse(&source.replace(
            "branch: alt \"x\" {",
            "A -> gate[out] : no frame\nbranch: alt \"x\" {"
        ))
        .is_err()
    );
}

#[test]
fn outside_participant_can_connect_to_a_scoped_fragment_gate() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\ncheckout: alt \"ready\" {\nClient -> gate[entry] : enter\ngate[entry] -> API : deliver\n}\n}\nlayout {\ncheckout covers API\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    let gates: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-gate=\"entry\""))
        .collect();
    assert_eq!(gates.len(), 1);
    let x = |line: &str| -> f64 {
        line.split(" d=\"M")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let paths: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-message=\"\""))
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .all(|path| path.contains(&format!("{:.2}", x(gates[0]))))
    );
    assert!(svg.contains("data-fragment-id=\"checkout\""));
    let invalid = source.replace("Client -> gate[entry] : enter", "Client -> API : enter");
    assert!(
        sequence::parse(&invalid)
            .unwrap_err()
            .message
            .contains("outside frame")
    );
    let duplicate_incoming = source.replace(
        "gate[entry] -> API : deliver",
        "API -> gate[entry] : deliver",
    );
    assert!(
        sequence::render::render(
            &sequence::parse(&duplicate_incoming).unwrap(),
            TextMode::System
        )
        .unwrap_err()
        .contains("one incoming and one outgoing")
    );
}
