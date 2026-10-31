//! Collision-checked multiplicity and role placement near relation endpoints.
use super::labels::label_clear_with_markers;
use super::{Label, Point, Route};
use crate::geometry::BoxNode;
use crate::syntax::{
    EndAnnotation, EndSide, EndSymbol, LabelPlacement, RelationDecl, RelationEndRule,
};
use crate::text::measure;

fn overlaps(x: f64, y: f64, width: f64, height: f64, label: &Label) -> bool {
    x < label.x + label.width
        && x + width > label.x
        && y < label.y + label.height
        && y + height > label.y
}

struct Context<'a> {
    size: f64,
    stroke: f64,
    own_markers: [bool; 2],
    points: &'a [Point],
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
        let label = Label {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            angle: 0.0,
            placement: LabelPlacement::Near,
        };
        if !label_clear_with_markers(
            &label,
            context.points,
            None,
            context.stroke,
            context.boxes,
            context.previous,
            context.own_markers,
        ) || current
            .iter()
            .chain(context.central)
            .any(|other| overlaps(x, y, width, height, other))
        {
            continue;
        }
        return Ok(label);
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
    font_and_stroke: (f64, f64),
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
        size: font_and_stroke.0,
        stroke: font_and_stroke.1,
        own_markers: [
            relation.appearance.start != EndSymbol::None,
            relation.appearance.end != EndSymbol::None,
        ],
        points,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn end_text_must_avoid_other_segments_of_its_own_route() {
        let p = |x, y| Point { x, y };
        let straight = [p(0.0, 0.0), p(200.0, 0.0)];
        let context = Context {
            size: 14.0,
            stroke: 2.0,
            own_markers: [false, false],
            points: &straight,
            boxes: &[],
            previous: &[],
            central: None,
        };
        assert!(place_one("role", -1.0, None, straight, &context, &[]).is_ok());
        let detour = [p(0.0, 0.0), p(200.0, 0.0), p(200.0, -18.0), p(0.0, -18.0)];
        let context = Context {
            points: &detour,
            ..context
        };
        assert!(place_one("role", -1.0, None, [detour[0], detour[1]], &context, &[]).is_err());
    }
}
