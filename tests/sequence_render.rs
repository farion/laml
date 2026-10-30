use laml::sequence::{parse, render::render};
use laml::svg::TextMode;

const SAMPLE: &str = "settings {\ntype: sequence\n}\nsemantics {\nClient(label=\"Browser\")\nAPI\nClient -> API+ : request\n<-- : response\nClient ->> API : event\nAPI -> API : retry\n}\n";

#[test]
fn renders_sequence_lifelines_arrows_activations_and_self_calls() {
    let diagram = parse(SAMPLE).unwrap();
    let svg = render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("Browser"));
    assert!(svg.contains("stroke-dasharray=\"5 5\""));
    assert!(svg.contains("stroke-dasharray=\"7 5\""));
    assert!(svg.contains("data-arrowhead=\"filled\""));
    assert!(svg.contains("data-arrowhead=\"open\""));
    assert!(svg.contains("h44 v24.00 h-44"));
    assert!(svg.contains("width=\"12.00\""));
}

#[test]
fn sequence_svg_is_convertible_to_png_and_pdf() {
    let diagram = parse(SAMPLE).unwrap();
    let svg = render(&diagram, TextMode::Embedded).unwrap();
    assert!(laml::export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF"));
}

#[test]
fn alternative_frame_has_guard_and_branch_separator() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nClient -> API : query\nalt \"yes\" {\n<-- : found\n}\nelse \"no\" {\n<-- : missing\n}\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"alt\""));
    assert!(svg.contains("[yes]"));
    assert!(svg.contains("[no]"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert_eq!(svg.matches("stroke-dasharray=\"7 5\"").count(), 2);
}

#[test]
fn annotation_is_drawn_at_selected_message_end() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nrequest: Client -> API : GET\nat[request.receive] : time-constraint=\"t <= 200 ms\"\nat[request.send] : state=\"Ready\"\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-annotation=\"time-constraint\""));
    assert!(svg.contains("data-annotation=\"state\""));
    assert!(svg.contains("t &lt;= 200 ms"));
}

#[test]
fn existing_node_and_relation_lss_selectors_style_sequence_objects() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nrequest: Client -> API : ask\n}\nstyling {\nnode[API] {\nfill: \"#123456\"\ncolor: \"#ffffff\"\nfont-size: 20\n}\nrelation[request] {\ncolor: \"#aa3131\"\nwidth: 3\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &styles, TextMode::System).unwrap();
    assert!(svg.contains("fill=\"#123456\""));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.contains("stroke=\"#aa3131\" stroke-width=\"3.00\""));
    assert!(svg.contains("font-size=\"20.00\""));
}

#[test]
fn explicit_creation_is_dashed_and_reply_can_use_a_filled_arrow() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nspawn(kind=create): A -> B+ : create\nreply(arrow=filled): A <-- B : done\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    let create = svg
        .lines()
        .find(|line| line.contains("data-message=\"spawn\""))
        .unwrap();
    let reply = svg
        .lines()
        .find(|line| line.contains("data-message=\"reply\""))
        .unwrap();
    assert!(create.contains("stroke-dasharray=\"7 5\""));
    assert!(reply.contains("stroke-dasharray=\"7 5\""));
    assert!(
        svg.lines()
            .filter(|line| line.contains("data-arrowhead=\"filled\""))
            .count()
            == 1
    );
}

#[test]
fn state_invariant_uses_curly_braces_not_a_state_box() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B\nat[request.receive] : invariant=\"ready & active\"\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-annotation=\"invariant\">{ready &amp; active}</text>"));
    assert!(!svg.contains("data-annotation=\"state\""));
}

#[test]
fn locally_named_executions_can_be_styled_individually() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nwork: API+ : \"Process\"\ncheck: API+ : \"Audit\"\ncheck: API-\nwork: API-\n}\nstyling {\nactivation[API] {\nfill: \"#cccccc\"\n}\nnode[API] activation[check] {\nfill: \"#772233\"\nfont-size: 18\n}\n}\n";
    let diagram = parse(source).unwrap();
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let work = svg
        .lines()
        .find(|line| line.contains("data-activation-id=\"work\""))
        .unwrap();
    let check = svg
        .lines()
        .find(|line| line.contains("data-activation-id=\"check\""))
        .unwrap();
    assert!(work.contains("fill=\"#cccccc\""));
    assert!(check.contains("fill=\"#772233\""));
    assert!(svg.contains("Process") && svg.contains("Audit"));
    assert!(check.contains("width=\""));
    assert!(
        svg.find("data-activation-id=\"work\"").unwrap()
            < svg.find("data-activation-id=\"check\"").unwrap()
    );
    let created =
        "settings {\ntype: sequence\n}\nsemantics {\nwork: API+ : \"Process\"\nwork: API-\n}\n";
    let created_svg = render(&parse(created).unwrap(), TextMode::System).unwrap();
    assert!(created_svg.contains("data-activation-id=\"work\""));
    assert!(created_svg.contains("Process"));
}

