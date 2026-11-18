//! Self-loops must not disable recovery from occupied connection corridors.
use laml::{parser, route, styles};
use std::path::Path;

#[test]
fn crowded_self_loop_retries_allocation_without_changing_source_or_sides() {
    let source = include_str!("fixtures/gne-self-loop-routing-order.laml");
    let diagram = parser::parse(source).unwrap();
    let sheet = styles::load(&diagram, Path::new("self-loop-order.laml")).unwrap();
    let (boxes, routes) = route::place_and_route(&diagram, &sheet).unwrap();
    assert_eq!(routes.len(), diagram.relations.len());
    for (relation, routed) in diagram.relations.iter().zip(&routes) {
        assert_eq!(
            routed.source_line, relation.line,
            "retain declaration order"
        );
        assert_eq!(boxes[routed.from].id, relation.from);
        assert_eq!(boxes[routed.to].id, relation.to);
    }
    let index = diagram
        .relations
        .iter()
        .position(|r| r.name.as_deref() == Some("Relation1"))
        .unwrap();
    let routed = &routes[index];
    let api = &boxes[routed.from];
    let p = &routed.points;
    assert_eq!(routed.from, routed.to);
    assert!((p[0].y - api.y).abs() < 1e-6, "mandatory top source");
    assert!(p[1].y < p[0].y);
    let tip = p[p.len() - 1];
    assert!(
        (tip.x - api.x - api.width).abs() < 1e-6,
        "mandatory right target"
    );
    assert!(p[p.len() - 2].x > tip.x);
}
