//! Association qualifier rectangles belong to their edge, outside the classifier border.
use super::{Point, Route};
use crate::geometry::BoxNode;
use crate::syntax::RelationDecl;
use crate::text::measure;

#[derive(Clone, Debug)]
pub struct QualifierBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub lines: Vec<String>,
    pub font_size: f64,
}

impl QualifierBox {
    pub fn overlaps(&self, x: f64, y: f64, width: f64, height: f64) -> bool {
        self.x < x + width
            && self.x + self.width > x
            && self.y < y + height
            && self.y + self.height > y
    }
}

fn one(
    points: &mut [Point],
    index: usize,
    lines: &[String],
    size: f64,
    boxes: &[BoxNode],
    owner: usize,
    previous: &[Route],
) -> Result<QualifierBox, String> {
    let text_width = lines
        .iter()
        .try_fold(0.0_f64, |max, line| -> Result<f64, String> {
            Ok(max.max(measure(line, size)?))
        })?;
    let width = text_width + 12.0;
    let height = lines.len() as f64 * size * 1.25 + 10.0;
    let last = points.len() - 1;
    let adjacent = points[if index == 0 { 1 } else { last - 1 }];
    let edge = points[index];
    let (dx, dy) = (adjacent.x - edge.x, adjacent.y - edge.y);
    let horizontal = dx.abs() >= dy.abs();
    let extent = if horizontal { width } else { height };
    if (if horizontal { dx.abs() } else { dy.abs() }) < extent + 12.0 {
        return Err("qualifier needs a longer relation segment outside its classifier".into());
    }
    let sign = if (if horizontal { dx } else { dy }) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let (x, y, far) = if horizontal {
        (
            if sign > 0.0 { edge.x } else { edge.x - width },
            edge.y - height / 2.0,
            Point {
                x: edge.x + sign * width,
                y: edge.y,
            },
        )
    } else {
        (
            edge.x - width / 2.0,
            if sign > 0.0 { edge.y } else { edge.y - height },
            Point {
                x: edge.x,
                y: edge.y + sign * height,
            },
        )
    };
    let box_ = QualifierBox {
        x,
        y,
        width,
        height,
        lines: lines.to_vec(),
        font_size: size,
    };
    if boxes.iter().enumerate().any(|(i, node)| {
        i != owner
            && !node.contains_package_content(x, y, width, height)
            && box_.overlaps(node.x, node.y, node.width, node.height)
    }) || previous.iter().any(|route| {
        route
            .qualifiers
            .iter()
            .any(|old| box_.overlaps(old.x, old.y, old.width, old.height))
            || route
                .label
                .iter()
                .chain(&route.end_labels)
                .any(|label| box_.overlaps(label.x, label.y, label.width, label.height))
            || route.points.windows(2).any(|segment| {
                super::labels::segment_hits_rect(segment[0], segment[1], x, y, width, height)
            })
    }) {
        return Err("qualifier box overlaps a node, label or prior relation".into());
    }
    points[index] = far;
    Ok(box_)
}

pub(super) fn attach(
    relation: &RelationDecl,
    points: &mut [Point],
    boxes: &[BoxNode],
    from: usize,
    to: usize,
    size: f64,
    previous: &[Route],
) -> Result<Vec<QualifierBox>, String> {
    let mut result = Vec::new();
    if let Some(lines) = relation.source_end.qualifier.as_deref() {
        if relation.from != boxes[from].id {
            return Err("qualifier requires a classifier node endpoint".into());
        }
        result.push(one(points, 0, lines, size, boxes, from, previous)?);
    }
    if let Some(lines) = relation.target_end.qualifier.as_deref() {
        if relation.to != boxes[to].id {
            return Err("qualifier requires a classifier node endpoint".into());
        }
        let last = points.len() - 1;
        let qualifier = one(points, last, lines, size, boxes, to, previous)?;
        if result
            .iter()
            .any(|other| qualifier.overlaps(other.x, other.y, other.width, other.height))
        {
            return Err("qualifier boxes overlap each other".into());
        }
        result.push(qualifier);
    }
    Ok(result)
}
