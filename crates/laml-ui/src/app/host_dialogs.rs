//! Native picker adapter. Poll the async platform API once per frame; never
//! block egui or move macOS UI operations onto an arbitrary worker thread.
use super::*;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub(super) struct Picker(Pin<Box<dyn Future<Output = Option<PathBuf>>>>);
struct Repaint(egui::Context);
impl Wake for Repaint {
    fn wake(self: Arc<Self>) {
        self.0.request_repaint();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.request_repaint();
    }
}
impl Picker {
    pub fn open() -> Self {
        Self(Box::pin(async {
            rfd::AsyncFileDialog::new()
                .add_filter("LAML diagrams", &["laml"])
                .pick_file()
                .await
                .map(|file| file.path().to_path_buf())
        }))
    }
    pub fn save(name: String, extension: &'static str) -> Self {
        Self(Box::pin(async move {
            rfd::AsyncFileDialog::new()
                .set_file_name(name)
                .add_filter(extension, &[extension])
                .save_file()
                .await
                .map(|file| file.path().to_path_buf())
        }))
    }
    pub fn poll(&mut self, ctx: &egui::Context) -> Poll<Option<PathBuf>> {
        let waker = Waker::from(Arc::new(Repaint(ctx.clone())));
        self.0.as_mut().poll(&mut Context::from_waker(&waker))
    }
}
