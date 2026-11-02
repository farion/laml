use laml::{geometry::place, parser::parse, route::route, styles::load, svg::render};
use std::path::Path;

fn draw(
    source: &str,
) -> (
    Vec<laml::geometry::BoxNode>,
    Vec<laml::route::Route>,
    String,
) {
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/class-basic.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    (boxes, routes, svg)
}
fn box_for<'a>(boxes: &'a [laml::geometry::BoxNode], id: &str) -> &'a laml::geometry::BoxNode {
    boxes.iter().find(|b| b.id == id).unwrap()
}
#[test]
fn compact_interfaces_align_with_their_providers_and_crop_empty_top() {
    let source = include_str!("../examples/class-lollipops.laml");
    let (boxes, routes, svg) = draw(source);
    assert_eq!(routes.len(), 2);
    for route in &routes {
        let provider = &boxes[route.from];
        let symbol = &boxes[route.to];
        assert!((provider.y + provider.height / 2.0 - symbol.y - symbol.height / 2.0).abs() < 0.01);
        assert!(
            route
                .points
                .windows(2)
                .all(|leg| (leg[0].y - leg[1].y).abs() < 0.01)
        );
    }
    let min_y = boxes.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let top: f64 = svg
        .split("viewBox=\"")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (min_y - top - 32.0).abs() < 0.01,
        "canvas must not preserve an empty origin above the drawing"
    );
}
#[test]
fn nested_package_connection_is_axial_unless_layout_says_otherwise() {
    let source = include_str!("../examples/class-packages.laml");
    let (boxes, routes, _) = draw(source);
    let customer = box_for(&boxes, "Vertrieb.Kunde");
    let product = box_for(&boxes, "Vertrieb.Lager.Produkt");
    assert!((customer.x + customer.width / 2.0 - product.x - product.width / 2.0).abs() < 0.01);
    let direct = &routes[0];
    assert!(
        direct
            .points
            .windows(2)
            .all(|leg| (leg[0].x - leg[1].x).abs() < 0.01),
        "the unpositioned child link should run vertically through its package"
    );
    let authored = format!("{source}\nlayout {{\n Vertrieb.Kunde < Vertrieb.Lager : gap=70\n}}\n");
    let (boxes, _, _) = draw(&authored);
    let customer = box_for(&boxes, "Vertrieb.Kunde");
    let warehouse = box_for(&boxes, "Vertrieb.Lager");
    assert!(
        warehouse.x > customer.x + customer.width + 50.0,
        "an explicit sibling placement must overrule the automatic stacking preference"
    );
}
#[test]
fn free_nary_diamond_aligns_with_middle_arm_and_routes_orthogonally() {
    let source = include_str!("../examples/class-nary.laml");
    let (boxes, routes, _) = draw(source);
    let patient = box_for(&boxes, "Patient");
    let diamond = box_for(&boxes, "Verordnet");
    assert!((patient.y + patient.height / 2.0 - diamond.y - diamond.height / 2.0).abs() < 0.01);
    assert!(routes.iter().all(|route| {
        route
            .points
            .windows(2)
            .all(|leg| (leg[0].x - leg[1].x).abs() < 0.01 || (leg[0].y - leg[1].y).abs() < 0.01)
    }));
    let authored = format!("{source}\nlayout {{\n Verordnet above Patient : gap=80\n}}\n");
    let (boxes, _, _) = draw(&authored);
    let patient = box_for(&boxes, "Patient");
    let diamond = box_for(&boxes, "Verordnet");
    assert!(
        diamond.y + diamond.height + 70.0 < patient.y,
        "mandatory hub placement must not be overwritten by median alignment"
    );
}
