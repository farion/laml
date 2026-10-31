use laml::parser::parse;
use laml::syntax::{ActivityFlow, ActivityPinDirection, NodeKind};
use laml::{export, geometry::place, route::route, styles::load, svg::render};
use std::path::Path;

const ORDER: &str = include_str!("../examples/activity-order.laml");

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
