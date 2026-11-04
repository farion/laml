use super::{BoxNode, Diagram, NodeKind, direct_owner};

// A lower sibling can lend its row to a leaf package linked to it while an
// upper sibling routes outward. Do not move authored positions.
pub(super) fn open_package_lanes(diagram: &Diagram, local: &mut [BoxNode], parent: Option<&str>) {
    if diagram.settings.diagram_type != crate::syntax::DiagramType::Package
        || parent.is_none()
        || !diagram.layout.is_empty()
    {
        return;
    }
    let parent = parent.unwrap();
    let center = |node: &BoxNode| node.y + node.height / 2.0;
    for right in 0..local.len() {
        if local[right].kind != NodeKind::Package
            || local[right].is_container
            || diagram
                .layout
                .iter()
                .any(|rule| rule.first == local[right].id || rule.second == local[right].id)
        {
            continue;
        }
        let candidate = (0..local.len())
            .filter(|&i| {
                local[i].kind == NodeKind::Package
                    && local[i].x + local[i].width + 16.0 < local[right].x
            })
            .flat_map(|upper| (0..local.len()).map(move |lower| (upper, lower)))
            .find(|&(upper, lower)| {
                if upper == lower
                    || (local[upper].x - local[lower].x).abs() > 4.0
                    || center(&local[upper]) + local[upper].height / 2.0
                        >= center(&local[lower]) - local[lower].height / 2.0 + 4.0
                {
                    return false;
                }
                let joined = diagram.relations.iter().any(|relation| {
                    relation.from == local[lower].id && relation.to == local[right].id
                });
                let outward = diagram.relations.iter().any(|relation| {
                    relation.from == local[upper].id
                        && direct_owner(diagram, &relation.to, Some(parent)).is_none()
                });
                joined && outward
            });
        if let Some((upper, lower)) = candidate {
            let proposed = center(&local[upper]) * 0.38 + center(&local[lower]) * 0.62
                - local[right].height / 2.0;
            let clear = local.iter().enumerate().all(|(i, node)| {
                i == right
                    || node.x + node.width + 8.0 <= local[right].x
                    || local[right].x + local[right].width + 8.0 <= node.x
                    || proposed + local[right].height + 8.0 <= node.y
                    || node.y + node.height + 8.0 <= proposed
            });
            if clear {
                local[right].y = proposed;
            }
        }
    }
}
