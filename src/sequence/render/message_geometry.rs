//! Shared message paths and arrowhead geometry for exports, picking and editor gestures.
use crate::{sequence::MessageKind, styles::StyleSheet};
pub type Point = (f64, f64);

pub fn path(send: Point, receive: Point, self_message: bool, diagonal: bool) -> Vec<Point> {
    if self_message {
        return self_path(send, receive, send.0.max(receive.0) + 44.0);
    }
    if diagonal || (send.1 - receive.1).abs() < 0.01 {
        return vec![send, receive];
    }
    let elbow =
        send.0 + (receive.0 - send.0).signum() * 24.0_f64.min((receive.0 - send.0).abs() / 2.0);
    vec![send, (elbow, send.1), (elbow, receive.1), receive]
}

pub fn self_path(send: Point, receive: Point, right: f64) -> Vec<Point> {
    vec![send, (right, send.1), (right, receive.1), receive]
}
pub fn svg_path(points: &[Point], self_message: bool) -> String {
    let (sx, sy) = points[0];
    let (tx, ty) = *points.last().unwrap();
    if self_message && points.len() == 4 {
        let right = points[1].0;
        let outward = if (right - sx - 44.0).abs() < 0.001 {
            "44".into()
        } else {
            format!("{:.2}", right - sx)
        };
        let inward = if (right - tx - 44.0).abs() < 0.001 {
            "44".into()
        } else {
            format!("{:.2}", right - tx)
        };
        return if (sx - tx).abs() < 0.01 {
            format!("M{sx:.2},{sy:.2} h{outward} v{:.2} h-{inward}", ty - sy)
        } else {
            format!("M{sx:.2},{sy:.2} h{outward} v{:.2} H{tx:.2}", ty - sy)
        };
    }
    if points.len() == 4 {
        return format!("M{sx:.2},{sy:.2} H{:.2} V{ty:.2} H{tx:.2}", points[1].0);
    }
    format!("M{sx:.2},{sy:.2} L{tx:.2},{ty:.2}")
}

pub fn arrowhead(tip: Point, direction: Point, modern: bool) -> [Point; 3] {
    let length = direction.0.hypot(direction.1).max(0.001);
    let (ux, uy) = (direction.0 / length, direction.1 / length);
    let (arrow_length, half_width) = if modern { (12.0, 6.0) } else { (10.0, 5.0) };
    let (bx, by) = (tip.0 - ux * arrow_length, tip.1 - uy * arrow_length);
    [
        tip,
        (bx - uy * half_width, by + ux * half_width),
        (bx + uy * half_width, by - ux * half_width),
    ]
}

pub fn dashed(kind: MessageKind) -> bool {
    matches!(kind, MessageKind::Reply | MessageKind::Create)
}
pub fn filled(kind: MessageKind, filled_reply: bool) -> bool {
    kind == MessageKind::Sync || filled_reply
}
pub fn activation_appearance(
    sheet: &StyleSheet,
    owner: &str,
    classes: &[String],
) -> (String, String, f64, f64) {
    let paint = super::styles::activation(sheet, owner, None, classes);
    (paint.fill, paint.border, paint.width, paint.border_width)
}
pub fn appearance(sheet: &StyleSheet, name: Option<&str>, classes: &[String]) -> (String, f64) {
    let paint = super::styles::message(sheet, name, classes);
    (paint.color, paint.width)
}
