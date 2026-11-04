use super::{Equality, GeometryError, Minimum, solve_axis};
use crate::geometry::BoxNode;
use crate::syntax::{Diagram, DiagramType, NodeKind, Placement};

// A note attached to a relation belongs over its open span, not in the lane
// of an unrelated vertical connection. Authored X constraints still win.
pub(super) fn prefer_x(
    diagram: &Diagram,
    boxes: &[BoxNode],
    eq_x: &[Equality],
    x: &[Minimum],
    pref_x: &mut [f64],
) -> Result<(), GeometryError> {
    if diagram.settings.diagram_type != DiagramType::Object {
        return Ok(());
    }
    for attachment in &diagram.attachments {
        let Some(note) = boxes.iter().position(|b| b.id == attachment.source) else {
            continue;
        };
        if boxes[note].kind != NodeKind::Note
            || diagram.layout.iter().any(|rule| {
                (rule.first == boxes[note].id || rule.second == boxes[note].id)
                    && (matches!(rule.placement, Placement::Left | Placement::Right)
                        || rule.align.is_some())
            })
        {
            continue;
        }
        let Some(link) = diagram
            .relations
            .iter()
            .find(|r| r.name.as_deref() == Some(attachment.target.as_str()))
        else {
            continue;
        };
        let owner = |id: &str| boxes.iter().position(|b| b.has_endpoint(id)).unwrap();
        let (a, b) = (owner(&link.from), owner(&link.to));
        let xs = solve_axis(boxes.len(), eq_x, x, pref_x)?;
        let (left, right) = if xs[a] + boxes[a].width <= xs[b] {
            (a, b)
        } else if xs[b] + boxes[b].width <= xs[a] {
            (b, a)
        } else {
            continue;
        };
        pref_x[note] = (xs[left] + boxes[left].width + xs[right] - boxes[note].width) / 2.0;
    }
    Ok(())
}
