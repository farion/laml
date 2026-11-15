use super::*;

pub(super) const MAX_CHANGES: usize = 256;
pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(super) struct Typing {
    pub at: std::time::Duration,
    pub kind: Kind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Insert,
    Delete,
    Boundary,
}

impl Change {
    pub(super) fn between(before: &str, after: &str, label: String) -> Self {
        let start = before
            .chars()
            .zip(after.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum();
        let suffix: usize = before[start..]
            .chars()
            .rev()
            .zip(after[start..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum();
        Self {
            label,
            start,
            before: before[start..before.len() - suffix].into(),
            after: after[start..after.len() - suffix].into(),
        }
    }
    pub(super) fn bytes(&self) -> usize {
        self.before.capacity() + self.after.capacity() + self.label.capacity()
    }
    pub(super) fn kind(&self) -> Kind {
        if self.before.contains(['\n', '\r']) || self.after.contains(['\n', '\r']) {
            Kind::Boundary
        } else if self.before.is_empty() {
            Kind::Insert
        } else if self.after.is_empty() {
            Kind::Delete
        } else {
            Kind::Boundary
        }
    }
    pub(super) fn merge(&mut self, next: &Self, kind: Kind) -> bool {
        match kind {
            Kind::Insert
                if self.before.is_empty() && self.start + self.after.len() == next.start =>
            {
                self.after.push_str(&next.after);
                true
            }
            Kind::Delete
                if self.after.is_empty() && next.start + next.before.len() == self.start =>
            {
                self.before.insert_str(0, &next.before);
                self.start = next.start;
                true
            }
            Kind::Delete if self.after.is_empty() && self.start == next.start => {
                self.before.push_str(&next.before);
                true
            }
            _ => false,
        }
    }
}

impl Document {
    /// Ordinary typing coalesces only adjacent insertions/deletions within 750 ms.
    /// Paste, explicit commands, caret movement, save and undo are group boundaries.
    pub fn replace_typed(
        &mut self,
        text: String,
        at: std::time::Duration,
    ) -> Result<bool, EditError> {
        if text == self.source {
            return Ok(false);
        }
        let change = Change::between(&self.source, &text, "Edit source".into());
        let kind = change.kind();
        let merge = self.typing.is_some_and(|last| {
            last.kind == kind
                && kind != Kind::Boundary
                && at
                    .checked_sub(last.at)
                    .is_some_and(|gap| gap <= std::time::Duration::from_millis(750))
        }) && self
            .undo
            .last_mut()
            .is_some_and(|last| last.merge(&change, kind));
        if !merge {
            self.undo.push(change);
        }
        self.source = text;
        self.redo.clear();
        self.revision += 1;
        self.typing = Some(Typing { at, kind });
        self.trim_history();
        Ok(true)
    }
    pub fn break_typing_group(&mut self) {
        self.typing = None;
    }
    /// Count retained changes and their allocated edit/string payload, excluding the
    /// live and saved buffers and the small bounded vector/entry overhead.
    pub fn history_usage(&self) -> (usize, usize) {
        (
            self.undo.len() + self.redo.len(),
            self.undo.iter().chain(&self.redo).map(Change::bytes).sum(),
        )
    }
    pub(super) fn trim_history(&mut self) {
        while self.history_usage().0 > self.history_limits.0
            || self.history_usage().1 > self.history_limits.1
        {
            if self.undo.is_empty() {
                self.redo.remove(0);
            } else {
                self.undo.remove(0);
            }
            self.typing = None;
        }
    }
}
