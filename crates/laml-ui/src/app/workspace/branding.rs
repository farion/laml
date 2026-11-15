//! Shared vector wordmark: the editor and README use identical artwork.
use super::*;

pub(super) fn wordmark(ui: &mut egui::Ui, width: f32) {
    let (uri, svg): (&str, &'static [u8]) = if ui.visuals().dark_mode {
        (
            "bytes://laml-wordmark-dark.svg",
            include_bytes!("../../../../../docs/images/laml-logo-dark.svg"),
        )
    } else {
        (
            "bytes://laml-wordmark-light.svg",
            include_bytes!("../../../../../docs/images/laml-logo.svg"),
        )
    };
    let width = width.min(ui.available_width());
    ui.add(
        egui::Image::from_bytes(uri, svg)
            .fit_to_exact_size(egui::vec2(width, width * 480.0 / 1560.0))
            .alt_text("LAML"),
    );
}
