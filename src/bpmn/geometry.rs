//! Source-derived placement: participant bands, lane bands, flow ranks and nested subprocesses.
use super::{Document, Kind, Node};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BoxAt {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
#[derive(Clone, Debug)]
pub(super) struct Band {
    pub id: Option<String>,
    pub label: String,
    pub y: f64,
    pub h: f64,
}
#[derive(Clone, Debug)]
pub(super) struct PoolBox {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub bands: Vec<Band>,
}
pub(super) struct Geometry {
    pub nodes: Vec<BoxAt>,
    pub pools: Vec<PoolBox>,
    pub width: f64,
    pub height: f64,
}
fn outer(n: &Node) -> bool {
    n.parent.is_none() && n.kind != Kind::Boundary
}
fn size(doc: &Document, node: &Node) -> (f64, f64) {
    match node.kind {
        Kind::Task
        | Kind::UserTask
        | Kind::ServiceTask
        | Kind::SendTask
        | Kind::ReceiveTask
        | Kind::ScriptTask
        | Kind::ManualTask
        | Kind::Collapsed => (
            (crate::text::measure(&node.label, 14.0).unwrap_or(100.0) + 100.0).max(166.0),
            70.0,
        ),
        Kind::Annotation => {
            let lines: Vec<_> = node.label.split('\n').collect();
            let text_width = lines
                .iter()
                .map(|line| crate::text::measure(line, 13.0).unwrap_or(0.0))
                .fold(0.0_f64, f64::max);
            (
                (text_width + 30.0).max(44.0),
                (lines.len() as f64 * 16.0 + 22.0).max(38.0),
            )
        }
        Kind::Start | Kind::End | Kind::Intermediate | Kind::Boundary => (54.0, 54.0),
        Kind::Exclusive | Kind::Parallel | Kind::Inclusive | Kind::EventBased => (70.0, 70.0),
        Kind::Data | Kind::Store => (72.0, 78.0),
        Kind::Subprocess => {
            let children = doc
                .nodes
                .iter()
                .filter(|n| {
                    n.parent.as_deref() == Some(node.id.rsplit('.').next().unwrap())
                        && n.pool == node.pool
                })
                .count();
            ((children as f64 * 180.0 + 70.0).max(220.0), 210.0)
        }
    }
}
mod aisles;
mod branch;
mod group;
use group::place_group;
mod notes;
mod ranks;
use ranks::ranks;
fn node_index(doc: &Document, name: &str) -> Result<usize, String> {
    let indices: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.id == name || n.id.ends_with(&format!(".{name}")))
        .collect();
    if indices.len() == 1 {
        Ok(indices[0].0)
    } else {
        Err(format!(
            "layout element '{name}' is ambiguous or unknown; use its qualified ID"
        ))
    }
}
fn oriented<'a>(doc: &Document, op: &'a str) -> &'a str {
    if doc.direction != crate::syntax::FlowDirection::TopToBottom {
        return op;
    }
    match op {
        "<" => "above",
        ">" => "below",
        "above" => "<",
        "below" => ">",
        _ => op,
    }
}
fn gaps(doc: &Document, a: BoxAt, b: BoxAt) -> (f64, f64) {
    if doc.direction == crate::syntax::FlowDirection::TopToBottom {
        ((a.h + b.h) / 2.0 + 55.0, (a.w + b.w) / 2.0 + 45.0)
    } else {
        ((a.w + b.w) / 2.0 + 55.0, (a.h + b.h) / 2.0 + 45.0)
    }
}
fn move_node(doc: &Document, boxes: &mut [BoxAt], index: usize, x: f64, y: f64) {
    let (dx, dy) = (x - boxes[index].x, y - boxes[index].y);
    boxes[index].x = x;
    boxes[index].y = y;
    for (i, node) in doc.nodes.iter().enumerate() {
        if node.attached.as_deref() == Some(&doc.nodes[index].id)
            || (doc.nodes[index].kind == Kind::Subprocess
                && node.pool == doc.nodes[index].pool
                && node.parent.as_deref() == doc.nodes[index].id.rsplit('.').next())
        {
            boxes[i].x += dx;
            boxes[i].y += dy;
        }
    }
}
pub(super) fn place(doc: &Document) -> Result<Geometry, String> {
    let ranks = ranks(doc);
    let mut boxes = vec![BoxAt::default(); doc.nodes.len()];
    let mut pools = Vec::new();
    let mut top = 35.0;
    let pool_ids: Vec<Option<String>> = if doc.pools.is_empty() {
        vec![None]
    } else {
        doc.pools.iter().map(|p| Some(p.id.clone())).collect()
    };
    let mut canvas_width = 0.0_f64;
    for pool_id in pool_ids {
        let pool = pool_id
            .as_ref()
            .and_then(|id| doc.pools.iter().find(|p| &p.id == id));
        let mut bands = Vec::new();
        // In top-to-bottom diagrams the pool title stays above the pool;
        // it must not reserve a second, empty strip at the pool's left.
        let mut band_top = top
            + if pool.is_some() && doc.direction != crate::syntax::FlowDirection::TopToBottom {
                30.0
            } else {
                0.0
            };
        let lane_ids: Vec<Option<String>> = if let Some(p) = pool {
            let mut ids: Vec<_> = p.lanes.iter().map(|l| Some(l.id.clone())).collect();
            if p.lanes.is_empty()
                || doc
                    .nodes
                    .iter()
                    .any(|n| n.pool == pool_id && n.lane.is_none() && outer(n))
            {
                ids.push(None);
            }
            ids.sort_by_key(|lane| {
                if doc
                    .nodes
                    .iter()
                    .any(|n| n.pool == pool_id && n.lane == *lane && n.kind == Kind::Start)
                {
                    0
                } else if lane.is_none() {
                    2
                } else {
                    1
                }
            });
            ids
        } else {
            vec![None]
        };
        let mut width = 280.0_f64;
        for lane_id in lane_ids {
            let indices: Vec<_> = doc
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.pool == pool_id && n.lane == lane_id && outer(n))
                .map(|(i, _)| i)
                .collect();
            let (w, h) = place_group(
                doc,
                &indices,
                &ranks,
                &mut boxes,
                if pool.is_some() { 175.0 } else { 95.0 },
                band_top,
                if doc.direction == crate::syntax::FlowDirection::TopToBottom
                    && pool.is_none_or(|p| p.lanes.is_empty())
                {
                    0.0
                } else {
                    30.0
                },
            );
            width = width.max(w + if pool.is_some() { 105.0 } else { 45.0 });
            let label = pool
                .and_then(|p| p.lanes.iter().find(|l| Some(&l.id) == lane_id.as_ref()))
                .map_or_else(
                    || {
                        if lane_id.is_none() && pool.is_some_and(|p| !p.lanes.is_empty()) {
                            "Unassigned".into()
                        } else {
                            String::new()
                        }
                    },
                    |l| l.label.clone(),
                );
            bands.push(Band {
                id: lane_id.clone(),
                label,
                y: band_top,
                h: h.max(125.0),
            });
            band_top += h.max(125.0);
        }
        let pool_height = band_top - top
            + if pool.is_some() && doc.direction != crate::syntax::FlowDirection::TopToBottom {
                16.0
            } else {
                0.0
            };
        canvas_width = canvas_width.max(width + 40.0);
        if let Some(p) = pool {
            pools.push(PoolBox {
                label: p.label.clone(),
                x: 25.0,
                y: top,
                w: width,
                h: pool_height,
                bands,
            });
        }
        top += pool_height
            + if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                96.0
            } else {
                32.0
            };
    }
    for (i, n) in doc
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == Kind::Subprocess)
    {
        let parent = n.id.rsplit('.').next().unwrap();
        let children: Vec<_> = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, child)| {
                child.parent.as_deref() == Some(parent)
                    && child.pool == n.pool
                    && child.kind != Kind::Boundary
            })
            .map(|(j, _)| j)
            .collect();
        let base = boxes[i];
        let _ = place_group(
            doc,
            &children,
            &ranks,
            &mut boxes,
            base.x - base.w / 2.0 + 70.0,
            base.y - base.h / 2.0 + 30.0,
            30.0,
        );
    }
    for (i, n) in doc
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == Kind::Boundary)
    {
        let owner = n
            .attached
            .as_ref()
            .and_then(|id| doc.nodes.iter().position(|m| &m.id == id))
            .ok_or_else(|| format!("boundary event {} has no owner", n.id))?;
        let o = boxes[owner];
        boxes[i] = BoxAt {
            x: o.x + o.w / 2.0 - 20.0,
            y: o.y + o.h / 2.0,
            w: 38.0,
            h: 38.0,
        };
    }
    aisles::clear_lower_branches(doc, &mut boxes);
    notes::clear_boundary_exit(doc, &mut boxes);
    for (a, op, b) in &doc.layout {
        let (a, b) = (node_index(doc, a)?, node_index(doc, b)?);
        if doc.nodes[a].pool != doc.nodes[b].pool
            || doc.nodes[a].lane != doc.nodes[b].lane
            || doc.nodes[a].parent != doc.nodes[b].parent
        {
            return Err(
                "layout cannot move an element between participants, lanes or subprocesses".into(),
            );
        }
        let (from, to) = (boxes[a], boxes[b]);
        let (xgap, ygap) = gaps(doc, from, to);
        match oriented(doc, op) {
            "<" => move_node(doc, &mut boxes, b, to.x.max(from.x + xgap), to.y),
            ">" => move_node(doc, &mut boxes, a, from.x.max(to.x + xgap), from.y),
            "above" => move_node(doc, &mut boxes, b, to.x, to.y.max(from.y + ygap)),
            "below" => move_node(doc, &mut boxes, a, from.x, from.y.max(to.y + ygap)),
            _ => unreachable!(),
        }
    }
    for (a, op, b) in &doc.layout {
        let (a, b) = (node_index(doc, a)?, node_index(doc, b)?);
        let (xgap, ygap) = gaps(doc, boxes[a], boxes[b]);
        let required = match oriented(doc, op) {
            "<" => boxes[a].x + xgap - 1.0 <= boxes[b].x,
            ">" => boxes[b].x + xgap - 1.0 <= boxes[a].x,
            "above" => boxes[a].y + ygap - 1.0 <= boxes[b].y,
            "below" => boxes[b].y + ygap - 1.0 <= boxes[a].y,
            _ => false,
        };
        if !required {
            return Err("conflicting mandatory BPMN layout rules".into());
        }
    }
    // Grow each affected lane; never let a manual rule move a node into another lane.
    for pi in 0..pools.len() {
        for bi in 0..pools[pi].bands.len() {
            let id = &doc.pools[pi].id;
            let lane = &pools[pi].bands[bi].id;
            let bottom = doc
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.pool.as_ref() == Some(id) && &n.lane == lane)
                .map(|(i, _)| {
                    boxes[i].y
                        + if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                            boxes[i].w / 2.0 + 24.0
                        } else {
                            boxes[i].h / 2.0 + 35.0
                        }
                })
                .fold(pools[pi].bands[bi].y + pools[pi].bands[bi].h, f64::max);
            let delta = (bottom - pools[pi].bands[bi].y - pools[pi].bands[bi].h).max(0.0);
            if delta < 0.1 {
                continue;
            }
            pools[pi].bands[bi].h += delta;
            pools[pi].h += delta;
            for later in bi + 1..pools[pi].bands.len() {
                pools[pi].bands[later].y += delta;
                let later_id = &pools[pi].bands[later].id;
                for (i, n) in doc.nodes.iter().enumerate() {
                    if n.pool.as_ref() == Some(id) && &n.lane == later_id {
                        boxes[i].y += delta;
                    }
                }
            }
            for (later, p) in pools.iter_mut().enumerate().skip(pi + 1) {
                p.y += delta;
                for band in &mut p.bands {
                    band.y += delta;
                }
                let later_id = &doc.pools[later].id;
                for (i, n) in doc.nodes.iter().enumerate() {
                    if n.pool.as_ref() == Some(later_id) {
                        boxes[i].y += delta;
                    }
                }
            }
        }
        let id = &doc.pools[pi].id;
        let right = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.pool.as_ref() == Some(id))
            .map(|(i, _)| {
                boxes[i].x
                    + if doc.direction == crate::syntax::FlowDirection::TopToBottom {
                        boxes[i].h / 2.0 + 50.0
                    } else {
                        boxes[i].w / 2.0 + 50.0
                    }
            })
            .fold(0.0_f64, f64::max);
        pools[pi].w = pools[pi].w.max(right - pools[pi].x);
    }
    if doc.direction == crate::syntax::FlowDirection::TopToBottom {
        for b in &mut boxes {
            std::mem::swap(&mut b.x, &mut b.y);
        }
        for p in &mut pools {
            std::mem::swap(&mut p.x, &mut p.y);
            std::mem::swap(&mut p.w, &mut p.h);
        }
    }
    let max_right = boxes
        .iter()
        .map(|b| b.x + b.w / 2.0 + 80.0)
        .fold(0.0_f64, f64::max);
    let max_bottom = boxes
        .iter()
        .map(|b| b.y + b.h / 2.0 + 90.0)
        .fold(0.0_f64, f64::max);
    for (i, a) in boxes.iter().enumerate() {
        if doc.nodes[i].kind == Kind::Boundary {
            continue;
        }
        for (j, b) in boxes.iter().enumerate().skip(i + 1) {
            if doc.nodes[j].kind == Kind::Boundary
                || doc.nodes[i].pool != doc.nodes[j].pool
                || doc.nodes[i].lane != doc.nodes[j].lane
                || doc.nodes[i].parent != doc.nodes[j].parent
            {
                continue;
            }
            if (a.x - b.x).abs() < (a.w + b.w) / 2.0 + 4.0
                && (a.y - b.y).abs() < (a.h + b.h) / 2.0 + 4.0
            {
                return Err(format!(
                    "BPMN nodes {} and {} overlap",
                    doc.nodes[i].id, doc.nodes[j].id
                ));
            }
        }
    }
    let right_edge = pools
        .iter()
        .map(|p| p.x + p.w + 25.0)
        .fold(0.0_f64, f64::max);
    let bottom_edge = pools
        .iter()
        .map(|p| p.y + p.h + 25.0)
        .fold(0.0_f64, f64::max);
    let rotated = doc.direction == crate::syntax::FlowDirection::TopToBottom;
    Ok(Geometry {
        nodes: boxes,
        pools,
        width: if rotated {
            max_right.max(right_edge)
        } else {
            canvas_width.max(max_right).max(right_edge)
        },
        height: if rotated {
            max_bottom.max(bottom_edge)
        } else {
            top.max(max_bottom).max(bottom_edge)
        },
    })
}
