use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::load;
use laml::svg::render;
use std::path::Path;

fn draw(source: &str) -> (Vec<laml::route::Route>, String) {
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/crossing.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    (routes, svg)
}

#[test]
fn later_relation_gets_a_bridge_and_earlier_gets_a_small_gap() {
    let source = include_str!("../examples/crossing.laml");
    let (routes, svg) = draw(source);
    assert_eq!(routes.len(), 2);
    assert!(routes[0].bridges.is_empty());
    assert_eq!(routes[0].gaps.len(), 1);
    assert_eq!(routes[1].bridges.len(), 1);
    assert!(routes[1].gaps.is_empty());
    assert!(routes[0].gaps[0].half < 20.0);
    assert!(svg.contains(" Q"));
    assert_eq!(
        svg.matches("<path d=").count(),
        svg.matches("<marker id=").count() + 3
    ); // three route fragments plus one path per available marker
    assert_eq!(
        svg,
        draw(source).1,
        "same source should reproduce SVG bytes"
    );
}

#[test]
fn crossing_showcase_bridge_rises_above_its_chord() {
    for source in [
        include_str!("../examples/crossing.laml").to_string(),
        include_str!("../examples/crossing.laml").replace("second: B -> C", "second: C -> B"),
    ] {
        let (routes, svg) = draw(&source);
        let bridged = routes.iter().find(|r| !r.bridges.is_empty()).unwrap();
        let bridge = &bridged.bridges[0];
        let pair = &bridged.points[bridge.segment..=bridge.segment + 1];
        let length = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        let center_y = pair[0].y + (pair[1].y - pair[0].y) * bridge.distance / length;
        let control_y: f64 = svg
            .split(" Q")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .split(',')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            control_y < center_y - 2.0,
            "bridge must arch upward regardless of arrow direction"
        );
    }
}
#[test]
fn crlf_source_preserves_crossing_geometry() {
    let source = include_str!("../examples/crossing.laml").replace("\r\n", "\n");
    let crlf = source.replace('\n', "\r\n");
    assert_eq!(draw(&source).1, draw(&crlf).1);
}

#[test]
fn reversing_relation_declarations_reverses_bridge_ownership() {
    // Git may check out .laml fixtures with CRLF on Windows.
    let source = include_str!("../examples/crossing.laml").replace("\r\n", "\n");
    let swapped = source.replace(
        "first: A -> D\n    second: B -> C",
        "second: B -> C\n    first: A -> D",
    );
    assert_ne!(source, swapped);
    let (routes, _) = draw(&swapped);
    assert!(routes[0].bridges.is_empty());
    assert_eq!(routes[1].bridges.len(), 1);
}
