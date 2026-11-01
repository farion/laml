use laml::{geometry::place, parser::parse, route::route, styles::load};
use std::path::Path;

#[test]
fn class_showcase_is_compact_and_uses_axis_aligned_connections() {
    let path = Path::new("examples/showcase-class.laml");
    let graph = parse(include_str!("../examples/showcase-class.laml")).unwrap();
    let styles = load(&graph, path).unwrap();
    let boxes = place(&graph, &styles).unwrap();
    let routes = route(&graph, &boxes, &styles).unwrap();
    let min_x = boxes.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    let max_x = boxes.iter().map(|b| b.x + b.width).fold(0.0_f64, f64::max);
    let min_y = boxes.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let max_y = boxes.iter().map(|b| b.y + b.height).fold(0.0_f64, f64::max);
    assert!(max_x - min_x < 1000.0);
    assert!(max_y - min_y < 500.0);
    assert_eq!(routes.len(), 3);
    for relation in &routes {
        for segment in relation.points.windows(2) {
            assert!(
                (segment[0].x - segment[1].x).abs() < 1e-6
                    || (segment[0].y - segment[1].y).abs() < 1e-6,
                "class showcase has a diagonal segment: {segment:?}"
            );
        }
    }
}
