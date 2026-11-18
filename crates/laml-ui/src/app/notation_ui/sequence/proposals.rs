//! Compute legal source transactions before exposing drag destinations.
use super::*;

fn proposal(
    result: Result<(Vec<laml_ui::document::Edit>, usize), String>,
    label: &'static str,
) -> Option<Proposal> {
    let (edits, line) = result.ok()?;
    (!edits.is_empty()).then_some(Proposal {
        edits,
        line: Some(line),
        label,
    })
}
fn plain(
    result: Result<Vec<laml_ui::document::Edit>, String>,
    label: &'static str,
) -> Option<Proposal> {
    let edits = result.ok()?;
    (!edits.is_empty()).then_some(Proposal {
        edits,
        line: None,
        label,
    })
}
fn destination(at: egui::Pos2, proposal: Proposal) -> Destination {
    Destination {
        at,
        proposal,
        alternatives: vec![],
        dock: None,
        hit_rect: None,
        preview: None,
    }
}
pub(super) fn message_destination(scene: &Scene, point: &Point, proposal: Proposal) -> Destination {
    let mut d = destination(point.at, proposal);
    d.dock = Some(point.dock.clone());
    if let Some(bar) = scene.bar_for(point) {
        d.hit_rect = Some(egui::Rect::from_min_max(
            egui::pos2(bar.rect.left(), point.at.y),
            egui::pos2(bar.rect.right(), point.at.y),
        ));
    }
    d
}
pub(super) fn destinations(
    source: &str,
    scene: &Scene,
    hit: &Hit,
    _operator: &str,
) -> Vec<Destination> {
    match hit {
        Hit::Participant(head) => {
            let mut heads = scene
                .heads
                .iter()
                .filter(|(h, _)| h.id != head.id && scope(&h.id) == scope(&head.id))
                .collect::<Vec<_>>();
            heads.sort_by(|a, b| a.1.center().x.total_cmp(&b.1.center().x));
            let mut out = vec![];
            for i in 0..=heads.len() {
                let before = heads.get(i).map(|(h, _)| h.id.as_str());
                let at = egui::pos2(
                    if let Some((_, r)) = heads.get(i) {
                        r.left() - 14.0
                    } else {
                        heads.last().map_or(0.0, |(_, r)| r.right() + 14.0)
                    },
                    scene.shape(head).center().y,
                );
                if let Some(p) = plain(
                    edit::sequence_scoped::reorder(source, &head.id, before),
                    "Reorder participants",
                ) {
                    out.push(destination(at, p));
                }
            }
            out
        }
        Hit::Resize(bar, lower) => resize(source, scene, bar, *lower),
        #[cfg(test)]
        Hit::Endpoint { .. } | Hit::Origin(_) => {
            super::message_targets::validated(source, scene, hit, _operator)
        }
        Hit::Destruction(r) => {
            let mut out = scene
                .regions
                .iter()
                .filter(|slot| slot.kind == "sequence-slot" && scope(&slot.id) == scope(&r.id))
                .filter_map(|slot| {
                    proposal(
                        edit::sequence_destruction::move_to(source, &r.id, r.line, slot.line),
                        "Move destruction",
                    )
                    .map(|p| destination(scene.point(r.x + r.width / 2.0, slot.y), p))
                })
                .collect::<Vec<_>>();
            for other in scene
                .regions
                .iter()
                .filter(|other| other.kind == "destroyed-end" && other.id != r.id)
            {
                if let Some(p) = plain(
                    edit::sequence_destruction::align(source, &r.id, &other.id),
                    "Align destruction",
                ) {
                    out.push(destination(
                        scene.point(r.x + r.width / 2.0, other.y + other.height / 2.0),
                        p,
                    ));
                }
            }
            out
        }
        Hit::Event(r) => scene
            .regions
            .iter()
            .filter(|r| r.kind == "sequence-slot" && scope(&r.id) == scope(&hit_owner(hit)))
            .filter_map(|slot| {
                proposal(
                    edit::sequence_scoped::move_event(source, r.line, slot.line, Some(&r.id)),
                    "Move sequence element",
                )
                .map(|p| destination(scene.point(slot.x, slot.y), p))
            })
            .collect(),
        _ => vec![],
    }
}
fn resize(source: &str, scene: &Scene, bar: &scene::Bar, lower: bool) -> Vec<Destination> {
    let edge = if lower {
        bar.region.end_line.unwrap_or(bar.region.line)
    } else {
        bar.region.line
    };
    let mut out = vec![];
    for slot in scene
        .regions
        .iter()
        .filter(|r| r.kind == "sequence-slot" && scope(&r.id) == scope(&bar.owner))
    {
        let candidate = edit::sequence_scoped::resize(
            source,
            &bar.owner,
            bar.region.line,
            edge,
            lower,
            lower && bar.region.end_line.is_none(),
            slot.line,
        );
        if let Some(p) = proposal(candidate, "Resize activation") {
            out.push(destination(
                egui::pos2(bar.rect.center().x, scene.point(slot.x, slot.y).y),
                p,
            ));
        }
    }
    for other in scene
        .bars
        .iter()
        .filter(|b| b.region.line != bar.region.line || b.owner != bar.owner)
    {
        let candidate = edit::sequence_scoped::align(
            source,
            &bar.owner,
            bar.region.line,
            &other.owner,
            other.region.line,
            lower,
        );
        if let Some(p) = proposal(candidate, "Align activation edge") {
            out.push(destination(
                egui::pos2(
                    bar.rect.center().x,
                    if lower {
                        other.rect.bottom()
                    } else {
                        other.rect.top()
                    },
                ),
                p,
            ));
        }
    }
    for message in scene.regions.iter().filter(|r| {
        r.id == bar.owner && matches!(r.kind.as_str(), "sequence-send" | "sequence-receive")
    }) {
        let candidate = edit::sequence_scoped::snap(
            source,
            &bar.owner,
            bar.region.line,
            edge,
            lower,
            message.line,
            message.kind == "sequence-receive",
        );
        if let Some(p) = proposal(candidate, "Dock activation edge") {
            out.push(destination(
                egui::pos2(bar.rect.center().x, scene.point(message.x, message.y).y),
                p,
            ));
        }
    }
    out
}

fn scope(id: &str) -> &str {
    id.rsplit_once('.').map_or("", |(scope, _)| scope)
}
fn hit_owner(hit: &Hit) -> String {
    match hit {
        Hit::Event(r) | Hit::Destruction(r) => r.id.clone(),
        _ => String::new(),
    }
}
