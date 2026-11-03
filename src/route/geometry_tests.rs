use super::*;
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
#[test]
fn t_junction_is_a_forbidden_unbridged_intersection() {
    assert!(crosses(p(0.0, 5.0), p(10.0, 5.0), p(5.0, 0.0), p(5.0, 5.0)));
}
#[test]
fn a_later_route_cannot_pass_through_an_existing_bridge_hump() {
    let mut older = Route {
        points: vec![p(0.0, 100.0), p(200.0, 100.0)],
        qualifiers: vec![],
        bridges: vec![],
        gaps: vec![],
        label: None,
        end_labels: vec![],
        from: 0,
        to: 1,
        color: String::new(),
        width: 2.0,
        font_size: 14.0,
        bidirectional: false,
        marker_end: false,
        appearance: Default::default(),
    };
    let later = [p(70.0, 116.0), p(130.0, 116.0)];
    assert!(route_is_clear(&later, &[], &[older.clone()]));
    older.bridges.push(super::super::Bridge {
        segment: 0,
        distance: 100.0,
        radius: 8.0,
        height: 8.0,
    });
    assert!(!route_is_clear(&later, &[], &[older]));
}
#[test]
fn marker_clearance_covers_the_largest_visible_diamond() {
    use crate::syntax::EndSymbol;
    assert!(marker_radius(EndSymbol::FilledDiamond, 2.0) > marker_radius(EndSymbol::Triangle, 2.0));
    assert!(marker_radius(EndSymbol::Triangle, 2.0) > marker_radius(EndSymbol::Arrow, 2.0));
    assert_eq!(marker_radius(EndSymbol::None, 0.0), 0.0);
}
#[test]
fn new_routes_do_not_clip_existing_arrowheads() {
    let mut old = Route {
        points: vec![p(0.0, 100.0), p(200.0, 100.0)],
        qualifiers: vec![],
        bridges: vec![],
        gaps: vec![],
        label: None,
        end_labels: vec![],
        from: 0,
        to: 1,
        color: String::new(),
        width: 2.0,
        font_size: 14.0,
        bidirectional: false,
        marker_end: false,
        appearance: Default::default(),
    };
    let later = [p(190.0, 112.0), p(210.0, 112.0)];
    assert!(route_is_clear(&later, &[], &[old.clone()]));
    old.appearance.end = crate::syntax::EndSymbol::Arrow;
    assert!(!route_is_clear(&later, &[], &[old]));
}
#[test]
fn folded_note_uses_diagonal_stroke_for_ports_and_obstacles() {
    let diagram = crate::parser::parse(
            "settings {\ntheme: plain\n}\nsemantics {\nMemo\n}\nstyling {\nnode[Memo] {\nshape: note\n}\n}\n",
        )
        .unwrap();
    let styles =
        crate::styles::load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = crate::geometry::place(&diagram, &styles).unwrap();
    let note = &boxes[0];
    let (x, y, w, h) = (note.x, note.y, note.width, note.height);
    let fold = note_fold(note);
    let hit = port(note, p(x + w, y));
    assert!((hit.x - hit.y - (x + w - fold - y)).abs() < 1e-6);
    assert!(hit.x > x + w - fold && hit.y > y);
    assert!(!intersection(
        p(x + w - fold * 0.1, y + fold * 0.1),
        p(x + w - fold * 0.1, y + fold * 0.4),
        note
    ));
    assert!(intersection(
        p(x + w / 2.0, y + h / 2.0),
        p(x + w + 10.0, y - 10.0),
        note
    ));
}
#[test]
fn collinear_overdraw_is_rejected() {
    assert!(crosses(
        p(0.0, 0.0),
        p(10.0, 0.0),
        p(5.0, 0.0),
        p(15.0, 0.0)
    ));
    assert!(!crosses(
        p(0.0, 0.0),
        p(10.0, 0.0),
        p(10.0, 0.0),
        p(15.0, 0.0)
    ));
}
