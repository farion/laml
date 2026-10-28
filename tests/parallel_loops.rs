use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::StyleSheet;

#[test]
fn parallel_relations_and_self_loop_have_distinct_paths() {
    let diagram = parse(include_str!("../examples/parallel-loops.laml")).unwrap();
    let sheet = laml::styles::load(
        &diagram,
        std::path::Path::new("examples/parallel-loops.laml"),
    )
    .unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routed = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routed.len(), 4);
    assert_ne!(routed[0].points[0].y, routed[1].points[0].y);
    assert_ne!(routed[1].points[0].y, routed[2].points.last().unwrap().y);
    assert_eq!(routed[3].from, routed[3].to);
    assert!(routed[3].points.len() >= 5);
    // The return relation detours above the boxes without diagonal stubs
    // from its offset boundary ports to the horizontal lane.
    assert!(routed[2].points.len() >= 4);
    let service = &boxes[0];
    let broker = &boxes[1];
    assert!(
        (routed[2].points[0].x - (broker.x + broker.width / 2.0)).abs() < 1e-6,
        "the return arrow has sole use of Broker's top face"
    );
    assert!(
        routed[2].points.last().unwrap().x > service.x + service.width / 2.0,
        "Service reserves separate top ports for the return arrow and self-loop"
    );
    assert!(
        routed[2].points.windows(2).all(
            |pair| (pair[0].x - pair[1].x).abs() < 1e-6 || (pair[0].y - pair[1].y).abs() < 1e-6
        )
    );
}

#[test]
fn appending_a_parallel_relation_does_not_move_nodes() {
    let before = parse("semantics {\nA\nB\nA -> B\n}\n").unwrap();
    let after = parse("semantics {\nA\nB\nA -> B\nA -> B\n}\n").unwrap();
    let sheet = StyleSheet::default();
    let first = place(&before, &sheet).unwrap();
    let second = place(&after, &sheet).unwrap();
    for index in 0..2 {
        assert_eq!(
            (first[index].x, first[index].y),
            (second[index].x, second[index].y)
        );
    }
    let routes = route(&after, &second, &sheet).unwrap();
    assert_ne!(routes[0].points, routes[1].points);
}

#[test]
fn overfull_parallel_face_fails_instead_of_overdrawing() {
    let mut source = "semantics {\nA\nB\n".to_string();
    source.push_str(&"A -> B\n".repeat(7));
    source.push_str("}\n");
    let diagram = parse(&source).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let error = route(&diagram, &boxes, &sheet).unwrap_err();
    assert!(
        error
            .message
            .contains("parallel relations cannot fit distinct ports")
    );
}

#[test]
fn self_loop_uses_another_quadrant_when_preferred_one_is_blocked() {
    let diagram =
        parse("semantics {\nBlocker\nA\nA -> A\n}\nlayout {\nBlocker < A : align=top; gap=24\n}\n")
            .unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routed = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routed.len(), 1);
    // Both left quadrants are blocked by Blocker; route outside the right face.
    assert!(routed[0].points[0].x >= boxes[1].x + boxes[1].width - 1e-6);
}

#[test]
fn fifth_self_relation_fails_with_explicit_limit() {
    let mut source = "semantics {\nA\n".to_string();
    source.push_str(&"A -> A\n".repeat(5));
    source.push_str("}\n");
    let diagram = parse(&source).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let error = route(&diagram, &boxes, &sheet).unwrap_err();
    assert!(error.message.contains("more than four self-relations"));
}
