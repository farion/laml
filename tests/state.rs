use laml::{export, geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const BASIC: &str = include_str!("../examples/state-basic.laml");
const COMPOSITE: &str = include_str!("../examples/state-composite.laml");
const LOOP: &str = include_str!("../examples/state-loop.laml");

fn image(source: &str) -> String {
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, Path::new("examples/state-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    render(&boxes, &routes).unwrap()
}

#[test]
fn state_pseudostates_guarded_choice_and_behaviors_render() {
    let svg = image(BASIC);
    assert!(svg.contains("r=\"9\""));
    assert!(svg.contains("r=\"12\""));
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("rx=\"14.00\""));
    for label in [
        "entry / startSession()",
        "do / awaitWork()",
        "exit / stopSession()",
        "[valid]",
        "[else]",
    ] {
        assert!(svg.contains(label), "missing {label}");
    }
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn single_region_composite_keeps_its_inner_graph_in_the_rounded_frame() {
    let diagram = parse(COMPOSITE).unwrap();
    let sheet = load(&diagram, Path::new("examples/state-composite.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let parent = boxes.iter().find(|node| node.id == "Processing").unwrap();
    let inner = boxes
        .iter()
        .find(|node| node.id == "Processing.Running")
        .unwrap();
    assert!(parent.is_container);
    assert!(inner.x >= parent.x && inner.x + inner.width <= parent.x + parent.width);
    assert!(inner.y >= parent.y + parent.port_start);
    let svg = image(COMPOSITE);
    assert!(svg.contains("entry / open()"));
    assert!(svg.contains("exit / close()"));
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn self_transition_has_an_exterior_route_without_merging_other_edges() {
    let diagram = parse(LOOP).unwrap();
    let sheet = load(&diagram, Path::new("examples/state-loop.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    let loop_path = routed
        .iter()
        .find(|line| boxes[line.from].id == "Open" && boxes[line.to].id == "Open")
        .unwrap();
    let active = boxes.iter().find(|node| node.id == "Open").unwrap();
    assert!(
        loop_path
            .points
            .iter()
            .any(|point| point.y > active.y + active.height)
    );
    let svg = render(&boxes, &routed).unwrap();
    assert!(svg.contains("ping / keepAlive()"));
}

#[test]
fn unadorned_note_attachment_remains_distinct_from_transition() {
    let source = "settings {\ntype: state\n}\nsemantics {\nstate Ready\nnote Explanation \"Data is displayed, not executed\"\nExplanation .. Ready\n}\n";
    let svg = image(source);
    assert!(svg.contains("Data is displayed, not executed"));
    assert!(!svg.contains("marker-end=\"url(#arrow)\""));
    let with_initial_note = "settings {\ntype: state\n}\nsemantics {\ninitial Begin\nstate Ready\nBegin --> Ready\nnote Hint \"start\"\nHint .. Begin\n}\n";
    parse(with_initial_note).unwrap(); // a note link is not an incoming transition
}

#[test]
fn state_notation_rejects_impossible_pseudostate_and_transition_roles() {
    let base = "settings {\ntype: state\n}\nsemantics {\nREPLACE\n}\n";
    for (fragment, reason) in [
        ("initial I\nstate A", "exactly one outgoing"),
        (
            "initial I\nstate A\nI --> A : start",
            "initial transitions must be unlabeled",
        ),
        ("initial I\nstate A\nA --> I\nI --> A", "no incoming"),
        ("state A\nfinal F\nF --> A", "final state cannot"),
        (
            "state A\nchoice C\nstate B\nA --> C\nC --> B : [ok]",
            "at least two",
        ),
        (
            "state A\nchoice C\nstate B\nA --> C\nC --> B : ok\nC --> A : [else]",
            "guard without a trigger",
        ),
        (
            "state A\nchoice C\nstate B\nA --> C\nC --> B : [ok]\nC --> A",
            "choice exits require a guard",
        ),
        ("state A\nstate B\nA --> B : go [broken", "closing"),
        ("state A\nstate B\nA --> B : go [ ]", "nonempty"),
        ("state A\nstate B\nA --> B : go /", "effect"),
        ("state A\nstate B\nA ..> B", "operator"),
        ("state A\nstate B\nA -- B", "operator"),
        ("initial I {\nstate A\n}", "pseudostates cannot contain"),
        (
            "state A {\nentry / one\nentry / two\n}",
            "duplicate state entry",
        ),
        ("state A {\nfield x: int\n}", "state body requires"),
        (
            "state Outer {\nstate Inner\n}\nstate Target\nOuter.Inner --> Target",
            "within one region",
        ),
        ("+state A", "cannot show classifier visibility"),
        ("state A <<artifact>>", "unchecked stereotypes"),
    ] {
        let error = parse(&base.replace("REPLACE", fragment)).unwrap_err();
        assert!(error.message.contains(reason), "{fragment}: {error:?}");
    }
}

#[test]
fn multiple_initials_in_one_region_and_duplicate_else_are_rejected() {
    for (body, reason) in [
        (
            "initial One\ninitial Two\nstate A\nOne --> A\nTwo --> A",
            "one initial per region",
        ),
        (
            "state A\nstate B\nchoice C\nA --> C\nC --> A : [else]\nC --> B : [else]",
            "only one [else]",
        ),
    ] {
        let source = format!("settings {{\ntype: state\n}}\nsemantics {{\n{body}\n}}\n");
        assert!(parse(&source).unwrap_err().message.contains(reason));
    }
}

#[test]
fn state_shape_and_initial_fill_cannot_be_styled_to_false_notation() {
    for (selector, property, reason) in [
        ("node[Idle]", "corner-radius: 0", "rounded corners"),
        ("node[Idle]", "shape: ellipse", "UML node form"),
        ("node[Start]", "fill: none", "visibly filled"),
        ("node[Check]", "shape: rectangle", "fixed UML primitive"),
        ("node[Idle]", "corner-radius: 90", "rounded corners"),
        ("node[Start]", "border-color: none", "visible outline"),
        ("node[Done]", "border-width: 0", "visible outline"),
    ] {
        let source = format!("{BASIC}\nstyling {{\n{selector} {{\n{property}\n}}\n}}\n");
        let diagram = parse(&source).unwrap();
        let sheet = load(&diagram, Path::new("examples/state-basic.laml")).unwrap();
        let err = place(&diagram, &sheet).unwrap_err();
        assert!(err.message.contains(reason), "{err:?}");
    }
}
