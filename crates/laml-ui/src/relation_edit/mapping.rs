//! Remap declaration lines through targeted patches, including sections before semantics.
use crate::{document::Edit, source::SourceIndex};

pub fn remap_line(source: &str, line: usize, edits: &[Edit]) -> usize {
    let Some(range) = SourceIndex::new(source).line(line) else {
        return line;
    };
    let mut mapped = line as isize;
    for edit in edits {
        if edit.range.end <= range.start {
            mapped += edit.replacement.bytes().filter(|b| *b == b'\n').count() as isize;
            mapped -= source[edit.range.clone()]
                .bytes()
                .filter(|b| *b == b'\n')
                .count() as isize;
        }
    }
    mapped.max(1) as usize
}