#[test]
fn coregion_marks_only_the_named_lifeline() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nAPI\nfirst: A -> API : request\nsecond: B -> API : event\ncoregion API {\nfirst.receive\nsecond.receive\n}\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-coregion=\"API\""));
    assert!(!svg.contains("data-coregion=\"A\""));
    let reordered = format!("{source}layout {{\nsecond.receive above first.receive\n}}\n");
    let svg = render(&parse(&reordered).unwrap(), TextMode::System).unwrap();
    let end_y = |id: &str| -> f64 {
        let line = svg
            .lines()
            .find(|line| line.contains(&format!("data-message=\"{id}\"")))
            .unwrap();
        line.split(" L")
            .nth(1)
            .unwrap()
            .split(',')
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(end_y("second") < end_y("first"));
    for name in ["first", "second"] {
        let line = svg
            .lines()
            .find(|line| line.contains(&format!("data-message=\"{name}\"")))
            .unwrap();
        let send_y: f64 = line
            .split("d=\"M")
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
        assert!(end_y(name) >= send_y, "message {name} points upward");
    }
    assert!(svg.contains("data-arrowhead=\"filled\""));
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let styled = reordered.replace("API\n", "API(style-classes=critical)\n")
        + "styling {\nnode.critical coregion {\ncolor: \"#771133\"\n}\n}\n";
    let diagram = parse(&styled).unwrap();
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
    let colored =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(
        colored
            .lines()
            .any(|line| line.contains("data-coregion=\"API\"")
                && line.contains("stroke=\"#771133\""))
    );
}

#[test]
fn referenced_note_has_folded_shape_and_independent_style() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nA -> A : first\nretryHint: note \"Retry <safely>\"\nA -> A : second\n}\nlayout {\nretryHint over A : gap=12\n}\nstyling {\nnote[retryHint] {\nfill: \"#eeeecc\"\nborder-color: \"#221100\"\ncolor: \"#332211\"\n}\n}\n";
    let diagram = parse(source).unwrap();
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.contains("data-note=\"retryHint\""));
    assert!(svg.contains("fill=\"#eeeecc\""));
    assert!(svg.contains("stroke=\"#221100\""));
    assert!(svg.contains("Retry &lt;safely&gt;"));
    let anchored = source
        .replace("A -> A : first", "request: A -> A : first")
        .replace("retryHint over A", "retryHint over request.receive");
    let anchored_svg = laml::sequence::render::render_with_styles(
        &parse(&anchored).unwrap(),
        &sheet,
        TextMode::System,
    )
    .unwrap();
    assert!(anchored_svg.contains("data-note-anchor=\"request.receive\""));
    let message_line = anchored_svg
        .lines()
        .find(|line| line.contains("h44 v24.00 h-44"))
        .unwrap();
    let leader_line = anchored_svg
        .lines()
        .find(|line| line.contains("data-note-anchor"))
        .unwrap();
    let path_y = |line: &str| -> f64 {
        line.split("d=\"M")
            .nth(1)
            .unwrap()
            .split(',')
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert_eq!(path_y(leader_line), path_y(message_line) + 24.0);
}

#[test]
fn relation_class_styles_messages_without_changing_other_arrows() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest(style-classes=protocol): A -> B : call\nA ->> B : event\n}\nstyling {\nrelation.protocol {\ncolor: \"#aabbcc\"\n}\n}\n";
    let diagram = parse(source).unwrap();
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.contains("stroke=\"#aabbcc\""));
    assert!(svg.contains("fill=\"#aabbcc\" data-arrowhead=\"filled\""));
    assert!(svg.contains("stroke=\"#20252b\""));
}

#[test]
fn contextual_lss_targets_parts_of_a_classed_participant() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI(label=\"Orders\"; style-classes=critical)\nClient -> API+ : request\n}\nstyling {\nnode.critical {\nfill: \"#332211\"\n}\nnode.critical lifeline {\ncolor: \"#112233\"\n}\nnode.critical activation {\nfill: \"#aabbcc\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.participants[1].classes, ["critical"]);
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.contains("fill=\"#332211\""));
    assert!(svg.contains("stroke=\"#112233\""));
    assert!(svg.contains("fill=\"#aabbcc\""));
    let bad = source.replace("node.critical lifeline", "lifeline.critical");
    assert!(parse(&bad).is_err());
}

