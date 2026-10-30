#[test]
fn inherited_execution_closes_independently_in_each_alternative() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nA+\nalt \"ok\" {\nA-\n}\nelse \"fail\" {\nA-\n}\n}\n";
    let svg = laml::sequence::render::render(
        &laml::sequence::parse(source).unwrap(),
        laml::svg::TextMode::System,
    )
    .unwrap();
    let bars: Vec<_> = svg
        .lines()
        .filter(|line| line.contains("data-activation-owner=\"A\""))
        .collect();
    assert_eq!(bars.len(), 2);
    let attribute = |line: &str, name: &str| -> f64 {
        line.split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(attribute(bars[0], "y") + attribute(bars[0], "height") <= attribute(bars[1], "y"));
}

#[test]
fn operand_local_creation_and_destruction_are_drawn_without_cross_branch_lifetimes() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nalt \"create\" {\nspawn(kind=create): Client -> Resource+ : new\nClient -> Resource! : remove\n}\nelse \"skip\" {\nClient ->> Client : idle\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
    assert!(svg.contains("data-participant=\"Resource\""));
    assert!(svg.contains("data-destroy=\"Resource\""));
    assert!(svg.contains("data-fragment-operator=\"alt\""));
    let without_end = source.replace("Client -> Resource! : remove\n", "");
    assert!(
        laml::sequence::parse(&without_end)
            .unwrap_err()
            .message
            .contains("must be destroyed within that operand")
    );
    let outside = source.replace(
        "Client -> Resource! : remove",
        "Client -> Resource : still alive",
    );
    assert!(laml::sequence::parse(&outside).is_err());
    let preexisting = source
        .replace("alt \"create\" {", "Resource\nalt \"create\" {")
        .replace("spawn(kind=create): Client -> Resource+ : new\n", "");
    assert!(
        laml::sequence::parse(&preexisting)
            .unwrap_err()
            .message
            .contains("different lifeline lifetimes")
    );
}

#[test]
fn combined_assert_consider_descriptor_uses_visible_message_list() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nassert consider \"allow, deny\" {\nA ->> B : allow\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"assert consider {allow, deny}\""));
    assert!(laml::sequence::parse(&source.replace("\"allow, deny\"", "\"\"")).is_err());
}

#[test]
fn long_operand_guard_and_large_font_expand_frame_and_canvas() {
    use laml::{styles::load, svg::TextMode, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nalt \"yes\" {\nA ->> B : first\n}\nelse \"a very long alternative guard that should fit without clipping\" {\nB ->> A : second\n}\n}\nstyling {\nfragment[alt] {\nfont-size: 31\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
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
    let rect = svg
        .lines()
        .find(|line| line.contains("data-fragment-operator=\"alt\""))
        .unwrap();
    let root = svg.lines().next().unwrap();
    assert!(attr(rect, "x") + attr(rect, "width") <= attr(root, "width"));
}

#[test]
fn same_lifeline_id_has_separate_birth_and_death_in_alternative_operands() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nalt \"first\" {\n(kind=create): Client -> Resource+ : create\nClient -> Resource! : delete\n}\nelse \"second\" {\n(kind=create): Client -> Resource+ : create\nClient -> Resource! : delete\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
    assert_eq!(
        diagram
            .births
            .iter()
            .filter(|(id, _)| id == "Resource")
            .count(),
        2
    );
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
    assert_eq!(svg.matches("data-participant=\"Resource\"").count(), 2);
    assert_eq!(svg.matches("data-destroy=\"Resource\"").count(), 2);
    let local = source.replace("(kind=create): Client -> Resource+ : create", "Resource+");
    assert!(laml::sequence::parse(&local).is_ok());
    let missing = source.replacen("Client -> Resource! : delete\n", "", 1);
    assert!(laml::sequence::parse(&missing).is_err());
}

#[test]
fn first_operand_guard_reserves_space_before_the_first_lifeline() {
    let diagram =
        laml::sequence::parse(include_str!("../examples/sequence-alternative-births.laml"))
            .unwrap();
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
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
    let head = svg
        .lines()
        .find(|line| line.contains("data-participant=\"Client\""))
        .unwrap();
    let text = svg
        .lines()
        .find(|line| line.contains(">[create from pool]</text>"))
        .unwrap();
    let center = attr(head, "x") + attr(head, "width") / 2.0;
    assert!(
        attr(text, "x") + laml::text::measure("[create from pool]", 14.0).unwrap() + 10.0 < center
    );
}

#[test]
fn preexisting_lifeline_can_die_in_every_alternative_but_not_only_one() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nClient\nResource\nalt \"one\" {\nClient -> Resource! : delete\n}\nelse \"two\" {\nClient -> Resource! : delete\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
    assert_eq!(diagram.events.iter().filter(|e| matches!(e, laml::sequence::Event::Destroy { participant, .. } if participant == "Resource")).count(), 2);
    assert!(
        laml::sequence::parse(&source.replacen(
            "Client -> Resource! : delete\n",
            "Client ->> Resource : keep\n",
            1
        ))
        .unwrap_err()
        .message
        .contains("different lifeline lifetimes")
    );
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
    assert_eq!(svg.matches("data-destroy=\"Resource\"").count(), 2);
    assert_eq!(svg.matches("stroke-dasharray=\"5 5\"").count(), 3);
}

#[test]
fn large_fragment_font_reserves_descriptor_height_before_first_message() {
    use laml::{styles::load, svg::TextMode, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nalt \"ready\" {\nA ->> B : message\n}\n}\nstyling {\nfragment[alt] {\nfont-size: 44\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
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
    let tab = svg
        .lines()
        .find(|line| line.contains("data-frame-descriptor=\"alt\""))
        .unwrap();
    let at: f64 = tab
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
    let msg = svg
        .lines()
        .find(|line| line.contains("data-message=\"\""))
        .unwrap();
    let at_msg: f64 = msg
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
    assert!(at_msg > at + 44.0 * 1.25 + 8.0);
}

#[test]
fn combined_fragment_operators_share_one_pentagon_and_keep_branch_form() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nstrict alt \"yes\" {\nA ->> B : one\n}\nelse \"no\" {\nB ->> A : two\n}\n}\n";
    let diagram = laml::sequence::parse(source).unwrap();
    let svg = laml::sequence::render::render(&diagram, laml::svg::TextMode::System).unwrap();
    assert!(svg.contains("data-frame-descriptor=\"strict alt\""));
    assert!(svg.contains("[no]"));
    let nested = source.replace(
        "strict alt \"yes\"",
        "opt loop(min=0; max=2; guard=\"repeat\")",
    );
    assert!(laml::sequence::parse(&nested).is_err()); // `else` belongs only to alt/par.
    let simple = "settings {\ntype: sequence\n}\nsemantics {\nA\nopt loop(min=0; max=2; guard=\"repeat\") {\nA ->> A : retry\n}\n}\n";
    let svg = laml::sequence::render::render(
        &laml::sequence::parse(simple).unwrap(),
        laml::svg::TextMode::System,
    )
    .unwrap();
    assert!(svg.contains("data-frame-descriptor=\"opt loop(0,2)\""));
}

#[test]
fn multiple_operator_pentagon_shorthand_supports_three_visible_operators() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nassert strict consider \"m,s\" {\nA ->> B : m\n}\n}\n";
    let svg = laml::sequence::render::render(
        &laml::sequence::parse(source).unwrap(),
        laml::svg::TextMode::System,
    )
    .unwrap();
    assert!(svg.contains("data-frame-descriptor=\"assert strict consider {m,s}\""));
}
