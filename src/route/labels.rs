use super::geometry::intersection;
use super::{Gap, Label, Point, Route};
use crate::geometry::BoxNode;
use crate::syntax::LabelPlacement;
use crate::text::measure;

pub(super) fn label_near(
    points: &[Point],
    text: &str,
    size: f64,
    stroke_width: f64,
    boxes: &[BoxNode],
    previous: &[Route],
) -> Result<Option<Label>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let width = measure(text, size)?;
    let height = size * 1.25;
    for pair in points.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        if dx.hypot(dy) < 1e-6 {
            continue;
        }
        for progress in [0.5, 0.3, 0.7] {
            let mid = Point {
                x: pair[0].x + dx * progress,
                y: pair[0].y + dy * progress,
            };
            for side in [-1.0, 1.0] {
                let (x, y) = if dx.abs() >= dy.abs() {
                    (
                        mid.x - width / 2.0,
                        mid.y + side * (height / 2.0 + 8.0) - height / 2.0,
                    )
                } else {
                    (
                        mid.x + side * (width / 2.0 + 8.0) - width / 2.0,
                        mid.y - height / 2.0,
                    )
                };
                let label = Label {
                    text: text.into(),
                    x,
                    y,
                    width,
                    height,
                    angle: 0.0,
                    placement: LabelPlacement::Near,
                };
                if !label_clear(&label, points, None, stroke_width, boxes, previous) {
                    continue;
                }
                return Ok(Some(label));
            }
        }
    }
    Err(format!(
        "cannot place relation label '{text}' without an overlap"
    ))
}
fn near_marker(x: f64, y: f64, w: f64, h: f64, point: Point, stroke_width: f64) -> bool {
    let radius = 14.0 + stroke_width / 2.0; // SVG markers use fixed user-space units
    x < point.x + radius
        && x + w > point.x - radius
        && y < point.y + radius
        && y + h > point.y - radius
}
pub(super) fn segment_hits_rect(a: Point, b: Point, x: f64, y: f64, w: f64, h: f64) -> bool {
    let node = BoxNode {
        id: String::new(),
        label: String::new(),
        lines: Vec::new(),
        body: Vec::new(),
        body_styles: Vec::new(),
        ports: Vec::new(),
        port_start: 0.0,
        top_band: 0.0,
        bottom_band: 0.0,
        abstract_node: false,
        kind: crate::syntax::NodeKind::Generic,
        shape: crate::geometry::NodeShape::Rectangle,
        x,
        y,
        width: w,
        height: h,
        fixed_width: None,
        fixed_height: None,
        font_size: 0.0,
        padding: 0.0,
        fill: String::new(),
        color: String::new(),
        border_color: String::new(),
        border_width: 0.0,
        radius: 0.0,
        line: 0,
    };
    intersection(a, b, &node)
}

fn overlaps(a: &Label, b: &Label) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
fn label_clear(
    label: &Label,
    points: &[Point],
    own_segment: Option<usize>,
    width: f64,
    boxes: &[BoxNode],
    previous: &[Route],
) -> bool {
    let (x, y, w, h) = (label.x, label.y, label.width, label.height);
    if boxes.iter().any(|b| {
        !b.contains_package_content(x, y, w, h)
            && x < b.x + b.width
            && x + w > b.x
            && y < b.y + b.height
            && y + h > b.y
    }) {
        return false;
    }
    if [points[0], *points.last().unwrap()]
        .iter()
        .any(|p| near_marker(x, y, w, h, *p, width))
    {
        return false;
    }
    if previous.iter().any(|r| {
        [r.points[0], *r.points.last().unwrap()]
            .iter()
            .any(|p| near_marker(x, y, w, h, *p, r.width))
            || r.label.as_ref().is_some_and(|other| overlaps(label, other))
            || r.end_labels.iter().any(|other| overlaps(label, other))
            || r.points
                .windows(2)
                .any(|pair| segment_hits_rect(pair[0], pair[1], x, y, w, h))
            || r.bridges.iter().any(|bridge| {
                let pair = &r.points[bridge.segment..=bridge.segment + 1];
                let len = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
                let t = bridge.distance / len;
                let p = Point {
                    x: pair[0].x + t * (pair[1].x - pair[0].x),
                    y: pair[0].y + t * (pair[1].y - pair[0].y),
                };
                let reach = bridge.radius + bridge.height + r.width;
                x < p.x + reach && x + w > p.x - reach && y < p.y + reach && y + h > p.y - reach
            })
    }) {
        return false;
    }
    !points.windows(2).enumerate().any(|(index, pair)| {
        Some(index) != own_segment && segment_hits_rect(pair[0], pair[1], x, y, w, h)
    })
}

