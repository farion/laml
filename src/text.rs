//! Font-backed measurement and grapheme-safe label truncation.

use harfrust::{FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use std::{cell::RefCell, path::Path};
use unicode_segmentation::UnicodeSegmentation;

pub const FONT: &[u8] = include_bytes!("../assets/Inter-Regular.otf");
pub const ITALIC_FONT: &[u8] = include_bytes!("../assets/Inter-Italic.otf");
pub const GEIST: &[u8] = include_bytes!("../assets/Geist-Regular.otf");
pub const GEIST_ITALIC: &[u8] = include_bytes!("../assets/Geist-Italic.otf");

struct Faces {
    regular: Vec<u8>,
    italic: Option<Vec<u8>>,
    family: String,
}
thread_local! {
    static ACTIVE: RefCell<Option<Faces>> = const { RefCell::new(None) };
}

/// One set of font bytes is used for shaping and every output format.
/// Local font files are resolved relative to the diagram and may not escape
/// its directory, including through symlinks.
pub fn with_fonts<T>(
    source: &Path,
    modern: bool,
    regular: Option<&str>,
    italic: Option<&str>,
    build: impl FnOnce() -> T,
) -> Result<T, String> {
    fn local(source: &Path, file: &str) -> Result<Vec<u8>, String> {
        let root = source
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let path = root
            .join(file)
            .canonicalize()
            .map_err(|e| format!("font file '{file}': {e}"))?;
        if !path.starts_with(&root) {
            return Err(format!("font file '{file}' escapes diagram directory"));
        }
        if path
            .metadata()
            .map_err(|e| format!("font file '{file}': {e}"))?
            .len()
            > 16 * 1024 * 1024
        {
            return Err(format!("font file '{file}' exceeds 16 MiB"));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("font file '{file}': {e}"))?;
        FontRef::new(&bytes)
            .map_err(|e| format!("font file '{file}' is not a supported font: {e}"))?;
        Ok(bytes)
    }
    if italic.is_some() && regular.is_none() {
        return Err("font-italic-file requires font-file".into());
    }
    fn family(bytes: &[u8]) -> Result<String, String> {
        let mut database = resvg::usvg::fontdb::Database::new();
        database.load_font_data(bytes.to_vec());
        let name = database
            .faces()
            .next()
            .and_then(|face| face.families.first())
            .map(|(name, _)| name.clone())
            .ok_or("font has no family name")?;
        if name.contains(['\"', '\'', '<', '>', '&']) || name.chars().any(char::is_control) {
            return Err("font family name contains unsupported XML characters".into());
        }
        Ok(name)
    }
    let faces = if let Some(file) = regular {
        let regular = local(source, file)?;
        let italic = italic.map(|f| local(source, f)).transpose()?;
        let name = family(&regular)?;
        if let Some(face) = &italic
            && family(face)? != name
        {
            return Err("italic and regular font files must belong to the same family".into());
        }
        Faces {
            regular,
            italic,
            family: name,
        }
    } else if modern {
        Faces {
            regular: GEIST.to_vec(),
            italic: Some(GEIST_ITALIC.to_vec()),
            family: "Geist".into(),
        }
    } else {
        Faces {
            regular: FONT.to_vec(),
            italic: Some(ITALIC_FONT.to_vec()),
            family: "Inter".into(),
        }
    };
    ACTIVE.with(|state| {
        if state.borrow().is_some() {
            return Err("nested font selection is unsupported".into());
        }
        *state.borrow_mut() = Some(faces);
        struct Reset<'a>(&'a RefCell<Option<Faces>>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                *self.0.borrow_mut() = None;
            }
        }
        let _reset = Reset(state);
        Ok(build())
    })
}

pub fn with_face<T>(italic: bool, f: impl FnOnce(&[u8], &str) -> T) -> Result<T, String> {
    ACTIVE.with(|state| {
        let borrowed = state.borrow();
        if let Some(faces) = borrowed.as_ref() {
            let data = if italic {
                faces
                    .italic
                    .as_deref()
                    .ok_or("selected font has no italic face")?
            } else {
                &faces.regular
            };
            Ok(f(data, &faces.family))
        } else {
            Ok(f(if italic { ITALIC_FONT } else { FONT }, "Inter"))
        }
    })
}

pub fn measure(text: &str, font_size: f64) -> Result<f64, String> {
    with_face(false, |face, _| measure_face(text, font_size, face))?
}

pub fn measure_italic(text: &str, font_size: f64) -> Result<f64, String> {
    with_face(true, |face, _| measure_face(text, font_size, face))?
}

fn measure_face(text: &str, font_size: f64, face: &[u8]) -> Result<f64, String> {
    if text.is_empty() {
        return Ok(0.0);
    }
    let font = FontRef::new(face).map_err(|e| format!("selected font cannot be read: {e}"))?;
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let glyphs = shaper.shape(buffer, ShapeOptions::default());
    if glyphs.glyph_infos().iter().any(|glyph| glyph.glyph_id == 0) {
        return Err(format!("selected font cannot represent label '{text}'"));
    }
    let advances: i64 = glyphs
        .glyph_positions()
        .iter()
        .map(|glyph| i64::from(glyph.x_advance))
        .sum();
    Ok((advances.abs() as f64) * font_size / f64::from(shaper.units_per_em()))
}

/// Swap the renderer's Inter family for the exact face used during shaping.
/// The generated SVG embeds the selected font for selectable, portable text.
pub fn svg_fonts(svg: &str) -> Result<String, String> {
    use base64::Engine;
    let original = base64::engine::general_purpose::STANDARD.encode(FONT);
    let original_italic = base64::engine::general_purpose::STANDARD.encode(ITALIC_FONT);
    fn embedded(svg: &str, original: &str, selected: &[u8]) -> String {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(selected);
        let changed = svg.replace(original, &encoded);
        if selected.starts_with(&[0, 1, 0, 0]) {
            changed.replace(
                &format!("data:font/otf;base64,{encoded}') format('opentype')"),
                &format!("data:font/ttf;base64,{encoded}') format('truetype')"),
            )
        } else {
            changed
        }
    }
    with_face(false, |regular, family| {
        embedded(svg, &original, regular)
            .replace(
                "font-family=\"Inter\"",
                &format!("font-family=\"{family}\""),
            )
            .replace("font-family: 'Inter'", &format!("font-family: '{family}'"))
    })
    .and_then(|result| {
        if result.contains(&original_italic) {
            with_face(true, |italic, _| {
                embedded(&result, &original_italic, italic)
            })
        } else {
            Ok(result)
        }
    })
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
