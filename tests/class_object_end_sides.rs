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
    let looped = SOURCE.replace("@r A --> B", "@r A --> A");
    assert!(parse(&looped).unwrap_err().message.contains("self-loop"));
}
