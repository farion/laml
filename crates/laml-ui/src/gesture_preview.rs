//! Transient routes share the compiler's SVG painter, theme and typography.
use crate::preview::Preview;
use laml::{
    route::{Point, Route},
    syntax::RelationDecl,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

pub fn draft_route(preview: &Preview, relation: &RelationDecl) -> Route {
    let (color, width, font_size) = laml::route::relation_style(relation, &preview.styles);
    Route {
        source_line: 0,
        points: vec![],
        qualifiers: vec![],
        bridges: vec![],
        gaps: vec![],
        label: None,
        end_labels: vec![],
        from: 0,
        to: 0,
        color,
        width,
        font_size,
        bidirectional: relation.appearance.start != laml::syntax::EndSymbol::None,
        marker_end: relation.appearance.end != laml::syntax::EndSymbol::None,
        appearance: relation.appearance,
    }
}

/// Orthogonal rubber-band routing with normal departure/arrival stubs. Final
/// obstacle-aware routing remains the compiler's checked transaction on drop.
pub fn orthogonal(start: Point, end: Point, source: (f64, f64), target: (f64, f64)) -> Vec<Point> {
    let fallback = if (end.x - start.x).abs() >= (end.y - start.y).abs() {
        ((end.x - start.x).signum(), 0.0)
    } else {
        (0.0, (end.y - start.y).signum())
    };
    let cardinal = |v: (f64, f64)| {
        if v.0.abs() >= v.1.abs() {
            (v.0.signum(), 0.0)
        } else {
            (0.0, v.1.signum())
        }
    };
    let source = cardinal(source);
    let target = cardinal(target);
    let source = if source == (0.0, 0.0) {
        fallback
    } else {
        source
    };
    let target = if target == (0.0, 0.0) {
        (-fallback.0, -fallback.1)
    } else {
        target
    };
    let stub = 24.0;
    let a = Point {
        x: start.x + source.0 * stub,
        y: start.y + source.1 * stub,
    };
    let b = Point {
        x: end.x + target.0 * stub,
        y: end.y + target.1 * stub,
    };
    let mut points = vec![start, a];
    if source.0 != 0.0 && target.0 != 0.0 {
        if (start.y - end.y).abs() < 0.01
            && (end.x - start.x) * source.0 > 0.0
            && (start.x - end.x) * target.0 > 0.0
        {
            return vec![start, end];
        }
        let y = if (a.y - b.y).abs() > 1.0 {
            (a.y + b.y) / 2.0
        } else {
            a.y - stub * 2.0
        };
        points.extend([Point { x: a.x, y }, Point { x: b.x, y }]);
    } else if source.1 != 0.0 && target.1 != 0.0 {
        if (start.x - end.x).abs() < 0.01
            && (end.y - start.y) * source.1 > 0.0
            && (start.y - end.y) * target.1 > 0.0
        {
            return vec![start, end];
        }
        let x = if (a.x - b.x).abs() > 1.0 {
            (a.x + b.x) / 2.0
        } else {
            a.x + stub * 2.0
        };
        points.extend([Point { x, y: a.y }, Point { x, y: b.y }]);
    } else if source.0 != 0.0 {
        points.push(Point { x: a.x, y: b.y });
    } else {
        points.push(Point { x: b.x, y: a.y });
    }
    points.extend([b, end]);
    points.dedup();
    points
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render(
    preview: &Preview,
    source: &str,
    path: &Path,
    route: &Route,
    bounds: (f64, f64, f64, f64),
) -> Result<String, String> {
    let (_, _, _, font, italic) = laml::theme::preprocess_settings(source)?;
    laml::text::with_fonts(
        path,
        preview.styles.theme.modern(),
        font.as_deref(),
        italic.as_deref(),
        || {
            let svg = laml::svg::render_route_overlay(
                &preview.nodes,
                std::slice::from_ref(route),
                preview.styles.theme,
                bounds,
            )?;
            let protected: Vec<_> = preview
                .styles
                .rules
                .iter()
                .flat_map(|r| &r.declarations)
                .filter_map(|d| match &d.value {
                    laml::styles::Value::Color(c) => Some(c.clone()),
                    _ => None,
                })
                .collect();
            let svg = laml::theme::tint_svg_defaults(&svg, preview.styles.theme, &protected);
            laml::export::outlines(&laml::text::svg_fonts(&svg)?)
        },
    )?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transient_paths_are_orthogonal_and_leave_and_enter_each_chosen_side_normally() {
        for from in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            for to in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let points = orthogonal(
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 180.0, y: 130.0 },
                    from,
                    to,
                );
                assert!(
                    points
                        .windows(2)
                        .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
                );
                let first = points[1];
                assert!(first.x * from.0 + first.y * from.1 > 0.0);
                let last = points[points.len() - 2];
                assert!((last.x - 180.0) * to.0 + (last.y - 130.0) * to.1 > 0.0);
            }
        }
    }
    #[test]
    fn every_arrow_type_uses_identical_document_and_overlay_svg_including_theme_corners() {
        for operator in crate::graph_edit::OPERATORS {
            let source = format!("semantics {{\n A\n B\n A {operator} B : request\n}}\n");
            let preview = crate::preview::build_dark(&source, Path::new("preview.laml")).unwrap();
            let mut route = preview.routes[0].clone();
            route.points = orthogonal(
                Point { x: 0.0, y: 0.0 },
                Point { x: 180.0, y: 130.0 },
                (1.0, 0.0),
                (-1.0, 0.0),
            );
            let theme = preview.styles.theme;
            let overlay = laml::svg::render_route_overlay(
                &preview.nodes,
                &[route.clone()],
                theme,
                (-50.0, -50.0, 350.0, 300.0),
            )
            .unwrap();
            let document = laml::svg::render_with_theme(
                &preview.nodes,
                &[route],
                laml::svg::TextMode::System,
                theme,
            )
            .unwrap();
            let body = overlay
                .split_once("</defs>")
                .unwrap()
                .1
                .strip_suffix("</svg>")
                .unwrap();
            assert!(
                document.contains(body),
                "operator {operator}\noverlay={body}\ndocument={}",
                document
                    .split_once("</defs>")
                    .unwrap()
                    .1
                    .split("<g data-node-id")
                    .next()
                    .unwrap()
            );
            assert!(body.contains("<path"));
            assert!(
                !render(
                    &preview,
                    &source,
                    Path::new("preview.laml"),
                    &preview.routes[0],
                    preview.bounds
                )
                .unwrap()
                .is_empty()
            );
        }
    }
}
