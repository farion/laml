//! Collision-checked multiplicity and role placement near relation endpoints.
use super::labels::label_clear_with_markers;
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
    own_markers: [f64; 2],
    points: &'a [Point],
    boxes: &'a [BoxNode],
    previous: &'a [Route],
    central: Option<&'a Label>,
}

fn options_one(
    text: &str,
    default_side: f64,
    requested: Option<EndSide>,
    pair: [Point; 2],
    context: &Context<'_>,
    current: &[Label],
) -> Result<Vec<Label>, String> {
    let mut options = Vec::new();
    let width = measure(text, context.size)?;
    let height = context.size * 1.25;
    let (dx, dy) = (pair[1].x - pair[0].x, pair[1].y - pair[0].y);
    let length = dx.hypot(dy);
    if length < 1e-6 {
        return Err("relation endpoint has no usable segment for annotation".into());
    }
    let (ux, uy) = (dx / length, dy / length);
    let horizontal = ux.abs() >= uy.abs();
    let preferred = match (horizontal, requested) {
        (_, None) => default_side,
        (true, Some(EndSide::Above)) | (false, Some(EndSide::Left)) => -1.0,
        (true, Some(EndSide::Below)) | (false, Some(EndSide::Right)) => 1.0,
        _ => {
            return Err(
                "mandatory association-end side is incompatible with this route segment".into(),
            );
        }
    };
    // A default is a preference, not a constraint: try the other side before
    // asking the author to move a classifier or remove an association label.
    for side in [preferred, -preferred]
        .into_iter()
        .take(if requested.is_some() { 1 } else { 2 })
    {
        for extra in [0.0, 12.0, 24.0, 40.0, 64.0, 96.0, 128.0]
            .into_iter()
            .chain((12..=32).map(|step| step as f64 * 12.0))
        {
            let distance =
                14.0 + if horizontal {
                    width / 2.0
                } else {
                    height / 2.0
                } + extra;
            if distance > length - 6.0 {
                break;
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
            options.push(label);
        }
    }
    if options.is_empty() {
        Err(format!(
            "cannot place association-end text '{text}' without overlap"
        ))
    } else {
        Ok(options)
    }
}

fn place_end(
    annotation: &EndAnnotation,
    pair: [Point; 2],
    context: &Context<'_>,
    rule: Option<&RelationEndRule>,
    result: &mut Vec<Label>,
) -> Result<(), String> {
    let multiplicities = match &annotation.multiplicity {
        Some(text) => options_one(
            text,
            -1.0,
            rule.and_then(|r| r.multiplicity),
            pair,
            context,
            result,
        )?
        .into_iter()
        .map(Some)
        .collect(),
        None => vec![None],
    };
    let mut role_error = None;
    for multiplicity in multiplicities {
        let mut placed = result.clone();
        if let Some(label) = multiplicity {
            placed.push(label);
        }
        if let Some(text) = &annotation.role {
            match options_one(text, 1.0, rule.and_then(|r| r.role), pair, context, &placed) {
                Ok(options) => placed.push(options.into_iter().next().unwrap()),
                Err(message) => {
                    role_error = Some(message);
                    continue;
                }
            }
        }
        *result = placed;
        return Ok(());
    }
    Err(role_error.unwrap_or_else(|| "cannot place association-end text".into()))
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
        own_markers: [
            super::geometry::marker_radius(relation.appearance.start, font_and_stroke.1),
            super::geometry::marker_radius(relation.appearance.end, font_and_stroke.1),
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
    fn unforced_multiplicity_yields_to_a_mandatory_role_side() {
        let length = 22.0 + measure("owner", 14.0).unwrap() / 2.0;
        let straight = [Point { x: 0.0, y: 0.0 }, Point { x: length, y: 0.0 }];
        let context = Context {
            size: 14.0,
            own_markers: [0.0, 0.0],
            points: &straight,
            boxes: &[],
            previous: &[],
            central: None,
        };
        let end = EndAnnotation {
            multiplicity: Some("1".into()),
            role: Some("owner".into()),
            qualifier: None,
        };
        let rule = RelationEndRule {
            name: "r".into(),
            end: crate::syntax::EndPosition::Source,
            side: None,
            multiplicity: None,
            role: Some(EndSide::Above),
            line: 1,
        };
        let mut labels = Vec::new();
        place_end(&end, straight, &context, Some(&rule), &mut labels).unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels[0].y > 0.0 && labels[1].y < 0.0);
    }

    #[test]
    fn end_text_must_avoid_other_segments_of_its_own_route() {
        let p = |x, y| Point { x, y };
        let straight = [p(0.0, 0.0), p(200.0, 0.0)];
        let context = Context {
            size: 14.0,
            own_markers: [0.0, 0.0],
            points: &straight,
            boxes: &[],
            previous: &[],
            central: None,
        };
        assert!(options_one("role", -1.0, None, straight, &context, &[]).is_ok());
        let detour = [p(0.0, 0.0), p(200.0, 0.0), p(200.0, -18.0), p(0.0, -18.0)];
        let context = Context {
            points: &detour,
            ..context
        };
        let moved = options_one("role", -1.0, None, [detour[0], detour[1]], &context, &[])
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(moved.y > 0.0, "the other side avoids the own detour");
        assert!(
            options_one(
                "role",
                -1.0,
                Some(EndSide::Above),
                [detour[0], detour[1]],
                &context,
                &[],
            )
            .is_err()
        );
    }
}
