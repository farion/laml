//! Canvas-space moving routes, painted by the actual compiler SVG renderer.
use super::*;
use connection_geometry::Dock;
use connection_gesture::{Gesture, Kind};

#[derive(Default)]
pub(super) struct Cache {
    pub uri: String,
    key: String,
    bytes: Vec<u8>,
    rect: Option<egui::Rect>,
}
impl Cache {
    pub fn publish(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        key: String,
        rect: egui::Rect,
        svg: String,
    ) {
        self.uri = format!("bytes://moving-route-{id}.svg");
        ctx.forget_image(&self.uri);
        self.bytes = svg.into_bytes();
        self.key = key;
        self.rect = Some(rect);
    }
}
#[derive(Debug)]
pub(super) struct Frame {
    pub canvas: egui::Rect,
    pub viewport: egui::Rect,
    pub start: egui::Pos2,
    pub end: egui::Pos2,
    pub source_direction: egui::Vec2,
    pub target_direction: egui::Vec2,
    pub target: Option<Dock>,
}
pub(super) struct Rendering<'a> {
    pub presentation: laml_ui::backend::Presentation,
    pub source: &'a str,
    pub path: &'a std::path::Path,
    pub service: &'a mut remote_ui::Remote,
}
pub(super) fn paint(
    ui: &egui::Ui,
    preview: &Preview,
    document: Rendering<'_>,
    id: u64,
    gesture: &Gesture,
    frame: Frame,
    cache: &mut Cache,
) {
    let key = format!("{frame:?}");
    if key != cache.key {
        let scale = f64::from(frame.canvas.width()) / (preview.bounds.2 - preview.bounds.0);
        let world = |p: egui::Pos2| laml::route::Point {
            x: preview.bounds.0 + f64::from(p.x - frame.canvas.left()) / scale,
            y: preview.bounds.1 + f64::from(p.y - frame.canvas.top()) / scale,
        };
        let mut route = gesture.route.clone();
        let start = world(frame.start);
        let end = world(frame.end);
        let old_start = route.points.first().copied().unwrap_or(start);
        let old_end = route.points.last().copied().unwrap_or(end);
        route.points = laml_ui::gesture_preview::orthogonal(
            start,
            end,
            (
                f64::from(frame.source_direction.x),
                f64::from(frame.source_direction.y),
            ),
            (
                f64::from(frame.target_direction.x),
                f64::from(frame.target_direction.y),
            ),
        );
        route.bridges.clear();
        route.gaps.clear();
        let index = |id: &str| preview.nodes.iter().position(|n| n.id == id);
        if let Kind::New { from, .. } = &gesture.kind {
            route.from = index(&from.owner).unwrap_or(0);
        }
        if let Some(dock) = &frame.target {
            let node = index(&dock.owner).unwrap_or(0);
            if matches!(
                gesture.kind,
                Kind::Reconnect {
                    end: laml::syntax::EndPosition::Source,
                    ..
                }
            ) {
                route.from = node;
            } else {
                route.to = node;
            }
        }
        for label in &mut route.end_labels {
            let source = (label.x - old_start.x).hypot(label.y - old_start.y)
                <= (label.x - old_end.x).hypot(label.y - old_end.y);
            let (old, new) = if source {
                (old_start, start)
            } else {
                (old_end, end)
            };
            label.x += new.x - old.x;
            label.y += new.y - old.y;
        }
        for q in &mut route.qualifiers {
            let source = (q.x - old_start.x).hypot(q.y - old_start.y)
                <= (q.x - old_end.x).hypot(q.y - old_end.y);
            let (old, new) = if source {
                (old_start, start)
            } else {
                (old_end, end)
            };
            q.x += new.x - old.x;
            q.y += new.y - old.y;
        }
        if let Some(label) = &mut route.label
            && let Some(pair) = route.points.windows(2).max_by(|a, b| {
                (a[0].x - a[1].x)
                    .hypot(a[0].y - a[1].y)
                    .total_cmp(&(b[0].x - b[1].x).hypot(b[0].y - b[1].y))
            })
        {
            label.x = (pair[0].x + pair[1].x - label.width) / 2.0;
            label.y = (pair[0].y + pair[1].y - label.height) / 2.0 - 12.0;
        }
        let min = world(frame.viewport.min);
        let max = world(frame.viewport.max);
        let operation = laml_ui::backend::protocol::Operation::GestureRender {
            presentation: document.presentation,
            source: document.source.into(),
            address: document.path.to_string_lossy().into_owned(),
            route: Some(Box::new(route)),
            bounds: (min.x, min.y, max.x, max.y),
            remove_line: None,
        };
        if let Err(error) =
            document
                .service
                .overlay(id, gesture.revision, key, frame.viewport, operation)
        {
            ui.painter().text(
                frame.viewport.left_top(),
                egui::Align2::LEFT_TOP,
                crate::i18n::message(&(error)),
                egui::FontId::proportional(14.0),
                egui::Color32::LIGHT_RED,
            );
        }
    }
    if let Some(rect) = cache.rect {
        egui::Image::from_bytes(cache.uri.clone(), cache.bytes.clone())
            .fit_to_exact_size(rect.size())
            .paint_at(ui, rect);
    }
    ui.ctx().request_repaint_after(Duration::from_millis(16));
}
