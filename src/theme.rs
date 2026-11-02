//! Theme defaults are resolved before explicit LSS declarations. A base color
//! supplies a hue; derived colors are chosen for legibility, not by copying its
//! original lightness. Each explicit LSS property replaces only that property.

use crate::syntax::NodeKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    Plain,
    #[default]
    Modern,
    ModernMono,
    PlainDark,
    ModernDark,
    ModernMonoDark,
}

impl Theme {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "plain" => Self::Plain,
            "modern" => Self::Modern,
            "modern-mono" => Self::ModernMono,
            "plain-dark" => Self::PlainDark,
            "modern-dark" => Self::ModernDark,
            "modern-mono-dark" => Self::ModernMonoDark,
            _ => return None,
        })
    }

    pub fn dark(self) -> bool {
        matches!(
            self,
            Self::PlainDark | Self::ModernDark | Self::ModernMonoDark
        )
    }

    pub fn mono(self) -> bool {
        matches!(self, Self::ModernMono | Self::ModernMonoDark)
    }

    pub fn modern(self) -> bool {
        !matches!(self, Self::Plain | Self::PlainDark)
    }

    pub fn canvas(self) -> &'static str {
        if self.dark() { "#000000" } else { "#ffffff" }
    }

    pub fn edge(self) -> &'static str {
        if self.mono() {
            if self.dark() { "#aaaaaa" } else { "#555555" }
        } else if self.dark() {
            "#aebccf"
        } else {
            "#536477"
        }
    }

    pub fn default_base(self, kind: NodeKind) -> &'static str {
        if self.mono() {
            return "#808080";
        }
        match kind {
            NodeKind::Note | NodeKind::StateChoice | NodeKind::ActivityDecision => "#d99520",
            NodeKind::Package | NodeKind::Profile | NodeKind::UseCaseSystem => "#8394a7",
            NodeKind::UseCaseActor => "#7266aa",
            _ => "#4e9fda",
        }
    }

    pub fn colors(self, base: &str) -> Palette {
        let (r, g, b) = parse_hex(base).unwrap_or((78, 159, 218));
        let (h, s) = hue_saturation(r, g, b);
        // Explicit base-color remains chromatic in a mono theme. Only its
        // default bases are gray; other explicit overrides are not filtered.
        let saturation = if s < 1e-9 { 0.0 } else { s.clamp(0.30, 0.70) };
        if self.dark() {
            Palette {
                fill: hsl(h, saturation * 0.40, 0.17),
                border: hsl(h, saturation * 0.75, 0.65),
                divider: hsl(h, saturation * 0.35, 0.36),
                text: hsl(h, saturation * 0.45, 0.93),
            }
        } else {
            Palette {
                fill: hsl(h, saturation * 0.48, 0.95),
                border: hsl(h, saturation * 0.75, 0.42),
                divider: hsl(h, saturation * 0.45, 0.82),
                text: hsl(h, saturation * 0.55, 0.17),
            }
        }
    }
}

pub struct Palette {
    pub fill: String,
    pub border: String,
    pub divider: String,
    pub text: String,
}

fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&h[0..2], 16).ok()?,
        u8::from_str_radix(&h[2..4], 16).ok()?,
        u8::from_str_radix(&h[4..6], 16).ok()?,
    ))
}

fn hue_saturation(r: u8, g: u8, b: u8) -> (f64, f64) {
    let (r, g, b) = (
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if d < 1e-9 {
        return (0.0, 0.0);
    }
    let l = (max + min) / 2.0;
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s)
}

