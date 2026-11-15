//! Server-workspace paths, rendered by the same egui shell (no local upload).
use super::*;
use std::task::Poll;
pub(super) struct Picker {
    path: String,
    extension: &'static str,
    opening: bool,
}
impl Picker {
    pub fn open() -> Self {
        Self {
            path: String::new(),
            extension: "laml",
            opening: true,
        }
    }
    pub fn save(name: String, extension: &'static str) -> Self {
        Self {
            path: name,
            extension,
            opening: false,
        }
    }
    pub fn poll(&mut self, ctx: &egui::Context) -> Poll<Option<PathBuf>> {
        let mut selected = None;
        let mut cancel = false;
        let dismissed = dialogs::show(
            ctx,
            "server-file-picker",
            if self.opening {
                "Open server file"
            } else {
                "Save on server"
            },
            520.0,
            |ui| {
                ui.label("Path relative to the configured server workspace:");
                ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
                ui.small(format!("File type: .{}", self.extension));
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui
                        .add_enabled(
                            !self.path.trim().is_empty(),
                            egui::Button::new(if self.opening { "Open" } else { "Choose path" }),
                        )
                        .clicked()
                    {
                        selected = Some(PathBuf::from(self.path.trim()));
                    }
                });
            },
        );
        if selected.is_some() || cancel || dismissed {
            Poll::Ready(selected)
        } else {
            Poll::Pending
        }
    }
}
