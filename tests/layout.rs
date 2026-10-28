use laml::layout::validate_ordering;
use laml::parser::parse;

#[test]
fn contradictory_horizontal_rules_fail_at_the_closing_rule() {
    let source = "semantics {\nA\nB\nC\n}\nlayout {\nA < B\nB < C\nC < A\n}\n";
    let error = validate_ordering(&parse(source).unwrap()).unwrap_err();
    assert_eq!(error.line, 9);
    assert!(error.message.contains("impossible horizontal placement"));
}

#[test]
fn contradictory_vertical_rules_fail_but_mixed_axes_are_allowed() {
    let bad = "semantics {\nA\nB\n}\nlayout {\nA above B\nB above A\n}\n";
    let error = validate_ordering(&parse(bad).unwrap()).unwrap_err();
    assert_eq!(error.line, 7);
    let okay = "semantics {\nA\nB\n}\nlayout {\nA < B\nB above A\n}\n";
    validate_ordering(&parse(okay).unwrap()).unwrap();
}

#[test]
fn semantic_relation_cycles_are_not_hard_layout_cycles() {
    let source = "semantics {\nA\nB\nA -> B\nB -> A\n}\n";
    validate_ordering(&parse(source).unwrap()).unwrap();
}

#[test]
fn a_node_cannot_be_strictly_left_of_itself() {
    let source = "semantics {\nA\n}\nlayout {\nA < A\n}\n";
    assert!(validate_ordering(&parse(source).unwrap()).is_err());
}
