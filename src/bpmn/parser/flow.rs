//! Inline gateways, attached events and the three BPMN flow kinds.
use super::{Context, Document, EdgeKind, Kind, add_edge, define, endpoint, error, token_end};
use crate::lex::{split_once_top_level, split_top_level};

fn find_outside<'a>(text: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let (mut quoted, mut escaped, mut depth) = (false, false, 0usize);
    for (i, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        if depth == 0 && text[i..].starts_with(marker) {
            return Some((&text[..i], &text[i + marker.len()..]));
        }
        if ch == '(' {
            depth += 1;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
        }
    }
    None
}
fn arrows(text: &str) -> Vec<(&str, Option<EdgeKind>)> {
    let mut parts = Vec::new();
    let (mut start, mut depth, mut quoted, mut escaped) = (0usize, 0usize, false, false);
    for (i, c) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
        } else if !quoted {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth = depth.saturating_sub(1);
            }
            if depth == 0 {
                let (arrow, kind) = if text[i..].starts_with("-->") {
                    ("-->", EdgeKind::Sequence)
                } else if text[i..].starts_with("~~>") {
                    ("~~>", EdgeKind::Message)
                } else if text[i..].starts_with("..>") {
                    ("..>", EdgeKind::Data)
                } else if text[i..].starts_with("..") {
                    ("..", EdgeKind::Association)
                } else {
                    continue;
                };
                parts.push((text[start..i].trim(), Some(kind)));
                start = i + arrow.len();
            }
        }
    }
    parts.push((text[start..].trim(), None));
    parts
}
pub(super) fn flow(
    doc: &mut Document,
    text: &str,
    ctx: &Context,
    line: usize,
) -> Result<(), String> {
    // An attached boundary event is drawn on the owner's border and has its own outgoing flow.
    if let Some((owner, rest)) = find_outside(text, " -(") {
        let (event, target) = rest
            .split_once(")->")
            .ok_or_else(|| error(line, "boundary event needs )->"))?;
        let owner = endpoint(doc, owner, ctx, line)?;
        let owner_node = doc.nodes.iter().find(|n| n.id == owner).unwrap();
        if !matches!(
            owner_node.kind,
            Kind::Task
                | Kind::UserTask
                | Kind::ServiceTask
                | Kind::SendTask
                | Kind::ReceiveTask
                | Kind::ScriptTask
                | Kind::ManualTask
                | Kind::Collapsed
                | Kind::Subprocess
        ) {
            return Err(error(line, "boundary event requires a task or subprocess"));
        }
        let (kind, event) = if let Some(rest) = event.strip_prefix("timer+") {
            ("timer+", rest)
        } else if let Some(rest) = event.strip_prefix("timer") {
            ("timer", rest)
        } else if let Some(rest) = event.strip_prefix("error") {
            ("error", rest)
        } else if let Some(rest) = event.strip_prefix("message+") {
            ("message+", rest)
        } else if let Some(rest) = event.strip_prefix("message") {
            ("message", rest)
        } else {
            return Err(error(line, "unsupported boundary event marker"));
        };
        let (name, label) = event
            .trim()
            .strip_prefix('@')
            .unwrap_or(event.trim())
            .split_once(':')
            .map_or((event.trim().trim_start_matches('@'), None), |(n, l)| {
                (n, Some(l.trim()))
            });
        let generated = format!("boundary_{line}");
        let id = define(
            doc,
            if name.is_empty() { &generated } else { name },
            Kind::Boundary,
            Some(kind.trim_end_matches('+')),
            ctx,
            line,
        )?;
        let boundary = doc.nodes.iter_mut().find(|n| n.id == id).unwrap();
        if boundary.attached.is_some() {
            return Err(error(line, "boundary event is already attached"));
        }
        boundary.attached = Some(owner);
        boundary.interrupting = !kind.ends_with('+');
        if let Some(label) = label {
            boundary.label = label.trim_matches('"').into();
        }
        let target = endpoint(doc, target, ctx, line)?;
        return add_edge(doc, id, target, EdgeKind::Sequence, None, line);
    }
    // The gateway marker creates an addressable diamond and the outgoing branches.
    for (marker, kind) in [
        ("<X>", Kind::Exclusive),
        ("<+>", Kind::Parallel),
        ("<O>", Kind::Inclusive),
        ("<event>", Kind::EventBased),
    ] {
        if let Some((left, tail)) = find_outside(text, marker) {
            if left.trim().is_empty() || left.contains("-->") {
                continue;
            }
            let source = endpoint(doc, left, ctx, line)?;
            let origin = doc.nodes.iter().find(|n| n.id == source).unwrap();
            let gateway_scope = Context {
                pool: origin.pool.clone(),
                lane: origin.lane.clone(),
                parent: origin.parent.clone(),
            };
            let tail = tail.trim();
            let (name, branches) = if let Some(named) = tail.strip_prefix('@') {
                let end = token_end(named);
                (&named[..end], named[end..].trim())
            } else {
                ("", tail)
            };
            let generated = format!("gateway_{line}");
            let id = define(
                doc,
                if name.is_empty() { &generated } else { name },
                kind,
                None,
                &gateway_scope,
                line,
            )?;
            add_edge(doc, source, id.clone(), EdgeKind::Sequence, None, line)?;
            let branches = split_top_level(branches, ';');
            if branches.len() < 2 || branches.iter().any(|b| b.trim().is_empty()) {
                return Err(error(line, "gateway split requires two or more branches"));
            }
            for branch in branches {
                let (target, label) = split_once_top_level(branch, ':')
                    .map_or((branch, None), |(t, l)| (t, Some(l.trim().to_owned())));
                if label.as_ref().is_some_and(String::is_empty) {
                    return Err(error(line, "empty branch label"));
                }
                let to = endpoint(doc, target, &gateway_scope, line)?;
                add_edge(doc, id.clone(), to, EdgeKind::Sequence, label, line)?;
            }
            return Ok(());
        }
    }
    let pieces = arrows(text);
    if pieces.len() > 1 {
        let mut from = endpoint(doc, pieces[0].0, ctx, line)?;
        for (i, (piece, _)) in pieces.iter().enumerate().skip(1) {
            let kind = pieces[i - 1].1.unwrap();
            let (target, label) = split_once_top_level(piece, ':')
                .map_or((*piece, None), |(t, l)| (t, Some(l.trim().to_owned())));
            let origin = doc.nodes.iter().find(|n| n.id == from).unwrap();
            let source_scope = Context {
                pool: origin.pool.clone(),
                lane: origin.lane.clone(),
                parent: origin.parent.clone(),
            };
            let scope = if matches!(kind, EdgeKind::Sequence) && target.trim().starts_with('<') {
                &source_scope
            } else {
                ctx
            };
            let to = endpoint(doc, target, scope, line)?;
            add_edge(doc, from, to.clone(), kind, label, line)?;
            from = to;
        }
    } else {
        endpoint(doc, text, ctx, line)?;
    }
    Ok(())
}
