//! Collision-checked multiplicity and role placement near relation endpoints.
use super::labels::segment_hits_rect;
use super::{Label, Point, Route};
use crate::geometry::BoxNode;
use crate::syntax::{EndAnnotation, EndSide, LabelPlacement, RelationDecl, RelationEndRule};
use crate::text::measure;

fn overlaps(x: f64, y: f64, width: f64, height: f64, label: &Label) -> bool {
    x < label.x + label.width
        && x + width > label.x
        && y < label.y + label.height
        && y + height > label.y
}

struct Context<'a> {
    size: f64,
    boxes: &'a [BoxNode],
    previous: &'a [Route],
    central: Option<&'a Label>,
}

fn place_one(
    text: &str,
    default_side: f64,
    requested: Option<EndSide>,
    pair: [Point; 2],
    context: &Context<'_>,
    current: &[Label],
) -> Result<Label, String> {
    let width = measure(text, context.size)?;
    let height = context.size * 1.25;
    let (dx, dy) = (pair[1].x - pair[0].x, pair[1].y - pair[0].y);
    let length = dx.hypot(dy);
    if length < 1e-6 {
        return Err("relation endpoint has no usable segment for annotation".into());
    }
    let (ux, uy) = (dx / length, dy / length);
    let horizontal = ux.abs() >= uy.abs();
    let side = match (horizontal, requested) {
        (_, None) => default_side,
        (true, Some(EndSide::Above)) | (false, Some(EndSide::Left)) => -1.0,
        (true, Some(EndSide::Below)) | (false, Some(EndSide::Right)) => 1.0,
        _ => {
            return Err(
                "mandatory association-end side is incompatible with this route segment".into(),
            );
        }
    };
    for extra in [0.0, 12.0, 24.0, 40.0, 64.0, 96.0, 128.0] {
        let distance =
            14.0 + if horizontal {
                width / 2.0
            } else {
                height / 2.0
            } + extra;
        if distance > length - 6.0 && extra > 0.0 {
            continue;
        }
        let mid = Point {
            x: pair[0].x + ux * distance,
            y: pair[0].y + uy * distance,
        };
        let (x, y) = if horizontal {
            (
                mid.x - width / 2.0,
                mid.y + if side < 0.0 { -height - 6.0 } else { 6.0 },
            )
        } else {
            (
                mid.x + if side < 0.0 { -width - 6.0 } else { 6.0 },
                mid.y - height / 2.0,
            )
        };
        if context.boxes.iter().any(|b| {
            !b.contains_package_content(x, y, width, height)
                && x < b.x + b.width
                && x + width > b.x
                && y < b.y + b.height
                && y + height > b.y
        }) {
            continue;
        }
        if current
            .iter()
            .chain(context.central)
            .any(|label| overlaps(x, y, width, height, label))
        {
            continue;
        }
        if context.previous.iter().any(|route| {
            route
                .label
                .as_ref()
                .is_some_and(|label| overlaps(x, y, width, height, label))
                || route
                    .end_labels
                    .iter()
                    .any(|label| overlaps(x, y, width, height, label))
                || route
                    .points
                    .windows(2)
                    .any(|segment| segment_hits_rect(segment[0], segment[1], x, y, width, height))
        }) {
            continue;
        }
        return Ok(Label {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            angle: 0.0,
            placement: LabelPlacement::Near,
        });
    }
    Err(format!(
        "cannot place association-end text '{text}' without overlap"
    ))
}

fn place_end(
    annotation: &EndAnnotation,
    pair: [Point; 2],
    context: &Context<'_>,
    rule: Option<&RelationEndRule>,
    result: &mut Vec<Label>,
) -> Result<(), String> {
    if let Some(text) = &annotation.multiplicity {
        result.push(place_one(
            text,
            -1.0,
            rule.and_then(|r| r.multiplicity),
            pair,
            context,
            result,
        )?);
    }
    if let Some(text) = &annotation.role {
        result.push(place_one(
            text,
            1.0,
            rule.and_then(|r| r.role),
            pair,
            context,
            result,
        )?);
    }
    Ok(())
}

pub(super) fn place(
    relation: &RelationDecl,
    points: &[Point],
    size: f64,
    boxes: &[BoxNode],
    previous: &[Route],
    central: Option<&Label>,
    rules: [Option<&RelationEndRule>; 2],
) -> Result<Vec<Label>, String> {
    if relation.source_end == EndAnnotation::default()
        && relation.target_end == EndAnnotation::default()
    {
        return Ok(Vec::new());
    }
    let context = Context {
        size,
        boxes,
        previous,
        central,
    };
    let mut labels = Vec::new();
    place_end(
        &relation.source_end,
        [points[0], points[1]],
        &context,
        rules[0],
        &mut labels,
    )?;
    let last = points.len() - 1;
    place_end(
        &relation.target_end,
        [points[last], points[last - 1]],
        &context,
        rules[1],
        &mut labels,
    )?;
    Ok(labels)
}
