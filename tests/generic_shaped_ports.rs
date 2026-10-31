use laml::{
    export,
    geometry::{NodeShape, place},
    parser::parse,
    route::route,
    styles::load,
    svg::render,
};
use std::path::Path;

fn build(shape: &str) -> Result<(String, Vec<laml::geometry::BoxNode>), String> {
    let source = format!(
        "semantics {{\nA {{\nport Out\n}}\nB\nA::Out --> B\n}}\nlayout {{\nA < B : gap=180\nport[A::Out] : side=right\n}}\nstyling {{\nnode[A] {{\nshape: {shape}\n}}\n}}\n"
    );
    let diagram = parse(&source).map_err(|e| e.message)?;
    let styles = load(&diagram, Path::new("examples/architecture.laml")).map_err(|e| e.message)?;
    let boxes = place(&diagram, &styles).map_err(|e| e.message)?;
    let routed = route(&diagram, &boxes, &styles).map_err(|e| e.message)?;
    let svg = render(&boxes, &routed)?;
    Ok((svg, boxes))
}

#[test]
fn ellipse_and_diamond_ports_use_real_contours_and_safe_labels() {
    for shape in ["ellipse", "diamond"] {
        let (svg, boxes) = build(shape).unwrap_or_else(|error| panic!("{shape}: {error}"));
        let node = &boxes[0];
        assert_eq!(
            node.shape,
            if shape == "ellipse" {
                NodeShape::Ellipse
            } else {
                NodeShape::Diamond
            }
        );
        assert!(node.ports[0].x < node.x + node.width);
        assert!(node.ports[0].x > node.x + node.width / 2.0);
        assert!(svg.contains("data-port-id=\"A::Out\""));
        assert!(export::png(&svg).is_ok());
        assert!(export::pdf(&svg).is_ok());
    }
}

#[test]
fn shaped_ports_can_use_top_bottom_and_left_contours() {
    for shape in ["ellipse", "diamond"] {
        for (side, placement) in [
            ("top", "A below B"),
            ("bottom", "A above B"),
            ("left", "A > B"),
        ] {
            let source = format!(
                "semantics {{\nA {{\nport Out\n}}\nB\nA::Out --> B\n}}\nlayout {{\n{placement} : gap=180\nport[A::Out] : side={side}\n}}\nstyling {{\nnode[A] {{\nshape: {shape}\n}}\n}}\n"
            );
            let diagram = parse(&source).unwrap();
            let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
            let boxes = place(&diagram, &styles)
                .unwrap_or_else(|error| panic!("{shape} {side}: {}", error.message));
            let node = &boxes[0];
            let port = &node.ports[0];
            match side {
                "top" => assert!(port.y >= node.y && port.y < node.y + node.height / 2.0),
                "bottom" => {
                    assert!(port.y <= node.y + node.height && port.y > node.y + node.height / 2.0)
                }
                _ => assert!(port.x >= node.x && port.x < node.x + node.width / 2.0),
            }
            let routed = route(&diagram, &boxes, &styles).unwrap();
            assert!((routed[0].points[0].x - port.x).abs() < 1e-6);
            assert!((routed[0].points[0].y - port.y).abs() < 1e-6);
        }
    }
}

#[test]
fn abstract_shaped_headers_are_checked_with_the_italic_face() {
    let source =
        "semantics {\nabstract A {\nport Out\n}\n}\nstyling {\nnode[A] {\nshape: ellipse\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert!(
        render(&boxes, &[])
            .unwrap()
            .contains("font-style=\"italic\"")
    );
}

#[test]
fn too_small_shaped_port_labels_fail_instead_of_clipping() {
    let source = "semantics {\nA {\nport VeryLongName\n}\n}\nstyling {\nnode[A] {\nshape: diamond\nwidth: 100\nheight: 50\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, Path::new("examples/architecture.laml")).unwrap();
    assert!(place(&diagram, &styles).is_err());
}
