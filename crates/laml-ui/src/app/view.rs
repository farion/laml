use super::*;
#[derive(Default)]
pub(super) struct View {
    pub zoom: Option<f32>,
    pub pan: egui::Vec2,
    pub raster_bucket: Option<u32>,
    pub image_ready: bool,
    pub zoom_request: Option<f32>,
    pub geometry: Option<(egui::Rect, egui::Vec2)>,
    pub canvas_id: Option<egui::Id>,
}
impl View {
    pub fn fit_active(&self) -> bool {
        self.zoom.is_none() && self.zoom_request.is_none() && self.pan == egui::Vec2::ZERO
    }
    fn fit(&mut self) {
        self.zoom = None;
        self.zoom_request = None;
        self.pan = egui::Vec2::ZERO;
    }
    fn actual_size(&mut self) {
        self.zoom = Some(1.0);
        self.zoom_request = None;
        self.pan = egui::Vec2::ZERO;
    }
    fn request_zoom(&mut self, factor: f32) {
        if let Some((viewport, size)) = self.geometry {
            self.zoom_at(viewport, size, viewport.center(), factor);
        } else {
            self.zoom_request = Some(factor);
        }
    }
    fn pan_by(&mut self, delta: egui::Vec2, viewport: egui::Rect, size: egui::Vec2) {
        if delta != egui::Vec2::ZERO {
            self.zoom = Some(self.scale(viewport, size));
            self.pan += delta;
        }
    }
    pub fn scale(&self, viewport: egui::Rect, size: egui::Vec2) -> f32 {
        self.zoom
            .unwrap_or_else(|| (viewport.width() / size.x).min(viewport.height() / size.y))
            .max(0.000001)
    }
    pub fn rect(&self, viewport: egui::Rect, size: egui::Vec2) -> egui::Rect {
        egui::Rect::from_center_size(
            viewport.center() + self.pan,
            size * self.scale(viewport, size),
        )
    }
    pub fn zoom_at(
        &mut self,
        viewport: egui::Rect,
        size: egui::Vec2,
        anchor: egui::Pos2,
        factor: f32,
    ) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let old = self.rect(viewport, size);
        let scale = (self.scale(viewport, size) * factor).clamp(0.000001, 64.0);
        let world = (anchor - old.min) / self.scale(viewport, size);
        self.pan = anchor - world * scale - viewport.center() + size * scale / 2.0;
        self.zoom = Some(scale);
    }
}
impl Editor {
    pub(super) fn view_controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().interact_size = egui::Vec2::splat(24.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        let items = [
            toolbar::Item {
                label: "Fit",
                icon: None,
                enabled: true,
                separator_before: false,
            },
            toolbar::Item {
                label: "100%",
                icon: None,
                enabled: true,
                separator_before: false,
            },
            toolbar::Item {
                label: "Zoom out",
                icon: Some(egui_phosphor::regular::MINUS),
                enabled: true,
                separator_before: false,
            },
            toolbar::Item {
                label: "Zoom in",
                icon: Some(egui_phosphor::regular::PLUS),
                enabled: true,
                separator_before: false,
            },
        ];
        let text_color = appearance::color(ui.ctx(), appearance::TEXT);
        let percentage_width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap("6400%".into(), egui::FontId::proportional(15.0), text_color)
                .size()
                .x
        });
        let width = (ui.available_width() - percentage_width - 6.0).max(24.0);
        let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, 24.0));
        let mut clicked = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            toolbar::show(ui, &items, |ui, index, in_menu| {
                if toolbar::selected_button(
                    ui,
                    &items[index],
                    in_menu,
                    (index == 0).then_some(self.view.fit_active()),
                )
                .clicked()
                {
                    clicked = Some(index);
                    if in_menu {
                        ui.close();
                    }
                }
            });
        });
        match clicked {
            Some(0) => self.view.fit(),
            Some(1) => self.view.actual_size(),
            Some(2) => self.view.request_zoom(0.8),
            Some(3) => self.view.request_zoom(1.25),
            _ => {}
        }
        if let Some((viewport, size)) = self.view.geometry {
            ui.label(format!("{:.0}%", self.view.scale(viewport, size) * 100.0));
        } else {
            ui.label("—%");
        }
    }
    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui) {
        for (index, (label, icon)) in [
            ("Fit", egui_phosphor::regular::CORNERS_OUT),
            ("100%", egui_phosphor::regular::FRAME_CORNERS),
            ("Zoom out", egui_phosphor::regular::MINUS),
            ("Zoom in", egui_phosphor::regular::PLUS),
        ]
        .into_iter()
        .enumerate()
        {
            let item = toolbar::Item {
                label,
                icon: Some(icon),
                enabled: true,
                separator_before: false,
            };
            if toolbar::button(ui, &item, true).clicked() {
                match index {
                    0 => self.view.fit(),
                    1 => self.view.actual_size(),
                    2 => self.view.request_zoom(0.8),
                    _ => self.view.request_zoom(1.25),
                }
                ui.close();
            }
        }
    }
    pub(super) fn viewport_input(
        &mut self,
        ui: &mut egui::Ui,
        viewport: egui::Rect,
        response: &egui::Response,
        size: egui::Vec2,
    ) -> bool {
        if dialogs::blocked(ui.ctx()) {
            return true;
        }
        // Popup windows may extend into adjacent panes. Keep drawing their owner
        // even when the pointer has left the canvas; this is not navigation.
        if egui::Popup::is_any_open(ui.ctx())
            || self.sequence_insertion_menu_open()
            || self.bpmn_insertion_menu_open()
            || self.graph.insertion.is_some()
        {
            return false;
        }
        let hand = ui.input(|i| i.key_down(egui::Key::Space));
        let pan = response.dragged_by(egui::PointerButton::Middle)
            || hand && response.dragged_by(egui::PointerButton::Primary);
        if pan {
            self.view
                .pan_by(ui.input(|i| i.pointer.delta()), viewport, size);
        }
        if response.hovered() {
            let (factor, scroll, anchor) = ui.input(|i| {
                (
                    i.zoom_delta(),
                    i.smooth_scroll_delta,
                    i.pointer.hover_pos().unwrap_or(viewport.center()),
                )
            });
            if factor != 1.0 {
                self.view.zoom_at(viewport, size, anchor, factor);
            } else {
                self.view.pan_by(scroll, viewport, size);
            }
        }
        let outside = ui
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| !viewport.contains(p));
        let graph_drag = self.notation().graph()
            && (self.placement.drag.is_some() || self.graph.gesture.is_some());
        if hand || pan || (outside && !graph_drag) {
            self.creation.drag = None;
            self.relation_drag = None;
            self.placement.cancel();
        }
        hand || pan || (outside && !graph_drag)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fit_is_a_mode_and_manual_navigation_exits_it() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let size = egui::vec2(1200.0, 400.0);
        let mut view = View {
            geometry: Some((viewport, size)),
            ..Default::default()
        };
        assert!(view.fit_active());
        assert_eq!(view.scale(viewport, size) * 100.0, 50.0);
        view.request_zoom(1.25);
        assert!(!view.fit_active());
        assert_eq!(view.scale(viewport, size) * 100.0, 62.5);
        view.fit();
        assert!(view.fit_active());
        assert_eq!(view.pan, egui::Vec2::ZERO);
        view.pan_by(egui::vec2(10.0, 0.0), viewport, size);
        assert!(!view.fit_active());
        view.pan_by(egui::vec2(-10.0, 0.0), viewport, size);
        assert!(!view.fit_active());
        view.fit();
        view.pan_by(egui::Vec2::ZERO, viewport, size);
        assert!(view.fit_active());
        assert_eq!(view.geometry, Some((viewport, size)));
        view.zoom_request = Some(2.0);
        view.actual_size();
        assert!(!view.fit_active());
        assert_eq!(view.scale(viewport, size), 1.0);
        assert_eq!(view.pan, egui::Vec2::ZERO);
        assert_eq!(view.zoom_request, None);
    }

    #[test]
    fn pointer_anchor_stays_on_the_same_world_position() {
        let viewport = egui::Rect::from_min_size(egui::pos2(80.0, 30.0), egui::vec2(600.0, 400.0));
        let size = egui::vec2(900.0, 300.0);
        let mut view = View::default();
        let anchor = egui::pos2(300.0, 100.0);
        let before = (anchor - view.rect(viewport, size).min) / view.scale(viewport, size);
        view.zoom_at(viewport, size, anchor, 2.0);
        let after = (anchor - view.rect(viewport, size).min) / view.scale(viewport, size);
        assert!((before - after).length() < 0.001);
    }
}
