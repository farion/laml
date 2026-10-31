use laml::{
    sequence::{parse, render::render_with_styles},
    styles::load,
    svg::TextMode,
};
use std::path::Path;

fn baseline(svg: &str, label: &str) -> f64 {
    let text = svg
        .lines()
        .find(|line| line.contains(&format!(">{label}</text>")))
        .expect("message label");
    text.split(" y=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn large_message_labels_expand_the_next_row_instead_of_overprinting_it() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\n@first A -> B : alpha\n@second A -> B : beta\n}\nstyling {\nrelation {\nfont-size: 80\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(
        &laml::syntax::Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new("examples/sequence-styled.laml"),
    )
    .unwrap();
    let svg = render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    assert!(baseline(&svg, "beta") - baseline(&svg, "alpha") > 84.0);
}

#[test]
fn several_observations_and_a_state_share_one_message_anchor() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\n@request A -> B : call\nat[request.send] : time-observation=\"t1\"\nat[request.send] : time-observation=\"t2\"\nat[request.send] : time-observation=\"t3\"\nat[request.send] : time-observation=\"t4\"\nat[request.send] : time-observation=\"t5\"\nat[request.send] : state=\"Ready\"\n}\n";
    let diagram = parse(source).unwrap();
    let svg = laml::sequence::render::render(&diagram, TextMode::System).unwrap();
    assert_eq!(
        svg.matches("data-annotation=\"time-observation\"").count(),
        10
    );
    assert!(svg.contains("data-annotation=\"state\""));
}

#[test]
fn duration_bracket_avoids_a_state_box_at_its_message_anchor() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sequence-annotations.laml"),
    )
    .unwrap();
    let svg = laml::sequence::render::render(&parse(&source).unwrap(), TextMode::System).unwrap();
    let state = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"state\""))
        .unwrap();
    let field = |name: &str| -> f64 {
        state
            .split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let bracket = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"duration-observation\""))
        .unwrap();
    let lane: f64 = bracket
        .split(" H")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(lane < field("x") - 2.0 || lane > field("x") + field("width") + 2.0);
}

#[test]
fn anchored_note_routes_around_time_constraint_text() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\n@request A -> B : call\nat[request.receive] : time-constraint=\"wait\"\nhint: note \"A note\"\n}\nlayout {\nhint over request.receive : gap=0\n}\n";
    let svg = laml::sequence::render::render(&parse(source).unwrap(), TextMode::System).unwrap();
    let note_link = svg
        .lines()
        .find(|line| line.contains("data-note-anchor=\"request.receive\""))
        .unwrap();
    let path = note_link
        .split(" d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(path.split(" L").count() >= 4);
}
