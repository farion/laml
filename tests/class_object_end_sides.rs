use laml::{geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

const SOURCE: &str = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\n@r A --> B\n}\nlayout {\nA < B : gap=150\nrelation[r].source : side=top\nrelation[r].target : side=bottom\n}\n";

#[test]
fn named_relation_ends_obey_mandatory_boundary_sides() {
    let diagram = parse(SOURCE).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let points = &routes[0].points;
    assert!((points[0].y - boxes[0].y).abs() < 1e-6);
    assert!((points.last().unwrap().y - boxes[1].y - boxes[1].height).abs() < 1e-6);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("url(#arrow)"));
    assert!(laml::export::png(&svg).unwrap().starts_with(b"\x89PNG"));
    assert!(laml::export::pdf(&svg).unwrap().starts_with(b"%PDF-"));
}

#[test]
fn relation_side_rules_reject_invalid_and_indirect_endpoints() {
    let invalid = SOURCE.replace("side=top", "side=above");
    assert!(parse(&invalid).unwrap_err().message.contains("end side"));
    let repeated = SOURCE.replace("side=top", "side=top; side=left");
    assert!(parse(&repeated).unwrap_err().message.contains("duplicate"));
    let indirect = SOURCE
        .replace("settings {\ntype: class\n}\n", "")
        .replace("class A", "class A {\n@p port out\n}")
        .replace("@r A --> B", "@r p --> B");
    assert!(
        parse(&indirect)
            .unwrap_err()
            .message
            .contains("direct node endpoint")
    );
}

#[test]
fn self_loop_sides_route_around_the_exterior_for_each_side_pair() {
    let sheet = StyleSheet::default();
    for source_side in ["left", "right", "top", "bottom"] {
        for target_side in ["left", "right", "top", "bottom"] {
            let text = format!(
                "settings {{\ntype: class\n}}\nsemantics {{\nclass A\n@loop A -- A\n}}\nlayout {{\nrelation[loop].source : side={source_side}\nrelation[loop].target : side={target_side}\n}}\n"
            );
            let diagram = parse(&text).unwrap();
            let boxes = place(&diagram, &sheet).unwrap();
            let routes = route(&diagram, &boxes, &sheet)
                .unwrap_or_else(|error| panic!("{source_side}->{target_side}: {}", error.message));
            let node = &boxes[0];
            let side = |p: laml::route::Point| {
                if (p.x - node.x).abs() < 1e-6 {
                    "left"
                } else if (p.x - node.x - node.width).abs() < 1e-6 {
                    "right"
                } else if (p.y - node.y).abs() < 1e-6 {
                    "top"
                } else {
                    "bottom"
                }
            };
            assert_eq!(side(routes[0].points[0]), source_side);
            assert_eq!(side(*routes[0].points.last().unwrap()), target_side);
            assert!(routes[0].points.len() >= 4);
        }
    }
}
