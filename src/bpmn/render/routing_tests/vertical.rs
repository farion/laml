//! Regression checks for top-to-bottom BPMN pool and lane geometry.
use super::{EdgeKind, Kind, bpmn, plan, ports};
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
    }
}
