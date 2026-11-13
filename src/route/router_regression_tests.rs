use super::*;
use crate::route::Point;
use crate::{geometry::place, parser::parse, styles::StyleSheet};

fn set_box(boxes: &mut [BoxNode], id: &str, x: f64, y: f64, w: f64, h: f64) {
    let b = boxes.iter_mut().find(|b| b.id == id).unwrap();
    b.x = x;
    b.y = y;
    b.width = w;
    b.height = h;
}

#[test]
fn nested_fanout_uses_local_corridors_without_crossings_or_left_loops() {
    let d=parse("semantics {\npackage Frame {\npackage Network {\npackage Private {\n@Gateway Gateway\n@Backend Backend\nBlocker\n@Database Database\n}\n}\n@Registry Registry\n@Telemetry Telemetry\n}\nGateway -> Backend\nBackend -> Database\nBackend ..> Telemetry\nGateway ..> Telemetry\nBackend ..> Registry : Download artifact\nGateway ..> Registry : Download artifact\n}\n").unwrap();
    let s = StyleSheet::default();
    let mut b = place(&d, &s).unwrap();
    for (id, x, y, w, h) in [
        ("Frame", 0., 0., 1300., 1150.),
        ("Frame.Network", 80., 100., 800., 800.),
        ("Frame.Network.Private", 110., 160., 500., 700.),
        ("Gateway", 200., 280., 160., 60.),
        ("Backend", 200., 420., 160., 60.),
        ("Frame.Network.Private.Blocker", 180., 510., 220., 60.),
        ("Database", 480., 420., 120., 60.),
        ("Registry", 980., 960., 150., 60.),
        ("Telemetry", 980., 1060., 240., 60.),
    ] {
        set_box(&mut b, id, x, y, w, h);
    }
    let before: Vec<_> = b.iter().map(|b| (b.x, b.y, b.width, b.height)).collect();
    let routes = route(&d, &b, &s).unwrap();
    assert_eq!(routes.len(), d.relations.len());
    assert!(routes.iter().all(|r| r.bridges.is_empty()), "{routes:?}");
    for r in &routes {
        assert!(super::super::obstacles::Obstacles::new(&b, r.from, r.to).clear(&r.points, &[]));
        assert!(super::super::orthogonal::axis_aligned(&r.points));
    }
    assert!(
        routes[3].points[0].y < routes[5].points[0].y,
        "the outer, farther target needs the upper departure slot"
    );
    for (i, r) in routes.iter().enumerate() {
        let others: Vec<_> = routes
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, r)| r.clone())
            .collect();
        assert!(
            super::super::obstacles::Obstacles::new(&b, r.from, r.to).clear(&r.points, &others),
            "joint clearance must survive restoring semantic order"
        );
    }
    let telemetry = &routes[2];
    assert!(
        telemetry.points.iter().all(|p| p.x >= 360.0),
        "no exterior left loop: {:?}",
        telemetry.points
    );
    let artifact = &routes[4];
    assert!(
        artifact.points.iter().all(|p| p.x >= 360.0),
        "artifact corridor: {:?}",
        artifact.points
    );
    assert_eq!(
        before,
        b.iter()
            .map(|b| (b.x, b.y, b.width, b.height))
            .collect::<Vec<_>>()
    );
    let again = route(&d, &b, &s).unwrap();
    assert_eq!(
        routes.iter().map(|r| &r.points).collect::<Vec<_>>(),
        again.iter().map(|r| &r.points).collect::<Vec<_>>()
    );
    // The result still follows semantic source order after allocation retries.
    for (r, decl) in routes.iter().zip(&d.relations) {
        assert_eq!(b[r.from].id, decl.from);
        assert_eq!(b[r.to].id, decl.to);
    }
}

#[test]
fn unrelated_objects_need_clearance_not_just_an_unintersected_interior() {
    let d = parse("semantics {\nA\nB\nObstacle\nA -> B\n}\n").unwrap();
    let mut b = place(&d, &StyleSheet::default()).unwrap();
    set_box(&mut b, "A", 0., 0., 100., 60.);
    set_box(&mut b, "B", 400., 0., 100., 60.);
    set_box(&mut b, "Obstacle", 180., 80., 100., 60.);
    let check = super::super::obstacles::Obstacles::new(&b, 0, 1);
    let path = |y| {
        vec![
            Point { x: 100., y: 30. },
            Point { x: 130., y: 30. },
            Point { x: 130., y },
            Point { x: 370., y },
            Point { x: 370., y: 30. },
            Point { x: 400., y: 30. },
        ]
    };
    assert!(
        !check.clear(&path(76.), &[]),
        "four pixels still hugs the obstacle"
    );
    assert!(check.clear(&path(72.), &[]), "the minimum gap is valid");
}

#[test]
fn a_short_labeled_top_arrival_does_not_need_a_hairpin() {
    let d=parse("semantics {\nA\nNeighbor\nStore\npackage Network {\nInner\n}\nA -> Store : Read objects\n}\n").unwrap();
    let s = StyleSheet::default();
    let mut b = place(&d, &s).unwrap();
    set_box(&mut b, "A", 200., 0., 160., 60.);
    set_box(&mut b, "Neighbor", 0., 0., 150., 60.);
    set_box(&mut b, "Store", -24., 84., 200., 60.);
    set_box(&mut b, "Network", 200., 84., 400., 300.);
    set_box(&mut b, "Network.Inner", 250., 200., 100., 60.);
    let r = route(&d, &b, &s).unwrap();
    assert_eq!(r[0].points.len(), 3, "{:?}", r[0].points);
    assert_eq!(r[0].points.last().unwrap().y, 84.0, "arrive from above");
    assert!(r[0].label.is_some());
}

#[test]
fn mandatory_top_side_can_dock_directly_on_a_package_tab() {
    let d=parse("semantics {\nA\npackage Network {\nInner\n}\n@r A -> Network\n}\nlayout {\nrelation[r].source : side=bottom\nrelation[r].target : side=top\n}\n").unwrap();
    let s = StyleSheet::default();
    let mut b = place(&d, &s).unwrap();
    set_box(&mut b, "A", 20., 0., 100., 60.);
    set_box(&mut b, "Network", 0., 100., 400., 300.);
    set_box(&mut b, "Network.Inner", 100., 200., 100., 60.);
    let r = route(&d, &b, &s).unwrap();
    assert_eq!(r[0].points.len(), 2, "{:?}", r[0].points);
    assert_eq!(r[0].points[1], Point { x: 70., y: 100. });
}
