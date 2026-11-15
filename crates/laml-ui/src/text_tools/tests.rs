use super::*;
use crate::document::Document;
fn one(range: Range<usize>) -> Vec<Range<usize>> {
    std::iter::once(range).collect()
}
fn run(source: &str, range: Range<usize>, action: Action) -> (String, Range<usize>) {
    let change = command(source, range, action).unwrap();
    let mut doc = Document::new(source.into());
    doc.apply(0, "Source command", change.edits).unwrap();
    (doc.source().into(), change.selection)
}
#[test]
fn literal_search_maps_unicode_lowercase_back_to_original_bytes() {
    assert_eq!(matches("Äpfel äPFEL", "äpfel", false), [0..6, 7..13]);
    assert_eq!(matches("Äpfel äPFEL", "äpfel", true), []);
    assert_eq!(matches("İi", "i", false), one(2..3));
    assert_eq!(matches("İi", "İ", false), one(0..2));
    assert_eq!(matches("ééé", "éé", true), one(0..4));
    assert!(matches("", "", true).is_empty());
    assert_eq!(matches("[x] \\n", "[x]", true), one(0..3));
}
#[test]
fn replace_all_is_one_undo_preserves_other_text_and_has_no_recursive_matching() {
    let source = "// Grün\r\nGrün(label=\"Grün\")\r\n";
    let mut doc = Document::new(source.into());
    doc.apply(
        0,
        "Replace all",
        replacements(source, "Grün", "GrünBlue", true),
    )
    .unwrap();
    assert_eq!(
        doc.source(),
        "// GrünBlue\r\nGrünBlue(label=\"GrünBlue\")\r\n"
    );
    doc.undo();
    assert_eq!(doc.source(), source);
    assert!(replacements(source, "", "any", false).is_empty());
}
#[test]
fn toggle_comments_uses_whole_selected_lines_and_preserves_crlf() {
    let source = "  A(label=\"Ä // keep\")\r\n\tB\r\nC\r\n";
    let end = source.find("C\r").unwrap();
    let (commented, range) = run(source, 2..end, Action::Comment);
    assert_eq!(commented, "  // A(label=\"Ä // keep\")\r\n\t// B\r\nC\r\n");
    let (restored, _) = run(&commented, range, Action::Comment);
    assert_eq!(restored, source);
}
#[test]
fn mixed_comments_are_not_silently_uncommented_and_blank_lines_are_stable() {
    let source = "A\n // B\n\n";
    let (text, _) = run(source, 0..source.len(), Action::Comment);
    assert_eq!(text, "// A\n // // B\n\n");
    assert_eq!(run("", 0..0, Action::Comment).0, "// ");
}
#[test]
fn indentation_is_unicode_safe_and_does_not_touch_the_next_unselected_line() {
    let source = "ä\r\nB\r\nC";
    let end = source.find('C').unwrap();
    let (indented, selection) = run(source, 0..end, Action::Indent);
    assert_eq!(indented, "    ä\r\n    B\r\nC");
    assert_eq!(run(&indented, selection, Action::Outdent).0, source);
    assert_eq!(run("äX", 2..2, Action::Indent), ("ä   X".into(), 5..5));
    assert_eq!(run("\t A", 3..3, Action::Outdent), (" A".into(), 2..2));
}
#[test]
fn enter_uses_existing_eol_indentation_and_ignores_quoted_or_commented_braces() {
    assert_eq!(
        run("semantics {\r\n}", 11..11, Action::Newline).0,
        "semantics {\r\n    \r\n}"
    );
    assert_eq!(run("  A", 3..3, Action::Newline), ("  A\n  ".into(), 6..6));
    assert_eq!(
        run("  A(label=\"{\")", 14..14, Action::Newline).0,
        "  A(label=\"{\")\n  "
    );
    assert_eq!(run("// {", 4..4, Action::Newline).0, "// {\n");
    assert_eq!(
        run("semantics {}", 11..11, Action::Newline),
        ("semantics {\n    \n}".into(), 16..16)
    );
}
#[test]
fn malformed_byte_ranges_fail_without_panics() {
    assert!(command("ä", 1..2, Action::Comment).is_err());
    assert!(command("A", 0..2, Action::Newline).is_err());
    let invalid = Range { start: 1, end: 0 };
    assert!(command("A", invalid, Action::Indent).is_err());
}

#[test]
fn unicode_lowercase_uses_the_same_per_character_normalization_on_both_sides() {
    assert_eq!(matches("ΟΣ", "ΟΣ", false), one(0..4));
    assert_eq!(matches("ΟΣ", "οσ", false), one(0..4));
}
