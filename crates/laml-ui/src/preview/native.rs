use super::{PickLabel, PickRoute, Preview};
use crate::diagnostic::Diagnostic;
use std::path::Path;

/// Rerender the already-computed geometry without one route, preserving the
/// original viewBox. Used only while dragging that route's endpoint.
pub fn without_route(
    preview: &Preview,
    source: &str,
    path: &Path,
    line: usize,
) -> Result<String, String> {
    let (_, _, _, font, italic) = laml::theme::preprocess_settings(source)?;
    laml::text::with_fonts(path, preview.styles.theme.modern(), font.as_deref(), italic.as_deref(), || {
        let routes: Vec<_> = preview
            .routes
            .iter()
            .filter(|r| r.source_line != line)
            .cloned()
            .collect();
        let mut svg = laml::svg::render_with_theme(
            &preview.nodes,
            &routes,
            laml::svg::TextMode::Embedded,
            preview.diagram.settings.theme,
        )?;
        let (left, top, right, bottom) = preview.bounds;
        let end = svg.find('>').ok_or("Missing SVG root")?;
        svg.replace_range(..=end, &format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">",
            right-left, bottom-top, left, top, right-left, bottom-top));
        let protected: Vec<String> = preview
            .styles
            .rules
            .iter()
            .flat_map(|r| &r.declarations)
            .filter_map(|d| match &d.value {
                laml::styles::Value::Color(color) => Some(color.clone()),
                _ => None,
            })
            .collect();
        let svg = laml::theme::tint_svg_defaults(&svg, preview.diagram.settings.theme, &protected);
        let svg = laml::text::svg_fonts(&svg)?;
        let svg = laml::theme::canvas(&svg, preview.diagram.settings.theme, preview.diagram.settings.transparent)?;
        let svg = laml::export::outlines(&svg)?;
        if preview.diagram.settings.transparent {
            laml::theme::canvas(&svg, preview.diagram.settings.theme, true)
        } else { Ok(svg) }
    })?
}

pub fn build(source: &str, path: &Path) -> Result<Preview, String> {
    build_diagnostic(source, path).map_err(|e| e.to_string())
}

pub fn build_diagnostic(source: &str, path: &Path) -> Result<Preview, Diagnostic> {
    build_impl(source, path, None, None)
}

/// Editor-only modern-dark/transparent rendering. The source is never rewritten;
/// document builds and exports continue to use `build` / `build_diagnostic`.
pub fn build_dark(source: &str, path: &Path) -> Result<Preview, String> {
    build_dark_diagnostic(source, path).map_err(|e| e.to_string())
}

pub fn build_dark_diagnostic(source: &str, path: &Path) -> Result<Preview, Diagnostic> {
    build_impl(source, path, Some(laml::theme::Theme::ModernDark), None)
}

pub fn build_relations(
    source: &str, path: &Path, theme_override: Option<laml::theme::Theme>, previous: &Preview,
) -> Result<Preview, Diagnostic> {
    build_impl(source, path, theme_override, Some(previous))
}

pub fn build_presented(
    source: &str, path: &Path, presentation: crate::backend::Presentation,
) -> Result<Preview, Diagnostic> {
    build_impl(source, path, presentation.theme_override(), None)
}

