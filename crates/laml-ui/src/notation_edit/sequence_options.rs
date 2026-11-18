//! Backend feasibility of direct insertion and each next graphical reference.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub title: String,
    pub values: BTreeMap<String, String>,
    pub keys: Vec<(String, Vec<String>)>,
    pub before: usize,
    pub owner: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub edits: Option<Vec<Edit>>,
    pub next: Vec<String>,
}
pub fn edits(source: &str, query: &Query) -> Result<Vec<Edit>, String> {
    let recipe = super::sequence_recipes::recipes()
        .into_iter()
        .find(|r| r.title == query.title)
        .ok_or("Unknown sequence construct")?;
    if recipe.title == "Participant" {
        return sequence_controls::participant(
            source,
            query.values.get("id").ok_or("Missing ID")?,
            query.values.get("label").ok_or("Missing label")?,
            query.owner.as_deref().map(|owner| (owner, false)),
        );
    }
    if matches!(recipe.title, "Activation" | "Destroy participant") {
        let dock = sequence_controls::Dock {
            owner: query
                .values
                .get("participant")
                .ok_or("Choose a participant")?
                .clone(),
            before: query.before,
            boundary: None,
        };
        return sequence_scoped::insert(
            source,
            &dock,
            if recipe.title == "Activation" {
                sequence_insert::Action::Activate
            } else {
                sequence_insert::Action::Destroy
            },
            "",
            "",
        );
    }
    sequence_create::create(
        source,
        recipe,
        &query.values,
        query.before,
        query.owner.as_deref(),
    )
}
fn completion(
    source: &str,
    query: &mut Query,
    depth: usize,
    valid: &mut impl FnMut(&str) -> bool,
) -> Option<Vec<Edit>> {
    if let Some((key, choices)) = query.keys.get(depth).cloned() {
        for value in choices {
            query.values.insert(key.clone(), value);
            if let Some(edits) = completion(source, query, depth + 1, valid) {
                return Some(edits);
            }
        }
        None
    } else {
        let edits = edits(source, query).ok()?;
        if edits.is_empty() {
            return None;
        }
        let candidate = apply(source, &edits).ok()?;
        valid(&candidate).then_some(edits)
    }
}
pub fn build_with(
    source: &str,
    queries: &[Query],
    references: bool,
    mut valid: impl FnMut(&str) -> bool,
) -> Vec<Choice> {
    let mut out = Vec::new();
    for (index, query) in queries.iter().enumerate() {
        let mut query = query.clone();
        if references && !query.keys.is_empty() {
            let (key, values) = query.keys[0].clone();
            let mut next = Vec::new();
            for value in values {
                query.values.insert(key.clone(), value.clone());
                if completion(source, &mut query, 1, &mut valid).is_some() {
                    next.push(value);
                }
            }
            if !next.is_empty() {
                out.push(Choice {
                    index,
                    edits: None,
                    next,
                });
            }
        } else if let Some(edits) = completion(source, &mut query, 0, &mut valid) {
            out.push(Choice {
                index,
                edits: query.keys.is_empty().then_some(edits),
                next: vec![],
            });
        }
    }
    out
}
