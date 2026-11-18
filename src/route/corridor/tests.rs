use super::*;
use crate::{
    geometry, parser,
    syntax::{EndPosition, PortSide},
};
use std::path::Path;

fn fixture(source: &str) -> (Diagram, StyleSheet) {
    let d = parser::parse(source).unwrap();
    let s = crate::styles::load(&d, Path::new("corridor-test.laml")).unwrap();
    (d, s)
}
fn assert_valid(d: &Diagram, boxes: &[BoxNode], routes: &[Route], sheet: &StyleSheet) {
    assert_eq!(routes.len(), d.relations.len());
    for (i, (r, decl)) in routes.iter().zip(&d.relations).enumerate() {
        assert_eq!(r.source_line, decl.line);
        assert!(boxes[r.from].has_endpoint(&decl.from));
        assert!(boxes[r.to].has_endpoint(&decl.to));
        assert!(super::super::orthogonal::axis_aligned(&r.points));
        assert!(
            search::simple(&r.points),
            "no self-crossing or doubled-back strokes"
        );
        assert!(super::super::orthogonal::normal_ends(
            &r.points,
            &boxes[r.from],
            &boxes[r.to]
        ));
        assert!(super::super::terminal::clear(
            &r.points,
            decl,
            r.width,
            sheet.theme.modern(),
            &boxes[r.from],
            &boxes[r.to]
        ));
        let others: Vec<_> = routes
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, r)| r.clone())
            .collect();
        assert!(
            super::super::obstacles::Obstacles::new(boxes, r.from, r.to).clear(&r.points, &others),
            "{}",
            decl.from
        );
        if let Some(label) = &r.label {
            assert!(super::super::label_ownership::clear(
                label, &r.points, boxes, &others
            ));
        }
        for (end, node, port) in [
            (EndPosition::Source, &boxes[r.from], r.points[0]),
            (EndPosition::Target, &boxes[r.to], *r.points.last().unwrap()),
        ] {
            if let Some(side) = super::super::side_rules::requested(d, i, end) {
                let fits = match side {
                    PortSide::Left => (port.x - node.x).abs() < 1e-6,
                    PortSide::Right => (port.x - node.x - node.width).abs() < 1e-6,
                    PortSide::Top => (port.y - node.y).abs() < 1e-6,
                    PortSide::Bottom => (port.y - node.y - node.height).abs() < 1e-6,
                };
                assert!(fits, "mandatory side {side:?}");
            }
        }
    }
}

#[test]
fn supplied_diagram_uses_corridors_without_exhaustive_enumeration() {
    let (d, s) = fixture(include_str!(
        "../../../tests/fixtures/gne-self-loop-routing-order.laml"
    ));
    let boxes = geometry::place_with_spacing(&d, &s, 64.).unwrap();
    let routes = route(&d, &boxes, &s).expect("the corridor planner itself must solve the fixture");
    assert_valid(&d, &boxes, &routes, &s);
    assert!(routes.iter().all(|r| r.bridges.is_empty()));
    let again = route(&d, &boxes, &s).unwrap();
    assert_eq!(
        serde_json::to_vec(&routes).unwrap(),
        serde_json::to_vec(&again).unwrap()
    );
    let (placed, compiled) = super::super::place_and_route(&d, &s).unwrap();
    assert_eq!(
        serde_json::to_vec(&compiled).unwrap(),
        serde_json::to_vec(&routes).unwrap()
    );
    assert_eq!(
        placed.iter().map(|b| (b.x, b.y)).collect::<Vec<_>>(),
        boxes.iter().map(|b| (b.x, b.y)).collect::<Vec<_>>()
    );
}

#[test]
fn direct_candidates_are_canonical_and_unique() {
    let (d, s) = fixture("semantics {\n A\n B\n @R A -> B\n}\n");
    let boxes = geometry::place(&d, &s).unwrap();
    let paths = direct(&d, 0, &boxes, &[], 32.);
    let unique: std::collections::BTreeSet<_> = paths
        .iter()
        .map(|p| {
            p.iter()
                .map(|p| (p.x.to_bits(), p.y.to_bits()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(paths.len(), unique.len());
    for p in paths {
        assert_eq!(p, search::simplify(p.clone()));
    }
}

#[test]
fn corridor_graph_routes_around_a_wall_without_fixed_bend_limit() {
    let (d, s) = fixture(
        "semantics {\n A\n B\n Top\n Bottom\n Wall\n @R A -> B\n}\nlayout {\n relation[R].source : side=right\n relation[R].target : side=left\n}\n",
    );
    let mut boxes = geometry::place(&d, &s).unwrap();
    for (id, x, y, w, h) in [
        ("A", 0., 0., 100., 60.),
        ("B", 600., 0., 100., 60.),
        ("Top", 120., -80., 280., 40.),
        ("Bottom", 120., 100., 280., 40.),
        ("Wall", 360., -40., 40., 100.),
    ] {
        let b = boxes.iter_mut().find(|b| b.id == id).unwrap();
        b.x = x;
        b.y = y;
        b.width = w;
        b.height = h;
    }
    let path = paths(&d, 0, &boxes, &[], 32., true)
        .next()
        .expect("walk through the lower opening");
    assert!(
        path.len() > 5,
        "a logical corridor can have arbitrarily many elbows: {path:?}"
    );
    assert!(super::super::obstacles::Obstacles::new(&boxes, 0, 1).clear(&path, &[]));
    assert!(super::super::orthogonal::normal_ends(
        &path, &boxes[0], &boxes[1]
    ));
}

#[test]
fn crossed_straight_lines_choose_a_crossing_free_detour_first() {
    let (d, s) = fixture("semantics {\n A\n B\n C\n D\n @one A -> B\n @two C -> D\n}\n");
    let mut boxes = geometry::place(&d, &s).unwrap();
    for (id, x, y) in [
        ("A", 0., 100.),
        ("B", 500., 100.),
        ("C", 250., -100.),
        ("D", 250., 300.),
    ] {
        let b = boxes.iter_mut().find(|b| b.id == id).unwrap();
        b.x = x;
        b.y = y;
        b.width = 80.;
        b.height = 50.;
    }
    let routes = route(&d, &boxes, &s).unwrap();
    assert_valid(&d, &boxes, &routes, &s);
    assert!(routes.iter().all(|r| r.bridges.is_empty()));
    assert!(!crosses(&routes[1].points, &routes[..1]));
}

#[test]
fn narrow_or_blocked_docks_do_not_invent_an_acceptable_route() {
    let (d, s) = fixture(
        "semantics {\n A\n B\n Wall\n @R A -> B\n}\nlayout {\n relation[R].source : side=right\n relation[R].target : side=left\n}\n",
    );
    let mut boxes = geometry::place(&d, &s).unwrap();
    for (id, x, y, w, h) in [
        ("A", 0., 0., 100., 60.),
        ("B", 400., 0., 100., 60.),
        ("Wall", 101., -100., 200., 300.),
    ] {
        let b = boxes.iter_mut().find(|b| b.id == id).unwrap();
        b.x = x;
        b.y = y;
        b.width = w;
        b.height = h;
    }
    assert!(route(&d, &boxes, &s).is_none());
    assert!(paths(&d, 0, &boxes, &[], 32., false).next().is_none());
}
