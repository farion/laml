use super::*;
use std::path::Path;
const SOURCE: &str =
    "semantics {\n A\n B\n C\n}\nlayout {\n A above B : align=left; gap=50 // keep\n B < C\n}\n";
fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}
#[test]
fn alignment_replaces_only_edited_axis_and_keeps_comments_and_gap() {
    let result = plan(SOURCE, &ids(&["A", "B"]), FreeAlignment::Right).unwrap();
    assert!(result.source.contains("A above B : gap=50 // keep"));
    assert!(result.source.contains(" B < C\n"));
    assert!(result.source.contains("A ? B : align=right"));
    crate::preview::build(&result.source, Path::new("test.laml")).unwrap();
}
#[test]
fn aligning_y_preserves_old_x_alignment_when_removing_vertical_separation() {
    let result = plan(SOURCE, &ids(&["A", "B"]), FreeAlignment::Top).unwrap();
    assert!(result.source.contains("A ? B : align=left // keep"));
    assert!(result.source.contains("A ? B : align=top"));
    // Two coincident edges do not authorize overlap. Preflight must reject this
    // instead of silently dropping the untouched X constraint.
    assert!(crate::preview::build(&result.source, Path::new("test.laml")).is_err());
}
#[test]
fn center_is_one_atomic_multi_node_edit_and_uses_last_selection_as_reference() {
    let source = "semantics {\r\n A\r\n B\r\n C\r\n}\r\n";
    let result = plan(source, &ids(&["C", "A", "B"]), FreeAlignment::Center).unwrap();
    assert_eq!(
        result.added,
        ["C ? B : align=center", "A ? B : align=center"]
    );
    assert!(result.source.contains("align=center\r\n"));
    crate::preview::build(&result.source, Path::new("test.laml")).unwrap();
    let mut doc = Document::new(source.into());
    doc.apply(0, "Align", result.edits).unwrap();
    doc.undo();
    assert_eq!(doc.source(), source);
}
#[test]
fn independent_center_splits_when_only_one_axis_changes() {
    let source = "semantics {\n A\n B\n}\nlayout {\n A ? B : align=center // keep\n}\n";
    let result = plan(source, &ids(&["A", "B"]), FreeAlignment::Left).unwrap();
    assert!(result.source.contains("A ? B : align=center-y // keep"));
    let dropped = crate::placement_edit::plan(source, "A", "B", Placement::Right).unwrap();
    assert!(dropped.source.contains("A ? B : align=center-y // keep"));
    assert!(dropped.source.contains("A > B"));
    let preview = crate::preview::build(&dropped.source, Path::new("test.laml")).unwrap();
    assert!(preview.nodes[0].x >= preview.nodes[1].x + preview.nodes[1].width);
}
#[test]
fn repeated_alignment_is_noop_and_unrelated_rules_survive() {
    let first = plan(SOURCE, &ids(&["A", "B"]), FreeAlignment::Right).unwrap();
    let second = plan(&first.source, &ids(&["A", "B"]), FreeAlignment::Right).unwrap();
    assert!(second.edits.is_empty());
    assert_eq!(first.source, second.source);
    assert!(second.source.contains(" B < C\n"));
}
#[test]
fn moved_node_keeps_perpendicular_alignment_after_changing_order() {
    let source = "semantics {\n A\n B\n C\n}\nlayout {\n A < B : align=center\n B < C\n}\n";
    let result = crate::placement_edit::plan(source, "A", "C", Placement::Right).unwrap();
    let preview = crate::preview::build(&result.source, Path::new("test.laml")).unwrap();
    assert!(result.source.contains("A ? B : align=center-y"));
    assert!(preview.nodes[0].x > preview.nodes[2].x);
    assert!((preview.nodes[0].y - preview.nodes[1].y).abs() < 1e-4);
}
