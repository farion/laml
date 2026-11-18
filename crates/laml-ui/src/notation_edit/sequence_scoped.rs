//! Scope adapters for sequence operations that already validate source semantics.
use super::sequence_scope::Scope;
use super::*;
type Movement = Result<(Vec<Edit>, usize), String>;
pub fn reorder(source: &str, owner: &str, before: Option<&str>) -> Result<Vec<Edit>, String> {
    if let Some(s) = Scope::of(source, owner)? {
        let before = before.map(|b| s.owner(b)).transpose()?;
        let edits =
            super::sequence_participants::reorder(&s.source, &s.owner(owner)?, before.as_deref())?;
        s.commit(source, &edits)
    } else {
        super::sequence_participants::reorder(source, owner, before)
    }
}
#[allow(clippy::too_many_arguments)]
pub fn resize(
    source: &str,
    owner: &str,
    start: usize,
    edge: usize,
    lower: bool,
    open: bool,
    before: usize,
) -> Movement {
    if let Some(s) = Scope::of(source, owner)? {
        let (edits, line) = super::sequence_align::resize(
            &s.source,
            &s.owner(owner)?,
            s.line(start),
            s.line(edge),
            lower,
            open,
            s.line(before),
        )?;
        Ok((s.commit(source, &edits)?, s.absolute(line)))
    } else {
        super::sequence_align::resize(source, owner, start, edge, lower, open, before)
    }
}
pub fn align(
    source: &str,
    owner: &str,
    start: usize,
    target: &str,
    target_start: usize,
    lower: bool,
) -> Movement {
    if let Some(s) = Scope::of(source, owner)? {
        let (edits, line) = super::sequence_align::align(
            &s.source,
            &s.owner(owner)?,
            s.line(start),
            &s.owner(target)?,
            s.line(target_start),
            lower,
        )?;
        Ok((s.commit(source, &edits)?, s.absolute(line)))
    } else {
        super::sequence_align::align(source, owner, start, target, target_start, lower)
    }
}
#[allow(clippy::too_many_arguments)]
pub fn snap(
    source: &str,
    owner: &str,
    start: usize,
    edge: usize,
    lower: bool,
    message: usize,
    receive: bool,
) -> Movement {
    if let Some(s) = Scope::of(source, owner)? {
        let (edits, line) = super::sequence_align::snap_to_occurrence(
            &s.source,
            &s.owner(owner)?,
            s.line(start),
            s.line(edge),
            lower,
            s.line(message),
            receive,
        )?;
        Ok((s.commit(source, &edits)?, s.absolute(line)))
    } else {
        super::sequence_align::snap_to_occurrence(
            source, owner, start, edge, lower, message, receive,
        )
    }
}
pub fn insert(
    source: &str,
    dock: &super::sequence_controls::Dock,
    action: super::sequence_insert::Action,
    id: &str,
    label: &str,
) -> Result<Vec<Edit>, String> {
    if let Some(s) = Scope::of(source, &dock.owner)? {
        let dock = s.dock(dock)?;
        let edits = insert(&s.source, &dock, action, id, label)?;
        s.commit(source, &edits)
    } else {
        super::sequence_insert::create(
            source,
            dock.before,
            &dock.owner,
            action,
            "",
            id,
            label,
            None,
            None,
            None,
        )
    }
}
pub fn move_event(source: &str, line: usize, before: usize, owner: Option<&str>) -> Movement {
    if let Some(owner) = owner
        && let Some(s) = Scope::of(source, owner)?
    {
        let (edits, line) = move_event(&s.source, s.line(line), s.line(before), None)?;
        return Ok((s.commit(source, &edits)?, s.absolute(line)));
    }
    let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
    if doc
        .events
        .iter()
        .any(|e| matches!(e,laml::sequence::Event::Message(m) if m.line==line))
    {
        super::sequence_docking::move_message(source, line, before)
    } else {
        super::sequence_drag::move_event(source, line, before)
    }
}
