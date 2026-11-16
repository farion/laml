//! Picking and gestures use exact source-linked regions from each compiler renderer.
use super::*;
use laml::authoring::Region;
fn distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = if dx * dx + dy * dy < 1e-9 {
        0.0
    } else {
        ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)
    }
    .clamp(0.0, 1.0);
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}
pub(super) fn hit(regions: &[Region], x: f64, y: f64, tolerance: f64) -> Option<&Region> {
    regions
        .iter()
        .filter(|r| {
            if r.kind.starts_with("sequence-")
                || matches!(r.kind.as_str(), "lifeline-end" | "destroyed-end")
            {
                return false;
            }
            if r.path.is_empty() {
                x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height
            } else {
                r.path
                    .windows(2)
                    .any(|p| distance((x, y), p[0], p[1]) <= tolerance)
            }
        })
        .min_by(|a, b| {
            let priority = |r: &Region| {
                if matches!(r.kind.as_str(), "pool" | "lane" | "frame" | "lifeline") {
                    1
                } else {
                    0
                }
            };
            priority(a).cmp(&priority(b)).then_with(|| {
                (a.width.max(1.0) * a.height.max(1.0))
                    .total_cmp(&(b.width.max(1.0) * b.height.max(1.0)))
            })
        })
}
impl Editor {
    pub(in crate::app) fn notation_canvas(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
        navigation: bool,
    ) {
        let Some(preview) = &self.preview else {
            return;
        };
        let bounds = preview.bounds;
        let regions = preview.regions.clone();
        let scale = f64::from(rect.width()) / (bounds.2 - bounds.0);
        let local = |p: egui::Pos2| {
            (
                bounds.0 + f64::from(p.x - rect.left()) / scale,
                bounds.1 + f64::from(p.y - rect.top()) / scale,
            )
        };
        let screen = |x: f64, y: f64| {
            rect.left_top()
                + egui::vec2(
                    ((x - bounds.0) * scale) as f32,
                    ((y - bounds.1) * scale) as f32,
                )
        };
        if let Some((line, id, _)) = &self.notation_ui.selected {
            for r in regions.iter().filter(|r| {
                r.line == *line
                    && !r.kind.starts_with("sequence-")
                    && id.as_ref().is_none_or(|id| &r.id == id)
            }) {
                // Endpoint grips identify selected sequence messages without painting
                // a solid highlight over their dashed strokes and arrowheads.
                if matches!(
                    (self.notation(), r.kind.as_str()),
                    (Notation::Sequence, "message") | (Notation::Bpmn, "flow")
                ) && !r.path.is_empty()
                {
                    continue;
                }
                if r.path.is_empty() {
                    let r = egui::Rect::from_min_max(
                        screen(r.x, r.y),
                        screen(r.x + r.width, r.y + r.height),
                    );
                    ui.painter().rect_stroke(
                        r.expand(3.0),
                        2.0,
                        egui::Stroke::new(1.5, appearance::color(ui.ctx(), appearance::ACCENT)),
                        egui::StrokeKind::Outside,
                    );
                } else {
                    for pair in r.path.windows(2) {
                        ui.painter().line_segment(
                            [screen(pair[0].0, pair[0].1), screen(pair[1].0, pair[1].1)],
                            egui::Stroke::new(3.0, appearance::color(ui.ctx(), appearance::ACCENT)),
                        );
                    }
                }
            }
        }
        let enabled = self.can_edit_visually();
        response.context_menu(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(16.0, 6.0);
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(crate::i18n::tr("Add construct…")),
                )
                .clicked()
            {
                self.open_construct_dialog();
                ui.close();
            }
            if ui.button(crate::i18n::tr("Elements…")).clicked() {
                self.notation_ui.navigator = true;
                ui.close();
            }
            if let Some((line, id, revision)) = self.notation_ui.selected.clone()
                && ui
                    .add_enabled(
                        enabled && revision == self.document.revision(),
                        egui::Button::new(crate::i18n::tr("Delete…")),
                    )
                    .clicked()
            {
                self.notation_delete(line, id.as_deref(), false);
                ui.close();
            }
        });
        if navigation {
            self.notation_ui.drag = None;
            return;
        }
        if self.notation() == Notation::Sequence && self.sequence_canvas(ui, rect) {
            return;
        }
        if self.notation() == Notation::Bpmn && self.bpmn_canvas(ui, rect) {
            return;
        }
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
        {
            let (x, y) = local(p);
            if let Some(r) = hit(&regions, x, y, 7.0 / scale) {
                self.notation_select(r.line, Some(r.id.clone()));
            } else {
                self.notation_ui.selected = None;
            }
        }
        if enabled
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = ui.input(|i| i.pointer.press_origin())
        {
            let (x, y) = local(p);
            if let Some(r) = hit(&regions, x, y, 7.0 / scale) {
                self.notation_select(r.line, Some(r.id.clone()));
                self.notation_ui.drag = Some((
                    r.id.clone(),
                    r.kind.clone(),
                    r.line,
                    self.document.revision(),
                    p,
                ));
            }
        }
        if let Some((id, kind, line, revision, start)) = self.notation_ui.drag.clone() {
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if revision != self.document.revision()
                || !enabled
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                self.notation_ui.drag = None;
            } else if let Some(p) = pointer {
                ui.painter().line_segment(
                    [start, p],
                    egui::Stroke::new(1.5, appearance::color(ui.ctx(), appearance::ACCENT)),
                );
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    self.notation_ui.drag = None;
                    let (x, y) = local(p);
                    let target = hit(&regions, x, y, 10.0 / scale);
                    if self.notation_ui.connect {
                        if let Some(target) = target.filter(|t| {
                            t.id != id && matches!(t.kind.as_str(), "node" | "participant")
                        }) && matches!(kind.as_str(), "node" | "participant")
                        {
                            self.notation_connect(&id, &target.id);
                        }
                    } else if self.notation() == Notation::Timing
                        && matches!(kind.as_str(), "state-change" | "event" | "tick")
                    {
                        self.timing_drag(line, f64::from(p.x - start.x) / scale);
                    } else if let Some(target) = target.filter(|t| t.id != id) {
                        if self.notation() == Notation::Timing
                            && kind == "lifeline"
                            && target.kind == "lifeline"
                        {
                            match edit::timing::move_lane(
                                self.document.source(),
                                &id,
                                &target.id,
                                y < target.y + target.height / 2.0,
                            ) {
                                Ok(edits) => {
                                    self.notation_submit("Reorder timing lifelines", edits, None)
                                }
                                Err(e) => self.command_error = Some(e),
                            }
                        } else if self.notation() == Notation::Sequence
                            && matches!(kind.as_str(), "note" | "continuation")
                            && matches!(target.kind.as_str(), "participant" | "message")
                        {
                            let target = if target.kind == "message" {
                                format!("{}.receive", target.id)
                            } else {
                                target.id.clone()
                            };
                            match edit::sequence::place_annotation(
                                self.document.source(),
                                &id,
                                &target,
                                kind == "continuation",
                                false,
                            ) {
                                Ok(edits) => self.notation_submit(
                                    "Place sequence annotation",
                                    edits,
                                    Some(line),
                                ),
                                Err(e) => self.command_error = Some(e),
                            }
                        } else if self.notation() == Notation::Sequence
                            && kind == "participant"
                            && target.kind == "participant"
                        {
                            let op = if x < target.x + target.width / 2.0 {
                                "<"
                            } else {
                                ">"
                            };
                            self.notation_relative(&id, &target.id, op);
                        } else if self.notation() == Notation::Sequence
                            && matches!(
                                kind.as_str(),
                                "message" | "frame" | "event" | "decomposition"
                            )
                            && matches!(
                                target.kind.as_str(),
                                "message" | "frame" | "event" | "decomposition"
                            )
                        {
                            let before = if y <= target.y {
                                Some(target.line)
                            } else {
                                edit::records(self.document.source())
                                    .into_iter()
                                    .find(|r| r.line > target.line && r.section == "semantics")
                                    .map(|r| r.line)
                            };
                            let mut parent = edit::record(self.document.source(), target.line)
                                .ok()
                                .and_then(|r| r.parent);
                            let mut before = before;
                            if target.kind == "frame" && y > target.y + 28.0 {
                                parent = Some(target.line);
                                before = None;
                            }
                            match edit::move_record(self.document.source(), line, parent, before) {
                                Ok(edits) => self.notation_submit("Move message", edits, None),
                                Err(e) => self.command_error = Some(e),
                            }
                        } else if self.notation() == Notation::Bpmn && kind == "node" {
                            if matches!(target.kind.as_str(), "lane" | "pool") {
                                match edit::bpmn::relocate(self.document.source(), &id, target.line)
                                {
                                    Ok(edits) => {
                                        self.notation_submit("Change BPMN membership", edits, None)
                                    }
                                    Err(e) => self.command_error = Some(e),
                                }
                            } else if target.kind == "node" {
                                let dx = (x - target.x - target.width / 2.0) / target.width;
                                let dy = (y - target.y - target.height / 2.0) / target.height;
                                self.notation_relative(
                                    &id,
                                    &target.id,
                                    if dx.abs() > dy.abs() {
                                        if dx < 0.0 { "<" } else { ">" }
                                    } else if dy < 0.0 {
                                        "above"
                                    } else {
                                        "below"
                                    },
                                );
                            }
                        }
                    }
                } else if !ui.input(|i| i.pointer.primary_down()) {
                    self.notation_ui.drag = None;
                }
            }
        }
    }
    fn timing_drag(&mut self, line: usize, dx: f64) {
        let Ok(doc) = laml::timing::parse(self.document.source()) else {
            return;
        };
        let Some(field) = edit::fields::of(self.document.source(), line)
            .into_iter()
            .find(|f| f.name == "Time")
        else {
            return;
        };
        let Ok(original) = field.value.parse::<f64>() else {
            return;
        };
        let value = ((original + dx / doc.scale).max(0.0) * 100.0).round() / 100.0;
        match edit::fields::edit(self.document.source(), &field, &value.to_string()) {
            Ok(edits) => self.notation_submit("Move timing occurrence", edits, Some(line)),
            Err(e) => self.command_error = Some(e),
        }
    }
    fn notation_relative(&mut self, from: &str, to: &str, op: &str) {
        let result = edit::relative(self.document.source(), from, to, op);
        match result {
            Ok(edits) => self.notation_submit("Change relative placement", edits, None),
            Err(e) => self.command_error = Some(e),
        }
    }
    pub(super) fn notation_connect(&mut self, from: &str, to: &str) {
        let operator = self.graph.operator.clone();
        self.connect_notation_default(from, to, &operator, None, None);
    }
}
