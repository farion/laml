use laml::{
    export,
    geometry::place,
    parser::parse,
    route::route,
    styles::StyleSheet,
    svg::{TextMode, render, render_with_mode},
    text::{measure, measure_italic},
};

#[test]
fn abstract_names_use_the_matching_embedded_italic_face_in_every_export() {
    let source = "settings {\ntype: class\n}\nsemantics {\nabstract class Service {\nstatic int count\n--\nabstract +run(): String\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let nodes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &nodes, &styles).unwrap();
    let image = render(&nodes, &routes).unwrap();
    assert_eq!(image.matches("@font-face").count(), 2);
    assert!(image.contains("font-style: italic"));
    assert!(image.contains("font-style=\"italic\""));
    assert!(image.contains("count: int"));
    assert!(image.contains("+run(): String"));
    assert!(measure_italic("Service", 14.0).unwrap() > 0.0);
    assert!(measure("Service", 14.0).unwrap() > 0.0);
    assert!(export::png(&image).is_ok());
    assert!(export::pdf(&image).is_ok());
    let outlines = render_with_mode(&nodes, &routes, TextMode::Paths).unwrap();
    assert!(!outlines.contains("<text "));
    let plain = "settings {\ntype: class\n}\nsemantics {\nclass Service\n}\n";
    let diagram = parse(plain).unwrap();
    let nodes = place(&diagram, &styles).unwrap();
    let image = render(&nodes, &[]).unwrap();
    assert_eq!(image.matches("@font-face").count(), 1);
}