fn hsl(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h / 60.0 {
        v if v < 1.0 => (c, x, 0.0),
        v if v < 2.0 => (x, c, 0.0),
        v if v < 3.0 => (0.0, c, x),
        v if v < 4.0 => (0.0, x, c),
        v if v < 5.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    format!(
        "#{:02x}{:02x}{:02x}",
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8
    )
}

/// Recolor legacy renderer literals, without touching any colors explicitly
/// chosen in LSS. This is a bridge while specialized renderers move their
/// defaults into typed paints; it never rewrites arbitrary user colors.
pub fn tint_svg_defaults(svg: &str, theme: Theme, protected: &[String]) -> String {
    if theme == Theme::Plain {
        return svg.into();
    }
    let palette = theme.colors(theme.default_base(NodeKind::Generic));
    let container = theme.colors(theme.default_base(NodeKind::Package));
    let text = if theme.dark() {
        palette.text.as_str()
    } else if theme.mono() {
        "#252525"
    } else {
        "#17212e"
    };
    let border = if theme.dark() {
        palette.border.as_str()
    } else {
        theme.edge()
    };
    let fill = if theme.dark() {
        palette.fill.as_str()
    } else {
        "#ffffff"
    };
    let mappings = [
        ("#20252b", border),
        ("#263548", border),
        ("#395373", border),
        ("#17212e", text),
        ("#182330", text),
        ("#66717d", theme.edge()),
        ("#454d55", theme.edge()),
        ("#f8fbff", container.fill.as_str()),
        ("#c8d2df", container.divider.as_str()),
        ("white", fill),
        ("#fff", fill),
    ];
    let mut result = svg.to_owned();
    for (from, to) in mappings {
        if protected
            .iter()
            .any(|color| color.eq_ignore_ascii_case(from))
        {
            continue;
        }
        for attr in ["fill", "stroke"] {
            result = result.replace(&format!("{attr}=\"{from}\""), &format!("{attr}=\"{to}\""));
            result = result.replace(&format!("{attr}:{from};"), &format!("{attr}:{to};"));
        }
    }
    // Text lacking an explicit paint would otherwise use the SVG viewer's
    // default black, invisible against an opaque black canvas.
    if theme.dark() {
        let end = result.find('>').map_or(0, |p| p + 1);
        result.insert_str(
            end,
            &format!("<style>text:not([fill]) {{fill:{text}}}</style>"),
        );
    }
    result
}

/// Parse the two shared visual settings for specialized diagram parsers that
/// currently own independent settings grammars. Preserve all line numbers.
pub type VisualSettings = (String, Theme, bool, Option<String>, Option<String>);

pub fn preprocess_settings(source: &str) -> Result<VisualSettings, String> {
    let mut out = String::new();
    let mut inside = false;
    let mut seen_theme = false;
    let mut seen_background = false;
    let mut theme = Theme::default();
    let mut transparent = false;
    let mut font_file = None;
    let mut font_italic_file = None;
    for (index, raw) in source.lines().enumerate() {
        let text = crate::lex::without_comment(raw).trim();
        if text == "settings {" {
            inside = true;
        }
        if inside && text == "}" {
            inside = false;
        }
        let setting = if inside { text.split_once(':') } else { None };
        match setting.map(|(key, value)| (key.trim(), value.trim())) {
            Some(("theme", value)) => {
                if seen_theme {
                    return Err(format!("{}: duplicate theme setting", index + 1));
                }
                theme =
                    Theme::parse(value).ok_or_else(|| format!("{}: invalid theme", index + 1))?;
                seen_theme = true;
                out.push('\n');
            }
            Some(("background", value)) => {
                if seen_background {
                    return Err(format!("{}: duplicate background setting", index + 1));
                }
                transparent = match value {
                    "transparent" => true,
                    "opaque" => false,
                    _ => {
                        return Err(format!(
                            "{}: background must be opaque or transparent",
                            index + 1
                        ));
                    }
                };
                seen_background = true;
                out.push('\n');
            }
            Some(("font-file", value)) | Some(("font-italic-file", value)) => {
                let name =
                    crate::lex::quoted_string(value).map_err(|e| format!("{}: {e}", index + 1))?;
                let slot = if text.starts_with("font-italic-file:") {
                    &mut font_italic_file
                } else {
                    &mut font_file
                };
                if slot.is_some() {
                    return Err(format!("{}: duplicate font file setting", index + 1));
                }
                *slot = Some(name);
                out.push('\n');
            }
            _ => {
                out.push_str(raw);
                out.push('\n');
            }
        }
    }
    Ok((out, theme, transparent, font_file, font_italic_file))
}

/// Insert an explicit canvas in every format; without it PNG keeps alpha and
/// SVG/PDF can be placed on an arbitrary underlying surface.
pub fn canvas(svg: &str, theme: Theme, transparent: bool) -> Result<String, String> {
    if transparent {
        return Ok(svg.replacen("<svg ", "<svg data-laml-transparent=\"true\" ", 1));
    }
    let end = svg.find('>').ok_or("generated SVG has no root element")? + 1;
    if !svg.starts_with("<svg ") {
        return Err("generated SVG has no root element".into());
    }
    let root = svg[..end].replacen(
        "<svg ",
        &format!("<svg data-laml-canvas=\"{}\" ", theme.canvas()),
        1,
    );
    let view = root
        .split("viewBox=\"")
        .nth(1)
        .and_then(|v| v.split('"').next())
        .ok_or("generated SVG has no viewBox")?;
    let parts: Vec<_> = view.split_whitespace().collect();
    if parts.len() != 4 {
        return Err("generated SVG has invalid viewBox".into());
    }
    let numbers = parts
        .iter()
        .map(|part| part.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "generated SVG has invalid viewBox")?;
    // Overscan beyond the viewport avoids a half-pixel fringe where a black
    // rectangle is antialiased against a light PDF/PNG viewer's background.
    Ok(format!(
        "{root}<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>{}",
        numbers[0] - 2.0,
        numbers[1] - 2.0,
        numbers[2] + 4.0,
        numbers[3] + 4.0,
        theme.canvas(),
        &svg[end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derives_dark_and_light_from_a_single_base() {
        let light = Theme::Modern.colors("#00ff00");
        let dark = Theme::ModernDark.colors("#00ff00");
        assert_ne!(light.fill, dark.fill);
        assert_ne!(light.fill, light.border);
        let gray = Theme::ModernMono.colors("#808080").fill;
        assert_eq!(gray, Theme::Modern.colors("#808080").fill);
        assert_eq!(&gray[1..3], &gray[3..5]);
        assert_eq!(&gray[3..5], &gray[5..7]);
    }
    #[test]
    fn canvas_preserves_alpha_when_requested() {
        let svg = "<svg viewBox=\"-1 2 30 40\"></svg>";
        assert!(
            canvas(svg, Theme::Modern, true)
                .unwrap()
                .contains("data-laml-transparent=\"true\"")
        );
        assert!(
            canvas(svg, Theme::PlainDark, false)
                .unwrap()
                .contains("fill=\"#000000\"")
        );
    }
}
