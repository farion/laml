//! Relative placement planning. Only conflicting rules incident to the moved node
//! on the edited axis may be removed. Never rewrite semantic relationships.
use crate::{
    document::{Document, Edit},
    source_patch::{insert_section, remove_line},
};
use laml::syntax::{LayoutRule, Placement};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct PlacementPlan {
    pub edits: Vec<Edit>,
    pub removed: Vec<String>,
    pub preserved: Vec<String>,
    pub added: String,
    pub source: String,
}

pub fn description(from: &str, target: &str, side: Placement) -> String {
    let operator = match side {
        Placement::Left => "<",
        Placement::Right => ">",
        Placement::Above => "above",
        Placement::Below => "below",
    };
    format!("{from} {operator} {target}")
}
fn horizontal(side: Placement) -> bool {
    matches!(side, Placement::Left | Placement::Right)
}
fn edge(rule: &LayoutRule) -> (&str, &str) {
    match rule.placement {
        Placement::Left | Placement::Above => (&rule.first, &rule.second),
        Placement::Right | Placement::Below => (&rule.second, &rule.first),
    }
}
fn reachable(edges: &[(&str, &str)], from: &str, target: &str) -> bool {
    let mut stack = vec![from];
    let mut visited = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        if visited.insert(node) {
            stack.extend(edges.iter().filter(|(a, _)| *a == node).map(|(_, b)| *b));
        }
    }
    false
}

