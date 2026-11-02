//! SVG normalization and vector/raster exports. All formats share one layout.

use resvg::{tiny_skia, usvg};

fn tree(svg: &str) -> Result<usvg::Tree, String> {
    let mut options = usvg::Options::default();
    // Resolve exactly the same bundled face used for measuring and for SVG.
    // Do not rely on fonts installed on the exporting machine.
    crate::text::with_face(false, |font, _| {
        options.fontdb_mut().load_font_data(font.to_vec())
    })?;
    if let Ok(()) = crate::text::with_face(true, |font, _| {
        options.fontdb_mut().load_font_data(font.to_vec())
    }) {
        // Some custom fonts intentionally supply only a regular face.
    }
    usvg::Tree::from_str(svg, &options).map_err(|e| format!("cannot parse generated SVG: {e}"))
}

pub fn outlines(svg: &str) -> Result<String, String> {
    let document = tree(svg)?;
    crate::export_metadata::preserve_svg_notice(
        svg,
        document.to_string(&usvg::WriteOptions::default()),
    )
}

pub fn png(svg: &str) -> Result<Vec<u8>, String> {
    let document = tree(svg)?;
    let size = document.size().to_int_size();
    // Avoid unbounded allocations from huge styled boxes or source labels.
    if u64::from(size.width()) * u64::from(size.height()) > 64_000_000 {
        return Err("PNG canvas exceeds 64 million pixels".into());
    }
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or("PNG canvas cannot be allocated")?;
    // Keep the original public export API white unless transparency was
    // explicitly requested; the CLI marks transparent output on the SVG root.
    if !svg.contains("data-laml-transparent=\"true\"") {
        pixmap.fill(if svg.contains("data-laml-canvas=\"#000000\"") {
            tiny_skia::Color::BLACK
        } else {
            tiny_skia::Color::WHITE
        });
    }
    resvg::render(
        &document,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let encoded = pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding failed: {e}"))?;
    if crate::export_metadata::uses_phosphor(svg) {
        crate::export_metadata::png_notice(encoded)
    } else {
        Ok(encoded)
    }
}

pub fn pdf(svg: &str) -> Result<Vec<u8>, String> {
    let document = tree(svg)?;
    let converted = svg2pdf::to_pdf(
        &document,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions { dpi: 96.0 },
    )
    .map_err(|e| format!("PDF conversion failed: {e}"))?;
    if crate::export_metadata::uses_phosphor(svg) {
        crate::export_metadata::pdf_notice(converted)
    } else {
        Ok(converted)
    }
}
