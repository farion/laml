use super::*;
use crate::{geometry::place, parser::parse, styles::StyleSheet};

#[test]
fn rectangular_connectors_require_normal_departure_and_arrival() {
    let diagram =
        parse("semantics {\nA\nB\nA -> B\n}\nlayout {\nA < B : align=center; gap=100\n}\n")
            .unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    let (a, b) = (&boxes[0], &boxes[1]);
    let y = a.y + a.height / 2.0;
    let start = Point {
        x: a.x + a.width,
        y,
    };
    let end = Point { x: b.x, y };
    let valid = [
        start,
        Point {
            x: start.x + 20.0,
            y,
        },
        Point { x: end.x - 20.0, y },
        end,
    ];
    assert!(normal_ends(&valid, a, b));
    let tangent = [
        start,
        Point {
            x: start.x,
            y: y + 30.0,
        },
        end,
    ];
    assert!(!normal_ends(&tangent, a, b));
    assert!(!non_tangent_ends(&tangent, a, b));
    let diagonal = [
        start,
        Point {
            x: start.x + 28.0,
            y: y + 15.0,
        },
        end,
    ];
    assert!(non_tangent_ends(&diagonal, a, b));
    let short = [
        start,
        Point {
            x: start.x + 19.0,
            y,
        },
        end,
    ];
    assert!(!normal_ends(&short, a, b));
    let tangent_arrival = [
        start,
        Point {
            x: end.x,
            y: y + 30.0,
        },
        end,
    ];
    assert!(!normal_ends(&tangent_arrival, a, b));
    assert!(!non_tangent_ends(&tangent_arrival, a, b));
}

#[test]
fn offers_a_corridor_beyond_an_existing_long_route() {
    let diagram = parse(
            "semantics {\nA\nB\nC\nA -> B\n}\nlayout {\nA < B : align=center; gap=180\nB above C : gap=80\n}\n",
        )
        .unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let previous = super::super::route(&diagram, &boxes, &sheet).unwrap();
    let outer_x = previous[0]
        .points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max)
        + 64.0;
    let options = candidates(
        &boxes[0], &boxes[2], None, None, None, None, &previous, None, 24.0,
    );
    assert!(options.iter().any(|path| {
        axis_aligned(path) && path.iter().any(|point| (point.x - outer_x).abs() < 1e-6)
    }));
}
