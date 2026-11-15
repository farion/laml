use super::*;
use std::time::Duration;
fn type_at(doc: &mut Document, text: &str, millis: u64) {
    doc.replace_typed(text.into(), Duration::from_millis(millis))
        .unwrap();
}
#[test]
fn adjacent_unicode_typing_is_one_step_but_revisions_are_never_coalesced() {
    let mut doc = Document::new("// ".into());
    for (n, text) in ["// G", "// Grü", "// Grün"].iter().enumerate() {
        type_at(&mut doc, text, n as u64 * 100);
    }
    assert_eq!(doc.revision(), 3);
    assert_eq!(doc.undo.len(), 1);
    doc.undo();
    assert_eq!(doc.source(), "// ");
    doc.redo();
    assert_eq!(doc.source(), "// Grün");
    assert_eq!(doc.apply(0, "stale", vec![]), Err(EditError::StaleRevision));
}
#[test]
fn save_idle_navigation_paste_and_newlines_split_typing_groups() {
    let mut doc = Document::new(String::new());
    type_at(&mut doc, "A", 0);
    doc.mark_saved();
    type_at(&mut doc, "AB", 10);
    type_at(&mut doc, "ABC", 800);
    doc.break_typing_group();
    type_at(&mut doc, "ABCD", 810);
    doc.replace_text("ABCDpaste".into()).unwrap();
    type_at(&mut doc, "ABCDpaste!", 820);
    type_at(&mut doc, "ABCDpaste!\r\n", 830);
    type_at(&mut doc, "ABCDpaste!\r\nX", 840);
    assert_eq!(doc.undo.len(), 8);
    for _ in 0..7 {
        assert!(doc.undo());
    }
    assert_eq!(doc.source(), "A");
    assert!(!doc.is_dirty());
}
#[test]
fn contiguous_backward_and_forward_deletes_restore_exact_unicode_crlf() {
    let mut doc = Document::new("äé中\r\n".into());
    type_at(&mut doc, "äé\r\n", 0);
    type_at(&mut doc, "ä\r\n", 10);
    assert_eq!(doc.undo.len(), 1);
    doc.undo();
    assert_eq!(doc.source(), "äé中\r\n");
    type_at(&mut doc, "é中\r\n", 20);
    type_at(&mut doc, "中\r\n", 30);
    assert_eq!(doc.undo.len(), 1);
    doc.undo();
    assert_eq!(doc.source(), "äé中\r\n");
}
#[test]
fn cursor_discontinuities_and_time_going_back_do_not_merge() {
    let mut doc = Document::new("AB".into());
    type_at(&mut doc, "ABC", 100);
    type_at(&mut doc, "XABC", 200);
    type_at(&mut doc, "XABCD", 300);
    type_at(&mut doc, "XABCDE", 50);
    assert_eq!(doc.undo.len(), 4);
}
#[test]
fn history_stores_small_deltas_not_snapshots_of_large_documents() {
    let mut doc = Document::new("A".repeat(1024 * 1024));
    for n in 0..100 {
        let text = format!("{}{}", doc.source(), n % 10);
        doc.replace_text(text).unwrap();
    }
    let (entries, bytes) = doc.history_usage();
    assert_eq!(entries, 100);
    assert!(bytes < 4096);
    for _ in 0..100 {
        assert!(doc.undo());
    }
    assert_eq!(doc.source(), "A".repeat(1024 * 1024));
}
#[test]
fn bounded_history_retains_valid_patch_chains_through_undo_redo_and_branching() {
    let mut doc = Document::new(String::new());
    doc.history_limits = (3, 1024);
    for n in 1..10 {
        doc.replace_text("é".repeat(n)).unwrap();
    }
    assert_eq!(doc.history_usage().0, 3);
    for _ in 0..3 {
        assert!(doc.undo());
    }
    assert!(!doc.undo());
    assert_eq!(doc.source(), "é".repeat(6));
    for _ in 0..3 {
        assert!(doc.redo());
    }
    assert_eq!(doc.source(), "é".repeat(9));
    doc.undo();
    doc.replace_text("branch".into()).unwrap();
    assert!(!doc.redo());
    doc.history_limits = (3, 12);
    doc.replace_text("large".repeat(20)).unwrap();
    assert_eq!(doc.history_usage(), (0, 0));
    assert_eq!(doc.source(), "large".repeat(20));
    doc.replace_loaded("new file".into());
    assert!(!doc.undo());
}
#[test]
fn grouped_capacity_is_included_in_the_byte_budget() {
    let mut doc = Document::new(String::new());
    doc.history_limits = (256, 64);
    for n in 1..200 {
        type_at(&mut doc, &"x".repeat(n), n as u64);
        assert!(doc.history_usage().1 <= 64);
    }
    assert_eq!(doc.source().len(), 199);
}
