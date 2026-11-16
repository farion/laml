use super::*;

impl Editor {
    pub(super) fn preview_loading(&self, ui: &mut egui::Ui, viewport: egui::Rect) {
        // Called by the pane after painting its contents, outside the canvas padding.
        let viewport = viewport.intersect(ui.clip_rect());
        if self.preview_updating() {
            ui.painter().rect_filled(
                viewport,
                0,
                if ui.visuals().dark_mode {
                    egui::Color32::from_black_alpha(110)
                } else {
                    egui::Color32::from_white_alpha(150)
                },
            );
            let rect = egui::Rect::from_center_size(viewport.center(), egui::vec2(220.0, 70.0));
            let mut loading = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("preview-loading")
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            loading.set_clip_rect(viewport);
            loading.add(
                egui::Spinner::new()
                    .size(28.0)
                    .color(appearance::color(ui.ctx(), appearance::TEXT)),
            );
            loading.add_space(8.0);
            loading.label(
                egui::RichText::new(crate::i18n::tr("Updating preview…"))
                    .color(appearance::color(ui.ctx(), appearance::TEXT)),
            );
        }
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        let enabled = self.can_edit_visually();
        if !enabled || ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.creation.drag = None;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let (left, top, right, bottom) = preview.bounds;
        let width = (right - left) as f32;
        let height = (bottom - top) as f32;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return;
        }
        let size = egui::vec2(width, height);
        let (viewport, response) = ui.allocate_exact_size(
            ui.available_size().max(egui::vec2(40.0, 40.0)),
            egui::Sense::click_and_drag(),
        );
        self.view.canvas_id = Some(response.id);
        let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(viewport));
        canvas.set_clip_rect(viewport.intersect(ui.clip_rect()));
        let ui = &mut canvas;
        self.view.geometry = Some((viewport, size));
        if let Some(factor) = self.view.zoom_request.take() {
            self.view.zoom_at(viewport, size, viewport.center(), factor);
        }
        let overlay_owned = self.inline_text.is_some() || self.alignment_overlay(ui, viewport);
        let panning = if overlay_owned {
            true
        } else {
            self.viewport_input(ui, viewport, &response, size)
        };
        let rect = self.view.rect(viewport, size);
        let scale = self.view.scale(viewport, size);
        let connection_owned = self.connection_input(ui, rect, &response, panning);
        let Some((uri, bytes)) = self.graph.drag_image.as_ref().or(self.image.as_ref()) else {
            return;
        };
        #[cfg(test)]
        {
            self.creation.canvas_rect = Some(rect);
        }
        let pixels = rect.width().max(rect.height()) * ui.pixels_per_point();
        let bucket = if pixels <= 512.0 {
            512
        } else if pixels <= 1024.0 {
            1024
        } else if pixels <= 2048.0 {
            2048
        } else {
            4096
        };
        if self.view.raster_bucket != Some(bucket) {
            ui.ctx().forget_image(uri);
            self.view.raster_bucket = Some(bucket);
        }
        let raster = (size * (bucket as f32 / size.x.max(size.y) / ui.pixels_per_point()))
            .max(egui::Vec2::splat(1.0));
        let image = egui::Image::from_bytes(uri.clone(), bytes.clone()).fit_to_exact_size(raster);
        let loaded = image.load_for_size(ui.ctx(), raster);
        match loaded {
            Ok(egui::load::TexturePoll::Ready { texture }) => {
                self.view.image_ready = true;
                ui.painter().image(
                    texture.id,
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            other => {
                self.view.image_ready = false;
                ui.painter().text(
                    viewport.center(),
                    egui::Align2::CENTER_CENTER,
                    match other {
                        Err(e) => e.to_string(),
                        _ => crate::i18n::tr("Rendering preview…").into(),
                    },
                    egui::FontId::proportional(14.0),
                    ui.visuals().text_color(),
                );
                return;
            }
        }
        appearance::image_bounds(ui.painter(), rect);
        if self.inline_text_canvas(ui, viewport, rect, &response) {
            return;
        }
        let notation_owned = self.notation_reorder_overlay(ui, viewport, panning);
        if self.specialized() {
            self.notation_canvas(ui, rect, &response, panning || notation_owned);
            return;
        }
        // Navigation may block gestures, but never the persistent selection overlays.
        let relation_owned = self.relation_canvas(
            ui,
            rect,
            &response,
            panning || connection_owned || notation_owned,
        ) || connection_owned
            || notation_owned;
        let preview = self.preview.as_ref().unwrap();
        if !panning
            && !relation_owned
            && response.clicked()
            && let Some(position) = response.interact_pointer_pos()
        {
            let x = left + ((position.x - rect.left()) / scale) as f64;
            let y = top + ((position.y - rect.top()) / scale) as f64;
            let node = laml_ui::selection::hit_node(&preview.nodes, x, y);
            let port = laml_ui::selection::hit_port(
                &preview.nodes,
                x,
                y,
                (7.0 / f64::from(scale)).max(5.0),
            );
            let toggle = ui.input(|input| input.modifiers.shift);
            if let Some(port) = port {
                self.selection.click_port(port, toggle);
            } else {
                self.selection
                    .click(node.map(|node| node.id.as_str()), toggle);
            }
            self.notation_ui.selected = None;
            self.property_drafts.clear();
            if self.rendered == Some(self.document.revision()) {
                self.source_jump = port
                    .and_then(|id| {
                        laml_ui::port_edit::find(&preview.diagram, id).map(|(_, p)| p.line)
                    })
                    .or_else(|| node.map(|node| node.line));
            }
        }
        let hit = |position: egui::Pos2| {
            let x = left + ((position.x - rect.left()) / scale) as f64;
            let y = top + ((position.y - rect.top()) / scale) as f64;
            laml_ui::selection::hit_port(&preview.nodes, x, y, (7.0 / f64::from(scale)).max(5.0))
                .map(str::to_string)
                .or_else(|| {
                    laml_ui::selection::hit_node(&preview.nodes, x, y).map(|node| node.id.clone())
                })
        };
        if !panning
            && enabled
            && self.creation.connect
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(position) = ui.input(|input| input.pointer.press_origin())
            && let Some(node) = hit(position)
        {
            self.creation.drag = Some((node, self.document.revision()));
        }
        let mut connect = None;
        if let Some((id, revision)) = self.creation.drag.clone() {
            if revision != self.document.revision() {
                self.creation.drag = None;
            } else {
                let start = preview
                    .nodes
                    .iter()
                    .flat_map(|node| &node.ports)
                    .find(|p| p.id == id)
                    .map(|p| (p.x, p.y))
                    .or_else(|| {
                        preview
                            .nodes
                            .iter()
                            .find(|node| node.id == id)
                            .map(|node| (node.x + node.width / 2.0, node.y + node.height / 2.0))
                    });
                if let Some((x, y)) = start
                    && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
                {
                    let start = rect.min + egui::vec2((x - left) as f32, (y - top) as f32) * scale;
                    ui.painter().arrow(
                        start,
                        pointer - start,
                        egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                    );
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if enabled
                        && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
                        && rect.contains(pointer)
                        && let Some(target) = hit(pointer)
                    {
                        connect = Some((id, target));
                    }
                    self.creation.drag = None;
                }
            }
        }
        for node in &preview.nodes {
            for port in &node.ports {
                if self.selection.ports.contains(&port.id) {
                    let point = rect.min
                        + egui::vec2((port.x - left) as f32, (port.y - top) as f32) * scale;
                    ui.painter().circle_stroke(
                        point,
                        (6.0 * scale).max(6.0),
                        egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                    );
                }
            }
            if self.selection.ids.contains(&node.id) {
                let min =
                    rect.min + egui::vec2((node.x - left) as f32, (node.y - top) as f32) * scale;
                let size = egui::vec2(node.width as f32, node.height as f32) * scale;
                let reference = self.selection.ids.last() == Some(&node.id);
                ui.painter().rect_stroke(
                    egui::Rect::from_min_size(min, size).expand(2.0),
                    2.0,
                    egui::Stroke::new(
                        if reference { 2.0 } else { 1.0 },
                        appearance::color(ui.ctx(), appearance::SELECTED),
                    ),
                    egui::StrokeKind::Outside,
                );
            }
        }
        if let Some((from, to)) = connect {
            self.create_relation(&from, &to);
        }
        let port_origin = ui.input(|i| i.pointer.press_origin()).is_some_and(|p| {
            let x = left + f64::from((p.x - rect.left()) / scale);
            let y = top + f64::from((p.y - rect.top()) / scale);
            self.preview.as_ref().is_some_and(|preview| {
                laml_ui::selection::hit_port(
                    &preview.nodes,
                    x,
                    y,
                    (7.0 / f64::from(scale)).max(5.0),
                )
                .is_some()
            })
        });
        if !panning && !relation_owned && !port_origin {
            self.placement_canvas(ui, rect, &response);
        }
        if self.placement.drag.is_some() {
            let status_rect = egui::Rect::from_min_size(
                viewport.left_top() + egui::vec2(12.0, 12.0),
                egui::vec2(260.0, 200.0),
            );
            let mut status = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("placement-status")
                    .max_rect(status_rect),
            );
            egui::Frame::new()
                .fill(appearance::color(ui.ctx(), appearance::PANEL))
                .inner_margin(10)
                .show(&mut status, |ui| self.placement_status(ui));
        }
        self.alignment_guides(ui, rect);
        self.connection_overlay(ui, rect, panning);
        self.graph_context_menu(ui, rect, &response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        editor: &mut Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| editor.canvas(ui),
        );
        output.textures_delta.clear();
        output
    }

    #[test]
    fn selection_and_image_boundary_remain_visible_outside_image_and_viewport() {
        let source = "semantics {\n A {\n @P port Out\n }\n B\n @r A -> B\n}\n";
        let mut editor = Editor::new(source.into(), None);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let preview = laml_ui::preview::build(source, &editor.path).unwrap();
        let line = preview.routes[0].source_line;
        editor.publish_preview(&ctx, preview);
        editor.view.zoom = Some(1.0);
        editor.selection.ids.push("A".into());
        editor.selection.ports.push("P".into());
        editor.selection.relations.push(line);
        for _ in 0..3 {
            frame(&mut editor, &ctx, vec![]);
        }
        assert!(editor.view.image_ready);
        let image = editor.creation.canvas_rect.unwrap();
        let (viewport, _) = editor.view.geometry.unwrap();
        let margin = viewport.min + egui::vec2(5.0, 5.0);
        assert!(!image.contains(margin));
        for event in [
            egui::Event::PointerMoved(image.center()),
            egui::Event::PointerMoved(margin),
            egui::Event::PointerMoved(egui::pos2(-10.0, -10.0)),
            egui::Event::PointerGone,
            egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ] {
            let output = frame(&mut editor, &ctx, vec![event]);
            assert!(
                output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Rect(rect) if rect.stroke.color == appearance::SELECTED
                )),
                "node selection must remain visible"
            );
            assert!(
                output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Circle(circle) if circle.stroke.color == egui::Color32::LIGHT_BLUE
                )),
                "port selection must remain visible"
            );
            assert!(
                output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Path(path) if path.stroke.width >= 3.0
                )),
                "relation selection must remain visible"
            );
            let dots: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Circle(circle)
                        if circle.fill == appearance::MUTED && circle.radius == 0.8 =>
                    {
                        Some(circle.center)
                    }
                    _ => None,
                })
                .collect();
            assert!(!dots.is_empty());
            for dot in dots {
                assert!(
                    dot.x == image.left()
                        || dot.x == image.right()
                        || dot.y == image.top()
                        || dot.y == image.bottom(),
                    "dots mark image edges, not a grid"
                );
            }
            assert_eq!(editor.selection.ids, ["A"]);
            assert_eq!(editor.selection.ports, ["P"]);
            assert_eq!(editor.selection.relations, [line]);
        }
        assert_eq!(editor.document.source(), source);
        assert_eq!(editor.document.revision(), 0);
    }
}