pub fn plan(
    source: &str,
    from: &str,
    target: &str,
    side: Placement,
) -> Result<PlacementPlan, String> {
    if from == target {
        return Err("A node cannot be placed relative to itself".into());
    }
    let diagram = crate::notation::parse_graph(source).map_err(|e| e.message)?;
    let moved = diagram
        .nodes
        .iter()
        .find(|node| node.id == from)
        .ok_or("Unknown moved node")?;
    let reference = diagram
        .nodes
        .iter()
        .find(|node| node.id == target)
        .ok_or("Unknown reference node")?;
    // This gesture means relative positioning, never moving into/out of a container.
    if moved.parent != reference.parent {
        return Err("Relative drops currently require nodes in the same container. Membership stays unchanged.".into());
    }
    laml::layout::validate_ordering(&diagram).map_err(|e| e.message)?;
    let rule = LayoutRule {
        first: from.into(),
        second: target.into(),
        placement: side,
        align: None,
        gap: None,
        line: 0,
    };
    let (new_before, new_after) = edge(&rule);
    let rules: Vec<_> = diagram
        .layout
        .iter()
        .filter(|r| horizontal(r.placement) == horizontal(side))
        .collect();
    let edges: Vec<_> = rules.iter().map(|r| edge(r)).collect();
    let mut edits = Vec::new();
    let mut removed = Vec::new();
    let mut preserved = Vec::new();
    for existing in &rules {
        let (a, b) = edge(existing);
        // A new target->moved edge can only contradict paths moved->...->target,
        // and vice versa. Cut only the moved node's edge on those paths.
        let conflicts = if new_before == from {
            b == from && reachable(&edges, new_after, a)
        } else {
            a == from && reachable(&edges, b, new_before)
        };
        if !conflicts {
            continue;
        }
        if let Some(align) = crate::alignment_edit::attached(existing) {
            let text = crate::alignment_edit::declaration(&existing.first, &existing.second, align);
            edits.push(crate::source_patch::replace_code(
                source,
                existing.line,
                text.clone(),
            )?);
            preserved.push(text);
        } else {
            edits.push(remove_line(source, existing.line)?);
        }
        removed.push(
            crate::style_edit::code(
                source
                    .lines()
                    .nth(existing.line - 1)
                    .ok_or("Missing layout source line")?,
            )
            .trim()
            .into(),
        );
    }
    for existing in &diagram.alignments {
        if existing.first != from && existing.second != from {
            continue;
        }
        let (ax, ay) = existing.align.anchors();
        let Some(anchor) = (if horizontal(side) { ax } else { ay }) else {
            continue;
        };
        let other = if existing.first == from {
            &existing.second
        } else {
            &existing.first
        };
        if !crate::alignment_edit::connected_without(
            &diagram,
            from,
            other,
            target,
            horizontal(side),
            anchor,
        ) {
            continue;
        }
        let keep =
            crate::alignment_edit::remaining(existing.align, horizontal(side), !horizontal(side));
        removed.push(
            crate::style_edit::code(
                source
                    .lines()
                    .nth(existing.line - 1)
                    .ok_or("Missing alignment line")?,
            )
            .trim()
            .into(),
        );
        if let Some(keep) = keep {
            let text = crate::alignment_edit::declaration(&existing.first, &existing.second, keep);
            edits.push(crate::source_patch::replace_code(
                source,
                existing.line,
                text.clone(),
            )?);
            preserved.push(text);
        } else {
            edits.push(remove_line(source, existing.line)?);
        }
    }
    for existing in &diagram.layout {
        if horizontal(existing.placement) == horizontal(side)
            || (existing.first != from && existing.second != from)
        {
            continue;
        }
        let Some(align) = crate::alignment_edit::attached(existing) else {
            continue;
        };
        let (x, y) = align.anchors();
        let Some(anchor) = (if horizontal(side) { x } else { y }) else {
            continue;
        };
        let other = if existing.first == from {
            &existing.second
        } else {
            &existing.first
        };
        if !crate::alignment_edit::connected_without(
            &diagram,
            from,
            other,
            target,
            horizontal(side),
            anchor,
        ) {
            continue;
        }
        let text = crate::alignment_edit::without_alignment(existing);
        edits.push(crate::source_patch::replace_code(
            source,
            existing.line,
            text.clone(),
        )?);
        removed.push(
            crate::style_edit::code(
                source
                    .lines()
                    .nth(existing.line - 1)
                    .ok_or("Missing alignment line")?,
            )
            .trim()
            .into(),
        );
        preserved.push(text);
    }
    let added = description(from, target, side);
    // An already-authored equivalent rule (including its gap/alignment) stays intact.
    if !rules
        .iter()
        .any(|existing| edge(existing) == (new_before, new_after))
    {
        edits.push(insert_section(source, &diagram, "layout", &added)?);
    }
    let mut document = Document::new(source.into());
    document
        .apply(0, "Relative placement", edits.clone())
        .map_err(|e| format!("Invalid edit plan: {e:?}"))?;
    let changed = crate::notation::parse_graph(document.source()).map_err(|e| e.message)?;
    laml::layout::validate_ordering(&changed).map_err(|e| e.message)?;
    Ok(PlacementPlan {
        edits,
        removed,
        preserved,
        added,
        source: document.source().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str =
        "semantics {\n A\n B\n C\n D\n}\nlayout {\n A < B // old order\n B < C\n A above D\n}\n";
    #[test]
    fn reorder_preserves_other_axis_unrelated_rules_and_comments() {
        let plan = plan(SOURCE, "A", "C", Placement::Right).unwrap();
        assert_eq!(plan.removed, ["A < B"]);
        assert!(plan.source.contains(" // old order\n"));
        assert!(plan.source.contains(" B < C\n A above D\n"));
        assert!(plan.source.contains("A > C"));
        assert_eq!(
            crate::notation::parse_graph(&plan.source).unwrap().relations.len(),
            0
        );
    }
    #[test]
    fn removes_all_conflicting_paths_not_all_rules_on_axis() {
        let source = SOURCE.replace(" A above D", " A < D\n D < C\n A above D");
        let first = plan(&source, "A", "C", Placement::Right).unwrap();
        assert_eq!(first.removed, ["A < B", "A < D"]);
        let source = SOURCE.replace(" A above D", " A < D\n A above D");
        let plan = plan(&source, "A", "C", Placement::Right).unwrap();
        assert!(plan.source.contains(" A < D\n"));
    }
    #[test]
    fn alignment_on_other_axis_is_not_silently_lost() {
        let source = SOURCE.replace("A < B //", "A < B : align=center //");
        let plan = plan(&source, "A", "C", Placement::Right).unwrap();
        assert!(plan.source.contains("A ? B : align=center-y // old order"));
        assert_eq!(plan.preserved, ["A ? B : align=center-y"]);
    }
    #[test]
    fn equivalent_rule_is_noop_and_reverse_spellings_are_detected() {
        assert!(
            plan(SOURCE, "B", "A", Placement::Right)
                .unwrap()
                .edits
                .is_empty()
        );
        let source = SOURCE.replace("A < B", "B > A");
        assert_eq!(
            plan(&source, "A", "C", Placement::Right).unwrap().removed,
            ["B > A"]
        );
    }
    #[test]
    fn vertical_reorder_keeps_horizontal_rules_and_crlf() {
        let source = SOURCE
            .replace(" A < B", " A above B")
            .replace(" B < C", " B above C")
            .replace(" A above D", " A < D")
            .replace('\n', "\r\n");
        let plan = plan(&source, "A", "C", Placement::Below).unwrap();
        assert!(plan.source.contains(" A < D\r\n"));
        assert!(plan.source.contains("A below C\r\n"));
        assert_eq!(plan.removed, ["A above B"]);
    }
}
