//! Timing lanes are independent lifeline groups; numeric event times never change
//! when the user reorders lanes vertically.
use super::*;
use std::collections::BTreeMap;
pub fn move_lane(source: &str, id: &str, target: &str, before: bool) -> Result<Vec<Edit>, String> {
    let doc = laml::timing::parse(source).map_err(|e| e.message)?;
    let from = doc
        .lifelines
        .iter()
        .position(|p| p.id == id)
        .ok_or("Missing timing lifeline")?;
    let to = doc
        .lifelines
        .iter()
        .position(|p| p.id == target)
        .ok_or("Missing target lifeline")?;
    if from == to {
        return Ok(vec![]);
    }
    let mut order: Vec<_> = (0..doc.lifelines.len()).collect();
    order.retain(|i| *i != from);
    let at = order.iter().position(|i| *i == to).unwrap() + usize::from(!before);
    order.insert(at, from);
    let mut owners = BTreeMap::new();
    for (i, lane) in doc.lifelines.iter().enumerate() {
        owners.insert(lane.line, i);
        for change in &lane.changes {
            owners.insert(change.line, i);
        }
        for event in &lane.events {
            owners.insert(event.line, i);
        }
    }
    let entries: Vec<_> = records(source)
        .into_iter()
        .filter(|r| r.section == "semantics")
        .collect();
    let first = entries
        .first()
        .ok_or("Missing timing semantics")?
        .block
        .start;
    let end = entries.last().unwrap().block.end;
    let mut groups = vec![String::new(); doc.lifelines.len()];
    let mut previous = first;
    for r in entries {
        let owner = owners.get(&r.line).ok_or("Unknown timing construct")?;
        groups[*owner].push_str(&source[previous..r.block.end]);
        previous = r.block.end;
    }
    let replacement = order
        .iter()
        .map(|i| groups[*i].as_str())
        .collect::<String>();
    let edits = vec![Edit {
        range: first..end,
        replacement,
    }];
    let candidate = apply(source, &edits)?;
    laml::timing::parse(&candidate).map_err(|e| e.message)?;
    Ok(edits)
}
pub fn adjacent_lane(source: &str, line: usize, down: bool) -> Result<Vec<Edit>, String> {
    let doc = laml::timing::parse(source).map_err(|e| e.message)?;
    let at =
        doc.lifelines.iter().position(|p| p.line == line).ok_or(
            "Select a lifeline to reorder lanes; change numeric times to move occurrences",
        )?;
    let other = if down {
        doc.lifelines.get(at + 1)
    } else {
        at.checked_sub(1).and_then(|i| doc.lifelines.get(i))
    }
    .ok_or("Already at the end")?;
    move_lane(source, &doc.lifelines[at].id, &other.id, !down)
}
