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

/// Normalize once while retaining the actual text geometry before outlining.
/// Bounds are the compiler SVG viewBox; usvg measures in viewport coordinates.
pub fn outlines_for_editor(
    svg: &str,
    bounds: (f64, f64, f64, f64),
) -> Result<(String, Vec<crate::authoring::TextRegion>), String> {
    fn collect(group: &usvg::Group, out: &mut Vec<crate::authoring::TextRegion>) {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => collect(group, out),
                usvg::Node::Text(text) => {
                    let b = text.abs_bounding_box();
                    out.push(crate::authoring::TextRegion {
                        text: text
                            .chunks()
                            .iter()
                            .map(|c| c.text())
                            .collect::<Vec<_>>()
                            .join("\n"),
                        x: f64::from(b.x()),
                        y: f64::from(b.y()),
                        width: f64::from(b.width()),
                        height: f64::from(b.height()),
                    });
                }
                _ => {}
            }
        }
    }
    let document = tree(svg)?;
    let mut texts = Vec::new();
    collect(document.root(), &mut texts);
    let sx = (bounds.2 - bounds.0) / f64::from(document.size().width());
    let sy = (bounds.3 - bounds.1) / f64::from(document.size().height());
    for text in &mut texts {
        text.x = bounds.0 + text.x * sx;
        text.y = bounds.1 + text.y * sy;
        text.width *= sx;
        text.height *= sy;
    }
    let svg = crate::export_metadata::preserve_svg_notice(
        svg,
        document.to_string(&usvg::WriteOptions::default()),
    )?;
    Ok((svg, texts))
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

#[cfg(not(target_arch = "wasm32"))]
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
