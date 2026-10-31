use laml::{
    export,
    geometry::{NodeShape, place},
    parser::parse,
    route::route,
    styles::load,
    svg::render,
};

#[test]
fn folded_note_ports_attach_to_stroke_not_missing_corner() {
    let source = "semantics {\nMemo(label=\"Memo\") {\nport Top\nport Out\n}\nSink\nMemo::Top --> Sink\nMemo::Out --> Sink\n}\nlayout {\nMemo < Sink : gap=200\nport[Memo::Top] : side=top\nport[Memo::Out] : side=right\n}\nstyling {\nnode[Memo] {\nshape: note\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!(boxes[0].shape, NodeShape::Note);
    let top = &boxes[0].ports[0];
    let out = &boxes[0].ports[1];
    let fold = 12.0_f64
        .min(boxes[0].width / 4.0)
        .min(boxes[0].height / 4.0);
    assert!((top.y - boxes[0].y).abs() < 1e-6);
    assert!(top.x <= boxes[0].x + boxes[0].width - fold);
    assert!((out.x - boxes[0].x - boxes[0].width).abs() < 1e-6);
    assert!(out.y >= boxes[0].y + fold);
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("data-port-id=\"Memo::Top\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}
