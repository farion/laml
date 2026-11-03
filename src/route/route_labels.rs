//! Choose a relation label and its end annotations as one collision-free unit.
use super::{Gap, Label, Point, Route, bridges::BridgePlan, end_labels, labels, side_rules};
use crate::geometry::BoxNode;
use crate::syntax::{LabelPlacement, RelationDecl, RelationEndRule};

// The candidate geometry must be evaluated together; splitting the inputs would
// permit a label decision inconsistent with its route, markers or end text.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn select(
    relation: &RelationDecl,
    points: &[Point],
    boxes: &[BoxNode],
    previous: &[Route],
    plan: &BridgePlan,
    qualifiers: &[super::QualifierBox],
    placement: LabelPlacement,
    font_size: f64,
    width: f64,
    rules: [Option<&RelationEndRule>; 2],
    package_pill: bool,
) -> Result<(Option<Label>, Vec<Label>, Option<Gap>), String> {
    let markers = side_rules::label_clearance(relation, width);
    let options = match relation.display_label() {
        None => vec![(None, None)],
        Some(text) => match placement {
            LabelPlacement::Near => {
                labels::near_options(points, &text, font_size, markers, boxes, previous)?
                    .into_iter()
                    .map(|label| (Some(label), None))
                    .collect()
            }
            LabelPlacement::Along => {
                labels::along_options(points, &text, font_size, markers, boxes, previous)?
                    .into_iter()
                    .map(|label| (Some(label), None))
                    .collect()
            }
            LabelPlacement::Inline => {
                labels::inline_options(points, &text, font_size, width, markers, boxes, previous)?
                    .into_iter()
                    .map(|(label, gap)| (Some(label), Some(gap)))
                    .collect()
            }
        },
    };
    let mut reason = None;
    for (label, gap) in options {
        // The modern package label's visible pill extends beyond its text box.
        // Reject a placement that would paint over another package outline.
        if package_pill
            && label.as_ref().is_some_and(|text| {
                let (x, y, w, h) = (
                    text.x - 4.0,
                    text.y - 2.0,
                    text.width + 8.0,
                    text.height + 4.0,
                );
                boxes.iter().any(|node| {
                    !node.contains_package_content(x, y, w, h)
                        && x < node.x + node.width
                        && x + w > node.x
                        && y < node.y + node.height
                        && y + h > node.y
                })
            })
        {
            reason = Some("relation label pill overlaps a package".into());
            continue;
        }
        if label.as_ref().is_some_and(|text| {
            qualifiers
                .iter()
                .any(|q| q.overlaps(text.x, text.y, text.width, text.height))
        }) {
            reason = Some("qualifier overlaps relation label".to_string());
            continue;
        }
        if gap.as_ref().is_some_and(|gap| {
            plan.events.iter().any(|(_, bridge, _)| {
                bridge.segment == gap.segment
                    && (bridge.distance - gap.distance).abs() < bridge.radius + gap.half + 4.0
            })
        }) {
            reason = Some("inline label gap conflicts with required bridge".to_string());
            continue;
        }
        if !plan.avoids_label(points, &label) {
            reason = Some("relation label conflicts with required bridge".to_string());
            continue;
        }
        let ends = match end_labels::place(
            relation,
            points,
            (font_size, width),
            boxes,
            previous,
            label.as_ref(),
            rules,
        ) {
            Ok(ends) => ends,
            Err(message) => {
                reason = Some(message);
                continue;
            }
        };
        if ends.iter().any(|end| {
            qualifiers
                .iter()
                .any(|q| q.overlaps(end.x, end.y, end.width, end.height))
        }) {
            reason = Some("qualifier overlaps association-end text".to_string());
            continue;
        }
        if ends
            .iter()
            .any(|end| !plan.avoids_label(points, &Some(end.clone())))
        {
            reason = Some("association-end text conflicts with required bridge".to_string());
            continue;
        }
        return Ok((label, ends, gap));
    }
    Err(reason.unwrap_or_else(|| "no collision-free relation annotation placement".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relation_title_and_end_text_are_chosen_together() {
        let diagram = crate::parser::parse(
            "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\n@r A \"1\"/owner -- \"0..*\"/items B : places\n}\n",
        )
        .unwrap();
        let points = [Point { x: 0.0, y: 0.0 }, Point { x: 280.0, y: 0.0 }];
        let plan = BridgePlan { events: Vec::new() };
        let (title, ends, _) = select(
            &diagram.relations[0],
            &points,
            &[],
            &[],
            &plan,
            &[],
            LabelPlacement::Near,
            14.0,
            2.0,
            [None, None],
            false,
        )
        .unwrap();
        let title = title.unwrap();
        assert_eq!(ends.len(), 4);
        for placement in [LabelPlacement::Along, LabelPlacement::Inline] {
            assert!(
                select(
                    &diagram.relations[0],
                    &points,
                    &[],
                    &[],
                    &plan,
                    &[],
                    placement,
                    14.0,
                    2.0,
                    [None, None],
                    false,
                )
                .is_ok()
            );
        }
        assert!(ends.iter().all(|end| {
            end.x >= title.x + title.width
                || title.x >= end.x + end.width
                || end.y >= title.y + title.height
                || title.y >= end.y + end.height
        }));
    }
}
