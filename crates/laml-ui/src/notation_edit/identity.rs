//! Identity edits update exact references, never labels, comments or substrings.
use super::*;
pub fn rename(source: &str, id: &str, line: usize, new: &str) -> Result<Vec<Edit>, String> {
    if !crate::graph_edit::identifier(new) {
        return Err(
            "Use an identifier: letters, digits and underscores, without a leading digit".into(),
        );
    }
    let ty = crate::notation::Notation::of(source);
    let c4_classifier = ty == crate::notation::Notation::C4Code
        && crate::notation::parse_graph(source).is_ok_and(|d| {
            d.nodes
                .iter()
                .any(|n| n.id == id && !n.classes.iter().any(|c| c.starts_with("c4")))
        });
    if ty.graph() && (!ty.c4() || c4_classifier) {
        return crate::node_edit::rename_id(source, id, new);
    }
    let old_short = id.rsplit('.').next().unwrap();
    let mut ids: Vec<String> = match ty {
        crate::notation::Notation::Bpmn => {
            let d = laml::bpmn::parse(source)?;
            d.nodes
                .into_iter()
                .map(|n| n.id)
                .chain(d.pools.into_iter().flat_map(|p| {
                    let id = p.id.clone();
                    std::iter::once(p.id)
                        .chain(p.lanes.into_iter().map(move |l| format!("{id}.{}", l.id)))
                }))
                .collect()
        }
        crate::notation::Notation::Sequence => {
            fn collect(doc: &laml::sequence::SequenceDiagram, prefix: &str, out: &mut Vec<String>) {
                out.extend(doc.participants.iter().map(|p| format!("{prefix}{}", p.id)));
                for event in &doc.events {
                    let id = match event {
                        laml::sequence::Event::Message(m) => m.name.as_deref(),
                        laml::sequence::Event::Note(n) => Some(n.id.as_str()),
                        laml::sequence::Event::Continuation(c) => Some(c.id.as_str()),
                        laml::sequence::Event::FrameStart { id, .. }
                        | laml::sequence::Event::Activate { id, .. }
                        | laml::sequence::Event::Deactivate { id, .. } => id.as_deref(),
                        _ => None,
                    };
                    if let Some(id) = id {
                        out.push(format!("{prefix}{id}"));
                    }
                }
                for part in &doc.decompositions {
                    collect(&part.diagram, &format!("{prefix}{}.", part.parent), out);
                }
            }
            let doc = laml::sequence::parse(source).map_err(|e| e.message)?;
            let mut out = Vec::new();
            collect(&doc, "", &mut out);
            out
        }
        crate::notation::Notation::Timing => laml::timing::parse(source)
            .map_err(|e| e.message)?
            .lifelines
            .into_iter()
            .map(|p| p.id)
            .collect(),
        _ => crate::notation::parse_graph(source)
            .map_err(|e| e.message)?
            .nodes
            .into_iter()
            .map(|n| n.id)
            .collect(),
    };
    ids.sort();
    ids.dedup();
    let new_id = id
        .rsplit_once('.')
        .map_or(new.to_owned(), |(prefix, _)| format!("{prefix}.{new}"));
    if ids.iter().any(|s| s == &new_id && s != id) {
        return Err("The new ID already exists".into());
    }
    let unique_short = ids
        .iter()
        .filter(|s| s.rsplit('.').next() == Some(old_short))
        .count()
        == 1;
    let mut edits = Vec::new();
    for r in records(source) {
        let raw = code(&source[r.range.clone()]);
        let scope = super::references::scope(source, &r, ty);
        for span in
            super::references::spans(raw, &r.section, ty == crate::notation::Notation::Timing)
        {
            let value = &raw[span.clone()];
            let qualified = format!("{scope}.{value}");
            let canonical = if !scope.is_empty() && ids.iter().any(|s| s == &qualified) {
                qualified.as_str()
            } else {
                value
            };
            let replacement = if canonical == id {
                Some(if value == old_short {
                    new.to_owned()
                } else {
                    new_id.clone()
                })
            } else if value.starts_with(&format!("{id}.")) || value.starts_with(&format!("{id}::"))
            {
                Some(format!("{new_id}{}", &value[id.len()..]))
            } else if (unique_short
                || r.line == line
                || canonical == qualified
                    && scope == id.rsplit_once('.').map_or("", |(scope, _)| scope))
                && (value == old_short || value.starts_with(&format!("{old_short}::")))
            {
                Some(format!("{new}{}", &value[old_short.len()..]))
            } else {
                None
            };
            if let Some(replacement) = replacement {
                edits.push(Edit {
                    range: r.range.start + span.start..r.range.start + span.end,
                    replacement,
                });
            }
        }
    }
    if edits.is_empty() {
        return Err("Cannot locate this element's source identity".into());
    }
    let preserve_label = match ty {
        crate::notation::Notation::Bpmn => laml::bpmn::parse(source)?
            .nodes
            .iter()
            .any(|n| n.id == id && n.label == old_short),
        crate::notation::Notation::Sequence => laml::sequence::parse(source)
            .map_err(|e| e.message)?
            .participants
            .iter()
            .any(|p| p.id == id && p.created_line.is_none() && p.label == old_short),
        ty if ty.c4() => crate::notation::parse_graph(source)
            .map_err(|e| e.message)?
            .nodes
            .iter()
            .any(|n| n.id == id && n.classes.iter().any(|c| c.starts_with("c4"))),
        _ => false,
    };
    if preserve_label && !fields::of(source, line).iter().any(|f| f.name == "Label") {
        edits.extend(attribute(source, line, Some(id), "label", old_short)?);
    }
    let candidate = apply(source, &edits)?;
    crate::notation::inspect(&candidate)?;
    Ok(edits)
}
