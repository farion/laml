//! Editor picking geometry derived from the same path builder as SVG output.
use crate::{
    geometry::BoxNode,
    route::{Point, Route},
    theme::Theme,
};
use resvg::{tiny_skia::PathSegment, usvg};

/// Flatten the actual rounded/bridged visible path, including marker-tip docking
/// and underpass gaps. Coordinates are in the compiler's diagram space.
pub fn route_pick_paths(
    route: &Route,
    boxes: &[BoxNode],
    theme: Theme,
) -> Result<Vec<Vec<Point>>, String> {
    let (start, end) = super::route_ends::marker_tips(route, boxes, theme);
    let fragments = crate::svg_paths::fragments_with_ends_rounded(
        route,
        start,
        end,
        if theme.modern() { 9.0 } else { 0.0 },
    )?;
    let paths = fragments
        .iter()
        .map(|d| format!("<path d=\"{d}\" fill=\"none\" stroke=\"black\"/>"))
        .collect::<String>();
    let svg =
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\">{paths}</svg>");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let point = |p: resvg::tiny_skia::Point| Point {
        x: p.x as f64,
        y: p.y as f64,
    };
    let mut result = Vec::new();
    for node in tree.root().children() {
        if let usvg::Node::Path(path) = node {
            let mut current = Vec::new();
            for segment in path.data().segments() {
                match segment {
                    PathSegment::MoveTo(p) => {
                        if !current.is_empty() {
                            result.push(std::mem::take(&mut current));
                        }
                        current.push(point(p));
                    }
                    PathSegment::LineTo(p) => current.push(point(p)),
                    PathSegment::QuadTo(control, finish) => {
                        let start = *current.last().ok_or("Missing curve start")?;
                        let c = point(control);
                        let end = point(finish);
                        for i in 1..=32 {
                            let t = i as f64 / 32.0;
                            let u = 1.0 - t;
                            current.push(Point {
                                x: u * u * start.x + 2.0 * u * t * c.x + t * t * end.x,
                                y: u * u * start.y + 2.0 * u * t * c.y + t * t * end.y,
                            });
                        }
                    }
                    PathSegment::CubicTo(c1, c2, finish) => {
                        let start = *current.last().ok_or("Missing curve start")?;
                        let (a, b, end) = (point(c1), point(c2), point(finish));
                        for i in 1..=32 {
                            let t = i as f64 / 32.0;
                            let u = 1.0 - t;
                            current.push(Point {
                                x: u.powi(3) * start.x
                                    + 3.0 * u * u * t * a.x
                                    + 3.0 * u * t * t * b.x
                                    + t.powi(3) * end.x,
                                y: u.powi(3) * start.y
                                    + 3.0 * u * u * t * a.y
                                    + 3.0 * u * t * t * b.y
                                    + t.powi(3) * end.y,
                            });
                        }
                    }
                    PathSegment::Close => {}
                }
            }
            if !current.is_empty() {
                result.push(current);
            }
        }
    }
    Ok(result)
}
