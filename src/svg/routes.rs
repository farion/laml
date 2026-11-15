//! The shared route painter for documents and transient editor overlays.
use super::escape::escape;
use super::{edge_labels, markers, route_ends};
use crate::{geometry::BoxNode, route::Route, theme::Theme};
use std::fmt::Write;

pub(super) fn draw(
    svg: &mut String,
    boxes: &[BoxNode],
    routes: &[Route],
    theme: Theme,
) -> Result<(), String> {
    for route in routes {
        let (visual_start, visual_end) = route_ends::marker_tips(route, boxes, theme);
        let start = markers::marker(route.appearance.start, true);
        let end = markers::marker(route.appearance.end, false);
        let start_marker = if start.is_empty() {
            String::new()
        } else {
            format!(" marker-start=\"{start}\"")
        };
        let end_marker = if end.is_empty() {
            String::new()
        } else {
            format!(" marker-end=\"{end}\"")
        };
        let dash = if route.appearance.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        if route.bridges.is_empty()
            && route.gaps.is_empty()
            && (!theme.modern() || !crate::svg_paths::has_rounded_corner(route, 9.0))
        {
            let last = route.points.len().saturating_sub(1);
            let points: Vec<String> = route
                .points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let point = if i == 0 {
                        visual_start.unwrap_or(*p)
                    } else if i == last {
                        visual_end.unwrap_or(*p)
                    } else {
                        *p
                    };
                    format!("{:.2},{:.2}", point.x, point.y)
                })
                .collect();
            writeln!(svg,"<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{end_marker}{start_marker} />",
                points.join(" "), route.color, route.width).unwrap();
        } else {
            let fragments = crate::svg_paths::fragments_with_ends_rounded(
                route,
                visual_start,
                visual_end,
                if theme.modern() { 9.0 } else { 0.0 },
            )?;
            let last = fragments.len() - 1;
            for (i, path) in fragments.iter().enumerate() {
                writeln!(svg,"<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}{}{} />",
                    path,route.color,route.width,
                    if i==0 { &start_marker } else { "" },
                    if i==last { &end_marker } else { "" }).unwrap();
            }
        }
        edge_labels::draw(svg, route, theme)?;
    }
    Ok(())
}

pub(super) fn qualifiers(svg: &mut String, routes: &[Route]) {
    for route in routes {
        for qualifier in &route.qualifiers {
            writeln!(svg,"<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"white\" stroke=\"{}\" stroke-width=\"{:.2}\" />",
                qualifier.x, qualifier.y, qualifier.width, qualifier.height, route.color, route.width/2.0).unwrap();
            for (index, line) in qualifier.lines.iter().enumerate() {
                writeln!(svg,"<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"Inter\" font-size=\"{:.2}\" fill=\"{}\">{}</text>",
                    qualifier.x+qualifier.width/2.0,qualifier.y+5.0+qualifier.font_size*1.25*(index as f64+0.78),
                    qualifier.font_size,route.color,escape(line)).unwrap();
            }
        }
    }
}
