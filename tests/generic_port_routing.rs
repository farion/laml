use laml::{geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

const SOURCE: &str = include_str!("../examples/showcase-generic.laml");

#[test]
fn named_ports_exit_normally_and_nearby_generic_nodes_connect_horizontally() {
    let graph = parse(SOURCE).unwrap();
    let styles = load(&graph, Path::new("examples/showcase-generic.laml")).unwrap();
    let boxes = place(&graph, &styles).unwrap();
    let routes = route(&graph, &boxes, &styles).unwrap();
    let named = |name: &str| {
        let index = graph
            .relations
            .iter()
            .position(|relation| relation.name.as_deref() == Some(name))
            .unwrap();
        &routes[index].points
    };
    let receive = named("receive");
    let ingress = boxes
        .iter()
        .flat_map(|node| &node.ports)
        .find(|port| port.id == "input")
        .unwrap();
    let incoming = receive[receive.len() - 2];
    let at = receive[receive.len() - 1];
    assert_eq!((at.x, at.y), (ingress.x, ingress.y));
    assert!((at.y - incoming.y).abs() < 1e-6 && incoming.x <= at.x - 18.0);

    let dispatch = named("dispatch");
    let egress = boxes
        .iter()
        .flat_map(|node| &node.ports)
        .find(|port| port.id == "output")
        .unwrap();
    assert_eq!((dispatch[0].x, dispatch[0].y), (egress.x, egress.y));
    assert!((dispatch[0].y - dispatch[1].y).abs() < 1e-6);
    assert!(dispatch[1].x >= dispatch[0].x + 18.0);

    let classify = named("decide");
    assert_eq!(classify.len(), 2);
    let center_y = |id: &str| {
        let node = boxes.iter().find(|node| node.id == id).unwrap();
        node.y + node.height / 2.0
    };
    assert!((classify[0].y - center_y("Platform.Validator")).abs() < 1e-6);
    assert!((classify[1].y - center_y("Choice")).abs() < 1e-6);
    let enqueue = named("buffer");
    assert!((enqueue[0].y - center_y("Choice")).abs() < 1e-6);
    assert!((enqueue[1].y - center_y("Queue")).abs() < 1e-6);
}

#[test]
fn automatic_relation_alignment_yields_to_mandatory_vertical_layout() {
    let source = "semantics {\nA\nB\nA -> B\n}\nlayout {\nA above B : gap=60\n}\n";
    let graph = parse(source).unwrap();
    let boxes = place(
        &graph,
        &load(&graph, Path::new("examples/gne-basic.laml")).unwrap(),
    )
    .unwrap();
    let a = boxes.iter().find(|node| node.id == "A").unwrap();
    let b = boxes.iter().find(|node| node.id == "B").unwrap();
    assert!(a.y + a.height + 60.0 <= b.y + 1e-6);
}

#[test]
fn measured_headings_have_clearance_above_their_separators() {
    let graph = parse(SOURCE).unwrap();
    let styles = load(&graph, Path::new("examples/showcase-generic.laml")).unwrap();
    let boxes = place(&graph, &styles).unwrap();
    let routes = route(&graph, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    let text_y = |name: &str| {
        let suffix = format!(">{name}</text>");
        let line = svg.lines().find(|line| line.ends_with(&suffix)).unwrap();
        line.split(" y=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    let gateway = boxes.iter().find(|node| node.id == "gateway").unwrap();
    let separator = gateway.y
        + gateway.padding
        + gateway.top_band
        + gateway.font_size * 1.25 * gateway.lines.len() as f64;
    assert!(separator - text_y("Gateway") > gateway.font_size * 0.7);

    let platform = boxes.iter().find(|node| node.id == "Platform").unwrap();
    let tab_bottom = platform.y
        + platform.padding
        + platform.font_size * 1.25 * platform.lines.len() as f64
        + 4.0;
    assert!(tab_bottom - text_y("Platform") > platform.font_size * 0.7);
}
