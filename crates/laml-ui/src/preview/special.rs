//! Specialized compiler renderers retain their own geometry and notation.
use super::Preview;
use crate::{diagnostic::Diagnostic, notation::Notation};
use laml::{styles::StyleSheet, syntax::Diagram, theme::Theme};
use std::path::Path;

pub(super) fn bounds(svg: &str) -> Result<(f64, f64, f64, f64), String> {
    let root = svg.split('>').next().ok_or("Missing SVG root")?;
    let value = root
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("Missing compiler SVG viewBox")?;
    let n = value
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if n.len() != 4 || !n.iter().all(|n| n.is_finite()) || n[2] <= 0.0 || n[3] <= 0.0 {
        return Err("Invalid compiler SVG viewBox".into());
    }
    Ok((n[0], n[1], n[0] + n[2], n[1] + n[3]))
}

pub(super) fn finish(
    svg: &str,
    styles: &StyleSheet,
    theme: Theme,
    transparent: bool,
) -> Result<String, String> {
    let protected = styles
        .rules
        .iter()
        .flat_map(|r| &r.declarations)
        .filter_map(|d| {
            if let laml::styles::Value::Color(c) = &d.value {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let svg = laml::theme::tint_svg_defaults(svg, theme, &protected);
    let svg = laml::text::svg_fonts(&svg)?;
    let svg = laml::theme::canvas(&svg, theme, transparent)?;
    let svg = laml::export::outlines(&svg)?;
    if transparent {
        laml::theme::canvas(&svg, theme, true)
    } else {
        Ok(svg)
    }
}

pub(super) fn build(
    source: &str,
    path: &Path,
    notation: Notation,
    theme: Theme,
    transparent: bool,
) -> Result<Preview, Diagnostic> {
    let mode = laml::svg::TextMode::Embedded;
    let mut diagram = Diagram::default();
    diagram.settings.theme = theme;
    diagram.settings.transparent = transparent;
    let parse_error = |e: laml::parser::ParseError| Diagnostic::at(path, e.line, e.message);
    let load = |diagram: &Diagram| {
        laml::styles::load(diagram, path)
            .map_err(|e| Diagnostic::in_file(e.path, e.line, e.message))
    };
    let ((svg, regions), styles) = match notation {
        Notation::Bpmn => {
            let mut doc = laml::bpmn::parse(source).map_err(Diagnostic::from)?;
            doc.theme = theme;
            diagram.settings.direction = doc.direction;
            diagram.styling = doc.styling.clone();
            let styles = load(&diagram)?;
            laml::bpmn::validate_styles(&doc, &styles)?;
            laml::bpmn::validate_geometry(&doc)?;
            (laml::bpmn::render_for_editor(&doc, &styles, mode)?, styles)
        }
        Notation::Sequence => {
            let mut doc = laml::sequence::parse(source).map_err(parse_error)?;
            doc.theme = theme;
            diagram.styling = doc.styling.clone();
            let styles = load(&diagram)?;
            (
                laml::sequence::render::render_for_editor(&doc, &styles, mode)?,
                styles,
            )
        }
        Notation::Timing => {
            let mut doc = laml::timing::parse(source).map_err(parse_error)?;
            doc.theme = theme;
            diagram.styling = doc.styling.clone();
            let styles = load(&diagram)?;
            (
                laml::timing::render_for_editor(&doc, &styles, mode)?,
                styles,
            )
        }
        _ => return Err("Expected specialized diagram renderer".into()),
    };
    let bounds = bounds(&svg)?;
    let svg = finish(&svg, &styles, theme, transparent)?;
    Ok(Preview {
        svg,
        notation,
        regions,
        diagram,
        styles,
        nodes: vec![],
        routes: vec![],
        route_paths: vec![],
        relation_properties: Default::default(),
        node_properties: Default::default(),
        port_properties: Default::default(),
        bounds,
    })
}