fn build_impl(
    source: &str, path: &Path, theme_override: Option<laml::theme::Theme>, previous: Option<&Preview>,
) -> Result<Preview, Diagnostic> {
    let (_, theme, transparent, font, italic) =
        laml::theme::preprocess_settings(source).map_err(Diagnostic::from)?;
    let (theme, transparent) = if let Some(theme) = theme_override {
        (theme, true)
    } else {
        (theme, transparent)
    };
    laml::text::with_fonts(
        path,
        theme.modern(),
        font.as_deref(),
        italic.as_deref(),
        || {
            let notation = crate::notation::Notation::of(source);
            if !notation.graph() {
                return super::special::build(source, path, notation, theme, transparent);
            }
            let mut diagram = crate::notation::parse_graph(source)
                .map_err(|e| Diagnostic::at(path, e.line, e.message))?;
            diagram.settings.theme = theme;
            diagram.settings.transparent = transparent;
            let styles = laml::styles::load(&diagram, path)
                .map_err(|e| Diagnostic::in_file(e.path, e.line, e.message))?;
            laml::layout::validate_ordering(&diagram)
                .map_err(|e| Diagnostic::at(path, e.line, e.message))?;
            let (mut boxes, routes) = match previous
                .and_then(|old| super::incremental::geometry(&diagram, &styles, old))
            {
                Some(geometry) => geometry,
                None => laml::route::place_and_route(&diagram, &styles)
                    .map_err(|e| Diagnostic::at(path, e.line, e.message))?,
            };
            let route_paths = routes
                .iter()
                .map(|route| {
                    Ok(PickRoute {
                        line: route.source_line,
                        paths: laml::svg::route_pick_paths(route, &boxes, theme)?,
                        labels: route
                            .label
                            .iter()
                            .chain(&route.end_labels)
                            .map(|label| {
                                let (width, height) = if label.angle.abs() > 1e-9 {
                                    (
                                        laml::text::measure(&label.text, route.font_size)?,
                                        route.font_size * 1.25,
                                    )
                                } else {
                                    (label.width, label.height)
                                };
                                Ok(PickLabel {
                                    x: label.x + label.width / 2.0,
                                    y: label.y + label.height / 2.0,
                                    width: width + if theme.modern() { 8.0 } else { 0.0 },
                                    height: height + if theme.modern() { 4.0 } else { 0.0 },
                                    angle: label.angle,
                                })
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

            let mode = laml::svg::TextMode::Embedded;
            let svg = match notation {
                crate::notation::Notation::Communication => {
                    let mut doc = laml::communication::parse(source)
                        .map_err(|e| Diagnostic::at(path, e.line, e.message))?;
                    doc.graph = diagram.clone();
                    laml::communication::render(&doc, &boxes, &routes, mode)
                }
                crate::notation::Notation::InteractionOverview => {
                    let mut doc = laml::interaction_overview::parse(source)
                        .map_err(|e| Diagnostic::at(path, e.line, e.message))?;
                    doc.graph = diagram.clone();
                    laml::interaction_overview::render(&doc, &boxes, &routes, mode)
                }
                ty if ty.c4() => laml::svg::c4::render(&boxes, &routes, mode, theme, &diagram),
                _ => laml::svg::render_with_theme(&boxes, &routes, mode, theme),
            }?;
            let bounds = super::special::bounds(&svg)?;
            let protected: Vec<String> = styles
                .rules
                .iter()
                .flat_map(|rule| &rule.declarations)
                .filter_map(|decl| match &decl.value {
                    laml::styles::Value::Color(color) => Some(color.clone()),
                    _ => None,
                })
                .collect();
            let svg = laml::theme::tint_svg_defaults(&svg, theme, &protected);
            let svg = laml::text::svg_fonts(&svg)?;
            let svg = laml::theme::canvas(&svg, theme, transparent)?;
            let svg = laml::export::outlines(&svg)?;
            // Outlining normalizes the SVG and drops custom root attributes.
            // Retain the transparent-canvas contract for PNG/PDF consumers too.
            let svg = if transparent {
                laml::theme::canvas(&svg, theme, true)?
            } else {
                svg
            };
            for node in &mut boxes {
                node.fill = rendered_color(&node.fill, theme, &protected);
                node.border_color = rendered_color(&node.border_color, theme, &protected);
                node.color = rendered_color(&node.color, theme, &protected);
                for port in &mut node.ports {
                    port.fill = rendered_color(&port.fill, theme, &protected);
                    port.border_color = rendered_color(&port.border_color, theme, &protected);
                    port.color = rendered_color(&port.color, theme, &protected);
                }
            }
            let mut routes = routes;
            for route in &mut routes {
                route.color = rendered_color(&route.color, theme, &protected);
            }
            let relation_properties =
                crate::properties::relation_values(&diagram, &routes, &styles);
            let node_properties = crate::properties::node_values(&diagram, &boxes, &styles);
            let port_properties = crate::properties::port_values(&diagram, &boxes, &styles);
            Ok(Preview {
                svg,
                notation,
                regions: vec![],
                diagram,
                styles,
                nodes: boxes,
                routes,
                route_paths,
                bounds,
                relation_properties,
                node_properties,
                port_properties,
            })
        },
    )
    .map_err(Diagnostic::from)?
}

// Apply the compiler's legacy paint compatibility pass to inspector colors too.
// This deliberately delegates the mapping rather than duplicating its palette.
fn rendered_color(color: &str, theme: laml::theme::Theme, protected: &[String]) -> String {
    let paint = format!("<svg><rect fill=\"{color}\"/></svg>");
    let tinted = laml::theme::tint_svg_defaults(&paint, theme, protected);
    tinted
        .split("fill=\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .unwrap_or(color)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_and_document_export_share_spacing_rescue_and_compiler_routes() {
        let source = "settings {\ntheme: modern-dark\nbackground: transparent\n}\nsemantics {\nA\nC\nB\n@r A -> B\n}\nlayout {\nA < C : align=center; gap=0\nC < B : align=center; gap=0\nrelation[r].source : side=right\nrelation[r].target : side=left\n}\n";
        let path = Path::new("routing-rescue.laml");
        let preview = build_dark(source, path).unwrap();
        let document = build(source, path).unwrap();
        assert_eq!(preview.routes[0].points, document.routes[0].points);
        assert_eq!(preview.svg, document.svg);
        assert_eq!(
            crate::output::render(source, path, crate::output::Format::Svg).unwrap(),
            document.svg.as_bytes()
        );
        let (nodes, routes) = laml::text::with_fonts(path, true, None, None, || {
            laml::route::place_and_route(&preview.diagram, &preview.styles).map_err(|e| e.message)
        })
        .unwrap()
        .unwrap();
        assert_eq!(routes[0].points, preview.routes[0].points);
        assert_eq!(
            nodes.iter().map(|n| (n.x, n.y)).collect::<Vec<_>>(),
            preview.nodes.iter().map(|n| (n.x, n.y)).collect::<Vec<_>>()
        );
        assert!(nodes[1].x >= nodes[0].x + nodes[0].width + 31.99);
    }

    fn corner_alpha(svg: &str) -> u8 {
        let bytes = laml::export::png(svg).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut output = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut output).unwrap();
        assert_eq!(frame.color_type, png::ColorType::Rgba);
        output[3]
    }

    #[test]
    fn dark_preview_overrides_theme_and_canvas_without_changing_document_exports() {
        let source = "settings {\n theme: plain\n background: opaque\n}\nsemantics {\n A\n B\n A -> B : request\n}\n";
        let path = Path::new("preview.laml");
        let original = source.to_owned();
        let dark = build_dark(source, path).unwrap();
        let document = build(source, path).unwrap();
        assert_eq!(source, original);
        assert_eq!(dark.diagram.settings.theme, laml::theme::Theme::ModernDark);
        assert!(dark.diagram.settings.transparent);
        assert_eq!(dark.styles.theme, laml::theme::Theme::ModernDark);
        assert_eq!(corner_alpha(&dark.svg), 0);
        assert_eq!(document.diagram.settings.theme, laml::theme::Theme::Plain);
        assert!(!document.diagram.settings.transparent);
        assert_eq!(corner_alpha(&document.svg), 255);
        assert_ne!(dark.nodes[0].color, document.nodes[0].color);
        assert_eq!(
            dark.node_properties["A"]["color"].value.as_deref(),
            Some(dark.nodes[0].color.as_str())
        );
        assert_eq!(
            crate::output::render(source, path, crate::output::Format::Svg).unwrap(),
            document.svg.as_bytes()
        );
    }

    #[test]
    fn light_override_is_transparent_while_exports_keep_the_authored_theme() {
        let source = "settings {\n theme: plain-dark\n background: opaque\n}\nsemantics {\n A\n B\n A -> B\n}\n";
        let path = Path::new("preview.laml");
        let light = build_presented(source, path, crate::backend::Presentation::EditorLight).unwrap();
        let document = build(source, path).unwrap();
        assert_eq!(light.diagram.settings.theme, laml::theme::Theme::Modern);
        assert!(light.diagram.settings.transparent);
        assert_eq!(corner_alpha(&light.svg), 0);
        assert_eq!(document.diagram.settings.theme, laml::theme::Theme::PlainDark);
        assert!(!document.diagram.settings.transparent);
        assert_eq!(crate::output::render(source, path, crate::output::Format::Svg).unwrap(), document.svg.as_bytes());
        let base = without_route(&document, source, path, document.routes[0].source_line).unwrap();
        assert_eq!(corner_alpha(&base), 255);
    }

    #[test]
    fn dark_preview_preserves_imported_explicit_colors_and_source_locations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("diagram.laml");
        std::fs::write(
            dir.path().join("colors.lamlstyle"),
            "styling {\n node[A] {\n fill: #123456\n color: #fedcba\n }\n}\n",
        )
        .unwrap();
        let source =
            "semantics {\n A\n B\n A -> B\n}\nstyling {\n import \"colors.lamlstyle\"\n}\n";
        let dark = build_dark(source, &path).unwrap();
        assert_eq!(dark.nodes[0].fill, "#123456");
        assert_eq!(dark.nodes[0].color, "#fedcba");
        assert_eq!(dark.nodes[0].line, 2);
        assert_eq!(dark.routes[0].source_line, 4);
        let error = build_dark_diagnostic("semantics {\n A -> Missing\n}\n", &path).unwrap_err();
        assert_eq!(error.line, Some(2));
        assert_eq!(error.path.as_deref(), Some(path.as_path()));
        assert!(
            build_dark(
                "settings {\n theme: invalid\n}\nsemantics {\n A\n}\n",
                &path
            )
            .is_err()
        );
    }

    #[test]
    fn renders_valid_gne_and_reports_invalid_source() {
        let result = build(
            "semantics {\n A\n B\n A -> B\n}\n",
            Path::new("preview.laml"),
        )
        .unwrap();
        assert!(result.svg.contains("<svg"));
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.nodes[0].line, 2);
        assert!(result.bounds.2 > result.bounds.0);
        assert!(build("semantics {\n A -> Missing\n}\n", Path::new("preview.laml")).is_err());
    }

    #[test]
    fn endpoint_drag_base_really_removes_the_arrow_without_moving_or_recoloring_nodes() {
        fn pixels(svg: &str) -> (u32, Vec<u8>) {
            let data = laml::export::png(svg).unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(data))
                .read_info()
                .unwrap();
            let mut output = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut output).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            output.truncate(info.buffer_size());
            (info.width, output)
        }
        let source = "semantics {\n A\n B\n @r A -> B\n}\n";
        let preview = build_dark(source, Path::new("drag.laml")).unwrap();
        let base = without_route(
            &preview,
            source,
            Path::new("drag.laml"),
            preview.routes[0].source_line,
        )
        .unwrap();
        let (width, original) = pixels(&preview.svg);
        let (base_width, without) = pixels(&base);
        assert_eq!(width, base_width);
        assert_eq!(original.len(), without.len());
        let sample = |data: &[u8], x: f64, y: f64| {
            let x = (x - preview.bounds.0).round() as usize;
            let y = (y - preview.bounds.1).round() as usize;
            let offset = (y * width as usize + x) * 4;
            data[offset..offset + 4].to_vec()
        };
        let points = &preview.routes[0].points;
        let mid = laml::route::Point {
            x: (points[0].x + points[1].x) / 2.0,
            y: (points[0].y + points[1].y) / 2.0,
        };
        assert!(sample(&original, mid.x, mid.y)[3] > 0);
        assert_eq!(sample(&without, mid.x, mid.y)[3], 0);
        for node in &preview.nodes {
            assert_eq!(
                sample(&original, node.x + 5.0, node.y + node.height / 2.0),
                sample(&without, node.x + 5.0, node.y + node.height / 2.0)
            );
        }
    }
}
