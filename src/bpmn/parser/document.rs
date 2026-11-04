//! BPMN section and scope parser.
use super::flow::flow;
use super::{Context, Document, Kind, Lane, Pool, endpoint, error, name_args};
use crate::bpmn::{EdgeKind, FlowPortRule, LabelOrientation, Side};
use crate::lex::{split_top_level, without_comment};

fn layout(doc: &mut Document, text: &str, line: usize) -> Result<(), String> {
    if let Some(rest) = text.strip_prefix("flow[") {
        let (endpoints, tail) = rest
            .split_once(']')
            .ok_or_else(|| error(line, "flow layout needs a closing ']'"))?;
        let (from, to) = endpoints
            .split_once("->")
            .ok_or_else(|| error(line, "flow layout expects source->target"))?;
        let properties = tail
            .trim()
            .strip_prefix(':')
            .ok_or_else(|| error(line, "flow layout needs ':' and side properties"))?;
        let (mut from_side, mut to_side, mut label_orientation) = (None, None, None);
        for property in split_top_level(properties, ';') {
            let (key, value) = property
                .trim()
                .split_once('=')
                .ok_or_else(|| error(line, "flow layout expects key=value"))?;
            let value = value.trim();
            if key.trim() == "label-orientation" {
                let orientation = match value {
                    "auto" => LabelOrientation::Auto,
                    "horizontal" => LabelOrientation::Horizontal,
                    "vertical" => LabelOrientation::Vertical,
                    _ => {
                        return Err(error(
                            line,
                            "label-orientation must be auto, horizontal or vertical",
                        ));
                    }
                };
                if label_orientation.replace(orientation).is_some() {
                    return Err(error(line, "duplicate label-orientation property"));
                }
                continue;
            }
            let side = match value {
                "left" => Side::Left,
                "top" => Side::Top,
                "right" => Side::Right,
                "bottom" => Side::Bottom,
                _ => {
                    return Err(error(
                        line,
                        "flow attachment side must be left, top, right or bottom",
                    ));
                }
            };
            match key.trim() {
                "from" if from_side.replace(side).is_none() => {}
                "to" if to_side.replace(side).is_none() => {}
                _ => return Err(error(line, "duplicate or unknown flow attachment property")),
            }
        }
        if from_side.is_none() && to_side.is_none() && label_orientation.is_none() {
            return Err(error(line, "empty flow attachment rule"));
        }
        doc.flow_layout.push(FlowPortRule {
            from: from.trim().into(),
            to: to.trim().into(),
            from_side,
            to_side,
            label_orientation,
        });
        return Ok(());
    }
    let parts: Vec<_> = text.split_whitespace().collect();
    if parts.len() != 3 || !matches!(parts[1], "<" | ">" | "above" | "below") {
        return Err(error(
            line,
            "BPMN layout expects A < B, A > B, A above B or A below B",
        ));
    }
    doc.layout
        .push((parts[0].into(), parts[1].into(), parts[2].into()));
    Ok(())
}
pub fn is_bpmn_source(source: &str) -> bool {
    let mut settings = false;
    for line in source.lines() {
        let text = without_comment(line).trim();
        if text == "settings {" {
            settings = true;
        } else if settings && text == "}" {
            settings = false;
        } else if settings && text == "type: bpmn" {
            return true;
        }
    }
    false
}
pub fn parse(source: &str) -> Result<Document, String> {
    let (stripped, theme, _, _, _) = crate::theme::preprocess_settings(source)?;
    let source = stripped.as_str();
    if !is_bpmn_source(source) {
        return Err("settings { type: bpmn } required".into());
    }
    let mut doc = Document {
        theme,
        ..Document::default()
    };
    let mut scopes: Vec<(String, Context)> = Vec::new();
    let mut stylesheet = String::new();
    let mut style_depth = 0usize;
    let mut seen_sections = std::collections::HashSet::new();
    let mut seen_diagonal_setting = false;
    for (offset, raw) in source.lines().enumerate() {
        let line = offset + 1;
        let text = without_comment(raw).trim();
        if style_depth > 0 {
            stylesheet.push_str(raw);
            stylesheet.push('\n');
            if text.ends_with('{') {
                style_depth += 1;
            }
            if text == "}" {
                style_depth -= 1;
            }
            if style_depth == 0 {
                scopes.pop();
            }
            continue;
        }
        if text.is_empty() {
            continue;
        }
        let ctx = scopes.last().map(|(_, c)| c.clone()).unwrap_or_default();
        if let Some(header) = text.strip_suffix(" {") {
            if scopes.is_empty()
                && matches!(header, "settings" | "semantics" | "layout" | "styling")
            {
                if !seen_sections.insert(header) {
                    return Err(error(line, "duplicate BPMN section"));
                }
                scopes.push((header.into(), Context::default()));
                if header == "styling" {
                    stylesheet.push_str(raw);
                    stylesheet.push('\n');
                    style_depth = 1;
                }
            } else if scopes.iter().any(|(s, _)| s == "semantics") {
                if let Some(name) = header.strip_prefix("pool ") {
                    if ctx.pool.is_some() {
                        return Err(error(line, "nested pools are not BPMN"));
                    }
                    let (id, label, icon) = name_args(name, line)?;
                    if icon.is_some() || doc.pools.iter().any(|p| p.id == id) {
                        return Err(error(line, "invalid or duplicate pool"));
                    }
                    doc.pools.push(Pool {
                        label: label.unwrap_or_else(|| id.clone()),
                        id: id.clone(),
                        lanes: vec![],
                    });
                    scopes.push((
                        "pool".into(),
                        Context {
                            pool: Some(id),
                            ..ctx
                        },
                    ));
                } else if let Some(name) = header.strip_prefix("lane ") {
                    if ctx.pool.is_none() || ctx.lane.is_some() || ctx.parent.is_some() {
                        return Err(error(line, "lane requires one pool"));
                    }
                    let (id, label, icon) = name_args(name, line)?;
                    if icon.is_some() {
                        return Err(error(line, "lane cannot carry an icon"));
                    }
                    let pool = doc
                        .pools
                        .iter_mut()
                        .find(|p| Some(&p.id) == ctx.pool.as_ref())
                        .unwrap();
                    if pool.lanes.iter().any(|l| l.id == id) {
                        return Err(error(line, "duplicate lane"));
                    }
                    pool.lanes.push(Lane {
                        label: label.unwrap_or_else(|| id.clone()),
                        id: id.clone(),
                    });
                    scopes.push((
                        "lane".into(),
                        Context {
                            lane: Some(id),
                            ..ctx
                        },
                    ));
                } else if ctx.parent.is_none() {
                    let id = endpoint(&mut doc, header, &ctx, line)?;
                    let n = doc.nodes.iter_mut().find(|n| n.id == id).unwrap();
                    if n.kind != Kind::Task {
                        return Err(error(
                            line,
                            "expanded subprocess must be a new task-like element",
                        ));
                    }
                    n.kind = Kind::Subprocess;
                    let parent = id.split('.').next_back().unwrap().to_owned();
                    scopes.push((
                        "subprocess".into(),
                        Context {
                            parent: Some(parent),
                            ..ctx
                        },
                    ));
                } else {
                    return Err(error(line, "nested expanded subprocess not supported yet"));
                }
            } else {
                return Err(error(line, "unsupported BPMN block"));
            }
        } else if text == "}" {
            if scopes.pop().is_none() {
                return Err(error(line, "unexpected closing brace"));
            }
        } else {
            let Some((section, _)) = scopes.last() else {
                return Err(error(line, "declaration outside section"));
            };
            match section.as_str() {
                "settings" if text == "type: bpmn" => {}
                "settings" if text.starts_with("allow-diagonal:") => {
                    if seen_diagonal_setting {
                        return Err(error(line, "duplicate allow-diagonal setting"));
                    }
                    doc.allow_diagonal = match text.trim_start_matches("allow-diagonal:").trim() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(error(line, "allow-diagonal must be true or false")),
                    };
                    seen_diagonal_setting = true;
                }
                "settings" if text == "direction: left-to-right" => {
                    doc.direction = crate::syntax::FlowDirection::LeftToRight;
                }
                "settings" if text == "direction: top-to-bottom" => {
                    doc.direction = crate::syntax::FlowDirection::TopToBottom;
                }
                "settings" => return Err(error(line, "unsupported BPMN setting")),
                "layout" => layout(&mut doc, text, line)?,
                "semantics" | "pool" | "lane" | "subprocess" => flow(&mut doc, text, &ctx, line)?,
                _ => return Err(error(line, "unsupported BPMN declaration")),
            }
        }
    }
    if !scopes.is_empty() {
        return Err("unclosed BPMN block".into());
    }
    if doc.nodes.is_empty() {
        return Err("BPMN diagram has no elements".into());
    }
    if !doc.pools.is_empty() && doc.nodes.iter().any(|n| n.pool.is_none()) {
        return Err("all elements must belong to a pool when pools are drawn".into());
    }
    for gateway in doc.nodes.iter().filter(|n| n.kind == Kind::EventBased) {
        for edge in doc.edges.iter().filter(|e| e.from == gateway.id) {
            let target = doc.nodes.iter().find(|n| n.id == edge.to).unwrap();
            if !matches!(target.kind, Kind::Intermediate | Kind::ReceiveTask) {
                return Err(format!(
                    "event-based gateway {} requires catching events or receive tasks",
                    gateway.id
                ));
            }
        }
    }
    if !stylesheet.is_empty() {
        doc.styling = crate::parser::parse_stylesheet(&stylesheet)
            .map_err(|e| format!("styling:{}: {}", e.line, e.message))?;
    }
    for rule in &mut doc.flow_layout {
        let selected: Vec<_> = doc
            .edges
            .iter()
            .filter(|e| {
                e.kind == EdgeKind::Sequence
                    && (e.from == rule.from || e.from.ends_with(&format!(".{}", rule.from)))
                    && (e.to == rule.to || e.to.ends_with(&format!(".{}", rule.to)))
            })
            .collect();
        if selected.len() != 1 {
            return Err(format!(
                "flow[{}->{}] must identify exactly one sequence flow",
                rule.from, rule.to
            ));
        }
        let edge = selected[0];
        rule.from = edge.from.clone();
        rule.to = edge.to.clone();
        let source = doc.nodes.iter().find(|n| n.id == rule.from).unwrap();
        let target = doc.nodes.iter().find(|n| n.id == rule.to).unwrap();
        let gateway = |kind| {
            matches!(
                kind,
                Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased
            )
        };
        if (rule.from_side.is_some() && !gateway(source.kind))
            || (rule.to_side.is_some() && !gateway(target.kind))
        {
            return Err("flow attachment overrides apply only to gateway ends".into());
        }
        if rule.label_orientation.is_some()
            && edge.label.is_none()
            && !(source.kind == Kind::Boundary && !source.label.is_empty())
        {
            return Err(
                "label-orientation requires a labeled sequence flow or boundary event".into(),
            );
        }
    }
    for (i, rule) in doc.flow_layout.iter().enumerate() {
        if doc.flow_layout[..i]
            .iter()
            .any(|prior| prior.from == rule.from && prior.to == rule.to)
        {
            return Err("duplicate BPMN flow layout rule".into());
        }
    }
    for (a, _, b) in &doc.layout {
        let ids = [a, b];
        for id in ids {
            if !doc
                .nodes
                .iter()
                .any(|n| &n.id == id || n.id.ends_with(&format!(".{id}")))
            {
                return Err(format!("layout references unknown element {id}"));
            }
        }
    }
    Ok(doc)
}
