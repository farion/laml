use laml::parser::parse;
use laml::syntax::{ActivityFlow, ActivityPinDirection, NodeKind};
use laml::{export, geometry::place, route::route, styles::load, svg::render};
use std::path::Path;

const ORDER: &str = include_str!("../examples/activity-order.laml");
const BASIC: &str = include_str!("../examples/activity-basic.laml");

#[test]
fn basic_activity_places_control_spine_and_input_object_on_separate_rows() {
    let diagram = parse(BASIC).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-basic.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let get = |id| boxes.iter().find(|node| node.id == id).unwrap();
    let (start, request, review, done) = (get("Start"), get("Request"), get("Review"), get("Done"));
    assert!(start.x + start.width < review.x);
    assert!(review.x + review.width < done.x);
    assert!((request.x + request.width / 2.0 - review.x - review.width / 2.0).abs() < 0.01);
    assert!(request.y >= review.y + review.height + 20.0);
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let get_route = |from: &str, to: &str| {
        &routes[diagram
            .relations
            .iter()
            .position(|r| r.from == from && r.to == to)
            .unwrap()]
    };
    let enter = get_route("Start", "Review");
    let leave = get_route("Review", "Done");
    assert_eq!(
        enter.points.len(),
        2,
        "initial control should enter directly from the left"
    );
    assert_eq!(
        leave.points.len(),
        2,
        "final control should leave directly to the right"
    );
    assert!((enter.points[0].y - enter.points[1].y).abs() < 0.01);
    assert!((enter.points[1].x - review.x).abs() < 0.01);
    assert!((leave.points[0].y - leave.points[1].y).abs() < 0.01);
    let object = get_route("Request", "Review::request");
    assert_eq!(object.points.len(), 4);
    assert!(object.points[1].x < review.x - 12.0);
    assert!((object.points[2].y - object.points[3].y).abs() < 0.01);
}

fn source(body: &str) -> String {
    format!("settings {{\ntype: activity\n}}\nsemantics {{\n{body}\n}}\n")
}

#[test]
fn explicit_control_and_object_flows_have_distinct_pin_endpoints() {
    let diagram = parse(ORDER).unwrap();
    let inspect = diagram
        .nodes
        .iter()
        .find(|node| node.id == "Inspect")
        .unwrap();
    assert_eq!(inspect.kind, NodeKind::ActivityAction);
    assert_eq!(
        inspect.ports[0].activity_pin,
        Some(ActivityPinDirection::Input)
    );
    assert!(
        diagram
            .relations
            .iter()
            .any(
                |relation| relation.activity_flow == Some(ActivityFlow::Object)
                    && relation.from == "Order"
                    && relation.to == "Inspect::order"
            )
    );
    assert!(
        diagram
            .relations
            .iter()
            .any(
                |relation| relation.activity_flow == Some(ActivityFlow::Control)
                    && relation.from == "Together"
                    && relation.to == "Dispatch"
            )
    );
    assert!(
        diagram
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ActivityFlowFinal)
    );
    assert!(
        diagram
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ActivityFinal)
    );
}

