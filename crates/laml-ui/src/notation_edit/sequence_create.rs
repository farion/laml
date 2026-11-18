//! Direct sequence insertion, with atomic naming of graphically selected references.
use super::*;
use std::collections::BTreeMap;

pub fn create(
    source: &str,
    recipe: catalog::Recipe,
    values: &BTreeMap<String, String>,
    before: usize,
    owner: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if let Some(owner) = owner
        && let Some(scope) = sequence_scope::Scope::of(source, owner)?
    {
        let mut local = values.clone();
        let prefix = owner.rsplit_once('.').unwrap().0;
        for key in catalog::parameters(recipe) {
            if let Some(value) = local.get_mut(&key) {
                if let Some(anchor) = value.strip_prefix('@') {
                    let (line, end) = anchor.split_once('.').ok_or("Invalid occurrence")?;
                    *value = format!(
                        "@{}.{end}",
                        scope.line(line.parse().map_err(|_| "Invalid occurrence")?)
                    );
                } else if reference(&key) {
                    *value = value
                        .strip_prefix(&format!("{prefix}."))
                        .ok_or("Choose references in the same interaction")?
                        .into();
                }
            }
        }
        let edits = create(
            &scope.source,
            recipe,
            &local,
            scope.line(before),
            Some(&scope.owner(owner)?),
        )?;
        return scope.commit(source, &edits);
    }
    let mut draft = source.to_owned();
    let mut resolved = values.clone();
    for value in resolved.values_mut() {
        if let Some(anchor) = value.strip_prefix('@') {
            let (line, end) = anchor.split_once('.').ok_or("Invalid occurrence")?;
            let line: usize = line.parse().map_err(|_| "Invalid occurrence")?;
            let doc = laml::sequence::parse(&draft).map_err(|e| e.message)?;
            if end == "frame" {
                let name = fresh(&draft, "Frame");
                let range = SourceIndex::new(&draft).line(line).ok_or("Missing frame")?;
                let raw = &draft[range.clone()];
                let at = range.start + raw.len() - raw.trim_start().len();
                draft.insert_str(at, &format!("{name}: "));
                *value = name;
                continue;
            }
            let message = doc
                .events
                .iter()
                .find_map(|e| match e {
                    laml::sequence::Event::Message(m) if m.line == line => Some(m),
                    _ => None,
                })
                .ok_or("Choose a message endpoint")?;
            let name = if let Some(name) = &message.name {
                name.clone()
            } else {
                let name = fresh(&draft, "Message");
                let range = SourceIndex::new(&draft)
                    .line(line)
                    .ok_or("Missing message")?;
                let raw = &draft[range.clone()];
                let at = range.start + raw.len() - raw.trim_start().len();
                draft.insert_str(at, &format!("{name}: "));
                name
            };
            *value = format!("{name}.{end}");
        }
    }
    let body = catalog::instantiate(recipe, &resolved)?;
    let mut edits = if recipe.section == "semantics" {
        sequence_insert::at(&draft, before, &body)?
    } else if matches!(
        recipe.title,
        "Note placement" | "Continuation placement" | "Frame coverage"
    ) {
        let prefix = body
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(r) = records(&draft)
            .into_iter()
            .find(|r| r.section == "layout" && r.heading.starts_with(&format!("{prefix} ")))
        {
            let raw = &draft[r.range.clone()];
            let text = code(raw).trim();
            let start = r.range.start + raw.find(text).unwrap();
            vec![Edit {
                range: start..start + text.len(),
                replacement: body,
            }]
        } else {
            insert(&draft, recipe.section, None, None, &body)?
        }
    } else {
        insert(&draft, recipe.section, None, None, &body)?
    };
    if !recipe.layout.is_empty() {
        let layout = catalog::instantiate(
            catalog::Recipe {
                body: recipe.layout,
                layout: "",
                ..recipe
            },
            &resolved,
        )?;
        edits.extend(insert(&draft, "layout", None, None, &layout)?);
    }
    let edits = crate::element_edit::merge_insertions(edits);
    let candidate = apply(&draft, &edits)?;
    laml::sequence::parse(&candidate).map_err(|e| e.message)?;
    Ok(crate::graph_edit::difference(source, &candidate))
}
pub fn fresh(source: &str, prefix: &str) -> String {
    (1..)
        .map(|i| format!("{prefix}{i}"))
        .find(|id| !source.contains(id))
        .unwrap()
}
pub fn reference(key: &str) -> bool {
    matches!(
        key,
        "participant"
            | "from"
            | "to"
            | "occurrence"
            | "first"
            | "second"
            | "frame"
            | "note"
            | "continuation"
    )
}
