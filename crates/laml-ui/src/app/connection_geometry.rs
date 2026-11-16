//! Screen-space connection handles, with ports taking precedence over node sides.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct Dock {
    pub id: String,
    pub owner: String,
    pub side: Option<&'static str>,
    pub point: egui::Pos2,
}
#[derive(Clone)]
pub(super) struct Handle {
    pub dock: Dock,
    pub point: egui::Pos2,
    pub radius: f32,
}

pub(super) fn screen(preview: &Preview, canvas: egui::Rect, x: f64, y: f64) -> egui::Pos2 {
    let (left, top, right, _) = preview.bounds;
    canvas.min
        + egui::vec2((x - left) as f32, (y - top) as f32) * (canvas.width() / (right - left) as f32)
}
pub(super) fn node_rect(
    preview: &Preview,
    canvas: egui::Rect,
    node: &laml::geometry::BoxNode,
) -> egui::Rect {
    egui::Rect::from_min_max(
        screen(preview, canvas, node.x, node.y),
        screen(preview, canvas, node.x + node.width, node.y + node.height),
    )
}
pub(super) fn radius(preview: &Preview, canvas: egui::Rect, node: &laml::geometry::BoxNode) -> f32 {
    (node.width.min(node.height) as f32 * 0.24).clamp(8.0, 14.0) * canvas.width()
        / (preview.bounds.2 - preview.bounds.0) as f32
}
pub(super) fn endpoint_radius(preview: &Preview, canvas: egui::Rect) -> f32 {
    8.0 * canvas.width() / (preview.bounds.2 - preview.bounds.0) as f32
}
pub(super) fn handles(
    preview: &Preview,
    canvas: egui::Rect,
    node: &laml::geometry::BoxNode,
    source: bool,
) -> Vec<Handle> {
    let rect = node_rect(preview, canvas, node);
    let radius = radius(preview, canvas, node);
    let offset = radius * 1.4;
    [
        ("left", rect.left_center(), egui::vec2(-offset, 0.0)),
        ("right", rect.right_center(), egui::vec2(offset, 0.0)),
        ("top", rect.center_top(), egui::vec2(0.0, -offset)),
        ("bottom", rect.center_bottom(), egui::vec2(0.0, offset)),
    ]
    .into_iter()
    .map(|(side, point, offset)| Handle {
        radius: if source { radius } else { radius * 0.72 },
        point: point + if source { offset } else { egui::Vec2::ZERO },
        dock: Dock {
            id: node.id.clone(),
            owner: node.id.clone(),
            side: Some(side),
            point,
        },
    })
    .collect()
}

pub(super) fn hit_node(
    preview: &Preview,
    canvas: egui::Rect,
    point: egui::Pos2,
) -> Option<&laml::geometry::BoxNode> {
    let (left, top, right, _) = preview.bounds;
    let scale = canvas.width() / (right - left) as f32;
    laml_ui::selection::hit_node(
        &preview.nodes,
        left + f64::from((point.x - canvas.left()) / scale),
        top + f64::from((point.y - canvas.top()) / scale),
    )
}
fn port(preview: &Preview, canvas: egui::Rect, point: egui::Pos2) -> Option<Dock> {
    preview
        .nodes
        .iter()
        .flat_map(|node| node.ports.iter().map(move |port| (node, port)))
        .filter_map(|(node, port)| {
            let position = screen(preview, canvas, port.x, port.y);
            (position.distance(point) <= radius(preview, canvas, node) * 0.72).then_some((
                position.distance(point),
                Dock {
                    id: port.id.clone(),
                    owner: node.id.clone(),
                    side: None,
                    point: position,
                },
            ))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, dock)| dock)
}
pub(super) fn source_at(preview: &Preview, canvas: egui::Rect, point: egui::Pos2) -> Option<Dock> {
    port(preview, canvas, point).or_else(|| {
        let owner = hit_node(preview, canvas, point).map(|node| &node.id);
        preview
            .nodes
            .iter()
            .filter(|node| owner.is_none_or(|id| id == &node.id))
            .flat_map(|node| handles(preview, canvas, node, true))
            .filter(|handle| handle.point.distance(point) <= handle.radius)
            .min_by(|a, b| a.point.distance(point).total_cmp(&b.point.distance(point)))
            .map(|handle| handle.dock)
    })
}
/// Shift changes side selection only; explicit named ports remain semantic endpoints.
pub(super) fn attachment(mut dock: Dock, automatic: bool) -> Dock {
    if automatic && dock.id == dock.owner {
        dock.side = None;
    }
    dock
}
pub(super) fn target_at(
    preview: &Preview,
    canvas: egui::Rect,
    point: egui::Pos2,
    automatic: bool,
) -> Option<Dock> {
    port(preview, canvas, point)
        .or_else(|| {
            preview
                .nodes
                .iter()
                .flat_map(|node| handles(preview, canvas, node, false))
                .filter(|handle| handle.point.distance(point) <= handle.radius)
                .min_by(|a, b| a.point.distance(point).total_cmp(&b.point.distance(point)))
                .map(|handle| handle.dock)
        })
        .or_else(|| {
            let node = hit_node(preview, canvas, point)?;
            let side = nearest_side(node_rect(preview, canvas, node), point);
            handles(preview, canvas, node, false)
                .into_iter()
                .find(|handle| handle.dock.side == Some(side))
                .map(|handle| handle.dock)
        })
        .map(|dock| attachment(dock, automatic))
}
pub(super) fn direction(side: &str) -> egui::Vec2 {
    match side {
        "left" => egui::vec2(-1.0, 0.0),
        "top" => egui::vec2(0.0, -1.0),
        "bottom" => egui::vec2(0.0, 1.0),
        _ => egui::vec2(1.0, 0.0),
    }
}

pub(super) fn automatic_anchor(
    preview: &Preview,
    canvas: egui::Rect,
    id: &str,
    toward: egui::Pos2,
) -> egui::Pos2 {
    let Some(node) = preview.nodes.iter().find(|n| n.id == id) else {
        return toward;
    };
    let rect = node_rect(preview, canvas, node);
    let mut delta = toward - rect.center();
    if delta.length_sq() < 0.01 {
        delta = egui::vec2(1.0, 0.0);
    }
    let half = rect.size() / 2.0;
    let factor = match node.shape {
        laml::geometry::NodeShape::Ellipse => {
            1.0 / ((delta.x / half.x).powi(2) + (delta.y / half.y).powi(2)).sqrt()
        }
        laml::geometry::NodeShape::Diamond => {
            1.0 / (delta.x.abs() / half.x + delta.y.abs() / half.y)
        }
        _ => (half.x / delta.x.abs()).min(half.y / delta.y.abs()),
    };
    rect.center() + delta * factor
}

pub(super) fn dock_direction(
    preview: &Preview,
    canvas: egui::Rect,
    dock: &Dock,
    point: egui::Pos2,
) -> egui::Vec2 {
    if let Some(side) = dock.side {
        return direction(side);
    }
    preview
        .nodes
        .iter()
        .find(|n| n.id == dock.owner)
        .map_or(egui::Vec2::ZERO, |node| {
            direction(nearest_side(node_rect(preview, canvas, node), point))
        })
}

pub(super) fn nearest_side(rect: egui::Rect, point: egui::Pos2) -> &'static str {
    [
        ("left", (point.x - rect.left()).abs()),
        ("right", (point.x - rect.right()).abs()),
        ("top", (point.y - rect.top()).abs()),
        ("bottom", (point.y - rect.bottom()).abs()),
    ]
    .into_iter()
    .min_by(|a, b| a.1.total_cmp(&b.1))
    .unwrap()
    .0
}
