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
    let mut previous = Vec::new();
    for (i, e) in doc.edges.iter().enumerate() {
        edges::edge(&mut String::new(), &doc, &geom, e, i, &ports, &mut previous).unwrap();
        let points = &previous.last().unwrap().points;
        if points.len() > 2 {
            for (segment, leg) in points.windows(2).enumerate() {
                let id = if segment == 0 { &e.from } else { &e.to };
                let bracket = (segment == 0 || segment == points.len() - 2)
                    && doc
                        .nodes
                        .iter()
                        .any(|n| &n.id == id && matches!(n.kind, Kind::Data | Kind::Annotation));
                let min = if bracket { 12.0 } else { 20.0 };
                assert!(
                    (leg[1].0 - leg[0].0).abs() + (leg[1].1 - leg[0].1).abs() >= min - 0.1,
                    "short leg on {} -> {}",
                    e.from,
                    e.to
                );
            }
        }
        for (out, point) in [(true, points[0]), (false, *points.last().unwrap())] {
            assert!(
                ports::alternatives::for_end(&doc, &geom, &ports, i, out)
                    .iter()
                    .any(|p| (point.0 - p.x).abs() < 0.08 && (point.1 - p.y).abs() < 0.08),
                "{} -> {} must dock on an available port",
                e.from,
                e.to
            );
        }
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
    for node in &doc.nodes {
        let mut anchors = Vec::new();
        for (i, e) in doc.edges.iter().enumerate() {
            if e.from == node.id {
                anchors.push(previous[i].points[0]);
            }
            if e.to == node.id {
                anchors.push(*previous[i].points.last().unwrap());
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
    let edge = |from: &str, to: &str| {
        doc.edges
            .iter()
            .position(|e| e.from == from && e.to == to)
            .unwrap()
    };
    assert_eq!(
        previous[edge("Shop.Received", "Shop.Review")].points.len(),
        2,
        "arrival should connect straight into its primary task port"
    );
    let timer = &previous[edge("Shop.Deadline", "Shop.Escalate")].points;
    assert_eq!(
        timer.len(),
        2,
        "boundary timer must enter Escalate without a dogleg"
    );
    assert!((timer[0].0 - timer[1].0).abs() < 0.01);
    assert_eq!(
        previous[edge("Shop.Decision", "Shop.Decline")].points.len(),
        3,
        "the no branch needs only one bend when its aisle is clear"
    );
    assert_eq!(
        previous[edge("Shop.Pack", "Shop.Merge")].points.len(),
        3,
        "pack should need only one bend into the centered join"
    );
    let escalation = &previous[edge("Shop.Escalate", "Shop.Merge")].points;
    assert_eq!(
        escalation.len(),
        3,
        "choose the task's bottom port instead of a short right-hand dogleg"
    );
    assert!((escalation[0].0 - escalation[1].0).abs() < 0.01);
    let archive = &previous[edge("Shop.Review", "Shop.Archive")].points;
    let store = geom.nodes[doc
        .nodes
        .iter()
        .position(|n| n.id == "Shop.Archive")
        .unwrap()];
    assert!(
        (archive.last().unwrap().0 - (store.x + 31.0)).abs() < 0.01
            && (archive.last().unwrap().1 - store.y).abs() < 0.01,
        "enter the store from its free right side instead of adding a tiny terminal kink"
    );
    let node = |id: &str| geom.nodes[doc.nodes.iter().position(|n| n.id == id).unwrap()];
    assert_eq!(
        node("Shop.Merge").y,
        (node("Shop.Pack").y + node("Shop.Decline").y) / 2.0
    );
    assert!(
        node("Shop.Escalate").x + node("Shop.Escalate").w / 2.0 + 8.0 < node("Shop.Decision").x
    );
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    assert!(svg.contains(" Q "), "crossings need bridge humps");
    for pool in &geom.pools {
        assert!(
            svg.contains(&format!(
                "M {:.1} {:.1} H {:.1}",
                pool.x,
                pool.y + 30.0,
                pool.x + pool.w
            )),
            "all pools need an identical 30px title divider"
        );
    }
    assert_eq!(
        svg.lines()
            .filter(|line| line.contains("width=\"30\"") && line.contains("fill=\"#f8fbff\""))
            .count(),
        2,
        "both Shop lane title strips must use the pool background color"
    );
    let shop = &geom.pools[1];
    let last = shop.bands.last().unwrap();
    assert!(
        svg.contains(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"30\" height=\"{:.1}\" fill=\"#f8fbff\"",
            shop.x,
            last.y,
            shop.y + shop.h - last.y
        )),
        "last lane title must extend all the way to the pool's bottom border"
    );
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

#[test]
fn vertical_lane_titles_reach_the_right_pool_border() {
    let doc = bpmn::parse(include_str!("../../../examples/bpmn-vertical.laml")).unwrap();
    let geom = super::super::geometry::place(&doc).unwrap();
    let svg = bpmn::render(&doc, TextMode::System).unwrap();
    let shop = geom.pools.iter().find(|p| p.label == "Shop").unwrap();
    let last = shop.bands.last().unwrap();
    assert!(
        svg.contains(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"30\" fill=\"#f8fbff\"",
            last.y,
            shop.y + 30.0,
            shop.x + shop.w - last.y
        )),
        "final horizontal lane title must reach the pool's right border"
    );
    assert!(svg.contains(&format!(
        "M {:.1} {:.1} H {:.1}",
        last.y,
        shop.y + 60.0,
        shop.x + shop.w
    )));
    for band in &shop.bands {
        assert!(
            svg.contains(&format!(
                "M {:.1} {:.1} V {:.1}",
                band.y,
                shop.y + 30.0,
                shop.y + shop.h
            )),
            "lane dividers must not cut through the pool title"
        );
    }
}
