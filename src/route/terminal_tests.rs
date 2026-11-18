use super::*;
use crate::{
    geometry,
    parser::parse,
    route,
    styles::StyleSheet,
    syntax::{EndPosition, PortSide},
};

fn fixture(target_side: &str) -> (crate::syntax::Diagram, Vec<BoxNode>, StyleSheet) {
    let source = format!(
        "semantics {{\n A\n D\n @r A -> D\n}}\nlayout {{\n relation[r].source : side={}\n relation[r].target : side={target_side}\n}}\nstyling {{\n node[D] {{\n shape: diamond\n }}\n}}\n",
        if target_side == "right" {
            "top"
        } else {
            "left"
        }
    );
    let diagram = parse(&source).unwrap();
    let sheet = crate::styles::load(&diagram, std::path::Path::new("terminal-test.laml")).unwrap();
    let mut boxes = geometry::place(&diagram, &sheet).unwrap();
    for node in &mut boxes {
        let (x, y, w, h) = if node.id == "D" {
            (0., 0., 100., 100.)
        } else if target_side == "right" {
            (50., 200., 100., 60.)
        } else {
            (200., 70., 100., 60.)
        };
        node.x = x;
        node.y = y;
        node.width = w;
        node.height = h;
    }
    (diagram, boxes, sheet)
}

#[test]
fn diamond_right_vertex_rejects_a_vertical_tangent_and_routes_from_the_right() {
    let (d, boxes, sheet) = fixture("right");
    let route = route::route(&d, &boxes, &sheet).unwrap().remove(0);
    let last = route.points.len() - 1;
    let tip = route.points[last];
    let before = route.points[last - 1];
    assert!((tip.x - 100.).abs() < 1e-6 && (tip.y - 50.).abs() < 1e-6);
    assert!(
        (before.y - tip.y).abs() < 1e-6 && before.x > tip.x + 20.,
        "{:?}",
        route.points
    );
    assert!(super::super::orthogonal::normal_ends(
        &route.points,
        &boxes[0],
        &boxes[1]
    ));
}
#[test]
fn diamond_bottom_vertex_has_a_vertical_run_longer_than_the_marker_and_rounding() {
    let (d, boxes, sheet) = fixture("bottom");
    let route = route::route(&d, &boxes, &sheet).unwrap().remove(0);
    let last = route.points.len() - 1;
    let tip = route.points[last];
    let before = route.points[last - 1];
    assert!((tip.x - 50.).abs() < 1e-6 && (tip.y - 100.).abs() < 1e-6);
    assert!((before.x - tip.x).abs() < 1e-6 && before.y > tip.y);
    assert!(
        straight_run(&route.points, false)
            >= minimum(
                route.appearance.end,
                route.width,
                sheet.theme.modern(),
                false
            )
    );
}
#[test]
fn short_marker_terminal_is_rejected_even_when_it_is_axial() {
    let (d, boxes, sheet) = fixture("bottom");
    let points = [
        Point { x: 200., y: 100. },
        Point { x: 160., y: 100. },
        Point { x: 160., y: 121. },
        Point { x: 50., y: 121. },
        Point { x: 50., y: 100. },
    ];
    assert!(!clear(
        &points,
        &d.relations[0],
        2.,
        sheet.theme.modern(),
        &boxes[0],
        &boxes[1]
    ));
    let longer = points.map(|p| Point {
        y: if p.y == 121. { 140. } else { p.y },
        ..p
    });
    assert!(clear(
        &longer,
        &d.relations[0],
        2.,
        sheet.theme.modern(),
        &boxes[0],
        &boxes[1]
    ));
}
#[test]
fn collinear_subdivisions_do_not_shorten_the_actual_terminal_run() {
    let points = [
        Point { x: 0., y: 0. },
        Point { x: 0., y: 50. },
        Point { x: 40., y: 50. },
        Point { x: 45., y: 50. },
        Point { x: 50., y: 50. },
    ];
    assert_eq!(straight_run(&points, false), 50.);
    assert_eq!(straight_run(&points, true), 50.);
}
#[test]
fn generic_shape_extremes_reject_tangency_on_every_side() {
    let (_, mut boxes, _) = fixture("right");
    for shape in [
        crate::geometry::NodeShape::Rectangle,
        crate::geometry::NodeShape::Ellipse,
        crate::geometry::NodeShape::Diamond,
        crate::geometry::NodeShape::Note,
    ] {
        boxes[1].shape = shape;
        let node = &boxes[1];
        for side in [
            PortSide::Left,
            PortSide::Right,
            PortSide::Top,
            PortSide::Bottom,
        ] {
            let dock = super::super::preferred::forced_port(node, side);
            let direction = match side {
                PortSide::Left => (-1., 0.),
                PortSide::Right => (1., 0.),
                PortSide::Top => (0., -1.),
                PortSide::Bottom => (0., 1.),
            };
            let outside = Point {
                x: dock.x + direction.0 * 40.,
                y: dock.y + direction.1 * 40.,
            };
            let tangent = Point {
                x: dock.x - direction.1 * 40.,
                y: dock.y + direction.0 * 40.,
            };
            let contours = super::super::orthogonal::Contours::new(node, node);
            assert!(
                contours.normal(&[dock, outside, dock], false),
                "{shape:?} {side:?}"
            );
            assert!(
                !contours.normal(&[dock, tangent, dock], false),
                "{shape:?} {side:?}"
            );
        }
    }
}
#[test]
fn replay_rejects_an_old_tangent_instead_of_republishing_it() {
    let (d, boxes, sheet) = fixture("right");
    let mut routes = route::route(&d, &boxes, &sheet).unwrap();
    routes[0].points = vec![Point { x: 100., y: 200. }, Point { x: 100., y: 50. }];
    assert!(route::replay(&d, &boxes, &sheet, &routes, &d).is_err());
    assert_eq!(
        super::super::side_rules::requested(&d, 0, EndPosition::Target),
        Some(PortSide::Right)
    );
}

#[test]
fn screenshot_parallel_diamond_renders_without_changing_source_constraints() {
    let source = include_str!("../../tests/fixtures/gne-parallel-diamond.laml");
    let diagram = parse(source).unwrap();
    let sheet = crate::styles::load(&diagram, std::path::Path::new("gne-test.laml")).unwrap();
    let (boxes, routes) = route::place_and_route(&diagram, &sheet).unwrap();
    assert_eq!(routes.len(), 6);
    for r in &routes {
        let relation = diagram
            .relations
            .iter()
            .find(|rel| rel.line == r.source_line)
            .unwrap();
        assert!(super::super::orthogonal::normal_ends(
            &r.points,
            &boxes[r.from],
            &boxes[r.to]
        ));
        assert!(clear(
            &r.points,
            relation,
            r.width,
            sheet.theme.modern(),
            &boxes[r.from],
            &boxes[r.to]
        ));
    }
    let rel = diagram
        .relations
        .iter()
        .find(|r| r.name.as_deref() == Some("Relation4"))
        .unwrap();
    let r = routes.iter().find(|r| r.source_line == rel.line).unwrap();
    let a = &boxes[r.from];
    let b = &boxes[r.to];
    assert_eq!(r.points[0].x, a.x + a.width);
    assert_eq!(r.points.last().unwrap().y, b.y + b.height);
    assert!(super::super::orthogonal::normal_ends(&r.points, a, b));
    assert!(clear(&r.points, rel, r.width, sheet.theme.modern(), a, b));
    let svg =
        crate::svg::render_with_theme(&boxes, &routes, crate::svg::TextMode::System, sheet.theme)
            .unwrap();
    assert!(svg.contains("marker-end"));
}
