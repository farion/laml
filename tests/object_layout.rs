use laml::{geometry::place, parser::parse, route::route, styles::StyleSheet};

#[test]
fn object_showcase_uses_straight_free_lanes_without_crossing_the_visitor_link() {
    let diagram = parse(include_str!("../examples/showcase-object.laml")).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let node = |id: &str| boxes.iter().find(|box_node| box_node.id == id).unwrap();
    let link = |from: &str, to: &str| {
        let index = diagram
            .relations
            .iter()
            .position(|r| r.from == from && r.to == to)
            .unwrap();
        &routes[index].points
    };
    for (from, to) in [("customer", "cart"), ("cart", "itemB")] {
        let points = link(from, to);
        assert_eq!(points.len(), 2, "{from} -> {to}: {points:?}");
        assert!((points[0].y - points[1].y).abs() < 1e-6);
    }
    let vertical = link("visitor", "customer");
    assert_eq!(vertical.len(), 2, "{vertical:?}");
    assert!((vertical[0].x - vertical[1].x).abs() < 1e-6);
    let note = node("Snapshot");
    assert!(node("visitor").x + node("visitor").width < note.x);
    let attachment = routes.last().unwrap();
    assert!(attachment.points.iter().all(|p| p.x > vertical[0].x));
    let first = link("cart", "itemA");
    assert!(first.windows(2).all(|pair| {
        (pair[0].x - pair[1].x).abs() < 1e-6 || (pair[0].y - pair[1].y).abs() < 1e-6
    }));
}
