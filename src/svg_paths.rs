//! SVG path fragments for routes with bridge humps and underpass gaps.

use crate::route::{Bridge, Point, Route, bridge_normal};

#[derive(Clone, Copy)]
enum Event<'a> {
    Bridge(&'a Bridge),
    Gap,
}
fn position(a: Point, unit: Point, distance: f64) -> Point {
    Point {
        x: a.x + unit.x * distance,
        y: a.y + unit.y * distance,
    }
}
fn append_line(path: &mut String, point: Point) {
    path.push_str(&format!(" L{:.2},{:.2}", point.x, point.y));
}

/// Returns ordered fragments. Splitting an underpass prevents SVG markers
/// from appearing at either side of the gap; only the whole relation's first
/// and last fragments receive semantic arrowheads.
pub fn fragments(route: &Route) -> Result<Vec<String>, String> {
    fragments_with_ends(route, None, None)
}

pub fn fragments_with_ends(
    route: &Route,
    start: Option<Point>,
    end: Option<Point>,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let first = start
        .or_else(|| route.points.first().copied())
        .ok_or("empty route")?;
    let mut current = format!("M{:.2},{:.2}", first.x, first.y);
    for (segment, pair) in route.points.windows(2).enumerate() {
        let length = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if length < 1e-9 {
            continue;
        }
        let unit = Point {
            x: (pair[1].x - pair[0].x) / length,
            y: (pair[1].y - pair[0].y) / length,
        };
        let normal = bridge_normal(unit);
        let mut events: Vec<(f64, f64, Event<'_>)> = route
            .bridges
            .iter()
            .filter(|b| b.segment == segment)
            .map(|b| {
                (
                    b.distance - b.radius,
                    b.distance + b.radius,
                    Event::Bridge(b),
                )
            })
            .chain(
                route
                    .gaps
                    .iter()
                    .filter(|g| g.segment == segment)
                    .map(|g| (g.distance - g.half, g.distance + g.half, Event::Gap)),
            )
            .collect();
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut cursor = 0.0;
        for (start, end, event) in events {
            if start < cursor - 1e-6 || end > length + 1e-6 {
                return Err("bridge/gap events overlap or leave their segment".into());
            }
            append_line(&mut current, position(pair[0], unit, start));
            match event {
                Event::Gap => {
                    paths.push(current);
                    let p = position(pair[0], unit, end);
                    current = format!("M{:.2},{:.2}", p.x, p.y);
                }
                Event::Bridge(bridge) => {
                    let midpoint = position(pair[0], unit, bridge.distance);
                    let control = Point {
                        x: midpoint.x + normal.x * bridge.height * 2.0,
                        y: midpoint.y + normal.y * bridge.height * 2.0,
                    };
                    let finish = position(pair[0], unit, end);
                    current.push_str(&format!(
                        " Q{:.2},{:.2} {:.2},{:.2}",
                        control.x, control.y, finish.x, finish.y
                    ));
                }
            }
            cursor = end;
        }
        append_line(
            &mut current,
            if segment + 2 == route.points.len() {
                end.unwrap_or(pair[1])
            } else {
                pair[1]
            },
        );
    }
    paths.push(current);
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::Gap;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn sample() -> Route {
        Route {
            points: vec![p(0.0, 0.0), p(200.0, 0.0)],
            qualifiers: vec![],
            bridges: vec![],
            gaps: vec![],
            label: None,
            end_labels: Vec::new(),
            from: 0,
            to: 1,
            color: String::new(),
            width: 2.0,
            font_size: 14.0,
            bidirectional: false,
            marker_end: true,
            appearance: Default::default(),
        }
    }
    #[test]
    fn hump_uses_quadratic_arc() {
        let mut r = sample();
        r.bridges.push(Bridge {
            segment: 0,
            distance: 100.0,
            radius: 8.0,
            height: 8.0,
        });
        let paths = fragments(&r).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("L92.00,0.00 Q100.00,-16.00 108.00,0.00"));
    }
    #[test]
    fn bridge_side_does_not_reverse_with_arrow_direction() {
        for (from, to, control) in [
            (p(0.0, 0.0), p(200.0, 0.0), "Q100.00,-16.00"),
            (p(200.0, 0.0), p(0.0, 0.0), "Q100.00,-16.00"),
            (p(0.0, 0.0), p(0.0, 200.0), "Q-16.00,100.00"),
            (p(0.0, 200.0), p(0.0, 0.0), "Q-16.00,100.00"),
        ] {
            let mut route = sample();
            route.points = vec![from, to];
            route.bridges.push(Bridge {
                segment: 0,
                distance: 100.0,
                radius: 8.0,
                height: 8.0,
            });
            assert!(fragments(&route).unwrap()[0].contains(control));
        }
        let diagonal = bridge_normal(p(-1.0_f64 / 2.0_f64.sqrt(), 1.0_f64 / 2.0_f64.sqrt()));
        assert!(
            diagonal.y < 0.0 && diagonal.x < 0.0,
            "the descending-left crossing in the showcase must arc upward"
        );
    }
    #[test]
    fn visual_port_tips_preserve_bridge_and_gap_positions() {
        let mut r = sample();
        r.gaps.push(Gap {
            segment: 0,
            distance: 90.0,
            half: 8.0,
        });
        r.bridges.push(Bridge {
            segment: 0,
            distance: 150.0,
            radius: 8.0,
            height: 8.0,
        });
        let paths = fragments_with_ends(&r, Some(p(6.0, 0.0)), Some(p(194.0, 0.0))).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].starts_with("M6.00,0.00"));
        assert!(paths[0].ends_with("L82.00,0.00"));
        assert!(paths[1].contains("Q150.00,-16.00"));
        assert!(paths[1].ends_with("L194.00,0.00"));
    }
    #[test]
    fn underpass_is_two_disconnected_subpaths() {
        let mut r = sample();
        r.gaps.push(Gap {
            segment: 0,
            distance: 100.0,
            half: 34.0,
        });
        let paths = fragments(&r).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("L66.00,0.00"));
        assert!(paths[1].starts_with("M134.00,0.00"));
    }
}
