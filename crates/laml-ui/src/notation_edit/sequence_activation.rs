//! Fuse a standalone activation start with an incoming message, retaining its end.
use super::*;
use laml::sequence::Event;

/// Join a before-start insertion and the consumed declaration into one patch.
/// Document transactions deliberately reject edits sharing the same byte start.
pub fn absorb_start(edits: &mut Vec<Edit>, removal: Edit) {
    if let Some(insertion) = edits.iter_mut().find(|e| e.range.is_empty() && e.range.start == removal.range.start) {
        insertion.range.end = removal.range.end;
    } else { edits.push(removal); }
}

pub fn consume_start(source: &str, line: usize, owner: &str) -> Result<Edit, String> {
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if doc.events.iter().any(|e| matches!(e, Event::Message(m) if m.line == line)) {
        return Err("This activation already starts with a message. Drop inside the bar instead.".into());
    }
    let (id, label) = doc.events.iter().find_map(|e| match e {
        Event::Activate { participant, id, label, line: at } if *at == line && participant == owner => Some((id, label)), _ => None,
    }).ok_or("The activation start no longer exists")?;
    if id.is_some() || label.is_some() {
        return Err("This activation has its own name or label. Keep that declaration and drop inside the bar, or start a new activation with the call.".into());
    }
    let record = record(source, line)?;
    let raw = &source[record.range.clone()];
    let body = code(raw);
    // An activation's inline comment remains a standalone comment at the same
    // source location; comments on the incoming message are preserved separately.
    if raw[body.trim_end().len()..].trim().is_empty() {
        Ok(Edit { range: record.block, replacement: String::new() })
    } else {
        let start = body.len() - body.trim_start().len();
        Ok(Edit { range: record.range.start + start..record.range.start + body.trim_end().len(), replacement: String::new() })
    }
}
