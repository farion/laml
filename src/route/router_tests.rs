use super::*;
use crate::{geometry::place, parser::parse};

#[test]
fn later_failure_is_not_hidden_by_an_earlier_failed_pass() {
    let failure = |line, message: &str| RouteError {
        line,
        message: message.into(),
    };
    let reported = furthest_failure(failure(4, "early"), failure(9, "later"));
    assert_eq!((reported.line, reported.message.as_str()), (9, "later"));
    let reported = furthest_failure(failure(9, "later"), failure(4, "early"));
    assert_eq!((reported.line, reported.message.as_str()), (9, "later"));
}

#[test]
fn picks_shorter_clear_corridor_over_first_external_detour() {
    let source = "semantics {\nA\nB\nC\nA -> C\n}\nlayout {\nA < B : align=center; gap=90\nB < C : align=center; gap=90\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let stable = route_pass(&diagram, &boxes, &sheet, None).unwrap();
    let chosen = route(&diagram, &boxes, &sheet).unwrap();
    assert!(
        ranking::better_than_orthogonal(&chosen, &stable),
        "bounded auto-routing should shorten a safe detour"
    );
    assert!(ranking::visibly_clear(&chosen, &boxes));
}
