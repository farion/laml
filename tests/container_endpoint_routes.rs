//! Connections to an enclosing frame use its inner face in both directions.
use laml::{geometry::place, parser::parse, route::route, styles::load};
use std::path::Path;

#[test]
fn enclosing_package_docks_from_inside_without_losing_mandatory_sides() {
    for reverse in [false, true] {
        for side in ["left", "right", "top", "bottom"] {
            let relation = if reverse { "P -> P.A" } else { "P.A -> P" };
            let end = if reverse { "source" } else { "target" };
            let source = format!(
                "semantics {{\n package P {{\n A\n }}\n @R {relation}\n}}\nlayout {{\n relation[R].{end} : side={side}\n}}\n"
            );
            let diagram = parse(&source).unwrap();
            let sheet = load(&diagram, Path::new("container-ends.laml")).unwrap();
            let boxes = place(&diagram, &sheet).unwrap();
            let routes = route(&diagram, &boxes, &sheet)
                .unwrap_or_else(|e| panic!("reverse={reverse}, side={side}: {e:?}"));
            let points = &routes[0].points;
            let (port, next) = if reverse {
                (points[0], points[1])
            } else {
                (points[points.len() - 1], points[points.len() - 2])
            };
            let p = boxes.iter().find(|n| n.id == "P").unwrap();
            match side {
                "left" => {
                    assert!((port.x - p.x).abs() < 1e-5);
                    assert!(next.x > port.x);
                }
                "right" => {
                    assert!((port.x - p.x - p.width).abs() < 1e-5);
                    assert!(next.x < port.x);
                }
                "top" => {
                    assert!(next.y > port.y);
                }
                "bottom" => {
                    assert!((port.y - p.y - p.height).abs() < 1e-5);
                    assert!(next.y < port.y);
                }
                _ => unreachable!(),
            }
            assert!(next.x > p.x && next.x < p.x + p.width);
            assert!(next.y > p.y && next.y < p.y + p.height);
        }
    }
}

#[test]
fn nested_child_can_connect_to_an_outer_package() {
    let source = "semantics {\n package P {\n package Q {\n A\n }\n }\n P.Q.A -> P\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = load(&diagram, Path::new("nested-container-ends.laml")).unwrap();
    let boxes = place(&diagram, &sheet).unwrap();
    assert_eq!(route(&diagram, &boxes, &sheet).unwrap().len(), 1);
}
