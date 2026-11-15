//! Atomic source edits and shared text/visual history.
use std::ops::Range;
mod history;
use history::Typing;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    StaleRevision,
    InvalidRange,
    Overlap,
}

#[derive(Clone, Debug)]
struct Change {
    label: String,
    start: usize,
    before: String,
    after: String,
}

#[derive(Clone, Debug)]
pub struct Document {
    source: String,
    saved: String,
    revision: u64,
    undo: Vec<Change>,
    redo: Vec<Change>,
    typing: Option<Typing>,
    history_limits: (usize, usize),
}

impl Document {
    pub fn new(source: String) -> Self {
        Self {
            saved: source.clone(),
            source,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            typing: None,
            history_limits: (history::MAX_CHANGES, history::MAX_BYTES),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn is_dirty(&self) -> bool {
        self.source != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.mark_saved_snapshot(self.source.clone());
    }
    pub fn mark_saved_snapshot(&mut self, source: String) {
        self.break_typing_group();
        self.saved = source;
    }
    /// Changing files or accepting an external reload invalidates every pending command.
    pub fn replace_loaded(&mut self, source: String) {
        self.source = source.clone();
        self.saved = source;
        self.break_typing_group();
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
    }

    /// Path/import changes invalidate previews without changing the text history.
    pub fn invalidate(&mut self) {
        self.break_typing_group();
        self.revision += 1;
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|change| change.label.as_str())
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// All ranges refer to the same source revision. Validate everything before mutation.
    pub fn apply(
        &mut self,
        revision: u64,
        label: impl Into<String>,
        mut edits: Vec<Edit>,
    ) -> Result<bool, EditError> {
        if revision != self.revision {
            return Err(EditError::StaleRevision);
        }
        edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
        for edit in &edits {
            if edit.range.start > edit.range.end
                || edit.range.end > self.source.len()
                || !self.source.is_char_boundary(edit.range.start)
                || !self.source.is_char_boundary(edit.range.end)
            {
                return Err(EditError::InvalidRange);
            }
        }
        for pair in edits.windows(2) {
            if pair[0].range.end > pair[1].range.start || pair[0].range.start == pair[1].range.start
            {
                return Err(EditError::Overlap);
            }
        }
        let mut after = self.source.clone();
        for edit in edits.into_iter().rev() {
            after.replace_range(edit.range, &edit.replacement);
        }
        if after == self.source {
            return Ok(false);
        }
        self.break_typing_group();
        self.undo
            .push(Change::between(&self.source, &after, label.into()));
        self.source = after;
        self.redo.clear();
        self.trim_history();
        self.revision += 1;
        Ok(true)
    }

    /// Text input uses the same history as visual commands.
    pub fn replace_text(&mut self, text: String) -> Result<bool, EditError> {
        self.apply(
            self.revision,
            "Edit source",
            vec![Edit {
                range: 0..self.source.len(),
                replacement: text,
            }],
        )
    }

    pub fn undo(&mut self) -> bool {
        self.break_typing_group();
        let Some(change) = self.undo.pop() else {
            return false;
        };
        self.source.replace_range(
            change.start..change.start + change.after.len(),
            &change.before,
        );
        self.redo.push(change);
        self.revision += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        self.break_typing_group();
        let Some(change) = self.redo.pop() else {
            return false;
        };
        self.source.replace_range(
            change.start..change.start + change.before.len(),
            &change.after,
        );
        self.undo.push(change);
        self.revision += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edit(range: Range<usize>, text: &str) -> Edit {
        Edit {
            range,
            replacement: text.into(),
        }
    }

    #[test]
    fn multiple_edits_preserve_comments_and_crlf_and_undo_together() {
        let source = "A -> B // keep\r\nC -> D\r\n";
        let mut doc = Document::new(source.into());
        doc.apply(
            0,
            "Rename",
            vec![edit(0..1, "Alpha"), edit(16..17, "Gamma")],
        )
        .unwrap();
        assert_eq!(doc.source(), "Alpha -> B // keep\r\nGamma -> D\r\n");
        assert_eq!(doc.undo_label(), Some("Rename"));
        assert!(doc.is_dirty());
        assert!(doc.undo());
        assert_eq!(doc.source(), source);
        assert!(!doc.is_dirty());
        assert!(doc.redo());
        assert!(doc.is_dirty());
    }

    #[test]
    fn rejects_invalid_transactions_without_partial_changes() {
        let mut doc = Document::new("äBC".into());
        assert_eq!(
            doc.apply(0, "bad", vec![edit(1..2, "x")]),
            Err(EditError::InvalidRange)
        );
        assert_eq!(
            doc.apply(0, "bad", vec![edit(2..4, "x"), edit(3..4, "y")]),
            Err(EditError::Overlap)
        );
        assert_eq!(
            doc.apply(0, "bad", vec![edit(2..2, "x"), edit(2..2, "y")]),
            Err(EditError::Overlap)
        );
        assert_eq!(doc.source(), "äBC");
        assert_eq!(doc.revision(), 0);
    }

    #[test]
    fn stale_commands_remain_stale_after_undo() {
        let mut doc = Document::new("A".into());
        doc.replace_text("B".into()).unwrap();
        doc.undo();
        assert_eq!(
            doc.apply(0, "old", vec![edit(0..1, "C")]),
            Err(EditError::StaleRevision)
        );
    }

    #[test]
    fn text_and_visual_commands_share_history_and_branch() {
        let mut doc = Document::new("A".into());
        doc.replace_text("AB".into()).unwrap();
        doc.apply(doc.revision(), "Visual", vec![edit(1..2, "C")])
            .unwrap();
        doc.mark_saved();
        doc.undo();
        assert_eq!(doc.source(), "AB");
        assert!(doc.is_dirty());
        doc.undo();
        assert_eq!(doc.source(), "A");
        doc.replace_text("D".into()).unwrap();
        assert!(!doc.redo());
    }
}

#[cfg(test)]
mod history_tests;
