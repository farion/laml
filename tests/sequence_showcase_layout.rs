use laml::sequence::{parse, render::render};
use laml::svg::TextMode;

fn attribute<'a>(line: &'a str, name: &str) -> &'a str {
    line.split_once(&format!("{name}=\""))
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap()
}
fn message(svg: &str, id: &str) -> (f64, f64) {
    let line = svg
        .lines()
        .find(|line| line.contains(&format!("data-message=\"{id}\"")))
        .unwrap();
    let path = attribute(line, "d").split_whitespace().collect::<Vec<_>>();
    let x = |part: &str| part[1..].split(',').next().unwrap().parse::<f64>().unwrap();
    (x(path[0]), x(path[1]))
}
fn bar(svg: &str, owner: &str) -> (f64, f64) {
    let line = svg
        .lines()
        .find(|line| line.contains(&format!("data-activation-owner=\"{owner}\"")))
        .unwrap();
    let left = attribute(line, "x").parse::<f64>().unwrap();
    (
        left,
        left + attribute(line, "width").parse::<f64>().unwrap(),
    )
}

#[test]
fn lifelines_are_visible_through_fragment_backgrounds_but_behind_executions() {
    let svg = render(
        &parse(include_str!("../examples/showcase-sequence.laml")).unwrap(),
        TextMode::System,
    )
    .unwrap();
    let loop_frame = svg.find("data-fragment-operator=\"loop\"").unwrap();
    let lifeline = svg.find("stroke-dasharray=\"5 5\"").unwrap();
    let execution = svg.find("data-activation-owner=\"API\"").unwrap();
    assert!(loop_frame < lifeline && lifeline < execution);
}

#[test]
fn nested_fragment_backgrounds_are_painted_outer_first() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nalt \"ready\" {\nloop(min=1; max=2) {\nA -> B : ping\n}\n}\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(
        svg.find("data-fragment-operator=\"alt\"").unwrap()
            < svg.find("data-fragment-operator=\"loop\"").unwrap()
    );
    assert!(
        svg.find("data-fragment-operator=\"loop\"").unwrap()
            < svg.find("stroke-dasharray=\"5 5\"").unwrap()
    );
}

#[test]
fn showcase_messages_meet_execution_edges_and_annotation_stays_clear() {
    let source = include_str!("../examples/showcase-sequence.laml");
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    let (api_left, api_right) = bar(&svg, "API");
    let (db_left, _) = bar(&svg, "DB");
    assert!((message(&svg, "request").1 - api_left).abs() < 0.01);
    assert!((message(&svg, "lookup").0 - api_right).abs() < 0.01);
    assert!((message(&svg, "lookup").1 - db_left).abs() < 0.01);
    assert!((message(&svg, "found").1 - api_right).abs() < 0.01);
    let invariant = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"invariant\""))
        .unwrap();
    assert!(attribute(invariant, "x").parse::<f64>().unwrap() > api_right + 25.0);
    assert!(svg.contains("data-destroy=\"Browser\""));
    assert!(svg.contains("data-destroy=\"Worker\""));
    assert!(!svg.contains("data-note-anchor="));
    let root = svg.lines().next().unwrap();
    assert!(attribute(root, "width").parse::<f64>().unwrap() < 1400.0);
    assert!(attribute(root, "height").parse::<f64>().unwrap() < 1300.0);
}

#[test]
fn invariant_clears_a_wide_styled_execution() {
    use laml::{styles::load, syntax::Diagram};
    use std::path::Path;
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\n@request A -> B+ : ask\nat[request.receive] : invariant=\"authenticated\"\nB-\n}\nstyling {\nactivation {\nwidth: 52\n}\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(
        &Diagram {
            styling: diagram.styling.clone(),
            ..Default::default()
        },
        Path::new("examples/showcase-sequence.laml"),
    )
    .unwrap();
    let svg =
        laml::sequence::render::render_with_styles(&diagram, &sheet, TextMode::System).unwrap();
    let (left, right) = bar(&svg, "B");
    assert!((message(&svg, "request").1 - left).abs() < 0.01);
    let text = svg
        .lines()
        .find(|line| line.contains("data-annotation=\"invariant\""))
        .unwrap();
    let center = attribute(text, "x").parse::<f64>().unwrap();
    let width = laml::text::measure("{authenticated}", 14.0).unwrap();
    assert!(center - width / 2.0 >= right + 11.0);
}

#[test]
fn recursive_self_call_meets_the_new_nested_execution_edge() {
    let source =
        "settings {\ntype: sequence\n}\nsemantics {\nA\nA+\n@retry A -> A+ : recurse\nA-\nA-\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    let line = svg
        .lines()
        .find(|line| line.contains("data-message=\"retry\""))
        .unwrap();
    assert!(attribute(line, "d").contains(" H"));
}

#[test]
fn note_right_of_participant_is_placed_without_a_leader() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nA -> B : request\nhint: note \"Hello\"\n}\nlayout {\nhint right of B : gap=12\n}\n";
    let svg = render(&parse(source).unwrap(), TextMode::System).unwrap();
    assert!(svg.contains("data-note=\"hint\""));
    assert!(!svg.contains("data-note-anchor="));
}