#[test]
fn order_example_renders_distinct_pins_and_control_symbols_in_all_formats() {
    let diagram = parse(ORDER).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    for id in [
        "Inspect::order",
        "Reserve::reservation",
        "Authorize::approval",
        "Dispatch::reservation",
        "Dispatch::approval",
    ] {
        assert!(
            svg.contains(&format!("data-port-id=\"{id}\"")),
            "missing visible pin {id}"
        );
    }
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("[invalid]"));
    assert!(svg.contains("[else]"));
    assert!(svg.contains("<path d=\"M")); // circled-X flow final
    assert!(export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn parallel_activity_places_objects_in_lanes_without_diagonal_or_crossing_flows() {
    let diagram = parse(ORDER).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let node = |id| boxes.iter().find(|n| n.id == id).unwrap();
    let (reserve, authorize, reservation, approval, join, dispatch) = (
        node("Reserve"),
        node("Authorize"),
        node("Reservation"),
        node("Approval"),
        node("Together"),
        node("Dispatch"),
    );
    assert!(
        (reservation.x + reservation.width / 2.0 - approval.x - approval.width / 2.0).abs() < 0.01
    );
    assert!(reserve.y < authorize.y);
    let fork = node("Parallel");
    assert_eq!(fork.width, 12.0);
    assert_eq!(fork.height, 170.0);
    assert!(reservation.x + reservation.width + 32.0 < join.x);
    assert!(join.x + join.width < dispatch.x);
    let pin_y =
        |n: &laml::geometry::BoxNode, name: &str| n.ports.iter().find(|p| p.id == name).unwrap().y;
    assert!(
        (reservation.y + reservation.height / 2.0 - pin_y(reserve, "Reserve::reservation")).abs()
            < 0.01
    );
    assert!(
        (approval.y + approval.height / 2.0 - pin_y(authorize, "Authorize::approval")).abs() < 0.01
    );
    assert!(
        (pin_y(dispatch, "Dispatch::reservation") - reservation.y - reservation.height / 2.0).abs()
            < 0.01
    );
    assert!(
        (pin_y(dispatch, "Dispatch::approval") - approval.y - approval.height / 2.0).abs() < 0.01
    );
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let flow = |from: &str, to: &str| {
        &routes[diagram
            .relations
            .iter()
            .position(|r| r.from == from && r.to == to)
            .unwrap()]
    };
    for (from, to) in [
        ("Reserve::reservation", "Reservation"),
        ("Reservation", "Dispatch::reservation"),
        ("Authorize::approval", "Approval"),
        ("Approval", "Dispatch::approval"),
        ("Parallel", "Reserve"),
        ("Parallel", "Authorize"),
        ("Together", "Dispatch"),
        ("Inspect", "Valid"),
    ] {
        let points = &flow(from, to).points;
        assert_eq!(points.len(), 2, "{from} -> {to} must be direct");
        assert!(
            (points[0].y - points[1].y).abs() < 0.01,
            "{from} -> {to} must be horizontal"
        );
    }
    for (from, to) in [
        ("Valid", "Parallel"),
        ("Reserve", "Together"),
        ("Authorize", "Together"),
    ] {
        let points = &flow(from, to).points;
        assert!(points.len() >= 3, "{from} -> {to}");
        assert!(
            points
                .windows(2)
                .all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01),
            "{from} -> {to} must be orthogonal"
        );
    }
    let (decision, reject) = (node("Valid"), node("Reject"));
    let invalid = &flow("Valid", "Reject").points;
    assert_eq!(
        invalid.len(),
        3,
        "top exit needs only one elbow before Reject"
    );
    assert!((invalid[0].x - decision.x - decision.width / 2.0).abs() < 0.01);
    assert!(
        (invalid[0].y - decision.y).abs() < 0.01,
        "invalid exits at the diamond top"
    );
    assert!((invalid[2].x - reject.x).abs() < 0.01);
    assert!(
        (invalid[2].y - reject.y - reject.height / 2.0).abs() < 0.01,
        "invalid enters the middle of Reject's left side"
    );
    assert!(
        invalid
            .windows(2)
            .all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01)
    );
    let else_path = &flow("Valid", "Parallel").points;
    assert!(
        (else_path[0].x - decision.x - decision.width / 2.0).abs() < 0.01,
        "else starts at the diamond's bottom centre"
    );
    assert!((else_path[0].y - decision.y - decision.height).abs() < 0.01);
    assert!(
        routes.iter().all(|r| r.bridges.is_empty()),
        "these parallel lanes need no crossings"
    );
}