/// Rotated real SVG text above or below a straight segment. The stored box is
/// an axis-aligned envelope for conservative collision checks.
pub(super) fn label_along(
    points: &[Point],
    text: &str,
    size: f64,
    stroke: f64,
    boxes: &[BoxNode],
    previous: &[Route],
) -> Result<Option<Label>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let text_width = measure(text, size)?;
    let line_height = size * 1.25;
    for (index, pair) in points.windows(2).enumerate() {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let len = dx.hypot(dy);
        if len < text_width + 32.0 {
            continue;
        }
        let mut angle = dy.atan2(dx).to_degrees();
        if angle > 90.0 {
            angle -= 180.0;
        }
        if angle <= -90.0 {
            angle += 180.0;
        }
        let radians = angle.to_radians();
        let unit = Point {
            x: radians.cos(),
            y: radians.sin(),
        };
        let normal = Point {
            x: unit.y,
            y: -unit.x,
        };
        let bbox_width = text_width * unit.x.abs() + line_height * unit.y.abs();
        let bbox_height = text_width * unit.y.abs() + line_height * unit.x.abs();
        for progress in [0.5, 0.3, 0.7] {
            for side in [1.0, -1.0] {
                let mid = Point {
                    x: pair[0].x + dx * progress,
                    y: pair[0].y + dy * progress,
                };
                let cx = mid.x + normal.x * (line_height / 2.0 + 8.0) * side;
                let cy = mid.y + normal.y * (line_height / 2.0 + 8.0) * side;
                let label = Label {
                    text: text.into(),
                    x: cx - bbox_width / 2.0,
                    y: cy - bbox_height / 2.0,
                    width: bbox_width,
                    height: bbox_height,
                    angle,
                    placement: LabelPlacement::Along,
                };
                if label_clear(&label, points, Some(index), stroke, boxes, previous) {
                    return Ok(Some(label));
                }
            }
        }
    }
    Err(format!(
        "cannot align relation label '{text}' without an overlap"
    ))
}

/// Horizontal text interrupts exactly one segment of its own relation.
pub(super) fn label_inline(
    points: &[Point],
    text: &str,
    size: f64,
    stroke: f64,
    boxes: &[BoxNode],
    previous: &[Route],
) -> Result<Option<(Label, Gap)>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let text_width = measure(text, size)?;
    let height = size * 1.25;
    for (index, pair) in points.windows(2).enumerate() {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let len = dx.hypot(dy);
        if len < 1e-6 {
            continue;
        }
        let half = (dx.abs() * text_width + dy.abs() * height) / (2.0 * len) + 8.0;
        if len < 2.0 * (half + 16.0 + stroke / 2.0) {
            continue;
        }
        for progress in [0.5, 0.3, 0.7] {
            let distance = len * progress;
            if distance < half + 16.0 + stroke / 2.0 || len - distance < half + 16.0 + stroke / 2.0
            {
                continue;
            }
            let mid = Point {
                x: pair[0].x + dx * progress,
                y: pair[0].y + dy * progress,
            };
            let label = Label {
                text: text.into(),
                x: mid.x - text_width / 2.0,
                y: mid.y - height / 2.0,
                width: text_width,
                height,
                angle: 0.0,
                placement: LabelPlacement::Inline,
            };
            if label_clear(&label, points, Some(index), stroke, boxes, previous) {
                return Ok(Some((
                    label,
                    Gap {
                        segment: index,
                        distance,
                        half,
                    },
                )));
            }
        }
    }
    Err(format!(
        "cannot place inline relation label '{text}' with a safe line gap"
    ))
}