#[test]
fn new_lss_selectors_style_lifeline_execution_and_fragment() {
    use laml::styles::load;
    use laml::syntax::Diagram;
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nAPI\nClient -> API+ : ask\nalt \"yes\" {\n<-- : okay\n}\nelse \"no\" {\n<-- : fail\n}\n}\nstyling {\nlifeline[API] {\ncolor: \"#112233\"\nwidth: 2\n}\nactivation[API] {\nfill: \"#ddeeff\"\nwidth: 16\n}\nfragment[alt] {\nborder-color: \"#445566\"\nfont-size: 18\n}\n}\n";
    let diagram = parse(source).unwrap();
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
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(svg.contains("stroke=\"#112233\" stroke-width=\"2.00\""));
    assert!(svg.contains("width=\"16.00\""));
    assert!(svg.contains("fill=\"#ddeeff\""));
    assert!(svg.contains("stroke=\"#445566\""));
    assert!(svg.contains("font-size=\"18.00\""));
}

#[test]
fn general_ordering_draws_explicit_link() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nfirst: A -> B : one\nsecond: B -> A : two\norder[first.receive,second.send]\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-general-ordering=\"true\""));
    assert!(svg.contains("stroke-dasharray=\"2 3\""));
}

#[test]
fn duration_brackets_connect_two_named_message_ends() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nrequest: A -> B : request\nreply: A <-- B : reply\nat[request.send] : time-observation=\"t1\"\nbetween[request.send,reply.receive] : duration-observation=\"delta\"\nbetween[request.receive,reply.send] : duration-constraint=\"<= 50 ms\"\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    for kind in [
        "time-observation",
        "duration-observation",
        "duration-constraint",
    ] {
        assert!(svg.contains(&format!("data-annotation=\"{kind}\"")));
    }
    assert!(svg.contains("[&lt;= 50 ms]"));
}

#[test]
fn gates_connect_to_the_ref_boundary_with_visible_names() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\nref \"Orders\" {\nAPI -> gate[auftrag] : request\ngate[ergebnis] -> API : response\n}\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"ref\""));
    assert!(svg.contains("Orders"));
    assert!(svg.contains("data-gate=\"auftrag\""));
    assert!(svg.contains("data-gate=\"ergebnis\""));
    assert_eq!(svg.matches("data-gate=").count(), 2);
}

#[test]
fn unknown_message_ends_are_dots_not_participant_headers() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI\n? -> API : incoming\nAPI -> ? : outgoing\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-unknown-end=\"source\""));
    assert!(svg.contains("data-unknown-end=\"destination\""));
    assert_eq!(svg.matches("<circle").count(), 2);
}

#[test]
fn standalone_labeled_plus_draws_head_and_bar_without_arrow() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nAPI(label=\"Orders API\")+\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("Orders API"));
    assert!(svg.contains("width=\"12.00\""));
    assert!(!svg.contains("marker-end=\"url(#filled)\""));
    let without = render(
        &parse(&source.replace("API(label=\"Orders API\")+", "API(label=\"Orders API\")")).unwrap(),
        TextMode::System,
    )
    .unwrap();
    assert!(!without.contains("width=\"12.00\""));
}

#[test]
fn destruction_draws_x_and_terminates_lifeline() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nClient\nClient -> API+ : start\nAPI!\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-destroy=\"API\""));
    assert!(svg.contains("stroke-width=\"2\" fill=\"none\""));
}

#[test]
fn first_plus_places_created_header_on_first_message() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nClient -> API+ : spawn\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains(">API</text>"));
    assert!(svg.contains("y=\"108.00\""));
    assert!(svg.contains("L"));
}

#[test]
fn loop_frame_displays_bounds_and_guard() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nloop(min=2; max=5; guard=\"ready\") {\nA ->> B : ping\n}\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"loop(2,5)\""));
    assert!(svg.contains("[ready]"));
}

#[test]
fn reference_frame_draws_label_without_loading_another_file() {
    let source = SAMPLE.replace(
        "API -> API : retry\n",
        "ref \"Check inventory\"\nAPI -> API : retry\n",
    );
    let diagram = parse(&source).unwrap();
    let svg = render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"ref\""));
    assert!(svg.contains("Check inventory"));
    assert!(svg.contains("height=\"50.00\" fill=\"none\""));
}

#[test]
fn mandatory_lane_gap_expands_canvas() {
    let baseline = parse(SAMPLE).unwrap();
    let with_gap = parse(&SAMPLE.replace(
        "API -> API : retry\n}\n",
        "API -> API : retry\n}\nlayout {\nClient < API : gap=600\n}\n",
    ))
    .unwrap();
    let width = |diagram| {
        let svg = render(diagram, TextMode::System).unwrap();
        let width = svg
            .split("width=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        width.parse::<f64>().unwrap()
    };
    assert!(width(&with_gap) > width(&baseline) + 400.0);
}

#[test]
fn labels_are_escaped_and_path_text_mode_is_supported() {
    let diagram = parse(&SAMPLE.replace("request", "<request & return>")).unwrap();
    let svg = render(&diagram, TextMode::System).unwrap();
    assert!(svg.contains("&lt;request &amp; return&gt;"));
    assert!(render(&diagram, TextMode::Paths).unwrap().contains("<svg"));
}
