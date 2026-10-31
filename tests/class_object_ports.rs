use laml::{
    export,
    geometry::place,
    parser::parse,
    route::route,
    styles::{StyleSheet, load},
    svg::render,
};

#[test]
fn port_styling_and_hiding_preserve_reference_safety() {
    let source = "semantics {\nA {\nport out\n}\nB {\n@in port Input\n}\nA::out --> in\n}\nstyling {\nnode[B] port[in] {\ncolor: \"#ff0000\"\nfill: \"#eeeeee\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routed = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routed).unwrap();
    assert!(svg.contains("fill=\"#ff0000\">Input</text>"));
    assert!(svg.contains("fill=\"#eeeeee\""));
    let hidden = source.replace("fill: \"#eeeeee\"", "display: none");
    let diagram = parse(&hidden).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &styles)
            .unwrap_err()
            .message
            .contains("hidden member")
    );
    let mandatory = hidden
        .replace("A::out --> in", "")
        .replace("styling {", "layout {\nport[in] : side=left\n}\nstyling {");
    let diagram = parse(&mandatory).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &styles)
            .unwrap_err()
            .message
            .contains("hidden port")
    );
}

#[test]
fn boundary_ports_have_scoped_ids_visible_names_and_mandatory_sides() {
    let source = "semantics {\nA {\nport Ausgang\n}\nB {\n@eingang port Eingang\n}\n@pipe A::Ausgang --> eingang\n}\nlayout {\nA < B : align=center; gap=180\nport[A::Ausgang] : side=right\nport[eingang] : side=left\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].points[0].x, boxes[0].x + boxes[0].width);
    assert_eq!(routes[0].points.last().unwrap().x, boxes[1].x);
    assert_eq!(routes[0].points[0].y, boxes[0].ports[0].y);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("data-port-id=\"A::Ausgang\""));
    assert!(svg.contains("data-port-id=\"eingang\""));
    assert!(svg.contains(">Eingang</text>"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let invalid = source.replace("side=left", "side=top");
    assert!(parse(&invalid).unwrap_err().message.contains("left/right"));
    let colliding = "semantics {\n@p A {\n@p port P\n}\n}\n";
    assert!(parse(colliding).unwrap_err().message.contains("global ID"));
    let too_small = "semantics {\nA {\nport P\n}\n}\nstyling {\nnode[A] {\nheight: 50\n}\n}\n";
    let diagram = parse(too_small).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("port")
    );
}
