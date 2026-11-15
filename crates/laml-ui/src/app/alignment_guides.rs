//! Screen-space alignment rulers: highlighted current alignment, hover and flash.
use super::*;
use laml::syntax::FreeAlignment;

pub(super) const ACTIVE: egui::Color32 = egui::Color32::from_rgb(83, 211, 255);

pub(super) struct Flash {
    pub align: FreeAlignment,
    pub ids: Vec<String>,
    pub revision: u64,
    pub started: Option<Instant>,
}

fn coordinate(rect: egui::Rect, align: FreeAlignment) -> (bool, f32) {
    match align {
        FreeAlignment::Left => (true, rect.left()),
        FreeAlignment::Right => (true, rect.right()),
        FreeAlignment::CenterX | FreeAlignment::Center => (true, rect.center().x),
        FreeAlignment::Top => (false, rect.top()),
        FreeAlignment::Bottom => (false, rect.bottom()),
        FreeAlignment::CenterY => (false, rect.center().y),
    }
}

fn dashed(painter: &egui::Painter, start: egui::Pos2, end: egui::Pos2, color: egui::Color32) {
    let delta = end - start;
    let length = delta.length();
    if length < 1.0 {
        return;
    }
    let direction = delta / length;
    // Clip the iteration too, so a large zoom never creates millions of dashes.
    let clip = painter.clip_rect();
    let (offset, stop) = if direction.x.abs() > 0.5 {
        (
            (clip.left() - start.x).max(0.0),
            (clip.right() - start.x).min(length),
        )
    } else {
        (
            (clip.top() - start.y).max(0.0),
            (clip.bottom() - start.y).min(length),
        )
    };
    let mut t = (offset / 10.0).floor() * 10.0;
    while t < stop {
        painter.line_segment(
            [
                start + direction * t,
                start + direction * (t + 5.0).min(length),
            ],
            egui::Stroke::new(1.0, color),
        );
        t += 10.0;
    }
}

impl Editor {
    pub(super) fn alignment_guides(&mut self, ui: &egui::Ui, canvas: egui::Rect) {
        if self.selection.ids.len() < 2
            || !self.selection.relations.is_empty()
            || !self.selection.ports.is_empty()
        {
            self.graph.flash = None;
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let (left, top, right, _) = preview.bounds;
        let scale = canvas.width() / (right - left) as f32;
        let rects: Vec<_> = self
            .selection
            .ids
            .iter()
            .filter_map(|id| preview.nodes.iter().find(|n| &n.id == id))
            .map(|node| {
                egui::Rect::from_min_size(
                    canvas.min + egui::vec2((node.x - left) as f32, (node.y - top) as f32) * scale,
                    egui::vec2(node.width as f32, node.height as f32) * scale,
                )
            })
            .collect();
        if rects.len() != self.selection.ids.len() {
            return;
        }
        let reference = *rects.last().unwrap();
        let bounds = rects
            .iter()
            .copied()
            .reduce(|a, b| a.union(b))
            .unwrap()
            .expand(20.0);
        let mut flash = None;
        if let Some(state) = &mut self.graph.flash {
            if state.ids != self.selection.ids || self.command_error.is_some() {
                self.graph.flash = None;
            } else if self.rendered.is_some_and(|r| r > state.revision) {
                let start = *state.started.get_or_insert_with(Instant::now);
                let remaining = 1.0 - start.elapsed().as_secs_f32();
                if remaining > 0.0 {
                    flash = Some((state.align, remaining));
                    ui.ctx().request_repaint();
                } else {
                    self.graph.flash = None;
                }
            }
        }
        // Shared geometry alone is not authored alignment: equal-height nodes
        // can coincide at top, center and bottom with only center-y configured.
        // Follow explicit free/attached alignment rules, including transitive
        // constraints through other nodes and both axes of align=center.
        let configured = |align: FreeAlignment| {
            let (x, y) = align.anchors();
            let (axis_x, anchor) = if let Some(anchor) = x {
                (true, anchor)
            } else {
                (false, y.unwrap())
            };
            let reference = self.selection.ids.last().unwrap();
            self.selection.ids.iter().all(|id| {
                laml_ui::alignment_edit::connected_without(
                    &preview.diagram, "", reference, id, axis_x, anchor,
                )
            })
        };
        let mut axes = graph_header::ALIGNMENTS;
        axes.sort_by_key(|(_, _, align)| (
            configured(*align),
            flash.is_some_and(|(active, _)| active == *align),
        ));
        for (_, _, align) in axes {
            let (vertical, value) = coordinate(reference, align);
            let actual = rects
                .iter()
                .all(|rect| (coordinate(*rect, align).1 - value).abs() <= 0.75);
            let active = configured(align);
            let alpha = if active {
                1.0
            } else if self.graph.hover_align == Some(align) {
                0.5
            } else if actual {
                0.22
            } else {
                continue;
            };
            let (start, end) = if vertical {
                (
                    egui::pos2(value, bounds.top()),
                    egui::pos2(value, bounds.bottom()),
                )
            } else {
                (
                    egui::pos2(bounds.left(), value),
                    egui::pos2(bounds.right(), value),
                )
            };
            let color = if active { ACTIVE } else { appearance::color(ui.ctx(), appearance::SELECTED) }.gamma_multiply(alpha);
            let width = if active { 2.5 } else { 1.0 };
            if active {
                ui.painter().line_segment(
                    [start, end],
                    egui::Stroke::new(5.0, appearance::color(ui.ctx(), appearance::BACKGROUND).gamma_multiply(alpha)),
                );
                ui.painter()
                    .line_segment([start, end], egui::Stroke::new(width, color));
            } else {
                dashed(ui.painter(), start, end, color);
            }
            // Small end caps give the guide a measured/ruler-like appearance.
            let cap = if vertical {
                egui::vec2(4.0, 0.0)
            } else {
                egui::vec2(0.0, 4.0)
            };
            for point in [start, end] {
                ui.painter()
                    .line_segment([point - cap, point + cap], egui::Stroke::new(width, color));
            }
        }
    }
}
