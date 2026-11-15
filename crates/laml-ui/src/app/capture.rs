//! Explicit automation capture of this application's framebuffer, never the desktop.
use super::*;
pub(super) struct Capture {
    path: PathBuf,
    scene: Option<String>,
    requested: bool,
    frames: usize,
    success: std::sync::Arc<std::sync::atomic::AtomicBool>,
    result: Option<mpsc::Receiver<std::sync::Arc<egui::ColorImage>>>,
}
impl Editor {
    pub fn set_capture(
        &mut self,
        path: Option<PathBuf>,
        success: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        self.capture = path.map(|path| Capture {
            path,
            scene: None,
            requested: false,
            frames: 0,
            success,
            result: None,
        });
    }
    pub fn set_capture_view(&mut self, scene: Option<String>) {
        if let Some(capture) = &mut self.capture {
            capture.scene = scene;
        }
    }
    pub(super) fn capture_tick(&mut self, ctx: &egui::Context) {
        if self.rendered == Some(self.document.revision())
            && self.view.image_ready
            && let Some(scene) = self.capture.as_mut().and_then(|c| c.scene.take())
        {
            if scene == "add-node" {
                self.open_node_dialog();
            } else {
                if let Some(node) = self.preview.as_ref().and_then(|p| p.nodes.first()) {
                    self.selection.click(Some(&node.id), false);
                }
                self.inspector_tab = if matches!(scene.as_str(), "style" | "icons") {
                    inspector_ui::Tab::Style
                } else {
                    inspector_ui::Tab::General
                };
                if scene == "icons" {
                    self.open_icon_picker(self.selection.ids.clone(), None);
                }
            }
        }
        let Some(capture) = &mut self.capture else {
            return;
        };
        if let Some(error) = &self.error {
            eprintln!("Native capture compiler failed: {error}");
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        ctx.request_repaint();
        let image = capture.result.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(image) = image {
            let result = (|| -> Result<(), String> {
                let path = laml_ui::output::target(
                    &capture.path,
                    laml_ui::output::Format::Png,
                    std::slice::from_ref(&self.path),
                )?;
                let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let mut bytes = vec![];
                {
                    let mut encoder =
                        png::Encoder::new(&mut bytes, image.size[0] as u32, image.size[1] as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .map_err(|e| e.to_string())?
                        .write_image_data(&rgba)
                        .map_err(|e| e.to_string())?;
                }
                laml_ui::output::write(&path, &bytes, None)
            })();
            match result {
                Ok(()) => capture
                    .success
                    .store(true, std::sync::atomic::Ordering::Release),
                Err(e) => eprintln!("Native capture failed: {e}"),
            };
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.rendered == Some(self.document.revision()) && self.view.image_ready {
            capture.frames += 1;
            if capture.frames >= 4 && !capture.requested {
                capture.requested = true;
                let (tx, rx) = mpsc::channel();
                capture.result = Some(rx);
                let repaint = ctx.clone();
                ctx.request_screenshot(move |image| {
                    let _ = tx.send(image);
                    repaint.request_repaint();
                });
            }
        }
    }
}
