//! Font-backed measurement and grapheme-safe label truncation.

use harfrust::{FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use unicode_segmentation::UnicodeSegmentation;

pub const FONT: &[u8] = include_bytes!("../assets/Inter-Regular.otf");
pub const ITALIC_FONT: &[u8] = include_bytes!("../assets/Inter-Italic.otf");

pub fn measure(text: &str, font_size: f64) -> Result<f64, String> {
    measure_face(text, font_size, FONT)
}

pub fn measure_italic(text: &str, font_size: f64) -> Result<f64, String> {
    measure_face(text, font_size, ITALIC_FONT)
}

fn measure_face(text: &str, font_size: f64, face: &[u8]) -> Result<f64, String> {
    if text.is_empty() {
        return Ok(0.0);
    }
    let font = FontRef::new(face).map_err(|e| format!("bundled Inter font cannot be read: {e}"))?;
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let glyphs = shaper.shape(buffer, ShapeOptions::default());
    if glyphs.glyph_infos().iter().any(|glyph| glyph.glyph_id == 0) {
        return Err(format!(
            "bundled Inter font cannot represent label '{text}'"
        ));
    }
    let advances: i64 = glyphs
        .glyph_positions()
        .iter()
        .map(|glyph| i64::from(glyph.x_advance))
        .sum();
    Ok((advances.abs() as f64) * font_size / f64::from(shaper.units_per_em()))
}

/// Shorten at a grapheme boundary and add exactly one ellipsis.
pub fn ellipsize(text: &str, max_width: f64, size: f64, force: bool) -> Result<String, String> {
    if !force && measure(text, size)? <= max_width {
        return Ok(text.into());
    }
    if measure("…", size)? > max_width {
        return Err("box too narrow to fit an ellipsis".into());
    }
    let graphemes: Vec<&str> = text.graphemes(true).collect();
    for keep in (0..=graphemes.len()).rev() {
        // A vertical overflow forces a visible ellipsis even if the last
        // displayed line itself fits without truncation.
        let candidate = format!("{}…", graphemes[..keep].concat());
        if measure(&candidate, size)? <= max_width {
            return Ok(candidate);
        }
    }
    Err("box too narrow to fit an ellipsis".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_width_increases_with_text() {
        assert!(measure("Backend API", 14.0).unwrap() > measure("API", 14.0).unwrap());
    }
    #[test]
    fn truncate_does_not_split_combining_sequence() {
        let width = measure("…", 14.0).unwrap() + 1.0;
        assert_eq!(ellipsize("e\u{301}f", width, 14.0, true).unwrap(), "…");
    }
}
