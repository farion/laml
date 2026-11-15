use super::*;
use crate::{geometry::place, parser::parse, styles::StyleSheet, syntax::LabelPlacement};
fn label() -> Label {
    Label {
        text: "Read objects".into(),
        x: 0.,
        y: 0.,
        width: 80.,
        height: 18.,
        angle: 0.,
        placement: LabelPlacement::Near,
    }
}
fn obstacle(x: f64, y: f64) -> BoxNode {
    let d = parse("semantics {\nOther\n}\n").unwrap();
    let mut n = place(&d, &StyleSheet::default()).unwrap().remove(0);
    n.x = x;
    n.y = y;
    n.width = 100.;
    n.height = 50.;
    n
}
#[test]
fn collision_free_text_must_still_be_closest_to_its_owner() {
    let l = label();
    let owner = [Point { x: 96., y: -20. }, Point { x: 96., y: 60. }];
    let crowded = obstacle(0., 22.);
    assert!(
        !clear(&l, &owner, &[crowded], &[]),
        "node gap 4 is smaller than arrow gap 16"
    );
    assert!(clear(&l, &owner, &[obstacle(0., 100.)], &[]));
}
#[test]
fn a_closer_arrow_can_make_the_same_text_position_unambiguous() {
    let l = label();
    let owner = [Point { x: 88., y: -20. }, Point { x: 88., y: 60. }];
    assert!(clear(&l, &owner, &[obstacle(100., 0.)], &[]));
    assert!(
        !clear(&l, &owner, &[obstacle(89., 0.)], &[]),
        "one-pixel advantage is not sufficient"
    );
}
#[test]
fn one_close_corner_does_not_make_a_wide_caption_belong_to_the_arrow() {
    let l = label();
    let owner = [Point { x: 88., y: -20. }, Point { x: 88., y: 60. }];
    assert!(
        !clear(&l, &owner, &[obstacle(0., 35.)], &[]),
        "the box underneath owns the caption centre even though the arrow is nearest to one edge"
    );
    assert!(
        steals(
            &l,
            &owner,
            Point { x: -20., y: 50. },
            Point { x: 120., y: 50. },
            1.
        ),
        "a later arrow must not become nearer to the caption centre either"
    );
}

#[test]
fn later_arrows_cannot_steal_an_already_positioned_label() {
    let l = label();
    let owner = [Point { x: -20., y: 26. }, Point { x: 120., y: 26. }];
    assert!(steals(
        &l,
        &owner,
        Point { x: -20., y: 24. },
        Point { x: 120., y: 24. },
        1.
    ));
    assert!(!steals(
        &l,
        &owner,
        Point { x: -20., y: 32. },
        Point { x: 120., y: 32. },
        1.
    ));
}
#[test]
fn a_new_caption_must_not_steal_an_existing_caption_either() {
    let old = label();
    let route = Route {
        source_line: 0,
        points: vec![Point { x: 108., y: -20. }, Point { x: 108., y: 120. }],
        qualifiers: vec![],
        bridges: vec![],
        gaps: vec![],
        label: Some(old),
        end_labels: vec![],
        from: 0,
        to: 1,
        color: "black".into(),
        width: 1.,
        font_size: 14.,
        bidirectional: false,
        marker_end: true,
        appearance: Default::default(),
    };
    let mut new = label();
    new.y = 60.;
    let owner = [Point { x: -20., y: 88. }, Point { x: 120., y: 88. }];
    assert!(
        !clear(&new, &owner, &[], &[route]),
        "the old caption's centre would be nearer to the new caption than to its own arrow"
    );
}

#[test]
fn a_containing_package_is_measured_by_its_visible_contour() {
    let d = parse("semantics {\npackage Parent {\nChild\n}\n}\n").unwrap();
    let mut n = place(&d, &StyleSheet::default()).unwrap().remove(0);
    n.x = -200.;
    n.y = -100.;
    n.width = 500.;
    n.height = 400.;
    let l = label();
    let owner = [Point { x: -20., y: 26. }, Point { x: 120., y: 26. }];
    assert!(clear(&l, &owner, &[n], &[]));
}
#[test]
fn segment_distance_accounts_for_full_text_extent_and_diagonals() {
    let l = label();
    assert!(
        (segment_distance(&l, Point { x: 83., y: 22. }, Point { x: 83., y: 30. }) - 5.).abs()
            < 1e-6
    );
    assert_eq!(
        segment_distance(&l, Point { x: -10., y: -10. }, Point { x: 90., y: 28. }),
        0.
    );
}