#[test]
fn authored_multi_input_action_height_is_not_overridden_by_auto_layout() {
    let source = ORDER.replace(
        "styling {",
        "styling {\n    node[Dispatch] {\n        height: 100\n    }",
    );
    let diagram = parse(&source).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert_eq!(
        boxes.iter().find(|n| n.id == "Dispatch").unwrap().height,
        100.0
    );
}

#[test]
fn merge_and_direct_pin_to_pin_flow_have_explicit_roles() {
    let merge = source(
        "initial Start\ndecision Branch\naction First\naction Second\nmerge Combine\nactivityFinal Done\ncontrol Start --> Branch\ncontrol Branch --> First : [ok]\ncontrol Branch --> Second : [else]\ncontrol First --> Combine\ncontrol Second --> Combine\ncontrol Combine --> Done",
    );
    assert!(
        parse(&merge)
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::ActivityMerge)
    );
    let direct = source(
        "action Produce {\noutput item\n}\naction Consume {\ninput item\n}\nobject Produce::item --> Consume::item",
    );
    assert!(
        parse(&direct)
            .unwrap()
            .relations
            .iter()
            .any(|relation| relation.activity_flow == Some(ActivityFlow::Object))
    );
}

#[test]
fn simple_activity_renders_and_exports() {
    let diagram = parse(&source("initial Start\naction Inspect\nactivityFinal Done\ncontrol Start --> Inspect\ncontrol Inspect --> Done")).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("r=\"9\""));
    assert!(svg.contains("r=\"12\""));
    assert!(svg.contains("rx=\"14.00\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn authored_bar_length_follows_flow_orientation_and_preserves_join_size() {
    for (direction, expected_fork, expected_join) in [
        ("", (12.0, 170.0), (12.0, 110.0)),
        ("direction: top-to-bottom", (170.0, 12.0), (110.0, 12.0)),
    ] {
        let text = ORDER.replacen("type: activity", &format!("type: activity\n{direction}"), 1)
            .replace("node[Parallel] {\n        length: 170\n    }", "node[Parallel] {\n        length: 170\n    }\n    node[Together] {\n        length: 110\n    }");
        let diagram = parse(&text).unwrap();
        let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
        let boxes = place(&diagram, &sheet).unwrap();
        let fork = boxes.iter().find(|n| n.id == "Parallel").unwrap();
        let join = boxes.iter().find(|n| n.id == "Together").unwrap();
        assert_eq!((fork.width, fork.height), expected_fork);
        assert_eq!((join.width, join.height), expected_join);
    }
}

#[test]
fn bar_length_does_not_resize_other_uml_control_symbols() {
    let too_short = ORDER.replace("length: 170", "length: 12");
    let diagram = parse(&too_short).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("at least 24")
    );

    let invalid = ORDER.replace("node[Parallel]", "node[Valid]");
    let diagram = parse(&invalid).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("only valid for activity fork/join")
    );

    let invalid = ORDER.replace("node[Parallel]", "node[Reserve]");
    let diagram = parse(&invalid).unwrap();
    let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("only valid for activity fork/join")
    );
}

#[test]
fn fork_and_join_bars_are_perpendicular_to_the_diagram_flow() {
    let body = "initial Start\naction A\nfork Split\naction Left\naction Right\njoin Together\nactivityFinal Done\ncontrol Start --> A\ncontrol A --> Split\ncontrol Split --> Left\ncontrol Split --> Right\ncontrol Left --> Together\ncontrol Right --> Together\ncontrol Together --> Done";
    for (setting, vertical) in [("", true), ("direction: top-to-bottom", false)] {
        let text =
            source(body).replacen("type: activity", &format!("type: activity\n{setting}"), 1);
        let diagram = parse(&text).unwrap();
        let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
        let boxes = place(&diagram, &sheet).unwrap();
        for node in boxes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::ActivityFork | NodeKind::ActivityJoin))
        {
            assert_eq!(node.height > node.width, vertical, "{}", node.id);
            if !vertical {
                assert_eq!(node.height, 12.0, "horizontal UML bars stay thin");
            }
        }
    }
}

