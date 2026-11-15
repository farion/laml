//! Async transient SVGs; accepted document edits still use compiler preflight.
use super::*;
use laml_ui::backend::{
    async_client::Client,
    protocol::{self, Operation, Reply, Stamp},
};
pub(super) struct Remote {
    client: Client,
    base: Option<Stamp>,
    overlay: Option<(Stamp, String, egui::Rect)>,
}
impl Remote {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base: None,
            overlay: None,
        }
    }
    pub fn cancel(&mut self) {
        if let Some(stamp) = self.base.take() {
            self.client.forget(stamp);
        }
        if let Some((stamp, _, _)) = self.overlay.take() {
            self.client.forget(stamp);
        }
    }
    pub fn overlay(
        &mut self,
        id: u64,
        revision: u64,
        key: String,
        rect: egui::Rect,
        operation: Operation,
    ) -> Result<(), String> {
        if self.overlay.is_none() {
            let stamp = self
                .client
                .submit(id, revision, operation)
                .map_err(|e| e.to_string())?;
            self.overlay = Some((stamp, key, rect));
        }
        Ok(())
    }
}
impl Editor {
    pub(super) fn request_gesture_base(&mut self, line: usize) -> Result<(), String> {
        self.remote.cancel();
        let stamp = self
            .remote
            .client
            .submit(
                self.id,
                self.document.revision(),
                Operation::GestureRender {
                    presentation: self.compiler.presentation,
                    source: self.document.source().into(),
                    address: protocol::address(&self.path).map_err(|e| e.to_string())?,
                    route: None,
                    bounds: self.preview.as_ref().ok_or("No preview")?.bounds,
                    remove_line: Some(line),
                },
            )
            .map_err(|e| e.to_string())?;
        self.remote.base = Some(stamp);
        Ok(())
    }
    pub(super) fn poll_remote(&mut self, ctx: &egui::Context) {
        while let Some(response) = self.remote.client.poll() {
            let base = self.remote.base == Some(response.stamp);
            let overlay = self
                .remote
                .overlay
                .as_ref()
                .is_some_and(|(s, _, _)| *s == response.stamp);
            let frame = if overlay {
                self.remote.overlay.take()
            } else {
                None
            };
            if base {
                self.remote.base = None;
            }
            if (!base && !overlay)
                || self.graph.gesture.as_ref().is_none_or(|g| {
                    g.revision != response.stamp.revision || g.revision != self.document.revision()
                })
            {
                continue;
            }
            match response.result {
                Ok(Reply::GestureSvg { base: svg, overlay }) => {
                    if let Some(svg) = svg {
                        let uri = format!("bytes://gesture-base-{}.svg", self.id);
                        ctx.forget_image(&uri);
                        self.graph.drag_image = Some((uri, svg.into_bytes()));
                    }
                    if let (Some(svg), Some((_, key, rect))) = (overlay, frame) {
                        self.graph.motion.publish(ctx, self.id, key, rect, svg);
                    }
                }
                Err(error) => {
                    self.end_connection(ctx);
                    self.graph.report_errors = true;
                    self.command_error = Some(error.to_string());
                }
                _ => {
                    self.end_connection(ctx);
                    self.command_error = Some("Invalid gesture response".into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
