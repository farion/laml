//! Resolve compiler-local occurrence lines back to source-qualified messages.
use laml::sequence::{Event, Message, SequenceDiagram};
pub fn message(doc: &SequenceDiagram, line: usize) -> Option<Message> {
    fn find(doc: &SequenceDiagram, line: usize, offset: isize, prefix: &str) -> Option<Message> {
        for event in &doc.events {
            if let Event::Message(m) = event
                && m.line.saturating_add_signed(offset) == line
            {
                let mut m = m.clone();
                m.line = line;
                if !prefix.is_empty() {
                    for owner in [&mut m.from, &mut m.to] {
                        if *owner != "?" && !owner.starts_with("gate[") {
                            *owner = format!("{prefix}.{owner}");
                        }
                    }
                }
                return Some(m);
            }
        }
        for d in &doc.decompositions {
            let prefix = if prefix.is_empty() {
                d.parent.clone()
            } else {
                format!("{prefix}.{}", d.parent)
            };
            if let Some(m) = find(&d.diagram, line, offset + d.line as isize - 5, &prefix) {
                return Some(m);
            }
        }
        None
    }
    find(doc, line, 0, "")
}