#[test]
fn reject_ambiguous_or_wrong_direction_flows() {
    for (body, message) in [
        ("action A\naction B\nA --> B", "explicit control or object"),
        ("action A\naction B\ncontrol A ..> B", "operator"),
        (
            "action A {\ninput item\n}\naction B\nobject A::item --> B",
            "output pin",
        ),
        (
            "objectNode Item\naction A\ncontrol Item --> A",
            "control vertices",
        ),
        (
            "action A {\noutput item\n}\naction B\nobject A::item --> B",
            "input pin",
        ),
        (
            "action A {\ninput item\ninput item\n}",
            "unique simple name",
        ),
        ("action A {\nport item\n}", "input Name or output Name"),
        ("fork F {\ninput item\n}", "only actions"),
        ("state Invalid", "not supported"),
        (
            "action A\nnote Hint \"x\"\ncontrol A .. Hint",
            "cannot be a control or object flow",
        ),
    ] {
        let err = parse(&source(body)).unwrap_err();
        assert!(err.message.contains(message), "{body}: {err:?}");
    }
}

#[test]
fn activity_pin_sides_are_not_arbitrary_layout_choices() {
    let text = format!(
        "{}\nlayout {{\nport[A::item] : side=right\n}}",
        source("action A {\ninput item\n}")
    );
    assert!(
        parse(&text)
            .unwrap_err()
            .message
            .contains("input pins stay left")
    );
}

#[test]
fn reject_unchecked_control_topologies_and_bad_guards() {
    for body in [
        "initial Start\naction A",
        "initial Start\naction A\ncontrol A --> Start\ncontrol Start --> A",
        "action A\nactivityFinal End\ncontrol End --> A",
        "action A\ndecision D\naction B\ncontrol A --> D\ncontrol D --> B : [yes]",
        "action A\ndecision D\naction B\ncontrol A --> D\ncontrol D --> B : yes\ncontrol D --> A : [else]",
        "action A\ndecision D\naction B\ncontrol A --> D\ncontrol D --> B : [else]\ncontrol D --> A : [else]",
        "action A\nfork F\naction B\ncontrol A --> F\ncontrol F --> B",
        "action A\njoin J\naction B\ncontrol A --> J\ncontrol J --> B",
        "action A\nmerge M\naction B\ncontrol A --> M\ncontrol M --> B",
    ] {
        assert!(parse(&source(body)).is_err(), "unexpectedly valid: {body}");
    }
}

#[test]
fn activity_shapes_and_connected_pins_cannot_be_styled_away() {
    let body = "initial Start\nobjectNode Order\naction Inspect {\ninput order\n}\nactivityFinal Done\nobject Order --> Inspect::order\ncontrol Start --> Inspect\ncontrol Inspect --> Done";
    for (selector, property, reason) in [
        ("node[Inspect]", "corner-radius: 0", "rounded corners"),
        ("node[Order]", "corner-radius: 20", "rectangular"),
        ("node[Start]", "fill: none", "visibly filled"),
        (
            "port[Inspect::order]",
            "display: none",
            "must remain visible",
        ),
        ("relation", "display: none", "flows must remain visible"),
    ] {
        let text = format!(
            "{}\nstyling {{\n{selector} {{\n{property}\n}}\n}}",
            source(body)
        );
        let diagram = parse(&text).unwrap();
        let sheet = load(&diagram, Path::new("examples/activity-order.laml")).unwrap();
        let err = place(&diagram, &sheet).unwrap_err();
        assert!(err.message.contains(reason), "{selector}: {err:?}");
    }
}

#[test]
fn note_links_do_not_count_as_control_edges() {
    let body =
        "initial Start\naction A\ncontrol Start --> A\nnote Hint \"annotation\"\nHint .. Start";
    assert!(parse(&source(body)).is_ok());
}
