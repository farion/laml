//! Byte-accurate line lookup without normalizing whitespace or line endings.
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct SourceIndex {
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl SourceIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(offset + 1);
            }
        }
        let ends = starts
            .iter()
            .enumerate()
            .map(|(index, start)| {
                let mut end = starts.get(index + 1).copied().unwrap_or(source.len());
                if end > *start && source.as_bytes()[end - 1] == b'\n' {
                    end -= 1;
                    if end > *start && source.as_bytes()[end - 1] == b'\r' {
                        end -= 1;
                    }
                }
                end
            })
            .collect();
        Self { starts, ends }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// One-based line containing a byte offset; offsets beyond EOF are rejected.
    pub fn line_at(&self, offset: usize) -> Option<usize> {
        if offset > *self.ends.last()? {
            return None;
        }
        Some(self.starts.partition_point(|start| *start <= offset))
    }

    /// Compiler line numbers are one-based. Returned ranges exclude CRLF/LF.
    pub fn line(&self, one_based: usize) -> Option<Range<usize>> {
        let index = one_based.checked_sub(1)?;
        Some(*self.starts.get(index)?..*self.ends.get(index)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexes_utf8_crlf_and_empty_last_line() {
        let text = "ä\r\nB\n";
        let index = SourceIndex::new(text);
        assert_eq!(index.line(1), Some(0..2));
        assert_eq!(index.line(2), Some(4..5));
        assert_eq!(index.line(3), Some(6..6));
        assert_eq!(index.line(0), None);
        assert_eq!(index.line(4), None);
        assert_eq!(SourceIndex::new("").line(1), Some(0..0));
    }
}
