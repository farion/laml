//! Regression checks for top-to-bottom BPMN pool and lane geometry.
use super::{EdgeKind, Kind, bpmn, plan, ports};
use crate::svg::TextMode;
fn attr(line: &str, key: &str) -> f64 {
    line.split(&format!("{key}=\""))
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
#[test]
fn vertical_pool_lanes_fit_and_center_their_nodes() {
    for source in [
        include_str!("../../../../examples/bpmn-vertical.laml").to_string(),
        include_str!("../../../../examples/bpmn-vertical.laml")
            .replace("Check order", "Check order and examine attachments"),
    ] {
        let doc = bpmn::parse(&source).unwrap();
        let geom = crate::bpmn::geometry::place(&doc).unwrap();
        let shop = geom.pools.iter().find(|p| p.label == "Shop").unwrap();
        for band in &shop.bands {
            let members: Vec<_> = doc
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    n.pool.as_deref() == Some("Shop")
                        && n.lane == band.id
                        && n.parent.is_none()
                        && n.kind != Kind::Boundary
                })
                .map(|(i, _)| geom.nodes[i])
                .collect();
            assert!(!members.is_empty());
            let left = band.y + 30.0;
            let right = band.y + band.h;
            let min = members
                .iter()
                .map(|n| n.x - n.w / 2.0)
                .fold(f64::INFINITY, f64::min);
            let max = members
                .iter()
                .map(|n| n.x + n.w / 2.0)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(
                min >= left + 20.0 && max <= right - 20.0,
                "nodes must stay inside the usable lane width"
            );
            assert!(
                ((min + max) / 2.0 - (left + right) / 2.0).abs() < 0.1,
                "lane nodes should be centered inside the area after its header"
            );
        }
        let customer = geom.pools.iter().find(|p| p.label == "Customer").unwrap();
        let customer_nodes: Vec<_> = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.pool.as_deref() == Some("Customer") && n.parent.is_none())
            .map(|(i, _)| geom.nodes[i])
            .collect();
        let min = customer_nodes
            .iter()
            .map(|n| n.x - n.w / 2.0)
            .fold(f64::INFINITY, f64::min);
        let max = customer_nodes
            .iter()
            .map(|n| n.x + n.w / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (min - customer.x - 24.0).abs() < 0.1
                && (customer.x + customer.w - max - 24.0).abs() < 0.1,
            "an unlaned vertical pool needs no phantom lane header at its left"
        );
        let ports = ports::assign(&doc, &geom).unwrap();
        let (_, routes) = plan::all(&doc, &geom, &ports).unwrap();
        let i = doc
            .edges
            .iter()
            .position(|e| e.kind == EdgeKind::Message)
            .unwrap();
        assert!(
            routes[i]
                .points
                .windows(2)
                .any(|leg| (leg[0].0 - leg[1].0).abs() < 0.01
                    && (leg[0].1 - leg[1].1).abs() > 80.0
                    && leg[0].0 > customer.x + customer.w + 15.0
                    && leg[0].0 < shop.x - 15.0),
            "message should use the inter-pool aisle, not the pool or lane border"
        );
        let svg = bpmn::render(&doc, TextMode::System).unwrap();
        let caption = svg
            .lines()
            .find(|l| l.ends_with(">Order received</text>"))
            .unwrap();
        let received = geom.nodes[doc
            .nodes
            .iter()
            .position(|n| n.id == "Shop.Received")
            .unwrap()];
        assert!(
            attr(caption, "y") < received.y - received.h / 2.0 - 5.0,
            "message start caption must sit above, not across the outgoing sequence flow"
        );
        let label = svg.lines().find(|l| l.ends_with(">order</text>")).unwrap();
        assert!(
            !label.contains("rotate("),
            "the message label fits horizontally in the aisle"
        );
        let half = crate::text::measure("order", super::super::super::type_sizes::FLOW).unwrap()
            / 2.0
            + 5.0;
        assert!(
            attr(label, "x") - half > customer.x + customer.w + 18.0
                && attr(label, "x") + half < shop.x - 18.0,
            "message caption must have room on both sides of the inter-pool aisle"
        );
    }
}
