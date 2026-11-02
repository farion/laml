//! Integration checks for the BPMN routing contract using real example geometry.
use super::{Geometry, edges, ports};
use crate::bpmn::{self, Document, EdgeKind, Kind};
use crate::svg::TextMode;

fn setup() -> (Document, Geometry) {
    let doc = bpmn::parse(include_str!("../../../examples/bpmn-order.laml")).unwrap();
    let geom = super::super::geometry::place(&doc).unwrap();
    (doc, geom)
}
#[test]
fn all_order_edges_have_unique_node_anchors_and_clear_paths() {
    let (doc, geom) = setup();
    let ports = ports::assign(&doc, &geom).unwrap();
    for node in &doc.nodes {
        let mut anchors = Vec::new();
        for (i, e) in doc.edges.iter().enumerate() {
            if e.from == node.id {
                let p = ports.ends[i].0.unwrap();
                anchors.push((p.x, p.y));
            }
            if e.to == node.id {
                let p = ports.ends[i].1.unwrap();
                anchors.push((p.x, p.y));
            }
        }
        for (i, p) in anchors.iter().enumerate() {
            for q in anchors.iter().skip(i + 1) {
                assert!(
                    (p.0 - q.0).abs() + (p.1 - q.1).abs() > 2.0,
                    "shared attachment at {}",
                    node.id
                );
            }
        }
    }
    let mut previous = Vec::new();
    for (i, e) in doc.edges.iter().enumerate() {
        edges::edge(&mut String::new(), &doc, &geom, e, i, &ports, &mut previous).unwrap();
        let points = &previous.last().unwrap().points;
        let start = ports.ends[i].0.unwrap();
        let end = ports.ends[i].1.unwrap();
        assert!(
            (points[0].0 - start.x).abs() < 0.08 && (points[0].1 - start.y).abs() < 0.08,
            "{} source",
            e.from
        );
        assert!(
            (points.last().unwrap().0 - end.x).abs() < 0.08
                && (points.last().unwrap().1 - end.y).abs() < 0.08
        );
        if e.from == "Shop.Deadline" {
            assert_eq!(
                doc.nodes.iter().find(|n| n.id == e.from).unwrap().kind,
                Kind::Boundary
            );
            assert!(
                (points[0].0 - points[1].0).abs() < 0.01,
                "timer must exit normal to circle"
            );
            assert!(
                (points[points.len() - 1].0 - points[points.len() - 2].0).abs() < 0.01,
                "timer flow must enter task orthogonally"
            );
        }
        if e.kind == EdgeKind::Association && e.from == "Shop.Policy" {
            let index = doc.nodes.iter().position(|n| n.id == e.from).unwrap();
            assert!(
                (points[0].0 - (geom.nodes[index].x - geom.nodes[index].w / 2.0)).abs() < 0.01,
                "note connector must meet the visible bracket"
            );
        }
        if e.kind == EdgeKind::Message && e.label.as_deref() == Some("answer") {
            let customer = &geom.pools[0];
            let shop = &geom.pools[1];
            assert!(
                points
                    .windows(2)
                    .any(|leg| (leg[0].1 - leg[1].1).abs() < 0.01
                        && leg[0].1 > customer.y + customer.h
                        && leg[0].1 < shop.y),
                "answer message must travel in inter-pool gap, not through gateway or pool side"
            );
        }
    }
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    assert!(svg.contains(" Q "), "crossings need bridge humps");
    let order = svg
        .lines()
        .find(|line| line.ends_with(">order</text>"))
        .unwrap();
    let label_y = order
        .split("y=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    let p = &geom.pools[0];
    assert!(
        label_y < p.y + p.h - 25.0 && label_y > p.y + 30.0 + 20.0,
        "order label must clear pool border"
    );
}
